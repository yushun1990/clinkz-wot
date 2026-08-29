# 0063 Consumer Aggregate Admission and Plan-Set Build Authority

Status: DISCUSSING

Kind: architecture reconciliation and replacement proposal

Priority: HIGH

Target: replace the rejected single-plan Consumer admission hypothesis with one constructible Servient-owned aggregate Consumer plan-set admission transaction, then migrate the exact accepted authority and ADR-0013 impact before production Rust resumes.

## Why this topic exists

`workspace/0062-consumer-plan-set-handoff-closure.md` established that the completed Consumer WP-200 and WP-300 tranches do not compose into the required public Servient path.

An unmerged first attempt at the next claim was developed in PR #56 under the `workspace/0063` number. Seven independent review cycles made that proposal progressively more constructible, but a later from-first-principles review found its central authority granularity wrong: it centered admission on one PlanId-bearing Planning lease while accepted architecture assigns admission and lifecycle to one Servient-owned aggregate compiled plan set.

Because PR #56 never entered `master`, this successor reuses the next mainline workspace number. The closed unmerged PR remains recoverable investigation history; its single-plan lease, Planning-held lease commit and single-coordinate admission transaction are not target architecture.

This topic does not change active authority, reopen a tranche, admit production source, register a Consumer architecture gate, or unblock WP-400.

Supporting source reconciliation and resource disposition live in:

- `workspace/0063-authority-reconciliation-notes.md`; and
- `workspace/0063-resource-work-applicability.md`.

## Normative ownership and runtime ownership

There is no `PLAN-SET-001` normative-owner conflict to solve.

`docs/spec/planning.md` remains the registered normative owner of `PLAN-SET-001`: it defines the required semantics of compiled-plan-set publication and reclamation. The same specification, ADR-0008, module boundaries, compiled-plan lifecycle architecture and WP-400 already assign the runtime plan-set record, build transaction, reservation, publication, draining and reclamation implementation to Servient.

The candidate therefore preserves this split:

- Planning specification owns the normative Planning/plan-set contract;
- `clinkz-wot-planning` owns deterministic plan-material algorithms and sealed build output;
- `clinkz-wot-servient` owns the runtime aggregate transaction and compiled-plan-set lifecycle record.

Migration may clarify wording that could be read as crate ownership, but it must not orphan or silently relocate `PLAN-SET-001`. ADR-0008 is reaffirmed, not reversed. WP-400 remains the runtime implementation owner and remains blocked until upstream migration is admitted.

## Candidate architectural direction

### 1. One complete Consumer plan-set admission is the authority unit

The central owner is one Servient-private, move-only transaction for the entire unpublished Consumer Property Read plan-set generation:

```text
ConsumerAdmissionTxn                         owner: Servient
  Captured
    -> Validating                            TD bounded Basic + typed census
    -> SelectingRegistration                 exact-one complete registration metadata selection
    -> Enumerating                           Planning resolves aggregate shape/count
    -> AssigningIdentities                   Servient reserves exact unpublished generation/PlanIds
    -> Bounding                              Planning constructs exact compiler inputs and calls bounds
    -> ReservingResources                    Servient atomically reserves capacity; no callback
    -> Building                              Planning + binding compiler progress
    -> Reconciling                           Servient verifies complete material + exact ledger <= reserved
    -> Frozen                                complete unpublished plan-set material + committed account + execution pin

  any failure/cancel
    -> Aborting                              first cause + all identity/resource/material/cursor owners retained
    -> FailedSettled                         no provisional identity/material/reservation remains
```

`Frozen -> Published` is later Servient/WP-400 work. A per-plan lease never becomes lifecycle authority. Individual `PlanId` values are assignments inside one plan-set generation, not independently committable transactions.

### 2. Registration selection is exactly-one, metadata-only

Before coordinate compilation, Servient selects exactly one eligible complete registration from the captured immutable startup snapshot.

Eligibility requires:

1. the complete registration already passed Core registration validation;
2. it advertises Consumer Property Read; and
3. it supplies the execution half for the active runtime profile.

Selection invokes no binding callback, support probe, wildcard probe or protocol I/O.

