# WP-200 Logical and Binding Planning

Status: Planned

Design revision: v4.9

Depends on: WP-100

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core, clinkz-wot-planning, clinkz-wot-td

## Scope

Implement the two-level logical/binding plan model and generation-bearing capability indexes
across `clinkz-wot-core` and `clinkz-wot-planning`. Migrate the current
`protocol-bindings/core` package to the target planning crate, then move TD scanning, effective
form resolution, capability pruning, bounded candidate ordering, URI-template compilation, and
binding-compiler coordination out of interaction hot paths.

Core owns immutable protocol-neutral plan values and the portable binding-compiler extension and
artifact SPI. `clinkz-wot-planning` owns the shared plan compiler, capability indexes,
form/operation/security resolution algorithms, URI-template helpers, and resumable build cursor.
The output of this package is complete immutable material for one unpublished Frozen plan-set
draft. WP-400 owns the Servient record and every Building, Frozen, Published, Draining, Failed,
and Reclaimed lifecycle transition. This package does not own binding execution traits, Servient
registrations, route lifecycle, concrete protocols, or application handles.

Response classification facts follow
`docs/amendments/WP-100-interaction-output-api-v1.md`: this package compiles the
primary/additional branch and schema/media facts consumed by WP-300, but does
not publish an interaction response.

Collection subscription planning is first-class. `subscribe_all_events` and
`observe_all_properties` select one compatible Thing-level form from the root `forms` array;
planning never lowers either operation into per-affordance requests or a local merged stream.

WP-200 consumes the WP-100 handler context and operation identities only as immutable plan
facts. It neither activates host handler registrations nor removes a compatibility facade needed
by WP-300, WP-400, or WP-600. New planning code must not call `PushFn`, `PublisherSink`,
`SubscriptionSender`, a legacy raw handler lookup, or an old handler trait.

## Requirements

The package index assigns this exact requirement set:

- `DOC-RUNTIME-001`, `DOC-RUNTIME-002`, and `DOC-RUNTIME-003`;
- `JSONLD-PREFIX-001`;
- `PLAN-COST-001`, `PLAN-COST-002`, and `PLAN-COST-003`;
- `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-REQUEST-001`, `PLAN-CACHE-001`, and
  `PLAN-BOUND-001`;
- `PLAN-SET-001` and `PLAN-ARTIFACT-001`;
- `BIND-PROGRESS-001` and `API-PAYLOAD-001`;
- `TD-MEM-001` and `TD-MEM-002`;
- `ADMIT-TXN-001` and `ADMIT-MEM-001`;
- `PERF-COMPLEXITY-001`, `PERF-INDEX-001`, `PERF-ADMISSION-001`, and `PERF-PEAK-001`.

Together these requirements govern structural sharing, indexed and bounded selection, the
compiler-extension/artifact contract, immutable plan-set draft material, source retention,
collection-plan attribution, admission rollback, and byte-aware complexity. The form-finalization,
security, validation, resource-profile, and lifecycle contracts defined by the v4.9 specifications
remain mandatory inputs, but their implementation evidence is assigned to the work packages that
own those surfaces.

## Crates and Feature Cells

- Modify Cargo package `clinkz-wot-core` and migrate `protocol-bindings/core` to
  the target `clinkz-wot-planning` package without preserving a second public compiler owner.
- In `clinkz-wot-core`, the `no-default`, `async-no-std`, and `std` cells expose identical
  protocol-neutral plan values; representation may differ only behind private storage.
- In `clinkz-wot-planning`, all three cells expose compilers, capability indexes,
  form/security/operation resolution, and URI-template helpers. The `async-no-std` cell may add
  adapters but no executor; `std` adds conveniences rather than a different plan contract.
- Preserve `foundation + td + core <- clinkz-wot-planning`; core must not depend on the
  compiler package, and neither package may depend on Servient or a concrete protocol.
- Add no-std compile fixtures that build a logical plan, index a static binding capability,
  resolve `base` plus relative `href`, incrementally compile a bounded fake artifact, and retain a
  generation-bearing binding-plan reference.

## Public API and Data Migration

Implement the frozen core-owned values:

- `clinkz_wot_core::LogicalInteractionPlan`, `BindingPlanRef`, `BindingCandidate`,
  `BindingSupport`, `InboundBindingPlan`, `BindingThingView`, `InboundRouteMatch`, and
  `BindingCapability`;
- `clinkz_wot_core::CollectionSubscriptionCapability` for the protocol-neutral topology,
  exact-source, target-bound, start, and teardown facts used by root collection plans;
