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
    "docs/ADRs/0001-crate-and-module-boundaries.org"
    "docs/ADRs/0002-producer-emission-dispatch.org"
    "docs/ADRs/0003-subscription-driver-ownership.org"
    "docs/ADRs/0004-collection-subscriptions.org"
    "docs/ADRs/0005-outbound-request.org"
    "docs/ADRs/0006-host-binding-call-cancellation.org"
    "docs/ADRs/0007-normative-document-hierarchy.org"
    "docs/ADRs/0008-compiled-plan-lifecycle.org"
    "docs/ADRs/0009-protocol-binding-integration-and-deployment.org"
    "docs/ADRs/0010-server-route-lifecycle.org"
    "docs/ADRs/0011-cleanup-reservation-and-transfer.org"
    "docs/ADRs/0012-serving-activation-permit.org"
    "docs/ADRs/0013-work-package-scoped-implementation-admission.org"
    "docs/ADRs/0014-transitional-normative-ownership.org"
    "docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org"
)

if [[ ! -f "$root/docs/ADRs/core.org" ]]; then
    echo "architecture ADR check: missing docs/ADRs/core.org" >&2
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

for id in ADR-0001 ADR-0002 ADR-0003 ADR-0004 ADR-0005 ADR-0006 ADR-0007 ADR-0008 ADR-0009 ADR-0010 ADR-0011 ADR-0012 ADR-0013 ADR-0014 ADR-0015; do
    if ! grep -Fq "$id" "$root/docs/ADRs/core.org"; then
        echo "architecture ADR check: decision index does not reference $id" >&2
        exit 1
    fi
    if ! grep -Fq "$id" "$root/docs/design.md"; then
        echo "architecture ADR check: active design does not reference $id" >&2
        exit 1
    fi
done

for fragment in \
    'A registered normative amendment is' \
    'a bare `ResourceProfileId` identifies a profile but does not prove its' \
    'implements neither `Copy` nor `Clone`' \
    "const LIMITS: &'static ResourceLimits;"; do
    if ! contains_normative_fragment "$fragment"; then
        echo "architecture ADR check: ADR-0014/0015 projection is missing $fragment" >&2
        exit 1
    fi
done

for fragment in \
    'Implementation admission is tranche-scoped as defined by ADR-0013' \
    'Global gates are the aggregate convergence and final' \
    'Implementation-produced measurements are completion'; do
    if ! contains_normative_fragment "$fragment"; then
        echo "architecture ADR check: tranche-admission policy is missing $fragment" >&2
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
    'SubscriptionStopInput,type,clinkz-wot-core,public,clinkz_wot_core::SubscriptionStopInput,frozen'
    'SubscriptionDriverEvent,type,clinkz-wot-core,public,clinkz_wot_core::SubscriptionDriverEvent,frozen'
    'SubscriptionDriverCleanupDisposition,type,clinkz-wot-core,public,clinkz_wot_core::SubscriptionDriverCleanupDisposition,frozen'
    'CleanupTransferEnvelope,type,clinkz-wot-core,public,clinkz_wot_core::CleanupTransferEnvelope,frozen'
    'CleanupTransferAcceptance,type,clinkz-wot-core,public,clinkz_wot_core::CleanupTransferAcceptance,frozen'
    'CleanupTransferTarget,trait,clinkz-wot-core,public,clinkz_wot_core::CleanupTransferTarget,frozen'
    'BindingCancellationDisposition,type,clinkz-wot-core,public,clinkz_wot_core::BindingCancellationDisposition,frozen'
    'BindingCallSettlement,type,clinkz-wot-core,public,clinkz_wot_core::BindingCallSettlement,frozen'
    'RouteCleanupSuccessor,type,clinkz-wot-core,public,clinkz_wot_core::RouteCleanupSuccessor,frozen'
    'HostRouteCleanupSuccessor,type,clinkz-wot-core,public,clinkz_wot_core::HostRouteCleanupSuccessor,frozen'
    'ServingActivationAuthority,type,clinkz-wot-core,public,clinkz_wot_core::ServingActivationAuthority,frozen'
    'RouteAcceptLease,type,clinkz-wot-core,public,clinkz_wot_core::RouteAcceptLease,frozen'
    'RouteAcceptClaim,type,clinkz-wot-core,public,clinkz_wot_core::RouteAcceptClaim,frozen'
    'RouteAcceptClaimError,type,clinkz-wot-core,public,clinkz_wot_core::RouteAcceptClaimError,frozen'
    'RouteActivationPermit,type,clinkz-wot-core,public,clinkz_wot_core::RouteActivationPermit,frozen'
    'HostCommittedRouteGuard,type,clinkz-wot-core,public,clinkz_wot_core::HostCommittedRouteGuard,frozen'
    'HostShutdownRouteGuard,type,clinkz-wot-core,public,clinkz_wot_core::HostShutdownRouteGuard,frozen'
    'HostSubscriptionDriverBox,type,clinkz-wot-core,public,clinkz_wot_core::HostSubscriptionDriverBox,frozen'
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
    'pub enum SubscriptionDriverCleanupDisposition' \
    'pub struct SubscriptionStopInput' \
    'BindingInputRejection<SubscriptionStopInput>' \
    'pub struct CleanupTransferEnvelope<T>' \
    'pub enum CleanupTransferAcceptance<T>' \
    'pub trait CleanupTransferTarget<T>' \
    'fn try_accept(' \
    'pub enum BindingCancellationDisposition<C>' \
    'pub enum BindingCallSettlement<T, C = NoCleanupSuccessor>' \
    'Returned(T)' \
    'pub enum RouteCleanupSuccessor<P, A, C>' \
    'ShutdownCommitted(C)' \
    'ResidualRouteState { route: BindingRouteKey }' \
    'pub type HostRouteCleanupSuccessor' \
    'pub struct ServingActivationAuthority' \
    'pub struct RouteAcceptLease' \
    'pub struct RouteAcceptClaim' \
    'pub enum RouteAcceptClaimError' \
    'pub struct RouteActivationPermit' \
    'pub fn claim_route' \
    'pub fn into_permit' \
    'pub enum HostShutdownRouteGuard' \
    'RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>' \
    'route: Pin<&mut HostCommittedRouteGuard>' \
    'permit: RouteActivationPermit' \
    ') -> Poll<T>;' \
    'HostBindingCallBox<CoreResult<InteractionOutput>>' \
    'pub struct HostSubscriptionDriverBox(' \
    'pub fn into_cleanup_transfer(' \
    'pub struct StaticSubscription' \
    'fn start_stop(' \
    'fn poll_subscription_start(' \
    'fn start_subscription_stop('; do
    if ! contains_normative_fragment "$fragment"; then
        echo "architecture ADR check: implementable API schema is missing $fragment" >&2
        exit 1
    fi
