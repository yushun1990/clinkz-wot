use alloc::{boxed::Box, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ClientBinding, CredentialStore, EventBroker, MapLock, PayloadCodec, SecurityProvider,
    ServerBinding,
};
use clinkz_wot_discovery::{InMemoryThingDirectory, ThingDirectory};

use crate::{
    ConsumedThingRegistry, ExposedThingRegistry,
    servient::{
        BindingFactoryEntry, BindingFactoryRegistry, Servient, ServientShared, ServientState,
    },
};

/// Builder for a Web of Things Servient.
///
/// Mirrors the [`Servient<D>`](Servient) shape: a single generic directory
/// parameter `D` (baseline §6). The exposed Thing registry and form/binding
/// caches are internal concrete types and are no longer injectable.
pub struct ServientBuilder<D = InMemoryThingDirectory> {
    pub(crate) directory: D,
    pub(crate) binding_factories: Vec<BindingFactoryEntry>,
    pub(crate) payload_codecs: Vec<Arc<dyn PayloadCodec>>,
    pub(crate) security_providers: Vec<Arc<dyn SecurityProvider>>,
    pub(crate) credential_store: Option<Arc<dyn CredentialStore>>,
    pub(crate) server_bindings: Vec<Arc<dyn ServerBinding>>,
    pub(crate) normalize_payloads: bool,
    #[cfg(feature = "async")]
    pub(crate) async_server_bindings: Vec<Arc<dyn clinkz_wot_core::AsyncServerBinding>>,
}

impl ServientBuilder<InMemoryThingDirectory> {
    /// Creates a builder using an in-memory Thing Description Directory.
    pub fn new() -> Self {
        Self {
            directory: InMemoryThingDirectory::new(),
            binding_factories: Vec::new(),
            payload_codecs: Vec::new(),
            security_providers: Vec::new(),
            credential_store: None,
            server_bindings: Vec::new(),
            normalize_payloads: true,
            #[cfg(feature = "async")]
            async_server_bindings: Vec::new(),
        }
    }
}

impl Default for ServientBuilder<InMemoryThingDirectory> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> ServientBuilder<D>
where
    D: ThingDirectory,
{
    /// Uses a caller-provided Thing Description Directory backend.
    pub fn with_directory<N>(self, directory: N) -> ServientBuilder<N>
    where
        N: ThingDirectory,
    {
        ServientBuilder {
            directory,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
            credential_store: self.credential_store,
            server_bindings: self.server_bindings,
            normalize_payloads: self.normalize_payloads,
            #[cfg(feature = "async")]
            async_server_bindings: self.async_server_bindings,
        }
    }

    /// Registers a factory used to attach protocol bindings to consumed Things.
    pub fn binding_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> Box<dyn ClientBinding + Send + Sync> + 'static,
    {
        self.binding_factories.push(BindingFactoryEntry {
            make: Box::new(factory),
            supports: Arc::new(|_, _, _| true),
        });
        self
    }

    /// Registers a factory used to attach protocol bindings to consumed Things.
    ///
    /// The `supports` predicate lets the Servient skip instantiating bindings
    /// that cannot handle the selected form/operation.
    pub fn binding_factory_with_support<F, S>(mut self, factory: F, supports: S) -> Self
    where
        F: Fn() -> Box<dyn ClientBinding + Send + Sync> + 'static,
        S: Fn(
                &clinkz_wot_td::thing::Thing,
                &clinkz_wot_td::form::Form,
                clinkz_wot_td::data_type::Operation,
            ) -> bool
            + 'static,
    {
        self.binding_factories.push(BindingFactoryEntry {
            make: Box::new(factory),
            supports: Arc::new(supports),
        });
        self
    }

    /// Registers a payload codec used by Servient interaction hooks.
    pub fn payload_codec(mut self, codec: impl PayloadCodec + 'static) -> Self {
        self.payload_codecs.push(Arc::new(codec));
        self
    }

    /// Registers a security provider used by Servient interaction hooks.
    pub fn security_provider(mut self, provider: impl SecurityProvider + 'static) -> Self {
        self.security_providers.push(Arc::new(provider));
        self
    }

    /// Registers a credential store for security providers to retrieve stored
    /// secrets (baseline addendum §1.2 `cz:credentialSource`).
    pub fn credential_store(mut self, store: Arc<dyn CredentialStore>) -> Self {
        self.credential_store = Some(store);
        self
    }

    /// Controls whether consumed-interaction payloads are decoded and
    /// re-encoded through registered codecs for canonicalization (default
    /// `true`).
    ///
    /// Set to `false` for high-frequency deployments that do not need canonical
    /// bytes (e.g. no signing/hashing) — this skips two `Vec<u8>` allocations
    /// and a full decode+encode round-trip per interaction whose content type
    /// matches a registered codec. Malformed payloads are no longer rejected by
    /// the codec in this mode; validation responsibility moves to the handler.
    pub fn normalize_payloads(mut self, enabled: bool) -> Self {
        self.normalize_payloads = enabled;
        self
    }

    /// Registers a server binding for inbound interactions.
    pub fn server_binding(mut self, binding: Arc<dyn ServerBinding>) -> Self {
        self.server_bindings.push(binding);
        self
    }

    /// Registers an async server binding for native-async inbound driving.
    #[cfg(feature = "async")]
    pub fn async_server_binding(
        mut self,
        binding: Arc<dyn clinkz_wot_core::AsyncServerBinding>,
    ) -> Self {
        self.async_server_bindings.push(binding);
        self
    }

    /// Builds the Servient.
    pub fn build(self) -> Servient<D> {
        let event_broker = EventBroker::new();

        // Hand the broker to every server binding so they can register
        // PublisherSinks during subsequent `expose` calls.
        for binding in &self.server_bindings {
            binding.set_event_broker(event_broker.clone());
        }
        #[cfg(feature = "async")]
        for binding in &self.async_server_bindings {
            binding.set_event_broker(event_broker.clone());
        }
        #[cfg(feature = "async")]
        let async_binding_generation = self.async_server_bindings.len() as u64;

        Servient::from_parts(
            ServientShared {
                #[allow(clippy::arc_with_non_send_sync)]
                exposed_registry: Arc::new(ExposedThingRegistry::new()),
                #[allow(clippy::arc_with_non_send_sync)]
                consumed_registry: Arc::new(ConsumedThingRegistry::new()),
                binding_factories: BindingFactoryRegistry::from_factories(self.binding_factories),
                #[allow(clippy::arc_with_non_send_sync)]
                payload_codecs: Arc::new(MapLock::new(self.payload_codecs)),
                #[allow(clippy::arc_with_non_send_sync)]
                security_providers: Arc::new(MapLock::new(Arc::new(self.security_providers))),
                credential_store: self.credential_store,
                event_broker,
                normalize_payloads: self.normalize_payloads,
                #[allow(clippy::arc_with_non_send_sync)]
                sync_server_bindings: Arc::new(MapLock::new(Arc::from(
                    self.server_bindings.clone(),
                ))),
                #[cfg(feature = "async")]
                #[allow(clippy::arc_with_non_send_sync)]
                async_server_bindings: Arc::new(MapLock::new(Arc::from(
                    self.async_server_bindings.clone(),
                ))),
            },
            ServientState {
                directory: self.directory,
                sync_binding_cursor: 0,
                #[cfg(feature = "async")]
                async_binding_generation,
                #[cfg(feature = "async")]
                async_accept_state: crate::servient::AsyncAcceptState::new(),
            },
        )
    }
}
