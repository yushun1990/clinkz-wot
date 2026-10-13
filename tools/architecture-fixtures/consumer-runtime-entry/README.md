# Consumer runtime entry evidence

`src/production.rs` is a downstream client with no server trait or component.
`tests/production.rs` constructs the actual Consumer-only Core owners, captures
closed support and runs the public paid Planning aggregate. Both representations
execute concrete artifacts after TD destruction, then destroy requests and the
separate complete registration and explicitly drive the original cleanup
root/account to terminal. Allocation failure runs cover every materialization
and supported Host compiler-slot request. The existing exact-source observer
also verifies the all-bounds barrier for both new checked adapters.

The production tests measure static and Host layouts, separate provisioning,
output and call allocations, and zero retained bytes after explicit cleanup.
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

Source inspection additionally finds infallible `Box::pin` in
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
| Quiescent output | Same node and artifact addresses cross a thread in one unique owner; detach/reattach each make zero allocator requests; original selector/sealed Core call/reclaimer; original node freed exactly once. Native Layouts: node 1376 bytes, original local pair 616 bytes, each unique owner 24 bytes, all aligned to 8 | Private experiment; no public production ownership API, Published record or drop recovery |
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
drive it. The proposed public bridge and production completion obligations are
owned by [its candidate](../../../docs/work-packages/WP-200-consumer-output-ownership-admission.md).

The fatal-allocation tests arm failure only in subprocesses, mark whether the
raw client has returned acceptance, check the allocation-abort diagnostic and
on Unix require SIGABRT. They exercise actual Core calls rather than a model
allocator return path. A fallible raw box alone would leave Core's accepted
wrapper allocation gap. No attempt is made to recover from abort with panic
handling. These tests add no new checker or fixture package; the existing
mainline runtime-entry command executes them.

Run just the discriminators with output:

```sh
cargo +1.95.0 test --locked --offline --manifest-path tools/architecture-fixtures/consumer-runtime-entry/Cargo.toml --test construction -- --nocapture
```

Run the full fixture with mainline's Rust 1.95.0 from the repository root:

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
no dependency update is intended. Mainline runs these five commands without
`--offline` to populate its clean runner cache.

Production completion must exercise new complete role-only public constructors,
support capture, aggregate adapters, sealed normal/late execution and exact
resources, and reaffirm intersecting Producer evidence. Passing this fixture is
neither that completion nor independent admission review. WP-400 stays
unregistered; no Consumer gate or milestone status is changed.
