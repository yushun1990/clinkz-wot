# 0062 Consumer Plan-Set Handoff Closure

Status: DISCUSSING

Kind: implementation-discovered architecture handoff investigation

Priority: HIGH

Target: the missing WP-200 -> WP-400 handoff needed to construct and publish the v5.1 Consumer Property Read plan set without Servient-owned TD interpretation

## Scope and authority

This topic records an implementation-boundary defect discovered immediately after completion of `WP-300-CONSUMER-PROPERTY-READ-BINDING` and before admission of the corresponding WP-400 Consumer tranche.

It does not itself change active v5.1 authority, admit Rust source, register the Consumer architecture gate, reopen or reaffirm a completed tranche, or activate `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`, `BIND-PROGRESS-001`, fallback, subscriptions, or production Zenoh.

The question is narrower: what Planning-owned value lets `Servient::consume` publish a usable consumed Property Read plan set when the already-completed WP-200 tranche compiles one exact `(property_name, property-form index)` coordinate at a time?

## Established repository facts

The following facts are already authoritative or completed implementation evidence.

1. `PLAN-SET-001` assigns every consumed handle generation one Servient-owned aggregate compiled-plan-set record. Planning constructs immutable set material; Servient owns the build transaction orchestration, publication, pins, operation leases, drain, and reclamation.
2. The general Planning algorithm owns enumeration of target contexts, effective operations, forms, and binding candidates. Servient does not reinterpret a TD to recover planning decisions.
3. The completed WP-200 Consumer tranche deliberately exposes only an exact-coordinate compiler entry:

   ```text
   PropertyReadPlanCompiler::consumer_call(
       property_name,
       property_form_index,
       ...
   )
   ```

   Its completion proof uses multiple readable properties and multiple readable forms and proves that the exact constructor cannot acquire target semantics from a different property/form merely because it appears earlier.
4. `select_consumer_property_read` consumes immutable `PlanBuildOutput`, property name, and narrowed `InteractionOptions`; it cannot receive a TD, raw Form, binding object, or support probe.
5. The completed WP-200 tranche returns one unpublished eager single-coordinate draft. Publication, leases, draining, and reclamation are intentionally left to WP-400.
6. The completed WP-300 tranche supplies the selected `OutboundRequest`, Host client call, static client slot, exact-request rejection, cancellation settlement, and Core response-validation handoff required after selection.
7. WP-400 v5.1 authority requires `consume` to publish an immutable consumed Property Read plan generation before returning the handle, and requires `read_property` to select through that published generation rather than entering the legacy `ConsumedThing` path.
8. The current legacy `ConsumedThingHandle` still scans the TD at call time to find a supporting Form, and current `Servient::consume(td)` has no explicit Property Read build coordinate.
9. `Thing::properties` is represented by `BTreeMap`, so original JSON/document property order is not retained. Property iteration has deterministic key order; each property's Form vector retains its own array order and original form index.
10. Current `PlanBuildOutput` owns only logical-plan, artifact-envelope, and artifact-reference vectors. It does not yet carry an aggregate resource/footprint ledger or zero-plan target diagnostics.
11. ADR-0013 requires a new or reopened finding to trigger explicit impact review. An affected completed tranche must be reaffirmed or reopened; dependent completed tranches may be treated as disjoint only when requirement, artifact, dependency, and evidence scope actually remain valid.

## The closure defect

The completed narrow tranches do not currently compose into the required public Servient path.

A WP-400 implementation cannot legally choose any of the following shortcuts:

- **Call-time TD/Form scanning.** This directly violates the v5.1 Consumer boundary and the negative evidence already required by WP-200.
- **Servient-owned startup TD enumeration.** Moving the scan from call time to `consume` does not fix ownership. Target/form/effective-operation enumeration belongs to Planning; Servient may orchestrate Planning but must not implement a second planner.
- **Selecting one readable property/form merely because the current WP-200 compiler needs one coordinate.** WP-200 explicitly proved that its exact constructor cannot silently substitute another coordinate.
- **Changing public `consume(td)` to require one property/form coordinate.** This would move an internal compilation coordinate into the application facade, narrow one consumed Thing to one preselected property, and create a new public-contract decision not required by current v5.1 authority.
- **Re-entering legacy `ConsumedThing`, `BindingRequest`, `supports_with_thing`, or bare client-binding arrays.** Those are the exact target backflows the Consumer architecture proof is intended to eliminate.

