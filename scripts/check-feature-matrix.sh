#!/usr/bin/env sh
# Build-check the supported feature cells for every product crate.

set -eu

pass=0
fail=0

check() {
    desc="$1"; shift
    if cargo check --locked "$@" 2>/dev/null; then
        pass=$((pass + 1))
        # echo "  ✓ $desc"
    else
        fail=$((fail + 1))
        echo "  ✗ FAIL: $desc ($*)"
    fi
}

echo "=== td ==="
check "td default"              -p clinkz-wot-td
check "td no-features"          -p clinkz-wot-td --no-default-features
check "td td2-preview"          -p clinkz-wot-td --features td2-preview

echo "=== foundation ==="
check "foundation default"      -p clinkz-wot-foundation
check "foundation no-features"  -p clinkz-wot-foundation --no-default-features
check "foundation async"        -p clinkz-wot-foundation --no-default-features --features async

echo "=== core ==="
check "core default"            -p clinkz-wot-core
check "core no-features"        -p clinkz-wot-core --no-default-features
check "core async"              -p clinkz-wot-core --no-default-features --features async
check "core td2-preview"        -p clinkz-wot-core --features td2-preview

echo "=== planning ==="
check "planning default"        -p clinkz-wot-planning
check "planning no-features"    -p clinkz-wot-planning --no-default-features
check "planning async"          -p clinkz-wot-planning --no-default-features --features async

echo "=== protocol-bindings ==="
check "pb default"              -p clinkz-wot-protocol-bindings
check "pb no-features"          -p clinkz-wot-protocol-bindings --no-default-features

echo "=== protocol-bindings-zenoh ==="
check "zenoh default"           -p clinkz-wot-protocol-bindings-zenoh
check "zenoh no-features"       -p clinkz-wot-protocol-bindings-zenoh --no-default-features
check "zenoh-pico"              -p clinkz-wot-protocol-bindings-zenoh --no-default-features --features zenoh-pico
check "zenoh td2-preview"       -p clinkz-wot-protocol-bindings-zenoh --features td2-preview

echo "=== discovery ==="
check "discovery default"       -p clinkz-wot-discovery
check "discovery no-features"   -p clinkz-wot-discovery --no-default-features

echo "=== servient ==="
check "servient default"        -p clinkz-wot-servient
check "servient no-features"    -p clinkz-wot-servient --no-default-features
check "servient async"          -p clinkz-wot-servient --no-default-features --features async
check "servient td2-preview"    -p clinkz-wot-servient --features td2-preview

echo "=== Property Read architecture fixtures ==="
check "aggregate binding default"      -p clinkz-wot-property-read-binding-fixture
check "aggregate binding no-features"  -p clinkz-wot-property-read-binding-fixture --no-default-features
check "aggregate runner default"       -p clinkz-wot-property-read-architecture-runner
check "aggregate runner no-features"   -p clinkz-wot-property-read-architecture-runner --no-default-features

echo "=== codec-cbor ==="
check "cbor default"            -p clinkz-wot-codec-cbor
check "cbor no-features"        -p clinkz-wot-codec-cbor --no-default-features

echo "=== facade ==="
check "facade default"          -p clinkz-wot
check "facade no-features"      -p clinkz-wot --no-default-features

echo ""
echo "Feature matrix: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
