# WP-300 Binding Contracts and Binding-Local Progress

Machine-readable package status, design revision, dependencies, document path,
and owner crates are defined only in [`index.toml`](index.toml). This document
specifies technical scope and acceptance boundaries.

## Scope

Replace the host-only binding shapes in `clinkz-wot-core` with the frozen
complete-registration and constrained associated-state contracts. Consume the
WP-200 compiler component unchanged when constructing each complete
registration; implement route-scoped readiness and permit-authorized
acceptance, request/response ownership,
subscription start/stop,
Producer emission, runtime status, form contribution, generation-safe typed operation slots,
binding-owned subscription progress, binding-local publication slots, and bounded cleanup
progress without adding a concrete protocol or a Servient scheduler.

This package defines and tests the execution SPI consumed by Servient and protocol packages.
WP-400 owns expose/destroy orchestration and registries; WP-600 owns zenoh and zenoh-pico
implementations. No protocol-specific route, transport, or authentication semantics enter core.

The narrow Property Read binding surface uses the exact mock-binding boundary
in the integration-gate manifest. It proves one route and one request without
claiming the subscription, emission, form-contribution, multi-route, workload,
or production-protocol parts of broad WP-300.

This is the only package that introduces `ProducerEmission`. Core defines the immutable emission
values and a `BindingEmissionSlot` for exactly one selected binding generation; it does not own
the Servient-wide fan-out record or a concrete dispatch policy. This package also provides bounded internal
compatibility adapters for unmigrated Servient and concrete-binding callers.
An adapter is not a second public emission contract and accepts no new callers.

One host or static binding is installable only as a complete startup bundle. Component traits
remain constructible and independently testable, but a compiler, contributor, client half,
server half, status policy, or ingress policy cannot be installed separately. The bundle is
immutable after `ServientBuilder` freezes its registration snapshot; v1 has no runtime binding
add, replace, remove, or unload path.

Broad registration/compiler/contributor signatures must remain constructible
in both host-erased and application-static forms. External authoring tests
cover a mostly synchronous binding and one with externally visible
preparation/readiness. Trivial phases may use bounded helpers or default
adapters, but those helpers must not invent protocol state or merge distinct
lifecycle ownership phases.

The v4.6 `WP-100-interaction-output-api-v1.md` proposal for a broad
`InboundResponse::try_success` and consumer response validator is historical
v5-inactive staging input. It does not admit those broad contracts. The current
narrow carrier is `RouteInboundResponse`. Core's single
`seal_property_read_handler_result` boundary validates handler-origin Property
Read success while consuming the existing response opportunity. Future
`InboundResponse` work may rename/generalize that carrier and kernel after
broad domain entry; it must not add a second runtime envelope.

## Scoped Property Read Technical Boundary

The narrow Property Read surface introduces the route, permit, response,
cleanup, resource, and host/static ownership boundary. It constructs one complete registration
that advertises only the Producer Property Read server role and consumes one
matching WP-200 compiler component.

Its behavior scope is:

1. validate registration, compiler/artifact, generation, profile-cell,
   footprint, ingress, readiness, status, overflow, and cleanup identity before
   publication;
2. cover both an immediate-ready mock and an externally-ready mock;
3. preserve owned guards through prepare, readiness, activate, commit,
   abort, and shutdown;
4. return a committed-closed guard without opening request admission;
5. accept one Property Read request only under a fresh borrowed route permit;
6. accept only Core-sealed successful Property Read output, preserving handler
   errors and converting invalid successful shapes into deliverable validation
   errors on the same response opportunity;
7. preserve the complete response opportunity and response on pre-acceptance
   rejection, then settle accepted delivery exactly once; and
8. prove the same contract through a host-erased `std` author and an
   application-static `no_std + alloc` author.

The applicable state projections are `binding-route-lifecycle`,
`binding-route-readiness`, `active-route-acceptance`,
`response-delivery-ownership`, and the cleanup-transfer projection used by
route or response calls. The narrow surface does not claim performance-workload
coverage; it tests deterministic resource/footprint behavior while the broad
package retains its workload boundaries.

The narrow public bundle and server interfaces omit inactive client,
subscription, publication, collection, contribution, and emission families.
Broad WP-300 may add bounded default rejection adapters only within the owning
broad domain. Retained API-inventory names do not authorize those behaviors in
this slice.

The narrow surface excludes:

- client invoke or subscribe behavior;
- subscription drivers and delivery;
- Producer emission or publication behavior;
- collection capability behavior and form contribution;
- broad cancellation/race matrices, multi-route fairness, and package
  workloads;
- Servient registry, publication, scheduling, or application dispatch;
- production protocol or Zenoh implementation;
- broad old-API removal; and
- either cross-package Property Read architecture fixture root.

Target code may consume only the WP-200 logical plan and artifact identity. It
must not depend on or call the legacy
`clinkz-wot-protocol-bindings` form selectors, receive a TD, or send a target
request through legacy `ServerBinding::serve`, `Dispatch`, or handler lookup.
Legacy selector and execution paths remain separate legacy generations until
their WP-600/WP-700 removal scope.

