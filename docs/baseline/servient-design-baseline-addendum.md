# Servient Runtime Design Baseline — Implementation Type Refinements (v3.1)

This document is the implementation-time addendum to
`docs/baseline/servient-design-baseline.md` (v3.0). v3.0 locked the five OPEN
items (concurrency, inbound security, subscription data flow, `expose()`
coordination, inbound request shape) at the **decision** level. This addendum
locks the **concrete types and sequencing details** that v3.0 left as
placeholders, prose, or examples, so that `docs/plan/servient-runtime-redesign-plan.md`
can be written against a fully specified surface.

These refinements are **consistent with, not reversals of**, the v3.0 LOCKED
decisions. Where a v3.0 statement was illustrative, this addendum makes it
normative. The signatures below are normative for the redesign; field names are
stable, exact trait bounds follow Section 7 of v3.0 and Rust idiom.

Scope of refinements:

- Section 1: Concrete types for the placeholders in v3.0 §8 and §11
  (`Principal.id`, `SecurityError`, `CorrelationId`, `AuthMaterial`).
- Section 2: Owned request/response model (resolves the borrow-vs-own tension
  between v3.0 §2 `&self` + spawnable `serve(self)` and the current borrowed
  `AffordanceTarget<'a>` / `BindingRequest<'a>`).
- Section 3: Directory-driven invalidation trigger mechanism (v3.0 §5.2 assumed
  a Directory that "reports" changes, which the current `ThingDirectory` trait
  cannot do).
- Section 4: `expose(td)` handler-attachment sequencing (v3.0 §3 / §6 / §10).
- Section 5: Error taxonomy (v3.0 §4 driving layer, §8 security).
- Section 6: Driving-layer clarifications (v3.0 §4, §9 queue capacity).

## 1. Concrete Core Types [LOCKED]

All types in this section live in `clinkz-wot-core` and are `no_std + alloc`.
They use owned data (`String`, `Vec<u8>`) so they are `'static` and usable
across `serve(self)` spawns (Section 2).

### 1.1 Identity and correlation

```rust
/// Canonical Thing identity. Replaces bare `String` ids across core,
/// discovery, and servient.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThingId(String);

/// Opaque, core-owned correlation token. A binding fills it from its
/// transport (for example a zenoh query id) and echoes it unchanged in the
/// matching `InboundResponse`. It is owned by core, not "defined by the
/// binding" — bindings only populate the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationId(Vec<u8>);
```

`ThingId` and `CorrelationId` are newtypes with private inner fields and
`new` / `as_str` / `as_bytes` / `into_*` accessors plus ergonomic `From`
conversions (`From<String>`, `From<&str>` for `ThingId`; `From<u64>`,
`From<Vec<u8>>` for `CorrelationId`). This refines v3.0 §11, which wrote
`CorrelationId` as "an opaque newtype the binding defines" — that wording is
superseded: `CorrelationId` is **core-owned**, and the binding only supplies
its byte content.

### 1.2 Auth material and principal

```rust
/// Transport-level credentials extracted by a binding and consumed by
/// `SecurityProvider::verify`. These are raw extractions; verification
/// happens in `verify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMaterial {
    /// Transport peer locator/identifier (for example a zenoh peer id).
    PeerId(String),
    /// Raw bearer token bytes (for example an Authorization header value).
    BearerToken(Vec<u8>),
    /// Raw certificate fingerprint bytes.
    CertificateFingerprint(Vec<u8>),
    /// Forward-compatible opaque carrier for schemes not yet enumerated.
    Other(Vec<u8>),
}

/// Established principal identity produced by a successful `verify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalId(String);

