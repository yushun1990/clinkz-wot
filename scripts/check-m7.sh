#!/usr/bin/env sh
# Deprecated alias — use scripts/check-baseline.sh (v4.0 rename, phase-p4 §4.7).
exec "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/check-baseline.sh" "$@"
