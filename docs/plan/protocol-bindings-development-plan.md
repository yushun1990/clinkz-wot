# Protocol Bindings Development Plan

## Parent Plan Relationship

This document is a crate-level subplan under the repository-level `PLAN.md`.
It refines the protocol binding work owned by:

- `clinkz-wot-protocol-bindings` at `protocol-bindings/core`.
- `clinkz-wot-protocol-bindings-zenoh` at
  `protocol-bindings/protocols/zenoh`.

Parent milestones covered by this subplan:

- M4: Protocol Bindings and Zenoh Binding.
- M7: Conformance and Embedded Support, only for checks and fixtures owned by
  protocol binding crates.

Parent milestones not covered by this subplan:

- M1: TD 1.1 Hardening.
- M2: Thing Model Support.
- M3: Protocol-Neutral Core.
- M5: Discovery and TDD.
- M6: Servient Runtime.

## Scope

The shared protocol binding crate owns protocol-neutral helpers for selecting
TD forms, resolving form targets, validating caller-selected affordance forms,
and reporting binding-level diagnostics.

Concrete protocol crates own protocol-specific metadata parsing, operation
planning, and transport adapter boundaries. Zenoh is the first concrete
binding, but it must remain optional and must not become a dependency of TD,
TM, or core runtime crates.

Concrete runtime backends are separate from planning crates. The zenoh planning
crate must stay independent from both the Rust `zenoh` runtime and
`zenoh-pico`; backend-specific session, I/O, and platform integration belongs
behind `ZenohTransport` implementations.

## Current Baseline

The current protocol binding crates provide:

- A shared `no_std + alloc` protocol binding utility crate.
- Affordance-level form lookup for Thing, property, action, and event forms.
- Effective operation based form selection using TD defaults.
- Content type and subprotocol criteria for form selection.
- Predicate-based protocol filtering for concrete bindings.
- Target resolution through the TD crate's `base` plus form `href` helper.
- Caller-selected affordance form validation.
- Protocol-neutral helpers for selected-form security references and scopes.
- A zenoh binding crate that recognizes `zenoh://` targets and
  `cz-zenoh:keyExpr`.
- Zenoh operation planning from WoT operations to transport-level operation
  kinds.
- First-pass zenoh extension metadata parsing for encoding, QoS, priority, and
  congestion control hints.
- Documented `cz-zenoh` extension vocabulary with stable and experimental term
  status.
- Zenoh affordance planning for Thing-level forms, bulk property and event
  operations, relative `href` targets resolved against zenoh `base`, and
  content type/subprotocol criteria.
- An injected `ZenohTransport` adapter boundary that avoids a required zenoh
  runtime dependency.
- A std-only shared zenoh transport handle so runtime binding factories can reuse
  one session, connection pool, or runtime adapter across cloned bindings.
- Runtime tests for fake transport propagation and the default no-transport
  error path.
- A documented runtime backend policy that keeps Rust `zenoh` and
  `zenoh-pico` integration out of the shared planning crate.
- A first Rust `zenoh` runtime adapter, `ZenohSessionTransport`, behind the explicit
  `runtime-zenoh` feature. The default build still has no concrete Rust
  `zenoh` dependency.
- First std runtime hardening for request/reply parameter propagation and
  broader metadata parser coverage under the explicit `runtime-zenoh` feature.
- Host subscription lifecycle state exposes the selected key expression,
  content type hint, configured reply timeout, next-sample waiting, and explicit
  undeclaration APIs.

## Current Development Sequence

The next development order is:

1. Run M4 verification checks whenever shared or zenoh binding APIs change.
2. Keep M7 no-std checks and compatibility documentation aligned with the
   current shared, zenoh planning, and optional std runtime surfaces.
3. Add opt-in real Rust `zenoh` runtime smoke tests behind the explicit
   `runtime-zenoh` feature and environment-variable gate.
4. Define the next runtime/backend acceptance target before adding another
   concrete backend increment.

Completion notes:

- PB-P0.1 and PB-P0.2 are complete for the current M4 scope.
- Shared selected-form validation now covers Thing-level forms, property forms,
  action defaults, event defaults, copied selected form values, operation
  mismatches, metadata mismatches, and forms outside the requested affordance.
- Shared diagnostics distinguish missing affordance, missing operation,
  metadata mismatch, protocol filter mismatch, target resolution failure, and
  selected-form validation failure.
- M4 verification passed for both shared and zenoh binding crates.
- The first Rust `zenoh` std backend hardening pass is complete for
  request/reply selector parameter validation, subscription lifecycle metadata,
  explicit undeclaration, and metadata mapping coverage.
