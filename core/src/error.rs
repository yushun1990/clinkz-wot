use alloc::string::String;
use core::fmt;

use clinkz_wot_td::data_type::Operation;

use crate::security::SecurityError;
use crate::thing::{AffordanceKind, AffordanceTarget};

/// Result type used by protocol-neutral core traits.
pub type CoreResult<T> = Result<T, CoreError>;

/// Protocol-neutral errors surfaced by core runtime abstractions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// The requested affordance does not exist on the Thing.
    UnknownAffordance { kind: AffordanceKind, name: String },
    /// The requested operation is not supported by the selected affordance or form.
    UnsupportedOperation(String),
    /// No binding could handle the requested form or operation.
    UnsupportedBinding(String),
    /// Payload encoding or decoding failed.
    Payload(String),
    /// Security material could not be applied or validated.
    Security(SecurityError),
    /// The transport adapter failed.
    Transport(String),
    /// The implementation returned an invalid interaction result.
    InvalidInteraction(String),
    /// An inbound interaction targeted an affordance with no attached handler
    /// (baseline addendum §4). Carries the target and operation so clients
    /// receive actionable diagnostics (e.g. HTTP 501 bodies) instead of an
    /// opaque "no handler" message.
    MissingHandler {
        target: AffordanceTarget,
        operation: Operation,
    },
    /// An inbound dispatch or routing failure with an opaque English reason.
    InboundDispatch(String),
    /// A handler panicked during dispatch (`std`-only panic→reply contract,
    /// AD30). Carries the target and operation for diagnostics.
    HandlerPanic {
        target: AffordanceTarget,
        operation: Operation,
    },
    /// An outbound call exceeded its requested `InteractionOptions.timeout`
    /// (AD39).
    Timeout,
    /// A `timeout` was requested but this build has no timer cfg (bare `no_std`,
    /// AD45). Fail-closed: never silently ignored.
    TimeoutUnsupported,
    /// A caller-pinned `form_index` points at a form no binding can drive
    /// (AD47).
    UnsupportedForm { index: usize },
    /// A byte-level handler emitted a content type the request's `Accept` hint
    /// did not permit. The engine does not transcode (AD48 / E1).
    ContentTypeMismatch { content_type: String },
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
            Self::Security(error) => write!(f, "Security error: {}", error),
            Self::Transport(message) => write!(f, "Transport error: {}", message),
            Self::InvalidInteraction(message) => write!(f, "Invalid interaction: {}", message),
            Self::MissingHandler { target, operation } => {
                write!(f, "No handler attached for {:?} on {:?}", operation, target)
            }
            Self::InboundDispatch(message) => write!(f, "Inbound dispatch error: {}", message),
            Self::HandlerPanic { target, operation } => {
                write!(
                    f,
                    "Handler panicked for {:?} on {:?}",
                    operation, target
                )
            }
            Self::Timeout => write!(f, "Outbound call timed out"),
            Self::TimeoutUnsupported => write!(
                f,
                "Outbound timeout requested but unsupported on this build"
            ),
            Self::UnsupportedForm { index } => {
                write!(f, "Caller-pinned form index {} is unsupported", index)
            }
            Self::ContentTypeMismatch { content_type } => {
                write!(f, "Content type not acceptable: {}", content_type)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Security(err) => Some(err),
            _ => None,
        }
    }
}
