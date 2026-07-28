# Interaction Core Specification

Status: v5.0 activation candidate.

This specification owns ten active requirements: `HANDLER-API-001`,
`HANDLER-CANCEL-001`, `HANDLER-CANCEL-002`, `API-TYPES-001`,
`API-HOT-ID-001`, `API-PAYLOAD-001`, `CLEANUP-RECORD-001`,
`ERR-TAXONOMY-001`, `ERR-RETRY-001`, and `STATE-INFLIGHT-001`.
`HANDLER-VALUE-001` remains owned by the completed handler amendment, whose
exact schemas refine this specification without changing the owner set.

Core owns protocol-neutral identities, request/result/error values, handler
semantics, and portable operation state. It does not own application handles,
Servient registries, plan construction, protocol I/O, or global scheduling.

## Handler boundary

`HANDLER-API-001`: Every operation-specific handler receives a
call-lifetime-borrowed request plus a copyable `HandlerContext` appropriate to
the compiled plan. Together they expose Thing identity, target, operation,
plan, correlation, verified principal, validated URI variables,
deadline/cancellation view, and application-safe binding metadata. The request
alone owns request-varying facts; the context contains dispatch identity only.
Neither exposes credentials, raw authentication material, provider-managed
body fields, or a mutable registry. Operation-specific setters fix exact input
and result types. Handler callbacks run without engine locks.

Host registration owns `Send + Sync + 'static` handlers. Portable async traits
use associated futures with the same results and may borrow the handler and
call owner; host erasure may add `Send` privately. Constrained registration may
use caller-owned slots or bounded references and does not require atomics,
`Arc`, or boxed futures unless its selected cell does.

`HANDLER-CANCEL-001`: A synchronous handler is cooperative and
non-preemptible. The engine MUST check cancellation and deadline state before
entry and after return, but MUST NOT claim to stop, bound, or time out user code
that does not return. A result arriving after the selected linearization point
is retained or discarded by the active drain policy and never published as an
on-time success.

`HANDLER-CANCEL-002`: Work requiring an enforced execution deadline MUST use a
cancellation-cooperative async handler or a poll/step handler with a bounded
per-step contract. Aggregate timeout and cancellation describe the aggregate
request/response lifecycle, not preemption of synchronous code. Public API
documentation MUST state this distinction.

The exact five handler values, eighteen-operation matrix, registration methods,
cancellation reducers, and completed Property Read handler representation are
owned by `docs/amendments/WP-100-handler-api-v1.md`.

## Values and errors

`API-TYPES-001`: Identifiers retained beyond one call are opaque newtypes rather
than interchangeable integers or strings. They implement only semantics
appropriate to their bounded representation. Generation-bearing identities
compare and hash both index and generation. Debug and Display output for
identifiers and errors MUST NOT reveal payloads or credentials.

`API-HOT-ID-001`: Runtime lookup and per-call records use bounded
slot/generation identities. Human-readable names remain at admission/API
boundaries and in immutable plan or diagnostic tables; request, response,
sample, and cleanup hot records MUST NOT clone them. Formatting consumes the
selected diagnostic budget.

`API-PAYLOAD-001`: `Payload` owns or shares immutable bytes with parsed media
metadata; moving it never copies the body. Inspection is borrowed; decoding is
explicit, fallible, codec-selected, and size-budgeted. `InteractionInput` owns
application-visible request facts. `InteractionOutput` owns one response
payload, normalized status, and fixed-size metadata. These values never borrow
a binding call stack.

`ERR-TAXONOMY-001`: `CoreError` is a non-exhaustive structured error with
bounded context and categories for invalid documents, validation, limits,
lookup, unsupported operations, selection, security, binding, payload,
backpressure, cancellation, timeout, stale handles, lifecycle, cleanup, and
internal invariants. External malformed input never becomes an internal
invariant. Selection and execution preserve plan/binding context, redaction,
and a bounded fallback when diagnostic allocation fails.

`ERR-RETRY-001`: Errors expose `Never`, `Safe`, or `CallerDecision` retry
classification plus an optional bounded retry hint. Validation, limits, stale
handles, unsupported operations, and authorization failures default to
`Never`. Read-only work is `Safe` only with proof that no side effect committed.
Writes, actions, publication, and teardown default to `CallerDecision` unless
idempotency or acknowledgement proves safe retry. The engine never retries
merely because a failure is transient.

`CLEANUP-RECORD-001`: A queued cleanup record contains only a
generation-bearing owner/plan reference, cleanup operation, deadline/retry
state, and bounded status. Teardown plans, targets, security expressions, and
diagnostic names stay in the owning guard or immutable plan arena. Enqueueing
MUST NOT clone a complete plan or payload; admission fails before ownership can
be lost if retained cleanup exceeds its budget.

The completed error, payload, retry, and cleanup amendments own exact Rust
schemas where they are narrower than this architectural contract.

## In-flight state

`STATE-INFLIGHT-001`: Dispatch reserves an in-flight slot only after confirming
Serving, then rechecks the registry generation while publishing the slot.
Destroy and admission share one synchronization boundary, so each request is
unambiguously rejected or counted. Completion releases the slot exactly once.
After drain expiry, the response gate closes before binding shutdown and late
handler results cannot retain a registry generation indefinitely.
