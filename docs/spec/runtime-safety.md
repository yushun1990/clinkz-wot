# Runtime safety — v6.0

`SAFE-AUTH-001`: Servient MUST own logical instances, current permission,
publication and application dispatch independently of descriptions, prepared
content and native physical owners. Each acceptance independently checks the
current recipient/operation/policy and reserves work/result/cleanup. Replacing or
revoking a slot stops new acceptance and cannot redirect accepted work to a
successor. Native matching/transport control is not semantic permission.
Participation in several Binding runtimes MUST retain one logical authority and
behavior owner. Currentness cannot be copied into per-Binding registries. A
retention reference for original accepted work grants neither fresh acceptance
nor native shutdown authority.

The Host finite Consumer's `ConsumerOwner` exclusively owns registered
instances and their named configurations. `register_thing` constructs the unique
Core authority; `set_configuration` consumes a move-only
`StandaloneConsumerAssociation`. Returned `ConsumerInstanceHandle` identities
and cloneable `ConsumerAssociation` views MUST NOT revoke or unregister on Drop,
and MUST NOT expose the parent revoker or consumed association owner. A view issued before
adoption remains non-revoking afterwards. Explicit owner APIs control removal,
replacement and shutdown; registration does not create execution capacity.

The finite owner's public management path addresses Things by their TD `id`
within that owner, and configurations by an explicit name within that Thing.
`servient::consumer::register_thing` rejects a missing TD ID; anonymous TDs remain
valid for preparation and low-level standalone composition. The source-free
`ConsumerOwner::register_thing` accepts the already-qualified URI string, with
exact UTF-8 equality and a 2048-byte local bound. Duplicate live IDs refuse rather
than replacing or merging. These are local registration rules, not global TD
uniqueness or URI normalization rules. Identity registration installs no mappings
or readiness promise; explicitly install prepared configurations before use.

`instance` and `association` recover current non-revoking snapshots after all
returned views are lost. ID-based Read and targeted removal need no retained
management handles or external handle table. `remove_thing` targets the current
registration at lookup; it cannot infer an external delayed command's intended
revision. Original-snapshot remove/replace APIs supply that conditional behavior.
Public ID lookup grants no permission: admission still checks the selected actual
authority, operation, policy and capacity. Accepted work never repeats ID lookup.
The holder of the logical owner has management capability; an application exposing
it to untrusted clients must authenticate commands and govern configuration
selection. A configuration name is a selector, not a credential or caller identity.

`set_configuration` explicitly installs/replaces one named association under the
same original parent. Configuration names are nonempty, at most 256 bytes, and
are exact local selectors, not additional Thing identities. Refusal returns the
unadopted original association without changing installed policy. Selection has
no implicit default, permission union or fallback to a sibling configuration.
Update fences the old association; accepted work retains its original mapping,
configuration policy, operands and accounts. A stale configuration prepared for
a removed parent cannot be installed into a new registration with the same ID.
`remove_configuration` affects only the selected current association. Existing
finite operations can use transient `association` lookup; this does not migrate
continuing Action or notification lifetimes into this finite owner.

One authority is retained across independently composed facets. Continuing
Action, notification and policy services can compose with the managed instance's
passive `ConsumerInstanceRef` after registration. Their storage/progress APIs
remain separate. `ConsumerInstance` and `StandaloneConsumerAssociation` remain
valid low-level standalone representations, including caller-selected coordinates
for static/caller-owned resource domains. Such coordinates do not enter the
managed registry: there is no managed raw-authority/unnamed-instance adoption API.
Composition requiring managed lifecycle starts with managed registration and
shares its passive reference. An already-live standalone instance is not silently
promoted or retargeted to a managed authority. Dropping a standalone facet still
never revokes its parent or siblings.

Owner removal linearizes at the original authority fence before registry detach
or linked destruction. Budgeted removal/replacement may refuse before mutation;
successful removal fences immediately. Removing an association fences only that facet; removing
or replacing a parent fences all its facets. Instance replacement requires the
the owner's retained slot ticket and checked next generation, without accepting
caller coordinates; registration membership also checks the original issuer, so
equal coordinates across owners or fresh
registrations cannot redirect a stale handle. Association replacement uses a
fresh authority; old views never resolve through a replacement's registry slot.
Owner shutdown fences every parent before any mapping/service destructor runs.
Registry borrows end before linked callbacks, allocation or destruction.
Removal completion means removal from acceptance, not native drain.

Association views retain funded tombstone metadata. Closing their actual owner
releases unselected mappings outside borrows; accepted calls retain only their
selected definition, original authority, policy, operands and accounts. Calls
recheck original currentness and captured dependencies after permission, codec,
credential and native callbacks before effects or successful local handoff.
A recorded first cause survives later cancellation; already accepted remote
effects remain uncertain. Preparation's destination check alone is insufficient.

Finite association construction consumes `ConsumerBinding` records and refuses
a bank different from those records' bank. Shared content, qualified mappings
and retained services keep their original backing charges. Binding supplies no
source/compiler callback or execution/configuration override. Actual native
concurrency remains enforced by its original registration. Dropping preparation
or removing a sibling cannot reclaim backing still needed by another mapping or
normal/late settlement. `ConsumerCalls` supplies bounded work capacity separately:
one owner may serve many associations, and one association may use several work
owners. Dedicated or on-demand capacity is valid; no pool, global registry,
per-instance Runtime or executor is required.

Admission reserves an original work slot before linked callbacks. Its call
retains the original association authority, selected funded binding, resolved
destination, lexical variables, input/selection, deadlines and reservations.
Native retirement cannot erase those operands or retarget them to a successor.
Settlement lends original input/result slices and destination/variables; no
cloneable payload owner escapes. [Foundation](foundation.md) owns account scope.

