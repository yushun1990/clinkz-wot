# 0062 Consumer Plan-Set Handoff Closure

Status: DISCUSSING

Kind: implementation-discovered architecture handoff investigation

Priority: HIGH

Target: the missing WP-200 -> WP-400 handoff needed to construct and publish the v5.1 Consumer Property Read plan set without Servient-owned TD interpretation

## Scope

This topic records a constructibility defect discovered after completion of
`WP-300-CONSUMER-PROPERTY-READ-BINDING` and before admission of the corresponding
WP-400 Consumer tranche.

It does not change active v5.1 authority, admit Rust source, reopen or reaffirm a
completed tranche, register the Consumer architecture gate, or unblock WP-400.

The original question was narrow: what Planning-owned aggregate value lets
`Servient::consume(td)` publish a complete Consumer Property Read plan set when
the completed WP-200 implementation compiles only one exact
`(property_name, property-form index)` coordinate at a time?

Five fresh independent reviews have now shown that the missing handoff cannot be
closed only by adding an aggregate container. It intersects pre-existing
admission, validation, registration-ownership, resource-accounting, identity,
and profile-progress contracts.

Under the migrated autonomous review-cycle rule in `workspace/0054`, an
upstream correction remains in the same engineering claim only when it shares
the same ownership, lifecycle, rollback, and validation boundary. The remaining
blockers no longer satisfy that condition. This topic therefore stops at the
stable finding below instead of absorbing additional cross-domain design.

## Established defect

The completed Consumer tranches do not currently compose into the required
public Servient path.

The repository currently has all of these facts at once:

1. `PLAN-SET-001` requires one immutable consumed plan-set generation to exist
   before `consume` returns a handle.
2. Planning owns effective TD interpretation, target/Form enumeration,
   candidate construction, target lookup, plan construction, and immutable set
   material. Servient must not implement a second planner.
3. The completed WP-200 Consumer compiler accepts one exact Property Read
   coordinate and returns only logical plans, artifact envelopes, and
   `BindingArtifactRef`s.
4. The completed WP-300 tranche provides selected Consumer binding execution
   after immutable selection.
5. Current `Servient::consume(td)` accepts an ordinary `Thing`, builds the legacy
   `ConsumedThing`, and has no exact Planning coordinate or aggregate build
   transaction.
6. The legacy `ConsumedThingHandle::read_property` still rescans TD/Form state at
   call time.

A conforming WP-400 implementation therefore cannot legally bridge the gap by:

- scanning or interpreting TD/Form state in Servient;
- choosing one arbitrary/first readable coordinate;
- changing public `consume(td)` into a one-property or one-Form API;
- re-entering legacy `ConsumedThing`, `BindingRequest`, or support-probe paths;
- publishing a partial aggregate;
- silently skipping a failed coordinate or choosing another Form/registration;
- dropping generation/registration ownership needed by later execution; or
- inventing uncharged/unreserved admission work to make the aggregate fit.

The ownership direction remains established: the missing set construction is a
Planning predecessor to WP-400, while Servient owns reservation, plan-set
lifecycle, publication, leases, drain, and reclamation.

## Constraints established by the review sequence

Although closure has not been accepted, the review sequence established several
constraints that any later solution must preserve.

### Deterministic target semantics

- `Thing::properties` is a `BTreeMap`; property traversal therefore uses
  deterministic key order, not original JSON insertion order.
- Forms retain their array order/index.
- Property name determines the target before Form selection.
- Omitted `form_index` may select only the addressed property's first retained
  applicable plan.
- Explicit `form_index` may examine only that addressed property's plan range.
- A hot lookup must use a prebuilt target-operation projection and must not scan
  unrelated targets.
- A declared property with no effective Property Read Form must remain
  distinguishable from an absent property after TD build inputs are gone.

### Aggregate and candidate semantics

- Every admitted effective first-proof Property Read coordinate is mandatory
  eager work.
- One terminal coordinate failure fails the whole unpublished aggregate; there
  is no skip, lazy negative, fallback, or implicit next-Form selection.
- First proof uses one globally selected Consumer-capable registration. The
  registration decision is not a Form-specific filter.
- The selected registration's actual snapshot ordinal and `candidate_order = 0`
  must survive into immutable set material.
