# 0063 Authority Reconciliation Notes

Status: DISCUSSING support material for `0063-consumer-aggregate-admission-plan-set-authority.md`

This note records the selected pre-decision reconciliation for normative ownership, runtime ownership, public no-bypass semantics, registration selection, and ADR-0013 impact. It is not an authoritative migration or work-package status change.

## Normative ownership and runtime ownership are distinct

The previous revision incorrectly described the opening sentence of `docs/spec/planning.md` as a crate/runtime ownership conflict. That was a category error.

`docs/spec/planning.md` is and remains the registered normative owner of `PLAN-SET-001`. Normative ownership means that this specification defines the required plan-set semantics. It does **not** mean that `clinkz-wot-planning` owns the runtime plan-set record or lifecycle transaction.

The selected reconciliation is therefore:

| Active source | Current claim | Selected disposition under 0063 |
| --- | --- | --- |
| `docs/spec/planning.md` opening owner sentence | Planning specification normatively owns effective-form planning, compiler coordination, compiled-plan-set publication and reclamation semantics | **Keep normative ownership.** During migration clarify that this is specification-domain ownership, not `clinkz-wot-planning` runtime ownership. `PLAN-SET-001` stays registered here unless a separate deliberate authority migration says otherwise. |
| `PLAN-SET-001` in `docs/spec/planning.md` | one Servient-owned aggregate compiled-plan-set record alone owns lifecycle, publication, pins, operation leases, cursors and accounting | **Keep unchanged in substance.** This is the runtime ownership rule implemented by Servient. |
| `docs/spec/planning.md` ownership table | Planning owns construction/coordination/admitted output; Servient owns build transaction, reservation, record, publication, draining and reclamation | **Keep.** This is the exact normative/runtime split used by the candidate. |
| ADR-0008 | explicit Servient-owned compiled-plan-set lifecycle; Planning owns shared algorithms and compiler coordination | **Keep.** 0063 refines the Consumer admission constructibility of this accepted decision; it does not reverse ADR-0008. |
| `docs/architecture/20-module-boundaries.md` | Planning produces admitted build material; Servient owns plan-set admission/lifecycle/cleanup | **Keep.** |
| `docs/architecture/30-compiled-plan-lifecycle.md` | lifecycle belongs to the Servient record; Building contains admission/build state | **Keep.** A nested Planning algorithm session may be stored inside Building without owning the lifecycle record. |
| `docs/state-machines.toml` compiled-plan-set owner record | `Servient CompiledPlanSetRecord` | **Keep.** |
| `docs/state-machines.toml` `Building -> Frozen` transition owner | `PlanningBuildOwner` | **Clarify during migration.** The transition linearizes inside the Servient-owned record. Rename the owner to an explicitly Servient-scoped build owner if required so it cannot be read as `clinkz-wot-planning` owning the lifecycle transition. |
| `docs/work-packages/WP-200-planning.md` | outputs complete immutable material for an unpublished draft; WP-400 owns Servient lifecycle states | **Keep as the Planning package boundary, then update its Consumer tranche to aggregate construction if impact review reopens it.** |
| `docs/work-packages/WP-400-servient.md` | Servient/StaticServient own startup registration snapshots, compiled-plan-set records, lifecycle transactions, cleanup and Consumer `consume` publication | **Keep as runtime implementation owner.** A future Consumer WP-400 tranche must consume the sealed aggregate draft plus the Servient-retained identity/resource/execution authority and must not reconstruct Planning input. |
| `workspace/0062-consumer-plan-set-handoff-closure.md` | Planning is the missing predecessor while Servient owns reservation/lifecycle/publication | **Keep as investigation evidence; later reconcile or supersede its old claim split.** |

There is therefore no requirement-owner vacuum in the candidate: `docs/spec/planning.md` continues to own `PLAN-SET-001` normatively, while Servient owns its runtime record and transitions as already required by that same specification and ADR-0008.

## Selected runtime ownership interpretation

The candidate has one non-overlapping runtime rule:

- Servient owns the aggregate transaction and every identity reservation, resource reservation, lifecycle, publication, pin, execution-owner retention, abort-settlement and reclamation authority;
- TD owns validation semantics and exact validated provenance;
- Planning owns deterministic aggregate interpretation, shape enumeration, compiler-bounds coordination, build progress, immutable indexes and sealed draft material;
- a Planning cursor/session may physically live inside the Servient-owned Building record without transferring lifecycle authority to Planning;
- binding compilers own only pure local cursor/artifact transformations under declared bounds.

## First-proof registration selection is closed

The first Consumer Property Read proof selects **exactly one eligible complete registration** from the immutable startup snapshot before any coordinate is compiled.

Eligibility is metadata-only and profile-specific:

1. the complete registration has already passed its Core registration validation;
2. it advertises Consumer Property Read capability; and
3. it contains the execution half for the active profile (Host-erased for Host, application-static for the constrained/static profile).

Selection performs no binding callback, no form-specific support probe, no wildcard probe and no protocol I/O.

The outcome is exact:

- zero eligible complete registrations -> structured no-eligible-registration admission failure;
- exactly one -> that exact snapshot entry is captured for the whole aggregate transaction;
- more than one -> structured ambiguous-registration admission failure.

Registration order never resolves ambiguity. Once selected, the same entry supplies compiler/candidate identity and the persistent execution owner for every mandatory coordinate. A later bounds/build failure cannot trigger registration reselection.

The snapshot ordinal is the positional coordinate used to retain the exact entry. `BindingRegistrationIdentity::diagnostic_ordinal()` remains diagnostic metadata and is never used as a snapshot index; the two values may differ.

## Selected public no-bypass dispositions

The no-bypass claim is scoped precisely: **only Servient may confer ClinkZ admitted Consumer plan-set publication/handle authority.** Public lower-layer algorithm/data/SPI values may remain directly usable, but manual composition of those values is not an admitted Servient handle and receives none of the Servient admission/publication guarantees.

The migration dispositions are selected as follows:

| Surface | Selected disposition |
| --- | --- |
| `Servient::consume` / `StaticServient` Consumer entry | Canonical admitted entry. Accepts ordinary TD/policy/profile inputs and drives the private aggregate transaction. It never accepts raw `PlanBuildOutput`, artifact envelopes/references, raw `PlanId`/`PlanSetGeneration`, or an externally assembled execution pin. |
| Servient freeze/publish/install operations | Private lifecycle operations over the live Servient transaction/record only. No safe public method accepts externally built Planning/Core material as already admitted. |
| `PlanCompiler` | Retain as a public lower-level Planning algorithm SPI where Producer/shared use requires it. Calling it does not create admission/publication authority. |
| `PlanBuildInput`, `PlanBuildOutput`, `PlanBuildIdentity`, `PlanBuildCursor`, `PlanBuildStep`, `PlanBuildFailure`, `PlanFootprint` | Retain as public lower-level algorithm/data values where compatibility/shared Producer use requires them, but document them as non-authoritative. No Servient Consumer publication path accepts them as proof of validation or reservation. |
| `PropertyReadPlanCompiler` / `PropertyReadBuildCursor` | Retain shared/legacy algorithm surface only as needed for compatibility and Producer behavior. They are not the admitted aggregate Consumer session. |
| `PropertyReadPlanCompiler::consumer_call` | Legacy one-coordinate Consumer algorithm entry is excluded from the target engine path. Reopened WP-200 must either deprecate/remove this public Consumer convenience or retain it explicitly as non-admitted lower-level API; either way it cannot feed Servient publication. This compatibility choice no longer affects authority closure because Servient accepts neither branch. |
| `select_consumer_property_read` | May remain a lower-level selector for legacy/test material; the admitted consumed handle selects only inside its Servient-owned Frozen/Published record. |
| public Core `PlanId`, `PlanSetGeneration`, `BindingArtifact*`, `OutboundRequest` and binding execution traits | Retain as Core data/SPI. Possession or manual composition does not represent Servient admission. Direct low-level binding use is outside the admitted Servient guarantee boundary. |

