# Bounded literal value construction

Non-production construction evidence for `WP-100-CONSUMER-VALIDATED-THING`.
The tranche remains `planned` / `candidate` / `current`. This fixture neither
implements production `ValidatedThing` nor changes admission or gate status.

## Why this boundary

The accepted #117/#118 literal value contract is the input to this work.
The existing rich typed Snapshot, whole Basic kernels, URI/default/security
queries and post-Snapshot-drop Planning output already establish their local
boundaries. Their construction still uses recursive calls and fixed headroom;
the original allocation witness copies an entire grow/seal prefix at once.
Those mechanisms cannot be treated as complete bounded construction evidence.

This fixture composes the allocation, Number-token and literal-value proofs
into a complete **JSON value-layer** path. Both borrowed wire and already typed
`serde_json::Value` enter the same mutable arenas, ordering/duplicate resolution,
reachability/compaction and exact-length seal. A complete TD field-policy
extraction can use this substrate, but is not implemented or assumed here.
More known-field readback or another JSON authority decision would not test
these construction dependencies.

Evidence reused:

- #94's checked Layout, real Foundation reservation, allocator-failure and
  live/peak/contiguous accounting body. `staged.rs` calls that body only with an
  empty replacement, then moves one paid element at a time. The original
  synchronous fixture and its tests remain available.
- #95's actual `NumberLexeme`, including first-excess-byte stop.
- #117's unchanged synchronous public RawValue/scalar literal oracle, moved
  into shared **test-only** support. It is not linked into the engine cursor.
- #96–#109's existing actual typed corpus, including long strings, opaque AP
  Numbers, nested schema values and the Basic-valid serializer-failure case.
  The TD test bridge enumerates root extensions and Property schema JSON values
  outside the cursor, then runs their borrowed values through this construction.
- #93/#108/#110–#115 retain their configuration, date, semantic, URI and owned
  Planning boundaries. This fixture does not copy those rules or expand their
  claims. #107's WP-300 history remains disjoint.

## Construction and progress

The wire entry requires a complete JSON object. Nested values cover every JSON
kind, UTF-8, escapes and surrogate pairs. Objects and strings never dispatch on
member spellings or parse embedded documents. The typed entry borrows values
directly, preserving `Number::as_str()` without serialization or another decode.
Its guarantee is additional engine-owned memory over the caller's typed graph.

The four build sites are node, edge, byte and traversal-frame arenas. Frames
contain only scalars/borrowed public iterators; compile-time assertions exclude
destructors. Grow keeps the old block and one replacement charged, pauses after
each copied element, then frees/releases the old block. No owning Value, Map,
String, Number, serializer output, sort buffer or recursive task tree is built.

Edges initially identify their parent. Stable in-place insertion sorting groups
them by parent; only Object members additionally compare decoded key bytes.
Each comparison/move and each compared byte is paid. Equal keys retain source
order until duplicate resolution removes every earlier occurrence. Every
overwritten value has already been syntax-checked and charged. No field type,
null/default or discriminator decision is made by this value-layer fixture.

Nonrecursive reachability marks surviving nodes/edges. In-place edge, byte and
node compaction removes all unreachable occurrences, remaps scalar references,
preserves Array order and exposes sorted Object members. Each retained arena is
then allocated at exact length, copied incrementally, and substituted for its
build predecessor before the next arena is sealed. Frames are released first.
Physical node/byte order need not be identical across source map orders;
logical associations, map iteration and final requested bytes are equal.
These physical IDs are not frozen global semantic diagnostic ordinals.

One monotonic lifetime remainder pays every accepted class unit. Structural
transitions, emitted/copied records and sort moves use `DocumentNodes`; reads
and emitted/copied bytes use their codec classes. Before each actual allocation
request, one `CleanupItems` and lifetime unit prepays its fixed release.
Zero budget observes no input and allocates nothing. No step credit is stored.
Cumulative sort/copy statistics use `u64`, bounded by that lifetime; repeated
work can exceed address width even when each thumb arena offset fits `usize`.
Failure and cancellation expose a fixed first cause only after the same
idempotent, allocation-free bounded release path has cleared all live charges.
Abandonment and completed-owner drop use that path too.

Number lexing retains no token buffer: it records a borrowed wire range and
fixed scalar facts. The raw ceiling is enforced while lexing, including zero.
After grammar completion, the decoded length is checked **before any Number
output**. Emission reproduces public AP content (`-0` -> `0`, `E` -> `e`, absent
exponent sign -> `+`) directly into the byte arena, paying for rereads/output.
The 400-token public AP differential corpus includes integer boundaries,
fractions, signed/zero-padded/very large exponents and negative mantissas.
This is a public-oracle-tested adapter, not dependency-source extraction or
private token dispatch. No numeric projection is performed here.

