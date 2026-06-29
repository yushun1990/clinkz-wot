# Phase P3 — Servient Rewire

> Baseline: `docs/baseline/engine-architecture-baseline.md` (v4.0) §7.

## Goal

Rewire `clinkz-wot-servient` on top of the P0–P2 surfaces: drop the directory
generic, async-only driving, frozen-TD lifecycle, and real async consumer.
This is the phase where the **workspace compiles whole again**.

## Entry Criteria

- P0 core (sync-primary handlers, `WotLock`, concrete Thing types), P1 discovery
  (`Discoverer`/`DirectoryPublisher`), P2 binding (async `ClientBinding`/
  `ServerBinding`, wholesale route lifecycle) are stable.

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
    exposed: ExposedThingRegistry,            // WotLock<BTreeMap<ThingId, ThingSlot>>
    consumed: ConsumedThingRegistry,           // WotLock<BTreeMap<ThingId, ConsumedThingEntry>>
    server_bindings: WotLock<Vec<Arc<dyn ServerBinding>>>,
    client_factories: BindingFactoryRegistry,  // generation-tracked, retained
    discoverer: Arc<dyn Discoverer>,
    directory_publisher: Option<Arc<dyn DirectoryPublisher>>,
    security: SecurityContext,
    codecs: WotLock<Vec<Arc<dyn PayloadCodec>>>,
    event_broker: EventBroker,
    shutdown: Arc<AtomicBool>,
}
```

`Servient` is `Clone` (cheap, `Arc`/`WotLock` clones), all methods `&self`,
`Send + Sync`. `WotLock<T>` (from P0) replaces every `Arc<MapLock<T>>`.

### Step 3.2 — Facade (`WoT` surface)

```rust
impl Servient {
    pub async fn produce(&self, td: Thing) -> CoreResult<ExposedThingHandle>;
    pub async fn consume(&self, td: Thing) -> CoreResult<ConsumedThingHandle>;
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess;
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> CoreResult<Thing>;
}
```

`produce`/`consume` validate the TD (well-formed, has `id`), insert into the
registry, return a handle. `discover` delegates to `Discoverer::discover`.
`fetch_td` delegates to `Discoverer::request_thing_description`.

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
- `expose()`: validate TD → insert registry entry →
  `ServerBinding::register_thing` (wholesale, P2) on every server binding →
  `DirectoryPublisher::register` (best-effort). Binding route failure is fatal
  (rollback the registry insert); directory failure is non-fatal (warn).
- `destroy()`: `ServerBinding::unregister_thing` → remove registry entry →
  `DirectoryPublisher::unregister` (best-effort). Order: routes-first.
- `destroy(own_id)` from within a handler uses deferred removal (`DrainFlag`
  semantics retained, simplified onto `WotLock`).

The TD is immutable between `expose()` and `destroy()`. Handler attachment
after `expose()` is permitted (a slot starts `None` → `MissingHandler` until
set), but the affordance set itself is frozen.

### Step 3.5 — Async-only driving

Single driving module replaces `driving_sync.rs` + `driving_async.rs`:

```rust
impl Servient {
    pub async fn poll_serve(&self) -> ServientResult<()>;     // one step
    pub async fn serve(&self);                                 // loop until shutdown
    pub fn poll_serve_once(&self, cx: &mut Context<'_>)
        -> Poll<ServientResult<()>>;                           // bare no_std super-loop
}
```

- `poll_serve`: snapshot `server_bindings` (`WotLock` read → `Arc<[...]>` clone),
  `select_all` over each binding's boxed `poll_accept` future (resolved A3); on
  accept, dispatch. Cross-Thing concurrency via a local `FuturesUnordered` for
  in-flight dispatches (retained from addendum §9.6), no `tokio::spawn`
  (Servient stays spawnable via local-task concurrency).
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
  `AsyncAcceptState`, the `AtomicUsize` round-robin cursor (async `select_all`
  is inherently fair).

### Step 3.6 — Dispatch (`dispatch.rs`)

Single async `InboundDispatcher`:

- Resolve `Thing` from exposed registry by `thing_id`.
- Resolve matched `Form` internally (security scheme lookup); never expose to
  handlers (v3.0 §11).
- `verify_inbound` → `Principal` (or anonymous for NoSec); inject into handler
  `InteractionInput`.
- Clone handler `Arc` out under brief per-Thing `WotLock`, release, then invoke
  — sync handler is called directly (zero-alloc), opt-in async handler is
  `.await`ed (one `Box`, I/O-bound). Reentrancy-safe (v4.0 §4.7).
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
bare super-loop — both paths verified by `check-no-std.sh`.

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

1. **`poll_serve_once` correctness on bare no_std** remains coupled to the
   zenoh-pico `poll_accept` model (deferred). When pico lands, verify that
   `select_all` over pico's poll-driven `poll_accept` futures makes progress
   under repeated `poll_serve_once(noop_waker)` calls.

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
