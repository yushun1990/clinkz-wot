#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:---candidate}
attestation_rel="docs/audits/WP-100-handler-value-primitives-review.toml"
attestation="$root/$attestation_rel"

fail() {
    echo "WP-100 handler value-primitives entry check: $*" >&2
    exit 1
}

run_state_check() {
    local state=$1
    cargo run --locked --quiet \
        --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-handler-value-primitives-entry-state "$state"
}

run_prechecks() {
    bash -n "$root/tools/check-wp100-handler-value-primitives-entry.sh"
    bash -n "$root/tools/check-wp100-handler-value-primitives.sh"
    cargo test --locked --quiet \
        --manifest-path "$root/tools/design-check/Cargo.toml"
    "$root/tools/check-api-ownership.sh"
    "$root/tools/check-architecture-adrs.sh"
    "$root/tools/check-resource-limits.sh"
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-work-packages
    "$root/tools/check-wp100-amendment.sh"
    "$root/tools/check-wp100-handler-amendment.sh"
}

require_preimplementation_failure() {
    local output status
    set +e
    output=$("$root/tools/check-wp100-handler-value-primitives.sh" 2>&1)
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        fail "completion check passed before implementation admission"
    fi
    if [[ $status -ne 1 ]] \
        || ! grep -Fq \
            'WP-100 handler value primitives check: core handler implementation is missing' \
            <<<"$output"; then
        printf '%s\n' "$output" >&2
        fail "completion check did not stop at the expected absent-implementation boundary"
    fi
}

case "$mode" in
    --candidate)
        run_state_check candidate
        run_prechecks
        require_preimplementation_failure
        echo "WP-100 handler value-primitives entry check: candidate ready for independent review"
        ;;
    --admission-ready)
        run_state_check admission-ready
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        attestation_ref=$(git -C "$root" rev-parse HEAD)

        mapfile -t approval_changes < <(
            git -C "$root" diff --name-only "$attestation_ref"
        )
        expected_approval_changes=(
            "PLAN.md"
            "docs/audits/WP-100-handler-value-primitives-entry.md"
            "docs/work-packages/index.toml"
        )
        [[ "${approval_changes[*]}" == "${expected_approval_changes[*]}" ]] \
            || fail "approval diff is not the exact three-file admission checkpoint"
        mapfile -t untracked_paths < <(
            git -C "$root" ls-files --others --exclude-standard
        )
        [[ ${#untracked_paths[@]} -eq 0 ]] \
            || fail "approval contains untracked paths: ${untracked_paths[*]}"

        run_prechecks
        "$root/tools/check-wp100-foundation-refresh.sh"
        require_preimplementation_failure
        echo "WP-100 handler value-primitives entry check: implementation admission ready"
        ;;
    *)
        echo "usage: tools/check-wp100-handler-value-primitives-entry.sh [--candidate|--admission-ready]" >&2
        exit 2
        ;;
esac
