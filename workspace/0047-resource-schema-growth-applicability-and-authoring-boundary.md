# 0047 Resource Schema Growth, Applicability, and Authoring Boundary

Status: MIGRATED

Kind: owner-raised resource-authority, configuration-surface, and evidence-maturity investigation

Priority: HIGH

Target: the ownership, taxonomy, applicability, generated authoring surface, field-admission discipline, default maturity, cross-field validation, Binding/Servient boundary, version evolution, and evidence claims behind the migrated exhaustive resource-schema decision

## Scope and authority

This topic follows the migrated `0037 Resource Configuration Surface Growth` decision. It does not reopen the requirements that externally influenced variable-size state and retained runtime ownership must remain deterministically bounded, that omission must not mean unbounded, that admission must precede publication or externally visible work, or that `docs/resource-limits.csv` remains the current exhaustive authority unless repository evidence supports a different decision.

It records a Project Owner concern that the migrated decision may have closed the abstract relationship between one exhaustive schema and generated ergonomic projections without yet closing the actual public authoring surface, executable applicability model, field-admission discipline, active-versus-deferred status, default-value maturity, cross-field consistency, Binding-versus-Servient ownership, or schema-version contract. The concern is not field count alone. It is that one flat authority may continue absorbing semantic capacity, lifecycle policy, scheduling bounds, internal runtime topology, operational tuning, and provisional defaults faster than its generated views and evidence can keep those concerns understandable and evolvable.

This topic does not prescribe a sparse representation, nested structs, role-specific builder type, overlay format, schema language, resource taxonomy, migration tool, default policy, field-removal rule, or new validation gate. It does not authorize implicit inheritance, permissive omission, unbounded capacity, hidden Binding buffers, or ordinary interaction APIs that expose reservation bookkeeping. It does not block WP-300 or the reviewed narrow Property Read path merely by existing. Codex owns the repository-grounded technical and product decision.

## Repository observations

- The migrated `0037` decision retains `docs/resource-limits.csv` as the flat exhaustive authority and describes named profiles and generated role/profile builders as checked projections that construct one complete `ResourceLimits`.
- The active Foundation specification requires one complete immutable resource snapshot, typed non-applicability, structured limit diagnostics, hierarchical accounting, and no implicit `inherit` or `unbounded` meaning.
- The current schema contains 195 fields and named values for `GatewayDefaultV1`, `DirectoryClientDefaultV1`, and `BenchmarkStaticReferenceV1`.
- The current generator fixes the expected field count at 195 and generates `ResourceKind`, schema metadata arrays, three named-profile arrays, and per-field getters.
- The current generated-authority path does not presently expose the promised role-scoped profile builders or compile-time applicable-field omission checks.
- `ResourceLimits` stores `[Option<u64>; RESOURCE_LIMIT_COUNT]`; public construction and mutation accept `Option<u64>` values, while the current Foundation-level invariant check is narrow rather than a complete role/applicability validation.
- The schema's `capability_roles` column is currently consumed as string metadata. The generated API exposes the text but does not currently represent the expression as an executable typed applicability value.
- Schema rows span document and payload limits, retained owners and queues, cleanup capacity, per-step work, lifecycle timeouts, host runtime lanes and queues, accounting reconciliation intervals, plan/cache state, Binding state footprints, and named product defaults.
- Some resource rows reference requirements whose clauses are inactive or deferred under the current v5.0 authority, while the rows and named values remain present in the exhaustive schema.
- Several exact Host, Binding, route, subscription, emission, cleanup, and scheduling defaults exist before broad WP-300 and WP-400 implementation and production measurement are complete.
- Foundation is active v5.0 authority, while the current generator still describes and validates the fixed field count as a v4.9 resource-limit schema revision.
- Binding registrations separately declare lifetime, transient, ingress, response, route, cleanup, and related footprints, while the application selects the complete Servient resource profile and admission reconciles declared demand with configured ceilings.
- Ordinary application interactions inherit the selected profile and do not expose low-level reservation accounting.

## Questions for investigation

### Authority and resource taxonomy

