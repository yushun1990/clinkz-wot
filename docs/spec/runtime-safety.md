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

A bound Property Read retains the selected mapping, original authority and fixed
destination. The Binding resolves a fixed destination once. Variable templates
retain per-interaction validation and resolution. Repeated Reads need no new
name lookup. Dropping a handle is passive. Removal or replacement fences it.
The destination owner supplies immutable bytes through the last handle,
native request or settlement using them. A dynamic destination belongs to
its admitted call. Refusal returns protected operands to their original owner.
Fresh dependency, permission, credential, time, capacity and actual result checks
continue at the existing callback and handoff boundaries.

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
or linked destruction. Removing an association fences only that facet. Removing
or replacing a parent fences all its facets. Instance replacement requires
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

Finite associations retain their actual bindings, content and services through
last use. Binding an association supplies no source/compiler callback or
execution/configuration override. Actual native
concurrency remains enforced by its original registration. Dropping preparation
or removing a sibling cannot reclaim backing still needed by another mapping or
normal/late settlement. `ConsumerCalls` supplies bounded work capacity separately:
one owner may serve many associations, and one association may use several work
owners. Dedicated or on-demand capacity is valid; no pool, global registry,
per-instance Runtime or executor is required.

Admission claims the original work slot before linked callbacks can reuse it.
Its call retains the original association authority, selected binding, resolved
destination, lexical variables, input/selection and deadline. Native retirement
cannot retarget those operands. Settlement lends its original input/result
slices and destination/variables. [Capacity and handoff](foundation.md)

