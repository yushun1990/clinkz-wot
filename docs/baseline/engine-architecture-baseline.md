# clinkz-wot Engine Architecture Baseline (v4.0)

This document is the consolidated, authoritative **engine-wide** architecture
baseline for `clinkz-wot`. It supersedes the Servient-only baselines
`docs/baseline/servient-design-baseline.md` (v3.0) and
`docs/baseline/servient-design-baseline-addendum.md` (v3.1) as the primary
design reference. Those two documents remain useful as historical record of the
concurrency and inbound-surface decisions that v4.0 inherits; where v4.0
diverges, the divergence is explicit and LOCKED here.

v4.0 is a **one-shot breaking refactor** triggered by three direction decisions
that collapse most of the accumulated complexity:

1. **Full WoT Scripting API alignment** — the engine targets Consumer, Producer,
   and Discovery *User Agent conformance* rather than the previous "Native WoT
   Runtime, Scripting API as design reference only" stance
   (`docs/wot-compliance.md` §Scripting API Boundary). This reverses the old
   positioning.
2. **No dynamic affordance lifecycle in v1** — a Thing Description is frozen at
   `expose()` time. `add_property` / `remove_*` after `expose`, and the per-
   affordance `register_affordance` / `unregister_affordance` binding surface,
   are removed. They return in a later iteration behind an explicit feature.
3. **Async-first, sync as a super-loop adapter** — handler traits and the
   driving layer are async. The four-way sync/async handler and driving
   duplication is collapsed to a single async surface plus a manual-poll
   primitive for bare `no_std` super-loops.

Every design decision below is **LOCKED**.

## 0. Specification Targets and Conformance Posture

Target specifications (normative):

- W3C WoT Architecture 1.1.
- W3C WoT Thing Description 1.1 (TD 2.0 stays behind `td2-preview`).
- W3C WoT Discovery.
- W3C WoT Profile.
- W3C WoT Scripting API — now a **conformance target**, not merely a design
  reference. The engine aims for the Consumer, Producer, and Discovery
  conformance classes defined by the Scripting API. Rust idiom (Result instead
  of throw, `impl Future` instead of Promise, owned buffers) is the *syntax*;
  the *method set, parameter semantics, and error model* follow the Scripting
  API.

Consequences of the posture change:

- The `WoT` facade, `ExposedThing`, `ConsumedThing`, and `ThingDiscovery`
  surfaces are defined against the Scripting API method catalogue. The
  conformance bar for a Thing is a conformant TD *plus* the protocol behavior
  declared by its forms *plus* faithful Scripting API interaction semantics.
- Engine-specific deviations from the Scripting API (notably the pull-queue
  subscription delivery model, §9) are documented as such and are the minimum
  set required for `no_std + alloc` safety. They do not invalidate the
  interaction *semantics*.

## 1. Design Principles

1. **Layering is non-negotiable.** Data contract (TD/TM) knows nothing of
   transport. Interaction core knows nothing of concrete protocols. Bindings
   know nothing of discovery or servient composition. Servient composes; it
   owns no domain logic.
2. **Interaction semantics are the primary abstraction; forms are transport.**
   `read_property` / `write_property` / `invoke_action` / `subscribe_event` are
   the engine's verbs. Form selection and protocol op mapping are machinery the
   core never sees.
3. **Async is the canonical execution model.** `no_std` super-loops drive the
   same async futures by manual polling. There is no parallel synchronous trait
   hierarchy.
4. **One lock primitive, always correct.** `WotLock<T>` (replacing the misnamed
   `MapLock<T>`) is always multi-thread safe: `std::sync` on `std`,
   `critical_section` on `no_std`. The `RefCell` / `UnsafeCell` / `multithread`-
   feature three-way bifurcation (`core/src/sync.rs`) is removed. `WotLock<T>`
   is itself a cheaply-cloneable (`Clone`) `Arc`-backed handle, eliminating the
   ubiquitous `Arc<MapLock<T>>` nesting.
5. **`no_std + alloc` is the baseline contract.** Every crate whose
   responsibility permits it compiles `no_std + alloc`. Networking, async
   runtimes, filesystems, and OS APIs live behind `std` features or in separate
   runtime crates.
6. **Stable unknown-field round-trip fidelity.** TD/TM documents are preserved
   verbatim through deserialization and serialization, including extension
   vocabulary and JSON-LD contexts.

## 2. Crate and Module Map

The crate boundaries are sound and are kept. The rewrites are *inside* the
crates.

| Crate | Path | `no_std+alloc` | v4.0 change |
|---|---|---|---|
| `clinkz-wot-td` | `td` | yes | Keep. Internal cleanups only. |
| `clinkz-wot-core` | `core` | yes | **Rewrite.** Single async interaction surface; concrete Thing types; single lock. |
| `clinkz-wot-protocol-bindings` | `protocol-bindings/core` | yes | Keep. No external change. |
| `clinkz-wot-protocol-bindings-zenoh` | `protocol-bindings/protocols/zenoh` | planning yes / runtime std | Real async consume; drop dynamic-affordance API. |
| `clinkz-wot-discovery` | `discovery` | yes (local) / std (storage) | **Rewrite** per `docs/plan/discovery-directory-refactor-plan.md`. |
| `clinkz-wot-servient` | `servient` | crate root yes | **Simplify.** Drop `Servient<D>`; async-only driving; frozen-TD lifecycle. |
| `clinkz-wot-codecs-cbor` | `codecs/cbor` | yes | Keep. |

## 3. Tier 0 — `clinkz-wot-td` (Data Contract)

Largely healthy. No public API change. Internal cleanups (tracked in
`docs/deferred-design-followups.md`):

- Split `td/src/core/data_type.rs` (957 lines, catch-all) into cohesive
  modules: `core/uri.rs`, `core/metadata.rs`, `core/version.rs`,
  `core/response.rs`, `core/operation.rs`.
