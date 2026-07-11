//! Zenoh binding constructors (v4.1 AD55).
//!
//! Application code constructs `ZenohServerBinding` and `ZenohBindingTransport`
//! directly and registers them via `ServientBuilder::with_server_binding` /
//! `with_client_binding`.
//!
//! - [`shared`] — backward-compatible single-session topology (server + client
//!   share one pre-opened session; the client ignores TD authorities).
//! - [`server`] — server-only binding from an explicit session.
//! - [`client_pooled`] — spec-aligned multi-router client: sessions are
//!   lazily opened per TD-resolved authority via a session policy.
//! - [`client`] — client-only binding from a pre-built transport.
//!
//! Available only with the `zenoh` feature.

use alloc::sync::Arc;

use clinkz_wot_core::{ClientBinding, ServerBinding};

use crate::{
    DefaultSessionPolicy, ZenohBindingTransport, ZenohServerBinding, ZenohSessionPool,
    ZenohSessionPolicy, ZenohRuntimeTransport,
};

/// Canonical shared-session constructor (legacy single-session topology).
///
/// Given an open `zenoh::Session`, builds a server binding and a client
/// transport that both reference the same session. The client side ignores TD
/// authorities — all interactions ride the single pre-opened session.
///
/// For new deployments that need to consume Things on multiple routers, prefer
/// [`client_pooled`] + [`server`] instead.
pub fn shared(session: zenoh::Session) -> (Arc<dyn ServerBinding>, Arc<dyn ClientBinding>) {
    let transport = ZenohRuntimeTransport::new(session.clone());
    let server: Arc<dyn ServerBinding> = Arc::new(ZenohServerBinding::new(session));
    let client: Arc<dyn ClientBinding> = Arc::new(ZenohBindingTransport::with_transport(transport));
    (server, client)
}

/// Client-only multi-router constructor (spec-aligned).
///
/// Sessions are lazily opened per TD-resolved authority via the given policy.
/// Use this when a Consumer needs to reach Things on different zenoh routers.
/// The default policy ([`DefaultSessionPolicy`]) connects via plain
/// `<transport>/<authority>` TCP/UDP; inject a custom policy for TLS,
/// credentials, or custom locators.
pub fn client_pooled(policy: Arc<dyn ZenohSessionPolicy>) -> Arc<dyn ClientBinding> {
    let pool = ZenohSessionPool::new(policy);
    Arc::new(ZenohBindingTransport::with_transport(pool))
}

/// Client-only multi-router constructor with the default policy.
pub fn client_pooled_default() -> Arc<dyn ClientBinding> {
    client_pooled(Arc::new(DefaultSessionPolicy))
}

/// Constructs a client-only zenoh binding from a shared runtime transport.
///
/// The returned `Arc` can be passed directly to
/// `ServientBuilder::with_client_binding`.
pub fn client(transport: ZenohRuntimeTransport) -> Arc<dyn ClientBinding> {
    Arc::new(ZenohBindingTransport::with_transport(transport))
}

/// Constructs a server-only zenoh binding from a shared session.
///
/// The returned `Arc` can be passed directly to
/// `ServientBuilder::with_server_binding`.
pub fn server(session: zenoh::Session) -> Arc<dyn ServerBinding> {
    Arc::new(ZenohServerBinding::new(session))
}
