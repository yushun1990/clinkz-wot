# 0030 Automatic Pull Request Integration

Status: MIGRATED

Kind: owner-raised repository-workflow proposal

Priority: MEDIUM

Target: removal of routine Owner merge clicks while preserving AI-led technical judgment, registered evidence, protected-mainline validation, and recoverable Git history

## Scope and authority

This topic records a Project Owner proposal to evaluate whether completed repository-changing tasks should be integrated automatically after their registered evidence and remote required checks pass, instead of requiring the Owner to click Merge for every pull request.

The proposal does not pre-decide that every pull request must auto-merge, does not treat one generic CI status as sufficient for every work package, and does not transfer architecture or milestone judgment to GitHub. Codex owns the repository-grounded decision, the safe eligibility boundary, and any resulting migration into project governance, workflow, ruleset, or agent instructions.

## Repository observations

The current repository records that:

- bounded repository-changing tasks are committed and pushed on task branches and handed off through one draft pull request;
- `AGENTS.md` and `PROJECT_GOVERNANCE.md` explicitly prohibit automatic pull-request merging and normally require remote review and integration before dependent work proceeds;
- the repository-level GitHub setting `allow_auto_merge` is enabled;
- active Ruleset `20009352` protects the default branch and requires the GitHub Actions status context `validation`;
- `.github/workflows/mainline.yml` runs committed diff hygiene, aggregate design checks, locked workspace tests, and the supported feature matrix on pull requests and default-branch pushes;
- registered candidate, admission, completion, workload, release, lifecycle, mutation, topology, and other task-specific checks remain additional evidence and are not replaced by the generic `validation` status;
- the project intentionally preserves semantically meaningful candidate, review, admission, implementation, and evidence commits rather than requiring squash integration; and
- the first pull-request handoff was merged manually without a required GitHub review submission, so the current manual click is not by itself a structured review-evidence mechanism.

## Owner proposal

The Project Owner would prefer not to perform a routine manual merge action when Codex has already determined that the bounded task is complete, all applicable task-specific evidence passes, the pull request is ready, and GitHub's required remote status checks succeed.

The desired outcome is not unattended integration at any cost. The desired outcome is a clear automation boundary in which Codex retains responsibility for technical completion and evidence sufficiency, while GitHub performs the final merge automatically once the repository's mechanical conditions are satisfied.

## Questions for investigation

1. What exact event should make a pull request eligible for automatic integration?
2. Which task-specific checks must Codex run or expose before changing a pull request from draft to ready and enabling auto-merge?
3. Is the required `validation` status sufficient for low-risk documentation changes, and how must higher-risk admission, public API, lifecycle, resource, protocol-boundary, or release changes add their scoped evidence?
4. Should all eligible pull requests use GitHub native auto-merge, or is a repository workflow or merge queue required for any defect class?
5. Which merge method preserves the project's meaningful commit topology and candidate/review/admission boundaries?
6. Can Codex enable auto-merge directly after local task evidence passes, or must an independent review attestation or other repository-owned state first be present for selected tranches?
7. How should draft status, ready-for-review status, auto-merge enablement, and later commits to the same pull request interact?
8. What must happen when required checks fail, are cancelled, become stale, or are superseded by a new head commit?
9. How should merge conflicts, a moved default branch, required branch updates, and stacked pull requests be handled without bypassing dependency truth?
10. Which changes still require explicit Owner intervention because they alter project goals, product trade-offs, external commitments, unacceptable directions, or another Owner-owned boundary?
11. Does the current Ruleset need an additional pull-request rule, review requirement, merge queue, required workflow, or no change beyond the existing `validation` requirement?
12. Should task-specific evidence become one additional remote required status, remain Codex-attested repository evidence, or use a mixed model based on risk?
13. How will the repository prove that auto-merge cannot activate before the applicable candidate, admission, completion, or release state is valid?
14. How should Codex report an automatically merged pull request and start dependent work without racing the final default-branch workflow result?
15. What rollback, disablement, or emergency stop is required if automatic integration exposes a governance, evidence, or GitHub configuration defect?
16. Which authoritative files must consume the decision, and what observable event proves the migration is complete?

