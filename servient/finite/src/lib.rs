//! Source-free, manually driven finite ingress. This deliberately does not use
//! TD, Planning, a route stage, or one native installation per Thing.
#![cfg(feature = "std")]
#![no_std]

extern crate alloc;

use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    vec::Vec,
};
use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, BindingInputRejection, BindingOperationalError,
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, HandlerContext, InteractionOutput,
    Operation, RetryClass,
    finite::{
        FiniteDelivery, FiniteEffect, FiniteIngress, FiniteIngressBinding, RawFiniteHandler,
        RawFiniteReply, RawFiniteSettlement, raw_finite_output_backing, raw_finite_property_shape,
    },
    producer::{ProducerInstanceAuthority, ProducerInstanceKey},
};
use clinkz_wot_foundation::{MonotonicInstant, WorkBudget, WorkClass};
use core::{
    mem::size_of,
    task::{Context, Poll},
};

// Bounded atomic lookup envelope for this manually driven Host facet. The
// registry is externally built before native setup; larger applications need
// a different indexed/resumable owner, not one unbounded HandlerSteps charge.
const MAX_INSTANCES: usize = 32;
const MAX_BEHAVIORS_PER_INSTANCE: usize = 16;
const MAX_TARGET_NAME_BYTES: usize = 256;

/// Handler storage and dispatch identity, independent of a native route.
pub struct RawFiniteBehavior {
    target: AffordanceTarget,
    operation: Operation,
    handler: Box<dyn RawFiniteHandler>,
}

impl RawFiniteBehavior {
    pub fn new(
        target: AffordanceTarget,
        operation: Operation,
        handler: impl RawFiniteHandler + 'static,
    ) -> Self {
        Self {
            target,
            operation,
            handler: Box::new(handler),
        }
    }
}

/// One move-only logical authority and behavior set, owned by Servient rather
/// than by any Binding runtime. Accepted work may retain this exact instance.
pub struct RawFiniteInstance {
    authority: ProducerInstanceAuthority,
    behaviors: Vec<RawFiniteBehavior>,
}

impl RawFiniteInstance {
    pub fn new(authority: ProducerInstanceAuthority, behaviors: Vec<RawFiniteBehavior>) -> Self {
        Self {
            authority,
            behaviors,
        }
    }

    pub fn key(&self) -> ProducerInstanceKey {
        self.authority.key()
    }
}

/// Fixed registration/lookup envelope at the logical owner. Construction and
/// replacement validate externally built behavior before changing authority.
#[derive(Clone, Copy, Debug)]
pub struct RawFiniteRegistryLimits {
    pub instances: usize,
    pub behaviors_per_instance: usize,
    /// Maximum UTF-8 bytes per registered or accepted native target name.
    pub target_name_bytes: usize,
}

struct InstanceEntry {
    instance: Rc<RawFiniteInstance>,
    current: bool,
}

struct AcceptedBehavior {
    instance: Rc<RawFiniteInstance>,
    behavior: usize,
}

/// One logical owner shared by independently typed native runtimes. Exclusive
/// manual access orders acceptance, revocation and replacement without locks.
/// This Host facet is single-threaded; retention references confer no authority.
pub struct RawFiniteServient {
    instances: Vec<InstanceEntry>,
    issuer: Rc<()>,
    limits: RawFiniteRegistryLimits,
}

impl RawFiniteServient {
    pub fn try_new(
        instances: Vec<RawFiniteInstance>,
        limits: RawFiniteRegistryLimits,
    ) -> Result<Self, BindingInputRejection<Vec<RawFiniteInstance>>> {
        let valid = (1..=MAX_INSTANCES).contains(&limits.instances)
            && (1..=MAX_BEHAVIORS_PER_INSTANCE).contains(&limits.behaviors_per_instance)
            && limits.target_name_bytes <= MAX_TARGET_NAME_BYTES
            && !instances.is_empty()
            && instances.len() <= limits.instances
            && instances.iter().enumerate().all(|(i, instance)| {
                Self::valid_instance(instance, limits)
                    && !instances[..i]
                        .iter()
                        .any(|prior| prior.key().slot().slot() == instance.key().slot().slot())
            });
        if !valid {
            return Err(rejection(instances));
        }
        Ok(Self {
            instances: instances
                .into_iter()
                .map(|instance| InstanceEntry {
                    instance: Rc::new(instance),
                    current: true,
                })
                .collect(),
            issuer: Rc::new(()),
            limits,
        })
    }

