use alloc::{boxed::Box, sync::Arc};

use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

use crate::{AffordanceTarget, CoreResult, InteractionInput, InteractionOutput, Subscription};

/// Request passed from the core runtime to a protocol binding.
///
/// This struct is owned and `'static` so it can cross a spawnable future
/// boundary (see the design baseline addendum §2). `Thing` and `Form` are
/// shared via [`Arc`] so dispatchers that cache the canonical TD and selected
/// form can hand out cheap clones without cloning the (potentially large) TD
/// on every call.
#[derive(Debug, Clone)]
pub struct BindingRequest {
    /// Thing Description that owns the selected form.
    pub thing: Arc<Thing>,
    /// Affordance location for the selected form.
    pub target: AffordanceTarget,
    /// Effective operation being performed.
    pub operation: Operation,
    /// Selected TD form.
    pub form: Arc<Form>,
    /// Caller input.
    pub input: InteractionInput,
}

/// Outbound protocol binding contract (baseline v3.0 §1, §2 / addendum §2.4).
///
/// A concrete binding implementing this trait owns its own interior mutability
/// for I/O state, so outbound calls are issued through a shared reference
/// (`invoke(&self)`). A single concrete protocol binding may also implement
/// [`crate::ServerBinding`] and share one protocol session across both
/// directions.
pub trait ClientBinding {
    /// Returns true when this binding can handle the selected form and operation.
    fn supports(&self, form: &Form, operation: Operation) -> bool;

    /// Returns true when this binding can handle the selected form, Thing, and
    /// operation.
    ///
    /// The default implementation falls back to [`ClientBinding::supports`].
    /// Concrete bindings that need the Thing (for example to resolve a relative
    /// form `href` against a Thing-level `base`) override this.
    fn supports_with_thing(&self, _: &Thing, form: &Form, operation: Operation) -> bool {
        self.supports(form, operation)
    }

    /// Performs the requested outbound interaction through the concrete protocol.
    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput>;

    /// Opens a long-lived streaming subscription for observe or subscribe
    /// operations.
    ///
    /// Returns a consumer-side [`Subscription`] for draining pushed samples and
    /// a [`SubscriptionGuard`] that owns the underlying wire subscription.
    /// When the caller is done, they should call
    /// [`SubscriptionGuard::close`](SubscriptionGuard::close) (or drop the guard)
    /// to release wire resources.
    ///
    /// The default implementation returns `UnsupportedOperation`, suitable for
    /// bindings that only support one-shot request/response operations.
    fn subscribe(
        &self,
        _request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        Err(crate::CoreError::UnsupportedOperation(
            "Binding does not support streaming subscriptions".into(),
        ))
    }
}

/// Protocol-specific cleanup handle for a streaming subscription.
///
/// Returned alongside a [`Subscription`] from
/// [`ClientBinding::subscribe`]. The guard owns the underlying wire
/// subscription (e.g. a zenoh subscriber). Call
/// [`close`](Self::close) to release wire resources; dropping the guard
/// without calling `close` also releases resources via `Drop`.
pub trait SubscriptionGuard: Send + Sync {
    /// Releases the underlying wire subscription resources.
    fn close(self: Box<Self>);
}

impl ClientBinding for Box<dyn ClientBinding> {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        self.as_ref().supports(form, operation)
    }

    fn supports_with_thing(&self, thing: &Thing, form: &Form, operation: Operation) -> bool {
        self.as_ref().supports_with_thing(thing, form, operation)
    }

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        self.as_ref().invoke(request)
    }

    fn subscribe(
        &self,
        request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        self.as_ref().subscribe(request)
    }
}
