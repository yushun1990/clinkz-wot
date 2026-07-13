# WP-100 Error Disposition Amendment

Status: Frozen

Base design revision: v4.6

Amendment id: WP-100-ERR-DISPOSITION-001

Affected requirements: API-PAYLOAD-001, API-RESOURCE-001, API-SURFACE-001,
ERR-TAXONOMY-001, HANDLER-API-001, BIND-IO-001, RES-LIMIT-001

## Purpose

This normative amendment freezes the boundary between successful interaction
status and protocol error disposition. It also closes the legacy
missing-handler and Servient predicate decisions required by the coordinated
WP-100 `CoreError` migration. It does not add an HTTP type to protocol-neutral
core and does not define reverse conversion from a protocol status into a
`CoreError`.

The shared defaults use the error-code subset recommended by the
[W3C WoT Profiles error contract](https://www.w3.org/TR/wot-profile/#errors).
That contract permits other valid protocol-specific codes, but a binding may
select one only through an explicit binding policy or a validated response plan,
not by inspecting diagnostic text.

## Successful Interaction Boundary

`InteractionStatus` describes only successful request completion. A
`CoreError` is never converted into an `InteractionStatus`, and an error must
not be returned inside a successful `InteractionOutput`.

The exact v1 Rust schema is:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum InteractionStatus {
    #[default]
    Ok,
    Created,
    Accepted,
}
```

It is owned by `clinkz_wot_core::interaction` and re-exported as
`clinkz_wot_core::InteractionStatus`. There is no `NoContent`, `Failed`, or raw
numeric variant. A binding derives `204` from the selected operation contract
and absence of a response representation; failures use `CoreError`.

`InteractionOutput` and its bounded metadata have these exact v1 logical
schemas, with private struct fields:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ResponsePayloadRole {
    #[default]
    Application,
    OperationStatus,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ResponseSelection {
    Primary,
    Additional(u16),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingResponseMetadata {
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    plan_id: PlanId,
    selection: ResponseSelection,
    status_code: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InteractionOutputMetadata {
    action_invocation: Option<ActionInvocationRef>,
    binding_response: Option<BindingResponseMetadata>,
    payload_role: ResponsePayloadRole,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct InteractionOutput {
    data: Option<Payload>,
    status: InteractionStatus,
    metadata: InteractionOutputMetadata,
}
```

All five types are owned by `clinkz_wot_core::interaction` and re-exported at
the core crate root. Every private field has a borrowed or copy-returning getter.
`InteractionOutputMetadata` has public builders for the action reference,
payload role, and untrusted binding-response metadata.
`BindingResponseMetadata` has public primary and checked-additional constructors
plus getters for each field. These constructors establish only bounded shape,
not authenticity. The checked constructor rejects an additional-response index
that is not below the active `additional_responses_per_form_max` limit.

The `InteractionOutput` public surface is:

```text
impl InteractionOutput {
    pub const fn empty() -> Self;
    pub fn with_data(data: Payload) -> Self;
    pub const fn with_status(self, status: InteractionStatus) -> Self;
    pub fn try_with_metadata(self, metadata: InteractionOutputMetadata) -> Option<Self>;
    pub fn data(&self) -> Option<&Payload>;
    pub const fn status(&self) -> InteractionStatus;
    pub const fn metadata(&self) -> &InteractionOutputMetadata;
    pub fn into_data(self) -> Option<Payload>;
    pub fn into_parts(
        self,
    ) -> (Option<Payload>, InteractionStatus, InteractionOutputMetadata);
}
```

`empty()` and `Default` produce no payload, `Ok`, and default metadata.
`with_data` changes only the payload. `try_with_metadata` rejects
`ResponsePayloadRole::OperationStatus` when no payload is present. The selected
response validator enforces operation-specific `Created`, `Accepted`, and action
reference requirements before an output becomes public.

Before any output crosses the validated interaction boundary, the shared
response validator treats incoming metadata as untrusted and verifies all of
the following against the selected live request and compiled response plan:

- `binding_id`, `binding_generation`, and `plan_id` exactly match the request;
- `Primary` or `Additional(index)` is the response branch actually classified
  by schema/media validation;
- an additional index is below both the active resource limit and the actual
  additional-response count in that compiled plan; and
- `status_code` came from the selected binding response, not from an application
  handler or caller-created output.

`ClientBinding::invoke` returns the untrusted metadata inside its
`InteractionOutput`, which gives a separate binding crate a protocol-neutral
channel for status and response-selection hints. Core retains that metadata only
after the selected binding call and response classification complete every
check above. Application-handler output that supplies binding-response metadata
is rejected as `Validation`. Public constructors never confer trusted or
validated status by themselves.

`ResponseSelection::Additional` is the zero-based index into the compiled
additional-response plan. The opaque `status_code` is interpreted only by the
identified binding generation; core never treats it as an HTTP code. A
protocol-native string, response object, schema, or extension map is not retained
in metadata. Action-domain state uses the single validated payload and marks it
with `OperationStatus`; there is no second metadata payload.

The fixed-size metadata adds no allocation or independent byte limit. Payload
storage remains governed by the applicable payload limits. Admission limits the
source and compiled additional-response array with
`additional_responses_per_form_max`; the profile values are gateway `32`,
Directory client `32`, and benchmark-static `16`.
Because `ResponseSelection::Additional` uses a zero-based `u16` index, profile
construction rejects `additional_responses_per_form_max` values greater than
`65_536`. A limit of `65_536` permits indices `0..=65_535`; zero disables
additional responses without disabling the primary response.

The v1 action invariants are exact:

- `Created` is reserved for creation of an addressable asynchronous action
  status resource. It requires one payload marked `OperationStatus` and an
  admitted `ActionInvocationRef`.
- `Accepted` represents asynchronous action acceptance without an addressable
  resource and therefore has no `ActionInvocationRef`.
- `OperationStatus` is legal only for action invocation, query, or cancellation
  responses and requires both the one payload and its `ActionInvocationRef`.
- subscription creation returns the separately owned subscription-start value;
  other created-resource results remain unsupported until a typed reference is
  frozen in a later design revision.

The default HTTP-shaped success disposition is:

| `InteractionStatus` and response condition | Default status |
| --- | ---: |
| `Ok` with a response representation | `200` |
| `Ok` with no payload when the operation contract requires no representation | `204` |
| `Created` with an addressable asynchronous action status resource | `201` |
| `Accepted` for explicit asynchronous acceptance without a newly addressable resource | `202` |

The selected operation and validated response plan, not payload emptiness
alone, decide whether `Ok` uses `200` or `204`. A missing required output is a
`Payload` or `Validation` error and must not be downgraded to `204`.

An asynchronous WoT action that creates an addressable action-status resource
uses `Created`, not `Accepted`. `query_action` and `cancel_action` return a
validated `InteractionOutput`; their action-lifecycle state remains in the
single payload marked `ResponsePayloadRole::OperationStatus`. Request-level
failure remains a `CoreError`.

## Shared Default Error Disposition

The `no_std` shared binding utility remains:

```rust
pub fn error_status(error: &CoreError) -> u16;
```

It is owned by `clinkz_wot_protocol_bindings::error_status`, performs no
allocation, cannot panic, and returns the following HTTP-shaped default. The
generated default error-disposition value is not stored in core interaction
state.

| Error category or structured reason | Default status |
| --- | ---: |
| `InvalidDocument` | `400` |
| `Validation` | `400` |
| `LimitExceeded` | `400` |
| `NotFound` | `404` |
| `UnsupportedOperation` with `ErrorPhase::Handler` | `500` |
| `UnsupportedOperation` with any other phase | `400` |
| `Selection::AffordanceMissing` | `404` |
| `Selection::OperationUnsupported` | `400` |
| `Selection::NoFormSupportsOperation` | `400` |
| `Selection::TargetResolutionFailed` | `400` |
| `Selection::NoSupportingBinding` | `500` |
| `Selection::AmbiguousBindingOwner` | `500` |
| `Selection::SecurityUnavailable` | `401` |
| `Selection::StrictSelectionMismatch` | `400` |
| `Security::MissingCredentials` | `401` |
| `Security::InvalidCredentials` | `401` |
| `Security::AuthorizationDenied` | `403` |
| `Security::UnsupportedScheme` | `401` |
| `Security::ProviderFailure` | `500` |
| `Application` | `500` |
| `Binding` | `503` |
| `Payload` | `400` |
| `Backpressure` | `503` |
| `Cancelled` while a reply opportunity remains | `503` |
| `TimedOut` | `503` |
| `StaleHandle` | `404` |
| `Lifecycle` | `503` |
| `Cleanup` | `500` |
| `InternalInvariant` | `500` |
| A future unknown `CoreError` or structured reason | `500` |

`LimitExceeded` covers a deterministic per-operation, per-item, or per-owner
configured ceiling, including an operation's temporary working-budget ceiling.
`Backpressure` covers currently occupied shared queue, slot, or concurrency
capacity that can clear without changing the request. If cancellation removes
the reply opportunity, the binding sends no response rather than manufacturing
a status. A retry-after hint may become a native retry hint on a `503` response,
but it never triggers an automatic retry.

The default intentionally remains coarse where the frozen error schema has no
typed subreason. In particular, `Payload` does not recover the legacy `406`
distinction, and `Selection::SecurityUnavailable` defaults to `401` even when
the unavailable component is a provider. Cause codes, redacted messages,
formatted output, and retry classes never alter the status.

An HTTP gateway policy may explicitly map an established upstream binding
failure to `502` or an established upstream timeout to `504`. A validated TD
response plan may select another declared status. Such an override belongs to
the binding plan or binding implementation, must have focused conformance
tests, and must fall back to the table above when its preconditions are not
proved. Non-HTTP bindings translate the disposition into their native error
mechanism without changing the `CoreError` category.

Reverse conversion is intentionally undefined. A numeric status lacks enough
information to reconstruct category, reason, context, or retry advice.

## Handler Absence and Legacy Predicates

An absent or cleared handler slot, an async-only slot reached through a sync
entry point, and a request admitted under `AllowLateHandlers` before handler
publication all produce:

```rust
CoreError::UnsupportedOperation(
    ErrorContext::new(ErrorPhase::Handler, RetryClass::Never)
)
```

The producer adds known Thing, affordance, operation, form, and plan identities.
It does not attach an unregistered cause code merely to recreate the removed
`MissingHandler` variant.

`ErrorPhase::Handler` remains diagnostic context, not a public
missing-handler discriminator. Application code can also return an unsupported
operation during handler execution, so consumers must not infer handler-slot
state from phase, cause code, or text.

The coordinated migration removes
`ServientError::{is_missing_handler,is_security,is_timeout}`. These convenience
predicates are absent from the frozen ownership matrix and collapse structured
core categories and reasons into an incomplete Boolean layer. Callers that
temporarily receive a `ServientError` use `as_core()` and match the frozen
`CoreError` taxonomy. Protocol-private predicates such as
`ZenohPicoError::is_timeout` are unaffected.

## Migration Evidence

The migration evidence must prove:

1. every current and future-default error disposition without inspecting
   strings or cause codes;
2. success statuses never contain a `CoreError` and error replies never become
   successful `InteractionOutput` values;
3. all missing-handler producer paths return `UnsupportedOperation` with
   `ErrorPhase::Handler` and `RetryClass::Never` while unknown affordances remain
   `NotFound` or `Selection::AffordanceMissing`;
4. the three removed Servient Boolean predicates and every legacy `CoreError`
   variant are absent from non-deprecated source and public documentation; and
5. protocol-specific status overrides are tested at the binding boundary and
   preserve the original structured error.
