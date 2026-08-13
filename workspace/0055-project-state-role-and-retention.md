# 0055 PROJECT_STATE Role and Retention

Status: OPEN

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
