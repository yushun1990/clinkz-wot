# Consumer runtime entry evidence

`src/production.rs` is a downstream client with no server trait or component.
`tests/production.rs` constructs the actual Consumer-only Core owners, captures
closed support and runs the public paid Planning aggregate. Both representations
execute concrete artifacts after TD destruction, then destroy requests and the
separate complete registration and explicitly drive the original cleanup
root/account to terminal. Allocation failure runs cover every materialization
and supported Host compiler-slot request. The existing exact-source observer
also verifies the all-bounds barrier for both new checked adapters.

The std production tests also exercise the public Planning terminal ownership
bridge. A paid Host output moves to a second thread, reattaches to the original
selector, executes the concrete artifact through Core's sealed call path, and
returns as the same unique backing. A partially paid rollback then moves to a
second thread and resumes the existing bounded reclaimer from its exact byte
position. The original cleanup account stays separately charged until the
original node and every child have been released.

The production tests measure static and Host layouts, ownership owner/return
layouts, separate provisioning, output and call allocations, and zero retained
bytes after explicit cleanup.
The normal library exposes target layout observations for the two ARM compile
cells. ARM compilation is not device runtime or device-fit evidence. Core's
public tests separately cover result-shape failures, cancellation, compiler
support rejection and the two supported fallible allocation sites.

The table below retains the original pre-implementation discriminator and its
limits. Its model constructors are not production eligibility evidence.

Non-production evidence for the [entry investigation](../../../workspace/0079-consumer-runtime-entry-constructibility.md)
and the [admitted Consumer-only registration tranche](../../../docs/work-packages/WP-300-consumer-only-registration-admission.md).
This isolated, non-publishable crate defines no product runtime or authority.

| Evidence | What runs | Limit |
| --- | --- | --- |
| Client-only construction | Actual Core `PollClientBinding`/target Host `binding::ClientBinding` implementations with no server trait; a closed native compiler or actual supported Host adapter; model complete inputs/owners | Model constructors cannot be installed, issue `ConsumerCompilerSupport`, mint seals or execute a request. No new production support/eligibility is established. |
| Rejection ownership | Wrong cell, compiler/client compatibility, actual static configuration, slot ceiling minus one/exact/plus one, transient overrun, declaration/retained-overflow rejection; exact client preservation and one destruction | Handwritten model checks, not a second authority or production validation. Host private support/configuration checks are deliberately not modeled. |
| Source-free local ownership | Actual production TD proof, paid aggregate preflight/materialization, static/public Host artifacts, source and complete-registration destruction, concrete target use, original cleanup root/account retained through explicit and abandoned-draft/cleanup reclaim | Uses completed dual-role registrations from the existing compiler fixture. Establishes a local unpublished owner; no new role-only production aggregate, call lease or Published record. |
| Auto traits | Compile-fail contract cases reject current static registration without a server and transferring/sharing current Planning output/root/rollback, despite thread-safe concrete artifacts | Deliberate local representation of the current aggregate. Not a defect in its accepted unpublished scope, nor proof that every Host runtime needs a shared carrier. |
| Capacity provenance | Two real independent Foundation child ledgers produce equal owner tuples and identical live/peak observations | Neither has a parent/global reservation. Numeric observations cannot establish transaction authority. No proposed public token/getter is introduced. |

The static model's closed compiler/configuration check combines only construction
facts; it does not emulate Core's trusted support issuer. The production
tests above and Core support tests exercise actual capture, unsupported/impostor/
configuration rejection and retained-slot allocation failure. Existing complete static-slot accounting
is projected with checked public item/byte arithmetic and `size_of`; model
agreement does not replace the production mediator and its measured representation cost.

The local owner keeps output, manual cleanup destination and the original charged
Foundation account together. Zero-budget reclaim is Pending. Dropping a draft or
cleanup lease returns work to that destination; each explicit driver keeps the
charge until terminal refresh and slot destruction, then releases it. The tests
do not drop the entire root with pending work: production recovery from that
event remains an unproved runtime obligation. The retained charge is logical
accounting; these tests do not measure allocator backing, machine stack, global
simultaneous peaks, fragmentation or device RAM.

The advanced std bridge consumes only a matching draft/root or variable
rollback/root pair. Wrong equal-looking, vacant, occupied and fixed roots return
the original owners without work. Zero credit returns Backpressure; exact credit
prepays detach and reattachment. Public compile contracts keep local handles
local and reject clone/forge/Sync/non-Send transfer claims for the unique owner.

At the pre-implementation boundary, source inspection found infallible `Box::pin` in
`HostBindingCallBox::new`, reached for raw call/decorator construction. That is
not exercised as an OOM recovery test here; null allocation can abort. A paid
compiler slot or retained-footprint declaration does not supply physical backing
or bounded behavior for an arbitrary client callback. The future runtime
candidate must choose and prove its call/cleanup admission boundary.

