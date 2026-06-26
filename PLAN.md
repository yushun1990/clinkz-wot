# clinkz-wot Implementation Plan

## Summary

`clinkz-wot` is a protocol-neutral Rust implementation of a W3C Web of Things engine for the Clinkz platform.

The engine uses TD and TM as the semantic contract layer. Protocol bindings are pluggable adapters. Zenoh is implemented first because Clinkz Platform uses it as the default communication bus, but zenoh is not a required dependency of the engine.

The default specification target is W3C WoT TD 1.1, Architecture 1.1, Discovery, and Profile. TD 2.0 is tracked as experimental work behind a feature flag.

## Scope

This file is the repository-level blueprint and milestone index. It defines the
intended delivery sequence and repository-wide acceptance criteria.

Detailed development tasks belong in module plans under `docs/plan/`. Milestone
sections reference those module plans when they exist.

## Milestones

Current focus:

- TD 1.1 hardening in M1 is complete for the current crate scope.
- Thing Model support has a first complete TD-crate implementation.
- M3 protocol-neutral core has the first trait and dispatcher surface needed by
  binding crates.
- M4 protocol binding hardening is complete for the current shared and zenoh
  binding scope.
- M5 Discovery has the first embedded-ready in-memory Thing Description
  Directory and query surface needed by runtime crates.
- M6 Servient runtime composition now has a first embedded-ready runtime
  surface that wires Discovery, local Things, consumed Things, injected
  protocol bindings, runtime registries, TD/form/binding-plan caches, payload
  codecs, and security providers. This first surface is about to be superseded
  by the Servient runtime redesign.
- The Servient runtime redesign has landed (phases SR-P0 through SR-P5) and
  the runtime has since been hardened — handler reentrancy (sync `&self`
  handlers cloned out of the per-Thing TD lock + a driving-loop `sync_lock`),
  dynamic affordance lifecycle (per-affordance `ServerBinding::register_affordance`
  / `unregister_affordance` + directory re-publish), inbound/outbound security
  moved out of registry locks, and several diagnostics/robustness fixes. See
  `docs/baseline/servient-design-baseline-addendum.md` §9 for the refinement
  record. The redesign baseline is
  `docs/baseline/servient-design-baseline.md` (v3.0) + addendum (v3.1), sequenced
  by `docs/plan/servient-runtime-redesign-plan.md`.
- The next concrete backend increment is the opt-in Rust `zenoh` runtime path
  behind `zenoh`, with live smoke and integration coverage kept outside the
  default workspace test path.
- The current `zenoh` next-step target is live metadata coverage for
  express QoS, priority, and congestion control on observable runtime paths.
- Tracked follow-up (not blocking): align handler `Send`/`Sync` trait-object
  bounds with v3.0 §7 (addendum §9.3) once multi-thread sync driving is in
  scope.
- Defer `zenoh-pico` runtime injection until the target hardware
  platform, C ABI strategy, and polling model are confirmed.
- Keep M7 conformance and no-std checks running across every crate that
  claims `no_std + alloc` support.

Immediate next sequence:

1. Keep M7 checks and compatibility documentation aligned with the current
   TD/TM, core, protocol binding, Discovery, and Servient surfaces.
2. Treat `zenoh` as the only active concrete runtime increment and
   expand its opt-in smoke and integration coverage without changing the
   default workspace verification path, starting with live metadata coverage
   for observable put and subscription paths.
3. Keep `zenoh-pico` at the planning boundary until the target
   hardware platform is selected and the runtime injection strategy is
   documented for that platform.

### M1: TD 1.1 Hardening

Plan: `docs/plan/wot-td-development-plan.md`.

Harden the TD 1.1 data model, validation, extension preservation, and
round-trip compatibility in `clinkz-wot-td`.

Current status:

- Foundation work is complete for `no_std + alloc`, URI field typing, builder
  error reporting, validation levels, security reference checks, field coverage
  audit, and shared `base` plus form `href` target resolution.
