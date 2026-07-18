#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
matrix="$root/docs/api-ownership.csv"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

expected_header='item,kind,defining_package,defining_module,visibility,public_path,compilation_cells,execution_models,resource_profiles,capability_roles,requirements,current_path,migration_action,status'
actual_header=$(head -n 1 "$matrix")
if [[ "$actual_header" != "$expected_header" ]]; then
    echo "api ownership check: invalid header" >&2
    exit 1
fi

awk -F, '
    NF != 14 {
        printf "api ownership check: line %d has %d columns; expected 14\n", NR, NF > "/dev/stderr"
        bad = 1
    }
    NR > 1 {
        for (i = 1; i <= NF; i++) {
            if ($i == "") {
                printf "api ownership check: line %d has an empty field\n", NR > "/dev/stderr"
                bad = 1
            }
        }
    }
    END { exit bad }
' "$matrix"

tail -n +2 "$matrix" | cut -d, -f1 | sort >"$tmp/items"
duplicates=$(uniq -d "$tmp/items")
if [[ -n "$duplicates" ]]; then
    printf 'api ownership check: duplicate items:\n%s\n' "$duplicates" >&2
    exit 1
fi

awk -F, 'NR > 1 && $5 == "public" { print $6 }' "$matrix" | sort >"$tmp/paths"
duplicates=$(uniq -d "$tmp/paths")
if [[ -n "$duplicates" ]]; then
    printf 'api ownership check: duplicate public paths:\n%s\n' "$duplicates" >&2
    exit 1
fi

