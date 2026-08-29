# 0063 Consumer Aggregate Admission and Plan-Set Build Authority

Status: DISCUSSING

Kind: architecture reconciliation and replacement proposal

Priority: HIGH

Target: replace the rejected single-plan Consumer admission hypothesis with one constructible Servient-owned aggregate Consumer plan-set admission transaction, then determine the exact authority migration and ADR-0013 impact before any production Rust resumes.

## Why this topic exists

`workspace/0062-consumer-plan-set-handoff-closure.md` established that the completed Consumer WP-200 and WP-300 tranches do not compose into the required public Servient path.

An unmerged first attempt at the next claim was developed in PR #56 under the `workspace/0063` number. Seven independent review cycles made that proposal progressively more constructible, but a later independent from-first-principles design review found that its central authority granularity was wrong: it centered admission on one PlanId-bearing Planning lease, while active architecture already assigns the complete compiled-plan-set transaction, reservation, lifecycle, publication, and cleanup authority to Servient.

Because PR #56 never entered `master`, this successor intentionally reuses the next mainline workspace number, `0063`. The closed unmerged PR remains recoverable Git/GitHub investigation history; its single-plan lease, Planning-held lease commit, fixture-local storage topology, and single-coordinate admission transaction are not carried forward as the default design.

This topic does not change active authority, reopen a tranche, admit production source, migrate 0062, register a Consumer architecture gate, or unblock WP-400.

Supporting exact-source reconciliation is recorded in `workspace/0063-authority-reconciliation-notes.md`.

## Repository conflict that must be reconciled

The active authority contains one material ownership conflict.

`docs/architecture/20-module-boundaries.md`, `docs/architecture/30-compiled-plan-lifecycle.md`, `PLAN-SET-001`, the Planning ownership table, WP-200, and 0062 consistently establish the important direction:

- Planning interprets validated TD state, resolves effective forms, coordinates binding compilers, and produces immutable admitted build material;
- Servient owns the composing transaction, immutable registration snapshot, capacity reservation, plan-set record, publication, pins, draining, reclamation, cancellation, and cleanup.

However, the opening ownership sentence in `docs/spec/planning.md` also calls that specification the normative owner of compiled-plan-set publication and plan reclamation. That sentence conflicts with the more specific lifecycle clauses in the same active specification and with the registered architecture boundary.

No implementation or migration may silently choose between those readings. If this proposal is accepted, migration must retain one coherent rule: Planning owns plan-material algorithms and sealed build output; Servient owns the aggregate transaction and plan-set lifecycle authority.

## Candidate architectural direction

The candidate is expressed as semantic ownership and lifecycle. Final Rust names and private storage strategy remain migration work.

### 1. The authority unit is one complete Consumer plan-set admission

The central owner is one Servient-private, move-only transaction covering the entire unpublished Consumer Property Read plan-set generation:

```text
ConsumerAdmissionTxn                         owner: Servient
  Captured
    -> Validating                            TD bounded Basic + typed census
    -> Enumerating                           Planning resolves aggregate shape/count
    -> AssigningIdentities                   Servient reserves exact unpublished generation/PlanIds
    -> Bounding                              Planning constructs exact compiler inputs and calls bounds
    -> ReservingResources                    Servient atomically reserves capacity; no callback
    -> Building                              Planning + binding compiler progress
    -> Reconciling                           Servient verifies exact <= reserved
    -> Frozen                                complete unpublished plan-set material

  any failure/cancel
    -> Aborting                              first cause + all identity/resource/cleanup owners retained
    -> FailedSettled                         no provisional identity/material/reservation remains
```

`Frozen -> Published` is a later Servient publication transition. It is not part of this admission-to-Planning constructibility claim and no Stage-C runtime/publication evidence is required to decide this topic.

A per-plan lease must not become the lifecycle authority. Individual `PlanId` values are assignments inside one plan-set generation, not independently committable admission transactions.

### 2. Identity assignment precedes compiler bounds; resource reservation does not

The first independent redesign proposed `compiler bounds -> reserve PlanIds/resources -> build`. Current Core makes that exact order unconstructible without an additional SPI change:

- `BindingCompilerExtension::bounds` accepts `BindingCompilerInput`;
- `BindingCompilerInput` exposes the complete `LogicalInteractionPlan`;
- `LogicalInteractionPlan` contains and exposes its exact `PlanId`.

Therefore a compiler is currently allowed to observe PlanId while calculating bounds. Supplying a placeholder PlanId would make preflight and build inputs differ. Delaying every PlanId until after bounds would make `bounds` impossible to call on the exact final input.

