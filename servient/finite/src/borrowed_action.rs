//! Caller-owned continuing Consumer Action. Borrowed source-free dependencies,
//! concrete native owners and exclusive progress require no shared container,
//! reference counting, atomics, synchronization or executor.
use clinkz_wot_core::{
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, InteractionOutputView, Operation,
    PayloadView, RetryClass,
    data_schema::PreparedSchema,
    execution::{ValueExecution, ValueValidator, action::*, read::LocalConsumerRef},
    security::SecurityRequirement,
};
use clinkz_wot_foundation::{
    MonotonicInstant, ResourceKind, ResourceReservation, WorkBudget, WorkClass as W,
};
use core::{cmp::Ordering, task::Context};

const OPERATIONS: [Operation; 3] = [
    Operation::InvokeAction,
    Operation::QueryAction,
    Operation::CancelAction,
];
fn index(op: Operation) -> usize {
    match op {
        Operation::InvokeAction => 0,
        Operation::QueryAction => 1,
        Operation::CancelAction => 2,
        _ => unreachable!("admitted Action operation"),
    }
}
type Token<B> = <<B as ActionBinding>::Exchange as ActionExchange>::Continuation;
type Output<B> = <<B as ActionBinding>::Exchange as ActionExchange>::Output;
type Reply<B> = ActionReply<Token<B>, Output<B>>;

