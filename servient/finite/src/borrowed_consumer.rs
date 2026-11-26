//! One caller-owned finite Property Read. This is an alternative representation
//! of modern prepared value execution, not a TD compiler, PlanSet, scheduler or
//! Embedded transport. Dependencies are borrowed; active work/results are owned.
use clinkz_wot_core::{
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, InteractionOutputView, Operation,
    RetryClass,
    execution::{
        FiniteCallDriver, FiniteCallState, NativeProgress, ValueExecution, ValueValidator, read::*,
        validate_finite_output,
    },
    security::SecurityRequirement,
};
use clinkz_wot_foundation::MonotonicInstant;
use core::{
    cell::{RefCell, RefMut},
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
    validator: &'a dyn ValueValidator,
    body_bytes: usize,
    media: &'a str,
}
impl<'a, B: ReadBinding> ReadDefinition<'a, B> {
    pub fn try_new(
        meaning: ValueExecution<'a>,
        binding: &'a B,
        program: &'a B::Program,
        configuration: B::Configuration,
        validator: &'a dyn ValueValidator,
    ) -> Result<Self, (B::Configuration, CoreError)> {
        if meaning.operation() != Operation::ReadProperty
            || meaning.uri_variables().is_some_and(|v| v.is_dynamic())
            || *meaning.security() != SecurityRequirement::NOSEC
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
            body_bytes: validator.body_bytes(),
            media: validator.content_type(),
        };
        if let Err(error) = definition.check() {
            return Err((definition.configuration, error));
        }
        Ok(definition)
    }
    fn check(&self) -> CoreResult<()> {
        if self.meaning.responses().is_some_and(|r| {
            !r.additional().is_empty()
                || r.get(clinkz_wot_core::ResponseSelection::Primary)
                    .is_none_or(|primary| {
                        primary.content_type.as_ref() != self.media
                            || primary.data
                                != clinkz_wot_core::execution::responses::ResponseData::Operation
                    })
        }) || self.binding.configuration() != &self.configuration
            || self.binding.mapping(self.program) != self.mapping
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
/// result cell is exclusively borrowed through settlement disposal.
pub struct ReadRuntime<'a, B: ReadBinding> {
    driving: &'a RefCell<()>,
    consumer: LocalConsumerRef<'a>,
    definition: &'a ReadDefinition<'a, B>,
    access: &'a dyn ReadAccess,
    revision: u64,
    closed: bool,
    call: Option<ReadWork<'a, B>>,
}
/// Actual storage for a normal or late reply. A settlement keeps the cell
/// occupied, independently of the active runtime and its native exchange.
pub struct ReadResultSlot<O>(RefCell<Option<ReadNativeSettlement<O>>>);
impl<O> Default for ReadResultSlot<O> {
    fn default() -> Self {
        Self::new()
    }
}
impl<O> ReadResultSlot<O> {
    pub const fn new() -> Self {
        Self(RefCell::new(None))
    }
}

