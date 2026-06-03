use alloc::string::String;
use core::fmt;

/// Result type used by protocol-neutral core traits.
pub type CoreResult<T> = Result<T, CoreError>;

/// Protocol-neutral errors surfaced by core runtime abstractions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// The requested affordance does not exist on the Thing.
    UnknownAffordance { kind: &'static str, name: String },
    /// The requested operation is not supported by the selected affordance or form.
    UnsupportedOperation(String),
    /// No binding could handle the requested form or operation.
    UnsupportedBinding(String),
    /// Payload encoding or decoding failed.
    Payload(String),
    /// Security material could not be applied or validated.
    Security(String),
    /// The transport adapter failed.
    Transport(String),
    /// The implementation returned an invalid interaction result.
    InvalidInteraction(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAffordance { kind, name } => {
                write!(f, "Unknown {} affordance: {}", kind, name)
            }
            Self::UnsupportedOperation(message) => write!(f, "Unsupported operation: {}", message),
            Self::UnsupportedBinding(message) => write!(f, "Unsupported binding: {}", message),
            Self::Payload(message) => write!(f, "Payload error: {}", message),
            Self::Security(message) => write!(f, "Security error: {}", message),
            Self::Transport(message) => write!(f, "Transport error: {}", message),
            Self::InvalidInteraction(message) => write!(f, "Invalid interaction: {}", message),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CoreError {}
