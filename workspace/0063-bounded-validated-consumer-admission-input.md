# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible bounded `Thing -> validated Consumer Planning` admission boundary for the v5.1 Consumer Property Read path without activating broad deferred validation/codec scope.

## Scope

Workspace topic 0062 established that the Consumer plan-set handoff cannot be closed while Planning still accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite admission boundary:

- one immutable typed source and real validation provenance;
- one linear validation-to-Planning owner across every `Pending` boundary;
- one checked Consumer resource-policy projection for borrowed typed input;
- typed-input resource/work semantics and physical Host/static accounting;
- one stable complete-registration snapshot used consistently for compiler derivation;
- one upstream unpublished plan-build authority consumed by Planning;
- complete `BindingCompilerBounds` ownership and lifetime-work enforcement;
- the WP-200/WP-300 impact required by the corrected Planning boundary; and
- the evidence/admission ordering required before implementation.

This topic does **not** own:

- persistent Consumer execution-registration pinning after plan-set publication;
- final aggregate Planning -> Servient plan-set publication material owned by 0062;
- allocation algorithms for `PlanId`, plan slots, or plan-set generations;
- the final Host/static product cancellation API;
- Consumer binding execution;
- WP-400 Consumer source implementation;
- broad validator compilation/cache/codec reuse; or
- Consumer architecture-gate completion.

## Stable repository facts

1. `Thing` is an ordinary cloneable/mutable public value. Deserialization and `ThingBuilder::build()` return ordinary `Thing`; neither type proves durable admission validation provenance.
2. Current Basic TD validation is synchronous/unmetered, and `ExtensionMap::validate_with_level(...)` is a semantic no-op.
3. Public `PlanBuildInput` is `Clone + Copy` and accepts raw `&Thing`, registration input, and `PlanSetGeneration`.
4. Public `PlanCompiler::start/step` accept fresh input on each call.
5. `PropertyReadPlanCompiler` stores plan/target/binding/configuration/compatibility/registration/candidate/role facts in `self`, not its cursor.
6. Current registration lookup checks the indexed registration's artifact compatibility but does not itself prove full identity equality with the separately constructed compiler.
7. `BindingRegistrationIdentity::diagnostic_ordinal()` and `BindingCandidate::registration_ordinal()` are distinct domains.
8. `BindingCompilerBounds` declares artifact footprint, cursor bytes, temporary bytes, and a lifetime `WorkBudget`; current Property Read Planning retains only the artifact bound.
9. `PlanId` and `PlanSetGeneration` are distinct identities and current Planning combines both into artifact identity.
10. Raw `ResourceLimits` is not a validated role/profile/cell/representation policy.
11. Current raw document/JSON resource identities cannot silently acquire typed-Rust `Thing` semantics.
12. Active Foundation authority requires representation-specific physical source/temp/runtime/diagnostic/cleanup, current/peak, and contiguous accounting.
13. `WorkBudget::consume()` mutates one class only and does not atomically coordinate a separate lifetime allowance.
14. `WP-200-CONSUMER-PROPERTY-READ-PLANNING` and `WP-300-CONSUMER-PROPERTY-READ-BINDING` are both currently registered complete/admitted/current.
15. The completed WP-300 Consumer tranche already defines complete Host/static registration bundles containing one registration identity and one compiler component in the same validated registration.
16. ADR-0019 did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`.

## Defect

The old path admits several independently forgeable/substitutable inputs:

```text
ordinary Thing
  -> raw PlanBuildInput
  -> Planning trusts "validated_td"

input A -> start -> Pending -> input B -> step

compiler identity A
  + registration snapshot entry B
  -> compatibility-only acceptance may preserve an A/B mismatch

raw PlanId
  + independently supplied PlanSetGeneration
  -> artifact identity

compiler bounds
  -> cursor/temp/lifetime-work declaration
  -> current Planning retains only artifact bound
