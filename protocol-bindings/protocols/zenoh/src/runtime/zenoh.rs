use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::time::Duration;

use crate::{ZenohFormMetadata, ZenohOperationKind, ZenohOperationPlan};
use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, CoreError, CoreResult, InteractionInput, InteractionOutput,
    Payload, ProtocolBinding,
};
use clinkz_wot_protocol_bindings::{validate_affordance_form, AffordanceRef, BindingError};
use clinkz_wot_td::{data_type::Operation, form::Form};
use zenoh::{
    bytes::Encoding, handlers::FifoChannelHandler, pubsub::Subscriber, sample::Sample, Wait,
};

mod metadata;
mod sample;

use self::{
    metadata::{parse_congestion_control, parse_express_qos, parse_priority},
    sample::payload_from_sample,
};
use super::selector::selector_with_parameters;

const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

type DefaultZenohSubscriber = Subscriber<FifoChannelHandler<Sample>>;

/// Transport backed by a concrete Rust `zenoh` session.
///
/// This adapter is available only with the `runtime-zenoh` feature. It keeps
/// the default zenoh binding crate usable as `no_std + alloc` planning code
/// while giving runtimes a first concrete Rust `zenoh` execution path.
#[derive(Debug, Clone)]
pub struct ZenohSessionTransport {
    session: zenoh::Session,
    reply_timeout: Duration,
}

/// Active zenoh subscription returned by [`ZenohSessionTransport`].
///
/// The binding-level [`ZenohTransport`] trait still exposes a one-shot
/// interaction result for protocol-neutral dispatch. Runtimes that need
/// explicit event lifecycle control can use this handle directly.
#[derive(Debug)]
pub struct ZenohSubscription {
    subscriber: DefaultZenohSubscriber,
    content_type_hint: Option<String>,
    reply_timeout: Duration,
}

impl ZenohSubscription {
    /// Returns the zenoh key expression this subscription listens on.
    pub fn key_expr(&self) -> &str {
        self.subscriber.key_expr().as_str()
    }

    /// Returns the content type hint applied to subscription samples, if any.
    pub fn content_type_hint(&self) -> Option<&str> {
        self.content_type_hint.as_deref()
    }

    /// Returns the default timeout used by [`ZenohSubscription::next_sample`].
    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }

    /// Waits for the next subscription sample using the default runtime timeout.
    pub fn next_sample(&mut self) -> CoreResult<InteractionOutput> {
        self.next_timeout(self.reply_timeout)
    }

    /// Waits for the next subscription sample using an explicit timeout.
    pub fn next_timeout(&mut self, timeout: Duration) -> CoreResult<InteractionOutput> {
        let sample = self
            .subscriber
            .recv_timeout(timeout)
            .map_err(transport_error)?
            .ok_or_else(|| {
                CoreError::Transport(format!(
                    "Zenoh subscription for '{}' timed out",
                    self.key_expr()
                ))
            })?;

        Ok(InteractionOutput::with_payload(payload_from_sample(
            &sample,
            self.content_type_hint.as_deref(),
        )))
    }

    /// Explicitly undeclares the underlying zenoh subscriber.
    pub fn undeclare(self) -> CoreResult<()> {
        self.subscriber.undeclare().wait().map_err(transport_error)
    }
}

impl ZenohSessionTransport {
    /// Creates a transport from an existing zenoh session.
    pub fn new(session: zenoh::Session) -> Self {
        Self {
            session,
            reply_timeout: DEFAULT_REPLY_TIMEOUT,
        }
    }

    /// Opens a zenoh session from a zenoh configuration.
    pub fn open(config: zenoh::Config) -> CoreResult<Self> {
        let session = zenoh::open(config).wait().map_err(transport_error)?;
        Ok(Self::new(session))
    }

    /// Sets the maximum time to wait for one query or subscription reply.
    pub fn with_reply_timeout(mut self, reply_timeout: Duration) -> Self {
        self.reply_timeout = reply_timeout;
        self
    }

