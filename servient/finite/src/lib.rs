//! Source-free, manually driven Producer ownership. The finite ingress facet
//! and independent Action obligations share one logical instance registry.
//! A separate local publication seam commits a supplied final description;
//! neither execution facet qualifies TD meaning or defines a universal runtime.
#![cfg(feature = "std")]
#![no_std]

extern crate alloc;

mod action;
pub use action::*;
mod publication;
pub use publication::*;

use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    sync::Arc,
    vec::Vec,
};
use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, BindingInputRejection, BindingOperationalError,
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, HandlerContext, InteractionOutput,
    Operation, RetryClass,
    finite::{
        FiniteDelivery, FiniteEffect, FiniteIngress, FiniteIngressBinding, RawFiniteHandler,
        RawFiniteReply, RawFiniteSettlement, raw_finite_property_shape,
    },
    interaction_output_backing,
    producer::{ProducerInstanceAuthority, ProducerInstanceKey},
};
use clinkz_wot_foundation::{MonotonicInstant, WorkBudget, WorkClass};
use core::{
    cell::Cell,
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
pub struct RawProducerInstance {
    authority: ProducerInstanceAuthority,
    // Dispose description before its behavior's last retained funding lease.
    description: Option<Arc<str>>,
    behaviors: Vec<RawFiniteBehavior>,
    actions: Vec<RawActionBehavior>,
    // Last to drop: original accepted work retains the actual shared capacity
    // owner through behavior/native/result disposal and acknowledgement.
    retained_owner: Option<Rc<dyn core::any::Any>>,
}

impl RawProducerInstance {
    pub fn new(authority: ProducerInstanceAuthority, behaviors: Vec<RawFiniteBehavior>) -> Self {
        Self {
            authority,
            description: None,
            behaviors,
            actions: Vec::new(),
            retained_owner: None,
        }
    }

    /// Install Action behavior on this *same* logical authority. An instance
    /// may have only Action behaviors; no finite native driver is implied.
    pub fn with_actions(mut self, actions: Vec<RawActionBehavior>) -> Self {
        self.actions = actions;
        self
    }

    pub fn key(&self) -> ProducerInstanceKey {
        self.authority.key()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn behaviors_described_by(
        &self,
        advertised: impl Fn(clinkz_wot_core::thing::AffordanceSite<'_>, Operation) -> bool,
    ) -> bool {
        self.behaviors.iter().all(|b| match &b.target {
            AffordanceTarget::Property(name) => advertised(
                clinkz_wot_core::thing::AffordanceSite::Property(name),
                b.operation,
            ),
            _ => false,
        }) && self.actions.iter().all(|a| {
            [
                Operation::InvokeAction,
                Operation::QueryAction,
                Operation::CancelAction,
            ]
            .into_iter()
            .filter(|op| a.supports(*op))
            .all(|op| advertised(clinkz_wot_core::thing::AffordanceSite::Action(a.name()), op))
        })
    }

    /// Staged actual behavior coverage, queried during pure TD selection.
    /// Controls require their own registered Action behavior; a mere described
    /// query/cancel Form never creates a working control operation.
    pub fn supports(&self, target: &AffordanceTarget, operation: Operation) -> bool {
        match (target, operation) {
            (
                AffordanceTarget::Action(name),
                Operation::InvokeAction | Operation::QueryAction | Operation::CancelAction,
            ) => self
                .actions
                .iter()
                .any(|b| b.name() == name.as_ref() && b.supports(operation)),
            _ => self
                .behaviors
                .iter()
                .any(|b| b.target == *target && b.operation == operation),
        }
    }
    pub fn supports_property(&self, name: &str, operation: Operation) -> bool {
        self.behaviors.iter().any(|b| {
            b.operation == operation
                && matches!(&b.target, AffordanceTarget::Property(n) if n.as_ref() == name)
        })
    }
    pub fn supports_action(&self, name: &str, operation: Operation) -> bool {
        self.actions
            .iter()
            .any(|b| b.name() == name && b.supports(operation))
    }

    /// Source-independent retained description. Raw callers remain responsible
    /// for qualification; the preparation facade supplies the checked value.
    pub fn with_description(mut self, description: Arc<str>) -> Self {
        self.description = Some(description);
        self
    }

    /// Controlled layouts and description/target backing. Application handler
    /// indirect storage remains a trusted, separately supplied declaration.
    pub fn backing_bytes(&self) -> Option<usize> {
        let mut bytes = size_of::<Self>()
            .checked_add(2 * size_of::<usize>())?
            .checked_add(self.behaviors.capacity() * size_of::<RawFiniteBehavior>())?
            .checked_add(self.actions.capacity() * size_of::<RawActionBehavior>())?;
        if let Some(description) = &self.description {
            bytes = bytes.checked_add(description.len() + 2 * size_of::<usize>())?;
        }
        for behavior in &self.behaviors {
            bytes = bytes.checked_add(core::mem::size_of_val(behavior.handler.as_ref()))?;
            if let AffordanceTarget::Property(name) = &behavior.target {
                bytes = bytes.checked_add(name.len() + 2 * size_of::<usize>())?;
            }
        }
        for behavior in &self.actions {
            bytes = bytes.checked_add(behavior.backing_bytes())?;
        }
        Some(bytes)
    }
}

/// Fixed registration/lookup envelope at the logical owner. Construction and
/// replacement validate externally built behavior before changing authority.
#[derive(Clone, Copy, Debug)]
pub struct RawProducerRegistryLimits {
    pub instances: usize,
    pub behaviors_per_instance: usize,
    /// Maximum UTF-8 bytes per registered or accepted native target name.
    pub target_name_bytes: usize,
}

struct InstanceEntry {
    // This is the one logical permission cell, shared with its publication
    // guard so lost roots can revoke without a lock or native callback.
    current: Rc<Cell<bool>>,
    participations: Vec<Weak<()>>,
    // Dispose registry/view backing before its last actual reservation owner.
    instance: Rc<RawProducerInstance>,
}

struct AcceptedBehavior {
    instance: Rc<RawProducerInstance>,
    behavior: usize,
}

/// One logical owner shared by independently typed native runtimes. Exclusive
/// manual access orders acceptance, revocation and replacement without locks.
/// This Host facet is single-threaded; retention references confer no authority.
pub struct RawProducerServient {
    instances: Vec<InstanceEntry>,
    issuer: Rc<()>,
    limits: RawProducerRegistryLimits,
    actions: Option<ActionStore>,
}

impl RawProducerServient {
    pub fn try_new(
        instances: Vec<RawProducerInstance>,
        limits: RawProducerRegistryLimits,
    ) -> Result<Self, BindingInputRejection<Vec<RawProducerInstance>>> {
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
        let mut entries = Vec::with_capacity(limits.instances);
        entries.extend(instances.into_iter().map(|instance| InstanceEntry {
            instance: Rc::new(instance),
            current: Rc::new(Cell::new(true)),
            participations: Vec::new(),
        }));
        Ok(Self {
            instances: entries,
            issuer: Rc::new(()),
            limits,
            actions: None,
        })
    }

    fn valid_instance(instance: &RawProducerInstance, limits: RawProducerRegistryLimits) -> bool {
        (!instance.behaviors.is_empty() || !instance.actions.is_empty())
            && instance.behaviors.len() + instance.actions.len() <= limits.behaviors_per_instance
            && instance.actions.iter().enumerate().all(|(j, action)| {
                action.name().len() <= limits.target_name_bytes
                    && !instance.actions[..j]
                        .iter()
                        .any(|prior| prior.name() == action.name())
            })
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

    /// Stop new acceptance everywhere. At most 32 key probes and 128 Action
    /// flags; no native callback, handler destruction or synchronous application
    /// cancellation occurs here. Funded steps request cancellation afterwards.
    pub fn revoke(&mut self, key: ProducerInstanceKey) -> bool {
        if let Some(entry) = self
            .instances
            .iter_mut()
            .find(|entry| entry.instance.key() == key && entry.current.get())
        {
            entry.current.set(false);
            if let Some(actions) = &mut self.actions {
                actions.revoke(key);
            }
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
        replacement: RawProducerInstance,
    ) -> Result<(), BindingInputRejection<RawProducerInstance>> {
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
        self.instances[index.expect("checked entry")]
            .current
            .set(false);
        self.instances[index.expect("checked entry")] = InstanceEntry {
            instance: Rc::new(replacement),
            current: Rc::new(Cell::new(true)),
            participations: Vec::new(),
        };
        if let Some(actions) = &mut self.actions {
            actions.revoke(expected);
        }
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
            .position(|entry| entry.instance.key() == candidate.instance && entry.current.get())
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

/// Output checking for this participant. Raw mode claims no schema conformance
/// and cannot satisfy qualified publication; explicit raw acceptance uses
/// `RawFiniteRuntime::activate_raw`. Required refuses an absent/lost service.
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
    participation: ProducerParticipationId,
    retained_owner: Option<Rc<dyn core::any::Any>>,
}

impl<B: FiniteIngressBinding> RawFiniteRuntime<B> {
    /// Associate a native owner with one Servient, without moving or copying
    /// logical authority/behavior. Rejection returns the complete native pair.
    /// `deadline` bounds the caller's logical interaction/lifetime envelope;
    /// native requests retain their independent Binding-owned deadline.
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        servient: &RawProducerServient,
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
            participation: ProducerParticipationId::new(),
            retained_owner: None,
        })
    }

    pub fn state(&self) -> RawFiniteState {
        self.state
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }

    /// A final qualification loan grants no native mutation or dispatch power.
    pub fn closed_binding(&self) -> Option<&B> {
        (self.state == RawFiniteState::InstalledClosed).then_some(&self.binding)
    }

    /// Fence locally while retaining the first cause and complete cleanup owner.
    pub fn fence(&mut self, cause: CoreError) {
        self.fail(cause);
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
        }) && interaction_output_backing(output)
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