```

A later Servient wrapper cannot make the existing public Consumer Planning contract admitted-safe while those raw construction paths remain valid public Consumer entry points.

The corrected first proof therefore requires one linear typed admission owner, one stable borrowed complete-registration snapshot, and one opaque upstream unpublished plan-build authority.

## Independent review history

### Reviews 1-4 — REQUEST CHANGES

Those reviews established the need for borrowed immutable source validation, linear validation-to-Planning ownership, checked resource policy, mandatory typed-ingestion schema revision, hierarchical accounting, one shared Basic semantic engine, atomic lifetime/step charging, exact same-registration compiler derivation, explicit ordinal domains, WP-200 public-contract reopening, and representation-specific Host/static physical accounting.

### Review 5 — REQUEST CHANGES

Review 5 additionally required complete `BindingCompilerBounds` cursor/temp/lifetime-work ownership, separation from PlanId allocation authority, non-circular evidence staging, and explicit snapshot-vs-diagnostic ordinal semantics.

### Review 6 — REQUEST CHANGES

Review 6 accepted the Stage-A/B/C split, ordinal separation, mandatory WP-200 reopening, and complete `BindingCompilerBounds` direction, but found four remaining pre-decision gaps:

1. the plan lease authorized `PlanId` without binding the independently supplied `PlanSetGeneration`;
2. Stage A described constructibility evidence but the reviewed head contained no compile/model fixture;
3. resource migration, checked policy, and Host/static storage remained checklists rather than complete definitions; and
4. typed TD work-class/unit semantics plus explicit transitive `WP-300-CONSUMER-PROPERTY-READ-BINDING` impact were missing.

The current head addresses those findings through the two Stage-A artifacts listed below. The topic remains `DISCUSSING` pending fresh independent review.

## Stage-A artifacts at the current head

Two non-production artifacts are part of the candidate and must be reviewed with this topic:

1. `tools/design-check/tests/consumer_admission_stage_a.rs`
   - compiled by the existing workspace test path;
   - models borrowed TD cursor topology and linear validation -> Planning typestate;
   - models stable external registration snapshot ownership and ephemeral Planning input reconstruction;
   - models same-entry compiler derivation and unequal ordinal domains;
   - models one opaque unpublished plan-build lease binding both `PlanId` and `PlanSetGeneration`;
   - captures complete `BindingCompilerBounds`, reserves memory before start, and retains compiler lifetime work;
   - proves lifetime+step pair-charge failure atomicity; and
   - defines/measures separate concrete Host/static enclosing storage models.

2. `workspace/0063-stage-a-resource-policy-layout.md`
   - freezes the candidate resource-schema migration table;
   - freezes the complete checked Consumer policy projection;
   - freezes `WorkClass::TypedTdAdmissionItems` and its exact unit mapping;
   - freezes the physical layout attribution rule used by the executable model;
   - freezes WP-200 reopening and WP-300 reaffirmation-required impact dispositions; and
   - maps each Stage-A constructibility claim to executable evidence.

Neither artifact is production implementation authority.

## Current candidate boundary

### 1. First proof borrows caller-owned typed input

The first proof captures `&'td Thing`.

Consequences:

- safe source mutation cannot coexist with the live admission borrow;
- TD cursor state may borrow stable caller-owned storage without creating a movable self-reference;
- borrowed source contributes zero engine-retained-source bytes;
- typed structural/resource/work limits still apply to traversal; and
- the borrow ends only after Planning has copied every fact required beyond admission.

The legacy by-value `Servient::consume(Thing)` facade is not an architectural constraint. Any later owned-input convenience path requires separately admitted physical accounting.

### 2. Host/static composition owns one immutable complete-registration snapshot

Host Servient or application-static composition owns the complete-registration snapshot. One admission borrows it for validation + Planning.

```text
Host/static composition
    owns CompleteRegistrationSnapshot
             |
             +---- immutable borrow ----> ConsumerAdmissionTxn<'reg>
```

The transaction never owns the snapshot and simultaneously stores a reference into that owned value.

When the existing generic Planning code is temporarily reused during migration, `PlanBuildInput` is reconstructed only as an ephemeral private call value from the same borrowed source, same borrowed snapshot, and the identities held by the same unpublished build lease. It is not stored across calls and is never caller-supplied on the admitted path.

