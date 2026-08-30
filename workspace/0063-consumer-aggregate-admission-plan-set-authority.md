# 0063 — Consumer aggregate admission and plan-set authority

Status: DISCUSSING

## Question

What is the smallest constructible ownership and admission model that can turn
one owned, validated Consumer Thing Description into the first immutable
Consumer Property Read plan set without weakening the accepted lifecycle,
binding, or resource authorities?

This topic replaces the earlier workspace/0063 proposal. It is a design
candidate for a later independent `DECIDED` review; it is not implementation
authority.

## Scope

This discussion is deliberately narrower than a general Consumer planner. It
covers the first v5.1 Consumer Property Read admission path:

- one owned TD entering one Servient-owned build transaction;
- deterministic enumeration of effective readable property forms;
- the already-admitted startup registration snapshot;
- one eager binding candidate and artifact per readable coordinate;
- one immutable aggregate published through the accepted plan-set lifecycle;
- host and application-static constructibility; and
- bounded cancellation, failure settlement, and reclamation.

It does not admit production Rust, start WP-400, change a work-package status,
define multi-binding policy, add runtime registration replacement, or migrate
any conclusion into `docs/`.

## Evidence basis

This revision was reconstructed from the current default branch and the exact
PR head, not from the previous proposal's conclusions. The controlling inputs
were:

- `AGENTS.md`, `PROJECT_GOVERNANCE.md`, `ARCHITECTURE_GOVERNANCE.md`, and
  `PLAN.md`;
- workspace/0061 and workspace/0062;
- ADR-0008, ADR-0015, and ADR-0019;
- the Planning, binding-SPI, compiled-plan-lifecycle, primary-flow, module, and
  Servient-lifecycle authorities;
- the admitted WP-200 and WP-300 Consumer evidence;
- the current `Thing`, planning, compiler, registration, resource, Servient,
  and static-destruction code; and
- every revision and currently available review finding on PR #57.

The previous design exposed four mismatches that should not be preserved:

1. It borrowed a `Thing` even though both current Consumer entry surfaces
   transfer an owned `Thing`.
2. It created an independent generation allocator for every plan although the
   accepted identity domain is the plan-set generation and plan IDs are only
   stable within that generation.
3. It invented a selected-registration execution pin although startup
   registrations already live in one immutable snapshot owned by the
   Servient/handle.
4. It allocated final owned plans before reserving their persistent storage.

The Stage-A evidence also disagreed with itself about identity, registration
lifetime, material shape, and cancellation borrowing. Contradictory fragments
are therefore removed rather than retained as historical constraints.

## Controlling conclusions

### 1. The input is transferred, not borrowed

The admission boundary receives a move-only `ValidatedConsumerThing`-equivalent
value. It owns:

- the exact `Thing` value accepted by Basic validation;
- the source-memory charge transferred with that value;
- a representation-neutral typed census used to reserve deterministic
  enumeration work and temporary storage; and
- validation provenance sufficient to prevent an unvalidated construction
  path inside the transaction.

The name above describes an ownership contract, not a proposed public type.
The host `Servient::consume(Thing)` convenience may perform validation and
create the value internally. A static builder may expose validation as an
incremental predecessor phase. In both representations, Planning only borrows
the validated value while deriving plans. No plan, compiler cursor, candidate,
artifact, index, or Frozen aggregate retains a TD lifetime.

The transaction releases the source TD and its source charge before it becomes
`Frozen`. This preserves the accepted rule that a compiled plan owns only the
small projection required at runtime.

Creating this value is not free work outside admission. The validation/census
predecessor must already own the source charge, temporary allowance, and exact
typed work budget, and it transfers the surviving source owner atomically into
aggregate construction. The Stage-A composite begins at that transfer point;
it does not claim that current monolithic `Thing::validate()` is itself a
bounded production implementation.

### 2. One transaction owns one plan-set identity domain

At build start the Servient reserves exactly one unpublished
`PlanSetGeneration`. Plan IDs are dense plan-set-local coordinates:

```text
PlanId {
    slot: dense logical-plan slot,
    generation: reserved PlanSetGeneration.get(),
}
```

The equality of those generation values is an invariant. A plan ID is not an
independently reusable global arena handle. The aggregate owns its plan vector,
so the slot has meaning only under the enclosing plan-set generation.

Abort before publication and reclaim after draining each advance the single
plan-set generation before the same storage can be reused. All plan IDs from
that set therefore become stale together. No per-slot generation allocator,
independent plan lease, or mixed-generation plan set is required.

Every artifact identity, compact artifact reference, runtime binding-plan
reference, target-index entry, and diagnostic coordinate carries or resolves
through this same plan-set generation and plan ID.

