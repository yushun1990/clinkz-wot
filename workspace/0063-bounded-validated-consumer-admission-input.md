# 0063 Bounded Validated Consumer Admission Input

Status: DISCUSSING

Kind: implementation-discovered cross-domain admission prerequisite

Priority: HIGH

Target: establish a constructible bounded `Thing -> validated Consumer Planning` admission boundary for the v5.1 Consumer Property Read path without activating broad deferred validation/codec scope.

## Scope

Workspace topic 0062 established that WP-400 cannot consume the completed WP-200/WP-300 path while Planning still accepts an ordinary `&Thing` under an informal `validated_td` premise.

This topic owns only the prerequisite admission boundary:

- one immutable typed source and one real validation provenance;
- one linear validation-to-Planning owner across every `Pending` boundary;
- checked Consumer resource-policy applicability;
- typed-input resource-schema semantics;
- bounded validation and binding-compiler work;
- hierarchical admission accounting and physical Host/static storage accounting;
- one stable complete-registration snapshot used consistently for compiler derivation;
- the WP-200 public-contract impact required to remove the raw Consumer Planning bypass; and
- the evidence/admission ordering required before implementation.

This topic does **not** own:

- persistent Consumer execution-registration pinning after plan-set publication;
- the final aggregate Planning -> Servient plan-set handoff owned by 0062;
- allocation of `PlanId` / plan-generation authority;
- final Host/static product cancellation API;
- Consumer binding execution;
- WP-400 implementation;
- broad validator compilation/cache/codec reuse; or
- the Consumer architecture-gate completion claim.

## Stable repository facts

1. `Thing` is an ordinary cloneable/mutable public value. Deserialization returns `Thing`; it does not establish durable validation provenance.
2. `ThingBuilder::build()` runs current Basic validation but still returns ordinary `Thing`.
3. `Thing::validate_with_level(Basic)` is synchronous and unmetered. `ExtensionMap::validate_with_level(...)` is currently a no-op; Basic semantic validation does not traverse unknown extension semantics.
4. `PlanBuildInput` is publicly constructible from raw `&Thing`, registration input, and `PlanSetGeneration`, and is `Clone + Copy`.
5. `PlanCompiler::start/step` accept fresh input on each call.
6. `PropertyReadPlanCompiler` stores build-defining plan/target/binding/configuration/compatibility/registration/candidate/role facts in `self`, not in the cursor.
7. `PropertyReadPlanCompiler::consumer_call(...)`, `PlanCompiler`, and `PlanBuildInput` are frozen public Planning API items.
8. Current registration lookup indexes the supplied registration snapshot and validates artifact compatibility, but does not by itself prove the indexed registration has the same binding id/generation/configuration used to construct the compiler.
9. `BindingRegistrationIdentity::diagnostic_ordinal()` and `BindingCandidate::registration_ordinal()` are distinct concepts: the former is a stable diagnostic identity; the latter addresses one entry in the immutable registration snapshot.
10. `BindingCompilerBounds` declares final artifact footprint, cursor bytes, temporary bytes, and a lifetime `WorkBudget`.
11. Current Property Read Planning retains only the artifact bound from `BindingCompilerBounds`; compiler cursor/temp/lifetime-work bounds are not carried as an owned Planning admission authority.
12. `WorkBudget` is a caller/driver step allowance. Its public `consume()` mutates one class only and cannot atomically coordinate another lifetime counter.
13. `PlanId` is a generation-bearing slot identity distinct from `PlanSetGeneration`; neither is a proof of the other.
14. Active Foundation authority requires distinct source/input, temporary, persistent-document, persistent-runtime, diagnostic, and cleanup accounting plus current/peak/contiguous physical accounting.
15. Existing `AdmissionLedger` is not by itself the whole local/global/hierarchical admission authority.
16. Raw `ResourceLimits` is not a validated profile/cell/representation policy merely because it was retained.
17. Current raw document/JSON resource identities cannot silently acquire typed-Rust `Thing` semantics.
18. Core structured errors are fixed-capacity, but physical admission accounting must describe the concrete enclosing Host/static storage rather than an abstract `size_of::<Error>()` charge.
19. ADR-0019 did not activate broad `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, or `API-CODEC-001`.

## Defect

The current path has adjacent substitution and accounting gaps:

```text
ordinary Thing
  -> raw PlanBuildInput
  -> Planning trusts "validated_td"

input A -> start -> Pending -> input B -> step

compiler identity A
  + registration snapshot entry B
  -> compatibility-only acceptance can preserve an A/B mismatch

compiler bounds
  -> cursor/temp/lifetime-work declaration
  -> current Planning retains only artifact bound
