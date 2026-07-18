#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
schema="$root/docs/resource-limits.csv"

expected_header='field,resource_kind,unit,scope,capability_roles,zero_semantics,gateway_default_v1,directory_client_default_v1,benchmark_static_reference_v1,requirements'
if [[ "$(head -n 1 "$schema")" != "$expected_header" ]]; then
    echo "resource limit check: invalid header" >&2
    exit 1
fi

awk -F, '
    NF != 10 {
        printf "resource limit check: line %d has %d columns; expected 10\n", NR, NF > "/dev/stderr"
        bad = 1
    }
    NR > 1 {
        for (i = 1; i <= 10; i++) {
            if ($i == "") {
                printf "resource limit check: line %d has an empty field\n", NR > "/dev/stderr"
                bad = 1
            }
        }
        if ($6 != "disabled" && $6 != "rendezvous") {
            printf "resource limit check: line %d has invalid zero semantics %s\n", NR, $6 > "/dev/stderr"
            bad = 1
        }
        applicable = 0
        for (i = 7; i <= 9; i++) {
            if ($i != "NA" && $i !~ /^[0-9]+$/) {
                printf "resource limit check: line %d has invalid profile value %s\n", NR, $i > "/dev/stderr"
                bad = 1
            }
            if ($i ~ /^[0-9]+$/) {
                applicable = 1
                if ($i == 0 && $6 != "rendezvous") {
                    printf "resource limit check: line %d uses zero outside rendezvous capacity\n", NR > "/dev/stderr"
                    bad = 1
                }
            }
        }
        if (!applicable) {
            printf "resource limit check: line %d is not applicable to any named profile\n", NR > "/dev/stderr"
            bad = 1
        }
        if ($5 !~ /(^|\|)(all|directory-client)(\||$)/ && $8 != "NA") {
            printf "resource limit check: line %d assigns a non-Directory field to DirectoryClientDefaultV1\n", NR > "/dev/stderr"
            bad = 1
        }
    }
    END { exit bad }
' "$schema"

duplicates=$(tail -n +2 "$schema" | cut -d, -f1 | sort | uniq -d)
if [[ -n "$duplicates" ]]; then
    printf 'resource limit check: duplicate fields:\n%s\n' "$duplicates" >&2
    exit 1
fi

expected_field_count=195
actual_field_count=$(($(wc -l <"$schema") - 1))
if [[ "$actual_field_count" -ne "$expected_field_count" ]]; then
    echo \
        "resource limit check: expected $expected_field_count fields; found $actual_field_count" \
        >&2
    exit 1
fi

v49_fields=(
    plan_sets_per_thing_max
    plan_sets_global_max
    plan_pins_per_plan_set_max
    plan_pins_global_max
    logical_plan_bytes_per_thing_max
    binding_artifacts_per_thing_max
    binding_artifacts_global_max
    binding_artifact_bytes_per_item_max
    binding_artifact_bytes_per_thing_max
    binding_artifact_bytes_global_max
    lazy_artifact_negative_bytes_per_item_max
    lazy_artifact_negative_bytes_global_max
    binding_compiler_cursor_bytes_per_item_max
    binding_compiler_cursor_bytes_global_max
    lazy_artifact_waiters_per_slot_max
    lazy_artifact_waiters_global_max
    plan_compile_work_units_per_step_max
    plan_reclaim_bytes_per_step_max
    binding_routes_per_thing_max
    binding_routes_global_max
    route_guard_bytes_per_item_max
    route_guard_bytes_per_thing_max
    route_guard_bytes_global_max
    route_readiness_tokens_per_thing_max
    route_readiness_tokens_global_max
    route_readiness_token_bytes_per_item_max
    route_readiness_token_bytes_global_max
    route_readiness_timeout_millis_max
    route_readiness_steps_max
    binding_ingress_items_per_route_max
    binding_ingress_items_per_binding_max
    binding_ingress_items_global_max
    binding_ingress_bytes_per_route_max
    binding_ingress_bytes_per_binding_max
    binding_ingress_bytes_global_max
    host_binding_call_bytes_per_item_max
    host_binding_call_bytes_per_binding_max
    host_binding_call_bytes_per_thing_max
    host_binding_call_bytes_global_max
    host_subscription_driver_bytes_per_item_max
    host_subscription_driver_bytes_per_thing_max
    host_subscription_driver_bytes_global_max
    binding_slot_state_bytes_per_item_max
    binding_slot_state_bytes_per_thing_max
    binding_slot_state_bytes_global_max
    binding_poll_temporary_bytes_per_call_max
    binding_poll_temporary_bytes_global_max
    binding_response_buffer_bytes_per_route_max
    binding_response_buffer_bytes_global_max
    binding_cancel_buffer_bytes_per_call_max
    binding_cancel_buffer_bytes_global_max
    cleanup_transfer_slots_global_max
    cleanup_transfer_bytes_global_max
    binding_wake_leases_global_max
    binding_reactor_queue_items_per_binding_max
    binding_reactor_queue_bytes_per_binding_max
)
mapfile -t actual_v49_fields < <(tail -n +141 "$schema" | cut -d, -f1)
if [[ "${actual_v49_fields[*]}" != "${v49_fields[*]}" ]]; then
    echo "resource limit check: v4.9 append-only field order is not frozen at 139..194" >&2
    exit 1
