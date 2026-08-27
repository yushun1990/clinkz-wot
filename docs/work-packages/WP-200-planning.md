# WP-200 Logical and Binding Planning

Machine-readable package status, design revision, dependencies, document path,
and owner crates are defined only in [`index.toml`](index.toml). This document
specifies technical scope and acceptance boundaries.

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

The broad response-classification staging in
`docs/amendments/WP-100-interaction-output-api-v1.md` is v5-inactive domain-entry
input. A later broad WP-200 review may re-adopt primary/additional branch and
schema/media planning facts; the current narrow Property Read response-sealing
prerequisite neither compiles those deferred facts nor publishes an interaction
response.

Collection subscription planning is first-class. `subscribe_all_events` and
`observe_all_properties` select one compatible Thing-level form from the root `forms` array;
planning never lowers either operation into per-affordance requests or a local merged stream.

WP-200 consumes the WP-100 handler context and operation identities only as immutable plan
facts. It neither activates host handler registrations nor removes a compatibility facade needed
by WP-300, WP-400, or WP-600. New planning code must not call `PushFn`, `PublisherSink`,
`SubscriptionSender`, a legacy raw handler lookup, or an old handler trait.

ADR-0017 closes the AR-004 selection/fallback design gap without returning form
selection to a Protocol Binding. `CandidateFallbackPolicy::PreExecution` is the
Consumer default; only side-effect-free security inapplicability and an exact
deterministic lazy-artifact negative may skip a candidate. Binding input
rejection, mutable health, transient or bounded-progress failure, security
commit, and all post-acceptance outcomes never trigger automatic fallback.
Every eligible skip has one pre-reserved, fixed-width diagnostic bounded by the
admitted candidate count.

The narrow Property Read planning boundary has bounded immutable input, no
runtime TD reads, identical static and host semantics, and explicit exclusions
from broad planning. Owning-crate tests cover both authoring profiles and the
owned output lifetime.

One associated-type Core contract serves both forms: an application-closed
compiler/cursor/artifact enum keeps constrained storage typed, while Core owns
safe host erasure and returns the complete erased cursor or artifact unchanged
on mismatch. WP-200 is the sole implementation owner of those Core components
and Planning coordination. WP-300 consumes one component only when it later
builds the complete installable registration; it does not implement a second
compiler/artifact SPI.


## v5.1 Consumer Property Read entry slice

Active authority consumed by this slice:

- `PLAN-REQUEST-001` plus active `PLAN-SET-001`, `PLAN-ARTIFACT-001`,
  `PLAN-BOUND-001`, `PLAN-COST-001`, and `PLAN-COST-003`.

Minimum scope:

- make the existing bounded Property Read compiler path constructible for
  `BindingArtifactRole::ConsumerCall` through a reviewed public Planning entry;
- build/freeze/publish one consumed Property Read plan using an eager admitted
  Consumer artifact;
- select only inside that immutable plan set using the narrowed options kernel;
- preserve the plan/binding/artifact generations required to build one
  `OutboundRequest`;
- prove the TD and compiler build inputs can be dropped before the call path.

Explicitly excluded:

- `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`, `PLAN-COST-002`;
- automatic candidate fallback;
- additional-response breadth;
- multi-binding fairness or performance closure.

The target-path negative fixture must poison call-time TD/Form scanning and raw
binding support probing.

### ADR-0013 Consumer tranche: `WP-200-CONSUMER-PROPERTY-READ-PLANNING`

This is the exact v5.1 Planning tranche that follows the completed
`WP-100-CONSUMER-CALL-VALUES-VALIDATOR` predecessor. It exposes the existing
bounded eager Consumer Property Read compiler and the first immutable-plan-only
selection projection. It does not broaden the planner beyond one eager
Consumer-call candidate.

The affected active requirements are exactly:

- `PLAN-REQUEST-001`;
- `PLAN-COST-001`;
- `PLAN-COST-003`;
- `PLAN-BOUND-001`;
- `PLAN-SET-001`;
- `PLAN-ARTIFACT-001`; and
- `API-OPTIONS-001`.

