# Consumer aggregate semantics and representation admission

Status: DISCUSSING

## Question and authority

Which parts of `WP-200-CONSUMER-PROPERTY-READ-AGGREGATE` must be shared, and
which compiler/resource obligations belong to a selected preparation
representation? Is correcting a universal compiler/allocator model actually
necessary to admit this narrow dynamic Consumer capability?

This reevaluation inspects fetched master
[`d1c100b`](https://github.com/yushun1990/clinkz-wot/tree/d1c100b2905cd2302e8debfaf84e0b687f391c7e).
[PR #141](https://github.com/yushun1990/clinkz-wot/pull/141) preserves the original
admission investigation and allocation reproduction against `94a8ea2`; its
observations remain valid. Its recommendation to repair a compiler-support
bridge is refined here, rather than treated as a predetermined design.
[PR #134](https://github.com/yushun1990/clinkz-wot/pull/134) is an independent,
non-normative product/runtime assessment at an earlier revision, not admission
for its recommended deployment changes. Topic
[0076](0076-composable-capabilities-and-runtime-boundaries.md) retains those
broader alternatives and their unknowns.

The [Planning specification](../docs/spec/planning.md#first-consumer-property-read-aggregate)
owns the corrected shared/representation contract. The
[unadmitted work-package boundary](../docs/work-packages/WP-200-planning.md#unadmitted-successor-boundary-wp-200-consumer-property-read-aggregate)
owns source scope and later completion. This topic records investigation,
alternatives and remaining admission questions under
[ADR-0013](../docs/ADRs/0013-work-package-scoped-implementation-admission.org).
It does not admit production changes or claim independent acceptance. The exact
checked support construction remains unresolved, so this topic stays DISCUSSING.

## Conclusion and assumptions challenged

Keep **one TD semantic program and one Planning program**, with supported
compiler/resource mechanisms proved for each preparation representation.
The smallest correction is to distinguish generic complete-registration
forwarding from actual compiler eligibility, compose physical representation
costs separately from binding payload bounds, and reconcile the future
manifest/API scope. Preserve the existing paired aggregate completion claim;
separate its physical evidence rather than silently narrowing it to static only.

The aggregate has an incomplete support boundary, not evidence that its shared
selection algorithm or the macro ownership architecture is wrong. No accepted
owner requires a universal Host/embedded physical resource model. The problematic
coupling is treating an all-cell generic registration adapter/parity obligation
as universal compiler eligibility or physical admission. Several stronger
premises do not survive inspection:

- **One semantic program requires one execution representation.** ADR-0001,
  ADR-0009 and the accepted Servient Consumer lifecycle already distinguish
  typed static ownership from Host erasure. Current Property Read Planning
  dispatches the same algorithm through both compiler projections. Distinct
  output storage, allocator behavior and progress mechanisms are permitted.
- **Application-static means embedded, heapless, or on-device Planning.** The
  static leaf and mock currently allocate. `no_std + alloc` is an admitted
  envelope, not a whole-device fit claim. Preparation location, execution
  platform and representation are independent choices. Build-time preparation
  is credible for fixed firmware, but its image/installation trust boundary is
  neither implemented nor required by this dynamic aggregate.
- **A common resource schema requires one allocator abstraction or physical
  footprint.** Foundation already separates logical allowances, actual Layouts,
  inline slots and simultaneous owners. A static artifact may live inline; Host
  may box the same value. Both owe bounded capacity and terminal ownership,
  with different physical accounting. Neither schema nor semantic parity
  requires identical byte totals, allocation counts or poll counts.
- **Every complete registration is an eligible bounded compiler.** Registration
  validity pairs compiler/execution identity and capabilities. It cannot prove
  pre-callback work, allocator backing, arbitrary `Drop`, or a closed supported
  implementation set. A compatibility constant is not implementation identity.
  Generic forwarding and a limits-only config cannot mint that evidence.
- **Extra Host allocations falsify the binding payload declaration itself.**
  Planning defines `BindingArtifactFootprint` as the binding-authored payload.
  The original mock's 25-byte target declaration remains that declaration;
  Host's additional payload box is a separate representation cost. Treating
  the 25 bytes as complete physical Host admission is false. Inflating the
  shared payload bound with Host erasure bytes would instead charge static
  storage incorrectly and risk double counting.

The all-readable preflight and all-bounds-before-start barrier are deliberately
strong dynamic-admission semantics. They provide zero starts on a later
materialization or bounds negative and prohibit a successful subset. Streaming
compilation could reduce retained build input, but changes that failure promise;
Host allocation overhead does not justify doing so. The current narrow
BindingPolls-only eligibility rule is not a general compiler workload model,
but no evidence here requires broadening it.

## Findings from PR #141, classified

| Finding | Classification and consequence |
|---|---|
| No constructible connection between checked policy, the actual supported compiler and the selected representation before callbacks | Genuine shared admission-contract gap. The frozen build signature and generic registration projection do not establish support. Freeze and exercise the checked carrier before admitting the aggregate. |
| `bounds` itself is an arbitrary callback, so its returned budget cannot prepay it | Shared boundedness obligation; not proof that the portable compiler trait must change. Closed source-reviewed compiler support may supply the cost before invoking this unchanged SPI. Each selected implementation/adapter must prove it. |
| Host forwards bounds unchanged while boxing cursor and payload | Real Host representation cost/eligibility gap. Payload bounds need not change; composed Host admission must include actual erasure Layouts, wrapper storage, allocation behavior and release work. Static support does not inherit this mechanism. |
| Host Pending/Failed consumes and reboxes the cursor, including direct zero-credit Pending | Host adapter mechanism. The probe is not an aggregate zero-budget counterexample because no aggregate exists and it may avoid invoking an unpaid callback. A future supported adapter must honor its public progress contract and account for transport work; prepaying an outer callback does not waive a promised zero-credit invariant of the public erased SPI. |
| The concrete mock allocates its target infallibly | Selected allocating compiler limitation, present in both mock representations. The fixed TD join explicitly excluded compiler allocation failure. It is not a universal static/embedded defect or a reason to add an allocator argument to every compiler. |
| Planning's normal dependency does not enable TD `validated-thing`, while allowed aggregate paths omit the manifest | Genuine shared integration/scope defect. Permit the manifest and require the existing capability explicitly during future implementation; check isolated normal graphs in all three cells. No TD semantic change is needed. |
| A standalone preflight function is frozen in API ownership while the consumed build emits Preflight through `Build::step` | Genuine authority-projection defect. Remove the never-implemented extra target row; retain the one consumed progress/preflight entry. |
| Variable aggregate rollback, exact physical footprint, publication and global accounting are not proved | Existing successor obligations, not newly falsified predecessor claims. The observation does not supply them, and a Host repair alone would not complete them. |

This distinguishes a missing shared *admission condition* from missing
representation-specific *evidence and mechanisms*. Keeping the ownership and
boundedness requirements is sound; requiring the same mechanism everywhere
would be the architectural error.

## Reproducible discriminators and their limits

The [compiler observation](../tools/architecture-fixtures/consumer-compiler-admission/README.md)
owns commands, assertions and measured Layouts. It now adds a fixture-only
fixed-capacity compiler, copying an already resolved target into an owned array,
through the production static component and Core Host erasure. It duplicates
no TD or Planning rule and implements no new production adapter.

On Rust 1.95.0/x86_64, the original mock still requests 25 target bytes on static
completion and `(25, 1)` plus `(24, 8)` on Host completion. The same inline
compiler requests no allocator work in observed static callbacks/output drop;
Host completion boxes its `(72, 8)` payload. Both preserve concrete target and
envelope identity after actual source/compiler destruction, and each aborted
cursor reaches one fixed abort. Inline capacity still occupies owner storage;
no allocator request does not mean zero physical cost. The original two Host
requests are not one 49-byte contiguous allocation. No simultaneous peak or
allocator metadata measurement follows from these request logs.

The existing [topic 0076 fixture](../tools/architecture-fixtures/product-runtime-boundaries/README.md)
runs the production leaf planner at build time, compares frozen selection and
response predicates, and checks its generated subset with Foundation-only normal
dependencies on Cortex-M0. Reproduction at this baseline retains the 15 selection
cases, eight malformed-output rejections and 3,456 response comparisons. This
supports separating preparation from execution dependencies without duplicating
rule bodies. It proves no real binding-artifact image, installed authority,
complete runtime, hardware fit, or allocation-free aggregate. No new platform
commitment or generated-image production entry is inferred.

The [production TD semantic join](../td/tests/semantic-join/README.md) remains
stronger evidence for actual paid TD lending, all-coordinate preparation,
empty ranges, later negatives with zero starts and source-independent concrete
artifacts. It remains a fixed static witness with infallible compiler allocation,
fixed prepaid destruction and no exact production PlanFootprint/global accounting.
Neither representation probe substitutes for that trusted TD boundary.

Source owners inspected include [Core erasure](../core/src/binding_compiler.rs),
[shared leaf Planning](../planning/src/property_read.rs),
[complete registration](../core/src/binding.rs), and the
[existing concrete compiler](../tools/architecture-fixtures/property-read-binding/src/lib.rs).
The component probes establish no complete-registration support attestation,
allocation-failure safety, variable cleanup or protocol execution.

## Alternatives and smallest correction

| Alternative | Strongest case and disposition |
|---|---|
| Leave all contracts unchanged and finish the aggregate first | Preserves momentum and the existing semantic architecture. Rejected as immediate admission: the unchecked support boundary and out-of-scope manifest cannot be repaired implicitly in three Planning source files. |
| Make returned bounds include all Host costs and impose one allocator-aware compiler SPI | Could centralize enforcement for arbitrary allocating extensions. Reject as the minimum: it conflates binding payload with representation/storage costs, burdens inline compilers, and still cannot bound arbitrary callbacks/destructors merely by adding a trait. |
| One shared planner with checked support and accounting for each selected compiler/representation | Selected. Preserves semantic and lifecycle guarantees; supports an inline source proof and a distinct allocating Host adapter/proof. Retains Core erasure ownership and Servient policy/parent authority, without a semantic fork. |
| Static-only or Host-only first aggregate tranche | Could shorten one implementation path. Viable only through explicit scope/dependency/completion migration; it does not finish the current paired boundary. Evidence here identifies different mechanisms, not a need to split that claim. |
| Separate Runtime crates or the leaf Contracts extraction first | May eventually improve product dependencies; no causal connection to paying the current compiler callbacks. Keep in 0076 until realistic product evidence justifies migration cost. |
| Build-time images replace dynamic aggregate preparation | Plausible for fixed firmware, inadequate for unknown Host TDs. Requires concrete artifact lowering, target storage, compatibility, fresh runtime identity and installation authority. Larger and independent of this correction. |
| Relax all-bounds, ownership or bounded cleanup to match today's compiler | Reduces immediate implementation work by weakening accepted guarantees. Reject: no demonstrated product trade-off justifies the weaker transaction or resource truth. |

The selected architectural correction is now projected only into Planning's
contract, its unadmitted work-package scope and the stale API row. Foundation,
TD, ADR-0021, binding SPI and Servient already permit distinct physical mechanisms
while owning the needed invariants; they need no parallel contract rewrite.
Topic 0076 links this earlier concrete escalation without adopting its broad
candidate. The historical review and accepted evidence are left at their own
revision/claim boundaries.

A supported closed static compiler may use a fixed inline capacity/destructor
proof with the existing Core trait. A selected allocating compiler must provide
fallible ownership-preserving allocation or proven held backing storage. Current
Host erasure supplies neither an allocation authorization seam nor that backing
proof. A local, additive Core-owned supported Host path is preferable to rewriting
all compilers if reservation safety cannot be established for the current path.
Do not make a logically reserved allowance stand in for an allocator reservation,
rely on incidental System allocator success, or intercept arbitrary process
allocations as a hidden common policy. Core-private costs cannot be fixed by
post-hoc Planning footprint checks.

## Precise next admission boundary

The next boundary is **checked compiler support for this dynamic aggregate,
including the selected static and public Host preparation representations**,
not broad WP-200 completion or a new runtime architecture. Before registering
aggregate implementation:

1. Freeze a constructible checked carrier and its ownership/API projections.
   It must connect the actual reviewed compiler implementation, complete
   registration identity, preparation representation/configuration and costs.
   Specify who constructs it, who retains it, how Servient captures eligibility,
   and how Planning consumes it without concrete protocol dependencies or an
   independent installable compiler half. A user-supplied flag/identity tuple,
   limits-only config or arbitrary third-party attestation cannot be authority.
2. Identify the supported closed implementation set and each callback/cleanup
   primitive, including pre-bounds cost and any compatibility callback used
   during the build. Prove cursor/input loans, Pending/Failed transport,
   exactly-once abort and post-registration artifact destruction. An inline
   compiler needs fixed storage evidence; allocating compiler/Host paths need
   actual Layout/overlap authorization and ownership-preserving failure, or a
   source-proved reservation that holds the physical backing through use.
3. Exercise that entry in a downstream pre-aggregate fixture through **complete
   registrations**, not only these compiler components. Reject absent, mismatched
   or unsupported support before callbacks. Check both physical representations,
   negative allocation/reservation cases and retained owners; do not demand the
   full aggregate's completion evidence before admitting its precursor.
4. If the Host path or support exposure changes Core, admit only that exact
   Core-owned precursor under ADR-0013, with exact paths/API/dependencies and
   regression evidence for the completed compiler/registration/Producer leaves.
   Complete it before the aggregate consumes the changed contract. Keep the
   portable compiler trait unless a concrete counterexample requires changing
   it. The remaining choice of backing proof versus additive fallible/reserved
   Host construction must be resolved in that admission design, not guessed
   inside aggregate implementation.

This PR does not register either precursor or aggregate. Independent exact-head
review must assess the authority correction; author validation is not independent
acceptance. Subsequent formal admission must reconcile the frozen support API
with all affected owners and exact evidence before production changes begin.

The paired aggregate then retains its existing completion key/path and all
variable output, all-bounds, monotonic work, exact-coordinate regression, consuming
completion, exact PlanFootprint and bounded rollback obligations. Both declared
exact predecessors retain their narrower accepted evidence; no broad package
completion is required. WP-400 separately owes parent/global pairing, slots,
cancellation/cleanup retention, permit/install, leases/drain and final reclaim.
The Consumer gate and real Host protocol path remain later claims.

Unresolved risks are the exact non-forgeable support construction across erased
registration, physical backing/failure guarantees, variable artifact destructor
costs, and complete aggregate/global composition. Reviewed support concerns
trusted Cargo-linked implementations; it is not a sandbox for arbitrary native
extensions. Generated images, genuine
client-only construction, real Pico ownership/progress, constrained hardware fit
and protocol-shape neutrality remain 0076 or their own successors. No resource
row, WorkClass, profile value, package dependency, roadmap ordering, milestone or
gate status change is justified by this reevaluation.
