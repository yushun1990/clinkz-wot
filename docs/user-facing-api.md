# clinkz-wot User-Facing API — Frozen Boundary Draft

> Status: **Draft v0.1** — external API boundary spec, presented before
> implementation refactoring. Signatures here are the contract; internals
> (`ClientBinding`, `ServerBinding`, `ClientBindingFactory`) retreat behind
> this surface and may be reorganized freely as long as the contract holds.
>
> Alignment target: W3C WoT Scripting API (Consumer/Producer/Discovery User
> Agent conformance), expressed in Rust idiom (`Result` not throw, `async fn`
> not Promise, owned buffers, `Arc<[T]>` for shared slices).

## 1. Goals and Scope

This document fixes the surface an application developer (and a binding
author) programs against. Anything not listed here is **not** public API.

### Goals

- One entry point per concern: `ServientBuilder` for composition,
  `Servient` for the runtime facade, `ConsumedThingHandle` /
  `ExposedThingHandle` for interaction, `ProtocolBinding` for binding
  authors.
- Method catalogue, parameter semantics, and error model follow the W3C WoT
  Scripting API. Rust syntax only — no JS naming (`camelCase` → `snake_case`).
- The engine-internal trait split (`ClientBinding` outbound vs `ServerBinding`
  inbound, async-vs-sync, per-instance vs singleton) is **not visible** to
  application code. It exists for binding authors and lives in
  `clinkz_wot_core`.
- `no_std + alloc` path remains viable for TD/TM construction, validation,
  local dispatch, and abstract transport adapters. The full Servient requires
  `async` (see §10).

### Non-goals

- Verbatim JS WebIDL reproduction. Rust idiom wins where they conflict
  (already decided by `docs/wot-compliance.md` §"Scripting API Boundary").
- Removing the internal `ClientBinding` / `ServerBinding` trait split. They
  stay; they just stop being part of the user-facing surface.
- A new umbrella crate. The user entrypoint stays `clinkz_wot_servient`
  (re-exporting from `clinkz_wot_core` as needed). A future facade crate is
  possible but out of scope here.

## 2. Layered API Model

```
                ┌───────────────────────────────────────────────┐
   Application  │  ServientBuilder · Servient                   │
   code         │  ConsumedThingHandle · ExposedThingHandle     │
                │  ThingDiscoveryProcess                        │
                └────────────────┬──────────────────────────────┘
                                 │  (frozen boundary)
   ──────────────────────────────┼──────────────────────────────
   Binding                       │
   authors      ┌────────────────▼──────────────────────────────┐
                │  ProtocolBinding  (clinkz_wot_core)            │
                └────────────────┬──────────────────────────────┘
                                 │  (engine-internal, pub in core)
                ┌────────────────▼──────────────────────────────┐
   Engine       │  ClientBinding · ServerBinding · ConsumedThing │
   internals    │  ExposedThing · EventBroker · Dispatch         │
                └───────────────────────────────────────────────┘
```

Three layers, three audiences:

| Layer | Audience | Crate |
|---|---|---|
| Application API | App developers | `clinkz_wot_servient` |
| Binding facade | Binding authors | `clinkz_wot_core` (`ProtocolBinding`) |
| Engine internals | Engine maintainers | `clinkz_wot_core` (the rest) |

`ClientBinding`, `ServerBinding`, `ClientBindingFactory` remain `pub` in
`clinkz_wot_core` because binding authors need them to implement
`ProtocolBinding`. **They stop being re-exported from
`clinkz_wot_servient`'s public surface.** Application code that touches
them is depending on internals.

## 3. Crate Topology and Dependency Direction

```
            ┌────────────────────────────┐
            │  clinkz_wot_servient        │
            │  (app entrypoint)           │
            └────────────┬───────────────┘
                  depends on │
            ┌──────▼─────────┐
            │  clinkz_wot_core│  ← defines ProtocolBinding,
            └──────┬─────────┘     ClientBinding, ServerBinding
       depends on  │
   ┌───────────────┴┬──────────────────────────────┐
   │                │                              │
clinkz_wot_td   clinkz_wot_discovery   clinkz_wot_protocol_bindings_*
```

- `ProtocolBinding` lives in **`clinkz_wot_core`** alongside
  `ClientBinding` / `ServerBinding`. All binding-related traits colocated.
- Concrete binding crates (e.g. `clinkz_wot_protocol_bindings_zenoh`) depend
  on `clinkz_wot_core` to implement `ProtocolBinding`. They do **not** depend
  on `clinkz_wot_servient`.
