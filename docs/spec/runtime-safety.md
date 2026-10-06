# Runtime Safety and Admission Specification

Status: active v5.1 authority; Consumer admission refined by ADR-0021.

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
A non-incremental operation is conforming when its admitted worst-case input is
explicitly bounded and its complete work/lifetime debit succeeds before the
operation starts; bounded progress does not require a cancellation point inside
every byte loop of such an atomic operation.

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

For the first Consumer Property Read transaction, [ADR-0021](../ADRs/0021-borrowed-consumer-td-admission.org)
selects caller-owned immutable typed input, complete bounded TD inspection/Basic,
a lifetime-bearing TD proof, paid semantic lending, source-independent owned
Planning output and Servient-owned publication. The exact TD API, byte oracle,
resource/support/progress and terminal refinements have one owner:
[the TD admission contract](../work-packages/WP-100-consumer-validated-thing-admission.md).

Whole-input structural inspection precedes complete shared Basic, including
unrelated Actions/Events/security/schemas/opaque fields. No trust proof exists
before both barriers. Resource Limit is distinct from Basic Invalid; diagnostics
preserve the shared first semantic cause in fixed inline coordinates. Basic
still accepts missing ID; Planning rejects it before runtime reservation,
materialization, bounds or start. Serialization cannot narrow typed validity.

The proof and semantic continuation borrow only external immutable input/config;
owned scratch uses private coordinates and short step loans. TD derives effective
operations/security/URI before lending a Ready coordinate. Ready retains completed
UTF-8 and scope-sizing facts across insufficient copy credit; no unpaid source
replay occurs. Every declared pass shares the remaining TD lifetime allowance.
Step/lifetime/multi-class debits complete before work, zero credit makes no hidden
progress, insufficient atomic credit carries no partial credit, lifetime exhaustion
is terminal Limit. Number projection keeps the existing bounded-atomic amendment
and explicit AP capability; production URI work is byte-resumable.

The caller keeps Thing immutable/live through Planning; validation Complete does
not shorten that lifetime. Input history is outside the additional controlled-state
claim. Engine-owned upstream source remains charged while live. TD frame/current
scratch, Planning/compiled output, simultaneous compiler/temporary state,
registration/erasure/record/slot, diagnostics and cleanup capacity all require
checked actual layout/accounting and paired local/parent allowances. Logical byte
or allowance totals are neither physical source capacities nor contiguous requests.

Planning preflight and complete materialization produce every property row and
readable original coordinate; all compiler bounds pass before any start. Only
one pure compiler cursor progresses at a time, with bounded callbacks and
exactly-once abort. Concrete artifacts/callbacks and suspended cursors must be
source-independent, not merely hidden behind a lifetime-free generic wrapper.
The consumed build owner structurally ends all input-bearing state before
returning source-free completion. Registration is a separate complete owner.

Failure fixes first cause and enters explicit bounded rollback. TD releases only
prepaid trivial workspace blocks; variable Planning output/artifacts use a
nonrecursive item/byte-budgeted cleanup owner with pre-reserved transfer capacity.
Caller input is never dropped by rejection, cancellation or abandonment.
No observable terminal loses protected live objects or parent allowances.

All build scratch cleanup and source-loan termination finish before Servient's
final identity/generation/resource/registration/cancellation/slot checks. A private
permit then closes cancellation under exclusive registry installation authority;
preallocated atomic install has no allocation, callback, yield, source query,
fallible release, new cancellation decision or retry. Readers see absent or
complete. Published owns only execution output, complete registration and runtime
lifecycle/resource records. It has no TD proof/view/borrow or Snapshot source
charge. Leases/drain and bounded child-before-parent reclamation remain required.

Strict bounded JSON is a separate future ingestion capability. ADR-0020's literal
value/field/duplicate/null/Number rules, ordinary-serde distinction and shared
resumable RFC3339 decoding remain binding there. That backend accounts controlled
source construction from the first allocation, shares TD meaning and proves
bounded decode/cancel/discard/release. Typed Consumer readmission does not require
its decoder; ordinary parse-and-borrow cannot claim its guarantee. Directory and
later external-input claims must enter their exact capability authority first.

This migration changes no tranche/gate/production status. Snapshot arena counts,
exact reseal and whole-TD post-copy equivalence are superseded; active semantic,
resource, progress, cleanup, complete-output and atomic-publication guarantees
remain at the replacement boundaries above.

`HANDLE-DROP-001`: An explicit destroy operation is the only handle API that
reports complete drain and cleanup. Dropping fixed private draft state performs only proved prepaid release; variable
draft state transfers to its pre-reserved bounded rollback owner. Dropping preparing or serving host state requests cancellation
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
