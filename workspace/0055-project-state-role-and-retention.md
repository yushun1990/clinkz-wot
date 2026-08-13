# 0055 PROJECT_STATE Role and Retention

Status: MIGRATED

Kind: owner-raised execution-governance investigation

Priority: HIGH

Target: the necessity, scope, and retention policy of `PROJECT_STATE.md`

## Scope and authority

This topic records a Project Owner concern about whether `PROJECT_STATE.md` is still serving its intended role as a compact continuation checkpoint or has become a redundant execution ledger that increases agent context and synchronization cost.

This is an investigation request, not a predetermined instruction to keep, delete, or rewrite `PROJECT_STATE.md`. Codex owns the technical decision after examining repository usage and evidence.

## Owner observation

`AGENTS.md` defines `PROJECT_STATE.md` as curated continuation memory rather than a session transcript and requires stale information to be replaced rather than accumulated. In practice, the file has repeatedly accumulated exact historical candidate, review, pull-request, merge, workflow, and transition chains.

At the same time, a fresh agent must still fetch and inspect the actual default branch before relying on remote integration state. Git history, work-package records, audits, evidence manifests, and GitHub already retain much of the historical information copied into `PROJECT_STATE.md`.

The Owner therefore questions whether the current file is providing enough unique continuation value to justify its size and maintenance burden.

## Questions for investigation

1. What information does a fresh Codex session genuinely need from `PROJECT_STATE.md` that cannot be recovered cheaply and reliably from `PLAN.md`, registered work packages, authoritative docs, Git history, and the fetched default branch?
2. Is `PROJECT_STATE.md` necessary as a separate artifact at all, or should its role be reduced, replaced, or merged into another existing owner?
3. If it remains, should it function only as a compact navigation/continuation cache containing the current basis, objective, established frontier, blockers, stopping point, next safe action, and a small set of relevant references?
4. Which historical candidate/review/admission/merge/workflow facts should never be retained there once they are available from their authoritative owners?
5. Why have the existing "curated memory" and "replace stale information" rules failed to prevent ledger-like accumulation, and are any other governance rules creating contrary incentives?
6. Should session entry continue to require reading the whole file, or should continuation be recoverable through a smaller bounded projection?
7. How should stale remote observations be represented so that `PROJECT_STATE.md` remains useful without pretending to be repository or GitHub truth?
8. Do migrated topics 0048 and 0051 already own part of this problem, and if so, why did their conclusions not prevent the current accumulation pattern?
9. What measurable property would show that the resulting continuation mechanism reduces agent recovery cost without weakening recoverability or evidence integrity?

## Constraints

- Preserve repository-native continuity across fresh AI sessions.
- Do not duplicate Git history, GitHub state, work-package records, audits, evidence, or normative specifications merely for convenience.
- Do not weaken exact evidence requirements at the artifacts that actually own those facts.
- Treat `PROJECT_STATE.md` as non-authoritative with respect to implementation, architecture, admission, completion, and remote integration truth.
- Prefer deletion or replacement of stale information over indefinite accumulation if the artifact is retained.
- Do not assume that retaining the current file is the correct outcome.

## Expected decision output

Codex should determine:

1. whether `PROJECT_STATE.md` remains necessary;
2. its exact unique responsibility if retained;
3. the minimum information required for fresh-session continuation;
4. which existing content belongs elsewhere and should be removed rather than copied forward;
5. whether session-entry and checkpointing rules need adjustment;
6. how the decision interacts with Git/GitHub truth, work-package/evidence ownership, review-cycle boundaries, and remote reconciliation; and
7. the authoritative owner(s) into which the stable conclusion should be migrated.

## Decision

`PROJECT_STATE.md` remains necessary, but only as a compact continuation and
observed-remote cache. A fresh session needs one cheap projection that says
which fetched default revision was actually inspected, which frontier is
established, what currently blocks or limits work, where the previous cycle
stopped, and what action applies before versus after a pending integration.
Those facts are not safely replaced by a roadmap or by asking every new session
to reconstruct the entire repository before it knows where to look.

The former ledger shape is rejected. Candidate, review, admission, commit,
merge, and workflow histories belong to Git, GitHub, registered manifests, and
audits. Accepted/rejected reasoning belongs to workspace decisions, ADRs, and
specifications. The current claim and acceptance boundary belong to the new
`EXECUTION.md`. `PLAN.md` owns only roadmap and milestone state.

The retained state file is limited to 200 lines and to seven content classes:
one exact observed default revision and observation basis; established
frontier; current execution-contract pointer; blockers/limits; stopping point;
conditional pre/post-integration actions; and a small navigation set. Stale
material is replaced. It never overrides GitHub or repository evidence.

The earlier rules failed because they simultaneously called the file curated
memory and required it to retain architecture understanding, accepted and
rejected decisions, every substantial change, exact remote history, and next
work. Continuous checkpointing therefore rewarded accumulation. The new rules
split those responsibilities and impose a measurable size bound in the
existing continuation check.

## Migration

The responsibility split and retention rules are migrated to `AGENTS.md` and
`PROJECT_GOVERNANCE.md`; `EXECUTION.md` now owns the active claim; `PLAN.md` and
`PROJECT_STATE.md` are rewritten to their bounded roles; and the existing
continuation checker enforces the state-file ceiling without validating prose.

Displaced default: append enough historical narrative to make state locally
self-contained. New default: link to authoritative owners and replace the
small continuation projection. First activation: this execution-model reset.
Falsifier: state again exceeds 200 lines, duplicates recoverable history, or
requires a fresh session to reconcile competing current-plan owners. Such
evidence reopens this topic.
