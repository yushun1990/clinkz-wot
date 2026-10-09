# Production TD semantic join

This external crate exercises obligation 5 of the [WP-100 borrowed TD
contract](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md)
through production public APIs. It explicitly requests TD's `validated-thing`
capability. Its normal dependencies are production TD, Foundation, Core and the
existing concrete Property Read binding fixture. No TD candidate, source
projection, private validator, copied semantic algorithm or production Planning/
Servient coordinator participates.

`Build::new` accepts only `ValidatedPropertyReadCursor`, obtained by driving
production inspection/Basic to its opaque proof. It cannot accept `&Thing` or
a synchronous-validation assertion. Both passes use the paid cursor; only Done
permits rewind. TD decides readable operations, security-definition meaning,
defaults and relative URI resolution. The witness checks exactly-one-NoSec
eligibility and owns output copies. It never queries raw source fields or
re-resolves a target. The integration tests provision ordinary typed input
outside admission and use independent literal output expectations.

The witness retains eight property/plan/bounds slots and four scopes per row.
Preflight counts complete output and checks a finite byte ceiling. Materialization
copies every lookup (including empty ranges), Core `LogicalInteractionPlan` and
the metadata omitted from that narrow Core value. Every copy predebits its
complete output/item/release work, then uses a checked, fallible exact allocation.
One Ready event stays unacknowledged across insufficient copies. A separate finite
Planning allowance never refills; TD retains its own production lifetime debit.

After the second Done the TD child dies. All actual compiler bounds are collected
before any start. The reused [complete static registration
adapter](../../../tools/architecture-fixtures/validated-thing-planning-handoff/src/registration.rs)
contains no TD algorithm or input; it joins the existing concrete `MockCompiler`,
`ManualMockBinding` and an inert Consumer client through Core's complete
registration API. Protocol execution panics if called. Candidate identity comes
from that registration. Each real compiler output becomes a Core
`BindingArtifactEnvelope<MockArtifact>` with ConsumerCall identity and no Producer
route reservation. Its owned target remains usable after actual input and
registration destruction. The fixture keeps a scalar compiler cursor across a
public suspension and aborts it exactly once on cancellation or abandonment.

The concrete compiler has one target allocation, one BindingPolls step, a scalar
cursor, no temporary block and no artifact callback/source pointer. These are
inspected support facts, checked against each returned bound. The witness pays
for its target copy and reserves its declared bytes before calling `step`.
Its Box allocation is infallible: allocation-failure injection deliberately ends
before compiler starts. This establishes neither a generic compiler contract nor
fallible compiler-allocation handling. The fixed output releases and abort are
prepaid; at most eight rows and four scopes per row die on any exit. There is no
variable collection Drop, output cleanup slot, detached rollback or publication.

The host observer records actual allocations and matching release Layouts. It
starts after caller provisioning, checks completed output bytes against its local
ledger, and requires zero controlled bytes on every terminal path. Inline
Build/Step/Draft and observer storage are external test-harness capacity, not
invented allocator requests or evidence of Servient global accounting. This join
executes on a host against std and no_std dependencies. ARM compilation checks
the portable representation; it is not bare-ARM execution of the join. The separate
[production TD runtime](../runtime/README.md) owns native/booted-ARM allocator and
access evidence.

The tests falsify:

- loss of non-first property ordinal 3 or original Form indices 1/2, incorrect
  inherited/default/explicit semantics, or loss of coding, subprotocol and UTF-8/
  empty scopes;
- missing empty lookup rows, including an entirely empty readable aggregate;
- input/registration retention in the concrete draft (also required `'static`),
  incorrect artifact/plan/registration identity, or protocol execution;
- starts after later materialization failure, later semantic failure or an
  actual later compiler bound exceeding the fixture's artifact ceiling;
- any leaked allocation at every observed fallible inspect/semantic/output
  request, cancellation or abandonment at every public join suspension;
- partial copy/item/release debits on repeated independently calculated
  insufficient Ready credit, compiler progress at zero credit, or duplicate abort.

Three compile-fail examples prohibit raw Thing input and premature source or
registration destruction. Existing production TD tests own forged-proof/config,
scratch-lending, capability and Basic/resource invariants; this fixture does not
replace them or the accepted historical candidate's narrower evidence.

Run from the repository root (CI uses Rust 1.95.0):

```sh
cargo test --locked --manifest-path td/tests/semantic-join/Cargo.toml
cargo test --locked --manifest-path td/tests/semantic-join/Cargo.toml --features order
cargo test --locked --manifest-path td/tests/semantic-join/Cargo.toml --no-default-features
cargo test --locked --manifest-path td/tests/semantic-join/Cargo.toml --no-default-features --features async
cargo check --locked --target thumbv7em-none-eabihf --manifest-path td/tests/semantic-join/Cargo.toml --no-default-features
cargo check --locked --target thumbv7em-none-eabihf --manifest-path td/tests/semantic-join/Cargo.toml --no-default-features --features async
```

The dependency graph is inspectable with `cargo tree --locked --manifest-path
td/tests/semantic-join/Cargo.toml --edges normal --no-default-features`.

## Boundary and remaining proof

A new external fixture gives production TD a real downstream public-API consumer
without modifying the admitted product scope. Retargeting the old discriminator
would depend on its private observations/Ready accessor and obscure its historical
claim. Extending the completed direct-Thing Planning leaf would not establish a
trusted aggregate entry. Implementing a general aggregate/publication transaction
would cross the separately admitted WP-200/WP-400 boundary.

This is candidate integration evidence, not WP-100 completion or gate acceptance.
WP-100 still needs a same-head reconciliation of the full six production
obligations: complete feature/lifetime/field/Basic/structural/resource/work and
failure coverage, the orphaned source-reclassification removal, disjoint evidence
reaffirmation and its independently accepted production completion record.

WP-200 owns the actual aggregate, generic supported compilers and owned Pending
artifacts, pre-admitted/actual costs, variable rollback/reclaim and exact
PlanFootprint. WP-400 owns policy/registration capture, simultaneous parent/global
accounting, slots/generations/cancellation races, cleanup transfer, source-free
consuming completion, final permit/installation and runtime leases/reclamation.
Neither package's completion follows from this fixed witness. Strict JSON/date
ingestion, real protocol Consumer execution and target hardware characterization
retain their separate owners. No architecture, profile value, admission or gate
status changes here.
