# 0063 Consumer Aggregate Admission and Plan-Set Build Authority

Status: DISCUSSING

Kind: architecture reconciliation and replacement proposal

Priority: HIGH

Target: replace the rejected single-plan Consumer admission hypothesis with one constructible Servient-owned aggregate Consumer plan-set admission transaction, then determine the exact authority migration and ADR-0013 impact before any production Rust resumes.

## Why this topic exists

`workspace/0062-consumer-plan-set-handoff-closure.md` established that the completed Consumer WP-200 and WP-300 tranches do not compose into the required public Servient path. Its original decomposition sent the bounded validated-input question into a distinct claim before returning to the aggregate Planning -> Servient handoff.

An unmerged first attempt at that claim was developed in PR #56 under the `workspace/0063` number. Seven independent review cycles made the proposal progressively more constructible, but a later independent from-first-principles design review found that its central authority granularity was wrong: it centered admission on one PlanId-bearing Planning lease, while active architecture already assigns the complete compiled-plan-set transaction, reservation, lifecycle, publication, and cleanup authority to Servient.

Because PR #56 never entered `master`, this successor intentionally reuses the next mainline workspace number, `0063`. The unmerged PR remains recoverable Git/GitHub investigation history; its single-plan lease, Planning-owned lease commit, fixture-local storage topology, and single-coordinate admission transaction are not carried forward as the default design.

This topic does not change active authority, reopen a tranche, admit production source, migrate 0062, register a Consumer architecture gate, or unblock WP-400.

## Repository conflict that must be reconciled

The active authority currently contains one material ownership conflict.

`docs/architecture/20-module-boundaries.md`, `PLAN-SET-001`, the Planning ownership table, WP-200, and 0062 all assign the important aggregate lifecycle direction consistently:

- Planning interprets the validated TD, resolves effective forms, coordinates binding compilers, and produces immutable admitted build material;
- Servient owns the composing transaction, immutable registration snapshot, capacity reservation, plan-set record, publication, pins, draining, reclamation, cancellation, and cleanup.

However, the opening ownership sentence in `docs/spec/planning.md` also calls that specification the normative owner of compiled-plan-set publication and plan reclamation. That sentence conflicts with the more specific lifecycle clauses in the same active specification and with the registered architecture boundary.

No implementation or migration may silently choose between those readings. If this proposal is accepted, the migration must remove the conflicting interpretation and retain one coherent rule: Planning owns plan material construction; Servient owns the aggregate transaction and plan-set lifecycle authority.

## Candidate architectural direction

The current candidate is intentionally expressed as semantic ownership and lifecycle, not final Rust spelling.

### 1. The authority unit is one complete Consumer plan-set admission

The central owner is one Servient-private, move-only transaction covering the entire unpublished Consumer Property Read plan-set generation:

```text
ConsumerAdmissionTxn                         owner: Servient
  Captured
    -> Validating                            TD bounded Basic + typed census
    -> Preflighting                          Planning enumeration/count/bounds
    -> Reserving                             Servient only; no callback
    -> Building                              Planning + binding compiler progress
    -> Reconciling                           Servient verifies exact <= reserved
    -> Frozen                                complete unpublished plan-set material

  any failure/cancel
    -> Aborting                              first cause + all cleanup owners retained
    -> FailedSettled                         no provisional material/reservation remains
```

`Frozen -> Published` is a later Servient publication transition. It is not part of the admission-to-Planning constructibility claim and no Stage-C runtime/publication evidence is required to decide this topic.

A per-plan lease must not become the lifecycle authority. Individual `PlanId` values are assignments inside one plan-set generation, not independently committable admission transactions.

### 2. Servient retains the plan-set lease

Servient owns one full-plan-set build/reservation authority for the whole transaction. Conceptually it binds:

- the exact `PlanSetGeneration`;
- every PlanId assignment for the aggregate;
- local and hierarchical/global resource reservations;
- persistent plan/artifact/index capacity;
- temporary/cursor/cleanup capacity needed by the admitted build;
- reconciliation and rollback ownership.