awk -F, '
    BEGIN {
        packages["clinkz-wot-foundation"] = 1
        packages["clinkz-wot-td"] = 1
        packages["clinkz-wot-core"] = 1
        packages["clinkz-wot-planning"] = 1
        packages["clinkz-wot-discovery"] = 1
        packages["clinkz-wot-servient"] = 1
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
    NR > 1 {
        if (!packages[$3]) fail("unknown defining package", $3)
        if (!kinds[$2]) fail("unknown item kind", $2)
        if ($5 != "public" && $5 != "crate") fail("invalid visibility", $5)
        if ($5 == "public" && $6 == "-") fail("public item has no path", $1)
        if ($5 == "crate" && $6 != "-") fail("crate item has public path", $1)
        if ($5 == "public" && index($6, prefixes[$3]) != 1) {
            fail("public path does not match defining package", $6)
        }
        validate_list($7, compilation, "compilation cell")
        validate_list($8, execution, "execution model")
        validate_list($9, resources, "resource profile")
        validate_list($10, roles, "capability role")
        if (!migrations[$13]) fail("invalid migration action", $13)
        if ($14 != "frozen" && $14 != "removed") fail("ownership is not frozen", $1)
        if ($12 == "absent" && $13 != "add" && $14 != "removed") {
            fail("absent item must use add migration", $1)
        }
        lower = tolower($0)
        if (lower ~ /(^|,)(tbd|todo|unknown|undecided|placeholder)(,|$)/) {
            fail("placeholder decision", $1)
        }
    }
    function fail(message, value) {
        printf "api ownership check: line %d: %s: %s\n", NR, message, value > "/dev/stderr"
        bad = 1
    }
    function validate_list(value, vocabulary, label, parts, count, i) {
        count = split(value, parts, "\\|")
        for (i = 1; i <= count; i++) {
            if (!vocabulary[parts[i]]) fail("invalid " label, parts[i])
        }
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

required_binding_paths=(
    'CleanupTransferEnvelope,clinkz_wot_core::CleanupTransferEnvelope,no-default|async-no-std|std'
    'CleanupTransferAcceptance,clinkz_wot_core::CleanupTransferAcceptance,no-default|async-no-std|std'
    'CleanupTransferTarget,clinkz_wot_core::CleanupTransferTarget,no-default|async-no-std|std'
    'BindingCancellationDisposition,clinkz_wot_core::BindingCancellationDisposition,no-default|async-no-std|std'
    'BindingCallSettlement,clinkz_wot_core::BindingCallSettlement,no-default|async-no-std|std'
    'RouteCleanupSuccessor,clinkz_wot_core::RouteCleanupSuccessor,no-default|async-no-std|std'
    'HostRouteCleanupSuccessor,clinkz_wot_core::HostRouteCleanupSuccessor,std'
    'ServingActivationAuthority,clinkz_wot_core::ServingActivationAuthority,no-default|async-no-std|std'
    'RouteAcceptLease,clinkz_wot_core::RouteAcceptLease,no-default|async-no-std|std'
    'RouteAcceptClaim,clinkz_wot_core::RouteAcceptClaim,no-default|async-no-std|std'
    'RouteAcceptClaimError,clinkz_wot_core::RouteAcceptClaimError,no-default|async-no-std|std'
    'RouteActivationPermit,clinkz_wot_core::RouteActivationPermit,no-default|async-no-std|std'
    'HostCommittedRouteGuard,clinkz_wot_core::HostCommittedRouteGuard,std'
    'host_committed_route_guard_try_state_pin_mut,clinkz_wot_core::HostCommittedRouteGuard::try_state_pin_mut,std'
    'HostShutdownRouteGuard,clinkz_wot_core::HostShutdownRouteGuard,std'
    'host_binding_call_box_as_pin_mut,clinkz_wot_core::HostBindingCallBox::as_pin_mut,std'
    'SubscriptionStopInput,clinkz_wot_core::SubscriptionStopInput,no-default|async-no-std|std'
    'SubscriptionDriverCleanupDisposition,clinkz_wot_core::SubscriptionDriverCleanupDisposition,no-default|async-no-std|std'
    'HostSubscriptionDriverBox,clinkz_wot_core::HostSubscriptionDriverBox,std'
    'poll_client_poll_subscription_start,clinkz_wot_core::PollClientBinding::poll_subscription_start,no-default|async-no-std|std'
    'poll_client_start_subscription_stop,clinkz_wot_core::PollClientBinding::start_subscription_stop,no-default|async-no-std|std'
)

for expected in "${required_binding_paths[@]}"; do
    item=${expected%%,*}
    actual=$(awk -F, -v item="$item" '$1 == item { print $1 "," $6 "," $7 }' "$matrix")
    if [[ "$actual" != "$expected" ]]; then
        echo "api ownership check: binding ownership projection mismatch for $item" >&2
        echo "  expected: $expected" >&2
        echo "  actual:   $actual" >&2
        exit 1
    fi
done

if awk -F, 'NR > 1 && ($1 ~ /start_stop_subscription|poll_start_subscription/ || $6 ~ /start_stop_subscription|poll_start_subscription/) { bad = 1 } END { exit !bad }' "$matrix"; then
    echo "api ownership check: a superseded subscription method spelling remains" >&2
    exit 1
fi

tail -n +2 "$root/docs/requirements.csv" | cut -d, -f1 | tr '|' '\n' \
    >"$tmp/requirement-expressions"
: >"$tmp/requirements-unsorted"
while IFS= read -r expression; do
    if [[ "$expression" =~ ^([A-Z][A-Z0-9-]*-)([0-9]{3})\.\.([0-9]{3})$ ]]; then
        prefix=${BASH_REMATCH[1]}
        first=$((10#${BASH_REMATCH[2]}))
        last=$((10#${BASH_REMATCH[3]}))
        for ((i = first; i <= last; i++)); do
            printf '%s%03d\n' "$prefix" "$i" >>"$tmp/requirements-unsorted"
        done
    else
        printf '%s\n' "$expression" >>"$tmp/requirements-unsorted"
    fi
done <"$tmp/requirement-expressions"
sort -u "$tmp/requirements-unsorted" >"$tmp/requirements"
tail -n +2 "$matrix" | cut -d, -f11 | tr '|' '\n' | sort -u >"$tmp/referenced"
comm -23 "$tmp/referenced" "$tmp/requirements" >"$tmp/unknown"
if [[ -s "$tmp/unknown" ]]; then
    echo "api ownership check: unknown requirement references:" >&2
    sed 's/^/  /' "$tmp/unknown" >&2
    exit 1
fi

echo "api ownership check: $(($(wc -l <"$matrix") - 1)) frozen items"
