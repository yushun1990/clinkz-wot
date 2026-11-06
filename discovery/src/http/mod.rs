//! Host HTTP/1.1 TD-server and Directory array-listing client.
//!
//! Introduction is explicit configuration. A pinned socket address prevents
//! DNS rebinding; TLS still verifies the configured URL's DNS name or IP SAN.
//! Continuations cannot leave the collection's origin/path. No redirect,
//! ThingLink, context, TD Form or authentication challenge is followed.
use crate::{DiscoveryError as E, DiscoveryResult as Result};
use clinkz_wot_td::thing::Thing;
use hyper::Uri;
use rustls::pki_types::CertificateDer;
use std::{
    format,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
    vec,
};
use tokio::sync::Notify;
use zeroize::Zeroizing;

mod admission;
mod links;
mod transport;

/// Positive per-session limits, checked against finite implementation ceilings.
#[derive(Clone, Copy, Debug)]
pub struct DiscoveryLimits {
    pub page_size: usize,
    pub max_pages: usize,
    pub max_results: usize,
    pub page_bytes: usize,
    pub td_bytes: usize,
    pub total_bytes: usize,
    pub request_timeout: Duration,
    pub session_timeout: Duration,
}
impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            page_size: 16,
            max_pages: 16,
            max_results: 128,
            page_bytes: 262_144,
            td_bytes: 32_768,
            total_bytes: 1_048_576,
            request_timeout: Duration::from_secs(10),
            session_timeout: Duration::from_secs(60),
        }
    }
}
impl DiscoveryLimits {
    fn check(self) -> Result<Self> {
        if self.page_size == 0
            || self.page_size > 128
            || self.max_pages == 0
            || self.max_pages > 128
            || self.max_results == 0
            || self.max_results > 1024
            || self.page_bytes == 0
            || self.page_bytes > 1_048_576
            || self.td_bytes == 0
            || self.td_bytes > self.page_bytes
            || self.total_bytes < self.page_bytes
            || self.total_bytes > 16_777_216
            || self.request_timeout.is_zero()
            || self.request_timeout > Duration::from_secs(120)
            || self.session_timeout.is_zero()
            || self.session_timeout > Duration::from_secs(600)
        {
            return Err(E::Configuration);
        }
        Ok(self)
    }
}

/// Provisioned native transport trust. Cleartext is an explicit loopback-only
/// development choice; credentials require verified TLS.
#[derive(Clone)]
pub struct DiscoveryTransport(Option<Arc<rustls::ClientConfig>>);
impl DiscoveryTransport {
    pub fn tls(roots: Vec<CertificateDer<'static>>) -> Result<Self> {
        if roots.is_empty() || roots.len() > 8 || roots.iter().any(|r| r.len() > 16_384) {
            return Err(E::Configuration);
        }
        let mut store = rustls::RootCertStore::empty();
        for root in roots {
            store.add(root).map_err(|_| E::Configuration)?;
        }
        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| E::Configuration)?
        .with_root_certificates(store)
        .with_no_client_auth();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        config.resumption = rustls::client::Resumption::disabled();
        config.max_fragment_size = Some(4096);
        Ok(Self(Some(Arc::new(config))))
    }
    pub fn cleartext_loopback() -> Self {
        Self(None)
    }
}
impl core::fmt::Debug for DiscoveryTransport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(if self.0.is_some() {
            "DiscoveryTransport(TLS)"
        } else {
            "DiscoveryTransport(loopback)"
        })
    }
}

