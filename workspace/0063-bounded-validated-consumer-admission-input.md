# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible, bounded `Thing -> validated Planning input` boundary for the v5.1 Consumer Property Read path without reactivating broad deferred validation/codec scope

## Scope

Workspace topic 0062 established that the missing Consumer plan-set handoff cannot be closed while `PlanBuildInput` accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite needed to make that premise real:

- what proves that one exact `Thing` is valid for Planning;
- how one linear admission owner preserves that exact source and all Planning inputs across resumable validation and Planning;
- how one immutable validated resource-policy projection, local/global accounting authority, and cumulative validation-work allowance remain bound to the whole operation;
- how typed-input resource applicability is versioned and enforced;
- how TD validation progress remains dependency-safe and constructible without self-referential cursors;
- how Host and application-static profiles share the same semantic validation contract; and
- what impact this has on the completed WP-200 Consumer planning tranche and the Foundation/TD/Servient admission substrate.

This topic does **not** own:

- persistent Consumer execution-registration pinning after plan-set publication or execution-owner lifetime;
- the final aggregate Planning -> Servient plan-set material from 0062;
- PlanId generation allocation;
- final static/Host consume-cancellation product API beyond the cancellation checks required while this admission transaction is live;
- Consumer binding execution;
- WP-400 Consumer source implementation;
- broad payload/schema validator compilation or codec reuse;
- production Zenoh evidence; or
- the Consumer architecture-gate completion claim.

## Current repository facts

1. Public `Thing` is an ordinary cloneable value with public mutable fields. A caller can construct or mutate a `Thing` without preserving any prior validation fact.
2. `Deserialize for Thing` checks Serde/typed field shape but returns an ordinary `Thing`; it does not call `Validate`.
3. `ThingBuilder::build()` calls the current default `Validate::validate()` (`ValidationLevel::Basic`) but also returns the same ordinary `Thing` type. The validation result therefore has no durable type-level provenance.
4. `Thing::validate_with_level(Basic)` traverses required Thing/security/schema/affordance/Form/local-reference semantics, but `ExtensionMap::validate_with_level(...)` is currently a no-op. Basic semantic validation therefore does **not** traverse unknown extension JSON values. The current API is synchronous and does not accept `WorkBudget`, resource accounts, or an incremental cursor.
5. `clinkz_wot_planning::PlanBuildInput::new(...)` accepts `&Thing`, an arbitrary registration snapshot reference, and a `PlanSetGeneration`; `PlanBuildInput` is `Clone + Copy`.
6. `PlanCompiler::start(...)` and `PlanCompiler::step(...)` both accept a fresh `&PlanBuildInput`. The current Property Read compiler rereads registration and plan-set-generation data after `Pending`, so a caller can currently substitute compatible build inputs across steps.
7. The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` evidence assumes a validated TD build input and proves only that the build output survives destruction of that input.
8. Current public `Servient::consume(td: Thing)` performs neither TD validation nor v5 admission accounting. It immediately constructs legacy `ConsumedThing` state.
9. `ServientBuilder::resource_limits(...)` currently retains the supplied `ResourceLimits` only when the narrow Producer Property Read registration is also present. A Consumer-only Servient therefore has no durable policy owner today.
10. `ResourceLimits` is only the canonical low-level value snapshot. Active Foundation authority says it becomes a validated configuration authority only after an owning builder binds the executable role/profile/cell applicability set and rejects every illegal `None`.
11. Active `ADMIT-TXN-001` places validation inside the reserve-build-publish admission transaction and requires bounded cancellation checkpoints.
12. Active `ADMIT-MEM-001` requires distinct accounting for source/input bytes, phase-local temporary bytes, persistent document retention, persistent compiled-runtime bytes, diagnostics, cleanup ownership, current live bytes, peak simultaneously live bytes, and largest contiguous allocation.
13. `AdmissionLedger` has six operation-local accounts and observes live/peak/contiguous values, but it does not itself enforce the schema's global ceilings, per-admission validation-work total, peak-live maximum, largest-contiguous maximum, or hierarchical reservations.
14. The resource schema contains both operation-local and global source/temporary/peak/runtime rows plus `largest_contiguous_allocation_bytes_max` and `document_validation_work_units_max`.
15. Current `WorkClass` has ten classes. None is explicitly a typed TD/admission-validation item class. Reusing `JsonSchemaNodes` for every typed TD traversal would be a semantic reinterpretation unless independently accepted.
16. The active schema's `document_bytes_max` and `json_*` identities describe document/JSON shape. A materialized `Thing` does not retain its original JSON syntax/tree, and its serializer may emit a different shape through omitted fields, one-or-many forms, and flattened extensions.
17. TD already depends on Foundation with `default-features = false`; Core depends on TD. TD therefore may own portable validation cursor/proof semantics but must not own a Core/Servient cancellation type.
18. The existing `Core::ErrorContext` demonstrates a fixed-capacity, allocation-free structured diagnostic representation.
19. ADR-0019 deliberately did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`. This topic must not import those deferred domains merely to establish admission provenance for a typed TD.

