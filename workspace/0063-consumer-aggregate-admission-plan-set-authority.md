# 0063 Consumer Aggregate Admission and Plan-Set Build Authority

Status: DISCUSSING

Kind: architecture reconciliation and replacement proposal

Priority: HIGH

Target: freeze the smallest constructible v5.1 Consumer Property Read admission boundary in which one Servient-owned aggregate transaction validates one borrowed typed `Thing`, selects one complete Consumer registration, constructs one complete immutable Property Read plan set, retains the exact execution owner, and reaches unpublished `Frozen` without giving Planning lifecycle/publication authority.

This topic does not change active authority, reopen a completed tranche, admit production Rust, register the Consumer architecture gate, or unblock WP-400.

## Decision boundary

The candidate authority unit is one complete consumed Property Read plan-set generation, not one property/Form plan.

```text
ConsumerAdmissionTxn                         runtime owner: Servient
  Captured
    -> Validating                            TD bounded Basic + typed provenance
    -> SelectingRegistration                 exactly one eligible complete registration
    -> Enumerating                           Planning target/coordinate projection
    -> AssigningIdentities                   Servient plan-set + independent plan-slot identities
    -> Bounding                              Planning + exact compiler bounds
    -> ReservingResources                    Servient atomic local/global reservation
    -> Building                              Planning + binding compiler progress
    -> Reconciling                           Servient exact join/ledger validation
    -> Frozen                                complete unpublished aggregate + committed owners

  any terminal failure/cancel
    -> Aborting                              first terminal cause immutable
    -> FailedSettled                         all unpublished owners settled
```

`Frozen -> Published` remains later Servient/WP-400 work. Stage-C protocol/runtime/load evidence is not required merely to decide this pre-publication architecture.

## Normative and runtime ownership

`docs/spec/planning.md` remains the normative owner of `PLAN-SET-001`. ADR-0008 remains accepted. Neither fact means that the Planning crate owns the runtime lifecycle record.

The runtime split is:

- TD owns typed validation semantics/provenance and pure TD defaults;
- Planning owns deterministic target/Form interpretation, aggregate enumeration, logical-plan construction, compiler-bounds coordination, build progress, candidate/index material, and sealed immutable draft material;
- Core owns immutable plan/candidate/artifact/compiler/execution identity contracts;
- Servient owns the admission transaction, startup registration snapshot, identity allocation, resource reservations, persistent execution pin, reconciliation, Frozen/Published record, cancellation, draining, cleanup, and reclamation authority;
- Foundation owns vocabulary-neutral generation, resource, reservation, and work primitives.

A Planning algorithm cursor may physically live inside the Servient-owned Building record. That does not transfer lifecycle authority to Planning.

## 1. First-proof registration selection

Before target enumeration, the transaction selects exactly one eligible complete Consumer Property Read registration from the immutable startup snapshot.

Eligibility is metadata-only:

1. the complete registration already passed Core registration validation;
2. it advertises Consumer Property Read capability; and
3. it contains the execution half for the active Host or application-static profile.

Selection invokes no binding callback, support probe, wildcard probe, credential provider, protocol I/O, or Form-specific filter.

Outcomes are fixed:

- zero eligible registrations -> terminal no-eligible-registration admission error;
- exactly one -> capture that exact snapshot entry for the complete aggregate;
- more than one -> terminal ambiguous-registration admission error.

Registration order never resolves ambiguity. Later bounds/build failure never causes registration reselection.

The exact selected entry yields both:

- the Planning/compiler projection used by every mandatory coordinate; and
- the persistent generation-checked execution pin retained by Frozen/Published material.

The registration snapshot ordinal is the actual snapshot coordinate. `BindingRegistrationIdentity::diagnostic_ordinal()` is diagnostic metadata only. They may differ and are never interchangeable.

## 2. Aggregate target and coordinate semantics

Planning traverses `Thing::properties` in deterministic `BTreeMap` key order and preserves each property's Form array index/order.

Every effective first-proof `ReadProperty` coordinate is mandatory eager work. One terminal coordinate error fails the whole unpublished aggregate. There is no skip, lazy negative, implicit next Form, fallback, or per-coordinate binding reselection.

Planning must also retain a target-operation projection for every declared property, including a property with zero readable Property Read Forms. Therefore after the TD borrow is released:

- `lookup("declared-but-unreadable")` returns an existing target entry with an empty binding-plan-reference range;
- `lookup("absent")` returns no target entry.

Servient never rescans the TD to recover this distinction.

The first security proof remains deterministic no-material NoSec only. Any security shape requiring credential/provider access, branch choice, applied-security material, or binding-carried security state is rejected rather than silently skipped.

## 3. Plan-set generation and PlanId generation are independent authority domains

