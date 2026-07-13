# clinkz-wot Current Documentation Plan

The active repository-wide design revision is:

- `docs/design.md` (v4.6 frozen implementation-ready revision)

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
- `docs/audits/WP-100-handler-entry-audit.md`

The executable performance contract checker and deterministic fixture
generator are in `tools/performance-harness`.

The v4.6 base revision remains the coordinated design target. The handler entry
audit reopened GATE-1, GATE-2, GATE-4, GATE-5, and GATE-6 because the next
WP-100 tranche still requires exact handler APIs, cancellation state, resource
limits, workloads, and acyclic migration ownership. GATE-3 remains closed.
Runtime API migration proceeds only through the dependency order in
`docs/work-packages/index.toml`; affected implementation is paused as required
by `REFACTOR-GATE-001` until a normative handler amendment closes those gates.

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

That amendment adds `additional_responses_per_form_max`. The resulting `WP-000`
refresh now covers all 118 resource fields in the generated foundation schema,
profile snapshots, boundary tests, and evidence. `WP-000` is complete and
`WP-100` has resumed.

## Current Implementation Checkpoint

The repository is not performing an unconstrained full rewrite. Implementation
advances one dependency-ordered work package and one reviewable tranche at a
time. The completed error and interaction-output tranches remain valid, but the
next handler tranche is paused after its entry audit exposed design
contradictions. Affected work resumes only after a normative revision closes the
reopened gates.

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

Before the remaining WP-100 implementation tranches proceed, the handler
amendment must freeze the exact operation matrix, context/input ownership,
cancellation and late-result state, Producer subscription transaction,
registration/storage limits, workload evidence, and WP-100/WP-300/WP-400
staging. It must also repair the invalid duplicate-receiver
`PollClientBinding::poll_subscription` skeleton.

After those gates close, the remaining WP-100 implementation tranches proceed
in this order:

1. Complete the operation-specific sync, async, and bounded-step handler APIs,
   including `HandlerContext`, `CancellationView`, cancellation ownership, and
   sparse handler storage.
2. Implement `DecodedPayload`, incremental codec state, exact byte accounting,
   and one-decode validation reuse.
3. Implement security probe/commit, credential leases, generation invalidation,
   body projection over `DecodedPayload`, fail-closed outbound application, and
   secret-redacted public `Debug` behavior.
4. Remove the remaining legacy push/lock facades, complete bounded progress
   behavior, and then prove that every handler, provider, codec, and status
   callback executes outside engine locks or constrained critical sections.
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
