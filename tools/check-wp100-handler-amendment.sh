#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
amendment="$root/docs/amendments/WP-100-handler-api-v1.md"
ownership="$root/docs/api-ownership.csv"
states="$root/docs/state-machines.toml"
limits="$root/docs/resource-limits.csv"
dag="$root/docs/work-packages/index.toml"
gateway="$root/docs/performance/gateway.toml"
constrained="$root/docs/performance/constrained.toml"
fixtures="$root/docs/performance/fixtures.lock.toml"
wp000="$root/docs/work-packages/WP-000-foundation.md"
wp100="$root/docs/work-packages/WP-100-core.md"
wp600="$root/docs/work-packages/WP-600-protocol-bindings.md"
artifacts="$root/docs/artifacts.csv"
compile_fixture="$root/tools/design-check/tests/handler_api_schema.rs"

workload_block() {
    local file=$1 id=$2
    awk -v id="$id" '
        /^\[\[workload\]\]$/ {
            if (capture) { exit }
            next
        }
        $0 == "id = \"" id "\"" { capture = 1 }
        capture { print }
    ' "$file"
}

fixture_block() {
    local id=$1
    awk -v id="$id" '
        /^\[\[fixture\]\]$/ {
            if (capture) { exit }
            next
        }
        $0 == "id = \"" id "\"" { capture = 1 }
        capture { print }
    ' "$fixtures"
}

require_workload_text() {
    local file=$1 id=$2 expected=$3
    local block
    block=$(workload_block "$file" "$id")
    if ! grep -Fq "$expected" <<<"$block"; then
        echo "WP-100 handler amendment check: $id misses: $expected" >&2
        exit 1
    fi
}

for metadata in \
    'Status: Frozen' \
    'Base design revision: v4.7' \
    'Amendment id: WP-100-HANDLER-API-001'; do
    if ! grep -Fq "$metadata" "$amendment"; then
        echo "WP-100 handler amendment check: missing metadata: $metadata" >&2
        exit 1
    fi
done

for signature in \
    'pub struct Deadline {' \
    'pub enum CancellationView {' \
    'pub struct InteractionInput {' \
    'pub struct HandlerContext' \
    'pub struct SubscriptionAcceptance {' \
    'pub struct HandlerFootprint {' \
    'pub enum HandlerStep<R> {' \
    'pub enum PendingWorkClass {' \
    "pub struct StaticHandlerRegistration<'h, H> {" \
    "type Future<'a>: core::future::Future" \
    "pub(crate) type HostHandlerFuture<'a, R>" \
    'pub(crate) struct HostAsyncAdapter<H>' \
    'fn handle(' \
    'fn start(' \
    'fn step(' \
    'fn cancel('; do
    if ! grep -Fq "$signature" "$amendment"; then
        echo "WP-100 handler amendment check: missing frozen signature: $signature" >&2
        exit 1
    fi
done

if ! grep -Fq '#[repr(u16)]' "$amendment" \
    || ! grep -Fq '#[repr(u16)]' "$compile_fixture"; then
    echo "WP-100 handler amendment check: PendingWorkClass must retain repr(u16)" >&2
    exit 1
fi

for discriminant in \
    'HandlerCall = 1 << 11,' \
    'ProducerSubscriptionSetup = 1 << 12,' \
    'ProducerSubscriptionTeardown = 1 << 13,'; do
    if ! grep -Fq "$discriminant" "$amendment"; then
        echo "WP-100 handler amendment check: missing pending-work discriminant: $discriminant" >&2
        exit 1
    fi
    if ! grep -Fq "$discriminant" "$compile_fixture"; then
        echo \
            "WP-100 handler amendment check: compile fixture misses discriminant: $discriminant" \
            >&2
        exit 1
    fi
done

for assignment in \
    'HandlerCall = 1 << 11' \
    'ProducerSubscriptionSetup = 1 << 12' \
    'ProducerSubscriptionTeardown = 1 << 13'; do
    if ! grep -Fq "$assignment" "$wp100"; then
        echo "WP-100 handler amendment check: work package misses bit assignment: $assignment" >&2
        exit 1
    fi
done

host_hrtb="for<'a> <H as AsyncReadPropertyHandler>::Future<'a>: Send"
if ! grep -Fq "$host_hrtb" "$amendment"; then
    echo "WP-100 handler amendment check: host setter misses exact Send HRTB" >&2
    exit 1
fi

fixture_host_setter=$(awk '
    /fn set_async_read_property_handler</ { capture = 1 }
    capture { print }
    capture && /^}$/ { exit }