- zero eligible entries -> structured no-eligible-registration admission failure;
- one eligible entry -> capture that exact snapshot entry for the entire aggregate;
- more than one -> structured ambiguous-registration admission failure.

Registration order never breaks ties. Bounds/build failure never causes reselection. The selected entry is the single source of compiler identity, candidate identity and persistent execution ownership. Snapshot ordinal is the positional coordinate; diagnostic ordinal is diagnostic metadata and may differ.

### 3. Exact identity assignment precedes current compiler bounds

Current Core makes `compiler bounds -> assign final PlanIds` unconstructible without changing the compiler SPI:

- `BindingCompilerExtension::bounds` receives `BindingCompilerInput`;
- `BindingCompilerInput` exposes the complete `LogicalInteractionPlan`;
- `LogicalInteractionPlan` contains/exposes final `PlanId`.

Therefore the selected sequence is:

1. bounded aggregate shape enumeration without compiler progress;
2. Servient reserves/assigns exact unpublished `PlanSetGeneration` and all mandatory PlanIds;
3. Planning constructs each exact final logical-plan/compiler input once;
4. Planning collects every `BindingCompilerBounds` before compiler `start`;
5. Servient reserves all applicable resources after all bounds succeed;
6. build starts only after resource reservation.

Identity reservation and resource reservation are separate phases of one Servient-owned authority. An aborted unpublished identity generation is invalidated before slot reuse; reuse advances generation so stale PlanIds cannot alias the next admission.

Changing Core compiler SPI solely to preserve bounds-before-identity ordering is rejected for this proof.

### 4. Servient retains identity, resource and lifecycle authority

The Servient owner retains:

- exact unpublished plan-set generation and PlanId assignments;
- selected complete-registration source and persistent execution pin;
- local plus parent/global resource reservations;
- persistent logical-plan/artifact/index capacity;
- compiler cursor/temp and cleanup capacity;
- reconciliation/rollback ownership.

Planning never receives the lifecycle lease and never commits/releases Servient reservations. It receives only immutable identity assignments and admitted resource/work views sufficient to construct plan material.

At successful reconcile, phase-local temporary reservation is released, but persistent plan/artifact/index capacity and execution ownership transfer into the Frozen Servient record as a committed account. Freeze must not drive all reservation/account state to zero. Persistent capacity reaches zero only at later reclamation.

### 5. TD provides bounded validated provenance

First proof begins from one caller-owned immutable borrowed `Thing`.

TD owns bounded Basic validation and typed semantic census. Successful validation produces non-forgeable provenance for the exact borrow. Planning cannot independently manufacture that provenance.

The selected resource migration does not reinterpret Raw-JSON `json_*`, `string_bytes_max` or `extension_bytes_max` rows as limits on arbitrary typed Rust input. Direct typed admission gains a representation-neutral semantic structural family covering node count, depth, map entries, sequence entries, visited semantic string bytes and typed traversal work.

Borrowed source contributes zero engine-owned retained-source bytes while remaining a real lifetime dependency. The borrow ends before the consumed runtime handle exists.

### 6. Planning owns aggregate algorithms and real sealed material

Planning owns one move-only aggregate algorithm session inside the Servient Building record. It receives captured validation provenance, checked policy, the exact selected registration projection, final non-authoritative identity assignments and admitted work/capacity views.

Pending/resume accepts no replacement Thing, policy, validation proof, registration, PlanId, PlanSetGeneration or raw `PlanBuildInput`.

For every mandatory coordinate, Planning constructs the exact final `LogicalInteractionPlan` once before compiler bounds. Build later consumes that same owned plan value; it does not reconstruct a second plan from PlanId-only facts.

The sealed aggregate draft contains real owned material:

- logical plans;
- admitted `BindingArtifactEnvelope`s;
- matching `BindingArtifactRef`s;
- immutable coordinate/index material; and
- exact measured ledger/footprint.

One coordinate failing after earlier coordinates completed does not discard those prior values implicitly. They remain provisional material owned by `Aborting` until bounded release settles them.

### 7. Aggregate first-proof semantics

