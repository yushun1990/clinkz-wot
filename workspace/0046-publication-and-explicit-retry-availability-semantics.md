# 0046 Publication and Explicit Retry Availability Semantics

Status: OPEN

Kind: owner-raised product-availability and runtime-semantics investigation

Priority: HIGH

Target: the product-availability consequences, recovery responsibilities, application facade, diagnostics, policy boundary, and evidence maturity behind the migrated all-required-route publication and explicit Consumer retry decisions

## Scope and authority

This topic follows the migrated `0034 Multi-Route Serving Availability` and `0039 Candidate Fallback Availability Semantics` decisions. It does not reopen the requirements that a published Producer generation must truthfully match its immutable TD, plan set, route set, and serving authority; that no uncommitted route may accept traffic; or that the engine must not silently retry another Consumer candidate after acceptance, committed security effects, or uncertain protocol side effects.

It records a Project Owner concern that those safety and truthfulness invariants may remain correct while the selected v1 product semantics place excessive availability-recovery, retry, failover, deadline, idempotency, generation, TD-reconstruction, and diagnostic responsibility on applications. It also asks whether atomic publication has been coupled more tightly than necessary to all-advertised-route success, and whether prohibiting implicit retry has been coupled more tightly than necessary to the absence of an engine-defined explicit retry action.

This topic does not prescribe availability groups, optional routes, TD variants, automatic degradation, a retry coordinator, health-based selection, circuit breakers, backoff, idempotency policy, facade methods, or a new validation gate. It does not authorize silent partial publication, post-acceptance candidate switching, mutable unversioned health as a second planner, carried-over security side effects, or hidden retry. It does not block WP-300 or the reviewed narrow Property Read path merely by existing. Codex owns the repository-grounded technical and product decision.

## Repository observations

- In v1 every route represented by an advertised Producer form in the frozen effective TD and immutable plan set is required for that serving generation.
- A failed route prevents publication, triggers rollback, and cannot join the same generation later. Omitting the form requires a different effective TD, plan set, and produced generation.
- Publication remains one atomic Servient transition that makes the immutable Producer plan set, produced registry generation, and one serving activation authority selectable only after all required routes are committed closed.
- The migrated decision treats `optional`, `redundant`, `alternative`, `degraded`, and `late joining` as absent v1 route labels rather than inferred runtime meanings of multiple forms.
- Consumer forms are ordered candidates rather than an automatic runtime failover pool.
- `PreExecution` fallback may skip only side-effect-free security inapplicability and deterministic cacheable lazy-artifact failure. Temporary transport unavailability, backpressure, timeout, resource failure, stale or draining generation, binding rejection, and every post-acceptance result terminate the interaction.
- Explicit retry is a new application action. The application may inspect phase, `RetryClass`, identities, candidate diagnostics, idempotency, and security policy, then issue a new call using a strict form or binding choice.
- The new call does not inherit the prior call's security commit, deadline, work budget, side effects, or cancellation state. `GatewayDefaultV1` performs no automatic retry.
- Mutable runtime health is diagnostic-only and cannot reorder, remove, or skip immutable candidates. A future health-aware policy requires separately admitted immutable versioned state and evidence.
- Current status and structured errors explain failed route, phase, generation, retry classification, and why another candidate was not tried, but do not yet define every product recovery action or an application-wide retry workflow.
- The narrow Property Read path cannot establish multi-route Producer availability, repeated Consumer failure recovery, non-idempotent operations, overall retry deadlines, generation churn, or production protocol failover behavior.

## Questions for investigation

### Atomic publication versus all-route conjunction