' "$compile_fixture")
if ! grep -Fq "$host_hrtb" <<<"$fixture_host_setter"; then
    echo "WP-100 handler amendment check: compile fixture host setter misses exact Send HRTB" >&2
    exit 1
fi

operations=(
    ReadProperty WriteProperty ObserveProperty UnobserveProperty
    InvokeAction QueryAction CancelAction SubscribeEvent UnsubscribeEvent
    ReadAllProperties WriteAllProperties ReadMultipleProperties
    WriteMultipleProperties ObserveAllProperties UnobserveAllProperties
    QueryAllActions SubscribeAllEvents UnsubscribeAllEvents
)
stems=(
    read_property write_property observe_property unobserve_property
    invoke_action query_action cancel_action subscribe_event unsubscribe_event
    read_all_properties write_all_properties read_multiple_properties
    write_multiple_properties observe_all_properties unobserve_all_properties
    query_all_actions subscribe_all_events unsubscribe_all_events
)

for operation in "${operations[@]}"; do
    for item in \
        "${operation}Handler" \
        "Async${operation}Handler" \
        "Step${operation}Handler"; do
        if ! awk -F, -v item="$item" '
            NR > 1 && $1 == item && $3 == "clinkz-wot-core" \
                && $4 == "handler" && $5 == "public" && $14 == "frozen" {
                found = 1
            }
            END { exit !found }
        ' "$ownership"; then
            echo "WP-100 handler amendment check: handler trait is not frozen: $item" >&2
            exit 1
        fi
    done
done

for item in Deadline CancellationView HandlerContext SubscriptionAcceptance \
    HandlerFootprint HandlerStep StaticHandlerRegistration; do
    if ! awk -F, -v item="$item" '
        NR > 1 && $1 == item && $3 == "clinkz-wot-core" && $14 == "frozen" {
            found = 1
        }
        END { exit !found }
    ' "$ownership"; then
        echo "WP-100 handler amendment check: value ownership is not frozen: $item" >&2
        exit 1
    fi
done

for stem in "${stems[@]}"; do
    for method in \
        "set_${stem}_handler" \
        "set_async_${stem}_handler" \
        "set_step_${stem}_handler" \
        "clear_${stem}_handler"; do
        expected_path="clinkz_wot_servient::ExposedThingHandle::${method}"
        if ! awk -F, -v item="$method" -v path="$expected_path" '
            NR > 1 && $1 == item && $3 == "clinkz-wot-servient" \
                && $5 == "public" && $6 == path && $14 == "frozen" { found = 1 }
            END { exit !found }
        ' "$ownership"; then
            echo "WP-100 handler amendment check: host method is not frozen: $method" >&2
            exit 1
        fi
    done
done

if ! awk -F, '
    NR > 1 && $1 == "clear_handlers" \
        && $6 == "clinkz_wot_servient::ExposedThingHandle::clear_handlers" \
        && $14 == "frozen" { found = 1 }
    END { exit !found }
' "$ownership"; then
    echo "WP-100 handler amendment check: clear_handlers is not frozen" >&2
    exit 1
fi

for internal in SelectedHandlerEntry HandlerCallOwner HandlerResultSink \
    CallbackLease ProducerSubscriptionOwner HandlerCleanupOwner HostAsyncAdapter \
    HostStepAdapter; do
    if ! awk -F, -v item="$internal" '
        NR > 1 && $1 == item && $3 == "clinkz-wot-core" \
            && $5 == "crate" && $6 == "-" && $14 == "frozen" { found = 1 }
        END { exit !found }
    ' "$ownership"; then
        echo "WP-100 handler amendment check: internal owner is not frozen: $internal" >&2
        exit 1
    fi
done

if ! awk -F, '
    NR > 1 && $1 == "HostHandlerFuture" && $3 == "clinkz-wot-core" \
        && $5 == "crate" && $6 == "-" && $7 == "std-async" \
        && $14 == "frozen" { found = 1 }
    END { exit !found }
' "$ownership"; then
    echo "WP-100 handler amendment check: HostHandlerFuture ownership is not frozen" >&2
    exit 1
fi

if awk -F, 'NR > 1 && $1 == "HandlerFuture" { found = 1 } END { exit !found }' \
    "$ownership"; then
    echo "WP-100 handler amendment check: obsolete public HandlerFuture remains frozen" >&2
    exit 1
fi

