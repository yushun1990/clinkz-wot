# Agent Guidance

This file defines how AI agents work in this repository.

The repository, not a conversation, is the durable carrier of project
continuity. Important project understanding, execution context,
blockers, and intended next work must not exist only in chat history.

## Authority

Use each repository artifact for its intended purpose:

  -----------------------------------------------------------------------
  Artifact                            Responsibility
  ----------------------------------- -----------------------------------
  `AGENTS.md`                         Stable AI operating rules

  `PROJECT_GOVERNANCE.md`             Project execution governance,
                                      collaboration process, milestone
                                      lifecycle, and review workflow

  `ARCHITECTURE_GOVERNANCE.md`        Technical convergence rules,
                                      architecture authority, frozen
                                      direction, and design change
                                      control

  `PROJECT_STATE.md`                  AI-maintained continuation state

  `PLAN.md`                           Project roadmap, milestones,
                                      objectives, dependencies, and
                                      progress state

  `docs/`                             Authoritative specifications and
                                      accepted decisions

  `workspace/`                        Active discussion, investigation,
                                      proposals, and unresolved reasoning

  source code and tests               Implementation truth

  Git history                         Change history and recoverable
                                      checkpoints
  -----------------------------------------------------------------------

When sources conflict, identify the conflict and resolve it according to
artifact ownership.

## AI-led Development Model

ClinkZ-WoT uses AI-led development.

AI agents hold primary technical decision responsibility for:

-   technical architecture;
-   public and internal API shape;
-   work-package decomposition;
-   implementation order;
-   technical risk assessment;
-   evidence sufficiency;
-   technical milestone status.

The Project Owner maintains project vision, goals, real-world constraints,
unacceptable directions, and product or usage feedback.

Owner feedback may appear as questions, counterexamples, concerns, or usage
experience reports in `workspace/`. Such feedback is input for AI
investigation. It is not a preset technical answer, not an implementation
instruction by itself, and not an automatic blocker for unrelated work.

AI agents must investigate Owner-raised topics against repository evidence:
architecture, code, tests, specifications, work packages, audits, and review
records. AI agents are responsible for deciding the technical direction,
recording the rationale, migrating stable conclusions to the proper
authoritative owner, and validating the result.

AI agents must not shift technical judgment that can be resolved from project
evidence onto the Project Owner. Ask the Owner for clarification only when a
choice depends on project goals, product trade-offs, real-world constraints,
unacceptable directions, or irreversible external commitments rather than
technical evidence.

Technical milestones are closed by AI from registered exit criteria and
repository evidence. A later Owner-provided goal conflict, missing constraint,
or credible counterexample may reopen a milestone or decision.

AI determines technical release readiness from evidence. The Project Owner
decides whether and when to execute an actual public release or other
irreversible external commitment.

## Session Entry

Before substantial work:

1.  Read `AGENTS.md`.
2.  Read `PROJECT_STATE.md`.
3.  When remote integration can affect the next safe action, fetch the remote
    default branch when available and reconcile its pull-request/merge state
    with `PROJECT_STATE.md` before relying on the recorded objective. A merged
    pull request counts as default-branch integration only after checking its
    actual base, merge ancestry from the fetched default branch, expected
    repository content, and applicable merge-revision validation. Offline work
    may use the last observed snapshot but may not release a dependent source
    transition from an unverified remote merge.
4.  Identify the active milestone and objective from `PLAN.md`.
5.  Follow references to the smallest necessary subset of governance,
    specifications, workspace discussions, code, tests, audits, and
    evidence.
6.  Inspect implementation before making implementation claims.

## Durable Continuation

`PROJECT_STATE.md` is the AI-owned continuation checkpoint.

It should allow a fresh agent without previous conversation history to
recover:

-   current project objective;
-   active milestone and work item;
-   relevant architecture understanding;
-   accepted decisions;
-   unresolved questions;
-   rejected approaches;
-   blockers;
-   stopping point;
-   next safe actions;
-   verification references.

`PROJECT_STATE.md` is curated memory, not a session transcript.

Rules:

-   Replace stale information instead of accumulating history.
-   Separate facts from assumptions.
-   Preserve reasoning needed for future decisions.
-   Do not duplicate authoritative specifications.
-   Do not store important knowledge only in chat history.
-   Record the exact fetched default-branch revision used to derive the
    continuation projection.
