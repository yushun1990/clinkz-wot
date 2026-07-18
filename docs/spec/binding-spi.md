# Protocol Binding SPI

Status: v4.9 architecture-closure candidate.

This specification is the single normative owner of Protocol Binding
registration and execution behavior. It refines the Protocol Binding boundary
in `docs/architecture/40-protocol-binding-spi-and-deployment.md`, the Servient
ownership rules in `docs/architecture/50-servient-runtime-lifecycle.md`, and
ADR-0006, ADR-0009, ADR-0010, and ADR-0011. Planning and compiled-plan-set
construction are owned by `docs/spec/planning.md`. Concrete protocol syntax and
I/O remain private to each binding crate.

## Stable requirements

`BIND-REG-001`: A Protocol Binding MUST enter one Servient through one complete,
generation-bearing registration bundle that atomically associates its identity,
configuration digest, capabilities, compiler extension, execution components,
form contribution, resource declarations, ingress policy, status policy, and
supported profile cells. Builder validation MUST reject an incomplete or
inconsistent bundle before publication. V1 composition is startup-only.

`BIND-ROUTE-001`: Producer execution MUST be engine-orchestrated and
route-scoped. Prepare, readiness, activation, commit, accept, abort, shutdown,
terminal reporting, and cleanup MUST identify one route generation, preserve
every guard across fallible transitions, and expose exactly one accept poll and
waker lease per active route. A binding MUST NOT receive an application dispatch
capability or call a handler from hidden work.

`BIND-STORAGE-001`: A constrained binding MUST expose associated protocol state
types and their maximum size, alignment, lifetime, and drop contract so the
caller can provide typed generation-bearing request, subscription, response,
emission, route, readiness, and cleanup slots. The binding MUST NOT replace
those slots with an unbounded or generation-unsafe side table.

`BIND-MEM-001`: Every binding-owned call, compiler cursor, artifact, prepared or
active route guard, readiness token, subscription driver, constrained state,
response/cancellation buffer, protocol-reactor queue, and ingress buffer MUST
declare an immutable maximum lifetime footprint before admission. Temporary
poll memory and external-input item/byte growth MUST be separately bounded at
per-route, per-binding, per-Thing where applicable, and global scopes.

`BIND-DELIVERY-001`: Response and emission delivery MUST preserve the complete
owned input and its response or publication opportunity on every failure before
acceptance. After acceptance, host and constrained representations MUST retain
the same exactly-once terminal result, cancellation, late-result, cleanup, and
retry classification; they MAY differ only in allocation and driving shape.

`BIND-CALL-CANCEL-001`: Every host binding operation that may remain pending
MUST return one owned, cancellation-aware call before its first protocol side
effect. Cancellation fixes one first cause and phase context, retains late
values, and reaches complete cleanup, acknowledged transfer of the complete
call, or durable residual state. Dropping a future, task, or call wrapper MUST
NOT be the cleanup protocol.

`BIND-HOST-CANCEL-001`: A binding call constructor MUST be nonblocking and
side-effect free until the engine has accepted its declared footprint and owns
the returned call. Constructor rejection certifies that no protocol resource or
cleanup obligation escaped. Once accepted, an operational error cannot certify
an empty cleanup obligation unless the call has actually settled it.

`BIND-IO-001`: `InboundRequest` and `InboundResponse` MUST own their route,
binding and route generations, correlation identity, plan identity, payload,
media/status metadata, and transport-authentication material across every SPI
call. A live correlation id is unique within one route generation. A binding
MUST validate route identity against its prepared route table and MUST NOT
borrow request or response data from a transport buffer after a call returns.

`BIND-OUT-001`: `OutboundRequest` MUST own only the selected binding and plan
identity plus per-call varying data. It MUST NOT contain a TD, raw form,
credential provider, mutable application options, or authority to select a
different candidate. A binding MUST NOT rescan the TD, reinterpret application
payload fields as credentials, weaken security, or perform implicit fallback.

