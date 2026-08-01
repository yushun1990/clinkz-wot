#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
fixture="$root/tools/compile-contracts/wp200-property-read-producer-route/Cargo.toml"
source="$root/planning/src/property_read.rs"

fail() {
    echo "WP-200 Property Read Producer-route check: $*" >&2
    exit 1
}

grep -Fq "pub struct PropertyReadPlanCompiler" "$source" \
    || fail "public Producer-route Property Read planner is missing"
grep -Fq "pub struct PropertyReadBuildCursor" "$source" \
    || fail "public opaque Property Read build cursor is missing"
grep -Fq "pub const fn producer_route(" "$source" \
    || fail "reviewed Producer-route constructor is missing"
grep -Fq "PropertyReadPlanCompiler" "$root/planning/src/lib.rs" \
    || fail "Planning root does not export the Producer-route compiler"
grep -Fq "PropertyReadBuildCursor" "$root/planning/src/lib.rs" \
    || fail "Planning root does not export the build cursor"

for forbidden in \
    "pub const fn artifact_role(" \
    "pub const fn consumer_call(" \
    "clinkz_wot_protocol_bindings" \
    "clinkz_wot_servient" \
    "select_affordance_form" \
    "select_form"; do
    if grep -Fq "$forbidden" "$source"; then
        fail "implementation contains excluded authority: $forbidden"
    fi
done

export CARGO_TARGET_DIR="$root/target/wp200-property-read-producer-route"

cargo check --locked --quiet --manifest-path "$fixture" --no-default-features
cargo check --locked --quiet --manifest-path "$fixture" --features async
cargo check --locked --quiet --manifest-path "$fixture" --features std
cargo test --locked --quiet --manifest-path "$fixture" --features std

cargo test --locked --quiet --manifest-path "$root/planning/Cargo.toml" --no-default-features
cargo test --locked --quiet --manifest-path "$root/planning/Cargo.toml" --features async
cargo test --locked --quiet --manifest-path "$root/planning/Cargo.toml" --features std

"$root/tools/check-wp200-property-read-plan-slice.sh"
"$root/tools/check-wp300-property-read-binding-slice.sh"

for fixture_root in \
    "$root/tools/architecture-fixtures/property-read-binding" \
    "$root/tools/architecture-fixtures/property-read-runner"; do
    [[ ! -e "$fixture_root" ]] \
        || fail "architecture fixture root exists before WP-400 review"
done

echo "WP-200 Property Read Producer-route check: real plan-to-PrepareInput handoff valid"
