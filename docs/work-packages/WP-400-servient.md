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

`FiniteOutputValidation::Required` refuses an absent actual provider service at
admission and fences start/acceptance/delivery on service loss. Raw mode remains
explicit. Complete shape/backing checks precede the codec callback; dispatch
prepays the count/name lookup plus the declared maximum body and both media
traversals. Invalid output becomes the retained exchange error before the native
reply. A service is part of its actual captured configuration, not a support tag.

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
`servient::notification::RawNotificationServient` owns one Consumer incarnation
and a captured Binding/configuration, with independently admitted logical slots,
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

`cargo test --locked -p clinkz-wot-servient --test notification` exercises actual
Core/Servient lifecycle code through deterministic external in-memory Binding
capabilities: compatible sharing, independent stop/cursors, tenant separation,
late/stale delivery, item/byte/native loss, reciprocal responses and cardinality,
policy/configuration changes, saturation, failed cleanup and root disposal.
This is raw Host lifecycle evidence, not real protocol I/O, TD/schema/security
qualification, fleet scalability, hardware evidence or package/gate completion.

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
