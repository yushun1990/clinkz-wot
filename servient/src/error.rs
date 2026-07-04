//! Servient composition errors (baseline v4.0 §7 / phase-p3).

use core::fmt;

use clinkz_wot_core::{CoreError, ThingId};
use clinkz_wot_discovery::DiscoveryError;
use clinkz_wot_protocol_bindings::BindingError;

/// Result type used by Servient composition APIs.
pub type ServientResult<T> = Result<T, ServientError>;

/// Errors produced while composing local Things, consumed Things, bindings,
/// and discovery backends.
#[derive(Debug)]
pub enum ServientError {
    /// Discovery or directory storage failed.
    Discovery(DiscoveryError),
    /// Shared protocol binding form selection or target resolution failed.
    Binding(BindingError),
    /// A dispatch-level failure from the core runtime (baseline addendum §5.2).
    Serve(CoreError),
    /// A produced Thing is already exposed with this id (`expose()` of a
    /// duplicate — `AlreadyExposed`; baseline §7.3 AD33).
    AlreadyExposed(ThingId),
    /// No exposed Thing is registered with this id.
    ExposedThingNotFound(ThingId),
    /// A Thing cannot be exposed/consumed without a stable TD id (E18).
    MissingThingId,
    /// An inbound route-registration failure during `expose` (E12/AD27
    /// rollback surfaces the binding's `CoreError`).
    RouteRegistration(CoreError),
}

impl fmt::Display for ServientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(err) => write!(f, "Discovery error: {}", err),
            Self::Binding(err) => write!(f, "Binding selection error: {}", err),
            Self::Serve(err) => write!(f, "Core error: {}", err),
            Self::AlreadyExposed(id) => {
                write!(f, "Servient already exposes Thing id '{}'", id)
            }
            Self::ExposedThingNotFound(id) => {
                write!(f, "Servient does not expose Thing id '{}'", id)
            }
            Self::MissingThingId => write!(f, "Thing Description is missing required id"),
            Self::RouteRegistration(err) => {
                write!(f, "Inbound route registration error: {}", err)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ServientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Discovery(err) => Some(err),
            Self::Binding(err) => Some(err),
            Self::Serve(err) => Some(err),
            Self::RouteRegistration(err) => Some(err),
            _ => None,
        }
    }
}

impl From<DiscoveryError> for ServientError {
    fn from(value: DiscoveryError) -> Self {
        Self::Discovery(value)
    }
}

impl From<BindingError> for ServientError {
    fn from(value: BindingError) -> Self {
        Self::Binding(value)
    }
}

impl From<CoreError> for ServientError {
    fn from(value: CoreError) -> Self {
        Self::Serve(value)
    }
}

impl From<clinkz_wot_core::SecurityError> for ServientError {
    fn from(value: clinkz_wot_core::SecurityError) -> Self {
        Self::Serve(value.into())
    }
}
