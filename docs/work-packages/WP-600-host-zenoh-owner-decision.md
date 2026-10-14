# Host Zenoh protocol-owner decision

Decision candidate for independent review, reconstructed from master
`0e3cafd52207e55327ef978cebf46e8d10092a34`, including the unresolved qualification
boundaries of [PR #175](https://github.com/yushun1990/clinkz-wot/pull/175).
This record owns the bounded WP-600 direction and prerequisite decision. It does
not register or admit a tranche, accept a new API, change a gate, or claim
production completion.

## Conclusion

**The unchanged stable Zenoh 1.9.0 SDK on the native System allocator is not an
admissible in-process Host Consumer backend under the accepted contract.** A
worker or K-credit epoch does not supply recoverable SDK backing. Fresh fault
injection aborts the process during SDK query submission on a warmed protocol
worker, even with its callback and builder already constructed. Transport
configuration also does not supply the assumed input/retained-byte ceiling. An actual original
close timeout leaves live callback work behind a successful retry and final Drop.

Select a **dependency qualification prerequisite**, preserving the existing
ownership, recovery and resource guarantees. Stop extending the separate-owner
experiment against this unchanged SDK. Prepare a bounded SDK correction or
replacement candidate satisfying the contract below before a production adapter
or shared Runtime change. No dependency change is implemented or admitted here.
If that prerequisite cannot be met at proportionate cost, process isolation or
an explicitly weaker deployment contract requires its own architecture decision;
neither is implicitly selected by this finding.

PR #175's concrete worker with separate Consumer epochs remains a possible
topology after the prerequisite, not the required or qualified solution. Retain
the distinction between semantic calls and physical infrastructure, original
accounts, Core result seals, selected requests, generation leases and Servient
orchestration. A generic registration-retirement SPI is **not justified by these
SDK failures**. The existing public engine boundary's ability to capture a
complete qualified infrastructure owner remains unproved, rather than disproved.

The necessary shared accounting/destructor interpretation is projected into
[Binding SPI](../spec/binding-spi.md#shared-protocol-infrastructure-lifetime)
and [Runtime lifecycle](../architecture/50-servient-runtime-lifecycle.md#consumed-thing-lifecycle).
Existing ADR-0006, ADR-0011 and ADR-0015 supply the rationale and ownership rules;
no new package boundary, shared lifecycle API or accepted-ADR replacement is
selected here.

## Evidence and problem framing

All ten Consumer gate predecessors remain accepted. The gate proves a finite
mock author, while the current Zenoh production client retains legacy TD/Form
execution, synchronous transport and unqualified allocation. The original
[0083 observation](../../workspace/0083-host-zenoh-consumer-query-ownership.md)
and its K-credit epoch discriminator distinguish receiver completion from
session timer retirement. They do not qualify shutdown or SDK physical maxima.

Current [ConsumerPropertyReadRuntime::shutdown](../../servient/src/consumer_property_read/host.rs)
destroys the moved registration
after call/generation reclamation; last-root Drop reports residual before finite,
allocation-free local discard. There is no asynchronous registration-retirement
hook. Compiler support and a numeric author declaration do not change that code.

The pinned Zenoh 1.9.0
[Session implementation](https://github.com/eclipse-zenoh/zenoh/blob/81c6c933b6e41d72a05f04c4442ef57717ddc72b/zenoh/src/api/session.rs)
and [close builder](https://github.com/eclipse-zenoh/zenoh/blob/81c6c933b6e41d72a05f04c4442ef57717ddc72b/zenoh/src/api/builders/close.rs)
make this a **physical-owner** question, rather
than a choice between async spelling and polling:

- `Session::drop` invokes `close().wait()` on the last strong session.
- `CloseBuilder::into_future` allocates a boxed future even after session close;
  its asynchronous close body also uses boxed state.
- First close takes primitives before waiting for task termination. Its default
  ten-second timeout can abandon that original join; retry success and
  `is_closed()` are not a replacement barrier.
- Default close does not wait for all callbacks. A synchronous get job can
  remain inside its callback after first close succeeds.

The [public-SDK fixture](../../tools/architecture-fixtures/host-zenoh-owner/README.md)
exercises blocking/allocating Drop, original-close abandonment and timeout,
get/callback quiescence, worker query-allocation failure, TCP receive-limit
semantics and an actual original Foundation account.
No engine request or registration is fabricated, and no production adapter is
implemented. Its scoped role lease is a model, not a public Runtime proof.

Topic 0076's applicable principle is independently deployable capabilities with
different physical owners. A complete Consumer registration need not own a
Producer, Directory, or WoT executor. Conversely, moving protocol work to a worker
does not remove it from resource or shutdown ownership. Its broad Contracts,
generated-image and Host/embedded package proposals supply no remedy for these
specific SDK facts. The existing role-only registration and unique output bridge
already remove their earlier construction impediments.

The affected active requirements are `BIND-REG-001`, `BIND-MEM-001`,
`BIND-CALL-CANCEL-001` and `BIND-HOST-CANCEL-001`, owned by Binding SPI.
`BIND-OUT-001`/`BIND-IO-001` and the accepted resource, first-cause, seal and
publication contracts are preserved. This interpretation adds no public API
family, trace-machine state or resource field/default. Exact representation and
projection changes, if needed, belong to the later scoped admission record.

## Ownership constraints on any successor

The engine owns selection, publication, first cause, deadline policy, call/plan
leases, normal/late result sealing and terminal semantic acknowledgement. The
protocol owner owns SDK sessions, query tails, callbacks, get/open/close jobs,
bounded mailboxes and their physical backing. If a worker is used, it can perform
protocol I/O and wake an engine driver; it receives no TD, handler, selection or
publication authority. Compiler work stays side-effect free and separate from
protocol startup.

If reusable epochs are selected, one finite epoch starts with K submission
credits. Debit before SDK get, including an attempt whose SDK start fails after
it may have created work. Never refund on reply, timeout, cancellation or an
error without demonstrated no-effect rejection.
At K, return unchanged input before submission. Rotation closes admission,
quiesces jobs/callbacks, fully drives the **original** close and physically
destroys its SDK owner before replenishing credits. Permit no old/new epoch
overlap in the first candidate. K=32 in historical experiments is not a selected
production limit or byte ceiling. K=1 remains a valid limiting candidate for this
same separate-owner topology; connection churn is not a measured latency verdict.

Infrastructure and call admission are separate: SDK tail capacity can stay held
after the one Runtime call/result opportunity is released. Infrastructure must
have a real pre-admitted lifetime maximum and original local/parent accounts;
bounded references are charged separately. Neither an application baseline label,
a synthetic receipt, a global SDK thread pool nor a spare numeric allowance is an
owner. Source audit and qualification must demonstrate actual capture and
child-before-parent release, including the Runtime's registration destruction.

Normal shutdown must close new protocol admission before retiring old work. A
role lease's destructor must only perform finite local release or notify an
already provisioned owner. It must not drop the last Session, join a thread,
start close, allocate, spawn work or release capacity still protecting SDK work.
Notification alone is not cleanup acknowledgement. Before root abandonment
destroys a lease, the existing durable Runtime report remains mandatory; any
independently surviving protocol obligation also needs its already backed owner
and truthful status. Do not report successful deployment shutdown merely because
a semantic Runtime or worker handle was destroyed.

Keep Consumer and Producer SDK session ownership separate for a successor's first
production candidate. That is a bounded qualification choice, not a permanent
role/session rule.
Future shared transport needs explicit compatible ownership, trust/configuration,
last-user and capacity evidence; a Consumer cancel/rotation must never close live
Producer work. Producer and Consumer capabilities remain independently
constructible, and adding both does not confer shared-session shutdown authority.

## Resource feasibility and stopping evidence

The new cases ran against the unchanged lock on native x86-64 Linux, Rust 1.99.0
and System allocation. Cached archives for zenoh, zenoh-config, zenoh-transport,
zenoh-task, zenoh-runtime and zenoh-sync were verified against Cargo.lock;
the twelve inspected source files matched their archive contents.

| Boundary | Executable finding and source-backed consequence |
| --- | --- |
| Recoverable worker submission | `query-start` warms one submission, then makes allocation requests after callback and next-builder construction. `query-start-oom` returns null for the next allocation on that worker and gets SIGABRT, not an SDK error or owned rejection. `Session::query` creates cancellation/task/query backing with infallible allocation. A fallible outer call box or `catch_unwind` cannot recover an allocator abort. |
| Physical input versus message setting | `rx-limits` uses one TCP client session/link, a 65,535-byte batch/buffer setting, no consolidation and a finite 16-item FIFO. A configured `max_message_size=1024` still accepts 8,192-byte replies. [Universal RX](https://github.com/eclipse-zenoh/zenoh/blob/81c6c933b6e41d72a05f04c4442ef57717ddc72b/io/zenoh-transport/src/unicast/universal/rx.rs) applies that setting to defragmentation, not the unfragmented frame path. |
| Pool setting versus retained storage | The same case retains 131,072 payload bytes in actual SDK replies, exceeding `buffer_size=65535`. [RX buffer acquisition](https://github.com/eclipse-zenoh/zenoh/blob/81c6c933b6e41d72a05f04c4442ef57717ddc72b/io/zenoh-transport/src/unicast/universal/link.rs) uses `try_take().unwrap_or_else(|| pool.alloc())`; an empty pool allocates additional backing. This is payload length, not total allocation capacity or a measured peak. |
| Original timeout versus retirement | `original-close-timeout` drives the SDK's actual ten-second timeout with a separate Tokio clock. The first close returns Err; retry returns Ok and final Session Drop returns while the held timeout callback is still live. The callback is then released. The original task wait was abandoned; flags, retry or Drop cannot certify its completion. |

These falsify the proposed shortcuts, not every conceivable finite SDK envelope.
A smaller negotiated batch, finite queues, immediate nonblocking callbacks and
an audited aggregate bound could contribute to such an envelope. The retention
case deliberately holds a finite set of replies; it does not demonstrate an
unbounded queue or force a production client to retain them. Callback rejection
still occurs after SDK reception/decoding, and pool sizing is not prebacking.
The public SDK has no per-owner fallible allocator/reservation interface that
turns the demonstrated submission/destructor aborts into supported recovery.
Physical admission numbers alone cannot provision that backing.

Source inspection also finds infallible remote expression/routing table growth
before application callbacks (`SessionState::remote_key_to_expr`,
`WeakSession::send_declare`, routing `register_expr`), session task storage and
process-wide Zenoh runtime pools. Peer/link, fragment and TX queue limits are
useful distinct controls; they do not by themselves qualify all these bytes,
largest allocations, work or original owner accounts. No universal memory ceiling,
unbounded-growth theorem or production default is inferred from these probes.

## Alternatives and decision

| Alternative | Disposition |
| --- | --- |
| Registration directly owns reusable Session, closes on Drop | Rejected for this Runtime: the reproducible last-session destructor blocks, allocates even after close, and aborts on controlled allocation failure. Epoch credits solve timer count, not this destructor boundary. |
| Each call directly owns its own Session, K=1 | Direct SDK ownership retains the final-destructor defect. Per-read sessions managed by the separate owner remain credible; they add open/close cycles whose cost needs measurement, not a different correctness rule. |
| Dedicated bounded protocol owner, thin role/call leases, finite Consumer epochs | PR #175's topology hypothesis. Useful execution/lifetime separation, but worker query submission also aborts and input limits do not preback SDK storage. Stop qualification against the unchanged native-System SDK; retain the option after the dependency prerequisite. |
| Application supplies an existing Session or unaccounted supervisor | Rejected as qualification. Borrowed/shared SDK state cannot confer exclusive close authority or prove original-account retention. An independently admitted, fully captured owner is a different candidate and must satisfy the same obligations. |
| Generic Core/Servient registration shutdown API now | Deferred. It may become necessary if a qualified full-owner discriminator demonstrates an unconstructible capture or original-account transfer. It cannot fix allocation abort or pre-callback SDK growth. No dummy server or empty final call should conceal that discovery. |
| Bounded SDK correction or replacement | Selected prerequisite. Qualify recoverable/prebacked storage, pre-callback physical limits and truthful whole-owner retirement before choosing engine integration. Per-query retirement may replace epoch tails but is not mandatory. Enabling unstable cancellation alone is insufficient. |
| Process-isolated SDK with bounded IPC and enforced OS memory limits | Credible alternative: contain child allocator abort and use acknowledged process exit/reaping as a physical release barrier. Requires a complete charged supervisor/reaper, bounded IPC/input/work, final-controller-loss proof and an explicit deployment/accounting contract. It can preserve the SDK pin but adds a process/IPC boundary. Its cost must be compared with the SDK correction; no process adapter or architecture change is admitted. |
| Same SDK with documented fatal OOM, trusted peer or best-effort cleanup | Not an admission path under accepted guarantees. Trusted peers can narrow input traffic but do not supply allocator backing or terminal retirement. A weaker contract would be an explicit architecture/product decision, not a binding implementation detail. |
| Contracts extraction, new runtime packages, universal scheduler/allocator, compatibility layer | Not justified by this boundary. Share accepted semantics and replace obsolete target edges directly under pre-release policy; do not duplicate planners or seals. |

## Precisely scoped prerequisite and completion boundary

The next engineering boundary is a non-production **SDK resource/retirement
qualification candidate**, confined to this Consumer TCP one-shot. It must supply:

1. A source-backed allocation contract for open, submission, receive/decode,
   query tasks, callbacks and close/final destruction: either recoverable failure
   retaining every accepted owner or actual finite prebacking. Account peak bytes,
   actual capacities, largest allocation and stack/thread/runtime storage.
   SDK allocation-failure tests must recover without process abort; do not count
   an outer fallible engine envelope as SDK backing.
2. Enforced pre-callback physical limits for the chosen transport/configuration,
   including both unfragmented and fragmented messages, held receive batches,
   payload/metadata, protocol expression/control tables, peers/links, queues and
   jobs. Test exact limits and one over with real external input. Keep finite
   callback work/nonblocking overflow separate from physical-byte accounting.
3. An observable whole-owner retirement barrier covering SDK jobs/tasks/callbacks
   and final destruction, including original close timeout/abandonment. Retrying
   a closed session cannot substitute for the original barrier. Per-query joins
   or bounded whole-epoch retirement are alternative mechanisms. A terminal
   SDK handle's local Drop must be finite and allocation-free on the engine path.

These are dependency eligibility conditions, not an instruction to implement a
universal allocator, fix all Zenoh protocols or measure an unimplemented production
adapter. A candidate may exclude unrelated SDK features/configurations explicitly.
No specific new SDK version or patch is endorsed without these passing probes and
source-backed bounds. If no proportionate candidate meets them, make the explicit
dependency/isolation/unsupported-deployment decision instead of adding wrappers.

Only then qualify the **complete final owner** at the actual public registration
boundary. Current Runtime charges include registration backing and are released
after registration Drop; they are not a transferable SDK account. A surviving
worker must carry a separately admitted physical owner and its real original
local/parent accounts, with only reference storage released by the Runtime.
Demonstrate explicit shutdown and final Runtime/role/controller loss, blocked work,
saturation and loss of any residual successor. The fixture's joined worker and
single cleanup item do not establish this. A retained Session clone, detached
thread, record or duplicated numeric charge is not the complete successor.

If that discriminator proves existing capture/account/lifecycle authority
unconstructible, correct only the demonstrated Core/Servient prerequisite under
its own admission. No such shared incompatibility has been demonstrated here.
This sequencing stops owner-model work that cannot remedy the present SDK failure
without pretending the later integration question is already resolved.

After both prerequisites close, prepare the existing ADR-0013 Host production
candidate with the six prospective keys already identified by 0083, exact
prechecks, source removal paths and independent admission. Its production boundary
remains one complete Consumer registration, one eager generation/call opportunity, two exact
selected property targets, repeated reads and a production Producer TCP loop.
Subscriptions, pico parity, reusable plan sharing, broad package closure and a
new architecture gate remain outside this decision.

The passed gates, completed predecessor evidence, 65 requirement identities,
resource defaults, performance identities and PLAN are reaffirmed within their
existing scopes. The new negative SDK evidence narrows concrete author eligibility;
it falsifies no demonstrated mock or Producer carrier invariant. No existing
admission or gate status is changed by the author.