## Defect

The current architecture contains two adjacent trust gaps:

```text
Thing
  -> PlanBuildInput::new(&thing, ...)
  -> field is called `validated_td`
  -> Planning assumes validation happened somewhere

PlanBuildInput A
  -> PlanCompiler::start(&A)
  -> Pending(cursor)
  -> PlanCompiler::step(&B, cursor, ...)
  -> Planning may observe a different TD / registration snapshot / generation
```

Calling `thing.validate()` immediately before Planning fixes neither gap. A freely borrowable validation view also fixes only the first gap if Planning may later substitute its other build inputs.

The closure therefore needs one **linear Consumer admission transaction** above TD that binds:

- the exact immutable source;
- validation provenance and cursor state;
- one validated resource-policy snapshot;
- one local/global accounting owner set;
- one monotonic admission validation-work allowance;
- the immutable Planning registration snapshot used for this build;
- the reserved plan-set generation used for this build; and
- cancellation observation for the live admission.

That owner must survive every `Pending` boundary until validation and Planning complete or abort.

## Independent review history

### Review 1 — REQUEST CHANGES

The first independent review accepted the trust-gap diagnosis and WP-200 impact direction but identified four blockers:

1. resumable validation accepted a fresh `&Thing` and account/policy view on every step;
2. retaining raw `ResourceLimits` did not establish a validated role/profile/cell policy;
3. bounded diagnostics had neither an authoritative ceiling nor a non-allocating admission error representation; and
4. raw-document resource fields lacked an explicit typed-input applicability disposition.

It also corrected repository fact 4: Basic validation does not traverse extension contents because `ExtensionMap::validate_with_level` is currently a no-op.

### Review 2 — REQUEST CHANGES

The second independent review found that the revised candidate still failed constructibility because:

1. the validation proof became repeatable at the Planning handoff while Planning accepts fresh copyable build inputs on every step;
2. an operation owning both `Thing` and iterators into that `Thing` would recreate a self-referential cursor problem;
3. applying current JSON resource identities to a newly invented typed-tree projection requires a schema revision unconditionally;
4. operation-local `AdmissionLedger` plus caller-replenishable `WorkBudget` does not enforce cumulative work, global/hierarchical memory, peak-live, or contiguous ceilings;
5. the proposed diagnostic row controlled no variable resource when the issue is already fixed-width; and
6. semantic equivalence with the existing Basic validator lacked executable proof.

The topic remains `DISCUSSING`.

## Current candidate boundary

The following is the only current investigation candidate. It is not implementation authority.

### 1. The first proof uses a borrowed external `Thing`

The first-proof admission representation is now deliberately **borrowed external typed input**.

The Consumer admission transaction captures `&'a Thing`; it does not take ownership of an arbitrary by-value Rust `Thing`.

Consequences:

- ordinary Rust borrowing prevents safe mutation for the whole live admission;
- the source object has a stable external address/lifetime, so TD validation cursors may safely borrow it without constructing a self-referential owner;
- caller-owned source allocations are not reclassified as engine-owned heap merely because the engine reads them;
- the engine still enforces every admitted typed structural/work limit while traversing the borrowed source;
- any engine-owned temporary/index/diagnostic/planning state remains fully charged; and
- the borrow ends only after Planning has copied every immutable fact needed beyond admission.

This choice intentionally removes `Servient::consume(td: Thing)` from the first-proof compatibility constraint. A later convenience wrapper may move/own a `Thing` only if a separately admitted measured/accounted input adapter proves its physical footprint without allocator undercounting. Preserving the old by-value facade is not a reason to weaken `ADMIT-MEM-001`.

