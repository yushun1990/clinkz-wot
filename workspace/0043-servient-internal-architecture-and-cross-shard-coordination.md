# 0043 Servient Internal Architecture and Cross-Shard Coordination

Status: MIGRATED

Kind: owner-raised runtime architecture-closure and implementation-readiness investigation

Priority: HIGH

Target: the internal decomposition, scheduling domains, cross-shard coordination, semantic-kernel ownership, tranche boundaries, and earliest executable evidence behind the migrated Servient orchestration decision

## Scope and authority

This topic follows the migrated `0033 Servient Orchestration Concentration` decision. It does not reopen the conclusion that Servient retains the single semantic authority for publication, route and plan leases, permit claims, handler dispatch, cancellation, status, and cleanup.

It records a Project Owner concern that the migrated decision may close the high-level ownership and sharding direction without yet closing enough of the internal implementation architecture to prevent a monolithic state machine, a universal scheduler, cross-domain lock coupling, host/constrained drift, or late discovery of scalability defects.

This topic does not prescribe an internal module layout, scheduler design, lock protocol, work-package split, or new validation gate. It does not block WP-300 or the reviewed narrow WP-400 Property Read path merely by existing. Codex owns the repository-grounded technical decision.

## Repository observations

- The migrated D26 decision retains one Servient semantic authority while partitioning host storage and scheduling by Thing/plan-set generation and then by route or operation slot.
- The decision requires bounded ready queues, retained cursors, brief claim/commit critical sections, callbacks outside locks, per-binding or bounded-shard status storage, bounded per-owner work quanta, and non-starvation of older cleanup and deadlines.
- The Servient runtime specification requires maintained queues or cursors instead of full-record readiness scans and defines a two-phase claim/callback/commit boundary.
- `WP-400` owns registration snapshots, plan-set lifecycle, produced and consumed handles, route and operation scheduling, exposure transactions, handler coordination, subscriptions, emission coordination, cleanup ownership, status, reclamation, host runtime, and constrained runtime composition.
- The exact Property Read Servient slice exercises one route and one request and makes no scalability claim.
- Broad WP-400 evidence registers multi-Thing, multi-binding, multi-route, never-ready, continuously hot, slow, draining, cleanup-heavy, contention, fairness, semantic-parity, and resource-accounting scenarios.
- Current authoritative text names the sharding direction and required outcomes but does not presently identify a complete internal subsystem ownership map, permitted dependency graph, scheduling-domain boundary, cross-shard coordination protocol, or unique code owner for the shared host/constrained semantic transition kernel.
- Current authoritative text registers extensive broad WP-400 evidence but does not presently state whether broad WP-400 will be admitted, reviewed, and completed through smaller independently reversible internal tranches.

## Questions for investigation

### Internal ownership and decomposition

1. Does the current D26 decision close only Servient's semantic authority and storage-sharding direction, or does it also close the required internal subsystem decomposition?
2. Which Servient-owned state must belong to distinct internal owners for registration snapshots, plan sets, produced lifecycle, consumed lifecycle, route progress, operation progress, handlers, subscriptions, emissions, cleanup, resources, status, and reclamation?
3. Which private records or coordinators, if any, may inspect or mutate more than one of those ownership domains?
4. What dependency direction must hold between registry structure, per-Thing lifecycle state, per-route state, per-binding state, cleanup ownership, status, and global resource accounting?
5. Which parts of the internal decomposition are architecture requirements, and which may remain implementation choices until measured evidence exists?
6. What repository evidence would distinguish a valid Servient facade over bounded internal owners from a monolithic `ServientInner` that merely stores sharded maps?
7. Could independently correct state machines still create excessive coupling through shared records, callbacks, error paths, or cleanup ownership?

### Work budget and scheduling domains

8. Does the decision to use one linear `WorkBudget` require one universal scheduler, or only one common bounded accounting model?
9. Which work classes share a fairness domain, and which require independently retained ready queues, cursors, deadlines, or progress policies?
10. How are readiness, acceptance, handlers, responses, subscriptions, emissions, cleanup, reclamation, and lazy planning ordered when several classes are simultaneously ready?
11. What does non-starvation mean for work classes whose urgency, lifetime, side effects, and deadlines differ?
12. How are per-owner quanta composed across route, Thing, binding, operation, cleanup owner, and global runtime scopes?
13. Which scheduling decisions must be deterministic across host and constrained profiles, and which may differ without changing observable semantics?
14. What evidence would reveal that one common budget or queue has become a hidden global coordination point?
15. How are zero-budget behavior, wake retention, deadline progress, and repeated hot readiness validated across independent scheduling domains?

### Cross-shard state and resource coordination

16. Are Thing/generation and route/operation-slot shards sufficient to represent every Servient-owned runtime responsibility?
17. Which runtime objects naturally cross Thing shards, including binding reactors or sessions, per-binding ingress and resource limits, cleanup executors, status aggregation, emission lanes, plan reclamation, and global resource ceilings?
18. May one operation hold or mutate more than one shard at a time?
19. If multi-shard access is permitted, what ordering, ownership, retry, rollback, and revalidation rules make it safe and bounded?
20. If multi-shard access is forbidden, what complete-object, lease, reservation, command, or acknowledgement boundary carries work between owners?
21. How are failed reservations or rejected transfers returned without losing generation, capacity, cleanup, or primary-failure ownership?
22. Could the global resource ledger, binding-level quota, status path, or cleanup owner become a process-wide hot lock even when Thing and route state are sharded?
23. How does a protocol reactor that serves multiple Things map wakeups into exact engine-owned route or call drivers without scanning or acquiring unrelated shard state?
24. What evidence proves that draining or saturating one Thing, binding, cleanup queue, or status shard cannot block unrelated owners?

