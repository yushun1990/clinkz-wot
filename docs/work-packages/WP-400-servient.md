# WP-400 — Logical runtime and optional preparation facade — v6.0

## Target and dependency

Own instances, current grants, publication, captured behavior, accepted-work progress and cleanup. Finish execution/preparation dependency separation through actual owners. Support final endpoint qualification after closed setup and locally atomic Serving.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: SAFE-AUTH-001, SAFE-PUBLISH-001, SAFE-CANCEL-001, SAFE-CLEANUP-001, SAFE-DROP-001, SAFE-CONCUR-001, SAFE-PROFILE-001, SAFE-OVERFLOW-001.

Required evidence follows actual claims: Publication/cancel races, prepublication ingress, behavior coverage, qualification defects, replacement/late delivery, saturation and acknowledged transfer. Shared physical service close must not cancel siblings. Exact Host/static physical evidence remains separate.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## Public finite publication, acceptance and retirement

The [finite deployment evidence](../evidence/finite-producer-publication.md)
covers ordinary Host exposure and a trusted caller-owned Read, including
original-result ownership across replacement. Continuing and mixed deployment
drivers retain their separate migration below.

Use one Servient lifecycle with linked Binding-owned assembly on Host and typed
caller-owned storage for a trusted final TD. Secure rollback before each
acquisition. Replace the six-charge `WorkLease` with active exchange storage,
an independently retained settlement and any expressly shared deployment permit.
Replace handler, candidate, setup and qualification byte estimates with their
actual owners. Admission of an effect secures the result, validation overlap
and retirement path. Move any promised continuation and correlation into place
without a second admission. [Runtime Safety](../spec/runtime-safety.md)

For an advertised advance reservation, use the same Binding and member pool.
Contend both before and after the promise. After success exhaust the free slots
and deny allocation of promised backing. Publish or roll back using the original
slots and release surplus. Compare coverage, first cause, wire effects and
final release with ordinary admission for the same admitted inputs. A Binding
unable to supply this promise can serve ordinarily and must refuse that guarantee
before acquiring its endpoint. This trial is conditional on a deployment need.

The Host deployment now steps the actual Consumer capacities directly. Finite
calls and the caller-owned Read use bounded native polls. Complete the migration
on continuing Consumer progress, `ComposedProducerExposure::step` and the custom
`OwnedProducerParticipation::new` turn. Remove the remaining `ProducerWork`
credit selector with its callers. Drive ready siblings and local drains
while admission is saturated. Hold the only result slot with a second request
waiting. Taking the result still holds the slot. Its final disposal alone must enable exactly one dispatch via a pool
wake or promised maintenance turn, including release during waiter registration.
Exercise a three-owner callback chain. Deferring nested drives bounds a one-poll
outer turn to one poll while a nested cancellation takes effect immediately.
If the driver permits nesting, exercise its maximum depth and fan-out.

The implementation increments below record the original Host and caller-owned
paths. Their account and work-credit oracles characterize those revisions.
Use the owner and release oracles above for each migrated entry.

## Raw finite shared-instance slice

