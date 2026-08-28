# 0062 Consumer Plan-Set Handoff Closure

Status: DISCUSSING

Kind: implementation-discovered architecture handoff investigation

Priority: HIGH

Target: the missing WP-200 -> WP-400 handoff needed to construct and publish the v5.1 Consumer Property Read plan set without Servient-owned TD interpretation

## Scope and authority

This topic records an implementation-boundary defect discovered after completion of `WP-300-CONSUMER-PROPERTY-READ-BINDING` and before admission of the corresponding WP-400 Consumer tranche.

It does not itself change active v5.1 authority, admit Rust source, register the Consumer architecture gate, reopen or reaffirm a completed tranche, or activate capability indexing under `PLAN-INDEX-001`, lazy/cache behavior, fallback, subscriptions, production Zenoh, or binding-carried security.

The question is narrow: what Planning-owned aggregate value and transaction boundary let `Servient::consume(td)` publish a usable consumed Property Read plan set when the completed WP-200 tranche compiles one exact `(property_name, property-form index)` coordinate at a time?

## Established repository facts

1. `PLAN-SET-001` assigns every consumed handle generation one Servient-owned aggregate compiled-plan-set record. Planning owns effective-view interpretation, target/Form enumeration, candidate construction, immutable set material, target lookup projections, and exact accounting. Servient owns persistent reservation, lifecycle state, publication, pins, operation leases, drain, and reclamation.
2. The completed WP-200 Consumer tranche exposes an exact-coordinate compiler entry through `PropertyReadPlanCompiler::consumer_call(...)` and immutable-plan-only selection.
3. The completed WP-300 Consumer tranche supplies the selected `OutboundRequest`, Host/static client execution contracts, cancellation settlement after execution selection, and Core response-validation handoff.
4. Current `Servient::consume(td)` has no exact Property Read coordinate and the legacy `ConsumedThingHandle::read_property` still rescans TD/Form state at call time.
5. `Thing::properties` is a `BTreeMap`; original property document order is not retained. Property traversal has deterministic key order while each property's Form vector retains array order and index.
6. Current `PlanBuildOutput` owns only logical-plan, artifact-envelope, and artifact-reference vectors. It does not retain the broad Planning contract's candidate list, `BindingPlanRef` join, target lookup, diagnostics, or exact aggregate accounting ledger.
7. `ADMIT-TXN-001` requires a reserve-build-publish transaction: work and temporary bytes are charged first, persistent capacity is reserved next, private persistent state is built only after reservation, and publication is atomic. Failure releases reservations idempotently. Cancellation is checked at bounded work intervals and immediately before publication.
8. The accepted Planning contract is incremental. Work-budget exhaustion returns `Pending(cursor)` without semantic progress beyond available work; failure returns ownership-preserving cursor state. Step partitioning must not change the final plan, candidate order, artifact, ledger, or error classification.
9. Active Planning complexity authority requires hot target-operation lookup through a prebuilt target lookup. Strict Form lookup must not scan unrelated targets or all registrations. This obligation is independent of inactive capability indexing under `PLAN-INDEX-001`.
10. `BindingCandidate` retains registration ordinal and candidate-order position. Active Planning authority also requires a `BindingPlanRef`-equivalent immutable join of logical-plan, candidate, and artifact slots.
11. The current exact Property Read child compiler constructs `BindingCandidate` transiently and drops its registration ordinal and candidate order when it returns the current three-vector `PlanBuildOutput`; `BindingArtifactIdentity` does not preserve those fields.
12. `BindingCompilerExtension::bounds(&BindingCompilerInput)` is the existing pure pre-progress declaration of final artifact, cursor, temporary, and typed-work bounds. It requires a resolved logical plan plus candidate, but it does not require `start`/`step` or produce an artifact.
13. The completed WP-300 admission deliberately excludes `AppliedSecurity`, credential-provider access, security branch selection, and binding-carried security material.
14. TD security inheritance treats a Form-level `security` value as an override even when explicitly empty. Security definitions can be `NoSec`, `Auto`, `Combo`, credential-bearing variants, and extension-bearing/proxy-bearing shapes.
15. `Servient` already owns one process/Servient-wide shutdown `AtomicBool` and exposes `ShutdownHandle::shutdown()`. The current public `consume(td)` accepts no per-call cancellation token. `CancellationView` is only a copyable snapshot, not a cancellation owner.
16. The plan-set state machine requires `Building + cancel -> Failed` and `Frozen + cancel -> Failed`; both outcomes are unpublished and transfer cleanup ownership to the plan-set reclaim owner.
17. Foundation currently exposes ten `WorkClass` values. None is explicitly a pure Planning/registration/index/reconciliation item class. `CONSTRAINED-WORK-001` nevertheless requires every bounded collection walk and target expansion to be charged before work begins.
18. The current exact Property Read child start path constructs/resolves the logical plan and queries compiler bounds/start after checking only that `BindingPolls` is nonzero; aggregate work admission therefore cannot simply claim a complete phase-by-phase Planning charge model without explicit WP-200/Foundation impact review.
19. ADR-0013 requires a new or reopened finding to trigger explicit impact review. Affected completed tranches must be reaffirmed or reopened before downstream work relies on them.