    fn valid_instance(instance: &RawFiniteInstance, limits: RawFiniteRegistryLimits) -> bool {
        !instance.behaviors.is_empty()
            && instance.behaviors.len() <= limits.behaviors_per_instance
            && instance.behaviors.iter().enumerate().all(|(j, behavior)| {
                matches!(
                    behavior.operation,
                    Operation::ReadProperty | Operation::WriteProperty
                ) && matches!(&behavior.target, AffordanceTarget::Property(name)
                        if name.len() <= limits.target_name_bytes)
                    && !instance.behaviors[..j].iter().any(|prior| {
                        prior.operation == behavior.operation && prior.target == behavior.target
                    })
            })
    }

    /// Stop new acceptance everywhere. At most 32 key probes; no native callback,
    /// handler destruction or accepted-work cancellation occurs here.
    pub fn revoke(&mut self, key: ProducerInstanceKey) -> bool {
        if let Some(entry) = self
            .instances
            .iter_mut()
            .find(|entry| entry.instance.key() == key && entry.current)
        {
            entry.current = false;
            true
        } else {
            false
        }
    }

    /// Validate the complete successor, then atomically replace the expected
    /// incarnation (current or revoked) in the same numeric slot. Old accepted
    /// work retains its original instance; no Binding is reconfigured or closed.
    pub fn replace(
        &mut self,
        expected: ProducerInstanceKey,
        replacement: RawFiniteInstance,
    ) -> Result<(), BindingInputRejection<RawFiniteInstance>> {
        let index = self
            .instances
            .iter()
            .position(|entry| entry.instance.key() == expected);
        if index.is_none()
            || replacement.key() == expected
            || replacement.key().slot().slot() != expected.slot().slot()
            || !Self::valid_instance(&replacement, self.limits)
        {
            return Err(rejection(replacement));
        }
        self.instances[index.expect("checked entry")] = InstanceEntry {
            instance: Rc::new(replacement),
            current: true,
        };
        Ok(())
    }

    fn resolve<R>(
        &self,
        ingress: &FiniteIngress<R>,
        limits: RawFiniteLimits,
    ) -> CoreResult<(usize, usize)> {
        if ingress.candidates.len() > limits.candidates
            || ingress
                .input
                .data
                .as_ref()
                .is_some_and(|data| data.body.len() > limits.input_body_bytes)
        {
            return Err(CoreError::Backpressure(context()));
        }
        let [candidate] = ingress.candidates.as_slice() else {
            return Err(CoreError::UnsupportedOperation(context()));
        };
        match &candidate.target {
            AffordanceTarget::Property(name) if name.len() > self.limits.target_name_bytes => {
                return Err(CoreError::Backpressure(context()));
            }
            AffordanceTarget::Property(_) => {}
            _ => return Err(CoreError::UnsupportedOperation(context())),
        }
        let instance_index = self
            .instances
            .iter()
            .position(|entry| entry.instance.key() == candidate.instance && entry.current)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let behavior_index = self.instances[instance_index]
            .instance
            .behaviors
            .iter()
            .position(|b| b.target == candidate.target && b.operation == candidate.operation)
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        Ok((instance_index, behavior_index))
    }