Rust method overloading cannot preserve the legacy
`ServerBinding::shutdown(&ThingId)` and add target
`shutdown(RouteShutdownInput)` to the same trait. The narrow target therefore
uses the uniquely named `RouteServerBinding`, `RouteInboundRequest`,
`RouteResponseOpportunity`, and `RouteInboundResponse` types in
`core/src/binding.rs`. Existing `core/src/inbound.rs` values remain the legacy
generation and are outside this narrow surface. WP-700 removes the old exports;
there is no alias or conversion from a target request to a legacy request.

The exact signatures, both readiness shapes, both public author profiles, and
the exclusions above are technical constraints of the narrow surface. They do
not imply subscription, emission, or the rest of broad WP-300.

Before the aggregate Property Read fixture or broad WP-300 work proceeds, run one bounded,
non-authoritative real-target Zenoh Property Read feedback probe against the
complete public registration surface. It uses actual Zenoh I/O and a network
round trip; carries a real plan/route output through readiness,
correlation, response, cancellation/drain, and cleanup; and includes enough
multiple-Thing/route/form shape to expose hidden single-fixture assumptions.
Helpers may group fields, generate closed static enums/tables, and adapt
synchronous `NoCleanupSuccessor` operations, but may not hide resources or
lifecycle ownership. The probe records cleanup-library mapping, diagnostics,
repeated workarounds, generic/monomorphization pressure, constrained
layout/code size, and unsafe/private dependency needs. Reopen the SPI for a
concrete ownership, portability, resource-accounting, unsafe-erasure, or
implementability defect, or for systematic authoring failures that repeatedly
lose semantic truth or exceed declared bounds. Mechanical repetition and
field count alone remain helper/generation concerns. The probe does not
release WP-600, prove protocol-shape neutrality, or claim production-protocol
progress.

### Real-target probe technical disposition

The executable probe is
`protocol-bindings/protocols/zenoh/tests/target_property_read_feedback_probe.rs`.
It contains external application-static and public Host-erased authors built
from the target Planning, Binding, and Servient surfaces. With the real Zenoh
Rust SDK and loopback TCP it proves:

- an immutable compiler-produced Producer-route artifact carries the resolved
  Zenoh transport, authority, key expression, property, selected form index,
  content type, and subprotocol through Servient's scoped artifact access;
- the scoped borrow naturally derives one non-`Clone`, route-local owned state
  containing the Zenoh session, queryable, one pending-or-in-flight query,
  correlation counter, and one accept waker, without copying `PrepareInput`,
  retaining a plan-set lease, or adding an artifact or binding route table;
- a real query enters through the applicable permit-gated static or Host
  accept surface, is dispatched by Servient to `ReadPropertyHandler`, and
  returns through the target response SPI to an independent Zenoh requester;
- actual declaration, readiness failure, pre-publication cancellation,
  draining, queryable undeclaration, session close, and post-cleanup route
  absence are externally exercised for both representations across three
  different Thing/property/form shapes; and
- zero `BindingPolls` budget performs no compiler or binding side effect.

The implementation needed one narrow public Zenoh helper that parses an
already-resolved Planning target. It does not revisit the TD or select a form.
Zenoh session open and queryable declaration map naturally to preparation;
query/reply and undeclare/close map naturally to correlation, response, and
cleanup. Zenoh has no distinct SDK operations corresponding to the target
readiness, activate, and commit stages: readiness is a Servient-visible check
after declaration, while activate and commit are publication barriers. The
probe contains one protocol-local stage marker but no second dispatch or
normative transition machine.

The static and Host server traits still require authors to spell their closed
callback sets, including mechanically unreachable phase methods. That is
concrete helper/generation and representation-duplication pressure. It is not
evidence of a second normative lifecycle kernel: the paired probe shares only
protocol-local Zenoh construction, response, and cleanup helpers, while
Servient remains the sole transition, publication, dispatch, and cleanup-policy
owner. Its generic state types compile without a private bound or binding-side
unsafe erasure; the probe did not obtain a useful standalone
monomorphization/code-size measurement from a debug test binary linked with the
Zenoh SDK.

The typed static `ServerRouteSlot` and corrected erased Host guards now keep
the same owned Zenoh state through all stages. One private Host carrier is
created with the original `PrepareInput`, immutable footprint, generation
identity, and erased state. Prepared -> active -> committed conversion moves
that carrier between the three linear guard types without accepting replacement
state or exposing consuming extraction. Type-checked access is a pinned shared
projection only; protocol-local methods encapsulate mutation without exposing
`&mut S` or `Pin<&mut S>`. Accept polling borrows the committed guard only by
shared reference while Servient retains the owner; the binding receives no
`Pin<&mut HostCommittedRouteGuard>` replacement path. Focused Core evidence
proves allocation identity and footprint continuity for `Unpin` and `!Unpin`
state, rejects a mismatched concrete type, compile-fails whole-state and
whole-guard replacement attempts, and observes one drop only after the
terminal owner is disposed. The paired real-target Host
scenarios additionally prove that readiness failure and pre-publication
cancellation retain the non-`Clone` Zenoh state until terminal cleanup, and
that success preserves generation, permit, correlation, response, and cleanup
semantics already exercised by the static path. A
response callback may retain a non-owning `Weak` alias while the route is live;
it cannot extend or replace the guard-owned primary state and is not a route
table. Workspace topic 0058 records the migrated correction. The real-target
prerequisite no longer blocks aggregate source admission. The separate
deterministic mock binding and runner now execute the aggregate gate in both
representations. They validate the complete live route/correlation identity,
observe the exact successful payload and media type, and settle a Core-sealed
invalid handler success once on its original response opportunity. This does
not widen the probe or aggregate evidence into broad WP-300 or WP-600
completion. The executable aggregate evidence is assembled and the registered
`PROPERTY-READ-ARCHITECTURE` gate has passed independent acceptance.