struct ReadWork<'a, B: ReadBinding> {
    progress: FiniteCallState,
    owner: ReadOwner<'a, B>,
}
struct ReadOwner<'a, B: ReadBinding> {
    driving: &'a RefCell<()>,
    definition: &'a ReadDefinition<'a, B>,
    consumer: LocalConsumerRef<'a>,
    access: &'a dyn ReadAccess,
    revision: u64,
    sequence: u64,
    native: Option<B::Call>,
    outcome: RefMut<'a, Option<ReadNativeSettlement<<B::Call as ReadNativeCall>::Output>>>,
    first_cause: Option<CoreError>,
    cleanup_error: Option<CoreError>,
    deadline: Deadline,
    last_time: MonotonicInstant,
}
impl<'a, B: ReadBinding> ReadRuntime<'a, B> {
    /// Sibling runtimes in one manual loop borrow the same drive cell.
    pub fn bind(
        driving: &'a RefCell<()>,
        consumer: LocalConsumerRef<'a>,
        definition: &'a ReadDefinition<'a, B>,
        access: &'a dyn ReadAccess,
    ) -> CoreResult<Self> {
        let _turn = driving.try_borrow_mut().map_err(|_| busy())?;
        let runtime = Self {
            driving,
            consumer,
            definition,
            access,
            revision: access.revision(),
            closed: false,
            call: None,
        };
        check_current(consumer, definition, access, runtime.revision)?;
        Ok(runtime)
    }
    /// Backpressure leaves the original result cell and native endpoint available
    /// to their current owners. Retry from the deployment's next independent turn.
    pub fn read(
        &mut self,
        result: &'a ReadResultSlot<<B::Call as ReadNativeCall>::Output>,
        deadline: Deadline,
        now: MonotonicInstant,
    ) -> CoreResult<ReadHandle<'a>> {
        let _turn = self.driving.try_borrow_mut().map_err(|_| busy())?;
        if self.closed || !self.consumer.is_current() {
            return Err(stale());
        }
        if self.call.is_some() {
            return Err(busy());
        }
        check_deadline(deadline, now)?;
        let outcome = result.0.try_borrow_mut().map_err(|_| busy())?;
        check_current(self.consumer, self.definition, self.access, self.revision)?;
        let sequence = self.consumer.reserve_sequence()?;
        let native = self
            .definition
            .binding
            .start(self.definition.program, deadline)?;
        let mut owner = ReadOwner {
            driving: self.driving,
            definition: self.definition,
            consumer: self.consumer,
            access: self.access,
            revision: self.revision,
            sequence,
            native: Some(native),
            outcome,
            first_cause: None,
            cleanup_error: None,
            deadline,
            last_time: now,
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
    pub fn step(&mut self, cx: &mut Context<'_>, now: MonotonicInstant) -> Poll<()> {
        let Some(call) = &mut self.call else {
            return Poll::Ready(());
        };
        match now.checked_cmp(call.owner.last_time) {
            Some(Ordering::Equal | Ordering::Greater) => call.owner.last_time = now,
            _ => call.owner.record(clock_error()),
        }
        let Ok(_turn) = self.driving.try_borrow_mut() else {
            call.owner.fence();
            return Poll::Pending;
        };
        let progress = call.progress.step(&mut call.owner, cx);
        if progress.is_ready() {
            drop(call.owner.native.take());
            call.progress.fence(&mut call.owner);
        }
        progress
    }
    pub fn poll(
        &mut self,
        handle: ReadHandle<'_>,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<ReadSettlement<'a, B>>> {
        if !self.original(handle) {
            return Poll::Ready(Err(stale()));
        }
        if self.step(cx, now).is_pending() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(ReadSettlement {
            owner: self.call.take().unwrap().owner,
        }))
    }
}

/// Retains the original result cell. A validated loan uses the exact native
/// view checked for this delivery and can survive later authority changes.
///
/// ```compile_fail,E0515
/// use clinkz_wot_core::execution::read::ReadBinding;
/// use clinkz_wot_servient_finite::borrowed_consumer::ReadSettlement;
/// fn escape<B: ReadBinding>(mut s: ReadSettlement<'_, B>) -> &'static [u8] {
///     s.result().unwrap().data().unwrap().body
/// }
/// ```
pub struct ReadSettlement<'a, B: ReadBinding> {
    owner: ReadOwner<'a, B>,
}
impl<B: ReadBinding> ReadSettlement<'_, B> {
    pub fn mapping(&self) -> ReadMapping<'_> {
        self.owner.definition.mapping
    }
    pub fn consumer(&self) -> LocalConsumerRef<'_> {
        self.owner.consumer
    }
    /// Inspect the original native record, including an unvalidated late value.
    pub fn native_output(&self) -> CoreResult<Option<InteractionOutputView<'_>>> {
        let _turn = self.owner.driving.try_borrow_mut().map_err(|_| busy())?;
        Ok(match self.owner.outcome.as_ref() {
            Some(ReadNativeSettlement::Returned(Ok(output))) => Some(output.view()),
            _ => None,
        })
    }
    pub fn cleanup_error(&self) -> Option<&CoreError> {
        self.owner.cleanup_error.as_ref()
    }
    pub fn result(&mut self) -> CoreResult<InteractionOutputView<'_>> {
        let owner = &mut self.owner;
        if let Some(error) = &owner.first_cause {
            return Err(error.clone());
        }
        let _turn = owner.driving.try_borrow_mut().map_err(|_| busy())?;
        let checked = (|| {
            let current = || {
                check_current(
                    owner.consumer,
                    owner.definition,
                    owner.access,
                    owner.revision,
                )
            };
            current()?;
            let output = match owner.outcome.as_ref().expect("native settlement") {
                ReadNativeSettlement::Returned(Ok(output)) => output.view(),
                ReadNativeSettlement::Returned(Err(error)) => return Err(error.clone()),
                ReadNativeSettlement::Cancelled => return Err(CoreError::Cancelled(context())),
            };
            let d = owner.definition;
            let m = d.mapping;
            validate_finite_output(
                m.binding,
                m.generation,
                m.plan,
                Operation::ReadProperty,
                output,
            )?;
            let payload = output.data().ok_or_else(invalid)?;
            if payload.body.len() > d.body_bytes
                || payload.content_type != d.media
                || payload.content_coding.is_some()
            {
                return Err(invalid());
            }
            current()?;
            d.validator
                .validate(d.meaning.schemas().output.as_ref().unwrap(), payload)?;
            current()?;
            Ok(output)
        })();
        if let Err(error) = &checked {
            owner.first_cause = Some(error.clone());
        }
        checked
    }
}

impl<B: ReadBinding> Drop for ReadOwner<'_, B> {
    fn drop(&mut self) {
        // A surrounding turn may already hold the cell. In either case it stays
        // held while both linked destructors run, including during unwinding.
        let _turn = self.driving.try_borrow_mut().ok();
        drop((self.native.take(), self.outcome.take()));
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
    fn check_current(&mut self) -> CoreResult<()> {
        check_current(self.consumer, self.definition, self.access, self.revision)
    }
    fn poll_native(
        &mut self,
        cx: &mut Context<'_>,
        cancel: Option<(&CoreError, bool)>,
    ) -> NativeProgress {
        let native = self.native.as_mut().expect("active native call");
        if let Some((cause, started)) = cancel {
            let result = if started {
                native.poll_cancel(cx)
            } else {
                match native.start_cancel(cause, cx) {
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
                    *self.outcome = Some(outcome);
                    NativeProgress::Settled
                }
            }
        } else {
            match native.poll_result(cx) {
                Poll::Pending => NativeProgress::Pending,
                Poll::Ready(result) => {
                    if let Err(error) = &result {
                        self.record(error.clone());
                    }
                    *self.outcome = Some(ReadNativeSettlement::Returned(result));
                    NativeProgress::Settled
                }
            }
        }
    }
}
fn check_current<B: ReadBinding>(
    consumer: LocalConsumerRef<'_>,
    d: &ReadDefinition<'_, B>,
    access: &dyn ReadAccess,
    revision: u64,
) -> CoreResult<()> {
    let dependencies = || {
        if !consumer.is_current() {
            return Err(stale());
        }
        d.check()?;
        if access.revision() != revision {
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
