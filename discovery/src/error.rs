use alloc::string::String;
use core::fmt;

use clinkz_wot_td::validate::ValidateError;

/// Result type used by Discovery and Thing Description Directory operations.
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Errors produced by protocol-neutral Discovery components.
#[derive(Debug, PartialEq, Eq)]
pub enum DiscoveryError {
    /// A TD cannot be registered without a stable Thing identifier.
    MissingThingId,
    /// A TD with the same Thing identifier already exists.
    DuplicateThingId(String),
    /// No TD exists for the requested Thing identifier.
    ThingNotFound(String),
    /// TD validation failed before a directory write.
    InvalidThingDescription(ValidateError),
    /// A shared directory backend lock is poisoned.
    #[cfg(feature = "std")]
    SharedDirectoryLockPoisoned,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingThingId => write!(f, "Thing Description is missing required id"),
            Self::DuplicateThingId(id) => write!(
                f,
                "Thing Description Directory already contains Thing id '{}'",
                id
            ),
            Self::ThingNotFound(id) => {
                write!(
                    f,
                    "Thing Description Directory does not contain Thing id '{}'",
                    id
                )
            }
            Self::InvalidThingDescription(err) => {
                write!(f, "Invalid Thing Description: {}", err)
            }
            #[cfg(feature = "std")]
            Self::SharedDirectoryLockPoisoned => {
                write!(f, "Shared Thing Directory lock is poisoned")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DiscoveryError {}

impl From<ValidateError> for DiscoveryError {
    fn from(value: ValidateError) -> Self {
        Self::InvalidThingDescription(value)
    }
}