for legacy in PushFn PublisherSink SubscriptionSender ReadSlot WriteSlot \
    ObserveSlot UnobserveSlot InvokeSlot QuerySlot CancelSlot SubscribeSlot \
    UnsubscribeSlot; do
    if ! awk -F, -v item="$legacy" '
        NR > 1 && $1 == item && $13 == "remove" && $14 == "removed" { found = 1 }
        END { exit !found }
    ' "$ownership"; then
        echo "WP-100 handler amendment check: legacy removal is not frozen: $legacy" >&2
        exit 1
    fi
done

check_limit() {
    local field=$1 domain=$2 gateway=$3 directory=$4 constrained=$5
    if ! awk -F, -v field="$field" -v domain="$domain" -v gateway="$gateway" \
        -v directory="$directory" -v constrained="$constrained" '
        NR > 1 && $1 == field && $2 == domain && $6 == "disabled" \
            && $7 == gateway && $8 == directory && $9 == constrained { found = 1 }
        END { exit !found }
    ' "$limits"; then
        echo "WP-100 handler amendment check: resource limit mismatch: $field" >&2
        exit 1
    fi
}

check_limit handler_slots_per_thing_max handler 4105 NA 256
check_limit handler_slots_global_max handler 262144 NA 256
check_limit handler_state_bytes_per_thing_max handler 4194304 NA 65536
check_limit handler_state_bytes_global_max handler 268435456 NA 262144
check_limit pending_handler_calls_per_thing_max handler 1024 NA 32
check_limit pending_handler_calls_global_max handler 65536 NA 256
check_limit handler_generations_per_slot_max handler 2 NA 2
check_limit handler_drain_timeout_millis_max handler 5000 NA NA
check_limit handler_drain_steps_max handler 1024 NA 64
check_limit producer_residual_records_global_max cleanup 65536 NA 256
check_limit producer_residual_record_bytes_max cleanup 256 NA 128
check_limit producer_residual_bytes_global_max cleanup 16777216 NA 32768

if [[ $(($(wc -l <"$limits") - 1)) -ne 139 ]]; then
    echo "WP-100 handler amendment check: active resource schema must contain 139 fields" >&2
    exit 1
fi
expected_additive_order=$(cat <<'EOF'
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
EOF
)
actual_additive_order=$(awk -F, 'NR > 1 { ordinal = NR - 2; if (ordinal >= 118 && ordinal <= 129) print $1 }' "$limits")
if [[ "$actual_additive_order" != "$expected_additive_order" ]]; then
    echo "WP-100 handler amendment check: additive ResourceKind order is not 118..129" >&2
    exit 1
fi
for append_invariant in \
    'The 118 pre-v4.7 fields therefore retain indices `0..=117`' \
    '`118..=129`' \
    'append after `producer_residual_bytes_global_max`'; do
    if ! grep -Fq "$append_invariant" "$amendment"; then
        echo "WP-100 handler amendment check: missing append-only ABI rule: $append_invariant" >&2
        exit 1
    fi
done

for machine in handler-sync-execution handler-async-execution \
    handler-step-execution producer-subscription; do
    if ! grep -Fq "id = \"$machine\"" "$states"; then
        echo "WP-100 handler amendment check: missing state machine: $machine" >&2
        exit 1
    fi
done

for invariant in \
    'released_semantics = "Released closes response delivery and capacity ownership but does not imply that user handler code' \
    'transition_contract = "exact"' \
    'terminal_replay_semantics = "Closed and Failed retain one bounded payload-free terminal outcome' \
    'obligation_bits = ["SetupCancellation", "GuardClose", "ApplicationTeardown"]' \
    'cleanup_order = ["SetupCancellation", "GuardClose", "ApplicationTeardown"]' \
    'terminal_release_preconditions = [' \
    'ack_atomicity = ' \
    'terminal_retention_semantics = ' \
    'teardown_wait_budget_semantics = ' \
    'producer-residual-token:reserved-before-setup' \
    'SetupCancellation-absent-or-complete-or-residual' \
    'ApplicationTeardown-absent-or-complete-or-residual' \
    'GuardClose-absent-or-complete-or-residual' \
    'setup_aborted' \
    'transfer_start_result' \
    'handler_generations_per_slot_max' \
    'WorkClass::HandlerSteps' \
    '`PendingWorkClass` appends `HandlerCall`, `ProducerSubscriptionSetup`, and' \
    'The migration partial order is exact.' \
    'WP-400, WP-500, and WP-600 are unordered sibling branches'; do
    if ! grep -Fq "$invariant" "$states" "$amendment"; then
        echo "WP-100 handler amendment check: missing invariant: $invariant" >&2
        exit 1
    fi
