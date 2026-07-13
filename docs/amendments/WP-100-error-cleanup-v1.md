# WP-100 Error and Cleanup Schema Amendment

Status: Frozen

Base design revision: v4.6

Amendment id: WP-100-ERR-CLEANUP-001

Affected requirements: API-TYPES-001, ERR-TAXONOMY-001, ERR-RETRY-001,
CLEANUP-RECORD-001, API-RESOURCE-001, BIND-IO-001

## Purpose

This normative amendment closes Rust schema decisions that the v4.6 prose left
open and adds the missing `Application` error category required to classify
bounded application failures and handler panics without misusing
`InternalInvariant`. It does not change the work-package DAG or execution
models. It freezes the representations required before WP-100 may replace the
legacy string-based error surface or publish cleanup records.

## Correlation Identity

`CorrelationId` is a copyable opaque `u64` newtype allocated by the binding or
binding adapter for one live binding generation. Zero is reserved for flows
that have no transport correlation. A binding that uses a byte string, pointer,
query object, or protocol-native identifier retains that value in its bounded
in-flight table and maps it to `CorrelationId`; core does not copy transport
identity bytes. Duplicate nonzero ids within one live binding generation are
rejected. Counter exhaustion stops admission before wraparound.

The public API is `new(u64)`, `get() -> u64`, `empty()`, and `is_empty()`.
`Debug` and `Display` reveal only the numeric core token, never the retained
protocol-native value.

## Error Value Schema

The following additional public enums are owned by `clinkz-wot-core::error` and
re-exported at the crate root:

```rust
pub enum RetryClass { Never, Safe, CallerDecision }

pub enum ErrorPhase {
    Unknown, Parse, Validate, Admission, Selection, Prepare, Readiness,
    Activate, Commit, Handler, Codec, Binding, Delivery, Cleanup,
}

pub enum SelectionFailureReason {
    AffordanceMissing,
    OperationUnsupported,
    NoFormSupportsOperation,
    TargetResolutionFailed,
    NoSupportingBinding,
    AmbiguousBindingOwner,
    SecurityUnavailable,
    StrictSelectionMismatch,
}

pub enum SecurityFailureReason {
    MissingCredentials,
    InvalidCredentials,
    AuthorizationDenied,
    UnsupportedScheme,
    ProviderFailure,
}
```

All four enums are non-exhaustive, copyable, orderable, hashable, and contain no
owned strings.

`ErrorContext` has private fields with these exact logical values:

- optional `ThingSlotId`, `AffordanceSlotId`, `Operation`, `u32` form index,
  `PlanId`, `(BindingId, BindingGeneration)`, and `CorrelationId`;
- one `ErrorPhase`;
- one `RetryClass` and optional `core::time::Duration` retry-after hint;
- optional `u16` redacted cause code;
- a private 96-byte inline UTF-8 redacted message, its `u8` length, and a
  truncation flag.

Writing the cause is allocation-free, truncates only at a UTF-8 boundary, and
cannot panic. Derived `Debug` is forbidden for `ErrorContext` and `CoreError`;
their manual `Debug` and `Display` implementations expose only compact ids,
category, phase, cause code, and the already-redacted inline message.
`std::error::Error::source()` returns `None` because raw provider, application,
codec, and transport errors are not retained.

`CoreError` is a non-exhaustive enum with this exact category schema:

```rust
pub enum CoreError {
    InvalidDocument(ErrorContext),
    Validation(ErrorContext),
    LimitExceeded {
        resource: ResourceKind,
        limit: u64,
        requested: Option<u64>,
        observed: Option<u64>,
        context: ErrorContext,
    },
    NotFound(ErrorContext),
    UnsupportedOperation(ErrorContext),
    Selection { reason: SelectionFailureReason, context: ErrorContext },
    Security { reason: SecurityFailureReason, context: ErrorContext },
    Application(ErrorContext),
    Binding(ErrorContext),
    Payload(ErrorContext),
    Backpressure(ErrorContext),
    Cancelled(ErrorContext),
    TimedOut(ErrorContext),
    StaleHandle(ErrorContext),
    Lifecycle(ErrorContext),
    Cleanup(ErrorContext),
    InternalInvariant(ErrorContext),
}
```

