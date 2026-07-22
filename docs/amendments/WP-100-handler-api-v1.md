# WP-100 Handler API, Execution, and Staging Amendment

Status: Frozen

Base design revision: v4.9

Amendment id: WP-100-HANDLER-API-001

Affected requirements: API-OWNERSHIP-001, API-PAYLOAD-001,
API-RESOURCE-001, API-SURFACE-001, API-TYPES-001, API-HOT-ID-001,
BIND-PROGRESS-001, CLEANUP-RECORD-001, HANDLE-DROP-001,
HANDLER-API-001, HANDLER-SUB-001, HANDLER-VALUE-001,
HANDLER-CANCEL-001, HANDLER-CANCEL-002, HANDLER-STORAGE-001, HOST-ASYNC-001,
HOST-DEFAULT-001, CONCUR-LIN-001, CONCUR-USER-001,
CONSTRAINED-OWN-001, CONSTRAINED-PROGRESS-001,
CONSTRAINED-WORK-001, STATE-INFLIGHT-001, PRODUCER-EMIT-001,
RES-LIMIT-001, RES-PROFILE-001, TIME-001, PERF-BENCH-001,
IMPL-CONFORM-001

## Purpose and Precedence

This normative amendment closes the handler decisions identified by
`docs/audits/WP-100-handler-entry-audit.md`. It freezes the exact request and
context ownership split, the eighteen-operation sync/async/step matrix,
registration boundaries, cancellation ownership, Producer subscription
transaction, resource limits, workload identities, and acyclic package
staging.

The amendment remains frozen for those non-time contracts. Its `Deadline`
material is a deferred candidate projection, not implementation authorization
and not a resolution of the open time-domain semantics. Any later clause that
mentions deadline expiry is conditional on clearing the
`TIME-DOMAIN-AND-DEADLINE` blocking scope.

Where this amendment is more specific, it supersedes the open alternatives in
`HANDLER-API-001`, `HANDLER-SUB-001`, `HANDLER-CANCEL-001`,
`HANDLER-CANCEL-002`, and `HANDLER-STORAGE-001`. It does not move logical-plan
construction from WP-200, binding-route construction from WP-300, or integrated
Servient lifecycle coordination from WP-400.

## Request and Handler Values

`InteractionInput` is the sole owner of application-visible request facts.
`HandlerContext` does not duplicate payload, principal, URI variables, accept
preferences, correlation, deadline, cancellation, action invocation, or
subscription identity.

`AcceptHint` is the exact bounded, protocol-neutral representation of an
inbound request's ordered response-media preferences. It contains no raw
header, quality-value parser, binding metadata, or protocol-specific token:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptHint {
    preferred: MediaType,
    alternatives: alloc::vec::Vec<MediaType>,
    retained_bytes: u64,
}

impl AcceptHint {
    pub fn try_single(
        preferred: MediaType,
        limits: &clinkz_wot_foundation::ResourceLimits,
    ) -> CoreResult<Self>;

    pub fn try_new(
        preferred: MediaType,
        alternatives: &[MediaType],
        limits: &clinkz_wot_foundation::ResourceLimits,
    ) -> CoreResult<Self>;

    pub fn preferred(&self) -> &MediaType;
    pub fn alternatives(&self) -> &[MediaType];
    pub const fn retained_bytes(&self) -> u64;
    pub fn accepts(&self, content_type: &str) -> bool;
}
```

The preferred value is always present. `alternatives` preserves caller order,
including repeated values, and uses an empty vector rather than a second
optional state. `AcceptHint` has private fields and deliberately has no
`Default`, public field mutation, transport-header conversion, or implicit
unbounded `IntoIterator` constructor. Both constructors work under
`no_std + alloc`; `try_single` is exactly `try_new` with an empty slice.

Construction first checks `1 + alternatives.len()` against
`form_binding_candidates_per_operation_max` without walking the slice. This is
the existing bound on the response/form candidates that one admitted operation
may inspect, so preferences beyond that count cannot affect a conforming
selection. It then computes a checked logical retained-byte total consisting of
the preferred string bytes, every alternative string byte, and one
`size_of::<MediaType>()` record per alternative. That total must individually
fit both `handler_state_bytes_per_thing_max` and
`handler_state_bytes_global_max`. Only after those checks pass may the bounded
slice be cloned. A missing applicable limit is `UnsupportedOperation`; a count,
byte, or checked-arithmetic excess is `LimitExceeded` for the exact limiting
`ResourceKind`. The constructor does not reserve live ledger capacity.

Before handler entry, dispatch charges `retained_bytes` as engine-owned input
storage against the same per-Thing and global handler-state transaction that
retains the `InteractionInput`. The fixed `AcceptHint`, `InteractionInput`, and
vector-header records are charged as normal engine owner overhead and are not
declared application `HandlerFootprint` bytes. This reuses the response-candidate
and handler-state limits according to their existing ownership semantics; it
does not add an Accept-specific resource row. Bindings must apply the same
count-first rule while tokenizing a raw Accept representation and must not build
an unbounded temporary preference vector before calling `try_new`.

### Frozen passive handler-value domain

`HANDLER-VALUE-001`: The handler value domain normatively freezes exactly five
passive public Core
schemas: `CancellationView`, `SubscriptionAcceptance`, `HandlerFootprint`,
`HandlerStep<R>`, and `StaticHandlerRegistration<'h, H>`. It completely owns
their schema attributes, derives, discriminants, variants, private fields,
const API, move/borrow ownership, generic bounds, redacted Debug shape, and
passive value semantics as specified in this amendment. The latter four
declarations appear in their applicable result, bounded-step, and registration
sections below.

This requirement does not own cancellation state transitions, deadline
comparison, time-domain behavior, handler execution, storage admission, or
subscription processing, work-budget charging, ownership admission, or resource
accounting. Those consumer behaviors retain their existing fine-grained
requirements on the consuming traits, state records, and admission items; they
are not assigned to the five passive API ownership rows. The five-value domain
and the `TIME-DOMAIN-AND-DEADLINE` blocking scope share only the global
`API-SURFACE-001` meta-requirement; their API-item and behavioral requirements
are otherwise disjoint.

The exact `CancellationView` schema is:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CancellationView {
    #[default]
    Active,
    Requested,
}

impl CancellationView {
    pub const fn is_requested(self) -> bool;
}
```

`CancellationView` is a copyable snapshot. It contains no pointer, atomic,
waker registration, callback, executor dependency, or cancellation transition.
It does not itself wake or cancel work. The authoritative execution record
remains engine-owned under the cancellation requirements.

### Deferred Deadline candidate

The API ownership freeze for `Deadline` establishes only its Core owner,
definition/public paths, and candidate value shape. It does not freeze the
clock comparison domain, raw-wrap behavior, reset/clock-id lifetime, cleanup
timing, dispatcher error disposition, or completion evidence. The following
shape is retained for review traceability and must not be implemented until
future corrective work is defined, independently admitted, and clears the
`TIME-DOMAIN-AND-DEADLINE` blocking scope.

The deferred `Deadline` candidate shape is:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Deadline {
    instant: Option<clinkz_wot_foundation::MonotonicInstant>,
}

impl Deadline {
    pub const NONE: Self = Self { instant: None };

    pub const fn at(
        instant: clinkz_wot_foundation::MonotonicInstant,
    ) -> Self;

    pub const fn instant(
        self,
    ) -> Option<clinkz_wot_foundation::MonotonicInstant>;

    pub fn checked_is_elapsed_at(
        self,
        now: clinkz_wot_foundation::MonotonicInstant,
    ) -> Option<bool>;
}
```

Under the preferred non-wrapping logical-time direction, the candidate behavior
would make `Deadline::NONE.checked_is_elapsed_at(now)` return `Some(false)`, a
finite deadline elapsed when `now >= deadline`, and different clock identities
return `None`. The `checked_` name reflects that incomparability is not a
Boolean result. These are deferred candidate semantics, not a current
implementation contract. In particular, the exact dispatcher error category,
phase, retry class, and behavior for incomparability remain unfrozen.

The preferred comparison direction requires same-clock `MonotonicInstant`
values to be in one non-wrapping logical tick domain and never infers ordering
from raw wrapping ticks or `RuntimeClock::wrap_period_ticks`. The
`TIME-DOMAIN-AND-DEADLINE` record is only a blocking impact placeholder for the
foundation clock-source contract, cleanup timing, prior time evidence, and
error disposition. It does not define a corrective tranche. The identity,
ownership, dependencies, completion contract, and evidence disposition of
future corrective work remain unfrozen before `Deadline` can enter
implementation.

The final v1 request value has private fields and no `Default` implementation:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct InteractionInput {
    data: Option<Payload>,
    uri_variables: BTreeMap<String, String>,
    principal: Option<Principal>,
    accept: Option<AcceptHint>,
    correlation: CorrelationId,
    deadline: Deadline,
    cancellation: CancellationView,
    action_invocation: Option<ActionInvocationRef>,
    subscription: Option<SubscriptionId>,
}