`BIND-PROGRESS-001`: Pending client, server, subscription, response, emission,
readiness, and cleanup operations MUST retain one generation-bearing owner,
consume explicit work, use register-then-recheck wake semantics, make no
observable progress with a zero budget, and retain a terminal result until one
acknowledgement. A successful constrained subscription start keeps its slot
active; one-shot success consumes its slot only after terminal retention.

`LIFE-EXPOSE-001`: `expose` publication is one Servient-local transaction.
Externally visible protocol effects are governed by each route's declared
preparation visibility and cleanup semantics; the API MUST NOT claim global
network atomicity.

`LIFE-EXPOSE-002`: A complete server registration MUST declare whether route
preparation is externally visible and MUST provide an activation gate that can
hold all required routes non-serving until the Servient commit boundary. A
registration that cannot enforce that boundary is rejected in v1.

`LIFE-EXPOSE-003`: Every failed or cancelled expose phase MUST produce an exact
per-route disposition: verified complete cleanup, acknowledged pending transfer
of the complete guard/progress object, or durable residual external state. The
aggregate outcome MUST retain the first cause and every route disposition; it
MUST NOT collapse partial rollback into a generic error.

## Scope and ownership

The engine owns semantic identities, requests, results, admission, scheduling,
handler dispatch, status, and cleanup ownership. A concrete binding owns only:

- protocol syntax, route parsing, framing, correlation, and status mapping;
- protocol-local client, listener, session, and native multiplexing state;
- bounded protocol reactors and their wake integration;
- protocol-local retry and flow control within the selected WoT operation;
- extraction of transport-native authentication material; and
- the compiler extension and immutable artifact payload paired with its
  execution implementation.

A binding does not own the Servient registry, plan-set lifecycle, application
handles, cross-binding fairness, global emission coordination, a universal
subscription queue, W3C defaulting, or Directory service behavior.

All binding, provider, codec, contributor, and application callbacks run outside
engine locks and constrained critical sections. Returning `Pending` never gives
permission to detach semantic ownership into an unregistered task.

## Complete registration

The installable units are `HostBindingRegistration` for erased host execution
and `StaticBindingRegistration<B>` for a constrained binding implementation.
Each bundle contains one immutable registration identity with:

- `BindingId`, `BindingGeneration`, and `BindingConfigurationDigest`;
- one deterministic capability declaration and one
  `BindingCompilerExtension` with a matching compatibility identity;
- optional deterministic `ServerFormContributor` metadata;
- optional client and server execution components;
- supported compilation, execution, resource-profile, and capability-role
  cells;
- lifetime and transient footprint declarations for every supported role;
- per-route, per-binding, and global ingress item and byte declarations;
- status retention, overflow, reactor, readiness, and cleanup declarations; and
- a stable diagnostic registration ordinal that never resolves ownership
  ambiguity.

The bundle constructor validates internal equality of id, generation,
configuration digest, compiler compatibility, artifact compatibility, and
execution compatibility. No public API independently installs a compiler,
client half, server half, form contributor, or runtime trait object. Component
values may remain public for downstream construction and testing, but only the
complete bundle is accepted by `ServientBuilder`.

Both complete registration representations expose the same keyed capability
operations:

```rust
impl HostBindingRegistration {
    pub fn try_with_collection_subscription_capability(
        self,
        operation: Operation,
        capability: CollectionSubscriptionCapability,
    ) -> CoreResult<Self>;

    pub fn collection_subscription_capability(
        &self,
        operation: Operation,
    ) -> Option<CollectionSubscriptionCapability>;
}

impl<B> StaticBindingRegistration<B> {
    pub fn try_with_collection_subscription_capability(
        self,
        operation: Operation,
        capability: CollectionSubscriptionCapability,
    ) -> CoreResult<Self>;

    pub fn collection_subscription_capability(
        &self,
        operation: Operation,
    ) -> Option<CollectionSubscriptionCapability>;
}
```

Another operation or a duplicate incompatible capability is rejected without
changing the registration.

The builder rejects duplicate binding ids, duplicate generations in one id,
unsupported selected profile cells, missing execution support for an advertised
artifact role, ambiguous exclusive Producer ownership, invalid wildcard
declarations, incompatible collection capabilities, and any declared maximum
that cannot fit the selected resource profile. It freezes one immutable
`BindingRegistrationSnapshot` before returning the Servient.

