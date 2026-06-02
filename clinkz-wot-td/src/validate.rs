use alloc::string::String;
use core::fmt;

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
    /// The provided URI does not conform to the expected format.
    InvalidUri(String),
    /// A named reference points to an item that is not defined in this document.
    InvalidReference { context: String, reference: String },
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
            Self::InvalidReference { context, reference } => {
                write!(
                    f,
                    "Invalid reference '{}' in context '{}'",
                    reference, context
                )
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