    /// Returns the underlying zenoh session.
    pub fn session(&self) -> &zenoh::Session {
        &self.session
    }

    /// Returns the configured query and subscription reply timeout.
    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }

    /// Declares a long-lived zenoh subscription from a planned subscribe operation.
    pub fn subscribe(&self, plan: ZenohOperationPlan) -> CoreResult<ZenohSubscription> {
        if plan.kind != ZenohOperationKind::Subscribe {
            return Err(CoreError::UnsupportedOperation(format!(
                "Zenoh {:?} operation cannot be opened as a subscription",
                plan.kind
            )));
        }

        self.declare_subscription(plan.key_expr, plan.metadata)
    }
}

impl ZenohTransport for ZenohSessionTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        match request.plan.kind {
            ZenohOperationKind::Put => self.put(request),
            ZenohOperationKind::Query | ZenohOperationKind::RequestReply => self.get(request),
            ZenohOperationKind::Subscribe => self.subscribe_once(request),
            ZenohOperationKind::Unsubscribe => Ok(InteractionOutput::empty()),
        }
    }
}

impl ZenohSessionTransport {
    fn put(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let body = request
            .payload
            .map(|payload| payload.body)
            .unwrap_or_default();
        let mut builder = self.session.put(request.plan.key_expr.as_str(), body);
        if let Some(content_type) = request.plan.metadata.content_type.as_deref() {
            builder = builder.encoding(Encoding::from(content_type));
        }
        if let Some(qos) = request.plan.metadata.qos.as_deref() {
            builder = builder.express(parse_express_qos(qos)?);
        }
        if let Some(priority) = request.plan.metadata.priority.as_deref() {
            builder = builder.priority(parse_priority(priority)?);
        }
        if let Some(congestion_control) = request.plan.metadata.congestion_control.as_deref() {
            builder = builder.congestion_control(parse_congestion_control(congestion_control)?);
        }
        builder.wait().map_err(transport_error)?;
        Ok(InteractionOutput::empty())
    }

    fn get(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let selector = selector_with_parameters(&request.plan.key_expr, &request.parameters)?;
        let mut builder = self.session.get(selector.as_str());
        if let Some(payload) = request.payload {
            builder = builder.payload(payload.body);
        }
        if let Some(content_type) = request.plan.metadata.content_type.as_deref() {
            builder = builder.encoding(Encoding::from(content_type));
        }
        if let Some(qos) = request.plan.metadata.qos.as_deref() {
            builder = builder.express(parse_express_qos(qos)?);
        }
        if let Some(priority) = request.plan.metadata.priority.as_deref() {
            builder = builder.priority(parse_priority(priority)?);
        }
        if let Some(congestion_control) = request.plan.metadata.congestion_control.as_deref() {
            builder = builder.congestion_control(parse_congestion_control(congestion_control)?);
        }

        let replies = builder.wait().map_err(transport_error)?;
        let reply = replies
            .recv_timeout(self.reply_timeout)
            .map_err(transport_error)?
            .ok_or_else(|| {
                CoreError::Transport(format!(
                    "Zenoh query for '{}' timed out",
                    request.plan.key_expr
                ))
            })?;
        let sample = reply.into_result().map_err(transport_error)?;

        Ok(InteractionOutput::with_payload(payload_from_sample(
            &sample, None,
        )))
    }

    fn subscribe_once(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        let mut subscription =
            self.declare_subscription(request.plan.key_expr, request.plan.metadata)?;
        let output = subscription.next_timeout(self.reply_timeout)?;
        subscription.undeclare()?;

        Ok(output)
    }

    fn declare_subscription(
        &self,
        key_expr: String,
        metadata: ZenohFormMetadata,
    ) -> CoreResult<ZenohSubscription> {
        let subscriber = self
            .session
            .declare_subscriber(key_expr.as_str())
            .wait()
            .map_err(transport_error)?;

        Ok(ZenohSubscription {
            subscriber,
            content_type_hint: metadata.content_type,
            reply_timeout: self.reply_timeout,
        })
    }
}

