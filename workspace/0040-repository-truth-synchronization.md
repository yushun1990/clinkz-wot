# 0040 Repository Truth Synchronization

Status: MIGRATED

Kind: owner-raised continuation and integration-integrity investigation

Priority: HIGH

Target: synchronization among merged GitHub state, `PROJECT_STATE.md`, `PLAN.md`, work-package status, audits, and the next safe Codex action

## Scope and authority

This topic records a Project Owner concern that remote integration can advance while durable continuation files still describe a pre-merge objective, causing a fresh Codex session to repeat completed work or wait at an already crossed boundary. It does not make GitHub metadata an architecture authority. Codex owns the governance decision.

## Repository observations

- PR #1 merged the reviewed WP-300 candidate into `master`.
- `PROJECT_STATE.md` on that merged revision still instructs the next session to publish that PR and wait for integration.
- The same file still describes the candidate as `pending`/`review-pending` even though the remote handoff has completed.
- Remote validation, repository files, candidate topology, and tranche admission remain different evidence surfaces.
- Automatic task handoff and proposed auto-merge increase the importance of a reliable post-integration continuation transition.

## Questions for investigation

1. Which state transition must occur before merge, at merge, and immediately after merge?
2. Can one PR truthfully contain a continuation state that predicts its own future integration without becoming stale after merge?
3. Should the next task begin with a mandatory remote reconciliation step before trusting `PROJECT_STATE.md`?
4. Can GitHub Actions or a follow-up agent safely update continuation state without creating recursive PRs or bypassing review topology?
5. Which facts are authoritative in GitHub, which are authoritative in repository records, and how are disagreements resolved?
6. How should draft, ready, auto-merge-enabled, merged, failed, and superseded PR states map to tranche status?
7. What checker can detect an impossible current objective without making remote availability a requirement for local design validation?

## Constraints

- Do not make remote GitHub availability necessary for offline source correctness.
- Do not allow stale continuation prose to redefine admission or completion truth.
- Preserve exact candidate, review, admission, implementation, and evidence topology.
- Avoid a new recursive support-artifact workflow that blocks product changes.

## Expected decision output

Codex should define the pre-merge and post-merge continuation protocol, remote reconciliation rules for fresh sessions, authoritative ownership of integration state, any lightweight stale-objective detection, and migrations to governance, task handoff, or auto-merge policy.

## Decision

GitHub owns pull-request draft/ready/check/merge facts. Git commits, registered
work-package records, audits, and evidence own candidate, review, admission,
implementation, and completion truth. `PROJECT_STATE.md` projects the last
observed combination and cannot override either source.

Every fresh substantial session first fetches the remote default branch when
available and reconciles the recorded task/PR state before following the
current objective. Offline work may use the last observed snapshot, but cannot
release a dependent source transition from an unverified remote merge.
Pre-merge state records the exact handoff and both conditional next actions.
After merge, the next task updates continuation state in its first checkpoint;
a write-capable post-merge workflow and recursive state-only pull request are
rejected.

This investigation found a concrete local defect beyond stale prose. The
WP-300 work-package checker used the current repository `HEAD` as though it
were the immutable review-attestation commit, and the future pre-source check
required the admission checkpoint to be a direct child of that old commit.
PR #1's merge commit and PR #2 made both assumptions false. Review-attestation
identity and current admission-base identity are now separate fields:

- the attestation ref identifies the immutable two-path review commit;
- the admission base identifies the fetched, reviewed default-branch
  descendant on which the five-file pre-source checkpoint is made; and
- the pre-source commit is the single child of that admission base, while the
  original attestation must remain in its ancestry.

The registered work-package check is added to required mainline validation so
this impossible objective/topology is detected locally without contacting
GitHub. Remote freshness still requires fetch; offline validation cannot infer
an unseen remote commit.

## Migration

The reconciliation protocol is projected into `AGENTS.md`,
`PROJECT_GOVERNANCE.md`, `PLAN.md`, and `PROJECT_STATE.md`. The WP-300 gate,
entry audit/checker, design checker, and mainline workflow consume the
attestation/admission-base separation. Product source remains blocked until
this correction receives its required independent review. This topic is
`MIGRATED`.
