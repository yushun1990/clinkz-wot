//! `ServientBuilder` — std-host consuming, move-fluent builder.

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ClientBindingFactory, EventBroker, ProtocolBinding, ServerBinding,
};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};

use crate::servient::Servient;
use crate::{ServientError, ServientResult};

/// Consuming, move-fluent builder for a [`Servient`].
pub struct ServientBuilder {
    server_bindings: Vec<Arc<dyn ServerBinding>>,
    client_factories: Vec<Arc<dyn ClientBindingFactory>>,
    discoverer: Option<Arc<dyn Discoverer>>,
}

impl ServientBuilder {
    pub fn new() -> Self {
        Self {
            server_bindings: Vec::new(),
            client_factories: Vec::new(),
            discoverer: None,
        }
    }

    /// Registers a unified [`ProtocolBinding`]. The Servient extracts the
    /// client factory and server singleton internally; this is the
    /// recommended entry point for protocol configuration.
    ///
    /// Equivalent to invoking both `with_server_binding` (when the binding
    /// provides a server) and `with_client_factory` (when it provides a
    /// client factory) in one call. Asymmetric bindings
    /// (pure-consumer / pure-exposer) are handled by the binding returning
    /// `None` from the relevant `ProtocolBinding` method.
    pub fn with_protocol_binding(
        mut self,
        binding: Arc<dyn ProtocolBinding>,
    ) -> Self {
        if let Some(factory) = binding.client_factory() {
            // Wrap the boxed factory in an `Arc<dyn ClientBindingFactory>`
            // via a thin adapter so the existing registry shape is
            // preserved. P0 keeps the registry API unchanged; P2/P3 may
            // collapse this layer once legacy hooks retire.
            self.client_factories.push(Arc::new(BoxedFactory(factory)));
        }
        if let Some(server) = binding.server() {
            self.server_bindings.push(server);
        }
        self
    }

    /// Legacy entry: registers a server-side binding directly.
    ///
    /// Superseded by [`Self::with_protocol_binding`]. Retained as `pub`
    /// through P0 so existing callers and test fixtures keep working; will
    /// be demoted to `pub(crate)` and removed from the public surface in P1
    /// (see `docs/plan/user-facing-api-implementation-plan.md`).
    pub fn with_server_binding(mut self, binding: Arc<dyn ServerBinding>) -> Self {
        self.server_bindings.push(binding);
        self
    }

    /// Legacy entry: registers a client-side binding factory directly.
    ///
    /// Superseded by [`Self::with_protocol_binding`]. Retained as `pub`
    /// through P0; will be demoted to `pub(crate)` in P1.
    pub fn with_client_factory(
        mut self,
        factory: Arc<dyn ClientBindingFactory>,
    ) -> Self {
        self.client_factories.push(factory);
        self
    }

    pub fn with_discoverer(mut self, discoverer: Arc<dyn Discoverer>) -> Self {
        self.discoverer = Some(discoverer);
        self
    }

    /// Builds the [`Servient`], then calls `configure(&BindingContext)` on
    /// every binding so each picks its dispatch model.
    pub fn build(self) -> ServientResult<Servient> {
        let Self {
            server_bindings,
            client_factories,
            discoverer,
        } = self;

        let discoverer: Arc<dyn Discoverer> = discoverer
            .unwrap_or_else(|| Arc::new(LocalDiscoverer::new(Arc::new(InMemoryDirectory::new()))));

        let event_broker = EventBroker::new();
        let server_bindings: Arc<[Arc<dyn ServerBinding>]> = Arc::from(server_bindings);
        let client_factories: Arc<[Arc<dyn ClientBindingFactory>]> = Arc::from(client_factories);

        let servient = Servient::assemble(
            Default::default(),
            Default::default(),
            server_bindings.clone(),
            client_factories,
            discoverer,
            event_broker.clone(),
        );

        // One configure call per binding — each picks what it needs.
        let dispatch: Arc<dyn clinkz_wot_core::Dispatch> = Arc::new(servient.clone());
        let ctx = clinkz_wot_core::BindingContext {
            event_broker,
            fanin_sender: None,
            dispatch: Some(dispatch),
        };
        for binding in server_bindings.iter() {
            binding.configure(&ctx);
        }

        let _ = ServientError::AlreadyExposed; // suppress unused import
        Ok(servient)
    }
}

/// Adapter that turns a `Box<dyn ClientBindingFactory>` (as returned by
/// `ProtocolBinding::client_factory`) into the `Arc<dyn ClientBindingFactory>`
/// shape stored in the builder's registry.
///
/// `Box<dyn ClientBindingFactory>` is already `Send + Sync` because the
/// trait declares them as supertraits, so this wrapper derives auto traits
/// without any unsafe.
struct BoxedFactory(Box<dyn ClientBindingFactory>);

impl ClientBindingFactory for BoxedFactory {
    fn build(&self) -> alloc::boxed::Box<dyn clinkz_wot_core::ClientBinding> {
        self.0.build()
    }
}

impl Default for ServientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
