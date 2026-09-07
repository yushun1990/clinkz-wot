# WP-100 Consumer Validated Thing Admission

Status: IMPACT REVIEW; ADMISSION WITHDRAWN on 2026-09-07 after independent
review of implementation candidate github-pr:71 at exact head
`14ececaf847e7eb68446813c5469c486d8cfb41f` falsified the retained-source and
bounded-progress completion claims. Under ADR-0013, this affected admitted
tranche returns to `planned` / `candidate`; this record does not authorize
continued production implementation or merge until corrective authority,
checks, and an independent readmission review pass.

The admission below was established by github-pr:68 and amended by
github-pr:70. It remains the historical frozen boundary that the rejected
candidate attempted to implement, not current implementation authority.
Foundation changes already merged by github-pr:69 remain current and are not
reopened by this finding. The github-pr:70 Context seam correction remains the
only previously permitted component-path change, but it is insufficient to
make every reachable `serde_json::Value` retained allocation observable.

## 2026-09-07 impact finding

Ordinary downstream Cargo feature unification can enable the following legal
representations in the same locked `serde_json 1.0.149` package used by TD:

- with `preserve_order`, `serde_json::Map` is backed by `IndexMap`, but exposes
  no capacity inspection. A one-entry `Map::new()` and a one-entry
  `Map::with_capacity(16_384)` therefore receive the same candidate census
  despite materially different retained backing capacity;
- with `arbitrary_precision`, `serde_json::Number` owns a private `String`,
  including caller-controlled content and spare capacity through the public
  representation, but the candidate treats every Number as allocation-free.

A downstream reproducer using both features against the exact reviewed head
reported 9309 bytes for both compact and over-capacity Maps, and 8108 bytes for
both one-digit and 16,384-digit Numbers. The absolute totals depend on the
surrounding Thing; equality within each pair proves that the candidate cannot
observe the retained allocations. Serialized length and `size_of` do not
repair the missing capacity authority. Cloning, normalizing, or shrinking the
input would violate the frozen exact-original-Thing ownership contract.

The same review found two additional charge-before-work violations inside the
otherwise permitted TD path: explicit `form.op` is scanned before its
per-operation `DocumentNodes` charges, and security roots/combo children are
bulk-expanded before their per-branch `SecurityBranches` charges. These local
scheduling defects must be repaired and tested after readmission, but fixing
them cannot resolve the representation-authority blocker.

The open design question and readmission boundary are recorded in
`workspace/0064-serde-json-retained-representation-impact.md`. Impact review
must select and project an enforceable dependency-feature/representation
inspection boundary, or deliberately change the exact-source contract, before
implementation resumes. The rejected `passed` completion evidence is removed;
github-pr:71 remains the impact-review location and non-mergeable reproducer.
The passed Producer Property Read architecture gate remains current because
its exact ten registered commands still pass and its paths use neither new
WorkClass.

ADR-0013 impact correction (accepted at github-pr:70): the in-progress
implementation (github-pr:69) proved that the frozen conservative
representation-aware `retained_source_bytes()` contract is not computable for
`Thing.context` inside the originally permitted production paths.
`Context.entries` is private with no accessor; `ContextBuilder::object`/`pair`
(`td/src/components/context.rs:153,159`) can inject caller-owned
`serde_json::Value` buffers with arbitrary reserved capacity into it, and the
public `ContextBuilder::uri`/`with_1_0_compatibility` path can grow the backing
`Vec` before retaining only the two standard entries without releasing that
capacity. Both forms are reachable into `Thing` through the public
`ThingBuilder::context`
(`td/src/thing.rs:448`); and every measurement available on the permitted
paths (serialized length, serde data-model counts, `size_of::<Thing>()`) is a
function of content, not of either retained capacity, so both are invisible to
all of them. Implementation correctly stopped at the permitted-path boundary
rather than widening paths itself or shipping a non-conservative census. This
correction amends the admission by adding exactly one narrowly scoped
production path, `td/src/components/context.rs`, which may gain only the
single read-only `pub(crate)` Context-entry storage inspection seam defined
under "Permitted production paths". The frozen `ValidatedThingCursor` /
`ValidatedThingStep` / `ValidatedThing` public API, the bounded Basic
bulk-phase strategy and its single `Thing::validate_with_level` authority, the
195-field resource schema, and the Step 2 boundary of the migrated `0063`
sequence are unchanged by this correction.

