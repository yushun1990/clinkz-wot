# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible bounded `Thing -> validated Consumer Planning` admission boundary for the v5.1 Consumer Property Read path without activating broad deferred validation/codec scope.

## Scope

Workspace topic 0062 established that the Consumer plan-set handoff cannot close while Planning still accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite admission boundary:

- borrowed typed-source validation provenance;
- one linear validation-to-Planning owner across every `Pending` boundary;
- checked Consumer resource-policy projection for `TypedThingBorrowed`;
- typed-input resource/work semantics and physical Host/static accounting;
- one stable complete-registration snapshot used consistently for compiler derivation;
- one upstream unpublished plan-build authority consumed by Planning;
- complete `BindingCompilerBounds` ownership and lifetime-work enforcement;
- WP-200/WP-300 impact required by the corrected Planning boundary; and
- pre-decision vs implementation-admission vs completion-evidence ordering.

This topic does **not** own persistent Consumer execution-registration pinning after publication, final 0062 plan-set publication material, plan-slot/generation allocation algorithms, final product cancellation API, Consumer binding execution, WP-400 Consumer implementation, broad validator compilation/cache/codec reuse, or architecture-gate completion.

## Stable repository facts

1. `Thing` is an ordinary cloneable/mutable public value. Deserialization and `ThingBuilder::build()` return ordinary `Thing`; neither type is durable admission provenance.
2. Current Basic TD validation is synchronous/unmetered; `ExtensionMap::validate_with_level(...)` is currently a semantic no-op.
3. Public `PlanBuildInput` is `Clone + Copy` and accepts raw `&Thing`, registration input, and `PlanSetGeneration`; public `PlanCompiler::start/step` accept fresh input on each call.
4. `PropertyReadPlanCompiler` stores plan/target/binding/configuration/compatibility/registration/candidate/role facts in `self`, not its cursor.
5. Current registration lookup validates artifact compatibility but does not itself prove full identity equality with the separately constructed compiler.
6. `BindingRegistrationIdentity::diagnostic_ordinal()` and `BindingCandidate::registration_ordinal()` are distinct domains.
7. `BindingCompilerBounds` declares artifact footprint, cursor bytes, temporary bytes, and lifetime `WorkBudget`; current Property Read Planning retains only the artifact bound.
8. `PlanId` and `PlanSetGeneration` are distinct identities and current Planning combines both into artifact identity.
9. Raw `ResourceLimits` is not a validated role/profile/cell/representation policy, and current raw document/JSON resource identities cannot silently acquire typed-Rust `Thing` semantics.
10. Active Foundation authority requires representation-specific source/temp/runtime/diagnostic/cleanup, current/peak, and contiguous accounting.
11. `WorkBudget::consume()` mutates one class only and does not atomically coordinate a separate lifetime allowance.
12. `WP-200-CONSUMER-PROPERTY-READ-PLANNING` and `WP-300-CONSUMER-PROPERTY-READ-BINDING` are both currently complete/admitted/current. The completed WP-300 Consumer tranche already bundles one registration identity and one compiler component in each validated complete registration.
13. ADR-0019 did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`.

## Defect

The old path admits several independently forgeable/substitutable inputs:

```text
ordinary Thing -> raw PlanBuildInput -> Planning trusts "validated_td"

input A -> start -> Pending -> input B -> step

compiler identity A + registration snapshot entry B
  -> compatibility-only acceptance may preserve an A/B mismatch

raw PlanId + independently supplied PlanSetGeneration -> artifact identity

BindingCompilerBounds -> cursor/temp/lifetime-work declaration
  -> current Planning retains only artifact bound