fn transport_error(error: impl core::fmt::Display) -> CoreError {
    CoreError::Transport(error.to_string())
}

/// Request passed from the zenoh binding planner to a zenoh transport adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohTransportRequest {
    /// Concrete zenoh execution plan.
    pub plan: ZenohOperationPlan,
    /// Optional encoded payload from the WoT interaction input.
    pub payload: Option<Payload>,
    /// Runtime parameters supplied by the caller.
    pub parameters: BTreeMap<String, String>,
}

/// Transport adapter contract for concrete zenoh runtime integrations.
///
/// This trait deliberately avoids depending on a concrete zenoh session type so
/// std, constrained, and test runtimes can provide their own integration layer.
pub trait ZenohTransport {
    /// Executes a planned zenoh operation.
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;
}

/// Shareable zenoh transport handle for std runtime integrations.
///
/// This wrapper lets binding factories clone a handle to the same underlying
/// session, connection pool, or runtime adapter while each `ZenohBinding`
/// still owns its protocol binding value.
#[cfg(feature = "zenoh")]
#[derive(Debug)]
pub struct SharedZenohTransport<T> {
    inner: std::sync::Arc<std::sync::Mutex<T>>,
}

#[cfg(feature = "zenoh")]
impl<T> SharedZenohTransport<T> {
    /// Creates a shared transport handle from a concrete transport adapter.
    pub fn new(transport: T) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(transport)),
        }
    }

    /// Creates a shared transport handle from an existing `Arc<Mutex<T>>`.
    pub fn from_arc(inner: std::sync::Arc<std::sync::Mutex<T>>) -> Self {
        Self { inner }
    }

    /// Returns the underlying shared transport container.
    pub fn inner(&self) -> &std::sync::Arc<std::sync::Mutex<T>> {
        &self.inner
    }
}

#[cfg(feature = "zenoh")]
impl<T> Clone for SharedZenohTransport<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(feature = "zenoh")]
impl<T> ZenohTransport for SharedZenohTransport<T>
where
    T: ZenohTransport,
{
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        self.inner
            .lock()
            .map_err(|_| CoreError::Transport("Zenoh shared transport lock is poisoned".into()))?
            .execute(request)
    }
}

/// Placeholder transport used when no concrete zenoh runtime is attached.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NoZenohTransport;

/// Zenoh binding implementation.
///
/// This type implements protocol selection and target extraction while keeping
/// concrete zenoh session execution behind an injected transport adapter.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ZenohBinding<T = NoZenohTransport> {
    supported_operations: Vec<Operation>,
    transport: T,
}

impl ZenohBinding<NoZenohTransport> {
    /// Creates a zenoh binding with the default WoT operation set and no attached transport.
    pub fn new() -> Self {
        Self {
            supported_operations: default_supported_operations(),
            transport: NoZenohTransport,
        }
    }

    /// Creates a zenoh binding with an explicit supported operation set and no attached transport.
    pub fn with_supported_operations(operations: impl IntoIterator<Item = Operation>) -> Self {
        Self {
            supported_operations: operations.into_iter().collect(),
            transport: NoZenohTransport,
        }
    }
}

impl<T> ZenohBinding<T> {
    /// Creates a zenoh binding with the default WoT operation set and an attached transport.
    pub fn with_transport(transport: T) -> Self {
        Self {
            supported_operations: default_supported_operations(),
            transport,
        }
    }

    /// Creates a zenoh binding with an explicit supported operation set and an attached transport.
    pub fn with_transport_and_supported_operations(
        transport: T,
        operations: impl IntoIterator<Item = Operation>,
    ) -> Self {
        Self {
            supported_operations: operations.into_iter().collect(),
            transport,
        }
    }

