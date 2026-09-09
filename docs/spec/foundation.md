# Foundation Domain Specification

Status: active v5.0 authority.

This specification owns exactly eight active requirements:
`API-RESOURCE-001`, `CONSTRAINED-STORAGE-001`,
`CONSTRAINED-STORAGE-002`, `CONSTRAINED-WORK-001`, `RES-LIMIT-001`,
`RES-LIMIT-002`, `RES-LIMIT-003`, and `ADMIT-MEM-001`.
`docs/resource-limits.csv` is the exhaustive field and named-profile
projection. Logical time remains owned by the narrow completed amendment
`docs/amendments/WP-100-time-domain-v1.md`. ADR-0015 owns borrowed immutable
profiles and the linear work budget; ADR-0016 owns extended logical time.

Foundation owns protocol-neutral resource, work, generation, storage, and time
primitives. It does not own TD vocabulary, interaction errors, plans,
registries, queues, protocol behavior, or a host runtime.

## Resource API and limits

`API-RESOURCE-001`: `ResourceLimits`, `ResourceProfileId`,
`StaticResourceProfile`, `ResourceKind`, `ResourceAccount`,
`ResourceReservation`, and `AdmissionLedger` are public protocol-neutral
Foundation types. `ResourceLimits` is one complete immutable configuration
snapshot. It may implement `Clone` for deliberate construction-time
duplication but MUST NOT implement `Copy`. A named static profile exposes one
complete statically stored snapshot by reference; a profile identity is not
authority for values.

Runtime construction, admission, document processing, planning, codecs,
security, bindings, discovery, and interaction execution accept
`&ResourceLimits` or a validated handle retaining the same snapshot. Missing
policy never means unbounded. Reservations are move-only, generation-bearing,
and release idempotently unless committed to a published owner. Accounting
objects are advanced runtime and SPI building blocks; ordinary application
interactions inherit their Servient profile.

The exhaustive flat schema remains the authority as the field set grows.
Named profiles and generated role/profile builders are checked projections
that must construct one complete `ResourceLimits`; they are not independent
configuration formats. A caller either names one explicit profile or supplies
every field applicable to its selected roles. `None` is permitted only for a
field whose schema declares typed non-applicability; it never means inherit or
unbounded. Validation diagnostics name the field, scope, accounting owner, and
profile/role projection that supplied the value.

Changing the authoritative representation requires measured evidence that the
flat schema or its generated projections no longer provide bounded
construction, reviewability, or compatibility. Field count alone is not such
evidence.

The flat schema is the canonical resource authority, not by itself the stable
external authoring surface. The current raw `[Option<u64>;
RESOURCE_LIMIT_COUNT]` construction and mutation APIs are low-level canonical
assembly only: they do not prove that every applicable field is present for a
selected role/profile set. A `ResourceLimits` value becomes a validated
configuration authority only after the owning builder binds an executable
role/profile-cell applicability set and rejects every illegal `None`.

Before broad Protocol Binding or Servient resource authoring is described as a
stable external surface, the canonical schema and generator must provide:

- an authority class for each row: semantic capacity, lifecycle/scheduler
  policy, runtime topology, operational tuning, product default,
  protocol-private declaration, or workload-only parameter;
- executable applicability separated across capability role, compilation cell,
  execution model, and product profile;
- lifecycle status (`active`, `deferred`, `historical`, `retired`, or
  `provisional`) plus the active owner and evidence/default maturity;
- complete checked role/profile projections whose omission checks fail when a
  newly applicable field has no explicit value;
- structured diagnostics that retain value origin and every effective limiting
  scope; and
- one first-class schema revision, stable ordering rule, and schema/value
  digest for application-defined profile identity.

The existing `capability_roles` text and fixed field count are not sufficient
executable applicability or schema identity. `None` is legal only after role
binding and only for typed non-applicability; inactive or unimplemented
behavior is not silently equivalent to `NA`.

Within one schema revision, stable numeric ordering is append-only. A rename,
split, merge, semantic reinterpretation, applicability change, or retirement
requires an explicit schema revision and migration disposition rather than
reusing an old `ResourceKind` identity. `ResourceProfileId::APPLICATION_DEFINED`
identifies an origin class, not one value set; external configuration, caches,
and audit records pair it with the schema revision and value digest.

A new global row is admitted only with an active owner, distinct reservation
or validation point, applicability, structured diagnostic, default-maturity
classification, boundary/negative evidence, and a reason it cannot remain a
private binding/runtime declaration or workload parameter. Rows without an
implemented owner may remain provisional/deferred input but cannot support a
product-default or runtime-enforcement claim. Retirement or reclassification
preserves the old identity as historical and names the replacement or
non-applicability disposition.

A concrete binding may declare bounded protocol-private physical costs in its
complete registration and aggregate them into admitted lifetime/transient
footprints without creating a new global `ResourceKind` for every library
allocation category. Semantic owner counts and reservations remain comparable
across profiles; Host allocation/queue costs and constrained
slot/layout/code-size costs are separately bounded physical evidence.

