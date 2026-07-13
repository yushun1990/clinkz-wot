# WP-300 Binding Contracts and Progress

Status: Planned

Design revision: v4.6

Depends on: WP-200

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core, clinkz-wot-protocol-bindings

## Scope

Replace the host-only binding shapes in `clinkz-wot-core` with the frozen host registration
and constrained poll contracts. Implement route readiness, request/response ownership,
subscription start/stop, Producer emission, runtime status, form contribution, generation-safe
operation slots, and bounded cleanup progress without adding a concrete protocol.

This package defines and tests the execution SPI consumed by Servient and protocol packages.
WP-400 owns expose/destroy orchestration and registries; WP-600 owns zenoh and zenoh-pico
implementations. No protocol-specific route, transport, or authentication semantics enter core.

## Requirements

- `API-SURFACE-001`, `BIND-IO-001`, `BIND-OUT-001`, and `BIND-PROGRESS-001` freeze host and
  poll execution, ownership, response, cancellation, and subscription progress.
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
- `HOST-ASYNC-001`, `HOST-SHARD-001`, `PERF-CALL-001`, `PERF-FANOUT-001`,
  `PERF-FANOUT-002`, and `PERF-ALLOC-001` govern erased adapters, independent progress, and
  allocation-sensitive paths.

## Crates and Feature Cells

- Modify Cargo package `clinkz-wot-core`; consume WP-000 foundation values and WP-100/WP-200
  core values without depending on Servient or a concrete protocol.
- The `no-default` cell exposes `PollClientBinding`, `PollServerBinding`, static registration
  values, caller-owned slots, form-contribution values, state/status records, and public
  subscription/emission values without `Arc`, boxed futures, atomics, or an executor.
- The `async-no-std` cell preserves the poll contract and may provide native async adapters
  without executor selection.
- The `std` cell exposes object-safe `ServerBinding` and `ClientBinding`, `BindingFuture`,
  route guards, host subscription start, runtime event sink configuration, and explicit
  registration values. Boxed futures are allowed only on these erased network paths.
- Use fake bindings and caller-owned tables in core integration tests. Do not implement zenoh,
  sockets, spawned transport tasks, or Servient registries in this package.

## Public API and Data Migration

Implement the frozen binding surface:

- values: `BindingRequest`, `InboundRequest`, `InboundResponse`, `PrepareInput`,
  `BindingRouteKey`, `BindingContext`, `ResponseDelivery`, `SubscriptionStart`, and
  `SubscriptionItem`;
- constrained slots/traits: `ClientRequestSlot`, `ClientSubscriptionSlot`,
  `ServerResponseSlot`, `PollClientBinding`, and `PollServerBinding`;
- host traits/registrations: `ServerBinding`, `ServerRouteGuard`, `ActiveRouteGuard`,
  `RouteReadinessDriver`, `RouteReadinessToken`, `ServerBindingRegistration`,
  `RuntimeEventSinkConfig`, `BindingFuture`, `ClientBinding`, `HostSubscriptionStart`, and
  `ClientBindingRegistration`;
- common/static registration: `RouteReadinessStatus`, `BindingDrivingMode`,
  `StaticServerBindingRegistration`, and `StaticClientBindingRegistration`.

Implement the frozen contribution and runtime surfaces:

- `ServerFormContributor`, `AffordanceFormRequirement`, `FormContributionContext`,
  `FormContribution`, `FormContributionCapability`, `EndpointReservationKey`, and
  `CollisionDomainId`;
- `Subscription`, `SubscriptionGuard`, `SubscriptionState`, and crate-private
  `SubscriptionRecord`;
- `RuntimeEvent`, `BindingRuntimeEvent`, `BindingStatusRecord`, and `OverflowPolicy`;
- `ProducerEmission`, `EmissionKind`, `BindingPublication`, `EmissionStatus`, and
  `ServerEmissionSlot`;
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
- A `ServerEmissionSlot` owns its immutable payload lease and generation-safe local-subscriber
  and binding-target cursors. Poll resumes the cursors; cancel retains already accepted
  per-binding outcomes and never restarts fan-out.
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
- Replace the current `core/src/binding.rs::ClientBinding` request shape with the frozen owned
  `BindingRequest`, validated output, and `HostSubscriptionStart` contract. Keep the name, not
  the obsolete signature or behavior.
- Remove public `TransportRequest`, `TransportResponse`, and `TransportAdapter` facades that
  bypass compiled route matches or duplicate protocol binding ownership.
- Remove `PublisherSink`, `SubscriptionSender`, and direct push paths that publish without
  `ProducerEmission`, bounded subscriber/binding results, and explicit overflow accounting.
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
- `drop-and-cleanup-ownership` for guard transfer, idempotent teardown, and residual state.

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
- `PERF-GW-008`, `PERF-CS-008`, and `PERF-CS-009` cover shared-payload fan-out and bounded
  cursor progress.
- `PERF-GW-009` and `PERF-GW-010` compare host-erased and poll metadata/allocation paths.
## Completion Conditions

- Every WP-300 ownership item exists at its frozen path and applicable feature cells; the
  no-default poll surface is useful and the host traits are object-safe.
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
  facades listed above are absent, and no concrete protocol logic has entered core.