The authority invariant is therefore structural rather than based on making every lower-level Rust constructor private: no externally assembled value can be converted into a Published consumed generation because the required live Servient record, committed ledger, generation reservation and execution-owner pin are private lifecycle state.

## Work-package and authority impact map

Machine-readable package/tranche status remains unchanged while this topic is DISCUSSING.

| Owner/tranche | Current state | Candidate impact | Required disposition before implementation |
| --- | --- | --- | --- |
| ADR-0008 | Accepted | reaffirm | 0063 refines constructibility but preserves its Servient-owned lifecycle decision. |
| Planning specification / `PLAN-SET-001` | active normative authority | amend wording/evidence only | keep requirement ownership in `docs/spec/planning.md`; clarify normative vs runtime ownership and aggregate Consumer algorithm contract. |
| Servient lifecycle architecture | active | amend Consumer detail | preserve Servient runtime owner; add aggregate Consumer admission/identity/resource/execution-pin details. |
| WP-000 | package complete | affected if Foundation work/resource primitives change | exact successor-vs-reopen decision after migration diff is known. |
| `WP-100-CONSUMER-CALL-VALUES-VALIDATOR` | complete/admitted/current | reaffirm leading result | call values/response sealing are independent unless migration changes their Core identity contract. |
| TD bounded Consumer validation/provenance | no matching narrow completed tranche | new narrow predecessor required | register exact typed structural census, work and provenance contract without activating broad deferred validation/cache/codec scope. |
| `WP-200-CONSUMER-PROPERTY-READ-PLANNING` | complete/admitted/current | affected; reopen leading result | replace single-coordinate admitted Consumer proof with aggregate enumerate/bound/build/sealed-draft proof and record surviving Producer/shared evidence. |
| Producer WP-200 slices | completed where registered | explicit reaffirm/disjoint evidence | Consumer migration must not silently alter Producer semantics. |
| `WP-300-CONSUMER-PROPERTY-READ-BINDING` | complete/admitted/current | affected | reaffirm existing call/response mechanics; reopen only the Core registration/execution-pin/no-bypass parts if production source/public contracts must change. |
| Producer WP-300 slices | completed where registered | explicit reaffirm/disjoint evidence | complete-registration migration must preserve Producer behavior. |
| WP-400 Consumer slice | not admitted | must be rewritten before admission, not reopened | its eventual tranche consumes the migrated aggregate Planning result and private Servient authority; it remains blocked now. |
| broad WP-400 package | not complete | no broad activation | 0063 does not activate broad scheduler/subscription/emission work. |
| Producer Property Read architecture gate | passed | retain unless touched | it is not Consumer evidence, but remains valid if migration diff proves its contracts unchanged. |

ADR-0013 still requires exact reaffirm/reopen and transitive-dependent handling when the migration diff is admitted. This table selects the intended disposition boundary but does not perform those status changes.

## Work taxonomy closure direction

Current Foundation `WorkClass` has no truthful class for generic typed-document traversal or Planning aggregate enumeration/index/reconciliation. The selected migration direction is additive vocabulary-neutral work accounting, not relabeling those operations as `JsonSchemaNodes`, `BindingPolls`, or `CleanupItems`.

The first proof requires lifetime ceilings for:

- typed semantic document traversal/validation work;
- total Planning aggregate enumeration/index/reconciliation work; and
- aggregate compiler work across every mandatory coordinate.

Per-step budgets remain additional progress caps and never replenish those lifetime ceilings.

## 0062 reconciliation

0062 remains correct about the missing aggregate handoff and Servient lifecycle ownership, but its previous decomposition is not preserved as architecture.

If 0063 is accepted and migrated, 0062 must be reduced to the surviving handoff contract — sealed aggregate draft plus Servient-retained identity/resource/execution ownership — or marked superseded if that leaves no independent unresolved question.