impl InteractionInput {
    pub fn new(correlation: CorrelationId) -> Self;
    pub fn with_data(correlation: CorrelationId, data: Payload) -> Self;

    pub fn with_uri_variables(
        self,
        uri_variables: BTreeMap<String, String>,
    ) -> Self;
    pub fn with_principal(self, principal: Principal) -> Self;
    pub fn with_accept(self, accept: AcceptHint) -> Self;
    pub fn with_deadline(self, deadline: Deadline) -> Self;
    pub fn with_cancellation(self, cancellation: CancellationView) -> Self;
    pub fn with_action_invocation(
        self,
        action_invocation: ActionInvocationRef,
    ) -> Self;
    pub fn with_subscription(self, subscription: SubscriptionId) -> Self;

    pub fn data(&self) -> Option<&Payload>;
    pub fn uri_variables(&self) -> &BTreeMap<String, String>;
    pub fn principal(&self) -> Option<&Principal>;
    pub fn accept(&self) -> Option<&AcceptHint>;
    pub const fn correlation(&self) -> CorrelationId;
    pub const fn deadline(&self) -> Deadline;
    pub const fn cancellation(&self) -> CancellationView;
    pub const fn action_invocation(&self) -> Option<ActionInvocationRef>;
    pub const fn subscription(&self) -> Option<SubscriptionId>;
}
```

The in-flight handler-call owner owns the `InteractionInput`; handlers borrow
it. No field borrows a binding call stack. Public construction creates data and
does not prove authentication, authorization, plan admission, or live identity.
Only dispatch through an admitted engine boundary guarantees that the
principal, variables, correlation, action reference, and subscription id were
validated.

The builder methods replace the corresponding optional field. A binding or
dispatcher may rebuild a request snapshot between bounded steps, but it never
mutates a snapshot while application code borrows it. Bulk-operation inputs
remain encoded in `data`; WP-100 does not invent WP-200 aggregate-plan or
per-property result schemas.

## Handler Context and Target Representation

`HandlerContext` contains dispatch identity only:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandlerContext<'a> {
    thing_id: &'a ThingId,
    thing_slot: ThingSlotId,
    target: &'a AffordanceTarget,
    operation: clinkz_wot_td::data_type::Operation,
    plan_id: PlanId,
    binding: Option<(BindingId, BindingGeneration)>,
}

impl<'a> HandlerContext<'a> {
    pub fn try_new(
        thing_id: &'a ThingId,
        thing_slot: ThingSlotId,
        target: &'a AffordanceTarget,
        operation: clinkz_wot_td::data_type::Operation,
        plan_id: PlanId,
        binding: Option<(BindingId, BindingGeneration)>,
    ) -> CoreResult<Self>;

    pub const fn thing_id(self) -> &'a ThingId;
    pub const fn thing_slot(self) -> ThingSlotId;
    pub const fn target(self) -> &'a AffordanceTarget;
    pub const fn operation(self) -> clinkz_wot_td::data_type::Operation;
    pub const fn plan_id(self) -> PlanId;
    pub const fn binding(self) -> Option<(BindingId, BindingGeneration)>;
}
```

`try_new` validates only operation/target-kind compatibility. It does not prove
that public ids are live. Property operations require a Property target, action
operations require an Action target, event operations require an Event target,
and every Thing/collection operation requires `AffordanceTarget::Thing`.
Admitted dispatch additionally validates every id and generation before it
constructs the context.

`HandlerContext` deliberately does not implement `Hash`. Hashing its borrowed
`ThingId` and `AffordanceTarget` would walk human-readable strings and would
invite its use as a hot record key. Engine tables use `ThingSlotId`,
`AffordanceSlotId`, and `PlanId` instead.

The canonical paths remain `clinkz_wot_core::AffordanceKind` and
`clinkz_wot_core::AffordanceTarget`. Their v1 schemas and methods are:

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AffordanceKind {
    Property,
    Action,
    Event,
}

impl AffordanceKind {
    pub const fn as_str(self) -> &'static str;
}