Application-static/manual progress likewise keeps the caller's `Thing` alive for the lifetime required by the admission transaction.

### 2. TD owns validation mechanics; Servient owns the composite admission transaction

TD owns only protocol-independent typed-TD census/validation semantics:

- borrowed source traversal;
- opaque borrowed validation cursor state;
- fixed-width validation issue identity/location;
- `ValidationLevel::Basic` semantic checks; and
- a non-forgeable validated TD view produced only after successful completion.

TD does **not** own Core/Servient cancellation, Planning registration snapshots, plan-set generation, global runtime accounts, publication, or Servient lifecycle.

The composite first-proof transaction belongs in the Servient composition layer because that crate already composes TD, Foundation, Core, Planning, Host, and application-static Property Read lifecycles without reversing dependencies.

Provisional semantic shape:

```rust
ConsumerAdmissionTxn<'a, R, State> {
    source: &'a Thing,
    policy: ValidatedConsumerPolicy,
    accounting: ConsumerAdmissionAccounting,
    registrations: R,
    plan_set_generation: PlanSetGeneration,
    cancellation: AdmissionCancellationView,
    state: State,
}
```

Exact names/layout are not frozen. The ownership requirement is frozen for this candidate: a `Pending` result returns the same transaction owner; callers cannot replace source, policy, accounting authority, registration snapshot, generation, or cancellation owner between steps.

### 3. TD validation cursor is borrowed and non-self-referential

Because the source is external and immutably borrowed for the whole operation, TD may use a normal borrowed cursor whose iterators/path state borrow the caller-owned `Thing` rather than the transaction itself.

The cursor must not contain references into engine-owned movable source storage.

The portable TD step contract is deliberately cancellation-agnostic:

```rust
TdValidationStep<'a> = Pending(TdValidationCursor<'a>)
                     | Complete(ValidatedThingView<'a>)
                     | Failed(ValidationIssue);
```

The exact representation may use nested borrowed iterators or another safe borrowed cursor. It must satisfy:

- no unsafe self-reference is required by the public design;
- one TD step performs only a bounded, pre-charged unit or declared bounded group of work;
- the outer Servient admission owner checks cancellation before each bounded TD step and at every required lifecycle boundary;
- TD does not depend on Core cancellation types; and
- step partitioning cannot change semantic success/failure or first issue.

If implementation cannot realize a safe borrowed iterator cursor across every nested TD container without hidden rewalk or unbounded work, it must stop and return to this topic; ordinal rewalk is not silently accepted because budget partitioning must not change admission success through duplicated work.

### 4. Validation completion is consumed into the same linear Planning admission owner

A freely repeatable `ValidatedThingView` is not the Planning handoff authority.

Successful validation transitions the existing `ConsumerAdmissionTxn` into a validated typestate. The only target-path entry into Planning **consumes that typestate** together with its already-captured registration snapshot and plan-set generation.

Conceptually:

```text
ConsumerAdmissionTxn<Validating>
    -> ConsumerAdmissionTxn<Validated>
    -> enter_planning(self)
    -> ConsumerAdmissionTxn<Planning { cursor, private_input }>
```

`private_input` captures once:

- the exact validated borrowed TD view/source identity;
- the immutable Planning registration snapshot selected for this build;
- the exact reserved `PlanSetGeneration`; and
- any Planning input facts copied from the validated admission state.

The admitted outer `step` API receives only the owned transaction plus a step `WorkBudget`/driver context. It does not accept a fresh `PlanBuildInput`.

Internally an adapter may call the existing `PlanCompiler` trait repeatedly with the same privately captured `PlanBuildInput`, but callers cannot supply `A` at start and `B` after `Pending`. Alternatively WP-200 may revise the generic compiler contract to capture input once; that broader choice is an impact-review decision, not something implementation may choose silently.

The existing public `PlanBuildInput::new(&Thing, registrations, generation)` cannot remain the admitted Consumer target-path authority unless it is sealed behind this linear wrapper. Bare `&Thing` construction must not bypass validation provenance.

This requirement is about **build-time Planning input identity**. Persistent execution-registration ownership after publication remains the separate execution-pinning prerequisite recorded by 0062.

### 5. Resource schema revision is mandatory for `TypedThing` ingestion

The current raw/document identities are not reused with invented typed semantics.