## Construction boundary experiments

`tests/construction.rs` adds bounded discriminators against baseline `e725de4`:

| Experiment | Result on Rust 1.95.0, x86_64 Linux | Limit |
| --- | --- | --- |
| Local worker | Actual paid aggregate, original root/account, bounded one-element command/reply lanes; caller abandons reply before zero-credit Pending then paid sealed call settlement and reclaim | Demonstrates feasibility of a local driver, not runtime scheduling, multiple calls, fairness, deadlines or worker/executor shutdown recovery |
| Quiescent output | Same node and artifact addresses cross a thread in one unique owner; detach/reattach each make zero allocator requests; original selector/sealed Core call/reclaimer; original node freed exactly once. Native Layouts: node 1376 bytes, original local pair 616 bytes, each unique owner 24 bytes, all aligned to 8 | Private admission precursor retained as comparative evidence; the production proof now uses the public bridge. No Published record or drop recovery. |
| Partially paid rollback | Matching node/root and original account cross a thread; zero-allocation conversions retain reclaim observations and partial byte credit; abandonment returns to the same original root | No second reclaim algorithm; fixed pre-allocation cleanup deliberately outside bridge scope |
| Root discrimination | Two equal public reservation tuples fail actual pointer matching, return both graphs for complete reclaim; short credit returns the original pair | Public identity equality is not ownership; no parent authority is issued |
| Shared graph | Public compile negatives reject `Arc<Draft>` and `Arc<Mutex<Draft>>`; a detached unique owner can be parked under a mutex with structural Send | Neither a full shared carrier nor callback-under-lock behavior is implemented |
| Parent construction | Existing real Foundation parent account is held/exhausted before actual cleanup-root construction; original child is released before parent | Cleanup capacity only; no full build/runtime governor, provenance certificate or simultaneous global peak claim |
| Host allocation failure | Actual Core installed start has five allocator requests. Each injected null aborts in a subprocess. Final failure follows raw acceptance in **both** normal and accepted-overrun wrapper branches | Deterministic fixture client; no real protocol, arbitrary callback bound, recoverable OOM or new call support |

`build.rs` copies the current production Planning source verbatim apart from
leading module documentation and preserves its existing relative test-support
module. It appends `probes/quiescent.rs` in the same private test module; no
production file is patched. The appendix replaces only ownership at a quiescent
two-reference boundary, with an ordinary `Box` over the original allocation.
It supplies no unsafe Send/Sync implementation and no second semantic program.
Its unsafe allocation round trip requires independent review. Whole-root
abandonment still leaks rather than performing unbudgeted variable destruction,
just as the original advanced manual root does; the tests explicitly retain and
drive it. The public implementation and completion obligations are owned by the
[admitted tranche](../../../docs/work-packages/WP-200-consumer-output-ownership-admission.md);
the private appendix does not satisfy them.

The historical fatal-allocation tests armed failure only in subprocesses, marked whether the
raw client had returned acceptance, checked the allocation-abort diagnostic and
on Unix required SIGABRT. They exercised actual Core calls rather than a model
allocator return path. A fallible raw box alone would leave Core's accepted
wrapper allocation gap. No attempt is made to recover from abort with panic
handling. These tests add no new checker or fixture package; the existing
opt-in `manual-validation` full runtime-entry command executes them.

Run just the discriminators with output:

```sh
cargo +1.95.0 test --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --test construction -- --nocapture
```

Run the full fixture with Rust 1.95.0 from the repository root:

```sh
cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml
cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --no-default-features
cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --no-default-features --features async
cargo check --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --no-default-features --target thumbv7em-none-eabihf
cargo check --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --no-default-features --features async --target thumbv7em-none-eabihf
```

Portable tests run on the Host. ARM checks compile this normal library and its
actual production dependencies, not tests, a linked firmware, protocol backend or
hardware runtime. The lockfile starts from the accepted aggregate fixture's pins;
no dependency update is intended. Opt-in full validation runs these commands without
`--offline` to populate its clean runner cache.

Passing this fixture supplies executable implementation evidence for the admitted
output-ownership tranche. It is not independent acceptance and does not mark the
tranche complete. WP-400 implementation is outside this fixture; no Consumer
gate or milestone status is changed.

## Historical Host call construction alternatives

The [WP-300 candidate](../../../docs/work-packages/WP-300-consumer-host-call-construction-admission.md)
selects a private complete Core envelope backed fallibly **before** raw invocation,
with an optional recoverable raw-call constructor. It keeps both installed
start signatures and the raw SPI. This fixture adds no production code or admission.