- Fixture expansion, shared TD default helpers, DataSchema Basic validation,
  and SecurityScheme Basic validation are complete.
- SecurityScheme deserialization now uses the `scheme` field as the concrete
  variant discriminator, keeps fixture-compatible API key `in: "uri"` values,
  and preserves TD round-trip behavior for the current fixture corpus.
- DataSchema deserialization now prefers the explicit `type` field as the
  concrete variant discriminator, while Basic validation still rejects
  inconsistent explicit type declarations and keeps round-trip behavior intact.

### M2: Thing Model Support

Plan: `docs/plan/wot-td-development-plan.md`.

Status: complete for the first TD-crate pass.

Add Thing Model modeling, validation, extension preservation, and a future path
from reusable TM templates to concrete TD documents.

Entry criteria:

- M1 fixture expansion and TD Basic validation hardening are complete.
- TM data structures can reuse TD component patterns without changing protocol
  boundaries or adding `std`-only dependencies.

Completion notes:

- Added `ThingModel` data structures and builders in `clinkz-wot-td`.
- TM parsing, serialization, validation, and extension preservation are covered
  by focused tests.
- TM support compiles under the same `no_std + alloc` check as TD.

### M3: Protocol-Neutral Core

Define the protocol-neutral engine trait surface for exposed Things, consumed
Things, interaction handlers, bindings, payload codecs, security providers, and
transport adapters.

Current status:

- Started `clinkz-wot-core` as a `no_std + alloc` workspace crate.
- Added the first protocol-neutral trait surface for payload codecs, exposed
  Things, consumed Things, protocol bindings, security providers, and transport
  adapters.
- Added protocol-neutral local Thing handler traits and a reusable dispatcher
  for property reads/writes, action invocation, and event subscription.
- Added a protocol-neutral consumed Thing dispatcher that validates selected
  affordance forms against TD effective operations and routes requests to
  matching bindings.
- Kept form selection, target URI resolution, and concrete protocol behavior
  outside the core crate for the later protocol-bindings milestone.
- Verified `clinkz-wot-core` with `cargo check -p clinkz-wot-core
  --no-default-features`.

Entry criteria:

- TD/TM public types expose effective operation, target, and security metadata
  needed by protocol binding consumers.
- The core trait surface remains independent of zenoh and other concrete
  transports.

### M4: Protocol Bindings and Zenoh Binding

Plan: `docs/plan/protocol-bindings-development-plan.md`.

Add shared binding utilities and implement zenoh as the first optional protocol
binding without making it a dependency of TD, TM, or core runtime crates.

Current status:

- Organized protocol binding crates under `protocol-bindings/`.
- Started `clinkz-wot-protocol-bindings` as a `no_std + alloc` workspace crate
  for shared protocol binding utilities.
- Added shared form selection based on TD effective operations and affordance
  context.
- Added shared form target resolution using the TD crate's `base` plus `href`
  helper.
- Added shared affordance-level form selection and target resolution for Thing,
  property, action, and event forms, including unknown-affordance errors.
- Added protocol-neutral form selection criteria for content type and
  subprotocol matching while preserving the existing operation-only selection
  API.
- Kept zenoh and other concrete protocol behavior out of
  `clinkz-wot-protocol-bindings`.
- Started `clinkz-wot-protocol-bindings-zenoh` as the first optional concrete
  protocol binding crate under `protocol-bindings/protocols/zenoh`.
- Added first-pass zenoh form support that recognizes `zenoh://` form targets
  and resolves relative TD forms against Thing-level `base`, without
  introducing a required zenoh runtime dependency.
- Added zenoh operation planning that maps WoT form operations to
  transport-level zenoh operation kinds while still avoiding a required zenoh
  runtime dependency.
- Extended zenoh operation planning with first-pass Clinkz extension metadata
  parsing for encoding, QoS, priority, and congestion control hints.