Therefore an ADR-0013 WP-400 source admission that starts directly from the current WP-200 single-coordinate draft would be incomplete: it would leave the source, bounds, identity layout, and failure semantics of the published consumed plan-set contents unspecified.

## Independent review outcome

The first closure candidate at `f1601ee79401845b804879727420cdadf8a19859` was independently reviewed against baseline `b9a547859cabaeb444ba3a330b4a8a2e16697842`.

The review accepts the **technical ownership direction**: a Planning-owned narrow aggregate Consumer Property Read tranche is the smallest legal location, and it can remain outside `PLAN-INDEX-001`, lazy/cache, fallback, subscriptions, and production Zenoh.

The review does **not** accept closure. The candidate remains `DISCUSSING` and must not become `DECIDED`, migrate authority, or justify WP-400 admission yet. Four missing contracts block closure:

1. aggregate per-coordinate failure/rollback and first-proof security scope;
2. constructible aggregate identity, diagnostics, and exact resource ledger;
3. ADR-0013 impact review of already-completed WP-200 and potentially WP-300 tranches; and
4. precise deterministic ordering semantics consistent with the TD `BTreeMap` representation.

The active v5.1 requirement identities are sufficient for this closure; no new requirement identity is currently justified. The missing work is an implementation/admission projection of already-active bounds and ownership.

## Revised candidate boundary

Subject to a later exact ADR-0013 admission, the Planning-owned aggregate direction should satisfy all of the following. These are required closure conditions, not current implementation claims.

### 1. Aggregate build is one eager atomic transaction

The first aggregate proof uses exactly one complete Consumer-capable registration and no binding-carried security material. The reference fixture should use `nosec`; adding `AppliedSecurity`, credential-provider input, or binding-carried security state remains outside this closure.

Planning enumerates every declared property in deterministic key order. For each property, it inspects that property's Form vector in original array-index order and identifies every effective `ReadProperty` coordinate. A coordinate whose effective operations do not contain `ReadProperty` is not a Consumer Property Read plan candidate.

For every effective `ReadProperty` coordinate, the aggregate transaction must invoke the already-completed exact `PropertyReadPlanCompiler::consumer_call` semantics under the same complete registration. The complete registration has only coarse Consumer Property Read capability in this first proof; there is no Form-specific support callback that authorizes silent omission.

Therefore the transaction is all-or-nothing:

- every effective `ReadProperty` coordinate must compile successfully;
- any compiler-bound, compiler-start, compiler-step, artifact-admission, identity, count, byte, temporary, peak, or work failure fails the entire aggregate build;
- no failed coordinate is skipped;
- no deterministic compiler negative is retained as a lazy/cache entry;
- no failure implicitly selects the next Form; and
- every partially produced logical plan, artifact, reference, temporary buffer, cursor, and reservation is released/rolled back before failure returns.

No binding or protocol side effect may occur during this transaction.

### 2. Deterministic coordinate and selection semantics

The aggregate cannot claim original property document order because `Thing::properties` is a `BTreeMap`.

The first-proof deterministic order is instead:

1. properties in the map's deterministic key order;
2. within one property, Forms by their retained original array index/order; and
3. only effective `ReadProperty` coordinates enter the plan sequence.

The property name determines the addressed target before Form selection. Omitted `form_index` selects that property's first retained applicable plan in Form-array order. It never selects a different property and it is initial immutable-plan selection, not post-failure fallback.

An explicit `form_index` must match a retained applicable plan for that property. It cannot rescan the TD or choose another Form after failure.

### 3. Aggregate identities are one-to-one and generation-bound

The aggregate admission must freeze one unique deterministic `PlanId` and artifact slot for every retained applicable coordinate.

The smallest constructible first-proof mapping is a dense ordinal over the aggregate applicable-coordinate sequence:

```text
ordinal = 0..N in (property-key order, then form-index order)
PlanId = PlanId::new(SlotIndex::new(ordinal), plan_set_generation.get())
artifact_slot = SlotIndex::new(ordinal)
```

The logical plan, artifact envelope, and `BindingArtifactRef` for one coordinate must agree on that `PlanId`, artifact slot, plan-set generation, binding id/generation, configuration, compatibility, and `ConsumerCall` role. Each retained plan has exactly one artifact envelope and exactly one reference; an artifact slot addresses exactly one envelope. Duplicate or missing ids/slots are structural failure before publication.

The existing exact-coordinate compiler may continue to accept a supplied `PlanId`; the aggregate owner supplies the deterministic id rather than changing compiler ownership.

