# WP-100 Core Interaction Semantics

Status: In Progress

Design revision: v4.9

Depends on: WP-000

Global convergence gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core, clinkz-wot-foundation, clinkz-wot-td

Handler entry prerequisite: WP-100-FOUNDATION-REFRESH -> WP-100-HANDLER-ENTRY

## Scope

Refactor `clinkz-wot-core` around the frozen interaction, handler, error, codec, security,
identity, status, and cleanup value contracts. Establish one-decode validation/security flow
and guarantee that application, provider, codec, and status callbacks run outside engine
locks or constrained critical sections.

This package does not build logical/binding plans, registration indexes, binding execution
traits, Servient lifecycle orchestration, Directory clients, or concrete protocols. WP-200
and WP-300 consume the values and callback invariants established here.

The first handler subtranche is an additive cross-crate prerequisite owned by
WP-100. It preserves the historical v4.8 139-field `ResourceKind` prefix,
appends the exact 56-field v4.9 planning and Protocol Binding resource
projection, and regenerates the named profiles, generated schema, snapshots,
and boundary tests. The resulting active schema has 195 fields. The previously
assigned `WorkClass::HandlerSteps` and three core pending-work variants remain
part of the same foundation checkpoint. ADR-0015 also removes implicit copying
from `ResourceLimits`, changes `StaticResourceProfile` to expose a static
reference, and makes `WorkBudget` nonduplicable. This does not reopen the
completed WP-000 package; new evidence is recorded against WP-100 before
implementation continues.

The machine-readable tranche record in `docs/work-packages/index.toml` is the
source of truth for this prerequisite. Ordinary design validation permits its
`pending` state, but handler implementation entry requires
`tools/check-design-artifacts.sh --handler-entry-ready` to verify that the
tranche is complete and backed by same-revision evidence.

The tranche's affected requirement set is exactly `API-RESOURCE-001`,
`API-SURFACE-001`, `CONSTRAINED-PROGRESS-001`,
`CONSTRAINED-STORAGE-002`, `CONSTRAINED-WORK-001`, `RES-LIMIT-001`,
`RES-LIMIT-002`, and `RES-PROFILE-001`. Handler, planning, binding, and cleanup
requirement ids in individual CSV resource rows are provenance for later
consumers, not behavioral implementation scope for this tranche.

## Requirements

- `CONCUR-LOCK-001`, `CONCUR-USER-001`, `CONCUR-LIN-001`, and `CONCUR-CRIT-001` govern lock
  order, reentrancy, publication, and bounded critical sections.
- `HANDLER-API-001`, `HANDLER-SUB-001`, `HANDLER-CANCEL-001`, `HANDLER-CANCEL-002`, and
  `HANDLER-STORAGE-001` govern operation-specific handlers and sparse storage.
- `API-TYPES-001`, `API-PAYLOAD-001`, `API-OPTIONS-001`, `API-SURFACE-001`,
  `API-SECURITY-001`, `API-CODEC-001`, and `API-HOT-ID-001` freeze public values and traits.
- `SEC-PERF-001`, `VALIDATE-COMPILE-001`, and `VALIDATE-REUSE-001` require side-effect-free
  probing, generation-aware reuse, and one payload decode.
- `ERR-TAXONOMY-001`, `ERR-RETRY-001`, `TIME-001`, and `CLEANUP-RECORD-001` govern errors,
  retry advice, deadlines, and bounded cleanup diagnostics.
- `CONSTRAINED-PROGRESS-001`, `CONSTRAINED-OWN-001`, and `CONSTRAINED-WORK-001` govern core
  status values and bounded incremental codec/security progress.
- `RES-LIMIT-001`, `RES-PROFILE-001`, and `API-RESOURCE-001` govern the additive handler-limit
  schema, exact named-profile values, and generated foundation surface required by this package.
- `PLAN-SET-001`, `PLAN-ARTIFACT-001`, `BIND-ROUTE-001`, `BIND-STORAGE-001`,
  `BIND-MEM-001`, `BIND-DELIVERY-001`, and `BIND-CALL-CANCEL-001` govern the
  v4.9 planning and binding limits projected into the shared foundation schema.
