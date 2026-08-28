# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible, bounded `Thing -> validated Planning input` boundary for the v5.1 Consumer Property Read path without reactivating broad deferred validation/codec scope

## Scope

Workspace topic 0062 established that the missing Consumer plan-set handoff cannot be closed while `PlanBuildInput` accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite needed to make that premise real:

- what proves that one `Thing` is valid for Planning;
- who performs that validation;
- how source/input memory, validation temporary memory, diagnostics, peak live memory, contiguous allocation, and validation work are bounded and accounted;
- how Host and application-static profiles share the same validation semantics; and
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
4. `Thing::validate_with_level(Basic)` traverses security definitions/references, schema maps, properties, actions, events, top-level Forms, extensions, and related local references. The current API is synchronous and does not accept `WorkBudget`, resource accounts, or an incremental cursor.
5. `clinkz_wot_planning::PlanBuildInput::new(...)` accepts `&Thing` and names the field `validated_td`; no type or constructor proves the claim.
6. The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` evidence explicitly assumes a validated TD build input and proves only that the build output survives destruction of that input.
7. Current public `Servient::consume(td: Thing)` performs neither TD validation nor v5 admission accounting. It immediately constructs legacy `ConsumedThing` state.
8. `ServientBuilder::resource_limits(...)` currently retains the supplied policy only when the narrow Producer Property Read registration is also present. A Consumer-only Servient therefore has no durable resource-policy owner today.
9. Active `ADMIT-TXN-001` explicitly places validation inside the reserve-build-publish admission transaction and requires bounded cancellation checkpoints.
10. Active `ADMIT-MEM-001` requires distinct accounting for source/input bytes, phase-local temporary bytes, persistent document retention, persistent compiled-runtime bytes, diagnostics, cleanup ownership, current live bytes, peak simultaneously live bytes, and largest contiguous allocation.
11. The active resource schema already contains `retained_source_bytes_*`, `admission_temporary_bytes_*`, `peak_live_bytes_*`, `largest_contiguous_allocation_bytes_max`, structural TD limits, and `document_validation_work_units_max`.
12. Current `WorkClass` has ten classes. None is explicitly a typed TD/admission-validation item class. `JsonSchemaNodes` is described as parsed JSON values and schema nodes visited; using it for every typed `Thing`/affordance/Form/reference traversal would be a semantic reinterpretation unless independently accepted.
13. TD already depends on Foundation with `default-features = false`, so a portable bounded validator can use Foundation budgets/types without reversing crate dependency direction.
14. ADR-0019 deliberately did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`. This topic must not import those deferred domains merely to establish admission provenance for a typed TD.

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

- bounded progress under `CONSTRAINED-WORK-001`;
- source/input memory accounting;
- temporary/diagnostic memory accounting;
- peak-live and contiguous-allocation accounting;
- cancellation/abort ownership during a long validation traversal; and
- static-profile manual-progress parity.

Conversely, introducing a marker type that can be constructed after an unbounded `validate()` call would prove provenance while leaving the admission transaction nonconforming.

The closure therefore needs **both** a non-forgeable validated view and a bounded way to produce it.

## Candidate boundary

The following is an investigation candidate, not implementation authority.

### 1. Preserve the public Host `consume(Thing)` shape

The first-proof Host facade should not require application code to manufacture an internal validation token or change the W3C-style call into `consume(validated_thing)`.

`Servient::consume(td: Thing)` remains the convenience surface. Internally, the Servient must own/inherit a resource policy before it begins validation/planning and must drive the bounded admission substrate to completion before returning a handle.

The application-static profile may expose manual start/step ownership around the same portable validation substrate; it need not copy the Host synchronous driver shape.

### 2. TD owns a borrowed validation proof

Planning should no longer accept a bare `&Thing` as proof of validation.

The candidate introduces a TD-owned opaque borrowed proof, provisionally:

```rust
pub struct ValidatedThing<'a> {
    thing: &'a Thing,
    level: ValidationLevel,
    // private provenance only
}
```

Required semantics:

- external crates cannot forge the proof;
- the proof borrows the exact `Thing` that was validated;
- while the proof exists, ordinary Rust borrowing prevents safe mutation of that `Thing`;
- the proof owns no clone of the TD and no lossless source tree;
- the first Consumer proof requires exactly `ValidationLevel::Basic` unless independent review establishes a narrower already-authoritative level;
- Planning may read the validated TD only through this proof during build/preflight; and
- no validated proof survives after the source `Thing` is dropped.

The exact public name and constructor are admission details. What matters for closure is that `PlanBuildInput` cannot claim a raw `&Thing` is validated merely by naming the field that way.

