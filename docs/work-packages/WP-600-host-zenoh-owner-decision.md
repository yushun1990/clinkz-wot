# Host Zenoh protocol-owner decision

Decision candidate for independent review, reconstructed from master
`0dbf0bbc0e144b5dc2695ecf6e66ace77bbb67fd`. This record owns the bounded WP-600
direction and remaining admission blocker. It does not register or admit a
tranche, accept a new API, change a gate, or claim production completion.

## Conclusion

Separate a semantic call's lifetime from the concrete protocol infrastructure
that services it. For the first Host candidate, pursue one bounded concrete
protocol owner with an exclusive reusable **Consumer** session epoch and
nonblocking role/call leases. Execute synchronous SDK opening, submission,
close and final SDK destruction on that owner's bounded protocol worker, outside
engine polls, locks and destructors. Retain the original infrastructure accounts
independently of terminal semantic calls. Keep the existing Core result seal,
selected request, generation leases and Servient orchestration.

This selects an owner topology and release contract for qualification, not a
qualified worker implementation. The scoped-worker experiment below proves only
explicitly joined retirement. Safe loss of its final owner and recoverable SDK
backing remain genuine admission blockers. A generic shared registration-lifecycle
SPI is **not yet justified**: neither raw SDK Drop's failure nor a scoped worker's
success proves that the existing public engine boundary can or cannot capture the
complete qualified owner. Do not implement WP-600 production or a shared SPI
change from this record.

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
exercises blocking/allocating Drop, original-close abandonment, get/callback
quiescence and an actual original Foundation account. The internal timeout is
source evidence, not a timeout case executed by that fixture.
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

## Concrete candidate boundary

The engine owns selection, publication, first cause, deadline policy, call/plan
leases, normal/late result sealing and terminal semantic acknowledgement. The
protocol owner owns SDK sessions, query tails, callbacks, get/open/close jobs,
bounded mailboxes and their physical backing. Its worker can perform protocol I/O
and wake an engine driver; it receives no TD, handler, selection or publication
authority. Compiler work stays side-effect free and separate from worker startup.

One finite epoch starts with K submission credits. Debit before SDK get, including
an attempt whose SDK start fails after it may have created work. Never refund on
reply, timeout, cancellation or an error without demonstrated no-effect rejection.
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

Keep Consumer and Producer SDK session ownership separate in the first production
loop. That is a bounded qualification choice, not a permanent role/session rule.
Future shared transport needs explicit compatible ownership, trust/configuration,
last-user and capacity evidence; a Consumer cancel/rotation must never close live
Producer work. Producer and Consumer capabilities remain independently
constructible, and adding both does not confer shared-session shutdown authority.

## Alternatives

| Alternative | Disposition |
| --- | --- |
| Registration directly owns reusable Session, closes on Drop | Rejected for this Runtime: the reproducible last-session destructor blocks, allocates even after close, and aborts on controlled allocation failure. Epoch credits solve timer count, not this destructor boundary. |
| Each call directly owns its own Session, K=1 | Direct SDK ownership retains the final-destructor defect. Per-read sessions managed by the separate owner remain credible; they add open/close cycles whose cost needs measurement, not a different correctness rule. |
| Dedicated bounded protocol owner, thin role/call leases, finite Consumer epochs | Selected qualification direction. It separates protocol execution from engine progress without altering shared semantics. Explicit scoped retirement is demonstrated; final-owner loss, full byte bounds and recoverability are unproved. |
| Application supplies an existing Session or unaccounted supervisor | Rejected as qualification. Borrowed/shared SDK state cannot confer exclusive close authority or prove original-account retention. An independently admitted, fully captured owner is a different candidate and must satisfy the same obligations. |
| Generic Core/Servient registration shutdown API now | Deferred. It may become necessary if a full-owner discriminator demonstrates an unconstructible capture or charge transfer. It does not itself fix SDK allocation, aborted joins or the final destructor. No dummy server or empty final call should conceal that discovery. |
| Pinned SDK correction with acknowledged per-query retirement and safe destruction | Credible fallback. Compare maintenance and allocation/resource changes against the concrete owner after preserving the smallest counterexample. Unstable cancellation alone does not retire timer tasks. |
| Contracts extraction, new runtime packages, universal scheduler/allocator, compatibility layer | Not justified by this boundary. Share accepted semantics and replace obsolete target edges directly under pre-release policy; do not duplicate planners or seals. |

## Precise admission blocker and next engineering boundary

The selected direction needs a **complete final-owner contract**. The current
fixture deliberately joins its scoped worker. If the last worker/controller is
lost while original close, a callback or SDK task remains live, there is still
no demonstrated finite destruction or acknowledged complete-object successor
retaining the original charge. Keeping a Session clone only postpones that
question; forgetting it or detaching a thread loses ownership. A residual status
alone cannot free live local SDK capacity. Following close timeout with another
close cannot establish retirement.

Additionally, the fixture's one cleanup item is only a real-account provenance
discriminator. It does not bound SDK bytes, thread stacks, transport input,
connections, query/callback state or allocator failure. Controlled allocation
failure during even a closed Session's final Drop aborts the process. A worker
relocates that failure; it does not make it recoverable. The supported allocator
and configuration contract must address this explicitly. Callback-side payload
rejection cannot alone bound allocation before callback entry; finite transport,
message, peer and SDK storage limits require source-backed qualification.

Complete a focused **non-production full-owner discriminator and resource
contract** using this topology: explicit shutdown, final Runtime/role/controller
loss, blocked jobs/callbacks, original-close timeout/abandonment, saturation and
construction/destruction allocation failure. Demonstrate either actual retirement
or a named, prebacked residual successor retaining every live local owner and the
actual original accounts. Prove what happens on loss of that successor too.
Audit supported SDK configuration and allocation maxima before asserting that a
numeric envelope is enforceable. Do not demand implemented production measurements
to prepare a contract: those remain completion evidence under ADR-0013.

If existing complete-registration capture/account ownership cannot implement that
contract, migrate only the demonstrated Core/Servient lifetime correction and
admit its exact predecessor slice before WP-600. If the locked SDK cannot support
it, preserve this reproduction and compare a minimal pinned dependency correction
with an explicitly unsupported deployment disposition. Do not relax boundedness
or manufacture a cleanup certificate.

After that blocker closes, prepare the existing ADR-0013 Host production candidate
with the six prospective keys already identified by 0083, exact prechecks, source
removal paths and independent admission. Its production boundary remains one
complete Consumer registration, one eager generation/call opportunity, two exact
selected property targets, repeated reads and a production Producer TCP loop.
Subscriptions, pico parity, reusable plan sharing, broad package closure and a
new architecture gate remain outside this decision.

The passed gates, completed predecessor evidence, 65 requirement identities,
resource defaults, performance identities and PLAN are reaffirmed within their
existing scopes. The new negative SDK evidence narrows concrete author eligibility;
it falsifies no demonstrated mock or Producer carrier invariant. No existing
admission or gate status is changed by the author.
