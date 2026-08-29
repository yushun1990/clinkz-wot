# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible bounded `Thing -> validated Consumer Planning` admission boundary for the v5.1 Consumer Property Read path without activating broad deferred validation/codec scope.

## Scope

Workspace topic 0062 established that the Consumer plan-set handoff cannot close while Planning still accepts ordinary `&Thing` under an informal `validated_td` premise.

0063 owns exactly the prerequisite admission boundary:

- borrowed typed-source validation provenance;
- one linear validation-to-Planning owner across every `Pending` boundary;
- checked `TypedThingBorrowed` Consumer resource-policy applicability;
- typed-input resource/work semantics;
- physical Host/static admission-storage accounting rules;
- one stable complete-registration snapshot used consistently for compiler derivation;
- one upstream unpublished plan-build reservation consumed by Planning;
- complete `BindingCompilerBounds` ownership and compiler lifetime-work enforcement;
- WP-200/WP-300 ADR-0013 impact caused by the corrected Planning boundary; and
- Stage-A constructibility vs Stage-B implementation admission vs Stage-C completion evidence.

0063 does **not** own persistent Consumer execution-registration pinning after publication, final 0062 aggregate plan-set publication material, final plan-slot/generation allocator design, Consumer binding execution, final product cancellation API, WP-400 source implementation, broad validator compilation/cache/codec reuse, or Consumer architecture-gate completion.

## Stable repository facts

1. `Thing` remains an ordinary cloneable/mutable public value. Deserialization and `ThingBuilder::build()` do not create durable admission provenance.
2. Basic TD validation is currently synchronous/unmetered; unknown extension semantic validation remains a no-op even though a resource census may traverse extension values.
3. Public `PlanBuildInput` accepts raw `&Thing`, registration input, and `PlanSetGeneration`, and public `PlanCompiler::start/step` accept fresh input on each call.
4. `PropertyReadPlanCompiler` stores build-defining plan/binding/registration facts outside its cursor, so a caller can currently cross-wire compiler/resume inputs.
5. Current registration lookup checks artifact compatibility but does not by itself prove the separately constructed compiler identity came from that exact complete registration.
6. snapshot registration ordinal and `BindingRegistrationIdentity::diagnostic_ordinal()` are distinct domains.
7. `BindingCompilerBounds` declares final artifact footprint, compiler cursor bytes, peak temporary bytes, and one lifetime `WorkBudget`.
8. The actual `BindingCompilerExtension::step` SPI receives exactly one `&mut WorkBudget`.
9. `PlanId` and `PlanSetGeneration` are distinct identities and both enter artifact identity.
10. raw `ResourceLimits` is not a checked role/profile/cell/representation policy.
11. active `PLAN-COST-003` / `PLAN-ARTIFACT-001` require logical-plan bytes, artifact count/bytes, compiler cursor, temporary bytes, and work bounds; the exhaustive resource schema provides explicit rows for them.
12. WP-200 Consumer Planning and WP-300 Consumer Binding are currently complete/admitted/current; WP-300 complete registrations already associate identity and compiler component in one validated registration bundle.
13. ADR-0019 did not activate broad validator-cache/codec domains.

## Defect

The existing admitted surfaces allow adjacent substitutions:

```text
ordinary Thing -> raw PlanBuildInput -> Planning trusts "validated_td"

input A -> start -> Pending -> input B -> step

compiler identity A + registration entry B
  -> compatibility equality can hide a same-registration mismatch

raw PlanId + independently supplied PlanSetGeneration
  -> one artifact identity

BindingCompilerBounds
  -> cursor/temp/artifact/lifetime-work declarations
  -> current Planning retains/enforces only a subset
```

A later Servient wrapper cannot make those public Consumer Planning paths admitted-safe while safe external callers can still construct them directly.

## Independent review history

Reviews 1-4 established borrowed immutable source admission, linear typestate, checked typed-policy applicability, hierarchical accounting, a shared bounded Basic semantic engine, same-registration derivation, ordinal separation, mandatory WP-200 reopening, and representation-specific physical accounting.