done

for lifecycle_invariant in \
    'setup_cancellation_obligation_semantics = ' \
    'setup_cancel_created' \
    'setup_cancel_claim' \
    'setup_cancel_pending' \
    'three-bit residual mask' \
    'owner_abort_response_transitions = [' \
    'no_result_response_precondition = ' \
    'producer-subscription:Stopping|CleanupPending:obligation_complete->same' \
    'producer-subscription:RollingBack|Stopping|CleanupPending:obligation_complete|obligation_error|obligation_residual->same' \
    'local_view_accounting = ' \
    'join_runtime_stop'; do
    if ! grep -Fq "$lifecycle_invariant" "$states" "$amendment"; then
        echo "WP-100 handler amendment check: missing lifecycle closure: $lifecycle_invariant" >&2
        exit 1
    fi
done

for workload in PERF-GW-020 PERF-GW-021 PERF-GW-022 \
    PERF-CS-015 PERF-CS-016 PERF-CS-017; do
    if ! grep -Fq "id = \"$workload\"" \
        "$root/docs/performance/gateway.toml" \
        "$root/docs/performance/constrained.toml"; then
        echo "WP-100 handler amendment check: missing workload: $workload" >&2
        exit 1
    fi
    if ! grep -Fq "\"$workload\"" "$dag"; then
        echo "WP-100 handler amendment check: unstaged workload: $workload" >&2
        exit 1
    fi
done

if grep -Fq 'handler_invocations_per_dispatch_max' "$gateway" "$constrained"; then
    echo "WP-100 handler amendment check: unscoped one-call invariant remains" >&2
    exit 1
fi

require_workload_text "$gateway" PERF-GW-021 \
    'selected_handler_calls_per_operation_target_max = 1'
require_workload_text "$constrained" PERF-CS-016 \
    'selected_handler_calls_per_operation_target_max = 1'

require_workload_text "$gateway" PERF-GW-022 \
    'setup_handler_kinds = ["sync", "async", "step"]'
require_workload_text "$gateway" PERF-GW-022 \
    'teardown_handler_kinds = ["sync", "async", "step"]'
for manifest in "$gateway" "$constrained"; do
    if ! grep -Fq \
        'producer_subscription_transaction = "before subscription admission ->' \
        "$manifest" \
        || ! grep -Fq \
            'teardown and replay cells continue through every response and cleanup acknowledgement' \
            "$manifest"; then
        echo "WP-100 handler amendment check: Producer transaction boundary excludes teardown" >&2
        exit 1
    fi
done
for pair in sync-sync sync-async sync-step async-sync async-async async-step \
    step-sync step-async step-step; do
    require_workload_text "$gateway" PERF-GW-022 "\"$pair\""
done
for requirement in CLEANUP-RECORD-001 CONCUR-LIN-001 CONCUR-USER-001 \
    HANDLER-CANCEL-001 HANDLER-CANCEL-002 HANDLER-STORAGE-001 \
    HANDLER-SUB-001 RES-LIMIT-001; do
    require_workload_text "$gateway" PERF-GW-022 "\"$requirement\""
done

require_workload_text "$constrained" PERF-CS-017 \
    'setup_handler_kinds = ["sync", "step"]'
require_workload_text "$constrained" PERF-CS-017 \
    'teardown_handler_kinds = ["sync", "step"]'
require_workload_text "$constrained" PERF-CS-017 'subscriber_count = 256'
for pair in sync-sync sync-step step-sync step-step; do
    require_workload_text "$constrained" PERF-CS-017 "\"$pair\""
done
for requirement in CLEANUP-RECORD-001 CONCUR-CRIT-001 CONCUR-LIN-001 \
    CONCUR-USER-001 CONSTRAINED-WORK-001 HANDLER-CANCEL-001 \
    HANDLER-CANCEL-002 HANDLER-STORAGE-001 HANDLER-SUB-001 RES-LIMIT-001; do
    require_workload_text "$constrained" PERF-CS-017 "\"$requirement\""
done

shared_transaction_cases=(
    early-terminal-absent-obligations
    accepted-response-invalid
    success-response-claim-race
    start-response-release-ack
    setup-cancel-cleanup-transfer
    setup-cancel-residual
    teardown-response-release-ack
    teardown-view-cancel-detaches-only
    teardown-view-drop-detaches-only
    owner-abort-step-teardown-no-result
    pending-call-capacity-saturation
    handler-state-capacity-saturation
    cleanup-capacity-saturation-residual
    terminal-replay-before-eviction
    terminal-eviction-stale-replay
)
for transaction_case in "${shared_transaction_cases[@]}"; do
    require_workload_text "$gateway" PERF-GW-022 "\"$transaction_case\""
    require_workload_text "$constrained" PERF-CS-017 "\"$transaction_case\""
