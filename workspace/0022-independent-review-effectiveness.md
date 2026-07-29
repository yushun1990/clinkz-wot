# 0022 Independent Review Effectiveness

Status: MIGRATED

Kind: owner-raised review-quality investigation

Priority: HIGH

Target: the independent root-session review model used for architecture, admission, and completion evidence

## Scope and authority

This topic records a Project Owner concern about whether the current independent review model provides sufficient technical independence and defect detection across architecture, API usability, runtime behavior, portability, and evidence truth.

The concern is an investigation input. It does not reject independent root-session review, does not request Owner approval gates, and does not prescribe a different reviewer, model, or process. Codex owns the review-quality judgment from repository evidence.

## Repository observations

The repository records that:

- important candidates and corrections receive review from a later independent root session;
- reviewers inspect exact candidate topology, registered paths, executable schemas, fixtures, mutation boundaries, tests, feature matrices, and diff hygiene;
- the process has detected evidence defects that author-side checks did not close;
- author and reviewer sessions may still operate from the same repository authority, specifications, tools, and model family;
- future implementation risks include API authoring friction, lifecycle behavior, resource bounds, performance, and real binding integration in addition to document and checker consistency.

## Owner concern

The Project Owner is concerned that session independence may effectively detect state, topology, manifest, and mutation defects while still sharing assumptions that reduce its ability to challenge architecture usability, implementation realism, runtime behavior, or hidden complexity. The concern is whether the repository can distinguish procedural independence from materially independent technical validation.

## Questions for investigation

1. What classes of defects has independent root-session review detected that author-side validation missed?
2. What classes of defects are the current review scripts, fixtures, and review prompts capable of detecting?
3. Which important risks depend on a different implementation perspective, external authoring perspective, runtime workload, or adversarial interpretation rather than a clean session alone?
4. Do reviewers independently reconstruct the intended contract from authoritative owners, or primarily verify a candidate against author-prepared audit material?
5. Can the same mistaken assumption be shared by candidate specifications, fixtures, checkers, author review, and independent review?
6. How does the repository validate API usability and constructibility beyond schema conformance?
7. How does it validate lifecycle, cancellation, cleanup, fairness, resource, and performance claims beyond static contract checks?
8. Does the required review evidence vary proportionately across Category A, B, and C work?
9. Are review findings recorded in a way that allows later assessment of review effectiveness and recurring blind spots?
10. What evidence would prove that the current independent review model is sufficient for the pending WP-200 slice and later WP-300/WP-400 execution work?
11. If a review gap exists, which authoritative governance or work-package owner must consume the decision?

## Constraints

- Do not assume that using the same model family makes review ineffective.
- Do not assume that a different model, human reviewer, or additional review stage is required.
- Do not weaken exact candidate review, mutation testing, feature validation, or rollback evidence.
- Do not introduce Owner approval as a substitute for technical review.
- Do not prescribe reviewer identity, tool selection, prompt structure, or review count before investigation.
- Preserve the AI-led model.

## Expected decision output

Codex should determine:

1. the defect classes covered and not covered by the current independent review model;
2. whether current evidence demonstrates material technical independence for each risk category;
3. whether review requirements remain proportionate across work categories;
4. whether any governance, audit, fixture, or work-package contract requires correction or reaffirmation;
5. the conditions for moving this topic through its workspace lifecycle.

## Decision

Independent root review has materially detected defects that author-side
validation missed: stale carry-forward truth and a contradictory pre-source
state projection. Its current exact-ref, path, mutation, feature, and
transition checks are sufficient for the pending WP-200 admission-evidence
claim.

Session separation alone is not sufficient for every later claim. Candidate
and audit material can share the same mistaken assumption, and static schemas
cannot establish callback behavior, cancellation, cleanup, fairness, resource
peaks, performance, or real production-binding usability. Review sufficiency
therefore follows defect class:

- pre-source review reconstructs authoritative contracts, exact topology,
  negative capabilities, public authoring boundaries, and the next state;
- implementation completion adds real compile/test and boundary behavior;
- lifecycle/resource/performance work adds failure injection and workload
  evidence; and
- vertical and release review adds production binding and end-to-end evidence.

An author-prepared audit is navigation, not the reviewer's authority.
Independent reconstruction, mutation cases, external public-boundary fixtures,
and executable observations provide material independence. No additional
reviewer identity or Owner approval gate is justified by current evidence.

## Migration

The claim-class and reconstruction rules are migrated into
`PROJECT_GOVERNANCE.md`, D15 in `PLAN.md`, and the continuation summary. The
WP-200 v2 audit already requires the exact next-state mutations; later work
packages retain their distinct completion and integration evidence.
