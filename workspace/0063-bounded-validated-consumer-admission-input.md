# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible, bounded `Thing -> validated Planning input` boundary for the v5.1 Consumer Property Read path without reactivating broad deferred validation/codec scope

## Scope

Workspace topic 0062 established that the missing Consumer plan-set handoff cannot be closed while `PlanBuildInput` accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite needed to make that premise real:

- what proves that one exact `Thing` is valid for Planning;
- how one linear admission owner preserves that source, policy/accounting state, Planning input, and Planning build authority across every resumable boundary;
- how typed-input resource applicability is versioned and enforced;
- how lifetime validation work and caller step work are charged atomically;
- how fixed admission diagnostics are represented in Foundation accounting without inventing a variable resource row;
- how TD validation remains dependency-safe and constructible without self-referential cursors;
- how Host and application-static profiles share the same semantic validation contract; and
- what impact this has on the completed WP-200 Consumer planning tranche and the Foundation/TD/Planning/Servient substrate.

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
6. `PlanCompiler::start(...)` and `PlanCompiler::step(...)` both receive `&self` and a fresh `&PlanBuildInput`. The current Property Read compiler rereads registration and plan-set-generation data after `Pending`.
7. `PropertyReadPlanCompiler` itself owns plan id, target, binding identity/generation/configuration/compatibility, registration ordinal, candidate order, and role. Those values are not encoded into `PropertyReadBuildCursor`, so using compiler A for `start` and compatible compiler B for `step` is not rejected by cursor ownership alone.
8. The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` evidence assumes a validated TD build input and proves only that the build output survives destruction of that input.
9. Current public `Servient::consume(td: Thing)` performs neither TD validation nor v5 admission accounting. It immediately constructs legacy `ConsumedThing` state.
10. `ServientBuilder::resource_limits(...)` currently retains the supplied `ResourceLimits` only when the narrow Producer Property Read registration is also present. A Consumer-only Servient therefore has no durable policy owner today.
11. `ResourceLimits` is only the canonical low-level value snapshot. Active Foundation authority says it becomes a validated configuration authority only after an owning builder binds the executable role/profile/cell applicability set and rejects every illegal `None`.
12. Active `ADMIT-TXN-001` places validation inside the reserve-build-publish admission transaction and requires bounded cancellation checkpoints.
13. Active `ADMIT-MEM-001` requires distinct accounting for source/input bytes, phase-local temporary bytes, persistent document retention, persistent compiled-runtime bytes, diagnostics, cleanup ownership, current live bytes, peak simultaneously live bytes, and largest contiguous allocation.
14. `AdmissionLedger` has six operation-local accounts and observes live/peak/contiguous values, but it does not itself enforce the schema's global ceilings, per-admission validation-work total, peak-live maximum, largest-contiguous maximum, or hierarchical reservations.
15. `AdmissionLedger::try_reserve_diagnostic(...)` requires a `ResourceKind`; the active schema has no truthful variable diagnostic resource for one fixed inline failure carrier.
16. The resource schema contains both operation-local and global source/temporary/peak/runtime rows plus `largest_contiguous_allocation_bytes_max` and `document_validation_work_units_max`.
17. `WorkBudget::consume(...)` mutates only one step-class counter and cannot roll back a separate lifetime counter. Charging a lifetime allowance and step allowance sequentially therefore cannot provide atomic pair semantics by convention alone.
18. Current `WorkClass` has ten classes. None is explicitly a typed TD/admission-validation item class. Reusing `JsonSchemaNodes` for every typed TD traversal would be a semantic reinterpretation unless independently accepted.
19. The active schema's `document_bytes_max` and `json_*` identities describe document/JSON shape. A materialized `Thing` does not retain its original JSON syntax/tree, and its serializer may emit a different shape through omitted fields, one-or-many forms, and flattened extensions.
20. TD already depends on Foundation with `default-features = false`; Core depends on TD. TD therefore may own portable validation cursor/proof semantics but must not own a Core/Servient cancellation type.
21. Core errors are fixed-capacity structured values. `CoreError::LimitExceeded` carries a fixed `ErrorContext`, resource identity, configured limit, and bounded requested/observed values; no source-derived variable allocation is required by that carrier.
22. ADR-0019 deliberately did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`. This topic must not import those deferred domains merely to establish admission provenance for a typed TD.