impl core::fmt::Display for AffordanceKind {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AffordanceTarget {
    Thing,
    Property(String),
    Action(String),
    Event(String),
}

impl AffordanceTarget {
    pub fn name(&self) -> Option<&str>;
    pub fn kind(&self) -> Option<AffordanceKind>;
}
```

The defining implementation relocates from `thing` to `interaction`, while
both existing module paths, `clinkz_wot_core::thing::AffordanceKind` and
`clinkz_wot_core::thing::AffordanceTarget`, remain public re-exports for v1.
The ownership action is therefore `relocate` for `AffordanceKind` and `replace`
for `AffordanceTarget`: the latter preserves its names, variants, methods, and
paths but deliberately replaces the public variant payload from `Arc<str>` to
`String`. No explicit `From` implementation is added or removed; construction
continues through the public variants and normal conversions into `String`.

No no-default field, variant, constructor, or conversion for either type may
mention `Arc`. Handler contexts borrow targets. Retained hot records use
`AffordanceSlotId`; they do not clone a human-readable target. The constrained
evidence cell compiles core for a target without pointer-width atomics and
checks the no-default source/API surface for accidental `alloc::sync::Arc` use.

## Handler Results

All operations except the four subscription-start operations return
`CoreResult<InteractionOutput>`. `InteractionOutput::status()` is the only
normalized successful status channel; no handler returns a second status enum.

The start operations return this linear acceptance value:

```rust
#[derive(Debug, Eq, PartialEq)]
#[must_use = "a successful acceptance must be consumed by the subscription transaction"]
pub struct SubscriptionAcceptance {
    response: InteractionOutput,
}

impl SubscriptionAcceptance {
    pub const fn new(response: InteractionOutput) -> Self;
    pub const fn response(&self) -> &InteractionOutput;
    pub fn into_response(self) -> InteractionOutput;
}
```

It deliberately does not implement `Clone`, `Copy`, or `Default`, and it is not
a destructor guard. It contains no push callback, stream, binding guard, route,
or cleanup closure.

The exact operation pairs are:

| Start | Paired teardown |
| --- | --- |
| `ObserveProperty` | `UnobserveProperty` |
| `ObserveAllProperties` | `UnobserveAllProperties` |
| `SubscribeEvent` | `UnsubscribeEvent` |
| `SubscribeAllEvents` | `UnsubscribeAllEvents` |

Before a start handler is called, the engine allocates a `SubscriptionId`,
places it in `InteractionInput`, selects and retains the paired teardown
handler generation, and verifies that one teardown invocation can fit the
profile's individual call-byte ceilings. It does not charge a live teardown
call count or `pending_call_bytes` while the subscription is merely active.
The setup transaction reserves its own pending-call count and bytes,
provisional `subscription_bytes`, one joint subscription/Producer/local-guard
slot, and one Producer residual-record token plus its maximum bytes before
application setup. Missing teardown behavior, an individually impossible
teardown footprint, or failure of those setup reservations fails before the
callback. Successful acceptance transfers `subscription_bytes` and the retained
teardown generation into exactly one application-teardown obligation tied to
the allocated id. Later replacement cannot change that obligation. Returning
`Err` from a synchronous or async start handler, or `Ready(Err(_))` from its
step form, asserts that no application or external setup obligation remains;
partial setup must be cleaned before that result is returned.

`QueryAction` and `CancelAction` require an action invocation reference. All
four start operations and all four paired teardown operations require a
subscription id. Shape failure is rejected before application code.

## Pending Work Bit ABI

`PendingWorkClass` retains its public `u16` bit representation. The v4.7
handler additions preserve every existing discriminant and append exactly the
following three bits; source order or inferred enum numbering is not an
equivalent implementation because `PendingWork::bits()` exposes this stable
representation.

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum PendingWorkClass {
    BindingInput = 1 << 0,
    ResponseDelivery = 1 << 1,
    OutboundRequest = 1 << 2,
    SubscriptionData = 1 << 3,
    Timer = 1 << 4,
    Cleanup = 1 << 5,
    EmissionFanOut = 1 << 6,
    BindingPublication = 1 << 7,
    SubscriptionCancellation = 1 << 8,
    RouteReadiness = 1 << 9,
    RouteCleanup = 1 << 10,
    HandlerCall = 1 << 11,
    ProducerSubscriptionSetup = 1 << 12,
    ProducerSubscriptionTeardown = 1 << 13,
}
```

`HandlerCall` reports a locally progressable handler completion or bounded
handler continuation. `ProducerSubscriptionSetup` and
`ProducerSubscriptionTeardown` report their matching locally progressable
Producer obligations. A synchronous callback currently executing on another
thread is not ready local work and sets none of these bits until its completion
is locally progressable.

## Exact Operation Matrix

The following table is the complete public trait expansion. There are no
generic catch-all public handler traits and no aliases retaining old names.

| Operation | Sync trait | Async trait | Step trait | Success value | Target |
| --- | --- | --- | --- | --- | --- |
| `ReadProperty` | `ReadPropertyHandler` | `AsyncReadPropertyHandler` | `StepReadPropertyHandler` | `InteractionOutput` | Property |
| `WriteProperty` | `WritePropertyHandler` | `AsyncWritePropertyHandler` | `StepWritePropertyHandler` | `InteractionOutput` | Property |
| `ObserveProperty` | `ObservePropertyHandler` | `AsyncObservePropertyHandler` | `StepObservePropertyHandler` | `SubscriptionAcceptance` | Property |
| `UnobserveProperty` | `UnobservePropertyHandler` | `AsyncUnobservePropertyHandler` | `StepUnobservePropertyHandler` | `InteractionOutput` | Property |
| `InvokeAction` | `InvokeActionHandler` | `AsyncInvokeActionHandler` | `StepInvokeActionHandler` | `InteractionOutput` | Action |
| `QueryAction` | `QueryActionHandler` | `AsyncQueryActionHandler` | `StepQueryActionHandler` | `InteractionOutput` | Action |
| `CancelAction` | `CancelActionHandler` | `AsyncCancelActionHandler` | `StepCancelActionHandler` | `InteractionOutput` | Action |
| `SubscribeEvent` | `SubscribeEventHandler` | `AsyncSubscribeEventHandler` | `StepSubscribeEventHandler` | `SubscriptionAcceptance` | Event |
| `UnsubscribeEvent` | `UnsubscribeEventHandler` | `AsyncUnsubscribeEventHandler` | `StepUnsubscribeEventHandler` | `InteractionOutput` | Event |
| `ReadAllProperties` | `ReadAllPropertiesHandler` | `AsyncReadAllPropertiesHandler` | `StepReadAllPropertiesHandler` | `InteractionOutput` | Thing |
| `WriteAllProperties` | `WriteAllPropertiesHandler` | `AsyncWriteAllPropertiesHandler` | `StepWriteAllPropertiesHandler` | `InteractionOutput` | Thing |
| `ReadMultipleProperties` | `ReadMultiplePropertiesHandler` | `AsyncReadMultiplePropertiesHandler` | `StepReadMultiplePropertiesHandler` | `InteractionOutput` | Thing |
| `WriteMultipleProperties` | `WriteMultiplePropertiesHandler` | `AsyncWriteMultiplePropertiesHandler` | `StepWriteMultiplePropertiesHandler` | `InteractionOutput` | Thing |
| `ObserveAllProperties` | `ObserveAllPropertiesHandler` | `AsyncObserveAllPropertiesHandler` | `StepObserveAllPropertiesHandler` | `SubscriptionAcceptance` | Thing |
| `UnobserveAllProperties` | `UnobserveAllPropertiesHandler` | `AsyncUnobserveAllPropertiesHandler` | `StepUnobserveAllPropertiesHandler` | `InteractionOutput` | Thing |
| `QueryAllActions` | `QueryAllActionsHandler` | `AsyncQueryAllActionsHandler` | `StepQueryAllActionsHandler` | `InteractionOutput` | Thing |
| `SubscribeAllEvents` | `SubscribeAllEventsHandler` | `AsyncSubscribeAllEventsHandler` | `StepSubscribeAllEventsHandler` | `SubscriptionAcceptance` | Thing |
| `UnsubscribeAllEvents` | `UnsubscribeAllEventsHandler` | `AsyncUnsubscribeAllEventsHandler` | `StepUnsubscribeAllEventsHandler` | `InteractionOutput` | Thing |

### Synchronous traits

Every synchronous row expands this exact object-safe template, with `R`
replaced by the table's success value:

```rust
pub trait ReadPropertyHandler {
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
```

The traits have no `Send`, `Sync`, or `'static` supertrait. Host registration
adds those bounds; portable direct/static dispatch does not. Input is never
passed as `&mut InteractionInput`; moving a payload out of an admitted request
would invalidate retry, validation, cancellation, and late-return ownership.

### Async traits

The async feature uses a generic associated future. The public trait has one
shape in every feature combination; enabling `std` never adds a `Send` bound to
an implementation that compiled in `async-no-std`:

```rust
#[cfg(feature = "async")]
pub trait AsyncReadPropertyHandler {
    type Future<'a>: core::future::Future<Output = CoreResult<InteractionOutput>> + 'a
    where
        Self: 'a;

    fn handle<'a>(
        &'a self,
        context: HandlerContext<'a>,
        input: &'a InteractionInput,
    ) -> Self::Future<'a>;
}
```

Every async row expands that template with its table result in both the
associated future output and `handle`. These GAT traits deliberately are not
object-safe. Portable and generated dispatch remain statically typed and may
return an inline, caller-owned, or otherwise non-`Send` future without a
mandatory allocation. Calling `handle` constructs the future but must not
perform application work or create an external obligation before the future is
polled. The selected handler, context backing, and input outlive the future.

For the four Producer start operations, every poll that returns `Pending` also
asserts that the future owns no fallible external setup obligation. It may own
ordinary memory and state whose nonblocking destructor is infallible. A setup
that must retain an addressable resource or run fallible cancellation before it
can finish uses the bounded-step form instead. Dropping a pending start future
therefore emits `setup_aborted` to the Producer coordinator and cannot orphan a
partially installed application resource. `Ready(Err(_))` makes the same
obligation-free assertion as a synchronous `Err`.

The `std` host boundary adds erasure after the generic setter has proved the
future is `Send`:

```rust
#[cfg(all(feature = "async", feature = "std"))]
pub(crate) type HostHandlerFuture<'a, R> = core::pin::Pin<
    alloc::boxed::Box<
        dyn core::future::Future<Output = CoreResult<R>> + Send + 'a,
    >,
>;

#[cfg(all(feature = "async", feature = "std"))]
pub(crate) struct HostAsyncAdapter<H> {
    handler: H,
}
```

`HostAsyncAdapter` is specialized internally for the selected operation
trait and converts its associated future into `HostHandlerFuture`. Neither item
is root-re-exported. Their box, erased vtable, and outer-future storage are
engine overhead; application allocations retained by the associated future are
covered by `HandlerFootprint::pending_call_bytes`. The ownership matrix names
the combined host cell `std-async`. The orthogonal `std` cell without `async`
does not expose an empty async placeholder and does not change the associated
future contract.

The cross-crate host-erasure seam is one opaque, public-but-internal core value.
It lets the Servient-owned public setters request core-owned erasure without
naming or constructing `HostAsyncAdapter`, `HostHandlerFuture`,
`HostStepAdapter`, `SelectedHandlerEntry`, or a handler slot record:

```rust
#[cfg(feature = "std")]
#[doc(hidden)]
#[must_use = "the registration must be installed or explicitly dropped"]
pub struct HostHandlerRegistration {
    // Private operation, target-kind, flavor, handler, and footprint owner.
}

#[cfg(feature = "std")]
impl HostHandlerRegistration {
    pub fn read_property<H>(handler: H, footprint: HandlerFootprint) -> Self
    where
        H: ReadPropertyHandler + Send + Sync + 'static;

    #[cfg(feature = "async")]
    pub fn async_read_property<H>(handler: H, footprint: HandlerFootprint) -> Self
    where
        H: AsyncReadPropertyHandler + Send + Sync + 'static,
        for<'a> <H as AsyncReadPropertyHandler>::Future<'a>: Send;

    pub fn step_read_property<H>(handler: H, footprint: HandlerFootprint) -> Self
    where
        H: StepReadPropertyHandler + Send + Sync + 'static,
        H::State: Send + 'static;
}

#[cfg(feature = "std")]
#[doc(hidden)]
pub trait HostHandlerRegistrationIngress: private::Sealed {
    fn install_host_handler(
        &self,
        name: Option<&str>,
        registration: HostHandlerRegistration,
    ) -> CoreResult<HandlerSlotId>;

    fn clear_host_handler(
        &self,
        operation: clinkz_wot_td::data_type::Operation,
        name: Option<&str>,
    ) -> CoreResult<bool>;

    fn clear_all_host_handlers(&self) -> CoreResult<u32>;
}
```

The three factory names for every operation are exactly `{operation_snake}`,
`async_{operation_snake}`, and `step_{operation_snake}`, using the eighteen
snake-case stems frozen below. Every factory expands the corresponding bounds
shown for `read_property`, stores one immutable operation and target kind, and
does not invoke the handler, create an associated future, or create a step
state. Sync and step factories are in the `std` cell; async factories require
the combined `std-async` cell. The opaque registration implements informative,
handler-redacted `Debug`, but not `Clone`, `Copy`, or `Default`.

`HostHandlerRegistrationIngress` is sealed and object-safe. It is implemented
by the core-owned `clinkz_wot_core::ExposedThing` handler store and is not
implemented by the Servient. `install_host_handler` takes the opaque value by
ownership. `name` must be `Some` for Property, Action, and Event registrations
and `None` for Thing/collection registrations; any mismatch is a validation
failure before publication. The registration fixes operation, target kind, and
flavor, so the ingress never accepts those as caller-controlled parallel
arguments. `clear_host_handler` applies the same name-shape rule using the
operation, and `clear_all_host_handlers` removes every published flavor.

The public-but-internal paths are
`clinkz_wot_core::handler::HostHandlerRegistration` and
`clinkz_wot_core::handler::HostHandlerRegistrationIngress`. They are
`#[doc(hidden)]`, are not root-re-exported, and exist solely because Rust crate
privacy otherwise prevents `clinkz-wot-servient` from installing core-owned
adapters. Their public signatures contain no crate-private type. Application
code uses only the Servient setter surface. A failed ingress admission leaves
the old generation unchanged and drops the uninstalled registration outside
all engine locks and constrained critical sections.

### Bounded-step traits

The portable bounded form chooses static dispatch and caller-owned typed state.
It does not claim that arbitrary state can simultaneously be dynamically
erased, allocation-free, and object-safe.

```rust
#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum HandlerStep<R> {
    Pending,
    Ready(CoreResult<R>),
}

pub trait StepReadPropertyHandler {
    type State;

    fn start(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
        budget: &mut WorkBudget,
    ) -> CoreResult<Self::State>;

    fn step(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
        state: &mut Self::State,
        budget: &mut WorkBudget,
    ) -> HandlerStep<InteractionOutput>;

    fn cancel(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
        state: &mut Self::State,
        budget: &mut WorkBudget,
    ) -> HandlerStep<()>;
}
```

Every step row expands this template with its table result. `start(Err)` is a
promise that no application or external obligation remains. If cancellation
wins while `start` runs and `start` returns a state, that state enters explicit
cancellation. `cancel` may return `Pending` and is repeatedly budget-driven.
For a Producer start, `step` returning `Ready(Err(_))` promises that its state
retains no application or external setup obligation. `cancel` returning
`Ready(Ok(()))` emits `setup_aborted`; `cancel` returning `Ready(Err(_))`
discharges local ownership and records terminal cleanup failure. A retryable or
still-addressable application cleanup remains `Pending`, rather than returning
an error while retaining an unrepresented obligation. Every `start`, `step`,
and `cancel` callback is charged one `WorkClass::HandlerSteps` unit before entry
and runs outside engine locks and constrained critical sections.

## Registration and Sparse Storage

Every registration declares three application-owned worst-case byte maxima:

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HandlerFootprint {
    retained_bytes: u64,
    pending_call_bytes: u64,
    subscription_bytes: u64,
}

impl HandlerFootprint {
    pub const fn new(
        retained_bytes: u64,
        pending_call_bytes: u64,
        subscription_bytes: u64,
    ) -> Self;

