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
  Current engineering contract       `EXECUTION.md`
  Continuation and remote cache       `PROJECT_STATE.md`

ClinkZ-WoT uses AI-led development. AI owns routine technical decision-making
and evidence closure. Owner feedback keeps the work aligned with project goals
and real-world constraints.

## Roles and Responsibilities

Technical authority is divided by capability role. The current operational
mapping is:

| Capability role | Default model/profile |
|---|---|
| Technical Lead | Max |
| Executor | High |
| Acceptance Reviewer | fresh Max context |
| Plan Challenger | ChatGPT |
| Periodic Repository Auditor | Ultra |

The mapping is an operational default, not architecture authority. It may be
updated when model capabilities change without redefining the stable roles.

### Technical Lead

The Technical Lead owns technical judgment for one cycle: architecture and API
direction, claim selection, work-package decomposition, implementation order,
risk classification, evidence sufficiency, and acceptance criteria. It writes
the Lead-owned sections of `EXECUTION.md` at an exact fetched default-branch
revision and freezes them before execution.

The Lead considers Plan Challenger feedback and incorporates accepted
corrections into the one contract. If implementation evidence falsifies the
plan, only the Lead may revise the claim, scope, constraints, or acceptance
criteria; it increments the contract revision and returns it to `PLANNED`.

### Executor

The Executor implements the frozen engineering plan, tests, and necessary
documentation; makes ordinary local implementation choices within that
contract; runs basic task-specific validation; and records an exact handoff.
It does not repeat a full architecture review and does not change the claim,
authority, scope, non-goals, or acceptance criteria.

If the real implementation exposes an unconstructible API, architecture
conflict, missing authority, invalid criterion, or material scope expansion,
the Executor preserves the smallest useful reproduction or finding, marks the
contract `BLOCKED`, and returns it to the Lead. Passing by an undocumented
workaround is not completion.

### Acceptance Reviewer

Acceptance uses a fresh context that did not implement the claim. It
reconstructs the intended result from registered authority, `EXECUTION.md`, the
exact diff, and executable evidence. It does not assume the Lead's or
Executor's summary is correct. It records concrete findings or one accepted
verdict, then handles eligible ready/merge and post-merge reconciliation.

Using Max for both planning and acceptance is permitted only with this fresh
context boundary. The model identity is not independent evidence by itself;
negative cases, public-boundary fixtures, runtime/workload observations, and
exact topology remain the material evidence.

### Plan Challenger

ChatGPT supplies an advisory independent view before execution for important
cycles: architecture-sensitive changes, public API changes, milestone or gate
transitions, release-claim changes, repeated correction patterns, or unusually
complex plans. It looks for direction drift, unnecessary complexity, and
accepted decisions that the plan does not operationalize. It does not become a
second plan owner. The Lead resolves its findings in `EXECUTION.md`; unresolved
technical concerns go to `workspace/`.

### Periodic Repository Auditor

Ultra performs low-frequency repository-wide audits at milestone closure,
major authority reset, release-candidate review, or when repeated drift shows
that local cycles may share a blind spot. It does not participate in routine
execution. Findings enter registered audits or workspace investigations and do
not silently override active authority.

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

- release targets;
- durable roadmap milestones and objectives;
- dependencies;
- milestone status;
- coarse exit goals; and
- a small current roadmap frontier when it changes durable ordering.

PLAN.md does not contain:

- session logs;
- temporary debugging information;
- the current engineering claim or implementation plan;
- task acceptance criteria or handoff state;
- detailed design discussions;
- architecture decisions;
- an open or migrated decision ledger;
- governance policies; or
- transient branch, pull-request, workflow-run, authentication, or handoff
  state used only to derive the next bounded task.

Exact package progress and evidence belong to registered work-package records,
audits, and tests. The current bounded task belongs to `EXECUTION.md`; volatile
remote observations belong to `PROJECT_STATE.md`.

## EXECUTION.md Maintenance Rules

`EXECUTION.md` is one replace-in-place contract, limited to 200 lines. It owns:

- lifecycle status and contract revision;
- exact planning base, task branch, and pull request;
- one engineering claim and its authoritative inputs;
- scope, non-goals, constraints, engineering plan, and acceptance criteria;
- the optional Plan Challenger's disposition and Lead response;
- escalation and stop conditions;
- the Executor's exact handoff and findings; and
- the fresh Acceptance Reviewer's verdict.

Lifecycle:

```text
IDLE -> PLANNED -> EXECUTING -> REVIEW_READY -> ACCEPTED
                       |              |
                       v              v
                    BLOCKED       EXECUTING
```

The Lead owns all contract sections except Executor Handoff and Acceptance
Review. The Executor may update only its handoff and permitted status fields.
The Acceptance Reviewer owns the verdict and records the exact reviewed
implementation head. A Lead revision after execution starts increments the
contract revision, explicitly identifies the changed assumption, and returns
the contract to `PLANNED` before work resumes.

Git history is the archive. Do not append completed contracts or maintain a
parallel task log. A terminal `ACCEPTED` contract or `IDLE` state authorizes no
successor; the next Lead replaces it in a new cycle.

