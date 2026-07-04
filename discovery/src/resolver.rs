//! Exploration resolver traits (baseline v4.0 §6 / phase-p1 §1.3).
//!
//! Exploration resolves Introduction [`DiscoveryEndpoint`](crate::DiscoveryEndpoint)s
//! into Thing Descriptions (via [`ThingDescriptionResolver`]) or other endpoints
//! (via [`ThingLinkResolver`]). Distinct traits, never collapsed into one
//! container. v1 ships a resolver wrapping the in-memory backend; concrete
//! HTTP/CoAP fetchers are integration points only.

use alloc::boxed::Box;

use clinkz_wot_td::{AbsoluteUri, thing::Thing};

use crate::{DiscoveryEndpoint, DiscoveryResult};

/// Fetches a Thing Description from a direct TD URL.
///
/// Async: a concrete fetcher performs a network round-trip.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait ThingDescriptionResolver: Send + Sync {
    /// Fetches the Thing Description at `url`.
    async fn request_thing_description(&self, url: &AbsoluteUri) -> DiscoveryResult<Thing>;
}

/// Resolves a Thing Link (a TD whose `link` points elsewhere) into the next
/// [`DiscoveryEndpoint`].
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait ThingLinkResolver: Send + Sync {
    /// Resolves `td` (a Thing Link) to the next endpoint.
    async fn resolve_thing_link(&self, td: &Thing) -> DiscoveryResult<DiscoveryEndpoint>;
}
