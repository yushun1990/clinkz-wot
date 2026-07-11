use alloc::{format, string::String};

use clinkz_wot_protocol_bindings::{
    AffordanceRef, FormSelectionCriteria, resolve_form_target,
    select_affordance_form_selection_with_result_filter,
};
use clinkz_wot_td::data_type::Operation;
use clinkz_wot_td::form::Form;
use fluent_uri::Uri;
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
    /// Zenoh transport protocol, derived from the URI scheme suffix.
    ///
    /// `zenoh` and `zenoh+tcp` normalize to `"tcp"`; `zenoh+udp` to `"udp"`.
    /// Combines with [`authority`](Self::authority) to form the zenoh locator
    /// `<transport>/<authority>`.
    pub transport: String,

    /// Zenoh router/peer endpoint (`host[:port]`), or `None` for the default
    /// session.
    ///
    /// Derived from the RFC 3986 authority component of the resolved href.
    /// Always non-empty for a valid form; an empty authority is rejected at
    /// extraction time as [`MissingAuthority`](ZenohBindingError::MissingAuthority).
    pub authority: String,

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
    /// Zenoh transport protocol (`"tcp"`, `"udp"`).
    pub transport: String,
    /// Zenoh router/peer endpoint.
    pub authority: String,
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
///
/// Recognizes both the bare `zenoh://` scheme and transport-suffixed variants
/// such as `zenoh+tcp://` and `zenoh+udp://` (RFC 8323 `coap+tcp` precedent).
pub fn is_zenoh_form(form: &Form) -> bool {
    has_zenoh_scheme(form.href.as_str())
}

/// Returns true when a form resolves to a zenoh target for a Thing.
pub fn is_zenoh_form_target(thing: &clinkz_wot_td::thing::Thing, form: &Form) -> bool {
    resolve_form_target(thing, form)
        .map(|target| has_zenoh_scheme(target.href.as_str()))
        .unwrap_or(false)
}

/// Resolves a form target and returns the zenoh target when the resolved href
/// uses the zenoh scheme, or `None` otherwise.
///
/// This combines the scheme check of [`is_zenoh_form_target`] with the key
/// expression extraction of [`extract_zenoh_target`] in a single target
/// resolution. Callers that need both the scheme check and the key expression
/// should prefer this over calling the two functions separately to avoid
/// resolving the form target twice.
pub fn try_extract_zenoh_target(
    thing: &clinkz_wot_td::thing::Thing,
    form: &Form,
) -> ZenohBindingResult<Option<ZenohFormTarget>> {
    let target = resolve_form_target(thing, form).map_err(ZenohBindingError::from)?;
    if has_zenoh_scheme(target.href.as_str()) {
        Ok(Some(extract_zenoh_target_from_resolved_href(
            target.href.as_str(),
        )?))
    } else {
        Ok(None)
    }
}

/// Extracts a zenoh key expression from a TD form.
///
/// The resolved `href` remains authoritative.
pub fn extract_zenoh_target(
    thing: &clinkz_wot_td::thing::Thing,
    form: &Form,
) -> ZenohBindingResult<ZenohFormTarget> {
    let target = resolve_form_target(thing, form).map_err(ZenohBindingError::from)?;
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
        transport: target.transport,
        authority: target.authority,
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
    // The result filter resolves each candidate's form target to check the
    // zenoh scheme. Because the filter API only yields a boolean, we cache the
    // resolved href of the first matching candidate so the key expression can
    // be extracted without resolving the target a second time.
    let mut resolved_href: Option<String> = None;
    let selected = select_affordance_form_selection_with_result_filter(
        thing,
        affordance,
        criteria,
        |form| -> ZenohBindingResult<bool> {
            let target = resolve_form_target(thing, form).map_err(ZenohBindingError::from)?;
            if has_zenoh_scheme(target.href.as_str()) {
                resolved_href = Some(target.href.as_str().into());
                Ok(true)
            } else {
                Ok(false)
            }
        },
    )?;
    // Invariant: the closure above sets `resolved_href` immediately before
    // returning `Ok(true)`, and the selection loop stops at the first
    // `Ok(true)`. Reaching this point means a form was selected, so the
    // cached href must be present.
    let href = resolved_href.ok_or_else(|| {
        ZenohBindingError::Selection("form selected without caching a resolved zenoh href".into())
    })?;
    let target = extract_zenoh_target_from_resolved_href(&href)?;
    let plan = ZenohOperationPlan {
        transport: target.transport,
        authority: target.authority,
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

/// Returns true when a resolved href uses a zenoh URI scheme.
///
/// Matches the bare `zenoh://` scheme and transport-suffixed variants such as
/// `zenoh+tcp://`.
fn has_zenoh_scheme(href: &str) -> bool {
    href.starts_with(ZENOH_SCHEME) || href.starts_with("zenoh+")
}

/// Splits a `zenoh[+transport]` scheme into a normalized transport string.
///
/// `zenoh` and `zenoh+tcp` normalize to `"tcp"`; `zenoh+udp` to `"udp"`;
/// unknown suffixes return [`UnsupportedTransport`](ZenohBindingError::UnsupportedTransport).
fn parse_zenoh_transport(scheme: &str) -> ZenohBindingResult<String> {
    match scheme {
        "zenoh" | "zenoh+tcp" => Ok("tcp".into()),
        "zenoh+udp" => Ok("udp".into()),
        other if other.starts_with("zenoh+") => {
            Err(ZenohBindingError::UnsupportedTransport(other.into()))
        }
        other => Err(ZenohBindingError::UnsupportedForm(format!(
            "href scheme '{}' is not zenoh",
            other
        ))),
    }
}

fn extract_zenoh_target_from_resolved_href(href: &str) -> ZenohBindingResult<ZenohFormTarget> {
    let uri = Uri::parse(href).map_err(|e| {
        ZenohBindingError::UnsupportedForm(format!("href '{}' is not a valid URI: {}", href, e))
    })?;
    let transport = parse_zenoh_transport(uri.scheme().as_str())?;
    let authority = uri
        .authority()
        .map(|a| String::from(a.as_str()))
        .filter(|a| !a.is_empty())
        .ok_or_else(|| {
            ZenohBindingError::MissingAuthority(format!(
                "zenoh form href '{}' has no authority; the TD base/href must name a router (e.g. zenoh://router:7447/...)",
                href
            ))
        })?;
    let path = uri.path().as_str();
    let key_expr = path.strip_prefix('/').ok_or_else(|| {
        ZenohBindingError::UnsupportedForm(format!("href '{}' has no path component", href))
    })?;
    if key_expr.is_empty() {
        return Err(ZenohBindingError::UnsupportedForm(format!(
            "href '{}' has an empty key expression",
            href
        )));
    }
    Ok(ZenohFormTarget {
        transport,
        authority,
        key_expr: key_expr.into(),
    })
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
