# 0051 PROJECT_STATE Merge-Transition Drift

Status: OPEN

Kind: owner review question

## Observation

A repository-changing task can reach technical completion on a task branch,
pass pull-request validation, merge into the default branch, and release a new
next objective. During that transition, the `PROJECT_STATE.md` present on the
default branch may still describe the pre-merge remote handoff as the current
objective, while the updated continuation state exists only on a later task
branch or pull request.

## Questions

- What exact truth is `PROJECT_STATE.md` intended to own between local technical
  completion, pull-request integration, merge-revision validation, and the
  first checkpoint of the next task?
- Can one continuation record remain accurate both before and immediately after
  its associated pull request merges?
- Which artifact owns the current objective during the interval in which GitHub
  reports a completed merge but the default branch still contains the prior
  pre-merge continuation text?
- How should a fresh AI session distinguish a stale current-objective statement
  from an unfinished remote reconciliation action?
- Can the current continuation model cause a fresh session to repeat an already
  completed handoff, delay the next task, or derive an obsolete blocker?
- Does the automatic remote handoff workflow guarantee that the default branch
  receives a current continuation record after merge, or does it rely on the
  next task to replace the prior state?
- Which pull-request facts belong only to GitHub, and which transition facts
  must remain represented in `PROJECT_STATE.md` for offline or fresh-session
  recovery?
- Is the current distinction among GitHub remote truth, repository technical
  truth, and `PROJECT_STATE.md` projection sufficient for the merge-transition
  interval observed in recent Property Read work?