1. Which invariant requires atomic publication, and which separate policy requires every advertised Producer form to be a required route?
2. Is all-advertised-route success a logical consequence of publication truthfulness, or a deliberately narrower v1 product choice?
3. Could a publication remain atomic and truthful while selecting one pre-admitted complete route or TD configuration before the publication point, or would that violate an existing immutable authority?
4. Which exact TD identity is authoritative when an application intentionally exposes fewer forms than its original draft or source document contained?
5. Does the current model distinguish source TD, effective TD, attempted generation, and successfully published TD clearly enough for application and Directory integration?
6. What evidence supports the conclusion that every advertised HTTP, Zenoh, MQTT, or future route should have equal required status for one generation?
7. Can adding a new form reduce whole-Thing availability even when the pre-existing routes remain healthy, and is that product effect explicitly accepted?
8. Does all-route-required encourage applications to omit legitimate alternative access paths, split one Thing into artificial Things, or manage additional routes outside the WoT model?
9. Which failures indicate a false advertised capability that must block publication, and which indicate temporary inability to realize one otherwise valid route?
10. Are route requirement semantics owned by TD content alone, by application configuration, by planning policy, or by a future separately versioned authority?
11. Which current architecture or specification text would need correction if atomic publication and all-route-required were determined to be distinct decisions?
12. What exact v1 availability claim is justified when optionality and alternative-route semantics are excluded?

### Producer failure recovery and regeneration

13. After one required route fails readiness, which component decides whether to retry the same generation configuration, remove a form, repair configuration, or stop?
14. What structured information tells that component whether the route failure is transient, permanent, configuration-related, resource-related, or generation-related?
15. Is rebuilding a new effective TD without the failed form an engine operation, an application operation, a platform-management operation, or explicitly outside the runtime contract?
16. How is the relationship between the rejected generation and a reduced replacement generation retained for diagnosis and audit?
17. Must the replacement TD be revalidated, resigned, republished to a Directory, or reconciled with an externally supplied source document?
18. When may a replacement expose begin relative to rollback, pending cleanup, durable residual state, endpoint release, and generation reclamation of the failed attempt?
19. Can repeated transient failures produce unbounded generation churn, planning work, cleanup work, endpoint reservations, or Directory updates?
20. Which backoff, attempt, deadline, or operator-intervention responsibility is currently owned, and which is left entirely to the application?
21. How does an application restore the original full route set after a reduced generation has been serving and the failed protocol recovers?
22. Does restoration require replacing a healthy serving generation, and what availability interruption or overlap semantics apply?
23. What happens when the failed route is security-critical, local-only, management-only, or merely an additional client access path?
24. Which status states distinguish failed exposure, retryable exposure, waiting for application policy, cleanup pending, and permanently rejected configuration?
25. What product-level example demonstrates that the current regeneration path is implementable without applications duplicating planning, TD mutation, generation, and lifecycle logic?

### Consumer retry action and failure certainty

26. What exact semantic claim does `RetryClass` make, and what retry decision remains application-owned?
27. Does the error model distinguish safe retry of the same candidate, safe use of another candidate, retry only with idempotency protection, unknown execution result, required handle rebuild, and non-retryable configuration failure?
28. Which failure phases prove that no protocol side effect occurred, and which merely prove that the binding did not return acceptance through the SPI?
29. Can `BindingInputRejection<OutboundRequest>` always justify trying another candidate, or can binding-local state, generation, resource, or protocol-library behavior make the correct action narrower?
30. How are transport connection failure before request write, partial write, accepted write with lost response, and remote execution with lost acknowledgement classified?
31. How do retry rules differ for Property Read, Write Property, Invoke Action, subscription start, subscription stop, and publication operations?
32. Which operations require caller-provided or plan-provided idempotency metadata before any alternate-candidate retry may be considered?
33. Does using another protocol form preserve the same remote operation identity, authorization scope, serialization semantics, and idempotency domain?
34. What prevents an application from interpreting a generic retryable error as safe alternate-form failover when only same-candidate retry is valid?
35. What structured terminal outcome reports that the operation result is unknown and must not be automatically retried?
36. Is the current public error and `RetryClass` surface sufficient to implement these distinctions without protocol-specific application knowledge?
37. What negative evidence proves that explicit retry cannot accidentally become hidden fallback after acceptance or uncertain side effects?

### Retry context, overall limits, and correlation