### 4. Property-target and zero-plan diagnostics survive TD destruction

The aggregate Planning result must retain enough immutable target metadata to distinguish these call-time outcomes after the TD has been dropped:

- addressed property was not declared -> `AffordanceMissing`;
- addressed property was declared but has zero retained effective `ReadProperty` plans -> `NoFormSupportsOperation`;
- addressed property has retained plans but an explicit `form_index` matches none -> `StrictSelectionMismatch`;
- omitted `form_index` -> first retained applicable plan for that property.

A zero-plan property therefore remains represented in bounded immutable target diagnostics even though it consumes no plan or artifact slot. WP-400 must not recover this distinction by retaining or rescanning the TD.

### 5. Exact aggregate resource ledger precedes publication

The aggregate result must not be only three unconstrained vectors. Its admission must define and implementation must produce one exact Planning-owned `PlanFootprint`/equivalent accounting ledger covering at least:

- declared property-target count and zero-plan diagnostic count;
- retained applicable plan count;
- artifact/reference count;
- logical-plan retained bytes;
- binding-artifact admitted/measured retained items and bytes;
- aggregate structural/diagnostic bytes;
- compiler cursor bytes;
- temporary build bytes;
- admitted peak bytes; and
- total consumed Planning work by applicable `WorkClass`/budget.

All counts, bytes, temporary state, peak state, and work are checked against already-active Planning/resource limits before the aggregate draft is returned. Overflow or budget failure is atomic build failure; there is no truncation.

The exact Rust projection and whether the ledger extends the current `PlanBuildOutput` or is owned by another Planning set-draft value must be frozen by the later tranche admission. What is already fixed here is ownership: Planning computes and returns the exact ledger from the build; WP-400 may reserve/commit that ledger into its plan-set lifecycle but must not rescan the TD, recount plans, remeasure artifacts, or invent a second accounting model.

### 6. Host/static semantics remain shared without representation leakage

The aggregate algorithm and selection semantics must be identical for Host-erased and application-static profiles. The artifact payload type remains generic and can differ physically by profile, while target order, plan ids, artifact slots, selection results, failure categories, and resource semantic deltas remain equal.

The static profile must not be forced to adopt host-erased storage solely to reuse aggregate logic.

### 7. Explicit completed-tranche impact review is mandatory

This finding necessarily touches the completed WP-200 Consumer area because the aggregate closure must extend or wrap the current `PlanBuildOutput` and immutable selector behavior and will likely add production work beside or inside `planning/src/property_read.rs`.

Before a new aggregate tranche is admitted, the repository must record ADR-0013 impact review for `WP-200-CONSUMER-PROPERTY-READ-PLANNING`:

- **reaffirm** only if the existing exact-coordinate public contract, its single-coordinate behavior, and its completion evidence remain valid unchanged and the new aggregate work is strictly additive around it; or
- **reopen** if the existing public contract, implementation semantics, admitted source boundary, or completion evidence must change.

`WP-300-CONSUMER-PROPERTY-READ-BINDING` is not predeclared unaffected. Its impact review may record **disjoint/reaffirmed** only if the aggregate closure preserves the already-frozen `BindingArtifactRef` identity semantics, complete-registration contract, `OutboundRequest` construction inputs, and selected Host/static binding execution contract. If any of those change, WP-300 and any transitively invalidated evidence must be reopened as required by ADR-0013.

This impact review occurs before downstream WP-400 admission; a completed-tranche status must not be silently carried across a changed handoff assumption.

## Explicit exclusions retained

The first aggregate tranche remains deliberately smaller than broad Planning:

- no `PLAN-INDEX-001` capability index;
- no lazy artifact or cache/single-flight state;
- no second Consumer binding candidate;
- no automatic candidate fallback or failure skip;
- no write/action/observe/collection planning;
- no binding-id/media/subprotocol/security-branch/validation-profile selector;
- no binding-carried security material in the first proof;
- no Servient lifecycle or binding execution implementation; and
- no Consumer architecture-gate or production Zenoh completion claim.

A bounded linear lookup over the immutable first-proof aggregate is acceptable unless implementation evidence falsifies an already-active bound. The inactive `PLAN-INDEX-001` contract does not authorize Servient to re-enter the TD and does not require a capability index merely to complete this first proof.

## WP-400 consequence

Only after the aggregate Planning handoff is admitted, implemented, evidenced, and its completed-tranche impact reviews are resolved may the WP-400 Consumer tranche be admitted from this closure.