1. What exact concerns belong in the global `ResourceLimits` authority rather than in lifecycle policy, host defaults, protocol configuration, Binding declarations, workload definitions, or private implementation constants?
2. Does the current schema distinguish semantic resource capacity from runtime topology, scheduling quantum, lifecycle timeout, retry policy, observability retention, and operational tuning?
3. Which existing fields protect externally observable safety or bounded ownership, and which only select one implementation strategy?
4. Can two fields share a resource category and unit while representing materially different authority or compatibility contracts?
5. Which field classes must remain public and application-selectable, and which may be engine-owned while still appearing in diagnostics and evidence?
6. Does one `ResourceKind` enum imply that every row has the same public stability and authoring obligations?
7. Are per-step work maxima resources, scheduler policy, or both, and which authority owns that distinction?
8. Are timeout and retention rows resource ceilings, lifecycle semantics, named defaults, or operational policy inputs?
9. Are host lane, reactor queue, wake lease, and accounting reconciliation rows necessary cross-package admission boundaries or current implementation projections?
10. Which current rows would become invalid if a conforming runtime used a different internal topology while preserving the same semantic capacities?
11. Does the current `resource_kind` column provide enough taxonomy to review this boundary?
12. Which architecture, specification, or API owner decides that a newly identified bound belongs in the exhaustive schema?

### Applicability and typed non-applicability

13. What executable value represents the selected roles and profile cells against which a row is applicable or non-applicable?
14. Is `capability_roles` intended to be normative machine-readable authority or documentation metadata?
15. Does the current role expression mix capability roles, component roles, runtime profiles, and product presets in one axis?
16. How is `producer|consumer`, `directory-client`, `gateway`, `all`, or another expression parsed, validated, and compared?
17. Can the current generator prove that a role-scoped projection hides only rows that are truly non-applicable?
18. At which construction boundary is `None` rejected for a row applicable to the selected profile?
19. Can `ResourceLimits::try_new` or `set` currently construct a snapshot whose required universal fields are `None`?
20. Is such a snapshot invalid immediately, valid but unbound to a role set, or expected to be rejected by a later component?
21. If applicability is validated later, which owner reports the failure and preserves the original profile identity and projection source?
22. Can one complete snapshot be valid for multiple role combinations with different `None` legality?
23. How is typed non-applicability distinguished from a capability excluded by v1 scope, an inactive requirement, an unavailable feature cell, or a deliberately disabled resource?
24. What prevents a generated role view and a dynamic application-defined profile from applying different applicability rules?
25. Which negative tests prove that `None`, zero, omission, disabled, and rendezvous semantics cannot be confused?

### Generated authoring surface and constructibility

26. Which generated authoring surfaces promised by the migrated decision exist today, and which remain downstream work?
27. What exact API allows an application to define a complete Producer-only, Consumer-only, Directory-only, Host Gateway, or constrained profile without directly handling unrelated rows?
28. What exact API allows a simple server-only Binding author to declare its required footprints without understanding Consumer, Directory, cache, emission, or Host-only fields?
29. Does the current `[Option<u64>; 195]` constructor constitute a supported application authoring surface, an internal canonical representation, or both?
30. Is repeated `set(ResourceKind, value)` expected to be a complete public authoring workflow?
31. How does an author know which applicable values remain unset before validation?
32. Can generated views preserve exhaustive source visibility while allowing applications to review only selected roles?
33. How are generated profile views tested against every schema row so a new field cannot disappear silently?
34. What happens to downstream source when a new applicable field is added to a generated projection?
35. Can a named profile be modified through a bounded explicit override without turning every unrelated value into application-owned configuration?
36. How is the origin of each final value retained: named profile, explicit application value, generated projection, or derived admission result?
37. Which generated documentation currently explains every field's owning subsystem, applicability, active status, and rejection behavior?
38. What executable example demonstrates authoring a nontrivial custom profile without raw-array construction or hundreds of unrelated decisions?
39. Which author-edit measurements were collected after the migrated decision, and what do they show?
40. At what checkpoint must the promised authoring projections exist before `ResourceLimits` is treated as a stable external application surface?

### Field admission and growth discipline