The insertion sort and some cleanup scans have quadratic worst-case work.
They are charged and resumable; this fixture makes no throughput or final
production algorithm claim. Its finite `Limits` is test scaffolding, **not**
`ValidatedThingAdmissionConfig` or a declaration of the supported Number `M`.

## Resource and terminal evidence

The allocator observer has only fixed thread-local scalar slots. It measures
actual requests/live bytes independently of the ledger, and fails each real
request in turn for both entries. It checks every Pending boundary for physical
cursor abandonment, including partially moved non-Copy borrowed iterators.
One-unit steps independently classify temporary versus sealed requests and
verify temporary peak; larger steps verify total peak, largest request, live
source and releases. Oracle/input construction is outside these intervals.

For the documented 64-bit Host allocator witness, both entries report:

| Observation | Requested bytes / count |
| --- | ---: |
| Exact retained source | 568 bytes |
| Temporary peak | 2,304 bytes |
| Additional total peak | 2,304 bytes |
| Largest actual request | 896 bytes |
| Actual allocation requests | 22 |
| Maximum simultaneous blocks | at most 5 |

The cursor/progress occupy 840 inline bytes and the completed fixture owner 600
on that Host build. These are capacity observations, not heap requests, target
stack bounds or future Servient owner limits. Allocator metadata is excluded.
All successful owners leave only their nonempty three exact source arenas;
drop returns physical live bytes to baseline. Diagnostics and cleanup allocate
no records. No persistent-document conversion is used.

Tests cover every request failure; arithmetic before allocation; separate
below/equal/above source, temporary, peak and contiguous ceilings; structural
rejections; raw/decoded Number ceilings, 63/64/65 and 255/256/257; zero-disabled
Numbers; a 65,536-byte late-invalid exponent stopped after byte 65 of its token;
all construction phases' cancellation; lifetime non-reset/exhaustion; malformed
overwritten values; and a 512-level wire input with frame-limit rejection.

## Readmission contribution and limits

| Obligation | Actual contribution / remaining boundary |
| --- | --- |
| 1. Frozen API/config/removals | No full-surface claim. The fixture owner is `'static` after either borrowed source dies. Full frozen positive/negative signatures and opaque pre-entry config remain. |
| 2. Allocation construction | Integrates the existing Layout/ledger body with all literal kinds, the frame arena, charged grow, sorting, duplicate discard, compaction and sequential seal. Whole TD traversal/field construction remains. |
| 3. Typed/strict equivalence | Equal literal logical values and requested source footprints across both entries, including collision/duplicate/Number-content cases and actual old typed corpus values. Full shared TD field-policy extraction, known-field null/default/discriminator projection and whole Basic/query parity through construction remain. |
| 4. Planning | #111/#113 remain reusable. No new Planning or public ValidatedThingView claim. |
| 5. Supported cells | Host std/order/no-default/async execution and actual thumb no-default/async compilation. Full capability-off/on, sibling/downstream/public-surface matrix and constrained execution remain. |
| 6. Progress/rollback | Composed literal decode/unescape, Number capture, sort/duplicates, nonrecursive traversal, compact, grow/seal, lifetime, cancellation, allocation failure and prepaid drop. Whole TD Basic/date/URI/equivalence stepping, complete configuration and global diagnostics remain. |
| 7. Resources | Actual source/temporary/peak/contiguous requests, overlap and release agree with allocator observations for this layer. Full atomic projection support envelope/M, TD owner capacity, complete Snapshot/plan overlap, parent/global release, Published zero charge and permit/install proof remain. |
| 8. Prior impact | No production Foundation/Context, resource schema, successor evidence or gate status changes. Existing registered checks validate their unchanged inputs; full independent exact-head reaffirmation and future admitted API removal remain. |

No obligation is declared complete as a whole. In particular this fixture has
no Thing field-policy table, shared serde field extraction, Basic/date/URI
visitor or full typed Thing cursor. Its generic views are fixture scaffolding,
not the frozen public Planning view. Production admission and Consumer gate
registration still require the complete independently accepted evidence set.

## Reproduction

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-value-construction/Cargo.toml
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-value-construction/Cargo.toml --features order
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-value-construction/Cargo.toml --no-default-features
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-value-construction/Cargo.toml --no-default-features --features async
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-value-construction/Cargo.toml --no-default-features
cargo check --locked --target thumbv7em-none-eabihf --manifest-path tools/architecture-fixtures/validated-thing-value-construction/Cargo.toml --no-default-features --features async
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe
```

The stand-alone lock selects serde_json 1.0.149, matching the existing oracle.
AP is explicit for this value-layer fixture. It does not simulate ordinary
capability-off TD. `preserve_order` remains Host-only because it enables std.
The root workspace wiring is dev-only: three stand-alone fixture roots are
explicitly excluded, and the TD dev dependency connects the old typed corpus.
