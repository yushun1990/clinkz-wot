use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::fmt;

use clinkz_wot_td::{form::Form, security_scheme::SecurityScheme, thing::Thing};

use crate::inbound::InboundRequest;
use crate::{CoreError, CoreResult, MapLock, TransportRequest};

/// Security metadata available while preparing a transport request.
#[derive(Clone, Copy)]
pub struct SecurityContext<'a> {
    /// Thing Description that owns the selected form.
    pub thing: &'a Thing,
    /// Selected form.
    pub form: &'a Form,
    /// Name of the security definition being applied.
    pub scheme_name: &'a str,
    /// Security scheme definition.
    pub scheme: &'a SecurityScheme,
    /// Credential store for retrieving stored secrets, if available.
    pub credentials: Option<&'a dyn CredentialStore>,
}

impl<'a> fmt::Debug for SecurityContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecurityContext")
            .field("scheme_name", &self.scheme_name)
            .field("scheme", &self.scheme.scheme())
            .field("has_credentials", &self.credentials.is_some())
            .finish_non_exhaustive()
    }
}

/// Protocol-neutral credential material retrieved from a credential store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credentials {
    /// Raw bearer token bytes.
    BearerToken(Vec<u8>),
    /// Basic auth username + password.
    Basic { username: String, password: String },
    /// API key string.
    ApiKey(String),
    /// Pre-shared key bytes.
    Psk(Vec<u8>),
    /// Forward-compatible opaque credentials.
    Other(Vec<u8>),
}

/// Credential storage contract (baseline addendum §1.2 `cz:credentialSource`).
///
/// A [`SecurityProvider`] calls [`get`](Self::get) during
/// [`apply`](SecurityProvider::apply) to retrieve stored secrets rather than
/// capturing them in closures. Implementations are interior-mutable (`&self`
/// methods) so the store can be shared across providers and threads.
pub trait CredentialStore {
    /// Returns credentials for the given Thing and security scheme, or `None`
    /// when no credentials are stored.
    fn get(&self, thing_id: &str, scheme_name: &str) -> Option<Credentials>;
}

/// Deterministic in-memory credential store backed by a nested `BTreeMap`.
///
/// Keys are `(thing_id, scheme_name)` pairs. Suitable for `no_std + alloc`
/// and for test fixtures.
///
/// The nested `BTreeMap<String, BTreeMap<String, Credentials>>` layout lets
/// `get` lookups by `&str` keys proceed without allocating `String` keys on
/// every secured outbound request.
pub struct InMemoryCredentialStore {
    entries: MapLock<BTreeMap<String, BTreeMap<String, Credentials>>>,
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self {
            entries: MapLock::new(BTreeMap::new()),
        }
    }
}

impl InMemoryCredentialStore {
    /// Creates an empty credential store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores credentials for a Thing + scheme pair.
    ///
    /// Returns [`CoreError::Lock`] if the store's lock was poisoned by a
    /// panicking thread; in that case the write is skipped (not applied to
    /// inconsistent state), so the caller can react to a credential that was
    /// not actually stored.
    pub fn put(
        &self,
        thing_id: impl Into<String>,
        scheme_name: impl Into<String>,
        credentials: Credentials,
    ) -> CoreResult<()> {
        self.entries.with(|map| {
            map.entry(thing_id.into())
                .or_default()
                .insert(scheme_name.into(), credentials);
        })?;
        Ok(())
    }

    /// Removes credentials for a Thing + scheme pair.
    ///
    /// Returns [`CoreError::Lock`] if the store's lock was poisoned; the
    /// removal is then skipped rather than applied to inconsistent state.
    pub fn remove(&self, thing_id: &str, scheme_name: &str) -> CoreResult<()> {
        self.entries.with(|map| {
            if let Some(schemes) = map.get_mut(thing_id) {
                schemes.remove(scheme_name);
                if schemes.is_empty() {
                    map.remove(thing_id);
                }
            }
        })?;
        Ok(())
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn get(&self, thing_id: &str, scheme_name: &str) -> Option<Credentials> {
        self.entries.with_recover(|map| {
            map.get(thing_id)
                .and_then(|schemes| schemes.get(scheme_name).cloned())
        })
    }
}

impl fmt::Debug for InMemoryCredentialStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.entries.with_recover(|map| map.len());
        f.debug_struct("InMemoryCredentialStore")
            .field("entries", &count)
            .finish_non_exhaustive()
    }
}