The exact Rust name and representation are not frozen here. `PlanSetBuildLease` is only a useful working name.

The lease never moves into Planning and Planning never commits or releases it. Planning may receive immutable, non-authoritative identity assignments sufficient to construct plan values. Servient alone reconciles the returned draft against the retained lease and transitions the aggregate toward Frozen or abort cleanup.

### 3. TD provides validated provenance, not Planning authority

Consumer admission begins from one caller-owned immutable borrowed `Thing` for the first proof.

TD should own one bounded Basic validation engine and typed structural census. Its successful output must be non-forgeable provenance for the exact borrowed Thing and must not be independently constructible by Planning callers. The final public name/representation is not frozen; `ValidatedThingRef<'td>` is a working semantic description.

The borrowed source:

- remains immutably borrowed for the validation/Planning admission lifetime;
- contributes zero engine-owned retained-source bytes in this representation;
- is released before the consumed runtime handle exists;
- may later have an owned convenience wrapper only if that wrapper drives the same canonical transaction and accounts its retained source separately.

TD validation does not select forms, registrations, bindings, plans, or execution owners.

### 4. Planning owns aggregate interpretation and sealed draft construction

Planning owns one move-only aggregate Consumer Property Read build session. It receives only already-captured, non-replaceable inputs/views supplied by the enclosing Servient transaction:

- validated TD provenance/view;
- checked Planning policy;
- immutable complete-registration Planning projection/snapshot view;
- non-authoritative plan identity assignments;
- admitted work/capacity views required for bounded progress.

After start, Pending/resume must not accept a replacement Thing, validation proof, policy, registration snapshot, PlanId, PlanSetGeneration, or raw `PlanBuildInput`.

Planning performs the deterministic aggregate algorithm and returns a sealed unpublished draft plus exact measured footprint. The draft has no publication, reservation, execution, or Servient-registry authority.

The final public/internal type names and exact generic storage strategy remain migration work.

### 5. Aggregate first-proof semantics

The first Consumer Property Read proof is one complete Property Read projection rather than one externally selected property/Form transaction.

The candidate semantics are:

- `Thing::properties` traversal follows deterministic `BTreeMap` key order;
- Forms preserve their source array index/order;
- every admitted effective `ReadProperty` coordinate is mandatory eager work;
- one terminal coordinate validation/bounds/compiler/reconciliation failure fails the whole unpublished aggregate;
- there is no silent skip, lazy negative, implicit next-Form selection, or per-coordinate registration reselection;
- one Consumer-capable complete registration is selected for the first proof before coordinate enumeration under an explicit non-ambiguous rule; registration order must not silently resolve ambiguity;
- capability, compiler, binding identity/configuration/compatibility, retained candidate identity, and the later execution half originate from the same complete registration entry;
- the actual registration-snapshot ordinal remains distinct from the registration's diagnostic ordinal;
- retained candidate order for the first single-registration proof is deterministic;
- a declared property with no readable effective Form remains distinguishable in the immutable target projection from an absent property;
- an aggregate with zero readable coordinates may be represented only if the resulting lookup semantics remain explicit and no runtime TD rescan is required.

The first-proof security boundary remains deliberately narrow: only deterministic no-material NoSec shapes that need no credential/provider/branch decision are eligible. Broader security remains outside this claim.

### 6. Build-time registration and execution ownership must join at one authority boundary

The old split between "build-time registration identity" and a later unrelated execution owner is unsafe if the plan-set can publish without a generation-checked path to the same selected execution half.

This topic therefore includes the contract-level requirement that the Servient-owned aggregate transaction capture a complete registration source from which both:

1. Planning derives the exact compiler/candidate identity used to build artifacts; and
2. the Frozen/Published plan-set generation can retain or pin the exact Host/static execution owner needed by the selected binding.

The temporary Planning view may disappear after build; the execution-capable owner required by the consumed generation may not.

The exact persistent execution-pin representation and later call mechanics remain a Core/Servient migration and WP-400 completion concern. This topic freezes only the no-substitution/no-bypass ownership relationship needed to make publication constructible.

### 7. Resource and work admission uses preflight -> reserve -> build -> reconcile

