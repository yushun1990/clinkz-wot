#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
ownership_test="$root/foundation/tests/resource_policy_ownership.rs"

v48_additive_fields=(
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

mapfile -t resource_fields < <(tail -n +2 "$root/docs/resource-limits.csv" | cut -d, -f1)
if [[ "${#resource_fields[@]}" -lt 195 ]]; then
    echo "WP-100 foundation refresh check: active schema lost the 195-field v4.9 prefix" >&2
    exit 1
fi

expected_v48_prefix_hash=309816c7533d0a2dbde24f89329120331327fb35bb0b4f8b9c1575a638741f82
actual_v48_prefix_hash=$(sed -n '2,140p' "$root/docs/resource-limits.csv" \
    | cut -d, -f1 | sha256sum | cut -d' ' -f1)
if [[ "$actual_v48_prefix_hash" != "$expected_v48_prefix_hash" ]]; then
    echo "WP-100 foundation refresh check: the first 139 v4.8 ResourceKind fields moved" >&2
    exit 1
fi

for offset in "${!v48_additive_fields[@]}"; do
    index=$((118 + offset))
    if [[ "${resource_fields[$index]}" != "${v48_additive_fields[$offset]}" ]]; then
        echo "WP-100 foundation refresh check: ResourceKind index $index is not ${v48_additive_fields[$offset]}" >&2
        exit 1
    fi
done
for offset in "${!v49_fields[@]}"; do
    index=$((139 + offset))
    if [[ "${resource_fields[$index]}" != "${v49_fields[$offset]}" ]]; then
        echo "WP-100 foundation refresh check: ResourceKind index $index is not ${v49_fields[$offset]}" >&2
        exit 1
    fi
done

for source_invariant in \
    'const EXPECTED_RESOURCE_LIMIT_COUNT: usize = 195;' \
    'revision v4.9 requires {EXPECTED_RESOURCE_LIMIT_COUNT} resource fields' \
    'revision v4.9 resource-limit schema'; do
    if ! grep -Fq "$source_invariant" "$root/foundation/build.rs"; then
        echo "WP-100 foundation refresh check: foundation generator is not at v4.9/195" >&2
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

resource_limits_derive=$(grep -B1 -m1 '^pub struct ResourceLimits {' \
    "$root/foundation/src/resource.rs" | head -n 1)
if [[ "$resource_limits_derive" != *Clone* \
    || "$resource_limits_derive" == *Copy* ]]; then
    echo "WP-100 foundation refresh check: ResourceLimits must be Clone but not Copy" >&2
    exit 1
fi

work_budget_derive=$(grep -B1 -m1 '^pub struct WorkBudget {' \
    "$root/foundation/src/budget.rs" | head -n 1)
if [[ "$work_budget_derive" == *Clone* || "$work_budget_derive" == *Copy* ]]; then
    echo "WP-100 foundation refresh check: WorkBudget allowance remains duplicable" >&2
    exit 1
fi

for profile_invariant in \
    "const LIMITS: &'static ResourceLimits;" \
    "fn limits() -> &'static ResourceLimits"; do
    if ! grep -Fq "$profile_invariant" "$root/foundation/src/resource.rs"; then
        echo "WP-100 foundation refresh check: static profile access is not borrowed" >&2
        exit 1
    fi
done

if ! grep -Fq \
    "pub fn profiles() -> [(ResourceProfileId, &'static ResourceLimits); 3]" \
    "$root/foundation/examples/no_std_surface.rs"; then
    echo "WP-100 foundation refresh check: no_std fixture copies complete profiles" >&2
    exit 1
fi

if [[ ! -f "$ownership_test" ]]; then
    echo "WP-100 foundation refresh check: ownership compile test is missing" >&2
    exit 1
fi
for test_invariant in \
    'trait AmbiguousIfCopy' \
    'trait AmbiguousIfClone' \
    'fn resource_policy_and_budget_ownership_are_exact()' \
    'fn assert_clone<T: Clone>()' \
    'assert_clone::<ResourceLimits>();' \
    '<ResourceLimits as AmbiguousIfCopy<_>>::marker' \
    '<WorkBudget as AmbiguousIfCopy<_>>::marker' \
    '<WorkBudget as AmbiguousIfClone<_>>::marker' \
    "let _: &'static ResourceLimits = GatewayDefaultV1::LIMITS;" \
    "let _: &'static ResourceLimits = GatewayDefaultV1::limits();"; do
    if ! grep -Fq "$test_invariant" "$ownership_test"; then
        echo "WP-100 foundation refresh check: ownership compile test misses $test_invariant" >&2
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

echo "WP-100 foundation refresh check: v4.8 prefix and 195-field v4.9 ABI refresh valid"
