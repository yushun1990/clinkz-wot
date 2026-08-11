# 0054 Autonomous Progression Review-Cycle Boundary

Status: MIGRATED

Kind: owner-raised execution-governance investigation

Priority: HIGH

Target: the stopping and Owner-visibility boundary for long-running autonomous AI progression

## Scope and authority

This topic records a Project Owner concern about how an AI agent should decide when to stop after a broad instruction such as `continue`, `continue progressing`, or an equivalent request that does not name a fixed technical endpoint.

The concern is not a request to weaken AI-led technical autonomy, to require Owner approval for routine technical decisions, or to impose arbitrary time, token, commit, pull-request, or line-count quotas. AI remains responsible for work-package decomposition, implementation order, technical risk, evidence sufficiency, and technical milestone judgment.

The question is narrower: after receiving a broad progression instruction, what repository-visible event should end the current autonomous progression cycle and return visibility to the Owner before the agent starts another materially distinct engineering claim?

## Owner observation

The Project Owner reports that a broad `continue progressing` instruction can cause Codex to keep advancing through successive released tasks for a very long session, including candidate/review/admission/implementation/completion/integration transitions, until the session is manually interrupted or practical usage limits are exhausted.

The Owner does not know in advance how many sessions a complex work package such as WP-400 should require and should not be made responsible for pre-decomposing technical work merely to create stopping points. The current collaboration pattern is instead:

1. Codex advances the project autonomously;
2. after a meaningful body of progress, the Owner obtains an independent project-level assessment of the resulting repository state;
3. if no goal conflict, unacceptable direction, or credible counterexample is found, the Owner asks Codex to continue.

The repository therefore needs to determine whether it has an explicit stopping boundary that supports this review cadence without turning the Owner into a routine technical approval gate.

## Repository observations

The current operating model already establishes that:

- AI owns technical decomposition and implementation order;
- `PROJECT_STATE.md` must retain a stopping point and next safe actions;
- every bounded repository-changing task has an automatic remote handoff;
- dependent work may begin after verified predecessor integration;
- after merge reconciliation, a newly released next safe action may immediately exist;
- Owner visibility is non-blocking by default and does not replace AI-owned technical review;
- the repository does not currently state that completion of one meaningful engineering claim must end a broad autonomous progression instruction before a materially distinct successor claim begins.

This creates a possible distinction between **task autonomy** and **roadmap autonomy** that is not currently explicit.

## Questions for investigation

1. What repository-visible event should end one autonomous progression cycle after a broad Owner instruction such as `continue`?
2. Should the boundary be defined by a meaningful engineering claim rather than by work-package number, commit count, pull-request count, elapsed time, or token usage?
3. Which events are sufficiently material to require an Owner-visibility handoff even when no Owner approval is technically required?
4. Should completion of a tranche, closure of a newly discovered architecture defect, passage of an integration gate, or crossing a package/ownership/lifecycle boundary normally end the current progression cycle?
5. When candidate, review, admission, implementation, completion, and remote integration all belong to one technical claim, should they remain inside one cycle?
6. When fixing an upstream defect is required to make the current downstream claim constructible, when does that correction remain inside the same cycle and when does it establish a new independently reviewable result?
7. After a cycle boundary, should the agent update `PROJECT_STATE.md` with the next safe action and stop without beginning that successor's candidate, review, admission, or source work until a new Owner progression instruction?
8. How should the rule preserve disjoint preparation, automatic remote handoff, and AI-owned technical judgment without creating ceremonial Owner approvals?
9. What wording should define broad instructions such as `continue`, `continue progressing`, and equivalent phrases so that their scope is recoverable by a fresh agent?
10. What evidence would demonstrate that the chosen cadence is neither too coarse to preserve Owner visibility nor so fine-grained that it serializes normal technical work?

## Candidate review-boundary model for investigation

A possible model to test is a **review cycle** rather than a fixed task-count or time budget.

A review cycle would start from the current next safe action and allow the AI to perform all technically necessary work for one coherent engineering claim, including analysis, workspace investigation, candidate construction, independent review, admission, implementation, validation, completion evidence, remote handoff, integration, and reconciliation when those activities share the same contract and evidence truth.

The cycle would end at the first stable point where the repository can state a new independently reviewable technical fact, for example:

- one tranche or similarly scoped engineering claim is complete;
- one previously blocking architecture or handoff defect is corrected and proved;
- one integration or architecture gate changes state in a way that releases materially different work;
- the next safe action crosses a materially different package, ownership, lifecycle, public-contract, rollback, or evidence boundary; or
- new evidence requires Owner input because it affects project goals, real-world constraints, unacceptable directions, or irreversible commitments.

At that point the agent would checkpoint the repository, identify but not begin the next materially distinct safe action, report the new state and remaining limitations, and stop. A new Owner `continue` instruction would authorize the next review cycle rather than indefinite roadmap execution.

This model is an investigation input, not a predetermined governance decision.

## Constraints

- Preserve AI-led technical decision-making.
- Do not make the Owner responsible for decomposing work packages or choosing technical tranche boundaries.
- Do not introduce routine Owner approval for candidate, review, admission, implementation, completion, or merge transitions.
- Do not use elapsed time, token consumption, line count, commit count, or pull-request count as the primary stopping rule.
- Do not force one whole WP or milestone to fit inside one cycle.
- Do not force every commit or governance checkpoint to become a separate cycle.
- Preserve automatic remote handoff and recoverable continuation.
- Preserve the ability to finish a coherent correction that is necessary to establish the current engineering claim.
- The stopping boundary must be expressible from repository state so that a fresh agent can apply it without relying on chat history.

## Expected decision output

Codex should determine:

1. whether the current governance lacks an explicit autonomous-progression stopping boundary;
2. the semantic unit, if any, that should define one Owner-visible review cycle;
3. which transitions remain inside one cycle and which establish a new cycle boundary;
4. how broad progression instructions are interpreted;
5. which authoritative owner should carry the stable rule, likely `AGENTS.md` and/or `PROJECT_GOVERNANCE.md`;
6. what `PROJECT_STATE.md` must record at a cycle boundary; and
7. how the rule interacts with remote handoff, automatic integration, successor release, and disjoint preparation.

## Decision

The concern is supported. Existing governance defines how a bounded task is
reviewed, handed off, integrated, and reconciled, but did not state that a
broad progression instruction ends before a materially distinct successor
claim. A valid next safe action could therefore silently expand one request
into indefinite roadmap execution.

A broad `continue`, `continue progressing`, or equivalent instruction now
authorizes one **autonomous review cycle** from the current next safe action.
The semantic unit is one coherent, independently reviewable engineering claim,
selected and decomposed by AI. Candidate construction, independent review,
admission, implementation, completion evidence, remote handoff, integration,
and reconciliation may remain together when they share the same contract,
rollback boundary, and evidence truth.

A required upstream correction remains in the same cycle only when it makes
that same claim constructible and shares its ownership, lifecycle, rollback,
and validation boundary. Otherwise the discovered defect is recorded as the
next distinct claim and the current cycle stops at the stable finding.

The cycle ends at the first stable independently reviewable result whose next
safe action crosses a materially different package, ownership, lifecycle,
public-contract, rollback, or evidence boundary. Tranche completion, a proved
blocking correction, or a material gate transition are typical boundaries. An
Owner-owned goal, real-world constraint, unacceptable direction, public
release, or irreversible commitment also ends the cycle.

At the boundary, AI records the established fact, fetched-default basis,
remaining limits, stopping point, and conditional next safe actions in
`PROJECT_STATE.md`, completes the current claim's recoverable checkpoint and
remote handoff when available, and does not begin the successor's candidate,
review, admission, implementation, or source preparation. This returns
visibility; it does not make the Owner a technical approval gate. A new broad
progression request starts the next cycle.

When remote integration is pending or unavailable, a complete draft handoff
plus a merge-stable continuation envelope is a valid boundary. A later request
may reconcile that claim, but does not silently authorize the already named
successor. An explicit request naming multiple claims or a broader endpoint
may set a wider bound. Time, tokens, lines, commits, and pull requests are not
the primary rule.

## Migration

The stable operating rule is migrated to `AGENTS.md`; detailed execution and
remote-boundary semantics are migrated to `PROJECT_GOVERNANCE.md`; PLAN
decision D47 records the roadmap-wide consequence; and `PROJECT_STATE.md`
records the current boundary and next distinct claim.

This decision packet is itself the first applied boundary: resolving and
migrating issues 0054 and reopened 0020, validating them, and publishing one
draft pull request is the current review cycle. The generic validation
convergence required by D48 is identified but not begun so the Owner can
evaluate this stable result.