/// Applies protocol-neutral security metadata to a transport request.
pub trait SecurityProvider {
    /// Returns the security definition name handled by this provider.
    fn scheme_name(&self) -> &str;

    /// Applies security material to a transport request.
    ///
    /// Takes `&self` (not `&mut self`) so providers can be shared across
    /// concurrent interactions via `Arc`. Implementations that need to mutate
    /// internal state (e.g. token caches) must use interior mutability.
    fn apply(&self, context: SecurityContext<'_>, request: &mut TransportRequest)
    -> CoreResult<()>;

    /// Optional hook for reporting unsupported scope names.
    fn supports_scopes(&self, _scopes: &[String]) -> bool {
        true
    }

    /// Verifies an inbound request before the dispatcher routes it to a handler
    /// (baseline v3.0 §8 / addendum §1.2).
    ///
    /// `scheme` is the security scheme resolved from the matched form. The
    /// default implementation rejects every inbound request as
    /// [`SecurityError::UnsupportedScheme`]; providers that handle inbound
    /// authentication override this and return the established [`Principal`].
    /// `verify` is synchronous, matching [`apply`](Self::apply).
    ///
    /// Per-affordance scope enforcement is performed by the dispatcher (using
    /// [`check_scopes`]) after `verify` succeeds, since the required scopes live
    /// on the matched form rather than the scheme.
    fn verify(
        &self,
        _request: &InboundRequest,
        _scheme: &SecurityScheme,
    ) -> Result<Principal, SecurityError> {
        Err(SecurityError::UnsupportedScheme)
    }
}

/// Transport-level credentials extracted by a binding and consumed by
/// [`SecurityProvider`] verification (baseline addendum §1.2).
///
/// These are raw extractions; verification happens in `verify`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AuthMaterial {
    /// Transport peer locator or identifier (for example a zenoh peer id).
    PeerId(String),
    /// Raw bearer token bytes (for example an Authorization header value).
    BearerToken(Vec<u8>),
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

/// Failure reported by inbound security verification (baseline addendum §1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// No auth material was supplied where the scheme requires it.
    MissingCredentials,
    /// Auth material was supplied but did not validate.
    InvalidCredentials,
    /// The matched security scheme is not supported by this provider.
    UnsupportedScheme,
    /// The principal lacks a required scope.
    ScopeDenied {
        /// Required scope names not satisfied by the caller.
        required: Vec<String>,
        /// Scope names established for the caller.
        present: Vec<String>,
    },
    /// Scheme- or transport-specific failure with an opaque English reason.
    SchemeFailure(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCredentials => f.write_str("missing credentials"),
            Self::InvalidCredentials => f.write_str("invalid credentials"),
            Self::UnsupportedScheme => f.write_str("unsupported security scheme"),
            Self::ScopeDenied { required, present } => {
                write!(
                    f,
                    "scope denied: required {:?}, present {:?}",
                    required, present
                )
            }
            Self::SchemeFailure(message) => {
                write!(f, "security scheme failure: {}", message)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SecurityError {}

/// Maps a [`SecurityError`] into a [`CoreError`], preserving the structured
/// taxonomy so callers can pattern-match on the specific failure.
impl From<SecurityError> for CoreError {
    fn from(error: SecurityError) -> Self {
        CoreError::Security(error)
    }
}

/// Checks whether `present` scopes satisfy every `required` scope.
///
/// Returns `Ok(())` when each required scope is among `present`. Otherwise
/// returns [`SecurityError::ScopeDenied`] reporting the unsatisfied required
/// scopes and the caller's present scopes for diagnostics.
///
/// Intended for inbound dispatchers to run after [`SecurityProvider::verify`]
/// succeeds, enforcing the matched form's required scopes (baseline v3.0 §8:
/// "authenticate plus an optional scope match").
pub fn check_scopes(required: &[String], present: &[String]) -> Result<(), SecurityError> {
    // Single pass over `required`: the previous implementation scanned
    // `present` twice — once via `all(contains)` for the success check and
    // again via `filter(contains)` to build the missing list. Collecting the
    // unsatisfied scopes once is strictly better on the failure path and equal
    // on the success path, while staying allocation-free in the common
    // fully-satisfied case (`Vec::new` does not allocate until the first push).
    // A `BTreeSet` was considered but rejected: it would allocate and build a
    // tree on every request, which hurts the typical small-scope hot path.
    let mut missing: Vec<String> = Vec::new();
    for req in required {
        if !present.contains(req) {
            missing.push(req.clone());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SecurityError::ScopeDenied {
            required: missing,
            present: present.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use clinkz_wot_td::{data_type::Operation, security_scheme::NoSecurityScheme};

    use crate::{AffordanceTarget, InteractionInput, ThingId};

    #[test]
    fn auth_material_carries_distinct_variants() {
        assert_eq!(
            AuthMaterial::PeerId(String::from("peer-1")),
            AuthMaterial::PeerId(String::from("peer-1"))
        );
        assert_eq!(
            AuthMaterial::BearerToken(alloc::vec![0u8, 1, 2]),
            AuthMaterial::BearerToken(alloc::vec![0u8, 1, 2])
        );
        assert_eq!(
            AuthMaterial::CertificateFingerprint(alloc::vec![0xde, 0xad]),
            AuthMaterial::CertificateFingerprint(alloc::vec![0xde, 0xad])
        );
        assert_eq!(
            AuthMaterial::Other(alloc::vec![]),
            AuthMaterial::Other(alloc::vec![])
        );
    }

    #[test]
    fn principal_id_round_trips() {
        let id = PrincipalId::from("subject-42");
        assert_eq!(id.as_str(), "subject-42");
        assert_eq!(id.into_string(), String::from("subject-42"));
    }

    #[test]
    fn security_error_maps_into_core_error_security_variant() {
        assert!(matches!(
            CoreError::from(SecurityError::MissingCredentials),
            CoreError::Security(SecurityError::MissingCredentials)
        ));
        assert!(matches!(
            CoreError::from(SecurityError::InvalidCredentials),
            CoreError::Security(SecurityError::InvalidCredentials)
        ));
        assert!(matches!(
            CoreError::from(SecurityError::UnsupportedScheme),
            CoreError::Security(SecurityError::UnsupportedScheme)
        ));
        let denied = CoreError::from(SecurityError::ScopeDenied {
            required: alloc::vec![String::from("read")],
            present: alloc::vec![],
        });
        assert!(matches!(
            denied,
            CoreError::Security(SecurityError::ScopeDenied { .. })
        ));
        let failure = CoreError::from(SecurityError::SchemeFailure(String::from("expired token")));
        assert!(matches!(
            failure,
            CoreError::Security(SecurityError::SchemeFailure(_))
        ));
    }

    #[test]
    fn check_scopes_accepts_when_all_required_present() {
        assert!(check_scopes(&[], &[]).is_ok());
        let required = alloc::vec![String::from("read"), String::from("write")];
        let present = alloc::vec![String::from("read"), String::from("write")];
        assert!(check_scopes(&required, &present).is_ok());
        // Extra granted scopes are fine.
        let extra = alloc::vec![
            String::from("read"),
            String::from("write"),
            String::from("admin")
        ];
        assert!(check_scopes(&required, &extra).is_ok());
    }

    #[test]
    fn check_scopes_denies_reporting_unsatisfied_scopes() {
        let required = alloc::vec![String::from("read"), String::from("write")];
        let present = alloc::vec![String::from("read")];
        let err = check_scopes(&required, &present).unwrap_err();
        assert_eq!(
            err,
            SecurityError::ScopeDenied {
                required: alloc::vec![String::from("write")],
                present: alloc::vec![String::from("read")],
            }
        );
    }

    #[test]
    fn default_verify_rejects_inbound_as_unsupported_scheme() {
        struct OutboundOnly;
        impl SecurityProvider for OutboundOnly {
            fn scheme_name(&self) -> &str {
                "nosec"
            }
            fn apply(&self, _: SecurityContext<'_>, _: &mut TransportRequest) -> CoreResult<()> {
                Ok(())
            }
        }

        let request = InboundRequest::new(
            ThingId::from("urn:thing:1"),
            AffordanceTarget::Thing,
            Operation::ReadProperty,
            InteractionInput::empty(),
        );
        let scheme = SecurityScheme::NoSec(NoSecurityScheme::default());
        assert_eq!(
            OutboundOnly.verify(&request, &scheme),
            Err(SecurityError::UnsupportedScheme)
        );
    }
}