- Added shared predicate-based form selection and a zenoh affordance operation
  planner so concrete bindings can choose protocol-supported forms from
  multi-form affordances before runtime transport execution is wired in.
- Extended the zenoh affordance planner to accept shared form selection
  criteria, with zenoh planner coverage for content type and subprotocol
  filters.
- Improved shared form selection diagnostics so operation mismatches are
  distinguished from metadata or caller-filter mismatches.
- Added an injected zenoh transport adapter boundary to the generic
  `ZenohBinding<T>` so planned zenoh operations can be executed by std or test
  integrations without adding a required zenoh runtime dependency.
- Added shared validation for caller-selected affordance forms and wired zenoh
  runtime invocation to reject forms that do not belong to the requested
  affordance or do not support the requested effective operation before
  transport execution.
- Added shared protocol-neutral helpers for selected-form security references
  and scopes, including inherited Thing-level security, form-level overrides,
  and nosec coverage.
- Documented the first Clinkz zenoh extension vocabulary, limited to
  experimental metadata hints while keeping TD `href` and `base` authoritative
  for target resolution.
- Expanded zenoh planning coverage for Thing-level forms, bulk property and
  event operations, content type and subprotocol criteria, and relative `href`
  values resolved against a zenoh `base`.
- Kept concrete zenoh execution optional behind `ZenohTransport`, with tests
  for fake transport propagation and the default no-transport error path.
- Stabilized shared selected-form validation coverage for Thing-level forms,
  property forms, action defaults, event defaults, copied selected form values,
  and selected-form mismatch cases.
- Finalized shared binding diagnostics with distinct errors for unknown
  affordances, unsupported operations, metadata mismatches, caller-filter
  mismatches, target resolution failures, and selected forms outside the
  requested affordance.
- Documented the runtime backend policy for future Rust `zenoh` and
  `zenoh-pico` adapters: the current zenoh planning crate remains
  `no_std + alloc`, while concrete runtime backends stay optional and
  feature-gated or crate-separated.
- Added the first concrete Rust `zenoh` std runtime backend behind the
  explicit `zenoh` feature while keeping the default and
  `--no-default-features` builds free of a concrete zenoh runtime dependency.
- Hardened the Rust `zenoh` std runtime backend with request/reply selector
  parameter validation, subscription lifecycle metadata and undeclaration, and
  first-pass metadata mapping for encoding, express QoS, priority, and
  congestion control.
- Added a std-only shared zenoh transport handle so Servient binding factories
  can reuse a session, pool, or runtime adapter across cloned bindings.
- Added an opt-in Rust `zenoh` runtime smoke test behind the `zenoh`
  feature and `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1`, covering concrete
  `ZenohSessionTransport` put and get/request-reply execution without requiring
  it in default workspace tests.

Completion notes:

- `clinkz-wot-protocol-bindings` and
  `clinkz-wot-protocol-bindings-zenoh` pass `cargo test`.
- Both protocol binding crates pass `cargo check --no-default-features`.
- Zenoh-specific code remains outside TD, TM, and core crates.

Exit criteria:

- `clinkz-wot-protocol-bindings` and
  `clinkz-wot-protocol-bindings-zenoh` pass `cargo test`.
- Both protocol binding crates pass `cargo check --no-default-features`.
- Shared utilities cover form selection, target resolution, selected-form
  validation, diagnostics, and security metadata extraction needed by runtime
  crates.
- Zenoh-specific code remains outside TD, TM, and core crates.

### M5: Discovery and TDD

Implement W3C Discovery concepts and Thing Description Directory behavior for
registration, lookup, update, deletion, and query flows.

Current status:

- Started `clinkz-wot-discovery` as a workspace crate.
- Added protocol-neutral Thing Description Directory traits for registration,
  retrieval, update, deletion, listing, and query.
