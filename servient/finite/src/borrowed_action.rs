//! Caller-owned continuing Consumer Action over a concrete native exchange,
//! a deployment drive cell and an exclusive typed result slot.
use clinkz_wot_core::{
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, InteractionOutputView, Operation,
    PayloadView, RetryClass,
    data_schema::PreparedSchema,
    execution::{ValueExecution, ValueValidator, action::*, read::LocalConsumerRef},
    security::SecurityRequirement,
};
use clinkz_wot_foundation::MonotonicInstant;
use core::{
    cell::{RefCell, RefMut},
    cmp::Ordering,
    task::{Context, Poll},
};

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
    body: usize,
    media: &'a str,
}
impl<'a> Data<'a> {
    fn capture(service: &'a dyn ValueValidator) -> Self {
        Self {
            service,
            body: service.body_bytes(),
            media: service.content_type(),
        }
    }
    fn check(&self, schema: &PreparedSchema) -> CoreResult<()> {
        if !self.service.available()
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

/// Source-free meaning and a native program. Installation supplies live
/// services separately and qualifies the exact program and configuration.
pub struct ActionDefinition<'a, P, C> {
    meaning: ValueExecution<'a>,
    program: &'a P,
    configuration: C,
}
impl<'a, P, C> ActionDefinition<'a, P, C> {
    pub fn try_new(
        meaning: ValueExecution<'a>,
        program: &'a P,
        configuration: C,
    ) -> Result<Self, (C, CoreError)> {
        if meaning.operation() != Operation::InvokeAction
            || *meaning.security() != SecurityRequirement::NOSEC
            || meaning.uri_variables().is_some_and(|v| v.is_dynamic())
            || meaning.responses().is_some()
        {
            return Err((configuration, invalid()));
        }
        Ok(Self {
            meaning,
            program,
            configuration,
        })
    }
}

struct Association<'a, B: ActionBinding> {
    definition: &'a ActionDefinition<'a, B::Program, B::Configuration>,
    binding: &'a B,
    consumer: LocalConsumerRef<'a>,
    mappings: [Option<ActionMapping<'a>>; 3],
    data: [Option<Data<'a>>; 2],
    access: [Option<&'a dyn ActionAccess>; 3],
    revisions: [u64; 3],
}
impl<B: ActionBinding> Copy for Association<'_, B> {}
impl<B: ActionBinding> Clone for Association<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<B: ActionBinding> Association<'_, B> {
    fn qualify(&self) -> CoreResult<()> {
        if self.mappings[0].is_none()
            || !self
                .binding
                .supports(self.definition.program, self.definition.meaning)
        {
            return Err(invalid());
        }
        for op in OPERATIONS {
            if self.mappings[index(op)].is_some() {
                self.dependencies(op, self.output_schema(op))?;
            }
        }
        self.check_data(
            Operation::InvokeAction,
            self.definition.meaning.schemas().input.as_ref(),
        )
    }
    fn output_schema(&self, op: Operation) -> Option<&PreparedSchema> {
        match op {
            Operation::InvokeAction | Operation::QueryAction => {
                self.definition.meaning.schemas().output.as_ref()
            }
            _ => None,
        }
    }
    fn check_data(&self, op: Operation, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        if let Some(schema) = schema {
            self.data[index(op)].ok_or_else(invalid)?.check(schema)?;
        }
        Ok(())
    }
    fn dependencies(&self, op: Operation, schema: Option<&PreparedSchema>) -> CoreResult<()> {
        if !self.consumer.is_current() {
            return Err(stale());
        }
        if self.binding.configuration() != &self.definition.configuration {
            return Err(invalid());
        }
        let original = [Operation::InvokeAction, op];
        for selected in original
            .into_iter()
            .take(if index(op) == 0 { 1 } else { 2 })
        {
            let i = index(selected);
            let mapping = self.mappings[i].ok_or_else(unsupported)?;
            if self.binding.mapping(self.definition.program, selected) != Some(mapping)
                || self.access[i].ok_or_else(unsupported)?.revision() != self.revisions[i]
            {
                return Err(invalid());
            }
        }
        self.check_data(op, schema)
    }
    fn authorize(
        &self,
        op: Operation,
        schema: Option<&PreparedSchema>,
        destination: &B::Destination,
    ) -> CoreResult<()> {
        self.dependencies(op, schema)?;
        let i = index(op);
        let mapping = self.mappings[i].expect("checked Action mapping");
        let result = self.access[i].expect("checked Action access").authorize(
            self.consumer,
            op,
            ActionMapping {
                target: destination.as_ref(),
                ..mapping
            },
        );
        self.dependencies(op, schema)?;
        result
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
    fn validate_output(&self, op: Operation, output: InteractionOutputView<'_>) -> CoreResult<()> {
        if !action_output_shape(output) {
            return Err(invalid());
        }
        self.validate(op, self.output_schema(op), output.data())
    }
}

/// Actual normal or late result storage. A settlement keeps its exclusive
/// loan across runtime reuse, until the result has been disposed.
pub struct ActionResultSlot<O>(RefCell<Option<CoreResult<O>>>);
impl<O> ActionResultSlot<O> {
    pub const fn new() -> Self {
        Self(RefCell::new(None))
    }
}
impl<O> Default for ActionResultSlot<O> {
    fn default() -> Self {
        Self::new()
    }
}

/// One caller-owned invocation opportunity. All siblings in a manual driver
/// borrow the same drive cell, including entry from a codec or destructor.
pub struct ActionSlot<'a, B: ActionBinding, I: ActionInput> {
    driving: &'a RefCell<()>,
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
    driving: &'a RefCell<()>,
    progress: ActionProgress,
    association: Association<'a, B>,
    sequence: u64,
    deadline: Deadline,
    last_time: MonotonicInstant,
    invoke_destination: Option<B::Destination>,
    control_destination: Option<B::Destination>,
    native_error: Option<CoreError>,
    exchange: Option<B::Exchange>,
    reply: Option<Reply<B>>,
    token: Option<Token<B>>,
    result: RefMut<'a, Option<CoreResult<Output<B>>>>,
    input: Option<I>,
}
impl<'a, B: ActionBinding, I: ActionInput> ActionSlot<'a, B, I> {
    pub fn bind(
        driving: &'a RefCell<()>,
        definition: &'a ActionDefinition<'a, B::Program, B::Configuration>,
        binding: &'a B,
        consumer: LocalConsumerRef<'a>,
        data: [Option<&'a dyn ValueValidator>; 2],
        access: [Option<&'a dyn ActionAccess>; 3],
    ) -> CoreResult<Self> {
        let _turn = driving.try_borrow_mut().map_err(|_| reentry())?;
        let association = Association {
            definition,
            binding,
            consumer,
            mappings: OPERATIONS.map(|op| binding.mapping(definition.program, op)),
            data: data.map(|service| service.map(Data::capture)),
            access,
            revisions: access.map(|a| a.map_or(0, ActionAccess::revision)),
        };
        association.qualify()?;
        Ok(Self {
            driving,
            association,
            work: None,
            closed: false,
        })
    }

    /// Admission claims the original result cell before linked code or native
    /// effects. On refusal the caller recovers the identical input.
    #[allow(clippy::type_complexity)]
    pub fn invoke(
        &mut self,
        input: Option<I>,
        result: &'a ActionResultSlot<Output<B>>,
        deadline: Deadline,
        now: MonotonicInstant,
    ) -> Result<ActionHandle<'a>, (Option<I>, CoreError)> {
        let a = self.association;
        let d = a.definition;
        let _turn = match self.driving.try_borrow_mut() {
            Ok(turn) => turn,
            Err(_) => return Err((input, reentry())),
        };
        let started = (|| {
            if self.closed || !a.consumer.is_current() {
                return Err(stale());
            }
            if self.work.is_some() {
                return Err(busy());
            }
            check_time(deadline, now)?;
            let result = result.0.try_borrow_mut().map_err(|_| busy())?;
            a.qualify()?;
            let operation = Operation::InvokeAction;
            let schema = d.meaning.schemas().input.as_ref();
            let payload = input.as_ref().map(ActionInput::view);
            let destination = a.binding.destination(d.program, operation, None)?;
            a.authorize(operation, schema, &destination)?;
            a.validate(operation, schema, payload)?;
            a.authorize(operation, schema, &destination)?;
            a.qualify()?;
            let sequence = a.consumer.reserve_sequence()?;
            let exchange =
                a.binding
                    .start(d.program, operation, &destination, payload, None, deadline)?;
            Ok((sequence, exchange, result, destination))
        })();
        let (sequence, exchange, result, destination) = match started {
            Ok(v) => v,
            Err(error) => return Err((input, error)),
        };
        let mut work = Invocation {
            driving: self.driving,
            progress: ActionProgress::default(),
            association: a,
            sequence,
            deadline,
            last_time: now,
            invoke_destination: Some(destination),
            control_destination: None,
            native_error: None,
            exchange: Some(exchange),
            reply: None,
            token: None,
            result,
            input,
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
        let work = self.original(handle)?;
        work.fence_local();
        Ok(work.progress.state)
    }
    pub fn query(&mut self, handle: ActionHandle<'_>, now: MonotonicInstant) -> CoreResult<()> {
        let _turn = self.driving.try_borrow_mut().map_err(|_| reentry())?;
        self.original(handle)?.control(Operation::QueryAction, now)
    }
    pub fn request_cancel(
        &mut self,
        handle: ActionHandle<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<()> {
        let _turn = self.driving.try_borrow_mut().map_err(|_| reentry())?;
        self.original(handle)?.control(Operation::CancelAction, now)
    }
    pub fn cancel_local(&mut self, handle: ActionHandle<'_>) -> CoreResult<()> {
        self.original(handle)?
            .close(CoreError::Cancelled(context()));
        Ok(())
    }
    pub fn begin_close(&mut self) {
        self.closed = true;
        if let Some(work) = &mut self.work {
            work.close(stale());
        }
    }
    pub fn is_idle(&self) -> bool {
        self.work.is_none()
    }
    pub fn step(&mut self, cx: &mut Context<'_>, now: MonotonicInstant) -> CoreResult<ActionTurn> {
        let _turn = self.driving.try_borrow_mut().map_err(|_| reentry())?;
        let Some(work) = &mut self.work else {
            return Ok(ActionTurn::Idle);
        };
        work.observe_time(now);
        Ok(step_action(work, cx))
    }
    /// Transfer the original retired invocation under current permission.
    /// The settlement keeps the original exclusive result slot.
    pub fn take_settlement(
        &mut self,
        handle: ActionHandle<'_>,
        now: MonotonicInstant,
    ) -> CoreResult<ActionSettlement<'a, B, I>> {
        let _turn = self.driving.try_borrow_mut().map_err(|_| reentry())?;
        let work = self.original(handle)?;
        work.observe_time(now);
        work.fence_local();
        if !work.progress.state.settlement_ready() {
            return Err(busy());
        }
        check_action_handoff(work);
        Ok(ActionSettlement(self.work.take().unwrap()))
    }
}

/// Lends the original input and result without releasing the slot that backs
/// them. Raw evidence and a freshly validated loan have separate access paths.
///
/// ```compile_fail,E0515
/// use clinkz_wot_core::execution::action::{ActionBinding, ActionInput};
/// use clinkz_wot_servient_finite::borrowed_action::ActionSettlement;
/// fn escape<B: ActionBinding, I: ActionInput>(mut s: ActionSettlement<'_, B, I>) -> &'static [u8] {
///     s.result().unwrap().data().unwrap().body
/// }
/// ```
pub struct ActionSettlement<'a, B: ActionBinding, I: ActionInput>(Invocation<'a, B, I>);
impl<B: ActionBinding, I: ActionInput> ActionSettlement<'_, B, I> {
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
    /// Inspection lends the input owner's current view. It may differ from
    /// the view passed to start.
    pub fn input(&self) -> CoreResult<Option<PayloadView<'_>>> {
        let Some(input) = self.0.input.as_ref() else {
            return Ok(None);
        };
        let _turn = self.0.driving.try_borrow_mut().map_err(|_| reentry())?;
        Ok(Some(input.view()))
    }
    /// Raw evidence can include rejected or late output. Its adapter is
    /// bounded and cannot drive a sibling Action or Read.
    pub fn native_result(
        &self,
    ) -> CoreResult<Option<Result<InteractionOutputView<'_>, &CoreError>>> {
        Ok(match self.0.result.as_ref() {
            Some(Ok(output)) => {
                let _turn = self.0.driving.try_borrow_mut().map_err(|_| reentry())?;
                Some(Ok(output.view()))
            }
            Some(Err(error)) => Some(Err(error)),
            None => None,
        })
    }
    /// Each application loan validates the exact view it returns, with fresh
    /// permission after the adapter and codec can change dependencies.
    pub fn result(&mut self) -> CoreResult<InteractionOutputView<'_>> {
        let work = &mut self.0;
        let _turn = work.driving.try_borrow_mut().map_err(|_| reentry())?;
        check_action_handoff(work);
        if let Some(error) = &work.progress.first_cause {
            return Err(error.clone());
        }
        let output = match work.result.as_ref() {
            Some(Ok(output)) if work.progress.state == ConsumerActionState::Completed => output,
            Some(Err(error)) => return Err(error.clone()),
            _ => return Err(invalid()),
        };
        let view = output.view();
        let operation = work.progress.operation();
        let association = work.association;
        let schema = association.output_schema(operation);
        let destination = work
            .control_destination
            .as_ref()
            .or(work.invoke_destination.as_ref())
            .expect("retained Action destination");
        let validated = association.dependencies(operation, schema).and_then(|()| {
            let validated = association.validate_output(operation, view);
            if let Err(error) = &validated {
                work.progress.validation_error = Some(error.clone());
            }
            association.authorize(operation, schema, destination)?;
            validated
        });
        if let Err(error) = &validated {
            work.progress.close(error.clone());
        }
        validated.map(|()| view)
    }
}

