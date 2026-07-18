# WP-300 Binding Contracts and Binding-Local Progress

Status: Planned

Design revision: v4.9

Depends on: WP-200

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core

## Scope

Replace the host-only binding shapes in `clinkz-wot-core` with the frozen complete-registration
and constrained associated-state contracts. Implement the core compiler-extension envelope,
route-scoped readiness and acceptance, request/response ownership, subscription start/stop,
Producer emission, runtime status, form contribution, generation-safe typed operation slots,
binding-owned subscription progress, binding-local publication slots, and bounded cleanup
progress without adding a concrete protocol or a Servient scheduler.

This package defines and tests the execution SPI consumed by Servient and protocol packages.
WP-400 owns expose/destroy orchestration and registries; WP-600 owns zenoh and zenoh-pico
implementations. No protocol-specific route, transport, or authentication semantics enter core.

This is the only package that introduces `ProducerEmission`. Core defines the immutable emission
values and a `BindingEmissionSlot` for exactly one selected binding generation; it does not own
the Servient-wide fan-out record or a concrete dispatch policy. This package also provides bounded internal
compatibility adapters so the still-unmigrated WP-400 Servient and WP-600 concrete binding can
cross this checkpoint without a dependency cycle. An adapter is not a second public emission
contract and accepts no new callers after this package completes.

One host or static binding is installable only as a complete startup bundle. Component traits
remain constructible and independently testable, but a compiler, contributor, client half,
server half, status policy, or ingress policy cannot be installed separately. The bundle is
immutable after `ServientBuilder` freezes its registration snapshot; v1 has no runtime binding
add, replace, remove, or unload path.

The final XOR-shaped `InboundResponse`, its producer-origin `try_success`
validation, and the public shared consumer-origin
`validate_untrusted_binding_output` function must follow
`docs/amendments/WP-100-interaction-output-api-v1.md`. This package replaces the
legacy response envelope once, after the route and planning values from WP-200
exist.

## Requirements

- `BIND-REG-001`, `BIND-ROUTE-001`, `BIND-STORAGE-001`, `BIND-MEM-001`, and
  `BIND-DELIVERY-001` govern complete startup bundles, route-scoped engine progress,
  associated-state storage, lifetime and ingress bounds, and input preservation at acceptance.
- `API-SURFACE-001`, `BIND-IO-001`, `BIND-OUT-001`, `BIND-PROGRESS-001`,
  `BIND-CALL-CANCEL-001`, and `BIND-HOST-CANCEL-001` freeze host and poll execution,
  ownership, response, cancellation settlement, and subscription progress.
- `API-PAYLOAD-001` governs response metadata, validation, and the exactly-one terminal value.
- `LIFE-EXPOSE-002`, `LIFE-EXPOSE-003`, `STATE-BIND-001`, and `STATE-INFLIGHT-001` govern
  readiness, guard ownership, cleanup transfer, and admitted requests.
- `STATE-SUB-001`, `HANDLE-DROP-001`, and `PRODUCER-EMIT-001` govern subscription and emission
  ownership through cancellation, drop, and partial publication.
- `CLEANUP-RECORD-001` requires bounded cleanup identity and retained ownership without
  cloning plans, payloads, credentials, or TD documents.
- `SUB-STORAGE-001` and `SUB-DATA-001` govern bounded shared storage, direct slot delivery, and
  terminal visibility.
- `FORM-FINALIZE-001`, `FORM-FINALIZE-002`, `FORM-OWNER-001`, and `FORM-COVERAGE-001` govern
  registration capability, deterministic contribution, ownership, and strict coverage inputs.
- `CAP-STATUS-001` and `CAP-OVERFLOW-001` govern bounded runtime events and durable critical
  status.
- `CONSTRAINED-PROGRESS-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-SCHED-001`, and
  `CONSTRAINED-OWN-001` govern slots, typed work, fairness, and non-atomic ownership.
- `HOST-ASYNC-001`, `PERF-CALL-001`, and `PERF-ALLOC-001` govern erased adapters and
  allocation-sensitive binding-local paths. WP-400 owns the `HOST-SHARD-001` and
  `PERF-FANOUT-001` through `PERF-FANOUT-002` coordinator requirements.

