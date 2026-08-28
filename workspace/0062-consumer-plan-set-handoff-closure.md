# 0062 Consumer Plan-Set Handoff Closure

Status: DISCUSSING

Kind: implementation-discovered architecture handoff investigation

Priority: HIGH

Target: the missing WP-200 -> WP-400 handoff needed to construct and publish the v5.1 Consumer Property Read plan set without Servient-owned TD interpretation

## Scope and authority

This topic records an implementation-boundary defect discovered after completion of `WP-300-CONSUMER-PROPERTY-READ-BINDING` and before admission of the corresponding WP-400 Consumer tranche.

It does not itself change active v5.1 authority, admit Rust source, register the Consumer architecture gate, reopen or reaffirm a completed tranche, or activate `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`, fallback, subscriptions, production Zenoh, or binding-carried security.

The question is narrow: what Planning-owned aggregate value and transaction boundary let `Servient::consume(td)` publish a usable consumed Property Read plan set when the completed WP-200 tranche compiles one exact `(property_name, property-form index)` coordinate at a time?

## Established repository facts

1. `PLAN-SET-001` assigns every consumed handle generation one Servient-owned aggregate compiled-plan-set record. Planning owns effective-view interpretation, target/form enumeration, candidate construction, immutable set material, and exact accounting. Servient owns persistent reservation, lifecycle state, publication, pins, operation leases, drain, and reclamation.
2. The completed WP-200 Consumer tranche exposes an exact-coordinate compiler entry through `PropertyReadPlanCompiler::consumer_call(...)` and immutable-plan-only selection.
3. The completed WP-300 Consumer tranche supplies the selected `OutboundRequest`, Host/static client execution contracts, cancellation settlement, and Core response-validation handoff after plan selection.
4. Current `Servient::consume(td)` has no exact Property Read coordinate and the legacy `ConsumedThingHandle::read_property` still rescans TD/Form state at call time.
5. `Thing::properties` is a `BTreeMap`; original property document order is not retained. Property traversal has deterministic key order while each property's Form vector retains array order and index.
6. Current `PlanBuildOutput` owns only logical-plan, artifact-envelope, and artifact-reference vectors; it does not yet project the broad Planning contract's target index, diagnostics, or exact aggregate accounting ledger.
7. `ADMIT-TXN-001` requires a reserve-build-publish transaction: charge work/temporary bytes first, reserve persistent capacity next, build private state only after reservation, then publish atomically; failure releases reservations idempotently.
8. The accepted Planning contract is incremental. Work-budget exhaustion returns `Pending(cursor)` without semantic progress beyond the available budget; failure returns ownership-preserving cursor state. Step partitioning must not change the final plan, candidate order, artifact, or error classification.
9. Active Planning complexity authority requires hot target-operation lookup through a prebuilt target index. Strict form lookup must not scan unrelated targets or all registrations. This target lookup obligation is independent of inactive `PLAN-INDEX-001` capability indexing.
10. The exact Consumer compiler receives `registration_index` and `candidate_order`; an aggregate caller must therefore freeze registration-snapshot ordinal and deterministic candidate position as well as PlanId/artifact identity.
11. The completed WP-300 admission deliberately excludes `AppliedSecurity`, credential-provider access, security branch selection, and binding-carried security material. Its first proof assumes a security decision that requires none of those representations.
12. ADR-0013 requires a new or reopened finding to trigger explicit impact review. Affected completed tranches must be reaffirmed or reopened before downstream work relies on them.

## The closure defect

The completed narrow tranches do not yet compose into the required public Servient path.

A conforming WP-400 implementation cannot recover the gap by:

- scanning TD/Form state at call time;
- moving TD interpretation into Servient startup code;
- selecting an arbitrary or first readable coordinate because the exact compiler needs one;
- changing public `consume(td)` into a one-property or one-form facade;
- re-entering legacy `ConsumedThing`, `BindingRequest`, support probing, or bare client-binding arrays; or
- building an aggregate draft before Servient has reserved its persistent capacity.