- The immutable aggregate needs candidate records and a
  `BindingPlanRef`-equivalent plan/candidate/artifact join; artifact identity
  alone does not preserve registration ordinal or candidate order.
- Host-erased and application-static cells must preserve the same protocol-neutral
  target, candidate, identity, failure, and accounting semantics even when their
  physical storage differs.

### Admission and lifecycle semantics

- `ADMIT-TXN-001` requires work/temporary charging, then persistent reservation,
  then private persistent build, then one atomic publication transition.
- Planning progress is resumable. Budget exhaustion returns owned pending state;
  terminal failure preserves cleanup ownership until abort/drop handling.
- Cancellation is checked at bounded intervals and immediately before
  publication.
- `Building + cancel -> Failed` and `Frozen + cancel -> Failed` are unpublished
  state-machine transitions.
- A failed/cancelled transaction releases unpublished reservations and private
  state idempotently.

### Security first-proof direction

The accepted implementation direction remains deliberately narrow: the first
Consumer aggregate proof must require a deterministic no-material NoSec case
and reject security shapes that require provider access, credentials,
`AppliedSecurity`, branch selection, or binding-carried security state rather
than silently skipping them.

The exact executable security predicate cannot become authoritative until the
validated-input boundary below is resolved.

### Resource and work semantics

- Planning must provide pre-reservation bounds and an exact completed ledger;
  WP-400 must not reconstruct either by rescanning/recounting/re-measuring.
- Every bounded collection walk, target expansion, URI/security operation,
  compiler progression, index construction, reconciliation step, and cleanup
  operation must be charged before work starts under an accurately named
  `WorkClass`.
- Current Foundation has no obviously correct pure Planning/registration/index
  work class; relabeling such work as `BindingPolls`, `CleanupItems`, or
  `JsonSchemaNodes` without authority review is rejected.

## Independent review history

All five reviews returned `REQUEST CHANGES` for closure acceptance while
preserving the Planning-owned predecessor diagnosis.

### Review 1

Identified missing atomic compile/failure semantics, security scope,
deterministic identities, zero-plan diagnostics, exact accounting, deterministic
property/Form order, and completed-tranche impact review.

### Review 2

Identified reserve-before-build ordering, executable security admission,
resumable cursor/failure ownership, prebuilt target lookup, and registration /
candidate ordinal gaps.

### Review 3

Identified missing admission cancellation semantics and an ambiguous
"registration boundary" that could illegally filter individual Forms.

### Review 4

Identified an assumed rather than constructible pre-publication cancellation
source, an underspecified NoSec predicate, loss of immutable candidate /
registration projection, and missing phase-by-phase work charging.

### Review 5

The fifth review moved beyond the local aggregate boundary and identified five
remaining constructibility blockers:

1. **Validated-input provenance.** `consume(td)` accepts an ordinary mutable
   `Thing`, and `PlanBuildInput::new` merely labels a borrowed `Thing` as
   validated. The repository does not yet define who validates Consumer input,
   the required validation level/proof, or how validation traversal is bounded
   and charged.
2. **Execution-owner retention.** Candidate identity plus registration ordinal
   is insufficient once the execution-owning registration/startup snapshot is
   destroyed. The consumed handle/plan-set generation needs a persistent,
   generation-checked owner capable of reaching the selected Host client or
   static `ClientRequestSlot` execution half.
3. **Static admission cancellation.** Host has a Servient shutdown authority,
   while the portable static surface uses caller-owned manual progress and
   `begin_destroy()`. The exact static request/owner mapping for pre-publication
   Consumer cancellation has not been frozen.
4. **Admission-memory accounts.** A single "persistent ceiling" is insufficient.
   `ADMIT-MEM-001` requires distinct source/input, phase-local temporary,
   persistent document-retention, persistent compiled-runtime, diagnostic, and
   cleanup accounts, plus live peak and largest-contiguous-allocation evidence.
5. **PlanId generation authority.** A dense ordinal determines a PlanId slot but
   not its non-wrapping `Generation`. `PlanId` generation is distinct in type
   and lifecycle role from `PlanSetGeneration`; allocation and equality between
   preflight/build/publication must be frozen explicitly.

These are semantic blockers. Green CI or `git diff --check` does not close them.

## Cross-domain boundary reached

