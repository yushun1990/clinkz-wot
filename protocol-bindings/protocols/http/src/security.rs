//! Concrete credential verification/placement and independent TLS trust.
//! These services are live instance/native owners, never prepared definitions.
use crate::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use clinkz_wot_core::{consumer::*, security::*};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use std::sync::Mutex;
use zeroize::Zeroizing;

/// Native credential placements. The mask retains every advertised branch;
/// bit N admits exactly evidence set N (Basic=1, API key=2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HttpSecurity(pub(crate) u8);
impl Default for HttpSecurity {
    fn default() -> Self {
        Self(1)
    }
}
impl HttpSecurity {
    fn conjunction(r: &SecurityRequirement, bits: &mut u8) -> CoreResult<()> {
        match r {
            SecurityRequirement::NoSec => Ok(()),
            SecurityRequirement::AllOf(children) if !children.is_empty() => {
                for child in children {
                    Self::conjunction(child, bits)?;
                }
                Ok(())
            }
            SecurityRequirement::Scheme { scheme, parameters } if parameters.len() == 2 => {
                let param = |key: &str| {
                    parameters
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.as_str())
                };
                let bit = match (scheme.as_str(), param("in"), param("name")) {
                    ("basic", Some("header"), Some(n))
                        if n.eq_ignore_ascii_case("Authorization") =>
                    {
                        1
                    }
                    ("apikey", Some("header"), Some(n)) if n.eq_ignore_ascii_case("X-Api-Key") => 2,
                    _ => return Err(denied(SecurityFailureReason::UnsupportedScheme)),
                };
                // Two definitions cannot compete for the same placement.
                if *bits & bit != 0 {
                    return Err(denied(SecurityFailureReason::UnsupportedScheme));
                }
                *bits |= bit;
                Ok(())
            }
            _ => Err(denied(SecurityFailureReason::UnsupportedScheme)),
        }
    }

    /// Inspect a closed selected requirement without allocating or choosing.
    fn selected_requirement(r: &SecurityRequirement) -> CoreResult<u8> {
        if !r.is_conjunction() {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        let mut bits = 0;
        Self::conjunction(r, &mut bits)?;
        Ok(bits)
    }
    pub(crate) fn from_requirement(r: &SecurityRequirement) -> CoreResult<Self> {
        let mut mask = 0;
        for branch in r.alternatives()? {
            let mut bits = 0;
            Self::conjunction(&branch, &mut bits)?;
            if mask & (1 << bits) != 0 {
                return Err(denied(SecurityFailureReason::UnsupportedScheme));
            }
            mask |= 1 << bits;
        }
        Ok(Self(mask))
    }
    pub(crate) fn selected(self) -> Option<u8> {
        (self.0.count_ones() == 1).then(|| self.0.trailing_zeros() as u8)
    }
    pub(crate) fn permits(self, bits: u8) -> bool {
        self.0 & (1 << bits) != 0
    }
    pub(crate) fn uses_basic(self) -> bool {
        self.0 & 0b1010 != 0
    }
    pub(crate) fn uses_api_key(self) -> bool {
        self.0 & 0b1100 != 0
    }
}
#[cfg(test)]
fn basic_requirement() -> SecurityRequirement {
    SecurityRequirement::scheme("basic", &[("in", "header"), ("name", "Authorization")])
}

/// Opaque API key in the single qualified native X-Api-Key header placement.
/// It is never a principal. Local owned material is cleared on disposal.
pub struct HttpApiKey(Zeroizing<Vec<u8>>);
impl HttpApiKey {
    pub fn try_new(key: &[u8]) -> CoreResult<Self> {
        if key.is_empty() || key.len() > 256 || !key.iter().all(|b| (33..=126).contains(b)) {
            return Err(denied(SecurityFailureReason::InvalidCredentials));
        }
        Ok(Self(Zeroizing::new(key.to_vec())))
    }
    fn header(&self) -> hyper::header::HeaderValue {
        let mut value = hyper::header::HeaderValue::from_bytes(&self.0).expect("bounded ASCII key");
        value.set_sensitive(true);
        value
    }
}
impl core::fmt::Debug for HttpApiKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HttpApiKey([redacted])")
    }
}
#[derive(Default)]
pub(crate) struct HttpEvidence {
    pub(crate) basic: Option<HttpBasicCredentials>,
    pub(crate) api_key: Option<HttpApiKey>,
}
impl HttpEvidence {
    pub(crate) fn branch(&self) -> u8 {
        u8::from(self.basic.is_some()) | (u8::from(self.api_key.is_some()) << 1)
    }
}

