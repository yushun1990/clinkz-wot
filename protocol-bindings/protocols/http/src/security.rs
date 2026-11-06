//! Concrete Basic credential verification/placement and TLS configuration.
//! These services are live instance/native owners, never prepared definitions.
use crate::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use clinkz_wot_core::{consumer::*, security::*};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use std::sync::{Mutex, atomic::AtomicU64};
use zeroize::Zeroizing;

pub(crate) const BASIC: SecurityRequirement = SecurityRequirement {
    scheme: "basic",
    parameters: &[("in", "header"), ("name", "Authorization")],
};
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

struct ClientState {
    instance: ConsumerInstanceRef,
    target: Box<str>,
    policy: Arc<dyn ConsumerAccessService>,
    credentials: Mutex<Option<HttpBasicCredentials>>,
    revision: AtomicU64,
}
/// Actual per-instance Basic access/credential service. Rotation invalidates
/// outstanding grants. No global lookup, username-as-principal or session cache.
pub struct HttpBasicClientAccess {
    state: Arc<ClientState>,
}
impl HttpBasicClientAccess {
    pub fn try_new(
        instance: ConsumerInstanceRef,
        target: &str,
        policy: Arc<dyn ConsumerAccessService>,
        credentials: Option<HttpBasicCredentials>,
    ) -> CoreResult<Self> {
        let target_info = Target::parse_for(target, true)?;
        if target_info.uri.scheme_str() != Some("https") {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        Ok(Self {
            state: Arc::new(ClientState {
                instance,
                target: target.into(),
                policy,
                credentials: Mutex::new(credentials),
                revision: AtomicU64::new(0),
            }),
        })
    }
    /// Revoke with None or replace credentials. Existing calls cannot adopt the
    /// replacement identity. Exhausted revision refuses without mutation.
    pub fn replace_credentials(&self, credentials: Option<HttpBasicCredentials>) -> CoreResult<()> {
        let mut current = self.state.credentials.lock().unwrap();
        let revision = self
            .state
            .revision
            .load(Ordering::Acquire)
            .checked_add(1)
            .ok_or_else(|| denied(SecurityFailureReason::ProviderFailure))?;
        *current = credentials;
        self.state.revision.store(revision, Ordering::Release);
        Ok(())
    }
}
impl ConsumerAccessService for HttpBasicClientAccess {
    fn security_revision(&self) -> u64 {
        self.state.revision.load(Ordering::Acquire)
    }
    fn supports_security(&self, requirement: &SecurityRequirement) -> bool {
        *requirement == BASIC
    }
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if target != self.state.target.as_ref() || operation != Operation::ReadProperty {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        self.state.policy.authorize(binding, target, operation)
    }
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if instance != &self.state.instance || !instance.is_current() {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        self.authorize(binding, target, operation)?;
        self.state
            .policy
            .authorize_instance(instance, binding, target, operation)?;
        if self.state.credentials.lock().unwrap().is_none() {
            return Err(denied(SecurityFailureReason::MissingCredentials));
        }
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
        if !self.supports_security(requirement) {
            return Err(denied(SecurityFailureReason::UnsupportedScheme));
        }
        self.authorize_instance(instance, binding, target, operation)?;
        Ok(Some(OutboundSecurity::new(BasicGrant {
            state: self.state.clone(),
            binding,
            revision: self.state.revision.load(Ordering::Acquire),
        })))
    }
    fn retained_bytes(&self) -> u64 {
        1024u64.saturating_add(self.state.policy.retained_bytes())
    }
    fn work(&self) -> u64 {
        1024u64.saturating_add(self.state.policy.work().saturating_mul(2))
    }
}
pub(crate) struct BasicGrant {
    state: Arc<ClientState>,
    binding: BindingRegistrationIdentity,
    revision: u64,
}
impl BasicGrant {
    pub(crate) fn for_binding(&self, request: &OutboundRequest) -> bool {
        request.binding_id() == self.binding.binding_id()
            && request.binding_generation() == self.binding.binding_generation()
            && request.artifact().identity().configuration() == self.binding.configuration()
            && request.operation() == Operation::ReadProperty
    }
    pub(crate) fn target(&self) -> &str {
        &self.state.target
    }
    pub(crate) fn check(&self, target: &str) -> CoreResult<()> {
        if target != self.state.target.as_ref()
            || !self.state.instance.is_current()
            || self.state.revision.load(Ordering::Acquire) != self.revision
        {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        Ok(())
    }
    pub(crate) fn header(&self, target: &str) -> CoreResult<hyper::header::HeaderValue> {
        self.check(target)?;
        let credentials = self.state.credentials.lock().unwrap();
        if self.state.revision.load(Ordering::Acquire) != self.revision {
            return Err(denied(SecurityFailureReason::AuthorizationDenied));
        }
        credentials
            .as_ref()
            .map(HttpBasicCredentials::header)
            .ok_or_else(|| denied(SecurityFailureReason::MissingCredentials))
    }
}

/// Actual bounded synchronous verifier. Credential evidence is borrowed only
/// during this callback. Success MUST establish identity/claims from a trusted
/// verifier; accepting a username or transport evidence alone is not verification.
/// work() includes its complete comparison, identity allocation and disposal;
/// callbacks are cooperative, cannot acquire I/O or call application behavior.
pub trait HttpBasicVerifier: Send + Sync {
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
/// Live server key and verification service, owned independently of programs.
#[derive(Clone)]
pub struct HttpServerSecurity {
    pub(crate) tls: Arc<ServerConfig>,
    verifier: Arc<dyn HttpBasicVerifier>,
    work: u64,
    pub(crate) registration: SecurityRegistration,
}
impl HttpServerSecurity {
    pub fn try_new(
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        verifier: Arc<dyn HttpBasicVerifier>,
    ) -> CoreResult<Self> {
        let work = verifier.work();
        if work == 0
            || work > 1_000_000
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
        Ok(Self {
            tls: Arc::new(tls),
            verifier,
            work,
            registration: SecurityRegistration(Arc::new(())),
        })
    }
    pub(crate) fn is_ready(&self) -> bool {
        self.verifier.work() == self.work
    }
    pub(crate) fn work(&self) -> u64 {
        self.work + 4096
    }
    pub(crate) fn verify(
        &self,
        credentials: Option<&HttpBasicCredentials>,
    ) -> CoreResult<Principal> {
        if self.verifier.work() != self.work {
            return Err(denied(SecurityFailureReason::ProviderFailure));
        }
        let credentials =
            credentials.ok_or_else(|| denied(SecurityFailureReason::MissingCredentials))?;
        let principal = self
            .verifier
            .verify(&credentials.username, &credentials.password)
            .map_err(|e| {
                denied(
                    e.security_reason()
                        .unwrap_or(SecurityFailureReason::InvalidCredentials),
                )
            })?;
        if principal.retained_bytes() > 2048
            || principal.id.as_str().len() > 128
            || principal.scopes.len() > 16
            || principal.scopes.iter().any(|s| s.len() > 64)
        {
            return Err(denied(SecurityFailureReason::ProviderFailure));
        }
        Ok(principal)
    }
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
            && Arc::ptr_eq(&self.verifier, &other.verifier)
            && self.work == other.work
    }
}
impl Eq for HttpServerSecurity {}