- `PERF-ALLOC-001` and `PERF-CALL-001` govern allocation-sensitive and composed interaction
  call paths.

## Crates and Feature Cells

- Modify Cargo package `clinkz-wot-core`. Modify `clinkz-wot-foundation` only
  for the append-only resource schema, named-profile values, generated tests
  and snapshots, `StaticResourceProfile` reference boundary, linear
  `WorkBudget`, and `WorkClass::HandlerSteps`; consume `clinkz-wot-td` only in
  the allowed dependency direction. This tranche projects limits and does not
  implement planning or binding runtime behavior.
- The `no-default` cell exposes interaction values, synchronous local dispatch roles,
  incremental codec/security roles, status values, and generation-bearing ids without
  requiring atomics, `Arc`, boxed futures, or an executor.
- The `async-no-std` cell adds cancellation-aware handler twins without selecting a runtime.
- The `std` cell adds host-erased handler storage and the internal `WotLock<T>` implementation.
  `WotLock<T>` remains crate-internal and is not an umbrella re-export.
- Keep codec implementations in codec packages. Core owns `PayloadCodec` and its state/status
  contract, not CBOR or protocol framing behavior.

## Public API and Data Migration

Implement the frozen `clinkz_wot_core` surface in these groups:

- status: `PendingWork`, `PendingWorkClass`, `StartStatus`, `ProcessEvent`, `ProcessTerminal`,
  `StepStatus`, `CleanupOutcome`, `CleanupOperation`, `CleanupRecord`, and `CleanupHandle`;
- errors: `CoreResult`, `CoreError`, `ErrorContext`, `ErrorPhase`,
  `SelectionFailureReason`, `SecurityFailureReason`, and `RetryClass`;
- identity: `ThingId`, `BindingId`, `BindingGeneration`, `PlanId`, `SubscriptionId`,
  `CorrelationId`, `ActionInvocationRef`, all frozen `*SlotId` types, `PreparedRouteId`,
  `ActiveRouteId`, and `PreparedRouteKey`;
- interaction: `AffordanceTarget`, `Payload`, `MediaType`, `ContentCoding`,
  `InteractionInput`, `InteractionOptions`, `InteractionOutput`, `InteractionStatus`,
  `ResponsePayloadRole`, `ResponseSelection`, `BindingResponseMetadata`,
  `InteractionOutputMetadata`, `HandlerContext`, `CancellationView`, `Deadline`,
  `SubscriptionAcceptance`, `HandlerFootprint`, `HandlerStep`, and
  `StaticHandlerRegistration`; the GAT future is associated with each async
  trait rather than exposed as a public `HandlerFuture` alias;
- codec: `PayloadCodec`, `DecodedPayload`, `PayloadDecoderState`, `PayloadEncoderState`,
  `DecodeStatus`, and `EncodeStatus`;
- security: `SecurityProvider`, `SecurityProviderGeneration`, `SecurityRequirementView`,
  `SecurityCapability`, probe/commit input and result values, `PrincipalId`, `Principal`,
  `TransportAuthMaterial`, `BodySecurityPlanView`, `BodyAuthSlot`,
  `ApplicationPayloadProjection`, `BodyAuthProjector`, `AuthMaterial`, `AppliedSecurity`,
  `CredentialStore`, credential probe/lease/generation values, and `EffectiveSecurityPlan`.

Before adding these handler values, refresh the foundation schema from
`docs/resource-limits.csv`. Indices `0..=138` are the immutable v4.8
checkpoint. Indices `139..=194` are exactly the 18 compiled-plan-set and
artifact limits followed by the 38 route, ingress, host-call, subscription,
typed-slot, temporary-poll, response, cancellation, cleanup-transfer, wake,
and reactor-queue limits registered by design revision v4.9. Gateway and
static-reference values are finite and nonzero for every appended field; all
56 fields are `NA` for `DirectoryClientDefaultV1`. Regenerate the three
feature-cell compile fixtures, profile snapshots, and exact/one-over boundary
tests. Every bounded handler `start`, `step`, `cancel`, or constrained adapter
poll charges its caller-supplied counter before work begins. The refresh must
not create a duplicate Rust-side resource schema or change the downward
dependency graph.

