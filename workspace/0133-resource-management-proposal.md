# Resource ownership and admission in the WoT Engine

**Status** DISCUSSING

**Recommendation.** Replace mandatory byte estimates and caller-supplied work budgets on the
ordinary Host path with bounded storage at each owner and explicit checks when an obligation
changes hands. Before accepting work, secure its result and cleanup path. Admit reversible
setup in stages. Use advance reservations when a deployment promises that a later stage will
have capacity. Drive ordinary calls with a deployment-configured quantum and keep explicit
metering and caller-owned storage available for constrained execution.

Under the proposed default, a Host can acquire an assigned socket and subsequently lose the
race for a final member slot. The caller recovers its inputs, the candidate owns the closed
lease through disposal and a coexisting incumbent keeps serving. After an exchange is
accepted, its space to settle survives even if another caller fills the result queue. Keep
#308 paused and rebuild the exposure path on this distinction after contract review. The
comparison uses master `fadb3aa6714c5faa20ba74e533f7a0aa8386e339` and #308
`733e4d708b51c3db1b8357899eb93b5485a11bf4`.
[PR #308](https://github.com/yushun1990/clinkz-wot/pull/308) ·
[Current publication contract](../docs/spec/runtime-safety.md#L513-L526)

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
Foundation can supply time, checked arithmetic and move-only permits to either deployment.
The following boundaries apply to both roles wherever they offer the interaction.

| Obligation | Who can enforce it, and where the obligation ends |
| --- | --- |
| Bound work and growth caused by input | Discovery caps bytes, pages, results and elapsed time across a retrieval. TD, Planning, codecs and access services bound their own decoding, depth, expansion, provider search and intermediate storage. A too-large description or unsupported security branch produces an explicit refusal. All advertised Producer branches must qualify. Consumer choice may stop at a complete viable interaction. |
| Bound Engine-owned populations | Servient limits simultaneous candidates, instances, calls, invocations, conversations, queued deliveries, draining work and held results in their respective domains. Variable-size records need a byte or shape bound as well as a count. Shared immutable definitions have one backing owner through last use. Each mapping or recipient still needs space for its own record. An idle Thing needs authority and a mapping. Call storage can come from a shared pool on demand. |
| Preserve the right to reply | Before accepting exclusive ingress, Servient secures the original instance, current permission, behavior, deadline and space for the promised outcome. The Binding moves the reply right, lends an exclusive native slot, or copies into admitted storage under its own delivery rules. Refusal returns the complete offer to its original owner, including its deadline. An active slot can be reused once its exchange retires. Any unread or caller-held data keeps its original backing or loan through disposal. |
| Keep continuing work distinct from individual exchanges | An Action invocation keeps its identity and continuation across Invoke, Query and Cancel under a bounded retention policy. A subscription retains its control identity. Each delivery, callback reply and queued recipient has a separate lifetime. The Binding owns native fan-out, matching and acknowledgements. A collection keeps its defined membership and result semantics. Apply declared overflow or visible loss only where the interaction permits it. |
| Cancel, replace and shut down truthfully | Servient fences the original generation immediately on revocation, records the first cause and retains late output and effect uncertainty. A Binding retires the exact exchange or participation, and owns final release of a shared listener, session or writer. A whole-owner transfer needs an accepting destination. A bounded diagnostic can survive an uncertain remote effect. Surviving local work still needs an owner. |

An arena slot or owned buffer supplies **backing**. A debit on a shared tenant account
supplies **exclusion** against other users of that account. A work allowance bounds **how
far one drive may run**. The owner refunds a quota debit at the last use it represents.
Spent work stays spent. None of these quantities measures kernel buffers, an opaque
application's heap, or elapsed CPU time. A contract can legitimately require two of them at
the same transition. [Foundation](../docs/spec/foundation.md) ·
[Binding delivery and retirement](../docs/spec/binding-spi.md#L62-L97)

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
  [Consumer ownership](../docs/spec/foundation.md#L30-L55) ·
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
or quota. A strict deployment can partition that pool or lend linear permits against it.
Independent banks supply independent ceilings. An advertised aggregate needs a common
enforcing owner whose admission covers concurrent and reentrant callers.

## How the recommended design works

On Host, bound each dynamic container where it grows. A pool of at most N entries with a
checked maximum size per entry gives a calculable envelope. If entries draw variable backing
from one shared ceiling, acquire the extra capacity before growing, use fallible
construction where supported, and return the provisional debit if construction fails. A
shared allocation has one physical owner through its last reference. A tenant debit may
legitimately follow the result for longer than its active call. Keep the bank only where it
enforces such a policy. Counters for diagnostics can be sampled separately from admission.
Use a fixed-size fallback when even error reporting is saturated.

### Publication

Before acquiring a closed base, check known semantic and behavior requirements and acquire
one bounded candidate with its rollback storage and deadline. The Binding returns the actual
endpoint and its unique guard. Resolve and qualify the exact final TD under current
configuration. Acquire storage and rollback capacity for the actual selected members *before
each corresponding acquisition*. Try the full set of scarce commit resources together, or
unwind partial acquisitions in a bounded order. Competing candidates must be able to release
their leases without waiting for each other's next slot. A short cooperative turn keeps the
same candidate and completed work. A permanent limit, conflict, cancellation or observed
expiry fences that candidate and starts rollback immediately. If rollback itself requires
a second slot, acquire that slot before setup. If a remote effect cannot be reversed, the
mapping must support a truthful residual outcome or decline the operation.

After the last callback, verify the expected incumbent, exact original members, behaviors,
coverage and readiness together. Commit the TD, authority and ingress gates in one bounded,
infallible local transition. An exclusive route that needs break-before-make has the service
gap already allowed by the roadmap. Closed local ingress can coexist with externally visible
native setup. Each Binding must identify any remote residue and provide an admissible
cleanup path.
[Current qualification and replacement rules](../docs/spec/planning.md#L107-L152) ·
[Roadmap on exclusive replacement](../PLAN.md)

### Accepting and retiring work

At each effect boundary secure everything the Engine must carry even if ordinary allocation
and queues immediately fill. This includes the actual input or loan, the reply owner, a
bounded normal or late result when promised, the minimal failure record and a way to retire
the work. A status cell suffices only for a failure whose contract owes no data. Recheck
original permission after reentrant callbacks and before the effect. Keep linked code and
destructors outside registry locks.

A handle can become its own cleanup record in place. An asynchronous transfer needs an
already available destination that accepts the whole owner. Supported allocation failure
before acceptance returns the original input. If rollback still borrows a unique input, its
owner returns it as soon as that borrow ends. After an ambiguous network effect, record
the uncertainty and first cause, retaining an existing output through validation or
disposal. Release only the retired exchange. Fence dependent participations if the native
service itself is lost. On root loss use finite local disposal or acknowledged transfer to a
longer-lived owner. Explicit close reports local drain and any unresolved remote effect.
Process abort or a stopped driver cannot promise eventual completion.
[Runtime cleanup and Drop](../docs/spec/runtime-safety.md#L535-L574) ·
[Native delivery/retirement](../docs/spec/binding-spi.md#L62-L97)

### Work per turn and over a job's lifetime

Require an enforceable input or batch bound for Engine work and a qualified, cooperative
bound for each linked callback. A fixed-size transition needs no public counter. A TD walk,
provider search, codec or fan-out over variable input needs finite input and total-work
limits and either a bounded atomic step or a cursor. Debit actual work before callback
entry. A retry continues the cursor or spends the original job's remaining allowance. Bound
staged lifetimes and candidate count so a stalled preparer cannot monopolize every lease. An
atomic step larger than every permitted turn must be refused explicitly or divided. Keep a
linear allowance when multiple alternatives spend one quota. Use a single bounded batch size
where that is sufficient and distinct units where, for example, bytes and provider calls
need independent limits. A generic poll count cannot bound the contents of a poll. A wall
clock can expire an obligation but cannot interrupt linked code.

Give retirement a runnable path under ordinary saturation. A cleanup turn can poll the
original native close and retain a racing value even if access or codec work has zero
credit. Delivering that value to the application still requires the applicable checks. A
zero-work manual turn may observe a fence or return an existing status. Linked callbacks
require positive credit. Host scheduling can rotate ready owners, reserve opportunities for
cleanup, and arrange a wake or a bounded retry when progress becomes possible. The
caller-driven profile supplies its clock, readiness and turns. Progress is conditional on
these services and on cooperative participants. Test a continuously ready sibling alongside
a stalled one. [Finite and Action kernels](../core/src/execution.rs) ·
[Host turn contract](../docs/spec/runtime-safety.md#host-producer-execution-turns)

### Host, caller-owned execution and the Binding SPI

On Host, configure finite capacities and overload policy once at deployment. The ordinary
operation is `expose(td, behaviors, replacement)` or a named Consumer interaction. Expose a
pending candidate or an admitted handle, a terminal refusal that returns recoverable inputs,
and explicit cancel/close and result retrieval. Give pending work a specific resumption
condition such as native readiness or the next turn. Report fixed limits with scope, phase
and required/available amount when known. A terminal report from the owning Host must be
final even while physical rollback drains. A poll on a foreign Host leaves the original
candidate with its owner. Resuming accepted work uses its original handle. A fresh attempt
after an uncertain effect needs an explicit application decision. Normal application code
should have no reason to estimate a closure's transitive heap, request readiness tokens or
fill twelve counters per member poll. If a handler transfers heap to a strictly bounded
deployment, require a bounded representation, a retained application owner or a measured and
enforced allocation contract at registration.
[#308 application example](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/protocol-bindings/protocols/http/examples/serve_properties.rs#L105-L205)

A third-party Binding declares its own ingress, buffering, concurrency and retirement
policy. Each implemented facet supplies the owners it needs. Publication supplies a closed
participation, exclusive ingress supplies its complete input and original reply or loan, and
an outbound call supplies its own completion and retirement. Each has bounded progress and
an observable local retirement or abort disposition. It returns protected operands on
refusal. It owns socket pools, frame limits, backoff and last-user release. Servient alone
authorizes application dispatch. Host preparation can keep Binding-owned assembly. Remove
`setup_peak_bytes`, `definition_bytes` and `final_check_temporary_bytes` as ordinary
registration requirements wherever the concrete owner already bounds growth. A strict quota
covering those bytes needs an enforced maximum with a quota receipt, a participating
allocator or an explicitly partitioned pool.
[#308 preparation SPI](https://github.com/yushun1990/clinkz-wot/blob/733e4d708b51c3db1b8357899eb93b5485a11bf4/planning/src/producer_composition.rs#L90-L160)

A Host adapter may erase types or share an explicit bank. A source-free caller may lend
typed slots, mutable slices, a clock and bounded manual steps, with no allocator, pointer
atomics, TD compiler or runtime imposed by this handoff. A fully backed typed slot can
replace a mandatory `ResourceReservation` at the caller-owned Read entry. Indirect buffers
and any second retained result need their own backing. If the Binding lends its sole output
buffer, the holder can legitimately delay reuse until disposal. A separately funded copy
buys concurrency. Hard RAM or latency claims require limits on payloads, SDK and callback
behavior, plus target stack, allocator, interrupt and high-water evidence. Host allocation
failure follows the participating allocator's actual failure contract. An opaque or aborting
SDK needs deployment-level containment for stronger promises.
[Caller-owned Read evidence](../docs/evidence/constrained-finite-execution.md) ·
[Caller-owned Action evidence](../docs/evidence/continuing-action-runtime-independence.md) ·
[Profile and environment contract](../docs/spec/foundation.md#L238-L261)

## Disposition of #308 and the roadmap

1. **Review the contract change first.** Keep #308 open as a source of behavior and tests.
   After adoption, supersede the current funding design in a successor change based on
   master. Migrate the general rules in ADR-0025 and ADR-0015's work amendment, Foundation
   `RES-OWNER/HANDOFF/WORK`, Runtime Safety
   `SAFE-PUBLISH/CLEANUP/DROP/CONCUR/OVERFLOW/PROFILE`, Planning isolation/coverage, the
   Binding ingress/delivery/retirement SPI and Core outcome semantics. Retain the existing
   authority and lifetime checks on all current paths until the successor is admitted.
   Distinguish the optional advance-completion guarantee from the required cleanup
   guarantee. Record the revised rationale in an ADR, reconcile the architecture backbone
   and authority/API/state projections, and adjust the affected WP-000/100/200/300/400
   clauses. Migrate affected contract tests with their new owner. Preserve the meanings of
   StrictV1 and the Consumer schema-2 overlay for retained adapters and their old evidence.
   Retiring an adapter can remove its bookkeeping with its callers. New meanings require new
   dispositions. [Authority map](../docs/spec/authority.toml) ·
   [Work packages](../docs/work-packages/index.toml)
2. **Rebuild the finite public Producer entry.** Reuse Binding-owned assembly,
   assigned-socket retention, exact final selection, shared listeners, returned handlers and
   the tested composed commit. Replace mandatory candidate/scratch estimates and speculative
   member prepayment on the default path. Move policy-specific pressure assertions to the
   stronger deployment that promises completion. The head review records four distinct
   defects. Terminal failure followed by a successful publish violates either design. Two
   Forms that share one final HTTP member can be refused with three cleanup slots because an
   unrelated registration raises the speculative member count. Lowering the readiness
   ceiling to one let both Reads succeed in the review. Configuration callbacks at zero work
   violate a metered entry. The HTTP execution-only test target also imports
   preparation-only APIs. Carry all four as successor regressions with the appropriate
   semantics.
   [Terminal failure](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764946)
   · [Cleanup ceiling](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764964)
   · [Zero-work callback](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764978)
   · [Feature boundary](https://github.com/yushun1990/clinkz-wot/pull/308#discussion_r4680764987)
3. **Use the two deployment paths to settle the SPI.** Join the public exposure in #295/#300
   to exchange retirement in #296 and the source-free Producer in #299. Stage two candidates
   for one final slot. One must be able to fail after acquisition, release its original
   endpoint and leave the winner serving. In a deployment promising advance capacity,
   pressure from the second candidate must leave the first one's reserved local completion
   path usable. Test split final membership, ambiguity, cancellation, stale and foreign
   owners, two native offers finishing out of order, and a retained result through
   replacement. Fill ordinary pools and withhold codec/authorization work while draining the
   original native right. Check the wire, socket reuse, exact handler/result owner, and
   bounded wake/turn progress in addition to counters. Preserve execution-only builds and
   exercise the same lifecycle with a separately authored Binding and caller storage.
   [Publication objective](https://github.com/yushun1990/clinkz-wot/issues/295) ·
   [Independent native work](https://github.com/yushun1990/clinkz-wot/issues/296) ·
   [Caller-owned execution](https://github.com/yushun1990/clinkz-wot/issues/299) ·
   [Public API](https://github.com/yushun1990/clinkz-wot/issues/300)
4. **Extend only the demonstrated boundary.** Exercise admission across Action controls and
   MQTT deliveries, then callback subscriptions and occurrence replies in #297. A single
   delivery cleanup failure should leave a healthy Thing serving. A damaged shared stream
   must fence its dependents. Use #298 to compare staged and held allocator bytes, quota
   headroom, useful completions and cleanups per turn under contention on ordinary public
   entry points. Compare one and many idle Things at fixed active demand. Retain explicit
   reservation wherever a smaller representation loses an owed result or cannot finish local
   rollback. Consumer selection (#293), per-effect security (#294) and bounded Discovery can
   progress on their own dependencies. Revisit the durable roadmap after acceptance. Keep
   milestone status tied to public integration, independent review and the advertised
   deployment. [Subscription ownership](https://github.com/yushun1990/clinkz-wot/issues/297)
   · [Cost and reuse](https://github.com/yushun1990/clinkz-wot/issues/298) ·
   [Roadmap and milestones](../PLAN.md)

**Evidence boundary.** On master I ran seven shared-work HTTP cases, 40 caller-owned
Consumer fixture tests, the fixture build for `thumbv6m-none-eabi`, 16 MQTT Producer Event
tests with Mosquitto 2.0.22, 17 remote Discovery tests and five HTTP tests with default
features disabled. On #308 all twelve public exposure cases passed. HTTP package tests with
default features disabled failed to compile the new exposure target. The staging
measurements above used a disposable test on that exact head. The proposed admission and
progress changes still need the paired Host and caller-owned execution described above.
