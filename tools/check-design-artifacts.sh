#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:-check}

check_continuation_projection() {
    local state="$root/PROJECT_STATE.md"
    local -a observed_bases=()

    require_projection_body() {
        local heading="$1"
        awk -v heading="$heading" '
            $0 == heading { in_section = 1; next }
            in_section && /^##+ / { exit }
            in_section && $0 !~ /^[[:space:]]*$/ { found = 1 }
            END { exit found ? 0 : 1 }
        ' "$state" \
            || { echo "continuation check: $heading has no action" >&2; exit 1; }
    }

    grep -Fqx '## Continuation Projection' "$state" \
        || { echo 'continuation check: merge-stable projection heading is missing' >&2; exit 1; }
    grep -Fqx 'Projection mode: conditional remote handoff' "$state" \
        || { echo 'continuation check: conditional handoff mode is missing' >&2; exit 1; }
    grep -Fqx '### Before verified integration' "$state" \
        || { echo 'continuation check: pre-integration action is missing' >&2; exit 1; }
    grep -Fqx '### After verified integration' "$state" \
        || { echo 'continuation check: post-integration action is missing' >&2; exit 1; }
    require_projection_body '### Before verified integration'
    require_projection_body '### After verified integration'
    if grep -Fqx '## Current Objective' "$state"; then
        echo 'continuation check: unconditional Current Objective is forbidden' >&2
        exit 1
    fi

    mapfile -t observed_bases < <(
        sed -nE 's/^Observed default branch: `([0-9a-f]{40})`$/\1/p' "$state"
    )
    [[ ${#observed_bases[@]} -eq 1 ]] \
        || { echo 'continuation check: expected one exact observed default revision' >&2; exit 1; }
    git -C "$root" cat-file -e "${observed_bases[0]}^{commit}" \
        || { echo 'continuation check: observed default revision is not a local commit' >&2; exit 1; }
    git -C "$root" merge-base --is-ancestor "${observed_bases[0]}" HEAD \
        || { echo 'continuation check: observed default revision is not reachable from HEAD' >&2; exit 1; }

    echo 'continuation check: merge-stable projection and local basis valid'
}

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

check_continuation_projection

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
