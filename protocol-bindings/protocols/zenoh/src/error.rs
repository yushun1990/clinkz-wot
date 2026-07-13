use alloc::string::String;
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
    /// A zenoh form resolved with an empty authority (e.g. `zenoh:///key`).
    /// Per the binding template (§2.2), the authority is mandatory — the TD
    /// must name the router so the Consumer can locate it.
    MissingAuthority(String),
    /// The URI scheme carries a transport suffix the binding does not
    /// recognize (e.g. `zenoh+serial`).
    UnsupportedTransport(String),
    /// A TD form target cannot be resolved to a concrete zenoh key expression.
    Target(ResolveFormHrefError),
    /// A structured error from the shared protocol-binding utilities,
    /// preserved instead of being collapsed into a generic binding failure so
    /// callers can pattern-match variants like
    /// [`BindingError::UnknownAffordance`] downstream.
    Shared(BindingError),
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
            Self::MissingAuthority(message) => {
                write!(f, "Missing zenoh router authority: {}", message)
            }
            Self::UnsupportedTransport(message) => {
                write!(f, "Unsupported zenoh transport: {}", message)
            }
            Self::Target(message) => write!(f, "Zenoh target error: {}", message),
            Self::Shared(err) => write!(f, "Shared binding error: {}", err),
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
            // Preserve structured shared-binding errors so the caller can still
            // distinguish (e.g.) `UnknownAffordance` from a generic selection
            // failure after the zenoh binding maps them into `CoreError`.
            other => Self::Shared(other),
        }
    }
}
