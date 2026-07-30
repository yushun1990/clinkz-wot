# 0033 Servient Orchestration Concentration

Status: MIGRATED

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

## Decision

Servient retains the single semantic authority for publication, route/plan
leases, permit claims, handler dispatch, cancellation, status, and cleanup.
That authority does not require one process-wide lock or polling scan.
Implementation is partitioned by Thing/plan-set generation and then by route
or operation slot, with bounded ready queues, one retained cursor per owner,
brief claim/commit critical sections, and callbacks outside those sections.
Status/event storage is per binding or bounded shard.

A binding reactor may own protocol I/O, correlation, transport buffers, local
credit, and wake production. It may wake only the engine-owned route/call
driver and cannot observe registries, choose handlers, or retain semantic
permits. Host and constrained profiles share transition and outcome semantics
but use separate synchronization/storage representations.

Scheduling uses one linear work budget with bounded per-owner quanta across
readiness, acceptance, handlers, responses, subscriptions, emissions,
reclamation, and cleanup. Older cleanup and deadlines cannot be starved by hot
routes. The earliest broad WP-400 workloads must cover multiple Things and
bindings, many routes, a never-ready route, continuously ready and slow
siblings, drain/accept races, terminal isolation, cleanup saturation, and
contention-free progress of unrelated shards. The single-route Property Read
gate makes no scalability claim.

## Migration

The minimal architecture and evidence boundary are projected into
`docs/work-packages/WP-400-servient.md`; existing registered fairness and
contention workloads remain the executable owners. This topic is `MIGRATED`.
