# Architecture Governance

## Purpose

This document defines how ClinkZ-WoT technical architecture converges, becomes
authoritative, changes, and is re-evaluated.

It governs:

- architecture authority and ownership;
- the active architecture target;
- architectural decision and migration rules;
- conflict detection and resolution;
- architecture review and closure evidence; and
- reopening decisions when new constraints or counterexamples invalidate them.

It does not own:

- the project roadmap or milestone ordering, which belong to `PLAN.md`;
- the current engineering claim, plan, or acceptance criteria, which belong to
  the active task session and pull request;
- continuation and observed remote state, which must be reconstructed from
  the repository, GitHub, implementation, tests, and CI;
- collaboration and implementation-admission policy, which belong to
  `PROJECT_GOVERNANCE.md` and `AGENTS.md`;
- exact work-package dependencies or admission state, which belong to
  `docs/work-packages/index.toml`; or
- session history and unresolved reasoning, which belong to `workspace/`.

Architecture governance must not become a second execution plan.

## AI-led Architecture Responsibility

ClinkZ-WoT uses AI-led development.

The Max Technical Lead holds primary responsibility for technical architecture
decisions and projects the exact implementation claim into the active task.
It must investigate repository evidence, select a direction, record rationale
and rejected alternatives in their proper owners, migrate the stable
conclusion, and define falsifiable acceptance criteria.

The High Executor implements accepted authority but does not silently revise
it. Concrete awkwardness, repeated workaround pressure, unconstructible public
APIs, or ownership/resource contradictions are architecture evidence: the
Executor records a minimal finding and returns it to the Lead. A fresh Max
Acceptance Reviewer checks both conformance and credible omitted defects.
ChatGPT may challenge important plans before execution, and Ultra periodically
audits the whole repository; neither advisory role silently overrides active
authority.

The Project Owner contributes project vision, target outcomes, real-world
constraints, unacceptable directions, product trade-offs, questions,
counterexamples, and usage feedback. Owner input is evidence for architectural
investigation; it is not a predetermined technical conclusion and is not a
routine architecture approval gate.

AI requests Owner clarification only when a choice depends on project goals,
product trade-offs, real-world constraints, unacceptable directions, or an
irreversible external commitment that repository evidence cannot resolve.

## Active Architecture Target

The authoritative revision is v5.1 Consumer one-shot authority. Its active
source map is `docs/spec/v5-authority-reset.toml`; accepted specifications,
ADRs, work packages, code, and tests own the current contract.

No individual v5.1 artifact grants implementation admission outside that
active set. Git history retains the superseded activation and rollback record;
current validation does not replay it.

The v4.8 detailed-design candidate is migration input only. A v4.8 contract is
not active merely because it still exists in a historical or residual document.
It becomes active only after reconciliation into the registered v4.9 authority
set.

The active v5 architecture-closure effort must produce one coherent and executable target across:

- architecture flows and module boundaries;
- accepted ADRs;
- domain specifications;
- public and internal API ownership;
- state and lifecycle models;
- resource and performance contracts;
- requirements and work packages;
- implementation and tests; and
- review, audit, and conformance evidence.

## Architecture Authority Model

Architecture must be read through registered owners rather than inferred from a
single narrative document.

Primary architecture sources are:

1. `docs/architecture/README.md` and its registered architecture documents for
   cross-domain structure, primary flows, boundaries, and invariants;
2. accepted ADRs under `docs/ADRs/` for decisions that require durable rationale;
3. registered domain specifications under `docs/spec/` for exact single-owner
   behavioral, API, lifecycle, and SPI contracts;
4. `docs/artifacts.csv` and other registered indexes for authority, ownership,
   and evidence projection;
5. `docs/work-packages/index.toml` for implementation dependency, admission,
   completion, removal, and evidence contracts; and
6. source code and tests for implementation truth and executable conformance.

