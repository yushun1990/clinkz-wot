#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
amendment="$root/docs/amendments/WP-100-error-cleanup-v1.md"
disposition="$root/docs/amendments/WP-100-error-disposition-v1.md"
output_api="$root/docs/amendments/WP-100-interaction-output-api-v1.md"

for metadata in \
    'Status: Frozen' \
    'Base design revision: v4.6' \
    'Amendment id: WP-100-ERR-CLEANUP-001'; do
    if ! grep -Fq "$metadata" "$amendment"; then
        echo "WP-100 amendment check: missing metadata: $metadata" >&2
        exit 1
    fi
done

for metadata in \
    'Status: Frozen' \
    'Base design revision: v4.6' \
    'Amendment id: WP-100-OUTPUT-API-001'; do
    if ! grep -Fq "$metadata" "$output_api"; then
        echo "WP-100 output API check: missing metadata: $metadata" >&2
        exit 1
    fi
done

for signature in \
    'pub const fn primary(' \
    'pub fn try_additional(' \
    'limits: &ResourceLimits' \
    'pub const fn with_untrusted_binding_response(' \
    'pub const fn binding_response(&self) -> Option<BindingResponseMetadata>' \
    'result: CoreResult<InteractionOutput>' \
    'pub fn try_success(' \
    'pub fn failure(' \
    'pub fn validate_untrusted_binding_output(' \
    'pub fn result(&self) -> Result<&InteractionOutput, &CoreError>' \
    'pub fn into_result(self) -> CoreResult<InteractionOutput>'; do
    if ! grep -Fq "$signature" "$output_api"; then
        echo "WP-100 output API check: missing frozen signature: $signature" >&2
        exit 1
    fi
done

for invariant in \
    '`None` and `Some(0)` both reject every additional' \
    'maximum accepted pair is limit `65_536` with index `65_535`' \
    'The private `CoreResult<InteractionOutput>` is the only terminal channel' \
    'WP-100 must not replace it with a' \
    'shared consumer/binding-origin validator at' \
    'WP-700 closes the end-to-end evidence'; do
    if ! grep -Fq "$invariant" "$output_api"; then
        echo "WP-100 output API check: missing invariant: $invariant" >&2
        exit 1
    fi
done

for evidence_key in \
    core-public-surface logical-plan-footprint binding-response-validation \
    servient-response-validation binding-response-provenance \
    end-to-end-response-boundary; do
    if ! grep -Fq "$evidence_key" "$output_api" \
        || ! grep -Fq "\"$evidence_key\"" "$root/docs/work-packages/index.toml"; then
        echo "WP-100 output API check: unstaged evidence key: $evidence_key" >&2
        exit 1
    fi
done

for metadata in \
    'Status: Frozen' \
    'Base design revision: v4.6' \
    'Amendment id: WP-100-ERR-DISPOSITION-001'; do
    if ! grep -Fq "$metadata" "$disposition"; then
        echo "WP-100 disposition check: missing metadata: $metadata" >&2
        exit 1
    fi
done

for mapping in \
    '| `InvalidDocument` | `400` |' \
    '| `Validation` | `400` |' \
    '| `LimitExceeded` | `400` |' \
    '| `NotFound` | `404` |' \
    '| `UnsupportedOperation` with `ErrorPhase::Handler` | `500` |' \
    '| `UnsupportedOperation` with any other phase | `400` |' \
    '| `Selection::AffordanceMissing` | `404` |' \
    '| `Selection::OperationUnsupported` | `400` |' \
    '| `Selection::NoFormSupportsOperation` | `400` |' \
    '| `Selection::TargetResolutionFailed` | `400` |' \
    '| `Selection::NoSupportingBinding` | `500` |' \
    '| `Selection::AmbiguousBindingOwner` | `500` |' \
    '| `Selection::SecurityUnavailable` | `401` |' \
    '| `Selection::StrictSelectionMismatch` | `400` |' \
    '| `Security::MissingCredentials` | `401` |' \
    '| `Security::InvalidCredentials` | `401` |' \
    '| `Security::AuthorizationDenied` | `403` |' \
    '| `Security::UnsupportedScheme` | `401` |' \
    '| `Security::ProviderFailure` | `500` |' \
    '| `Application` | `500` |' \
    '| `Binding` | `503` |' \
    '| `Payload` | `400` |' \
    '| `Backpressure` | `503` |' \
    '| `Cancelled` while a reply opportunity remains | `503` |' \
    '| `TimedOut` | `503` |' \
    '| `StaleHandle` | `404` |' \
    '| `Lifecycle` | `503` |' \
    '| `Cleanup` | `500` |' \
    '| `InternalInvariant` | `500` |' \
    '| A future unknown `CoreError` or structured reason | `500` |'; do
    if ! grep -Fq "$mapping" "$disposition"; then
        echo "WP-100 disposition check: missing mapping: $mapping" >&2
        exit 1
    fi
