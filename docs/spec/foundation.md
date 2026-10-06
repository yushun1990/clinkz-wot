# Foundation Domain Specification

Status: active v5.1 authority; Consumer admission refined by ADR-0021.

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

The Consumer typed admission projection is specified in
[the TD admission contract](../work-packages/WP-100-consumer-validated-thing-admission.md#typed-content-and-resource-projection).
It registers resource interpretation revision 2: `document_bytes_max` applies
`typed-content-v1` to an already typed loan; future strict ingestion retains its
consumed-wire-byte projection. This explicit operation-qualified refinement
keeps byte units, stable numeric row ordering and all 196 named values. This is
an explicit operation-schema migration: semantic identity includes the registered
interpretation revision and operation, not only the ResourceKind discriminant.
The former Snapshot interpretation is historical. Production checked handles,
configuration/cache identity and evidence must distinguish revision 2 typed
content from that interpretation and from wire ingestion. A shared numeric value
does not make their oracles interchangeable. No generated ResourceKind/getter or
low-level schema assembly behavior changes in this migration.

`number_lexeme_bytes_max` remains a provisional per-Number Consumer
`+validated-thing` control (gateway 256, Directory NA, static benchmark 64).
TD's opaque checked operation projection validates applicable fields and its
supported Number/iterator/URI/workspace envelopes before allowance, ledger,
input or progress ownership. It ignores unrelated Consumer NA cells and does
not certify a complete role/profile. Missing/unsupported configuration is
separate from per-input Limit; raw ResourceLimits cannot bypass the projection.
Numbers are length-checked before projection; opaque within-limit `1e309`
remains legal supplied content. Future strict ingestion checks raw and decoded
Number length before excess copy/projection. Zero disables Number admission,
not the role or feature. The numeric amendment owns exact projection semantics.

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

For the first Consumer Property Read transaction, source input is an immutable
caller loan. Physical source and persistent-document accounts have zero **new**
use for that loan. Caller allocation history is baseline; the typed-content
oracle is not a capacity census. An upstream engine source keeps its existing
source/global charge while lent, until physical release by that owner.

Controlled TD frames/current derived bytes are temporary. Planning output,
artifacts, attributable registration/erasure, slots/records and cleanup/reclaim
capacity have their actual owners/accounts. The exhaustive site/formula contract
is in the TD admission record and Planning's PlanFootprint contract. Actual
capacities, inline slot bytes, alignment, transfer/return overlap and any selected
allocator surcharge are accounted. There is no mandatory complete TD Snapshot,
exact-length TD seal, fixed arena-count prescription or copied-source footprint.

Each physical ledger reservation observes one actual checked Layout, including
old/new overlap during growth. Aggregate preflight allowances use separate logical
capacity reservations; feeding their sum into a largest-request primitive is
invalid. Current live, peak simultaneous live, largest request, allocation count,
temporary peak and additional controlled admission peak remain distinct. Released
storage does not erase historical peak. Shared startup storage is not counted
twice, and concurrent global peak sums actual simultaneous owners, not local
maxima. Inline reserved capacity belongs to owner/slot accounting, not an
invented physical allocation request.

The admission coordinator pairs local child capacity with parent/global
allowances before work/allocation, retains parents while any protected child is
live, reconciles actual owners and releases children before parents on every
failure/abandonment/success path. AdmissionLedger alone is not the global
aggregator. A rejected charge does no allocator work. Allocation ordering is:
check structure/support and lifetime work; predebit accepted work and fixed
release; check current/replacement capacity and actual Layout; obtain matching
local/parent temporary/runtime/peak/contiguous authorization; allocate; transfer
under paid progress; physically release old storage then its charge.

Complete Planning output owns every runtime fact. All input/scratch/config/compiler
loans end and bounded build cleanup finishes before final reconciliation/checks
and publication permit. No still-live owner is discharged by ending a borrow.
Published carries independently committed runtime/registration/lifecycle/cleanup
charges only. Generic source, temporary, persistent-document and runtime accounts
remain for their actual lifecycles. The already orphaned
`AdmissionLedger::reclassify_source_to_persistent_document` remains a future
source-removal obligation, not a typed publication step.

Future strict ingestion accounts all controlled parse/source state from its first
allocation, retains upstream charges while lending and proves bounded physical
release before publication. Borrowed wire buffers remain caller capacity.
This capability can supply an absolute engine-controlled input-through-build
bound; ordinary typed provisioning plus borrowing supplies only the additional
controlled-state bound. Neither is a whole-process/firmware RAM promise.

Fixed TD first-cause diagnostics allocate nothing. Variable output rollback or
compiler destruction needs an admitted nonrecursive cleanup owner and pre-reserved
capacity; it cannot be hidden in a terminal destructor. Cleanup never replaces
first cause or makes a subset published.

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
A non-incremental operation is conforming only when its maximum admitted input
is explicit and its complete work/lifetime debit succeeds before it starts.

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

`DocumentNodes` charges typed structural/Basic visits, frame transfer and
operation/query discovery not owned by a more specific class. Typed string and
Number reads/comparisons pay CodecInputBytes; owned copies pay CodecOutputBytes;
URI semantics pay UriBytes; schema visits pay JsonSchemaNodes; security work pays
SecurityBranches; each fixed block deallocation prepays CleanupItems. Iterator
primitives use the admitted backend envelope, not one unbounded native lookup.
No map canonical sort, normalization/seal or post-copy equivalence pass is required.
The TD contract owns exact applicable work formulas and numeric/URI support.

For bounded admission, JSON Number lexing, borrowed lexical inspection, and
lossless copying remain byte-charged work. When one of the five TD Basic
`serde_json::Value::Number` predicates needs a numeric projection, the lexeme
length `n` is already known and is bounded by the configured
finite `number_lexeme_bytes_max`. Before the atomic projection starts, the
cursor debits all `n` `CodecInputBytes` units from the current step budget and
the same `n` units from the shared non-resettable lifetime remainder. If the
current step budget is insufficient, the operation makes no numeric progress
and remains pending; if the lifetime remainder is insufficient, it terminates
as the existing resource limit. A repeated projection incurs the same debit
again. No new WorkClass is introduced for numeric projection.

During TD inspect/Basic/semantic lending, every accepted class-specific unit
also consumes one unit from a shared
non-resettable lifetime remainder derived from the existing
`document_validation_work_units_max`; byte classes consume one unit per byte.
Prepaid cleanup consumes both units before its allocation becomes live. Host
may drive the same pure cursor to completion synchronously, while application-
static callers resume it; fresh per-step budgets do not replace that lifetime
remainder. Exhausting it is a resource limit, not Basic invalidity. Planning
output materialization and cleanup have their own non-resettable derived work
envelopes; copying a lent fact does not debit the same TD semantic action again.

TD frame and current-byte workspace has a fixed catalog of nonrecursive,
trivially destructible blocks with prepaid release. Caller Thing destruction
remains caller-owned. Planning variable output instead uses a monotonic bounded
rollback/reclaim owner and supported bounded artifact/cursor cleanup. Abandonment
performs only fixed prepaid release or transfers the complete object to reserved
cleanup capacity; no record allocation or lost live owner is permitted on drop.

`PlanningItems` charges aggregate enumeration, row construction, lookup
sealing, reconciliation, and reclamation. A monotonic cursor visits each
admitted property, Form, plan row, and artifact a fixed number of times. The
existing step limits bound one call, the existing document/Form maxima bound
the complete admission, and `plan_reclaim_bytes_per_step_max` bounds
reclamation. No new per-plan or per-admission resource row or generated getter
is introduced for either work class.
