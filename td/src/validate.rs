use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use crate::{
    context::Context,
    data_schema::DataSchema,
    data_type::{AdditionalExpectedResponse, Operation},
    form::Form,
    security_scheme::SecurityScheme,
};

/// Validation strictness for Thing Description documents and components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Accepts any value that passed serde shape and typed field parsing.
    Minimal,
    /// Checks TD required fields, operation context, and local references.
    Basic,
    /// Checks WoT Profile compatibility rules.
    Profile,
    /// Checks all practical semantic rules.
    Full,
}

/// Errors that can occur during the validation of a Thing Description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// A required field according to the W3C WoT specification is missing.
    MissingRequiredField(String),
    /// An operation type is not allowed in the current context (e.g., 'invokeaction' in a Property).
    InvalidOperation { context: String, found: String },
    /// The data schema constraints are violated.
    InvalidSchema(String),
    /// The security scheme constraints are violated.
    InvalidSecurity(String),
    /// The provided URI does not conform to the expected format.
    InvalidUri(String),
    /// A named reference points to an item that is not defined in this document.
    InvalidReference { context: String, reference: String },
    /// A semantic or profile-level constraint is violated (e.g., missing
    /// standard `@context`, missing interaction affordances).
    InvalidContext(String),
    /// Two or more validation failures discovered in one pass.
    ///
    /// Builders accumulate every error they encounter (instead of returning
    /// only the first) so a caller can fix several issues per rebuild. The
    /// order mirrors discovery order.
    Multiple(Vec<ValidateError>),
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidOperation { context, found } => {
                write!(f, "Invalid operation '{}' in context '{}'", found, context)
            }
            Self::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            Self::InvalidSecurity(msg) => write!(f, "Invalid security scheme: {}", msg),
            Self::InvalidUri(uri) => write!(f, "Invalid URI: {}", uri),
            Self::InvalidReference { context, reference } => {
                write!(
                    f,
                    "Invalid reference '{}' in context '{}'",
                    reference, context
                )
            }
            Self::InvalidContext(msg) => write!(f, "Invalid context: {}", msg),
            Self::Multiple(errors) => {
                write!(f, "Multiple validation errors ({}):", errors.len())?;
                for err in errors {
                    write!(f, "\n  - {err}")?;
                }
                Ok(())
            }
        }
    }
}

/// A trait for validating components against W3C WoT Thing Description constraints.
pub trait Validate {
    /// Validates the component with the default `Basic` validation level.
    fn validate(&self) -> Result<(), ValidateError> {
        self.validate_with_level(ValidationLevel::Basic)
    }

    /// Validates the component at the requested strictness level.
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError>;
}

/// Parses a URI-valued builder field using `parse`, returning the parsed value
/// as `Some` on success or `None` after recording an
/// [`ValidateError::InvalidUri`] on `errors`.
///
/// Builders share the same "parse-or-record-error" pattern for URI fields.
/// Pass the concrete parser (e.g., `AbsoluteUri::parse` or `BaseUri::parse`)
/// so the helper can serve both absolute and base URI fields.
pub(crate) fn parse_uri_field<T, E>(
    label: &str,
    value: &str,
    parse: impl FnOnce(&str) -> Result<T, E>,
    errors: &mut Vec<ValidateError>,
) -> Option<T> {
    match parse(value) {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(ValidateError::InvalidUri(format!("{}: {}", label, value)));
            None
        }
    }
}

/// Flattens a [`ValidateError`] into a concise message string.
///
/// `InvalidSchema` is unwrapped to its inner message so that callers can wrap it
/// again without producing a redundant `Invalid schema:` prefix; any other
/// variant is rendered through its [`fmt::Display`] implementation.
pub(crate) fn schema_error_message(err: ValidateError) -> String {
    match err {
        ValidateError::InvalidSchema(message) => message,
        other => other.to_string(),
    }
}

/// Collapses a collected `Vec<ValidateError>` into a single `Result`.
///
/// Empty → `Ok(())`. One error → that error verbatim. Two or more → a single
/// [`ValidateError::Multiple`] aggregating them, so callers learn about every
/// problem in one pass instead of one-at-a-time across rebuilds.
pub(crate) fn collected_errors(errors: Vec<ValidateError>) -> Result<(), ValidateError> {
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.into_iter().next().expect("len == 1")),
        _ => Err(ValidateError::Multiple(errors)),
    }
}

