# 0033 Servient Orchestration Concentration

Status: OPEN

Kind: owner-raised runtime architecture and scalability investigation

Priority: HIGH

Target: Servient ownership of route lifecycle, publication, fairness, handler dispatch, status, cancellation, and cleanup

## Scope and authority

This topic records a Project Owner concern that removing hidden Binding dispatch correctly centralizes semantic authority but may also concentrate excessive state-machine, scheduling, and performance responsibility in Servient. It does not propose returning dispatch authority to bindings. Codex owns the architecture judgment.

## Repository observations

- Servient owns admission, route records, serving publication, route permits, handler dispatch, fairness, cancellation, status, and cleanup.
- Bindings may run bounded protocol reactors but only wake engine-owned drivers.
- The target Servient orchestration is not yet implemented.
- Required evidence includes many-route readiness fairness, never-ready routes, drain/accept races, terminal isolation, and cleanup at each lifecycle stage.
- The first Property Read gate exercises only one route and one request.

## Questions for investigation

1. Which state and scheduling responsibilities must remain centralized for correctness?
2. Which protocol-local progress can remain encapsulated without creating a second semantic authority?
3. What data structures and leases prevent Servient from becoming a global lock or polling hotspot?
4. How are fairness and work budgets composed across Things, bindings, routes, handlers, responses, and cleanup owners?
5. Which multi-route and load tests must occur before broad WP-400 completion?
6. Can host and constrained runtimes share lifecycle logic without sharing an unsuitable synchronization representation?
7. What evidence would expose unacceptable central contention or scheduler complexity early?

## Constraints

- Do not reintroduce binding-owned handler dispatch, registry observation, or detached semantic ownership.
- Do not infer scalability from the single-route Property Read gate.
- Preserve bounded work, generation rejection, and cleanup ownership.
- Prefer measured runtime evidence over speculative decomposition.

## Expected decision output

Codex should define the minimal Servient state/scheduling architecture, the earliest deterministic scalability workloads, the acceptable protocol-reactor boundary, and any work-package or performance-evidence changes needed before broad runtime claims.