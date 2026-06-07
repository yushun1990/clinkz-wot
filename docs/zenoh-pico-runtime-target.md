# Zenoh-Pico Runtime Target

This document defines the acceptance target for the future constrained
`zenoh-pico` runtime backend.

The current `clinkz-wot-protocol-bindings-zenoh` crate already exposes the
protocol-neutral zenoh planning surface and the `ZenohTransport` adapter
boundary. The `runtime-zenoh-pico` feature is reserved and intentionally fails
to compile until a real backend is implemented.

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

The `runtime-zenoh-pico` feature must remain mutually exclusive with
`runtime-zenoh`.

Before the backend is implemented, enabling `runtime-zenoh-pico` must fail with
a clear compile-time error. After implementation, enabling it must not pull
`std` into crates that claim `no_std + alloc` support.

## Acceptance Criteria

- Default zenoh binding builds still have no concrete zenoh runtime
  dependency.
- `cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features`
  passes.
- `scripts/check-no-std.sh` passes.
- `runtime-zenoh` and `runtime-zenoh-pico` remain mutually exclusive with a
  clear diagnostic.
- `scripts/check-reserved-features.sh` confirms that the reserved
  `runtime-zenoh-pico` feature and incompatible backend feature combination
  fail with the expected diagnostics until implementation starts.
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
scripts/check-no-std.sh
scripts/check-reserved-features.sh
```

When the real backend exists, add a focused feature check for the constrained
backend and document any target-specific toolchain requirements here.