```

A Servient wrapper around the existing public Consumer Planning surface cannot close all of these gaps because safe external callers could still enter the frozen raw Consumer Planning API directly.

The accepted boundary must therefore be linear from typed admission through Planning and must reopen the affected WP-200 Consumer public contract.

## Independent review history

### Review 1 — REQUEST CHANGES

Found source/policy/account substitution across validation `Pending`, raw `ResourceLimits` authority, unbounded diagnostic representation, typed-input applicability gaps, and corrected the ExtensionMap Basic-validation fact.

### Review 2 — REQUEST CHANGES

Found repeatable validation proof at Planning handoff, self-referential owned-source cursors, mandatory typed-ingestion schema revision, missing hierarchical/cumulative accounting, an unjustified variable diagnostic row, and non-executable Basic-validator equivalence.

### Review 3 — REQUEST CHANGES

Found non-atomic lifetime/step validation-work charging, compiler substitution across Planning `Pending`, and fixed diagnostics still incorrectly routed through a `ResourceKind`-requiring reservation.

### Review 4 — REQUEST CHANGES

Found compiler identity could be cross-wired with an equal-compatibility registration, registration snapshot/input ownership could self-reference, the frozen public raw Consumer Planning bypass lacked a final disposition, fixed diagnostic accounting lacked a physical enclosing representation, and one borrowed-source global-source test was vacuous.

### Review 5 — REQUEST CHANGES

Found four remaining closure gaps:

1. `BindingCompilerBounds` cursor/temp/lifetime-work declarations were still not owned/enforced by the sealed Planning authority;
2. compiler construction required `PlanId` even though this topic does not own PlanId allocation authority;
3. required evidence mixed pre-decision constructibility evidence with post-implementation runtime evidence, creating a circular governance dependency; and
4. snapshot registration ordinal and diagnostic ordinal remained ambiguous.

The topic remains `DISCUSSING`.

## Current candidate boundary

The following is one candidate architecture for independent acceptance. It is not implementation admission.

### 1. First proof borrows caller-owned typed input

First-proof Consumer admission captures `&'td Thing` rather than taking ownership of an arbitrary Rust `Thing`.

Required consequences:

- safe mutation cannot coexist with the live admission borrow;
- TD cursors may borrow stable external source storage without a movable self-reference;
- caller-owned source allocation is not reclassified as engine-retained source memory;
- typed structural/work limits still apply to traversal of the borrowed source; and
- the borrow ends only after Planning has copied every immutable fact required beyond admission.

The old by-value `Servient::consume(Thing)` shape is not an architectural constraint. A later convenience adapter may own/copy input only after a separately admitted physical-footprint rule exists.

### 2. Host/static composition owns the complete-registration snapshot; admission borrows it

The registration snapshot is **not** stored by value inside a transaction that also stores references into it.

Host Servient or application-static composition owns one stable immutable complete-registration snapshot. A Consumer admission borrows that snapshot for its whole validation + Planning lifetime:

```text
Host/static composition
    owns CompleteRegistrationSnapshot
             |
             +---- immutable borrow ----> ConsumerAdmissionTxn<'reg>
```

Any use of the existing generic Planning API reconstructs an ephemeral `PlanBuildInput` for one call from the same source/view, same borrowed snapshot, and same captured generation/lease facts. No `PlanBuildInput` borrowing transaction-owned registration storage is persisted across calls.

Persistent execution registration ownership after publication remains a separate 0062 prerequisite.

### 3. Snapshot ordinal and diagnostic ordinal are deliberately distinct domains

The mapping is frozen as follows:

- **registration snapshot ordinal**: the index/slot of one exact entry in the captured immutable complete-registration snapshot. It is the value carried by `BindingCandidate::registration_ordinal()` and is used for build-time indexed registration lookup.
- **diagnostic ordinal**: `BindingRegistrationIdentity::diagnostic_ordinal()`. It is a stable diagnostic/reporting identity and is never used as the registration snapshot index merely because both are `u32`.

They are not required to match.

Required fixture: a valid registration at snapshot ordinal `3` with diagnostic ordinal `17` must build through snapshot entry `3`; no path may reinterpret `17` as the registration lookup ordinal.

### 4. Planning receives a non-forgeable plan identity lease; this topic does not allocate PlanId

This topic does not allocate `PlanId` and does not derive one from raw `PlanSetGeneration`.

Before sealed Consumer Planning construction, the higher Servient/plan-set authority owned by the eventual 0062 composition supplies an opaque non-forgeable **plan identity lease** (name provisional). The lease proves ownership of exactly one reserved logical-plan identity for this unpublished plan-set build.

