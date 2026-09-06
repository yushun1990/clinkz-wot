# WP-100 Consumer Validated Thing Admission

Status: ADMITTED under ADR-0013 for design revision v5.1 by the independent
acceptance of this exact docs-only revision. This revision registers the
tranche and records its pre-code checks and Producer-gate impact-review
boundary; it changes and authorizes no production source by itself. The first
permitted functional change is a separate implementation revision on this
admission, producing the completion evidence defined below.

This is the Step 1B registration of the migrated `0063` Consumer Plan-Set
Handoff Closure decision. Its technical boundary was projected during the
authority migration into `WP-100-core.md` ("Successor boundary:
`WP-100-CONSUMER-VALIDATED-THING`"), `docs/spec/foundation.md`,
`docs/spec/runtime-safety.md`, `docs/architecture/10-primary-data-flows.md`,
and `docs/architecture/20-module-boundaries.md`; the public API projection is
already recorded in `docs/api-ownership.csv` as absent `add` rows. This
admission freezes none of those contracts differently; it adds the missing
tranche registration and reviewed record required by ADR-0013.

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
sequence (the WP-300 name-free request readmission and its replacement
evidence, PR #67) is complete and disjoint from this tranche's paths and
requirements; neither direction depends on the other.

## Admission preconditions

All ADR-0013 admission conditions were verified at this revision:

1. `WP-000` is `complete` in `docs/work-packages/index.toml`; the package DAG,
   backbone ownership, and dependency direction needed here are stable.
2. The authority migration is merged and `0063` is `MIGRATED`. No
   `OPEN`/`DISCUSSING`/`DECIDED` workspace topic intersects this scope.
3. The requirement and artifact scope is closed: every behavior above is
   owned by the six indexed requirements, the spec/architecture projections
   named in the header, and the existing `docs/api-ownership.csv` rows
   `AdmissionLedger::reclassify_source_to_persistent_document` and
   `ValidatedThing` (both `absent`/`add`/`frozen`). No new API-ownership row
   is needed or added by this admission.
4. Every executable pre-code check passes at the recorded head (below).
5. The Producer Property Read gate WorkClass impact-review boundary is
   complete and recorded below.
6. Completion evidence identity, cells, and acceptance criteria are defined
   below before any implementation begins.

## Owned boundary

The tranche owns exactly three production changes:

1. **Append-only `WorkClass` discriminants.** `WorkClass::DocumentNodes` then
   `WorkClass::PlanningItems` are appended after `WorkClass::HandlerSteps`
   (discriminants `10` and `11`); `WORK_CLASS_COUNT` grows from ten to twelve;
   `WorkClass::ALL` grows to `[Self; 12]`; `WorkBudget`'s private counter
   array grows to `[u64; 12]` with both new counters defaulting to zero. All
   existing discriminants, the first ten `WorkClass::ALL` entries, and every
   existing `WorkBudget` construction/charge semantic are unchanged.
   `DocumentNodes` charges only typed-document validation and census visits
   not already owned by a more specific class (schema-node visits, URI
   bytes, and security branches stay `JsonSchemaNodes`, `UriBytes`, and
   `SecurityBranches`; no unit is relabelled or double charged); its one
   non-resettable admission-lifetime allowance derives from the existing
   `document_validation_work_units_max`. `PlanningItems` is admitted here as
   a discriminant only; no code path outside this tranche may charge it yet.
2. **The narrow ledger account transfer.** One new checked
   `AdmissionLedger` operation,
   `reclassify_source_to_persistent_document`, moves the same already-live
   source bytes into persistent-document accounting: it checks the
   persistent-document destination limit before changing either account,
   changes `live_bytes`, `peak_live_bytes`, and
   `largest_contiguous_allocation` not at all, and on a failed destination
   check leaves the source charge and the owned representation intact for
   ordinary rollback. It is an ownership-preserving reclassification, not a
   second reservation and not a second Thing representation.
3. **The TD-owned move-only validated input/census path.** A new
   `td`-owned `ValidatedThing` whose successful construction proves:
   ownership of the exact input `Thing`; complete `ValidationLevel::Basic`
   validation; checked structural limits for the typed representation; a
   conservative representation-aware retained-source footprint plus the
   counts needed by later Planning preflight; and bounded Host and
   application-static progress with cancellation through the same pure
   cursor, whose `DocumentNodes` lifetime remainder cannot be reset by fresh
   per-step budgets. It exposes no public unchecked constructor and no
   mutable raw-`Thing` projection. A Basic-valid `Thing` without an ID
   remains constructible here; its preflight rejection belongs to the later
   aggregate/runtime tranches, and this tranche neither strengthens Basic
   validation globally nor synthesizes an identity.

## Permitted production paths

Production implementation may change exactly:

- `foundation/src/budget.rs`;
- `foundation/src/resource.rs`;
- `foundation/src/lib.rs`;
- `td/src/validated.rs` (new);
- `td/src/validate.rs`;
- `td/src/thing.rs`; and
- `td/src/lib.rs`.

Tests and the registered completion-evidence file may be added outside those
production paths. A required production change elsewhere stops implementation
and returns the tranche to impact review.

Outside the tranche, and unchanged by it:

- `docs/resource-limits.csv` and all 195 generated resource fields;
- `foundation/build.rs`, generated resource-profile assertions, and
  `tools/check-resource-limits.sh`;
- every existing `WorkClass` discriminant value and the first ten
  `WorkClass::ALL` entries;
- the singular passed Producer `integration_gate_manifest` and
  `docs/artifacts.csv`, `docs/spec/README.md`, `tools/design-check`;
- the active 65-requirement set and design revision; and
- the completed Consumer call-values, exact-coordinate planning, and
  name-free binding tranches and their evidence.

## Explicit exclusions

This tranche does not implement or claim:

- aggregate Planning preflight, materialization, bounds, barrier, lookup, or
  draft (the later `WP-200-CONSUMER-PROPERTY-READ-AGGREGATE` boundary);
- any Servient reservation/publication/execution/lifecycle work (the later
  `WP-400` boundary);
- any `OutboundRequest` or Core binding change (none is needed; the completed
  name-free WP-300 tranche remains current);
- Consumer architecture-gate registration or any second manifest;
- any resource row, named-profile value, generated getter, or resource-schema
  change;
- renumbering, inserting before, or reordering any existing work class;
- global Basic-validation strengthening, ID synthesis, or TD representation
  normalization for accounting;
- any charge of `PlanningItems` outside this tranche's own admitted paths;
- registration of any future tranche in `index.toml`; and
- broad WP-100 completion.

## Producer Property Read gate impact review (WorkClass append)

`docs/work-packages/property-read-architecture-gate.toml` registers the
passed v5.0 Producer `PROPERTY-READ-ARCHITECTURE` gate. The append-only
`WorkClass` change intersects its registered evidence and commands, so an
explicit impact review is mandatory before the WP-100 source may merge. The
pre-code half of that review is recorded here; the exact-head disposition is
mandatory completion evidence.

Structural findings at the pre-code head:

1. The only fixed-width work projection in the registered evidence is
   `CleanupContextEvidence::work: [u64; 10]` in
   `tools/architecture-fixtures/property-read-binding/src/lib.rs`, populated
   by `core::array::from_fn` reading `WorkClass::ALL[index]` for indices
   `0..10`. It therefore snapshots exactly the unchanged first ten entries;
   after the append it still compiles, still reads only indices `0..10`, and
   consumes neither appended class. The fixed width is the intentional
   ten-class cleanup prefix, not an accidental `WorkClass::ALL` width
   dependency.
2. The Producer fixture and runner directly consume only
   `WorkClass::BindingPolls` and `WorkClass::CleanupItems`. No registered
   Producer evidence source or command references `DocumentNodes` or
   `PlanningItems`.
3. `compiler_work_is_portable` in `planning/src/property_read.rs` iterates
   every `WorkClass::ALL` entry. For every budget value constructible from
   the existing public API the appended counters are zero, so its result is
   unchanged; after the append it can only newly reject a budget that
   declares one of the appended classes, which is exactly the conservative
   direction the completed exact-coordinate tranche already requires.
4. The only `WorkClass::ALL` width assertion in production source is
   foundation's own in-file unit test
   `handler_steps_are_appended_and_independently_budgeted`, inside this
   tranche's permitted `foundation/src/budget.rs` path; updating its length
   expectation to twelve with prefix/order assertions is part of the
   admitted source change, not a gate-evidence change.
5. `WorkBudget`'s counter array is private with no external fixed-width
   dependency: no other `[u64; 10]`, `WORK_CLASS_COUNT`, or `WorkClass::ALL`
   indexing exists outside `foundation/src/budget.rs` and the two points
   above.

Baseline rerun: every command registered in the gate manifest passed at
pre-code head `0f9db3ed86f556d581a750b4ed32cc1deb7acae4` (all ten
`test_commands`, including the complete fixture, runner, Servient, Core,
Planning, and real-target Zenoh probe commands).

Disposition: the append-only change cannot invalidate the registered
Producer claim, whose cleanup evidence intentionally pins the unchanged
ten-class prefix; the gate remains `passed` and current at this head. This
pre-code disposition does not pre-judge the implementation head: before the
WP-100 source change may merge, the implementation must rerun every
registered command at the exact implementation head, record that disposition
in the completion evidence, and—if any registered claim is invalidated
there—stop before merge while a separate independent gate-control change
reopens the gate. The implementation author may not change gate status as
part of the repair.

## Pre-implementation checks

The following passed at the recorded pre-code head (same Rust tree as this
docs-only revision):

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
becomes complete, that evidence must record the exact implementation commit
and passing results for:

- stable `WorkClass` discriminant order: the ten existing entries and
  discriminants unchanged, `DocumentNodes` then `PlanningItems` appended;
- both feature profiles driving the same pure validation cursor, including
  cancellation and proof that a fresh per-step budget cannot reset the
  `DocumentNodes` lifetime remainder;
- every Basic-validation and census limit boundary (structural limits,
  retained-source footprint, and work exhaustion);
- exact source-account retention on a failed destination check, and
  unchanged total `live_bytes`/`peak_live_bytes`/
  `largest_contiguous_allocation` on successful reclassification;
- proof that schema, URI, and security validation work remains in its
  existing classes and is not double charged as `DocumentNodes`;
- proof of no unchecked `Thing` entry, no mutable raw-`Thing` projection,
  and no ID-less Basic-valid `Thing` rejection inside this tranche;
- proof that `docs/resource-limits.csv`, the generated projection, and
  `tools/check-resource-limits.sh` are unchanged;
- the exact-head Producer-gate impact disposition rerunning every registered
  command and either reaffirming the passed claim or stopping before merge
  for independent gate reopening; and
- normal mainline CI passing.

The evidence must not claim aggregate Planning, `OutboundRequest`, Servient
runtime, the Consumer architecture gate, any future-tranche registration, or
broad WP-100 completion.