V1 exposes no runtime add, remove, replace, or code-unload operation. A new
binding or configuration is deployed through a new application, process,
container, or firmware generation. Existing handles keep the registration and
plan-set generations they captured until drain and reclamation.

## Shared input and identity contract

`OutboundRequest` is created only after planning selected one candidate and
security application committed. It owns:

- binding id, binding generation, configuration digest, plan-set generation,
  plan id, binding-artifact reference, target, operation, and route identity;
- resolved target and caller URI-variable values;
- input payload and media metadata;
- typed committed `AppliedSecurity`, without credentials or provider handles;
- response-classification metadata;
- correlation, deadline, cancellation view, and optional idempotency metadata;
  and
- subscription start or teardown reservation identity when applicable.

Static target strings, schemas, security expressions, response tables,
extension maps, and URI-template programs remain behind the pinned plan
reference. The binding checks every generation and artifact compatibility
before protocol work starts.

`InboundRequest` owns one `BindingRouteKey`, exact `InboundRouteMatch`, binding
and route generations, plan-set and plan ids, correlation id, wire payload,
media metadata, URI-variable values, and `TransportAuthMaterial`. URI matching
and framing are binding work. Effective authorization, body-auth extraction,
schema validation, and application projection are core work performed against
the immutable route match.

`InboundResponse` owns the same route and correlation identities and exactly
one success output or structured error mapping. A response opportunity is
generation-bearing and single-use. Duplicate live correlation ids on one route
are rejected; unrelated route generations may reuse the wire value.

## Cleanup reservation and transfer

`CleanupReservation` is allocated before a side effect. It carries the maximum
item and byte reservation, durable-status reservation, owner class, and complete
identity seed needed by a possible cleanup obligation. Independent obligations
use independent reservations.

At cancellation, stop, abort, shutdown, or remote-terminal linearization, the
runtime binds one reservation into a `CleanupPhaseContext`. The context fixes:

- one `CleanupOperation`;
- the immutable first cause;
- subject, owner, binding, plan, route, and subscription generations as
  applicable;
- an independent drain deadline measured from that phase; and
- the admitted work and lifetime-footprint bounds.

Start cancellation, active subscription stop, remote-terminal cleanup,
readiness cancellation, prepared-route abort, active-route shutdown, response
cancellation, and emission cancellation are distinct operations. A context is
not reused or mutated into a later phase.

`BindingCallSettlement<T>` distinguishes a terminal value, verified complete
cleanup, a transfer request, and durable residual external state. A transfer
request is provisional: it carries the phase and bounded record but does not by
itself mean `PendingCleanup`. The runtime commits `PendingCleanup` only after it
moves the complete call, guard, driver, input, or typed slot into a named owner
that acknowledges capacity and responsibility. Executor or queue rejection
returns the complete object to a pre-reserved manual cleanup owner.

`CleanupRecord` is bounded durable identity and status, not the work object.
It cannot be polled and does not prove transfer. A pending owner retains one
progress lease, supplies deadline wakeups even when transport does not wake,
charges `WorkBudget`, and commits complete or residual status before destroying
the object outside locks. Zero budget retains the object without invoking
binding code. Destructors never block and are never the only cleanup path.

## Host binding calls

`HostBindingCall<T>` is the common erased host call role for client invoke,
subscription start, route lifecycle callbacks, response delivery, publication,
and cleanup. Its public contract provides:

- an immutable declared lifetime footprint available before admission;
- polling of one terminal `T` or structured operational failure;
- cancellation polling with `Context`, `CleanupPhaseContext`, and
  `WorkBudget`;
- a next-deadline or equivalent runtime wake contract; and
- transfer as one owned `HostBindingCallBox<T>`.

A constructor returns the owned box before the first protocol side effect.
First polling may commit a side effect. The footprint includes all retained
growth through late completion, cancellation, and cleanup and cannot shrink
after first poll. Actual retained footprint is verified before acceptance and
must never exceed the declaration.

