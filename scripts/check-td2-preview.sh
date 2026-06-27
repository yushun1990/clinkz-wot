#!/usr/bin/env sh

# Verifies the experimental TD 2.0 surface (`td2-preview`) end to end:
# data model, core trait surface, Servient runtime dispatch, and the zenoh
# binding planning layer. Default builds target strict TD 1.1, so the gated
# code and the TD 2.0 fixtures are only exercised here.
#
# This check is part of the M7 baseline (see scripts/check-m7.sh) to keep the
# gated surface compiling and tested.

set -eu

cargo check -p clinkz-wot-td --features td2-preview
cargo check -p clinkz-wot-core --features td2-preview
cargo check -p clinkz-wot-servient --features td2-preview
cargo check -p clinkz-wot-protocol-bindings-zenoh --features td2-preview

cargo test -p clinkz-wot-td --features td2-preview
cargo test -p clinkz-wot-servient --features td2-preview
cargo test -p clinkz-wot-protocol-bindings-zenoh --features td2-preview