`ResourceLimits` remains explicitly `Clone` for startup customization but is
not `Copy`. `StaticResourceProfile::LIMITS` and `limits()` return
`&'static ResourceLimits`; a bare `ResourceProfileId` never authorizes values.
`WorkBudget` implements neither `Clone` nor `Copy`, and every progress API
consumes one unique value through `&mut WorkBudget`. Update the no-std surface
fixture so it retains references rather than returning three complete profile
arrays by value. Do not freeze the 3,120-byte/80-byte reviewed layouts as ABI;
test the ownership and reference contracts instead. A dependency-free
compile-time ambiguity assertion must prove `ResourceLimits: Clone + !Copy`,
`WorkBudget: !Clone + !Copy`, and the exact `&'static ResourceLimits` profile
accessor types; checking only a derive line is insufficient.

Preserve only `CoreResult`, `ThingId`, and `PrincipalId` in place as allowed by the ownership
matrix. Preserve the public name and variants of `AffordanceTarget`, but replace its
`Arc<str>` representation with bounded `alloc` ownership that requires neither atomic reference
counting nor pointer-width atomics. Replace every other listed current representation or add the
absent target type. Public struct fields remain private unless the design intentionally freezes
direct access; constructors validate bounded ids, media metadata, messages, and byte storage.

The exact error, retry, correlation, and cleanup schemas and the coordinated
legacy mapping are frozen by `docs/amendments/WP-100-error-cleanup-v1.md`.
The success/error boundary, shared default error disposition, handler-absence
mapping, and legacy Servient predicate removals are frozen by
`docs/amendments/WP-100-error-disposition-v1.md`.
The exact metadata methods and response-validation staging are frozen by
`docs/amendments/WP-100-interaction-output-api-v1.md`; WP-100 implements the
interaction values but does not replace the WP-300-owned route-bearing response
envelope with an interim public type.

The exact operation stems are `ReadProperty`, `WriteProperty`, `ObserveProperty`,
`UnobserveProperty`, `InvokeAction`, `QueryAction`, `CancelAction`, `SubscribeEvent`,
`UnsubscribeEvent`, `ReadAllProperties`, `WriteAllProperties`, `ReadMultipleProperties`,
`WriteMultipleProperties`, `ObserveAllProperties`, `UnobserveAllProperties`,
`QueryAllActions`, `SubscribeAllEvents`, and `UnsubscribeAllEvents`. Core owns exactly three
traits for each stem: `{Stem}Handler`, `Async{Stem}Handler`, and `Step{Stem}Handler`. The 54
public trait paths, feature cells, and migration dispositions are enumerated individually in
`docs/api-ownership.csv`; a catch-all handler trait is not an equivalent public surface.

Every flavor receives the frozen `HandlerContext` and `InteractionInput` boundary and returns
the operation result frozen by the handler amendment. `InteractionInput` is the sole owner of
application input, verified principal, URI variables, correlation facts, deadline, and current
cancellation snapshot. `HandlerContext` is a call-lifetime dispatch-identity view; it does not
duplicate or override those facts. Async twins expose a feature-additive GAT associated future;
only the crate-private `std-async` `HostAsyncAdapter` erases it into
`HostHandlerFuture` after the generic host setter proves
`for<'a> <H as Async{Stem}Handler>::Future<'a>: Send`. Step twins return `HandlerStep` and
accept a refreshed cancellation view and typed `WorkBudget` on every bounded call. Codec state
reports exact consumed and produced bytes and resumes without re-decoding prior input.

Append `PendingWorkClass::HandlerCall = 1 << 11`,
`PendingWorkClass::ProducerSubscriptionSetup = 1 << 12`, and
`PendingWorkClass::ProducerSubscriptionTeardown = 1 << 13` to the public `#[repr(u16)]`
bit schema. Every existing discriminant through `RouteCleanup = 1 << 10` remains unchanged.
These bits describe maintained, locally progressable work; they do not treat a synchronous
callback currently running on another thread as ready local work.

