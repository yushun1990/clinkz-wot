#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
export CLINKZ_WOT_REPOSITORY_ROOT="$root"
mode=${1:---candidate}
gate="$root/docs/work-packages/property-read-architecture-gate.toml"
attestation_rel="docs/audits/WP-200-property-read-producer-route-review.toml"
attestation="$root/$attestation_rel"
fixture="$root/tools/compile-contracts/wp200-property-read-producer-route"
schema="$root/tools/design-check/tests/wp200_property_read_producer_route_schema.rs"
candidate_base="b2adf0756c06cc41be5d809c33211d7c20f86aba"

fail() {
    echo "WP-200 Property Read Producer-route entry check: $*" >&2
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
                || fail "local candidate is not based on the registered base"
            candidate=${fields[0]}
        elif [[ ${#fields[@]} -eq 3 ]]; then
            [[ "${fields[1]}" == "$candidate_base" ]] \
                || fail "PR merge checkout does not use the registered base as first parent"
            candidate=${fields[2]}
            git -C "$root" diff --quiet "$candidate" "${fields[0]}" -- \
                || fail "PR merge checkout tree differs from its candidate parent"
        else
            fail "checkout is neither the exact candidate nor a two-parent PR merge"
        fi
        printf '%s\n' "$candidate"
    fi
}

verify_candidate_topology() {
    local candidate parent_count parent
    candidate=$(candidate_ref)
    [[ "$candidate" =~ ^[0-9a-f]{40}$ ]] || fail "cannot resolve immutable candidate ref"
    parent_count=$(git -C "$root" rev-list --parents -n 1 "$candidate" | awk '{print NF - 1}')
    [[ "$parent_count" -eq 1 ]] || fail "candidate is not a single-parent commit"
    parent=$(git -C "$root" rev-parse "$candidate^")
    [[ "$parent" == "$candidate_base" ]] || fail "candidate is not the single child of its registered base"

    mapfile -t actual < <(git -C "$root" diff-tree --no-commit-id --name-only -r "$candidate" | sort)
    expected=(
        "PLAN.md"
        "PROJECT_STATE.md"
        "docs/api-ownership.csv"
        "docs/artifacts.csv"
        "docs/audits/WP-200-property-read-producer-route-entry.md"
        "docs/governance.toml"
        "docs/spec/planning.md"
        "docs/spec/v5-artifact-carry-forward.toml"
        "docs/work-packages/PROPERTY-READ-ARCHITECTURE.md"
        "docs/work-packages/WP-200-planning.md"
        "docs/work-packages/index.toml"
        "docs/work-packages/property-read-architecture-gate.toml"
        "tools/check-wp200-property-read-producer-route-entry.sh"
        "tools/check-wp200-property-read-producer-route.sh"
        "tools/compile-contracts/wp200-property-read-producer-route/Cargo.lock"
        "tools/compile-contracts/wp200-property-read-producer-route/Cargo.toml"
        "tools/compile-contracts/wp200-property-read-producer-route/src/lib.rs"
        "tools/compile-contracts/wp200-property-read-producer-route/tests/producer_route.rs"
        "tools/design-check/src/main.rs"
        "tools/design-check/tests/wp200_property_read_producer_route_schema.rs"
        "workspace/0049-property-read-producer-route-planning-gap.md"
        "workspace/INDEX.org"
    )
    [[ "${actual[*]}" == "${expected[*]}" ]] || fail "candidate diff is not the exact registered 22-path topology"
}

inspect_contract() {
    for path in \
        "$fixture/Cargo.lock" \
        "$fixture/Cargo.toml" \
        "$fixture/src/lib.rs" \
        "$fixture/tests/producer_route.rs" \
        "$schema"; do
        [[ -f "$path" ]] || fail "contract source is missing: ${path#"$root/"}"
    done
    for marker in \
        "PropertyReadPlanCompiler::producer_route(" \
        "registration.compiler()" \
        "BindingArtifactRole::ProducerRoute" \
        "PrepareInput::new(" \
        "start_prepare("; do
        grep -Fq "$marker" "$fixture/src/lib.rs" \
            || fail "real handoff fixture is missing: $marker"
    done
    grep -Fq "PropertyReadPlanCompiler::producer_route(plan_id, registration.identity(), 0, 0)" \
        "$fixture/src/lib.rs" \
        || fail "planner input does not consume the complete registration identity"
    for forbidden in \
        "BindingArtifactRole::ConsumerCall" \
        "clinkz_wot_protocol_bindings" \
        "clinkz_wot_servient" \
        "select_affordance_form" \
        "select_form"; do
        if grep -FRq "$forbidden" "$fixture/Cargo.toml" "$fixture/src" "$fixture/tests"; then
            fail "contract contains excluded authority: $forbidden"
        fi
    done
}

run_prechecks() {
    bash -n "$root/tools/check-wp200-property-read-producer-route-entry.sh"
    bash -n "$root/tools/check-wp200-property-read-producer-route.sh"
    cargo fmt --check --manifest-path "$fixture/Cargo.toml"
    cargo test --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml"
    "$root/tools/check-design-requirements.sh"
    "$root/tools/check-api-ownership.sh"
    "$root/tools/check-architecture-adrs.sh"
    "$root/tools/check-resource-limits.sh"
    "$root/tools/check-v5-authority-reset-candidate.sh"
    cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- \
        check-work-packages
    "$root/tools/check-wp200-property-read-plan-slice.sh"
    "$root/tools/check-wp300-property-read-binding-slice.sh"
}

require_preimplementation_failure() {
    local output status
    set +e
    output=$("$root/tools/check-wp200-property-read-producer-route.sh" 2>&1)
    status=$?
    set -e
    if [[ $status -eq 0 ]]; then
        fail "completion check passed before implementation admission"
    fi
    if [[ $status -ne 1 ]] \
        || ! grep -Fq 'public Producer-route Property Read planner is missing' <<<"$output"; then
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
        echo "WP-200 Property Read Producer-route entry check: candidate ready for independent review"
        ;;
    --reviewed)
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        verify_candidate_topology
        inspect_contract
        run_prechecks
        require_preimplementation_failure
        echo "WP-200 Property Read Producer-route entry check: reviewed candidate remains pre-source"
        ;;
    --admission-ready)
        [[ -f "$attestation" ]] || fail "independent review attestation is missing"
        admission_base=$(git -C "$root" rev-parse HEAD)
        mapfile -t approval_changes < <(git -C "$root" diff --name-only "$admission_base")
        expected_approval_changes=(
            "PLAN.md"
            "PROJECT_STATE.md"
            "docs/audits/WP-200-property-read-producer-route-entry.md"
            "docs/spec/v5-artifact-carry-forward.toml"
            "docs/work-packages/property-read-architecture-gate.toml"
        )
        [[ "${approval_changes[*]}" == "${expected_approval_changes[*]}" ]] \
            || fail "approval diff is not the exact five-file pre-source checkpoint"
        mapfile -t untracked < <(git -C "$root" ls-files --others --exclude-standard)
        [[ ${#untracked[@]} -eq 0 ]] || fail "approval contains untracked paths: ${untracked[*]}"
        grep -Fqx "admission_base_ref = \"$admission_base\"" "$gate" \
            || fail "approval does not bind the exact current admission base"
        inspect_contract
        run_prechecks
        require_preimplementation_failure
        echo "WP-200 Property Read Producer-route entry check: implementation admission ready"
        ;;
    *)
        echo "usage: tools/check-wp200-property-read-producer-route-entry.sh [--candidate|--reviewed|--admission-ready]" >&2
        exit 2
        ;;
esac
