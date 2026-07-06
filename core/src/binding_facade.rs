//! Unified protocol-binding facade for application-side composition.
//!
//! [`ProtocolBinding`] is the single trait a concrete protocol binding
//! (zenoh, http, mqtt, ...) implements. Application code never calls its
//! methods directly; the Servient extracts the client/server adapters at
//! build time through
//! [`ServientBuilder::with_protocol_binding`](../../../clinkz_wot_servient/struct.ServientBuilder.html#method.with_protocol_binding).
//!
//! # Why a facade over [`ClientBinding`] + [`ServerBinding`]
//!
//! The engine-internal split between [`ClientBinding`] (outbound,
//! per-Consumed-Thing, async) and [`ServerBinding`] (inbound, singleton,
//! sync) is preserved underneath this facade — the two directions genuinely
//! differ in async-ness, lifetime, and method set — but application code
//! only sees [`ProtocolBinding`]. A single concrete binding type can
//! implement this facade once and share one protocol session across both
//! directions.
//!
//! # `no_std` compatibility
//!
//! This module is `no_std + alloc` compatible; it imposes no runtime or
//! I/O capability beyond what [`ClientBinding`] / [`ServerBinding`] already
//! require.

use alloc::{boxed::Box, sync::Arc};

use crate::{ClientBindingFactory, ServerBinding};

/// Human-readable protocol identifier returned by [`ProtocolBinding::protocol`].
///
/// Carried as `&'static str` because protocol names are conventionally
/// static literals (`"zenoh"`, `"http"`, ...). A typed / owned variant is
/// deferred per `docs/user-facing-api.md` §12.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolId(pub &'static str);

impl ProtocolId {
    /// Returns the inner static string.
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl From<&'static str> for ProtocolId {
    fn from(value: &'static str) -> Self {
        Self(value)
    }
}

impl core::fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0)
    }
}

/// Unified facade for a protocol binding.
///
/// A concrete binding (zenoh, http, mqtt, ...) implements this trait once
/// and registers the instance via
/// [`ServientBuilder::with_protocol_binding`](../../../clinkz_wot_servient/struct.ServientBuilder.html#method.with_protocol_binding).
/// Internally, the Servient extracts the client side (as a per-Consumed-Thing
/// [`ClientBindingFactory`]) and the server side (as a singleton
/// [`ServerBinding`]).
///
/// # Asymmetric bindings
///
/// Pure-consumer bindings (e.g. a cloud controller that never exposes local
/// Things) return `None` from [`Self::server`]. Pure-exposer bindings (e.g.
/// a sensor that never consumes remote Things) return `None` from
/// [`Self::client_factory`]. The [`client_only`] and [`server_only`]
/// constructors adapt such one-directional bindings into a
/// `Arc<dyn ProtocolBinding>`.
///
/// A full two-direction binding implements this trait directly on its
/// concrete type.
///
/// # Object safety
///
/// The trait has no generic methods and takes `&self` throughout; it is
/// object-safe and intended for use as `Arc<dyn ProtocolBinding>`.
pub trait ProtocolBinding: Send + Sync {
    /// Returns the protocol identifier, used for diagnostics and
    /// form-selection logging.
    fn protocol(&self) -> ProtocolId;

    /// Returns a fresh client-side binding factory, or `None` for
    /// pure-exposer bindings.
    ///
    /// The Servient invokes [`ClientBindingFactory::build`] once per
    /// consumed Thing. Each call to this method should yield an independent
    /// factory whose `build()` produces a fresh [`crate::ClientBinding`]
    /// instance sharing the binding's underlying protocol session.
    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>>;

    /// Returns the shared server-side binding, or `None` for pure-consumer
    /// bindings.
    ///
    /// The Servient registers this once and shares the same instance across
    /// all exposed Things.
    fn server(&self) -> Option<Arc<dyn ServerBinding>>;
}

/// Wrapper adapting a [`ClientBindingFactory`] into a client-only
/// [`ProtocolBinding`].
///
/// Construct via [`client_only`].
pub struct ClientOnly<F> {
    protocol: ProtocolId,
    factory: F,
}

impl<F> ProtocolBinding for ClientOnly<F>
where
    F: ClientBindingFactory + Clone + Send + Sync + 'static,
{
    fn protocol(&self) -> ProtocolId {
        self.protocol
    }

    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>> {
        Some(Box::new(self.factory.clone()))
    }

    fn server(&self) -> Option<Arc<dyn ServerBinding>> {
        None
    }
}

/// Wraps a [`ClientBindingFactory`] as a client-only
/// `Arc<dyn ProtocolBinding>`.
///
/// `F` must be `Clone`: each call to [`ProtocolBinding::client_factory`]
/// yields a fresh boxed factory cloned from the original.
pub fn client_only<F>(protocol: impl Into<ProtocolId>, factory: F) -> Arc<dyn ProtocolBinding>
where
    F: ClientBindingFactory + Clone + Send + Sync + 'static,
{
    Arc::new(ClientOnly {
        protocol: protocol.into(),
        factory,
    })
}

