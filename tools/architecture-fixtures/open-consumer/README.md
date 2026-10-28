# Independently linked finite Consumer

This crate is a standalone Cargo workspace, excluded from root membership. Its
library authors a real non-HTTP finite datagram Binding through public Core and
Planning APIs. The native request is an owned variable `Vec<u8>` assembled by a
resumable byte-by-byte compiler. Its destructor, compiler callbacks, native
calls, validation and final socket disposal are independently observable.
The peer speaks `READ /temperature\n` and returns a JSON integer datagram; it
uses an actual loopback socket, not a synthetic Binding result.

Run from the repository root:

```sh
cargo test --locked --offline --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml -- --test-threads=1
cargo check --locked --offline --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --no-default-features
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --no-default-features --edges normal
```

The integration binary composes this Binding and the existing HTTP participant
through one public TD-based selection/preparation path, then destroys TD/parser,
semantic and compiler owners before repeated real calls. Other cases exercise
per-operation support, ambiguity, selectors, schema/security refusals, source-free
funding, typed/static generic registration, failed/cancelled compilation, invalid
artifacts, bounded lifetime, actual allocator refusal, service/configuration loss,
late/unread results, replacement and pending/failed cleanup. The library's normal
`--no-default-features` graph has no TD/Planning/compiler dependency. Dev-only
integration participants are deliberately outside that normal graph.

The datagram mapping and integer codec are test participants, not a production
protocol or general JSON/schema implementation. The Host runtime is manually
progressed. These checks do not qualify notification I/O, SSE, constrained device
execution, authentication, every DataSchema assertion, or package/release gates.
