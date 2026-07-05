//! `ServientBuilder` — std-host consuming, move-fluent builder.

use alloc::{sync::Arc, vec::Vec};

use clinkz_wot_core::{EventBroker, ServerBinding};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};

use crate::servient::{ClientBindingFactory, Servient};
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

    pub fn with_server_binding(mut self, binding: Arc<dyn ServerBinding>) -> Self {
        self.server_bindings.push(binding);
        self
    }

    pub fn with_client_factory(mut self, factory: Arc<dyn ClientBindingFactory>) -> Self {
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

impl Default for ServientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