## Defect

The current architecture contains three adjacent substitution/accounting gaps:

```text
Thing
  -> PlanBuildInput::new(&thing, ...)
  -> field is called `validated_td`
  -> Planning assumes validation happened somewhere

PlanBuildInput A
  -> compiler.start(&A)
  -> Pending(cursor)
  -> compiler.step(&B, cursor, ...)
  -> Planning may observe a different TD / registration snapshot / generation

compiler A
  -> start(private_input)
  -> Pending(cursor)
compiler B
  -> step(private_input, cursor)
  -> cursor alone does not prove the same build authority resumed it
```

The resource/work model has the analogous problem when one semantic work unit must debit both a transaction-lifetime allowance and the caller's current step budget. Sequential mutation can consume one counter even when the other rejects the same unit.

The closure therefore needs one **linear Consumer admission transaction** above TD that binds:

- the exact immutable borrowed source;
- validation provenance and cursor state;
- one validated resource-policy snapshot;
- one local/global accounting owner set;
- one cumulative validation-work allowance;
- the immutable Planning registration snapshot;
- the reserved plan-set generation;
- the exact Planning compiler/aggregate-build authority; and
- cancellation observation for the live admission.

That owner survives every `Pending` boundary until validation and Planning complete or abort.

## Independent review history

### Review 1 — REQUEST CHANGES

The first independent review accepted the trust-gap diagnosis and WP-200 impact direction but identified four blockers:

1. resumable validation accepted a fresh `&Thing` and account/policy view on every step;
2. retaining raw `ResourceLimits` did not establish a validated role/profile/cell policy;
3. bounded diagnostics had neither an authoritative ceiling nor a non-allocating admission error representation; and
4. raw-document resource fields lacked an explicit typed-input applicability disposition.

It also corrected repository fact 4: Basic validation does not traverse extension contents because `ExtensionMap::validate_with_level` is currently a no-op.

### Review 2 — REQUEST CHANGES

The second independent review found six further constructibility defects:

1. the validation proof became repeatable at the Planning handoff while Planning accepts fresh copyable build inputs on every step;
2. an operation owning both `Thing` and iterators into that `Thing` would recreate a self-referential cursor problem;
3. applying current JSON resource identities to a newly invented typed-tree projection requires a schema revision unconditionally;
4. operation-local `AdmissionLedger` plus caller-replenishable `WorkBudget` does not enforce cumulative work, global/hierarchical memory, peak-live, or contiguous ceilings;
5. the proposed diagnostic row controlled no variable resource when the issue is already fixed-width; and
6. semantic equivalence with the existing Basic validator lacked executable proof.

### Review 3 — REQUEST CHANGES

The third independent review accepted the borrowed-source and schema-revision directions but found three remaining blockers:

1. lifetime validation work and current step-budget work were charged sequentially, so a failed second charge could consume the first and break partition equivalence;
2. the linear Planning owner captured the input but not the exact `PlanCompiler`/aggregate-build authority, allowing compiler substitution across `Pending`; and
3. the fixed diagnostic proposal still tried to use `AdmissionLedger`'s `ResourceKind`-requiring diagnostic reservation despite deliberately having no variable diagnostic resource row.

The topic remains `DISCUSSING`.

## Current candidate boundary

The following is the only current investigation candidate. It is not implementation authority.

### 1. The first proof uses a borrowed external `Thing`

The first-proof admission representation is deliberately **borrowed external typed input**.

The Consumer admission transaction captures `&'a Thing`; it does not take ownership of an arbitrary by-value Rust `Thing`.

Consequences:

- ordinary Rust borrowing prevents safe mutation for the whole live admission;
- the source object has a stable external address/lifetime, so TD validation cursors may safely borrow it without constructing a self-referential owner;
- caller-owned source allocations are not reclassified as engine-owned heap merely because the engine reads them;
- the engine still enforces every admitted typed structural/work limit while traversing the borrowed source;
- any engine-owned temporary/index/diagnostic/planning state remains fully charged; and
- the borrow ends only after Planning has copied every immutable fact needed beyond admission.

