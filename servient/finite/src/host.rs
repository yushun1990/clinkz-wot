//! Source-free, manually driven Producer ownership. The finite ingress facet
//! and independent Action obligations share one logical instance registry.
//! A separate local publication seam commits a supplied final description;
//! neither execution facet qualifies TD meaning or defines a universal runtime.

#[path = "consumer.rs"]
pub mod consumer;
#[path = "consumer_action.rs"]
pub mod consumer_action;
#[path = "consumer_execution.rs"]
pub mod consumer_execution;
#[path = "consumer_notification.rs"]
pub mod consumer_notification;
#[path = "notification.rs"]
pub mod notification;

#[path = "action.rs"]
mod action;
pub use action::*;
#[path = "notification_publication.rs"]
mod notification_publication;
pub use notification_publication::*;
#[path = "publication.rs"]
mod publication;
pub use publication::*;
#[path = "producer_capacity.rs"]
mod producer_capacity;
pub use producer_capacity::*;

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
const MAX_HANDLERS_PER_INSTANCE: usize = 16;
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
    action_query: Option<Box<dyn clinkz_wot_core::action::ActionQueryPolicy>>,
    notification_sources: Vec<ProducerNotificationSource>,
    notification_work: Cell<usize>,
    // Includes unread terminal Action entries. Taking a settlement removes
    // its entry; the returned settlement still pins this instance's backing.
    action_entries: Cell<usize>,
    action_result_accounts: Option<Arc<clinkz_wot_core::host_accounts::HostResourceAccounts>>,
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
            action_query: None,
            notification_sources: Vec::new(),
            notification_work: Cell::new(0),
            action_entries: Cell::new(0),
            action_result_accounts: None,
            retained_owner: None,
        }
    }

    /// Install Action behavior on this *same* logical authority. An instance
    /// may have only Action behaviors; no finite native driver is implied.
    pub fn with_actions(mut self, actions: Vec<RawActionBehavior>) -> Self {
        self.actions = actions;
        self
    }

    /// Register permission for a root invocation-status query on this instance.
    pub fn with_action_query(
        mut self,
        policy: impl clinkz_wot_core::action::ActionQueryPolicy + 'static,
    ) -> Self {
        self.action_query = Some(Box::new(policy));
        self
    }

    /// Prepay one lifetime cleanup item per original source before taking its
    /// table. The last instance owner can be an unread result after replacement.
    pub fn with_notification_sources(
        mut self,
        sources: Vec<ProducerNotificationSource>,
        lifetime: &mut WorkBudget,
    ) -> Result<Self, BindingInputRejection<(Self, Vec<ProducerNotificationSource>)>> {
        if lifetime
            .consume(WorkClass::CleanupItems, sources.len() as u64)
            .is_err()
        {
            let error = CoreError::Backpressure(context().with_thing(self.key().slot()));
            return Err(BindingInputRejection::new(
                (self, sources),
                BindingOperationalError::new(error),
            ));
        }
        self.notification_sources = sources;
        Ok(self)
    }
    fn retains_publication_work(&self) -> bool {
        self.notification_work.get() != 0 || self.action_entries.get() != 0
    }
    pub fn behavior_count(&self) -> usize {
        self.behaviors.len()
            + self.actions.len()
            + usize::from(self.action_query.is_some())
            + self.notification_sources.len()
    }
    pub fn supports_notification(&self, target: &AffordanceTarget, operation: Operation) -> bool {
        self.notification_sources.iter().any(|s| {
            s.target() == *target
                && (operation == s.kind().start_operation()
                    || operation == s.kind().stop_operation())
        })
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
            AffordanceTarget::Thing => {
                advertised(clinkz_wot_core::thing::AffordanceSite::Thing, b.operation)
            }
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
        }) && (self.action_query.is_none()
            || advertised(
                clinkz_wot_core::thing::AffordanceSite::Thing,
                Operation::QueryAllActions,
            ))
            && self.notification_sources.iter().all(|e| {
                advertised(
                    match e {
                        ProducerNotificationSource::AllProperties
                        | ProducerNotificationSource::AllEvents => {
                            clinkz_wot_core::thing::AffordanceSite::Thing
                        }
                        ProducerNotificationSource::Property(n) => {
                            clinkz_wot_core::thing::AffordanceSite::Property(n)
                        }
                        ProducerNotificationSource::Event(n) => {
                            clinkz_wot_core::thing::AffordanceSite::Event(n)
                        }
                    },
                    e.kind().start_operation(),
                )
            })
    }

    /// Staged actual behavior coverage, queried during pure TD selection.
    /// Controls require their own registered Action behavior; a mere described
    /// query/cancel Form never creates a working control operation.
    pub fn supports(&self, target: &AffordanceTarget, operation: Operation) -> bool {
        match (target, operation) {
            (AffordanceTarget::Thing, Operation::QueryAllActions) => self.action_query.is_some(),
            (
                _,
                Operation::ObserveProperty
                | Operation::UnobserveProperty
                | Operation::ObserveAllProperties
                | Operation::UnobserveAllProperties
                | Operation::SubscribeEvent
                | Operation::UnsubscribeEvent
                | Operation::SubscribeAllEvents
                | Operation::UnsubscribeAllEvents,
            ) => self.supports_notification(target, operation),
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
        (matches!(
            operation,
            Operation::ObserveProperty | Operation::UnobserveProperty
        ) && self
            .notification_sources
            .iter()
            .any(|s| matches!(s, ProducerNotificationSource::Property(n) if n.as_ref() == name)))
            || self.behaviors.iter().any(|b| {
                b.operation == operation
                    && matches!(&b.target, AffordanceTarget::Property(n) if n.as_ref() == name)
            })
    }
    pub fn supports_event(&self, name: &str, operation: Operation) -> bool {
        matches!(
            operation,
            Operation::SubscribeEvent | Operation::UnsubscribeEvent
        ) && self
            .notification_sources
            .iter()
            .any(|s| matches!(s, ProducerNotificationSource::Event(n) if n.as_ref() == name))
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
            .checked_add(self.actions.capacity() * size_of::<RawActionBehavior>())?
            .checked_add(
                self.notification_sources.capacity() * size_of::<ProducerNotificationSource>(),
            )?;
        if let Some(description) = &self.description {
            bytes = bytes.checked_add(description.len() + 2 * size_of::<usize>())?;
        }
        for behavior in &self.behaviors {
            bytes = bytes.checked_add(core::mem::size_of_val(behavior.handler.as_ref()))?;
            if let AffordanceTarget::Property(name) = &behavior.target {
                bytes = bytes.checked_add(name.len() + 2 * size_of::<usize>())?;
            }
        }
        if let Some(policy) = &self.action_query {
            bytes = bytes.checked_add(core::mem::size_of_val(policy.as_ref()))?;
        }
        for behavior in &self.actions {
            bytes = bytes.checked_add(behavior.backing_bytes())?;
        }
        for event in &self.notification_sources {
            bytes = bytes.checked_add(event.backing_bytes())?;
        }
        Some(bytes)
    }
}

