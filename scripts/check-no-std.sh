#!/usr/bin/env sh

set -eu

cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
cargo check -p clinkz-wot-discovery --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
cargo check -p clinkz-wot-codec-cbor --no-default-features

# Async no_std flavor (embassy-style).
cargo check -p clinkz-wot-core --no-default-features --features async
cargo check -p clinkz-wot-servient --no-default-features --features async
