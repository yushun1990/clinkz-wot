# Consumer aggregate compiler admission boundary

Status: DISCUSSING

## Question and authority

Can `WP-200-CONSUMER-PROPERTY-READ-AGGREGATE` satisfy its bounded compiler,
Host/static and cleanup contracts using only its three permitted Planning
source paths and the existing complete-registration/compiler SPI?

This is a scoped admission investigation under [ADR-0013](../docs/ADRs/0013-work-package-scoped-implementation-admission.org).
The [Planning specification](../docs/spec/planning.md#first-consumer-property-read-aggregate)
and [unadmitted work-package boundary](../docs/work-packages/WP-200-planning.md#unadmitted-successor-boundary-wp-200-consumer-property-read-aggregate)
remain authoritative. No production change, admission, completion or gate
acceptance is established by this topic or its probe.

The inspected implementation basis is
[`94a8ea2`](https://github.com/yushun1990/clinkz-wot/tree/94a8ea222cc5259f9d6571b6af24b5347497d776).
The following conclusions distinguish a predecessor's accepted claim from the
stronger contract a successor would consume; they do not reopen that predecessor.

## Dependencies and evidence boundary

Both declared exact predecessors are `complete/admitted/current`, with passed
registered evidence: `WP-100-CONSUMER-VALIDATED-THING` and
`WP-200-CONSUMER-PROPERTY-READ-PLANNING`. Broader WP-100 completion, strict JSON
ingestion, WP-400 runtime completion and the Consumer gate are not prerequisites
of this Planning slice. WP-300's completed execution boundary is also not a
substitute for a bounded compiler support contract.

The completed TD tranche supplies the actual opaque proof, complete shared
Basic validation and paid semantic cursor, including rewind without a lifetime
refill. The existing leaf still accepts `&Thing`; it must remain regression
evidence, not become the aggregate's trusted entry. The
[production semantic join](../td/tests/semantic-join/README.md) exercises all
coordinates, empty lookup ranges, later failures with zero starts and concrete
source-independent output. Its fixed slots, static compiler, infallible compiler
target allocation and prepaid fixed destructor are explicit limits. It does
not establish generic compiler admission, Host erasure, variable rollback or
an exact production `PlanFootprint`.

Topic [0076](0076-composable-capabilities-and-runtime-boundaries.md) keeps the
current aggregate unchanged and defers capability/crate/image reconsideration
until real target Consumer feedback. Its broader alternatives do not require
an immediate rewrite. The concrete resource gap below intersects this tranche
and therefore needs its own resolution before admission. Previously migrated
TD and handoff investigations supply narrower historical evidence, not a waiver.

## Concrete admission gaps

### Compiler support must precede callbacks, including Core Host erasure

The aggregate contract requires callback costs to be paid before `bounds`,
cursor/temporary capacity before `start`, actual Layout authorization before
allocation, bounded exactly-once abort and source-free artifact cleanup.
It explicitly says that a complete registration is not a support attestation.

Current production has a smaller contract:

- `BindingCompilerExtension::bounds` returns its declaration only after an
  arbitrary callback. The complete registration captures identity/capabilities
  and runtime declarations; it supplies no reviewed pre-callback compiler-cost,
  allocation or destructor attestation.
- `HostCompilerAdapter::bounds` forwards those bounds unchanged. `start`
  allocates a cursor box; `step` consumes that box, reboxes Pending/Failed
  cursors, or allocates an artifact payload box. These are Core-private
  `Box::new` operations. The bound does not describe the artifact erasure Layout
  or the repeated allocation/release work. No capacity/allocator argument or
  fallible erasure operation is exposed at that boundary.
- The existing concrete `MockCompiler` also creates its target with an
  infallible `Box<str>` conversion. Its accepted static witness expressly stops
  allocation-failure injection before compiler start. That is insufficient for
  the new aggregate's stronger physical-allocation/terminal-ownership claim.

The [external probe](../tools/architecture-fixtures/consumer-compiler-admission/README.md)
invokes these actual production components, without copying compiler or TD
algorithms. On the inspected x86_64 representation, identical static/Host
bounds declare a 25-byte artifact, a 1-byte cursor and zero temporary bytes.
Static completion requests 25 bytes; Host completion requests those 25 bytes
plus a 24-byte, alignment-8 payload box. Host start requests the cursor box,
and a direct zero-credit Pending call releases/reallocates it. All observed
output releases have matching Layouts.

The Pending observation is not an aggregate zero-budget failure: no aggregate
exists, and an aggregate may avoid invoking that callback without its own
prepaid callback credit. It demonstrates erasure work that its support contract
must include. Likewise, the delta does not invalidate the completed leaf's
logical artifact-footprint/parity evidence. It falsifies treating that evidence
or the unchanged returned bound as complete physical Host admission evidence.

The frozen aggregate entry accepts a complete registration and a config checked
from `ResourceLimits`; it has no identified carrier for a checked compiler
support attestation. A limits projection cannot itself establish the behavior
of an erased compiler. Merely sealing an adapter implemented generically for
complete registrations does not close this distinction. A precise source-level
closed compiler/allocator proof could be sufficient, as the specification
allows, but its selected implementations, enforcement and Host representation
costs must be identified before admission. They are not supplied by the fixed
static join.

This intersects `PLAN-COST-003`, `PLAN-ARTIFACT-001`, `ADMIT-MEM-001`,
`CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001` and `CONSTRAINED-OWN-001`.
Correcting Core-owned erasure or its support interface exceeds the aggregate's
currently permitted three Planning source paths. An author cannot repair that
dependency implicitly during aggregate implementation.

### The production feature dependency is omitted from the allowed paths

`ValidatedThing` and its lending types are exported only with TD's explicit
`validated-thing` capability. Neither `planning/Cargo.toml` nor its Core
dependency requests that capability. Planning's standalone no-default normal
feature graph has no such activation. A workspace test or downstream sibling
can activate it through feature unification, hiding the missing dependency.

The aggregate promises useful exports in all three Planning cells and forbids
raw Thing input. Its current path list excludes `planning/Cargo.toml`, so it
cannot establish that normal dependency within its declared scope. The small
correction is to include the manifest in the admission scope and explicitly
request TD's existing capability. No new TD semantics or requirement activation
is needed. Standalone normal-dependency checks must cover no-default, async
without std, and std, rather than relying on workspace feature unification.

There is also an API projection to reconcile in that same contract review:
`docs/api-ownership.csv` still freezes `preflight_consumer_property_read`, while
the consumed build contract delivers preflight through `Build::step`. The review
must specify whether that extra function has a legal bounded ownership shape or
remove its obsolete target row; implementation must not invent a second entry.

## Smallest justified correction and alternatives

Prefer one narrow compiler-support contract correction before admitting the
full aggregate. Identify the supported static and public Host compiler set and
the enforceable connection between its complete registration, pre-callback
costs, actual allocation/erasure Layouts, pending cursor ownership, abort and
artifact destruction. Keep erasure in Core and policy capture in Servient;
Planning consumes the checked support rather than discovering or trusting it
after callbacks. Determine whether a source-proved reserved allocator suffices
or Core needs an admitted fallible/reserved erasure adapter, then record the
exact API and predecessor/path consequences. This is a bounded decision, not
authorization to implement either alternative.

If Core changes are necessary, give that precursor its own ADR-0013 admission
with exact Core ownership and regression evidence before the aggregate depends
on it. Amend the aggregate's manifest path/feature and API projections in the
same authority correction. Do not require completion of a whole package or
runtime merely to expose this bridge.

| Alternative | Assessment |
|---|---|
| Admit now and treat returned bounds or registration validation as sufficient | Reject: the observed Core allocations and the pre-callback obligation remain outside the declared evidence and paths. |
| Limit the first aggregate to a source-proved static compiler | Possible smaller implementation, but needs an explicit tranche split and revised Host/aggregate completion claim. It cannot be called completion of the existing paired boundary. |
| Retain Host/static scope and correct only the compiler support bridge | Preferred: directly addresses the stronger obligation while preserving TD semantics, the all-bounds barrier and public Host ownership. |
| Adopt topic 0076's leaf-crate/generated-image design first | Disproportionate: no evidence here requires a new package, binary image, universal allocator framework or broad runtime rewrite. |

No new resource row, WorkClass, profile value or global gate is justified.
Use existing document/Form, artifact/cursor/temporary/runtime and compile/reclaim
limits; distinguish logical allowances from actual allocation requests.

## Falsifiable closure and later completion

The pre-admission correction is closed only when the exact supported callback
and allocation/cleanup boundary has a constructible checked entry, its API and
ownership projections agree, and a pre-code external fixture can exercise that
dependency through complete registrations in both representations. It must
account for Host cursor/payload erasure, callback/abort costs and allocation
failure or a proven non-failing reservation before callbacks, with ownership
preserved at every failure. Independent exact-head review must accept that
contract and its evidence; passing this observational probe alone cannot do so.

The eventual aggregate admission must retain the already specified completion
key `consumer-property-read-aggregate` and output path
`docs/evidence/WP-200-consumer-property-read-aggregate.toml`. Its implementation
completion still needs variable-sized output and rollback/reclaim, actual
Layout/overlap/peak reconciliation and exact `PlanFootprint`; all bounds before
any start and zero-start later negatives; monotonic TD, Planning, compiler and
cleanup allowances; supported Pending/abort/destructor behavior; exact-coordinate
regression; and external consuming completion usable after input and compiler
registration destruction in all three cells.

Those results that need the aggregate are completion evidence, not reasons to
demand aggregate implementation before admission. WP-400 separately owns paired
parent/global allowances, generation/slot and cancellation races, cleanup
retention, permit/install, leases/drain and final runtime reclaim. Consumer gate,
real Host protocol evidence and strict external ingestion retain their later
boundaries. Neither this investigation nor a successful compiler correction
establishes any of those claims.