/// Fixed registration/lookup envelope at the logical owner. Construction and
/// replacement validate externally built behavior before changing authority.
#[derive(Clone, Copy, Debug)]
pub struct RawProducerRegistryLimits {
    pub instances: usize,
    /// Total caller-provisioned registrations, including notification sources.
    pub behaviors_per_instance: usize,
    /// Maximum UTF-8 bytes per registered or accepted native target name.
    pub target_name_bytes: usize,
}

impl RawProducerRegistryLimits {
    // Finite and Action dispatch scan only executable handlers. A wider table
    // of notification sources does not increase their atomic lookup envelope.
    fn handler_limit(self) -> usize {
        self.behaviors_per_instance.min(MAX_HANDLERS_PER_INSTANCE)
    }
}

struct InstanceEntry {
    permission: Rc<PublicationPermission>,
    // None is explicitly raw; published Actions have captured routes/services.
    action_routes: Option<Vec<CapturedActionRoute>>,
    notification_routes: Option<Vec<CapturedNotificationRoute>>,
    // Dispose registry/view backing before its last actual reservation owner.
    instance: Rc<RawProducerInstance>,
}

struct AcceptedBehavior {
    permission: Rc<PublicationPermission>,
    instance: Rc<RawProducerInstance>,
    behavior: usize,
}

/// One-way revocation of an original Producer incarnation. This handle grants
/// no dispatch, replacement or publication authority and cannot address a reused
/// slot. Revocation clears permission and withdraws its bounded set of gates,
/// including when called from an application callback.
/// The weak cell and its original capacity owner are retained together; cloning
/// a handle creates no allocation and dropping the last one releases its lease.
#[derive(Clone)]
pub struct ProducerRevocation {
    key: ProducerInstanceKey,
    permission: Weak<PublicationPermission>,
    _retained_owner: Option<Rc<dyn core::any::Any>>,
}
impl ProducerRevocation {
    pub fn key(&self) -> ProducerInstanceKey {
        self.key
    }
    /// Does not invoke a handler, native callback, waker or registry mutation.
    /// Original pending work observes revocation on its next funded step.
    pub fn revoke(&self) {
        if let Some(permission) = self.permission.upgrade() {
            permission.revoke();
        }
    }
}

