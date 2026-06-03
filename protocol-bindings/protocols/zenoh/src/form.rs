use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use clinkz_wot_core::{
    AffordanceTarget, BindingRequest, CoreError, CoreResult, InteractionInput, InteractionOutput,
    Payload, ProtocolBinding,
};
use clinkz_wot_protocol_bindings::{
    AffordanceRef, BindingCoreError, FormSelectionCriteria, resolve_form_target,
    select_affordance_form_with_filter, validate_affordance_form,
};
use clinkz_wot_td::{data_type::Operation, form::Form};
use serde_json::Value;

use crate::{ZenohBindingError, ZenohBindingResult};

/// URI scheme used by TD forms that directly target zenoh.
pub const ZENOH_SCHEME: &str = "zenoh://";

/// Clinkz JSON-LD extension term for an explicit zenoh key expression.
pub const CZ_ZENOH_KEY_EXPR: &str = "cz-zenoh:keyExpr";
/// Clinkz JSON-LD extension term for zenoh payload encoding metadata.
pub const CZ_ZENOH_ENCODING: &str = "cz-zenoh:encoding";
/// Clinkz JSON-LD extension term for zenoh QoS metadata.
pub const CZ_ZENOH_QOS: &str = "cz-zenoh:qos";
/// Clinkz JSON-LD extension term for zenoh priority metadata.
pub const CZ_ZENOH_PRIORITY: &str = "cz-zenoh:priority";
/// Clinkz JSON-LD extension term for zenoh congestion control metadata.
pub const CZ_ZENOH_CONGESTION_CONTROL: &str = "cz-zenoh:congestionControl";

/// Resolved zenoh form target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohFormTarget {
    /// Zenoh key expression used by the concrete zenoh operation.
    pub key_expr: String,
}

/// Transport-level zenoh operation shape selected for a WoT operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZenohOperationKind {
    /// Query/get-style operation that expects a reply stream or value.
    Query,
    /// Put-style operation that publishes or sets data at a key expression.
    Put,
    /// Subscription-style operation that receives updates.
    Subscribe,
    /// Subscription cancellation.
    Unsubscribe,
    /// Request/reply operation for action-like interactions.
    RequestReply,
}

/// Zenoh-specific metadata parsed from Clinkz TD extension terms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZenohFormMetadata {
    /// Optional zenoh payload encoding hint.
    pub encoding: Option<String>,
    /// Optional zenoh QoS hint.
    pub qos: Option<String>,
    /// Optional zenoh priority hint.
    pub priority: Option<String>,
    /// Optional zenoh congestion control hint.
    pub congestion_control: Option<String>,
}

/// Concrete zenoh execution plan derived from a TD form and WoT operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohOperationPlan {
    /// Zenoh key expression used by the concrete zenoh operation.
    pub key_expr: String,
    /// Transport-level operation shape.
    pub kind: ZenohOperationKind,
    /// Zenoh-specific execution metadata parsed from TD extension terms.
    pub metadata: ZenohFormMetadata,
}