This is the Step 1B registration of the migrated `0063` Consumer Plan-Set
Handoff Closure decision. Its technical boundary was projected during the
authority migration into `WP-100-core.md`, `docs/spec/foundation.md`,
`docs/spec/runtime-safety.md`, `docs/architecture/10-primary-data-flows.md`,
and `docs/architecture/20-module-boundaries.md`. This admission does not alter
that architecture; it closes the exact public TD/Foundation contract needed so
implementation does not choose a cross-crate API implicitly.

## Tranche

- id: `WP-100-CONSUMER-VALIDATED-THING`
- work package: `WP-100`
- predecessor tranches: none
- package dependency: `WP-000` (complete)
- owner packages: `clinkz-wot-foundation`, `clinkz-wot-td`
- feature cells: `no-default`, `async-no-std`, `std`
- completion evidence key: `consumer-validated-thing-work-classes`

The affected active requirements are exactly:

- `DOC-RUNTIME-001`;
- `ADMIT-MEM-001`;
- `ADMIT-TXN-001`;
- `CONSTRAINED-WORK-001`;
- `CONSTRAINED-PROGRESS-001`; and
- `CONSTRAINED-OWN-001`.

The tranche has no dependency on the completed
`WP-100-CONSUMER-CALL-VALUES-VALIDATOR`,
`WP-200-CONSUMER-PROPERTY-READ-PLANNING`, or
`WP-300-CONSUMER-PROPERTY-READ-BINDING` tranches. Step 1A of the migrated
sequence is complete and disjoint from this tranche's source paths and
requirements; neither direction depends on the other.

## Admission preconditions

All ADR-0013 admission conditions must hold on the accepted revision:

1. `WP-000` is `complete` in `docs/work-packages/index.toml`.
2. The authority migration is merged and `0063` is `MIGRATED`; no
   `OPEN`/`DISCUSSING`/`DECIDED` workspace topic intersects this scope.
3. Requirement, artifact, public API, resource, ownership, and progress scope
   is closed by this record plus its indexed authoritative artifacts.
4. Every executable pre-code check below passes.
5. The Producer Property Read gate WorkClass impact-review boundary is closed.
6. Completion-evidence identity, cells, and acceptance criteria are defined
   before implementation begins.

## Owned boundary

The tranche owns exactly three production changes:

1. **Append-only `WorkClass` discriminants.** `WorkClass::DocumentNodes` then
   `WorkClass::PlanningItems` are appended after `WorkClass::HandlerSteps`
   (discriminants `10` and `11`); `WORK_CLASS_COUNT` grows from ten to twelve;
   `WorkClass::ALL` grows to `[Self; 12]`; `WorkBudget`'s private counter array
   grows to `[u64; 12]` with both new counters defaulting to zero. Every
   existing discriminant and the first ten entries remain unchanged.
   `DocumentNodes` charges only typed-document validation/census work not
   already owned by a more specific class. Schema nodes remain
   `JsonSchemaNodes`, URI-template bytes remain `UriBytes`, and security
   branches remain `SecurityBranches`; work is not relabelled or double
   charged. Its non-resettable admission-lifetime allowance derives from the
   existing `document_validation_work_units_max`. `PlanningItems` is admitted
   here as a discriminant only; no Planning code charges it in this tranche.
2. **The narrow ledger account transfer.** One checked
   `AdmissionLedger::reclassify_source_to_persistent_document` operation moves
   already-live source bytes to persistent-document accounting without a
   second reservation or cloned representation.
3. **The TD-owned move-only validated input/census path.** TD owns one linear
   validation cursor that consumes the exact input `Thing`, performs complete
   Basic validation plus bounded representation-aware census, and can complete
   as one move-only `ValidatedThing`. Host and application-static callers drive
   exactly the same cursor contract. The census's conservative
   `retained_source_bytes()` accounting for `Thing.context` reads the retained
   entry buffers and their backing `Vec` capacity through the single read-only
   `pub(crate)` Context-entry storage inspection seam admitted in
   `td/src/components/context.rs` below; no other component change is owned or
   permitted.

## Frozen Foundation transfer API

The exact additive Foundation method is:

```rust
impl AdmissionLedger {
    pub fn reclassify_source_to_persistent_document(&mut self, bytes: u64) -> bool;
}
```

The method operates only on already committed ledger account charges. It
returns `false` without mutation when the source account contains fewer than
`bytes`, when destination addition overflows, or when the resulting
persistent-document usage would exceed its destination account limit. On
success it subtracts exactly `bytes` from source usage, adds exactly `bytes` to
persistent-document usage, updates only the destination account's own peak as
applicable, and leaves aggregate `live_bytes`, `peak_live_bytes`, and
`largest_contiguous_allocation` unchanged. `bytes == 0` succeeds without
changing any counter. It performs no allocation and creates no reservation or
new ownership token.