    /// Progress one runtime using this exact logical owner. A different issuer
    /// returns an error without consuming or altering its exchange. Native polls
    /// and retirement can also be driven directly through `RawFiniteRuntime::step`.
    pub fn step<B: FiniteIngressBinding>(
        &mut self,
        runtime: &mut RawFiniteRuntime<B>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> CoreResult<Poll<RawFiniteState>> {
        if runtime.issuer.as_ptr() != Rc::as_ptr(&self.issuer) {
            return Err(CoreError::StaleHandle(context()));
        }
        runtime.check_time(now);
        if runtime.state != RawFiniteState::Dispatching {
            return Ok(runtime.step(cx, budget, now));
        }
        let mut units = (self.limits.instances
            + self.limits.behaviors_per_instance * (self.limits.target_name_bytes + 1)
            + 1) as u64;
        if runtime.validation == FiniteOutputValidation::Required {
            // Prepay the complete declared value/media traversal, before
            // behavior entry or a codec callback. Linked validators still owe
            // bounded cooperative work; this is not callback preemption.
            let bytes = runtime
                .limits
                .output_media_capacity
                .checked_mul(2)
                .and_then(|media| runtime.limits.output_body_bytes.checked_add(media))
                .and_then(|n| u64::try_from(n).ok())
                .and_then(|n| units.checked_add(n));
            let Some(total) = bytes else {
                runtime.fail(CoreError::Backpressure(context()));
                return Ok(Poll::Ready(runtime.state));
            };
            units = total;
        }
        if budget.consume(WorkClass::HandlerSteps, units).is_err() {
            return Ok(Poll::Pending);
        }
        if !runtime.pair_is_current() {
            runtime.fail(CoreError::Validation(context()));
            return Ok(Poll::Ready(runtime.state));
        }
        let ingress = runtime.ingress.take().expect("received exchange retained");
        let single = ingress
            .candidates
            .first()
            .filter(|_| ingress.candidates.len() == 1);
        let recipient = single.map(|candidate| candidate.instance);
        let operation = single.map(|candidate| candidate.operation);
        let mut effect = FiniteEffect::NotInvoked;
        let result = self.resolve(&ingress, runtime.limits).and_then(|(i, b)| {
            let instance = &self.instances[i].instance;
            let behavior = &instance.behaviors[b];
            behavior.handler.authorize(&ingress.input)?;
            // Policy is application code and can invalidate an externally
            // mutable provider. Recheck before behavior entry, then retain the
            // actual service loan through the synchronous callback/validation.
            if !runtime.pair_is_current() {
                return Err(CoreError::Validation(context()));
            }
            let validator = if runtime.validation == FiniteOutputValidation::Required {
                Some(
                    runtime
                        .binding
                        .output_validator()
                        .ok_or_else(|| CoreError::Validation(context()))?,
                )
            } else {
                None
            };
            let ctx = HandlerContext::for_producer(
                instance.key(),
                &behavior.target,
                behavior.operation,
                runtime.identity,
            )?;
            // Acceptance is ordered by exclusive access to this owner. No lock,
            // native callback, allocation or re-lookup surrounds application code.
            runtime.accepted_instance = Some(AcceptedBehavior {
                instance: Rc::clone(instance),
                behavior: b,
            });
            effect = FiniteEffect::Unknown;
            let output = behavior.handler.handle(ctx, &ingress.input)?;
            if !runtime.output_fits(&output) {
                return Err(CoreError::Backpressure(context()));
            }
            if !raw_finite_property_shape(behavior.operation, &output) {
                return Err(CoreError::Validation(
                    ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                        .with_thing(instance.key().slot())
                        .with_operation(behavior.operation),
                ));
            }
            if let Some(validator) = validator {
                validator.validate(&output)?;
            }
            Ok(output)
        });
        runtime.reply = Some(RawFiniteReply {
            ingress,
            result,
            effect,
            recipient,
            operation,
        });
        runtime.state = RawFiniteState::Delivering;
        Ok(Poll::Ready(runtime.state))
    }
}

/// Per-runtime exchange opportunity and Engine-retained result backing. Output is
/// application-backed during the handler callback; on return it is moved into
/// Engine ownership only if the *entire* measured representation fits this
/// declared result opportunity. Native input/metadata/backing is Binding-owned.
#[derive(Clone, Copy, Debug)]
pub struct RawFiniteLimits {
    pub candidates: usize,
    pub input_body_bytes: usize,
    pub output_body_bytes: usize,
    pub output_media_capacity: usize,
    pub output_retained_bytes: usize,
}

/// Output checking for this participant, independent of TD publication. Raw
/// mode claims no schema conformance; Required refuses an absent/lost service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteOutputValidation {
    Raw,
    Required,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawFiniteState {
    Starting,
    InstalledClosed,
    Serving,
    Dispatching,
    Delivering,
    Closing,
    Closed,
}

/// A single native driver may match many logical instances. At most one owned
/// exclusive exchange is in the Engine at a time in this manual Host slice.
/// Refusals still travel through the original Binding reply opportunity.
pub struct RawFiniteRuntime<B: FiniteIngressBinding> {
    // Complete pending work drops before the physical driver on local disposal.
    ingress: Option<FiniteIngress<B::Reply>>,
    reply: Option<RawFiniteReply<B::Reply>>,
    settlement: Option<RawFiniteSettlement>,
    binding: B,
    // Survives native disposal and ends only at settlement acknowledgement.
    accepted_instance: Option<AcceptedBehavior>,
    issuer: Weak<()>,
    identity: (BindingId, BindingGeneration),
    configuration: B::Configuration,
    limits: RawFiniteLimits,
    state: RawFiniteState,
    first_cause: Option<CoreError>,
    validation: FiniteOutputValidation,
    deadline: Deadline,
    io_deadline: Deadline,
    last_now: Option<MonotonicInstant>,
    // Starting may retain partial native effects even when poll_start returned
    // Pending. State alone cannot distinguish timing admission from live work.
    native_started: bool,
}

impl<B: FiniteIngressBinding> RawFiniteRuntime<B> {
    /// Associate a native owner with one Servient, without moving or copying
    /// logical authority/behavior. Rejection returns the complete native pair.
    /// `deadline` bounds the caller's logical interaction/lifetime envelope;
    /// native requests retain their independent Binding-owned deadline.
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        servient: &RawFiniteServient,
        limits: RawFiniteLimits,
        validation: FiniteOutputValidation,
        deadline: Deadline,
    ) -> Result<Self, BindingInputRejection<(B, B::Configuration)>> {
        let identity = binding.identity();
        let valid = binding.configuration() == &configuration
            && limits.candidates > 0
            && limits.output_retained_bytes >= size_of::<InteractionOutput>()
            && (validation == FiniteOutputValidation::Raw || binding.output_validator().is_some());
        if !valid {
            return Err(rejection((binding, configuration)));
        }
        Ok(Self {
            ingress: None,
            reply: None,
            binding,
            accepted_instance: None,
            issuer: Rc::downgrade(&servient.issuer),
            identity,
            configuration,
            limits,
            state: RawFiniteState::Starting,
            first_cause: None,
            settlement: None,
            validation,
            deadline,
            io_deadline: Deadline::NONE,
            last_now: None,
            native_started: false,
        })
    }