Direct WP-400 source admission therefore remains blocked until one exact Planning -> Servient aggregate handoff is accepted.

## Independent review history

### First review

The first independent review accepted the Planning-owned aggregate direction but requested changes before closure. It identified missing contracts for:

- all-or-nothing aggregate compile/failure semantics;
- first-proof security scope;
- deterministic PlanId/artifact identity;
- zero-plan property diagnostics;
- exact count/byte/temp/peak/work accounting;
- deterministic property/Form ordering; and
- ADR-0013 impact review of completed WP-200 and potentially WP-300 tranches.

Those findings remain valid.

### Second review

A second fresh independent review again returned `REQUEST CHANGES`. It confirmed the diagnosis and ownership direction, but found five remaining conflicts with active authority:

1. persistent reservation occurred too late in the candidate flow;
2. `nosec` was only a fixture choice rather than an admitted public `consume(td)` boundary;
3. aggregate bounded-progress, cursor, failure, and cleanup ownership were unspecified;
4. aggregate-wide linear target lookup contradicted the active prebuilt target-operation lookup contract; and
5. registration-snapshot ordinal and candidate ordering were unspecified.

The topic therefore remains `DISCUSSING`. Neither review accepts closure, authority migration, source admission, or WP-400 implementation.

## Revised candidate closure

The following is the current candidate to be challenged by a later independent review. It is not active authority.

### 1. One reserve-build-publish transaction

The first-proof Consumer Property Read aggregate is one transaction over a captured validated TD, immutable registration snapshot, plan-set generation, and active resource policy.

Its intended ordering is:

```text
Planning preflight
  -> Servient persistent reservation
  -> Planning bounded/resumable aggregate build
  -> Planning exact-ledger completion
  -> Servient reconciliation/commit
  -> atomic publication
```

Planning preflight may consume explicitly charged work and temporary bytes and may retain only bounded transaction-local temporary state needed to resume the transaction. It must not construct unreserved persistent plan-set state.

Preflight must produce a bounded persistent reservation ceiling sufficient for every retained first-proof target/index/plan/artifact/reference/diagnostic item and every admitted persistent byte that the subsequent aggregate build may commit. The ceiling is Planning-owned accounting information, not a second Servient accounting model.

Servient reserves that persistent ceiling before private aggregate state is built. A reservation failure stops the transaction before aggregate construction.

The bounded Planning build then constructs only within the reserved ceiling. On successful completion Planning returns the immutable aggregate material plus the exact ledger. Exact persistent usage must be less than or equal to the preflight reservation ceiling. Servient releases any unused reservation delta and commits only the exact persistent footprint before the single publication transition.

If exact usage exceeds the preflight ceiling, the transaction fails as an internal admission/bounds failure; Servient must not expand the reservation after private state has already been built.

### 2. First-proof security is an admission condition

The first aggregate tranche admits only the existing no-material security case.

Every retained effective `ReadProperty` coordinate must resolve deterministically to the admitted no-material `nosec` case. A coordinate requiring `AppliedSecurity`, credentials, provider access, binding-carried security material, a security branch selector, or another security representation is outside this first proof.

A TD containing a Property Read coordinate outside that admitted security case causes deterministic aggregate admission failure before publication. Such coordinates are not skipped and do not fall through to another Form.

The later source admission must freeze the exact existing error classification and provide negative evidence for secured/mixed security inputs. It must not invent a new security representation merely to make this tranche pass.

### 3. Aggregate build remains eager and atomic

Planning enumerates declared properties in deterministic `BTreeMap` key order and each property's Forms in retained array-index order.

Every effective `ReadProperty` coordinate admitted by the first-proof security and registration boundary is mandatory eager work. Each is compiled using the completed exact `PropertyReadPlanCompiler::consumer_call(...)` semantics rather than a second artifact compiler.