/// Zenoh execution plan selected from a Thing affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohAffordanceOperationPlan<'a> {
    /// Affordance location used to select the TD form.
    pub affordance: AffordanceRef<'a>,
    /// Index of the selected form in the affordance form list.
    pub form_index: usize,
    /// Concrete zenoh execution plan.
    pub operation: ZenohOperationPlan,
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
/// host, embedded, and test runtimes can provide their own integration layer.
pub trait ZenohTransport {
    /// Executes a planned zenoh operation.
    fn execute(&mut self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;
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
            ZenohBindingError::TransportUnavailable(format!(
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
        self.supported_operations.contains(&operation) && is_zenoh_form(form)
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        validate_affordance_form(
            request.thing,
            affordance_ref_from_target(request.target),
            request.form,
            request.operation,
        )
        .map_err(core_error_from_binding_error)?;

        let plan = plan_zenoh_operation(request.thing, request.form, request.operation)
            .map_err(|err| CoreError::Transport(err.to_string()))?;
        let transport_request = build_zenoh_transport_request(plan, request.input);

        self.transport.execute(transport_request)
    }
}

/// Returns true when a form carries zenoh-specific target metadata.
pub fn is_zenoh_form(form: &Form) -> bool {
    form.href.as_str().starts_with(ZENOH_SCHEME)
        || form._extra_fields.contains_key(CZ_ZENOH_KEY_EXPR)
}

/// Returns true when a form resolves to a zenoh target for a Thing.
pub fn is_zenoh_form_target(thing: &clinkz_wot_td::thing::Thing, form: &Form) -> bool {
    form._extra_fields.contains_key(CZ_ZENOH_KEY_EXPR)
        || resolve_form_target(thing, form)
            .map(|target| target.href.as_str().starts_with(ZENOH_SCHEME))
            .unwrap_or(false)
}

/// Extracts a zenoh key expression from a TD form.
///
/// `cz-zenoh:keyExpr` takes precedence over `href` so TDs can keep a transport
/// URL in `href` while declaring the concrete key expression separately.
pub fn extract_zenoh_target(
    thing: &clinkz_wot_td::thing::Thing,
    form: &Form,
) -> ZenohBindingResult<ZenohFormTarget> {
    if let Some(key_expr) = extension_key_expr(form)? {
        return Ok(ZenohFormTarget { key_expr });
    }

    let target = resolve_form_target(thing, form)
        .map_err(|err| ZenohBindingError::Target(err.to_string()))?;
    let href = target.href.as_str();

    href.strip_prefix(ZENOH_SCHEME)
        .map(|key_expr| ZenohFormTarget {
            key_expr: key_expr.into(),
        })
        .ok_or_else(|| {
            ZenohBindingError::UnsupportedForm(format!("href '{}' is not a zenoh target", href))
        })
}

/// Builds the zenoh execution plan for a selected TD form and WoT operation.
pub fn plan_zenoh_operation(
    thing: &clinkz_wot_td::thing::Thing,
    form: &Form,
    operation: Operation,
) -> ZenohBindingResult<ZenohOperationPlan> {
    let target = extract_zenoh_target(thing, form)?;
    let kind = zenoh_operation_kind(operation);

    Ok(ZenohOperationPlan {
        key_expr: target.key_expr,
        kind,
        metadata: extract_zenoh_metadata(form)?,
    })
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

/// Selects a zenoh form from an affordance and builds its execution plan.
pub fn plan_zenoh_affordance_operation<'a>(
    thing: &'a clinkz_wot_td::thing::Thing,
    affordance: AffordanceRef<'a>,
    operation: Operation,
) -> ZenohBindingResult<ZenohAffordanceOperationPlan<'a>> {
    plan_zenoh_affordance_operation_with_criteria(
        thing,
        affordance,
        FormSelectionCriteria::operation(operation),
    )
}

/// Selects a zenoh form from an affordance using criteria and builds its execution plan.
pub fn plan_zenoh_affordance_operation_with_criteria<'a>(
    thing: &'a clinkz_wot_td::thing::Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
) -> ZenohBindingResult<ZenohAffordanceOperationPlan<'a>> {
    let selected = select_affordance_form_with_filter(thing, affordance, criteria, |form| {
        is_zenoh_form_target(thing, form)
    })
    .map_err(|err| ZenohBindingError::Selection(err.to_string()))?;
    let plan = plan_zenoh_operation(thing, selected.selection.form, criteria.operation)?;

    Ok(ZenohAffordanceOperationPlan {
        affordance: selected.affordance,
        form_index: selected.selection.index,
        operation: plan,
    })
}

/// Maps a WoT operation to the transport-level zenoh operation shape.
pub fn zenoh_operation_kind(operation: Operation) -> ZenohOperationKind {
    use Operation::*;

    match operation {
        ReadProperty
        | ReadAllProperties
        | ReadMultipleProperties
        | QueryAction
        | QueryAllActions => ZenohOperationKind::Query,
        WriteProperty | WriteAllProperties | WriteMultipleProperties => ZenohOperationKind::Put,
        ObserveProperty | ObserveAllProperties | SubscribeEvent | SubscribeAllEvents => {
            ZenohOperationKind::Subscribe
        }
        UnobserveProperty | UnobserveAllProperties | UnsubscribeEvent | UnsubscribeAllEvents => {
            ZenohOperationKind::Unsubscribe
        }
        InvokeAction | CancelAction => ZenohOperationKind::RequestReply,
    }
}

/// Extracts optional zenoh execution metadata from Clinkz extension terms.
pub fn extract_zenoh_metadata(form: &Form) -> ZenohBindingResult<ZenohFormMetadata> {
    Ok(ZenohFormMetadata {
        encoding: extension_string(form, CZ_ZENOH_ENCODING)?,
        qos: extension_string(form, CZ_ZENOH_QOS)?,
        priority: extension_string(form, CZ_ZENOH_PRIORITY)?,
        congestion_control: extension_string(form, CZ_ZENOH_CONGESTION_CONTROL)?,
    })
}

fn core_error_from_binding_error(err: BindingCoreError) -> CoreError {
    match err {
        BindingCoreError::UnknownAffordance { kind, name } => {
            CoreError::UnknownAffordance { kind, name }
        }
        BindingCoreError::UnsupportedOperation(message) => CoreError::UnsupportedOperation(message),
        BindingCoreError::MetadataMismatch(message) => CoreError::InvalidInteraction(message),
        BindingCoreError::CallerFilterMismatch(message) => CoreError::InvalidInteraction(message),
        BindingCoreError::FormNotInAffordance => CoreError::InvalidInteraction(err.to_string()),
        BindingCoreError::TargetResolution(message) => CoreError::Transport(message),
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

fn extension_key_expr(form: &Form) -> ZenohBindingResult<Option<String>> {
    extension_string(form, CZ_ZENOH_KEY_EXPR)
}

fn extension_string(form: &Form, term: &str) -> ZenohBindingResult<Option<String>> {
    match form._extra_fields.get(term) {
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
        Some(Value::String(_)) => Err(ZenohBindingError::Target(format!(
            "{} must not be empty",
            term
        ))),
        Some(_) => Err(ZenohBindingError::Target(format!(
            "{} must be a string",
            term
        ))),
        None => Ok(None),
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
