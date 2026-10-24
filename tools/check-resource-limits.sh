#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
schema="$root/docs/resource-limits.csv"

expected_header='field,resource_kind,unit,scope,capability_roles,zero_semantics,gateway_default_v1,directory_client_default_v1,benchmark_static_reference_v1,requirements'
if [[ "$(head -n 1 "$schema")" != "$expected_header" ]]; then
    echo "resource limit check: invalid header" >&2
    exit 1
fi

awk -F, '
    NF != 10 {
        printf "resource limit check: line %d has %d columns; expected 10\n", NR, NF > "/dev/stderr"
        bad = 1
    }
    NR > 1 {
        for (i = 1; i <= 10; i++) {
            if ($i == "") {
                printf "resource limit check: line %d has an empty field\n", NR > "/dev/stderr"
                bad = 1
            }
        }
        if ($6 != "disabled" && $6 != "rendezvous") {
            printf "resource limit check: line %d has invalid zero semantics %s\n", NR, $6 > "/dev/stderr"
            bad = 1
        }
        applicable = 0
        for (i = 7; i <= 9; i++) {
            if ($i != "NA" && $i !~ /^[0-9]+$/) {
                printf "resource limit check: line %d has invalid profile value %s\n", NR, $i > "/dev/stderr"
                bad = 1
            }
            if ($i ~ /^[0-9]+$/) {
                applicable = 1
                if ($i == 0 && $6 != "rendezvous") {
                    printf "resource limit check: line %d uses zero outside rendezvous capacity\n", NR > "/dev/stderr"
                    bad = 1
                }
            }
        }
        if (!applicable) {
            printf "resource limit check: line %d is not applicable to any named profile\n", NR > "/dev/stderr"
            bad = 1
        }
        if ($5 !~ /(^|\|)(all|directory-client)(\||$)/ && $8 != "NA") {
            printf "resource limit check: line %d assigns a non-Directory field to DirectoryClientDefaultV1\n", NR > "/dev/stderr"
            bad = 1
        }
    }
    END { exit bad }
' "$schema"

duplicates=$(tail -n +2 "$schema" | cut -d, -f1 | sort | uniq -d)
if [[ -n "$duplicates" ]]; then
    printf 'resource limit check: duplicate fields:\n%s\n' "$duplicates" >&2
    exit 1
fi

expected_field_count=196
actual_field_count=$(($(wc -l <"$schema") - 1))
if [[ "$actual_field_count" -ne "$expected_field_count" ]]; then
    echo \
        "resource limit check: expected $expected_field_count fields; found $actual_field_count" \
        >&2
    exit 1
fi

# Freeze the complete original schema, not just a suffix of field names.
# Baseline: d4cf245eb1bf79e607c4851bba4b16a58e83a04e:docs/resource-limits.csv.
# Hash includes the header, every column of indices 0..194, row order, and LF.
# A change to this digest requires an explicit schema revision/migration under
# docs/spec/foundation.md; an append-only addition leaves it unchanged.
frozen_prefix_sha256=65e3edbcfbbc38b8fe16b4efd5db92d7964fc653ad71cc8e747ff056fe3b519e
actual_prefix_sha256=$(head -n 196 "$schema" | sha256sum | cut -d' ' -f1)
if [[ "$actual_prefix_sha256" != "$frozen_prefix_sha256" ]]; then
    echo "resource limit check: frozen metadata at indices 0..194 changed; explicit schema migration required" >&2
    exit 1
fi

