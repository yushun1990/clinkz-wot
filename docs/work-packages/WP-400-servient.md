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

## Raw finite shared-instance slice

The additional [caller-owned finite Read](../spec/runtime-safety.md#caller-owned-finite-read)
uses the modern common finite kernel with borrowed prepared meaning/services,
lifetime-bound local authority and caller-funded active work/results. Its
[downstream datagram fixture](../evidence/constrained-finite-execution.md) checks
bounded progress, first cause, stale references, late/invalid output and final
physical release without making the Host registry representation mandatory.
Existing Host finite execution uses that same transition kernel. Only NoSec
Property Read is implemented for the constrained representation; other families,
TD preparation, real Embedded transports and automatic driving remain open.
This is a single architecture-risk correction, not a new portability programme
or a prerequisite to common Security composition and execution-semantics work.

The execution-only Host slice in `servient/finite` separates the bounded logical
registry from typed native runtimes. Registry count/name limits belong to
`RawProducerRegistryLimits`; per-runtime input/result opportunities belong to
`RawFiniteLimits`. Revocation and complete same-slot replacement occur at the
logical owner. Accepted work retains the original instance through native
settlement and acknowledgement. Registry mutation/destruction has at most 32
entries and 16 behaviors per instance with names at most 256 bytes; linked
callbacks/local destructors remain trusted cooperative code, not preemptible.
Caller-provisioned runtime slots have independent native/dispatch/cleanup budgets
and no growing Engine runtime queue. There is no aggregate process-RAM claim.

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
explicit. Complete shape/backing checks precede the codec callback; dispatch
prepays the count/name lookup plus the declared maximum body and both media
traversals. Invalid output becomes the retained exchange error before the native
reply. Input is checked before fresh authorization and handler effects using
that same original candidate. A service is part of its actual captured
configuration, not a support tag. The output-only finite API is replaced directly.

Progress takes one caller-qualified monotonic instant. The runtime retains the
original native request deadline across incomplete input, dispatch and pending
send; expiry, clock mismatch/regression and explicit close keep the first cause,
complete original exchange and accepted behavior until retirement/acknowledgement.
Zero credits can fence locally but cannot run a native/cleanup poll. Drop remains
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
The Servient reserves a bounded per-owner invocation slot and declared application
work footprint before handler entry, retains original behavior/principal and
monotonic invocation identity, and enforces input/output and paid lookup/step
ceilings. Separate query/cancel routes may belong to other Bindings. Controls
recheck current instance, route, principal and policy; neither a copied reference
nor a native request correlation is execution permission.

Revocation/replacement fence new acceptance and request cooperative cancellation
of accepted work without redirecting it. Cancel and local abandon allow late
completion; a funded explicit close or owner retirement first delivers any queued
application cancellation once (retaining its error) before polling local close.
Explicit close retains failed/pending cleanup and reports local cancellation
without claiming remote terminal state. Input traversal and handle-based step
lookup are prepaid before scanning admitted fields/invocations. While the owner
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
controls, target names and per-owner Engine reservations. Each slot prepays one
extra handoff and one response payload; native participation/reply indirect
backing stays at the Binding owner. These conservative logical allowances are
not additive shared physical RAM or aggregate Host-account qualification. Linear
work budgets precede callbacks/queue progress. Stop fences delivery before mapped
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

`consumer_notification::ConsumerNotificationRuntime` now consumes the typed
TD-selected Property or Event definition and captures its actual data/access services around
this same lifecycle. Validation prepays work before native handoff and checks
each candidate before logical acceptance, with post-callback authority checks.
Original bank leases fund the root's queue and validation envelope until runtime
disposal. Property value schemas and Event subscription/data/dataResponse/cancellation
roles remain distinct;
the MQTT subset explicitly rejects unsupported application control/response data.
Real broker tests join the qualified Engine Producer to these independent
Consumers and exercise stop/unsubscribe, loss, replacement and cleanup. Native
matching/distribution/sharing and physical release remain Binding-owned.
Constrained execution and automatic progress remain unimplemented. No universal runtime,
session or all-family acceptance gate is introduced.

The separate `consumer_action::ConsumerActionRuntime` now retains TD-selected
continuing invocation identity, mappings and services after each native exchange
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

Unread native completion remains revocable until a paid settlement handoff with
fresh time, original authority and captured service/access checks. The HTTP
Consumer regressions cover direct handoff and additional progress after
revocation, denial, service/root loss, cancellation or expiry, plus insufficient
handoff work and callback-time service loss. An actual HTTP query exchange also
returns its original late output through retirement after a poll error; the
original error, late output and captured validation failure remain independently
retained and funded. These repairs do not change package acceptance status.

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

## Open finite Consumer ownership

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
controls, late-result validation and finite native disposal remain. Original
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
removal and replacement, owner shutdown, silent-peer retention with zero credit
then funded cleanup, and abandoned A with active or settled B in both loss orders.
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

The managed HTTP fixture retains its 1024-credit admission cases and adds repeated
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
late completion. Every native/cleanup callback needs credit. A successful raw
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

The Host account bank reserves controlled owner layouts, declared handler and
configuration indirect backing, description/property backing, one input/result
opportunity and prepaid cleanup before native setup. Original reservations are
shared by runtime and behavior, so accepted instances and unread results retain
them through replacement. Final TD decoding/semantic temporary capacity and its
overlapping Engine charge are reserved separately before setup, owned only by
the exposure's qualification phase. Pending qualification keeps them for retry;
completed qualification releases them after its temporary owners are destroyed
and before the local publication commit. Cancellation, timeout, setup failure,
cleanup-only progress and exposure Drop also end that phase without releasing
the independent lifetime/result/cleanup leases. Caller-provided
description/handler/provider construction
precedes admission; output construction stays application-backed until accepted.
JSON decoding uses bounded input and serde's finite recursion limit with a
conservative Host temporary allowance, not an allocator-intercepted heap maximum.
Native OS/SDK allocation and finite disposal remain Binding/deployment policy.
These are aggregate reservations for this owner, not complete qualification of
all legacy flat resource fields or a measured maximum-RAM/performance claim.

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
definitions, captured immutable startup registrations and independently typed
native participation owners. Refusal returns those same input owners. This is
an ownership handoff after caller construction/preparation, before the managed
participants acquire native work; it does not reuse the single-Property facade.

The fixed member vector admits 1..=32 participants and 1..=32 actual capability
registrations in this manual Host slice. Final common/service/behavior coverage
is checked before the handoff and again after closed setup. Each effective
operation has exactly one native owner and every member covers the qualified
exposure. The source-free publication stages two bounded original identity
vectors before setup, moves one into the registry at commit, and rejects changed,
duplicate, omitted or foreign retirement sets. No allocation, native progress or
fallible preparation is added inside the local publication commit.

The existing shared `HostResourceAccounts` bank reserves Engine retained bytes,
cleanup byte/item allowances and a Thing slot before ownership handoff. Costs
include controlled layouts, definition storage, captured reference vectors,
declared handler indirect backing, actual participant configuration/input bounds
and promised retained result opportunities. Qualification reserves the bounded
Host common TD allowance plus the maximum declared peak temporary backing of
the actual sequential support/final-check callbacks, with the overlapping Engine
charge. Final comparison uses original definitions without a second retained
set. Unknown bounds refuse admission and return the original owners before a
callback or native setup; a current bound above the captured funded peak refuses
publication before checking. It ends after final qualification and before commit,
or on cancellation/definitive qualification failure/successful retirement/Drop.
Successful retirement ends qualification even when callers closed every typed
native owner directly. Failed retirement and pending pure work preserve their
applicable owners for retry. Lifetime and cleanup reservations are one shared physical
reservation owner retained by the exposure, actual participants and original
logical instance; shared retention references do not debit the bank again.

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
share the original store's bounded slots; managed admission additionally reserves
each result's maximum byte/cleanup capacity from the captured original bank before
behavior. The [HTTP regressions](../../protocol-bindings/protocols/http/tests/query_all_actions.rs)
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
Producer Action publication captures the validator's extra-work allowance;
invoke prepays both potential validation callbacks and a continuing poll prepays
output validation. Changed service bounds fence validation without replacing
original work or requiring handler/codec credits for independent close.

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