### 3. The immutable registration snapshot is the lifetime authority

The Servient captures one immutable startup registration snapshot before
planning. The snapshot contains complete registrations, including compiler and
client execution components. The Frozen aggregate retains one lease/owner for
that snapshot, not a separate borrowed pin for each selected registration.

A candidate records the admitted identity already defined by Core:

- binding ID and generation;
- configuration digest;
- artifact compatibility;
- registration-snapshot ordinal; and
- deterministic candidate order.

The ordinal is a lookup accelerator and diagnostic coordinate, not identity by
itself. Execution resolves the ordinal against the retained snapshot and
compares the complete identity before using the client component. The host
representation may implement the snapshot lease with shared immutable
ownership; the static representation may retain it structurally inside the
root object. Neither requires a self-reference or a new binding-SPI pin API.

### 4. Security is an explicit narrow admission predicate

The first proof does not yet have the complete effective-security projection
required by the general logical-plan architecture. It therefore admits only a
deterministic no-material NoSec shape: each readable Form's effective security
list contains exactly one definition name, that name resolves in the validated
TD, and the definition is `NoSec`.

Any auto, combo, credential-bearing, provider-dependent, multi-branch,
unresolved, or otherwise non-NoSec shape fails the whole unpublished aggregate
before plan materialization. It is not silently omitted and is not delegated to
the binding compiler. No security provider is probed and no applied security
material is retained for this proof.

This compatibility predicate is intentionally temporary. A later expansion
must freeze the accepted structured effective security expression in the
logical plan before admitting other schemes; it must not teach WP-400 to rescan
the TD.

### 5. Singleton Consumer selection is a first-proof compatibility rule

The first v5.1 aggregate proof has no admitted PLAN-INDEX or multi-binding
selection policy. For that proof only, the captured snapshot must contain
exactly one registration advertising Consumer Property Read. Producer-only or
otherwise ineligible registrations may coexist in the snapshot.

- zero eligible registrations fails before enumeration;
- one eligible registration is used for every first-proof coordinate; and
- more than one eligible registration fails as an unsupported/ambiguous
  first-proof shape.

This is not a permanent global tie-break rule and does not redefine Planning's
candidate taxonomy. Multiple candidates remain a later PLAN-INDEX design
problem. The diagnostic ordinal is preserved; eligible-list position must
never be substituted for the snapshot ordinal.

### 6. Reservation precedes the allocation or progress it admits

Admission uses staged reservations. A single speculative "reserve everything"
number is neither necessary nor sufficient.

1. **Source transfer.** Accept the validated owned TD together with its source
   charge and typed census.
2. **Enumeration reservation.** Reserve the existing applicable URI/security
   work, the future admitted structural/build work, and peak temporary storage
   before allocating coordinate projections. Current Foundation lacks an
   accurately named class for the structural/build portion, so production
   admission remains gated on that predecessor.
3. **Enumeration.** Walk properties in deterministic map order and forms in
   source order. Resolve operation and security defaults and targets, reject a
   readable coordinate outside the first-proof NoSec predicate, and retain
   coordinates whose effective operations contain `ReadProperty`. A target
   with zero readable forms is valid and receives an explicit empty projection.
4. **Shape reservation.** From the exact coordinate count and measured string
   lengths, reserve the plan-set slot and persistent ceilings for logical
   plans, candidates, target entries, artifact-reference entries, runtime
   joins, and bounded diagnostics before constructing any final plan.
5. **Single materialization.** Assign dense plan IDs and construct each final
   `LogicalInteractionPlan` exactly once. Candidate and target records refer to
   those final IDs. No placeholder plan or reconstruction is allowed.
6. **Compiler bounds.** Call `bounds` with a borrow of each exact final plan and
   its final candidate.
7. **Compiler reservation.** Reserve every declared artifact, cursor,
   temporary, and typed-work ceiling before the first `start` call.
8. **Compiler progress.** Drive bounded `start`/`step`, preserving cursor
   ownership. Admit measured artifacts into envelopes and retain completed
   earlier artifacts while later coordinates are built.
9. **Reconcile and seal.** Prove the complete one-to-one join, reconcile every
   measured amount against its reservation, release source and temporary
   accounts, then seal Planning's immutable draft.
10. **Freeze.** Commit the plan-set identity lease, resource lease, snapshot
    lease, and sealed material atomically into the Frozen aggregate.

Compiler `bounds`, `start`, and every `step` observe the same logical-plan
address and value later retained by the aggregate. This is a Stage-A proof
constraint, not a public address-stability promise after ordinary Rust moves.