/// Identity established for an inbound caller after verification.
/// Refines v3.0 §8 `Principal { id: /* opaque identity */ }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub id: PrincipalId,
    pub scopes: Vec<String>,
}
```

`PrincipalId` is a newtype with private inner field and accessors. `Principal`
fields are public, matching v3.0 §8 intent; the identity type is
`PrincipalId`, not a bare placeholder.

### 1.3 Security error

```rust
/// Failure reported by `SecurityProvider::verify`. Defined in core so the
/// inbound dispatcher can propagate it through `CoreError` (Section 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// No auth material was supplied where the scheme requires it.
    MissingCredentials,
    /// Auth material was supplied but did not validate.
    InvalidCredentials,
    /// The matched security scheme is not supported by this provider.
    UnsupportedScheme,
    /// The principal lacks a required scope.
    ScopeDenied {
        required: Vec<String>,
        present: Vec<String>,
    },
    /// Scheme- or transport-specific failure with an opaque English reason.
    SchemeFailure(String),
}
```

`From<SecurityError> for CoreError` is provided (Section 5).

## 2. Owned Request/Response Model [LOCKED]

v3.0 §2 specifies `ClientBinding::invoke(&self, BindingRequest)` and §4
specifies `serve(self) -> impl Future<Output = ()> + Send + 'static`. The
current core types `AffordanceTarget<'a>` (`core/src/thing.rs:14`, with
`&'a str` variants) and `BindingRequest<'a>` (`core/src/binding.rs:8`, borrowing
`&'a Thing` and `&'a Form`) are **borrowed** and therefore cannot cross a
spawnable future boundary. They are made **owned** in the redesign.

### 2.1 AffordanceTarget — owned

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AffordanceTarget {
    Thing,
    Property(String),
    Action(String),
    Event(String),
}
```

This drops the lifetime and the `Copy` impl (it is now `Clone`). All existing
call sites that built `AffordanceTarget::Property("x")` move to
`AffordanceTarget::Property("x".into())`. This is part of the one-shot
breaking refactor (v3.0 §0).

### 2.2 BindingRequest — owned, shared Thing/Form

```rust
pub struct BindingRequest {
    pub thing: Arc<Thing>,
    pub target: AffordanceTarget,
    pub operation: Operation,
    pub form: Arc<Form>,
    pub input: InteractionInput,
}
```

`Thing` and `Form` are carried as `Arc` rather than by-value so that handles
(which cache the canonical TD and selected form) can hand out cheap clones
without cloning the (potentially large) TD on every outbound call, and so the
request is `'static + Send + Sync`. `Arc` is available under `no_std + alloc`.
`InteractionInput` is already owned and is retained unchanged.

### 2.3 InboundRequest / InboundResponse — owned, no Thing/Form on request

```rust
pub struct InboundRequest {
    pub thing_id: ThingId,
    pub target: AffordanceTarget,
    pub operation: Operation,
    pub input: InteractionInput,
    pub auth: Option<AuthMaterial>,
    pub correlation: CorrelationId,
}

pub struct InboundResponse {
    pub output: InteractionOutput,
    pub correlation: CorrelationId,
}
```

Consistent with v3.0 §11: the inbound request does **not** carry the `Thing` or
the matched `Form`. The dispatcher resolves the `Thing` from the exposed
registry by `thing_id`, resolves the matched `Form` internally (for security
scheme lookup per §8), and never exposes the `Form` to handlers. `InteractionOutput`
is already owned and is retained unchanged.

### 2.4 Binding trait split — owned receivers

```rust
/// Outbound interactions (consuming remote Things). v3.0 §2.
pub trait ClientBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool;
    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput>;
}

/// Inbound request source (serving exposed Things). v3.0 §2 / §4.
pub trait ServerBinding {
    /// Non-blocking immediate poll; returns `None` when no request is ready.
    fn poll_accept_sync(&self) -> Option<InboundRequest>;
    /// Native-async accept; pending until a request arrives.
    fn poll_accept(&self) -> impl Future<Output = InboundRequest>;
}
```

The single current `ProtocolBinding` trait (`&mut self`, outbound only) is
removed. `invoke`/`poll_accept*` take `&self`; each concrete binding owns its
interior mutability for I/O state (v3.0 §7). The `supports` predicate is
retained on `ClientBinding` for form/operation capability queries.