`context()`, `retry_class()`, and `retry_after()` are available for every
variant. Selection, security, and limit detail accessors return their structured
fields without formatting. `Application` covers bounded application errors and
handler panics; `InternalInvariant` remains reserved for detected engine defects.

## Retry Rules

Validation, limit, unsupported-operation, stale-handle, authentication, and
authorization errors use `Never`. A read-only binding failure uses `Safe` only
when the binding proves no side effect was committed. Writes, action invocation,
publication, response delivery, cancellation, and teardown use
`CallerDecision` unless an idempotency key or protocol acknowledgement proves a
safe retry. Retry-after is a hint and never causes automatic core retry.

## Legacy Error Mapping

The WP-100 migration removes the old variants in one coordinated workspace
change. No compatibility feature or deprecated duplicate enum is permitted.

| Legacy variant | Required target |
| --- | --- |
| `UnknownAffordance` | `NotFound` or selection `AffordanceMissing` |
| `UnsupportedOperation` | `UnsupportedOperation` |
| `UnsupportedBinding` | selection `NoSupportingBinding` |
| `Payload` | `Payload` |
| `Security` | structured `Security`, or selection `SecurityUnavailable` before commitment |
| `Transport` | `Binding`; transport timeouts become `TimedOut` |
| `InvalidInteraction` | split into `InvalidDocument`, `Validation`, `Selection`, `Binding`, or `InternalInvariant` at the producer |
| `MissingHandler` | `UnsupportedOperation` |
| `InboundDispatch` | `StaleHandle`, `Lifecycle`, or `Binding` at the producer |
| `HandlerPanic` | `Application` |
| `Timeout` | `TimedOut` |
| `TimeoutUnsupported` | `UnsupportedOperation` |
| `UnsupportedForm` | selection `StrictSelectionMismatch` |
| `ContentTypeMismatch` | `Payload` |

The public `SecurityError` enum is removed. Security producers construct
`CoreError::Security` with `SecurityFailureReason`; raw required/present scope
lists and provider messages are not retained in public errors.

## Cleanup Value Schema

`CleanupOperation` is a non-exhaustive copyable enum with exactly these v1
variants: `CancelRouteReadiness`, `AbortPreparedRoute`, `ShutdownRoute`,
`CancelRequest`, `CancelSubscriptionStart`, `StopSubscription`,
`CancelResponseDelivery`, `CancelEmission`, and `CancelProcess`.

`CleanupHandle` is a transparent newtype over `CleanupSlotId`.
`CleanupRecord` is copyable and has private fields with these exact values:

- `CleanupHandle` record identity;
- generation-bearing `CleanupSlotId` subject and owner identities;
- optional `ThingSlotId`, `BindingSlotId`, `(BindingId, BindingGeneration)`,
  and `PlanId` diagnostic identities;
- one `CleanupOperation`;
- optional monotonic deadline and retry-not-before instants;
- `u16` retry attempts, `RetryClass`, and `u16` redacted status code.

The record stores no string, payload, credential, TD, teardown plan, URI,
security expression, error object, or error chain and is at most 128 bytes on
supported targets. Runtime construction rejects retry attempts above
`cleanup_retry_attempts_max`. `CleanupOutcome` remains exactly `Complete`,
`PendingCleanup(CleanupRecord)`, or
`ResidualExternalState(CleanupRecord)`.
Deadline and retry instants must use the runtime clock id, and retry-not-before
must not be later than a present terminal deadline.

## Resource Schema Addition

`cleanup_retry_attempts_max` is an exhaustive `ResourceLimits` field with
gateway, Directory-client, and benchmark-static values `16`, `16`, and `4`.
It bounds attempts for one retained cleanup item. Zero disables retry and does
not disable the initial cleanup attempt. The existing
`cleanup_retry_records_max` independently bounds simultaneously retained retry
records.

## Implementation Order

WP-100 implements this amendment in the following order:

1. retry, phase, selection, security-reason, context, and correlation values;
2. cleanup values and the new resource limit;
3. coordinated `CoreError` and `SecurityError` workspace migration;
4. redaction, size, retry, wire-status, and old-surface absence evidence.
