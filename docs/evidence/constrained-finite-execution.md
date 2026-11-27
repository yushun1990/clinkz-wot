# Modern finite execution with caller-owned storage

This evidence concerns one NoSec Property Read representation on the modern
value and prepared-content boundary.

Core shares `ValueExecution`, `ValueSchemas`/`PreparedSchema`, borrowed output,
the finite identity/shape check and `FiniteCallState`. The existing Host finite
Consumer and the caller-owned Servient slot execute the same transition kernel.
The `host-meaning` case lends the actual schema inside modern Host content to
the alternative owner without preparing, copying or reinterpreting it.

The external Cargo-linked fixture defines a concrete datagram Binding, native
call owner, inline 64-byte output and bounded numeric codec. Production fixture
code uses no std/Arc/Rc/atomics. Hosted tests supply real nonblocking UDP sockets
under that same Binding; wire/native logic remains outside the Engine. The codec
supplies exact decimal numeric evidence to Core's existing schema evaluator.
This is ownership and execution evidence, not a production Binding mapping or
an Embedded socket implementation.

The executable cases cover these ownership and execution boundaries.

- one native poll per turn across sibling runtimes sharing a caller drive cell,
  including explicit refusal to drive a UDP Read during destruction of an active call or taken output
- fresh permission for unread results and validation of the exact view returned,
  including a native value that changes between views
- actual native identity/status/role/media/payload-bound and DataSchema refusals,
  retaining the original invalid output;
- cancellation and failed or pending cleanup with a racing raw native value
- same-slot replacement/foreign issuer and reused-call token rejection;
- callback-time policy revision changes and authority revocation;
- loss of the original validation service without invoking an unavailable codec;
- monotonic deadline expiry, clock mismatch/regression and first-cause retention;
- configuration, unsupported schema/security, capacity and permission refusal
  before native construction;
- result views tied to settlement lifetime, exclusive typed storage through final
  disposal, refused competing admission and reuse on release
- socket reuse at acknowledgement while the result is held, and finite local
  disposal of unacknowledged work.

The optional `prepare` feature exercises a bounded source job using TD's
portable validated Form loan. It prepares two explicitly selected non-first
Forms, then drops the TD. The same typed runtime reads each original destination
over real UDP with the prepared integer constraints. The source profile requires
the standard TD 1.1 context as its sole root entry. The regression verifies
refusal of additional root entries and local contexts on the selected Property,
Form and effective security definition. The fixture also runs with preparation
disabled and explicitly supplied program and meaning. Both execute with
deployment-supplied storage, time and authorization.

Run from the repository root.

```sh
cargo test --locked -p clinkz-wot-constrained-consumer-fixture
cargo test --locked -p clinkz-wot-constrained-consumer-fixture --features host-meaning,prepare
cargo test --locked -p clinkz-wot-servient-finite --no-default-features --doc
cargo check --locked -p clinkz-wot-constrained-consumer-fixture --target thumbv6m-none-eabi
cargo check --locked -p clinkz-wot-constrained-consumer-fixture --features prepare --target thumbv6m-none-eabi
cargo test --locked -p clinkz-wot-core -p clinkz-wot-servient-finite
cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml
```

The fixture's normal dependency graph selects no std features. The `thumbv6m`
check builds Core, Foundation, the real alternative Servient owner and the test
Binding/codec/output contracts for a target lacking pointer atomics. Executable
host tests provide state-transition/I/O evidence alongside this compilation;
hardware qualification has its own evidence requirements. The separate
`host-meaning` feature intentionally enables the Host content owner for its
loan test and is not used in the no-atomic compilation.

Immutable dependencies remain in caller storage through safe loans. Scalar
prepared meaning and inline work/output require no Engine allocation. General
structured-schema capture still uses the existing alloc-backed representation;
caller-provided arbitrary schema arenas are not demonstrated. Host account
banks, Arc payloads, boxed/pinned calls, Rc registries, existing synchronization
and native SDKs remain legitimate Host implementations. General caller-owned TD preparation, notifications, Producer publication,
credential composition and concrete embedded transports have separate evidence.

Continuing Actions now have a separate
[cross-representation increment](continuing-action-runtime-independence.md).
Its common kernel and native ownership evidence do not widen this finite Read
claim or qualify constrained TD preparation and embedded transports.
