use clinkz_wot_td::data_type::Operation;

use crate::{
    AffordanceTarget, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext,
    ErrorPhase, HandlerSlotId, InteractionOutput, PlanId, RetryClass, ThingId, ThingSlotId,
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

/// The response produced by a successful subscription-start handler.
#[derive(Debug, Eq, PartialEq)]
#[must_use = "a successful acceptance must be consumed by the subscription transaction"]
pub struct SubscriptionAcceptance {
    response: InteractionOutput,
}

impl SubscriptionAcceptance {
    /// Creates an acceptance carrying the handler response.
    pub const fn new(response: InteractionOutput) -> Self {
        Self { response }
    }

    /// Borrows the handler response.
    pub const fn response(&self) -> &InteractionOutput {
        &self.response
    }

    /// Consumes the acceptance and returns the handler response.
    pub fn into_response(self) -> InteractionOutput {
        self.response
    }
}

/// Application-owned worst-case storage declared for one handler.
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
#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum HandlerStep<R> {
    /// More budgeted work is required.
    Pending,
    /// The handler reached its terminal result.
    Ready(CoreResult<R>),
}

/// A borrowed handler registration for a statically authored table.
pub struct StaticHandlerRegistration<'h, H> {
    slot_id: HandlerSlotId,
    handler: &'h H,
    footprint: HandlerFootprint,
}

impl<'h, H> StaticHandlerRegistration<'h, H> {
    /// Creates a registration for a handler slot.
    pub const fn new(
        slot_id: HandlerSlotId,
        handler: &'h H,
        footprint: HandlerFootprint,
    ) -> Self {
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandlerContext<'a> {
    thing_id: &'a ThingId,
    thing_slot: ThingSlotId,
    target: &'a AffordanceTarget,
    operation: clinkz_wot_td::data_type::Operation,
    plan_id: PlanId,
    binding: Option<(BindingId, BindingGeneration)>,
}

impl<'a> HandlerContext<'a> {
    /// Creates a context after validating the operation/target-kind pairing.
    pub fn try_new(
        thing_id: &'a ThingId,
        thing_slot: ThingSlotId,
        target: &'a AffordanceTarget,
        operation: clinkz_wot_td::data_type::Operation,
        plan_id: PlanId,
        binding: Option<(BindingId, BindingGeneration)>,
    ) -> CoreResult<Self> {
        let compatible = match target {
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
        };

        if !compatible {
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
            thing_id,
            thing_slot,
            target,
            operation,
            plan_id,
            binding,
        })
    }

    /// Returns the borrowed human-readable Thing identity.
    pub const fn thing_id(self) -> &'a ThingId {
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

    /// Returns the operation selected by the compiled plan.
    pub const fn operation(self) -> clinkz_wot_td::data_type::Operation {
        self.operation
    }

    /// Returns the immutable plan identity.
    pub const fn plan_id(self) -> PlanId {
        self.plan_id
    }

    /// Returns the optional binding identity and generation.
    pub const fn binding(self) -> Option<(BindingId, BindingGeneration)> {
        self.binding
    }
}