    pub const fn retained_bytes(self) -> u64;
    pub const fn pending_call_bytes(self) -> u64;
    pub const fn subscription_bytes(self) -> u64;
}
```

`retained_bytes` bounds application storage retained for the published handler
generation. `pending_call_bytes` bounds additional application storage retained
from callback/future/step start until that invocation terminates.
`subscription_bytes` bounds application storage retained per successfully
accepted Producer observation or event subscription until its application
teardown terminates. Only the four start-operation registrations may declare a
nonzero `subscription_bytes`; every other setter or static-table admission
rejects a nonzero value as invalid registration shape.

Each value is an application assertion of a hard worst-case upper bound,
including nested allocations that the engine cannot inspect. Under-reporting is
a handler contract violation and invalidates conformance evidence; it is not an
implicit request for unbounded memory and the engine does not claim that a
global allocator can discover it reliably. Instrumented test adapters must fail
the handler-footprint evidence when they observe an overrun. Zero declares no
extra application bytes; it never means zero engine overhead. The engine
separately charges slot records, erased wrappers, call owners, host future
boxes, typed step state, context/input retention, and generation metadata.

Host registration is owned by `clinkz_wot_servient::ExposedThingHandle`. Every
matrix row has the following three setter families and one clear family. This
property example freezes spelling, argument order, result, and bounds:

```rust
pub fn set_read_property_handler<H>(
    &self,
    name: &str,
    handler: H,
    footprint: HandlerFootprint,
) -> CoreResult<HandlerSlotId>
where
    H: ReadPropertyHandler + Send + Sync + 'static;

#[cfg(feature = "async")]
pub fn set_async_read_property_handler<H>(
    &self,
    name: &str,
    handler: H,
    footprint: HandlerFootprint,
) -> CoreResult<HandlerSlotId>
where
    H: AsyncReadPropertyHandler + Send + Sync + 'static,
    for<'a> <H as AsyncReadPropertyHandler>::Future<'a>: Send;

pub fn set_step_read_property_handler<H>(
    &self,
    name: &str,
    handler: H,
    footprint: HandlerFootprint,
) -> CoreResult<HandlerSlotId>
where
    H: StepReadPropertyHandler + Send + Sync + 'static,
    H::State: Send + 'static;

pub fn clear_read_property_handler(&self, name: &str) -> CoreResult<bool>;
pub fn clear_handlers(&self) -> CoreResult<u32>;
```

Each setter constructs the matching
`clinkz_wot_core::handler::HostHandlerRegistration` factory value and passes it
by ownership to
`clinkz_wot_core::handler::HostHandlerRegistrationIngress::install_host_handler`.
Each operation-specific clear delegates to `clear_host_handler`, and
`clear_handlers` delegates to `clear_all_host_handlers`. Servient neither
duplicates erasure nor accesses a core slot, adapter, reducer, or registry
field. This is the only cross-crate host registration ingress; adding parallel
public core setters or exposing raw slot mutation is nonconforming.

The method names are `set_{operation}_handler`,
`set_async_{operation}_handler`, `set_step_{operation}_handler`, and
`clear_{operation}_handler`, where `{operation}` is the `Operation::as_str()`
word split into snake case. Thing/collection methods omit `name`; individual
Property, Action, and Event methods require it. `clear_handlers` removes every
operation flavor for the exposed Thing. The count is the number of published
operation slots removed, not the number of retired generations.

One operation-specific clear removes whichever sync, async, or step flavor is
currently published. A setter is replacement; the last successfully published
setter wins. Registration and replacement reserve the new slot or generation,
`retained_bytes`, and all known engine registration/wrapper bytes before
publication. They do not reserve a pending-call count, `pending_call_bytes`, or
`subscription_bytes`. Failure leaves the old generation and every counter
unchanged. A selected dispatch retains its old generation. No more than
`handler_generations_per_slot_max` generations may be retained at once.

After selecting a handler and before entering any application callback, an
ordinary dispatch reserves one pending-handler-call count, its declared
`pending_call_bytes`, and the known engine call-owner/future/step-state bytes.
It releases that reservation only when the handler execution owner reaches its
terminal acknowledgment, including a late synchronous return, host future
drop, or completed step cancellation. Failure to reserve rejects the dispatch
without invoking application code.

A Producer start transaction additionally reserves, before setup begins, the
selected start handler's `subscription_bytes`, one subscription item, handler-
state bytes for its Producer owner and embedded local guard slot, and one token
plus its byte maximum in the independent Producer residual ledger. The one
subscription item jointly bounds `SubscriptionId`, `ProducerSubscriptionOwner`,
and the local guard slot; there is no unbounded or separately counted guard
table. The transaction retains the paired teardown generation and its declared
footprint as immutable metadata, but it does not charge a teardown pending-call
count or bytes while no teardown callback exists.

Before acceptance, `subscription_bytes` is a provisional reservation. A setup
error or abort that creates no acceptance releases it when setup execution is
acknowledged, independently of start-response delivery and tombstone retention.
Acceptance converts it to an active application-storage charge. That charge and
the retained teardown generation release only after `ApplicationTeardown` is
`Complete` or `Residual` and any transferred cleanup owner acknowledges its
handoff. The subscription item and remaining Producer-record bytes stay charged
through the bounded terminal tombstone until deterministic eviction.

When rollback or stopping first needs application teardown, that owner acquires
one normal pending-call count and the teardown handler's
`pending_call_bytes`. Capacity failure leaves the obligation owned and reports
`ProducerSubscriptionTeardown` as pending work; it does not invoke application
code or consume a cleanup-queue item. The first failure starts one cumulative
admission wait: host progress is bounded by
`handler_drain_timeout_millis_max`, while manual-poll progress charges one
`WorkClass::HandlerSteps` unit per failed attempt and is bounded by
`handler_drain_steps_max`. Zero selects immediate fallback. At the first
applicable bound, no further normal admission occurs: cleanup capacity is
charged and ownership transfers, or the transaction commits its already
reserved Producer residual record. Thus full handler-state or pending-call
ledgers cannot wait for storage that only this same subscription can release.
This timing keeps the profile maximum of 256 active constrained subscriptions
compatible with 256 pending calls and 64 ordinary cleanup items.

Portable static/generated registration uses this exact borrowed value:

```rust
pub struct StaticHandlerRegistration<'h, H> {
    slot_id: HandlerSlotId,
    handler: &'h H,
    footprint: HandlerFootprint,
}

impl<'h, H> StaticHandlerRegistration<'h, H> {
    pub const fn new(
        slot_id: HandlerSlotId,
        handler: &'h H,
        footprint: HandlerFootprint,
    ) -> Self;

    pub const fn slot_id(&self) -> HandlerSlotId;
    pub const fn handler(&self) -> &'h H;
    pub const fn footprint(&self) -> HandlerFootprint;
}

