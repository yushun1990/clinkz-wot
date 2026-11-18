//! Continuing invocation authority, independent of each finite native exchange.
//! No TD, native token interpretation, URL synthesis, retry or protocol cases.
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{
    action::raw_action_output_shape, consumer::*, consumer_action::*, data_schema::PreparedSchema,
    host_accounts::*, *,
};
use clinkz_wot_foundation::{MonotonicInstant, ResourceKind as K, WorkBudget, WorkClass as W};
use core::{
    any::Any,
    cell::{Cell, RefCell},
    cmp::Ordering,
    task::{Context, Poll, Waker},
};

use clinkz_wot_core::execution::action::{
    ActionDriver, ActionObservation, ActionProgress, ActionWork, step_action,
};
pub use clinkz_wot_core::execution::action::{ConsumerActionCancellation, ConsumerActionState};
#[derive(Clone, Debug)]
pub struct ConsumerActionIdentity {
    consumer: ConsumerInstanceRef,
    sequence: u64,
    authority: Rc<()>,
}
impl PartialEq for ConsumerActionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.authority, &other.authority)
    }
}
impl Eq for ConsumerActionIdentity {}
impl ConsumerActionIdentity {
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.consumer
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}
type AccessOwner = Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>;
/// Explicit per-operation access binding. Controls retain independently selected
/// registrations and may use independent actual policy services.
pub struct ConsumerActionAccess {
    pub invoke: AccessOwner,
    pub query: Option<AccessOwner>,
    pub cancel: Option<AccessOwner>,
}
struct CapturedAccess {
    owner: AccessOwner,
    work: u64,
    revision: u64,
}
impl CapturedAccess {
    fn check(&self) -> CoreResult<()> {
        if self.owner.work() != self.work
            || self.work == 0
            || self.owner.qualification_revision() != self.revision
            || self.owner.retained_bytes() != self.owner.backing_bytes()
            || !self
                .owner
                .supports_security(&clinkz_wot_core::security::SecurityRequirement::NOSEC)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
struct Selected<'a> {
    op: &'a ConsumerActionOperation,
    access: &'a CapturedAccess,
    policy: ConsumerDataPolicy,
    access_work: u64,
    bounds: ConsumerActionBounds,
    identity: BindingRegistrationIdentity,
}
impl Selected<'_> {
    fn check(
        &self,
        state: &State,
        control: &Control,
        continuation: Option<&dyn Any>,
        schema: Option<&PreparedSchema>,
    ) -> CoreResult<()> {
        control.check(state)?;
        self.check_services(schema)?;
        control.check(state)?;
        let destination = self.op.mapping.destination(continuation)?;
        self.check_services(schema)?;
        control.check(state)?;
        let result = self.access.owner.authorize_instance(
            &control.identity.consumer,
            self.identity,
            &destination,
            self.op.operation,
        );
        // Preserve dependency loss even when the policy callback also refuses.
        self.check_services(schema)?;
        control.check(state)?;
        result
    }
    fn check_services(&self, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        self.op.check_native_dependencies()?;
        self.access.check()?;
        self.check_data(schema)
    }
    fn check_data(&self, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        if let Some(schema) = schema {
            self.op.check_data_dependencies()?;
            if !self.op.data.supports_schema(schema) {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
struct Association {
    consumer: ConsumerInstanceRef,
    definition: Rc<ConsumerActionDefinition>,
    access: [Option<CapturedAccess>; 3],
    bank: Arc<HostResourceAccounts>,
    _funding: Vec<HostCapacityReservation>,
}
impl Association {
    fn operations(&self) -> [Option<&ConsumerActionOperation>; 3] {
        let d = self.definition.input();
        [Some(&d.invoke), d.query.as_ref(), d.cancel.as_ref()]
    }
    /// Complete qualification for a new association/invocation. Retaining this
    /// whole set does not make every operation a live dependency of old work.
    fn check_dependencies(&self) -> CoreResult<()> {
        self.definition.check_dependencies()?;
        for access in self.access.iter().flatten() {
            access.check()?;
        }
        Ok(())
    }
    /// One operation's original qualification, without importing unused
    /// siblings into an admitted invocation's invalidation scope.
    fn check_operation(&self, operation: usize, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        let selected = self.selected(operation)?;
        selected.check_services(schema)?;
        let definition = self.definition.input();
        if operation != 0 {
            selected
                .op
                .mapping
                .supports_continuation(definition.invoke.mapping.as_ref())?;
        }
        Ok(())
    }
    fn selected(&self, i: usize) -> CoreResult<Selected<'_>> {
        let op = self.operations()[i].ok_or_else(unsupported)?;
        let access = self.access[i].as_ref().ok_or_else(unsupported)?;
        Ok(Selected {
            op,
            access,
            policy: op.data_policy,
            access_work: access.work,
            bounds: op.bounds,
            identity: op.identity,
        })
    }
    fn validate(
        &self,
        operation: usize,
        schema: Option<&PreparedSchema>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        let selected = self.selected(operation)?;
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(s), Some(p))
                if p.body.len() as u64 <= selected.policy.body_bytes
                    && p.content_type == *selected.op.content_type
                    && p.content_coding.is_none() =>
            {
                // Late observations use the same original representation even
                // after authority loss. Never substitute the Invoke codec or
                // refresh a changed service's declaration before callback entry.
                selected.check_data(Some(s))?;
                let result = selected.op.data.validate(s, p);
                selected.check_data(Some(s))?;
                result
            }
            _ => Err(invalid()),
        }
    }
}
/// Persistent checked association, independent of invocation capacity. Clones
/// are passive non-revoking views; admitted work keeps the original association.
pub struct ConsumerActionAssociation {
    captured: Rc<Association>,
}
impl Clone for ConsumerActionAssociation {
    fn clone(&self) -> Self {
        Self {
            captured: self.captured.clone(),
        }
    }
}
impl ConsumerActionAssociation {
    #[allow(clippy::type_complexity)]
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definition: Rc<ConsumerActionDefinition>,
        access: ConsumerActionAccess,
        bank: Arc<HostResourceAccounts>,
        budget: &mut WorkBudget,
    ) -> Result<Self, BindingInputRejection<(Rc<ConsumerActionDefinition>, ConsumerActionAccess)>>
    {
        // This is an explicit new policy binding, never a replacement snapshot
        // of prepared native/data declarations.
        let captured = [
            Some(access.invoke.clone()),
            access.query.clone(),
            access.cancel.clone(),
        ]
        .map(|owner| {
            owner.map(|owner| CapturedAccess {
                work: owner.work(),
                revision: owner.qualification_revision(),
                owner,
            })
        });
        let build = (|| {
            definition.check_dependencies()?;
            if !consumer.is_current() {
                return Err(stale());
            }
            let d = definition.input();
            for (op, selected) in [Some(&d.invoke), d.query.as_ref(), d.cancel.as_ref()]
                .into_iter()
                .zip(&captured)
            {
                let (op, selected) = match (op, selected) {
                    (Some(op), Some(selected)) => (op, selected),
                    (None, None) => continue,
                    _ => return Err(invalid()),
                };
                selected.check()?;
                if !pay(budget, W::ProviderProbes, selected.work) {
                    return Err(busy());
                }
                let result = selected
                    .owner
                    .authorize(op.identity, &op.endpoint, op.operation);
                definition.check_dependencies()?;
                // A later operation's callback can invalidate an earlier owner.
                for selected in captured.iter().flatten() {
                    selected.check()?;
                }
                if !consumer.is_current() {
                    return Err(stale());
                }
                result?;
            }
            reserve(
                &bank,
                &[(
                    K::EngineLiveBytesGlobalMax,
                    core::mem::size_of::<Association>() as u64 + 128,
                )],
            )
        })();
        let funding = match build {
            Ok(funding) => funding,
            Err(error) => return Err(reject((definition, access), error)),
        };
        Ok(Self {
            captured: Rc::new(Association {
                consumer,
                definition,
                access: captured,
                bank,
                _funding: funding,
            }),
        })
    }
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.captured.consumer
    }
    pub fn definition(&self) -> &Rc<ConsumerActionDefinition> {
        &self.captured.definition
    }
}
struct Exchange(Option<Box<dyn ConsumerActionExchange>>);
impl Drop for Exchange {
    fn drop(&mut self) {
        if let Some(e) = self.0.take() {
            e.abort_local();
        }
    }
}
struct Invocation {
    progress: ActionProgress,
    control: Rc<Control>,
    continuation: Option<Box<dyn Any>>,
    exchange: Option<Exchange>,
    reply: Option<ConsumerActionReply>,
    native_result: Option<CoreResult<InteractionOutput>>,
    native_error: Option<CoreError>,
    input: Option<Payload>,
    operation: usize,
    effect: ConsumerEffect,
    association: Rc<Association>,
    _funding: Vec<HostCapacityReservation>,
}
struct State {
    bank: Arc<HostResourceAccounts>,
    active: Cell<bool>,
    registry: RefCell<Registry>,
    // Last: all original native, service and control owners precede funding.
    _funding: Vec<HostCapacityReservation>,
}
struct Registry {
    slots: Vec<Slot>,
    sequence: u64,
    next: usize,
}
#[allow(clippy::large_enum_variant)]
enum Slot {
    Vacant,
    Reserved {
        sequence: u64,
        control: Option<Rc<Control>>,
    },
    Ready(Box<Invocation>),
}
impl Slot {
    fn control(&self) -> Option<&Rc<Control>> {
        match self {
            Self::Vacant => None,
            Self::Reserved { control, .. } => control.as_ref(),
            Self::Ready(v) => Some(&v.control),
        }
    }
}
/// Passive intent remains accessible while one stack frame owns an invocation.
/// This is continuing invocation control, never a second native exchange owner.
struct Control {
    identity: ConsumerActionIdentity,
    cause: RefCell<Option<CoreError>>,
    deadline: Deadline,
    last_time: Cell<MonotonicInstant>,
    abandoned: Cell<bool>,
    waker: RefCell<Option<Waker>>,
    recheck: Cell<bool>,
    _funding: HostCapacityReservation,
}
impl Control {
    fn allocation_bytes() -> u64 {
        (core::mem::size_of::<Self>() + 4 * core::mem::size_of::<usize>()) as u64
    }
    fn new(
        consumer: &ConsumerInstanceRef,
        sequence: u64,
        bank: &Arc<HostResourceAccounts>,
        deadline: Deadline,
        now: MonotonicInstant,
    ) -> CoreResult<Rc<Self>> {
        // Both Rc allocations, including the separately cloneable identity.
        let funding = bank
            .reserve(K::EngineLiveBytesGlobalMax, Self::allocation_bytes())
            .ok_or_else(busy)?;
        Ok(Rc::new(Self {
            identity: ConsumerActionIdentity {
                consumer: consumer.clone(),
                sequence,
                authority: Rc::new(()),
            },
            cause: RefCell::new(None),
            deadline,
            last_time: Cell::new(now),
            abandoned: Cell::new(false),
            waker: RefCell::new(None),
            recheck: Cell::new(false),
            _funding: funding,
        }))
    }
    fn record(&self, cause: CoreError) {
        self.cause.borrow_mut().get_or_insert(cause);
    }
    fn cause(&self) -> Option<CoreError> {
        self.cause.borrow().clone()
    }
    fn observe_time(&self, now: MonotonicInstant) {
        match now.checked_cmp(self.last_time.get()) {
            Some(Ordering::Equal | Ordering::Greater) => self.last_time.set(now),
            _ => self.record(invalid()),
        }
    }
    fn check(&self, state: &State) -> CoreResult<()> {
        if let Err(e) = check_time(self.deadline, self.last_time.get()) {
            self.record(e);
        }
        if !state.active.get() || !self.identity.consumer.is_current() {
            self.record(stale());
        }
        self.cause().map_or(Ok(()), Err)
    }
    fn register(&self, waker: &Waker) {
        let next = waker.clone();
        let old = self.waker.borrow_mut().replace(next);
        drop(old);
    }
    fn wake(&self) {
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
/// An occupied slot is never temporarily vacant during callbacks or unwind.
struct Lease<'a> {
    state: &'a State,
    slot: usize,
    sequence: u64,
    invocation: Option<Box<Invocation>>,
}
impl State {
    fn reserve_slot(&self) -> CoreResult<Lease<'_>> {
        let mut r = self.registry.borrow_mut();
        let slot = r
            .slots
            .iter()
            .position(|s| matches!(s, Slot::Vacant))
            .ok_or_else(busy)?;
        let sequence = r.sequence.checked_add(1).ok_or_else(busy)?;
        r.sequence = sequence;
        r.slots[slot] = Slot::Reserved {
            sequence,
            control: None,
        };
        Ok(Lease {
            state: self,
            slot,
            sequence,
            invocation: None,
        })
    }
    fn checkout(
        &self,
        slot: usize,
        sequence: Option<u64>,
        waker: Option<&Waker>,
    ) -> CoreResult<Lease<'_>> {
        let (lease, control) = {
            let mut r = self.registry.borrow_mut();
            let entry = &mut r.slots[slot];
            match entry {
                Slot::Ready(v) if sequence.is_none_or(|n| n == v.control.identity.sequence) => {
                    let control = v.control.clone();
                    let sequence = control.identity.sequence;
                    let Slot::Ready(v) = core::mem::replace(
                        entry,
                        Slot::Reserved {
                            sequence,
                            control: Some(control.clone()),
                        },
                    ) else {
                        unreachable!()
                    };
                    (
                        Some(Lease {
                            state: self,
                            slot,
                            sequence,
                            invocation: Some(v),
                        }),
                        control,
                    )
                }
                Slot::Reserved {
                    sequence: original,
                    control: Some(control),
                } if sequence.is_none_or(|n| n == *original) => {
                    if waker.is_some() {
                        control.recheck.set(true);
                    }
                    (None, control.clone())
                }
                _ => return Err(stale()),
            }
        };
        // A waker's clone/drop/wake is also linked code. The lease already exists
        // so even unwinding registration restores the original occupied owner.
        if let Some(waker) = waker {
            control.register(waker);
        }
        lease.ok_or_else(busy)
    }
    fn revoke(&self) {
        self.active.set(false);
        for control in self
            .registry
            .borrow()
            .slots
            .iter()
            .filter_map(Slot::control)
        {
            control.record(stale());
        }
    }
    fn close(&self) {
        self.revoke();
        let count = self.registry.borrow().slots.len();
        for i in 0..count {
            let control = self.registry.borrow().slots[i].control().cloned();
            if let Some(control) = control {
                control.wake();
            }
        }
    }
}
impl Lease<'_> {
    fn attach(&self, control: Rc<Control>) {
        let mut r = self.state.registry.borrow_mut();
        let Slot::Reserved { control: c, .. } = &mut r.slots[self.slot] else {
            unreachable!()
        };
        *c = Some(control);
    }
    fn take(mut self) -> Box<Invocation> {
        self.invocation.take().unwrap()
    }
}
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        let replacement = self.invocation.take().map_or(Slot::Vacant, Slot::Ready);
        let old = {
            let mut r = self.state.registry.borrow_mut();
            core::mem::replace(&mut r.slots[self.slot], replacement)
        };
        let Slot::Reserved { sequence, control } = old else {
            unreachable!()
        };
        debug_assert_eq!(sequence, self.sequence);
        if let Some(control) = control
            && control.recheck.replace(false)
        {
            control.wake();
        }
    }
}
/// Bounded continuing Action execution capacity shared by independent Consumer
/// associations. No definition, instance or native exchange is allocated per
/// vacant slot. Drop/close fences this capacity's admitted work only.
pub struct ConsumerActions {
    state: Rc<State>,
}
impl ConsumerActions {
    pub fn try_new(invocations: usize, bank: Arc<HostResourceAccounts>) -> CoreResult<Self> {
        if !(1..=32).contains(&invocations) {
            return Err(invalid());
        }
        let bytes = (core::mem::size_of::<State>()
            + 2 * core::mem::size_of::<usize>()
            + invocations * core::mem::size_of::<Slot>()) as u64;
        let funding = reserve(&bank, &[(K::EngineLiveBytesGlobalMax, bytes)])?;
        let mut slots = Vec::new();
        slots.try_reserve_exact(invocations).map_err(|_| busy())?;
        slots.resize_with(invocations, || Slot::Vacant);
        Ok(Self {
            state: Rc::new(State {
                bank,
                active: Cell::new(true),
                registry: RefCell::new(Registry {
                    slots,
                    sequence: 0,
                    next: 0,
                }),
                _funding: funding,
            }),
        })
    }
    /// No input or native opportunity is lost on pre-effect refusal. One
    /// accepted invocation is never replayed by progress, query or cancellation.
    pub fn invoke_action(
        &self,
        association: &ConsumerActionAssociation,
        input: Option<Payload>,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerActionHandle, BindingInputRejection<Option<Payload>>> {
        let state = &self.state;
        let checked = (|| {
            let d = association.captured.clone();
            if !state.active.get() || !d.consumer.is_current() {
                return Err(stale());
            }
            d.check_dependencies()?;
            check_time(deadline, now)?;
            let reserved = state.reserve_slot()?;
            let control = Control::new(&d.consumer, reserved.sequence, &state.bank, deadline, now)?;
            reserved.attach(control.clone());
            let invoke = d.selected(0)?;
            let mut native = 0;
            let mut temporary = 0;
            let mut continuation = 0;
            let mut response = 0;
            for s in d.operations().into_iter().flatten() {
                native = native.max(s.bounds.exchange_bytes);
                temporary = temporary.max(s.bounds.temporary_bytes);
                let schemas = &d.definition.input().schemas;
                let has_data = match s.operation {
                    Operation::InvokeAction => schemas.input.is_some() || schemas.output.is_some(),
                    Operation::QueryAction => schemas.output.is_some(),
                    _ => false,
                };
                if has_data {
                    temporary = temporary.max(s.data_policy.temporary_bytes);
                }
                continuation = continuation.max(s.bounds.continuation_bytes);
                response = response.max(s.bounds.response_bytes);
            }
            if temporary
                > state
                    .bank
                    .account(K::BindingPollTemporaryBytesPerCallMax)
                    .limit()
            {
                return Err(busy());
            }
            let input_bytes =
                clinkz_wot_core::notification::notification_payload_backing(input.as_ref())
                    .ok_or_else(invalid)? as u64;
            let lifetime = native
                .checked_add(continuation)
                .and_then(|n| n.checked_add(response))
                .and_then(|n| n.checked_add(input_bytes))
                .and_then(|n| n.checked_add(core::mem::size_of::<Invocation>() as u64 + 512))
                .ok_or_else(invalid)?;
            let mut funding = reserve(
                &state.bank,
                &[
                    (
                        K::EngineLiveBytesGlobalMax,
                        lifetime.checked_add(temporary).ok_or_else(invalid)?,
                    ),
                    (K::BindingPollTemporaryBytesGlobalMax, temporary),
                    (
                        K::CleanupBytesMax,
                        lifetime
                            .checked_add(Control::allocation_bytes())
                            .ok_or_else(invalid)?,
                    ),
                    (K::CleanupItemsMax, 1),
                    (K::PendingHandlerCallsGlobalMax, 1),
                    (K::BindingResponseBufferBytesGlobalMax, response),
                ],
            )?;
            // A separate association domain constrains outstanding obligations,
            // including unread/retained results; this is a quota, not RAM backing.
            if !Arc::ptr_eq(&d.bank, &state.bank) {
                funding.try_reserve_exact(1).map_err(|_| busy())?;
                funding.extend(reserve(&d.bank, &[(K::PendingHandlerCallsGlobalMax, 1)])?);
            }
            let schema = d.definition.input().schemas.input.as_ref();
            prepay(&invoke, budget, schema.is_some())?;
            invoke.check(state, &control, None, schema)?;
            d.validate(0, schema, input.as_ref())?;
            invoke.check(state, &control, None, schema)?;
            d.check_dependencies()?;
            control.check(state)?;
            // Allocate the separately funded record before native effects. Input
            // stays caller-owned until start succeeds, including on refusal.
            let mut invocation = Box::new(Invocation {
                progress: ActionProgress::default(),
                control,
                association: d.clone(),
                continuation: None,
                exchange: None,
                reply: None,
                native_result: None,
                native_error: None,
                input: None,
                operation: 0,
                effect: ConsumerEffect::Unknown,
                _funding: funding,
            });
            let raw = invoke.op.mapping.start(input.as_ref(), None)?;
            invocation.exchange = Some(Exchange(Some(raw)));
            Ok((reserved, invocation))
        })();
        let (mut reserved, mut invocation) = match checked {
            Ok(v) => v,
            Err(e) => return Err(reject(input, e)),
        };
        let slot = reserved.slot;
        let sequence = reserved.sequence;
        let control = invocation.control.clone();
        invocation.input = input;
        // A successful start may own native effects already. Keep its complete
        // returned exchange even if the callback closed this invocation.
        invocation.fence(state);
        reserved.invocation = Some(invocation);
        drop(reserved);
        Ok(ConsumerActionHandle {
            state: self.state.clone(),
            slot,
            sequence,
            control,
        })
    }
    pub fn begin_close(&self) {
        self.state.close();
    }
    pub fn step(&self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        let (start, count) = {
            let mut r = self.state.registry.borrow_mut();
            for control in r.slots.iter().filter_map(Slot::control) {
                control.observe_time(now);
            }
            let count = r.slots.len();
            let start = r.next;
            r.next = (start + 1) % count;
            (start, count)
        };
        for i in 0..count {
            if let Ok(mut lease) = self
                .state
                .checkout((start + i) % count, None, Some(cx.waker()))
            {
                let v = lease.invocation.as_mut().unwrap();
                v.step(&self.state, cx, budget);
                if v.control.abandoned.get() && v.progress.state.settlement_ready() {
                    drop(lease.take());
                }
            }
        }
    }
    pub fn is_idle(&self) -> bool {
        self.state
            .registry
            .borrow()
            .slots
            .iter()
            .all(|s| matches!(s, Slot::Vacant))
    }
}
impl Drop for ConsumerActions {
    fn drop(&mut self) {
        self.state.revoke();
    }
}

/// Unique local authority over an original invocation. No raw token or caller-
/// supplied operation mapping can address/replace this handle's work.
pub struct ConsumerActionHandle {
    state: Rc<State>,
    slot: usize,
    sequence: u64,
    control: Rc<Control>,
}
impl ConsumerActionHandle {
    fn live(&self) -> CoreResult<()> {
        if self.state.registry.borrow().slots[self.slot]
            .control()
            .is_some_and(|c| Rc::ptr_eq(c, &self.control))
        {
            Ok(())
        } else {
            Err(stale())
        }
    }
    pub fn identity(&self) -> CoreResult<ConsumerActionIdentity> {
        self.live()?;
        Ok(self.control.identity.clone())
    }
    fn get<'a>(&self, r: &'a Registry) -> CoreResult<&'a Invocation> {
        match &r.slots[self.slot] {
            Slot::Ready(v) if v.control.identity.sequence == self.sequence => Ok(v),
            Slot::Reserved { sequence, .. } if *sequence == self.sequence => Err(busy()),
            _ => Err(stale()),
        }
    }
    /// Inspection of native state while its owner is executing returns Backpressure.
    pub fn state(&self) -> CoreResult<ConsumerActionState> {
        let r = self.state.registry.borrow();
        let v = self.get(&r)?;
        Ok(if self.control.cause().is_some() {
            if v.exchange.is_some() {
                ConsumerActionState::Closing
            } else {
                ConsumerActionState::Closed
            }
        } else {
            v.progress.state
        })
    }
    pub fn first_cause(&self) -> CoreResult<Option<CoreError>> {
        self.live()?;
        Ok(self.control.cause())
    }
    pub fn cleanup_error(&self) -> CoreResult<Option<CoreError>> {
        Ok(self
            .get(&self.state.registry.borrow())?
            .progress
            .cleanup_error
            .clone())
    }
    pub fn remote_cancellation(&self) -> CoreResult<ConsumerActionCancellation> {
        Ok(self
            .get(&self.state.registry.borrow())?
            .progress
            .remote_cancel)
    }
    /// Explicit described control; neither call automatically queries/reinvokes.
    /// A reentrant control of the checked-out original invocation is busy.
    pub fn query(&self, now: MonotonicInstant, budget: &mut WorkBudget) -> CoreResult<()> {
        self.control(1, now, budget)
    }
    pub fn request_cancel(&self, now: MonotonicInstant, budget: &mut WorkBudget) -> CoreResult<()> {
        self.control(2, now, budget)
    }
    fn control(
        &self,
        operation: usize,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.live()?;
        self.control.observe_time(now);
        let mut lease = self.state.checkout(self.slot, Some(self.sequence), None)?;
        let v = lease.invocation.as_mut().unwrap();
        v.fence(&self.state);
        self.control.check(&self.state)?;
        if v.progress.state != ConsumerActionState::Pending || v.exchange.is_some() {
            return Err(busy());
        }
        let association = v.association.clone();
        let selected = association.selected(operation)?;
        // Controls carry no application input. Their output validation is paid
        // separately before native progress can return a completed value.
        prepay(&selected, budget, false)?;
        let schema = if operation == 1 {
            association.definition.input().schemas.output.as_ref()
        } else {
            None
        };
        let continuation = v.continuation.as_deref().ok_or_else(invalid)?;
        let preflight = selected
            .check(&self.state, &self.control, Some(continuation), schema)
            .and_then(|()| association.check_operation(operation, schema));
        // The requested control has no admitted work yet. Refusal cannot
        // withdraw Invoke authority, but its callbacks may invalidate the
        // invocation's original dependencies. Record that observation before
        // returning an error or permitting another native effect.
        v.fence(&self.state);
        self.control.check(&self.state)?;
        preflight?;
        let continuation = v.continuation.as_deref().ok_or_else(invalid)?;
        let raw = match selected.op.mapping.start(None, Some(continuation)) {
            Ok(raw) => raw,
            Err(error) => {
                v.fence(&self.state);
                self.control.check(&self.state)?;
                return Err(error);
            }
        };
        v.exchange = Some(Exchange(Some(raw)));
        v.operation = operation;
        v.progress.retiring = false;
        v.reply = None;
        v.progress.state = if operation == 1 {
            ConsumerActionState::Querying
        } else {
            ConsumerActionState::Cancelling
        };
        if operation == 2 {
            v.progress.remote_cancel = ConsumerActionCancellation::Uncertain;
        }
        v.fence(&self.state);
        Ok(())
    }
    /// Stops local work only. Does not send a hidden CancelAction or claim that
    /// a remote effect was undone. First cause and any late result are retained.
    pub fn cancel_local(&self) {
        if self.live().is_ok() {
            self.control.record(CoreError::Cancelled(context()));
            self.control.wake();
        }
    }
    /// Reentrant progress records time and wake interest but cannot poll twice.
    /// It returns Backpressure until the outer owner restores the slot.
    pub fn progress(
        &self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerActionState> {
        self.live()?;
        self.control.observe_time(now);
        let mut lease = self
            .state
            .checkout(self.slot, Some(self.sequence), Some(cx.waker()))?;
        let v = lease.invocation.as_mut().unwrap();
        v.step(&self.state, cx, budget);
        Ok(v.progress.state)
    }
    /// Native completion remains revocable until this paid, fresh handoff.
    /// Busy/short-credit entry retains the original slot and complete evidence.
    pub fn take_settlement(
        &self,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerActionSettlement> {
        self.live()?;
        self.control.observe_time(now);
        let mut lease = self.state.checkout(self.slot, Some(self.sequence), None)?;
        let v = lease.invocation.as_mut().unwrap();
        v.fence(&self.state);
        if !v.progress.state.settlement_ready() || !pay(budget, W::CleanupItems, 1) {
            return Err(busy());
        }
        v.refresh_access(&self.state, budget)?;
        v.fence(&self.state);
        Ok(ConsumerActionSettlement(lease.take()))
    }
}
impl Drop for ConsumerActionHandle {
    fn drop(&mut self) {
        if self.live().is_ok() {
            self.control.abandoned.set(true);
            self.control.record(CoreError::Cancelled(context()));
        }
    }
}
/// Complete original invocation/result owner. Lends bytes without detaching
/// their original funding; share this owner to retain the original backing.
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::consumer_action::ConsumerActionSettlement;
/// use std::sync::Arc;
/// fn detach(s: &ConsumerActionSettlement) -> Arc<[u8]> {
///     Arc::clone(&s.result().unwrap().data().unwrap().body)
/// }
/// ```
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::consumer_action::ConsumerActionSettlement;
/// use std::sync::Arc;
/// fn detach(s: &ConsumerActionSettlement) -> Arc<[u8]> {
///     Arc::clone(&s.native_result().unwrap().unwrap().data().unwrap().body)
/// }
/// ```
///
/// ```compile_fail,E0308
/// use clinkz_wot_servient_finite::consumer_action::ConsumerActionSettlement;
/// use std::sync::Arc;
/// fn detach(s: &ConsumerActionSettlement) -> Arc<[u8]> {
///     Arc::clone(&s.input().unwrap().body)
/// }
/// ```
///
/// ```compile_fail,E0515
/// use clinkz_wot_servient_finite::consumer_action::ConsumerActionSettlement;
/// fn escape(s: ConsumerActionSettlement) -> &'static [u8] {
///     s.result().unwrap().data().unwrap().body
/// }
/// ```
pub struct ConsumerActionSettlement(Box<Invocation>);
impl ConsumerActionSettlement {
    pub fn input(&self) -> Option<PayloadView<'_>> {
        self.0.input.as_ref().map(Payload::view)
    }
    pub fn identity(&self) -> &ConsumerActionIdentity {
        &self.0.control.identity
    }
    pub fn effect(&self) -> ConsumerEffect {
        self.0.effect
    }
    pub fn state(&self) -> ConsumerActionState {
        self.0.progress.state
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.0.progress.first_cause.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.0.progress.cleanup_error.as_ref()
    }
    pub fn validation_error(&self) -> Option<&CoreError> {
        self.0.progress.validation_error.as_ref()
    }
    pub fn accepted(&self) -> bool {
        self.0.progress.accepted
    }
    pub fn remote_cancellation(&self) -> ConsumerActionCancellation {
        self.0.progress.remote_cancel
    }
    pub fn native_result(&self) -> Option<Result<InteractionOutputView<'_>, &CoreError>> {
        self.0
            .native_result
            .as_ref()
            .map(|r| r.as_ref().map(InteractionOutput::view))
    }
    /// Original poll failure, independent of a subsequently retained late output
    /// and of an earlier local cancellation or cleanup failure.
    pub fn native_error(&self) -> Option<&CoreError> {
        self.0.native_error.as_ref()
    }
    pub fn result(&self) -> CoreResult<InteractionOutputView<'_>> {
        if let Some(e) = &self.0.progress.first_cause {
            return Err(e.clone());
        }
        match &self.0.native_result {
            Some(Ok(o)) if self.0.progress.state == ConsumerActionState::Completed => Ok(o.view()),
            Some(Err(e)) => Err(e.clone()),
            _ => Err(invalid()),
        }
    }
    pub fn selected_form(
        &self,
        operation: Operation,
    ) -> Option<(BindingRegistrationIdentity, u32)> {
        let i = match operation {
            Operation::InvokeAction => 0,
            Operation::QueryAction => 1,
            Operation::CancelAction => 2,
            _ => return None,
        };
        self.0.association.operations()[i].map(|s| (s.identity, s.form))
    }
}
impl Invocation {
    fn live_output_schema(&self) -> Option<&PreparedSchema> {
        if self.operation != 2
            && (self.exchange.is_some()
                || matches!(self.reply, Some(ConsumerActionReply::Complete(_)))
                || matches!(self.native_result, Some(Ok(_))))
        {
            self.association.definition.input().schemas.output.as_ref()
        } else {
            None
        }
    }
    fn check_dependencies(&self) -> CoreResult<()> {
        // Invoke owns native continuation and authority throughout. A retired
        // input codec does not own a Query representation. Only the exchange
        // or unread completed value keeps its actual output codec live.
        self.association.check_operation(0, None)?;
        if self.operation != 0 {
            self.association.check_operation(self.operation, None)?;
        }
        self.association
            .selected(self.operation)?
            .check_data(self.live_output_schema())
    }
    fn close(&mut self, cause: CoreError) {
        self.control.record(cause);
        self.progress.first_cause = self.control.cause();
        self.progress.retiring = self.exchange.is_some();
        self.progress.state = if self.progress.retiring {
            ConsumerActionState::Closing
        } else {
            ConsumerActionState::Closed
        };
    }
    /// Passive controls are rechecked after linked callbacks, including Pending.
    fn fence(&mut self, state: &State) {
        if let Err(e) = self.control.check(state) {
            self.close(e);
        }
        if self.progress.first_cause.is_none()
            && let Err(e) = self.check_dependencies()
        {
            self.close(e);
        }
        if let Err(e) = self.control.check(state) {
            self.close(e);
        }
    }
    fn refresh_access(&mut self, state: &State, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.progress.first_cause.is_some() {
            return Ok(());
        }
        let selected = self.association.selected(self.operation)?;
        // The native bound funds destination calculation, not a new exchange.
        // No codec or native I/O callback is needed for an already validated result.
        if !pay(budget, W::ProviderProbes, selected.access_work)
            || !pay(budget, W::BindingPolls, selected.bounds.poll_work)
        {
            return Err(busy());
        }
        if let Err(e) = selected
            .check(
                state,
                &self.control,
                self.continuation.as_deref(),
                self.live_output_schema(),
            )
            .and_then(|()| self.check_dependencies())
        {
            self.close(e);
        }
        Ok(())
    }
    fn step(&mut self, state: &State, cx: &mut Context<'_>, budget: &mut WorkBudget) {
        step_action(
            &mut HostActionDriver {
                invocation: self,
                state,
            },
            cx,
            budget,
        );
    }
}

