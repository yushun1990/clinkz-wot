#!/usr/bin/env sh
# Feature-matrix build-check (audit defect AD5 / phase-p4 §4.3).
#
# Build-checks ALL valid feature combinations per crate (~28) to catch
# compile-time feature-interaction defects. This is a BUILD-CHECK
# (`cargo check`), not a test run — test coverage is a representative
# subset (see docs/verification.md).

set -eu

pass=0
fail=0

check() {
    desc="$1"; shift
    if cargo check "$@" 2>/dev/null; then
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

echo "=== core ==="
check "core default"            -p clinkz-wot-core
check "core no-features"        -p clinkz-wot-core --no-default-features
check "core async"              -p clinkz-wot-core --no-default-features --features async
check "core td2-preview"        -p clinkz-wot-core --features td2-preview

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

echo "=== codec-cbor ==="
check "cbor default"            -p clinkz-wot-codec-cbor
check "cbor no-features"        -p clinkz-wot-codec-cbor --no-default-features

echo ""
echo "Feature matrix: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
