# Servient Runtime Lifecycle

## Ownership role

Servient is the application runtime and transaction coordinator. It owns
registration snapshots, plan sets, produced/consumed handles, route records,
in-flight operations, subscription facades, emission coordination, cleanup
records, and observable status. It does not own protocol syntax or I/O.

## Native Host Property Read replacement

For the admitted native Host Producer slice, the
[Binding ownership contract](../spec/binding-spi.md#host-producer-installation-checkpoint)
supersedes the staged Host builder/plan-set/route lifecycle described below.
`ServientBuilder::new(limits, shared_accounts).produce(prepared, authority, deadline)`
consumes completed whole-description preparation and the registered handler.
The returned typed `Servient<E, H>` owns one logical instance, publication,
current permits, dispatch, sealed response and retirement. `begin_expose` and
manual `step` reach Serving only after readiness. No TD, compiler, mandatory
logical plan or staged adapter survives into this owner. The former public
`produce_td`, `ExposedThingHandle` and post-build handler mutation are withdrawn.
The public Consumer runtime remains separately constructible and shares the
same Host account authority.

A typed single-instance root is the currently executable composition, not a
universal Servient API or infrastructure cardinality constraint. Multiple roots
may share capacity; a Binding installation may lease shared infrastructure.
The first HTTP adapter chooses an exclusive listener. Broader registration sets,
protocols, handlers and automatic scheduling remain product goals. Historical
static/Consumer architecture evidence retains its exact scope; the removed Host
staged tests do not transfer their prior acceptance to this successor.

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

<a id="proposed-instance-and-infrastructure-separation-review-08"></a>
## Instance and infrastructure separation

ADR-0024 adopts the distinction: Servient owns instance publication, current
logical authorization, generation-to-handler lookup, cancellation and drain.
Binding owns native matching/captures and infrastructure progress over installed
logical registrations. Each zero/many candidate remains generation pinned and
receives independent current acceptance; overlapping or identical filters do not
collapse recipients. Shared packets and per-recipient deliveries have separate
bounds. Deferred candidates cannot retarget; accepted work keeps original data.

One instance's close revokes its gate/participation without closing sibling
service. Final route-registration release initiates ordinary native retirement;
owner polling only progresses retirement already initiated. Replies/jobs and
observations retain the physical closer to final drain. Whole-infrastructure
shutdown needs separate revoke/drain authority. Semantic acknowledgement,
physical retirement and executor shutdown are distinct boundaries.

Host/static forms keep common semantic checks with different storage/progress
and explicit disposal policy. Existing narrow adapters retain their generation,
permit and last-root obligations. This extraction implements no shared native
retirement successor or reusable production backing. The current Servient still
composes Planning-owned output/reclaim/build types; their extraction is the
concrete prerequisite for a genuinely execution-only Servient feature.

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

The first Consumer Property Read transaction retains a caller-owned immutable
Thing loan through complete TD inspection/Basic and paid semantic lending. Planning
preflight precedes Servient runtime reservation, which precedes complete owned
materialization and all bounds before any compiler start. The terminal build owner
ends input/derived/config/compiler loans structurally before returning its complete
source-independent Frozen draft. The exact TD boundary is owned by the
[admission record](../work-packages/WP-100-consumer-validated-thing-admission.md).

Servient pairs local/parent build allowances, reserves slot/record/rollback capacity,
finishes bounded build cleanup and reconciles actual simultaneous owners before
final identity/generation/resource/registration/cancellation/slot checks. Caller
source is baseline; engine-owned upstream source stays charged while physically
live. TD frames/current derived bytes, owned plans/artifacts, compiler temporary,
registration/erasure/record and cleanup costs remain distinct, including inline
and transfer overlap. No mandatory Snapshot exists and no synthetic source release
or reclassification occurs.

Before the advanced Planning runtime-capacity handoff or cleanup-slot construction,
the coordinator validates the live transaction nonce, captured policy/registration,
generation and matching held parent allowance. A caller-constructible child
`AdmissionLedger` or equal reservation-owner tuple is not that authority. Planning
enforces local charges under its copied policy. The coordinator retains the actual
committed cleanup-slot account and paired parent until the destination and every
deposited/taken cleanup child are terminal; ending a reservation loan releases
neither charge. The [Planning handoff](../spec/planning.md#aggregate-construction)
owns local progress and guard-preserving constructor rejection.

A private permit closes cancellation under exclusive installation authority, then
performs the preallocated atomic install without allocation, callback, yield,
source query, fallible operation or retry. Failure before permit publishes neither
handle nor subset and retains cleanup until terminal child-before-parent release.
Published contains owned output/lookup, the separate complete registration and
runtime generation/lease/drain/cleanup/resource records. Legacy owning consume
requires a separate bounded source-destructor adapter before joining this facade;
Arc<Thing> retention cannot substitute for source-independent output.

Host and application-static forms share these semantics but not a physical
container or progress API. Host startup and every live consumed record retain
shared ownership of the one complete Consumer-capable registration (including
the client-only `HostConsumerBindingRegistration`); a call and any
transferred cleanup owner retain the matching plan-set lease. The application-
static form uses one caller-owned root containing the typed complete
registration, aggregate record, build/reclaim progress, and request slots;
short mutable borrows drive progress and no `Arc`, self-reference, interior-
mutable registration pin, or erased Host container is required.

Both forms resolve one eager artifact and call only the complete registration's
sealed `start_consumer_property_read` path. The runtime keeps Thing/property
names in API, plan, or diagnostics, never in
`OutboundRequest`. Close rejects new leases/calls, drains existing calls and
cleanup owners, and starts monotonic reclamation only after terminal ownership
is proved.

The admitted [Consumer Runtime tranche](../work-packages/WP-400-consumer-property-read-runtime-admission.md)
projects this lifecycle into one generation and one active/read-result opportunity.
Host parks the unique transferable output/rollback owner, checks it out outside
structural locks, and prepays its return bridge before lending local selections.
The manual root retains complete call, plan lease, original cleanup account and
parent capacity when caller interest disappears. Static retains the corresponding
typed root and associated-state slot directly. This projection adds no worker,
executor, call pool or lower-layer contract change; implementation acceptance
remains independent.

[ADR-0023](../ADRs/0023-resource-ownership-and-policy-boundaries.org) selects a
successor resource/disposal target. The current contract below remains active
for strict registrations; existing finite mock evidence is preserved.

The accepted [Consumer resource migration](../spec/consumer-resource-ownership.md)
and [successor readmission](../work-packages/WP-400-consumer-resource-ownership-admission.md)
define that admitted boundary. They preserve original-account lifetime and funded
engine fallback; the scoped SDK-local disposal refinement applies only to the
explicitly owned successor.

The tranche requires final-root abandonment to commit residual/incomplete
shutdown to a caller-retained fixed report before bounded local destruction.
It then uses the original Planning reclaimer and a single prepaid finite terminal
allowance, without protocol progress, allocation or work refill, releasing parent
capacity last. Supported local destructors must be audited and finite. This is
local resource disposition, not successful external cleanup. Leaking the advanced
manual Planning root is not an implementation of that Runtime obligation.

This obligation must also be checked against the concrete protocol infrastructure
behind a registration. Semantic call/generation reclamation does not certify
SDK session/task/callback retirement. The
[Binding SPI](../spec/binding-spi.md#shared-protocol-infrastructure-lifetime)
owns the original-account and final-owner contract. Foundation owns the
[Host failure scope](../spec/foundation.md#host-allocation-and-environment-failure-scope)
under ADR-0022. The application-provided reactor remains live through dependent
owner cleanup; stopping it is not a shutdown acknowledgement. A complete HTTP
connection owner is qualified in its WP-600 tranche. Deferred Zenoh findings
remain evidence for that future backend, not a shared Runtime prerequisite.
No asynchronous registration-retirement API is added; a concrete capture defect
must be demonstrated before correcting that boundary.
The admitted finite mock Runtime remains accepted; installing raw blocking SDK
destruction or unaccounted detached cleanup would exceed its author boundary.

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

## Host Producer admission subset

For the accepted [Host HTTP Producer admission](../work-packages/WP-600-http-producer-property-read.md),
Building privately resolves/compiles and checks complete advertised coverage.
The [endpoint contract](../spec/binding-spi.md#host-producer-endpoint-contract)
then supplies Installing -> LocallyReady -> Serving -> Closing -> Closed.
Publication is the existing Engine-only local linearization; Closing disables
new permits before native teardown. Its recorded scope replaces phase-specific
Host orchestration. The implemented source-free public path uses
`Reserved -> Installing -> InstalledClosed`, then publication and the
`Serving`/`Dispatching`/`Delivering` substates through Closing and Closed.
The typed root currently owns one registration and synchronous Property Read
handler; this is not the eventual multi-Thing/multi-Binding composition boundary.
It does not change static representation or silently broaden earlier gate
acceptance. The tranche owns production completion evidence and its independent
acceptance separately from the earlier mock gate.