This choice intentionally removes `Servient::consume(td: Thing)` from the first-proof compatibility constraint. A later convenience wrapper may move/own a `Thing` only if a separately admitted measured/accounted input adapter proves its physical footprint without allocator undercounting.

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

Conceptually:

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

Exact names/layout are not frozen. A `Pending` result returns the same transaction owner; callers cannot replace source, policy, accounting authority, registration snapshot, generation, cancellation owner, or Planning build authority between steps.

### 3. TD validation cursor is borrowed and non-self-referential

Because the source is external and immutably borrowed for the whole operation, TD may use a normal borrowed cursor whose iterator/path state borrows the caller-owned `Thing` rather than the transaction itself.

The cursor must not contain references into engine-owned movable source storage.

The portable TD step contract is cancellation-agnostic:

```rust
TdValidationStep<'a> = Pending(TdValidationCursor<'a>)
                     | Complete(ValidatedThingView<'a>)
                     | Failed(ValidationIssue);
```

The exact representation may use nested borrowed iterators or another safe borrowed cursor. It must satisfy:

- no unsafe self-reference is required by the public design;
- one TD step performs only a bounded, atomically pre-charged work unit or declared bounded group;
- the outer Servient admission owner checks cancellation before each bounded TD step and at every required lifecycle boundary;
- TD does not depend on Core cancellation types; and
- step partitioning cannot change semantic success/failure or first issue.

If implementation cannot realize a safe borrowed cursor across every nested TD container without hidden rewalk or unbounded work, it must stop and return to this topic. Ordinal rewalk is not silently accepted because duplicated traversal would change cumulative work under partitioning.

### 4. Validation completion is consumed into one Planning owner that also owns the exact build authority

A freely repeatable `ValidatedThingView` is not the Planning handoff authority.

Successful validation transitions the existing `ConsumerAdmissionTxn` into a validated typestate. The only target-path entry into Planning **consumes that typestate** together with its already-captured registration snapshot and plan-set generation, and it also captures the exact Planning compiler/aggregate-build authority by move.

Conceptually:

```text
ConsumerAdmissionTxn<Validating>
    -> ConsumerAdmissionTxn<Validated>
    -> enter_planning(self, compiler)
    -> ConsumerAdmissionTxn<Planning {
           compiler,
           cursor,
           private_input,
       }>
```

`private_input` captures once:

- the exact validated borrowed TD view/source identity;
- the immutable Planning registration snapshot selected for this build;
- the exact reserved `PlanSetGeneration`; and
- any Planning input facts copied from the validated admission state.

The Planning typestate captures once, by ownership, the exact `PlanCompiler` or aggregate-build object whose configuration defines the build. Later outer `step` calls receive only the owned transaction plus driver budget/context; they receive neither a fresh `PlanBuildInput` nor a fresh compiler/build-authority argument.

The first-proof rule therefore chooses **ownership over identity checking**: compiler A starts and resumes its own cursor because compiler A is stored inside the linear Planning state. Compiler B cannot resume A's cursor through the admitted outer API because B cannot be supplied after entry.

An internal adapter may call the existing generic `PlanCompiler::start/step` with `&self.compiler` and the same privately captured `PlanBuildInput` on every step. The existing generic trait may remain internally reusable only behind that sealed linear adapter. If this cannot preserve WP-200 semantics, the generic Planning contract must reopen rather than add a caller-visible substitution path.

The existing public `PlanBuildInput::new(&Thing, registrations, generation)` cannot remain the admitted Consumer target-path authority unless sealed behind this wrapper. Bare `&Thing` construction must not bypass validation provenance.

Persistent execution-registration ownership after publication remains the separate execution-pinning prerequisite recorded by 0062.

### 5. Resource schema revision is mandatory for `TypedThingBorrowed`

The current raw/document identities are not reused with invented typed semantics.

Closure requires an explicit next Foundation resource-schema revision that adds an ingestion-representation applicability dimension and records a migration disposition for every field whose meaning/applicability changes.

For first-proof `TypedThingBorrowed`:

| Existing field/family | Revised first-proof disposition |
| --- | --- |
| `document_bytes_max` | `RawJson` only; typed non-applicable. No reserialization proxy. |
| `json_nesting_depth_max` | `RawJson` only; typed non-applicable. |
| `json_members_per_object_max` | `RawJson` only; typed non-applicable. |
| `json_array_items_max` | `RawJson` only; typed non-applicable. |
| `json_value_nodes_per_document_max` | `RawJson` only; typed non-applicable. |
| `string_bytes_max` | historical/raw-document identity is not silently reinterpreted for typed ingestion; migration disposition required. |
| `extension_bytes_max` | historical/raw-document identity is not silently reinterpreted for typed ingestion; migration disposition required. |
| `generated_effective_document_bytes_max` | derived-runtime/effective-document bound, not an input-byte proxy; applicable only when that derived representation is actually materialized. |
| `retained_source_bytes_*` | engine-retained-source account; borrowed external first-proof input contributes zero engine-owned retained-source bytes. |
| `admission_temporary_bytes_*`, peak/global/runtime/contiguous rows | remain applicable to engine-owned admission/runtime state. |
| typed affordance/Form/schema/security count rows | remain applicable where their existing semantic unit already names typed WoT structure; migration table confirms each row. |
| `document_validation_work_units_max` | per-admission cumulative semantic work ceiling independent of caller step-budget replenishment. |

The revised schema introduces distinct typed-input structure identities rather than reusing raw JSON ones. First proof needs at least equivalents of:

- typed value nesting depth per Thing;
- typed map/object members per container;
- typed array/vector items per container;
- typed value nodes per Thing; and
- typed UTF-8 string bytes per Thing.

Those resource counts include nested `serde_json::Value` extension contents without making Basic semantic validation interpret extension semantics.

A Foundation migration table covers **every** existing document/input row and states one of: unchanged semantic applicability, RawJson-only, TypedThingBorrowed-only/new identity, derived-runtime-only, or retired/replaced.

### 6. A validated Consumer policy binds the revised schema before traversal

Raw `ResourceLimits` does not authorize admission.

Before any typed source traversal, composition creates a checked immutable Consumer policy handle binding:

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

`AdmissionLedger` may remain one component, but observing peak or contiguous values after mutation is insufficient. A new reservation/allocation is rejected **before** mutation if it would exceed local peak, global peak, parent/global account, or contiguous ceiling.

Hierarchical acquisition/release is deterministic and rollback-safe:

```text
reserve child/local + parent/global as one rollback-safe operation
  -> success: all applicable scopes own the charge
  -> failure: no scope retains a partial charge
```

Static/constrained composition supplies bounded application-owned parent/global account storage; Host Servient owns corresponding shared accounts. Neither profile gets an implicit unbounded global scope.

### 8. Lifetime validation work and current step work are one atomic composite charge

`WorkBudget` remains the caller's per-step linear allowance. The transaction also owns a monotonic lifetime allowance initialized from checked `document_validation_work_units_max`.

They are **not** charged sequentially.

Foundation must provide one exact composite operation, name provisional, equivalent to:

```rust
AdmissionWorkAllowance::try_consume_with_step(
    &mut self,
    step: &mut WorkBudget,
    class: WorkClass,
    units: u64,
) -> Result<(), CompositeBudgetExceeded>
```

Required semantics:

1. preflight the lifetime allowance for `units` without mutation;
2. preflight the requested `WorkClass` in the current `WorkBudget` without mutation;
3. if either preflight fails, **both counters remain bit-for-bit unchanged**;
4. only when both checks succeed are both counters committed; and
5. the bounded TD/Planning work begins only after that joint commit.

Because both counters are held under unique mutable access, Foundation may implement this by checking both remaining values and then performing a non-failing internal pair commit. It must not emulate atomicity by consuming one public counter and attempting to roll it back if the other fails.

A wrong/exhausted step `WorkClass`, zero current step allowance, or exhausted lifetime allowance therefore consumes neither side. Replenishing a later `WorkBudget` cannot restore lifetime work, and a failed partition cannot poison the lifetime allowance.

Evidence must prove:

- lifetime=1 + step=0 leaves lifetime=1 and step=0;
- lifetime=0 + step=1 leaves both unchanged;
- wrong/exhausted class leaves both unchanged;
- one successful unit decrements both exactly once; and
- one large step and any partitioning with the same admitted work produce the same semantic outcome and final lifetime usage.

The exact Foundation `WorkClass` for typed census/validation still requires impact review. Reusing `JsonSchemaNodes` without explicit authority migration is rejected; an append-only `ValidationItems` or `AdmissionItems` class remains the preferred direction unless an existing class is independently proven exact.

### 9. Fixed diagnostics use a Foundation base-footprint primitive, not a `ResourceKind` reservation

The first proof retains at most one fixed-width admission failure carrier and no variable source-derived diagnostic collection.

The Servient layer defines one fixed carrier covering every failure form that may coexist with the transaction/result, conceptually:

```rust
ConsumerAdmissionFailure {
    Validation(ValidationIssue),
    Core(CoreError),
}
```

The exact enum/layout may differ, but the accounted diagnostic footprint is the compile-time measured size/alignment of the **largest actual retained failure carrier**, not merely `ValidationIssue`. This necessarily covers structured limit errors and their fixed `ErrorContext` when they are the larger variant.

Because this footprint is mandatory structural storage rather than a caller-variable semantic resource, it does not use `AdmissionLedger::try_reserve_diagnostic(ResourceKind, ...)` and does not justify a new configurable diagnostic row.

Foundation instead needs an explicit fixed/base-footprint accounting primitive, name provisional, with semantics equivalent to:

```text
AdmissionFixedFootprint {
    diagnostic_bytes: N,
    // other fixed account bytes may be added only with explicit owners
}

create admission accounting
  -> seed the DIAGNOSTIC account with N fixed bytes
  -> include N in current/peak live accounting
  -> include the containing allocation in contiguous-allocation checks
  -> do not require a ResourceKind for this non-configurable structural charge
```

The higher layer supplies `N` from the actual concrete admission failure carrier it owns; Foundation remains protocol-neutral and does not depend on Core or TD error types.

This fixed charge participates in all applicable aggregate per-admission/global peak and engine-live checks. Its local diagnostic-account ceiling is exactly the admitted fixed base footprint; there is no separate diagnostic exhaustion path because the transaction cannot exist without this storage.

If future work retains multiple or variable diagnostic records, that becomes an externally variable resource and must justify a real schema row and `ResourceKind`.

### 10. Basic validation has one semantic engine

The bounded TD admission validator and synchronous `Validate::validate_with_level(Basic)` must not become independent implementations.

The target design has one TD-owned Basic semantic engine/check graph. The incremental driver is the canonical traversal. The synchronous API becomes an adapter that drives the same engine outside the Servient admission transaction and expands the fixed issue location against the original `Thing` into the existing authoring-friendly `ValidateError` representation.

During migration, differential tests additionally cover existing validation fixtures and adversarial cases, proving:

- success/failure agreement;
- first deterministic issue category/location agreement with legacy Basic after projection; and
- no rule exists only in one path.

Once synchronous Basic delegates to the shared engine, equivalence is structural rather than merely a test convention.

Builder-side collected construction errors may remain a separate authoring concern; this claim governs semantic Basic validation of completed `Thing` values.

### 11. Cancellation is owned above TD and checked at bounded outer intervals

The Servient-owned admission transaction captures the applicable Host/static cancellation source once.

TD receives no Core cancellation type. The outer owner checks cancellation:

- before the first census/validation step;
- before every bounded TD step;
- before entering Planning;
- before every bounded Planning step;
- before reservation/reconciliation transitions owned by later 0062 composition; and
- immediately before any publication transition.

Cancellation cannot be swapped across Pending boundaries, and TD's dependency direction remains unchanged.

The final product API that requests cancellation in Host/static profiles remains a later lifecycle projection; this topic freezes only the dependency-safe ownership/checkpoint requirement needed by this admission.

### 12. Distinct account phases remain explicit

