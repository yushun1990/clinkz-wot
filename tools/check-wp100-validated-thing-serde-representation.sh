#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/tools/compile-fixtures/wp100-serde-representation/Cargo.toml"
target_root=$(mktemp -d)
trap 'rm -rf "$target_root"' EXIT

if ! grep -F 'serde_json = { version = "=1.0.149"' "$root/td/Cargo.toml" >/dev/null; then
    echo "WP-100 serde representation check: TD must pin exactly serde_json 1.0.149" >&2
    exit 1
fi

CARGO_TARGET_DIR="$target_root/supported-no-default" cargo check --locked --manifest-path "$manifest" --no-default-features
CARGO_TARGET_DIR="$target_root/supported-std" cargo check --locked --manifest-path "$manifest" --no-default-features --features std

expect_guard_failure() {
    local cell=$1
    local features=$2
    shift 2
    local log="$target_root/$cell.log"

    if CARGO_TARGET_DIR="$target_root/$cell" cargo check --locked --manifest-path "$manifest" --no-default-features --features "$features" >"$log" 2>&1
    then
        echo "WP-100 serde representation check: $cell unexpectedly compiled" >&2
        return 1
    fi

    local diagnostic
    for diagnostic in "$@"; do
        if ! grep -F "$diagnostic" "$log" >/dev/null; then
            echo "WP-100 serde representation check: $cell missed diagnostic: $diagnostic" >&2
            sed -n '1,240p' "$log" >&2
            return 1
        fi
    done
}

map_diagnostic="unsupported serde_json preserve_order/IndexMap Map representation"
number_diagnostic="unsupported serde_json arbitrary_precision/String-backed Number representation"

expect_guard_failure preserve-order std,preserve-order "$map_diagnostic"
expect_guard_failure arbitrary-precision arbitrary-precision "$number_diagnostic"
expect_guard_failure combined std,preserve-order,arbitrary-precision "$map_diagnostic" "$number_diagnostic"

echo "WP-100 serde representation check: supported cells compiled and all unified negative cells hit the named guards"
