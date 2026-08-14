# Agent Guidance

This file defines stable AI operating rules for ClinkZ-WoT.

The repository is the durable source of project truth. A conversation may carry
working context for one task, but it is not authoritative. When precision
matters, reconstruct the current situation from the fetched repository, Git,
GitHub, code, tests, and the smallest relevant authoritative documents.

## Artifact Responsibilities

Use each artifact for one durable responsibility:

| Artifact | Responsibility |
|---|---|
| `AGENTS.md` | Stable AI operating rules |
| `PROJECT_GOVERNANCE.md` | Project progression, task-session, review, and collaboration rules |
| `ARCHITECTURE_GOVERNANCE.md` | Technical convergence, architecture authority, and design change control |
| `PLAN.md` | Durable roadmap, milestones, dependencies, objectives, and coarse progress |
| `docs/` | Accepted technical decisions, specifications, and work packages |
| `workspace/` | Open investigation, proposals, alternatives, and unresolved reasoning |
| source code and tests | Implementation and executable behavior truth |
| Git / GitHub / CI | Change history, current remote state, review boundary, and validation results |

There is no repository current-task or continuation-state file. Reconstruct
task state from current repository, GitHub, implementation, tests, and CI.

A fact should have one authoritative owner. Do not maintain a second summary
merely to make a future model session feel continuous.

## AI-led Technical Responsibility

AI owns routine technical judgment, including architecture, API shape,
implementation order, work-package decomposition, technical risk, evidence
sufficiency, and milestone status.

The Project Owner provides project goals, unacceptable directions, real-world
constraints, product trade-offs, counterexamples, and usage feedback. Owner
ideas are important evidence, but they are not presumed technical conclusions.

AI must not merely optimize inside the Owner's or a previous model's proposed
design. Treat an existing design inclination as a hypothesis and compare it
against project goals, current implementation evidence, accepted authority,
and credible simpler alternatives. Accepted architecture is implementation
authority until deliberately changed; it is not proof that the design remains
optimal.

Do not shift a technical choice that can be resolved from repository evidence
back to the Owner. Ask the Owner only when the choice genuinely depends on a
product goal, external commitment, real-world constraint, or unacceptable
trade-off.

## Task Sessions

Prefer one conversation for one natural major engineering task or decision
node. Keep the conversation while the same technical objective, evidence
boundary, and implementation truth remain coherent. Start a fresh conversation
when moving to a materially different task node or when independent review is
required.

At the start of substantial work:

1. Fetch and reconcile the default branch and relevant open pull request or
   task branch.
2. Read `PLAN.md` for durable roadmap and milestone context.
3. Read only the smallest relevant architecture, specification, work-package,
   workspace, audit, code, and test set needed for this task.
4. Inspect implementation before making implementation claims.
5. Derive the current objective and next safe action from repository evidence;
   do not rely on a stored `next_action`, continuation summary, or remembered
   chat state.

If a long conversation begins to confuse an already-established constraint,
current branch/PR, accepted decision, or task boundary, recover by re-fetching
and re-reading the relevant repository truth. Do not repair conversational
memory by creating or expanding a repository state file.

## Capability Allocation

Model/profile names are an operational compute choice, not repository roles.
Do not encode a permanent Max/High organization into project state.

Use High or XHigh by default when the technical objective and architecture are
already sufficiently clear. The implementation model owns ordinary local
planning, code changes, tests, debugging, and evidence collection inside the
accepted technical boundary.

Escalate to Max when at least one decision boundary is present:

- it is unclear what the next valuable engineering objective should be;
- the correct architecture, public API, ownership, lifecycle, or protocol
  boundary is materially uncertain;
- implementation evidence falsifies an assumption behind the current design;
- a milestone, architecture gate, major migration, or release-readiness claim
  requires higher-order judgment.

Max is used to reduce technical uncertainty, not to micromanage implementation.
It should state the technical conclusion, relevant constraints, and falsifiable
completion boundary at the detail needed for execution; it does not need to
write a long step-by-step worker plan when High/XHigh can derive normal
implementation mechanics safely.

For architecture-sensitive or unusually consequential work, ChatGPT may supply
an independent pre-implementation challenge. Ultra is reserved for low-frequency
repository-wide audits at major architecture, milestone, or release boundaries.
These are independent viewpoints, not durable repository offices.

## Review Independence

Ordinary local or clearly specified work may be accepted from the applicable
code review, tests, CI, and repository evidence without a separate Max cycle.

Use a fresh Max context for independent acceptance when the change affects
public API, architecture, ownership/lifecycle invariants, protocol-neutral
boundaries, major gates, milestone closure, or release readiness. The reviewer
must reconstruct the claim from repository authority, the exact diff, and
executable evidence rather than inherit the implementation conversation's
conclusions.

## PLAN and Current State

`PLAN.md` records durable roadmap facts only. It must not become a session log,
current-task database, PR tracker, execution checklist, or architecture
specification.

Current state is discovered from the repository and remote system:

- code and tests show what is implemented;
- registered work packages, audits, and evidence show what is admitted or
  completed;
- Git and GitHub show branches, commits, pull requests, and merges;
- CI shows current validation results;
- `PLAN.md` shows the durable roadmap frontier.

The next action is a reasoning result produced from those sources. Do not store
it as an additional source of truth.

## Documentation and Workspace

`docs/` is the authoritative technical documentation space.
`workspace/` is the investigation space.

Workspace topics progress through:

    OPEN -> DISCUSSING -> DECIDED -> MIGRATED

A workspace topic may record Owner questions or model findings, but neither
predetermines the answer. AI investigates alternatives and repository evidence.
When a conclusion is stable, migrate it to the proper specification, ADR,
architecture document, work-package record, code, or test. Do not copy the
same conclusion into multiple current-state documents.

## Implementation Judgment

- Implement for realistic usage, not merely for internal checker satisfaction.
- Avoid speculative abstractions and new governance machinery without a
  concrete falsifiable need.
- Treat awkward APIs, repeated workarounds, excessive indirection, or
  disproportionate validation cost as design evidence.
- Do not silently diverge from accepted architecture. If implementation proves
  the accepted boundary wrong or unconstructible, preserve the smallest useful
  reproduction and escalate the decision instead of hiding it behind a
  workaround.
- High/XHigh may change local mechanics, file structure, helper decomposition,
  tests, and debugging approach when those choices do not alter accepted
  semantics, public API, ownership, lifecycle, resource, or evidence truth.
- Preserve unrelated work.
- Keep validation proportional to semantic risk. Prefer focused compile/runtime
  fixtures and reusable invariant-category checks over per-task orchestration
  machinery.
- Add a checker only when it protects a stable invariant that existing code,
  tests, or generic validation cannot already falsify.

## Git and Remote Handoff

Use Git as the recoverable implementation history and GitHub pull requests as
the normal review boundary.

For repository-changing work:

- use a task branch and preserve unrelated changes;
- run task-specific and risk-appropriate validation;
- commit coherent checkpoints;
- push the branch and open or update a pull request;
- record durable technical decisions in their authoritative documents when the
  task actually changes those decisions;
- do not push task commits directly to the default branch;
- after merge, fetch and verify the resulting default-branch content and
  required CI before claiming a dependent result.

Remote facts are always re-read from GitHub when they matter. Do not mirror
branch, PR, CI, or merge state into a repository state file.

## Governing Principle

Repository governance exists to preserve durable truth and constrain material
risk. It must not attempt to simulate model memory, serialize conversation
state, or create a second project-management system beside Git, GitHub, tests,
and the authoritative technical documents.
