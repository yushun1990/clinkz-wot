# Phase P3 — Servient Rewire

> Baseline: `docs/baseline/engine-architecture-baseline.md` (v4.0) §7.

## Goal

Rewire `clinkz-wot-servient` on top of the P0–P2 surfaces: drop the directory
generic, async-only driving, frozen-TD lifecycle, and real async consumer.
This is the phase where the **workspace compiles whole again**.

## Entry Criteria

- P0 core (sync-primary handlers, `WotLock`, concrete Thing types), P1 discovery
  (`Discoverer`/`DirectoryPublisher`), P2 binding (async `ClientBinding`;
  **sync `ServerBinding::try_accept`** + wholesale route lifecycle) are stable.

## Current State (being replaced)

- `servient/src/servient.rs` (904 lines): `Servient<D>` generic over directory;
  `BindingFactoryRegistry` with generation tracking.
- `servient/src/servient/{driving_sync.rs, driving_async.rs}`: the four-way
  sync/async driving duplication.
- `servient/src/servient/{dispatch.rs, bulk.rs, security.rs}`: dispatch, bulk
  ops, security application.
- `servient/src/handle.rs` (1520 lines): `ExposedThingHandle<D>` /
  `ConsumedThingHandle<D>` with dynamic affordance mutation (`add_*`/`remove_*`)
  and fake-async consumer methods.
- `servient/src/{registry.rs, consumed.rs, cache.rs, interaction.rs, builder.rs,
  lock.rs, error.rs}`.

## Work Breakdown

### Step 3.1 — `Servient` shape (drop `<D>`)

`Servient` becomes non-generic, holding trait-object discovery:

```rust
pub struct Servient {
    exposed: ExposedThingRegistry,            // Arc-snapshot BTreeMap (lock-free reads)
    consumed: ConsumedThingRegistry,           // Arc-snapshot BTreeMap (lock-free reads)
    server_bindings: BindingList,              // Arc<[...]> snapshot (lock-free reads)
    inbound_fanin: Receiver<InboundRequest>,   // std: bindings self-push via set_request_sink; no_std: try_accept poll
    inbound_fanin_tx: FanInSender<InboundRequest>, // std only; cloned into each binding at registration
    client_factories: BindingFactoryRegistry,  // generation-tracked, retained
    discoverer: Arc<dyn Discoverer>,
    directory_publisher: Option<Arc<dyn DirectoryPublisher>>,
    security: SecurityContext,
    codecs: WotLock<Vec<Arc<dyn PayloadCodec>>>, // read-write-frequent → WotLock
    event_broker: EventBroker,
    shutdown: Arc<AtomicBool>,
}
```

`Servient` is `Clone` (cheap, `Arc`/snapshot clones), all methods `&self`,
`Send + Sync`. **Read-heavy-rare-write state (registries, binding list,
handler tables) is published as `Arc` snapshots with lock-free reads; `WotLock`
is reserved for read-write-frequent / exclusive-semantics state** (audit defect
AD2 — avoids disabling interrupts on the hot read path on `no_std`).

### Step 3.2 — Facade (`WoT` surface)

```rust
impl Servient {
    pub async fn produce(&self, td: Thing) -> CoreResult<ExposedThingHandle>;
    pub async fn consume(&self, td: Thing) -> CoreResult<ConsumedThingHandle>;
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess;
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> CoreResult<Thing>;
}
```

**Lifecycle state machine (audit defect AD8 — closed, single source of truth).**
A produced Thing follows exactly one path, with **one** insertion and **one**
removal, and the draft state lives in **no registry at all**:

- **draft** — `produce()` validates the TD (well-formed, has `id`) and returns
  an `ExposedThingHandle` whose `Arc` state (TD + handler slots) is **entirely
  owned by the handle**. It is NOT in any registry/container, NOT remotely
  servable, NOT discoverable. Local interactions (`read_property` on the
  handle) dispatch directly to the handlers. Dropping a draft handle drops its
  state — nothing to clean up.