Completion committed under the call lease wins a simultaneous cancellation.
Otherwise the first accepted cancellation fixes the phase context. A late
successful value remains in `BindingCallSettlement::LateValue` or an equivalent
owned terminal branch so the runtime can apply operation-specific cleanup and
retry classification. Cancellation never converts an unknown side effect into
`NoSideEffect`.

Dropping an application awaiter transfers only caller interest. Servient keeps
the call until settlement. Dropping the root runtime without explicit shutdown
does not certify external cleanup.

The host call and route outcome shapes are exact. `BindingCallSettlement` is
returned only after cancellation has linearized. `TransferRequired` is a
provisional request: the caller still owns the complete call until a named
cleanup owner acknowledges the handoff.

```rust
pub enum BindingCallSettlement<T> {
    TerminalValue(T),
    CleanupComplete(CleanupRecord),
    TransferRequired(CleanupTransferRequest),
    ResidualExternalState(CleanupRecord),
}

pub trait HostBindingCall<T>: Send {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint;

    fn poll_result(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<T>>;

    fn start_cancel(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<T>>>;

    fn poll_cancel(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<T>>>;

    fn next_deadline(&self) -> Option<Deadline>;
}

pub type HostBindingCallBox<T> = Pin<Box<dyn HostBindingCall<T>>>;

pub enum RoutePrepareOutcome<G> {
    Prepared(G),
    RejectedNoResource(BindingOperationalError),
}

pub enum RouteReadinessOutcome<G> {
    Ready(G),
    Failed {
        guard: G,
        error: BindingOperationalError,
    },
}

pub enum RouteActivationOutcome<P, A> {
    Active(A),
    NotActivated {
        guard: P,
        error: BindingOperationalError,
    },
}

pub enum RouteCommitOutcome<A> {
    Serving(A),
    Failed {
        guard: A,
        error: BindingOperationalError,
    },
}
```

`CleanupTransferRequest` contains bounded identity and requested-owner data,
not the call itself. The caller converts it into `PendingCleanup` only while
atomically moving `HostBindingCallBox<T>` into the acknowledged owner. A
rejected handoff leaves the box with the source owner and is not observable as
pending cleanup.

## Server route SPI

### Route identities and guards

One frozen inbound plan yields one `BindingRouteKey` and one
`RouteReservationIdentity` composed of `CollisionDomainId` and
`EndpointReservationKey`. Collision identity is independent of registration
generation; an old prepared, active, draining, or cleanup-pending owner blocks
reuse until terminal disposition.

Host prepared and active guards are downstream-constructible owned erased
values. Each exposes its exact binding and route generations, reservation
identity, immutable lifetime footprint, and an `into` operation that transfers
its private binding state exactly once. Static counterparts use typed
caller-owned route slots. No guard relies on `Drop` as a lifecycle event.

### Lifecycle calls

The host server component uses owned calls for every callback that can remain
pending:

1. prepare accepts one `PrepareInput` and returns a call whose terminal outcome
   is a prepared guard or a certified no-resource rejection;
2. readiness moves that prepared guard into one
   `HostBindingCallBox<RouteReadinessOutcome<HostPreparedRouteGuard>>` and
   returns ready with the same guard, failure with the same abortable guard, or
   a cleanup settlement;
3. activation accepts a prepared guard and returns either an active guard or an
   explicit non-activated outcome retaining the prepared guard;
4. commit accepts the active guard and returns committed serving ownership or a
   failure retaining an active guard suitable for shutdown;
5. abort consumes a prepared guard and reaches complete, acknowledged transfer,
   or residual state; and
6. shutdown consumes an active/serving guard and reaches the same three cleanup
   dispositions.

An outer invalid-call error occurs before ownership transfer and returns the
original input through `BindingInputRejection<T>`. Operational failures are
typed call outcomes so the predecessor or successor guard cannot disappear.
Cancellation retains the call until a late guard is classified and sent to the
stage-appropriate abort or shutdown path.

