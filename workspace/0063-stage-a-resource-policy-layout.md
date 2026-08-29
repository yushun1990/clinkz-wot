# 0063 Stage-A Resource, Work, Layout, and Impact Definitions

Status: NON-PRODUCTION CONSTRUCTIBILITY EVIDENCE

Owner topic: `workspace/0063-bounded-validated-consumer-admission-input.md`

This artifact exists only to make the Stage-A candidate falsifiable before 0063 may become `DECIDED`. It does not change the active resource schema, public API, work-package status, or implementation admission by itself.

The executable companion is:

- `tools/design-check/tests/consumer_admission_stage_a.rs`

That fixture is compiled by the existing workspace test path and models the ownership/typestate/layout shape without creating the future production Consumer admission API.

## 1. One unpublished build lease binds both plan identities

The Stage-A boundary no longer consumes a token that proves only `PlanId`.

The upstream 0062 / Servient plan-set identity authority must issue one opaque move-only unpublished build lease carrying one indivisible pair:

```text
UnpublishedPlanBuildLease
  = exact PlanId
  + exact PlanSetGeneration
```

Properties:

- the lease has no public constructor from raw `PlanId` or raw `PlanSetGeneration`;
- one issuer reserves both values under the same unpublished plan-set build;
- Consumer Planning accepts the lease as one value and has no independent generation parameter;
- ephemeral Planning input reconstruction reads both identities from that same lease;
- abort returns/releases the reservation through the upstream issuer;
- successful freeze transfers the pair into the later plan-set lifecycle; and
- 0063 does not choose the allocator/slot-generation algorithm used by the issuer.

`tools/design-check/tests/consumer_admission_stage_a.rs` models this as `UnpublishedPlanBuildLease` and proves that `PlanId` plus `PlanSetGeneration` are reconstructed from one lease.

## 2. Typed TD admission receives one new WorkClass with an exact unit

The accepted migration must append one Foundation work class:

```text
WorkClass::TypedTdAdmissionItems
```

It is **not** an alias or reinterpretation of `JsonSchemaNodes`.

One `TypedTdAdmissionItems` unit means exactly one predeclared bounded TD admission transition from this table:

| Transition | Work units |
| --- | ---: |
| inspect one scalar/optional TD field for presence/value shape | 1 |
| visit one key/value entry in a typed map (`BTreeMap` or equivalent) | 1 |
| visit one element in a typed sequence (`Vec`/slice or equivalent) | 1 |
| visit one nested `serde_json::Value` node in extension resource census | 1 |
| evaluate one Basic semantic predicate that is not already fused into the charged field/map/sequence visit | 1 |
| advance or pop one bounded traversal frame without inspecting a new TD value | 0 |
| account UTF-8 bytes of one already-visited string | 0 additional work units; bytes are charged to the typed string-byte resource |

A production step may batch `n` transitions only when `n` is known before the batch and both the lifetime meter and current step meter are atomically charged by `n` before the first transition.

The corresponding TypedThingBorrowed lifetime ceiling is a new schema row:

```text
typed_td_admission_work_units_max
```

The existing `document_validation_work_units_max` is not silently redefined for the typed representation.

## 3. Exhaustive first-proof migration disposition for existing document/input rows

This table is exhaustive for every existing resource row whose current semantic unit is a TD/document/schema input unit or whose memory/work limit is directly exercised by the first `TypedThingBorrowed` Consumer admission. Rows outside this set (payload/codec, Directory/query, subscriptions/emission, cleanup, cache/lazy planning, resolver, Producer-only route state, and later capability families) retain their current authority and are not activated by 0063.

Legend:

- **RawJson-only** — not applicable to `TypedThingBorrowed`; no serialization proxy is permitted.
- **Logical-TD unchanged** — the existing semantic unit is already representation-independent and remains applicable after the schema revision.
- **Engine-memory unchanged** — same physical engine-owned memory meaning; applicability depends on the actual representation/materialization.
- **Derived-only** — applies only if the derived representation is actually materialized.
- **Typed replacement** — old row is non-applicable to typed input and a new typed row is introduced.
- **Inactive first proof** — remains defined in the schema but does not enter the checked first-proof policy.