WP-100 owns the portable `StaticHandlerRegistration` descriptor, not a heterogeneous constrained
registry object. A constrained application registers any subset of the 54 traits in caller-owned
generated tables or explicit static fields keyed by `HandlerSlotId`; each descriptor carries the
generation-bearing slot id, borrowed handler, and declared `HandlerFootprint` charged before
publication. The generated field or dispatch method fixes the operation and flavor through its
concrete handler bound; the descriptor does not duplicate those facts at runtime. Generated table
storage and lookup code remains application-owned, while the descriptor has the single frozen
core path recorded in `docs/api-ownership.csv`. Both use the same traits, generation checks,
replacement reducer, and resource admission rules. This prevents an erased registry from silently
requiring `Arc`, atomics, `Send`, `Sync`, or boxed futures.

## State and Ownership Migration

- Select or clone the minimum handler/provider reference while holding its slot boundary,
  release all engine locks or critical sections, and only then call user code. Apply the same
  rule to probe, commit, codec extension, and status-sink callbacks.
- Linearize handler replacement at slot publication. An admitted dispatch retains one old or
  new handler for its entire call and never switches after selection.
- Implement the crate-private records `SelectedHandlerEntry`, `HandlerCallOwner`,
  `CallbackLease`, `ProducerSubscriptionOwner`, and `HandlerCleanupOwner` exactly as assigned by
  the ownership matrix. `HostStepAdapter` is `std`-only. These records make selected generation,
  pending-call admission, non-preemptible synchronous late return, callback exclusivity,
  subscription setup/teardown, and retained cleanup explicit without exposing mutable slots.
- Charge handler count and retained bytes before a replacement is published. A failed
  replacement leaves the previous generation selected and releases every temporary charge.
  At most the active and one retiring generation occupy one slot; a selected retiring generation
  remains charged until its final `HandlerCallOwner` and `CallbackLease` release.
- Decode the wire payload once into `DecodedPayload`. `BodyAuthProjector` extracts body
  credentials into `BodyAuthSlot` and returns the application projection; security and schema
  validation reuse that representation or an overlay.
- Keep probes side-effect-free. Commit exactly one selected security branch, keep credential
  leases out of immutable plans, and invalidate reusable results by provider, credential,
  schema, or policy generation.
- Make `CleanupRecord` bounded and redacted. It references cleanup identity and ownership but
  never clones a plan, payload, credential, TD, or unbounded error chain.
- Make `StepStatus::Progress` able to carry both a value and maintained `PendingWork`; do not
  scan registries to construct the pending summary.

## Old API Removal

- Replace the current `core/src/error.rs::CoreError` variants with the frozen taxonomy and
  retry/context model; remove conversions that flatten selection, limit, cancellation, and
  execution failures into strings.
- Replace the current `core/src/payload.rs::PayloadCodec` and `CodecInput` surface with the
  bounded whole-value plus incremental state API. Remove any double-decode validation path.
- Replace the current `SecurityContext`, `Credentials`, `AuthMaterial`, `SecurityProvider`, and
  `CredentialStore` shapes with the frozen probe/commit and lease model. Remove public access
  to credential-bearing debug output and immutable-plan credential storage.
- Replace the current `CorrelationId` representation that requires `Arc` in portable builds
  with a bounded no-std-capable owned representation.
- Add the target handler values, 54 traits, reducers, and constrained storage boundary without
  deleting a facade still required by a later package. WP-300 first supplies
  `ProducerEmission` plus compatibility adapters; WP-400 then removes `PushFn`, the
  `SubscriptionSender` handler path, the legacy nine sync/async handler families, `ReadSlot`,
  `WriteSlot`, `ObserveSlot`, `UnobserveSlot`, `InvokeSlot`, `QuerySlot`, `CancelSlot`,
  `SubscribeSlot`, `UnsubscribeSlot`, and all nine public raw handler lookup methods. WP-600
  removes `PublisherSink` from concrete protocol publication. WP-700 proves the final absence.
- No new WP-100 implementation may call a legacy push or raw lookup surface. The bounded
  compatibility interval above is a downstream migration bridge, not a deprecated target API,
  and its removal owners are recorded as explicit completion conditions in those packages.
- Remove any public re-export of internal `WotLock` or mutable registry access once the target
  handler surface is available.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `handler-foundation-refresh` for the immutable 139-field prefix, exact
  56-field v4.9 suffix, all named-profile values including `NA`,
  `WorkClass::HandlerSteps`, generated snapshots, boundaries, and three feature
  cells;
