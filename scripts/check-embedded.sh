#!/usr/bin/env sh

set -eu

cargo check -p clinkz-wot-td --no-default-features
cargo check -p clinkz-wot-core --no-default-features
cargo check -p clinkz-wot-protocol-bindings --no-default-features
cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features
cargo check -p clinkz-wot-discovery --no-default-features
cargo check -p clinkz-wot-servient --no-default-features
