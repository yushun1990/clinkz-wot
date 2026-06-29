> **⚠ SUPERSEDED.** This plan sequenced the now-superseded Servient baselines
> (v3.0/v3.1). The current implementation plan is `PLAN.md` phases P0–P4
> against `docs/baseline/engine-architecture-baseline.md` (v4.0). Retained as
> historical record only.

# Servient Runtime Redesign Plan

## Parent Plan Relationship

This document is a crate-level subplan under the repository-level `PLAN.md`. It
refines the one-shot Servient runtime redesign owned by:

- `clinkz-wot-core` at `core`.
- `clinkz-wot-servient` at `servient`.
- `clinkz-wot-protocol-bindings-zenoh` at `protocol-bindings/protocols/zenoh`
  (server-side surface only).

It is written against the authoritative design baseline
`docs/baseline/servient-design-baseline.md` (v3.0) and its implementation-time
refinements `docs/baseline/servient-design-baseline-addendum.md` (v3.1). Every
decision in those documents is LOCKED; this plan only sequences the work.

Parent milestones covered by this subplan:

- M3: Protocol-Neutral Core (forced inbound-surface changes).
- M4: Protocol Bindings and Zenoh Binding (zenoh server side only).
- M6: Servient Runtime (the bulk of the work).
- M7: Conformance and Embedded Support, for checks and fixtures owned by the
  redesigned crates.

Parent milestones not covered by this subplan:

- M1: TD 1.1 Hardening.
- M2: Thing Model Support.
- M5: Discovery and TDD (no new directory trait in v1; see SR-P3).

## Scope

The redesign turns the current `Servient<D, R, C, S, P>` composition shell into
the v3.0 target: a single-generic `Servient<D>`, interior-mutable `&self` API,
`Clone`, typed handles, no global `start`/`stop`, a sync/async driving layer,
two-level registry locking, an inbound serving path, directory-driven consumed-
Thing invalidation, and a zenoh server binding that shares the client session.

It is a **one-shot breaking refactor** (baseline §0): the public API changes
directly to the target shape with no deprecation shims, because M6 has no
downstream consumers yet. **Each phase below is, however, atomic**: it lands
across every affected crate in one step and keeps `cargo test --workspace`
green. Internal phasing is for verification, not for preserving the old API.

## Current Baseline (Being Superseded)

The current implementation is the starting point being replaced:

- `Servient` carries 5 generic parameters `D, R, C, S, P`
  (`servient/src/servient.rs:17`), is **not** `Clone`, and uses `&mut self` for
  nearly all operations.
- `start()` / `stop()` (`servient/src/servient.rs:88`) only flip a `running`
  boolean; they spawn no tasks and drive no accept loop.
- The only binding trait is `ProtocolBinding` (`core/src/binding.rs:22`),
  `&mut self`, outbound-only. There is no `ClientBinding` / `ServerBinding`
  split, no `InboundRequest` / `InboundResponse`, no `InboundDispatcher`, no
  `EventBroker`, no core `Subscription`, no `Principal`.
- `AffordanceTarget<'a>` (`core/src/thing.rs:14`) and `BindingRequest<'a>`
  (`core/src/binding.rs:8`) are borrowed.
- `ThingDirectory` (`discovery/src/directory.rs:42`) is pull-only CRUD; no
  change-notification surface.
- The zenoh binding is outbound-only; `ZenohBindingTransport<T>`
  (`protocol-bindings/protocols/zenoh/src/zenoh.rs:41`) implements only
  `ProtocolBinding`. `ZenohSessionTransport` does put/get/subscribe but no
  queryable, put-listener, or publisher serving.

Feature flags today: core has `default = ["std"]`, `std`; servient has
`default = ["std"]`, `std`, `test-zenoh`; zenoh binding has `default =
["zenoh"]`, `zenoh`, `zenoh-pico`.

## Current Development Sequence

1. **SR-P0** (done): core inbound surface and owned interaction types.
2. **SR-P1** (done): `Servient<D>` collapse with interior mutability, typed
   handles, and the two-level registry.
3. **SR-P2.1 + SR-P2.3** (done): sync driving layer and `expose`/`destroy`
   coordination.
