#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
deadline_source="$root/core/src/deadline.rs"
status_source="$root/core/src/status.rs"
root_source="$root/core/src/lib.rs"
fixture_root="$root/tools/compile-contracts/wp100-deadline-cleanup-timing"
fixture_manifest="$fixture_root/Cargo.toml"

for required in \
    "$fixture_manifest" \
    "$fixture_root/Cargo.lock" \
    "$fixture_root/src/lib.rs" \
    "$fixture_root/tests/semantics.rs" \
    "$fixture_root/ui/private-deadline-instant.rs"; do
    if [[ ! -f "$required" ]]; then
        echo "WP-100 deadline cleanup timing check: fixture artifact is missing: $required" >&2
        exit 1
    fi
done

cargo metadata --locked --offline --no-deps --format-version 1 \
    --manifest-path "$fixture_manifest" >/dev/null

if [[ ! -f "$deadline_source" ]]; then
    echo "WP-100 deadline cleanup timing check: Core Deadline implementation is missing" >&2
    exit 1
fi

for contract in \
    'pub struct Deadline' \
    'pub const NONE: Self' \
    'pub const fn at(' \
    'pub const fn instant(' \
    'pub fn checked_is_elapsed_at(' \
    'now.checked_cmp(instant)'; do
    if ! grep -Fq "$contract" "$deadline_source"; then
        echo "WP-100 deadline cleanup timing check: Deadline source misses: $contract" >&2
        exit 1
    fi
done

if ! grep -Fq 'retry_not_before.checked_cmp(deadline)' "$status_source"; then
    echo "WP-100 deadline cleanup timing check: CleanupRecord does not use checked logical ordering" >&2
    exit 1
fi
if grep -Fq 'retry_not_before.ticks() > deadline.ticks()' "$status_source"; then
    echo "WP-100 deadline cleanup timing check: raw CleanupRecord tick ordering remains" >&2
    exit 1
fi
if ! grep -Fq 'pub mod deadline;' "$root_source" \
    || ! grep -Fq 'pub use deadline::Deadline;' "$root_source"; then
    echo "WP-100 deadline cleanup timing check: Core Deadline module/root export is missing" >&2
    exit 1
fi

export CARGO_TARGET_DIR="$root/target/wp100-deadline-cleanup-timing"

cargo check --locked --manifest-path "$fixture_manifest" --no-default-features --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features async --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features std --lib
cargo test --locked --manifest-path "$fixture_manifest" \
    --no-default-features --test semantics

output=$(mktemp "${TMPDIR:-/tmp}/clinkz-wot-deadline-ui.XXXXXX")
if cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features ui-private-deadline-instant \
    --bin ui-private-deadline-instant >"$output" 2>&1; then
    rm -f "$output"
    echo "WP-100 deadline cleanup timing check: private Deadline field UI target compiled" >&2
    exit 1
fi
if ! grep -Fq 'Deadline' "$output" \
    || ! grep -Eq 'E0451|private field|fields .* private' "$output"; then
    echo "WP-100 deadline cleanup timing check: Deadline privacy target failed for the wrong reason" >&2
    sed -n '1,160p' "$output" >&2
    rm -f "$output"
    exit 1
fi
rm -f "$output"

cargo test --locked -p clinkz-wot-core --lib
"$root/tools/check-wp100-logical-time-correction.sh"

echo "WP-100 deadline cleanup timing check: Core deadline and cleanup timing contract valid"
