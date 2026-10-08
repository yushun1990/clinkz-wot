#!/usr/bin/env bash
set -euo pipefail

probe_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# Check the transitive target graph, excluding the intentional Host build tools.
# Package names avoid coupling this experiment to versions or checkout paths.
actual="$(cargo tree --locked --manifest-path "$probe_dir/Cargo.toml" \
  -p static-plan-probe --target thumbv6m-none-eabi \
  --edges normal --prefix none --format '{p}' --color never \
  | awk '{print $1}' | LC_ALL=C sort -u)"
expected=$'clinkz-wot-foundation\nruntime-contracts-probe\nstatic-plan-probe'

if [[ "$actual" != "$expected" ]]; then
  printf 'Unexpected Cortex-M0 normal dependency graph.\nExpected packages:\n%s\nActual packages:\n%s\n' \
    "$expected" "$actual" >&2
  exit 1
fi
printf 'Cortex-M0 normal dependency isolation passed:\n%s\n' "$actual"
