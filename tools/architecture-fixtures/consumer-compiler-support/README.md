# Closed Consumer compiler support construction

This isolated, non-publishable crate retains the construction model for
[topic 0077](../../../workspace/0077-consumer-compiler-admission-boundary.md)
and the [admitted Core precursor](../../../docs/work-packages/WP-200-consumer-compiler-support-admission.md).
Its library and `witness` binary are historical model evidence. The
[`production` integration tests](tests/production.rs) reuse
[Core's external completion tests](../../../core/tests/consumer_compiler_support.rs)
against this isolated dependency graph. Those tests construct the actual public
typed and supported Host complete registrations and production compiler. No
manifest or root dependency change is needed.

```sh
RUSTUP_TOOLCHAIN=1.95.0 cargo run --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml --bin witness
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml --no-default-features
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml --no-default-features --features async
RUSTUP_TOOLCHAIN=1.95.0 cargo fmt --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml -- --check
```

The lock was seeded from the root lock and pruned by Cargo. `--offline` assumes
the same registry dependencies are cached. No private Core constructor, TD
semantic copy, generated source overlay, process allocator policy or concrete
protocol dependency is used.

The closed compiler copies one already resolved UTF-8 target into a 64-byte
array. Its actual immutable capacity/configuration participates in capture.
The source bodies are fixed; there is no user-defined lowering or destructor.
Forced start/step failures are explicit fixture scenarios, not proposed public
production modes. Endpoint implementations make complete registration validation
real but reject execution starts; their unreachable polling methods deliberately
panic. This is compiler support evidence, not protocol execution.

| Claim | Executable discriminator |
| --- | --- |
| Complete owner and actual implementation | Private closed construction; real dual-role `StaticBindingRegistration` validation; same-compatibility/same-associated-type impostor rejected without a preparation callback |
| Actual configuration | A startup-valid bundle whose digest describes a different private compiler capacity is rejected before callbacks |
| Eligibility | Absent support, five qualified registration-identity mismatches, another registration's configuration, Producer-only capability, invalid capacity and ordinary public Host erasure rejected; original complete owners returned |
| Paid primitive and lifetime work | Bounds cost paid before callback; zero/one-unit progress retries invoke no callback; two paid steps use one non-resettable two-unit compiler remainder; retrying Failed with a fresh budget cannot refill it |
| Static storage | No observed allocator requests/releases during bounds/start/unpaid/paid Pending/Complete or artifact destruction; source and complete registration are actually destroyed before target use; paired output identities match |
| Held Host transport model | One exact nonzero Layout acquired fallibly before native start; address unchanged through unpaid retries, paid Pending, Failed and Complete; output target/type/compatibility checks survive actual source/registration destruction |
| Acquisition and terminal ownership | Logical capacity, largest-request and unpaid release negatives do not call allocator/compiler; actual null acquisition restores the original ledger and preserves another held owner; start/step failure and abandonment release once, aborting exactly once only when a cursor exists |
| Public construction boundary | Rustdoc negative cannot write the checked carrier's private fields; downstream cannot choose an implementation/cost/attestation constructor |
| Feature cells | The same complete-registration/static-output test and privacy negative run against std, no-default and async without std; std adds the allocator-observed transport binary |

The binary is single-threaded. Its fixed atomic observer records requested
allocation/release Layouts with no allocation in hooks. Failure injection is
armed only around the model's single known physical acquisition. It is a test
instrument, not a hidden allocator governor or proof that arbitrary native
allocations are bounded. Logical reservations alone are never reported as
physical backing.

On Rust 1.95.0/x86_64 the model reports:

```text
reserved Host: slab=(160, 8), owner=(280, 8), binding payload=72; one allocation retained through Pending and Complete
```

The slab includes the enum, native cursor/completed `BindingCompilerOutput`,
metadata, padding and inline binding payload. Adding another 72 bytes as a second
payload allocation would double count it. The 280-byte outer model owner includes
its owned Foundation ledger and registration loan; it is not a production Core
wrapper constant. Native completion and the slot coexist briefly; the compiler
declares a conservative native-result temporary Layout. These observations are
not a whole-process/allocator-metadata peak, exact production PlanFootprint or
target-device measurement. Startup registration storage and caller input are
outside these observation windows.

The successful Host result is a **new transport model borrowing a complete
typed Core registration**. It is not a successful call through production
`HostBindingCompilerRegistration`. The binary separately constructs a real
Consumer-capable `HostBindingRegistration` and rejects its current ordinary
adapter as unsupported, even for the known compiler. The production tests now
repeat the positive, failure injection, payload projections and ownership
evidence through the real public Core surface, including its fallible startup
adapter acquisition. Their measured Layouts are independent of the model's
constants.

No aggregate, variable rollback, parent/global governor, installation, Producer
execution, real protocol, bare-device runtime or acceptance is proved.
Independent completion acceptance remains required. The existing allocating
mock is not made eligible.