The first proof should not build final aggregate objects before Servient knows the complete reservation requirement.

The candidate flow is:

1. bounded TD Basic validation and typed census;
2. Planning aggregate preflight/count/bounds using the same captured source and registration snapshot;
3. side-effect-free collection of every applicable `BindingCompilerBounds` before compiler `start`;
4. Servient atomically reserves the aggregate identity/capacity/resource bundle;
5. Planning constructs final plans/artifacts/index material only inside admitted bounds;
6. Servient reconciles exact measured footprint against the retained reservation; actual use must be `<=` reserved and unused capacity is released before Frozen.

Servient must not rescan/reinterpret TD/Form state to reconstruct counts or footprints.

Foundation must remain vocabulary-neutral. The current work taxonomy lacks an obviously correct class for generic typed-document traversal and for Planning enumeration/index/reconciliation. If accepted, migration must introduce accurately scoped generic work classes and lifetime ceilings without placing TD vocabulary in Foundation. The working names `DocumentItems` and `PlanningItems` are not frozen by this topic.

At minimum the bounded work model must distinguish:

- typed document traversal/predicate work;
- URI/security work where existing classes already apply;
- Planning coordinate/candidate/index/reconciliation progress;
- binding compiler progress (`BindingPolls` unless authority changes);
- cleanup progress.

A per-step compiler cap is insufficient by itself. Admission also needs bounded aggregate compiler lifetime work and bounded total Planning lifetime work.

When an existing SPI receives one `&mut WorkBudget`, the admitted wrapper may use a child-budget partition only if the partition/reconciliation is failure-atomic against both the admission-lifetime allowance and the caller's current step allowance. Exact Foundation primitives are migration work, not silently invented local accounting.

### 8. Failure, cancellation, and abort are transaction states

Pending is the same linear transaction/session, not a cursor that can be paired with fresh inputs.

The first terminal cause is immutable. Failure or cancellation transitions into explicit Aborting ownership before terminal failure becomes observable.

If a binding compiler has a live cursor, its real `BindingCompilerExtension::abort` must be invoked exactly once under the accepted callback contract before that cursor is discarded or its cleanup responsibility is transferred. Artifact/temp/reservation release must remain bounded and outside broad Servient registry locks.

A zero applicable step budget performs no semantic callback.

Host and application-static profiles preserve the same semantic lifecycle while using profile-appropriate storage/progress. Final implementation should prefer safe state representations; a `union + ManuallyDrop` exploration fixture is not an architectural requirement.

Drop behavior, cleanup transfer, and manual static progress require an accepted exact contract before implementation admission, but Stage-C production executor behavior is not required merely to decide the ownership model.

## Public authority boundary

The canonical admitted Consumer path must originate at Servient from an ordinary borrowed TD input and drive the same linear transaction. Exact API spelling remains migration work; conceptually `begin_consume(&Thing, ...)` / `consume(&Thing, ...).await` are possible Host-facing forms.

Existing raw Planning/Core values may remain useful data or lower-level algorithm/testing surfaces, but they must not independently confer admitted Consumer publication/execution authority.

In particular, the migration must explicitly disposition current public Consumer surfaces such as:

- `PlanBuildInput::new(&Thing, &registrations, generation)`;
- `PropertyReadPlanCompiler::consumer_call(raw PlanId, raw registration identity, raw ordinal, ...)`;
- public raw build-output construction; and
- any Servient installation/execution entry that could accept independently forged plan/output/artifact identity.

The accepted result may narrow, split, trust-label, or replace those surfaces, but it cannot leave a second safe public path that bypasses validation provenance, aggregate reservation, same-registration construction, or Servient publication authority.

Producer Planning/public APIs are not presumed affected merely because some current Rust types are shared. Their exact impact must be proven rather than inferred.

## Authority migration candidates if this direction is accepted

No file below is changed by this DISCUSSING proposal. A later migration must project one coherent interpretation into every affected registered owner.

At minimum the impact review must inspect:

- `docs/architecture/20-module-boundaries.md`;
- `docs/architecture/30-compiled-plan-lifecycle.md`;
- `docs/spec/planning.md`, including removal of the publication/reclamation ownership contradiction;
- `docs/spec/foundation.md` work/resource admission rules;
- TD validation authority and any validated-view/public-constructor contract;
- Core registration, compiler, identity, artifact, and execution-pin/no-bypass contracts;
- `docs/state-machines.toml`, especially any `PlanningBuildOwner` wording that could imply crate-level plan-set transaction ownership;
- `docs/api-ownership.csv` and applicable public-surface ownership/removal records;
- WP-000 / WP-100 / WP-200 / WP-300 / WP-400 work-package and evidence records;
- `workspace/0062`, whose final handoff should be reframed around an aggregate sealed draft plus Servient-retained reservation/execution ownership rather than a single-plan handoff.

A durable cross-domain ownership reversal/clarification may require an ADR if existing accepted rationale is materially changed rather than merely reconciled.

## ADR-0013 impact hypotheses, not status changes

The following are investigation hypotheses only. `docs/work-packages/index.toml` remains authoritative until an independently reviewed impact migration changes it.

- `WP-000`: affected because generic work taxonomy, paired/lifetime budgeting, and hierarchical reservation/reconciliation may require Foundation changes. Decide scoped successor tranche versus reopen only after exact public/source impact is known.
- `WP-100-CONSUMER-CALL-VALUES-VALIDATOR`: likely reaffirm; its call values/response validator contract is not inherently invalidated by aggregate admission.
- TD bounded Consumer Basic admission: requires one narrow predecessor contract/tranche or another explicit authoritative owner; do not activate broad deferred validation/cache/codec scope.
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING`: definitely affected. Its frozen single-coordinate Consumer API/output is inconsistent with the aggregate admitted path candidate, so impact review must test whether completed evidence can survive; reopen is the leading hypothesis, not yet an authorized status transition.
- Producer-side WP-200 Planning: presumed unchanged only if the Consumer migration is split cleanly from shared public surfaces and regression evidence proves no semantic change.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING`: definitely affected by same-registration execution pinning/no-bypass and aggregate compiler ownership. A scoped reopen is the leading hypothesis if Core public/source contracts change; response sealing and existing Host/static call mechanics may remain reaffirmable sub-evidence.
- Producer-side WP-300: no change is presumed; impact review must record disjointness or required reaffirmation.
- WP-400 Consumer tranche: still not admitted and remains blocked; there is no completed Consumer WP-400 result to reopen.
- existing Producer Property Read architecture acceptance remains Producer evidence and must not be reused as Consumer aggregate-admission acceptance.

Transitive dependents must be handled exactly as ADR-0013 requires once the affected completed-tranche set is known.

## Relationship to 0062 and the abandoned PR #56 candidate

This proposal intentionally recombines parts of 0062's prior claims A and B only where the new evidence shows they share one aggregate Servient ownership/lifecycle boundary:

- validation provenance is an input phase of the same Consumer admission transaction;
- Planning aggregate construction is the algorithmic middle of that transaction;
- complete-registration build identity and the persistent execution owner must join before a plan-set can become safely publishable.

Runtime execution/call completion remains outside this proposal.

If this topic becomes DECIDED and is migrated, 0062 should not restart from its old "add an aggregate container" framing. Its remaining handoff should be reduced to the exact sealed aggregate draft / Servient-retained lease / execution-pin contract that survives this decision, or 0062 should be marked superseded if no distinct local question remains.

PR #56 remains useful historical evidence that several lower-level invariants are constructible in isolation: borrowed source, bounded Basic direction, same-registration identity checks, complete compiler-bounds capture, paired work budgeting, ordinal separation, and failure-safe reservation handling. Those fixtures do not prove the new aggregate transaction and are not migration authority.

## Rejected directions

The current investigation rejects these as target directions unless new evidence reopens them:

1. **Continue patching the single-PlanId lease proposal.** It preserves the wrong lifecycle authority granularity and keeps Planning involved in Servient reservation/commit ownership.
2. **Let Servient rescan/reinterpret TD/Form state after Planning.** That creates a second planner and violates the existing module boundary.
3. **Let Planning own capacity reservation, publication, or plan-set reclamation.** That contradicts the Servient-owned lifecycle already established by architecture, `PLAN-SET-001`, and WP-200.
4. **Treat green CI or the old Stage-A fixtures as acceptance of the redesigned aggregate model.** They test a different topology.
5. **Immediately reopen every named package.** ADR-0013 requires an explicit requirement/artifact/dependency impact review, not a conversational status edit.
6. **Activate broad deferred Consumer validation/security/cache/fallback scope to solve this one-shot proof.** The v5.1 first proof remains deliberately narrow.

## Evidence required before DECIDED

This topic may move from DISCUSSING to DECIDED only after the following pre-decision evidence exists at one exact reviewed revision:

1. **Authority conflict map:** every active claim about Planning versus Servient plan-set/build/publication/reclamation ownership is enumerated and one migration direction is non-conflicting.
2. **Lifecycle constructibility:** a minimal non-production type/state sketch or compile fixture demonstrates that one Servient-owned transaction can retain the plan-set authority while a Planning-owned aggregate session moves through Pending without accepting replacement source/policy/snapshot/identity inputs.
3. **Aggregate preflight/build contract:** the candidate proves how compiler bounds are collected before start, how aggregate identity assignments are supplied without transferring Servient authority, and how final measured footprint is reconciled against retained reservation.
4. **Failure/abort ownership:** the constructibility evidence includes at least one live compiler cursor and proves exactly-once real compiler abort plus bounded reservation/material settlement before terminal failure.
5. **Resource/work applicability:** every active first-proof source, temporary, persistent, runtime, diagnostic, cleanup, peak, contiguous, Planning-work, and compiler-work control has an explicit owner/applicability disposition; exact production measurements remain completion evidence.
6. **Public no-bypass disposition:** every existing safe public Consumer Planning/install/execution constructor that could forge or replace admitted inputs has a selected migration disposition.
7. **ADR-0013 impact map:** every affected completed tranche and transitive dependent has an explicit proposed reaffirm/reopen/disjoint result with the authority/evidence reason. Status changes occur only after independent acceptance.
8. **Host/static constructibility:** both profiles have a plausible safe storage/progress representation preserving identical semantic ownership without requiring Stage-C protocol/runtime evidence.
9. **Independent architecture review:** a fresh review of the exact proposal and pre-decision evidence finds no unresolved architecture/public-API/ownership/resource contradiction within this claim.

These are constructibility/decision artifacts. Production Zenoh behavior, final publication runtime, concurrent load measurements, real cleanup executor operation, and WP-400 completion remain post-migration implementation/completion evidence.

## Candidate migration order after acceptance

If the exact candidate is independently accepted, the smallest expected migration sequence is:

1. reconcile Planning/Servient normative ownership and state-machine wording;
2. freeze generic Foundation work/lifetime/reservation primitives required by the aggregate transaction;
3. freeze TD bounded Basic/census/validated-provenance contract;
4. freeze Core complete-registration Planning projection plus persistent execution-pin/no-bypass identity contract;
5. reopen/reaffirm completed tranches according to the accepted ADR-0013 impact map and register exact successor admission records;
6. migrate WP-200 Consumer Planning from the single-coordinate admitted surface to aggregate preflight/build/sealed-draft semantics without changing Producer semantics unless separately reviewed;
7. add only the Servient admission/plan-set lease/cancellation/storage skeleton necessary for constructibility; do not claim Stage-C publication or real protocol completion;
8. reconcile or supersede 0062 around the final aggregate handoff;
9. independently review the migrated authority/admission revision before production implementation continues.

This order is a migration hypothesis, not an implementation authorization.

## Decision question

Does active v5.1 Consumer Property Read converge on one Servient-owned aggregate admission transaction, with TD providing bounded validated provenance, Planning providing aggregate preflight/build/sealed material only, Foundation providing vocabulary-neutral bounded primitives, and Core joining complete-registration build identity to a persistent execution owner—while publication/runtime completion remains a later Servient tranche?