v49_fields=(
    plan_sets_per_thing_max
    plan_sets_global_max
    plan_pins_per_plan_set_max
    plan_pins_global_max
    logical_plan_bytes_per_thing_max
    binding_artifacts_per_thing_max
    binding_artifacts_global_max
    binding_artifact_bytes_per_item_max
    binding_artifact_bytes_per_thing_max
    binding_artifact_bytes_global_max
    lazy_artifact_negative_bytes_per_item_max
    lazy_artifact_negative_bytes_global_max
    binding_compiler_cursor_bytes_per_item_max
    binding_compiler_cursor_bytes_global_max
    lazy_artifact_waiters_per_slot_max
    lazy_artifact_waiters_global_max
    plan_compile_work_units_per_step_max
    plan_reclaim_bytes_per_step_max
    binding_routes_per_thing_max
    binding_routes_global_max
    route_guard_bytes_per_item_max
    route_guard_bytes_per_thing_max
    route_guard_bytes_global_max
    route_readiness_tokens_per_thing_max
    route_readiness_tokens_global_max
    route_readiness_token_bytes_per_item_max
    route_readiness_token_bytes_global_max
    route_readiness_timeout_millis_max
    route_readiness_steps_max
    binding_ingress_items_per_route_max
    binding_ingress_items_per_binding_max
    binding_ingress_items_global_max
    binding_ingress_bytes_per_route_max
    binding_ingress_bytes_per_binding_max
    binding_ingress_bytes_global_max
    host_binding_call_bytes_per_item_max
    host_binding_call_bytes_per_binding_max
    host_binding_call_bytes_per_thing_max
    host_binding_call_bytes_global_max
    host_subscription_driver_bytes_per_item_max
    host_subscription_driver_bytes_per_thing_max
    host_subscription_driver_bytes_global_max
    binding_slot_state_bytes_per_item_max
    binding_slot_state_bytes_per_thing_max
    binding_slot_state_bytes_global_max
    binding_poll_temporary_bytes_per_call_max
    binding_poll_temporary_bytes_global_max
    binding_response_buffer_bytes_per_route_max
    binding_response_buffer_bytes_global_max
    binding_cancel_buffer_bytes_per_call_max
    binding_cancel_buffer_bytes_global_max
    cleanup_transfer_slots_global_max
    cleanup_transfer_bytes_global_max
    binding_wake_leases_global_max
    binding_reactor_queue_items_per_binding_max
    binding_reactor_queue_bytes_per_binding_max
)
mapfile -t actual_v49_fields < <(sed -n '141,196p' "$schema" | cut -d, -f1)
if [[ "${actual_v49_fields[*]}" != "${v49_fields[*]}" ]]; then
    echo "resource limit check: v4.9 append-only field order is not frozen at 139..194" >&2
    exit 1
fi

for field in "${v49_fields[@]}"; do
    if ! awk -F, -v field="$field" '
        $1 == field && $7 ~ /^[1-9][0-9]*$/ && $8 == "NA" \
            && $9 ~ /^[1-9][0-9]*$/ && $10 ~ /(^|\|)RES-LIMIT-001(\||$)/ {
            found = 1
        }
        END { exit !found }
    ' "$schema"; then
        echo "resource limit check: incomplete v4.9 projection for field: $field" >&2
        exit 1
    fi
done

if ! awk -F, '
    NR == 197 && $1 == "number_lexeme_bytes_max" \
        && $2 == "document" && $3 == "bytes" && $4 == "per-item" \
        && $5 == "consumer" && $6 == "disabled" \
        && $7 == "256" && $8 == "NA" && $9 == "64" \
        && $10 ~ /(^|\|)RES-LIMIT-001(\||$)/ \
        && $10 ~ /(^|\|)CONSTRAINED-WORK-001(\||$)/ { found = 1 }
    END { exit !found }
' "$schema"; then
    echo "resource limit check: bounded Number lexeme field is not frozen at index 195" >&2
    exit 1
fi

for field in compiled_runtime_bytes_per_thing_max compiled_runtime_bytes_global_max; do
    if ! awk -F, -v field="$field" '
        $1 == field && $10 ~ /(^|\|)PLAN-SET-001(\||$)/ \
            && $10 ~ /(^|\|)BIND-ROUTE-001(\||$)/ { found = 1 }
        END { exit !found }
    ' "$schema"; then
        echo "resource limit check: serving authority is not projected into $field" >&2
        exit 1
    fi
done

# General publication/resource ownership is checked by the v6 source map.
# The numeric schema below retains only its original restricted adapter meaning.

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
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
tail -n +2 "$schema" | cut -d, -f10 | tr '|' '\n' | sort -u >"$tmp/referenced"
comm -23 "$tmp/referenced" "$tmp/requirements" >"$tmp/unknown"
if [[ -s "$tmp/unknown" ]]; then
    echo "resource limit check: unknown requirement references:" >&2
    sed 's/^/  /' "$tmp/unknown" >&2
    exit 1
fi