The additional [caller-owned finite Read](../spec/runtime-safety.md#caller-owned-finite-read)
uses the modern common finite kernel with borrowed prepared meaning/services,
lifetime-bound local authority, inline work and an exclusively borrowed result cell. Its
[downstream datagram fixture](../evidence/constrained-finite-execution.md) checks
bounded progress, first cause, stale references, late/invalid output and final
physical release without making the Host registry representation mandatory.
Existing Host finite execution uses that same transition kernel. That evidence
implements only NoSec Property Read; the separate continuing Action increment
below adds its own scoped execution claim. TD preparation, real Embedded
transports and automatic driving remain open.
This is a single architecture-risk correction, not a new portability programme
or a prerequisite to common Security composition and execution-semantics work.

The [caller-owned Action increment](../spec/runtime-safety.md#caller-owned-continuing-action)
now executes source-free NoSec Invoke with optional Query/Cancel through the
same continuing kernel as Host `ConsumerActions`. A concrete single slot borrows
meaning, services and original issuer authority while retaining native work,
input/result/token and original funding through settlement/disposal. It does not
wrap the Host registry or import its account bank, shared services or erasure.
Real datagrams and Host callback/I/O regressions test the same invariant
categories; no-atomic compilation remains separate evidence. Exact scope,
alternatives and remaining runtime-independence work belong to the
[evidence](../evidence/continuing-action-runtime-independence.md). Existing
notification/Producer ownership, TD integration, credentials and embedded driving
are not ported. No package, milestone or gate status changes.
The generic caller-owned output facet validates each exact application loan.
Owning a settlement cannot seal a later native view. Yield or invalid data
preserves original evidence and backing. Input inspection
lends the original owner's current view rather than an implicit dispatch receipt.
The existing runtime contract and output-loan regressions own these boundaries.

The execution-only Host slice in `servient/finite` separates the bounded logical
registry from typed native runtimes. Registry count/name limits belong to
`RawProducerRegistryLimits`; per-runtime input/result opportunities belong to
`RawFiniteLimits`. Revocation and complete same-slot replacement occur at the
logical owner. Accepted work retains the original instance through native
settlement and acknowledgement. Registry mutation/destruction has at most 32
entries and 16 behaviors per instance with names at most 256 bytes; linked
callbacks/local destructors remain trusted cooperative code, not preemptible.
Caller-provisioned runtime slots bound the native work and retirement they own. There is no aggregate process-RAM claim.

The optional [finite Producer capacity increment](../evidence/producer-finite-capacity.md)
separates persistent typed native participation from active work and retained
outcomes. A shared original work bank admits finite receives without prepaying
an idle input/result opportunity per publication. Native settlement returns
active credit; unread and caller-held outcomes retain their original bounded
quota/backing through disposal. The default single-opportunity policy remains
available. Complete qualification, independent publication/permission, original
native correlation, effect/delivery evidence and acknowledged final retirement
are unchanged. Real shared HTTP tests cover active/retained/account pressure,
replacement and bank/socket release; effectful HTTP Write adds normal/error/invalid
results and Pending cleanup. No native multi-exchange, Producer Host loop,
continuing-family migration or package/gate acceptance is claimed.

The former runtime-owned-instance constructor and runtime-local revocation are
replaced directly. Core settlement retains original Binding/target identity;
the Binding facet remains native and receives no registry/handler capability.
Affected checks are the finite `native_ingress` integration test (including its
`shared_ownership` cases), Core handler contracts, execution-only dependency and
feature checks, and authority/API projections. TCP/datagram test providers are
ownership/I/O evidence, not production protocol qualification. Historical
Property Read gates and PR #229 evidence keep their original revision scope;
independent acceptance of the changed API/ownership remains necessary. TD
qualification/publication, optional Planning, threading, Action/Event and
constrained execution are separate work.

## Prepared validation and time on shared finite ingress

`FiniteValidation::Required` refuses an absent actual provider input/output service at
admission and fences start/acceptance/delivery on service loss. Raw mode remains
explicit. Check the complete shape and backing before the codec callback.
Bound the lookup and body/media traversal at their actual owners. Invalid output becomes the retained exchange error before the native
reply. Input is checked before fresh authorization and handler effects using
that same original candidate. A service is part of its actual captured
configuration, not a support tag. The output-only finite API is replaced directly.

Progress takes one caller-qualified monotonic instant. The runtime retains the
original native request deadline across incomplete input, dispatch and pending
send; expiry, clock mismatch/regression and explicit close keep the first cause,
complete original exchange and accepted behavior until retirement/acknowledgement.
Expiry of a prepared reply can retire its work independently under the
[finite participation contract](../spec/runtime-safety.md#finite-producer-participation-and-work-capacity).
Passive observation can see a recorded fence without polling native work. Drop remains
finite local disposal, not a completed network drain.
Old finite callers migrate directly to the explicit time/validation arguments.

Timing errors follow the retained [time-domain authority](../amendments/WP-100-time-domain-v1.md).
Before the first funded native start invocation, invalid clock/deadline input is
Validation/Admission. That invocation admits native effects even when readiness
stays Pending. Later clock violations are InternalInvariant/Binding for native
request/progress timing, InternalInvariant/Handler for the caller's logical
deadline, and InternalInvariant/Cleanup while retiring. Incomparability is never
TimedOut. A later cleanup clock/error cannot replace the original cause or
release an accepted result before physical retirement and acknowledgement.

The HTTP `finite_producer` integration target now exercises this owner over real
HTTP, alongside the finite TCP/UDP ownership regressions. It includes required
schema checks, service loss, bounded funding, partial-write uncertainty, deadlines,
replacement and independent physical retirement. Caller-provisioned finite slots
still have no aggregate Host account admission. Required validation is not final
TD qualification; no publication certificate/facade or package completion is added.

## Captured validation service on the retained HTTP path

The older single-provider Producer root requires an actual output-validation
facet for Serving, beyond numeric response limits. Preparation and runtime
admission reject a missing facet. Funded Serving/dispatch/delivery progress
rechecks the captured provider/configuration and required service before permits,
behavior or native delivery; loss closes acceptance and retains first cause and
funded native cleanup. Installation-only providers omit the facet. Core checks
result shape/backing bounds before the external codec service.

This fixes a concrete prerequisite on the working HTTP path. It does not move
that path into the shared finite registry or implement final-TD qualification/
publication. Those contracts remain governed by SAFE-PUBLISH-001/PREP-COVER-001.

## Raw Producer Action invocation kernel

`RawProducerServient`/`RawProducerInstance` replace the finite-only registry
names directly; the finite native runtime remains a finite-only facet. The same
instance may have Property and Action behavior, or only Action behavior. Core
defines raw Action behavior, independent application continuation, selected
per-operation Binding mapping association and effect-uncertain result shape.
Before handler entry the Servient secures an invocation slot, retained result
and local retirement. It retains the original behavior, principal and monotonic
invocation identity. Bound input, output and lookup at their actual owners. Separate query/cancel routes may belong to other Bindings. Controls
recheck current instance, route, principal and policy; neither a copied reference
nor a native request correlation is execution permission.

Revocation/replacement fence new acceptance and request cooperative cancellation
of accepted work without redirecting it. Cancel and local abandon allow late
completion; a funded explicit close or owner retirement first delivers any queued
application cancellation once (retaining its error) before polling local close.
Explicit close retains failed/pending cleanup and reports local cancellation
without claiming remote terminal state. Bound input traversal and handle lookup,
including the synchronous callbacks they invoke. While the owner
lives, capacity, original behavior and result remain charged until settlement
acknowledgement. Host linked callbacks and local disposal are trusted/cooperative;
the owner is manually progressed and does not supply a scheduler or guarantee a
deadline for non-returning code.
Native initiating/control exchanges, opaque wire continuation, delivery and
physical resources remain Binding-owned and are not implemented by this kernel.

An Action handle is not the cleanup owner. The Servient can fence new Action
acceptance and drive bounded owner-level retirement after **every** handle is
lost; it returns each complete original settlement to the caller, keeps pending
and failed close obligations charged, and steps independent cleanup fairly.
Dropping the Host root without completing that explicit drain invokes each live
application continuation's required finite, nonblocking local abort, rather than
silently treating `poll_close` as completed. Linked applications unable to
guarantee that local abort are not qualified for this raw Host kernel. Drop does
not report a completed drain or settle any Binding-owned native obligation.

Deterministic `action_lifecycle` tests exercise distinct control routes, fresh
policy, old-instance late completion, saturation, invalid results, cancellation,
handle-loss retirement, failed cleanup, root local abort and Action-only admission.
This slice is explicitly raw: no TD
publication/`synchronous` interpretation, schema-validated Action payloads,
native Action Binding, notification conversation, constrained execution or
aggregate Host resource qualification is claimed.

## Raw Consumer notification conversation

`core::notification` separates subscription input, raw Property/Event data,
Event dataResponse opportunity, cancellation input and native delivery/retirement.
`servient-finite::notification::RawNotificationServient` retains the independent
Consumer authority reference and a captured Binding/configuration, with independently admitted logical slots,
non-cloneable cursors, fresh application permission, stop intent and retained
cleanup. The existing finite and compiled Property Read adapters are not reused:
neither owns the required long-lived control/response lifetime. Per-logical native
participation may be a lease on shared infrastructure; no Engine distribution,
native session manager or new protocol integration is introduced.

`RawNotificationLimits` bounds root/queue storage, retained payload backing,
controls and target names. Retain backing for an extra late handoff and the
promised response when the ordinary queue is full. Native participation and
reply backing stay at the Binding. Each queue or native operation has a bound. Stop fences delivery before mapped
cancellation, including unsupported/denied cancellation. Pending/error retains
original data, reply jobs, first cause and capacity; bounded owner cleanup can
return complete terminal reports after all cursors are lost. Host-root Drop uses
the native owners' required finite local abort and reports no completed drain.

Public notification delivery now lends Core `PayloadView` and `AffordanceSite`
slices directly, replacing payload/name allocation-owner borrows. Compile-fail
cases reject original-owner extraction, escaping slice lifetimes and advance
while a delivery loan remains live. The
[raw occurrence regression](../../servient/finite/tests/notification.rs) tracks
exact payload/name allocations through pending reply retirement and final release;
the [Mosquitto regression](../../protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs)
repeats delivery/advance without retaining original response allocations or growing
original charges, then verifies mapped unsubscribe and complete account release.
Downstream reciprocal-response tests observe allocation liveness at the trusted
native boundary while using only slices at the public handoff. These corrections
preserve native ownership, validation, controls, limits and cleanup rules.

`cargo test --locked -p clinkz-wot-servient-finite --test notification` exercises actual
Core/Servient lifecycle code through deterministic external in-memory Binding
capabilities: compatible sharing, independent stop/cursors, tenant separation,
late/stale delivery, item/byte/native loss, reciprocal responses and cardinality,
policy/configuration changes, saturation, failed cleanup and root disposal.
This is raw Host lifecycle evidence, not real protocol I/O, TD/schema/security
qualification, fleet scalability, hardware evidence or package/gate completion.

## Joined Consumer execution

The implemented [operation/instance composition](WP-200-planning.md#consumer-operation-composition)
is a foothold for [the roadmap's broader capability objective](../../PLAN.md#near-term-capability-growth-and-priority).
Finite and raw notification owners now share a separate move-only Core
`ConsumerInstance` authority through passive retained references. Prepared
definitions and native resources contain no
such permission. Equal numeric slots do not identify the same incarnation.

Finite execution supports Property Read/Write and explicitly synchronous Actions with
separate input/output checks and operation-aware instance authorization. Callback
work, input/result backing, unread/late settlement and cleanup remain funded by
the original bank. Closing a finite facet or stopping a subscription is local;
finite settlements expose conservative remote-effect uncertainty after native
progress, independently of local cancellation and result delivery. There is no
implicit replay. Revoking/dropping the Consumer authority fences every facet,
even after callbacks or with no native progress credit. Replacement cannot
retarget old work.

The [HTTP Write integration](../../protocol-bindings/protocols/http/tests/property_write.rs)
joins the existing open finite Consumer and composed Producer paths over real
GET/PUT I/O. It exercises independent Read/Write definitions on one Property,
pre-effect refusal, empty terminal completion, ambiguous replies, handler failure
after effects, mutable policy/service loss, cancellation/deadline/replacement,
late original results, unread admission pressure and complete final release.
The downstream Consumer fixture also executes Write using its own non-HTTP
completion convention, including a late native success after local cancellation.

The notification executor and its existing Event/overflow/response/cleanup tests
move into the execution-only crate, without an old-path re-export. Its independent
raw lifecycle remains distinct from finite calls. A downstream test-only native
observation Binding exercises shared socket ownership, real delivery, sibling
retention, callback revocation and final release alongside the finite owner.
Raw notification limits remain owner-scoped: this does not claim TD/schema
qualification or shared aggregate Host-bank accounting for notification queues.

`ConsumerNotificationAssociation` now retains a shared typed TD-selected mapping,
original Consumer reference, retained access owner and instance bank without
vacant conversation capacity. `ConsumerNotifications` supplies independent work
capacity around the raw lifecycle, with original contexts retained per entry.
The [Runtime contract](../spec/runtime-safety.md#notification-associations-and-conversation-capacity)
owns admission, declaration fencing, occurrence policy, report ownership and cleanup.
The [composition evidence](../evidence/consumer-notification-composition.md)
exercises one compilation/eight associations/two slots/one real MQTT interest,
original service funding, independent queued policy/stop/revocation, saturation
and final release. Occurrence-specific refusals now retire only the original item;
subscription Maintain/authority errors remain separate. Native and queued-policy
counterexamples deliver a later permitted Event on the same conversation, and
reply-owner tests cover refused required responses with bounded pending/error
retirement. Existing Producer-to-Consumer, loss and late-work evidence is retained. The downstream reciprocal-response fixture checks original occurrence
policy at delayed send and complete reply retirement; MQTT still refuses that
response capability. Constrained notifications and authenticated source provenance
remain unimplemented. Optional aggregate progress is scoped below; no package,
architecture gate or milestone is closed.

The separate `consumer_action::ConsumerActionAssociation` and `ConsumerActions`
replace the former per-Consumer Runtime. Persistent associations retain reusable
TD-selected definitions and explicitly bound per-operation access; independently
funded capacity admits invocations from many Consumers. Each invocation retains
its original identity, association, mappings and services after each exchange
ends. Explicit query/cancel uses the original association and current authority;
cancel acknowledgement is not terminal completion. First local cause, remote
cancellation status, effect uncertainty, native result, validation and cleanup
errors remain distinct. Maximum native/input/result/continuation/scratch and
cleanup backing is funded before invocation. Pending/error retirement and unread
settlement keep original owners; stale handles cannot control reused slots.
The HTTP integration covers real acceptance, pending/completion, cancel, service/
policy loss, deadlines, both role replacements, resource pressure and final
release. The downstream datagram fixture covers different actual Binding types,
zero-credit progress and blocked/error cleanup with a retained late invalid result.
The original synchronous Consumer path remains separately exercised.
The [Action composition evidence](../evidence/consumer-action-composition.md)
adds shared physical service funding, original prepared-declaration refusal,
one actual definition/three associations/two independent capacities over real HTTP,
original instance quotas through settlement disposal and independent access.
Complete qualification remains at association/admission; live invalidation follows
original Invoke native/authority obligations and the operation owning an exchange/unread result.
Independent unused controls and refusals before native start leave unrelated
accepted work usable; relevant admitted failures retain original late cleanup.
Shared Action output meaning is checked with the returning operation's captured
codec, media and body/scratch/work policy. Retired Invoke input validation does
not govern Query output. Real HTTP contrasts independent body allowances;
downstream datagrams contrast JSON/plain-text media, output-only Query support,
exact selected validation work and changed-declaration refusal during late cleanup.
The root QueryAllActions fixture shares one Consumer authority across separate
finite and continuing capacity. Managed mixed Consumer execution, reusable
Producer exposure composition and Host Producer turns have their separate scoped
sections below; broader profile qualification remains open.

Unread native completion remains revocable until a paid settlement handoff with
fresh time, original authority and captured service/access checks. The HTTP
Consumer regressions cover direct handoff and additional progress after
revocation, denial, service/root loss, cancellation or expiry, plus insufficient
handoff work and callback-time service loss. The earlier HTTP Query fixture
validates a late output during retirement and records a validation failure
separately from the poll error and bytes. WP-100's successor case releases
the native slot with the codec unavailable and validates only on a requested
handoff. [Bounded execution and TD admission](WP-100-core.md#bounded-execution-and-td-admission)

The continuing owner now isolates linked execution from registry borrows under
[SAFE-CONCUR-001](../spec/runtime-safety.md). Its
[public callback regressions](../../servient/finite/tests/consumer_action_callbacks.rs)
reproduce admission/control, native poll/retirement, validation, settlement and
destruction borrow failures. They cover occupied-slot reentry, independent
invocation identity, cancellation and service-loss fencing, nested time,
synchronous native/cancellation wakes, unwind evidence retention, short work,
original funding and final disposal. The actual HTTP Action integration adds
reentrant access, validator and native poll/retirement callbacks with retained
fenced completion; existing HTTP and independent datagram lifecycle evidence
remains applicable. The
[investigation](../../workspace/0111-continuing-action-callback-ownership.md)
distinguishes reproduced defects from already-correct replacement behavior.
These changes preserve the continuing association and its explicit described
controls; no finite-call substitution or package/gate acceptance is implied.

## Host Consumer execution composition

The [borrowed turn contract](../spec/runtime-safety.md#host-consumer-execution-turns)
adds aggregate active/cleanup notification progress and an optional common turn
over existing finite, continuing Action and notification capacities. It creates
no work owner, scheduler or Thing-to-capacity relationship. Live cursors retain
their terminal reports; aggregate driving reclaims abandoned cursors only after
original retirement. The manual per-conversation/report cleanup path remains
available when reports are required after cursor loss.

The [mixed HTTP/MQTT evidence](../evidence/consumer-execution-composition.md)
uses managed TD-ID registration, separate application permission services,
shared prepared definitions and independent capacity. Deployment-supplied native
wakes and a timer drive one Host task's admitted work/results and cleanup,
including silent deadlines, passive replacement/owner loss and abandonment.
Original unread results, physical sibling ownership and final release are checked;
existing callback/late-result and F1/F2/F3 evidence stays applicable. The compiled
example exposes the same integration. No independent acceptance, Producer reuse,
mixed authentication/source verification, constrained continuing implementation,
production qualification or package/milestone closure is claimed.

## Host Producer execution composition

The [Producer turn contract](../spec/runtime-safety.md#host-producer-execution-turns)
replaces storage-only participant erasure with the original native member and
its typed Servient turn. Existing finite, Action and notification capabilities
retain independent authority, preparation, native ownership and settlement.
Native Action ingress and existing drains can progress independently. A stalled
participant cannot starve a ready sibling. Marking revoked work and selecting
an original Action can share a bounded scan, retaining its index through close. Manual and aggregate progress share
that algorithm while keeping their distinct local-close policies. Publication, emissions, result
observation/acknowledgement and complete-set retirement remain explicit.

The [real Host integration](../evidence/host-execution-integration.md) combines
independently qualified publications, actual HTTP definition reuse and a shared
native listener/work capacity with Engine Consumer Property/Action/Event work
through HTTP and Mosquitto. Replacement, original Pending/error cleanup,
caller-held outcomes and final physical/account release are exercised. Native
startup failure and typed-loan substitution additionally test original membership
and sibling progress. Tight-budget HTTP cases reproduce the former duplicate-scan
stall and check exact scan/callback allowances, Pending/error close, normal late
completion, described cancellation, fair siblings and final original-owner release.
Existing qualification/native-authority/resource evidence
is retained. This does not close the work package or qualify production/security,
shared continuing-family capacity, constrained execution or broader reuse.

## Open finite Consumer ownership

The optional bound Property Read is a funded selected-operation handle over the
same finite call owner. [Runtime Safety](../spec/runtime-safety.md) owns its
authority, destination and release contract.
The destination now shares immutable bytes and original physical funding with
native requests and settlements. Core's `outbound_destination` regression checks
final-owner funding; the callback fixture checks identical pointers through
resolution, permission, native construction and retained results. Required SPI
revision adoption is checked by compile-fail cases and the mutable-support
counterexamples in `qualification_adoption.rs`.
`cargo test --locked -p clinkz-wot-binding-http --test consumer_callbacks --test uri_variables`
includes fixed/variable real HTTP, indexed and bound lookup, support/resolution
counters, original revision/bound changes, callback revocation, binding refusal,
late/unread ownership and final account release. The external
`cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --test integration`
also executes bound reads over independent datagrams and HTTP after source and
compiler disposal. Existing security, selected-read, replacement and reuse
regressions remain applicable; no gate/profile/milestone acceptance follows.

The source-free `servient-finite::consumer` owner consumes Core instance bindings and
pins their exact actual execution/configuration and services. It accepts no
call-time executor, codec, validator, security or configuration override. Managed
association construction refuses bindings from a foreign instance bank. Finite
admission accepts separately funded capacity, while retaining the original
native/services and complete result/cleanup obligations.

`ConsumerAssociation` and `ConsumerCalls` replace the bundled `ConsumerRuntime`
constructor directly. Associations retain no vacant Call Slots; an explicitly
chosen work owner can serve many instances, and each association can use several
owners. The existing `FiniteCallState`, reserved-slot checkout, independent
controls, raw late evidence and finite native disposal preserve the original call. Original
binding wrapper backing now survives selected settlement retention independently
of the association's unselected mappings. Exact scopes and account ownership are
specified in [Runtime Safety](../spec/runtime-safety.md) and
[Foundation](../spec/foundation.md).

The real HTTP [independent-capacity regressions](../../protocol-bindings/protocols/http/tests/support/independent_capacity.rs)
run with `cargo test --locked -p clinkz-wot-binding-http --test uri_variables`.
They exercise eight independent associations over one exact qualified mapping
and one slot, two permitted destinations, independent work/instance quota refusal,
retained input/variables, fixed-target refusal, reentry, late results across root
loss/replacement and abandoned real-socket disposal. Existing HTTP callback,
structured-data, reuse and downstream finite/Action/notification regressions
remain applicable after direct caller migration.

The [managed HTTP regressions](../../protocol-bindings/protocols/http/tests/support/managed_capacity.rs)
extend that command with non-revoking handle loss, explicit facet/instance
removal and replacement, owner shutdown, silent-peer retention until the next maintenance
turn, and abandoned A with active or settled B in both loss orders.
`ConsumerOwner` creates unique instance authority and consumes named standalone association authority, then
returns non-revoking handles/views. Registration metadata and call capacity have
separate limits and funding. Original identity/tombstone funding survives stale
handles; selected native/mapping/result ownership survives accepted work.
The [callback fixture](../../protocol-bindings/protocols/http/tests/consumer_callbacks.rs)
also exercises explicit managed removal during credential issuance, original
input recovery, shutdown-before-reentrant-service-destruction, and facade loss
during native checkout. No source/driver registry, eventfd or automatic
authority-change notification is retained. The application supplies lifecycle
progress, clock observations and actual timer wakeups where required.

The same managed HTTP fixture covers TD-ID-addressed registration/read/removal
after every Thing view is dropped, independent named configurations under one
identity, configuration update with retained original context, and same-ID
recreation while old work/results remain live. Source-aware ID ingress lives in
`servient::consumer`; the source-free owner performs lookup and management in
its actual registry. Anonymous TD parsing/preparation remains valid; this named
ingress requires an ID. Duplicate live IDs refuse, missing configurations never
fall back, and snapshot-conditioned management stays distinct from a command
targeting the current ID. [Management tests](../../servient/finite/tests/consumer_lifecycle.rs)
cover recovery, duplicate scopes, refusal and final authority backing.

The registry uses individually funded AVL nodes and private reusable slot
tickets; no operation shifts a population-sized array. Tests cover a million-entry
configured limit without eager allocation, 4096 long-prefix IDs, and 6000 mixed
public lifecycle workload steps at each of 64 and 4096 initial instances with the same
4096 QueueOperations allowance. A separate map model checks original identities,
refusal, removal/recreation and replacement. Private tree tests inspect ordering,
balance, stored heights and counts throughout 20,000 mixed mutations.

The managed HTTP fixture exercises indexed admission and repeated
removal/recreation and configuration changes among 2048 Things while original
calls/results retain identity, sockets, quotas and cleanup. Refusal sweeps verify
partial node-acquisition refunds and preservation of a reusable ticket. Reentrant
service destruction and independent work-owner cleanup remain exercised. These
are focused correctness/complexity claims, not production latency qualification.

This is bounded Host finite capability with caller-driven execution,
not continuing Action, notification or Producer association/capacity
implementation. HTTP's one-exchange-per-native-
registration restriction remains. F1/F2/F3 are unchanged outside the affected
finite path; no gate, package, milestone or profile acceptance is inferred.

The [reusable execution fixture](../../protocol-bindings/protocols/http/tests/reusable_execution.rs)
executes independent HTTP Consumers from one semantic preparation, specialized
endpoint artifacts and independently bound policy/authority/accounts. It covers
incompatible meaning/configuration, actual changed paths/headers, stale authority,
changed service support/bounds, refusal under saturation, retained late/cancelled
results and final physical release. Its allocator characterization compares shared
and separately prepared content over 1–32 instances; measured costs and limits
are recorded in [the evidence](../evidence/consumer-execution-reuse.md). This does
not close a package/milestone or qualify Producer reuse, broader schemas/security
or portable execution.

The [multi-instance characterization](../evidence/consumer-multi-instance-scaling.md)
extends this finite Host path to increasing populations and active fractions,
with independent policy/accounts, explicit native lanes, distinct targets,
allocator/reservation separation and final retirement evidence. It records the
prior bundled runtime layout and its dense account/slot costs
and preparation limits; no package or profile qualification is implied.

Calls and abandoned-handle cleanup retain their original native owner and first
cause through cancellation errors/Pending, policy/service loss, revocation and
late completion. Each native or cleanup poll has a bounded operation and owner. A successful raw
result is validated by the captured codec/type service before logical success;
invalid output remains retained separately until settlement disposal. The
settlement's `result()` is the application outcome; `native_outcome()` preserves
original late evidence and cannot override the first cause. Revocation also
fences terminal results still unread at the owner. A transfer request without a
granted named owner is retained as refused evidence while the same original call
continues manual cleanup; it never becomes a fabricated transfer acknowledgement
or terminal release. A returned settlement retains
its native/result/definition/account owners through its final disposal.
All normal, rejected and late/native payload access lends Core slice-only output
views. `ConsumerNativeOutcome` separately preserves returned errors or borrowed
cancellation retry/cleanup evidence. Affordance access lends its original name.
No accessor exposes a cloneable original output/body allocation. The
[HTTP ownership regressions](../../protocol-bindings/protocols/http/tests/query_all_actions.rs)
retain the exact original response across call/runtime loss and shared settlement
owners, including invalid output and cancellation/revocation before handoff;
only final owner disposal returns the original response and cleanup capacity.

Finite facet loss closes its own acceptance. Consumer authority loss revokes the
incarnation across all attached facets. Existing handles can finish original cleanup;
lost handles can be drained by root progress. Final Drop uses the captured
ConsumerOwnedHostV2 finite nonblocking local disposal contract, with no assertion
of remote cancellation or completed network drain. The bank and original leases
are shared with retained legacy Consumer/Producer owners, whose existing resource
and transfer regressions remain applicable. Notifications are not finite calls.

Finite Consumer registry access now excludes linked callbacks and destruction.
Admission reserves the original slot before callbacks; progress checks out its
complete call while passive close/cancel/abandonment remains accessible. Nested
entry cannot reuse an occupied slot or poll checked-out work. Current controls
are checked after callbacks before effects/handoff; late native evidence keeps
its original funding. The control allocation has its own original-bank lease
through the final reference, including a call handle retained after settlement.
[Callback regressions](../../protocol-bindings/protocols/http/tests/consumer_callbacks.rs)
reproduce the prior public-API borrow panic and exercise nested capacity,
sibling progress, close/cancel, first cause, abandonment, wakers, unwind
restoration, advancing-time nested progress/admission, genuine clock faults,
pending-call driver wake retention and actual HTTP result retention/physical
retirement. Time observations are recorded at API entry and remain accessible
while a slot is checked out; resumed progress and deadline fences use the latest
observation. Both direct polls and runtime drivers register wake interest on busy
slots and request a restoration retry. Their executed
revision and validation results belong to the PR; no broad package acceptance
is inferred. The separate continuing Action correction is evidenced above.

## First joined finite Producer

The optional `producer::ProducerServient<C>` captures one frozen actual
`FiniteProducerCompiler` at construction. `prepare` captures one behavior and
unique authority, qualifies the entire exact description, reserves Engine
ownership and stages `ProducerPublication` before native acquisition. It creates
`ProducerExposure<B>` over the existing source-free runtime; it never constructs
or wraps `ProducerInstallationRuntime`. Legacy tests retain their original
independent scope. [0092](../../workspace/0092-joined-producer-publication.md)
records the comparison of available participants and rejected alternatives.

The first paid step may acquire native resources and yields InstalledClosed.
A subsequent step requalifies the entire final TD against the same actual
provider, behavior coordinate, configuration, services and native endpoint/
readiness facts. The source-free `publish` first checks issuer/current slot and
captured runtime pairing. Its non-fallible local commit then installs the
preallocated description/behavior/authority references and opens that runtime.
It calls no native/service/application code, allocates nothing and destroys no
displaced behavior inside the commit. No remote/global visibility atomicity is
promised. Explicit endpoint meaning is retained; endpoint rewriting is not
implemented in this slice.

The publication guard and registry refer to the same logical permission cell.
Cancellation before publication prevents it even at zero work credit; cancellation,
native failure, deadline and lost guards fence this incarnation. A replacement
cannot redirect old accepted work. Explicit close retains native work and the
complete first cause/result/delivery through Pending/error; settlement must be
acknowledged before `retire` removes its logical record. Native-only cleanup
can continue after root loss. Drop uses the concrete Binding's finite local
abort and reports no completed remote drain. All dispatch still occurs in the
single source-free Servient registry, with fresh policy/current-instance checks.

The earlier single-Read facade reserved a dedicated opportunity and worst-case
finalization scratch at its bank before setup. Replace those estimates in the
public successor. Keep the partial-setup rollback, exact final TD, late result
and socket-release cases. JSON input size and native SDK limits stay with their
respective owners.

`joined_producer` exercises the public facade over real HTTP peers and the public
Consumer using the exact published TD: complete coverage refusals, anonymous/
empty-key and alias positives, closed traffic, repeated reads, fresh policy,
capacity pressure, final dependency/support loss, cancellation, deadline,
replacement, unread/late result and delivery, partial-write uncertainty, failed/
pending cleanup, root loss, final socket release and original-bank release.
Existing finite TCP/datagram, Action and notification regressions remain scoped
independent evidence. This is one Host/manual/synchronous read mapping and one
logical slot/required participant. It establishes no multi-participant atomicity,
async behavior, Action/Event/observation publication, HTTPS/authentication,
hardware/profile qualification or package/gate/milestone completion.

## Owned composed Host Producer

Property values and Event data use the same instance registry and closed
whole-TD publication with explicit `ProducerNotificationSource` families and
captured notification mappings/services. `publish_notification` checks the
family-qualified target and `PublicationData` before native acceptance. Native
publication work retains original authority/capacity, first cause, effect,
cleanup and unread settlement through acknowledgement; outstanding work blocks
logical retirement. This replaces the Event-only publication assumption without
adding a second observation kernel or a session abstraction.
The [MQTT Property tests](../../protocol-bindings/protocols/mqtt/tests/property_observation.rs)
join qualified Producer values to TD-selected Consumers, including same-named
Property/Event separation, cancellation, replacement, resource retention and
physical release. Existing Event regressions reaffirm their distinct semantics.
This is a bounded Host subset, not general family or production qualification.

`producer_composition::ComposedProducerExposure` takes a complete pure
`ComposedProducerInput`: actual staged instance/behavior/description, prepared
definitions, the deployment's eligible registration set and independently
typed native participation owners. Refusal returns those same input owners. This is
an ownership handoff after caller construction/preparation, before the managed
participants acquire native work; it does not reuse the single-Property facade.

The [reusable Producer increment](../evidence/producer-preparation-reuse.md)
retains a distinct `PreparedThing` and complete publication for every instance,
while actual HTTP mappings reference explicitly funded shared instructions.
`ComposedProducerExposure` needs no shared-Thing wrapper or new runtime:
[the ownership contract](../spec/runtime-safety.md#reusable-producer-definition-ownership)
keeps physical definition funding independent of native membership, final
qualification, behavior/authority and instance accounts. A conflicting staged
publication and one sibling's unread result/retirement cannot revoke another
publication or refund its original shared definition.

The manual Host supports 1..=32 native members and an independent eligible set
of 1..=32 capability registrations. Managed admission checks the exact
final TD and complete behavior set with the actual providers, including every
negative match. Closed setup then checks the actual endpoint, mapping/service
identity, native readiness and expected incumbent. Source-free installation
pins the observed registration, provider and native facts through the local
decision. A stale candidate retires its original members and leaves the
incumbent current. Each effective operation has exactly one native owner.
Before setup, `PublicationPermission` reserves one vector of weak member
identities and one of weak ingress gates sized to the managed candidate's
actual membership. Admission records the original
identities. Commit records the gates in their reserved storage and shares the
permission with the registry. The publication guard and accepted work retain
that same permission. Retirement checks the complete original set and rejects
changed, duplicate, omitted or foreign members. The local commit opens all
prechecked gates without allocation, native progress or fallible preparation.

The mapping-capacity fixture qualifies 33 Event Forms with 66 effective
operations. One MQTT member delivers the final Event to its subscribed peer
while a separate HTTP exchange occupies the finite pool. Reducing the route
bank to 32 refuses the unchanged TD. Splitting the same Forms across two
required members refuses at a one-member bank with all 33 mappings funded.
Both refusals return the original closed owners. The fixture also exercises a
same-topic collision across members, replacement of an 86-route incumbent,
an 86-Event native close over two Gateway cleanup turns, retirement of an
11-member publication, and last-settlement disposal after loss of the facade.
[Broker and HTTP fixture](../../protocol-bindings/protocols/mqtt/tests/support/notification_capacity.rs)

Cleanup evidence includes a full two-entry registry with four-item turns, 32
orphaned members beside 128 unrelated Actions, the original source table
held by a displaced unread Event, and Servient-first disposal of one, 33 and
86 captured rows after exact lifetime funding. A serving incumbent with 257
mappings refuses a 256-item successor before setup. Its returned successor can retry
with a 257-item policy or after the incumbent closes and drains one row.
[Retirement fixtures](../../protocol-bindings/protocols/mqtt/tests/support/notification_capacity/retirement.rs)

The earlier composed path tested contention on its account bank. For ordinary
staging, replace the conservative reservation with actual candidate and
qualification storage. Keep the selected definitions and all relevant support
answers current until the local commit. On refusal or cancellation release
provisional backing and retire exactly the original closed members. Carry
the serving owners into the publication. An unread result survives its loss.

Typed participant loans do not permit substitution to escape accounting: an
extracted native owner retains its original funding, and a replacement object's
different identity cannot publish or retire the original set. Explicit close
requests bounded local participation retirement; concrete native progress stays
caller-driven. Pending/error cleanup and unread finite or Action settlements
retain original identity, first cause and capacity. Only acknowledged complete
retirement removes a logical record. Actual exposure/native disposal releases
their surviving layout charges. Lost exposure Drop revokes and performs each
actual member's finite local disposal, reporting no completed drain. Funded
`retire_revoked_publication` can reclaim a lost-guard registry record only after
all original native owner identities have disappeared and original Actions have
been acknowledged; a surviving shared native service may continue for siblings.

The raw logical registry/Action-store baseline and immutable startup registration
backing are explicitly caller-provisioned. Application continuation limits remain
the raw Action owner's existing independent contract; native SDK/OS/shared
infrastructure remains Binding/deployment-owned. The new reservations bound
this handoff's controlled ownership, not total process RAM, allocator recovery,
all legacy flat resource fields, production Action/Event mappings, constrained
profiles, hardware or package/milestone/release acceptance.

`cargo test --locked --offline -p clinkz-wot-binding-http --test composed_thing`
includes real two-listener HTTP and shared test-only UDP Action cases for exact
membership, aggregate-bank saturation, scratch release, replacement/late result,
unread acknowledgement, lost exposure/root, extracted native owners, partial
setup failure, pending/failed cleanup and final socket/account release.
The [coherence cases](../../protocol-bindings/protocols/http/tests/composed_thing/coherent.rs)
exercise a competitor moving between two Forms, replacement of the eligible set
on a staged successor, and provider or native changes in the last gate callback.
The [HTTPS tests](../../protocol-bindings/protocols/http/tests/security_https.rs)
also change and restore a real verifier's declared work after closed install.
Large HTTP definition wrappers also exercise 24 MiB of original buffers through
admission and real publication without regenerating them; declared callback
temporaries are checked at actual bank limits before allocation, and direct
typed-owner retirement releases scratch while retaining lifetime/cleanup backing.

## Executable root collection read

The existing finite Consumer and composed Producer owners now execute
ReadAllProperties with independent root identity, a registered root behavior,
captured combined validation, original authority and funded retirement. No
individual Property dispatch or universal collection lifecycle is introduced.
The real [Engine-to-Engine tests](../../protocol-bindings/protocols/http/tests/read_all_properties.rs)
exercise single native request, complete publication, member refusal, replacement,
cancellation, service loss, saturation and final account/socket release.

ReadMultipleProperties retains a separate per-call selection with the original
finite definition and authority. Encoding starts only after its complete payload,
scratch and cleanup funding; native refusal drops encoded input before release.
The [selected-read tests](../../protocol-bindings/protocols/http/tests/read_multiple_properties.rs)
exercise different subsets from one TD, unsupported/malformed/partial results,
authorization, prepublication, service loss, cancellation/replacement, failed
sibling admission, unpaid work and final account/socket release.

WriteAllProperties uses one distinct root behavior after complete input
validation and group authorization. Reported completion, possible partial effects,
native delivery and local handoff remain separate; uncertain writes never replay.
The [collection-write tests](../../protocol-bindings/protocols/http/tests/write_all_properties.rs)
exercise real partial application, authority replacement, unread completion,
cancellation, retained cleanup and final release under the existing finite owners.

WriteMultipleProperties retains a separate validated set for each original call,
uses set-aware Consumer permission and one Producer root behavior, and performs
no implicit member fan-out. Valid incomplete reports remain native evidence with
Reported effect while whole-group `result()` fails. Local cause, late/unread
reports, delivery and physical retirement remain separate. The
[selected-write tests](../../protocol-bindings/protocols/http/tests/write_multiple_properties.rs)
and independent datagram fixture cover real effects, original authority and
final account/socket release under the existing finite owners.

## Root Property notification ownership

ObserveAllProperties reuses the notification owner with one root identity and
complete-map validation. Producer publication requires a distinct root source,
kind and captured route/services. [Interaction Core](../spec/interaction-core.md)
and [Runtime Safety](../spec/runtime-safety.md) own the semantics. The
[real broker tests](../../protocol-bindings/protocols/mqtt/tests/observe_all_properties.rs)
exercise invalid members, fresh permission, short validation work, overflow,
replacement, late work, independent stop and original-bank/final socket release.

Event collections use one root owner while retaining the occurrence's exact
Event target through validation, queueing, borrowed exposure and reply retirement.
[Broker evidence](../../protocol-bindings/protocols/mqtt/tests/subscribe_all_events.rs)
adds heterogeneous member schemas, unknown identity and cross-schema refusal,
independent cursors over one native filter, pre-start stop and physical release.
The [downstream response fixture](../../tools/architecture-fixtures/open-consumer/tests/notification.rs)
checks per-Event response schemas and original retained native reply owners;
it is not a production protocol or MQTT response claim.

This scoped capability changes no work-package dependencies, gate, milestone or production qualification
status.

## Root Action status ownership

QueryAllActions uses one finite Consumer owner and one root Producer observation
over original retained invocation owners. [Core](../spec/interaction-core.md) owns
the semantic distinction and root/member permission; [Runtime Safety](../spec/runtime-safety.md)
owns paid callbacks, replacement, settlement and final release. The
[HTTP tests](../../protocol-bindings/protocols/http/tests/query_all_actions.rs) cover
concurrent same-definition invocations, cancellation uncertainty, invalid output,
root/member denial, original principal and route, short credit, late replies,
replacement, native loss, blocked retirement and original bank/socket disposal.
They also falsified the old acknowledgement path: `RawActionSettlement` now
retains its original instance/funding until result disposal. Existing raw and
continuing Action evidence must be rerun for that direct ownership correction.
No dependency, milestone, gate or production status changes.


The result-ownership review corrections use slice-only public query/settlement
views and a per-admission result lease. Live and acknowledged-but-retained results
share the original store's bounded slots. Before behavior, admission must
secure the actual outcome storage and retirement path. The [HTTP regressions](../../protocol-bindings/protocols/http/tests/query_all_actions.rs)
retain the exact allocation across shared settlement/root loss, exhaust retained
result slots, reuse only disposed capacity and verify rollback for each exhausted
account. Raw lifecycle tests likewise require disposal before slot reuse.
The exact ownership contract is [Runtime Safety](../spec/runtime-safety.md).

The [mixed collection regression](../../protocol-bindings/protocols/mqtt/tests/collection_coexistence.rs)
qualifies one Thing with both root families and same-named Action/Event members.
Its real HTTP/Mosquitto case retains independent Consumer facets under one
authority, preserves the Action inventory after mapped Event stop, and verifies
original Producer/Consumer result funding after native and logical root disposal.
The combined contracts preserve existing limits, native owners and result leases.

## Executable reusable structured data

The bounded [prepared DataSchema contract](../spec/interaction-core.md) now
covers nested object properties/required names, integer boundaries and Unicode
string lengths for the real HTTP finite slice. Core owns the alloc-only
constraint model/evaluator; TD supplies common facts, Planning owns pure
construction/borrowed compatibility, and codecs lend actual encoded values.
TypeSchema is replaced directly; data-service schema inputs borrow PreparedSchema.
Property Read/Write and ReadAllProperties demonstrate constraint reuse across
independent Consumers, while fully qualified Producers validate input before
behavior and declared output before delivery. Dynamic storage/destruction and
actual validator qualification remain funded. Whole-TD normalization reuse,
native retargeting, richer schema assertions, non-JSON interoperability,
continuing-family reuse and a portable Engine runtime remain unproven. Detailed
executable scope is in [structured schema evidence](../evidence/structured-dataschema.md).
No package, architecture gate or milestone acceptance is claimed.

Continuing HTTP Actions also execute this structured input/output subset using
their existing independent invocation store and native control exchanges. Consumer
input, completed/late output and Producer input/output use captured validators;
schema backing survives normal/cancelled/revoked original work and settlements.
This does not establish cross-instance continuing definition reuse or broaden
notification and root status profiles. The structured-data evidence records the
specific runtime tests; no common interaction state machine is introduced.
Qualify the Action validator for both input and immediate or continuing output.
Changed services fence validated delivery. The original close poll can retire
work without calling a handler for new effects or entering the codec.

## Finite TD Security / HTTPS increment

The selected Basic/header Property Read/Write capability and its boundary corrections,
qualification, negative network evidence, ownership and remaining unsupported
roles/profiles are recorded in [the security evidence](../evidence/td-security-https.md).
The applicable preparation, Core and Binding contracts remain authoritative;
this increment grants no broad scheme/composition or production-profile acceptance.

## Structured Notification execution

Notification validation now retains the original funded definition through the
existing logical owner, without copying structured role trees. Original queues,
reply opportunities, first causes, independent stop and cleanup remain in the
same runtime. [Structured Notification evidence](../evidence/structured-dataschema.md#notification-execution)
covers real MQTT leaf execution and external structured controls/responses;
cross-instance reusable definition binding remains separate work.


## Security composition correction

The existing EXEC-SECURITY-001 / PREP-SELECT/COVER/REUSE-001 scope now replaces
singleton requirements with bounded effective composition, exact Consumer branch
selection and complete Producer security qualification. HTTP finite Read/Write
executes independently paired Basic/API-key services, conjunctions and distinguishable
alternatives. Domain specifications own meaning and the Binding owns placement,
identity composition and operation limits. [Scoped evidence and superseded
assumptions](../evidence/security-composition.md) identify the supported subset,
revocation/ownership obligations and remaining work. No package/milestone,
production-security or additional Embedded acceptance is claimed.

## Dynamic Property calls

Finite calls retain actual variable values/destinations and their original
authority, permission checks, resources and native settlement owners. Refused
Writes return their input; cancellation and late outcomes preserve independent
effect certainty. [Real-I/O evidence](../evidence/uri-variables.md) records this
scope without changing dependencies or package/gate/milestone status.

## TD-described finite responses

Finite runtime application-failure delivery preserves original authority, first cause, resource owners and separate effect certainty; see [response evidence](../evidence/td-responses.md).
No package, gate or milestone acceptance follows from this increment.

## Producer Action semantic execution ownership

The existing bounded Action store additionally retains each invocation's exact
native participation association and terminal-retention metadata. Servient's
`step_actions` accepts Core candidates, performs fresh authorization and captured
validation, dispatches behavior and lends semantic replies to the original native
encoder. `progress_actions`, `acknowledge_action` and `reap_actions` keep logical
continuation/settlement decisions here. Native references cannot recover local management handles
in another participation; described controls/root inventories may qualify distinct
participants under the original instance/route/principal checks. Definitions retain no live invocation/native owner.

`reconcile_participation` and notification-specific driving consume native failure
facts and decide logical revocation without giving that authority back to a
Binding. Managed composition funds its logical notification publication metadata
separately from declared native storage/alignment and publication slots. Failure
before publication cannot admit application work; native close after acceptance
cannot release its original continuation/result. Complete retirement still waits
for both original native obligations and Action/result ownership.

[HTTP Producer tests](../../protocol-bindings/protocols/http/tests/action_producer.rs),
[continuing Consumer-to-Producer tests](../../protocol-bindings/protocols/http/tests/action_consumer.rs)
and [root Action queries](../../protocol-bindings/protocols/http/tests/query_all_actions.rs)
cover actual TCP invoke/query/cancel, denial, revocation/replacement, late output,
original-bank retention and final release. The MQTT broker tests distinguish
native failure evidence from the logical reconciliation decision and retain
mixed HTTP/MQTT execution. A source-free candidate fixture additionally checks
distinct invoke/query/cancel/root participants without claiming real I/O for that
topology. This remains a bounded Host/manual implementation;
Review 10 F2/F3, continuing Consumer capacity, cross-instance compilation reuse
and broader profile/production acceptance are not completed by this correction.
