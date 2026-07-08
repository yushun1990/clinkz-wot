use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::fmt;

use clinkz_wot_td::{form::Form, security_scheme::SecurityScheme, thing::Thing};

use crate::inbound::InboundRequest;
use crate::{CoreError, CoreResult, TransportRequest, WotLock};

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
    entries: WotLock<BTreeMap<String, BTreeMap<String, Credentials>>>,
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self {
            entries: WotLock::new(BTreeMap::new()),
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
    /// The store's lock heals any std poisoning internally and never fails;
    /// the write is always applied. Returns `Ok(())` for API symmetry with
    /// [`CredentialStore`].
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
        });
        Ok(())
    }

    /// Removes credentials for a Thing + scheme pair.
    ///
    /// The lock never fails (poisoning is healed internally), so the removal
    /// is always applied.
    pub fn remove(&self, thing_id: &str, scheme_name: &str) -> CoreResult<()> {
        self.entries.with(|map| {
            if let Some(schemes) = map.get_mut(thing_id) {
                schemes.remove(scheme_name);
                if schemes.is_empty() {
                    map.remove(thing_id);
                }
            }
        });
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
pub trait SecurityProvider: Send + Sync {
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

// ---------------------------------------------------------------------------
// Built-in security providers (P-Sec).
// ---------------------------------------------------------------------------

/// `NoSec` security provider — always passes, returns an anonymous principal.
///
/// Register this provider when a Thing declares `"nosec"` in its
/// `securityDefinitions` and you want inbound requests to pass without
/// authentication. This is the W3C WoT default.
///
/// ```
/// use clinkz_wot_core::{NoSecurityProvider, SecurityProvider};
/// let provider = NoSecurityProvider::new();
/// assert_eq!(provider.scheme_name(), "nosec");
/// ```
pub struct NoSecurityProvider {
    scheme_name: String,
}

impl NoSecurityProvider {
    /// Creates a `NoSec` provider for the default `"nosec"` scheme name.
    pub fn new() -> Self {
        Self {
            scheme_name: String::from("nosec"),
        }
    }

    /// Creates a `NoSec` provider with a custom scheme name (for `Auto`
    /// security definitions that map to a no-op verification).
    pub fn with_scheme_name(scheme_name: impl Into<String>) -> Self {
        Self {
            scheme_name: scheme_name.into(),
        }
    }
}

impl Default for NoSecurityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityProvider for NoSecurityProvider {
    fn scheme_name(&self) -> &str {
        &self.scheme_name
    }

    fn apply(&self, _context: SecurityContext<'_>, _request: &mut TransportRequest) -> CoreResult<()> {
        Ok(())
    }

    fn verify(
        &self,
        _request: &InboundRequest,
        _scheme: &SecurityScheme,
    ) -> Result<Principal, SecurityError> {
        Ok(Principal::anonymous())
    }
}

/// Bearer token security provider — verifies inbound requests against a
/// single configured valid token.
///
/// This is a minimal v0.1 implementation suitable for development and
/// testing. Production deployments should replace this with a provider
/// that validates JWT signatures, checks expiry, introspects tokens via
/// an authorization server, etc.
///
/// The provider compares the inbound `AuthMaterial::BearerToken` bytes
/// against the configured token using a constant-time comparison to
/// avoid timing side channels.
pub struct BearerSecurityProvider {
    scheme_name: String,
    valid_token: Vec<u8>,
    principal_id: String,
    scopes: Vec<String>,
}

impl BearerSecurityProvider {
    /// Creates a Bearer provider for the `"bearer"` scheme name.
    ///
    /// `valid_token` is the exact token bytes an inbound caller must
    /// present in `AuthMaterial::BearerToken`. `principal_id` is the
    /// identity assigned to a successfully verified caller; `scopes` are
    /// the authorization scopes granted to that caller.
    pub fn new(
        valid_token: impl Into<Vec<u8>>,
        principal_id: impl Into<String>,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            scheme_name: String::from("bearer"),
            valid_token: valid_token.into(),
            principal_id: principal_id.into(),
            scopes: scopes.into_iter().map(|s| s.into()).collect(),
        }
    }

    /// Creates a Bearer provider with a custom scheme name (for `OAuth2`
    /// definitions that carry bearer tokens in practice).
    pub fn with_scheme_name(
        scheme_name: impl Into<String>,
        valid_token: impl Into<Vec<u8>>,
        principal_id: impl Into<String>,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            scheme_name: scheme_name.into(),
            valid_token: valid_token.into(),
            principal_id: principal_id.into(),
            scopes: scopes.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl SecurityProvider for BearerSecurityProvider {
    fn scheme_name(&self) -> &str {
        &self.scheme_name
    }

    fn apply(&self, _context: SecurityContext<'_>, _request: &mut TransportRequest) -> CoreResult<()> {
        // Outbound Bearer application is handled by the binding when it
        // builds the transport request; this hook is a no-op for v0.1.
        Ok(())
    }

    fn verify(
        &self,
        request: &InboundRequest,
        _scheme: &SecurityScheme,
    ) -> Result<Principal, SecurityError> {
        match &request.auth {
            Some(AuthMaterial::BearerToken(token)) => {
                if constant_time_eq(token, &self.valid_token) {
                    Ok(Principal {
                        id: PrincipalId::from(self.principal_id.as_str()),
                        scopes: self.scopes.clone(),
                    })
                } else {
                    Err(SecurityError::InvalidCredentials)
                }
            }
            Some(_) => Err(SecurityError::InvalidCredentials),
            None => Err(SecurityError::MissingCredentials),
        }
    }
}

/// Basic auth security provider (RFC 7617) — verifies inbound requests
/// against a single configured username + password.
///
/// Minimal v0.1 implementation for development and testing. Production
/// deployments should use a provider backed by a credential database.
pub struct BasicSecurityProvider {
    scheme_name: String,
    valid_username: String,
    valid_password: String,
    principal_id: String,
    scopes: Vec<String>,
}

impl BasicSecurityProvider {
    /// Creates a Basic auth provider for the `"basic"` scheme name.
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        principal_id: impl Into<String>,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            scheme_name: String::from("basic"),
            valid_username: username.into(),
            valid_password: password.into(),
            principal_id: principal_id.into(),
            scopes: scopes.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl SecurityProvider for BasicSecurityProvider {
    fn scheme_name(&self) -> &str {
        &self.scheme_name
    }

    fn apply(&self, _context: SecurityContext<'_>, _request: &mut TransportRequest) -> CoreResult<()> {
        Ok(())
    }

    fn verify(
        &self,
        request: &InboundRequest,
        _scheme: &SecurityScheme,
    ) -> Result<Principal, SecurityError> {
        match &request.auth {
            Some(AuthMaterial::Basic { username, password }) => {
                if constant_time_eq(username.as_bytes(), self.valid_username.as_bytes())
                    && constant_time_eq(password.as_bytes(), self.valid_password.as_bytes())
                {
                    Ok(Principal {
                        id: PrincipalId::from(self.principal_id.as_str()),
                        scopes: self.scopes.clone(),
                    })
                } else {
                    Err(SecurityError::InvalidCredentials)
                }
            }
            Some(_) => Err(SecurityError::InvalidCredentials),
            None => Err(SecurityError::MissingCredentials),
        }
    }
}

/// Constant-time byte-slice comparison to avoid timing side channels on
/// credential checks. Not a cryptographic constant-time guarantee (the
/// length comparison leaks), but strictly better than `==` for short
/// secrets. Returns `true` when both slices have the same length and
/// content.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod provider_tests {
    use super::*;
    use crate::inbound::InboundRequest;
    use crate::{AffordanceTarget, InteractionInput};
    use alloc::vec;
    use clinkz_wot_td::data_type::Operation;

    fn request_with(auth: Option<AuthMaterial>) -> InboundRequest {
        let mut req = InboundRequest::new(
            crate::ThingId::from("urn:test"),
            AffordanceTarget::Property("x".into()),
            Operation::ReadProperty,
            InteractionInput::empty(),
        );
        req.auth = auth;
        req
    }

    #[test]
    fn nosec_provider_always_passes() {
        let provider = NoSecurityProvider::new();
        let scheme =
            clinkz_wot_td::security_scheme::NoSecurityScheme::default();
        let result = provider.verify(
            &request_with(None),
            &clinkz_wot_td::security_scheme::SecurityScheme::NoSec(scheme),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id.as_str(), "anonymous");
    }

    #[test]
    fn bearer_provider_accepts_valid_token() {
        let provider =
            BearerSecurityProvider::new(b"secret-token".to_vec(), "user-1", ["read"]);
        let scheme =
            clinkz_wot_td::security_scheme::BearerSecurityScheme::default();
        let result = provider.verify(
            &request_with(Some(AuthMaterial::BearerToken(b"secret-token".to_vec()))),
            &clinkz_wot_td::security_scheme::SecurityScheme::Bearer(scheme),
        );
        assert!(result.is_ok());
        let principal = result.unwrap();
        assert_eq!(principal.id.as_str(), "user-1");
        assert_eq!(principal.scopes, vec![String::from("read")]);
    }

    #[test]
    fn bearer_provider_rejects_invalid_token() {
        let provider = BearerSecurityProvider::new(b"correct".to_vec(), "u", Vec::<String>::new());
        let err = provider
            .verify(
                &request_with(Some(AuthMaterial::BearerToken(b"wrong".to_vec()))),
                &clinkz_wot_td::security_scheme::SecurityScheme::Bearer(
                    clinkz_wot_td::security_scheme::BearerSecurityScheme::default(),
                ),
            )
            .unwrap_err();
        assert!(matches!(err, SecurityError::InvalidCredentials));
    }

    #[test]
    fn bearer_provider_rejects_missing_credentials() {
        let provider = BearerSecurityProvider::new(b"x".to_vec(), "u", Vec::<String>::new());
        let err = provider
            .verify(&request_with(None), &clinkz_wot_td::security_scheme::SecurityScheme::Bearer(
                clinkz_wot_td::security_scheme::BearerSecurityScheme::default(),
            ))
            .unwrap_err();
        assert!(matches!(err, SecurityError::MissingCredentials));
    }

    #[test]
    fn basic_provider_accepts_valid_credentials() {
        let provider = BasicSecurityProvider::new("alice", "pw", "alice", ["read", "write"]);
        let result = provider.verify(
            &request_with(Some(AuthMaterial::Basic {
                username: "alice".into(),
                password: "pw".into(),
            })),
            &clinkz_wot_td::security_scheme::SecurityScheme::Basic(
                clinkz_wot_td::security_scheme::BasicSecurityScheme::default(),
            ),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn basic_provider_rejects_wrong_password() {
        let provider = BasicSecurityProvider::new("alice", "correct", "alice", Vec::<String>::new());
        let err = provider
            .verify(
                &request_with(Some(AuthMaterial::Basic {
                    username: "alice".into(),
                    password: "wrong".into(),
                })),
                &clinkz_wot_td::security_scheme::SecurityScheme::Basic(
                    clinkz_wot_td::security_scheme::BasicSecurityScheme::default(),
                ),
            )
            .unwrap_err();
        assert!(matches!(err, SecurityError::InvalidCredentials));
    }

    #[test]
    fn constant_time_eq_matches_builtin_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
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