    pub fn state(&self) -> RawFiniteState {
        self.state
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }

    /// A delivered or explicitly retired exchange must be acknowledged before
    /// the next receive. This bounded record reports local effect and native
    /// delivery independently; neither an error nor close proves a write absent.
    pub fn take_settlement(&mut self) -> Option<RawFiniteSettlement> {
        let settlement = self.settlement.take();
        if settlement.is_some() {
            self.accepted_instance = None;
        }
        settlement
    }

    /// Locally enable raw acceptance after native readiness; this does not
    /// publish or qualify a TD, even with Required output validation. No native
    /// callback or handler runs here.
    pub fn activate_raw(&mut self) -> CoreResult<()> {
        if self.state != RawFiniteState::InstalledClosed {
            return Err(CoreError::Validation(context()));
        }
        if self.issuer.strong_count() == 0 {
            let error = CoreError::StaleHandle(context());
            self.fail(error.clone());
            return Err(error);
        }
        if !self.pair_is_current() {
            let error = CoreError::Validation(context());
            self.fail(error.clone());
            return Err(error);
        }
        self.state = RawFiniteState::Serving;
        Ok(())
    }

    pub fn begin_close(&mut self) {
        if self.state != RawFiniteState::Closed {
            if (self.ingress.is_some() || self.reply.is_some()) && self.first_cause.is_none() {
                self.first_cause = Some(CoreError::Cancelled(ErrorContext::new(
                    ErrorPhase::Cleanup,
                    RetryClass::Never,
                )));
            }
            // Unaccepted ingress retains its *entire* original native input and
            // opportunity for the Binding's refusal/retirement path.
            self.state = RawFiniteState::Closing;
        }
    }