`RES-LIMIT-001`: Every public ingestion and runtime-construction surface MUST
accept or inherit a resource policy before processing externally influenced
variable-size state. `docs/resource-limits.csv` is the single exhaustive field
schema. It bounds source and retained bytes, temporary and aggregate live
bytes, structures and work, retained owners, queues and buffers, cleanup and
diagnostics, protocol progress, and applicable time or step limits at their
declared scopes. `NA` means typed non-applicability. Omission, `inherit`, and
`unbounded` are invalid. Zero disables a resource unless the schema explicitly
declares rendezvous capacity; zero never means unbounded.

`RES-LIMIT-002`: A resource-policy violation MUST stop before rejected work or
externally reachable publication and return a structured limit category naming
the resource, configured limit, safely known requested or observed amount, and
phase. Processing MUST NOT silently truncate candidates, schemas, security
branches, documents, pages, extensions, or response opportunities to fit a
limit. Diagnostic exhaustion uses a bounded fallback without erasing the
resource category.

`RES-LIMIT-003`: Limits compose hierarchically at every configured scope,
including item, operation, Thing, client, principal or publisher when known,
binding or adapter, shard or local account, and global scope. Capacity is
charged before becoming locally visible and is reserved before publication.
Rollback and cleanup release it idempotently. Batching is allowed only when
batch size, idle capacity, reconciliation work, and return deadline are
bounded. Interaction hot paths MUST NOT require one process-wide resource
ledger mutex.

## Admission memory

`ADMIT-MEM-001`: Admission accounts source/input bytes, phase-local temporary
bytes, persistent document-retention bytes, persistent compiled-runtime bytes,
diagnostics, and cleanup ownership in distinct ledger accounts. It records or
can measure current live bytes, peak simultaneously live bytes, and largest
contiguous allocation. Phase-local storage is released at the earliest safe
boundary; atomic publication MUST NOT retain every phase's complete
representation. A failed source, temporary, persistent, peak, or contiguous
charge changes no published state.

Physically live engine-owned arena, pool, heap, or exclusively reserved
caller-provided capacity is charged. Verification records which representation
is measured. Rollback metadata MUST NOT duplicate the resources it protects.

For the first v5.1 Consumer Property Read aggregate, TD constructs a normalized
retained snapshot with three possible exact-length allocations: node, edge,
and byte arenas. Its structured footprint keeps five values distinct:

- total requested bytes of the live sealed arenas;
- count of their non-empty allocations;
- largest actual single allocation request across build, grow, seal, and final
  retention;
- peak simultaneously live temporary requested bytes; and
- peak simultaneously live project-owned bytes added by conversion over the
  selected entry baseline, including old/new grow or seal overlap.

An `AdmissionLedger::try_reserve_source` or `try_reserve_temporary` call for
this path corresponds to one actual checked `Layout` request. Callers MUST NOT
reserve an aggregate footprint through one such call and thereby report it as
a physical contiguous allocation. Account usage and peak-live values sum the
individually reserved requests; largest-contiguous observes their maximum.
The existing `retained_source_bytes_*`, `admission_temporary_bytes_*`,
`peak_live_bytes_per_admission_max`,
`admission_peak_live_bytes_global_max`, and
`largest_contiguous_allocation_bytes_max` rows therefore remain sufficient.
Allocation count is bounded by the frozen arena catalog and needs no new global
resource field.

`AdmissionLedger` is the generation-bearing per-owner/per-admission physical
account; it records local live, peak, and largest-request facts but is not by
itself the concurrent global aggregator. The admission coordinator owns the
corresponding parent/global allowances under existing `ResourceAccount` and
Servient resource-account authority. It caps the child operation before TD
entry, retains that outer reservation while the cursor is live, reconciles it
from the exact footprint on `Complete`, and releases it on every other terminal
or cursor abandonment. No callback or allocation occurs between outer
reservation and child-ledger ownership transfer. Implementing that coordinator
belongs to the later Servient tranche; this migration changes its contract but
does not advance it.

The arbitrary-`Thing` compatibility entry borrows a pre-existing typed value.
Foundation can guarantee only the additional project-owned conversion peak
over that entry baseline; it cannot retroactively charge caller allocation
history. The strict project-owned JSON builder/decoder begins accounting at its
first controlled allocation and provides an absolute engine-owned input-
processing-through-retention bound. Its borrowed input buffer remains caller
capacity but its length and processing work are bounded. The two entries use
the same source/temporary accounts and snapshot representation; this guarantee
difference is not hidden by one ambiguous peak scalar.

Inline `ValidatedThing` and Servient record capacity is not an allocation
request and remains in its owning slot/runtime capacity. It is not folded into
largest-contiguous. Allocator-private headers, bins, and rounding require a
separate allocator-specific surcharge if a profile chooses to govern them.

After TD-owned Basic validation, normalization, semantic-equivalence checking,
and exact-length seal, the completed owner keeps one `AdmissionLedger`. The
existing checked reclassification operation moves exactly the snapshot's total
requested bytes from source to persistent-document accounting after checking
destination capacity. Success changes no physical allocation, allocation
count, total live bytes, conversion peak, or largest actual request. Failure
leaves the source charge and normalized owner intact for rollback. It is an
account-classification transfer, not a second reservation, clone, or arena
allocation.

