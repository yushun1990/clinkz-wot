#!/usr/bin/env sh

set -eu

OUTPUT_FILE=$(mktemp)
trap 'rm -f "$OUTPUT_FILE"' EXIT

if cargo check -p clinkz-wot-protocol-bindings-zenoh --features runtime-zenoh-pico >"$OUTPUT_FILE" 2>&1; then
    echo "expected runtime-zenoh-pico to fail until the constrained backend is implemented" >&2
    exit 1
fi

if ! grep -q "zenoh-pico runtime backend is reserved but not implemented yet" "$OUTPUT_FILE"; then
    cat "$OUTPUT_FILE" >&2
    echo "runtime-zenoh-pico failed without the expected reserved-backend diagnostic" >&2
    exit 1
fi

OUTPUT_FILE_CONFLICT=$(mktemp)
trap 'rm -f "$OUTPUT_FILE" "$OUTPUT_FILE_CONFLICT"' EXIT

if cargo check -p clinkz-wot-protocol-bindings-zenoh --features runtime-zenoh,runtime-zenoh-pico >"$OUTPUT_FILE_CONFLICT" 2>&1; then
    echo "expected runtime-zenoh and runtime-zenoh-pico to be mutually exclusive" >&2
    exit 1
fi

if ! grep -q "Only one concrete zenoh runtime backend can be enabled" "$OUTPUT_FILE_CONFLICT"; then
    cat "$OUTPUT_FILE_CONFLICT" >&2
    echo "runtime backend conflict failed without the expected diagnostic" >&2
    exit 1
fi
