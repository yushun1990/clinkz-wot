# Consumer aggregate semantics and representation admission

Status: MIGRATED

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
It does not admit production changes or claim independent acceptance. The later
construction decision below is migrated into the Binding SPI, Planning and the
registered precursor candidate. Migration of that decision does not admit the
precursor or complete its required production evidence.

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

## Construction decision and migration

The new [downstream construction witness](../tools/architecture-fixtures/consumer-compiler-support/README.md)
adds evidence beyond the earlier allocation observation. Its upstream model owns
one private, immutable target-copy compiler and the support issuer. It creates
real complete Consumer-capable Core registrations; the caller can choose checked
configuration, but cannot supply a callback, destructor, implementation identity,
cost attestation or support constructor. A same-compatibility impostor is rejected.
The actual private capacity/configuration is compared with the complete owner,
not trusted from its identity tuple. The checked carrier owns that entire bundle;
preparation borrows it, and consuming completion ends the loan.

The Host transport model acquires one cursor/output slot before native start,
keeps its address across unpaid retries, paid Pending and Failed, and transfers
it to source-independent output without allocation after a callback. Injected
null acquisition leaves the original ledger and another held owner intact.
Actual typed and model-Host outputs remain usable after input and registration
destruction. Commands, measured Layouts and the distinction from current public
Host erasure belong to the fixture README. In particular, the existing public
Host complete registration is constructed and rejected as unsupported; its
positive production replacement remains completion work, not a result of this
model.

This resolves the issuer question narrowly: the supported first implementation
is a **closed Core-owned resolved-target copy primitive** with immutable checked
configuration. It performs no protocol or TD rule. Core knows its callback and
destruction bodies; erasure records the actual private implementation/adapter
kind, and only complete-registration capture can create the supported owner.
The open native compiler SPI cannot certify itself. Accepting arbitrary binding
compilers would require a separate explicit trusted source-admission boundary,
which is unnecessary for this first constructibility proof. The existing
allocating mock and future Zenoh compilers are not silently admitted.

The [Binding SPI](../docs/spec/binding-spi.md#closed-consumer-compiler-support)
owns the exact additive API and one held-slot Host mechanism. The
[precursor candidate](../docs/work-packages/WP-200-consumer-compiler-support-admission.md)
and `docs/work-packages/index.toml` own its three Core production paths, complete
predecessors, all three feature cells, prechecks, exclusions and production
completion key. Planning projects that checked complete owner as its existing
registration parameter. There is no installable compiler half, public proof
factory, portable compiler-trait change or generic allocator framework.

A separately held allocation cannot authorize today's boxes, and fallible
boxing after Complete has no place to retain the consumed result. Acquiring the
one fixed cursor/output slot before native start preserves the existing failure
shape. Static storage stays inline and pays its own physical costs. The Core
helper's narrow capacity and lack of foreign callbacks are intentional scope,
not evidence for arbitrary native compilation or whole-device fit.

Startup and aggregate work are distinct: ordinary complete construction already
calls compatibility. Those startup costs are separately owned; support capture
itself invokes no compiler callback, and all later aggregate callbacks are
prepaid. This removes a literal timing contradiction without weakening aggregate
eligibility or all-bounds-before-start.

This topic's decision is migrated. The precursor is only planned/candidate and
still needs independent exact-head admission review, followed by implementation
and positive evidence through the real public Host/static Core surface. The
model cannot substitute for that completion. Independent review must challenge
whether Core's closed helper earns its API/ownership cost, not merely accept the
valid allocation counterexamples as proof of the proposed remedy.

The aggregate remains unadmitted until the precursor is complete/current. Its
existing TD enumeration, all-bounds barrier, variable output, exact footprint,
rollback and paired representation completion claims are preserved. WP-400
separately owns parent/global capacity, publication and terminal retention.
Topic 0076 still owns product/deployment alternatives; real protocols, genuine
client-only construction, generated images, Pico progress and hardware fit are
outside this decision. No existing completion, gate, milestone, resource value
or roadmap status is changed or independently accepted here.
