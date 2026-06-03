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

## Current Baseline

The current protocol binding crates provide:

- A shared `no_std + alloc` protocol binding utility crate.
- Affordance-level form lookup for Thing, property, action, and event forms.
- Effective operation based form selection using TD defaults.
- Content type and subprotocol criteria for form selection.
- Predicate-based protocol filtering for concrete bindings.
- Target resolution through the TD crate's `base` plus form `href` helper.
- Caller-selected affordance form validation.
- A zenoh binding crate that recognizes `zenoh://` targets and
  `cz-zenoh:keyExpr`.
- Zenoh operation planning from WoT operations to transport-level operation
  kinds.
- First-pass zenoh extension metadata parsing for encoding, QoS, priority, and
  congestion control hints.
- An injected `ZenohTransport` adapter boundary that avoids a required zenoh
  runtime dependency.

## Current Development Sequence

The next development order is:

1. Stabilize shared binding validation and diagnostics before more concrete
   bindings depend on them.
2. Document the first Clinkz zenoh extension vocabulary and clearly mark any
   unstable metadata hints.
3. Expand protocol binding fixtures around multi-form affordances, bulk
   operations, Thing-level forms, security metadata, and `base` plus relative
   `href` handling.
4. Add optional host-runtime zenoh integration only behind an explicit feature
   and only after the adapter boundary is fully tested with fake transports.
5. Move to M5 Discovery once M4 exit criteria pass.

## PB-P0: Shared Binding Utility Hardening

### PB-P0.1 Stabilize Selected Form Validation

Status: in progress.

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

### PB-P0.2 Finalize Shared Diagnostics

Status: planned.

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

### PB-P0.3 Add Security Metadata Helpers

Status: planned.

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

## PB-P1: Zenoh Binding Hardening

### PB-P1.1 Document Zenoh Extension Vocabulary

Status: planned.

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

### PB-P1.2 Expand Zenoh Operation Planning Coverage

Status: planned.

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

### PB-P1.3 Keep Runtime Integration Optional

Status: planned.

Goal: allow host deployments to attach real zenoh execution without making the
default zenoh binding crate depend on a concrete zenoh runtime.

Work items:

- Keep `ZenohTransport` as the stable adapter boundary.
- Add more fake transport tests for request payloads, parameters, and output
  propagation.
- If a concrete zenoh dependency is introduced, gate it behind an explicit
  host/runtime feature.
- Avoid async runtime requirements in the default feature set.

Acceptance criteria:

- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  continues to pass.
- Default builds do not require a concrete zenoh runtime dependency.
- Host-runtime integration can be omitted without changing TD/TM/core crates.

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
