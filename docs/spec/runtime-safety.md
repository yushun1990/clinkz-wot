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
if native wake interest has moved to another task. Zero/partial credit retains
runnable/progress responsibility; no autonomous eventual cleanup is claimed.

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
The finite Producer retires its native exchange before settlement acknowledgement;
an unread Producer settlement keeps that runtime unavailable for the next receive.
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
Applications still schedule progress/cleanup after passive revocation and cursor
loss. There is no scheduler or automatic authority-change wake mechanism.

Prepared data policy/media/native support MUST match at association binding,
admission and applicable delayed handoffs. Fresh availability/schema support and
access work/backing/revision checks surround policy/validation callbacks. Changed
declarations refuse/fence the original obligation without entering an unfunded
callback or accepting a new baseline. Cleanup retains its prepaid owners even
when those declarations are no longer usable. Explicit new qualification is
required to use changed dependencies.

The validator borrows the original Property value/map or Event role trees,
prepays validation before native handoff and checks data before queue acceptance.
The original occurrence coordinate survives staging/queueing and is supplied to
fresh policy at acceptance, `front`, response admission and delayed native response
send. Check original recipient, scope and coordinate bounds before policy; the
composed policy also checks captured Event membership. Invalid/late candidates and
first causes remain with the original conversation through cleanup. NoSec MQTT
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
A control authorization/service failure fences the invocation even if the service
later recovers. Short work or an unsupported control remains a refusal without
inventing a remote effect. A successful native start returned after revocation
is retained for local retirement, without polling it for further effects.
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
reentrant controls, Host account bank and boxed native calls. Continuing Actions,
notifications, Producer publication, TD preparation, general caller-provided
structured-schema storage, concrete Embedded drivers and automatic scheduling
are outside this constrained execution claim.

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
their original funding through normal or late result disposal. The invoke
association's actual captured validator checks input before native effects and
completed output even when it arrives during control-exchange retirement. A
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
