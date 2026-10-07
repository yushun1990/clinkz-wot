# Borrowed Consumer pre-readmission evidence candidate

This non-production crate constructs the six replacement obligations in the
[Consumer admission contract](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission).
Its authority is [ADR-0013](../../../docs/ADRs/0013-work-package-scoped-implementation-admission.org)
and [ADR-0021](../../../docs/ADRs/0021-borrowed-consumer-td-admission.org), as merged
on master after PR #129 (`a7ed82b1d87843a6c71f549f023138a35793e422`).
[evidence.toml](evidence.toml) records the claim boundaries and repeatable checks.
The review subject is the complete Git/PR head containing these files; no
acceptance or implementation-completion status is recorded here.

Production sources, historical evidence, the candidate tranche, the Consumer
gate frontier and resource rows are unchanged. Readmission needs independent
exact-head acceptance followed by a separate admission-only PR. Production TD
completion must repeat these claims through its real public boundary, including
actual Host and no_std allocator/runtime observations.

## Construction and review map

`src/td.rs` compiles **inside** the existing TD source-projection candidate, under
a separate `readmission` feature. The source projection copies the actual TD
model and the previously differential-tested shared Basic/schema kernels; it
does not edit production TD. The historical `borrowed-admission` feature and
discriminator still compile their original implementation.

| Replacement obligation | New executable witness |
| --- | --- |
| 1. Capability and lifetimes | `matrix/downstream` compiles the exact progress, proof, diagnostics, lending and getter signatures; `check_boundary.py` runs 32 separately resolved graphs and 14 expected-diagnostic negatives; `tests/join.rs` actually destroys Thing and complete registration before using the consumed draft |
| 2. Complete typed meaning | `typed.rs` enumerates every supplied typed field and private Context entry; `inspect.rs` applies the typed-content resource scopes; `tests/support/oracle.rs` independently counts public fields and actual text-byte checksum; `boundary.rs` checks the whole Basic/first-rule corpus; `progress.rs` checks unrelated reachability, serializer failure, absent ID, operation vocabulary, common prefixes and Number amendment |
| 3. Support and progress | `policy.rs` checks the complete 27-kind operation projection before input/ledger/progress; `td.rs` retains actual map/byte/frame positions; `uri.rs` is byte-resumable; `progress.rs` checks atomic Number/support/work/semantic thresholds, full-debit retries, monotonic lifetime, URI observations and paid Ready facts; `join.rs` observes real UTF-8/scope accesses during repeated failed copy/cleanup debits |
| 4. Resources and ownership | `storage.rs` admits real checked Layout requests before allocator entry and retains old/new frame blocks through paid transfer; `resources.rs` matches requests/alignment/peak/largest/count, rejects all six heap limits before allocation, injects every TD allocation failure, exhausts suspension positions, pairs five capacity scopes, and retains a real upstream source charge; fixed-pool runtime includes startup storage/metadata/alignment |
| 5. External semantic join | `planning.rs` imports only trusted TD loans, captures the existing complete Core registration, constructs actual Core plans and concrete MockArtifacts; `join.rs` checks non-first original indices, complete/empty lookup, relative targets, later materialization/bounds failures with zero starts, source/registration destruction and fixed prepaid rollback |
| 6. Disjoint evidence and impact | Authority checks, Foundation/TD/Core/Planning tests, the completed Consumer leaf graphs and every registered Producer gate command are rerun; the historical strict/date/arena/handoff/discriminator suites retain their original claims |

The private TD implementation is split into the exact API facade (`api.rs`),
checked policy, complete typed enumeration, resource scopes, two-site storage,
and URI continuation. Planning lives in this separate crate and cannot access
the proof's input or TD scratch. Every retained TD reference/iterator borrows
the external immutable Thing; no suspended reference points into TD's movable
current URI. The only borrowed output target is a fresh Ready loan.

## Meaning, accounting and supported primitives

The typed-content-v1 oracle counts supplied text/key/URI/vocabulary/AP lexical
bytes, eight bytes per typed scalar, and eight date components. Tags, absent
fields and static Rust labels contribute no invented bytes. StringBytes counts
all supplied text except AP Number spelling. ExtensionBytes counts the existing
extra/const/default/enum scopes. Effective bytes start with the complete supplied
baseline and add the derived target bytes emitted in one complete semantic pass.
Rewind starts another pass without refilling lifetime work.

