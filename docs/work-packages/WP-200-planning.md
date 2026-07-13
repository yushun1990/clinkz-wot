# WP-200 Logical and Binding Planning

Status: Planned

Design revision: v4.6

Depends on: WP-100

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core, clinkz-wot-protocol-bindings, clinkz-wot-td

## Scope

Implement the two-level logical/binding plan model and generation-bearing capability indexes
across `clinkz-wot-core` and `clinkz-wot-protocol-bindings`. Move TD scanning, effective form
resolution, capability pruning, bounded candidate ordering, URI-template compilation, and
lazy compiled-artifact coordination out of interaction hot paths.

Core owns immutable protocol-neutral plan values. `clinkz-wot-protocol-bindings` owns shared
compilers, capability indexes, form/operation/security resolution algorithms, and URI-template
helpers. This package does not own binding execution traits, Servient registrations, route
lifecycle, concrete protocols, or application handles.

Response classification facts follow
`docs/amendments/WP-100-interaction-output-api-v1.md`: this package compiles the
primary/additional branch and schema/media facts consumed by WP-300, but does
not publish an interaction response.

## Requirements

- `PLAN-COST-001`, `PLAN-COST-002`, and `PLAN-COST-003` define logical sharing, bounded
  binding-specific compilation, and admission failures.
- `API-PAYLOAD-001` governs the immutable response-classification facts consumed by WP-300.
- `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`, `PLAN-REQUEST-001`, and
  `PLAN-BOUND-001` define pruning, lazy/single-flight state, per-call ownership, and candidate
  limits.
- `TD-MEM-001` and `TD-MEM-002` prevent duplicated resident TD trees and charge explicit
  effective-view materialization.
- `DOC-RUNTIME-001`, `DOC-RUNTIME-002`, and `DOC-RUNTIME-003` define source retention, runtime
  views, and explicit effective-document materialization.
- `JSONLD-PREFIX-001` governs bounded deterministic prefix resolution during effective plan
  construction.
- `FORM-FINALIZE-002`, `FORM-OWNER-001`, and `FORM-COVERAGE-001` apply to the compiler inputs,
  deterministic owner result, and frozen plan output; lifecycle execution remains WP-300 and
  WP-400 work.
- `SEC-PERF-001`, `VALIDATE-COMPILE-001`, and `VALIDATE-REUSE-001` apply to immutable compiled
  security and validator references.
- `RES-LIMIT-001` through `RES-LIMIT-003`, `ADMIT-TXN-001`, and `ADMIT-MEM-001` govern plan
  probes, candidates, bytes, lazy slots, temporary memory, and rollback.
- `PERF-COMPLEXITY-001` and `PERF-INDEX-001` define byte-aware work counters, recursion bounds,
  and adversarial lookup behavior.
- `PERF-ADMISSION-001` and `PERF-PEAK-001` govern phase release and peak live bytes during
  plan admission.

## Crates and Feature Cells

- Modify Cargo packages `clinkz-wot-core` and `clinkz-wot-protocol-bindings`.
- In `clinkz-wot-core`, the `no-default`, `async-no-std`, and `std` cells expose identical
  protocol-neutral plan values; representation may differ only behind private storage.
- In `clinkz-wot-protocol-bindings`, all three cells expose compilers, capability indexes,
  form/security/operation resolution, and URI-template helpers. The `async-no-std` cell may add
  adapters but no executor; `std` adds conveniences rather than a different plan contract.
- Preserve `foundation + td + core <- protocol-bindings/core`; core must not depend on the
  compiler package, and neither package may depend on Servient or a concrete protocol.
- Add no-std compile fixtures that build a logical plan, index a static binding capability,
  resolve `base` plus relative `href`, and retain a generation-bearing binding-plan reference.

## Public API and Data Migration

Implement the frozen core-owned values:

- `clinkz_wot_core::LogicalInteractionPlan`, `BindingPlanRef`, `BindingCandidate`,
  `BindingSupport`, `InboundBindingPlan`, `BindingThingView`, `InboundRouteMatch`, and
  `BindingCapability`;
- use WP-100 `PlanId`, `BindingId`, `BindingGeneration`, slot ids, `EffectiveSecurityPlan`,
  and compact metadata references rather than cloning static request data.

Implement the frozen compiler-owned surface:

- `clinkz_wot_protocol_bindings::CapabilityIndex`, `PlanCompiler`, `PlanBuildInput`,
  `PlanBuildOutput`, `CompiledUriTemplate`, and `ResolvedFormTarget`.

Keep `ResolvedFormTarget` at its current owner and public path. Replace the remaining current
selection views with the target compiler inputs/outputs or make narrowly useful helpers
private. `PlanBuildInput` captures an immutable registration/capability generation snapshot,
limits, source identity, and TD view. `PlanBuildOutput` returns shared logical plans, compact
binding references, exact admission charges, and structured failures without embedding
execution trait objects.

Resolve effective operation, root-versus-affordance form context, original form index, `base`
plus relative `href`, media defaults, response metadata, URI variables, security inheritance,
scope, extensions, and stable plan identity exactly once per logical form. Preserve TD order in
candidate vectors and retain enough source identity for strict selection and diagnostics.

## State and Ownership Migration

- Build separate client and server `CapabilityIndex` snapshots keyed by resolved scheme and
  declared secondary capabilities. Store the snapshot generation with every compact binding
  reference.
