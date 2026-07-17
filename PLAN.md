# clinkz-wot Current Documentation Plan

The active repository-wide design revision is:

- `docs/design.md` (v4.8 design-closure candidate)

Normative implementation-support artifacts referenced by that design are:

- `docs/artifacts.csv`
- `docs/api-ownership.csv`
- `docs/refactor-gates.csv`
- `docs/resource-limits.csv`
- `docs/state-machines.toml`
- `docs/performance/manifest.schema.json`
- `docs/performance/result.schema.json`
- `docs/performance/fixtures.lock.toml`
- `docs/performance/fixture-generator.md`
- `docs/performance/constrained.toml`
- `docs/performance/gateway.toml`
- `docs/performance/directory.toml`
- `docs/requirements.csv`
- `docs/work-packages/index.toml`
- `docs/work-packages/*.md`
- `docs/amendments/WP-100-error-cleanup-v1.md`
- `docs/amendments/WP-100-error-disposition-v1.md`
- `docs/amendments/WP-100-interaction-output-api-v1.md`
- `docs/amendments/WP-100-handler-api-v1.md`
- `docs/audits/WP-100-handler-entry-audit.md`
- `docs/ADR/0001-crate-and-module-boundaries.org`
- `docs/ADR/0002-producer-emission-dispatch.org`
- `docs/ADR/0003-subscription-driver-ownership.org`
- `docs/ADR/0004-collection-subscriptions.org`
- `docs/ADR/0005-outbound-request.org`
- `docs/ADR/0006-host-binding-call-cancellation.org`

The executable performance contract checker and deterministic fixture
generator are in `tools/performance-harness`.

The v4.8 revision is the coordinated design target. It becomes the frozen
implementation-ready revision only after every affected refactor gate records
passing same-revision review evidence. It incorporates ADR-0001 through
ADR-0005: crate and module boundaries, Servient-owned emission coordination,
binding-owned subscription drivers with a Servient public facade, native
root-form-only collection subscriptions, and the directional `OutboundRequest`
binding envelope. `docs/review/review-01.org` records the non-normative evidence
and deviations that led to those accepted decisions. Normative amendment
`WP-100-HANDLER-API-001` resolves the handler-entry audit with exact
handler APIs, cancellation state, resource limits, workloads, and acyclic
migration ownership. Runtime API migration proceeds only through the dependency
order in `docs/work-packages/index.toml`; no handler implementation tranche may
start until the amendment and every affected artifact pass the registered gate
checks in the same revision.

The frozen WP-100 error, retry, correlation, and cleanup Rust schemas are closed
by normative amendment `WP-100-ERR-CLEANUP-001`. The amendment resolves schema
details left open by the base prose without changing the implementation DAG.
The success/error boundary, shared wire disposition, handler-absence mapping,
and legacy Servient predicate removal are closed by normative amendment
`WP-100-ERR-DISPOSITION-001`.

The exact binding-response metadata methods, final XOR-shaped inbound response
envelope, and package-by-package response-validation ownership are closed by
normative amendment `WP-100-OUTPUT-API-001`. WP-100 implements the interaction
value surface; route-bearing response delivery remains in its declared WP-300
owner package rather than introducing a temporary public envelope.

The output amendment added `additional_responses_per_form_max`; its completed
WP-000 checkpoint covered 118 fields. Revision v4.7 raised the authoritative
schema to 130 fields by appending nine handler and three Producer-residual
limits, and added one work class plus three pending work classes.
Revision v4.8 raises the schema to 139 fields by appending nine architecture
limits for per-binding and global binding-emission slots, collection sources,
per-binding and global host emission lanes, pending host client calls, and host
binding cancellation drain. The additive 21-field refresh is
the first active WP-100 subtranche after design gate closure; it preserves the
first 118 `ResourceKind` indices and does not reopen or rewrite the historical
WP-000 completion evidence. It must complete before handler code changes.

## Current Implementation Checkpoint

