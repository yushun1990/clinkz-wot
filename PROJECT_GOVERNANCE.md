# Project Governance

This document defines how ClinkZ-WoT is planned, progressed, reviewed, and
recovered across AI-assisted development sessions.

Technical architecture decisions are owned by `ARCHITECTURE_GOVERNANCE.md`,
registered specifications, ADRs, work packages, code, and tests. This document
does not redefine them.

## Governance Model

ClinkZ-WoT uses repository-grounded, AI-led development.

The durable project model is intentionally small:

| Concern | Owner |
|---|---|
| AI operating behavior | `AGENTS.md` |
| Project progression and collaboration | `PROJECT_GOVERNANCE.md` |
| Architecture authority and design change | `ARCHITECTURE_GOVERNANCE.md` |
| Roadmap and milestones | `PLAN.md` |
| Accepted technical truth | `docs/`, work packages, ADRs, code, tests |
| Open investigation | `workspace/` |
| Current implementation and history | Git, GitHub pull requests, CI |

The repository does not maintain an additional current-task or continuation
state database. `EXECUTION.md` and `PROJECT_STATE.md` are deprecated as active
governance artifacts and remain only where historical evidence topology still
references their paths.

## No Shadow State

Do not persist transient state merely so a future model can resume a
conversation. In particular, do not maintain repository copies of:

- the current model role;
- the current task checklist;
- the next action;
- the last observed pull-request or CI status;
- conversational handoff notes;
- a summary of facts already owned by code, Git, GitHub, tests, specs, work
  packages, audits, or `PLAN.md`.

A fresh session must re-discover current state from authoritative sources.
This is deliberate: remote and implementation truth may have changed, and a
saved model summary is not more authoritative than the sources it summarizes.

## Task-Session Boundary

Prefer one conversation for one natural major engineering task or decision
node. A task node is coherent when its objective, relevant authority,
implementation boundary, and evidence remain materially connected.

A new conversation is appropriate when:

- the prior major task has reached a natural completion or blocker;
- work moves to a materially different architecture or package objective;
- an independent acceptance or audit view is required; or
- the current conversation shows context drift that cannot be cheaply corrected
  by re-reading the relevant repository truth.

Do not create a new session merely because a fixed number of commits, tokens,
files, or hours has passed. Do not keep one conversation indefinitely across
unrelated roadmap nodes merely to preserve memory.

## Session Reconstruction

Every substantial new task session reconstructs its working state rather than
loading a hand-maintained continuation file:

1. Fetch the default branch and inspect the relevant task branch or open pull
   request when one exists.
2. Read `PLAN.md` for durable roadmap and milestone context.
3. Identify the smallest relevant specifications, ADRs, work packages,
   workspace topics, audits, code, and tests.
4. Inspect current implementation and executable evidence before asserting
   status or selecting a source-changing action.
5. Derive the next engineering objective from project goals, roadmap,
   authority, implementation gaps, and evidence.

Remote branch, pull-request, merge, and CI facts are read from Git/GitHub when
they matter. They are not cached as repository governance state.

During a long conversation, re-run this reconstruction locally when the model
starts to confuse settled constraints, current branch/PR facts, task scope, or
accepted design. Recovery is a read/reason operation, not a documentation task.

## Technical Decision Behavior

AI owns routine technical judgment. The Owner supplies goals, constraints,
unacceptable directions, product trade-offs, counterexamples, and external
commitments.

Existing design, Owner suggestions, roadmap wording, and prior model proposals
are inputs, not protected conclusions. Before committing to a consequential
technical direction, AI must ask whether the proposed direction is actually the
best fit for the project goals and current evidence, not merely whether it can
be made internally consistent.

Accepted architecture remains binding implementation authority until changed
through the applicable architecture process. Questioning an accepted design is
permitted and expected when new evidence justifies it; silently diverging from
it is not.

## Capability Allocation: Max, XHigh, and High

Model/profile selection is a compute-allocation choice inside a task session,
not a durable project role hierarchy.

### Default execution

Use High or XHigh when the technical objective and design boundary are already
clear enough to implement safely.

The implementation model owns normal local decomposition, edit order, helper
shape, debugging, testing, and evidence collection. It does not require a
separate Max-authored worker plan for routine implementation mechanics.

Use XHigh rather than High when implementation is locally difficult—for
example, cross-crate Rust ownership/lifecycle work—while the architecture and
completion boundary are already settled.

### Escalate to Max

Use Max when higher-order technical uncertainty exists, especially when:

- the next valuable engineering objective is unclear;
- competing architecture, API, ownership, lifecycle, or protocol boundaries
  must be judged;
- existing project or Owner design inclination itself needs to be challenged;
- implementation evidence falsifies an assumption behind the current design;
- a major gate, milestone, migration, or release claim requires broad judgment.

Max's job is to reduce uncertainty and establish a technical conclusion,
constraints, and falsifiable completion boundary. Max does not need to manage
High/XHigh step-by-step when ordinary implementation choices can be derived
safely by the executor.

A fixed `Max -> High -> Max` lifecycle is not required for every task.

### Independent challenge and audit