4. **SR-P3** (done): Servient-mediated consumed-Thing invalidation.
5. **SR-P4** (done): zenoh server binding on the shared session.
6. **SR-P5** (done): M7 feature-matrix, no-std, and documentation alignment.
7. **SR-P2.2** (done): native-async driving layer behind the `async` feature.

Phase dependency: SR-P1 depends on SR-P0; SR-P2 depends on SR-P1; SR-P3 depends
on SR-P2; SR-P4 depends on SR-P2 (server binding) and SR-P3 (invalidation for
served Things observed through discovery); SR-P5 is continuous but finalizes
after SR-P4.

## SR-P0: Core Inbound Surface and Owned Types

Goal: deliver the v3.0 §2 / §11 and v3.1 §1–§2 core surface. This phase is
additive at first and removes the old `ProtocolBinding` at its end. It lands
across `core`, `protocol-bindings/protocols/zenoh`, and `servient` so the
workspace stays green.

### SR-P0.1 Owned Interaction Types

Status: done.

Goal: make the interaction request path `'static` so it can cross a spawnable
future (v3.1 §2).

Work items:

- Add `ThingId(String)` core newtype with accessors and `From` conversions.
- Change `AffordanceTarget` from `AffordanceTarget<'a>` (`&'a str` variants,
  `Copy`) to owned `Thing | Property(String) | Action(String) | Event(String)`
  (`Clone`, no `Copy`).
- Change `BindingRequest<'a>` to an owned `BindingRequest` carrying
  `Arc<Thing>` and `Arc<Form>` plus owned `AffordanceTarget`, `Operation`,
  `InteractionInput`.
- Update every call site in `core`, `servient`, the shared binding crate, and
  the zenoh binding that built `AffordanceTarget::Property("x")` or borrowed
  `BindingRequest<'a>`.
- Keep `InteractionInput` / `InteractionOutput` unchanged (already owned).

Acceptance criteria:

- `cargo test --workspace` passes.
- `cargo check -p clinkz-wot-core --no-default-features` passes.
- No borrowed lifetime remains on `AffordanceTarget` or `BindingRequest`.

### SR-P0.2 Concrete Inbound and Security Types

Status: done.

Goal: add the v3.1 §1 types so the inbound path is fully specified.

Work items:

- Add `CorrelationId(Vec<u8>)` core newtype with `From<u64>`, `From<Vec<u8>>`,
  and `as_bytes` / `into_bytes` accessors.
- Add `AuthMaterial` enum (`PeerId(String)`, `BearerToken(Vec<u8>)`,
  `CertificateFingerprint(Vec<u8>)`, `Other(Vec<u8>)`).
- Add `PrincipalId(String)` newtype and `Principal { id, scopes }`.
- Add `SecurityError` enum (`MissingCredentials`, `InvalidCredentials`,
  `UnsupportedScheme`, `ScopeDenied { required, present }`,
  `SchemeFailure(String)`).
- Add `From<SecurityError> for CoreError`.
- Add owned `InboundRequest { thing_id, target, operation, input, auth,
  correlation }` and `InboundResponse { output, correlation }`.

Acceptance criteria:

- `cargo test --workspace` passes.
- `cargo check -p clinkz-wot-core --no-default-features` passes.
- Unit tests cover `CorrelationId` round-trip, `AuthMaterial` variants, and
  `SecurityError` → `CoreError` conversion.

### SR-P0.3 Binding Trait Split and Inbound Dispatcher

Status: done.

Goal: replace the single outbound `ProtocolBinding` with the v3.1 §2.4
`ClientBinding` / `ServerBinding` split and add the `InboundDispatcher`.

Work items:

- Add `ClientBinding` with `supports(&self, ...)` and
  `invoke(&self, BindingRequest) -> CoreResult<InteractionOutput>` (both
  `&self`).
- Add `ServerBinding` with `poll_accept_sync(&self) -> Option<InboundRequest>`
  and `poll_accept(&self) -> impl Future<Output = InboundRequest>`.
- Add `InboundDispatcher::dispatch(&self, InboundRequest) ->
  CoreResult<InboundResponse>`. The dispatcher resolves the `Thing` from the
  exposed registry by `thing_id`, resolves the matched `Form` internally (for
  security scheme lookup), and never exposes the `Form` to handlers.