The candidate resolves this without broadening Core SPI:

1. Planning first performs bounded aggregate shape enumeration sufficient to know the exact mandatory coordinate count and deterministic source identities, without starting any compiler;
2. Servient reserves/assigns the exact unpublished `PlanSetGeneration` and PlanIds for that aggregate;
3. those immutable non-authoritative identity assignments enter Planning's exact logical-plan/compiler inputs;
4. Planning collects every applicable `BindingCompilerBounds` before compiler `start`;
5. only then does Servient atomically reserve the aggregate memory/work/capacity bundle;
6. compiler/build progress may start only after that resource reservation succeeds.

Identity reservation and resource reservation are therefore distinct phases of one Servient-owned plan-set build authority. An identity assignment has no publication/execution authority and is released/invalidated with the unpublished generation on any failure before Frozen.

Changing the compiler SPI so `bounds` cannot observe PlanId remains a possible later alternative only if migration evidence shows the identity-first sequence is materially worse. It is not required by this first-proof candidate.

### 3. Servient retains the full plan-set authority

Servient owns one full-plan-set build authority through all phases. Conceptually it eventually binds:

- the exact unpublished `PlanSetGeneration`;
- every exact PlanId assignment for the aggregate;
- local and hierarchical/global resource reservations;
- persistent plan/artifact/index capacity;
- temporary/cursor/cleanup capacity needed by admitted build progress;
- reconciliation and rollback ownership.

The exact Rust name is not frozen; `PlanSetBuildLease` is only a working description.

This authority never moves into Planning and Planning never commits or releases it. Planning receives immutable identity assignments and admitted resource/work views that are sufficient to build values but do not carry Servient lifecycle authority. Servient alone reconciles the returned draft against its retained identity/reservation owner and moves the record toward Frozen or bounded abort settlement.

### 4. TD provides validated provenance, not Planning authority

Consumer admission begins from one caller-owned immutable borrowed `Thing` for the first proof.

TD should own one bounded Basic validation engine and typed structural census. Successful validation must produce non-forgeable provenance for the exact borrowed Thing and must not be independently constructible by Planning callers. The final public name/representation is not frozen; `ValidatedThingRef<'td>` is a working semantic description.

The borrowed source:

- remains immutably borrowed for the validation/Planning admission lifetime;
- contributes zero engine-owned retained-source bytes in this representation;
- is released before the consumed runtime handle exists;
- may later have an owned convenience wrapper only if that wrapper drives the same canonical transaction and accounts its retained source separately.

TD validation does not select forms, registrations, bindings, plans, or execution owners.

### 5. Planning owns aggregate interpretation and sealed draft construction

Planning owns one move-only aggregate Consumer Property Read algorithm session. It receives only captured/non-replaceable inputs or views supplied by the enclosing Servient transaction:

- validated TD provenance/view;
- checked Planning policy;
- immutable complete-registration Planning projection/snapshot view;
- exact non-authoritative plan identity assignments after `AssigningIdentities`;
- admitted work/capacity views after `ReservingResources`.

After a phase starts, Pending/resume must not accept a replacement Thing, validation proof, policy, registration snapshot, PlanId, PlanSetGeneration, or raw `PlanBuildInput`.

The enumeration/bounding/build phases may be separate Planning typestates, but they are algorithmic subowners inside one Servient transaction. Planning returns a sealed unpublished aggregate draft plus exact measured footprint. The draft has no publication, reservation, execution, or Servient-registry authority.

### 6. Aggregate first-proof semantics

The first Consumer Property Read proof is one complete Property Read projection rather than one externally selected property/Form transaction.

Candidate semantics:

- `Thing::properties` traversal follows deterministic `BTreeMap` key order;
- Forms preserve source array index/order;
- every admitted effective `ReadProperty` coordinate is mandatory eager work;
- one terminal coordinate validation/bounds/compiler/reconciliation failure fails the whole unpublished aggregate;
- no silent skip, lazy negative, implicit next-Form selection, or per-coordinate registration reselection;
- one Consumer-capable complete registration is selected before coordinate compilation under an explicit non-ambiguous rule; registration order must not silently resolve ambiguity;
- capability, compiler, binding identity/configuration/compatibility, retained candidate identity, and later execution half originate from the same complete registration entry;
- actual registration-snapshot ordinal remains distinct from registration diagnostic ordinal;
- retained candidate order for the first single-registration proof is deterministic;
- a declared property with no readable effective Form remains distinguishable in immutable target material from an absent property;
- an aggregate with zero readable coordinates is legal only if lookup remains explicit and no runtime TD rescan is required.