- Added backend-portable structured directory queries with exact-match filters,
  conjunctive matching, pagination metadata, and owned result entries suitable
  for memory, SQL, RDF/SPARQL, HTTP, or other runtime backends.
- Added a deterministic in-memory directory backend with configurable TD
  validation level and a local predicate-query convenience API for tests and
  local filtering.
- Added focused tests for duplicate registration, update behavior, deletion,
  lookup by id, deterministic listing, structured query predicates, pagination,
  missing ids, validation failures, and owned result cloning.

Entry criteria:

- M4 shared binding APIs are stable enough for Discovery and Servient crates to
  refer to TD forms without duplicating form selection or target resolution.
- TD validation exposes the Basic checks needed to reject invalid directory
  entries explicitly instead of during deserialization.

Planned work:

- Add `clinkz-wot-discovery` as a workspace crate with crate-root shared
  directory and query APIs, no-std local directory capabilities, and std-only
  storage extension points.
- Define protocol-neutral directory traits for registration, retrieval, update,
  deletion, listing, and query.
- Implement a deterministic in-memory directory backend first.
- Keep concrete production storage backends separate from the shared Discovery
  query model, following the same adapter boundary used by protocol bindings.
- Validate TD inputs explicitly at configurable validation levels before
  registration and update.
- Preserve full TD round-trip data, including unknown extension fields and
  JSON-LD contexts, through directory storage.
- Add tests for duplicate registration policy, overwrite/update behavior,
  deletion, lookup by id, listing, and basic query predicates.

Exit criteria:

- The discovery crate has focused tests for CRUD and query behavior.
- Discovery does not depend on zenoh or any concrete binding.
- Directory storage keeps TD/TM semantic data and extension fields intact.

### M6: Servient Runtime

Compose TD/TM, protocol bindings, discovery, security, and observability into a
Servient runtime that supports exposed and consumed Things.

The M6 target shape is defined by the locked design baseline
`docs/baseline/servient-design-baseline.md` (v3.0) and its implementation
refinements `docs/baseline/servient-design-baseline-addendum.md` (v3.1). The
sequencing and acceptance details are in
`docs/plan/servient-runtime-redesign-plan.md`. The redesign is a one-shot
breaking refactor to a single-generic interior-mutable `Servient<D>` with a
sync driving layer, an inbound serving path, directory-driven consumed-Thing
invalidation, and a zenoh server binding.

Current status — redesign phases SR-P0 through SR-P5 landed, plus compliance
fixes T1 (event pipeline) and T3 (principal threading):

- **SR-P0** (core inbound surface and owned types): owned `AffordanceTarget`,
  `BindingRequest`, `InboundRequest`, `InboundResponse`; `ThingId`,
  `CorrelationId`, `Principal`, `SecurityError`; `ClientBinding` /
  `ServerBinding` trait split replacing `ProtocolBinding`; `EventBroker` /
  `Subscription`; `SecurityProvider::verify` + `check_scopes`.
- **SR-P1** (`Servient<D>` collapse): single-generic `Servient<D>` that is
  `Clone` with all `&self` methods; typed `ExposedThingHandle` /
  `ConsumedThingHandle`; two-level `MapLock` + `DrainFlag` locking;
  `ConsumedThingRegistry` interning with per-entry form/binding-plan caches.
- **SR-P2.1** (sync driving layer): `poll_serve_sync` / `serve_sync`;
  `InboundDispatcher` implementation resolving Thing, form security, and
  handler dispatch; `CoreError::MissingHandler` for unhandled affordances.
- **SR-P2.2** (async driving layer): `AsyncServerBinding` trait (dyn-compatible
  via `#[async_trait]`) behind the `async` feature; `Servient::poll_serve` /
  `Servient::serve` using `select_all` to race all async bindings concurrently;
  `ZenohServerBinding` implements `AsyncServerBinding` using `tokio::sync::Notify`.
