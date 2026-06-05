//! Host runtime composition for Web of Things Servient flows.
//!
//! This crate wires protocol-neutral core dispatch, Discovery directory
//! storage, and protocol binding factories without making any concrete
//! protocol binding mandatory.

use std::collections::BTreeMap;
use std::fmt;

use clinkz_wot_core::{
    AffordanceTarget, BoundConsumedThing, ConsumedThing, CoreError, EventSink, ExposedThing,
    InteractionInput, InteractionOutput, LocalThing, ProtocolBinding,
};
use clinkz_wot_discovery::{
    DirectoryEntry, DirectoryPage, DirectoryQuery, DiscoveryError, InMemoryThingDirectory,
    ThingDirectory,
};
use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

/// Result type used by Servient runtime composition APIs.
pub type ServientResult<T> = Result<T, ServientError>;

/// Errors produced while composing local Things, consumed Things, bindings,
/// and discovery backends.
#[derive(Debug)]
pub enum ServientError {
    /// Discovery or directory storage failed.
    Discovery(DiscoveryError),
    /// Core dispatch or binding interaction failed.
    Core(CoreError),
    /// A local exposed Thing is already registered with this id.
    DuplicateExposedThing(String),
    /// No local exposed Thing is registered with this id.
    ExposedThingNotFound(String),
    /// A local Thing cannot be exposed without a stable TD id.
    MissingThingId,
}

impl fmt::Display for ServientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(err) => write!(f, "Discovery error: {}", err),
            Self::Core(err) => write!(f, "Core error: {}", err),
            Self::DuplicateExposedThing(id) => {
                write!(f, "Servient already exposes Thing id '{}'", id)
            }
            Self::ExposedThingNotFound(id) => {
                write!(f, "Servient does not expose Thing id '{}'", id)
            }
            Self::MissingThingId => write!(f, "Thing Description is missing required id"),
        }
    }
}

impl std::error::Error for ServientError {}

impl From<DiscoveryError> for ServientError {
    fn from(value: DiscoveryError) -> Self {
        Self::Discovery(value)
    }
}

impl From<CoreError> for ServientError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

type BindingFactory = Box<dyn Fn() -> Box<dyn ProtocolBinding>>;

/// Builder for a host Servient.
pub struct ServientBuilder<D = InMemoryThingDirectory> {
    directory: D,
    binding_factories: Vec<BindingFactory>,
}

