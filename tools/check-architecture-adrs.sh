#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
normative_sources=(
    "$root/docs/design.md"
    "$root/docs/architecture"
    "$root/docs/spec"
)

contains_normative_fragment() {
    local fragment=$1
    grep -RFq "$fragment" "${normative_sources[@]}"
}

adrs=(
    "docs/ADR/0001-crate-and-module-boundaries.org"
    "docs/ADR/0002-producer-emission-dispatch.org"
    "docs/ADR/0003-subscription-driver-ownership.org"
    "docs/ADR/0004-collection-subscriptions.org"
    "docs/ADR/0005-outbound-request.org"
    "docs/ADR/0006-host-binding-call-cancellation.org"
    "docs/ADR/0007-normative-document-hierarchy.org"
    "docs/ADR/0008-compiled-plan-lifecycle.org"
    "docs/ADR/0009-protocol-binding-integration-and-deployment.org"
    "docs/ADR/0010-server-route-lifecycle.org"
    "docs/ADR/0011-cleanup-reservation-and-transfer.org"
)

if [[ ! -f "$root/docs/ADR/core.org" ]]; then
    echo "architecture ADR check: missing docs/ADR/core.org" >&2
    exit 1
fi

for relative in "${adrs[@]}"; do
    path="$root/$relative"
    if [[ ! -f "$path" ]]; then
        echo "architecture ADR check: missing $relative" >&2
        exit 1
    fi
    if ! grep -Fqx '#+status: Accepted' "$path"; then
        echo "architecture ADR check: $relative is not accepted" >&2
        exit 1
    fi
done

for id in ADR-0001 ADR-0002 ADR-0003 ADR-0004 ADR-0005 ADR-0006 ADR-0007 ADR-0008 ADR-0009 ADR-0010 ADR-0011; do
    if ! grep -Fq "$id" "$root/docs/ADR/core.org"; then
        echo "architecture ADR check: decision index does not reference $id" >&2
        exit 1
    fi
    if ! grep -Fq "$id" "$root/docs/design.md"; then
        echo "architecture ADR check: active design does not reference $id" >&2
        exit 1
    fi
done

architecture_files=(
    "docs/architecture/README.md"
    "docs/architecture/00-system-goals-and-context.md"
    "docs/architecture/10-primary-data-flows.md"
    "docs/architecture/20-module-boundaries.md"
    "docs/architecture/30-compiled-plan-lifecycle.md"
    "docs/architecture/40-protocol-binding-spi-and-deployment.md"
    "docs/architecture/50-servient-runtime-lifecycle.md"
)

for relative in "${architecture_files[@]}"; do
    if [[ ! -f "$root/$relative" ]]; then
        echo "architecture ADR check: missing $relative" >&2
        exit 1
    fi
done

for fragment in \
    'The engine has an explicit compiled-plan-set lifecycle' \
    'The v1 registration set is startup-only' \
    'A Protocol Binding is an ordinary Rust crate' \
    'The v1 server SPI is engine-orchestrated and route-scoped' \
    'The v4.9 target renames the current shared'; do
    if ! grep -RFq "$fragment" "$root/docs/architecture"; then
        echo "architecture ADR check: backbone is missing $fragment" >&2
        exit 1
    fi
done

matrix="$root/docs/api-ownership.csv"
expected_rows=(
    'CollectionSubscriptionCapability,type,clinkz-wot-core,public,clinkz_wot_core::CollectionSubscriptionCapability,frozen'
    'OutboundRequest,type,clinkz-wot-core,public,clinkz_wot_core::OutboundRequest,frozen'
    'HostSubscriptionDriver,trait,clinkz-wot-core,public,clinkz_wot_core::HostSubscriptionDriver,frozen'
    'SubscriptionStopRequest,type,clinkz-wot-core,public,clinkz_wot_core::SubscriptionStopRequest,frozen'
    'SubscriptionDriverEvent,type,clinkz-wot-core,public,clinkz_wot_core::SubscriptionDriverEvent,frozen'
    'BindingEmissionSlot,state_record,clinkz-wot-core,public,clinkz_wot_core::BindingEmissionSlot,frozen'
    'Subscription,type,clinkz-wot-servient,public,clinkz_wot_servient::Subscription,frozen'
    'StaticSubscription,type,clinkz-wot-servient,public,clinkz_wot_servient::StaticSubscription,frozen'
    'EmissionCoordinator,type,clinkz-wot-servient,crate,-,frozen'
    'EmissionDispatchPolicy,type,clinkz-wot-servient,public,clinkz_wot_servient::EmissionDispatchPolicy,frozen'
    'EmissionRecord,state_record,clinkz-wot-servient,crate,-,frozen'
    'CapabilityIndex,type,clinkz-wot-planning,public,clinkz_wot_planning::CapabilityIndex,frozen'
    'PlanCompiler,trait,clinkz-wot-planning,public,clinkz_wot_planning::PlanCompiler,frozen'
    'PlanBuildInput,type,clinkz-wot-planning,public,clinkz_wot_planning::PlanBuildInput,frozen'
    'PlanBuildOutput,type,clinkz-wot-planning,public,clinkz_wot_planning::PlanBuildOutput,frozen'
)