41. What evidence is required to add one new global `ResourceKind`?
42. Must every row reference at least one active requirement, implemented owner, concrete reservation point, diagnostic path, and validation fixture?
43. Can a deferred or historical requirement introduce or retain a current public field and named default?
44. How is duplication detected when a new field appears to protect an existing item, byte, owner, or global ceiling?
45. When are per-item, per-route, per-Thing, per-Binding, per-shard, and global ceilings independently necessary rather than mechanically generated scope combinations?
46. Which scopes can be derived safely from other limits and admitted object counts, and which must remain independently configurable?
47. Can one runtime concept expand into count, bytes, temporary bytes, cleanup bytes, queue items, queue bytes, timeout, retry count, and per-step work without a distinct justification for every row?
48. Is there a bounded field-growth budget, review checklist, or schema-complexity metric?
49. Who reviews whether a field exposes an implementation detail that should remain private to a Binding or runtime adapter?
50. Can a field be experimental or provisional without becoming part of stable `ResourceKind::ALL` ordering?
51. Which rows have never been consumed by a runtime reservation, validation, diagnostic, fixture, or workload?
52. What is the process for retiring, replacing, merging, or reclassifying a field?
53. Does stable CSV order require permanent enum slots for obsolete rows?
54. What evidence would justify saying that schema growth is controlled rather than merely exhaustive?

### Active, deferred, and maturity status

55. How does a schema row express whether its owning requirement is active, deferred entry-review input, historical, retired, or not yet implemented?
56. Are rows referencing inactive planning cache, lazy, or index clauses required configuration in the active v5.0 profile?
57. If such rows remain for future compatibility, how are they distinguished from active enforceable limits?
58. Can an inactive row carry a non-`NA` Gateway default without implying current product behavior?
59. Which current named defaults are supported product values, benchmark references, design placeholders, or provisional estimates?
60. What measurement or implementation evidence supports each exact Host runtime, Binding state, route, subscription, emission, cleanup, and scheduling default?
61. Is a default allowed to become `GatewayDefaultV1` before the corresponding runtime behavior exists?
62. How are post-implementation measurements allowed to change a versioned named profile without confusing compatibility or reproducibility claims?
63. Does the schema need to distinguish configured maximum, tested maximum, measured reference, and supported product default?
64. What prevents provisional values from becoming de facto compatibility promises merely because they are generated into public constants?
65. Which release claim is justified for named profiles before broad runtime and production-binding evidence exists?
66. What audit identifies rows whose defaults predate their first executable owner or workload?

### Cross-field consistency and effective capacity

67. Which relationships between per-item, per-owner, per-route, per-Thing, per-Binding, shard, and global fields are normative?
68. Does `ResourceLimits::try_new` validate those relationships, or are intentionally contradictory ceilings allowed?
69. If contradictory ceilings are allowed, how is the effective limiting ceiling explained to an application or Binding author?
70. Can `count * per-item bytes` exceed an aggregate byte ceiling by design, and what runtime behavior follows?
71. Which relationships are strict invariants, advisory diagnostics, or workload assumptions?
72. How are integer overflow and multiplication bounds handled when deriving worst-case aggregate demand?
73. Can independent fields admit a configuration in which no legal instance of an advertised capability can ever be constructed?
74. Does validation detect a profile that enables a capability while assigning zero to one of its indispensable resources?
75. How are static profile table capacities reconciled with byte ceilings, alignment, largest-contiguous-allocation limits, and generated closed-enum sizes?
76. Which diagnostic reports every ceiling that participated in a rejection rather than only the first failing field?
77. Can diagnostics identify the configured value, declared Binding footprint, multiplying owner count, current usage, and effective limiting scope?
78. How is a bounded diagnostic produced when explaining the interaction of many ceilings would itself exceed diagnostic limits?
79. What evidence proves that profile tuning is understandable without reconstructing admission code manually?

### Binding declarations and Servient profile ownership

