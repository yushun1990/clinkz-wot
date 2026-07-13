# WP-100 Core Interaction Semantics

Status: Planned

Design revision: v4.6

Depends on: WP-000

Required gates: GATE-1, GATE-2, GATE-3, GATE-4, GATE-5, GATE-6

Owner packages: clinkz-wot-core, clinkz-wot-td

## Scope

Refactor `clinkz-wot-core` around the frozen interaction, handler, error, codec, security,
identity, status, and cleanup value contracts. Establish one-decode validation/security flow
and guarantee that application, provider, codec, and status callbacks run outside engine
locks or constrained critical sections.

This package does not build logical/binding plans, registration indexes, binding execution
traits, Servient lifecycle orchestration, Directory clients, or concrete protocols. WP-200
and WP-300 consume the values and callback invariants established here.

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
- `PERF-ALLOC-001` and `PERF-CALL-001` govern allocation-sensitive and composed interaction
  call paths.

## Crates and Feature Cells

- Modify Cargo package `clinkz-wot-core`; consume `clinkz-wot-foundation` and
  `clinkz-wot-td` only in the allowed dependency direction.
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

- status: `PendingWork`, `StartStatus`, `ProcessEvent`, `ProcessTerminal`, `StepStatus`,
  `CleanupOutcome`, `CleanupOperation`, `CleanupRecord`, and `CleanupHandle`;
- errors: `CoreResult`, `CoreError`, `ErrorContext`, `SelectionFailureReason`, and
  `RetryClass`;
- identity: `ThingId`, `BindingId`, `BindingGeneration`, `PlanId`, `SubscriptionId`,
  `CorrelationId`, `ActionInvocationRef`, all frozen `*SlotId` types, `PreparedRouteId`,
  `ActiveRouteId`, and `PreparedRouteKey`;
- interaction: `AffordanceTarget`, `Payload`, `MediaType`, `ContentCoding`,
  `InteractionInput`, `InteractionOptions`, `InteractionOutput`, `InteractionStatus`,
  `HandlerContext`, `CancellationView`, and `Deadline`;
- codec: `PayloadCodec`, `DecodedPayload`, `PayloadDecoderState`, `PayloadEncoderState`,
  `DecodeStatus`, and `EncodeStatus`;
- security: `SecurityProvider`, `SecurityProviderGeneration`, `SecurityRequirementView`,
  `SecurityCapability`, probe/commit input and result values, `PrincipalId`, `Principal`,
  `TransportAuthMaterial`, `BodySecurityPlanView`, `BodyAuthSlot`,
  `ApplicationPayloadProjection`, `BodyAuthProjector`, `AuthMaterial`, `AppliedSecurity`,
  `CredentialStore`, credential probe/lease/generation values, and `EffectiveSecurityPlan`.

Preserve only `CoreResult`, `ThingId`, `AffordanceTarget`, and `PrincipalId` in place as allowed
by the ownership matrix. Replace every other listed current representation or add the absent
target type. Public struct fields remain private unless the design intentionally freezes
direct access; constructors validate bounded ids, media metadata, messages, and byte storage.

Operation-specific sync handlers receive `HandlerContext` plus their typed input and result.
Async twins own their selected handler and cancellation state across suspension. Codec state
reports exact consumed and produced bytes and resumes without re-decoding prior input.

## State and Ownership Migration

- Select or clone the minimum handler/provider reference while holding its slot boundary,
  release all engine locks or critical sections, and only then call user code. Apply the same
  rule to probe, commit, codec extension, and status-sink callbacks.
- Linearize handler replacement at slot publication. An admitted dispatch retains one old or
  new handler for its entire call and never switches after selection.
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
- Remove `PushFn`, `PublisherSink`, and other handler return/push facades that bypass typed
  Producer emission ownership. Do not retain them as deprecated public aliases.
- Remove any public re-export of internal `WotLock`, raw handler slots, or mutable registry
  access once the target handler surface is available.

## Evidence

Produce these package evidence keys exactly as indexed by the work-package DAG:

- `core-public-surface` for paths, feature cells, owned values, and trait shapes;
- `handler-cancellation` for sync-late, async-cooperative, and bounded-step behavior;
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
## Completion Conditions

- Every WP-100 ownership item exists at its frozen path in each applicable feature cell, and
  `--no-default-features` has no required `std`, atomics, `Arc`, executor, or boxed-future path.
- Reentrant handler, provider, codec, and status callbacks run with no engine lock or critical
  section held; replacement and cancellation race tests pass.
- Security probe/commit selection, lease lifetime, body projection, one-decode validation, and
  generation invalidation have focused tests and bounded failure behavior.
- `CoreError`, retry advice, hot ids, cleanup records, codec states, and progress values satisfy
  their compile, size, redaction, and transition evidence.
- All listed workload adapters attributable to core are registered and emit schema-valid result
  identity, even when a later package supplies the final end-to-end runner.
- The old public security, codec, error, push, and lock facades listed above are absent; no
  unavailable-by-default compatibility feature remains.