Readiness does not define a second public host driver. Its retained
`HostBindingCall` is the unique progress object and owns the prepared guard
until it returns that guard or the complete call is transferred for cleanup.
Servient polls all readiness calls fairly under one expose deadline and bounded
per-owner quantum. Polling registers wake interest before rechecking state. A
never-ready route does not block other routes from readiness or cancellation.

The host server surface has the following exact ownership signatures. The
fields of `RouteAbortInput` and `RouteShutdownInput` are private; their
constructors consume the complete guard and phase context, and their
`into_parts` accessors return both exactly once.

```rust
pub struct RouteAbortInput { /* prepared guard plus cleanup phase */ }
pub struct RouteShutdownInput { /* active guard plus cleanup phase */ }

pub trait ServerBinding: Send + Sync {
    fn prepare(
        &self,
        input: PrepareInput,
    ) -> Result<
        HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>>,
        BindingInputRejection<PrepareInput>,
    >;

    fn start_readiness(
        &self,
        guard: HostPreparedRouteGuard,
    ) -> Result<
        HostBindingCallBox<RouteReadinessOutcome<HostPreparedRouteGuard>>,
        BindingInputRejection<HostPreparedRouteGuard>,
    >;

    fn activate(
        &self,
        guard: HostPreparedRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
        >,
        BindingInputRejection<HostPreparedRouteGuard>,
    >;

    fn commit(
        &self,
        guard: HostActiveRouteGuard,
    ) -> Result<
        HostBindingCallBox<RouteCommitOutcome<HostActiveRouteGuard>>,
        BindingInputRejection<HostActiveRouteGuard>,
    >;

    fn poll_accept(
        &self,
        route: Pin<&mut HostActiveRouteGuard>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>>;

    fn abort(
        &self,
        input: RouteAbortInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome>,
        BindingInputRejection<RouteAbortInput>,
    >;

    fn shutdown(
        &self,
        input: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome>,
        BindingInputRejection<RouteShutdownInput>,
    >;

    fn deliver_response(
        &self,
        response: InboundResponse,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<InboundResponse>,
    >;

    fn publish(
        &self,
        publication: BindingPublication,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<BindingPublication>,
    >;
}
```

No method above may return a plain operational error after consuming its owned
input. `CoreResult` from polling is limited to a stale or invalid call that did
not change the call/guard state; protocol and lifecycle failures appear in the
typed outcome and therefore retain the required predecessor or successor.

### Commit and acceptance

The activation gate must hold every required route non-serving until Servient's
registry commit. A route that cannot enforce the gate is not a v1 registration.
There is no post-publication advertise phase that can fail outside the expose
transaction.

`poll_accept` is scoped to one active route guard. It returns exactly one:

- `RouteAcceptEvent::Request(InboundRequest)`;
- `RouteAcceptEvent::OperationalError(BindingOperationalError)`; or
- `RouteAcceptEvent::Terminal(RouteTerminal)`.

Every event carries the route generation. One route has one mutable accept
cursor and one waker owner. A terminal event is emitted at most once, closes
later acceptance for that route, and does not terminate a sibling route or the
whole registration. Operational errors update bounded status but do not imply
terminal state.

Destroy marks the route draining before shutdown, so no new request can be
admitted. Requests accepted before that linearization retain their plan and
route leases through response settlement.

## Expose transaction

Planning freezes the Producer plan set and exact route owners before the first
route side effect. Servient then:

1. reserves all route, guard, readiness, ingress, in-flight, response, status,
   and cleanup capacity;
2. starts route preparation outside locks and retains every call lease;
3. fairly drives readiness under one deadline;
4. activates all routes behind their gates;
5. commits all routes;
6. atomically publishes the produced record, plan set, and serving gates; and
7. releases provisional admission state.

Any failure fixes one first cause, closes new callback admission, cancels or
joins outstanding calls, classifies late guards, and drives every route through
abort or shutdown. The returned aggregate identifies each route as complete,
transferred pending, or residual. A `PendingCleanup` route names the
acknowledged owner of its complete object. Local publication never hides a
partial rollback result.

Preparation visibility is explicit registration metadata. An externally
visible prepared endpoint may exist before local publication, but it cannot
serve requests before the activation gate. That limited external visibility is
reported in diagnostics and does not weaken rollback accounting.