### 3. Snapshot ordinal and diagnostic ordinal remain distinct

- **snapshot ordinal** is the index/slot in the captured immutable registration snapshot and becomes `BindingCandidate::registration_ordinal()`;
- **diagnostic ordinal** is `BindingRegistrationIdentity::diagnostic_ordinal()` and is reporting-only.

They are not required to match.

The Stage-A fixture uses snapshot ordinal `3` and diagnostic ordinal `17` and proves that entry `3`, not `17`, supplies compiler bounds/start.

### 4. One opaque unpublished build lease binds `PlanId` + `PlanSetGeneration`

0063 no longer consumes a PlanId-only token.

The eventual 0062 / Servient plan-set identity owner supplies one opaque, move-only authority:

```text
UnpublishedPlanBuildLease {
    exact PlanId,
    exact PlanSetGeneration,
}
```

There is no admitted constructor from either raw identity and Consumer Planning accepts no independent `PlanSetGeneration`.

Conceptually:

```text
0062 / Servient plan-set identity authority
       -> reserve one unpublished build
       -> UnpublishedPlanBuildLease(PlanId + PlanSetGeneration)

0063 validated admission
       + same lease
       -> sealed Consumer Planning
```

0063 freezes only this authority boundary. It does not choose how the upstream owner allocates/reuses plan slots or generations. Abort returns/releases the reservation to that upstream owner; successful freeze transfers the exact pair into the later 0062 lifecycle.

### 5. The admitted Planning constructor shape is semantically frozen now

The exact future public Rust names remain a reopened WP-200 migration choice, but the admitted safe constructor semantics are no longer deferred.

Equivalent shape:

```text
ValidatedConsumerAdmission::enter_planning(
    self,
    selected_snapshot_ordinal,
    UnpublishedPlanBuildLease,
) -> Result<ConsumerPlanningTxn, AdmissionFailure>
```

It does **not** accept raw `Thing`, raw `PlanBuildInput`, raw `PlanId`, raw `PlanSetGeneration`, external `PropertyReadPlanCompiler`, external `BindingRegistrationIdentity`, or a replacement registration snapshot.

The constructor indexes the already-borrowed snapshot, obtains identity and compiler execution from that same complete registration entry, and derives the sealed compiler authority internally.

A reopened WP-200 implementation may choose different type names/generic factoring, but no accepted alternative may reintroduce one of the independent inputs above.

### 6. Same-registration derivation is stronger than compatibility equality

For the selected snapshot entry, compiler identity/execution must originate from that same complete registration.

Before compiler `bounds()`/`start()`, binding id, binding generation, configuration digest, and artifact compatibility must correspond to that exact entry. Equal compatibility between registration A and registration B does not permit using A's identity with B's compiler.

The executable Stage-A fixture constructs competing equal-compatibility registrations and proves only the selected snapshot entry receives `bounds/start`.

### 7. Complete `BindingCompilerBounds` becomes owned Planning admission authority

For the selected registration and exact compiler input:

```text
bounds() exactly once
  -> capture artifact footprint
  -> capture cursor bytes
  -> capture peak temporary bytes
  -> capture complete compiler lifetime WorkBudget
  -> reserve cursor/temp/artifact physical capacity
  -> only then compiler.start()
```

A memory admission failure after `bounds()` but before `start()` leaves `start()` uncalled.

The complete declaration remains owned until each corresponding resource/work lifetime ends and is reconciled/released on completion, failure, or abort.

### 8. Validation and compiler lifetime work have distinct authorities

Typed TD census/Basic admission uses the proposed append-only Foundation class:

```text
WorkClass::TypedTdAdmissionItems
```

Its exact unit mapping and the new `typed_td_admission_work_units_max` resource row are defined in `workspace/0063-stage-a-resource-policy-layout.md`. It is not a reinterpretation of `JsonSchemaNodes`.

Binding compiler work continues to use the exact classes declared by `BindingCompilerBounds::work()`.

For both lifetime authorities, a work unit is admitted only through one failure-atomic pair operation:

```text
preflight lifetime remaining
+ preflight caller step WorkBudget
  -> any failure: neither changes
  -> success: both decrement once
  -> then work begins
```

