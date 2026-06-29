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

    /// Returns a reference to the async variant of this binding, if the
    /// concrete binding implements [`AsyncClientBinding`].
    ///
    /// Bindings that support native async I/O (e.g., a zenoh session using
    /// `session.get().await`) override this to return `Some(self)`. The
    /// async consumer path (`read_property_async`, etc.) checks this and
    /// routes through `invoke_async` when available, avoiding a blocking
    /// call inside an async context.
    ///
    /// The default implementation returns `None`, so bindings that do not
    /// implement `AsyncClientBinding` fall back to the synchronous
    /// [`invoke`](Self::invoke) path.
    #[cfg(feature = "async")]
    fn as_async_binding(&self) -> Option<&dyn AsyncClientBinding> {
        None
    }
}

/// Async outbound protocol binding contract.
///
/// Bindings that perform I/O through an async runtime (e.g., zenoh's native
/// `session.get().await`) implement this trait so that the async consumer
/// path (`ConsumedThingHandle::read_property_async`, etc.) gets true
/// non-blocking I/O instead of wrapping the synchronous [`ClientBinding`]
/// path.
///
/// A concrete binding typically implements **both** [`ClientBinding`] and
/// `AsyncClientBinding`. The sync path is used by `poll_serve_sync` and
/// `serve_sync`; the async path is used by `poll_serve`, `serve`, and the
/// `*_async` consumer methods.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncClientBinding: Send + Sync {
    /// Performs the requested outbound interaction through the concrete
    /// protocol asynchronously.
    async fn invoke_async(&self, request: BindingRequest) -> CoreResult<InteractionOutput>;

    /// Opens a long-lived streaming subscription asynchronously.
    ///
    /// The default implementation returns `UnsupportedOperation`. Override
    /// when the concrete binding has a native async subscription API.
    async fn subscribe_async(
        &self,
        _request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        Err(crate::CoreError::UnsupportedOperation(
            "Binding does not support async streaming subscriptions".into(),
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

    /// Forwards to the inner binding so boxed bindings retain their async
    /// capability. Without this, `BoundConsumedThing` (which stores
    /// `Vec<Box<dyn ClientBinding>>`) would always see `None` and the async
    /// consumer path would silently degrade to blocking `invoke`.
    #[cfg(feature = "async")]
    fn as_async_binding(&self) -> Option<&dyn AsyncClientBinding> {
        (**self).as_async_binding()
    }
}
