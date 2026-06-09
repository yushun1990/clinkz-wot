# Zenoh-Pico Runtime Target

This document defines the acceptance target for the constrained `zenoh-pico`
runtime backend.

The current `clinkz-wot-protocol-bindings-zenoh` crate already exposes the
protocol-neutral zenoh planning surface and the `ZenohTransport` adapter
boundary. The `zenoh-pico` feature exposes a `no_std + alloc`
platform-hook backend through `ZenohPicoPlatform` and `ZenohPicoTransport`.
The real C ABI binding remains target-specific platform work.

This backend is currently deferred at the runtime-injection stage until the
target hardware platform, C ABI approach, polling model, and allocation
constraints are confirmed.

## Goal

Add constrained zenoh execution without weakening the `no_std + alloc`
boundary of TD, core, shared protocol bindings, Discovery, Servient, or the
default zenoh planning crate.

The backend must keep C ABI, platform I/O, memory ownership, polling, and
executor decisions outside TD/TM, core, and shared binding crates.

## Non-Goals

The first constrained backend increment does not need to provide:

- A mandatory global allocator choice.
- A required async runtime.
- Filesystem, socket, thread, or process APIs in embedded-ready crates.
- Remote JSON-LD context loading.
- A hard zenoh-pico dependency in the default zenoh binding build.

## Required Boundary

The backend must implement the existing `ZenohTransport` trait or a narrowly
scoped adapter that feeds into it.

The implementation should accept `ZenohTransportRequest` values produced by
the planner and must not reimplement TD traversal, affordance lookup,
operation inference, `base` plus `href` resolution, or Clinkz extension parsing.

Platform-specific pieces should be injectable:

- Session or transport handle.
- Polling or receive loop.
- Timeouts.
- Buffer ownership.
- Payload allocation strategy.
- Error mapping from zenoh-pico status values.

## Feature Policy

The `zenoh-pico` feature must remain mutually exclusive with
`zenoh`.

Enabling `zenoh-pico` must not pull `std` into crates that claim
`no_std + alloc` support.

## Acceptance Criteria

- Default zenoh binding builds still have no concrete zenoh runtime
  dependency.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  passes.
- `scripts/check-no-std.sh` passes.
- `zenoh` and `zenoh-pico` remain mutually exclusive with a
  clear diagnostic.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
  --features zenoh-pico` passes.
- `cargo test -p clinkz-wot-protocol-bindings-zenoh --features
  zenoh-pico` passes.
- `scripts/check-reserved-features.sh` confirms that `zenoh-pico`
  compiles, runs fake platform tests, and confirms that incompatible backend
  feature combinations fail with the expected diagnostic.
- The constrained backend uses shared planning outputs and does not duplicate
  form selection or target resolution logic.
- Tests cover request planning handoff, payload propagation, error mapping,
  timeout behavior, and subscription lifecycle behavior with fake platform
  hooks before requiring target hardware.

## Verification

Run the standard workspace path after each backend increment:

```sh
cargo fmt --check
cargo test --workspace
cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh-pico
scripts/check-no-std.sh
scripts/check-reserved-features.sh
```

The target-specific acceptance boundary for real C ABI integrations is defined
in `docs/zenoh-pico-c-abi-integration-target.md`.
