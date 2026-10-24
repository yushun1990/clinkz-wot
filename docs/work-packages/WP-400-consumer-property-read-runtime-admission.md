# Historical / restricted Property Read record

Status: historical v5 authority; **no v6 implementation admission or architecture
acceptance**. The original record and exact prose are preserved below and at
[the predecessor revision](https://github.com/yushun1990/clinkz-wot/blob/ed09f059e383b7a1b9d22902ee29aa218b4471a6/docs/work-packages/WP-400-consumer-property-read-runtime-admission.md).
[ADR-0025](../ADRs/0025-semantic-engine-and-binding-runtimes.org) supersedes its
general API, gate ordering and lifecycle projections. The [index](index.toml)
records individual tranche impact dispositions. Tests of retained Property Read
code remain useful at that narrow scope; neither a passed status nor this
record constrains the successor SPI. Exact schema/resource encodings still used
by retained adapters keep their original meaning. Any new source scope must use
the successor requirements and applicable review, not inherit this admission.

## Preserved predecessor record (non-authoritative for the general architecture)

# Consumer Property Read Runtime admission

`index.toml` owns `WP-400-CONSUMER-PROPERTY-READ-RUNTIME` admission and completion
state. This record defines its admitted v5.1 ADR-0013 implementation and evidence
envelope.

[Independent re-review of PR #165](https://github.com/yushun1990/clinkz-wot/pull/165#pullrequestreview-5505656673)
accepted base `4892a5ca8224dde1328a93460a862cdc826dbb5f` to head
`cf68b6250fefe9c311784aaabfac3bbb380bc1d5` with no remaining blocking findings,
superseding the initial changes-requested conclusion. The reviewed head and
merged master commit `6ee94a06865f92da079e560a90f0d8c51931c047` have the same tree
`6818ddcad576741f2f4dfd5bbd5c7c29ba854177`.
Required [exact-head validation](https://github.com/yushun1990/clinkz-wot/actions/runs/38417119093)
and [merge-push validation](https://github.com/yushun1990/clinkz-wot/actions/runs/38419010198)
passed; these checks cover diff hygiene only. The review records exact-head
prechecks separately from the broader checks at the initial candidate revision.

The separate docs-only admission transition followed that acceptance and permits
only the implementation envelope below. Production completion is recorded
separately.

[Independent production re-review of PR #168](https://github.com/yushun1990/clinkz-wot/pull/168#pullrequestreview-5513459509)
accepted exact head `b5517e8d9721084ba005d03c64f3cf32c7a1a79f` with no blocking
findings across all six completion obligations below. It verifies all six earlier
findings fixed and supersedes their changes-required conclusions. The reviewed
head and merged master commit `4324abdd6733e7080d3928f027c02341fd773a76` have the
same tree `eb4c42e8c7fdbb65de027d7b9d35955077b06c52`. Production and tests match
`cf2e0dec522f0854bcc898e1d437cab6e5fca57b`; the final reviewed commit changes
only the evidence file.
Required [exact-head validation](https://github.com/yushun1990/clinkz-wot/actions/runs/38508811697)
and [merge-push validation](https://github.com/yushun1990/clinkz-wot/actions/runs/38517353767)
passed; both cover diff hygiene only. The review independently reproduced all
47 recorded verification entries, including both native toolchains, actual
Host/static applications, ARM library compilation, predecessor/Producer
regressions and focused Memcheck, and passed its six downstream regression
probes. The [registered completion evidence](../evidence/WP-400-consumer-property-read-runtime.toml)
retains the exact executable revisions, six obligation mappings, normalized
outcomes, physical measurements, author bounds and scope exclusions.

This separate docs-only transition records completion of the bounded Runtime
tranche only: one startup registration, one generation and one manual call/result
opportunity in Host and application-static representations. Consumer
architecture-gate registration/acceptance, real Consumer protocol qualification,
broad WP-400/milestone closure and full performance matrices retain their
independent boundaries. Topics 0081 and 0082 remain OPEN and non-blocking for
this one-shot tranche; no sharing or long-lived subscription decision is made.

## Result and reconstruction

Deliver one usable Consumer-only Servient path: borrow an immutable typed Thing,
publish its complete eager Property Read generation, read a property through
the selected complete binding, return Core's sealed result, then close and reclaim.
Implement consume, call admission, manual progress, cancellation, drain and
reclamation together. A publication-only record or an additional call-storage
precursor would not deliver that path and has no separate unresolved lower-layer
contract to justify splitting this work.

The examined production baseline is `4892a5ca8224dde1328a93460a862cdc826dbb5f`.
`servient/src/property_read.rs` implements the accepted narrow **Producer**
kernel, route/handler owners and explicit Host/static progress. Its Host owner
takes runtime state out of a mutex before callbacks. It is useful evidence for
short owner checkouts, not an existing Consumer registry or global governor.
`servient/src/servient.rs::consume`, `servient/src/handle.rs` and
`core/src/thing.rs` still build/use the legacy TD-bearing ConsumedThing and
legacy client selection. The legacy builder also provisions Directory/event
state. Wrapping those methods would retain the wrong execution path.

Accepted predecessor owners, all `complete/current` with `passed` evidence:

| Predecessor | Accepted evidence and exact responsibility |
| --- | --- |
| `WP-100-CONSUMER-VALIDATED-THING` | `WP-100-consumer-borrowed-td-admission.toml`: complete inspection/Basic and paid semantic lending from caller-owned typed input |
| `WP-200-CONSUMER-COMPILER-SUPPORT` | `WP-200-consumer-compiler-support.toml`: closed compiler support and bounded compiler storage/work |
| `WP-200-CONSUMER-PROPERTY-READ-AGGREGATE` | `WP-200-consumer-property-read-aggregate.toml`: paid whole-readable census, all-bounds barrier, complete source-free output, original manual root/reclaimer |
| `WP-300-CONSUMER-PROPERTY-READ-BINDING` | `WP-300-consumer-property-read-binding-execution.toml`: name-free selected requests and installed normal/late-result sealing |
| `WP-300-CONSUMER-ONLY-REGISTRATION` | `WP-300-consumer-only-registration.toml`: real complete client-only owners, without a dummy server |
| `WP-200-CONSUMER-OUTPUT-OWNERSHIP` | `WP-200-consumer-output-ownership.toml`: unique structurally Send output/rollback transfer, preserving the physical root and original account; [independent review](https://github.com/yushun1990/clinkz-wot/pull/159#issuecomment-6138896086) |
| `WP-300-CONSUMER-HOST-CALL-CONSTRUCTION` | `WP-300-consumer-host-call-construction.toml`: recoverable raw construction and complete Core backing before invocation; [independent review](https://github.com/yushun1990/clinkz-wot/pull/163#pullrequestreview-5503833960) |

The table is dependency rationale; the index and evidence files own their states
and measurements. None proves publication, Runtime client eligibility, global
capacity, automatic root cleanup or a real protocol. Their accepted contracts
are reaffirmed unchanged. Runtime composes them and does not modify Foundation,
TD, Planning, Core seals, registration or installed-call signatures.

## Smallest coherent scope and alternatives

One startup Consumer-only complete registration, one consumed generation and
one active/read-result opportunity at a time per runtime. Repeated reads are
supported after terminal acknowledgement; a new consume is allowed only after
the old generation is reclaimed. Generations never wrap or reuse stale tickets.
Second consume/read admission returns structured backpressure, preserving its
input, rather than allocating another record. There is no call pool, dynamic
registry, hidden worker, executor requirement or overflow allocation.

| Alternative | Disposition |
| --- | --- |
| Finish this path within current packages | Selected. All necessary lower-layer contracts exist; private Runtime composition is the missing behavior. |
| Local worker with command/reply channels | Constructible in the existing investigation, but a worker adds startup, delivery and shutdown ownership. A unique manual driver with one bounded command/result cell needs no thread. |
| Put Draft/root under Arc or atomicize Planning | Rejected. The local graph is non-Send/non-Sync. Use the accepted consuming bridge, without unsafe auto-trait assertions or a second selector/reclaimer. |
| A shared reference into permanently local Planning state | Rejected. It creates self-reference, cross-thread loan and reclaim problems. Borrow a selection only during exclusive checkout. |
| Runtime-owned worker/executor cleanup or reusable call pools first | Deferred. An explicit manual cleanup owner and fixed call opportunity are sufficient. An executor may later adapt the same complete-owner handoff. |
| Contracts extraction, generated images, role features, optional Discovery refactor | Disjoint product investigations in topic 0076; unnecessary to construct this path. The new path itself provisions no Directory or Producer. |
| Route the new facade through legacy consume | Rejected. It retains TD/Form scanning and raw legacy clients, bypassing the admitted target path. |

This is manual Runtime evidence in all cells, with transferable Host handles and
driver state under `std`. `async` without `std` adds no executor and keeps the
static surface. An async convenience wrapper, multi-owner scheduler, Producer
integration and real Zenoh remain outside this tranche.

## Public construction and implementation envelope

Expose `consumer_property_read` from Servient, independently constructible from
complete client-only registration, checked policy, clock/deadline policy and
capacity owners. Constructing it must not pass through the legacy builder or
create a Directory, event broker, server, handler registry or legacy client list.
The existing Servient/StaticServient Producer surface remains valid.

The additive public families are registered in `api-ownership.csv`:

- `ConsumerPropertyReadPolicy`: immutable checked resource/work/input and cleanup
  policy with TD/Planning projections and captured binding author declarations;
- `ConsumerPropertyReadRuntime` and `StaticConsumerPropertyReadRuntime<C>`:
  the durable manual roots, construction, consume/read entry, step, close,
  explicit shutdown and fixed cleanup-record observation;
- `HostConsumerPropertyReadBuild<'td, 'runtime>` and
  `StaticConsumerPropertyReadBuild<'td, 'runtime, C>`: local consuming build
  transactions borrowing the external Thing and runtime environment;
- `ConsumerPropertyReadHandle` and `ConsumerPropertyReadCall`: non-Clone Host
  generation/interest facades, with bounded submission, result observation and
  cancellation. They never own the sole physical call or Planning root.

Static users drive the same lifecycle directly on the typed runtime and its
borrowed build/read adapters; a second shared handle/queue abstraction is not
required. Constructors return original complete registrations/inputs on failure.
Build completion returns a Host handle or static published readiness only after
installation. Public method contracts follow the phases below; helper names,
private records and short-borrow adapter signatures are implementation choices.
Adding a new public family, changing the one-generation/one-call limit or changing
these ownership semantics requires scope review.

Production changes are confined to `servient/src/consumer_property_read.rs`,
`servient/src/lib.rs` and, if needed for bounded construction, a private helper
under `servient/src/consumer_property_read/`. Tests/examples live in
`servient/tests/consumer_property_read.rs`, its support directory and the two
Consumer binding/runner fixture packages named in the index. Cargo manifests
and locks may only register these fixtures/dev dependencies; no production
dependency, new feature, transport SDK or executor is admitted. Evidence output
is `docs/evidence/WP-400-consumer-property-read-runtime.toml` with stable key
`consumer-property-read-runtime`.

No legacy public API is removed in this bounded change. Every new entry has zero
target-to-legacy calls. Broad ConsumedThing migration retains its own removal
obligations. No production source is changed by this admission transition.

## Owners and publication

| Owner | Retains until |
| --- | --- |
| Startup root | Actual complete registration and checked policy; last generation/call/cleanup owner is terminal |
| Build transaction | External immutable Thing loan, TD scratch, Planning continuation, real local and parent allowances, preallocated registry destination, original cleanup root/account; consumed completion or bounded rollback handoff |
| Published generation | Complete owned output/root, registration reference, Thing/plan-set/slot identities, pin gate, call opportunity, cleanup/status and real capacity owners; drain plus last terminal lease |
| Accepted call/cleanup envelope | Decorated Host call or complete typed slot, unused Core seal inside it, exact generation lease, first cause, input/result ownership, phase/deadline and capacity; one terminal settlement and actual destruction/acknowledgement |
| Manual root | All abandoned build/call/reclaim work and fixed terminal report; explicit shutdown or the final residual-discard path below |

Host parks only `ConsumerPropertyReadOwnedOutput` or owned rollback in a private
record. The structural Send bridge may sit behind a mutex; local Draft/root may
not. One driver checks out the complete owner, releases structural locks, then
reattaches and uses the existing selector. Before checkout it reserves the fixed
bridge credit required to return the graph on every exit, including disinterest
and unwinding of the driver adapter. A borrowed selection cannot escape checkout.
The driver retains the actual root account and a strong root reference while
outside the lock. Repark before releasing that reference. An empty parked slot
means a live exclusive driver claim, never available capacity or Idle cleanup.

Host root, consumed handle and call ticket have a fixed bounded ownership graph;
no unbounded Clone surface is added. Closing/dropping the root or consumed handle
closes the pin/admission gate. Dropping a call ticket commits disinterest and
cancellation once against its call generation; it does not destroy the call.
Callbacks, allocation, validation, compiler work, result destruction and reclaim
execute outside structural locks. Different runtime roots share no mandatory
callback lock or ready queue; broader fairness is not claimed.

Static retains the typed complete registration, local aggregate/root, original
accounts, phase/cursors and actual `StaticConsumerPropertyReadSlot<C::RequestState>`
in caller-owned storage. Short mutable borrows drive operations. It uses no Host
bridge, Arc graph, erased slot or self-reference. Associated-state acknowledgement
must finish before slot reuse; stale generations cannot mutate a new occupant.

Publication order is fixed:

1. Check startup support before observing input or allocating runtime work. Hold
   paired build-owner, TD scratch/diagnostic and rollback capacity before TD work.
2. Complete TD inspection/Basic and paid Planning preflight. A missing Thing ID
   fails here before runtime reservation. Reserve the real parent/global output,
   runtime-record, call/cleanup and terminal-discard envelopes, then call
   `admit_runtime`; materialize all rows/coordinates and finish all bounds before
   the first compiler start.
3. Consume input-bearing completion, finish bounded scratch cleanup and end all
   source/compiler/configuration loans. Reconcile physical owners and accounts.
   Retained upstream engine source stays charged while physically live.
4. Predebit the complete fixed final-check/install envelope, including any bridge
   conversion. Zero/short credit remains FinalChecks Pending without publication.
   Under exclusive slot authority revalidate phase/nonce, policy, actual captured
   registration, all generations, held capacity, cancellation and reserved slot.
5. Issue a private permit closing cancellation, then move the already backed
   complete record into the registry. No allocation, callback, budget request,
   yield, source query, fallible release or cancellation decision occurs between
   permit and install. Before-permit cancellation wins; after-permit cancellation
   initiates drain of the installed generation.

Use separate checked Thing-slot, plan-set and call generations, not a borrowed
registration generation as the allocator. All failed checks keep the exact whole
unpublished owner and first cause. No subset or partial lookup is published.

## Resource admission and trusted callback boundary

Foundation's existing accounts suffice. The Runtime privately binds actual held
parent/global reservations, policy/registration identity, transaction nonce,
phase and generations before constructing local ledgers or cleanup roots. Equal
public owner tuples or ceiling values cannot authorize parent capacity. Retain
the **original** committed cleanup account; release physical children before
local committed charges and parent allowances, including detached and abandoned
work. There is no new Foundation provenance token or global shadow ledger.

Measure admission/build scratch, record/header/shared-registration provisioning,
complete output, original cleanup destination, request/map capacities, call/result,
Core envelope, raw call backing and transient checkout/return/transfer overlap.
Apply existing plan-set, compiled-runtime, engine-live, admission-peak,
largest-allocation, in-flight, binding-state and cleanup item/byte limits at both
local and parent scopes. An aggregate allowance or a sum of Layouts is not a
contiguous allocation or a measured simultaneous peak. Shared registration bytes
are charged once, with references/inline storage accounted separately. There is
no TD/source charge attributed to Published borrowed input.

Call and cleanup capacity are paired **before invocation**: one fixed admitted
call/result record plus a manual destination large enough for the complete call,
lease, phase, reservations and status. Accepted resource overrun keeps the raw
call and Core accepted-error mediator; it cannot become input rejection or free
the slot early. Cleanup never needs another call slot or an allocation fallback.

The startup policy consumes explicit binding-author declarations for finite
constructor, footprint/deadline/poll/cancel/destructor work, request/result storage
and supported input limits. It binds those declarations to the **actual moved
complete registration**, not an arbitrary numeric identity supplied later.
Check their arithmetic, role/cell, compiler support and resource applicability
before TD work. Missing or declared-unsupported callback/input profiles reject
construction. This is a trusted binding-author contract, not a dynamic proof or
sandbox of arbitrary Rust callbacks; the implementation evidence must audit the
selected concrete author against it. Compiler support and compatibility alone
never qualify a client. No new Core/Foundation public support issuer is added.
The Runtime cannot inspect an erased constructor to discover dishonesty in those
declarations. Source audit and executable concrete-client evidence establish the
claimed supported boundary; a literal fixture-ID allowlist would not do so.

Completion fixtures must author an actually recoverable, finite client for both
profiles, with pre-admitted/prebacked variable results or checked fallible backing
before acceptance. Reuse the accepted Core witness's construction principles,
not the older runtime-entry Host client's infallible output/raw allocations.
That older client remains a negative authoring example. Cover bounded URI-variable
count, UTF-8 lengths and **actual capacities**, bounded result body/content type,
selection-name comparison work, and constructor failure/destruction. Invalid or
over-capacity owned inputs return unchanged; rejection cannot perform an unpaid
variable destructor. Name/option copying uses paid fallible backing. This tranche
does not certify arbitrary third-party callbacks or real protocol libraries.

ADR-0022 preserves this completed tranche and its finite, recoverable mock author
unchanged. A future real Host protocol author additionally uses Foundation's
[Host allocation/failure scope](../spec/foundation.md#host-allocation-and-environment-failure-scope):
raw/Core construction recovery remains required, while SDK allocation during
owned I/O may use the declared process-failure policy. Its input/storage envelope,
runtime lifetime and final destruction require new concrete evidence; this
tranche's mock measurements cannot qualify them. No Runtime API or destructor
contract changes here.

## Read, cancellation and terminal release

Only Published admits a generation-bearing lease. Under the fixed opportunity's
claim, apply the narrowed `InteractionOptions`, use the existing aggregate
selector, derive `OutboundRequest` solely from the selected artifact reference,
and pass only the name-free request to the complete registration's
`start_consumer_property_read`. There is no TD scan, registration scan, second
lookup/validator, raw installed-client bypass, fallback or automatic retry.
Bound selection/comparison and input work before invocation. A selection or
pre-acceptance construction failure preserves input and releases only its own
reservations; a subsequent explicit user call is a new attempt.

Normal output and both immediate/polled cancellation-late Returned outputs pass
through the existing Core mediator once. An invalid late success remains a sealed
Validation result; raw structured errors keep their category. Caller disinterest
never changes acceptance history. The result opportunity remains occupied until
delivery, bounded discarded-result release or cleanup/residual disposition and
static acknowledgement; a pending or unread result cannot be overwritten.

First Manual, Deadline, Drop or Drain cause wins against the call generation.
Use a fresh `CleanupPhaseContext` and independent cancel-drain deadline; never
extend it by refilling step work. Pending cancellation retains the decorated
call and generation lease. Complete and Returned are settled exactly once.
Host TransferRequired offers the **whole** call + lease + accounts + first cause
to its reserved manual destination via the existing acknowledged envelope
contract. Rejection returns the identical owner to that destination; only
acknowledged acceptance permits PendingCleanup. A status record is not work.
No executor exists to reject/shut down in this tranche: root-owned manual
retention is unconditional. An optional transfer target's refusal is still tested.
Static Core deliberately rejects client-fabricated TransferRequired; preserve
that contract and its negative test. The Runtime's reserved typed root already
owns the complete slot and lease during manual cancellation. It does not need
to mint transfer authority or clear a Cancelling slot. Only normal terminal
acknowledgement or recorded residual followed by destruction of the complete
slot releases it. Normalize terminal ownership/outcomes across profiles, while
reporting these different physical retention/transfer mechanisms separately.

Close stops new leases/calls before Draining is observable. Existing leases stay
valid; cancellation/drain and late results finish within bounded policy or commit
a durable residual. Reclaim starts only after all call/selection/cleanup owners
are terminal. Reuse original Planning rollback, preserving partial byte credit.

Child Drop only marks/transfers into already retained root storage. Explicit
shutdown drives calls and reclaim and returns the fixed final report. The fixed
report destination survives independently of root/interest references, with its
own provisioning charge. A caller can retain it after every Runtime owner is
gone. Dropping
the last root reference without shutdown cannot imply successful external
cleanup: commit an incomplete-shutdown/residual record to a caller-retained fixed
report destination **before** destroying unresolved protocol-local state. Then
release locally owned call/results and run the original Planning reclaimer with
one prepaid finite terminal-discard allowance derived with checked arithmetic
from the supported census/input bounds. No network progress, application handler,
allocation, budget refill or recursive destruction is allowed in this last-root
path. Client/artifact local destructors must meet the audited finite contract;
protocol-progress callbacks are never invoked.
The allowance includes reclaimer item/byte work, bridge and terminal root release.
Release accounts and parents last. A missing report destination or insufficient
terminal allowance rejects startup/preflight before side effects. This local
discard is separately measured; it supplies no external compensation or hard
wall-clock latency guarantee. Leaving the bridge's ManuallyDrop root unreachable
is not an acceptable implementation of Runtime Drop.

The existing reclaimer is constructible without another Planning API. Retain
the paid preflight: `B = output_capacity_bytes` bounds total variable release
bytes, and conservative `I = remaining_planning_items + 3` bounds
scope/coordinate/row/table/terminal item releases. Each progressing
`Rollback::step` pays at least one byte or one item; with sufficient terminal
credit and nonzero configured reclaim cap, at most checked `B + I` steps are
needed. Reserve those byte/item credits plus fixed coordinator/bridge/release
overhead before materialization; account the finite coordinator step envelope
separately without refilling underlying allowances. Partial byte credit reduces
remaining work. Check all arithmetic/applicability before acquiring children.
The discriminator also demonstrates a looser source-free bound from retained
bytes plus row/coordinate counts: each scope owns an accounted table entry even
when its text is empty. Neither bound measures machine stack or elapsed time.
Constructor/payload destruction needs its own audited bound.

## Completion and independent acceptance

The evidence file must bind exact production/test revisions, toolchain/target,
commands, normalized Host/static case results and physical measurements. Required
falsifiable obligations are:

1. A downstream client-only application constructs the public Runtime, borrows
   a real Thing, consumes the input-bearing build, destroys the Thing, reads at
   least two properties and non-first original Form coordinates using actual
   compiled artifacts, receives Core-sealed output, closes and reclaims. Repeat
   reads and generation reuse work. Both cells use the same case/expected-outcome
   table; static has actual associated state. No dummy server, legacy call or TD
   retention is reachable. Unsupported policy/client declarations fail before
   input observation, allowances, compiler calls or binding effects.
2. Inject failure/cancellation/abandonment at each unpublished phase, every
   controlled allocation and final check, including missing ID, later compiler
   bounds, cancellation with scratch pending and stale/equal-looking slots.
   Return exact owners/inputs, keep first cause, stop new starts and reclaim all
   children before parent release. Concurrent roots exhaust a shared real parent
   at build, output and call/cleanup peaks; local ceilings alone cannot pass.
3. Execute real Host registry races for zero/short/exact final work, stale
   generation/slot reuse and both cancel/permit orders. Readers see absent or
   complete. Observe zero allocations/callbacks/releases after permit. Move
   quiescent Host driver state across threads; cancel/drop interest while state
   is checked out. It reparks once with unchanged artifact/root identity and
   accounts. No busy owner is reported as terminal/Idle. Callback reentrancy
   observes no held structural lock.
4. Saturate the one read/result opportunity and its cleanup destination. Cover
   pre-acceptance request recovery, accepted overrun, normal/error/invalid results,
   zero work, deadline wake/progress and no retry. Cancellation matrix includes
   before first poll, Pending, completion race, call-ticket/handle/root Drop,
   drain expiry, immediate/polled late Returned, Complete, Host acknowledged transfer,
   identical rejection/manual fallback, static fabricated-transfer rejection and
   residual. Each call, seal, slot,
   result and plan lease has one terminal release/acknowledgement.
5. Explicit shutdown and final-root abandonment both release all controlled local
   output/root/call/registration/record backing and original accounts exactly once.
   Abandonment records residual before destruction, never reports successful
   external cleanup, uses one finite non-refilled terminal allowance and performs
   no protocol progress/allocation. Keep the report observer outside the root.
   Include an abandoned build, outstanding result, pending call and partially paid
   rollback. A mere leaked manual root or lost cleanup charge fails acceptance.
6. Measure actual local/global simultaneous live/peak/largest allocations,
   provisioning versus admission versus steady call costs, Host envelope/raw/
   result and return overlap, static layout and terminal work. Published retains
   zero borrowed-build TD bytes. After reclaim controlled retained bytes and
   committed charges are zero. Allocation failure at each fallible site is
   recoverable; arbitrary source/callback/allocator/stack/device-fit claims are
   excluded. Reproduce predecessor regressions and affected feature cells.

Existing workload identities `PERF-GW-009`, `PERF-GW-010`, `PERF-GW-028`,
`PERF-CS-020`, `PERF-GW-029` and `PERF-CS-021` apply only to the Property Read,
call/slot and plan-retention dimensions exercised here. Keep their manifests,
fixtures and budgets unchanged. Attach focused allocation/work measurements
and a case mapping; full network/subscription/emission/fairness matrices and
release performance completion remain outstanding. The accepted manifest harness
validates their contracts before code; target measurements follow implementation.
No new benchmark framework or gate is needed.

Prechecks in the index use the existing downstream fixture and actual Core and
Servient tests plus generic authority/resource/performance validation. The added
three `runtime_candidate` cases exercise real output transfer, original reclaim
and real accounts; publication/header/checkout logic is explicitly a model.
Its six-unit final debit, conservative output-only discard bound and cleanup-only parent
reservation are **not** production formulas, a whole-Runtime governor, client
qualification or acceptance. Production obligations above remain mandatory.

Topic 0076's intersecting ownership/call/resource/shutdown questions are resolved
for this bounded path and projected here and in the lifecycle owner. Its broader
deployment/package/image questions remain disjoint and DISCUSSING. Topic 0079's
role-only decision remains migrated. Review 07 is not accepted as a wholesale
redesign. The broad WP-400 two-Thing/two-binding scheduler checkpoint is neither
a predecessor nor satisfied by this admission. Existing Producer gate status,
65/31/121 authority identities, resource schema/defaults and PLAN stay unchanged.

[Topic 0081](../../workspace/0081-reusable-plan-and-thing-identity.md) remains
OPEN and non-blocking for this bounded proof. Unique per-Thing lifecycle/root
ownership and the one-generation limit do not require physical duplication of
compatible immutable execution data or equate Thing, device and transport-session
counts. Rejecting shared access to the current local Draft/root graph leaves
future compatible separation of reusable immutable execution data from per-Thing
context and lifecycle ownership open; this admission selects no sharing
representation or fleet-scalability claim.

The production acceptance recorded above supports this tranche's completion.
Consumer architecture-gate registration and acceptance remain separate subsequent
boundaries; do not add product code merely to register the gate.
