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

## Internal owner and dependency graph

One Servient facade coordinates transactions, but broad source is partitioned
among private owners for the immutable registration snapshot, plan-set
arena/leases, per-Thing lifecycle generation, per-route driver, per-operation
call/handler state, subscription, emission, cleanup, reclamation,
per-binding resource accounts, and status projection. A monolithic
`ServientInner` that lets unrelated transitions mutate one shared record does
not satisfy this boundary merely because its maps are sharded.

Dependency direction is one-way:

1. immutable registration/profile snapshots and plan arenas are inputs;
2. Thing lifecycle owners retain generation-bearing leases into them;
3. route and operation owners retain only their exact leases and reservations;
4. cleanup owners receive complete objects through acknowledged transfer;
5. reclamation consumes only terminal lease/capacity facts; and
6. status observes committed events and never becomes lifecycle authority.

No callback runs while two mutable shards are held. Cross-shard work uses a
generation-bearing command, complete-object envelope, reservation, and
acknowledgement; rejection returns the complete input and commit revalidates
every generation. A bounded global or parent resource account may reserve
allowances for local shards, but it cannot be a mandatory process-wide
interaction-path lock.

`clinkz-wot-servient` owns one profile-neutral transition kernel and one
versioned machine-readable trace oracle for every Host/constrained capability
claimed in both profiles. Host synchronization and static caller-owned records
adapt that kernel; they do not independently compute generation validity,
terminal class, cleanup ownership, retry classification, or semantic resource
deltas.

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

Commit returns a distinct committed-closed route guard. The registry becomes
serving only after every required route has reached that state. One
generation-checked Servient transition publishes the immutable plan set and
produced registry generation and makes their shared immutable serving
activation authority selectable inside the private serving-activation record.
No binding callback runs at that boundary, and there is no undocumented
post-publication advertise phase.

Every `poll_accept` requires the private record to validate current serving
state, move the route's unique accept lease into one claimed-call owner, and
consume that claim into a non-cloneable route-scoped permit. The permit
exclusively borrows the lease through the callback. A registration that cannot
prevent request admission without the permit is rejected in v1. A binding
neither opens its own gate nor observes registry state.

Atomic publication and the v1 all-advertised-route rule are separate
contracts. Atomicity preserves one truthful immutable generation; the
all-route rule is the conservative v1 policy that every route in that
generation is required. A route failure therefore returns a structured failed
expose and completes rollback/cleanup. The engine does not delete a form,
publish a degraded generation, republish a TD, or retry exposure. The
application or deployment platform may construct and validate a different
effective TD and start a new generation after conflicting endpoint and cleanup
ownership is settled. Signing, Directory publication/withdrawal, backoff, and
restoration of the full route set remain explicit application/platform
responsibilities.

Destroy stops new permit issuance and marks the registry draining before route
shutdown. No new accept claim is admitted after that transition. A poll claimed
before drain retains its route and plan leases; in-flight handlers may finish
only within the selected bounded drain policy.

## Consumed Thing lifecycle

```text
BuildingPlans -> Published -> Draining -> Reclaimed
```

Each interaction, call, or subscription pins the consumed plan generation. A
handle drop prevents new selection, cancels or transfers outstanding operations,
and releases the plan set only after every lease and cleanup owner is terminal.

The first v5.1 Consumer Property Read runtime contains one validated retained
Thing, one sealed all-readable aggregate, and one complete Consumer-capable
Property Read registration. Planning preflight precedes Servient's conservative
persistent-capacity reservation; that reservation precedes Planning
materialization and the all-bounds-before-start barrier. Servient alone owns
the final cancellation/seal check and the `BuildingPlans -> Published`
linearization. Failure before it releases all unpublished reservations and
returns no handle or partial lookup.

Host and application-static forms share these semantics but not a physical
container or progress API. Host startup and every live consumed record retain
shared ownership of the one complete `HostBindingRegistration`; a call and any
transferred cleanup owner retain the matching plan-set lease. The application-
static form uses one caller-owned root containing the typed complete
registration, aggregate record, build/reclaim progress, and request slots;
short mutable borrows drive progress and no `Arc`, self-reference, interior-
mutable registration pin, or erased Host container is required.

Both forms resolve one eager artifact and call only the complete registration's
sealed `start_consumer_property_read` path. The runtime keeps Thing/property
names in API, retained source, plan, or diagnostics, never in
`OutboundRequest`. Close rejects new leases/calls, drains existing calls and
cleanup owners, and starts monotonic reclamation only after terminal ownership
is proved.

## Scheduling and fairness

Servient schedules ready work through maintained queues/cursors. A work step
does not discover readiness by scanning all records. Host policy may use
bounded per-binding lanes; constrained policy uses retained round-robin cursors
and explicit `WorkBudget`.

`WorkBudget` is the common linear accounting model, not a mandate for one
universal scheduler or queue. Route readiness/acceptance, operation and
delivery progress, subscription/emission progress, cleanup/deadline work, and
reclamation/status work have independently retained cursors or ready queues
and bounded inter-domain arbitration. Cleanup deadlines and older retained
owners cannot be starved by hot foreground work.

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

For inbound acceptance, the claim phase also verifies the private serving
record, immutable authority identity, route generation, committed guard, and
unique accept lease. The callback phase receives only the resulting scoped
route permit and immutable input. It receives no registry view, handler
dispatcher, or capability that can be retained after the call.

## Cleanup ownership

Before any side effect, Servient reserves the maximum number and bytes of
cleanup obligations that the operation can create. Independent obligations use
independent reservations; for example readiness cancellation and prepared-route
abort cannot compete for one item.

The operation's machine-readable coexistence matrix determines that maximum.
Obligations that can coexist reserve independently; mutually exclusive phases
are not multiplied merely because each phase has a distinct name. Reusing
capacity across exclusive phases requires an ownership-preserving reservation
transfer with no interval in which both phases can claim it.

Cleanup progress is explicit and budgeted. A transfer moves the complete owned
call, guard, driver, or slot into a named Servient/static-runtime owner. A
`CleanupRecord` alone is not the work object. Deadline exhaustion records a
bounded residual status before the object is destroyed outside locks.

In v1 that residual is durable only for the configured lifetime of the owning
Servient and its explicit final shutdown report. The engine promises neither
process-restart persistence nor automatic external compensation. Pending
status identifies the owner class, progress mode, phase, deadline/age class,
and blocked reclamation/shutdown consequence without exposing protocol-private
state.

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
