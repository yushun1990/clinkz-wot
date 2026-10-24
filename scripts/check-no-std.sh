#!/usr/bin/env sh
# `no_std + alloc` compile-check against a real bare-metal target.
#
# IMPORTANT: this MUST cross-compile to a `no_std` target triple, not the host.
# Compiling `--no-default-features` against the (std) host only exercises the
# crate's own `#![no_std]` attribute — it does NOT catch std-only transitive
# dependencies (e.g. `tokio`, `getrandom`, or `time`'s `formatting` feature).
# Portable compilation is distinct from protocol-backend or hardware qualification.

set -eu

TARGET="thumbv7em-none-eabihf"

if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    echo "Installing no_std target $TARGET ..."
    rustup target add "$TARGET"
fi

echo "--- no_std + alloc (bare) ---"
cargo check --locked -p clinkz-wot-td --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot-foundation --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot-core --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot-planning --no-default-features --target "$TARGET"
cargo check --locked --manifest-path planning/tests/normal-dependency/Cargo.toml --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot-discovery --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot-servient --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot-codec-cbor --no-default-features --target "$TARGET"
cargo check --locked -p clinkz-wot --no-default-features --target "$TARGET"

echo "--- no_std + alloc + async ---"
cargo check --locked -p clinkz-wot-core --no-default-features --features async --target "$TARGET"
cargo check --locked -p clinkz-wot-planning --no-default-features --features async --target "$TARGET"
cargo check --locked --manifest-path planning/tests/normal-dependency/Cargo.toml --no-default-features --features async --target "$TARGET"
cargo check --locked -p clinkz-wot-servient --no-default-features --features async --target "$TARGET"
cargo check --locked -p clinkz-wot-discovery --no-default-features --features async --target "$TARGET"

echo "--- TD preparation and Core vocabulary without pointer atomics ---"
rustup target add thumbv6m-none-eabi
cargo check --locked -p clinkz-wot-td --no-default-features --features validated-thing --target thumbv6m-none-eabi