That passed Producer gate does not admit broad client, subscription, or emission
work by implication. Consumer-side request/call work must first complete its
explicit domain-entry authority review and the required Consumer Property Read
cross-package proof. Subscription/emission work must first complete its own
domain-entry review and the required minimal `ObserveProperty` long-lived
cross-package proof. Exact unaffected tranches may still proceed under ADR-0013
when their predecessor and authority scopes are independently shown disjoint.

The probe also records two maturity limits rather than hiding them. First,
optional Zenoh Form extensions such as priority and congestion control are not
present in the current protocol-neutral logical-plan projection; the minimum
route uses target, content type, and subprotocol metadata only. Second, the
declared route/readiness/response layouts account for binding-visible retained
state, queue slots, and wakers, but the Zenoh SDK and caller-supplied Tokio
reactor own additional host heap, task, socket, and wake resources that the
current declarations do not physically admit or measure. The real SDK cannot
exercise a constrained/no-`std` layout or yield a meaningful cross-profile
code-size result. Polling the SDK futures also requires a caller-supplied Tokio
reactor; driving `Session::close` without one fails at runtime, so the Host
runtime-cell ownership cannot remain implicit. These are explicit broad
resource-authoring and profile limits, not claims of WP-600 completion.

Broad WP-300 additionally requires:

- one executable capability/profile applicability taxonomy;
- one shared Core semantic-kernel owner and versioned machine-readable trace
  oracle consumed by both Host and constrained runners;
- semantic resource deltas separated from profile-specific physical
  allocation/layout/code-size costs;
- a cleanup-obligation coexistence matrix for every activated operation
  family; and
- exact resource-authoring closure for typed applicability,
  complete role projections, and schema identity before the resource surface
  is called stable for external applications or bindings.

These are broad maturity boundaries. They do not expand the narrow
Property Read source paths or turn its package-local constructibility claim
into a production, protocol-neutrality, ergonomics, or runtime-parity claim.


## v5.1 Consumer Property Read entry slice

Active authority consumed by this slice:

- `BIND-OUT-001` plus active `BIND-REG-001`, `BIND-STORAGE-001`,
  `BIND-MEM-001`, `BIND-DELIVERY-001`, `BIND-IO-001`,
  `BIND-CALL-CANCEL-001`, and `BIND-HOST-CANCEL-001`.

Minimum scope:

- add one Consumer Property Read client capability to a complete registration;
- construct one `OutboundRequest` only after selection/security commit;
- invoke only the selected client execution component, with no `Thing`, raw
  `Form`, mutable `InteractionOptions`, `supports_with_thing`, or reselection
  authority in the binding input;
- Host execution returns one owned cancellation-aware call before protocol side
  effects;
- constrained execution uses one admitted generation-bearing request slot with
  the same accepted/rejected/terminal semantics;
- pre-acceptance rejection returns the exact request; caller drop, cancellation,
  timeout, late result, and cleanup preserve one owner;
- every installed Host or constrained execution starts through its complete
  registration, which derives private single-use validation authority before
  moving the request;
- the Host registration returns the existing call box around a thin sealing
  decorator, while the constrained registration mediates one opaque complete
  Consumer slot covering synchronous-ready, pending, and cancellation-late
  results;
- binding-origin success remains untrusted until Core validation in its
  original normal or late-return terminal branch;
- raw client traits and raw static request slots remain authoring SPIs, but an
  installed registration exposes no raw Consumer component or acknowledgement
  and clear bypass; and
- Host transfer retains the decorated call, seal, and accounting together,
  while the admitted static phase has no named transfer owner and introduces
  no static cleanup-transfer machinery.

Explicitly excluded:

- `BIND-PROGRESS-001`;
- subscription driver/start APIs as active implementation work;
- broad retry/fallback;
- concrete Zenoh production implementation.

Legacy `BindingRequest` and raw-form selection may remain only for legitimate
unmigrated capabilities. The Consumer Property Read target path must have zero
edges to them.

## Requirements

- `BIND-REG-001`, `BIND-ROUTE-001`, `BIND-STORAGE-001`, `BIND-MEM-001`, and
  `BIND-DELIVERY-001` govern complete startup bundles, route-scoped engine progress,
  associated-state storage, lifetime and ingress bounds, and input preservation at acceptance.
- `API-SURFACE-001`, `BIND-IO-001`, `BIND-OUT-001`, `BIND-PROGRESS-001`,
  `BIND-CALL-CANCEL-001`, and `BIND-HOST-CANCEL-001` freeze host and poll execution,
  ownership, response, cancellation settlement, and subscription progress.
