use clinkz_wot_core::{LocalThing, PayloadCodec, ProtocolBinding, SecurityProvider};
use clinkz_wot_discovery::{
    DirectoryEntry, DirectoryPage, DirectoryQuery, InMemoryThingDirectory, ThingDirectory,
};
use clinkz_wot_td::thing::Thing;

use crate::{
    BindingPlanCache, ConsumedThingCache, ExposedThingRegistry, InMemoryBindingPlanCache,
    InMemoryConsumedThingCache, InMemoryExposedThingRegistry, InMemorySelectedFormCache,
    SelectedFormCache, ServientBuilder, ServientError, ServientResult,
};

pub(crate) type BindingFactory = Box<dyn Fn() -> Box<dyn ProtocolBinding>>;

/// Host Servient that composes discovery, exposed Things, and consumed Things.
pub struct Servient<
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
    pub(crate) running: bool,
}

impl
    Servient<
        InMemoryThingDirectory,
        InMemoryExposedThingRegistry,
        InMemoryConsumedThingCache,
        InMemorySelectedFormCache,
        InMemoryBindingPlanCache,
    >
{
    /// Creates a Servient backed by an in-memory Thing Description Directory.
    pub fn builder() -> ServientBuilder<
        InMemoryThingDirectory,
        InMemoryExposedThingRegistry,
        InMemoryConsumedThingCache,
        InMemorySelectedFormCache,
        InMemoryBindingPlanCache,
    > {
        ServientBuilder::new()
    }

    /// Creates a default in-memory Servient.
    pub fn new() -> Self {
        Self::builder().build()
    }
}

impl Default
    for Servient<
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

