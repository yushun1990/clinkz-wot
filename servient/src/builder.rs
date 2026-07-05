//! `ServientBuilder` — std-host consuming, move-fluent builder
//! (baseline v4.0 §7.3/§3.11 / phase-p3 §3.11).

use alloc::{sync::Arc, vec::Vec};

use clinkz_wot_core::{EventBroker, InboundRequest, ServerBinding};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};

use crate::ServientResult;
use crate::servient::{ClientBindingFactory, Servient};

/// Default inbound fan-in channel capacity (audit AD6a/O5).
const DEFAULT_FANIN_CAPACITY: usize = 256;

/// Consuming, move-fluent builder for a [`Servient`].
///
/// Required: ≥1 server binding (to serve) or explicit local-only; ≥1 client
/// binding factory (to consume). Discovery defaults to a
/// [`LocalDiscoverer`] over a fresh [`InMemoryDirectory`].
pub struct ServientBuilder {
    server_bindings: Vec<Arc<dyn ServerBinding>>,
    client_factories: Vec<Arc<dyn ClientBindingFactory>>,
    discoverer: Option<Arc<dyn Discoverer>>,
    fanin_capacity: usize,
}

impl ServientBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self {
            server_bindings: Vec::new(),
            client_factories: Vec::new(),
            discoverer: None,
            fanin_capacity: DEFAULT_FANIN_CAPACITY,
        }
    }

    /// Adds a server binding (≥1 required to serve).
    pub fn with_server_binding(mut self, binding: Arc<dyn ServerBinding>) -> Self {
        self.server_bindings.push(binding);
        self
    }

    /// Adds a client binding factory (≥1 required to consume).
    pub fn with_client_factory(mut self, factory: Arc<dyn ClientBindingFactory>) -> Self {
        self.client_factories.push(factory);
        self
    }

    /// Sets the Discoverer. If omitted, `build()` installs a
    /// [`LocalDiscoverer`] over a fresh [`InMemoryDirectory`] (embedded/local-only).
    pub fn with_discoverer(mut self, discoverer: Arc<dyn Discoverer>) -> Self {
        self.discoverer = Some(discoverer);
        self
    }

    /// Sets the bounded inbound fan-in channel capacity (AD6a/O5).
    pub fn with_fanin_capacity(mut self, capacity: usize) -> Self {
        self.fanin_capacity = capacity.max(1);
        self
    }

    /// Builds the [`Servient`]: constructs the fan-in channel, injects the
    /// sender + [`EventBroker`] into every server binding, and assembles the
    /// registries.
    pub fn build(self) -> ServientResult<Servient> {
        let Self {
            server_bindings,
            client_factories,
            discoverer,
            fanin_capacity,
        } = self;

        let discoverer: Arc<dyn Discoverer> = discoverer.unwrap_or_else(|| {
            let dir = Arc::new(InMemoryDirectory::new());
            Arc::new(LocalDiscoverer::new(dir))
        });

        let event_broker = EventBroker::new();
        let (inbound_tx, inbound_rx) = async_channel::bounded::<InboundRequest>(fanin_capacity);

        let server_bindings: Arc<[Arc<dyn ServerBinding>]> = Arc::from(server_bindings);
        let client_factories: Arc<[Arc<dyn ClientBindingFactory>]> = Arc::from(client_factories);

        // Assemble the Servient FIRST (needs owned event_broker + server_bindings).
        // Clone what we still need for post-construction wiring.
        let broker_for_wiring = event_broker.clone();
        let tx_for_wiring = inbound_tx.clone();
        let bindings_for_wiring = server_bindings.clone();

        let servient = Servient::assemble(
            Default::default(),
            Default::default(),
            server_bindings,
            client_factories,
            discoverer,
            event_broker,
            inbound_tx,
            Arc::new(inbound_rx),
        );

        // THEN wire bindings — one configure call per binding with a context
        // containing all available capabilities. Each binding picks what it needs.
        let dispatch: Arc<dyn clinkz_wot_core::Dispatch> = Arc::new(servient.clone());
        let ctx = clinkz_wot_core::BindingContext {
            event_broker: broker_for_wiring,
            fanin_sender: Some(tx_for_wiring),
            dispatch: Some(dispatch),
        };
        for binding in bindings_for_wiring.iter() {
            binding.configure(&ctx);
        }

        Ok(servient)
    }
}

impl Default for ServientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
