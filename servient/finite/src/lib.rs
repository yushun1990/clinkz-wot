//! Source-free, manually driven finite ingress. This deliberately does not use
//! TD, Planning, a route stage, or one native installation per Thing.
#![cfg(feature = "std")]
#![no_std]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, BindingInputRejection, BindingOperationalError,
    CoreError, CoreResult, ErrorContext, ErrorPhase, HandlerContext, InteractionOutput, Operation,
    RetryClass,
    finite::{
        FiniteDelivery, FiniteEffect, FiniteIngress, FiniteIngressBinding, RawFiniteHandler,
        RawFiniteReply, RawFiniteSettlement, raw_finite_output_backing, raw_finite_property_shape,
    },
    producer::{ProducerInstanceAuthority, ProducerInstanceKey},
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use core::{
    mem::size_of,
    task::{Context, Poll},
};

// Bounded atomic lookup envelope for this manually driven Host facet. The
// registry is externally built before native setup; larger applications need
// a different indexed/resumable owner, not one unbounded HandlerSteps charge.
const MAX_INSTANCES: usize = 32;
const MAX_BEHAVIORS_PER_INSTANCE: usize = 16;

/// Handler storage and dispatch identity, independent of a native route.
pub struct RawFiniteBehavior {
    target: AffordanceTarget,
    operation: Operation,
    handler: Box<dyn RawFiniteHandler + Send>,
}

impl RawFiniteBehavior {
    pub fn new(
        target: AffordanceTarget,
        operation: Operation,
        handler: impl RawFiniteHandler + Send + 'static,
    ) -> Self {
        Self {
            target,
            operation,
            handler: Box::new(handler),
        }
    }
}

/// One move-only logical instance. Multiple behaviors may use one physical
/// Binding service without giving the Binding any handler authority.
pub struct RawFiniteInstance {
    authority: ProducerInstanceAuthority,
    behaviors: Vec<RawFiniteBehavior>,
    current: bool,
}

impl RawFiniteInstance {
    pub fn new(authority: ProducerInstanceAuthority, behaviors: Vec<RawFiniteBehavior>) -> Self {
        Self {
            authority,
            behaviors,
            current: true,
        }
    }

    pub fn key(&self) -> ProducerInstanceKey {
        self.authority.key()
    }
}

/// Fixed Host lookup envelope and Engine-retained result backing. Output is
/// application-backed during the handler callback; on return it is moved into
/// Engine ownership only if the *entire* measured representation fits this
/// declared result opportunity. Native input/metadata/backing is Binding-owned.
#[derive(Clone, Copy, Debug)]
pub struct RawFiniteLimits {
    pub instances: usize,
    pub behaviors_per_instance: usize,
    pub candidates: usize,
    pub input_body_bytes: usize,
    pub output_body_bytes: usize,
    pub output_media_capacity: usize,
    pub output_retained_bytes: usize,
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
    binding: B,
    instances: Vec<RawFiniteInstance>,
    identity: (BindingId, BindingGeneration),
    configuration: B::Configuration,
    limits: RawFiniteLimits,
    state: RawFiniteState,
    first_cause: Option<CoreError>,
    settlement: Option<RawFiniteSettlement>,
}