- **SR-P2.3** (expose/destroy coordination): `expose` with route registration
  + rollback on failure; `destroy` with route unregistration; directory
  publish/unpublish as best-effort; `ServientError::Accept`,
  `RouteRegistration`, `From<SecurityError>`.
- **SR-P3** (directory-driven invalidation): Servient-mediated
  `ConsumedThingRegistry::invalidate` after `update`, `unregister`, and
  `destroy`.
- **SR-P4** (zenoh server binding): `ZenohServerBinding` implementing
  `ServerBinding` on the shared `zenoh::Session`; readproperty/invokeaction
  via `declare_queryable`, writeproperty via put-listener; route planning
  reusing the `no_std + alloc` zenoh planner; also implements
  `AsyncServerBinding` for native-async driving.
- **SR-P5** (M7 alignment): feature-matrix and no-std verification confirmed;
  documentation updated.
- **T1** (event pipeline): `EventBroker` wired into `ServientInner` as a
  `Clone`-able shared broker (`Arc<MapLock<…>>`); `ServerBinding::set_event_broker`
  default method feeds the broker to each binding during `build()` and
  `register_server_binding()`; `ZenohServerBinding` registers
  `ZenohPublisherSink`s (wrapping `session.put`) for each `subscribeevent` /
  `observeproperty` form during `register_thing`; `ExposedThingHandle::emit_event`
  fans payloads through the broker to all registered publisher sinks;
  `dispatch_to_handler` routes `SubscribeEvent` / `UnsubscribeEvent` /
  `ObserveProperty` / `UnobserveProperty` through the broker; `destroy` cleans
  up all broker sinks for the Thing via `EventBroker::remove_thing`.
- **T3** (principal threading): `verify_inbound` returns the real verified
  `Principal` (or anonymous for NoSec) instead of discarding it;
  `InteractionInput` gains a `principal: Option<Principal>` field;
  `dispatch_inbound` injects the verified principal into the handler
  `InteractionInput` so handlers can authorize per-caller.
- **T2** (consumer streaming subscriptions): `ClientBinding::subscribe` method
  (default `UnsupportedOperation`) opens a long-lived wire subscription and
  returns `(Subscription, Box<dyn SubscriptionGuard>)`; `ZenohTransport::
  open_subscription` on `ZenohSessionTransport` uses `session.declare_subscriber`
  with a callback that pushes samples into a `SubscriptionSender`;
  `ConsumedThingHandle::subscribe_event` / `observe_property` now return
  `Subscription` instead of one-shot `InteractionOutput`; added
  `unsubscribe_event` / `unobserve_property` for wire cleanup; subscription
  guards stored in `ConsumedThingEntry` and cleaned up on invalidation.
- **C5** (split handler traits): `PropertyHandler` replaced by separate
  `PropertyReadHandler`, `PropertyWriteHandler`, `PropertyObserveHandler`;
  `EventHandler` replaced by `EventSubscribeHandler`, `EventUnsubscribeHandler`;
  `LocalThing` stores per-affordance `PropertyHandlerSet` / `EventHandlerSet`;
  `ExposedThingHandle` has separate `set_property_read_handler` /
  `set_property_write_handler` / `set_property_observe_handler` /
  `set_event_subscribe_handler` / `set_event_unsubscribe_handler`; dispatcher
  falls back to read+emit for observe and ack for unsubscribe when no dedicated
  handler is registered.
- **C6** (bulk property operations): `read_multiple_properties`,
  `read_all_properties`, `write_multiple_properties` on both
  `ExposedThingHandle` and `ConsumedThingHandle`. The consumed side prefers a
  single Thing-level form declaring the matching bulk meta-operation (W3C TD
  §6.3.3) and otherwise fans out over individual property operations; the
  inbound serving path fans out across exposed handlers and combines the
  results so Thing-level bulk forms are servable end-to-end.
