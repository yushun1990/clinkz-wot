# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible, bounded `Thing -> validated Planning input` boundary for the v5.1 Consumer Property Read path without reactivating broad deferred validation/codec scope

## Scope

Workspace topic 0062 established that the missing Consumer plan-set handoff cannot be closed while Planning accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite needed to make that premise real:

- one immutable borrowed typed source and one validated resource-policy projection;
- one linear admission owner across validation and Planning;
- one stable complete-registration snapshot for build-time Planning;
- one sealed Planning authority derived from the exact selected registration entry and ordinal;
- bounded typed-input census and Basic validation;
- hierarchical local/global admission accounting;
- atomic lifetime-plus-step work charging;
- representation-specific physical accounting for the actual Host/static admission storage; and
- the exact impact on the completed WP-200 Consumer Planning tranche.

This topic does **not** own:

- persistent Consumer execution-registration pinning after publication;
- the final aggregate Planning -> Servient plan-set material from 0062;
- PlanId generation allocation;
- final Host/static cancellation product API beyond the checks required while this admission is live;
- Consumer binding execution;
- WP-400 source implementation;
- broad payload/schema validator compilation or codec reuse;
- production Zenoh evidence; or
- the Consumer architecture-gate completion claim.

## Current repository facts

1. `Thing` is an ordinary cloneable value with public mutable fields. Deserialization returns an ordinary `Thing`; it does not establish durable semantic-validation provenance.
2. `ThingBuilder::build()` performs current Basic validation but still returns the same ordinary `Thing` type.
3. `Thing::validate_with_level(Basic)` performs current Thing/security/schema/affordance/Form/local-reference checks. `ExtensionMap::validate_with_level(...)` is currently a no-op, so Basic semantic validation does not interpret unknown extension JSON values.
4. `PlanBuildInput::new(...)` accepts `&Thing`, an arbitrary registration snapshot reference, and a `PlanSetGeneration`; `PlanBuildInput` is `Clone + Copy`.
5. `PlanCompiler::start(...)` and `PlanCompiler::step(...)` each receive `&self` plus a fresh `&PlanBuildInput`.
6. `PropertyReadPlanCompiler` owns plan id, exact target, binding id/generation/configuration/compatibility, registration ordinal, candidate order, and role. Those build-defining facts are not encoded into `PropertyReadBuildCursor`.
7. `PropertyReadPlanCompiler::registration(...)` selects by `registration_index` and currently validates only artifact compatibility before using that registration. Equal compatibility does not prove equal binding id, binding generation, or configuration digest.
8. `BindingRegistrationIdentity` already names the complete binding id, binding generation, configuration digest, artifact compatibility, and diagnostic ordinal.
9. `PlanCompiler`, `PropertyReadPlanCompiler`, `PropertyReadPlanCompiler::consumer_call`, `PlanBuildInput`, and related Planning types are frozen public API items in `docs/api-ownership.csv`.
10. The accepted WP-200 Consumer tranche explicitly freezes `PropertyReadPlanCompiler::consumer_call(...)` as the public Consumer compiler entry and records the tranche as `complete` / `admitted` / `current` in `docs/work-packages/index.toml`.
11. Current `Servient::consume(td: Thing)` performs neither v5 TD admission validation nor v5 resource accounting before creating legacy consumed state.
12. Raw `ResourceLimits` is not by itself a validated role/profile/cell policy; active Foundation authority requires checked applicability and rejects illegal `None` values.
13. `ADMIT-TXN-001` places validation in the reserve-build-publish transaction and requires bounded cancellation checkpoints.
14. `ADMIT-MEM-001` requires representation-specific physical accounting for source/input, temporary, persistent document, persistent runtime, diagnostics, cleanup, current live bytes, peak simultaneously live bytes, and largest contiguous allocation.
15. `AdmissionLedger` provides six operation-local account states and live/peak observations, but current authority also has parent/global ceilings, peak/contiguous ceilings, and per-admission work ceilings that require a broader composite owner.
16. `WorkBudget::consume(...)` mutates one step-class counter only. It cannot atomically coordinate another lifetime counter by itself.
17. The current resource schema's `document_bytes_max` and `json_*` fields describe raw/document JSON representation. A materialized `Thing` does not retain its original JSON representation.
18. TD depends on Foundation while Core depends on TD. TD can own portable validation mechanics but must not own Core/Servient cancellation or publication state.
19. Core structured errors are fixed-capacity values; admission does not need source-derived unbounded diagnostic strings.
20. ADR-0019 did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`.

## Defect

The current architecture permits several independent authorities to be combined after the fact:

```text
raw Thing
  + public PlanBuildInput
  + independently constructed PropertyReadPlanCompiler
  + independently supplied registration snapshot
  + independently supplied PlanSetGeneration
