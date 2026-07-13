//! Servient composition errors (baseline v4.0 §7 / phase-p3).
//!
//! [`ServientError`] is the **single application-facing error type** for the
//! clinkz-wot engine. It wraps the engine-internal `CoreError`, the
//! protocol-neutral `BindingError`, and the discovery `DiscoveryError` into
//! one tree, plus Servient-lifecycle variants that have no counterpart in
//! the lower layers.
//!
//! ## Conversion tree
//!
//! ```text
//! BindingError   ────────►  ServientError
//! CoreError      ────────►  ServientError
//! DiscoveryError ────────►  ServientError
//! ```
//!
//! - Handler code returns `CoreResult<T>` (`Result<T, CoreError>`).
//! - Servient-level operations return `ServientResult<T>`.
//! - Each lower-layer error converts into `ServientError` via `From`, so `?`
//!   works seamlessly across layer boundaries.
//!
//! ## Discriminating errors
//!
//! Callers can either pattern-match on `ServientError` directly, or use the
//! typed [`as_core`](Self::as_core), [`as_binding`](Self::as_binding), and
//! [`as_discovery`](Self::as_discovery) accessors.

use core::fmt;

use clinkz_wot_core::{CoreError, ThingId};
use clinkz_wot_discovery::DiscoveryError;
use clinkz_wot_protocol_bindings::BindingError;

/// Result type used by Servient composition APIs.
pub type ServientResult<T> = Result<T, ServientError>;

/// Errors produced while composing local Things, consumed Things, bindings,
/// and discovery backends.
///
/// The single application-facing error type. Non-exhaustive so future
/// engine concerns can be added without breaking downstream `match`
/// expressions.
#[allow(
    clippy::large_enum_variant,
    reason = "boxing the frozen CoreError would allocate while converting a bounded core failure"
)]
#[derive(Debug)]
#[non_exhaustive]
pub enum ServientError {
    /// Discovery or directory storage failed (TD lookup, registration,
    /// lease expiry, session closed, ...).
    Discovery(DiscoveryError),
    /// Shared protocol binding form selection or target resolution failed.
    /// Preserves the structured `BindingError` taxonomy so callers can
    /// distinguish `UnknownAffordance` from `UnsupportedOperation`,
    /// `TargetResolution`, etc.
    Binding(BindingError),
    /// A dispatch-level failure from the core runtime (handler, payload,
    /// security, binding, lifecycle, or timeout failure). The pre-unification
    /// split between `Serve` and `RouteRegistration` has been collapsed into
    /// this single variant because both carried a `CoreError`.
    Serve(CoreError),
    /// A produced Thing is already exposed with this id (`expose()` of a
    /// duplicate — baseline §7.3 AD33).
    AlreadyExposed(ThingId),
    /// No exposed Thing is registered with this id.
    ExposedThingNotFound(ThingId),
    /// A Thing cannot be exposed/consumed without a stable TD id (E18).
    MissingThingId,
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
            // Future variants fall through here; `non_exhaustive` guarantees
            // callers already have a `_` arm so this is forward-compatible.
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
            // Lifecycle variants (AlreadyExposed / ExposedThingNotFound /
            // MissingThingId) have no inner cause.
            _ => None,
        }
    }
}

// --- conversions from lower-layer errors ------------------------------------

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

// --- predicates and accessors ----------------------------------------------

impl ServientError {
    /// Returns `true` when the error originated in the discovery layer
    /// (`ServientError::Discovery(_)`).
    pub fn is_discovery(&self) -> bool {
        matches!(self, Self::Discovery(_))
    }

    /// Returns `true` when the error originated in the protocol binding
    /// form-selection layer (`ServientError::Binding(_)`).
    pub fn is_binding(&self) -> bool {
        matches!(self, Self::Binding(_))
    }

    /// Returns the wrapped `CoreError` if this error is a `Serve` variant.
    ///
    /// Returns `None` for lifecycle variants (`AlreadyExposed`,
    /// `ExposedThingNotFound`, `MissingThingId`) and for `Discovery` /
    /// `Binding` (which carry their own typed payloads, not a `CoreError`).
    /// For those, use `as_discovery` / `as_binding`.
    pub fn as_core(&self) -> Option<&CoreError> {
        match self {
            Self::Serve(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the wrapped `DiscoveryError` if this error is a `Discovery`
    /// variant.
    pub fn as_discovery(&self) -> Option<&DiscoveryError> {
        match self {
            Self::Discovery(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the wrapped `BindingError` if this error is a `Binding`
    /// variant.
    pub fn as_binding(&self) -> Option<&BindingError> {
        match self {
            Self::Binding(err) => Some(err),
            _ => None,
        }
    }
}