- M7 no-std checks now include both protocol binding crates through
  `scripts/check-no-std.sh`.

## PB-P0: Shared Binding Utility Hardening

### PB-P0.1 Stabilize Selected Form Validation

Status: complete.

Goal: every runtime path that accepts a caller-selected form can verify that
the form belongs to the requested affordance and supports the requested
effective operation.

Work items:

- Keep validation based on TD default operation helpers.
- Preserve distinct errors for unknown affordance, unsupported operation, and
  selected form not belonging to the requested affordance.
- Add tests for Thing-level forms, property forms, action default operations,
  event default operations, and copied form values.

Acceptance criteria:

- Shared validation tests cover success and failure paths.
- Zenoh runtime invocation refuses invalid selected forms before transport
  execution.
- Error messages are stable enough for downstream runtime crates.

Completion notes:

- Added focused shared validation tests for Thing-level forms, event default
  operations, copied selected form values, operation mismatches, metadata
  mismatches, and forms that do not belong to the requested affordance.
- Existing zenoh runtime invocation keeps rejecting invalid selected forms
  before transport execution through the shared validation helper.

### PB-P0.2 Finalize Shared Diagnostics

Status: complete.

Goal: make form selection and validation failures actionable for Discovery and
Servient users without encoding protocol-specific behavior in the shared crate.

Work items:

- Audit `BindingCoreError` variants for selection, validation, and target
  resolution.
- Keep caller-filter mismatch diagnostics separate from operation mismatch
  diagnostics.
- Add tests for metadata criteria mismatches and protocol filter mismatches.

Acceptance criteria:

- Shared diagnostics distinguish missing affordance, missing operation,
  metadata mismatch, protocol filter mismatch, target resolution failure, and
  selected-form validation failure.

Completion notes:

- Added explicit `BindingCoreError` variants for metadata mismatches and caller
  filter mismatches.
- Kept unsupported operations, unknown affordances, target resolution failures,
  and selected forms outside the requested affordance as distinct variants.
- Added focused tests for metadata mismatch, protocol filter mismatch, and
  target resolution failure diagnostics.

### PB-P0.3 Add Security Metadata Helpers

Status: complete.

Goal: expose protocol-neutral helpers for form-level security and scope
metadata so runtime crates and concrete bindings do not duplicate TD traversal
logic.

Work items:

- Resolve effective security references for Thing-level and form-level
  metadata.
- Preserve protocol-neutral `security` and `scopes` semantics.
- Do not interpret concrete authentication mechanisms in the shared binding
  crate.
- Add fixtures for inherited Thing-level security and form-level overrides.

Acceptance criteria:

- Shared helpers return effective security references and scopes for a selected
  form.
- Tests cover inherited security, overridden security, and nosec forms.

Completion notes:

- Added protocol-neutral helpers for resolving effective form security
  references and form-level scopes from selected affordance forms.
- Kept authentication mechanism interpretation outside the shared binding
  crate.
- Added tests for inherited Thing-level security, form-level overrides with
  scopes, and nosec metadata.

## PB-P1: Zenoh Binding Hardening

### PB-P1.1 Document Zenoh Extension Vocabulary

Status: complete.

Goal: document the first Clinkz zenoh JSON-LD extension terms before they are
treated as stable TD authoring vocabulary.

Work items:

- Document `cz-zenoh:keyExpr` as the explicit zenoh key expression term.
- Document `cz-zenoh:encoding`, `cz-zenoh:qos`, `cz-zenoh:priority`, and
  `cz-zenoh:congestionControl` as metadata hints.
- State precedence rules between `href` and `cz-zenoh:keyExpr`.
- Mark any terms that are still experimental.

Acceptance criteria:

- Documentation explains namespace, term purpose, expected JSON type, and
  validation behavior.
- Tests continue to reject non-string and empty extension values.

Completion notes:

- Documented the `cz-zenoh` namespace, `keyExpr` target term, and metadata hint
  terms in `docs/protocol-bindings.md`.
- Marked `cz-zenoh:keyExpr` stable and encoding, QoS, priority, and congestion
  control terms as experimental hints.
- Documented `cz-zenoh:keyExpr` precedence over `zenoh://` `href` targets and
  the string/non-empty validation behavior.

### PB-P1.2 Expand Zenoh Operation Planning Coverage

Status: complete.

Goal: cover all operation families currently mapped by `ZenohOperationKind`.

Work items:

- Add focused tests for Thing-level forms.
- Add focused tests for bulk property and event operations.
- Add tests for relative `href` values resolved against a zenoh `base`.
- Add tests for forms selected by content type and subprotocol when multiple
  zenoh forms are present.

Acceptance criteria:

- Every supported operation family has at least one planning test.
- Multi-form affordance tests verify protocol filtering and metadata criteria.

Completion notes:

- Added planning tests for Thing-level forms, bulk property operations, and bulk
  event operations.
- Added coverage for relative form `href` values resolved against a zenoh
  Thing-level `base`.
- Kept existing multi-form criteria coverage for content type and subprotocol
  selection.

### PB-P1.3 Keep Runtime Integration Optional

Status: complete.

Goal: allow std deployments to attach real zenoh execution without making the
default zenoh binding crate depend on a concrete zenoh runtime.

Work items:

- Keep `ZenohTransport` as the stable adapter boundary.
- Add more fake transport tests for request payloads, parameters, and output
  propagation.
- If a concrete zenoh dependency is introduced, gate it behind an explicit
  std runtime feature.
- Avoid async runtime requirements in the default feature set.

Acceptance criteria:

- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  continues to pass.
- Default builds do not require a concrete zenoh runtime dependency.
- Std runtime integration can be omitted without changing TD/TM/core crates.

Completion notes:

- Kept concrete zenoh execution behind the injected `ZenohTransport` adapter.
- Added runtime tests for injected fake transport request/output propagation and
  the default no-transport error path.
- Confirmed the zenoh binding crate has no concrete zenoh runtime dependency in
  its default feature set.

### PB-P1.4 Add First Concrete Rust Zenoh Runtime Backend

Status: complete for the first Rust `zenoh` std backend.

Goal: support std and constrained zenoh execution without weakening the
`no_std + alloc` boundary of TD, core, shared bindings, or the zenoh planning
crate.

Work items:

- Add a std runtime backend that depends on the Rust `zenoh` crate behind a
  `std` feature or in a separate `std` runtime crate.
- Add a constrained runtime backend path for `zenoh-pico` behind its own
  feature or crate, handling C ABI, platform I/O, memory, and polling details
  outside the planning crate.
- If both backends are exposed from one crate, enforce mutually exclusive
  `zenoh` and `zenoh-pico` features with a compile-time error.
- Keep backend implementations behind `ZenohTransport` so form planning and TD
  traversal remain reusable across both runtimes.

Acceptance criteria:

- Enabling the Rust `zenoh` backend does not affect `--no-default-features`
  checks for TD, core, shared bindings, or the zenoh planning crate.
- Enabling the `zenoh-pico` backend does not introduce `std` into crates that
  claim `no_std + alloc`.
- Backend feature combinations fail clearly when incompatible features are
  enabled together.

Completion notes:

- Added `ZenohSessionTransport` behind the explicit `runtime-zenoh` feature.
- Kept the default feature set free of a concrete Rust `zenoh` dependency and
  preserved `cargo check -p clinkz-wot-protocol-bindings-zenoh
  --no-default-features`.
- Implemented first-pass std execution for put, get/request-reply, one-shot
  subscribe, and unsubscribe acknowledgement through the existing
  `ZenohTransport` trait.
- Added `ZenohSubscription` so std runtimes can keep a subscription open,
  receive multiple samples over time, and explicitly undeclare it when the
  `runtime-zenoh` feature is enabled.
- Added first-pass runtime metadata mapping for encoding, express QoS,
  priority, and congestion control hints on put and get/request-reply builders.
- Added request/reply selector parameter propagation from runtime interaction
  parameters, with validation for ambiguous zenoh selector separators.
- Expanded request/reply selector parameter coverage so runtime interaction
  parameters are appended correctly when a planned zenoh selector already
  contains query parameters, while malformed selectors with multiple parameter
  separators fail clearly.
- Exposed subscription lifecycle metadata on `ZenohSubscription`, including the
  selected key expression, content type hint, and default reply timeout used by
  `next`.
- Added runtime-feature tests for request/reply parameter selector building,
  metadata case and whitespace normalization, and unsupported priority and
  congestion control diagnostics.
- Hardened request/reply selector parameter validation so empty or blank
  runtime parameter names fail before an invalid zenoh selector is built.
- Reserved the `runtime-zenoh-pico` feature with a clear compile-time error so
  constrained runtime work cannot be enabled accidentally before a real backend
  exists.
- Left the actual `zenoh-pico` backend implementation as follow-up work once
  constrained C ABI, platform I/O, memory, and polling boundaries are designed.