- `Thing::properties` traversal uses deterministic `BTreeMap` key order;
- Forms preserve source array index/order;
- every admitted effective `ReadProperty` coordinate is mandatory eager work;
- terminal coordinate validation/bounds/compiler/reconciliation failure fails the entire unpublished aggregate;
- no silent skip, lazy negative, next-Form selection, per-coordinate registration reselection or fallback;
- all coordinates use the one selected complete registration;
- candidate/order/index identities are deterministic;
- a declared property with no readable effective Form remains distinguishable from an absent property;
- zero-readable-coordinate aggregate is legal only if immutable lookup semantics remain explicit with no runtime TD rescan;
- first-proof security admits only deterministic no-material NoSec requiring no credential/provider/branch decision.

### 8. Build and execution ownership join before Frozen

The selected complete registration is one indivisible authority source for this proof:

1. Planning derives compiler/candidate identity from that exact entry; and
2. Frozen/Published generation retains a pin/reference to that same entry's profile execution half.

Equal compatibility/configuration from another registration is insufficient and cannot be cross-wired. The temporary Planning projection may disappear after build; the execution-capable owner may not.

Exact production pin storage and later call mechanics remain Core/Servient migration plus WP-400 completion work, but no-substitution ownership is part of this decision.

### 9. Resource/work admission is enumerate -> identify -> bound -> reserve -> build -> reconcile

Detailed flow:

1. bounded TD Basic validation/typed census;
2. metadata-only exact-one registration selection;
3. Planning bounded aggregate enumeration/count;
4. Servient exact unpublished identity assignment;
5. Planning constructs final logical-plan/candidate inputs and collects all `BindingCompilerBounds`;
6. Planning returns complete aggregate maxima for persistent material, cursor/temp, cleanup, contiguous allocation and lifetime work;
7. Servient atomically reserves applicable local + hierarchical/global capacity;
8. Planning builds complete real aggregate material;
9. Servient reconciles every plan/envelope/ref/index identity and exact measured ledger against the retained reservation;
10. unused temporary/excess capacity is released and exact persistent usage is committed to Frozen.

Servient never rescans/reinterprets TD/Form state to rebuild counts or material.

The selected resource map is exhaustive for the current 195-row schema and uses `Active`, `ZeroContribution`, `Deferred`, or `NotApplicable`. A Stage-A coverage fixture fails if schema growth creates an unclassified row.

Foundation remains vocabulary-neutral. Migration adds truthful generic work accounting for typed semantic traversal and Planning enumeration/index/reconciliation. It also adds lifetime ceilings for total Planning work and aggregate compiler work. Existing per-step compiler cap remains additional and cannot replenish lifetime allowance.

Hierarchical accounting rows apply to reservation/reconcile; cleanup retry contributes zero before protocol side effects; cleanup transfer is deferred to runtime call cleanup.

### 10. Failure, cancellation and abort are real transaction states

Pending is one linear session, never a free cursor plus replaceable inputs.

First terminal cause is immutable. Failure/cancellation enters `Aborting` before failure becomes observable.

If a binding compiler has a live cursor, real `BindingCompilerExtension::abort` is invoked exactly once before cursor discard. Earlier completed provisional plan/artifact/ref/index material remains owned until bounded release. Resource reservation is released on abort. Unpublished plan-set generation is invalidated/advanced before reusable identity storage can be republished.

Zero applicable step budget performs no semantic callback.

Host and application-static profiles use different physical storage backends but preserve the same semantic state graph. No union/`ManuallyDrop`/unsafe representation is an architectural requirement.

## Public authority boundary: selected disposition

Only Servient can confer **admitted Consumer plan-set publication/handle authority**.

The canonical `Servient::consume` / static equivalent starts from ordinary TD/policy/profile inputs and drives the private aggregate transaction. Servient freeze/publish/install transitions are private record operations and never accept externally assembled `PlanBuildOutput`, artifacts, PlanIds, plan-set generations or execution pins as already admitted.

Public lower-level Planning/Core data and SPIs may remain directly usable for compatibility, Producer/shared algorithms, testing or manual low-level composition. They are explicitly non-authoritative: manually composing them does not produce a Servient admitted handle.

Selected target-path dispositions:

- `PlanCompiler`, `PlanBuildInput`, `PlanBuildOutput`, generic build cursor/step/failure/footprint values may remain public lower-level algorithm/data surfaces; no Servient Consumer publication API accepts them as validation/reservation proof;
- `PropertyReadPlanCompiler`/`PropertyReadBuildCursor` are not the aggregate admitted Consumer session;
- `PropertyReadPlanCompiler::consumer_call` is excluded from the target engine path and may be deprecated/removed or retained only as a documented non-admitted legacy convenience during WP-200 migration; this compatibility choice does not create a publication bypass;
- `select_consumer_property_read` may remain a lower-level selector for legacy/test material; admitted consumed handles select only from their private Published record;
- Core `PlanId`, `PlanSetGeneration`, `BindingArtifact*`, `OutboundRequest` and binding traits remain public data/SPI. Possession is not Servient authority, and direct low-level binding invocation is outside Servient admission guarantees.

The no-bypass invariant is structural: an external value cannot be converted into a Published consumed generation without the private live Servient record, committed account, generation reservation and execution-owner pin.

## Authority migration if accepted

Migration must reconcile, not silently relocate, these owners:

- `docs/spec/planning.md`: keep `PLAN-SET-001` normative ownership; clarify specification ownership versus runtime crate ownership and add aggregate Consumer algorithm details;
- ADR-0008: reaffirm;
- Servient lifecycle architecture and `docs/architecture/30-compiled-plan-lifecycle.md`: preserve Servient runtime owner and add exact identity/account/failure semantics;
- `docs/state-machines.toml`: keep Servient compiled-plan-set record; clarify/rename `PlanningBuildOwner` if needed so Building->Frozen cannot be read as Planning-crate lifecycle ownership;
- `docs/spec/foundation.md` and resource schema: add typed semantic structural/work and Planning/compiler lifetime controls plus exact hierarchical reservation/reconcile semantics;
- TD authority: add bounded typed Basic/census/provenance contract;
- Core binding registration/compiler/identity contracts: freeze exact-one metadata selection inputs, same-registration execution retention and no-substitution facts;
- `docs/api-ownership.csv`: mark lower-level public values non-authoritative and record target aggregate Consumer entry ownership;
- WP-000/WP-100/WP-200/WP-300/WP-400 records/evidence according to accepted ADR-0013 impact;
- workspace/0062: reconcile or supersede its former split framing.

## ADR-0013 impact selected for decision review, not yet applied

Machine-readable status remains unchanged while DISCUSSING.