`probes/run_host_call_candidate.py` archives its explicit `--source-revision`
(default `988acf3`, the admitted pre-implementation master) to a disposable
directory, optionally applies one checked experimental patch there, and appends
`probes/host_call_candidate.rs` to the existing Core binding integration test.
It runs the actual Core program and existing validators/tests, not a second
sealing algorithm. The selected patch touches only disposable copies of Binding
and the existing compiler allocation primitive's visibility. The inline comparator
also changes the disposable Core export and one cleanup-carrier type alias.
Production files, manifests, locks and CI workflows are not patched. This is a
small reproduction driver, not a checker or new fixture package.

Run from the repository root; use `--toolchain 1.95.0` to reproduce the original
compiler. The driver prints the compiler actually used (`stable` may change):

```sh
python3 tools/architecture-fixtures/consumer-runtime-entry/probes/run_host_call_candidate.py --baseline
python3 tools/architecture-fixtures/consumer-runtime-entry/probes/run_host_call_candidate.py --alternative preallocated
python3 tools/architecture-fixtures/consumer-runtime-entry/probes/run_host_call_candidate.py --alternative inline
```

The driver prints the source commit and compiler version. Patch contexts must
apply exactly. All Cargo commands are locked/offline and serialized. Each variant
reuses Core's two focused integration suites (14 compiler cases and 20 binding
cases) plus four constructor discriminators. Candidate variants also compile Core
in no-default and async-no-std cells. Offline reproduction needs the locked
dependency cache; populating a missing cache does not justify changing pins.
No whole-workspace, ARM/device or full performance run is asserted here.

At production baseline `d22d5af6ae98211d56709b9ffebcf4fe8a86c601`, native
x86_64 Linux/Rust 1.95.0 observations are:

| Variant | Normal allocation requests, in order `(bytes, alignment)` | Accepted-overflow requests | Requests after raw acceptance | Installed call / Result Layout |
| --- | --- | --- | --- | --- |
| Existing Core, fixed-output client | `(256, 8)`, `(112, 8)` | `(256, 8)`, `(344, 8)` | 1 in either branch | `(16, 8)` / `(432, 8)` |
| Selected pre-backed complete envelope | `(344, 8)`, `(256, 8)` | Same | 0 in either branch | `(16, 8)` / `(432, 8)` |
| Inline installed owner comparator | `(256, 8)` | Same | 0 in either branch | `(344, 8)` / `(432, 8)` |

The fixed-output client isolates backing: prepare a single response before call
construction, restrict the URI map to the one fixed `room=west` input, use a
nonblocking one-element source, and attempt one concrete raw box. Its constructor
performs no protocol I/O. Its inspected finite input/output allocation requests
plus the actual 256-byte state fit a conservative 2048-byte raw declaration;
registration/shared source owners remain separate. The observation does not
establish a global ledger, parent reservation or allocator metadata bound.

The baseline null failures abort before raw acceptance at site 0 and after it
at site 1, verified in subprocesses (SIGABRT on Unix). The selected candidate's
site 0 returns the identical request without invoking the raw client; site 1
returns it from the failed concrete raw owner. Both preserve its original URI
string address, perform zero raw footprint/poll callbacks on failure and release
every successfully allocated observed block once. In the inline comparator,
site 0 is recoverable and site 1 is never attempted. These experiments cover
both normal and accepted-overrun construction. The earlier production fixture
above separately reproduces all five original allocations, including output
construction; this fixed-output witness does not hide those original failures.

Other discriminators check exact ceiling minus-one/exact/plus-one and arithmetic
overflow, raw rejection after envelope provisioning, no constructor raw polling,
zero allocation during the fixed result step, and exact raw/envelope release.
The reused production tests cover Consumer-only and dual-role Core sealing,
all invalid output shapes, normal/late failures, deadlines, Pending/zero-budget,
accepted/rejected cleanup transfer, static acknowledgement and Producer compatibility.
The copied Core result/cancel/transfer mediators are unchanged in both alternatives.

The selected envelope retains the generic 16-byte boxed carrier and existing
cleanup type while moving the allocation earlier. Its normal wrapper grows from
112 to 344 bytes; record and reserve this actual maximum, including raw/seal/error/
variant/vacant storage. The inline alternative removes that heap allocation but
grows the installed carrier by 328 bytes and changes both installed return types.
The Result's size stays unchanged because its existing rejection variant is larger.
`size_of` observations are not machine-stack or simultaneous physical-peak proof.

These results demonstrate a constructible admission target, not public production
support. They do not certify arbitrary synchronous `invoke`, footprint/deadline,
poll/cancel or Drop behavior, real protocol libraries, variable input destruction,
executor/root shutdown or WP-400 eligibility. The candidate record defines
additional production tests (including ZST/over-aligned/pinned owners), concrete
cost attestations, independent admission, implementation acceptance and separate
completion. Required GitHub `validation` checks diff hygiene only; scoped local
commands supply the evidence under the new CI policy, with no manual full run.

