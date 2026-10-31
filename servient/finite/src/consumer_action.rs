//! Continuing invocation authority, independent of each finite native exchange.
//! No TD, native token interpretation, URL synthesis, retry or protocol cases.
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};
use clinkz_wot_core::{
    action::raw_action_output_shape, consumer::*, consumer_action::*, host_accounts::*, *,
};
use clinkz_wot_foundation::{MonotonicInstant, ResourceKind as K, WorkBudget, WorkClass as W};
use core::{
    any::Any,
    cell::RefCell,
    cmp::Ordering,
    task::{Context, Poll},
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
struct Selected {
    op: ConsumerActionOperation,
    policy: ConsumerDataPolicy,
    access_work: u64,
    bounds: ConsumerActionBounds,
    identity: BindingRegistrationIdentity,
}
impl Selected {
    fn new(op: ConsumerActionOperation) -> Self {
        Self {
            policy: op.services.data.policy(),
            access_work: op.services.access.work(),
            bounds: op.mapping.bounds(),
            identity: op.mapping.identity(),
            op,
        }
    }
    fn check(
        &self,
        consumer: &ConsumerInstanceRef,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<()> {
        if !consumer.is_current() {
            return Err(stale());
        }
        self.check_services()?;
        let destination = self.op.mapping.destination(continuation)?;
        self.op.services.access.authorize_instance(
            consumer,
            self.identity,
            &destination,
            self.op.mapping.operation(),
        )?;
        if !consumer.is_current() {
            return Err(stale());
        }
        self.check_services()
    }
    fn check_services(&self) -> CoreResult<()> {
        if self.op.services.data.policy() != self.policy
            || !self.op.services.data.available()
            || self.op.services.access.work() != self.access_work
            || self.op.mapping.bounds() != self.bounds
            || self.op.mapping.identity() != self.identity
        {
            return Err(invalid());
        }
        Ok(())
    }
}
struct Definition {
    target: AffordanceTarget,
    schemas: ConsumerSchemas,
    selected: [Option<Selected>; 3],
    bank: Arc<HostResourceAccounts>,
    _funding: Vec<HostCapacityReservation>,
}
impl Definition {
    fn data_current(&self) -> CoreResult<()> {
        let s = self.selected(0)?;
        if !s.op.services.data.available()
            || s.op.services.data.policy() != s.policy
            || !self
                .schemas
                .input
                .into_iter()
                .chain(self.schemas.output)
                .all(|schema| s.op.services.data.supports_schema(schema))
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn from_prepared(d: ConsumerActionDefinition) -> Self {
        let d = d.into_input();
        Self {
            target: d.target,
            schemas: d.schemas,
            selected: [
                Some(Selected::new(d.invoke)),
                d.query.map(Selected::new),
                d.cancel.map(Selected::new),
            ],
            bank: d.bank,
            _funding: d.funding,
        }
    }
    fn selected(&self, i: usize) -> CoreResult<&Selected> {
        self.selected[i].as_ref().ok_or_else(unsupported)
    }
    fn validate(
        &self,
        schema: Option<clinkz_wot_core::value_type::TypeSchema>,
        payload: Option<&Payload>,
    ) -> CoreResult<()> {
        let selected = self.selected(0)?;
        if !selected.op.services.data.available()
            || selected.op.services.data.policy() != selected.policy
        {
            return Err(invalid());
        }
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(s), Some(p))
                if p.body.len() as u64 <= selected.policy.body_bytes
                    && selected.op.services.data.supports_schema(s) =>
            {
                selected.op.services.data.validate(s, p)?;
                if !selected.op.services.data.available()
                    || selected.op.services.data.policy() != selected.policy
                    || !selected.op.services.data.supports_schema(s)
                {
                    return Err(invalid());
                }
                Ok(())
            }
            _ => Err(invalid()),
        }
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
    identity: ConsumerActionIdentity,
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
    abandoned: bool,
    deadline: Deadline,
    last_time: MonotonicInstant,
    definition: Rc<Definition>,
    _funding: Vec<HostCapacityReservation>,
}
struct State {
    consumer: ConsumerInstanceRef,
    definitions: Vec<Rc<Definition>>,
    slots: Vec<Option<Invocation>>,
    active: bool,
    sequence: u64,
    next: usize,
    _funding: Vec<HostCapacityReservation>,
}
/// Reusable definitions on one independent Consumer incarnation. Handles keep
/// original work alive; dropping this root fences new work and local acceptance.
pub struct ConsumerActionRuntime {
    state: Rc<RefCell<State>>,
}
impl ConsumerActionRuntime {
    pub fn try_new(
        consumer: ConsumerInstanceRef,
        definitions: Vec<ConsumerActionDefinition>,
        invocations: usize,
    ) -> Result<Self, BindingInputRejection<Vec<ConsumerActionDefinition>>> {
        let build = (|| {
            if !consumer.is_current()
                || definitions.is_empty()
                || definitions.len() > 32
                || !(1..=32).contains(&invocations)
                || definitions.iter().enumerate().any(|(i, d)| {
                    definitions[..i]
                        .iter()
                        .any(|old| old.input().target == d.input().target)
                })
            {
                return Err(invalid());
            }
            let bytes = (core::mem::size_of::<State>()
                + invocations * core::mem::size_of::<Option<Invocation>>()
                + definitions.len() * (core::mem::size_of::<Definition>() + 64))
                as u64;
            // All definitions of this facet share the original accounting bank.
            let bank = &definitions[0].input().bank;
            if definitions
                .iter()
                .any(|d| !Arc::ptr_eq(bank, &d.input().bank))
            {
                return Err(invalid());
            }
            let funding = reserve(bank, &[(K::EngineLiveBytesGlobalMax, bytes)])?;
            let mut slots = Vec::new();
            slots.try_reserve_exact(invocations).map_err(|_| busy())?;
            slots.resize_with(invocations, || None);
            let mut entries = Vec::new();
            entries
                .try_reserve_exact(definitions.len())
                .map_err(|_| busy())?;
            Ok((slots, entries, funding))
        })();
        let (slots, mut entries, funding) = match build {
            Ok(v) => v,
            Err(e) => return Err(reject(definitions, e)),
        };
        entries.extend(
            definitions
                .into_iter()
                .map(|d| Rc::new(Definition::from_prepared(d))),
        );
        Ok(Self {
            state: Rc::new(RefCell::new(State {
                consumer,
                definitions: entries,
                slots,
                active: true,
                sequence: 0,
                next: 0,
                _funding: funding,
            })),
        })
    }
    /// No input or native opportunity is lost on pre-effect refusal. One
    /// accepted invocation is never replayed by progress, query or cancellation.
    pub fn invoke_action(
        &self,
        name: &str,
        input: Option<Payload>,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ConsumerActionHandle, BindingInputRejection<Option<Payload>>> {
        let mut state = self.state.borrow_mut();
        let checked = (|| {
            if !state.active || !state.consumer.is_current() {
                return Err(stale());
            }
            if name.len() > 256 {
                return Err(invalid());
            }
            if !pay(
                budget,
                W::PlanningItems,
                (state.definitions.len() * (name.len() + 1)) as u64,
            ) {
                return Err(busy());
            }
            let d = state
                .definitions
                .iter()
                .find(|d| d.target.name() == Some(name))
                .ok_or_else(unsupported)?
                .clone();
            check_time(deadline, now)?;
            let slot = state
                .slots
                .iter()
                .position(Option::is_none)
                .ok_or_else(busy)?;
            let sequence = state.sequence.checked_add(1).ok_or_else(busy)?;
            let invoke = d.selected(0)?;
            let mut native = 0;
            let mut temporary = invoke.policy.temporary_bytes;
            let mut continuation = 0;
            let mut response = 0;
            for s in d.selected.iter().flatten() {
                native = native.max(s.bounds.exchange_bytes);
                temporary = temporary.max(s.bounds.temporary_bytes);
                continuation = continuation.max(s.bounds.continuation_bytes);
                response = response.max(s.bounds.response_bytes);
            }
            if temporary
                > d.bank
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
            let funding = reserve(
                &d.bank,
                &[
                    (
                        K::EngineLiveBytesGlobalMax,
                        lifetime.checked_add(temporary).ok_or_else(invalid)?,
                    ),
                    (K::BindingPollTemporaryBytesGlobalMax, temporary),
                    (K::CleanupBytesMax, lifetime),
                    (K::CleanupItemsMax, 1),
                    (K::PendingHandlerCallsGlobalMax, 1),
                    (K::BindingResponseBufferBytesGlobalMax, response),
                ],
            )?;
            prepay(invoke, budget)?;
            invoke.check(&state.consumer, None)?;
            d.validate(d.schemas.input, input.as_ref())?;
            invoke.check(&state.consumer, None)?;
            let raw = invoke.op.mapping.start(input.as_ref(), None)?;
            Ok((slot, sequence, d, funding, raw))
        })();
        let (slot, sequence, definition, funding, raw) = match checked {
            Ok(v) => v,
            Err(e) => return Err(reject(input, e)),
        };
        state.sequence = sequence;
        state.slots[slot] = Some(Invocation {
            identity: ConsumerActionIdentity {
                consumer: state.consumer.clone(),
                sequence,
                authority: Rc::new(()),
            },
            definition,
            continuation: None,
            exchange: Some(Exchange(Some(raw))),
            reply: None,
            native_result: None,
            native_error: None,
            input,
            state: ConsumerActionState::Invoking,
            operation: 0,
            retiring: false,
            first_cause: None,
            cleanup_error: None,
            validation_error: None,
            accepted: false,
            effect: ConsumerEffect::Unknown,
            remote_cancel: ConsumerActionCancellation::NotRequested,
            abandoned: false,
            deadline,
            last_time: now,
            _funding: funding,
        });
        Ok(ConsumerActionHandle {
            state: self.state.clone(),
            slot,
            sequence,
        })
    }
    pub fn begin_close(&self) {
        self.state.borrow_mut().active = false;
    }
    pub fn step(&self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        let mut s = self.state.borrow_mut();
        let active = s.active;
        let count = s.slots.len();
        let start = s.next;
        s.next = (start + 1) % count;
        for i in 0..count {
            let slot = &mut s.slots[(start + i) % count];
            if let Some(v) = slot {
                v.step(active, cx, now, budget);
                if v.abandoned && v.state.settlement_ready() {
                    *slot = None;
                }
            }
        }
    }
    pub fn is_idle(&self) -> bool {
        self.state.borrow().slots.iter().all(Option::is_none)
    }
}
impl Drop for ConsumerActionRuntime {
    fn drop(&mut self) {
        self.state.borrow_mut().active = false;
    }
}

/// Unique local authority over an original invocation. No raw token or caller-
/// supplied operation mapping can address/replace this handle's work.
pub struct ConsumerActionHandle {
    state: Rc<RefCell<State>>,
    slot: usize,
    sequence: u64,
}
impl ConsumerActionHandle {
    pub fn identity(&self) -> CoreResult<ConsumerActionIdentity> {
        Ok(self.get(&self.state.borrow())?.identity.clone())
    }
    fn get<'a>(&self, s: &'a State) -> CoreResult<&'a Invocation> {
        s.slots[self.slot]
            .as_ref()
            .filter(|v| v.identity.sequence == self.sequence)
            .ok_or_else(stale)
    }
    pub fn state(&self) -> CoreResult<ConsumerActionState> {
        Ok(self.get(&self.state.borrow())?.state)
    }
    pub fn first_cause(&self) -> CoreResult<Option<CoreError>> {
        Ok(self.get(&self.state.borrow())?.first_cause.clone())
    }
    pub fn cleanup_error(&self) -> CoreResult<Option<CoreError>> {
        Ok(self.get(&self.state.borrow())?.cleanup_error.clone())
    }
    pub fn remote_cancellation(&self) -> CoreResult<ConsumerActionCancellation> {
        Ok(self.get(&self.state.borrow())?.remote_cancel)
    }
    /// Explicit described control; neither call automatically queries/reinvokes.
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
        let mut s = self.state.borrow_mut();
        if !s.active {
            return Err(stale());
        }
        let v = s.slots[self.slot]
            .as_mut()
            .filter(|v| v.identity.sequence == self.sequence)
            .ok_or_else(stale)?;
        if !v.identity.consumer.is_current() || v.first_cause.is_some() {
            return Err(stale());
        }
        if v.state != ConsumerActionState::Pending || v.exchange.is_some() {
            return Err(busy());
        }
        v.time(now)?;
        v.definition.data_current()?;
        let selected = v.definition.selected(operation)?;
        prepay(selected, budget)?;
        let continuation = v.continuation.as_deref().ok_or_else(invalid)?;
        selected.check(&v.identity.consumer, Some(continuation))?;
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
        Ok(())
    }
    /// Stops local work only. Does not send a hidden CancelAction or claim that
    /// a remote effect was undone. First cause and any late result are retained.
    pub fn cancel_local(&self) {
        let mut s = self.state.borrow_mut();
        if let Some(v) = s.slots[self.slot]
            .as_mut()
            .filter(|v| v.identity.sequence == self.sequence)
        {
            v.close(CoreError::Cancelled(context()));
        }
    }
    pub fn progress(
        &self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerActionState> {
        let mut s = self.state.borrow_mut();
        let active = s.active;
        let v = s.slots[self.slot]
            .as_mut()
            .filter(|v| v.identity.sequence == self.sequence)
            .ok_or_else(stale)?;
        v.step(active, cx, now, budget);
        Ok(v.state)
    }
    /// A completed native exchange leaves a revocable unread result. Successful
    /// handoff requires current time/authority/services and a paid fresh access
    /// check. Short credit retains the complete original result and slot. A
    /// fenced settlement retains native evidence without granting success.
    pub fn take_settlement(
        &self,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ConsumerActionSettlement> {
        let mut s = self.state.borrow_mut();
        let active = s.active;
        let v = s.slots[self.slot]
            .as_mut()
            .filter(|v| v.identity.sequence == self.sequence)
            .ok_or_else(stale)?;
        v.fence(active, now);
        if !v.state.settlement_ready() || !pay(budget, W::CleanupItems, 1) {
            return Err(busy());
        }
        v.refresh_access(budget)?;
        Ok(ConsumerActionSettlement(s.slots[self.slot].take().unwrap()))
    }
}
impl Drop for ConsumerActionHandle {
    fn drop(&mut self) {
        let mut s = self.state.borrow_mut();
        if let Some(v) = s.slots[self.slot]
            .as_mut()
            .filter(|v| v.identity.sequence == self.sequence)
        {
            v.abandoned = true;
            v.close(CoreError::Cancelled(context()));
        }
    }
}
pub struct ConsumerActionSettlement(Invocation);
impl ConsumerActionSettlement {
    pub fn input(&self) -> Option<&Payload> {
        self.0.input.as_ref()
    }
    pub fn identity(&self) -> &ConsumerActionIdentity {
        &self.0.identity
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
    pub fn native_result(&self) -> Option<&CoreResult<InteractionOutput>> {
        self.0.native_result.as_ref()
    }
    /// Original poll failure, independent of a subsequently retained late output
    /// and of an earlier local cancellation or cleanup failure.
    pub fn native_error(&self) -> Option<&CoreError> {
        self.0.native_error.as_ref()
    }
    pub fn result(&self) -> CoreResult<&InteractionOutput> {
        if let Some(e) = &self.0.first_cause {
            return Err(e.clone());
        }
        match &self.0.native_result {
            Some(Ok(o)) if self.0.state == ConsumerActionState::Completed => Ok(o),
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
        self.0.definition.selected[i]
            .as_ref()
            .map(|s| (s.identity, s.op.form))
    }
}
impl Invocation {
    fn time(&mut self, now: MonotonicInstant) -> CoreResult<()> {
        if now
            .checked_cmp(self.last_time)
            .is_none_or(|c| c == Ordering::Less)
        {
            return Err(invalid());
        }
        self.last_time = now;
        check_time(self.deadline, now)
    }
    fn close(&mut self, cause: CoreError) {
        if self.first_cause.is_none() {
            self.first_cause = Some(cause);
        }
        self.retiring = self.exchange.is_some();
        self.state = if self.retiring {
            ConsumerActionState::Closing
        } else {
            ConsumerActionState::Closed
        };
    }
    /// Passive fences also apply after native completion, until the invocation
    /// is moved out of its slot by the settlement handoff.
    fn fence(&mut self, active: bool, now: MonotonicInstant) {
        if let Err(e) = self.time(now) {
            self.close(e);
        }
        if !active || !self.identity.consumer.is_current() {
            self.close(stale());
        }
        if self.first_cause.is_none()
            && let Err(e) = self.definition.data_current()
        {
            self.close(e);
        }
    }
    fn refresh_access(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.first_cause.is_some() {
            return Ok(());
        }
        let selected = self.definition.selected(self.operation)?;
        // The native bound funds destination calculation, not a new exchange.
        // No codec or native I/O callback is needed for an already validated result.
        if !pay(budget, W::ProviderProbes, selected.access_work)
            || !pay(budget, W::BindingPolls, selected.bounds.poll_work)
        {
            return Err(busy());
        }
        if let Err(e) = selected
            .check(&self.identity.consumer, self.continuation.as_deref())
            .and_then(|()| self.definition.data_current())
        {
            self.close(e);
        }
        Ok(())
    }
    fn step(
        &mut self,
        active: bool,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) {
        self.fence(active, now);
        if self.state.settlement_ready() {
            let _ = self.refresh_access(budget);
            return;
        }
        let selected = self
            .definition
            .selected(self.operation)
            .expect("captured operation");
        // All callback and late validation work is paid before native progress.
        if prepay(selected, budget).is_err() {
            return;
        }
        if !pay(
            budget,
            W::CodecInputBytes,
            self.definition.selected(0).unwrap().policy.validation_work,
        ) {
            return;
        }
        if self.first_cause.is_none()
            && let Err(e) = selected.check(&self.identity.consumer, self.continuation.as_deref())
        {
            self.close(e);
        }
        if let Some(raw) = self.exchange.as_mut()
            && !self.retiring
        {
            match raw.0.as_mut().expect("owned exchange").poll(cx) {
                Poll::Pending => return,
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
        if let Some(raw) = self.exchange.as_mut() {
            match raw.0.as_mut().expect("owned exchange").poll_close(cx) {
                Poll::Pending => return,
                Poll::Ready(Err(e)) => {
                    if self.cleanup_error.is_none() {
                        self.cleanup_error = Some(e);
                    }
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
        if let Some(reply) = self.reply.take() {
            self.accept_reply(reply);
        }
        if self.first_cause.is_some() {
            self.state = ConsumerActionState::Closed;
        }
        self.retiring = false;
    }
    fn accept_reply(&mut self, reply: ConsumerActionReply) {
        // Keep the original output even when validation, cancellation or a
        // replacement prevents it from becoming an application success.
        match reply {
            ConsumerActionReply::Complete(output) if self.operation != 2 => {
                let validation = if raw_action_output_shape(&output)
                    && interaction_output_backing(&output).is_some_and(|n| {
                        n as u64
                            <= self
                                .definition
                                .selected(self.operation)
                                .unwrap()
                                .bounds
                                .response_bytes
                    }) {
                    self.definition
                        .validate(self.definition.schemas.output, output.data())
                } else {
                    Err(invalid())
                };
                self.native_result = Some(Ok(output));
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
        if self.first_cause.is_none() {
            let s = self.definition.selected(self.operation).unwrap();
            if let Err(e) = s.check(&self.identity.consumer, self.continuation.as_deref()) {
                // Terminal state must not suppress fencing of this late result.
                self.first_cause = Some(e);
            }
        }
    }
}
fn prepay(s: &Selected, budget: &mut WorkBudget) -> CoreResult<()> {
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
