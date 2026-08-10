#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
fixture="$root/tools/compile-contracts/wp400-property-read-servient-slice/Cargo.toml"
servient_cargo="$root/servient/Cargo.toml"
property_source="$root/servient/src/property_read.rs"

fail() {
    echo "WP-400 Property Read Servient check: $*" >&2
    exit 1
}

grep -Fq 'clinkz-wot-planning = { path = "../planning"' "$servient_cargo" \
    || fail "Servient Planning dependency is missing"
grep -Fq 'clinkz-wot-foundation = { path = "../foundation"' "$servient_cargo" \
    || fail "Servient Foundation dependency is missing"
grep -Fq 'mod property_read;' "$root/servient/src/lib.rs" \
    || fail "reviewed Property Read module is not installed"
[[ -f "$property_source" ]] || fail "reviewed Property Read source is missing"

for marker in \
    "pub trait StaticServient" \
    "pub struct StaticServientBuilder" \
    "pub enum ExposeState" \
    "pub enum CompiledPlanSetState" \
    "struct PropertyReadHandlerRecord" \
    "struct CompiledPlanSetRecord" \
    "struct PlanSetLease" \
    "struct BindingRouteRecord" \
    "struct InFlightRecord" \
    "struct ServingActivationRecord" \
    "PropertyReadPlanCompiler::producer_route(" \
    "BindingRouteKey::new(" \
    "PrepareInput::new(" \
    "ServingActivationAuthority::new(" \
    "RouteAcceptLease::new(" \
    "HandlerContext::try_new(" \
    "RouteInboundResponse::new("; do
    grep -FRq "$marker" "$root/servient/src" \
        || fail "reviewed Servient source is missing: $marker"
done

for marker in \
    "pub fn binding_registration(" \
    "pub fn resource_limits(" \
    "pub fn set_read_property_handler" \
    "pub fn begin_expose(" \
    "pub fn step("; do
    grep -FRq "$marker" "$root/servient/src" \
        || fail "reviewed Servient public boundary is missing: $marker"
done

for forbidden in \
    "BindingRouteKey::new(" \
    "PrepareInput::new(" \
    "ServingActivationAuthority::new(" \
    "RouteAcceptLease::new(" \
    "HandlerContext::try_new(" \
    "RouteInboundResponse::new(" \
    "RouteReservationIdentity::new(" \
    "CollisionDomainId::new(" \
    "EndpointReservationKey::new("; do
    if grep -FRq "$forbidden" \
        "$root/tools/compile-contracts/wp400-property-read-servient-slice/src" \
        "$root/tools/compile-contracts/wp400-property-read-servient-slice/tests"; then
        fail "runner constructs forbidden product-owned value: $forbidden"
    fi
done

export CARGO_TARGET_DIR="$root/target/wp400-property-read-servient-slice"

cargo check --locked --quiet --manifest-path "$fixture" --no-default-features
cargo check --locked --quiet --manifest-path "$fixture" --features async
cargo check --locked --quiet --manifest-path "$fixture" --features std
cargo test --locked --quiet --manifest-path "$fixture" --features std

cargo test --locked --quiet --manifest-path "$servient_cargo" --no-default-features
cargo test --locked --quiet --manifest-path "$servient_cargo" --features async
cargo test --locked --quiet --manifest-path "$servient_cargo" --features std

"$root/tools/check-wp100-property-read-handler-slice.sh"
"$root/tools/check-wp200-property-read-plan-slice.sh"
"$root/tools/check-wp300-property-read-binding-slice.sh"
"$root/tools/check-wp200-property-read-producer-route.sh"
"$root/tools/check-wp200-property-read-route-reservation.sh"

for fixture_root in \
    "$root/tools/architecture-fixtures/property-read-binding" \
    "$root/tools/architecture-fixtures/property-read-runner"; do
    [[ ! -e "$fixture_root" ]] \
        || fail "final architecture fixture root exists before architecture admission"
done

echo "WP-400 Property Read Servient check: lifecycle composition valid"
