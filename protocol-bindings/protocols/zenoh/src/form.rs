use alloc::{
    format,
    string::{String, ToString},
};

use clinkz_wot_protocol_bindings::{
    AffordanceRef, BindingError, FormSelectionCriteria, resolve_form_target,
    select_affordance_form_selection_with_result_filter,
};
use clinkz_wot_td::data_type::Operation;
use clinkz_wot_td::form::Form;
use serde_json::Value;

use crate::{ZenohBindingError, ZenohBindingResult};

/// URI scheme used by TD forms that directly target zenoh.
pub const ZENOH_SCHEME: &str = "zenoh://";

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
    /// Content type carried by the WoT form.
    pub content_type: Option<String>,
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

/// Returns true when a form carries zenoh-specific target metadata.
pub fn is_zenoh_form(form: &Form) -> bool {
    form.href.as_str().starts_with(ZENOH_SCHEME)
}

/// Returns true when a form resolves to a zenoh target for a Thing.
pub fn is_zenoh_form_target(thing: &clinkz_wot_td::thing::Thing, form: &Form) -> bool {
    resolve_form_target(thing, form)
        .map(|target| target.href.as_str().starts_with(ZENOH_SCHEME))
        .unwrap_or(false)
}

fn zenoh_target_matches(
    thing: &clinkz_wot_td::thing::Thing,
    form: &Form,
) -> ZenohBindingResult<bool> {
    let target = resolve_form_target(thing, form).map_err(zenoh_target_error_from_binding)?;

    Ok(target.href.as_str().starts_with(ZENOH_SCHEME))
}

/// Extracts a zenoh key expression from a TD form.
///
/// The resolved `href` remains authoritative.
pub fn extract_zenoh_target(
    thing: &clinkz_wot_td::thing::Thing,
    form: &Form,
) -> ZenohBindingResult<ZenohFormTarget> {
    let target = resolve_form_target(thing, form).map_err(zenoh_target_error_from_binding)?;
    extract_zenoh_target_from_resolved_href(target.href.as_str())
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

/// Selects a zenoh form from an affordance and builds its execution plan.
pub fn plan_zenoh_affordance_operation<'a>(
    thing: &'a clinkz_wot_td::thing::Thing,
    affordance: AffordanceRef<'a>,
    operation: Operation,
) -> ZenohBindingResult<ZenohAffordanceOperationPlan<'a>> {
    plan_zenoh_affordance_operation_with_criteria(
        thing,
        affordance,
        FormSelectionCriteria::new(operation),
    )
}

/// Selects a zenoh form from an affordance using criteria and builds its execution plan.
pub fn plan_zenoh_affordance_operation_with_criteria<'a>(
    thing: &'a clinkz_wot_td::thing::Thing,
    affordance: AffordanceRef<'a>,
    criteria: FormSelectionCriteria<'_>,
) -> ZenohBindingResult<ZenohAffordanceOperationPlan<'a>> {
    let selected =
        select_affordance_form_selection_with_result_filter(thing, affordance, criteria, |form| {
            zenoh_target_matches(thing, form)
        })?;
    let target = extract_zenoh_target(thing, selected.selection.form)?;
    let plan = ZenohOperationPlan {
        key_expr: target.key_expr,
        kind: zenoh_operation_kind(criteria.operation),
        metadata: extract_zenoh_metadata(selected.selection.form)?,
    };

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
        content_type: Some(form.content_type.clone()),
        qos: extension_string(form, CZ_ZENOH_QOS)?,
        priority: extension_string(form, CZ_ZENOH_PRIORITY)?,
        congestion_control: extension_string(form, CZ_ZENOH_CONGESTION_CONTROL)?,
    })
}

fn extract_zenoh_target_from_resolved_href(href: &str) -> ZenohBindingResult<ZenohFormTarget> {
    if let Some(key_expr) = href.strip_prefix(ZENOH_SCHEME) {
        return Ok(ZenohFormTarget {
            key_expr: key_expr.into(),
        });
    }

    Err(ZenohBindingError::UnsupportedForm(format!(
        "href '{}' is not a zenoh target",
        href
    )))
}

fn extension_string(form: &Form, term: &'static str) -> ZenohBindingResult<Option<String>> {
    match form._extra_fields.get(term) {
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
        Some(Value::String(_)) => Err(ZenohBindingError::InvalidExtension {
            term,
            message: "must not be empty".into(),
        }),
        Some(_) => Err(ZenohBindingError::InvalidExtension {
            term,
            message: "must be a string".into(),
        }),
        None => Ok(None),
    }
}

fn zenoh_target_error_from_binding(err: BindingError) -> ZenohBindingError {
    match err {
        BindingError::TargetResolution(message) => ZenohBindingError::Target(message),
        other => ZenohBindingError::Selection(other.to_string()),
    }
}