| Existing field | TypedThingBorrowed disposition | First-proof authority |
| --- | --- | --- |
| `document_bytes_max` | RawJson-only | `None` / NA; never reserialize `Thing` to invent source bytes |
| `string_bytes_max` | RawJson-only | replaced by `typed_td_string_bytes_per_thing_max` |
| `extension_bytes_max` | RawJson-only | no encoded-byte proxy; extension growth is bounded by typed node/map/sequence/string rows |
| `generated_effective_document_bytes_max` | Derived-only | required only if effective-document materialization occurs; first proof does not materialize it |
| `retained_source_bytes_per_owner_max` | Engine-memory unchanged | applicable account exists, but borrowed source contribution is exactly zero |
| `retained_source_bytes_global_max` | Engine-memory unchanged | applicable account exists, but borrowed source contribution is exactly zero |
| `admission_temporary_bytes_per_operation_max` | Engine-memory unchanged | required `Some` in checked Consumer policy |
| `admission_temporary_bytes_global_max` | Engine-memory unchanged | required `Some` in checked Consumer policy |
| `peak_live_bytes_per_admission_max` | Engine-memory unchanged | required `Some` |
| `admission_peak_live_bytes_global_max` | Engine-memory unchanged | required `Some` |
| `engine_live_bytes_global_max` | Engine-memory unchanged | required `Some` |
| `largest_contiguous_allocation_bytes_max` | Engine-memory unchanged | required `Some`; applies to each real allocation/exclusive static reservation |
| `compiled_plan_bytes_max` | Engine-memory unchanged | required for Planning draft/artifact reservation |
| `compiled_runtime_bytes_per_thing_max` | Engine-memory unchanged | required when Planning/runtime reservation begins |
| `compiled_runtime_bytes_global_max` | Engine-memory unchanged | required when Planning/runtime reservation begins |
| `validator_cache_bytes_per_owner_max` | Inactive first proof | no validator cache is activated |
| `validator_cache_bytes_global_max` | Inactive first proof | no validator cache is activated |
| `json_nesting_depth_max` | Typed replacement | RawJson-only; typed uses `typed_td_nesting_depth_max` |
| `json_members_per_object_max` | Typed replacement | RawJson-only; typed uses `typed_td_members_per_map_max` |
| `json_array_items_max` | Typed replacement | RawJson-only; typed uses `typed_td_items_per_sequence_max` |
| `json_value_nodes_per_document_max` | Typed replacement | RawJson-only; typed uses `typed_td_value_nodes_per_thing_max` |
| `affordances_per_thing_max` | Logical-TD unchanged | required `Some`; counts typed Property/Action/Event affordances |
| `forms_per_context_max` | Logical-TD unchanged | required `Some`; applies to each typed form-owning context |
| `forms_per_thing_max` | Logical-TD unchanged | required `Some`; total typed forms in one Thing |
| `additional_responses_per_form_max` | Logical-TD unchanged | required for resource census only; does not activate broad response behavior |
| `uri_variables_per_form_max` | Logical-TD unchanged | required `Some` |
| `schema_nodes_per_document_max` | Logical-TD unchanged | required `Some`; semantic unit remains one logical typed `DataSchema` node in the TD, independent of source encoding |
| `schema_composition_depth_max` | Logical-TD unchanged | required `Some` |
| `schema_reference_edges_per_document_max` | Logical-TD unchanged | required `Some`; logical TD schema-reference edge count |
| `document_validation_work_units_max` | Typed replacement | remains for its existing document path; typed admission uses `typed_td_admission_work_units_max` |
| `uri_template_source_bytes_max` | Logical-TD unchanged | required when the selected typed Form contains a URI template source |
| `uri_template_variables_max` | Logical-TD unchanged | required when URI-template variables are compiled |
| `form_binding_candidates_per_operation_max` | Logical Planning unchanged | required by the first Planning candidate bound; not a source byte/shape proxy |
| `things_global_max` | Engine/runtime unchanged | checked at composition/registry admission, not charged by TD traversal itself |
| `bindings_global_max` | Engine/runtime unchanged | checked by composition/registration ownership, not reinterpreted as TD shape |

### New typed-input rows in the next schema revision

The minimal additive typed rows are:

| New field | Unit | Scope | Meaning |
| --- | --- | --- | --- |
| `typed_td_nesting_depth_max` | depth | per-thing | maximum nested typed TD/container/extension-value depth traversed by census |
| `typed_td_members_per_map_max` | items | per-map | maximum key/value entries in any typed map or extension object |
| `typed_td_items_per_sequence_max` | items | per-sequence | maximum elements in any typed sequence or extension array |
| `typed_td_value_nodes_per_thing_max` | nodes | per-thing | total typed TD/container/extension value nodes visited by census |
| `typed_td_string_bytes_per_thing_max` | bytes | per-thing | sum of UTF-8 bytes of typed strings visited by census; no serialization overhead |
| `typed_td_admission_work_units_max` | items | per-admission | cumulative `WorkClass::TypedTdAdmissionItems` lifetime allowance |