- `API-PAYLOAD-001` governs response metadata, validation, and the exactly-one terminal value.
- `LIFE-EXPOSE-002`, `LIFE-EXPOSE-003`, `STATE-BIND-001`, and `STATE-INFLIGHT-001` govern
  readiness, guard ownership, cleanup transfer, and admitted requests.
- `STATE-SUB-001`, `HANDLE-DROP-001`, and `PRODUCER-EMIT-001` govern subscription and emission
  ownership through cancellation, drop, and partial publication.
- `CLEANUP-RECORD-001` requires bounded cleanup identity and retained ownership without
  cloning plans, payloads, credentials, or TD documents.
- `SUB-STORAGE-001` and `SUB-DATA-001` govern bounded shared storage, direct slot delivery, and
  terminal visibility.
- `FORM-FINALIZE-001`, `FORM-FINALIZE-002`, `FORM-OWNER-001`, and `FORM-COVERAGE-001` govern
  registration capability, deterministic contribution, ownership, and strict coverage inputs.
- `CAP-STATUS-001` and `CAP-OVERFLOW-001` govern bounded runtime events and durable critical
  status.
- `CONSTRAINED-PROGRESS-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-SCHED-001`, and
  `CONSTRAINED-OWN-001` govern slots, typed work, fairness, and non-atomic ownership.
- `HOST-ASYNC-001`, `PERF-CALL-001`, and `PERF-ALLOC-001` govern erased adapters and
  allocation-sensitive binding-local paths. WP-400 owns the `HOST-SHARD-001` and
  `PERF-FANOUT-001` through `PERF-FANOUT-002` coordinator requirements.

## Crates and Feature Cells

- Modify Cargo package `clinkz-wot-core`; consume WP-000 foundation values and WP-100/WP-200
  core values without depending on Servient or a concrete protocol.
- The `no-default` cell exposes `PollClientBinding`, `PollServerBinding`,
  `StaticBindingRegistration<B>`, associated-state layouts, caller-owned typed slots,
  form-contribution values, state/status records, and public subscription/emission values
  without `Arc`, boxed futures, atomics, or an executor.
- The `async-no-std` cell preserves the poll contract and may provide native async adapters
  without executor selection.
- The `std` cell exposes object-safe server/client execution components and `HostBindingCall`,
  owned call boxes, prepared, active, committed-closed, and shutdown route guards,
  `HostSubscriptionDriver`, host subscription start, and one complete
  `HostBindingRegistration`. Boxed futures are allowed only on these erased network paths;
  status, overflow, reactor, and ingress policies are fields of the bundle rather than a
  separately installable sink configuration.
- Use fake bindings and caller-owned tables in core integration tests. Do not implement zenoh,
  sockets, spawned transport tasks, or Servient registries in this package.

## Public API and Data Migration

The broad values below are v4.9 domain-entry input unless they are already
implemented by the scoped Property Read surface above and owned by an active
v5 requirement. In particular, the list does not activate the historical broad
`InboundResponse`, client output validation, subscriptions, or emissions.

Implement the frozen shared binding surface:

- values: `OutboundRequest`, `InboundRequest`, `InboundResponse`, `PrepareInput`,
  `BindingRouteKey`, `BindingContext`, `SubscriptionStart`, `SubscriptionStopRequest`,
  `SubscriptionStopInput`,
  `SubscriptionItem`, `SubscriptionDriverEvent`, `SubscriptionDriverCleanupDisposition`,
  `BindingInputRejection<T>`, `CleanupReservation`, `CleanupPhaseContext`,
  `CleanupTransferRequest`, `CleanupTransferEnvelope<T>`,
  `CleanupTransferAcceptance<T>`, `CleanupTransferTarget<T>`, `NoCleanupSuccessor`,
  `BindingCancellationDisposition<C>`, and `BindingCallSettlement<T, C>`;
- compiler-extension components from WP-200:
  `HostBindingCompilerRegistration` or
  `StaticBindingCompilerRegistration<B::Compiler>`, together with their
  Core-owned artifact and compiler values; WP-300 consumes exactly one in a
  complete bundle and must not redefine, erase, wrap, or separately implement
  the compiler/artifact SPI;
- consume the WP-200 `CollectionSubscriptionCapability` unchanged when starting a root collection
  request; the SPI may not infer capability from protocol text or synthesize affordance fan-out;
- constrained traits and storage: `BindingStateLayout`, `PollClientBinding`,
  `PollServerBinding`, and typed `ClientRequestSlot<B::RequestState>`,
  `ClientSubscriptionSlot<B::SubscriptionState>`, route/readiness slots over the server
  associated states, `ServerResponseSlot<B::ResponseState>`, and
  `BindingEmissionSlot<B::EmissionState>`; a committed route slot records
  `CommittedClosed` and `poll_accept` requires a borrowed `RouteActivationPermit<'_>`;
- host execution components: narrow `RouteServerBinding`, `HostPreparedRouteGuard`,
  `HostActiveRouteGuard`, `HostCommittedRouteGuard`, `HostShutdownRouteGuard`,
  `RouteCommitOutcome<A, C>`, `RouteCleanupSuccessor<P, A, C>`,
  `HostRouteCleanupSuccessor`, route-scoped `RouteAcceptEvent`, `BindingCallFootprint`,
  `HostBindingCall`, `HostBindingCallBox`, `ClientBinding`, `HostSubscriptionDriver`, and
  `HostSubscriptionStart`;
