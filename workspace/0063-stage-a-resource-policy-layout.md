# 0063 Stage-A Resource, Work, Layout, and Impact Definitions

Status: NON-PRODUCTION CONSTRUCTIBILITY EVIDENCE

Owner topic: `workspace/0063-bounded-validated-consumer-admission-input.md`

This artifact makes the Stage-A candidate falsifiable before 0063 may become `DECIDED`. It does not change the active resource schema, public API, work-package status, or implementation admission by itself.

Executable companions:

- `planning/tests/consumer_admission_stage_a.rs`
- `planning/tests/consumer_admission_stage_a_pending.rs`

Both fixtures are compiled by the ordinary locked workspace test path. They model ownership, resource applicability, the current binding-compiler SPI, release/rejection paths, and concrete storage topology without creating the future production Consumer admission API.

## 1. Review-7 closure targets

The current Stage-A candidate explicitly closes four constructibility defects found after Review 6:

1. storage must physically fit the modeled validation/Planning states rather than partition placeholder arrays;
2. the checked policy must carry every active first-proof planning/artifact/cursor/per-step-work control in addition to typed-TD and admission-memory controls;
3. compiler lifetime + caller-step work admission must wrap the real `BindingCompilerExtension::step(..., &mut WorkBudget)` contract rather than bypass it with a helper; and
4. the unpublished plan-build reservation must be returned or released on every rejection/abort path.

These remain Stage-A design proofs. Production TD traversal, actual Servient storage, concurrent global ledgers, cancellation runtime, publication, and target-specific measurements remain Stage C.

## 2. One unpublished build lease binds both plan identities and one reservation lifetime

The upstream 0062 / Servient plan-set identity authority issues one opaque move-only lease:

```text
UnpublishedPlanBuildLease
  = exact PlanId
  + exact PlanSetGeneration
  + one unpublished-reservation ownership token
```

Rules:

- no admitted public constructor from raw `PlanId` or raw `PlanSetGeneration`;
- one issuer reserves both values for the same unpublished plan-set build;
- Consumer Planning accepts the lease as one value and has no independent generation parameter;
- ephemeral compiler/Planning input reconstruction reads both identities from that same lease;
- pre-Planning rejection returns the exact lease with the rejected validated transaction;
- Planning abort returns the exact lease;
- dropping an internal lease without explicit transfer releases the reservation idempotently;
- successful freeze transfers/commits the reservation exactly once into the later plan-set lifecycle; and
- 0063 does not choose the final allocator, plan-slot table, or generation algorithm.

The two executable fixtures model the rejection, abort, drop, and successful-transfer ownership paths. A failed `enter_planning` is not `Result<_, ()>` that silently destroys the reservation.

## 3. Typed TD admission WorkClass and exact unit

The accepted Foundation migration must append one work class:

```text
WorkClass::TypedTdAdmissionItems
```

It is not an alias or reinterpretation of `JsonSchemaNodes`.

One unit means one transition from this table:

| Transition | Units |
| --- | ---: |
| inspect one scalar/optional TD field for presence/value shape | 1 |
| visit one key/value entry in a typed map | 1 |
| visit one element in a typed sequence | 1 |
| visit one nested `serde_json::Value` node in extension resource census | 1 |
| evaluate one Basic semantic predicate not already fused into the charged field/map/sequence visit | 1 |
| advance/pop a bounded traversal frame without inspecting a new TD value | 0 |
| account UTF-8 bytes of an already visited string | 0 additional work units; bytes use typed string-byte resource |

Typed admission uses a distinct lifetime row:

```text
typed_td_admission_work_units_max
```

The existing `document_validation_work_units_max` remains RawJson/document-path authority and is not silently reinterpreted.

## 4. First-proof resource-schema migration disposition

Legend:

- **RawJson-only** — not applicable to `TypedThingBorrowed`; no serialization proxy.
- **Logical-TD unchanged** — existing representation-independent WoT structure.
- **Engine-memory unchanged** — existing physical engine-owned memory meaning.
- **Typed replacement** — old row is non-applicable and a typed identity is introduced.
- **Plan-build active** — existing active Planning/plan-set/artifact authority that must be present in the checked projection before the relevant phase.
- **Deferred lifecycle** — value is carried in the checked snapshot for the same active PLAN-SET authority but first consumed only by later 0062 publication/drain/reclaim work.
- **Inactive domain** — requirement/domain is not activated by the first Consumer proof.