```

That is not one admission transaction.

Even if validation becomes bounded, the existing public Consumer Planning surface still permits:

```text
validated source A + input A -> start -> Pending
input B                    -> step

compiler A -> start -> Pending
compiler B -> step

compiler identity from registration A
registration slot B with equal compatibility
```

The resource side has the analogous problem if one semantic work unit mutates a lifetime allowance and a current step budget sequentially, or if fixed diagnostic bytes are charged without naming the actual physical allocation that contains them.

The closure therefore needs one **linear Consumer admission transaction** whose Planning authority is derived from one exact captured registration entry rather than assembled from independent public pieces.

## Independent review history

### Review 1 — REQUEST CHANGES

Found source/policy/account replacement across resumable validation, raw `ResourceLimits` authority, diagnostic bounding, typed-input applicability gaps, and corrected the false assumption that Basic validation traverses extension contents.

### Review 2 — REQUEST CHANGES

Found repeatable validation-to-Planning authority, a self-referential owned-source cursor shape, mandatory typed-ingestion resource-schema revision, missing hierarchical/cumulative accounting, an unjustified variable diagnostic row, and non-executable Basic-validator equivalence.

### Review 3 — REQUEST CHANGES

Found non-atomic lifetime/step work charging, compiler substitution across `Pending`, and a contradiction between fixed diagnostic storage and `AdmissionLedger::try_reserve_diagnostic(ResourceKind, ...)`.

### Review 4 — REQUEST CHANGES

Accepted the borrowed-source cursor, shared Basic engine, atomic work direction, and owned compiler direction, but found:

1. the compiler could still be constructed from registration A while the captured snapshot supplied registration B with equal compatibility;
2. transaction-owned registration storage plus a stored `PlanBuildInput` borrowing it would recreate a movable self-reference;
3. the frozen public raw Consumer Planning API remained an admitted bypass with no final disposition;
4. fixed diagnostic accounting did not identify the real Host/static enclosing allocation and could double-count or undercount overlapping enum storage; and
5. a concurrent global source-byte evidence item was inapplicable for borrowed input that intentionally charges zero retained-source bytes.

The topic remains `DISCUSSING`.

## Current candidate boundary

The following is the only current investigation candidate. It is not implementation authority.

### 1. First proof uses caller-owned borrowed `Thing`

The admitted first-proof source is `&'td Thing`.

The transaction does not take ownership of an arbitrary by-value Rust `Thing`.

Consequences:

- the source has a stable caller-owned address/lifetime for the admission;
- ordinary Rust borrowing prevents safe source mutation while the admission is live;
- caller-owned source allocations are not reclassified as engine-owned retained-source bytes;
- all typed structural/work limits are still enforced while traversing the borrowed source;
- every engine-owned cursor/index/accounting/planning allocation remains charged; and
- the borrow ends only after Planning has copied all immutable facts needed beyond admission.

The old by-value `consume(Thing)` facade is not a first-proof architectural constraint. A future convenience adapter requires its own measured/accounted owned-input admission.

### 2. TD owns validation mechanics; Servient owns the composite transaction

TD owns:

- borrowed typed-source census/validation traversal;
- an opaque borrowed cursor;
- fixed-width `ValidationIssue` category/location;
- the shared Basic semantic engine; and
- a non-forgeable validated borrowed view after successful completion.

TD does not own cancellation, registration snapshots, plan-set generation, global resource accounts, Planning publication, or Servient lifecycle.

The Servient composition layer owns the composite linear transaction because it already composes TD, Foundation, Core, Planning, Host, and application-static lifecycles.

### 3. The build-time registration snapshot is externally stable and borrowed

The candidate now makes one constructible ownership choice.

The Consumer admission transaction does **not** own a movable `R` and then store a `PlanBuildInput` borrowing that `R`.

Instead:

- Host Servient composition owns one immutable complete-registration snapshot for its startup composition lifetime;
- application-static composition owns/provides the equivalent stable complete-registration snapshot storage;
- the Consumer admission transaction borrows that snapshot for the whole validation + Planning lifetime; and
- the snapshot cannot be replaced or mutated while the admission borrow is live.

