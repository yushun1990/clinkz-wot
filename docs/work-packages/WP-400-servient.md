# WP-400 Servient Lifecycle and Host Runtime

Status: Planned
Design revision: v4.6
Depends on: `WP-300`
Required gates: `GATE-1`, `GATE-2`, `GATE-3`, `GATE-4`, `GATE-5`, `GATE-6`
Owner packages: `clinkz-wot-servient`, `clinkz-wot-core`

## Scope

Implement the Servient composition and lifecycle layer after the binding registrations,
operation slots, cleanup records, and emission contracts from `WP-300` are complete. This
package owns the host `Servient`, the manually driven `StaticServient`, produced and consumed
handles, registry records, lifecycle transactions, cleanup ownership, and the application-facing
Discovery facade.

The work includes host and constrained construction paths, but it does not add a protocol driving
loop, a concrete transport, or a Directory service. `clinkz-wot-servient` composes frozen lower
layer contracts; it does not reinterpret forms, security expressions, Directory requests, or
binding-specific state. Work may begin only after `WP-300` is complete and every entry gate above
is closed.

## Requirements

- `LIFE-EXPOSE-001`
- `LIFE-EXPOSE-002`
- `LIFE-EXPOSE-003`
- `STATE-EXPOSE-001`
- `STATE-SUB-001`
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
`clinkz-wot-protocol-bindings`, but neither package may depend back on the Servient. A concrete
protocol package is not a mandatory dependency of `clinkz-wot-servient`; protocol-specific test
integration remains behind an explicit test feature.

## Public API and Data Migration

- Add the frozen `clinkz_wot_servient::StaticServient` and
  `clinkz_wot_servient::StaticServientBuilder` surfaces for caller-owned storage and manual
  progress. Their `step` operation uses `WorkBudget` and returns
  `StepStatus<RuntimeEvent>` exactly as frozen by `API-SURFACE-001`.
- Replace the current host builder's bare binding vectors with
  `ServerBindingRegistration` and `ClientBindingRegistration` snapshots. Convenience methods may
  still accept `Arc<dyn ServerBinding>` or `Arc<dyn ClientBinding>` and construct complete default
  registrations.
- Replace the current `Servient`, `ServientBuilder`, `ExposedThingHandle`, and
  `ConsumedThingHandle` implementations while preserving those frozen public names. Add
  `CleanupExecutor` and the public `ExposeState`; keep `ExposedThingRecord`,
  `BindingRouteRecord`, `InFlightRecord`, `StaticServientRecord`, and `CleanupQueueRecord`
  crate-private as assigned by `docs/api-ownership.csv`.
- Change Scripting-compatible `produce` to accept the documented
  `ExposedThingInit` or Partial TD input. Provide explicitly named `produce_td` and
  `produce_document` paths for complete `ThingDescription` and `TdDocument` inputs rather than
  overloading one method with incompatible source-retention semantics.
- Build `consume` from the immutable plans produced by `WP-200`; the handle captures binding
  registrations and plan generations rather than registering bindings into a mutable
  `ConsumedThing` after construction.
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
- Keep registry structure separate from per-Thing lifecycle, handler, in-flight, subscription, and
  status state. Release structural guards before validation, security providers, codecs, binding
  calls, user handlers, and event/status callbacks.
- Use per-binding or bounded-shard runtime event queues and durable status records. Critical
  lifecycle status is retained before any best-effort aggregate notification, and unrelated
  Things or bindings do not share one mandatory hot-path mutex.
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

No compatibility facade may keep the removed lifecycle callable on a releasable feature cell.

## Evidence

- `servient-expose-failure-injection`: exhaustive prepare, readiness, activate, commit,
  cancellation, timeout, and rollback failure injection with retained primary and cleanup results.
- `servient-cleanup-outcomes`: drop, destroy, full cleanup queue, pending cleanup, residual state,
  idempotence, and stale-generation evidence.
- `servient-constrained-fairness`: bounded manual steps, round-robin progress, reserved response
  and cleanup work, and no executor or atomic-reference-counting dependency.
- `host-independent-progress`: contention evidence that unrelated Things and bindings progress
  independently and that callbacks execute outside engine locks.
- `host-default-snapshots`: exhaustive `GatewayDefaultV1` and `DirectoryClientDefaultV1` timeout,
  source-retention, overflow, and observability defaults.

The evidence must also include compile fixtures for all three feature cells and model tests that
cover every legal and illegal transition in the `expose`, `binding-route`, and `in-flight`
machines.

## Performance Workloads

- Gateway: `PERF-GW-001`, `PERF-GW-002`, `PERF-GW-007`, `PERF-GW-008`,
  `PERF-GW-011`, `PERF-GW-012`, `PERF-GW-013`, `PERF-GW-016`, `PERF-GW-017`,
  `PERF-GW-018`, and `PERF-GW-019`.
- Constrained: `PERF-CS-012`.

Every gating workload must run with the locked fixture and named resource profile. The erased
async and allocation-sensitive paths remain distinct result series. `PERF-GW-018` is
characterization only and cannot close a performance requirement by itself.

## Completion Conditions

- `WP-300` is complete, all entry gates remain closed, and no lower crate acquires a Servient
  dependency.
- All frozen Servient items have the owner, visibility, public path, feature cells, and migration
  disposition recorded in `docs/api-ownership.csv`.
- Host and constrained exposure, destroy, drop, subscription, cleanup, and emission integration
  pass their state-model and failure-injection evidence without a leaked guard or reservation.
- Every required feature cell compiles with its documented public surface; `async` alone pulls no
  executor and `--no-default-features` exposes a useful manual runtime.
- No Servient constructor creates a Directory service, and no protocol driving loop is owned by
  the Servient.
- The listed performance workloads satisfy their absolute budgets and structural invariants, with
  result identities accepted by `tools/performance-harness`.
- The legacy lifecycle and default in-process Directory APIs listed above are absent from public
  compile fixtures and internal call sites.
