# 0051 PROJECT_STATE Merge-Transition Drift

Status: MIGRATED

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

## Decision

The authority split remains correct: GitHub owns live pull-request, check, and
merge facts; Git history and registered evidence own technical state; and
`PROJECT_STATE.md` owns a dated continuation projection. No static repository
file can truthfully own a live merge fact that changes after its commit.

The defect is the shape and enforcement of the projection. The prior default
branch contained an unconditional `Current Objective` that became impossible
as soon as its own handoff merged, even though governance already required two
conditional next actions. Behavioral reconciliation alone did not prevent the
recurrence.

Every handoff checkpoint must therefore use a merge-stable continuation
envelope containing one exact fetched-default basis, the action that remains
while the task is not verified on the default branch, and the successor action
after default reachability, expected content, and merge-revision validation
are proved. A fresh session resolves that predicate from Git/GitHub truth before
relying on either branch. The next repository-changing task replaces the
resolved envelope in its first checkpoint; no post-merge bot, recursive
state-only pull request, or remote state database is introduced.

The aggregate design orchestrator now checks the envelope locally. It rejects
the old unconditional heading, requires both branches, parses exactly one
40-character observed default revision, proves that revision is a local commit,
and proves it is an ancestor of the checked revision. It deliberately does not
contact GitHub or pretend that local ancestry establishes current remote state.

## Rejected alternatives

- Pre-writing an unconditional successful merge result is rejected because the
  merge may fail, be retargeted, be reverted, or lack merge-revision validation.
- A write-capable post-merge workflow or recurring continuation-only pull
  request is rejected because it creates a recursive handoff problem.
- Treating stale objectives as harmless narration is rejected when they can
  repeat completed work, preserve a false blocker, or admit the wrong next
  transition.
- Parsing all continuation prose is rejected; only the stable envelope shape
  and local basis reachability are mechanically enforced.

## Migration

The envelope rule is projected into `AGENTS.md` and
`PROJECT_GOVERNANCE.md`; `PROJECT_STATE.md` now uses it, and
`tools/check-design-artifacts.sh` enforces the local invariant. This topic is
`MIGRATED`.