Logical removal and actual logical-owner Drop are passive fences. They do not
signal an OS descriptor, run a scheduler, call a Rust Waker or perform native
cancellation. The deployment supplies bounded `ConsumerCall::poll` or
`ConsumerCalls::step` turns after lifecycle changes and abandonment, independent
of native I/O wakes. A lifecycle-command loop, cyclic executive or actual timer
may supply those turns. A logical `Deadline` is only an instant/comparison;
expiry is observed on supplied clock observations, not by an installed timer.
Native completion does not cover a silent peer or indefinite subscription.
Aggregate progress also requires the application to arrange result-consumer turns
if native wake interest has moved to another task. The optional
[Host turn composition](#host-consumer-execution-turns) observes application results
in the same driving task. Work that yields retains its progress owner.

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
bounds the full selected map. A change invalidating the service fences dependent dispatch.
Selected reads retain the original per-call bitmap into the immutable schema;
neither replacement nor another call can change it. The captured data policy
(`encoded_selection_bytes`) separately declares complete encoded-selection
payload backing, including media/Arc allocations; zero declines that facet. The request
array is encoded in bounded storage with space for the result and rollback. Refused
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
output, lends Core `InteractionOutputView`/`PayloadView` slices. The views keep
the payload under its original backing owner. Native
cancellation still exposes its original retry and borrowed cleanup disposition;
it grants no transfer ownership. Retain the settlement (including a shared Rc
owner) to share original bytes. Caller-owned explicit copies have distinct
backing. The retained response and its local disposal stay backed through final
settlement disposal, including after a local error or root loss.
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
runtime exclusively. A callback can request progress for a sibling under the
inclusive turn bound in SAFE-CONCUR-001. The selected driver has exclusive use
of the original native work. `ProducerRevocation` captures one-way control of
the original incarnation. It withdraws its gates without a registry lookup or
a linked callback. A retained control cannot address a successor.
The final acceptance and delivery checks observe revocation inside callbacks.

### Finite Producer participation and work capacity

A publication owns its ready members. An accepted exchange owns its original
reply and an independently backed settlement. A serial Binding may lend its only
exchange slot, and a multiplexing Binding may support several on one member.
Publication requires the ability to serve the mapping. Concurrent call storage
may be supplied on demand, independently of the number of idle Things.

Before semantic acceptance, verify the original issuer, generation, membership,
current permission, behavior, deadline and capacity. A Binding may move the
reply, lend an exclusive native slot or copy into admitted storage under its
delivery contract. A refused offer remains complete with its original owner and
deadline. Pending native parsing belongs to the Binding. Accepting an exchange
requires actual storage for its input, promised result/handoff and local close.
An active slot and a retained result slot can have different release points.

On timeout, stop new effects and retain the first cause, original reply,
behavior result, effect and delivery evidence. A retryable cleanup error on an
isolated reply stays with that exchange, separately observable from its timeout.
Keep the original reply and its active capacity for retry while another exchange
on the same healthy member finishes. Check original membership, services and
ingress health before the next acceptance. When the Binding reports loss of
isolation or of required ingress, fence the affected participation and its
dependent publication and drain their original work. Record the member failure
separately. [Finite reply retirement](binding-spi.md#finite-reply-retirement)

Release active capacity at native retirement. Transferring an unread outcome
to the caller moves its original backing or exclusive loan. The application
may hold it across publication retirement, replacement or root loss. Only
disposal makes that backing reusable. Acknowledging an old result cannot release
a reused coordinate. A full result pool may refuse *new* admissions. An
already accepted exchange can still settle. [Earlier serial HTTP evidence](../evidence/producer-finite-capacity.md)

### Continuing Action admission and handoff

Before an Invoke handler can start continuing work, Servient claims the
original invocation and the Binding secures the initiating reply. A mapping
that promises an addressable wire identity or a later control or inventory
lookup must also secure its correlation through handler entry and installation.
Installing the ID needs no further acquisition. If a required claim fails,
unwind before entering the handler. An acknowledgement-only mapping can start
local work with its invocation and reply backed. Failure of the initial reply
leaves the continuation, cleanup and any promised correlation owned for their
advertised lifetimes. Each later exchange retains its own reply and deadline.

On the Consumer, the root retains an abandoned invocation handle for
round-robin manual cleanup. If
all local owners disappear, the captured native finite-abort contract applies.
Neither path claims that independent remote Action work stopped. Callback-time
authority/service checks fence late success; original native result, validation
failure and first local cause remain distinct. Handles cannot substitute mappings,
move to a successor incarnation or address a reused slot.

Native result readiness is separate from successful local handoff. Unread Action
completions remain subject to cancellation, original authority/root revocation,
deadline and captured service/policy loss. Settlement handoff takes current time
from the supplied monotonic clock. Check destination, current access and captured
services after callbacks. A delayed handoff retains the original slot and result.
Native errors and late output remain evidence. Validating the output after local
retirement preserves the first cause.

The [caller-owned Action representation](#caller-owned-continuing-action) also
validates each application result loan. Moving its generic output owner does
not prove that a later `ActionOutput::view` selects the previously checked value.

### Notification associations and conversation capacity

A notification association retains the original Consumer, definition and access
service. Passive views cannot revoke on Drop. Binding checks the prepared
dependencies and captures the access configuration. A later capacity constructor
cannot refresh it. An idle association needs no queue, interest or work slot.

Subscribe claims a bounded conversation slot and usable storage for the selected
validation, queue, reply and retirement. Include any quota the deployment
advertises for that subscription. Each entry keeps its original participant
and rights through local retirement. Acknowledgement releases the conversation
slot. Retained reports keep original prepared/association owners for any
shared target backing until report disposal. Capacity Drop performs qualified
whole-owner finite local abort; it does not revoke persistent associations or
other capacity domains. Original instance revocation fences only its work.
Applications schedule progress/cleanup after passive revocation and cursor loss,
either individually or with the aggregate turn below. There is no scheduler or
automatic authority-change wake mechanism.

`RawNotificationServient::step` and `ConsumerNotifications::step` visit every
original conversation at most once per turn, rotating the first visit across
turns. A pending participant or retained error cannot starve sibling visits.
Each bounded visit uses the original policy, validation, queue, response and
retirement transitions. Passive observation can see a stop or authority fence.
The turn invents no application
response, cancellation input, remote control or permission.

A live cursor retains queued values and its closed unread report. Cursor Drop
records abandonment separately from stop intent. Aggregate progress disposes an
abandoned cursor's report only after complete local retirement. A pending or failed retirement retains the original response or
participation and its capacity. A surviving stop view or saved identity does not prevent
this disposal or revive the cursor. Applications needing reports after cursor
loss use the existing explicit `progress`/`step_cleanup`/`take_settlement` path
instead of automatic abandoned-report disposal. This is an opt-in consequence
of aggregate driving, matching abandonment of finite/Action handles; it does not
discard a live recipient's result or report. Neither acknowledgement nor idle
capacity proves release of independently retained result/definition/native owners.

Prepared data policy/media/native support MUST match at association binding,
admission and applicable delayed handoffs. Availability, schema, access and configuration checks surround policy and validation callbacks. Changed
dependencies refuse or fence the original obligation. Cleanup retains its
original owner even when those dependencies are unusable. Changed services
require explicit qualification.

Validation MUST preserve an observed dependency violation separately from ordinary
caller-input refusal or a yield. Both scalar and collection validators recheck
captured dependencies after the codec callback, including on error. Observed
dependency loss during response admission or receive fences the original
conversation. Restoring the configuration cannot revive it or permit a response.
A recoverable refusal still requires a passive currentness
check before return so synchronous stop, instance revocation or pairing loss is
not hidden by the error. Dependency origin cannot be inferred from `CoreError` or
reconstructed by rereading declarations after the failure.

Response refusal returns the exact caller input without adopting it or consuming
the native reply opportunity. A fenced conversation retains its original occurrence,
reply job, first cause and retirement storage even while close is pending.
Local close uses the original right independently of validation or permission.
Admission failure has no admitted conversation to fence. Cancellation has already
fenced local semantic use before its validation callback. Validation failure of an
owned native candidate remains fatal regardless of error origin. None of these
paths changes subscription authority versus individual occurrence refusal.

The validator borrows the original Property value/map or Event role trees,
uses its bounded validation workspace and checks data before queue acceptance.
For a composed conversation, each payload must match the selected Form's
content type before the codec sees it. This applies to subscription input,
occurrences, responses and mapped cancellation input, including collection data.
The original occurrence coordinate survives staging/queueing and is supplied to
fresh Deliver/Respond policy at acceptance, `front`, response admission and delayed
native response send. These decisions always carry an owned item. Separate Maintain
checks, without an occurrence, preserve the current subscription grant before
polling or semantic use. Fresh Maintain checks after occurrence policy ensure that
synchronous grant withdrawal overrides even a positive item refusal. Both checks
fit within the qualified turn. Check original recipient, scope, backing and native
response support before occurrence policy. The composed policy also checks
captured Event membership.

An explicit RejectOccurrence marks only the original staged/queued item as refused.
It cannot be exposed, advanced, responded to or resurrected by subsequent policy
changes. `front()` returns None for that refused head; `respond()` returns the exact
unaccepted caller input with AuthorizationDenied. A bounded progress or
round-robin cleanup visit retires the refused item without closing the native
participation. Each turn performs at most one native retirement poll or one item
disposal; it never scans a stream of refused items. Later queue entries remain
behind the original refused head until its retirement completes. Callers continue
manual progress after an empty view or pending retirement.

Refused payloads and submitted responses stay funded until their exact reply
owner retires and their own disposal step completes. Required response opportunities
can be retired on refusal without fabricating a send. Pending cleanup retains
the original owner and needs no renewed delivery permission or codec callback.
Failure to retire the reply fences its conversation and retains the error and
reply for cleanup. Preserve sent or uncertain response evidence independently
of the refusal.
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

### Producer publication

`SAFE-PUBLISH-001`: Host and caller-owned Producers MUST use one publication
lifecycle. Before taking a closed endpoint, validate known draft requirements
and behavior, secure a bounded candidate and the capacity to release that
endpoint and return the inputs. Each Binding assembles its participants with
ingress closed. Resolve Forms against the actual base and apply PREP-COVER-001
to every final operation, alternative, eligible provider and behavior. A
source-free caller supplies a trusted prepared final definition and verifies its
coverage against the actual deployment. Changed meaning needs new preparation
or refusal.

Acquire the exact selected members and their rollback storage as they become
known. On shortage unwind partial claims without waiting for another staged
candidate to yield a member. An advance promise supplies its already-held slots
to these same operations and releases surplus. Capacity cannot select a cheaper
provider, hide ambiguity or shrink the TD. After the last linked callback,
hold the expected incumbent, final TD, coverage, behavior, services, membership,
collision checks and readiness current together. Publish that TD and open the
original gates in one bounded, callback-free, infallible local commit.

A yield retains the candidate and endpoint. Cancellation, expiry or terminal
refusal fences the candidate, keeps ingress closed and returns the original
inputs once rollback releases them. A terminal error stays terminal while the
physical owner drains. A foreign driver cannot consume the candidate. A
coexisting incumbent keeps serving. An exclusive route may require a service
gap. Report any visible native residue. Publication commits local authority
over this description. Remote caches have independent lifetimes.

A later deterministic unsupported operation/codec/schema/security/behavior/config
branch already present at publication is a qualification/implementation defect,
not normal Unsupported. Fail closed for affected acceptance, preserve exact first
cause and cleanup, and requalify before claiming readiness. Changed endpoint
meaning/providers/support policy invalidates prior qualification; equivalent
reconnection is availability recovery. Per-use authorization/credential rotation
is not by itself new immutable definition content.

### Cancellation and late results

`SAFE-CANCEL-001`: Cancellation/timeout MUST fix one first local cause and fence
new interaction effects. Accepted work keeps its original identity, result
backing and retirement owner. When timeout wins, the caller receives `TimedOut`.
The original owner MUST retain a later native result as evidence through
settlement or disposal. It cannot turn the timed-out operation into successful
delivery. Mapped remote cancellation requires fresh authorization. Local
settlement leaves remote rollback, effect count and physical shutdown uncertain. A constructor rejection proves no escaped
obligation; an operational error after effects cannot make that assertion merely
by its error type. Cancellation, request deadline and cleanup/drain deadline are
distinct phase contexts. All deadlines use the supplied monotonic clock. Recheck
at effect and handoff boundaries after linked code returns. Local close proceeds
with its original owner under saturation, independently of access checks, codec
work and the success of a mapped remote Cancel.

`SAFE-CLEANUP-001`: Every accepted or provisional obligation MUST be able to
retire in its original slot or transfer to a destination with available backing.
Ownership transfers only when the destination
acknowledges receipt of the complete live owner/backing; a request, status record,
spawned task or numeric token is not transfer. Rejection returns the same owner
to retained/manual cleanup. Capacity stays charged through quarantine/pending
cleanup. A residual report distinguishes external uncertainty and does not permit
freeing surviving local state. A Host root needs qualified finite local abort or
acknowledged transfer to a longer-lived supervisor retaining all obligations;
an executor that may disappear is insufficient. Physical retirement remains at
the actual infrastructure owner. No callback/guard can silently orphan work.

`SAFE-DROP-001`: Drop MAY fence or request cancellation, dispose of bounded
local state, or move the whole owner to an acknowledging destination. Explicit
close/destroy reports local drain and outstanding remote uncertainty. A driver
retains an owner for variable destruction and can release it over bounded turns.
A destructor cannot wait for a peer or user callback, or certify completed drain.

`SAFE-CONCUR-001`: Acceptance, revocation, publication, completion and transfer
MUST have explicit linearization points under one logical ownership authority.
No handler, Binding/service callback, allocation or unbounded destruction occurs
under registry-wide locks or constrained critical sections. A public turn
includes all synchronous callbacks and nested drives, even through independent
siblings. The deployment may defer nested drives and schedule the target for a
later turn. Cancellation and revocation still fence checked-out work immediately.
A deployment allowing nesting must bound inclusive depth, fan-out and callback
cost at every reachable drive entry. Ready siblings and draining owners get
turns under admission pressure.

Pending names the retry owner and enabling event. An event-driven capacity
wait registers bounded interest, then rechecks before sleeping. Publish the
released slot before signalling. A release racing registration must either be
seen on recheck or wake the driver. A maintenance or caller-owned loop may
instead guarantee independent retry turns. On retry, check the original
deadline and current authority. Another contender may win. Before acceptance
the caller may take explicit backpressure and the complete offer, or the
Binding may retain it for redelivery. After acceptance the owner remains
responsible for progress. Taking a result leaves its loan held until disposal.

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
its staged backing and consumes no slot ticket. Lookup and mutation are bounded
by key length and tree height. Removing or replacing an entry fences it before
dropping owners outside borrows. Accepted calls remain in their
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
Things. Removal can drain that Thing's own configurations and services in bounded slices. Whole-owner shutdown necessarily visits every parent
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

The Host `ConsumerActionAssociation` retains the original Consumer, a shared
prepared definition and separately qualified per-operation access owners. Association construction allocates no vacant slot, exchange,
continuation or work quota. It rechecks prepared data/native declarations and
original backing before/after policy qualification. Explicit new access binding
captures its actual configuration and revision and preserves the original prepared dependencies.
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

Invoke's captured data service validates its input. For validated output,
the operation that returned the value supplies the original validator, media
and limits for the shared Action schema. The exchange's native response bound
still applies. The current subset admits no content coding. Keep the value and
its validation workspace available for a promised handoff. A close poll retains
racing output as raw evidence and releases the native slot when retirement
completes. Both retirement and raw-evidence disposal must finish with zero
access or codec callbacks, even if the validator is unavailable. Validated
delivery requires current permission and validation covering that exact value
and schema. Preserve the original bytes, first cause and any native, cleanup
or validation errors independently.

`ConsumerActions` supplies bounded invocation storage independently of the
Consumer. Before effects, claim an invocation slot, passive control, native
continuation, input and result/handoff backing and a path for retirement. Unread
results retain their backing through transfer and final disposal. A distinct
outstanding-work quota follows the lifetime promised by the deployment.
Refusal returns the original input and provisional capacity without native effects.

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
The original owner backs cleanup and any requested validated handoff. Physical
retirement can complete before validation. Neither changes the first cause or
resolves an uncertain remote effect.
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

A Host can drive finite calls, continuing Actions and notification capacities
in the same loop. Each turn uses the supplied monotonic clock and a bounded
number of ready operations, including callbacks under SAFE-CONCUR-001. Rotate
through ready and draining owners. A stalled family or full result pool cannot
block an independent ready sibling. Idle inspection is passive. An idle driver
can still have outstanding application-held results.

Managed composition starts with `ConsumerOwner` registration and shares its
original passive reference with independently authorized family associations.
The existing finite named-configuration registry stays finite; Action and
notification associations are explicitly bound, not looked up through a new
universal configuration registry. Removing/replacing a parent or dropping its
owner fences every original facet without retargeting work. Closing a capacity
still fences only that capacity. The progress helper accepts no new authority,
definition, service or configuration override and changes no admission rules.

The deployment supplies turns for native readiness, passive lifecycle changes,
deadlines and transitions that are locally runnable. A timer, command loop or
cyclic executive may supply maintenance. Parking for capacity uses the release
or retry contract in SAFE-CONCUR-001. Native wake interest alone cannot service
a silent deadline. Observe application results in the driving task or arrange
notification for their actual observer. Explicit cancellation can signal a
registered waiter. Passive owner loss remains a fence.

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

A Producer turn verifies original participation identity, issuer and instance
before entering a member. It can advance native receive, an accepted Action or
a drain independently. Use bounded operations or a cursor, rotating among ready
members. Retain the selected original invocation throughout its turn. Revoked
work may progress local close. A described CancelAction is a separate effect.
One stalled participant cannot starve another. Keep the member's first failure
and its native, delivery and cleanup evidence separately. Servient reconciles
observed loss of a required member against the committed publication.

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

The caller supplies actual definition, service, work and result storage. An
exclusive typed slot with sufficient backing and a retirement path suffices at
admission. Any separately promised quota follows its own owner. Refusal returns
the protected input or slot. A result loan can occupy the only available buffer
until disposal. More concurrency requires independent backing.

The shared kernel preserves the first cause independently of native errors,
cleanup failures and normal/late output. Read's native facet permits local cleanup
acknowledgement or a retained racing result; it grants no external cleanup transfer
destination or remote cancellation guarantee. Pending/failed cleanup retains its
original native owner, which can still close after revocation. Drop
must dispose local resources finitely without network waits and cannot claim an
acknowledged drain. Losing a copyable token does not drive background cleanup;
the slot owner can close/step or perform local disposal.

The [existing downstream fixture](../evidence/constrained-finite-execution.md)
exercises issuer identity, real datagrams and inline result retention, and
compiles for a target without pointer atomics. Its Read entry still takes a
reservation and work credits. The successor must exercise those cases with
the typed slot as backing. Host execution currently shares the finite kernel
and uses an Rc registry, reentrant controls, a Host account bank and boxed
native calls.

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

Capture one input payload view before fresh
authorization. Validation and native start MUST use that same borrowed view,
including its media/coding. Retaining the original input owner does not prove
that repeated views select the same payload. No further input-view callback may
follow final authorization; input-view-time revocation MUST refuse before native
start. This requires neither an input copy nor a different ownership format.

Definition and service storage stay with the caller through safe loans. Before
native construction claim usable storage for the input, token, output/handoff
and retirement. Refusal returns the identical input and any provisional slot.
Accepted work retains its owners across exchanges and unread settlement. At
admission check the whole mapping. For already admitted work, original Invoke
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

A slot retains an unread completion until successful handoff or local disposal.
Handoff checks the supplied time, original authority, destination and services
under current permission. Failure leaves the native evidence with its owner.
Where a generic output can expose different values on successive views, capture
one view and validate its identity, shape, media, coding, size and schema before
lending that exact value. A previous check can be reused only for the exact
value and schema covered. Permission loss refuses a new validated delivery.
A loan already returned remains an ordinary borrow under its original owner.

Raw evidence can expose rejected or late native output under its access policy.
Inspection of a retained input can expose its current view. Only an immutable
input or an explicit receipt establishes the bytes previously dispatched.
A validation failure retains its first cause and native evidence. The
settlement owns the original input, output, continuation and dependencies
until disposal. Neither inspection nor validation drives another native effect.

The [cross-representation evidence](../evidence/continuing-action-runtime-independence.md)
joins real datagrams, common Host Action regressions and no-atomic compilation
under the earlier admission contract. The successor must also demonstrate
authorized handoff of the exact value while general capacity is occupied.
Constrained TD preparation, notifications, Producer publication, credentials,
general schema arenas, hardware/allocator/time qualification and broader runtime
independence remain open.

QueryAllActions retains its finite query's original Consumer authority, root
mapping, named output schemas, actual services and normal/late native payload
through settlement disposal. Cancellation of this query closes its native
exchange; it does not cancel any Action. Short validation or handoff work retains
the original observation and slot. Recheck authority and service/policy after
callbacks and before handoff, including an already validated unread result.

Producer root status observation bounds the inventory walk, handle construction
and lookups, and rechecks original currentness after policy callbacks.
It borrows original invocation owners without polling, cancellation, re-binding
or result acknowledgement. The Binding bounds native formatting and backing.
Capacity failure refuses the whole query without truncation. Publication's
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

Before behavior entry, both managed and caller-owned Producers secure an actual
result slot and its handoff and retirement backing. The Binding protects the
matching continuation record under EXEC-ACTION-001. Failure releases provisional
claims and leaves behavior untouched. The result may outlive the publication.

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

Each staged Producer owns its closed participants and current qualification
workspace. Commit moves serving members into the publication and releases
candidate-only storage. Each accepted work item and shared service keeps its
own backing through last use. Logical retirement also checks outstanding
Actions and Events.
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
exchange retains its original participant, instance, result backing and local
retirement owner.
Closing that participant releases its native lease even if its original unread
settlement still prevents logical retirement. A sibling or replacement cannot
acknowledge the old work. The Binding bounds shared socket, parser and queue
storage. Each instance owns its mapping records and each call owns its settlement.

Producer Property value/Event data publication owns an independent native publication and pins the
original instance and backing through replacement and loss of
the exposure or Servient root. Data admission invokes the publication-captured
policy/validator and checks the complete retained carrier backing. The native
participant supplies bounded emission, frame, result and retirement storage.
Refusal returns the original data. Cancellation records the first local cause
immediately. Bounded progress retains late native effects and retires the
original emission. An unread settlement occupies its original
publication slot through disposal. The Binding controls the lifetime of each
physical resource used to produce that outcome. Logical retirement additionally
checks outstanding notification work, even if a participant incorrectly reports
itself retired. An individual delivery or recoverable cleanup error stays with
that work while a healthy Thing serves. Observed loss of a required installed
member fences its original publication immediately, including when an emission
observes it. Loss of a shared stream fences its dependents. An independent
service continues. Drop reports only actual local disposal.

Native retirement Pending/error retains the complete emission and prevents
acknowledgement. The first cleanup error remains observable alongside the first
local cause and native publication effect; later successful local disposal is
required before terminal acknowledgement. Linked native callbacks are followed
by fresh authority checks before successful local settlement.

Complete inventory pairing checks every prepared association against the actual
participation and every installed operation against the final TD. Each distinct
retained definition needs backing. A shared reference retains its original
owner through last use.

## Reusable Producer definition ownership

Complete Producer publication remains per instance. `ComposedProducerExposure`
stages that instance's selected mappings, eligible registrations, whole-TD
coverage and the actual workspace needed for qualification. Installation checks the actual native
mapping/service identities and holds all observed configuration versions
through commit. Completing or cancelling staging drops its preparation
references. Independent native resources, logical authority and accepted work
retain their own publication and retirement owners. A failed publication retires only its original membership. A sibling
keeps its own current permission, behavior, native progress and settlement.

Shared HTTP instructions retain their original backing through every program,
definition and native snapshot. Each instance supplies space for its references,
operands and runtime state. A detached immutable program reference keeps the
instruction, schema and response storage alive until final disposal. This is
separate from native socket retirement and original unread result ownership.

The [preparation contract](planning.md#reusable-producer-preparation) and
[concrete HTTP boundary](binding-spi.md#reusable-http-producer-instructions)
do not remove complete final-TD qualification, native collisions, fresh ingress
authority or acknowledged retirement. The [evidence](../evidence/producer-preparation-reuse.md)
is a bounded manual Host integration, not a per-Thing scheduler or runtime.

## Resolved Consumer destinations

The finite Host owner validates variable inputs, reserves original operand,
destination, native/result and cleanup backing, then runs bounded native
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
qualification and available scratch for bounded input, output and late-result checks.
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
Invoke data service checks input before native effects. A request for a
validated Query value uses Query's captured validator and policy, including
when the value arrived during retirement. A Producer's captured Action
validator checks before behavior authorization and before successful result
observation. Acceptance/pending/cancel acknowledgements
do not become application output. Native failure, validation failure, local
cancellation/revocation and remote effect certainty remain separate evidence;
replacement cannot retarget the original constraints, work or result.

The captured Action validator must handle the bounded input and possible
immediate or continuing output. Observe service changes around authorization
and validation. A change invalidating output support refuses delivery and
retains the original evidence. Close uses the original native right even
after validator or access loss.

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
before propagating native failure or withdrawal to logical revocation. The
registry and accepted work share the original member identities and ingress
gates. TD lookup and fresh dispatch observe their availability, including
withdrawal of a native owner moved out through a typed loan. Servient checks
this permission after linked callbacks and before granting behavior entry.
A finite participant checks its installed services and ingress independently
of new admission. Terminal member loss withdraws its gate. Servient fences
the gates of the original dependent publication. A delivery-local failure
retains its own settlement. An accepted reply keeps its original socket
through bounded delivery or retirement.
At replacement commit, Servient withdraws every displaced gate before opening
the successor's gates. Old participants can then drain their own work and
retire even if nobody looks up the old TD. A healthy staged gate waits for
its own publication decision.
A stale, foreign or uncommitted participant cannot revoke a publication or
cancel its accepted work.
At a staged publication decision, a recorded native failure rolls back the
candidate, releases qualification scratch and starts retirement of its original
members. This also applies when the terminal cause is backpressure. A healthy
candidate can yield for a bounded turn or wait for identified native readiness. Native
and application progress, acknowledgement and final result disposal remain
independent and caller-driven.
An encoded reply may finish after ingress closes; completed native delivery never
acknowledges the Action. Original Action work/result funding survives native
retirement, replacement and lost handles through the existing store/retirement
owner. Dropping a root still owes its qualified finite local abort; this correction
adds no detached task or generic cleanup destination.
