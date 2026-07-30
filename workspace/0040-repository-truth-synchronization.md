# 0040 Repository Truth Synchronization

Status: OPEN

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