Conceptually:

```text
0062 / Servient plan-set identity authority
       -> reserve unpublished plan identity
       -> PlanIdentityLease

0063 admission
       + PlanIdentityLease
       -> sealed Consumer Planning authority
```

Required properties:

- external callers cannot forge the admitted lease from a raw `PlanId` or `PlanSetGeneration`;
- the lease contains or exclusively authorizes the exact `PlanId` needed to construct the compiler/logical plan;
- the lease is move-only or otherwise single-use for one unpublished plan build;
- abort returns/releases the reservation under the owning plan-set authority;
- successful freeze transfers the identity into the later 0062 plan-set lifecycle; and
- 0063 does not freeze how plan slots/generations themselves are allocated.

If 0062 chooses another equivalent non-forgeable reservation token, 0063 consumes that authority rather than duplicating it.

### 5. Sealed Planning authority is derived atomically from one registration entry + one plan identity lease

The admitted transition accepts neither an external `PropertyReadPlanCompiler` nor an independent `BindingRegistrationIdentity`.

It consumes:

- validated Consumer admission typestate;
- one exact snapshot registration ordinal;
- the registration entry found at that ordinal in the already-captured snapshot;
- that same entry's complete `BindingRegistrationIdentity`;
- the exact property/Form coordinate;
- deterministic first-proof candidate order; and
- the non-forgeable `PlanIdentityLease`.

From those values it constructs one sealed Planning/build authority.

Before any binding-compiler `bounds/start` progress, the sealed constructor verifies that build identity and registration execution come from the same complete registration entry. Full agreement includes binding id, binding generation, configuration digest, and artifact compatibility. Compatibility equality alone is insufficient.

Required negative fixture: two complete registrations with equal artifact compatibility but different binding id/generation/configuration are cross-wired; construction must fail before binding-compiler `bounds()` or `start()`.

### 6. BindingCompilerBounds is captured exactly once and becomes an owned Planning admission authority

For the selected complete registration and exact compiler input, the sealed Planning authority calls binding-compiler `bounds()` exactly once before compiler `start()`.

The returned `BindingCompilerBounds` is not reduced to only `artifact()`.

The sealed Planning state captures the complete declaration:

- artifact retained items/bytes;
- compiler cursor bytes;
- peak compiler temporary bytes; and
- compiler lifetime work allowance by `WorkClass`.

Required ordering:

```text
exact registration + exact compiler input
  -> bounds() once
  -> validate bounds are portable/allowed
  -> reserve admitted cursor/temp/artifact memory against local+global/peak/contiguous authority
  -> capture compiler lifetime work allowance
  -> only then compiler.start()
```

No compiler progress begins if the declared cursor/temp/artifact reservation cannot be admitted.

The declared cursor/temp bounds remain owned until their respective lifetime boundaries and are reconciled/released deterministically on complete/failure/abort.

### 7. Binding-compiler lifetime work and current step work use the same atomic pair semantics as validation

Caller `WorkBudget` remains a per-step driver allowance. `BindingCompilerBounds::work()` supplies the separate lifetime compiler allowance.

Every binding-compiler work debit is one composite operation:

1. preflight the relevant remaining compiler-lifetime `WorkClass` units without mutation;
2. preflight the same `WorkClass` units in the caller's current step budget without mutation;
3. any failure leaves both unchanged;
4. success commits both exactly once; and
5. compiler work starts only after joint commit.

A replenished caller step budget cannot restore or exceed the lifetime allowance declared by `BindingCompilerBounds`.

Evidence must include zero/wrong-class failures on either side, exact success debit, replenished-step exhaustion, and partition equivalence.

The Foundation primitive may be shared with admission validation work, but validation lifetime allowance and binding-compiler lifetime allowance remain distinct owners/ceilings.

### 8. WP-200 Consumer public contract must reopen

`WP-200-CONSUMER-PROPERTY-READ-PLANNING` is affected and must reopen before implementation migration. Reaffirmation solely through a later Servient wrapper is rejected.

The migrated public Consumer Planning contract must make these sequences unavailable to safe external Consumer callers:

- admitted Consumer Planning from raw unvalidated `&Thing`;
- independent compiler identity + different registration entry;
- fresh registration/generation input after `Pending`;
- compiler A start + compiler B resume; and
- raw caller-provided `PlanId`/generation values standing in for the non-forgeable plan identity reservation required by the admitted path.

Because `PlanCompiler`, `PlanBuildInput`, and `PropertyReadPlanCompiler` are shared Producer/Consumer API items, the reopening includes explicit Producer/shared/transitive impact review. The exact shared API migration is decided in the reopened WP-200 tranche; it may preserve a low-level surface only if that surface is explicitly classified as non-admitted/legacy and cannot masquerade as the admitted Consumer contract.

