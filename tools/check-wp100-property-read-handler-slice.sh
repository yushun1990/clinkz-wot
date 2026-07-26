#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/tools/compile-contracts/wp100-property-read-handler-slice/Cargo.toml"

fail() {
    echo "WP-100 property-read handler slice check: $*" >&2
    exit 1
}

expect_unresolved_import() {
    local feature=$1
    local binary=$2
    local symbol=$3
    local output status

    set +e
    output=$(cargo check --locked --quiet \
        --manifest-path "$manifest" \
        --features "$feature" \
        --bin "$binary" 2>&1)
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        fail "$symbol unexpectedly entered the scoped slice"
    fi
    if ! grep -Fq "no \`$symbol\` in the root" <<<"$output"; then
        printf '%s\n' "$output" >&2
        fail "$binary did not fail at the expected unresolved-import boundary"
    fi
}

if ! grep -Fq "pub trait ReadPropertyHandler" "$root/core/src/handler.rs"; then
    fail "Core ReadPropertyHandler implementation is missing"
fi

cargo run --locked --quiet \
    --manifest-path "$root/tools/design-check/Cargo.toml" -- \
    check-property-read-handler-source
cargo run --locked --quiet \
    --manifest-path "$root/tools/design-check/Cargo.toml" -- \
    check-handler-value-primitives-source
cargo run --locked --quiet \
    --manifest-path "$root/tools/design-check/Cargo.toml" -- \
    check-handler-context-source

cargo check --locked --quiet --manifest-path "$manifest" --no-default-features
cargo check --locked --quiet --manifest-path "$manifest" --features async
cargo check --locked --quiet --manifest-path "$manifest" --features std
cargo test --locked --quiet --manifest-path "$manifest" --features std

expect_unresolved_import \
    ui-unscoped-async-read-property \
    ui-unscoped-async-read-property \
    AsyncReadPropertyHandler
expect_unresolved_import \
    ui-unscoped-step-read-property \
    ui-unscoped-step-read-property \
    StepReadPropertyHandler

cargo test --locked --quiet \
    --manifest-path "$root/core/Cargo.toml" \
    --no-default-features
cargo check --locked --quiet \
    --manifest-path "$root/servient/Cargo.toml" \
    --no-default-features
cargo check --locked --quiet \
    --manifest-path "$root/protocol-bindings/core/Cargo.toml" \
    --no-default-features

"$root/tools/check-wp100-handler-context.sh"

for fixture_root in \
    "$root/tools/architecture-fixtures/property-read-binding" \
    "$root/tools/architecture-fixtures/property-read-runner"; do
    [[ ! -e "$fixture_root" ]] \
        || fail "planned architecture fixture root exists before its owning slice is reviewed"
done

echo "WP-100 property-read handler slice check: synchronous static seam valid"