/// One logical owner shared by independently typed native runtimes. Exclusive
/// manual runtime access orders acceptance; passive revocation needs no registry
/// borrow, while replacement retains exclusive owner access without locks.
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
            && limits.behaviors_per_instance != 0
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
            permission: Rc::new(PublicationPermission::raw()),
            action_routes: None,
            notification_routes: None,
        }));
        Ok(Self {
            instances: entries,
            issuer: Rc::new(()),
            limits,
            actions: None,
        })
    }

    fn valid_instance(instance: &RawProducerInstance, limits: RawProducerRegistryLimits) -> bool {
        let behaviors = instance.behavior_count();
        (1..=limits.behaviors_per_instance).contains(&behaviors)
            && behaviors - instance.notification_sources.len() <= limits.handler_limit()
            && instance
                .notification_sources
                .iter()
                .enumerate()
                .all(|(i, e)| {
                    e.name().is_none_or(|n| n.len() <= limits.target_name_bytes)
                        && !instance.notification_sources[..i]
                            .iter()
                            .any(|prior| prior.target() == e.target() && prior.kind() == e.kind())
                })
            && instance.actions.iter().enumerate().all(|(j, action)| {
                action.name().len() <= limits.target_name_bytes
                    && !instance.actions[..j]
                        .iter()
                        .any(|prior| prior.name() == action.name())
            })
            && instance.behaviors.iter().enumerate().all(|(j, behavior)| {
                (matches!(
                    behavior.operation,
                    Operation::ReadProperty | Operation::WriteProperty
                ) && matches!(&behavior.target, AffordanceTarget::Property(name)
                        if name.len() <= limits.target_name_bytes)
                    || matches!(
                        behavior.operation,
                        Operation::ReadAllProperties
                            | Operation::ReadMultipleProperties
                            | Operation::WriteAllProperties
                            | Operation::WriteMultipleProperties
                    ) && behavior.target == AffordanceTarget::Thing)
                    && !instance.behaviors[..j].iter().any(|prior| {
                        prior.operation == behavior.operation && prior.target == behavior.target
                    })
            })
    }

    /// Stop new acceptance after a bounded key lookup and original-member check.
    /// Mark at most 128 Action flags. Funded steps request cancellation afterwards.
    pub fn revoke(&mut self, key: ProducerInstanceKey) -> bool {
        if let Some(entry) = self
            .instances
            .iter_mut()
            .find(|entry| entry.instance.key() == key && entry.permission.is_current())
        {
            entry.permission.revoke();
            if entry.instance.action_entries.get() != 0
                && let Some(actions) = &mut self.actions
            {
                actions.revoke(key);
            }
            true
        } else {
            false
        }
    }

    /// Capture callback-safe revocation for this exact current incarnation.
    /// Managed publication funding survives until every captured handle drops,
    /// including after registry replacement and native/result disposal.
    pub fn revocation(&self, key: ProducerInstanceKey) -> Option<ProducerRevocation> {
        self.instances
            .iter()
            .find(|entry| entry.instance.key() == key && entry.permission.is_current())
            .map(|entry| ProducerRevocation {
                key,
                permission: Rc::downgrade(&entry.permission),
                _retained_owner: entry.instance.retained_owner.clone(),
            })
    }

    /// Validate the complete successor, then atomically replace the expected
    /// incarnation (current or revoked) in the same numeric slot. Old accepted
    /// work retains its original instance; no Binding is reconfigured or closed.
    pub fn replace(
        &mut self,
        expected: ProducerInstanceKey,
        replacement: RawProducerInstance,
    ) -> Result<(), BindingInputRejection<RawProducerInstance>> {
        let Some(index) = self
            .instances
            .iter()
            .position(|entry| entry.instance.key() == expected)
        else {
            return Err(rejection(replacement));
        };
        if replacement.key() == expected
            || replacement.key().slot().slot() != expected.slot().slot()
            || !Self::valid_instance(&replacement, self.limits)
        {
            return Err(rejection(replacement));
        }
        let entry = InstanceEntry {
            instance: Rc::new(replacement),
            permission: Rc::new(PublicationPermission::raw()),
            action_routes: None,
            notification_routes: None,
        };
        self.instances[index].permission.revoke();
        let displaced = core::mem::replace(&mut self.instances[index], entry);
        if displaced.instance.action_entries.get() != 0
            && let Some(actions) = &mut self.actions
        {
            actions.revoke(expected);
        }
        drop(displaced);
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
            AffordanceTarget::Property(_) | AffordanceTarget::Thing => {}
            _ => return Err(CoreError::UnsupportedOperation(context())),
        }
        let instance_index = self
            .instances
            .iter()
            .position(|entry| {
                entry.instance.key() == candidate.instance && entry.permission.is_current()
            })
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
        &self,
        runtime: &mut RawFiniteRuntime<B>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> CoreResult<Poll<RawFiniteState>> {
        if runtime.issuer.as_ptr() != Rc::as_ptr(&self.issuer) {
            return Err(CoreError::StaleHandle(context()));
        }
        // Native maintenance has its own allowance even while an offered
        // request waits for handler credit. A newly received offer dispatches
        // on a later turn.
        let had_candidate = runtime.state == RawFiniteState::Dispatching;
        let progress = runtime.step(cx, budget, now);
        if !had_candidate || runtime.state != RawFiniteState::Dispatching {
            return Ok(progress);
        }
        let mut units = (self.limits.instances
            + self.limits.handler_limit() * (self.limits.target_name_bytes + 1)
            + 1) as u64;
        if runtime.validation == FiniteValidation::Required {
            // Prepay the complete declared value/media traversal, before
            // behavior entry or a codec callback. Linked validators still owe
            // bounded cooperative work; this is not callback preemption.
            let bytes = runtime
                .limits
                .output_media_capacity
                .checked_mul(2)
                .and_then(|media| runtime.limits.output_body_bytes.checked_add(media))
                .and_then(|n| n.checked_add(runtime.limits.input_body_bytes))
                .and_then(|n| u64::try_from(n).ok())
                .and_then(|n| n.checked_add(runtime.validation_work))
                .and_then(|n| units.checked_add(n));
            let Some(total) = bytes else {
                runtime.fail(CoreError::Backpressure(context()));
                return Ok(Poll::Ready(runtime.state));
            };
            units = total;
        }
        let Some(units) = runtime
            .security_work
            .checked_mul(2)
            .and_then(|w| units.checked_add(w))
        else {
            runtime.fail(CoreError::Backpressure(context()));
            return Ok(Poll::Ready(runtime.state));
        };
        if budget.consume(WorkClass::HandlerSteps, units).is_err() {
            return Ok(Poll::Pending);
        }
        if let Err(error) = runtime.check_installed_ingress() {
            runtime.fail(error);
            return Ok(Poll::Ready(runtime.state));
        }
        let mut ingress = runtime.ingress.take().expect("received exchange retained");
        let single = ingress
            .candidates
            .first()
            .filter(|_| ingress.candidates.len() == 1);
        let recipient = single.map(|candidate| candidate.instance);
        let operation = single.map(|candidate| candidate.operation);
        let mut effect = FiniteEffect::NotInvoked;
        let mut retirement_cause = None;
        let result = self.resolve(&ingress, runtime.limits).and_then(|(i, b)| {
            let instance = &self.instances[i].instance;
            let behavior = &instance.behaviors[b];
            let security_revision = runtime.binding.security_revision();
            if runtime.security_work != 0 {
                ingress.input.principal = runtime.binding.verify_ingress(&ingress)?;
            }
            if runtime.validation == FiniteValidation::Required {
                runtime
                    .binding
                    .validator()
                    .ok_or_else(|| CoreError::Validation(context()))?
                    .validate_input(&ingress.candidates[0], &ingress.input)?;
            }
            // Validation is a trusted callback, but fresh authorization still
            // precedes handler entry and cannot be replaced by native parsing.
            let policy_revision = behavior.handler.authorization_revision();
            behavior.handler.authorize(&ingress.input)?;
            // Policy is application code and can invalidate an externally
            // mutable provider. Recheck before behavior entry, then retain the
            // actual service loan through the synchronous callback/validation.
            if !runtime.pair_is_current() {
                return Err(CoreError::Validation(context()));
            }
            if runtime.security_work != 0
                && runtime.binding.verify_ingress(&ingress)? != ingress.input.principal
            {
                return Err(CoreError::Security {
                    reason: clinkz_wot_core::SecurityFailureReason::InvalidCredentials,
                    context: context(),
                });
            }
            let validator = if runtime.validation == FiniteValidation::Required {
                Some(
                    runtime
                        .binding
                        .validator()
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
            )?
            .with_response_contract(match validator {
                Some(service) => service.response_contract(&ingress.candidates[0])?,
                None => None,
            });
            // Verification, validation and application policy can invalidate
            // publication or captured services. Check after the last callback,
            // immediately before acquiring behavior authority. The original
            // ingress stays owned through refusal and native retirement.
            let current = if !runtime.pair_is_current() {
                Err(CoreError::Validation(context()))
            } else if runtime.binding.security_revision() != security_revision
                || behavior.handler.authorization_revision() != policy_revision
            {
                Err(CoreError::Security {
                    reason: clinkz_wot_core::SecurityFailureReason::AuthorizationDenied,
                    context: context(),
                })
            } else if let Err(error) = runtime.binding.check_ingress() {
                Err(error)
            } else if !runtime.ingress_gate.is_open() || !self.instances[i].permission.is_current()
            {
                Err(CoreError::StaleHandle(context()))
            } else {
                Ok(())
            };
            if let Err(error) = current {
                retirement_cause = Some(error.clone());
                return Err(error);
            }
            // Acceptance is ordered by exclusive access to this runtime. No lock,
            // native callback, allocation or re-lookup surrounds application code.
            runtime.accepted_instance = Some(AcceptedBehavior {
                permission: Rc::clone(&self.instances[i].permission),
                instance: Rc::clone(instance),
                behavior: b,
            });
            effect = FiniteEffect::Unknown;
            let output = behavior.handler.handle(ctx, &ingress.input)?;
            if !runtime.output_fits(&output) {
                return Err(CoreError::Backpressure(context()));
            }
            if !(if validator.is_some() {
                clinkz_wot_core::finite::finite_handler_shape(&output)
            } else {
                raw_finite_property_shape(behavior.operation, &output)
            }) {
                return Err(CoreError::Validation(
                    ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                        .with_thing(instance.key().slot())
                        .with_operation(behavior.operation),
                ));
            }
            if let Some(validator) = validator {
                // The handler may change interior service policy despite this
                // retained loan. Check this exact validator's prepaid bound.
                // The loan still owes late-result validation if lookup
                // availability is lost; the delivery step fences that loss.
                if validator.additional_validation_work() != runtime.validation_work {
                    let error = CoreError::Validation(
                        ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                            .with_thing(instance.key().slot())
                            .with_operation(behavior.operation),
                    );
                    retirement_cause = Some(error.clone());
                    return Err(error);
                }
                validator.validate_output(&ingress.candidates[0], &ingress.input, &output)?;
            }
            if output.status() == clinkz_wot_core::InteractionStatus::Ok
                && matches!(
                    behavior.operation,
                    Operation::WriteProperty
                        | Operation::WriteAllProperties
                        | Operation::WriteMultipleProperties
                )
            {
                effect = if output
                    .metadata()
                    .property_write_report()
                    .is_some_and(|r| !r.is_complete())
                {
                    FiniteEffect::Reported
                } else {
                    FiniteEffect::Completed
                };
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
        if let Some(error) = retirement_cause {
            // The original input, reply and any accepted behavior stay owned
            // through native retirement; invalidation grants no delivery.
            runtime.fail(error);
        } else {
            runtime.state = RawFiniteState::Delivering;
        }
        Ok(Poll::Ready(runtime.state))
    }
}

/// Maximum exchange opportunity and Engine-retained result backing. Output is
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

/// Input and output checking for this participant. Raw mode claims no schema conformance
/// and cannot satisfy qualified publication; explicit raw acceptance uses
/// `RawFiniteRuntime::activate_raw`. Required refuses an absent/lost service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteValidation {
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
    /// The original reply is expiring. Its native owner still holds the lane.
    RetiringReply,
    Closing,
    Closed,
}

/// Persistent typed native participation. One exclusive exchange may be active
/// on this driver. By default its single reserved opportunity also retains one
/// unread result. `with_capacity` instead admits work from an independent shared
/// bank and permits further exchanges while original funded outcomes are unread.
/// Refusals still travel through the original Binding reply opportunity.
pub struct RawFiniteRuntime<B: FiniteIngressBinding> {
    // Complete pending work drops before the physical driver on local disposal.
    ingress: Option<FiniteIngress<B::Reply>>,
    reply: Option<RawFiniteReply<B::Reply>>,
    settlement: Option<RawFiniteSettlement>,
    completed: producer_capacity::ProducerFiniteQueue,
    binding: B,
    // Survives native disposal and ends only at settlement acknowledgement.
    accepted_instance: Option<AcceptedBehavior>,
    issuer: Weak<()>,
    identity: (BindingId, BindingGeneration),
    configuration: B::Configuration,
    limits: RawFiniteLimits,
    state: RawFiniteState,
    ingress_gate: Rc<ProducerIngressGate>,
    first_cause: Option<CoreError>,
    // Moves into this reply's settlement even if participation later fails.
    exchange_cause: Option<CoreError>,
    validation: FiniteValidation,
    validation_work: u64,
    security_work: u64,
    deadline: Deadline,
    io_deadline: Deadline,
    last_now: Option<MonotonicInstant>,
    // Starting may retain partial native effects even when poll_start returned
    // Pending. State alone cannot distinguish timing admission from live work.
    native_started: bool,
    participation: ProducerParticipationId,
    retained_owner: Option<Rc<dyn core::any::Any>>,
    // Active input/result/cleanup funding outlives native driver/config disposal.
    work: Option<Box<ProducerFiniteSettlement>>,
    capacity: Option<ProducerFiniteCapacity>,
}

impl<B: FiniteIngressBinding> RawFiniteRuntime<B> {
    /// Associate a native owner with one Servient, without moving or copying
    /// logical authority/behavior. Rejection returns the complete native pair.
    /// `deadline` bounds this participation's lifetime. Each native request
    /// retains its independent Binding-owned deadline.
    pub fn try_new(
        binding: B,
        configuration: B::Configuration,
        servient: &RawProducerServient,
        limits: RawFiniteLimits,
        validation: FiniteValidation,
        deadline: Deadline,
    ) -> Result<Self, BindingInputRejection<(B, B::Configuration)>> {
        let identity = binding.identity();
        let security_work = binding.security_work();
        let validation_work = binding
            .validator()
            .map_or(0, |v| v.additional_validation_work());
        let valid = binding.configuration() == &configuration
            && limits.candidates > 0
            && limits.output_retained_bytes >= size_of::<InteractionOutput>()
            && (validation == FiniteValidation::Raw || binding.validator().is_some());
        if !valid {
            return Err(rejection((binding, configuration)));
        }
        Ok(Self {
            ingress: None,
            reply: None,
            work: None,
            completed: producer_capacity::ProducerFiniteQueue::default(),
            binding,
            accepted_instance: None,
            issuer: Rc::downgrade(&servient.issuer),
            identity,
            configuration,
            limits,
            state: RawFiniteState::Starting,
            ingress_gate: Rc::new(ProducerIngressGate::default()),
            first_cause: None,
            exchange_cause: None,
            settlement: None,
            validation,
            validation_work,
            security_work,
            deadline,
            io_deadline: Deadline::NONE,
            last_now: None,
            native_started: false,
            participation: ProducerParticipationId::new(),
            retained_owner: None,
            capacity: None,
        })
    }

    /// Select independently funded finite work before native setup or managed
    /// admission. Publication/readiness needs no vacant work slot. The capacity
    /// must belong to this logical owner and cover every declared exchange bound.
    /// Rejection returns this complete unstarted participation.
    pub fn with_capacity(
        mut self,
        capacity: ProducerFiniteCapacity,
    ) -> Result<Self, BindingInputRejection<Self>> {
        if self.state != RawFiniteState::Starting
            || self.native_started
            || self.retained_owner.is_some()
            || !capacity.supports(&self.issuer, self.limits)
        {
            return Err(rejection(self));
        }
        self.capacity = Some(capacity);
        Ok(self)
    }

    pub fn state(&self) -> RawFiniteState {
        if self.state == RawFiniteState::InstalledClosed && self.ingress_gate.is_open() {
            RawFiniteState::Serving
        } else {
            self.state
        }
    }
    /// First cause closing this participation. An isolated request timeout
    /// belongs to the original settlement.
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

    /// Acknowledge the default dedicated opportunity, before its next receive.
    /// Participants using `with_capacity` use `take_retained_settlement` instead.
    /// Neither an error nor close proves a write absent.
    pub fn take_settlement(&mut self) -> Option<RawFiniteSettlement> {
        let settlement = self.settlement.take();
        if settlement.is_some() {
            self.accepted_instance = None;
        }
        settlement
    }

    /// Acknowledge the oldest outcome admitted from explicit capacity. The
    /// returned owner retains original result/instance/publication funding and
    /// its retained-work debit until disposal. Execution credit was already
    /// released at native settlement; taking this record grants no native right.
    pub fn take_retained_settlement(&mut self) -> Option<Box<ProducerFiniteSettlement>> {
        ProducerFiniteSettlement::take(&mut self.completed)
    }

    /// Locally enable raw acceptance after native readiness; this does not
    /// publish or qualify a TD, even with Required output validation. No native
    /// callback or handler runs here.
    pub fn activate_raw(&mut self) -> CoreResult<()> {
        // A managed composed handoff cannot be opened through the raw escape
        // hatch before its complete final TD has committed. Raw publication
        // callers remain responsible for not interleaving these two modes.
        if self.state != RawFiniteState::InstalledClosed || self.retained_owner.is_some() {
            return Err(CoreError::Validation(context()));
        }
        if self.issuer.strong_count() == 0 {
            let error = CoreError::StaleHandle(context());
            self.fail(error.clone());
            return Err(error);
        }
        if !self.pair_is_current() || !self.ingress_gate.is_staged() {
            let error = CoreError::Validation(context());
            self.fail(error.clone());
            return Err(error);
        }
        self.state = RawFiniteState::Serving;
        self.ingress_gate.open();
        Ok(())
    }

    pub fn begin_close(&mut self) {
        let cause = (self.ingress.is_some() || self.reply.is_some()).then(|| {
            CoreError::Cancelled(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never))
        });
        self.close_participation(cause);
    }

    fn fail(&mut self, error: CoreError) {
        self.close_participation(Some(error));
    }

    fn close_participation(&mut self, cause: Option<CoreError>) {
        self.ingress_gate.withdraw();
        if let Some(cause) = cause {
            if self.reply.is_some() {
                self.exchange_cause.get_or_insert_with(|| cause.clone());
            }
            self.first_cause.get_or_insert(cause);
        }
        if self.state != RawFiniteState::Closed {
            self.state = RawFiniteState::Closing;
        }
    }

    fn pair_is_current(&self) -> bool {
        self.binding.identity() == self.identity
            && self.binding.configuration() == &self.configuration
            && self.binding.security_work() == self.security_work
            && (self.validation == FiniteValidation::Raw
                || self
                    .binding
                    .validator()
                    .is_some_and(|v| v.additional_validation_work() == self.validation_work))
    }

    fn check_installed_ingress(&self) -> CoreResult<()> {
        if !self.pair_is_current() {
            return Err(CoreError::Validation(context()));
        }
        self.binding.check_ingress()
    }

    fn check_time(&mut self, now: MonotonicInstant) {
        if self.state == RawFiniteState::Closed {
            return;
        }
        let monotonic_failure = self
            .last_now
            .is_some_and(|last| now.checked_duration_since(last).is_none());
        let cleanup = matches!(
            self.state,
            RawFiniteState::Closing | RawFiniteState::RetiringReply
        );
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
        let lifetime_elapsed = self.deadline.checked_is_elapsed_at(now);
        let request_elapsed = self.io_deadline.checked_is_elapsed_at(now);
        let comparisons = [
            (lifetime_elapsed, interaction_phase),
            (request_elapsed, native_phase),
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
        } else if lifetime_elapsed == Some(true) {
            self.fail(CoreError::TimedOut(timing_context(interaction_phase)));
        } else if request_elapsed == Some(true) && self.state != RawFiniteState::RetiringReply {
            let timeout = CoreError::TimedOut(timing_context(native_phase));
            if self.state == RawFiniteState::Delivering {
                self.exchange_cause = Some(timeout);
                self.state = RawFiniteState::RetiringReply;
            } else {
                self.fail(timeout);
            }
        }
        self.last_now = Some(now);
    }

    fn settle_reply(&mut self, delivery: FiniteDelivery) {
        if let Some(reply) = self.reply.take() {
            assert!(self.settlement.is_none(), "unread finite settlement");
            let settlement = RawFiniteSettlement {
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
                first_cause: self.exchange_cause.take(),
                effect: reply.effect,
                delivery,
                result: reply.result,
            };
            // Dispose the original native input/opportunity before refunding
            // active/ingress credit, including a linked Reply destructor.
            drop(reply.ingress);
            self.retain_settlement(settlement);
        }
    }

    fn settle_ingress(&mut self, delivery: FiniteDelivery) {
        if let Some(ingress) = self.ingress.take() {
            assert!(self.settlement.is_none(), "unread finite settlement");
            let single = ingress
                .candidates
                .first()
                .filter(|_| ingress.candidates.len() == 1);
            let settlement = RawFiniteSettlement {
                binding: self.identity,
                target: None,
                recipient: single.map(|c| c.instance),
                operation: single.map(|c| c.operation),
                first_cause: self.first_cause.clone(),
                effect: FiniteEffect::NotInvoked,
                delivery,
                result: Err(self.first_cause.clone().expect("close has a first cause")),
            };
            drop(ingress);
            self.retain_settlement(settlement);
        }
    }

    fn retain_settlement(&mut self, settlement: RawFiniteSettlement) {
        if self.capacity.is_some() {
            self.work.take().expect("prepaid finite receive").complete(
                settlement,
                self.accepted_instance.take(),
                self.retained_owner.clone(),
                &mut self.completed,
            );
        } else {
            self.settlement = Some(settlement);
        }
    }

    fn finish_retirement(&mut self, delivery: FiniteDelivery) {
        self.settle_ingress(delivery);
        self.settle_reply(delivery);
        self.work = None;
        self.io_deadline = Deadline::NONE;
        if self.state == RawFiniteState::Closing {
            self.state = RawFiniteState::Closed;
            return;
        }
        // The retirement callback and original reply destructor have now run.
        // Neither can grant fresh authority or certify the installed service.
        if let Err(error) = self.check_installed_ingress() {
            self.fail(error);
        } else if self.issuer.strong_count() == 0 {
            self.fail(CoreError::StaleHandle(context()));
        } else if !self.ingress_gate.is_open() {
            self.begin_close();
        } else {
            self.state = RawFiniteState::Serving;
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
        if self.state == InstalledClosed && self.ingress_gate.is_open() {
            self.state = Serving;
        }
        // Local fencing requires no native work credit. Cleanup still does.
        self.check_time(now);
        if self.state == Dispatching && !self.ingress_gate.is_open() {
            self.fail(CoreError::StaleHandle(context()));
            return Poll::Ready(self.state);
        }
        if matches!(self.state, Closed | InstalledClosed) {
            return Poll::Ready(self.state);
        }
        // Serving is only a native state, not a lasting grant of publication
        // authority. A withdrawn gate fences each subsequent receive while
        // outstanding work keeps its original owner.
        let unread = self.state == Serving && self.settlement.is_some();
        let class = match self.state {
            RetiringReply | Closing => WorkClass::CleanupItems,
            _ => WorkClass::BindingPolls,
        };
        if budget.remaining(class) == 0 || (self.state == Serving && !self.ingress_gate.is_open()) {
            return if unread || (self.state == Dispatching && self.issuer.strong_count() != 0) {
                Poll::Ready(self.state)
            } else {
                Poll::Pending
            };
        }
        if matches!(self.state, Starting | Serving | Dispatching | RetiringReply)
            && self.issuer.strong_count() == 0
        {
            self.fail(CoreError::StaleHandle(context()));
            return Poll::Ready(self.state);
        }
        if self.state != Closing && !self.pair_is_current() {
            self.fail(CoreError::Validation(context()));
            return Poll::Ready(self.state);
        }
        if matches!(
            self.state,
            Serving | Dispatching | Delivering | RetiringReply
        ) && self.ingress_gate.is_open()
            && let Err(error) = self.binding.check_ingress()
        {
            if self.state != Delivering {
                self.fail(error);
                return Poll::Ready(self.state);
            }
            // An independently failed listener can still own a usable reply
            // socket. Fence fresh ingress while the original delivery runs.
            self.ingress_gate.withdraw();
            self.first_cause.get_or_insert(error);
        }
        if unread || self.state == Dispatching {
            return Poll::Ready(self.state);
        }
        // Reserve terminal handoff work before the native poll. Charge only
        // on actual FIFO insertion. Queue pressure cannot hide ingress loss.
        let queue_work = if self.capacity.is_some()
            && matches!(self.state, Delivering | RetiringReply | Closing)
            && (self.ingress.is_some() || self.reply.is_some())
        {
            self.completed.insertion_work()
        } else {
            0
        };
        if budget.remaining(WorkClass::QueueOperations) < queue_work {
            return Poll::Pending;
        }
        match self.state {
            Starting => {
                budget.consume(class, 1).expect("prechecked native poll");
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
                if let Some(capacity) = &self.capacity {
                    self.work = capacity.acquire();
                    if self.work.is_none() {
                        // Bounded work/result pressure does not unpublish or
                        // close a persistent native participant.
                        return Poll::Pending;
                    }
                }
                // Capacity refusal performs no Binding poll and must leave a
                // shared turn's native credit available to the releasing owner.
                budget.consume(class, 1).expect("prechecked native poll");
                let received = self.binding.poll_receive(cx, now);
                self.io_deadline = self.binding.request_deadline();
                match received {
                    Poll::Pending => {
                        // Incomplete parsing remains Binding-owned and bounded
                        // by its native policy, independently of Engine work.
                        self.work = None;
                        Poll::Pending
                    }
                    Poll::Ready(Err(e)) => {
                        self.fail(e);
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Ok(ingress)) => {
                        self.ingress = Some(ingress);
                        // A linked Binding can revoke publication while its
                        // receive callback runs. Retain its original ingress
                        // for native retirement, never offer it for dispatch.
                        if self.ingress_gate.is_open() {
                            self.state = Dispatching;
                        } else {
                            self.fail(CoreError::Cancelled(ErrorContext::new(
                                ErrorPhase::Cleanup,
                                RetryClass::Never,
                            )));
                        }
                        Poll::Ready(self.state)
                    }
                }
            }
            Delivering => {
                // Security-bearing exchanges recheck identity and application
                // permission under a fresh complete allowance before each send.
                if self.security_work != 0 && self.reply.as_ref().is_some_and(|r| r.result.is_ok())
                {
                    if !self.ingress_gate.is_open() {
                        self.fail(CoreError::StaleHandle(context()));
                        return Poll::Ready(self.state);
                    }
                    let Some(work) = self
                        .security_work
                        .checked_mul(2)
                        .and_then(|work| work.checked_add(1))
                    else {
                        self.fail(CoreError::Backpressure(context()));
                        return Poll::Ready(self.state);
                    };
                    if budget.consume(WorkClass::HandlerSteps, work).is_err() {
                        return Poll::Pending;
                    }
                    let reply = self.reply.as_ref().expect("reply retained");
                    let checked = (|| {
                        let security_revision = self.binding.security_revision();
                        let accepted = self
                            .accepted_instance
                            .as_ref()
                            .ok_or_else(|| CoreError::StaleHandle(context()))?;
                        let handler = &accepted.instance.behaviors[accepted.behavior].handler;
                        if self.binding.verify_ingress(&reply.ingress)?
                            != reply.ingress.input.principal
                        {
                            return Err(CoreError::Security {
                                reason: clinkz_wot_core::SecurityFailureReason::InvalidCredentials,
                                context: context(),
                            });
                        }
                        if !accepted.permission.is_current() {
                            return Err(CoreError::StaleHandle(context()));
                        }
                        let policy_revision = handler.authorization_revision();
                        handler.authorize(&reply.ingress.input)?;
                        if !self.pair_is_current() {
                            return Err(CoreError::Validation(context()));
                        }
                        // Application policy can change an external verifier.
                        // It cannot grant delivery using a now-revoked identity.
                        if self.binding.verify_ingress(&reply.ingress)?
                            != reply.ingress.input.principal
                        {
                            return Err(CoreError::Security {
                                reason: clinkz_wot_core::SecurityFailureReason::InvalidCredentials,
                                context: context(),
                            });
                        }
                        if !self.pair_is_current() {
                            return Err(CoreError::Validation(context()));
                        }
                        if self.binding.security_revision() != security_revision
                            || handler.authorization_revision() != policy_revision
                        {
                            return Err(CoreError::Security {
                                reason: clinkz_wot_core::SecurityFailureReason::AuthorizationDenied,
                                context: context(),
                            });
                        }
                        if !self.ingress_gate.is_open() || !accepted.permission.is_current() {
                            return Err(CoreError::StaleHandle(context()));
                        }
                        Ok(())
                    })();
                    if let Err(error) = checked {
                        self.fail(error);
                        return Poll::Ready(self.state);
                    }
                }
                budget.consume(class, 1).expect("prechecked native poll");
                match self
                    .binding
                    .poll_send(cx, self.reply.as_ref().expect("reply retained"))
                {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(e)) => {
                        self.fail(e);
                        Poll::Ready(self.state)
                    }
                    Poll::Ready(Ok(())) => {
                        budget
                            .consume(WorkClass::QueueOperations, queue_work)
                            .expect("prechecked terminal handoff");
                        self.settle_reply(FiniteDelivery::Delivered);
                        self.io_deadline = Deadline::NONE;
                        self.state = if self.first_cause.is_some() {
                            Closing
                        } else {
                            Serving
                        };
                        Poll::Ready(self.state)
                    }
                }
            }
            RetiringReply | Closing => {
                budget.consume(class, 1).expect("prechecked cleanup poll");
                let cause = self.exchange_cause.as_ref().or(self.first_cause.as_ref());
                let retired = if self.state == RetiringReply {
                    self.binding.poll_retire_reply(
                        cx,
                        self.reply.as_ref().expect("retiring original reply"),
                        cause.expect("exchange retirement cause"),
                    )
                } else {
                    self.binding
                        .poll_close(cx, self.ingress.as_ref(), self.reply.as_ref(), cause)
                };
                match retired {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(e)) => {
                        self.fail(e);
                        Poll::Ready(Closing)
                    }
                    Poll::Ready(Ok(delivery)) => {
                        budget
                            .consume(WorkClass::QueueOperations, queue_work)
                            .expect("prechecked terminal handoff");
                        self.finish_retirement(delivery);
                        Poll::Ready(self.state)
                    }
                }
            }
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