for expected in "${expected_rows[@]}"; do
    item=${expected%%,*}
    actual=$(awk -F, -v item="$item" \
        '$1 == item { print $1 "," $2 "," $3 "," $5 "," $6 "," $14 }' "$matrix")
    if [[ "$actual" != "$expected" ]]; then
        echo "architecture ADR check: ownership mismatch for $item" >&2
        echo "  expected: $expected" >&2
        echo "  actual:   $actual" >&2
        exit 1
    fi
done

expected_cells=(
    'SubscriptionDriverEvent,no-default|async-no-std|std,all,all'
    'Subscription,std,host-async,gateway-default-v1'
    'StaticSubscription,no-default|async-no-std|std,manual-poll,application-static'
)

for expected in "${expected_cells[@]}"; do
    item=${expected%%,*}
    actual=$(awk -F, -v item="$item" \
        '$1 == item { print $1 "," $7 "," $8 "," $9 }' "$matrix")
    if [[ "$actual" != "$expected" ]]; then
        echo "architecture ADR check: feature-cell mismatch for $item" >&2
        echo "  expected: $expected" >&2
        echo "  actual:   $actual" >&2
        exit 1
    fi
done

for removed in BindingRequest BindingDrivingMode EventBroker EventName EventStream PublisherSink RuntimeEventSinkConfig SubscriptionSender; do
    status=$(awk -F, -v item="$removed" '$1 == item { print $14 }' "$matrix")
    if [[ "$status" != "removed" ]]; then
        echo "architecture ADR check: $removed is not recorded as removed" >&2
        exit 1
    fi
done

if grep -REq '(pub[[:space:]]+(struct|enum|trait|type)[[:space:]]+BindingRequest|impl[[:space:]]+BindingRequest)' \
    "$root/docs/design.md" \
    "$root/docs/spec" \
    "$root/docs/work-packages/WP-200-planning.md" \
    "$root/docs/work-packages/WP-300-bindings.md" \
    "$root/docs/work-packages/WP-400-servient.md" \
    "$root/docs/work-packages/WP-600-protocol-bindings.md"; then
    echo "architecture ADR check: a normative target declaration retains BindingRequest" >&2
    exit 1
fi

for fragment in \
    'pub struct EmissionDispatchPolicy' \
    'lanes_per_binding: NonZeroU32' \
    'max_in_flight_per_lane: NonZeroU32' \
    '`GatewayDefaultV1` constructs `EmissionDispatchPolicy::new(1, 16)`'; do
    if ! contains_normative_fragment "$fragment"; then
        echo "architecture ADR check: dispatch policy schema is missing $fragment" >&2
        exit 1
    fi
done

for fragment in \
    'pub fn try_with_collection_subscription_capability(' \
    'pub enum SubscriptionDriverEvent' \
    'pub struct StaticSubscription' \
    'fn start_stop(' \
    'fn start_subscription_stop('; do
    if ! contains_normative_fragment "$fragment"; then
        echo "architecture ADR check: implementable API schema is missing $fragment" >&2
        exit 1
    fi
done


for fragment in \
    'version = 2' \
    'lanes_per_binding = 1' \
    'max_in_flight_per_lane = 16' \
    'require_exact_emission_policy_configuration = true'; do
    if ! grep -Fq "$fragment" "$root/docs/performance/gateway.toml"; then
        echo "architecture ADR check: Gateway emission workload is missing $fragment" >&2
        exit 1
    fi
done

echo "architecture ADR check: eleven accepted decisions and the v4.9 backbone are registered"
