use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, CoreError, CoreResult, InteractionInput, InteractionOutput,
    Payload, ProtocolBinding,
};
use clinkz_wot_protocol_bindings::{validate_affordance_form, AffordanceRef, BindingError};
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
pub trait ZenohTransport {
    /// Executes a planned zenoh operation.
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;
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

impl<T> ProtocolBinding for ZenohBindingTransport<T>
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
