# 0044 Cleanup State-Machine Complexity and Evidence Boundary

Status: MIGRATED

Kind: owner-raised cleanup architecture-closure and implementation-risk investigation

Priority: HIGH

Target: the representation cost, reservation model, progress ownership, observability, residual-state meaning, authoring surface, and evidence timing behind the migrated complete-object cleanup decision

## Scope and authority

This topic follows the migrated `0035 Cleanup Protocol Implementability` decision. It does not reopen the requirement that every live cleanup-capable object retain one identifiable owner until complete cleanup, acknowledged complete-object transfer, or durable residual disposition.

It records a Project Owner concern that the selected cleanup semantics may remain correct while their Rust representation, reservation footprint, scheduling burden, observable state, Binding-author surface, or evidence sequence amplifies implementation complexity beyond what the migrated decision presently closes.

This topic does not prescribe a replacement cleanup protocol, generic hierarchy, reservation algorithm, scheduler, persistence layer, helper API, or protocol spike. It does not authorize destructor-only cleanup, record-only transfer, loss of first-cause or generation truth, or unacknowledged ownership transfer. It does not block WP-300 merely by existing. Codex owns the repository-grounded technical decision.

## Repository observations

- The migrated decision defines one linear semantic pattern: source-owned work is offered as a complete object, then either acknowledged as transferred or returned unchanged to a pre-reserved manual owner, and finally reaches complete cleanup or durable residual state.
- Binding authors currently see phase context, transfer envelopes, acceptance results, settlements or outcomes, typed successors, and an explicit `NoCleanupSuccessor` form; Servient owns executor queues, manual fallback, retry scheduling, status projection, and residual storage.
- Cleanup reservations are established before side effects, and distinct cancellation, readiness, abort, shutdown, response, subscription, emission, or remote-terminal phases may bind distinct contexts, deadlines, footprints, and ownership obligations.
- `PendingCleanup` is committed only after a named owner accepts the complete work object. `CleanupRecord` is status rather than progress-capable work.
- Host and constrained profiles are required to share transition and outcome semantics while using different storage, executor, wake, and progress representations.
- The narrow WP-300 Property Read slice exercises route and response cleanup through repository-owned mock authors but excludes subscriptions, emissions, broad cancellation races, multi-route scheduling, Servient cleanup coordination, and production protocol behavior.
- The first external Zenoh authoring spike follows narrow WP-300 completion and precedes broad WP-300 admission; authoritative production transport evidence remains downstream.
- The current specifications define the single-object transfer handshake in detail but do not presently close every question about aggregate type growth, mutually exclusive cleanup obligations, progress classification, restart durability, cleanup scheduling isolation, or authoring complexity across operation families.

## Questions for investigation

### Semantic kernel versus Rust representation growth

1. Which cleanup semantics must be represented by one shared transition kernel, and which operation-family work representations may differ without creating multiple cleanup protocols?
2. Does the current family of settlements, typed successors, transfer envelopes, host call boxes, route guards, response slots, subscription drivers, and emission slots produce avoidable generic nesting or enum multiplication?
3. What evidence would distinguish necessary type-level ownership precision from representation complexity that obscures the same linear cleanup semantics?
4. Can individually correct operation-specific successor types collectively create unacceptable compile diagnostics, monomorphization, binary-size growth, or maintenance duplication?
5. Which current public types are semantic contracts, which are transport containers, and which remain provisional implementation representations?
6. Could helpers or generated code preserve the current semantics while accidentally becoming a second effective cleanup protocol?
7. What exact implementation or measurement evidence would justify changing a cleanup representation without weakening complete-object ownership?

### Reservation independence and obligation coexistence

8. Which cleanup obligations can be simultaneously live for one call, route, subscription, response, emission, Thing generation, or Binding generation?
9. Which cleanup phases are mutually exclusive by state-machine construction even though they currently have distinct operation names or reservations?
10. Does the current requirement for independent reservations imply additive worst-case capacity for obligations that cannot coexist?
11. What artifact or evidence proves the compatibility and coexistence relation between readiness cancellation, route abort, route shutdown, response cancellation, subscription rollback or stop, emission cancellation, and remote-terminal cleanup?
12. How are nested or sequential cleanup obligations charged when one phase completes ownership transfer before another phase begins?
13. Could conservative reservation multiplication make valid constrained or gateway configurations inadmissible without improving actual ownership safety?
14. What evidence distinguishes a required independent reservation from duplicate accounting of the same retained object or storage?
15. Do current resource profiles and workload checks expose over-reservation separately from real cleanup saturation?

### Pending cleanup ownership and observability