38. How does an application impose one overall deadline across multiple new retry calls when each call owns a fresh deadline?
39. How are maximum attempts, total work, total provider probes, total resource reservations, and total elapsed time bounded across the retry sequence?
40. Does each explicit call receive a new correlation identity, and what higher-level identity links the attempts into one product operation?
41. How are diagnostics, traces, metrics, and final errors correlated across attempts without implying that the engine carried side effects or ownership between them?
42. Can repeated retries reset per-interaction bounds in a way that defeats the intended global resource or probe policy?
43. Which layer owns backoff, jitter, attempt ordering, retry cancellation, and final-result selection?
44. How is caller cancellation of the overall product operation propagated to an active attempt and prevented from starting later attempts?
45. Can an explicit retry reuse an idempotency key while still creating a new call, deadline, security application, and binding operation identity?
46. How is a stale or draining plan-set generation handled between attempts: strict failure, selection of another retained candidate, consumed-handle rebuild, or new Servient generation?
47. Does a strict `form_index` remain valid only in the original plan-set generation, and what diagnostic prevents its accidental reuse against another generation or form array?
48. What final structured outcome preserves every attempted candidate and failure without unbounded history?

### Runtime health and repeated predictable failure

49. When the first immutable candidate is temporarily unavailable, does every ordinary call repeatedly select and fail on it until the application supplies a strict alternative?
50. Which standard mechanism allows an application to avoid repeating a known failure without implementing an unversioned mutable health planner outside the engine?
51. Is a caller-maintained temporary preferred-form override semantically different from the mutable runtime health policy rejected inside planning?
52. What immutable generation, expiry, scope, bounds, and diagnostic identity would be required for any application-provided availability input?
53. Can health information narrow candidates for a new call without mutating the existing compiled plan set or reordering its authority?
54. How are stale health observations prevented from suppressing a recovered preferred candidate or selecting an unavailable alternative?
55. Which availability observations are protocol-local diagnostics, and which are safe inputs to application policy?
56. Does deferring the versioned health-policy design leave v1 applications with no reusable way to implement repeated-call failover?
57. What product behavior does `GatewayDefaultV1` provide during a sustained outage of the first candidate while a later candidate is healthy?
58. Is no automatic retry an intentionally limited safe default, or also the complete v1 Gateway availability offering?

### Facade and user-expectation boundary

59. Is a strict `form_index` or binding choice an adequate application-facing retry abstraction, or only a low-level selection primitive?
60. How does an application discover the next admissible candidate without rescanning TD forms, depending on document order, or duplicating planning logic?
61. Which APIs expose immutable candidate metadata while preventing callers from forging plan, form, binding, security, or generation identities?
62. Can a Scripting-compatible caller express safe same-candidate retry, alternate-form retry, overall deadline, idempotency, and generation refresh without engine-specific internals?
63. How are Producer multiple forms and Consumer multiple forms explained so users do not infer the same failover meaning on both sides?
64. Which documentation and diagnostics state that multiple Consumer forms are not a runtime failover pool and multiple Producer forms are all required in v1?
65. What application examples show temporary HTTP failure with healthy Zenoh for Property Read, Write Property, and Invoke Action?
66. What application example shows HTTP and Zenoh ready while one advertised MQTT Producer route is temporarily unavailable?
67. How much protocol, TD, generation, and retry knowledge must ordinary Gateway application code contain under the current design?
68. Which repeated application boilerplate would demonstrate that the product facade is incomplete even though the underlying contracts are implementable?
69. Could different applications implement incompatible interpretations of the same `RetryClass` and candidate diagnostics?
70. What stable product-level action vocabulary is needed to keep retry decisions consistent without authorizing hidden retry?

### Evidence timing and availability claims

71. Which publication and retry claims are proved by the narrow mock Property Read slices?
72. Which claims remain unproved until multi-route exposure, broad Consumer calls, non-idempotent actions, subscriptions, production bindings, and Gateway facades execute?
73. What deterministic scenario first proves recovery from one temporarily unavailable required Producer route?
74. What scenario proves that a reduced replacement generation is truthful, bounded, observable, and recoverable back to the full configuration?
75. What scenario proves safe alternate-candidate retry before protocol side effects?
76. What scenario proves that uncertain post-acceptance failure cannot select another candidate even under an application retry policy?
77. What scenario proves one overall deadline and bounded attempt history across multiple explicit calls?
78. What production Zenoh, HTTP, or other protocol behavior is required before retry certainty classifications are trusted?
79. Which availability workload measures generation churn, rollback duration, repeated first-candidate failure, alternative-candidate success, and unrelated-owner progress?
80. At what checkpoint must the Gateway product demonstrate a usable recovery path rather than only structured failure reporting?
81. Do current architecture, specification, work-package, milestone, risk, or continuation statements imply stronger degraded-service, failover, retry, or recovery capability than repository evidence supports?
82. Which findings would be normal facade or implementation feedback, and which would reopen ADR-0012, ADR-0017, Planning, WP-400 publication, or the public Consumer call contract?

