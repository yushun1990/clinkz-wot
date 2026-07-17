#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)

expected_fields=(
    handler_slots_per_thing_max
    handler_slots_global_max
    handler_state_bytes_per_thing_max
    handler_state_bytes_global_max
    pending_handler_calls_per_thing_max
    pending_handler_calls_global_max
    handler_generations_per_slot_max
    handler_drain_timeout_millis_max
    handler_drain_steps_max
    producer_residual_records_global_max
    producer_residual_record_bytes_max
    producer_residual_bytes_global_max
    binding_emission_slots_per_binding_max
    binding_emission_slots_global_max
    collection_subscription_sources_per_subscription_max
    host_emission_lanes_per_binding_max
    host_emission_lanes_global_max
    pending_client_calls_per_binding_max
    pending_client_calls_per_thing_max
    pending_client_calls_global_max
    host_binding_cancel_drain_timeout_millis_max
)

mapfile -t resource_fields < <(tail -n +2 "$root/docs/resource-limits.csv" | cut -d, -f1)
if [[ "${#resource_fields[@]}" -ne 139 ]]; then
    echo "WP-100 foundation refresh check: expected 139 resource fields" >&2
    exit 1
fi
for offset in "${!expected_fields[@]}"; do
    index=$((118 + offset))
    if [[ "${resource_fields[$index]}" != "${expected_fields[$offset]}" ]]; then
        echo "WP-100 foundation refresh check: ResourceKind index $index is not ${expected_fields[$offset]}" >&2
        exit 1
    fi
done

for source_invariant in \
    'const EXPECTED_RESOURCE_LIMIT_COUNT: usize = 139;' \
    'revision v4.8 requires {EXPECTED_RESOURCE_LIMIT_COUNT} resource fields' \
    'revision v4.8 resource-limit schema'; do
    if ! grep -Fq "$source_invariant" "$root/foundation/build.rs"; then
        echo "WP-100 foundation refresh check: foundation generator is not at v4.8/139" >&2
        exit 1
    fi
done

for budget_invariant in \
    'const WORK_CLASS_COUNT: usize = 10;' \
    'HandlerSteps,' \
    'Self::HandlerSteps,'; do
    if ! grep -Fq "$budget_invariant" "$root/foundation/src/budget.rs"; then
        echo "WP-100 foundation refresh check: HandlerSteps is not appended" >&2
        exit 1
    fi
done

for pending_invariant in \
    'HandlerCall = 1 << 11,' \
    'ProducerSubscriptionSetup = 1 << 12,' \
    'ProducerSubscriptionTeardown = 1 << 13,'; do
    if ! grep -Fq "$pending_invariant" "$root/core/src/status.rs"; then
        echo "WP-100 foundation refresh check: pending-work bit ABI is incomplete" >&2
        exit 1
    fi
done

"$root/tools/check-wp-000.sh"
cargo test --locked -p clinkz-wot-foundation --all-features \
    --manifest-path "$root/Cargo.toml"
cargo test --locked -p clinkz-wot-core --all-features \
    pending_work_class_discriminants_are_exact --manifest-path "$root/Cargo.toml"

echo "WP-100 foundation refresh check: 139-field and work-class ABI refresh valid"