impl<H> Copy for StaticHandlerRegistration<'_, H> {}

impl<H> Clone for StaticHandlerRegistration<'_, H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H> core::fmt::Debug for StaticHandlerRegistration<'_, H> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("StaticHandlerRegistration")
            .field("slot_id", &self.slot_id)
            .field("footprint", &self.footprint)
            .finish_non_exhaustive()
    }
}
```

No trait bound is placed on the generic struct. The generated table field or
dispatch method applies exactly one matching sync, async, or step trait bound.
An async registration retains the borrowed handler but does not retain a
future; its associated future is created only after dispatch reserves
`pending_call_bytes`, and the application chooses its caller-owned storage.
A step registration likewise creates its typed state only after the per-call
reservation succeeds.

A generated table owns fixed capacity and typed async/step call slots, admits
the whole private replacement generation against the same limits, and
publishes or rejects it atomically. It may expose application-local setters
with the host names, but those generated methods are not a second engine public
API. The descriptor itself uses no `Arc`, `Box`, `Send`, `Sync`, atomics, or
erased state. `async-no-std` does not introduce the std-only
`HostAsyncAdapter`; allocation, pinning, and executor choice remain with
the application and are charged to the declared footprint and engine call-slot
storage as applicable.

## Public Failure Matrix

The following failure mapping is exact. Every context attaches all known
fixed-size Thing, affordance-slot, operation, plan, binding, and correlation
identities named by the row; it never copies an affordance name into the error.
Limit errors use the named `ResourceKind`, set `limit` to the configured value,
set `requested` to the checked projected total when representable, and set
`observed` to the current charged total. Checked-add overflow sets `requested`
to `None` and is still a `LimitExceeded` result.

| Boundary | Condition | Error category | Phase | Retry class | Observable result |
| --- | --- | --- | --- | --- | --- |
| Accept-hint construction | An applicable response-candidate or handler-state limit is `None` | `CoreError::UnsupportedOperation` | `ErrorPhase::Admission` | `RetryClass::Never` | No `AcceptHint` is constructed and no live ledger capacity is reserved |
| Accept-hint construction | Preferred plus alternative count exceeds `form_binding_candidates_per_operation_max`, the checked retained-byte total exceeds either handler-state byte ceiling, or checked arithmetic overflows | `CoreError::LimitExceeded` for the exact ceiling | `ErrorPhase::Admission` | `RetryClass::Never` | No alternative is cloned and no live ledger capacity is reserved; overflow uses `requested = None` |
| Context construction | `HandlerContext::try_new` receives an operation whose kind is incompatible with its `AffordanceTarget` | `CoreError::Validation` | `ErrorPhase::Validate` | `RetryClass::Never` | No context is constructed; attach the Thing slot, operation, plan, and optional binding identities |
| Registration or clear | The `ExposedThingHandle` is stale, or a static table names a stale `HandlerSlotId` generation | `CoreError::StaleHandle` | `ErrorPhase::Admission` | `RetryClass::Never` | No slot or counter changes |
| Registration or clear | The Thing is draining, destroyed, or otherwise closed to handler mutation | `CoreError::Lifecycle` | `ErrorPhase::Admission` | `RetryClass::Never` | No slot or counter changes |
| Registration or clear | A named Property, Action, or Event does not exist | `CoreError::NotFound` | `ErrorPhase::Admission` | `RetryClass::Never` | No slot or counter changes; attach the Thing slot and operation identities |
| Registration or clear | The target exists, but the requested operation was not admitted for it | `CoreError::UnsupportedOperation` | `ErrorPhase::Admission` | `RetryClass::Never` | No slot or counter changes; attach the Thing and affordance slots plus operation |
| Dispatch | The operation was admitted, but no handler flavor is currently published | `CoreError::UnsupportedOperation` | `ErrorPhase::Handler` | `RetryClass::Never` | Application code is not entered; attach every known admitted-request identity |
| Registration or static admission | A non-start operation declares nonzero `subscription_bytes` | `CoreError::Validation` | `ErrorPhase::Admission` | `RetryClass::Never` | The registration is rejected without changing a slot or counter |
| Static admission | A generated table repeats a `HandlerSlotId` or its registration shape contradicts the fixed operation or flavor | `CoreError::Validation` | `ErrorPhase::Admission` | `RetryClass::Never` | The whole table generation is rejected atomically |
| Registration or dispatch | The selected profile has `None` for a required handler limit | `CoreError::UnsupportedOperation` | `ErrorPhase::Admission` | `RetryClass::Never` | No unbounded fallback is selected and application code is not entered |
| Registration or replacement | The projected total exceeds `handler_slots_per_thing_max`, `handler_slots_global_max`, `handler_state_bytes_per_thing_max`, or `handler_state_bytes_global_max` | `CoreError::LimitExceeded` for the exact ceiling | `ErrorPhase::Admission` | `RetryClass::Never` | The old published generation and every counter remain unchanged |
| Replacement | Publishing the replacement would retain a third live generation for one handler slot | `CoreError::Backpressure` | `ErrorPhase::Admission` | `RetryClass::Safe` | The old published generation remains selected and every counter is unchanged |
| Clear | The target and operation are valid, but no handler flavor is currently published | None | None | None | `clear_{operation}_handler` returns `Ok(false)`; `clear_handlers` returns `Ok(0)` when every slot is absent |
| Dispatch | One declared call or Producer reservation by itself exceeds a per-Thing or global handler-state byte ceiling | `CoreError::LimitExceeded` for the exact ceiling | `ErrorPhase::Handler` | `RetryClass::Never` | Application code is not entered |
| Dispatch | Other calls or teardown guarantees occupy `pending_handler_calls_per_thing_max` or `pending_handler_calls_global_max` | `CoreError::Backpressure` | `ErrorPhase::Handler` | `RetryClass::Safe` | Application code is not entered |
| Dispatch | The call fits individually, but current charged handler-state bytes leave insufficient transient capacity | `CoreError::Backpressure` | `ErrorPhase::Handler` | `RetryClass::Safe` | Application code is not entered |
| Producer start | The paired teardown handler is not published | `CoreError::UnsupportedOperation` | `ErrorPhase::Handler` | `RetryClass::Never` | Setup is not entered; attach the start operation and admitted-request identities |
| Producer start | Live work prevents reservation of the start call, `subscription_bytes`, the joint subscription/owner/guard slot, or the independent Producer residual token and bytes | `CoreError::Backpressure` | `ErrorPhase::Handler` | `RetryClass::Safe` | Setup is not entered and every partial reservation is rolled back |

`clear_{operation}_handler` returns `Ok(false)` only when its target and
operation are valid but no flavor is currently published; `Ok(true)` means one
published slot was removed. `clear_handlers` returns the number of published
slots removed and returns `Ok(0)` when none were present. Neither method counts
or waits for retired generations. Application `CoreError` values returned by a
handler are not remapped by this table.

Sparse storage is keyed by admitted operation and affordance slot. Unsupported
operations consume no handler object, async wrapper, typed call state, or
retired generation. An implementation may choose a sparse table, profile-proven
fixed table, or generated dispatch, but its externally visible replacement,
generation, accounting, and cancellation behavior is identical.

The final public surface does not expose `ReadSlot`, `WriteSlot`,
`ObserveSlot`, `UnobserveSlot`, `InvokeSlot`, `QuerySlot`, `CancelSlot`,
`SubscribeSlot`, `UnsubscribeSlot`, or raw `property_handler_*`,
`action_handler_*`, and `event_handler_*` lookups. Core dispatches through a
selected-call boundary; downstream crates do not inspect slot variants.

## Handler Execution Ownership and Cancellation

`InFlightState` owns only the response opportunity. It never owns handler
execution. A distinct bounded `HandlerCallOwner` retains, until callback
termination:

- the `SelectedHandlerEntry` snapshot;
- context backing and `InteractionInput`;
- handler, Thing, and plan generations plus correlation diagnostics;
- the async future or bounded-step state when applicable; and
- the pending-call byte/count reservation.

The crate-private ownership records frozen by the ownership matrix are
`SelectedHandlerEntry`, `HandlerCallOwner`, `HandlerResultSink`,
`CallbackLease`, `ProducerSubscriptionOwner`, `HandlerCleanupOwner`, and the
`std-async`-only `HostHandlerFuture` and `HostAsyncAdapter`, plus the std-only
`HostStepAdapter`. None is a root re-export. Admission fixes one
`HandlerResultSink::{DirectResponse, ProducerAcceptance, ProducerTeardown}` in
each call owner; a call cannot change sinks after selection.

The separate `HostHandlerRegistration` and
`HostHandlerRegistrationIngress` are public only at their documented hidden
module paths so the downstream Servient can cross the Rust crate boundary.
They expose no state-record field and do not make any crate-private owner
nameable.

Closing and releasing the response opportunity may occur while a synchronous
handler is still running. `Cancelled -> Released` in the in-flight machine does
not release or reuse the handler-call slot. Only the terminal acknowledgment of
the corresponding handler-execution machine permits call-owner and generation
reuse.

Ready and late results use this exact disposition:

| Sink | On-time `CoreResult` | Late `CoreResult` | In-flight effect |
| --- | --- | --- | --- |
| `DirectResponse` | Atomically pair `claim_direct_response` with generation-checked `complete_direct`, then move the result once to validation and delivery | `discard_direct_result` outside locks after retaining bounded identity and cause | The paired claim changes `Admitted -> Completing`; a closed or reused generation cannot be claimed |
| `ProducerAcceptance` | `transfer_start_result` moves `Ok(SubscriptionAcceptance)` or `Err` to the matching Producer `Accepting` generation | `transfer_late_start_result` moves the result to `AcceptingCancelled`; a late acceptance creates teardown ownership before its response payload is discarded | No handler transition claims the start response; success claims only in `installed_and_published`, while an on-time error uses the Producer error-response composition |
| `ProducerTeardown` | `transfer_teardown_result` moves the result to the matching cleanup generation | `transfer_late_teardown_result` moves it to the same cleanup owner for terminal or residual classification | Each wire teardown view owns its own in-flight response; the handler call never uses the start response token |

An async `ProducerAcceptance` future dropped after cancellation reports
`setup_aborted`. A step setup whose explicit cancellation completes reports
`setup_aborted`; cleanup transfer, terminal cleanup error, and residual paths
report `setup_cleanup_pending`, `setup_cleanup_failed`, or `setup_residual`
respectively. The Producer remains `AcceptingCancelled` until one of those
events or a transferred late result consumes the setup generation. No such
path silently discards a linear acceptance.

Pre-acceptance bounded-step cleanup is represented by the third linear
obligation bit, `SetupCancellation`. It is `Absent` for sync and async starts.
When cancellation first owns typed step setup state it becomes `Pending`; each
budgeted cancel callback changes it to `Claimed(callback_nonce)`, a `Pending`
return restores `Pending`, clean completion or a no-state late result commits
`Complete`, charged cleanup transfer commits `PendingCleanup`, and durable
fallback commits `Residual`. If cancellation wins while bounded `start` is
running, the matching late `state_ready` and `setup_cancel_created` composition
creates the bit before explicit cancellation. A transferred cleanup owner
retries that exact typed cancel state through the same nonce and
`WorkClass::HandlerSteps` rule.
It does not invoke the paired application teardown and does not acquire another
pending-handler-call count. This bit makes setup cleanup and setup residual
representable before an acceptance has created `ApplicationTeardown`.

Cancellation, deadline, or drop of a teardown request's response view closes
only that generation-checked `InFlightState`; it never cancels the independent
`ProducerTeardown` call. Servient drain, Thing destruction, or an explicit
`HandlerCleanupOwner` abort policy may cancel teardown execution. If an async
teardown then terminates without a `CoreResult`, the transaction conservatively
commits `ApplicationTeardown` as `Residual` in its reserved Producer record. A
step teardown cancel maps `Ready(Ok(()))` to `Complete`, `Ready(Err(_))` to a
terminal cleanup error under the no-remaining-obligation contract, cleanup
transfer to `PendingCleanup`, and drain residual to `Residual`. Every mapping
and call-reservation release is one generation- and nonce-checked composition.

An execution-owner abort must first resolve an initiating teardown response
view independently of the call: an `Admitted` view atomically changes through
the cancellation, deadline, drain, or drop transition matching the immutable
abort cause; a view already `Completing`, `Cancelled`, or `Released` is joined
without replacement. Only then may teardown execution be cancelled. Thus a
no-`CoreResult` path never leaves an `Admitted` response opportunity. A claimed
cancellation response or closed view reaches release and acknowledges its
checked Producer view normally, independently of whether the obligation becomes
`Complete`, `PendingCleanup`, or `Residual`.

An on-time paired teardown result may return while the Producer is
`RollingBack`, `Stopping`, or `CleanupPending`. `RollingBack` and local cleanup
have no initiating response view. In `Stopping` or `CleanupPending`, an
`Admitted` initiating wire view receives the validated output through
`complete_direct`; if that view is already `Completing`, `Cancelled`, or
`Released`, cleanup still settles the obligation once and discards the
unavailable output without retry. A cleanup-queue retry therefore has the same
complete response disposition as the first teardown admission attempt.

For direct calls, manual cancellation or deadline expiry claims a structured
`Cancelled` or `TimedOut` response when the response channel can still deliver.
When delivery is already unavailable, or drain/drop wins, the in-flight record
closes through `Cancelled` without delivery. The first serialized cause among
manual cancellation, deadline, drain, and drop is retained and later causes
join it. The handler cause update, its callback nonce, and the matching
in-flight or Producer transition are committed under the documented lock order;
the running `CallbackLease` remains uniquely owned outside the lock and is
never concurrently acquired by the cancellation actor.

Release of a Producer start or teardown `InFlightState` atomically acknowledges
the matching response generation in `ProducerSubscriptionOwner`. A start
acknowledgement marks its single response terminal; a teardown acknowledgement
decrements the checked view count exactly once. Local stop views use the same
generation rule through `local_view_released`. Duplicate and stale
acknowledgements change no count. Response-delivery failure records its bounded
cause in that same boundary before lifecycle cleanup continues.

`Active::stop` and each nonterminal `join_stop` attach one checked local view and
increment the local-view count once. Repeated drop or destroy uses
`join_runtime_stop`, which joins cleanup ownership without a view or count
change. A `Closed` or `Failed` `join_stop` validates the generation and returns
the retained summary in one boundary with zero net count. Every earlier retained
local view decrements once through `local_view_released`; a duplicate or stale
release is a no-op.

The exact sync, async, and step reducers are the machines named
`handler-sync-execution`, `handler-async-execution`, and
`handler-step-execution` in `docs/state-machines.toml`. Their transition set is
normative. In addition:

- Sync sees the snapshot taken immediately before entry. A running call is
  non-preemptible. Cancellation changes it to a late state; the callback,
  handler, input, and reservation remain pinned until actual return.
- Async sees the entry snapshot. Cancellation drops the owned future at the
  first engine boundary after cancellation, outside every engine guard. A poll
  already in progress may return once, but no later poll is scheduled. The
  future is never detached. Stale wakes are rejected by call generation and
  state.
- Step receives a refreshed input/cancellation snapshot on every `start`,
  `step`, and `cancel` call. Cancellation is explicitly budget-driven until a
  terminal result. State drop runs outside critical sections.
- The first cancellation cause wins internally and distinguishes manual
  cancellation, deadline, drain, and drop even though the application snapshot
  is only Active or Requested.
- Cancellation cannot preempt a currently executing sync callback,
  `Future::poll`, step callback, or destructor. Future and application
  destructors must be nonblocking and cannot be the sole owner of fallible
  external cleanup.

A future that returns `Ready` and a step state that reaches a ready or cancelled
terminal remain inside the current `CallbackLease` until their destructor has
run outside every engine guard. Only the matching nonce commit may then release
the call generation. A stale wake or callback return cannot destruct or commit
state owned by a newer generation.

`handler_drain_steps_max` is cumulative per cancelled step call. Reaching the
limit, or selecting the zero-step immediate-close policy, stops normal cancel
callbacks and attempts one atomic ownership transfer of the handler, typed
state, context/input, generation, and existing byte reservation to
`HandlerCleanupOwner`. Successful transfer records `PendingCleanup`; it does
not double-charge the retained state, while the compact `CleanupRecord` and
queue slot use the cleanup ledger. If no cleanup slot is available, the engine
records `ResidualExternalState` durably before dropping local typed state. For a
Producer transaction this uses the residual token reserved before setup and is
independent of the ordinary cleanup ledger. This Producer-specific reservation
does not alter existing non-Producer cleanup admission. The bounded drain never
loops indefinitely or pretends that an unowned external obligation was cleaned.

Handler callbacks are required not to panic. In a `std` host adapter, panic at
future creation, poll, sync invocation, step, cancel, or destructor is caught at
the `CallbackLease` boundary and converted to bounded
`CoreError::Application` diagnostics. A Producer setup panic before a valid
acceptance is conservatively classified as cleanup failure or
`ResidualExternalState` unless the applicable step state can be transferred and
cancelled explicitly. In `no_std` configurations whose panic strategy aborts,
the process aborts; the engine makes no recovery or response-delivery claim.
Panic handling never runs while an engine lock is held.

Late terminal records contain bounded cause and identity only. They never
retain a payload, future, handler object, TD, credential, binding guard, or
unbounded error chain.

## Producer Subscription Transaction

The existing `subscription` state machine remains Consumer-owned. Producer
observe/subscribe setup is the separate `producer-subscription` machine in
`docs/state-machines.toml`.

`ProducerSubscriptionOwner` reserves the subscription id, provisional
`subscription_bytes`, the joint subscription/owner/local-guard slot, setup-call
capacity, and one independent Producer residual token plus its record bytes
before setup. It references the separate start-response token and retains the
selected teardown generation plus declared footprint without charging a live
teardown call. A successful acceptance creates exactly one
`ApplicationTeardown` obligation and converts `subscription_bytes` to an active
charge. Complete binding/local guard installation creates the separate
`GuardClose` obligation. Replacement never retargets either obligation.

An on-time transferred setup `Err` atomically claims the start response for that
error when delivery remains available, or records it unavailable, before the
Producer enters `Failed`. An `Ok(SubscriptionAcceptance)` does not claim a
response and remains private in `Accepted` until installation. Thus no handler
result can deliver a successful start response before the guard is installed.
If accepted-response validation or guard installation fails before publication,
the same-generation Producer transition atomically claims the start error
response before entering rollback. If that response view is already terminal,
the transition records it unavailable before rollback instead; neither branch
can leave an admitted response view or an unowned cleanup obligation.

The coordinated `installed_and_published` transition:

1. rechecks subscription generation, cancellation, and deadline;
2. retains the completely owned installed guard;
3. claims the separate success response opportunity into `Completing`;
4. publishes the active subscription generation; and
5. transfers the accepted response to response delivery.

The claim and publication are one non-callback state boundary in the documented
engine lock order. If response close wins before the claim, the transaction
enters rollback and is never Active. Start-response resolution is retained as
an independent field rather than inferred only from the lifecycle state. A
delivery failure may therefore join `AcceptingCancelled`,
`InstallingCancelled`, `RollingBack`, `Active`, `Stopping`, `CleanupPending`,
`Closed`, or `Failed`; it records the failure once, starts stopping when still
Active, and cannot strand or resurrect a subscription.
Guard installation returns either a completely owned guard or a failure plus an
addressable cleanup obligation; it may not fail while leaving an unowned
external guard.

`RollingBack`, `Stopping`, and `CleanupPending` retain the three-bit obligation
reducer: `SetupCancellation`, `GuardClose`, and `ApplicationTeardown`. Each bit
is one of `Absent`, `Pending`, `Claimed(callback_nonce)`,
`Complete`, `PendingCleanup(cleanup_handle)`, or
`Residual(producer_residual_record)`. The machine order is
`SetupCancellation`, `GuardClose`, `ApplicationTeardown`. On accepted paths the
first bit is already `Absent` or `Complete`, so the effective order is guard
close followed by application teardown and new samples stop before application
cleanup. On a pre-acceptance path the latter two bits are `Absent`, so only the
typed setup state progresses. Every later applicable bit
is attempted even when an earlier one reports an error, pending cleanup, or
residual state. A claim changes exactly one bit to
`Claimed`, moves its object into `CallbackLease`, and commits only a matching
nonce.

Application teardown acquires its normal pending-call count and bytes only when
it becomes runnable. A teardown `Err` is terminal only under the handler
contract that no retryable or addressable application obligation remains;
otherwise the owner retains `PendingCleanup` or durably records
`ResidualExternalState`. Guard close follows the same complete, pending, and
residual classification. Neither destructor is the sole fallible cleanup path.

The transaction's single fixed Producer residual record covers all three
obligation bits and contains only Thing/subscription slot generations, a
three-bit residual mask, first cause, bounded error disposition, optional
binding generation, and checked cleanup attempts. Its token and maximum bytes
are reserved before
setup from `producer_residual_records_global_max` and
`producer_residual_bytes_global_max`; each committed record is also bounded by
`producer_residual_record_bytes_max`. It does not consume
`cleanup_items_max`, `cleanup_bytes_max`, or `cleanup_retry_records_max`.
Normal completion releases the unused token. A committed record survives
Producer tombstone eviction until explicit reconciliation acknowledges it.

Repeated local stop, runtime drop/destroy, and wire teardown join the same
generation and never reinvoke application teardown. Local stop observers and
runtime ownership joins follow the separate count rules frozen above. Each wire
request owns a separately admitted in-flight view; the Producer record stores
only a checked view count and a payload-free terminal summary, never an
unbounded waiter list. The first
teardown observer may receive the validated handler output. Later terminal
replay returns the same bounded `CoreError` on failure, or an empty
`InteractionOutput` carrying the retained `InteractionStatus` on success; it
does not retain or replay the original payload or binding metadata. Late setup
acceptance, late guard installation, response-view release, drop, deadline,
cancellation, and teardown-without-result follow the exact compositions in the
machine.

A Producer terminal becomes quiescent only when all of these conditions hold:
the start response is terminal; every teardown response view is terminal;
`SetupCancellation`, `GuardClose`, and `ApplicationTeardown` are `Absent`,
`Complete`, or `Residual`; the cleanup owner is
absent or acknowledged; and the checked local-view count is zero. These are an
all-of conjunction, not alternative acknowledgments. Quiescence does not
immediately release the record. `Closed` or `Failed` remains a bounded tombstone
in its existing subscription slot and replays its payload-free summary.

`release_when_quiescent` occurs only on Thing destruction or admission pressure
from a subscription-item or handler-state-byte ceiling. The deterministic
victim is the smallest `(ThingSlotId, SubscriptionId, subscription generation)`
tuple among quiescent tombstones eligible for that scope. Replay before eviction
returns the retained terminal summary; replay after eviction returns
`StaleHandle`. The active `subscription_bytes` charge and selected teardown
generation release earlier, when `ApplicationTeardown` becomes `Complete` or
`Residual` and cleanup handoff is acknowledged. Tombstone bytes and its joint
subscription slot release only at eviction. Retained terminal data excludes
payloads, handler objects, futures, TDs, credentials, and guards.

WP-100 owns the acceptance value, paired-handler reservation rule, reducers,
and fake-owner tests. WP-300 supplies guard installation, route-generation, and
response-opportunity primitives. WP-400 owns the integrated coordinator,
Servient lifecycle wiring, and retained terminal replay. No interim
`BindingRouteKey` or placeholder plan schema is introduced in WP-100.

## Resource and Work-Budget Closure

The nine handler limits and three independent Producer-residual limits first
added by the v4.7 handler closure remain the exhaustive handler-owned subset of
the active `docs/resource-limits.csv` schema:

| Field | Gateway | Directory | Static reference |
| --- | ---: | ---: | ---: |
| `handler_slots_per_thing_max` | 4,105 | NA | 256 |
| `handler_slots_global_max` | 262,144 | NA | 256 |
| `handler_state_bytes_per_thing_max` | 4,194,304 | NA | 65,536 |
| `handler_state_bytes_global_max` | 268,435,456 | NA | 262,144 |
| `pending_handler_calls_per_thing_max` | 1,024 | NA | 32 |
| `pending_handler_calls_global_max` | 65,536 | NA | 256 |
| `handler_generations_per_slot_max` | 2 | NA | 2 |
| `handler_drain_timeout_millis_max` | 5,000 | NA | NA |
| `handler_drain_steps_max` | 1,024 | NA | 64 |
| `producer_residual_records_global_max` | 65,536 | NA | 256 |
| `producer_residual_record_bytes_max` | 256 | NA | 128 |
| `producer_residual_bytes_global_max` | 16,777,216 | NA | 32,768 |

`ResourceKind` has a public zero-based representation/index generated in CSV
data-row order. The 118 pre-v4.7 fields therefore retain indices `0..=117`, and
the twelve fields in the table above retain their exact order at `118..=129`.
The additional v4.8 fields occupy `130..=138`, and the v4.9 planning/binding
projection occupies `139..=194`; those fields are owned by the active resource
schema rather than this handler amendment. A generator must not sort by field
or resource kind. Insertion, removal, or reordering of any existing row is an
ABI/source-identity break and requires reviewed impact handling.

All use `zero_semantics=disabled`. For drain fields, zero selects only an
explicit immediate-close policy; it never permits unlimited waiting.

The handler-state byte ledger includes sparse engine tables,
`SelectedHandlerEntry`, active and retired registration generations,
`HandlerCallOwner`, `CallbackLease` metadata, host erased adapters and boxes,
pending futures and step states, context/input overhead, cancellation snapshots,
`ProducerSubscriptionOwner`, payload-free terminal summaries,
`HandlerCleanupOwner` metadata, and declared registration, pending-call, and
per-subscription application bytes. A transferred typed call/state keeps its
existing handler-state charge; the compact `CleanupRecord` and queue slot are
charged to the cleanup ledger and are not double-counted. Payload backing
remains charged to its existing payload/in-flight ledger while pinned and is
not double-counted as handler state.

One `subscriptions_per_thing_max`/`subscriptions_global_max` item jointly owns
the subscription id, Producer record, and embedded local guard slot; those
objects do not create an unfrozen item ledger. Their bytes, terminal tombstone,
and provisional or active `subscription_bytes` are charged to handler state.
The three Producer-residual limits form a separate ledger: every admitted start
reserves one record token and `producer_residual_record_bytes_max` bytes before
application setup. The global values cover every simultaneously admitted
Producer subscription in each profile. Ordinary completion returns the token;
residual commit retains it independently of cleanup-queue saturation and
Producer tombstone eviction.

`WorkClass` appends `HandlerSteps`. One unit is charged for each bounded handler
`start`, `step`, or `cancel`, and for each constrained async-adapter poll.
`PendingWorkClass` appends `HandlerCall`, `ProducerSubscriptionSetup`, and
`ProducerSubscriptionTeardown`. A host synchronous callback currently executing
on another thread is not ready local pending work; its completion becomes
`HandlerCall` only when locally progressable.

Under ADR-0015, `ResourceLimits` is explicitly cloneable but not `Copy`,
`StaticResourceProfile` exposes `&'static ResourceLimits`, and `WorkBudget`
implements neither `Clone` nor `Copy`. Handler, codec, and progress APIs borrow
the limits snapshot and mutate one unique budget; they never duplicate an
allowance or materialize a complete profile on entry.