Closure requires an explicit next Foundation resource-schema revision that adds an ingestion-representation applicability dimension and records a migration disposition for every field whose meaning/applicability changes.

For the first-proof `TypedThingBorrowed` representation, the candidate disposition is:

| Existing field/family | Revised first-proof disposition |
| --- | --- |
| `document_bytes_max` | `RawJson` only; typed non-applicable. No reserialization proxy. |
| `json_nesting_depth_max` | `RawJson` only; typed non-applicable. |
| `json_members_per_object_max` | `RawJson` only; typed non-applicable. |
| `json_array_items_max` | `RawJson` only; typed non-applicable. |
| `json_value_nodes_per_document_max` | `RawJson` only; typed non-applicable. |
| `string_bytes_max` | historical/raw-document identity is not silently reinterpreted for typed ingestion; migration disposition required. |
| `extension_bytes_max` | historical/raw-document identity is not silently reinterpreted for typed ingestion; migration disposition required. |
| `generated_effective_document_bytes_max` | derived-runtime/effective-document bound, not an input-byte proxy; remains applicable only when that derived representation is actually materialized. |
| `retained_source_bytes_*` | remains an engine-retained-source account. Borrowed external first-proof input contributes zero engine-owned retained-source bytes; a later owned adapter must charge it explicitly. |
| `admission_temporary_bytes_*`, peak/global/runtime/contiguous rows | remain applicable to engine-owned admission/runtime state. |
| typed affordance/Form/schema/security count rows | remain applicable where their existing semantic unit already names typed WoT structure; exact migration table must confirm each row. |
| `document_validation_work_units_max` | remains the per-admission cumulative semantic work ceiling, independent of caller step-budget replenishment. |

The revised schema introduces distinct typed-input structure identities rather than reusing raw JSON ones. The exact stable names are to be frozen in migration, but the first proof needs at least equivalents of:

- typed value nesting depth per Thing;
- typed map/object members per container;
- typed array/vector items per container;
- typed value nodes per Thing; and
- typed UTF-8 string bytes per Thing.

Those typed structural counts include nested `serde_json::Value` extension contents for resource purposes. This still does not make Basic semantic validation interpret extension semantics.

A Foundation migration table must cover **every** existing document/input row, not only `document_bytes_max`, and state one of: unchanged semantic applicability, RawJson-only, TypedThingBorrowed-only/new identity, derived-runtime-only, or retired/replaced.

### 6. A validated Consumer policy binds the revised schema projection before traversal

Raw `ResourceLimits` does not authorize admission.

Before any typed source traversal, composition creates a checked immutable Consumer policy handle that binds:

- the exact revised resource-schema identity;
- capability role `consumer`;
- first-proof domain `Consumer Property Read one-shot`;
- Host or application-static execution cell;
- ingestion representation `TypedThingBorrowed`;
- named profile or application-defined origin/value digest; and
- every applicable local/global value with illegal `None` rejected.

The transaction captures this validated policy once. It cannot rotate profile, schema revision, applicability, values, or origin after progress begins.

### 7. Admission accounting is one hierarchical authority, not only `AdmissionLedger`

The transaction captures one move-only `ConsumerAdmissionAccounting` authority (name provisional) that composes all applicable scopes from the validated policy.

It owns or holds reservations/guards for at least:

- operation-local admission source/temporary/persistent-runtime/diagnostic/cleanup accounts;
- Servient/global source/temporary/peak/runtime accounts where applicable;
- per-Thing compiled-runtime capacity when Planning reservation begins;
- global compiled-runtime/engine-live capacity when applicable;
- enforced per-admission peak-live ceiling;
- enforced global admission peak-live ceiling;
- enforced largest-contiguous-allocation ceiling; and
- one monotonic cumulative validation-work allowance initialized from `document_validation_work_units_max`.

`AdmissionLedger` may remain one component, but observing `peak_live_bytes` or `largest_contiguous_allocation` after reservation is not sufficient. A successful new reservation/allocation must be rejected **before** mutation if it would exceed the applicable local peak, global peak, parent/global account, or contiguous ceiling.

Hierarchical acquisition/release must be deterministic and rollback-safe. A child reservation cannot succeed locally and then leave a leaked local mutation if a parent/global reservation fails.

The exact Foundation primitive may be an extended ledger, a composite permit, or checked reservation group. The admitted semantics are:

```text
reserve child/local + parent/global as one rollback-safe operation
  -> success: all applicable scopes own the charge
  -> failure: no scope retains a partial charge
```

Static/constrained composition supplies bounded application-owned parent/global account storage; Host Servient owns its corresponding shared accounts. Neither profile gets an implicit unbounded global scope.

### 8. Caller step budgets cannot reset the per-admission validation-work ceiling

`WorkBudget` remains the per-step linear allowance supplied by the driver, but it is not the lifetime validation-work authority.

The admission transaction also owns a monotonic `validation_work_remaining` (or equivalent) initialized from the checked `document_validation_work_units_max` value.

Before each TD census/validation work unit:

1. the cumulative per-admission allowance is charged;
2. the applicable `WorkClass` in the caller's current `WorkBudget` is charged; and
3. only then does the work begin.

A caller may replenish a later step's `WorkBudget`; it cannot restore the transaction's cumulative allowance.

Evidence must prove that many small replenished step budgets cannot exceed the same lifetime validation-work ceiling that one large step budget would face.

The exact Foundation `WorkClass` for typed census/validation remains subject to impact review. Reusing `JsonSchemaNodes` without explicit authority migration is rejected; an append-only `ValidationItems` or `AdmissionItems` class remains the preferred direction unless an existing class is independently proven exact.

### 9. Diagnostics are one fixed inline issue; no new diagnostic resource row is proposed

The first proof retains at most one deterministic fixed-width `ValidationIssue`:

```rust
ValidationIssue {
    kind: ValidationIssueKind,
    location: ValidationLocation,
}
```

It contains only fixed-width enums/ordinals/field ids and no source-derived `String`, `Vec`, payload, or recursive cause.

Because this is a fixed part of the transaction/failure layout rather than externally variable retained diagnostic storage, the candidate **withdraws** the proposed `admission_diagnostic_bytes_per_operation_max` resource row.

`ADMIT-MEM-001` still requires diagnostic bytes to be separately attributable. The operation-local diagnostic account therefore uses the compile-time measured size/alignment footprint of this one inline issue as its fixed ceiling/charge; it does not obtain a caller-configurable semantic capacity row for a resource that cannot vary.

No diagnostic-exhaustion fallback exists for this inline issue because constructing the sole fixed slot cannot consume additional variable capacity. Resource-limit failures themselves use an already-admitted fixed structured resource error projection that names the resource category/limit/phase; they do not recursively allocate a validation issue.

If later work retains multiple or variable diagnostic records, that is a new externally variable resource and must justify a schema row under Foundation's new-row rule.

### 10. Basic validation has one semantic engine

The bounded TD admission validator and the synchronous authoring `Validate::validate_with_level(Basic)` must not become independent implementations.

The target design has one TD-owned Basic semantic engine/check graph. The incremental driver is the canonical traversal of that engine. The synchronous API becomes an adapter that drives the same engine to completion outside the Servient admission transaction and then expands the fixed `ValidationIssue` location against the original `Thing` into the existing authoring-friendly allocating `ValidateError` representation.

During migration, differential tests are additionally required across all existing TD validation fixtures plus adversarial cases. For every fixture they must prove:

- success/failure agreement;
- first deterministic issue category/location agreement with the legacy Basic outcome after projection; and
- no rule exists only in one path.

Once the synchronous API itself delegates to the shared engine, semantic equivalence is structural rather than merely a test convention.

Builder-side collected construction errors may remain a separate authoring concern; this claim governs semantic Basic validation of the completed `Thing`.

### 11. Cancellation is owned above TD and checked at bounded outer intervals

The Servient-owned admission transaction captures the applicable Host/static cancellation source once.

TD receives no Core cancellation type. Instead, the outer owner checks cancellation:

- before the first census/validation step;
- before every bounded TD step;
- before entering Planning;
- before every bounded Planning step;
- before reservation/reconciliation transitions owned by later 0062 composition; and
- immediately before any publication transition.

Cancellation therefore cannot be swapped across Pending boundaries, and TD's dependency direction remains unchanged.

The final product API that requests cancellation in Host/static profiles remains a later lifecycle projection; this topic freezes only the dependency-safe ownership/checkpoint requirement needed by this admission.

### 12. Distinct account phases remain explicit

The first-proof phase/account model is:

| Phase | Borrowed source | Engine temporary | Persistent document | Persistent runtime | Diagnostic | Cleanup | Hierarchical/peak |
| --- | --- | --- | --- | --- | --- | --- | --- |
| policy validation/composition | not traversed | checked-builder local only | none | durable policy/account owners | fixed structured policy/resource error | none | parent/global accounts established |
| typed census | caller-owned borrowed source; structural limits enforced | TD cursor/scratch | none | none | one inline fixed issue | none | local/global temporary + peak/contiguous enforced |
| Basic validation | same immutable borrow | TD cursor/scratch | none | none | one inline fixed issue | none | cumulative work + local/global peak enforced |
| Planning | borrow retained until copied facts complete | Planning/child cursors | none | reservation acquired before private runtime build | one fixed issue/error context as applicable | unpublished abort only | per-Thing + global runtime + engine live enforced |
| Frozen/Published | borrow ended at earliest safe boundary | validation/planning temporary released | none for first proof | exact plan-set runtime footprint committed | retained only if separately admitted | lifecycle-owned | published/global accounts remain committed |

The first Consumer proof retains no complete TD source document after Planning has copied all admitted immutable facts.

### 13. Consumer policy/accounting owners survive independently of Producer setup

A Consumer admission transaction cannot depend on `HostPropertyReadOwner` merely to retain resource values or accounting state.

Host Servient composition must own the validated Consumer policy and Host parent/global resource accounts independently of Producer Property Read registration.

Application-static composition must likewise provide validated static policy plus bounded parent/global account storage before admission starts.

### 14. Broad deferred validation/codec authority remains inactive

This prerequisite does not require validator compilation caches, payload-schema validator reuse, codec pipelines, or broad response validation.

The active authority already requires validation as part of admission and defines resource/work bounds. The bounded borrowed typed-TD provenance operation is still treated as a constructibility refinement of those active requirements.

If independent review concludes that an inactive validation identity owns unavoidable behavior here, this topic stops for narrow domain-entry review rather than smuggling that domain in.

## Foundation / TD / Planning / Servient impact now known

This topic requires explicit ADR-0013 impact review before implementation.

At minimum:

- **Foundation resource schema** — create a new schema revision with ingestion-representation applicability, migrate every existing document/input field, and add the minimal typed-input structural identities required by `TypedThingBorrowed`;
- **Foundation policy projection** — add/complete a checked Consumer role/profile/cell/ingestion policy handle rather than treating raw `ResourceLimits` as authority;
- **Foundation accounting** — provide rollback-safe hierarchical local/global reservation semantics, enforce peak/contiguous ceilings before mutation, and retain cumulative admission validation work;
- **Foundation work classes** — explicitly admit the typed census/validation work class;
- **TD validation** — provide one borrowed resumable Basic semantic engine with a fixed-width issue projection and make synchronous Basic validation adapt over it;
- **Planning** — prevent the admitted Consumer path from substituting raw TD/registration/generation input after validation or across `Pending`; the linear Servient adapter may preserve the existing generic compiler internally only if it passes the exact same captured input every time;
- **Servient composition** — own the linear admission transaction, validated Consumer policy, Host/static parent/global account authority, and cancellation checks; and
- **Consumer input facade** — first proof uses borrowed `Thing`; any by-value compatibility wrapper requires a separate measured input adapter.

Persistent execution-registration pinning remains outside this claim.

## WP-200 impact