## Crates and Feature Cells

- Modify Cargo package `clinkz-wot-core`; consume WP-000 foundation values and WP-100/WP-200
  core values without depending on Servient or a concrete protocol.
- The `no-default` cell exposes `PollClientBinding`, `PollServerBinding`,
  `StaticBindingRegistration<B>`, associated-state layouts, caller-owned typed slots,
  form-contribution values, state/status records, and public subscription/emission values
  without `Arc`, boxed futures, atomics, or an executor.
- The `async-no-std` cell preserves the poll contract and may provide native async adapters
  without executor selection.
- The `std` cell exposes object-safe server/client execution components and `HostBindingCall`,
  owned call boxes, owned route guards, `HostSubscriptionDriver`, host subscription start, and
  one complete `HostBindingRegistration`. Boxed futures are allowed only on these erased network
  paths; status, overflow, reactor, and ingress policies are fields of the bundle rather than a
  separately installable sink configuration.
- Use fake bindings and caller-owned tables in core integration tests. Do not implement zenoh,
  sockets, spawned transport tasks, or Servient registries in this package.

## Public API and Data Migration

Implement the frozen shared binding surface:

- values: `OutboundRequest`, `InboundRequest`, `InboundResponse`, `PrepareInput`,
  `BindingRouteKey`, `BindingContext`, `ResponseDelivery`, `SubscriptionStart`,
  `SubscriptionStopRequest`, `SubscriptionItem`, `SubscriptionDriverEvent`,
  `BindingInputRejection<T>`, `CleanupReservation`, `CleanupPhaseContext`, and
  `BindingCallSettlement<T>`;
- compiler-extension values: `BindingArtifactCompatibility`, `BindingArtifactFootprint`,
  `BindingArtifact`, `BindingArtifactEnvelope`, `BindingArtifactRef`, `BindingCompilerInput`,
  and `BindingCompilerExtension`; core owns this protocol-neutral SPI while WP-200 planning owns
  compiler coordination and plan-set construction;
- consume the WP-200 `CollectionSubscriptionCapability` unchanged when starting a root collection
  request; the SPI may not infer capability from protocol text or synthesize affordance fan-out;
- constrained traits and storage: `BindingStateLayout`, `PollClientBinding`,
  `PollServerBinding`, and typed `ClientRequestSlot<B::RequestState>`,
  `ClientSubscriptionSlot<B::SubscriptionState>`, route/readiness slots over the server
  associated states, `ServerResponseSlot<B::ResponseState>`, and
  `BindingEmissionSlot<B::EmissionState>`;
- host execution components: `ServerBinding`, owned prepared/readiness/active route wrappers,
  route-scoped `RouteAcceptEvent`, `BindingCallFootprint`, `HostBindingCall`,
  `HostBindingCallBox`, `ClientBinding`, `HostSubscriptionDriver`, and
  `HostSubscriptionStart`;
- installable units: `HostBindingRegistration` and `StaticBindingRegistration<B>`, each carrying
  compiler, execution, contribution, footprint, ingress, status, overflow, readiness, reactor,
  cleanup, capability, and profile-cell metadata as one validated startup bundle.
- Host and static complete registrations expose the same exact
  `try_with_collection_subscription_capability` and
  `collection_subscription_capability` methods, keyed only by
  `ObserveAllProperties` or `SubscribeAllEvents`; they do not infer native
  collection support from protocol text.

Implement the frozen contribution and runtime surfaces:

- `ServerFormContributor`, `AffordanceFormRequirement`, `FormContributionContext`,
  `FormContribution`, `FormContributionCapability`, `EndpointReservationKey`, and
  `CollisionDomainId`;
- `SubscriptionState`; the application `Subscription` facade and private `SubscriptionRecord`
  belong to WP-400 Servient;
- Preserve the orthogonality of `SubscriptionDriverEvent` fields: driver-slot lifecycle follows
  `CleanupOutcome`, while `ProcessTerminal` is retained unchanged for the parent facade. Complete
  cleanup retires the driver even when the process terminal is `Failed`; it must not be recoded as
  a driver residual.