- Share one `LogicalInteractionPlan` across form-binding pairs. A `BindingPlanRef` owns only
  binding identity/generation, static support outcome, and non-shareable compiled state.
- Admit every probe, wildcard, candidate, schema/security node, URI byte, compiled byte, lazy
  slot, and temporary byte through the WP-000 limits and ledger before publishing a plan.
- Coordinate concurrent lazy compilation as single-flight per plan key and dependency
  generation. Run compiler/provider callbacks outside registry and eviction locks; publish one
  immutable result or bounded backpressure.
- Retain deterministic negative results only for deterministic current-generation failures.
  Invalidate provider, binding, credential, schema, or policy changes by O(1) generation
  publication and reclaim stale entries incrementally within cleanup budgets.
- Keep source documents authoritative when retained. Effective views use immutable sharing,
  overlays, indexes, or side tables; owned effective-document materialization is explicit and
  charged.

## Old API Removal

- Remove `core::thing::ConsumedThing::bindings: Vec<Arc<dyn ClientBinding>>` and its
  `register_binding` planning path. Consumed plans retain binding ids/generations or static
  slots, while WP-300 registrations own execution objects.
- Remove public planning dependence on `AffordanceRef`, `FormSelectionCriteria`,
  `SelectedForm`, `SelectedAffordanceForm`, `SelectedAffordanceSelection`, and
  `EffectiveFormSecurity` after their target equivalents are available. A private compiler
  helper may keep an internal role but not the obsolete cross-crate contract.
- Remove per-call TD-tree scanning, repeated `base`/default/security resolution, and plan-time
  cloning of target strings, schemas, response metadata, security expressions, or extension
  maps into `BindingRequest`.
- Remove any full logical-plan copy stored per binding candidate and any invalidation path that
  synchronously scans all Things or plans.
- Do not move `ClientBinding`, `ServerBinding`, or their registrations into
  `clinkz-wot-protocol-bindings`; that ownership would violate the frozen dependency graph.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `logical-plan-footprint` for two-level sharing, compact binding references, and immutable
  primary/additional response-classification facts;
- `capability-index-pruning` for keyed probes and admitted wildcard work;
- `bounded-candidate-selection` for strict/fallback selection and 1/8/32 limits;
- `lazy-plan-single-flight` for concurrency, deterministic negative entries, and backpressure;
- `plan-generation-invalidation` for O(1) publication and incremental reclamation;
- `admission-transaction-rollback` for exact charges, phase release, and peak memory.

These records satisfy the corresponding requirement-index evidence families:

- `plan-cost-and-limits` for structural sharing, exact charges, rollback, and one-over limits;
- `index-lazy-request-size` for capability pruning, wildcard bounds, lazy policy, and compact
  per-call records;
- `lazy-cache-single-flight-generation` for races, negative classification, O(1)
  invalidation, bounded reclamation, and no global callback lock;
- `per-operation-candidate-bound` for 1/8/32 candidates, strict selection, and shared provider
  probe budgets;
- `td-memory-ownership` for retained source, overlays, explicit materialization, and live-byte
  measurements;
- `form-finalization` for deterministic compiled form identity, ownership outcomes, and frozen
  inbound plans consumed by later packages;
- `complexity-scaling` for bytes, nodes, strings, URI output, recursion depth, and hostile hash
  inputs;
- `cargo-dependency-direction`, `feature-public-surface`, and `frozen-cross-crate-surface` for
  package ownership and all required cells.

Evidence must distinguish admission failure from first-use compilation failure and record the
plan, registration, provider, schema, credential, and policy generations used by the case.

## Performance Workloads

- `PERF-GW-003` and `PERF-CS-003` cover compiled plan lookup.
- `PERF-GW-004` and `PERF-CS-004` cover the 32-candidate admitted bound.
- `PERF-GW-011` and `PERF-CS-010` cover concurrent single-flight lazy compilation.
- `PERF-GW-012` covers generation invalidation across 4,096 Things without an eager scan.
- `PERF-GW-014`, `PERF-GW-015`, `PERF-CS-013`, and `PERF-CS-014` cover profile-maximum
  planning and one-axis byte/structure scaling.
- `PERF-DIR-001` and `PERF-DIR-006` consume the same protocol-neutral request/publication
  planning primitives; Directory client orchestration remains WP-500.
- `PERF-DIR-009` covers Directory-facing admission byte and structure scaling on the shared
  planning substrate.

## Completion Conditions

- Every WP-200 ownership item exists at its frozen package and public path in all applicable
  feature cells; compiler crates contain no binding execution trait or Servient registration.
- Plan fixtures cover root and affordance forms, multiple forms, relative targets, strict form
  selection, ordered fallback, inherited/form security, and structured selection errors.
- Structural tests prove logical plans are shared rather than copied per binding pair and
  per-call requests contain only varying data plus compact plan references.
- Capability probes are pruned by generation-bearing indexes, wildcard work is admitted, and
  the 1/8/32 scaling and maximum-profile workloads emit schema-valid results.
- Lazy compilation is single-flight, generation-aware, nonblocking across unrelated keys, and
  reclaimed incrementally; callbacks run outside global/cache-eviction locks.
- All obsolete public selector views, per-call TD scans, execution-trait ownership leaks, and
  eager global invalidation scans listed above are removed.
