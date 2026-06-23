#!/usr/bin/env sh

set -eu

OUTPUT_FILE=$(mktemp)
trap 'rm -f "$OUTPUT_FILE"' EXIT

if ! cargo check -p clinkz-wot-protocol-bindings-zenoh --no-default-features --features zenoh-pico >"$OUTPUT_FILE" 2>&1; then
    cat "$OUTPUT_FILE" >&2
    echo "expected zenoh-pico platform-hook backend to compile" >&2
    exit 1
fi

if ! cargo test -p clinkz-wot-protocol-bindings-zenoh --no-default-features --features zenoh-pico >"$OUTPUT_FILE" 2>&1; then
    cat "$OUTPUT_FILE" >&2
    echo "expected zenoh-pico fake platform tests to pass" >&2
    exit 1
fi

OUTPUT_FILE_CONFLICT=$(mktemp)
trap 'rm -f "$OUTPUT_FILE" "$OUTPUT_FILE_CONFLICT"' EXIT

if cargo check -p clinkz-wot-protocol-bindings-zenoh --features zenoh,zenoh-pico >"$OUTPUT_FILE_CONFLICT" 2>&1; then
    echo "expected zenoh and zenoh-pico to be mutually exclusive" >&2
    exit 1
fi

if ! grep -q "Only one concrete zenoh runtime backend can be enabled" "$OUTPUT_FILE_CONFLICT"; then
    cat "$OUTPUT_FILE_CONFLICT" >&2
    echo "runtime backend conflict failed without the expected diagnostic" >&2
    exit 1
fi
