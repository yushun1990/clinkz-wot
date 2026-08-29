# 0063 Stage-A Resource, Work, Layout, and Impact Definitions

Status: NON-PRODUCTION CONSTRUCTIBILITY EVIDENCE

Owner topic: `workspace/0063-bounded-validated-consumer-admission-input.md`

This artifact makes the Stage-A candidate falsifiable before 0063 may become `DECIDED`. It does not change the active resource schema, public API, work-package status, or implementation admission by itself.

Executable companion:

- `planning/tests/consumer_admission_stage_a.rs`

The fixture is compiled by the ordinary locked workspace test path and models ownership/typestate/work/layout constructibility without creating the future production Consumer admission API.

## 1. One unpublished build lease binds both plan identities

The upstream 0062 / Servient plan-set identity authority must issue one opaque move-only unpublished build lease carrying one indivisible pair:

```text
UnpublishedPlanBuildLease
  = exact PlanId
  + exact PlanSetGeneration
```

Rules:

- no admitted public constructor from raw `PlanId` or raw `PlanSetGeneration`;
- one issuer reserves both values for the same unpublished plan-set build;
- Consumer Planning accepts the lease as one value and has no independent generation parameter;
- ephemeral Planning input reconstruction reads both identities from the same lease;
- abort returns/releases the reservation through the upstream owner;
- successful freeze transfers the pair into the later plan-set lifecycle; and
- 0063 does not choose the allocator/slot-generation algorithm.

The executable fixture models this boundary and proves reconstruction of both values from one lease.

## 2. Typed TD admission WorkClass and exact unit

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
| account UTF-8 bytes of an already-visited string | 0 additional work units; bytes use typed string-byte resource |

A production step may batch `n` transitions only when `n` is known before the batch and both lifetime and current step meters are atomically charged by `n` before the first transition.

Typed input uses a distinct lifetime row:

```text
typed_td_admission_work_units_max
```

The existing `document_validation_work_units_max` is not reinterpreted for `TypedThingBorrowed`.

## 3. First-proof resource-schema migration disposition

Legend:

- **RawJson-only** — not applicable to `TypedThingBorrowed`; no serialization proxy.
- **Logical-TD unchanged** — existing semantic unit already names representation-independent WoT structure.
- **Engine-memory unchanged** — physical engine-owned memory meaning is unchanged.
- **Derived-only** — applies only when that derived representation is materialized.
- **Typed replacement** — old row is non-applicable for typed ingestion and a new typed identity is introduced.
- **Inactive first proof** — row remains in the schema but is not projected into this first-proof policy.

The following table covers every current document/input/validation/admission-memory row exercised or intentionally excluded by first-proof typed admission:

| Existing field | `TypedThingBorrowed` disposition | First-proof rule |
| --- | --- | --- |
| `document_bytes_max` | RawJson-only | NA; never reserialize `Thing` to invent source bytes |
| `string_bytes_max` | RawJson-only | replaced by `typed_td_string_bytes_per_thing_max` |
| `extension_bytes_max` | RawJson-only | no encoded-byte proxy; extension growth bounded by typed structural/string rows |
| `generated_effective_document_bytes_max` | Derived-only | only if effective-document representation is actually materialized; first proof does not materialize it |
| `retained_source_bytes_per_owner_max` | Engine-memory unchanged | account exists; borrowed source contribution is exactly zero |
| `retained_source_bytes_global_max` | Engine-memory unchanged | account exists; borrowed source contribution is exactly zero |
| `admission_temporary_bytes_per_operation_max` | Engine-memory unchanged | required `Some` in checked policy |
| `admission_temporary_bytes_global_max` | Engine-memory unchanged | required `Some` |
| `peak_live_bytes_per_admission_max` | Engine-memory unchanged | required `Some` |
| `admission_peak_live_bytes_global_max` | Engine-memory unchanged | required `Some` |
| `engine_live_bytes_global_max` | Engine-memory unchanged | required `Some` |
| `largest_contiguous_allocation_bytes_max` | Engine-memory unchanged | required `Some`; applies to each real allocation/exclusive static reservation |
| `compiled_plan_bytes_max` | Engine-memory unchanged | required for Planning draft/artifact reservation |
| `compiled_runtime_bytes_per_thing_max` | Engine-memory unchanged | required when Planning/runtime reservation begins |
| `compiled_runtime_bytes_global_max` | Engine-memory unchanged | required when Planning/runtime reservation begins |
| `validator_cache_bytes_per_owner_max` | Inactive first proof | no validator cache activated |
| `validator_cache_bytes_global_max` | Inactive first proof | no validator cache activated |
| `json_nesting_depth_max` | Typed replacement | RawJson-only; typed uses `typed_td_nesting_depth_max` |
| `json_members_per_object_max` | Typed replacement | RawJson-only; typed uses `typed_td_members_per_map_max` |
| `json_array_items_max` | Typed replacement | RawJson-only; typed uses `typed_td_items_per_sequence_max` |
| `json_value_nodes_per_document_max` | Typed replacement | RawJson-only; typed uses `typed_td_value_nodes_per_thing_max` |
| `affordances_per_thing_max` | Logical-TD unchanged | required; counts typed Property/Action/Event affordances |
| `forms_per_context_max` | Logical-TD unchanged | required for each typed form-owning context |
| `forms_per_thing_max` | Logical-TD unchanged | required total typed forms per Thing |
| `additional_responses_per_form_max` | Logical-TD unchanged | resource census only; does not activate broad additional-response behavior |
| `uri_variables_per_form_max` | Logical-TD unchanged | required |
| `schema_nodes_per_document_max` | Logical-TD unchanged | logical typed `DataSchema` nodes; representation-independent unit |
| `schema_composition_depth_max` | Logical-TD unchanged | required |
| `schema_reference_edges_per_document_max` | Logical-TD unchanged | logical typed schema-reference edges |
| `document_validation_work_units_max` | Typed replacement | remains document-path authority; typed uses `typed_td_admission_work_units_max` |
| `uri_template_source_bytes_max` | Logical-TD unchanged | required when selected typed Form contains template source |
| `uri_template_variables_max` | Logical-TD unchanged | required when URI-template variables are compiled |
| `form_binding_candidates_per_operation_max` | Logical Planning unchanged | required candidate bound; not a source-shape proxy |
| `things_global_max` | Engine/runtime unchanged | composition/registry authority; not charged as TD traversal shape |
| `bindings_global_max` | Engine/runtime unchanged | registration authority; not reinterpreted as TD shape |

