//! Outbound protocol binding contract and consumed-side request model
//! (baseline v4.0 §4.5 / §5.1).
//!
//! [`ClientBinding::invoke`] and [`ClientBinding::subscribe`] are `async fn`
//! (resolved A1): the outbound path is network-bound, so one `async_trait`
//! `Box` per call is accepted as network-amortized. The trait is therefore
//! gated behind the `async` feature.

use alloc::{boxed::Box, sync::Arc};

use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

use crate::AffordanceTarget;
use crate::interaction::InteractionInput;
#[cfg(feature = "async")]
use crate::{CoreError, CoreResult, Subscription, interaction::InteractionOutput};

/// Request passed from the core runtime to a protocol binding.
///
/// This struct is owned and `'static` so it can cross a spawnable future
/// boundary (baseline addendum §2). `Thing` and `Form` are shared via [`Arc`]
/// so dispatchers that cache the canonical TD and selected form can hand out
/// cheap clones without cloning the (potentially large) TD on every call.
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

/// Outbound protocol binding contract (baseline v4.0 §4.5 / §5.1).
///
/// A concrete binding implementing this trait owns its own interior mutability
/// for I/O state, so outbound calls are issued through a shared reference. A
/// single concrete protocol binding may also implement [`crate::ServerBinding`]
/// and share one protocol session across both directions.
///
/// `invoke` and `subscribe` are `async fn` (resolved A1); the outbound path is
/// network-bound, so one `async_trait` `Box` per call is accepted as
/// network-amortized.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait ClientBinding: Send + Sync {
    /// Returns true when this binding can handle the selected form and operation.
    fn supports(&self, form: &Form, operation: Operation) -> bool;

    /// Returns true when this binding can handle the selected form, Thing, and
    /// operation. The default falls back to [`Self::supports`]; bindings that
    /// need the Thing (e.g. to resolve a relative form `href` against a
    /// Thing-level `base`) override this.
    fn supports_with_thing(&self, _: &Thing, form: &Form, operation: Operation) -> bool {
        self.supports(form, operation)
    }

    /// Performs the requested outbound interaction through the concrete protocol.
    async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput>;

    /// Opens a long-lived streaming subscription for observe or subscribe
    /// operations. Returns a consumer-side [`Subscription`] for draining pushed
    /// samples and a [`SubscriptionGuard`] that owns the underlying wire
    /// subscription.
    ///
    /// The default returns `UnsupportedOperation`, suitable for bindings that
    /// only support one-shot request/response operations.
    async fn subscribe(
        &self,
        _request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        Err(CoreError::UnsupportedOperation(
            "Binding does not support streaming subscriptions".into(),
        ))
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ClientBinding for Box<dyn ClientBinding> {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        self.as_ref().supports(form, operation)
    }

    fn supports_with_thing(&self, thing: &Thing, form: &Form, operation: Operation) -> bool {
        self.as_ref().supports_with_thing(thing, form, operation)
    }

    async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        self.as_ref().invoke(request).await
    }

    async fn subscribe(
        &self,
        request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        self.as_ref().subscribe(request).await
    }
}

/// Constructs a fresh [`ClientBinding`] for a consumed Thing.
///
/// Owned by the Servient and invoked once per consumed Thing (see
/// `Servient::consume`). Moved to `clinkz_wot_core` so that
/// [`crate::ProtocolBinding`] can reference it without pulling in the
/// Servient crate.
///
/// Concrete factories typically hold a shared, clone-able handle (e.g.
/// `Arc<MySession>`) and produce a fresh binding per `build()` call so each
/// consumed Thing owns its own plan cache / state while sharing the
/// underlying session.
#[cfg(feature = "async")]
pub trait ClientBindingFactory: Send + Sync {
    /// Produces a fresh boxed [`ClientBinding`] instance.
    fn build(&self) -> Box<dyn ClientBinding>;
}

/// Protocol-specific cleanup handle for a streaming subscription.
///
/// Returned alongside a [`Subscription`] from
/// [`ClientBinding::subscribe`]. Call [`close`](Self::close) to release wire
/// resources; dropping the guard without calling `close` also releases them.
pub trait SubscriptionGuard: Send + Sync {
    /// Releases the underlying wire subscription resources.
    fn close(self: Box<Self>);
}