## Client execution and subscriptions

The host client component exposes `invoke` and `subscribe`. Each accepts one
owned `OutboundRequest` and returns an admitted `HostBindingCallBox` before its
first protocol side effect. Unsupported operations reject without side effects.

`invoke` has one terminal validated `InteractionOutput` or structured failure.
The binding maps wire status and metadata, and the shared response validator
classifies primary and additional responses. Transport success is not
automatically WoT success. Protocol retry remains binding-local and never
reselects a form or repeats application behavior.

`subscribe` succeeds only after start response validation and returns
`HostSubscriptionStart` containing the exact engine-reserved metadata and one
owned `HostSubscriptionDriver`. An error certifies no driver, remote resource,
or cleanup obligation remains; otherwise cleanup is a call settlement, not a
plain error.

One driver owns one receive cursor, protocol resource, native flow control, and
binding-local cleanup state. It is not a cloneable handle or universal queue.
Its item event always contains the exact `SubscriptionId`, source
`AffordanceTarget`, and payload. Collection subscription uses one selected
Thing-root form and exact source attribution; remote fan-out or multiplexing
stays inside the binding.

Process termination and resource cleanup are orthogonal. A driver retains the
first `ProcessTerminal`, then starts or joins one phase-specific cleanup
operation. It publishes one terminal driver event only with a `CleanupOutcome`
that is complete, acknowledged pending, or residual. Process failure with
complete cleanup is closed resource state, not residual resource state.

Explicit stop and implicit drop each create a new `CleanupPhaseContext`.
`start_stop` receives a `Context`, exact `SubscriptionStopRequest`, phase
context, and work budget, registers wake interest before rechecking, and
accepts the request at most once. A remote terminal racing with stop retains one
process cause and joins the same resource cleanup without reusing the start-call
context.

The portable terminal and host driver roles have this exact shape:

```rust
pub enum SubscriptionDriverEvent {
    Item(SubscriptionItem),
    Terminal {
        terminal: ProcessTerminal,
        cleanup: CleanupOutcome,
    },
}

pub trait HostSubscriptionDriver: Send {
    fn poll_item(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<SubscriptionDriverEvent>>;

    fn start_stop(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        request: SubscriptionStopRequest,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<CleanupOutcome>>;

    fn poll_stop(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<CleanupOutcome>>;
}
```

The fields of `SubscriptionDriverEvent` are exact. Operational transport,
validation, timeout, overflow, cancellation, or remote failure is retained in
the terminal value; the outer result is reserved for a stale identity or
invalid call that does not change driver state.

## Response and emission delivery

Host response delivery is an owned call. Before the call is accepted, an
invalid route, stale generation, capacity failure, or backpressure result is
`BindingInputRejection<InboundResponse>` and returns the complete response and
opportunity. Once accepted, the call owns both and reaches exactly one delivery
result, cancellation settlement, late result, or residual.

Constrained `start_response` follows the same boundary: it either completes
synchronously, transfers the response into the caller-owned response slot, or
returns `BindingInputRejection<InboundResponse>`. `poll_response` and
`poll_cancel_response` operate only after acceptance. The application handler
is never invoked again to retry delivery.

Producer publication receives one selected `BindingPublication` and one
immutable payload lease per binding generation. Host publication returns an
owned call. Constrained `start_emission` completes, transfers the full input to
the emission slot, or returns `BindingInputRejection<BindingPublication>`.
Cross-binding and local-subscriber scheduling remain Servient work. Protocol
remote fan-out and retry remain binding-local.

Response and emission terminal classifications are identical across host and
constrained forms. Neither path may report backpressure after consuming an
input without retaining it in an admitted owner.

## Constrained associated-state SPI

Constrained client and server traits use associated state types rather than
private concrete core slot payloads. The semantic shape is:

```rust
pub trait PollClientBinding {
    type RequestState;
    type SubscriptionState;

    fn request_state_layout(&self) -> BindingStateLayout;
    fn subscription_state_layout(&self) -> BindingStateLayout;

    fn start_subscription(
        &mut self,
        request: OutboundRequest,
        slot: &mut ClientSubscriptionSlot<Self::SubscriptionState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<SubscriptionStart>>;

    fn poll_subscription_start(
        &mut self,
        cx: &mut Context<'_>,
        subscription: &mut ClientSubscriptionSlot<Self::SubscriptionState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<SubscriptionStart>>;

    fn start_subscription_stop(
        &mut self,
        cx: &mut Context<'_>,
        request: SubscriptionStopRequest,
        cleanup: CleanupPhaseContext,
        slot: &mut ClientSubscriptionSlot<Self::SubscriptionState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<CleanupOutcome>>;

    // The request/subscription start, poll, cancel, item, and terminal
    // acknowledgement methods use the same caller-owned typed slots.
}

pub trait PollServerBinding {
    type RouteState;
    type ReadinessState;
    type ResponseState;
    type EmissionState;

    fn route_state_layout(&self) -> BindingStateLayout;
    fn readiness_state_layout(&self) -> BindingStateLayout;
    fn response_state_layout(&self) -> BindingStateLayout;
    fn emission_state_layout(&self) -> BindingStateLayout;
    // Prepare, accept, delivery, publication, cancellation, and cleanup use
    // caller-owned typed slots.
}
```

The comment placeholders above describe method families, not permission to
omit them. The API ownership matrix freezes their exact public names and paths.
Every family has start, poll/step, cancellation where applicable, terminal
acknowledgement, and explicit cleanup operations using the same semantic input
and result types as the host SPI.

`BindingStateLayout` declares maximum size, alignment, immutable lifetime
footprint, transient-per-poll bound, and whether state destruction is trivial
after terminal acknowledgement. A registration's static maximum is validated
against the caller-provided storage before a start.

Typed slots are generic over their binding state and carry a core-owned header:
slot index, slot generation, operation state, identity references, admitted
footprint, first cause, cleanup owner, and retained terminal result. Generic
struct definitions do not require behavior bounds; method implementations place
bounds only where an operation needs them.

The binding constructs and destroys its associated state in caller storage
through safe public operations or a separately reviewed unsafe abstraction with
documented invariants. Reuse increments the generation only after the terminal
result is acknowledged, cleanup is complete or residual is durable, and state
drop has run outside the critical section. A stale token cannot observe or
destroy reused state.

With zero work budget, a step performs no binding callback and leaves state
unchanged. One step cannot exceed its declared work quantum. Fair scheduling is
owned by the caller; a binding cannot scan or advance unrelated slots as a side
effect of polling one token.

## Memory, ingress, and reactor bounds

The active resource schema must separately cover at least:

- prepared and active route counts and guard bytes;
- readiness token counts, bytes, work quantum, and timeout;
- per-route, per-binding, and global ingress items and bytes;
- host call counts and bytes per item, binding, Thing, and global scope;
- installed subscription-driver counts and bytes;
- constrained state bytes per item, Thing, and global scope;
- response and cancellation buffers;
- transient poll bytes per call and globally;
- cleanup reservations, manual cleanup slots, tasks, records, and bytes; and
- durable status, critical event, wake lease, and reactor queue capacity.

Ingress is admitted before route activation. External input that exceeds a
route limit applies that route's explicit backpressure, rejection, or terminal
overflow policy without blocking unrelated routes. A binding cannot hide an
unbounded transport-runtime channel behind `poll_accept`.

A lifetime declaration includes worst-case growth after the first poll and all
cancellation and cleanup state. Shared payload leases are charged once to their
owner and referenced by bounded leases; bindings do not evade the global ledger
through unreported transport-library or reactor buffers. Temporary memory is
charged while live but is not double-counted as lifetime storage.

Zero never means unbounded. A disabled capability cannot be started. A
declaration or actual footprint overrun is a binding contract violation and is
reported before accepting new work where possible; already accepted ownership
still follows cleanup and residual rules.

## Wake, deadlines, and fairness

Every pending operation either registers a waker and rechecks progress or is
documented as manual-progress-only in its selected execution cell. Servient or
the static runtime always supplies deadline progress independently of protocol
wakes. A wake contains no authority; generation and lease validation precede
state mutation.

