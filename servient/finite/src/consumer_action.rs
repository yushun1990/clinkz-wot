//! Continuing invocation authority, independent of each finite native exchange.
//! No TD, native token interpretation, URL synthesis, retry or protocol cases.
use super::{ConsumerObserver, ConsumerOwned, ConsumerTurn};
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{
    artifact::try_box, consumer::*, consumer_action::*, data_schema::PreparedSchema,
    host_accounts::*, *,
};
use clinkz_wot_foundation::{MonotonicInstant, ResourceKind as K};
use core::{
    any::Any,
    cell::{Cell, RefCell},
    cmp::Ordering,
    task::{Context, Poll},
};

use clinkz_wot_core::execution::action::{
    ActionDriver, ActionObservation, ActionProgress, action_output_shape, check_action_handoff,
    retain_action_reply, step_action,
};
pub use clinkz_wot_core::execution::action::{
    ActionTurn, ConsumerActionCancellation, ConsumerActionState,
};
#[derive(Clone)]
pub struct ConsumerActionIdentity(Rc<Identity>);
struct Identity {
    consumer: ConsumerInstanceRef,
    sequence: u64,
    // A passive identity can outlive the handle and result.
    _live: HostCapacityReservation,
}
impl core::fmt::Debug for ConsumerActionIdentity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConsumerActionIdentity")
            .field("consumer", self.consumer())
            .field("sequence", &self.sequence())
            .finish_non_exhaustive()
    }
}
impl PartialEq for ConsumerActionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ConsumerActionIdentity {}
impl ConsumerActionIdentity {
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.0.consumer
    }
    pub fn sequence(&self) -> u64 {
        self.0.sequence
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
    revision: u64,
}
impl CapturedAccess {
    fn check(&self) -> CoreResult<()> {
        if self.owner.qualification_revision() != self.revision
            || self.owner.retained_bytes() != self.owner.backing_bytes()
            || !self
                .owner
                .supports_security(&security::SecurityRequirement::NOSEC)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
struct Selected<'a> {
    op: &'a ConsumerActionOperation,
    access: &'a CapturedAccess,
    data_revision: u64,
}
impl Selected<'_> {
    fn authorize(
        &self,
        state: &State,
        control: &Control,
        destination: &str,
        schema: Option<&PreparedSchema>,
    ) -> CoreResult<()> {
        control.check(state)?;
        self.check_services(schema)?;
        control.check(state)?;
        let result = self.access.owner.authorize_instance(
            control.identity.consumer(),
            self.op.identity,
            destination,
            self.op.operation,
        );
        control.check(state)?;
        self.check_services(schema)?;
        control.check(state)?;
        result
    }
    fn check_services(&self, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        self.op.check_native_dependencies()?;
        self.access.check()?;
        if schema.is_some() {
            self.check_data()?;
        }
        Ok(())
    }
    fn check_data(&self) -> CoreResult<()> {
        self.op.check_data_dependencies()?;
        if self.op.data.qualification_revision() != self.data_revision {
            return Err(invalid());
        }
        Ok(())
    }
}
struct Association {
    consumer: ConsumerInstanceRef,
    definition: Rc<ConsumerActionDefinition>,
    access: [Option<CapturedAccess>; 3],
    data_revisions: [u64; 3],
    bank: Arc<HostResourceAccounts>,
    _live: HostCapacityReservation,
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
        let definition = self.definition.input();
        for (i, op) in self.operations().into_iter().enumerate() {
            if op.is_some() {
                let selected = self.selected(i)?;
                selected.access.check()?;
                if (i == 0 && definition.schemas.input.is_some())
                    || (i != 2 && definition.schemas.output.is_some())
                {
                    selected.check_data()?;
                }
            }
        }
        Ok(())
    }
    /// One operation's original qualification, without importing unused
    /// siblings into an admitted invocation's invalidation scope.
    fn check_operation(&self, operation: usize, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        self.selected(operation)?.check_services(schema)
    }
    fn selected(&self, i: usize) -> CoreResult<Selected<'_>> {
        let op = self.operations()[i].ok_or_else(unsupported)?;
        let access = self.access[i].as_ref().ok_or_else(unsupported)?;
        Ok(Selected {
            op,
            access,
            data_revision: self.data_revisions[i],
        })
    }
    fn validate(
        &self,
        operation: usize,
        schema: Option<&PreparedSchema>,
        payload: Option<PayloadView<'_>>,
    ) -> CoreResult<()> {
        let selected = self.selected(operation)?;
        let op = selected.op;
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(s), Some(p))
                if p.body.len() as u64 <= op.body_bytes
                    && p.content_type == &*op.content_type
                    && p.content_coding.is_none() =>
            {
                selected.check_data()?;
                let validator = op.data.action_validator().ok_or_else(invalid)?;
                let result = validator.validate(s, p);
                selected.check_data()?;
                result
            }
            _ => Err(invalid()),
        }
    }
}
/// Persistent checked association, independent of invocation capacity. Clones
/// are passive non-revoking views; admitted work keeps the original association.
pub struct ConsumerActionAssociation {
    captured: Rc<ConsumerOwned<Association>>,
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
    ) -> Result<Self, BindingInputRejection<(Rc<ConsumerActionDefinition>, ConsumerActionAccess)>>
    {
        let _turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => return Err(reject((definition, access), error)),
        };
        // This is an explicit new policy binding, never a replacement snapshot
        // of prepared native/data declarations.
        let captured = [
            Some(access.invoke.clone()),
            access.query.clone(),
            access.cancel.clone(),
        ]
        .map(|owner| {
            owner.map(|owner| CapturedAccess {
                revision: owner.qualification_revision(),
                owner,
            })
        });
        let d = definition.input();
        let operations = [Some(&d.invoke), d.query.as_ref(), d.cancel.as_ref()];
        let data_revisions =
            operations.map(|op| op.map_or(0, |op| op.data.qualification_revision()));
        let build = (|| {
            definition.check_dependencies()?;
            if !consumer.is_current() {
                return Err(stale());
            }
            for (i, (op, selected)) in operations.into_iter().zip(&captured).enumerate() {
                match (op, selected) {
                    (Some(op), Some(selected)) => {
                        selected.check()?;
                        if i != 2 && op.data.qualification_revision() != data_revisions[i] {
                            return Err(invalid());
                        }
                    }
                    (None, None) => {}
                    _ => return Err(invalid()),
                }
            }
            if !consumer.is_current() {
                return Err(stale());
            }
            claim(
                &bank,
                K::EngineLiveBytesGlobalMax,
                rc_bytes::<ConsumerOwned<Association>>(),
            )
        })();
        let funding = match build {
            Ok(funding) => funding,
            Err(error) => return Err(reject((definition, access), error)),
        };
        Ok(Self {
            captured: Rc::new(ConsumerOwned::new(Association {
                consumer,
                definition,
                access: captured,
                data_revisions,
                bank,
                _live: funding,
            })),
        })
    }
    pub fn consumer(&self) -> &ConsumerInstanceRef {
        &self.captured.consumer
    }
    pub fn definition(&self) -> &Rc<ConsumerActionDefinition> {
        &self.captured.definition
    }
}
/// Counts actual retained owners against the still-shared Host ceilings.
struct RetainedUsage {
    _live: HostCapacityReservation,
    _cleanup: HostCapacityReservation,
}
impl RetainedUsage {
    fn claim(bank: &Arc<HostResourceAccounts>, bytes: u64) -> CoreResult<Self> {
        Ok(Self {
            _live: claim(bank, K::EngineLiveBytesGlobalMax, bytes)?,
            _cleanup: claim(bank, K::CleanupBytesMax, bytes)?,
        })
    }
}
struct ResponseQuota {
    _buffer: HostCapacityReservation,
    _retained: RetainedUsage,
}
struct ExchangeQuotas {
    native: RetainedUsage,
    scratch: HostCapacityReservation,
    scratch_live: HostCapacityReservation,
    response: ResponseQuota,
}
impl ExchangeQuotas {
    fn claim(
        bank: &Arc<HostResourceAccounts>,
        op: &ConsumerActionOperation,
        uses_validator: bool,
    ) -> CoreResult<Self> {
        let bounds = op.bounds;
        let temporary = if uses_validator {
            bounds.temporary_bytes.max(op.validation_scratch_bytes)
        } else {
            bounds.temporary_bytes
        };
        let limit = bank.account(K::BindingPollTemporaryBytesPerCallMax).limit();
        if temporary > limit {
            return Err(limit_error(
                K::BindingPollTemporaryBytesPerCallMax,
                limit,
                temporary,
                None,
            ));
        }
        Ok(Self {
            native: RetainedUsage::claim(bank, bounds.exchange_bytes)?,
            scratch: claim(bank, K::BindingPollTemporaryBytesGlobalMax, temporary)?,
            scratch_live: claim(bank, K::EngineLiveBytesGlobalMax, temporary)?,
            response: ResponseQuota {
                _buffer: claim(
                    bank,
                    K::BindingResponseBufferBytesGlobalMax,
                    bounds.response_bytes,
                )?,
                _retained: RetainedUsage::claim(bank, bounds.response_bytes)?,
            },
        })
    }
}
struct Exchange {
    owner: Option<Box<dyn ConsumerActionExchange>>,
    _usage: RetainedUsage,
}
impl Drop for Exchange {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            owner.abort_local();
        }
    }
}
struct Invocation {
    progress: ActionProgress,
    control: Rc<Control>,
    exchange: Option<Exchange>,
    reply: Option<ConsumerActionReply>,
    continuation: Option<Box<dyn Any>>,
    native_result: Option<CoreResult<ConsumerActionOutput>>,
    native_error: Option<CoreError>,
    input: Option<Payload>,
    invoke_destination: Box<str>,
    control_destination: Option<(Box<str>, RetainedUsage)>,
    association: Rc<ConsumerOwned<Association>>,
    continuation_usage: Option<RetainedUsage>,
    response_quota: Option<ResponseQuota>,
    scratch: Option<(HostCapacityReservation, HostCapacityReservation)>,
    _retained: RetainedUsage,
    _cleanup_item: HostCapacityReservation,
    _outstanding: HostCapacityReservation,
    _instance_outstanding: Option<HostCapacityReservation>,
}
struct State {
    bank: Arc<HostResourceAccounts>,
    active: Cell<bool>,
    controller_present: Cell<bool>,
    registry: RefCell<Registry>,
    _live: HostCapacityReservation,
}
impl Drop for State {
    fn drop(&mut self) {
        let _turn = ConsumerTurn::scope();
        self.registry.get_mut().slots.clear();
    }
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
    observer: ConsumerObserver,
    _funding: HostCapacityReservation,
}
impl Control {
    fn new(
        consumer: &ConsumerInstanceRef,
        sequence: u64,
        bank: &Arc<HostResourceAccounts>,
        deadline: Deadline,
        now: MonotonicInstant,
    ) -> CoreResult<Rc<Self>> {
        let funding = claim(bank, K::EngineLiveBytesGlobalMax, rc_bytes::<Self>())?;
        let identity = ConsumerActionIdentity(Rc::new(Identity {
            consumer: consumer.clone(),
            sequence,
            _live: claim(bank, K::EngineLiveBytesGlobalMax, rc_bytes::<Identity>())?,
        }));
        Ok(Rc::new(Self {
            identity,
            cause: RefCell::new(None),
            deadline,
            last_time: Cell::new(now),
            abandoned: Cell::new(false),
            observer: ConsumerObserver::default(),
            _funding: funding,
        }))
    }
    fn record(&self, cause: CoreError) {
        let cause = if self.identity.consumer().is_current() {
            cause
        } else {
            stale()
        };
        self.cause.borrow_mut().get_or_insert(cause);
    }
    fn cause(&self) -> Option<CoreError> {
        self.cause.borrow().clone()
    }
    fn observe_time(&self, now: MonotonicInstant) {
        match now.checked_cmp(self.last_time.get()) {
            Some(Ordering::Equal | Ordering::Greater) => {
                self.last_time.set(now);
                if let Err(error) = check_time(self.deadline, now) {
                    self.record(error);
                }
            }
            _ => self.record(invalid()),
        }
    }
    fn check(&self, state: &State) -> CoreResult<()> {
        if !state.active.get() || !self.identity.consumer().is_current() {
            self.record(stale());
        }
        self.cause().map_or(Ok(()), Err)
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
    fn reserve_slot<'a>(&'a self, _turn: &'a mut ConsumerTurn) -> CoreResult<Lease<'a>> {
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
    fn checkout<'a>(
        &'a self,
        _turn: &'a mut ConsumerTurn,
        slot: usize,
        original: Option<u64>,
    ) -> Option<Lease<'a>> {
        let mut registry = self.registry.borrow_mut();
        let entry = &mut registry.slots[slot];
        let Slot::Ready(invocation) = entry else {
            return None;
        };
        let sequence = invocation.control.identity.sequence();
        if original.is_some_and(|expected| expected != sequence) {
            return None;
        }
        let reserved = Slot::Reserved {
            sequence,
            control: Some(invocation.control.clone()),
        };
        let Slot::Ready(invocation) = core::mem::replace(entry, reserved) else {
            unreachable!()
        };
        Some(Lease {
            state: self,
            slot,
            sequence,
            invocation: Some(invocation),
        })
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
        let _turn = ConsumerTurn::scope();
        self.revoke();
        let count = self.registry.borrow().slots.len();
        for i in 0..count {
            let control = self.registry.borrow().slots[i].control().cloned();
            if let Some(control) = control {
                control.observer.wake();
            }
        }
    }
    fn abort_slot(&self, index: usize, sequence: Option<u64>) {
        if self.controller_present.get() {
            return;
        }
        let removed = {
            let mut registry = self.registry.borrow_mut();
            match &registry.slots[index] {
                Slot::Ready(work)
                    if work.control.abandoned.get()
                        && sequence.is_none_or(|s| s == work.control.identity.sequence()) =>
                {
                    Some(core::mem::replace(&mut registry.slots[index], Slot::Vacant))
                }
                _ => None,
            }
        };
        drop(removed);
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
        let abandoned = if !self.state.controller_present.get()
            && self
                .invocation
                .as_ref()
                .is_some_and(|work| work.control.abandoned.get())
        {
            self.invocation.take()
        } else {
            None
        };
        let replacement = self.invocation.take().map_or(Slot::Vacant, Slot::Ready);
        let old = {
            let mut r = self.state.registry.borrow_mut();
            core::mem::replace(&mut r.slots[self.slot], replacement)
        };
        let Slot::Reserved { sequence, .. } = old else {
            unreachable!()
        };
        debug_assert_eq!(sequence, self.sequence);
        drop(abandoned);
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
        let bytes = rc_bytes::<State>() + (invocations * core::mem::size_of::<Slot>()) as u64;
        let funding = claim(&bank, K::EngineLiveBytesGlobalMax, bytes)?;
        let mut slots = Vec::new();
        slots.try_reserve_exact(invocations).map_err(|_| busy())?;
        slots.resize_with(invocations, || Slot::Vacant);
        Ok(Self {
            state: Rc::new(State {
                bank,
                active: Cell::new(true),
                controller_present: Cell::new(true),
                registry: RefCell::new(Registry {
                    slots,
                    sequence: 0,
                    next: 0,
                }),
                _live: funding,
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
    ) -> Result<ConsumerActionHandle, BindingInputRejection<Option<Payload>>> {
        let mut turn = match ConsumerTurn::enter() {
            Ok(turn) => turn,
            Err(error) => return Err(reject(input, error)),
        };
        let state = &self.state;
        let mut input = input;
        let checked = (|| {
            let d = association.captured.clone();
            if !state.active.get() || !d.consumer.is_current() {
                return Err(stale());
            }
            check_time(deadline, now)?;
            let mut reserved = state.reserve_slot(&mut turn)?;
            let control = Control::new(&d.consumer, reserved.sequence, &state.bank, deadline, now)?;
            reserved.attach(control.clone());
            let invoke = d.selected(0)?;
            let schemas = &d.definition.input().schemas;
            let quotas = ExchangeQuotas::claim(
                &state.bank,
                invoke.op,
                schemas.input.is_some() || schemas.output.is_some(),
            )?;
            let input_bytes =
                clinkz_wot_core::notification::notification_payload_backing(input.as_ref())
                    .ok_or_else(invalid)? as u64;
            let retained = input_bytes
                .checked_add(core::mem::size_of::<Invocation>() as u64)
                .and_then(|n| n.checked_add(MAX_DESTINATION_BYTES as u64))
                .ok_or_else(invalid)?;
            let mut invocation = try_box(Invocation {
                progress: ActionProgress::default(),
                control: control.clone(),
                association: d.clone(),
                continuation: None,
                exchange: None,
                reply: None,
                native_result: None,
                native_error: None,
                input: None,
                invoke_destination: "".into(),
                control_destination: None,
                continuation_usage: Some(RetainedUsage::claim(
                    &state.bank,
                    invoke.op.bounds.continuation_bytes,
                )?),
                response_quota: None,
                scratch: None,
                _retained: RetainedUsage::claim(&state.bank, retained)?,
                _cleanup_item: claim(&state.bank, K::CleanupItemsMax, 1)?,
                _outstanding: claim(&state.bank, K::PendingHandlerCallsGlobalMax, 1)?,
                _instance_outstanding: if Arc::ptr_eq(&d.bank, &state.bank) {
                    None
                } else {
                    Some(claim(&d.bank, K::PendingHandlerCallsGlobalMax, 1)?)
                },
            })
            .map_err(|_| busy())?;
            d.check_dependencies()?;
            let schema = schemas.input.as_ref();
            let destination = resolve_destination(invoke.op, None)?;
            invoke.authorize(state, &control, &destination, schema)?;
            d.validate(0, schema, input.as_ref().map(Payload::view))?;
            invoke.authorize(state, &control, &destination, schema)?;
            d.check_dependencies()?;
            control.check(state)?;
            let raw = match invoke.op.mapping.start(&destination, input.as_ref(), None) {
                Ok(raw) => raw,
                Err(error) => {
                    control.check(state)?;
                    return Err(error);
                }
            };
            invocation.invoke_destination = destination;
            invocation.start_exchange(raw, quotas);
            let slot = reserved.slot;
            let sequence = reserved.sequence;
            let control = invocation.control.clone();
            invocation.input = input.take();
            // A successful start may already own native effects, even if its
            // callback closed this invocation. Retain the complete exchange.
            reserved.invocation = Some(invocation);
            reserved.invocation.as_mut().unwrap().fence(state);
            Ok(ConsumerActionHandle {
                state: self.state.clone(),
                slot,
                sequence,
                control,
            })
        })();
        checked.map_err(|error| reject(input, error))
    }
    pub fn begin_close(&self) {
        self.state.close();
    }
    /// Observe original deadlines and give one rotating slot a bounded turn.
    pub fn step(&self, cx: &mut Context<'_>, now: MonotonicInstant) -> CoreResult<ActionTurn> {
        let mut turn = ConsumerTurn::enter()?;
        let index = {
            let mut registry = self.state.registry.borrow_mut();
            for control in registry.slots.iter().filter_map(Slot::control) {
                control.observe_time(now);
            }
            let index = registry.next;
            registry.next = (index + 1) % registry.slots.len();
            index
        };
        let Some(mut lease) = self.state.checkout(&mut turn, index, None) else {
            return Ok(ActionTurn::Idle);
        };
        let invocation = lease.invocation.as_mut().unwrap();
        let progress = invocation.step(&self.state, cx);
        if invocation.control.abandoned.get() && invocation.progress.state.settlement_ready() {
            drop(lease.take());
        }
        Ok(progress)
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
        let _turn = ConsumerTurn::scope();
        self.state.controller_present.set(false);
        self.state.revoke();
        let count = self.state.registry.borrow().slots.len();
        for index in 0..count {
            self.state.abort_slot(index, None);
        }
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
    fn checkout<'a>(
        &'a self,
        turn: &'a mut ConsumerTurn,
        now: MonotonicInstant,
    ) -> CoreResult<Lease<'a>> {
        let lease = self
            .state
            .checkout(turn, self.slot, Some(self.sequence))
            .ok_or_else(stale)?;
        self.control.observe_time(now);
        Ok(lease)
    }
    pub fn identity(&self) -> CoreResult<ConsumerActionIdentity> {
        self.live()?;
        Ok(self.control.identity.clone())
    }
    fn get<'a>(&self, r: &'a Registry) -> CoreResult<&'a Invocation> {
        match &r.slots[self.slot] {
            Slot::Ready(v) if v.control.identity.sequence() == self.sequence => Ok(v),
            Slot::Reserved { sequence, .. } if *sequence == self.sequence => Err(busy()),
            _ => Err(stale()),
        }
    }
    /// Inspection of native state while its owner is executing returns Backpressure.
    pub fn state(&self) -> CoreResult<ConsumerActionState> {
        let r = self.state.registry.borrow();
        let v = self.get(&r)?;
        Ok(if self.control.cause().is_some() {
            if v.progress.state.settlement_ready() {
                ConsumerActionState::Closed
            } else {
                ConsumerActionState::Closing
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
    /// Mapped control requires an independent application turn.
    pub fn query(&self, now: MonotonicInstant) -> CoreResult<()> {
        self.control(Operation::QueryAction, now)
    }
    pub fn request_cancel(&self, now: MonotonicInstant) -> CoreResult<()> {
        self.control(Operation::CancelAction, now)
    }
    fn control(&self, operation: Operation, now: MonotonicInstant) -> CoreResult<()> {
        let mut turn = ConsumerTurn::enter()?;
        let mut lease = self.checkout(&mut turn, now)?;
        let invocation = lease.invocation.as_mut().unwrap();
        invocation.fence(&self.state);
        self.control.check(&self.state)?;
        if !invocation.progress.can_control() {
            return Err(busy());
        }
        let index = operation_index(operation);
        let association = invocation.association.clone();
        let selected = association.selected(index)?;
        let schema = if operation == Operation::QueryAction {
            association.definition.input().schemas.output.as_ref()
        } else {
            None
        };
        let prepared = (|| {
            let quotas = ExchangeQuotas::claim(&self.state.bank, selected.op, schema.is_some())?;
            let destination_usage =
                RetainedUsage::claim(&self.state.bank, MAX_DESTINATION_BYTES as u64)?;
            selected.check_services(schema)?;
            let destination = resolve_destination(selected.op, invocation.continuation.as_deref())?;
            selected.authorize(&self.state, &self.control, &destination, schema)?;
            Ok((destination, destination_usage, quotas))
        })();
        // Refusing a control leaves the original invocation available, while
        // a callback-time loss of its authority still fences that invocation.
        invocation.fence(&self.state);
        self.control.check(&self.state)?;
        let (destination, destination_usage, quotas) = prepared?;
        match selected
            .op
            .mapping
            .start(&destination, None, invocation.continuation.as_deref())
        {
            Ok(raw) => {
                invocation.control_destination = Some((destination, destination_usage));
                invocation.start_exchange(raw, quotas);
                invocation.progress.start_control(operation);
                invocation.fence(&self.state);
                Ok(())
            }
            Err(error) => {
                invocation.fence(&self.state);
                self.control.check(&self.state)?;
                Err(error)
            }
        }
    }
    /// Stops local work only. Does not send a hidden CancelAction or claim that
    /// a remote effect was undone. First cause and any late result are retained.
    pub fn cancel_local(&self) {
        let _turn = ConsumerTurn::scope();
        if self.live().is_ok() {
            self.control.record(CoreError::Cancelled(context()));
            self.control.observer.wake();
        }
    }
    /// Drive the original invocation. A callback attempting another drive
    /// receives a lifecycle error before it can change the clock or waiter.
    pub fn progress(&self, cx: &mut Context<'_>, now: MonotonicInstant) -> CoreResult<ActionTurn> {
        let mut turn = ConsumerTurn::enter()?;
        let mut lease = self.checkout(&mut turn, now)?;
        Ok(lease.invocation.as_mut().unwrap().step(&self.state, cx))
    }
    /// Transfer the original result only after acknowledged native retirement
    /// and a fresh check of the returning operation's authority.
    pub fn take_settlement(&self, now: MonotonicInstant) -> CoreResult<ConsumerActionSettlement> {
        let mut turn = ConsumerTurn::enter()?;
        let mut lease = self.checkout(&mut turn, now)?;
        let invocation = lease.invocation.as_mut().unwrap();
        if !invocation.progress.state.settlement_ready() {
            return Err(busy());
        }
        check_action_handoff(&mut HostActionDriver {
            invocation,
            state: &self.state,
        });
        Ok(ConsumerActionSettlement {
            invocation: Some(lease.take()),
        })
    }
}
impl Drop for ConsumerActionHandle {
    fn drop(&mut self) {
        let _turn = ConsumerTurn::scope();
        if self.live().is_ok() {
            self.control.abandoned.set(true);
            self.control.record(CoreError::Cancelled(context()));
            self.state.abort_slot(self.slot, Some(self.sequence));
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
pub struct ConsumerActionSettlement {
    invocation: Option<Box<Invocation>>,
}
impl Drop for ConsumerActionSettlement {
    fn drop(&mut self) {
        let _turn = ConsumerTurn::scope();
        drop(self.invocation.take());
    }
}
impl ConsumerActionSettlement {
    fn invocation(&self) -> &Invocation {
        self.invocation
            .as_deref()
            .expect("settlement retains its invocation until disposal")
    }
    pub fn input(&self) -> Option<PayloadView<'_>> {
        self.invocation().input.as_ref().map(Payload::view)
    }
    pub fn identity(&self) -> &ConsumerActionIdentity {
        &self.invocation().control.identity
    }
    pub fn effect(&self) -> ConsumerEffect {
        ConsumerEffect::Unknown
    }
    pub fn state(&self) -> ConsumerActionState {
        self.invocation().progress.state
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.invocation().progress.first_cause.as_ref()
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.invocation().progress.cleanup_error.as_ref()
    }
    pub fn validation_error(&self) -> Option<&CoreError> {
        self.invocation().progress.validation_error.as_ref()
    }
    pub fn accepted(&self) -> bool {
        self.invocation().progress.accepted
    }
    pub fn remote_cancellation(&self) -> ConsumerActionCancellation {
        self.invocation().progress.remote_cancel
    }
    pub fn native_result(&self) -> Option<Result<InteractionOutputView<'_>, &CoreError>> {
        self.invocation()
            .native_result
            .as_ref()
            .map(|r| r.as_ref().map(ConsumerActionOutput::view))
    }
    /// Original poll failure, independent of a subsequently retained late output
    /// and of an earlier local cancellation or cleanup failure.
    pub fn native_error(&self) -> Option<&CoreError> {
        self.invocation().native_error.as_ref()
    }
    pub fn result(&self) -> CoreResult<InteractionOutputView<'_>> {
        if let Some(e) = &self.invocation().progress.first_cause {
            return Err(e.clone());
        }
        match &self.invocation().native_result {
            Some(Ok(o)) if self.invocation().progress.state == ConsumerActionState::Completed => {
                Ok(o.view())
            }
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
        self.invocation().association.operations()[i].map(|s| (s.identity, s.form))
    }
}
impl Invocation {
    fn store_reply(&mut self, reply: ConsumerActionReply) -> CoreResult<()> {
        if let ConsumerActionReply::Complete(output) = &reply {
            let selected = self
                .association
                .selected(operation_index(self.progress.operation()))?;
            if output
                .backing_bytes()
                .is_none_or(|n| n as u64 > selected.op.bounds.response_bytes)
            {
                let error = invalid();
                self.progress.validation_error = Some(error.clone());
                return Err(error);
            }
        }
        self.reply = Some(reply);
        Ok(())
    }
    fn start_exchange(&mut self, owner: Box<dyn ConsumerActionExchange>, quotas: ExchangeQuotas) {
        self.exchange = Some(Exchange {
            owner: Some(owner),
            _usage: quotas.native,
        });
        self.response_quota = Some(quotas.response);
        self.scratch = Some((quotas.scratch, quotas.scratch_live));
    }
    fn live_output_schema(&self) -> Option<&PreparedSchema> {
        if self.progress.operation() != Operation::CancelAction
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
        let operation = operation_index(self.progress.operation());
        if operation != 0 {
            self.association.check_operation(0, None)?;
        }
        self.association
            .check_operation(operation, self.live_output_schema())
    }
    fn close(&mut self, cause: CoreError) {
        self.control.record(cause);
        self.progress
            .close(self.control.cause().expect("recorded Action cause"));
    }
    fn fence_local(&mut self, state: &State) {
        if let Err(error) = self.control.check(state) {
            self.close(error);
        }
    }
    fn fence(&mut self, state: &State) {
        self.fence_local(state);
        if self.progress.first_cause.is_none()
            && let Err(error) = self.check_dependencies()
        {
            self.close(error);
        }
        self.fence_local(state);
    }
    fn step(&mut self, state: &State, cx: &mut Context<'_>) -> ActionTurn {
        self.control.observer.register(cx.waker());
        step_action(
            &mut HostActionDriver {
                invocation: self,
                state,
            },
            cx,
        )
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
    fn fence(&mut self) {
        self.invocation.fence(self.state);
    }
    fn fence_local(&mut self) {
        self.invocation.fence_local(self.state);
    }
    fn close(&mut self, cause: CoreError) {
        self.invocation.close(cause);
    }
    fn invalid_reply(&self) -> CoreError {
        invalid()
    }
    fn check_current(&mut self) -> CoreResult<()> {
        let invocation = &self.invocation;
        let operation = operation_index(invocation.progress.operation());
        invocation.association.selected(operation)?.authorize(
            self.state,
            &invocation.control,
            invocation
                .control_destination
                .as_ref()
                .map_or(&*invocation.invoke_destination, |(destination, _)| {
                    destination
                }),
            invocation.live_output_schema(),
        )
    }
    fn poll_exchange(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let invocation = &mut self.invocation;
        invocation
            .exchange
            .as_mut()
            .unwrap()
            .owner
            .as_mut()
            .unwrap()
            .poll(cx)
            .map(|result| match result {
                Ok(reply) => invocation.store_reply(reply),
                Err(error) => {
                    invocation.native_error = Some(error.clone());
                    invocation.native_result = Some(Err(error.clone()));
                    Err(error)
                }
            })
    }
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let invocation = &mut self.invocation;
        let Some(exchange) = invocation.exchange.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        exchange
            .owner
            .as_mut()
            .unwrap()
            .poll_close(cx)
            .map(|result| {
                result.map(|late| {
                    if invocation.reply.is_none()
                        && let Some(late) = late
                        && let Err(error) = invocation.store_reply(late)
                    {
                        invocation.close(error);
                    }
                    let mut retired = invocation.exchange.take().unwrap();
                    drop(retired.owner.take());
                })
            })
    }
    fn retain_reply(&mut self) -> Option<ActionObservation> {
        let invocation = &mut self.invocation;
        let reply = invocation.reply.take()?;
        let operation = invocation.progress.operation();
        Some(retain_action_reply(
            operation,
            reply,
            &mut invocation.continuation,
            &mut invocation.native_result,
        ))
    }
    fn validate_output(&mut self) -> CoreResult<()> {
        let invocation = &self.invocation;
        let output = invocation
            .native_result
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .view();
        if !action_output_shape(output) {
            return Err(invalid());
        }
        invocation.association.validate(
            operation_index(invocation.progress.operation()),
            invocation
                .association
                .definition
                .input()
                .schemas
                .output
                .as_ref(),
            output.data(),
        )
    }
    fn finish_operation(&mut self) {
        let invocation = &mut self.invocation;
        invocation.scratch = None;
        if !matches!(
            &invocation.native_result,
            Some(Ok(output)) if output.view().data().is_some()
        ) {
            invocation.response_quota = None;
        }
        if invocation.progress.state.settlement_ready() {
            drop((
                invocation.continuation.take(),
                invocation.continuation_usage.take(),
            ));
        } else {
            invocation.control_destination = None;
        }
    }
}

const MAX_DESTINATION_BYTES: usize = 256;
fn resolve_destination(
    op: &ConsumerActionOperation,
    continuation: Option<&dyn Any>,
) -> CoreResult<Box<str>> {
    let destination = op.mapping.destination(continuation)?;
    if destination.is_empty() || destination.len() > MAX_DESTINATION_BYTES {
        return Err(invalid());
    }
    Ok(destination)
}
fn operation_index(operation: Operation) -> usize {
    match operation {
        Operation::InvokeAction => 0,
        Operation::QueryAction => 1,
        Operation::CancelAction => 2,
        _ => unreachable!("admitted Action operation"),
    }
}
fn rc_bytes<T>() -> u64 {
    (core::mem::size_of::<T>() + 2 * core::mem::size_of::<usize>()) as u64
}
fn claim(
    bank: &Arc<HostResourceAccounts>,
    kind: K,
    bytes: u64,
) -> CoreResult<HostCapacityReservation> {
    bank.reserve(kind, bytes).ok_or_else(|| {
        let account = bank.account(kind);
        limit_error(kind, account.limit(), bytes, Some(account.used()))
    })
}
fn limit_error(resource: K, limit: u64, requested: u64, observed: Option<u64>) -> CoreError {
    CoreError::LimitExceeded {
        resource,
        limit,
        requested: Some(requested),
        observed,
        context: ErrorContext::new(
            ErrorPhase::Admission,
            if requested > limit {
                RetryClass::Never
            } else {
                RetryClass::Safe
            },
        ),
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
