# Consumer runtime entry discriminator

Non-production evidence for the [entry investigation](../../../workspace/0079-consumer-runtime-entry-constructibility.md)
and the [Consumer-only registration candidate](../../../docs/work-packages/WP-300-consumer-only-registration-admission.md).
This isolated, non-publishable crate adds no production registration or runtime.

| Evidence | What runs | Limit |
| --- | --- | --- |
| Client-only construction | Actual Core `PollClientBinding`/target Host `binding::ClientBinding` implementations with no server trait; a closed native compiler or actual supported Host adapter; model complete inputs/owners | Model constructors cannot be installed, issue `ConsumerCompilerSupport`, mint seals or execute a request. No new production support/eligibility is established. |
| Rejection ownership | Wrong cell, compiler/client compatibility, actual static configuration, slot ceiling minus one/exact/plus one, transient overrun, declaration/retained-overflow rejection; exact client preservation and one destruction | Handwritten model checks, not a second authority or production validation. Host private support/configuration checks are deliberately not modeled. |
| Source-free local ownership | Actual production TD proof, paid aggregate preflight/materialization, static/public Host artifacts, source and complete-registration destruction, concrete target use, original cleanup root/account retained through explicit and abandoned-draft/cleanup reclaim | Uses completed dual-role registrations from the existing compiler fixture. Establishes a local unpublished owner; no new role-only production aggregate, call lease or Published record. |
| Auto traits | Compile-fail contract cases reject current static registration without a server and transferring/sharing current Planning output/root/rollback, despite thread-safe concrete artifacts | Deliberate local representation of the current aggregate. Not a defect in its accepted unpublished scope, nor proof that every Host runtime needs a shared carrier. |
| Capacity provenance | Two real independent Foundation child ledgers produce equal owner tuples and identical live/peak observations | Neither has a parent/global reservation. Numeric observations cannot establish transaction authority. No proposed public token/getter is introduced. |

The static model's closed compiler/configuration check combines only construction
facts; it does not emulate Core's trusted support issuer. The production
successor must test actual capture, unsupported/impostor/configuration rejection
and retained-slot allocation failure. Existing complete static-slot accounting
is projected with checked public item/byte arithmetic and `size_of`; model
agreement does not prove the future mediator or its exact representation cost.

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

Run with mainline's Rust 1.95.0 from the repository root:

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
