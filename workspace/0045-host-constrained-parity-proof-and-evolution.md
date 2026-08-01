# 0045 Host and Constrained Parity Proof and Evolution

Status: MIGRATED

Kind: owner-raised cross-profile semantic-closure and evidence-risk investigation

Priority: HIGH

Target: shared semantic-kernel ownership, resource and liveness equivalence, capability boundaries, host-default projection, trace-oracle authority, async-no-std claims, API evolution, and production cross-profile evidence behind the migrated Host/constrained parity decision

## Scope and authority

This topic follows the migrated `0036 Host and Constrained Semantic Parity` decision. It does not reopen the conclusion that Host-erased execution and constrained caller-owned execution must implement one representation-independent semantic contract while retaining profile-appropriate storage, dispatch, wake, synchronization, allocation, and executor mechanics.

It records a Project Owner concern that the migrated decision clearly defines the parity objective and several observable comparisons, but may not yet close enough of the code ownership, liveness, resource taxonomy, capability taxonomy, default expansion, trace authority, evidence timing, or evolution contract to prevent two implementations from passing nominally identical cases while diverging in progress guarantees, costs, supported behavior, or upgrade impact.

This topic does not prescribe a shared module layout, pure-function kernel, generated state machine, differential harness, trace format, executor, default policy, compatibility scheme, production backend, or new validation gate. It does not require identical physical representation or identical capability sets across profiles. It does not block WP-300 or the reviewed narrow Property Read path merely by existing. Codex owns the repository-grounded technical decision.

## Repository observations

- The migrated decision defines identity checks, lifecycle transitions, ownership classifications, terminal retention, cancellation settlement, cleanup transfer, generation rejection, resource charges, and observable outcomes as one representation-independent semantic kernel.
- Host erasure and constrained associated-state storage are required to adapt that kernel rather than implement separate outcome rules.
- Evidence that claims both profiles uses shared versioned trace case ids and compares transitions, outcomes, resource deltas, late-result handling, terminal acknowledgement, and clear or reuse behavior.
- Allocation, trait-object versus enum dispatch, waker versus caller polling, synchronization containers, and executor presence are permitted representation differences.
- A different accepted input, terminal class, cleanup owner, generation result, or resource charge is classified as semantic drift.
- The Binding SPI states that Host and constrained delivery paths share identical observable outcomes and resource deltas, while storage, dispatch, waker, executor, and critical-section mechanics may differ.
- The constrained profile uses associated state, caller-owned typed slots, static tables, and explicit `WorkBudget`; the Host profile may use erased owned calls, boxes, wakers, reactors, executor integration, sharded status, and named defaults.
- The narrow Property Read evidence uses repository-owned Host and static authors and limited runtime scenarios. Broader subscriptions, emissions, cleanup, fairness, async behavior, production protocol integration, and long-term API evolution remain downstream.
- The `async-no-std` cell may remain compile-only while no runtime claim is made, but deterministic executor-neutral runtime traces are required once async cancellation, wake, deadline, or cleanup behavior is claimed.
- Current authoritative text names a shared semantic kernel and shared trace ids but does not presently identify the unique code owner of every shared transition, the unique machine-readable owner of each trace scenario, a complete capability-parity taxonomy, a normalized semantic-versus-physical resource model, or an explicit cross-version parity contract.

## Questions for investigation

### Shared semantic-kernel ownership

1. What exact code, generated artifact, table, or other repository owner contains the representation-independent semantic truth shared by Host and constrained execution?
2. May Host and constrained paths separately implement the same transition rules if shared traces currently agree?
3. Which lifecycle decisions must be computed once and consumed by both profile adapters rather than recomputed independently?
4. Who computes accepted-input classification, generation validation, ownership disposition, terminal class, cleanup owner, retry class, and semantic resource delta?
5. Which profile-specific records may contain transition logic, and which may only contain representation and driving mechanics?
6. What evidence distinguishes a real shared semantic kernel from two implementations constrained only by duplicated assertions?
7. Could a shared helper surface still conceal profile-specific outcome decisions in callbacks, adapters, conversions, or error mapping?
8. How is semantic-kernel versioning tied to trace-case versioning and public API versioning?
9. Which changes to a Host or constrained adapter require review as a semantic-kernel change rather than a representation-only change?

