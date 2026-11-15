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

The Host finite Consumer represents these owners separately: common prepared
content retains its first backing reservations; actual data/access services and
native registration backing retain their original startup bank; each specialized
mapping retains its own compilation leases; each binding/association/call owner
funds its own reference, policy association, records, result and cleanup capacity. Reusing
content or a qualified mapping never copies, refunds or transfers those physical
leases. Retirement follows the last actual mapping/call/settlement reference.
Heap allocation measurements and reservation counters describe different facts;
temporary allowances and overlapping reservations are reported separately from
requested retained allocator bytes.

Persistent logical associations MUST NOT require prepaid idle call capacity.
In the Host finite representation, association records and selected binding
wrappers retain their explicit instance bank; `ConsumerCalls` funds its slot
storage from a separately chosen work bank. Each admitted call acquires its
native lifetime, operand/result backing, callback scratch and cleanup capacity
from that work bank before effects. Sharing content, a mapping or work storage
does not share authorization or promise native availability.

`PendingHandlerCallsGlobalMax` constrains outstanding obligations in each chosen
work domain, including retained settlements. When the association uses a distinct
instance bank, admission also acquires one outstanding-obligation quota debit
there. Both survive through final call/settlement disposal; a common bank is
charged once. The instance debit is a logical quota, not a second physical RAM
owner. A shared instance bank supplies a shared allowance, not per-Thing hard
isolation. Separate work banks compose only the limits actually shared between
them; no process-wide aggregate quota is inferred. Other per-instance byte or
rate isolation is not provided by this outstanding-call debit.
These counters follow the expressly selected banks; they do not discover or
aggregate every association sharing one `ConsumerInstance` identity.

Slot removal transfers the complete funded call to its settlement and may reuse
storage without refunding retained operands/results/cleanup. Physical input
bodies may be shared by native and Engine snapshots under the original call
reservation; copied metadata and variable records have separate allowances.
Capacity-owner loss cannot migrate these reservations to the association or a
successor bank. Captured native limits remain additional independent constraints.

The Host notification representation imports the same original data/access
service owners. A shared prepared mapping pays its actual program/schema/reference
records, without charging service backing again. Independent associations fund
only their retained records at their selected instance banks. Explicit notification
capacity reserves its Engine envelope, cleanup slots and `SubscriptionsGlobalMax`
at its work bank. Each admitted conversation additionally reserves the original
`SubscriptionsPerThingMax` quota at its instance bank and data-validation scratch
at its work bank. These subscription counters are logical capacity limits, not
additional physical RAM. A shared instance bank is a shared allowance, not inferred
per-Thing isolation or aggregation across arbitrary banks.

Conversation acknowledgement releases scratch/quota after complete local retirement;
retained report target references keep their original prepared/association funding
until final report disposal. The capacity envelope remains reserved until the
capacity owner is disposed, independently of persistent association count. Native
MQTT buffers/interests/sockets retain their Binding-owned policy; Engine reservations
are not a transitive process-memory measurement.

The managed finite Consumer owner allocates no population-sized startup backing.
Each AVL node reserves `size_of::<Node<V>>()` at the owner bank before Box
allocation; this includes its inline value, child links, height and reservation.
ID/configuration strings and identity headers have their separate original
reservations. There are no node-array capacity shifts or opaque map allocations.
Removing a node detaches its children before returning it for destruction outside
registry borrows. Original node backing lasts through payload disposal. Retained
passive snapshots keep their own funded tombstones, not other registry nodes.
Selected definitions and original call/result reservations survive independently.

For stored tree height h and query byte length L, lookup prepays
`h*(L+1)+1` QueueOperations. Structural mutation prepays
`2*(h+1)*(L+65)`: at most two height-bounded spines (search and successor
extraction), each frame's key comparison and a 64-operation envelope for link
rewiring, cached-height maintenance and at most two rotations. Actual AVL height
is logarithmic in node count; no configured maximum or population-wide movement
allowance is charged. Each actual lookup/commit pass is prepaid before effects.
Per-entry cleanup and qualified mapping/service disposal are prepaid on adoption.
This is a deterministic per-operation bound, not amortized resizing work.

Owner-assigned slot tickets cost one fixed, funded Box per allocated coordinate.
An active instance or the free list exclusively owns that ticket. Reuse and release
are constant-time moves without allocation; free backing follows actual high-water
use and remains charged until reuse or owner disposal. A staged new ticket is not
consumed and the allocation counter is not advanced if registration fails. Checked
generation exhaustion retires a ticket outside borrows. A removed instance's
original Core authority remains independently funded through its final passive
reference via `ConsumerInstance::try_new_funded`; recycling a ticket cannot refund
or mutate accepted work. Low-level standalone `new` remains caller-funded startup
construction, separate from managed allocation.

Removing a Thing also disposes its own configuration subtree under those entries'
original prepaid allowances; shutdown disposes the owner's whole population and
iteratively drains its ticket free list. Those scope-sized final disposals are
not hidden global work in an ordinary single-ID registry mutation. Owner metadata
accounts add no idle call slots or pending-call quota. Original association/work
banks retain those quotas through settlement disposal, and separate banks do not
imply an automatic per-Thing/process aggregate. No authority-change observer or
native indication allowance is introduced.

Independently composed Host finite Consumer registries/Servients share service
backing by propagating an explicit `ConsumerServiceOwner`, adopted once at the
composition root. Imports retain the original reservations; they neither move
them to the importing bank nor acquire another backing reservation there. Data
and access owners can originate at different banks. Each registry separately
funds its records/references and actual native registrations; each instance and
accepted interaction retains its own existing accounts. Registry-only records
retire with the registry, while retained services/native owners survive through
their final mapping or normal/late settlement disposal. Owner retention grants
no permission, credential, policy revision or logical authority.

Raw-service startup adoption matches aliases only within one construction.
Independently adopting a raw alias again is outside the supported shared-owner
composition contract: neither equivalent configuration nor an independent
reservation proves another physical allocation. Composition roots must carry
the original retained owner. Concrete services still declare and enforce their
own backing, including any internally shared dependencies; this mechanism does
not discover shared suballocations or manage Binding-native infrastructure.

Prepared structured constraints add bounded owned field vectors and names to
scalar data roles as well as collection members. A source census prepays their
variable construction/destruction; retained content charges actual backing, not
just a fixed enum size. Reused views keep the first owner and do not clone the
tree during calls. HTTP configuration snapshots and accepted native programs
retain their immutable scalar schema through an Arc in the concrete Host
participant; this is not an Arc requirement in Core's semantic model. Codec
scratch/work and instance/call/result allowances remain distinct obligations.
The external owned-compiler fixture declares artifact bounds using the new
actual schema/member layout rather than preserving its former 512-byte group
artifact assumption.

Continuing Action preparation also prepays the shared borrowed schema census
and construction/destruction bounds. One original Consumer definition owns its
input/output constraint backing across native operation selection and accepted
invocations. Concrete HTTP Producer definition clones and captured validators
retain immutable schema owners; their variable backing and support-probe scratch
are included in the participant declarations. These data owners neither fund nor
replace independent native exchanges, continuing work, result or cleanup owners.

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