The intended composition remains:

```text
consume(td)
  -> Planning-owned atomic aggregate Consumer Property Read draft + exact ledger
  -> drop TD/build-only inputs
  -> Servient reserve/commit + atomic plan-set publication
  -> ConsumedThingHandle with generation-bearing plan-set ownership

read_property(name, options)
  -> acquire operation/plan-set lease
  -> Planning-owned immutable-set selection
  -> OutboundRequest::property_read(...)
  -> selected Host call / static ClientRequestSlot
  -> validate_untrusted_binding_output(...)
  -> InteractionOutput
  -> exactly-once call + plan lease settlement
```

Servient may own the aggregate record, generation, publication state, leases, admission commit, drain, cancellation ownership, and cleanup. It must not own effective-form enumeration, raw Form interpretation, candidate construction, target diagnostics reconstruction, artifact remeasurement, or a duplicate selection algorithm.

The existing legacy `ConsumedThingHandle` implementation may remain for unmigrated capabilities, but the target `read_property` evidence must poison its TD scan, `BindingRequest`, support-probe, and bare-client-binding edges.

## Revised dependency consequence

If a later independent review accepts this revised closure and its exact admission projection, the implementation order is expected to become:

1. completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` exact-coordinate tranche, with explicit impact review disposition;
2. completed `WP-300-CONSUMER-PROPERTY-READ-BINDING` tranche, with explicit impact review disposition;
3. newly admitted narrow WP-200 Consumer aggregate-plan-set tranche;
4. WP-400 Consumer Property Read Servient tranche depending on the aggregate Planning handoff and the valid WP-300 execution contracts;
5. cross-package Consumer Property Read architecture gate; and
6. real Host Zenoh Consumer Property Read evidence.

The aggregate closure is upstream constructibility work, not broad WP-200 completion. Whether it is strictly additive to the completed exact-coordinate tranche and disjoint from WP-300 is an impact-review result, not a premise.

## Required evidence before closure acceptance

A later closure/admission review must require at least:

- two declared properties in a key order that differs from insertion/source order assumptions;
- one property with multiple Forms including multiple effective `ReadProperty` coordinates;
- one declared property with zero effective `ReadProperty` plans;
- deterministic dense PlanId/artifact-slot mapping and one-to-one plan/envelope/reference identity;
- omitted Form selection of the addressed property's first retained applicable Form only;
- explicit Form selection and mismatch behavior;
- `AffordanceMissing`, `NoFormSupportsOperation`, and `StrictSelectionMismatch` after the TD/build inputs are dropped;
- atomic rollback when the Nth effective coordinate fails compiler bounds/start/step/artifact admission;
- proof that no earlier successful coordinate remains published or retained after aggregate failure;
- exact count/byte/temp/peak/work ledger admission and overflow cases;
- nosec/no binding-carried security material in the first proof;
- equivalent semantic aggregate and selection behavior in Host-erased and application-static cells;
- poison checks against Servient TD/Form scanning, legacy `ConsumedThing`, `BindingRequest`, raw support probing, lazy/cache/fallback, and second binding candidates; and
- recorded ADR-0013 impact-review outcomes for completed WP-200 and WP-300 Consumer tranches.

## Rejected immediate progression

Directly admitting or implementing WP-400 before this handoff is closed remains rejected. The missing aggregate construction source affects package ownership, public Consumer facade correctness, immutable plan-set semantics, accounting, completed-tranche validity, and the negative legacy-backflow proof. It is therefore an architecture-sensitive predecessor issue, not a local Servient implementation detail.

Likewise, the current aggregate direction must not be treated as accepted merely because its ownership location is plausible. The exact transaction, ledger, identity, diagnostics, and impact-review contracts above must survive a later independent closure review.

## Merge and migration condition

This document may merge while `DISCUSSING` as the durable investigation record of the discovered handoff defect and the independent review findings. Such a merge does not accept the candidate closure and creates no source authority.

This topic may become `DECIDED` only after an independent review accepts an exact legal Planning -> Servient handoff including aggregate failure semantics, deterministic identity/order, zero-plan diagnostics, resource ledger, security scope, and completed-tranche impact dispositions.

It becomes `MIGRATED` only after that accepted conclusion is represented in the appropriate Planning/WP-400 authoritative owners and the necessary ADR-0013 source tranche/dependency projection is independently admitted. WP-400 implementation remains blocked until then.
