# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible, bounded `Thing -> validated Planning input` boundary for the v5.1 Consumer Property Read path without reactivating broad deferred validation/codec scope

## Scope

Workspace topic 0062 established that the missing Consumer plan-set handoff cannot be closed while `PlanBuildInput` accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite needed to make that premise real:

- what proves that one exact `Thing` is valid for Planning;
- who owns and drives that validation transaction;
- how one immutable resource-policy projection and ledger owner remain bound to the whole operation;
- how typed-input structural applicability, source/input memory, validation temporary memory, diagnostics, peak live memory, contiguous allocation, and validation work are bounded and accounted;
- how Host and application-static profiles share the same semantic validation contract; and
- what impact this has on the completed WP-200 Consumer planning tranche and the Foundation/TD/Servient admission substrate.

This topic does **not** own:

- Consumer execution-registration pinning or execution-owner lifetime;
- the aggregate Planning -> Servient handoff from 0062;
- PlanId generation allocation;
- final static/Host consume-cancellation ownership;
- Consumer binding execution;
- WP-400 source implementation;
- broad payload/schema validator compilation or codec reuse;
- production Zenoh evidence; or
- the Consumer architecture-gate completion claim.

## Current repository facts

1. Public `Thing` is an ordinary cloneable value with public mutable fields. A caller can construct or mutate a `Thing` without preserving any prior validation fact.
2. `Deserialize for Thing` checks Serde/typed field shape but returns an ordinary `Thing`; it does not call `Validate`.
3. `ThingBuilder::build()` calls the current default `Validate::validate()` (`ValidationLevel::Basic`) but also returns the same ordinary `Thing` type. The validation result therefore has no durable type-level provenance.
4. `Thing::validate_with_level(Basic)` traverses required Thing/security/schema/affordance/Form/local-reference semantics, but `ExtensionMap::validate_with_level(...)` is currently a no-op. Basic semantic validation therefore does **not** traverse unknown extension JSON values. The current API is synchronous and does not accept `WorkBudget`, resource accounts, or an incremental cursor.
5. `clinkz_wot_planning::PlanBuildInput::new(...)` accepts `&Thing` and names the field `validated_td`; no type or constructor proves the claim.
6. The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` evidence explicitly assumes a validated TD build input and proves only that the build output survives destruction of that input.
7. Current public `Servient::consume(td: Thing)` performs neither TD validation nor v5 admission accounting. It immediately constructs legacy `ConsumedThing` state.
8. `ServientBuilder::resource_limits(...)` currently retains the supplied `ResourceLimits` only when the narrow Producer Property Read registration is also present. A Consumer-only Servient therefore has no durable policy owner today.
9. `ResourceLimits` is only the canonical low-level value snapshot. Active Foundation authority says it becomes a validated configuration authority only after an owning builder binds the executable role/profile/cell applicability set and rejects every illegal `None`.
10. Active `ADMIT-TXN-001` places validation inside the reserve-build-publish admission transaction and requires bounded cancellation checkpoints.
11. Active `ADMIT-MEM-001` requires distinct accounting for source/input bytes, phase-local temporary bytes, persistent document retention, persistent compiled-runtime bytes, diagnostics, cleanup ownership, current live bytes, peak simultaneously live bytes, and largest contiguous allocation.
12. `AdmissionLedger` already has distinct source, temporary, persistent-document, persistent-runtime, diagnostic, and cleanup accounts, but the active resource schema has no diagnostic-specific `ResourceKind`/ceiling suitable for constructing source-derived validation diagnostics.
13. The active resource schema contains `retained_source_bytes_*`, `admission_temporary_bytes_*`, `peak_live_bytes_*`, `largest_contiguous_allocation_bytes_max`, structural TD limits, `document_validation_work_units_max`, and raw-document JSON shape limits.
14. Current `WorkClass` has ten classes. None is explicitly a typed TD/admission-validation item class. `JsonSchemaNodes` is described as parsed JSON values and schema nodes visited; using it for every typed `Thing`/affordance/Form/reference traversal would be a semantic reinterpretation unless independently accepted.
15. TD already depends on Foundation with `default-features = false`, so a portable bounded validator can use Foundation budgets/types without reversing crate dependency direction.
16. ADR-0019 deliberately did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`. This topic must not import those deferred domains merely to establish admission provenance for a typed TD.

