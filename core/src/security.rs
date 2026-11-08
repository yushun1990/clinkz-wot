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

/// Immutable, source-independent WoT security meaning. This value owns only
/// semantic operands, never credentials, policy, transport trust or authority.
/// It needs neither reference counting nor pointer atomics. A selected Consumer
/// requirement is one complete conjunction; Producer qualification visits every
/// alternative. `Unavailable` preserves a branch that this preparation adapter
/// cannot interpret; it is never equivalent to anonymous access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecurityRequirement {
    NoSec,
    Scheme {
        scheme: String,
        parameters: Vec<(String, String)>,
    },
    AllOf(Vec<Self>),
    OneOf(Vec<Self>),
    Unavailable {
        scheme: String,
    },
}
impl SecurityRequirement {
    pub const NOSEC: Self = Self::NoSec;
    pub const MAX_NODES: usize = 64;
    pub const MAX_DEPTH: usize = 8;
    pub const MAX_ALTERNATIVES: usize = 16;
    pub const MAX_COMPONENTS: usize = 4;

    pub fn scheme(scheme: &str, parameters: &[(&str, &str)]) -> Self {
        Self::Scheme {
            scheme: scheme.into(),
            parameters: parameters
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        }
    }

    /// A closed executable candidate has no remaining choice or unavailable
    /// interpretation. This checks shape and bounds, not scheme support.
    pub fn is_conjunction(&self) -> bool {
        fn visit(
            r: &SecurityRequirement,
            depth: usize,
            nodes: &mut usize,
            components: &mut usize,
        ) -> bool {
            *nodes += 1;
            if depth > SecurityRequirement::MAX_DEPTH || *nodes > SecurityRequirement::MAX_NODES {
                return false;
            }
            match r {
                SecurityRequirement::NoSec => true,
                SecurityRequirement::Scheme { .. } => {
                    *components += 1;
                    *components <= SecurityRequirement::MAX_COMPONENTS
                }
                SecurityRequirement::AllOf(v) => {
                    !v.is_empty() && v.iter().all(|r| visit(r, depth + 1, nodes, components))
                }
                _ => false,
            }
        }
        visit(self, 0, &mut 0, &mut 0)
    }

    /// Distribute conjunction over alternatives without weakening or truncating
    /// the expression. Limits refuse the whole expression, including unavailable
    /// branches. NoSec is the conjunction identity, not an empty-expression
    /// default. Declaration order supplies deterministic Consumer preference.
    pub fn alternatives(&self) -> CoreResult<Vec<Self>> {
        fn expand<'a>(
            r: &'a SecurityRequirement,
            depth: usize,
            nodes: &mut usize,
        ) -> CoreResult<Vec<Vec<&'a SecurityRequirement>>> {
            *nodes += 1;
            if depth > SecurityRequirement::MAX_DEPTH || *nodes > SecurityRequirement::MAX_NODES {
                return Err(unsupported());
            }
            match r {
                SecurityRequirement::AllOf(children) | SecurityRequirement::OneOf(children) => {
                    if children.is_empty() {
                        return Err(unsupported());
                    }
                    let conjunction = matches!(r, SecurityRequirement::AllOf(_));
                    let mut result = if conjunction {
                        alloc::vec![Vec::new()]
                    } else {
                        Vec::new()
                    };
                    for child in children {
                        let next = expand(child, depth + 1, nodes)?;
                        if conjunction {
                            if result.len().saturating_mul(next.len())
                                > SecurityRequirement::MAX_ALTERNATIVES
                            {
                                return Err(unsupported());
                            }
                            let mut product = Vec::new();
                            for left in &result {
                                for right in &next {
                                    if left.len() + right.len()
                                        > SecurityRequirement::MAX_COMPONENTS
                                    {
                                        return Err(unsupported());
                                    }
                                    let mut branch = left.clone();
                                    branch.extend(right);
                                    product.push(branch);
                                }
                            }
                            result = product;
                        } else {
                            if result.len() + next.len() > SecurityRequirement::MAX_ALTERNATIVES {
                                return Err(unsupported());
                            }
                            result.extend(next);
                        }
                    }
                    Ok(result)
                }
                SecurityRequirement::NoSec => Ok(alloc::vec![Vec::new()]),
                _ => Ok(alloc::vec![alloc::vec![r]]),
            }
        }
        expand(self, 0, &mut 0)?
            .into_iter()
            .map(|branch| {
                Ok(match branch.len() {
                    0 => Self::NoSec,
                    1 => branch[0].clone(),
                    _ => Self::AllOf(branch.into_iter().cloned().collect()),
                })
            })
            .collect()
    }

    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Scheme { scheme, parameters } => {
                scheme.capacity()
                    + parameters.capacity() * core::mem::size_of::<(String, String)>()
                    + parameters
                        .iter()
                        .map(|(k, v)| k.capacity() + v.capacity())
                        .sum::<usize>()
            }
            Self::Unavailable { scheme } => scheme.capacity(),
            Self::AllOf(v) | Self::OneOf(v) => {
                v.capacity() * core::mem::size_of::<Self>()
                    + v.iter().map(Self::retained_bytes).sum::<usize>()
            }
            Self::NoSec => 0,
        }
    }
    pub fn drop_work(&self) -> u64 {
        match self {
            Self::AllOf(v) | Self::OneOf(v) => 1 + v.iter().map(Self::drop_work).sum::<u64>(),
            Self::Scheme { parameters, .. } => 2 + parameters.len() as u64 * 2,
            _ => 1,
        }
    }
}

fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
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