    fn fail(&mut self, error: CoreError) {
        if self.first_cause.is_none() {
            self.first_cause = Some(error);
        }
        self.begin_close();
    }

    fn pair_is_current(&self) -> bool {
        self.binding.identity() == self.identity
            && self.binding.configuration() == &self.configuration
            && (self.validation == FiniteOutputValidation::Raw
                || self.binding.output_validator().is_some())
    }

    fn check_time(&mut self, now: MonotonicInstant) {
        if self.state == RawFiniteState::Closed {
            return;
        }
        let monotonic_failure = self
            .last_now
            .is_some_and(|last| now.checked_duration_since(last).is_none());
        let cleanup = self.state == RawFiniteState::Closing;
        let admitted = self.native_started || cleanup;
        let interaction_phase = if cleanup {
            ErrorPhase::Cleanup
        } else {
            ErrorPhase::Handler
        };
        let native_phase = if cleanup {
            ErrorPhase::Cleanup
        } else {
            ErrorPhase::Binding
        };
        let comparisons = [
            (self.deadline.checked_is_elapsed_at(now), interaction_phase),
            (self.io_deadline.checked_is_elapsed_at(now), native_phase),
        ];
        // Check the actual deadline domains before the general progress clock.
        // This preserves Handler versus Binding attribution when both fail.
        let clock_phase = comparisons
            .iter()
            .find_map(|(elapsed, phase)| elapsed.is_none().then_some(*phase))
            .or_else(|| {
                monotonic_failure.then_some(if self.deadline.instant().is_some() {
                    interaction_phase
                } else {
                    native_phase
                })
            });
        let timing_context = |phase| {
            ErrorContext::new(
                if admitted {
                    phase
                } else {
                    ErrorPhase::Admission
                },
                RetryClass::Never,
            )
        };
        if let Some(phase) = clock_phase {
            let context = timing_context(phase);
            self.fail(if admitted {
                CoreError::InternalInvariant(context)
            } else {
                CoreError::Validation(context)
            });
        } else if let Some(phase) = comparisons
            .iter()
            .find_map(|(elapsed, phase)| (*elapsed == Some(true)).then_some(*phase))
        {
            self.fail(CoreError::TimedOut(timing_context(phase)));
        }
        self.last_now = Some(now);
    }

    fn settle_reply(&mut self, delivery: FiniteDelivery) {
        if let Some(reply) = self.reply.take() {
            assert!(self.settlement.is_none(), "unread finite settlement");
            self.settlement = Some(RawFiniteSettlement {
                binding: self.identity,
                // A view of the captured registered target shares its funded
                // backing, rather than extending Binding-private input storage.
                target: self.accepted_instance.as_ref().map(|accepted| {
                    accepted.instance.behaviors[accepted.behavior]
                        .target
                        .clone()
                }),
                recipient: reply.recipient,
                operation: reply.operation,
                effect: reply.effect,
                delivery,
                result: reply.result,
            });
        }
    }

