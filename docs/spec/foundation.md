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