Nested `serde_json::Value` extension data consumes the typed depth/map/sequence/node/string resources. That is resource census only; it does not claim extension semantic validation.

## 4. Complete checked Consumer policy projection

The first-proof policy is not `ResourceLimits`. It is a checked immutable projection produced only after representation/cell/profile applicability has been validated.

Conceptual exact field set:

```text
TypedThingBorrowedConsumerPolicyV1 {
  schema_revision,
  profile_id,
  profile_value_digest,
  execution_cell,                    // Host | ApplicationStatic

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
  retained_source_bytes_per_owner_max,   // account exists; borrowed contribution = 0
  retained_source_bytes_global_max,      // account exists; borrowed contribution = 0
}
```

Construction rules:

1. bind exact revised schema identity, `consumer` role, `Consumer Property Read one-shot`, execution cell, `TypedThingBorrowed`, and profile origin/value digest first;
2. every field listed above must resolve to `Some(limit)` under that applicability set;
3. all RawJson-only fields (`document_bytes_max`, `string_bytes_max`, `extension_bytes_max`, `json_*`, existing `document_validation_work_units_max`) must be `None` / NA for this typed projection;
4. validator-cache fields remain outside the first-proof projection because validator caching is inactive;
5. zero values retain the row's declared disabled/rendezvous semantics and are not converted to unbounded values;
6. after successful construction, the projection contains no `Option<u64>` for an applicable limit and cannot switch schema/profile/cell/representation while an admission is live.

A raw `ResourceLimits` value with any illegal applicability combination cannot produce this handle and therefore cannot start TD census.

## 5. Concrete Stage-A Host/static physical storage definitions

The executable fixture defines two real `#[repr(C)]` enclosing types:

```text
HostAdmissionStorage<'td, 'reg>
StaticAdmissionStorage<'td, 'reg>
```

Both contain actual borrowed source/snapshot pointers, cancellation generation, fixed state storage, one `FailureSlot`, accounting storage, and compiler-bound storage. The Host and application-static types deliberately have different state capacity so their layouts are measured independently.

`FailureSlot` is a real `union` whose alternatives are:

```text
ManuallyDrop<ValidationIssue>
ManuallyDrop<CoreError>
```

Therefore its size/alignment is the actual maximum required by those two fixed failure carriers on the compiled target; it is not a guessed `N` and not a second allocation.

### Physical attribution rule

For each concrete enclosing type, the fixture computes:

```text
total_size = size_of::<EnclosingStorage>()
alignment  = align_of::<EnclosingStorage>()
```

and uses `offset_of!` to partition the one enclosing allocation/slot into exactly five contiguous non-overlapping regions:

1. `structural = [0, state_offset)` — tag, borrowed pointers, cancellation identity, and all leading/interstitial padding before state;
2. `state = [state_offset, failure_offset)` — current admission typestate/cursor region plus padding up to failure;
3. `diagnostic = [failure_offset, accounting_offset)` — the real `FailureSlot` region plus any padding immediately following it;
4. `accounting = [accounting_offset, compiler_offset)` — local/global accounting owner data plus padding up to compiler state;
5. `compiler = [compiler_offset, total_size)` — compiler reservations/lifetime work plus trailing padding.

The fixture asserts the ranges start at zero, touch without gaps/overlap, and end exactly at `total_size`.

Consequences:

- every physical byte in the enclosing allocation/slot has exactly one attribution owner;
- padding cannot disappear or be double-counted;
- diagnostic bytes correspond to an actual region of the enclosing storage;
- current/peak live accounting charges the enclosing storage once;
- `largest_contiguous_allocation_bytes_max` compares against `total_size` for the enclosing Host allocation/static exclusive slot, not a sum of logical field sizes; and
- target-specific ABI differences are legitimate: Stage C records actual Host/static production layouts separately rather than freezing one cross-target numeric size in this investigation document.

The Stage-A fixture is a constructibility model, not the production layout. Stage B must migrate the accepted attribution rule to the actual Servient storage types before implementation admission; Stage C verifies those production layouts.

## 6. Constructibility fixture coverage

`tools/design-check/tests/consumer_admission_stage_a.rs` provides current-head executable model evidence for:

| Stage-A property | Fixture |
| --- | --- |
| borrowed external TD can coexist with retained traversal iterators without self-reference | `borrowed_td_cursor_is_constructible_without_source_ownership` |
| one opaque lease binds `PlanId` + `PlanSetGeneration` | `build_lease_binds_plan_id_and_plan_set_generation_together` |
| snapshot ordinal and diagnostic ordinal remain distinct | `ordinal_domains_remain_distinct_and_same_entry_derives_compiler_identity` |
| equal-compatibility registrations cannot inject an independent compiler identity; selected compiler facts come from the exact snapshot entry | same ordinal/same-entry fixture; competing entry receives zero `bounds/start` calls |
| compiler bounds are obtained exactly once per attempt and memory rejection occurs before compiler start | `compiler_bounds_are_reserved_before_start_and_owned_after_entry` |
| captured compiler lifetime work survives entry | same bounds fixture |
| compiler lifetime + current step charge is failure-atomic and step replenishment cannot reset lifetime | `compiler_pair_charge_is_atomic_and_step_replenishment_cannot_reset_lifetime` |
| proposed typed-TD lifetime + current step charge has the same atomic semantics | `typed_td_pair_charge_has_the_same_failure_atomicity` |
| Host/static enclosing layouts are concrete, separately measured, fully attributed, non-overlapping, and contain the real maximum fixed failure slot | `host_and_static_layouts_cover_one_enclosing_allocation_without_overlap` |

This fixture intentionally does **not** claim production TD semantic traversal, publication, cancellation runtime behavior, global concurrency behavior, or final public API completion. Those remain Stage C after admission.

## 7. ADR-0013 impact disposition, including completed WP-300 Consumer tranche

| Authority/tranche | Stage-A impact disposition | Required Stage-B action |
| --- | --- | --- |
| Foundation resource schema / work budget substrate | affected | revise schema/applicability, append `TypedTdAdmissionItems`, add typed rows and atomic pair-charge/hierarchical accounting primitives under independent admission |
| TD Basic validation substrate | affected | admit shared bounded Basic engine/cursor work; synchronous API later adapts over the same engine |
| `WP-200-CONSUMER-PROPERTY-READ-PLANNING` | **must reopen** | replace the admitted raw Consumer Planning bypass with the sealed validated/lease/registration-derived contract; shared Producer API impact reviewed explicitly |
| `WP-300-CONSUMER-PROPERTY-READ-BINDING` | **affected; reaffirmation required before 0063/0062 may rely on it** | re-run impact/evidence review against same-entry compiler sourcing. Current complete Host/static registration design already owns one registration identity and one compiler component in the same validated bundle, so 0063 does not presently require a WP-300 public/source change. If Stage-B migration discovers that a new accessor/ownership/API contract is required, ADR-0013 escalates this tranche from reaffirmation to reopen before implementation. |
| WP-300 completion evidence `consumer-property-read-binding-execution` | affected | add/reaffirm evidence that the exact complete registration entry supplying `identity()` is also the entry supplying the compiler component consumed by sealed Planning; equal compatibility alone is insufficient |
| future Consumer WP-400 Servient tranche | not yet admitted/complete | its later admission depends on migrated 0063 + 0062 prerequisites; there is no completed Consumer WP-400 tranche to reopen now |
| shared Producer Planning surfaces | affected by WP-200 public API migration | explicit transitive compatibility/behavior disposition before WP-200 reimplementation; Producer behavior is not presumed unchanged |
| `WP-100-CONSUMER-CALL-VALUES-VALIDATOR` | no direct semantic change from 0063 | remains predecessor evidence; broad response validator/codec domains are not activated |

The WP-300 disposition is intentionally specific: **the completed Consumer binding tranche cannot remain merely `impact_status = current` by assumption; it requires an explicit reaffirmation decision after the WP-200/0063 migration shape is accepted.** Its current complete-registration architecture is evidence for reaffirmation, not permission to skip impact review.

## 8. Stage-A closure boundary

0063 may move from `DISCUSSING` to `DECIDED` only if an independent reviewer accepts all of the following together:

- the opaque unpublished build lease binds `PlanId` and `PlanSetGeneration` as one authority;
- the compile/model fixture demonstrates the borrow/typestate/same-registration/bounds/work/layout topology at the reviewed head;
- `TypedTdAdmissionItems` and its exact unit mapping are acceptable Foundation additions rather than a reinterpretation of `JsonSchemaNodes`;
- the resource migration table is complete for first-proof document/input/admission rows and the checked policy projection has no unresolved applicability holes;
- the Host/static layout attribution rule is physically coherent and falsifiable;
- WP-200 reopening and WP-300 reaffirmation-required dispositions are acceptable under ADR-0013; and
- no Stage-C runtime proof is being used as a prerequisite for the decision itself.

Acceptance of this Stage-A artifact still does not authorize production Rust implementation. Stage B authority migration and independent implementation admission remain mandatory.
