# 0029 WP-300 Admission Machinery Regression

Status: OPEN

Kind: owner-raised execution-governance investigation

Priority: MEDIUM

Target: reuse of governance, evidence, checker, and review machinery for the WP-300 Property Read admission path

## Scope and authority

This topic records a Project Owner question about whether the evidence and validation machinery proven during WP-200 can be reused for WP-300 without recreating support-artifact correction cycles or allowing governance defects to dominate the implementation critical path.

This question is linked to the migrated conclusions in workspace topics 0017 and 0020, but it does not reopen those decisions without new evidence. It asks whether their migrated rules are sufficient when applied to the next work-package boundary. Codex owns the repository-grounded technical decision and any resulting migration.

## Repository observations

The current repository records that:

- the WP-200 path required two evidence-boundary correction cycles before its finite transition was proven;
- those defects were classified as governance-implementation defects rather than changes to the technical contract;
- D9, D10, D13, and the directed owner-to-validation model now bound additional refinement and require next-state transition checks;
- a separate design-check runtime-root defect later caused a false aggregate failure and has been corrected;
- the next objective is a new WP-300 admission candidate with different ownership, lifecycle, portability, and authoring-fixture claims.

## Questions for investigation

1. Which WP-200 admission and transition mechanisms are intended to be reused unchanged for WP-300?
2. Which WP-300 claims require genuinely new governance or validation machinery because their defect classes differ?
3. Can the WP-300 candidate checker exercise the complete declared next state before attestation, including exact source absence, pre-source transition, implementation topology, and completion handoff?
4. Are all support artifacts that must change with the WP-300 candidate identified before the candidate is frozen?
5. Can any registry, carry-forward manifest, status projection, audit, or checker derive contradictory expectations for the same WP-300 transition?
6. Does the current directed owner-to-projection-to-evidence-to-validation model identify one authoritative source for every WP-300 admission claim?
7. Can a support-only inconsistency block WP-300 after the technical contract and exact next-state transition have already passed independent review?
8. If it can, what new intersecting falsification would justify that block under D9 and D13?
9. Are false failures caused by worktrees, shared targets, stale generated artifacts, or repository-root selection fully covered by the current validation setup?
10. Does the remote `mainline / validation` workflow exercise the same relevant state as local candidate and admission checks, or can those evidence surfaces diverge?
11. What observable event proves that WP-300 preparation is complete and implementation must be the next critical-path action?
12. What evidence would distinguish necessary WP-300 lifecycle review from repeated reassurance or support-artifact churn?
13. Could the existing governance machinery understate a real WP-300 ownership or lifecycle defect by classifying it as support-only?
14. If a new correction cycle occurs, how will Codex determine whether it reveals a technical defect, a transition-validation defect, or an unrelated tooling defect?
15. Are the conclusions of workspace topics 0017 and 0020 fully projected into every artifact that the WP-300 admission path will use?

## Constraints

- Do not assume that WP-200 correction cycles will recur in WP-300.
- Do not assume that existing governance machinery is sufficient merely because WP-200 eventually completed.
- Do not weaken independent review, transition validation, ownership, lifecycle, portability, or evidence requirements.
- Do not create new governance layers or checkers unless repository evidence proves a missing falsifiable claim.
- Do not reopen migrated topics 0017 or 0020 without identifying new evidence that conflicts with their conclusions.
- Preserve the AI-led model: the Owner raises the question, while Codex determines the technical answer and migration from repository evidence.
