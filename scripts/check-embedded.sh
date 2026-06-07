#!/usr/bin/env sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# Stable embedded verification entry point. Keep this aligned with
# docs/verification.md as embedded target checks expand.
exec "$SCRIPT_DIR/check-no-std.sh" "$@"
