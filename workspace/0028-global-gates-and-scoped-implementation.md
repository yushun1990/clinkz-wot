# 0028 Global Gates and Scoped Implementation

Status: OPEN

Kind: owner-raised architecture-closure investigation

Priority: HIGH

Target: the interaction between open global convergence gates and independently admitted implementation slices

## Scope and authority

This topic records a Project Owner question about whether the repository defines a sufficiently precise and bounded relationship between still-open global architecture gates and scoped implementation admitted under ADR-0013.

The question is investigation input only. It does not assert that scoped implementation should stop, that global gates should be weakened, or that completed evidence is immune from legitimate later findings. Codex owns the repository-grounded technical decision and any resulting migration.

## Repository observations

The current repository records that:

- GATE-3 is closed while GATE-1, GATE-2, GATE-4, GATE-5, and GATE-6 remain open;
- ADR-0013 permits an independently reviewed, dependency-complete tranche to proceed when it is disjoint from open global findings;
- all global gates must close before final integration and release conformance;
- a later global-gate finding invalidates earlier evidence only through explicit impact and revalidation rules;
- the WP-100 and WP-200 Property Read slices are complete while WP-300 and WP-400 remain on the scoped vertical path.

## Questions for investigation

1. What exact technical domains are still owned by each open global gate?
2. Which of those domains can intersect the WP-300 Property Read binding slice?
3. How is disjointness between a proposed tranche and an open global finding represented and validated?
4. Is disjointness an explicit checked claim, or is it inferred informally from package and path boundaries?
5. What kinds of later global-gate findings may invalidate a completed WP-100 or WP-200 slice?
6. What kinds of findings may require revalidation without invalidating the completed implementation claim?
7. What kinds of findings are necessarily disjoint and therefore cannot reopen the scoped critical path?
8. Does the repository identify the exact evidence affected by a gate finding, or can one finding trigger broad undifferentiated re-review?
9. Are impact propagation and revalidation rules executable and consistent across architecture records, work packages, evidence, and checkers?
10. Could an open gate remain broad enough that any future implementation detail can be classified as intersecting after the fact?
11. Conversely, could scoped admission incorrectly classify a real cross-domain dependency as disjoint?
12. Is the current PROJECT_STATE accurate about what remains open, what is allowed to proceed, and what evidence may later be affected?
13. What evidence proves that scoped implementation can continue without silently weakening final global closure?
14. If the current relationship is not sufficiently bounded, which authoritative owner contains the ambiguity?

## Constraints

- Do not assume that an open global gate blocks every implementation tranche.
- Do not assume that completed scoped evidence can never be affected by a later legitimate finding.
- Do not weaken final convergence or release requirements under this topic.
- Do not expand a gate finding beyond its demonstrated technical and evidence impact.
- Do not prescribe gate closure order or pause the active critical path before Codex reconstructs the exact intersections.
- Preserve the AI-led model: the Owner raises the question, while Codex determines the technical answer and migration from repository evidence.
