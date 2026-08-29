# 0063 Consumer Aggregate Resource and Work Applicability

Status: DISCUSSING support material for `0063-consumer-aggregate-admission-plan-set-authority.md`

This note selects the first-proof resource/work applicability rules. It does not change `docs/resource-limits.csv` or Foundation API. A migration must project these decisions into registered authority before implementation admission.

First-proof boundary:

- caller-owned borrowed typed `Thing`, not Raw JSON;
- Consumer one-shot Property Read only;
- deterministic no-material NoSec only;
- exactly one eligible complete Consumer registration selected from immutable startup metadata;
- eager aggregate plans/artifacts; no lazy/cache/fallback;
- no protocol I/O before Frozen;
- publication, pins and request execution occur later under WP-400.

Every current resource row receives one of four meanings for this proof:

- `Active`: enforced or reserved by aggregate admission;
- `ZeroContribution`: the row is semantically relevant but this representation contributes exactly zero;
- `Deferred`: belongs to the same captured runtime profile but is charged only after Frozen/Published;
- `NotApplicable`: excluded capability/representation.

There is no implicit fifth state.

## 1. Typed structural limits: selected migration

The first proof **does not reinterpret the existing `json_*`, `string_bytes_max`, or `extension_bytes_max` rows as limits on an arbitrary directly constructed Rust `Thing`**.

The selected migration is additive: introduce a representation-neutral typed-semantic structural family for direct typed TD admission. Exact registered names may follow schema naming conventions, but the units are fixed by this decision:

- typed semantic node count per document;
- typed semantic depth per document;
- typed map entries per container;
- typed sequence items per container;
- UTF-8 bytes in semantic strings visited by the admitted validator/planner;
- one lifetime work ceiling for typed semantic traversal/validation.

Existing Raw/serialized-document rows remain unchanged for Raw JSON ingestion.

Consequences for current rows:

| Current row | Disposition | Decision |
| --- | --- | --- |
| `document_bytes_max` | `NotApplicable` | borrowed typed input has no canonical serialized source byte length |
| `json_nesting_depth_max` | `NotApplicable` | Raw JSON representation limit; typed family replaces it for direct typed entry |
| `json_members_per_object_max` | `NotApplicable` | same |
| `json_array_items_max` | `NotApplicable` | same |
| `json_value_nodes_per_document_max` | `NotApplicable` | same |
| `string_bytes_max` | `NotApplicable` | keep existing source/serialized-document meaning; use new typed semantic-string ceiling for direct typed entry |
| `extension_bytes_max` | `NotApplicable` | first proof does not traverse or retain extension payloads for Consumer Planning; caller-owned ignored extensions do not become engine admission work |
| `affordances_per_thing_max` | `Active` | typed census before Planning |
| `forms_per_context_max` | `Active` | typed census/effective-form bound |
| `forms_per_thing_max` | `Active` | typed census and aggregate shape upper bound |
| `additional_responses_per_form_max` | `Active` structural only | bounded Basic shape; deferred response-planning semantics remain inactive |
| `uri_variables_per_form_max` | `Active` | typed census / URI-template bound |
| `schema_nodes_per_document_max` | `Active` | semantic schema nodes visited by Basic validation |
| `schema_composition_depth_max` | `Active` | semantic schema depth |
| `schema_reference_edges_per_document_max` | `Active` | semantic schema reference edges |
| `document_validation_work_units_max` | `Active` pending migration | retained as the policy ceiling but must map to the new vocabulary-neutral typed traversal work class/unit; it is not `JsonSchemaNodes` by fiat |
| `generated_effective_document_bytes_max` | `ZeroContribution` | first proof materializes plan facts, not a second effective TD |
| `remote_resolver_*` | `ZeroContribution` | no remote resolution |

Borrowed source memory is independent of structural work:

- `retained_source_bytes_per_owner_max` -> `ZeroContribution(0)`;
- `retained_source_bytes_global_max` -> `ZeroContribution(0)`.