- Implement the exact `binding-call` machine for host call records and every constrained typed
  slot header. Host constructors are nonblocking and side-effect-free, declare and report their
  complete lifetime footprint, and return owned `HostBindingCallBox` values. Cancellation binds
  a pre-admitted `CleanupReservation` into a phase-specific `CleanupPhaseContext`, retains the
  first cause, routes late request/subscription/response/publication results, and never drops a
  live call as cleanup.
- Make `poll_cancel_request`, `poll_cancel_subscription_start`, `poll_cancel_response`, and
  `poll_cancel_emission` return the portable settlement type. A returned late value, verified
  completion, transfer request, committed pending owner, and durable residual state are
  generation-safe and retain the complete work object until terminal acknowledgement. A
  `CleanupRecord` alone is never transferable work.
- `RuntimeEvent`, `BindingRuntimeEvent`, `BindingStatusRecord`, and `OverflowPolicy`;
- `ProducerEmission`, `EmissionKind`, `BindingPublication`, `EmissionStatus`, and
  `BindingEmissionSlot`;
- `BindingRouteState`, `InFlightState`, and the crate-private request, subscription, response,
  and emission slot state records.

All request, response, route, correlation, auth, payload, plan, binding-generation, and deadline
values are owned across an SPI call. Every consuming start has one typed pre-acceptance rejection
that returns the complete input. A registration carries the complete compiler/execution pairing,
identity, capabilities, readiness, diagnostics, ingress, reactor, status, overflow, cleanup, and
contributor metadata; a bare trait object is never the configuration contract.

## State and Ownership Migration

- A prepared route remains caller-addressable through every fallible readiness and activation
  outcome; commit failure returns an active guard suitable for shutdown. Readiness
  failure/cancellation uses abort; active or committed cleanup uses shutdown. `PendingCleanup`
  is returned only after the complete guard or call moves to and is acknowledged by the named
  cleanup owner.
- `BindingRouteState` follows the frozen route machine and never uses guard drop as a
  transition. Readiness, activation, commit, route-scoped acceptance, abort, shutdown, and retry
  are idempotent for one route generation; late callbacks with stale generations are discarded
  and recorded. There is one accept cursor and waker lease per active route, never one
  registration-wide `poll_accept` cursor.
- Admit an in-flight response opportunity only after the serving state and generation recheck.
  Host send consumes it in the call; constrained start consumes it only after the response is
  accepted into `ServerResponseSlot`.
- A constrained request slot is generic over its binding's associated request state and is
  consumed by a terminal result. A successful subscription start instead retains its typed
  slot/generation as `Active`; start cancellation, item polling, stop, state destruction, and
  terminal retention use that same slot.
- `HostSubscriptionDriver` is the object-safe receive/stop SPI. It has one linear receive cursor,
  returns `SubscriptionItem` with the exact `SubscriptionId` and `AffordanceTarget`, and drives
  wire teardown through one accepted `SubscriptionStopRequest` and the same retained cleanup
  state. Explicit teardown carries a selected `OutboundRequest`; drop uses an implicit request
  and cannot invent caller options. Core provides no queue, sender, cloneable consumer, or merge
  policy.
- A `BindingEmissionSlot` owns one immutable payload lease and one selected binding generation's
  publication and cleanup state. Servient's private `EmissionRecord` owns local-subscriber and
  binding-target cursors; core poll methods never perform engine-wide fan-out.
- Translate a legacy handler-path publication into exactly one admitted `ProducerEmission` at
  the compatibility boundary. Preserve payload ownership, target, subscription, route, binding,
  and generation identity; the adapter may not clone an unbounded stream or publish directly to
  a concrete protocol. WP-400 removes the handler-side adapter entry and WP-600 removes the
  protocol-side adapter exit.
- Classify runtime events before overflow. Critical details update the bounded durable status
  record before a queued copy can be dropped; no payload, credentials, or redacted TD fields
  enter status storage.
- Invoke readiness, transport, contributor, guard, event-sink, and status callbacks outside
  engine locks and critical sections. Reserve response and cleanup progress before new work.