Field/resource inspection visits Actions, Events, schemaDefinitions, all schema
composition and URI-variable paths, responses/additional responses, security
payloads, links, metadata, date components, private Context and opaque values.
The test-only recursive public-field oracle is independent of Task/slot
enumeration and serialization. Actual text reads update byte and checksum
observations, rather than merely advancing declared lengths. The same shared
Basic rule/discovery owner supplies public Basic and the bounded candidate;
resource discovery completes before Basic. The shared first-rule diagnostic,
including original coordinates, is compared rather than just success/failure.
Serializer-failing valid input is borrowed directly. Explicit-empty operations
and security preserve their override semantics. Basic-valid unrelated Combo
definitions are not expanded into a selected plan.

The projection contains all 27 applicable kinds listed by `policy::CATALOG`;
unrelated NA is ignored. Missing limits, unrepresentable target positions/Layouts
and unsupported atomic envelopes fail construction without allocation, input or
an allowance. Its support checks include combined multi-class atomic cost,
not only each class separately. On 32-bit targets, a Host-only global value
larger than isize is not automatically supported; the named static profile's
limits and Layouts have their own representation envelope.

The resolved backends are standard BTreeMap and serde_json's BTree or
preserve_order/IndexMap adapter. Native iterator creation, advancement, seek/
ascent and end detection use conservative `len + 1` DocumentNodes envelopes.
Creating and advancing together, or advancing an owner while opening its map,
pay both envelopes. Schema traversal uses JsonSchemaNodes for its own native
primitive. Security searches retain the native iterator and both byte positions;
comparisons pay each byte actually read from both operands. The bounded adapter
uses no restarted `nth`, synchronous raw-map lookup or prefix replay.
Dense iterators have constant public iterator primitives; the same conservative
envelope covers them. This is logical work over the locked dependency backends,
not a private BTree node-layout or target instruction/cycle claim.

The fixed schema field program has a conservative 512-byte label/recognition
envelope plus the configured AP lexical bound. The Number primitive uses the
public `as_f64` after checking the borrowed spelling length. The tests cover
64/256, L-1/L/L+1, zero, failed projection (`1e309` really returns None), late
exponent/underflow/rounding/long mantissa cases, and the five amended generic
predicates. Caller-constructed NaN/infinity typed NumberSchema fields retain
the existing comparison semantics. No private float parser is copied.

Before an atomic action, all class debits and their summed TD lifetime debit are
checked together. Short credit changes neither budget, allowance, state nor
observations. One monotonic `DocumentValidationWorkUnitsMax` remainder survives
validation, proof movement, semantic queries and rewinds. Allocation acquires
prepaid fixed-block CleanupItems; cancellation/drop can release those blocks at
zero new credit. Semantic failures retain their original cause on later polls.

URI component discovery, authority scan, reverse merge search, prefix emission,
segment classification, path pop, query/fragment emission, relative-double-slash
shift and final UTF-8 validation retain positions and pay actual bounded byte
actions. Scratch capacity is the configured 16-KiB/4-KiB ceiling, not the
historical 256-byte fallback. Differential tests compare the public TD resolver
on 152 component cases and 1,500 segment/base combinations. Ready UTF-8 and
scope observations originate in the actual validator and iterator accesses,
including downstream scope copying. Re-loans never rescan or resize facts.

**Observed profile intersection:** the unchanged static 16,384-unit lifetime
rejects the tested 4-KiB *relative derived* target with Work Limit at observed
16,385. This is retained as a counterexample, with no changed profile value or
URI-length/configuration fallback. The full 4-KiB byte algorithm is separately
run with explicitly configured finite work 1,048,576, using the same operation
projection, URI capacity and progress machine. The gateway 16-KiB case uses its
unchanged work allowance. The unchanged static policy also reaches Ready for a
4-KiB absolute borrowed target and a 1,025-byte derived target. These witnesses
do not claim that every independently bounded resource maximum can coexist
within a named profile's lifetime allowance.

## Instantiated resource and cleanup catalog

All capacities below are checked on the selected compiler/target. These are
candidate formulas and observations, not new profile constants.

