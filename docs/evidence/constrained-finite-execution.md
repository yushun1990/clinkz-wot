# Modern finite execution with caller-owned storage

This evidence concerns one NoSec Property Read representation on the modern
value/prepared-content boundary. It does not use the historical static Read
runtime, PlanSet or either historical architecture gate as proof.

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

The executable cases cover:

- paid native progress and codec/handoff yields, with no callback on short credit;
- terminal unread success checked against fresh policy without decoding again;
- actual native identity/status/role/media/payload-bound and DataSchema refusals,
  retaining the original invalid output;
- cancellation, failed/pending cleanup and a validated racing native value;
- same-slot replacement/foreign issuer and reused-call token rejection;
- callback-time policy revision changes and authority revocation;
- loss of the original validation service without invoking an unavailable codec;
- monotonic deadline expiry, clock mismatch/regression and first-cause retention;
- configuration, unsupported schema/security, capacity and permission refusal
  before native construction;
- result views tied to settlement lifetime, original caller lease reuse/refund,
  native socket retention through result ownership, acknowledged local cleanup
  and finite local disposal without a false acknowledgement.

Reproduce from the repository root:

```sh
cargo test --locked -p clinkz-wot-constrained-consumer-fixture
cargo test --locked -p clinkz-wot-constrained-consumer-fixture --features host-meaning
cargo test --locked -p clinkz-wot-servient-finite --no-default-features --doc
cargo check --locked -p clinkz-wot-constrained-consumer-fixture --target thumbv6m-none-eabi
cargo test --locked -p clinkz-wot-core -p clinkz-wot-servient-finite
cargo test --locked --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml
```

The fixture's normal dependency graph selects no std features. The `thumbv6m`
check builds Core, Foundation, the real alternative Servient owner and the test
Binding/codec/output contracts for a target lacking pointer atomics. Executable
host tests provide state-transition/I/O evidence alongside this compilation;
neither alone establishes a hardware/runtime qualification. The separate
`host-meaning` feature intentionally enables the Host content owner for its
loan test and is not used in the no-atomic compilation.

Immutable dependencies remain in caller storage through safe loans. Scalar
prepared meaning and inline work/output require no Engine allocation. General
structured-schema capture still uses the existing alloc-backed representation;
caller-provided arbitrary schema arenas are not demonstrated. Host account
banks, Arc payloads, boxed/pinned calls, Rc registries, existing synchronization
and native SDKs remain legitimate Host implementations. Modern TD-driven
Planning/registration, continuing Actions, notifications, Producer publication,
credential composition, concrete Embedded transports and automatic schedulers
are not ported by this proof. All previously scoped Host evidence retains its
obligations; no package, milestone, gate or production profile is accepted here.
