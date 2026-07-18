# WP-400 Servient Lifecycle and Host Runtime

Status: Planned
Design revision: v4.9
Depends on: `WP-300`
Required gates: `GATE-1`, `GATE-2`, `GATE-3`, `GATE-4`, `GATE-5`, `GATE-6`
Owner packages: `clinkz-wot-servient`, `clinkz-wot-core`

## Scope

Implement the Servient composition and lifecycle layer after the binding registrations,
operation slots, cleanup records, and emission contracts from `WP-300` are complete. This
package owns the host `Servient`, the manually driven `StaticServient`, produced and consumed
handles, registry records, lifecycle transactions, cleanup ownership, the application-facing
`Subscription` facade, and profile-specific Producer emission coordination.

The work includes host and constrained construction paths, but it does not add a protocol driving
loop, a concrete transport, or a Directory service. `clinkz-wot-servient` composes frozen lower
layer contracts; it does not reinterpret forms, security expressions, Directory requests, or
binding-specific state. Work may begin only after `WP-300` is complete and every entry gate above
is closed.

Handler-origin response validation follows
`docs/amendments/WP-100-interaction-output-api-v1.md`: every producer response
uses WP-300 `InboundResponse::try_success`, which rejects binding metadata and
invalid action/status combinations. Servient does not implement a second
validator.

## Requirements

- `LIFE-EXPOSE-001`
- `LIFE-EXPOSE-002`
- `LIFE-EXPOSE-003`
- `API-PAYLOAD-001`
- `HANDLER-API-001`
- `HANDLER-SUB-001`
- `HANDLER-CANCEL-001`
- `HANDLER-CANCEL-002`
- `HANDLER-STORAGE-001`
- `BIND-IO-001`
- `BIND-OUT-001`
- `BIND-PROGRESS-001`
- `BIND-CALL-CANCEL-001`
- `BIND-HOST-CANCEL-001`
- `SUB-STORAGE-001`
- `SUB-DATA-001`
- `PRODUCER-EMIT-001`
- `STATE-EXPOSE-001`
- `STATE-SUB-001`
- `HANDLE-DROP-001`
- `CLEANUP-RECORD-001`
- `PERF-FANOUT-001`
- `PERF-FANOUT-002`
- `HOST-SHARD-001`
- `HOST-SHARD-002`
- `HOST-ASYNC-001`
- `HOST-DEFAULT-001`
- `HOST-DEFAULT-002`
- `OBS-PROFILE-001`

The package also consumes, without redefining, the `STATE-BIND-001`, `STATE-INFLIGHT-001`,
`HANDLE-DROP-001`, `PRODUCER-EMIT-001`, and `CLEANUP-RECORD-001` results delivered by `WP-300`.

## Crates and Feature Cells

| Cargo package | Feature cell | Required surface |
| --- | --- | --- |
| `clinkz-wot-servient` | `--no-default-features` | `StaticServient`, `StaticServientBuilder`, caller-owned records, and manual progress without host synchronization or erased futures |
| `clinkz-wot-servient` | `async`, no `std` | Async adapters over the same lifecycle and operation state without an executor dependency |
| `clinkz-wot-servient` | `std` | `Servient`, `ServientBuilder`, host handles, cleanup executor, sharded status, and named host defaults |
| `clinkz-wot-core` | all required cells | Frozen binding registrations, route and operation identities, runtime events, cleanup outcomes, and dispatch values consumed by the Servient |
| `clinkz-wot-foundation` | all required cells | `ResourceLimits`, `WorkBudget`, clocks, generations, reservations, and named profiles consumed without a higher-layer dependency |

The Servient may depend on `clinkz-wot-discovery` and
`clinkz-wot-planning`, but neither package may depend back on the Servient. A concrete
protocol package is not a mandatory dependency of `clinkz-wot-servient`; protocol-specific test
integration remains behind an explicit test feature.

## Public API and Data Migration