Under active v5.1 authority, `docs/design.md` is the concise revision entry
point and normative-source manifest. It must not override a more specific
registered owner. ADR-0018 established the bounded v5.0 reset; ADR-0019 then
re-adopted exactly `PLAN-REQUEST-001`, `BIND-OUT-001`, and the narrowed
`API-OPTIONS-001` for Consumer one-shot entry. The active v5.1 set therefore
contains exactly 65 requirements while the remaining identities retain their
checked inactive dispositions.

`workspace/` is non-authoritative. It records questions, proposals,
investigations, alternatives, and reasoning history. A workspace topic becomes
architecturally effective only after its stable conclusion is migrated into the
proper registered authority.

## Frozen Direction for v1

The accepted v1 architecture direction active in v5.1 includes:

- an explicit compiled-plan-set lifecycle and binding-artifact boundary;
- immutable admitted plan sets separating protocol-neutral logical plans from
  binding-owned artifacts;
- Cargo-linked, application-registered Protocol Binding crates;
- startup-only binding composition for one Servient instance;
- Servient-owned activation, orchestration, plan-set lifetime, scheduling, and
  cleanup policy;
- engine-orchestrated, route-scoped binding progress with no hidden direct
  handler-dispatch path;
- Protocol Bindings that own protocol syntax, I/O, correlation, adaptation, and
  binding-local flow control, but do not select handlers or reinterpret TDs;
- atomic serving publication through one Servient-owned activation authority
  and nonretained route-scoped accept permits;
- generation ownership, bounded resources, cancellation, and terminal cleanup
  for retained operations;
- protocol-neutral semantics across host and `no_std + alloc` profiles with
  profile-appropriate progress mechanisms; and
- a modular normative-document hierarchy with single-owner detailed contracts.

The exact contracts live in the registered architecture, ADR, specification,
API, state, resource, performance, requirement, and work-package artifacts.
This summary is navigation, not an alternative normative owner.

For the target Protocol Binding and Servient surface, the invariant direction
above is frozen, but empirical maturity is deliberately narrower. Immutable
artifacts, protocol-owned I/O and correlation, Servient-owned dispatch and
publication, generation identity, route-scoped bounded progress,
permit-gated acceptance, and explicit cleanup transfer remain authoritative.
A real-target Zenoh Property Read probe now exercises both application-static
and public Host-erased paths through actual protocol I/O and a network round
trip. Its corrected Host carrier preserves one nonreplaceable erased route
state, original preparation input, footprint, and generation identity across
the distinct prepared, active, and committed stage owners until terminal
cleanup, exposing only a type-checked shared pinned projection for
protocol-local operations. Servient retains the committed owner during accept
polling and lends the binding only a shared guard reference. The exact helper
surface, broad operation
signatures, multi-route resource model, and profile-specific physical resource
accounting remain maturity and correction points; the paired probe does not
freeze those broader containers or justify merging the Host and static
lifecycle APIs.

That bounded probe precedes the aggregate mock Property Read candidate. It is
architecture feedback rather than WP-600 product admission. Zenoh and
zenoh-pico supply two runtime profiles for one protocol family, not general
protocol-shape neutrality. A broader empirical claim requires a materially
contrasting protocol shape; otherwise release language remains limited to
protocol-independent engine ownership plus Zenoh-family operation.

## Architectural Decision Lifecycle

Architecture questions normally progress through:

```text
OPEN -> DISCUSSING -> DECIDED -> MIGRATED
```

- `OPEN`: the Owner or AI identifies a question, conflict, counterexample,
  proposal, or missing contract.
- `DISCUSSING`: AI investigates alternatives, repository evidence, affected
  owners, compatibility, lifecycle, resources, and validation impact.
- `DECIDED`: AI selects a technical direction and records the rationale and
  rejected alternatives.
- `MIGRATED`: the stable conclusion is projected into every affected
  authoritative owner, work package, checker, fixture, audit, review, and
  evidence record.

A decision is not complete at `DECIDED` when authoritative artifacts still
contradict it. Migration must eliminate competing active interpretations rather
than relying on informal precedence.

## Architecture Change Control

