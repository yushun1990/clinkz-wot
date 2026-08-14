#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
matrix="$root/docs/api-ownership.csv"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

expected_header='item,kind,defining_package,defining_module,visibility,public_path,compilation_cells,execution_models,resource_profiles,capability_roles,requirements,current_path,migration_action,status'
if [[ $(head -n 1 "$matrix") != "$expected_header" ]]; then
    echo "api ownership check: invalid header" >&2
    exit 1
fi

awk -F, '
    BEGIN {
        packages["clinkz-wot-foundation"] = packages["clinkz-wot-td"] = 1
        packages["clinkz-wot-core"] = packages["clinkz-wot-planning"] = 1
        packages["clinkz-wot-discovery"] = packages["clinkz-wot-servient"] = 1
        kinds["type"] = kinds["trait"] = kinds["registration"] = 1
        kinds["state_record"] = kinds["function"] = kinds["profile"] = 1
        migrations["add"] = migrations["keep"] = migrations["replace"] = 1
        migrations["relocate"] = migrations["remove"] = 1
        compilation["no-default"] = compilation["async-no-std"] = compilation["std"] = 1
        compilation["std-async"] = 1
        execution["all"] = execution["manual-poll"] = execution["host-async"] = 1
        resources["all"] = resources["application-static"] = 1
        resources["gateway-default-v1"] = resources["directory-client-default-v1"] = 1
        resources["benchmark-static-reference-v1"] = 1
        roles["all"] = roles["producer"] = roles["consumer"] = 1
        roles["directory-client"] = roles["gateway"] = 1
        prefixes["clinkz-wot-foundation"] = "clinkz_wot_foundation::"
        prefixes["clinkz-wot-td"] = "clinkz_wot_td::"
        prefixes["clinkz-wot-core"] = "clinkz_wot_core::"
        prefixes["clinkz-wot-planning"] = "clinkz_wot_planning::"
        prefixes["clinkz-wot-discovery"] = "clinkz_wot_discovery::"
        prefixes["clinkz-wot-servient"] = "clinkz_wot_servient::"
    }
    NF != 14 { fail("wrong column count", NF) }
    NR > 1 {
        for (i = 1; i <= NF; i++) if ($i == "") fail("empty field", i)
        if (items[$1]++) fail("duplicate item", $1)
        if (!packages[$3]) fail("unknown defining package", $3)
        if (!kinds[$2]) fail("unknown item kind", $2)
        if ($5 != "public" && $5 != "crate") fail("invalid visibility", $5)
        if ($5 == "public" && $6 == "-") fail("public item has no path", $1)
        if ($5 == "crate" && $6 != "-") fail("crate item has public path", $1)
        if ($5 == "public" && index($6, prefixes[$3]) != 1) fail("public path has wrong package", $6)
        if ($5 == "public" && public_paths[$6]++) fail("duplicate public path", $6)
        validate_list($7, compilation, "compilation cell")
        validate_list($8, execution, "execution model")
        validate_list($9, resources, "resource profile")
        validate_list($10, roles, "capability role")
        if (!migrations[$13]) fail("invalid migration action", $13)
        if ($14 != "frozen" && $14 != "removed") fail("ownership is not frozen", $1)
        if ($12 == "absent" && $13 != "add" && $14 != "removed") fail("invalid absent item", $1)
    }
    function fail(message, value) {
        printf "api ownership check: line %d: %s: %s\n", NR, message, value > "/dev/stderr"
        bad = 1
    }
    function validate_list(value, vocabulary, label, parts, count, i) {
        count = split(value, parts, "\\|")
        for (i = 1; i <= count; i++) if (!vocabulary[parts[i]]) fail("invalid " label, parts[i])
    }
    END { exit bad }
' "$matrix"

awk -F, 'NR > 1 && $12 != "absent" { print $1 "," $12 }' "$matrix" |
while IFS=, read -r item current_path; do
    if [[ ! -e "$root/$current_path" ]]; then
        echo "api ownership check: $item current path does not exist: $current_path" >&2
        exit 1
    fi
done

tail -n +2 "$root/docs/requirements.csv" | cut -d, -f1 | tr '|' '\n' >"$tmp/expressions"
: >"$tmp/requirements"
while IFS= read -r expression; do
    if [[ "$expression" =~ ^([A-Z][A-Z0-9-]*-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
        prefix=${BASH_REMATCH[1]}
        first=$((10#${BASH_REMATCH[2]}))
        last=$((10#${BASH_REMATCH[3]}))
        for ((i = first; i <= last; i++)); do printf '%s%03d\n' "$prefix" "$i"; done
    else
        printf '%s\n' "$expression"
    fi
done <"$tmp/expressions" | sort -u >"$tmp/requirements"

tail -n +2 "$matrix" | cut -d, -f11 | tr '|' '\n' | sort -u >"$tmp/referenced"
comm -23 "$tmp/referenced" "$tmp/requirements" >"$tmp/unknown"
if [[ -s "$tmp/unknown" ]]; then
    echo "api ownership check: unknown requirement references:" >&2
    sed 's/^/  /' "$tmp/unknown" >&2
    exit 1
fi

echo "api ownership check: schema, ownership paths, and requirement references valid"
