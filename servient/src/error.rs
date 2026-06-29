use alloc::string::String;
use core::fmt;

use clinkz_wot_core::{CoreError, MapLockError, SecurityError};
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
    /// A local exposed Thing is already registered with this id.
    DuplicateExposedThing(String),
    /// No local exposed Thing is registered with this id.
    ExposedThingNotFound(String),
    /// A local Thing cannot be exposed without a stable TD id.
    MissingThingId,
    /// A `ServerBinding` accept failure surfaced from the driving loop.
    Accept(String),
    /// An inbound route-registration failure during `expose` (baseline §10
    /// step 3).
    RouteRegistration(String),
    /// A shared engine lock was poisoned by a panicking thread.
    Lock(MapLockError),
}

impl fmt::Display for ServientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(err) => write!(f, "Discovery error: {}", err),
            Self::Binding(err) => write!(f, "Binding selection error: {}", err),
            Self::Serve(err) => write!(f, "Core error: {}", err),
            Self::DuplicateExposedThing(id) => {
                write!(f, "Servient already exposes Thing id '{}'", id)
            }
            Self::ExposedThingNotFound(id) => {
                write!(f, "Servient does not expose Thing id '{}'", id)
            }
            Self::MissingThingId => write!(f, "Thing Description is missing required id"),
            Self::Accept(message) => write!(f, "Server binding accept error: {}", message),
            Self::RouteRegistration(message) => {
                write!(f, "Inbound route registration error: {}", message)
            }
            Self::Lock(err) => write!(f, "{}", err),
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
            Self::Lock(err) => Some(err),
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

impl From<SecurityError> for ServientError {
    fn from(value: SecurityError) -> Self {
        Self::Serve(value.into())
    }
}

impl From<MapLockError> for ServientError {
    fn from(value: MapLockError) -> Self {
        Self::Lock(value)
    }
}
