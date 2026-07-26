use crate::{CoreResult, HandlerSlotId, InteractionOutput};

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
