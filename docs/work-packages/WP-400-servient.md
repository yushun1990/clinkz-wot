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
