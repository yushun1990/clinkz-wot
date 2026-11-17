# Host execution across independent Consumer and Producer capabilities

Status: MIGRATED

## Counterexample and boundary

After independent finite Producer capacity, complete qualification/publication,
Binding-native sharing and original outcome funding are constructible. Consumer
capacities already retain aggregate turns. `ComposedProducerExposure`, however,
erases each participant to publication metadata plus `Any`. An application must
recover its concrete inventory to dispatch finite ingress, reconcile native
failure, poll Action ingress, progress original continuations and close each
revoked invocation. Merely adding another example repeats that orchestration.
In particular, `progress_actions` requests cancellation after revocation but
keeps polling normal work; a job that requires `poll_close` cannot drain without
application handle enumeration. HTTP also rejects polls after local close even
while the logical owner must continue retiring its original Actions.

These are Host execution integration gaps. The existing independent semantic
authority, native matching, source-independent preparation and publication model
do not require replacement. Application decisions about publication, Action
query/cancel cadence, notification emission/response, outcome observation and
acknowledgement are legitimate application policy. A transport-neutral scheduler,
universal runtime or Engine router would solve a different, unproved problem.

## Selected increment

Replace storage-only `OwnedProducerParticipation` erasure with one owned value
that erases the original native participant together with its typed Servient
turn. Finite, Action and notification constructors select existing public
capability operations in Servient. A bounded custom turn remains available for
linked capabilities and qualification fixtures. It receives the logical owner
in Servient; no native SPI callback receives dispatch authority. No native job,
shared result registry, runtime-family inheritance tree or executor is added.

`ComposedProducerExposure::step` visits its exact retained set with independent
participant/lane credit, continues after short work/errors, reconciles native
evidence at the logical owner and preserves the first operational turn error.
Native Action polling cannot spend continuation/cleanup credit. An opt-in scoped
cleanup operation marks only this participant's revoked original invocations for
local close. It neither acknowledges outcomes nor sends remote CancelAction.
HTTP closed polling returns no new candidate and remains usable during drain.

### Action progress cost correction

At `8e3593cad2b75880b5f0c7e6e728d48c59c0931a`, the independent review's
four-slot probe and a repository HTTP regression both reproduce a stall: 100
aggregate turns with four Actions CleanupItems make zero continuation polls,
while direct `progress_actions` with the same allowance completes. The separate
`close_revoked_actions` scan consumes the entire allowance even with no revoked
entry; selection then cannot run. Calling this merely insufficient caller credit
would protect an unnecessary implementation cost rather than the product goal.

The correction replaces that helper with `progress_actions_closing_revoked`.
One prepaid scan both marks scoped revoked entries and selects the least-recently
progressed original invocation. It shares a private algorithm with manual
`progress_actions`, whose cancellation/late-completion policy remains unchanged.
The exclusive store loan preserves the selected index through the original
callback and terminal timestamp update, so neither a handle lookup nor a second
completion lookup is necessary. Existing direct handle steps keep their own
lookup costs. Actual callbacks and original validation/cleanup remain paid;
Pending/error neither refunds their work nor releases their owners.

Raising caller credit makes the old sequence execute but retains the unnecessary
scan. Refunding the first scan after discovering no revoked work would misaccount
work already done. A cached revoked count/dirty flag would need to track passive
currentness changes, adding state to avoid work already covered by selection.
A per-participant index or resumable/global scheduler is unnecessary for this
counterexample; the conservative configured-store scan bound remains. Marking
only the selected revoked entry could also work, but merging marking into the
same traversal preserves the original all-matching close-intent policy without
extra scans or lifecycle state.

The [runtime owner](../docs/spec/runtime-safety.md#host-producer-execution-turns)
now specifies the single-scan/callback boundary. [HTTP evidence](../docs/evidence/host-execution-integration.md#action-progress-under-tight-credit)
checks the original 100-turn counterexample, exact and short budgets, current and
revoked siblings, Pending/error cleanup, manual late completion and described
CancelAction. No new resource, scheduler, Runtime or native protocol change is
required; current gates and roadmap milestones are unaffected.

Publication and final retirement remain explicit. The existing exact complete-set
retirement check still includes unread finite results, Action owners/results and
notification publication obligations. No idle/Ready value is promoted to proof
of complete physical release. Native installation remains a concrete Binding
operation; notification emissions retain their own explicit work owner/step.
Consumer turns compose alongside these Producer turns in the supplied Host loop.

## Alternatives

- Keep storage-only erasure and publish downcast loops in each application.
  This retains internal progression and revoked Action cleanup duplication.
- Put a semantic `step(owner)` on the native Binding trait. That reverses the
  corrected Engine/Binding authority boundary and couples Binding crates back
  to Servient. Retain the operation in Servient's existing storage boundary.
- Introduce an all-role runtime, Engine scheduler or shared protocol router.
  Existing capacities, independent publications and Binding leases already own
  their resources; the concrete deficit is retaining execution with erasure.
- Auto-publish, reap results or discard failed work in the turn. This would
  obscure application policy, result ownership or truthful retirement. Keep
  those handoffs explicit and retain errors/Pending cleanup.
- Share one allowance across native Action ingress and original work. New
  ingress could spend the credit required to release existing work. Supply
  independent lane budgets without claiming general scheduling fairness.

## Completion boundary and remaining product gaps

Use actual Engine Consumer-to-Producer HTTP plus Mosquitto, independently
published Things, actual HTTP definition reuse/shared listener/shared finite
capacity, continuing Actions and notification delivery. Show independent policy,
zero credit, unread and caller-held results, replacement, Pending/error local
cleanup, explicit acknowledgement and final socket/session/account release.
Preserve prior qualification, native authority and capacity regressions.

This does not establish mixed security, verified notification provenance,
cross-endpoint Action/notification compilation reuse, shared continuing Producer
work capacity, automatic timers/result wakes, Discovery-to-execution ergonomics,
constrained continuing execution or production/performance/v1 qualification.
The detailed runtime, native SPI, ownership projection, WP-400/WP-700 and roadmap
are updated at their existing owners; [scoped evidence](../docs/evidence/host-execution-integration.md)
records the executed boundary. No new ADR, gate, dependency edge,
review initiation or milestone closure is warranted for this bounded correction.