## 3. Directory-Driven Invalidation Trigger [LOCKED]

v3.0 §5.2 states: "When the `Directory` reports a TD change for a Thing
identity, the corresponding interned entry in `ConsumedThingRegistry` is
invalidated." The current `ThingDirectory` trait
(`discovery/src/directory.rs:42`) is a pull-only CRUD trait with no
watch/notify/stream capability, so "reports" has no mechanism today.

Resolution (split by directory locality):

- **v1 (co-located, no_std-safe):** the `Servient` mediates directory writes
  (`register` / `update` / `unregister`). After a successful directory `update`
  or `delete` for a Thing id, the `Servient` calls
  `ConsumedThingRegistry::invalidate(id)` **synchronously in the same call**.
  This needs no new directory trait and works on `no_std + alloc`. The explicit
  `invalidate(id)` programmatic entry point from v3.0 §5.2 is retained and is
  the same method the Servient calls internally.
- **Deferred (remote directory observation, behind `std`):** when `D` is a
  remote Thing Description Directory client (TDs change remotely and
  asynchronously), the local Servient must observe the directory. This is added
  later via an optional `DirectoryWatch` extension trait that durable/remote
  backends implement, gated behind the `std` feature. It is out of scope for v1.

This keeps the v1 invalidation contract honest and `no_std`-safe: invalidation
is Servient-mediated and co-located, with the same `invalidate(id)` entry point
v3.0 §5.2 already named. The wording "the Directory reports a change" in v3.0
§5.2 is read, for v1, as "the Servient has just performed a directory write."

## 4. `expose(td)` Handler-Attachment Sequencing [LOCKED]

v3.0 §3 states `expose(td)` "immediately registers the inbound serving work for
that Thing" (LOCKED), §6 specifies `expose(&self, td) -> ExposedThingHandle`
(only `td`), and §10 step 1 says "validate the TD and handler set." These three
are reconciled as follows:

- `expose(td)` validates the **TD** (well-formed, has an `id`), inserts the
  `ExposedThing` entry, registers inbound routes (§10 step 3), and publishes to
  the `Directory` (§10 step 4). Serving starts immediately, per §3.
- Handlers are attached **after** `expose`, through the returned
  `ExposedThingHandle` (`set_property_handler` / `set_action_handler` /
  `set_event_handler`). v3.0 §10 step 1 wording "validate ... handler set" is
  refined: handler **completeness is not a gate** at `expose` time.
- Dispatching an interaction to an affordance that has no attached handler
  returns a structured `CoreError::MissingHandler` in the `InboundResponse`
  path (Section 5) rather than panicking or dropping the request.
- Applications are advised (non-normatively) to attach handlers immediately
  after `expose` to minimize the window in which an affordance is served but
  unhandled. v1 keeps the single-argument `expose(td)`; a future produce-then-
  activate split is not introduced.

## 5. Error Taxonomy [LOCKED]

### 5.1 Core errors

`CoreError` (`core/src/error.rs`) gains the following variants to carry the
inbound path's structured failures:

```rust
pub enum CoreError {
    // ... existing variants retained ...

    /// An inbound interaction targeted an affordance with no attached handler.
    /// Section 4.
    MissingHandler,
    /// An inbound security verification failed. Section 1.3.
    Security(SecurityError),
    /// An inbound dispatch/routing failure with an opaque English reason.
    InboundDispatch(String),
}

impl From<SecurityError> for CoreError { /* ... */ }
```

### 5.2 Servient errors

`ServientError` (`servient/src/error.rs`) gains driving-layer and route
variants and keeps its existing `From<CoreError>` conversion:

```rust
pub enum ServientError {
    // ... existing composition variants retained ...

    /// A `ServerBinding::poll_accept*` failure surfaced from the driving loop.
    Accept(String),
    /// An inbound route-registration failure during `expose` (v3.0 §10 step 3).
    RouteRegistration(String),
    /// A dispatch-level failure wrapped from core.
    Serve(CoreError),
}

impl From<CoreError> for ServientError { /* via Serve */ }
impl From<SecurityError> for ServientError { /* via Serve(Security(..)) */ }
```

The driving-loop methods `poll_serve` / `poll_serve_sync` / `serve` / `serve_sync`
(v3.0 §4) return `ServientResult<()>`; `expose` returns `ServientResult<ExposedThingHandle>`
and surfaces route-registration failures as `RouteRegistration`.

## 6. Driving-Layer and Subscription Clarifications [LOCKED]

### 6.1 Outbound subscription queue capacity

v3.0 §9 specifies a bounded per-subscription queue for outbound (consumed)
events with drop-oldest + overflow-counter backpressure. The queue capacity is:

- **Configurable per subscription**, passed to the subscribe call as a capacity
  parameter (with a crate-level default constant when omitted).
- On `no_std`, realized via `heapless::spsc::Queue` with the capacity fixed at
  subscription creation (heapless requires a `const` generic). Per-subscription
  capacity is therefore bounded at construction time on `no_std`.
- On `std`, realized via `flume` / `tokio::mpsc` with the same per-subscription
  capacity.

The drop-oldest + overflow-counter policy (v3.0 §9) applies identically in both
builds and is observable on the `Subscription` handle for diagnostics.

### 6.2 `serve_sync` scope

v3.0 §4 retains both `poll_serve_sync(&self) -> ServientResult<()>` and
`serve_sync(&self) -> !`. Clarification: on a bare `no_std` MCU super-loop,
**`poll_serve_sync` is the primary driving primitive** — the application calls
it once per super-loop iteration alongside its other work. The synchronous
primitive is **stepwise**: one `poll_serve_sync` call processes at most one
inbound request across all registered sync bindings, rather than draining one
binding's entire pending queue.

This keeps the sync flavor aligned with the mental model of native-async
`poll_serve()`:

- one call advances the serving state by one inbound request;
- callers remain in control of outer scheduling;
- multi-binding fairness is improved by resuming polling from the binding after
  the one that most recently produced work.

`serve_sync` remains the std host/cloud convenience wrapper that repeatedly
invokes `poll_serve_sync`, but it should be understood as a host-facing loop,
not the defining semantics of the sync flavor. Host implementations may apply
idle backoff (for example `yield_now()` followed by a short sleep after
repeated idle polls) so the loop does not busy-spin when all
`poll_accept_sync()` calls return `None`.

## 7. Deferred Items (tracked, not in v1)

The following are explicitly out of scope for the v1 redesign and remain at the
same boundaries v3.0 already placed them:

- **Remote directory observation** (`DirectoryWatch`) behind `std` (Section 3).
- **Asynchronous action completion** (HTTP/CoAP 202-style) — v1 actions are
  synchronous (v3.0 §11).
- **Full per-affordance security policy engine** — v1 does authenticate plus an
  optional scope match (v3.0 §8).
- **TD 2.0** behavior stays behind an experimental feature flag (PLAN.md M7).

## 8. Relationship to v3.0 and Next Steps

This addendum resolves every placeholder and prose-only statement in v3.0 that
blocked a concrete implementation. With Sections 1–6 locked:

1. `docs/plan/servient-runtime-redesign-plan.md` can be written against a fully
   specified surface.
2. `PLAN.md` M6 should be updated to mark its current status as superseded by
   v3.0 + this addendum and to point at the redesign plan.
3. The one-shot refactor sequence is: core inbound surface and owned types
   (Section 1–2) → `Servient<D>` collapse, interior mutability, driving layer
   (v3.0 §4–§7, this Section 4–5) → directory invalidation wiring (Section 3)
   → zenoh server side and dual backend (v3.0 §12–§13) → M7 feature-matrix and
   no-std verification alignment.

No LOCKED decision in v3.0 is changed by this addendum.