- `clinkz_wot_core::BindingArtifactCompatibility`, `BindingArtifactFootprint`,
  `BindingArtifact`, `BindingArtifactEnvelope`, `BindingArtifactRef`, `BindingCompilerInput`, and
  `BindingCompilerExtension` for the portable, side-effect-free compiler and opaque artifact
  contract;
- use WP-100 `PlanId`, `BindingId`, `BindingGeneration`, slot ids, `EffectiveSecurityPlan`,
  and compact metadata references rather than cloning static request data.

Implement the frozen compiler-owned surface:

- `clinkz_wot_planning::CapabilityIndex`, `PlanCompiler`, `PlanBuildIdentity`, `PlanBuildInput`,
  `PlanBuildCursor`, `PlanBuildOutput`, `PlanFootprint`, `CompiledUriTemplate`, and
  `ResolvedFormTarget`.

Compile `CollectionSubscriptionCapability` so it records topology, exact source
attribution, maximum target count, start semantics, and teardown semantics. A standard collection
plan is admitted only when a compatible root form and one binding generation provide those
facts. Protocol wildcard or topic-filter syntax remains private to a concrete binding compiler.

Move `ResolvedFormTarget` to its frozen planning owner and public path. Replace the remaining
current selection views with the target compiler inputs/outputs or make narrowly useful helpers
private. `PlanBuildInput` captures an immutable startup registration/capability snapshot, limits,
source identity, and TD view. `PlanBuildCursor` owns bounded resumable build state and provisional
pure artifacts. `PlanBuildOutput` returns the complete immutable material and exact footprint for
one unpublished Frozen plan-set draft, including shared logical plans, compact binding references,
artifact envelopes, lazy descriptors, and structured failures without embedding execution trait
objects or Servient lifecycle state.

Resolve effective operation, root-versus-affordance form context, original form index, `base`
plus relative `href`, media defaults, response metadata, URI variables, security inheritance,
scope, extensions, and stable plan identity exactly once per logical form. Preserve TD order in
candidate vectors and retain enough source identity for strict selection and diagnostics.

## State and Ownership Migration

- Build separate client and server `CapabilityIndex` values from the complete startup-only
  registration snapshot, keyed by resolved scheme and declared secondary capabilities. Store the
  captured binding and configuration generations with every compact binding and artifact
  reference.
- Share one `LogicalInteractionPlan` across form-binding pairs. A `BindingPlanRef` owns only
  binding identity/generation, static support outcome, and a checked artifact reference.
- Admit every probe, wildcard, candidate, schema/security node, URI byte, compiled byte, lazy
  descriptor, compiler cursor, and temporary byte through the WP-000 limits and ledger before
  handing a Frozen draft to WP-400.
- Provide deterministic, resumable compiler operations and the immutable lazy descriptors needed
  for single flight. WP-400 owns compiler leases, waiter capacity, Ready/Negative publication,
  drain, eviction, and incremental reclamation; callbacks still run outside registry and eviction
  locks.
- Key artifacts and eligible deterministic negative results by the complete captured snapshot.
  V1 has no runtime binding add, remove, replace, or in-place generation invalidation. A new
  binding, configuration, compiler, policy, or schema snapshot applies only to a new Servient or a
  newly admitted generation and never rewrites an existing handle or scans its plan set.
- Keep credentials and per-call credential generations out of planning inputs and cache
  invalidation. Runtime security applicability selects from the precompiled security expression.
- Keep source documents authoritative when retained. Effective views use immutable sharing,
  overlays, indexes, or side tables; owned effective-document materialization is explicit and
  charged.
- Carry only `HandlerSlotId`, operation, generation, and response facts needed for later
  dispatch. Do not embed a handler object, associated handler future,
  `HostHandlerFuture`, step state, generated static registry, or compatibility
  adapter in an immutable logical or binding plan.

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
  maps into `OutboundRequest`.
- Remove planning paths that expand a standard root collection operation into N affordance
  operations, `EventStream`, or `Subscription::merge`. With no compatible root form, selection
  returns the structured no-compatible-form failure.
- Remove any full logical-plan copy stored per binding candidate and any invalidation path that
  synchronously scans all Things or plans.
- Do not move `ClientBinding`, `ServerBinding`, or their registrations into
  `clinkz-wot-planning`; that ownership would violate the frozen dependency graph.
- Do not add a runtime registration-replacement API or rebuild existing handles when a different
  startup registration bundle is used by a later Servient instance.