required_fields=(
    accounting_batch_items_max
    accounting_idle_items_max
    accounting_reconcile_owners_per_step_max
    admission_peak_live_bytes_global_max
    cache_entries_global_max
    cache_generations_per_key_max
    cache_reclamation_items_per_step_max
    cleanup_bytes_max
    cleanup_items_max
    cleanup_work_items_per_step_max
    directory_response_buffer_bytes_global_max
    emission_binding_results_max
    engine_live_bytes_global_max
    fanout_cursors_global_max
    fanout_subscribers_per_step_max
    number_lexeme_bytes_max
    peak_live_bytes_per_admission_max
    query_bytes_max
    query_nesting_depth_max
    query_nodes_max
    query_terms_max
    "${v49_fields[@]}"
)
for field in "${required_fields[@]}"; do
    if ! awk -F, -v field="$field" 'NR > 1 && $1 == field { found = 1 } END { exit !found }' \
        "$schema"; then
        echo "resource limit check: missing required field: $field" >&2
        exit 1
    fi
done

check_manifest_profile() {
    local manifest=$1
    local expected=$2
    local actual
    actual=$(sed -nE 's/^resource_profile = "([^"]+)"$/\1/p' "$manifest")
    if [[ "$actual" != "$expected" ]]; then
        echo "resource limit check: $manifest references $actual; expected $expected" >&2
        exit 1
    fi
}

check_manifest_profile "$root/docs/performance/gateway.toml" GatewayDefaultV1
check_manifest_profile "$root/docs/performance/directory.toml" DirectoryClientDefaultV1
check_manifest_profile "$root/docs/performance/constrained.toml" BenchmarkStaticReferenceV1

if grep -Eq '^\[static_profile\]$' "$root/docs/performance/constrained.toml"; then
    echo "resource limit check: constrained manifest duplicates the resource profile" >&2
    exit 1
fi

for profile in GatewayDefaultV1 DirectoryClientDefaultV1 BenchmarkStaticReferenceV1; do
    if ! awk -F, -v profile="$profile" '
        NR > 1 && $1 == profile && $2 == "profile" && ($14 == "frozen" || $14 == "restricted") { found = 1 }
        END { exit !found }
    ' "$root/docs/api-ownership.csv"; then
        echo "resource limit check: retained profile is not implemented in API ownership: $profile" >&2
        exit 1
    fi
done

python3 -I - "$root" <<'PY'
import csv
import copy
import hashlib
import json
from pathlib import Path
import sys
import tomllib

root = Path(sys.argv[1])
overlay = tomllib.loads((root / "docs/resource-contracts/consumer-ownership-v2.toml").read_text())
assert overlay["schema_revision"] == 2
assert overlay["status"] in {"candidate", "accepted"}
assert overlay["operation"] == "consumer-property-read"
assert overlay["typed_input_interpretation_revision"] == 2
assert overlay["policy_identity_encoding"] == "complete-policy-v1"
assert overlay["numeric_schema"] == "docs/resource-limits.csv"
source = (root / overlay["numeric_schema"]).read_bytes()
assert hashlib.sha256(source).hexdigest() == overlay["numeric_schema_sha256"], "unversioned numeric schema change"
rows = list(csv.DictReader(source.decode().splitlines()))
assert len(rows) == overlay["field_count"] == 196
assert overlay["default_disposition"] == "unchanged"
assert overlay["capability_roles"] == ["consumer-only"]
assert overlay["execution_models"] == ["manual-poll"]
assert set(overlay["compilation_cells"]) == {"std", "no-default", "async-no-std"}
assert set(overlay["disposal_policies"]) == {"strict-local", "host-process-failure"}
assert (root / overlay["contract"]).is_file()
known = {row["field"]: row for row in rows}
seen = set()
groups = {group["id"] for group in overlay["group"]}
assert len(overlay["group"]) == len(groups) == 5
assert groups == {"engine-storage", "call-carrier", "callback-scratch", "cancellation", "cleanup-transfer"}
for group in overlay["group"]:
    assert group["disposition"] == "reinterpreted"
    for key in ("owner", "reservation_point", "diagnostic", "meaning"):
        assert group[key].strip(), f"missing {key} for {group['id']}"
    assert group["fields"]
    for field in group["fields"]:
        assert field in known, f"unknown resource row {field}"
        assert field not in seen, f"duplicate disposition for {field}"
        assert known[field]["unit"] == "bytes", f"non-byte reinterpretation {field}"
        seen.add(field)
