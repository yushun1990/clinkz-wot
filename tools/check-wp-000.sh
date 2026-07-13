#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)

cargo check --locked -p clinkz-wot-foundation --no-default-features \
    --manifest-path "$root/Cargo.toml"
cargo check --locked -p clinkz-wot-foundation --no-default-features --features async \
    --manifest-path "$root/Cargo.toml"
cargo check --locked -p clinkz-wot-foundation --features std \
    --manifest-path "$root/Cargo.toml"
cargo check --locked -p clinkz-wot-foundation --example no_std_surface \
    --no-default-features --manifest-path "$root/Cargo.toml"
cargo test --locked -p clinkz-wot-foundation --no-default-features \
    --manifest-path "$root/Cargo.toml"
cargo doc --locked -p clinkz-wot-foundation --no-default-features --no-deps \
    --manifest-path "$root/Cargo.toml"

dependency_tree=$(cargo tree --locked -p clinkz-wot-foundation --edges normal \
    --prefix none --manifest-path "$root/Cargo.toml")
if [[ "$(wc -l <<<"$dependency_tree")" -ne 1 ]]; then
    printf 'WP-000 check: foundation has forbidden normal dependencies:\n%s\n' \
        "$dependency_tree" >&2
    exit 1
fi

duplicate_definitions=$(
    cd "$root"
    rg 'pub (struct|enum|trait|type) (WorkClass|WorkBudget|BudgetExceeded|ResourceProfileId|ResourceLimits|StaticResourceProfile|ResourceKind|ResourceAccount|ResourceReservation|AdmissionLedger|ClockId|MonotonicInstant|RuntimeClock|SourceTimestamp|Generation|SlotIndex|GatewayDefaultV1|DirectoryClientDefaultV1|BenchmarkStaticReferenceV1)\b' \
        --glob '*.rs' --glob '!foundation/**' . || true
)
if [[ -n "$duplicate_definitions" ]]; then
    printf 'WP-000 check: duplicate foundation public definitions:\n%s\n' \
        "$duplicate_definitions" >&2
    exit 1
fi

cargo check --locked -p clinkz-wot-td --no-default-features \
    --manifest-path "$root/Cargo.toml"
cargo check --locked -p clinkz-wot-core --no-default-features \
    --manifest-path "$root/Cargo.toml"
cargo check --locked -p clinkz-wot-core --no-default-features --features async \
    --manifest-path "$root/Cargo.toml"

echo "WP-000 check: feature matrix, tests, docs, and dependency direction valid"
