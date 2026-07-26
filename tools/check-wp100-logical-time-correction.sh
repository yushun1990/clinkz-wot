#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
source_file="$root/foundation/src/time.rs"
fixture_root="$root/tools/compile-contracts/wp100-logical-time-correction"
fixture_manifest="$fixture_root/Cargo.toml"
historical_evidence="$root/docs/evidence/WP-000.toml"
expected_historical_blob="5ef6eecbf4c28d26b7ddcc19afef52a791e6e935"

for required in \
    "$fixture_manifest" \
    "$fixture_root/Cargo.lock" \
    "$fixture_root/src/lib.rs" \
    "$fixture_root/tests/semantics.rs"; do
    if [[ ! -f "$required" ]]; then
        echo "WP-100 logical time correction check: fixture artifact is missing: $required" >&2
        exit 1
    fi
done

cargo metadata --locked --offline --no-deps --format-version 1 \
    --manifest-path "$fixture_manifest" >/dev/null

if [[ $(grep -Fc \
    'pub fn checked_cmp(self, other: Self) -> Option<Ordering>' \
    "$source_file") -ne 2 ]]; then
    echo \
        "WP-100 logical time correction check: Foundation SourceTimestamp::checked_cmp is missing" \
        >&2
    exit 1
fi

for contract in \
    'extended logical tick' \
    'raw source' \
    'diagnostic metadata' \
    'does not select a comparison algorithm' \
    'checked_cmp(self, other: Self)'; do
    if ! grep -Fq "$contract" "$source_file"; then
        echo "WP-100 logical time correction check: source contract misses: $contract" >&2
        exit 1
    fi
done

for obsolete in \
    'Returns the raw tick value.' \
    'Returns the finite wrap period when the clock wraps.' \
    'admitted operation lifetimes can treat'; do
    if grep -Fq "$obsolete" "$source_file"; then
        echo "WP-100 logical time correction check: obsolete raw-wrap contract remains: $obsolete" >&2
        exit 1
    fi
done

if [[ $(git -C "$root" hash-object "$historical_evidence") != "$expected_historical_blob" ]]; then
    echo "WP-100 logical time correction check: historical WP-000 evidence was rewritten" >&2
    exit 1
fi

export CARGO_TARGET_DIR="$root/target/wp100-logical-time-correction"

cargo check --locked --manifest-path "$fixture_manifest" --no-default-features --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features async --lib
cargo check --locked --manifest-path "$fixture_manifest" \
    --no-default-features --features std --lib
cargo test --locked --manifest-path "$fixture_manifest" \
    --no-default-features --test semantics

"$root/tools/check-wp-000.sh"

echo "WP-100 logical time correction check: extended logical time contract valid"
