#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
module="$root/core/src/handler.rs"
fixture_root="$root/tools/compile-contracts/wp100-handler-value-primitives"
fixture_manifest="$fixture_root/Cargo.toml"
design_check_manifest="$root/tools/design-check/Cargo.toml"

for required in \
    "$fixture_manifest" \
    "$fixture_root/Cargo.lock" \
    "$fixture_root/src/lib.rs" \
    "$fixture_root/tests/semantics.rs" \
    "$fixture_root/ui/private-subscription-acceptance.rs" \
    "$fixture_root/ui/private-handler-footprint.rs" \
    "$fixture_root/ui/private-static-handler-registration.rs" \
    "$fixture_root/ui/must-use-subscription-acceptance.rs" \
    "$fixture_root/ui/must-use-handler-step.rs"; do
    if [[ ! -f "$required" ]]; then
        echo "WP-100 handler value primitives check: fixture artifact is missing: $required" >&2
        exit 1
    fi
done

# This part of the completion contract is executable before implementation:
# prove the frozen nested workspace and lockfile are internally valid without
# trying to resolve the not-yet-present Core API.
cargo metadata --locked --offline --no-deps --format-version 1 \
    --manifest-path "$fixture_manifest" >/dev/null

if [[ ! -f "$module" ]]; then
    echo "WP-100 handler value primitives check: core handler implementation is missing" >&2
    exit 1
fi

cargo run --locked --quiet --manifest-path "$design_check_manifest" -- \
    check-handler-value-primitives-source

export CARGO_TARGET_DIR="$root/target/wp100-handler-value-primitives"

cargo check --locked --manifest-path "$fixture_manifest" --no-default-features --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features async --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features std --lib
# Extra regression coverage; this does not substitute for the exact std-only cell.
cargo check --locked --manifest-path "$fixture_manifest" --all-features --lib
cargo test --locked --manifest-path "$fixture_manifest" \
    --no-default-features --test semantics

expect_ui_failure() {
    local target=$1
    local contract_type=$2
    local diagnostic=$3
    local output
    output=$(mktemp "${TMPDIR:-/tmp}/clinkz-wot-handler-ui.XXXXXX")
    if cargo check --locked --manifest-path "$fixture_manifest" \
        --no-default-features --features "$target" --bin "$target" \
        >"$output" 2>&1; then
        rm -f "$output"
        echo "WP-100 handler value primitives check: UI target unexpectedly compiled: $target" >&2
        exit 1
    fi
    if ! grep -Fq "$contract_type" "$output" || ! grep -Eq "$diagnostic" "$output"; then
        echo "WP-100 handler value primitives check: UI target failed for the wrong reason: $target" >&2
        sed -n '1,160p' "$output" >&2
        rm -f "$output"
        exit 1
    fi
    rm -f "$output"
}

expect_ui_failure \
    ui-private-subscription-acceptance SubscriptionAcceptance 'E0451|private field|fields .* private'
expect_ui_failure \
    ui-private-handler-footprint HandlerFootprint 'E0451|private field|fields .* private'
expect_ui_failure \
    ui-private-static-handler-registration StaticHandlerRegistration \
    'E0451|private field|fields .* private'
expect_ui_failure \
    ui-must-use-subscription-acceptance SubscriptionAcceptance 'unused_must_use|must be used'
expect_ui_failure \
    ui-must-use-handler-step HandlerStep 'unused_must_use|must be used'

cargo check --locked -p clinkz-wot-servient -p clinkz-wot-protocol-bindings \
    --manifest-path "$root/Cargo.toml"

echo "WP-100 handler value primitives check: five-value public contract valid"
