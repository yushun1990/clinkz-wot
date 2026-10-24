# Foundation: ownership, capacity and progress — v6.0

Foundation owns protocol-neutral work, capacity/reservation, generation and time
primitives. It knows no TD, protocol, handler registry or universal scheduler.
Logical time remains exactly owned by the [time amendment](../amendments/WP-100-time-domain-v1.md)
and ADR-0016. ADR-0015's borrowed immutable policy and linear budget principles
remain; a full flat policy snapshot is no longer a universal configuration ABI.

`RES-OWNER-001`: Limits MUST apply at the actual enforcing owner. Engine admission
bounds its controlled retained backing and promised handoffs. Bindings bound native
ingress/growth, concurrent operations, queues, pools/caches and retry policy.
Deployment composes process/firmware/OS/SDK containment. Every retained allocation
or caller-provided exclusive capacity has one funded physical owner; shared views
retain that owner/original account until final destruction. Actual recipient,
reference, operand and record costs are charged separately. Logical allowances
or overlapping reservations MUST NOT be presented as additive physical RAM,
contiguous allocation backing or transitive SDK maxima.

`RES-HANDOFF-001`: Capacity MUST precede each acquisition/acceptance and fund its
result/status/cleanup path. New growth may fail after acceptance only under a
declared policy preserving existing owners; admission MUST NOT promise simultaneous
worst-case success without backing. Rejection returns complete protected owners.
No truncation of required Forms, schemas, security branches, recipients or reply
rights may conceal capacity failure. Limits compose across actual local/aggregate
accounts without double-charging shared backing. Zero disables unless an explicit
bounded rendezvous policy says otherwise; omission never implies unbounded.
Diagnostics retain resource, owner/scope, limit, safely known amount and phase,
with bounded fallback. Release follows actual disposal, child before retained
parent, exactly once. Cancellation/cleanup retain capacity under saturation.

`RES-WORK-001`: Bounded jobs MUST consume linear nonduplicable step/lifetime
allowances before their work, with declared atomic envelopes or resumable
continuations. Zero budget makes no hidden progress. Required variable work,
rollback and destruction have bounded owners/cursors; phase scratch ends at the
earliest safe boundary. Borrow immutable policy instead of copying large snapshots
per hot call. User/native code is trusted cooperative code: budgets are no proof
of preemption or hard-real-time execution. Profiles prove their actual layout,
allocation overlap, largest request, retained backing and release behavior.

`RES-SCHEMA-001`: Policy identities MUST include their interpretation/schema
revision and actual complete relevant configuration; names/default ids/digests
alone confer no enforcement or permission. Renaming/reinterpreting/splitting
existing numeric fields requires explicit disposition, not silent reuse.
The unchanged [resource-limits.csv](../resource-limits.csv) is the **legacy
numeric schema for retained Property Read/Foundation profile adapters**, not
an exhaustive v6 resource architecture. Its original StrictV1 meanings/defaults
and the scoped [Consumer schema-2 overlay](../resource-contracts/consumer-ownership-v2.toml)
remain unchanged at those adapters. SDK bytes cannot be silently removed from
an old strict footprint. New capabilities use owner-scoped contracts with exact
reservation/growth/cleanup/diagnostic boundaries and compatible shared backing;
they MUST NOT inherit 196 mandatory global fields or add an operation-specific
policy framework without a concrete enforcement need. Strict profiles remain
optional stronger supported policies, requiring their own evidence.

`RES-ENV-001`: Admission capacity is not committed OS memory or guaranteed SDK
allocator recovery. Host process OOM/abort/kill/runtime loss is an environment
failure and MUST NOT become successful cleanup or release of surviving local
state. Bindings declare actual SDK/input/growth limits and lifetime assumptions;
deployment provides containment/recovery. While required owners/runtime remain
live, ordinary supported errors, cancellation and saturation follow retained
first-cause/release contracts. Stronger allocator/static-storage/hardware claims
need their own qualification. A process cap does not replace bounded ingress,
owner retention or cleanup. A Servient is not a memory-protection boundary.

## Historical adapter policy

[Consumer resource ownership](consumer-resource-ownership.md) and its encoding
vectors still define the retained Consumer adapter, with original-account and
complete-backing checks. They neither establish general native resource quotas
nor constrain future facet representations. Performance manifests retain their
historical recipes as non-gating characterization inputs. New measurements must
name owners, configuration/profile, workload, semantic obligations and revision.
