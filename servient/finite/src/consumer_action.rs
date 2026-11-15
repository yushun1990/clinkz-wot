//! Continuing invocation authority, independent of each finite native exchange.
//! No TD, native token interpretation, URL synthesis, retry or protocol cases.
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{
    action::raw_action_output_shape, consumer::*, consumer_action::*, host_accounts::*, *,
};
use clinkz_wot_foundation::{MonotonicInstant, ResourceKind as K, WorkBudget, WorkClass as W};
use core::{
    any::Any,
    cell::{Cell, RefCell},
    cmp::Ordering,
    task::{Context, Poll, Waker},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerActionState {
    Invoking,
    Pending,
    Querying,
    Cancelling,
    Closing,
    /// Validated native completion, still revocable until settlement handoff.
    Completed,
    Failed,
    Closed,
}
impl ConsumerActionState {
    fn settlement_ready(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Closed)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerActionCancellation {
    NotRequested,
    /// A native cancel may have reached the peer. Local failure proves no more.
    Uncertain,
    /// Peer accepted a cancellation request; execution may still complete.
    Requested,
    /// Peer reports terminal execution, without supplying the terminal result.
    AlreadyTerminal,
}
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
            || self.owner.security_revision() != self.revision
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
    ) -> CoreResult<()> {
        control.check(state)?;
        self.check_services()?;
        control.check(state)?;
        let destination = self.op.mapping.destination(continuation)?;
        self.check_services()?;
        control.check(state)?;
        let result = self.access.owner.authorize_instance(
            &control.identity.consumer,
            self.identity,
            &destination,
            self.op.operation,
        );
        // Preserve dependency loss even when the policy callback also refuses.
        self.check_services()?;
        control.check(state)?;
        result
    }
    fn check_services(&self) -> CoreResult<()> {
        self.op.check_dependencies()?;
        self.access.check()
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
    fn check_dependencies(&self) -> CoreResult<()> {
        self.definition.check_dependencies()?;
        for access in self.access.iter().flatten() {
            access.check()?;
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
        schema: Option<&clinkz_wot_core::data_schema::PreparedSchema>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        let current = || {
            let d = self.definition.input();
            let op = &d.invoke;
            if !op.data.available()
                || op.data.policy() != op.data_policy
                || op.data_policy.retained_bytes != op.data.backing_bytes()
                || op.data.content_type() != &*op.content_type
                || !d.schemas.supports(op.data.service())
            {
                return Err(invalid());
            }
            Ok(())
        };
        current()?;
        let selected = self.selected(0)?;
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(s), Some(p)) if p.body.len() as u64 <= selected.policy.body_bytes => {
                let result = selected.op.data.validate(s, p);
                current()?;
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
                revision: owner.security_revision(),
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
    control: Rc<Control>,
    continuation: Option<Box<dyn Any>>,
    exchange: Option<Exchange>,
    reply: Option<ConsumerActionReply>,
    native_result: Option<CoreResult<InteractionOutput>>,
    native_error: Option<CoreError>,
    input: Option<Payload>,
    state: ConsumerActionState,
    operation: usize,
    retiring: bool,
    first_cause: Option<CoreError>,
    cleanup_error: Option<CoreError>,
    validation_error: Option<CoreError>,
    accepted: bool,
    effect: ConsumerEffect,
    remote_cancel: ConsumerActionCancellation,
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
            let mut temporary = invoke.policy.temporary_bytes;
            let mut continuation = 0;
            let mut response = 0;
            for s in d.operations().into_iter().flatten() {
                native = native.max(s.bounds.exchange_bytes);
                temporary = temporary
                    .max(s.bounds.temporary_bytes)
                    .max(s.data_policy.temporary_bytes);
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
            prepay(&invoke, budget)?;
            invoke.check(state, &control, None)?;
            d.validate(d.definition.input().schemas.input.as_ref(), input.as_ref())?;
            invoke.check(state, &control, None)?;
            d.check_dependencies()?;
            control.check(state)?;
            // Allocate the separately funded record before native effects. Input
            // stays caller-owned until start succeeds, including on refusal.
            let mut invocation = Box::new(Invocation {
                control,
                association: d.clone(),
                continuation: None,
                exchange: None,
                reply: None,
                native_result: None,
                native_error: None,
                input: None,
                state: ConsumerActionState::Invoking,
                operation: 0,
                retiring: false,
                first_cause: None,
                cleanup_error: None,
                validation_error: None,
                accepted: false,
                effect: ConsumerEffect::Unknown,
                remote_cancel: ConsumerActionCancellation::NotRequested,
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
                if v.control.abandoned.get() && v.state.settlement_ready() {
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
            v.state
        })
    }
    pub fn first_cause(&self) -> CoreResult<Option<CoreError>> {
        self.live()?;
        Ok(self.control.cause())
    }
    pub fn cleanup_error(&self) -> CoreResult<Option<CoreError>> {
        Ok(self
            .get(&self.state.registry.borrow())?
            .cleanup_error
            .clone())
    }
    pub fn remote_cancellation(&self) -> CoreResult<ConsumerActionCancellation> {
        Ok(self.get(&self.state.registry.borrow())?.remote_cancel)
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
        if v.state != ConsumerActionState::Pending || v.exchange.is_some() {
            return Err(busy());
        }
        let selected = v.association.selected(operation)?;
        prepay(&selected, budget)?;
        let continuation = v.continuation.as_deref().ok_or_else(invalid)?;
        if let Err(e) = selected
            .check(&self.state, &self.control, Some(continuation))
            .and_then(|()| v.association.check_dependencies())
            .and_then(|()| self.control.check(&self.state))
        {
            v.close(e.clone());
            return Err(e);
        }
        let raw = selected.op.mapping.start(None, Some(continuation))?;
        v.exchange = Some(Exchange(Some(raw)));
        v.operation = operation;
        v.retiring = false;
        v.reply = None;
        v.state = if operation == 1 {
            ConsumerActionState::Querying
        } else {
            ConsumerActionState::Cancelling
        };
        if operation == 2 {
            v.remote_cancel = ConsumerActionCancellation::Uncertain;
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
        Ok(v.state)
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
        if !v.state.settlement_ready() || !pay(budget, W::CleanupItems, 1) {
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
        self.0.state
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.0.first_cause.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.0.cleanup_error.as_ref()
    }
    pub fn validation_error(&self) -> Option<&CoreError> {
        self.0.validation_error.as_ref()
    }
    pub fn accepted(&self) -> bool {
        self.0.accepted
    }
    pub fn remote_cancellation(&self) -> ConsumerActionCancellation {
        self.0.remote_cancel
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
        if let Some(e) = &self.0.first_cause {
            return Err(e.clone());
        }
        match &self.0.native_result {
            Some(Ok(o)) if self.0.state == ConsumerActionState::Completed => Ok(o.view()),
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
    fn close(&mut self, cause: CoreError) {
        self.control.record(cause);
        self.first_cause = self.control.cause();
        self.retiring = self.exchange.is_some();
        self.state = if self.retiring {
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
        if self.first_cause.is_none()
            && let Err(e) = self.association.check_dependencies()
        {
            self.close(e);
        }
        if let Err(e) = self.control.check(state) {
            self.close(e);
        }
    }
    fn refresh_access(&mut self, state: &State, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.first_cause.is_some() {
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
            .check(state, &self.control, self.continuation.as_deref())
            .and_then(|()| self.association.check_dependencies())
        {
            self.close(e);
        }
        Ok(())
    }
    fn step(&mut self, state: &State, cx: &mut Context<'_>, budget: &mut WorkBudget) {
        self.fence(state);
        if self.state.settlement_ready() {
            let _ = self.refresh_access(state, budget);
            self.fence(state);
            return;
        }
        let selected = self
            .association
            .selected(self.operation)
            .expect("captured operation");
        // All callback and late validation work is paid before native progress.
        if prepay(&selected, budget).is_err() {
            return;
        }
        if !pay(
            budget,
            W::CodecInputBytes,
            self.association.selected(0).unwrap().policy.validation_work,
        ) {
            return;
        }
        if self.first_cause.is_none()
            && let Err(e) = selected.check(state, &self.control, self.continuation.as_deref())
        {
            self.close(e);
        }
        self.fence(state);
        if let Some(raw) = self.exchange.as_mut()
            && !self.retiring
        {
            match raw.0.as_mut().expect("owned exchange").poll(cx) {
                Poll::Pending => {
                    self.fence(state);
                    if self.first_cause.is_none() {
                        return;
                    }
                }
                Poll::Ready(Ok(reply)) => {
                    self.reply = Some(reply);
                    self.retiring = true;
                }
                Poll::Ready(Err(e)) => {
                    // A poll failure is an outcome of its own. Retirement
                    // may still supply the original response; neither can
                    // overwrite the first local cause or erase the other.
                    self.native_error.get_or_insert_with(|| e.clone());
                    self.native_result = Some(Err(e.clone()));
                    self.close(e);
                }
            }
        }
        self.fence(state);
        if let Some(raw) = self.exchange.as_mut() {
            match raw.0.as_mut().expect("owned exchange").poll_close(cx) {
                Poll::Pending => {
                    self.fence(state);
                    return;
                }
                Poll::Ready(Err(e)) => {
                    if self.cleanup_error.is_none() {
                        self.cleanup_error = Some(e);
                    }
                    self.fence(state);
                    return;
                }
                Poll::Ready(Ok(late)) => {
                    if let Some(late) = late
                        && self.reply.is_none()
                    {
                        self.reply = Some(late);
                    }
                    self.exchange = None;
                }
            }
        }
        self.fence(state);
        if let Some(reply) = self.reply.take() {
            self.accept_reply(state, reply);
        }
        self.fence(state);
        if self.first_cause.is_some() {
            self.state = ConsumerActionState::Closed;
        }
        self.retiring = false;
    }
    fn accept_reply(&mut self, state: &State, reply: ConsumerActionReply) {
        // Keep the original output even when validation, cancellation or a
        // replacement prevents it from becoming an application success.
        match reply {
            ConsumerActionReply::Complete(output) if self.operation != 2 => {
                // Store before validator entry: unwind must retain the original
                // evidence, even though arbitrary panicking code need not resume.
                self.native_result = Some(Ok(output));
                let output = self.native_result.as_ref().unwrap().as_ref().unwrap();
                let validation = if raw_action_output_shape(output)
                    && interaction_output_backing(output).is_some_and(|n| {
                        n as u64
                            <= self
                                .association
                                .selected(self.operation)
                                .unwrap()
                                .bounds
                                .response_bytes
                    }) {
                    self.association.validate(
                        self.association.definition.input().schemas.output.as_ref(),
                        output.data(),
                    )
                } else {
                    Err(invalid())
                };
                if let Err(e) = validation {
                    self.validation_error = Some(e.clone());
                    self.close(e);
                }
                if self.operation == 0 {
                    self.accepted = true;
                }
                self.state = ConsumerActionState::Completed;
            }
            ConsumerActionReply::Accepted(token) if self.operation == 0 => {
                self.continuation = Some(token);
                self.accepted = true;
                self.state = ConsumerActionState::Pending;
            }
            ConsumerActionReply::Pending if self.operation == 1 => {
                self.state = ConsumerActionState::Pending
            }
            ConsumerActionReply::CancelRequested if self.operation == 2 => {
                self.remote_cancel = ConsumerActionCancellation::Requested;
                self.state = ConsumerActionState::Pending;
            }
            ConsumerActionReply::AlreadyTerminal if self.operation == 2 => {
                self.remote_cancel = ConsumerActionCancellation::AlreadyTerminal;
                self.state = ConsumerActionState::Pending;
            }
            ConsumerActionReply::Failed(e) => {
                self.native_result = Some(Err(e));
                self.state = ConsumerActionState::Failed;
            }
            _ => self.close(invalid()),
        }
        // Native/validator callbacks can revoke original identity or policy.
        self.fence(state);
        if self.first_cause.is_none() {
            let s = self.association.selected(self.operation).unwrap();
            if let Err(e) = s.check(state, &self.control, self.continuation.as_deref()) {
                // Terminal state must not suppress fencing of this late result.
                self.close(e);
            }
        }
    }
}
fn prepay(s: &Selected<'_>, budget: &mut WorkBudget) -> CoreResult<()> {
    let access = s.access_work.checked_mul(2).ok_or_else(invalid)?;
    if !pay(budget, W::ProviderProbes, access)
        || !pay(budget, W::BindingPolls, s.bounds.poll_work)
        || !pay(budget, W::CleanupItems, 1)
        || !pay(budget, W::CodecInputBytes, s.policy.validation_work)
    {
        return Err(busy());
    }
    Ok(())
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