- `clinkz_wot_servient` depends on `clinkz_wot_core` (uses `ProtocolBinding`
  in its builder). No cycle.

## 4. Layer 1 — Binding Facade

### 4.1 `ProtocolBinding` trait

Lives in `clinkz_wot_core`, re-exported from `clinkz_wot_servient` so binding
authors can import from either crate.

```rust
// clinkz_wot_core::binding_facade (new module)
//
// The single trait a concrete protocol binding (zenoh, http, mqtt, ...)
// implements. Application code never calls these methods directly; the
// Servient extracts the client/server adapters at build time.

pub trait ProtocolBinding: Send + Sync {
    /// Human-readable protocol identifier, e.g. `"zenoh"`, `"http"`.
    /// Used for diagnostics and form-selection logging only.
    fn protocol(&self) -> &str;

    /// Returns a fresh client-side binding factory, or `None` for
    /// pure-exposer bindings (e.g. a sensor that never consumes remote
    /// Things). The Servient invokes `build()` once per consumed Thing.
    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>>;

    /// Returns the shared server-side binding, or `None` for pure-consumer
    /// bindings (e.g. a cloud controller that never exposes local Things).
    /// The Servient registers this once and shares it across all exposed
    /// Things.
    fn server(&self) -> Option<Arc<dyn ServerBinding>>;
}
```

### 4.2 Default implementations for the asymmetric cases

A small blanket helper is provided so binding authors do not write `None`
twice for the common one-directional cases:

```rust
// For pure-consumer bindings (client-only):
impl<T: ClientBindingFactory + Send + Sync> ProtocolBinding for ClientOnly<T> { ... }

// For pure-exposer bindings (server-only):
impl<T: ServerBinding + Send + Sync> ProtocolBinding for ServerOnly<T> { ... }
```

These wrappers are constructed via free functions:

```rust
pub fn client_only(factory: impl ClientBindingFactory + 'static) -> Box<dyn ProtocolBinding>;
pub fn server_only(server: impl ServerBinding + 'static) -> Arc<dyn ProtocolBinding>;
```

A full two-direction binding (the common case) implements `ProtocolBinding`
directly on its concrete type.

### 4.3 What application code sees

Nothing in this section. Application code only meets `ProtocolBinding`
through `ServientBuilder::with_protocol_binding` (§5.1).

## 5. Layer 2 — Servient Composition

### 5.1 `ServientBuilder`

```rust
// clinkz_wot_servient::builder  (std feature)

pub struct ServientBuilder { /* private */ }

impl ServientBuilder {
    pub fn new() -> Self;

    /// Registers a protocol binding. The Servient extracts the client
    /// factory and server singleton internally. Call once per protocol.
    ///
    /// Equivalent to the legacy `with_server_binding` +
    /// `with_client_factory` pair, collapsed into one entry.
    pub fn with_protocol_binding(mut self, binding: Arc<dyn ProtocolBinding>) -> Self;

    /// Overrides the default `LocalDiscoverer` (backed by
    /// `InMemoryDirectory`).
    pub fn with_discoverer(mut self, discoverer: Arc<dyn Discoverer>) -> Self;

    /// Assembles the Servient, calls `configure(&BindingContext)` on every
    /// server binding so each picks its dispatch model.
    pub fn build(self) -> ServientResult<Servient>;
}

impl Default for ServientBuilder { fn default() -> Self { Self::new() } }
```

The previous `with_server_binding` and `with_client_factory` are demoted to
`pub(crate)` (or `#[doc(hidden)] pub` if external mock-test support is
needed). They are **not** part of the stable surface and are removed from
the crate's public re-exports.

### 5.2 `Servient` facade

Maps 1:1 to the W3C Scripting API `WoT` interface.

```rust
// clinkz_wot_servient::servient  (async feature)

#[derive(Clone)]
pub struct Servient { /* private */ }

impl Servient {
    /// `WoT.produce(td)` — instantiate a locally-exposed Thing from its TD.
    /// The returned handle holds handler slots and TD state; routes are not
    /// installed until `expose()` is called.
    pub fn produce(&self, td: Thing) -> ServientResult<ExposedThingHandle>;

    /// `WoT.consume(td)` — instantiate a client proxy against a remote
    /// Thing's TD. The Servient pre-registers every `ProtocolBinding`'s
    /// client adapter into the returned handle.
    #[cfg(feature = "async")]
    pub fn consume(&self, td: Thing) -> ServientResult<ConsumedThingHandle>;

    /// `WoT.discover(filter)` — begins a discovery process.
    #[cfg(feature = "async")]
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess;

    /// Fetches a single TD by URL. Async because of network I/O.
    #[cfg(feature = "async")]
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> ServientResult<Thing>;

    /// Returns a handle that triggers graceful shutdown when called.
    pub fn shutdown_handle(&self) -> ShutdownHandle;
}

pub struct ShutdownHandle { /* private */ }
impl ShutdownHandle {
    pub fn shutdown(&self);
}
```