assert len(seen) == 20 and len(set(known) - seen) == 176
vectors = overlay["value_identity_vector"]
assert len(vectors) == 3
assert {v["column"] for v in vectors} == {
    "gateway_default_v1", "directory_client_default_v1", "benchmark_static_reference_v1"
}
for vector in vectors:
    encoded = bytearray(b"clinkz-resource-values-v1\0")
    for row in rows:
        value = row[vector["column"]]
        encoded.extend(b"\0" if value == "NA" else b"\1" + int(value).to_bytes(8, "little"))
    assert hashlib.sha256(encoded).hexdigest() == vector["sha256"], "configuration digest encoding drift"
policy_lengths = {"registration": 2, "call": 2, "input": 4, "result": 2, "work": 7, "timeouts": 2}
fixture = overlay["policy_identity_fixture"]
assert set(fixture) == set(policy_lengths) | {"scratch_bytes"}

def policy_key(vector, settings):
    assert set(settings) == set(fixture), "incomplete policy constructor settings"
    assert vector["compilation_cell"] in overlay["compilation_cells"]
    assert vector["disposal_policy"] in overlay["disposal_policies"]
    assert vector["disposal_policy"] != "host-process-failure" or vector["compilation_cell"] == "std"
    coordinates = [str(overlay["schema_revision"]), str(overlay["typed_input_interpretation_revision"]),
                   overlay["operation"], "consumer-only", "manual-poll", vector["compilation_cell"],
                   vector["disposal_policy"], str(vector["profile_origin"])]
    encoded = bytearray(b"clinkz-consumer-policy-identity-v1\0")
    for coordinate in coordinates:
        raw = coordinate.encode()
        encoded.extend(len(raw).to_bytes(2, "little") + raw)
    value = next(v["sha256"] for v in vectors if v["column"] == vector["value_column"])
    encoded.extend(bytes.fromhex(value))
    for operand, length in policy_lengths.items():
        assert len(settings[operand]) == length, f"wrong {operand} operand shape"
        for index, amount in enumerate(settings[operand]):
            width = 4 if operand in {"registration", "call"} and index == 0 else 8
            assert type(amount) is int and 0 <= amount < 1 << (width * 8)
            encoded.extend(amount.to_bytes(width, "little"))
    scratch = settings["scratch_bytes"]
    assert type(scratch) is int and 0 <= scratch < 1 << 64
    encoded.extend(scratch.to_bytes(8, "little"))
    return hashlib.sha256(encoded).hexdigest()

policy_keys = set()
policy_vectors = {v["id"]: v for v in overlay["policy_identity_vector"]}
assert len(overlay["policy_identity_vector"]) == len(policy_vectors) == 5
assert set(policy_vectors) == {"std-strict", "std-host", "scratch-only", "result-only", "timeout-only"}
for vector in policy_vectors.values():
    assert set(vector["overrides"]) <= set(fixture), "unknown policy operand"
    key = policy_key(vector, fixture | vector["overrides"])
    assert key == vector["sha256"] and key not in policy_keys, "policy identity alias/encoding drift"
    policy_keys.add(key)
baseline = policy_vectors["std-strict"]
assert baseline["overrides"] == {}
for case, operand in [("scratch-only", "scratch_bytes"), ("result-only", "result"), ("timeout-only", "timeouts")]:
    vector = policy_vectors[case]
    for coordinate in ["compilation_cell", "disposal_policy", "profile_origin", "value_column"]:
        assert vector[coordinate] == baseline[coordinate], "negative vector changed table/coordinates"
    assert set(vector["overrides"]) == {operand}
    assert vector["overrides"][operand] != fixture[operand]

# Independently perturb every scalar so a dropped constructor setting cannot
# retain the complete key even when all 196 numeric schema values are unchanged.
mutations = 0
for operand, length in policy_lengths.items():
    for index in range(length):
        changed = copy.deepcopy(fixture)
        changed[operand][index] += -1 if operand == "input" and index == 1 else 1
        assert policy_key(baseline, changed) != baseline["sha256"], f"policy key omits {operand}[{index}]"
        mutations += 1
changed = fixture | {"scratch_bytes": fixture["scratch_bytes"] + 1}
assert policy_key(baseline, changed) != baseline["sha256"], "policy key omits scratch"
assert mutations + 1 == 20

# 0086's closed-compiler correction adds binding eligibility vectors. It does
# not reinterpret numeric rows or the independently versioned engine policy.
assert overlay["binding_configuration_encoding"] == "closed-consumer-binding-v1"
binding_fixture = overlay["binding_configuration_fixture"]
assert set(binding_fixture) == {"contract", "compilation_cell", "target_capacity", "private_policy", "body_bytes", "media_capacity"}