Each failed manual-poll teardown admission consumes one `HandlerSteps` unit and
increments the transaction's cumulative wait through
`handler_drain_steps_max`; host time uses the same wait origin and
`handler_drain_timeout_millis_max`. Reaching either applicable bound forces
cleanup transfer or the already reserved Producer residual commit. Repeated
polls cannot reset the count or deadline, so active `subscription_bytes` cannot
deadlock teardown by occupying the capacity needed to release itself.

Replacement reserves only new retained registration/engine bytes and a live
generation before publication; dispatch and subscription reservations follow
the timing frozen above.
Generation 2 may publish while generation 1 is pinned; another retained
generation is rejected until capacity is released. A late synchronous call
retains its pending-call reservation until actual return, independently of the
response gate.

## Lock, Callback, and Drop Rule

Every application, future, guard, cleanup, status, wake, and destructor callback
uses this four-phase boundary:

1. validate generation and state under the owning boundary and move the exact
   object or obligation into one unique `CallbackLease`;
2. publish the callback-in-progress state and nonce;
3. release every engine lock and constrained critical-section guard; and
4. invoke, poll, close, wake, drop, or report, then reacquire the boundary and
   commit only when generation and nonce still match.

This rule applies to sync invocation, async future creation/poll/drop, step
start/step/cancel, subscription setup/teardown, guard install/close, and
diagnostic/status callbacks. User code may reenter handler replacement, clear,
dispatch, stop, and lifecycle APIs subject to their published state.