One route, call, subscription, response, emission, or cleanup owner receives at
most the configured work quantum before the scheduler advances its retained
cursor. A slow or never-waking binding does not indefinitely block another
binding or route. Protocol ordering within one owner is preserved.

Callbacks that return pending retain all inputs and do not require the caller
to retry a consuming start. Busy retry loops and unbounded ready scans are not
conforming progress mechanisms.

## State and outcome projection

Machine-readable state artifacts must project at least:

- complete registration validation and immutable snapshot publication;
- route preparation, readiness, activation, commit, serving, drain, direct
  complete cleanup, acknowledged transfer, residual, and late guard results;
- host call construction, first poll, completion/cancellation race,
  transfer-required, transfer-committed, drain expiry, residual, and terminal
  acknowledgement;
- client request and active subscription slots with stale-generation rejection;
- response and emission rejection before acceptance and exactly-once terminal
  settlement after acceptance;
- process terminal separated from subscription cleanup phase; and
- cleanup task offer, acceptance, rejection/manual fallback, deadline wake,
  executor drop, residual commitment, and acknowledgement.

Every public or crate-private ownership `state_record` in the API matrix must be
covered by one machine or an explicit composition role. State reachability
alone is insufficient: checkers validate outcome-specific ownership and reject
mutations that remove returned inputs, guards, transfer acknowledgement, direct
complete cleanup, residual cleanup, or wake registration.

## API ownership roles

The API ownership matrix provides exact public paths. It must represent these
roles without creating a dependency from core to planning, Servient, or a
concrete binding:

| Role | Defining owner |
| --- | --- |
| Complete host/static registration, registration identity, capabilities, route/call/driver/ingress footprints, state layout, requests, outcomes, guards, calls, drivers, and poll traits | `clinkz-wot-core` |
| Compiler-extension and artifact envelope/reference SPI | `clinkz-wot-core` |
| Effective-form compiler coordination | `clinkz-wot-planning` |
| Registration snapshot, route/call/slot registries, scheduling, cleanup tasks, status facade, and application handles | `clinkz-wot-servient` |
| Protocol state types, compiler payload, client/server implementations, and bounded reactor | Concrete binding crate |

`BindingDrivingMode`, a general binding `Dispatch`, independently installable
client/server registrations, `RuntimeEventSinkConfig`, `ProtocolBinding`,
`ClientBindingFactory`, `BindingRequest`, universal event queues, and bare
trait-object builder registration are not target APIs.

## Required evidence

Evidence uses deterministic virtual time, fixed allocator/accounting probes,
fixed binding/configuration generations, and exact manifest and fixture
identities. At minimum it covers:

- a fake third-party binding crate outside the workspace member list that
  constructs one complete bundle and supports consume and expose without
  umbrella changes;
- duplicate, incomplete, incompatible, unsupported-cell, and over-footprint
  bundle rejection before publication;
- prepare/readiness/activate/commit failure and cancellation at every boundary,
  including late prepared/active guards and direct complete, transferred, and
  residual rollback;
- many-route fairness with one never-ready route, one accept waker per route,
  route-terminal isolation, and commit/drain admission boundaries;
- host invoke, subscribe, response, and publication cancellation races,
  late values, drain expiry, executor accept/reject/drop, manual fallback, and
  zero lost owners;
- every constrained associated-state slot at layout limits, zero budget,
  stale generation, reuse, cancellation, typed rejection, and state drop;
- response/emission input preservation on every pre-acceptance failure and
  aligned host/static terminal classifications;
- lifetime footprint at the declared maximum and one byte over, including
  hidden-buffer detection;
- per-route, per-binding, and global ingress item/byte saturation while an
  unrelated route continues; and
- explicit shutdown with zero unowned live calls, guards, drivers, slots,
  cleanup tasks, or unrecorded residual state.

No benchmark or inspection report closes a gate unless its workload identity,
profile, feature cell, registration set, limits, policy, clock, allocator, and
expected ownership counters are fixed by the registered performance artifacts.