- Remove `ProtocolBinding` and its `Box<dyn>` forwarding impl. Update
  `BoundConsumedThing` (`core/src/thing.rs`) and servient to use
  `ClientBinding`.
- Migrate the zenoh binding's `ZenohBindingTransport<T>` from
  `ProtocolBinding` to `ClientBinding` (server side comes in SR-P4).

Acceptance criteria:

- `cargo test --workspace` passes.
- `cargo check -p clinkz-wot-core --no-default-features` passes.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  passes.
- No reference to `ProtocolBinding` remains in source.

### SR-P0.4 EventBroker and Subscription

Status: done.

Goal: add the v3.0 §9 / v3.1 §6.1 event fan-out and outbound subscription
surface in core.

Work items:

- Add `PublisherSink` trait (`fn publish(&self, payload: &Payload) ->
  CoreResult<()>`), stored as `Box<dyn PublisherSink + Send + Sync>` (async) /
  `+ Send` (sync) via a `pub(crate)` cfg-selected alias.
- Add `EventBroker` holding `Map<(ThingId, EventName), Vec<Box<dyn
  PublisherSink + ...>>>`. Local `EventHandler::subscribe` is invoked once with
  a broker-backed `EventSink`; each `emit` fans out to every registered
  `PublisherSink`.
- Add a `Subscription` handle holding an `Arc` to a bounded per-subscription
  queue, exposing `poll_next() -> Option<Payload>` (sync), a `Stream` impl
  (async), `stop()`, an overflow counter, and `Clone` (+ `Send + Sync` async).
- Feature-gate the queue primitive: `heapless::spsc::Queue` (no_std, capacity
  fixed at creation) and `flume`/`tokio::mpsc` (std). Per-subscription capacity
  is configurable with a crate default constant.
- Implement drop-oldest + overflow-counter backpressure on the bounded queue.

Acceptance criteria:

- `cargo test --workspace` passes.
- `cargo check -p clinkz-wot-core --no-default-features` passes.
- Tests cover inbound fan-out to multiple sinks and outbound drop-oldest with
  overflow counting.

### SR-P0.5 Inbound Security Verification

Status: done.

Goal: add the symmetric inbound security entry point (v3.0 §8 / v3.1 §1.3).

Work items:

- Extend `SecurityProvider` with
  `verify(&self, request: &InboundRequest, scheme: &SecurityScheme) ->
  Result<Principal, SecurityError>`.
- Keep `apply` (outbound) and the existing `scheme_name`-keyed registry.
- Wire the `InboundDispatcher` to call `verify` before routing to a handler,
  resolving the matched `Form`'s security scheme internally.
- v1 scope: authenticate plus optional scope match against affordance
  `security` / `scopes`. No per-affordance policy engine (deferred).

Acceptance criteria:

- `cargo test --workspace` passes.
- `cargo check -p clinkz-wot-core --no-default-features` passes.
- Tests cover successful verify, missing/invalid credentials, and scope denial
  flowing through the dispatcher into `InboundResponse`.

## SR-P1: Servient Collapse and Interior Mutability

Goal: reshape `Servient` to the v3.0 §5–§7 / v3.1 §2 target.

### SR-P1.1 Single-Generic Interior-Mutable Servient

Status: done.

Goal: collapse to `Servient<D>`, `Clone`, `&self` everywhere.

Work items:

- Collapse `Servient<D, R, C, S, P>` to `Servient<D>`. `ExposedThingRegistry`,
  `ConsumedThingRegistry`, and the form/binding-plan caches become internal
  concrete types (the caches merge into `ConsumedThingRegistry` per v3.0 §5.1).
- Make `Servient<D>` `Clone` (cheap, `Arc`-based) with all public methods taking
  `&self` (never `&mut self`). Remove the `running` flag and `start`/`stop`.
- Remove the manual three-way synchronization between directory, exposed
  registry, and consumed registry.
- Update `ServientBuilder` to the single-`D` shape.

Acceptance criteria:

- `cargo test --workspace` passes.
- `Servient<D>` is `Clone`; no public method takes `&mut self`.
- Existing servient integration tests are migrated to the new shape.

### SR-P1.2 Typed Interaction Handles

Status: done.

Goal: expose interactions through v3.0 §6 handles.

