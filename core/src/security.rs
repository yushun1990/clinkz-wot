//! Runtime identity, credential material and scope checks.
use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass, SecurityFailureReason};
use alloc::{string::String, vec::Vec};
/// Transport-level credential material extracted by a binding.
///
/// These unverified values do not authenticate a caller, establish a
/// [`Principal`], or select a security scheme.
#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AuthMaterial {
    /// Transport peer locator or identifier (for example a zenoh peer id).
    PeerId(String),
    /// Raw bearer token bytes (for example an Authorization header value).
    BearerToken(Vec<u8>),
    /// Basic auth username + password (RFC 7617).
    Basic { username: String, password: String },
    /// Raw certificate fingerprint bytes.
    CertificateFingerprint(Vec<u8>),
    /// Forward-compatible opaque carrier for schemes not yet enumerated.
    Other(Vec<u8>),
}

/// Established principal identity produced by a successful inbound verification
/// (baseline addendum §1.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PrincipalId(String);

impl PrincipalId {
    /// Creates a principal identity from an owned string.
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Returns the principal identity as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the underlying owned principal identity string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for PrincipalId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for PrincipalId {
    fn from(id: &str) -> Self {
        Self(String::from(id))
    }
}

impl core::borrow::Borrow<str> for PrincipalId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Identity established for an inbound caller after verification (baseline
/// addendum §1.2 / v3.0 §8).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Principal {
    /// Verified caller identity.
    pub id: PrincipalId,
    /// Scopes or claims carried for authorization, if any.
    pub scopes: Vec<String>,
}

impl Principal {
    /// Actual owned indirect backing, used before admitting verifier output.
    pub fn retained_bytes(&self) -> usize {
        self.id
            .0
            .capacity()
            .saturating_add(
                self.scopes
                    .capacity()
                    .saturating_mul(core::mem::size_of::<String>()),
            )
            .saturating_add(self.scopes.iter().map(String::capacity).sum::<usize>())
    }

    /// Creates an anonymous principal with no scopes, used by the `NoSec`
    /// scheme and other schemes that authenticate but do not establish a
    /// named identity.
    pub fn anonymous() -> Self {
        Self {
            id: PrincipalId::from("anonymous"),
            scopes: Vec::new(),
        }
    }
}

/// Checks whether `present` scopes satisfy every `required` scope.
///
/// Returns `Ok(())` when each required scope is among `present`. Otherwise
/// returns a bounded [`SecurityFailureReason::AuthorizationDenied`] error. Raw
/// required and present scope lists are not retained in public errors.
///
/// Use after caller authentication to compare established scopes with required
/// scopes. This helper does not verify credentials or implement a security scheme.
pub fn check_scopes(required: &[String], present: &[String]) -> CoreResult<()> {
    if required.iter().all(|scope| present.contains(scope)) {
        Ok(())
    } else {
        Err(CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never),
        })
    }
}

/// Immutable, source-independent scheme meaning. Parameters are semantic
/// operands, never credential values. Static operands keep this small supported
/// preparation adapter bounded; this is not a universal security expression IR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecurityRequirement {
    pub scheme: &'static str,
    pub parameters: &'static [(&'static str, &'static str)],
}
impl SecurityRequirement {
    pub const NOSEC: Self = Self {
        scheme: "nosec",
        parameters: &[],
    };
}

/// Per-call capability issued by an actual security service. Only its concrete
/// Binding interprets it; Core neither formats nor inspects the live material.
#[cfg(target_has_atomic = "ptr")]
#[derive(Clone)]
pub struct OutboundSecurity(alloc::sync::Arc<dyn core::any::Any + Send + Sync>);
#[cfg(target_has_atomic = "ptr")]
impl OutboundSecurity {
    pub fn new<T: core::any::Any + Send + Sync>(value: T) -> Self {
        Self(alloc::sync::Arc::new(value))
    }
    pub fn downcast_ref<T: core::any::Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}
#[cfg(target_has_atomic = "ptr")]
impl core::fmt::Debug for OutboundSecurity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("OutboundSecurity([redacted])")
    }
}
#[cfg(target_has_atomic = "ptr")]
impl PartialEq for OutboundSecurity {
    fn eq(&self, other: &Self) -> bool {
        alloc::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
#[cfg(target_has_atomic = "ptr")]
impl Eq for OutboundSecurity {}
impl core::fmt::Debug for AuthMaterial {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AuthMaterial([redacted])")
    }
}