### Resource equivalence

10. What does `identical resource deltas` mean across profiles with different allocation, storage, dispatch, wake, executor, and synchronization mechanics?
11. Which resource units are semantic and therefore comparable across profiles, including calls, route leases, response opportunities, subscriptions, cleanup obligations, ingress items, and generation pins?
12. Which resource costs are physical or profile-specific and therefore must be reported without being required to match?
13. Could Host allocation, task, queue, waker, or synchronization costs be omitted from accounting because they are not part of the constrained semantic delta?
14. Could constrained slot size, alignment, closed-enum growth, static table capacity, or code-size cost be hidden because the Host path has no matching unit?
15. How are semantic reservations related to profile-specific backing storage without forcing one profile to simulate the other?
16. What normalized comparison proves equal admission and release semantics while allowing different physical cost?
17. Does any current requirement or evidence key use `resource delta` in a way that could be interpreted as physical equality?
18. Which resource differences indicate legitimate representation cost, and which reveal different ownership or admission semantics?

### Safety and liveness parity

19. Is parity complete when both profiles eventually reach the same terminal outcome, or must they also satisfy equivalent progress obligations?
20. Which liveness properties are semantic across profiles: zero-budget immobility, wake retention, bounded deadline progress, non-starvation, cancellation linearization, terminal visibility, cleanup progress, and reclamation eligibility?
21. How are Host wake-driven progress and constrained caller-driven `step` progress compared without assuming equal wall-clock time or executor behavior?
22. What normalized work unit or transition bound, if any, is required to compare progress under explicit budgets?
23. Can one profile require materially more logical progress calls for the same semantic transition while still claiming parity?
24. What evidence proves that a constrained ready owner cannot remain indefinitely unpolled while the Host counterpart progresses automatically?
25. What evidence proves that Host wake coalescing, task scheduling, or executor shutdown cannot weaken a progress guarantee available to the constrained path?
26. How are deadlines compared when one profile receives timer wakes and the other depends on caller-supplied time and repeated stepping?
27. Which fairness differences are permitted policy choices, and which produce observable semantic drift?
28. Are outcome traces sufficient to detect starvation, lost wake, delayed cancellation, or permanently retained terminal state?

### Capability and applicability taxonomy

29. Must parity apply only to capabilities supported by both profiles, or does the current design imply identical capability sets?
30. How are `supported in both`, `Host-only`, `constrained-only`, `compile-only`, `not applicable`, and `same semantics with different driving` represented authoritatively?
31. Which Host conveniences are merely adapters over a common capability, and which are separate capabilities not promised by constrained profiles?
32. Does the absence of async handlers, an executor, a configurable Host dispatch policy, or automatic cleanup driving in a constrained profile constitute unsupported capability or semantic divergence?
33. How does registration validation reject a capability that one selected profile cell cannot implement without changing the common semantic contract?
34. Can a complete Binding bundle advertise different profile-cell capability subsets while retaining one Binding identity and configuration digest?
35. What happens when a plan or application request selects behavior available in one profile but unavailable in another?
36. How are tests prevented from comparing non-equivalent capability scenarios and labeling the difference semantic drift?
37. Which capability differences are release-visible and which remain internal authoring distinctions?

### Host defaults and convenience layers

38. Which Host defaults are expanded into explicit semantic inputs before the shared kernel executes?
39. Do default timeouts, work budgets, retry limits, overflow policies, cleanup driving, status retention, or emission policies alter accepted input or observable outcomes?
40. What is the constrained equivalent of a Host operation whose progress is automatically driven by an executor or background wake integration?
41. Can Host automatic cleanup or drop convenience perform semantic work that constrained drop cannot perform?
42. Can a Host queue or executor policy change visible backpressure, cancellation order, deadline behavior, event ordering, or residual classification?
43. How are executor rejection and shutdown compared with a constrained caller that stops invoking `step` or `poll_cleanup`?
44. Which Host default snapshots prove policy values only, and which must also prove equivalence after expansion into kernel inputs?
45. What prevents a convenience adapter from becoming a second effective runtime contract?
46. Which Host-only diagnostics or status events are permitted without changing the shared observable semantic result?

### Trace-oracle authority and test symmetry

