# Zenoh-Pico C ABI Integration Target

This document defines the acceptance target for target-specific integrations
that connect the `runtime-zenoh-pico` platform-hook backend to a real
`zenoh-pico` C ABI.

The existing `clinkz-wot-protocol-bindings-zenoh` crate owns TD-driven zenoh
planning and the `ZenohPicoPlatform` hook contract. Target-specific crates or
applications own the concrete C ABI binding, session lifecycle, polling,
allocator choice, and buffer ownership.

## Goal

Provide a repeatable integration pattern for constrained targets without
turning the zenoh planning crate into a hardware, OS, or C toolchain owner.

The first real C ABI increment should prove that a target-specific platform
hook can execute planner-produced `ZenohPicoRequest` values through zenoh-pico
for put, query or request/reply, subscribe, and unsubscribe paths.

## Non-Goals

The first C ABI increment does not need to provide:

- A repository-wide mandatory zenoh-pico crate or bindgen workflow.
- A required global allocator or executor.
- A portable guarantee that every MCU or RTOS target is supported.
- Runtime loading of JSON-LD contexts.
- TD traversal, form selection, target resolution, or Clinkz extension parsing
  outside the existing planner.
- Default workspace tests that require target hardware or a zenoh router.

## Ownership Boundary

The platform integration owns:

- The real zenoh-pico headers, libraries, and link configuration.
- Session creation, lease configuration, open and close behavior.
- Polling or receive-loop scheduling.
- Mapping `ZenohPicoRequest` key expressions, payloads, metadata, parameters,
  and timeouts into C ABI calls.
- Owned or borrowed payload buffers returned from query and subscription paths.
- Conversion from zenoh-pico status values into `ZenohPicoError`.

The `clinkz-wot-protocol-bindings-zenoh` crate continues to own:

- WoT operation to zenoh operation planning.
- `base` plus relative form `href` target resolution.
- `cz-zenoh:*` metadata parsing.
- The `ZenohPicoPlatform` trait and `ZenohPicoTransport` adapter.
- The `ZenohPicoRequest` helper methods that expose a validated selector or key
  expression target string for the selected zenoh operation.
- Feature-gate checks that keep `runtime-zenoh` and `runtime-zenoh-pico`
  mutually exclusive.

## Feature And Crate Policy

Prefer a target-specific crate or application module for the real C ABI
implementation. If a reusable repository crate is added later, it must remain
optional and must not become a default dependency of TD, core, shared protocol
bindings, Discovery, Servient, or the zenoh planning crate.

Any reusable C ABI crate must document:

- Supported target triples and C toolchain requirements.
- Required zenoh-pico version or commit.
- Linker inputs and build flags.
- Allocator and buffer ownership rules.
- Whether it requires `std`, an RTOS, or bare-metal hooks.

## Acceptance Criteria

- A real platform hook implements `ZenohPicoPlatform` without duplicating TD
  traversal, affordance lookup, operation inference, target resolution, or
  Clinkz extension parsing.
- Put, query or request/reply, subscribe, and unsubscribe paths consume
  `ZenohPicoRequest` values produced by `ZenohPicoTransport`.
- Payload content type and body bytes are preserved through put and reply
  paths.
- Request parameters and timeout values are either mapped to supported
  zenoh-pico behavior or rejected with a clear `ZenohPicoError`.
- Target-side request validation errors such as unsupported selector
  parameters, unsupported timeout modes, or missing target prerequisites use
  `ZenohPicoError::invalid_request` instead of being reported as platform
  status-code failures.
- Zenoh-pico status codes are mapped to `ZenohPicoError::with_code`, and
  timeout paths are mapped to `ZenohPicoError::timeout` when the platform can
  distinguish timeout from other platform failures.
- Subscription lifecycle behavior is explicit: the integration documents
  whether subscribe waits for one sample, keeps a declared subscriber alive, or
  requires a separate polling loop.
- Default workspace tests continue to pass without target hardware, a C
  toolchain, or a zenoh router.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
  --features runtime-zenoh-pico` continues to pass.
- `scripts/check-no-std.sh` and `scripts/check-reserved-features.sh` continue
  to pass.

## Verification

Run the repository checks after changes to the shared platform-hook surface:

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets
scripts/check-no-std.sh
scripts/check-reserved-features.sh
```

Run target-specific checks in the crate or application that owns the real C ABI
integration. Those checks should be documented with the target triple, linker
inputs, and any required zenoh router or peer setup.