### Existing decision intersection

83. Which questions in this topic are already fully resolved by the migrated `0034` and `0039` decisions, ADR-0012, ADR-0017, Planning, the Binding SPI, WP-400, or registered evidence?
84. Which questions are intentionally deferred product choices that should remain open until broad implementation provides concrete evidence?
85. Which questions are unresolved architecture, public-API, retry-classification, generation, TD-authority, or evidence contracts that must close before broad WP-400 or Gateway availability claims?
86. Does any unresolved concern intersect the reviewed narrow WP-300 Property Read implementation, or can it remain downstream without weakening that tranche's exact claim?
87. Is all-route-required a consciously accepted v1 limitation with an explicit non-goal and user-facing consequence, or currently presented as if atomic publication logically required it?
88. Is application-created strict retry a consciously accepted v1 limitation with an explicit non-goal and user-facing consequence, or currently presented as complete failover support?
89. Would any additional requirement, artifact, fixture, workload, example, or maturity boundary protect a distinct falsifiable claim not already owned by existing evidence?

## Constraints

- Preserve atomic and truthful publication of one immutable Producer generation.
- Do not permit an uncommitted, stale, omitted, or unadvertised route to accept traffic under a published generation.
- Do not silently publish a partial route set while retaining a TD or plan set that advertises unavailable routes.
- Do not automatically switch Consumer candidates after binding acceptance, committed security effects, uncertain protocol side effects, or an operation result whose execution certainty is unknown.
- Do not let mutable unversioned runtime health reorder or replace immutable planning authority.
- Preserve independent call ownership, cancellation, deadline, work, security, correlation, and cleanup semantics unless an explicitly admitted higher-level contract says otherwise.
- Do not infer that strict publication or explicit retry is defective solely because broad WP-400, production bindings, or Gateway facades are not yet implemented.
- Do not treat availability preference, convenience, field count, or subjective ergonomics alone as proof of a semantic defect.
- Do not prescribe availability groups, TD variants, route labels, retry orchestration, health snapshots, backoff, idempotency policy, facade shape, or protocol behavior before Codex classifies the questions against repository evidence.
- Do not reopen the migrated publication-truthfulness or no-hidden-retry invariants without concrete intersecting product, semantic, ownership, security, lifecycle, generation, resource, portability, or implementability evidence.
- Do not add a blocking gate unless it protects a distinct falsifiable claim not already covered by an existing work package, state machine, fixture, workload, or evidence owner.
- Do not block WP-300 or the narrow Property Read path merely because this topic is OPEN.

## Expected decision output

Codex should:

1. classify which concerns are already resolved, intentionally deferred, or genuinely unclosed;
2. distinguish atomic publication truthfulness from the v1 all-advertised-route requirement and state whether their current coupling is intentional;
3. define the application-visible recovery responsibility after Producer route failure, including TD, generation, rollback, cleanup, status, and Directory boundaries where applicable;
4. determine whether `RetryClass`, phase, candidate diagnostics, strict selection, deadlines, idempotency, and generation handling provide a sufficient explicit Consumer retry contract;
5. determine whether repeated-call health, overall retry bounds, attempt correlation, Gateway defaults, or product facades require additional authoritative closure;
6. identify any unsupported degraded-service, failover, retry, recovery, or availability claim;
7. identify any architecture, ADR, specification, work package, state machine, error taxonomy, retry classification, TD or generation authority, fixture, workload, checker, audit, example, plan, risk projection, default, or continuation record that requires correction; and
8. migrate only conclusions supported by repository evidence.
