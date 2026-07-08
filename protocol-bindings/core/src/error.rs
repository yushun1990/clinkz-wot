use alloc::string::String;
use core::fmt;

use clinkz_wot_core::{AffordanceKind, CoreError};
use clinkz_wot_td::data_type::ResolveFormHrefError;

/// Result type used by protocol-neutral binding utilities.
pub type BindingResult<T> = Result<T, BindingError>;

/// Protocol-neutral errors surfaced by shared binding utilities.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindingError {
    /// The requested affordance does not exist on the Thing Description.
    UnknownAffordance {
        /// Affordance collection kind.
        kind: AffordanceKind,
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
    TargetResolution(ResolveFormHrefError),
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAffordance { kind, name } => {
                write!(f, "Unknown {} affordance: {}", kind, name)
            }
            Self::UnsupportedOperation(message) => {
                write!(f, "Unsupported operation: {}", message)
            }
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
impl std::error::Error for BindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // The wrapped `ResolveFormHrefError` carries structured detail about
            // why form href + base resolution failed; expose it as the cause so
            // `anyhow` / `eyre` chains and top-level `Error::source()` walks
            // surface it instead of stringifying it inside Display.
            Self::TargetResolution(err) => Some(err),
            _ => None,
        }
    }
}

/// Structured conversion from the protocol-neutral `BindingError` taxonomy
/// into the engine-internal `CoreError`.
///
/// Replaces the lossy private bridges that lived in
/// `protocol-bindings-protocols-zenoh/src/zenoh.rs` before this impl landed.
/// Two variants map structurally (preserving their typed payload) because
/// `CoreError` has matching variants:
///
/// | `BindingError` variant | `CoreError` variant |
/// |---|---|
/// | `UnknownAffordance { kind, name }` | `UnknownAffordance { kind, name }` |
/// | `UnsupportedOperation(message)` | `UnsupportedOperation(message)` |
///
/// The remaining four variants have no direct `CoreError` counterpart and are
/// funnelled through `CoreError::InvalidInteraction` with their `Display`
/// text — preserving the human-readable detail while losing the typed
/// payload. Callers that need the structured form should match on
/// `ServientError::Binding(BindingError)` (which preserves the original)
/// rather than letting the conversion run.
impl From<BindingError> for CoreError {
    fn from(err: BindingError) -> Self {
        match err {
            BindingError::UnknownAffordance { kind, name } => {
                CoreError::UnknownAffordance { kind, name }
            }
            BindingError::UnsupportedOperation(message) => CoreError::UnsupportedOperation(message),
            // No structural counterpart in CoreError; surface as
            // InvalidInteraction with the BindingError's Display text so
            // the diagnostic detail is preserved verbatim.
            other => CoreError::InvalidInteraction(alloc::format!("{other}")),
        }
    }
}
