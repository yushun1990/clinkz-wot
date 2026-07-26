# Project Governance

This document defines how the ClinkZ-WoT project is planned, reviewed,
tracked, and progressed.

It does not define technical architecture decisions. Technical convergence
rules are maintained in `ARCHITECTURE_GOVERNANCE.md`.

## Governance Principles

ClinkZ-WoT separates:

  Concern                            Artifact
  ---------------------------------- ------------------------------
  AI operating behavior              `AGENTS.md`
  Project execution governance       `PROJECT_GOVERNANCE.md`
  Technical convergence governance   `ARCHITECTURE_GOVERNANCE.md`
  Project roadmap                    `PLAN.md`
  Current execution context          `PROJECT_STATE.md`

ClinkZ-WoT uses AI-led development. AI owns routine technical decision-making
and evidence closure. Owner feedback keeps the work aligned with project goals
and real-world constraints.

## Roles and Responsibilities

### AI Agent

Responsible for:

- maintaining `PROJECT_STATE.md`;
- keeping milestone progress current in `PLAN.md`;
- deciding technical architecture and API direction from repository evidence;
- decomposing work packages and selecting implementation order;
- assessing technical risk and evidence sufficiency;
- investigating workspace questions, counterexamples, and concerns;
- migrating stable conclusions to the proper authoritative owner;
- closing technical milestones when registered exit criteria and evidence are
  satisfied;
- determining technical release readiness.

AI agents must not silently change accepted project goals or release claims.
They also must not transfer technical judgment to the Owner when the decision
can be made from architecture, code, tests, specifications, audits, or other
repository evidence.

### Project Owner

Responsible for:

- maintaining project vision, target outcomes, and unacceptable directions;
- identifying real-world constraints and product trade-offs;
- raising questions, counterexamples, doubts, and usage-experience feedback;
- deciding actual public release or other irreversible external commitments.

The Owner is not a routine technical approval gate. Owner input does not
preselect a technical answer and does not automatically block unrelated work.

AI requests Owner clarification only when a choice depends on project goals,
product trade-offs, real-world constraints, unacceptable directions, or
irreversible external commitments rather than technical evidence.

## Milestone Lifecycle

Milestones are defined in `PLAN.md`.

    OPEN
     |
    IN_PROGRESS
     |
    REVIEW
     |
    CLOSED

Additional states:

    IN_PROGRESS -> BLOCKED
    REVIEW -> REOPEN
    CLOSED -> REOPEN

`REVIEW` means AI is assembling or checking repository evidence against the
registered exit criteria. `CLOSED` means AI has determined from repository
evidence that the milestone's technical exit criteria are satisfied.

Owner visibility and feedback points are non-blocking by default. A milestone
may reopen when Owner feedback identifies a project-goal conflict, omitted
constraint, unacceptable direction, or credible counterexample that invalidates
the technical closure evidence.

## Milestone Update Rules

Milestone status must reflect repository evidence.

Evidence may include:

- implementation;
- documentation;
- tests;
- validation results;
- audits and reviews;
- registered work-package and gate status.

Do not use percentage completion as the primary progress indicator.

AI updates milestone status when the evidence changes. Owner approval is not
required for routine technical milestone closure.

## PLAN.md Maintenance Rules

PLAN.md contains:

- objectives;
- release targets;
- milestones;
- dependencies;
- milestone status;
- acceptance objectives;
- AI-owned open decision queue.

PLAN.md does not contain:

- session logs;
- temporary debugging information;
- detailed design discussions;
- architecture decisions;
- governance policies.

## Open Decision Management

Open project decisions listed in `PLAN.md` are AI-owned unless they explicitly
depend on project goals, product trade-offs, real-world constraints,
unacceptable directions, or irreversible external commitments.

For each open technical decision, AI must:

- investigate the workspace topic and related repository evidence;
- record alternatives, selected direction, and rejected approaches;
- update or create the authoritative document, work package, code, or test that
  owns the conclusion;
- update `PROJECT_STATE.md`;
- keep unrelated admitted work moving when the open decision is disjoint.

