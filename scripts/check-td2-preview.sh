#!/usr/bin/env sh

# Verifies the experimental TD 2.0 surface (`td2-preview`) end to end. The only
# gated TD 2.0 term is `ActionAffordance.synchronous`; the full TD 1.1 `op`
# vocabulary (including `cancelaction`, `subscribeallevents`,
# `unsubscribeallevents`) is always available in default builds.
#
# Keep the gated surface compiling and tested across its consumers.

set -eu

cargo check --locked -p clinkz-wot-td --features td2-preview
cargo check --locked -p clinkz-wot-core --features td2-preview
cargo check --locked -p clinkz-wot-servient --features td2-preview
cargo check --locked -p clinkz-wot-protocol-bindings-zenoh --features td2-preview

cargo test --locked -p clinkz-wot-td --features td2-preview
cargo test --locked -p clinkz-wot-servient --features td2-preview
cargo test --locked -p clinkz-wot-protocol-bindings-zenoh --features td2-preview