The repository is not performing an unconstrained full rewrite. Implementation
advances one dependency-ordered work package and one reviewable tranche at a
time. The completed error and interaction-output tranches remain valid. The
handler audit was resolved in design before implementation. The July
architecture review then superseded the v4.7 candidate with v4.8 before further
runtime migration. The next code change remains the bounded foundation refresh,
followed by the frozen handler value and trait surface, but only after all
affected v4.8 ownership, state, work-package, resource, and performance artifacts
agree and their gates close.

`WP-100` remains **In Progress**. Commit `9181070` completes its coordinated
error-taxonomy migration and shared default error-disposition mapping: the
frozen `CoreError` surface, retry context, legacy `SecurityError` removal,
handler-absence mapping, binding selection reasons, redacted protocol
conversions, shared default status mapping, and workspace-wide legacy-surface
evidence. Commit `3bd9aa5` completes the frozen interaction-output value
surface: the six exact value schemas, private fields, bounded additional-response
construction, metadata round trips, local `OperationStatus` shape check, and
workspace caller migration across core, Servient, Zenoh, and the umbrella
example. It deliberately leaves route-bearing `InboundResponse` replacement and
binding-response authenticity validation in WP-300. Neither commit completes
WP-100 or implements every requirement governed by the normative amendments;
those amendments remain frozen design inputs.

The remaining implementation tranches proceed in this order:

1. Refresh foundation for all 139 resource fields without changing the first
   118 `ResourceKind` indices. Append the existing twelve v4.7 fields followed
   by the nine v4.8 architecture fields. Add `WorkClass::HandlerSteps` and the
   three handler/Producer pending classes.
2. Complete the operation-specific sync, async, and bounded-step handler APIs,
   including `HandlerContext`, `CancellationView`, cancellation ownership, and
   sparse handler storage.
3. Implement `DecodedPayload`, incremental codec state, exact byte accounting,
   and one-decode validation reuse.
4. Implement security probe/commit, credential leases, generation invalidation,
   body projection over `DecodedPayload`, fail-closed outbound application, and
   secret-redacted public `Debug` behavior.
5. Complete the WP-100-owned compatibility boundary and callback isolation.
   WP-200 then freezes collection capability and native root-form plans. WP-300
   lands `OutboundRequest`, `HostSubscriptionDriver`, `ProducerEmission`, and
   one-binding `BindingEmissionSlot`, and removes the legacy public
   `BindingRequest` envelope. WP-400 lands the non-cloneable Servient
   `Subscription` facade, driver registry, `EmissionCoordinator`, private
   aggregate `EmissionRecord`, and public closed-shape
   `EmissionDispatchPolicy` configuration; it removes the core
   queue/merge and global broker ownership. WP-600 migrates Zenoh to native
   collection drivers and selected binding publication targets and deletes the
   legacy sink path. Complete bounded progress behavior and prove that every
   handler, provider, codec, binding driver, and status callback executes outside
   engine locks or constrained critical sections.
   Register the WP-100 performance workloads and close every WP-100 evidence and
   completion condition only after those API migrations are complete.

`WP-200` does not start until the WP-100 completion audit passes. Later packages
continue to follow `docs/work-packages/index.toml`; cross-crate compatibility
changes needed by a WP-100 tranche are part of that coordinated tranche, not an
independent start of a downstream package.

The Directory performance artifact covers only the engine-side Directory client
contract. Directory service topology, storage backends, server-side query
execution, and production service SLOs are deferred to a later design.
Non-normative inputs retained for that future design are in
`docs/future/directory-service.md`; they are not active engine requirements.

Implementation refactoring starts from the requirement-scoped `WP-000`
foundation package and follows the `IMPL-CONFORM-001` DAG. Existing code is not
a competing design source, and partial implementation compatibility must not
weaken the target design. Cross-crate API, state, ownership, resource, and
performance changes are coordinated before a conforming release is declared.

Historical baselines, implementation plans, target notes, audit follow-ups, and
the previous root `PLAN.md` are archived under:

- `docs/deprecated/`

For new task sessions, read `docs/design.md` first, followed by the active
artifact relevant to the task. Open deprecated documents only when historical
rationale or migration context is explicitly needed.