Conceptually:

```rust
ConsumerAdmissionTxn<'td, 'reg, State> {
    source: &'td Thing,
    registrations: &'reg CompleteRegistrationSnapshot,
    policy: ValidatedConsumerPolicy,
    accounting: ConsumerAdmissionAccounting,
    plan_set_generation: PlanSetGeneration,
    cancellation: AdmissionCancellationView,
    state: State,
}
```

The transaction never stores a self-referential `PlanBuildInput` that borrows transaction-owned registration storage.

When the current generic Planning trait is used internally during migration, an **ephemeral** `PlanBuildInput` is reconstructed for each call from the same immutable borrowed source/view, the same borrowed snapshot, and the same frozen generation. It does not survive the call and cannot be supplied by an external caller on the admitted path.

This is build-time ownership only. Persistent execution-owner pinning after publication remains the separate prerequisite recorded by 0062.

### 4. Planning entry derives one sealed build authority from the exact snapshot entry

`enter_planning(self, compiler)` is rejected because compiler construction is an independent substitution boundary.

The admitted Consumer Planning transition instead derives its build authority internally from:

- the already-captured complete-registration snapshot;
- the exact selected registration ordinal;
- that entry's complete `BindingRegistrationIdentity`;
- the exact property/Form coordinate;
- the reserved plan-set generation; and
- the deterministic first-proof candidate order.

Conceptually:

```text
ConsumerAdmissionTxn<Validated>
  -> select exact registration ordinal inside captured snapshot
  -> read complete registration identity from that same entry
  -> derive compiler/build authority from that entry
  -> ConsumerAdmissionTxn<Planning {
         authority,
         cursor,
       }>
```

No public/external compiler object or separate `BindingRegistrationIdentity` argument is accepted by this transition.

The sealed Planning authority owns by move the exact compiler/build object it derived. The registration identity inside that compiler is therefore sourced from the same complete registration entry that supplies compiler execution.

Before compiler `bounds`, `start`, or `step`, the admitted adapter requires full identity agreement with the captured entry: binding id, binding generation, configuration digest, artifact compatibility, and the reviewed ordinal relationship. Compatibility equality alone is insufficient.

A test with registration A and registration B having equal artifact compatibility but different binding id/generation/configuration must prove that an A/B cross-wire is impossible through the sealed constructor or is rejected before any compiler `bounds`/`start` work.

After Planning entry, outer `step` receives only the owned transaction plus bounded driver context. It receives neither a new compiler/build authority nor a new registration snapshot/input.

### 5. WP-200 Consumer public contract must reopen; reaffirm-by-adapter is rejected

The review has established an issue in the completed public Consumer Planning contract itself, not only in a later Servient adapter.

The current WP-200 tranche freezes a public `PropertyReadPlanCompiler::consumer_call(...)` that accepts a separately supplied `BindingRegistrationIdentity`, while the generic public Planning input accepts a raw `&Thing` and independently supplied registration snapshot/generation.

Therefore this topic chooses the following impact disposition:

**`WP-200-CONSUMER-PROPERTY-READ-PLANNING` is affected and must reopen before migration/implementation of this boundary. Reaffirmation solely by adding a Servient wrapper is rejected.**

The migrated Consumer Planning public contract must ensure that no safe public Consumer call sequence can:

- construct admitted Consumer Planning directly from a raw unvalidated `&Thing`;
- pair an independently supplied compiler identity with a different complete registration entry;
- replace registration/generation input after `Pending`; or
- resume a Consumer build with a different build authority.

The exact shared-API migration is a WP-200 reopening decision, but the result cannot retain the existing raw Consumer bypass as another admitted Consumer entry.

Because `PlanCompiler`, `PlanBuildInput`, and `PropertyReadPlanCompiler` are shared Producer/Consumer public items, the WP-200 reopening must perform explicit Producer/shared/transitive impact review. Producer behavior is not presumed unaffected merely because this finding originated in the Consumer tranche.

The actual `index.toml`, WP-200 authority, API-ownership, evidence, and source changes occur only after this topic is accepted and the reopened tranche is independently admitted. This DISCUSSING PR records the required disposition; it does not silently rewrite completed authority.

### 6. Typed validation cursor stays borrowed and non-self-referential

TD validation progress borrows the external source, not engine-owned movable source storage.

One TD step performs only a bounded, pre-charged work unit or declared bounded group. TD does not receive Core cancellation types; the outer transaction checks cancellation before each bounded TD step and at required lifecycle boundaries.

