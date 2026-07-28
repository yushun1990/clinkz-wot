#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/docs/spec/v5-authority-reset.toml"
index="$root/docs/requirements.csv"
gate="$root/docs/work-packages/property-read-architecture-gate.toml"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "v5 authority reset decision check: $*" >&2
    exit 1
}

[[ -f "$manifest" ]] || fail "missing transition manifest"
[[ -f "$index" ]] || fail "missing current requirement registry"

expand_expression() {
    local expression=$1
    local prefix first last i
    if [[ "$expression" =~ ^([A-Z][A-Z0-9-]*-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
        prefix=${BASH_REMATCH[1]}
        first=$((10#${BASH_REMATCH[2]}))
        last=$((10#${BASH_REMATCH[3]}))
        for ((i = first; i <= last; i++)); do
            printf '%s%03d\n' "$prefix" "$i"
        done
    elif [[ "$expression" =~ ^[A-Z][A-Z0-9-]*-[0-9]{3}$ ]]; then
        printf '%s\n' "$expression"
    else
        fail "invalid current requirement expression '$expression'"
    fi
}

tail -n +2 "$index" | while IFS=, read -r expressions _; do
    IFS='|' read -r -a parts <<<"$expressions"
    for expression in "${parts[@]}"; do
        expand_expression "$expression"
    done
done >"$tmp/current"

sed -nE 's/^[[:space:]]+"([A-Z][A-Z0-9-]*-[0-9]{3})",?$/\1/p' \
    "$manifest" >"$tmp/classified"

section_ids() {
    local section=$1
    awk -v header="[classification.$section]" '
        $0 == header { active = 1; next }
        active && /^\[/ { exit }
        active { print }
    ' "$manifest" \
        | sed -nE 's/^[[:space:]]+"([A-Z][A-Z0-9-]*-[0-9]{3})",?$/\1/p'
}

check_section() {
    local section=$1
    local expected=$2
    section_ids "$section" >"$tmp/$section"
    local actual
    actual=$(wc -l <"$tmp/$section")
    [[ "$actual" -eq "$expected" ]] \
        || fail "$section class has $actual requirements; expected $expected"
    local declared
    declared=$(awk -v header="[classification.$section]" '
        $0 == header { active = 1; next }
        active && /^\[/ { exit }
        active && /^expected_count = / { print $3; exit }
    ' "$manifest")
    [[ "$declared" == "$expected" ]] \
        || fail "$section class declares expected_count=$declared; expected $expected"
}

check_section indispensable 41
check_section property_read 21
check_section v1_deferred 34
check_section design_input 15
check_section retired 4
check_section redundant 6

[[ $(wc -l <"$tmp/classified") -eq 121 ]] \
    || fail "classification does not contain 121 requirement ids"
sort "$tmp/classified" | uniq -d >"$tmp/duplicates"
[[ ! -s "$tmp/duplicates" ]] \
    || fail "classification repeats: $(tr '\n' ' ' <"$tmp/duplicates")"
sort "$tmp/classified" >"$tmp/classified-sorted"
sort "$tmp/current" >"$tmp/current-sorted"
cmp -s "$tmp/current-sorted" "$tmp/classified-sorted" \
    || fail "classification does not exactly cover the current 121 requirements"

cat "$tmp/indispensable" "$tmp/property_read" | sort >"$tmp/active"
[[ $(wc -l <"$tmp/active") -eq 62 ]] \
    || fail "active classes do not contain 62 requirements"

awk '
    $0 == "requirements = [" { active = 1; next }
    active && $0 == "]" { exit }
    active { print }
' "$gate" \
    | sed -nE 's/^[[:space:]]+"([A-Z][A-Z0-9-]*-[0-9]{3})",?$/\1/p' \
    | sort >"$tmp/gate"
sort "$tmp/property_read" >"$tmp/property-read-sorted"
cmp -s "$tmp/gate" "$tmp/property-read-sorted" \
    || fail "Property Read class differs from the integration gate"

grep -Fqx 'Status: MIGRATED' \
    "$root/workspace/0015-normative-authority-reset.md" \
    || fail "workspace topic is not migrated"
grep -Fq 'docs/spec/v5-authority-reset.toml' \
    "$root/docs/ADRs/0018-bounded-v5-normative-authority-reset.org" \
    || fail "ADR-0018 does not register the exact transition manifest"
grep -Fq '| D7 | MIGRATED |' "$root/PLAN.md" \
    || fail "PLAN does not record migrated D7"
grep -Fq 'is abandoned as an activation' \
    "$root/docs/ADRs/0018-bounded-v5-normative-authority-reset.org" \
    || fail "Foundation D3 candidate disposition is missing"

echo "v5 authority reset decision check: 121 classified, 62 active, activation withheld"
