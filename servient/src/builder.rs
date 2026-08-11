//! `ServientBuilder` — std-host consuming, move-fluent builder (v4.1 AD55–AD58).

use alloc::{sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ClientBinding, CoreError, CredentialStore, ErrorContext, ErrorPhase, EventBroker,
    HostBindingRegistration, RetryClass, SecurityProvider, ServerBinding,
};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};
use clinkz_wot_foundation::ResourceLimits;

use crate::ServientResult;
use crate::property_read::{HostPropertyReadConfig, HostPropertyReadOwner};
use crate::servient::Servient;

/// Consuming, move-fluent builder for a [`Servient`].
pub struct ServientBuilder {
    server_bindings: Vec<Arc<dyn ServerBinding>>,
    #[cfg(feature = "async")]
    client_bindings: Vec<Arc<dyn ClientBinding>>,
    security_providers: Vec<Arc<dyn SecurityProvider>>,
    credential_store: Option<Arc<dyn CredentialStore>>,
    discoverer: Option<Arc<dyn Discoverer>>,
    resource_limits: Option<ResourceLimits>,
    property_read_binding: Option<HostBindingRegistration>,
}

impl ServientBuilder {
    pub fn new() -> Self {
        Self {
            server_bindings: Vec::new(),
            #[cfg(feature = "async")]
            client_bindings: Vec::new(),
            security_providers: Vec::new(),
            credential_store: None,
            discoverer: None,
            resource_limits: None,
            property_read_binding: None,
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

    /// Registers a [`CredentialStore`] for outbound request-level security
    /// ([`SecurityProvider::apply`]). The store is shared with every
    /// [`ConsumedThing`](clinkz_wot_core::ConsumedThing) produced by this
    /// Servient.
    pub fn with_credential_store(mut self, store: Arc<dyn CredentialStore>) -> Self {
        self.credential_store = Some(store);
        self
    }

    pub fn with_discoverer(mut self, discoverer: Arc<dyn Discoverer>) -> Self {
        self.discoverer = Some(discoverer);
        self
    }

    /// Installs the explicit resource policy used by the narrow manually
    /// progressed Property Read runtime.
    pub fn resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Installs one complete host-erased Producer Property Read registration.
    pub fn binding_registration(mut self, registration: HostBindingRegistration) -> Self {
        self.property_read_binding = Some(registration);
        self
    }

    /// Builds the [`Servient`].
    pub fn build(self) -> ServientResult<Servient> {
        let Self {
            server_bindings,
            #[cfg(feature = "async")]
            client_bindings,
            security_providers,
            credential_store,
            discoverer,
            resource_limits,
            property_read_binding,
        } = self;

        let property_read = match (resource_limits, property_read_binding) {
            (Some(limits), Some(registration)) => Some(HostPropertyReadOwner::new(
                HostPropertyReadConfig::new(limits, registration),
            )),
            (None, Some(_)) => {
                return Err(CoreError::Validation(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Never,
                ))
                .into());
            }
            (_, None) => None,
        };

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
            credential_store,
            discoverer,
            event_broker,
            property_read,
        );

        Ok(servient)
    }
}

impl Default for ServientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