## Frozen TD validation/progress API

TD MUST NOT depend on Core progress/status types. The Step 1B public progress
surface is exactly the following protocol-neutral TD/Foundation contract:

```rust
use clinkz_wot_foundation::{ResourceKind, ResourceLimits, WorkBudget};

pub struct ValidatedThingCursor {
    /* private fields */
}

pub enum ValidatedThingStep {
    Pending(ValidatedThingCursor),
    Complete(ValidatedThing),
    Invalid {
        thing: Thing,
        error: ValidateError,
    },
    Limit {
        thing: Thing,
        kind: ResourceKind,
        configured: Option<u64>,
        observed: Option<u64>,
    },
    Cancelled(Thing),
}

impl ValidatedThingCursor {
    pub fn new(thing: Thing, limits: &ResourceLimits) -> Self;

    pub fn step(
        self,
        budget: &mut WorkBudget,
        cancel_requested: bool,
    ) -> ValidatedThingStep;
}

impl ValidatedThing {
    pub fn thing(&self) -> &Thing;
    pub const fn retained_source_bytes(&self) -> u64;
    pub const fn property_count(&self) -> u64;
    pub const fn readable_property_form_count(&self) -> u64;
}
```

`ValidatedThingCursor` and `ValidatedThing` implement neither `Clone` nor
`Copy`. `ValidatedThingStep` is linear because every nonterminal continuation
owns the unique cursor and every terminal variant owns either the resulting
`ValidatedThing` or the exact original `Thing`. No public API exposes a mutable
raw `Thing` while validation is in progress or after success.

`ValidatedThingCursor::new` moves the exact `Thing` and captures only the fixed
set of scalar limits required by this validation/census path. It performs no TD
traversal, Basic validation, variable-size allocation, or other externally
influenced work. Missing required applicable limits are not interpreted as
unbounded; the first step that establishes that condition terminates as
`ValidatedThingStep::Limit` with the corresponding `ResourceKind` and
`configured = None`.

`step` is the only progress operation. It consumes the prior cursor, so a
caller cannot duplicate or resume an older state. The rules are:

- `cancel_requested == true` is observed before new traversal work and returns
  `Cancelled` with the exact owned `Thing`, including at zero work budget;
- otherwise every externally influenced unit is charged before it starts;
- insufficient caller-supplied per-step budget returns `Pending` with the
  unique continuation and does not perform the rejected unit;
- a generic typed-document visit requires both one caller-supplied
  `WorkClass::DocumentNodes` unit and one unit from the cursor-owned lifetime
  remainder captured from `document_validation_work_units_max`; a fresh
  `WorkBudget` never resets that remainder;
- schema, URI, and security work consume their existing classes rather than a
  second `DocumentNodes` charge;
- semantic Basic-validation failure returns `Invalid` with the original Thing
  and existing `ValidateError` taxonomy;
- structural/resource/lifetime-work failure returns `Limit` with the exact
  resource field, configured ceiling when present, and safely known observed
  amount when one exists; and
- only complete Basic validation plus complete census can return `Complete`.

### Single Basic-validation authority and bounded bulk phase

The existing `Validate for Thing` implementation remains the sole semantic
owner of Basic-validation rules. This tranche MUST NOT duplicate the recursive
Basic rules from `components/data_schema.rs`, affordance, Form, or security
modules inside `validated.rs` merely to make them resumable; those component
files remain outside the admitted production paths.

The cursor therefore has an explicit two-part private execution strategy:

1. an incremental census phase walks the typed representation under the
   caller's per-step `WorkBudget`, enforces the applicable structural limits,
   records the retained-source/count census, and computes the complete work
   charge required by the existing Basic semantic pass; then
2. a Basic semantic phase runs only after one `step` can pre-charge that
   complete already-bounded semantic-pass work from the caller budget and the
   cursor-owned lifetime remainder. If the complete charge is unavailable,
   `step` returns `Pending` without entering `Thing::validate_with_level`.
   After the charge succeeds, the cursor calls the existing
   `Thing::validate_with_level(ValidationLevel::Basic)` exactly once and maps
   its result to `Complete` or `Invalid`.

The semantic phase is deliberately a bounded bulk phase, not a hidden
unbounded traversal: the preceding census has already proved all structural
maxima and the complete pass charge, and `document_validation_work_units_max`
remains the non-resettable admission-lifetime ceiling. Cancellation is checked
immediately before this bulk phase. No component validation implementation is
edited, no second semantic validator is introduced, and no existing Basic
rule may be widened or narrowed by the census. If implementation cannot prove
a complete conservative pre-charge for the existing Basic pass without
changing a component validation source, it MUST stop and return this admission
to impact review rather than widening the permitted paths or copying the rules.