There is no skip, lazy negative, cache entry, post-failure fallback, or implicit next-Form selection. A terminal coordinate failure makes the aggregate transaction fail; no partial generation may be published.

Atomicity does not mean destroying resumable ownership prematurely. Pending and Failed steps preserve transaction ownership as described below.

### 4. Aggregate cursor and failure ownership

The aggregate Planning implementation requires one explicit resumable cursor/equivalent transaction state.

That state owns, directly or transitively:

- deterministic property/Form enumeration position;
- preflight or build phase identity;
- current aggregate count/work/temp accounting;
- provisional target-index and diagnostic material built only after reservation;
- completed provisional per-coordinate logical-plan/artifact/reference material;
- the current child exact-coordinate compiler cursor when one coordinate is in progress; and
- any other Planning-owned transaction-local buffers needed to resume without rescanning unrelated completed work.

A bounded step obeys the existing Planning contract:

- zero available work performs no contributor/compiler progress and returns ownership without semantic advancement;
- insufficient work returns `Pending(aggregate_cursor)`;
- terminal Planning failure returns an ownership-preserving aggregate failure/cursor rather than silently dropping caller-owned cleanup state; and
- varying step partitions cannot change coordinate order, PlanId, registration/candidate ordinal, artifact content, exact ledger, selection result, or terminal error classification.

The later admission must freeze the exact abort/cleanup API shape. The ownership division is already constrained:

- Planning abort/drop releases Planning-owned provisional plans, artifacts, indexes, diagnostics, child compiler cursor state, and transaction-local buffers according to the accepted compiler ownership contract;
- Servient owns the persistent reservation token and releases it idempotently after Planning cleanup/ownership transfer on every non-published exit; and
- neither side may release or forget the other's live ownership.

### 5. Deterministic target and Form semantics

The aggregate cannot depend on original JSON property order.

The first-proof deterministic coordinate sequence is:

1. property names in `BTreeMap` key order;
2. Forms inside one property in retained array-index order; and
3. only coordinates whose effective operation includes `ReadProperty` and which satisfy the admitted first-proof security/registration boundary.

Property name determines the addressed target before Form selection.

Omitted `form_index` selects that property's first retained applicable plan in Form order. Explicit `form_index` must match a retained applicable plan for that property. Neither path may rescan the TD or select a different property.

### 6. Prebuilt target-operation lookup is mandatory

The immutable aggregate must contain a bounded prebuilt target-operation lookup projection sufficient for Property Read hot lookup after the TD is dropped.

A sorted target table, target-to-plan-range table, bounded map, or equivalent representation is acceptable if it preserves the active Planning complexity contract.

For this first proof the index must allow:

- lookup of one property target without scanning unrelated properties;
- distinction between an absent property and a declared property with zero retained Property Read plans;
- direct access to that property's retained plan range; and
- strict `form_index` matching by examining only plans belonging to the addressed property/operation.

This is the active target-operation lookup obligation. It is not activation of `PLAN-INDEX-001` capability indexing and does not require multi-binding fallback infrastructure.

### 7. Zero-plan diagnostics survive TD destruction

The immutable target projection must preserve enough information to distinguish after TD/build inputs are gone:

- undeclared property -> `AffordanceMissing`;
- declared property with zero retained applicable Property Read plans -> `NoFormSupportsOperation`;
- declared property with retained plans but unmatched explicit `form_index` -> `StrictSelectionMismatch`; and
- omitted `form_index` -> first retained applicable plan for that property.

A declared zero-plan property therefore has a target/index record even though it owns no logical plan or artifact slot.

### 8. Plan, artifact, registration, and candidate identity

Each retained applicable coordinate receives one unique deterministic dense aggregate ordinal in the frozen coordinate sequence.

The candidate mapping remains:

```text
ordinal = 0..N in (property-key order, then form-index order)
PlanId slot = ordinal
artifact slot = ordinal
```

All generated identities carry the same plan-set generation required by their existing Core types.

