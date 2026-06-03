use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use clinkz_wot_core::{BindingRequest, CoreError, CoreResult, InteractionOutput, ProtocolBinding};
use clinkz_wot_protocol_bindings::resolve_form_target;
use clinkz_wot_td::{data_type::Operation, form::Form};
use serde_json::Value;

use crate::{ZenohBindingError, ZenohBindingResult};

/// URI scheme used by TD forms that directly target zenoh.
pub const ZENOH_SCHEME: &str = "zenoh://";

/// Clinkz JSON-LD extension term for an explicit zenoh key expression.
pub const CZ_ZENOH_KEY_EXPR: &str = "cz-zenoh:keyExpr";

/// Resolved zenoh form target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohFormTarget {
    /// Zenoh key expression used by the concrete zenoh operation.
    pub key_expr: String,
}

/// First-pass zenoh binding implementation.
///
/// This type implements protocol selection and target extraction while keeping
/// real zenoh session execution as a later runtime integration step.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ZenohBinding {
    supported_operations: Vec<Operation>,
}

impl ZenohBinding {
    /// Creates a zenoh binding with the default WoT operation set.
    pub fn new() -> Self {
        Self {
            supported_operations: default_supported_operations(),
        }
    }

    /// Creates a zenoh binding with an explicit supported operation set.
    pub fn with_supported_operations(operations: impl IntoIterator<Item = Operation>) -> Self {
        Self {
            supported_operations: operations.into_iter().collect(),
        }
    }
}

impl ProtocolBinding for ZenohBinding {
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        self.supported_operations.contains(&operation) && is_zenoh_form(form)
    }

    fn invoke(&mut self, request: BindingRequest<'_>) -> CoreResult<InteractionOutput> {
        let target = extract_zenoh_target(request.thing, request.form)
            .map_err(|err| CoreError::Transport(err.to_string()))?;

        Err(CoreError::Transport(
            ZenohBindingError::TransportUnavailable(format!(
                "zenoh operation {:?} for '{}' is not implemented yet",
                request.operation, target.key_expr
            ))
            .to_string(),
        ))
    }
}

/// Returns true when a form carries zenoh-specific target metadata.
pub fn is_zenoh_form(form: &Form) -> bool {
    form.href.as_str().starts_with(ZENOH_SCHEME)
        || form._extra_fields.contains_key(CZ_ZENOH_KEY_EXPR)
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

fn extension_key_expr(form: &Form) -> ZenohBindingResult<Option<String>> {
    match form._extra_fields.get(CZ_ZENOH_KEY_EXPR) {
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
        Some(Value::String(_)) => Err(ZenohBindingError::Target(format!(
            "{} must not be empty",
            CZ_ZENOH_KEY_EXPR
        ))),
        Some(_) => Err(ZenohBindingError::Target(format!(
            "{} must be a string",
            CZ_ZENOH_KEY_EXPR
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