Rows belonging only to payload/codec, Directory/query, subscriptions/emission, cleanup, cache/lazy planning, resolver, Producer route state, or later inactive capability families retain their existing semantics and are not activated by 0063.

### New typed-input rows

| New field | Unit | Scope | Meaning |
| --- | --- | --- | --- |
| `typed_td_nesting_depth_max` | depth | per-thing | max nested typed TD/container/extension-value depth traversed by census |
| `typed_td_members_per_map_max` | items | per-map | max entries in any typed map or extension object |
| `typed_td_items_per_sequence_max` | items | per-sequence | max elements in any typed sequence or extension array |
| `typed_td_value_nodes_per_thing_max` | nodes | per-thing | total typed/container/extension value nodes visited by census |
| `typed_td_string_bytes_per_thing_max` | bytes | per-thing | sum of UTF-8 bytes of typed strings visited; no serialization overhead |
| `typed_td_admission_work_units_max` | items | per-admission | cumulative `TypedTdAdmissionItems` lifetime allowance |

Nested extension `serde_json::Value` consumes typed depth/map/sequence/node/string resources. That remains resource census, not extension semantic validation.

## 4. Complete checked Consumer policy projection

The first-proof authority is a checked immutable projection, not raw `ResourceLimits`:

```text
TypedThingBorrowedConsumerPolicyV1 {
  schema_revision,
  profile_id,
  profile_value_digest,
  execution_cell, // Host | ApplicationStatic

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

  admission_temporary_bytes_per_operation_max,
  admission_temporary_bytes_global_max,
  peak_live_bytes_per_admission_max,
  admission_peak_live_bytes_global_max,
  engine_live_bytes_global_max,
  largest_contiguous_allocation_bytes_max,
  compiled_plan_bytes_max,
  compiled_runtime_bytes_per_thing_max,
  compiled_runtime_bytes_global_max,
  retained_source_bytes_per_owner_max,
  retained_source_bytes_global_max,
}
```

Construction rules:

1. bind exact revised schema identity, `consumer` role, Consumer Property Read one-shot domain, execution cell, `TypedThingBorrowed`, and profile origin/value digest first;
2. every field above must resolve to `Some(limit)` under that applicability set;
3. RawJson-only fields (`document_bytes_max`, `string_bytes_max`, `extension_bytes_max`, `json_*`, and existing `document_validation_work_units_max`) must be NA for this typed projection;
4. validator-cache fields remain outside first proof because caching is inactive;
5. zero retains each row's declared zero semantics and never means unbounded; and
6. after construction, applicable limits are not `Option<u64>` and schema/profile/cell/representation cannot rotate while admission is live.

Raw `ResourceLimits` with illegal applicability cannot produce this policy and cannot start TD census.

## 5. Concrete Stage-A Host/static physical storage definitions

The executable fixture defines two real `#[repr(C)]` enclosing models:

```text
HostAdmissionStorage<'td, 'reg>
StaticAdmissionStorage<'td, 'reg>
```

Both contain actual borrowed source/snapshot pointers, cancellation identity, fixed state region, one `FailureSlot`, accounting storage, and compiler-bound storage. Host and static have deliberately different state capacity and are measured independently.

`FailureSlot` is a real union whose alternatives include fixed `ValidationIssue` and actual `CoreError`; therefore its size/alignment is the concrete maximum required by those modeled fixed carriers on the compiled target, not a guessed extra allocation.

