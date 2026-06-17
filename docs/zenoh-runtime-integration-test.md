# Zenoh Runtime Integration Test Target

This document defines the acceptance target for real Rust `zenoh` runtime
integration tests.

The default zenoh binding tests intentionally stop at planning and injected
transport boundaries. They do not require a zenoh router. Real router coverage
belongs behind the explicit `zenoh` feature and an opt-in test gate.

This runtime path is the current active concrete backend increment for the
repository. Work on it should strengthen live Rust `zenoh` execution coverage
without changing the default workspace verification baseline.

## Goal

Add a small smoke-test path for `ZenohRuntimeTransport` against a real zenoh
runtime without making normal workspace tests depend on external processes,
network ports, or host-specific router configuration.

## Test Policy

Real runtime integration tests must be:

- Compiled only with the `zenoh` feature.
- Skipped unless an explicit environment variable opts in.
- Safe to leave out of `cargo test --workspace`.
- Documented with the required router/session setup.
- Focused on backend execution, not TD traversal or form selection already
  covered by planner tests.

The opt-in environment variable should be:

```text
CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1
```

If a test connects to an externally managed router or peer, the endpoint should
be configurable through:

```text
CLINKZ_WOT_ZENOH_ENDPOINT=tcp/127.0.0.1:7447
```

## Initial Smoke Coverage

The current smoke-test increment covers:

- Opening or receiving a concrete `zenoh::Session`.
- Executing a planned put operation through `ZenohRuntimeTransport`.
- Verifying live put-path metadata mapping for encoding, express QoS,
  priority, and congestion control on the observed `zenoh::Sample`.
- Executing a get/request-reply path where the test can provide a deterministic
  reply.
- Propagating a request payload and encoding through the live runtime so a
  queryable can observe the incoming body and request encoding.
- Verifying that request/reply output payload content types follow the live
  reply sample encoding instead of reusing the request encoding hint.
- Propagating request/reply selector parameters through the live runtime so a
  queryable can observe the final selector and appended parameters.
- Mapping a live request/reply timeout through `ZenohRuntimeTransport` into
  `CoreError::Transport` without panics.
- Executing the one-shot subscribe path used by `ZenohTransport::execute`.
- Declaring a long-lived `ZenohSubscription`, receiving multiple samples
  through repeated `next_sample` calls, and explicitly undeclaring the
  subscriber.
- Mapping a live subscription timeout through `next_timeout` into
  `CoreError::Transport` without panics.

## Non-Goals

The first runtime integration increment does not need to:

- Start or manage a router process automatically.
- Require router availability in default CI.
- Cover constrained `zenoh-pico`.
- Duplicate fake transport tests for planner handoff.
- Test every metadata mapping path against a live runtime, especially request
  metadata that the current query observer API does not expose beyond payload
  and encoding.

## Verification

The default verification path remains:

```sh
cargo fmt --check
cargo test --workspace
scripts/check-no-std.sh
scripts/check-reserved-features.sh
```

Opt-in runtime verification should use a focused command such as:

```sh
CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1 \
cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh
```

If the test requires a specific router endpoint, set
`CLINKZ_WOT_ZENOH_ENDPOINT` as well.
