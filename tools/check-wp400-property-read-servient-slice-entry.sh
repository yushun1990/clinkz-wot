#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:---candidate}
gate="$root/docs/work-packages/property-read-architecture-gate.toml"
attestation_rel="docs/audits/WP-400-property-read-servient-slice-review.toml"
attestation="$root/$attestation_rel"
fixture="$root/tools/compile-contracts/wp400-property-read-servient-slice"
schema="$root/tools/design-check/tests/wp400_property_read_servient_schema.rs"
original_base="fcce9e69036459506a163ac73ef5542f92e5eb7f"
original_candidate="2d63e151ac6f89ef294c089d5f48917e8e324773"
first_correction="4456632367069fb5cdd20dd51aeade1035e3768b"
candidate_base="$first_correction"

fail() {
    echo "WP-400 Property Read Servient entry check: $*" >&2
    exit 1
}

candidate_ref() {
    if [[ -f "$attestation" ]]; then
        sed -n 's/^reviewed_ref = "\([0-9a-f]\{40\}\)"$/\1/p' "$attestation"
    else
        local candidate head_record
        local -a fields
        head_record=$(git -C "$root" rev-list --parents -n 1 HEAD)
        read -r -a fields <<<"$head_record"
        if [[ ${#fields[@]} -eq 2 ]]; then
            [[ "${fields[1]}" == "$candidate_base" ]] \
                || fail "local second corrective candidate is not based on the first correction"
            candidate=${fields[0]}
        elif [[ ${#fields[@]} -eq 3 ]]; then
            git -C "$root" merge-base --is-ancestor \
                "${fields[1]}" "$candidate_base" \
                || fail "PR merge checkout first parent is not an ancestor of the second corrective base"
            candidate=${fields[2]}
            [[ "$(git -C "$root" rev-parse "$candidate^")" == "$candidate_base" ]] \
                || fail "PR merge correction is not the unique child of the first correction"
            git -C "$root" diff --quiet "$candidate" "${fields[0]}" -- \
                || fail "PR merge checkout tree differs from its candidate parent"
        else
            fail "checkout is neither the exact candidate nor a two-parent PR merge"
        fi
        printf '%s\n' "$candidate"
    fi
}

verify_candidate_topology() {
    local candidate correction_parent correction_parent_count original_parent original_parent_count parent parent_count
    original_parent_count=$(git -C "$root" rev-list --parents -n 1 "$original_candidate" | awk '{print NF - 1}')
    [[ "$original_parent_count" -eq 1 ]] \
        || fail "original candidate is not a single-parent commit"
    original_parent=$(git -C "$root" rev-parse "$original_candidate^")
    [[ "$original_parent" == "$original_base" ]] \
        || fail "original candidate is not the unique child of its default base"
    mapfile -t original_actual < <(git -C "$root" diff-tree --no-commit-id --name-only -r "$original_candidate" | sort)
    original_expected=(
        "PLAN.md"
        "PROJECT_STATE.md"
        "docs/api-ownership.csv"
        "docs/artifacts.csv"
        "docs/audits/WP-400-property-read-servient-slice-entry.md"
        "docs/governance.toml"
        "docs/spec/v5-artifact-carry-forward.toml"
        "docs/work-packages/PROPERTY-READ-ARCHITECTURE.md"
        "docs/work-packages/WP-400-servient.md"
        "docs/work-packages/property-read-architecture-gate.toml"
        "tools/check-wp400-property-read-servient-slice-entry.sh"
        "tools/check-wp400-property-read-servient-slice.sh"
        "tools/compile-contracts/wp400-property-read-servient-slice/Cargo.lock"
        "tools/compile-contracts/wp400-property-read-servient-slice/Cargo.toml"
        "tools/compile-contracts/wp400-property-read-servient-slice/src/lib.rs"
        "tools/compile-contracts/wp400-property-read-servient-slice/tests/host.rs"
        "tools/design-check/src/main.rs"
        "tools/design-check/tests/wp400_property_read_servient_schema.rs"
    )
    [[ "${original_actual[*]}" == "${original_expected[*]}" ]] \
        || fail "original candidate diff is not the registered 18-path topology"

    correction_parent_count=$(git -C "$root" rev-list --parents -n 1 "$first_correction" | awk '{print NF - 1}')
    [[ "$correction_parent_count" -eq 1 ]] \
        || fail "first correction is not a single-parent commit"
    correction_parent=$(git -C "$root" rev-parse "$first_correction^")
    [[ "$correction_parent" == "$original_candidate" ]] \
        || fail "first correction is not the unique child of the original candidate"
    mapfile -t correction_actual < <(git -C "$root" diff-tree --no-commit-id --name-only -r "$first_correction" | sort)
    correction_expected=(
        "PLAN.md"
        "PROJECT_STATE.md"
        "docs/api-ownership.csv"
        "docs/audits/WP-400-property-read-servient-slice-entry.md"
        "docs/spec/v5-artifact-carry-forward.toml"
        "docs/work-packages/property-read-architecture-gate.toml"
        "tools/check-wp400-property-read-servient-slice-entry.sh"
        "tools/check-wp400-property-read-servient-slice.sh"
        "tools/compile-contracts/wp400-property-read-servient-slice/src/lib.rs"
        "tools/design-check/src/main.rs"
        "tools/design-check/tests/wp400_property_read_servient_schema.rs"
    )
    [[ "${correction_actual[*]}" == "${correction_expected[*]}" ]] \
        || fail "first correction diff is not the exact registered 11-path topology"

    candidate=$(candidate_ref)
    [[ "$candidate" =~ ^[0-9a-f]{40}$ ]] || fail "cannot resolve immutable candidate ref"
    parent_count=$(git -C "$root" rev-list --parents -n 1 "$candidate" | awk '{print NF - 1}')
    [[ "$parent_count" -eq 1 ]] || fail "candidate is not a single-parent commit"
    parent=$(git -C "$root" rev-parse "$candidate^")
    [[ "$parent" == "$candidate_base" ]] \
        || fail "second corrective candidate is not the single child of the first correction"

    mapfile -t actual < <(git -C "$root" diff-tree --no-commit-id --name-only -r "$candidate" | sort)
    expected=(
        "PLAN.md"
        "PROJECT_STATE.md"
        "docs/audits/WP-400-property-read-servient-slice-entry.md"
        "docs/spec/v5-artifact-carry-forward.toml"
        "docs/work-packages/property-read-architecture-gate.toml"
        "tools/check-wp400-property-read-servient-slice-entry.sh"
        "tools/compile-contracts/wp400-property-read-servient-slice/tests/host.rs"
        "tools/design-check/src/main.rs"
        "tools/design-check/tests/wp400_property_read_servient_schema.rs"
    )
    [[ "${actual[*]}" == "${expected[*]}" ]] \
        || fail "second corrective candidate diff is not the exact registered 9-path topology"
}

inspect_contract() {
    for path in \
        "$fixture/Cargo.lock" \
        "$fixture/Cargo.toml" \
        "$fixture/src/lib.rs" \
        "$fixture/tests/host.rs" \
        "$schema"; do
        [[ -f "$path" ]] || fail "contract source is missing: ${path#"$root/"}"
    done

    for marker in \
        "StaticServientBuilder::new(" \
        ".binding_registration(" \
        ".read_property_handler(" \
        "servient.step(" \
        "ServientBuilder::new()" \
        ".resource_limits(limits)" \
        ".set_read_property_handler(" \
        ".begin_expose(" \
        "host_property_read_fixture" \
        "probe.enqueue_property_read"; do
        grep -FRq "$marker" "$fixture/src" "$fixture/tests" \
            || fail "runner contract is missing product-boundary marker: $marker"
    done
    if grep -Fq "ServientBuilder::new(limits)" "$fixture/src/lib.rs"; then
        fail "host fixture replaces the existing zero-argument builder constructor"
    fi
    grep -Fq "GatewayDefaultV1::LIMITS.clone()" "$fixture/tests/host.rs" \
        || fail "host fixture does not use the existing named Gateway resource profile"
    if grep -Fq "ResourceLimits::default()" "$fixture/tests/host.rs"; then
        fail "host fixture assumes a nonexistent Foundation Default implementation"
    fi

    for forbidden in \
        "BindingRouteKey::new(" \
        "PrepareInput::new(" \
        "ServingActivationAuthority::new(" \
        "RouteAcceptLease::new(" \
        "HandlerContext::try_new(" \
        "RouteInboundResponse::new(" \
        "RouteReservationIdentity::new(" \
        "CollisionDomainId::new(" \
        "EndpointReservationKey::new(" \
        "struct CompiledPlanSetRecord" \
        "struct BindingRouteRecord" \
        "struct ServingActivationRecord"; do
        if grep -FRq "$forbidden" "$fixture/src" "$fixture/tests"; then
            fail "runner contract contains fixture-owned WP-400 authority: $forbidden"
        fi
    done

    for mutation in \
        "fixture_restated_artifact_or_reservation_is_rejected" \
        "dropped_or_mismatched_generation_is_rejected" \
        "planning_or_servient_reservation_reconstruction_is_rejected" \
        "host_erasure_metadata_loss_is_rejected" \
        "unrelated_reservation_with_real_artifact_is_rejected" \
        "partial_or_bare_registration_is_rejected" \
        "binding_side_effect_before_reservations_is_rejected" \
        "host_static_semantic_divergence_is_rejected" \
        "one_argument_host_constructor_replacement_is_rejected" \
        "manifest_lockfile_transition_omission_is_rejected" \
        "nonexistent_foundation_default_assumption_is_rejected"; do
        grep -Fq "$mutation" "$schema" \
            || fail "executable schema is missing negative mutation: $mutation"
    done

    for fixture_root in \
        "$root/tools/architecture-fixtures/property-read-binding" \
        "$root/tools/architecture-fixtures/property-read-runner"; do
        [[ ! -e "$fixture_root" ]] \
            || fail "final architecture fixture root exists before WP-400 completion"
    done
}

run_prechecks() {
    bash -n "$root/tools/check-wp400-property-read-servient-slice-entry.sh"
    bash -n "$root/tools/check-wp400-property-read-servient-slice.sh"
    cargo fmt --check --manifest-path "$fixture/Cargo.toml"
    cargo test --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" \
        --test wp400_property_read_servient_schema
    "$root/tools/check-design-requirements.sh"
    "$root/tools/check-api-ownership.sh"
    "$root/tools/check-architecture-adrs.sh"
    "$root/tools/check-resource-limits.sh"
    "$root/tools/check-v5-authority-reset-candidate.sh"
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-work-packages
    "$root/tools/check-wp100-property-read-handler-slice.sh"
    "$root/tools/check-wp200-property-read-plan-slice.sh"
    "$root/tools/check-wp300-property-read-binding-slice.sh"
    "$root/tools/check-wp200-property-read-producer-route.sh"
    "$root/tools/check-wp200-property-read-route-reservation.sh"
}

require_preimplementation_failure() {
    local output status
    set +e
    output=$("$root/tools/check-wp400-property-read-servient-slice.sh" 2>&1)
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        fail "completion check passed before implementation admission"
    fi
    if [[ $status -ne 1 ]] \
        || ! grep -Fq 'Servient Planning dependency is missing' <<<"$output"; then
        printf '%s\n' "$output" >&2
        fail "completion check did not stop at the expected absent-source boundary"
    fi
}

case "$mode" in
    --candidate)
        [[ ! -e "$attestation" ]] || fail "independent review attestation is premature"
        verify_candidate_topology
        inspect_contract
        run_prechecks
        require_preimplementation_failure
        echo "WP-400 Property Read Servient entry check: candidate ready for independent review"
        ;;
    --reviewed)
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        verify_candidate_topology
        inspect_contract
        run_prechecks
        require_preimplementation_failure
        echo "WP-400 Property Read Servient entry check: reviewed candidate remains pre-source"
        ;;
    --admission-ready)
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        admission_base=$(git -C "$root" rev-parse HEAD)
        mapfile -t approval_changes < <(git -C "$root" diff --name-only "$admission_base")
        expected_approval_changes=(
            "PLAN.md"
            "PROJECT_STATE.md"
            "docs/audits/WP-400-property-read-servient-slice-entry.md"
            "docs/spec/v5-artifact-carry-forward.toml"
            "docs/work-packages/property-read-architecture-gate.toml"
        )
        [[ "${approval_changes[*]}" == "${expected_approval_changes[*]}" ]] \
            || fail "approval diff is not the exact five-file pre-source checkpoint"
        mapfile -t untracked < <(git -C "$root" ls-files --others --exclude-standard)
        [[ ${#untracked[@]} -eq 0 ]] \
            || fail "approval contains untracked paths: ${untracked[*]}"
        grep -Fqx "admission_base_ref = \"$admission_base\"" "$gate" \
            || fail "approval does not bind the exact current admission base"
        inspect_contract
        run_prechecks
        require_preimplementation_failure
        echo "WP-400 Property Read Servient entry check: implementation admission ready"
        ;;
    *)
        echo "usage: tools/check-wp400-property-read-servient-slice-entry.sh [--candidate|--reviewed|--admission-ready]" >&2
        exit 2
        ;;
esac