/// Wrapper adapting a [`ServerBinding`] into a server-only [`ProtocolBinding`].
///
/// Construct via [`server_only`].
pub struct ServerOnly<S> {
    protocol: ProtocolId,
    server: Arc<S>,
}

impl<S> ProtocolBinding for ServerOnly<S>
where
    S: ServerBinding + Send + Sync + 'static,
{
    fn protocol(&self) -> ProtocolId {
        self.protocol
    }

    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>> {
        None
    }

    fn server(&self) -> Option<Arc<dyn ServerBinding>> {
        Some(self.server.clone())
    }
}

/// Wraps an `Arc<S>` (where `S: ServerBinding`) as a server-only
/// `Arc<dyn ProtocolBinding>`.
pub fn server_only<S>(
    protocol: impl Into<ProtocolId>,
    server: Arc<S>,
) -> Arc<dyn ProtocolBinding>
where
    S: ServerBinding + Send + Sync + 'static,
{
    Arc::new(ServerOnly {
        protocol: protocol.into(),
        server,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::{format, string::String, vec, vec::Vec};

    use crate::{ClientBinding, CoreResult, InteractionOutput};

    /// Captures the count of `build()` invocations so tests can verify that
    /// the facade produces a fresh factory per `client_factory()` call.
    #[derive(Debug, Default, Clone)]
    struct CountingFactory {
        build_calls: Arc<core::sync::atomic::AtomicUsize>,
    }

    impl ClientBindingFactory for CountingFactory {
        fn build(&self) -> Box<dyn ClientBinding> {
            self.build_calls
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            Box::new(NoopClient)
        }
    }

    struct NoopClient;

    #[cfg(feature = "async")]
    #[async_trait::async_trait]
    impl ClientBinding for NoopClient {
        fn supports(
            &self,
            _form: &clinkz_wot_td::form::Form,
            _operation: clinkz_wot_td::data_type::Operation,
        ) -> bool {
            false
        }

        async fn invoke(
            &self,
            _request: crate::BindingRequest,
        ) -> CoreResult<InteractionOutput> {
            Ok(InteractionOutput::empty())
        }
    }

    #[test]
    fn client_only_returns_protocol_id_and_no_server() {
        let build_calls = Arc::new(core::sync::atomic::AtomicUsize::new(0));
        let factory = CountingFactory {
            build_calls: Arc::clone(&build_calls),
        };
        let binding: Arc<dyn ProtocolBinding> = client_only("test", factory);

        assert_eq!(binding.protocol().as_str(), "test");
        assert!(binding.server().is_none());

        let f1 = binding.client_factory().expect("client_factory");
        let f2 = binding.client_factory().expect("client_factory");
        // Each call to client_factory returns a fresh boxed factory; building
        // from each only increments the counter on that clone's snapshot.
        let _ = f1.build();
        let _ = f2.build();
        let _ = binding
            .client_factory()
            .expect("client_factory")
            .build();
        assert_eq!(
            build_calls.load(core::sync::atomic::Ordering::Relaxed),
            3
        );
    }

    /// Minimal `ServerBinding` stub exercising the server-only path. The
    /// method bodies mirror the no-op defaults so we can construct one
    /// without dragging in transport plumbing.
    struct NoopServer;
    impl crate::ServerBinding for NoopServer {
        fn send_response(&self, _response: crate::InboundResponse) {}

        fn register_thing(
            &self,
            _thing_id: &crate::ThingId,
            _td: &clinkz_wot_td::thing::Thing,
        ) -> CoreResult<()> {
            Ok(())
        }

        fn unregister_thing(&self, _thing_id: &crate::ThingId) {}
    }

    #[test]
    fn server_only_returns_protocol_id_and_no_client() {
        let server: Arc<NoopServer> = Arc::new(NoopServer);
        let binding: Arc<dyn ProtocolBinding> = server_only("test-srv", server);

        assert_eq!(binding.protocol().as_str(), "test-srv");
        assert!(binding.client_factory().is_none());
        assert!(binding.server().is_some());
    }

    #[test]
    fn protocol_id_round_trips_through_from_str() {
        let id = ProtocolId::from("zenoh");
        assert_eq!(id.as_str(), "zenoh");
        assert_eq!(format!("{id}"), "zenoh");
        // Vec! used to ensure alloc import is exercised.
        let collected: Vec<ProtocolId> = vec![ProtocolId("a"), ProtocolId("b")];
        assert_eq!(collected.len(), 2);
        let _ = String::from("ensure-string-import");
    }
}