Host execution is a convenience policy above this API: it may repeatedly call
the same `step` contract with replenished per-step budgets until terminal.
Application-static execution stores the returned cursor and resumes it later.
TD defines no second host-only validation algorithm, boxed future, executor, or
Core adapter in this tranche.

The successful `ValidatedThing` records private checked structural census state
and exposes only the four accessors above. Their exact meanings are:

- `thing()` borrows the same exact owned typed Thing; there is no mutable
  projection and no second normalized/canonical Thing;
- `retained_source_bytes()` is the conservative representation-aware retained
  footprint of that exact owned Thing. Serialized length alone and
  `size_of::<Thing>()` alone are insufficient proofs;
- `property_count()` is the number of declared Property affordance entries,
  including properties that have no readable Form; and
- `readable_property_form_count()` is the checked number of Property Forms
  whose TD-owned effective operation set includes `ReadProperty`, after TD
  defaulting and without changing original Form order or indices.

Other validation counters needed only to enforce structural limits remain
private. Later Planning may borrow `thing()` plus the two public counts to run
its own deterministic `PlanningItems` preflight; it does not receive a mutable
Thing, an unchecked cursor, a prebuilt plan row, or a TD-owned Planning policy.
The `ValidatedThing` itself is the move-only ownership handoff and may be
retained by Servient after publication; no `into_thing` escape is admitted in
this tranche.

A Basic-valid Thing without an ID can reach `Complete`. ID absence remains a
later Consumer preflight rejection and is neither synthesized nor globally
added to Basic validation here.

## Permitted production paths

Production implementation may change exactly:

- `foundation/src/budget.rs`;
- `foundation/src/resource.rs`;
- `foundation/src/lib.rs`;
- `td/src/validated.rs` (new);
- `td/src/validate.rs`;
- `td/src/thing.rs`;
- `td/src/lib.rs`; and
- `td/src/components/context.rs` (inspection seam only, defined below).

Tests and the registered completion-evidence file may be added outside those
production paths. A required production change elsewhere stops implementation
and returns the tranche to impact review.

`td/src/components/context.rs` is admitted by the impact correction above for
exactly one narrow change: a single read-only `pub(crate)` Context-entry
storage inspection seam on `Context` that exposes both a borrowed per-entry
view and the retained capacity of its backing `Vec` — for example one accessor
returning `(&[ContextEntry], usize)` or an equivalent read-only view. This lets
the census in `td/src/validated.rs` observe the retained entry buffers and
compute the conservative capacity-aware `Thing.context` footprint, including
the entry container itself. The seam adds no public item and changes no
existing field or method behavior, `Serialize`/`Deserialize`/`Default`
behavior, builder behavior, or validation rule of `Context` or
`ContextBuilder`. It is the only permitted change to any
`td/src/components/**` file in this tranche.

Outside the tranche, and unchanged by it:

- `docs/resource-limits.csv` and all 195 generated resource fields;
- `foundation/build.rs`, generated resource-profile assertions, and
  `tools/check-resource-limits.sh`;
- `td/src/components/**` Basic-validation and serialization implementations,
  with `td/src/components/context.rs` changed only by the single admitted
  read-only `pub(crate)` inspection seam;
- every existing `WorkClass` discriminant value and the first ten
  `WorkClass::ALL` entries;
- the singular passed Producer `integration_gate_manifest` and
  `docs/artifacts.csv`, `docs/spec/README.md`, `tools/design-check`;
- the active 65-requirement set and design revision; and
- the completed Consumer call-values, exact-coordinate planning, and
  name-free binding tranches and their evidence.

## Explicit exclusions

This tranche does not implement or claim:

- aggregate Planning preflight/materialization/bounds/barrier/lookup/draft;
- Servient reservation/publication/execution/lifecycle work;
- `OutboundRequest` or Core binding changes;
- Consumer architecture-gate registration or any second manifest;
- any resource row, named-profile value, generated getter, or resource-schema
  change;
- renumbering/inserting/reordering an existing work class;
- global Basic-validation strengthening or ID synthesis;
- TD normalization or clone-for-accounting;
- any charge of `PlanningItems` by Planning/Servient code;
- a second Basic semantic validator or edits to `td/src/components/**` other
  than the single read-only `pub(crate)` Context-entry storage inspection seam
  in `td/src/components/context.rs`;
