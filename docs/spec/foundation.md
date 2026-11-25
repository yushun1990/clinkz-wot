# Foundation — ownership, capacity and bounded progress

Foundation supplies generation and monotonic time primitives. Resource limits belong
to the owner that can enforce them. Host and caller-owned deployments use the same
rules for acceptance, effects, handoff and retirement. They can supply different
storage, clocks and drivers. [Time domain](../amendments/WP-100-time-domain-v1.md) ·
[Architecture decision](../ADRs/0025-semantic-engine-and-binding-runtimes.org)

`RES-OWNER-001`: Each owner MUST bound growth where it occurs. Servient bounds
candidates, instances, calls, continuations, queued deliveries, draining work and
held results. Bound both item count and backing bytes where either can exhaust the
owner. A fixed number of slots of bounded shape can establish both. TD, Planning,
codecs, access services and Discovery bound the input, expansion, intermediate
storage and populations they control. Bindings own native buffers, ingress, retry
and last-user release. An idle mapping need not occupy a call slot.

A loan of usable storage excludes competing use for its lifetime. A shared
immutable allocation has one backing owner through its last reference. Each
association, recipient and queued record still needs space. Dynamic growth checks
its bound before allocation and releases provisional claims if construction fails.
A byte debit alone cannot supply the promised storage. Measured heap, overlapping
logical allowances and native or kernel memory describe different quantities.

**Shared ceilings.** A deployment offering an aggregate limit MUST identify its
participants, the unit they compete for, its sharing policy and release point.
A shared pool or a move-only permit from the quota owner conserves the last unit
across concurrent and reentrant admissions. A permit can enforce a tenant limit
in addition to an exclusive native slot. Fixed partitions are valid only if their
sum respects the ceiling and the policy permits unused capacity to stay in one
partition. Independent pools imply only their local limits.

During migration, every counted holder, including an old result taken but still
held by its caller, keeps its unit through the promised release point. Coexisting
paths draw on the same remainder or account for each live claim in exactly one
permitted partition. To replace the pool outright, fence all old admissions and
drain every counted holder or transfer the outstanding claims and release duties
exactly once before opening the new capacity. A deployment can retire an obsolete
ceiling with its configuration and checks. Accepted work still owns its backing.

`RES-HANDOFF-001`: Before acquiring a reversible resource, its owner MUST
secure the means to undo that acquisition and return the protected inputs.
Acquire later resources when needed, including scratch that must coexist with
durable output. Before consuming an exclusive offer or entering code that can
cause an effect, secure the input or its loan, any owed reply or continuing
conversation, the promised outcome and its handoff, a bounded failure record,
and local retirement.
Backing can be inline, lent, previously reserved or acquired at this boundary.
After acceptance these obligations must remain serviceable when ordinary pools
fill. A status cell suffices only for an outcome that owes no value. Each owner
releases its storage or permit at its own last use.

A deployment promising *advance availability* names the supplying owner, covered
capacity and range of selections, the point the promise starts and its release.
Set aside usable backing at every owner needed for that promise, including
temporary overlap. If final members are unknown, reserve interchangeable slots
from their common pool or hold the needed capacity at every possible private
owner. A Servient record cannot guarantee an opaque SDK or an independent
route. Move the reserved capacity to its ordinary owner at each stage and
return surplus. Refuse an unbackable promise before native acquisition. Choosing
ordinary admission is explicit. [Publication](runtime-safety.md#producer-publication)
and [result handoff](interaction-core.md#finite-execution-and-ownership-representations)
define the corresponding lifecycle boundaries.

`RES-WORK-001`: A public drive MUST bound the work until it returns. Use a
bounded native or handler poll, a byte slice, or a cursor over a bounded number
of records or nodes. Count synchronous callbacks and reentry into siblings in
that bound. Split, restrict or decline callbacks whose inclusive cost exceeds
the supported turn. A passive observation or zero-sized batch invokes no linked
code. On yield keep the cursor and the original owners. Distinguish advancement
or a runnable yield, waiting for an identified event, exhausted job limits and
terminal outcomes. Host and manual entry points take the operation they can
drive without a general `WorkBudget` or per-member credit factory.

Input size, fan-out and a forward-only algorithm may already bound a whole
job. Where repeated probes, fallback, nested scans or rewind could exceed its
promised total, the owning job enforces a nonrenewable allowance. Debit before
the visit or callback. Delegates draw from the same remaining balance across
resumption, rewind and alternatives. Exhaustion ends that job. A long-lived
conversation bounds each new delivery under its queue and overflow policy.
Closing existing work has its own bounded path under admission pressure.

`RES-SCHEMA-001`: Each enforcing owner MUST define the units, finite defaults,
zero/omission behavior and failure of its applicable limits. An omitted setting
cannot silently admit unbounded growth. A prepared mapping binds its actual
relevant configuration, semantics and provider identity. Rebuild or refuse it
when a change invalidates that interpretation. A deployment can compose its
resource settings. A name or digest alone proves neither compatibility nor
current permission. Report the enforcing scope, phase,
limit and safely known amount, retry disposition and effect certainty. Provide a
bounded fallback when diagnostic storage is full.

The [numeric profiles](../resource-limits.csv) and [schema-2 Read format](consumer-resource-ownership.md)
describe callers still using their original interpretation. Migrate the source
and prepared-only Read entries, then delete the overlay, generator, frozen
digests and schema-wide checks with their last consumers. Keep benchmark inputs
with their workload. The target has no exhaustive public limit schema, generic
admission receipt or registration-wide footprint declaration.

## Host allocation and environment failure scope

`RES-ENV-001`: Ordinary saturation, cancellation and supported allocation failure
MUST preserve first cause, protected owners and release. Before acceptance return
original inputs, as soon as any rollback loan ends. After acceptance preserve late
evidence and retire in place or transfer to an available, acknowledging owner.
Process abort, a stopped driver and an opaque SDK have their own failure domains.
Allocator recovery, hard RAM, latency and hardware claims require evidence for the
actual target, including stack, allocator, callbacks, interrupts and native
infrastructure where applicable. A deadline on the supplied monotonic clock
fences an expired obligation when observed. Cooperative callbacks run until
they return.
