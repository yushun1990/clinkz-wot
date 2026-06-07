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
- `clinkz-wot-core`
- `clinkz-wot-protocol-bindings`
- `clinkz-wot-protocol-bindings-zenoh`
- `clinkz-wot-discovery`
- `clinkz-wot-servient`

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

Real Rust `zenoh` runtime tests are opt-in and are documented in
`docs/zenoh-runtime-integration-test.md`. They must not be required by the
default workspace test path.

Run reserved feature checks when changing feature gates or planned backend
surfaces:

```sh
scripts/check-reserved-features.sh
```

## Documentation Updates

Update this document, `docs/no-std-embedded.md`, and the relevant milestone
plan when:

- A new workspace crate claims `no_std + alloc` support.
- A crate drops or changes its embedded support contract.
- A new required verification script or target is added.
- A milestone changes the compatibility baseline for TD/TM, core, bindings,
  Discovery, or Servient.