## Acyclic Migration and Removal Checkpoints

The migration partial order is exact. Checkpoints 1 through 4 are sequential.
After WP-300, WP-400, WP-500, and WP-600 are unordered sibling branches; WP-700
joins all three branches. Numbering does not add an edge between the sibling
branches:

1. The WP-100 foundation-entry subtranche preserves the first 139
   `ResourceKind` indices and appends the exact 56-field v4.9 projection at
   `139..=194`. It also adds `WorkClass::HandlerSteps`, the three
   `PendingWorkClass` values, borrowed static profiles, nonduplicable budgets,
   profile snapshots, and boundary tests in the foundation/core packages owned
   by WP-100. It lands before the handler-code tranche and records
   `handler-foundation-refresh` without reopening the historical WP-000
   completion.
2. WP-100 adds the frozen values, 54 traits, static-registration value,
   public-but-internal `HostHandlerRegistration` factories and sealed ingress,
   execution reducers, selected-call ownership, constrained/static paths, and
   compatibility adapters. Core implements the ingress on `ExposedThing`; the
   target traits contain no `PushFn`. Existing cross-crate publication facades
   may remain crate-private compatibility code until their replacement exists.
3. WP-200 supplies real admitted plan and target facts. WP-100 does not create
   placeholder logical or binding plans.