Work items:

- Add `expose(&self, td) -> ServientResult<ExposedThingHandle>` and
  `consume(&self, td) -> ServientResult<ConsumedThingHandle>`.
- `ExposedThingHandle` and `ConsumedThingHandle` hold `Arc` clones of the
  relevant stores plus ids; they are `Clone` and (async build) `Send + Sync`.
- Both expose `read_property` / `write_property` / `invoke_action` /
  `subscribe_event` aligned with the WoT Scripting API.
- Local in-process interactions go directly to the handler without form
  selection or transport security (v3.0 §6).
- Add handler attachment on `ExposedThingHandle` (`set_property_handler`,
  `set_action_handler`, `set_event_handler`) per v3.1 §4.

Acceptance criteria:

- `cargo test --workspace` passes.
- Handles are `Clone`; the sync/async `Send + Sync` split is cfg-selected.

### SR-P1.3 Two-Level Registry Locking

Status: done.

Goal: implement the v3.0 §7 locking discipline.

Work items:

- Implement `ExposedThingRegistry` as
  `Arc<MapLock<BTreeMap<ThingId, Arc<ThingLock<ThingEntry>>>>>`.
- Add the `pub(crate)` `MapLock` / `ThingLock` newtype selecting `RefCell`
  (sync), `std::sync::Mutex` (async + std), or `critical_section::Mutex` /
  `embassy_sync` (async + embassy) per feature.
- Implement the dispatch discipline: lock map → clone `Arc<ThingEntry>` → drop
  map lock → lock entry → run handler → drop entry lock. Never hold a lock
  across `.await` or a handler that calls back into the Servient.
- Implement deferred `destroy(own_id)`: set a `draining` flag and complete
  removal after the in-flight handler returns.

Acceptance criteria:

- `cargo test --workspace` passes.
- Tests cover concurrent interactions against different Things (no contention)
  and serialized interactions within one Thing.
- A handler calling `destroy(own_id)` does not self-deadlock.

### SR-P1.4 ConsumedThingRegistry Interning

Status: done.

Goal: implement the v3.0 §5.1 interning map.

Work items:

- Add `ConsumedThingRegistry` as the interned map of live `ConsumedThing`
  instances keyed by Thing identity.
- `consume()` of the same Thing returns the same handle (identity interning).
- Internalize form selection, binding plan, and key-expression mapping into the
  interned instance (replacing per-call recomputation).
- Keep it in-memory and never persisted. Rebuild lazily from `consume()` calls
  on restart.
- Add the explicit `invalidate(id)` entry point (used by SR-P3).

Acceptance criteria:

- `cargo test --workspace` passes.
- Tests prove repeated `consume()` of the same identity shares one live
  instance.

## SR-P2: Driving Layer and expose/destroy Coordination

Goal: provide the v3.0 §4 driving primitives and the v3.0 §10 / v3.1 §4
coordination semantics.

### SR-P2.1 Sync Driving Layer

Status: done.

Goal: the default sync flavor (v3.0 §4 / v3.1 §6.2).

Work items:

- Add `poll_serve_sync(&self) -> ServientResult<()>` (one synchronous
  iteration) and `serve_sync(&self) -> !` (the infinite loop wrapper).
- `poll_serve_sync` polls each `ServerBinding::poll_accept_sync()`, runs the
  `InboundDispatcher`, and writes the `InboundResponse` back through the
  binding.
- Document that on bare `no_std` MCU super-loops, `poll_serve_sync` is the
  primary primitive; `serve_sync` targets std host/cloud runtimes.

Acceptance criteria:

- `cargo test --workspace` passes.
- `cargo check -p clinkz-wot-servient --no-default-features` passes (sync
  flavor with no async runtime).

### SR-P2.2 Async Driving Layer and Feature Matrix

Status: done.

Goal: the native-async flavor behind a feature (v3.0 §4, §12).

Work items:

- Add an `async` feature to `clinkz-wot-servient` gating the async primitives.
- Add `poll_serve(&self) -> impl Future<Output = ServientResult<()>> + Send`
  and `serve(self) -> impl Future<Output = ()> + Send` (`self` by value for a
  `'static + Send` spawnable future).