impl<B: FiniteIngressBinding> RawFiniteRuntime<B> {
    /// Validate actual registered behavior before starting native effects. This
    /// is a raw runtime registration, not a TD coverage/publication certificate.
    #[allow(clippy::type_complexity)]
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        instances: Vec<RawFiniteInstance>,
        limits: RawFiniteLimits,
    ) -> Result<Self, BindingInputRejection<(B, B::Configuration, Vec<RawFiniteInstance>)>> {
        let identity = binding.identity();
        let valid = binding.configuration() == &configuration
            && (1..=MAX_INSTANCES).contains(&limits.instances)
            && (1..=MAX_BEHAVIORS_PER_INSTANCE).contains(&limits.behaviors_per_instance)
            && instances.len() <= limits.instances
            && limits.candidates > 0
            && limits.output_retained_bytes >= size_of::<InteractionOutput>()
            && !instances.is_empty()
            && instances.iter().enumerate().all(|(i, instance)| {
                !instance.behaviors.is_empty()
                    && instance.behaviors.len() <= limits.behaviors_per_instance
                    && !instances[..i]
                        .iter()
                        .any(|prior| prior.key().slot() == instance.key().slot())
                    && instance.behaviors.iter().enumerate().all(|(j, behavior)| {
                        matches!(
                            behavior.operation,
                            Operation::ReadProperty | Operation::WriteProperty
                        ) && !instance.behaviors[..j].iter().any(|prior| {
                            prior.operation == behavior.operation && prior.target == behavior.target
                        }) && HandlerContext::for_producer(
                            instance.key(),
                            &behavior.target,
                            behavior.operation,
                            identity,
                        )
                        .is_ok()
                    })
            });
        if !valid {
            return Err(BindingInputRejection::new(
                (binding, configuration, instances),
                BindingOperationalError::new(CoreError::Validation(context())),
            ));
        }
        Ok(Self {
            ingress: None,
            reply: None,
            binding,
            instances,
            identity,
            configuration,
            limits,
            state: RawFiniteState::Starting,
            first_cause: None,
            settlement: None,
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
        self.settlement.take()
    }

    /// Locally enable raw acceptance after native readiness; this does not
    /// publish or qualify a TD. No native callback or handler runs here.
    pub fn activate_raw(&mut self) -> CoreResult<()> {
        if self.state != RawFiniteState::InstalledClosed || !self.pair_is_current() {
            return Err(CoreError::Validation(context()));
        }
        self.state = RawFiniteState::Serving;
        Ok(())
    }

    /// Revocation stops *new* semantic acceptance, without closing the native
    /// driver or any sibling. Its lookup is at most MAX_INSTANCES (32) probes;
    /// already accepted replies keep their original owner.
    pub fn revoke(&mut self, key: ProducerInstanceKey) -> bool {
        if let Some(instance) = self
            .instances
            .iter_mut()
            .find(|i| i.key() == key && i.current)
        {
            instance.current = false;
            true
        } else {
            false
        }
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
            for instance in &mut self.instances {
                instance.current = false;
            }
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
    }

    fn settle_reply(&mut self, delivery: FiniteDelivery) {
        if let Some(reply) = self.reply.take() {
            assert!(self.settlement.is_none(), "unread finite settlement");
            self.settlement = Some(RawFiniteSettlement {
                recipient: reply.recipient,
                operation: reply.operation,
                effect: reply.effect,
                delivery,
                result: reply.result.map(|_| ()),
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

    fn resolve(&self, ingress: &FiniteIngress<B::Reply>) -> CoreResult<(usize, usize)> {
        if ingress.candidates.len() > self.limits.candidates
            || ingress
                .input
                .data
                .as_ref()
                .is_some_and(|data| data.body.len() > self.limits.input_body_bytes)
        {
            return Err(CoreError::Backpressure(context()));
        }
        // This facet has one reply, not a multicast/aggregation promise.
        let [candidate] = ingress.candidates.as_slice() else {
            return Err(CoreError::UnsupportedOperation(context()));
        };
        let instance_index = self
            .instances
            .iter()
            .position(|i| i.key() == candidate.instance && i.current)
            .ok_or_else(|| CoreError::StaleHandle(context()))?;
        let behavior_index = self.instances[instance_index]
            .behaviors
            .iter()
            .position(|b| b.target == candidate.target && b.operation == candidate.operation)
            .ok_or_else(|| CoreError::UnsupportedOperation(context()))?;
        Ok((instance_index, behavior_index))
    }

    /// One paid binding poll, synchronous cooperative handler call, or cleanup
    /// poll. Pending never discards the reply or changes the first failure.
    pub fn step(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> Poll<RawFiniteState> {
        use RawFiniteState::*;
        if self.state == Closed
            || self.state == InstalledClosed
            || (self.state == Serving && self.settlement.is_some())
        {
            return Poll::Ready(self.state);
        }
        let class = match self.state {
            Dispatching => WorkClass::HandlerSteps,
            Closing => WorkClass::CleanupItems,
            _ => WorkClass::BindingPolls,
        };
        // One atomic, *bounded* lookup: at most every instance and every
        // behavior of the selected instance, plus one handler invocation.
        let units = if self.state == Dispatching {
            (self.limits.instances + self.limits.behaviors_per_instance + 1) as u64
        } else {
            1
        };
        if budget.consume(class, units).is_err() {
            return Poll::Pending;
        }
        if self.state != Closing && !self.pair_is_current() {
            self.fail(CoreError::Validation(context()));
            return Poll::Ready(self.state);
        }
        match self.state {
            Starting => match self.binding.poll_start(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(())) => {
                    self.state = InstalledClosed;
                    Poll::Ready(self.state)
                }
                Poll::Ready(Err(e)) => {
                    self.fail(e);
                    Poll::Ready(self.state)
                }
            },
            Serving => match self.binding.poll_receive(cx) {
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
            },
            Dispatching => {
                let ingress = self.ingress.take().expect("received exchange retained");
                let single = ingress
                    .candidates
                    .first()
                    .filter(|_| ingress.candidates.len() == 1);
                let mut effect = FiniteEffect::NotInvoked;
                let result = self
                    .resolve(&ingress)
                    .and_then(|(instance_index, behavior_index)| {
                        let instance = &self.instances[instance_index];
                        let behavior = &instance.behaviors[behavior_index];
                        // No registry-wide lock, native callback or Binding code runs
                        // around the authorization and application callbacks.
                        behavior.handler.authorize(&ingress.input)?;
                        let ctx = HandlerContext::for_producer(
                            instance.key(),
                            &behavior.target,
                            behavior.operation,
                            self.identity,
                        )?;
                        effect = FiniteEffect::Unknown;
                        let output = behavior.handler.handle(ctx, &ingress.input)?;
                        if !self.output_fits(&output) {
                            return Err(CoreError::Backpressure(context()));
                        }
                        if !raw_finite_property_shape(behavior.operation, &output) {
                            return Err(CoreError::Validation(
                                ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                                    .with_thing(instance.key().slot())
                                    .with_operation(behavior.operation),
                            ));
                        }
                        Ok(output)
                    });
                self.reply = Some(RawFiniteReply {
                    opportunity: ingress.reply,
                    result,
                    effect,
                    recipient: single.map(|c| c.instance),
                    operation: single.map(|c| c.operation),
                });
                self.state = Delivering;
                Poll::Ready(self.state)
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
            InstalledClosed | Closed => unreachable!(),
        }
    }
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
