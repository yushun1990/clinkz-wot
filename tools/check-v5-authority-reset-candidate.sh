#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/docs/spec/v5-authority-reset.toml"
carry_forward="$root/docs/evidence/v5-authority-carry-forward.toml"
artifact_carry_forward="$root/docs/spec/v5-artifact-carry-forward.toml"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
    echo "v5 authority reset activation check: $*" >&2
    exit 1
}

"$root/tools/check-v5-authority-reset-decision.sh" >/dev/null

grep -Fqx 'status = "active"' "$manifest" \
    || fail "transition manifest does not identify active v5 authority"
grep -Fqx 'status = "independently-reviewed-and-integrated"' "$manifest" \
    || fail "candidate review/integration state is incomplete"
grep -Fqx 'authority_activation = "active"' "$manifest" \
    || fail "candidate authority is not active"
grep -Fqx 'runtime_or_public_api_changes_allowed = false' "$manifest" \
    || fail "candidate does not prohibit runtime/public API changes"
grep -Fqx 'independent_review_required = true' "$manifest" \
    || fail "candidate does not require independent review"
grep -Fqx 'separate_integration_required = true' "$manifest" \
    || fail "candidate does not require separate integration"

expected_artifact_header='path,role,normativity,design_revision,schema_version,requirement_source'
[[ $(head -n 1 "$root/docs/artifacts.csv") == "$expected_artifact_header" ]] \
    || fail "artifact registry header is invalid"
awk -F, 'NR > 1 && NF != 6 { exit 1 }' "$root/docs/artifacts.csv" \
    || fail "artifact registry contains a malformed row"
cut -d, -f1 "$root/docs/artifacts.csv" | tail -n +2 | sort | uniq -d \
    >"$tmp/duplicate-artifacts"
[[ ! -s "$tmp/duplicate-artifacts" ]] \
    || fail "artifact registry contains duplicate paths"
while IFS=, read -r path _; do
    [[ -e "$root/$path" ]] || fail "registered artifact '$path' does not exist"
done < <(tail -n +2 "$root/docs/artifacts.csv")