| Phase | Borrowed source | Engine temporary | Persistent document | Persistent runtime | Diagnostic | Cleanup | Hierarchical/peak |
| --- | --- | --- | --- | --- | --- | --- | --- |
| policy validation/composition | not traversed | checked-builder local only | none | durable policy/account owners | fixed structured policy/resource error | none | parent/global accounts established |
| typed census | caller-owned borrowed source; structural limits enforced | TD cursor/scratch | none | none | fixed base failure footprint | none | local/global temporary + peak/contiguous enforced |
| Basic validation | same immutable borrow | TD cursor/scratch | none | none | same fixed base footprint | none | atomic lifetime+step work + local/global peak enforced |
| Planning | borrow retained until copied facts complete | Planning/child cursors | none | reservation acquired before private runtime build | same fixed base footprint | unpublished abort only | per-Thing + global runtime + engine live enforced |
| Frozen/Published | borrow ended at earliest safe boundary | validation/planning temporary released | none for first proof | exact plan-set runtime footprint committed | admission failure base released with transaction | lifecycle-owned | published/global accounts remain committed |

The first Consumer proof retains no complete TD source document after Planning has copied all admitted immutable facts.

### 13. Consumer policy/accounting owners survive independently of Producer setup

Host Servient composition owns the validated Consumer policy and Host parent/global resource accounts independently of Producer Property Read registration.

Application-static composition likewise provides validated static policy plus bounded parent/global account storage before admission starts.

### 14. Broad deferred validation/codec authority remains inactive

This prerequisite does not require validator compilation caches, payload-schema validator reuse, codec pipelines, or broad response validation.

The active authority already requires validation as part of admission and defines resource/work bounds. The bounded borrowed typed-TD provenance operation remains a constructibility refinement of those active requirements.

If independent review concludes that an inactive validation identity owns unavoidable behavior here, this topic stops for narrow domain-entry review rather than smuggling that domain in.

## Foundation / TD / Planning / Servient impact now known

This topic requires explicit ADR-0013 impact review before implementation.

At minimum:

- **Foundation resource schema** — create a new schema revision with ingestion-representation applicability, migrate every existing document/input field, and add the minimal typed-input structural identities required by `TypedThingBorrowed`;
- **Foundation policy projection** — add/complete a checked Consumer role/profile/cell/ingestion policy handle rather than treating raw `ResourceLimits` as authority;
- **Foundation accounting** — provide rollback-safe hierarchical local/global reservation semantics, enforce peak/contiguous ceilings before mutation, and add a fixed admission base-footprint path that does not require a semantic `ResourceKind`;
- **Foundation work** — provide an atomic lifetime+step composite work charge and explicitly admit the typed census/validation `WorkClass`;
- **TD validation** — provide one borrowed resumable Basic semantic engine with a fixed-width issue projection and make synchronous Basic validation adapt over it;
- **Planning** — prevent the admitted Consumer path from substituting raw TD/registration/generation input or compiler/build authority across `Pending`; the linear Servient adapter may preserve the existing generic compiler internally only when it owns and reuses the exact same compiler plus private input;
- **Servient composition** — own the linear admission transaction, validated Consumer policy, Host/static parent/global account authority, fixed failure carrier, and cancellation checks; and
- **Consumer input facade** — first proof uses borrowed `Thing`; any by-value compatibility wrapper requires a separate measured input adapter.

Persistent execution-registration pinning remains outside this claim.

## WP-200 impact

The completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` tranche is affected and requires impact review.

At minimum the review examines:

- `PlanBuildInput` is currently `Clone + Copy` and publicly constructible from raw `&Thing`;
- `PlanCompiler::start/step` accept fresh inputs;
- current Property Read code rereads registration/generation after `Pending`;
- `PropertyReadPlanCompiler` build-defining fields are held in `self`, not bound into its cursor;
- existing fixtures call the compiler with ordinary Thing values regarded as validated;
- the target Consumer path now requires a linear owner that captures validated source, registration snapshot, generation, and exact compiler/build authority once; and
- current Planning typed work charging may change under the accepted Foundation work-class/composite-work model.

WP-200 may be reaffirmed only if a reviewed adapter seals the admitted Consumer path while preserving exact-coordinate compiler behavior and evidence meaning. If the generic Planning contract itself must change, the affected tranche/evidence reopens under ADR-0013 before 0062 relies on it.

## Required evidence before this topic can become DECIDED

An accepted closure must require at least:

- a deserialized or manually mutated invalid `Thing` cannot enter target Planning as validated input;
- `ThingBuilder::build()` returning an ordinary `Thing` is not itself durable admission provenance;
- first-proof admission borrows caller-owned `Thing` storage and safe mutation cannot coexist with the live admission;
- the TD cursor is constructible without owning/moving the source it borrows and without hidden uncharged rewalk;
- one started admission cannot substitute a second `Thing`, resource policy, ledger/global account owner, registration snapshot, plan-set generation, cancellation source, or Planning build authority after `Pending`;
- validation completion cannot be used as a freely repeatable authority to create multiple target Planning transactions;
- target Planning cannot accept `PlanBuildInput A` at start and `B` after `Pending`;
- compiler A can start/resume only its own admitted Planning state; an A-start/B-step attempt is unrepresentable or rejected before compiler progress;
- a nonzero registration snapshot ordinal and non-initial plan-set generation remain identical across every Planning step;
- raw `ResourceLimits` with illegal `None` cannot start census;
- Consumer-only Host/static composition owns a validated policy and local/global accounting authority before source traversal;
- resource-schema revision explicitly dispositions every prior document/input row for `RawJson`, `TypedThingBorrowed`, derived runtime, or retirement/replacement;
- typed structural identities bound nested extension `serde_json::Value` depth/map/array/node/string growth without claiming semantic extension validation;
- borrowed input contributes no engine-owned retained-source bytes while engine-owned temporary/index/diagnostic state is still charged;
- zero typed-validation step work produces no corresponding traversal progress;
- lifetime=1 with step=0 leaves both work counters unchanged; lifetime=0 with step=1 leaves both unchanged; wrong-class failure leaves both unchanged;
- successful composite work decrements lifetime and the requested step class exactly once;
- replenishing `WorkBudget` across many steps cannot exceed `document_validation_work_units_max`;
- one large step and many partitions have the same semantic validation result and cumulative work usage;
- concurrent admissions cannot exceed global source/temporary/peak/runtime ceilings even when each local ledger individually fits;
- local success plus parent/global failure leaves no partial reservation;
- peak-live and largest-contiguous failures occur before rejected allocation/reservation mutation;
- Foundation fixed diagnostic/base-footprint accounting requires no `ResourceKind` and includes the actual largest `ConsumerAdmissionFailure` carrier, including structured limit error/context size;
- the fixed failure footprint is reflected in diagnostic attribution, current/peak live totals, global/engine-live accounting, and contiguous-allocation checks from admission creation until release;
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
2. validation and Planning cannot substitute source, policy/accounting owners, registration snapshot, generation, cancellation source, or Planning build authority across `Pending`;
3. TD owns one shared bounded Basic semantic engine and non-forgeable validated view, while cancellation/accounting/lifecycle stay above TD;
4. lifetime and step work are atomically charged through one Foundation primitive;
5. fixed admission failure storage is accounted as Foundation base footprint without a fabricated variable resource identity;
6. the exact revised Foundation policy/schema/work/hierarchical-accounting primitives used by typed admission;
7. when the borrowed TD lifetime ends relative to Planning private state; and
8. the exact WP-200 impact disposition.

0062 must not absorb this topic's TD validator, resource-schema migration, policy validation, composite-work, or hierarchical-accounting design back into its local aggregate closure.

Consumer execution-registration pinning after publication remains a separate later claim.

## Merge condition

This document may merge while `DISCUSSING` only as an investigation record after independent review of the current candidate boundary.

It may become `DECIDED` only after a fresh independent review accepts a constructible borrowed-source, linear validation-to-Planning handoff, captured Planning build authority, revised resource-schema/policy, hierarchical-accounting, atomic composite-work, fixed diagnostic base-footprint, and shared Basic-validation model consistent with active v5.1 authority.

It becomes `MIGRATED` only after the accepted conclusion is projected into the appropriate TD/Foundation/Planning/Servient authority and ADR-0013 impact/admission records. No Rust source implementation is authorized by this workspace topic alone.
