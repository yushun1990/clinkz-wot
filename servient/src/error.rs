use alloc::string::String;
use core::fmt;

use clinkz_wot_core::CoreError;
use clinkz_wot_discovery::DiscoveryError;
use clinkz_wot_protocol_bindings::BindingCoreError;

/// Result type used by Servient runtime composition APIs.
pub type ServientResult<T> = Result<T, ServientError>;

/// Errors produced while composing local Things, consumed Things, bindings,
/// and discovery backends.
#[derive(Debug)]
pub enum ServientError {
    /// Discovery or directory storage failed.
    Discovery(DiscoveryError),
    /// Shared protocol binding form selection or target resolution failed.
    Binding(BindingCoreError),
    /// Core dispatch or binding interaction failed.
    Core(CoreError),
    /// A local exposed Thing is already registered with this id.
    DuplicateExposedThing(String),
    /// No local exposed Thing is registered with this id.
    ExposedThingNotFound(String),
    /// Runtime composition cannot be mutated while the Servient is running.
    Running,
    /// A local Thing cannot be exposed without a stable TD id.
    MissingThingId,
}

impl fmt::Display for ServientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(err) => write!(f, "Discovery error: {}", err),
            Self::Binding(err) => write!(f, "Binding selection error: {}", err),
            Self::Core(err) => write!(f, "Core error: {}", err),
            Self::DuplicateExposedThing(id) => {
                write!(f, "Servient already exposes Thing id '{}'", id)
            }
            Self::ExposedThingNotFound(id) => {
                write!(f, "Servient does not expose Thing id '{}'", id)
            }
            Self::Running => write!(
                f,
                "Servient runtime composition cannot be changed while running"
            ),
            Self::MissingThingId => write!(f, "Thing Description is missing required id"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ServientError {}

impl From<DiscoveryError> for ServientError {
    fn from(value: DiscoveryError) -> Self {
        Self::Discovery(value)
    }
}

impl From<BindingCoreError> for ServientError {
    fn from(value: BindingCoreError) -> Self {
        Self::Binding(value)
    }
}

impl From<CoreError> for ServientError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}
