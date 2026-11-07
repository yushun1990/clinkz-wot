# Multi-instance Consumer execution scalability

Preserve the current reusable-content, exact-definition and instance-binding
seams for the tested Host finite path. They support 10,000 independently
authorized logical instances with stable execution throughput at a fixed native
capacity. They do not establish a finalized scaling architecture. One bounded
HTTP correction is justified: release native exchange occupancy after the
connection/driver retires, independently of retained result consumption.

The meaningful remaining constraints are repeated TD qualification, dense account
storage, prepaid inline call slots, shared-account preparation reservations,
semantic resolver capacity, allocator traffic/retention and native concurrency.
Their costs and limits differ; none is a device-count limit.

## Reproduce and inspect the data

```sh
cargo build --release --locked --offline -p clinkz-wot-binding-http --example multi_instance
python3 tools/multi-instance/run.py --suite full --repeat 3 --output /tmp/clinkz-multi-instance
python3 tools/multi-instance/summarize.py /tmp/clinkz-multi-instance > /tmp/clinkz-multi-instance/summary.csv
```

The [workload documentation](../../tools/multi-instance/README.md) defines every
phase, option and measurement boundary. Source is the
[executable](../../protocol-bindings/protocols/http/examples/multi_instance.rs),
[allocator meter](../../protocol-bindings/protocols/http/examples/multi_instance/meter.rs)
and [separate-process peer](../../protocol-bindings/protocols/http/examples/multi_instance/peer.rs).

- [Measured environment and all commands](multi-instance/measured/environment.json):
  `f68273307e0c4de08f818933ab6a753c216984b2`; [median/range CSV](multi-instance/measured/summary.csv);
  [raw JSONL, gzip](multi-instance/measured/measurements.jsonl.gz).
- [Baseline environment](multi-instance/baseline/environment.json):
  `0f0efc48ac504745f8039cc9fb29ec44a615d151`, with unmodified post-#267 production code;
  [CSV](multi-instance/baseline/summary.csv) and
  [raw JSONL, gzip](multi-instance/baseline/measurements.jsonl.gz).
- [Fully active distinct-target environment](multi-instance/targeted-load/environment.json):
  `b2fb83863dc822ccf90cadcfef054d162c2921e9`; [CSV](multi-instance/targeted-load/summary.csv) and
  [raw JSONL, gzip](multi-instance/targeted-load/measurements.jsonl.gz). The native
  executable is unchanged; its binary hash matches the main sweep.

The archived sweep comprises **42 configurations × 3 fresh consumer processes**,
plus a focused **10,000-distinct-target, fully active** configuration repeated
three times: 120 supported runs and nine expected preparation refusals, with
**304,794 measured calls** plus warm-up/lifecycle exchanges. The current full
suite includes all 43 configurations. Populations are 1, 100, 1,000 and
10,000; active fractions are 0, 1%, 10% and 100%; native lanes are 1, 8 and 32.
Additional cases vary slots, account topology, schema members and preparation
mode. Three rounds rotate each active cohort through the population. Each
instance has fresh authority, an instance/destination-bound mutable permission
service and its actual funding domain. The peer checks every selected path/header
and the caller checks the complete returned member map.

Measurements use release Rust 1.95.0, Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz, Linux x86_64 and
glibc 2.43; affinity, kernel, compiler, binary and lock hashes
are recorded above. Cases run serially; alternate replicates reverse case order.
These are observations on an unpinned general-purpose host. Tables report medians;
the CSV retains throughput ranges and raw data retains all replicates. Short
three-call cases establish functionality, not sustained throughput. MB below
means 1,000,000 bytes.

The workload uses actual HTTP Binding I/O against one independently implemented
loopback peer process, with many addressable resources. Registrations are
explicit native lanes, not Things, devices or permanently open connections.
Preparation opens no connection; each actual call uses the Binding's existing
fresh connection policy. No production quota or work allowance is increased.

## Preparation and native compilation

For 10,000 instances, eight native lanes, sixteen integer members and one slot:

| Preparation configuration | Native jobs | Common owners | Total ms | Parse ms | Native job-step ms | Retained MB | Requested traffic MB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Shared exact definitions | 8 | 1 | 1.90 | 0.39 | 0.02 | 0.018 | 0.980 |
| Distinct targets, shared semantics | 10000 | 1 | 2,211.09 | 433.54 | 19.77 | 20.131 | 1,223.083 |
| Distinct targets, rebuilt semantics | 10000 | 10000 | 2,394.01 | 441.43 | 20.88 | 39.259 | 1,242.211 |