### 3. Validation is bounded and resumable

The TD crate owns a portable incremental validation driver/cursor that is semantically equivalent, for the admitted level, to the existing `Thing::validate_with_level(...)` rules.

Provisional shape:

```rust
pub struct ThingValidationCursor { /* opaque deterministic traversal state */ }

pub enum ThingValidationStep<'a> {
    Pending(ThingValidationCursor),
    Complete(ValidatedThing<'a>),
    Failed { error: ValidateError, cursor: ThingValidationCursor },
}
```

The exact type layout is not frozen here. The contract is:

- `start` is side-effect free and does no hidden unbounded traversal;
- `step` receives `&Thing`, an owned cursor, the applicable resource policy/account view, and `&mut WorkBudget`;
- every collection/reference/schema/security visit is charged before the visit;
- zero available work for the next required class performs no semantic validation progress;
- insufficient work returns `Pending` with complete resumable ownership;
- failure returns the owned cursor/error needed for deterministic abort/drop handling;
- `abort`/drop releases validation-local temporary/diagnostic state without protocol cleanup; and
- step partitioning cannot change validation success/failure or the first deterministic failure classification.

The existing synchronous `Validate` API may remain a user convenience, but admission cannot use it as an unmetered substitute. The implementation should share semantic validation helpers rather than maintain two independently drifting rule sets.

### 4. Typed input source census precedes semantic validation

`consume(Thing)` receives an already-materialized typed TD rather than raw JSON bytes. The first admission phase therefore performs a bounded **typed source census** before Planning.

The census must establish and charge the applicable source/input facts required by active resource authority, including at least:

- source/input retained bytes for the admitted representation;
- string/extension owned bytes or their already-authoritative structural limits;
- affordance/Form/schema/security collection counts needed by the active TD limits;
- largest contiguous owned allocation relevant to the admitted representation; and
- current/peak live source contribution while validation and later Planning temporary state overlap it.

The census must not reserialize the TD merely to invent a raw-document byte count unless an independent review explicitly chooses that representation. `document_bytes_max` belongs naturally to raw-document ingestion; a typed `Thing` entry needs an exact declared measurement representation rather than pretending serialized bytes are still present.

A failed source census publishes nothing and drops/returns the private input according to the eventual facade contract.

### 5. Source measurement representation must be explicit

`ADMIT-MEM-001` requires the physically live representation to be accounted, not just a semantic item count.

The closure therefore must select one constructible TD-owned measurement rule for the typed `Thing` representation. Acceptable directions to review include:

- an exact TD-owned logical/allocated-footprint walk where every owned allocation category is representable portably;
- a conservative representation-specific bound whose over-accounting is explicit and testable; or
- a caller-provided pre-accounted storage envelope for constrained/static construction while Host `consume(Thing)` uses a measured owned-input adapter.

A rule that ignores `BTreeMap`/`Vec`/`String` backing allocations or treats `size_of::<Thing>()` as total source memory is not sufficient.

This is currently the hardest constructibility point in the candidate and must be challenged independently before DECIDED.

### 6. Distinct admission accounts remain distinct

The validated-input prerequisite does not collapse the later 0062 aggregate ledger into one ceiling.

The expected phase/account model is:

| Phase | Source/input | Temporary | Persistent document | Persistent runtime | Diagnostic | Cleanup | Peak/contiguous |
| --- | --- | --- | --- | --- | --- | --- | --- |
| typed source census | measured/charged | bounded cursor/census scratch | none | none | bounded failure only | none | source + census overlap |
| Basic validation | remains live/charged | validation cursor/scratch | none | none | bounded validation error | none | source + validation overlap |
| Planning preflight/build | remains live until safe drop | Planning/child cursors | none unless later explicitly retained | separately reserved compiled runtime | separately bounded | pure unpublished abort only | source + planning + reserved/runtime overlap |
| Frozen/Published | source released at earliest safe boundary | released | none for first Consumer proof | exact committed plan-set footprint | separately retained if admitted | lifecycle-owned | published live footprint |

The first Consumer proof should retain **no complete TD source document** after Planning has copied all admitted immutable facts. If a later feature intentionally retains source material, that is a separate persistent-document-retention decision and charge.

### 7. Resource policy must survive independently of Producer setup

A Consumer admission transaction cannot inherit a policy that disappears when no Producer Property Read registration is configured.

The eventual Servient composition must retain the configured `ResourceLimits` (or an admitted profile reference/value with equivalent semantics) independently of `HostPropertyReadOwner`.