| Owner/site | Complete capacity and release formula |
| --- | --- |
| TD frames | `F = 2 * max(JsonNestingDepthMax, SchemaCompositionDepthMax) + 16`; actual `Layout::array::<Job>(c)`, c grows from 4 geometrically to F; old/new capacities coexist while one initialized trivial Job moves per paid action; no recursive frame destructor |
| TD current derived URI | At most one `Layout::array::<u8>(UriTemplateSourceBytesMax)`; reuse for each coordinate/pass; aliases have no owned buffer; validation frames are released before URI allocation |
| TD controlled heap allowance | Conservative `H = max(2 * F * size_of::<Job>(), URI ceiling)`; it covers old/new transfer and spare capacity without summing unrelated phase maxima; largest-request checks use each actual Layout, never H |
| TD owner and return | Two caller-reserved slots of the largest inspect/proof/semantic/progress Layout; includes the fixed 256-event observation catalog, diagnostics, policy, Work counters and iterator/URI state; the short event/config/budget fit reserved spare capacity |
| Fixed Planning witness | Two largest Build/Step slots; includes Draft's 16 Row slots, 16 lookup slots, 16 bounds, 16 scopes per row, one scalar MockCompilerCursor, cause/debit/rollback metadata; Draft's unused slots are included in sizeof |
| Planning strings/artifact | Exact checked byte Layout per nonempty string; logical inline reserve precedes materialization; runtime byte/request limits precede fallible allocation; the inspected MockCompiler's one target allocation is reserved before its callback |
| Registration | The unchanged complete StaticBindingRegistration and its compiler/server/inert client are caller startup capacity; no TD-derived fields or Host erasure delta is created; the draft contains no registration callback |
| Fixed allocator | 131,072-byte aligned caller-reserved arena, 512 placement slots, observer and all unused/placement/padding capacity paid once; no per-request malloc header or dynamic metadata; startup charge remains after children die |

`Job` has no Drop. On the supported 32/64-bit Layouts, at most 59 geometric
capacities including a final capped capacity fit the checked isize bound;
four events per successful allocation and the one reusable URI block fit the
fixed 256-event catalog. Terminal TD release touches at most replacement,
current-frame and URI blocks, each prepaid at acquisition. Physical release
precedes child ledger release. Five caller scopes reserve the complete
inline-plus-heap child envelope before input/allocator entry and release it
only after every child is physically dead. Exhaustion/occupied-parent examples
reject before entry and recover after release. These are allowance-composition
witnesses, not Servient slot/race implementation.

The System observer measures actual requested Layouts, successful allocations,
live/peak/largest/count and matching physical deallocation. It does not claim
malloc usable size, OS RSS or allocator-internal pages. The additional fixed-pool
runtime supplies an allocator with explicit complete backing/metadata/alignment
cost: global startup backing is counted once, child suballocations consume its
delegated capacity, and the startup parent retains its charge after TD cleanup.
The actual no_std candidate executes with this allocator on the native Host.
ARM evidence is compilation/Layout instantiation, not ARM execution.

`resources.rs` provisions a real engine-owned upstream Thing while its actual
allocator Layouts acquire original Foundation source and global-parent charges
before allocation. Lending does not transfer/reclassify/refill those charges;
they stay until real Thing destruction. Ordinary caller input is external
baseline, not a charge synthesized from typed byte counts or collection len.
This upstream setup is not a bounded serde-ingestion proof.

Observed with rustc 1.99.0 (`b940084d7`, 2026-09-28), LLVM 23.1.1:

| Layout | x86_64 native no_std + alloc | thumbv7em-none-eabihf compile |
| --- | ---: | ---: |
| Inspect / progress | 10,584 / 10,584 | 8,296 / 8,296 |
| Proof / semantic cursor | 10,216 / 11,448 | 8,112 / 8,816 |
| Job frame | 424 | 296 |
| Build / Step / Draft | 24,096 / 24,096 / 10,216 | 17,800 / 17,800 / 6,616 |
| Short semantic step | 192 | 112 |

All those alignments are eight; URI byte alignment is one. The emitted
`WP100_READMISSION_LAYOUT_WORDS` records the target compiler's actual values.
The tests recompute Layouts rather than assert these observations as constants.
Gateway F=144/native H=122,112; static F=80/native H=67,840,
thumb H=47,360. Native two TD owner slots occupy 22,896 bytes; the five-scope
gateway allowance is 145,008 bytes.

