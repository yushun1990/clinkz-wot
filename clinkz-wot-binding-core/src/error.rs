use alloc::string::String;
use core::fmt;

/// Result type used by protocol-neutral binding utilities.
pub type BindingCoreResult<T> = Result<T, BindingCoreError>;

/// Protocol-neutral errors surfaced by shared binding utilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingCoreError {
    /// The requested operation is not supported by any candidate form.
    UnsupportedOperation(String),
    /// The selected form target could not be resolved.
    TargetResolution(String),
}

impl fmt::Display for BindingCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOperation(message) => write!(f, "Unsupported operation: {}", message),
            Self::TargetResolution(message) => write!(f, "Target resolution error: {}", message),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BindingCoreError {}
