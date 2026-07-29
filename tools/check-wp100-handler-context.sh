#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
handler_source="$root/core/src/handler.rs"
root_source="$root/core/src/lib.rs"
fixture_root="$root/tools/compile-contracts/wp100-handler-context"
fixture_manifest="$fixture_root/Cargo.toml"

for required in \
    "$fixture_manifest" \
    "$fixture_root/Cargo.lock" \
    "$fixture_root/src/lib.rs" \
    "$fixture_root/tests/semantics.rs" \
    "$fixture_root/ui/private-handler-context.rs" \
    "$fixture_root/ui/handler-context-not-hash.rs" \
    "$fixture_root/ui/handler-context-not-default.rs"; do
    if [[ ! -f "$required" ]]; then
        echo "WP-100 handler context check: fixture artifact is missing: $required" >&2
        exit 1
    fi
done

cargo metadata --locked --offline --no-deps --format-version 1 \
    --manifest-path "$fixture_manifest" >/dev/null

if ! grep -Fq 'pub struct HandlerContext' "$handler_source"; then
    echo "WP-100 handler context check: Core HandlerContext implementation is missing" >&2
    exit 1
fi

cargo run --locked --quiet \
    --manifest-path "$root/tools/design-check/Cargo.toml" -- \
    check-handler-context-source

if ! grep -Fq 'HandlerContext' "$root_source"; then
    echo "WP-100 handler context check: Core root re-export is missing" >&2
    exit 1
fi

export CARGO_TARGET_DIR="$root/target/wp100-handler-context"

cargo check --locked --manifest-path "$fixture_manifest" --no-default-features --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features async --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features std --lib
cargo test --locked --manifest-path "$fixture_manifest" \
    --no-default-features --test semantics

expect_ui_failure() {
    local target=$1
    local diagnostic=$2
    local output
    output=$(mktemp "${TMPDIR:-/tmp}/clinkz-wot-handler-context-ui.XXXXXX")
    if cargo check --locked --manifest-path "$fixture_manifest" \
        --no-default-features --features "$target" --bin "$target" \
        >"$output" 2>&1; then
        rm -f "$output"
        echo "WP-100 handler context check: UI target unexpectedly compiled: $target" >&2
        exit 1
    fi
    if ! grep -Fq 'HandlerContext' "$output" || ! grep -Eq "$diagnostic" "$output"; then
        echo "WP-100 handler context check: UI target failed for the wrong reason: $target" >&2
        sed -n '1,160p' "$output" >&2
        rm -f "$output"
        exit 1
    fi
    rm -f "$output"
}

expect_ui_failure \
    ui-private-handler-context \
    'E0451|private field|fields .* private'
expect_ui_failure \
    ui-handler-context-not-hash \
    'E0277|Hash.*not implemented|trait bound.*Hash'
expect_ui_failure \
    ui-handler-context-not-default \
    'E0277|Default.*not implemented|trait bound.*Default'

cargo test --locked -p clinkz-wot-core --lib
cargo check --locked -p clinkz-wot-servient -p clinkz-wot-protocol-bindings \
    --manifest-path "$root/Cargo.toml"
"$root/tools/check-wp100-handler-value-primitives.sh"

echo "WP-100 handler context check: borrowed dispatch identity contract valid"