/// Prepends an affordance/security `context` to every message-carrying variant
/// of a [`ValidateError`] **without changing its variant**, so the original
/// error taxonomy is preserved for programmatic matching.
pub(crate) fn prepend_context(context: String, err: ValidateError) -> ValidateError {
    match err {
        ValidateError::InvalidSchema(msg) => {
            ValidateError::InvalidSchema(format!("{}: {}", context, msg))
        }
        ValidateError::InvalidSecurity(msg) => {
            ValidateError::InvalidSecurity(format!("{}: {}", context, msg))
        }
        ValidateError::InvalidUri(msg) => {
            ValidateError::InvalidUri(format!("{}: {}", context, msg))
        }
        ValidateError::InvalidContext(msg) => {
            ValidateError::InvalidContext(format!("{}: {}", context, msg))
        }
        ValidateError::MissingRequiredField(field) => {
            ValidateError::MissingRequiredField(format!("{}: {}", context, field))
        }
        ValidateError::InvalidOperation {
            context: inner,
            found,
        } => ValidateError::InvalidOperation {
            context: format!("{}: {}", context, inner),
            found,
        },
        ValidateError::InvalidReference {
            context: inner,
            reference,
        } => ValidateError::InvalidReference {
            context: format!("{}: {}", context, inner),
            reference,
        },
        ValidateError::Multiple(errors) => ValidateError::Multiple(
            errors
                .into_iter()
                .map(|e| prepend_context(context.clone(), e))
                .collect(),
        ),
    }
}

/// Validates that every name in `security` is defined in `security_definitions`.
pub(crate) fn validate_security_references(
    context: &str,
    security: &[String],
    security_definitions: &BTreeMap<String, SecurityScheme>,
) -> Result<(), ValidateError> {
    for reference in security {
        if !security_definitions.contains_key(reference) {
            return Err(ValidateError::InvalidReference {
                context: context.to_string(),
                reference: reference.clone(),
            });
        }
    }

    Ok(())
}

/// Validates each [`DataSchema`] in an optional schema map, contextualizing
/// failures as `"{context}.{name}: {message}"`.
///
/// Accepts an `Option` so callers with optional maps (e.g., `schemaDefinitions`
/// and `uriVariables`) can pass them through directly.
pub(crate) fn validate_schema_map(
    context: &str,
    schemas: Option<&BTreeMap<String, DataSchema>>,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    let Some(schemas) = schemas else {
        return Ok(());
    };

    for (name, schema) in schemas {
        schema.validate_with_level(level).map_err(|err| {
            ValidateError::InvalidSchema(format!(
                "{}.{}: {}",
                context,
                name,
                schema_error_message(err)
            ))
        })?;
    }

    Ok(())
}

/// Abstraction over form-like types that expose `additionalResponses`.
///
/// Enables [`validate_form_response_references`] to serve both concrete TD forms
/// and Thing Model form templates.
pub(crate) trait HasAdditionalResponses {
    /// Returns the additional expected responses, if any.
    fn additional_responses(&self) -> Option<&[AdditionalExpectedResponse]>;
}

impl HasAdditionalResponses for Form {
    fn additional_responses(&self) -> Option<&[AdditionalExpectedResponse]> {
        self.additional_responses.as_deref()
    }
}

