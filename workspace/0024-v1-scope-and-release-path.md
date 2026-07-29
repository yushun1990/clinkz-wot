# 0024 v1 Scope and Release Path

Status: OPEN

Kind: owner-raised release-risk investigation

Priority: HIGH

Target: the remaining milestone, work-package, architecture-gate, integration, and release path for ClinkZ-WoT v1

## Scope and authority

This topic records a Project Owner concern about whether the current v1 target has a bounded, internally consistent, and evidence-grounded path from the present WP-200 state through Protocol Binding, Servient, Directory client, Zenoh migration, umbrella integration, conformance, and release review.

The concern is an investigation input. It does not assert that the v1 scope is too large, does not select a smaller release, and does not prescribe milestone removal, reprioritization, or a release schedule. Codex owns the repository-grounded technical judgment and release-readiness model.

## Repository observations

The repository records that:

- v5.0 bounded-core authority is active while several global architecture gates remain open;
- M2 Core stabilization remains in progress;
- M3 Planning, M4 Protocol Binding SPI, M5A Servient, M5B Directory and Discovery client, M5C Zenoh and zenoh-pico migration, M6 umbrella integration, and M7 release review remain open;
- the first Property Read vertical path has completed only its WP-100 handler slice;
- WP-400, WP-500, and WP-600 depend on WP-300, while WP-700 joins those branches;
- the release target is a protocol-neutral W3C WoT runtime with a stable Servient architecture and Zenoh binding support;
- technical release readiness is an AI evidence judgment, while actual publication is an Owner decision.

## Owner concern

The Project Owner is concerned that the repository may have a clear package order without yet demonstrating that the total remaining v1 scope, cross-branch dependencies, global gates, production-binding work, conformance obligations, and release evidence form a finite and credible execution path. The concern is also whether local authority or contract progress can create an overly optimistic impression of release proximity.

## Questions for investigation

1. What exact mandatory behavior, package completion, architecture-gate closure, integration evidence, conformance evidence, and release evidence remain before v1 technical readiness?
2. Which remaining requirements are indispensable to the stated v1 release claim, and which are domain-entry obligations that may not affect the first release boundary?
3. What is the complete dependency graph across M1 through M7, including shared and branch-specific blockers?
4. Which branches can progress independently after WP-300, and which must rejoin before any meaningful executable or release claim?
5. Does the Directory and Discovery client scope contribute directly to the stated v1 release target, and how is that dependency represented?
6. What exact Zenoh and zenoh-pico behaviors are required for the first production binding claim?
7. What end-to-end interactions, profiles, feature cells, constrained targets, resource workloads, lifecycle cases, and failure cases must pass before release?
8. Are any global gates capable of invalidating completed package evidence late in the release path?
9. Does the current plan expose the dominant release-critical path rather than only the package order?
10. How does the repository distinguish architecture closure, package completion, executable vertical integration, production readiness, and release conformance?
11. What evidence would prove that the current v1 scope is bounded and achievable without relying on percentage estimates or informal confidence?
12. If the path is not credible or internally consistent, which authoritative release target, milestone, work-package, gate, or conformance owner must consume the decision?

## Constraints

- Do not assume that the remaining scope is excessive because many milestones remain open.
- Do not assume that v1 must be reduced, delayed, split, or redefined.
- Do not prescribe a minimal release, feature removal, milestone reorder, deadline, or roadmap before investigation.
- Do not treat documentation, checker, or local contract completion as executable or release progress without matching evidence.
- Do not weaken protocol neutrality, host/constrained support, lifecycle safety, resource bounds, production binding behavior, or conformance claims.
- Preserve the distinction between AI technical readiness judgment and Owner publication authority.

## Expected decision output

Codex should determine:

1. the exact remaining v1 technical and evidence scope;
2. the dominant and branch-specific dependency paths to release readiness;
3. whether the current release target and milestone model are internally consistent and bounded;
4. whether any progress or readiness claim requires correction;
5. whether an authoritative release, milestone, work-package, gate, or conformance decision is required;
6. the conditions for moving this topic through its workspace lifecycle.
