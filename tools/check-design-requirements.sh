#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
design="$root/docs/design.md"
index="$root/docs/requirements.csv"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

sed -nE 's/.*`([A-Z][A-Z0-9-]+-[0-9]{3})`:.*/\1/p' "$design" \
    | sort >"$tmp/defined"

tail -n +2 "$index" | cut -d, -f1 | tr '|' '\n' | while IFS= read -r expression; do
    if [[ "$expression" =~ ^([A-Z][A-Z0-9-]+-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
        prefix=${BASH_REMATCH[1]}
        first=$((10#${BASH_REMATCH[2]}))
        last=$((10#${BASH_REMATCH[3]}))
        for ((i = first; i <= last; i++)); do
            printf '%s%03d\n' "$prefix" "$i"
        done
    else
        printf '%s\n' "$expression"
    fi
done | sort >"$tmp/indexed"

if [[ -s "$tmp/defined" ]] && [[ -s "$tmp/indexed" ]]; then
    comm -23 "$tmp/defined" "$tmp/indexed" >"$tmp/missing"
    comm -13 "$tmp/defined" "$tmp/indexed" >"$tmp/unknown"
else
    echo "design requirement check: empty design or index" >&2
    exit 1
fi

duplicates=$(sort "$tmp/defined" | uniq -d)
if [[ -n "$duplicates" || -s "$tmp/missing" || -s "$tmp/unknown" ]]; then
    [[ -z "$duplicates" ]] || printf 'duplicate definitions:\n%s\n' "$duplicates" >&2
    [[ ! -s "$tmp/missing" ]] || { echo "missing from index:" >&2; sed 's/^/  /' "$tmp/missing" >&2; }
    [[ ! -s "$tmp/unknown" ]] || { echo "unknown in index:" >&2; sed 's/^/  /' "$tmp/unknown" >&2; }
    exit 1
fi

echo "design requirement check: $(wc -l <"$tmp/defined") requirements indexed"
