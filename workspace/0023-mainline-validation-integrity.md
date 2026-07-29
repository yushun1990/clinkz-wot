# 0023 Mainline Validation Integrity

Status: OPEN

Kind: owner-raised repository-integrity investigation

Priority: HIGH

Target: validation and integration guarantees applied to the default branch and source-changing checkpoints

## Scope and authority

This topic records a Project Owner concern about whether the repository's documented local validation and independent review evidence are reliably enforced for every relevant mainline change, including interrupted sessions, direct commits, dependency changes, and future implementation work.

The concern is an investigation input. It does not assert that current validation is unreliable, does not require a specific CI service or branch policy, and does not prescribe a repository workflow. Codex owns the repository-grounded judgment.

## Repository observations

The repository records validation commands including:

- default workspace tests;
- the supported feature matrix;
- aggregate design-artifact checks;
- candidate-specific checks;
- mutation tests for selected admission and review boundaries;
- diff hygiene and exact topology inspection.

The repository also records AI-maintained checkpoints and independent root-session reviews. Current public repository metadata does not by itself establish which checks are automatically required for every default-branch update or whether all direct commits have remote status evidence.

## Owner concern

The Project Owner is concerned that a validation system can be technically comprehensive while still depending on each Codex session to remember, execute, interpret, and record every required command correctly. The concern includes whether mainline truth remains protected when sessions are interrupted or when changes do not pass through the same candidate-review path.

## Questions for investigation

1. Which validation commands are mandatory for every default-branch change, and which apply only to specific candidates, work packages, or categories?
2. Where is that requirement authoritatively defined and mechanically enforced?
3. Can a direct commit reach the default branch without the required workspace tests, feature matrix, design checks, or diff validation running successfully?
4. How does the repository detect an interrupted or incomplete validation sequence?
5. Are toolchain, lockfile, dependency, feature, and operating-environment assumptions reproducible outside the author session?
6. Are invalid feature combinations distinguished consistently from supported feature cells in all validation entry points?
7. Can candidate-specific checkers become stale or silently stop running after their immediate tranche completes?
8. Does remote repository status provide evidence equivalent to the validation claims recorded in audits and PROJECT_STATE?
9. How are flaky, environment-dependent, or nondeterministic failures classified without weakening the baseline?
10. Are branch, merge, parent-topology, and exact-path assumptions protected when changes are integrated outside the expected sequence?
11. What evidence would prove that mainline validation integrity is complete and durable under the current workflow?
12. If a gap exists, which governance, repository-policy, work-package, or checker owner must consume the decision?

## Constraints

- Do not assume that absence of visible remote status proves validation was not run.
- Do not assume that local validation is insufficient merely because it is local.
- Do not prescribe GitHub Actions, branch protection, pull requests, required checks, or another integration workflow before investigation.
- Do not duplicate validations without establishing distinct failure or evidence boundaries.
- Do not weaken supported feature, design, test, topology, or diff-hygiene baselines.
- Preserve the AI-led model.

## Expected decision output

Codex should determine:

1. the authoritative validation matrix for default-branch and tranche-specific changes;
2. whether every required validation is durably and reproducibly enforced;
3. whether audit and state claims can diverge from remote repository truth;
4. whether any repository policy, checker, workflow, or evidence owner requires correction;
5. the conditions for moving this topic through its workspace lifecycle.