## Defect

The current architecture contains a trust gap:

```text
Thing
  -> PlanBuildInput::new(&thing, ...)
  -> field is called `validated_td`
  -> Planning assumes validation happened somewhere
```

Nothing in the public type system or Servient admission path establishes that premise.

Calling `thing.validate()` immediately before Planning would fix only the semantic check. It would still violate or leave ambiguous:

- operation-linear source identity;
- resource-policy provenance and applicability;
- bounded progress under `CONSTRAINED-WORK-001`;
- source/input memory accounting;
- temporary/diagnostic memory accounting;
- peak-live and contiguous-allocation accounting;
- cancellation/abort ownership during a long validation traversal; and
- static-profile manual-progress parity.

Conversely, introducing a marker type that can be constructed after an unbounded `validate()` call would prove provenance while leaving the admission transaction nonconforming.

The closure therefore needs **one linear bounded operation** that captures the exact source, policy, ledger ownership, validation level, and progress state from start through validated Planning use.

## Independent review history

### Review 1 — REQUEST CHANGES

The first independent review accepted the trust-gap diagnosis and WP-200 impact direction but identified four blockers:

1. resumable validation accepted a fresh `&Thing` and account/policy view on every step, allowing source substitution/mutation or policy/account rotation across `Pending` boundaries;
2. retaining raw `ResourceLimits` did not establish a validated role/profile/cell policy;
3. bounded diagnostics had neither an authoritative ceiling nor a non-allocating admission error representation; and
4. raw-document resource fields lacked an explicit typed-input applicability disposition.

It also corrected repository fact 4: Basic validation does not traverse extension contents because `ExtensionMap::validate_with_level` is currently a no-op.

The topic remains `DISCUSSING`.

## Current candidate boundary

The following is the only current investigation candidate. It is not implementation authority.

### 1. One linear validation/admission operation captures source, policy, and ledger at start

A resumable validator must not accept a new source or accounting context on every step.

The first-proof operation starts only after Servient/static composition has produced one **validated resource-policy handle**. Start captures, exactly once:

- the exact source `Thing` ownership/borrow used by this admission;
- `ValidationLevel::Basic`;
- one immutable validated resource-policy snapshot/handle;
- one generation-bearing admission-ledger owner and its distinct accounts;
- the deterministic typed-source census/validation cursor state; and
- the admission cancellation view/source required by the eventual owner.

Provisional semantic shape:

```rust
ConsumerValidationTxn<Source, State> {
    source: Source,
    level: ValidationLevel,
    policy: ValidatedResourcePolicy,
    ledger: AdmissionLedger,
    state: State,
}

ValidationStep<Txn> = Pending(Txn) | Complete(Txn) | Failed { diagnostic, txn };
```

The exact Rust generics/layout are not frozen. The required ownership rule is:

- `step` receives the **owned transaction** plus `&mut WorkBudget` and the admitted cancellation snapshot/checkpoint input only;
- `step` does not receive `&Thing`, `&ResourceLimits`, a replacement policy handle, or a replacement ledger/account view;
- a `Pending` result returns the same operation ownership with the same source identity, policy snapshot, owner generation, level, and prior charges;
- source mutation cannot coexist with a live immutable borrow, and an owned Host source cannot be substituted while the transaction exists;
- policy/account switching across steps is structurally impossible rather than detected heuristically; and
- abort/drop releases only state owned by that same captured ledger owner.

Evidence must include attempted source substitution, safe mutation while pending, validated-policy replacement, and ledger-owner/account replacement. Those operations must be unrepresentable on the admitted API or deterministically rejected before progress.

### 2. Validation completion returns a validated transaction state, not a self-referential proof value

