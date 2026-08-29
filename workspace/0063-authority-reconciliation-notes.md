# 0063 Authority Reconciliation Notes

Status: DISCUSSING support material for `0063-consumer-aggregate-admission-plan-set-authority.md`

This note records exact current-authority conflicts and affected frozen surfaces. It is not an authoritative migration, work-package status change, or final specification.

## Plan-set ownership conflict map

| Active source | Current claim | Reconciliation direction under 0063 candidate |
| --- | --- | --- |
| `docs/architecture/20-module-boundaries.md` responsibility map | Planning produces admitted-plan build output; Servient owns plan-set ownership, admission, cleanup | Keep |
| `docs/architecture/20-module-boundaries.md` Planning boundary | Planning receives validated document/policy/resource/registration inputs and returns values/admitted footprints; it does not retain a Servient handle | Keep; clarify that resource inputs are non-authoritative admitted views, not the Servient reservation owner |
| `docs/architecture/20-module-boundaries.md` Servient boundary | Servient owns the transaction that composes modules, registration snapshot, publication/retirement, cleanup reservation/status | Keep |
| `docs/architecture/30-compiled-plan-lifecycle.md` | lifecycle state belongs to Servient record; Building owns document/policy/registration snapshot, admission transaction, compiler cursors, provisional artifacts | Keep; Building is a Servient-record state even when Planning owns a nested algorithm session/cursor |
| `docs/spec/planning.md` opening sentence | Planning spec calls itself normative owner of compiled-plan-set publication and plan reclamation | Conflict; replace with material-construction / compiler-coordination ownership only |
| `PLAN-SET-001` in `docs/spec/planning.md` | one Servient-owned aggregate compiled-plan-set record alone owns lifecycle, publication, pins, leases, cursors, accounting | Keep |
| `docs/spec/planning.md` scope invariants | aggregate lifecycle state, pins, compiler cursors, lazy slots, reclamation cursors belong to Servient-owned plan-set record | Keep, but distinguish Servient-owned enclosing cursor/storage from Planning-owned algorithm state moved inside it |
| `docs/spec/planning.md` ownership table | Planning owns construction/coordination/admitted build output; Servient owns build transaction, reservation, record, publication, draining, reclamation | Keep |
| `docs/work-packages/WP-200-planning.md` | WP-200 output is complete immutable material for one unpublished Frozen plan-set draft; WP-400 owns the Servient record and every Building/Frozen/Published/Draining/Failed/Reclaimed transition | Keep and use as the work-package migration anchor |
| `workspace/0062-consumer-plan-set-handoff-closure.md` | Planning is the missing aggregate predecessor; Servient owns reservation, lifecycle, publication, leases, drain, reclamation | Keep as investigation evidence; later reconcile/supersede its split-claim framing |
| `docs/state-machines.toml` compiled-plan-set machine | owner record is `Servient CompiledPlanSetRecord` | Keep |
| `docs/state-machines.toml` Building -> Frozen | transition owner is `PlanningBuildOwner` | Clarify: nested/private build subowner inside the Servient-owned record, not crate-level transaction/publication authority. Rename if needed to make this unambiguous. |

### Selected ownership interpretation for review

The candidate has one non-overlapping rule:

- Servient owns the aggregate transaction and every reservation/lifecycle/publication/cleanup authority;
- TD owns validation semantics/provenance;
- Planning owns deterministic interpretation, preflight measurement, compiler coordination, build progress, and sealed immutable draft material;
- a Planning algorithm cursor may physically live inside the Servient-owned Building record without transferring lifecycle authority to the Planning crate;
- binding compilers own only their pure local compiler cursor/artifact transformations under declared bounds.

## Frozen public Consumer Planning surface requiring disposition

`docs/api-ownership.csv` currently freezes these Planning values as public Producer/Consumer API:

| Surface | Current issue for admitted Consumer aggregate path | Candidate disposition question |
| --- | --- | --- |
| `PlanCompiler` | generic start/step surface can be driven outside Servient admission | retain only as algorithm SPI, split Consumer admitted entry, or narrow visibility/semantics |
| `PropertyReadPlanCompiler` | public constructor/session represents one coordinate, not one aggregate Consumer plan set | Producer path may remain; Consumer admitted role needs aggregate replacement |
| `PropertyReadPlanCompiler::consumer_call` | accepts raw `PlanId`, registration identity, registration ordinal and one target coordinate | cannot remain a second admitted Consumer authority; remove/split/trust-label for Consumer during WP-200 migration |
| `PropertyReadBuildCursor` | public resumable cursor can be paired with generic build inputs | admitted Consumer resume must remain inside one captured aggregate session/transaction |
| `PlanBuildInput` | public input carries raw TD/registration/generation values and is repeatably supplied | cannot itself certify Consumer validation/admission provenance; retain only for non-admitted/shared algorithm use or replace Consumer path |
| `PlanBuildOutput` | public output is a data value that current narrow Consumer selection can consume | may remain data/test value only if Servient publication cannot accept it without a sealed draft/lease match; otherwise narrow/split constructor authority |
| `PlanBuildIdentity` / `PlanBuildCursor` / `PlanBuildStep` / `PlanBuildFailure` | generic public Planning machinery shares Producer/Consumer roles | explicit role impact needed; do not break Producer merely to close Consumer authority |
| `PlanFootprint` | public measurement value | likely retain as non-authoritative measurement, subject to aggregate footprint expansion |
| `BindingCandidate`, `BindingArtifact*`, `PlanId`, `PlanSetGeneration` | public Core immutable values | retain as data identities; possession must not imply publication/execution authority |

