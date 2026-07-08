//! `ServientBuilder` — std-host consuming, move-fluent builder.

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ClientBindingFactory, EventBroker, ProtocolBinding, SecurityProvider, ServerBinding,
};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};

use crate::servient::Servient;
use crate::{ServientError, ServientResult};

/// Consuming, move-fluent builder for a [`Servient`].
pub struct ServientBuilder {
    server_bindings: Vec<Arc<dyn ServerBinding>>,
    client_factories: Vec<Arc<dyn ClientBindingFactory>>,
    security_providers: Vec<Arc<dyn SecurityProvider>>,
    discoverer: Option<Arc<dyn Discoverer>>,
}

impl ServientBuilder {
    pub fn new() -> Self {
        Self {
            server_bindings: Vec::new(),
            client_factories: Vec::new(),
            security_providers: Vec::new(),
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

    /// Registers a [`SecurityProvider`] for inbound request verification.
    ///
    /// Multiple providers may be registered, each handling a different
    /// scheme name. During dispatch, the Servient resolves the Thing's
    /// effective security scheme(s) and finds the matching provider by
    /// `scheme_name()`. If no matching provider is found for a declared
    /// scheme, the request is rejected with `SecurityError::UnsupportedScheme`.
    ///
    /// ```no_run
    /// use clinkz_wot_core::{NoSecurityProvider, BearerSecurityProvider};
    /// use clinkz_wot_servient::ServientBuilder;
    /// use std::sync::Arc;
    ///
    /// let servient = ServientBuilder::new()
    ///     .with_security_provider(Arc::new(NoSecurityProvider::new()))
    ///     .with_security_provider(Arc::new(
    ///         BearerSecurityProvider::new(b"secret".to_vec(), "user-1", ["read"])
    ///     ))
    ///     .build()
    ///     .expect("build");
    /// ```
    pub fn with_security_provider(
        mut self,
        provider: Arc<dyn SecurityProvider>,
    ) -> Self {
        self.security_providers.push(provider);
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
            security_providers,
            discoverer,
        } = self;

        let discoverer: Arc<dyn Discoverer> = discoverer
            .unwrap_or_else(|| Arc::new(LocalDiscoverer::new(Arc::new(InMemoryDirectory::new()))));

        let event_broker = EventBroker::new();
        let server_bindings: Arc<[Arc<dyn ServerBinding>]> = Arc::from(server_bindings);
        let client_factories: Arc<[Arc<dyn ClientBindingFactory>]> = Arc::from(client_factories);
        let security_providers: Arc<[Arc<dyn SecurityProvider>]> = if security_providers.is_empty() {
            // Default: register a NoSec provider so Things declaring the
            // W3C default `"nosec"` scheme pass without configuration.
            // Things with empty `security` lists pass regardless (dispatch
            // skips verification entirely). Things declaring other schemes
            // (bearer, basic, ...) still require explicit registration.
            Arc::from([Arc::new(clinkz_wot_core::NoSecurityProvider::new())
                as Arc<dyn SecurityProvider>])
        } else {
            Arc::from(security_providers)
        };

        let servient = Servient::assemble(
            Default::default(),
            Default::default(),
            server_bindings.clone(),
            client_factories,
            security_providers,
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