For each enclosing type the fixture computes `size_of`, `align_of`, and `offset_of` and partitions one enclosing allocation/exclusive static slot into five contiguous non-overlapping regions:

1. structural — tag, borrowed pointers, cancellation identity, and leading/interstitial padding before state;
2. state — admission typestate/cursor region plus padding up to failure;
3. diagnostic — real `FailureSlot` plus padding up to accounting;
4. accounting — accounting owner data plus padding up to compiler state;
5. compiler — compiler reservations/lifetime work plus trailing padding.

The fixture asserts the regions start at zero, touch without gaps/overlap, and end exactly at total enclosing size.

Consequences:

- every byte has one attribution owner;
- padding cannot disappear or be double-counted;
- diagnostic attribution names actual storage;
- current/peak live charges the enclosing storage once; and
- `largest_contiguous_allocation_bytes_max` compares with the whole enclosing Host allocation/static exclusive slot, not a sum of field sizes.

These are Stage-A constructibility models, not production Servient layouts. Stage B migrates the accepted rule to the actual storage; Stage C verifies target-specific production layouts.

## 6. Current-head executable fixture coverage

`planning/tests/consumer_admission_stage_a.rs` demonstrates:

| Stage-A property | Current-head proof |
| --- | --- |
| external borrowed TD cursor topology without source self-reference | borrowed cursor fixture |
| one opaque lease binds `PlanId` + `PlanSetGeneration` | build-lease fixture |
| snapshot ordinal and diagnostic ordinal are distinct | ordinal fixture uses `3` vs `17` |
| selected identity/compiler facts come from the same snapshot entry despite equal-compatibility competitor | same-entry fixture; competitor receives zero `bounds/start` calls |
| compiler bounds obtained before start and memory rejection prevents start | compiler-bounds reservation fixture |
| complete compiler lifetime work survives Planning entry | same compiler-bounds fixture |
| compiler lifetime + current step charge is failure-atomic and replenishment cannot reset lifetime | compiler pair-charge fixture |
| proposed typed-TD lifetime + current step charge has same atomic semantics | typed-TD pair-charge fixture |
| Host/static enclosing layouts are separately measured and fully/non-overlappingly attributed | layout fixture |

The model intentionally does not claim production TD semantic traversal, publication, cancellation runtime behavior, concurrent global-account behavior, or final public API completion. Those are Stage C after admission.

## 7. ADR-0013 impact disposition

| Authority/tranche | Stage-A disposition | Stage-B obligation |
| --- | --- | --- |
| Foundation resource schema/work/accounting | affected | revise schema/applicability; append `TypedTdAdmissionItems`; add typed rows and accepted atomic/hierarchical primitives under independent admission |
| TD Basic validation substrate | affected | admit shared bounded Basic engine/cursor; synchronous API later adapts over same engine |
| `WP-200-CONSUMER-PROPERTY-READ-PLANNING` | **must reopen** | replace raw admitted Consumer bypass with validated/lease/same-registration-derived contract; review shared Producer impact |
| `WP-300-CONSUMER-PROPERTY-READ-BINDING` | **affected; explicit reaffirmation required** | re-review same-entry compiler sourcing. Existing complete registration design supports reaffirmation; if migration needs new Core public/source contract, escalate to reopen before implementation |
| WP-300 evidence `consumer-property-read-binding-execution` | affected | prove exact complete registration entry supplying `identity()` is also the compiler-component source; compatibility equality alone is insufficient |
| future Consumer WP-400 Servient tranche | not complete/admitted | later admission depends on migrated 0063 + 0062; nothing completed to reopen now |
| shared Producer Planning surfaces | affected by WP-200 public migration | explicit transitive compatibility/behavior disposition |
| `WP-100-CONSUMER-CALL-VALUES-VALIDATOR` | no direct semantic change | remains predecessor; broad response/codec domains stay inactive |

WP-300 therefore cannot remain `impact_status = current` merely by assumption. It needs an explicit reaffirmation after accepted WP-200/0063 migration shape; current complete-registration architecture is evidence for reaffirmation, not permission to skip the review.

## 8. Stage-A closure boundary

0063 may move from `DISCUSSING` to `DECIDED` only if a fresh independent reviewer accepts the three current-head artifacts together and concludes:

- build lease binds `PlanId` and `PlanSetGeneration` as one authority;
- compile/model evidence demonstrates borrow/typestate/same-registration/bounds/work/layout topology;
- `TypedTdAdmissionItems` and its unit mapping are acceptable additions rather than reinterpretations;
- resource migration and checked policy have no first-proof applicability holes;
- Host/static physical attribution is coherent and falsifiable;
- WP-200 reopening and WP-300 reaffirmation-required dispositions are correct under ADR-0013; and
- no Stage-C runtime proof is being required before implementation is admitted.

Acceptance of Stage A still does not authorize production Rust implementation. Stage-B authority migration and independent implementation admission remain mandatory.
