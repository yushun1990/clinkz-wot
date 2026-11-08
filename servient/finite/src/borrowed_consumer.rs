//! One caller-owned finite Property Read. This is an alternative representation
//! of modern prepared value execution, not a TD compiler, PlanSet, scheduler or
//! Embedded transport. Dependencies are borrowed; active work/results are owned.
use clinkz_wot_core::{
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, InteractionOutputView, Operation,
    RetryClass,
    execution::{
        FiniteCallDriver, FiniteCallState, NativeProgress, ValueExecution, read::*,
        validate_finite_output,
    },
    security::SecurityRequirement,
};
use clinkz_wot_foundation::{
    MonotonicInstant, ResourceKind, ResourceReservation, WorkBudget, WorkClass as W,
};
use core::{
    cmp::Ordering,
    task::{Context, Poll},
};

/// Qualified borrowed dependencies, without instance authority or active work.
/// Pure construction accepts only the executable NoSec Property Read subset.
/// It does not attest TD admission/selection; those belong above execution.
pub struct ReadDefinition<'a, B: ReadBinding> {
    meaning: ValueExecution<'a>,
    binding: &'a B,
    program: &'a B::Program,
    configuration: B::Configuration,
    mapping: ReadMapping<'a>,
    validator: &'a dyn ReadValidator,
    validation_work: u64,
    body_bytes: usize,
    media: &'a str,
    backing_bytes: u64,
}
impl<'a, B: ReadBinding> ReadDefinition<'a, B> {
    pub fn try_new(
        meaning: ValueExecution<'a>,
        binding: &'a B,
        program: &'a B::Program,
        configuration: B::Configuration,
        validator: &'a dyn ReadValidator,
    ) -> Result<Self, (B::Configuration, CoreError)> {
        if meaning.operation() != Operation::ReadProperty
            || *meaning.security() != SecurityRequirement::NOSEC
            || validator.work() == 0
        {
            return Err((configuration, invalid()));
        }
        let definition = Self {
            meaning,
            binding,
            program,
            configuration,
            mapping: binding.mapping(program),
            validator,
            validation_work: validator.work(),
            body_bytes: validator.body_bytes(),
            media: validator.content_type(),
            backing_bytes: binding.backing_bytes(),
        };
        if let Err(error) = definition.check() {
            return Err((definition.configuration, error));
        }
        Ok(definition)
    }
    fn check(&self) -> CoreResult<()> {
        if self.binding.configuration() != &self.configuration
            || self.binding.mapping(self.program) != self.mapping
            || self.binding.backing_bytes() != self.backing_bytes
            || self.validator.work() != self.validation_work
            || self.validator.body_bytes() != self.body_bytes
            || self.validator.content_type() != self.media
            || !self.validator.available()
            || !self
                .validator
                .supports(self.meaning.schemas().output.as_ref().ok_or_else(invalid)?)
            || !self
                .binding
                .supports(self.program, self.meaning, self.media)
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn meaning(&self) -> ValueExecution<'a> {
        self.meaning
    }
}

/// Token identifies the original authority and admitted obligation. Sequences
/// are issued across all facets of that incarnation, never reset on slot reuse.
#[derive(Clone, Copy)]
pub struct ReadHandle<'a> {
    consumer: LocalConsumerRef<'a>,
    sequence: u64,
}