Owner questions and counterexamples are evidence inputs. They are not direct
technical instructions and not predetermined conclusions.

## Risk-Proportional Implementation Admission

Implementation admission remains tranche-scoped. No runtime or public-API
change starts without a recorded admitted tranche when the authoritative design
requires one.

Admission authoring and review depth are proportional to semantic risk:

- Category A, local additive implementation: passive values, constructors,
  accessors, error-free conversions, local trait implementations, mechanical
  module moves, or compile-time registration values with no lifecycle behavior.
  Required controls are an existing authoritative contract, exact named scope,
  satisfied dependencies, disjointness from unresolved findings, local
  compile/test evidence, completion evidence, and a recoverable Git checkpoint.
  Category A does not require a new ADR, global architecture review, or broad
  evidence rewrite unless implementation reveals a semantic conflict.
- Category B, cross-module contract implementation: handler entry, planner to
  binding compilation, binding artifact boundaries, Servient orchestration,
  cleanup transfer, resource reservation, or similar work. Required controls
  include explicit work-package/tranche records, dependency and ownership
  review, conformance fixtures, relevant audit or review projection, and
  impact analysis.
- Category C, architecture or invariant change: ownership, lifecycle, time,
  resource accounting, protocol-neutral boundaries, execution paths, or other
  invariant changes. Required controls include workspace investigation,
  authoritative design or ADR migration, work-package revision, evidence
  invalidation or reaffirmation, and architecture review where required.

AI owns the category classification and records the rationale. If later
evidence shows that a Category A change alters semantics, ownership, lifecycle,
resources, progress, or evidence truth, the tranche is reclassified and
reviewed under Category B or C before the affected work proceeds.

A tranche is split only when the parts have different blockers, ownership,
lifecycle effects, authoritative contracts, validation independence,
rollback/failure boundaries, or evidence truth. A tranche is not split merely
because each type, trait, or file can be named separately.

## Review Requirements

A milestone entering `REVIEW` should provide evidence.

Review verifies:

- intended goal achieved;
- registered exit criteria satisfied;
- implementation matches specifications;
- no known architectural conflict intersects the milestone closure claim.

Independent technical reviews or audits may be required by architecture
governance, work-package records, or milestone exit criteria. Those reviews are
technical evidence requirements, not Owner approval gates.

## Change Management

Changes affecting project goals, release claims, unacceptable directions,
external commitments, or product trade-offs require Owner clarification.

Changes affecting technical architecture must follow
`ARCHITECTURE_GOVERNANCE.md`.

Changes affecting implementation sequencing, work-package boundaries, API
shape, evidence sufficiency, or technical milestone state are decided by AI
from repository evidence, subject to the accepted governance and architecture
rules.

## Workspace Transition

Unresolved topics belong in `workspace/`.

Lifecycle:

    OPEN -> DISCUSSING -> DECIDED -> MIGRATED

- `OPEN`: the Owner or AI identified a question, concern, review finding, or
  proposal.
- `DISCUSSING`: AI is investigating alternatives, evidence, trade-offs, and
  impact.
- `DECIDED`: AI has selected a direction and recorded the rationale, but the
  conclusion has not yet been fully projected.
- `MIGRATED`: the stable conclusion is present in its authoritative owner:
  documentation, work-package records, source code, tests, or governance.

Workspace records are non-authoritative discussion history. They must not be
treated as Owner instructions or accepted technical decisions merely because
they exist.

If later Owner feedback introduces a new target constraint, goal conflict, or
credible counterexample, AI reopens the topic or creates a linked follow-up and
re-evaluates the migrated conclusion.

## Release Responsibility

Technical release readiness is an evidence judgment made by AI from the
registered release criteria, clean-checkout verification, known limitations,
and conformance records.

Actual public release execution is an Owner decision because it is an external
project commitment. The Owner may choose to publish, defer, or change the
release timing after AI reports technical readiness.

## AI Session Continuity

Before ending substantial work:

- update `PROJECT_STATE.md`;
- record blockers;
- record next safe actions;
- ensure milestone status is accurate.

The repository must remain understandable without previous conversation
history.
