# Resource ownership and admission in the WoT Engine

**Status** MIGRATED to the architecture contracts. The investigation and measurements
below describe the assessed revisions. [Decision](../docs/ADRs/0025-semantic-engine-and-binding-runtimes.org) ·
[Owner-enforced limits](../docs/spec/foundation.md) · [Migration sequence](../PLAN.md#engineering-priorities-and-dependencies)

**Recommendation.** Use one publication and execution lifecycle per capability on Host and
caller-owned deployments. Before accepting an obligation, secure its actual result and
cleanup path. Admit reversible setup in stages. A deployment can guarantee specified local
capacity by supplying the same owners' storage earlier. Make that promise only where the
deployment can back it. Remove the general `WorkBudget` from execution, including manual
driving. Give each driver bounded operations it can call and keep counters at the few owners
that must enforce a cumulative limit. Retire the flat resource schema and its compatibility
APIs as their callers migrate.

Under the proposed default, a Host can acquire an assigned socket and subsequently lose the
race for a final member slot. The candidate holds the endpoint with ingress closed until
rollback releases it. The caller recovers its inputs and a coexisting incumbent keeps
serving. After an exchange is accepted, its space to settle survives even if another caller
fills the result queue. Keep #308 paused and rebuild the exposure path on this distinction
after contract review. The comparison uses master
`fadb3aa6714c5faa20ba74e533f7a0aa8386e339` and #308
`733e4d708b51c3db1b8357899eb93b5485a11bf4`.
[PR #308](https://github.com/yushun1990/clinkz-wot/pull/308) ·
[Current publication contract](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/runtime-safety.md#L513-L526)

## Guarantees visible to the application

A Consumer uses a Form to perform an operation. A Producer advertises how to reach its
Properties, Actions, Events and described collections. A TD can be prepared elsewhere and a
Binding can implement a single opportunity on a small device or multiplex many over a shared
service. The W3C Recommendations specify interaction meaning, Forms and URI resolution.
ClinkZ chooses how to bound its own memory, work, fan-out, cleanup and diagnostics on each
supported deployment. Publishing a TD commits the Producer to the advertised support.
Concurrent call capacity and scheduling are deployment decisions.
[WoT Architecture 1.1, protocol bindings](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/#sec-protocol-bindings)
· [TD 1.1, Forms](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#form) ·
[ClinkZ product goals](../docs/architecture/00-system-goals-and-context.md)

Core defines the shared meaning of identity, authorized effects, data and outcomes.
Servient decides when a particular instance may act and when it has accepted an obligation.
Foundation supplies time and generation primitives. A permit belongs to the pool whose
capacity it excludes from other users.
The following boundaries apply to both roles wherever they offer the interaction.

| Obligation | Who can enforce it, and where the obligation ends |
| --- | --- |
| Bound work and growth caused by input | Discovery caps bytes, pages, results and elapsed time across a retrieval. TD, Planning, codecs and access services bound their own decoding, depth, expansion, provider search and intermediate storage. A too-large description or unsupported security branch produces an explicit refusal. All advertised Producer branches must qualify. Consumer choice may stop at a complete viable interaction. |
| Bound Engine-owned populations | Servient limits simultaneous candidates, instances, calls, invocations, conversations, queued deliveries, draining work and held results in their respective domains. Variable-size records need a byte or shape bound as well as a count. Shared immutable definitions have one backing owner through last use. Each mapping or recipient still needs space for its own record. An idle Thing needs authority and a mapping. Call storage can come from a shared pool on demand. |
| Preserve the right to reply | Before accepting exclusive ingress, Servient secures the original instance, current permission, behavior, deadline and space for the promised outcome. The Binding moves the reply right, lends an exclusive native slot, or copies into admitted storage under its own delivery rules. Refusal returns the complete offer to its original owner, including its deadline. An active slot can be reused once its exchange retires. Any unread or caller-held data keeps its original backing or loan through disposal. |
| Keep continuing work distinct from individual exchanges | Before Invoke or subscription setup can create continuing work, Servient secures the continuation and the Binding secures the means to resolve later controls. Both survive retirement of the first reply for the promised lifetime. Each subsequent exchange, delivery and queued recipient has its own lifetime. The Binding owns native matching, fan-out and acknowledgements. Collections retain their membership and result semantics. Apply overflow or visible loss only where the interaction permits it. |
| Cancel, replace and shut down truthfully | Servient fences the original generation immediately on revocation, records the first cause and retains late output and effect uncertainty. A Binding retires the exact exchange or participation, and owns final release of a shared listener, session or writer. A whole-owner transfer needs an accepting destination. A bounded diagnostic can survive an uncertain remote effect. Surviving local work still needs an owner. |

An arena slot or owned buffer supplies **backing**. A debit on a shared tenant account
supplies **exclusion** against other users of that account. A cursor bounds **how far one
drive may run** when its next step has a known maximum cost. A job can also cap the **total
work across drives**. The owner refunds a quota debit at last use. Spent work stays spent.
Kernel buffers, an opaque application's heap and elapsed CPU time have separate owners and
require separate evidence. [Foundation](../docs/spec/foundation.md) ·
[Binding delivery and retirement](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/binding-spi.md#L62-L97)

## What master and #308 actually enforce

On master, Foundation offers checked reservations and a non-copyable `WorkBudget` with 12
work classes. A Host bank has limit, usage, peak and identity data indexed by the 196
resource kinds. Its atomic check and move-only receipt can enforce a real shared ceiling
even when admissions race. Reserving bytes changes counters. The backing allocation or
exclusive loan is a separate operation. ADR-0025 already assigns native resources to
Bindings and treats the flat schema as a legacy adapter. ADR-0023 is superseded. The
remaining coupling lies in the specific handoff and work rules imposed on those owners.
[Foundation budget](../foundation/src/budget.rs) ·
[Host bank and concurrent reservation test](../core/src/host_accounts.rs) ·
[ADR-0015](../docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org) ·
[ADR-0025](../docs/ADRs/0025-semantic-engine-and-binding-runtimes.org)

- **Finite Producer.** The optional shared capacity acquires six debits and boxes a
  settlement before native receive. At terminal native settlement it returns active capacity
  and the ingress debit. A held result continues to occupy a retained slot, byte and cleanup
  allowances, and handler/response quotas. The shared-listener tests demonstrate that a busy
  receiving Thing must leave sufficient progress for the sibling holding the original reply.
  The behavior worth carrying forward is the independent release of reply and result,
  including after replacement. Six receipts and holding an entire input allowance through
  result disposal are costs of this representation.
  [Capacity and release](../servient/finite/src/producer_capacity.rs) ·
  [Real HTTP saturation and replacement](../protocol-bindings/protocols/http/tests/composed_thing/capacity.rs)
- **Consumer and continuing work.** Managed associations and call pools can have different
  banks. A caller-owned Read already has an exclusive slot, a typed native call and a result
  that lends its bytes, yet its entry also requires a byte receipt matching the instance
  coordinate. The common finite driver gates native cancellation behind access work. The
  continuing Action driver prepays access, native, cleanup and validation together before a
  close can run. The incremental TD admission walk shows where metering earns its cost. It
  charges variable input inspection and can resume a cursor.
  [Consumer ownership](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/foundation.md#L30-L55) ·
  [Borrowed Read](../servient/finite/src/borrowed_consumer.rs) ·
  [Finite driver](../core/src/execution.rs) ·
  [Action driver](../core/src/execution/action.rs) · [TD cursor](../td/src/validated.rs)
- **Other owners.** MQTT's QoS 0 writer becomes available after a complete packet even if
  the emission outcome is held. Abandoning a partial frame can damage the shared connection.
  Servient currently also revokes publication permission on an individual emission cleanup
  error. Scope the fence to the resource actually lost. Discovery's page, TD, total-byte and
  session limits illustrate enforceable service-local bounds. A previous 10,000-instance
  Consumer workload separately measured account provisioning at 62.973 MB with independent
  domains and 0.019 MB with a common domain at one call slot per instance. Sharing changes
  the isolation promise. Those measurements characterize that historical deployment.
  [MQTT native writer](../protocol-bindings/protocols/mqtt/src/native.rs) ·
  [Emission retirement](../servient/finite/src/notification_publication.rs) ·
  [Discovery limits](../discovery/src/http/mod.rs) ·
  [Consumer scaling measurements](../docs/evidence/consumer-multi-instance-scaling.md#idle-active-and-physical-storage)

On master, managed publication starts with an application-assembled instance, prepared
definitions and native participants. #308 brings that assembly behind registered services.
It validates a draft, acquires a closed endpoint, resolves the actual base, revisits
eligible providers including earlier losers, and feeds the original members into the
composed publication decision. Its HTTP and independent UDP cases show why an assigned
socket must survive a short turn and why a second eligible final owner requires refusal.
Fixed listeners can serve disjoint Things.
[Master composition](../servient/src/producer_composition.rs) ·
[#308 exposure path](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/servient/src/host_producer.rs#L643-L790)
· [#308 wire tests](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/tests/producer_exposure.rs)

The PR also reserves candidate live and cleanup bytes, qualification scratch, a Thing slot,
readiness slots and cleanup items before acquisition. The possible final member count
depends on draft operations, registrations and ceilings. Selected members release the
surplus. Both revisions include an 8 MiB term (`16,384 × 512`) in the qualification scratch
estimate before the per-operation limit and provider costs. The scratch debits both
temporary and Engine live bytes. Commit transfers serving funding on the original bank. Two
candidates can complete their admitted local work under subsequent bank pressure. This
guarantee of **local capacity under contention** has a cost even when neither candidate uses
its worst-case allowance.
[Scratch calculation](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/servient/src/producer_composition.rs#L1042-L1066)
· [Member and storage funding](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/servient/src/producer_composition.rs#L447-L535)
· [Qualification funding](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/servient/src/producer.rs#L47-L66)
· [Pressure test](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/tests/producer_exposure.rs#L497-L594)

I measured two simultaneous candidates with one relative HTTP Read each on #308. Each used
its own assigned loopback socket on one Host and bank. The baseline included the Host, TD
fixture and registration. Numbers are byte deltas from that baseline, with separate columns
for the overlapping account debits. The last two columns measure process allocator request
sizes on x86-64 Linux, Rust 1.95.0, debug build.

| State | Candidate allowance | Engine live debit | Cleanup byte debit | Temporary debit | Requested heap held | Requested heap peak during staging |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Two staged | 64 KiB each | 34,932,704 | 131,072 | 34,801,632 | 11,744 | 45,582 |
| Two staged | 128 KiB each | 35,194,848 | 262,144 | 34,932,704 | 11,744 | 45,582 |
| One published, second retired | Either | 6,130 | 6,130 | 0 | 4,992 | — |

The surviving Thing served value 7. Its result outlived the Host and the bank returned to
zero on disposal. With the Engine live-byte ceiling set to 35,000,000 on the same workload,
both 64 KiB candidates staged. At 128 KiB the second returned backpressure and the first
still served. The selected allowance therefore changed which work could proceed. This
workload measures requested heap and local account pressure. Worst-case input, allocator
overhead, socket memory, throughput and device fit require different evidence.
[Fixture and provisioning](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/tests/producer_exposure.rs#L19-L58)
· [Allocator meter](../protocol-bindings/protocols/http/examples/multi_instance/meter.rs)

## Alternatives and their costs

| Architecture | Where it earns its cost | What it gives up |
| --- | --- | --- |
| Prepay all possible local completion paths before setup | Local capacity is predictable after admission. Two contenders can finish under pressure. Appropriate for fixed-capacity firmware or a tenant sold a guaranteed reservation, provided the backing actually exists. | The Host predicts future membership and opaque callback costs. A feasible Thing can be refused because a different final shape would need more cleanup. Every new phase or Binding can change the prediction. A receipt alone cannot make allocating the promised buffer succeed. |
| Rely on fallible allocation, RAII and the runtime | Straightforward for purely local, reversible scratch. Moving a socket guard or borrowing a slice can establish unique ownership directly. | Many separately valid calls can exhaust shared capacity. A full queue after a handler changed a device leaves nowhere for its result. Dropping a future cannot acknowledge protocol cleanup or give a native callback back its reply right. Process limits can terminate a deployment without settling the interaction. |
| Require fixed pools and one bounded step everywhere | Direct evidence of capacity, predictable local admission and small caller-controlled turns on a constrained target. | Variable TD preparation, many idle Things and independent Host Bindings would need worst-case partitions or a common allocator/scheduler. One step can still hide arbitrary work inside a callback. Static storage is an excellent deployment choice with a finite maximum item size. |
| **Select owner-enforced limits with explicit handoffs** | A bounded queue, a loan, a preallocated result cell or a shared permit each proves the fact its owner controls. A deployment can combine them for aggregate tenant limits or guaranteed local completion. | Ordinary Host setup can fail later under contention. The SPI must distinguish pending work from a consumed right and document cleanup and callback bounds. Static fragmentation, fairness and aggregate isolation still require deployment choices. |

A Binding-only solution would miss the Engine's own population of candidates, logical
recipients and retained results. Per-call size limits alone would admit arbitrarily many
individually small items. An Engine-wide allocator would miss the kernel and
nonparticipating SDKs. Keep aggregate admission where callers can contend for an actual pool
or quota. For eight outstanding calls across two Bindings, a shared eight-slot pool or
eight permits at the tenant can let either Binding take the last place. Fixed partitions
also enforce the ceiling but leave unused capacity in one partition unavailable to the
other. The deployment chooses according to its advertised sharing policy. Each capacity
right can be taken only once, including by concurrent or reentrant callers.
[Architectural discussion](https://github.com/yushun1990/clinkz-wot/pull/309#issuecomment-7156870761)

## How the recommended design works

On Host, bound each dynamic container where it grows. A pool of at most N entries with a
checked maximum size per entry gives a calculable envelope. If entries draw variable backing
from one shared ceiling, acquire the extra capacity before growing, use fallible
construction where supported, and return the provisional debit if construction fails. A
shared allocation has one physical owner through its last reference. A tenant debit may
legitimately follow the result for longer than its active call. Keep the bank only where it
enforces such a policy. Counters for diagnostics can be sampled separately from admission.
Use a fixed-size fallback when even error reporting is saturated.

### One publication path, including advance reservations

Consider two candidates for an assigned address. One draft registration can own both Forms,
yet the final address can place them on different members. Servient has one job in either
deployment. Keep ingress closed, resolve the acquired base, qualify every final Form against
the current eligible registrations and actual behaviors, stage the selected members, then
commit or roll back. Capacity never selects a cheaper Binding or removes a competitor from
qualification. [Split-membership case](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/tests/producer_exposure.rs#L602-L696)

| Point in that path | Ordinary admission | Deployment promising local member capacity before taking the base |
| --- | --- | --- |
| Before acquiring the closed endpoint | Secure one bounded candidate and the capacity to release the base and return the inputs. | Also set aside actual member slots and their cleanup storage for the promised range of final selections. Bind each reservation to its pool and candidate. Obtain the whole set or unwind before making the promise. |
| When the exact members are known | Take each selected slot from its owner with its rollback path. If the set cannot be acquired, unwind without waiting on another staged candidate to surrender a slot. | Move the selected slots already held into those same member owners. Return the surplus. A competitor may exhaust the free pool between the two stages. The covered candidate needs no second acquisition from it. |
| Readiness, publication, dispatch and retirement | Both follow the same path from here. | The same member and reply handles enter the same checks, local commit, cancellation, result and retirement operations. Pool provenance follows each handle through last use. |

The promise must name **which capacity, from which point, until which release**. Reserving
two Servient records can guarantee room for two members. Guaranteeing *all* subsequent
local capacity would also require actual usable backing for final qualification, temporary
overlap, native assembly, retained results to the extent promised, and cleanup. If the final
member is still unknown, interchangeable slots can come from one bounded pool. Slots private
to different Bindings must be secured from each possible owner or guaranteed by a concrete
partition at deployment. A byte receipt cannot reserve a route from an independent service.
If an opaque SDK, fallible late allocation or unknown final shape defeats the requested
guarantee, refuse it before acquisition. Proceed under ordinary admission only by explicit
choice. A narrower guarantee may become possible once final membership is known.
[What #308 actually prepays](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/servient/src/producer_composition.rs#L447-L535)

Even a fully provisioned candidate can lose authorization, collide on a route, observe a
different configuration or lose its native service. After the last callback, both deployments
verify the expected incumbent, exact members, behaviors, coverage and readiness together.
The bounded local commit publishes the TD and opens the original gates. Short progress keeps
the candidate and its endpoint. Cancellation, permanent refusal or observed expiry fences
it and starts rollback. An exclusive route may require a service gap. A visible native
effect requires a truthful residual outcome or refusal of the mapping.
[Publication coverage](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/planning.md#L107-L152) · [Replacement policy](../PLAN.md)

The same rule applies to Consumer results. An ordinary pool can supply the result cell when
a call is admitted. Firmware can dedicate that cell at construction. The exchange owns the
same result storage from acceptance until disposal under either policy. Dedicating it early
can refuse a competing call that would otherwise fit. The benefit is availability for the
named caller when it needs the cell.

**Decision for the first successor.** Ship ordinary bounded admission and the ability to
supply concrete storage. A firmware partition or Host deployment can use those same
operations with slots it already owns. Introduce advance admission at a particular owner
only for a stated availability requirement. Its extra logic ends when the reservation is
moved into the pending operation. If meeting the requirement needs a second publication
algorithm, a strict flag in dispatch/cleanup, or resource callbacks on unrelated Bindings,
decline that guarantee for the configuration. A process-wide strict mode has no identified
v1 consumer. The pressure test in #308 establishes protection from competing *account*
debits for its case. An allocator or socket availability guarantee needs its own evidence.
[Two-candidate pressure test](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/tests/producer_exposure.rs#L497-L594)

### Accepting and retiring work

Every supported deployment must secure what the Engine owes before the first effect that
can create the obligation. This includes the input or loan, reply right, promised outcome
and handoff, minimal failure record and a way to retire the work. A status cell suffices
only for a failure whose contract owes no data. Recheck original permission after
reentrant callbacks and before the effect. Keep linked code and destructors outside
registry locks.

**Continuation identity.** Before entering an Invoke handler that can start work, Servient
claims the invocation slot for the original instance and generation. The Binding secures
the means to resolve the promised later controls. It can derive a generation-scoped ID
from the pinned invocation, hold exclusive use of its native table, or reserve a record
and move it into the mapping. The owners exclude competing and reentrant use through
handler entry and installation of the ID. Installing the correlation after entry must
require no further capacity acquisition. If either owner is short, refuse before starting
the handler and release any provisional claim. An admitted ID resolves to the original
invocation for its advertised lifetime, with fresh authorization on Query and Cancel.
If the handler starts work and the first reply fails, the continuation and cleanup retain
their owners.

Exercise a full Binding correlation table with a free reply slot and verify zero handler
calls. Then contend for the last correlation slot from a callback and query or cancel the
admitted invocation by its original wire ID after the first reply retires.
[Servient invocation claim](../servient/finite/src/action.rs#L935-L978) ·
[HTTP correlation](../protocol-bindings/protocols/http/src/action_producer.rs#L496-L513) ·
[Continuation review](https://github.com/yushun1990/clinkz-wot/pull/309#discussion_r4690985983)

**Result handoff under saturation.** Qualify what the mapping owes. A raw-evidence
interface retains the original bytes and outcome under its access policy. A validated
handoff also owes a still-authorized recipient a way to obtain a valid value while
ordinary pools are full. At acceptance secure the overlap of the held result with any
workspace needed to decode, validate and lend or copy it. An allocation-free codec or
one settlement-only scratch area can suffice. If settlements share it, each driven
handoff must get a turn and release the scratch independently of how long the application
holds the result. Reuse prior validation only while it covers the exact value and schema
being handed over. Recheck current permission. Offer raw retention as the sole result
guarantee only where the interaction permits it.

For a 514-byte JSON body, the review measured zero additional heap bytes for a type check
and a 512-byte peak for a string-length check. In the successor, hold valid results, deny
ordinary allocations after acceptance and retrieve a length-constrained value while the
other results remain held. Invalid data or revoked permission must still prevent successful
handoff. [JSON length check](../codecs/json/src/lib.rs#L244-L248) ·
[HTTP validation workspace](../protocol-bindings/protocols/http/src/producer.rs#L222-L245) ·
[Measurement](https://github.com/yushun1990/clinkz-wot/pull/309#discussion_r4690985990)

A handle can become its own cleanup record in place. An asynchronous transfer needs an
already available destination that accepts the whole owner. Supported allocation failure
before acceptance returns the original input. If rollback still borrows a unique input, its
owner returns it as soon as that borrow ends. After an ambiguous network effect, record
the uncertainty and first cause, retaining an existing output through validation or
disposal. Release only the retired exchange. Fence dependent participations if the native
service itself is lost. On root loss use finite local disposal or acknowledged transfer to a
longer-lived owner. Explicit close reports local drain and any unresolved remote effect.
Process abort or a stopped driver cannot promise eventual completion.
[Runtime cleanup and Drop](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/runtime-safety.md#L535-L574) ·
[Native delivery/retirement](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/binding-spi.md#L62-L97)

### Which work needs a meter

Remove the twelve-class vector from bounds passed among Core, Servient, Bindings, handlers
and codecs. Moving it into the Host loop would leave the same declarations, prepayment and
cross-phase dependencies in the implementation. A single Engine-wide fuel counter would
still need a defensible exchange rate between a provider call, a schema visit and a wire
byte. Use the least mechanism that enforces an observable bound at the owner doing the work.

| Existing work classes | Target mechanism and reason |
| --- | --- |
| `QueueOperations`, `HandlerSteps`, `BindingPolls`, `CleanupItems` | A bounded queue insertion, handler/native poll, gate change or fixed-slot disposal needs no debit. Bound the actual traversal and include callbacks and synchronous re-entry in the cost of one public drive. For variable destruction or bulk retirement, keep a cursor over the remaining owners and release each on its last use. Secure cleanup storage at admission. |
| `CodecInputBytes`, `CodecOutputBytes`, `UriBytes` | Enforce actual input, expanded output and retained byte limits at the codec or URI owner. A small bounded value can be processed in one call. For a stream or expansion too large for one turn, process a bounded slice and retain an offset. A concrete Binding may cap bytes written per poll, as MQTT does. Byte limits alone do not bound repeated schema comparisons or provider calls. |
| `DocumentNodes`, `JsonSchemaNodes`, `PlanningItems`, `SecurityBranches`, `ProviderProbes` | Bound depth, alternatives and candidate sets at admission. Walk a bounded number of nodes or candidates per step. If the finite input bounds and algorithm already bound total work acceptably, the cursor position suffices. Rewinds, nested searches, fallback or repeated probes that can exceed the supported total need a nonrenewable allowance at the owning job, charged before the visit or callback. A permission check on one fixed target still runs at the effect boundary. |

Three examples determine the boundary. A registry lookup can be bounded by maximum key
length and tree height. Replaying its worst-case comparison/rotation count into a public
queue budget adds no protection if that bound fits a turn. A Binding can cap the bytes and
attempts in one native poll using its private state. The TD cursor, however, can rewind and
repeat a semantic pass with identical input. Its total allowance must survive that rewind
and new turns. The Linux TD admission fixture exercises the exact lifetime limit and refusal
one unit below it. Discovery likewise has meaningful session-wide byte/page limits, while
its current admission driver also sums twelve credit balances to detect progress. Have the
cursor report advancement, completion or its own exhausted limit directly. A consumer of
the cursor can then decide when to let the next step run.
[Registry work formulas](../servient/finite/src/consumer_tree.rs#L23-L45) ·
[MQTT write chunk](../protocol-bindings/protocols/mqtt/src/native.rs#L950-L976) ·
[TD rewind boundary](../td/tests/runtime/src/scenarios.rs#L896-L963) ·
[Discovery admission loop](../discovery/src/http/admission.rs#L78-L134)

Use a second local counter only if exhausting it independently changes a promised outcome,
for example a Directory session's byte limit and page limit. A finite job that delegates
parts of one allowance must debit the parent or share its remaining balance. Starting a
fresh turn, trying another registration or reentering a handler cannot mint more of it.
Exhaustion ends that search. Previously accepted results and native cleanup stay with
their owners and keep their own progress path.
Long-lived subscriptions renew work per bounded delivery under their queue and overflow
policy. The supplied monotonic clock fences expired obligations when observed. It cannot preempt a synchronous callback.
A deployment must restrict, break up or decline a callback whose worst supported input
would exceed its promised turn bound.

**Cleanup is a different phase.** On master I cancelled a caller-owned Consumer Action
before its first UDP send. With all other classes funded, independently withholding provider
credit or codec-input credit left its original socket bound and made zero native close
calls. Restoring credit closed that socket once, with zero send polls and no further
validation. In the existing Producer Action control, cleanup alone closes the work even
after its validator declaration changes. In the target, a bounded close poll can advance
under ordinary saturation. A pending close keeps the native right. A racing output keeps
any backing or loan it requires through disposal. Delivering that output still requires
its own validation and fresh permission. A mapped remote Cancel remains an
independently authorized effect.
[Consumer prepayment](../core/src/execution/action.rs#L195-L211) ·
[Public caller-owned cancellation control](../tools/architecture-fixtures/constrained-consumer/src/action/tests.rs#L1033-L1046) ·
[Producer cleanup control](../servient/finite/src/action/validation_work_tests.rs#L232-L250)

Manual driving means the caller can perform one bounded step and stop. Observing status or
a recorded fence invokes no linked callback. A variable-size operation may accept a batch
count in its own units. Zero is passive. Retain the cursor on yield. Divide or refuse a
step too large for every supported turn. A Host driver can rotate through ready owners and
give draining work a turn under admission pressure. A caller-owned loop supplies its own
clock and progress.
[Host Consumer turns](https://github.com/yushun1990/clinkz-wot/blob/0e16c323549873d2dc19f7d9ade8bad10e19ef9c/servient/finite/src/consumer_execution.rs#L12-L52) ·
[Producer driving](../servient/src/producer_execution.rs#L22-L49)

**Reentrant drives.** A public turn bound covers all synchronous work until the outer call
returns, including callbacks that drive independently admitted siblings. In the review,
one outer step ran three native polls across three one-slot owners. For a one-poll turn,
keep the right to drive at the deployment's outer entry. A nested request can mark the
target for a later turn and return without invoking another driver. Cancellation and
revocation fence further local effects immediately, including on checked-out work. Drain can
follow. This rule must cover every drive reachable from a callback in that deployment.
A busy flag on just the first owner leaves sibling entry open. A deployment that permits
synchronous nesting must qualify its inclusive bound on depth, fan-out and callback work.
Offer a bounded turn only where the deployment can enforce one of these rules.

Exercise a callback chain through three independent owners. With deferral, a one-poll
outer turn must return after one native poll, a nested cancellation must already be
visible, and the driver must give a ready drain its promised turn. With qualified nesting,
exercise the stated maximum and defer or refuse work beyond it. The driving adapter owns
this behavior on both Host and caller-owned deployments.
[Sibling re-entry](../protocol-bindings/protocols/http/tests/consumer_callbacks.rs#L933-L952) ·
[Three-owner control](https://github.com/yushun1990/clinkz-wot/pull/309#discussion_r4690985993)

### Host, caller-owned execution and the Binding SPI

A Host configures pool sizes and overload policy when building the service. An application
calls `expose(td, behaviors, replacement)` or a named Consumer interaction. It gets a pending
handle, success, or a terminal refusal with a way to recover its inputs. Pending identifies
who will retry and which local turn, native readiness event, timer or pool release can
enable progress. The same handle supports cancellation, close and result retrieval.
Terminal failure on the owning Host is final even if physical rollback drains. A foreign
Host leaves the candidate intact. Retry after an uncertain effect is an application
decision. Report the enforcing scope, phase and amount when known, and distinguish
transient contention from a permanently unsupported input or guarantee.
[#308 application example](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/examples/serve_properties.rs#L105-L205)

**Waiting for capacity.** If an event-driven Host parks on a full pool, that pool can
register bounded interest and recheck availability before the driver sleeps. Publish the
released slot before signalling the waiter. A release racing registration must either
make the recheck succeed or wake the driver. A wake tells the driver to retry ordinary
admission with the original deadline and authority. Another contender may win the slot.
A maintenance-driven deployment can instead promise independent retry turns, including
in a caller-owned loop. Before acceptance the owner can return explicit backpressure
with the intact offer and assign retry to its caller.
A Binding that keeps an unconsumed offer owns its redelivery. After acceptance the owner
remains responsible for progress and retirement.

Exercise one retained slot with a second HTTP request already waiting. Taking the first
result still holds the slot. In the review, dropping it produced zero wakes and an
explicit drive then delivered HTTP 200. In the successor, let final release be the only
enabling event. Verify one dispatch on the pool wake or promised maintenance turn without
another peer write, including release during waiter registration. While the result is
held, an event-driven waiter can sleep without repeated self-wakes.
[Admission before native receive](../servient/finite/src/host.rs#L1325-L1337) ·
[Final result release](../servient/finite/src/producer_capacity.rs#L156-L179) ·
[Release-only control](https://github.com/yushun1990/clinkz-wot/pull/309#discussion_r4690985997)

Each Binding facet supplies the actual owner its operation needs. Publication yields a closed
participation. Exclusive ingress supplies the input and original reply right or loan. An
outbound exchange retains its completion and retirement. A refusal returns the protected
operands. The Binding bounds its own buffering, native polls, backoff and last-user release.
Servient authorizes dispatch. A Host may erase these types or multiplex a shared listener.
An ordinary third-party Binding can implement the facet without a footprint catalogue,
reservation provider or executor. A deployment offering a stronger guarantee must control
the relevant backing, including anything a callback transfers into it.
[Binding delivery and retirement](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/binding-spi.md#L62-L97)

A source-free caller supplies prepared meaning, typed slots, slices, a clock and manual
progress. If an assigned base changes compiled meaning, the caller must supply a qualified
final definition or the candidate must refuse publication. Core shares the effect and
retirement transitions across representations. The caller's typed Read slot supplies the
storage that currently also requires a `ResourceReservation`. Scalar inline execution uses
caller storage without heap allocation, pointer atomics or an executor. Structured schema
preparation has its own storage requirements and can run elsewhere. A sole loaned output
buffer can delay the next call until the holder releases it. A separately provisioned
buffer buys concurrency. A hard RAM or latency promise additionally needs bounded
callbacks, target stack, allocator/SDK and interrupt evidence. A Host can report a recoverable allocation
failure only when its allocator permits recovery.
[Caller-owned Read](../docs/evidence/constrained-finite-execution.md) ·
[Caller-owned Action](../docs/evidence/continuing-action-runtime-independence.md) ·
[Environment contract](https://github.com/yushun1990/clinkz-wot/blob/fadb3aa6714c5faa20ba74e533f7a0aa8386e339/docs/spec/foundation.md#L238-L261)

## Disposition of existing resource abstractions

The table gives the target for each family. Each replacement ships with its consuming entry
point and the check that can falsify the retained guarantee.

| Current abstraction | Decision and destination |
| --- | --- |
| `ResourceKind`, `ResourceLimits`, `ResourceProfileId`, `StaticResourceProfile` and the three named profiles | **Delete the exhaustive public schema.** Move applicable fields, units, zero semantics and finite defaults to the owner that checks them. A deployment can compose those settings. Delete the CSV generator, frozen digests and schema-wide checker with the last dependent path. Keep benchmark inputs with the benchmark. [Schema generator](../foundation/build.rs) · [Frozen checks](../tools/check-resource-limits.sh) |
| `ResourceAccount`, `ResourceReservation`, `AdmissionLedger` | **Replace generic admission receipts.** A loan or slot owns its actual backing. A TD or Planning job owns any bounded scratch and durable output it needs, including their overlap. Keep high-water measurements at the allocation site where useful. Remove the six-bucket ledger and its commit/manual-refund API once their callers move. [Foundation accounting](../foundation/src/resource.rs) |
| `HostResourceAccounts`, `HostCapacityReservation` | **Relocate explicitly retained aggregate limits to their deployment.** A shared pool or move-only permit can enforce the ceiling through last use. Keep its admission decision atomic across all participating paths. Delete the dense Core bank and mandatory bank parameter with their callers. Remove obsolete aggregate fields when the corresponding deployment promise is retired. [Bank and race test](../core/src/host_accounts.rs#L151-L176) |
| `RawProducerRegistryLimits`, `RawFiniteLimits`, `RawActionLimits`, `RawNotificationLimits`, `DiscoveryLimits`, `BindingIngressLimits` | **Retain concrete bounds** for the registry, exchange, invocation, delivery or retrieval that can enforce them. Keep item and byte ceilings together where either can exhaust capacity independently. The native facet owns its wire limits. The application can observe which owner refused it. [Finite limits](../servient/finite/src/host.rs) · [Discovery limits](../discovery/src/http/mod.rs) |
| `ProducerFiniteCapacity`, its six-charge `WorkLease`, `ConsumerCalls`, `ConsumerActions`, `ConsumerNotifications`, borrowed `ReadRuntime` and `ActionSlot` | **Retain the pools, slots and distinct release points. Replace the Producer funding bundle** with real active/settlement storage and any separately enforced shared quota. `FiniteIngress`, `RawFiniteReply` and `RawFiniteSettlement` carry the original work and outcome. `ProducerAcceptPermit` proves current authority independently of capacity. [Producer capacity](../servient/finite/src/producer_capacity.rs) · [Native owners](../core/src/finite.rs) · [Accept authority](../core/src/producer.rs) |
| `CleanupReservation`, `CleanupPhaseContext`, `CleanupTransferEnvelope`, `CleanupTransferAcceptance` | **Replace numeric cleanup funding. Retain whole-owner transfer.** Retire in the original slot or use an already available destination. Carry subject, first cause and deadline. Rejection returns the complete work to its source, acceptance transfers it exactly once. Keep a transfer operation only on facets that actually hand off surviving work. [Current transfer contract](../core/src/binding.rs#L651-L846) |
| `WorkBudget`, `WorkClass`, `BudgetExceeded`, `ActionWork::prepay`, `work()`/`poll_work()` declarations and per-member budget factories | **Delete across the full call chain.** Replace with bounded polls, byte slices, cursors or a job-local lifetime allowance under the criteria above. Keep explicit yield, I/O wait, exhaustion and terminal dispositions. Remove the Host factories and the manual credit argument together for each migrated operation. [Action entry](../core/src/execution/action.rs) · [Host driving](https://github.com/yushun1990/clinkz-wot/blob/0e16c323549873d2dc19f7d9ade8bad10e19ef9c/servient/finite/src/consumer_execution.rs) |
| `BindingResourceDeclarations`, `BindingResourceContract::{StrictV1, ConsumerOwnedV2, ConsumerOwnedHostV2}`, lifetime/transient footprints and `BindingStateLayout` | **Delete registration-wide modes and transitive cost declarations.** Use the real type's layout for a supplied slot, enforce indirect backing at its owner and keep bounds on particular native facets. Relocate useful artifact/plan measurements to their builders. Replace handler footprint estimates with a bounded value or a retained application owner wherever the Engine promises backing. [Binding declarations](../core/src/binding.rs) · [Handler footprint](../core/src/handler.rs) |
| `ConsumerPropertyReadPolicy`, its Host/static build and runtime adapters, schema-2 encoding and configuration digest | **Replace the adapters and delete their policy format** after the public Read path and both source-enabled and prepared-only clients migrate. Preserve operation/Binding/generation and relevant configuration identity. Rebuild or reject an old prepared artifact if its meaning changed. Retire the overlay with its final reader. [Read adapters](../servient/src/consumer_property_read.rs) · [Schema-2 contract](../docs/spec/consumer-resource-ownership.md) |
| #308's `ProducerStorage`, `MemberFunding`, `FiniteSetupFunding`, `ProducerStageFunding`, `QualificationFunding` and public byte estimates | **Replace staged funding with the candidate's real leases, slots and qualification scratch.** Remove `candidate_bytes`, `handler_backing_bytes`, `base_backing_bytes`, `setup_peak_bytes`, `definition_bytes` and `final_check_temporary_bytes` from ordinary setup. A named advance reservation can live at the supplying pool. Reuse exact final qualification, closed endpoint retention and composed commit. [Funding bundle](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/servient/src/producer_composition.rs#L447-L535) · [Preparation SPI](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/planning/src/producer_composition.rs#L90-L160) |
| `CoreError::LimitExceeded` and `ResourceFailureDomain` | **Retain bounded failure reporting.** Replace the obligatory global resource key with the enforcing owner's limit and amount where meaningful. Preserve phase, domain, retry information and effect certainty, including when allocating a diagnostic would fail. Protocol fields stay inside the Binding. [Core errors](../core/src/error.rs) |

## Disposition of #308 and the roadmap

1. **Change the accepted contracts before resuming implementation.** Keep #308 paused. Use
   its public wire cases and independent review when building a successor from master.
   Successor trials test whether each owner delivers the selected guarantee at its boundary.
   Amend ADR-0015/0025, Foundation `RES-OWNER/HANDOFF/WORK/SCHEMA/ENV` and the affected
   Runtime Safety, Planning, Core and Binding clauses. Specify what must already be secured
   at each acquisition, acceptance and transfer. Keep generation, authority, bounded
   cleanup, overflow and equivalent Host/caller-owned behavior in force. Adjust the
   affected work packages and projections with their owning specifications. This proposal
   carries no milestone or contract status change. [Authority map](../docs/spec/authority.toml)
   · [Work packages](../docs/work-packages/index.toml)
2. **Build one finite Producer slice through the public Host and caller-owned entries.**
   Join #295/#300 publication, #296 independent exchanges and #299 source-free execution.
   Reuse #308's Binding-owned assembly, assigned socket, exact final qualification and
   rollback. Let two ordinary candidates acquire closed endpoints for one remaining member
   slot. Return the loser's inputs and release its original socket while the winner serves.
   Check split final membership, ambiguity and an unrelated registration. Run cancellation,
   foreign or stale owners, late completion and result retention through replacement.
   Verify the wire, actual socket reuse and exact handler/settlement ownership. Keep the
   four #308 review regressions explicit. Failure reported as terminal stays terminal. Two
   Reads sharing one final member fit the real cleanup slots. Passive zero-work observation
   invokes no configuration callback. The execution-only HTTP build accepts its own test
   targets. [Terminal failure](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764946)
   · [Cleanup ceiling](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764964)
   · [Zero-work callback](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764978)
   · [Feature boundary](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764987)
   · [Public API](https://github.com/yushun1990/clinkz-wot/issues/300)
3. **Test an advance reservation at its owner when a deployment needs it.** Repeat the
   finite case with the same Binding, same pool and same candidate, commit, dispatch and
   retirement operations. Apply contention before the promise and after it. Early refusal is
   valid before the promise. After success, exhaust all free slots and deny further
   allocation of the promised backing. The covered operation must still use its original
   slots and return them on rollback or last release. The two policies can admit different
   contenders. For an admitted candidate with the same inputs and environment, compare the
   published coverage, wire effects, first cause and release sequence across both trials.
   Verify ordinary dispatch with a Binding that cannot supply the promise and explicit
   rejection of that guarantee before native acquisition. If this needs a second execution
   loop or changes Binding selection, narrow or withdraw the promise. Ordinary admission
   can ship independently. [#308 contention control](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/tests/producer_exposure.rs#L497-L594)
4. **Migrate work bounds through each complete call chain.** For finite Read and continuing
   Action, change the shared Core step, both storage adapters, the Binding/codec/access
   facet and their public drivers together. Saturate ordinary pools and settle an original
   exchange with no new access or codec calls for local close. Preserve late data for its
   authorized retrieval. A ready sibling and a drain must progress while new admission is
   refused. Exercise a separately authored Binding and caller-owned UDP on the host, and
   compile the source-free entry for `thumbv6m-none-eabi`. For TD/Planning/Discovery, move
   the step and exhaustion result to the traversed job. Repeat exact and one-short lifetime
   tests across rewind and resumption. Exercise bounded bytes/pages, callback counts and
   explicit yield versus I/O wait. Remove the migrated calls' cost declarations and budget
   factories at both ends. [Shared finite step](../core/src/execution.rs) ·
   [Caller-owned execution](https://github.com/yushun1990/clinkz-wot/issues/299) ·
   [Independent exchanges](https://github.com/yushun1990/clinkz-wot/issues/296)
5. **Cut over each retained shared quota as a unit.** Decide which aggregate the target
   deployment still promises, to whom, which operations may draw on it, and when a unit
   returns. For example, a deployment may cap one tenant at eight outstanding Consumer
   and Producer calls combined, including unread and caller-held results through disposal.
   While old and new paths serve that tenant, all admissions and releases must conserve
   those eight rights. Either path may use the last place if that sharing is promised.
   Both can draw on the original pool or its permits. Disjoint partitions are valid if the
   promise allows them, their limits sum to at most eight and every live claim counts in
   exactly one share. Alternatively, switch all participants together. Fence old admissions
   and drain every counted holder before opening a fresh full pool, or transfer outstanding
   claims and their release obligations exactly once. An old receipt refunding its original
   bank cannot protect an independent new pool granting the full allowance.

    Where both paths draw on one shared remainder, fill the last unit with an old result.
    A new call with a free native slot must be refused, including after the old result is
    taken but still held. Dispose of it and race two new callers for the released unit.
    Exactly one must succeed. For fixed partitions, verify that the unit returns to its
    assigned share. [Shared Host bank](../core/src/host_accounts.rs#L10-L29) ·
    [Retained-result control](../protocol-bindings/protocols/http/tests/composed_thing/capacity.rs#L290-L312) ·
    [Quota cutover review](https://github.com/yushun1990/clinkz-wot/pull/309#discussion_r4690986003)

    **Retire obsolete contracts directly.** Sharing a bank in an old example or naming
    a field in a profile creates no compatibility obligation. For example, a deployment
    can explicitly retire its aggregate `CleanupBytesMax` ceiling while still backing
    each accepted call's cleanup. Remove that ceiling, its configuration and checks with
    their last consumer. Test old/new contention only where a retained guarantee spans
    coexisting paths. Pre-v1 removal requires no compatibility adapter, dual execution or
    transfer of historical counters. Accepted work keeps its result and cleanup owners
    through release. Migrate the Host and static Read entries, re-exports, examples,
    benchmarks and fixtures. Delete the profile and overlay with their last reader.
    Rebuild prepared artifacts whose meaning changed and preserve old evidence at its Git
    revision. [Pre-release compatibility](../PROJECT_GOVERNANCE.md#pre-release-api-compatibility)
    · [Read API consumer](../tools/architecture-fixtures/consumer-property-read-runner)
    · [Preparation consumer](../tools/architecture-fixtures/property-read-runner)

Extend the demonstrated boundary to MQTT deliveries and #297 subscriptions. An individual
cleanup failure can leave a healthy Thing serving. A lost shared transport must fence its
dependents. Use #298 to measure staged and held allocator bytes, quota headroom, useful
completions and cleanups per turn on the public paths. Compare one and many idle Things at
fixed active demand. Retain a concrete reservation where a smaller representation loses an
owed result or the ability to retire. Consumer selection (#293), per-effect security (#294)
and bounded Discovery retain their own dependencies. Revise the durable roadmap on
acceptance. [Publication objective](https://github.com/yushun1990/clinkz-wot/issues/295) ·
[Subscription ownership](https://github.com/yushun1990/clinkz-wot/issues/297) ·
[Cost and reuse](https://github.com/yushun1990/clinkz-wot/issues/298) ·
[Roadmap and milestones](../PLAN.md)

**Evidence boundary.** Master `fadb3aa6` supplied seven shared-work HTTP cases, 40
caller-owned Consumer tests, 16 MQTT Producer Event tests with Mosquitto 2.0.22, 17 remote
Discovery cases and five execution-only HTTP tests. The earlier reassessment also checked
the `thumbv6m-none-eabi` build, Producer Action cleanup, TD lifetime exhaustion on rewind
and the disposable UDP cancellation control. For the inline review I reproduced the JSON
scratch, nested-poll and release-only wake observations on the same master revision. The
existing HTTP Action wire-refusal, retained-result pressure and shared-bank race controls
passed. On #308 `733e4d70`, the twelve public exposure cases passed. The execution-only
HTTP test build failure and two-candidate heap measurement pertain to that head. The
proposed contention, validated handoff and bounded-drive trials belong to the respective
successor implementations.
