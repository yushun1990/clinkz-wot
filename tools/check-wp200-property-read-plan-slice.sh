#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
fixture="$root/tools/compile-contracts/wp200-property-read-plan-slice/Cargo.toml"

fail() {
    echo "WP-200 property-read plan slice check: $*" >&2
    exit 1
}

if [[ ! -f "$root/core/src/binding_compiler.rs" ]]; then
    fail "Core binding compiler implementation is missing"
fi

implementation_paths=(
    "Cargo.lock"
    "Cargo.toml"
    "core/src/binding_compiler.rs"
    "core/src/identity.rs"
    "core/src/lib.rs"
    "core/src/plan.rs"
    "planning/Cargo.toml"
    "planning/src/lib.rs"
    "planning/src/property_read.rs"
)
for path in "${implementation_paths[@]}"; do
    [[ -f "$root/$path" ]] || fail "registered implementation path is missing: $path"
done

grep -Fq '"planning"' "$root/Cargo.toml" \
    || fail "root workspace does not register the Planning crate"
grep -Fq "pub trait BindingCompilerExtension" "$root/core/src/binding_compiler.rs" \
    || fail "portable binding compiler trait is missing"
grep -Fq "pub struct HostBindingCompilerRegistration" "$root/core/src/binding_compiler.rs" \
    || fail "Core host compiler erasure is missing"
grep -Fq "pub struct StaticBindingCompilerRegistration" "$root/core/src/binding_compiler.rs" \
    || fail "static compiler registration is missing"
grep -Fq "pub trait PlanCompiler" "$root/planning/src/lib.rs" \
    || fail "Planning compiler contract is missing"

export CARGO_TARGET_DIR="$root/target/wp200-property-read-plan-slice"

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
cargo check --locked --quiet --manifest-path "$root/servient/Cargo.toml" --no-default-features
cargo check --locked --quiet \
    --manifest-path "$root/protocol-bindings/core/Cargo.toml" \
    --no-default-features

"$root/tools/check-wp100-property-read-handler-slice.sh"

for fixture_root in \
    "$root/tools/architecture-fixtures/property-read-binding" \
    "$root/tools/architecture-fixtures/property-read-runner"; do
    [[ ! -e "$fixture_root" ]] \
        || fail "downstream architecture fixture root exists before its owning slice is reviewed"
done

echo "WP-200 property-read plan slice check: compiler/artifact and owned plan boundary valid"