#[derive(Clone, Copy)]
struct Data<'a> {
    service: &'a dyn ValueValidator,
    work: u64,
    body: usize,
    media: &'a str,
}
impl<'a> Data<'a> {
    fn capture(service: &'a dyn ValueValidator) -> Self {
        Self {
            service,
            work: service.work(),
            body: service.body_bytes(),
            media: service.content_type(),
        }
    }
    fn check(&self, schema: &PreparedSchema) -> CoreResult<()> {
        if self.work == 0
            || !self.service.available()
            || self.service.work() != self.work
            || self.service.body_bytes() != self.body
            || self.service.content_type() != self.media
            || !self.service.supports(schema)
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn validate(&self, schema: &PreparedSchema, payload: PayloadView<'_>) -> CoreResult<()> {
        self.check(schema)?;
        if payload.body.len() > self.body
            || payload.content_type != self.media
            || payload.content_coding.is_some()
        {
            return Err(invalid());
        }
        let result = self.service.validate(schema, payload);
        self.check(schema)?;
        result
    }
}

/// Qualified source-free meaning and actual native dependencies. Construction
/// grants no TD admission, permission, instance identity or execution capacity.
/// Only static NoSec Action meaning with primary application output is supported.
pub struct ActionDefinition<'a, B: ActionBinding> {
    meaning: ValueExecution<'a>,
    binding: &'a B,
    program: &'a B::Program,
    configuration: B::Configuration,
    mappings: [Option<ActionMapping<'a>>; 3],
    native_work: [u64; 3],
    data: [Option<Data<'a>>; 2],
    backing: u64,
}
impl<'a, B: ActionBinding> ActionDefinition<'a, B> {
    pub fn try_new(
        meaning: ValueExecution<'a>,
        binding: &'a B,
        program: &'a B::Program,
        configuration: B::Configuration,
        invoke_data: Option<&'a dyn ValueValidator>,
        query_data: Option<&'a dyn ValueValidator>,
    ) -> Result<Self, (B::Configuration, CoreError)> {
        let d = Self {
            meaning,
            binding,
            program,
            configuration,
            mappings: OPERATIONS.map(|op| binding.mapping(program, op)),
            native_work: OPERATIONS.map(|op| binding.poll_work(op)),
            data: [
                invoke_data.map(Data::capture),
                query_data.map(Data::capture),
            ],
            backing: binding.backing_bytes(),
        };
        let checked = (|| {
            if meaning.operation() != Operation::InvokeAction
                || *meaning.security() != SecurityRequirement::NOSEC
                || meaning.uri_variables().is_some_and(|v| v.is_dynamic())
                || meaning.responses().is_some()
                || d.mappings[0].is_none()
                || !binding.supports(program, meaning)
            {
                return Err(invalid());
            }
            d.qualify()
        })();
        match checked {
            Ok(()) => Ok(d),
            Err(e) => Err((d.configuration, e)),
        }
    }
    fn check_native(&self, op: Operation) -> CoreResult<()> {
        let i = index(op);
        if *self.binding.configuration() != self.configuration
            || self.binding.backing_bytes() != self.backing
            || self.mappings[i].is_none()
            || self.native_work[i] == 0
            || self.binding.mapping(self.program, op) != self.mappings[i]
            || self.binding.poll_work(op) != self.native_work[i]
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn check_data(&self, op: Operation, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        if let Some(schema) = schema {
            self.data
                .get(index(op))
                .and_then(|d| *d)
                .ok_or_else(invalid)?
                .check(schema)?;
        }
        Ok(())
    }
    fn qualify(&self) -> CoreResult<()> {
        for op in OPERATIONS {
            if self.mappings[index(op)].is_some() {
                self.check_native(op)?;
                if op != Operation::CancelAction {
                    self.check_data(op, self.meaning.schemas().output.as_ref())?;
                }
            }
        }
        self.check_data(
            Operation::InvokeAction,
            self.meaning.schemas().input.as_ref(),
        )
    }
    fn validate(
        &self,
        op: Operation,
        schema: Option<&PreparedSchema>,
        payload: Option<PayloadView<'_>>,
    ) -> CoreResult<()> {
        match (schema, payload) {
            (None, None) => Ok(()),
            (Some(schema), Some(payload)) => self.data[index(op)]
                .ok_or_else(invalid)?
                .validate(schema, payload),
            _ => Err(invalid()),
        }
    }
    fn validation_work(&self, op: Operation, schema: Option<&PreparedSchema>) -> u64 {
        if schema.is_some() {
            self.data[index(op)].map_or(0, |d| d.work)
        } else {
            0
        }
    }
    fn validate_output_view(
        &self,
        op: Operation,
        output: InteractionOutputView<'_>,
    ) -> CoreResult<()> {
        if !action_output_shape(output) {
            return Err(invalid());
        }
        self.validate(op, self.meaning.schemas().output.as_ref(), output.data())
    }
}

struct Association<'a, B: ActionBinding> {
    definition: &'a ActionDefinition<'a, B>,
    consumer: LocalConsumerRef<'a>,
    access: [Option<&'a dyn ActionAccess>; 3],
    revisions: [u64; 3],
    access_work: [u64; 3],
}
impl<B: ActionBinding> Copy for Association<'_, B> {}
impl<B: ActionBinding> Clone for Association<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<B: ActionBinding> Association<'_, B> {
    fn qualify(&self) -> CoreResult<()> {
        self.definition.qualify()?;
        for op in OPERATIONS {
            if self.definition.mappings[index(op)].is_some() {
                self.dependencies(op, None)?;
            }
        }
        Ok(())
    }
    fn dependencies(&self, op: Operation, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        if !self.consumer.is_current() {
            return Err(stale());
        }
        let d = self.definition;
        d.check_native(Operation::InvokeAction)?;
        d.check_native(op)?;
        d.check_data(op, schema)?;
        // Original Invoke permission remains an invocation dependency; a retired
        // input codec or unused control is not a live output dependency.
        for i in [0, index(op)] {
            let access = self.access[i].ok_or_else(unsupported)?;
            if access.revision() != self.revisions[i]
                || access.work() != self.access_work[i]
                || self.access_work[i] == 0
                || self.access_work[i].checked_mul(2).is_none()
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn check(
        &self,
        op: Operation,
        schema: Option<&PreparedSchema>,
        token: Option<&Token<B>>,
    ) -> CoreResult<()> {
        self.dependencies(op, schema)?;
        let d = self.definition;
        let destination = d.binding.destination(d.program, op, token)?;
        let mapping = d.mappings[index(op)].unwrap();
        if destination.binding != mapping.binding || destination.generation != mapping.generation {
            return Err(invalid());
        }
        self.dependencies(op, schema)?;
        let result = self.access[index(op)]
            .unwrap()
            .authorize(self.consumer, op, destination);
        self.dependencies(op, schema)?;
        result
    }
    fn work(&self, op: Operation, schema: Option<&PreparedSchema>) -> ActionWork {
        ActionWork {
            access: self.access_work[index(op)],
            native: self.definition.native_work[index(op)],
            validation: self.definition.validation_work(op, schema),
        }
    }
}

/// One separately funded invocation opportunity. Dropping a copyable handle
/// drives no cleanup; the exclusive slot owner supplies turns or local disposal.
pub struct ActionSlot<'a, B: ActionBinding, I: ActionInput> {
    association: Association<'a, B>,
    work: Option<Invocation<'a, B, I>>,
    closed: bool,
}
#[derive(Clone, Copy)]
pub struct ActionHandle<'a> {
    consumer: LocalConsumerRef<'a>,
    sequence: u64,
}
struct Invocation<'a, B: ActionBinding, I: ActionInput> {
    progress: ActionProgress,
    association: Association<'a, B>,
    sequence: u64,
    operation: Operation,
    deadline: Deadline,
    last_time: MonotonicInstant,
    native_error: Option<CoreError>,
    exchange: Option<B::Exchange>,
    reply: Option<Reply<B>>,
    token: Option<Token<B>>,
    result: Option<CoreResult<Output<B>>>,
    input: Option<I>,
    // Last: original native work, token, input and result die before funding.
    funding: ResourceReservation<'a>,
}
impl<'a, B: ActionBinding, I: ActionInput> ActionSlot<'a, B, I> {
    pub fn try_new(
        definition: &'a ActionDefinition<'a, B>,
        consumer: LocalConsumerRef<'a>,
        access: [Option<&'a dyn ActionAccess>; 3],
    ) -> CoreResult<Self> {
        definition.qualify()?;
        let association = Association {
            definition,
            consumer,
            access,
            revisions: access.map(|a| a.map_or(0, ActionAccess::revision)),
            access_work: access.map(|a| a.map_or(0, ActionAccess::work)),
        };
        for op in OPERATIONS {
            if definition.mappings[index(op)].is_some() {
                association.dependencies(op, None)?;
            }
        }
        Ok(Self {
            association,
            work: None,
            closed: false,
        })
    }
    /// Complete inline work/result opportunity plus declared indirect backing.
    /// Input owners declare any additional backing separately at admission.
    pub fn required_bytes(&self, input: Option<&I>) -> CoreResult<u64> {
        (core::mem::size_of::<Invocation<'a, B, I>>() as u64)
            .checked_add(self.association.definition.backing)
            .and_then(|n| n.checked_add(input.map_or(0, ActionInput::backing_bytes)))
            .ok_or_else(invalid)
    }
    pub fn invoke(
        &mut self,
        input: Option<I>,
        funding: ResourceReservation<'a>,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ActionHandle<'a>, (Option<I>, ResourceReservation<'a>, CoreError)> {
        let a = self.association;
        let d = a.definition;
        let checked = (|| {
            if self.closed || !a.consumer.is_current() {
                return Err(stale());
            }
            if self.work.is_some() {
                return Err(busy());
            }
            if funding.kind() != ResourceKind::EngineLiveBytesGlobalMax
                || funding.owner() != (a.consumer.slot().slot(), a.consumer.slot().generation())
                || funding.bytes() < self.required_bytes(input.as_ref())?
            {
                return Err(invalid());
            }
            check_time(deadline, now)?;
            a.qualify()?;
            let schema = d.meaning.schemas().input.as_ref();
            if !a.work(Operation::InvokeAction, schema).prepay(budget) {
                return Err(busy());
            }
            // Keep one loan across authorization, validation and native start.
            // A new view may select different bytes or revoke the issuer.
            let payload = input.as_ref().map(ActionInput::view);
            a.check(Operation::InvokeAction, schema, None)?;
            d.validate(Operation::InvokeAction, schema, payload)?;
            a.check(Operation::InvokeAction, schema, None)?;
            a.qualify()?;
            let sequence = a.consumer.reserve_sequence()?;
            let exchange =
                d.binding
                    .start(d.program, Operation::InvokeAction, payload, None, deadline)?;
            Ok((sequence, exchange))
        })();
        let (sequence, exchange) = match checked {
            Ok(v) => v,
            Err(e) => return Err((input, funding, e)),
        };
        let mut work = Invocation {
            progress: ActionProgress::default(),
            association: a,
            sequence,
            operation: Operation::InvokeAction,
            deadline,
            last_time: now,
            native_error: None,
            exchange: Some(exchange),
            reply: None,
            token: None,
            result: None,
            input,
            funding,
        };
        work.fence();
        self.work = Some(work);
        Ok(ActionHandle {
            consumer: a.consumer,
            sequence,
        })
    }
    fn original(&mut self, handle: ActionHandle<'_>) -> CoreResult<&mut Invocation<'a, B, I>> {
        self.work
            .as_mut()
            .filter(|w| w.association.consumer == handle.consumer && w.sequence == handle.sequence)
            .ok_or_else(stale)
    }
    pub fn state(&mut self, handle: ActionHandle<'_>) -> CoreResult<ConsumerActionState> {
        let w = self.original(handle)?;
        w.fence();
        Ok(w.progress.state)
    }
    pub fn query(
        &mut self,
        handle: ActionHandle<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.original(handle)?
            .control(Operation::QueryAction, now, budget)
    }
    pub fn request_cancel(
        &mut self,
        handle: ActionHandle<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.original(handle)?
            .control(Operation::CancelAction, now, budget)
    }
    pub fn cancel_local(&mut self, handle: ActionHandle<'_>) -> CoreResult<()> {
        self.original(handle)?
            .close(CoreError::Cancelled(context()));
        Ok(())
    }
    pub fn begin_close(&mut self) {
        self.closed = true;
        if let Some(w) = &mut self.work {
            w.close(stale());
        }
    }
    pub fn is_idle(&self) -> bool {
        self.work.is_none()
    }
    pub fn step(&mut self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        if let Some(w) = &mut self.work {
            w.observe_time(now);
            step_action(w, cx, budget);
        }
    }
    /// Fresh paid owner/permission transfer. Short credit retains the original
    /// occupied slot. Application payload loans require their own exact-view
    /// validation at `ActionSettlement::result`; failed settlements retain native evidence.
    pub fn take_settlement(
        &mut self,
        handle: ActionHandle<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<ActionSettlement<'a, B, I>> {
        let w = self.original(handle)?;
        w.observe_time(now);
        w.fence();
        if !w.progress.state.settlement_ready() || budget.consume(W::CleanupItems, 1).is_err() {
            return Err(busy());
        }
        w.refresh_access(budget)?;
        w.fence();
        Ok(ActionSettlement(self.work.take().unwrap()))
    }
}

/// Original owners survive slot reuse. Result/input views cannot detach their
/// backing or outlive this settlement; only explicit copies acquire new owners.
///
/// ```compile_fail,E0515
/// use clinkz_wot_core::execution::action::{ActionBinding, ActionInput};
/// use clinkz_wot_servient_finite::borrowed_action::ActionSettlement;
/// fn escape<B: ActionBinding, I: ActionInput>(mut s: ActionSettlement<'_, B, I>) -> &'static [u8] {
///     s.result(&mut clinkz_wot_foundation::WorkBudget::new()).unwrap().data().unwrap().body
/// }
/// ```
///
/// ```compile_fail,E0505
/// use clinkz_wot_core::execution::action::{ActionBinding, ActionInput};
/// use clinkz_wot_servient_finite::borrowed_action::ActionSettlement;
/// fn refund_while_borrowed<B: ActionBinding, I: ActionInput>(mut s: ActionSettlement<'_, B, I>,
///     budget: &mut clinkz_wot_foundation::WorkBudget) {
///     let view = s.result(budget).unwrap();
///     let _funding = s.into_funding();
///     let _bytes = view.data().unwrap().body;
/// }
/// ```
pub struct ActionSettlement<'a, B: ActionBinding, I: ActionInput>(Invocation<'a, B, I>);
impl<B: ActionBinding, I: ActionInput> ActionSettlement<'_, B, I> {
    /// Matches the Host continuing Action's conservative effect evidence.
    /// Local completion/cancellation/retirement supplies no rollback proof.
    pub fn effect(&self) -> clinkz_wot_core::execution::ConsumerEffect {
        clinkz_wot_core::execution::ConsumerEffect::Unknown
    }
    pub fn consumer(&self) -> LocalConsumerRef<'_> {
        self.0.association.consumer
    }
    pub fn sequence(&self) -> u64 {
        self.0.sequence
    }
    pub fn state(&self) -> ConsumerActionState {
        self.0.progress.state
    }
    pub fn accepted(&self) -> bool {
        self.0.progress.accepted
    }
    pub fn remote_cancellation(&self) -> ConsumerActionCancellation {
        self.0.progress.remote_cancel
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
    pub fn native_error(&self) -> Option<&CoreError> {
        self.0.native_error.as_ref()
    }
    /// Current view supplied by the original retained input owner. This calls
    /// `ActionInput::view` again; bytes/media/coding may differ from the exact
    /// view used for admission and dispatch. No dispatch snapshot is retained.
    pub fn input(&self) -> Option<PayloadView<'_>> {
        self.0.input.as_ref().map(ActionInput::view)
    }
    /// Current native evidence, including rejected or late output. This view
    /// carries no application-validation claim and may differ on another call.
    pub fn native_result(&self) -> Option<Result<InteractionOutputView<'_>, &CoreError>> {
        self.0
            .result
            .as_ref()
            .map(|r| r.as_ref().map(ActionOutput::view))
    }
    /// Capture, validate and return one exact application loan. Native output
    /// owners may select different views between calls; prior validation grants
    /// no certificate for a new loan. Prepay one HandlerSteps adapter operation
    /// and the original returning operation's complete codec allowance before
    /// obtaining a view. Short credit retains all owners without recording a
    /// failure; invalid output records a sticky first cause and validation error.
    /// Ownership/current permission was transferred at `take_settlement`; this
    /// does no new authorization, native progress or remote-effect inference.
    pub fn result(&mut self, budget: &mut WorkBudget) -> CoreResult<InteractionOutputView<'_>> {
        if let Some(e) = &self.0.progress.first_cause {
            return Err(e.clone());
        }
        let output = match &self.0.result {
            Some(Ok(output)) if self.state() == ConsumerActionState::Completed => output,
            Some(Err(e)) => return Err(e.clone()),
            _ => return Err(invalid()),
        };
        let d = self.0.association.definition;
        let work = d.validation_work(self.0.operation, d.meaning.schemas().output.as_ref());
        if budget.consume(W::HandlerSteps, 1).is_err()
            || budget.consume(W::CodecInputBytes, work).is_err()
        {
            return Err(busy());
        }
        let view = output.view();
        if let Err(error) = d.validate_output_view(self.0.operation, view) {
            self.0.progress.validation_error = Some(error.clone());
            // Owner transfer already required acknowledged exchange retirement.
            self.0.progress.close(error.clone(), false);
            return Err(error);
        }
        Ok(view)
    }
}
impl<'a, B: ActionBinding, I: ActionInput> ActionSettlement<'a, B, I> {
    pub fn into_funding(self) -> ResourceReservation<'a> {
        let Invocation {
            exchange,
            reply,
            token,
            result,
            input,
            funding,
            ..
        } = self.0;
        drop(exchange);
        drop(reply);
        drop(token);
        drop(result);
        drop(input);
        funding
    }
}
impl<B: ActionBinding, I: ActionInput> Invocation<'_, B, I> {
    fn output_schema(&self) -> Option<&PreparedSchema> {
        if self.operation != Operation::CancelAction
            && (self.exchange.is_some()
                || matches!(self.reply, Some(ActionReply::Complete(_)))
                || matches!(self.result, Some(Ok(_))))
        {
            self.association
                .definition
                .meaning
                .schemas()
                .output
                .as_ref()
        } else {
            None
        }
    }
    fn observe_time(&mut self, now: MonotonicInstant) {
        match now.checked_cmp(self.last_time) {
            Some(Ordering::Equal | Ordering::Greater) => self.last_time = now,
            _ => self.close(invalid()),
        }
    }
    fn control(
        &mut self,
        op: Operation,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        self.observe_time(now);
        self.fence();
        if let Some(e) = &self.progress.first_cause {
            return Err(e.clone());
        }
        if self.progress.state != ConsumerActionState::Pending || self.exchange.is_some() {
            return Err(busy());
        }
        let a = self.association;
        let d = a.definition;
        if d.mappings[index(op)].is_none() {
            return Err(unsupported());
        }
        let schema = if op == Operation::QueryAction {
            d.meaning.schemas().output.as_ref()
        } else {
            None
        };
        if !a.work(op, None).prepay(budget) {
            return Err(busy());
        }
        let preflight = a.check(op, schema, self.token.as_ref());
        self.fence();
        if let Some(e) = &self.progress.first_cause {
            return Err(e.clone());
        }
        preflight?;
        let started = d
            .binding
            .start(d.program, op, None, self.token.as_ref(), self.deadline);
        self.fence();
        match started {
            Err(e) => Err(self.progress.first_cause.clone().unwrap_or(e)),
            Ok(exchange) => {
                self.exchange = Some(exchange);
                self.operation = op;
                self.progress.retiring = false;
                self.progress.state = if op == Operation::QueryAction {
                    ConsumerActionState::Querying
                } else {
                    self.progress.remote_cancel = ConsumerActionCancellation::Uncertain;
                    ConsumerActionState::Cancelling
                };
                self.fence();
                Ok(())
            }
        }
    }
}
impl<B: ActionBinding, I: ActionInput> ActionDriver for Invocation<'_, B, I> {
    fn progress(&mut self) -> &mut ActionProgress {
        &mut self.progress
    }
    fn operation(&self) -> Operation {
        self.operation
    }
    fn has_exchange(&self) -> bool {
        self.exchange.is_some()
    }
    fn work(&self) -> ActionWork {
        self.association.work(self.operation, self.output_schema())
    }
    fn close(&mut self, cause: CoreError) {
        self.progress.close(cause, self.exchange.is_some());
    }
    fn invalid_reply(&self) -> CoreError {
        invalid()
    }
    fn fence(&mut self) {
        if let Err(e) = check_time(self.deadline, self.last_time) {
            self.close(e);
        }
        if !self.association.consumer.is_current() {
            self.close(stale());
        }
        if self.progress.first_cause.is_none()
            && let Err(e) = self
                .association
                .dependencies(self.operation, self.output_schema())
        {
            self.close(e);
        }
    }
    fn check_current(&mut self) -> CoreResult<()> {
        self.association
            .check(self.operation, self.output_schema(), self.token.as_ref())
    }
    fn refresh_access(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        if self.progress.first_cause.is_some() {
            return Ok(());
        }
        let i = index(self.operation);
        if budget
            .consume(W::ProviderProbes, self.association.access_work[i])
            .is_err()
            || budget
                .consume(W::BindingPolls, self.association.definition.native_work[i])
                .is_err()
        {
            return Err(busy());
        }
        if let Err(e) = self.check_current() {
            self.close(e);
        }
        Ok(())
    }
    fn poll_exchange(&mut self, cx: &mut Context<'_>) -> core::task::Poll<CoreResult<()>> {
        self.exchange
            .as_mut()
            .unwrap()
            .poll(cx)
            .map(|result| match result {
                Ok(reply) => {
                    self.reply = Some(reply);
                    Ok(())
                }
                Err(e) => {
                    self.native_error.get_or_insert_with(|| e.clone());
                    self.result = Some(Err(e.clone()));
                    Err(e)
                }
            })
    }
    fn poll_close(&mut self, cx: &mut Context<'_>) -> core::task::Poll<CoreResult<()>> {
        self.exchange
            .as_mut()
            .unwrap()
            .poll_close(cx)
            .map(|result| {
                result.map(|late| {
                    if self.reply.is_none() {
                        self.reply = late;
                    }
                    self.exchange = None;
                })
            })
    }
    fn retain_reply(&mut self) -> Option<ActionObservation> {
        Some(match self.reply.take()? {
            ActionReply::Accepted(token) if self.operation == Operation::InvokeAction => {
                self.token = Some(token);
                ActionObservation::Accepted
            }
            ActionReply::Accepted(_) => ActionObservation::Invalid,
            ActionReply::Complete(output) => {
                self.result = Some(Ok(output));
                ActionObservation::Complete
            }
            ActionReply::Failed(e) => {
                self.result = Some(Err(e));
                ActionObservation::Failed
            }
            ActionReply::Pending => ActionObservation::Pending,
            ActionReply::CancelRequested => ActionObservation::CancelRequested,
            ActionReply::AlreadyTerminal => ActionObservation::AlreadyTerminal,
        })
    }
    fn validate_output(&mut self) -> CoreResult<()> {
        let output = self.result.as_ref().unwrap().as_ref().unwrap().view();
        self.association
            .definition
            .validate_output_view(self.operation, output)
    }
    fn reset_operation(&mut self) {
        self.operation = Operation::InvokeAction;
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