A Host transaction may own its `Thing`; a static transaction may borrow caller-owned storage. Returning a standalone borrowed proof from an owning transaction would create an avoidable self-reference problem.

The candidate therefore makes validation provenance a **state of the linear transaction**.

After `Complete`, the transaction exposes an opaque TD-owned borrowed view only while the completed transaction/source remains alive:

```rust
impl ValidatedConsumerInput {
    pub fn validated_thing(&self) -> ValidatedThingView<'_>;
}
```

Required semantics:

- external crates cannot forge `ValidatedThingView`;
- the view borrows the exact source captured by the transaction;
- the view records/proves the admitted validation level privately;
- Planning accepts the view rather than a bare `&Thing` on the target path;
- Planning cannot persist the borrowed view into the compiled plan set;
- after Planning has copied every admitted immutable fact, the validation/source owner can be released at the earliest safe boundary; and
- no validated proof survives independently after the source owner is dropped.

This preserves a constructible Rust ownership model for both an owned Host source and a borrowed/static source.

### 3. A validated resource-policy handle is a prerequisite, not raw `ResourceLimits`

The first source census performs no externally influenced traversal until composition has produced a validated policy projection.

The provisional Foundation-owned/checked handle, name not frozen, must bind at least:

- one resource-schema revision;
- capability role `consumer`;
- first-proof capability/domain `Consumer Property Read one-shot`;
- execution/profile cell (`Host` or application-static/constrained);
- ingestion representation (`TypedThing` for this topic);
- named profile or application-defined value origin; and
- the exact immutable `ResourceLimits` snapshot/value digest used by the transaction.

Construction rejects every `None` whose field is applicable to that projection. `ResourceLimits::new/try_new`, cloning, or retention alone is not policy validation.

Host `GatewayDefaultV1` may be used only through the checked projection, not because the profile id by itself is authority. Application-defined policies must provide every field applicable to the selected projection.

The operation captures this validated handle/snapshot at start. It cannot swap to a different limits value, profile, role, cell, or applicability projection after any census/validation progress.

### 4. Typed input gets an explicit applicability projection

`consume` receives a typed `Thing`, not a raw JSON byte source. The validated policy must therefore bind an **ingestion representation** rather than pretending every raw-document field remains directly measurable.

The candidate disposition is:

| Resource field family | `TypedThing` first-proof disposition |
| --- | --- |
| `document_bytes_max` | raw-source-only; typed non-applicability must be made explicit by the checked applicability/schema projection before source admission. The engine must not reserialize merely to invent this value. |
| `json_nesting_depth_max` | enforced against a deterministic canonical typed-tree projection of the `Thing`, including nested `serde_json::Value` extension values. It does not claim to reproduce original raw syntax. |
| `json_members_per_object_max` | enforced against each object/map in the canonical typed-tree projection. |
| `json_array_items_max` | enforced against each vector/array in the canonical typed-tree projection, including extension arrays. |
| `json_value_nodes_per_document_max` | enforced against total nodes in the canonical typed-tree projection, including extension values. |
| typed TD counts such as affordances/forms/schema/security limits | enforced directly from the typed representation. |
| string/extension byte limits | enforced from the owned typed values under the accepted source-footprint representation. |

The canonical typed-tree projection is a resource census, not semantic validation. In particular, Basic validation may continue to ignore unknown extension semantics while the census still traverses extension `serde_json::Value` trees to enforce structural/resource limits.

If active schema applicability cannot legally mark `document_bytes_max` typed-nonapplicable without a schema revision, this topic requires that explicit Foundation applicability/schema revision before source admission. It must not silently reinterpret document bytes as heap bytes.

### 5. Typed-source census and semantic Basic validation are distinct phases

Before Planning, the linear operation performs a bounded typed-source census and then bounded semantic Basic validation.

The census establishes/charges applicable facts including:

- the accepted source/input footprint representation;
- canonical typed-tree depth/member/array/node limits;
- string/extension owned bytes under the accepted representation;
- affordance/Form/schema/security collection counts;
- largest contiguous allocation relevant to engine-owned/accounted source state; and
- current/peak source contribution while validation and later Planning temporary state overlap it.

Semantic validation then enforces the existing Basic TD rules without pretending extension values are semantically validated.

Both phases:

- progress from cursor state captured in the linear transaction;
- charge before every bounded collection/reference/tree visit;
- perform no semantic progress when the next required work class is exhausted;
- return the same source/policy/ledger ownership on `Pending`;
- observe cancellation at admitted bounded checkpoints; and
- publish nothing on failure/cancellation.

### 6. Source-memory representation remains a constructibility decision

`ADMIT-MEM-001` requires the physically live engine-owned/accounted representation to be measured or reserved, not just semantic item counts.

An arbitrary already-materialized Rust `Thing` contains `String`, `Vec`, `BTreeMap`, and nested `serde_json::Value` allocations. `size_of::<Thing>()` is insufficient, and portable exact allocator/node overhead is not obviously available from these public containers.

Closure must therefore independently accept one constructible ownership/measurement branch:

1. **accounted owned input** — a TD-owned source-footprint rule/envelope proves a conservative non-underestimating bound for every engine-owned allocation category before the by-value source becomes admission state; or
2. **borrowed external input** — the target admission API borrows caller-owned `Thing` storage for census/validation/Planning and accounts only engine-owned/exclusively-reserved state while enforcing all typed structural limits on the borrowed input; or
3. another measured representation that satisfies `ADMIT-MEM-001` without depending on undocumented allocator internals.

The prior preference to preserve `Servient::consume(td: Thing)` is therefore only a compatibility hypothesis. If a by-value arbitrary `Thing` cannot be accounted without undercounting, API impact review must prefer a constructible borrowed/accounted input boundary over preserving the old facade shape.

A serialization-based approximation is not accepted merely because it is easy to compute.

### 7. Admission validation uses a bounded non-source-owning diagnostic representation

The existing `ValidateError` is appropriate for ordinary authoring convenience but is not a safe admission-progress carrier: its variants allocate source-derived `String`s and `Multiple(Vec<ValidateError>)` can allocate a variable collection.

The bounded admission validator therefore uses a separate TD-owned fixed/bounded issue projection, provisionally:

```rust
ValidationIssue {
    kind: ValidationIssueKind,
    location: ValidationLocation,
    // fixed-width ordinals/field ids; no cloned source strings in first proof
}
```

The first proof records deterministic field/affordance/Form/reference coordinates or stable field ids rather than cloning arbitrary source names/messages. Ordinary `ValidateError` formatting may remain for the synchronous authoring API and may be produced outside the admitted bounded driver when appropriate; it is not the resumable transaction's failure payload.

The validator must determine the issue first without allocating source-derived diagnostic storage. It then pre-charges any admitted retained diagnostic bytes before constructing/storing them.

### 8. Diagnostic accounting requires an explicit Foundation ceiling

`AdmissionLedger` has a diagnostic account, but the exhaustive active schema currently has no diagnostic-specific `ResourceKind` that can truthfully supply its validation failure ceiling.

The current candidate therefore requires an additive Foundation resource-schema projection before source admission, provisionally one per-operation semantic capacity such as:

`admission_diagnostic_bytes_per_operation_max`

The exact stable field name is not frozen, but the accepted row must define:

- Foundation as semantic owner;
- bytes as unit;
- admission operation as scope;
- first-proof Consumer applicability plus any justified broader `all` applicability;
- zero semantics;
- default/profile values and constrained reference value;
- its `ADMIT-MEM-001` / `RES-LIMIT-001` ownership; and
- the exact reservation/diagnostic construction point.

If the primary diagnostic cannot be charged, `RES-LIMIT-002` requires a fixed-size, non-allocating fallback that still names the diagnostic resource category, configured limit, safely known requested amount, and admission phase. Diagnostic exhaustion must not trigger recursive diagnostic allocation or erase the original resource category.

