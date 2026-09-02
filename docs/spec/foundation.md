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

For the first v5.1 Consumer Property Read aggregate, the same owned typed
`Thing` is both the validated admission input and the retained source view.
Servient initially charges a conservative source envelope using the existing
`retained_source_bytes_*`, document, peak-live, and largest-contiguous limits.
After TD-owned validation and representation-aware census, one narrow checked
`AdmissionLedger` operation reclassifies the same live bytes from the source
account to persistent-document accounting. The operation checks destination
capacity before changing either account; success changes neither total live
bytes, peak-live bytes, nor largest-contiguous allocation, and failure leaves
the source charge and owned value intact for rollback. It is an ownership-
preserving account transfer, not a second reservation or a cloned document.

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

`DocumentNodes` charges only typed-document validation and census visits not
already owned by a more specific class. Typed schema-node visits remain
`JsonSchemaNodes`, URI-template bytes remain `UriBytes`, and security branches
remain `SecurityBranches`; work is neither relabelled nor double charged merely
because it occurs during validation. One validation owns a non-resettable
lifetime remainder derived from the existing
`document_validation_work_units_max`. Host may drive the same pure cursor to
completion synchronously, while application-static callers may resume it;
fresh per-step budgets do not replace that lifetime remainder.

`PlanningItems` charges aggregate enumeration, row construction, lookup
sealing, reconciliation, and reclamation. A monotonic cursor visits each
admitted property, Form, plan row, and artifact a fixed number of times. The
existing step limits bound one call, the existing document/Form maxima bound
the complete admission, and `plan_reclaim_bytes_per_step_max` bounds
reclamation. No new per-plan or per-admission resource row or generated getter
is introduced for either work class.