4. WP-300 implements `ProducerEmission`, `BindingPublication`,
   `EmissionStatus`, and the binding-local `BindingEmissionSlot`, migrates
   protocol-neutral publication adapters, and records
   `producer-emission-migration` evidence. WP-400 later owns the distinct
   Servient-global `EmissionRecord` coordinator state.
5. The unordered post-WP-300 branches complete independently:
   - WP-400 activates every frozen host setter by constructing only the
     matching opaque core registration and calling its sealed ingress, activates
     Servient invocation, composes the Producer transaction, and removes
     `PushFn` and `SubscriptionSender` from the handler/Servient path.
   - WP-500 completes the Discovery client migration without creating a
     handler or publication dependency on WP-400 or WP-600.
   - WP-600 moves concrete binding publication to `ProducerEmission` and
     removes `PublisherSink` from the protocol-binding path.
6. WP-700 proves that all nine raw slot enums, raw handler lookups, `PushFn`,
   `SubscriptionSender`, and `PublisherSink` are absent from the public and
   cross-crate surfaces. No deprecated aliases or default-off compatibility
   feature remain.

This staging removes the former WP-100/WP-300 cycle. WP-100 freezes the new
handler surface without deleting the only publication path before WP-300
provides its replacement.

## Performance and Evidence

The exact gating workload ownership is:

- WP-100: `PERF-GW-020`, `PERF-GW-021`, `PERF-CS-015`, and `PERF-CS-016`;
- WP-400: `PERF-GW-022` and `PERF-CS-017`.

The locked manifests and fixtures define slot-density, sync/async/step,
replacement, late-return, reentrancy, setup/install/rollback, repeated teardown,
and constrained budget cases. The Producer workloads also cover an early
terminal with absent obligations, response claim/release acknowledgement and
delivery failure, teardown-view detach without call cancellation, async and
step teardown cancellation without a result, pending-call and handler-state
saturation, cleanup saturation with Producer-residual fallback, retained replay
before deterministic tombstone eviction, and stale replay after eviction.

Their fixed footprints and prefill values are part of workload identity. A
`handler_state_bytes_charged_after_active` value is the final charged total,
including the transaction's own active bytes. Pending-call and handler-state
prefill occurs only after the tested subscriptions are Active; cleanup prefill
occurs immediately before fallback; and Producer-residual prefill plus all
per-subscription pre-reservations equals the profile ceiling before setup.

The required deterministic gates include zero unsupported-operation handler
bytes, atomic replacement rollback, no more than two live slot generations,
zero engine guards during callbacks and destructors, at most one selected
handler invocation per operation/target dispatch, zero late-result delivery,
at most one setup and teardown call per subscription, zero publication before
Active, zero leaked guards, zero duplicate response-ack count changes, no
teardown-wait overshoot, zero early-terminal reservation leaks, and zero
constrained budget overshoot. A bulk protocol dispatch may invoke multiple
per-property handlers only through the separately bounded aggregation policy;
it does not reset the per-target invariant.

The exact evidence keys are:

- WP-100 `handler-foundation-refresh`, `handler-value-primitives`,
  `handler-api-matrix`, `handler-storage-replacement`, `handler-cancellation`,
  `affordance-target-no-atomics`, and `callback-lock-isolation`;
- WP-300 `producer-emission-migration`;
- WP-400 `producer-subscription-transaction`;
- WP-700 `legacy-handler-surface-removal`.

The API matrix evidence checks all 18 rows, all three execution forms, result
types, cfg cells, object-safety choices, host names, static registration, the
opaque host factories and sealed ingress, and private fields. It also checks
the exact `AcceptHint` shape, count-first admission, retained-byte accounting,
and absence of an unbounded iterator constructor. State evidence checks every
transition and owner in the four new machines plus response-owner separation
in `in-flight`. Resource evidence checks all twelve handler/Producer
boundaries, Accept-hint reuse of the candidate and handler-state limits, their
append-only `ResourceKind` indices, and atomic rollback. The no-atomic evidence
includes a no-default core build for `thumbv6m-none-eabi` or an equivalently
incapable target and an API/source rejection for `Arc` in `AffordanceTarget`.

## Binding Poll Signature Traceability

`docs/spec/binding-spi.md` is the sole normative owner of the
`PollClientBinding` contract. The following signature is a traceability copy of
that domain specification; this handler amendment neither refines nor overrides
it. At design revision v4.9 the canonical method name, associated-state slot,
receiver, and parameter order are:

```rust
fn poll_subscription_start(
    &mut self,
    cx: &mut Context<'_>,
    subscription: &mut ClientSubscriptionSlot<Self::SubscriptionState>,
    budget: &mut WorkBudget,
) -> Poll<CoreResult<SubscriptionStart>>;
```

The handler amendment checker requires this traceability copy to match the
domain specification exactly and requires the API ownership row to name
`PollClientBinding::poll_subscription_start`. Later binding packages must not
copy an older malformed or nongeneric skeleton.

## Audit Closure

This amendment and its registered artifacts close the audit findings as
follows:

- H-1 and H-2: exact ownership rows, values, matrix, signatures, results, and
  registration/removal surfaces;
- H-3: independent call ownership and exact sync/async/step reducers;
- H-4: the Producer subscription transaction and obligation rules;
- H-5: explicit WP-300/WP-400/WP-600 removal checkpoints;
- H-6: nine handler limits, three Producer residual limits, footprint
  declaration, admission, and drain policy;
- H-7: six locked workloads and named evidence keys;
- H-8: one domain-owned poll-signature trace and executable rejection; and
- H-9: owned-string target representation plus no-atomic evidence.

Handler implementation may resume only after the amendment, ownership matrix,
state artifact, resource profiles, performance manifests, work-package DAG,
and executable checker agree and the affected refactor gates are closed.