- serving authorization values: one non-`Clone`, non-`Copy`
  `ServingActivationAuthority` per produced generation; one caller-owned
  `RouteAcceptLease` per route driver; the exclusive `RouteAcceptClaim<'a>` plus
  `RouteAcceptClaimError`; and the non-`Clone`, non-`Copy`, lifetime-bound
  `RouteActivationPermit<'a>` created only by consuming that claim. None exposes
  a registry view or application dispatch capability;
- recoverable author inputs: `HostBindingRegistrationInput` and
  `StaticBindingRegistrationInput<B>`; validation rejection returns the
  complete input before publication or protocol side effects;
- installable units: `HostBindingRegistration` and
  `StaticBindingRegistration<B>`, each carrying compiler, execution,
  contribution, footprint, ingress, status, overflow, readiness, reactor,
  cleanup, capability, and profile-cell metadata as one validated startup
  bundle. Their constructors consume the input and its matching WP-200
  compiler component; no API installs that component by itself.
- Host and static complete registrations expose the same exact
  `try_with_collection_subscription_capability` and
  `collection_subscription_capability` methods, keyed only by
  `ObserveAllProperties` or `SubscribeAllEvents`; they do not infer native
  collection support from protocol text.

Implement the frozen contribution and runtime surfaces:

- `ServerFormContributor`, `AffordanceFormRequirement`, `FormContributionContext`,
  `FormContribution`, `FormContributionCapability`, `EndpointReservationKey`, and
  `CollisionDomainId`;
- `SubscriptionState`; the application `Subscription` facade and private `SubscriptionRecord`
  belong to WP-400 Servient;
- Preserve the orthogonality of `SubscriptionDriverEvent` fields: driver-slot lifecycle follows
  `SubscriptionDriverCleanupDisposition`, while `ProcessTerminal` is retained unchanged for the
  parent facade. A borrowed driver callback returns only `Complete`,
  `TransferRequired(CleanupTransferRequest)`, or
  `ResidualExternalState(CleanupRecord)`; it cannot return `PendingCleanup`. Complete cleanup
  retires the driver even when the process terminal is `Failed`; it must not be recoded as a
  driver residual.
- Implement the exact `binding-call` machine for host call records and every constrained typed
  slot header. Host constructors are nonblocking and side-effect-free, declare and report their
  complete lifetime footprint, and return owned `HostBindingCallBox` values. Cancellation binds
  a pre-admitted `CleanupReservation` into a phase-specific `CleanupPhaseContext`, retains the
  first cause, routes late request/subscription/response/publication results, and never drops a
  live call as cleanup.
- Make request, subscription-start, response, emission, and route cancellation return the
  portable `BindingCallSettlement<T, C>` shape. `Returned(T)` is the only normal or late-value
  branch. `Cancelled` retains `RetryClass` plus one
  `BindingCancellationDisposition<C>`: verified `Complete`, provisional `TransferRequired`, or
  `ResidualExternalState`. Route lifecycle calls fix `C` to
  `HostRouteCleanupSuccessor`; consumer calls use `T = CoreResult<U>`. No outer error may discard
  a typed successor. A `CleanupRecord` alone is status and is never transferable work.
- Implement the exact transfer handshake. `TransferRequired` leaves the complete call, guard,
  driver, input, or typed slot with the source. The source moves it into
  `CleanupTransferEnvelope<T>` and publishes `CleanupOutcome::PendingCleanup` only after
  `CleanupTransferTarget::try_accept` returns `CleanupTransferAcceptance::Accepted`. Rejection
  returns the identical envelope to the pre-reserved manual owner. For Host work, Servient binds
  the named owner from its admitted transfer reservation into `CleanupPhaseContext`; binding code
  consumes that production-derived carrier into `CleanupTransferRequest` and never derives an
  owner from Servient slot arithmetic. Manual fallback retains the same request and phase across
  Pending and retryable callback error. An accepted executor task
  that cannot finish commits its pre-reserved durable residual before destruction.
- `RuntimeEvent`, `BindingRuntimeEvent`, `BindingStatusRecord`, and `OverflowPolicy`;
- `ProducerEmission`, `EmissionKind`, `BindingPublication`, `EmissionStatus`, and
  `BindingEmissionSlot`;
- `BindingRouteState`, `InFlightState`, and the crate-private request, subscription, response,
  and emission slot state records.

All request, response, route, correlation, auth, payload, plan, binding-generation, and deadline
values are owned across an SPI call. Every consuming start has one typed pre-acceptance rejection
that returns the complete input. A registration carries the complete compiler/execution pairing,
identity, capabilities, readiness, diagnostics, ingress, reactor, status, overflow, cleanup, and
contributor metadata; a bare trait object is never the configuration contract.

## State and Ownership Migration

- A prepared route remains caller-addressable through every fallible readiness and activation
  outcome. Commit consumes an active guard and returns either
  `RouteCommitOutcome::Committed(HostCommittedRouteGuard)` or
  `RouteCommitOutcome::NotCommitted { guard: HostActiveRouteGuard, error }`; neither branch opens
  admission. Readiness failure or cancellation uses abort. Active and committed-closed cleanup
  uses shutdown through `HostShutdownRouteGuard`. `PendingCleanup` is returned only after the
  complete guard or call moves to and is acknowledged by the named cleanup owner.
