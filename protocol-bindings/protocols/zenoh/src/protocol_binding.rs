//! Zenoh binding constructors (v4.1 AD55).
//!
//! Replaces the v4.0 `ZenohProtocolBinding` facade. Application code
//! constructs `ZenohServerBinding` and `ZenohBindingTransport` directly and
//! registers them via `ServientBuilder::with_server_binding` /
//! `with_client_binding`. The canonical shared-session topology is
//! encapsulated in [`shared`].
//!
//! Available only with the `zenoh` feature.

use alloc::sync::Arc;

use clinkz_wot_core::{ClientBinding, ServerBinding};

use crate::{ZenohBindingTransport, ZenohRuntimeTransport, ZenohServerBinding};

/// Canonical shared-session constructor.
///
/// Given an open `zenoh::Session`, builds a server binding and a client
/// transport that both reference the same session, returning them as
/// `Arc<dyn ServerBinding>` and `Arc<dyn ClientBinding>` ready for
/// `ServientBuilder::with_server_binding` / `with_client_binding`.
pub fn shared(session: zenoh::Session) -> (Arc<dyn ServerBinding>, Arc<dyn ClientBinding>) {
    let transport = ZenohRuntimeTransport::new(session.clone());
    let server: Arc<dyn ServerBinding> = Arc::new(ZenohServerBinding::new(session));
    let client: Arc<dyn ClientBinding> = Arc::new(ZenohBindingTransport::with_transport(transport));
    (server, client)
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
