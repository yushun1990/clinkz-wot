# Agent Guidance

This file defines how AI agents should work in this repository.

## Working Agreement

- Read the current repository state before making architectural assumptions.
- Preserve unrelated user changes.
- Raise conflicts instead of silently choosing one interpretation.

## Session Workflow

Before substantial work:

1. Read `PLAN.md` for current execution status.
2. Read relevant authoritative documents under `docs/`.
3. Read relevant topics under `workspace/`.
4. Inspect the existing implementation.

`docs/` is the authoritative specification.

`workspace/` is the discussion area for project evolution. It may contain
architecture questions, development-process improvements, implementation
strategy, engineering trade-offs, design reviews, experiments, or any other
topic that helps the project converge. It is intentionally broader than
architecture alone.

## Implementation Judgment

- Implement for realistic usage.
- Avoid speculative abstractions.
- Treat awkward APIs as design feedback.
- Surface architectural problems instead of hiding them with temporary
  workarounds.
- Prefer solving the real problem over satisfying the current task literally.

## Workspace Workflow

The `workspace/` directory records discussions rather than specifications.

Typical topics include:

- Architecture and ownership
- API and SPI design
- Development workflow
- AI collaboration strategy
- Engineering trade-offs
- Implementation readiness
- Review notes
- Long-term ideas

Each topic should progress through:

OPEN → DISCUSSING → DECIDED → MIGRATED (when applicable)

Not every topic must become an ADR. Some discussions may simply conclude with a
project policy or a development guideline.

When a stable conclusion affects the project, migrate it into the appropriate
authoritative location (`docs/`, `AGENTS.md`, ADRs, etc.) and leave the
discussion history in `workspace/`.

## Git Checkpoints

Create small recoverable checkpoints during long implementation sessions.