- Declare and admit the immutable maximum lifetime footprint for every call, guard, driver,
  artifact, slot state, reactor queue, and ingress buffer before first side effect. Bound external
  ingress independently per route, per binding, per Thing where applicable, and globally; no
  hidden transport queue may turn zero or an omitted limit into unbounded storage.

## Old API Removal

- Replace the current `core/src/inbound.rs::ServerBinding` methods with the frozen
  route-scoped prepare/readiness/activate/commit/accept/abort/shutdown contract and the server
  component inside a complete registration bundle. Remove any registration-wide acceptance and
  any cleanup path whose only completion signal is guard drop or an unstructured outer error.
- Replace the current `core/src/outbound.rs::ClientBinding` request shape with the frozen owned
  `OutboundRequest`, validated output, owned `HostBindingCall`, `HostSubscriptionDriver`, and
  `HostSubscriptionStart` contracts. Remove `BindingRequest` and `BindingFuture`; no public
  compatibility alias remains at package completion.
- Remove public `TransportRequest`, `TransportResponse`, and `TransportAdapter` facades that
  bypass compiled route matches or duplicate protocol binding ownership.
- Remove direct push paths from the new binding SPI and reject any new registration that can
  publish without `ProducerEmission`, bounded subscriber/binding results, and explicit overflow
  accounting. Retain only the named migration adapters needed by existing WP-400 and WP-600
  callers. WP-400 removes `PushFn` and the `SubscriptionSender` handler path after host activation;
  WP-600 removes `PublisherSink` after both concrete backends migrate; WP-700 verifies that none
  is public or referenced.
- Remove the core-owned queue `Subscription`, `SubscriptionGuard`, `SubscriptionSender`,
  `EventStream`, `Subscription::merge`, `EventBroker`, and `EventName` routing key. The staged
  concrete `PublisherSink` call sites must migrate through WP-600 and are absent from the final
  target surface.
- Remove binding vectors embedded in consumed Things and bare trait objects as the stored
- Remove independently installable `ServerBindingRegistration`, `ClientBindingRegistration`,
  `StaticServerBindingRegistration`, `StaticClientBindingRegistration`,
  `RuntimeEventSinkConfig`, and `BindingDrivingMode` targets. Component values remain usable for
  constructing and testing one complete `HostBindingRegistration` or
  `StaticBindingRegistration<B>` only.
- Do not restore the removed `ProtocolBinding` or `ClientBindingFactory` facades, and do not
  retain a binding-owned unbounded pending request, subscription, response, or emission table.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `complete-binding-registration` for atomic compiler/execution/contributor/policy bundles,
  startup-only publication, rejection of incomplete bundles, and owned I/O values;
- `route-scoped-binding-lifecycle` for ownership-preserving route transitions, one accept/waker
  lease per route, terminal isolation, and absence of direct handler dispatch;
- `typed-binding-state-storage` for associated-state layout limits, typed slots, generation-safe
  construction/destruction, and zero-budget retention;
- `binding-lifetime-and-ingress-memory` for declared lifetime/transient footprints, per-route,
  per-binding, per-Thing/global ingress saturation, and hidden-buffer detection;
- `response-emission-input-preservation` for typed pre-acceptance rejection, exactly-once
  post-acceptance settlement, late results, and host/constrained classification parity;
- `form-finalization-and-collision` for deterministic contributions and reservation identity;
- `binding-slot-state-model` for route, in-flight, subscription, response, and emission states;
- `bounded-response-subscription-emission` for start/poll/cancel and terminal retention;
- `binding-response-validation` for the response XOR, producer and consumer validation entry
  points, identity/branch checks, action invariants, and additional-response bounds;
- `drop-and-cleanup-ownership` for complete work-object transfer, handoff rejection/manual
  fallback, idempotent teardown, deadline progress, and durable residual state.
- `producer-emission-migration` for the one-way legacy-adapter boundary, identity preservation,
  bounded admission, and proof that no new caller enters the bridge.
- `host-subscription-driver` for object safety, exact source attribution, one receive cursor,
  binding-owned flow control, stop/drop teardown, and absence of a core queue.
- `binding-emission-slot` for one-binding ownership, retained poll/cancel progress, stale
  generation rejection, and proof that Servient-wide fan-out is not stored in core.
