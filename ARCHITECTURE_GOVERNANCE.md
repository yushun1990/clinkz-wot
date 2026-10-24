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

The target in this tree is v6.0, the semantic Engine over Binding-owned native
runtimes selected by ADR-0025. `docs/spec/authority.toml` is its sole source map.
Mainline activation is the atomic integration of the independently reviewed
complete authority tree. A task branch does not change master's active authority;
there is no split per-document or per-feature activation.

`docs/design.md` is the concise normative-source entry. Registered architecture,
ADRs, domain specifications and API/state/resource/work-package projections must
agree. Code/tests establish implementation availability and cannot silently amend
contracts. Historical v4/v5 records, the old fixed requirement set and passed
Property Read gates have explicit dispositions; they grant no successor admission.

## Architecture Authority Model

Read authority through its registered detailed owners: backbone for component
relationships/flows, ADRs for rationale/supersession, domain specifications for
exact behavior, machine projections for their declared fields, work packages for
dependencies/admission, and code/tests for actual behavior. Each active requirement
has exactly one definition; changed meanings use new identities where necessary.
The source map preserves each prior identity's disposition. Neither a fixed
historical requirement count nor a checker can veto a justified architecture
correction. Workspace topics remain investigation/provenance rather than authority.

## Selected direction

Shared TD semantics, independent live authority, truthful Producer advertisement,
logical/physical ownership separation, funded bounded cancellation/cleanup and
equivalent supported Host/constrained semantics remain binding. Separate execution
capabilities sit over concrete Binding-owned native runtimes. Native matching,
correlation, distribution and I/O stay out of the Engine. Eager plan sets, route
stage inventories, one route/Thing resource ownership and universal session/RPC
representations are not the general architecture. Exact detailed contracts have
one owner in the source map; this section is navigation, not a second specification.

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
3. distinguish the current registered revision from historical or restricted
   implementation input;
4. open or update a workspace investigation when the conflict affects semantics,
   ownership, lifecycle, resources, public behavior, or evidence truth;
5. select and record one coherent direction;
6. migrate the decision so each detailed contract has one active owner; and
7. invalidate or reaffirm prior evidence affected by the correction.

No architecture conflict is resolved merely by adding another summary document.

## Architecture Closure

The architecture-closure milestone is technically complete only when:

1. backbone, ADRs, domain owners and projections identify one coherent revision;
2. each active requirement has one exact owner and each inherited identity an
   explicit disposition;
3. public ownership, semantic state, resource and work-package boundaries agree;
4. applicable structural and executable checks pass at the claimed revision;
5. implementation and performance claims have current scoped evidence rather
   than inherited acceptance; and
6. independent same-revision review finds no unresolved material conflict.

A merge activates authority, not implementation completeness or release readiness.

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
