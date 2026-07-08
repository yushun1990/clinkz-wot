#!/usr/bin/env sh
# `no_std + alloc` compile-check against a real bare-metal target.
#
# IMPORTANT: this MUST cross-compile to a `no_std` target triple, not the host.
# Compiling `--no-default-features` against the (std) host only exercises the
# crate's own `#![no_std]` attribute — it does NOT catch std-only transitive
# dependencies (e.g. `tokio`, `getrandom`, or `time`'s `formatting` feature).
# The async no-std flavor checks the embassy-style target (P2 §2.7, AD16).
#
# Runtime verification of the no_std driving path is deferred with the
# zenoh-pico hardware platform (P2 §2.7, AD16).

set -eu

TARGET="thumbv7em-none-eabihf"

if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    echo "Installing no_std target $TARGET ..."
    rustup target add "$TARGET"
fi

echo "--- no_std + alloc (bare) ---"
cargo check -p clinkz-wot-td --no-default-features --target "$TARGET"
cargo check -p clinkz-wot-core --no-default-features --target "$TARGET"
cargo check -p clinkz-wot-protocol-bindings --no-default-features --target "$TARGET"
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features --target "$TARGET"
cargo check -p clinkz-wot-discovery --no-default-features --target "$TARGET"
cargo check -p clinkz-wot-servient --no-default-features --target "$TARGET"
cargo check -p clinkz-wot-codec-cbor --no-default-features --target "$TARGET"

echo "--- no_std + alloc + async (embassy flavor) ---"
cargo check -p clinkz-wot-core --no-default-features --features async --target "$TARGET"
cargo check -p clinkz-wot-servient --no-default-features --features async --target "$TARGET"