`PlanSetGeneration` identifies the aggregate plan-set generation. `PlanId` contains a dense plan slot plus that **plan slot's own `Generation`**. They have different lifecycle roles and no numeric equality invariant.

The selected allocation rules are:

1. Servient owns one unpublished plan-set slot generation and a bounded set of reusable plan slots;
2. aggregate enumeration determines the exact mandatory readable-coordinate count before plan identities are allocated;
3. Servient reserves one exact `PlanSetGeneration` and one exact `PlanId` per mandatory coordinate;
4. each `PlanId` uses the current non-wrapping generation of its own reusable plan slot;
5. `PlanId::generation()` is not copied from, compared for equality with, or derived from `PlanSetGeneration::get()`;
6. the exact `PlanId` assigned to a coordinate is immutable across logical-plan construction, compiler `bounds`, compiler `start/step`, artifact identity, aggregate reconciliation, Frozen storage, and later lookup;
7. on unpublished abort/failure, the plan-set slot generation and every reserved plan-slot generation advance independently before the same storage slots may be reused;
8. on successful Frozen, both domains remain pinned by the Frozen owner and may advance only when the generation is later reclaimed/reused;
9. generation exhaustion is terminal capacity failure; neither domain wraps.

A dense coordinate ordinal may choose a plan slot but never supplies its generation.

## 4. Identity assignment precedes compiler bounds

Current `BindingCompilerExtension::bounds` accepts `BindingCompilerInput`, and that input exposes the full `LogicalInteractionPlan`, including its final `PlanId`.

Therefore the exact sequence is:

1. bounded target/coordinate enumeration with no compiler start;
2. Servient assigns the exact unpublished `PlanSetGeneration` plus independent per-slot `PlanId`s;
3. Planning constructs each final owned `LogicalInteractionPlan` exactly once;
4. the same owned plan values are borrowed by compiler `bounds`;
5. all compiler bounds are collected before any compiler `start`;
6. Servient atomically reserves the aggregate resource bundle;
7. those same owned plan values move into Building and are borrowed by compiler `start/step`;
8. completed plans move directly into provisional aggregate material; they are not reconstructed from identity fields.

Placeholder PlanIds or bounds-time/build-time plan reconstruction are rejected.

## 5. Compiler and work admission

For every mandatory coordinate the transaction captures all of one `BindingCompilerBounds` result:

- admitted artifact footprint;
- cursor bytes;
- temporary bytes; and
- compiler lifetime `WorkBudget`.

The aggregate bounds are known before compiler `start`.

Compiler lifetime work is not replenishable by later caller step budgets. When the existing compiler SPI receives one `&mut WorkBudget`, the enclosing admission may create a child budget only by atomically partitioning both:

- the remaining aggregate/compiler lifetime allowance; and
- the current caller step allowance.

Unused child capacity is reconciled to both parents. If either allowance cannot supply work, no compiler callback occurs.

The first proof also requires a separate bounded lifetime allowance for typed validation and Planning enumeration/index/reconciliation work. Foundation work names remain vocabulary-neutral; TD-specific vocabulary is not moved into Foundation.

## 6. Sealed aggregate material

A successful Planning build returns one sealed, non-authoritative aggregate draft. Before Frozen it is private provisional material owned by the Servient transaction.

The draft contains at minimum:

- every final owned `LogicalInteractionPlan`;
- every retained `BindingCandidate`, including actual registration snapshot ordinal and deterministic `candidate_order`;
- every admitted `BindingArtifactEnvelope`;
- every `BindingArtifactRef`;
- one compact `BindingPlanRef`-equivalent join containing logical-plan slot, candidate slot, and artifact slot;
- the immutable target-operation projection, including zero-length ranges for declared targets with no readable Form;
- immutable first-proof diagnostics needed to explain admission/selection; and
- the exact measured aggregate ledger.

The runtime join is mandatory. Artifact identity alone is not sufficient because it does not preserve registration ordinal or candidate order.

For every binding-plan reference, reconciliation proves:

- the referenced logical plan's `PlanId` equals the artifact identity plan id;
- the artifact identity plan-set generation equals the aggregate generation;
- binding id/generation/configuration/compatibility equal the referenced candidate;
- candidate registration ordinal/candidate order equal the retained immutable candidate record;
- artifact reference identity and artifact slot equal the referenced envelope;
- the candidate still matches the exact selected persistent execution pin; and
- every target projection range lies within the binding-plan-reference table.

No target/index/join field is reconstructed later by Servient from the TD.

## 7. Resource reservation, measured reconciliation, and Frozen ownership

The aggregate flow is `enumerate -> identify -> bound -> reserve -> build -> reconcile`.

Before compiler `start`, Servient atomically reserves all applicable local plus parent/global capacity for:

- phase-local validation/Planning/compiler temporary storage;
- compiler cursor capacity;
- persistent logical-plan/candidate/artifact/ref/join/index/diagnostic material;
- compiled-runtime capacity;
- cleanup/abort settlement capacity;
- peak live bytes and largest contiguous allocation; and
- admitted lifetime work ceilings.

Planning builds only inside those ceilings.

At reconciliation the transaction measures the actual final aggregate from the real retained values. Every measured component must be `<=` its retained reservation. Identity/join validation and ledger validation happen before Frozen.

Successful Frozen performs an ownership transfer rather than "settling everything to zero":

- phase-local temporary/cursor reservation is released;
- unused persistent reservation is released;
- exact measured persistent plan/candidate/artifact/ref/join/index/diagnostic/runtime capacity transfers into a **Frozen committed resource account**;
- the exact persistent execution pin transfers into the Frozen owner;
- the exact plan-set and PlanId generation owners remain pinned by Frozen;
- persistent committed accounting reaches zero only during later reclamation.

On pre-Frozen failure, provisional completed coordinates remain owned by Aborting until their memory/accounting is released. A failure after one coordinate completed may not drop that material outside the transaction.

## 8. Failure, compiler abort, and first cause

Pending is the same move-only transaction/session, not a cursor that can be paired with fresh source, policy, registration, PlanId, PlanSetGeneration, compiler, or resource owner.

The first terminal cause recorded by the transaction is immutable.

If a compiler cursor is live when abort begins, the exact selected compiler receives `abort(cursor)` exactly once before that cursor is discarded. Pure compiler abort has no protocol cleanup obligation, but the outer admission still charges bounded settlement work and retains all provisional material/resource/identity owners until settlement completes.

Zero applicable step budget performs no semantic callback.

## 9. Application-static cancellation boundary

The portable static profile has no Host shutdown handle. Its selected pre-publication cancellation owner is the existing caller-driven `StaticServient::begin_destroy()` lifecycle request.

The contract is:

1. a new admission may start only while the owning StaticServient is Active;
2. the admission captures a read-only cancellation view of that exact StaticServient owner/request generation; it does not own or clone lifecycle authority;
3. `begin_destroy()` transitions the owner to destroy-requested and advances/records its destroy request generation idempotently;
4. every caller-driven admission progress step checks that view before invoking TD/Planning/compiler semantic callbacks;
5. the view is checked again immediately before the unpublished Frozen transition;
6. if destroy was requested between static progress steps, cancellation linearizes before the next callback;
7. if a semantic terminal failure already linearized first, a later destroy request does not replace that first cause;
8. once the transaction enters Aborting, later cancellation/failure observations do not replace its first cause;
9. a static admission cannot start after destroy is already requested.

This is the application-static mapping required by 0062. Host cancellation may use the Host lifecycle authority, but both profiles feed the same profile-neutral `Aborting -> FailedSettled` semantic state.

## 10. Public no-bypass boundary

Only Servient may confer admitted Consumer plan-set publication/handle authority.

Lower-level public Planning/Core values may remain usable as algorithm/data/SPI surfaces, but no safe public Servient operation accepts externally assembled `PlanBuildOutput`, logical plans, artifacts, raw `PlanId`/`PlanSetGeneration`, candidate records, binding-plan refs, or execution pins as proof of admission.

The admitted Consumer path begins from ordinary TD/policy/profile input and drives the private Servient transaction. Frozen/publish/install operations operate only on that live private record.

`PropertyReadPlanCompiler::consumer_call` and other one-coordinate Consumer helpers therefore cannot remain a second admitted engine path. Reopened WP-200 may remove/deprecate them or retain them as explicitly non-admitted lower-level APIs, but neither choice may feed Servient publication.

Producer/shared APIs are changed only when an explicit impact review proves they are affected.

## 11. Resource-schema direction

Direct borrowed typed `Thing` input does not silently inherit Raw-JSON lexical/structural limits.

The selected migration direction is an additive representation-neutral typed structural family for the typed admission projection. Existing Raw-JSON `json_*`, `string_bytes_max`, and `extension_bytes_max` rows keep their current semantics unless separately migrated.

`workspace/0063-resource-work-applicability.md` classifies every current registered resource row as one of:

- `Active`;
- `ZeroContribution`;
- `Deferred`; or
- `NotApplicable`.

The accompanying coverage fixture must fail when the registered schema gains an unclassified row. Production measurements remain later completion evidence.

## 12. Authority and work-package impact

The selected pre-migration impact direction is:

- ADR-0008: reaffirm;
- `docs/spec/planning.md` / `PLAN-SET-001`: retain normative ownership; clarify normative-vs-runtime wording and aggregate Consumer material semantics;
- Servient lifecycle architecture: retain runtime ownership; add exact Consumer admission/cancellation/Frozen ownership detail;
- WP-000: affected only to the extent generic work/reservation primitives require source/public changes; exact reopen vs successor is decided from migration diff;
- `WP-100-CONSUMER-CALL-VALUES-VALIDATOR`: reaffirm is the leading result;
- TD: add one narrow bounded typed validation/provenance predecessor without activating broad deferred validation/cache/codec scope;
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING`: affected; reopen is the leading result because the admitted Consumer proof becomes aggregate and retains candidate/join/index material;
- Producer WP-200: explicit disjoint/reaffirm evidence required;
- `WP-300-CONSUMER-PROPERTY-READ-BINDING`: affected by same-registration persistent execution pin/no-bypass source; valid call/response mechanics remain candidate-reaffirmable;
- Producer WP-300: explicit disjoint/reaffirm evidence required;
- WP-400 Consumer: not admitted, therefore not reopened; its future tranche consumes the migrated aggregate draft plus private Servient owners;
- broad WP-400 remains inactive/incomplete.

ADR-0013 status transitions are applied only after independent acceptance and exact migration impact review.

## 13. Relationship to 0062

0062 remains correct about the missing aggregate handoff and the following still-local requirements:

- deterministic PlanId slot **and independent generation** authority;
- static cancellation mapping through `begin_destroy()`;
- candidate/`BindingPlanRef` and target-index retention;
- exact preflight/build identity continuity;
- exact completed ledger reconciliation; and
- persistent execution-owner retention.

This 0063 candidate now incorporates those facts because the from-first-principles redesign showed that they are inseparable parts of the same Servient-owned aggregate admission lifecycle. If this topic is accepted/migrated, 0062 should be reconciled to the surviving sealed-draft handoff or marked superseded if no separate local claim remains.

Closed PR #56 remains history only and is not candidate authority.

## Stage-A evidence required before DECIDED

The exact reviewed revision must demonstrate all of the following without production implementation authority:

1. normative Planning requirement ownership remains distinct from Servient runtime lifecycle ownership;
2. exactly-one complete registration selection with zero/multiple rejection and no probe/reselection path;
3. one owned logical-plan value per coordinate survives identity assignment -> bounds -> reservation -> start/step -> aggregate material -> Frozen;
4. full aggregate material retains logical plans, candidates, artifact envelopes/refs, binding-plan joins, target-operation projection, and exact measured ledger;
5. declared-but-unreadable target remains distinguishable from absent target after source release;
6. PlanSetGeneration and PlanId generation use independent allocators/lifecycle rules and both advance safely on aborted slot reuse;
7. same selected complete registration supplies compiler identity and persistent execution pin;
8. aggregate reconcile validates identity/join invariants and measured `<=` reserved;
9. partial-success abort retains and settles already completed provisional material;
10. successful Frozen transfers persistent accounting, generation ownership, and execution pin rather than clearing them;
11. Host and caller-owned application-static storage can preserve the same semantic state graph;
12. application-static `begin_destroy()` maps to admission cancellation with callback-before-check, pre-Frozen check, and immutable first-cause rules;
13. every current resource row has an explicit applicability classification; and
14. a fresh independent review finds no unresolved architecture/public-API/ownership/resource contradiction in this claim.

Production Zenoh behavior, final Published runtime, concurrent load measurement, real cleanup executor operation, and Consumer WP-400 completion remain later work.

## Migration order after acceptance

1. reconcile normative Planning wording and Servient lifecycle/state-machine wording without moving `PLAN-SET-001` requirement ownership;
2. freeze Foundation vocabulary-neutral typed/lifetime work and reservation primitives required by the accepted transaction;
3. freeze TD bounded typed Basic/census/provenance contract;
4. freeze Core complete-registration Planning projection plus persistent execution-pin and independent plan-slot generation rules;
5. apply accepted ADR-0013 reaffirm/reopen/successor transitions;
6. reopen/migrate the Consumer WP-200 aggregate Planning surface and retained candidate/join/index material;
7. migrate only the Servient admission/identity/resource/cancellation/Frozen skeleton needed by the accepted boundary;
8. reconcile/supersede 0062;
9. independently review migrated authority/admission before production Consumer WP-400 implementation proceeds.

## Decision question

Does v5.1 Consumer Property Read converge on one Servient-owned aggregate admission transaction in which TD supplies bounded validated provenance, exactly one complete Consumer registration supplies both compiler and persistent execution ownership, Servient allocates independent plan-set and plan-slot generations before current compiler bounds, Planning moves the same owned plans through bounded aggregate build into complete candidate/join/index material, Servient reconciles and commits the exact persistent Frozen account, and application-static `begin_destroy()` drives the same pre-publication cancellation/abort semantics—while Published execution remains a later WP-400 tranche?