### 7. The Frozen aggregate is the only published runtime owner

For the first Consumer Property Read slice, the aggregate contains:

- the plan-set generation;
- owned logical plans;
- one candidate for each plan;
- admitted artifact envelopes;
- compact artifact references;
- runtime binding-plan references joining plan, candidate, registration, and
  artifact identities;
- a target projection for every input property, including explicit empty
  projections;
- the reconciled persistent resource ledger;
- bounded diagnostic material; and
- the registration-snapshot lease.

Before publication the transaction proves:

- plan IDs are unique and dense within the set;
- every plan ID generation equals the set generation;
- every candidate, artifact identity, artifact reference, runtime join, and
  target entry resolves to exactly one plan;
- every candidate resolves to the retained registration snapshot by ordinal
  and complete identity;
- every readable coordinate has exactly one admitted artifact and runtime
  join;
- every target appears exactly once, even when its plan list is empty;
- no undeclared retained TD or compiler cursor remains; and
- measured persistent ownership fits its admitted ledger.

Publication exposes only the sealed aggregate. No partially built map or
compiler cursor becomes reachable through a Consumer handle.

### 8. Failure settlement is private Building state

The accepted public plan-set graph remains unchanged:

```text
Building -> Frozen -> Published -> Draining -> Reclaimed
Building -> Failed
Frozen   -> Failed
```

Validation, enumeration, reservation, materialization, compiling, sealing,
and failure settlement are private substates of `Building`. Terms such as
`Aborting` or `FailedSettled` may describe implementation phases, but they are
not additional observable plan-set states.

On failure or cancellation, the transaction records the first cause, stops
starting new compiler work, aborts every live caller-owned cursor exactly once,
drops completed unpublished artifacts, releases temporary and persistent
reservations, releases the TD and snapshot lease, invalidates the unpublished
plan-set generation, and only then reports authoritative `Failed`.

If cleanup itself reports a secondary defect, the first cause remains the
externally classified error and the cleanup defect is bounded diagnostic
material. No failed aggregate is published.

### 9. Cancellation follows the real owner topology

Application-static destruction already enters through `&mut self`. The static
root therefore records cancellation and its first cause directly in the live
admission transaction between bounded progress calls. The next progress call
checks cancellation before any compiler callback, and sealing checks again
immediately before committing Frozen ownership.

The host adapter may use an owned/shared cancellation signal because its task
driver has a different execution representation. The semantic rule is shared;
the borrowing mechanism is not. A long-lived immutable "cancellation view"
inside a mutably driven static root is neither required nor constructible.

## Ownership and reservation handshake

The proposal does not move Planning algorithms into Servient. The boundary is:

- TD validation owns creation of validation provenance and the typed census;
- Planning owns deterministic registration/candidate selection, form
  enumeration, default/security/target resolution, final plan construction,
  compiler driving, aggregate reconciliation, and sealed draft material;
- Core owns the immutable identity, plan, candidate, compiler, artifact, and
  error value contracts;
- the binding registration owns its compiler and execution components; and
- Servient owns the source transfer, snapshot lease, resource/identity leases,
  cancellation, public plan-set lifecycle, and atomic publication.

Constructibility requires a resumable two-barrier Planning protocol:

1. Planning borrows the validated TD and compiler-only registration projection,
   performs bounded enumeration, and suspends with opaque cursor ownership plus
   exact `AggregateShapeRequirements`-equivalent facts.
2. Servient either returns a shape-admission token after reserving identity and
   persistent aggregate capacity or resumes Planning with a terminal failure.
   Planning cannot materialize final plans without that token.
3. Planning materializes each final plan once, calls pure compiler `bounds` on
   those retained values, and suspends again with exact aggregate compiler
   requirements while retaining the same plans in its opaque cursor.
4. Servient reserves artifact, cursor, temporary, and work ceilings and returns
   a compiler-admission token. Planning cannot call `start` without it.
5. Planning drives bounded compiler progress and returns either an
   ownership-preserving failure cursor or an internally reconciled sealed draft
   plus exact measured ledger. Servient checks that ledger against its held
   reservations and commits the surrounding owners to Frozen.

The token names are explanatory, not proposed APIs. Their required property is
unforgeable phase ordering: ordinary public construction cannot resume across
either barrier without the matching Servient-owned reservation. Planning never
receives the client execution object, resource registry, publication map, or
Servient handle. Servient never reimplements TD traversal or reconstructs the
completed ledger.

## Ownership sketch

