# 0055 PROJECT_STATE Role and Retention

Status: MIGRATED

Kind: owner-raised execution-governance investigation

Priority: HIGH

Target: the necessity, scope, and retention policy of `PROJECT_STATE.md`

## Scope and authority

This topic records a Project Owner concern about whether `PROJECT_STATE.md` is
serving useful durable continuity or has become a redundant execution ledger
that increases context and synchronization cost.

The investigation does not presume retention or deletion. The technical
conclusion must follow repository usage evidence.

## Owner observation

Earlier governance described `PROJECT_STATE.md` as curated continuation memory,
but the file repeatedly accumulated exact candidate, review, pull-request,
merge, workflow, transition, blocker, and next-action state already recoverable
from Git, GitHub, work packages, audits, evidence, and code.

A fresh AI session must still fetch and inspect the real default branch before
relying on any cached remote or implementation state. The Owner therefore
questions whether a separate model-authored state projection provides enough
unique value to justify its maintenance cost.

## Questions for investigation

1. What information does a fresh session genuinely need that cannot be recovered
   cheaply and more reliably from `PLAN.md`, Git/GitHub, authoritative docs,
   work packages, code, tests, and evidence?
2. Is a separate state artifact necessary at all?
3. Does introducing another current-task carrier merely move duplication from
   one state file to two?
4. Why did earlier curated-memory rules fail to prevent accumulation?
5. Can task continuity instead be recovered by scoping one conversation to one
   natural major task and reconstructing repository truth at the next task
   boundary?
6. What evidence would falsify either the retain or delete decision?

## Constraints

- Preserve repository-native technical truth across fresh AI sessions.
- Do not duplicate Git history, GitHub state, work-package records, audits,
  evidence, or normative specifications for conversational convenience.
- Do not weaken exact evidence at the artifacts that actually own those facts.
- Current remote and implementation truth must be reconstructed from their real
  owners when precision matters.
- Prefer fewer state owners over bounded duplicate summaries.

## Original decision

The first 0055 decision retained `PROJECT_STATE.md` as a compact continuation
and observed-remote cache. It limited the file to a fetched default revision,
established frontier, current execution pointer, blockers, stopping point,
conditional actions, and navigation links. Candidate/review/admission/merge
history was rejected from the file.

That decision explicitly defined its own falsifier: reopen the conclusion if the
state file again duplicates recoverable facts or creates competing current-plan
owners.

## First-activation evidence

The first real activation of the new state-plus-execution model falsified the
retain assumption before implementation began:

- `EXECUTION.md` expanded from its idle template to almost the entire 200-line
  budget during planning alone, before challenge, executor handoff, or review;
- `PROJECT_STATE.md` repeated the same current PR, task, blocker, stop point,
  execution boundary, and next-action information already present in the
  execution contract and recoverable from Git/GitHub;
- the fresh Max planning session still inspected the repository, implementation,
  roadmap, and relevant authority to obtain precise current truth rather than
  trusting the state cache;
- constraining the state files changed their size but did not create unique
  technical knowledge; and
- keeping model collaboration state in repository files created additional
  synchronization and checker obligations without improving architectural
  judgment.

The original retention rationale therefore did not survive its first real use.

## Revised decision

`PROJECT_STATE.md` is no longer an active governance artifact. Current project
state is reconstructed from the fetched repository and its authoritative
owners:

- `PLAN.md` for durable roadmap and milestone context;
- specifications, ADRs, work packages, audits, and evidence for durable
  technical truth;
- source and tests for implementation truth;
- Git/GitHub/CI for branch, pull-request, merge, and validation state.

The next action is intentionally not persisted as a separate source of truth.
It is re-derived at the start of a task session from project goals and current
repository evidence.

`EXECUTION.md` is likewise not used as a current-task state machine. Model
profile selection and working plans belong to the active task conversation.
High/XHigh are the default implementation capacity when the design boundary is
clear; Max is used when a genuine decision boundary requires stronger
higher-order judgment. A major task normally uses one coherent conversation;
a new task or independent review may use a fresh context.

The two legacy paths remain only because historical immutable review and
transition records may name them in exact path sets. They contain deprecation
notices and must not receive new current state.

## Migration

The revised conclusion is migrated to:

- `AGENTS.md` — task-session reconstruction, capability allocation, and no
  shadow-state rules;
- `PROJECT_GOVERNANCE.md` — active progression, review, recovery, and
  model-selection governance;
- `PLAN.md` — durable roadmap wording with no current-state dependency;
- `EXECUTION.md` and `PROJECT_STATE.md` — compatibility tombstones only; and
- `tools/check-design-artifacts.sh` — removal of checker logic that enforced
  conversational state-file shape.

New default: repository documents store durable rules, requirements, design
intent, decisions, and evidence. Current state is discovered; it is not
maintained as a second model-authored project database.