Review 5 required complete `BindingCompilerBounds` ownership, separation from plan-id allocation authority, explicit ordinal semantics, and non-circular evidence staging.

Review 6 required one build lease binding both `PlanId` and `PlanSetGeneration`, executable Stage-A constructibility evidence, concrete resource/policy/layout definitions, explicit typed-TD work units, and explicit WP-300 transitive impact.

Review 7 accepted the Stage-A/Stage-B boundary but found four remaining constructibility defects at the reviewed head:

1. Host/static state regions were placeholder arrays smaller than the modeled `BorrowedTdCursor` / `Validating` states;
2. checked policy omitted active logical-plan/artifact/compiler-cursor/per-step-work controls;
3. compiler lifetime + caller-step charging was demonstrated only by a helper and not through the real one-budget compiler SPI; and
4. rejected/aborted unpublished-build leases had no modeled return/release path.

The current candidate addresses those four findings in the Stage-A artifacts below. Status remains `DISCUSSING` pending fresh independent acceptance.

## Current-head Stage-A artifact set

Fresh review must evaluate these four artifacts together:

1. `workspace/0063-bounded-validated-consumer-admission-input.md` — claim boundary and governance staging.
2. `workspace/0063-stage-a-resource-policy-layout.md` — complete typed-source/memory/Planning resource disposition, checked policy, real-SPI work adapter, physical layout, and ADR-0013 impact definitions.
3. `planning/tests/consumer_admission_stage_a.rs` — primary executable constructibility model.
4. `planning/tests/consumer_admission_stage_a_pending.rs` — focused move-only Pending/resume substitution and rejection/abort lease-ownership model.

None is production implementation authority.

## Candidate boundary

### 1. Borrowed typed source

First-proof admission captures `&'td Thing`. Safe mutation cannot coexist with that live borrow. TD traversal state borrows caller-owned stable storage rather than owning a movable `Thing`. Borrowed source contributes zero engine-retained-source bytes while typed structural/string/work limits still apply to observed input.

The legacy by-value `Servient::consume(Thing)` facade is not an architectural requirement. Any future owned-input convenience path needs separately admitted physical accounting.

### 2. One externally owned immutable complete-registration snapshot

Host/static composition owns the registration snapshot. Validation + Planning borrow that same snapshot for the complete admission transaction.

No transaction owns the snapshot by value while storing references into it. Any reuse of current generic Planning internals reconstructs ephemeral input from the already captured source/snapshot/build authority; the admitted caller never supplies a replacement `PlanBuildInput` after entry.

### 3. Ordinal domains remain distinct

The snapshot ordinal is an index into the captured complete-registration snapshot and becomes `BindingCandidate::registration_ordinal()`.

`BindingRegistrationIdentity::diagnostic_ordinal()` is diagnostic identity only. A fixture uses snapshot ordinal `3` and diagnostic ordinal `17`; compiler progress comes from entry `3`.

### 4. One opaque unpublished plan-build lease owns both identity and reservation lifetime

The upstream 0062/Servient identity authority issues one move-only `UnpublishedPlanBuildLease` binding:

```text
exact PlanId
+ exact PlanSetGeneration
+ one unpublished-build reservation lifetime
```

Admitted Consumer Planning accepts no independent raw `PlanId` or `PlanSetGeneration`.

A failed Planning entry returns the exact validated transaction + lease. Planning abort returns the exact lease. Internal lease destruction has an idempotent release fallback. Successful freeze commits/transfers the reservation once into the later 0062 lifecycle. 0063 still does not decide the final plan-slot/generation allocator.

### 5. Same complete registration supplies identity and compiler

The sealed Planning entry accepts no external compiler and no independent `BindingRegistrationIdentity`.

The selected complete-registration entry supplies binding id, binding generation, configuration digest, artifact compatibility, and compiler component. Equal compatibility between registrations A and B is insufficient to combine A's identity with B's compiler.

### 6. Complete compiler bounds are captured before compiler start

For the exact same-registration compiler input:

```text
compiler.bounds(input) exactly once
 -> final artifact footprint
 -> compiler cursor bytes
 -> peak compiler temporary bytes
 -> lifetime WorkBudget
 -> preflight/reserve every applicable resource authority
 -> only then compiler.start(input)
```

A rejected cursor/temp/artifact/logical-plan reservation leaves `start()` uncalled.

### 7. Checked policy includes typed, memory, and active Planning authority

The checked first-proof policy is not a four-field adapter. It carries concrete non-optional values for:

- typed-TD depth/map/sequence/node/string/work plus logical TD/form/schema/template/candidate controls;
- retained-source, admission temporary, local/global peak, engine-live, contiguous, and compiled-runtime controls; and
- active/deferred PLAN-SET/PLAN-ARTIFACT controls including `compiled_plan_bytes_max`, `logical_plan_bytes_per_thing_max`, plan-set/pin limits, artifact count/bytes per-item/per-Thing/global, compiler cursor per-item/global, `plan_compile_work_units_per_step_max`, and reclaim step bytes.

Raw JSON-only fields are explicitly NA under `TypedThingBorrowed`, not omitted accidentally. Inactive lazy/cache/index/payload/subscription families remain excluded because their owning requirements are inactive.

Exact migration/disposition is frozen in `workspace/0063-stage-a-resource-policy-layout.md`.

### 8. Compiler lifetime + caller-step work wraps the real current SPI

The current Core SPI remains:

```rust
fn step(
    &self,
    input: &BindingCompilerInput<'_>,
    cursor: Self::Cursor,
    budget: &mut WorkBudget,
) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;
```

Stage A now demonstrates this constructible adapter:

```text
BindingCompilerBounds lifetime WorkBudget
 + caller current-step WorkBudget
 + plan_compile_work_units_per_step_max
 -> jointly reserve both parents before work
 -> child WorkBudget containing only that reservation
 -> real BindingCompilerExtension::step(..., &mut child)
 -> reconcile unused child capacity to both parents
```

The reservation holds exclusive mutable access to both parent budgets while the compiler sees only the child. Therefore the compiler cannot exceed either lifetime or current-step capacity, and the total child grant cannot exceed the explicit per-step plan-compile ceiling.

Zero available work causes no compiler callback. Unused reservation is returned after the call; actual consumed work remains charged exactly once.

The primary fixture implements a real `BindingCompilerExtension` and drives its actual `step` method this way. Therefore this Stage-A finding does **not** currently require changing the Core compiler-step signature. WP-300 still requires reaffirmation because same-registration sourcing/admitted-wrapper semantics changed; if Stage B discovers a Core public/source change is necessary, WP-300 escalates to reopen before implementation.

### 9. Typed TD work remains a separate new class

Typed census/Basic admission uses proposed append-only `WorkClass::TypedTdAdmissionItems` with exact item-transition semantics and its own lifetime row `typed_td_admission_work_units_max`.

It is not a reinterpretation of `JsonSchemaNodes` or `document_validation_work_units_max`.

### 10. Physical Stage-A storage contains the modeled states

The placeholder `state_words` arrays are removed.

The primary executable fixture defines one actual inline union whose alternatives are:

```text
BorrowedTdCursor
Validating
Validated
Planning
```

Each alternative is stored through `ManuallyDrop`. The test directly proves union size and alignment are at least every modeled alternative, then proves the Host and application-static state regions physically contain that union.

The concrete Host/static `#[repr(C)]` enclosures also contain a real `FailureSlot`, accounting storage, and fixed fixture compiler cursor/temp/artifact regions plus lifetime-work storage. `offset_of` partitions structural/state/diagnostic/accounting/compiler attribution without overlap.

This is a Stage-A physical constructibility model, not a claim that future production Servient uses exactly this layout. Heap-backed Host plan/artifact bytes are separate charged allocations under the explicit logical-plan/artifact/runtime/contiguous rows; application-static implementations may use equivalent exclusive reserved buffers. Stage B chooses production representation and Stage C measures it.