pub(crate) fn denied(reason: SecurityFailureReason) -> CoreError {
    CoreError::Security {
        reason,
        context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never),
    }
}

/// ASCII RFC 7617 subset. Username/password are bounded, never Debug/Display.
/// Owned source and encoding scratch are cleared on local disposal. Native TLS,
/// Hyper and kernel copies do not carry a process-wide memory-erasure guarantee.
pub struct HttpBasicCredentials {
    username: Zeroizing<Vec<u8>>,
    password: Zeroizing<Vec<u8>>,
}
impl core::fmt::Debug for HttpBasicCredentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HttpBasicCredentials([redacted])")
    }
}
impl HttpBasicCredentials {
    pub fn try_new(username: &[u8], password: &[u8]) -> CoreResult<Self> {
        if username.is_empty()
            || username.len() > 64
            || password.is_empty()
            || password.len() > 128
            || username.contains(&b':')
            || !username
                .iter()
                .chain(password)
                .all(|b| (32..=126).contains(b))
        {
            return Err(denied(SecurityFailureReason::InvalidCredentials));
        }
        Ok(Self {
            username: Zeroizing::new(username.to_vec()),
            password: Zeroizing::new(password.to_vec()),
        })
    }
    fn header(&self) -> hyper::header::HeaderValue {
        let mut plain = Zeroizing::new(Vec::with_capacity(193));
        plain.extend_from_slice(&self.username);
        plain.push(b':');
        plain.extend_from_slice(&self.password);
        let encoded = Zeroizing::new(STANDARD.encode(&*plain));
        let header = Zeroizing::new(format!("Basic {}", encoded.as_str()));
        let mut value =
            hyper::header::HeaderValue::from_str(&header).expect("bounded ASCII base64");
        value.set_sensitive(true);
        value
    }
    pub(crate) fn parse(header: &[u8]) -> CoreResult<Self> {
        let split = header
            .iter()
            .position(|b| *b == b' ')
            .ok_or_else(|| denied(SecurityFailureReason::InvalidCredentials))?;
        let (scheme, encoded) = (&header[..split], &header[split + 1..]);
        if !scheme.eq_ignore_ascii_case(b"Basic") || encoded.len() > 260 {
            return Err(denied(SecurityFailureReason::InvalidCredentials));
        }
        let bytes = Zeroizing::new(
            STANDARD
                .decode(encoded)
                .map_err(|_| denied(SecurityFailureReason::InvalidCredentials))?,
        );
        let split = bytes
            .iter()
            .position(|b| *b == b':')
            .ok_or_else(|| denied(SecurityFailureReason::InvalidCredentials))?;
        let (u, p) = (&bytes[..split], &bytes[split + 1..]);
        Self::try_new(u, p)
    }
}

