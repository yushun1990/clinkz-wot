#!/usr/bin/env sh

set -eu

cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

"$SCRIPT_DIR/check-no-std.sh"
"$SCRIPT_DIR/check-feature-matrix.sh"
"$SCRIPT_DIR/check-reserved-features.sh"
"$SCRIPT_DIR/check-td2-preview.sh"