The immutable borrow remains a lifetime constraint even when engine-owned retained-source bytes are zero.

## 2. Registration selection and Thing capacity

The selected registration rule is exactly-one eligible complete registration. Selection reads only validated startup metadata; it performs no binding/contributor callback.

| Row | Disposition | Decision |
| --- | --- | --- |
| `bindings_global_max` | `Active` structural ceiling | bounds the captured immutable startup snapshot scanned by narrow metadata selection; admission allocates no new binding |
| `binding_and_contributor_probes_per_admission_max` | `ZeroContribution(0)` | no binding/contributor probe callback is invoked |
| `wildcard_binding_and_contributor_probes_per_admission_max` | `ZeroContribution(0)` | wildcard probing is forbidden |
| `things_global_max` | `Active` reservation | one consumed Thing/runtime-record capacity unit is reserved before Frozen so later publication cannot fail solely because Thing capacity was never admitted |

Zero eligible registrations and multiple eligible registrations are structured admission failures. Registration order is not a tie-breaker.

## 3. Admission physical memory

These rows are `Active` during the aggregate transaction:

- `admission_temporary_bytes_per_operation_max`;
- `admission_temporary_bytes_global_max`;
- `peak_live_bytes_per_admission_max`;
- `admission_peak_live_bytes_global_max`;
- `engine_live_bytes_global_max`;
- `largest_contiguous_allocation_bytes_max`;
- `compiled_runtime_bytes_per_thing_max`;
- `compiled_runtime_bytes_global_max`.

They account physically live engine-owned heap/arena state or exclusively reserved caller-owned static capacity exactly once. Logical field sizes are not double-counted as allocations.

## 4. Plan-set and compiler rows

`Active` before or at Frozen:

- `compiled_plan_bytes_max`;
- `logical_plan_bytes_per_thing_max`;
- `form_binding_candidates_per_operation_max`;
- `plan_sets_per_thing_max`;
- `plan_sets_global_max`;
- `binding_artifacts_per_thing_max`;
- `binding_artifacts_global_max`;
- `binding_artifact_bytes_per_item_max`;
- `binding_artifact_bytes_per_thing_max`;
- `binding_artifact_bytes_global_max`;
- `binding_compiler_cursor_bytes_per_item_max`;
- `binding_compiler_cursor_bytes_global_max`;
- `plan_compile_work_units_per_step_max`;
- `plan_reclaim_bytes_per_step_max` for bounded abort/reclaim of provisional or Frozen material.

`Deferred` until Published/runtime:

- `plan_pins_per_plan_set_max`;
- `plan_pins_global_max`.

`NotApplicable` for the first eager proof:

- all `lazy_plan_slots_*`;
- all `lazy_artifact_*`;
- all `cache_*` rows whose authority is `PLAN-CACHE-001`/`PLAN-LAZY-001`.

For each exact mandatory coordinate, current `BindingCompilerBounds` contributes final artifact footprint, cursor bytes, temporary bytes and typed lifetime work. Final PlanIds exist before `bounds`; all resource reservation occurs after every `bounds` succeeds and before any compiler `start`.

The schema still needs two first-proof lifetime ceilings before implementation:

1. total Planning aggregate enumeration/index/reconciliation work per admission;
2. aggregate compiler work summed across all mandatory coordinates.

The selected direction is additive Foundation-neutral work accounting. Per-step limits never replenish lifetime allowance.

## 5. URI and security

| Row | Disposition |
| --- | --- |
| `uri_template_source_bytes_max` | `Active` |
| `uri_template_variables_max` | `Active` |
| `expanded_uri_bytes_max` | `Deferred` to call-time unless immutable Planning material explicitly stores a bounded expansion result |
| `security_expression_depth_max` | `Active` structural bound |
| `security_branches_per_plan_max` | `Active` structural bound; semantic first-proof predicate still admits only deterministic NoSec/no branch choice |
| `provider_probes_per_interaction_max` | `ZeroContribution(0)` during admission |

No security row activates credentials/provider work in this proof.