// One local invalidation sequence covers both credential replacement and
// observed wrapped-policy revision changes. Never hash/XOR independent revisions
// into a value that could accidentally revalidate original work.
struct ClientCredentials {
    current: Option<HttpBasicCredentials>,
    api_key: Option<HttpApiKey>,
    revision: u64,
    policy_revision: u64,
    exhausted: bool,
}
impl ClientCredentials {
    fn advance(&mut self) -> CoreResult<()> {
        if let Some(next) = self.revision.checked_add(1).filter(|_| !self.exhausted) {
            self.revision = next;
            Ok(())
        } else {
            self.exhausted = true;
            Err(denied(SecurityFailureReason::ProviderFailure))
        }
    }
    fn observe_policy(&mut self, revision: u64) {
        if revision != self.policy_revision {
            self.policy_revision = revision;
            let _ = self.advance(); // Exhaustion permanently fences this owner.
        }
    }
}
struct ClientState {
    instance: ConsumerInstanceRef,
    target: Box<str>,
    policy: Arc<dyn ConsumerAccessService>,
    credentials: Mutex<ClientCredentials>,
    capabilities: u8,
}
impl ClientState {
    fn snapshot(&self) -> (u64, bool) {
        // Passive linked metadata is read outside the credential lock.
        let policy_revision = self.policy.security_revision();
        let mut credentials = self.credentials.lock().unwrap();
        credentials.observe_policy(policy_revision);
        (credentials.revision, !credentials.exhausted)
    }
    fn capture_revision(&self) -> CoreResult<u64> {
        let (revision, valid) = self.snapshot();
        if valid {
            Ok(revision)
        } else {
            Err(denied(SecurityFailureReason::ProviderFailure))
        }
    }
    fn check_revision(&self, expected: u64) -> CoreResult<()> {
        if self.capture_revision()? == expected {
            Ok(())
        } else {
            Err(denied(SecurityFailureReason::AuthorizationDenied))
        }
    }
}
/// Actual per-instance composed access/credential service. Credential and wrapped
/// policy revision changes invalidate old bindings/grants. No global lookup,
/// username-as-principal or session cache.
pub struct HttpClientAccess {
    state: Arc<ClientState>,
}
impl HttpClientAccess {
    pub fn try_new(
        instance: ConsumerInstanceRef,
        target: &str,
        policy: Arc<dyn ConsumerAccessService>,
        credentials: Option<HttpBasicCredentials>,
    ) -> CoreResult<Self> {
        Self::construct(instance, target, policy, credentials, None, 1)
    }
    /// Capabilities are fixed by configured credential slots, independently of
    /// the native Binding. Rotation/revocation never changes prepared meaning.
    pub fn try_composed(
        instance: ConsumerInstanceRef,
        target: &str,
        policy: Arc<dyn ConsumerAccessService>,
        basic: Option<HttpBasicCredentials>,
        api_key: Option<HttpApiKey>,
    ) -> CoreResult<Self> {
        let capabilities = u8::from(basic.is_some()) | (u8::from(api_key.is_some()) << 1);
        Self::construct(instance, target, policy, basic, api_key, capabilities)
    }
    fn construct(
        instance: ConsumerInstanceRef,
        target: &str,
        policy: Arc<dyn ConsumerAccessService>,
        credentials: Option<HttpBasicCredentials>,
        api_key: Option<HttpApiKey>,
        capabilities: u8,
    ) -> CoreResult<Self> {
        let target_info = Target::parse_for(target, true)?;
        if target_info.uri.scheme_str() != Some("https") {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        let policy_revision = policy.security_revision();
        Ok(Self {
            state: Arc::new(ClientState {
                instance,
                target: target.into(),
                policy,
                capabilities,
                credentials: Mutex::new(ClientCredentials {
                    current: credentials,
                    api_key,
                    revision: 0,
                    policy_revision,
                    exhausted: false,
                }),
            }),
        })
    }
    /// Revoke with None or replace credentials. Existing calls cannot adopt the
    /// replacement identity. Exhaustion preserves the current credentials and
    /// fences this owner.
    pub fn replace_credentials(&self, credentials: Option<HttpBasicCredentials>) -> CoreResult<()> {
        if self.state.capabilities & 1 == 0 {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        let policy_revision = self.state.policy.security_revision();
        let mut state = self.state.credentials.lock().unwrap();
        state.observe_policy(policy_revision);
        state.advance()?;
        state.current = credentials;
        Ok(())
    }
    pub fn replace_api_key(&self, key: Option<HttpApiKey>) -> CoreResult<()> {
        if self.state.capabilities & 2 == 0 {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        let policy_revision = self.state.policy.security_revision();
        let mut state = self.state.credentials.lock().unwrap();
        state.observe_policy(policy_revision);
        state.advance()?;
        state.api_key = key;
        Ok(())
    }
}
impl ConsumerAccessService for HttpClientAccess {
    fn security_revision(&self) -> u64 {
        self.state.snapshot().0
    }
    fn supports_security(&self, requirement: &SecurityRequirement) -> bool {
        HttpSecurity::selected_requirement(requirement)
            .ok()
            .is_some_and(|bits| bits & !self.state.capabilities == 0)
            && self.state.snapshot().1
    }
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        let revision = self.state.capture_revision()?;
        if target != self.state.target.as_ref() {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        self.state.policy.authorize(binding, target, operation)?;
        self.state.check_revision(revision)
    }
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        let revision = self.state.capture_revision()?;
        if instance != &self.state.instance || !instance.is_current() {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        self.authorize(binding, target, operation)?;
        self.state
            .policy
            .authorize_instance(instance, binding, target, operation)?;
        self.state.check_revision(revision)?;
        Ok(())
    }
    fn outbound_security(
        &self,
        instance: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
        requirement: &SecurityRequirement,
    ) -> CoreResult<Option<OutboundSecurity>> {
        let revision = self.state.capture_revision()?;
        if !self.supports_security(requirement) {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        self.authorize_instance(instance, binding, target, operation)?;
        self.state.check_revision(revision)?;
        let branch = HttpSecurity::selected_requirement(requirement)?;
        if branch == 0 {
            return Ok(None);
        }
        {
            let credentials = self.state.credentials.lock().unwrap();
            if (branch & 1 != 0 && credentials.current.is_none())
                || (branch & 2 != 0 && credentials.api_key.is_none())
            {
                return Err(denied(SecurityFailureReason::MissingCredentials));
            }
        }
        Ok(Some(OutboundSecurity::new(HttpGrant {
            branch,
            state: self.state.clone(),
            binding,
            operation,
            revision,
        })))
    }
    fn retained_bytes(&self) -> u64 {
        1024u64.saturating_add(self.state.policy.retained_bytes())
    }
    fn work(&self) -> u64 {
        1024u64.saturating_add(self.state.policy.work().saturating_mul(2))
    }
}
pub(crate) struct HttpGrant {
    pub(crate) branch: u8,
    state: Arc<ClientState>,
    binding: BindingRegistrationIdentity,
    operation: Operation,
    revision: u64,
}
impl HttpGrant {
    pub(crate) fn for_binding(&self, request: &OutboundRequest) -> bool {
        request.binding_id() == self.binding.binding_id()
            && request.binding_generation() == self.binding.binding_generation()
            && request.artifact().identity().configuration() == self.binding.configuration()
            && request.operation() == self.operation
    }
    pub(crate) fn target(&self) -> &str {
        &self.state.target
    }
    pub(crate) fn check(&self, target: &str) -> CoreResult<()> {
        if target != self.state.target.as_ref() || !self.state.instance.is_current() {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        self.state.check_revision(self.revision)
    }
    pub(crate) fn headers(
        &self,
        target: &str,
    ) -> CoreResult<Vec<(&'static str, hyper::header::HeaderValue)>> {
        self.check(target)?;
        let policy_revision = self.state.policy.security_revision();
        let mut credentials = self.state.credentials.lock().unwrap();
        credentials.observe_policy(policy_revision);
        if credentials.exhausted || credentials.revision != self.revision {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        let mut headers = Vec::with_capacity(2);
        if self.branch & 1 != 0 {
            headers.push((
                "authorization",
                credentials
                    .current
                    .as_ref()
                    .ok_or_else(|| denied(SecurityFailureReason::MissingCredentials))?
                    .header(),
            ));
        }
        if self.branch & 2 != 0 {
            headers.push((
                "x-api-key",
                credentials
                    .api_key
                    .as_ref()
                    .ok_or_else(|| denied(SecurityFailureReason::MissingCredentials))?
                    .header(),
            ));
        }
        Ok(headers)
    }
}

/// Actual bounded synchronous verifier. Credential evidence is borrowed only
/// during this callback. Success MUST establish identity/claims from a trusted
/// verifier; accepting a username or transport evidence alone is not verification.
/// work() includes its complete comparison, identity allocation and disposal;
/// callbacks are cooperative, cannot acquire I/O or call application behavior.
pub trait HttpBasicVerifier: Send + Sync {
    /// Passive non-reused invalidation revision. Mutable verification policy
    /// must advance this on change, even during verify(). Immutable verifiers
    /// may use zero. Reading it performs no I/O or linked callbacks.
    fn revision(&self) -> u64 {
        0
    }
    fn verify(&self, username: &[u8], password: &[u8]) -> CoreResult<Principal>;
    fn work(&self) -> u64;
}

#[derive(Clone)]
pub struct HttpClientTls(pub(crate) Arc<ClientConfig>);
impl HttpClientTls {
    /// Explicit trust anchors; normal rustls chain, expiry and IP SAN checks
    /// remain enabled. TLS 1.3 only; no dangerous verifier injection API.
    pub fn from_roots(roots: Vec<CertificateDer<'static>>) -> CoreResult<Self> {
        if roots.is_empty() || roots.len() > 8 || roots.iter().any(|c| c.len() > 16384) {
            return Err(denied(SecurityFailureReason::ProviderFailure));
        }
        let mut store = RootCertStore::empty();
        for cert in roots {
            store
                .add(cert)
                .map_err(|_| denied(SecurityFailureReason::ProviderFailure))?;
        }
        let mut config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .map_err(|_| denied(SecurityFailureReason::ProviderFailure))?
                .with_root_certificates(store)
                .with_no_client_auth();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        config.resumption = rustls::client::Resumption::disabled();
        config.max_fragment_size = Some(4096);
        Ok(Self(Arc::new(config)))
    }
}
#[derive(Clone, Debug)]
pub(crate) struct SecurityRegistration(Arc<()>);
impl PartialEq for SecurityRegistration {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SecurityRegistration {}
/// Independent API-key verifier. Success establishes a trusted identity/claims;
/// a key's presence or bytes alone never establish a Principal. The cooperative
/// callback/work and passive non-reused revision contracts match HttpBasicVerifier.
/// No I/O or application behavior is permitted in these callbacks.
pub trait HttpApiKeyVerifier: Send + Sync {
    fn revision(&self) -> u64 {
        0
    }
    fn verify(&self, key: &[u8]) -> CoreResult<Principal>;
    fn work(&self) -> u64;
}
struct VerifierRevisions {
    observed: (u64, u64),
    revision: u64,
    exhausted: bool,
}
/// Live TLS key and independently qualified verification services. Programs
/// retain only immutable native requirements and the original pairing identity.
#[derive(Clone)]
pub struct HttpServerSecurity {
    pub(crate) tls: Arc<ServerConfig>,
    basic: Option<Arc<dyn HttpBasicVerifier>>,
    api_key: Option<Arc<dyn HttpApiKeyVerifier>>,
    work: (u64, u64),
    revisions: Arc<Mutex<VerifierRevisions>>,
    pub(crate) registration: SecurityRegistration,
}
impl HttpServerSecurity {
    pub fn try_new(
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        verifier: Arc<dyn HttpBasicVerifier>,
    ) -> CoreResult<Self> {
        Self::try_composed(chain, key, Some(verifier), None)
    }

    pub fn try_composed(
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        basic: Option<Arc<dyn HttpBasicVerifier>>,
        api_key: Option<Arc<dyn HttpApiKeyVerifier>>,
    ) -> CoreResult<Self> {
        let work = (
            basic.as_ref().map_or(0, |v| v.work()),
            api_key.as_ref().map_or(0, |v| v.work()),
        );
        if (basic.is_some() && work.0 == 0)
            || (api_key.is_some() && work.1 == 0)
            || work.0.saturating_add(work.1) > 1_000_000
            || chain.is_empty()
            || chain.len() > 8
            || chain.iter().any(|c| c.len() > 16384)
            || key.secret_der().len() > 16384
        {
            return Err(denied(SecurityFailureReason::ProviderFailure));
        }
        let mut tls =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .map_err(|_| denied(SecurityFailureReason::ProviderFailure))?
                .with_no_client_auth()
                .with_single_cert(chain, key)
                .map_err(|_| denied(SecurityFailureReason::ProviderFailure))?;
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        tls.send_tls13_tickets = 0;
        tls.max_fragment_size = Some(4096);
        let observed = (
            basic.as_ref().map_or(0, |v| v.revision()),
            api_key.as_ref().map_or(0, |v| v.revision()),
        );
        Ok(Self {
            tls: Arc::new(tls),
            basic,
            api_key,
            work,
            revisions: Arc::new(Mutex::new(VerifierRevisions {
                observed,
                revision: 0,
                exhausted: false,
            })),
            registration: SecurityRegistration(Arc::new(())),
        })
    }
    pub(crate) fn supports(&self, requirement: HttpSecurity) -> bool {
        self.is_ready()
            && (!requirement.uses_basic() || self.basic.is_some())
            && (!requirement.uses_api_key() || self.api_key.is_some())
    }
    pub(crate) fn is_ready(&self) -> bool {
        self.basic.as_ref().map_or(0, |v| v.work()) == self.work.0
            && self.api_key.as_ref().map_or(0, |v| v.work()) == self.work.1
            && !self.revisions.lock().unwrap().exhausted
    }
    pub(crate) fn work(&self) -> u64 {
        self.work.0 + self.work.1 + 4096
    }
    pub(crate) fn revision(&self) -> u64 {
        let observed = (
            self.basic.as_ref().map_or(0, |v| v.revision()),
            self.api_key.as_ref().map_or(0, |v| v.revision()),
        );
        let mut state = self.revisions.lock().unwrap();
        if observed != state.observed {
            state.observed = observed;
            if let Some(next) = state.revision.checked_add(1).filter(|_| !state.exhausted) {
                state.revision = next;
            } else {
                state.exhausted = true;
            }
        }
        state.revision
    }
    pub(crate) fn verify(
        &self,
        requirement: HttpSecurity,
        evidence: &HttpEvidence,
    ) -> CoreResult<Principal> {
        let revision = self.revision();
        if !self.supports(requirement) {
            return Err(denied(SecurityFailureReason::ProviderFailure));
        }
        let branch = evidence.branch();
        if !requirement.permits(branch) {
            return Err(denied(SecurityFailureReason::MissingCredentials));
        }
        // Exact evidence presence chooses the branch. Failed verification never
        // tries another branch, including anonymous or single-factor access.
        let mut principal = None;
        if let Some(credentials) = &evidence.basic {
            principal = Some(bounded_principal(
                self.basic
                    .as_ref()
                    .ok_or_else(|| denied(SecurityFailureReason::UnsupportedScheme))?
                    .verify(&credentials.username, &credentials.password)
                    .map_err(redact_verification)?,
            )?);
        }
        if let Some(key) = &evidence.api_key {
            // An earlier verifier can invalidate this callback's captured work
            // or policy. Reject before invoking it, in addition to the final
            // whole-conjunction check before publishing the Principal.
            if self.revision() != revision || !self.is_ready() {
                return Err(denied(SecurityFailureReason::ProviderFailure));
            }
            let verified = bounded_principal(
                self.api_key
                    .as_ref()
                    .ok_or_else(|| denied(SecurityFailureReason::UnsupportedScheme))?
                    .verify(&key.0)
                    .map_err(redact_verification)?,
            )?;
            if principal.as_ref().is_some_and(|p| p != &verified) {
                return Err(denied(SecurityFailureReason::InvalidCredentials));
            }
            principal = Some(verified);
        }
        let principal = principal.unwrap_or_else(Principal::anonymous);
        if self.revision() != revision
            || !self.is_ready()
            || principal.retained_bytes() > 2048
            || principal.id.as_str().len() > 128
            || principal.scopes.len() > 16
            || principal.scopes.iter().any(|s| s.len() > 64)
        {
            return Err(denied(SecurityFailureReason::ProviderFailure));
        }
        Ok(principal)
    }
}
fn bounded_principal(principal: Principal) -> CoreResult<Principal> {
    if principal.retained_bytes() > 2048
        || principal.id.as_str().len() > 128
        || principal.scopes.len() > 16
        || principal.scopes.iter().any(|s| s.len() > 64)
    {
        return Err(denied(SecurityFailureReason::ProviderFailure));
    }
    Ok(principal)
}
fn redact_verification(e: CoreError) -> CoreError {
    denied(
        e.security_reason()
            .unwrap_or(SecurityFailureReason::InvalidCredentials),
    )
}
impl core::fmt::Debug for HttpServerSecurity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HttpServerSecurity([redacted])")
    }
}
impl PartialEq for HttpServerSecurity {
    fn eq(&self, other: &Self) -> bool {
        self.registration == other.registration
            && Arc::ptr_eq(&self.tls, &other.tls)
            && Arc::ptr_eq(&self.revisions, &other.revisions)
            && self.work == other.work
    }
}
impl Eq for HttpServerSecurity {}

#[cfg(test)]
mod tests {
    use super::*;

    struct Policy;
    impl ConsumerAccessService for Policy {
        fn authorize(
            &self,
            _: BindingRegistrationIdentity,
            _: &str,
            op: Operation,
        ) -> CoreResult<()> {
            // Application policy authorizes Write independently of scheme support.
            if op == Operation::WriteProperty {
                Ok(())
            } else {
                Err(denied(SecurityFailureReason::AuthorizationDenied))
            }
        }
        fn authorize_instance(
            &self,
            _: &ConsumerInstanceRef,
            id: BindingRegistrationIdentity,
            target: &str,
            op: Operation,
        ) -> CoreResult<()> {
            self.authorize(id, target, op)
        }
        fn retained_bytes(&self) -> u64 {
            0
        }
        fn work(&self) -> u64 {
            1
        }
    }

    #[test]
    fn issued_composed_grant_cannot_cross_operation_instance_registration_or_destination() {
        let config = HttpConsumerConfig::for_https_ip().unwrap();
        let binding = BindingRegistrationIdentity::new(
            BindingId::new(77),
            BindingGeneration::INITIAL,
            config.configuration.configuration(),
            COMPATIBILITY,
            0,
        );
        let instance =
            ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
        let access = HttpClientAccess::try_composed(
            instance.reference(),
            "https://127.0.0.1:443/on",
            Arc::new(Policy),
            Some(HttpBasicCredentials::try_new(b"user", b"password").unwrap()),
            Some(HttpApiKey::try_new(b"unit-key").unwrap()),
        )
        .unwrap();
        let requirement = SecurityRequirement::AllOf(vec![
            basic_requirement(),
            SecurityRequirement::scheme("apikey", &[("in", "header"), ("name", "X-Api-Key")]),
        ]);
        let issue = |instance: &ConsumerInstanceRef, op| {
            access.outbound_security(
                instance,
                binding,
                "https://127.0.0.1:443/on",
                op,
                &requirement,
            )
        };
        assert!(issue(&instance.reference(), Operation::ReadProperty).is_err());
        assert!(issue(&instance.reference(), Operation::WriteAllProperties).is_err());
        let other = ConsumerInstance::new(instance.reference().slot());
        assert!(issue(&other.reference(), Operation::WriteProperty).is_err());
        let capability = issue(&instance.reference(), Operation::WriteProperty)
            .unwrap()
            .unwrap();
        let grant = capability.downcast_ref::<HttpGrant>().unwrap();
        let request = |id, op| {
            let artifact = BindingArtifactIdentity::new(
                PlanSetGeneration::INITIAL,
                PlanId::new(SlotIndex::new(0), Generation::INITIAL),
                id,
                binding.binding_generation(),
                binding.configuration(),
                COMPATIBILITY,
                BindingArtifactRole::ConsumerCall,
            );
            OutboundRequest::finite(
                BindingArtifactRef::new(artifact, SlotIndex::new(0)),
                op,
                (op == Operation::WriteProperty)
                    .then(|| Payload::new(b"1".as_slice(), "application/json")),
                Default::default(),
                None,
            )
            .unwrap()
        };
        assert!(grant.for_binding(&request(binding.binding_id(), Operation::WriteProperty)));
        assert!(!grant.for_binding(&request(binding.binding_id(), Operation::ReadProperty)));
        assert!(!grant.for_binding(&request(BindingId::new(78), Operation::WriteProperty)));
        assert!(grant.check("https://127.0.0.1:443/other").is_err());
        assert_eq!(grant.headers("https://127.0.0.1:443/on").unwrap().len(), 2);
        access.replace_api_key(None).unwrap();
        assert!(grant.check("https://127.0.0.1:443/on").is_err());
    }
}