Evidence must include diagnostic ceiling exhaustion independently from source, temporary, and work exhaustion.

### 9. Distinct admission accounts remain distinct

The expected phase/account model is:

| Phase | Source/input | Temporary | Persistent document | Persistent runtime | Diagnostic | Cleanup | Peak/contiguous |
| --- | --- | --- | --- | --- | --- | --- | --- |
| policy validation/composition | no source traversal yet | checked-builder local only | none | durable validated policy owner | bounded policy diagnostic | none | composition scope |
| typed source census | measured/observed under accepted source branch | bounded cursor/census scratch | none | none | fixed/bounded issue | none | source + census overlap |
| Basic validation | remains live/owned or externally borrowed as accepted | validation cursor/scratch | none | none | fixed/bounded issue | none | source + validation overlap |
| Planning preflight/build | remains live until safe drop | Planning/child cursors | none unless later explicitly retained | separately reserved compiled runtime | separately bounded | pure unpublished abort only | source + planning + reserved/runtime overlap |
| Frozen/Published | source released/borrow ended at earliest safe boundary | released | none for first Consumer proof | exact committed plan-set footprint | retained only if separately admitted | lifecycle-owned | published live footprint |

The first Consumer proof retains no complete TD source document after Planning has copied all admitted immutable facts.

### 10. Consumer resource policy survives independently of Producer setup

A Consumer admission transaction cannot inherit a policy that disappears when no Producer Property Read registration is configured.

The eventual Servient composition must retain a **validated Consumer-capable policy handle/snapshot** independently of `HostPropertyReadOwner`.

`ServientBuilder::resource_limits(...)` retaining raw values is not sufficient. The owning builder/projection must validate role/profile/cell/ingestion applicability before producing the Servient/static owner used by Consumer admission.

### 11. Work-class impact must be reviewed, not relabeled

The active resource schema already has `document_validation_work_units_max`, but current `WorkClass` has no clearly named typed-validation traversal class.

Three directions require independent review:

1. prove that an existing class already owns the exact typed census/validation traversal semantics without reinterpretation;
2. add a narrow `ValidationItems` class; or
3. add a more general typed `AdmissionItems` class that covers non-specialized bounded admission traversal while URI/security/binding/cleanup work continues to charge specialized classes.

The candidate prefers an additive typed admission/validation class over silently reusing `JsonSchemaNodes`, but the exact class name/scope remains undecided.

Any Foundation extension must preserve append-only stable enum ordering, zero-budget no-progress, and existing caller semantics. It triggers ADR-0013 impact review of Foundation and of the completed WP-200 child path where current Planning start performs uncharged typed work.

### 12. Broad deferred validation/codec authority remains inactive

This prerequisite does not require validator compilation caches, payload-schema validator reuse, codec pipelines, or broad response validation.

The active authority already requires validation as part of admission (`ADMIT-TXN-001`) and defines TD/resource/work bounds. The hypothesis remains that a bounded Basic typed-TD provenance operation is a constructibility refinement of those active requirements, not activation of `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`.

If independent review concludes that an inactive validation identity owns unavoidable behavior here, this topic must stop for narrow domain-entry review rather than smuggle it in.

## Foundation / TD / Servient impact now known

This topic is no longer plausibly a documentation-only adapter around existing primitives.

Before source admission, impact review must explicitly disposition:

- **Foundation resource policy projection** — current raw `ResourceLimits` is not a validated role/profile/cell policy;
- **Foundation resource schema** — typed ingestion applicability for raw-document fields and a diagnostic ceiling/resource kind may require additive/revisioned projection;
- **Foundation work classes** — typed admission/validation traversal currently lacks an obviously correct class;
- **TD validation** — the admission driver needs deterministic non-allocating issue discovery and resumable traversal while preserving existing synchronous `Validate` behavior;
- **Servient composition** — Consumer must inherit one validated policy independently of Producer registration setup; and
- **Host/static input ownership** — the accepted source-footprint branch determines whether the target API owns or borrows the typed input.