/// Validates that every `additionalResponses[*].schema` reference resolves to a
/// named entry in `schema_definitions`.
///
/// Only runs at [`ValidationLevel::Profile`] or stricter. At lower levels,
/// dangling references are tolerated so that Basic validation stays lenient.
pub(crate) fn validate_form_response_references<T>(
    context: &str,
    forms: &[T],
    schema_definitions: Option<&BTreeMap<String, DataSchema>>,
    level: ValidationLevel,
) -> Result<(), ValidateError>
where
    T: HasAdditionalResponses,
{
    if !matches!(level, ValidationLevel::Profile | ValidationLevel::Full) {
        return Ok(());
    }

    for (form_index, form) in forms.iter().enumerate() {
        let Some(additional_responses) = form.additional_responses() else {
            continue;
        };

        for (response_index, response) in additional_responses.iter().enumerate() {
            let Some(schema) = &response.schema else {
                continue;
            };

            // Build the reference context lazily; only needed on the error path.
            let reference_context = || {
                format!(
                    "{}[{}].additionalResponses[{}].schema",
                    context, form_index, response_index
                )
            };

            let Some(schema_definitions) = schema_definitions else {
                return Err(ValidateError::InvalidReference {
                    context: reference_context(),
                    reference: schema.clone(),
                });
            };

            if !schema_definitions.contains_key(schema) {
                return Err(ValidateError::InvalidReference {
                    context: reference_context(),
                    reference: schema.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validates that the `@context` contains at least one standard WoT context URI.
///
/// Only runs at [`ValidationLevel::Profile`] or stricter. At lower levels,
/// the context shape is accepted as-is because serde parsing already rejected
/// structurally invalid contexts (e.g., empty arrays).
pub(crate) fn validate_context_at_profile_level(
    context: &Context,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    if !matches!(level, ValidationLevel::Profile | ValidationLevel::Full) {
        return Ok(());
    }

    if !context.has_wot_context() {
        return Err(ValidateError::InvalidContext(
            "@context must contain at least one standard WoT context URI \
             (https://www.w3.org/2019/wot/td/v1 or https://www.w3.org/2022/wot/td/v1.1)"
                .to_string(),
        ));
    }

    if !context.is_wot_context_first() {
        return Err(ValidateError::InvalidContext(
            "@context must start with a standard WoT context URI \
             (https://www.w3.org/2019/wot/td/v1 or https://www.w3.org/2022/wot/td/v1.1); \
             extension namespaces must follow the standard context"
                .to_string(),
        ));
    }

    Ok(())
}

/// Validates the `op` values declared on Thing-level forms (TD 1.1 §5.3.4).
///
/// Forms declared at the Thing level may only carry meta-interaction
/// operations that target the Thing as a whole (`readallproperties`,
/// `writeallproperties`, `readmultipleproperties`, `writemultipleproperties`,
/// `observeallproperties`, `unobserveallproperties`, `queryallactions`,
/// `subscribeallevents`, `unsubscribeallevents`). Operations that belong to a
/// specific affordance (e.g. `readproperty`, `invokeaction`) are rejected.
/// Forms without an explicit `op` are accepted: Thing-level forms have no
/// default operation, so an omitted `op` simply makes the form unusable until a
/// consumer selects it by an operation it advertises elsewhere.
pub(crate) fn validate_thing_level_form_operations(forms: &[Form]) -> Result<(), ValidateError> {
    for form in forms {
        let Some(operations) = &form.op else {
            continue;
        };
        for operation in operations {
            if !is_thing_level_operation(operation) {
                return Err(ValidateError::InvalidOperation {
                    context: "Thing.forms".to_string(),
                    found: operation.as_str().to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Returns `true` when `operation` is a valid meta-operation for a Thing-level
/// form (TD 1.1 §5.3.4).
fn is_thing_level_operation(operation: &Operation) -> bool {
    if matches!(
        operation,
        Operation::ReadAllProperties
            | Operation::WriteAllProperties
            | Operation::ReadMultipleProperties
            | Operation::WriteMultipleProperties
            | Operation::ObserveAllProperties
            | Operation::UnobserveAllProperties
            | Operation::QueryAllActions
            | Operation::SubscribeAllEvents
            | Operation::UnsubscribeAllEvents
    ) {
        return true;
    }
    false
}
/// Validates that the Thing declares at least one interaction affordance or
/// top-level form at Profile/Full level.
///
/// A WoT Profile-conformant Thing MUST provide at least one interaction
/// affordance (property, action, event) or a top-level form so that consumers
/// can discover a usable operation.
pub(crate) fn validate_profile_interaction_presence(
    has_properties: bool,
    has_actions: bool,
    has_events: bool,
    has_top_level_forms: bool,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    if !matches!(level, ValidationLevel::Profile | ValidationLevel::Full) {
        return Ok(());
    }

    if !has_properties && !has_actions && !has_events && !has_top_level_forms {
        return Err(ValidateError::InvalidContext(
            "Profile-conformant Thing must declare at least one interaction \
             affordance (properties, actions, events) or a top-level form"
                .to_string(),
        ));
    }

    Ok(())
}