### 5.3 Typical composition

```rust
use clinkz_wot_servient::{ServientBuilder, ProtocolBinding};
use clinkz_wot_protocol_bindings_zenoh::ZenohBinding;

# async fn run() -> ServientResult<()> {
let zenoh = Arc::new(ZenohBinding::open(session).await?);

let servient = ServientBuilder::new()
    .with_protocol_binding(zenoh)
    // .with_protocol_binding(Arc::new(HttpBinding::bind(addr)?))
    .build()?;

let sensor = servient.produce(sensor_td)?;
sensor.set_property_read_handler("temperature", MyHandler)?;
sensor.expose().await?;

let lamp = servient.consume(lamp_td)?;
lamp.write_property("on", InteractionOptions::with_data(...)).await?;
# Ok(()) }
```

## 6. Layer 3 — Thing Interaction

### 6.1 `ConsumedThingHandle` — full Scripting API catalogue

All methods are `async` because they drive `ClientBinding::invoke` /
`subscribe` over the network.

```rust
// clinkz_wot_servient::handle  (async feature)

pub struct ConsumedThingHandle { /* private */ }

impl ConsumedThingHandle {
    // --- identity / metadata ---

    pub fn id(&self) -> &ThingId;
    pub fn thing_description(&self) -> &Thing;

    // --- one-shot property ops ---

    pub async fn read_property(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<InteractionOutput>;

    pub async fn write_property(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<InteractionOutput>;

    // --- bulk property ops (Scripting API §6.5) ---

    pub async fn read_all_properties(
        &self, options: InteractionOptions,
    ) -> CoreResult<InteractionOutput>;

    pub async fn read_multiple_properties(
        &self, names: &[&str], options: InteractionOptions,
    ) -> CoreResult<InteractionOutput>;

    pub async fn write_multiple_properties(
        &self, entries: &BTreeMap<&str, Payload>,
        options: InteractionOptions,
    ) -> CoreResult<()>;

    // --- action ops ---

    pub async fn invoke_action(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<InteractionOutput>;

    // --- observable property ops (Scripting API §6.6) ---

    /// Opens a long-lived subscription to property changes and returns a
    /// `Subscription` implementing `futures_core::Stream<Item = Payload>`.
    pub async fn observe_property(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<Subscription>;

    pub async fn unobserve_property(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<()>;

    // --- event ops (Scripting API §6.7) ---

    pub async fn subscribe_event(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<Subscription>;

    pub async fn unsubscribe_event(
        &self, name: &str, options: InteractionOptions,
    ) -> CoreResult<()>;

    /// Subscribes to every event declared by the consumed Thing.
    /// Returns a merged `Subscription` whose stream yields `(name, Payload)`.
    pub async fn subscribe_all_events(
        &self, options: InteractionOptions,
    ) -> CoreResult<EventStream>;
}
```

**Gaps closed vs current source** (`servient/src/handle.rs:202-350`):
`observe_property`, `unobserve_property`, `subscribe_event`,
`unsubscribe_event`, `read_all_properties`, `read_multiple_properties`,
`write_multiple_properties`, `subscribe_all_events`. The plumbing
(`ClientBinding::subscribe`, `Subscription`, `Subscription::merge`) already
exists; only the handle surface was missing.

**New helper type**:

```rust
/// Merged event stream yielding `(EventName, Payload)` tuples.
/// Implements `futures_core::Stream`.
pub struct EventStream { /* private */ }
```

### 6.2 `ExposedThingHandle` — handler attachment, lifecycle, local dispatch

