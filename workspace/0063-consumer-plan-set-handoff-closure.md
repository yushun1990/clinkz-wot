# 0063 Consumer Plan-Set Handoff Closure

Status: DISCUSSING

Kind: preserved downstream architecture handoff investigation

Priority: HIGH after Consumer binding result sealing

Target: the missing WP-200 -> WP-400 aggregate handoff needed to construct and
publish one v5.1 Consumer Property Read plan set without Servient-owned TD
interpretation

## Scope and relationship to 0062

This topic preserves the aggregate investigation previously stored under 0062.
Independent review of PR #58 confirmed that Core-mediated result sealing is a
smaller predecessor with a different ownership and validation boundary. That
correction now remains in
`workspace/0062-consumer-binding-result-sealing.md`; moving it did not resolve
or invalidate the aggregate questions below.

This document deliberately separates repository facts from hypotheses produced
by prior investigations. It does not assume that closed PRs #56/#57, five prior
review rounds, their proposed aggregate containers, or their proposed ordering
of subclaims are correct. It retains the unresolved evidence so a later task
can reconstruct rather than rediscover it.

While `DISCUSSING`, this topic does not change active v5.1 authority, admit
Foundation/TD/Planning/Core/Servient source, reaffirm or reopen WP-200, admit
WP-400 Consumer work, register the Consumer architecture gate, or claim
production protocol progress.

## Established aggregate gap

The current repository has all of these implementation and authority facts:

1. `PLAN-SET-001` requires one immutable consumed plan-set generation to exist
   before `consume` returns a handle.
2. Planning owns effective TD interpretation, target/Form enumeration,
   candidate construction, target lookup material, logical-plan construction,
   and binding-artifact compilation. Servient must not become a second planner.
3. The completed WP-200 Consumer compiler accepts one exact Property Read
   `(property name, property-form index)` coordinate. Its output contains
   logical plans, artifact envelopes, and compact artifact references for that
   coordinate, not an aggregate consumed generation.
4. Current public `Servient::consume(td)` accepts an ordinary `Thing`, creates
   the legacy `ConsumedThing`, and does not enter the target Planning build
   transaction.
5. Legacy `ConsumedThingHandle::read_property` rescans TD/Form state at call
   time and invokes legacy selection/binding paths.
6. WP-400's active Consumer slice instead requires publication of one immutable
   plan generation, indexed target selection, plan-lease retention, selected
   target binding execution, response validation, and exact release/drain.

Therefore a conforming WP-400 Consumer implementation cannot bridge the gap by:

- scanning or interpreting TD/Form state in Servient;
- choosing an arbitrary first property or Form;
- narrowing public `consume(td)` to one fixture coordinate;
- re-entering legacy `ConsumedThing`, `BindingRequest`, support probes, or
  runtime Form selection;
- publishing only a successfully compiled subset without an accepted policy;
- losing plan/binding generation or execution-owner reachability; or
- performing unbounded or unaccounted validation/build/index work.

The stable ownership direction is Planning-owned semantic construction followed
by Servient-owned reservation, publication, plan-set lifetime, leases, drain,
and reclamation. The exact handoff remains unresolved.

## Unresolved validated-input boundary

`PlanBuildInput::new` currently accepts `&Thing` under the field name
`validated_td`; the type carries no validation proof. Public `consume(td)`
receives an ordinary owned `Thing`. A later decision must establish:

- which TD/Foundation/Core/Servient owner performs Consumer admission
  validation and at what validation level;
- whether the proof is a type, an owned effective view, a transaction state, or
  another bounded carrier;
- source/input lifetime and bytes;
- validation traversal, resumability, cancellation, and work charging;
- phase-local temporary storage, peak overlap, and largest contiguous
  allocation;
- how Planning consumes the result without trusting a field name; and
- ADR-0013 impact on TD/Foundation and the completed exact WP-200 tranche.

The prior investigation proposed resolving this before aggregate source
admission. That ordering remains plausible but is not accepted merely by this
workspace record.

## Unresolved aggregate construction and lookup

A later decision must determine the deterministic coordinate set and immutable
output needed by `consume(td)`:

- traversal order for `Thing::properties` (`BTreeMap` key order is the current
  implementation fact) and each property's retained Form order/index;
- treatment of a declared property with no effective Property Read Form;
- whether every effective coordinate is mandatory eager work or whether an
  explicitly authorized exclusion/negative record exists;
- atomic failure semantics when one coordinate fails;
- construction of a target/operation lookup that avoids hot-path scans;
- omitted versus explicit `form_index` behavior within only the addressed
  property's plan range;
- candidate and registration-ordinal retention needed in addition to
  `BindingArtifactRef`;
- the plan/candidate/artifact join used by execution; and
- semantic parity between Host-erased and application-static physical storage.

Some prior reviews proposed mandatory eager compilation, one globally selected
Consumer registration, `candidate_order = 0`, and a
`BindingPlanRef`-equivalent join. These are candidate semantics, not active
authority. They must be rechecked against the selected registration model,
resource limits, realistic multi-binding usage, and the smallest executable
Consumer proof.

## Unresolved execution-owner retention

An immutable artifact/candidate identity is not by itself the live startup
object that can execute a call. A consumed generation must retain or reach the
correct complete registration for its whole lifetime. The later decision must
freeze:

- the persistent registration/startup snapshot owner pinned by a consumed
  generation;
- generation-checked registration ordinal/slot lookup;
- binding id/generation, configuration, compatibility, role, plan-set, plan,
  and artifact equality at execution entry;
- Host-erased and application-static ownership representations;
- which Planning snapshot/view is temporary and may be dropped after freeze;
- which execution-owning registration state remains pinned; and
- how the installed result-sealing projections from workspace 0062 are reached
  without exposing raw client halves.

This is not authority to give bindings a TD, retain raw Forms, or restore bare
legacy binding arrays to the hot path.

## Unresolved admission transaction and resources

The aggregate must eventually fit the accepted admission/publication model:

```text
bounded validation and semantic preflight
  -> reserve persistent and cleanup capacity
  -> resumable private Planning build
  -> reconcile completed ledger with reserved bounds
  -> final cancellation check
  -> one atomic publication
```

Questions still requiring exact authority include:

- the resumable cursor and complete owned failure/abort state;
- cancellation checks at bounded intervals and immediately before publication;
- unpublished `Building`/`Frozen` cancellation and idempotent release;
- distinct source/input, phase-temporary, compiled-runtime, diagnostic, and
  cleanup memory accounts under `ADMIT-MEM-001`;
- live peak and largest-contiguous-allocation evidence;
- pre-reservation ceilings versus the exact completed ledger;
- correct work classes and charge units for pure TD/Planning/registration/index
  work; and
- proof that WP-400 does not rescan, recount, or remeasure Planning output.

The current use of `WorkClass::BindingPolls` during exact Consumer planning is
implementation evidence that work taxonomy may need correction. Relabeling
Planning work as binding, cleanup, or schema work merely to reuse a counter is
not an accepted solution.

## Unresolved identity and generation authority

A deterministic dense aggregate ordinal can choose a `PlanId` slot, but it
does not allocate the non-wrapping `Generation` inside `PlanId`.
`PlanSetGeneration` is a different identity with a different lifecycle role.
The later decision must identify:

- the owner of PlanId slot and generation allocation;
- equality between preflight, build, publication, lookup, and execution;
- exhaustion/failure behavior without wraparound;
- deterministic behavior across Host/static representations; and
- the relationship between aggregate PlanIds and the exact one-plan WP-200
  compiler inputs already implemented.

## Security and profile questions

The first Consumer proof is intended to stay narrow. Prior investigation
favored a deterministic no-material NoSec case and rejection of shapes that
need credentials, provider access, `AppliedSecurity`, branch selection, or
binding-carried security state. The exact executable predicate depends on the
validated-input boundary and remains unaccepted here.

Portable application-static progress also needs an exact pre-publication
cancellation owner. Host shutdown authority cannot simply be assumed to exist
in the caller-driven `begin_destroy()` representation. Result sealing from
workspace 0062 is shared semantics, but aggregate cancellation and publication
may retain profile-specific physical mechanics.

## Evidence retained from prior review rounds

The prior aggregate reviews consistently identified these omitted categories:

- atomic compile/failure and zero-plan semantics;
- deterministic target/Form ordering and indexed lookup;
- executable validation/security admission;
- reserve-before-build ordering and exact resource accounting;
- resumable cursor, cancellation, and failure ownership;
- candidate/registration ordinal and execution-owner retention;
- phase-specific work charging;
- static admission cancellation; and
- PlanId generation authority.

That convergence is evidence that the aggregate is broader than a container,
but it does not preselect the old proposed data model or make every prior
constraint authoritative.

## Relationship to project progression

Workspace 0062 is the smaller blocking predecessor because its defect appears
with one already-built plan and one call. Once result sealing is accepted,
migrated, readmitted, implemented, and proved, the next task session must
reconstruct this aggregate topic against the then-current repository. It may
select one bounded upstream subclaim, a combined aggregate decision, or a
simpler implementation proof if new evidence justifies it.

This document does not store that future next action and does not require that
the prior “validated input first, registration pinning second” sequence be
followed.

The intended high-level composition remains:

```text
ordinary Consumer input
  -> bounded validated admission input
  -> Planning-owned aggregate preflight/build
  -> Servient-owned reservation and atomic publication
  -> consumed generation pins immutable plans and installed registrations

read_property(name, options)
  -> indexed immutable target lookup
  -> strict candidate/plan/artifact reference
  -> generation-checked complete registration
  -> OutboundRequest::property_read(...)
  -> Core-mediated sealed Host/static execution
  -> delivery and terminal settlement
```

No step authorizes Servient TD interpretation, call-time TD/Form scans, hidden
fallback, or unbounded/unaccounted admission work.

## Review and migration condition

This topic may remain merged as `DISCUSSING` so the unresolved aggregate
evidence has a current discoverable owner. Before it can become `DECIDED`, a
fresh independent review must reconstruct the exact current implementation and
select a constructible handoff with explicit validation, identity, lifecycle,
resource, cancellation, and Host/static evidence.

Only after that acceptance may stable conclusions migrate into TD/Foundation,
Planning, Binding, Servient, API/resource, and work-package authority. This
topic itself admits no production source and changes no gate or milestone
status.
