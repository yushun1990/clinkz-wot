#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:-check}

check_continuation_projection() {
    local state="$root/PROJECT_STATE.md"
    local -a observed_bases=()
    local state_lines

    state_lines=$(wc -l <"$state")
    [[ $state_lines -le 200 ]] \
        || { echo "continuation check: PROJECT_STATE.md exceeds 200 lines ($state_lines)" >&2; exit 1; }

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

check_execution_contract() {
    local execution="$root/EXECUTION.md"
    local execution_lines
    local -a statuses=()

    execution_lines=$(wc -l <"$execution")
    [[ $execution_lines -le 200 ]] \
        || { echo "execution check: EXECUTION.md exceeds 200 lines ($execution_lines)" >&2; exit 1; }

    mapfile -t statuses < <(sed -nE 's/^Status: ([A-Z_]+)$/\1/p' "$execution")
    [[ ${#statuses[@]} -eq 1 ]] \
        || { echo 'execution check: expected one lifecycle status' >&2; exit 1; }
    case "${statuses[0]}" in
        IDLE|PLANNED|EXECUTING|REVIEW_READY|ACCEPTED|BLOCKED) ;;
        *) echo "execution check: invalid lifecycle status: ${statuses[0]}" >&2; exit 1 ;;
    esac

    for heading in \
        '## Engineering Claim' \
        '## Engineering Plan' \
        '## Plan Challenge' \
        '## Acceptance Criteria' \
        '## Executor Handoff' \
        '## Acceptance Review'; do
        grep -Fqx "$heading" "$execution" \
            || { echo "execution check: missing required slot: $heading" >&2; exit 1; }
    done

    echo 'execution check: bounded single-claim contract shape valid'
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
check_execution_contract

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
