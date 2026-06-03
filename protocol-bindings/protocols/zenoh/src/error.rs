use alloc::string::String;
use core::fmt;

/// Result type used by the zenoh protocol binding.
pub type ZenohBindingResult<T> = Result<T, ZenohBindingError>;

/// Errors surfaced by the zenoh protocol binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZenohBindingError {
    /// A TD form does not describe a zenoh target.
    UnsupportedForm(String),
    /// A TD form target cannot be resolved to a concrete zenoh key expression.
    Target(String),
    /// Real zenoh transport execution is not wired into this crate version yet.
    TransportUnavailable(String),
}

impl fmt::Display for ZenohBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedForm(message) => write!(f, "Unsupported zenoh form: {}", message),
            Self::Target(message) => write!(f, "Zenoh target error: {}", message),
            Self::TransportUnavailable(message) => {
                write!(f, "Zenoh transport unavailable: {}", message)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ZenohBindingError {}