16. What exact operational claim does `PendingCleanup` make beyond acknowledged ownership transfer?
17. Does the observable model distinguish executor-driven progress, caller-driven manual progress, waiting for a transport wake, deadline-due work, accepted-but-not-yet-polled work, and residual-commit work?
18. Which pending conditions require the application to continue calling a progress API, and which are owned entirely by the runtime?
19. How can an operator determine whether cleanup is advancing, stalled within admitted policy, awaiting external activity, or approaching durable residual disposition?
20. Which pending cleanup owners block plan-set reclamation, Binding replacement through a new Servient generation, root shutdown, or resource-capacity release?
21. Can bounded status projection preserve owner class, phase, age, deadline, progress condition, and blocking consequence without exposing protocol-private state?
22. What evidence prevents a transferred work object from remaining permanently pending while retaining capacity and generation leases indefinitely?

### Residual-state meaning and durability boundary

23. What does `durable residual state` guarantee in v1: retention for the current Servient lifetime, persistence across root shutdown, persistence across process restart, or only a terminal bounded record before local destruction?
24. Which residual identities and facts must remain available for audit, diagnosis, security review, or external compensation?
25. Which residual cases represent only possible remote resource leakage, and which represent continuing authorization, subscription, publication, or safety exposure?
26. Is residual persistence an engine responsibility, an application or platform integration responsibility, or explicitly outside v1 scope?
27. What happens to an accepted cleanup task and its residual fallback when the executor, root Servient, or process terminates?
28. Which current words, APIs, or status names could cause users to infer stronger restart or recovery guarantees than the implementation will provide?
29. What evidence is required before a residual state may be described as durable rather than merely reported once?

### Cleanup scheduling and saturation

30. Which component is the unique progress owner for transferred cleanup work in host and constrained profiles?
31. Are executor-owned cleanup, manual fallback cleanup, deadline wakeups, late-result settlement, and residual commitment one fairness domain or several independently bounded domains?
32. How are cleanup owners ordered when age, deadline, owner class, Binding identity, Thing identity, and work cost conflict?
33. What prevents one permanently pending protocol cleanup from monopolizing a queue slot, wake lease, deadline entry, or work quantum?
34. How does saturation of one cleanup queue or owner class affect new calls, routes, subscriptions, emissions, unrelated Bindings, and unrelated Things?
35. Which cleanup capacity is local to a route or Binding, and which capacity is necessarily Servient-wide?
36. Could the cleanup executor, manual fallback queue, deadline structure, status path, or residual store become a global contention point despite route and Thing sharding?
37. What deterministic evidence proves cleanup progress under zero budget, scarce budget, lost transport wake, executor rejection, executor shutdown, repeated hot foreground work, and multi-owner saturation?
38. Which findings would belong to cleanup implementation, which would reopen Servient scheduling architecture, and which would require lower-layer SPI correction?

### Binding-author surface and operation-family complexity

39. Which cleanup concepts must every Binding author understand, and which may remain engine-owned without hiding resource or lifecycle truth?
40. Does `NoCleanupSuccessor` cover only fully synchronous no-resource completion, or also common bindings whose protocol library exposes one simple owned cancel or close operation?
41. How many distinct cleanup paths must a minimal Property Read Binding, a session-based Binding, a subscription-capable Binding, and a constrained Binding implement directly?
42. Which authoring burden is mechanical repetition, and which burden reveals that an operation-family cleanup contract is decomposed incorrectly?
43. Can a Binding use a protocol reactor as the retained cleanup owner without moving semantic ownership into an unregistered task or hidden side table?
44. What diagnostics are produced when an author loses a complete object, returns the wrong generation, acknowledges twice, exceeds declared footprint, or cannot express a library-native cancellation model?
45. What repeated workaround, private dependency, unsafe escape, generic failure, or protocol mismatch would justify reopening the public cleanup surface?
46. Does the current reopening threshold recognize an implementable but systematically error-prone cleanup API, or only outright ownership and representation impossibility?

### Evidence timing and maturity claims

47. What cleanup claim is justified when the narrow WP-300 mock implementation completes?
48. Which cleanup properties remain unproved until the narrow WP-400 Servient slice, broad cancellation matrices, subscriptions, emissions, multi-route cleanup scheduling, and production Binding work execute?
49. What exact cleanup evidence must the external Zenoh authoring spike record beyond successful construction or compilation?
50. Could repository-owned mock authors satisfy the transfer protocol while hiding friction caused by a production library's callbacks, shared sessions, reactor ownership, or non-returning cancellation API?
51. At which checkpoint must generic growth, code size, constrained storage, reservation pressure, scheduler saturation, and author diagnostics be measured?
52. Which cleanup defects may be corrected as normal candidate implementation feedback, and which require reopening ADR-0011, the Binding SPI, WP-300, or WP-400 architecture?
53. Do any current architecture, specification, work-package, milestone, risk, or continuation statements imply stronger cleanup implementability, scalability, observability, or durability than repository evidence presently supports?