## 6. Hierarchical accounting rows

`PERF-ACCOUNT-001` applies because aggregate reservation and reconcile use hierarchical resource accounts.

| Row | Profile/disposition | Decision |
| --- | --- | --- |
| `accounting_batch_items_max` | `Active` | any batched atomic debit/release operation is bounded by this row |
| `accounting_idle_items_max` | `ZeroContribution(0)` for admission | first-proof admission does not defer resource ownership into an idle accounting queue |
| `accounting_reconcile_owners_per_step_max` | `Active` | bounds incremental owner reconciliation/rollback progress |
| `accounting_reconcile_interval_millis_max` | `NotApplicable` to Consumer admission | registered only for gateway/directory-client periodic reconciliation, not this admission transition |
| `accounting_reconcile_steps_max` | `Active` only where the selected profile supplies a value; otherwise schema-`NA` | bounds constrained/manual reconciliation completion; `NA` is not converted into an invented host limit |

The aggregate material fixture must retain a committed Frozen ledger/account state rather than decrementing every persistent reservation to zero at freeze.

## 7. Diagnostics and status

The current first-proof admission diagnostic is a bounded structured `CoreError`/fixed-width cause projection; it does not create durable binding status history.

Therefore:

- `durable_status_*` -> `Deferred` to runtime/status ownership, not Stage-A admission;
- `binding_runtime_event_*` -> `Deferred` to runtime/status ownership;
- no variable-sized diagnostic buffer is inferred from cleanup/status limits.

If migration introduces variable-sized retained admission diagnostics, it must register an explicit bounded resource row rather than silently charging another family.

## 8. Cleanup, retry and transfer

Active during Stage-A failure/abort:

- `cleanup_items_max`;
- `cleanup_bytes_max`;
- `cleanup_item_bytes_max`;
- `cleanup_work_items_per_step_max`;
- `plan_reclaim_bytes_per_step_max`.

The current binding compiler cursor is pure in-memory planning state and `BindingCompilerExtension::abort(cursor)` has no protocol cleanup obligation. Consequently:

- `cleanup_retry_records_max` -> `ZeroContribution(0)` for pre-Frozen aggregate admission;
- `cleanup_retry_attempts_max` -> `ZeroContribution(0)`;
- `cleanup_transfer_slots_global_max` -> `Deferred` to post-publication binding/call cleanup transfer;
- `cleanup_transfer_bytes_global_max` -> `Deferred`;
- binding cancel buffers / host call cleanup buffers -> `Deferred` to request execution.

Partial aggregate success is still real provisional material: if coordinate N fails after earlier coordinates completed, those earlier logical plans/artifact envelopes/refs/index entries remain owned by `Aborting` until bounded release completes.

## 9. Runtime binding/call/subscription/Producer domains

The following are `Deferred` when they are Consumer runtime rows and `NotApplicable` when they are capability-exclusive outside this proof:

- pending Consumer call counts, Host call bytes, binding request-slot state, binding poll temporary bytes, cancel buffers and wake/reactor resources -> `Deferred` until Published request execution;
- subscriptions/collection subscription queues/drivers -> `NotApplicable`;
- Producer handlers, in-flight responses, routes, route guards/readiness/ingress, endpoint reservations, emission/fanout -> `NotApplicable`;
- Directory/discovery/query/watch/session/publication rows -> `NotApplicable`.

## 10. Mechanical projection rule

Migration must mechanically project `docs/resource-limits.csv` together with this map; it must not hand-copy a partial checked policy.

The Stage-A coverage fixture classifies every current row with `consumer` or `all` applicability into `Active`, `ZeroContribution`, `Deferred`, or `NotApplicable` and fails on any unclassified row. Rows whose capability roles exclude Consumer are mechanically `NotApplicable` for this proof.

For `Active`, `ZeroContribution`, and `Deferred`, profile `NA` is legal only when the registered schema marks that selected profile `NA`. No current active first-proof limit becomes unbounded merely because a field was omitted from a hand-maintained struct.