- ADR-0008: reaffirm.
- `WP-100-CONSUMER-CALL-VALUES-VALIDATOR`: reaffirm is the leading result unless migration changes its Core identity assumptions.
- TD bounded Consumer validation/provenance: new narrow predecessor contract/tranche required; broad deferred validation/cache/codec remains inactive.
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING`: affected; reopen is the leading result because its admitted Consumer proof is single-coordinate rather than aggregate. Producer/shared evidence must be explicitly reaffirmed/dispositioned.
- `WP-300-CONSUMER-PROPERTY-READ-BINDING`: affected. Existing call/response mechanics remain candidate reaffirmable evidence; registration/execution-pin/no-bypass source changes determine whether a scoped reopen is required.
- Producer WP-200/WP-300: require explicit disjoint/reaffirm evidence; no automatic reopen.
- WP-400 Consumer slice: not admitted, so nothing is reopened. Its future admission text must be rewritten to consume the migrated aggregate draft/private Servient authority.
- broad WP-400 remains inactive/incomplete.
- Producer Property Read gate remains Producer evidence unless exact migration diff touches its contract.

Actual reopen/reaffirm transitions occur only after independent acceptance and exact migration impact review.

## Stage-A constructibility evidence

Current replacement evidence consists of:

- `servient/tests/consumer_aggregate_registration_selection_stage_a.rs`: zero/one/multiple registration selection and profile eligibility;
- `servient/tests/consumer_aggregate_registration_pin_stage_a.rs`: same complete registration yields Planning identity and persistent execution pin; cross-wire rejection;
- `servient/tests/consumer_aggregate_admission_stage_a.rs`: identity-before-current-bounds, reservation-before-start, Pending input retention, paired/lifetime compiler work, zero-budget behavior and real compiler abort;
- `servient/tests/consumer_aggregate_material_reconcile_stage_a.rs`: real logical plans, artifact envelopes, refs, indexes and measured ledger retained through draft/reconcile; partial-success abort owns/releases prior completed material;
- `servient/tests/consumer_aggregate_identity_frozen_stage_a.rs`: aborted identity generation advances before reuse and successful freeze transfers persistent account + execution pin into Frozen ownership;
- `servient/tests/consumer_aggregate_admission_storage_stage_a.rs`: same semantic state graph in Host heap and caller-owned static storage without unsafe topology requirement;
- `servient/tests/consumer_aggregate_resource_projection_stage_a.rs`: every current resource row receives a first-proof disposition and review-sensitive families are pinned explicitly.

These fixtures are constructibility evidence only. They do not authorize production implementation or claim publication/Zenoh runtime completion.

## Rejected directions

1. Continue patching a single-PlanId Planning lease.
2. Use placeholder PlanIds for compiler bounds and substitute later.
3. Delay final PlanIds until after current bounds without changing SPI.
4. Change Core compiler SPI solely to preserve the original ordering.
5. Use registration order to resolve multiple Consumer registrations.
6. Probe bindings/forms to choose among multiple registrations in this first proof.
7. Let Servient rescan/reinterpret TD/Form state after Planning.
8. Let Planning own capacity reservation, publication or plan-set reclamation.
9. Treat lower-level public Rust constructors as admission capability.
10. Activate broad deferred Consumer validation/security/cache/fallback scope.
11. Immediately reopen every named package before exact ADR-0013 impact migration.

## Evidence required before DECIDED

One exact revision must establish:

1. normative/runtime ownership reconciliation with `PLAN-SET-001`, ADR-0008, lifecycle architecture and WP-400 dispositioned;
2. exactly-one registration selection with zero/multiple rejection and same-entry execution retention;
3. one Servient-owned transaction retaining identity/resource authority across Planning Pending without replacement inputs;
4. exact final identities before current compiler bounds, and generation invalidation on abort/reuse;
5. compiler bounds before start, resource reservation before build and failure-atomic bounded progress;
6. complete real aggregate material plus identity/index/ledger reconciliation;
7. partial-success abort settlement of already completed provisional material;
8. Frozen transfer of persistent account/capacity and execution pin rather than zeroing all tracked ownership;
9. exhaustive current resource/work applicability with typed structural migration selected;
10. selected public no-bypass semantics;
11. ADR-0013 impact map for completed and pending tranches;
12. safe Host/static storage constructibility; and
13. fresh independent architecture acceptance of the exact revision.

Production Zenoh behavior, final publication runtime, concurrent load measurements, real cleanup executor operation and WP-400 completion remain post-migration evidence.

## Candidate migration order after acceptance

1. migrate normative/runtime wording while keeping `PLAN-SET-001` in its registered Planning specification;
2. admit generic Foundation typed-work/lifetime/reservation changes required by the aggregate transaction;
3. admit the TD typed Basic/census/provenance predecessor;
4. migrate Core complete-registration selection/execution-pin contracts as required;
5. apply independently accepted ADR-0013 tranche reaffirm/reopen decisions;
6. migrate WP-200 Consumer Planning to aggregate enumerate/bound/build/sealed-draft semantics;
7. add only the Servient aggregate admission/identity/resource/storage skeleton admitted by the new WP-400 Consumer tranche;
8. reconcile/supersede 0062 around the final handoff;
9. fresh review of migrated authority before production implementation proceeds.

This order is a migration hypothesis, not implementation authorization.

## Decision question

Does v5.1 Consumer Property Read converge on one Servient-owned aggregate admission transaction in which TD provides bounded typed validated provenance, exactly one complete Consumer registration supplies both compiler and persistent execution ownership, Planning constructs one sealed real aggregate under final identities and admitted resources, Servient reconciles and retains the committed Frozen account/lifecycle authority, and only Servient publication can confer an admitted consumed handle?