```

A later Servient wrapper cannot make the existing public Consumer Planning contract admitted-safe while those raw construction paths remain valid Consumer entry points.

## Independent review history

Reviews 1-4 established the borrowed immutable source, linear validation-to-Planning ownership, checked typed policy/schema revision, hierarchical accounting, shared Basic engine, atomic work charging, same-registration derivation, explicit ordinal domains, WP-200 reopening, and physical Host/static accounting directions.

Review 5 additionally required complete `BindingCompilerBounds` ownership, separation from PlanId allocation authority, non-circular evidence staging, and explicit snapshot-vs-diagnostic ordinal semantics.

Review 6 accepted those directions but found four remaining Stage-A gaps: the plan token bound only `PlanId`; there was no current-head constructibility fixture; resource/policy/layout remained checklist-level; and typed TD WorkClass/unit plus explicit completed WP-300 Consumer impact were missing.

The current head addresses Review 6 through the Stage-A artifacts below. The topic remains `DISCUSSING` pending fresh independent acceptance.

## Stage-A artifacts at the current head

The candidate is a three-artifact set and must be reviewed together:

1. `workspace/0063-bounded-validated-consumer-admission-input.md` — this boundary and governance staging.
2. `workspace/0063-stage-a-resource-policy-layout.md` — exhaustive first-proof resource migration, checked policy, typed-work unit, physical layout, and ADR-0013 impact definitions.
3. `planning/tests/consumer_admission_stage_a.rs` — non-production executable constructibility model. It is compiled and run by the ordinary locked workspace test path; it does not create a production API or admitted runtime path.

The reviewed head must keep all three coherent.

## Current candidate boundary

### 1. Borrowed typed source

First-proof admission captures `&'td Thing`.

Safe mutation cannot coexist with the live borrow; TD cursor state may borrow stable caller-owned storage without a movable self-reference; borrowed source contributes zero engine-retained-source bytes; typed structural/resource/work limits still apply; and the borrow ends only after Planning has copied every fact required beyond admission.

The legacy by-value `Servient::consume(Thing)` facade is not an architectural constraint. Any later owned-input convenience path needs separately admitted physical accounting.

### 2. Externally owned immutable complete-registration snapshot

Host Servient or application-static composition owns the snapshot. One admission borrows it for validation + Planning:

```text
Host/static composition
    owns CompleteRegistrationSnapshot
             |
             +---- immutable borrow ----> ConsumerAdmissionTxn<'reg>
```

The transaction never owns the snapshot and simultaneously stores a reference into it. If existing generic Planning code is reused during migration, `PlanBuildInput` is reconstructed only as an ephemeral private call value from the same source, same snapshot, and same build lease; it is never persisted or caller-supplied on the admitted path.

### 3. Ordinal domains remain distinct

The registration snapshot ordinal addresses the exact captured snapshot entry and becomes `BindingCandidate::registration_ordinal()`.

`BindingRegistrationIdentity::diagnostic_ordinal()` is reporting-only. The two are not required to match.

The Stage-A fixture uses snapshot ordinal `3` and diagnostic ordinal `17` and verifies that entry `3`, not `17`, supplies compiler bounds/start.

### 4. One opaque unpublished build lease binds both plan identities

0063 no longer consumes a PlanId-only token.

The eventual 0062 / Servient plan-set identity owner supplies one opaque move-only authority:

```text
UnpublishedPlanBuildLease {
    exact PlanId,
    exact PlanSetGeneration,
}
```

There is no admitted constructor from either raw identity and Consumer Planning accepts no independent `PlanSetGeneration`.

```text
0062 / Servient plan-set identity authority
       -> reserve one unpublished build
       -> UnpublishedPlanBuildLease(PlanId + PlanSetGeneration)

0063 validated admission + same lease -> sealed Consumer Planning
```

0063 freezes only this authority boundary, not the upstream slot/generation allocator. Abort returns/releases the reservation to that owner; successful freeze transfers the exact pair into the later 0062 lifecycle.

### 5. Admitted Planning constructor semantics are frozen now

Equivalent semantic shape:

```text
ValidatedConsumerAdmission::enter_planning(
    self,
    selected_snapshot_ordinal,
    UnpublishedPlanBuildLease,
) -> Result<ConsumerPlanningTxn, AdmissionFailure>
```

It accepts no raw `Thing`, raw `PlanBuildInput`, raw `PlanId`, raw `PlanSetGeneration`, external `PropertyReadPlanCompiler`, external `BindingRegistrationIdentity`, or replacement registration snapshot.

The exact future Rust names/generic factoring are a reopened WP-200 migration choice, but an accepted alternative cannot reintroduce those independent inputs.

### 6. Same-registration derivation is stronger than compatibility equality

The selected snapshot entry supplies both complete binding identity and compiler execution. Before compiler `bounds/start`, binding id, binding generation, configuration digest, and artifact compatibility correspond to that exact entry.

