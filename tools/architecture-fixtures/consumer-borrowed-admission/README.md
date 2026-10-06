# Borrowed Consumer admission discriminator

Non-production feasibility evidence for
[workspace 0075](../../../workspace/0075-consumer-borrowed-admission-construction.md).
That topic owns architecture rationale, selected policy meanings, migration
inputs and evidence limits. No production contract or admission status changes.

`src/td.rs` is compiled **inside** the existing TD source-projection candidate
with `borrowed-admission`. It reuses typed field enumeration, shared Basic and
schema discovery/predicates, and the already differential-tested URI kernel.
Only frame storage is acquired through the existing checked grow/transfer/
prepaid-release implementation; no full source arenas are built. Iterators
borrow the external Thing. Lookup comparisons retain positions and pay each
byte. Structural and Basic work share one unreplenishable lifetime allowance.

`src/planning.rs` is an external semantic consumer. It accepts only TD's private
proof, preflights and materializes complete owned inputs, performs the all-bounds
barrier, and compiles actual Core logical plans/artifact envelopes with the
existing MockCompiler. It uses checked fallible exact-layout string allocations
and measures that mock's inspected allocation behavior. A ready TD coordinate
is re-lent until copy credit is available, then acknowledged. The current
resolved URI buffer stays in TD; no reference into it survives moving TD state.

The output has a fixed sixteen-slot catalog; URI input is capped at 256 bytes
with a 512-byte current buffer and a fully prepaid conservative quadratic query
envelope. Inline output capacity is reported separately from heap requests.
The private one-slot Registry is a publication ownership model, not production
Servient, concurrent publication, global resource pairing or lease/drain proof.
There is no JSON decoder, generic compiler proof, target runtime/stack claim or
production variable-output cleanup implementation.

Run the focused checks:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml -- --nocapture
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --features order
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --no-default-features
cargo check --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --no-default-features --target thumbv7em-none-eabihf
```

The runtime tests exercise the shared whole-Thing Basic/first-error corpus,
budget chunk invariance, zero/insufficient credit, lifetime exhaustion, deep
opaque inspection, long common-prefix references, frame-grow movement/failure/
abandonment, Number limits/projection, relative URI lending, explicit empties,
unrelated invalidity, missing ID, later coordinate/bounds failure, actual
allocation accounting, concrete output after source/complete-registration destruction, and
publication rejection/install. Compile-fail doctests establish the private
proof, raw-input exclusion and source/derived lifetime restrictions. Host tests
also use allocator instrumentation as a test oracle; the library requires no
global counting allocator or std.

Heap ledgers cover TD frame requests and owned output separately; fixed
continuation/result sizes are reported as well. These are not full transaction,
parent/global or stack accounting claims. The workspace topic specifies those
owners and the remaining production evidence.

This crate locks the existing fixture dependency versions. `order` exercises
the alternate serde object backend; its upstream feature enables std. The
borrowed candidate explicitly enables AP for lexical Number access. A successful
thumb compile is only compilation, not constrained runtime or no-atomic proof.