```rust
// clinkz_wot_servient::handle  (async feature)

pub struct ExposedThingHandle { /* private */ }

impl ExposedThingHandle {
    // --- identity / metadata ---

    pub fn id(&self) -> &ThingId;
    pub fn thing_description(&self) -> Thing;  // clones

    // --- sync handler setters (Scripting API §7.3) ---
    //
    // Replaceable throughout produce → expose → destroy (AD14). The TD
    // affordance set is frozen at `expose()` (decision 2): no add/remove
    // post-produce.

    pub fn set_property_read_handler(
        &self, name: impl Into<String>, handler: impl PropertyReadHandler + 'static,
    );
    pub fn set_property_write_handler(
        &self, name: impl Into<String>, handler: impl PropertyWriteHandler + 'static,
    );
    pub fn set_property_observe_handler(
        &self, name: impl Into<String>, handler: impl PropertyObserveHandler + 'static,
    );
    pub fn set_property_unobserve_handler(
        &self, name: impl Into<String>, handler: impl PropertyUnobserveHandler + 'static,
    );
    pub fn set_action_handler(
        &self, name: impl Into<String>, handler: impl ActionHandler + 'static,
    );
    pub fn set_action_query_handler(
        &self, name: impl Into<String>, handler: impl ActionQueryHandler + 'static,
    );
    pub fn set_action_cancel_handler(
        &self, name: impl Into<String>, handler: impl ActionCancelHandler + 'static,
    );
    pub fn set_event_subscribe_handler(
        &self, name: impl Into<String>, handler: impl EventSubscribeHandler + 'static,
    );
    pub fn set_event_unsubscribe_handler(
        &self, name: impl Into<String>, handler: impl EventUnsubscribeHandler + 'static,
    );

    // --- async handler setters (mirror the sync set; for I/O-bound handlers) ---
    //
    // Gated on `#[cfg(feature = "async")]`. Both flavours are registerable
    // on the same handle; per-affordance the last setter wins.

    #[cfg(feature = "async")]
    pub fn set_async_property_read_handler(
        &self, name: impl Into<String>, handler: impl AsyncPropertyReadHandler + 'static,
    );
    // ... 8 more, one per sync setter above ...

    // --- lifecycle ---

    /// Registers routes on every server binding, inserts into the servable
    /// registry, publishes the TD. Multi-binding rollback on failure
    /// (E12/AD27). Freezes the TD affordance set.
    pub async fn expose(&self) -> ServientResult<()>;

    /// Quiescing teardown (AD15): unregisters routes, drains in-flight,
    /// removes the registry entry, unpublishes. Idempotent (AD27/E13).
    pub async fn destroy(&self) -> ServientResult<()>;

    // --- local (server-side) interaction ---
    //
    // Pass-through dispatch through the registered handler. Two flavours
    // because handlers themselves can be sync or async. Use the `_async`
    // variants when any handler in the call chain is async; the sync
    // variants work for handlers registered via `set_*_handler` only.

    pub fn read_property(
        &self, name: &str, input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    pub fn write_property(
        &self, name: &str, input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    pub fn invoke_action(
        &self, name: &str, input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    pub fn query_action(
        &self, name: &str, input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    pub fn cancel_action(
        &self, name: &str, input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    pub fn observe_property(
        &self, name: &str, input: &InteractionInput, push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput>;
    pub fn unobserve_property(
        &self, name: &str, input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    pub fn subscribe_event(
        &self, name: &str, input: &InteractionInput, push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput>;
    pub fn unsubscribe_event(
        &self, name: &str, input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    #[cfg(feature = "async")]
    pub async fn read_property_async(
        &self, name: &str, input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
    // ... 8 more `*_async` local-dispatch variants ...

    // --- emit (server-side push) ---

    /// Fans an event payload out to registered subscribers via the broker.
    pub fn emit_event(&self, name: &str, payload: Payload) -> CoreResult<()>;

    /// Fans a property-change payload out via the broker (same path as events).
    pub fn emit_property_change(&self, name: &str, payload: Payload) -> CoreResult<()>;
}
```

**Gaps closed vs current source** (`servient/src/handle.rs:29-193`):
9 `set_async_*` setters, `query_action`, `cancel_action`, `observe_property`,
`unobserve_property`, `subscribe_event`, `unsubscribe_event` (sync + async
local dispatch).

## 7. Handler Contract

Handler traits live in `clinkz_wot_core::thing` (sync) and are re-exported
from `clinkz_wot_servient` so application code does not import from `core`
directly. Same for the `Async*` twins.

### 7.1 Sync handler traits

```rust
pub type PushFn<'a> = &'a mut dyn FnMut(Payload) -> CoreResult<()>;

pub trait PropertyReadHandler: Send + Sync {
    fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait PropertyWriteHandler: Send + Sync {
    fn write(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait PropertyObserveHandler: Send + Sync {
    fn observe(&self, input: &InteractionInput, push: PushFn<'_>) -> CoreResult<InteractionOutput>;
}
pub trait PropertyUnobserveHandler: Send + Sync {
    fn unobserve(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait ActionHandler: Send + Sync {
    fn invoke(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait ActionQueryHandler: Send + Sync {
    fn query(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait ActionCancelHandler: Send + Sync {
    fn cancel(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}
pub trait EventSubscribeHandler: Send + Sync {
    fn subscribe(&self, input: &InteractionInput, push: PushFn<'_>) -> CoreResult<InteractionOutput>;
}
pub trait EventUnsubscribeHandler: Send + Sync {
    fn unsubscribe(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
```

### 7.2 Async handler traits (mirror, `#[async_trait]`)

```rust
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyReadHandler: Send + Sync {
    async fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}
// ... 8 more, one per sync trait ...
```

Both flavours are registerable on the same `ExposedThingHandle`. Last write
wins per affordance. The handler set is the only mutable state on the
otherwise-`Send + Sync` handle.

## 8. Discovery Surface

```rust
// clinkz_wot_discovery  (re-exported from clinkz_wot_servient)

pub struct DiscoveryFilter { /* fields below */ }
impl DiscoveryFilter {
    pub fn all() -> Self;
    pub fn new(filter: DirectoryFilter) -> Self;
    pub fn with_count(self, mode: CountMode) -> Self;
    pub fn into_filter(self) -> DirectoryFilter;
    pub fn count_mode(&self) -> CountMode;
}
impl Default for DiscoveryFilter { fn default() -> Self { Self::all() } }

#[async_trait]
pub trait Discoverer: Send + Sync {
    fn discover(&self, filter: DiscoveryFilter) -> DiscoveryResult<ThingDiscoveryProcess>;
    fn explore_directory(
        &self, dir: DirectoryRef, query: DirectoryQuery,
    ) -> DiscoveryResult<ThingDiscoveryProcess>;
    async fn request_thing_description(&self, url: &AbsoluteUri) -> DiscoveryResult<Thing>;
}

pub struct ThingDiscoveryProcess { /* private */ }
impl ThingDiscoveryProcess {
    pub async fn next(&mut self) -> DiscoveryResult<Option<Thing>>;
    pub async fn stop(&mut self) -> DiscoveryResult<()>;
    pub fn error(&self) -> Option<&DiscoveryError>;
}

#[non_exhaustive]
pub enum DirectoryRef { Local, Url(AbsoluteUri) }
```

`Servient::discover` returns a `ThingDiscoveryProcess`; the caller drains it
via `next()` in a `while let` loop. This matches W3C Scripting API §5.

## 9. Cross-Cutting Types

### 9.1 Interaction model

```rust
pub struct InteractionInput {            // inbound to a handler
    pub data: Option<Payload>,
    pub uri_variables: BTreeMap<String, String>,
    pub principal: Option<Principal>,
    pub accept: Option<AcceptHint>,
}
impl InteractionInput {
    pub fn empty() -> Self;
    pub fn with_data(data: Payload) -> Self;
}

pub struct InteractionOptions {          // outbound call options
    pub uri_variables: BTreeMap<String, String>,
    pub form_index: Option<usize>,       // force a specific form
    pub data: Option<Payload>,
    pub timeout: Option<Duration>,
}
impl InteractionOptions {
    pub fn new() -> Self;
    pub fn with_data(data: Payload) -> Self;     // convenience
    pub fn with_uri_variable(k: &str, v: &str) -> Self;  // builder
}

#[non_exhaustive]
pub enum InteractionStatus { Ok, Created, Accepted }

pub struct InteractionOutput {
    pub data: Option<Payload>,
    pub status: InteractionStatus,
}
impl InteractionOutput {
    pub fn empty() -> Self;
    pub fn with_data(data: Payload) -> Self;
}
```

`InteractionOptions::with_data` and `with_uri_variable` are **new builder
conveniences** (current source only exposes bare fields). They keep call
sites readable:

```rust
handle.write_property("on", InteractionOptions::with_data(payload)).await?;
```

### 9.2 Payload

```rust
pub struct Payload {
    pub body: Arc<[u8]>,
    pub content_type: String,
    pub content_coding: Option<String>,
}
impl Payload {
    pub fn new(body: impl Into<Arc<[u8]>>, content_type: impl Into<String>) -> Self;
    pub fn with_content_coding(self, coding: impl Into<String>) -> Self;
}
```

### 9.3 Subscription (consumer-side streaming)

```rust
pub struct Subscription { /* private */ }
impl Subscription {
    pub fn channel(capacity: usize) -> (SubscriptionSender, Self);
    pub fn merge(subs: Vec<Subscription>) -> Self;
    pub fn poll_next(&self) -> Option<Payload>;     // no_std path
    pub fn stop(&self);
    pub fn is_stopped(&self) -> bool;
    pub fn overflow_count(&self) -> u64;
    pub fn capacity(&self) -> usize;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
#[cfg(feature = "async")]
impl futures_core::Stream for Subscription { type Item = Payload; }
```

### 9.4 Errors

The current split (`CoreError`, `ServientError`, `BindingError`,
`SecurityError`, `DiscoveryError`) is preserved as-is for v0.1. A future
unified facade error tree is listed in §11.

Conversion direction:

```
SecurityError  ─┐
BindingError   ─┼─►  CoreError  ─┐
DiscoveryError ─┘                ├──►  ServientError
                                 │      (the top-level Result the user sees)
CoreError ───────────────────────┘
```

`ServientResult<T>` is the type the application layer receives from
`Servient::*` and `*ThingHandle::*`. `CoreResult<T>` is returned by handler
traits and by local-dispatch methods; the Servient wraps it via
`From<CoreError> for ServientError`.

## 10. Feature Flags and `no_std` Posture

| Flag | Effect |
|---|---|
| *(default)* | `no_std + alloc` TD/TM construction, validation, abstract dispatch. |
| `async` | Enables `Servient`, handles, `ConsumedThing`, async handler traits. Required for any network runtime. On `no_std` this means `no_std + async` (embassy). |
| `std` | Implies `async`. Enables `ServientBuilder`, `LocalDiscoverer`, std-only transport bindings. |

`ProtocolBinding` itself is `no_std + alloc`-compatible. A pure-`no_std`
binding can implement it without `std`. The Servient composition layer
requires `async`.

## 11. Migration Delta From Current Source

> Status: P0 (`ProtocolBinding` facade + `with_protocol_binding` entry) and
> P1 (legacy hooks retired) have landed. P2/P3 still pending.

| Item | Current | Target | Status |
|---|---|---|---|
| `ProtocolBinding` trait | absent (removed in redesign) | added in `clinkz_wot_core` | ✅ P0 |
| `ServientBuilder::with_protocol_binding` | absent | added as primary entry | ✅ P0 |
| `with_server_binding` / `with_client_factory` | `pub` | deleted (no in-tree caller post-migration) | ✅ P1 |
| `ClientBindingFactory` re-exported from servient | yes (`lib.rs:34`) | removed from re-exports | ✅ P1 |
| `ConsumedThingHandle` streaming ops | absent | 8 methods added (§6.1) | ⏳ P2 |
| `ExposedThingHandle` async setters | absent | 9 `set_async_*` added (§6.2) | ⏳ P3 |
| `ExposedThingHandle` local dispatch | 3 ops (`read/write/invoke`) | 9 ops (sync + 9 `_async`) (§6.2) | ⏳ P3 |
| `InteractionOptions::with_data` / `with_uri_variable` | bare fields only | builder conveniences added | ⏳ P3 |
| Unified facade error tree | split across 4 enums | deferred to follow-up (§9.4) | — |

## 12. Open Questions (deferred, not blocking v0.1)

1. Should `read_all_properties` / `read_multiple_properties` fan out across
   multiple bindings in parallel, or stay sequential? Current
   `affordance_form` picks one form per affordance; parallelism would need
   `join_all`.
2. `subscribe_all_events` return shape: flat `EventStream` yielding
   `(name, Payload)`, or a `BTreeMap<String, Subscription>`? Scripting API
   is silent; flat is simpler for `while let` loops.
3. Whether to expose a top-level `clinkz_wot` umbrella crate that re-exports
   `servient + td + protocol-bindings-zenoh` so users `use clinkz_wot::*`
   once. Convenience win, but couples release cadence.
4. `ProtocolBinding::protocol()` returning `&str` vs a typed `ProtocolId`
   newtype. `&str` is simpler; newtype enables stable matching and
   diagnostics.
5. Whether `with_server_binding` / `with_client_factory` should be kept as
   `#[doc(hidden)] pub` (external mock tests) or fully `pub(crate)`.