done
require_workload_text "$gateway" PERF-GW-022 \
    '"owner-abort-async-teardown-no-result"'

gateway_transaction_fields=(
    'handler_footprint_retained_bytes = 64'
    'handler_footprint_pending_call_bytes = 256'
    'handler_footprint_subscription_bytes = 256'
    'pending_handler_calls_prefill = 1024'
    'handler_state_bytes_charged_after_active = 4194304'
    'cleanup_items_prefill = 1024'
    'cleanup_bytes_prefill = 4194304'
    'producer_residual_records_prefill = 64512'
    'producer_residual_bytes_prefill = 16515072'
    'producer_residual_records_reserved_per_subscription = 1'
    'producer_residual_record_bytes_reserved_per_subscription = 256'
    'teardown_admission_wait_timeout_millis_max = 5000'
    'teardown_admission_wait_steps_max = 1024'
    'terminal_replay_attempts_before_eviction_min = 2'
    'require_stale_handle_after_tombstone_eviction = true'
    'coverage_dimensions = ["handler_kind_pairs", "transaction_cases"]'
    'require_complete_matrix_coverage = true'
    'early_terminal_reservation_leaks_max = 0'
    'duplicate_response_ack_count_changes_max = 0'
    'teardown_wait_budget_overshoot_units_max = 0'
)
for field in "${gateway_transaction_fields[@]}"; do
    require_workload_text "$gateway" PERF-GW-022 "$field"
done

constrained_transaction_fields=(
    'handler_footprint_retained_bytes = 32'
    'handler_footprint_pending_call_bytes = 128'
    'handler_footprint_subscription_bytes = 64'
    'pending_handler_calls_prefill = 32'
    'handler_state_bytes_charged_after_active = 65536'
    'cleanup_items_prefill = 64'
    'cleanup_bytes_prefill = 262144'
    'producer_residual_records_prefill = 0'
    'producer_residual_bytes_prefill = 0'
    'producer_residual_records_reserved_per_subscription = 1'
    'producer_residual_record_bytes_reserved_per_subscription = 128'
    'teardown_admission_wait_steps_max = 64'
    'terminal_replay_attempts_before_eviction_min = 2'
    'require_stale_handle_after_tombstone_eviction = true'
    'coverage_dimensions = ["handler_kind_pairs", "transaction_cases"]'
    'require_complete_matrix_coverage = true'
    'early_terminal_reservation_leaks_max = 0'
    'duplicate_response_ack_count_changes_max = 0'
    'teardown_wait_budget_overshoot_units_max = 0'
)
for field in "${constrained_transaction_fields[@]}"; do
    require_workload_text "$constrained" PERF-CS-017 "$field"
done

for fixture in FX-GW-020 FX-GW-021 FX-GW-022 \
    FX-CS-015 FX-CS-016 FX-CS-017; do
    if ! grep -Fq "id = \"$fixture\"" "$root/docs/performance/fixtures.lock.toml"; then
        echo "WP-100 handler amendment check: missing fixture: $fixture" >&2
        exit 1
    fi
done

declare -A fixture_recipes=(
    [FX-GW-020]='document_bytes=1048576;forms=32;handler_slots=4105'
    [FX-GW-021]='document_bytes=1024;forms=1;handler_slots=32;payload_bytes=64'
    [FX-GW-022]='document_bytes=16384;forms=32;handler_slots=32;payload_bytes=64;subscribers=1024'
    [FX-CS-015]='document_bytes=65536;forms=16;handler_slots=256'
    [FX-CS-016]='document_bytes=1024;forms=1;handler_slots=8;payload_bytes=64'
    [FX-CS-017]='document_bytes=16384;forms=16;handler_slots=4;payload_bytes=64;subscribers=256'
)
for fixture in "${!fixture_recipes[@]}"; do
    block=$(fixture_block "$fixture")
    expected="recipe = \"${fixture_recipes[$fixture]}\""
    if ! grep -Fq "$expected" <<<"$block"; then
        echo "WP-100 handler amendment check: invalid fixture recipe: $fixture" >&2
        exit 1
    fi
done