The first-proof security boundary remains deliberately narrow: only deterministic no-material NoSec shapes needing no credential/provider/branch decision are eligible. Broader security remains outside this claim.

### 7. Build-time registration and execution ownership join at one authority boundary

The old split between build-time registration identity and a later unrelated execution owner is unsafe if a plan-set can publish without a generation-checked path to the same selected execution half.

The Servient-owned aggregate transaction must capture a complete registration source from which both:

1. Planning derives the exact compiler/candidate identity used to build artifacts; and
2. the Frozen/Published plan-set generation can retain or pin the exact Host/static execution owner needed by the selected binding.

The temporary Planning projection may disappear after build; the execution-capable owner needed by the consumed generation may not.

Exact persistent execution-pin representation and later call mechanics remain Core/Servient migration and WP-400 completion work. This topic freezes only the no-substitution/no-bypass relationship needed to make publication constructible.

### 8. Resource/work admission is enumerate -> identify -> bound -> reserve -> build -> reconcile

The first proof must not build final aggregate objects before Servient knows the complete reservation requirement, but exact identity assignments must exist before current compiler bounds are called.

Detailed candidate flow:

1. bounded TD Basic validation and typed census;
2. Planning bounded shape enumeration/count using the same captured source and registration snapshot, with no compiler start/step;
3. Servient assigns the exact unpublished plan-set generation and all mandatory PlanIds;
4. Planning constructs exact logical-plan/candidate inputs and collects every applicable `BindingCompilerBounds` before any compiler `start`;
5. Planning returns aggregate requirements: final logical/index/artifact maxima, cursor/temp peaks, cleanup needs, largest-contiguous requirement, Planning lifetime work, aggregate compiler lifetime work, and other active first-proof bounds;
6. Servient atomically acquires all applicable local + parent/global resource reservations while retaining the already-assigned unpublished identities;
7. Planning constructs final plans/artifacts/index material only within those admitted bounds;
8. Servient reconciles exact measured footprint against the retained reservation; actual use must be `<=` reserved and unused capacity is released before Frozen.

Servient must not rescan/reinterpret TD/Form state to reconstruct counts or footprints.

Foundation must remain vocabulary-neutral. Current `WorkClass` lacks an obviously correct class for generic typed-document traversal and for Planning enumeration/index/reconciliation. If accepted, migration must add accurately scoped generic work accounting without placing TD vocabulary in Foundation. Working names such as `DocumentItems` or `PlanningItems` are not frozen here.

At minimum the bounded work model distinguishes:

- typed document traversal/predicate work;
- URI/security work where existing classes already apply;
- Planning coordinate/candidate/index/reconciliation progress;
- binding compiler progress (`BindingPolls` unless authority changes);
- cleanup progress.

A per-step compiler cap is insufficient. Admission also needs bounded aggregate compiler lifetime work and bounded total Planning lifetime work.

When an existing SPI receives one `&mut WorkBudget`, an admitted child-budget partition is acceptable only if partition/reconciliation is failure-atomic against both admission-lifetime allowance and caller step allowance. Exact Foundation primitives are migration work, not fixture-local authority.

### 9. Failure, cancellation, and abort are transaction states

Pending is the same linear transaction/session, not a cursor that can be paired with fresh inputs.

The first terminal cause is immutable. Failure/cancellation enters explicit Aborting ownership before terminal failure becomes observable.

If a binding compiler has a live cursor, its real `BindingCompilerExtension::abort` is invoked exactly once before the cursor is discarded or cleanup responsibility is transferred. Artifact/temp/resource reservations and unpublished identity assignments are released or invalidated under bounded settlement. Release occurs outside broad Servient registry locks.

A zero applicable step budget performs no semantic callback.

Host and application-static profiles preserve identical semantic lifecycle with profile-appropriate storage/progress. Final implementation should prefer safe state representations; a `union + ManuallyDrop` exploration fixture is not an architectural requirement.

Drop behavior, cleanup transfer, and manual static progress require an accepted exact contract before implementation admission, but Stage-C production executor behavior is not required merely to decide this ownership model.

## Public authority boundary

The canonical admitted Consumer path originates at Servient from an ordinary borrowed TD input and drives the same linear transaction. Exact API spelling remains migration work; conceptually `begin_consume(&Thing, ...)` / `consume(&Thing, ...).await` are possible Host-facing forms.

