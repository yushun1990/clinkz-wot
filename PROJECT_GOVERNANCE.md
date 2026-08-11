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
- governance policies; or
- transient branch, pull-request, workflow-run, authentication, or handoff
  state used only to derive the next bounded task.

`PLAN.md` may name an immutable evidence checkpoint when it is part of a
milestone or package acceptance fact. Volatile remote observations and the
currently selected bounded task belong in `PROJECT_STATE.md`.

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

## Autonomous Review Cycles

This section uses **review cycle** for the Owner-visibility cadence of broad
autonomous progression. It does not replace or rename an independent technical
review, candidate review, or milestone `REVIEW` state.

A broad request such as `continue`, `continue progressing`, or an equivalent
instruction with no fixed endpoint authorizes one review cycle beginning at
the next safe action recorded in `PROJECT_STATE.md`. AI remains responsible for
selecting and decomposing the technical claim. The semantic unit is one
coherent, independently reviewable engineering claim, not a work-package,
session, elapsed-time, token, line, commit, or pull-request quota.

Activities stay inside one cycle when they establish the same claim and share
its authoritative contract, rollback boundary, and evidence truth. This may
include investigation, authoritative migration, candidate construction,
independent review, pre-source admission, implementation, completion evidence,
automatic remote handoff, integration, and reconciliation. A discovered
upstream defect may be corrected in the same cycle only when the correction is
necessary to make the current claim constructible and shares its ownership,
lifecycle, rollback, and validation boundary. Otherwise it is a separate
claim and the current cycle ends after recording the finding and safe next
action.

The first stable repository fact that is independently reviewable ends the
cycle when the successor crosses a materially different boundary. Such facts
include:

- completion of one tranche or equivalently scoped claim;
- proved correction of a blocking architecture or handoff defect;
- a material admission, integration, or architecture-gate transition that
  releases different work;
- a next safe action with different package, ownership, lifecycle,
  public-contract, rollback, or evidence truth; or
- evidence requiring Owner input on goals, real-world constraints,
  unacceptable directions, public release, or another irreversible external
  commitment.

Before returning visibility, AI completes the current claim's recoverable
checkpoint and required remote handoff when available, and updates
`PROJECT_STATE.md` with the exact established fact, fetched-default basis,
remote/evidence limits, stopping point, and conditional next actions. It names
but does not begin a materially distinct successor's candidate, review,
admission, implementation, or source preparation. Disjoint preparation after
the boundary is limited to recording that it is safe; materially beginning it
requires a new explicit request.

The boundary is not an Owner approval gate. The repository remains technically
ready to continue, and a later broad progression request authorizes the next
cycle. If remote integration or reconciliation is pending or unavailable, a
complete draft handoff plus its merge-stable continuation envelope is itself a
stable stopping point. A later cycle may finish that reconciliation, but does
not inherit authority to start the already identified successor unless the
Owner's request also scopes it. A request that explicitly names multiple
claims or a broader terminal fact overrides the single-claim default only to
that stated bound.

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
