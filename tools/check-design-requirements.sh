#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
design="$root/docs/design.md"
index="$root/docs/requirements.csv"
expected_header='requirement,compilation_cells,execution_models,resource_profiles,capability_roles,owner_packages,evidence_kinds,evidence_key,source_path'
expected_requirement_count=112
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "design requirement check: $*" >&2
    exit 1
}

validate_list() {
    local value=$1
    local field=$2
    local line_number=$3
    local allowed=$4
    local token
    local seen='|'
    local -a tokens=()

    [[ -n "$value" ]] || fail "empty $field on CSV line $line_number"
    IFS='|' read -r -a tokens <<<"$value"
    [[ ${#tokens[@]} -gt 0 ]] || fail "empty $field on CSV line $line_number"

    for token in "${tokens[@]}"; do
        [[ -n "$token" ]] || fail "empty token in $field on CSV line $line_number"
        case "|$allowed|" in
            *"|$token|"*) ;;
            *) fail "unknown $field token '$token' on CSV line $line_number" ;;
        esac
        case "$seen" in
            *"|$token|"*) fail "duplicate $field token '$token' on CSV line $line_number" ;;
            *) seen+="$token|" ;;
        esac
    done
}

[[ -f "$design" ]] || fail "missing docs/design.md"
[[ -f "$index" ]] || fail "missing docs/requirements.csv"

header=$(head -n 1 "$index")
[[ "$header" == "$expected_header" ]] || fail "unexpected CSV header"

awk -F, '
    NR > 1 && NF != 9 {
        printf "design requirement check: CSV line %d has %d columns; expected 9\n", NR, NF > "/dev/stderr"
        failed = 1
    }
    END { exit failed }
' "$index"

: >"$tmp/indexed-unsorted"
line_number=1
while IFS=, read -r requirement compilation_cells execution_models resource_profiles \
    capability_roles owner_packages evidence_kinds evidence_key source_path; do
    line_number=$((line_number + 1))

    [[ -n "$requirement" ]] || fail "empty requirement expression on CSV line $line_number"
    validate_list "$compilation_cells" compilation_cells "$line_number" \
        'no-default|async-no-std|std'
    validate_list "$execution_models" execution_models "$line_number" \
        'manual-poll|host-async'
    validate_list "$resource_profiles" resource_profiles "$line_number" \
        'application-static|gateway-default-v1|directory-client-default-v1'
    validate_list "$capability_roles" capability_roles "$line_number" \
        'producer|consumer|directory-client|gateway'
    validate_list "$owner_packages" owner_packages "$line_number" \
        'workspace|clinkz-wot|clinkz-wot-foundation|clinkz-wot-td|clinkz-wot-core|clinkz-wot-protocol-bindings|clinkz-wot-discovery|clinkz-wot-servient|clinkz-wot-codec-cbor'
    validate_list "$evidence_kinds" evidence_kinds "$line_number" \
        'inspection|compile|model|test|benchmark'

    [[ "$evidence_key" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]] \
        || fail "invalid evidence_key '$evidence_key' on CSV line $line_number"
    [[ "$source_path" != /* && "$source_path" != *..* ]] \
        || fail "source_path must be a repository-relative path on CSV line $line_number"
    [[ -f "$root/$source_path" ]] \
        || fail "source_path '$source_path' does not exist on CSV line $line_number"

    IFS='|' read -r -a expressions <<<"$requirement"
    [[ ${#expressions[@]} -gt 0 ]] \
        || fail "empty requirement expression on CSV line $line_number"
    for expression in "${expressions[@]}"; do
        if [[ "$expression" =~ ^([A-Z][A-Z0-9-]*-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
            prefix=${BASH_REMATCH[1]}
            first=$((10#${BASH_REMATCH[2]}))
            last=$((10#${BASH_REMATCH[3]}))
            ((first <= last)) \
                || fail "descending requirement range '$expression' on CSV line $line_number"
            for ((i = first; i <= last; i++)); do
                printf '%s%03d\n' "$prefix" "$i" >>"$tmp/indexed-unsorted"
            done
        elif [[ "$expression" =~ ^[A-Z][A-Z0-9-]*-[0-9]{3}$ ]]; then
            printf '%s\n' "$expression" >>"$tmp/indexed-unsorted"
        else
            fail "invalid requirement expression '$expression' on CSV line $line_number"
        fi
    done
done < <(tail -n +2 "$index")

sed -nE 's/.*`([A-Z][A-Z0-9-]+-[0-9]{3})`:.*/\1/p' "$design" \
    >"$tmp/defined-unsorted"

[[ -s "$tmp/defined-unsorted" ]] || fail "no stable requirements found in docs/design.md"
[[ -s "$tmp/indexed-unsorted" ]] || fail "no stable requirements found in docs/requirements.csv"

sort "$tmp/defined-unsorted" >"$tmp/defined"
sort "$tmp/indexed-unsorted" >"$tmp/indexed"
sort "$tmp/defined-unsorted" | uniq -d >"$tmp/duplicate-definitions"
sort "$tmp/indexed-unsorted" | uniq -d >"$tmp/duplicate-index-entries"
comm -23 "$tmp/defined" "$tmp/indexed" >"$tmp/missing"
comm -13 "$tmp/defined" "$tmp/indexed" >"$tmp/unknown"

defined_count=$(wc -l <"$tmp/defined")
indexed_count=$(wc -l <"$tmp/indexed")
if [[ "$defined_count" -ne "$expected_requirement_count" ]]; then
    fail "docs/design.md defines $defined_count requirements; expected $expected_requirement_count"
fi
if [[ "$indexed_count" -ne "$expected_requirement_count" ]]; then
    fail "docs/requirements.csv expands to $indexed_count requirements; expected $expected_requirement_count"
fi

failed=0
if [[ -s "$tmp/duplicate-definitions" ]]; then
    echo "duplicate design definitions:" >&2
    sed 's/^/  /' "$tmp/duplicate-definitions" >&2
    failed=1
fi
if [[ -s "$tmp/duplicate-index-entries" ]]; then
    echo "duplicate index entries:" >&2
    sed 's/^/  /' "$tmp/duplicate-index-entries" >&2
    failed=1
fi
if [[ -s "$tmp/missing" ]]; then
    echo "missing from index:" >&2
    sed 's/^/  /' "$tmp/missing" >&2
    failed=1
fi
if [[ -s "$tmp/unknown" ]]; then
    echo "unknown in index:" >&2
    sed 's/^/  /' "$tmp/unknown" >&2
    failed=1
fi
((failed == 0)) || exit 1

echo "design requirement check: $defined_count requirements indexed with explicit profile axes"