fi

for field in "${v49_fields[@]}"; do
    if ! awk -F, -v field="$field" '
        $1 == field && $7 ~ /^[1-9][0-9]*$/ && $8 == "NA" \
            && $9 ~ /^[1-9][0-9]*$/ && $10 ~ /(^|\|)RES-LIMIT-001(\||$)/ {
            found = 1
        }
        END { exit !found }
    ' "$schema"; then
        echo "resource limit check: incomplete v4.9 projection for field: $field" >&2
        exit 1
    fi
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
tail -n +2 "$root/docs/requirements.csv" | cut -d, -f1 | tr '|' '\n' \
    >"$tmp/requirement-expressions"
: >"$tmp/requirements-unsorted"
while IFS= read -r expression; do
    if [[ "$expression" =~ ^([A-Z][A-Z0-9-]*-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
        prefix=${BASH_REMATCH[1]}
        first=$((10#${BASH_REMATCH[2]}))
        last=$((10#${BASH_REMATCH[3]}))
        for ((i = first; i <= last; i++)); do
            printf '%s%03d\n' "$prefix" "$i" >>"$tmp/requirements-unsorted"
        done
    else
        printf '%s\n' "$expression" >>"$tmp/requirements-unsorted"
    fi
done <"$tmp/requirement-expressions"
sort -u "$tmp/requirements-unsorted" >"$tmp/requirements"
tail -n +2 "$schema" | cut -d, -f10 | tr '|' '\n' | sort -u >"$tmp/referenced"
comm -23 "$tmp/referenced" "$tmp/requirements" >"$tmp/unknown"
if [[ -s "$tmp/unknown" ]]; then
    echo "resource limit check: unknown requirement references:" >&2
    sed 's/^/  /' "$tmp/unknown" >&2
    exit 1
fi

required_fields=(
    accounting_batch_items_max
    accounting_idle_items_max
    accounting_reconcile_owners_per_step_max
    admission_peak_live_bytes_global_max
    cache_entries_global_max
    cache_generations_per_key_max
    cache_reclamation_items_per_step_max
    cleanup_bytes_max
    cleanup_items_max
    cleanup_work_items_per_step_max
    directory_response_buffer_bytes_global_max
    emission_binding_results_max
    engine_live_bytes_global_max
    fanout_cursors_global_max
    fanout_subscribers_per_step_max
    peak_live_bytes_per_admission_max
    query_bytes_max
    query_nesting_depth_max
    query_nodes_max
    query_terms_max
    "${v49_fields[@]}"
)
for field in "${required_fields[@]}"; do
    if ! awk -F, -v field="$field" 'NR > 1 && $1 == field { found = 1 } END { exit !found }' \
        "$schema"; then
        echo "resource limit check: missing required field: $field" >&2
        exit 1
    fi
done

check_manifest_profile() {
    local manifest=$1
    local expected=$2
    local actual
    actual=$(sed -nE 's/^resource_profile = "([^"]+)"$/\1/p' "$manifest")
    if [[ "$actual" != "$expected" ]]; then
        echo "resource limit check: $manifest references $actual; expected $expected" >&2
        exit 1
    fi
}

check_manifest_profile "$root/docs/performance/gateway.toml" GatewayDefaultV1
check_manifest_profile "$root/docs/performance/directory.toml" DirectoryClientDefaultV1
check_manifest_profile "$root/docs/performance/constrained.toml" BenchmarkStaticReferenceV1

if grep -Eq '^\[static_profile\]$' "$root/docs/performance/constrained.toml"; then
    echo "resource limit check: constrained manifest duplicates the resource profile" >&2
    exit 1
fi

for profile in GatewayDefaultV1 DirectoryClientDefaultV1 BenchmarkStaticReferenceV1; do
    if ! awk -F, -v profile="$profile" '
        NR > 1 && $1 == profile && $2 == "profile" && $14 == "frozen" { found = 1 }
        END { exit !found }
    ' "$root/docs/api-ownership.csv"; then
        echo "resource limit check: profile is not frozen in API ownership: $profile" >&2
        exit 1
    fi
done

echo "resource limit check: $(($(wc -l <"$schema") - 1)) fields and 3 profiles"