- Do not remove or extend the staged handler/emission compatibility bridge in this package.
  WP-300 owns `ProducerEmission` and its adapters, WP-400 owns host handler activation and the
  legacy handler-path removal, and WP-600 owns concrete-protocol `PublisherSink` removal.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `logical-plan-footprint` for two-level sharing, compact binding references, and immutable
  primary/additional response-classification facts;
- `capability-index-pruning` for keyed probes and admitted wildcard work;
- `bounded-candidate-selection` for strict/fallback selection and 1/8/32 limits;
- `lazy-plan-single-flight` for pre-reserved lazy descriptors, resumable compiler cursors, and the
  deterministic compiler-side contract consumed by WP-400 single-flight state;
- `plan-generation-snapshot-isolation` for startup-only snapshot pinning, O(1) generation
  comparison, and proof that a later snapshot does not mutate or scan existing plan sets;
- `compiled-plan-set-draft` for bounded resumable construction, exact immutable material and
  footprint, and transfer of one unpublished Frozen draft without Servient lifecycle state;
- `binding-compiler-extension` for a third-party core-owned compiler extension, deterministic
  bounded artifacts, identity mismatch rejection, and absence of protocol side effects;
- `admission-transaction-rollback` for exact charges, phase release, and peak memory;
- `native-collection-plan-selection` for root-form selection, exact source attribution, typed
  capability rejection, one selected binding generation, and proof that no implicit fan-out plan
  is produced.

These records satisfy the corresponding requirement-index evidence families:

- `plan-cost-and-limits` for structural sharing, exact charges, rollback, and one-over limits;
- `index-lazy-request-size` for capability pruning, wildcard bounds, lazy policy, and compact
  per-call records;
- `lazy-cache-single-flight-generation` for races, negative classification, complete snapshot
  isolation, bounded reclamation, and no global callback lock;
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
plan-set, registration, compiler, configuration, schema, and policy generations used by the case.
Credential identity is recorded only by runtime selection evidence and is never a planning-cache
dependency.

## Performance Workloads

- `PERF-GW-003` and `PERF-CS-003` cover compiled plan lookup.
- `PERF-GW-004` and `PERF-CS-004` cover the 32-candidate admitted bound.
- `PERF-GW-011` and `PERF-CS-010` consume the compiler-side counters and fixtures in WP-400's
  concurrent single-flight lazy-compilation runs.
- `PERF-GW-012` covers snapshot-generation isolation across 4,096 Things without rewriting or
  eagerly scanning existing plan sets.
- `PERF-GW-014`, `PERF-GW-015`, `PERF-CS-013`, and `PERF-CS-014` cover profile-maximum
  planning and one-axis byte/structure scaling.
- `PERF-DIR-001` and `PERF-DIR-006` consume the same protocol-neutral request/publication
  planning primitives; Directory client orchestration remains WP-500.
- `PERF-DIR-009` covers Directory-facing admission byte and structure scaling on the shared
  planning substrate.
- `PERF-GW-023` proves constant-time compiled emission-target lookup without TD rescans, and
  `PERF-GW-026` covers publication-target construction at maximum exposure scale.
- `PERF-GW-027` and `PERF-CS-019` cover exact-source native collection plan selection with one
  root-form start and no per-affordance fallback.
- `PERF-GW-029` and `PERF-CS-021` cover plan-set build, lazy artifact single-flight, generation
  pinning, snapshot isolation, and bounded reclamation in the host and static profiles.

## Completion Conditions

- Every WP-200 ownership item exists at its frozen package and public path in all applicable
  feature cells; core owns the portable compiler/artifact SPI, and the planning crate contains no
  binding execution trait or Servient registration.
- Plan fixtures cover root and affordance forms, multiple forms, relative targets, strict form
  selection, ordered fallback, inherited/form security, and structured selection errors.
- Collection fixtures prove that each standard root operation creates one native plan and rejects
  missing or inexact collection capability instead of silently creating per-affordance plans.
- Structural tests prove logical plans are shared rather than copied per binding pair and
  per-call requests contain only varying data plus compact plan references.
- Capability probes are pruned by generation-bearing indexes, wildcard work is admitted, and
  the 1/8/32 scaling and maximum-profile workloads emit schema-valid results.
- Compiler steps are deterministic, bounded, resumable, and nonblocking across unrelated keys;
  the WP-200 fixtures drive the core compiler SPI without protocol side effects, while WP-400
  owns single-flight publication and reclamation evidence.
- All obsolete public selector views, per-call TD scans, execution-trait ownership leaks, and
  eager global invalidation scans listed above are removed.
- Source inspection proves planning has no dependency on either the legacy handler surface or the
  future WP-300 `ProducerEmission` implementation boundary.