if ! grep -Fq \
    'owner_packages = ["clinkz-wot-core", "clinkz-wot-foundation", "clinkz-wot-td"]' \
    "$dag" \
    || ! grep -Fq \
        'Owner packages: clinkz-wot-core, clinkz-wot-foundation, clinkz-wot-td' \
        "$wp100" \
    || ! grep -Fq '"handler-foundation-refresh"' "$dag" \
    || ! grep -Fq '`handler-foundation-refresh`' "$wp100"; then
    echo "WP-100 handler amendment check: foundation refresh is not owned by WP-100" >&2
    exit 1
fi

if grep -Fq 'WP-000-owned refresh checkpoint' "$wp000" \
    || grep -Fq 'The handler-amendment refresh proves' "$wp000"; then
    echo "WP-100 handler amendment check: completed WP-000 claims future handler work" >&2
    exit 1
fi

if ! grep -Fq \
    'tools/design-check/Cargo.toml,design-structure-checker,non-normative-checker' \
    "$artifacts"; then
    echo "WP-100 handler amendment check: design checker is not registered" >&2
    exit 1
fi

for evidence in handler-api-matrix handler-storage-replacement \
    handler-cancellation affordance-target-no-atomics callback-lock-isolation \
    producer-emission-migration producer-subscription-transaction \
    legacy-handler-surface-removal; do
    if ! grep -Fq "\"$evidence\"" "$dag"; then
        echo "WP-100 handler amendment check: unstaged evidence: $evidence" >&2
        exit 1
    fi
done

for checkpoint in \
    'WP-300 implements `ProducerEmission`' \
    'WP-400 activates every frozen host setter' \
    'WP-500 completes the Discovery client migration' \
    'WP-600 moves concrete binding publication' \
    'WP-700 proves that all nine raw slot enums'; do
    if ! grep -Fq "$checkpoint" "$amendment"; then
        echo "WP-100 handler amendment check: missing staging checkpoint: $checkpoint" >&2
        exit 1
    fi
done

wp600_block=$(awk '
    /^\[\[package\]\]$/ {
        if (capture) { exit }
        in_package = 1
    }
    in_package && $0 == "id = \"WP-600\"" { capture = 1 }
    capture { print }
' "$dag")
if ! grep -Fq 'depends_on = ["WP-300"]' <<<"$wp600_block" \
    || ! grep -Fq 'feature_cells = ["no-default", "async-no-std", "std"]' \
        <<<"$wp600_block"; then
    echo "WP-100 handler amendment check: WP-600 sibling DAG or feature cells drifted" >&2
    exit 1
fi

for invariant in \
    'The WP-600 feature-cell set is exactly `no-default`, `async-no-std`, and `std`.' \
    '| `clinkz-wot-planning` | `--no-default-features` |' \
    '| `clinkz-wot-protocol-bindings-zenoh` | `--no-default-features` |'; do
    if ! grep -Fq "$invariant" "$wp600"; then
        echo "WP-100 handler amendment check: WP-600 no-default contract misses: $invariant" >&2
        exit 1
    fi
done

poll_block=$(awk '
    /fn poll_subscription\(/ { capture = 1 }
    capture { print }
    capture && /Poll<CoreResult<SubscriptionStart>>;/ { exit }
' "$amendment")
if [[ $(grep -Fc '&mut self' <<<"$poll_block") -ne 1 ]] \
    || ! grep -Fq "cx: &mut Context<'_>" <<<"$poll_block" \
    || ! grep -Fq 'subscription: &mut ClientSubscriptionSlot' <<<"$poll_block" \
    || ! grep -Fq 'budget: &mut WorkBudget' <<<"$poll_block"; then
    echo "WP-100 handler amendment check: poll_subscription signature is invalid" >&2
    exit 1
fi

design_poll_block=$(awk '
    /fn poll_subscription\(/ { capture = 1 }
    capture { print }
    capture && /Poll<CoreResult<SubscriptionStart>>;/ { exit }
' "$root/docs/design.md")
if [[ $(grep -Fc '&mut self' <<<"$design_poll_block") -ne 1 ]]; then
    echo "WP-100 handler amendment check: base poll_subscription has duplicate receiver" >&2
    exit 1
fi

if ! grep -Fq 'Property(String)' "$amendment" \
    || grep -Fq 'Property(Arc<str>)' "$amendment"; then
    echo "WP-100 handler amendment check: AffordanceTarget no-atomic shape is not frozen" >&2
    exit 1
fi

echo "WP-100 handler amendment check: API, state, resource, workload, and staging frozen"
