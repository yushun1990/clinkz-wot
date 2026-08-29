# 0063 Consumer Aggregate Resource and Work Applicability

Status: DISCUSSING support material for `0063-consumer-aggregate-admission-plan-set-authority.md`

This note dispositions the active resource/work families that can intersect the first borrowed-typed Consumer Property Read aggregate admission. It does not change `docs/resource-limits.csv`, Foundation API, or any active requirement.

The first proof has these deliberate boundaries:

- input representation: caller-owned borrowed typed `Thing`, not Raw JSON;
- capability: Consumer one-shot Property Read only;
- security: deterministic no-material NoSec only;
- planning: eager aggregate Property Read material, no lazy/cache/fallback;
- protocol execution: none before Frozen;
- publication/pins/calls: later Servient/WP-400 stages, not Stage-A completion evidence.

## A. Source and typed-document validation

| Existing field/family | First-proof disposition | Required migration interpretation |
| --- | --- | --- |
| `document_bytes_max` | not applicable to borrowed typed input | Keep Raw/serialized-document semantics; do not pretend a typed Rust tree has a source byte length. |
| `json_nesting_depth_max` | not directly applicable | Raw JSON structural bound; direct typed `Thing` construction needs a representation-neutral typed structural depth bound if bounded Basic validation can recurse. |
| `json_members_per_object_max` | not directly applicable | Raw JSON object bound; typed maps need an explicitly named typed/map structural bound where traversal is admitted. |
| `json_array_items_max` | not directly applicable | Raw JSON array bound; typed sequences need an explicit typed/sequence bound where traversal is admitted. |
| `json_value_nodes_per_document_max` | not directly applicable | Raw JSON node count; typed validation/census needs its own representation-accurate node/item ceiling. |
| `string_bytes_max` | applicable semantic ceiling candidate | Count the UTF-8 bytes physically reachable in the typed first-proof projection under an explicitly defined census; do not silently reuse Raw JSON lexical bytes. If current authority means serialized-document bytes, split the field during migration instead. |
| `extension_bytes_max` | applicable only to extensions actually traversed/retained by the first proof | Define whether typed extension `serde_json::Value` payloads are structurally counted even when semantically ignored. |
| `affordances_per_thing_max` | applicable | Typed census checks before Planning enumeration. |
| `forms_per_context_max` | applicable | Typed census / effective-form validation. |
| `forms_per_thing_max` | applicable | Typed census / Planning aggregate upper bound. |
| `additional_responses_per_form_max` | structurally applicable, semantically deferred | Basic validation must keep bounded shape, but first Property Read Planning does not activate broad additional-response planning semantics. |
| `uri_variables_per_form_max` | applicable | Typed census and URI-template compilation bound. |
| `schema_nodes_per_document_max` | applicable to typed schema structures reached by Basic validation | Count semantic schema nodes in typed structures, not Raw JSON nodes. |
| `schema_composition_depth_max` | applicable | Basic validation bound. |
| `schema_reference_edges_per_document_max` | applicable | Basic validation bound. |
| `document_validation_work_units_max` | applicable lifetime ceiling | Must be charged through an accurately named Foundation-neutral work class/unit mapping; the existing ceiling alone does not define the unit. |
| `generated_effective_document_bytes_max` | not applicable to this Consumer first proof unless Planning materializes a second effective TD | Candidate avoids retaining/generating a second TD; effective facts belong in plan material. |
| `remote_resolver_*` | zero/not applicable | First proof performs no remote resolution during admission. |

### Required schema consequence

Direct typed `Thing` input makes the current `json_*` structural ceilings insufficient as proof of bounded typed traversal. Migration must either:

1. add representation-neutral typed structural fields with exact census units; or
2. redefine/split the existing document structural fields through an independently reviewed resource-schema migration.

It must not silently claim that `json_*` rows already bound an arbitrary directly constructed typed Rust tree.

Borrowed source accounting is distinct from structural validation:

- `retained_source_bytes_per_owner_max`: local engine-owned charge is exactly `0` for the borrowed representation;
- `retained_source_bytes_global_max`: this admission contributes exactly `0` retained source bytes;
- the source borrow still has a lifetime/ownership contract even though it consumes no engine-owned retained-source capacity.

## B. Admission physical-memory accounts