The tranche has one predecessor tranche,
`WP-100-CONSUMER-CALL-VALUES-VALIDATOR`, which must be complete. No broad
WP-100 package-completion claim is required because the roadmap admits this
Consumer path by exact tranche dependency.

Permitted production source paths are exactly:

- `planning/src/property_read.rs`; and
- `planning/src/lib.rs`.

A required production change outside those paths stops implementation and
returns the tranche to impact review.

The public Consumer compiler entry is frozen to:

```rust
impl PropertyReadPlanCompiler {
    pub fn consumer_call(
        plan_id: PlanId,
        property_name: Box<str>,
        form_index: u32,
        registration: BindingRegistrationIdentity,
        registration_index: u32,
        candidate_order: u32,
    ) -> Self;
}
```

`consumer_call` owns one exact Property Read target coordinate: `property_name`
plus the index in that property's own form array. It copies the complete binding
id, binding generation, configuration digest, and artifact compatibility from
`registration` and fixes `BindingArtifactRole::ConsumerCall`. It does not
accept arbitrary role or split registration-identity fields. The existing
`producer_route(...)` contract is unchanged.

The Consumer builder MUST compile exactly that target coordinate. A TD may have
multiple readable properties and multiple readable forms. Their document order
must not select the Consumer target. Missing property, out-of-range form index,
or a targeted form that does not support `ReadProperty` is a structured
selection failure; the builder MUST NOT scan forward/backward to another
property or form and MUST NOT silently emit the first readable competitor. This
coordinate is startup/build input only and is retained in the owned logical
plan; it is not a call-time TD/Form selection mechanism.

The public immutable-plan selection entry is frozen to:

```rust
pub fn select_consumer_property_read<A>(
    output: &PlanBuildOutput<A>,
    property_name: &str,
    options: &InteractionOptions,
) -> CoreResult<BindingArtifactRef>;
```

This selector is deliberately incapable of receiving a TD, raw Form,
registration snapshot, compiler, binding execution object, or support-probe
callback. It operates only on the owned immutable `PlanBuildOutput`, the
application-addressed property name, and the narrowed call options. The exact
build target is therefore chosen before this selector exists; this selector can
only accept or reject the already-compiled coordinate and cannot repair or
reinterpret a wrongly built target.

For this first slice the admitted output shape is exactly one owned Property
Read logical plan for the constructor's exact target coordinate, one eager
`ConsumerCall` artifact envelope, and one matching `BindingArtifactRef`.
Selection validates that the logical plan, envelope, and reference agree on
plan, plan-set, binding, binding-generation, configuration, compatibility,
role, and artifact slot before returning the reference. A mismatched or forged
narrow output is rejected before any binding work.

The selector applies only these call-time rules:

- the addressed `property_name` must equal the immutable logical-plan target;
- omitted `form_index` leaves the already-admitted form selected;
- explicit `form_index` must equal that plan's original form index or returns
  `CoreError::Selection` with `StrictSelectionMismatch`;
- URI-variable values and timeout intent remain call-varying facts for WP-300
  and do not trigger planning reinterpretation; and
- legacy `InteractionOptions::data` is ignored and cannot affect planning or
  selection.

A missing addressed property returns the existing structured
`AffordanceMissing` selection failure. This tranche has no automatic candidate
fallback, second-candidate probing, lazy artifact creation, or runtime support
probe.

The compiler output must remain usable after the validated TD, registration
snapshot, compiler registration, and `PlanBuildInput` have all been dropped.
The target-path test then performs selection from that owned output after those
sources are gone. This is the required negative proof against call-time TD/Form
scanning and raw binding support probing.

WP-200 produces an unpublished immutable eager plan-set draft. `PLAN-SET-001`
publication, pins, leases, draining, and reclamation remain WP-400 ownership;
this tranche must not create Servient publication state merely to satisfy the
word "publish" in older roadmap shorthand.