    /// Returns a shared reference to the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Returns a mutable reference to the underlying transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl ZenohTransport for NoZenohTransport {
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
        Err(CoreError::Transport(
            crate::ZenohBindingError::TransportUnavailable(format!(
                "zenoh {:?} operation for '{}' is not implemented yet",
                request.plan.kind, request.plan.key_expr
            ))
            .to_string(),
        ))
    }
}

impl<T> ProtocolBinding for ZenohBinding<T>
where
    T: ZenohTransport,
{
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        self.supported_operations.contains(&operation) && crate::form::is_zenoh_form(form)
    }

    fn supports_with_thing(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Form,
        operation: Operation,
    ) -> bool {
        self.supported_operations.contains(&operation)
            && crate::form::is_zenoh_form_target(thing, form)
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        validate_affordance_form(
            request.thing,
            affordance_ref_from_target(request.target),
            request.form,
            request.operation,
        )
        .map_err(core_error_from_binding_error)?;

        let plan =
            crate::form::plan_zenoh_operation(request.thing, request.form, request.operation)
                .map_err(core_error_from_zenoh_error)?;
        let transport_request = build_zenoh_transport_request(plan, request.input);

        self.transport.execute(transport_request)
    }
}

/// Builds a transport request from a zenoh execution plan and WoT interaction input.
pub fn build_zenoh_transport_request(
    plan: ZenohOperationPlan,
    input: InteractionInput,
) -> ZenohTransportRequest {
    ZenohTransportRequest {
        plan,
        payload: input.payload,
        parameters: input.parameters,
    }
}

fn core_error_from_binding_error(err: BindingError) -> CoreError {
    match err {
        BindingError::UnknownAffordance { kind, name } => {
            CoreError::UnknownAffordance { kind, name }
        }
        BindingError::UnsupportedOperation(message) => CoreError::UnsupportedOperation(message),
        BindingError::MetadataMismatch(message) => CoreError::InvalidInteraction(message),
        BindingError::CallerFilterMismatch(message) => CoreError::InvalidInteraction(message),
        BindingError::FormNotInAffordance => CoreError::InvalidInteraction(err.to_string()),
        BindingError::TargetResolution(message) => {
            CoreError::InvalidInteraction(message.to_string())
        }
    }
}

fn core_error_from_zenoh_error(err: crate::ZenohBindingError) -> CoreError {
    match err {
        crate::ZenohBindingError::Selection(message)
        | crate::ZenohBindingError::UnsupportedForm(message)
        | crate::ZenohBindingError::InvalidExtension { message, .. } => {
            CoreError::InvalidInteraction(message)
        }
        crate::ZenohBindingError::Target(message) => {
            CoreError::InvalidInteraction(message.to_string())
        }
        crate::ZenohBindingError::TransportUnavailable(message) => CoreError::Transport(message),
    }
}

fn affordance_ref_from_target(target: AffordanceTarget<'_>) -> AffordanceRef<'_> {
    match target {
        AffordanceTarget::Thing => AffordanceRef::Thing,
        AffordanceTarget::Property(name) => AffordanceRef::Property(name),
        AffordanceTarget::Action(name) => AffordanceRef::Action(name),
        AffordanceTarget::Event(name) => AffordanceRef::Event(name),
    }
}

fn default_supported_operations() -> Vec<Operation> {
    use Operation::*;

    vec![
        ReadProperty,
        WriteProperty,
        ObserveProperty,
        UnobserveProperty,
        InvokeAction,
        QueryAction,
        CancelAction,
        SubscribeEvent,
        UnsubscribeEvent,
        ReadAllProperties,
        WriteAllProperties,
        ReadMultipleProperties,
        WriteMultipleProperties,
        ObserveAllProperties,
        UnobserveAllProperties,
        QueryAllActions,
        SubscribeAllEvents,
        UnsubscribeAllEvents,
    ]
}
