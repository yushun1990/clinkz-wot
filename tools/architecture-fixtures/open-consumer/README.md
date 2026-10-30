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
late/unread results, replacement and pending/failed cleanup. Failure-path
artifact destructors must observe their original byte funding; oversized
alternatives retain per-operation unsupported/selector reports; rotating runtime
priority must progress independent I/O and cleanup with one native credit. The library's normal
`--no-default-features` graph has no TD/Planning/compiler dependency. Dev-only
integration participants are deliberately outside that normal graph.

The datagram mapping and integer codec are test participants, not a production
protocol or general JSON/schema implementation. The Host runtime is manually
progressed. These checks do not qualify notification I/O, SSE, constrained device
execution, authentication, every DataSchema assertion, or package/release gates.

The same compiler/finite owner also executes explicitly synchronous Actions with
separate input/output type schemas, including a Property and Action sharing a
name. `tests/support/composition.rs` authors a separate raw observation Binding
over a shared real datagram socket. One independent Consumer authority fences
both lifecycles while local facet close and sibling native ownership remain
independent. These are test protocols, not new production Bindings or TD-qualified
notification support. The moved notification lifecycle regressions run in
`clinkz-wot-servient-finite`, with Core/Foundation as the only normal project
dependencies. HTTP SSE is deferred.

`tests/notification.rs` prepares an Event with a `dataResponse` schema through
the public typed notification registry and destroys TD/registry owners before
execution. Its in-memory response-capable Binding verifies that the composed
`decline_response` path retains pending reply ownership, data and funding, then
permits advance and the next Event without a send or subscription stop. Required
and already submitted responses remain non-declinable. This is composed API and
lifecycle evidence, not protocol response interoperability or MQTT response support.