- **exposed** — `expose()` is the **single** mutation into shared state: it
  atomically inserts a `ThingSlot` (wrapping the handle's `Arc` state) into the
  servable `ExposedThingRegistry`, calls `register_thing` on every binding, and
  publishes the TD. The handle now references that registry entry (it is an
  "exposed handle"). Remotely servable + discoverable.
- **removed** — `destroy()` is the **single** removal: unregisters routes,
  removes the `ExposedThingRegistry` entry, unpublishes the TD. The Thing is
  **gone** (NOT back to draft — matches Scripting API `destroy()`). The handle
  is inert afterwards; re-`produce` to re-expose.

`consume()` validates the TD and inserts into the **consumed** registry
(consumed Things have no expose/destroy — drop the handle / unsubscribe).
`discover` is synchronous and returns a lazy `ThingDiscoveryProcess`
(§3.2.1). `fetch_td` delegates to `Discoverer::request_thing_description`.

The earlier "destroy → back to draft (or full removal)" wording is withdrawn —
it was ambiguous and contradicted AD8.

### Step 3.3 — Handles (drop `<D>`)

`ExposedThingHandle` / `ConsumedThingHandle` drop the `<D>` generic (they hold
a `Servient` clone). They wrap P0's `LocalExposedThing` / `BoundConsumedThing`
via `Arc`.

### Step 3.4 — Frozen-TD lifecycle (decision 2)

```rust
impl ExposedThingHandle {
    // handler attachment (between produce and expose):
    pub fn set_property_read_handler(&self, name, handler);
    pub fn set_property_write_handler(&self, name, handler);
    pub fn set_property_observe_handler(&self, name, handler);
    pub fn set_action_handler(&self, name, handler);  // invoke
    pub fn set_event_subscribe_handler(&self, name, handler);
    // ... unobserve/unsubscribe/query/cancel slots
    pub async fn expose(&self) -> ServientResult<()>;
    pub async fn destroy(&self) -> ServientResult<()>;
}
```

- **Remove** `add_property`/`add_action`/`add_event`/`remove_property`/
  `remove_action`/`remove_event` and all directory re-publish-on-mutation.
- `expose()` (draft → exposed): validate the configured TD → **single** insert
  into the servable exposed registry (ThingSlot wrapping the handle's `Arc`
  state) → `ServerBinding::register_thing` (wholesale, P2) on every server
  binding → `DirectoryPublisher::register` (best-effort). Binding route failure
  is fatal (rollback the registry insert); directory failure is non-fatal
  (warn). `produce()` does NOT insert — see the state machine above (AD8).
- `destroy()` (exposed → removed): `ServerBinding::unregister_thing` → remove
  registry entry → `DirectoryPublisher::unregister` (best-effort). Order:
  routes-first. The Thing is **gone** (not back to draft); re-`produce` to
  re-expose.

### Step 3.2.1 — `discover()` sync/async boundary (audit defect AD10)

`Servient::discover(&self, filter) -> ThingDiscoveryProcess` is **synchronous
and returns immediately**, and so is `Discoverer::discover()` (P1 §1.9) — both
are sync entry points. The `ThingDiscoveryProcess` is **lazy**: it stashes the
reader + query (`Pending`); the real async `DirectoryReader::open_search().await`
+ Introduction/Exploration happens in the **first `next()`** on the process
(async; `Pending`→`Open`). So no network/directory work happens at construction
— matching the WoT Scripting API `discover()` → lazy `ThingDiscovery` model.
This closes the half-sync/half-async gap (AD10): sync `Servient::discover()`
calls sync `Discoverer::discover()` → lazy process; async only inside `next()`.
- `destroy(own_id)` from within a handler uses deferred removal (`DrainFlag`
  semantics retained, simplified onto `WotLock`).

The TD is immutable between `expose()` and `destroy()`. Handler attachment
after `expose()` is permitted (a slot starts `None` → `MissingHandler` until
set), but the affordance set itself is frozen.

### Step 3.5 — Async-only driving

Single driving module replaces `driving_sync.rs` + `driving_async.rs`:

```rust
impl Servient {
    /// Processes AT MOST ONE inbound request, then returns. Native async.
    pub async fn poll_serve(&self) -> ServientResult<()>;
    pub async fn serve(&self);                                 // loop until shutdown
    /// Processes AT MOST ONE inbound request per call (strict bounded step),
    /// under a caller Context. For bare no_std super-loops.
    pub fn poll_serve_once(&self, cx: &mut Context<'_>)
        -> Poll<ServientResult<()>>;
}
```

**Step contract (audit defect AD6b — strict bounded step).** `poll_serve` and
`poll_serve_once` each advance by **at most one inbound request** per call —
they do NOT drain a ready backlog. This keeps the bare super-loop cooperative:
one request per tick, interleaved with other super-loop work, never
monopolizing the loop when many requests are ready.

- `poll_serve`: **bounded fan-in accept, not `select_all` over boxed
  `poll_accept` futures** (AD1). The Servient owns one **bounded** inbound
  fan-in channel (`Receiver<InboundRequest>`). On std, bindings enqueue from
  their **synchronous** zenoh   callbacks via `fanin_tx.try_send(req)` (callbacks
  cannot `await`; sender injected via `set_request_sink` at registration, AD13;
  no binding-internal queue, AD6a) and the loop is
  `receiver.recv().await` — **O(1), zero per-binding boxing**. It takes ONE
  request and dispatches it. **The driving layer does NOT define the
  saturation policy** — on `Full`, the *binding* applies the AD9 dual-track
  contract (request/response → explicit error reply; streaming/events →
  drop-oldest + overflow). P3 must not re-state or flatten that contract
  (audit defect: P3 previously wrote a uniform drop-oldest, contradicting P2). On bare no_std, the loop
  does ONE round with a **rotation cursor** (audit defect AD7 — without a
  cursor the fixed-order scan starves later bindings, contradicting any
  fairness claim):
  `let start = cursor.fetch_add(1) % n; for i in 0..n { let b = snapshot[(start+i)%n]; if let Some(r) = b.try_accept() { dispatch(r); break; } }`
  — the start offset advances each tick, so across ticks every binding gets a
  fair first-ready turn (no binding starved when another stays busy). Strict
  one step per call (AD6b). The `server_bindings` snapshot is an `Arc<[...]>`
  clone (lock-free load). Cross-Thing concurrency via a local `FuturesUnordered`
  for in-flight dispatches (retained from addendum §9.6), no `tokio::spawn`.
- `serve(&self)` (resolved A4): `while !shutdown.load() { poll_serve().await; }`
  with std-gated idle backoff. Spawn via
  `tokio::spawn(async move { svc.clone().serve().await })` — the `async move`
  block owns the clone and `serve(&self)` borrows it (Pin makes the
  self-referential future sound). Consistent with `poll_serve(&self)` and
  `poll_serve_once(&self)`.
- `poll_serve_once(&self, cx)`: manually polls the `poll_serve` future under a
  caller `Context` (noop-waker for pure super-loops). The bare super-loop usage
  is documented in v4.0 §7.2.
- Delete `driving_sync.rs`, `driving_async.rs`, `DrivingState`,
  `AsyncAcceptState`. **Keep a lightweight `AtomicUsize` rotation cursor** for
  the no_std poll-loop fairness (AD7); the old cursor deletion note assumed
  `select_all`-inherent fairness, which no longer applies once `select_all`
  was removed.

### Step 3.6 — Dispatch (`dispatch.rs`)

Single async `InboundDispatcher`:

- Resolve `Thing` from the exposed-registry **snapshot** (lock-free `Arc`
  load — audit defect AD2; no `WotLock::with_read`, no interrupt disable on the
  hot read path) by `thing_id`.
- Resolve matched `Form` internally (security scheme lookup); never expose to
  handlers (v3.0 §11).
- `verify_inbound` → `Principal` (or anonymous for NoSec); inject into handler
  `InteractionInput`.
- Clone the handler `Arc` from the per-Thing handler-set snapshot
  (`Arc<HandlerSet>`, lock-free), then invoke — sync handler is called directly
  (zero-alloc), opt-in async handler is `.await`ed (one `Box`, I/O-bound).
  Reentrancy-safe (v4.0 §4.7).
- Missing handler slot → `CoreError::MissingHandler { target, operation }` →
  `InboundResponse.error` → binding maps to status (P2 `error_status`).
- Bulk meta-operations (`readallproperties`, etc.) fan out across handlers and
  combine (retained from PLAN C6).

### Step 3.7 — Real async `ConsumedThingHandle`

```rust
impl ConsumedThingHandle {
    pub async fn read_property(&self, name, options) -> CoreResult<InteractionOutput>;
    pub async fn write_property(&self, name, value, options) -> ...;
    pub async fn invoke_action(&self, name, params, options) -> ...;
    pub async fn observe_property(&self, name, options) -> CoreResult<Subscription>;
    pub async fn unobserve_property(&self, name, sub) -> ...;
    pub async fn subscribe_event(&self, name, options) -> CoreResult<Subscription>;
    pub async fn unsubscribe_event(&self, name, sub) -> ...;
    // bulk: read_all/write_all/read_multiple/write_multiple/subscribe_all/unsubscribe_all
}
```

All drive the real async `ClientBinding::invoke`/`subscribe`. **Remove** the
fake `*_async` delegation (PLAN M8) — the methods ARE async now. Form selection
+ binding-plan interning retained (`cache.rs`/`consumed.rs`); the cached live
binding instance is reused via `Arc` clone (addendum §9.4). Directory-driven
invalidation (addendum §3) retained: Servient-mediated
`ConsumedThingRegistry::invalidate(id)` after directory `update`/`unregister`.

### Step 3.8 — `EventBroker` wiring

`emit_event` / `emit_property_change` fan out to registered `PublisherSink`s
(snapshotted `Arc<[...]>` per `(ThingId, EventName)`, retained from hardening).
Inbound `subscribeevent`/`observeproperty` route through the broker-backed sink;
`unsubscribeevent`/`unobserveproperty` remove the sink.

### Step 3.9 — Security

- Inbound: `SecurityProvider::verify` → `Principal`; `check_scopes` against
  affordance `security`/`scopes`. Retained (v3.0 §8).
- Outbound: `SecurityProvider::apply` returns the metadata it added (P0 §0.9);
  bindings send it as protocol headers/attachments. Remove the post-apply diff.
- `CredentialStore`/`InMemoryCredentialStore` retained; `SecurityContext`
  passes the store to `apply`.

### Step 3.10 — Graceful shutdown

`Servient::shutdown_handle()` → `ShutdownHandle` (`Clone`, `Arc<AtomicBool>`).
`serve`/`poll_serve`/`poll_serve_once` check the flag and exit after the
current iteration (retained from PLAN M12).

### Step 3.11 — `ServientBuilder`

Assembles: `discoverer` + optional `directory_publisher`, security providers,
payload codecs, client binding factories (+ support predicates), server
bindings. The builder is the only place that constructs the
`InMemoryDirectory`-backed `LocalDiscoverer` for embedded/local-only use, or
injects a remote-capable `Discoverer` for cloud.

### Step 3.12 — `no_std + alloc` boundary

Crate root + driving primitives (`poll_serve`/`poll_serve_once`) +
registries + handles are `no_std + alloc`. `serve` loop, idle backoff,
`std::eprintln!` diagnostics, host conveniences behind `std`. The async driving
requires an executor on `no_std` (embassy) or manual `poll_serve_once` in a
bare super-loop. **`check-no-std.sh` verifies COMPILATION only** (`cargo check
--no-default-features` = the crate roots compile `no_std + alloc`); it does
NOT exercise the no_std driving path at runtime, and there is no concrete
`no_std` binding (zenoh-pico) to exercise it against in v1. Runtime
verification of the no_std driving is deferred with the pico hardware
platform (see Open Questions).

## Resolved Decisions

- **A4 (serve ownership).** `serve(&self)`, consistent with its `&self`
  siblings `poll_serve` and `poll_serve_once`. Rationale: the three driving
  primitives form a family; `poll_serve_once(&self)` must reuse one `Servient`
  across super-loop iterations and share it with other work, so `&self` is
  required there, and consistency favors `&self` for `serve` too. The
  `WotLock`-based shared-state model makes `Servient` cheaply cloneable and
  `Send + Sync`, so `&self` is sufficient. Spawn uses
  `tokio::spawn(async move { svc.clone().serve().await })`.
- **Bare-no_std waker semantics deferred.** The `poll_serve_once` manual-poll
  primitive is designed poll-driven-friendly, but its waker semantics against a
  real `no_std` binding are validated only when zenoh-pico lands (resolved A3).
  v4.0's `no_std` promise is: the engine and driving primitives compile
  `no_std + alloc`; concrete `no_std` binding execution is deferred with the
  pico hardware platform.

### Open Questions

1. **`poll_serve_once` runtime correctness on bare no_std** is unverified in v1
   (compile-only; coupled to the zenoh-pico `try_accept` model, deferred).
   When pico lands, verify the strict one-step contract (`for b in snapshot {
   if let Some(r) = b.try_accept() { dispatch(r); break; } }` under
   `poll_serve_once(noop_waker)`) — one request advanced per tick, round-robin
   fairness across bindings, no backlog drain.

## Deliverables

- `Servient` (non-generic) matching v4.0 §7.
- End-to-end produce→expose→interact and consume→interact flows.
- **Workspace compiles whole** (`cargo test --workspace`).

## Exit Criteria

- `clinkz-wot-servient` compiles `no_std + alloc` (root) and `std`.
- `cargo test --workspace` passes.
- Integration tests cover:
  - produce → set handlers → expose → (remote) read/write/invoke/subscribe via
    a fake `ServerBinding` and via the opt-in zenoh binding;
  - consume → read/write/invoke/observe/subscribe via a fake `ClientBinding`
    and opt-in zenoh;
  - bulk operations end-to-end;
  - directory-driven consumed-Thing invalidation;
  - `destroy(own_id)` from within a handler (deferred removal);
  - graceful shutdown.
- No `Servient<D>`, `add_*`/`remove_*`, sync driving modules, or fake-async
  consumer references remain.

## Risks

- `handle.rs` is 1520 lines and tightly coupled to the current `<D>` + dynamic
  affordance model. The rewrite is the largest single file change in P3; split
  it into `handle/exposed.rs` + `handle/consumed.rs` during the rewrite (per
  AGENTS.md module guidance) rather than preserving one mega-file.
- `select_all` + `FuturesUnordered` on `no_std` requires
  `futures-util` `alloc` feature (already a workspace dep) and no `tokio`
  primitives in the no_std path. Verify `poll_serve_once` uses only
  `core::task` + `futures-core`.
