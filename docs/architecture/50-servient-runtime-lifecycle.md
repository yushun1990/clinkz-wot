# Servient Runtime Lifecycle

## Ownership role

Servient is the application runtime and transaction coordinator. It owns
registration snapshots, plan sets, produced/consumed handles, route records,
in-flight operations, subscription facades, emission coordination, cleanup
records, and observable status. It does not own protocol syntax or I/O.

## Construction

`ServientBuilder` accepts complete registrations and explicit resource,
security, codec, discovery-client, clock, and runtime-policy configuration.
`build` validates the entire set and freezes it. V1 has no runtime binding
add/remove/replace API.

Bare protocol trait objects are not sufficient server registration because the
builder cannot safely invent preparation visibility, capabilities, compiler,
resource footprint, ingress policy, readiness, or cleanup behavior. Binding
crates return complete registrations.

## Produced Thing lifecycle

```text
Draft -> BuildingPlans -> FrozenPlans -> PreparingRoutes
      -> ReadyRoutes -> Activating -> Committing -> Serving
      -> Draining -> Cleaning -> Destroyed
```

Every fallible binding callback has a unique lease retained by the route record.
Cancellation or handle drop records one immutable cause but does not destroy
inputs still held by a running callback. Late results remain owned:

- a late prepared guard is aborted;
- a late active guard is shut down;
- a late commit result joins shutdown; and
- a callback error returns or preserves every guard needed for cleanup.

The registry becomes serving only after all routes are committed and their
activation gates can enforce the same transition. A registration that cannot
enforce the gate is rejected in v1; there is no undocumented post-publication
advertise phase.

Destroy marks the registry draining before route shutdown. No new request is
admitted after that transition. In-flight handlers may finish only within the
selected bounded drain policy.

## Consumed Thing lifecycle

```text
BuildingPlans -> Published -> Draining -> Reclaimed
```

Each interaction, call, or subscription pins the consumed plan generation. A
handle drop prevents new selection, cancels or transfers outstanding operations,
and releases the plan set only after every lease and cleanup owner is terminal.

## Scheduling and fairness

Servient schedules ready work through maintained queues/cursors. A work step
does not discover readiness by scanning all records. Host policy may use
bounded per-binding lanes; constrained policy uses retained round-robin cursors
and explicit `WorkBudget`.

Route readiness polls all ready tokens fairly under one overall expose deadline.
One slow token cannot prevent other tokens from progressing or being cancelled.
Emission, subscription, response, and cleanup progress use the same isolation
principle.

## Lock and callback boundary

The runtime follows a two-phase rule:

1. under the appropriate lock/critical section, validate generation, reserve
   capacity, capture immutable input, and claim a callback lease;
2. release the guard, call user/provider/codec/binding code, then reacquire and
   commit only if the lease and generation still match.

No callback receives a reference into mutable registry storage. Reentrant calls
observe a well-defined public state and cannot deadlock on a lock retained by
the original callback.

## Cleanup ownership

Before any side effect, Servient reserves the maximum number and bytes of
cleanup obligations that the operation can create. Independent obligations use
independent reservations; for example readiness cancellation and prepared-route
abort cannot compete for one item.

Cleanup progress is explicit and budgeted. A transfer moves the complete owned
call, guard, driver, or slot into a named Servient/static-runtime owner. A
`CleanupRecord` alone is not the work object. Deadline exhaustion records a
bounded residual status before the object is destroyed outside locks.

Child-handle drop never blocks; it transfers into a pre-reserved runtime owner.
The root Servient provides explicit shutdown and a final report. Dropping the
root without shutdown cannot be documented as successful external cleanup.

## Status and observability

Operational and terminal status is generation-bearing, bounded, and queryable
while its configured retention is live. Queue overflow cannot recursively
enqueue into the same queue. Critical terminal facts update fixed-capacity or
overwrite-in-place status before an event copy may be dropped.

Bindings report through returned SPI events and settlements. Logs are
diagnostic output, not lifecycle state.

## Performance invariants

- Hot interactions do not scan TD documents or all bindings.
- Maximum-plan compilation is budgeted and resumable.
- One slow binding does not indefinitely block unrelated bindings.
- Long-lived binding objects and ingress buffers are admitted before side
  effects.
- Payload storage uses leases where possible and is not copied once per target.
- Cleanup, cancellation, and reclamation terminate within configured work/time
  budgets or produce explicit residual status.

Exact APIs, states, limits, and workloads are closed in domain specifications
and machine-readable artifacts before implementation resumes.