### 4.1 Typed source and validation rows

| Existing field | `TypedThingBorrowed` disposition | First-proof rule |
| --- | --- | --- |
| `document_bytes_max` | RawJson-only | NA; never reserialize `Thing` to invent source bytes |
| `string_bytes_max` | Typed replacement | typed source uses `typed_td_string_bytes_per_thing_max` |
| `extension_bytes_max` | RawJson-only | extension growth bounded by typed structure/node/string rows, not encoded-byte proxy |
| `json_nesting_depth_max` | Typed replacement | typed uses `typed_td_nesting_depth_max` |
| `json_members_per_object_max` | Typed replacement | typed uses `typed_td_members_per_map_max` |
| `json_array_items_max` | Typed replacement | typed uses `typed_td_items_per_sequence_max` |
| `json_value_nodes_per_document_max` | Typed replacement | typed uses `typed_td_value_nodes_per_thing_max` |
| `document_validation_work_units_max` | Typed replacement | typed uses `typed_td_admission_work_units_max` |
| `affordances_per_thing_max` | Logical-TD unchanged | required |
| `forms_per_context_max` | Logical-TD unchanged | required |
| `forms_per_thing_max` | Logical-TD unchanged | required |
| `additional_responses_per_form_max` | Logical-TD unchanged | census only; does not activate broad additional-response behavior |
| `uri_variables_per_form_max` | Logical-TD unchanged | required |
| `schema_nodes_per_document_max` | Logical-TD unchanged | required typed DataSchema-node census |
| `schema_composition_depth_max` | Logical-TD unchanged | required |
| `schema_reference_edges_per_document_max` | Logical-TD unchanged | required |
| `uri_template_source_bytes_max` | Logical-TD unchanged | required for selected form template source |
| `uri_template_variables_max` | Logical-TD unchanged | required when template variables are compiled |
| `form_binding_candidates_per_operation_max` | Logical-TD/Planning unchanged | required candidate bound |

New typed rows:

| New field | Unit | Scope |
| --- | --- | --- |
| `typed_td_nesting_depth_max` | depth | per-thing |
| `typed_td_members_per_map_max` | items | per-map |
| `typed_td_items_per_sequence_max` | items | per-sequence |
| `typed_td_value_nodes_per_thing_max` | nodes | per-thing |
| `typed_td_string_bytes_per_thing_max` | bytes | per-thing |
| `typed_td_admission_work_units_max` | items | per-admission |

Nested extension `serde_json::Value` consumes the typed resource census above while Basic semantic validation continues to treat unknown extension semantics according to the TD contract.

### 4.2 Admission/physical-memory rows

| Existing field | Disposition |
| --- | --- |
| `retained_source_bytes_per_owner_max` | Engine-memory unchanged; borrowed first-proof source contributes exactly zero |
| `retained_source_bytes_global_max` | Engine-memory unchanged; borrowed first-proof source contributes exactly zero |
| `admission_temporary_bytes_per_operation_max` | required |
| `admission_temporary_bytes_global_max` | required |
| `peak_live_bytes_per_admission_max` | required |
| `admission_peak_live_bytes_global_max` | required |
| `engine_live_bytes_global_max` | required |
| `largest_contiguous_allocation_bytes_max` | required for every real Host allocation or exclusive static reservation |
| `compiled_runtime_bytes_per_thing_max` | Plan-build active |
| `compiled_runtime_bytes_global_max` | Plan-build active |
| `generated_effective_document_bytes_max` | NA in first proof because no effective-document representation is materialized |
| `validator_cache_bytes_per_owner_max` | Inactive domain |
| `validator_cache_bytes_global_max` | Inactive domain |

### 4.3 Active PLAN-COST / PLAN-SET / PLAN-ARTIFACT rows

The first-proof checked policy must not stop at `compiled_plan_bytes_max`. The following current schema rows are explicitly dispositioned:

| Existing field | First-proof disposition |
| --- | --- |
| `compiled_plan_bytes_max` | Plan-build active; total compiled logical + artifact representation must fit |
| `logical_plan_bytes_per_thing_max` | Plan-build active; reserve/preflight before logical-plan allocation/build |
| `plan_sets_per_thing_max` | Deferred lifecycle; carried for 0062 plan-set owner |
| `plan_sets_global_max` | Deferred lifecycle; carried for 0062 plan-set owner |
| `plan_pins_per_plan_set_max` | Deferred lifecycle; carried for 0062 pin authority |
| `plan_pins_global_max` | Deferred lifecycle; carried for 0062 pin authority |
| `binding_artifacts_per_thing_max` | Plan-build active |
| `binding_artifacts_global_max` | Plan-build active |
| `binding_artifact_bytes_per_item_max` | Plan-build active; compare final measured artifact footprint |
| `binding_artifact_bytes_per_thing_max` | Plan-build active |
| `binding_artifact_bytes_global_max` | Plan-build active |
| `binding_compiler_cursor_bytes_per_item_max` | Plan-build active; compare `BindingCompilerBounds::cursor_bytes()` |
| `binding_compiler_cursor_bytes_global_max` | Plan-build active |
| `plan_compile_work_units_per_step_max` | Plan-build active; caps the total child WorkBudget grant for one compiler callback |
| `plan_reclaim_bytes_per_step_max` | Deferred lifecycle; carried for later 0062 reclamation |

`lazy_artifact_*`, lazy waiters, cache/index/probe rows owned by inactive `PLAN-LAZY-001`, `PLAN-CACHE-001`, or `PLAN-INDEX-001` remain inactive and are not pulled into the first proof merely because they are present in the exhaustive schema.

Rows belonging only to payload/codec, Directory/query, subscription/emission, Producer route state, runtime calls, or other inactive capability families retain their existing semantics and are not activated by 0063.

## 5. Complete checked Consumer policy projection

The first-proof authority is an immutable checked projection, not raw `ResourceLimits`:

```text
TypedThingBorrowedConsumerPolicyV1 {
  schema_revision,
  profile_id,
  profile_value_digest,
  execution_cell, // Host | ApplicationStatic

  // typed source / semantic admission
  typed_td_nesting_depth_max,
  typed_td_members_per_map_max,
  typed_td_items_per_sequence_max,
  typed_td_value_nodes_per_thing_max,
  typed_td_string_bytes_per_thing_max,
  typed_td_admission_work_units_max,
  affordances_per_thing_max,
  forms_per_context_max,
  forms_per_thing_max,
  additional_responses_per_form_max,
  uri_variables_per_form_max,
  schema_nodes_per_document_max,
  schema_composition_depth_max,
  schema_reference_edges_per_document_max,
  uri_template_source_bytes_max,
  uri_template_variables_max,
  form_binding_candidates_per_operation_max,

  // admission / engine memory
  retained_source_bytes_per_owner_max,
  retained_source_bytes_global_max,
  admission_temporary_bytes_per_operation_max,
  admission_temporary_bytes_global_max,
  peak_live_bytes_per_admission_max,
  admission_peak_live_bytes_global_max,
  engine_live_bytes_global_max,
  largest_contiguous_allocation_bytes_max,
  compiled_runtime_bytes_per_thing_max,
  compiled_runtime_bytes_global_max,

  // active/deferred plan-set authority captured in the same snapshot
  compiled_plan_bytes_max,
  logical_plan_bytes_per_thing_max,
  plan_sets_per_thing_max,
  plan_sets_global_max,
  plan_pins_per_plan_set_max,
  plan_pins_global_max,
  binding_artifacts_per_thing_max,
  binding_artifacts_global_max,
  binding_artifact_bytes_per_item_max,
  binding_artifact_bytes_per_thing_max,
  binding_artifact_bytes_global_max,
  binding_compiler_cursor_bytes_per_item_max,
  binding_compiler_cursor_bytes_global_max,
  plan_compile_work_units_per_step_max,
  plan_reclaim_bytes_per_step_max,
}
```

Construction rules:

1. bind exact revised schema identity, `consumer` role, Consumer Property Read one-shot domain, Host/static execution cell, `TypedThingBorrowed`, profile origin, and profile-value digest first;
2. every field above resolves to a concrete value under that applicability set; applicable `None` is a construction error;
3. RawJson-only fields are explicitly NA for this typed projection rather than absent by accident;
4. inactive lazy/cache/index/payload/subscription families remain excluded by authority, not because a checked builder forgot them;
5. zero retains each row's declared zero semantics and never means unbounded; and
6. schema/profile/cell/representation cannot rotate while admission is live.

The executable fixture represents the projection as nested non-optional typed-TD, memory, and Planning policy structs. It is intentionally larger than the earlier four-field model and includes the active plan/artifact/cursor/per-step controls above.

## 6. BindingCompilerBounds and the real compiler-step SPI

The current SPI is:

```rust
fn step(
    &self,
    input: &BindingCompilerInput<'_>,
    cursor: Self::Cursor,
    budget: &mut WorkBudget,
) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;
```

Stage A does **not** propose adding a second lifetime-budget parameter to that SPI.

Instead the Planning admission wrapper constructs one joint work reservation:

```text
compiler lifetime WorkBudget from BindingCompilerBounds
      + caller current-step WorkBudget
      + plan_compile_work_units_per_step_max
      -> reserve jointly before compiler work
      -> bounded child WorkBudget
      -> existing BindingCompilerExtension::step(..., &mut child)
      -> reconcile unused child allowance
```

Required semantics:

1. determine per-class grants from the intersection of remaining compiler-lifetime and caller-step counters;
2. cap the sum of grants by `plan_compile_work_units_per_step_max`;
3. debit/reserve both parent authorities before invoking the compiler;
4. hold exclusive mutable ownership of both parents for the callback/reconciliation window;
5. pass only the child `WorkBudget` to the existing compiler SPI;
6. the compiler cannot consume more than the reservation because it sees only the child;
7. after the callback, unused child units are returned to both parents and actual consumed units remain charged exactly once; and
8. zero available grant causes no compiler callback/progress.

The current-head fixture implements a real `BindingCompilerExtension`, invokes its real `step` method with the child budget, and asserts no callback under zero caller budget, exact lifetime/caller debit after progress, and per-step capping.

Stage-A conclusion: **the current Core compiler-step signature is constructible without modification for this work-meter requirement**. WP-300 still requires ADR-0013 reaffirmation because the compiler source and admitted wrapper semantics changed, but this finding alone does not require reopening Core/WP-300 public SPI.

If Stage-B implementation discovers that the reservation/reconciliation primitive cannot remain in Foundation/Planning without changing the Core SPI, WP-300 must escalate from reaffirmation to reopen before source work.

## 7. Complete BindingCompilerBounds resource ownership

For one selected complete registration and exact compiler input:

```text
same registration entry
  -> compiler.bounds(input) exactly once
  -> capture artifact footprint + cursor bytes + peak temporary bytes + lifetime WorkBudget
  -> preflight/reserve every applicable local/global/peak/contiguous/plan/artifact/cursor row
  -> only then compiler.start(input)
```

The first-proof reservation checks at minimum:

- logical plan bytes against `logical_plan_bytes_per_thing_max` and `compiled_plan_bytes_max` before logical-plan materialization;
- compiler cursor against per-item/global cursor rows and contiguous limit;
- compiler temporary bytes against local/global admission temporary and contiguous limits;
- final artifact count against per-Thing/global artifact-count rows;
- final artifact bytes against per-item/per-Thing/global artifact-byte rows;
- compiled logical + artifact bytes against `compiled_plan_bytes_max`;
- retained artifact/runtime bytes against compiled-runtime per-Thing/global and engine-live limits; and
- lifetime compiler work plus each real step through the child-budget reservation above.

No compiler `start` occurs when those declarations cannot be admitted.

## 8. Concrete Stage-A Host/static physical storage

The earlier placeholder `state_words` arrays are removed.

The executable fixture now defines one real inline state union:

```text
AdmissionStateSlot<'td, 'reg> = union {
  BorrowedTdCursor<'td>,
  Validating<'td, 'reg>,
  Validated<'td, 'reg>,
  Planning<'td, 'reg>,
}
```

Each union alternative is wrapped in `ManuallyDrop`, so the union's compiled size/alignment is the actual maximum required by the modeled inline states. The fixture asserts:

- `size_of::<AdmissionStateSlot>() >= size_of` of every modeled alternative;
- `align_of::<AdmissionStateSlot>() >= align_of` every modeled alternative;
- the Host state region is at least the union size;
- the static state region is at least the union size; and
- the enclosing Host/static alignment is sufficient for that state slot.

Both concrete `#[repr(C)]` enclosures also contain a real `FailureSlot`, `AccountingStorage`, and `CompilerStorage`. `CompilerStorage` has actual fixed fixture regions for the declared compiler cursor, peak temporary bytes, final artifact bytes, and lifetime work budget; the fixture asserts the compiler region physically fits that storage.

The layouts are partitioned by actual `offset_of` values into structural/state/diagnostic/accounting/compiler regions with no gaps or overlap in attribution.

