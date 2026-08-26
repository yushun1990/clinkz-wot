#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
registry="$root/docs/artifacts.csv"

expected_header='path,role,normativity,design_revision,schema_version,requirement_source'
if [[ $(head -n 1 "$registry") != "$expected_header" ]]; then
    echo "design artifact check: invalid artifact registry header" >&2
    exit 1
fi

awk -F, '''
    NF != 6 {
        printf "design artifact check: line %d has %d columns; expected 6\n", NR, NF > "/dev/stderr"
        bad = 1
    }
    NR > 1 && seen[$1]++ {
        printf "design artifact check: duplicate path: %s\n", $1 > "/dev/stderr"
        bad = 1
    }
    END { exit bad }
''' "$registry"

while IFS=, read -r relative _role _normativity _revision _schema requirement_source; do
    [[ "$relative" == "path" ]] && continue
    if [[ "$relative" = /* || "$relative" == *".."* || ! -e "$root/$relative" ]]; then
        echo "design artifact check: invalid or missing registered path: $relative" >&2
        exit 1
    fi
    if [[ "$requirement_source" = /* || "$requirement_source" == *".."* || ! -e "$root/$requirement_source" ]]; then
        echo "design artifact check: invalid or missing requirement source: $requirement_source" >&2
        exit 1
    fi
done <"$registry"

python3 "$root/tools/check-v5.1-authority.py"
cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- check
"$root/tools/check-api-ownership.sh"
"$root/tools/check-architecture-adrs.sh"
"$root/tools/check-directory-client-scope.sh"
"$root/tools/check-resource-limits.sh"
"$root/tools/check-legacy-api-absence.sh"
cargo run --locked --quiet --manifest-path "$root/tools/performance-harness/Cargo.toml" -- verify

echo "design artifact check: active v5.1 authority and stable cross-cutting invariants validated"
