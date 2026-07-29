#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:---candidate}
attestation_rel="docs/audits/WP-100-property-read-handler-slice-review.toml"
attestation="$root/$attestation_rel"

fail() {
    echo "WP-100 property-read handler slice entry check: $*" >&2
    exit 1
}

run_prechecks() {
    bash -n "$root/tools/check-wp100-property-read-handler-slice-entry.sh"
    bash -n "$root/tools/check-wp100-property-read-handler-slice.sh"
    cargo test --locked --quiet \
        --manifest-path "$root/tools/design-check/Cargo.toml"
    "$root/tools/check-design-requirements.sh"
    "$root/tools/check-api-ownership.sh"
    "$root/tools/check-architecture-adrs.sh"
    "$root/tools/check-resource-limits.sh"
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-work-packages
    "$root/tools/check-wp100-amendment.sh"
    "$root/tools/check-wp100-handler-amendment.sh"
    "$root/tools/check-wp100-foundation-refresh.sh"
    "$root/tools/check-wp100-handler-value-primitives.sh"
    "$root/tools/check-wp100-logical-time-correction.sh"
    "$root/tools/check-wp100-deadline-cleanup-timing.sh"
    "$root/tools/check-wp100-handler-context.sh"
}

require_preimplementation_failure() {
    local output status
    set +e
    output=$("$root/tools/check-wp100-property-read-handler-slice.sh" 2>&1)
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        fail "completion check passed before implementation admission"
    fi
    if [[ $status -ne 1 ]] \
        || ! grep -Fq \
            'Core ReadPropertyHandler implementation is missing' \
            <<<"$output"; then
        printf '%s\n' "$output" >&2
        fail "completion check did not stop at the expected absent-trait boundary"
    fi
}

case "$mode" in
    --candidate)
        run_prechecks
        [[ ! -e "$attestation" ]] || fail "independent review attestation is premature"
        require_preimplementation_failure
        echo "WP-100 property-read handler slice entry check: candidate ready for independent review"
        ;;
    --admission-ready)
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        attestation_ref=$(git -C "$root" rev-parse HEAD)

        mapfile -t approval_changes < <(
            git -C "$root" diff --name-only "$attestation_ref"
        )
        expected_approval_changes=(
            "PLAN.md"
            "docs/audits/WP-100-property-read-handler-slice-entry.md"
            "docs/work-packages/property-read-architecture-gate.toml"
        )
        [[ "${approval_changes[*]}" == "${expected_approval_changes[*]}" ]] \
            || fail "approval diff is not the exact three-file admission checkpoint"
        mapfile -t untracked_paths < <(
            git -C "$root" ls-files --others --exclude-standard
        )
        [[ ${#untracked_paths[@]} -eq 0 ]] \
            || fail "approval contains untracked paths: ${untracked_paths[*]}"

        run_prechecks
        require_preimplementation_failure
        echo "WP-100 property-read handler slice entry check: implementation admission ready"
        ;;
    *)
        echo \
            "usage: tools/check-wp100-property-read-handler-slice-entry.sh [--candidate|--admission-ready]" \
            >&2
        exit 2
        ;;
esac
