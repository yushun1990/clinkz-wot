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
- current execution state, which belongs to `PROJECT_STATE.md`;
- collaboration and implementation-admission policy, which belong to
  `PROJECT_GOVERNANCE.md` and `AGENTS.md`;
- exact work-package dependencies or admission state, which belong to
  `docs/work-packages/index.toml`; or
- session history and unresolved reasoning, which belong to `workspace/`.

Architecture governance must not become a second execution plan.

## AI-led Architecture Responsibility

ClinkZ-WoT uses AI-led development.

AI agents hold primary responsibility for technical architecture decisions.
They must investigate repository evidence, select a direction, record the
rationale and rejected alternatives, migrate the stable conclusion to its
proper authoritative owner, and validate the resulting repository state.

The Project Owner contributes project vision, target outcomes, real-world
constraints, unacceptable directions, product trade-offs, questions,
counterexamples, and usage feedback. Owner input is evidence for architectural
investigation; it is not a predetermined technical conclusion and is not a
routine architecture approval gate.

AI requests Owner clarification only when a choice depends on project goals,
product trade-offs, real-world constraints, unacceptable directions, or an
irreversible external commitment that repository evidence cannot resolve.

## Active Architecture Target

The active target is the v4.9 architecture-closure revision.

The v4.8 detailed-design candidate is migration input only. A v4.8 contract is
not active merely because it still exists in a historical or residual document.
It becomes active only after reconciliation into the registered v4.9 authority
set.

The v4.9 closure effort must produce one coherent and executable target across:

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

`docs/design.md` is the active revision entry point, normative-source manifest,
and temporary residual owner for contracts not yet migrated. It must not be
used to override a more specific registered v4.9 owner. Its residual detailed
ownership must continue to shrink until D3 is decided and migrated.

`workspace/` is non-authoritative. It records questions, proposals,
investigations, alternatives, and reasoning history. A workspace topic becomes
architecturally effective only after its stable conclusion is migrated into the
proper registered authority.

## Frozen Direction for v1

The accepted v1 architecture direction currently includes:

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
3. distinguish active v4.9 authority from historical or residual migration
   input;
4. open or update a workspace investigation when the conflict affects semantics,
   ownership, lifecycle, resources, public behavior, or evidence truth;
5. select and record one coherent direction;
6. migrate the decision so each detailed contract has one active owner; and
7. invalidate or reaffirm prior evidence affected by the correction.

No architecture conflict is resolved merely by adding another summary document.

## Architecture Closure

The v4.9 architecture-closure milestone is technically complete only when:

1. the architecture backbone freezes primary flows, module boundaries,
   compiled-plan lifecycle, Servient orchestration, and Protocol Binding
   integration and deployment;
2. every accepted ADR has one non-conflicting authoritative projection;
3. every active detailed requirement has one registered normative owner;
4. residual v4.8 and `docs/design.md` ownership has been reconciled into the
   modular v4.9 specification set;
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
- `PROJECT_STATE.md` for the current work item, blockers, stopping point, and
  next safe actions;
- `PROJECT_GOVERNANCE.md` for collaboration, review, milestone lifecycle, and
  risk-proportional implementation governance;
- `docs/work-packages/index.toml` for exact implementation admission and DAG
  authority; and
- Git history for recoverable change checkpoints.

If this document begins to accumulate live task ordering, session status, or
milestone scheduling, that material must be moved to its proper execution owner.