Exact exclusions are:

- no `OutboundRequest`, ClientBinding execution, call settlement, or response
  delivery;
- no Servient consumed-plan publication or application facade;
- no additional Consumer operation family;
- no capability index, lazy/cache/single-flight, or automatic fallback;
- no additional-response planning breadth;
- no binding-id/media/subprotocol/security-branch/validation-profile option;
- no TD/Form or registration back-reference in the selected result; and
- no Consumer architecture-gate registration or completion claim.

The authoritative tranche registration is
`docs/work-packages/index.toml`. Completion evidence uses stable key
`consumer-property-read-planning-selection` at
`docs/evidence/WP-200-consumer-property-read-planning-selection.toml`.
Before the tranche becomes complete, that evidence must record the exact
implementation checkpoint and passing results for:

- static and Host `consumer_call` builds with equal selected identity;
- a TD containing at least two readable properties, with at least two readable
  forms on the targeted property, where the constructor targets a non-first
  property and non-first form and the output proves that exact coordinate;
- missing property, out-of-range form index, and targeted non-ReadProperty form
  failures that do not fall back to any readable competing property/form;
- exact eager `ConsumerCall` role and absence of Producer route reservation;
- owned output surviving TD, registration snapshot, compiler registration, and
  build-input destruction;
- selection after those source values are gone;
- omission and explicit matching `form_index` success;
- wrong call-time property and strict form mismatch failures;
- rejection of forged/mismatched output identity;
- proof that URI variables, timeout, and legacy data do not cause replanning or
  alter the selected static reference;
- no-default, async-no-std, and std Planning feature-cell compilation; and
- normal locked workspace and authority validation.

Completion of this tranche does not claim broad WP-200 completion, WP-300,
WP-400, or the Consumer Property Read architecture gate.

## Requirements

This package is governed by:

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
security, validation, resource-profile, and lifecycle contracts defined by the current specifications
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
- `clinkz_wot_core::BindingArtifactCompatibility`,
  `BindingArtifactFootprint`, `BindingArtifactRole`,
  `BindingCompilerBounds`, `BindingArtifactIdentity`, `BindingArtifact`,
  `BindingArtifactEnvelope`, `BindingArtifactRef`,
  `BindingArtifactRejection`, `BindingCompilerInput`,
  `BindingCompilerOutput`, `BindingCompilerFailure`, `BindingCompilerStep`,
  and `BindingCompilerExtension` for the portable, side-effect-free compiler
  and opaque artifact contract;
- `clinkz_wot_core::StaticBindingCompilerRegistration<C>` in every feature
  cell and `HostBindingCompilerRegistration`, `HostBindingCompilerCursor`, and
  `HostBindingArtifact` under `std`; these are constructible compiler
  components but are never independently installable;
- use WP-100 `PlanId`, `BindingId`, `BindingGeneration`, slot ids, `EffectiveSecurityPlan`,
  and compact metadata references rather than cloning static request data.

Implement the frozen compiler-owned surface:

- `clinkz_wot_planning::CapabilityIndex`, `PlanCompiler`, `PlanBuildIdentity`,
  `PlanBuildInput`, `PlanBuildCursor`, `PlanBuildOutput`,
  `PlanBuildFailure`, `PlanBuildStep`, `PlanFootprint`, `CompiledUriTemplate`,
  `ResolvedFormTarget`, `CandidateFallbackPolicy`, `CandidateSkipReason`,
  `CandidateSkipDiagnostic`, and `CandidateSelectionDiagnostics`.

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

The Property Read slice implements only this additive surface:

- one Core-owned Property Read `LogicalInteractionPlan` constructor and compact
  `BindingCandidate`;
- the complete portable compiler/artifact values and host/static component
  registrations;
- generic `PlanBuildInput<'a, R>`, `PlanBuildOutput<A>`, `PlanBuildStep`, and
  `PlanCompiler<R>`;