/// Application-configured trusted retrieval destination. This capability covers
/// metadata retrieval only. It cannot authorize a TD's native interactions.
#[derive(Clone)]
pub struct TrustedDiscoveryEndpoint {
    url: String,
    uri: Uri,
    address: SocketAddr,
    transport: DiscoveryTransport,
}
impl TrustedDiscoveryEndpoint {
    /// URL port must match the pinned socket. TLS verifies URL host, not an
    /// untrusted peer-provided name. No runtime resolver, proxy or redirect.
    pub fn new(url: &str, address: SocketAddr, transport: DiscoveryTransport) -> Result<Self> {
        let uri = parse_url(url)?;
        let secure = uri.scheme_str() == Some("https");
        if secure {
            rustls::pki_types::ServerName::try_from(
                uri.host().unwrap().trim_matches(['[', ']']).to_string(),
            )
            .map_err(|_| E::DestinationDenied)?;
        }
        if secure != transport.0.is_some()
            || uri.port_u16().unwrap_or(if secure { 443 } else { 80 }) != address.port()
            || address.ip().is_unspecified()
            || address.ip().is_multicast()
            || (!secure
                && (!address.ip().is_loopback()
                    || uri
                        .host()
                        .and_then(|h| h.trim_matches(['[', ']']).parse::<std::net::IpAddr>().ok())
                        .is_none_or(|h| h != address.ip())))
        {
            return Err(E::DestinationDenied);
        }
        // IP literals cannot be pinned to a different address. DNS hosts may be
        // deliberately pinned by deployment; the certificate must match the host.
        if let Ok(ip) = uri
            .host()
            .unwrap()
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            && ip != address.ip()
        {
            return Err(E::DestinationDenied);
        }
        Ok(Self {
            url: url.into(),
            uri,
            address,
            transport,
        })
    }
}
impl core::fmt::Debug for TrustedDiscoveryEndpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("TrustedDiscoveryEndpoint([redacted])")
    }
}
fn parse_url(url: &str) -> Result<Uri> {
    if url.len() > 2048 || url.bytes().any(|b| b <= 32 || b >= 127 || b == b'\\') {
        return Err(E::DestinationDenied);
    }
    let parsed = fluent_uri::Uri::parse(url).map_err(|_| E::DestinationDenied)?;
    if parsed.fragment().is_some() || parsed.authority().is_none_or(|a| a.userinfo().is_some()) {
        return Err(E::DestinationDenied);
    }
    let authority = parsed.authority().unwrap();
    if authority.port().is_some()
        && authority
            .port_to_u16()
            .map_err(|_| E::DestinationDenied)?
            .is_none_or(|p| p == 0)
    {
        return Err(E::DestinationDenied);
    }
    let uri: Uri = url.parse().map_err(|_| E::DestinationDenied)?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.host().is_none() {
        return Err(E::DestinationDenied);
    }
    Ok(uri)
}

/// Bounded ASCII Basic credentials, scoped to the exact configured origin/path.
/// Neither source nor encoding scratch is formatted. Headers are sensitive;
/// this makes no process-wide erasure guarantee for native/kernel copies.
pub struct DiscoveryCredentials {
    audience: String,
    username: Zeroizing<Vec<u8>>,
    password: Zeroizing<Vec<u8>>,
}
impl DiscoveryCredentials {
    pub fn basic(
        endpoint: &TrustedDiscoveryEndpoint,
        username: &[u8],
        password: &[u8],
    ) -> Result<Self> {
        if endpoint.transport.0.is_none()
            || username.is_empty()
            || username.len() > 64
            || password.is_empty()
            || password.len() > 128
            || username.contains(&b':')
            || !username
                .iter()
                .chain(password)
                .all(|b| (32..=126).contains(b))
        {
            return Err(E::Configuration);
        }
        Ok(Self {
            audience: audience(&endpoint.uri),
            username: Zeroizing::new(username.to_vec()),
            password: Zeroizing::new(password.to_vec()),
        })
    }
    fn header(&self, uri: &Uri) -> Result<hyper::header::HeaderValue> {
        use base64::Engine;
        if self.audience != audience(uri) || uri.scheme_str() != Some("https") {
            return Err(E::DestinationDenied);
        }
        let mut plain = Zeroizing::new(Vec::with_capacity(193));
        plain.extend_from_slice(&self.username);
        plain.push(b':');
        plain.extend_from_slice(&self.password);
        let encoded = Zeroizing::new(base64::engine::general_purpose::STANDARD.encode(&*plain));
        let header = Zeroizing::new(format!("Basic {}", encoded.as_str()));
        let mut value =
            hyper::header::HeaderValue::from_str(&header).map_err(|_| E::Configuration)?;
        value.set_sensitive(true);
        Ok(value)
    }
}
impl core::fmt::Debug for DiscoveryCredentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("DiscoveryCredentials([redacted])")
    }
}
fn audience(uri: &Uri) -> String {
    format!(
        "{}://{}{}",
        uri.scheme_str().unwrap(),
        uri.authority().unwrap(),
        uri.path()
    )
}