Use ChatGPT as an advisory independent challenge before implementation when a
plan is architecture-sensitive, public-API-sensitive, unusually complex, or
appears anchored to an existing design inclination that deserves attack.

Use Ultra for low-frequency repository-wide audits at major architecture,
milestone, or release boundaries, or when repeated local corrections suggest a
shared blind spot.

These viewpoints do not own project state and do not require repository role
records.

## Review and Acceptance

Review depth is proportional to semantic risk.

Ordinary local work may be accepted through the normal pull-request diff,
focused tests, CI, and relevant package evidence without a separate Max review.

Use a fresh Max context for independent acceptance when work affects one or
more of:

- public API or protocol-facing contract;
- architecture or ownership/lifecycle invariants;
- resource/progress/cancellation semantics;
- a major cross-package integration gate;
- milestone closure;
- release readiness.

The fresh reviewer reconstructs the intended result from repository authority,
the exact reviewed diff, and executable evidence. It must not assume the
implementation conversation's summary or conclusion is correct.

## Milestone Lifecycle

Milestones are defined in `PLAN.md` and may use:

    OPEN -> IN_PROGRESS -> REVIEW -> CLOSED

with `BLOCKED` or `REOPEN` when evidence requires them.

Milestone status reflects registered repository evidence, not conversational
confidence or percentage estimates. Owner approval is not required for routine
technical closure, but new Owner constraints or credible counterexamples may
reopen a conclusion.

## PLAN.md Rules

`PLAN.md` owns only durable roadmap information:

- release targets;
- milestones and objectives;
- durable dependencies and ordering;
- coarse status;
- milestone exit goals;
- a small roadmap frontier when ordering itself is a durable project fact.

It does not own:

- current task state or detailed implementation plans;
- model roles;
- session history;
- PR/branch/CI state;
- temporary blockers or debugging notes;
- architecture specifications or decision rationale;
- work-package admission/completion evidence already owned elsewhere.

The roadmap may identify a durable next dependency, but it does not authorize
or serialize one exact next action. The next engineering action is re-derived
from current repository evidence in each task session.

## Open Decision Management

Open technical investigations live in `workspace/` and progress through:

    OPEN -> DISCUSSING -> DECIDED -> MIGRATED

AI investigates the question against project goals, alternatives, code, tests,
specifications, work packages, and relevant external protocol evidence. Owner
input is evidence, not a predetermined answer.

When a decision stabilizes, migrate it to the artifact that actually owns the
truth: architecture/specification/ADR, work package, code, test, or roadmap.
Do not create a current-state document merely to repeat the result.

## Risk-Proportional Implementation Admission

Maintain strict controls where semantic risk is real, but avoid multiplying
process artifacts by task count.

- Local additive changes with settled contracts require local compile/test
  evidence and normal review.
- Cross-module contract work requires the relevant work-package, ownership,
  dependency, and conformance evidence.
- Architecture or invariant changes require the applicable workspace/design
  migration and revalidation of affected evidence.

Split work only when ownership, lifecycle, blockers, rollback, authority, or
validation truth differ materially. Do not split work merely because types or
files can be named separately.

An implementation discovery that changes semantics, ownership, lifecycle,
resources, protocol boundary, or evidence truth is a decision boundary. Stop
forcing the old assumption and escalate the decision rather than hiding it in
implementation mechanics.

## Validation Governance

Validation protects technical invariants; it is not a project-management
system.

Prefer:

- behavioral tests and realistic external fixtures;
- reusable invariant-category validators;
- machine-readable manifests for facts that genuinely need registration;
- focused negative cases for falsifiable boundaries.

Avoid new checker control flow whose only purpose is to enforce task ceremony,
conversation sequencing, model roles, branch choreography, or duplicated
current-state prose.

Add a checker only when a stable technical invariant can be violated later and
existing code/tests/generic validation cannot already detect it.

Project progress remains distinguishable across:

- architecture/authority closure;
- package-local contract completion; and
- executable vertical integration.

Evidence on one track must not be reported as completion on another.

## Git and Pull-Request Workflow

Git is the recoverable implementation history. GitHub pull requests are the
normal remote collaboration and review boundary.

For repository-changing work:

1. Work on a task branch and preserve unrelated changes.
2. Commit coherent, recoverable checkpoints.
3. Run task-specific and risk-appropriate validation.
4. Push the branch and open or update a pull request.
5. Keep durable technical rationale only in its authoritative owner.
6. Use the pull-request diff, tests, audits, and CI as review evidence.
7. Do not push task commits directly to the default branch.
8. After merge, fetch and verify the default-branch content and required CI
   before claiming dependent work is released.

A pull-request description may summarize the task goal, important constraints,
and evidence for human/reviewer convenience. It is not a parallel authoritative
state machine and need not mirror an entire conversation.

Remote facts are owned by GitHub. Historical implementation is owned by Git.
Technical admission/completion is owned by the relevant registered artifacts
and executable evidence.

## Governing Principle

Governance should make durable truth easier to find and material mistakes
harder to hide. It should not attempt to make a stateless model feel stateful.

When a proposed governance artifact mainly exists to remember what the model
was doing, prefer repository reconstruction over another layer of state.
