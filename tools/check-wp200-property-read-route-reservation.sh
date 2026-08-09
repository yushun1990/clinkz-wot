#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
fixture="$root/tools/compile-contracts/wp200-property-read-route-reservation/Cargo.toml"
source="$root/core/src/binding_compiler.rs"

fail() {
    echo "WP-200 Property Read route-reservation check: $*" >&2
    exit 1
}

grep -Fq "pub const fn producer_route(" "$source" \
    || fail "Producer-route artifact reservation constructor is missing"
grep -Fq "pub const fn route_reservation(" "$source" \
    || fail "artifact route-reservation accessor is missing"
grep -Fq "pub fn into_route_parts(" "$source" \
    || fail "complete route-reservation consuming surface is missing"
grep -Fq "MissingRouteReservation" "$source" \
    || fail "missing-route-reservation rejection is absent"
grep -Fq "UnexpectedRouteReservation" "$source" \
    || fail "unexpected-route-reservation rejection is absent"
grep -Fq "route_reservation" "$root/planning/src/property_read.rs" \
    || fail "Planning does not preserve the reviewed route metadata contract"
grep -Fq "BindingArtifact::producer_route(" \
    "$root/tools/compile-contracts/wp300-property-read-binding-slice/src/lib.rs" \
    || fail "complete WP-300 mock compiler does not produce route metadata"

for forbidden in \
    "RouteReservationIdentity::new(" \
    "CollisionDomainId::new(" \
    "EndpointReservationKey::new("; do
    if grep -Fq "$forbidden" "$root/tools/compile-contracts/wp200-property-read-route-reservation/src/lib.rs"; then
        fail "runner fixture constructs forbidden route identity: $forbidden"
    fi
done

export CARGO_TARGET_DIR="$root/target/wp200-property-read-route-reservation"

cargo check --locked --quiet --manifest-path "$fixture" --no-default-features
cargo check --locked --quiet --manifest-path "$fixture" --features async
cargo check --locked --quiet --manifest-path "$fixture" --features std
cargo test --locked --quiet --manifest-path "$fixture" --features std

cargo test --locked --quiet --manifest-path "$root/core/Cargo.toml" --no-default-features
cargo test --locked --quiet --manifest-path "$root/core/Cargo.toml" --features async
cargo test --locked --quiet --manifest-path "$root/core/Cargo.toml" --features std
cargo test --locked --quiet --manifest-path "$root/planning/Cargo.toml" --no-default-features
cargo test --locked --quiet --manifest-path "$root/planning/Cargo.toml" --features async
cargo test --locked --quiet --manifest-path "$root/planning/Cargo.toml" --features std

"$root/tools/check-wp200-property-read-plan-slice.sh"
"$root/tools/check-wp300-property-read-binding-slice.sh"
"$root/tools/check-wp200-property-read-producer-route.sh"

for fixture_root in \
    "$root/tools/architecture-fixtures/property-read-binding" \
    "$root/tools/architecture-fixtures/property-read-runner"; do
    [[ ! -e "$fixture_root" ]] \
        || fail "architecture fixture root exists before WP-400 review"
done

echo "WP-200 Property Read route-reservation check: compiler-owned identity handoff valid"
