#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

sed -n '/^## Discovery$/,/^## Servient$/p' "$root/docs/design.md" >"$tmp"

for forbidden in \
    'the directory first applies' \
    'A Directory that issued a lease token MUST' \
    'Redaction is applied before enqueue' \
    'matched property-by-property against the caller-authorized'; do
    if grep -Fqi "$forbidden" "$tmp"; then
        echo "directory scope check: active design retains server rule: $forbidden" >&2
        exit 1
    fi
done

if ! grep -Fq 'scope = "engine-side-directory-client"' \
    "$root/docs/performance/directory.toml"; then
    echo "directory scope check: Directory performance scope is not client-only" >&2
    exit 1
fi

if awk -F, '
    NR > 1 && $3 == "clinkz-wot-discovery" &&
        $1 ~ /(DirectoryServer|DirectoryService|StorageBackend|InMemoryDirectory)/ {
        print $1
        found = 1
    }
    END { exit found ? 0 : 1 }
' "$root/docs/api-ownership.csv"; then
    echo "directory scope check: service or storage API is assigned to discovery" >&2
    exit 1
fi

if grep -Fq 'docs/future/directory-service.md' "$root/docs/artifacts.csv"; then
    echo "directory scope check: deferred service input is marked active" >&2
    exit 1
fi

if ! grep -Fq 'Status: non-normative input for a future design revision.' \
    "$root/docs/future/directory-service.md"; then
    echo "directory scope check: deferred service input lacks non-normative status" >&2
    exit 1
fi

echo "directory scope check: active contract is client-only"
