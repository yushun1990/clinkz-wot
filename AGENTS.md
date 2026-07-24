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

## Session Entry

Before substantial work:

1.  Read `AGENTS.md`.
2.  Read `PROJECT_STATE.md`.
3.  Identify the active milestone and objective from `PLAN.md`.
4.  Follow references to the smallest necessary subset of governance,
    specifications, workspace discussions, code, tests, audits, and
    evidence.
5.  Inspect implementation before making implementation claims.

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

## Implementation Judgment

-   Implement for realistic usage.
-   Avoid speculative abstractions.
-   Treat awkward APIs as design feedback.
-   Surface architectural problems.
-   Preserve unrelated changes.
-   Inspect code and tests before asserting behavior.

## Git Checkpoints

Create recoverable checkpoints during long sessions.

Git protects repository changes. `PROJECT_STATE.md` protects project
understanding and continuity.

Use both.
