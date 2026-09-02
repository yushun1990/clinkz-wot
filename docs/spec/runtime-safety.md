# Runtime Safety and Admission Specification

Status: active v5.0 authority.

This specification owns ten active cross-domain safety requirements:
`DOC-RUNTIME-001`, `API-SECURITY-001`, `CONSTRAINED-PROGRESS-001`,
`CONSTRAINED-OWN-001`, `CAP-OVERFLOW-001`, `ADMIT-TXN-001`,
`HANDLE-DROP-001`, `HOST-ASYNC-001`, `STATE-EXPOSE-001`, and
`STATE-BIND-001`.

## Runtime representation and security

`DOC-RUNTIME-001`: Lossless document ownership and compiled runtime ownership
are separate. A source document may preserve extensions, spelling, ordering,
and evidence; a compiled Thing retains only admitted runtime identities,
indexes, plans, and diagnostics. A runtime handle MUST NOT retain a complete
generic JSON tree or lossless TD merely because admission began from it. Source
retention is explicit and separately charged.

`API-SECURITY-001`: Security capability and applicability probes are bounded
and side-effect-free. Credential or verification commit happens only after one
candidate is selected and returns an owned generation-bearing lease or
principal. Provider identity and generation participate in plan validity.
Credentials and provider-managed authentication fields never enter application
payloads, logs, plans, or handler context. Callbacks run outside engine locks;
body projection is explicit, bounded, and cannot cause a second unaccounted
decode.

## Progress, ownership, and overflow

`CONSTRAINED-PROGRESS-001`: A manual runtime step consumes an explicit budget
of transitions and typed work, returns at most one value plus exact pending or
terminal state, and makes no hidden progress at zero budget. Cleanup has
reserved bounded capacity. When that capacity is full, ownership remains with
an explicit bounded operation or cleanup-pending handle; it is never dropped.

`CONSTRAINED-OWN-001`: Constrained handles use lifetimes, unique ownership, or
generation-bearing table references and MUST NOT require `Arc` or pointer-width
atomics. Cross-context sharing belongs to the application-selected critical
section or message-passing boundary. Allocation and user callbacks occur
outside critical sections.

`CAP-OVERFLOW-001`: Overflow reporting MUST NOT enqueue into the queue that is
already full. Loss counters and the latest summary use fixed or overwrite-in-
place storage. Shutdown selected by overflow MUST make progress without relying
exclusively on the blocked producer.

## Admission and lifecycle

`ADMIT-TXN-001`: Parsing, validation, effective-view construction, planning,
and registry publication form a reserve-build-publish transaction. Work and
temporary bytes are charged first, persistent capacity is reserved next,
private state is built before one publication transition, and every failure
releases reservations idempotently. Cancellation is checked at bounded work
intervals and before publication.

For the first v5.1 Consumer Property Read admission, the input is one opaque,
move-only TD-owned validated value. Successful construction owns the exact
typed `Thing`, proves complete `ValidationLevel::Basic`, records checked
structural limits and a conservative representation-aware retained-source
census, and exposes neither an unchecked constructor nor mutable raw-Thing
projection. Serialized length and `size_of::<Thing>()` alone are not a valid
retained-footprint proof. Host and application-static profiles drive the same
bounded pure validation cursor; Host may complete it synchronously, while the
static profile resumes it with its retained lifetime allowance.

The validated `Thing` remains the one retained application/source view after
publication and is never cloned merely for accounting. Source-to-persistent-
document reclassification preserves the same owned representation and total
live/peak allocation truth. A Basic-valid Thing without an ID is rejected by
Consumer preflight before persistent-capacity reservation, materialization,
compiler bounds, or compiler start; the slice does not synthesize an identity
or globally strengthen Basic validation.

Validation, preflight, conservative persistent-capacity reservation,
materialization, the all-coordinate bounds barrier, compilation,
reconciliation, and the final cancellation check are unpublished phases.
Cancellation is observed before external/compiler callbacks and at bounded
pure-work intervals. Failure or cancellation fixes the first cause, starts no
new compiler work, aborts the one live pure cursor at most once, releases all
still-uncommitted reservations idempotently, spends the reserved generation,
and publishes neither a handle nor a partial lookup.

`HANDLE-DROP-001`: An explicit destroy operation is the only handle API that
reports complete drain and cleanup. Dropping private draft state releases it
synchronously. Dropping preparing or serving host state requests cancellation
or draining exactly once and transfers complete cleanup ownership to a
reserved Servient executor or explicit manual driver without blocking on user
or network work. Exhausted cleanup capacity returns a structured cleanup/limit
result and never forgets a live guard.

`HOST-ASYNC-001`: Boxed object-safe futures are compatibility adapters rather
than the only host execution path. A binding may expose native async, poll, or
reusable operation slots that avoid per-interaction allocation. Every claimed
allocation-sensitive path is measured separately. Generation-safe pools apply
backpressure on exhaustion and MUST NOT fall back to unbounded allocation.

`STATE-EXPOSE-001`: Exposure progresses through private preparation, readiness,
activation, committed-closed, publication, serving, draining, and terminal
cleanup states. Publication and cancellation share one linearization boundary.
Before publication no route is dispatchable; after cancellation wins it never
becomes dispatchable. Every readiness token, route guard, reservation, and
cleanup outcome retains exactly one owner through failure, cancellation, drop,
destroy, retry, or terminal state.

`STATE-BIND-001`: A binding route progresses from absent through prepared,
ready, active, committed-closed, serving, draining, and closed, with explicit
cleanup-pending transitions after resource acquisition. Servient owns state
transitions; the guard owns protocol resources. A Host route's prepared,
active, and committed stage guards successively own one unchanged Core-private
carrier containing its complete preparation input, footprint, generation, and
binding-private concrete state. A stage transition cannot replace or extract
that state. Core exposes only a type-checked shared pinned projection of the
state, never a safe whole-state mutable projection; protocol-local mutation is
encapsulated behind methods on the shared state. Host accept polling borrows
the committed guard only by shared reference; Servient never lends mutable
whole-guard authority that could replace, extract, or prematurely dispose the
linear lifecycle owner. Operations are idempotent for one Thing/binding
generation. A guard drop is not a transition, late callbacks are generation
checked, and a draining or closed route never returns to serving. Terminal
cleanup or durable residual acknowledgement releases the carrier state exactly
once.