#[derive(Default)]
struct Cancellation {
    cancelled: AtomicBool,
    notify: Notify,
}
/// Explicit cancellation, independent of the caller dropping an in-flight future.
#[derive(Clone, Default)]
pub struct DiscoveryCancellation(Arc<Cancellation>);
impl DiscoveryCancellation {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        self.0.notify.notify_waiters();
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    async fn cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// Immutable description admitted through the TD Basic/inspection layer.
/// No execution, selected Form support, endpoint trust or permission is implied.
/// Unknown extensions remain preserved for later shared semantic selection.
pub struct DiscoveredThing {
    thing: Thing,
}
impl DiscoveredThing {
    pub fn thing(&self) -> &Thing {
        &self.thing
    }
    pub fn into_thing(self) -> Thing {
        self.thing
    }
}
impl core::fmt::Debug for DiscoveredThing {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("DiscoveredThing([untrusted metadata])")
    }
}

struct Client {
    endpoint: TrustedDiscoveryEndpoint,
    credentials: Option<DiscoveryCredentials>,
    limits: DiscoveryLimits,
}
/// One direct introduction, one exploration session, no implicit Servient state.
/// Each next_page attempt is terminal on failure or abandonment. Earlier complete
/// pages are application-owned; partial pages never escape.
pub struct HttpDiscoverySession {
    client: Option<Client>,
    next: Option<String>,
    collection: Option<String>,
    api_base: String,
    offset: usize,
    etag: Option<String>,
    last_id: Option<String>,
    pages: usize,
    results: usize,
    bytes: usize,
    deadline: tokio::time::Instant,
    error: Option<E>,
}
impl HttpDiscoverySession {
    pub fn td(
        endpoint: TrustedDiscoveryEndpoint,
        credentials: Option<DiscoveryCredentials>,
        limits: DiscoveryLimits,
    ) -> Result<Self> {
        Self::new(endpoint, credentials, limits, false)
    }
    /// endpoint is the explicitly provisioned Directory collection URL, ending
    /// in '/things'. The API base is its parent URL. No Directory TD is inferred.
    pub fn directory(
        endpoint: TrustedDiscoveryEndpoint,
        credentials: Option<DiscoveryCredentials>,
        limits: DiscoveryLimits,
    ) -> Result<Self> {
        Self::new(endpoint, credentials, limits, true)
    }
    fn new(
        endpoint: TrustedDiscoveryEndpoint,
        credentials: Option<DiscoveryCredentials>,
        limits: DiscoveryLimits,
        directory: bool,
    ) -> Result<Self> {
        let limits = limits.check()?;
        let api_base = links::resolve(&endpoint.url, ".")?;
        let collection = if directory {
            if !endpoint.uri.path().ends_with("/things") || endpoint.uri.query().is_some() {
                return Err(E::Configuration);
            }
            Some(endpoint.url.clone())
        } else {
            None
        };
        if credentials
            .as_ref()
            .is_some_and(|c| c.audience != audience(&endpoint.uri))
        {
            return Err(E::DestinationDenied);
        }
        let next = if directory {
            format!("{}?limit={}", endpoint.url, limits.page_size)
        } else {
            endpoint.url.clone()
        };
        Ok(Self {
            client: Some(Client {
                endpoint,
                credentials,
                limits,
            }),
            next: Some(next),
            collection,
            api_base,
            offset: 0,
            etag: None,
            last_id: None,
            pages: 0,
            results: 0,
            bytes: 0,
            deadline: tokio::time::Instant::now() + limits.session_timeout,
            error: None,
        })
    }
    pub fn error(&self) -> Option<E> {
        self.error
    }
    /// Synchronous local stop drops trust/credential owners and continuation
    /// backing. Dropping an in-flight next_page future first releases its socket.
    pub fn stop(&mut self) {
        self.finish(None);
    }
    fn finish(&mut self, error: Option<E>) {
        if self.error.is_none() {
            self.error = error;
        }
        self.next = None;
        self.client = None;
        self.etag = None;
        self.last_id = None;
        self.collection = None;
        self.api_base.clear();
    }
    pub async fn next_page(
        &mut self,
        cancel: &DiscoveryCancellation,
    ) -> Result<Option<Vec<DiscoveredThing>>> {
        let Some(url) = self.next.take() else {
            return Ok(None);
        };
        // Mark before the first await. A dropped future cannot replay a request
        // or treat an incomplete page as a successful empty result.
        self.error = Some(E::Interrupted);
        let client = self.client.take().expect("live next owns client");
        let result = self.retrieve(&client, &url, cancel).await;
        match result {
            Ok((things, next, etag)) => {
                self.pages += 1;
                self.results += things.len();
                self.offset += things.len();
                self.last_id = things
                    .last()
                    .and_then(|t| t.thing.id.as_ref().map(|id| id.as_str().into()))
                    .or(self.last_id.take());
                self.error = None;
                if let Some(next) = next {
                    self.next = Some(next);
                    self.etag = etag;
                    self.client = Some(client);
                } else {
                    self.finish(None);
                }
                Ok(Some(things))
            }
            Err(error) => {
                self.error = Some(error);
                self.finish(Some(error));
                Err(error)
            }
        }
    }
    async fn retrieve(
        &mut self,
        client: &Client,
        url: &str,
        cancel: &DiscoveryCancellation,
    ) -> Result<(Vec<DiscoveredThing>, Option<String>, Option<String>)> {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        if self.pages >= client.limits.max_pages {
            return Err(E::Limit);
        }
        let end = self
            .deadline
            .min(tokio::time::Instant::now() + client.limits.request_timeout);
        let retrieval = async {
            let (body, headers) = transport::get(client, url, self.collection.is_some()).await?;
            self.bytes = self.bytes.checked_add(body.len()).ok_or(E::Limit)?;
            if self.bytes > client.limits.total_bytes {
                return Err(E::Limit);
            }
            let things =
                admission::page(&body, self.collection.is_some(), client.limits, cancel).await?;
            if self.results + things.len() > client.limits.max_results {
                return Err(E::Limit);
            }
            let (next, etag) = if let Some(collection) = &self.collection {
                let navigation = links::navigation(
                    &headers,
                    &self.api_base,
                    collection,
                    self.etag.as_deref(),
                    self.offset,
                    things.len(),
                    client.limits.page_size,
                )?;
                if (navigation.0.is_some() || navigation.1.is_some())
                    && things.len() > client.limits.page_size
                {
                    return Err(E::Limit);
                }
                let mut previous = self.last_id.as_deref();
                for thing in &things {
                    let id = thing
                        .thing
                        .id
                        .as_ref()
                        .ok_or(E::InvalidThingDescription)?
                        .as_str();
                    if previous.is_some_and(|p| p >= id) {
                        return Err(E::InvalidContinuation);
                    }
                    previous = Some(id);
                }
                navigation
            } else {
                (None, None)
            };
            if next.is_some()
                && (self.pages + 1 >= client.limits.max_pages
                    || self.results + things.len() >= client.limits.max_results)
            {
                return Err(E::Limit);
            }
            Ok((things, next, etag))
        };
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(E::Cancelled),
            _ = tokio::time::sleep_until(end) => Err(E::Timeout),
            result = retrieval => result,
        }
    }
}