This does not require this workspace topic to redesign the entire Servient builder. It does require any later source admission to treat Consumer resource-policy inheritance as a predecessor, not as a WP-400 afterthought.

### 8. Work-class impact must be reviewed, not relabeled

The active resource schema already has `document_validation_work_units_max`, but the current `WorkClass` enum has no clearly named typed-validation traversal class.

Three directions require independent review:

1. prove that an existing class already owns the exact validation traversal semantics without reinterpretation;
2. add a narrow `ValidationItems` class; or
3. add a more general typed `AdmissionItems` class that can later cover non-specialized bounded admission traversal while URI/security/binding/cleanup work continues to charge its specialized classes.

The candidate currently prefers **an additive typed admission/validation class over silently reusing `JsonSchemaNodes`**, but the exact class name/scope is not DECIDED.

Any Foundation extension must preserve append-only stable enum ordering, zero-budget no-progress, and all existing callers/tests. It triggers ADR-0013 impact review of Foundation and of the completed WP-200 child path where the current Planning start step performs uncharged typed work.

### 9. Broad deferred validation/codec authority remains inactive

This prerequisite does not require validator compilation caches, payload-schema validator reuse, codec pipelines, or broad response validation.

The active authority already requires validation as part of admission (`ADMIT-TXN-001`) and already defines TD/resource/work bounds. The hypothesis to review is that a bounded Basic TD validation proof is a refinement necessary to make those active requirements constructible, not activation of `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`.

If independent review concludes that an inactive validation identity actually owns an unavoidable behavior here, this topic must stop and perform the corresponding narrow domain-entry review rather than smuggling it in.

## WP-200 impact

The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` tranche cannot simply be declared unaffected.

At minimum the impact review must examine:

- `PlanBuildInput::new` currently accepts a raw `&Thing`;
- existing tests/fixtures call it with ordinary Thing values they regard as validated;
- completion evidence claims the validated TD can be dropped after build;
- changing `PlanBuildInput` to require a TD validation proof is a public Planning input-contract change even if the exact compiler algorithm and owned output remain unchanged; and
- current child start/step work charging may need reopening if the accepted Foundation class changes how typed Planning work is debited.

WP-200 may be reaffirmed only if an accepted additive adapter preserves its frozen exact-coordinate compiler behavior and evidence meaning. Otherwise the affected tranche/evidence must reopen under ADR-0013 before 0062 relies on it.

## Required evidence before this topic can become DECIDED

An accepted closure should require at least:

- deserialized or manually mutated invalid `Thing` cannot obtain a Planning validation proof;
- `ThingBuilder::build()` returning an ordinary Thing is not itself treated as durable provenance;
- successful Basic validation produces an opaque proof borrowing the exact source Thing;
- safe mutation cannot coexist with a live proof;
- Planning cannot construct `PlanBuildInput` from a bare `&Thing` on the target path;
- zero validation work produces no traversal progress;
- step partitions produce identical validation result and first failure;
- source census and validation respect structural counts, source bytes, temporary bytes, peak-live, and contiguous-allocation bounds;
- oversized source failure and validation failure publish nothing and release private state/accounts idempotently;
- no complete source TD is retained by the first Consumer published plan set;
- Host synchronous driving and application-static manual driving use the same portable validation semantics;
- Consumer-only Servient composition demonstrably retains a resource policy before validation begins;
- no broad deferred validation/codec capability becomes active implicitly;
- Foundation work-class disposition is recorded explicitly; and
- ADR-0013 impact disposition is recorded for WP-200 and any affected Foundation/TD/Servient tranche.

## Relationship to 0062

0062 remains blocked while this topic is `OPEN` or `DISCUSSING`.

A DECIDED/MIGRATED outcome from this topic should give 0062 only these facts:

1. what type/proof Planning receives instead of an assumed validated `&Thing`;
2. how that proof is produced under bounded work/resource rules;
3. how source/input memory overlaps later Planning admission and when it is released;
4. what Foundation work/accounting primitives are authoritative; and
5. the exact WP-200 impact disposition.

0062 must not absorb this topic's TD validator or source-memory design back into its local aggregate closure.

Consumer execution-registration pinning remains a separate later claim.

## Merge condition

This document may merge while `DISCUSSING` only as an investigation record after independent review of the candidate boundary.

It may become `DECIDED` only after a fresh independent review accepts a constructible bounded validation provenance and source-accounting model consistent with active v5.1 authority.

It becomes `MIGRATED` only after the accepted conclusion is projected into the appropriate TD/Foundation/Planning/Servient authority and ADR-0013 impact/admission records. No Rust source implementation is authorized by this workspace topic alone.