impl<D, R, C, S, P> Servient<D, R, C, S, P>
where
    D: ThingDirectory,
    R: ExposedThingRegistry,
    C: ConsumedThingCache,
    S: SelectedFormCache,
    P: BindingPlanCache,
{
    /// Starts the host runtime lifecycle.
    ///
    /// Starting is idempotent. Runtime composition changes must be made before
    /// start or after stop.
    pub fn start(&mut self) -> ServientResult<()> {
        self.running = true;
        Ok(())
    }

    /// Stops the host runtime lifecycle.
    ///
    /// Stopping is idempotent. Directory and exposure mutations are allowed
    /// after the runtime has stopped.
    pub fn stop(&mut self) -> ServientResult<()> {
        self.running = false;
        Ok(())
    }

    /// Returns whether the Servient lifecycle is currently started.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Returns the underlying Thing Description Directory.
    pub fn directory(&self) -> &D {
        &self.directory
    }

    /// Returns the underlying Thing Description Directory mutably.
    pub fn directory_mut(&mut self) -> &mut D {
        &mut self.directory
    }

    /// Returns the underlying exposed Thing registry.
    pub fn exposed_registry(&self) -> &R {
        &self.exposed_registry
    }

    /// Returns the underlying exposed Thing registry mutably.
    pub fn exposed_registry_mut(&mut self) -> &mut R {
        &mut self.exposed_registry
    }

    /// Returns the underlying consumed Thing cache.
    pub fn consumed_cache(&self) -> &C {
        &self.consumed_cache
    }

    /// Returns the underlying consumed Thing cache mutably.
    pub fn consumed_cache_mut(&mut self) -> &mut C {
        &mut self.consumed_cache
    }

    /// Returns the underlying selected form cache.
    pub fn selected_form_cache(&self) -> &S {
        &self.selected_form_cache
    }

    /// Returns the underlying selected form cache mutably.
    pub fn selected_form_cache_mut(&mut self) -> &mut S {
        &mut self.selected_form_cache
    }

    /// Returns the underlying binding plan cache.
    pub fn binding_plan_cache(&self) -> &P {
        &self.binding_plan_cache
    }

    /// Returns the underlying binding plan cache mutably.
    pub fn binding_plan_cache_mut(&mut self) -> &mut P {
        &mut self.binding_plan_cache
    }

    /// Returns registered payload codecs.
    pub fn payload_codecs(&self) -> &[Box<dyn PayloadCodec>] {
        &self.payload_codecs
    }

    /// Returns registered security providers.
    pub fn security_providers(&self) -> &[Box<dyn SecurityProvider>] {
        &self.security_providers
    }

    /// Registers a protocol binding factory after the Servient has been built.
    pub fn register_binding_factory<F>(&mut self, factory: F) -> ServientResult<()>
    where
        F: Fn() -> Box<dyn ProtocolBinding> + 'static,
    {
        self.ensure_stopped()?;
        self.binding_factories.push(Box::new(factory));
        Ok(())
    }

    /// Registers a payload codec after the Servient has been built.
    pub fn register_payload_codec(
        &mut self,
        codec: impl PayloadCodec + 'static,
    ) -> ServientResult<()> {
        self.ensure_stopped()?;
        self.payload_codecs.push(Box::new(codec));
        Ok(())
    }

    /// Registers a security provider after the Servient has been built.
    pub fn register_security_provider(
        &mut self,
        provider: impl SecurityProvider + 'static,
    ) -> ServientResult<()> {
        self.ensure_stopped()?;
        self.security_providers.push(Box::new(provider));
        Ok(())
    }

    /// Registers a TD in the directory without exposing local handlers.
    pub fn register(&mut self, thing: Thing) -> ServientResult<DirectoryEntry> {
        self.ensure_stopped()?;
        let entry = self.directory.register(thing)?;
        self.consumed_cache
            .insert(entry.id.clone(), entry.thing.clone());
        self.selected_form_cache.remove_thing(&entry.id);
        self.binding_plan_cache.remove_thing(&entry.id);
        Ok(entry)
    }

    /// Updates a TD in the directory.
    pub fn update(&mut self, thing: Thing) -> ServientResult<DirectoryEntry> {
        self.ensure_stopped()?;
        let entry = self.directory.update(thing)?;
        self.consumed_cache
            .insert(entry.id.clone(), entry.thing.clone());
        self.selected_form_cache.remove_thing(&entry.id);
        self.binding_plan_cache.remove_thing(&entry.id);
        Ok(entry)
    }

    /// Removes a TD from the directory.
    pub fn unregister(&mut self, id: &str) -> ServientResult<Thing> {
        self.ensure_stopped()?;
        let thing = self.directory.delete(id)?;
        self.exposed_registry.remove(id);
        self.consumed_cache.remove(id);
        self.selected_form_cache.remove_thing(id);
        self.binding_plan_cache.remove_thing(id);
        Ok(thing)
    }

    /// Lists directory entries in deterministic backend order.
    pub fn list(&self) -> DirectoryPage {
        self.directory.list()
    }

    /// Queries directory entries with the shared Discovery query model.
    pub fn query(&self, query: DirectoryQuery) -> DirectoryPage {
        self.directory.query(query)
    }

    /// Exposes a local Thing and registers its TD in the directory.
    pub fn expose(&mut self, thing: LocalThing) -> ServientResult<DirectoryEntry> {
        self.ensure_stopped()?;
        let id = thing_id(thing.thing_description())?;
        if self.exposed_registry.contains_id(&id) {
            return Err(ServientError::DuplicateExposedThing(id));
        }

        let entry = self.directory.register(thing.thing_description().clone())?;
        self.consumed_cache
            .insert(entry.id.clone(), entry.thing.clone());
        self.selected_form_cache.remove_thing(&entry.id);
        self.binding_plan_cache.remove_thing(&entry.id);
        self.exposed_registry.insert(id, thing);
        Ok(entry)
    }

    /// Removes a locally exposed Thing and its directory entry.
    pub fn unexpose(&mut self, id: &str) -> ServientResult<LocalThing> {
        self.ensure_stopped()?;
        let thing = self
            .exposed_registry
            .remove(id)
            .ok_or_else(|| ServientError::ExposedThingNotFound(id.to_owned()))?;
        self.consumed_cache.remove(id);
        self.selected_form_cache.remove_thing(id);
        self.binding_plan_cache.remove_thing(id);
        self.directory.delete(id)?;
        Ok(thing)
    }

    /// Returns a mutable local exposed Thing dispatcher.
    pub fn exposed_thing_mut(&mut self, id: &str) -> ServientResult<&mut LocalThing> {
        self.exposed_registry
            .get_mut(id)
            .ok_or_else(|| ServientError::ExposedThingNotFound(id.to_owned()))
    }

    fn ensure_stopped(&self) -> ServientResult<()> {
        if self.running {
            Err(ServientError::Running)
        } else {
            Ok(())
        }
    }
}

fn thing_id(thing: &Thing) -> ServientResult<String> {
    thing
        .id
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .ok_or(ServientError::MissingThingId)
}