## The closure defect

The completed narrow tranches do not yet compose into the required public Servient path.

A conforming WP-400 implementation cannot recover the gap by:

- scanning TD/Form state at call time;
- moving TD interpretation into Servient startup code;
- selecting an arbitrary or first readable coordinate because the exact compiler needs one;
- changing public `consume(td)` into a one-property or one-Form facade;
- re-entering legacy `ConsumedThing`, `BindingRequest`, support probing, or bare client-binding arrays;
- constructing persistent aggregate state before Servient reserves capacity;
- scanning unrelated targets during Property Read lookup;
- silently filtering a coordinate because the selected registration later fails to compile it;
- dropping the selected candidate/registration ordinal after compilation; or
- claiming bounded aggregate progress without charging the traversal that produces it.

Direct WP-400 source admission remains blocked until one exact Planning -> Servient aggregate handoff is independently accepted.

## Independent review history

Four fresh independent reviews have returned `REQUEST CHANGES` while agreeing that a Planning-owned aggregate predecessor is the correct ownership direction.

### Review 1

The first review identified missing contracts for all-or-nothing compilation, first-proof security scope, deterministic identity, zero-plan diagnostics, exact accounting, deterministic property/Form ordering, and completed-tranche impact review.

### Review 2

The second review identified reserve-before-build ordering, executable first-proof security scope, bounded cursor/failure ownership, prebuilt target lookup, and registration/candidate ordinal gaps.

### Review 3

The third review identified admission cancellation semantics and ambiguity that could allow a global registration decision to become an illegal Form-specific filter.

### Review 4

The fourth review accepted those narrative corrections but found four constructibility blockers:

1. the pre-publication cancellation source/owner was assumed rather than tied to a constructible Servient API and the `Building -> Failed` state-machine transfer;
2. "no-material nosec" was not an executable structural predicate with deterministic failure precedence;
3. the completed aggregate did not retain the selected candidate/registration projection required by broad Planning authority; and
4. aggregate work did not assign a `WorkClass`/charge unit to every bounded traversal and reconciliation phase.

The topic therefore remains `DISCUSSING`. No review has accepted closure, authority migration, source admission, or WP-400 implementation.

## Current candidate closure

The following is the only current candidate. Earlier candidate wording is superseded by this section. It remains non-authoritative until a fresh independent review accepts it.

### 1. One reserve-build-publish admission transaction

The first-proof Consumer Property Read aggregate is one Servient-owned admission transaction over:

- one captured validated TD;
- one immutable complete-registration/compiler snapshot;
- one plan-set generation;
- one active resource policy and `WorkBudget`; and
- the existing Servient-wide shutdown source.

The transaction is represented internally by a fixed-size Servient admission guard/record whose lifecycle begins in `Building` before preflight starts. This guard is transaction-local lifecycle/accounting ownership, not an unpublished compiled plan set and not a public handle.

The ordering is:

```text
Building transaction guard
  -> Planning bounded preflight
  -> cancellation checkpoint
  -> Servient persistent reservation
  -> Planning bounded/resumable aggregate build
  -> Planning exact-ledger completion
  -> Servient reconciliation -> Frozen
  -> final cancellation checkpoint
  -> atomic publication -> Published
```

Every cancellation or terminal failure before publication ends through the state-machine's unpublished `Failed` transition. There is no direct return path that bypasses `Building/Frozen -> Failed` cleanup ownership once the admission guard exists.

### 2. Preflight ceiling is constructible without artifact progress

Preflight may use charged bounded temporary state but may not retain the compiled plan-set state that the persistent reservation is intended to cover.

For the first proof, preflight computes the persistent ceiling in two classes:

1. **structural aggregate ceiling** from deterministic TD/snapshot facts: target/lookup entries, zero-plan diagnostics, retained Property Read coordinate count, candidate records, `BindingPlanRef` records, logical-plan retained string/structural bytes, artifact-reference/envelope structural bytes, and aggregate ledger/index structure; and
2. **binding-artifact ceiling** by constructing one charged temporary resolved logical-plan/candidate view for each admitted coordinate and invoking the selected registration compiler's existing pure `bounds(&BindingCompilerInput)` method.

The preflight bounds probe:

- may resolve URI/material needed to create the temporary logical-plan view;
- may construct a temporary `BindingCandidate` carrying the selected registration ordinal and `candidate_order = 0`;
- may call `bounds(...)`;
- must not call compiler `start(...)` or `step(...)`;
- must not produce or retain an artifact;
- must charge all source traversal, URI, security, registration, and Planning-item work before performing it; and
- drops or retains only explicitly bounded temporary preflight state after its ceiling contribution has been recorded.

After Servient reserves the resulting ceiling, the real build is allowed to reconstruct the exact logical plan and call the exact child compiler normally. This first proof deliberately tolerates duplicate, separately charged preflight/build resolution rather than changing the already-completed child compiler API merely to optimize it.

For the same captured inputs, the build-time compiler bound for a coordinate must not exceed its preflight bound. A larger later bound is an internal admission/invariant failure; Servient never grows persistent reservation after build state exists.

This makes the first-proof ceiling constructible using current compiler contracts without building a protocol artifact before reservation.

### 3. Admission cancellation source is the existing Servient shutdown authority

The first proof does **not** add a per-`consume` cancellation token or change the public `consume(td)` signature.

Its sole pre-publication cancellation source is the existing Servient-wide shutdown flag already shared through `ShutdownHandle`.

Semantics:

- `ShutdownHandle::shutdown()` is the request operation;
- the Servient admission driver reads the same flag and converts it to the protocol-neutral active/requested cancellation view at each admitted checkpoint;
- Host-erased and application-static profiles observe the same semantic shutdown event and state-machine outcome; representation may differ only in storage/driver mechanics;
- a shutdown request already present when `consume(td)` begins causes the newly created `Building` admission transaction to cancel to unpublished `Failed` without persistent reservation;
- a request observed during preflight or after reservation causes `Building -> Failed` and transfers provisional Planning state plus any reservation to `PlanSetReclaimOwner`/the admitted cleanup owner before failure becomes observable;
- a request observed after successful build/reconciliation but before publication causes `Frozen -> Failed`;
- the required final shutdown read occurs immediately before the single publication transition; and
- once `Published` wins the linearization race, later shutdown is drain/destroy lifecycle work and does not retroactively convert the generation to admission failure.

`CancellationView` remains a snapshot passed/read at a checkpoint. It is not treated as the cancellation owner or request handle.

### 4. First-proof `nosec` predicate is exact and structural

Security eligibility is determined during Planning preflight from the validated TD only. No credential provider, `AppliedSecurity`, binding callback, or security execution occurs.

For one effective `ReadProperty` Form, the coordinate is admitted by the first-proof security predicate **iff all of the following hold**:

1. `effective_form_security(thing, form)` contains exactly one security-definition name;
2. that name resolves in the validated Thing's security definitions;
3. the resolved variant is exactly `SecurityScheme::NoSec`;
4. the NoSec context's `scheme` discriminator is `"nosec"`;
5. the NoSec context has no `proxy`; and
6. the NoSec context has no preserved extension fields.

Human-readable description/tag metadata on that exact NoSec definition is ignored by the first-proof security semantics because it neither selects nor carries security material.

The following are outside the first proof and fail the entire aggregate rather than being skipped:

- an explicitly empty effective Form security override;
- more than one effective security name, even when every referenced definition is NoSec;
- `Auto`, `Combo`, Basic/Digest/APIKey/Bearer/PSK/OAuth2, or any other non-NoSec variant;
- a proxy-bearing NoSec definition;
- an extension-bearing NoSec definition; or
- any shape requiring branch selection, provider access, credentials, scopes, binding-carried security material, or another security projection.

A structurally valid but first-proof-unsupported security shape returns `CoreError::UnsupportedOperation` with `ErrorPhase::Admission`, `RetryClass::Never`, `Operation::ReadProperty`, and the offending Form index when representable. A missing referenced security definition that survives the validated-input boundary is an `InternalInvariant` rather than being reinterpreted as nosec.

Preflight failure precedence is frozen as:

1. cancellation observed at the checkpoint before the next work item;
2. deterministic target/Form traversal and first offending unsupported security coordinate in `(property-key, form-index)` order;
3. registration-cardinality selection;
4. per-coordinate URI/logical-plan preflight and compiler-bounds validation; and
5. persistent reservation.

Thus an unsupported security coordinate is never hidden by a later registration/bounds failure, and registration/bounds failures never reinterpret security.

### 5. Registration selection is global and metadata-only

The first proof admits exactly one complete registration in the captured immutable snapshot for Consumer Property Read.

Preflight registration eligibility uses only captured complete-registration metadata and the already-admitted profile/compiler/execution presence. It does not receive a Form and does not invoke compiler `bounds/start/step` or any protocol support callback.

The eligible registration must:

- advertise the coarse Consumer Property Read capability;
- expose the compiler half required by the active Host/static cell;
- expose the corresponding admitted execution half for that cell; and
- preserve one exact identity/configuration/compatibility tuple across the captured compiler/execution registration halves.

Cardinality is deterministic:

- zero eligible registrations -> existing `SelectionFailureReason::NoSupportingBinding` classification;
- more than one eligible registration -> existing `SelectionFailureReason::AmbiguousBindingOwner` classification because multi-candidate selection is outside this tranche;
- exactly one -> its **actual ordinal in the captured snapshot**, including a nonzero ordinal, becomes `registration_index` for every retained coordinate.

The selected registration has no Form-specific eligibility predicate. Every effective first-proof-NoSec `ReadProperty` coordinate is mandatory eager compile work against this same registration. Any bounds/start/step/artifact/compatibility failure of one coordinate fails the aggregate; it never becomes zero-plan, omission, next-Form selection, or alternate-registration selection.

Every coordinate has exactly one candidate, therefore `candidate_order = 0`.

### 6. Aggregate work is explicitly charged before progress

The aggregate cannot claim bounded progress using only informal "work" language.

The first-proof charge projection is:

| Phase/work item | Charge before work |
| --- | --- |
| Visit one Planning source item that is not accurately represented by an existing specialized class: registration snapshot entry, property/Form planning item, target/index record construction, candidate/`BindingPlanRef` materialization, aggregate ledger/reconciliation item | one `PlanningItems` unit |
| Inspect one effective security name/definition branch | one `SecurityBranches` unit |
| Resolve/copy URI-template or resolved target bytes | exact `UriBytes` consumed for the admitted source/output byte contract |
| Invoke binding compiler bounded progress | existing `BindingPolls` contract supplied by the compiler path |
| Release one provisional aggregate/candidate/artifact/index/ledger cleanup item | one `CleanupItems` unit |

`PlanningItems` is a **candidate additive Foundation work class**, not current source authority. The current ten-class enum has no semantically accurate class for pure registration/index/reconciliation work; mislabeling such work as `BindingPolls`, `CleanupItems`, or `JsonSchemaNodes` is rejected.