Equal compatibility between A and B cannot permit A's identity with B's compiler. The executable Stage-A fixture includes competing equal-compatibility registrations and verifies that only the selected entry receives `bounds/start`.

### 7. Complete `BindingCompilerBounds` becomes owned Planning admission authority

For the exact selected registration/compiler input:

```text
bounds() exactly once
  -> capture artifact footprint
  -> capture cursor bytes
  -> capture peak temporary bytes
  -> capture complete compiler lifetime WorkBudget
  -> reserve cursor/temp/artifact capacity
  -> only then compiler.start()
```

A memory-admission failure after `bounds()` but before `start()` leaves `start()` uncalled. The complete declaration remains owned until each corresponding lifetime ends and is reconciled/released on completion, failure, or abort.

### 8. Typed TD and compiler work have distinct lifetime authorities

Typed TD census/Basic admission uses the proposed append-only Foundation class `WorkClass::TypedTdAdmissionItems`. Its exact unit mapping and new `typed_td_admission_work_units_max` row are frozen in `workspace/0063-stage-a-resource-policy-layout.md`. It is not a reinterpretation of `JsonSchemaNodes` or `document_validation_work_units_max`.

Binding compiler work uses the classes declared by `BindingCompilerBounds::work()`.

Both use failure-atomic lifetime+step charging:

```text
preflight lifetime remaining
+ preflight caller step budget
  -> any failure: neither changes
  -> success: both decrement exactly once
  -> then work begins
```

Later step-budget replenishment cannot replenish either lifetime authority.

### 9. Resource schema and checked policy are defined, not deferred

`workspace/0063-stage-a-resource-policy-layout.md` dispositions the first-proof document/input/admission rows and adds only the minimal typed identities:

- `typed_td_nesting_depth_max`;
- `typed_td_members_per_map_max`;
- `typed_td_items_per_sequence_max`;
- `typed_td_value_nodes_per_thing_max`;
- `typed_td_string_bytes_per_thing_max`; and
- `typed_td_admission_work_units_max`.

Raw JSON/document rows are not serialization proxies for a materialized Rust `Thing`.

The same artifact defines the complete non-optional `TypedThingBorrowedConsumerPolicyV1` projection. Raw `ResourceLimits` cannot start traversal until schema/profile/role/cell/ingestion applicability is bound and every applicable value is present.

### 10. Physical Host/static accounting is constructibly defined

The Stage-A model defines distinct concrete `#[repr(C)]` Host/static enclosing storage types plus a real fixed failure slot capable of holding the modeled `ValidationIssue` or actual `CoreError` alternative.

`size_of`, `align_of`, and `offset_of` partition the one enclosing allocation/slot into structural, state, diagnostic, accounting, and compiler regions. The attribution covers every byte once, includes padding, gives diagnostics a real physical region, charges current/peak live once from the enclosing storage, and measures largest contiguous allocation from the whole allocation/exclusive static slot.

These types prove Stage-A constructibility only. Stage B maps the accepted attribution rule to real Servient storage; Stage C verifies production layouts.

### 11. One shared Basic semantic engine

TD owns one borrowed resumable Basic semantic engine/check graph and fixed-width issue location. Synchronous `Thing::validate_with_level(Basic)` must converge on that engine. Migration differential tests cover exact success/failure and first-issue agreement until delegation becomes structural.

Extension resource census may traverse extension JSON values without claiming extension semantic validation.

### 12. Cancellation remains above TD

The linear admission captures its Host/static cancellation source once. The outer owner checks cancellation before first traversal, every bounded TD/Planning step, resource transition, and publication transfer. TD receives no Core cancellation type.

## ADR-0013 impact disposition

The companion artifact carries the full table. The completed Consumer tranches are explicitly dispositioned:

- `WP-200-CONSUMER-PROPERTY-READ-PLANNING` — **must reopen**. A Servient wrapper cannot reaffirm the frozen raw public Consumer Planning contract.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING` — **affected; explicit reaffirmation required before 0063/0062 may rely on it**. Its current complete registration already owns identity + compiler component in one validated bundle, so this candidate does not presently require WP-300 source/public API changes. If Stage-B migration requires such a change, the tranche escalates to reopen before implementation.
- `consumer-property-read-binding-execution` evidence — affected and must prove sealed Planning obtains identity and compiler from the exact same complete registration entry.
- shared Producer Planning API/evidence — affected by mandatory WP-200 public migration and requires explicit transitive review.
- no completed Consumer WP-400 tranche exists to reopen; its future admission remains blocked on migrated 0063 + 0062 prerequisites.

## Evidence and governance ordering

### Stage A — before 0063 may become DECIDED

Stage A proves architecture constructibility, not production completion.

Fresh independent review evaluates this three-artifact set and decides whether it proves:

- borrowed source/cursor topology is Rust-constructible;
- linear typestate removes caller substitution across `Pending`;
- snapshot ownership is external/stable and Planning input reconstruction is ephemeral;
- ordinal domains are distinct;
- one opaque lease binds both `PlanId` and `PlanSetGeneration`;
- same-entry compiler derivation prevents equal-compatibility identity cross-wire;
- complete `BindingCompilerBounds` ownership/reservation/work lifetime is constructible;
- TD/compiler lifetime+step charging is failure-atomic;
- the typed WorkClass/unit and resource migration are semantically exact;
- the checked policy has no applicability holes;
- Host/static physical layout attribution is coherent/non-overlapping; and
- WP-200 reopen + WP-300 reaffirmation-required impact are correctly scoped.

A Stage-A model is explicitly non-production and cannot claim implementation completion.

### Stage B — before production implementation admission

After independent Stage-A acceptance and `DECIDED`, migration must formally reopen WP-200 Consumer Planning; impact-review/reaffirm WP-300 Consumer binding or reopen it if migration requires Core public/source changes; migrate accepted Foundation schema/policy/accounting/work authority; migrate TD bounded Basic authority; migrate the WP-200 public Consumer Planning contract; establish actual Servient admission storage/cancellation ownership; establish the upstream 0062/Servient `UnpublishedPlanBuildLease` contract; and obtain independent ADR-0013 admission for every affected production tranche.

### Stage C — post-implementation completion evidence

Only after admitted implementation exists must runtime evidence prove invalid typed input exclusion, non-substitution across `Pending`, unequal ordinal handling, equal-compatibility cross-wire rejection, complete compiler-bounds reservation/reconciliation, lifetime-work enforcement under replenished step budgets, typed/global/peak/contiguous resource enforcement, zero borrowed-source retention charge, actual production Host/static layout attribution, synchronous/incremental Basic equivalence, cancellation/failure rollback, no retained complete TD, and current reopened/reaffirmed WP-200/WP-300/shared evidence.

## Relationship to 0062

0062 remains blocked while 0063 is `DISCUSSING`.

An accepted/migrated 0063 gives 0062 only these facts:

1. Consumer admission begins from borrowed immutable typed input under one checked policy;
2. validation and Planning form one linear non-substitutable admission chain;
3. Host/static composition owns the stable complete-registration snapshot;
4. ordinal domains are explicit and same-entry compiler derivation is enforced;
5. complete `BindingCompilerBounds` memory/work declarations are owned;
6. one upstream opaque unpublished build lease binds `PlanId` + `PlanSetGeneration` and is consumed by Planning;
7. WP-200 Consumer Planning must migrate away from the raw admitted bypass;
8. WP-300 Consumer binding evidence requires explicit reaffirmation against the same-entry source rule; and
9. Foundation/TD/Planning/Servient migration obligations are frozen before implementation admission.

0062 must not absorb 0063's TD validator, resource-schema migration, WP-200 reopening, or physical-accounting design back into its local aggregate handoff. Persistent execution-registration pinning after publication remains the separate second prerequisite already identified by 0062.

## Merge / transition condition

This document may squash-merge while `DISCUSSING` as a durable investigation record.

It may become `DECIDED` only after a fresh independent review accepts the current Stage-A trio at one exact head:

- `workspace/0063-bounded-validated-consumer-admission-input.md`;
- `workspace/0063-stage-a-resource-policy-layout.md`; and
- `planning/tests/consumer_admission_stage_a.rs`.

`DECIDED` does not authorize production Rust implementation. The topic becomes `MIGRATED` only after the accepted conclusion is projected into relevant Foundation/TD/Planning/Servient authority and ADR-0013 impact/admission records.