## Constraints

- Do not weaken the protected-default-branch `validation` requirement.
- Do not treat generic mainline validation as a substitute for registered task-specific evidence.
- Do not enable auto-merge while a pull request is draft, while an applicable independent review is incomplete, or while the task's authoritative state still declares the source transition blocked.
- Do not require a routine Owner click merely to preserve the appearance of human review when no structured review evidence is produced.
- Preserve Owner authority over project goals, product trade-offs, external commitments, and explicitly Owner-owned decisions.
- Preserve AI responsibility for architecture, implementation sequencing, admission, evidence sufficiency, and technical milestone judgment under the accepted governance model.
- Preserve semantically required commit topology; do not adopt squash merging by default where it would erase candidate, review, admission, implementation, or evidence boundaries.
- Do not grant a workflow broader write permissions than the selected integration mechanism actually requires.
- Do not bypass dependency ordering, exact-parent requirements, required status checks, unresolved review findings, or branch rules.
- Prefer GitHub-native mechanisms over custom automation unless repository evidence identifies a missing falsifiable guarantee.

## Expected decision output

Codex should determine:

1. whether routine pull-request integration can safely become automatic;
2. the exact eligibility predicate for enabling auto-merge;
3. which risk classes require additional remote or repository evidence before eligibility;
4. the merge method and commit-topology policy;
5. the handling of draft transitions, new commits, failed checks, conflicts, stacked work, and dependent-task release;
6. the remaining Owner-intervention boundary;
7. any required GitHub setting, Ruleset, workflow-permission, checker, or status-context changes;
8. the authoritative migrations required in `AGENTS.md`, `PROJECT_GOVERNANCE.md`, `PLAN.md`, `PROJECT_STATE.md`, or other owners; and
9. the validation and remote evidence that prove the new workflow is complete without weakening integration integrity.

## Decision

Routine integration may use GitHub native auto-merge, but only as the terminal
state of the existing AI-owned evidence workflow. Auto-merge is not itself an
admission or review mechanism.

An AI agent may move a draft pull request to ready and enable auto-merge only
when all of the following hold for the exact current head:

1. the intended diff is complete and contains no unrelated work;
2. every applicable candidate, independent-review, admission, completion,
   workload, release, and removal record is present and current;
3. the task-specific local checks and the required remote `validation` job
   cover the current head;
4. the branch is current with the target branch, has no conflict, is not a
   dependent stack, and has no unresolved review conversation or requested
   change;
5. the pull request crosses no unresolved Owner-owned product-goal, trade-off,
   unacceptable-direction, public-release, or other external-commitment
   boundary; and
6. the remote ruleset is verified to require strict current-base validation
   and conversation resolution.

The merge method is a merge commit. Squash and rebase integration are
ineligible when they would erase or rewrite candidate, review, admission,
implementation, or evidence object identities. Auto-merge is enabled with an
expected head object id. A later commit invalidates the eligibility decision
and must rerun the evidence predicate; failed, cancelled, stale, or missing
checks leave the pull request unmerged. Conflicts and stacked work require a
new current-base validation rather than bypass.

The existing GitHub-native mechanism is sufficient once those prerequisites
hold; no custom write-capable merge workflow or merge queue is introduced.
The mainline workflow now executes the registered work-package/evidence check
in addition to the generic matrix so its required status is not blind to
repository-owned task state. Until the remote ruleset prerequisites are
verified, handoff remains draft and auto-merge remains disabled.

Dependent work is released only after the merge is visible on the fetched
default branch and the default-branch validation for the merge revision
passes. A merge click, automatic or manual, never substitutes for that
reconciliation event.

## Migration

The eligibility predicate, merge method, failure handling, Owner boundary, and
post-merge release event are projected into `AGENTS.md`,
`PROJECT_GOVERNANCE.md`, `.github/workflows/mainline.yml`, `PLAN.md`, and
`PROJECT_STATE.md`. Operational activation remains conditional on verified
remote ruleset state. This topic is therefore `MIGRATED`.