done

for predicate in is_missing_handler is_security is_timeout; do
    if ! grep -Fq "$predicate" "$disposition"; then
        echo "WP-100 disposition check: missing predicate disposition: $predicate" >&2
        exit 1
    fi
done

for status_variant in Ok Created Accepted; do
    if ! grep -Eq "^[[:space:]]*$status_variant,?$" "$disposition"; then
        echo "WP-100 disposition check: missing InteractionStatus variant: $status_variant" >&2
        exit 1
    fi
done

if ! grep -Fq '#[non_exhaustive]' "$disposition" \
    || ! grep -Fq '#[default]' "$disposition"; then
    echo "WP-100 disposition check: InteractionStatus traits/default are not frozen" >&2
    exit 1
fi

for output_field in \
    'data: Option<Payload>' \
    'status: InteractionStatus' \
    'metadata: InteractionOutputMetadata' \
    'action_invocation: Option<ActionInvocationRef>' \
    'binding_response: Option<BindingResponseMetadata>' \
    'payload_role: ResponsePayloadRole'; do
    if ! grep -Fq "$output_field" "$disposition"; then
        echo "WP-100 disposition check: missing InteractionOutput field: $output_field" >&2
        exit 1
    fi
done

for invariant in \
    'exactly match the request' \
    'additional-response count in that compiled plan' \
    'came from the selected binding response' \
    'constructors establish only bounded shape' \
    'separate binding crate a protocol-neutral' \
    '`Created` is reserved for creation of an addressable asynchronous action' \
    '`Accepted` represents asynchronous action acceptance without an addressable'; do
    if ! grep -Fq "$invariant" "$disposition"; then
        echo "WP-100 disposition check: missing output invariant: $invariant" >&2
        exit 1
    fi
done

if grep -Eq 'handler-missing|unsupported-operation or handler-missing' "$root/docs/design.md"; then
    echo "WP-100 disposition check: base design still permits handler-missing taxonomy" >&2
    exit 1
fi

if ! grep -Fq '`query_action` and `cancel_action` return a validated' \
    "$root/docs/design.md"; then
    echo "WP-100 disposition check: action success boundary is not frozen" >&2
    exit 1
fi

for item in \
    ErrorPhase SecurityFailureReason ResponsePayloadRole ResponseSelection \
    BindingResponseMetadata InteractionOutputMetadata InboundResponse; do
    if ! awk -F, -v item="$item" '
        NR > 1 && $1 == item && $3 == "clinkz-wot-core" && $14 == "frozen" { found = 1 }
        END { exit !found }
    ' "$root/docs/api-ownership.csv"; then
        echo "WP-100 amendment check: $item is not frozen in API ownership" >&2
        exit 1
    fi
done

if ! awk -F, '
    NR > 1 && $1 == "InboundResponse" && $4 == "binding" && $14 == "frozen" {
        found = 1
    }
    END { exit !found }
' "$root/docs/api-ownership.csv"; then
    echo "WP-100 output API check: InboundResponse ownership is not frozen" >&2
    exit 1
fi

if ! awk -F, '
    NR > 1 && $1 == "BindingResponseMetadata" \
        && $11 ~ /BIND-IO-001/ && $11 ~ /BIND-OUT-001/ \
        && $14 == "frozen" { found = 1 }
    END { exit !found }
' "$root/docs/api-ownership.csv"; then
    echo "WP-100 output API check: binding response metadata roles are not frozen" >&2
    exit 1
fi

if ! awk -F, '
    NR > 1 && $1 == "validate_untrusted_binding_output" \
        && $3 == "clinkz-wot-planning" && $4 == "response" \
        && $5 == "public" && $14 == "frozen" { found = 1 }
    END { exit !found }
' "$root/docs/api-ownership.csv"; then
    echo "WP-100 output API check: binding response validator ownership is not frozen" >&2
    exit 1
fi

if ! awk -F, '
    NR > 1 && $1 == "cleanup_retry_attempts_max" && $7 == 16 && $8 == 16 && $9 == 4 {
        found = 1
    }
    END { exit !found }
' "$root/docs/resource-limits.csv"; then
    echo "WP-100 amendment check: cleanup retry-attempt limit is not frozen" >&2
    exit 1
fi

if ! grep -Fq 'values greater than' "$disposition" \
    || ! grep -Fq '`65_536`' "$disposition"; then
    echo "WP-100 disposition check: additional-response index bound is not frozen" >&2
    exit 1
fi

if ! awk -F, '
    NR > 1 && $1 == "additional_responses_per_form_max" \
        && $7 == 32 && $8 == 32 && $9 == 16 { found = 1 }
    END { exit !found }
' "$root/docs/resource-limits.csv"; then
    echo "WP-100 disposition check: additional-response limit is not frozen" >&2
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

echo "WP-100 amendment check: schemas, output APIs, disposition, and staging frozen"