- `core-public-surface` for paths, feature cells, owned values, and trait shapes;
- `handler-api-matrix` for every one of the 18 operation results, three handler flavors, and
  applicable compilation cells;
- `handler-cancellation` for sync-late, async-cooperative, and bounded-step behavior;
- `handler-storage-replacement` for exact/one-over count and byte admission, rollback, sparse
  density, generation pinning, clear, and replacement races;
- `affordance-target-no-atomics` for a true constrained-target compile fixture proving that
  `AffordanceTarget` and the static handler boundary require no atomic reference counting;
- `callback-lock-isolation` for handlers, providers, codecs, and status callbacks;
- `security-selection-and-redaction` for probe/commit, leases, body projection, and secrets;
- `codec-validator-reuse` for incremental accounting and one-decode reuse;
- `core-error-taxonomy` for category, context, redaction, and retry mappings.

These records satisfy the corresponding requirement-index evidence families:

- `lock-order-and-reentrancy` and `linearization-races` for callback and replacement races;
- `handler-ownership`, `cancellation-and-late-results`, and `sparse-handler-storage` for sync,
  async, poll, and unused-operation cases;
- `common-public-types` and `frozen-cross-crate-surface` for exact paths and feature cells;
- `security-probe-commit` for capability generations, branch selection, side effects, leases,
  and redaction;
- `validation-codec-reuse` for incremental byte accounting and one-decode projections;
- `error-taxonomy` for exhaustive category, context, redaction, and retry mappings;
- `hot-records` for id and cleanup-record size/allocation assertions;
- `manual-runtime` for `StepStatus`, `PendingWork`, and typed-budget progress behavior.

Each test that invokes user code instruments lock/critical-section ownership and fails if a
callback observes an engine guard. Panic-freedom and secret-redaction checks are required for
all public error and debug representations.

## Performance Workloads

- `PERF-GW-001` and `PERF-CS-001` cover allocation-sensitive synchronous local interaction.
- `PERF-GW-002` and `PERF-CS-002` cover core inbound dispatch without transport cost.
- `PERF-GW-005` and `PERF-CS-005` cover bounded security branch/provider probes.
- `PERF-GW-006` and `PERF-CS-006` cover one-decode composed validation/security interaction.
- `PERF-GW-020` and `PERF-CS-015` cover sparse handler density, flavor mix, replacement,
  rollback, clear, and generation retention at the exact storage limits.
- `PERF-GW-021` and `PERF-CS-016` cover synchronous late return, async drop, bounded-step
  cancellation, replacement during dispatch, and reentrant handler operations.
## Completion Conditions

- The WP-100 handler-foundation subtranche is complete before implementation
  resumes: generated schemas and snapshots contain the unchanged 139-field
  v4.8 prefix and exact 56-field v4.9 suffix for 195 fields total, plus
  `WorkClass::HandlerSteps` and the three core pending-work variants, with no
  duplicate schema or reopened WP-000 completion claim.
- Every WP-100 ownership item exists at its frozen path in each applicable feature cell, and
  `--no-default-features` has no required `std`, atomics, `Arc`, executor, or boxed-future path.
- Reentrant handler, provider, codec, and status callbacks run with no engine lock or critical
  section held; replacement and cancellation race tests pass.
- Every operation has the exact sync, async, and step trait and result shape; sparse constrained
  registration proves zero storage for unsupported operations, while host setter activation
  remains assigned to WP-400.
- Security probe/commit selection, lease lifetime, body projection, one-decode validation, and
  generation invalidation have focused tests and bounded failure behavior.
- `CoreError`, retry advice, hot ids, cleanup records, codec states, and progress values satisfy
  their compile, size, redaction, and transition evidence.
- All listed workload adapters attributable to core are registered and emit schema-valid result
  identity, even when a later package supplies the final end-to-end runner.
- The old public security, codec, error, and lock facades owned by WP-100 are absent. Only the
  explicitly staged handler/emission compatibility bridge may remain, and source inspection must
  prove it has no new callers and names WP-300, WP-400, WP-600, and WP-700 as its removal owners.
