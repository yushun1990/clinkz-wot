use alloc::string::{String, ToString};
use core::fmt;

use clinkz_wot_protocol_bindings::BindingError;
use clinkz_wot_td::data_type::ResolveFormHrefError;

/// Result type used by the zenoh protocol binding.
pub type ZenohBindingResult<T> = Result<T, ZenohBindingError>;

/// Errors surfaced by the zenoh protocol binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZenohBindingError {
    /// A zenoh form could not be selected for the requested interaction.
    Selection(String),
    /// A TD form does not describe a zenoh target.
    UnsupportedForm(String),
    /// A TD form target cannot be resolved to a concrete zenoh key expression.
    Target(ResolveFormHrefError),
    /// A zenoh extension value is malformed.
    InvalidExtension {
        /// Extension term name.
        term: &'static str,
        /// Description of the malformed value.
        message: String,
    },
}

impl fmt::Display for ZenohBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Selection(message) => write!(f, "Zenoh form selection error: {}", message),
            Self::UnsupportedForm(message) => write!(f, "Unsupported zenoh form: {}", message),
            Self::Target(message) => write!(f, "Zenoh target error: {}", message),
            Self::InvalidExtension { term, message } => {
                write!(f, "Invalid zenoh extension {}: {}", term, message)
            }
        }
    }
}

#[cfg(feature = "zenoh")]
impl std::error::Error for ZenohBindingError {}

impl From<BindingError> for ZenohBindingError {
    fn from(err: BindingError) -> Self {
        match err {
            BindingError::TargetResolution(message) => Self::Target(message),
            other => Self::Selection(other.to_string()),
        }
    }
}