impl<B: ActionBinding, I: ActionInput> Drop for Invocation<'_, B, I> {
    fn drop(&mut self) {
        let _turn = self.driving.try_borrow_mut().ok();
        drop((
            self.exchange.take(),
            self.reply.take(),
            self.token.take(),
            self.result.take(),
            self.input.take(),
            self.control_destination.take(),
            self.invoke_destination.take(),
        ));
    }
}
impl<B: ActionBinding, I: ActionInput> Invocation<'_, B, I> {
    fn destination(&self) -> &B::Destination {
        self.control_destination
            .as_ref()
            .or(self.invoke_destination.as_ref())
            .expect("admitted Action destination")
    }
    fn live_schema(&self) -> Option<&PreparedSchema> {
        let operation = self.progress.operation();
        if self.exchange.is_some()
            || matches!(self.reply, Some(ActionReply::Complete(_)))
            || matches!(*self.result, Some(Ok(_)))
        {
            self.association.output_schema(operation)
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
    fn fence_local(&mut self) {
        if let Err(e) = check_time(self.deadline, self.last_time) {
            self.progress.close(e);
        }
        if !self.association.consumer.is_current() {
            self.progress.close(stale());
        }
    }
    fn check_cause(&self) -> CoreResult<()> {
        self.progress.first_cause.clone().map_or(Ok(()), Err)
    }
    fn control(&mut self, op: Operation, now: MonotonicInstant) -> CoreResult<()> {
        self.observe_time(now);
        self.fence();
        self.check_cause()?;
        if !self.progress.can_control() {
            return Err(busy());
        }
        let a = self.association;
        let binding = a.binding;
        let program = a.definition.program;
        let schema = a.output_schema(op);
        let prepared = (|| {
            a.dependencies(op, schema)?;
            let token = self.token.as_ref().ok_or_else(invalid)?;
            let destination = binding.destination(program, op, Some(token))?;
            a.authorize(op, schema, &destination)?;
            Ok(destination)
        })();
        self.fence();
        self.check_cause()?;
        let destination = prepared?;
        let exchange = binding.start(
            program,
            op,
            &destination,
            None,
            self.token.as_ref(),
            self.deadline,
        );
        match exchange {
            Ok(exchange) => {
                self.exchange = Some(exchange);
                self.control_destination = Some(destination);
                self.progress.start_control(op);
                self.fence();
                Ok(())
            }
            Err(error) => {
                self.fence();
                self.check_cause()?;
                Err(error)
            }
        }
    }
}
impl<B: ActionBinding, I: ActionInput> ActionDriver for Invocation<'_, B, I> {
    fn progress(&mut self) -> &mut ActionProgress {
        &mut self.progress
    }
    fn close(&mut self, cause: CoreError) {
        self.fence_local();
        self.progress.close(cause);
    }
    fn fence_local(&mut self) {
        Invocation::fence_local(self);
    }
    fn invalid_reply(&self) -> CoreError {
        invalid()
    }
    fn fence(&mut self) {
        self.fence_local();
        if self.progress.first_cause.is_none()
            && let Err(error) = self
                .association
                .dependencies(self.progress.operation(), self.live_schema())
        {
            self.close(error);
        }
    }
    fn check_current(&mut self) -> CoreResult<()> {
        self.association.authorize(
            self.progress.operation(),
            self.live_schema(),
            self.destination(),
        )
    }
    fn poll_exchange(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.exchange
            .as_mut()
            .unwrap()
            .poll(cx)
            .map(|result| match result {
                Ok(reply) => {
                    self.reply = Some(reply);
                    Ok(())
                }
                Err(error) => {
                    self.native_error = Some(error.clone());
                    *self.result = Some(Err(error.clone()));
                    Err(error)
                }
            })
    }
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let Some(exchange) = self.exchange.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        exchange.poll_close(cx).map(|result| {
            result.map(|late| {
                if self.reply.is_none() {
                    self.reply = late;
                }
                drop(self.exchange.take());
            })
        })
    }
    fn retain_reply(&mut self) -> Option<ActionObservation> {
        Some(retain_action_reply(
            self.progress.operation(),
            self.reply.take()?,
            &mut self.token,
            &mut self.result,
        ))
    }
    fn validate_output(&mut self) -> CoreResult<()> {
        let output = self.result.as_ref().unwrap().as_ref().unwrap().view();
        self.association
            .validate_output(self.progress.operation(), output)
    }
    fn finish_operation(&mut self) {
        if self.progress.state.settlement_ready() {
            drop(self.token.take());
        } else {
            drop(self.control_destination.take());
        }
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
fn reentry() -> CoreError {
    CoreError::Lifecycle(ErrorContext::new(ErrorPhase::Readiness, RetryClass::Never))
}
