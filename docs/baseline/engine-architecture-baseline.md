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

The nine synchronous single-method handler traits plus their three async twins
(`core/src/thing.rs`) are collapsed to a **coherent, consolidated handler
model**: one trait per interaction operation, with **synchronous handlers as
the primary, zero-allocation path** and **async handlers as an opt-in variant**
for the rare I/O-bound cloud/gateway handler.

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

**Opt-in async variant.** A handler that legitimately needs to await (a cloud
handler querying a DB, calling another service) cannot block the executor. For
those, an async twin trait is provided behind the `async` feature
(`#[async_trait]`, `+ Send + Sync`). Registration offers both; at most one
flavor occupies a slot. The async path pays one `async_trait` `Box` per call,
which is acceptable because the handler is I/O-bound (the Box is noise next to
the awaited work).

```rust
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyReadHandler: Send + Sync {
    async fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
// AsyncPropertyWriteHandler, AsyncActionHandler — opt-in, behind `async`.
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

Because the driving layer is async, `ClientBinding::invoke` / `subscribe` are
`async fn` (resolved A1), and `ServerBinding::poll_accept` returns a **boxed
future** (`Pin<Box<dyn Future<Output = InboundRequest> + Send + '_>>`) for
`dyn`-compatibility — the Servient stores `Vec<Arc<dyn ServerBinding>>` and
`select_all`s their `poll_accept` futures (resolved A3). The dispatcher calls a
sync handler directly (no allocation) or awaits an opt-in async handler
(§4.2); the binding no longer needs a separate sync accept path. The
`poll_accept_sync` / `AsyncServerBinding` split (addendum §6.2, §9.6) collapses
to a single boxed-future `poll_accept`. The std zenoh backend implements it via
a `tokio::mpsc` accept queue (waker-driven); the zenoh-pico backend's model is
deferred with its hardware platform. The sync driving primitive (§7.2) manually
polls the `poll_serve` future.

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
  `DirectorySession`, `CountMode`, `ConsistencyMode`, `ProjectionMode` — not a
  local CRUD container.
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
after `expose()`. Handlers are attached between `produce()` and `expose()`
(this is the Scripting API produce→configure→expose flow). `expose()` registers
all inbound routes wholesale and publishes the TD; the TD is immutable
thereafter until `destroy()`. `destroy()` from within a Thing's own handler
uses the deferred-removal rule (v3.0 §7), retained.

The dynamic-affordance network propagation (addendum §9.2), directory
re-publish-on-mutation, and `register_affordance`/`unregister_affordance` are
all removed.

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
- **Handler invocation** clones one `Arc<dyn Handler>` out of a per-Thing
  handler-set map under a brief lock, releases the lock, then invokes. The
  primary sync handler path is a direct virtual call — **zero per-interaction
  heap allocation**. The opt-in async handler path pays one `async_trait` `Box`
  per call (acceptable: the handler is I/O-bound).
- **Outbound form/binding plan** is interned in the consumed registry entry
  (addendum §9.4 retained); repeated consumed interactions reuse the cached
  binding instance via `Arc` clone — no `make_binding`, no plan recompute.
- **Event fan-out** shares `Payload` bytes via `Arc<[u8]>` (retained); media
  metadata may move to `Arc<str>` if profiling warrants (deferred #1).
- **Lock contention** is bounded by the two-level model: registry lock is
  coarse but rare (expose/destroy only); per-Thing lock is brief. The single
  unified lock primitive removes the `multithread` feature coordination cost.
- **Directory queries** are continuation-based (one batch + token), not
  full-table scan with `offset+total` (discovery refactor).

## 12. Sequencing

The refactor is sequenced to keep the workspace compiling at each phase:

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
| D6 | Handler storage | One consolidated handler-set per affordance; sync traits primary, async twins opt-in per Scripting API method. (§4.2) |
| D7 | Discovery | Execute the Introduction/Exploration/session refactor; `Servient` holds `Discoverer` trait object. (§6, §7.1) |
