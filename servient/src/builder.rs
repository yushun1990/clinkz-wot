use clinkz_wot_core::{PayloadCodec, ProtocolBinding, SecurityProvider};
use clinkz_wot_discovery::{InMemoryThingDirectory, ThingDirectory};

use crate::{
    BindingPlanCache, ConsumedThingCache, ExposedThingRegistry, InMemoryBindingPlanCache,
    InMemoryConsumedThingCache, InMemoryExposedThingRegistry, InMemorySelectedFormCache,
    SelectedFormCache,
    runtime::{BindingFactory, Servient},
};

/// Builder for a host Servient.
pub struct ServientBuilder<
    D = InMemoryThingDirectory,
    R = InMemoryExposedThingRegistry,
    C = InMemoryConsumedThingCache,
    S = InMemorySelectedFormCache,
    P = InMemoryBindingPlanCache,
> {
    pub(crate) directory: D,
    pub(crate) exposed_registry: R,
    pub(crate) consumed_cache: C,
    pub(crate) selected_form_cache: S,
    pub(crate) binding_plan_cache: P,
    pub(crate) binding_factories: Vec<BindingFactory>,
    pub(crate) payload_codecs: Vec<Box<dyn PayloadCodec>>,
    pub(crate) security_providers: Vec<Box<dyn SecurityProvider>>,
}

impl
    ServientBuilder<
        InMemoryThingDirectory,
        InMemoryExposedThingRegistry,
        InMemoryConsumedThingCache,
        InMemorySelectedFormCache,
        InMemoryBindingPlanCache,
    >
{
    /// Creates a builder using an in-memory Thing Description Directory.
    pub fn new() -> Self {
        Self {
            directory: InMemoryThingDirectory::new(),
            exposed_registry: InMemoryExposedThingRegistry::new(),
            consumed_cache: InMemoryConsumedThingCache::new(),
            selected_form_cache: InMemorySelectedFormCache::new(),
            binding_plan_cache: InMemoryBindingPlanCache::new(),
            binding_factories: Vec::new(),
            payload_codecs: Vec::new(),
            security_providers: Vec::new(),
        }
    }
}

impl Default
    for ServientBuilder<
        InMemoryThingDirectory,
        InMemoryExposedThingRegistry,
        InMemoryConsumedThingCache,
        InMemorySelectedFormCache,
        InMemoryBindingPlanCache,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl<D, R, C, S, P> ServientBuilder<D, R, C, S, P>
where
    D: ThingDirectory,
    R: ExposedThingRegistry,
    C: ConsumedThingCache,
    S: SelectedFormCache,
    P: BindingPlanCache,
{
    /// Uses a caller-provided Thing Description Directory backend.
    pub fn with_directory<N>(self, directory: N) -> ServientBuilder<N, R, C, S, P>
    where
        N: ThingDirectory,
    {
        ServientBuilder {
            directory,
            exposed_registry: self.exposed_registry,
            consumed_cache: self.consumed_cache,
            selected_form_cache: self.selected_form_cache,
            binding_plan_cache: self.binding_plan_cache,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
        }
    }

    /// Uses a caller-provided exposed Thing registry backend.
    pub fn with_exposed_registry<N>(self, exposed_registry: N) -> ServientBuilder<D, N, C, S, P>
    where
        N: ExposedThingRegistry,
    {
        ServientBuilder {
            directory: self.directory,
            exposed_registry,
            consumed_cache: self.consumed_cache,
            selected_form_cache: self.selected_form_cache,
            binding_plan_cache: self.binding_plan_cache,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
        }
    }

    /// Uses a caller-provided consumed Thing cache backend.
    pub fn with_consumed_cache<N>(self, consumed_cache: N) -> ServientBuilder<D, R, N, S, P>
    where
        N: ConsumedThingCache,
    {
        ServientBuilder {
            directory: self.directory,
            exposed_registry: self.exposed_registry,
            consumed_cache,
            selected_form_cache: self.selected_form_cache,
            binding_plan_cache: self.binding_plan_cache,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
        }
    }

    /// Uses a caller-provided selected form cache backend.
    pub fn with_selected_form_cache<N>(
        self,
        selected_form_cache: N,
    ) -> ServientBuilder<D, R, C, N, P>
    where
        N: SelectedFormCache,
    {
        ServientBuilder {
            directory: self.directory,
            exposed_registry: self.exposed_registry,
            consumed_cache: self.consumed_cache,
            selected_form_cache,
            binding_plan_cache: self.binding_plan_cache,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
        }
    }

    /// Uses a caller-provided binding plan cache backend.
    pub fn with_binding_plan_cache<N>(self, binding_plan_cache: N) -> ServientBuilder<D, R, C, S, N>
    where
        N: BindingPlanCache,
    {
        ServientBuilder {
            directory: self.directory,
            exposed_registry: self.exposed_registry,
            consumed_cache: self.consumed_cache,
            selected_form_cache: self.selected_form_cache,
            binding_plan_cache,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
        }
    }

    /// Registers a factory used to attach protocol bindings to consumed Things.
    pub fn binding_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> Box<dyn ProtocolBinding> + 'static,
    {
        self.binding_factories.push(Box::new(factory));
        self
    }

    /// Registers a payload codec used by Servient interaction hooks.
    pub fn payload_codec(mut self, codec: impl PayloadCodec + 'static) -> Self {
        self.payload_codecs.push(Box::new(codec));
        self
    }

    /// Registers a security provider used by Servient interaction hooks.
    pub fn security_provider(mut self, provider: impl SecurityProvider + 'static) -> Self {
        self.security_providers.push(Box::new(provider));
        self
    }

    /// Builds the Servient.
    pub fn build(self) -> Servient<D, R, C, S, P> {
        Servient {
            directory: self.directory,
            exposed_registry: self.exposed_registry,
            consumed_cache: self.consumed_cache,
            selected_form_cache: self.selected_form_cache,
            binding_plan_cache: self.binding_plan_cache,
            binding_factories: self.binding_factories,
            payload_codecs: self.payload_codecs,
            security_providers: self.security_providers,
            running: false,
        }
    }
}
