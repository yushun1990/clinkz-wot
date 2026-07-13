//! Shared mapping from [`CoreError`](clinkz_wot_core::CoreError) to HTTP-like
//! status codes (W3C WoT Profile alignment).
//!
//! Bindings use [`error_status`] to produce consistent protocol-level error
//! replies without each re-deriving the mapping.

use clinkz_wot_core::{CoreError, ErrorPhase, SecurityFailureReason, SelectionFailureReason};

/// HTTP-like status code for a [`CoreError`].
///
/// Returns 500 (internal server error) for any future variant added to
/// `CoreError` (which is `#[non_exhaustive]`) so bindings stay forward-
/// compatible without a release-coordinated update.
pub fn error_status(error: &CoreError) -> u16 {
    match error {
        CoreError::InvalidDocument(_)
        | CoreError::Validation(_)
        | CoreError::LimitExceeded { .. }
        | CoreError::Payload(_) => 400,
        CoreError::NotFound(_) | CoreError::StaleHandle(_) => 404,
        CoreError::UnsupportedOperation(context) => {
            if context.phase() == ErrorPhase::Handler {
                500
            } else {
                400
            }
        }
        CoreError::Selection { reason, .. } => selection_status(*reason),
        CoreError::Security { reason, .. } => security_status(*reason),
        CoreError::Application(_) | CoreError::Cleanup(_) | CoreError::InternalInvariant(_) => 500,
        CoreError::Binding(_)
        | CoreError::Backpressure(_)
        | CoreError::Cancelled(_)
        | CoreError::TimedOut(_)
        | CoreError::Lifecycle(_) => 503,
        _ => 500,
    }
}

const fn selection_status(reason: SelectionFailureReason) -> u16 {
    match reason {
        SelectionFailureReason::AffordanceMissing => 404,
        SelectionFailureReason::OperationUnsupported
        | SelectionFailureReason::NoFormSupportsOperation
        | SelectionFailureReason::TargetResolutionFailed
        | SelectionFailureReason::StrictSelectionMismatch => 400,
        SelectionFailureReason::NoSupportingBinding
        | SelectionFailureReason::AmbiguousBindingOwner => 500,
        SelectionFailureReason::SecurityUnavailable => 401,
        _ => 500,
    }
}

const fn security_status(reason: SecurityFailureReason) -> u16 {
    match reason {
        SecurityFailureReason::MissingCredentials
        | SecurityFailureReason::InvalidCredentials
        | SecurityFailureReason::UnsupportedScheme => 401,
        SecurityFailureReason::AuthorizationDenied => 403,
        SecurityFailureReason::ProviderFailure => 500,
        _ => 500,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_core::{ErrorContext, RetryClass};
    use clinkz_wot_foundation::ResourceKind;

    const fn context(phase: ErrorPhase) -> ErrorContext {
        ErrorContext::new(phase, RetryClass::Never)
    }

    #[test]
    fn maps_every_context_only_category() {
        let cases = [
            (CoreError::InvalidDocument(context(ErrorPhase::Parse)), 400),
            (CoreError::Validation(context(ErrorPhase::Validate)), 400),
            (
                CoreError::LimitExceeded {
                    resource: ResourceKind::PayloadBytesMax,
                    limit: 128,
                    requested: Some(129),
                    observed: None,
                    context: context(ErrorPhase::Admission),
                },
                400,
            ),
            (CoreError::NotFound(context(ErrorPhase::Selection)), 404),
            (CoreError::Application(context(ErrorPhase::Handler)), 500),
            (CoreError::Binding(context(ErrorPhase::Binding)), 503),
            (CoreError::Payload(context(ErrorPhase::Codec)), 400),
            (CoreError::Backpressure(context(ErrorPhase::Admission)), 503),
            (CoreError::Cancelled(context(ErrorPhase::Delivery)), 503),
            (CoreError::TimedOut(context(ErrorPhase::Binding)), 503),
            (CoreError::StaleHandle(context(ErrorPhase::Selection)), 404),
            (CoreError::Lifecycle(context(ErrorPhase::Commit)), 503),
            (CoreError::Cleanup(context(ErrorPhase::Cleanup)), 500),
            (
                CoreError::InternalInvariant(context(ErrorPhase::Unknown)),
                500,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error_status(&error), expected);
        }
    }

    #[test]
    fn handler_unsupported_operation_is_a_server_failure() {
        assert_eq!(
            error_status(&CoreError::UnsupportedOperation(context(
                ErrorPhase::Handler,
            ))),
            500,
        );
        assert_eq!(
            error_status(&CoreError::UnsupportedOperation(context(
                ErrorPhase::Selection,
            ))),
            400,
        );
    }

    #[test]
    fn maps_every_selection_reason() {
        let cases = [
            (SelectionFailureReason::AffordanceMissing, 404),
            (SelectionFailureReason::OperationUnsupported, 400),
            (SelectionFailureReason::NoFormSupportsOperation, 400),
            (SelectionFailureReason::TargetResolutionFailed, 400),
            (SelectionFailureReason::NoSupportingBinding, 500),
            (SelectionFailureReason::AmbiguousBindingOwner, 500),
            (SelectionFailureReason::SecurityUnavailable, 401),
            (SelectionFailureReason::StrictSelectionMismatch, 400),
        ];

        for (reason, expected) in cases {
            let error = CoreError::Selection {
                reason,
                context: context(ErrorPhase::Selection),
            };
            assert_eq!(error_status(&error), expected);
        }
    }

    #[test]
    fn maps_every_security_reason() {
        let cases = [
            (SecurityFailureReason::MissingCredentials, 401),
            (SecurityFailureReason::InvalidCredentials, 401),
            (SecurityFailureReason::AuthorizationDenied, 403),
            (SecurityFailureReason::UnsupportedScheme, 401),
            (SecurityFailureReason::ProviderFailure, 500),
        ];

        for (reason, expected) in cases {
            let error = CoreError::Security {
                reason,
                context: context(ErrorPhase::Commit),
            };
            assert_eq!(error_status(&error), expected);
        }
    }

    #[test]
    fn diagnostic_context_does_not_change_disposition() {
        let plain = CoreError::Binding(ErrorContext::new(
            ErrorPhase::Binding,
            RetryClass::CallerDecision,
        ));
        let annotated = CoreError::Binding(
            ErrorContext::new(ErrorPhase::Binding, RetryClass::Safe)
                .with_redacted_cause(17, "redacted diagnostic"),
        );

        assert_eq!(error_status(&plain), 503);
        assert_eq!(error_status(&annotated), 503);
    }
}