- `BindingRouteState` follows the frozen route machine and never uses guard drop as a
  transition. Readiness, activation, commit to `CommittedClosed`, permit-authorized acceptance,
  abort, shutdown, and retry are idempotent for one route generation; late callbacks with stale
  generations are discarded and recorded. There is one accept cursor and waker lease per
  committed-closed route, never one registration-wide `poll_accept` cursor.
- Keep `ServingActivationAuthority` out of binding state. Servient owns one authority for the
  complete produced generation and makes it selectable only with the Producer plan set and
  serving registry generation in one atomic transition. Each host and constrained
  `poll_accept` receives a fresh borrowed `RouteActivationPermit<'_>` only after Servient moves
  the exact route accept lease into a claimed-call owner and consumes its exclusive
  `RouteAcceptClaim`. The permit cannot be retained in a guard, associated state,
  reactor queue, or detached task and no binding callback runs at publication.
- A drain transition stops permit issuance before `Draining` becomes observable. A poll claimed
  before the transition may return one request under its retained route and plan leases; later
  claims, stale wakes, and mismatched permits are rejected before binding state changes.
- Make preparation visibility and closed-ingress behavior part of the complete registration.
  Externally visible preparation declares exactly one policy: reject, backpressure, or buffer
  only within admitted binding ingress limits. Before publication no policy may create a response
  opportunity, report application acceptance, or emit an `InboundRequest`; buffered input stays
  route-owned through rollback and shutdown.
- Admit an in-flight response opportunity only after the serving state and generation recheck.
  Host send consumes it in the call; constrained start consumes it only after the response is
  accepted into `ServerResponseSlot`.
- A constrained request slot is generic over its binding's associated request state and is
  consumed by a terminal result. A successful subscription start instead retains its typed
  slot/generation as `Active`; start cancellation, item polling, stop, state destruction, and
  terminal retention use that same slot.
- `HostSubscriptionDriver` is the object-safe receive/stop SPI. It has one linear receive cursor,
  returns `SubscriptionItem` with the exact `SubscriptionId` and `AffordanceTarget`, and drives
  wire teardown through one accepted `SubscriptionStopRequest` and the same retained cleanup
  state. Explicit teardown carries a selected `OutboundRequest`; drop uses an implicit request
  and cannot invent caller options. Core provides no queue, sender, cloneable consumer, or merge
  policy.
- A `BindingEmissionSlot` owns one immutable payload lease and one selected binding generation's
  publication and cleanup state. Servient's private `EmissionRecord` owns local-subscriber and
  binding-target cursors; core poll methods never perform engine-wide fan-out.
- Translate a legacy handler-path publication into exactly one admitted `ProducerEmission` at
  the compatibility boundary. Preserve payload ownership, target, subscription, route, binding,
  and generation identity; the adapter may not clone an unbounded stream or publish directly to
  a concrete protocol. WP-400 removes the handler-side adapter entry and WP-600 removes the
  protocol-side adapter exit.
- Classify runtime events before overflow. Critical details update the bounded durable status
  record before a queued copy can be dropped; no payload, credentials, or redacted TD fields
  enter status storage.
- Invoke readiness, transport, contributor, guard, event-sink, and status callbacks outside
  engine locks and critical sections. Reserve response and cleanup progress before new work.
- Declare and admit the immutable maximum lifetime footprint for every call, guard, driver,
  artifact, slot state, reactor queue, and ingress buffer before first side effect. Bound external
  ingress independently per route, per binding, per Thing where applicable, and globally; no
  hidden transport queue may turn zero or an omitted limit into unbounded storage.

## Old API Removal

- Add the frozen route-scoped
  prepare/readiness/activate/commit/accept/abort/shutdown contract as
  `core/src/binding.rs::RouteServerBinding` inside a complete registration
  bundle. Keep the current `core/src/inbound.rs::ServerBinding` confined to the
  legacy generation until WP-400/WP-600 migrate its consumers; WP-700 removes
  its export. Remove any registration-wide acceptance and any cleanup path
  whose only completion signal is guard drop or an unstructured outer error
  in that owning removal scope.
- Remove any successful `RouteCommitOutcome::Serving` branch, any `poll_accept` overload that
  accepts an active guard or omits `RouteActivationPermit<'_>`, every per-route `open_gate` or
  `release_gate` callback, and every binding view of Servient registry state. Successful commit
  produces a committed-closed guard; only Servient's current shared authority can lend admission.
- Remove any preparation path with undeclared external visibility or an implicit closed-ingress
  policy. A complete registration that cannot enforce hidden preparation or one bounded declared
  reject, backpressure, or buffer policy is invalid.
- Replace the current `core/src/outbound.rs::ClientBinding` request shape with the frozen owned
  `OutboundRequest`, validated output, owned `HostBindingCall`, `HostSubscriptionDriver`, and
  `HostSubscriptionStart` contracts. Remove `BindingRequest` and `BindingFuture`; no public
  compatibility alias remains at package completion.
- Remove public `TransportRequest`, `TransportResponse`, and `TransportAdapter` facades that
  bypass compiled route matches or duplicate protocol binding ownership.
