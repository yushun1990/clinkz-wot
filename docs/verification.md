# Workspace Verification

This document defines the regular verification path for `clinkz-wot`.

Use these checks before marking a milestone increment complete and before
starting a new runtime or backend increment. Crate-specific plans may add
focused checks, but they should not replace the workspace path when shared
behavior, public APIs, or embedded-ready crates change.

## Standard Workspace Checks

Run the standard Rust formatting and test checks:

```sh
cargo fmt --check
cargo test --workspace
```

Run Clippy when Rust code changes:

```sh
cargo clippy --workspace --all-targets
```

Clippy is advisory for early design work when a dependency or platform target
is still moving, but default Clippy findings should be treated as actionable
before completing a stable milestone increment.

## Embedded Checks

Run the embedded-ready crate checks whenever a crate that claims
`no_std + alloc` support changes:

```sh
scripts/check-no-std.sh
```

The script currently checks:

- `clinkz-wot-td`
- `clinkz-wot-core` (includes the inbound surface: `ServerBinding`,
  `AsyncServerBinding`, `InboundRequest`, `InboundResponse`, `EventBroker`,
  `SecurityProvider::verify`)
- `clinkz-wot-protocol-bindings`
- `clinkz-wot-protocol-bindings-zenoh` (planning layer only;
  `ZenohServerBinding` is behind the `zenoh` feature)
- `clinkz-wot-discovery`
- `clinkz-wot-servient` (sync flavor with `poll_serve_sync`)
- `clinkz-wot-core --no-default-features --features async` (async `no_std`
  flavor with `AsyncServerBinding`)
- `clinkz-wot-servient --no-default-features --features async` (async `no_std`
  flavor with `poll_serve` / `serve`)

`scripts/check-embedded.sh` is the stable alias for the embedded verification
entry point. It currently runs the same no-default-features checks.

## Focused Checks

When changing only one crate, use the focused crate tests first for fast
feedback, then run the workspace path before completing the increment:

```sh
cargo test -p clinkz-wot-td
cargo test -p clinkz-wot-core
cargo test -p clinkz-wot-protocol-bindings
cargo test -p clinkz-wot-protocol-bindings-zenoh
cargo test -p clinkz-wot-discovery
cargo test -p clinkz-wot-servient
```

For protocol binding work, keep shared and concrete binding checks together:

```sh
cargo test -p clinkz-wot-protocol-bindings -p clinkz-wot-protocol-bindings-zenoh
```

When changing the constrained zenoh-pico backend surface, run its
feature-gated fake platform tests. `scripts/check-reserved-features.sh` also
executes this test command as part of the backend feature verification path:

```sh
cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh-pico
```

Real Rust `zenoh` runtime tests are opt-in and are documented in
`docs/zenoh-runtime-integration-test.md`. They must not be required by the
default workspace test path.

The zenoh server binding has its own opt-in runtime smoke tests covering
read-property, write-property (put-listener), invoke-action, error-reply, and
unregister flows through the shared session:

```sh
CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1 \
  cargo test -p clinkz-wot-protocol-bindings-zenoh --test server_binding_smoke_test
```

These tests must also not be required by the default workspace test path.

Run backend feature checks when changing feature gates or planned backend
surfaces. This script compiles the constrained `zenoh-pico` backend,
runs its fake platform tests, and verifies that incompatible runtime backend
feature combinations fail with the expected diagnostic:

```sh
scripts/check-reserved-features.sh
```

For a full M7 baseline pass, use the aggregate check:

```sh
scripts/check-m7.sh
```

This script runs formatting, workspace tests, Clippy, embedded checks,
reserved backend feature checks, and the TD 2.0 experimental gate check in the
same order documented above.

## TD 2.0 Experimental Gate

Default builds target full TD 1.1 (see `docs/wot-compliance.md`), including the
complete TD 1.1 `op` vocabulary (`cancelaction`, `subscribeallevents`, and
`unsubscribeallevents` are TD 1.1 terms and their runtime dispatch and binding
planning are always available). The only experimental TD 2.0 surface is the
`ActionAffordance.synchronous` field, gated behind the `td2-preview` feature
and absent from default builds. It is exercised under `td2-preview`:

```sh
scripts/check-td2-preview.sh
```

This script compiles and tests `clinkz-wot-td`, `clinkz-wot-core`,
`clinkz-wot-servient`, and `clinkz-wot-protocol-bindings-zenoh` with
`--features td2-preview`, and is part of the M7 baseline so the gated surface
keeps compiling and stays covered.

## Documentation Updates

Update this document, `docs/no-std-embedded.md`, and the relevant milestone
plan when:

- A new workspace crate claims `no_std + alloc` support.
- A crate drops or changes its embedded support contract.
- A new required verification script or target is added.
- A milestone changes the compatibility baseline for TD/TM, core, bindings,
  Discovery, or Servient.
