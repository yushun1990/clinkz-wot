#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
registry="$root/docs/artifacts.csv"
gates="$root/docs/refactor-gates.csv"
mode=${1:-check}

expected_artifact_header='path,role,normativity,design_revision,schema_version,requirement_source'
if [[ "$(head -n 1 "$registry")" != "$expected_artifact_header" ]]; then
    echo "design artifact check: invalid artifact registry header" >&2
    exit 1
fi

awk -F, '
    NF != 6 { printf "design artifact check: line %d has %d columns\n", NR, NF > "/dev/stderr"; bad = 1 }
    NR > 1 && $4 != "4.6" { printf "design artifact check: revision mismatch on line %d\n", NR > "/dev/stderr"; bad = 1 }
    END { exit bad }
' "$registry"

duplicates=$(tail -n +2 "$registry" | cut -d, -f1 | sort | uniq -d)
if [[ -n "$duplicates" ]]; then
    printf 'design artifact check: duplicate artifact paths:\n%s\n' "$duplicates" >&2
    exit 1
fi

tail -n +2 "$registry" | cut -d, -f1 | while IFS= read -r path; do
    if [[ ! -e "$root/$path" ]]; then
        echo "design artifact check: missing active artifact: $path" >&2
        exit 1
    fi
done

expected_gate_header='gate,status,requirements,artifacts,checks,review_evidence'
if [[ "$(head -n 1 "$gates")" != "$expected_gate_header" ]]; then
    echo "design artifact check: invalid refactor gate header" >&2
    exit 1
fi

awk -F, '
    NF != 6 { printf "design artifact check: gate line %d has %d columns\n", NR, NF > "/dev/stderr"; bad = 1 }
    NR > 1 && $2 != "open" && $2 != "closed" {
        printf "design artifact check: invalid gate status on line %d\n", NR > "/dev/stderr"
        bad = 1
    }
    END { exit bad }
' "$gates"

gate_count=$(($(wc -l <"$gates") - 1))
if [[ "$gate_count" -ne 6 ]]; then
    echo "design artifact check: expected 6 gates; found $gate_count" >&2
    exit 1
fi

duplicates=$(tail -n +2 "$gates" | cut -d, -f1 | sort | uniq -d)
if [[ -n "$duplicates" ]]; then
    printf 'design artifact check: duplicate gates:\n%s\n' "$duplicates" >&2
    exit 1
fi

if [[ "$mode" == "--refactor-ready" ]]; then
    open_gates=$(awk -F, 'NR > 1 && $2 != "closed" { print $1 }' "$gates")
    if [[ -n "$open_gates" ]]; then
        printf 'design artifact check: refactor gates remain open:\n%s\n' "$open_gates" >&2
        exit 1
    fi
elif [[ "$mode" != "check" ]]; then
    echo "usage: tools/check-design-artifacts.sh [--refactor-ready]" >&2
    exit 2
fi

"$root/tools/check-design-requirements.sh"
"$root/tools/check-api-ownership.sh"
"$root/tools/check-directory-client-scope.sh"
"$root/tools/check-resource-limits.sh"
cargo run --locked --quiet --manifest-path "$root/tools/performance-harness/Cargo.toml" -- verify
cargo run --locked --quiet --manifest-path "$root/tools/design-check/Cargo.toml" -- check

echo "design artifact check: $gate_count gates registered"