- Remove direct push paths from the new binding SPI and reject any new registration that can
  publish without `ProducerEmission`, bounded subscriber/binding results, and explicit overflow
  accounting. Retain only the named migration adapters needed by existing WP-400 and WP-600
  callers. WP-400 removes `PushFn` and the `SubscriptionSender` handler path after host activation;
  WP-600 removes `PublisherSink` after both concrete backends migrate; WP-700 verifies that none
  is public or referenced.
- Remove the core-owned queue `Subscription`, `SubscriptionGuard`, `SubscriptionSender`,
  `EventStream`, `Subscription::merge`, `EventBroker`, and `EventName` routing key. The staged
  concrete `PublisherSink` call sites must migrate through WP-600 and are absent from the final
  target surface.
- Remove binding vectors embedded in consumed Things and bare trait objects as the stored
- Remove independently installable `ServerBindingRegistration`, `ClientBindingRegistration`,
  `StaticServerBindingRegistration`, `StaticClientBindingRegistration`,
  `RuntimeEventSinkConfig`, and `BindingDrivingMode` targets. Component values remain usable for
  constructing and testing one complete `HostBindingRegistration` or
  `StaticBindingRegistration<B>` only.
- Do not restore the removed `ProtocolBinding` or `ClientBindingFactory` facades, and do not
  retain a binding-owned unbounded pending request, subscription, response, or emission table.

## Verification Responsibilities

Owning-crate unit, integration, compile-fail, model, and workload tests cover
these technical invariants:

- `property-read-binding-slice` for one complete mock registration, immutable
  property-read artifact, committed-closed route, permit-authorized accepted
  request, exactly-once response opportunity, cleanup, and proof that the mock
  has no Servient or application-handler dependency;
- `complete-binding-registration` for atomic compiler/execution/contributor/policy bundles,
  startup-only publication, rejection of incomplete bundles, and owned I/O values;
- `route-scoped-binding-lifecycle` for ownership-preserving route transitions, one accept/waker
  lease per committed-closed route, terminal isolation, and absence of direct handler dispatch;
- `serving-activation-permit-contract` for distinct committed-closed guards, exactly one shared
  authority per produced generation, atomic plan/registry/authority publication, fresh borrowed
  per-route permits created only from an exclusive `RouteAcceptLease` claim, zero unclaimed
  permits or duplicate concurrent claims, zero permit retention, drain-before-claim ordering,
  stale-permit rejection, bounded closed-ingress policies, and host/constrained trace equivalence;
- `typed-binding-state-storage` for associated-state layout limits, typed slots, generation-safe
  construction/destruction, and zero-budget retention;
- `binding-lifetime-and-ingress-memory` for declared lifetime/transient footprints, per-route,
  per-binding, per-Thing/global ingress saturation, and hidden-buffer detection;
- `response-emission-input-preservation` for typed pre-acceptance rejection, exactly-once
  post-acceptance settlement, late results, and host/constrained classification parity;
- `form-finalization-and-collision` for deterministic contributions and reservation identity;
- `binding-slot-state-model` for route, in-flight, subscription, response, and emission states;
- `bounded-response-subscription-emission` for start/poll/cancel and terminal retention;
- `binding-response-validation` for the response XOR, producer and consumer validation entry
  points, identity/branch checks, action invariants, and additional-response bounds;
- `drop-and-cleanup-ownership` for complete work-object transfer, handoff rejection/manual
  fallback, idempotent teardown, deadline progress, and durable residual state.
- `cleanup-kernel-implementability` for the common source-owned -> offered ->
  acknowledged-transfer/manual-return -> complete/residual machine, including
  synchronous `NoCleanupSuccessor` authors with no executor requirement.
- `producer-emission-migration` for the one-way legacy-adapter boundary, identity preservation,
  bounded admission, and proof that no new caller enters the bridge.
- `host-subscription-driver` for object safety, exact source attribution, one receive cursor,
  binding-owned flow control, stop/drop teardown, and absence of a core queue.
- `binding-emission-slot` for one-binding ownership, retained poll/cancel progress, stale
  generation rejection, and proof that Servient-wide fan-out is not stored in core.
- `binding-call-settlement` for constructor/poll/cancel races, late Returned routing, exact
  cleanup-reservation binding and acknowledged work transfer, declared footprint admission,
  zero-budget retry, and generation-safe slot reuse.
- `host-constrained-semantic-parity` for shared trace case ids, identical
  observable outcomes, semantic reservation/release deltas, normalized
  zero-budget/wake/deadline liveness, separately bounded physical costs,
  explicit applicability classes, and explicit compile-only `async-no-std`
  claims.
- `binding-semantic-trace-oracle` for one versioned machine-readable scenario
  source consumed unchanged by Host and constrained runners, with negative
  mutations for terminal class, owner, generation, resource delta, deadline,
  wake, and acknowledgement.
- `cleanup-obligation-coexistence` for the legal simultaneous-obligation
  matrix, non-additive mutually exclusive reservations, and saturation at each
  real coexistence boundary.
- `binding-authoring-usability` for the real-target Zenoh feedback probe's
  public declarations, actual network lifecycle, multiple Thing/route/form
  shape, diagnostics, workarounds, generic/layout/code-size, cleanup mapping,
  and unsafe/private dependency findings, with a technical disposition before
  aggregate fixture work begins.