struct HostActionDriver<'a> {
    invocation: &'a mut Invocation,
    state: &'a State,
}
impl ActionDriver for HostActionDriver<'_> {
    fn progress(&mut self) -> &mut ActionProgress {
        &mut self.invocation.progress
    }
    fn operation(&self) -> Operation {
        [
            Operation::InvokeAction,
            Operation::QueryAction,
            Operation::CancelAction,
        ][self.invocation.operation]
    }
    fn has_exchange(&self) -> bool {
        self.invocation.exchange.is_some()
    }
    fn work(&self) -> ActionWork {
        let v = &self.invocation;
        let s = v
            .association
            .selected(v.operation)
            .expect("captured operation");
        ActionWork {
            access: s.access_work,
            native: s.bounds.poll_work,
            validation: if v.live_output_schema().is_some() {
                s.policy.validation_work
            } else {
                0
            },
        }
    }
    fn fence(&mut self) {
        self.invocation.fence(self.state);
    }
    fn close(&mut self, cause: CoreError) {
        self.invocation.close(cause);
    }
    fn invalid_reply(&self) -> CoreError {
        invalid()
    }
    fn check_current(&mut self) -> CoreResult<()> {
        let v = &self.invocation;
        v.association.selected(v.operation)?.check(
            self.state,
            &v.control,
            v.continuation.as_deref(),
            v.live_output_schema(),
        )
    }
    fn refresh_access(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        self.invocation.refresh_access(self.state, budget)
    }
    fn poll_exchange(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let v = &mut self.invocation;
        v.exchange
            .as_mut()
            .unwrap()
            .0
            .as_mut()
            .unwrap()
            .poll(cx)
            .map(|result| match result {
                Ok(reply) => {
                    v.reply = Some(reply);
                    Ok(())
                }
                Err(e) => {
                    v.native_error.get_or_insert_with(|| e.clone());
                    v.native_result = Some(Err(e.clone()));
                    Err(e)
                }
            })
    }
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let v = &mut self.invocation;
        v.exchange
            .as_mut()
            .unwrap()
            .0
            .as_mut()
            .unwrap()
            .poll_close(cx)
            .map(|result| {
                result.map(|late| {
                    if v.reply.is_none() {
                        v.reply = late;
                    }
                    v.exchange = None;
                })
            })
    }
    fn retain_reply(&mut self) -> Option<ActionObservation> {
        let v = &mut self.invocation;
        Some(match v.reply.take()? {
            ConsumerActionReply::Accepted(token) if v.operation == 0 => {
                v.continuation = Some(token);
                ActionObservation::Accepted
            }
            ConsumerActionReply::Accepted(_) => ActionObservation::Invalid,
            ConsumerActionReply::Complete(output) => {
                v.native_result = Some(Ok(output));
                ActionObservation::Complete
            }
            ConsumerActionReply::Failed(e) => {
                v.native_result = Some(Err(e));
                ActionObservation::Failed
            }
            ConsumerActionReply::Pending => ActionObservation::Pending,
            ConsumerActionReply::CancelRequested => ActionObservation::CancelRequested,
            ConsumerActionReply::AlreadyTerminal => ActionObservation::AlreadyTerminal,
        })
    }
    fn validate_output(&mut self) -> CoreResult<()> {
        let v = &self.invocation;
        let output = v.native_result.as_ref().unwrap().as_ref().unwrap();
        if !raw_action_output_shape(output)
            || interaction_output_backing(output).is_none_or(|n| {
                n as u64
                    > v.association
                        .selected(v.operation)
                        .unwrap()
                        .bounds
                        .response_bytes
            })
        {
            return Err(invalid());
        }
        v.association.validate(
            v.operation,
            v.association.definition.input().schemas.output.as_ref(),
            output.data(),
        )
    }
    fn reset_operation(&mut self) {
        self.invocation.operation = 0;
    }
}

fn prepay(s: &Selected<'_>, budget: &mut WorkBudget, codec: bool) -> CoreResult<()> {
    s.access_work.checked_mul(2).ok_or_else(invalid)?;
    if (ActionWork {
        access: s.access_work,
        native: s.bounds.poll_work,
        validation: if codec { s.policy.validation_work } else { 0 },
    })
    .prepay(budget)
    {
        Ok(())
    } else {
        Err(busy())
    }
}

fn check_time(deadline: Deadline, now: MonotonicInstant) -> CoreResult<()> {
    match deadline.checked_is_elapsed_at(now) {
        Some(false) => Ok(()),
        Some(true) => Err(CoreError::TimedOut(context())),
        None => Err(invalid()),
    }
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn reject<T>(input: T, e: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(e))
}
fn pay(b: &mut WorkBudget, w: W, n: u64) -> bool {
    b.consume(w, n).is_ok()
}