The fifth review establishes that continuing to elaborate one monolithic 0062
candidate would cross materially different ownership and validation boundaries.
That is now rejected as the progression strategy.

Two upstream independently reviewable claims must be resolved before 0062 can
return to local closure work.

### Next distinct claim A: bounded validated Consumer admission input

This claim should determine the smallest legal TD/Foundation/Servient admission
boundary for an ordinary `Thing` entering Consumer planning. It must resolve at
least:

- validation ownership and exact validation level/proof;
- whether validation is incremental or safely admitted as bounded
  non-incremental work;
- source/input bytes and lifetime;
- validation/effective-view temporary storage;
- phase-local work classes and charge units;
- `ADMIT-MEM-001` account separation, overlap, peak, and contiguous allocation;
- the relationship between validation output and Planning's current
  `PlanBuildInput` contract; and
- ADR-0013 impact on TD, Foundation, the existing WP-200 exact child, and any
  already-completed evidence relying on an informal "validated TD" premise.

No aggregate source admission should precede this claim.

### Next distinct claim B: Consumer execution registration pinning

This claim should determine the smallest Core/Servient ownership model that
lets immutable candidate references reach the correct execution half for the
entire consumed plan-set generation. It must resolve at least:

- what persistent registration/startup snapshot owner is pinned by a consumed
  generation;
- Host-erased and application-static ownership representations;
- generation-checked registration ordinal/slot lookup;
- identity/configuration/compatibility checks between candidate, artifact, and
  execution owner;
- which temporary Planning snapshot/view may be dropped after freeze;
- which execution-owning registration state must remain pinned; and
- ADR-0013 impact on WP-200/WP-300 and existing complete-registration APIs.

This is not a reason to give a binding the TD or to restore bare legacy binding
arrays to the hot path.

The ordering between claims A and B may be reviewed independently, but 0062
cannot become `DECIDED` until both are resolved or replaced by a smaller
accepted equivalent.

## Local 0062 work after the upstream claims

When the upstream claims are stable, 0062 may reopen one narrow closure review
for the remaining Planning -> Servient handoff. That review must freeze only the
still-local facts, including:

- exact aggregate preflight/build handoff using the accepted validated input and
  resource accounts;
- deterministic PlanId **slot and generation** allocation authority;
- Host cancellation mapping and the portable static cancellation request
  (likely evaluated against the existing manual `begin_destroy` boundary rather
  than assuming a Host `ShutdownHandle` exists in static code);
- candidate/`BindingPlanRef` and target-index retention;
- exact preflight vs build bound equality/ceiling rules;
- exact completed ledger reconciliation; and
- final ADR-0013 dispositions for WP-200, WP-300, Foundation/TD changes, and the
  future WP-400 Consumer admission.

Only after that narrow closure is independently accepted may the topic become
`DECIDED` and migrate to authoritative owners.

## WP-400 consequence

WP-400 Consumer Property Read remains blocked.

The intended high-level composition is still:

```text
ordinary Consumer input
  -> bounded validated admission input
  -> Planning-owned aggregate preflight/build
  -> Servient-owned reservation + lifecycle publication
  -> consumed generation pins immutable plans and execution registration owner

read_property(name, options)
  -> indexed immutable target lookup
  -> strict candidate/plan/artifact reference
  -> generation-checked execution registration owner
  -> OutboundRequest::property_read(...)
  -> selected Host/static binding execution
  -> response validation and settlement
```

No step authorizes Servient TD interpretation, call-time TD/Form scans, hidden
fallback, or unbounded/unaccounted admission work.

## Merge and migration condition

This document may be squash-merged while `DISCUSSING` as the durable record of
the discovered Consumer handoff defect, the five independent review rounds,
and the cross-domain stopping boundary.

Such a merge:

- does **not** accept a closure candidate;
- does **not** make 0062 `DECIDED` or `MIGRATED`;
- does **not** admit Foundation, TD, Planning, Core, Servient, or WP-400 source;
- does **not** reopen/reaffirm WP-200 or WP-300 by itself; and
- does **not** unblock the Consumer architecture gate or production Zenoh.

After merge, the next autonomous review cycle should take exactly one of the two
upstream claims above as its engineering claim. It should not start by running a
sixth closure review of 0062 against prerequisites that are already known to be
missing.