## Production Host call construction evidence

The production implementation is in Core, within the
[admission envelope](../../../docs/work-packages/WP-300-consumer-host-call-construction-admission.md).
[Ready completion evidence](../../../docs/evidence/WP-300-consumer-host-call-construction.toml)
owns the exact source boundary, executed checks and outstanding acceptance.
Tranche, gate and milestone state are unchanged.

`core/tests/consumer_property_read_binding/call_construction.rs` exercises the
public `try_new` and both complete installed Host registrations. Its fixed
observer pairs actual pointers and allocation/deallocation Layouts. The witness
accepts only the single `room=west` URI map, with the existing plan 72/binding
43/configuration `[47; 32]`/compatibility `[41; 16]` artifact helpers and one
prebuilt four-byte response. It retains the original request, uses a nonblocking
one-element source, and makes one recoverable concrete allocation. Its raw state
is `!Unpin`; address checks through footprint, result/cancel callbacks and Drop
falsify movement of pinned concrete state during envelope construction or transfer.

The constructor performs no protocol I/O or response allocation. Footprint and
deadline callbacks inspect fixed fields. Every paid result/cancel step consumes
one `BindingPolls` unit; zero credit returns Pending without progress. Ready
output needs no wake registration, and zero-credit progress requires caller
credit/refill. Cancellation has a finite immediate/next-poll settlement or an
explicit complete-owner transfer. Destruction releases one fixed request map,
at most one fixed payload, one shared counter reference and the concrete box.
Failed raw construction first recovers the request; both failed destruction and
terminal release perform no allocation. These are source and executable bounds
for this finite test witness, not a wall-clock or arbitrary binding attestation.

Native x86_64 Layout observations agree on Rust 1.95.0 and 1.99.0:

| Representation | Bytes / alignment |
| --- | --- |
| Complete Core envelope, disposition, and backing-attempt return | 344 / 8 each |
| Normal mediator / accepted-error mediator | 112 / 8; 344 / 8 |
| Private seal / fixed Core error | 76 / 4; 232 / 8 |
| Witness raw pinned state / raw construction return | 400 / 8 each |
| Installed boxed carrier / installed start return | 16 / 8; 432 / 8 |
| Request owner / inline raw result / cleanup transfer transport | 128 / 8; 232 / 8; 424 / 8 |
| Over-aligned pinned state / recoverable construction return | 256 / 256 each |
| Zero-sized concrete state | 0 / 1; no allocator request |

The observed construction allocation order is Core `(344, 8)` then raw
`(400, 8)`, with 744 bytes simultaneously live in that observation interval and
zero requests after acceptance in normal, finite overrun and arithmetic-overflow
paths. Both blocks remain owned until explicit final release. Core charges one
item and the complete 344 bytes once in both branches; the normal branch's old
112-byte backing measurement is superseded.

The prebuilt request separately retains 552 heap bytes (one 544-byte BTree node
and two four-byte strings). Response construction attempts `(4, 1)`, `(24, 8)`
and `(16, 1)`: the temporary Vec is freed, leaving 40 bytes of response backing.
These observations plus the 400-byte raw state fit the witness's seven-item,
2048-byte raw allowance; the complete call reports eight items and 2392 bytes.
The client and shared statistics are separate preexisting owners. The declared
2048 transient bytes conservatively cover the witness's concrete construction
and failed-state transport/destruction; Core's already charged vacant 344-byte
backing overlaps raw construction. Type Layouts describe inline transport,
not measured stack frames. No second envelope or static slot is charged.

`tests/construction.rs` now observes five allocation requests in its older
downstream client: Core backing failure recovers, while its four infallible raw
construction allocations still abort **before** raw acceptance. It supplies a
negative eligibility discriminator, not a supported bounded client. Its ordinary
Host aggregate calls now measure 512/520/512-byte heap peaks and a 584-byte
complete declaration, replacing the historical 280/288/280 and 352-byte values
only at this new implementation boundary.

The future runtime must hold declaration/cleanup capacity before construction,
including vacant/raw overlap and failure destruction, and retain the original
owner until settlement and final release. These tests neither issue parent/global
reservation authority nor prove physical device fit, allocator metadata,
fragmentation, whole-process peaks, protocol eligibility or WP-400 policy.

Run production evidence directly:

```sh
cargo +stable test --locked --offline -p clinkz-wot-core --test consumer_property_read_binding --test consumer_compiler_support -- --nocapture
cargo +stable test --locked --offline -p clinkz-wot-core --lib host_consumer_call_layouts -- --nocapture
cargo +stable test --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --test construction -- --nocapture
```