Each retained plan has exactly one logical plan, one artifact envelope, and one `BindingArtifactRef`; each artifact slot addresses exactly one envelope. Duplicate, missing, stale-generation, or mismatched plan/envelope/reference identity is structural admission failure before publication.

For the first proof, Planning must find exactly one complete registration in the captured immutable snapshot that is eligible for the admitted Consumer Property Read role.

- zero eligible registrations -> deterministic preflight admission failure;
- more than one eligible registration -> deterministic preflight admission failure because multi-candidate selection/fallback is outside this tranche;
- exactly one eligible registration -> its ordinal in the captured registration snapshot becomes `registration_index`; and
- every retained coordinate has exactly one candidate, therefore `candidate_order = 0`.

Other ineligible registrations may exist in the snapshot but must not be scanned during hot strict lookup after publication.

### 9. Exact aggregate accounting ledger

The completed Planning result must include one exact accounting ledger/equivalent projection covering the first-proof aggregate's admitted semantic costs, including at least:

- declared property-target/index entries;
- zero-plan diagnostic entries;
- retained applicable plans;
- artifact/reference counts;
- logical-plan retained bytes;
- target/index and diagnostic retained bytes;
- binding-artifact admitted/measured retained items and bytes;
- aggregate structural retained bytes;
- aggregate/child compiler cursor temporary bytes;
- other temporary build bytes;
- admitted peak bytes; and
- consumed Planning work by applicable `WorkClass`/budget.

The exact Rust shape and whether this extends `PlanBuildOutput` or introduces a Planning-owned aggregate draft wrapper remain admission details.

What is fixed by the candidate ownership boundary is:

- Planning computes both the preflight persistent ceiling and the exact completed ledger;
- Servient reserves the ceiling and later commits/reconciles the exact result;
- WP-400 does not rescan the TD, recount plans, rebuild target indexes, remeasure artifacts, or invent a parallel accounting model; and
- overflow, ceiling violation, budget exhaustion, and terminal build failure never truncate the aggregate or publish partial state.

### 10. Host/static semantics remain shared

The aggregate algorithm, target ordering, registration/candidate ordinals, PlanIds, artifact slots, target index semantics, selection results, failure categories, and accounting semantics must match between Host-erased and application-static profiles.

Artifact payload representation may remain generic/profile-specific. The static profile must not be forced to adopt Host-erased storage merely to reuse the semantic aggregate algorithm.

### 11. Completed-tranche impact review remains mandatory

Before any aggregate source tranche is admitted, ADR-0013 impact review must be recorded for `WP-200-CONSUMER-PROPERTY-READ-PLANNING`.

It may be reaffirmed only if the existing exact-coordinate public contract, exact compiler behavior, selector guarantees, and completion evidence remain valid unchanged and the aggregate work is strictly additive around them. Otherwise it must be reopened together with invalidated dependent evidence.

`WP-300-CONSUMER-PROPERTY-READ-BINDING` is not presumed disjoint. It may be reaffirmed/disjoint only if the aggregate closure preserves the frozen `BindingArtifactRef` semantics, registration identity contract, `OutboundRequest` inputs, and Host/static selected execution contract. Otherwise ADR-0013 reopening rules apply.

These impact dispositions are predecessors of downstream WP-400 admission.

## Explicit exclusions

The first aggregate closure does not add:

- capability indexing under `PLAN-INDEX-001` beyond the already-active target-operation lookup obligation;
- lazy artifact/cache/single-flight state;
- a second eligible Consumer binding candidate;
- automatic candidate fallback or failure skip;
- write/action/observe/collection planning;
- advanced binding/media/subprotocol/security selectors;
- `AppliedSecurity`, credential-provider access, or binding-carried security material;
- Servient binding execution implementation;
- Consumer architecture-gate completion; or
- production Zenoh evidence.

## WP-400 consequence

Only after this aggregate handoff is independently accepted, projected into authority, admitted, implemented, evidenced, and its completed-tranche impact reviews are resolved may the WP-400 Consumer tranche proceed.