### 11. TD owns one shared bounded Basic semantic engine

TD owns resumable borrowed Basic validation semantics and fixed-width issue location. Synchronous `Thing::validate_with_level(Basic)` must converge on the same semantic engine. Extension resource census may traverse extension JSON structure without claiming unknown-extension semantic validation.

### 12. Cancellation remains above TD

The linear admission captures its cancellation source once. The outer lifecycle checks it before traversal, bounded TD/Planning steps, reservation/reconciliation transitions, and later publication transfer. TD receives no Core cancellation type.

## ADR-0013 impact

- **Foundation** — affected by typed-ingestion applicability, new typed-TD work class/row, and potentially a shared paired-work reservation helper. No production change is admitted yet.
- **TD** — affected by the future bounded shared Basic engine/cursor.
- **WP-200-CONSUMER-PROPERTY-READ-PLANNING** — **must reopen**. The raw public Consumer Planning contract cannot be reaffirmed behind a Servient wrapper.
- **WP-300-CONSUMER-PROPERTY-READ-BINDING** — **affected; explicit reaffirmation required**. Current Stage-A work demonstrates the existing one-budget compiler SPI can remain unchanged through a child-budget adapter. If Stage-B migration requires any Core public/source change, WP-300 escalates to reopen before implementation.
- **WP-300 completion evidence** — affected; must prove same complete-registration entry supplies compiler + identity and no alternate compiler path bypasses admitted Planning.
- **shared Producer Planning surface/evidence** — transitively affected by mandatory WP-200 public migration and must be explicitly dispositioned.
- **future Consumer WP-400 tranche** — not yet admitted; remains blocked on migrated 0063 + 0062 prerequisites.

Persistent Consumer execution-registration pinning after publication remains the separate prerequisite already identified by 0062 and is not absorbed here.

## Governance stages

### Stage A — before DECIDED

Fresh independent review judges whether the four current-head artifacts prove the architecture is constructible: borrowed source/cursor, linear Pending ownership, same snapshot/registration derivation, plan-build lease ownership and rollback, complete compiler bounds, no checked-policy applicability hole, real-SPI paired work metering, concrete modeled state fit, typed-TD work semantics, and correct WP-200/WP-300 impact.

Stage A does not require production TD traversal, final Servient storage, concurrent global ledgers, final cancellation API, publication, or real binding runtime evidence.

### Stage B — before production implementation admission

Only after fresh Stage-A acceptance may 0063 become `DECIDED`. Then authoritative migration must formally reopen WP-200 Consumer Planning; impact-review/reaffirm or reopen WP-300; migrate Foundation resource/work/accounting authority; migrate TD bounded Basic authority; migrate Planning public contract; establish actual Servient admission owner/storage/cancellation contract; establish the upstream 0062 unpublished-build lease contract; and obtain independent ADR-0013 production admission for every affected tranche.

### Stage C — post-implementation completion evidence

Only admitted implementation must prove invalid-input exclusion, no substitution across Pending, same-registration compiler source, actual resource-limit failures and rollback, lifetime/per-step compiler work under replenishment, borrowed-source zero retention, production Host/static layout measurements, synchronous/incremental Basic equivalence, cancellation/failure cleanup, no complete TD retention, and current reopened/reaffirmed evidence.

## Relationship to 0062

An accepted/migrated 0063 supplies 0062 only the validated input/Planning boundary, same-registration compiler derivation, complete compiler-bound/resource ownership, and the required opaque unpublished-build lease contract. It does not own final aggregate plan-set publication or persistent execution-registration pinning.

0062 remains blocked while 0063 is `DISCUSSING`.

## Merge / transition condition

This document may squash-merge while `DISCUSSING` only as a durable investigation record.

It may become `DECIDED` only after a fresh independent review accepts one exact current head containing all four Stage-A artifacts. `DECIDED` still does not authorize production Rust implementation.

It becomes `MIGRATED` only after accepted conclusions are projected into Foundation/TD/Planning/Servient authority and ADR-0013 impact/admission records.