    fn settle_ingress(&mut self, delivery: FiniteDelivery) {
        if let Some(ingress) = self.ingress.take() {
            assert!(self.settlement.is_none(), "unread finite settlement");
            let single = ingress
                .candidates
                .first()
                .filter(|_| ingress.candidates.len() == 1);
            self.settlement = Some(RawFiniteSettlement {
                binding: self.identity,
                target: None,
                recipient: single.map(|c| c.instance),
                operation: single.map(|c| c.operation),
                effect: FiniteEffect::NotInvoked,
                delivery,
                result: Err(self.first_cause.clone().expect("close has a first cause")),
            });
        }
    }

    fn output_fits(&self, output: &InteractionOutput) -> bool {
        output.data().is_none_or(|payload| {
            payload.body.len() <= self.limits.output_body_bytes
                && payload.content_type.capacity() <= self.limits.output_media_capacity
                && payload
                    .content_coding
                    .as_ref()
                    .is_none_or(|coding| coding.capacity() <= self.limits.output_media_capacity)
        }) && raw_finite_output_backing(output)
            .is_some_and(|bytes| bytes <= self.limits.output_retained_bytes)
    }

    /// One paid native or cleanup poll. Dispatching waits for the associated
    /// Servient; native retirement never needs logical authority or a handler.
    /// Pending never discards the reply or changes the first failure.
    pub fn step(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> Poll<RawFiniteState> {
        use RawFiniteState::*;
        // Local fencing requires no native work credit. Cleanup still does.
        self.check_time(now);
        if self.state == Closed
            || self.state == InstalledClosed
            || (self.state == Serving && self.settlement.is_some())
            || (self.state == Dispatching && self.issuer.strong_count() != 0)
        {
            return Poll::Ready(self.state);
        }
        let class = match self.state {
            Closing => WorkClass::CleanupItems,
            _ => WorkClass::BindingPolls,
        };
        if budget.consume(class, 1).is_err() {
            return Poll::Pending;
        }
        if matches!(self.state, Starting | Serving | Dispatching) && self.issuer.strong_count() == 0
        {
            self.fail(CoreError::StaleHandle(context()));
            return Poll::Ready(self.state);
        }
        if self.state != Closing && !self.pair_is_current() {
            self.fail(CoreError::Validation(context()));
            return Poll::Ready(self.state);
        }
        match self.state {
            Starting => {
                // The first paid native invocation may create partial effects,
                // independently of its readiness result.
                self.native_started = true;
                match self.binding.poll_start(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(())) => {
                        self.state = InstalledClosed;
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Err(e)) => {
                        self.fail(e);
                        Poll::Ready(self.state)
                    }
                }
            }
            Serving => {
                let received = self.binding.poll_receive(cx, now);
                self.io_deadline = self.binding.request_deadline();
                match received {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(e)) => {
                        self.fail(e);
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Ok(ingress)) => {
                        self.ingress = Some(ingress);
                        self.state = Dispatching;
                        Poll::Ready(self.state)
                    }
                }
            }
            Delivering => match self
                .binding
                .poll_send(cx, self.reply.as_ref().expect("reply retained"))
            {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(e)) => {
                    self.fail(e);
                    Poll::Ready(self.state)
                }
                Poll::Ready(Ok(())) => {
                    self.settle_reply(FiniteDelivery::Delivered);
                    self.io_deadline = Deadline::NONE;
                    self.state = Serving;
                    Poll::Ready(self.state)
                }
            },
            Closing => match self.binding.poll_close(
                cx,
                self.ingress.as_ref(),
                self.reply.as_ref(),
                self.first_cause.as_ref(),
            ) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(e)) => {
                    if self.first_cause.is_none() {
                        self.first_cause = Some(e);
                    }
                    Poll::Ready(Closing)
                }
                Poll::Ready(Ok(delivery)) => {
                    self.settle_ingress(delivery);
                    self.settle_reply(delivery);
                    self.state = Closed;
                    Poll::Ready(Closed)
                }
            },
            InstalledClosed | Closed | Dispatching => unreachable!(),
        }
    }
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}

fn rejection<T>(input: T) -> BindingInputRejection<T> {
    BindingInputRejection::new(
        input,
        BindingOperationalError::new(CoreError::Validation(context())),
    )
}