-   During remote handoff, record both conditional next actions: what remains
    before verified default-branch integration and what becomes next after
    integration plus merge-revision validation. Do not use an unconditional
    objective that becomes false merely because its own pull request merges.

## Continuous Checkpointing

Update `PROJECT_STATE.md` whenever substantial understanding or
execution state changes.

Examples:

-   architecture analysis;
-   design direction selection;
-   blocker discovery;
-   rejected approaches;
-   meaningful code, test, documentation, or review completion;
-   milestone transition.

Before starting another major task:

> If this conversation ended now, could a fresh agent continue correctly
> from the repository?

If not, checkpoint first.

## Autonomous Review Cycles

A broad progression request such as `continue` or `continue progressing`
authorizes one coherent review cycle from the current next safe action. AI
chooses the technical decomposition; the Owner does not need to name the
tranche or pre-compute the stopping point.

One review cycle establishes one independently reviewable engineering claim.
Analysis, decision migration, candidate construction, independent review,
admission, implementation, completion evidence, remote handoff, integration,
and reconciliation may remain in that cycle when they share the same contract,
rollback boundary, and evidence truth. A necessary upstream correction remains
inside the cycle only when it is required to make that same claim constructible
and shares those boundaries.

The cycle ends at the first stable new fact that can be reviewed independently,
including a completed tranche or equivalent claim, a proved correction of a
blocking defect, a material gate transition, or a next safe action that crosses
a different package, ownership, lifecycle, public-contract, rollback, or
evidence boundary. An Owner-owned goal, real-world constraint, unacceptable
direction, public release, or other irreversible commitment also ends the
cycle.

At the boundary, update `PROJECT_STATE.md` with the established fact, exact
observed remote basis, remaining limitations, stopping point, and next safe
action. Complete the current claim's required checkpoint and remote handoff
when available, then identify but do not begin the materially distinct
successor's candidate, review, admission, or source work. Returning to the
Owner is a visibility handoff, not a routine technical approval gate; a new
progression request starts the next cycle.

If remote integration or reconciliation is unavailable or still pending, a
recoverable remote handoff plus the conditional continuation envelope is a
valid stopping point. A later broad progression request may finish that
reconciliation, but must not silently expand into the already identified
successor claim. An explicit request that names a broader endpoint or multiple
claims may authorize a correspondingly bounded cycle. Time, token, line,
commit, and pull-request counts do not define the boundary.

## Governance and Planning Separation

ClinkZ-WoT separates execution governance from technical governance.

### PROJECT_GOVERNANCE.md

Defines how the project progresses:

-   milestone lifecycle;
-   review workflow;
-   owner and AI responsibilities;
-   execution process;
-   progress tracking rules.

### ARCHITECTURE_GOVERNANCE.md

Defines how technical direction remains consistent:

-   architecture authority;
-   active architecture target;
-   frozen design direction;
-   convergence criteria;
-   design change control.

### PLAN.md

Defines what the project intends to achieve:

-   roadmap;
-   milestones;
-   objectives;
-   dependencies;
-   milestone status;
-   acceptance goals.

PLAN.md must not become a session log, architecture specification, ADR
replacement, or governance policy document.

## Documentation and Workspace

`docs/` is the authoritative specification space.

`workspace/` records discussion rather than specification.

Workspace contains:

-   questions;
-   proposals;
-   investigations;
-   alternatives;
-   reasoning history.

Docs contain:

-   accepted decisions;
-   specifications;
-   stable architecture;
-   formal records.

Workspace topics progress through:

    OPEN -> DISCUSSING -> DECIDED -> MIGRATED

Either the Owner or AI may open a workspace topic. AI is responsible for
investigation and for moving the topic to `DECIDED` when repository evidence
supports a conclusion. When the conclusion is projected into the proper
authoritative document, work package, code, or test, the topic becomes
`MIGRATED`.

Workspace topics must not be treated as Owner instructions or predetermined
technical conclusions. If the Owner later provides a new project constraint,
goal conflict, or credible counterexample, AI should reopen the topic or create
a new linked topic and re-evaluate the migrated conclusion.

## Implementation Judgment

