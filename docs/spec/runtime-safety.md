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

For the first v5.1 Consumer Property Read admission, successful construction
owns one move-only TD `ValidatedThing` whose physical source is a private,
immutable, project-controlled normalized snapshot. It owns no caller `Thing`,
opaque standard/serde container, caller spare capacity, or input borrow. It
proves complete `ValidationLevel::Basic`, typed fieldwise semantic equivalence
for the compatibility entry, checked structural counts, and an exhaustive
requested-allocation footprint. No unchecked constructor, `&Thing`, mutable
view, raw arena, or storage offset is public.

Compatibility conversion borrows a typed `Thing` and provides an exact
additional project-owned peak over the pre-existing caller baseline. The
strict project-owned JSON builder/decoder charges every engine-owned
allocation from first input processing through retained completion and provides
the absolute engine-owned admission path required by application-static users.
Both drive the same bounded cursor, normalized representation, TD semantic
kernel, terminal model, and view; Host may drive it synchronously while a
static caller resumes it.

Semantic equivalence is defined over the typed `Thing` model: all known fields
and optional distinctions, ordered sequences and original Form indices, map
associations, string/URI content, and nested extension values including
lossless numbers. The compatibility path traverses typed fields directly. It
MUST NOT serialize and deserialize the `Thing`, use serialized length as the
equivalence oracle, or reject a Basic-valid typed value because a serializer is
stricter than Basic validation.

The retained snapshot has exactly three possible exact-length allocations: a
typed node arena, an edge/range arena, and a byte arena. Its nodes own no nested
allocation. Build storage is limited to mutable node/edge/byte arenas and one
traversal arena; grow and seal overlap is explicit and charged. Therefore
terminal cleanup drops a bounded fixed allocation set instead of recursively
destroying one owned value per input depth.

The path accepts ordinary legal serde_json feature unification within the
feature graphs supported by each profile; this compatibility requirement does
not require a `std`-only upstream feature to compile in a `no_std` profile.
Host supports the base/default, `preserve_order`, `arbitrary_precision`, and
combined graphs. The real `thumbv7em-none-eabihf` `no_std + alloc` profile
supports the base `no_std + alloc` and `arbitrary_precision` graphs. In the
resolved serde_json 1.0.149 graph, `preserve_order` enables `std`, so it and the
combined graph are not supported constrained-profile cells. Exact serde_json,
rustc, liballoc, target layout, and private allocator behavior are not
compatibility authority.

Every explicit Property Form operation and every normalized map-sort
comparison advances only after its own `DocumentNodes` charge. Every
security-expression root or combo child advances only after its own
`SecurityBranches` charge. String/number/URI bytes and strict JSON input bytes
are charged before copy, formatting, resolution, or decode. No whole list,
batch, serializer output, recursive task tree, or sort scratch allocation is
created first. These progress rules derive the retained readable-Form count and
normalized storage without duplicating Basic validation.

One TD-owned storage-neutral semantic kernel is the Basic/default/URI/security
authority for both `Thing` and normalized snapshot adapters. The allocation-
free `ValidatedThingView` exposes identity, deterministic Property iteration
and lookup, Property ordinal, original Form index, raw and resolved URI,
content metadata, effective operations/security, and security-definition
scheme lookup. Planning MUST NOT reconstruct a `Thing`, parse the snapshot, or
copy those rules.

The normalized `ValidatedThing` remains the one retained application/source
owner after publication. Source-to-persistent-document reclassification moves
only the sealed arenas' total requested bytes and preserves physical storage,
allocation count, live/peak truth, and largest actual request. Aggregate
capacity reservation is never treated as a contiguous physical allocation. A
Basic-valid semantic value without an ID is rejected by Consumer preflight
before persistent-capacity reservation, materialization, compiler bounds, or
compiler start; the slice does not synthesize identity or strengthen Basic
validation.

Input inspection, Basic validation, normalization, seal, semantic-equivalence
comparison, Planning preflight, conservative persistent-capacity reservation,
materialization, the all-coordinate bounds barrier, compilation,
reconciliation, and the final cancellation check are unpublished phases.
Cancellation is observed before work and callbacks and at bounded intervals.
Invalid, limit, cancellation, and conversion failure first enter the registered
normalization rollback state. The first cause remains immutable; no terminal is
returned until every partial project allocation and ledger reservation is
released. Precharged fixed-allocation cursor drop is the only implicit cleanup
and performs no recursive semantic destruction. Later aggregate failure starts
no new compiler work, aborts the one live pure cursor at most once, releases all
still-uncommitted reservations idempotently, spends the reserved generation,
and publishes neither a handle nor a partial lookup.

The normalization terminal never owns the existing allocation-bearing,
recursive `ValidateError`. It stores a fixed inline invalid category, cause
phase, and optional input-byte or semantic-node coordinate. The public
`Thing::validate_with_level` adapter retains its established error surface, but
both adapters receive acceptance/rejection from the same TD Basic rule kernel.
Changing the diagnostic sink cannot narrow or widen the Basic-valid set.

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
