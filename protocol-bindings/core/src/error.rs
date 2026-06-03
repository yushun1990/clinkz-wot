use alloc::string::String;
use core::fmt;

/// Result type used by protocol-neutral binding utilities.
pub type BindingCoreResult<T> = Result<T, BindingCoreError>;

/// Protocol-neutral errors surfaced by shared binding utilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingCoreError {
    /// The requested affordance does not exist on the Thing Description.
    UnknownAffordance {
        /// Affordance collection name.
        kind: &'static str,
        /// Requested affordance name.
        name: String,
    },
    /// The requested operation is not supported by any candidate form.
    UnsupportedOperation(String),
    /// Candidate forms support the operation but not the requested metadata criteria.
    MetadataMismatch(String),
    /// Candidate forms support the operation and metadata criteria, but not the caller filter.
    CallerFilterMismatch(String),
    /// The selected form does not belong to the requested affordance.
    FormNotInAffordance,
    /// The selected form target could not be resolved.
    TargetResolution(String),
}

impl fmt::Display for BindingCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAffordance { kind, name } => {
                write!(f, "Unknown {} affordance: {}", kind, name)
            }
            Self::UnsupportedOperation(message) => write!(f, "Unsupported operation: {}", message),
            Self::MetadataMismatch(message) => write!(f, "Metadata mismatch: {}", message),
            Self::CallerFilterMismatch(message) => {
                write!(f, "Caller filter mismatch: {}", message)
            }
            Self::FormNotInAffordance => {
                write!(
                    f,
                    "Selected form does not belong to the requested affordance"
                )
            }
            Self::TargetResolution(message) => write!(f, "Target resolution error: {}", message),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BindingCoreError {}