80. Which resource facts must a Binding declare, and which may be derived from its associated state layouts, compiler artifacts, route counts, or execution capability declarations?
81. Can a Binding declaration contain implementation-specific resource categories absent from the global schema?
82. Must every Binding footprint dimension map one-to-one to a public `ResourceKind`?
83. How does the application know whether a registration rejection should be corrected by changing the Binding implementation, its declaration, the Servient profile, the Thing configuration, or the expected concurrency?
84. Can the same Binding bundle declare different footprints for Host and constrained representations without creating semantic-resource drift?
85. How are lifetime maximum, transient maximum, current reservation, measured peak, and cleanup successor footprint distinguished?
86. Which declaration values are trusted, statically checked, measured, or validated through negative evidence?
87. What happens when an external Binding under-declares hidden protocol-library buffers or reactor state?
88. Does the global schema force the engine to understand protocol-private allocation categories that should remain Binding-local?
89. Can a Binding evolve its footprint without forcing unrelated applications to revise a complete resource profile?
90. Which public authoring example shows the complete declaration-to-admission-to-diagnostic chain for a production Binding?

### Schema identity, ordering, and evolution

91. What is the first-class schema revision identity for the active resource authority?
92. Why does Foundation describe active v5.0 authority while the generator still validates and documents a fixed v4.9 schema revision?
93. Is the schema identity determined by field count, CSV order, a version constant, profile ids, generated code, or an external artifact digest?
94. What compatibility claim does the stable numeric `ResourceKind` ordering provide?
95. Can a new field be inserted, or must it be appended permanently?
96. How are renamed, split, merged, deprecated, or removed fields represented without reinterpreting stored numeric identities?
97. Does `ResourceProfileId::APPLICATION_DEFINED` identify the value set, or only its origin class?
98. How are two application-defined snapshots with different schema revisions or values distinguished in diagnostics, cache identities, configuration management, and audit logs?
99. Can an older application-defined profile be loaded after the engine adds a new required field?
100. Is source incompatibility intentional for every schema addition, and which downstream authors are expected to change?
101. How are generated role projections versioned relative to the canonical schema and named profiles?
102. Which compatibility categories apply: Rust source, serialized configuration, generated source, static layout, diagnostic identity, semantic admission, or named-profile behavior?
103. What migration evidence is required before the schema is exposed through external configuration files or platform management APIs?

### Evidence, usability, and maturity claims

104. Which parts of the migrated `0037` decision are proven by current generated code and tests?
105. Which parts remain documentation promises: role builders, omission checks, grouped documentation, applicability validation, and author-edit evidence?
106. What narrow Property Read evidence exercises custom profile construction beyond named defaults?
107. What broad WP-300 evidence measures the number and type of resource decisions required from a production Binding author?
108. What WP-400 evidence measures profile retained bytes, construction cost, clone cost, compile time, generated-code size, binary size, and runtime lookup cost?
109. What evidence compares raw-array authoring, named-profile use, and role-scoped custom authoring without prescribing one implementation?
110. Which negative fixtures prove a new schema row cannot enter without active ownership, applicability, diagnostics, defaults, and evidence classification?
111. What workload proves that hierarchical accounting provides value not already supplied by fewer aggregate limits?
112. What scenario proves that an application can diagnose and safely tune a real multi-Thing, multi-Binding saturation failure?
113. At what checkpoint must application and Binding authoring usability be demonstrated before broad public SPI stability is claimed?
114. Do current architecture, Foundation specification, work packages, milestones, defaults, or continuation statements imply that generated projections and applicability enforcement are more complete than source evidence supports?
115. Which findings would be normal implementation work, and which would reopen the resource authority, public API, named-profile, Binding SPI, or compatibility contract?

### Existing decision intersection

116. Which questions in this topic are already fully resolved by `0037`, ADR-0015, the Foundation specification, the resource schema, Foundation source, WP-300, WP-400, or registered evidence?
117. Which questions are intentionally deferred implementation choices that should remain open until source and authoring evidence exists?
118. Which questions are genuinely unclosed architecture, public-API, applicability, default, compatibility, or evidence contracts?
119. Does any unresolved concern intersect the reviewed narrow WP-300 Property Read implementation, or can it remain downstream without weakening that tranche's exact claim?
120. Is retaining one flat exhaustive authority consciously separated from exposing one flat application authoring surface?
121. Is schema width currently presented as controlled because safe generated projections exist, or because they are planned to exist?
122. Would any additional requirement, artifact, fixture, workload, example, or maturity boundary protect a distinct falsifiable claim not already owned by existing evidence?