47. What is the unique repository owner of each shared trace scenario's initial state, input event, budget, clock, expected transition, resource delta, and terminal projection?
48. Do Host and constrained runners consume one shared scenario value, or may each fixture independently reconstruct the same case id?
49. How is accidental asymmetry detected when one runner omits an assertion, uses a different pre-state, advances time differently, or performs extra progress calls?
50. Can both profiles pass the same trace id while sharing the same incomplete expectation?
51. Which trace dimensions are compared structurally rather than through profile-specific prose assertions?
52. How are unsupported or not-applicable capabilities represented without silently deleting cases from one profile?
53. What negative mutations prove that the parity harness detects changed terminal class, owner, generation result, resource charge, deadline behavior, wake behavior, or clear and reuse behavior?
54. How are trace-case changes reviewed when a kernel rule, public API, or resource taxonomy changes?
55. At what point does a growing trace matrix itself become too indirect to establish shared implementation truth?

### Async-no-std claim boundary

56. Which work package or milestone first makes an `async-no-std` runtime claim rather than a surface-availability claim?
57. What exact behavior moves the cell from compile-only evidence to executor-neutral runtime evidence?
58. Can successive work packages each defer runtime evidence by claiming that the underlying poll kernel already owns semantics?
59. If v1 does not provide an executable async-no-std integration, how is the cell described without implying cancellation, wake, deadline, or cleanup parity?
60. Is a compile-only cell part of the release capability matrix, the authoring surface matrix, or both?
61. What harness can exercise async adapters without selecting a production executor or weakening the no-executor contract?
62. Which runtime claims can be inherited from the no-default poll path, and which require adapter-specific evidence?
63. What prevents the async adapter from adding buffering, wake, cancellation, or drop behavior absent from the poll contract?

### API and representation evolution

64. What parity guarantee applies across versions rather than only within one repository revision?
65. Can a public API change be representation-compatible for Host erasure while forcing constrained applications to regenerate closed enums, enlarge slots, change alignment, or rebuild static tables?
66. How are profile-specific breaking costs classified when the observable semantic contract remains unchanged?
67. Which additions to operation state, cleanup successors, handler forms, capability metadata, or resource maxima require a constrained layout-version change?
68. How are older Host Binding crates and older constrained application tables treated when the shared kernel or trace version changes?
69. Does one Binding semantic version imply identical compatibility guarantees for Host and constrained installation units?
70. Which compatibility claims belong to Rust source compatibility, binary compatibility, generated-artifact compatibility, state-layout compatibility, or semantic compatibility?
71. Could Host convenience evolution become the de facto design driver because it is easier to adapt than constrained closed representations?
72. What evidence is required before an SPI change is described as equally portable across profiles?

### Production cross-profile evidence

73. Which production or protocol-backed scenario first compares Host and constrained execution beyond repository-owned mocks?
74. What common capability intersection exists between the initial Host Zenoh backend and constrained zenoh-pico backend?
75. How are differences in callbacks, sessions, query lifetimes, cancellation, buffering, reactor ownership, and wake integration normalized into shared semantic inputs?
76. Which backend differences are explicit unsupported capabilities rather than hidden adapter behavior?
77. Does using one protocol family provide sufficient diversity to expose runtime-representation drift, or only protocol-equivalence evidence?
78. What differential evidence compares acceptance, cancellation, late results, cleanup, deadlines, backpressure, and resource accounting across the two backends?
79. Which production findings may be fixed in one adapter, and which require reopening the common kernel, capability taxonomy, resource model, or public SPI?
80. At what checkpoint must production-backed parity exist before the project claims constrained runtime maturity?

### Existing decision intersection and maturity claims

81. Which questions in this topic are already fully resolved by the migrated `0036` decision, the Binding SPI, WP-300, WP-400, state machines, feature-cell matrix, or registered evidence?
82. Which questions are intentionally deferred implementation choices that should remain open until source work produces concrete evidence?
83. Which questions are unresolved architecture, public-SPI, capability, or evidence contracts that must be closed before broad WP-300, broad WP-400, or constrained release claims?
84. Does any unresolved item intersect the reviewed narrow WP-300 Property Read implementation, or can it remain downstream without weakening that tranche's exact claim?
85. What parity maturity claim is justified after mock Host/static Property Read traces but before async, broad lifecycle, and production backend evidence?
86. Do current architecture, specification, work-package, milestone, risk, feature-cell, or continuation statements imply stronger liveness, resource, capability, evolution, or production parity than repository evidence presently supports?
87. Would any additional requirement, artifact, trace, workload, or maturity boundary protect a distinct falsifiable claim not already owned by existing evidence?

