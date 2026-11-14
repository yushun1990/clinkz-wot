# Consumer execution reuse characterization

This records the first content-reuse increment and the later
[explicit HTTP compilation increment](#explicit-http-definition-compilation). The subsequent
[multi-instance scalability workload](consumer-multi-instance-scaling.md)
extends measurement to 10,000 logical instances, controlled active fractions,
native capacity, quota topology, allocator/RSS behavior and sustained execution.
It also records the later HTTP occupancy correction. The historical numbers
below retain their original fixture and measurement scope.

PR #260 validates the **reusable-content/independent-instance boundary**. The
Owner accepts this implementation as a valid first increment. It does not
finalize the Engine's concrete type hierarchy, compilation model, runtime layout
or scaling architecture. The distinctions below separate existing architectural
invariants from replaceable implementation choices and unproven capabilities.

## Measured execution evidence

Characterization on Linux x86_64, Rust 1.95.0, debug test build. This is scoped
implementation evidence for PREP-REUSE-001 and RES-OWNER-001, not independent
architecture acceptance, fleet throughput, production or profile qualification.
The implementation assessed is `89b4359c286f08b78a674fa6af3c56091d62dfb3`;
the baseline is master `5ccd9ad`. Documentation updates do not extend the
implementation evidence to additional capabilities or workloads.

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
These historical counts used separate actual registrations and the original
one-outstanding-call policy through settlement disposal. The later occupancy
policy and its independent result lifetime are measured in the scalability
evidence linked above. Semantic content introduces no executor or shared-backing pointer; its current
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
funded mappings. The [service ownership regressions](../../protocol-bindings/protocols/http/tests/support/service_ownership.rs)
exercise distinct registrations with aliased data, access, or both services:
60,000 bytes of actual shared backing fit a 100,000-byte account alongside both
registrations, while equal but distinct service backing refuses. Each
registration retains its own native/reference costs. Partial refusal returns
the original aliased inputs and releases acquired reservations; real HTTP
results keep the shared services funded at their first bank through final
result disposal after the registry and both instances retire.
Existing HTTP and external owned/incremental compiler regressions
continue to supply the broader finite interaction/cleanup evidence.

Reuse qualification currently supports ReadAllProperties meaning. Other finite
operations keep ordinary preparation and the corrected handoff; continuing
Actions, notifications and Producer reuse are separate increments. Existing
JSON/type-only, endpoint NoSec, fixed bounds and cooperative Host execution limits
continue. General DataSchema/security, arbitrary first-use compilation after
source retirement, native retargeting, automatic caching and broad scalability
are not established by this evidence.

## Architectural invariants to preserve

These are applications of existing contracts, rather than new requirements or
an endorsement of the present Rust types:

- **Independent meaning, authority and accepted work.** Immutable prepared
  meaning can outlive its source and be shared without sharing current
  permission, revocation, call state or result ownership. Completed execution
  mappings retain no source TD, semantic arena or compiler callback. Each
  acceptance and successful handoff checks its original logical incarnation and
  actual services; replacement cannot redirect original or late work.
  [Planning](../spec/planning.md) and [runtime safety](../spec/runtime-safety.md)
  own these obligations.
- **Complete actual qualification.** Reuse establishes no permission or blanket
  support certificate. Selected Consumer interactions still need their complete
  schema, media, response, security, endpoint, configuration and service support.
  Producer publication still qualifies the entire final retained TD, behaviors,
  native readiness and collisions. Changed dependencies cannot be accepted from
  a matching name, hash or type. See [Planning](../spec/planning.md).
- **One funded physical owner, separately funded logical participation.** Shared
  backing keeps its original account until its last actual use and destruction;
  each instance funds its own references, operands, records and accepted work.
  Acquisition, cancellation, rollback and normal/late result disposal remain
  bounded and funded. Reservations remain distinct from physical RAM
  measurements. Independent registry or Servient boundaries do not relax
  [RES-OWNER-001 and RES-HANDOFF-001](../spec/foundation.md).
- **Distinct interaction semantics and native ownership.** Finite calls,
  continuing Action invocations/controls, notification cursors/replies and
  Producer publication keep their different lifecycles and effect certainty.
  A native collection interaction does not silently become leaf interactions.
  Native matching, correlation, connections and shared infrastructure remain
  Binding responsibilities; shared content confers no native shutdown authority.
  See [Core](../spec/interaction-core.md), [Binding SPI](../spec/binding-spi.md)
  and [runtime safety](../spec/runtime-safety.md).

## Explicit HTTP definition compilation

The [definition fixture](../../protocol-bindings/protocols/http/tests/support/definition_reuse.rs)
exercises actual cross-instance native compilation reuse through the owned
compiler interface. The [Binding contract](../spec/binding-spi.md#explicit-http-definition-compilation)
owns the single-input and resource rules. Run:

```sh
cargo test --locked -p clinkz-wot-binding-http --test reusable_execution definition_reuse -- --nocapture
cargo test --locked -p clinkz-wot-binding-http --test security_https compiled_request_reuse
```

### Architecture reassessment and correctness boundary

The first PR prototype (`add6cbd`) kept an independent semantic dependency visitor
beside HTTP lowering and reserved 10,040 bytes per definition, including an
8192-byte raw projection. Its tests established skipped lowering, but did not
justify that retained cost or prevent the two inventories drifting. That design
is replaced, rather than made an accepted maintenance obligation.

Fresh and reused preparation now call one `HttpCompiler::extract` to interpret
and qualify source facts. It constructs `HttpRequestInput`, a concrete private
HTTP type with operation, secure mode, native authentication placement, method,
borrowed header pairs and normalized response facts. The sole Consumer lowering
function accepts only this input. Its derived equality compares the newly
extracted input against the same type's owned retained value. A lowering change
cannot accidentally read an unrecorded TD field: source-dependent information
must pass through the input type, whose new fields enter equality automatically
and must be provided by exhaustive constructors. This is the code boundary
protecting reuse as support evolves, alongside normal private-policy, work,
storage and independent review obligations. There is no generic access recorder,
parallel dependency schema, hash identity or second TD interpretation.

Every source still incurs common TD admission/lending, semantic-content
construction or borrowed comparison, HTTP applicability/default/method selection,
response interpretation/support and security/service checks. Only header
validation/reserved-placement checks and packing, and response-table packing,
are skipped for compatible inputs. The bound destination and fresh registration,
policy, credentials, authority, operands, accepted work and cleanup stay separate.
The result is actual reuse of successful native lowering and physical instruction
backing, not avoidance of all protocol interpretation or all preparation work.

### Measured costs and benefit

Linux x86_64, Rust 1.95.0 debug characterization. N is the **total** mapping count,
K the number of distinct definitions (K <= N). Both controls use identical
sources/configuration and the same existing common semantic content. Account and
fixture provisioning and common-content setup are excluded equally. Timings,
allocation traffic and retained deltas include initial compilation of K source
mappings, creation of import capabilities and N-K checked bindings. Compilation
owners/counters are disposed before retained readings; the K physical definitions
and all N qualified mappings remain. The independent control freshly compiles N
mappings. Neither control executes I/O in this cost experiment.

| K | N | Shared requested heap | Independent requested heap | Shared capacity | Independent capacity |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 3,549 | 2,889 | 12,085 | 12,505 |
| 1 | 2 | 5,214 | 5,746 | 22,206 | 25,010 |
| 1 | 8 | 15,236 | 22,920 | 82,932 | 100,040 |
| 1 | 32 | 55,388 | 91,680 | 325,836 | 400,160 |
| 2 | 2 | 7,066 | 5,746 | 24,170 | 25,010 |
| 2 | 8 | 17,088 | 22,920 | 84,896 | 100,040 |
| 2 | 32 | 57,240 | 91,680 | 327,800 | 400,160 |

Requested heap is actual allocator Layout bytes, including vectors, names, Arc
and erased output slots; capacity is original account reservations. They are
not interchangeable: at K=N=1 sharing reserves slightly less but uses **660 more
heap bytes**. At K=1, N=32 it saves 36,292 heap bytes; K=2 saves 34,440. The fixture's
owned instruction/input definition reserves 1,964 bytes instead of 10,040; actual
input descendants determine this cost rather than a fixed maximum. Unused Form
metadata is not retained. These are measured layout/workload results, not stable
ABI sizes or a general break-even threshold.

Actual lowerings are K versus N. Reuse comparisons are N-K, total successful
bindings N. For the one-header/one-response fixture this avoids N-K header
syntax/placement validations and pack operations and N-K response table pack
operations; response interpretation and eligibility still run N times. At K=1,
N=32 the shared/control runs made 6188/6244 allocation calls requesting
2,657,328/2,697,516 bytes cumulatively. These are allocation traffic, not retained
or peak RAM. Full source-inclusive debug preparation remains roughly 29–31 ms
in both paths at N=32. There is no established preparation-time gain, statistical
latency result, warm-I/O improvement or fleet-throughput claim. Work credits bound
callbacks and must not be presented as measured CPU instructions saved.

The cost test checks the entire retained allocator delta returns to its initial
value after both controls release, in addition to every account reaching zero.
This covers actual physical release, rather than treating a smaller reservation
or shared Arc pointer as a RAM measurement.

### Behavioral evidence, alternatives and limitations

Public-path controls exercise changed headers/method/coding/media/subprotocol,
response status/success/media, schema, configuration and invalid targets. Prefix
aliases, moved Form coordinates, equivalent explicit defaults and large ignored
metadata reuse the same compilation. Changed supported header instructions
refuse reuse and recover through explicit fresh compilation. Real peers verify
POST/header instructions, percent-escaped paths/queries and additional HTTP 409
discrimination. Engine HTTP/TLS tests retain independent policy/credential and
authority changes, a real late native output, sibling progress and original
funding through the final result. Zero-credit, cancelled and capacity-refused
preparation retain bounded rollback and retry.

Alternatives were reassessed, not assumed inferior because the prototype exists:

- **Ordinary per-instance compilation:** simplest for unique definitions or
  small populations; it is a supported choice and wins retained heap at N=K in
  this fixture. It cannot alone provide the requested compile-once capability.
- **Shrink the serialized projection:** fixes some capacity waste but preserves
  an independently maintained list of compiler dependencies. Rejected.
- **Recompile and compare/intern native output:** avoids retaining input facts,
  but repeats header validation/packing and cannot prove compilation-work reuse.
- **One private normalized HTTP input:** selected. It replaces the visitor with
  data already needed by lowering and derived equality, with actual-sized owned
  backing. It needs no new public hierarchy or Engine protocol case.
- **Generic dependency-recording compiler, cache, or whole-TD IR:** could pursue
  broader reuse but adds machinery without evidence this small lowering is a
  startup bottleneck. It is unnecessary for the demonstrated memory benefit.
- **Share complete ConsumerDefinition:** conflates concrete endpoint/executor/
  service associations with immutable instructions; it does not preserve the
  required independent instance/dependency boundaries.

The remaining tradeoff is deliberate: owned normalized facts duplicate some
header/response content alongside packed instructions, and comparison/extraction
still cost work and temporary vectors. Both modes use that extractor, so these
CPU/allocation comparisons do not establish an improvement over the earlier
one-pass compiler. Keeping a second optimized TD parser would recreate the
dependency drift this change removes. Exact header order/spelling can refuse
semantically equivalent inputs. A fresh description is still required to bind a
new mapping; call execution is source-free. Runtime calls still copy inline
instructions, and their destination buffer remains part of that existing layout.
Continuing Actions, notifications, Producer definition sharing, other Bindings,
portable/static representations, caches, persistence and execution coordination
are separate capabilities. No package, architecture gate or milestone is closed;
independent acceptance must assess the exact updated PR revision.

## Provisional choices and unproven evolution

`ConsumerExecutionContent`, `PreparedConsumerContent`, `ConsumerDefinition` and
`ConsumerBinding` demonstrate responsibility and lifetime separation. Their
number, names, field placement and public hierarchy are provisional. So are
eager endpoint compilation, repeated admission, exact member-order comparison,
per-instance runtime objects, fixed capacities and Host ownership carriers.
Equivalent simpler representations may replace them while preserving the
invariants above.

Each possible direction below is **unproven**, not an accepted design or a
mandatory optimization. A functional need or measured bottleneck should justify
the corresponding experiment against the current implementation. Repetition
alone does not justify a universal compiler, cache, scheduler or runtime.
Correctness counterexamples still require preserving the existing invariants,
whether or not a proposed optimization saves memory or time.

### Endpoint-specific native compilation and instance operands

**Original evidence.** The first fixture constructs one common content owner but N
native artifacts for N endpoints. The
[HTTP artifact](../../protocol-bindings/protocols/http/src/program.rs) embeds the
complete target, method and headers. Real peers verify changed paths and headers;
the owned/incremental compiler path remains supported. Binding an already
qualified definition needs no compiler or TD.

**Provisional choice and limitation.** Exact-target specialization is safe for
the current mapping, but repeats lowering and retained instruction storage.
It proves neither checked retargeting nor a benefit from avoiding compilation.
The current artifact cannot acquire a new endpoint merely by changing its
envelope. Native compilation cost has not been isolated as a fleet bottleneck.

**Successor.** The explicit HTTP increment above separates reusable instructions
from checked destinations. Other Bindings and richer operand contracts remain
unproven. A concrete Binding can separate reusable
request instructions from explicitly checked endpoint or header operands, with
complete configuration compatibility, destination policy and operand funding.
Other Bindings may remain specialized. Program reuse would not by itself imply
connection pooling, increased native concurrency or shared logical authority.

**Justifying workload/experiment.** Provision hundreds or thousands of devices
with the same operation/mapping shape and different endpoints, then measure
native lowering time and retained artifact bytes separately from common
qualification. Compare specialization with a Binding-local operand prototype
using real wire requests, changed paths/headers, incompatible configurations,
denied destinations and retained late results. Pursue reuse if it addresses a
measured provisioning/storage constraint without weakening those checks.

### Repeated TD admission and reusable normalized preparation

**Current evidence.**
[`prepare_with_content`](../../planning/src/open_consumer.rs) still performs
Basic admission, the common semantic walk, borrowed assertion comparison and
actual provider/service qualification for every endpoint. It avoids constructing
another retained member/name table. The work/allocation measurements above do
not establish a preparation latency gain. Warm execution and source-free binding
already avoid TD scans.

**Provisional choice and limitation.** Reuse of retained output is narrower than
reuse of all preparation work. New endpoint mappings still require a source
`Thing` during preparation; destruction of the source after preparation does
not prove arbitrary new mapping construction after source retirement. Exact
member order is also a current comparison rule, not proof that every differently
ordered description has different WoT meaning.

**Possible evolution, unproven.** Retain bounded normalized common meaning and
the resolved Form facts needed by a demonstrated use case. Reuse unchanged
admission/defaulting while qualifying the actual endpoint, concrete Binding,
services and mutable dependencies separately. Unretained or changed assertions
would still require admission or refusal. This need not introduce a universal
compiler input format or an automatic cache.

**Justifying workload/experiment.** Use large descriptions whose instances vary
only in endpoint facts, and independently vary member/schema/security/extension
meaning in negative controls. Attribute time, work credits, temporary allocation
and retained backing to admission, semantic resolution, support checks and
native lowering. Compare a bounded normalized-input prototype, including
construction after source retirement, only if repeated common interpretation
materially limits the intended provisioning workload. Check that any accepted
reordering preserves every member and data role.

### Per-Consumer runtime overhead and multi-instance execution

**Current evidence.** The measured layout uses 2,352 requested live heap bytes
and 13 allocation calls per instance, including authority, policy association,
binding, runtime and vacant call storage. External bank provisioning is excluded.
The [finite runtime](../../servient/finite/src/consumer.rs) owns an `Rc<State>`,
per-instance definition entries and a prepaid slot vector. Evidence covers
1–32 instances with manually driven I/O and bounded native concurrency. The
approximately 82.8 KB active reservation per instance is an allowance, not RAM.

**Provisional choice and limitation.** One runtime object and provisioned slot
storage per Consumer may be expensive for many mostly idle Things or many
operations per Thing. The current fixed per-runtime bounds are implementation
choices, not a demonstrated Engine-wide scaling limit. These measurements do
not establish sustained throughput, fairness, tail latency or a need for pooling.

**Possible evolution, unproven.** First compare appropriate slot sizing and
reference/record layout. If necessary, a Servient could hold chunked or pooled
instance/call records while retaining independent authority, original accounts
and result/cleanup owners. Shared native progress remains a separate Binding
decision; compact logical records do not require a universal scheduler or a
common lifecycle for all interactions.

**Justifying workload/experiment.** Exercise thousands of mostly idle instances,
several operation counts and a controlled active fraction under the same actual
native capacity. Include bank provisioning, idle/active heap, allocator traffic,
CPU/poll cost, throughput, tail latency, progress gaps, replacement and final
release under saturation. Compare a simpler layout only if idle storage or
execution overhead exceeds a real deployment budget; keep logical allowances
separate from physical measurements.

### Physical service ownership across registries and Servients

**Reproduced limitation.** At the post-#266 baseline, independently constructed
registries given the same raw service reserve 69,152 bytes **each** for one
60,000-byte backing. The characterization checkpoint `3b98056` tests shared data,
shared access and both, using one bank and separate banks. A second construction
under a common 100,000-byte limit refuses admission and rolls back. This is
observable duplicate accounting and false capacity refusal, not a native I/O or
logical-authority failure. #260 explicitly left this composition unproven; that
evidence limit did not exempt shared backing from RES-OWNER-001/RES-HANDOFF-001.

**Supported correction.** A composition root adopts each actual service once
with `ConsumerServiceOwner::retain_data` / `retain_access` and supplies clones of
that retained owner to independent `ConsumerRegistry::try_from_retained` calls.
`RetainedConsumerRegistrationInput` keeps preparation, data and access ownership
explicit. Different owners may originate at different banks; imports do not move
or reserve their backing again. Each registry funds its local records/references
and actual native registration. Instance, mapping, call/result and cleanup costs
remain independently funded. Raw `try_new` remains local startup adoption; using
it to adopt aliases in separate constructions is explicitly outside this shared
composition contract. No global pointer index, configuration-based matching,
universal manager or Engine-native shutdown right is added.
The earlier preparation/allocation measurements remain historical at their named
revision; the service-account figures below come from this successor fixture.

The [service regressions](../../protocol-bindings/protocols/http/tests/support/service_ownership.rs)
exercise same/separate banks and both retirement orders for data-only,
access-only and combined 60,000-byte backing. Equal but distinct services keep
separate owners, even with equivalent configuration. Registries can combine data
and access from different original banks. Zero-credit adoption returns its last
raw input for retry. Zero-credit and partial registry refusal return original
retained inputs and roll back only new local acquisitions; retry succeeds at an
adequate bank. Changed declared backing refuses import without refinancing it.

On the x86-64 Host fixture, the retained shared services reserve **60,632 bytes**
(including their object/owner envelopes), and each registry reserves **9,384 bytes**:
**848** for registry-only records and **8,536** for the actual native registration.
These are declared reservation counters, not additive measured process RAM.
The additional destructor observers are fixture instrumentation; the pre-change
raw reproduction above predates them and the new registry record charge.

The real Engine HTTP test uses independently constructed registries and native
registrations, separate Consumer authorities at equal numeric slots, and separate
instance banks. It retains a normal result and an actual returned HTTP output
awaiting validation, then cancels and/or retires the latter's original authority.
The late result refuses application success while keeping the original native
outcome. Exact startup-account snapshots are asserted at each transition:

| Transition | One common startup bank | Original bank / second startup bank |
| --- | ---: | ---: |
| Both registries admitted | 79,400 | 70,016 / 9,384 |
| Registries retired; mappings/results retained | 77,704 | 69,168 / 8,536 |
| Normal result disposed; late result retained | 69,168 | 60,632 / 8,536 |
| Late result disposed first; normal retained | 69,168 | 69,168 / 0 |
| Final result disposed | 0 | 0 / 0 |

Every resource account in both startup banks and both instance banks returns to
zero. Actual data/access service destructors run once each while their original
owner reservations are still present, then the final leases release. This is
resource-accounting and actual-disposal evidence alongside identity checks,
rather than an inference from Arc strong counts.

The [HTTPS regression](../../protocol-bindings/protocols/http/tests/security_https.rs)
imports one shared data owner into two independently constructed registries with
separate native trust/registration funding and independent access owners,
credentials, targets, policies, revisions and instance banks. Distinct real TLS
peers require different passwords. Changing A's credentials/policy and retiring
A's authority leaves B's revision and successful execution independent; B's policy
change fences B's subsequent calls without changing A's revision. Already
handed-off completions retain their original settled meaning. Original service
funding survives the registries and instances through the final result.

**Remaining limitations.** This is explicit Host finite Consumer composition,
not automatic deduplication of separately adopted raw aliases or different
service objects with shared internal suballocations. Concrete services own those
internal declarations/limits. It does not add retained-owner import to the
separate continuing Action, notification or Producer registries, nor prove
portable/static ownership, general cross-process accounting, native session
sharing, aggregate OS containment or arbitrary tenant/trust compatibility.
Physical retention never grants credential, authorization or interaction rights.

### Richer DataSchema, continuing Actions, notifications and Producers

**Current evidence.** Borrowed semantic reuse is implemented only for
ReadAllProperties with exact supported members/roles, JSON/type-only validation
and endpoint NoSec. Other finite operations use the corrected handoff with
ordinary preparation. Existing real HTTP Actions, MQTT notifications and
qualified Producer paths establish their own interaction behavior, not reuse of
their prepared content across independent instances.

**Provisional choice and limitation.** The finite schema enum and current
member comparison do not prove reusable compiled DataSchema constraints,
security requirements or all operation families. Continuing native tokens,
notification cursors/replies and Producer behavior/publication cannot simply
become fields of one shared finite definition. Consumer selected support also
cannot substitute for complete final-TD Producer qualification.

**Possible evolution, unproven.** Extend eligible immutable schema/data-role and
security meaning through one useful operation at a time, retaining actual
validator support and independent runtime credentials/permission. Preserve
separate invocation, control, conversation and publication owners. Replace the
present carriers if those lifetimes expose a better boundary; no universal
program or common interaction state machine follows from reuse.

**Justifying workload/experiment.** A read/write workload with nested required
fields and numeric bounds can compare shared constraint preparation across two
endpoints with changed validators and incompatible input/output assertions. A two-instance continuing
Action can test distinct native tokens, query/cancel and retained late output.
A shared native notification subscription can test independent cursors, stop,
loss and any supported reply rights. Two Producers sharing eligible meaning
can return different behavior results while independently qualifying every final
Form, service, readiness and publication dependency. Choose the experiment that
delivers a required capability or exposes a concrete ownership failure; existing
family I/O alone does not establish any of these reuse claims.

### Host ownership representations and portable/static execution

**Current evidence.** This path uses `Arc` retained owners, `Rc`/`RefCell`
runtime state, vectors, Host resource accounts and cooperative manual progress.
The [Consumer Core module](../../core/src/lib.rs) requires `std` and pointer
atomics. Owned/incremental compiler support and other static/profile fixtures do
not establish this complete execution path on static, allocation-free or
non-atomic targets.

**Provisional choice and limitation.** Atomic reference counting, heap ownership,
interior mutability, current fixed capacities and single-thread runtime layout
are Host choices. They may impede constrained use, but neither their replacement
cost nor an equivalent portable implementation is demonstrated. Linear work
credits do not prove preemption, hard real-time behavior or recoverable Host OOM.

**Possible evolution, unproven.** Compare caller-owned arenas, borrowed immutable
prepared tables, static storage or generation-checked handles with explicit
capacity and lifetime owners. A single-thread or allocation-capable profile can
choose different carriers from a concurrent Host profile. Preserve source
independence, checked associations, bounded progress and original funding rather
than requiring the same public pointer hierarchy everywhere.

**Justifying workload/experiment.** A concrete fixed-memory deployment should
name its supported operations and target budget. First exercise two independently
revocable instances with caller-owned/static backing and real test-Binding I/O;
then build and run the selected subset on a chosen `no_std` target without
pointer atomics. Measure storage/stack, allocation behavior, work/cleanup bounds
and retained late-result lifetime under exhaustion. Qualify the actual target's
native backend separately before claiming embedded protocol support. Port the
needed capability, rather than redesigning every Host path in anticipation.