Therefore closure acceptance requires an ADR-0013 impact review of Foundation before aggregate source admission:

- if an independent authority review identifies an existing work class whose documented semantics already and accurately covers every `PlanningItems` row, the table may be projected onto that existing class without changing Foundation; otherwise
- Foundation must be reopened/additively extended with an explicit Planning/admission-item class before the aggregate tranche is admitted.

The same impact review must examine the completed WP-200 exact child because its current Start path constructs/resolves the logical plan and calls compiler bounds/start after only a nonzero `BindingPolls` check. WP-200 may be reaffirmed only if the accepted charge projection can wrap it without changing its frozen behavior/evidence; otherwise it must reopen.

Zero budget for the next required class performs no corresponding walk, resolution, bounds probe, materialization, or reconciliation work. Changing step partitioning cannot change final identity, candidate order, artifact, ledger, or error classification.

### 7. Aggregate build remains eager, atomic, and resumable

Planning enumerates declared properties in deterministic `BTreeMap` key order and each property's Forms in retained array-index order.

Every effective `ReadProperty` coordinate satisfying the exact first-proof NoSec predicate is mandatory eager work after the single global registration is selected. Each real artifact build uses the completed exact `PropertyReadPlanCompiler::consumer_call(...)` semantics rather than a second artifact compiler.

There is no skip, lazy negative, cache entry, post-failure fallback, Form-specific registration filter, or implicit next-Form selection. A terminal coordinate failure makes the aggregate transaction fail; no partial generation may be published.

The aggregate cursor owns, directly or transitively:

- phase and deterministic source position;
- preflight ceiling/count/work/temp accumulation;
- current selected registration identity/ordinal;
- provisional target lookup and zero-plan diagnostics after reservation;
- completed provisional logical plans, candidate records, binding-plan references, artifact envelopes, and artifact references;
- the current child exact-coordinate compiler cursor; and
- exact-ledger/reconciliation progress.

`Pending` and `Failed` preserve caller-visible ownership according to the accepted Planning/compiler contract. Explicit abort/drop releases Planning-owned state. Servient owns the persistent reservation token and releases/transfers it idempotently on every unpublished exit.

### 8. Deterministic target/Form and zero-plan semantics

The first-proof coordinate sequence is:

1. property names in `BTreeMap` key order;
2. Forms inside one property in retained array-index order; and
3. Forms whose effective operation includes `ReadProperty` and whose security satisfies the exact first-proof NoSec predicate.

Registration selection is global and cannot remove a coordinate from this sequence.

A declared property may have zero Property Read plans only when none of its Forms has effective `ReadProperty`. An effective `ReadProperty` Form that fails security or compilation is an aggregate admission failure, not a zero-plan diagnostic.

After TD/build inputs are gone, immutable target metadata must distinguish:

- undeclared property -> `AffordanceMissing`;
- declared property with zero effective Property Read coordinates -> `NoFormSupportsOperation`;
- declared property with retained plans but unmatched explicit `form_index` -> `StrictSelectionMismatch`; and
- omitted `form_index` -> first retained plan for that property in Form-array order.

### 9. Prebuilt target lookup and candidate projection are mandatory

The immutable aggregate contains a prebuilt target-operation lookup. Hot lookup addresses one property without scanning unrelated properties, reaches only that property's plan range, and performs strict Form matching only inside that range.

The aggregate also retains the candidate/registration projection required by active Planning authority.

For each retained coordinate, the dense ordinal mapping is:

```text
ordinal = 0..N in (property-key order, then form-index order)
plan slot = ordinal
candidate slot = ordinal
artifact slot = ordinal
PlanId slot = ordinal
candidate.registration_ordinal = selected snapshot registration_index
candidate.candidate_order = 0
BindingPlanRef = (plan slot, candidate slot, artifact slot)
```

Each retained coordinate therefore has:

- one logical plan;
- one immutable `BindingCandidate`-equivalent record preserving binding identity/generation/configuration/compatibility, actual registration snapshot ordinal, and candidate order;
- one `BindingPlanRef`-equivalent join;
- one artifact envelope; and
- one `BindingArtifactRef`.