- a second host-specific validation path, async executor, or Core dependency;
- registration of any future tranche; or
- broad WP-100 completion.

## Producer Property Read gate impact review

`docs/work-packages/property-read-architecture-gate.toml` registers the passed
v5.0 Producer `PROPERTY-READ-ARCHITECTURE` gate. The append-only WorkClass
change intersects registered evidence, so the exact implementation head must
rerun every registered command before source merge.

The pre-code structural findings are:

1. `CleanupContextEvidence::work: [u64; 10]` intentionally snapshots indices
   `0..10` of `WorkClass::ALL`, so appending entries 10 and 11 preserves the
   exact cleanup prefix.
2. Producer fixture/runner work uses only `BindingPolls` and `CleanupItems`.
3. `compiler_work_is_portable` iterates all classes, but every pre-existing
   budget has zero in appended counters; the append can only conservatively
   reject a future budget that declares those classes.
4. The only production width assertion is Foundation's in-file WorkClass test,
   inside the permitted `foundation/src/budget.rs` path.
5. `WorkBudget`'s counter array is private; no other registered source owns a
   ten-wide complete WorkBudget representation.

Every command registered in the Producer gate manifest passed at pre-code head
`0f9db3ed86f556d581a750b4ed32cc1deb7acae4`. The pre-code disposition is
reaffirmed. Before source merge, implementation must rerun the complete gate at
the exact implementation head and record either reaffirmation or stop for an
independent gate-control reopening. The implementation author may not change
gate status as part of source repair.

## Pre-implementation checks

The following passed at the recorded pre-code head:

```text
tools/check-design-artifacts.sh
cargo test --workspace --locked
cargo check --locked -p clinkz-wot-foundation --no-default-features
cargo check --locked -p clinkz-wot-foundation --no-default-features --features async
cargo check --locked -p clinkz-wot-foundation
cargo check --locked -p clinkz-wot-td --no-default-features
cargo check --locked -p clinkz-wot-td
```

## Completion evidence

The evidence key is `consumer-validated-thing-work-classes` at
`docs/evidence/WP-100-consumer-validated-thing.toml`. Before the tranche
becomes complete, evidence must record the exact implementation commit and
passing proof for:

- stable WorkClass prefix/discriminants with `DocumentNodes` then
  `PlanningItems` appended;
- exact compilation of the frozen ledger and TD public signatures above;
- cursor/step linearity, including compile-fail proof that cursor and validated
  output are not Clone/Copy and that no mutable/raw mid-progress Thing
  projection exists;
- cancellation returning the exact Thing before new work, including zero
  budget and immediately before the Basic bulk phase;
- Pending retaining the unique cursor with no hidden progress when the next
  required per-step class or complete Basic-pass pre-charge has insufficient
  budget;
- a fresh per-step budget not resetting the cursor-owned DocumentNodes
  lifetime remainder;
- complete Basic-validation positive/negative parity with the existing
  `Thing::validate_with_level(ValidationLevel::Basic)` path, proving the cursor
  introduces no second semantic validator;
- every structural/resource limit boundary, including missing applicable
  limits and safely known observed values in `ValidatedThingStep::Limit`;
- retained-source footprint/count correctness, including properties with empty
  readable ranges and effective ReadProperty defaulting, and conservatism of
  the `Thing.context` footprint against both caller-over-reserved entry buffers
  and an over-capacity backing `Vec` produced by growing entries before
  `with_1_0_compatibility`; content-based serialized length and a slice-only
  entry view must demonstrably under-report these respective cases;
- schema/URI/security work remaining in existing work classes without double
  charging;
- ID-less Basic-valid Thing completion;
- exact source-account retention on failed reclassification, destination
  account peak behavior, and unchanged aggregate live/peak/contiguous values on
  success;
- unchanged `docs/resource-limits.csv`, generated projection,
  `foundation/build.rs`, `tools/check-resource-limits.sh`, and
  `td/src/components/**` except the single admitted seam;
- the `td/src/components/context.rs` diff being exactly one read-only
  `pub(crate)` Context-entry storage inspection seam exposing the borrowed
  entries and backing `Vec` capacity, adding no public API and changing no
  `Context`/`ContextBuilder` construction, serialization, or Basic-validation
  behavior; and
- exact-head Producer-gate impact disposition rerunning every registered
  command; and
- normal mainline CI.

The evidence must not claim aggregate Planning, `OutboundRequest`, Servient
runtime, Consumer gate registration, any future tranche, or broad WP-100
completion.