If a safe borrowed cursor cannot cover a nested TD container without hidden rewalk or unbounded work, implementation stops and returns to this topic. Ordinal rewalk is not silently accepted because duplicated traversal would alter cumulative work under partitioning.

### 7. Resource-schema revision is mandatory for `TypedThingBorrowed`

The current raw JSON identities are not reused with typed semantics.

Closure requires a next Foundation resource-schema revision that adds ingestion-representation applicability and records a migration disposition for **every** document/input field.

For `TypedThingBorrowed`:

- `document_bytes_max` and existing `json_*` shape fields are RawJson-only and typed-nonapplicable;
- typed ingestion receives distinct typed depth/map/array/node/string identities;
- nested extension `serde_json::Value` participates in typed resource census without gaining Basic semantic interpretation;
- historical `string_bytes_max` / `extension_bytes_max` receive explicit migration dispositions rather than silent reinterpretation;
- `generated_effective_document_bytes_max` remains derived-representation authority only when that representation is materialized;
- `retained_source_bytes_*` remains engine-retained-source authority, so borrowed first-proof input contributes zero retained-source bytes; and
- temporary/peak/runtime/contiguous and cumulative validation-work rows remain applicable where their semantics already match engine-owned admission work/state.

### 8. A checked Consumer policy is captured before traversal

Raw `ResourceLimits` does not authorize admission.

Composition first creates an immutable checked Consumer policy binding:

- resource-schema revision;
- Consumer one-shot role/domain;
- Host or application-static cell;
- ingestion representation `TypedThingBorrowed`;
- profile/application-defined origin and value digest; and
- all applicable local/global values with illegal `None` rejected.

The transaction captures that policy once and cannot rotate it after progress begins.

### 9. Admission accounting is one hierarchical authority

The transaction captures one move-only composite accounting authority covering applicable:

- operation-local temporary/persistent-runtime/diagnostic/cleanup attribution;
- parent/global temporary/peak/runtime accounts;
- per-Thing and global compiled-runtime capacity when Planning begins;
- engine-live/global peak authority;
- largest contiguous allocation checks; and
- cumulative per-admission validation work.

Borrowed first-proof source contributes **zero engine-owned retained-source bytes** locally and globally.

Hierarchical reservation is rollback-safe:

```text
preflight local + parent/global + peak + contiguous
  -> any failure: no participating scope changes
  -> all success: commit all applicable scopes
```

Neither Host nor static profile has an implicit unbounded global scope.

### 10. Lifetime validation work and current step work are atomically charged

`WorkBudget` remains the caller's current step allowance. The transaction also owns a monotonic lifetime allowance initialized from checked `document_validation_work_units_max`.

Foundation provides one composite operation that:

1. preflights lifetime units without mutation;
2. preflights the requested `WorkClass` in the current step budget without mutation;
3. leaves both bit-for-bit unchanged if either check fails;
4. commits both only after both checks succeed; and
5. starts the bounded work only after joint commit.

It must not emulate atomicity by consuming one public counter and attempting rollback of the other.

Evidence includes zero/wrong-class failure on either side, exact one-unit success, and partition equivalence under replenished step budgets.

The exact typed census/validation `WorkClass` still requires Foundation impact/admission; semantic reuse of `JsonSchemaNodes` is not assumed.

### 11. Fixed failure storage is accounted from the actual enclosing Host/static representation

The prior model of charging `N = size_of::<largest failure carrier>` as an abstract diagnostic reservation is insufficient because it does not prove what physical storage contains those bytes.

The first proof therefore requires an explicit concrete admission-storage representation for each execution profile.

Conceptually the storage has a dedicated, non-overlapping fixed failure slot rather than reusing an overlapping state-enum payload:

```rust
ConsumerAdmissionStorage<...> {
    // source/snapshot are borrows, not owned source allocations
    state: AdmissionStateStorage<...>,
    failure_slot: MaybeUninit<ConsumerAdmissionFailure>,
    // other explicit fixed fields
}
```

Exact Rust layout is not frozen, but these accounting rules are:

- **Host:** the engine identifies the actual allocation/arena slot that physically contains the admission storage and records its concrete `Layout` size/alignment;
- **application-static:** the application provides/exclusively reserves the actual bounded slot/storage that physically contains the admission state, and the same representation-specific layout is recorded;
- the fixed failure field is a real non-overlapping region of that enclosing storage, not an abstract second allocation and not overlapping cursor enum storage;
- account attribution partitions real physical storage regions/fields; one byte range is not charged twice merely because different logical states may use it;
- padding/structural overhead receives one explicit owner in the layout record rather than disappearing or being charged to multiple accounts;
- current live accounting charges the enclosing physical storage exactly once;
- diagnostic attribution names the actual dedicated failure-slot region;
- largest contiguous allocation/exclusive reservation is measured once from the **whole enclosing Host allocation/static slot**, not by adding the diagnostic field as another allocation; and
- if Host and static use different concrete layouts, each profile has its own measured layout evidence rather than assuming equal `size_of`.

Foundation's primitive is therefore a representation-specific **admission layout/base-footprint record**, not `try_reserve_diagnostic(ResourceKind, N)`.

The higher layer supplies measured layout/account-region facts for the concrete storage type it owns; Foundation remains protocol-neutral and validates that account attribution is non-overlapping and consistent with the enclosing total/contiguous layout.

`ConsumerAdmissionFailure` covers the actual largest fixed failure form that may occupy the dedicated slot, including structured Core limit errors and fixed validation issues.

If later work retains variable diagnostic collections or strings, that new variable resource must justify a real schema row/`ResourceKind`.

### 12. Basic validation has one semantic engine

The bounded TD admission validator and synchronous `Thing::validate_with_level(Basic)` must share one TD-owned Basic semantic engine/check graph.

The incremental driver is the canonical traversal. The synchronous API becomes an adapter over the same engine and may expand the fixed issue location into authoring-friendly allocating `ValidateError` only outside the bounded Servient admission carrier.

Migration differential tests prove success/failure and first deterministic issue agreement until the synchronous API delegates to the shared engine structurally.

### 13. Cancellation is owned above TD

The Servient-owned transaction captures the Host/static cancellation source once.

The outer owner checks cancellation before the first TD step, before every bounded TD/Planning step, before admission reservation/reconciliation boundaries, and immediately before publication. TD receives no Core cancellation type.

The final user-facing cancellation request API remains a later lifecycle projection.

### 14. Consumer policy/accounting/snapshot owners survive independently of Producer setup

Host Servient composition owns:

- the validated Consumer policy;
- Host parent/global resource accounts; and
- the immutable complete-registration snapshot whose borrow outlives each Consumer admission.

Application-static composition provides equivalent bounded policy/account/snapshot ownership before admission starts.

None of these owners depend on a Producer Property Read registration merely to exist.

### 15. Broad deferred validation/codec authority remains inactive

This prerequisite does not activate validator compilation caches, payload-schema reuse, codec pipelines, or broad response validation.

If independent review proves an inactive validation identity is unavoidable, this topic stops for narrow domain-entry rather than importing it implicitly.

## Required authority impact

Before implementation admission, accepted migration requires explicit ADR-0013 impact across:

- **Foundation resource schema** — revised ingestion applicability and typed structural identities;
- **Foundation policy projection** — checked Consumer role/profile/cell/ingestion authority;
- **Foundation accounting** — hierarchical rollback-safe reservations, peak/contiguous enforcement, and representation-specific admission layout/base-footprint accounting;
- **Foundation work** — atomic lifetime+step composite charge and accepted typed validation work class;
- **TD validation** — borrowed resumable shared Basic engine and fixed issue location;
- **Planning / WP-200** — reopen the completed Consumer tranche and its frozen public contract; remove the admitted raw Consumer bypass and bind build authority to the exact complete registration entry;
- **shared Producer/Consumer Planning API** — explicit transitive impact because the currently frozen public items are shared;
- **Servient composition** — linear transaction, borrowed snapshot/source lifetime, checked policy, accounting, cancellation, and concrete Host/static admission storage; and
- **Consumer facade** — first proof is borrowed typed input; by-value convenience requires separate measured owned-input admission.

Persistent execution-registration pinning after publication remains outside this claim.

## Required evidence before DECIDED

An accepted closure requires at least:

- invalid deserialized/manually mutated `Thing` cannot enter admitted Consumer Planning;
- first-proof admission borrows caller-owned `Thing` and safe mutation cannot coexist with the live transaction;
- TD cursor is safe without self-reference or hidden uncharged rewalk;
- source, policy, accounting owner, registration snapshot, generation, cancellation source, or Planning authority cannot be substituted after `Pending`;
- the build-time complete-registration snapshot has a stable external Host/static owner and outlives the admission borrow;
- no stored `PlanBuildInput` self-borrows transaction-owned registration storage;
- ephemeral internal Planning inputs always reconstruct from the same source/snapshot/generation;
- sealed Planning authority derives compiler identity and compiler execution from the **same** complete registration entry and ordinal;
- two registrations with equal artifact compatibility but different binding id/generation/configuration cannot be cross-wired, and rejection occurs before compiler `bounds`/`start`;
- compiler A cannot start and compiler B resume one admitted cursor;
- the WP-200 Consumer public contract is formally reopened before implementation migration; reaffirm-by-Servient-wrapper is not used;
- shared Producer/Consumer API and evidence impact is explicitly dispositioned;
- revised schema dispositions every prior document/input row for RawJson, TypedThingBorrowed, derived runtime, or retirement/replacement;
- typed structural identities bound extension JSON depth/map/array/node/string growth without claiming extension semantic validation;
- raw `ResourceLimits` with illegal applicability cannot start traversal;
- borrowed input leaves local/global retained-source byte accounts unchanged at zero;
- concurrent borrowed admissions are tested against applicable **global temporary, peak, engine-live, and runtime** ceilings; a nonzero global source-byte exhaustion test is explicitly not required for this representation;
- local success plus parent/global failure leaves no partial reservation;
- peak and contiguous rejection occurs before physical allocation/reservation mutation;
- lifetime=1 + step=0, lifetime=0 + step=1, and wrong-class failures leave both work counters unchanged;
- successful composite work decrements both counters exactly once;
- replenished step budgets cannot exceed the cumulative validation-work ceiling;
- one large step and equivalent partitions produce the same semantic result and cumulative usage;
- concrete Host admission storage layout is measured from the actual enclosing allocation/arena slot;
- concrete static admission storage layout is measured from the actual exclusively reserved slot;
- the fixed failure carrier occupies a real dedicated non-overlapping field/region;
- account attribution plus padding ownership covers the measured enclosing storage without double counting;
- largest contiguous measurement charges the enclosing allocation/slot once rather than summing field sizes;
- the largest fixed failure slot covers structured Core limit error/context and `ValidationIssue` cases;
- synchronous Basic validation shares the same semantic engine or migration differential evidence proves exact agreement;
- Basic semantic validation does not falsely claim extension semantic traversal;
- structural/work/local/global/peak/contiguous/semantic failures publish nothing and release private state idempotently;
- cancellation across TD and Planning steps reaches unpublished failure/abort handling without replacing transaction ownership;
- no complete source TD is retained by the first published Consumer plan set;
- Host and static profiles prove the same validation/accounting semantics; and
- ADR-0013 impact/admission records exist for every affected Foundation/TD/Planning/Servient tranche before implementation.

## Relationship to 0062

0062 remains blocked while this topic is `OPEN` or `DISCUSSING`.

A DECIDED/MIGRATED outcome gives 0062 only these facts:

1. first-proof Consumer admission uses borrowed typed input and one linear Servient-owned transaction;
2. build-time registration snapshot ownership/lifetime is stable and externally owned by Host/static composition;
3. Consumer Planning authority is derived from one exact complete registration entry and cannot be independently cross-wired;
4. WP-200 Consumer public Planning contract has reopened and migrated away from the admitted raw bypass;
5. TD owns one shared bounded Basic engine while lifecycle/accounting stay above TD;
6. lifetime and step work use one atomic Foundation charge;
7. concrete Host/static admission storage has representation-specific physical accounting; and
8. the exact revised Foundation policy/schema/work/accounting primitives and WP-200 impact disposition are authoritative.

0062 must not absorb this topic's validator, schema migration, Planning public-API reopening, or admission-storage/accounting design back into its local aggregate closure.

Consumer execution-registration pinning after publication remains a separate later claim.

## Merge condition

This document may merge while `DISCUSSING` only as an investigation record after independent review of the current candidate boundary.

It may become `DECIDED` only after a fresh independent review accepts the borrowed-source validation model, stable external registration-snapshot lifetime, sealed same-registration Planning authority, mandatory WP-200 reopening, revised resource schema/policy, hierarchical accounting, atomic work charging, concrete Host/static physical storage accounting, and shared Basic engine.

It becomes `MIGRATED` only after the accepted conclusion is projected into Foundation/TD/Planning/Servient authority and the reopened WP-200 plus all other affected tranches are independently admitted under ADR-0013. No Rust implementation is authorized by this workspace topic alone.
