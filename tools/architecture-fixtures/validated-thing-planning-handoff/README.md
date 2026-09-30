# ValidatedThing post-Planning handoff witness

This external, non-production crate supplies the post-drop ownership part of
[pre-readmission item 4](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#evidence-required-before-separate-readmission).
The authority is the build-scoped lifecycle migrated by PR #112 and the
[owned aggregate boundary](../../../docs/spec/planning.md#first-consumer-property-read-aggregate).
WP-100 remains `planned` / `candidate`. This fixture supplies no admission,
production implementation, completion evidence, or gate transition.

## Construction and external boundary

`td/tests/support/planning_handoff_probe.rs` connects the existing three-arena
Snapshot and PR #111's frozen-API-shaped borrowed views to this external crate.
Its adapter delegates every query to those established views and shared TD
default/security/URI kernels. The only Operation conversion bridges the two TD
crate instances created by Rust unit-test compilation; it is an exhaustive
vocabulary conversion, not a second defaulting rule.

`ViewSource` and `ValidatedThingView::from_source` are test plumbing, outside the
frozen production surface. The facade forwards queries and exposes the frozen
Thing/Property/Form/security view methods and iterator lifetimes. Its backend
is private to `view.rs`; `planning.rs` receives only `ValidatedThingView`.
There is no Thing reconstruction, snapshot parsing, raw arena coordinate,
URI resolution, or default/security walker in that consumer.

The borrowed query source from PR #111 is compiled unchanged in both the TD
test module and this external crate (only its Rust visibility is widened).
The new proof reuses its non-first coordinate query rather than introducing a
second local query or another semantic kernel.

The complete Consumer-capable registration is a real Core
`StaticBindingRegistration` containing the existing mock compiler/server and
an inert client. Client execution callbacks panic if invoked. The compiler
produces the existing owned `MockArtifact`; no new compiler interpretation or
protocol execution is introduced. The registration has no TD lifetime or
TD-derived field and is retained separately from the output.

`Draft` owns existing `LogicalInteractionPlan`, `BindingCandidate`,
`BindingArtifactEnvelope<MockArtifact>`, and `BindingArtifactRef` values through
`PlanBuildOutput`, plus owned Form metadata and sealed property lookup rows.
All nested storage is a scalar, owned String/Box, or owned collection. The
closed payload type matters: a generic `PlanBuildOutput<A>` having no explicit
lifetime is insufficient when `A` itself borrows TD or registration data.
The fixture crate forbids unsafe code. Its only ranges address its own plan
collections, never Snapshot node/edge/byte storage.

## Executable witness

The runtime test uses the existing Basic-valid typed corpus with a small
ownership-specific mutation:

- `alpha` is present but has no readable Form;
- `zeta` has an unreadable Form at original index 0 and a defaulted readable
  Form at original index 1, with inherited NoSec, coding, subprotocol, scopes,
  and a cached composite resolved URI;
- `zz_other` has another readable Form, so the sealed aggregate spans multiple
  properties and contains every readable coordinate in the mutated input.

The test runs existing Basic on that input, normalizes it, and destroys the
input Thing before Planning. It then:

1. Executes PR #111's borrowed query through the external facade with zero
   allocation calls.
2. Seals owned plans, candidates, eager artifacts/references, metadata, lookup,
   and a fixture requested-byte footprint while the complete Snapshot is live.
3. Ends every view and compiler-input borrow and statically requires both the
   concrete draft and complete registration to be `'static`.
4. Destroys the real Snapshot. A fixed three-address allocator observer proves
   that its node, edge, and byte arenas are each deallocated exactly once,
   with zero allocations and exactly the reported source byte total.
5. Selects original Form index 1 from retained artifact slot 0, distinguishes
   empty from missing properties, rejects the filtered original index 0 and
   an out-of-range index, selects the later property, verifies owned runtime
   facts/identities/artifact payloads/footprint, and rejects a stale binding
   generation. This whole post-drop interval performs zero allocations.
6. Destroys the separate registration and continues using the output, then
   drops the output and observes release of exactly its reported owned bytes.

The thread-local observer extends the existing Host counting allocator; it
allocates no tracking collection. Successful allocations and reallocations
update requested-byte live truth; failed calls retain the old live truth.
Reallocation changes are observed at the allocator API boundary, excluding
allocator-internal storage/overlap. Sealed collection sizes/capacities and owned
payload lengths independently calculate the fixture footprint, which must
equal both the build interval's net live bytes and the final drop's freed bytes.
The Planning overlap peak adds the continuously live Snapshot request total
to that interval's maximum additional requested bytes. Snapshot release does
not subtract from this recorded peak.

For the fixed witness on a 64-bit Host, the observed Snapshot requests total
64,940 bytes, the sealed owned output requests total 2,272 bytes, and the
Planning overlap peak is 67,548 bytes. These are fixture observations, not
profile ceilings or portable allocation-layout guarantees.

The crate's doctests also include one positive generic source/registration
drop-order contract and four negative contracts: nested Form use after owner
drop, a TD string hidden in an artifact, a borrowed registration callback
hidden in an artifact, and access to the facade's private backend.

## Reproduce

From the repository root:

```sh
cargo test --locked -p clinkz-wot-td --lib external_planning -- --nocapture
cargo test --locked -p validated-thing-planning-handoff-probe
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --no-default-features
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --features serde_json/preserve_order
cargo test --locked -p clinkz-wot-td --lib normalized_snapshot_probe --no-default-features --features td2-preview
cargo check --locked --target thumbv7em-none-eabihf -p validated-thing-planning-handoff-probe
```

Workspace tests run the runtime witness and all five doctests. Mainline CI also
runs the three alternate Snapshot graphs and the real thumb compilation.

## Exact proof boundary

The newly closed subgap is constructive feasibility of ending every borrowed
view and using a complete, concretely owned Planning projection after physical
Snapshot release, with no hidden registration borrow. It extends PR #111's
build-time query evidence and does not repeat its URI equivalence claim.

Item 4 remains incomplete as a whole: the future production `ValidatedThing`
and public View do not exist, and full admitted construction must repeat this
boundary. The facade/backend is a fixture mechanism, not a production API or
representation choice. The synchronous, fixed-corpus `seal` uses ordinary owned
allocations and assertions; it is not the future WP-200 preflight, all-bounds
barrier, charged/resumable aggregate algorithm, deterministic lookup complexity
proof, production PlanFootprint, or rollback evidence.

For item 7, this fixture observes Snapshot/output overlap and physical source
release only. It does not supply the coordinator's matching parent/global
allowance, observe the child-ledger-to-parent release order, prove the complete
resource/work envelope, or exercise a Published record, publication permit,
cancellation boundary, or atomic installation. Prototype destruction retains
the existing arena ledger checks; it adds no source-to-persistent conversion.

Items 1–3, the full capability/downstream/runtime matrix in item 5, complete
progress/rollback in item 6, and independent reaffirmation in item 8 remain
separate obligations. Thumb compilation is not constrained execution or a
physical-target memory/cycle/stack claim. No production source, public target,
resource authority, admission record, or previous completion/gate evidence is
changed.