```text
caller-owned Thing
    |
    v  validate + transfer source charge/census
Servient Building transaction
    +-- unpublished PlanSetGeneration lease
    +-- owned validated Thing
    +-- immutable registration-snapshot lease
    +-- staged resource reservations
    +-- final logical plans / candidates / cursors / artifacts
    |
    +-- failure/cancel --> settle privately --> Failed
    |
    `-- reconcile + seal + commit
            |
            v
Frozen aggregate
    +-- sealed plans/candidates/artifacts/joins/targets
    +-- persistent resource ledger
    `-- registration-snapshot lease
```

## Public boundary and bypass prevention

The eventual public Consumer construction path must have one canonical
admission route. Existing convenience APIs may remain source-compatible only
by delegating to that route. They must not publish a legacy scan-backed
Consumer handle alongside the aggregate-backed handle.

This discussion does not choose final public method names. It does require
that validation provenance, snapshot capture, resource admission, compilation,
reconciliation, and publication cannot be skipped by a second constructor.

## Stage-A evidence boundary

The replacement evidence intentionally uses three fixtures:

1. `consumer_aggregate_admission_stage_a.rs` is the principal composite. It
   exercises real TD construction and Basic validation, owned input transfer,
   deterministic enumeration, first-proof registration scoping, plan-set-local
   identity, staged resource events, same-plan compiler observation, full
   material reconciliation, snapshot-based execution resolution, partial
   failure cleanup, and generation invalidation.
2. `consumer_aggregate_static_cancellation_stage_a.rs` isolates the actual
   `&mut self` static-owner topology and first-cause cancellation semantics.
3. `consumer_aggregate_resource_projection_stage_a.rs` checks the scoped
   resource-table projection against `docs/resource-limits.csv`.

The evidence is intentionally non-production. It proves that ownership and
phase ordering can be expressed with current public value contracts. It does
not pre-admit the final API, allocator layout, async executor, PLAN-INDEX, or
WP-400 code. It also exposes, rather than disguises, the missing typed-TD
census/provenance boundary and structural aggregate-build work class.

## Work-package impact hypothesis

This remains a proposal for independent review:

- ADR-0008's aggregate authority and lifecycle are reaffirmed.
- WP-100's value/response boundary is unaffected.
- TD validation/provenance and typed census are a likely predecessor change.
- WP-200 is affected at its leading edge: its current single-coordinate output
  is not the complete aggregate draft required here.
- WP-300's admitted compiler and complete registration identities appear
  sufficient; the prior execution-pin proposal is withdrawn. An exact-diff
  review is still required before deciding whether any WP-300 change is needed.
- Producer route planning is disjoint from this Consumer aggregate admission
  decision.
- WP-400 remains unadmitted and unstarted.

No work-package status changes in this discussion.

## Rejected alternatives

- **Borrow the caller's TD through build.** Contradicts current ownership
  surfaces and complicates static storage and cancellation.
- **Clone the TD for admission.** Duplicates source memory and weakens transfer
  accounting without providing a runtime need.
- **Independent PlanId generations.** Creates a second lifecycle domain with no
  accepted stale-handle requirement inside an immutable aggregate.
- **Per-registration execution pins.** Duplicates the lifetime already provided
  by the immutable snapshot and pressures static representations toward
  self-reference.
- **Pick the first eligible registration permanently.** Silently converts a
  missing PLAN-INDEX policy into registration-order semantics.
- **Build plans before reservation.** Makes persistent allocation precede its
  admission authority.
- **Reconstruct plans after bounds.** Breaks the exact-value constructibility
  requirement and can make compiler evidence refer to a different value.
- **Expose cleanup phases as plan-set states.** Contradicts the accepted public
  lifecycle instead of implementing it.

## Proposed migration order after a DECIDED review

1. Migrate the accepted ownership, identity, phase, and aggregate invariants to
   their single authoritative documents.
2. Admit the TD validation/provenance/census predecessor, if the review confirms
   it is necessary.
3. Reconcile WP-200 and WP-300 exact diffs against the accepted boundary.
4. Define and admit the aggregate construction work package.
5. Only then admit WP-400 runtime implementation.

## Decision request

A later independent review should try to falsify, at minimum:

- owned input transfer and source release before Frozen;
- the shared plan-set/PlanId generation invariant;
- whole-snapshot lifetime ownership and exact execution resolution;
- explicit rejection outside the temporary no-material NoSec predicate;
- the deliberately temporary singleton Consumer-registration rule;
- reservation-before-allocation/progress at both stage boundaries;
- the complete aggregate join and zero-readable-target representation;
- private failure settlement under the accepted public lifecycle; and
- constructible static cancellation without self-reference.

Until that review reaches a stable conclusion, workspace/0063 remains
`DISCUSSING`.
