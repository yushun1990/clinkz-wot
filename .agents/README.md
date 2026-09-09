# Optional Agent Workflow Adapters

This directory contains optional, repository-scoped workflow helpers for AI
development tools that support the open Agent Skills format.

These files are **not project authority** and do not replace `AGENTS.md`,
`PROJECT_GOVERNANCE.md`, `ARCHITECTURE_GOVERNANCE.md`, `PLAN.md`, accepted
technical documents, source code, tests, Git, GitHub, or CI.

The project deliberately remains tool- and provider-neutral:

- the Project Owner chooses the model, provider, reasoning level, and execution
  surface for each task;
- no skill assigns a permanent model role or requires Codex specifically;
- an agent that does not support these skills should follow the repository
  governance directly;
- a skill may structure one session, but it must never become a second source
  of project state or technical authority.

The skills here encode only repeatable process boundaries that already follow
from repository governance:

- `clinkz-authority-audit`: investigate and converge an architecture/authority
  question before implementation;
- `clinkz-implement-frozen`: implement inside an accepted technical boundary
  without silently redesigning it;
- `clinkz-gate-review`: independently review an exact candidate against
  repository authority and executable evidence.

Use them explicitly when useful. Do not force every task through all three
skills, and do not treat skill invocation as evidence that a gate, milestone,
or design has been accepted.