- Extract a shared `FormData` core to deduplicate `ThingModelForm` from `Form`
  (deferred #6).
- Extract shared Thing/ThingModel affordance validation helpers (deferred #7).
- Convert free-form `String` error messages to structured enum variants where
  callers match programmatically (deferred #8).
- **Re-export `AbsoluteUri` at the td crate root**
  (`pub use core::data_type::AbsoluteUri;`) — audit defect AD11: P1 discovery
  uses `AbsoluteUri` as a public type (`DiscoveryEndpoint`, `DirectoryRef`,
  `DirectoryQuery`); it is already defined at `core/data_type.rs:86` and
  reachable via `data_type::AbsoluteUri`, but the root re-export is a hard P1
  prerequisite, not an open question.

## 4. Tier 1 — `clinkz-wot-core` (Interaction Core) — REWRITE

This is where the divergence and complexity concentrate. v4.0 rewrites the
public surface.

### 4.1 Thing types become concrete

The single-impl `ExposedThing` and `ConsumedThing` traits
(`core/src/thing.rs`) are removed (deferred #3). `core` owns two concrete
types:

- `LocalExposedThing` — a produced Thing plus its handler set. Lives in core so
  the protocol-neutral dispatcher can drive it.
- `BoundConsumedThing` — a consumed Thing plus its resolved binding plan. Lives
  in core so the consumed dispatch path can invoke it.

`Servient` wraps these in `Arc`-based handles (`ExposedThingHandle`,
`ConsumedThingHandle`), exactly as today, but the indirection trait is gone.

### 4.2 Handler model — sync primary, opt-in async

The nine synchronous single-method handler traits (`core/src/thing.rs`) are
collapsed to a **coherent, consolidated handler model**: one trait per
interaction operation, with **synchronous handlers as the primary,
zero-allocation path** and **an async twin per operation** (all nine, not a
subset) as an opt-in variant for I/O-bound cloud/gateway handlers.

**Why sync-primary.** A handler invocation is the inbound hot path — every
remote property read / event subscription triggers one. On an always-on MCU
gateway doing thousands of interactions per second, an `async_trait` `Box` per
call would fragment the heap over time and add WCET. Handlers are semantically
short callbacks (read a register, return a value), naturally synchronous for
the dominant device case. So the primary handler traits are plain synchronous
`fn`s stored as `Arc<dyn …>`: dispatch is a direct virtual call, **zero
per-interaction heap allocation**.

```rust
// Primary: synchronous, plain trait, zero-alloc dispatch.
pub trait PropertyReadHandler {
    fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait PropertyWriteHandler {
    fn write(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}
// PropertyObserveHandler, PropertyUnsubscribeHandler,
// ActionHandler (invoke), ActionQueryHandler, ActionCancelHandler,
// EventSubscribeHandler, EventUnsubscribeHandler — all plain sync `fn`.
```

**Opt-in async variant (all nine operations).** A handler that legitimately
needs to await (a cloud handler querying a DB, setting up a downstream
subscription, calling another service) cannot block the executor. **Every one
of the nine operations has an async twin** behind the `async` feature
(`#[async_trait]`, `+ Send + Sync`) — observe/unobserve, query/cancel, and
event subscribe/unsubscribe included, not just read/write/invoke. Partial
coverage would force cloud/gateway handlers on the uncovered interactions to
block the executor or bypass the unified abstraction. Registration offers both
flavors per slot; at most one occupies a slot. The async path pays one
`async_trait` `Box` per call, which is acceptable because the handler is
I/O-bound (the Box is noise next to the awaited work).

```rust
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyReadHandler: Send + Sync {
    async fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
// Async twins for ALL nine operations, behind `async`:
// AsyncPropertyRead/Write/Observe/UnsubscribeHandler,
// AsyncActionHandler (invoke) + AsyncActionQuery/CancelHandler,
// AsyncEventSubscribe/UnsubscribeHandler.
```

**Consolidated storage.** One handler-set struct per affordance, with one slot
per operation, each slot holding whichever flavor was registered. This
eliminates the nine parallel trait-object maps:

```rust
pub enum ReadHandler {
    Sync(Arc<dyn PropertyReadHandler>),       // zero-alloc dispatch (primary)
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncPropertyReadHandler>), // Box per call (opt-in)
}
pub struct PropertyHandlerSet {
    pub read:    Option<ReadHandler>,
    pub write:   Option<WriteHandler>,
    pub observe: Option<ObserveHandler>,
    pub unobserve: Option<UnobserveHandler>,
}
pub struct ActionHandlerSet { invoke, query, cancel }
pub struct EventHandlerSet  { subscribe, unsubscribe }
```

`LocalExposedThing` holds `BTreeMap<AffordanceName, PropertyHandlerSet>` (and
Action/Event equivalents). Registration methods (`set_property_read_handler`)
mutate one slot. Dispatch looks up the set once and reads the slot; an absent
slot yields `CoreError::MissingHandler`. This collapses the mechanical
repetition across registration and dispatch while preserving Scripting API
fidelity (separate `set_*` methods, separate trait objects).

Bounds: sync handler trait objects are `Send + Sync` (so `Arc` clones share a
handler across concurrent dispatches and the driving loop stays `Send`). The
current divergence where sync handler trait objects are non-`Send` (addendum
§9.3) is thereby resolved.

### 4.3 Interaction I/O aligned to Scripting API

`InteractionInput` / `InteractionOutput` are reworked to mirror the Scripting
API's `InteractionOptions` (Scripting API §7.1) and response shapes:

```rust
pub struct InteractionOptions {
    pub uri_variables: BTreeMap<String, String>,
    pub form_index: Option<usize>,
    pub data: Option<Payload>,
    pub timeout: Option<Duration>,
}

pub struct InteractionOutput {
    pub data: Option<Payload>,
    pub status: InteractionStatus, // Ok / Created / Accepted-style hint, for async actions later
}
```

The current `InteractionInput.security_metadata` field is removed from the
handler-facing type. Security material belongs to the binding/transport layer,
not to handler inputs. Outbound security application stays on the
`SecurityProvider`/binding path; the verified `Principal` remains on the
inbound handler input (addendum §T3 is kept).

### 4.4 Affordance addressing and correlation

Retained from v3.1 §1/§2: `ThingId`, `CorrelationId`, `AffordanceTarget`
(`Arc<str>`-backed, owned, `'static`), `InboundRequest`, `InboundResponse`,
`BindingRequest` (owned, `Arc<Thing>` / `Arc<Form>`). These are correct and
unchanged.

### 4.5 Binding trait split

Retained: `ClientBinding` (outbound) and `ServerBinding` (inbound), both `&self`
with interior mutability (v3.0 §2, v3.1 §2.4). The dynamic-affordance methods
`register_affordance` / `unregister_affordance` added in addendum §9.2 are
**removed** (decision 2). A binding registers a Thing's routes wholesale during
`expose()` and unregisters them during `destroy()`.

`ClientBinding::invoke` / `subscribe` are `async fn` (resolved A1) — the
outbound path; one `async_trait` `Box` per call, accepted as network-amortized.

**Inbound accept uses a fan-in channel, not `select_all` over boxed
`poll_accept` futures** (audit defect 1). `ServerBinding` exposes a single
**synchronous, non-blocking** `try_accept`:

```rust
pub trait ServerBinding {
    /// Non-blocking drain of one currently-ready inbound request, or `None`.
    /// No `async_trait`, no `Box` — a plain virtual call. (no_std polled path.)
    fn try_accept(&self) -> Option<InboundRequest>;
    /// std fan-in injection (audit defect AD13): the Servient hands each
    /// binding a clone of the bounded fan-in sender at registration; the
    /// binding `try_send`s from its sync transport callbacks. Formalized on
    /// the trait so the std main path is not prose-only implicit coupling.
    #[cfg(feature = "std")]
    fn set_request_sink(&self, sender: FanInSender<InboundRequest>);
    fn register_thing(&self, thing_id: &ThingId, td: &Thing);
    fn unregister_thing(&self, thing_id: &ThingId);
}
```

The driving loop never builds a `select_all` wait set over per-binding boxed
futures. Instead it uses a **single bounded fan-in channel** as the one and
only inbound buffer:

- **std path (main):** the Servient owns one **bounded** fan-in channel
  (`FanInSender<InboundRequest>` / `Receiver`). At registration the Servient
  calls `ServerBinding::set_request_sink(sender)` (AD13) to hand each binding a
  sender clone; the binding enqueues inbound requests from its **synchronous**
  transport callbacks via **`fanin_tx.try_send(req)`** — zenoh callbacks are
  sync closures (`move |query| { … }`, `server.rs:558,601`) and cannot `.await`.
  Bounded capacity ⇒ on `Full` a **policy split by interaction kind** (audit
  defect AD9): request/response is **rejected with an explicit error reply**
  (mapped via `error_status`, immediate client feedback — not silent
  drop/timeout); streaming/events use drop-oldest + overflow counter — there is
  **no binding-internal accept queue** (audit defect AD6a) and no async-bridge
  task. The driving loop is `receiver.recv().await` — **O(1) per step, zero
  per-binding boxing, one request per step**.
- **no_std path:** there is no executor, so bindings cannot self-push; the
  driving loop takes **one** request per tick with a **rotation cursor** so no
  binding is starved:
  `let start = cursor.fetch_add(1) % n; for i in 0..n { let b = snapshot[(start+i)%n]; if let Some(r) = b.try_accept() { dispatch(r); break; } }`
  — the start offset advances each tick, delivering round-robin fairness;
  strict one request per tick, no backlog drain (audit defects AD6b/AD7).
  O(N_bindings) per tick but N is the protocol-binding count (typically 1–5),
  each poll a plain sync virtual call.

This removes the `poll_accept_sync` / `AsyncServerBinding` / boxed-`poll_accept`
surface entirely (addendum §6.2, §9.6 superseded). On std, `try_accept` is
unused (direct push is the main path); on no_std, the zenoh-pico backend's
`try_accept` polls its transport and returns one ready request. The sync
driving primitive (§7.2) drives the same one-step loop.

### 4.6 Subscription primitives

Retained: `EventBroker` (inbound event fan-out) and `Subscription`
(outbound pull-queue with drop-oldest + overflow counter). The queue capacity
model (v3.1 §6.1) is retained. The pull-queue delivery model is the documented
deviation from the Scripting API's listener callback (§9).

### 4.7 Single lock primitive — `WotLock<T>`

The `MapLock<T>` name (which implied it locked a `Map`, yet appeared as
`MapLock<()>`, `MapLock<Vec<…>>`, `MapLock<BindingFactoryState>`) is renamed to
`WotLock<T>`: the WoT engine's portable, always-thread-safe, interior-mutable
lock container. The name is domain-scoped ("WoT" spans every layer of the
engine, so it does not tie a core primitive to a higher layer the way
`ServientLock` would) and handles the pure-lock case `WotLock<()>` naturally
(which a "Cell" name would not). It is also reworked to be itself a
cheaply-cloneable `Arc`-backed handle (`Clone`), so the pervasive
`Arc<MapLock<T>>` nesting becomes plain `WotLock<T>`.

`core/src/sync.rs` becomes:

| Build | `WotLock<T>` backing |
|---|---|
| `std` | `std::sync::RwLock<T>` (read-mostly) / `std::sync::Mutex<T>` (exclusive) |
| `no_std` | `critical_section::Mutex<T>` (there is no blocking `RwLock` for `no_std` — `critical_section` is the primitive; `embassy_sync`'s RwLock is async-only and thus the wrong tool for these always-synchronous brief holds) |

The `RefCell` single-thread backend and the `multithread` feature are removed.
On a bare single-thread `no_std` target, `critical_section` resolves to a
disable-interrupt / no-op implementation that is correct and cheap. This
removes the entire `sync_lock` / `async_lock` / `DrainFlag` / `multithread`
matrix of addendum §9.1.

API shape:

```rust
pub struct WotLock<T> { /* Arc<RwLock<T> | Mutex<T>> */ }
impl<T> WotLock<T> {
    pub fn new(value: T) -> Self;
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R;        // exclusive
    pub fn with_read<R>(&self, f: impl FnOnce(&T) -> R) -> R;       // shared (exclusive on no_std)
}
impl<T> Clone for WotLock<T> { /* refcount bump */ }
```

Two-level locking is retained (v3.0 §7): an outer registry `WotLock` for
insert/remove/enumerate, an inner per-Thing `WotLock` held only across a single
handler call. The reentrancy discipline (clone handler `Arc` out under a brief
lock, release, then invoke) is retained.

**Read-heavy-rare-write state uses lock-free snapshots, not `WotLock` reads**
(audit defect 2). On `no_std`, `WotLock::with_read` degrades to a
`critical_section::Mutex` exclusive entry (interrupt-disabled). Putting the
registry lookup / handler-table lookup / subscription-state read — every
inbound dispatch — behind that path would serialize them and lengthen the
interrupt-disabled window, which is hostile to real-time MCU targets. So the
read-mostly-write-rarely state avoids `WotLock` reads entirely and uses
copy-on-write snapshots:

- `ExposedThingRegistry` / `ConsumedThingRegistry` publish
  `Arc<BTreeMap<ThingId, Arc<ThingSlot>>>` snapshots; a write (expose/destroy)
  builds a new snapshot under a brief write-side critical section and atomically
  swaps the published `Arc`; a **read is a single atomic load — no interrupt
  disable**.
- Per-Thing handler sets are `Arc<HandlerSet>`; dispatch clones the `Arc`
  atomically (lock-free) and invokes outside any lock.
- The server-binding list and `EventBroker` fan-out table already use this
  `Arc<[...]>` snapshot pattern (PLAN §Performance Hardening); it is extended to
  the registries and handler tables.

`WotLock` is reserved for genuinely read-write-frequent or
exclusive-semantics state (driving state, credential store, binding-factory
registry generation counter). The snapshot pattern keeps the inbound hot read
path lock-free on every build.

## 5. Tier 2 — Protocol Bindings

### 5.1 Shared binding (`clinkz-wot-protocol-bindings`)

Healthy. No external change. Form selection, op→form resolution, target
resolution, security metadata extraction, and the structured `BindingError`
taxonomy are kept. Minor: convert remaining free-form `String` `BindingError`
messages to structured variants (deferred #8).

### 5.2 Zenoh binding (`clinkz-wot-protocol-bindings-zenoh`)

Two changes:

1. **Real async consume.** The fake-async consumer surface (PLAN M8, "delegates
   to sync path") is replaced. `ZenohSessionTransport` exposes
   `async fn invoke(request: BindingRequest) -> CoreResult<InteractionOutput>`
   that drives the real `zenoh::Session` (`session.get`, `session.put`). The
   client binding trait becomes async, matching §4.5.
2. **Drop dynamic-affordance API.** The per-affordance
   `register_affordance`/`unregister_affordance` route tracking (addendum §9.2)
   is removed. `expose()` declares all routes for the Thing; `destroy()`
   undeclares them.

The `runtime-*` feature split (planning layer `no_std+alloc` vs concrete
`zenoh`/`zenoh-pico` backends) is retained.

## 6. Tier 3 — Discovery (`clinkz-wot-discovery`) — REWRITE

Execute `docs/plan/discovery-directory-refactor-plan.md` in full. Summary of
the target shape:

- **Introduction** discovers `DiscoveryEndpoint`s (not Things).
- **Exploration** resolves endpoints into TDs or directory sessions via
  `ThingDescriptionResolver`, `ThingLinkResolver`, `DirectoryReader`.
- **Directory** is an Exploration service with continuation-based
  `DirectorySession`, `CountMode`, `ProjectionMode` — not a local CRUD
  container. **v1 ships `ConsistencyMode::Live` only** (audit defect 3);
  `SessionStable` (snapshot-at-open) is deferred — it would re-introduce the
  large-result-set materialization cost that lazy continuation was meant to
  remove, especially for remote/large directories. `ConsistencyMode` stays
  `#[non_exhaustive]` so `SessionStable` is added non-breakingly once its
  snapshot semantics and remote-backend cost are resolved.
- **Discovery process** (`ThingDiscoveryProcess`) is a lazy session handle, not
  a buffered `VecDeque<Thing>`.
- **Publisher side** is lease/revision-aware (`DirectoryPublisher`).
- **Watch** (`DirectoryWatch`) is separate from search pagination.

`Servient` no longer carries `Servient<D>`; it holds
`Arc<dyn Discoverer>` + `Option<Arc<dyn DirectoryPublisher>>`. The in-memory
backend is demoted to a reference implementation of those traits.

`discovery/src/scripting.rs` (the transitional `ThingFilter.method` model,
`DiscoveryMethod::Local/Directory/Multicast/Everything`) is replaced by the
`Discoverer` trait surface (`discover` / `explore_directory` /
`request_thing_description`).

## 7. Tier 4 — Servient (`clinkz-wot-servient`) — SIMPLIFY

### 7.1 Shape

```rust
pub struct Servient {
    exposed: ExposedThingRegistry,
    consumed: ConsumedThingRegistry,
    server_bindings: BindingList,            // Arc-snapshot of registered ServerBindings
    client_factories: BindingFactoryRegistry,
    discoverer: Arc<dyn Discoverer>,
    directory_publisher: Option<Arc<dyn DirectoryPublisher>>,
    security: SecurityContext,
    codecs: PayloadCodecRegistry,
    event_broker: EventBroker,
    shutdown: Arc<AtomicBool>,
}
```

`Servient` is `Clone` (cheap, `Arc`-based), all methods `&self`, `Send + Sync`.

### 7.2 Driving layer — async only

The four-way sync/async duplication collapses:

```rust
impl Servient {
    /// One step: accept at most one inbound request across all bindings and
    /// dispatch it. Native async; suspends on Waker when no request is ready.
    pub async fn poll_serve(&self) -> ServientResult<()>;

    /// Convenience loop: `while !shutdown { poll_serve().await; }`.
    pub async fn serve(&self);

    /// Manual-poll primitive for bare no_std super-loops without an executor.
    /// Advances the poll_serve future one step under a caller-supplied
    /// Context. Returns Pending when no request is ready.
    pub fn poll_serve_once(&self, cx: &mut core::task::Context<'_>)
        -> core::task::Poll<ServientResult<()>>;
}
```

All three driving primitives take `&self` (resolved A4), forming a consistent
family: `poll_serve_once(&self)` reuses one `Servient` across super-loop
iterations and shares it with other work, so `&self` is required there, and
`serve`/`poll_serve` match for consistency. `serve()` is spawnable on
tokio/embassy via `tokio::spawn(async move { svc.clone().serve().await })` —
the `async move` block owns the cheaply-cloned `Servient` and `serve(&self)`
borrows it (Pin makes the self-referential future sound).

**Step contract — at most one inbound request per call** (audit defect AD6b).
`poll_serve` and `poll_serve_once` each advance by **at most one** request —
they never drain a ready backlog, so a bare super-loop stays cooperative (one
request per tick, interleaved with other work).

**Accept is a single bounded fan-in channel, not `select_all`** (audit defect
AD1/AD6a, see §4.5). The driving step does NOT build a `select_all` over
per-binding boxed `poll_accept` futures and there is **no binding-internal
accept queue**. On std the binding enqueues from its **synchronous** zenoh
callbacks via `fanin_tx.try_send(req)` (zenoh callbacks cannot `.await`;
bounded capacity — on `Full` request/response is rejected with an explicit
error reply and streaming/events drop-oldest + overflow, see AD9); the loop
`receiver.recv().await`s the single bounded fan-in channel (O(1), one request
per step). On bare no_std it takes one request per tick
`for b in snapshot.rotate_from(cursor) { if let Some(r) = b.try_accept() {
dispatch(r); cursor.advance_past(b); break; } }` (rotation cursor below;
O(N_bindings), sync, no boxing, no drain).

The bare super-loop usage:

```rust
// no_std cooperative super-loop, no executor
loop {
    let waker = noop_waker();
    let mut cx = core::task::Context::from_waker(&waker);
    let _ = svc.poll_serve_once(&mut cx);
    // ...other super-loop work (sensor reads, sub-device polling)...
}
```

The current `driving_sync.rs` / `driving_async.rs` / `DrivingState` /
`AsyncAcceptState` split is replaced by a single driving module.

### 7.3 Lifecycle — frozen TD (decision 2)

```rust
impl Servient {
    pub async fn produce(&self, td: Thing) -> CoreResult<ExposedThingHandle>;
    pub async fn consume(&self, td: Thing) -> CoreResult<ConsumedThingHandle>;
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess;
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> CoreResult<Thing>;
}

impl ExposedThingHandle {
    pub fn set_property_read_handler(&self, name, handler);
    pub fn set_property_write_handler(&self, name, handler);
    // ... set_action_handler, set_event_subscribe_handler, etc.
    pub async fn expose(&self) -> ServientResult<()>;   // registers routes + publishes TD; TD frozen after
    pub async fn destroy(&self) -> ServientResult<()>;  // unregisters routes + unpublishes TD
    pub async fn read_property(&self, name, options) -> ...;  // server-side local read
    pub async fn write_property(...);
    pub async fn emit_event(&self, name, data);
    pub async fn emit_property_change(&self, name, data);
}
```

There is **no** `add_property` / `remove_property` / `add_action` / `add_event`
after `expose()` — the **TD affordance set is frozen** at expose (decision 2).
**Handlers, however, may be attached or replaced throughout the exposed
lifetime** (audit defect AD14 — the earlier "handlers only between produce and
expose" wording conflicted with P3 and with the Scripting API). Rationale: a
handler is runtime behavior for an already-declared affordance, not TD
structure, and the Scripting API allows `setPropertyReadHandler` etc. at any
time. An affordance whose handler slot is still `None` returns
`CoreError::MissingHandler` — a **designed-in** semantic for an exposed-but-
unwired affordance, not an error condition. (Handler swap publishes a new
`Arc` handler-set snapshot; an in-flight dispatch keeps the handler `Arc` it
cloned out.) **Lifecycle state machine (AD8):** `produce()` creates a draft
handle whose `Arc` state (TD + handler slots) lives in **no registry**;
`expose()` is the **single** insertion into the servable exposed registry
(ThingSlot wrapping that `Arc` state) + route registration + TD publish;
`destroy()` is the **single** removal (Thing **gone**, not back to draft —
re-`produce` to re-expose). One insertion, one removal, no second "becomes a
registry thing" point. `expose()` registers all inbound routes wholesale and
publishes the TD; the TD affordance set is immutable thereafter until
`destroy()`.

**`destroy()` quiescing (audit defect AD15).** Teardown is more than
routes-first; it defines the fate of every in-flight request:

1. `ServerBinding::unregister_thing` on every binding (routes-first → no **new**
   requests can arrive).
2. Set the ThingSlot `draining` flag. The driving loop honors it: any
   not-yet-dispatched request already in the fan-in channel (or accepted via
   `try_accept`) that targets this Thing is **rejected** — request/response
   gets a synthesized "Thing gone" error reply (status-mapped via
   `error_status`, 410-style); streaming/events are dropped.
3. **In-flight handlers already executing are allowed to complete** (they hold
   a handler `Arc` cloned out before draining); their results are **discarded**
   if the Thing is already removed (the response goes nowhere). Async handlers
   are not cancelled mid-`.await`.
4. Once no in-flight dispatch remains (quiesce point), remove the registry
   entry.
5. `DirectoryPublisher::unregister` (best-effort).

`destroy(own_id)` from within the Thing's own handler is the special case: the
in-flight handler is step 3 itself, so removal is **deferred** until it returns
(v3.0 §7 deferred-removal rule, retained).

The dynamic-affordance network propagation (addendum §9.2), directory
re-publish-on-mutation, and `register_affordance`/`unregister_affordance` are
all removed.

**`discover()` sync/async boundary (audit defect AD10).**
`Servient::discover(&self, filter) -> ThingDiscoveryProcess` is **synchronous
and returns immediately**, and so is `Discoverer::discover()` — both are sync
entry points. The `ThingDiscoveryProcess` is lazy: it stashes the reader +
query (`Pending`), and the real async work (`DirectoryReader::open_search().await`
+ Introduction/Exploration) happens in the **first `next()`** on the process
(which is async; `Pending`→`Open` on first call). No network/directory work at
construction (matches the WoT Scripting API `discover()` → lazy `ThingDiscovery`
model). `Discoverer::request_thing_description()` stays async (a concrete TD
fetch IS a network round-trip).

### 7.4 ConsumedThing — real async

```rust
impl ConsumedThingHandle {
    pub async fn read_property(&self, name, options) -> CoreResult<InteractionOutput>;
    pub async fn write_property(&self, name, value, options) -> ...;
    pub async fn invoke_action(&self, name, params, options) -> ...;
    pub async fn observe_property(&self, name, options) -> CoreResult<Subscription>;
    pub async fn unobserve_property(&self, name, sub) -> ...;
    pub async fn subscribe_event(&self, name, options) -> CoreResult<Subscription>;
    pub async fn unsubscribe_event(&self, name, sub) -> ...;
    // bulk: read_all_properties, write_all_properties, read_multiple_properties,
    //       write_multiple_properties, subscribe_all_events, unsubscribe_all_events
}
```

All methods drive the real async `ClientBinding`. The fake "delegates to sync"
consumer surface (M8) is removed. Bulk operations prefer a Thing-level
meta-operation form when the TD advertises one (W3C TD §6.3.3), otherwise fan
out (behavior retained from PLAN C6).

### 7.5 Security and credentials

Retained: `SecurityProvider` (with `verify` for inbound, `apply` for outbound),
`Principal`/`PrincipalId`, `CredentialStore`/`InMemoryCredentialStore`,
inbound `AuthMaterial` extraction. The `apply_security` post-apply diff is
replaced by having `apply` return the metadata it added (deferred #4).

## 8. Feature Policy

| Feature | Effect |
|---|---|
| `default = ["std"]` | std runtime + tokio convenience. |
| `alloc` | dynamic data on `no_std`. |
| `std` | networking, filesystem, async runtime, host convenience (`serve` loop, idle backoff). |
| `async` | native-async driving (always on for `std`; required for the canonical model). On `no_std`, driving is manual-poll by default and native-async suspension requires an executor (embassy). |
| `zenoh` | Rust `zenoh` std backend (real async consume + inbound). |
| `zenoh-pico` | constrained `no_std+alloc` platform-hook backend (mutually exclusive with `zenoh`). |
| `td2-preview` | experimental TD 2.0 fields. |

The `multithread` feature is **removed** — the lock primitive is always
thread-safe.

## 9. Documented Deviations from the Scripting API

These are the minimum deviations required for `no_std + alloc` and are
documented, not hidden:

1. **Subscription delivery is a pull queue, not a push callback.** A
   `ConsumedThingHandle::subscribe_event` returns a `Subscription` drained by
   `poll_next` (sync) or a `Stream` impl (async). Rationale: a callback fired
   from inside the protocol poll can self-deadlock or block the super-loop on a
   bare MCU; decoupling arrival from handling is the safe model. The semantic
   contract (the subscriber eventually observes the event) is preserved.
2. **Errors are `Result`, not thrown exceptions.** Rust idiom.
3. **`fetchTD` / directory exploration are trait objects (`Discoverer`),** not a
   built-in `fetch` — the engine is protocol-neutral and the concrete transport
   is injected.

No other deviations are permitted without an explicit entry here.

## 10. Scripting API Conformance Map

| Scripting API | clinkz-wot surface | Notes |
|---|---|---|
| `WoT.produce(td)` | `Servient::produce(td)` | returns `ExposedThingHandle` |
| `WoT.consume(td)` | `Servient::consume(td)` | returns `ConsumedThingHandle` |
| `WoT.discover(filter)` | `Servient::discover(filter)` | returns `ThingDiscoveryProcess` (lazy session) |
| `WoT.fetchTD(url)` | `Servient::fetch_td(url)` | async |
| `ExposedThing.setPropertyReadHandler` | `ExposedThingHandle::set_property_read_handler` | |
| `ExposedThing.setPropertyWriteHandler` | `set_property_write_handler` | |
| `ExposedThing.setPropertyObserveHandler` | `set_property_observe_handler` | |
| `ExposedThing.setActionHandler` | `set_action_handler` | invoke op |
| `ExposedThing.setEventSubscribeHandler` | `set_event_subscribe_handler` | |
| `ExposedThing.readProperty`/`writeProperty` | `read_property`/`write_property` (server-side local) | |
| `ExposedThing.emitEvent`/`emitPropertyChange` | `emit_event`/`emit_property_change` | |
| `ExposedThing.expose()`/`destroy()` | `expose()`/`destroy()` | TD frozen after expose |
| `ConsumedThing.readProperty` | `read_property(name, options)` | async, real binding |
| `ConsumedThing.writeProperty` | `write_property` | |
| `ConsumedThing.invokeAction` | `invoke_action` | |
| `ConsumedThing.observeProperty`/`unobserveProperty` | `observe_property`/`unobserve_property` | returns `Subscription` (deviation §9.1) |
| `ConsumedThing.subscribeEvent`/`unsubscribeEvent` | `subscribe_event`/`unsubscribe_event` | returns `Subscription` (deviation §9.1) |
| `ConsumedThing.readAllProperties`/`writeAllProperties`/`readMultipleProperties`/`writeMultipleProperties`/`subscribeAllEvents`/`unsubscribeAllEvents` | bulk methods | retained from PLAN C6 |
| `ThingDiscovery.start/next/stop` | `ThingDiscoveryProcess` (async session) | lazy, continuation-based |

## 11. Performance Targets

The per-interaction hot path must be allocation-light and lock-bounded:

- **Affordance addressing** uses `Arc<str>` (already done, retained).
- **Handler invocation** clones one `Arc<dyn Handler>` from a per-Thing
  handler-set **snapshot** (`Arc<HandlerSet>`, lock-free atomic load — audit
  defect 2), then invokes. The primary sync handler path is a direct virtual
  call — **zero per-interaction heap allocation**. The opt-in async handler
  path pays one `async_trait` `Box` per call (acceptable: the handler is
  I/O-bound).
- **Inbound accept** is a single **bounded** fan-in channel on std (O(1)
  `recv`, zero boxing; binding enqueues via sync `try_send` from zenoh
  callbacks — they cannot `await`; on `Full` request/response is rejected with
  an explicit error reply, streaming/events drop-oldest + overflow — no
  binding-internal queue, AD6a/AD9) and a sync `try_accept` poll on no_std (one
  request per tick, rotation cursor, O(N_bindings), no boxing — AD6b). No
  `select_all`, no per-binding boxed `poll_accept` future (audit defect AD1).
- **Registry / handler-table / subscription-state reads** are lock-free
  `Arc`-snapshot loads; no `WotLock::with_read` (no interrupt disable) on the
  hot read path (audit defect 2).
- **Outbound form/binding plan** is interned in the consumed registry entry
  (addendum §9.4 retained); repeated consumed interactions reuse the cached
  binding instance via `Arc` clone — no `make_binding`, no plan recompute.
- **Event fan-out** shares `Payload` bytes via `Arc<[u8]>` (retained); media
  metadata may move to `Arc<str>` if profiling warrants (deferred #1).
- **Lock contention** is bounded: `WotLock` is reserved for read-write-frequent
  / exclusive-semantics state; read-heavy-rare-write state uses snapshots.
- **Directory queries** are continuation-based (one batch + token), `Live`
  consistency only in v1, not full-table scan with `offset+total` (discovery
  refactor; audit defect 3).

## 12. Sequencing

The refactor is sequenced for **target-crate isolation through P2, workspace
whole at P3** (audit defect AD17 — unifies with `PLAN.md` §Dependency shape;
the earlier "keep the workspace compiling at each phase" wording was wrong
because P0 rewrites core's public surface and breaks core's dependents until
they adapt):

- **P0 — Core interaction surface rewrite.** Sync-primary handler trait set
  with opt-in async twins; consolidated handler storage; concrete
  `LocalExposedThing` / `BoundConsumedThing`; `WotLock`; `InteractionOptions`/
  `InteractionOutput` rework. `no_std+alloc` verified.
- **P1 — Discovery rewrite.** Introduction/Exploration/session traits; in-memory
  backend as reference impl; `Discoverer`/`DirectoryPublisher`/
  `DirectoryWatch`. Execute the discovery refactor plan.
- **P2 — Binding async.** Real async `ClientBinding::invoke`; zenoh
  `ZenohSessionTransport` async consume; remove dynamic-affordance API.
- **P3 — Servient rewire.** Drop `Servient<D>`; async-only driving
  (`poll_serve`/`serve`/`poll_serve_once`); frozen-TD lifecycle; real async
  `ConsumedThingHandle`; remove `add_*`/`remove_*` and the sync driving
  modules.
- **P4 — Compliance and verification.** Scripting API conformance map tests;
  feature-matrix checks; `check-no-std.sh`; fixtures; Clippy. Update
  `PLAN.md`, `docs/technical-spec.md`, `docs/wot-compliance.md`,
  `docs/no-std-embedded.md`, `docs/verification.md`.

Each phase is independently shippable behind the workspace build.

## 13. What This Supersedes

- `docs/baseline/servient-design-baseline.md` (v3.0) — retained as historical
  record; v4.0 inherits §1 roles, §5 storage ownership, §7 two-level locking,
  §8 security, §9 subscription flow, §10 expose/destroy coordination, §11
  inbound request shape. v4.0 reverses the async/sync duality (§4) and the
  dynamic-affordance surface (§9.2 of the addendum).
- `docs/baseline/servient-design-baseline-addendum.md` (v3.1) — retained as
  historical record; v4.0 inherits §1 concrete types, §2 owned request model,
  §3 directory-invalidation trigger, §5 error taxonomy. v4.0 reverses §9.1
  sync-lock / §9.2 dynamic affordance / §9.3 Send-bound divergence.
- `docs/wot-compliance.md` §Scripting API Boundary — the "Native Runtime, not a
  Scripting API UA" positioning is reversed. The subscription-deviation note
  is preserved as §9.1 here.
- `docs/no-std-embedded.md` MCU three-layer plan — Layer 1 (`multithread`
  feature) is superseded by the unified lock primitive; Layer 2 (zenoh-pico)
  and Layer 3 (embassy) boundaries are retained.

## 14. Decision Index

| Decision | Topic | Resolution |
|---|---|---|
| D1 | Scripting API alignment | Full Consumer/Producer/Discovery UA conformance target. (§0) |
| D2 | Dynamic affordance lifecycle | Removed in v1; TD frozen at expose. (§4.5, §7.3) |
| D3 | Async/sync model | Async driving/transport layer; sync handlers primary (zero-alloc hot path) with opt-in async handlers (feature/cloud); sync driving is a manual-poll super-loop adapter. (§4.2, §7.2) |
| D4 | Lock primitive | `WotLock<T>`: `Arc`-backed portable handle, `std::sync` / `critical_section`; renames `MapLock`; `multithread` feature removed. (§4.7) |
| D5 | Thing abstractions | Concrete `LocalExposedThing`/`BoundConsumedThing`; single-impl traits removed. (§4.1) |
| D6 | Handler storage | One consolidated handler-set per affordance; sync traits primary, async twins (all 9 ops) opt-in per Scripting API method. (§4.2) |
| D7 | Discovery | Execute the Introduction/Exploration/session refactor; `Servient` holds `Discoverer` trait object. (§6, §7.1) |

### Audit defect resolutions (locked)

| Defect | Topic | Resolution |
|---|---|---|
| AD1 | Inbound accept fan-in | Drop boxed `poll_accept` + `select_all`; fan-in channel (std, O(1)) + sync `try_accept` (no_std, O(N_bindings), no boxing). (§4.5, §7.2) |
| AD6a | Unbounded accept buffer | Single **bounded** fan-in channel (capacity configurable); std bindings enqueue from **synchronous** zenoh callbacks via `try_send` (callbacks cannot `await`); on `Full` the policy is split by interaction kind (AD9); **no binding-internal queue**, no async-bridge task. (§4.5, §7.2) |
| AD6b | `poll_serve_once` step semantics | Strict bounded step: at most ONE inbound request per `poll_serve`/`poll_serve_once` call; no backlog drain (no_std `if let … break`, not `while let`). (§7.2) |
| AD7 | no_std poll-loop fairness | Restore a lightweight `AtomicUsize` rotation cursor for the no_std `try_accept` poll loop (the old "select_all-inherent fairness" rationale died with `select_all`); start offset advances each tick. (§4.5, §7.2) |
| AD8 | produce/expose registry insertion | `produce()` creates a draft handle only (no registry insert); `expose()` is the SINGLE insertion into the servable exposed registry. Closes the lifecycle state machine: draft → exposed → removed. (§7.3) |
| AD9 | Overload policy for request/reply | On fan-in `Full`: request/response interactions are **rejected with an explicit error reply** (mapped via `error_status`, immediate client feedback); only streaming/events use drop-oldest + overflow. No silent drop/timeout as the request/reply default. (§4.5, §11) |
| AD10 | `discover()` sync/async boundary | `Servient::discover()` AND `Discoverer::discover()` are both **sync**, returning a lazy `ThingDiscoveryProcess`; the async `DirectoryReader::open_search()` is deferred to the first async `next()`. No network/directory work at construction. `request_thing_description()` stays async (real network fetch). (§6, §7.3) |
| AD11 | `AbsoluteUri` exposure | td re-exports `AbsoluteUri` at its crate root as a hard P1 prerequisite (it was a P1 open question; P1's independent-compile promise rested on it). (§3) |
| AD12 | Dynamic affordance surface removed from code | The `register_affordance`/`unregister_affordance` binding trait methods, the `ExposedThingHandle::{add,remove}_{property,action,event}` methods, their Servient propagation (`sync_added/sync_removed_affordance`), the zenoh per-affordance impls, and the dedicated tests are **deleted from the current code** (not just docs), closing the code↔baseline divergence. Workspace `cargo check --all-targets` and `cargo test --workspace` pass. |
| AD13 | Fan-in sender injection formalized | The std fan-in `Sender` injection is a **trait method** `ServerBinding::set_request_sink(sender)` (std-gated), called by the Servient at registration — not prose-only "the binding receives a Sender clone". The driving layer drains; it does not own the overload policy (that stays the binding's AD9 contract). (§4.5, §7.2) |
| AD14 | Handler lifecycle vs TD freeze | The TD **affordance set** is frozen at `expose()` (decision 2), but **handlers may be attached/replaced throughout the exposed lifetime** (Scripting API aligned). `MissingHandler` is the designed-in semantic for an exposed-but-unwired affordance. Resolves the baseline-vs-P3 conflict. (§7.3) |
| AD15 | `destroy()` quiescing | Teardown = routes-first + `draining` flag (pending requests rejected: request/reply → "Thing gone" error, streaming dropped) + in-flight handlers allowed to complete (results discarded) + entry removed at quiesce + unpublish. Self-`destroy` from a handler = deferred removal. (§7.3) |
| AD16 | no_std driving = compile-time architecture only | P3's no_std path is compile-only in v1; runtime validation is gated on zenoh-pico (P2 §2.7). P3 depends on the `try_accept` trait *shape*, not on pico's server-side runtime being finalized. (§7.2, P3 §3.12) |
| AD17 | Phase compile boundary | P0–P2 are target-crate isolation (each target crate compiles/tests alone); the workspace is made whole at P3. Unifies baseline §12 with `PLAN.md`. |
| AD18 | `ProjectionMode` vs `ThingDiscoveryProcess` | `ThingDiscoveryProcess` (Scripting-API surface yielding full `Thing`s) **forces `FullThingDescription`**; `IdOnly`/`Summary` are confined to the lower-level `DirectorySession`/`DirectoryItem` API (directory-admin use) and do not flow into the Scripting process. (`docs/plan/phase-p1-discovery.md` §1.4/§1.6) |
| AD6c | no_std verification overclaim | `check-no-std.sh` is compile-only; runtime no_std driving deferred with zenoh-pico. (§7.2, `docs/plan/phase-p3-servient.md`, `phase-p4-compliance.md`) |
| AD2 | `WotLock` no_std read degradation | Read-heavy-rare-write state (registries, handler tables, subscription state) uses lock-free `Arc`-snapshot reads; `WotLock` reserved for read-write-frequent/exclusive state. (§4.7, §11) |
| AD3 | `SessionStable` snapshot cost | v1 ships `ConsistencyMode::Live` only; `SessionStable` deferred (`#[non_exhaustive]`). (§6) |
| AD4 | Async handler coverage | Async twins for ALL 9 interaction operations, not just read/write/invoke. (§4.2) |
| AD5 | Conservative compliance matrix | P4 build-checks all valid feature combinations per crate; tests a representative subset. (`docs/plan/phase-p4-compliance.md`) |