The migration must distinguish **public data/algorithm values** from **authority-bearing admitted Consumer entry**. A public constructor is not automatically forbidden; it is forbidden only if a safe caller can use it to bypass validation provenance, Servient reservation, same-registration ownership, aggregate reconciliation, or publication gating.

## Work-package impact map for independent review

Machine-readable status remains unchanged while this topic is DISCUSSING.

| Tranche/package | Current registered state | Candidate impact | Evidence question before status change |
| --- | --- | --- | --- |
| WP-000 | package complete | affected if Foundation public work/reservation primitives change | can the new generic work/lifetime/reservation primitives be an additive successor tranche, or do existing completed resource contracts become false? |
| `WP-100-CONSUMER-CALL-VALUES-VALIDATOR` | complete/admitted/current | likely reaffirm | call values and response validation do not depend on aggregate build authority unless Core identity/request semantics change |
| TD validation scope under WP-100 ownership | no matching completed Consumer admission tranche | new narrow predecessor likely required | exact Basic/census/provenance authority and resource/work contract must be registered without activating broad deferred validation domains |
| `WP-200-CONSUMER-PROPERTY-READ-PLANNING` | complete/admitted/current | definitely affected; reopen leading hypothesis | frozen public single-coordinate Consumer API/output/evidence conflicts with aggregate admitted path; determine what Producer/shared evidence survives |
| Producer WP-200 slices | completed where registered | presumed disjoint/reaffirm | prove Consumer API split does not change Producer algorithms/contracts |
| `WP-300-CONSUMER-PROPERTY-READ-BINDING` | complete/admitted/current | definitely affected; scoped reopen leading hypothesis if Core public/source changes | same complete registration must supply compiler identity and persistent execution owner; no-bypass/pin evidence must be added without discarding valid call/response mechanics |
| Producer WP-300 slices | completed where registered | presumed disjoint/reaffirm | prove complete-registration changes preserve Producer bundle/execution semantics |
| WP-400 Consumer | not admitted | blocked | successor admission cannot start until upstream migrated authority and impact review pass |
| Producer Property Read architecture gate | passed | not Consumer evidence | retain accepted Producer claim unless migration actually touches its contract/evidence |

ADR-0013 requires every affected completed tranche to be reaffirmed or reopened with transitive dependents as applicable. This table is the hypothesis to review, not the status transition itself.

## Resource/work authority gaps to resolve

Current Foundation `WorkClass` is vocabulary-neutral and currently exposes `JsonSchemaNodes`, codec bytes, `UriBytes`, `SecurityBranches`, `ProviderProbes`, `QueueOperations`, `BindingPolls`, `CleanupItems`, and `HandlerSteps`.

The aggregate Consumer candidate has work that does not truthfully fit an existing class:

- generic typed-document structural traversal/census;
- Planning coordinate enumeration;
- candidate/index materialization;
- aggregate reconciliation.

Therefore migration must choose additive vocabulary-neutral classes or another generic bounded-work representation. It must not relabel this work as `JsonSchemaNodes`, `BindingPolls`, or `CleanupItems` merely to reuse an existing counter.

Separate lifetime ceilings are also required for:

- total Planning progress in one Consumer admission; and
- aggregate binding-compiler work across every mandatory coordinate.

A per-step compiler cap remains useful but cannot substitute for either lifetime ceiling.

## 0062 reconciliation

0062 remains correct on its established defect and ownership direction but its claim decomposition is no longer assumed correct.

If 0063 is accepted, 0062 must be re-evaluated against the aggregate transaction:

- bounded validation provenance becomes the first phase of the same Servient admission owner;
- aggregate Planning preflight/build becomes the middle algorithmic phase;
- build-time complete-registration identity and the persistent execution owner must join before Frozen can be considered constructible;
- the remaining local handoff, if any, is `sealed aggregate draft + Servient-retained reservation/identity/execution ownership`, not a single-plan container.

If that leaves no independent local question, 0062 should be marked superseded during migration rather than artificially preserved as another design layer.