done

for source in \
    "$root/docs/ADRs/0006-host-binding-call-cancellation.org" \
    "$root/docs/spec/binding-spi.md"; do
    for fragment in \
        'pub enum BindingCancellationDisposition<C>' \
        'pub enum BindingCallSettlement<T, C = NoCleanupSuccessor>' \
        'Returned(T)' \
        'TransferRequired(CleanupTransferRequest)' \
        'retry_class: RetryClass'; do
        if ! grep -Fq "$fragment" "$source"; then
            echo "architecture ADR check: settlement schema mismatch in ${source#$root/}: $fragment" >&2
            exit 1
        fi
    done
done

settlement_adr=$(awk '
    /^pub enum BindingCancellationDisposition<C> \{/ { capture = 1 }
    capture {
        print
        if ($0 == "}") {
            closes++
            if (closes == 2) exit
        }
    }
' "$root/docs/ADRs/0006-host-binding-call-cancellation.org")
settlement_spec=$(awk '
    /^pub enum BindingCancellationDisposition<C> \{/ { capture = 1 }
    capture {
        print
        if ($0 == "}") {
            closes++
            if (closes == 2) exit
        }
    }
' "$root/docs/spec/binding-spi.md")
if [[ "$settlement_adr" != "$settlement_spec" ]]; then
    echo "architecture ADR check: ADR-0006 and binding SPI freeze different settlement schemas" >&2
    exit 1
fi

subscription_driver_adr=$(awk '
    /^pub trait HostSubscriptionDriver:/ { capture = 1 }
    capture {
        print
        if ($0 == "}") exit
    }
' "$root/docs/ADRs/0003-subscription-driver-ownership.org")
subscription_driver_spec=$(awk '
    /^pub trait HostSubscriptionDriver:/ { capture = 1 }
    capture {
        print
        if ($0 == "}") exit
    }
' "$root/docs/spec/binding-spi.md")
if [[ "$subscription_driver_adr" != "$subscription_driver_spec" ]]; then
    echo "architecture ADR check: ADR-0003 and binding SPI freeze different host driver signatures" >&2
    exit 1
fi

if grep -REq \
    '(BindingCallSettlement::(LateValue|TerminalValue)|TerminalValue\(T\),|CleanupComplete\(CleanupRecord\)|fn start_stop_subscription\(|fn poll_start_subscription\(|Arc<Self>|StartStatus<CleanupOutcome>|Poll<CoreResult<CleanupOutcome>>|RouteCommitOutcome<A>[[:space:]]*\{|Serving\(A\),|route: Pin<&mut HostActiveRouteGuard>)' \
    "$root/docs/ADRs/0003-subscription-driver-ownership.org" \
    "$root/docs/ADRs/0006-host-binding-call-cancellation.org" \
    "$root/docs/spec/binding-spi.md"; then
    echo "architecture ADR check: a superseded cancellation or subscription spelling remains" >&2
    exit 1
fi

commit_outcome_adr=$(awk '
    /^pub enum RouteCommitOutcome<A, C> \{/ { capture = 1 }
    capture {
        print
        if ($0 == "}") exit
    }
' "$root/docs/ADRs/0012-serving-activation-permit.org")
commit_outcome_spec=$(awk '
    /^pub enum RouteCommitOutcome<A, C> \{/ { capture = 1 }
    capture {
        print
        if ($0 == "}") exit
    }
' "$root/docs/spec/binding-spi.md")
if [[ -z "$commit_outcome_adr" || "$commit_outcome_adr" != "$commit_outcome_spec" ]]; then
    echo "architecture ADR check: ADR-0012 and binding SPI freeze different commit outcomes" >&2
    exit 1
fi


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

echo "architecture ADR check: fifteen accepted decisions and the v4.9 backbone are registered"