## Constraints

- Preserve deterministic finite bounds for externally influenced work, retained ownership, queues, buffers, memory, cleanup, and applicable progress.
- Preserve explicit admission before publication or externally visible side effects.
- Do not treat omission, `None`, inheritance, zero, disabled, rendezvous, or unbounded as interchangeable meanings.
- Preserve one authoritative resource schema unless repository evidence supports an explicit replacement or decomposition decision.
- Keep ordinary interaction calls free of reservation, ledger, and low-level accounting parameters.
- Do not allow generated or role-scoped projections to become independent schemas with different semantics.
- Do not infer a defect solely from field count, long generated code, or the absence of broad downstream implementation.
- Do not prescribe a schema representation, taxonomy, builder design, overlay model, migration mechanism, field-removal policy, or default-value process before Codex classifies the questions against repository evidence.
- Do not reopen the migrated bounded-resource and exhaustive-authority principles without concrete intersecting safety, ownership, lifecycle, portability, compatibility, authoring, or implementability evidence.
- Do not add a blocking gate unless it protects a distinct falsifiable claim not already covered by an existing requirement, work package, fixture, workload, checker, or evidence owner.
- Do not block WP-300 or the narrow Property Read path merely because this topic is OPEN.

## Expected decision output

Codex should:

1. classify which concerns are already resolved, intentionally deferred, normal implementation work, or genuinely unclosed;
2. define the exact authority boundary between semantic resource capacity, lifecycle and scheduling policy, runtime topology, operational tuning, product defaults, protocol-private declarations, and workload-only parameters;
3. determine whether applicability, `None` legality, active/deferred status, and schema revision require stronger executable ownership;
4. determine whether current generated authoring projections and diagnostics are sufficient for application and external Binding authors, and state the justified maturity claim before they exist;
5. define the evidence and compatibility boundary for admitting, defaulting, evolving, deprecating, or retiring resource fields;
6. identify any unsupported resource-authoring, named-profile, applicability, compatibility, boundedness, or usability claim;
7. identify any architecture, ADR, Foundation specification, resource-schema row, named profile, generated API, Binding declaration, work package, fixture, workload, checker, audit, example, plan, risk projection, default, or continuation record that requires correction; and
8. migrate only conclusions supported by repository evidence.

## Decision

The flat exhaustive resource schema remains the canonical finite authority. It
is not, by itself, a stable public authoring surface. Raw construction of the
generated `[Option<_>; N]` representation is a low-level mechanism and cannot
serve as evidence that application authors or independent binding authors can
configure the schema safely.

Before broad resource or Binding-SPI maturity is claimed, each field needs an
authority class, executable applicability axes, lifecycle status and default
maturity. Generated role projections or builders must validate cross-field
rules, distinguish omitted, disabled, zero, rendezvous, inherited, and typed
not-applicable values, and retain diagnostics that identify the value origin.
`None` becomes not-applicable only after a typed role/profile binding proves
that meaning; it is not a universal sentinel.

The schema also needs an explicit revision, canonical field order, and digest.
Evolution is append-only within a revision and uses an explicit migration when
semantics or order change. Application-defined named profiles bind their
revision and value digest. Field admission, defaulting, deprecation, and
retirement require a checklist covering semantic authority, bounds,
applicability, lifecycle, compatibility, authoring projection, diagnostics,
and executable evidence.

Protocol-private physical costs do not automatically become global semantic
resource fields. They may remain Binding declarations or aggregate footprint
evidence when they do not affect cross-binding admission or Servient policy.

The current narrow WP-300 package supplies an exact fixed profile and remains
admissible. Broad WP-300/WP-400 authoring maturity is blocked until the above
closure exists; no Foundation source change is admitted by this topic alone.

## Migration

The canonical-authority versus authoring-surface boundary, applicability and
revision requirements, evolution discipline, and field lifecycle checklist
are projected into `PLAN.md`, `docs/spec/foundation.md`,
`docs/work-packages/WP-300-bindings.md`, and
`docs/work-packages/WP-400-servient.md`. This topic is `MIGRATED`.
