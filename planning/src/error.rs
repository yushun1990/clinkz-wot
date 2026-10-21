use alloc::string::String;
use core::fmt;

use clinkz_wot_core::{
    AffordanceKind, CoreError, ErrorContext, ErrorPhase, RetryClass, SelectionFailureReason,
};
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
    /// A caller-selected form does not support the requested operation.
    SelectedFormOperationMismatch(String),
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
            Self::SelectedFormOperationMismatch(message) => {
                write!(f, "Selected form operation mismatch: {}", message)
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

/// Converts a binding-selection failure into the bounded core taxonomy.
///
/// Caller-provided names, filters, targets, and resolution errors are not
/// copied into [`ErrorContext`]. The conversion retains only the structured
/// category and selection reason available at this boundary.
impl From<BindingError> for CoreError {
    fn from(err: BindingError) -> Self {
        match err {
            BindingError::UnknownAffordance { .. } => {
                selection_error(SelectionFailureReason::AffordanceMissing)
            }
            BindingError::UnsupportedOperation(_) => {
                selection_error(SelectionFailureReason::NoFormSupportsOperation)
            }
            BindingError::SelectedFormOperationMismatch(_)
            | BindingError::MetadataMismatch(_)
            | BindingError::CallerFilterMismatch(_)
            | BindingError::FormNotInAffordance => {
                selection_error(SelectionFailureReason::StrictSelectionMismatch)
            }
            BindingError::TargetResolution(_) => {
                selection_error(SelectionFailureReason::TargetResolutionFailed)
            }
        }
    }
}

fn selection_error(reason: SelectionFailureReason) -> CoreError {
    CoreError::Selection {
        reason,
        context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_preserves_bounded_selection_reasons() {
        let cases = [
            (
                BindingError::UnknownAffordance {
                    kind: AffordanceKind::Property,
                    name: String::from("private-name"),
                },
                SelectionFailureReason::AffordanceMissing,
            ),
            (
                BindingError::UnsupportedOperation(String::from("private-operation")),
                SelectionFailureReason::NoFormSupportsOperation,
            ),
            (
                BindingError::SelectedFormOperationMismatch(String::from("private-operation")),
                SelectionFailureReason::StrictSelectionMismatch,
            ),
            (
                BindingError::MetadataMismatch(String::from("private-metadata")),
                SelectionFailureReason::StrictSelectionMismatch,
            ),
            (
                BindingError::CallerFilterMismatch(String::from("private-filter")),
                SelectionFailureReason::StrictSelectionMismatch,
            ),
            (
                BindingError::TargetResolution(ResolveFormHrefError::TemplateBase(String::from(
                    "private-base",
                ))),
                SelectionFailureReason::TargetResolutionFailed,
            ),
        ];

        for (binding_error, reason) in cases {
            let core_error = CoreError::from(binding_error);
            assert_eq!(core_error.selection_reason(), Some(reason));
            assert_eq!(core_error.context().phase(), ErrorPhase::Selection);
            assert_eq!(core_error.retry_class(), RetryClass::Never);
            assert!(core_error.context().redacted_cause().is_none());
        }
    }

    #[test]
    fn foreign_form_is_a_strict_selection_mismatch() {
        let error = CoreError::from(BindingError::FormNotInAffordance);

        assert_eq!(
            error.selection_reason(),
            Some(SelectionFailureReason::StrictSelectionMismatch)
        );
        assert_eq!(error.context().phase(), ErrorPhase::Selection);
        assert_eq!(error.retry_class(), RetryClass::Never);
    }

    #[test]
    fn conversion_does_not_retain_raw_binding_text() {
        let error = CoreError::from(BindingError::CallerFilterMismatch(String::from(
            "sensitive-filter-value",
        )));
        let debug = alloc::format!("{error:?}");
        let display = alloc::format!("{error}");

        assert!(!debug.contains("sensitive-filter-value"));
        assert!(!display.contains("sensitive-filter-value"));
    }
}