The nested native corpus observes peak 20,352, largest request 16,384, five
allocations and 29 paid frame moves. Its real upstream source, including the
observed boxed Thing root, retains 14,752 requested bytes. The concrete two-plan join retains 240 requested bytes in
18 allocations (largest 36); simultaneous current URI/output peak is 16,624.
The fixed allocator startup Layout is 176,832 bytes aligned to 64, including
backing and observer metadata; nested arena span is 25,440, including any unused
gaps, and observed alignment padding is zero for these aligned requests.
Fixed-pool and System live/request observations agree.

The cancellation/drop corpus exhausts 499 inspection/Basic positions, 159
semantic positions and 718 external-build positions. Both explicit cancellation
and abandonment are checked, including transfer and a started owned compiler.
The closed MockCompilerCursor's abort is a scalar primitive and is observed
exactly once whenever owned. Fixed Draft destruction visits at most 16 rows and
16 lookup rows, with at most 401 separately prepaid string/artifact blocks;
there is no recursive caller Thing destruction. Every TD allocator request and
every fallible materialization request before the bounds barrier is injected
with null, retaining first cause and zero starts/controlled child bytes.

## Reused evidence and deliberate successor boundaries

Reused rule owners are the source-projected shared Basic/schema kernel and
typed/private Context access seam, the existing public TD resolver as an oracle,
the typed and Basic corpora, the complete handoff registration and the concrete
property-read MockCompiler. The discriminator supplies the starting lending/
position/owned-join construction and actual Ready scan instrumentation. Its
small atomic URI/policy/parent/publication model is not reused as replacement
proof. New storage/projection/URI/oracle/parent/runtime witnesses discharge
those candidate construction claims here.

Foundation #69 borrowing/linear primitive evidence and Context #70's existing
public semantics remain applicable; general resource/work primitives and the
196-row/value schema are unmodified and rerun. Completed exact WP-100 call/
response, WP-200 selected leaf and WP-300 name-free execution evidence retain
their scopes. Every command in the passed Producer gate is rerun, including
seven real Host Zenoh loopback feedback tests. No falsified disjoint claim was
observed. The orphan source-to-persistent-document method is still present and
must be removed in its future admitted production change.

Historical strict-value construction, field classification, atomic Number,
RFC3339, arena, semantic and Snapshot handoff evidence is preserved. Its new
meaning is not retrospectively strengthened. Strict wire ingestion, decoded AP
spelling growth and resumable date construction belong to future ingestion.

The external witness is intentionally **fixed**. Generic compiler declarations,
fallible artifact allocation/rollback, compiler-Pending resource/lifecycle
contracts, variable output cleanup and exact production PlanFootprint remain
WP-200. The existing MockCompiler's Box allocation is infallible on OOM; null
injection therefore claims TD and fallible pre-bounds materialization only.
It is not a generic artifact-allocation failure witness. Complete Servient
policy/registration capture, upstream/global races, generation/cleanup slots,
Host erasure, permit/install, cancellation linearization, leases/drain and
runtime reclamation remain WP-400. Real constrained runtime/cycle/stack/RAM
claims and a Consumer gate require the later production owners and review.

## Reproduce at the review head

From the repository root (add `--offline` when the locked dependencies are cached):

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-readmission/Cargo.toml -- --nocapture
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-readmission/Cargo.toml --features order
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-readmission/Cargo.toml --no-default-features --features validated-thing,async
python3 tools/architecture-fixtures/consumer-borrowed-readmission/check_boundary.py
cargo rustc --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-readmission/Cargo.toml --lib --no-default-features --features validated-thing -- --emit=llvm-ir
cargo rustc --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-readmission/Cargo.toml --lib --no-default-features --features validated-thing --target thumbv7em-none-eabihf -- --emit=llvm-ir
tools/check-design-artifacts.sh
```

The 29 runtime tests execute under default BTree, preserve_order and native
no_std + alloc/async graphs. The matrix separately checks eight Host default,
four native no-default, eight thumb and twelve import/unification cells; the
serde-only capability negatives cannot pass because another cell enabled it.
No_std library code uses the same progress/allocations, not a Host driver copy.
Mainline CI runs these checks alongside unchanged historical and production
regressions. ARM LLVM globals can be decoded as little-endian 32-bit words to
repeat the Layout table without running on an ARM device.