- `poll_serve` awaits `ServerBinding::poll_accept()` natively (Waker-based),
  never as a sync wrapper.
- Add the optional deps behind features: `critical_section` + `embassy_sync`
  (async + no_std), `flume` (std queue), `heapless` (no_std queue).
- Lock the feature matrix: default (sync, `RefCell`, `!Send`); `async`
  (`critical_section`/`embassy_sync`, `Send + Sync`); `std` + `async`
  (`std::sync::Mutex`, `Send + Sync`).
- Enforce that a sync build uses only the sync pair and an async build uses
  only the async pair (no crossing).

Acceptance criteria:

- `cargo test --workspace` passes with default features.
- `cargo test -p clinkz-wot-servient --features async` passes.
- `cargo check -p clinkz-wot-servient --no-default-features --features async`
  passes (embassy-style no_std async).

### SR-P2.3 expose/destroy Coordination

Status: done.

Goal: the v3.0 §10 / v3.1 §4 coordination semantics.

Work items:

- `expose(td)` under the outer map lock: validate the TD (well-formed, has id),
  insert the entry, register inbound routes in each `ServerBinding`, publish to
  the `Directory`.
- A binding route-registration failure is fatal: remove the entry and return
  `Err(ServientError::RouteRegistration(..))`.
- A `Directory` publish failure is non-fatal: the Thing remains locally
  exposed; surface a warning.
- `destroy(id)`: unregister inbound routes first, then remove the entry, then
  best-effort unpublish from the `Directory`.
- Dispatch to an affordance with no attached handler returns
  `CoreError::MissingHandler` in the response (v3.1 §4).

Acceptance criteria:

- `cargo test --workspace` passes.
- Tests cover fatal route failure rollback, non-fatal directory publish
  failure, and `destroy(own_id)` deferred removal during a handler.

## SR-P3: Directory-Driven Invalidation Wiring

Goal: wire the v3.1 §3 v1 invalidation contract.

### SR-P3.1 Servient-Mediated Co-Located Invalidation

Status: done.

Goal: keep `ConsumedThingRegistry` fresh after directory writes without a new
directory trait (v3.1 §3).

Work items:

- After a successful `Directory::update` or `Directory::delete` for a Thing id,
  the Servient calls `ConsumedThingRegistry::invalidate(id)` synchronously in
  the same call.
- Keep the explicit `invalidate(id)` programmatic entry point available.
- Do **not** add a watch/notify/stream surface to `ThingDirectory` in v1.
- Document that remote Thing Description Directory observation (where `D` is a
  remote-directory client) is deferred behind a future `std`-gated
  `DirectoryWatch` extension trait.

Acceptance criteria:

- `cargo test --workspace` passes.
- Tests prove a directory `update`/`delete` invalidates the corresponding
  interned consumed Thing so the next interaction rebuilds its form selection
  and binding plan.

## SR-P4: Zenoh Server Binding

Goal: add the v3.0 §1 / §12 / §13 server side on the shared client session.

### SR-P4.1 ServerBinding on the Shared Session

Status: done.

Goal: serve exposed Things over zenoh without a second session (v3.0 §1, §13).

Work items:

- Implement `ServerBinding` for the zenoh binding against the same session used
  by `ClientBinding` (one shared session for a gateway Servient).
- Map the v3.0 §13 inbound operations: `readproperty`/`invokeaction` via
  `declare_queryable`, `writeproperty` via a put listener on the key,
  `observeproperty`/`subscribeevent` via a publisher on the key.
- Extract `AuthMaterial` and `CorrelationId` from inbound zenoh samples/queries
  and echo `CorrelationId` in the `InboundResponse`.
- Keep the server side behind the existing `zenoh` feature; keep
  `--no-default-features` free of a concrete zenoh runtime.

Acceptance criteria:

- `cargo test -p clinkz-wot-protocol-bindings-zenoh` passes.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  passes.
- An opt-in smoke test (gated by `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1`) covers
  a round-trip read, write, action invoke, and event subscription through the
  shared session.

### SR-P4.2 Inbound Route Registration

Status: done.

Goal: reuse the zenoh planner in the reverse direction for route registration.

Work items:

- During `expose`, register each affordance form's protocol key (derived from
  the existing zenoh planner) as an inbound route in the zenoh server binding.