impl ServientBuilder<InMemoryThingDirectory> {
    /// Creates a builder using an in-memory Thing Description Directory.
    pub fn new() -> Self {
        Self {
            directory: InMemoryThingDirectory::new(),
            binding_factories: Vec::new(),
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

    /// Builds the Servient.
    pub fn build(self) -> Servient<D> {
        Servient {
            directory: self.directory,
            exposed_things: BTreeMap::new(),
            binding_factories: self.binding_factories,
            running: false,
        }
    }
}

/// Host Servient that composes discovery, exposed Things, and consumed Things.
pub struct Servient<D = InMemoryThingDirectory> {
    directory: D,
    exposed_things: BTreeMap<String, LocalThing>,
    binding_factories: Vec<BindingFactory>,
    running: bool,
}

impl Servient<InMemoryThingDirectory> {
    /// Creates a Servient backed by an in-memory Thing Description Directory.
    pub fn builder() -> ServientBuilder<InMemoryThingDirectory> {
        ServientBuilder::new()
    }

    /// Creates a default in-memory Servient.
    pub fn new() -> Self {
        Self::builder().build()
    }
}

impl Default for Servient<InMemoryThingDirectory> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> Servient<D>
where
    D: ThingDirectory,
{
    /// Starts the host runtime lifecycle.
    pub fn start(&mut self) -> ServientResult<()> {
        self.running = true;
        Ok(())
    }

    /// Stops the host runtime lifecycle.
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

    /// Registers a protocol binding factory after the Servient has been built.
    pub fn register_binding_factory<F>(&mut self, factory: F)
    where
        F: Fn() -> Box<dyn ProtocolBinding> + 'static,
    {
        self.binding_factories.push(Box::new(factory));
    }

    /// Registers a TD in the directory without exposing local handlers.
    pub fn register(&mut self, thing: Thing) -> ServientResult<DirectoryEntry> {
        self.directory.register(thing).map_err(Into::into)
    }

    /// Updates a TD in the directory.
    pub fn update(&mut self, thing: Thing) -> ServientResult<DirectoryEntry> {
        self.directory.update(thing).map_err(Into::into)
    }

    /// Removes a TD from the directory.
    pub fn unregister(&mut self, id: &str) -> ServientResult<Thing> {
        self.exposed_things.remove(id);
        self.directory.delete(id).map_err(Into::into)
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
        let id = thing_id(thing.thing_description())?;
        if self.exposed_things.contains_key(&id) {
            return Err(ServientError::DuplicateExposedThing(id));
        }

        let entry = self.directory.register(thing.thing_description().clone())?;
        self.exposed_things.insert(id, thing);
        Ok(entry)
    }

    /// Removes a locally exposed Thing and its directory entry.
    pub fn unexpose(&mut self, id: &str) -> ServientResult<LocalThing> {
        let thing = self
            .exposed_things
            .remove(id)
            .ok_or_else(|| ServientError::ExposedThingNotFound(id.to_owned()))?;
        self.directory.delete(id)?;
        Ok(thing)
    }

    /// Returns a mutable local exposed Thing dispatcher.
    pub fn exposed_thing_mut(&mut self, id: &str) -> ServientResult<&mut LocalThing> {
        self.exposed_things
            .get_mut(id)
            .ok_or_else(|| ServientError::ExposedThingNotFound(id.to_owned()))
    }

    /// Reads a property on a locally exposed Thing.
    pub fn read_property(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.exposed_thing_mut(id)?
            .read_property(name, input)
            .map_err(Into::into)
    }

    /// Writes a property on a locally exposed Thing.
    pub fn write_property(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.exposed_thing_mut(id)?
            .write_property(name, input)
            .map_err(Into::into)
    }

    /// Invokes an action on a locally exposed Thing.
    pub fn invoke_action(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.exposed_thing_mut(id)?
            .invoke_action(name, input)
            .map_err(Into::into)
    }

    /// Subscribes to an event on a locally exposed Thing.
    pub fn subscribe_event(
        &mut self,
        id: &str,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> ServientResult<InteractionOutput> {
        self.exposed_thing_mut(id)?
            .subscribe_event(name, input, sink)
            .map_err(Into::into)
    }

    /// Creates a consumed Thing dispatcher from a directory entry.
    pub fn consume(&self, id: &str) -> ServientResult<BoundConsumedThing> {
        let thing = self.directory.get(id)?;
        Ok(self.bound_consumed_thing(thing))
    }

    /// Creates a consumed Thing dispatcher directly from a TD.
    pub fn consume_thing(&self, thing: Thing) -> BoundConsumedThing {
        self.bound_consumed_thing(thing)
    }

    /// Reads a property on a remote Thing through a caller-selected form.
    pub fn read_remote_property(
        &self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.consume(id)?
            .request(
                AffordanceTarget::Property(name),
                Operation::ReadProperty,
                form,
                input,
            )
            .map_err(Into::into)
    }

    /// Writes a property on a remote Thing through a caller-selected form.
    pub fn write_remote_property(
        &self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.consume(id)?
            .request(
                AffordanceTarget::Property(name),
                Operation::WriteProperty,
                form,
                input,
            )
            .map_err(Into::into)
    }

    /// Invokes an action on a remote Thing through a caller-selected form.
    pub fn invoke_remote_action(
        &self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.consume(id)?
            .request(
                AffordanceTarget::Action(name),
                Operation::InvokeAction,
                form,
                input,
            )
            .map_err(Into::into)
    }

    /// Subscribes to a remote event through a caller-selected form.
    pub fn subscribe_remote_event(
        &self,
        id: &str,
        name: &str,
        form: &Form,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.consume(id)?
            .request(
                AffordanceTarget::Event(name),
                Operation::SubscribeEvent,
                form,
                input,
            )
            .map_err(Into::into)
    }

    fn bound_consumed_thing(&self, thing: Thing) -> BoundConsumedThing {
        let mut consumed = BoundConsumedThing::new(thing);
        for factory in &self.binding_factories {
            consumed.register_binding(factory());
        }
        consumed
    }
}

fn thing_id(thing: &Thing) -> ServientResult<String> {
    thing
        .id
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .ok_or(ServientError::MissingThingId)
}
