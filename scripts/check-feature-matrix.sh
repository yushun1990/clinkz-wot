#!/usr/bin/env sh
# Build-check the supported feature cells for every product crate.

set -eu

pass=0
fail=0

check() {
    desc="$1"; shift
    if cargo check --locked "$@"; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        echo "  ✗ FAIL: $desc ($*)"
    fi
}

echo "=== td ==="
check "td default"              -p clinkz-wot-td
check "td no-features"          -p clinkz-wot-td --no-default-features

echo "=== foundation ==="
check "foundation default"      -p clinkz-wot-foundation
check "foundation no-features"  -p clinkz-wot-foundation --no-default-features
check "foundation async"        -p clinkz-wot-foundation --no-default-features --features async

echo "=== core ==="
check "core default"            -p clinkz-wot-core
check "core no-features"        -p clinkz-wot-core --no-default-features
check "core async"              -p clinkz-wot-core --no-default-features --features async

echo "=== planning ==="
check "planning default"        -p clinkz-wot-planning
check "planning no-features"    -p clinkz-wot-planning --no-default-features
check "planning async"          -p clinkz-wot-planning --no-default-features --features async

echo "=== isolated Planning normal dependency ==="
check "planning isolated default"     --manifest-path planning/tests/normal-dependency/Cargo.toml
check "planning isolated no-default"  --manifest-path planning/tests/normal-dependency/Cargo.toml --no-default-features
check "planning isolated async"       --manifest-path planning/tests/normal-dependency/Cargo.toml --no-default-features --features async

echo "=== protocol-bindings-http ==="
check "http preparation"        -p clinkz-wot-binding-http
check "http execution-only"     -p clinkz-wot-binding-http --no-default-features

echo "=== discovery ==="
check "discovery default"       -p clinkz-wot-discovery
check "discovery no-features"   -p clinkz-wot-discovery --no-default-features
check "discovery async"         -p clinkz-wot-discovery --no-default-features --features async

echo "=== servient ==="
check "servient default"        -p clinkz-wot-servient
check "servient no-features"    -p clinkz-wot-servient --no-default-features
check "servient async"          -p clinkz-wot-servient --no-default-features --features async

echo "=== source-free Producer execution ==="
check "finite default"          -p clinkz-wot-servient-finite
check "finite no-features"      -p clinkz-wot-servient-finite --no-default-features

echo "=== Property Read architecture fixtures ==="
check "aggregate binding default"      -p clinkz-wot-property-read-binding-fixture
check "aggregate binding no-features"  -p clinkz-wot-property-read-binding-fixture --no-default-features
check "aggregate binding async"        -p clinkz-wot-property-read-binding-fixture --no-default-features --features async
check "aggregate runner default"       -p clinkz-wot-property-read-architecture-runner
check "aggregate runner no-features"   -p clinkz-wot-property-read-architecture-runner --no-default-features
check "aggregate runner async"         -p clinkz-wot-property-read-architecture-runner --no-default-features --features async
check "consumer binding default"       -p clinkz-wot-consumer-property-read-binding-fixture
check "consumer binding no-features"   -p clinkz-wot-consumer-property-read-binding-fixture --no-default-features
check "consumer binding async"         -p clinkz-wot-consumer-property-read-binding-fixture --no-default-features --features async
check "consumer runner default"        -p clinkz-wot-consumer-property-read-runtime-runner
check "consumer runner no-features"    -p clinkz-wot-consumer-property-read-runtime-runner --no-default-features
check "consumer runner async"          -p clinkz-wot-consumer-property-read-runtime-runner --no-default-features --features async

echo "=== codec-cbor ==="
check "cbor default"            -p clinkz-wot-codec-cbor
check "cbor no-features"        -p clinkz-wot-codec-cbor --no-default-features

echo "=== facade ==="
check "facade default"          -p clinkz-wot
check "facade no-features"      -p clinkz-wot --no-default-features
check "facade async"            -p clinkz-wot --no-default-features --features async
check "facade cbor"             -p clinkz-wot --features cbor

echo ""
echo "Feature matrix: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