This DISCUSSING topic records the required reopening disposition; it does not itself mutate `index.toml` completion state or frozen API authority.

### 9. TypedThingBorrowed requires a resource-schema revision and checked Consumer policy

The next accepted resource-schema revision must add ingestion-representation applicability and disposition every existing document/input field.

For `TypedThingBorrowed`:

- existing raw `document_bytes_max` / raw `json_*` identities are RawJson-only;
- typed ingestion gets distinct typed depth/map/array/node/string identities;
- nested extension `serde_json::Value` participates in typed resource census without gaining Basic semantic interpretation;
- historical `string_bytes_max` / `extension_bytes_max` receive explicit migration dispositions;
- `generated_effective_document_bytes_max` applies only to an actually materialized derived representation;
- engine-retained-source bytes remain zero for borrowed first-proof input; and
- applicable temporary/peak/runtime/contiguous and work ceilings remain enforced.

Before traversal, composition must produce an immutable checked Consumer policy binding exact schema revision, role/domain, Host/static cell, ingestion representation, profile/origin, and every applicable local/global value with illegal `None` rejected.

### 10. Admission accounting is hierarchical and representation-specific

The linear admission owns/borrows one composite accounting authority covering applicable local and parent/global temporary/runtime/peak/engine-live capacity, per-Thing/global compiled runtime, contiguous-allocation checks, cleanup, and lifetime work authorities.

Hierarchical reservation obeys:

```text
preflight every participating local + parent/global + peak + contiguous ceiling
  -> any failure: no account changes
  -> all succeed: one deterministic commit
```

Borrowed first-proof source contributes zero engine-owned retained-source bytes locally and globally.

### 11. Concrete Host/static admission storage defines physical fixed-footprint accounting

Fixed failures are not charged as an abstract extra allocation.

Each execution profile defines the actual enclosing admission storage it owns/reserves. If a dedicated failure field is used, it must be a real non-overlapping region in that representation.

For each Host/static representation the layout record must identify:

- actual enclosing allocation / arena slot / exclusively reserved static slot;
- concrete size and alignment;
- non-overlapping byte attribution to logical accounts;
- ownership of padding/structural overhead;
- the real region capable of holding the largest fixed admission failure carrier;
- current/peak live contribution; and
- largest contiguous allocation measured once from the whole enclosing storage.

One physical byte range cannot be charged twice simply because multiple logical states can inhabit it. Different Host/static layouts require separate evidence.

### 12. TD owns one shared bounded Basic semantic engine

TD owns the borrowed resumable Basic validation semantics and fixed-width validation issue location. Servient/Core lifecycle/cancellation/accounting stays above TD.

Synchronous `Thing::validate_with_level(Basic)` must converge on the same semantic engine. During migration, differential fixtures prove success/failure and first deterministic issue agreement; once the synchronous API delegates to the shared engine, equivalence is structural.

Extension resource census may traverse nested extension values while Basic semantic validation continues to treat `ExtensionMap::validate_with_level(...)` according to its current semantic contract.

### 13. Cancellation is captured above TD

The linear admission captures its Host/static cancellation source once. Outer admission checks cancellation before first traversal, before each bounded TD/Planning step, before reservation/reconciliation transitions, and immediately before publication-related transfer.

TD receives no Core cancellation type. The user-facing Host/static cancellation request API remains later lifecycle work.

## Evidence and governance ordering

The previous candidate mixed design evidence and runtime completion evidence. This is replaced by three explicit stages.

### Stage A — evidence required before 0063 may become DECIDED

This stage proves architecture **constructibility**, not production completion. It may use type sketches, compile-only fixtures, deterministic model/unit proofs that do not create an admitted production path, existing-source inspection, and bounded layout calculations for proposed concrete storage types.

Required before `DECIDED`:

- demonstrate a safe borrowed TD cursor can represent Basic progress without self-reference or hidden uncharged rewalk;
- demonstrate linear typestate APIs make source/policy/account/snapshot/cancellation/build-authority substitution unavailable across `Pending`;
- demonstrate registration snapshot external ownership and ephemeral `PlanBuildInput` reconstruction are Rust-constructible;
- demonstrate snapshot ordinal and diagnostic ordinal remain distinct, including nonzero unequal example;
- demonstrate sealed registration-derived compiler construction rejects equal-compatibility A/B identity cross-wire before `bounds/start`;
- define the non-forgeable `PlanIdentityLease` interface required from 0062 without choosing 0062's allocator implementation;
- demonstrate complete `BindingCompilerBounds` ownership/reservation/work-lifetime semantics are constructible;
- demonstrate atomic lifetime+step work debit semantics for validation and binding compiler;
- define the resource-schema migration table and checked Consumer policy projection;
- define concrete Host/static admission-storage representations sufficiently to prove non-overlapping physical attribution and contiguous-layout semantics;
- define the WP-200 reopening/public API migration obligations; and
- record ADR-0013 impact scope for Foundation, TD, Planning/WP-200, Servient, and shared Producer/Consumer API.