/// One explicitly driven slot. Caller storage bounds its cardinality; the
/// caller-provided complete lease stays with active work and unread settlement.
pub struct ReadRuntime<'a, B: ReadBinding> {
    consumer: LocalConsumerRef<'a>,
    definition: &'a ReadDefinition<'a, B>,
    access: &'a dyn ReadAccess,
    revision: u64,
    access_work: u64,
    closed: bool,
    call: Option<ReadWork<'a, B>>,
}
struct ReadWork<'a, B: ReadBinding> {
    progress: FiniteCallState,
    owner: ReadOwner<'a, B>,
}
struct ReadOwner<'a, B: ReadBinding> {
    definition: &'a ReadDefinition<'a, B>,
    consumer: LocalConsumerRef<'a>,
    access: &'a dyn ReadAccess,
    revision: u64,
    access_work: u64,
    sequence: u64,
    native: B::Call,
    outcome: Option<ReadNativeSettlement<<B::Call as ReadNativeCall>::Output>>,
    first_cause: Option<CoreError>,
    cleanup_error: Option<CoreError>,
    validation_error: Option<CoreError>,
    deadline: Deadline,
    last_time: MonotonicInstant,
    // Last to drop: all actual native/output state precedes original funding.
    _funding: ResourceReservation<'a>,
}
impl<'a, B: ReadBinding> ReadRuntime<'a, B> {
    pub fn bind(
        consumer: LocalConsumerRef<'a>,
        definition: &'a ReadDefinition<'a, B>,
        access: &'a dyn ReadAccess,
        budget: &mut WorkBudget,
    ) -> CoreResult<Self> {
        let runtime = Self {
            consumer,
            definition,
            access,
            revision: access.revision(),
            access_work: access.work(),
            closed: false,
            call: None,
        };
        if runtime.access_work == 0 {
            return Err(invalid());
        }
        if budget
            .consume(W::ProviderProbes, runtime.access_work)
            .is_err()
        {
            return Err(busy());
        }
        check_current(
            consumer,
            definition,
            access,
            runtime.revision,
            runtime.access_work,
        )?;
        Ok(runtime)
    }
    pub fn required_bytes(&self) -> CoreResult<u64> {
        (core::mem::size_of::<ReadWork<'a, B>>() as u64)
            .checked_add(self.definition.backing_bytes)
            .ok_or_else(invalid)
    }
    /// Reserve caller capacity before native construction. Refusal returns the
    /// identical reservation; no hidden allocation or background owner exists.
    pub fn read(
        &mut self,
        funding: ResourceReservation<'a>,
        deadline: Deadline,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Result<ReadHandle<'a>, (ResourceReservation<'a>, CoreError)> {
        let checked = (|| {
            if self.closed || !self.consumer.is_current() {
                return Err(stale());
            }
            if self.call.is_some() {
                return Err(busy());
            }
            if funding.kind() != ResourceKind::EngineLiveBytesGlobalMax
                || funding.owner()
                    != (
                        self.consumer.slot().slot(),
                        self.consumer.slot().generation(),
                    )
                || funding.bytes() < self.required_bytes()?
            {
                return Err(invalid());
            }
            check_deadline(deadline, now)?;
            if budget.consume(W::ProviderProbes, self.access_work).is_err() {
                return Err(busy());
            }
            check_current(
                self.consumer,
                self.definition,
                self.access,
                self.revision,
                self.access_work,
            )?;
            if budget.remaining(W::BindingPolls) == 0 || budget.remaining(W::CleanupItems) == 0 {
                return Err(busy());
            }
            budget.consume(W::BindingPolls, 1).unwrap();
            budget.consume(W::CleanupItems, 1).unwrap();
            let sequence = self.consumer.reserve_sequence()?;
            self.definition
                .binding
                .start(self.definition.program, deadline)
                .map(|native| (sequence, native))
        })();
        let (sequence, native) = match checked {
            Ok(v) => v,
            Err(e) => return Err((funding, e)),
        };
        let mut owner = ReadOwner {
            definition: self.definition,
            consumer: self.consumer,
            access: self.access,
            revision: self.revision,
            access_work: self.access_work,
            sequence,
            native,
            outcome: None,
            first_cause: None,
            cleanup_error: None,
            validation_error: None,
            deadline,
            last_time: now,
            _funding: funding,
        };
        owner.fence();
        self.call = Some(ReadWork {
            progress: FiniteCallState::default(),
            owner,
        });
        Ok(ReadHandle {
            consumer: self.consumer,
            sequence,
        })
    }
    fn original(&self, handle: ReadHandle<'_>) -> bool {
        handle.consumer == self.consumer
            && self
                .call
                .as_ref()
                .is_some_and(|c| c.owner.sequence == handle.sequence)
    }
    pub fn cancel(&mut self, handle: ReadHandle<'_>) -> CoreResult<()> {
        if !self.original(handle) {
            return Err(stale());
        }
        self.call
            .as_mut()
            .unwrap()
            .owner
            .record(CoreError::Cancelled(context()));
        Ok(())
    }
    pub fn begin_close(&mut self) {
        self.closed = true;
        if let Some(call) = &mut self.call {
            call.owner.record(stale());
        }
    }
    pub fn is_idle(&self) -> bool {
        self.call.is_none()
    }
    /// Progress/cleanup remains possible after cancellation or revocation.
    /// An unread terminal result stays in its original occupied slot.
    pub fn step(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Poll<()> {
        let Some(call) = &mut self.call else {
            return Poll::Ready(());
        };
        match now.checked_cmp(call.owner.last_time) {
            Some(Ordering::Equal | Ordering::Greater) => call.owner.last_time = now,
            _ => call.owner.record(clock_error()),
        }
        call.progress.step(&mut call.owner, cx, budget)
    }
    pub fn poll(
        &mut self,
        handle: ReadHandle<'_>,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<ReadSettlement<'a, B>>> {
        if !self.original(handle) {
            return Poll::Ready(Err(stale()));
        }
        if self.step(cx, now, budget).is_pending() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(ReadSettlement {
            work: self.call.take().unwrap(),
        }))
    }
}

/// Complete original normal/late/rejected output and native owner. Slices cannot
/// outlive this settlement, nor can original funding be refunded before Drop.
///
/// ```compile_fail,E0515
/// use clinkz_wot_core::execution::read::ReadBinding;
/// use clinkz_wot_servient_finite::borrowed_consumer::ReadSettlement;
/// fn escape<B: ReadBinding>(s: ReadSettlement<'_, B>) -> &'static [u8] {
///     s.result().unwrap().data().unwrap().body
/// }
/// ```
pub struct ReadSettlement<'a, B: ReadBinding> {
    work: ReadWork<'a, B>,
}
impl<B: ReadBinding> ReadSettlement<'_, B> {
    pub fn mapping(&self) -> ReadMapping<'_> {
        self.work.owner.definition.mapping
    }
    pub fn consumer(&self) -> LocalConsumerRef<'_> {
        self.work.owner.consumer
    }
    pub fn native_output(&self) -> Option<InteractionOutputView<'_>> {
        match self.work.owner.outcome.as_ref()? {
            ReadNativeSettlement::Returned(Ok(output)) => Some(output.view()),
            _ => None,
        }
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.work.owner.cleanup_error.as_ref()
    }
    pub fn result(&self) -> CoreResult<InteractionOutputView<'_>> {
        if let Some(error) = self.work.progress.first_cause() {
            return Err(error.clone());
        }
        match self.work.owner.outcome.as_ref().unwrap() {
            ReadNativeSettlement::Returned(Ok(output)) => Ok(output.view()),
            ReadNativeSettlement::Returned(Err(error)) => Err(error.clone()),
            ReadNativeSettlement::Cancelled => Err(CoreError::Cancelled(context())),
        }
    }
}

