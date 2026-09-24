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