The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` tranche is affected and requires impact review.

At minimum the review must examine:

- `PlanBuildInput` is currently `Clone + Copy` and publicly constructible from raw `&Thing`;
- `PlanCompiler::start/step` accept fresh inputs, and current Property Read code rereads registration/generation after `Pending`;
- existing fixtures call the compiler with ordinary `Thing` values regarded as validated;
- completion evidence claims only that a validated TD can be dropped after build;
- the target Consumer path now requires a linear owner that captures validated source, registration snapshot, and generation once; and
- current Planning typed work charging may change under the accepted Foundation work-class/cumulative-work model.

WP-200 may be reaffirmed only if a reviewed adapter seals the admitted Consumer path while preserving exact-coordinate compiler behavior and evidence meaning. If the generic Planning contract itself must change, the affected tranche/evidence reopens under ADR-0013 before 0062 relies on it.

## Required evidence before this topic can become DECIDED

An accepted closure must require at least:

- a deserialized or manually mutated invalid `Thing` cannot enter target Planning as validated input;
- `ThingBuilder::build()` returning an ordinary `Thing` is not itself durable admission provenance;
- first-proof admission borrows caller-owned `Thing` storage and safe mutation cannot coexist with the live admission;
- the TD cursor is constructible without owning/moving the source it borrows and without hidden uncharged rewalk;
- one started admission cannot substitute a second `Thing`, resource policy, ledger/global account owner, registration snapshot, plan-set generation, or cancellation source after `Pending`;
- validation completion cannot be used as a freely repeatable authority to create multiple target Planning transactions;
- target Planning cannot accept `PlanBuildInput A` at start and `B` after `Pending`;
- a nonzero registration snapshot ordinal and non-initial plan-set generation remain identical across every Planning step;
- raw `ResourceLimits` with illegal `None` cannot start census;
- Consumer-only Host/static composition owns a validated policy and local/global accounting authority before source traversal;
- resource-schema revision explicitly dispositions every prior document/input row for `RawJson`, `TypedThingBorrowed`, derived runtime, or retirement/replacement;
- typed structural identities bound nested extension `serde_json::Value` depth/map/array/node/string growth without claiming semantic extension validation;
- borrowed input contributes no engine-owned retained-source bytes while engine-owned temporary/index/diagnostic state is still charged;
- zero typed-validation step work produces no corresponding traversal progress;
- replenishing `WorkBudget` across many steps cannot exceed `document_validation_work_units_max`;
- one large step and many partitions have the same semantic validation result under the same cumulative admission allowance;
- concurrent admissions cannot exceed global source/temporary/peak/runtime ceilings even when each local ledger would individually fit;
- local success plus parent/global failure leaves no partial reservation;
- peak-live and largest-contiguous failures occur before rejected allocation/reservation mutation;
- the single inline `ValidationIssue` remains fixed-width/non-allocating and is separately attributable to the diagnostic account without a new variable diagnostic row;
- synchronous `Thing::validate_with_level(Basic)` drives the shared semantic engine or, during migration, differential fixtures prove exact Basic agreement;
- Basic semantic validation does not falsely claim extension semantic traversal;
- oversized structural, validation-work, local-memory, global-memory, peak, contiguous, and semantic-validation failures publish nothing and release private engine-owned state/accounts idempotently;
- cancellation before/among TD steps and Planning steps returns the same transaction ownership to unpublished failure/abort handling;
- no complete source TD is retained by the first Consumer published plan set;
- Host and application-static driving use the same TD validation semantics and accounting invariants;
- no broad deferred validation/codec capability becomes active implicitly; and
- ADR-0013 impact disposition is recorded for every affected Foundation/TD/Planning/Servient tranche before implementation admission.

## Relationship to 0062

0062 remains blocked while this topic is `OPEN` or `DISCUSSING`.

A DECIDED/MIGRATED outcome from this topic gives 0062 only these facts:

1. first-proof Consumer admission uses one borrowed immutable typed source and one linear Servient-owned transaction;
2. validation and Planning cannot substitute source, policy/accounting owners, registration snapshot, generation, or cancellation source across `Pending`;
3. TD owns one shared bounded Basic semantic engine and non-forgeable validated view, while cancellation/accounting/lifecycle stay above TD;
4. the exact revised Foundation policy/schema/work/hierarchical-accounting primitives used by typed admission;
5. when the borrowed TD lifetime ends relative to Planning private state; and
6. the exact WP-200 impact disposition.

0062 must not absorb this topic's TD validator, resource-schema migration, policy validation, or hierarchical accounting design back into its local aggregate closure.

Consumer execution-registration pinning after publication remains a separate later claim.

## Merge condition

This document may merge while `DISCUSSING` only as an investigation record after independent review of the current candidate boundary.

It may become `DECIDED` only after a fresh independent review accepts a constructible borrowed-source, linear validation-to-Planning handoff, revised resource-schema/policy, hierarchical-accounting, cumulative-work, and shared Basic-validation model consistent with active v5.1 authority.

It becomes `MIGRATED` only after the accepted conclusion is projected into the appropriate TD/Foundation/Planning/Servient authority and ADR-0013 impact/admission records. No Rust source implementation is authorized by this workspace topic alone.