### PB-P1.5 Add Shared Zenoh Transport Ownership

Status: complete.

Goal: allow Servient binding factories and std runtime adapters to reuse one
transport session, connection pool, or runtime adapter without making Servient
depend on concrete protocol types.

Work items:

- Add a cloneable shared transport handle for `ZenohTransport`
  implementations.
- Keep the shared handle behind the `std` feature so the zenoh planning crate's
  `no_std + alloc` checks continue to pass.
- Add tests that prove cloned bindings share the same underlying transport
  state.

Acceptance criteria:

- Binding factories can clone a shared transport handle into newly created
  `ZenohBinding` values.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  continues to pass.
- Servient can reuse a shared zenoh transport without adding a required zenoh
  dependency.

Completion notes:

- Added `SharedZenohTransport<T>` as a std-only `Arc<Mutex<T>>` wrapper that
  implements `ZenohTransport` by forwarding to the shared adapter.
- Added zenoh binding tests for cloned bindings reusing one underlying
  transport state.
- Added Servient integration coverage for a binding factory that clones a
  shared zenoh transport handle into each `ZenohBinding`.

### PB-P1.6 Define the Constrained Zenoh-Pico Runtime Target

Status: complete.

Goal: document the acceptance target for the future constrained `zenoh-pico`
runtime backend before implementing C ABI, platform I/O, memory, polling, or
target-specific integration code.

Work items:

- Keep the current `runtime-zenoh-pico` feature reserved with a clear
  compile-time error until a real backend exists.
- Define the required adapter boundary for constrained execution through
  `ZenohTransport` and `ZenohTransportRequest`.
- Record what platform-specific responsibilities must stay injectable rather
  than becoming TD, core, shared binding, Discovery, or Servient dependencies.
- Document the feature policy and verification checks for the future backend.

Acceptance criteria:

- The acceptance target is documented in
  `docs/zenoh-pico-runtime-target.md`.
- `runtime-zenoh` and `runtime-zenoh-pico` remain mutually exclusive.
- Enabling the reserved `runtime-zenoh-pico` feature continues to fail clearly
  until implementation starts.
- `scripts/check-reserved-features.sh` covers the reserved backend and
  incompatible backend feature diagnostics.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  continues to pass.
- `scripts/check-no-std.sh` continues to pass.

Completion notes:

- Added `docs/zenoh-pico-runtime-target.md` with the constrained backend goal,
  non-goals, adapter boundary, feature policy, acceptance criteria, and
  verification path.
- Added `scripts/check-reserved-features.sh` to lock the reserved
  `runtime-zenoh-pico` diagnostic and the concrete backend feature conflict
  diagnostic.
- Verified the regular workspace, no-std, and reserved feature checks.

### PB-P1.7 Add Opt-In Rust Zenoh Runtime Smoke Tests

Status: complete.

Goal: add focused integration coverage for `ZenohSessionTransport` against a
real Rust `zenoh` runtime without making default workspace tests depend on a
router, network port, or host-specific setup.

Work items:

- Keep real runtime tests behind the `runtime-zenoh` feature.
- Skip runtime tests unless `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1` is set.
- Allow externally managed router or peer configuration through
  `CLINKZ_WOT_ZENOH_ENDPOINT` when needed.
- Start with smoke coverage for concrete session creation, put execution,
  get/request-reply behavior with deterministic replies, and error mapping.
- Keep planner, form selection, and TD traversal coverage in the existing unit
  and fixture-driven tests.

Acceptance criteria:

- The acceptance target is documented in
  `docs/zenoh-runtime-integration-test.md`.
- `cargo test --workspace` continues to pass without a zenoh router.
- `scripts/check-no-std.sh` continues to pass.
- The opt-in runtime command is documented and does not run accidentally in
  default CI or local verification.

Completion notes:

- Added an opt-in real Rust `zenoh` smoke test gated behind the
  `runtime-zenoh` feature and `CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1`.
- The smoke test opens a concrete `zenoh::Session`, executes a planned put
  operation through `ZenohSessionTransport`, and verifies a deterministic
  request/reply path.
- `CLINKZ_WOT_ZENOH_ENDPOINT` can point the test at an externally managed
  router or peer when required.
- Default workspace tests continue to skip real runtime execution unless the
  explicit environment variable is set.

## Verification

Required checks for this subplan:

```sh
cargo fmt --check
cargo test -p clinkz-wot-protocol-bindings -p clinkz-wot-protocol-bindings-zenoh
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
```

Run broader workspace checks before moving from M4 to M5:

```sh
cargo test --workspace
cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
```
