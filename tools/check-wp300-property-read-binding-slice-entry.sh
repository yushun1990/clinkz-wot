#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:---candidate}
attestation_rel="docs/audits/WP-300-property-read-binding-slice-review.toml"
attestation="$root/$attestation_rel"
fixture_root="$root/tools/compile-contracts/wp300-property-read-binding-slice"
schema="$root/tools/design-check/tests/wp300_property_read_binding_schema.rs"
gate_manifest="$root/docs/work-packages/property-read-architecture-gate.toml"

fail() {
    echo "WP-300 property-read binding slice entry check: $*" >&2
    exit 1
}

verify_review_basis() {
    review_attestation_ref=$(
        git -C "$root" log --diff-filter=A -1 --format=%H -- "$attestation_rel"
    )
    [[ "$review_attestation_ref" =~ ^[0-9a-f]{40}$ ]] \
        || fail "cannot resolve the immutable review-attestation commit"
    git -C "$root" merge-base --is-ancestor "$review_attestation_ref" HEAD \
        || fail "review attestation is not an ancestor of the current admission basis"
    grep -Fqx \
        "review_attestation = \"$attestation_rel\"" \
        "$gate_manifest" \
        || fail "gate manifest does not bind the review attestation path"
    grep -Fqx \
        "review_attestation_ref = \"$review_attestation_ref\"" \
        "$gate_manifest" \
        || fail "gate manifest does not bind the immutable review attestation commit"
}

inspect_contract_sources() {
    for path in \
        "$fixture_root/Cargo.lock" \
        "$fixture_root/Cargo.toml" \
        "$fixture_root/src/lib.rs" \
        "$fixture_root/tests/host.rs" \
        "$schema"; do
        [[ -f "$path" ]] || fail "contract source is missing: ${path#"$root/"}"
    done

    static_source="$fixture_root/src/lib.rs"
    host_source="$fixture_root/tests/host.rs"
    for marker in \
        "impl PollServerBinding for ManualMockBinding" \
        "type Compiler = MockCompiler" \
        "fn start_prepare(" \
        "fn start_readiness(" \
        "fn poll_accept(" \
        "fn start_response(" \
        "StaticBindingRegistrationInput::new(" \
        "StaticBindingRegistration::new("; do
        grep -Fq "$marker" "$static_source" \
            || fail "static authoring contract is missing: $marker"
    done
    for marker in \
        "impl RouteServerBinding for HostMockBinding" \
        "ReadyCall::pending_once" \
        "HostPreparedRouteGuard::new" \
        "HostActiveRouteGuard::new" \
        "HostCommittedRouteGuard::new" \
        "HostBindingRegistrationInput::new(" \
        "HostBindingRegistration::new("; do
        grep -Fq "$marker" "$host_source" \
            || fail "host authoring contract is missing: $marker"
    done
    for marker in \
        "complete_registration_rejects_mismatch_and_returns_the_author_input" \
        "static_immediate_property_read_reaches_response_and_explicit_cleanup" \
        "external_readiness_zero_budget_and_response_rejection_preserve_ownership" \
        "rejected_cleanup_transfer_returns_the_complete_work_object" \
        "host_erasure_covers_immediate_and_external_readiness_without_dispatch"; do
        grep -Fq "$marker" "$schema" \
            || fail "executable schema is missing: $marker"
    done

    for forbidden in \
        "BindingPublication" \
        "Dispatch" \
        "PollClientBinding" \
        "ProducerEmission" \
        "ReadPropertyHandler" \
        "clinkz_wot_protocol_bindings" \
        "clinkz_wot_servient" \
        "select_affordance_form" \
        "select_form"; do
        if grep -FRq "$forbidden" \
            "$fixture_root/Cargo.toml" \
            "$fixture_root/src" \
            "$fixture_root/tests"; then
            fail "authoring contract contains excluded authority: $forbidden"
        fi
    done
}

run_prechecks() {
    bash -n "$root/tools/check-wp300-property-read-binding-slice-entry.sh"
    bash -n "$root/tools/check-wp300-property-read-binding-slice.sh"
    cargo fmt --check --manifest-path "$fixture_root/Cargo.toml"
    cargo test --locked --quiet \
        --manifest-path "$root/tools/design-check/Cargo.toml"
    "$root/tools/check-design-requirements.sh"
    "$root/tools/check-api-ownership.sh"
    "$root/tools/check-architecture-adrs.sh"
    "$root/tools/check-resource-limits.sh"
    "$root/tools/check-v5-authority-reset-candidate.sh"
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-state
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-work-packages
    "$root/tools/check-wp200-property-read-plan-slice.sh"
}

require_preimplementation_failure() {
    local output status
    set +e
    output=$("$root/tools/check-wp300-property-read-binding-slice.sh" 2>&1)
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        fail "completion check passed before implementation admission"
    fi
    if [[ $status -ne 1 ]] \
        || ! grep -Fq \
            'Core Property Read binding implementation is missing' \
            <<<"$output"; then
        printf '%s\n' "$output" >&2
        fail "completion check did not stop at the expected absent-source boundary"
    fi
}

case "$mode" in
    --candidate)
        inspect_contract_sources
        [[ -f "$attestation" ]] \
            || fail "independent semantic review attestation is missing"
        verify_review_basis
        grep -Fqx \
            'admission_base_ref = "register-at-admission"' \
            "$gate_manifest" \
            || fail "review-pending gate must defer the admission base"
        run_prechecks
        require_preimplementation_failure
        echo "WP-300 property-read binding slice entry check: reviewed semantic candidate and deferred admission basis are valid"
        ;;
    --admission-ready)
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        verify_review_basis
        admission_base_ref=$(git -C "$root" rev-parse HEAD)

        mapfile -t approval_changes < <(
            git -C "$root" diff --name-only "$admission_base_ref"
        )
        expected_approval_changes=(
            "PLAN.md"
            "PROJECT_STATE.md"
            "docs/audits/WP-300-property-read-binding-slice-entry.md"
            "docs/spec/v5-artifact-carry-forward.toml"
            "docs/work-packages/property-read-architecture-gate.toml"
        )
        [[ "${approval_changes[*]}" == "${expected_approval_changes[*]}" ]] \
            || fail "approval diff is not the exact five-file pre-source checkpoint"
        mapfile -t untracked_paths < <(
            git -C "$root" ls-files --others --exclude-standard
        )
        [[ ${#untracked_paths[@]} -eq 0 ]] \
            || fail "approval contains untracked paths: ${untracked_paths[*]}"
        grep -Fqx \
            "admission_base_ref = \"$admission_base_ref\"" \
            "$gate_manifest" \
            || fail "approval does not bind the exact current admission base"

        inspect_contract_sources
        run_prechecks
        require_preimplementation_failure
        echo "WP-300 property-read binding slice entry check: implementation admission ready"
        ;;
    *)
        echo \
            "usage: tools/check-wp300-property-read-binding-slice-entry.sh [--candidate|--admission-ready]" \
            >&2
        exit 2
        ;;
esac
