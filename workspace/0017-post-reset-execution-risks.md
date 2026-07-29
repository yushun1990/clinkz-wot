# 0017 WP-200 Admission Evidence Recursion Risk

Status: OPEN

Kind: owner-raised execution-risk investigation

Priority: HIGH

Target: the admission path from the current WP-200 Property Read plan candidate to the first product-source implementation commit

## Scope and authority

This topic records a Project Owner concern about whether the current WP-200 admission path can terminate cleanly after the pending second correction review, or whether evidence-boundary, checker, state, registry, and checkpoint interactions can continue exposing additional serial correction cycles before product source is admitted.

The concern is an investigation input. It is not an accepted finding, predetermined conclusion, implementation instruction, governance change, or proposal for weakening review or admission requirements. Codex owns the repository-grounded technical judgment and any resulting migration.

## Repository observations

The current repository records that:

- the WP-200 Property Read semantic candidate has passed independent v1 review;
- the original pre-source checkpoint exposed a stale carry-forward digest;
- the first correction repaired that digest boundary but exposed a contradictory implementation-source presence check;
- a second six-path correction candidate is pending independent v2 review;
- no WP-200 product source has yet been admitted;
- D9 states that design preparation should convert through one bounded review, one pre-source checkpoint, implementation, and completion evidence unless a new intersecting issue is proven.

## Owner concern

The Project Owner is concerned that the admission machinery itself may be complex enough to generate new critical-path defects after each correction, even when the reviewed WP-200 API, ownership, fixture, implementation-path, exclusion, and precheck semantics remain unchanged.

The concern is not that the already discovered evidence-truth defects should be ignored. It is whether the repository can now demonstrate a finite and trustworthy stopping condition for admission preparation.

## Questions for investigation

1. What exact observable repository event proves that the WP-200 admission boundary is complete and product-source implementation must be the next critical-path action?
2. Can any remaining checker, manifest, state, registry, audit, or topology rule still reject the planned five-file pre-source checkpoint after the pending second correction passes?
3. Are all validations needed for that checkpoint exercised before the v2 attestation is created, including the exact transition into the registered implementation commit?
4. Do the current admission rules distinguish a defect in product architecture from a defect created solely by the governance implementation?
5. Could another evidence-only inconsistency require a third correction cycle without changing the WP-200 technical contract?
6. Does D9 provide an enforceable refusal condition for additional non-implementation refinement after the current correction closes?
7. Is the current WP-200 admission state accurately represented across PLAN, PROJECT_STATE, work-package records, audits, manifests, and executable checkers?
8. Are any duplicated or circular truth dependencies present among those artifacts?
9. What evidence would prove that the recursion risk is absent and that the transition to the nine registered product-source paths is now bounded?
10. If the risk is real, which exact repository mechanism causes it, and which authoritative owner must record the resulting decision?

## Constraints

- Do not assume that the risk is real merely because two corrections have occurred.
- Do not assume that any existing review, evidence, or checker requirement is unnecessary.
- Do not weaken or bypass the pending v2 review or pre-source admission boundary under this topic.
- Do not reopen the accepted WP-200 compiler/artifact representation unless repository evidence proves a direct semantic intersection.
- Do not prescribe a correction, new process, reduced process, artifact count, review threshold, or implementation schedule before the investigation reaches a technical conclusion.
- Preserve the AI-led model: the Owner raises the concern, while Codex determines the technical answer from repository evidence.

## Expected decision output

Codex should determine:

1. whether the WP-200 admission path has a finite verified stopping condition;
2. whether another evidence-only correction cycle remains possible under the current repository rules;
3. the exact mechanism responsible if the risk exists;
4. whether the current authoritative records overstate or understate implementation readiness;
5. whether any stable conclusion must be migrated to governance, work-package, audit, checker, state, or implementation owners;
6. the conditions under which this topic can move from `OPEN` to `DECIDED`, and from `DECIDED` to `MIGRATED`.
