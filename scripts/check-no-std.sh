#!/usr/bin/env sh
# v4.0 no_std + alloc compile-check (AD16: compile-time architecture only,
# NOT a runtime test). Asserts each crate root compiles `--no-default-features`;
# the async no-std flavor checks the embassy-style target.
#
# Runtime verification of the no_std driving path is deferred with the
# zenoh-pico hardware platform (P2 §2.7, AD16).

set -eu

echo "--- no_std + alloc (bare) ---"
cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
cargo check -p clinkz-wot-discovery --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
cargo check -p clinkz-wot-codec-cbor --no-default-features

echo "--- no_std + alloc + async (embassy flavor) ---"
cargo check -p clinkz-wot-core --no-default-features --features async
cargo check -p clinkz-wot-servient --no-default-features --features async
