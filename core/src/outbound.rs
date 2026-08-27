//! Outbound protocol binding contract and consumed-side request model
//! (baseline v4.1 §4.5 / §5.1).
//!
//! This module holds the consumer-side (outbound) binding contract; the
//! producer-side (inbound) contract — [`ServerBinding`], [`BindingContext`],
//! [`InboundDispatcher`] — lives in [`crate::inbound`]. A single concrete
//! protocol binding may implement both directions and share one protocol
//! session across them.
//!
//! [`ClientBinding::invoke`] and [`ClientBinding::subscribe`] are `async_fn`
//! (resolved A1): the outbound path is network-bound, so one `async_trait`
//! `Box` per call is accepted as network-amortized. The trait is therefore
//! gated behind the `async` feature.
//!
//! v4.1 (AD57): `ClientBinding` is stored as a shared `Arc<dyn ClientBinding>`
//! — one instance per protocol serves all consumed Things. The
//! `ClientBindingFactory` trait is removed.
//!
//! [`ServerBinding`]: crate::ServerBinding
//! [`BindingContext`]: crate::BindingContext
//! [`InboundDispatcher`]: crate::InboundDispatcher

use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc};

use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

use crate::interaction::InteractionInput;
use crate::{
    AffordanceTarget, BindingArtifactRef, BindingArtifactRole, BindingGeneration, BindingId,
    CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, PlanId, PlanSetGeneration,
    RetryClass, ThingId,
};
#[cfg(feature = "async")]
use crate::{Subscription, interaction::InteractionOutput};

/// Owned execution envelope for one already-selected Consumer Property Read.
///
/// Static target and protocol facts remain in the immutable binding artifact.
/// This request carries only the selected identity plus call-varying URI
/// variables and deadline intent; it contains no TD, Form, options, security
/// provider, candidate list, or fallback authority.
#[derive(Debug, Eq, PartialEq)]
pub struct OutboundRequest {
    thing_id: ThingId,
    target: AffordanceTarget,
    artifact: BindingArtifactRef,
    uri_variables: BTreeMap<String, String>,
    deadline: Option<Deadline>,
}

impl OutboundRequest {
    /// Constructs one structurally valid selected Property Read request.
    pub fn property_read(
        thing_id: ThingId,
        target: AffordanceTarget,
        artifact: BindingArtifactRef,
        uri_variables: BTreeMap<String, String>,
        deadline: Option<Deadline>,
    ) -> CoreResult<Self> {
        let identity = artifact.identity();
        if !matches!(&target, AffordanceTarget::Property(_))
            || identity.role() != BindingArtifactRole::ConsumerCall
        {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_operation(Operation::ReadProperty)
                    .with_plan(identity.plan_id())
                    .with_binding(identity.binding_id(), identity.binding_generation()),
            ));
        }

        Ok(Self {
            thing_id,
            target,
            artifact,
            uri_variables,
            deadline,
        })
    }

    /// Returns the selected Thing identity.
    pub const fn thing_id(&self) -> &ThingId {
        &self.thing_id
    }

    /// Returns the selected Property affordance target.
    pub const fn target(&self) -> &AffordanceTarget {
        &self.target
    }

    /// Returns the operation frozen by this request constructor.
    pub const fn operation(&self) -> Operation {
        Operation::ReadProperty
    }

    /// Returns the exact immutable artifact reference selected by Planning.
    pub const fn artifact(&self) -> BindingArtifactRef {
        self.artifact
    }

    /// Derives the binding id from the selected artifact reference.
    pub const fn binding_id(&self) -> BindingId {
        self.artifact.identity().binding_id()
    }

    /// Derives the binding generation from the selected artifact reference.
    pub const fn binding_generation(&self) -> BindingGeneration {
        self.artifact.identity().binding_generation()
    }

    /// Derives the plan-set generation from the selected artifact reference.
    pub const fn plan_set_generation(&self) -> PlanSetGeneration {
        self.artifact.identity().plan_set_generation()
    }

    /// Derives the selected logical-plan id from the artifact reference.
    pub const fn plan_id(&self) -> PlanId {
        self.artifact.identity().plan_id()
    }

    /// Returns the owned resolved URI-variable values for this call.
    pub fn uri_variables(&self) -> &BTreeMap<String, String> {
        &self.uri_variables
    }

    /// Returns the optional call deadline.
    pub const fn deadline(&self) -> Option<Deadline> {
        self.deadline
    }
}

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
    /// Request-level security metadata applied by [`SecurityProvider::apply`]
    /// (e.g. `"Authorization" → "Bearer <token>"`). Each binding maps these
    /// entries to its protocol-specific wire format. Empty when the form's
    /// effective security is `nosec` or no provider matched.
    ///
    /// [`SecurityProvider::apply`]: crate::SecurityProvider
    pub applied_security: BTreeMap<String, String>,
}

/// Outbound protocol binding contract (baseline v4.1 §4.5 / §5.1).
///
/// A concrete binding implementing this trait owns its own interior mutability
/// for I/O state, so outbound calls are issued through a shared reference. A
/// single concrete protocol binding may also implement [`crate::ServerBinding`]
/// and share one protocol session across both directions.
///
/// v4.1 (AD57): `ClientBinding` is effectively stateless — all per-Thing
/// context (TD, form, operation, input) is carried in [`BindingRequest`]. One
/// shared `Arc<dyn ClientBinding>` per protocol serves all consumed Things.
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
        request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        Err(CoreError::UnsupportedOperation(
            ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
                .with_operation(request.operation),
        ))
    }
}

/// Delegates [`ClientBinding`] through an `Arc<dyn ClientBinding>` so that
/// `ConsumedThing` can store shared binding instances (v4.1 AD57).
#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ClientBinding for Arc<dyn ClientBinding> {
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

/// Protocol-specific cleanup handle for a streaming subscription.
///
/// Returned alongside a [`Subscription`] from
/// [`ClientBinding::subscribe`]. Call [`close`](Self::close) to release wire
/// resources; dropping the guard without calling `close` also releases them.
pub trait SubscriptionGuard: Send + Sync {
    /// Releases the underlying wire subscription resources.
    fn close(self: Box<Self>);
}