## PROJECT_STATE.md Maintenance Rules

`PROJECT_STATE.md` is a non-authoritative continuation cache limited to 200
lines. It owns only one exact observed default revision and basis, the
established frontier, a pointer to the current execution contract, blockers or
limits, stopping point, conditional handoff actions, and a small navigation
set.

It never retains historical candidate/review/admission/merge chains, accepted
or rejected decision history, detailed architecture, test logs, or facts
already recoverable from Git, GitHub, work-package manifests, audits, specs,
or `EXECUTION.md`. Stale content is replaced, not accumulated. Remote state is
always an observation and never overrides GitHub or repository evidence.

## Open Decision Management

Open project decisions live in `workspace/` and its index. They are AI-owned
unless they depend on project goals, product trade-offs, real-world
constraints, unacceptable directions, or irreversible external commitments.

For each open technical decision, AI must:

- investigate the workspace topic and related repository evidence;
- record alternatives, selected direction, and rejected approaches;
- update or create the authoritative document, work package, code, or test that
  owns the conclusion;
- update the current execution contract or continuation cache only when their
  unique state changes; and
- keep unrelated admitted work moving when the open decision is disjoint.

Owner questions and counterexamples are evidence inputs. They are not direct
technical instructions and not predetermined conclusions.

## Role-Separated Execution Cycles

A broad `continue` or equivalent request authorizes one `EXECUTION.md` claim
from the reconciled frontier. It does not authorize every released roadmap
successor.

1. A fresh Max Lead fetches and reconciles the default branch, selects one
   coherent claim, writes `PLANNED` at an exact base, and creates or updates the
   task branch and draft pull request.
2. For an important cycle, ChatGPT challenges the plan before implementation.
   Max either revises the one contract or records why a finding does not apply.
3. High changes the contract to `EXECUTING`, implements within it, runs basic
   validation, records exact evidence and deviations, pushes the same branch,
   and leaves the pull request draft at `REVIEW_READY`.
4. A fresh Max context independently reviews the exact head. Findings return
   the contract to `EXECUTING`; a passed review records `ACCEPTED` and may
   perform eligible remote integration.
5. Max fetches the resulting default branch, verifies ancestry, expected
   content, and merge-revision validation, updates the compact continuation
   envelope as required, and stops before the next distinct claim.

Plan, implementation, and acceptance normally use the same task branch and
pull request so the contract and result cannot drift across parallel task
records. Semantically meaningful candidate, review, admission, implementation,
and evidence commits remain distinct when their registered topology requires
it.

An upstream correction stays inside the claim only when required for
constructibility and sharing its ownership, lifecycle, rollback, and evidence
truth. Otherwise High records the finding and stops. Owner input ends the
cycle only at an Owner-owned goal, constraint, unacceptable direction, public
release, or irreversible commitment. Time, tokens, lines, commits, and pull
requests are not cycle boundaries.

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

### Scalable Validation Architecture

Executable validation must scale with distinct invariant categories rather
than with the number of work-package or tranche instances. The validation
architecture has four responsibilities:

1. registered specifications, work-package records, gate manifests, and
   evidence records declare instance-specific facts;
2. one generic transition validator checks common lifecycle/status pairs,
   immutable refs, parent/tree relationships, exact path sets, expected
   absent/present boundaries, check/artifact registration, and
   attestation/evidence linkage;
3. reusable invariant-category validators check semantics such as real-value
   provenance, ownership/dependency direction, capability absence, resource
   bounds, and profile/cell parity; and
4. focused external compile, runtime, mutation, workload, or source fixtures
   prove behavior at the applicable public or protocol boundary.

Candidate and correction object ids, per-transition path arrays, precheck
lists, admission/completion refs, and evidence keys are transition data. They
remain worth validating, but normally belong in the declarative owner consumed
by the generic transition validator. A tranche-specific entry or completion
script may orchestrate the generic validator and focused behavioral evidence;
it must not become a second independent restatement of the same topology.

New `tools/design-check` control flow is admitted only when a new falsifiable
invariant category cannot be expressed by the existing generic schema and
validators. Its checkpoint must name that invariant, the defect class it
detects, why declarative validation is insufficient, and its positive and
negative evidence. Merely adding another tranche, correction ref, exact path
set, status transition, or completion record is not sufficient reason.

Existing bespoke validation is not deleted on size grounds. Migration first
runs old and generic validators as parallel oracles and proves parity across
valid state, negative mutations, commit topology, and current completion
evidence. Only then may duplicated instance-specific control flow be retired.
The measurable scaling criterion is that a new tranche using existing
invariant categories adds declarative records and focused behavior evidence
without requiring a new tranche branch in the generic engine.

### Tranche Completion and Successor Entry

A tranche completion record proves only the requirements, public contracts,
implementation paths, exclusions, feature cells, and evidence named by that
tranche. It does not implicitly prove that every possible downstream role can
be constructed or that a successor may edit source. In particular, running a
predecessor regression check proves preservation of the predecessor contract;
it is not by itself a cross-package handoff test.

Downstream progression has two separate release events:

1. predecessor completion may release successor candidate construction,
   fixtures, independent review, and pre-source transition simulation; and
2. successor source admission requires the successor's own approved pre-source
   checkpoint plus every registered entry dependency.

When a successor declares that it consumes an upstream value, capability, or
owned object, its entry evidence owns the transition proof. That evidence must
carry a real output from the registered upstream implementation into the first
legal downstream entry point, preserve the declared identity and ownership,
and reject fixture-created substitutes for either side of the handoff. If the
first downstream candidate cannot construct that proof, successor source stays
blocked and the defect is corrected in the package that owns the missing
output or adapter. The already completed local tranches remain valid unless
the finding falsifies one of their explicit completion claims.

The integration-gate manifest owns cross-package order and the exact transition
whose evidence releases source; the predecessor completion record owns only
its local claim; and the successor admission record owns source-edit authority.
`PLAN.md` may project these durable release relationships but cannot broaden
them. Terms such as "releases the next tranche" mean candidate/admission
preparation unless the registered transition explicitly says source admission.

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

The `EXECUTION.md` acceptance verdict is written by a fresh Max context that
did not implement the claim. The reviewer verifies the frozen criteria but
also reports a concrete architecture, usability, ownership, or evidence defect
when the authored checklist omitted it. Changing the claim or acceptance
boundary in response is a new Lead revision, not an acceptance-side
workaround.

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
2. update the Executor handoff and continuation state, then run the
   task-specific evidence plus the risk-appropriate default-branch matrix;
3. retain semantically necessary checkpoint boundaries instead of squashing
   immutable candidate, review, admission, implementation, or evidence
   topology into one commit;
4. commit on the current task branch and push it to `origin`;
5. open one draft pull request targeting the remote default branch, or update
   the existing pull request for that task; and
6. hand off the pull-request URL, exact commits, checks, remote workflow state,
   and known limitations to the Owner.

Under the role-separated cycle, the Executor stops at a draft pull request and
`REVIEW_READY`. The fresh Acceptance Reviewer owns the verdict and any
promotion to ready, automatic integration, or merge-revision reconciliation.

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
but cannot release dependent source work from an unverified merge.

Every pre-merge continuation checkpoint is a merge-stable envelope. It records
the exact fetched default-branch basis used for the projection and two
conditional next actions: the remaining handoff/reconciliation work while the
task content is not verified on the default branch, and the successor action
after default reachability, expected content, and merge-revision validation are
proved. An unconditional "current objective" that becomes false merely because
its own pull request merges is invalid continuation state. A local aggregate
check enforces the envelope shape and verifies that the recorded basis is a
commit reachable from the checked revision; it does not query or infer live
GitHub state.

After integration, the next repository-changing task resolves the envelope's
predicate and replaces it in its first checkpoint. A disjoint workspace-only
task may perform that repair because continuation truth travels with the first
checkpoint that relies on it; this does not change the task's technical review
claim. The project does not use a write-capable post-merge workflow or a
recursive state-only pull request.

A dependent task begins only after the merge is visible in the fetched
default branch and the default-branch validation for the merge revision
passes. This rule applies equally to manual and automatic integration.

`merged = true` is historical pull-request state, not sufficient
default-branch integration evidence. Reconciliation records and verifies:

1. the pull request's actual base and expected head;
2. whether the merge/repair commit is an ancestor of the fetched default
   branch;
3. whether the expected paths and semantic/evidence content are present and
   not subsequently reverted or superseded; and
4. whether required validation covers the applicable merge revision/current
   base rather than only an obsolete head.

A merge into a feature branch may complete a review/handoff step but releases
no default-branch dependency. A later repair pull request may be the content's
canonical integration event while preserving the earlier merge as history.
Stacked, retargeted, reverted, superseded, and repaired tasks are described by
their actual commit reachability and content, not by branch name or pull
request number alone.

`PROJECT_STATE.md` records the last fetched default-branch commit and
observation date/basis used to derive its objective. Freshness is semantic, not
commit-distance based. An intervening disjoint change may leave the objective
unchanged, but a known change to the executable objective, blocker/release
set, source-admission boundary, milestone status, or next safe action is
dangerous projection drift and is repaired in the first checkpoint before
further dependent work. Overclaims stop affected work immediately; false
blockers and impossible objectives are also defects because they repeat
completed work. A lightweight local checker may validate admission-critical
ancestry and expected content, but local correctness never depends on live
GitHub access or parsing every narrative sentence.

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

`MIGRATED` proves durable projection, not that every downstream consequence is
already implemented or empirically effective. When a migration changes future
execution behavior, its record names the displaced default, the new default,
the first claim expected to exercise it, and evidence that would falsify
adoption. Later real work supplies effectiveness evidence. If it continues the
displaced default without a justified exception, reopen the topic or create a
linked finding; do not add another lifecycle state or a checker that merely
cross-references governance prose.

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

- update the current `EXECUTION.md` handoff or verdict when applicable;
- replace `PROJECT_STATE.md` fields whose remote basis, frontier, blocker,
  stopping point, or conditional next action changed;
- ensure milestone status is accurate.

The repository must remain understandable without previous conversation
history.