These active `ADMIT-MEM-001` / peak fields all apply to the aggregate transaction even when the retained-source contribution is zero:

| Field | First-proof charge/meaning |
| --- | --- |
| `admission_temporary_bytes_per_operation_max` | phase-local TD/Planning/compiler scratch owned or exclusively reserved by one admission |
| `admission_temporary_bytes_global_max` | concurrent aggregate temporary capacity |
| `peak_live_bytes_per_admission_max` | maximum simultaneous physically live admission bytes across validation, enumeration, identity metadata, compiler cursor/temp, provisional material, diagnostics and cleanup ownership |
| `admission_peak_live_bytes_global_max` | concurrent admission live peak |
| `engine_live_bytes_global_max` | all engine-owned live bytes, including admission/frozen material as applicable |
| `largest_contiguous_allocation_bytes_max` | largest real Host allocation or exclusively reserved static region used by this admission; measured once per physical allocation |
| `compiled_runtime_bytes_per_thing_max` | Frozen unpublished compiled runtime material for the consumed Thing generation |
| `compiled_runtime_bytes_global_max` | global compiled runtime material reservation |

Physical accounting follows `ADMIT-MEM-001`: actual engine heap/arena or exclusively reserved caller-provided capacity is charged; logical field sizes are not invented as independent allocations. Padding/structural overhead has one physical owner and no byte is double counted.

The Host/static storage fixture proves only safe ownership topology. Exact production layout measurements remain completion evidence after migration selects real storage types.

## C. Active Planning/plan-set controls

| Existing field | Phase | First-proof disposition |
| --- | --- | --- |
| `compiled_plan_bytes_max` | Bounding/Reserving | applicable aggregate ceiling over admitted compiled plan material |
| `logical_plan_bytes_per_thing_max` | Bounding/Reserving | applicable |
| `form_binding_candidates_per_operation_max` | Enumeration/Bounding | applicable even though first proof retains one selected registration/candidate per coordinate |
| `binding_and_contributor_probes_per_admission_max` | first-proof applicability depends on accepted registration-selection algorithm | current `PLAN-INDEX-001` is inactive; do not activate broad probe/index semantics accidentally. If the narrow selected-registration rule performs a bounded probe, it needs an explicit v5.1 authority disposition. |
| `wildcard_binding_and_contributor_probes_per_admission_max` | not applicable unless narrow authority explicitly admits wildcard probing | no implicit wildcard path |
| `plan_sets_per_thing_max` | AssigningIdentities/Reserving | applicable to Servient plan-set slot/generation reservation |
| `plan_sets_global_max` | AssigningIdentities/Reserving | applicable global plan-set capacity |
| `plan_pins_per_plan_set_max` | later Published operation | policy remains captured but no pin is allocated/charged in Stage-A admission |
| `plan_pins_global_max` | later Published operation | same |
| `binding_artifacts_per_thing_max` | Bounding/Reserving | applicable aggregate artifact count |
| `binding_artifacts_global_max` | Reserving | applicable global artifact capacity |
| `binding_artifact_bytes_per_item_max` | compiler Bounds/Reconcile | applicable to every mandatory coordinate |
| `binding_artifact_bytes_per_thing_max` | Bounding/Reserving/Reconcile | applicable aggregate bytes |
| `binding_artifact_bytes_global_max` | Reserving/Reconcile | applicable global bytes |
| `binding_compiler_cursor_bytes_per_item_max` | compiler Bounds/Building | applicable before `start` |
| `binding_compiler_cursor_bytes_global_max` | Reserving/Building | applicable across concurrent compiler cursors |
| `plan_compile_work_units_per_step_max` | Building | applicable per external Planning step/callback window; not a lifetime ceiling |
| `plan_reclaim_bytes_per_step_max` | Aborting / later reclamation | applicable when releasing provisional/frozen plan bytes incrementally; exact phase mapping must align with the accepted reclaim owner |
| `lazy_plan_slots_*`, `lazy_artifact_*`, cache rows | not applicable | `PLAN-LAZY-001` / `PLAN-CACHE-001` remain inactive for first proof; all Consumer Property Read artifacts are eager |

### Missing active lifetime controls

The current schema has a compiler per-step work cap but no explicit first-proof ceiling for:

- total Planning coordinate/candidate/index/reconciliation work in one admission; and
- aggregate binding-compiler work summed across every mandatory coordinate in one admission.

Those ceilings are required before implementation admission. Exact field names and whether they are new `ResourceKind`s or another checked policy projection are migration decisions; they must be Foundation-neutral.

## D. Compiler declared bounds

Every exact mandatory coordinate is assigned its final unpublished `PlanId` before current compiler `bounds` because current `BindingCompilerInput` exposes the logical plan and its PlanId.

For each exact input, the enclosing Servient transaction records all of `BindingCompilerBounds`:

- final artifact items/bytes;
- cursor bytes;
- temporary bytes;
- typed work allowance.

All coordinate declarations are aggregated without calling `start`. Only after the complete aggregate requirements are known may Servient acquire the resource bundle. Compiler `start`/`step` cannot run on a failed reservation.

Compiler lifetime work is a declared upper bound, not a replenishable per-step allowance. Each callback sees only a child budget atomically partitioned from both:

1. remaining aggregate/compiler lifetime allowance; and
2. the caller's current step allowance.

Unused child capacity is reconciled back to both parents; failure to acquire either allowance performs no compiler callback.

## E. URI and security controls

| Existing field | First-proof disposition |
| --- | --- |
| `uri_template_source_bytes_max` | applicable while compiling exact resolved/effective Form target rules |
| `uri_template_variables_max` | applicable |
| `expanded_uri_bytes_max` | call-time execution concern; no expanded request URI is produced during aggregate admission unless current Planning contract explicitly materializes one |
| `security_expression_depth_max` | applicable structural validation ceiling |
| `security_branches_per_plan_max` | applicable structural ceiling, but accepted first-proof semantic predicate requires deterministic NoSec/no branch choice |
| `provider_probes_per_interaction_max` | zero during first-proof admission; no provider/credential access is permitted |

No security resource row authorizes activation of deferred credential/application-security migration.

## F. Cleanup and failure settlement

The first proof can fail after identity assignment, resource reservation, compiler start, or provisional artifact creation. Therefore cleanup ownership is active even though no protocol I/O has begun.

Applicable rows:

- `cleanup_items_max`;
- `cleanup_bytes_max`;
- `cleanup_item_bytes_max`;
- `cleanup_work_items_per_step_max`;
- plan reclaim bytes where provisional/frozen plan material is the released resource.

Not applicable before protocol execution:

- pending client-call limits;
- host binding call bytes;
- binding slot/request runtime state;
- binding poll temporary bytes for request execution;
- binding cancel buffers;
- subscription queues/drivers;
- route/ingress/publication/emission resources.

A compiler cursor itself is pure in-memory Planning state under the current compiler SPI and its `abort(cursor)` has no protocol cleanup obligation. The aggregate transaction still charges bounded outer cleanup iteration/release work and invokes the live compiler abort exactly once before dropping that cursor.

## G. Domains explicitly outside the first proof

Rows restricted to these capabilities are inapplicable and must not be pulled into checked Consumer admission policy merely because they exist in the 195-field schema:

- Directory/discovery sessions, publications, queries, watches and response buffers;
- Producer handlers, in-flight responses, route reservations/guards/readiness/ingress;
- Producer emission/fanout;
- Consumer subscriptions/collection subscription queues/drivers;
- protocol request-call runtime resources that begin only after Published selection.

Their absence from the first-proof checked admission projection is an explicit applicability result, not an unreviewed hole.

## Checked-policy projection rule

The future checked Consumer admission policy should be generated or mechanically projected from the registered resource schema plus this accepted applicability map. It must not duplicate a hand-maintained forty-field schema whose omissions can drift from `docs/resource-limits.csv`.

For each resource row, projection records one of:

- `Active(value)` for an enforced first-proof limit;
- `ZeroContribution(value)` when the authority is applicable but this representation contributes exactly zero (for example borrowed retained source);
- `Deferred(authority)` when the limit belongs to a later lifecycle phase but must remain associated with the same profile; or
- `NotApplicable(reason)` for a capability/representation excluded by the first proof.

`None`/`NA` in the selected profile is legal only when the registered schema says that field is not applicable to that profile. An active first-proof limit must not become unbounded by omission.
