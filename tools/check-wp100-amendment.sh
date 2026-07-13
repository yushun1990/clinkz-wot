#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
amendment="$root/docs/amendments/WP-100-error-cleanup-v1.md"

for metadata in \
    'Status: Frozen' \
    'Base design revision: v4.6' \
    'Amendment id: WP-100-ERR-CLEANUP-001'; do
    if ! grep -Fq "$metadata" "$amendment"; then
        echo "WP-100 amendment check: missing metadata: $metadata" >&2
        exit 1
    fi
done

for item in ErrorPhase SecurityFailureReason; do
    if ! awk -F, -v item="$item" '
        NR > 1 && $1 == item && $3 == "clinkz-wot-core" && $14 == "frozen" { found = 1 }
        END { exit !found }
    ' "$root/docs/api-ownership.csv"; then
        echo "WP-100 amendment check: $item is not frozen in API ownership" >&2
        exit 1
    fi
done

if ! awk -F, '
    NR > 1 && $1 == "cleanup_retry_attempts_max" && $7 == 16 && $8 == 16 && $9 == 4 {
        found = 1
    }
    END { exit !found }
' "$root/docs/resource-limits.csv"; then
    echo "WP-100 amendment check: cleanup retry-attempt limit is not frozen" >&2
    exit 1
fi

for legacy in \
    UnknownAffordance UnsupportedOperation UnsupportedBinding Payload Security \
    Transport InvalidInteraction MissingHandler InboundDispatch HandlerPanic \
    Timeout TimeoutUnsupported UnsupportedForm ContentTypeMismatch; do
    if ! grep -Fq "\`$legacy" "$amendment"; then
        echo "WP-100 amendment check: missing legacy mapping for $legacy" >&2
        exit 1
    fi
done

echo "WP-100 amendment check: error, cleanup, and migration schemas frozen"