Normalization reserves in this order: check structural and lifetime-work
bounds; charge each work/byte unit before processing; check final or current
build capacity; check the source/temporary account, operation/global peak, and
actual contiguous request; precharge one `CleanupItems` unit for the live
allocation; reserve the exact checked `Layout`; then allocate. A grow or seal
reserves the replacement while the old allocation remains live and releases
the old charge only after successful transfer. Invalid input, Basic invalidity,
limit, cancellation, allocation/arithmetic failure, or equivalence failure
fixes the first cause and releases all partial account charges before exposing
a terminal. This tranche retains only a fixed inline invalid/limit/conversion
diagnostic (category, phase, and numeric coordinate), so its diagnostic account
stays at zero and no diagnostic allocation or resource row is required. The
existing allocation-owning public `ValidateError` adapter is not used as the
normalization terminal.

## Constrained storage

`CONSTRAINED-STORAGE-001`: A constrained runtime uses caller-owned bounded
arenas or tables for retained runtime objects. Every externally retained slot
reference contains an index and generation. Removal increments the generation
before reuse; mismatch returns a stale-handle error and never aliases a new
owner. A finite generation representation MUST retire a slot before wrap could
make a live stale reference valid. Lifetimes or unique ownership are conforming
alternatives, but a bare reusable index is not.

`CONSTRAINED-STORAGE-002`: Construction reserves all table capacity from an
explicit static profile. Admission reserves every slot and byte needed for
publication before publication. Exhaustion returns a structured limit error
without evicting live state. Variable-size state may use `alloc`, but every
allocation is charged; v5.0 makes no heapless claim. No-default support does
not imply host builders, tasks, sockets, filesystem storage, or `Arc<dyn ...>`
registration.

## Linear bounded work

`CONSTRAINED-WORK-001`: A work unit names a bounded cost class, including
applicable JSON/schema nodes, exact codec bytes, URI bytes, security branches,
provider probes, queue operations, binding progress, cleanup, and handler or
adapter progress. Work is charged before it starts. A step MUST NOT hide an
unbounded decode, collection walk, target expansion, or unrelated queue drain.
Non-incremental calls declare their maximum admitted input and external
worst-case execution responsibility.

`WorkBudget` is uniquely mutated and implements neither `Copy` nor `Clone`.
Every consumer receives `&mut WorkBudget`; copying an allowance to restart
fallback, probing, cleanup, handler work, or another step is nonconforming. A
partition operation may exist only if it atomically debits the parent.

The first Consumer aggregate requires two append-only `WorkClass`
discriminants after the existing ten entries: `DocumentNodes` and
`PlanningItems`. Existing discriminants and the first ten entries of
`WorkClass::ALL` remain unchanged. Their source projection belongs to the
separately admitted `WP-100-CONSUMER-VALIDATED-THING` tranche; this authority
does not itself admit that source change.

`DocumentNodes` charges generic typed-document validation, normalization,
semantic-equivalence, map-sort comparisons/moves, and structural visits not
already owned by a more specific class. Typed schema-node visits remain
`JsonSchemaNodes`; strict JSON bytes and typed source bytes read remain
`CodecInputBytes`; normalized bytes emitted or copied remain
`CodecOutputBytes`; URI bytes remain `UriBytes`; security branches remain
`SecurityBranches`; and destruction of one live arena consumes a prepaid
`CleanupItems` unit. Work is neither
relabelled nor double charged merely because it occurs during normalization.
Every accepted class-specific unit also consumes one unit from a shared
non-resettable lifetime remainder derived from the existing
`document_validation_work_units_max`; byte classes consume one unit per byte.
Prepaid cleanup consumes both units before its allocation becomes live. Host
may drive the same pure cursor to completion synchronously, while application-
static callers resume it; fresh per-step budgets do not replace that lifetime
remainder. Exhausting it is a resource limit, not Basic invalidity.

The normalized node and edge elements own no nested allocation or recursive
drop. The exhaustive three-retained/four-temporary allocation catalog makes
terminal release fixed and bounded independently of document depth. Deep input
rejection MUST NOT perform an uncharged recursive `Thing`, map, vector, JSON,
or task-tree drop inside TD. Compatibility input destruction remains with its
borrowed caller. Dropping an unpublished cursor may only consume its prepaid
fixed-allocation cleanup and release its owned ledger; any future fallible or
unbounded destruction requires an explicit cleanup owner and architecture
review.

`PlanningItems` charges aggregate enumeration, row construction, lookup
sealing, reconciliation, and reclamation. A monotonic cursor visits each
admitted property, Form, plan row, and artifact a fixed number of times. The
existing step limits bound one call, the existing document/Form maxima bound
the complete admission, and `plan_reclaim_bytes_per_step_max` bounds
reclamation. No new per-plan or per-admission resource row or generated getter
is introduced for either work class.
