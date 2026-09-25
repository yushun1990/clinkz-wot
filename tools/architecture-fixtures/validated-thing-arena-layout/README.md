# ValidatedThing arena layout prototype

This is non-production source evidence for item 2 of
[`WP-100-CONSUMER-VALIDATED-THING`](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission).
It does not admit the tranche or implement typed TD conversion. The records
contain only illustrative scalar ranges; the fixture deliberately does not
claim complete semantic equivalence, work charging, cleanup prepayment, or a
validated admission configuration.

## Reproduce

From the repository root:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-arena-layout/Cargo.toml
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-arena-layout/Cargo.toml
cargo fmt --manifest-path tools/architecture-fixtures/validated-thing-arena-layout/Cargo.toml -- --check
```

The thumb command compiles the `no_std + alloc` library; it does not execute a
target allocator or establish target memory availability.

## Allocation catalog and formulas

The fixed `Prototype` owns exactly four temporary arena handles: mutable
`RetainedNode`, `RetainedEdge`, and byte build arrays, plus a traversal-frame
array. Sealing retains exactly three possible arrays: exact-length node, edge,
and byte arrays. Empty arrays allocate zero bytes and retain no allocation.
All element types are explicit `Copy` scalar records with no destructor or
nested owning value. The prototype calls `alloc` and `dealloc` itself with
one checked `Layout::array::<T>(capacity)` for each nonempty array.

For one proposed allocation, let `r = Layout::size()`, `S` be live source
requests, and `T` be live temporary requests. Before the allocator call, the
prototype checks:

```text
r <= largest_contiguous_allocation_bytes_max
S + T + r <= peak_live_bytes_per_admission_max
S + r <= retained_source_bytes_per_owner_max       (source request)
T + r <= admission_temporary_bytes_per_operation_max (temporary request)
```

Every sum and the `Layout` construction is checked. Exactly one Foundation
`AdmissionLedger` reservation of `r` bytes precedes exactly one allocator
request of that `Layout`; a rejected reservation performs no allocation.
After successful allocation, the reservation commits. Growth copies the
initialized prefix to the new array, deallocates the old array, then releases
its old charge. Seal makes a source request with capacity equal to the used
length while the corresponding build array is still charged, copies that
prefix, and releases the build array. This records old/new overlap in both
the temporary peak (on growth) and conversion peak (on growth and seal).

The sealed footprint uses the sum of the three live source `Layout::size()`
values, the count of nonempty retained arrays, the ledger's largest actual
single request, the simultaneous temporary peak, and the ledger's total live
peak. The worked test has a 40-to-80-byte node grow, a 120-byte temporary
overlap peak, a 137-byte conversion peak, and a sealed 39-byte/three-allocation
footprint. Tests also cover zero and spare-capacity inputs, a rejected grow
that preserves the old initialized array, source and peak rejection before
seal, contiguous-request rejection, and checked `Layout` overflow.

## Scope of the evidence

This demonstrates that the frozen allocation catalog can be implemented using
the current Foundation ledger without treating an aggregate as one physical
request. It does not prove the complete TD `Thing` traversal fits that catalog.
In particular, typed-field decoding, map ordering, URI resolution, Basic
semantic sharing, all structural/work charges, and real-target allocator
observations remain separate pre-readmission obligations. A required fifth
temporary allocation category or fourth retained category in the full
conversion still returns the tranche to impact review.

## Typed corpus storage slice

`td/tests/support/normalized_snapshot_probe.rs` imports this arena source in
TD's test-only `Context` child module. It uses the exact same input constructor
as `td/tests/validated_thing_typed_corpus.rs`. The prototype walks typed
`Thing`, `ContextEntry`, `Form`, and extension `Value` directly and seals one
node, one edge, and one byte arena. Reserved contiguous edge ranges retain
sequence indices and map key/value pairs. Byte ranges retain URI and string
content and AP Number text. No snapshot element owns another allocation. The
input `Thing` is outside the snapshot ownership boundary.

Run `cargo test --locked -p clinkz-wot-td` from the repository root to execute
both consumers of the fixed corpus. The TD test dependency selects serde
arbitrary precision so borrowed `Number::as_str()` is available; the
no-default library remains unchanged.

The test-only placement allows inspection of private `Context.entries`
without expanding the production Context or ValidatedThing API. The fixed
headroom and recursive call stack are prototype mechanics; the full bounded
cursor, traversal-frame use, all known fields/variants, allocation growth,
work charging, URI/default/security queries,
and a shared storage-neutral Basic kernel remain open. Existing Basic is used
only while constructing the fixed typed input. The snapshot traversal does not
copy a semantic rule or claim validation of the sealed snapshot. This is
partial item-3 evidence, not readmission or item-3 completion.

## Nested JSON Object ordering slice

The snapshot probe now reserves each JSON Object's map edges, stores scalar
source-member indices in those slots, and insertion-sorts the slots by key
before emitting any child node or byte. Each sorted index selects its original
key and value together. This keeps nested Object and Object-in-Array layouts
identical across insertion histories, including `preserve_order`; Array and
Form/Context sequence edges retain their original indices. The sort uses only
scalar slots in the existing edge arena. Its input `Thing` and JSON maps are
caller-owned, and no sort buffer or additional allocation category is added.
The test checks full node/edge/byte equality, sorted keys, nested associations,
array-order sensitivity, opaque Number text, equal footprints, and exactly
three retained arena allocations.

Run the probe in separate Host invocations:

```sh
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --features serde_json/preserve_order
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --no-default-features
```

TD's test dependency enables AP in all three invocations, so the first is AP
and the second is AP + order. The separate feature-boundary matrix checks Host
base/order/AP/combined requests and actual resolved serde features; base and
order without the validated capability remain ordinary TD/Basic graphs and do
not run the AP-dependent snapshot probe. Exact work charging, bounded
resumable sorting, and complete item-3 equivalence remain open.
