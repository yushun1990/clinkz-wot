#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

[[ -f "$root/docs/ADRs/core.org" ]] || {
    echo "architecture ADR check: missing decision index" >&2
    exit 1
}

find "$root/docs/ADRs" -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]-*.org' \
    -printf '%P\n' | sort >"$tmp/files"
if [[ ! -s "$tmp/files" ]]; then
    echo "architecture ADR check: no accepted decisions" >&2
    exit 1
fi

: >"$tmp/actual-ids"
while IFS= read -r relative; do
    number=${relative%%-*}
    id="ADR-$number"
    if ! grep -Fqx '#+status: Accepted' "$root/docs/ADRs/$relative"; then
        echo "architecture ADR check: $relative is not accepted" >&2
        exit 1
    fi
    if ! grep -Fq "$id" "$root/docs/ADRs/core.org"; then
        echo "architecture ADR check: decision index does not reference $id" >&2
        exit 1
    fi
    printf '%s\n' "$id" >>"$tmp/actual-ids"
done <"$tmp/files"

grep -oE 'ADR-[0-9]{4}' "$root/docs/ADRs/core.org" | sort -u >"$tmp/indexed-ids"
if ! cmp -s "$tmp/actual-ids" "$tmp/indexed-ids"; then
    echo "architecture ADR check: decision files and index differ" >&2
    diff -u "$tmp/actual-ids" "$tmp/indexed-ids" >&2 || true
    exit 1
fi

for projection in \
    'docs/design.md|ADR-0018' \
    'docs/design.md|ADR-0019' \
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
        echo "architecture ADR check: $path does not project $id" >&2
        exit 1
    fi
done

grep -Fq '`CRATE-DEPS-001`:' "$root/docs/architecture/20-module-boundaries.md" || {
    echo "architecture ADR check: crate dependency boundary projection is missing" >&2
    exit 1
}
grep -Fq 'ADR-0014' "$root/docs/ADRs/0018-bounded-v5-normative-authority-reset.org" || {
    echo "architecture ADR check: ADR-0014 supersession is not recorded" >&2
    exit 1
}

echo "architecture ADR check: accepted decisions and current projections valid"