Caller replenishment of a later step budget cannot replenish either lifetime authority.

### 9. Resource-schema migration and checked policy are fully dispositioned

The companion Stage-A definition artifact contains the first-proof migration table for every existing document/schema/input/memory row exercised by `TypedThingBorrowed` admission.

The candidate adds only the minimal typed identities:

- `typed_td_nesting_depth_max`;
- `typed_td_members_per_map_max`;
- `typed_td_items_per_sequence_max`;
- `typed_td_value_nodes_per_thing_max`;
- `typed_td_string_bytes_per_thing_max`; and
- `typed_td_admission_work_units_max`.

Raw document/json rows are not serialization proxies for a materialized Rust `Thing`.

The companion also defines the complete non-optional `TypedThingBorrowedConsumerPolicyV1` projection. A raw `ResourceLimits` value cannot start traversal until schema/profile/role/cell/ingestion applicability is bound and every applicable value is present.

### 10. Physical Host/static accounting is defined from real enclosing storage

The Stage-A compile fixture defines separate concrete `#[repr(C)]` Host/static enclosing storage models and a real `FailureSlot` union containing fixed `ValidationIssue` and actual `CoreError` alternatives.

For each model, `size_of`, `align_of`, and `offset_of` partition the one enclosing allocation/slot into structural, state, diagnostic, accounting, and compiler regions.

The attribution rule covers every byte exactly once, includes padding, gives the diagnostic account a real physical region, charges current/peak live from the enclosing storage once, and measures largest contiguous allocation from the whole enclosing allocation/exclusive static slot rather than a sum of field sizes.

The Stage-A types prove constructibility only. Stage B migrates the accepted rule to real Servient storage; Stage C verifies actual production layouts.

### 11. TD Basic semantic engine remains single-source

TD owns the borrowed resumable Basic semantic engine/check graph and a fixed-width validation issue location.

The synchronous `Thing::validate_with_level(Basic)` path must converge on the same engine. During migration, differential tests prove exact success/failure and first-issue agreement until delegation becomes structural.

Extension resource census may traverse extension JSON values without claiming extension semantic validation.

### 12. Cancellation remains above TD

The linear admission captures its Host/static cancellation source once. The outer owner checks cancellation before first traversal, each bounded TD/Planning step, resource transition, and publication transfer.

TD receives no Core cancellation type. The final user-facing cancellation API remains later lifecycle work.

## ADR-0013 impact disposition

The companion Stage-A definition records the complete table. The key completed Consumer tranches are explicitly dispositioned here:

- `WP-200-CONSUMER-PROPERTY-READ-PLANNING` — **must reopen**. A Servient wrapper cannot reaffirm the frozen raw public Consumer Planning contract.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING` — **affected; explicit reaffirmation required before 0063/0062 may rely on it**. Its existing complete Host/static registration already owns identity + compiler component in one validated bundle, so this candidate does not presently require WP-300 source/public API changes. If Stage-B migration proves such a change necessary, the tranche escalates to reopen under ADR-0013 before implementation.
- `consumer-property-read-binding-execution` evidence — affected by the same reaffirmation and must prove that sealed Planning obtains identity and compiler component from the exact same complete registration entry.
- shared Producer Planning API/evidence — affected by the mandatory WP-200 public API migration and receives explicit transitive review.
- no completed Consumer WP-400 tranche exists to reopen; its future admission remains blocked on migrated 0063 + 0062 prerequisites.

## Evidence and governance ordering

### Stage A — required before 0063 may become DECIDED

Stage A proves architecture constructibility, not production completion.

Current-head required artifacts now exist:

- `tools/design-check/tests/consumer_admission_stage_a.rs`;
- `workspace/0063-stage-a-resource-policy-layout.md`; and
- this topic.

Fresh independent review must decide whether those artifacts together prove:

- borrowed external source/cursor topology is Rust-constructible;
- linear typestate removes caller substitution across `Pending`;
- registration snapshot ownership is external and stable;
- ephemeral Planning input reconstruction does not self-borrow owned snapshot storage;
- snapshot/diagnostic ordinal domains are distinct;
- one opaque build lease binds both `PlanId` and `PlanSetGeneration`;
- same-entry compiler derivation prevents equal-compatibility identity cross-wire;
- complete `BindingCompilerBounds` memory/work ownership is constructible;
- validation and compiler lifetime+step debits are failure-atomic;
- the typed WorkClass/unit and schema migration are semantically exact;
- the checked policy has no applicability holes;
- Host/static physical layout attribution is coherent/non-overlapping; and
- WP-200 reopening + WP-300 reaffirmation-required impact are sufficient and correctly scoped.

A Stage-A model may be non-production. It cannot claim source implementation completion.

### Stage B — required before production implementation admission

After independent acceptance of Stage A and transition to `DECIDED`, migration must:

- formally reopen `WP-200-CONSUMER-PROPERTY-READ-PLANNING`;
- explicitly impact-review/reaffirm `WP-300-CONSUMER-PROPERTY-READ-BINDING` or reopen it if migration requires a Core API/source change;
- migrate the accepted Foundation schema/policy/accounting/work authority;
- migrate the TD bounded shared Basic engine authority;
- migrate the WP-200 public Consumer Planning contract;
- establish actual Servient admission owner/storage/cancellation authority;
- establish the upstream 0062/Servient `UnpublishedPlanBuildLease` contract; and
- obtain independent ADR-0013 implementation admission for every affected production tranche.

### Stage C — post-implementation completion evidence

Only after admitted implementation exists must runtime evidence prove:

- invalid typed `Thing` cannot enter admitted Planning;
- source/policy/snapshot/build-lease/compiler/cancellation substitution is impossible across `Pending`;
- snapshot ordinal `3` and diagnostic ordinal `17` remain distinct;
- equal-compatibility registration cross-wire cannot progress;
- complete compiler bounds are reserved before start and reconciled on all terminals;
- replenished step budgets cannot exceed TD or compiler lifetime work;
- typed resource/policy limits, global hierarchy, peak, contiguous, and runtime ceilings are enforced;
- borrowed input contributes zero retained-source bytes;
- actual Host/static production layouts match their accepted attribution;
- Basic synchronous/incremental semantics agree;
- cancellation/failure publishes nothing and releases private ownership idempotently;
- no complete source TD survives into the first published Consumer plan set; and
- reopened/reaffirmed WP-200/WP-300/shared evidence is current.

## Relationship to 0062

0062 remains blocked while 0063 is `DISCUSSING`.

An accepted/migrated 0063 gives 0062 only these facts:

1. Consumer admission begins from borrowed immutable typed input under one checked policy;
2. validation and Planning form one linear non-substitutable admission chain;
3. Host/static composition owns the stable complete-registration snapshot;
4. ordinal domains are explicit and same-entry compiler derivation is enforced;
5. complete `BindingCompilerBounds` memory/work declarations are owned;
6. one upstream opaque unpublished build lease binds `PlanId` + `PlanSetGeneration` and is consumed by Planning;
7. WP-200 Consumer public Planning must migrate away from the raw admitted bypass;
8. WP-300 Consumer binding evidence must be explicitly reaffirmed against the same-entry source rule; and
9. Foundation/TD/Planning/Servient migration obligations are frozen before implementation admission.

0062 must not absorb 0063's TD validator, resource-schema migration, WP-200 reopening, or physical accounting design back into its local aggregate handoff.

Persistent execution-registration pinning after publication remains the separate second prerequisite already identified by 0062.

## Merge / transition condition

This document may squash-merge while `DISCUSSING` as a durable investigation record.

It may become `DECIDED` only after a fresh independent review accepts the current Stage-A trio at one exact head:

- this topic;
- `workspace/0063-stage-a-resource-policy-layout.md`; and
- `tools/design-check/tests/consumer_admission_stage_a.rs`.

`DECIDED` does not authorize production Rust implementation.

The topic becomes `MIGRATED` only after the accepted conclusion is projected into the relevant Foundation/TD/Planning/Servient authority and ADR-0013 impact/admission records.