section_ids() {
    local section=$1
    awk -v header="[classification.$section]" '
        $0 == header { active = 1; next }
        active && /^\[/ { exit }
        active { print }
    ' "$manifest" \
        | sed -nE 's/^[[:space:]]+"([A-Z][A-Z0-9-]*-[0-9]{3})",?$/\1/p'
}

cat <(section_ids indispensable) <(section_ids property_read) \
    | sort >"$tmp/classified-active"
cat <(section_ids v1_deferred) <(section_ids design_input) \
    <(section_ids retired) <(section_ids redundant) \
    | sort >"$tmp/classified-inactive"

awk '
    function flush() {
        if (path != "") {
            print path "," expected > counts
        }
    }
    BEGIN { counts = ARGV[2]; ARGV[2] = "" }
    $0 == "[[active_source]]" {
        flush()
        registered = 1
        path = ""
        expected = ""
        in_requirements = 0
        next
    }
    /^path = "/ {
        value = $0
        sub(/^path = "/, "", value)
        sub(/"$/, "", value)
        path = value
        next
    }
    /^expected_count = / {
        expected = $3
        next
    }
    registered && /^requirements = \[/ { in_requirements = 1; next }
    in_requirements && /^\]/ { in_requirements = 0; next }
    in_requirements {
        value = $0
        if (match(value, /"[A-Z][A-Z0-9-]*-[0-9][0-9][0-9]"/)) {
            id = substr(value, RSTART + 1, RLENGTH - 2)
            print id "," path
        }
    }
    END { flush() }
' "$manifest" "$tmp/source-counts" >"$tmp/declared-pairs"

[[ $(wc -l <"$tmp/declared-pairs") -eq 62 ]] \
    || fail "active sources do not declare exactly 62 requirements"
cut -d, -f1 "$tmp/declared-pairs" | sort >"$tmp/declared-active"
cmp -s "$tmp/classified-active" "$tmp/declared-active" \
    || fail "active source declarations differ from the two active classes"
sort "$tmp/declared-active" | uniq -d >"$tmp/duplicate-declarations"
[[ ! -s "$tmp/duplicate-declarations" ]] \
    || fail "an active requirement is assigned to multiple sources"

: >"$tmp/defined-pairs"
while IFS=, read -r source expected; do
    [[ -n "$source" && "$expected" =~ ^[1-9][0-9]*$ ]] \
        || fail "invalid active source registration '$source,$expected'"
    [[ -f "$root/$source" ]] || fail "missing active source '$source'"
    if ! awk -F, -v source="$source" '
        $1 == source && $4 == "5.0" { found++ }
        END { exit found != 1 }
    ' "$root/docs/artifacts.csv"; then
        fail "$source is not registered exactly once as a v5 artifact"
    fi
    sed -nE 's/.*`([A-Z][A-Z0-9-]+-[0-9]{3})`:.*/\1/p' "$root/$source" \
        >"$tmp/source-definitions"
    [[ $(wc -l <"$tmp/source-definitions") -eq "$expected" ]] \
        || fail "$source defines $(wc -l <"$tmp/source-definitions") active ids; expected $expected"
    while IFS= read -r id; do
        printf '%s,%s\n' "$id" "$source" >>"$tmp/defined-pairs"
    done <"$tmp/source-definitions"
done <"$tmp/source-counts"

sort "$tmp/declared-pairs" >"$tmp/declared-pairs-sorted"
sort "$tmp/defined-pairs" >"$tmp/defined-pairs-sorted"
cmp -s "$tmp/declared-pairs-sorted" "$tmp/defined-pairs-sorted" \
    || fail "active definitions do not exactly match their registered owners"
cut -d, -f1 "$tmp/defined-pairs" | sort | uniq -d >"$tmp/duplicate-definitions"
[[ ! -s "$tmp/duplicate-definitions" ]] \
    || fail "an active requirement is defined more than once"
comm -12 "$tmp/declared-active" "$tmp/classified-inactive" \
    >"$tmp/active-inactive-overlap"
[[ ! -s "$tmp/active-inactive-overlap" ]] \
    || fail "an inactive requirement appears in the active owner set"

[[ $(wc -l <"$root/docs/design.md") -lt 300 ]] \
    || fail "docs/design.md is no longer a concise authority manifest"
[[ ! -e "$root/docs/spec/decomposition.csv" ]] \
    || fail "superseded D3 decomposition still exists in the candidate"
! grep -Fq 'docs/spec/decomposition.csv,' "$root/docs/artifacts.csv" \
    || fail "superseded D3 decomposition remains registered"

extract_requirement_ids() {
    local source=$1
    awk '
        /requirement_ids = \[/ { active = 1 }
        active {
            line = $0
            while (match(line, /"[A-Z][A-Z0-9-]*-[0-9][0-9][0-9]"/)) {
                print substr(line, RSTART + 1, RLENGTH - 2)
                line = substr(line, RSTART + RLENGTH)
            }
            if ($0 ~ /\]/) active = 0
        }
    ' "$source"
}

: >"$tmp/evidence-original"
for source in "$root"/docs/evidence/WP-*.toml; do
    relative=${source#"$root/"}
    while IFS= read -r id; do
        printf '%s,%s\n' "$relative" "$id" >>"$tmp/evidence-original"
    done < <(extract_requirement_ids "$source")
done

awk '
    /^source = "/ {
        source = $0
        sub(/^source = "/, "", source)
        sub(/"$/, "", source)
        next
    }
    /^active_claim_ids = \[/ { mode = "active" }
    /^inactive_claim_ids = \[/ { mode = "inactive" }
    mode != "" {
        line = $0
        while (match(line, /"[A-Z][A-Z0-9-]*-[0-9][0-9][0-9]"/)) {
            id = substr(line, RSTART + 1, RLENGTH - 2)
            print source "," id > all_claims
            print id > (mode == "active" ? active_claims : inactive_claims)
            line = substr(line, RSTART + RLENGTH)
        }
        if ($0 ~ /\]/) mode = ""
    }
' all_claims="$tmp/evidence-carry" active_claims="$tmp/evidence-active" \
    inactive_claims="$tmp/evidence-inactive" "$carry_forward"

sort -u "$tmp/evidence-original" >"$tmp/evidence-original-sorted"
sort -u "$tmp/evidence-carry" >"$tmp/evidence-carry-sorted"
cmp -s "$tmp/evidence-original-sorted" "$tmp/evidence-carry-sorted" \
    || fail "carry-forward manifest does not disposition every completed evidence claim"
sort -u "$tmp/evidence-active" >"$tmp/evidence-active-sorted"
sort -u "$tmp/evidence-inactive" >"$tmp/evidence-inactive-sorted"
comm -23 "$tmp/evidence-active-sorted" "$tmp/classified-active" \
    >"$tmp/bad-active-evidence"
[[ ! -s "$tmp/bad-active-evidence" ]] \
    || fail "carry-forward treats inactive evidence as active"
comm -23 "$tmp/evidence-inactive-sorted" "$tmp/classified-inactive" \
    >"$tmp/bad-inactive-evidence"
[[ ! -s "$tmp/bad-inactive-evidence" ]] \
    || fail "carry-forward gives an invalid inactive disposition"

awk -F, 'NR > 1 {
    if ($2 != "open" && $2 != "closed") exit 2
    n = split($3, ids, "|")
    if (n == 0 || $3 == "") exit 3
    for (i = 1; i <= n; i++) print ids[i]
}' "$root/docs/refactor-gates.csv" | sort -u >"$tmp/gate-requirements" \
    || fail "v5 refactor gate registry is malformed"
comm -23 "$tmp/gate-requirements" "$tmp/classified-active" \
    >"$tmp/inactive-gate-requirements"
[[ ! -s "$tmp/inactive-gate-requirements" ]] \
    || fail "a v5 gate still cites inactive requirements"
[[ $(tail -n +2 "$root/docs/refactor-gates.csv" | wc -l) -eq 6 ]] \
    || fail "refactor gate registry does not contain six gates"

awk '
    function flush() {
        if (path != "") print family "," path "," digest
    }
    $0 == "[[artifact]]" { flush(); family = path = digest = ""; next }
    /^family = "/ { family = $0; sub(/^family = "/, "", family); sub(/"$/, "", family); next }
    /^path = "/ { path = $0; sub(/^path = "/, "", path); sub(/"$/, "", path); next }
    /^sha256 = "/ { digest = $0; sub(/^sha256 = "/, "", digest); sub(/"$/, "", digest); next }
    END { flush() }
' "$artifact_carry_forward" >"$tmp/carried-artifacts"
[[ $(wc -l <"$tmp/carried-artifacts") -eq 16 ]] \
    || fail "machine-artifact carry-forward does not contain 16 exact files"
for family in api-ownership state-model resource-policy requirement-metadata \
    implementation-gates governance performance-input work-package-dag \
    property-read-integration-gate; do
    grep -Eq "^$family," "$tmp/carried-artifacts" \
        || fail "machine-artifact carry-forward is missing family '$family'"
done
while IFS=, read -r family path expected_digest; do
    [[ -f "$root/$path" ]] || fail "carried artifact '$path' does not exist"
    actual_digest=$(sha256sum "$root/$path" | cut -d' ' -f1)
    [[ "$actual_digest" == "$expected_digest" ]] \
        || fail "carried artifact '$path' changed without disposition update"
done <"$tmp/carried-artifacts"

base=$(sed -nE 's/^candidate_base_ref = "([0-9a-f]{40})"$/\1/p' "$manifest")
candidate=$(sed -nE 's/^candidate_ref = "([0-9a-f]{40})"$/\1/p' "$manifest")
attestation=$(sed -nE 's/^review_attestation_ref = "([0-9a-f]{40})"$/\1/p' \
    "$manifest")
integration=$(sed -nE 's/^integration_ref = "([0-9a-f]{40})"$/\1/p' "$manifest")
rollback=$(sed -nE 's/^activation_rollback_ref = "([0-9a-f]{40})"$/\1/p' \
    "$manifest")
for ref in "$base" "$candidate" "$attestation" "$integration" "$rollback"; do
    [[ -n "$ref" ]] || fail "activation history ref is missing"
    git -C "$root" cat-file -e "$ref^{commit}" \
        || fail "activation history ref $ref does not resolve"
done
[[ "$rollback" == "$attestation" ]] \
    || fail "activation rollback is not the exact pre-integration mainline"
[[ $(git -C "$root" rev-list --parents -n 1 "$candidate") \
    == "$candidate $base" ]] \
    || fail "candidate is not the single child of its frozen base"
[[ $(git -C "$root" rev-list --parents -n 1 "$integration") \
    == "$integration $attestation $candidate" ]] \
    || fail "activation checkpoint does not have exact attestation/candidate parents"
git -C "$root" merge-base --is-ancestor "$integration" HEAD \
    || fail "activation checkpoint is not an ancestor of HEAD"

awk '
    /^expected_changed_paths = \[/ { active = 1; next }
    active && /^\]/ { exit }
    active {
        value = $0
        if (match(value, /"[^"]+"/)) print substr(value, RSTART + 1, RLENGTH - 2)
    }
' "$manifest" | sort >"$tmp/expected-changed"
git -C "$root" diff --name-only "$base..$candidate" | sort >"$tmp/changed"
cmp -s "$tmp/changed" "$tmp/expected-changed" \
    || fail "candidate changed paths differ from the exact registered boundary"
if grep -E '(^|/)(Cargo\.(toml|lock)|[^/]+\.rs)$|^(foundation|core|td|servient|discovery|protocol-bindings|codecs)/' \
    "$tmp/changed" >"$tmp/runtime-paths"; then
    fail "candidate changes runtime or Cargo paths: $(tr '\n' ' ' <"$tmp/runtime-paths")"
fi
git -C "$root" diff --check "$base..$candidate" \
    || fail "candidate fails diff hygiene"
while IFS= read -r path; do
    git -C "$root" diff --quiet "$candidate" "$integration" -- "$path" \
        || fail "activation altered reviewed candidate path '$path'"
done <"$tmp/expected-changed"

git -C "$root" diff-tree --no-commit-id --name-only -r "$attestation" \
    | sort >"$tmp/attestation-paths"
printf '%s\n' \
    'docs/artifacts.csv' \
    'docs/audits/D7-v5-authority-reset-review.toml' \
    | sort >"$tmp/expected-attestation-paths"
cmp -s "$tmp/attestation-paths" "$tmp/expected-attestation-paths" \
    || fail "review attestation commit changed paths outside its exact record"
review_object="$attestation:docs/audits/D7-v5-authority-reset-review.toml"
git -C "$root" show "$review_object" >"$tmp/review"
for field in \
    'status = "passed"' \
    'reviewer_attestation_kind = "independent-root-session"' \
    'reviewer_id = "codex-agent:/root"' \
    "reviewed_ref = \"$candidate\"" \
    'reviewed_path_count = 27' \
    'runtime_or_public_api_changes = false'; do
    grep -Fqx "$field" "$tmp/review" \
        || fail "review attestation is missing exact field: $field"
done

candidate_audit="$root/docs/audits/D7-v5-authority-reset-candidate.toml"
for field in \
    'status = "passed-and-activated"' \
    'authority_activation = "active"' \
    "candidate_ref = \"$candidate\"" \
    "activation_rollback_ref = \"$rollback\"" \
    "integration_ref = \"$integration\"" \
    'status = "passed"' \
    "attestation_ref = \"$attestation\"" \
    'reviewer = "codex-agent:/root"'; do
    grep -Fqx "$field" "$candidate_audit" \
        || fail "candidate audit is missing exact activated field: $field"
done

grep -Fq 'workspace issue 0014' "$root/docs/design.md" \
    || fail "WP-200 representation blocker is not retained"
grep -Fq 'issue 0014' "$root/PROJECT_STATE.md" \
    || fail "continuation state does not retain the WP-200 blocker"

echo "v5 authority reset activation check: exact reviewed candidate integrated; 62 active owners and 59 inactive dispositions valid"
