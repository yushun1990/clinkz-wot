# 0026 WP-300 Critical-Path Concentration

Status: MIGRATED

Kind: owner-raised execution-risk investigation

Priority: HIGH

Target: the dependency concentration around WP-300 and its effect on the v1 critical path

## Scope and authority

This topic records a Project Owner question about whether the current work-package dependency structure concentrates too much downstream progress behind WP-300, and whether the repository accurately distinguishes necessary source serialization from preparation or evidence work that can proceed independently.

The question is investigation input only. It does not assert that the dependency graph is wrong, that WP-300 should be split, or that downstream packages should begin implementation early. Codex owns the repository-grounded technical decision and any resulting migration.

## Repository observations

The current repository records that:

- the dominant executable path proceeds through WP-100, WP-200, WP-300, and WP-400 before the Property Read architecture gate closes;
- WP-400, WP-500, and WP-600 depend on WP-300 and later rejoin at WP-700;
- the exact WP-300 Property Read slice may seek scoped admission before broad WP-300 completion;
- later preparation is allowed but is not admission or executable vertical progress;
- WP-300 owns the execution SPI consumed by Servient and concrete protocol bindings.

## Questions for investigation

1. Which exact WP-300 outputs are true source dependencies of the WP-400 Property Read slice?
2. Which exact WP-300 outputs are true source dependencies of WP-500 Directory/Discovery client work?
3. Which exact WP-300 outputs are true source dependencies of WP-600 Zenoh and zenoh-pico migration?
4. Does the current DAG distinguish dependency on the narrow Property Read slice from dependency on broad WP-300 package completion?
5. Are any downstream packages blocked by package-level status even when their exact required WP-300 contract is already complete?
6. Conversely, would allowing any downstream source work before broad WP-300 completion create duplicate ownership, unstable interfaces, or invalid evidence?
7. Is the current serialization caused by unavoidable public ownership and lifecycle boundaries, or by coarse work-package dependency representation?
8. Can non-authoritative preparation for WP-400, WP-500, or WP-600 materially reduce critical-path uncertainty without creating premature implementation claims?
9. Does the current milestone model make that preparation visible without overstating executable progress?
10. Could one unresolved broad WP-300 domain delay all three downstream branches even when it does not intersect their exact contracts?
11. What evidence proves that the present dependency concentration is necessary and bounded rather than an accidental project bottleneck?
12. If the concentration is not necessary, which authoritative dependency or status record is inaccurate?
13. If the concentration is necessary, what exact closure event releases each downstream branch?

## Constraints

- Do not assume that parallel work is valid merely because files or packages can be edited independently.
- Do not assume that package-level serialization is necessary merely because the current DAG records it.
- Do not bypass scoped admission, public-contract stability, or predecessor completion requirements.
- Do not prescribe a package split, DAG rewrite, or parallel implementation schedule before the dependency evidence is reconstructed.
- Preserve the AI-led model: the Owner raises the question, while Codex determines the technical answer and migration from repository evidence.

## Decision

The dependency concentration is necessary but has two different release
boundaries. The registered tranche DAG already releases the narrow
`WP-400-PROPERTY-READ-SERVIENT-SLICE` when the exact WP-300 Property Read
binding slice completes. It does not wait for broad WP-300 package completion.

The broad downstream branches correctly retain their package dependency:

- broad WP-400 consumes the complete server/client/lifecycle SPI;
- WP-500 consumes the complete client-call, cancellation, and generation-safe
  execution boundary rather than the Producer Property Read subset; and
- WP-600 implements the complete host and constrained binding SPI and performs
  the concrete legacy migration.

Therefore broad WP-300 completion is the release event for broad WP-400,
WP-500, and WP-600, while narrow WP-300 completion releases only the narrow
WP-400 integration tranche. A later broad WP-300 finding cannot delay the
narrow WP-400 tranche unless an explicit impact record intersects its exact
requirements, lifecycle, or evidence.

Non-authoritative contract reconstruction, fixture planning, workload
preparation, and migration inspection may proceed in downstream packages.
They do not grant source admission or executable vertical-progress credit.
No additional package split or DAG rewrite is justified before a downstream
tranche has a distinct dependency-complete contract and evidence boundary.

## Migration

The two release boundaries are projected into D19 and the critical-path text
in `PLAN.md`, the Property Read gate document, the WP-300 work-package record,
and `PROJECT_STATE.md`. The existing package and tranche DAG edges remain
unchanged.
