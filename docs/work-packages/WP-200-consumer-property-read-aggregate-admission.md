# Historical / restricted Property Read record

Status: historical v5 authority; **no v6 implementation admission or architecture
acceptance**. The original record and exact prose are preserved below and at
[the predecessor revision](https://github.com/yushun1990/clinkz-wot/blob/ed09f059e383b7a1b9d22902ee29aa218b4471a6/docs/work-packages/WP-200-consumer-property-read-aggregate-admission.md).
[ADR-0025](../ADRs/0025-semantic-engine-and-binding-runtimes.org) supersedes its
general API, gate ordering and lifecycle projections. The [index](index.toml)
records individual tranche impact dispositions. Tests of retained Property Read
code remain useful at that narrow scope; neither a passed status nor this
record constrains the successor SPI. Exact schema/resource encodings still used
by retained adapters keep their original meaning. Any new source scope must use
the successor requirements and applicable review, not inherit this admission.

## Preserved predecessor record (non-authoritative for the general architecture)

# WP-200 Consumer Property Read aggregate admission

This is the ADR-0013 implementation admission record for v5.1.
[The index](index.toml) owns its state. The
[Planning specification](../spec/planning.md#first-consumer-property-read-aggregate)
owns behavior and the public consumed build/rollback boundary. This record owns
the exact implementation and evidence envelope.

[Independent admission review of PR #148](https://github.com/yushun1990/clinkz-wot/pull/148#issuecomment-6108374621)
accepted base `dd03a0a88b3762d9e934f931ccc678cec2e677ad` to head
`cc92ba381411c9266c3c49310e2f68d4ba973fbd` with no blocking findings.
That acceptance covers the admission basis only. The separate
[independent production review of PR #150](https://github.com/yushun1990/clinkz-wot/pull/150#issuecomment-6118559011)
accepted the complete unpublished Planning aggregate and its execution evidence
at head `362e56295d408548bfe6eccf18a02051934da814`.
The [registered completion evidence](../evidence/WP-200-consumer-property-read-aggregate.toml)
retains the implementation reference, nine-obligation results and separate
static/public Host measurements. Completion covers this tranche only; broader
WP-200, WP-400 admission/completion, Consumer gate registration/acceptance and
milestones remain separate.

## Decision and challenged assumptions

The smallest useful implementation unit is the complete unpublished,
one-registration/all-readable Property Read aggregate, paired across typed
static and public Host preparation. Preflight, materialization, the all-bounds
barrier, sequential compilation, output sealing and variable rollback share
one failure/ownership boundary. Splitting them into source tranches would not
supply independently usable output or remove a different dependency. No
Servient publication, call execution or broad compiler framework belongs here.

The completed TD and Core precursors do not prove this aggregate. TD owns
complete Basic and paid effective meaning; Core owns eligibility of its closed
resolved-target primitive and representation costs. Planning must compose them
and prove its own variable storage, lifetime work and terminal owners. A checked
complete registration is necessary; open callbacks, equal compatibility values
and limits-only configuration remain insufficient.

The [bounded investigation](../../workspace/0078-consumer-aggregate-capacity-handoff.md)
found a real contract mismatch: the old Planning text asked an opaque,
caller-constructible child `AdmissionLedger` to prove transaction/policy origin.
It also described a borrowed reservation as though it were an owned capacity
account. The selected correction puts provenance checks at the existing
Servient coordinator, retains local per-Layout enforcement in Planning, and
uses Foundation's existing commit semantics for the fixed cleanup destination.
The frozen constructor now returns the original guard on rejection. This is
an explicit authority correction, not evidence that global pairing exists.

Alternatives considered:

| Alternative | Disposition |
| --- | --- |
| Add Foundation identity/policy getters or mint a new universal capacity certificate | An equal tuple/ceiling does not prove a held parent allowance. The coordinator already owns transaction association; no Foundation API is needed for this local preparation boundary. |
| Keep a `ResourceReservation` inside the lifetime-free cleanup slot | It borrows its account. Commit the guard and retain the actual account/parent at the coordinator instead. |
| Streaming compile, a singleton, or a successful subset | Violates all-readable output and zero starts after a later materialization/bounds failure. |
| Static-only admission | Silently narrows the accepted paired capability despite completed public Host support. Physical evidence remains separate. |
| Contracts extraction, generated images, or runtime/crate redesign | Review 07 and topic 0076 identify credible later alternatives, but no concrete defect here requires them. Keep one TD meaning and one Planning program. |
| Another fixed compiler implementation or copied TD rules | The production closed primitive and paid TD cursor already supply those responsibilities. |

## Dependency and implementation envelope

Exact predecessors, all complete/current with passed registered evidence:

- `WP-100-CONSUMER-VALIDATED-THING` —
  [borrowed TD evidence](../evidence/WP-100-consumer-borrowed-td-admission.toml);
- `WP-200-CONSUMER-PROPERTY-READ-PLANNING` —
  [exact-coordinate regression](../evidence/WP-200-consumer-property-read-planning-selection.toml);
- `WP-200-CONSUMER-COMPILER-SUPPORT` —
  [closed compiler evidence](../evidence/WP-200-consumer-compiler-support.toml),
  including its completed WP-300 complete-registration dependency.

The package dependency is complete WP-000. Broader WP-100/200/300 completion is
not inferred. WP-400 is a consumer of this output, not an implemented dependency
silently assumed here. Its coordinator obligations are callable preconditions
of the advanced Planning handoff and require separate executable integration.

Permitted production paths are exactly:

- `planning/Cargo.toml`, only to request TD's existing `validated-thing`
  capability in normal dependencies;
- `planning/src/consumer_property_read.rs`;
- `planning/src/property_read.rs`, only private preparation sharing that
  preserves the completed leaf's public behavior; and
- `planning/src/lib.rs`, for the registered public projection.

No production changes were made by the admission itself. Tests may use
`planning/tests/consumer_property_read_aggregate.rs`, its focused support files,
and the existing external admission fixture. Core, Foundation, TD, Servient,
protocol and legacy production changes are outside this envelope. A concrete
need outside it returns to impact review; it is not an implicit extension.

Applicable requirements, API items, state projection and source paths are
registered in `index.toml`. There are no old-API removals, new requirements,
resource rows, WorkClasses, profile values or performance workloads. Active
v5.1 has no aggregate performance requirement requiring a new benchmark; finite
work/resource behavior still requires executable conformance. No broad
throughput, latency, device-fit or protocol-neutrality claim is made.

## Pre-implementation evidence

The [historical composition discriminator](https://github.com/yushun1990/clinkz-wot/blob/5091685/tools/architecture-fixtures/consumer-aggregate-admission/README.md)
normally depends on production TD/Core/Foundation and requests `validated-thing`
explicitly. It reuses only inert execution halves from the compiler fixture;
support comes from real Core complete-owner capture. It runs two paid TD passes,
materializes two real Core coordinates, calls all real bounds before any start,
then uses static/public Host concrete artifacts after actual source/registration
destruction. A later actual over-capacity bound fails with zero starts. Fixed
slot commit/rejection and consuming/abandoned input-owner witnesses establish
that the resource and lifetime shapes compose without a self-borrow or retained
reservation loan. Compiler work negatives include foreign classes, zero,
over-step and checked-product overflow.

These are fixed construction discriminators, not the variable aggregate, a
prototype global governor, exact aggregate PlanFootprint, or publication proof.
They use ordinary test-side owned copies and do not claim fallible aggregate
allocation handling or full phase work enforcement. Prior TD/Core evidence
retains its original narrower claim and must not be relabeled as aggregate
completion. At admission, product Planning's normal dependency did not yet
enable the TD capability. The allowed manifest change and isolated normal-graph
checks are now covered by the registered production completion evidence,
without relying on this fixture's direct TD dependency.

Run the registered commands with Rust 1.95.0. They were executable before target
code existed: authority validation, locked workspace tests, the fixture in
std/no-default/async, the existing public compiler fixture in those cells, the
production TD semantic join in those cells, and current Planning feature checks.
The admission fixture's rustdoc negatives cover raw Thing input, a raw compiler
as eligible registration, and premature source/registration destruction.

No unresolved topic intersects the selected scope. Topic 0076 leaves broader
deployment alternatives discussing and explicitly defers them unless a concrete
conflict appears. Topic 0078's correction is migrated into Planning, Foundation's
accounting explanation and the existing Consumer state projection. Existing
Producer evidence is disjoint: no Producer source, lifecycle, reservation or
gate contract changes. No Consumer architecture gate is registered here.

## Required production completion

Stable key: `consumer-property-read-aggregate`. Result location:
`docs/evidence/WP-200-consumer-property-read-aggregate.toml`. The result must
name the exact implementation revision, feature cells, commands, measured
Layouts/accounts and outcomes. Passing execution is a candidate for independent
acceptance; it does not set gate, milestone or tranche completion status.

Completion must demonstrate the following through the actual public Planning
entry, not just the discriminator:

1. Checked config and sealed adapters accept only production checked complete
   Core owners. Unsupported compiler/configuration/role/cell rejects before
   aggregate callbacks. Missing ID, non-NoSec/multiple/empty effective security,
   absent config fields and arithmetic failures reject at their declared phases.
   Explicit empty ops/security are preserved by TD, never repaired by Planning.
2. Every property has one deterministic lookup range, including empty ranges and
   an entirely empty readable aggregate. Use at least two readable properties
   separated by an empty property, multiple Forms with non-first original indices,
   UTF-8 names/scopes and a trailing empty range. Lookup distinguishes missing
   from present-empty and strict original-index mismatch. It visits only the
   matching sealed range and uses no TD, registration scan or support probe.
3. Preflight is emitted once from the consumed build, and waits without replay.
   Both semantic passes retain the same non-resettable TD allowance. Ready
   retries pay no prefix sizing/URI/security replay and acknowledge no partial
   copy. Materialization completes before every bounds call, each exactly once;
   all succeed before the first start. Every materialization/bounds negative,
   including a last-coordinate failure, has zero starts and complete rollback.
4. Compiler progress is sequential and retains its coordinate remainder through
   movement, zero/short budgets and fresh caller budgets. Validate zero,
   non-BindingPolls, over-step and checked-product overflow with test-side bound
   injection where the closed compiler cannot emit them. Production eligibility
   is not broadened to manufacture those cases. Prepay bounds/start and fixed
   abort/allocation/destruction/release; step's own debit is counted once.
5. Before each phase, implement a checked finite item/byte/cleanup envelope from
   admitted counts, fixed pass count and actual output capacities. For `P`
   property rows and `F` readable Forms, tables use checked `P * row_layout`
   and `F * coordinate_layout` plus their actual capacity/slack; compiler work
   is checked `F * compile_step_max`. Sum every paid text/program occurrence,
   including repeated IDs/property names/defaults, rather than treating input
   bytes as a one-copy output bound. Repeated Ready, Pending, moves and retries
   never refill any phase. One-short/exact/one-spare cases and overflow fail
   without partial multi-class debit. Reclaim obeys existing item/byte step caps.
6. Exact PlanFootprint separates owned strings/programs, table capacity,
   candidates/lookup, fixed owners, compiler cursor/temporary overlap, artifacts
   and reclaim metadata. Source and lazy capacity are zero. Static inline
   payload and Host payload-in-slot each occur once; Host slot padding/metadata
   and outer owners remain distinct. Largest request observes actual checked
   allocations, never the logical output sum. Registration startup is baseline;
   any actual retained delta is explicit. Record simultaneous live/peak/largest
   observations and retained reservation history, separately for both forms.
7. Inject failure at every new aggregate allocation, retain another independent
   live owner, and reject before an unauthorized request. Every failure/cancel
   or abandonment keeps first cause, performs supported exactly-once cursor
   abort, ends TD/config/registration loans, and preserves all variable output
   and account objects in source-free rollback. Drive item/byte cursors to
   terminal child-before-parent release; no variable collection Drop may hide
   cleanup. Exercise occupied-slot entry rejection, guard-preserving constructor
   rejection, cleanup transfer/take, and root-driven terminal release. The
   coordinator's committed slot account stays live throughout.
8. External consuming completion ends all source/scratch/compiler loans before
   exposing the complete concrete draft. Destroy Thing and complete registration,
   then inspect/select/use every concrete artifact and identity. Preserve
   exact-coordinate Consumer and Producer regressions. The draft is neither
   Published nor installable by treating its identity/footprint as a permit.
9. Exercise shared logical outcomes and invariants in both representations, plus
   their separate physical/failure/cleanup paths. Run isolated normal Planning
   graphs in std, no-default and async-no-std, proving `validated-thing` comes
   from Planning without dev/sibling unification. Portable ARM compilation is
   useful compile evidence only. Run locked workspace, authority and all
   registered Producer gate commands.

Servient still owes checked policy/registration capture, transaction and parent
capacity provenance, simultaneous global accounting, slot/generation/cancellation
races, final reconciliation/permit/install, publication, leases/drain and
published-output reclamation. Real Consumer protocol execution follows that
integration. Those are neither waived nor prerequisites masquerading as completed
implementation here.