### Existing decision intersection

54. Which questions in this topic are already fully resolved by the migrated `0035` decision, ADR-0011, `docs/spec/binding-spi.md`, WP-300, WP-400, existing state machines, or registered evidence?
55. Which questions are intentionally deferred implementation choices that should remain open until source work provides concrete evidence?
56. Which questions are unresolved architecture or public-SPI contracts that must be closed before broad WP-300 or broad WP-400 admission?
57. Does any unresolved concern intersect the reviewed narrow WP-300 Property Read implementation, or can it remain downstream without weakening that tranche's exact claim?
58. Would any additional requirement, artifact, test, workload, or maturity boundary protect a distinct falsifiable claim not already owned by existing evidence?

## Constraints

- Preserve complete-object ownership until verified completion, acknowledged transfer, or durable residual disposition.
- Preserve first-cause, deadline, generation, resource, late-result, and residual-state truth.
- Do not treat `CleanupRecord`, logging, task drop, future drop, or destructor execution as the cleanup work object or as proof of successful cleanup.
- Do not infer that the cleanup design is defective solely because broad WP-300, WP-400, or WP-600 source is not yet implemented.
- Do not treat generic count, field count, API length, or subjective complexity alone as proof of a semantic defect.
- Do not prescribe a generic hierarchy, operation-family enum, reservation-sharing scheme, scheduler, persistence mechanism, authoring tier, helper API, or protocol adapter before Codex classifies the questions against repository evidence.
- Do not reopen the migrated complete-object ownership decision without concrete intersecting ownership, lifecycle, resource, portability, safety, or implementability evidence.
- Do not add a blocking gate unless it protects a distinct falsifiable claim not already covered by an existing work package, state machine, fixture, workload, or evidence owner.
- Do not block WP-300 merely because this topic is OPEN.

## Expected decision output

Codex should:

1. classify which concerns are already resolved, intentionally deferred, or genuinely unclosed;
2. define the boundary between the invariant cleanup semantic kernel and operation/profile-specific representation freedom;
3. determine whether reservation coexistence, pending-state observability, residual durability, cleanup scheduling, or authoring surface require additional authoritative closure;
4. determine whether current implementation and external-evidence timing are sufficient;
5. identify any unsupported cleanup maturity, durability, scalability, or ergonomics claim;
6. identify any architecture, ADR, specification, work package, state machine, resource profile, fixture, workload, checker, audit, plan, risk projection, or continuation record that requires correction; and
7. migrate only conclusions supported by repository evidence.

## Decision

The complete-object cleanup invariant remains frozen: ownership ends only at
verified completion, acknowledged transfer, or an explicit durable residual
disposition. Exact Rust containers, generic parameters, helper layers, and
operation-family layout remain provisional implementation choices so long as
they preserve that semantic kernel.

Broad cleanup admission now requires an operation-by-obligation coexistence
matrix. Obligations that can be live together reserve independent bounded
capacity; obligations proven mutually exclusive may reuse capacity and must
not be counted additively. A generic slot count or a sum of all declared
operation costs is not evidence for either case.

Every observable `Pending` cleanup state must expose a unique progress owner,
the retained complete object, deadline or bound, and the condition blocking
completion. The progress owner may be a static driver in a constrained
profile or a Servient scheduling domain on Host, but it cannot be ambiguous or
depend on task destruction. Cleanup receives its own bounded progress
authority where sharing a hot interaction lane could starve it.

For v1, “durable residual” means retained in-instance authority plus a bounded
final shutdown report. Process-restart persistence is not claimed. A stronger
restart-survival claim requires an explicit persistence format, recovery
owner, and executable evidence rather than an overloaded cleanup record.

The external Zenoh authoring checkpoint must record operation-to-cleanup
mapping, diagnostics, generic/layout pressure, and code-size impact. Broad
WP-300 and broad WP-400 remain gated on that evidence; the narrow Property Read
slice remains unblocked because its exact cleanup topology is already finite.

## Migration

The coexistence matrix, pending observability, progress ownership, v1
durability boundary, and external-authoring evidence are projected into
`PLAN.md`, `docs/spec/binding-spi.md`,
`docs/architecture/50-servient-runtime-lifecycle.md`,
`docs/state-machines.toml`, `docs/work-packages/WP-300-bindings.md`, and
`docs/work-packages/WP-400-servient.md`. This topic is `MIGRATED`.
