# clinkz-wot Architecture Workspace Package

This package contains the latest proposed architecture-workspace files.

Copy `workspace/` to the repository root.

Append `AGENTS-workspace-appendix.md` to the existing root `AGENTS.md`.
Do not replace the existing repository guidance.

The key boundary is:

- `workspace/`: architectural reasoning and design convergence only.
- ADR/design/specification: authoritative architecture.
- `PLAN.md`, GitHub Issues, and PRs: implementation execution and tracking.

Suggested flow:

workspace topic
    -> discussion
    -> DECIDED
    -> ADR / design / specification
    -> MIGRATED
    -> implementation planning