Existing raw Planning/Core values may remain public data or lower-level algorithm/testing surfaces, but they must not independently confer admitted Consumer publication/execution authority.

Migration must explicitly disposition current public Consumer/shared surfaces including:

- `PlanCompiler`;
- `PropertyReadPlanCompiler` and `PropertyReadPlanCompiler::consumer_call`;
- `PropertyReadBuildCursor`;
- `PlanBuildInput`;
- `PlanBuildOutput` and public raw output construction;
- any Servient installation/execution entry that could accept independently forged plan/output/artifact identity.

The accepted result may narrow, split, trust-label, or replace these surfaces, but it cannot leave a second safe public path that bypasses validation provenance, Servient identity/resource reservation, same-registration construction, aggregate reconciliation, or publication gating.

Producer Planning/public APIs are not presumed affected merely because current Rust types are shared. Their exact impact must be proven rather than inferred.

## Authority migration candidates if accepted

No file below is changed by this DISCUSSING proposal. A later migration must project one coherent interpretation into every affected registered owner.

At minimum inspect:

- `docs/architecture/20-module-boundaries.md`;
- `docs/architecture/30-compiled-plan-lifecycle.md`;
- `docs/spec/planning.md`, including removal of the publication/reclamation ownership contradiction;
- `docs/spec/foundation.md` work/resource admission rules;
- TD validation authority and validated-provenance contract;
- Core registration, compiler, identity, artifact, and execution-pin/no-bypass contracts;
- `docs/state-machines.toml`, especially `PlanningBuildOwner` wording inside the Servient-owned compiled-plan-set record;
- `docs/api-ownership.csv` and public-surface migration/removal records;
- WP-000 / WP-100 / WP-200 / WP-300 / WP-400 work-package and evidence records;
- `workspace/0062`.

A durable cross-domain reversal/clarification may require an ADR if accepted rationale is materially changed rather than merely made internally consistent.

## ADR-0013 impact hypotheses, not status changes

`docs/work-packages/index.toml` remains authoritative while this topic is DISCUSSING.

- `WP-000`: affected if generic work taxonomy, paired/lifetime budgeting, or hierarchical reservation/reconciliation requires Foundation public/source changes. Decide additive successor tranche versus reopen only from exact impact.
- `WP-100-CONSUMER-CALL-VALUES-VALIDATOR`: likely reaffirm; call values/response validation are not inherently invalidated by aggregate admission.
- TD bounded Consumer Basic admission: requires one narrow predecessor contract/tranche or another explicit authoritative owner; broad deferred validation/cache/codec scope remains inactive.
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING`: definitely affected. Its frozen single-coordinate Consumer API/output conflicts with the aggregate admitted path candidate. Reopen is the leading hypothesis but not an authorized status change until impact review.
- Producer WP-200: presumed unchanged only if Consumer migration is split cleanly from shared public surfaces and regression evidence proves no semantic change.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING`: definitely affected by same-registration execution pinning/no-bypass and aggregate compiler ownership. Scoped reopen is the leading hypothesis if Core public/source contracts change; valid response/call mechanics may remain reaffirmable evidence.
- Producer WP-300: no change is presumed; impact review records disjointness or required reaffirmation.
- WP-400 Consumer: still not admitted and remains blocked; there is no completed Consumer WP-400 result to reopen.
- existing Producer Property Read architecture acceptance remains Producer evidence and is not Consumer aggregate-admission acceptance.

Transitive dependents are handled exactly as ADR-0013 requires once the affected completed-tranche set is known.

## Relationship to 0062 and closed PR #56

This proposal recombines parts of 0062's prior claims A and B only where new evidence shows they share one aggregate Servient ownership/lifecycle boundary:

- validation provenance is an input phase of the same Consumer admission transaction;
- aggregate Planning enumeration/bounds/build is the algorithmic middle;
- complete-registration build identity and persistent execution owner must join before safe publication is constructible.

Runtime execution/call completion remains outside this proposal.

If this topic becomes DECIDED and migrated, 0062 should not restart from its old "add an aggregate container" framing. Its remaining handoff should reduce to the exact sealed aggregate draft / Servient-retained identity-resource-execution ownership that survives this decision, or 0062 should be marked superseded if no distinct local question remains.

Closed PR #56 remains useful historical evidence that several lower-level invariants are constructible in isolation: borrowed source, bounded Basic direction, same-registration identity checks, complete compiler-bounds capture, paired work budgeting, ordinal separation, and failure-safe reservation handling. Those fixtures do not prove this aggregate transaction and are not migration authority.

