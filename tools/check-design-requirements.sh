#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
index="$root/docs/requirements.csv"
decomposition="$root/docs/spec/decomposition.csv"
expected_header='requirement,compilation_cells,execution_models,resource_profiles,capability_roles,owner_packages,evidence_kinds,evidence_key,source_path'
expected_decomposition_header='domain,target_path,sequence,depends_on,requirements'
expected_requirement_count=121
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "design requirement check: $*" >&2
    exit 1
}

if grep -Fqx 'status = "active"' \
    "$root/docs/spec/v5-authority-reset.toml"; then
    "$root/tools/check-v5-authority-reset-candidate.sh" >/dev/null
    echo "design requirement check: active v5 authority has 62 exact owners and 59 explicit inactive dispositions"
    exit 0
fi

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

[[ -f "$index" ]] || fail "missing docs/requirements.csv"
[[ -f "$decomposition" ]] || fail "missing docs/spec/decomposition.csv"

header=$(head -n 1 "$index")
[[ "$header" == "$expected_header" ]] || fail "unexpected CSV header"
decomposition_header=$(head -n 1 "$decomposition")
[[ "$decomposition_header" == "$expected_decomposition_header" ]] \
    || fail "unexpected decomposition CSV header"

awk -F, '
    NR > 1 && NF != 9 {
        printf "design requirement check: CSV line %d has %d columns; expected 9\n", NR, NF > "/dev/stderr"
        failed = 1
    }
    END { exit failed }
' "$index"

awk -F, '
    NR > 1 && NF != 5 {
        printf "design requirement check: decomposition CSV line %d has %d columns; expected 5\n", NR, NF > "/dev/stderr"
        failed = 1
    }
    END { exit failed }
' "$decomposition"

