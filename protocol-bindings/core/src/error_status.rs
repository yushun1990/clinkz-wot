//! Shared mapping from [`CoreError`](clinkz_wot_core::CoreError) to HTTP-like
//! status codes (W3C WoT Profile alignment).
//!
//! Bindings use [`error_status`] to produce consistent protocol-level error
//! replies without each re-deriving the mapping.

use clinkz_wot_core::{CoreError, SecurityError};

/// HTTP-like status code for a [`CoreError`].
pub fn error_status(error: &CoreError) -> u16 {
    match error {
        CoreError::UnknownAffordance { .. } => 404,
        CoreError::UnsupportedOperation(_) | CoreError::UnsupportedBinding(_) => 501,
        CoreError::Payload(_) | CoreError::InvalidInteraction(_) => 400,
        CoreError::Security(security_error) => security_status(security_error),
        CoreError::Transport(_) => 502,
        CoreError::MissingHandler { .. } => 501,
        CoreError::InboundDispatch(_) => 500,
        // Transient/overload-class failures (P0 added these structured variants):
        // a panicked handler or a timeout is a server-side transient fault.
        CoreError::HandlerPanic { .. } => 500,
        CoreError::Timeout | CoreError::TimeoutUnsupported => 504,
        // The caller pinned a form no binding can drive, or the handler emitted
        // an unacceptable content type: both are caller-side (400-class).
        CoreError::UnsupportedForm { .. } => 400,
        CoreError::ContentTypeMismatch { .. } => 406,
    }
}

fn security_status(error: &SecurityError) -> u16 {
    match error {
        SecurityError::MissingCredentials
        | SecurityError::InvalidCredentials
        | SecurityError::UnsupportedScheme => 401,
        SecurityError::ScopeDenied { .. } => 403,
        SecurityError::SchemeFailure(_) => 500,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_core::{AffordanceKind, AffordanceTarget, SecurityError};
    use clinkz_wot_td::data_type::Operation;

    #[test]
    fn maps_every_core_error_variant() {
        assert_eq!(
            error_status(&CoreError::UnknownAffordance {
                kind: AffordanceKind::Property,
                name: "x".into(),
            }),
            404
        );
        assert_eq!(
            error_status(&CoreError::UnsupportedOperation("op".into())),
            501
        );
        assert_eq!(
            error_status(&CoreError::UnsupportedBinding("b".into())),
            501
        );
        assert_eq!(error_status(&CoreError::Payload("p".into())), 400);
        assert_eq!(
            error_status(&CoreError::Security(SecurityError::MissingCredentials)),
            401
        );
        assert_eq!(
            error_status(&CoreError::Security(SecurityError::InvalidCredentials)),
            401
        );
        assert_eq!(
            error_status(&CoreError::Security(SecurityError::UnsupportedScheme)),
            401
        );
        assert_eq!(
            error_status(&CoreError::Security(SecurityError::ScopeDenied {
                required: alloc::vec![],
                present: alloc::vec![],
            })),
            403
        );
        assert_eq!(
            error_status(&CoreError::Security(SecurityError::SchemeFailure(
                "fail".into()
            ))),
            500
        );
        assert_eq!(error_status(&CoreError::Transport("t".into())), 502);
        assert_eq!(
            error_status(&CoreError::InvalidInteraction("bad".into())),
            400
        );
        assert_eq!(
            error_status(&CoreError::MissingHandler {
                target: AffordanceTarget::Property("x".into()),
                operation: Operation::ReadProperty,
            }),
            501
        );
        assert_eq!(error_status(&CoreError::InboundDispatch("d".into())), 500);
    }
}
