//! WoT Discovery Introduction layer (baseline v4.0 §6 / phase-p1 §1.2).
//!
//! Introduction discovers [`DiscoveryEndpoint`]s — entry points that Exploration
//! (resolvers + directory reader) resolves into Thing Descriptions or directory
//! sessions. Concrete introducers (mDNS, BLE, DNS-SD) are out of scope for v1;
//! only a [`DirectUrlIntroducer`] reference impl is provided.

use alloc::string::String;

use clinkz_wot_td::AbsoluteUri;

#[cfg(feature = "async")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "async")]
use crate::DiscoveryResult;

/// A discovered entry point produced by Introduction.
#[derive(Debug, Clone)]
pub struct DiscoveryEndpoint {
    /// The endpoint URL (absolute).
    pub url: AbsoluteUri,
    /// What the endpoint serves.
    pub kind: EndpointKind,
    /// How the endpoint was discovered.
    pub source: IntroductionSource,
    /// Optional authentication hint for accessing the endpoint.
    pub auth_hint: Option<AuthHint>,
}

/// What a [`DiscoveryEndpoint`] serves.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EndpointKind {
    /// A direct Thing Description.
    ThingDescription,
    /// A Thing Description Directory service.
    ThingDirectory,
    /// A Thing Link (redirect to another endpoint).
    ThingLink,
}

/// How a [`DiscoveryEndpoint`] was introduced.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum IntroductionSource {
    /// Directly configured URL.
    DirectUrl,
    /// Self-description by the Thing.
    SelfDescription,
    /// DNS-SD service discovery.
    DnsSd,
    /// DHCP option.
    Dhcp,
    /// Beacon (BLE, etc.).
    Beacon,
}

/// Authentication hint for accessing a [`DiscoveryEndpoint`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AuthHint {
    /// No auth required.
    None,
    /// Bearer token expected.
    Bearer,
    /// Basic auth expected.
    Basic,
    /// OAuth2.
    OAuth2,
    /// A named security scheme.
    Scheme(String),
}

/// An Introduction mechanism that discovers [`DiscoveryEndpoint`]s.
///
/// v1 ships only the synchronous, locally-resolved path; concrete network
/// introducers are added when a remote backend lands (E6).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait Introducer: Send + Sync {
    /// Discovers endpoints. Async: a real introducer performs network I/O.
    async fn discover_endpoints(&self) -> DiscoveryResult<Vec<DiscoveryEndpoint>>;
}

/// Reference introducer that surfaces one directly-configured URL as a
/// [`DiscoveryEndpoint`] of a given kind. No network I/O.
#[cfg(feature = "async")]
pub struct DirectUrlIntroducer {
    url: AbsoluteUri,
    kind: EndpointKind,
}

#[cfg(feature = "async")]
impl DirectUrlIntroducer {
    /// Creates a direct-URL introducer for a Thing Description endpoint.
    pub fn thing_description(url: AbsoluteUri) -> Self {
        Self {
            url,
            kind: EndpointKind::ThingDescription,
        }
    }

    /// Creates a direct-URL introducer for a Thing Directory endpoint.
    pub fn directory(url: AbsoluteUri) -> Self {
        Self {
            url,
            kind: EndpointKind::ThingDirectory,
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl Introducer for DirectUrlIntroducer {
    async fn discover_endpoints(&self) -> DiscoveryResult<Vec<DiscoveryEndpoint>> {
        Ok(alloc::vec![DiscoveryEndpoint {
            url: self.url.clone(),
            kind: self.kind.clone(),
            source: IntroductionSource::DirectUrl,
            auth_hint: None,
        }])
    }
}
