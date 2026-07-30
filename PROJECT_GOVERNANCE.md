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

## Executable Critical-Path Conversion

The active executable critical path must have a bounded conversion from design
uncertainty to implementation. `PROJECT_STATE.md` must name:

- one next executable objective;
- the finite set of blockers that prevent its exact implementation candidate;
- the observable design-closure event after which candidate preparation may
  begin; and
- the next source-changing event expected after review and admission.

A blocking workspace investigation must define a finite closure boundary:
questions to answer, affected authoritative owners, required authoring
fixtures, and the candidate or evidence output that consumes the decision.
Newly discovered detail remains inside that boundary when it affects the same
ownership, lifecycle, resource, public-contract, rollback, or evidence truth.
It becomes a separate blocking topic only when the tranche-sizing rule above
proves a distinct boundary. Disjoint detail is deferred and cannot extend the
active critical path.

When one technical decision, its authoritative migration, and implementation
admission have the same affected contract, rollback boundary, and independent
validation truth, they form one conversion packet and one exact scoped review
boundary. They must not be serialized into separate candidate/review cycles
merely because workspace, specification, work-package, fixture, audit, and
registry artifacts are different files. A separate ADR or review remains
required when architecture governance identifies durable cross-domain
rationale, a different reversal cost, or independently falsifiable evidence.

Preparation ends when the recorded closure boundary is satisfied and the exact
candidate's pre-implementation checks pass. Further non-implementation work may
block that candidate only when an explicit impact record shows a newly
discovered change to semantics, ownership, lifecycle, resources, dependency
truth, or completion-evidence truth. Otherwise the next actions are independent
review, one recorded admission checkpoint, and implementation. Separate
approval and in-progress checkpoints are not required when one recoverable
pre-source admission checkpoint records both truths.

Continuity updates, registries, audits, and checkers travel with the decision,
admission, implementation, or completion checkpoint whose truth they record.
They are not independent critical-path prerequisites. Add a checker only when
it protects a stable invariant that implementation or a later authority change
could violate and that existing executable checks do not already prove.

Independent review, pre-source admission, risk-proportional evidence, and
architecture change control remain mandatory. This rule bounds their
composition; it does not waive them.

### Validation Truth and Support Artifacts

Validation artifacts have a directed responsibility model:

- registered specifications and work-package records own technical contracts,
  dependency, admission, completion, and removal truth;
- `PLAN.md` projects roadmap and milestone state, while `PROJECT_STATE.md`
  projects the current continuation point;
- audits and attestations record evidence about immutable candidates or
  implementation checkpoints;
- registries enumerate owners and evidence without redefining their content;
  and
- executable checks derive and falsify invariants from those owners.

A support artifact does not become an independent source of technical truth
merely because another support artifact references it. A support-only failure
blocks work only when it demonstrates a false contract, dependency, admission,
completion, authority, or evidence claim. Otherwise its repair travels with the
checkpoint whose truth it records and does not reopen an already reviewed
technical contract.

Once a review candidate exists, its identity must be immutable and independent
of later unrelated `HEAD` movement. A state-changing review must exercise the
declared next repository transition before attestation, including its exact
path boundary, required manifest or registry updates, expected absent/present
source boundary, and the next implementation topology. Passing only the
candidate's current state is insufficient evidence for a transition claim.

Project progress is reported on three distinct tracks:

- architecture/authority closure;
- package-local contract completion; and
- executable vertical integration, identified by the highest completed tranche
  in the active integration gate.

One track must not be presented as executable progress on another.

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

Review claims must identify the defect class they cover. A pre-source review
may close contract, ownership, topology, portability-schema, and admission
transition claims, but it cannot close runtime behavior, workload, lifecycle,
resource, performance, or production-author usability claims without matching
executable evidence. Reviewers reconstruct the intended contract from
authoritative owners; an author-prepared audit is navigation and evidence, not
a substitute authority.

Session separation is one independence mechanism, not the evidence claim by
itself. Material independence comes from immutable candidate reconstruction,
negative or mutation cases, external public-boundary fixtures, and
risk-appropriate compile, runtime, workload, and integration evidence.

## Default-Branch Validation

Every proposed default-branch revision must pass one reproducible mainline
matrix:

- committed diff hygiene for the proposed revision range;
- `tools/check-design-artifacts.sh`;
- `cargo test --workspace --locked`; and
- `sh scripts/check-feature-matrix.sh`.

