# Agent Guidance

This file defines how AI agents work in this repository.

The repository, not a conversation, is the durable carrier of project
continuity. A conversation may end at any time. Important project
understanding, execution context, blockers, and intended next work must not
exist only in chat history.

## Authority

Use each repository artifact for its intended purpose:

| Artifact | Responsibility |
|---|---|
| `AGENTS.md` | Stable agent operating rules |
| `PROJECT_STATE.md` | AI-maintained continuation state |
| `PLAN.md` | Stable execution direction and work ordering |
| `docs/` | Authoritative specifications and accepted decisions |
| `workspace/` | Active discussion, investigation, and unresolved reasoning |
| source code and tests | Implementation truth |
| Git history | Change history and recoverable checkpoints |

When these sources conflict, do not silently choose an interpretation. Identify
the conflict and resolve it against the appropriate authority.

## Session Entry

Before substantial work:

1. Read `AGENTS.md`.
2. Read `PROJECT_STATE.md`.
3. Decide whether the state is sufficient and current enough for the requested
   work.
4. Follow the state file's references to the smallest necessary subset of
   `PLAN.md`, `workspace/`, `docs/`, code, tests, audits, and evidence.
5. Inspect the implementation before making implementation claims.

Do not reread the entire documentation set merely to reconstruct project
context that is already coherently maintained in `PROJECT_STATE.md`.

## Durable Continuation

`PROJECT_STATE.md` is the AI-owned continuation checkpoint for this repository.

Its purpose is to transfer continuity from temporary conversations into the
project itself. A fresh agent without access to previous conversations should
be able to use it to recover the current project understanding, identify the
correct next work, and avoid repeating completed analysis.

The agent owns both the structure and contents of `PROJECT_STATE.md`. No fixed
schema is required. The agent may reorganize, compress, expand, or remove
sections as the project evolves.

The state should contain whatever is needed for fast and correct continuation,
including, when useful:

- the current project phase and active objective;
- the agent's compact working model of the relevant architecture;
- the active work item and why it is next;
- completed analysis that should not be repeated;
- active blockers, uncertainties, and rejected paths;
- distinctions between accepted decisions and unresolved discussion;
- the current implementation or documentation stopping point;
- unverified or incomplete changes;
- the next safe and useful actions;
- references to the authoritative files needed for targeted verification;
- the commit or repository condition against which important state was checked.

These are examples, not mandatory headings.

## State Trust and Validation

Trust `PROJECT_STATE.md` by default as the primary continuation context.

Do not perform a full repository audit at every session. Validate only the
parts required by the current work, especially when:

- the recorded commit materially differs from the current repository;
- referenced files have changed;
- code, tests, or authoritative documents contradict the state;
- the state marks information as uncertain, stale, or unverified;
- the task changes public APIs, architecture, lifecycle, ownership, resource
  guarantees, admission conditions, or release evidence.

When validation changes the understanding, update `PROJECT_STATE.md`
immediately.

A state checkpoint is effective only when a fresh agent can answer, without the
previous conversation:

- What is the project currently trying to accomplish?
- What work is active, and why is it the correct next work?
- What has already been established and should not be rediscovered?
- What remains unresolved, blocked, or unverified?
- Where did work stop?
- What should happen next?
- Which exact files must be read before acting?

If the state cannot support correct continuation, repair it before substantial
work continues.

## Continuous Checkpointing

Do not wait for a normal session ending. Sessions may be interrupted.

Update `PROJECT_STATE.md` whenever substantial understanding or execution state
changes, including after:

- a complex documentation or architecture analysis;
- selection of a work item or implementation direction;
- discovery or removal of a blocker;
- rejection of a previously plausible approach;
- completion of a meaningful code, test, documentation, or review tranche;
- transition to a different problem;
- any point where losing the current conversation would force significant
  rediscovery.

Before moving into another substantial unit of work, ask:

> If this conversation ended now, could a fresh agent continue correctly from
> the repository?

If not, checkpoint the state first.

## State Writing Rules

`PROJECT_STATE.md` is curated memory, not a session transcript.

- Replace stale state instead of appending an endless diary.
- Preserve useful explanations, not only labels and links.
- Record verified facts as facts.
- Mark uncertainty explicitly.
- Cite repository paths for important conclusions.
- Keep enough reasoning to explain why current and next work are appropriate.
- Do not copy full specifications when a compact working model and source
  references are sufficient.
- Do not use the state file as an authoritative replacement for `docs/`.
- Do not store important continuation context only in commit messages or chat.

If `PROJECT_STATE.md` is absent, empty, or marked uninitialized, the first
substantial agent must reconstruct the current project state from `PLAN.md`,
the architecture and ADR indexes, active workspace topics, work-package
metadata, implementation, tests, audits, and evidence. It must then initialize
the state before beginning new substantial work.

## Plan Usage

`PLAN.md` defines stable execution direction:

- the active target;
- execution principles and admission rules;
- durable milestones;
- high-level work ordering;
- locations of authoritative machine-readable planning data.

It must not become a session log or duplicate the live continuation state.

Current blockers, active work, recent progress, exact stopping points, and next
session actions belong in `PROJECT_STATE.md`.

Authoritative package dependencies, tranche admission, evidence requirements,
and completion contracts remain in their registered documents and
machine-readable indexes.

## Documentation and Workspace

`docs/` is the authoritative specification.

`workspace/` records discussion rather than specification. It may contain
architecture questions, process improvements, implementation strategy,
engineering trade-offs, design reviews, experiments, or other material that
helps the project converge.

The distinction is:

- `workspace/` contains unresolved questions, proposals, investigations, alternatives, and reasoning history.
- `docs/` contains accepted knowledge, stable decisions, formal reviews, specifications, and project records.

A workspace artifact exists because the project is still deciding something.

A documentation artifact exists because the project already knows something.

Workspace topics should progress through:

`OPEN -> DISCUSSING -> DECIDED -> MIGRATED`

When a conclusion becomes stable and affects the project, migrate it into the
appropriate authoritative location, such as `docs/`, an ADR, `PLAN.md`, or
`AGENTS.md`. Keep the discussion history in `workspace/`.

Update `PROJECT_STATE.md` when a workspace topic changes the active project
understanding or execution path.

## Implementation Judgment

- Implement for realistic usage.
- Avoid speculative abstractions.
- Treat awkward APIs as design feedback.
- Surface architectural problems instead of hiding them with temporary
  workarounds.
- Prefer solving the real problem over satisfying a task mechanically.
- Preserve unrelated user changes.
- Inspect current code and tests before asserting implementation behavior.

## Git Checkpoints

Create small recoverable Git checkpoints during long implementation sessions.

Git checkpoints protect repository changes. `PROJECT_STATE.md` protects project
understanding and execution continuity. Use both.