- **C7** (Discovery API): `DiscoveryMethod` enum (`Local` / `Directory` /
  `Multicast` / `Everything`); `ThingFilter` with method, url, query, fragment;
  `ThingDiscovery` process object implementing `Iterator<Item = Thing>` with
  `stop()` / `is_done()` / `error()` / `remaining()`; `Servient::discover(filter)`
  backed by the local directory for `Local`/`Everything`; `Directory` and
  `Multicast` methods set discovery error (deferred to protocol-specific
  transports).
- **M3+M4** (security end-to-end): `InteractionInput.security_metadata` field
  separates transport-level auth headers from URI variables;
  `apply_security` diffs provider-modified metadata into `security_metadata`;
  zenoh server extracts `AuthMaterial::BearerToken` from `Query`/`Sample`
  attachment via `attachment_to_auth`.
- **M5** (error→status mapping): `clinkz_wot_protocol_bindings::error_status`
  maps `CoreError` to HTTP-like status codes (404/401/403/501 etc.); zenoh
  server includes `[status]` prefix in error replies.
- **M12** (graceful shutdown): `Servient::shutdown_handle()` returns a `Clone`
  `ShutdownHandle`; `serve_sync`, `serve`, and `poll_serve_sync` check the
  `Arc<AtomicBool>` flag and exit gracefully.
- **M7** (credential vault): `CredentialStore` trait +
  `InMemoryCredentialStore` (`BTreeMap<(thing_id, scheme), Credentials>` backed
  by `MapLock`); `Credentials` enum (BearerToken, Basic, ApiKey, Psk, Other);
  `SecurityContext.credentials` field passes the store to
  `SecurityProvider::apply`; `ServientBuilder::credential_store(store)`.