Candidate-, admission-, completion-, workload-, and release-specific checks
remain additional requirements when their registered owner applies; the
mainline workflow also executes the registered work-package/evidence checker
so the required status validates repository-owned task state for the proposed
revision. That checker does not invent missing task-specific evidence. A local
result is valid author or review evidence, while a successful remote workflow
status is integration evidence. Do not claim that the default branch is
mechanically protected unless the remote branch rule actually requires the
recorded mainline status check.

## Remote Task Review and Publication

A bounded repository-changing task is handed off through GitHub automatically
at its completion. The standing workflow is:

1. confirm the intended diff and preserve unrelated work;
2. update continuation state and run the task-specific evidence plus the
   risk-appropriate default-branch matrix;
3. retain semantically necessary checkpoint boundaries instead of squashing
   immutable candidate, review, admission, implementation, or evidence
   topology into one commit;
4. commit on the current task branch and push it to `origin`;
5. open one draft pull request targeting the remote default branch, or update
   the existing pull request for that task; and
6. hand off the pull-request URL, exact commits, checks, remote workflow state,
   and known limitations to the Owner.

When a new task starts from the default branch, its branch is named
`agent/<task-slug>`. Follow-up changes for the same bounded task remain on the
same branch and pull request. A dependent task normally starts only after its
predecessor pull request is remotely reviewed and integrated. If the Owner
explicitly requests stacked work, the dependent pull request must name its
predecessor and use the predecessor branch as its review base until the stack
is rebased after integration.

Task commits are never pushed directly to the default branch. The Owner's
remote review may contribute project constraints, product feedback, or
counterexamples, while AI remains responsible for technical evidence and
milestone judgment. A remote workflow pass is integration evidence and does
not replace local candidate, admission, completion, or release checks.

### Automatic Integration Eligibility

A draft pull request may be promoted to ready and use GitHub native auto-merge
only for the exact current head and only after all of these are true:

1. the intended scope is complete and contains no unrelated work;
2. every applicable candidate, independent-review, admission, completion,
   workload, release, and removal record is present and current;
3. the task-specific local checks pass and the required remote `validation`
   job covers that head;
4. the branch is current with the target branch, conflict-free, not a
   dependent stack, and has no unresolved review conversation or requested
   change;
5. the task crosses no unresolved Owner-owned project-goal, product-trade-off,
   unacceptable-direction, public-release, or other external-commitment
   boundary; and
6. the active remote ruleset has been verified to require strict
   current-base validation and conversation resolution.

Eligible automatic integration uses GitHub's native mechanism, a merge commit,
and an expected head object id. Squash and rebase integration are prohibited
when they rewrite semantically meaningful candidate, review, admission,
implementation, or evidence identities. A later commit invalidates the
eligibility decision and reruns the applicable evidence. Failed, cancelled,
stale, missing, or superseded checks, conflicts, or stacked dependencies leave
the pull request unmerged. No custom write-capable merge workflow or merge
queue is introduced without a separately evidenced need.

Until the remote ruleset prerequisites are verified, the pull request remains
draft and auto-merge remains disabled. Owner intervention remains required for
Owner-owned boundaries and actual public release, not as a ceremonial merge
click for routine technical work.

If push or pull-request creation fails, the task remains locally checkpointed
but remote handoff is incomplete. The blocker and exact retry action must be
recorded in `PROJECT_STATE.md`; the AI must not silently treat a local commit
as remotely reviewable.

### Remote Reconciliation

GitHub owns pull-request draft/ready/check/merge facts. Git commits, registered
work-package records, audits, and evidence own candidate, review, admission,
implementation, and completion truth. `PROJECT_STATE.md` projects the last
observed combination and cannot override either owner.

Before substantial work whose next action depends on remote integration, a
fresh session fetches the remote default branch when available and reconciles
the recorded task/PR state. Offline work may use the last observed snapshot
but cannot release dependent source work from an unverified merge. Pre-merge
continuation state records both conditional next actions. After integration,
the next task updates continuation state in its first checkpoint; the project
does not use a write-capable post-merge workflow or a recursive state-only pull
request.

A dependent task begins only after the merge is visible in the fetched
default branch and the default-branch validation for the merge revision
passes. This rule applies equally to manual and automatic integration.

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
