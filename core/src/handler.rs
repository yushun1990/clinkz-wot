use crate::Operation;

use crate::{
    AffordanceTarget, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext,
    ErrorPhase, HandlerSlotId, InteractionInput, InteractionOutput, PlanId, RetryClass, ThingId,
    ThingSlotId,
};

/// A copyable snapshot of whether cancellation has been requested.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CancellationView {
    /// Cancellation has not been requested.
    #[default]
    Active,
    /// Cancellation has been requested.
    Requested,
}

impl CancellationView {
    /// Returns whether this snapshot represents a cancellation request.
    pub const fn is_requested(self) -> bool {
        matches!(self, Self::Requested)
    }
}

/// Application-owned worst-case storage declared for one handler.
///
/// ```compile_fail
/// use clinkz_wot_core::HandlerFootprint;
/// let _ = HandlerFootprint {
///     retained_bytes: 1,
///     pending_call_bytes: 2,
///     subscription_bytes: 3,
/// };
/// ```
///
/// ```compile_fail
/// use clinkz_wot_core::HandlerFootprint;
/// fn require_default<T: Default>() {}
/// require_default::<HandlerFootprint>();
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HandlerFootprint {
    retained_bytes: u64,
    pending_call_bytes: u64,
    subscription_bytes: u64,
}

impl HandlerFootprint {
    /// Creates a footprint from its three independent byte maxima.
    pub const fn new(
        retained_bytes: u64,
        pending_call_bytes: u64,
        subscription_bytes: u64,
    ) -> Self {
        Self {
            retained_bytes,
            pending_call_bytes,
            subscription_bytes,
        }
    }

    /// Returns the bytes retained for the published handler generation.
    pub const fn retained_bytes(self) -> u64 {
        self.retained_bytes
    }

    /// Returns the additional bytes retained by one pending call.
    pub const fn pending_call_bytes(self) -> u64 {
        self.pending_call_bytes
    }

    /// Returns the bytes retained by one accepted subscription.
    pub const fn subscription_bytes(self) -> u64 {
        self.subscription_bytes
    }
}

/// One bounded step of a portable handler.
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use clinkz_wot_core::HandlerStep;
/// HandlerStep::<()>::Pending;
/// ```
///
/// ```compile_fail
/// use clinkz_wot_core::HandlerStep;
/// fn require_clone<T: Clone>() {}
/// require_clone::<HandlerStep<()>>();
/// ```
#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum HandlerStep<R> {
    /// More budgeted work is required.
    Pending,
    /// The handler reached its terminal result.
    Ready(CoreResult<R>),
}

/// A borrowed handler registration for a statically authored table.
///
/// ```compile_fail
/// use clinkz_wot_core::{HandlerFootprint, HandlerSlotId, StaticHandlerRegistration};
/// use clinkz_wot_foundation::{Generation, SlotIndex};
/// let handler = ();
/// let _ = StaticHandlerRegistration {
///     slot_id: HandlerSlotId::new(SlotIndex::new(1), Generation::INITIAL),
///     handler: &handler,
///     footprint: HandlerFootprint::new(2, 3, 5),
/// };
/// ```
pub struct StaticHandlerRegistration<'h, H> {
    slot_id: HandlerSlotId,
    handler: &'h H,
    footprint: HandlerFootprint,
}

impl<'h, H> StaticHandlerRegistration<'h, H> {
    /// Creates a registration for a handler slot.
    pub const fn new(slot_id: HandlerSlotId, handler: &'h H, footprint: HandlerFootprint) -> Self {
        Self {
            slot_id,
            handler,
            footprint,
        }
    }

    /// Returns the generation-bearing handler slot.
    pub const fn slot_id(&self) -> HandlerSlotId {
        self.slot_id
    }

    /// Returns the borrowed handler.
    pub const fn handler(&self) -> &'h H {
        self.handler
    }

    /// Returns the declared application footprint.
    pub const fn footprint(&self) -> HandlerFootprint {
        self.footprint
    }
}

impl<H> Copy for StaticHandlerRegistration<'_, H> {}

impl<H> Clone for StaticHandlerRegistration<'_, H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H> core::fmt::Debug for StaticHandlerRegistration<'_, H> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("StaticHandlerRegistration")
            .field("slot_id", &self.slot_id)
            .field("footprint", &self.footprint)
            .finish_non_exhaustive()
    }
}

