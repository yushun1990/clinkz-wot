use alloc::{borrow::Cow, string::String};

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    data_schema::{DataSchema, DataSchemaContext},
    data_type::{AdditionalExpectedResponse, Operation},
    form::Form,
    thing::Thing,
};

const NO_OPERATIONS: &[Operation] = &[];
const PROPERTY_READ_WRITE_OPERATIONS: &[Operation] =
    &[Operation::ReadProperty, Operation::WriteProperty];
const PROPERTY_READ_OPERATIONS: &[Operation] = &[Operation::ReadProperty];
const PROPERTY_WRITE_OPERATIONS: &[Operation] = &[Operation::WriteProperty];
const ACTION_OPERATIONS: &[Operation] = &[Operation::InvokeAction];
// TD 1.1 §5.4 Default Value Definitions: a form of an Event affordance without
// an explicit `op` defaults to both `subscribeevent` and `unsubscribeevent`.
const EVENT_OPERATIONS: &[Operation] = &[Operation::SubscribeEvent, Operation::UnsubscribeEvent];

/// Context used to resolve the effective operations of a TD form.
#[derive(Debug, Clone, Copy)]
pub enum FormContext<'a> {
    /// A form declared at Thing level.
    Thing,
    /// A form declared inside a Property affordance.
    Property(&'a PropertyAffordance),
    /// A form declared inside an Action affordance.
    Action(&'a ActionAffordance),
    /// A form declared inside an Event affordance.
    Event(&'a EventAffordance),
}

/// Returns the operations that apply to a form after TD defaults are resolved.
///
/// Explicit `op` values are returned unchanged. Missing `op` values are
/// inferred from the form context according to TD 1.1 defaults. Thing-level
/// forms do not have a default operation, so an omitted `op` resolves to an
/// empty slice for that context.
pub fn effective_form_operations<'a>(
    context: FormContext<'a>,
    form: &'a Form,
) -> Cow<'a, [Operation]> {
    if let Some(operations) = &form.op {
        return Cow::Borrowed(operations.as_slice());
    }

    Cow::Borrowed(default_operations_for_context(context))
}

/// Returns the security names that apply to a form after TD inheritance.
///
/// A form-level `security` value overrides Thing-level security, including an
/// explicitly empty list. When the form omits `security`, Thing-level security
/// is inherited.
pub fn effective_form_security<'a>(thing: &'a Thing, form: &'a Form) -> &'a [String] {
    form.security
        .as_deref()
        .unwrap_or(thing.security.as_slice())
}

/// Returns the content type that applies to an additional response after TD
/// defaults are resolved.
///
/// An explicit `additionalResponses[*].contentType` value is returned
/// unchanged. When the additional response omits `contentType`, the parent
/// form's `contentType` is inherited.
pub fn effective_additional_response_content_type<'a>(
    form: &'a Form,
    response: &'a AdditionalExpectedResponse,
) -> &'a str {
    response
        .content_type
        .as_deref()
        .unwrap_or(form.content_type.as_str())
}

/// Returns the default operations for a form context.
pub fn default_operations_for_context(context: FormContext<'_>) -> &'static [Operation] {
    match context {
        FormContext::Thing => NO_OPERATIONS,
        FormContext::Property(property) => default_property_operations(property),
        FormContext::Action(_) => ACTION_OPERATIONS,
        FormContext::Event(_) => EVENT_OPERATIONS,
    }
}

fn default_property_operations(property: &PropertyAffordance) -> &'static [Operation] {
    let schema = schema_context(&property._schema);

    match (schema.read_only, schema.write_only) {
        (true, false) => PROPERTY_READ_OPERATIONS,
        (false, true) => PROPERTY_WRITE_OPERATIONS,
        // `readOnly` and `writeOnly` both `true` is invalid per TD 1.1 / JSON
        // Schema and is rejected by Basic validation. When validation is
        // skipped (Minimal level), fall back to the neutral read+write default
        // so the property stays usable instead of silently having no
        // operations.
        (true, true) | (false, false) => PROPERTY_READ_WRITE_OPERATIONS,
    }
}

fn schema_context(schema: &DataSchema) -> &DataSchemaContext {
    schema.context()
}