- one bounded Property Read builder that reads a validated TD only while the
  build input is borrowed; and
- owned output that remains usable after the TD, registration snapshot, and
  compiler input are dropped.

It does not implement broad capability indexes, fallback/lazy caching,
collection operations, Producer form contribution, plan-set lifecycle,
binding execution, Servient publication, or either cross-package architecture
fixture root.

The scoped Producer-route projection is part of the current Property Read
boundary because the general private algorithm emits only `ConsumerCall`,
while the WP-300 registration advertises a Producer Property Read server. It
provides public
`PropertyReadPlanCompiler::producer_route` and an opaque
`PropertyReadBuildCursor<C, A>` in `planning/src/property_read.rs`, re-exports
them from `planning/src/lib.rs`, and has no broader product-source reach. The
constructor consumes the complete `BindingRegistrationIdentity`, fixes the
role to `ProducerRoute`, and accepts a borrowed static or host compiler
projection from the complete registration. Arbitrary role selection remains excluded. The earlier Producer-only exclusion of a public Consumer-call constructor is superseded only by the exact v5.1 `WP-200-CONSUMER-PROPERTY-READ-PLANNING` admission above; no broader Consumer constructor is implied.

The scoped route-reservation projection is also required because the public
Producer-route constructor preserves role and registration identity
but does not supply the protocol-canonical endpoint collision identity needed
to construct a production `BindingRouteKey`. This projection
extends the existing Core `BindingArtifact<A>` wrapper with immutable optional
route metadata, requires it exactly for `ProducerRoute`, preserves it through
static and host erasure, and exposes it through the validated artifact envelope.
The concrete compiler remains the only protocol canonicalization owner;
Planning retains but never derives or interprets the value.

Implementation ownership remains in `core/src/binding_compiler.rs` and
`planning/src/property_read.rs`. The external WP-300 mock compiler in Servient
tests exercises the real output-to-`PrepareInput` path. Form contribution,
capability indexes, collision tables, Servient lifecycle, production
protocols, and aggregate architecture fixture roots remain outside this
narrow scope.

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
- While legacy concrete callers remain, the public selector implementation in
  `protocol-bindings/core` is a legacy-generation source boundary, not a
  target adapter. New Planning, Core, and WP-300 code must not call it or send a
  selected target artifact back through it. WP-600 removes the concrete Zenoh
  call edge and WP-700 proves final absence of the legacy selector exports.
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

## Verification Responsibilities

Owning-crate unit, integration, compile-fail, and workload tests cover these
technical invariants:

- `property-read-plan-slice` for one immutable property-read logical plan,
  bounded construction from a read-only TD, a binding artifact sufficient
  without runtime TD access, and the exact identities consumed by the
  cross-package architecture gate;
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
- `property-read-producer-route-projection` for a real complete WP-300
  registration whose borrowed compiler projection produces a real
  `ProducerRoute` artifact reference that constructs `PrepareInput` and starts
  real route preparation after TD and registration-projection borrows end;
- `property-read-route-reservation-projection` for a real complete WP-300
  registration whose compiler supplies the canonical Producer-route
  `RouteReservationIdentity`, whose validated envelope preserves it through
  static and host erasure, and whose runner constructs no substitute collision
  or endpoint identity;
- `admission-transaction-rollback` for exact charges, phase release, and peak memory;
- `native-collection-plan-selection` for root-form selection, exact source attribution, typed
  capability rejection, one selected binding generation, and proof that no implicit fan-out plan
  is produced.

Coverage also includes these requirement families:

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

Tests must distinguish admission failure from first-use compilation failure and record the
plan-set, registration, compiler, configuration, schema, and policy generations used by the case.
Credential identity is recorded only by runtime selection diagnostics and is never a planning-cache
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
  selection, ordered fallback under the frozen typed policy, inherited/form
  security, bounded skipped-candidate diagnostics, and structured selection
  errors. No execution failure after binding acceptance triggers implicit
  fallback.
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
