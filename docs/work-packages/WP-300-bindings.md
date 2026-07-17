# WP-300 Binding Contracts and Binding-Local Progress

Status: Planned

Design revision: v4.8

Depends on: WP-200

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core

## Scope

Replace the host-only binding shapes in `clinkz-wot-core` with the frozen host registration
and constrained poll contracts. Implement route readiness, request/response ownership,
subscription start/stop, Producer emission, runtime status, form contribution, generation-safe
operation slots, binding-owned subscription progress, binding-local publication slots, and
bounded cleanup progress without adding a concrete protocol or a Servient scheduler.

This package defines and tests the execution SPI consumed by Servient and protocol packages.
WP-400 owns expose/destroy orchestration and registries; WP-600 owns zenoh and zenoh-pico
implementations. No protocol-specific route, transport, or authentication semantics enter core.

This is the only package that introduces `ProducerEmission`. Core defines the immutable emission
values and a `BindingEmissionSlot` for exactly one selected binding generation; it does not own
the Servient-wide fan-out record or a concrete dispatch policy. This package also provides bounded internal
compatibility adapters so the still-unmigrated WP-400 Servient and WP-600 concrete binding can
cross this checkpoint without a dependency cycle. An adapter is not a second public emission
contract and accepts no new callers after this package completes.

The final XOR-shaped `InboundResponse`, its producer-origin `try_success`
validation, and the public shared consumer-origin
`validate_untrusted_binding_output` function must follow
`docs/amendments/WP-100-interaction-output-api-v1.md`. This package replaces the
legacy response envelope once, after the route and planning values from WP-200
exist.

## Requirements

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
- The `no-default` cell exposes `PollClientBinding`, `PollServerBinding`, static registration
  values, caller-owned slots, form-contribution values, state/status records, and public
  subscription/emission values without `Arc`, boxed futures, atomics, or an executor.
- The `async-no-std` cell preserves the poll contract and may provide native async adapters
  without executor selection.
- The `std` cell exposes object-safe `ServerBinding`, `ClientBinding`, and `HostBindingCall`,
  owned call boxes, route guards, `HostSubscriptionDriver`, host subscription start, runtime event sink
  configuration, and explicit registration values. Boxed futures are allowed only on these
  erased network paths.
- Use fake bindings and caller-owned tables in core integration tests. Do not implement zenoh,
  sockets, spawned transport tasks, or Servient registries in this package.

## Public API and Data Migration

Implement the frozen binding surface:

- values: `OutboundRequest`, `InboundRequest`, `InboundResponse`, `PrepareInput`,
  `BindingRouteKey`, `BindingContext`, `ResponseDelivery`, `SubscriptionStart`,
  `SubscriptionStopRequest`, `SubscriptionItem`, `SubscriptionDriverEvent`,
  `CleanupTransferContext`, and `BindingCallSettlement`;
- consume the WP-200 `CollectionSubscriptionCapability` unchanged when starting a root collection
  request; the SPI may not infer capability from protocol text or synthesize affordance fan-out;
- constrained slots/traits: `ClientRequestSlot`, `ClientSubscriptionSlot`,
  `ServerResponseSlot`, `PollClientBinding`, and `PollServerBinding`;
- host traits/registrations: `ServerBinding`, `ServerRouteGuard`, `ActiveRouteGuard`,
  `RouteReadinessDriver`, `RouteReadinessToken`, `ServerBindingRegistration`,
  `RuntimeEventSinkConfig`, `BindingCallFootprint`, `HostBindingCall`, `HostBindingCallBox`,
  `ClientBinding`, `HostSubscriptionDriver`, `HostSubscriptionStart`, and
  `ClientBindingRegistration`;
- common/static registration: `RouteReadinessStatus`, `BindingDrivingMode`,
  `StaticServerBindingRegistration`, and `StaticClientBindingRegistration`.
- Host and static client registrations expose the same exact
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
- Implement the exact `binding-call` machine for host call records and all four constrained slot
  headers. Host constructors are nonblocking and side-effect-free, declare and report retained
  footprint, and return owned `HostBindingCallBox` values. Cancellation uses
  `BindingCallSettlement`, retains the first cause, routes late request/subscription/publication
  results, validates `CleanupTransferContext`, and never drops a live call as cleanup.
