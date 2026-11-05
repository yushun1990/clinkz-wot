# Consumer execution reuse characterization

Characterization on Linux x86_64, Rust 1.95.0, debug test build. This is scoped
implementation evidence for PREP-REUSE-001 and RES-OWNER-001, not independent
architecture acceptance, fleet throughput, production or profile qualification.
The PR identifies the validated source revision; the baseline is master `5ccd9ad`.

The [executable fixture](../../protocol-bindings/protocols/http/tests/reusable_execution.rs)
uses existing fully qualified Engine HTTP Producers. Each count has independent
loopback endpoints, actual registrations, logical authorities, policies and banks.
The retained readable schema has sixteen members and 1,035 bytes of vector/name
backing. Property Forms are empty in this characterization TD: its advertised
executable operation is the root read. Every endpoint performs one native HTTP
interaction, with complete Producer final-TD qualification and no leaf dispatch.

Run with:

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test reusable_execution -- --test-threads=1 --nocapture
```

One ordinary preparation constructs the common operation/data-role content;
subsequent preparation calls borrow it, compare complete supported assertions,
and compile distinct exact-target native artifacts. The preparation result counts actual content constructions. Pointer checks
cover the actual content, member vector and name backing. The separate control constructs
each operation independently. Content constructions are respectively 1 and N;
native compilation jobs are N in both cases. This is explicit preparation, with
no cache or post-preparation interning.

The test allocator records requested heap bytes and allocation calls on the test
thread. "Live" means allocated and not yet freed, including Arc/vector backing;
it excludes allocator metadata/fragmentation, RSS, stack and SDK/OS buffers.
External account-bank provisioning, TD/peer storage and measurement vectors are
outside these deltas. Preparation includes retained mappings/services and common
content; instance deltas include authority, policy, binding, runtime and vacant
call-slot storage. Admitted-call deltas are measured before native progress.

| Instances | Shared preparation live bytes | Separate preparation live bytes | Instance live bytes | Admitted call live bytes | Shared preparation reserved bytes | Instance reserved bytes | Active instance reserved bytes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 3,716 | 3,716 | 2,352 | 2,745 | 13,180 | 2,544 | 82,824 |
| 2 | 6,269 | 7,432 | 4,704 | 5,490 | 25,109 | 5,088 | 165,648 |
| 4 | 11,375 | 14,864 | 9,408 | 11,012 | 48,967 | 10,176 | 331,296 |
| 8 | 21,587 | 29,728 | 18,816 | 22,056 | 96,683 | 20,352 | 662,592 |
| 16 | 42,011 | 59,456 | 37,632 | 44,144 | 192,115 | 40,704 | 1,325,184 |
| 32 | 82,859 | 118,912 | 75,264 | 88,320 | 382,979 | 81,408 | 2,650,368 |

At 32 instances, sharing removes 36,053 requested retained heap bytes versus the
separate control: 31 copies of the 1,163-byte common physical owner, including its
1,035-byte member/name backing. The observed specialized mapping/service cost is
2,553 retained bytes per endpoint; the measured instance increment is 2,352 bytes
and 13 allocation calls per instance. These are observations for this supported
layout, not stable ABI sizes or process-wide memory bounds.

Preparation still repeats Basic/semantic admission, borrowed compatibility
checking, support queries and native compilation for every endpoint. At N=32 the
shared run made 13,715 allocation/reallocation calls requesting 4,464,139 bytes
cumulatively; the separate control made 14,304 calls requesting 4,500,192 bytes.
Cumulative requested bytes are allocation traffic, not peak or retained memory.
One initial preparation consumed work credits
`[119,1882,0,0,6,513,0,7,28,0,3650,317]` in `WorkClass::ALL` order;
one reused endpoint consumed `[119,1882,0,0,6,513,0,7,11,0,3650,1370]`.
The borrowed comparison has an explicit conservative PlanningItems allowance;
common-content destruction is prepaid only on construction. Sharing does not
eliminate qualification work. Debug elapsed times are printed for reproduction;
a single run and these work credits establish no latency or throughput gain.

Reservation columns are allowances, not additional allocated bytes. Active
instance reservation includes native/result/scratch/cleanup guarantees and is
about 82.8 KB per instance here, substantially larger than admitted live heap.
Counts use separate actual registrations, respecting HTTP's one-outstanding-call
policy through settlement disposal. No connection pool or concurrency change is
claimed. Semantic content introduces no executor or shared-backing pointer; its current
coordinate/schema types keep their existing gates. This execution representation
remains Host/std with pointer atomics.

Every count performs real I/O, retires preparation/runtime/authority roots while
keeping unread results, verifies original backing remains charged, then disposes
of results and verifies every account returns to zero and the common owner is
gone. Other fixture tests exercise independent permission/revocation/replacement,
equal numeric slots with different authority issuers, changed mapping facts,
foreign providers/configuration, unavailable or changed validators, stale
authority, saturated binding refusal, cancellation and retained late native output.
Shared registration/data backing also retains its first bank across separately
funded mappings. Existing HTTP and external owned/incremental compiler regressions
continue to supply the broader finite interaction/cleanup evidence.

Reuse qualification currently supports ReadAllProperties meaning. Other finite
operations keep ordinary preparation and the corrected handoff; continuing
Actions, notifications and Producer reuse are separate increments. Existing
JSON/type-only, endpoint NoSec, fixed bounds and cooperative Host execution limits
continue. General DataSchema/security, arbitrary first-use compilation after
source retirement, native retargeting, automatic caching and broad scalability
are not established by this evidence.