Logical removal and actual logical-owner Drop are passive fences. They do not
signal an OS descriptor, run a scheduler, call a Rust Waker or perform native
cancellation. The application MUST arrange funded `ConsumerCall::poll` or
`ConsumerCalls::step` turns after lifecycle changes and abandonment, independent
of native I/O wakes. A lifecycle-command loop, cyclic executive or actual timer
may supply those turns. A logical `Deadline` is only an instant/comparison;
expiry is observed on supplied clock observations, not by an installed timer.
Native completion does not cover a silent peer or indefinite subscription.
Aggregate progress also requires the application to arrange result-consumer turns
if native wake interest has moved to another task. The optional
[Host turn composition](#host-consumer-execution-turns) observes application results
in the same driving task. Zero/partial credit retains runnable/progress
responsibility; no autonomous eventual cleanup is claimed.

Explicit call cancellation and work-owner close wake registered progress outside
borrows. Work-owner close fences only its admitted calls and preserves `step`
access. Work-owner Drop records causes without invoking `Waker::wake`; it also
vacates abandoned ready slots and destroys their complete original work under
the qualified finite local-abort contract. A checked-out call remains owned by
its lease; restoration locally disposes it if controller and handle are gone.
Later abandonment after controller loss has the same effect. An unrelated live
or settled B handle cannot be the sole reason abandoned A remains retained.
Surviving handles retain their own cleanup; returned settlements keep original
identity, normal/late evidence and accounts through disposal. Native/result
backing retires before original reservations. Supported unwind retains remaining
sweep/lease duties; arbitrary panicking/blocking providers are not qualified by
metadata. This Host finite subset requires complete bounded nonblocking local
abort, including native/service/credential/Waker destruction. Asynchronously
call-owned retirement needs a separately implemented acknowledged complete-owner
successor; a detached task/status token is insufficient. Local abort is neither
remote cancellation acknowledgement nor rollback or network drain.

Thing-root ReadAllProperties/ReadMultipleProperties/WriteAllProperties/WriteMultipleProperties retain their own root
coordinate, actual registration, combined schema and services under the existing finite ownership and settlement
rules. Its bounded schema/name backing and destruction stay funded through final
release; normal and late data use the captured member validator. Producer
publication requires a root behavior and actual native collection mapping as well
as full support of every final retained Form. Cancellation, replacement and local
close cannot transfer that interaction to individual Property behaviors or a
successor Thing. Missing/invalid members confer no partial success; native and
application aggregation provide no implicit atomic snapshot. Collection validation
work is prepaid in full, with service-bound changes fencing further dispatch.
Selected reads retain the original per-call bitmap into the immutable schema;
neither replacement nor another call can change it. The captured data policy
(`encoded_selection_bytes`) separately declares complete encoded-selection
payload backing, including media/Arc allocations; zero declines that facet. The request
array is encoded only after lifetime, scratch and cleanup reservations and work prepayment. Refused
encoded input is destroyed before its original funding releases, including native
admission refusal. Service loss during encoding fences the next callback and
native start. Authorization-time data-service changes are rechecked before
encoding, generic input validation, final native admission and result handoff.
The shared finite check inspects captured configuration, service availability,
data/access work policy and required facets both before and after authorization;
pre-callback observations cannot authorize the next boundary. The same check
applies after result validation and when handing off an already validated unread
result. Fencing retains the original result, authority, first cause and funding.
Every public finite Consumer result path, including rejected and late native
output, lends Core `InteractionOutputView`/`PayloadView` slices. No output, Payload
or body Arc may escape the original reservation through these views. Native
cancellation still exposes its original retry and borrowed cleanup disposition;
it grants no transfer ownership. Retain the settlement (including a shared Rc
owner) to share original bytes. Caller-owned explicit copies have distinct
backing. Call/root disposal or a local error cannot refund the retained original
response and cleanup reservations before final settlement disposal.
Loss during validation fences successful application handoff. The
Producer validates output against its original retained input, including late
results across authority/service loss, using the same captured validator rules.

Both root writes validate their complete invocation input map and
fresh group permission before native admission and before Producer application
effects. No member is applied by Engine validation or individual behavior fan-out.
A behavior error, invalid success shape or changed validator bound after entry
keeps possible partial effects. Completed group evidence is independent of
native delivery and local handoff, as defined by [Core outcomes](interaction-core.md).
Consumer uncertainty never triggers replay. Cancellation, replacement, service
loss and late results keep the original root input, behavior, authority, captured
services, first cause and all cleanup owners. A validated unread completion
retains its evidence while fresh policy can still refuse local success.
Selected writes derive the original set from the captured validator, not a
second caller-supplied list. Set-aware Consumer permission uses that retained
set before effects, after native/codec callbacks and on unread handoff; changes
cannot grant permission through another call's set. Optional member reports are
checked against the original set even after cancellation/replacement. Validated
incomplete reports retain their distinct Reported effect and native evidence;
local revocation does not turn them into Unknown or whole-group completion.
Invalid reports remain separately observable as rejected output. Inline report
backing is included in existing size-based call/output funding; native owners,
results and schema leases remain charged until actual disposal. No logical
cancellation or terminal receipt proves physical release or remote rollback.

Property Write uses the same finite cancellation, replacement and settlement
owners. Input validation and fresh application authorization precede handler
effects; a lost required service fences dispatch. A completed native reply cannot
override a retained local cancellation, timeout, revoked original instance or
policy/validation failure. Already accepted Producer writes retain their original
behavior/result across replacement, while pending acceptance cannot target the
successor. Retained Consumer settlements keep their original call/result/service
funding until final disposal. Consumer Engine call slots remain occupied until
settlement handoff or funded abandoned-call disposal; reusing a slot cannot refund
a separately retained settlement. The finite HTTP Consumer's native exchange
lease follows [Binding SPI](binding-spi.md#native-thing-root-property-collection-reads):
connection/driver retirement releases that lease independently of retained
settlements, whose later destruction cannot release a successor's occupancy.
The finite Producer retires its native exchange before settlement acknowledgement.
Its default dedicated opportunity stops receives until the result is taken;
optional independent finite capacity separates that policy as specified below.
Validated Property Write completion is retained independently of that local
cause and native delivery. Pre-dispatch refusal remains NotInvoked; errors after
behavior entry without valid completion remain Unknown. Finite authenticated
acceptance/delivery rechecks application and verifier revisions after all linked
callbacks as well as original authority/publication and captured services.
Policy callbacks cannot authorize a stale decision through a later verifier.
Finite Producer dispatch borrows the logical owner shared and the selected
runtime exclusively. A callback may progress a different runtime without a
registry lock or exclusive owner loan; recursively borrowing the same native
runtime remains unavailable. `ProducerRevocation` captures only one-way control
of the original incarnation's permission cell and its original funding. It
performs no lookup, native work, behavior callback or replacement, and cannot
revoke a successor in a reused slot. Its weak cell cannot preserve liveness;
the retained capacity lease lasts through the last control handle so late weak
storage does not outlive its accounting. Passive instance/publication revocation
inside callbacks is observed at the final acceptance/delivery boundary.

### Finite Producer participation and work capacity

`RawFiniteRuntime` remains a persistent typed native participant, with at most
one active exclusive exchange on that driver. Native configuration, closed
readiness, complete final-TD qualification, publication membership and native
retirement remain independent of execution capacity. A caller may select the
existing dedicated single-opportunity policy or attach `ProducerFiniteCapacity`
through `with_capacity` before native setup and managed admission. Attachment
requires the same actual logical owner and sufficient bounds for every field
of `RawFiniteLimits`; rejection returns the complete original participant.
Publication/readiness MUST NOT require a vacant shared work opportunity.

The optional capacity has separately bounded active and retained obligations
(1 through 32 active, active through 32 retained) at one explicit original work
bank. Retained counts active work, unread outcomes and acknowledged outcomes
still held by callers. Before each native receive callback it reserves both
credits and complete Engine input, result and cleanup backing. Saturation or
account refusal leaves publication current and runs no native receive/handler.
It MUST NOT consume `BindingPolls`: that credit remains available to a sibling
whose native progress can release the occupied capacity. Zero native credit
MUST NOT acquire an opportunity. Each actual native callback is charged just
before invocation, after its capacity and other work prerequisites succeed.
Pending parsing remains bounded by the Binding's existing native owner/policy
and returns the temporary Engine credits; it does not reset the original native
deadline. A returned ingress keeps its permit through fresh logical admission,
dispatch/refusal, Pending/error send and terminal native retirement. Zero work
credit makes no acquisition or callback progress; passive fencing still applies.

Native terminal settlement returns active credit, without granting another
concurrent exchange on that same driver. A bounded participant-local FIFO retains
the original outcome, accepted behavior, publication funding and work lease.
`take_retained_settlement` transfers the oldest complete `ProducerFiniteSettlement`;
it does not refund retained credit or its original bank. Output and target views
lend slices/coordinates without exposing original allocation owners. Only final
disposal refunds retained-work/result/cleanup backing. A capacity-local checked
sequence distinguishes records, including requests with identical Binding,
recipient and operation; it confers no authority or native correlation right.
Acknowledgement cannot redirect an original reply or refund a successor's debit.
Before a delivery/cleanup poll that may create such a record, the runtime checks
that `QueueOperations` can cover its current participant-local unread count plus
one: one visit per existing FIFO link and one insertion. The count is maintained
without traversal and cannot change during the exclusive native poll. This cost
is at most the configured retained limit, not an unconditional global maximum.
Insufficient queue credit invokes no native callback and consumes no native or
cleanup credit. Pending/error native polls add no queue charge; their normal
poll and security work still applies. Terminal success charges the checked
queue cost once, before FIFO mutation.
Native terminal handoff therefore cannot strand an outcome for lack of queue
credit, and repeated Pending polls do not repeatedly pay for an absent insertion.

Closing one publication does not close the capacity or a sibling. Final logical
retirement requires native close and acknowledgement of all its unread records;
returned outcome owners may survive that retirement and retain original funding.
Dropping a participant locally disposes its original native driver/configuration
before releasing active work funding. Unread records are locally disposed within
the retained bound. The capacity owns no work/result registry or scheduler; loss
of a facade cannot strand work behind a separate capacity queue. Native socket
occupancy and final physical listener release remain Binding obligations.

The dedicated raw handoff and narrow `ProducerExposure` facade deliberately keep
their prior one-opportunity policy. Continuing Producer Actions/notifications
retain their own capacity and acknowledgement contracts. The shared finite
increment adds no general Producer Host loop, automatic wakes, timer, fairness,
native concurrency or all-profile claim. [Executable scope](../evidence/producer-finite-capacity.md)
uses the existing real shared HTTP listener and qualified final descriptions.

### Continuing Action admission and handoff

Continuing Action admission reserves the maximum selected native exchange,
opaque continuation, input, response and validation scratch plus cleanup before
native acceptance. One invocation has at most one exchange; accepted continuation
identity survives exchange retirement. Pending/error cleanup keeps the complete
original owner and reservation, including through root loss and unread terminal
settlement. Finite local abort disposes the native owner before funding refunds.
An abandoned handle is retained for round-robin manual cleanup by its root; if
all local owners disappear, the captured native finite-abort contract applies.
Neither path claims that independent remote Action work stopped. Callback-time
authority/service checks fence late success; original native result, validation
failure and first local cause remain distinct. Handles cannot substitute mappings,
move to a successor incarnation or address a reused slot.

Native result readiness is separate from successful local handoff. Unread Action
completions remain subject to cancellation, original authority/root revocation,
deadline and captured service/policy loss. Settlement handoff takes current time
and work credit, pays fresh destination/access checks and rechecks captured
services after callbacks. Insufficient credit grants no success and keeps the
original slot and funding. Native poll errors and late retirement output retain
independent evidence; even after an error, captured validation applies to that
late output before final settlement, without clearing the first cause.

The [caller-owned Action representation](#caller-owned-continuing-action) also
validates each application result loan. Moving its generic output owner does
not prove that a later `ActionOutput::view` selects the previously checked value.

### Notification associations and conversation capacity

`ConsumerNotificationAssociation` retains the original Consumer reference,
qualified immutable definition, actual retained access owner and chosen instance
bank. Its passive views MUST NOT revoke on Drop. Construction checks the prepared
definition's original declarations, qualifies the supplied access owner and prepays
its record before retention. It captures access work/security revision at this
explicit binding boundary; a later capacity constructor cannot refresh them.
It allocates no conversation queue, native interest or vacant work slot.

`ConsumerNotifications` supplies a bounded raw conversation slab at an independently
chosen work bank. Subscribe acquires the original instance's outstanding-subscription
quota and the prepared data service's validation scratch before native preparation.
Each entry retains its exact Consumer, definition, policy, participant, queue,
response rights and original funding. A closed unacknowledged entry still occupies
its slot. Acknowledgement releases work scratch and subscription quota after local
retirement; retained reports keep original prepared/association owners for any
shared target backing until report disposal. Capacity Drop performs qualified
whole-owner finite local abort; it does not revoke persistent associations or
other capacity domains. Original instance revocation fences only its work.
Applications schedule progress/cleanup after passive revocation and cursor loss,
either individually or with the aggregate turn below. There is no scheduler or
automatic authority-change wake mechanism.

`RawNotificationServient::step` and `ConsumerNotifications::step` visit every
original conversation at most once per turn, rotating the first visit across
turns. A pending participant, retained error or short credit cannot skip sibling
visits. Each visit uses the existing conversation's policy, validation, queue,
response and retirement transitions. Zero credit can observe passive stop and
authority fences but cannot enter native cleanup. The turn invents no application
response, cancellation input, remote control or permission.

A live cursor retains queued values and its closed unread report. Cursor Drop
records abandonment separately from stop intent. Aggregate progress disposes an
abandoned cursor's report only after complete local retirement and a paid cleanup
visit; pending/failed response or participation retirement retains its original
owners and capacity. A surviving stop view or saved identity does not prevent
this disposal or revive the cursor. Applications needing reports after cursor
loss use the existing explicit `progress`/`step_cleanup`/`take_settlement` path
instead of automatic abandoned-report disposal. This is an opt-in consequence
of aggregate driving, matching abandonment of finite/Action handles; it does not
discard a live recipient's result or report. Neither acknowledgement nor idle
capacity proves release of independently retained result/definition/native owners.

Prepared data policy/media/native support MUST match at association binding,
admission and applicable delayed handoffs. Fresh availability/schema support and
access work/backing/revision checks surround policy/validation callbacks. Changed
declarations refuse/fence the original obligation without entering an unfunded
callback or accepting a new baseline. Cleanup retains its prepaid owners even
when those declarations are no longer usable. Explicit new qualification is
required to use changed dependencies.

Validation MUST preserve an observed dependency violation separately from ordinary
caller-input or short-work refusal. Both the scalar and collection validators
recheck captured dependencies after the codec callback, including when the codec
returns an error. Response admission and prepayment before native receive MUST
fence the original conversation on a dependency result; restoring a declaration
cannot revive it or permit a retry to send a response. An input/work refusal with
valid dependencies remains recoverable, but still requires a passive currentness
check before return so synchronous stop, instance revocation or pairing loss is
not hidden by the error. Dependency origin cannot be inferred from `CoreError` or
reconstructed by rereading declarations after the failure.

Response refusal returns the exact caller input without adopting it or consuming
the native reply opportunity. A fenced conversation retains its original occurrence,
reply job, first cause and prepaid cleanup owners through zero-credit or pending
retirement; cleanup requires neither codec credit nor renewed delivery permission.
Admission failure has no admitted conversation to fence. Cancellation has already
fenced local semantic use before its validation callback. Validation failure of an
owned native candidate remains fatal regardless of error origin. None of these
paths changes subscription authority versus individual occurrence refusal.

The validator borrows the original Property value/map or Event role trees,
prepays validation before native handoff and checks data before queue acceptance.
The original occurrence coordinate survives staging/queueing and is supplied to
fresh Deliver/Respond policy at acceptance, `front`, response admission and delayed
native response send. These decisions always carry an owned item. Separate Maintain
checks, without an occurrence, preserve the current subscription grant before
polling or semantic use. Fresh Maintain checks after occurrence policy ensure that
synchronous grant withdrawal overrides even a positive item refusal; both probes
are prepaid before acquiring a native candidate or entering the item callback. Check original recipient, scope, backing and native
response support before occurrence policy; the composed policy also checks
captured Event membership.

An explicit RejectOccurrence marks only the original staged/queued item as refused.
It cannot be exposed, advanced, responded to or resurrected by subsequent policy
changes. `front()` returns None for that refused head; `respond()` returns the exact
unaccepted caller input with AuthorizationDenied. Progress, or the existing paid
round-robin cleanup visit, retires the refused item without closing the native
participation. Each turn performs at most one native retirement poll or one item
disposal; it never scans a stream of refused items. Later queue entries remain
behind the original refused head until its retirement completes. Callers continue
manual progress after an empty view or pending retirement.

Refused payloads and submitted responses stay funded until their exact reply
owner retires and their own disposal step completes. Required response opportunities
can be retired on refusal without fabricating a send. Pending or short cleanup
credit retains all original owners/capacity; cleanup does not need renewed delivery
permission or codec credit. Native reply-retirement failure records its cleanup
error and fences the conversation, retaining that reply for the ordinary cleanup
path. Actual sent/uncertain response evidence remains distinct from refusal intent.
Authority/dependency errors, instance revocation, invalid native/schema contracts
and native failures also fence as before; an occurrence refusal itself sets no
subscription first cause, stop intent or native loss count. Invalid/late candidates
and first causes remain with the original conversation through cleanup. NoSec MQTT
supplies declared Event attribution, not publisher authentication; source-proof
verification, proof freshness and its retained trust dependencies remain unimplemented.

Native topic sharing, distribution, bounded queues and connection/control-frame
disposal remain in the Binding, independently of each recipient's authority,
cursor, loss and stop. An Engine slot grants no whole-service shutdown right.

Raw and composed notification `front()` lend only payload/media and occurrence
name slices under the existing exclusive root borrow. No public delivery loan
exposes cloneable owners of those original allocations. Active loans prevent
advance and cleanup mutation; queue, reply and original account ownership remain
with the existing root until the appropriate disposal boundary. Keeping an
explicit copy after advance does not retain the original allocation. This rule
applies to leaf Property/Event and root Property/Event notification delivery.

A collection observation retains one root authority and original named schema
through late candidates, stop and settlement. Complete-map validation accepts
all members together or fences the entire owned sample; no partially validated
member may escape. Cursor replacement never inherits buffered maps. Shared
native topic loss/overflow is visible per logical recipient, with whole-sample
DropNewest or termination and no implicit coalescing. Mapped root unobserve and
physical housekeeping remain separate: stopping a shared interest may require
no native unsubscribe, and local disposal alone cannot claim a remote ACK.

An Event collection retains that root authority separately from each occurrence's
exact Event target. Name backing joins data in the per-handoff/queue byte bound,
including the extra late/overflow slot. The named schema table and construction,
destruction and validation work have original funding; native queues have their
separate fixed Binding owner. Data and reciprocal responses validate the original
member role. A wrong identity/schema, revoked grant or late callback retains its
actual data/reply owner until successful retirement or bounded local abort; it
cannot move into a successor's queue. Group stop fences every occurrence without
revoking sibling logical subscriptions. Final native interest disposal and mapped
unsubscribe evidence remain separate from logical closure and unread settlement.


`SAFE-PUBLISH-001`: Producer setup MUST validate the draft/known dependencies,
capture actual providers/behavior and reserve setup/ingress/finalization/cleanup
before acquiring native resources. Owned closed native participation may determine
endpoints. Finalize the effective TD and revalidate affected common/Binding
support, collision and ownership facts for that exact revision. All retained
Forms/effective operations require complete qualified support and required
readiness under PREP-COVER-001. Final dependency/cancellation checks and publication
of final TD, logical registry and current authority share one Servient-local
linearization boundary with no new fallible native preparation, allocation or
handler call. Cancellation winning prevents publication; failure retains partial
setup and rollback owners. No prepublication input dispatches behavior. A complete
reduced TD can be deliberately requalified, never silently substituted. No global
network/remote cached-TD atomicity is claimed.

A later deterministic unsupported operation/codec/schema/security/behavior/config
branch already present at publication is a qualification/implementation defect,
not normal Unsupported. Fail closed for affected acceptance, preserve exact first
cause and cleanup, and requalify before claiming readiness. Changed endpoint
meaning/providers/support policy invalidates prior qualification; equivalent
reconnection is availability recovery. Per-use authorization/credential rotation
is not by itself new immutable definition content.

`SAFE-CANCEL-001`: Cancellation/timeout MUST fix one first local cause, stop new
local acceptance/delivery and request mapped protocol/application cancellation.
Already accepted work and late successes retain identity, result and release
owners. Local terminal settlement does not imply remote rollback, exactly-once
remote effect or physical shutdown. A constructor rejection proves no escaped
obligation; an operational error after effects cannot make that assertion merely
by its error type. Cancellation, request deadline and cleanup/drain deadline are
distinct phase contexts. Synchronous linked code is cooperative, not preemptible.

`SAFE-CLEANUP-001`: Every accepted/provisional obligation MUST have funded bounded
cleanup even under saturation. Ownership transfer commits only when a successor
acknowledges receipt of the complete live owner/backing; a request, status record,
spawned task or numeric token is not transfer. Rejection returns the same owner
to retained/manual cleanup. Capacity stays charged through quarantine/pending
cleanup. A residual report distinguishes external uncertainty and does not permit
freeing surviving local state. A Host root needs qualified finite local abort or
acknowledged transfer to a longer-lived supervisor retaining all obligations;
an executor that may disappear is insufficient. Physical retirement remains at
the actual infrastructure owner. No callback/guard can silently orphan work.

`SAFE-DROP-001`: Drop MAY revoke/request cancellation and move complete work into
prepaid cleanup storage; it MUST NOT wait for a network/user callback or report
completed drain. Explicit close/destroy reports drain/local release/remote residue.
Caller-driven profiles retain and drive cleanup. If reserved transfer capacity
is unavailable, fail closed before accepting new work; exhaustion cannot forget
a live guard. Fixed trivial release may be prepaid; variable destruction needs
bounded progress or a qualified owner.

`SAFE-CONCUR-001`: Acceptance, revocation, publication, completion and transfer
MUST have explicit linearization points under one logical ownership authority.
No handler, Binding/service callback, allocation or unbounded destruction occurs
under registry-wide locks or constrained critical sections. Pending progress
registers wake interest then rechecks state; terminal results survive until
acknowledged. Fairness/work budgets prevent unrelated supported logical work
from being indefinitely blocked by a stalled Binding. This mandates outcomes,
not a global lock, fixed lane/shard algorithm or identical scheduling traces.

The Host finite Consumer registry is borrowed only to reserve, check out,
restore or remove an original call slot. Admission/progress callbacks and linked
destructors run after that borrow ends. A checked-out or admitting slot remains
occupied; reentrant progress cannot poll it again or overwrite it through nested
admission. Passive per-call control retains the first cause and abandonment even
while native work is outside the registry. Explicit cancellation and work-owner close
wake registered progress outside registry/control borrows. Managed removal and
standalone authority loss are passive fences requiring application-scheduled
progress. Work-owner close fences all its admitted calls before notifying
any waiter. Callback-time close/cancellation is observed before another native
effect or successful handoff, including construction and validated unread output.
Removing a terminal call is its handoff linearization point. Native/codec late
evidence remains with that original call regardless of the local first cause.

Managed instance registration linearizes at unique-owner installation in the
logical registry. Named configuration adoption moves its unique owner into that
registry; only non-revoking views leave it. Lookup checks original issuer/authority rather
than names or numeric coordinates alone. Configuration refusal returns complete
unadopted input owners; failed identity construction publishes nothing, refunds
its staged backing and consumes no slot ticket. Bounded lookup/mutation is paid
before work; registered
entry retirement is prepaid before adoption. Removing or replacing an entry
fences it before dropping owners outside borrows. Accepted calls remain in their
original independent work stores, with no transfer inferred from registry detach.
The same rule applies to low-level standalone close; passive view loss changes
no logical permission. The common finite kernel has no observer registry or
native authority-change notification machinery.

The managed registry is a private AVL map by TD ID; each instance has an AVL
map by configuration name. Every node's cached height equals one plus its tallest
child and its child heights differ by at most one. Search/insert/erase therefore
visit logarithmic paths, including adversarial long-prefix IDs. Nodes are
individually funded and allocated before commit: there is no global shift,
rehash, arena growth or compaction pass. Rotations move complete nodes without
changing their keys, values or authorities. Two-child deletion extracts a
successor along one additional bounded spine and returns only the original
removed node, detached from both children. Linked payload destruction occurs
outside borrows. Accepted work retains original owners, never node addresses or
mutable registry positions.

A private funded ticket belongs either to one active registration or the owner's
free list. Registration pops a ticket in constant time or allocates one before
commit; failure leaves the list/counter unchanged. Removal fences/detaches first,
then advances the ticket's checked generation and puts it on the free list without
allocation. Generation exhaustion retires that ticket rather than wrapping.
Replacement retains its ticket and advances the generation internally. Original
Core references keep their immutable coordinate/generation and unique issuer;
ticket recycling cannot revive them. No numeric slot index or caller-slot search
is required. Free-list destruction is iterative to bound shutdown stack use.

Single-Thing management performs no work proportional to unrelated registered
Things. Disposing that Thing's own configurations/services still requires their
prepaid scope-local work; whole-owner shutdown necessarily visits every parent
before disposal and releases all retained ticket backing. Neither operation
claims native drain or automatic scheduling.

Runtime driver entry records its time observation on every original live or
admitting call before invoking callbacks; direct call polling records its own
entry observation. Resumed progress uses each call's latest accepted observation,
including later nested progress or admission, rather than replaying an outer
sweep's earlier timestamp. Genuine caller time regressions and clock-domain
changes still record an invariant failure. Deadline fences use that latest
observation after callbacks and before native effects or successful handoff.
Runtime drivers and direct call polls both retain wake interest when the original
slot is checked out. Busy entry requests a retry on restoration; waker registration
and the restoration wake occur outside registry borrows, including when an inline
cancellation wake reenters a pending call's driver.

Slot restoration on unwind retains the original owner; it grants no guarantee
that arbitrary panicking linked code can resume execution. The independent
control allocation is funded before native construction and keeps its original
lease through its last handle/call reference. It cannot address a reused slot.

### Continuing Action associations and invocation capacity

The Host `ConsumerActionAssociation` retains one original Consumer, shared
prepared definition, separately qualified per-operation access owners and
instance bank. Association construction allocates no vacant slot, exchange,
continuation or work quota. It rechecks prepared data/native declarations and
original backing before/after policy qualification. Explicit new access binding
captures its own work/revision; it never replaces prepared data/native facts.
Association construction and new invocation admission check the complete
selected Invoke/Query/Cancel set against its original declarations. Retaining
that set does not create a blanket invalidation domain for admitted work.

A live invocation checks its original Invoke native mapping/continuation and bound
access declarations, plus the operation owning its current exchange or unread
result. Data-service checks follow the application-value role still owned by
that exchange/result. After input validation and Invoke exchange retirement,
Invoke's codec is not a dependency of an independently represented Query result;
the original native continuation and Invoke authority remain live.
An unused Query/Cancel dependency cannot fence unrelated accepted work
or its handoff. A Query result retains its original Query dependencies until
handoff. Once a nonterminal status or cancellation acknowledgement has been
consumed and its exchange fully retired, that control no longer governs the
pending invocation. The original continuation and known cancellation status
remain retained. Shared actual services still invalidate every relevant use;
separate operation coordinates do not isolate an aliased dependency.

A control request checks its original native/access and used value-role
qualification, continuation compatibility and fresh destination/instance
permission before native start.
An unavailable or denied control refuses without native effects or withdrawal
of unrelated Invoke work. Every refusal still observes callback-time loss of
already relevant dependencies and logical authority. Fresh policy checks remain
scoped to the actual operation, including progress and handoff that depend on it;
Invoke permission never grants Query/Cancel permission. Consumer/root revocation,
time and local cancellation continue to govern the entire invocation.
Declaration restoration does not erase an already observed violation of relevant
admitted work. Independent invocations keep their own first causes.

The Action output schema is shared semantic meaning, not a mandate to reuse
Invoke's codec. Invoke validates input and any immediate output with its own
original data service; Query validates its completed output with Query's original
service, media and body/scratch/work policy, through normal and late retirement
paths. Native response backing is additionally bounded by that exchange's
original response declaration. Media must match the captured representation;
the current subset admits no content coding. Before any poll/close that may
return an application value, prepay that operation's captured validation work
once. Never add Invoke's allowance to Query's. Control start carries no
application input; Cancel progress invokes no application codec. Admission
reserves the maximum selected native scratch and used data-role scratch before
effects; an unused Cancel application codec requires no scratch. Check the
actual output service's original declaration and schema support before/after
validation even if authority was already lost or the validator refuses. A changed
declaration cannot authorize larger work or silently replace the original codec.
The original payload/media, validation error and earlier native/local/cleanup
failures remain separate evidence. Sharing semantic constraints transfers no
permission and permits no transcoding, new service adoption or funding refund.

`ConsumerActions` owns a bounded slot registry and its capacity bank independently
of any Consumer. Vacant slots contain only registry/control references; each
admitted invocation record is separately allocated and funded, then retains that
allocation through checkout and settlement. Slot storage does not charge that
record a second time. Admission reserves a slot, passive control, complete native/
continuation/input/result/scratch backing and cleanup credit before effects.
The capacity bank funds actual work backing. A distinct association bank debits
one outstanding-work quota (`PendingHandlerCallsGlobalMax`), including retained
settlements; an identical bank is debited once. This quota is not a second RAM
reservation. Unread results retain their slot. Settlement transfers evidence,
association, quota and backing to the result owner; the vacant slot alone cannot
refund a result's funding. Refused admission returns exact original input and
rolls back all newly reserved capacity without native effects.

Capacity close/Drop fences its admitted work, without revoking associations or
sibling capacities. Original Consumer revocation fences its own invocations in
any capacity. Loss of association views is passive. Handles retain original
capacity/control ownership for progress; settlements retain their original
association and work leases through final native/data/service disposal. Input,
normal and late output access lends slices tied to that settlement; it cannot
detach the original payload Arc from its funding.
Cleanup still uses the bounded Host local-abort contract; this adds no scheduler,
background task, native pool or general async cleanup transfer.

The Host continuing Action owner has its own occupied invocation slots and
separately funded passive controls. Registry borrows end before access,
validation, destination/start, poll/retirement and linked destruction callbacks.
The original invocation keeps its opaque token, selected mappings, exchange,
normal/late evidence and funding across checkout. Admission reserves its slot
and sequence before callbacks; nested admission cannot reuse or overwrite it.
Reentrant query/cancel, settlement and native-state inspection return
`Backpressure` while that invocation is checked out. Identity, first cause and
local cancellation remain accessible. Reentrant progress records time and wake
interest, returns `Backpressure`, and requests a retry after restoration; it
never polls the same exchange concurrently. Runtime drivers retain that interest
too. Time observations, explicit close/cancellation wakes, passive Drop and
unwind restoration follow the boundary rules above independently for this owner.

Action access, native and validation callbacks are followed by fresh original
control/dependency checks before another native effect or successful handoff.
After native control start, loss of that exchange's authorization/services
fences its admitted work irreversibly. A refusal before start leaves unrelated
accepted work usable; it cannot hide an observed original Invoke dependency or
logical authority violation. Short work or an unsupported control remains a
refusal without inventing a remote effect. A successful native start returned
after revocation is retained for local retirement, without polling it for further
effects.
Required cleanup and late validation continue under original funding; neither
can erase a first cause or promote uncertain effects to success/rollback.
Native completion evidence is stored before validator entry, including on
unwind. A terminal invocation is removed only at the settlement handoff (the
acceptance linearization point), or for abandoned local disposal outside the
registry. The control lease survives through its last original handle/owner;
settlement disposal releases the original native/result/service funding.
These are distinct Host ownership implementations, not a universal runtime or a
claim of broader constrained-profile or arbitrary panic recovery support.

`SAFE-PROFILE-001`: Supported Host and constrained interactions MUST have equivalent
WoT meaning, authority, late-result, loss, cancellation and cleanup guarantees.
Storage, erasure, async syntax, thread mobility, atomics, allocation and driving
mechanisms are independent choices; constrained execution MUST be usable without
mandatory pointer atomics/Arc. Profiles use common semantic kernels/invariant
cases where applicable and their own storage/wake/release evidence. No portable
compile, matching mock trace or std flag qualifies hardware, SDK, allocator or
real-time performance. Missing implemented profiles remain gaps, not exclusions.

Runtime-independence claims distinguish four guarantees. Compilation reuse is
owned by PREP-REUSE-001. Logical instance independence requires distinct current
authority and original obligations even when content/infrastructure is shared.
Executor independence permits deployment-supplied manual or async driving without
an Engine scheduler. Runtime/storage representation independence additionally
requires actual execution without imposing another profile's account bank,
reference counts, synchronization, work registry or native object layout. None
of the first three alone establishes the fourth. Shared semantic kernels do not
require identical native Binding artifacts in incompatible environments; each
representation must qualify its actual dependencies, controls, work and release.
Scoped executable support does not close the remaining profile/interaction gaps.

### Host Consumer execution turns

`consumer_execution::ConsumerProgress` is an optional borrowed progress interface
implemented by finite calls, continuing Actions and typed notification capacities.
`step_consumers` visits the supplied capacities with independently supplied work
budgets and one caller-qualified clock observation. It allocates no registry,
stores no work, owns no Consumer and acquires no native resources. The slice may
contain any subset or several capacities, including notifications from different
Bindings. Progress remains bounded/cooperative under each capacity's existing
contract. Per-capacity credit prevents one family from spending a sibling's
allowance; each capacity retains its existing within-capacity fairness. Errors,
short credit, normal/late results and cleanup remain with their original owners.
The return value reports idle capacities after the turn, including unread reports;
it is not a claim that external settlements or physical infrastructure are gone.
Idle inspection is passive and must not invoke linked callbacks or mutate work.

Managed composition starts with `ConsumerOwner` registration and shares its
original passive reference with independently authorized family associations.
The existing finite named-configuration registry stays finite; Action and
notification associations are explicitly bound, not looked up through a new
universal configuration registry. Removing/replacing a parent or dropping its
owner fences every original facet without retargeting work. Closing a capacity
still fences only that capacity. The progress helper accepts no new authority,
definition, service or configuration override and changes no admission rules.

A Host deployment MUST supply turns on native wakes and an independent maintenance
source covering passive lifecycle/stop changes, deadlines, replenished credit and
native Ready-to-next-phase transitions. An actual timer, command loop or cyclic
main loop can provide this source; a logical Deadline and native wake interest
alone cannot. In async integration, register that source before returning Pending
and keep application result observation/decisions in the same driving task.
Existing manual handles do not supply broadcast wake notification to independently
awaiting result tasks. The helper neither self-wakes on exhausted credit nor
claims progress after the deployment stops supplying turns. Explicit finite/Action
cancellation retains its existing registered wake behavior; passive owner loss
and notification stop remain passive. Native descriptors/timers/sessions stay in
their actual Binding/deployment, not in this interface.

Application QueryAction, CancelAction, response, advance and mapped stop remain
explicit decisions under fresh policy. Shutdown fences managed acceptance, keeps
the capacities progressing and consumes live results/reports until original local
retirement; abandoned work can then be reclaimed by its capacity. Dropping a
capacity still invokes only its qualified finite local abort. No background
executor, mandatory scheduler, per-Thing work owner or complete drain on Drop is
introduced. Manual per-handle poll/progress and capacity step remain usable alone.
The [mixed execution evidence](../evidence/consumer-execution-composition.md)
and [compiled Host example](../../protocol-bindings/protocols/mqtt/examples/mixed_consumer.rs)
establish the scoped Consumer integration. Producer integration has its separate
scope below; mixed security/source verification and constrained continuing
execution remain unqualified here.

### Host Producer execution turns

`producer_composition::OwnedProducerParticipation` retains a native member with
its typed Servient turn rather than erasing only publication metadata and `Any`.
The `finite`, `actions` and `notifications` constructors select existing native
capabilities and semantic operations in Servient. The bounded `new` extension
seam supports linked capabilities without a Core whitelist. Construction remains
pure; the original native owner, operation pointer, identity reference and error
storage are paid by the existing composed admission. No native callback receives
a Servient, handler or semantic dispatcher. Custom turns owe the same current
authority, captured-service, funding, outcome and cleanup obligations as direct
use; they cannot implicitly publish, emit, acknowledge, replay or discard work.

`ComposedProducerExposure::step` visits the retained complete member set, checking
the original participation identity, issuer and instance before any callback.
Typed loans cannot substitute a new participant into that turn. Each member and
`ProducerWork` lane receives independent deployment-supplied credit. Native Action
polling and accepted continuation/cleanup progress use separate allowances; both
are attempted even if native polling lacks credit or returns an error. A member's
short turn/error cannot skip its siblings. The turn returns an operational error
in preference to backpressure, retains each member's first returned non-backpressure
error and exposes it through `progress_failure`; absent such an error the accessor
lends that member's native failure evidence. The original native, application,
delivery and cleanup owners remain authoritative for their detailed outcomes.
Native evidence is reconciled by Servient against committed membership before
revocation; uncommitted or foreign evidence cannot revoke another publication.

Action turns use `progress_actions_closing_revoked`: one association check and
one prepaid original-store scan both mark only this member's revoked invocations
for local close and select its least-recently progressed entry. Manual
`progress_actions` uses the identical selection/progress algorithm without the
opt-in close marking; revocation still requests cooperative cancellation and
permits normal late completion. Ordinary described CancelAction cannot force
local close of current work. No current sibling is marked or progressed through
another participant's turn, and no remote CancelAction is synthesized.

For a configured Action-store bound `N`, either entry point consumes `N`
CleanupItems before the scan, including when selection is empty. Below that
allowance there is no marking, selection or application callback. The scan
retains its original selected index under the exclusive store loan through
progress and completion bookkeeping. It does not construct a handle and repeat
a lookup scan. Direct handle-based `step_raw_action` still pays its own lookup.
Work after selection pays the original callback envelope: normal work consumes
HandlerSteps for output body/media, the captured validator and poll, plus one
unit if cancellation must be requested; local close consumes one CleanupItems
unit for its poll plus one if cancellation must be requested. Unread completed
entries require no further callback allowance. Short callback credit may retain
paid selection/close intent but invokes no application code; it never refunds a
scan already performed. Least-recent selection remains participant-scoped,
including after Pending/error and with unread results; its existing progress
counter remains checked.

The same Actions allowance funds selection, optional close marking and actual
callbacks. There is no separate unconditional close scan. For example, with
`N=4`, normal work needs four cleanup units, first cancel/close needs six, and
later close polls need five, plus any normal HandlerSteps envelope. Pending/error
close retains original work, result funding, cancellation failure and first close
failure; no result is acknowledged by a turn. Native closed Action polling remains
valid maintenance and yields no new ingress rather than an unsupported-operation
error. Conservative global-store selection bounds, deployment scheduling and
partitioning of credit between participants remain separate from this correction.

These Producer turns compose alongside [Consumer turns](#host-consumer-execution-turns)
in a deployment-supplied Host loop with the same native/maintenance, clock and
result-observation obligations. Concrete native installation, complete final-TD
publication, semantic query/cancel/response decisions, each notification emission's
work, retention/reaping and acknowledgement remain explicit. No shared scheduler,
protocol router, task/result registry, automatic publication or background drain
is introduced. A turn's return does not establish idle work or complete release.
Explicit `retire` still checks the exact original member set, native retirement,
unread finite outcomes, original Actions and notification publications. Disposed
native resources and retired logical publication do not refund externally held
settlement/definition funding. Drop retains only its qualified finite local abort.
The [Host integration evidence](../evidence/host-execution-integration.md) bounds
availability; this is not mixed security, all-profile or production qualification.

### Caller-owned finite Read

`borrowed_consumer::ReadRuntime` demonstrates the modern finite kernel with one
caller-owned slot, no std, reference counts, pointer atomics or synchronization.
Its `LocalConsumerRef` loan proves issuer lifetime/identity without revocation
power; the `LocalConsumerInstance` owner's monotonic Cell
revokes authority, and checked non-reused sequences distinguish obligations across
slot/facet reuse. Equal numeric Thing slots do not grant authority. Exclusive
`&mut` progress owns native work; permission and codec callbacks can still revoke
external authority, which is checked again before acceptance/handoff.

`ReadDefinition` borrows source-free value meaning, an actual Binding/program,
the actual bounded validator, and owns its configuration snapshot. Current access
policy is bound separately. Passive mapping, complete support, validator bounds,
availability and policy revision are checked around linked permission callbacks.
Neither NoSec nor a schema marker grants permission. The representation currently
refuses all other security requirements rather than bypassing their obligations.

Caller-provided definition/service/scratch storage remains with its original
owner through safe loans. Before native construction, the caller supplies a
Foundation `ResourceReservation` for complete Engine work/output/backing, including
retirement; owner identity, resource kind and capacity must match. The reservation
moves with the accepted native work and unread settlement. Refusal returns it
unchanged. `into_funding` disposes native work and output before returning the
same complete lease for reuse; Drop refunds it after local disposal. This is no
universal account/storage abstraction or claim about private native SDK memory.

The shared kernel preserves the first cause independently of native errors,
cleanup failures and normal/late output. Read's native facet permits local cleanup
acknowledgement or a retained racing result; it grants no external cleanup transfer
destination or remote cancellation guarantee. Pending/failed cleanup retains its
original native owner and paid progress remains possible after revocation. Drop
must dispose local resources finitely without network waits and cannot claim an
acknowledged drain. Losing a copyable token does not drive background cleanup;
the slot owner can close/step or perform local disposal.

The [downstream fixture](../evidence/constrained-finite-execution.md) exercises
these rules with real datagrams and inline output, plus no-atomic compilation.
Host finite execution shares the kernel but retains its original Rc registry,
reentrant controls, Host account bank and boxed native calls. This finite Read
evidence does not qualify continuing Actions. Notifications, Producer publication,
TD preparation, general caller-provided
structured-schema storage, concrete Embedded drivers and automatic scheduling
are outside this constrained execution claim.

### Caller-owned continuing Action

`borrowed_action::ActionSlot` supplies one continuing Consumer invocation
opportunity using the common Action kernel. It borrows a source-free
`ActionDefinition`, actual native program, captured per-operation data/access
services and `LocalConsumerRef`; that loan proves original issuer lifetime and
identity without revocation power. Separate equal numeric slots are foreign.
Checked non-reused issuer sequences fence old handles across slot reuse. The
exclusive mutable slot loan prevents duplicate native progress; callback-time
external authority/service changes remain observable and must be fenced.

The implemented subset is NoSec InvokeAction with optional QueryAction and
CancelAction, static prepared value meaning and primary output. Unsupported
security, dynamic URI meaning, response alternatives, missing service/schema
support or changed native configuration refuse before dependent effects. This
is execution-only qualification, not TD parsing/admission or a production
protocol mapping. Native continuation compatibility belongs to the actual
Binding's checked configuration; its typed `ActionExchange`/token/output can
differ from Host Box/Any/Arc carriers. Destination calculation is pure and
original-instance policy checks cover the same actual destination used by start.
Invoke input and Invoke/Query output use their own captured media/body/work
services over one shared schema pair. No codec is selected from untrusted media.

After admission prepayment, capture one input payload view before fresh
authorization. Validation and native start MUST use that same borrowed view,
including its media/coding. Retaining the original input owner does not prove
that repeated views select the same payload. No further input-view callback may
follow final authorization; input-view-time revocation MUST refuse before native
start. This requires neither an input copy nor a different ownership format.

Definition and service storage stay with the caller through safe loans. Before
native construction a complete original Foundation reservation must match owner,
resource kind and complete inline plus declared indirect work/input/result/token/
retirement backing. Refusal returns the identical input and reservation.
Accepted work owns them through every exchange and unread settlement. Complete
new-work qualification is distinct from live dependency checking: original Invoke
native/authority dependencies remain, while data dependencies follow the actual
current exchange/unread output. Unused or retired controls and a retired input
codec do not revoke independently represented Query work. A pre-effect control
refusal retains unrelated accepted work; it grants no control permission.

Query and remote Cancel remain explicit. Local cancel, deadline, dependency loss,
slot close or issuer revocation preserve first cause independently of native
poll failure, failed/Pending close, validation failure and late output. Cleanup
uses original funding without new authority. It enters no further effectful poll
after local fencing. Accepted remote cancellation remains uncertain about remote
termination/rollback; native continuation is evidence, not logical authority.
Close acknowledgement concerns local exchange retirement only. Drop performs
finite local disposal without waiting or claiming an acknowledged drain.

The slot retains an unread completion until `take_settlement` prepays cleanup
and performs fresh time, original authority and paid service/access checks.
Short credit retains the slot and original output. Failed handoff retains native
output as evidence. Successful handoff transfers the complete outcome owner and
its current recipient permission; subsequent revocation cannot retarget or
retract that ownership. This transfer provides no new payload view and does not
seal the bytes that a generic `ActionOutput::view` may select later.

`ActionSettlement::result(&mut WorkBudget)` is a paid application-loan edge.
Before obtaining a view it prepays one `HandlerSteps` bounded view-adapter
operation plus the returning operation's originally captured `CodecInputBytes`
validation allowance. It captures one view, checks its complete application
shape (including primary response metadata), media/coding/body bounds and shared
output schema with the original data service, and returns that exact loan.
No second output view or authorization callback follows this validation.
Progress/retirement validation remains mandatory, including late output after
local failure; its result is no certificate for a subsequent arbitrary view.
This contract guarantees each returned loan matches its own validation, rather
than asserting that all historic views are identical. Host's immutable output
carrier remains a different qualified representation.

Short loan credit enters no view/codec callback, records no failure and retains
all original owners for retry. A loan validation failure records a sticky first
cause and validation error while retaining native evidence, input, continuation
and funding; it cannot erase an earlier cause or assert remote rollback. An
unavailable original codec is not re-entered. No native poll, cleanup, new account,
allocation, copy or fresh authorization is performed by this loan. Permission
already transferred with the owner. Already-returned loans remain ordinary
immutable borrows even if a selector, service or logical authority later changes.
Future loans require their own original-service validation and may refuse.

`native_result()` lends current unvalidated native evidence, including rejected
and late output. `input()` calls `view()` on the retained original input owner
and lends its current view; it is not the exact dispatched payload or a dispatch
receipt. Admission's single input loan is used unchanged for validation/start
but is not stored in the movable settlement. An input facet backed by a selector
may therefore expose `99` here even though the actual wire request used validated
`7`. Applications requiring a byte receipt can explicitly retain one or use an
immutable input representation; no implicit copy is required.

The settlement retains original input, output, continuation, dependencies and
reservation; result loans cannot escape their owner or survive moving it into
`into_funding`. That method disposes native/input/result owners before returning
the exact lease. Drop refunds it only after local disposal.

The [cross-representation evidence](../evidence/continuing-action-runtime-independence.md)
joins real datagrams, common Host Action regressions and no-atomic compilation.
No separate awaited-result wake contract or embedded transport is supplied.
Constrained TD preparation, notifications, Producer publication, credentials,
general schema arenas, hardware/allocator/time qualification and broader runtime
independence remain open.

QueryAllActions retains its finite query's original Consumer authority, root
mapping, named output schemas, actual services and normal/late native payload
through settlement disposal. Cancellation of this query closes its native
exchange; it does not cancel any Action. Short validation or handoff work retains
the original observation and slot. Recheck authority and service/policy after
callbacks and before handoff, including an already validated unread result.

Producer root status observation prepays the bounded inventory walk before root
or member policy callbacks and rechecks original currentness after callbacks.
It borrows original invocation owners without polling, cancellation, re-binding
or result acknowledgement. Native formatting/backing retains its own paid bound;
capacity failure refuses the whole query without truncation. Publication's
complete inventory check prevents a partial native owner advertising all Actions.
Pending original replies retain their original identity across replacement and
close, with native local-write evidence distinct from remote receipt.
Each admitted Producer Action owns a result-slot lease independently of its
logical record. The slot covers both live invocation and transferred result;
acknowledgement does not replenish capacity. It moves into `RawActionSettlement`
and survives native-record removal, replacement and Servient/exposure loss.
Retained settlements continue to count against the original store's invocation
bound; repeated acknowledgement cannot accumulate unbounded results under one
fixed reservation. Disposing one result permits exactly that slot to be reused.

Managed publication binds its original Host account bank before Serving. Before
behavior entry, each Action acquires separate EngineLiveBytesGlobalMax and
CleanupBytesMax reservations for `output_retained_bytes + size_of::<RawActionSettlement>()`,
plus one CleanupItemsMax reservation. Any failed acquisition releases the partial
set and leaves application behavior and result-slot usage untouched. Successful
admission transfers these exact leases through result disposal. These are
controlled-backing/cleanup allowances, not additive physical RAM measurements;
native I/O/formatting and application work have their existing separate owners.
Raw execution remains caller-funded and enforces the same result-slot lifetime.

Original result/target/validator storage is disposed before its result leases and
retained instance funding. Both completed-query and settlement APIs lend only
byte/media slices and fixed metadata; they never expose cloneable owners of the
original payload allocation. Sharing the original requires retaining the result
owner (for example Rc<RawActionSettlement>); explicitly copying its bytes creates
separate caller-owned storage. A borrow of InteractionOutput/Payload would permit
an uncharged Arc escape and is not this contract. Original validator retention
also survives acknowledgement. Acknowledgement is not final physical release.

`SAFE-OVERFLOW-001`: Every retained input/result/notification queue MUST have item
and backing-byte limits with visible loss/backpressure/termination policy.
Overflow reporting and cleanup MUST progress without enqueueing into the full
queue or relying only on the blocked producer. Fixed counters/bounded terminal
status retain first cause. Application credit cannot imply native backpressure.

The state projection names these semantic boundaries without mandating public
guard stages. Current Property Read implementations keep their existing linear
carriers, final checks, result seals and funding at their measured scopes. Their
historic success is not acceptance of the successor runtime.

The owned Host composed Producer path stages original native participation
identities before setup and reserves its controlled backing and qualification
scratch at a shared account bank before taking the caller's pure owners. The
original instance and each participation retain the same lifetime reservation;
replacement, lost exposure guards and unread results cannot refund it. Complete
native retirement and acknowledgement of retained Actions/Events are separate checks.
After a lost publication guard, logical reclamation requires actual disappearance
of every original native owner and acknowledgement of all original Action/Event work.
Neither guard Drop nor a diagnostic instance key proves completed native drain.

The local publication commit flips preinstalled per-participation ingress gates
rather than calling third-party `open()` after committing logical authority.
Readiness, gate loans, uniqueness and all fallible setup finish before commit;
gate writes have no callback or partial-failure branch. A revoked exposure closes
every original gate without requiring mutable access to the native drivers.
Accepted Action work and physical retirement retain their independent owners.

The optional [shared HTTP listener](binding-spi.md#independent-http-listener-participation)
retains native infrastructure independently of these publication gates. Staged
native buffering and route leases grant no application authority; every handed-off
exchange retains its original participant, instance and result/cleanup reservation.
Closing that participant releases its native lease even if its original unread
settlement still prevents logical retirement. A sibling or replacement cannot
acknowledge or fund the old work. Shared socket/parser/queue backing has the
Binding's bounded native policy; each managed publication still pays its actual
mapping, input/result, qualification and cleanup costs at its own original bank.

Producer Property value/Event data publication owns an independent native publication and pins the
original logical instance/reservation, including through replacement and loss of
the exposure or Servient root. Data admission invokes the publication-captured
policy/validator and checks the complete retained carrier backing. The native
participant's declared bounded slot covers emission, frame, data, result and
cleanup backing; refusal returns the original data. Cancellation records first
local cause without work credit; funded progress retains any late native effect
and completes retirement. Unread terminal settlement still holds its native slot
and funding. Logical retirement additionally checks outstanding notification work, even
if a participant incorrectly reports itself retired. Native publication errors
fence affected logical currentness, and actual native service failure fences its
dependent participations on progress. Drop performs bounded local disposal and
never claims delivered Property values/Event data or completed remote cancellation.

Native retirement Pending/error retains the complete emission and prevents
acknowledgement. The first cleanup error remains observable alongside the first
local cause and native publication effect; later successful local disposal is
required before terminal acknowledgement. Linked native callbacks are followed
by fresh authority checks before successful local settlement.

A managed Producer participant that excludes already funded prepared backing
from its additional byte declaration must prove complete inventory pairing as
well as exact shared-owner identity. The existing publication coverage check
compares its declared operation count with the covered prepared associations at
admission and before publication. Unpaired native definitions refuse handoff;
selected-entry equality alone cannot fund an additional retained schema.

## Reusable Producer definition ownership

Complete Producer publication remains per instance. `ComposedProducerExposure`
owns that instance's complete selected mappings, actual providers, native member
identities, final-check scratch and publication/retirement guard. It may retain
references to shared immutable Binding instructions; it does not become a shared
Thing or acquire another instance's authority, native participant or shutdown
right. A failed publication retires only its original membership. A sibling
keeps its own current permission, behavior, native progress and settlement.

Explicitly funded HTTP instructions retain their original physical definition
bank through every program/definition/native snapshot, independently of all
exposure banks. Instance admission pays its actual reference/operand and runtime
cost rather than charging the shared physical allocation again. Source or sibling
retirement cannot release that original charge early or move it to another bank.
Even a detached immutable program reference keeps the original lease; the final
physical instruction/schema/response owner releases it after disposal. This is
separate from native socket retirement and original unread result ownership.

The [preparation contract](planning.md#reusable-producer-preparation) and
[concrete HTTP boundary](binding-spi.md#reusable-http-producer-instructions)
do not remove complete final-TD qualification, native collisions, fresh ingress
authority or acknowledged retirement. The [evidence](../evidence/producer-preparation-reuse.md)
is a bounded manual Host integration, not a per-Thing scheduler or runtime.

## Resolved Consumer destinations

The finite Host owner validates variable inputs, reserves original operand,
destination, native/result and cleanup backing, then pays for pure native
resolution. The returned destination is retained through the original call and
settlement. Fresh access checks, credential issuance and every later permission
check use that operand. Service/configuration/currentness checks fence callbacks
before native effects and successful handoff. Native start independently verifies
the resolution; a mismatched destination or credential audience refuses before
I/O. An unresolved template is never a destination permission certificate.

`read_property_with_variables` and `write_property_with_variables` borrow bounded
lexical inputs during admission and retain funded copies for native acceptance.
Refusal returns original Write payload ownership. Cancellation, replacement,
late values and final release use the existing finite state machine and original
native owners; dynamic targets add no replay, cleanup transfer or remote-effect
guarantee. Result retention is independent of native connection occupancy.
Host reserved allocation policy is not a recoverable process-wide OOM guarantee.

## Structured data validation

The finite Consumer borrows its original PreparedSchema, with actual validator
qualification and paid scratch/work at input, output and late-result boundaries.
Source retirement, cancellation or revocation do not replace or refund the
constraint owner. Invalid input returns its protected payload before native
admission. A normal or late native value failing nested constraints remains
original rejected evidence, never validated application success.
The qualified HTTP Producer applies the same Core constraints through its
captured JSON value access before behavior permission/effects and before declared
output delivery. Its complete final-TD/publication checks remain independent of
Consumer partial support. Decoder traversal and constraint disposal have the
bounded envelopes in [Core](interaction-core.md); callbacks remain cooperative.

Continuing Action input/output borrow the same prepared constraints and retain
their original funding through normal or late result disposal. The Consumer's
Invoke data service checks input before native effects; the operation returning
a completed value supplies its captured validator and resource policy, including
Query output arriving during control-exchange retirement. A
Producer's captured Action validator checks before behavior authorization and
before successful result observation. Acceptance/pending/cancel acknowledgements
do not become application output. Native failure, validation failure, local
cancellation/revocation and remote effect certainty remain separate evidence;
replacement cannot retarget the original constraints, work or result.

Producer Action validation also retains the original extra callback-work bound.
The invoke path funds both input and possible immediate output before entering
validation or behavior; a continuing poll funds output validation before it can
materialize a result. Short credit refuses pre-effect invocation or keeps the
continuation pending without polling it. Metadata is rechecked before/after
validation, after authorization and before immediate/continuing output validation;
a changed allowance produces a
validation failure without calling an unfunded output validator. Restoring the
declaration cannot reopen that terminal failure. Close continues on its separate
cleanup budget without handler/codec credits, even if service metadata changed.

## Declared finite failures

A validated TD-declared application failure remains an original output with
ApplicationError status and its selected branch/data. It does not replace a
first cancellation, timeout, policy or authority cause. Normal, unread and late
outputs retain original definitions, actual services, bounded result backing and
cleanup funding through disposal. Failed schema checks remain validation errors
and retain rejected Consumer output under the existing settlement loan.
Producer failure after behavior entry and Consumer failure after native progress
remain effect-uncertain; a declared error cannot prove pre-effect refusal or
rollback. Delivery of that error response is separately observable. The shared
finite transition kernel and Security verification/revision ordering are unchanged.

The response-context loan is obtained before the final pre-handler currentness,
service-pair and security-revision checks. It exposes immutable meaning only;
its callback cannot dispatch, perform I/O or refresh authority. The original
native work retains its mapping/contract through validation and delivery.
Clearing native work after physical retirement cannot redirect an accepted or
late result to a sibling mapping. A borrowed handler context cannot outlive that
original owner or turn branch inspection into native execution authority.

Under SPI-INGRESS/RETIRE-001 and SAFE-AUTH/CLEANUP-001, native Producer failure
facts are reconciled at Servient. A failed participant may fence its own ingress
and close its native resources; it cannot revoke logical currentness or dispose
application continuations. Servient validates the original issuer/instance
association and exact membership in that incarnation's committed native set
before propagating native failure to logical revocation. A stale, foreign or
uncommitted participant cannot revoke a publication or cancel its accepted work.
Uncommitted failure cleanup remains local and may precede publication. Native
and application progress, acknowledgement and final result disposal remain
independent and caller-driven.
An encoded reply may finish after ingress closes; completed native delivery never
acknowledges the Action. Original Action work/result funding survives native
retirement, replacement and lost handles through the existing store/retirement
owner. Dropping a root still owes its qualified finite local abort; this correction
adds no detached task or generic cleanup destination.