Shared exact definitions require eight actual TD qualifications and eight native
jobs, then 10,000 source-free binds. Distinct targets require 10,000 actual TD
qualifications and 10,000 exact HTTP mappings, even when they share one prepared
semantic owner. The cold control also rebuilds that owner 10,000 times. TDs are
actually destroyed before instance binding/execution.

Semantic reuse saves **19,128,087 retained bytes** against the cold 10,000-target
control, or 9,999 copies of the 1,913-byte common owner. It does not remove repeated
admission, resolution, comparison or service qualification. Distinct-target
preparation makes **3,820,020 allocation/reallocation requests**, requesting
**1,223,083,033 bytes cumulatively**, while retaining about 20 MB. Its preparation
peak above the starting heap is **20,192,761 bytes**, not 1.22 GB. The parse
intervals request 781,306,670 bytes; qualification requests about 44,170 bytes per
mapping on average, including the current fixed output-table provisioning.

Native job steps consume about 19.77 ms of
2,211.09 ms total, below 1%. They include the compilation wrapper
but exclude earlier native-input snapshot/bounds construction, which remains
inside qualification. Time is nested and must not be added twice. Native steps
make 50,000 allocation requests, request 13,287,780 bytes and retain 12,640,000
bytes in this interval. Removing native lowering alone cannot remove the dominant
TD/qualification traffic.

Work credits tell a different story from elapsed time. The shared-semantic
10,000-target case uses 32,647,780 DocumentNodes and 1,064,323,999 PlanningItems;
the cold control uses the same DocumentNodes but 4,420,000 PlanningItems. Reuse's
borrowed comparison prepays a deliberately conservative bound. Both perform
40,000 native BindingPoll credits across 10,000 jobs; the concrete compiler has
one bounded lowering callback per job. Credits are charged work allowances,
not CPU instructions or allocator bytes.

## Idle, active and physical storage

For shared definitions, one measured idle instance consists of:

| Component | Requested retained bytes per instance |
| --- | ---: |
| Authority and permission-control flag | 48 |
| Example permission service and its retained owner | 178 |
| Source-free definition binding | 24 |
| Runtime construction, including its one vacant call slot | 2,160 |
| Caller-owned logical root record | 32 |
| **Total, excluding account provisioning** | **2,442** |

The bundle makes 14 allocation requests per instance plus one root-vector
allocation. Runtime construction accounts for seven of those requests. A vacant
inline slot is **1,792 bytes**; each additional provisioned slot adds that amount.
These are current layouts for this fixture, not ABI commitments or a requirement
for this number of public types. The example permission service is application
code; its cost is not an irreducible Engine cost.

At 10,000 instances and 1% active, changing quota-domain or slot provisioning gives:

| Account domains | Slots/instance | Account provisioning MB | Idle bundle MB | Bind/construct ms | glibc in-use MB at idle | RSS MB at idle |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Independent | 1 | 62.973 | 24.420 | 15.85 | 89.297 | 93.544 |
| Independent | 32 | 62.973 | 579.940 | 145.99 | 644.817 | 649.142 |
| Common | 1 | 0.019 | 24.420 | 14.96 | 26.182 | 30.491 |
| Common | 32 | 0.019 | 579.940 | 144.03 | 581.702 | 586.019 |

A bank itself is 6,272 bytes; its Arc allocation and vector reference add 24 bytes
per independent domain. The account phase also includes two physical-service/
registration banks. Common accounts preserve independent authorization, but
supply a common quota domain instead of per-instance quotas. They cannot be
substituted when independent quotas are required.

Provisioning one slot instead of 32 saves **555,520,000 idle bytes** in this
one-call-per-instance workload, without a new allocator or scheduler. Both are
valid configurations with different promised per-instance capacity. The
one-slot population constructs in about 15.85 ms; source-free
binding itself retains 24 bytes per instance and spends two ProviderProbes.

The eight-lane service/registration setup retains **10,336 requested heap bytes**
and reserves **74,376 bytes**, independently of logical population and exact
mapping count. Actual data and startup access services have one retained owner;
registry records, each actual native registration, semantic content, mapping,
instance, call and result retain their distinct owners. This is explicit physical
sharing, not repeated raw adoption or one connection per logical Thing.

For the representative eight-lane shared case, admitting eight calls adds
**4,598 requested live heap bytes per lane**, including actual preallocated caller
result carriers and call handles. Completed retained results use **4,744 bytes
per lane** above the idle baseline. Their original EngineLiveBytes reservation
increases by **80,280 bytes per lane**, including result/scratch/cleanup capacity.
Reservation is an allowance, not an allocator reading. Call/result carrier memory
is included even when its vector was allocated before the result arrived.