## Constraints

- Preserve one representation-independent semantic state and outcome model for every capability claimed by both profiles.
- Do not impose `Arc`, atomics, threads, dynamic allocation, trait-object dispatch, or an executor on constrained profiles.
- Do not make Host behavior normative merely because it is easier to execute, observe, or test.
- Do not require identical physical storage, allocation, dispatch, synchronization, wake mechanics, code size, or capability sets where the profile contract permits differences.
- Do not treat compilation, shared type names, shared case ids, or equal terminal enums alone as proof of runtime parity.
- Do not infer semantic drift solely from profile-specific implementation cost, unsupported capability, or absent downstream source.
- Do not prescribe a kernel representation, code-generation scheme, trace format, differential harness, executor-neutral runner, compatibility policy, or production backend before Codex classifies the questions against repository evidence.
- Do not reopen the migrated shared-semantic-kernel decision without concrete intersecting semantic, ownership, lifecycle, resource, portability, safety, or implementability evidence.
- Do not add a blocking gate unless it protects a distinct falsifiable claim not already covered by an existing work package, state machine, feature-cell check, fixture, trace, workload, or evidence owner.
- Do not block WP-300 or the narrow Property Read path merely because this topic is OPEN.

## Expected decision output

Codex should:

1. classify which concerns are already resolved, intentionally deferred, or genuinely unclosed;
2. define the exact ownership and version boundary of the shared Host/constrained semantic kernel and trace oracle;
3. distinguish semantic resources from profile-specific physical costs, and safety parity from liveness parity;
4. determine whether capability applicability, Host-default expansion, async-no-std claims, API evolution, or production cross-profile evidence require additional authoritative closure;
5. identify any unsupported parity, portability, constrained-runtime maturity, or cross-version claim;
6. identify any architecture, ADR, specification, work package, state machine, capability matrix, resource profile, fixture, trace, workload, checker, audit, plan, risk projection, feature-cell statement, or continuation record that requires correction; and
7. migrate only conclusions supported by repository evidence.

## Decision

Every capability claimed by both Host and constrained profiles must be owned
by one code-level semantic transition kernel and one machine-readable trace
oracle. Shared prose, type names, case identifiers, or independently copied
transition functions do not prove parity. Profile adapters may differ in
storage, wake integration, polling, allocation, and dispatch mechanics.

Parity compares semantic reservations, releases, outcomes, generation and
cleanup truth, plus normalized liveness obligations. It does not require equal
physical bytes, allocation behavior, synchronization primitives, wake counts,
or code size. Those profile-specific costs remain separately declared and
bounded.

Capability applicability must be executable rather than inferred from absent
fields. The authority distinguishes common required capabilities,
profile-required capabilities, optional supported capabilities, and typed
not-applicable cases. Host convenience defaults are expanded to the canonical
semantic configuration before entering the shared kernel; they do not create
a second normative model.

`async-no-std` compilation proves surface portability only. Runtime parity for
the common production capability intersection is established during WP-600,
and the release claim is closed by WP-700 traces and workloads. Mock
Host/static Property Read traces prove narrow constructibility, not broad
liveness, production support, or future-version compatibility.

The narrow WP-300 tranche remains unblocked. Broad WP-300 and WP-400 must
supply the shared oracle and applicability model before claiming generalized
cross-profile parity.

## Migration

The kernel and oracle ownership, semantic-versus-physical resource boundary,
normalized liveness rule, capability taxonomy, Host-default expansion, and
evidence maturity boundary are projected into `PLAN.md`,
`docs/spec/binding-spi.md`, `docs/work-packages/WP-300-bindings.md`,
`docs/work-packages/WP-400-servient.md`,
`docs/work-packages/WP-600-protocol-bindings.md`, and
`docs/work-packages/WP-700-integration.md`. This topic is `MIGRATED`.