- Make `poll_cancel_request`, `poll_cancel_subscription_start`, `poll_cancel_response`, and
  `poll_cancel_emission` return the portable settlement type. A returned late value and each of
  Complete, PendingCleanup, and ResidualExternalState must be generation-safe and must retain the
  slot until its terminal acknowledgement.
- `RuntimeEvent`, `BindingRuntimeEvent`, `BindingStatusRecord`, and `OverflowPolicy`;
- `ProducerEmission`, `EmissionKind`, `BindingPublication`, `EmissionStatus`, and
  `BindingEmissionSlot`;
- `BindingRouteState`, `InFlightState`, and the crate-private request, subscription, response,
  and emission slot state records.

All request, response, route, correlation, auth, payload, plan, binding-generation, and
deadline values are owned across an SPI call. A registration carries identity, capabilities,
driving mode, readiness, diagnostics, overflow, and contributor metadata; bare trait-object
builder conveniences are not the configuration contract.

## State and Ownership Migration

- A prepared route remains caller-addressable until activation consumes its generation.
  Readiness failure/cancellation uses abort; active or committed cleanup uses shutdown.
  `PendingCleanup` is returned only after atomic transfer to the named cleanup owner.
- `BindingRouteState` follows the frozen route machine and never uses guard drop as a
  transition. Readiness, abort, shutdown, and retry are idempotent for one route generation;
  late callbacks with stale generations are discarded and recorded.
- Admit an in-flight response opportunity only after the serving state and generation recheck.
  Host send consumes it in the call; constrained start consumes it only after the response is
  accepted into `ServerResponseSlot`.
- A constrained request slot is consumed by a terminal result. A successful subscription start
  instead retains its slot/generation as `Active`; start cancellation, item polling, stop, and
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

## Old API Removal

- Replace the current `core/src/inbound.rs::ServerBinding` methods with the frozen
  prepare/readiness/activate/commit/abort/shutdown contract and explicit
  `ServerBindingRegistration`. Remove any cleanup path whose only completion signal is guard
  drop or an unstructured outer error.
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
  Servient configuration record. Host conveniences may wrap a trait object in a complete
  registration at the call boundary.
- Do not restore the removed `ProtocolBinding` or `ClientBindingFactory` facades, and do not
  retain a binding-owned unbounded pending request, subscription, response, or emission table.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `binding-registration-ownership` for registration metadata and owned I/O values;
- `form-finalization-and-collision` for deterministic contributions and reservation identity;
- `binding-slot-state-model` for route, in-flight, subscription, response, and emission states;
- `bounded-response-subscription-emission` for start/poll/cancel and terminal retention;
- `binding-response-validation` for the response XOR, producer and consumer validation entry
  points, identity/branch checks, action invariants, and additional-response bounds;
- `drop-and-cleanup-ownership` for guard transfer, idempotent teardown, and residual state.
- `producer-emission-migration` for the one-way legacy-adapter boundary, identity preservation,
  bounded admission, and proof that no new caller enters the bridge.
- `host-subscription-driver` for object safety, exact source attribution, one receive cursor,
  binding-owned flow control, stop/drop teardown, and absence of a core queue.
- `binding-emission-slot` for one-binding ownership, retained poll/cancel progress, stale
  generation rejection, and proof that Servient-wide fan-out is not stored in core.
- `binding-call-settlement` for constructor/poll/cancel races, late Returned routing, exact cleanup
  transfer, declared footprint admission, zero-budget retry, and generation-safe slot reuse.

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
## Completion Conditions

- Every WP-300 ownership item exists at its frozen path and applicable feature cells; the
  no-default poll surface is useful and the host traits are object-safe.
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
- The obsolete transport, push, bare-registration, old-signature, and unbounded pending-work
  facades owned by WP-300 are absent, and no concrete protocol logic has entered core. Only the
  explicitly named WP-400/WP-600 compatibility adapter edges may remain, with compile and source
  evidence assigning their removal to those packages.