/// Call-lifetime dispatch identity supplied to an operation handler.
///
/// Construction is validated and the fields cannot be forged by callers.
/// The context deliberately has neither `Default` nor `Hash` semantics.
///
/// ```compile_fail
/// use clinkz_wot_core::HandlerContext;
/// fn require_default<T: Default>() {}
/// require_default::<HandlerContext<'static>>();
/// ```
///
/// ```compile_fail
/// use core::hash::Hash;
/// use clinkz_wot_core::HandlerContext;
/// fn require_hash<T: Hash>() {}
/// require_hash::<HandlerContext<'static>>();
/// ```
///
/// ```compile_fail,E0451
/// use clinkz_wot_core::{AffordanceTarget, HandlerContext, Operation, PlanId, ThingId, ThingSlotId};
/// use clinkz_wot_foundation::{Generation, SlotIndex};
/// let thing_id = ThingId::from("urn:thing:private");
/// let target = AffordanceTarget::Thing;
/// let _ = HandlerContext {
///     thing_id: Some(&thing_id),
///     thing_slot: ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL),
///     target: &target,
///     operation: Operation::ReadAllProperties,
///     plan_id: Some(PlanId::new(SlotIndex::new(2), Generation::INITIAL)),
///     #[cfg(feature = "std")]
///     producer_instance: None,
///     binding: None,
/// };
/// ```
/// The same public values can construct a context through its checked API:
/// ```
/// use clinkz_wot_core::{AffordanceTarget, HandlerContext, Operation, PlanId, ThingId, ThingSlotId};
/// use clinkz_wot_foundation::{Generation, SlotIndex};
/// let thing_id = ThingId::from("urn:thing:private");
/// let target = AffordanceTarget::Thing;
/// let context = HandlerContext::try_new(
///     &thing_id,
///     ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL),
///     &target,
///     Operation::ReadAllProperties,
///     PlanId::new(SlotIndex::new(2), Generation::INITIAL),
///     None,
/// ).unwrap();
/// assert_eq!(context.operation(), Operation::ReadAllProperties);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandlerContext<'a> {
    thing_id: Option<&'a ThingId>,
    thing_slot: ThingSlotId,
    target: &'a AffordanceTarget,
    operation: crate::Operation,
    plan_id: Option<PlanId>,
    #[cfg(feature = "std")]
    producer_instance: Option<crate::producer::ProducerInstanceKey>,
    binding: Option<(BindingId, BindingGeneration)>,
}

impl<'a> HandlerContext<'a> {
    fn compatible(target: &AffordanceTarget, operation: Operation) -> bool {
        match target {
            AffordanceTarget::Property(_) => matches!(
                operation,
                Operation::ReadProperty
                    | Operation::WriteProperty
                    | Operation::ObserveProperty
                    | Operation::UnobserveProperty
            ),
            AffordanceTarget::Action(_) => matches!(
                operation,
                Operation::InvokeAction | Operation::QueryAction | Operation::CancelAction
            ),
            AffordanceTarget::Event(_) => matches!(
                operation,
                Operation::SubscribeEvent | Operation::UnsubscribeEvent
            ),
            AffordanceTarget::Thing => matches!(
                operation,
                Operation::ReadAllProperties
                    | Operation::WriteAllProperties
                    | Operation::ReadMultipleProperties
                    | Operation::WriteMultipleProperties
                    | Operation::ObserveAllProperties
                    | Operation::UnobserveAllProperties
                    | Operation::QueryAllActions
                    | Operation::SubscribeAllEvents
                    | Operation::UnsubscribeAllEvents
            ),
        }
    }

    /// Creates a context after validating the operation/target-kind pairing.
    pub fn try_new(
        thing_id: &'a ThingId,
        thing_slot: ThingSlotId,
        target: &'a AffordanceTarget,
        operation: crate::Operation,
        plan_id: PlanId,
        binding: Option<(BindingId, BindingGeneration)>,
    ) -> CoreResult<Self> {
        if !Self::compatible(target, operation) {
            let mut context = ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                .with_thing(thing_slot)
                .with_operation(operation)
                .with_plan(plan_id);
            if let Some((binding_id, generation)) = binding {
                context = context.with_binding(binding_id, generation);
            }
            return Err(CoreError::Validation(context));
        }

        Ok(Self {
            thing_id: Some(thing_id),
            thing_slot,
            target,
            operation,
            plan_id: Some(plan_id),
            #[cfg(feature = "std")]
            producer_instance: None,
            binding,
        })
    }

    /// Creates a source-free native Producer context for a compatible target and
    /// operation. Neither a source Thing id nor a logical plan is invented.
    #[cfg(feature = "std")]
    pub fn for_producer(
        instance: crate::producer::ProducerInstanceKey,
        target: &'a AffordanceTarget,
        operation: Operation,
        binding: (BindingId, BindingGeneration),
    ) -> CoreResult<Self> {
        if !Self::compatible(target, operation) {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                    .with_thing(instance.slot())
                    .with_operation(operation)
                    .with_binding(binding.0, binding.1),
            ));
        }
        Ok(Self {
            thing_id: None,
            thing_slot: instance.slot(),
            target,
            operation,
            plan_id: None,
            producer_instance: Some(instance),
            binding: Some(binding),
        })
    }

    /// Returns the actual native instance identity, when this is native dispatch.
    #[cfg(feature = "std")]
    pub const fn producer_instance(self) -> Option<crate::producer::ProducerInstanceKey> {
        self.producer_instance
    }

    /// Returns the borrowed source Thing identity, if retained by this execution.
    pub const fn thing_id(self) -> Option<&'a ThingId> {
        self.thing_id
    }

    /// Returns the generation-bearing Thing slot.
    pub const fn thing_slot(self) -> ThingSlotId {
        self.thing_slot
    }

    /// Returns the borrowed Thing or affordance target.
    pub const fn target(self) -> &'a AffordanceTarget {
        self.target
    }

    /// Returns the operation selected during preparation.
    pub const fn operation(self) -> crate::Operation {
        self.operation
    }

    /// Returns an actual logical plan identity, if this execution has one.
    pub const fn plan_id(self) -> Option<PlanId> {
        self.plan_id
    }

    /// Returns the optional binding identity and generation.
    pub const fn binding(self) -> Option<(BindingId, BindingGeneration)> {
        self.binding
    }
}

/// Handles one synchronous property-read interaction.
///
/// Async and portable-step handler families remain outside this narrow public
/// slice.
///
/// ```compile_fail
/// use clinkz_wot_core::AsyncReadPropertyHandler;
/// ```
///
/// ```compile_fail
/// use clinkz_wot_core::StepReadPropertyHandler;
/// ```
pub trait ReadPropertyHandler {
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
