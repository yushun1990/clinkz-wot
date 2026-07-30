# 0025 WP-300 Property Read Slice Scope

Status: MIGRATED

Kind: owner-raised execution-risk investigation

Priority: HIGH

Target: the admission boundary for `WP-300-PROPERTY-READ-BINDING-SLICE`

## Scope and authority

This topic records a Project Owner question about whether the exact WP-300 Property Read binding slice has a finite scope that can be admitted and implemented independently from the broader WP-300 lifecycle, subscription, emission, cancellation, resource, and constrained-runtime obligations.

The question is investigation input only. It does not assert that the current WP-300 contract is over-scoped, that any requirement should be deferred, or that any evidence obligation should be weakened. Codex owns the repository-grounded technical decision and any resulting migration.

## Repository observations

The current repository records that:

- the narrow WP-200 Property Read plan slice is complete;
- the exact WP-300 Property Read binding slice is the only exception to the blocked `WP-300-BROAD-ENTRY`;
- WP-300 as a package owns complete registration, route lifecycle, request/response execution, cancellation and cleanup, subscription progress, Producer emission, runtime status, ingress bounds, and host/constrained execution surfaces;
- no WP-300 product-source path is currently admitted;
- the next objective is to prepare the exact WP-300 candidate and public host/constrained authoring fixtures.

## Questions for investigation

1. What exact public contracts and state transitions are semantically required to implement one Property Read binding slice?
2. Which WP-300 requirements directly intersect that slice, and which belong only to broader package completion?
3. Does the current work-package record distinguish an interface that must exist for forward compatibility from behavior that must be implemented and evidenced in this tranche?
4. Can the exact slice be constructed without implementing subscription delivery, Producer emission, broad cancellation matrices, or unrelated lifecycle paths?
5. If any of those broader domains are required by the slice, what concrete ownership or lifecycle dependency makes them inseparable?
6. Do the planned host and constrained authoring fixtures prove only the Property Read boundary, or do they implicitly require a nearly complete third-party binding implementation?
7. Are AR-002 and AR-003 bounded to the exact slice, or can their closure require evidence belonging to broad WP-300 completion?
8. Is there one observable repository state that proves the candidate is complete and source implementation is the next critical-path event?
9. Could the candidate remain indefinitely in preparation because every omitted future WP-300 behavior can be interpreted as an admission defect?
10. Are the current PLAN, PROJECT_STATE, work-package DAG, gate manifest, and WP-300 specification consistent about the exact slice boundary?
11. What repository evidence would falsify the concern that the narrow slice has inherited broad-package scope?
12. If the concern is valid, which authoritative artifact currently creates the scope coupling?

## Constraints

- Do not assume that the WP-300 package is over-scoped merely because it owns many lifecycle domains.
- Do not assume that subscription, emission, cancellation, cleanup, resource, or constrained-runtime contracts are unrelated without tracing their exact dependencies.
- Do not weaken ownership, lifecycle, portability, resource, or evidence requirements under this topic.
- Do not preselect a smaller or larger tranche before the repository evidence is reconstructed.
- Preserve the AI-led model: the Owner raises the question, while Codex determines the technical answer and migration from repository evidence.

## Decision

The concern is confirmed at the admission-record boundary, not at the target
architecture boundary. The Property Read slice is finite, but its previous
machine-readable record named only requirements and three broad blockers. It
did not yet distinguish API surface required for forward-compatible complete
registration from behavior claimed by this tranche.

The exact slice contains one complete registration that advertises only the
Producer Property Read server role and consumes one matching WP-200 compiler
component. It covers:

- registration identity and compatibility validation;
- immediate-readiness and externally-ready route preparation;
- prepared -> ready -> active -> committed-closed route ownership;
- one borrowed-permit Property Read acceptance;
- one owned response opportunity and response delivery;
- explicit abort or shutdown with retained cleanup ownership; and
- equivalent host-erased and application-static authoring forms.

Public traits may include default rejection adapters for optional capabilities
needed by the complete bundle shape. Merely exposing such an interface does
not claim working client invocation, subscription delivery, Producer emission,
form contribution, broad cancellation matrices, multi-route fairness,
production networking, Servient orchestration, or package completion. Those
domains remain broad WP-300 work.

AR-002 and AR-003 close for this tranche when the exact signatures, both
readiness shapes, both authoring profiles, exclusions, implementation paths,
and expected absent-source completion boundary are executable in the immutable
candidate. They do not expand to every broad WP-300 behavior.

## Migration

The finite boundary and interface-versus-behavior distinction are projected
into `docs/spec/binding-spi.md`,
`docs/architecture/40-protocol-binding-spi-and-deployment.md`,
`docs/work-packages/WP-300-bindings.md`, the Property Read architecture-gate
document, D18 in `PLAN.md`, and `PROJECT_STATE.md`. The candidate record and
checks consume this decision; they are not a new design cycle.