A change is architecture-sensitive when it alters one or more of:

- module or ownership boundaries;
- lifecycle phases or state transitions;
- time, generation, or identity semantics;
- resource accounting or boundedness;
- cancellation, cleanup, or failure ownership;
- protocol-neutral boundaries;
- public API semantics;
- execution paths or orchestration authority;
- plan-set, route, subscription, or binding-artifact lifetime; or
- an accepted invariant or release claim.

Architecture-sensitive changes require:

1. a bounded workspace investigation;
2. identification of all affected authoritative owners and requirements;
3. explicit alternatives and a selected direction;
4. an ADR when durable cross-domain rationale or reversal cost requires one;
5. migration into the exact architecture and domain specifications;
6. work-package and dependency revision where implementation scope changes;
7. invalidation, replacement, or explicit reaffirmation of affected evidence;
8. updated checkers, fixtures, audits, and reviews where necessary; and
9. a recoverable Git checkpoint.

Local additive implementation that does not change architecture remains governed
by the risk-proportional admission rules in `PROJECT_GOVERNANCE.md`. This file
does not duplicate those categories or authorize implementation work.

## Conflict Resolution

When active sources conflict, AI must not silently choose whichever text is most
convenient.

AI must:

1. identify the exact conflicting claims;
2. determine each artifact's registered ownership and revision status;
3. distinguish active v5.1 authority from historical or residual migration
   input;
4. open or update a workspace investigation when the conflict affects semantics,
   ownership, lifecycle, resources, public behavior, or evidence truth;
5. select and record one coherent direction;
6. migrate the decision so each detailed contract has one active owner; and
7. invalidate or reaffirm prior evidence affected by the correction.

No architecture conflict is resolved merely by adding another summary document.

## Architecture Closure

The active v5 architecture-closure milestone is technically complete only when:

1. the architecture backbone freezes primary flows, module boundaries,
   compiled-plan lifecycle, Servient orchestration, and Protocol Binding
   integration and deployment;
2. every accepted ADR has one non-conflicting authoritative projection;
3. every active detailed requirement has one registered normative owner;
4. residual v4.8/v4.9 ownership is historical rather than active, all 121 v4.9
   identities have one checked v5 authority disposition, and exactly 65 active
   requirements have one registered normative owner;
5. API, state, resource, performance, requirement, and work-package artifacts
   identify the same revision and contracts;
6. architecture-boundary and registered executable checks pass;
7. affected evidence is current rather than silently carried across incompatible
   changes; and
8. an independent same-revision review closes every applicable architecture
   gate.

AI closes the technical milestone from registered evidence. Owner feedback may
reopen it when a project-goal conflict, omitted real-world constraint,
unacceptable direction, or credible counterexample invalidates the closure
claim.

## Reopening and Supersession

An accepted or migrated architecture decision must be reopened when new evidence
shows that it is unimplementable, internally inconsistent, operationally
unrealistic, incompatible with a project constraint, or contradicted by a
credible counterexample.

Reopening must:

- preserve the prior rationale as history;
- identify the invalidated assumption or evidence;
- keep unrelated admitted work moving when it is demonstrably disjoint;
- update every affected authoritative projection; and
- avoid presenting a superseded contract as simultaneously active.

## Relationship to Planning and Execution

This document defines how architecture remains coherent. It does not decide what
work runs next.

Use:

- `PLAN.md` for roadmap, milestones, objectives, dependencies, and progress
  state;
- the active task session and pull request for the current claim, exact plan,
  acceptance criteria, and review boundary;
- the repository, GitHub, implementation, tests, and CI for current and remote
  state;
- `PROJECT_GOVERNANCE.md` for collaboration, review, milestone lifecycle, and
  risk-proportional implementation governance;
- `docs/work-packages/index.toml` for exact implementation admission and DAG
  authority; and
- Git history for recoverable change checkpoints.

If this document begins to accumulate live task ordering, session status, or
milestone scheduling, that material must be moved to its proper execution owner.
