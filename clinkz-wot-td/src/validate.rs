use alloc::string::String;
use core::fmt;

/// Errors that can occur during the validation of a Thing Description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// A required field according to the W3C WoT specification is missing.
    MissingRequiredField(String),
    /// An operation type is not allowed in the current context (e.g., 'invokeaction' in a Property).
    InvalidOperation {
        context: String,
        found: String,
    },
    /// The data schema constraints are violated.
    InvalidSchema(String),
    /// The provided URI does not conform to the expected format.
    InvalidUri(String),
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidOperation { context, found } => {
                write!(f, "Invalid operation '{}' in context '{}'", found, context)
            }
            Self::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            Self::InvalidUri(uri) => write!(f, "Invalid URI: {}", uri),
        }
    }
}

/// A trait for validating components against W3C WoT Thing Description constraints.
pub trait Validate {
    /// Validates the component. Returns `Ok(())` if valid, or a `ValidateError` otherwise.
    fn validate(&self) -> Result<(), ValidateError>;
}