def strict_binding_key(key):
    return (len(key) == 32 and key[:3] == b"ct\1" and 1 <= key[3] <= 64
            and key[4:] == bytes(28))

def binding_key(settings):
    assert set(settings) == set(binding_fixture)
    contract = {"ConsumerOwnedV2": 1, "ConsumerOwnedHostV2": 2}[settings["contract"]]
    cell = {"no-default": 0, "async-no-std": 1, "std": 2}[settings["compilation_cell"]]
    assert contract != 2 or cell == 2, "Host disposal outside std"
    capacity = settings["target_capacity"]
    assert type(capacity) is int and 1 <= capacity <= 64
    private = bytes.fromhex(settings["private_policy"])
    assert len(private) == 32
    encoded = bytearray(b"clinkz-consumer-binding-configuration-v1\0")
    encoded.extend(bytes([contract, cell, capacity]) + b"clinkz-target-v1" + private)
    for operand in ("body_bytes", "media_capacity"):
        value = settings[operand]
        assert type(value) is int and 0 < value < 1 << 64
        encoded.extend(value.to_bytes(8, "little"))
    key = hashlib.sha256(encoded).digest()
    assert not strict_binding_key(key), "owned key aliases legacy strict namespace"
    return key.hex()

binding_vectors = {v["id"]: v for v in overlay["binding_configuration_vector"]}
expected = {"baseline": None, "capacity-only": "target_capacity", "contract-only": "contract",
            "cell-no-default": "compilation_cell", "cell-async": "compilation_cell",
            "private-policy-only": "private_policy", "body-only": "body_bytes", "media-only": "media_capacity"}
assert len(overlay["binding_configuration_vector"]) == len(binding_vectors) == len(expected)
assert set(binding_vectors) == set(expected)
binding_keys = set()
for name, operand in expected.items():
    vector = binding_vectors[name]
    overrides = vector["overrides"]
    assert set(overrides) == (set() if operand is None else {operand})
    if operand is not None:
        assert overrides[operand] != binding_fixture[operand]
    key = binding_key(binding_fixture | overrides)
    assert key == vector["sha256"] and key not in binding_keys, "binding configuration alias/encoding drift"
    binding_keys.add(key)
assert binding_keys.isdisjoint(policy_keys), "binding key conflated with engine policy"
for capacity in range(1, 65):
    assert strict_binding_key(b"ct\1" + bytes([capacity]) + bytes(28))
for key in (bytes(32), b"ct\1\0" + bytes(28), b"ct\1\x41" + bytes(28), b"ct\2\x40" + bytes(28),
            b"ct\1\x40" + bytes(27) + b"\1"):
    assert not strict_binding_key(key), "strict namespace exclusion is too broad"
for overrides in ({"target_capacity": 0}, {"target_capacity": 65}, {"body_bytes": 0},
                  {"media_capacity": 0}, {"body_bytes": 1 << 64},
                  {"contract": "ConsumerOwnedHostV2", "compilation_cell": "no-default"},
                  {"contract": "ConsumerOwnedHostV2", "compilation_cell": "async-no-std"}):
    try:
        binding_key(binding_fixture | overrides)
    except AssertionError:
        pass
    else:
        raise AssertionError(f"unsupported binding configuration accepted: {overrides}")
# Freeze the interpreted schema as well as the numeric table. Lifecycle status
# is not accounting meaning; changing metadata requires an explicit migration.
# The 0086 amendment adds the binding encoding/vectors only; all row and
# complete-policy projections remain byte-for-byte unchanged.
metadata = {key: value for key, value in overlay.items() if key != "status"}
metadata_digest = hashlib.sha256(json.dumps(metadata, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
assert metadata_digest == "255d45b38440415d32a9dfa9c6ff008ad39d175900f9153ba415b7518590869a", "schema-2 metadata changed without an explicit migration"
print("resource ownership candidate: 20 versioned byte rows; 176 unchanged; 3 value and 5 complete-policy vectors; 20 operand mutations")
print("Consumer binding configuration: 8 canonical vectors; strict namespace and unsupported-cell/operand controls")
PY

echo "resource limit check: $(($(wc -l <"$schema") - 1)) fields and 3 profiles"