- `binding-call-settlement` for constructor/poll/cancel races, late Returned routing, exact
  cleanup-reservation binding and acknowledged work transfer, declared footprint admission,
  zero-budget retry, and generation-safe slot reuse.

These records satisfy the corresponding requirement-index evidence families:

- `frozen-cross-crate-surface` for object safety, static registrations, owned values, and every
  applicable feature cell;
- `binding-io-ownership` for route/correlation generation, response opportunities, one decode,
  and structured binding errors;
- `bounded-subscription-response-progress` for start, pending, cancel, active, delivery, and
  terminal slot behavior;
- `state-machines` for every legal/illegal binding, in-flight, and subscription transition and
  stale callback;
- `expose-failure-injection` for readiness, abort, activation, commit, shutdown, cleanup
  transfer, and residual external state at the SPI boundary;
- `form-finalization` for deterministic contributions, index pruning, collision identity,
  owner ambiguity, limits, and rollback values;
- `overflow-status` for event classes, loss counters, critical journals/compaction, and shutdown
  progress under exhaustion;
- `drop-and-emission` for exactly-once teardown, payload sharing, per-target order, partial
  outcomes, cancellation cursors, and cleanup ownership;
- `manual-runtime`, `host-independent-progress`, `interaction-call-path`, and
  `zero-allocation-paths` for bounded poll and erased-adapter behavior.

Fake binding tests must prove that callbacks can reenter without an engine guard and that no
terminal or critical status is lost merely because a bounded queue is full.

## Performance Workloads

- `PERF-GW-007`, `PERF-CS-007`, `PERF-GW-018`, and `PERF-GW-019` cover subscription start,
  hot delivery, stop, and cancellation progress.
- `PERF-GW-008`, `PERF-CS-008`, and `PERF-CS-009` cover the binding-local payload lease and
  bounded slot progress; Servient-wide fan-out is measured by WP-400.
- `PERF-GW-009` and `PERF-GW-010` compare host-erased and poll metadata/allocation paths.
- `PERF-GW-024` covers exact per-binding publication result scaling, and `PERF-CS-018` proves
  that a retained `BindingEmissionSlot` resumes within its work budget without restarting.
- `PERF-GW-028` covers the owned host-call cancellation, late-result, transfer, and residual
  matrix; `PERF-CS-020` covers typed slots and complete pre-acceptance input rejection.
- `PERF-GW-030` and `PERF-CS-022` cover route preparation, readiness fairness, activation gates,
  guard retention, and route-scoped terminal isolation.
- `PERF-GW-031` validates a complete third-party registration and rejects every incomplete bundle.
- `PERF-GW-032` and `PERF-CS-023` cover bounded ingress items/bytes, backpressure, and hidden
  buffer detection at route, binding, Thing, and global scopes.

## Completion Conditions

- Every WP-300 ownership item exists at its frozen path and applicable feature cells; the
  no-default poll surface is useful and the host traits are object-safe.
- Complete host and static startup bundles are the only installable binding units; their compiler
  and execution compatibility, resource maxima, ingress policy, status policy, and supported
  profile cells are validated before snapshot publication.
- Core exposes no concrete subscription queue, merged stream, global emission coordinator, or
  dispatch policy; `BindingEmissionSlot` represents one binding generation only.
- Exhaustive transition tests cover route readiness through cleanup, in-flight admission and
  response consumption, subscription start/cancel/stop, emission poll/cancel, stale
  generations, and retained terminal outcomes.
- Slot/pool exhaustion returns backpressure before ownership transfer; accepted work reaches
  one terminal result without invoking the application handler again.
- Runtime event and durable status behavior remains bounded and preserves critical details
  under full-queue and full-journal cases.
- All listed workload adapters emit schema-valid, fixture-identified results for both poll and
  host-erased paths where applicable.
- The obsolete transport, push, split-registration, bare-registration, registration-wide accept,
  old-signature, opaque concrete-slot, and unbounded pending-work facades owned by WP-300 are
  absent, and no concrete protocol logic has entered core. Only the explicitly named
  WP-400/WP-600 compatibility adapter edges may remain, with compile and source evidence assigning
  their removal to those packages.