## Rejected directions

1. **Continue patching a single-PlanId Planning lease.** Wrong lifecycle authority granularity.
2. **Call compiler bounds with placeholder PlanIds and substitute final ids later.** Current compiler input exposes PlanId; this would make bounds and build operate on different inputs.
3. **Delay PlanId assignment until after current compiler bounds without changing SPI.** Current `BindingCompilerInput` makes that sequence unconstructible.
4. **Change Core compiler SPI immediately merely to preserve the original preflight ordering.** More cross-package/public churn than the identity-first sequence currently requires.
5. **Let Servient rescan/reinterpret TD/Form state after Planning.** Creates a second planner.
6. **Let Planning own capacity reservation, publication, or plan-set reclamation.** Conflicts with active lifecycle ownership.
7. **Treat green CI or old Stage-A fixtures as aggregate acceptance.** They test a different topology.
8. **Immediately reopen every named package.** ADR-0013 requires exact impact review.
9. **Activate broad deferred Consumer validation/security/cache/fallback scope.** Outside narrow v5.1 one-shot proof.

## Evidence required before DECIDED

This topic may move to DECIDED only after one exact revision contains:

1. **Authority conflict map:** every active Planning/Servient build/publication/reclamation claim is dispositioned to one non-conflicting owner.
2. **Lifecycle constructibility:** a minimal non-production type/state fixture shows one Servient-owned transaction retaining identity/resource authority while Planning moves through Pending with no replacement source/policy/snapshot/identity input.
3. **Identity-before-bounds proof:** the fixture shows exact unpublished PlanId assignments are available to current compiler `bounds`, remain Servient-owned/non-authoritative to Planning, and are released/invalidated on failure before Frozen.
4. **Aggregate preflight/build proof:** compiler bounds are collected before `start`; resource reservation occurs after bounds but before build; exact measured footprint reconciles against retained reservation.
5. **Failure/abort ownership:** at least one live compiler cursor is aborted through the real SPI exactly once and identity/resource/material ownership settles before terminal failure.
6. **Resource/work applicability:** every active first-proof source, temporary, persistent, runtime, diagnostic, cleanup, peak, contiguous, Planning-work, and compiler-work control has an explicit owner/applicability disposition; production measurements remain completion evidence.
7. **Public no-bypass disposition:** every safe public Consumer/shared Planning/install/execution constructor that could forge or replace admitted inputs has a selected migration disposition.
8. **ADR-0013 impact map:** every affected completed tranche and transitive dependent has a proposed reaffirm/reopen/disjoint result with authority/evidence reason. Status changes occur only after independent acceptance.
9. **Host/static constructibility:** both profiles have a plausible safe storage/progress representation preserving identical semantic ownership without Stage-C protocol/runtime evidence.
10. **Independent architecture review:** a fresh review of the exact proposal/evidence finds no unresolved architecture/public-API/ownership/resource contradiction in this claim.

Production Zenoh behavior, final publication runtime, concurrent load measurements, real cleanup executor operation, and WP-400 completion remain post-migration evidence.

## Candidate migration order after acceptance

1. reconcile Planning/Servient normative ownership and state-machine wording;
2. freeze generic Foundation work/lifetime/reservation primitives required by the aggregate transaction;
3. freeze TD bounded Basic/census/validated-provenance contract;
4. freeze Core complete-registration Planning projection plus persistent execution-pin/no-bypass identity contract;
5. apply the independently accepted ADR-0013 completed-tranche impact transitions and register exact successor admissions;
6. migrate WP-200 Consumer Planning from single-coordinate admitted surface to aggregate enumerate/bound/build/sealed-draft semantics without changing Producer semantics unless separately reviewed;
7. add only the Servient admission/identity/resource lease/cancellation/storage skeleton necessary for constructibility, not Stage-C publication/protocol completion;
8. reconcile or supersede 0062 around the final aggregate handoff;
9. independently review the migrated authority/admission revision before production implementation continues.

This order is a migration hypothesis, not implementation authorization.

## Decision question

Does active v5.1 Consumer Property Read converge on one Servient-owned aggregate admission transaction in which TD provides bounded validated provenance, Planning provides aggregate enumeration/bounds/build/sealed material only, Servient assigns exact unpublished plan identities before compiler bounds and retains all identity/resource/lifecycle authority, Foundation supplies vocabulary-neutral bounded primitives, and Core joins complete-registration build identity to a persistent execution owner—while publication/runtime completion remains a later Servient tranche?
