//! Runtime identity, credential material and scope checks.
use crate::{CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass, SecurityFailureReason};
use alloc::{string::String, vec::Vec};
/// Transport-level credentials extracted by a binding and consumed by
/// [`SecurityProvider`] verification (baseline addendum §1.2).
///
/// These are raw extractions; verification happens in `verify`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
/// Intended for inbound dispatchers to run after [`SecurityProvider::verify`]
/// succeeds, enforcing the matched form's required scopes (baseline v3.0 §8:
/// "authenticate plus an optional scope match").
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