- **M13** (runtime TD mutation): `ExposedThingHandle::add_property` /
  `add_action` / `add_event` / `remove_property` / `remove_action` /
  `remove_event` modify the TD after expose; handlers for removed affordances
  are cleaned up automatically. Mutations also propagate to the network side
  via the per-affordance `ServerBinding::register_affordance` /
  `unregister_affordance` API (default no-op; the zenoh binding declares /
  undeclares the affordance's routes incrementally, tracked per-affordance)
  and re-publish the post-mutation TD to the directory, closing the dynamic
  affordance lifecycle (W3C Scripting API) so new affordances become remotely
  reachable and discoverable and removed ones stop being served.
- **M8** (async consumer API): `ConsumedThingHandle` gains async variants
  (`read_property_async`, `write_property_async`, `invoke_action_async`,
  `subscribe_event_async`, `observe_property_async`) behind the `async` feature;
  current implementation delegates to sync path (resolves immediately) with
  forward-compatible API for future native async bindings.
- **M9** (async handlers): `AsyncPropertyReadHandler`, `AsyncPropertyWriteHandler`,
  `AsyncActionHandler` traits (`#[async_trait]`, `Send`, behind `async` feature);
  `LocalThing` stores async handlers alongside sync handlers; async driving loop
  (`poll_serve`) calls `dispatch_inbound_async` which uses the take-out / await /
  return pattern to avoid holding the thing slot lock across `.await`; falls
  back to sync handlers when no async handler is registered.

Deferred:

- M6: Remote directory transport (`DirectoryWatch`, `Directory`/`Multicast`
  discovery methods — requires HTTP/CoAP TDD client).

Entry criteria:

- M5 provides a usable directory abstraction and in-memory backend.
- Core consumed/exposed Thing dispatch can route through protocol bindings with
  validated forms.

Exit criteria:

- `Servient<D>` exposes and consumes Things through the protocol-neutral core
  `ClientBinding` / `ServerBinding` / `AsyncServerBinding` traits with both sync
  and async driving flavors.
- Zenoh remains optional and can be omitted from Servient builds.
- Runtime integration tests cover discovery, inbound serving, binding dispatch,
  and zenoh server binding round-trips (read, write, invoke, error).
- The workspace M7 baseline (formatting, tests, Clippy, no-default-features
  checks, `scripts/check-no-std.sh`, `scripts/check-reserved-features.sh`, and
  `scripts/check-m7.sh`) passes with the redesigned surfaces.

### M7: Conformance and Embedded Support

TD/TM plan: `docs/plan/wot-td-development-plan.md`.

Add W3C compatibility checks, fixture coverage, and no-std-oriented
`no_std + alloc` verification for crates that claim embedded support.

Planned work:

- Keep TD/TM round-trip fixtures aligned with TD 1.1 field coverage and
  extension preservation requirements.
- Keep workspace-level verification documentation for `cargo test`,
  `cargo fmt`, Clippy, and `--no-default-features` checks in
  `docs/verification.md`.
- Add no-default-features checks for every embedded-ready crate as it is added
  or changed.
- Keep `scripts/check-no-std.sh` aligned with every crate that claims
  `no_std + alloc` support.
- Add conformance-oriented fixtures for multi-form affordances, form security,
  `base` plus relative `href`, JSON-LD context preservation, and Clinkz
  extension terms.
- Keep TD 2.0 behavior behind an experimental feature and out of default
  compatibility expectations.

Current status:

- `docs/verification.md` defines the regular workspace verification path for
  formatting, tests, Clippy, no-default-features checks, focused crate checks,
  and documentation updates.
- `docs/zenoh-runtime-integration-test.md` records the acceptance target for
  the current active opt-in real Rust `zenoh` runtime increment.
- `docs/zenoh-pico-runtime-target.md` records the acceptance target for the
  deferred constrained zenoh-pico runtime backend and its target-specific C ABI
  follow-up boundary.
- `docs/zenoh-pico-c-abi-integration-target.md` records the target-specific
  acceptance boundary for real zenoh-pico C ABI integrations.
- `scripts/check-reserved-features.sh` covers constrained zenoh-pico feature
  compilation, fake platform tests, and incompatible runtime backend feature
  diagnostics for the current zenoh binding feature policy.
- The zenoh binding tests consume the shared
  `clinkz-extension-defaults.td.jsonld` fixture to verify Clinkz JSON-LD
  extension terms, multi-form affordance selection, `base` plus relative
  `href` resolution, form security overrides, content type criteria, and
  Thing-level forms through the binding planner.
- `scripts/check-no-std.sh` covers every current workspace crate that claims
  `no_std + alloc` support: TD, core, shared protocol bindings, zenoh binding,
  Discovery, and Servient.
- The current protocol binding M7 verification path passes:
  - `cargo fmt --check`
  - `cargo test -p clinkz-wot-protocol-bindings -p clinkz-wot-protocol-bindings-zenoh`
  - `cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh-pico`
  - `scripts/check-no-std.sh`
  - `scripts/check-reserved-features.sh`
- The current workspace M7 baseline passes:
  - `cargo fmt --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets`
  - `scripts/check-no-std.sh`
  - `scripts/check-reserved-features.sh`
- `scripts/check-m7.sh` is the aggregate entry point for the current workspace
  M7 baseline.
- The opt-in Rust `zenoh` runtime smoke test passes with:
  - `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1 cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh runtime_zenoh_transport_executes_put_and_get_smoke_paths`

Exit criteria:

- All crates that claim embedded support pass no-default-features checks.
- Public fixtures cover the TD/TM, core dispatch, binding selection, discovery,
  and Servient flows needed for a first compatibility baseline.
- No protocol-specific behavior leaks into TD/TM/core crates.

## Acceptance Criteria

- Core TD/TM documents can be parsed, validated, serialized, and round-tripped without losing extension data.
- The TD/TM/core crates compile without `std` when built with the no-default-features set.
- The engine core has no dependency on zenoh.
- The zenoh binding can be enabled as an optional crate or feature.
- Protocol bindings all use the same protocol-neutral trait surface.
- Technical documentation and comments are English-only.