- During `destroy`, undeclare those routes.
- Keep the planning layer in the existing `no_std + alloc` zenoh planner; only
  the route registration/undeclaration is new server-side runtime state.

Acceptance criteria:

- `cargo test --workspace` passes.
- Tests cover route registration on `expose` and undeclaration on `destroy`.

### SR-P4.3 Zenoh-Pico Server-Side Scope Note

Status: done.

Goal: size and document the constrained server-side work without blocking v1.

Work items:

- Note in `docs/zenoh-pico-runtime-target.md` that `declare_queryable` and
  publishers on zenoh-pico are a larger scope than the existing outbound
  platform hooks, and remain target-specific.
- Keep the v1 server binding on the Rust `zenoh` (std) backend; do not require
  zenoh-pico server support for v1 acceptance.

Acceptance criteria:

- The scope note is recorded.
- v1 acceptance does not depend on zenoh-pico server execution.

## SR-P5: M7 Alignment

Goal: keep conformance and embedded checks aligned with the redesigned
surfaces (v3.0 §12, v3.1 §8).

### SR-P5.1 Feature-Matrix and no-std Verification

Status: done.

Goal: extend M7 checks to the new feature matrix.

Work items:

- Extend `scripts/check-no-std.sh` to cover the new core inbound surface and
  the servient sync flavor (`--no-default-features`).
- Add a check for the async no_std flavor
  (`--no-default-features --features async`) for core and servient.
- Keep `scripts/check-reserved-features.sh` aligned with the servient `async`
  feature and the existing zenoh backend feature policy.
- Update `scripts/check-m7.sh` to include the new checks.

Acceptance criteria:

- `scripts/check-m7.sh` passes.
- `scripts/check-no-std.sh` covers core, servient (sync), and the existing
  no_std crates.

### SR-P5.2 Documentation Updates

Status: done.

Goal: align the repository documentation referenced by baseline §15 / v3.1 §8.

Work items:

- Update `PLAN.md` M6 (status, current scope, exit criteria) to point at the
  redesign plan and mark the old status superseded.
- Update `docs/technical-spec.md` Servient and feature-policy sections for the
  single-generic `Servient<D>`, the `async` feature, and the sync/async split.
- Update `docs/no-std-embedded.md` for the new embedded capabilities and the
  feature matrix.
- Update `docs/verification.md` for the new M7 check commands.

Acceptance criteria:

- The four documents are consistent with the baseline, the addendum, and the
  implemented feature matrix.

## Compliance Fixes (post-redesign)

After the SR-P0–SR-P5 redesign, a full WoT compliance audit identified
additional gaps. The following compliance fixes have landed:

### T1: Event Pipeline (done)

Status: done.

- `EventBroker` is now `Clone` (via `Arc<MapLock<…>>`) and wired into
  `ServientInner` as `event_broker`.
- `ServerBinding::set_event_broker` default method feeds the shared broker to
  each binding during `ServientBuilder::build()` and
  `Servient::register_server_binding()`.
- `ZenohServerBinding` registers `ZenohPublisherSink` (wrapping `session.put`)
  for each `subscribeevent` / `observeproperty` form during `register_thing`.
- `ExposedThingHandle::emit_event(name, payload)` fans payloads through the
  broker to all registered publisher sinks (W3C Scripting API `emitEvent`).
- `dispatch_to_handler` routes `SubscribeEvent`, `UnsubscribeEvent`,
  `ObserveProperty`, `UnobserveProperty` through the broker.
- `EventBroker::remove_thing` and `remove_event` clean up sinks during
  `destroy`.
- 6 new tests: broker clone/remove, emit_event delivery/noop/destroy-cleanup,
  dispatcher subscribe/observe/unsubscribe/unobserve.

### T3: Principal Threading (done)

Status: done.

- `verify_inbound` returns the real verified `Principal` from
  `SecurityProvider::verify` instead of discarding it and returning hardcoded
  `anonymous`. Anonymous is only the fallback for NoSec.
- `InteractionInput` gains `principal: Option<Principal>` field.
- `dispatch_inbound` injects the verified principal into the handler
  `InteractionInput` so handlers can authorize per-caller (W3C Scripting API
  `InteractionRequest` model).