-   Implement for realistic usage.
-   Avoid speculative abstractions.
-   Treat awkward APIs as design feedback.
-   Surface architectural problems.
-   Preserve unrelated changes.
-   Inspect code and tests before asserting behavior.
-   Apply risk-proportional implementation admission. Keep strict controls for
    ownership, lifecycle, resource, time, protocol-boundary, and cross-module
    changes; keep local additive work narrow when its authoritative contract,
    dependencies, disjointness, and local evidence are already clear.
-   Split a tranche only when blockers, ownership, lifecycle, contracts,
    validation independence, rollback boundaries, or evidence truth differ.
    Do not split work merely because each type or file can be named.
-   Treat tranche completion as a package-local claim unless a registered
    integration owner says otherwise. Completion may release successor
    candidate and review preparation, but successor source admission requires
    its own approved checkpoint and, for a declared cross-package handoff,
    evidence that a real upstream output reaches the first legal downstream
    entry without a fixture-owned substitute.
-   Keep instance-specific candidate refs, path sets, transition topology,
    evidence links, and status bookkeeping declarative in their authoritative
    manifests. New custom checker control flow is admitted only for a genuinely
    new falsifiable invariant category that generic validation cannot express,
    with the reason recorded. Focused compile/runtime fixtures continue to own
    behavioral evidence.
-   Migrate existing tranche-specific validation only after the generic path
    matches its positive, negative, topology, and current-evidence coverage;
    until then, retain the existing checker as an oracle rather than weakening
    evidence to reduce code volume.

## Git Checkpoints

Create recoverable checkpoints during long sessions.

Git protects repository changes. `PROJECT_STATE.md` protects project
understanding and continuity.

Use both.

## Remote Task Handoff

Every bounded task that changes the repository ends with an automatic remote
handoff. The AI agent must:

-   preserve unrelated work and commit only the task's intended scope;
-   update `PROJECT_STATE.md` before the task's final evidence-topology commit
    when that topology requires an exact path set;
-   run the task-specific checks and the risk-appropriate default-branch
    matrix;
-   commit the completed task on its task branch;
-   push that branch without waiting for a separate Owner prompt;
-   open or update one draft pull request targeting the repository default
    branch; and
-   report the branch, commits, pull-request URL, validation state, and any
    remaining limitation.

Never push task commits directly to the default branch. Follow-up fixes for
the same task update the same branch and pull request. A dependent task waits
for remote integration and the passing default-branch workflow unless the
Owner explicitly requests stacked work; disjoint preparation may continue but
cannot claim the dependent task's progress.

After handoff, AI may mark the pull request ready and enable GitHub native
auto-merge only when the exact current head satisfies all of these conditions:

-   the intended diff is complete and contains no unrelated work;
-   all applicable candidate, independent-review, admission, completion,
    workload, release, and removal evidence is current;
-   task-specific local checks and the required remote `validation` job cover
    that head;
-   the branch is current with the default branch, conflict-free, not a
    dependent stack, and has no unresolved conversation or requested change;
-   no unresolved Owner-owned goal, product trade-off, unacceptable direction,
    public release, or other external commitment is crossed; and
-   the active remote ruleset is verified to require strict current-base
    validation and conversation resolution.

Eligible automatic integration uses a merge commit and an expected head object
id. Do not squash or rebase semantically meaningful candidate, review,
admission, implementation, or evidence commits. A later commit requires the
eligibility predicate and all applicable checks to be rerun. Failed,
cancelled, stale, missing, or superseded checks, merge conflicts, or stacked
dependencies leave auto-merge disabled. Until the remote ruleset prerequisites
are verified, keep the pull request draft.

Remote Owner review is a collaboration and integration boundary. It does not
replace AI-owned technical judgment, registered independent review, or
repository evidence. If authentication, network access, or the remote is
unavailable, keep a local Git checkpoint, record the blocker in
`PROJECT_STATE.md`, and do not claim that remote handoff is complete.

After any manual or automatic merge, fetch the default branch and reconcile
the pull request, merge revision, and default-branch workflow before starting
dependent source work. A merge into another task branch, reverted content, or
superseded head does not release a default-branch dependency merely because
GitHub reports `merged = true`; a later repair commit may be the canonical
content-integration event. GitHub owns remote draft/check/merge facts;
repository commits, registered work-package state, audits, and evidence own
technical admission and completion truth. `PROJECT_STATE.md` records its last
observed fetched-default basis and never overrides either source.