Per-call requested allocation traffic is roughly 52 KB; active peak heap also
includes temporary native SDK/future storage. These transient allocations do not
accumulate with the number of idle Things. The load-driver queue/latency vectors,
source inputs and expected-body corpus have separate phases.

**Final object release does not imply RSS release.** Every account returns to
zero, both observed physical owners expire, and process FD count returns to its
starting value in every run. Final requested live heap is only about 23 KB of
runtime/measurement/application envelope. However glibc retains freed arena pages:
for example the 10,000-instance/32-slot independent-bank case still has about
649 MB RSS after account release. The one-slot case remains about 94 MB. Raw
records include glibc in-use/mapped bytes at each phase. These observations are
allocator retention/fragmentation, not live Engine objects; resource reservations
are not a process RSS guarantee. No malloc trimming, global pool or allocator
replacement is used to improve the reported numbers.

## Execution and native concurrency

At 1,000 instances, all active, three measured rounds, and sixteen members:

| Native lanes | Engine calls/s, no delay | Binding control calls/s, no delay | Engine calls/s, requested 1 ms peer delay | Binding control calls/s, requested 1 ms peer delay |
| --- | ---: | ---: | ---: | ---: |
| 1 | 8,884 | 9,791 | 443 | 446 |
| 8 | 11,827 | 14,119 | 2,902 | 3,004 |
| 32 | 12,087 | 14,213 | 7,465 | 8,507 |

The Binding control executes the same qualified artifacts, Core finite seal,
peer, bodies, queue and connection policy. It omits Consumer Runtime permission,
validation and bookkeeping. At eight lanes the Engine adds roughly two allocation
requests and 520 requested bytes per call over that control. Synchronous
admission/poll elapsed spans are recorded separately from waits between polls;
they include SDK work and possible OS preemption. Neither those spans nor a
subtraction of two runs is an exact exclusive Engine CPU measurement.

Increasing zero-delay concurrency beyond eight does not materially increase
throughput on this current-thread process/SDK/peer workload. With delayed peers,
more lanes permit more overlap. The requested delay is not a precise timer
promise; actual service latency includes Tokio timer granularity and network/SDK
progress. This supports a native concurrency/latency finding, not a universal
scheduler or global fairness conclusion.

For 10,000 instances at eight native lanes, changing the active cohort gives:

| Active instances/round | Calls/s | Admitted p99 microseconds | Cohort p99 ms, including queue |
| --- | ---: | ---: | ---: |
| 100 | 11,271 | 852 | 9.98 |
| 1000 | 11,865 | 814 | 85.59 |
| 10000 | 11,809 | 840 | 838.46 |

At 100% active, the 1,000-instance rate is 11,827 calls/s
(range 11,647–11,952); the 10,000-instance
rate is 11,809 (range 11,630–11,830).
The fully active **distinct-target** control prepares and binds all 10,000
paths and executes all of them in each of three rounds. It reaches
11,870 calls/s (range 11,800–11,957), admitted p99
838 microseconds and cohort p99 834.62 ms. This is
actual I/O through every target, beyond the sparse 1% targeted cases.

Idle population does not force a whole-population progress sweep in this workload:
the application explicitly drives only accepted active calls. Queue delay grows
with offered work at fixed native capacity. No claim is made for an application
that sweeps every idle runtime, or for other operations/notification drivers.

## Corrected occupancy lifetime

In all nine baseline runs an independent sibling is refused while a completed
result is held, although the peer reports no active response. Source inspection
shows the connection/future is already destroyed by `Call::retire`, while only
`Call::drop` clears the registration's busy flag. Result consumption can therefore
block unrelated instances indefinitely after their physical exchange has ended.

The correction makes the busy reference an optional, exactly-once occupancy
lease. Retirement drops the connection/future/target first, then takes and releases
that lease. The old call remains a funded result owner. Later old-result Drop
cannot clear a successor's flag. Actual concurrency remains one accepted,
unretired exchange per registration; an accepted unpolled call still owns its
lane until native retirement. The concrete private policy becomes v16, while
artifact payload format remains v003.

All **102** post-correction lifecycle probes admit the sibling while retaining the
old result. The [real I/O regressions](../../protocol-bindings/protocols/http/tests/support/native_occupancy.rs)
add normal/error result retention, cancellation, unpolled abandonment, successor
exclusion and original-account final release. Existing Write/root-query tests
continue to assert input recovery, current authority, effect evidence and unread
Engine-slot capacity independently of native occupancy.