These are all inside this claim because they jointly establish one bounded validated-input transaction. Consumer execution-registration lifetime remains separate.

## WP-200 impact

The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` tranche cannot be declared unaffected.

At minimum the impact review must examine:

- `PlanBuildInput::new` currently accepts a raw `&Thing`;
- existing fixtures call it with ordinary Thing values they regard as validated;
- completion evidence claims the validated TD can be dropped after build;
- changing Planning to require `ValidatedThingView` is a public Planning input-contract change even if the exact compiler algorithm and owned output remain unchanged; and
- current child start/step work charging may need reopening if the accepted Foundation class changes how typed Planning work is debited.

WP-200 may be reaffirmed only if an accepted additive adapter preserves its exact-coordinate compiler behavior and evidence meaning. Otherwise the affected tranche/evidence reopens under ADR-0013 before 0062 relies on it.

## Required evidence before this topic can become DECIDED

An accepted closure must require at least:

- a deserialized or manually mutated invalid `Thing` cannot reach Planning as validated input;
- `ThingBuilder::build()` returning an ordinary `Thing` is not itself durable provenance;
- one started transaction cannot substitute a second `Thing` after `Pending`;
- safe source mutation cannot coexist with the transaction/view's immutable source ownership;
- a transaction cannot switch `ResourceLimits`, validated policy projection, profile/cell, or ledger owner/account after progress;
- raw `ResourceLimits` with illegal `None` cannot start Consumer census;
- Consumer-only Servient/static composition owns a validated policy before any source traversal;
- Planning cannot construct target `PlanBuildInput` from a bare `&Thing`;
- zero typed-validation work produces no corresponding traversal progress;
- step partitions produce identical validation result and first deterministic issue;
- Basic semantic validation does not falsely claim extension semantic traversal, while resource census still bounds nested extension values;
- every raw-document resource field has an executable `TypedThing` applicability/measurement disposition;
- source ownership/footprint evidence proves the accepted owned/borrowed representation without allocator-underestimation;
- source census and validation respect source, temporary, peak-live, contiguous-allocation, and structural limits;
- primary validation diagnostic construction is pre-charged;
- diagnostic ceiling exhaustion returns the bounded fallback naming the diagnostic resource category without recursive allocation;
- oversized source, structural-limit, validation, diagnostic, and work failures publish nothing and release private engine-owned state/accounts idempotently;
- no complete source TD is retained by the first Consumer published plan set;
- Host and application-static driving use the same portable typed census/Basic validation semantics;
- no broad deferred validation/codec capability becomes active implicitly;
- Foundation schema/work-class/policy dispositions are recorded explicitly; and
- ADR-0013 impact disposition is recorded for WP-200 and every affected Foundation/TD/Servient tranche.

## Relationship to 0062

0062 remains blocked while this topic is `OPEN` or `DISCUSSING`.

A DECIDED/MIGRATED outcome from this topic gives 0062 only these facts:

1. what completed linear transaction/view Planning receives instead of an assumed validated `&Thing`;
2. how the exact source, validated policy snapshot, and ledger owner remain bound across resumable validation;
3. how typed source applicability/memory overlaps later Planning admission and when the source ownership/borrow ends;
4. what Foundation policy/schema/work/accounting primitives are authoritative; and
5. the exact WP-200 impact disposition.

0062 must not absorb this topic's TD validator, policy validation, diagnostic, or source-memory design back into its local aggregate closure.

Consumer execution-registration pinning remains a separate later claim.

## Merge condition

This document may merge while `DISCUSSING` only as an investigation record after independent review of the current candidate boundary.

It may become `DECIDED` only after a fresh independent review accepts a constructible linear validation provenance, validated-policy, typed-input applicability, source-accounting, diagnostic, and bounded-work model consistent with active v5.1 authority.

It becomes `MIGRATED` only after the accepted conclusion is projected into the appropriate TD/Foundation/Planning/Servient authority and ADR-0013 impact/admission records. No Rust source implementation is authorized by this workspace topic alone.