impl<'a, B: ReadBinding> ReadSettlement<'a, B> {
    /// Dispose original native work and output before returning the complete
    /// caller reservation for another call. Dropping it instead refunds capacity.
    /// No borrowed result can survive consuming this owner.
    pub fn into_funding(self) -> ResourceReservation<'a> {
        let ReadOwner {
            native,
            outcome,
            _funding,
            ..
        } = self.work.owner;
        drop(native);
        drop(outcome);
        _funding
    }
}

impl<B: ReadBinding> FiniteCallDriver for ReadOwner<'_, B> {
    fn fence(&mut self) -> Option<CoreError> {
        if let Err(error) = check_deadline(self.deadline, self.last_time) {
            self.record(error);
        }
        if !self.consumer.is_current() {
            self.record(stale());
        }
        self.first_cause.clone()
    }
    fn record(&mut self, error: CoreError) {
        self.first_cause.get_or_insert(error);
    }
    fn access_work(&self) -> u64 {
        self.access_work
    }
    fn check_current(&mut self) -> CoreResult<()> {
        check_current(
            self.consumer,
            self.definition,
            self.access,
            self.revision,
            self.access_work,
        )
    }
    fn poll_native(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<(&CoreError, bool)>,
        budget: &mut WorkBudget,
    ) -> NativeProgress {
        if let Some((cause, started)) = cancel {
            let result = if started {
                self.native.poll_cancel(cx, budget)
            } else {
                match self.native.start_cancel(cause, cx, budget) {
                    Ok(Poll::Pending) => return NativeProgress::Cancelling,
                    Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
                    Err(error) => Poll::Ready(Err(error)),
                }
            };
            match result {
                Poll::Pending => NativeProgress::Pending,
                Poll::Ready(Err(error)) => {
                    self.cleanup_error.get_or_insert(error);
                    NativeProgress::Pending
                }
                Poll::Ready(Ok(outcome)) => {
                    self.outcome = Some(outcome);
                    NativeProgress::Settled
                }
            }
        } else {
            match self.native.poll_result(cx, budget) {
                Poll::Pending => NativeProgress::Pending,
                Poll::Ready(result) => {
                    if let Err(error) = &result {
                        self.record(error.clone());
                    }
                    self.outcome = Some(ReadNativeSettlement::Returned(result));
                    NativeProgress::Settled
                }
            }
        }
    }
    fn has_success(&self) -> bool {
        matches!(self.outcome, Some(ReadNativeSettlement::Returned(Ok(_))))
    }
    fn validate_output(&mut self, budget: &mut WorkBudget) -> Poll<CoreResult<()>> {
        if let Some(error) = &self.validation_error {
            return Poll::Ready(Err(error.clone()));
        }
        let checked = (|| {
            self.definition.check()?;
            let ReadNativeSettlement::Returned(Ok(output)) = self.outcome.as_ref().unwrap() else {
                unreachable!()
            };
            let output = output.view();
            let m = self.definition.mapping;
            validate_finite_output(
                m.binding,
                m.generation,
                m.plan,
                Operation::ReadProperty,
                output,
            )?;
            let payload = output.data().ok_or_else(invalid)?;
            if payload.body.len() > self.definition.body_bytes
                || payload.content_type != self.definition.media
                || payload.content_coding.is_some()
            {
                return Err(invalid());
            }
            if budget
                .consume(W::CodecInputBytes, self.definition.validation_work)
                .is_err()
            {
                return Ok(Poll::Pending);
            }
            self.definition.validator.validate(
                self.definition.meaning.schemas().output.as_ref().unwrap(),
                payload,
            )?;
            Ok(Poll::Ready(Ok(())))
        })();
        match checked {
            Ok(p) => p,
            Err(error) => {
                self.validation_error = Some(error.clone());
                Poll::Ready(Err(error))
            }
        }
    }
}
fn check_current<B: ReadBinding>(
    consumer: LocalConsumerRef<'_>,
    d: &ReadDefinition<'_, B>,
    access: &dyn ReadAccess,
    revision: u64,
    work: u64,
) -> CoreResult<()> {
    let dependencies = || {
        if !consumer.is_current() {
            return Err(stale());
        }
        d.check()?;
        if access.revision() != revision || access.work() != work {
            return Err(invalid());
        }
        Ok(())
    };
    dependencies()?;
    access.authorize(consumer, d.mapping)?;
    dependencies()
}
fn check_deadline(deadline: Deadline, now: MonotonicInstant) -> CoreResult<()> {
    match deadline.checked_is_elapsed_at(now) {
        Some(false) => Ok(()),
        Some(true) => Err(CoreError::TimedOut(context())),
        None => Err(clock_error()),
    }
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn clock_error() -> CoreError {
    CoreError::InternalInvariant(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