### Shared host and constrained semantic kernel

25. What exact code or generated artifact owns the representation-independent Servient state-transition truth?
26. May host `poll` paths and constrained `step` paths implement the same normative transitions separately if trace tests currently agree?
27. Who computes generation validation, terminal outcomes, cleanup dispositions, and resource deltas shared by both profiles?
28. What prevents host synchronization conveniences from becoming a second effective runtime contract?
29. What prevents constrained storage or manual-progress constraints from forcing unnecessary structural complexity into the host execution surface?
30. Which differences between host and constrained scheduling are representational, and which would indicate semantic divergence?
31. What evidence detects drift before one profile has accumulated substantial profile-specific implementation?

### Broad WP-400 execution and feedback timing

32. Does broad WP-400 currently have one implementation-admission and completion boundary, or are independently reviewable internal tranches already implied elsewhere?
33. Which broad WP-400 concerns have distinct ownership, rollback, evidence, or failure boundaries that prevent them from being safely reviewed as one tranche?
34. Which concerns must remain joined because separating them would create a second contract, duplicate state machine, or invalid completion claim?
35. At what checkpoint should the first multi-owner, multi-shard executable feedback occur?
36. What minimum executable scenario is required to test sharding, fairness, cleanup progress, drain isolation, and callback lock boundaries without claiming broad scalability?
37. Could waiting for the registered broad WP-400 workloads allow substantial lifecycle, subscription, emission, or cleanup implementation to depend on an unsuitable scheduler or shard topology?
38. Which scalability or contention findings may be corrected locally, and which would require reopening D26, WP-400 architecture, or lower-layer SPI contracts?
39. What maturity claim is justified after the single-route Property Read architecture gate, before any multi-owner runtime evidence exists?
40. Do any current architecture, work-package, milestone, evidence, or continuation statements imply more internal Servient closure than the repository presently proves?

### Existing authority and decision intersection

41. Which questions in this topic are already fully resolved by D26, D28, D29, `docs/architecture/50-servient-runtime-lifecycle.md`, `docs/work-packages/WP-400-servient.md`, or registered workloads?
42. Which questions are intentionally deferred implementation choices that should not be closed before WP-400 source work exposes concrete evidence?
43. Which questions represent unresolved architecture contracts that must be closed before broad WP-400 admission?
44. Does any unresolved item intersect the narrow WP-400 Property Read slice, or can it remain downstream without weakening that slice's exact claim?
45. Would any new requirement or evidence checkpoint protect a distinct falsifiable claim not already owned by the existing work package and workloads?

## Constraints

- Do not reintroduce binding-owned handler dispatch, registry observation, serving authority, or detached semantic ownership.
- Do not infer that the Servient architecture is defective solely because WP-400 is not yet implemented.
- Do not treat absent source code, queue types, lock types, or module names as defects when they are valid downstream implementation choices.
- Do not treat sharded maps alone as proof of independent progress, bounded contention, or non-starvation.
- Do not infer scalability from the single-route Property Read architecture gate.
- Do not prescribe a module tree, scheduler lane structure, actor model, lock-free design, executor, or tranche split before Codex classifies the unresolved contract.
- Do not add a blocking gate unless it protects a distinct falsifiable claim not already covered by an existing work package, machine, workload, or evidence owner.
- Do not block WP-300 or the narrow WP-400 Property Read path merely because this topic is OPEN.

## Expected decision output

Codex should:

1. classify which concerns are already resolved, intentionally deferred, or genuinely unclosed;
2. define the exact boundary between Servient semantic authority, internal architecture requirements, and implementation freedom;
3. determine whether scheduling domains, cross-shard coordination, and semantic-kernel ownership require additional authoritative closure;
4. determine whether the current WP-400 tranche and evidence timing are sufficient;
5. identify any unsupported maturity or scalability claim;
6. identify any architecture, specification, work-package, machine, workload, checker, audit, plan, or continuation projection that requires correction; and
7. migrate only conclusions supported by repository evidence.

## Decision

D26 already closes Servient as the owner of orchestration, registry
observation, interaction acceptance, handler dispatch, publication, and
cross-binding policy. Exact Rust module boundaries, map types, executor
choices, and queue representations remain implementation choices. That
authority decision did not, however, close the internal ownership graph needed
before broad WP-400 work.

Broad WP-400 must preserve one private mutable owner per lifecycle shard and a
one-way dependency graph. A cross-shard transition transfers a complete owned
object or immutable fact, receives an explicit acknowledgement where required,
and revalidates generation and deadline state before committing the next
transition. A callback may not retain simultaneous mutable authority over two
shards. Common `WorkBudget` accounting does not require one universal queue:
acceptance, binding progress, handler execution, response, publication, and
cleanup may use distinct scheduling domains while consuming the same bounded
authority.

Servient also owns one shared semantic transition kernel and machine-readable
trace oracle for behavior common across bindings and profiles. Per-route or
per-profile orchestration may adapt inputs and drive progress, but may not
duplicate normative transition logic.

Before broad WP-400 admission, an early executable feedback checkpoint must
exercise at least two Things, two binding profiles, and a multi-route
generation, including a hot route, a never-ready or stalled route, and cleanup
progress. This protects ownership, non-starvation, and cross-shard handoff
claims before a large implementation tranche accumulates. It is not a claim
of arbitrary-scale throughput.

The already bounded Property Read tranche remains admissible because it does
not claim the broad internal topology, scheduling, or scalability closure.

## Migration

The owner graph, scheduling-domain boundary, cross-shard handoff rules, shared
kernel ownership, and early feedback checkpoint are projected into `PLAN.md`,
`docs/architecture/50-servient-runtime-lifecycle.md`, and
`docs/work-packages/WP-400-servient.md`. This topic is `MIGRATED`.
