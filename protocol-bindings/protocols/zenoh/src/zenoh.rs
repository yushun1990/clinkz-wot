use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, ClientBinding, CoreError, CoreResult, InteractionInput,
    InteractionOutput, Payload, Subscription, SubscriptionGuard,
};
use clinkz_wot_protocol_bindings::{AffordanceRef, BindingError, validate_affordance_form};
use clinkz_wot_td::{data_type::Operation, form::Form};

use crate::ZenohOperationPlan;

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
/// The receiver is `&self` (baseline addendum §2.4 / §7): each concrete backend
/// owns its own interior mutability for I/O state.
pub trait ZenohTransport {
    /// Executes a planned zenoh operation.
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;

    /// Opens a long-lived zenoh subscription for streaming observe/subscribe
    /// operations.
    ///
    /// Returns a consumer-side [`Subscription`] for draining samples and a
    /// [`SubscriptionGuard`] that owns the underlying zenoh subscriber.
    /// The default implementation returns `UnsupportedOperation`.
    fn open_subscription(
        &self,
        _request: ZenohTransportRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        Err(CoreError::UnsupportedOperation(
            "Zenoh transport does not support streaming subscriptions".into(),
        ))
    }
}

/// Zenoh binding implementation.
///
/// This type implements protocol selection and target extraction while keeping
/// concrete zenoh session execution behind an injected transport adapter.
#[derive(Debug, Clone)]
pub struct ZenohBindingTransport<T> {
    supported_operations: Vec<Operation>,
    transport: T,
}

impl<T> ZenohBindingTransport<T> {
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

    /// Creates a zenoh binding with the default WoT operation set and an attached transport.
    pub fn with_transport(transport: T) -> Self {
        Self {
            supported_operations: default_supported_operations(),
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

impl<T> ClientBinding for ZenohBindingTransport<T>
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

    fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        validate_affordance_form(
            &request.thing,
            affordance_ref_from_target(&request.target),
            &request.form,
            request.operation,
        )
        .map_err(core_error_from_binding_error)?;

        let plan =
            crate::form::plan_zenoh_operation(&request.thing, &request.form, request.operation)
                .map_err(core_error_from_zenoh_error)?;
        let transport_request = build_zenoh_transport_request(plan, request.input);

        self.transport.execute(transport_request)
    }

    fn subscribe(
        &self,
        request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        validate_affordance_form(
            &request.thing,
            affordance_ref_from_target(&request.target),
            &request.form,
            request.operation,
        )
        .map_err(core_error_from_binding_error)?;

        let plan =
            crate::form::plan_zenoh_operation(&request.thing, &request.form, request.operation)
                .map_err(core_error_from_zenoh_error)?;
        let transport_request = build_zenoh_transport_request(plan, request.input);

        self.transport.open_subscription(transport_request)
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
    }
}

fn affordance_ref_from_target(target: &AffordanceTarget) -> AffordanceRef<'_> {
    match target {
        AffordanceTarget::Thing => AffordanceRef::Thing,
        AffordanceTarget::Property(name) => AffordanceRef::Property(name.as_str()),
        AffordanceTarget::Action(name) => AffordanceRef::Action(name.as_str()),
        AffordanceTarget::Event(name) => AffordanceRef::Event(name.as_str()),
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