The intended composition is:

```text
consume(td)
  -> Planning preflight + persistent reservation ceiling
  -> Servient reserve persistent capacity
  -> Planning bounded/resumable aggregate build
  -> exact aggregate draft + target index + ledger
  -> drop TD/build-only inputs
  -> Servient reconcile/commit reservation
  -> atomic plan-set publication
  -> ConsumedThingHandle with generation-bearing plan-set ownership

read_property(name, options)
  -> acquire operation/plan-set lease
  -> indexed immutable target lookup
  -> addressed-property Form selection
  -> OutboundRequest::property_read(...)
  -> selected Host call / static ClientRequestSlot
  -> validate_untrusted_binding_output(...)
  -> InteractionOutput
  -> exactly-once call + plan lease settlement
```

Servient may orchestrate Planning, own reservation/publication/lifecycle state, and drive cleanup. It must not own TD interpretation, effective-form enumeration, candidate construction, target-index reconstruction, artifact remeasurement, or duplicate selection logic.

## Required evidence before closure acceptance

A later independent closure/admission review should require at least:

- preflight ceiling is computed before persistent reservation and no persistent aggregate state exists before reserve succeeds;
- reservation failure builds/publishes nothing;
- exact persistent ledger never exceeds the reserved ceiling and unused reservation is released before/at commit;
- secured or mixed-security Property Read coordinates outside the admitted no-material `nosec` case fail deterministically before publication;
- zero eligible and multiple eligible Consumer registrations fail deterministically; exactly one freezes its snapshot `registration_index` and `candidate_order = 0`;
- two properties whose insertion/source assumptions differ from `BTreeMap` key order;
- one property with multiple effective Property Read Forms;
- one declared property with zero retained Property Read plans;
- target lookup never scans unrelated property targets and strict Form lookup examines only the addressed property's plan range;
- deterministic PlanId/artifact-slot and one-to-one plan/envelope/reference identity;
- omitted and explicit Form selection semantics after TD destruction;
- `AffordanceMissing`, `NoFormSupportsOperation`, and `StrictSelectionMismatch` after TD/build inputs are dropped;
- zero-work steps make no semantic/compiler progress;
- different work-budget step partitions produce identical final aggregate identity/order/artifacts/ledger/error classification;
- pending progress preserves aggregate/child cursor ownership;
- terminal failure preserves cleanup ownership until explicit abort/drop semantics release Planning state;
- Servient releases persistent reservation idempotently on every non-published exit;
- Nth-coordinate terminal failure publishes no earlier provisional coordinate;
- exact count/byte/temp/peak/work accounting and overflow/ceiling-failure cases;
- equivalent semantic aggregate behavior in Host-erased and application-static cells;
- poison checks against Servient TD/Form scanning, legacy `ConsumedThing`, `BindingRequest`, raw hot support probing, lazy/cache/fallback, and second binding candidates; and
- recorded ADR-0013 impact-review outcomes for completed WP-200 and WP-300 Consumer tranches.

## Rejected immediate progression

Direct WP-400 admission or implementation remains rejected.

Likewise, this revised candidate must not be treated as accepted merely because it now incorporates two rounds of review. The exact preflight/reservation ceiling, cursor/abort ownership, no-material security predicate, target-index representation, zero/multiple-registration failure classification, ledger projection, and impact dispositions still require fresh independent closure review before the topic may become `DECIDED`.

## Merge and migration condition

This document may merge while `DISCUSSING` as the durable investigation record of the discovered handoff defect and independent review findings. Such a merge does not create source authority or unblock WP-400.

This topic may become `DECIDED` only after a fresh independent review accepts one exact legal Planning -> Servient handoff consistent with active admission, Planning progress, lookup, security, resource, and completed-tranche requirements.

It becomes `MIGRATED` only after that accepted conclusion is projected into the appropriate authoritative Planning/WP-400 artifacts and the necessary ADR-0013 source tranche/dependency projection is independently admitted.