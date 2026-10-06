# 0074 Consumer TD Admission Architecture Proposal and Impact Review

Status: DECIDED — recommended direction; independent architecture review pending

Kind: independent architecture decision proposal and pre-migration impact review

Review baseline: latest fetched `master` on 2026-10-05,
`2c8b41124c12b1fb40261a9861bf6f3d6e1aab57` (PR #126 integrated).

This document selects a target for review, **not implementation authority**.
The current WP-100 admission record, ADRs, specifications, resource schema,
state machines, and work-package states remain binding until a separately
reviewed migration changes them. No production Rust, authoritative contract,
admission/completion state, evidence manifest, or gate status changes in this
proposal. The full review is intentionally retained here as decision reasoning;
the PR is the independent review boundary, not architecture acceptance.

## 1. Recommendation and decision boundary

Use an already constructed, immutably borrowed Rust `Thing` as the core v1
Consumer input. TD performs bounded structural inspection and complete Basic
validation, then lends an unforgeable validated semantic capability to Planning.
Keep `ValidatedThing` as that conceptual proof boundary; it need not own a
complete normalized TD. Compose TD validation, Planning preflight, reservations,
materialization, all compiler bounds, compilation, and publication inside one
Servient-owned admission transaction.

Materialize only derived TD facts that require storage, plus the complete owned
Plan/Artifact/lookup output. Account actual controlled allocation requests and
capacities throughout the transaction. Remove mandatory whole-TD arena copying,
exact-length TD sealing, and post-copy fieldwise equivalence from this path.
Replace the independently exposed `ValidatedThingAdmissionConfig` with checked
operation-specific policy access inherited from the Consumer transaction.
Configuration validation remains required.

Separate JSON ingestion from core Consumer admission. Ordinary serde and typed
authoring keep their existing responsibilities. A future bounded external TD
ingestion capability may use a flat arena representation and ADR-0020 literal
JSON semantics, feeding the same TD semantic authority and validated Planning
interface. It need not construct a Rust `Thing` or match the typed path's physical
storage. Do not make this capability a prerequisite for the first v1 Consumer
Property Read integration.

The storage change preserves the active semantic, work, cleanup, Plan ownership,
and publication guarantees. The JSON disposition is a **deliberate capability
scope change**: it defers the currently specified absolute engine-owned
JSON-decode-through-build guarantee. It must be approved in the architecture
review and migrated explicitly; moving an unbounded parser into an adapter does
not supply that guarantee. If a current deployment requires bounded runtime
ingestion of untrusted JSON, retain or separately admit that capability instead
of claiming the typed-only path satisfies it.

This recommendation is alternatives 2 and 3 in combination, with alternative 4
as the extension boundary. The irreducible objects are a trusted TD semantic
boundary, admitted owned runtime material, and the atomic transaction. A second
complete TD representation is not irreducible.

## 2. Evidence and authority used

References below are repository-relative at the pinned baseline. Requirement
identities are classified by the active authority manifest, not merely by their
presence in `requirements.csv`. Historical topics and PRs explain dependencies;
tests establish bounded observations, not design authority.

| Ref | Evidence owner and inspected boundary |
| --- | --- |
| R1 | [System goals][r1], [primary flows][r1-flow], [module boundaries][r1-mod], and [Servient lifecycle][r1-runtime]: WoT compatibility, profiles, semantic boundaries, publication, and runtime retention. |
| R2 | [Active authority manifest][r2] and [specification index][r2-index]: 65 active identities; historical TD memory/performance identities and deferred Directory/lazy/cache/index/codec identities are separately classified. |
| R3 | [Foundation specification][r3]: `API-RESOURCE-001`, `RES-LIMIT-001..003`, `ADMIT-MEM-001`, constrained storage/work; current admission-specific refinements. |
| R4 | [Runtime safety][r4]: `DOC-RUNTIME-001`, `API-SECURITY-001`, `ADMIT-TXN-001`, progress, ownership, and current normalized-input refinements. |
| R5 | [Planning specification][r5]: active requirements, first Consumer aggregate, compiler contract, owned output, resource accounting, and publication. Its marked deferred clauses are not activated by this review. |
| R6 | [WP-100 admission record][r6]: frozen inputs/API, three retained/four temporary sites, strict decoding, footprint/view, progress/rollback, and eight readmission items. |
| R7 | [WP-100][r7-100], [WP-200][r7-200], [WP-400][r7-400], and [tranche index][r7-index]: exact completed versus candidate/future boundaries. |
| R8 | [State machines][r8]: `validated-thing-normalization`, `compiled-plan-set`, and `consumer-plan-publication` transaction. |
| R9 | [Resource schema][r9], [Foundation ledger][r9-ledger], and [work budget][r9-work]: actual accounts, limits, reservations, and linear work primitives. |
| R10 | [ADR-0008][r10-plan], [ADR-0015][r10-policy], [ADR-0019][r10-consumer], and [ADR-0020][r10-json]: plan lifecycle, borrowed policy, Consumer entry, and strict value semantics. |
| R11 | [0063][r11-handoff], [0064][r11-serde], and [0065][r11-btree]: retained original Thing, opaque-container counterexamples, normalized representation choice, and former published retention. |
| R12 | [0072 build lifetime][r12] and [0071 resource/target distinction][r12-target]: removal of published Snapshot retention; profile capacity versus generic guarantees and target characterization. |
| R13 | [Typed Thing][r13-thing], [public validation][r13-validation], [defaults][r13-defaults], [URI types/resolution][r13-uri], [Context][r13-context], and [TD manifest][r13-cargo]: actual public models, synchronous validation, dependency features, and private storage. |
| R14 | [Planning input/output][r14-input], [Property Read compiler][r14-leaf], [Core plans][r14-plan], [compiler/artifact SPI][r14-artifact], [legacy consume][r14-consume], and [legacy ConsumedThing][r14-legacy]: implementation reachability and ownership. |
| R15 | [Shared semantic fixture][r15], [Basic kernel][r15-basic], [typed adapter][r15-typed], [schema typed adapter][r15-schema], and [fixture source generator][r15-build]: storage-neutral rule convergence and its limits. |
| R16 | [Arena fixture][r16], [literal construction][r16-values], [typed canonical constructor][r16-typed], and [movable result-Basic driver][r16-owning]: checked storage, work, and ownership composition. |
| R17 | [Owned Planning handoff][r17], [external Planning consumer][r17-plan], and [TD handoff test][r17-test]: semantic-view-only consumption and use after source/registration destruction. |
| R18 | [0073 decode investigation][r18], [Number amendment][r18-number], and [RFC3339 fixture][r18-date]: wire differences and bounded atomic/decode obligations. |
| R19 | [WP-500][r19] and [roadmap][r19-plan]: later Directory domain entry, staged Consumer integration, and absence of an admitted universal ingestion prerequisite. |

### Actual implementation, distinct from the frozen target

At this baseline production TD has no `ValidatedThing`, `validated.rs`, or
`validated-thing` Cargo feature. `Thing` owns ordinary maps/vectors/strings;
public Basic validation is synchronous and can allocate diagnostic/context
strings. The production Context sequence remains private. PR #70 authorized a
read-only inspection seam; the present fixture generator supplies a seam in its
build-directory candidate. That is not a production accessor on master.
[R13, R15]

Production `PlanBuildInput` still borrows `&Thing`. The completed
`PropertyReadPlanCompiler::consumer_call` leaf uses TD operation-default and URI helpers
and emits owned logical data and artifacts for an exact coordinate. Neither the
future all-readable aggregate nor its `PlanFootprint` and publication transaction
is implemented. Existing async `Servient::consume(Thing)` constructs legacy
`ConsumedThing`, which retains `Arc<Thing>` and resolves interaction data at
runtime. That legacy path is migration input, not proof of the target flow.
[R7, R14]

PRs [#124](https://github.com/yushun1990/clinkz-wot/pull/124) and
[#126](https://github.com/yushun1990/clinkz-wot/pull/126), inspected against their
actual merged diffs, add non-production canonical typed-Thing construction and
movable post-construction Basic composition. #126 includes the paid Basic patch
from #125 that had not entered master through #124's earlier squash. These
proofs are meaningful: the current representation has not been shown impossible.
They do not implement the complete strict Thing decoder, full owning entry,
configuration/support envelope, integrated URI/date queries, parent/global
pairing, complete Plan overlap, or publication. Their source and README state
those remaining boundaries. No eight-item completion is inferred. [R15, R16]

## 3. Reconstructed non-negotiable requirements

The product consumes a TD to execute WoT operations through selected bindings.
It needs stable runtime execution facts, not a lossless TD resident in every
handle. The input-to-runtime flow is:

```text
TD input with an explicit processing/resource boundary
  -> TD-owned complete validation and effective semantic access
  -> Planning over captured immutable policy/registration identities
  -> owned logical plans and bounded binding compiler inputs
  -> all-coordinate compiler bounds barrier
  -> immutable eager artifacts and complete owned lookup
  -> Servient reconciliation and final publication checks
  -> one complete published generation
  -> selection and name-free requests from plans under leases
  -> binding-owned I/O, Core response validation, terminal cleanup
```

| Guarantee | Requirement/evidence and consequence |
| --- | --- |
| One TD semantic authority | TD owns Basic, default operations, effective security/reference interpretation, and normalized-path URI resolution. Planning compiles URI templates, chooses coordinates/candidates, and applies the slice's NoSec eligibility. Bindings receive resolved selected inputs. Moving storage or transaction phases must not copy these rules. [R1, R4–R6, R10, R15] |
| Complete validation before trusted Planning | The WP-100 contract requires complete Basic, including unrelated affordances/schemas/security definitions, rather than validating only selected readable Forms. Resource/structural rejection is distinct from Basic invalidity. Basic does not require an ID; Consumer preflight does, before runtime reservation or compiler work. Do not add Profile/Full requirements or new combo-cycle restrictions. [R6, R7, R13, R15] |
| Bounded externally influenced processing | `RES-LIMIT-001..003`, `PLAN-COST-003`, and `CONSTRAINED-WORK-001` require finite policy before processing, charging before work/capacity visibility, structured rejection, and no silent truncation. Count/depth/byte limits apply even to opaque supplied extension values where they are resource inputs, without inventing semantic predicates. [R3, R5, R6, R9] |
| Resumability and cancellation | Manual progress makes no hidden progress at zero budget. Per-step replenishment cannot reset lifetime work. Bounded atomic operations are allowed only after a complete debit over an explicit admitted input envelope. Cancellation is observed before work/callbacks, at bounded intervals, and before publication. [R3, R4, R8, R10, R18] |
| Exact ownership of charged resources | Account source, temporary, persistent runtime, diagnostics, and cleanup separately; observe live/peak and actual largest request. Parent/global allowances remain paired with local ownership. Caller input history cannot be reconstructed from logical length. Inline slots and exclusive static capacity also cost resources. [R3, R6, R9] |
| Bounded rollback and cleanup | Every failure is unpublished, preserves its first cause, and retains/releases every reservation and partial owner exactly once. No recursive caller graph destruction or variable-sized diagnostic formatting may hide inside bounded admission. Plan/artifact rollback and reclamation retain their own bounded owners. [R3–R6, R8] |
| `no_std + alloc` and semantic profile parity | The portable path cannot require `std`, `Arc`, atomics, an executor, or a global counting allocator. Variable-sized allocations remain permitted when charged. Static and Host physical containers may differ while outcomes/identities/resource meanings agree. Thumb compilation is not target runtime or stack evidence. [R1, R3, R4, R10, R12] |
| Independent immutable Plan/Artifact ownership | `PLAN-COST-001`, `PLAN-SET-001`, `PLAN-ARTIFACT-001`, and `PLAN-REQUEST-001` require shared logical data, bounded immutable artifacts, generation-qualified leases/refs, and owned runtime facts. No TD/input/configuration/registration borrow may hide in an artifact. [R5, R10, R14, R17] |
| Complete first Consumer aggregate | All declared Properties have lookup rows, including empty readable ranges; all effective readable Forms retain original indices and deterministic order. Exactly one complete registration, eager Consumer artifacts, and the slice's exactly-one-NoSec eligibility remain. A failed later coordinate cannot leave a published successful subset. [R5–R7, R11] |
| Full bounds barrier and pure compilation | Reserve conservatively after preflight; materialize all inputs and evaluate every compiler bound before any `start`. Even though compilers are pure, the current zero-start-on-materialization/bounds-failure guarantee remains. No compiler invokes I/O, credentials, handlers, tasks, or external leases. [R5, R7, R10] |
| Atomic Servient publication | Reserve identity, runtime capacity, cleanup, and registry slot; complete all fallible/cancellable checks before a private publication permit. Its final install is non-yielding, allocation-free, callback-free, and non-fallible. Readers see absent or complete; drain closes new leases and retains existing owners to terminal settlement. [R1, R4, R5, R7, R8] |
| Runtime freedom from source retention/reinterpretation | `DOC-RUNTIME-001` and the migrated 0072 boundary forbid Published Snapshot/TD retention. Runtime selection uses owned plans; source retention for a separate product purpose is explicit, independently charged, and not a backdoor planner. [R4, R5, R12, R17] |

The manifest classifies `TD-MEM-001..002`, `DOC-RUNTIME-002`, and several
performance identities as historical inputs. Directory streaming, broad codec
validation, lazy/cache/index behavior, and long-lived interactions require
later entry review. They cannot make a universal strict JSON decoder mandatory
for this Consumer slice merely because historical rows mention them. Conversely,
current *refinements* of active admission requirements do explicitly freeze
strict entry and Snapshot mechanics; they must be deliberately revised rather
than ignored. [R2, R3, R4, R6, R19]

## 4. Requirements versus mechanisms

“Implementation choice” here means removable through reviewed architecture
change, not optional under today's frozen record.

| Current mechanism | Classification and concrete disposition |
| --- | --- |
| Complete Basic proof and immutable validated semantic boundary | Required. Preserve; no raw-Thing Planning shortcut or unchecked proof constructor. |
| Independently materialized `ValidatedThing` owner | Choice. A validated borrow/session capability can prove the same semantic boundary while the caller keeps input immutable. |
| Complete normalized TD copy | Choice needed by the selected owned-Snapshot contract, not by runtime execution. Remove from typed admission; retain full source semantics in the caller's `Thing`. Copy every fact actually needed by output. |
| Three retained arenas | Choice that makes a fully owned generic TD observable and cheaply destructible. A borrowed typed graph has no owned source graph to census/drop. Smaller derived-fact and traversal storage needs its own complete allocation catalog, not a prescribed three arenas. |
| Four temporary allocation categories | Choice of the current construction algorithm. Exhaustive accounting/cleanup is required; the number and identity of categories are not. Do not invent new categories without a concrete need. |
| Exact-length sealing | Choice. Immutability, recorded actual capacity, checked requests, and bounded destruction do not require `capacity == len`. Remove mandatory TD seal; runtime output still freezes semantically. |
| Fieldwise post-normalization equivalence | Required if a full semantic copy is constructed. Unnecessary where a proof refers to the same immutable typed value. Preserve Basic/query regression and output completeness; future alternate source adapters require semantic agreement evidence. |
| `ValidatedThingFootprint` | Choice of source handoff, with necessary accounting contents. Replace Snapshot-specific public footprint with transaction-owned accounting for actual scratch/derived/output owners. Keep live/peak/largest-request and release evidence; never reduce truth to a single “bytes” scalar. |
| `ValidatedThingAdmissionConfig` | Choice of public API and operation-local projection. Validated policy/applicability and implementation-supported limits are required. Reuse a checked Consumer transaction/profile boundary with TD-owned support checks; do not pass unchecked raw limits or validate unrelated `NA` fields. |
| Strict JSON decoder inside WP-100 admission | Current capability commitment, not an identified minimum Consumer execution requirement. Defer core coupling explicitly. Any retained bounded ingestion capability still needs a conforming decoder and its own source/work/cleanup proof. |
| Identical typed/JSON physical representation | Choice. Equal logical values must reach the same TD meaning and Planning behavior. Distinct borrowed/owned source backends are conforming behind TD-controlled access. ADR-0020 already disproves unconditional same-wire equality as a useful universal requirement. |
| Additional-over-baseline typed guarantee | Required honest scope for opaque pre-existing input. Borrowing preserves it; source copying does not improve the caller baseline. |
| Absolute bounded JSON ingestion | Required **when that capability is advertised**. The recommendation defers that capability for core v1; it does not relabel ordinary serde as bounded or claim a whole-process memory bound. |
| Plan/Artifact accounting, all-bounds barrier, publication permit | Required independently of TD storage. Preserve and complete them. They govern the long-lived resources and visibility that make Consumer execution safe. |

## 5. Historical reasons and whether they still apply

| Historical dependency | What was justified then | What remains justified now |
| --- | --- | --- |
| 0063, subsequently projected into the admission contract | A move-only validated input retained the exact original `Thing`; Published kept it and transferred source charges to persistent-document charges. A retained graph census was necessary for that ownership choice. [R11; 0063 “Validated input and retained source”] | Full validation and owned aggregate remain necessary. Published raw-Thing retention was superseded by 0072. The old census requirement is not a reason to own the typed source today. |
| [PR #69](https://github.com/yushun1990/clinkz-wot/pull/69), [#70](https://github.com/yushun1990/clinkz-wot/pull/70), and 0064 | Context hid buffers/capacity; downstream serde features hid Map/Number storage. Accounting exact retained caller storage failed from public content alone. #69 added work classes/ledger transfer; #70 authorized a seam, rather than implementing the TD cursor. [R9, R11, R13] | Counterexamples remain valid. Do not revive private-layout accounting. A core borrowed-input transaction need not own or census those allocations. Context semantic inspection may still need a private TD seam. |
| [PR #74](https://github.com/yushun1990/clinkz-wot/pull/74), 0065, [#76](https://github.com/yushun1990/clinkz-wot/pull/76), and [#78](https://github.com/yushun1990/clinkz-wot/pull/78) | Retained-empty BTree roots falsified a length-based bound on the CI toolchain. A project-controlled normalized graph separated semantics from opaque allocation history. A strict builder supplied an absolute bound for controlled construction. | #74 did not merge. The counterexample proves that logical size cannot bound retained opaque storage; it does **not** prove that all bounded admission must create a full semantic copy. Observable owned storage is still necessary for any engine-owned ingestion output. |
| Three-arena/exact-seal projection in #78 | Exact requests, allocation-free typed queries, fixed release catalog, deterministic accounting, and lossless copied semantics were constructive answers for the selected owned graph. [R6, R11, R16] | These properties are real. Their broader guarantees remain, but a build-only capacity-charged owner can supply them too; a borrowed typed source removes most of the graph-storage problem altogether. |
| 0072, [PR #112](https://github.com/yushun1990/clinkz-wot/pull/112), [#113](https://github.com/yushun1990/clinkz-wot/pull/113) | `DOC-RUNTIME-001`/`ADMIT-MEM-001` corrected retention: owned plans survive source destruction, and Snapshot disappears before atomic install. 0072 preserved the representation and recorded capacity-based storage as a possible later review. [R12, R17] | Runtime retention, document reclassification, and exact-size steady-state source savings no longer justify mandatory Snapshot copying. Build-time boundedness and independent output still do. This proposal reviews the representation question 0072 intentionally left open. |
| 0073 and [PR #117](https://github.com/yushun1990/clinkz-wot/pull/117) | Ordinary serde can reinterpret literal objects/strings. A bounded strict decoder needed an explicit value algebra; ADR-0020 selected literal JSON and shared TD field policy. [R10, R18] | The decoding decision remains sound for that strict capability. Core typed admission receives already typed values and cannot recover their original wire provenance. It need not implement a second wire path now. |
| #119–#126 composition | Controlled literal construction, shared schema fields, canonical schemas/Thing, paid Basic, and owning continuations establish concrete constructibility and expose remaining joins. [R15, R16; actual master history] | This is reusable semantic/resource/ownership evidence. It is neither product demand for every layer nor evidence that a simpler typed path fails. #126's self-borrow problem is specific to owning source arenas and continuations together; external typed borrows avoid that source self-reference. |

There is no finding here that the previous decisions were irrational. They
solved the then-selected retained ownership problem. The later lifetime
correction changed the cost/benefit of that solution without independently
reassessing the entire admission architecture.

## 6. Strongest arguments on both sides

### Retain the current architecture

The current shape offers one fully controlled, immutable, input-independent
source. Its allocation catalog supports exact requested-byte accounting and
constant-catalog destruction of arbitrarily nested semantics. Planning receives
allocation-free cached URI/default/security facts. Both entry paths can release
their caller input before Planning, and a static caller can ingest untrusted
JSON with engine-owned storage charged from the first allocation. One physical
backend limits downstream adapter/evidence combinations. Recent composition
has demonstrated substantial ownership/progress feasibility, rather than merely
asserting it. [R6, R15–R18]

This is the best choice if **bounded runtime external ingestion is required
immediately**, source lifetime must end before a pending Planning build, or
another admitted consumer genuinely needs a complete owned semantic TD. Exact
sealing also improves predictable retained size under tight build peaks after
growth, despite its own overlap. Those are concrete advantages. The review
cannot claim that a borrowed validator is already implemented or that current
snapshot allocation costs always exceed every alternative.

Its weakest link is necessity: the first runtime owns only Plans/Artifacts; the
typed compatibility path already excludes caller allocation history; and no
admitted first-Consumer behavior consumes the copied opaque metadata after
Planning. A fixed allocation catalog is a sufficient means to bounded cleanup,
not evidence that borrowed input itself must be copied.

### Simplify the architecture

A stable `&Thing` already supplies immutable typed meaning during a build.
TD can validate that value directly and expose effective semantics without
duplicating the entire model. The caller retains graph cleanup, while the engine
owns only bounded traversal/derived state and the runtime facts it must keep.
This preserves the existing typed baseline guarantee, removes copy/seal/
equivalence work, and directs the strongest accounting proof to the Plan/
Artifact/record storage that actually survives admission. [R3–R6, R12–R17]

The most concrete size evidence is #113's fixed 64-bit Host witness: 64,940
Snapshot requested bytes, 2,272 owned Planning-output requested bytes, and a
67,548-byte Planning overlap peak. The test was rerun at the review baseline
and reproduced those numbers. This is a within-witness observation, not a
portable ratio, a proposed ceiling, or a measured saving for a borrowed
implementation. It shows that copying irrelevant source content can dominate
the runtime projection in realistic rich typed input. [R17]

The simplification is valid because semantic proof, accounting of *engine-owned*
storage, bounded cleanup, and atomic publication each have a smaller concrete
supplier described below. Its explicit cost is keeping the input loan alive
until Planning ends, proving a new charged typed traversal, and postponing the
currently coupled strict ingestion capability. Difficulty or sunk investment
alone decides none of these trade-offs.

## 7. Comparison of credible alternatives

The following names refer to architectural shapes, not proposed public APIs:

- **A — current Snapshot:** full normalized owned node/edge/byte arenas, exact
  seal, typed and strict JSON entries.
- **A' — capacity Snapshot:** same full normalized semantics, immutable observed
  capacities, no mandatory shrink/copy seal.
- **B — validated typed loan:** TD proof over `&Thing`, with bounded traversal
  storage and only necessary derived semantic facts.
- **C — integrated admission:** no separate public validated owner; TD validation
  capability and Planning/resource phases compose inside one transaction.
- **D — shared semantics, distinct sources:** B for typed input; a future owned
  bounded ingestion backend supplies the same TD-controlled validated facade.

### Authority, work, ownership, and execution

| Effect | A / A' | B | C | D |
| --- | --- | --- | --- | --- |
| Single TD authority; Basic/default/security/URI | Shared TD kernel plus snapshot adapter. A' preserves it. | Same kernel on typed facts; only TD exposes effective results. Avoid an ad hoc borrowed validator. | Keep TD module ownership and an unforgeable completion barrier even if the proof is private transaction state. Moving Basic into Planning would fail. | Shared rule source with source-specific fact adapters; value decoding differs only at the explicitly defined ingestion boundary. |
| Bounded work and resumability | A charges inspect/build/sort/copy/seal/equivalence plus validation. A' removes seal-copy work but retains full construction. | Stateful borrowed iterators, charged byte comparisons, bounded nonrecursive frames, per-step and lifetime work. Existing synchronous typed adapters are not enough. | Can reuse B's cursor while composing phases; validation must complete before trusted Planning and all bounds before compiler starts. | Each source pays its own parse/inspection work; no adapter may hide an atomic full decode behind a common view. |
| Cancellation and rollback | Drop fixed owned catalog, preserve first cause; abort pure compiler once later. | Release only engine scratch/derived/output owners; never drop caller's graph. | One transaction fixes cause and owns reservations across phases; no partial plan publication. | Ingestion owner must also roll back its controlled source; typed loan has no such source owner. |
| Bounded cleanup | Strong fixed-catalog source cleanup; A' can retain the same catalog and trivial elements. | Non-owning frames have no recursive source drop; scratch allocation and derived byte-block release are prepaid. Plan rollback still needs bounded reclamation. | Phase ownership must stay explicit; integrating phases does not legalize a large implicit draft destructor. | Different catalogs are acceptable when each is exhaustive and release is bounded. |
| `no_std + alloc` | Existing arena/library compilation shows feasibility; full target runtime remains open. | References, slices, borrowed iterators and checked scratch are viable without atomics/erasure. Typed lifetime may affect application API. | Use a caller-driven transaction with uniquely owned budget/slots; Host may adapt it. | Static generic/private closed adapters can avoid a mandatory allocating trait object. No Host-only map backend obligation on thumb. |
| Planning independence | TD semantic view hides storage. | Hide `&Thing` behind TD-produced facade; no raw structs, schema walker, or W3C helper copy in Planning. | A private validated phase/capability enforces the same restriction; plain “validated” boolean plus raw `Thing` is insufficient. | Interface agreement, not byte/arena identity, protects Planning. Do not allow callers to implement an unchecked trusted backend. |
| Plan/Artifact ownership | Owned after last view; generic artifact type must also be source-independent. | Same; copy output facts while loan is live, end loan, then exercise output after caller destroys Thing. | Same; no cursor or artifact may retain the transaction's semantic scratch. | Same drop-order test for each source backend, with owned runtime output as the convergence point. |
| Atomic publication and runtime freedom | Existing permit sequence releases Snapshot before install. | Complete output and end loan before final install; no source owner needs release. Derived scratch follows charged release order. | All fallible/cancellable checks precede one permit/install; no intermediate trusted phase is externally published. | Source-specific release precedes install, or occurs earlier after the last source loan; Published remains TD-free. |

### Physical resources and complexity

| Effect | A / A' | B | C | D |
| --- | --- | --- | --- | --- |
| Memory accounting | Observable full copy; A' charges spare capacity and may admit less input under the same ceiling. Caller typed baseline remains opaque in both. | Exact new scratch/derived/runtime storage, zero newly owned full source graph. Caller baseline stays outside core claim. | One reservation/physical ledger composition avoids independent admission allowances that forget overlap. | Upstream engine-owned ingestion source remains charged by its owner throughout any loan; global composition cannot erase it by calling it “borrowed.” |
| Build-time peak | Full Snapshot + Plan/Artifact residency, plus growth; A adds seal overlap. | Caller graph + new scratch/derived/plans at system level; core accounts the new component. Derived data can overlap Plan until transferred/released. | Same as physical choice; integrated transaction tracks maximum simultaneous owners, not sum of every phase maximum. | Include ingestion source, temporary construction, core build, and runtime storage whenever simultaneously live. |
| Duplicated state/copying | Complete typed copy, canonicalization, equivalence; future strict construction can include literal-to-canonical rebuild. A' removes only exact seal copies. | Only required derived/results copied; original opaque fields are inspected for limits/Basic as applicable and remain caller-owned. | Does not eliminate the independent output, counts, or reserved ownership needed for barriers. Avoid a second aggregate or lookup in Servient. | Duplicate semantic rules forbidden; different storage adapters do add implementation paths. |
| Implementation/evidence burden | Large full model/decoder/catalog/parity/configuration composition. Current probes reduce uncertainty. | New charged typed traversal and semantic facade; a smaller catalog, no full copied-model equivalence or strict decoder prerequisite. | Fewer public intermediate types, but phase/cancellation/barrier tests still required. Do not make Servient a combined TD/planner implementation. | Additional source-specific evidence only when ingestion enters scope; common semantic and output contracts remain reusable. |

**A is conforming but larger than the identified core v1 need.** It supplies the
currently promised strict capability most directly, and remains the fallback
if that capability cannot be deferred. No counterexample here makes A unsafe.

**A' is a valid smaller change if full ownership is retained.** Exact sealing
does not create immutability: access control does. Given observed capacity,
private trivial arena elements, bounded charged growth, and an exhaustive
release set, holding a 128-element allocation with 100 initialized elements is
fully accountable as `Layout::array::<T>(128)`. No semantic traversal of the 28
unused elements is needed. A' may need more source/build capacity and yields no
automatic saving when spare capacity is large. It removes one mechanism, not
the full copy or decoder dependency. [R6, R12, R16]

**B is the simplest physical shape for typed core input.** Rust borrowing
protects input stability; the semantic capability protects validation; controlled
scratch protects work/cleanup; owned plans protect runtime independence. None
requires a complete normalized source copy. Its missing constructive proof is
a migration gate, not a reason to pretend synchronous Basic is bounded.

**C is useful composition, not permission to fuse semantic owners.** Taken
alone, a single pass that starts an early compiler before validating a later
affordance fails the current full-validation/all-bounds guarantees. Integrated
phases with private barriers succeed. Keep a TD proof capability even if no
independently public `ValidatedThing` type survives.

**D is the clean extension architecture.** It shares meaning rather than forcing
all ingestion onto arenas. Only implement the typed backend now. A future
strict arena backend must pass its own work, resource, semantic and output
independence evidence. It is not admitted by an interface diagram.

## 8. Concrete target and guarantee suppliers

### TD proof and semantic access

The core entry takes a stable typed loan for the whole TD/Planning build.
Conceptually `ValidatedThing<'td>` contains a private proof tied to that immutable
input and captured applicable policy, checked counts, and access to any charged
derived semantic state. This is an illustrative lifetime boundary, not a frozen
Rust signature. Construction exists only through TD's checked path. Planning
receives a restricted validated view, never `&Thing`, raw source containers,
storage offsets, or an arbitrary user-implemented “trusted” trait.

Use the existing storage-neutral Basic/default/security rules. Retain complete
Basic acceptance and first-error order, the specified five-predicate numeric
amendment, explicit-empty security overriding inheritance, operation defaults,
original Form indices, and key-order Property ordinals. Don't use JSON
serialization as validation, resource measurement, or typed input equivalence.
Typed timestamps are already fixed values and need no RFC3339 parse. Opaque
extensions retain their supplied typed associations. [R6, R13, R15, R18]

The view can borrow names/content/explicit operations/security. It cannot return
a borrowed resolved URI that was never stored. TD must compute resolution under
`UriBytes`/output and temporary limits, retain only required derived bytes in a
charged build block, and lend the completed result; or use a TD-owned charged
resolution operation that emits into transaction-owned output. Select the
smaller constructible mechanism during the migration prototype. Planning owns
URI-template compilation; it must not implement TD URI resolution. Reuse #111's
semantic resolution owner and direct-output evidence. [R5, R6, R13, R16]

A compact table of borrowed references/derived facts may be justified by charged
lookup or cached results. It is **not** a complete TD copy and must not grow into
one for API convenience. Its allocations and inline continuation bytes remain
explicit. The allocation-free-query benefit of today's cached Snapshot can be
preserved for finalized facts, without requiring every raw field to be copied.

Allocation-free does not mean work-free. An arbitrarily long explicit operation
list or security-reference lookup cannot be rescanned by each view query without
charge. TD can capture effective ReadProperty membership and resolved security
facts during its charged pass, or lend a charged semantic continuation. Planning
still owns operation/candidate eligibility, but neither its convenience queries
nor a default/security helper may bypass the source's work envelope.

### Bounded traversal and cleanup, rather than a free borrow

Before entering a variable-sized collection, inspect its cheap length against
the applicable count bound. Then retain monotonic borrowed iterators, bounded
slice positions and byte-comparison continuations. Use a bounded nonrecursive
frame workspace whose elements borrow external input and own no recursive
values. Charge frame requests, old/replacement overlap, moves and prepaid
release. Existing typed-construction `Children` and literal-construction borrowed
`Frame` types demonstrate this mechanics direction. [R16]

Existing `TypedBasicAccess`/`TypedAccess` are synchronous semantic evidence:
several accessors use `.nth(index)`, `filter().count()`, or map lookups. They are
not a ready bounded cursor. A bounded adapter must not restart those scans on
each poll or hide arbitrary string comparisons inside a node charge. Preserve
iterators, charge inspected key bytes, and use a small borrowed-reference index
where repeated lookup requires it. A bounded atomic container primitive needs
a justified structural maximum and full work debit; public container APIs do
not expose private node layouts or establish target cycle limits. If these
mechanics cannot meet the admitted work contract, that falsifies B's prototype
and requires a narrower derived representation, not a silent weaker budget.
[R3, R15, R16]

Full structural inspection covers opaque values for applicable depth/node/
member/string/extension and Number limits, while Basic visits exactly its
existing semantic predicates. No new “opaque extension validation” rule is
introduced. Retain one non-resettable TD lifetime allowance across all TD work;
repeated inspections/projections pay again. Planning uses its own monotonic
`PlanningItems` accounting and existing structural/work eligibility envelope.
Zero budget, insufficient atomic credit, cancellation and first-cause handling
keep their existing meanings. [R3, R5, R6, R9, R18]

Dropping an admission cursor releases engine-owned frame/derived blocks and
prepaid reservations only. It does not drop the caller `Thing`. If private Plan
rollback grows with plan count, use the existing bounded reclaim owner; don't
replace fixed Snapshot drop with an unbounded `Vec<Plan>` destructor. Final
publication must wait until all scratch cleanup required for the non-fallible
install boundary is complete.

### The Consumer transaction and barriers

1. Servient captures immutable configuration and complete registration
   identities, validates applicable policy/support, and reserves/caps the
   parent/local build allowance before controlled work/allocation.
2. TD structurally inspects and completes Basic under bounded progress. Failure
   rolls back TD-owned scratch before returning its terminal result.
3. TD exposes the validated capability. Planning preflights all Properties/
   readable Forms and slice eligibility. Missing ID remains a preflight failure.
4. Servient reserves the conservative runtime/record/slot envelope from that
   preflight; Planning materializes complete owned inputs under the same
   transaction's peak accounting.
5. Planning evaluates every pure compiler bound and completes the existing
   all-coordinate barrier. Any materialization/bounds failure produces zero
   starts, complete rollback, and no partial lookup.
6. Planning sequentially compiles, reconciles actual artifacts, and seals the
   complete owned draft. Every TD/derived-fact/registration input loan ends.
   Exercise draft selection after caller source destruction in external evidence.
7. Release unnecessary derived/traversal state at the earliest safe boundary.
   If release requires stepped work, finish it while still unpublished and
   cancellable; retain its charge until physical release. Perform final
   identity/generation/resource/cancellation/registry-slot checks.
8. Issue one private publication permit. The reserved complete record installs
   without allocation, callback, yield or fallible check. Any remaining fixed
   prepaid build release can precede install in this non-fallible interval; no
   source or semantic query is needed. Published owns runtime material only.

Ending a caller loan is not deallocating caller memory. The input need not be
destroyed in the final permit interval; it simply cannot be retained by the
runtime. If an engine-owned ingestion source is composed later, its source
owner must release and reconcile its charges under its own proved order before
the applicable install boundary. Preserve the child-before-parent physical
release rule wherever charged storage exists. [R3–R8, R12]

This order keeps the complete validation barrier before Planning and the full
bounds barrier before `start`, which is stronger than merely forbidding early
protocol I/O. Count/preflight passes and conservative reservations remain;
“one transaction” does not mean an unsafe one-pass streaming compiler.

### Resource accounting where resources actually live

At time `t`, the core admission's controlled live storage is the disjoint sum:

```text
Live_core(t) = TD derived facts + TD traversal workspace
             + Planning temporaries + logical/candidate/lookup capacity
             + artifact and live compiler capacity
             + registration/record/slot capacity attributable to this owner
             + diagnostic and cleanup capacity
Peak_core    = max_t Live_core(t)
Largest      = max over actual controlled allocation requests
```

Each physical block is counted once even if ownership changes. A preflight
aggregate reservation is a logical allowance, not one contiguous allocator
request. Check/reserve an actual proposed `Layout` before allocation; charge
actual observed capacity, including old/new grow overlap. An ordinary
`Vec::reserve` followed by observing capacity is insufficient if it can exceed
the reserved envelope *before* the check. Use controlled checked allocation or
a proven conservative growth envelope; returning `Limit` after excess allocation
does not satisfy admission. Immutability can retain charged spare capacity.
Do not equate `shrink_to_fit` or conversion to boxed storage with a portable
pre-admitted exact seal. [R3, R6, R9, R16]

The caller's opaque graph remains baseline memory. At system level,
`Live_system(t)` includes that still-live graph plus `Live_core(t)` and other
owners; borrowing does not erase peak residency. If another engine component
allocated the source under a bounded policy, its original ledger must remain
included in global live/peak accounting while core borrows it. An ownership
label cannot launder an engine allocation into uncharged “caller” memory.

Long-lived resource admission is primarily the actual Plan/Artifact/lookup/
registration/runtime-record capacity, already required by `ADMIT-MEM-001` and
`PLAN-COST-003`/`PLAN-SET-001`. An intermediate TD footprint cannot bound it:
compiler artifact sizes, cursor overlap, metadata allocation, erasure overhead,
and cleanup records have independent owners. Keep conservative preflight
reservation and exact output reconciliation; don't allocate plans freely just
because a TD bound passed. First Consumer retains zero full-source bytes, but
the source/document accounts remain available for genuinely owned source in
other capabilities. [R3, R5, R7, R9, R14]

The same observability discipline applies to output containers. Replacing a TD
Snapshot with an opaque owned map inside a Plan and estimating its bytes from
length would repeat 0065's defect. Controlled arrays/blocks or independently
proved capacities are needed for project-owned output. A binding artifact stays
opaque to Planning: its author reports conservative final/cursor/temporary
bounds and an honest actual footprint under the existing compiler contract.
That declaration needs evidence from the binding; a compiler-supplied scalar
is not automatic allocator introspection or a whole-system physical-memory
proof. The full Consumer accounting join remains work even after removing TD
copying. [R5, R9, R11, R14]

The recommendation changes which storage is charged, not which facts are
required: local/global scope, peak overlap, largest request, allocator-specific
surcharges when selected, and inline/static owner costs all survive. Host
artifact erasure and static payloads have different physical accounting; they
need not use identical arenas. None of this is fully proved by today's leaf or
fixtures.

### Policy and Number support

An opaque standalone `ValidatedThingAdmissionConfig` is not required to prevent
bypass. A checked Consumer transaction can retain one immutable profile and
give TD a private narrow validated projection. TD must still validate its
applicable limits and the numeric implementation's supported envelope; Servient
must not invent TD parser support. Complete role/cell validation stays with the
profile owner, and unrelated `NA` fields must not invalidate a TD operation.
Direct advanced TD entry, if retained, needs the same checked operation boundary
before work; its internal implementation may still use a small opaque record.
[R3, R6, R10, R12]

Keep the finite per-Number limit, typed borrowed-length check, complete atomic
projection debit, unsupported-configuration versus per-input-limit distinction,
and selected Basic amendment. Dropping normalization removes lossless-copy
work; it does not make projection, numeric overflow/rounding behavior, opaque
Number resource limits, or downstream AP feature unification disappear. Do not
silently remove `td/validated-thing`'s proposed lexical-access capability or
restore 256 as a universal implementation maximum. Re-evaluate the exact feature
name/API surface and supported `M` in the later authority migration. No target
cycle/stack claim follows from this proposal. [R6, R12, R18]

### JSON and constrained deployment scope

The architecture's `no_std + alloc` axis describes compilation and ownership;
it does not require every MCU to ingest arbitrary untrusted TD JSON at runtime.
A constrained Consumer can operate from a provisioned/preconstructed typed TD
and use bounded core validation/Planning/publication. The application must
provide its input graph's construction, allocator and eventual destruction
budget. `Thing` is an allocation-backed authoring model, not a heapless static
TD format, and moving allocation to provisioning does not prove its memory
cost. [R1, R3, R12, R13]

Host ingestion via ordinary serde remains useful but advertises no bounded
core parsing guarantee. Directory/network TD ingestion is a real later product
capability; WP-500 and deferred `DIR-STREAM-001` make its own bounded entry review
necessary. It should not silently reuse an unbounded serde graph and claim
absolute engine admission. When admitted, a lexer/owned source backend can
keep ADR-0020 literal values and shared TD field decoding, then lend the same
validated semantics to Planning. An explicit bounded adapter may be needed
before a deployment or later v1 release claim; deferral is not permanent removal
of Discovery's requirements. [R2, R6, R10, R18, R19]

No reconstructed product evidence establishes that this first Consumer gate
must supply the universal external-ingestion capability now. The current strict
requirement arose as the controlled alternative to retaining opaque caller
graphs. The recommendation narrows core scope openly, retaining the correct
semantic decision for any future strict path.

## 9. Exact disposition and eight-item readmission impact

| Boundary | Recommended disposition after independent review and migration |
| --- | --- |
| `ValidatedThing` | Retain the TD-produced proof/typestate concept over an immutable typed loan; prefer private transaction capability plus a restricted view. Do not freeze a standalone owned public object for continuity with previous names. |
| Arena normalized Snapshot | Remove as a mandatory typed Consumer intermediate. Retain prototype/history and potential future owned ingestion backend. No retained runtime Snapshot. |
| Exact seal | Remove mandatory TD exact-length copy. Semantic freeze and charged actual capacity suffice. Exact allocation remains an optional local optimization when its overlap/work improves a measured constraint. |
| `ValidatedThingAdmissionConfig` | Remove the independently exposed configuration obligation; retain checked narrow TD policy/support validation inside the transaction. Never accept a raw unchecked policy by default. |
| Strict JSON entry | Defer as a separately scoped bounded ingestion capability; preserve ordinary serde and ADR-0020's explicit literal/ordinary distinction for a future strict path. This is the proposal's scope change requiring explicit migration. |
| WP-100 / WP-200 | WP-100 owns TD validation, structural/work limits, effective semantic access, required derived TD state, and TD rollback. WP-200 owns candidate/preflight/output/lookup/compiler coordination and its footprints; it receives only validated semantics. WP-400 owns policy capture, parent reservations, transaction orchestration, final publication, and runtime lifecycle. |
| Planning resources | Make actual output/cursor/temporary/record capacities the runtime admission basis, retaining complete conservative preflight and artifact bounds. Account all simultaneous physical owners at build time. |
| Current eight-item model | Keep it binding until migration. Replace only mechanism-specific acceptance claims, while preserving every live invariant through risk-appropriate new evidence. Do not claim existing probes complete replacement readmission. |

The eight existing items are an evidence plan for the **selected full-Snapshot
contract**, not eight independent product requirements. Their replacement is
traceable as follows; this is a migration impact map, not a new gate registry.

| Existing item [R6] | What must remain | What changes or becomes obsolete |
| ---: | --- | --- |
| 1. Public signatures/removals | Compile/negative proof of unforgeable validation, immutable view, no raw-TD Planning bypass, linear progress/reservations, and no input borrow in published output. | Owned owner/cursor/builder/config/footprint signatures and exact old normalization terminals are superseded. Define only the target actually needed, then review API compatibility. |
| 2. Allocation catalog | Exhaustive physical sites for typed traversal, derived facts, Plan build, rollback; checked requests before allocation, overlap and exact release. | Exactly three retained/four temporary categories and TD seal-copy formulas cease to be mandatory. Future bounded ingestion has its own catalog, possibly reusing these arenas. |
| 3. Semantic equivalence | Complete Basic/query/default/security/URI regression, numeric amendment, first-error order, input immutability, output completeness, serializer-failing typed acceptance. | No full typed-copy equivalence pass when no full copy exists. Strict wire/field/duplicate/null/Number-content construction is deferred with ingestion. Cross-source equal-logical-value agreement remains required when a second backend is admitted. |
| 4. Planning view/handoff | Non-first coordinates, original indices, complete owned aggregate/artifacts, last-loan end, selection after source destruction, separate registration ownership. | Replace Snapshot-drop-only witness with typed caller-loan end and actual Thing destruction after build. Repeat through target production surface. |
| 5. Feature/profile matrix | Host/static semantic agreement, no-default/async/std cells, actual thumb compile, downstream/sibling capabilities and public-surface negatives; separate runtime/target claims. | Arena/strict constructor surface cells move to that capability. Retain numeric lexical-access/AP evidence where typed bounded validation uses it. Do not equate thumb compilation with execution. |
| 6. Policy/progress/terminal | Applicable policy/support checks before work, charged typed traversal and lookups/URI/Number handling, monotonic lifetime, zero-budget behavior, first cause, cancellation, failure and abandonment cleanup. | Decoder/unescape/date/sort/seal/equivalence phases disappear from typed admission. Planning phases remain independently charged. Owning-source relocation requirements do not apply to an external typed loan. |
| 7. Complete resources | Actual capacities, peak overlap, per-request contiguous facts, local/parent/global pairing, inline/static costs, Plan independence, zero source in Published, complete release. | No invented Snapshot footprint or source allowance in a typed-only path. Measure real scratch/derived/output owners; upstream owned sources stay charged if composed. |
| 8. Prior impact | Exact-diff impact review of Foundation, Context authority, resource schema, completed WP-200/WP-300 leaves and Producer gate; reopen only falsified/intersecting claims through their owners. | Remove the already orphaned narrow document-reclassification method in a later authorized source change; don't reopen disjoint gates merely because this proposal revises Consumer input. No self-awarded passed status. |

Retaining all eight old items unchanged would force proof of objects the target
doesn't need. Waiving them without replacement would remove real safety proof.
The smallest valid migration rewrites their claims around the actual new owners
and performs independent acceptance at an exact revision, followed by normal
separate tranche admission.

## 10. Reuse and work that should stop

| Recent work | Reuse under the recommendation |
| --- | --- |
| #69 Foundation and #70 Context impact | Keep `DocumentNodes`/`PlanningItems`, unique budgets, ordinary ledger accounts and work/reservation tests. Reuse private Context semantic-inspection rationale; no production seam is assumed to exist. The source-to-persistent-document transfer is already orphaned by 0072. |
| #94 arena and #95 Number lexer; typed storage #96–#109 | Keep checked allocation/growth/release mechanics as possible scratch/ingestion building blocks and semantic corpora. Whole typed Snapshot productionization is not needed for B. Lexer proof belongs to future strict input. |
| #108 RFC3339 | Shared decoder semantic/resumability evidence remains useful for bounded ingestion. Typed admission copies/borrows typed dates and needs no decoder integration. |
| #110/#111 TD semantics and URI/query proof | Reuse shared default/security/URI rules, original-coordinate corpus and restricted view behavior. Adapt storage/lending; don't duplicate rules in Planning. |
| #112/#113 lifetime and owned handoff | Retain TD-free runtime and source-independent draft conclusion. External concrete artifact/drop-order proof is central; rerun with a typed loan. Snapshot-specific source-release measurements become historical backend evidence. |
| #114/#115 full shared Basic | Reuse rule extraction, public/inline diagnostic separation, complete acceptance/first-error and numeric corpus. Add a genuinely charged typed driver; `.nth`/bulk filters in synchronous adapters are not that driver. |
| #116/#117/#118 strict decoding decision | Preserve ordinary wire observations and literal strict semantics; don't reopen them merely to defer the decoder. Their full construction duty moves with ingestion capability. |
| #119–#123 literal/schema construction | Retain common field-policy, Number, charged comparison, request-precedence, frame and cancellation evidence. Arena rebuild/reseal mechanics are future-ingestion evidence, not typed-core prerequisites. |
| #124 typed Thing construction and #126 paid/movable Basic | Reuse full typed field corpus, complete Basic discovery/predicates, first-cause and budget traces, allocator-failure methodology, bounded frame transfers, and concrete ownership lessons. Index/range source continuations remain appropriate for an owned ingestion backend. Borrowed input needs no source-owner self-reference. |
| Completed Consumer call values, exact WP-200 leaf and WP-300 execution correction | Keep options, response sealing, complete-registration/name-free request boundary, generations, and owned artifact behavior. A later aggregate adapts its input without silently changing the completed leaf. Producer evidence remains separately owned. [R7, R14] |

After this decision is independently accepted, stop investing in the following
as **prerequisites of typed Consumer integration**: full typed arena copying;
whole-model post-copy equivalence; mandatory exact TD reseal; completing every
strict Thing/Form/Context/Link/response decoder join; owning normalized-source
continuations solely to satisfy the old public signatures; Snapshot-only
configuration/footprint plumbing; and satisfying three/four-site catalog counts
for their own sake. Preserve their source/history and disjoint evidence. Do not
delete fixtures or change their historical claims in this PR.

Work that should continue is the common semantic kernel, bounded typed driver,
derived URI facts, Plan preflight/output accounting, all-bounds barrier, complete
registration execution, and atomic publication evidence. Neither the copied
Snapshot nor its replacement validator alone completes Consumer admission.

The current record remains candidate/not admitted. This proposal does not
silently authorize a borrowed production implementation or change remote
tranche state. Pending independent review, don't build further mandatory-copy
mechanisms on the assumption that this direction has already been accepted.

## 11. Smallest safe migration sequence

1. **Independently review this proposal.** Reconstruct the input/Plan/runtime
   guarantees from the pinned authority and diff. Resolve whether a committed
   immediate bounded-runtime-JSON use case defeats the scope recommendation.
   Acceptance is a separate fresh architecture context, not this author's
   successful checks or the PR's merge alone.
2. **Discriminate the borrowed mechanics with one focused prototype.** Reuse
   the shared kernel/corpus and current external handoff fixture. Demonstrate
   charged full typed traversal, restricted effective queries with relative URI
   derivation, movable external-input continuation, checked scratch capacity,
   cancellation/drop, and owned artifact/output use after loan end. Include
   long common-prefix map/security names, nested opaque values/schemas,
   explicit-empty operations/security, Number limits, serializer-failing input,
   absent ID, and later-coordinate failure. This is non-production feasibility
   evidence; no general new checker or implementation framework is needed.
3. **Migrate one coherent authority revision after that evidence.** A durable
   cross-domain ADR is appropriate because this changes API/input scope and
   reverses the normalized-owner choice. Update exact owners listed below;
   explicitly retain/defer the strict capability and ADR-0020 semantics.
   Replace obsolete readmission claims, preserve active guarantees and distinct
   evidence applicability, and keep the tranche candidate until reviewed
   admission. No need to invent new requirement IDs merely for phase names.
4. **Admit and implement TD, then aggregate/runtime in their existing owners.**
   Use normal ADR-0013 boundaries. Complete the typed proof/semantic facade and
   bounded policy/work/cleanup evidence; then implement WP-200 preflight,
   complete output, all-bounds barrier and actual footprint. WP-400 supplies
   parent pairing, runtime reservation, permit/install, lease/drain/reclaim.
   Preserve the completed exact leaf's behavior while adapting aggregate input.
5. **Accept the end-to-end result and dispose obsolete surfaces.** External
   Host/static traces must poison raw TD runtime access, exercise failures at
   every barrier, prove source/registration-independent output, release all
   controlled memory, and preserve cancellation/publication linearization.
   Remove orphaned source reclassification through its owning authorized source
   change. Register/accept the Consumer gate only after its existing entry
   conditions hold; proceed to real Host Zenoh evidence. Bounded ingestion
   enters later through its own exact capability/domain requirements.

If the prototype shows a minimal derived table is insufficient, expand only the
specific TD facts whose charged access is otherwise unconstructible. If that
forces full ownership or an immediate strict capability is required, revisit A'
or A with the actual counterexample. Do not hide such a failure in uncharged
queries, unrestricted raw access, or an implicit copied Thing.

### Authoritative owners affected by a later migration

| Owner | Required change/impact |
| --- | --- |
| Architecture 10/20/30/50 and ADR index/new ADR | Replace normalized-source flow with typed proof and transaction; preserve runtime/semantic/module invariants. Record strict capability scope and historical supersession precisely. |
| Runtime safety and Foundation | Revise admission-specific Snapshot/strict/config refinements, not the root resource/transaction contracts. Define baseline, derived storage, policy/support checks, physical overlap/release. |
| WP-100 admission record and WP-100 package | Replace materialized owner/build phases/public signature catalog and readmission claims with typed validation/view duties. Separate future ingestion evidence. |
| Planning specification and WP-200 | Change source capability while preserving preflight/complete aggregate/bounds/owned output and actual `PlanFootprint` responsibilities. No semantic rules migrate to Planning. |
| Servient architecture/spec projections and WP-400 | End typed/derived loans, reconcile actual live owners, and preserve final checks/permit/atomic install without an invented Snapshot release. |
| API ownership registry | Remove/replace frozen absent Snapshot/builder/config/footprint entries coherently; specify new proof/view entry only after prototype. Audit legacy owned `consume` migration. |
| State machines | Replace normalization-only input/normalize/seal/equivalence with actual TD validation/derived-state phases; preserve first cause, rollback, compiled-set and publication transitions. No extra public permit state is needed. |
| Resource schema/generated projections and Number amendment | Retain finite structural/work/Plan/temporary/peak controls and numeric behavior. Review typed semantic byte-limit meanings and operation applicability; source rows remain valid for other owners. No row removal/addition is justified solely by fewer arenas. |
| Tranche/evidence records and checkers | Revise only affected requirements, API items, paths, predecessor/completion claims and mechanism assertions. Keep candidate/admission/gate control independent. Preserve historical probes as scoped evidence and reaffirm disjoint completion claims against the exact diff. |
| Roadmap | Update only if a durable prerequisite changes, such as decoupling strict ingestion from Consumer integration. Do not record this review's task/PR/checklist state in `PLAN.md`. |

This table is an impact inventory for independent review. None of these
authoritative files is modified by this PR.

## 12. Risks, unresolved questions, and falsifiable acceptance

**Bounded typed traversal is not yet constructed.** The rule kernel and borrowed
iterator mechanics are demonstrated separately, but typed Basic adapters hide
repeated scans today. Prototype worst-case key/reference work, frame growth,
zero/small budgets, numeric atomicity and all failure/abandonment positions.
Fail the recommendation if the necessary bounds cannot be supplied without
violating active complexity/work guarantees.

**Input lifetime changes API ergonomics.** A caller must retain `Thing` until
the build finishes. This is acceptable for the recommended bounded core entry,
but the old by-value `consume(Thing)` cannot silently drop an arbitrary nested
caller graph inside a bounded terminal. Review whether a separate Host
convenience returns the source, takes an explicitly caller-budgeted owner, or
remains outside the bounded core API. Do not preserve `Arc<Thing>` in Published
to make this migration easier.

**Semantic query lifetimes can recreate self-reference.** A transaction that
owns derived URI storage and a Planning cursor borrowing it cannot move that
pair naively. Use short per-step loans and private coordinates into that
scratch, or transfer owned derived results; external references may point only
to the immutably borrowed caller input. #126's ownership lesson still applies
to owned scratch even when the full Snapshot is removed. Do not solve this with
hidden lifetime erasure or an unrestricted runtime source pointer.

**Baseline and peak claims are easy to overstate.** Typed core admission bounds
new controlled state, not pre-entry Thing construction, allocator metadata or
whole firmware/process RAM. A caller graph stays live through Planning, and
owned ingestion sources remain globally charged. Measure peak at shared owner
boundaries; never add unrelated phase maxima or subtract destroyed storage from
a historical peak. The future PlanFootprint is not implemented today.

**Typed document byte limits need precise meaning.** An already typed Thing
has no original wire length. Structural/string/extension/Number limits remain
useful, but migration must specify their logical content/counted-byte oracle
without serialization or an opaque-container census. If `document_bytes_max`
remains wire-specific, classify it as ingestion policy and retain typed
structure/work limits; do not invent a byte estimate and call it physical
storage. This detail must be resolved before production admission.

**Strict-input deferral may intersect a real product commitment.** Repository
authority establishes a useful bounded ingestion capability, but no admitted
first-Consumer use requires its runtime JSON entry. An Owner deployment
counterexample can overturn deferral. Accepting typed-only core does not permit
later Directory/Discovery release claims without bounded external ingestion
evidence. If such input is required immediately, D with an explicitly admitted
owned backend or A' is preferable to an unbounded parse-and-borrow shortcut.

**Semantic storage independence is not free.** Preserve private TD adapters,
one rule source and concrete ownership constraints. A public open trait that
arbitrary code can implement must not mint the validated proof. A generic
artifact without an explicit lifetime can still contain a borrowed payload;
test actual concrete artifacts and registration callbacks. Extend the semantic
view for future operations only from demonstrated compiler needs.

**Numeric/URI guarantees survive the removed copy.** Public Basic still needs
the intended failed/non-finite projection amendment. Opaque typed Numbers
retain resource limits; support configuration and bounded URI derivation remain
unresolved integration work. No AP, target-stack, or cycle evidence is waived
merely because a full copied snapshot disappears.

**Rollback shifts emphasis to real runtime drafts.** Fewer source blocks do
not make Plan destruction constant-time. Prove bounded draft rollback/reclaim,
prepaid cleanup ownership, exactly-once compiler abort and child/parent release.
No callback/allocator/yield/failure may enter the permit-to-install interval.

The falsifiable completion boundary for the recommended design is a complete
typed Consumer transaction that validates the entire input, exposes only TD
semantics, accounts every controlled owner before work/allocation, fails later
coordinates with zero premature compiler starts, seals source-independent owned
output, ends all input loans, and atomically publishes without source retention.
It must preserve Host/static semantic outcomes and release everything on
failure, cancellation, abandonment and eventual runtime reclaim. A passing
Snapshot fixture or a smaller interface alone is not that evidence.

## 13. Verification of this review

The review reconstructed the fetched default branch, checked GitHub's default
branch and relevant merged PR metadata, inspected actual production and fixture
source, and traced historical ownership changes through the linked workspace
topics and merge diffs. PR descriptions were used to identify scope, then
checked against source/history; their acceptance claims were not imported as
architecture conclusions.

At the pinned baseline these existing checks were executed successfully on the
local Host with rustc 1.99.0 / Cargo 1.99.0. Repository mainline CI selects Rust
1.95.0; these local runs are not a claim that its remote checks were rerun:

```sh
tools/check-design-artifacts.sh
cargo test --locked -p clinkz-wot-td --lib external_planning -- --nocapture
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test thing_basic --test owning_basic
```

The authority check validates the unchanged active 65-requirement source set,
state/DAG/API/ADR/resource projections (196 resource fields, three profiles),
and registered performance fixtures. The handoff test reproduced the reported
source/output/overlap bytes. The semantic fixture ran its two complete-public-
Basic/default-security tests and six owning-Basic composition tests. The TD
test build emitted an existing unused-constant warning; tests passed.

These checks corroborate this review's stated existing evidence boundaries.
They do not test the proposed borrowed implementation, complete any readmission
item, accept architecture, or establish constrained runtime/performance claims.
The proposal/index diff is separately checked for whitespace, valid local
references, and docs-only scope before submission.

[r1]: ../docs/architecture/00-system-goals-and-context.md
[r1-flow]: ../docs/architecture/10-primary-data-flows.md
[r1-mod]: ../docs/architecture/20-module-boundaries.md
[r1-runtime]: ../docs/architecture/50-servient-runtime-lifecycle.md
[r2]: ../docs/spec/v5-authority-reset.toml
[r2-index]: ../docs/spec/README.md
[r3]: ../docs/spec/foundation.md
[r4]: ../docs/spec/runtime-safety.md
[r5]: ../docs/spec/planning.md
[r6]: ../docs/work-packages/WP-100-consumer-validated-thing-admission.md
[r7-100]: ../docs/work-packages/WP-100-core.md
[r7-200]: ../docs/work-packages/WP-200-planning.md
[r7-400]: ../docs/work-packages/WP-400-servient.md
[r7-index]: ../docs/work-packages/index.toml
[r8]: ../docs/state-machines.toml
[r9]: ../docs/resource-limits.csv
[r9-ledger]: ../foundation/src/resource.rs
[r9-work]: ../foundation/src/budget.rs
[r10-plan]: ../docs/ADRs/0008-compiled-plan-lifecycle.org
[r10-policy]: ../docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org
[r10-consumer]: ../docs/ADRs/0019-consumer-one-shot-authority-entry.org
[r10-json]: ../docs/ADRs/0020-strict-json-value-decoding.org
[r11-handoff]: 0063-consumer-plan-set-handoff-closure.md
[r11-serde]: 0064-serde-json-retained-representation-impact.md
[r11-btree]: 0065-liballoc-btreemap-retained-representation-impact.md
[r12]: 0072-consumer-validated-thing-build-lifetime.md
[r12-target]: 0071-constrained-resource-authority-and-target-characterization.md
[r13-thing]: ../td/src/thing.rs
[r13-validation]: ../td/src/validate.rs
[r13-defaults]: ../td/src/td_defaults.rs
[r13-uri]: ../td/src/core/data_type/uri.rs
[r13-context]: ../td/src/components/context.rs
[r13-cargo]: ../td/Cargo.toml
[r14-input]: ../planning/src/lib.rs
[r14-leaf]: ../planning/src/property_read.rs
[r14-plan]: ../core/src/plan.rs
[r14-artifact]: ../core/src/binding_compiler.rs
[r14-consume]: ../servient/src/servient.rs
[r14-legacy]: ../core/src/thing.rs
[r15]: ../tools/architecture-fixtures/validated-thing-schema-kernel/README.md
[r15-basic]: ../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_kernel.rs
[r15-typed]: ../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_typed.rs
[r15-schema]: ../tools/architecture-fixtures/validated-thing-schema-kernel/src/typed_access.rs
[r15-build]: ../tools/architecture-fixtures/validated-thing-schema-kernel/build.rs
[r16]: ../tools/architecture-fixtures/validated-thing-arena-layout/README.md
[r16-values]: ../tools/architecture-fixtures/validated-thing-value-construction/src/lib.rs
[r16-typed]: ../tools/architecture-fixtures/validated-thing-schema-kernel/src/thing_build.rs
[r16-owning]: ../tools/architecture-fixtures/validated-thing-schema-kernel/src/thing_step.rs
[r17]: ../tools/architecture-fixtures/validated-thing-planning-handoff/README.md
[r17-plan]: ../tools/architecture-fixtures/validated-thing-planning-handoff/src/planning.rs
[r17-test]: ../td/tests/support/planning_handoff_probe.rs
[r18]: 0073-shared-json-value-decode-boundary.md
[r18-number]: ../docs/amendments/WP-100-bounded-atomic-number-v1.md
[r18-date]: ../tools/architecture-fixtures/validated-thing-rfc3339-resumable/README.md
[r19]: ../docs/work-packages/WP-500-discovery.md
[r19-plan]: ../PLAN.md
