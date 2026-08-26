#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

manifest="$root/docs/spec/v5-authority-reset.toml"
[[ -f "$root/docs/ADRs/core.org" ]] || {
    echo "architecture ADR candidate check: missing decision index" >&2
    exit 1
}

status=$(awk -F' *= *' '$1 == "status" { gsub(/"/, "", $2); print $2; exit }' "$manifest")
candidate_decision=$(awk -F' *= *' '$1 == "candidate_decision" { gsub(/"/, "", $2); print $2; exit }' "$manifest")
if [[ "$status" != "candidate" || "$candidate_decision" != "docs/ADRs/0019-consumer-one-shot-authority-entry.org" ]]; then
    echo "architecture ADR candidate check: v5.1 candidate metadata is missing" >&2
    exit 1
fi

find "$root/docs/ADRs" -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]-*.org' \
    -printf '%P\n' | sort >"$tmp/files"
: >"$tmp/accepted-ids"

while IFS= read -r relative; do
    number=${relative%%-*}
    id="ADR-$number"
    if grep -Fqx '#+status: Accepted' "$root/docs/ADRs/$relative"; then
        if ! grep -Fq "$id" "$root/docs/ADRs/core.org"; then
            echo "architecture ADR candidate check: decision index does not reference accepted $id" >&2
            exit 1
        fi
        printf '%s\n' "$id" >>"$tmp/accepted-ids"
    elif grep -Fqx '#+status: Proposed' "$root/docs/ADRs/$relative"; then
        if [[ "docs/ADRs/$relative" != "$candidate_decision" ]]; then
            echo "architecture ADR candidate check: unexpected proposed ADR $relative" >&2
            exit 1
        fi
        if grep -Fq "$id" "$root/docs/ADRs/core.org"; then
            echo "architecture ADR candidate check: proposed candidate $id is already indexed as accepted" >&2
            exit 1
        fi
    else
        echo "architecture ADR candidate check: $relative has unsupported status" >&2
        exit 1
    fi
done <"$tmp/files"

grep -oE 'ADR-[0-9]{4}' "$root/docs/ADRs/core.org" | sort -u >"$tmp/indexed-ids"
if ! cmp -s "$tmp/accepted-ids" "$tmp/indexed-ids"; then
    echo "architecture ADR candidate check: accepted decision files and index differ" >&2
    diff -u "$tmp/accepted-ids" "$tmp/indexed-ids" >&2 || true
    exit 1
fi

for projection in \
    'docs/design.md|ADR-0018' \
    'docs/design.md|ADR-0013' \
    'docs/design.md|ADR-0014' \
    'docs/spec/foundation.md|ADR-0015' \
    'docs/amendments/WP-100-time-domain-v1.md|ADR-0016' \
    'docs/spec/planning.md|ADR-0017' \
    'docs/spec/binding-spi.md|ADR-0006' \
    'docs/spec/binding-spi.md|ADR-0009' \
    'docs/spec/binding-spi.md|ADR-0010' \
    'docs/spec/binding-spi.md|ADR-0011' \
    'docs/spec/binding-spi.md|ADR-0012'; do
    path=${projection%%|*}
    id=${projection#*|}
    if ! grep -Fq "$id" "$root/$path"; then
        echo "architecture ADR candidate check: $path does not project $id" >&2
        exit 1
    fi
done

if ! grep -Fq 'ADR-0019' "$root/docs/spec/interaction-core.md" && \
   ! grep -Fq 'ADR-0019' "$root/docs/spec/planning.md" && \
   ! grep -Fq 'ADR-0019' "$root/docs/spec/binding-spi.md"; then
    echo "architecture ADR candidate check: candidate ADR is not projected by a candidate spec" >&2
    exit 1
fi

grep -Fq '`CRATE-DEPS-001`:' "$root/docs/architecture/20-module-boundaries.md" || {
    echo "architecture ADR candidate check: crate dependency boundary projection is missing" >&2
    exit 1
}
grep -Fq 'ADR-0014' "$root/docs/ADRs/0018-bounded-v5-normative-authority-reset.org" || {
    echo "architecture ADR candidate check: ADR-0014 supersession is not recorded" >&2
    exit 1
}

echo "architecture ADR candidate check: accepted decisions plus proposed ADR-0019 projection valid"
