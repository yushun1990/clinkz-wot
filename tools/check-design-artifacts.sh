#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:-check}

case "$mode" in
    check)
        readiness_command=""
        ;;
    --refactor-ready)
        readiness_command="check-refactor-ready"
        ;;
    --handler-entry-ready)
        readiness_command="check-handler-entry"
        ;;
    *)
        echo "usage: tools/check-design-artifacts.sh [--refactor-ready|--handler-entry-ready]" >&2
        exit 2
        ;;
esac

if grep -Fqx 'status = "active"' \
    "$root/docs/spec/v5-authority-reset.toml"; then
    if [[ -n "$readiness_command" ]]; then
        cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
            "$readiness_command"
    fi

    "$root/tools/check-v5-authority-reset-candidate.sh"
    "$root/tools/check-design-requirements.sh"
    "$root/tools/check-api-ownership.sh"
    "$root/tools/check-architecture-adrs.sh"
    "$root/tools/check-directory-client-scope.sh"
    "$root/tools/check-resource-limits.sh"
    "$root/tools/check-wp100-amendment.sh"
    "$root/tools/check-wp100-handler-amendment.sh"
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-state
    cargo run --locked --quiet --manifest-path "$root/tools/performance-harness/Cargo.toml" -- \
        verify
    "$root/tools/check-wp-000.sh"
    "$root/tools/check-wp100-foundation-refresh.sh"
    "$root/tools/check-wp100-handler-value-primitives.sh"
    "$root/tools/check-wp100-logical-time-correction.sh"
    "$root/tools/check-wp100-deadline-cleanup-timing.sh"
    "$root/tools/check-wp100-handler-context.sh"
    "$root/tools/check-wp100-property-read-handler-slice.sh"
    echo "design artifact check: active v5 authority and all completed implementation evidence validated"
    exit 0
fi

if [[ -n "$readiness_command" ]]; then
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        "$readiness_command"
fi

"$root/tools/check-design-requirements.sh"
"$root/tools/check-v5-authority-reset-decision.sh"
"$root/tools/check-api-ownership.sh"
"$root/tools/check-architecture-adrs.sh"
"$root/tools/check-directory-client-scope.sh"
"$root/tools/check-resource-limits.sh"
"$root/tools/check-wp100-amendment.sh"
"$root/tools/check-wp100-handler-amendment.sh"
cargo run --locked --quiet --manifest-path "$root/tools/performance-harness/Cargo.toml" -- verify
cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- check

cargo test --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml"

echo "design artifact check: governance and six refactor gates validated"
