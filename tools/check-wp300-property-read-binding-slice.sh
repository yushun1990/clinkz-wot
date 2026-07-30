#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
fixture="$root/tools/compile-contracts/wp300-property-read-binding-slice/Cargo.toml"

fail() {
    echo "WP-300 property-read binding slice check: $*" >&2
    exit 1
}

if [[ ! -f "$root/core/src/binding.rs" ]]; then
    fail "Core Property Read binding implementation is missing"
fi

implementation_paths=(
    "core/src/binding.rs"
    "core/src/lib.rs"
)
for path in "${implementation_paths[@]}"; do
    [[ -f "$root/$path" ]] || fail "registered implementation path is missing: $path"
done

binding_source="$root/core/src/binding.rs"
for marker in \
    "pub trait PollServerBinding" \
    "pub trait RouteServerBinding" \
    "pub struct HostBindingRegistrationInput" \
    "pub struct HostBindingRegistration" \
    "pub struct StaticBindingRegistrationInput" \
    "pub struct StaticBindingRegistration" \
    "pub struct RouteResponseOpportunity" \
    "pub struct ServingActivationAuthority" \
    "pub struct CleanupTransferEnvelope"; do
    grep -Fq "$marker" "$binding_source" \
        || fail "reviewed binding marker is missing: $marker"
done

grep -Fq "pub mod binding;" "$root/core/src/lib.rs" \
    || fail "Core does not expose the target binding module"
for marker in \
    "PollServerBinding" \
    "RouteInboundRequest" \
    "RouteInboundResponse" \
    "RouteServerBinding" \
    "StaticBindingRegistrationInput" \
    "StaticBindingRegistration"; do
    grep -Fq "$marker" "$root/core/src/lib.rs" \
        || fail "Core root export is missing: $marker"
done

for forbidden in \
    "clinkz_wot_protocol_bindings" \
    "select_affordance_form" \
    "select_form" \
    "InboundDispatcher" \
    "ReadPropertyHandler" \
    "ServerBinding::serve" \
    "serve_request"; do
    if grep -Fq "$forbidden" "$binding_source"; then
        fail "target binding source contains forbidden legacy authority: $forbidden"
    fi
done

export CARGO_TARGET_DIR="$root/target/wp300-property-read-binding-slice"

cargo check --locked --quiet --manifest-path "$fixture" --no-default-features
cargo check --locked --quiet --manifest-path "$fixture" --features async
cargo check --locked --quiet --manifest-path "$fixture" --features std
cargo test --locked --quiet --manifest-path "$fixture" --features std

cargo test --locked --quiet --manifest-path "$root/core/Cargo.toml" --no-default-features
cargo test --locked --quiet --manifest-path "$root/core/Cargo.toml" --features async
cargo test --locked --quiet --manifest-path "$root/core/Cargo.toml" --features std
cargo check --locked --quiet --manifest-path "$root/servient/Cargo.toml" --no-default-features
cargo check --locked --quiet \
    --manifest-path "$root/protocol-bindings/core/Cargo.toml" \
    --no-default-features

"$root/tools/check-wp200-property-read-plan-slice.sh"

for fixture_root in \
    "$root/tools/architecture-fixtures/property-read-binding" \
    "$root/tools/architecture-fixtures/property-read-runner"; do
    [[ ! -e "$fixture_root" ]] \
        || fail "downstream architecture fixture root exists before its owning slice is reviewed"
done

echo "WP-300 property-read binding slice check: registration, route, response, and cleanup boundary valid"