- 2 new tests: handler receives verified principal with bearer auth; handler
  receives anonymous principal for NoSec.

### T2: Consumer Streaming Subscriptions (done)

Status: done.

- `ClientBinding::subscribe` method (default `UnsupportedOperation`) opens a
  long-lived wire subscription, returns
  `(Subscription, Box<dyn SubscriptionGuard>)`.
- `SubscriptionGuard` trait: `fn close(self: Box<Self>)` for protocol-specific
  cleanup.
- `ZenohTransport::open_subscription` (default `UnsupportedOperation`):
  `ZenohSessionTransport` declares `session.declare_subscriber(key_expr)` with a
  callback that pushes samples into a `SubscriptionSender`; returns a
  `ZenohSubscriptionGuard` that undeclares the subscriber on close.
- `SharedZenohTransport<T>` delegates `open_subscription`.
- `ZenohBindingTransport<T>::subscribe` validates the form, plans the zenoh
  operation, and delegates to `T::open_subscription`.
- `ConsumedThingHandle::subscribe_event` / `observe_property` now return
  `Subscription` (streaming) instead of one-shot `InteractionOutput`.
- `ConsumedThingHandle::unsubscribe_event` / `unobserve_property` stop wire
  subscriptions and release resources.
- `ConsumedThingEntry` stores subscription guards; `invalidate` calls
  `stop_all_subscriptions` to clean up on TD update / destroy.
- 5 new tests: subscribe_event streaming, observe_property streaming,
  unsubscribe cleanup, unobserve cleanup, poll_next + stop lifecycle.

### C5: Split Handler Traits (done)

Status: done.

- `PropertyHandler` replaced by `PropertyReadHandler`, `PropertyWriteHandler`,
  `PropertyObserveHandler`.
- `EventHandler` replaced by `EventSubscribeHandler`,
  `EventUnsubscribeHandler`.
- `LocalThing` stores per-affordance `PropertyHandlerSet` (read/write/observe)
  and `EventHandlerSet` (subscribe/unsubscribe).
- `ExposedThing` trait gains `observe_property` and `unsubscribe_event`.
- `ExposedThingHandle` has 6 separate setters: `set_property_read_handler`,
  `set_property_write_handler`, `set_property_observe_handler`,
  `set_event_subscribe_handler`, `set_event_unsubscribe_handler`, and
  `set_action_handler`.
- Dispatcher falls back to read+emit for ObserveProperty and ack for
  UnsubscribeEvent when no dedicated handler is registered.
- 3 new tests: read-only property, observe handler, unsubscribe handler.

### C6: Bulk Property Operations (done)

Status: done.

- `read_multiple_properties(names)`, `read_all_properties()`,
  `write_multiple_properties(values)` on both `ExposedThingHandle` and
  `ConsumedThingHandle`.
- Fan-out over individual property operations for binding portability.
- 3 new tests: read_multiple, read_all, write_multiple.

### C7: Discovery API (done)

Status: done.

- `ThingFilter`: fragment-based discovery filter with `new`,
  `fragment`, `fragment_field`, and `with_fragment`.
- `ThingDiscovery`: process object with `stop()`, `is_done()`, `error()`,
  `remaining()`, `filter_ref()`, synchronous `next_now()`, and async `next()`.
- `discover(directory, filter)` function in the discovery crate uses the
  in-memory directory backend and applies fragment filtering locally.
- `Servient::discover(filter)` wraps the discovery function.
- `DiscoveryError` now covers directory validation and storage failures only;
  the old discovery-method branch has been removed.
- 5 tests cover all-results iteration, fragment filtering, stop behavior, and
  filter inspection.

### Remaining compliance gaps (deferred)

- M6: Remote directory transport and protocol-specific discovery backends.

## Verification

Per-phase minimum checks (run after each phase):

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
scripts/check-no-std.sh
```

Additional checks after SR-P2.2 and SR-P4:

```sh
cargo test -p clinkz-wot-servient --features async
cargo check -p clinkz-wot-servient --no-default-features --features async
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
scripts/check-reserved-features.sh
```

Opt-in zenoh runtime smoke (after SR-P4):

```sh
CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1 \
  cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh
```

Final aggregate (after SR-P5):

```sh
scripts/check-m7.sh
```
