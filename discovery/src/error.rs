//! Discovery error taxonomy (baseline v4.0 §6 / phase-p1 §1.11).

use alloc::string::String;
use core::fmt;

use clinkz_wot_core::ThingId;
use clinkz_wot_td::validate::ValidateError;

/// Result type used by Discovery operations.
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Errors produced by protocol-neutral Discovery components.
#[derive(Debug, PartialEq, Eq)]
pub enum DiscoveryError {
    /// A TD cannot be registered without a stable Thing identifier.
    MissingThingId,
    /// TD validation failed before a directory write.
    InvalidThingDescription(ValidateError),
    /// No TD exists for the requested Thing identifier.
    UnknownThing(ThingId),
    /// A TD with the same Thing identifier is already registered, or a
    /// publisher update conflicted with the stored revision.
    PublisherConflict {
        /// Thing identifier in conflict.
        id: ThingId,
        /// Stored revision at the time of conflict (publisher-side optimistic
        /// concurrency; `0` when the conflict is a plain duplicate registration).
        revision: u64,
    },
    /// A lease expired or its renewal token is unrecognized.
    LeaseExpired,
    /// The requested endpoint kind cannot be served (e.g. a remote URL with no
    /// fetcher backend — v1 records this as a backend-availability gap, §1.9 E6).
    UnsupportedEndpoint,
    /// A resolver (TD fetch or ThingLink) failed to produce a result.
    ResolverFailed(String),
    /// A requested `CountMode` cannot be satisfied by the backend.
    UnsupportedCountMode,
    /// A requested `ConsistencyMode` cannot be served (v1 ships `Live` only).
    UnsupportedConsistency,
    /// A requested `ProjectionMode` cannot be served by the backend.
    UnsupportedProjection,
    /// The session was stopped or has terminated; further `next()` returns
    /// `Ok(None)` but `stop()`/out-of-band mutation may surface this.
    SessionClosed,
    /// A continuation token was malformed or no longer valid for this session.
    InvalidContinuation,
    /// The requested operation is not implemented by this backend (v1
    /// placeholder for unimplemented remote paths).
    NotImplemented,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingThingId => write!(f, "Thing Description is missing required id"),
            Self::InvalidThingDescription(err) => write!(f, "Invalid Thing Description: {}", err),
            Self::UnknownThing(id) => write!(f, "No Thing Description for id '{}'", id),
            Self::PublisherConflict { id, revision } => write!(
                f,
                "Publisher conflict for id '{}' at revision {}",
                id, revision
            ),
            Self::LeaseExpired => write!(f, "Directory lease expired or token unrecognized"),
            Self::UnsupportedEndpoint => write!(f, "Unsupported discovery endpoint"),
            Self::ResolverFailed(message) => write!(f, "Resolver failed: {}", message),
            Self::UnsupportedCountMode => write!(f, "Unsupported count mode"),
            Self::UnsupportedConsistency => write!(f, "Unsupported consistency mode"),
            Self::UnsupportedProjection => write!(f, "Unsupported projection mode"),
            Self::SessionClosed => write!(f, "Discovery session is closed"),
            Self::InvalidContinuation => write!(f, "Invalid or stale continuation token"),
            Self::NotImplemented => write!(f, "Discovery operation not implemented"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // Surface the underlying TD validation failure as the cause so
            // error chains walk through to the structured ValidateError
            // instead of stopping at the DiscoveryError wrapper.
            Self::InvalidThingDescription(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ValidateError> for DiscoveryError {
    fn from(value: ValidateError) -> Self {
        Self::InvalidThingDescription(value)
    }
}