Median observed local cancellation-to-settlement is **12.67 microseconds**;
replacement plus original late-result settlement is **9.42 microseconds**
across the lifecycle cases. The original identity/error and real returned output
survive replacement, and independent permission changes refuse before I/O.
Dropping the last eight results in the representative 10,000-instance case takes
about **17.54 microseconds**, with zero allocation requests. The subsequent
all-kind account verification scan is outside that timing. These are local
observations, not remote cancellation acknowledgements or deadline guarantees.

Baseline raw data is retained as a functional counterexample. Its earlier
measurement layout preallocated result/logical-root carriers in fixture storage
and included verification in release timing. Use the final measured data for
active/idle/release costs; do not infer a timing regression from those differing
boundaries.

## Hard limits and alternative designs

The workload executes both sides of two additional refusal boundaries:

- **Semantic scopes:** this exact root-map TD works at 29 members and refuses at
  30 and 32 before native compilation or I/O. The resolver has 32 scopes, including
  Thing, Form and security scopes, and 128 nodes. Generic collection capacity of
  32 members is not proof every such TD fits resolution. These are explicit
  refusals, not missing members or leaf-operation fan-out.
- **Shared preparation quota:** 896 distinct mappings prepare in one common
  Gateway account. The 897th fails with `CleanupItemsMax`, limit/observed 1,024,
  requested 1. Existing artifacts retain 896 items, the preparation output table
  reserves 128, and the new native artifact needs another item. Failure occurs
  before its native job starts; all 896 prior mappings/services then release and
  the peer sees zero requests. This is a conservative preparation overlap limit,
  not 896 Things: eight shared mappings bind 10,000 instances in that same quota
  topology. No limit is raised to hide the refusal.

The separate API probes accept 32 registrations and reject 33 per registry, and
accept 32 call slots while rejecting zero/33 per finite runtime, returning the
original binding on refusal. Source inspection additionally bounds selected
finite definitions/support rows at 128, targets at 64 bytes and HTTP bodies at
4,096 bytes. These bounds apply to their actual artifacts/owners, not device
cardinality. Other profile and aggregate quotas can refuse earlier; independent
banks do not prove a shared global quota domain fits the same workload.

| Alternative | Evidence and disposition |
| --- | --- |
| Reuse an already qualified exact definition | Executed across 10,000 independent authorities. Use when target/native facts really agree; it removes repeated TD qualification and native compilation from binding. |
| Share semantics while independently compiling targets | Prepared and bound 10,000 distinct targets with original banks; the fully active control executes every target three times per run. Saves retained common meaning but keeps qualification traffic and per-mapping reservations. Preserve this truthful distinction. |
| Binding-specific checked operands / normalized common preparation | Could address repeated TD interpretation and artifact cardinality; native lowering itself is below 1% here. No prototype or safety/equivalence claim is made. It needs a concrete bounded qualification/ownership design, not a mandatory universal program or cache. |
| Size existing slots for actual concurrency | Measured 555.52 MB idle saving at 10,000 instances. Adequate for this one-call-per-instance workload. It does not supply the 32-call promise of the larger configuration. |
| Box/pool/chunk runtime records or use sparse account storage | Could reduce vacant-slot and dense-bank costs, but adds allocation, progress or quota-domain trade-offs. No deployment budget or tested operation mix demonstrates those new lifetime/failure mechanisms are needed for this path. Keep the current layout provisional. |
| Use one common account | Measured large bank-memory saving, with a different quota contract and a demonstrated 896-mapping preparation boundary. Preserve independent accounts when independent quotas are required. |
| Release native occupancy with exchange retirement | Implemented and falsified against the baseline. Removes indefinite sibling blockage while retaining original permission/result/funding and one native exchange. |
| Add a universal scheduler/cache, per-Thing connections or larger quotas | Not justified. The explicit workload queue, native capacity, retained artifacts and original funding already explain these observations. |

This leaves a usable scoped execution path, not proof of an optimal hierarchy,
layout or native compiler model. Higher operation counts, mixed interaction
families, long-running churn and lifetime fragmentation, sustained fairness/tail
SLOs, network/device diversity, TLS/credential cost at population scale, larger or
structured TDs, portable/static profiles and hardware fit remain unproven.
Existing correctness evidence for those capabilities is not inherited fleet-scale
evidence. No gate, milestone or production qualification status changes. The PR
records executable validation revisions; the changed native policy still needs
independent acceptance under repository governance.