- Add the frozen `clinkz_wot_servient::StaticServient`,
  `clinkz_wot_servient::StaticSubscription`, and
  `clinkz_wot_servient::StaticServientBuilder` surfaces for caller-owned storage and manual
  progress. Their `step` operation uses `WorkBudget` and returns
  `StepStatus<RuntimeEvent>` exactly as frozen by `API-SURFACE-001`.
- Replace the current host builder's bare binding vectors with
  `ServerBindingRegistration` and `ClientBindingRegistration` snapshots. Convenience methods may
  still accept `Arc<dyn ServerBinding>` or `Arc<dyn ClientBinding>` and construct complete default
  registrations.
- Replace the current `Servient`, `ServientBuilder`, `ExposedThingHandle`, and
  `ConsumedThingHandle` implementations while preserving those frozen public names. Add
  `CleanupTask`, `CleanupExecutor`, and the public `ExposeState`; keep `ExposedThingRecord`,
  `BindingRouteRecord`, `InFlightRecord`, `StaticServientRecord`, and `CleanupQueueRecord`
  crate-private as assigned by `docs/api-ownership.csv`.
- Keep every host invoke, subscription-start, and publication operation in one crate-private
  `HostBindingCallRecord` across constructor, result poll, cancel, late return, cleanup transfer,
  and residual settlement. Reserve per-binding/per-Thing/global call counts, declared footprint,
  result capacity, and cleanup item/bytes before the side-effect-free binding constructor.
- Implement the exact lossless cleanup handoff: `CleanupExecutor::try_spawn` either accepts the
  complete non-Clone `CleanupTask` or returns that same task to its already reserved manual queue
  slot. `Servient::poll_cleanup` drives that queue with an explicit `WorkBudget`. Executor
  shutdown/drop commits the task's pre-reserved residual fallback before destroying a live call.
- Add the non-`Clone` `clinkz_wot_servient::Subscription` facade and keep its driver registry in
  the crate-private `SubscriptionRecord`, keyed by `SubscriptionId`. One facade owns one receive
  cursor. `stop` validates options, builds one WP-300 `SubscriptionStopRequest`, and drives the
  binding driver's start/poll teardown. Drop uses the implicit form and retained cleanup state;
  neither path merely stops a local queue or fabricates caller input.
- Translate `SubscriptionDriverEvent::Terminal` into one public `ProcessEvent::Terminal` only
  after retaining its paired cleanup outcome. `StaticSubscription` performs the same translation
  through `StaticServient` without a boxed host driver or hidden default work budget.
- Keep `SubscriptionState` orthogonal to `ProcessTerminal`: complete cleanup reaches `Closed`
  even for a retained source failure, while `Failed` represents durable residual external state.
  A no-resource pre-publication start error closes only its private start record. Expose both
  dimensions through `terminal()` and
  `cleanup_outcome()`.
- Add the public configurable host `EmissionDispatchPolicy` and the crate-private
  `EmissionCoordinator` and `EmissionRecord`. The record retains local-subscriber and
  binding-target cursors, admitted result capacity, payload lease, and terminal outcome. The
  constrained runtime drives the same semantics directly with `WorkBudget`; it does not
  instantiate the host policy.
- Activate host handler registration only through `ExposedThingHandle`. For every operation stem
  frozen by WP-100, implement exactly `set_{operation_snake}_handler`,
  `set_async_{operation_snake}_handler`, `set_step_{operation_snake}_handler`, and
  `clear_{operation_snake}_handler`, plus `clear_handlers`. The 73 associated public paths are
  individually frozen in `docs/api-ownership.csv`. Property, action, and event affordance
  operations take the affordance name before the handler; the nine Thing-level/collection
  operations omit it. A setter admits `HandlerFootprint` and publishes one new generation or
  leaves the old generation unchanged; a clear operation is explicit and generation-bearing.
- The exact operation snake names are `read_property`, `write_property`, `observe_property`,
  `unobserve_property`, `invoke_action`, `query_action`, `cancel_action`, `subscribe_event`,
  `unsubscribe_event`, `read_all_properties`, `write_all_properties`,
  `read_multiple_properties`, `write_multiple_properties`, `observe_all_properties`,
  `unobserve_all_properties`, `query_all_actions`, `subscribe_all_events`, and
  `unsubscribe_all_events`. Affordance-first legacy spellings are not aliases.