Artifact identity still binds plan-set generation, PlanId, binding identity/generation, configuration, compatibility, and `ConsumerCall` role. Candidate ordinal/order are preserved by the candidate projection and its `BindingPlanRef`; they are not incorrectly inferred from `BindingArtifactIdentity`.

Duplicate/missing/stale/mismatched plan/candidate/reference/artifact identity is structural admission failure before publication.

Evidence must include a sole eligible Consumer registration at a **nonzero** snapshot ordinal and prove that the published candidate projection retains that ordinal after the TD and registration snapshot are dropped.

### 10. Exact ceiling and ledger include all retained projections

The Planning preflight ceiling and completed exact ledger both cover, at minimum:

- target lookup/zero-plan diagnostic records and bytes;
- logical-plan records and retained bytes;
- candidate records and bytes;
- `BindingPlanRef` records and bytes;
- artifact envelopes, artifact references, admitted/measured artifact items and bytes;
- aggregate structural bytes;
- preflight/aggregate/child cursor temporary bytes;
- other peak temporary bytes; and
- consumed work for every applicable `WorkClass`.

Planning computes the ceiling and exact ledger. Servient reserves the ceiling, reconciles the exact result, releases unused reservation, and commits only the exact footprint. WP-400 does not rescan TD, registration snapshots, candidates, target lookup, artifacts, or ledgers.

### 11. Host/static semantics remain shared

Host-erased and application-static profiles share:

- the Servient-wide shutdown cancellation event and `Building/Frozen -> Failed` semantics;
- exact security predicate and failure precedence;
- registration cardinality and actual snapshot ordinal;
- target/Form ordering;
- PlanId/candidate/artifact slot layout and `BindingPlanRef` joins;
- work-charge semantics;
- target lookup/selection outcomes; and
- ceiling/ledger semantics.

Artifact payload and internal storage representation remain profile-specific. Static code is not forced into Host-erased storage.

### 12. Completed-tranche/Foundation impact review is mandatory

Before aggregate source admission, ADR-0013 impact review must record dispositions for:

1. `WP-200-CONSUMER-PROPERTY-READ-PLANNING` — reaffirm only if its exact-coordinate public behavior/evidence remains valid and aggregate/preflight/work changes are strictly additive; otherwise reopen;
2. `WP-300-CONSUMER-PROPERTY-READ-BINDING` — reaffirm/disjoint only if candidate/artifact identity, `OutboundRequest`, execution, and cancellation-settlement contracts remain valid; otherwise reopen; and
3. Foundation work budgeting — reaffirm only if the existing ten `WorkClass` semantics can accurately express every required aggregate charge without semantic relabeling; otherwise reopen/additively extend the work-class contract before source admission.

These dispositions precede WP-400 admission.

## Explicit exclusions

The first aggregate closure does not add:

- capability indexing under `PLAN-INDEX-001` beyond active target-operation lookup;
- lazy artifact/cache/single-flight state;
- a second eligible Consumer binding candidate;
- automatic candidate fallback or failure skip;
- write/action/observe/collection planning;
- advanced security branch/provider behavior;
- `AppliedSecurity`, credential-provider access, or binding-carried security material;
- per-`consume` public cancellation tokens;
- Servient binding execution implementation;
- Consumer architecture-gate completion; or
- production Zenoh evidence.

## WP-400 consequence

Only after this aggregate handoff is independently accepted, projected into authority, admitted, implemented, evidenced, and all required impact reviews are resolved may the WP-400 Consumer tranche proceed.

The intended composition is:

```text
consume(td)
  -> Servient creates Building admission guard
  -> Planning bounded preflight
       -> exact NoSec structural validation
       -> global registration selection
       -> temporary logical-plan/candidate + pure compiler bounds probes
       -> persistent reservation ceiling
  -> Servient shutdown/cancellation checkpoint
  -> Servient reserve persistent capacity
  -> Planning bounded aggregate build
       -> target lookup + diagnostics
       -> logical plans + candidates + BindingPlanRefs
       -> exact child artifacts + BindingArtifactRefs
       -> exact ledger
  -> Servient reconcile reservation -> Frozen
  -> final shutdown/cancellation checkpoint
  -> atomic publication -> Published
  -> ConsumedThingHandle

read_property(name, options)
  -> acquire operation/plan-set lease
  -> indexed immutable target lookup
  -> addressed-property Form selection
  -> BindingPlanRef / candidate / artifact identity validation
  -> OutboundRequest::property_read(...)
  -> selected Host call / static ClientRequestSlot
  -> validate_untrusted_binding_output(...)
  -> InteractionOutput
  -> exactly-once call + plan lease settlement
```

Servient orchestrates Planning and owns cancellation source, reservation, publication, lifecycle, and cleanup transfer. It does not own TD interpretation, effective-Form enumeration, security interpretation, candidate construction, target-lookup reconstruction, artifact measurement, or duplicate selection logic.

## Required evidence before closure acceptance

A later closure/admission review should require at least:

- preflight creates no protocol artifact and calls no compiler `start/step` before persistent reservation;
- preflight temporary logical-plan/candidate + pure `bounds` probes produce a ceiling that the later exact build never exceeds;
- reservation failure builds/publishes no persistent aggregate state;
- shutdown already requested before `consume` produces a `Building -> Failed` unpublished outcome;
- shutdown during preflight, after reservation, during child compilation, during reconciliation, and at the final pre-publication checkpoint produces the required unpublished `Failed` ownership transfer;
- publication-vs-shutdown race has one linearization winner;
- identical cancellation semantics in Host/static profiles;
- exact NoSec acceptance for one referenced plain NoSec definition;
- deterministic rejection for explicit empty security override, multiple NoSec names, Auto, Combo, credential-bearing schemes, proxy-bearing NoSec, extension-bearing NoSec, and mixed-security inputs;
- security rejection precedence over registration/bounds failures when the security coordinate is encountered first;
- zero/multiple eligible registrations fail deterministically;
- sole eligible registration at a nonzero snapshot ordinal is retained by the immutable candidate projection;
- every effective accepted ReadProperty coordinate compiles against that one registration and one coordinate failure fails the whole aggregate;
- target lookup does not scan unrelated targets;
- candidate and `BindingPlanRef` projections survive TD/registration snapshot destruction;
- dense plan/candidate/artifact slot identity and one-to-one joins;
- `AffordanceMissing`, `NoFormSupportsOperation`, `StrictSelectionMismatch`, and omitted-Form selection after TD destruction;
- zero budget for each required work class makes no corresponding progress;
- phase-by-phase charge evidence for source traversal, security branches, URI bytes, registration/index/candidate/reconciliation items, binding progress, and cleanup;
- work step-partition invariance;
- exact ceiling/ledger count/byte/temp/peak/work accounting including candidate/`BindingPlanRef` bytes;
- Nth-coordinate failure/cancellation leaves no published earlier coordinate;
- Host/static semantic parity; and
- recorded ADR-0013 dispositions for WP-200, WP-300, and Foundation work budgeting.

## Rejected immediate progression

Direct WP-400 admission or implementation remains rejected.

This candidate also must not be treated as accepted merely because four review rounds have narrowed the findings. The new Servient-wide cancellation projection, executable NoSec predicate, candidate/`BindingPlanRef` retention, preflight bounds probe, charge table, and Foundation/WP-200 impact consequences require another fresh independent closure review.

## Merge and migration condition

This document may merge while `DISCUSSING` as the durable investigation record of the discovered handoff defect and independent review findings. Such a merge does not create source authority or unblock WP-400.

This topic may become `DECIDED` only after a fresh independent review accepts one constructible Planning -> Servient handoff consistent with active admission, cancellation, Planning candidate/index, security, work-budget, resource, lifecycle, and completed-tranche requirements.

It becomes `MIGRATED` only after that accepted conclusion is projected into the appropriate authoritative Planning/Foundation/WP-400 artifacts and the necessary ADR-0013 source tranche/dependency projection is independently admitted.