- `target-legacy-no-backflow` for poisoned legacy selector,
  `ServerBinding::serve`, and `Dispatch` boundaries with zero
  target-generation calls.

Coverage also includes these requirement families:

- `frozen-cross-crate-surface` for object safety, static registrations, owned values, and every
  applicable feature cell;
- `binding-io-ownership` for route/correlation generation, response opportunities, one decode,
  and structured binding errors;
- `bounded-subscription-response-progress` for start, pending, cancel, active, delivery, and
  terminal slot behavior;
- `state-machines` for every legal/illegal binding, in-flight, and subscription transition and
  stale callback;
- `expose-failure-injection` for readiness, abort, activation, commit, shutdown, cleanup
  transfer, and residual external state at the SPI boundary;
- `form-finalization` for deterministic contributions, index pruning, collision identity,
  owner ambiguity, limits, and rollback values;
- `overflow-status` for event classes, loss counters, critical journals/compaction, and shutdown
  progress under exhaustion;
- `drop-and-emission` for exactly-once teardown, payload sharing, per-target order, partial
  outcomes, cancellation cursors, and cleanup ownership;
- `manual-runtime`, `host-independent-progress`, `interaction-call-path`, and
  `zero-allocation-paths` for bounded poll and erased-adapter behavior.

Fake binding tests must prove that callbacks can reenter without an engine guard and that no
terminal or critical status is lost merely because a bounded queue is full.

## Performance Workloads

- `PERF-GW-007`, `PERF-CS-007`, `PERF-GW-018`, and `PERF-GW-019` cover subscription start,
  hot delivery, stop, and cancellation progress.
- `PERF-GW-008`, `PERF-CS-008`, and `PERF-CS-009` cover the binding-local payload lease and
  bounded slot progress; Servient-wide fan-out is measured by WP-400.
- `PERF-GW-009` and `PERF-GW-010` compare host-erased and poll metadata/allocation paths.
- `PERF-GW-024` covers exact per-binding publication result scaling, and `PERF-CS-018` proves
  that a retained `BindingEmissionSlot` resumes within its work budget without restarting.
- `PERF-GW-028` covers the owned host-call cancellation, late-result, transfer, and residual
  matrix; `PERF-CS-020` covers typed slots and complete pre-acceptance input rejection.
- `PERF-GW-030` and `PERF-CS-022` cover pre-publication traffic, all-route commit, Nth-route
  commit failure, publication/cancellation orderings, stale permits, duplicate concurrent claims,
  attempts to create a permit without a claim, drain/claim orderings, all three externally visible
  closed-ingress policies, committed-guard retention, and identical host/constrained activation
  traces. They require one authority, atomic publication, an exclusive route-lease borrow, zero
  unclaimed permits, zero duplicate claims, zero pre-publication or partial admissions, zero
  post-drain claims, zero stale-permit mutations, zero lost committed guards, and zero retained
  permit bytes.
- `PERF-GW-031` validates a complete third-party registration and rejects every incomplete bundle.
- `PERF-GW-032` and `PERF-CS-023` cover bounded ingress items/bytes, backpressure, and hidden
  buffer detection at route, binding, Thing, and global scopes.

## Completion Conditions

- Every WP-300 ownership item exists at its frozen path and applicable feature cells; the
  no-default poll surface is useful and the host traits are object-safe.
- A third-party host fixture and a `no_std + alloc` static fixture compile using
  only documented public registration, compiler, contributor, client/server,
  slot, cancellation, and cleanup APIs. Both the mostly synchronous and
  externally-ready lifecycle shapes are covered without consulting a concrete
  protocol implementation.
- Complete host and static startup bundles are the only installable binding units; their compiler
  and execution compatibility, resource maxima, ingress policy, status policy, and supported
  profile cells are validated before snapshot publication.
- Core exposes no concrete subscription queue, merged stream, global emission coordinator, or
  dispatch policy; `BindingEmissionSlot` represents one binding generation only.
- Exhaustive transition tests cover route readiness through cleanup, in-flight admission and
  response consumption, subscription start/cancel/stop, emission poll/cancel, stale
  generations, and retained terminal outcomes.
- Activation tests prove that all required routes are committed-closed before the one serving
  publication, every accept poll carries a fresh borrowed permit from the exact current
  authority, drain stops new issuance, and visible pre-publication ingress follows its declared
  bounded policy without admitting an engine request.
- Slot/pool exhaustion returns backpressure before ownership transfer; accepted work reaches
  one terminal result without invoking the application handler again.
- Runtime event and durable status behavior remains bounded and preserves critical details
  under full-queue and full-journal cases.
- All listed workload adapters emit schema-valid, fixture-identified results for both poll and
  host-erased paths where applicable.
- The obsolete transport, push, split-registration, bare-registration, registration-wide accept,
  active-guard accept, per-route activation-gate, registry-observation, old-signature, opaque
  concrete-slot, and unbounded pending-work facades owned by WP-300 are absent, and no concrete
  protocol logic has entered core. Only the explicitly named WP-400/WP-600 compatibility adapter
  edges may remain, with compile and source evidence assigning their removal to those packages.