- `StaticServient` consumes application-owned generated/static handler tables through
  `StaticHandlerRegistration`, the WP-100 traits, and `HandlerSlotId`; it does not define a second
  public heterogeneous constrained registry or erase an associated step state.
- Change Scripting-compatible `produce` to accept the documented
  `ExposedThingInit` or Partial TD input. Provide explicitly named `produce_td` and
  `produce_document` paths for complete `ThingDescription` and `TdDocument` inputs rather than
  overloading one method with incompatible source-retention semantics.
- Build `consume` from the immutable plans produced by `WP-200`; the handle captures binding
  registrations and plan generations rather than registering bindings into a mutable
  `ConsumedThing` after construction.
- The standard `ConsumedThingHandle::subscribe_all_events` and `observe_all_properties` methods
  execute exactly one selected root-form `OutboundRequest` and install the returned binding-owned
  driver. If no exact-source native collection plan exists, return the structured
  no-compatible-form error. Do not enumerate affordances or merge local streams.
- Keep `discover`, `explore_directory`, and `request_thing_description` as client facades over an
  injected `Discoverer`. Scripting-compatible methods expose bare TD views; explicitly named
  document methods expose `TdDocument` source envelopes.
- Route all public failures through the frozen `CoreError` taxonomy and retain bounded,
  generation-safe error context. Any crate-specific convenience error must not duplicate or hide
  a `CoreError` category.

## State and Ownership Migration

- Implement the complete `Draft -> Preparing -> ReadyPendingActivation -> Activating ->
  Committing -> Serving` exposure transaction. The only publication linearization point is the
  shared `Committing -> Serving` registry transition.
- Store every prepared guard, readiness token, active guard, reservation, and primary failure in
  the `ExposedThingRecord` until cleanup is complete, residual, or atomically transferred to the
  bounded cleanup owner. A failed call must never leave unaddressable binding state.
- Make cancellation, expose-future drop, handle drop, and `destroy` race against publication at
  the state-machine boundary. Before publication they enter `Cancelling`; after publication they
  enter `Draining`. Drop transfers cleanup without blocking.
- Mark a serving registry generation draining before rejecting new requests and beginning binding
  shutdown. In-flight admission rechecks the serving generation at the same synchronization
  boundary; late handler results lose their response opportunity and are reported rather than
  reviving the route.
- Implement Producer observe/subscribe as one transaction owned by
  `ProducerSubscriptionOwner`: reserve the `SubscriptionId`, setup-call capacity,
  `subscription_bytes`, Producer record, and local guard slot before user setup;
  retain the paired teardown generation without pre-charging a live teardown call
  or cleanup slot; invoke setup outside engine guards; install binding/local
  guards; then and only then publish `Active`.
  Rejection, cancellation, drop, late acceptance, or install failure runs bounded rollback.
  A published subscription invokes the matched application teardown at most once under
  `CallbackLease`; repeated stop returns the retained terminal result. Failed teardown transfers
  exactly one bounded `HandlerCleanupOwner` or closes terminal with the structured residual
  outcome. No sample is published before `Active`.
- On cancellation, drop an async handler future at the first engine cancellation boundary outside
  locks. Repeatedly drive a step handler's explicit cancel entry within `HandlerSteps` and the
  drain-step limit. A non-preemptible sync callback may lose its response opportunity but retains
  its `HandlerCallOwner`, `CallbackLease`, selected generation, input, and context until actual
  return; its late result is never delivered.
- Keep registry structure separate from per-Thing lifecycle, handler, in-flight, subscription, and
  status state. Release structural guards before validation, security providers, codecs, binding
  calls, user handlers, and event/status callbacks.
- Use per-binding or bounded-shard runtime event queues and durable status records. Critical
  lifecycle status is retained before any best-effort aggregate notification, and unrelated
  Things or bindings do not share one mandatory hot-path mutex.
