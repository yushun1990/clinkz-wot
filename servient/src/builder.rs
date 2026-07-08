//! `ServientBuilder` — std-host consuming, move-fluent builder (v4.1 AD55–AD58).

use alloc::{sync::Arc, vec::Vec};

use clinkz_wot_core::{ClientBinding, EventBroker, SecurityProvider, ServerBinding};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};

use crate::servient::Servient;
use crate::{ServientError, ServientResult};

/// Consuming, move-fluent builder for a [`Servient`].
pub struct ServientBuilder {
    server_bindings: Vec<Arc<dyn ServerBinding>>,
    #[cfg(feature = "async")]
    client_bindings: Vec<Arc<dyn ClientBinding>>,
    security_providers: Vec<Arc<dyn SecurityProvider>>,
    discoverer: Option<Arc<dyn Discoverer>>,
}

impl ServientBuilder {
    pub fn new() -> Self {
        Self {
            server_bindings: Vec::new(),
            #[cfg(feature = "async")]
            client_bindings: Vec::new(),
            security_providers: Vec::new(),
            discoverer: None,
        }
    }

    /// Registers a server binding (inbound). The Servient stores it as a
    /// default; `ExposedThingHandle` clones an `Arc` reference at `produce()`
    /// time. Call once per protocol.
    pub fn with_server_binding(mut self, binding: Arc<dyn ServerBinding>) -> Self {
        self.server_bindings.push(binding);
        self
    }

    /// Registers a client binding (outbound). The Servient stores it as a
    /// default; `ConsumedThingHandle` clones an `Arc` reference at `consume()`
    /// time. Call once per protocol.
    #[cfg(feature = "async")]
    pub fn with_client_binding(mut self, binding: Arc<dyn ClientBinding>) -> Self {
        self.client_bindings.push(binding);
        self
    }

    /// Registers a [`SecurityProvider`] for inbound request verification.
    pub fn with_security_provider(mut self, provider: Arc<dyn SecurityProvider>) -> Self {
        self.security_providers.push(provider);
        self
    }

    pub fn with_discoverer(mut self, discoverer: Arc<dyn Discoverer>) -> Self {
        self.discoverer = Some(discoverer);
        self
    }

    /// Builds the [`Servient`].
    pub fn build(self) -> ServientResult<Servient> {
        let Self {
            server_bindings,
            #[cfg(feature = "async")]
            client_bindings,
            security_providers,
            discoverer,
        } = self;

        let discoverer: Arc<dyn Discoverer> = discoverer
            .unwrap_or_else(|| Arc::new(LocalDiscoverer::new(Arc::new(InMemoryDirectory::new()))));

        let event_broker = EventBroker::new();
        let server_bindings: Arc<[Arc<dyn ServerBinding>]> = Arc::from(server_bindings);
        #[cfg(feature = "async")]
        let client_bindings: Arc<[Arc<dyn ClientBinding>]> = Arc::from(client_bindings);
        let security_providers: Arc<[Arc<dyn SecurityProvider>]> =
            if security_providers.is_empty() {
                Arc::from([Arc::new(clinkz_wot_core::NoSecurityProvider::new())
                    as Arc<dyn SecurityProvider>])
            } else {
                Arc::from(security_providers)
            };

        let servient = Servient::assemble(
            Default::default(),
            Default::default(),
            server_bindings,
            #[cfg(feature = "async")]
            client_bindings,
            security_providers,
            discoverer,
            event_broker,
        );

        let _ = ServientError::AlreadyExposed; // suppress unused import
        Ok(servient)
    }
}

impl Default for ServientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