Important boundary: this proves the **inline owner/control-block and fixture-reserved compiler regions are physically constructible**. Heap-backed bytes owned by a future Host logical plan/artifact representation are separate real allocations and must be charged under the logical-plan/artifact/runtime/contiguous rows above; Stage A does not hide those bytes inside the inline union. Application-static implementations may instead use exclusive pre-reserved buffers, but those buffers require the same byte attribution and contiguous checks.

Stage B chooses actual production Host/static representations. Stage C measures those production representations; it does not retroactively validate the Stage-A placeholder because there is no placeholder state region left.

## 9. Current-head executable coverage

`planning/tests/consumer_admission_stage_a.rs` demonstrates:

| Stage-A property | Proof |
| --- | --- |
| borrowed TD cursor topology | real borrowed iterators over caller-owned `Thing` |
| exact PlanId + PlanSetGeneration reservation | one opaque lease |
| rejection/abort/drop reservation release | returned lease + explicit release + RAII fallback tests |
| snapshot ordinal vs diagnostic ordinal | `3` vs `17` fixture |
| same complete registration supplies identity and compiler | equal-compatibility competitor receives zero compiler calls |
| complete compiler bounds before start | memory rejection leaves compiler start count zero |
| checked policy carries active plan/artifact/cursor/per-step controls | non-optional typed policy structs used by admission accounting |
| real SPI work metering | child WorkBudget passed to actual `BindingCompilerExtension::step` |
| compiler lifetime/caller-step/per-step cap | paired reservation and reconciliation assertions |
| typed TD lifetime + step work | failure-atomic typed meter fixture |
| Host/static modeled state physically fits | real union + size/alignment/offset assertions |

`planning/tests/consumer_admission_stage_a_pending.rs` separately proves that one move-only Planning transaction is the only resumable owner across `Pending`, reconstructs ephemeral input internally, accepts no replacement source/snapshot/lease/raw plan identity, and now also returns/releases its lease on rejection/abort.

## 10. ADR-0013 impact disposition

| Authority/tranche | Stage-A disposition | Stage-B obligation |
| --- | --- | --- |
| Foundation resource schema/work/accounting | affected | revise typed-ingestion applicability; append `TypedTdAdmissionItems`; add accepted paired-work reservation/reconciliation primitive if shared rather than Planning-private |
| TD Basic validation substrate | affected | admit shared bounded Basic engine/cursor; synchronous API later adapts over same semantic engine |
| `WP-200-CONSUMER-PROPERTY-READ-PLANNING` | **must reopen** | replace raw admitted Consumer bypass with validated/lease/same-registration-derived contract and complete resource enforcement |
| `WP-300-CONSUMER-PROPERTY-READ-BINDING` | **affected; reaffirmation required** | re-review same-entry compiler sourcing and confirm existing `BindingCompilerExtension::step(..., &mut WorkBudget)` remains sufficient with the child-budget wrapper |
| WP-300 evidence `consumer-property-read-binding-execution` | affected | prove the same complete registration supplies identity/compiler and that no alternate compiler/SPI path bypasses admitted Planning |
| future Consumer WP-400 Servient tranche | not yet admitted | later admission depends on migrated 0063 + 0062 |
| shared Producer Planning surfaces | affected by WP-200 migration | explicit transitive compatibility/behavior disposition |
| `WP-100-CONSUMER-CALL-VALUES-VALIDATOR` | no direct semantic change | remains predecessor; broad response/codec domains stay inactive |

A Core/WP-300 reopen is **not currently required solely for paired compiler work**, because Stage A now demonstrates that the existing single-budget compiler SPI accepts a jointly reserved child budget. Reopen remains mandatory if Stage-B source design needs any Core public/source contract change.

## 11. Stage-A closure boundary

0063 may move from `DISCUSSING` to `DECIDED` only if a fresh independent reviewer accepts the current-head artifacts together and concludes:

- the build lease binds identity and reservation lifetime with recoverable rejection/abort paths;
- checked policy projection has no active first-proof Planning/artifact/cursor/per-step applicability holes;
- complete compiler bounds are admitted before start;
- lifetime + caller-step work is constructibly enforced through the real existing compiler SPI;
- concrete modeled state actually fits both Host/static Stage-A storage definitions;
- WP-200 reopening and WP-300 reaffirmation/escalation rules satisfy ADR-0013; and
- no Stage-C runtime proof is being required before implementation admission.

Acceptance of Stage A still does not authorize production Rust implementation. Stage-B authority migration and independent implementation admission remain mandatory.