- Drive Producer emission from the frozen publication target set. Preserve full
  `AffordanceTarget`, route and binding generations, and per-affordance order; isolate slow or
  full binding lanes and invoke every binding outside engine locks. A TD protocol label or runtime
  TD rescan never creates a publication target.
- Reserve cleanup capacity at construction. `CleanupOutcome::PendingCleanup` is published only
  after the guard and remaining operation have transferred atomically to the owner named by the
  `CleanupRecord`.

## Old API Removal

- Remove Servient calls to the legacy `ServerBinding::serve(thing_id, td, context)` and
  `ServerBinding::shutdown(thing_id)` APIs. Lifecycle integration must use the prepared route,
  readiness, activation, commit, bounded shutdown, and cleanup contracts completed by `WP-300`.
- Remove builder-created `LocalDiscoverer` and `InMemoryDirectory` defaults. A Servient
  constructor must never create an in-process Directory; an omitted client integration is an
  explicit unavailable capability, not a hidden service.
- Remove the current single `draining: AtomicBool` registry model and the shutdown-before-drain
  ordering. It cannot represent preparation, cancellation, cleanup ownership, generations, or
  in-flight linearization.
- Remove post-construction `ConsumedThing::register_binding` assembly and any hot-path scan of the
  current bare binding arrays. Handles use captured registration generations and compiled indexes.
- Remove or make private `ShutdownHandle` if it only toggles the current unowned global flag.
  Shutdown must instead be a documented, bounded lifecycle operation with retained cleanup and
  status outcomes.
- Remove legacy Servient error variants and public aliases that collapse the frozen error taxonomy
  or omit binding, plan, generation, correlation, and cleanup context.
- After every host registration call site uses the 73 target methods, remove `PushFn` and every
  `SubscriptionSender`-based handler setup/publication path. Remove the legacy affordance-first
  sync/async handler traits and setters, all nine public `*Slot` enums (`ReadSlot`, `WriteSlot`,
  `ObserveSlot`, `UnobserveSlot`, `InvokeSlot`, `QuerySlot`, `CancelSlot`, `SubscribeSlot`, and
  `UnsubscribeSlot`), and all nine public raw handler lookup methods. Do not carry these names
  behind a compatibility feature. `PublisherSink` remains solely for WP-600 protocol migration
  and is not callable from the migrated Servient.
- Remove Servient storage or calls for `EventBroker`, `EventName`, core `Subscription`,
  `SubscriptionGuard`, `EventStream`, `Subscription::merge`, and `SubscriptionSender`. Standard
  collection methods have no implicit per-affordance fan-out fallback.

No compatibility facade may keep the removed lifecycle callable on a releasable feature cell.

## Evidence

- `servient-expose-failure-injection`: exhaustive prepare, readiness, activate, commit,
  cancellation, timeout, and rollback failure injection with retained primary and cleanup results.
- `servient-cleanup-outcomes`: drop, destroy, full cleanup queue, pending cleanup, residual state,
  idempotence, and stale-generation evidence.
- `host-binding-call-ownership`: construction cancellation, late Returned routing, independent
  cancel-drain deadline, Complete/PendingCleanup/Residual settlement, declared footprint
  accounting, executor accept/reject/shutdown, manual cleanup progress, and zero owner loss.
- `servient-constrained-fairness`: bounded manual steps, round-robin progress, reserved response
  and cleanup work, and no executor or atomic-reference-counting dependency.
- `servient-response-validation`: every handler-origin result passes through the WP-300
  `InboundResponse::try_success` boundary using the admitted route-match operation, including
  binding-metadata and action/status failure cases plus route/generation/correlation rechecks.
- `host-independent-progress`: contention evidence that unrelated Things and bindings progress
  independently and that callbacks execute outside engine locks.
- `host-default-snapshots`: exhaustive `GatewayDefaultV1` and `DirectoryClientDefaultV1` timeout,
  source-retention, overflow, observability, and emission-policy defaults. The exact
  `GatewayDefaultV1` emission snapshot is one lane per binding generation and sixteen in-flight
  publications per lane.