A bounded prototype is permitted only when explicitly labeled **non-production constructibility evidence** and excluded from admitted source paths/public API. It cannot be used to claim implementation completion.

### Stage B — required before production implementation admission

After 0063 is independently accepted as `DECIDED`, migration/admission must:

- reopen `WP-200-CONSUMER-PROPERTY-READ-PLANNING` formally;
- perform shared Producer/Consumer API and evidence impact review;
- migrate accepted Foundation resource-schema/policy/accounting/work authority;
- migrate accepted TD validation authority;
- migrate Planning public-contract authority;
- establish the Servient admission owner/lifetimes/storage contract;
- establish the exact plan-identity lease contract supplied by the 0062/Servient plan-set authority; and
- record independent ADR-0013 admission for each affected production tranche before source implementation.

### Stage C — post-implementation completion evidence

Only after admitted source implementation exists must completion evidence prove runtime/physical behavior, including:

- invalid typed Thing cannot enter admitted Consumer Planning;
- safe source mutation is prevented for the live borrowed admission;
- source/policy/snapshot/generation/lease/compiler/cancellation substitution cannot occur across `Pending`;
- snapshot ordinal `3` and diagnostic ordinal `17` remain distinct and build uses snapshot entry `3`;
- equal-compatibility A/B registration cross-wire fails before binding-compiler progress;
- `BindingCompilerBounds` cursor/temp/artifact reservations are made before `start`, respected during progress, and released/reconciled on all terminal paths;
- replenished step budgets cannot exceed validation or binding-compiler lifetime work allowances;
- atomic pair-charge failure leaves both counters unchanged and partition equivalence holds;
- raw ResourceLimits with illegal applicability cannot start traversal;
- typed schema structural limits bound nested extension JSON structure without claiming extension semantic validation;
- borrowed input leaves retained-source accounts at zero;
- concurrent borrowed admissions enforce applicable global temporary/peak/engine-live/runtime ceilings (no nonzero source-byte exhaustion test is required for this representation);
- hierarchical failure leaves no partial reservation;
- actual Host and static enclosing storage layouts match their recorded physical attribution and contiguous measurements;
- Basic synchronous/incremental semantic equivalence holds;
- cancellation and every failure path publish nothing and release private ownership idempotently;
- no complete source TD survives into the first published Consumer plan set; and
- all reopened WP-200/shared/transitive evidence is current.

## Relationship to 0062

0062 remains blocked while this topic is `DISCUSSING`.

An accepted/migrated 0063 gives 0062 only these facts:

1. Consumer admission begins from borrowed immutable typed input under a checked policy;
2. validation and Planning form one linear non-substitutable admission chain;
3. Host/static composition owns the stable complete-registration snapshot;
4. snapshot ordinal and diagnostic ordinal are distinct domains;
5. sealed Planning derives binding compiler authority from one exact complete registration entry;
6. complete `BindingCompilerBounds` memory/work declarations are owned and enforced;
7. 0063 consumes, but does not allocate, one non-forgeable plan identity lease supplied by 0062/Servient plan-set authority;
8. WP-200 Consumer public contract must reopen away from the raw admitted bypass;
9. Foundation/TD/Planning/Servient migration obligations are explicit; and
10. runtime completion evidence is performed only after independent implementation admission.

0062 must not absorb 0063's validator, resource-schema migration, WP-200 reopening, or physical admission-storage design back into its local plan-set handoff topic.

Persistent Consumer execution-registration pinning after publication remains the separate second prerequisite from 0062.

## Merge / transition condition

This document may squash-merge while `DISCUSSING` only as a durable investigation record.

It may become `DECIDED` only after a fresh independent review accepts the Stage-A constructibility boundary, including complete binding-compiler bounds ownership, non-forgeable plan-identity lease dependency, explicit ordinal domains, WP-200 reopening disposition, typed policy/schema/accounting model, and concrete Host/static storage-accounting model.

It becomes `MIGRATED` only after the accepted conclusion is projected into the relevant Foundation/TD/Planning/Servient authority and ADR-0013 impact/admission records.

No production Rust implementation is authorized by this workspace topic alone.