: >"$tmp/indexed-unsorted"
: >"$tmp/indexed-source-unsorted"
: >"$tmp/sources-unsorted"
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
        'workspace|clinkz-wot|clinkz-wot-foundation|clinkz-wot-td|clinkz-wot-core|clinkz-wot-planning|clinkz-wot-discovery|clinkz-wot-servient|clinkz-wot-codec-cbor'
    validate_list "$evidence_kinds" evidence_kinds "$line_number" \
        'inspection|compile|model|test|benchmark'

    [[ "$evidence_key" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]] \
        || fail "invalid evidence_key '$evidence_key' on CSV line $line_number"
    [[ "$source_path" != /* && "$source_path" != *..* ]] \
        || fail "source_path must be a repository-relative path on CSV line $line_number"
    [[ -f "$root/$source_path" ]] \
        || fail "source_path '$source_path' does not exist on CSV line $line_number"
    printf '%s\n' "$source_path" >>"$tmp/sources-unsorted"

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
                id=$(printf '%s%03d' "$prefix" "$i")
                printf '%s\n' "$id" >>"$tmp/indexed-unsorted"
                printf '%s,%s\n' "$id" "$source_path" >>"$tmp/indexed-source-unsorted"
            done
        elif [[ "$expression" =~ ^[A-Z][A-Z0-9-]*-[0-9]{3}$ ]]; then
            printf '%s\n' "$expression" >>"$tmp/indexed-unsorted"
            printf '%s,%s\n' "$expression" "$source_path" \
                >>"$tmp/indexed-source-unsorted"
        else
            fail "invalid requirement expression '$expression' on CSV line $line_number"
        fi
    done
done < <(tail -n +2 "$index")

: >"$tmp/defined-unsorted"
: >"$tmp/defined-source-unsorted"
sort -u "$tmp/sources-unsorted" >"$tmp/sources"
while IFS= read -r source_path; do
    sed -nE 's/.*`([A-Z][A-Z0-9-]+-[0-9]{3})`:.*/\1/p' \
        "$root/$source_path" >"$tmp/source-definitions"
    while IFS= read -r requirement; do
        [[ -n "$requirement" ]] || continue
        printf '%s\n' "$requirement" >>"$tmp/defined-unsorted"
        printf '%s,%s\n' "$requirement" "$source_path" \
            >>"$tmp/defined-source-unsorted"
    done <"$tmp/source-definitions"
done <"$tmp/sources"

[[ -s "$tmp/defined-unsorted" ]] \
    || fail "no stable requirements found in registered requirement sources"
[[ -s "$tmp/indexed-unsorted" ]] || fail "no stable requirements found in docs/requirements.csv"

sort "$tmp/defined-unsorted" >"$tmp/defined"
sort "$tmp/indexed-unsorted" >"$tmp/indexed"
sort "$tmp/defined-source-unsorted" >"$tmp/defined-source"
sort "$tmp/indexed-source-unsorted" >"$tmp/indexed-source"
sort "$tmp/defined-unsorted" | uniq -d >"$tmp/duplicate-definitions"
sort "$tmp/indexed-unsorted" | uniq -d >"$tmp/duplicate-index-entries"
comm -23 "$tmp/defined" "$tmp/indexed" >"$tmp/missing"
comm -13 "$tmp/defined" "$tmp/indexed" >"$tmp/unknown"
comm -23 "$tmp/indexed-source" "$tmp/defined-source" >"$tmp/misplaced-missing"
comm -13 "$tmp/indexed-source" "$tmp/defined-source" >"$tmp/misplaced-extra"

defined_count=$(wc -l <"$tmp/defined")
indexed_count=$(wc -l <"$tmp/indexed")
if [[ "$defined_count" -ne "$expected_requirement_count" ]]; then
    fail "registered sources define $defined_count requirements; expected $expected_requirement_count"
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
if [[ -s "$tmp/misplaced-missing" || -s "$tmp/misplaced-extra" ]]; then
    echo "requirement source mismatches (expected id,source):" >&2
    if [[ -s "$tmp/misplaced-missing" ]]; then
        sed 's/^/  missing /' "$tmp/misplaced-missing" >&2
    fi
    if [[ -s "$tmp/misplaced-extra" ]]; then
        sed 's/^/  extra   /' "$tmp/misplaced-extra" >&2
    fi
    failed=1
fi
((failed == 0)) || exit 1

: >"$tmp/decomposition-unsorted"
: >"$tmp/decomposition-target-unsorted"
: >"$tmp/decomposition-dependencies"
declare -A decomposition_sequences=()
declare -A target_domains=()

line_number=1
while IFS=, read -r domain target_path sequence depends_on requirements; do
    line_number=$((line_number + 1))

    [[ "$domain" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]] \
        || fail "invalid decomposition domain '$domain' on CSV line $line_number"
    [[ -z "${decomposition_sequences[$domain]+registered}" ]] \
        || fail "duplicate decomposition domain '$domain' on CSV line $line_number"
    [[ "$target_path" != /* && "$target_path" != *..* ]] \
        || fail "decomposition target_path must be repository-relative on CSV line $line_number"
    case "$target_path" in
        docs/design.md|docs/architecture/*.md|docs/spec/*.md) ;;
        *) fail "invalid decomposition target_path '$target_path' on CSV line $line_number" ;;
    esac
    [[ -z "${target_domains[$target_path]+registered}" ]] \
        || fail "decomposition target_path '$target_path' is shared by domains \
'${target_domains[$target_path]}' and '$domain'"
    [[ "$sequence" =~ ^(0|[1-9][0-9]*)$ ]] \
        || fail "invalid decomposition sequence '$sequence' on CSV line $line_number"
    [[ -n "$depends_on" ]] \
        || fail "empty decomposition depends_on on CSV line $line_number"
    [[ -n "$requirements" ]] \
        || fail "empty decomposition requirements on CSV line $line_number"

    decomposition_sequences["$domain"]=$sequence
    target_domains["$target_path"]=$domain
    printf '%s,%s\n' "$domain" "$depends_on" >>"$tmp/decomposition-dependencies"

    IFS='|' read -r -a expressions <<<"$requirements"
    [[ ${#expressions[@]} -gt 0 ]] \
        || fail "empty decomposition requirement expression on CSV line $line_number"
    for expression in "${expressions[@]}"; do
        if [[ "$expression" =~ ^([A-Z][A-Z0-9-]*-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
            prefix=${BASH_REMATCH[1]}
            first=$((10#${BASH_REMATCH[2]}))
            last=$((10#${BASH_REMATCH[3]}))
            ((first <= last)) \
                || fail "descending decomposition requirement range '$expression' \
on CSV line $line_number"
            for ((i = first; i <= last; i++)); do
                id=$(printf '%s%03d' "$prefix" "$i")
                printf '%s\n' "$id" >>"$tmp/decomposition-unsorted"
                printf '%s,%s\n' "$id" "$target_path" \
                    >>"$tmp/decomposition-target-unsorted"
            done
        elif [[ "$expression" =~ ^[A-Z][A-Z0-9-]*-[0-9]{3}$ ]]; then
            printf '%s\n' "$expression" >>"$tmp/decomposition-unsorted"
            printf '%s,%s\n' "$expression" "$target_path" \
                >>"$tmp/decomposition-target-unsorted"
        else
            fail "invalid decomposition requirement expression '$expression' \
on CSV line $line_number"
        fi
    done
done < <(tail -n +2 "$decomposition")

[[ ${#decomposition_sequences[@]} -gt 0 ]] \
    || fail "docs/spec/decomposition.csv has no data rows"

while IFS=, read -r domain depends_on; do
    [[ "$depends_on" == "-" ]] && continue

    IFS='|' read -r -a dependencies <<<"$depends_on"
    seen='|'
    for dependency in "${dependencies[@]}"; do
        [[ "$dependency" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]] \
            || fail "invalid dependency '$dependency' for decomposition domain '$domain'"
        case "$seen" in
            *"|$dependency|"*) fail "duplicate dependency '$dependency' for domain '$domain'" ;;
            *) seen+="$dependency|" ;;
        esac
        [[ -n "${decomposition_sequences[$dependency]+registered}" ]] \
            || fail "unknown dependency '$dependency' for decomposition domain '$domain'"
        [[ "$dependency" != "$domain" ]] \
            || fail "decomposition domain '$domain' depends on itself"
        dependency_sequence=${decomposition_sequences[$dependency]}
        domain_sequence=${decomposition_sequences[$domain]}
        ((dependency_sequence < domain_sequence)) \
            || fail "dependency '$dependency' must precede decomposition domain '$domain'"
    done
done <"$tmp/decomposition-dependencies"

sort "$tmp/decomposition-unsorted" >"$tmp/decomposition-index"
sort "$tmp/decomposition-target-unsorted" >"$tmp/decomposition-target"
sort "$tmp/decomposition-unsorted" | uniq -d >"$tmp/duplicate-decomposition-entries"
comm -23 "$tmp/indexed" "$tmp/decomposition-index" >"$tmp/decomposition-missing"
comm -13 "$tmp/indexed" "$tmp/decomposition-index" >"$tmp/decomposition-unknown"

failed=0
if [[ -s "$tmp/duplicate-decomposition-entries" ]]; then
    echo "duplicate decomposition entries:" >&2
    sed 's/^/  /' "$tmp/duplicate-decomposition-entries" >&2
    failed=1
fi
if [[ -s "$tmp/decomposition-missing" ]]; then
    echo "requirements missing from decomposition index:" >&2
    sed 's/^/  /' "$tmp/decomposition-missing" >&2
    failed=1
fi
if [[ -s "$tmp/decomposition-unknown" ]]; then
    echo "unknown requirements in decomposition index:" >&2
    sed 's/^/  /' "$tmp/decomposition-unknown" >&2
    failed=1
fi
((failed == 0)) || exit 1

join -t, "$tmp/indexed-source" "$tmp/decomposition-target" \
    >"$tmp/requirement-authority-map"

final_target_count=0
residual_count=0
amendment_count=0
while IFS=, read -r requirement source_path target_path; do
    if [[ "$source_path" == "$target_path" ]]; then
        [[ -f "$root/$target_path" ]] \
            || fail "final target '$target_path' for '$requirement' does not exist"
        final_target_count=$((final_target_count + 1))
    elif [[ "$source_path" == "docs/design.md" ]]; then
        residual_count=$((residual_count + 1))
    elif [[ "$source_path" == docs/amendments/*.md ]]; then
        amendment_count=$((amendment_count + 1))
    else
        fail "requirement '$requirement' has current source '$source_path' but final \
target '$target_path'; expected an existing final target, docs/design.md, or a \
registered amendment"
    fi
done <"$tmp/requirement-authority-map"

mapped_count=$((final_target_count + residual_count + amendment_count))
[[ "$mapped_count" -eq "$indexed_count" ]] \
    || fail "authority map classified $mapped_count requirements; expected $indexed_count"

echo "design requirement check: $defined_count requirements indexed; \
$final_target_count at final targets, $residual_count residual in docs/design.md, \
$amendment_count in registered amendments across ${#decomposition_sequences[@]} target domains"