- `producer-subscription-transaction`: handler accept/reject, guard install, cancel/drop during
  setup, late accept, install rollback, active stop, repeated stop, teardown failure, retained
  cleanup, reentrant stop, and proof of exactly-once setup/teardown callback leases. Gateway
  evidence covers the complete sync/async/step setup-by-teardown matrix; constrained evidence
  covers the complete sync/step setup-by-teardown matrix.
- `binding-owned-subscription-driver`: single and native collection starts, exact source items,
  one receive cursor, binding backpressure, stop/drop, cleanup transfer, and no core queue.
- `emission-coordinator`: full-target identity, pre-admission, per-affordance order, retained
  cursors, payload sharing, partial outcomes, slow-lane isolation, and host/constrained policies.
- `native-collection-subscriptions`: one root-form request and one driver for each standard
  collection operation, plus structured rejection without implicit fan-out.

The evidence must also include compile fixtures for all three feature cells and model tests that
cover every legal and illegal transition in the `expose`, `binding-route`, and `in-flight`
machines.

## Performance Workloads

- Gateway: `PERF-GW-001`, `PERF-GW-002`, `PERF-GW-007`, `PERF-GW-008`,
  `PERF-GW-011`, `PERF-GW-012`, `PERF-GW-013`, `PERF-GW-016`, `PERF-GW-017`,
  `PERF-GW-018`, and `PERF-GW-019`.
- Constrained: `PERF-CS-012`.
- Emission cursor and isolation: `PERF-CS-008` and `PERF-CS-009`.
- Handler transaction: `PERF-GW-022` and `PERF-CS-017`.
- Emission coordination: `PERF-GW-024`, `PERF-GW-025`, and `PERF-GW-026` cover binding-count
  scaling, slow-lane isolation, and committed publication-target construction;
  `PERF-CS-018` covers retained constrained binding progress.
- Native collection subscriptions: `PERF-GW-027` and `PERF-CS-019` prove one root-form driver,
  exact source attribution, and no local merge queues.

Every gating workload must run with the locked fixture and named resource profile. The erased
async and allocation-sensitive paths remain distinct result series. `PERF-GW-018` is
characterization only and cannot close a performance requirement by itself. `PERF-GW-022`
executes all nine setup/teardown flavor pairs at 1,024 subscribers. `PERF-CS-017` executes all
four supported flavor pairs at the static profile maximum of 256 subscribers. Active
subscriptions retain ordinary teardown obligations without consuming cleanup-queue entries;
`cleanup_items_max` applies only when an actual transfer to `HandlerCleanupOwner` occurs, and
exhausted transfer capacity falls back to durable residual recording.

## Completion Conditions

- `WP-300` is complete, all entry gates remain closed, and no lower crate acquires a Servient
  dependency.
- All frozen Servient items have the owner, visibility, public path, feature cells, and migration
  disposition recorded in `docs/api-ownership.csv`.
- Host and constrained exposure, destroy, drop, subscription, cleanup, and emission integration
  pass their state-model and failure-injection evidence without a leaked guard or reservation.
- Application subscription tests prove linear receive ownership and binding-driven teardown;
  collection tests prove one native root operation and no hidden per-affordance merge.
- All 73 host registration methods have positive type-check fixtures; incompatible operation
  traits fail to compile; replacement, clear, cancellation, and Producer subscription transaction
  cases pass with no leaked owner, late delivery, or duplicate callback.
- Every required feature cell compiles with its documented public surface; `async` alone pulls no
  executor and `--no-default-features` exposes a useful manual runtime.
- No Servient constructor creates a Directory service, and no protocol driving loop is owned by
  the Servient.
- The listed performance workloads satisfy their absolute budgets and structural invariants, with
  result identities accepted by `tools/performance-harness`.
- The legacy lifecycle and default in-process Directory APIs listed above are absent from public
  compile fixtures and internal call sites.
- `PushFn`, the `SubscriptionSender` handler path, legacy handler traits/setters, raw slot enums,
  and raw lookup methods are absent. The only remaining staged emission debt is the WP-600-owned
  concrete-protocol `PublisherSink` edge.
