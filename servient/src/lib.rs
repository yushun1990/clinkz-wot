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
use clinkz_wot_protocol_bindings::{
    AffordanceRef, BindingCoreError, FormSelectionCriteria, select_affordance_form_with_criteria,
    validate_affordance_form_with_criteria,
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
    /// Shared protocol binding form selection or target resolution failed.
    Binding(BindingCoreError),
    /// Core dispatch or binding interaction failed.
    Core(CoreError),
    /// A local exposed Thing is already registered with this id.
    DuplicateExposedThing(String),
    /// No local exposed Thing is registered with this id.
    ExposedThingNotFound(String),
    /// Runtime composition cannot be mutated while the Servient is running.
    Running,
    /// A local Thing cannot be exposed without a stable TD id.
    MissingThingId,
}

impl fmt::Display for ServientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(err) => write!(f, "Discovery error: {}", err),
            Self::Binding(err) => write!(f, "Binding selection error: {}", err),
            Self::Core(err) => write!(f, "Core error: {}", err),
            Self::DuplicateExposedThing(id) => {
                write!(f, "Servient already exposes Thing id '{}'", id)
            }
            Self::ExposedThingNotFound(id) => {
                write!(f, "Servient does not expose Thing id '{}'", id)
            }
            Self::Running => write!(
                f,
                "Servient runtime composition cannot be changed while running"
            ),
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

impl From<BindingCoreError> for ServientError {
    fn from(value: BindingCoreError) -> Self {
        Self::Binding(value)
    }
}

impl From<CoreError> for ServientError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

type BindingFactory = Box<dyn Fn() -> Box<dyn ProtocolBinding>>;

/// Registry boundary for locally exposed Thing dispatchers.
pub trait ExposedThingRegistry {
    /// Returns true when the registry contains the given Thing id.
    fn contains_id(&self, id: &str) -> bool;

    /// Inserts a local Thing dispatcher by Thing id.
    fn insert(&mut self, id: String, thing: LocalThing) -> Option<LocalThing>;

    /// Removes a local Thing dispatcher by Thing id.
    fn remove(&mut self, id: &str) -> Option<LocalThing>;

    /// Returns a mutable local Thing dispatcher by Thing id.
    fn get_mut(&mut self, id: &str) -> Option<&mut LocalThing>;
}

/// Deterministic in-memory registry for locally exposed Things.
pub struct InMemoryExposedThingRegistry {
    things: BTreeMap<String, LocalThing>,
}

impl InMemoryExposedThingRegistry {
    /// Creates an empty exposed Thing registry.
    pub fn new() -> Self {
        Self {
            things: BTreeMap::new(),
        }
    }

    /// Returns the number of exposed Things in the registry.
    pub fn len(&self) -> usize {
        self.things.len()
    }

    /// Returns true when the registry contains no exposed Things.
    pub fn is_empty(&self) -> bool {
        self.things.is_empty()
    }
}

impl Default for InMemoryExposedThingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExposedThingRegistry for InMemoryExposedThingRegistry {
    fn contains_id(&self, id: &str) -> bool {
        self.things.contains_key(id)
    }

    fn insert(&mut self, id: String, thing: LocalThing) -> Option<LocalThing> {
        self.things.insert(id, thing)
    }

    fn remove(&mut self, id: &str) -> Option<LocalThing> {
        self.things.remove(id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut LocalThing> {
        self.things.get_mut(id)
    }
}

/// Cache boundary for consumed Thing TDs used by Servient-level invocation APIs.
pub trait ConsumedThingCache {
    /// Retrieves a cached Thing Description by Thing id.
    fn get(&self, id: &str) -> Option<Thing>;

    /// Inserts or replaces a cached Thing Description by Thing id.
    fn insert(&mut self, id: String, thing: Thing) -> Option<Thing>;

    /// Removes a cached Thing Description by Thing id.
    fn remove(&mut self, id: &str) -> Option<Thing>;
}

/// Deterministic in-memory cache for consumed Thing TDs.
pub struct InMemoryConsumedThingCache {
    things: BTreeMap<String, Thing>,
}

impl InMemoryConsumedThingCache {
    /// Creates an empty consumed Thing cache.
    pub fn new() -> Self {
        Self {
            things: BTreeMap::new(),
        }
    }

    /// Returns the number of cached Thing Descriptions.
    pub fn len(&self) -> usize {
        self.things.len()
    }

    /// Returns true when the cache contains no Thing Descriptions.
    pub fn is_empty(&self) -> bool {
        self.things.is_empty()
    }
}

impl Default for InMemoryConsumedThingCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsumedThingCache for InMemoryConsumedThingCache {
    fn get(&self, id: &str) -> Option<Thing> {
        self.things.get(id).cloned()
    }

    fn insert(&mut self, id: String, thing: Thing) -> Option<Thing> {
        self.things.insert(id, thing)
    }

    fn remove(&mut self, id: &str) -> Option<Thing> {
        self.things.remove(id)
    }
}

/// Owned affordance location used by selected form cache keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedFormCacheAffordance {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(String),
    /// An action affordance by name.
    Action(String),
    /// An event affordance by name.
    Event(String),
}

impl SelectedFormCacheAffordance {
    fn from_affordance_ref(affordance: AffordanceRef<'_>) -> Self {
        match affordance {
            AffordanceRef::Thing => Self::Thing,
            AffordanceRef::Property(name) => Self::Property(name.to_owned()),
            AffordanceRef::Action(name) => Self::Action(name.to_owned()),
            AffordanceRef::Event(name) => Self::Event(name.to_owned()),
        }
    }
}

/// Cache key for a Servient-selected TD form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFormCacheKey {
    /// Thing id used for the consumed interaction.
    pub thing_id: String,
    /// Affordance location used for form selection.
    pub affordance: SelectedFormCacheAffordance,
    /// Required effective operation.
    pub operation: Operation,
    /// Optional required form content type.
    pub content_type: Option<String>,
    /// Optional required form subprotocol.
    pub subprotocol: Option<String>,
}

impl SelectedFormCacheKey {
    /// Creates a cache key from a Thing id, affordance location, and selection criteria.
    pub fn new(
        thing_id: impl Into<String>,
        affordance: SelectedFormCacheAffordance,
        criteria: FormSelectionCriteria<'_>,
    ) -> Self {
        Self {
            thing_id: thing_id.into(),
            affordance,
            operation: criteria.operation,
            content_type: criteria.content_type.map(str::to_owned),
            subprotocol: criteria.subprotocol.map(str::to_owned),
        }
    }
}

/// Cache boundary for selected TD forms used by Servient-level invocation APIs.
pub trait SelectedFormCache {
    /// Retrieves a cached form selection.
    fn get(&self, key: &SelectedFormCacheKey) -> Option<Form>;

    /// Inserts or replaces a cached form selection.
    fn insert(&self, key: SelectedFormCacheKey, form: Form) -> Option<Form>;

    /// Removes a cached form selection.
    fn remove(&self, key: &SelectedFormCacheKey) -> Option<Form>;

    /// Removes all cached form selections for a Thing id.
    fn remove_thing(&self, id: &str);
}

/// Deterministic in-memory cache for Servient-selected TD forms.
pub struct InMemorySelectedFormCache {
    forms: std::cell::RefCell<Vec<(SelectedFormCacheKey, Form)>>,
}

impl InMemorySelectedFormCache {
    /// Creates an empty selected form cache.
    pub fn new() -> Self {
        Self {
            forms: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Returns the number of cached form selections.
    pub fn len(&self) -> usize {
        self.forms.borrow().len()
    }

    /// Returns true when the cache contains no selected forms.
    pub fn is_empty(&self) -> bool {
        self.forms.borrow().is_empty()
    }
}

impl Default for InMemorySelectedFormCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectedFormCache for InMemorySelectedFormCache {
    fn get(&self, key: &SelectedFormCacheKey) -> Option<Form> {
        self.forms
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, form)| form.clone())
    }

    fn insert(&self, key: SelectedFormCacheKey, form: Form) -> Option<Form> {
        let mut forms = self.forms.borrow_mut();
        if let Some((_, cached_form)) = forms.iter_mut().find(|(candidate, _)| *candidate == key) {
            let previous = cached_form.clone();
            *cached_form = form;
            Some(previous)
        } else {
            forms.push((key, form));
            None
        }
    }

    fn remove(&self, key: &SelectedFormCacheKey) -> Option<Form> {
        let mut forms = self.forms.borrow_mut();
        forms
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| forms.remove(index).1)
    }

    fn remove_thing(&self, id: &str) {
        self.forms
            .borrow_mut()
            .retain(|(key, _)| key.thing_id != id);
    }
}

/// Protocol-neutral cached binding plan for a criteria-selected remote request.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingPlan {
    /// Selected TD form for the remote interaction.
    pub form: Form,
    /// Index of the protocol binding factory selected for this form.
    pub binding_factory_index: usize,
}

/// Cache boundary for criteria-selected forms and protocol binding factories.
pub trait BindingPlanCache {
    /// Retrieves a cached binding plan by the same key used for selected forms.
    fn get(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan>;

    /// Inserts or replaces a cached binding plan.
    fn insert(&self, key: SelectedFormCacheKey, plan: BindingPlan) -> Option<BindingPlan>;

    /// Removes a cached binding plan.
    fn remove(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan>;

    /// Removes all cached binding plans for a Thing id.
    fn remove_thing(&self, id: &str);
}

/// Deterministic in-memory cache for Servient binding plans.
pub struct InMemoryBindingPlanCache {
    plans: std::cell::RefCell<Vec<(SelectedFormCacheKey, BindingPlan)>>,
}

impl InMemoryBindingPlanCache {
    /// Creates an empty binding plan cache.
    pub fn new() -> Self {
        Self {
            plans: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Returns the number of cached binding plans.
    pub fn len(&self) -> usize {
        self.plans.borrow().len()
    }

    /// Returns true when the cache contains no binding plans.
    pub fn is_empty(&self) -> bool {
        self.plans.borrow().is_empty()
    }
}

impl Default for InMemoryBindingPlanCache {
    fn default() -> Self {
        Self::new()
    }
}

impl BindingPlanCache for InMemoryBindingPlanCache {
    fn get(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan> {
        self.plans
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, plan)| plan.clone())
    }

    fn insert(&self, key: SelectedFormCacheKey, plan: BindingPlan) -> Option<BindingPlan> {
        let mut plans = self.plans.borrow_mut();
        if let Some((_, cached_plan)) = plans.iter_mut().find(|(candidate, _)| *candidate == key) {
            let previous = cached_plan.clone();
            *cached_plan = plan;
            Some(previous)
        } else {
            plans.push((key, plan));
            None
        }
    }

    fn remove(&self, key: &SelectedFormCacheKey) -> Option<BindingPlan> {
        let mut plans = self.plans.borrow_mut();
        plans
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| plans.remove(index).1)
    }

    fn remove_thing(&self, id: &str) {
        self.plans
            .borrow_mut()
            .retain(|(key, _)| key.thing_id != id);
    }
}

struct ActiveBindingPlan {
    form: Form,
    binding: Box<dyn ProtocolBinding>,
}

/// Builder for a host Servient.
pub struct ServientBuilder<
    D = InMemoryThingDirectory,
    R = InMemoryExposedThingRegistry,
    C = InMemoryConsumedThingCache,
    S = InMemorySelectedFormCache,
    P = InMemoryBindingPlanCache,
> {
    directory: D,
    exposed_registry: R,
    consumed_cache: C,
    selected_form_cache: S,
    binding_plan_cache: P,
    binding_factories: Vec<BindingFactory>,
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
    pub fn build(self) -> Servient<D, R, C, S, P> {
        Servient {
            directory: self.directory,
            exposed_registry: self.exposed_registry,
            consumed_cache: self.consumed_cache,
            selected_form_cache: self.selected_form_cache,
            binding_plan_cache: self.binding_plan_cache,
            binding_factories: self.binding_factories,
            running: false,
        }
    }
}

/// Host Servient that composes discovery, exposed Things, and consumed Things.
pub struct Servient<
    D = InMemoryThingDirectory,
    R = InMemoryExposedThingRegistry,
    C = InMemoryConsumedThingCache,
    S = InMemorySelectedFormCache,
    P = InMemoryBindingPlanCache,
> {
    directory: D,
    exposed_registry: R,
    consumed_cache: C,
    selected_form_cache: S,
    binding_plan_cache: P,
    binding_factories: Vec<BindingFactory>,
    running: bool,
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

    /// Registers a protocol binding factory after the Servient has been built.
    pub fn register_binding_factory<F>(&mut self, factory: F) -> ServientResult<()>
    where
        F: Fn() -> Box<dyn ProtocolBinding> + 'static,
    {
        self.ensure_stopped()?;
        self.binding_factories.push(Box::new(factory));
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
        let thing = self.consumed_thing_description(id)?;
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

    /// Reads a property on a remote Thing through the first form matching criteria.
    pub fn read_remote_property_with_criteria(
        &self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Property(name),
            AffordanceRef::Property(name),
            criteria_for_operation(criteria, Operation::ReadProperty),
            input,
        )
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

    /// Writes a property on a remote Thing through the first form matching criteria.
    pub fn write_remote_property_with_criteria(
        &self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Property(name),
            AffordanceRef::Property(name),
            criteria_for_operation(criteria, Operation::WriteProperty),
            input,
        )
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

    /// Invokes an action on a remote Thing through the first form matching criteria.
    pub fn invoke_remote_action_with_criteria(
        &self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Action(name),
            AffordanceRef::Action(name),
            criteria_for_operation(criteria, Operation::InvokeAction),
            input,
        )
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

    /// Subscribes to a remote event through the first form matching criteria.
    pub fn subscribe_remote_event_with_criteria(
        &self,
        id: &str,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.request_remote_with_criteria(
            id,
            AffordanceTarget::Event(name),
            AffordanceRef::Event(name),
            criteria_for_operation(criteria, Operation::SubscribeEvent),
            input,
        )
    }

    fn request_remote_with_criteria(
        &self,
        id: &str,
        target: AffordanceTarget<'_>,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let thing = self.consumed_thing_description(id)?;
        let active_plan = self.cached_or_select_binding_plan(&thing, id, affordance, criteria)?;
        let mut consumed = self.bound_consumed_thing_with_binding(thing, active_plan.binding);

        consumed
            .request(target, criteria.operation, &active_plan.form, input)
            .map_err(Into::into)
    }

    fn cached_or_select_binding_plan(
        &self,
        thing: &Thing,
        id: &str,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<ActiveBindingPlan> {
        let key = SelectedFormCacheKey::new(
            id,
            SelectedFormCacheAffordance::from_affordance_ref(affordance),
            criteria,
        );

        if let Some(plan) = self.binding_plan_cache.get(&key) {
            match self.active_binding_plan_from_cache(thing, affordance, criteria, plan) {
                Ok(active_plan) => return Ok(active_plan),
                Err(_) => {
                    self.binding_plan_cache.remove(&key);
                }
            }
        }

        let form = self.cached_or_select_form(thing, id, affordance, criteria)?;
        let (binding_factory_index, binding) =
            self.select_binding_factory_for_form(&form, criteria.operation)?;
        self.binding_plan_cache.insert(
            key,
            BindingPlan {
                form: form.clone(),
                binding_factory_index,
            },
        );

        Ok(ActiveBindingPlan { form, binding })
    }

    fn active_binding_plan_from_cache(
        &self,
        thing: &Thing,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
        plan: BindingPlan,
    ) -> ServientResult<ActiveBindingPlan> {
        validate_affordance_form_with_criteria(thing, affordance, &plan.form, criteria)?;
        let binding = self.binding_from_factory_index(plan.binding_factory_index)?;
        if binding.supports(&plan.form, criteria.operation) {
            Ok(ActiveBindingPlan {
                form: plan.form,
                binding,
            })
        } else {
            Err(CoreError::UnsupportedBinding(format!(
                "Cached binding factory {} no longer supports {:?} for {}",
                plan.binding_factory_index,
                criteria.operation,
                plan.form.href.as_str()
            ))
            .into())
        }
    }

    fn cached_or_select_form(
        &self,
        thing: &Thing,
        id: &str,
        affordance: AffordanceRef<'_>,
        criteria: FormSelectionCriteria<'_>,
    ) -> ServientResult<Form> {
        let key = SelectedFormCacheKey::new(
            id,
            SelectedFormCacheAffordance::from_affordance_ref(affordance),
            criteria,
        );

        if let Some(form) = self.selected_form_cache.get(&key) {
            if validate_affordance_form_with_criteria(thing, affordance, &form, criteria).is_ok() {
                return Ok(form);
            }
            self.selected_form_cache.remove(&key);
        }

        let form = select_affordance_form_with_criteria(thing, affordance, criteria)?
            .selection
            .form
            .clone();
        self.selected_form_cache.insert(key, form.clone());
        Ok(form)
    }

    fn select_binding_factory_for_form(
        &self,
        form: &Form,
        operation: Operation,
    ) -> ServientResult<(usize, Box<dyn ProtocolBinding>)> {
        for (index, factory) in self.binding_factories.iter().enumerate() {
            let binding = factory();
            if binding.supports(form, operation) {
                return Ok((index, binding));
            }
        }

        Err(CoreError::UnsupportedBinding(format!(
            "No binding supports {:?} for {}",
            operation,
            form.href.as_str()
        ))
        .into())
    }

    fn binding_from_factory_index(&self, index: usize) -> ServientResult<Box<dyn ProtocolBinding>> {
        self.binding_factories
            .get(index)
            .map(|factory| factory())
            .ok_or_else(|| {
                CoreError::UnsupportedBinding(format!(
                    "Binding factory index {} is not registered",
                    index
                ))
                .into()
            })
    }

    fn consumed_thing_description(&self, id: &str) -> ServientResult<Thing> {
        match self.consumed_cache.get(id) {
            Some(thing) => Ok(thing),
            None => self.directory.get(id).map_err(Into::into),
        }
    }

    fn bound_consumed_thing(&self, thing: Thing) -> BoundConsumedThing {
        let mut consumed = BoundConsumedThing::new(thing);
        for factory in &self.binding_factories {
            consumed.register_binding(factory());
        }
        consumed
    }

    fn bound_consumed_thing_with_binding(
        &self,
        thing: Thing,
        binding: Box<dyn ProtocolBinding>,
    ) -> BoundConsumedThing {
        let mut consumed = BoundConsumedThing::new(thing);
        consumed.register_binding(binding);
        consumed
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

fn criteria_for_operation<'a>(
    criteria: FormSelectionCriteria<'a>,
    operation: Operation,
) -> FormSelectionCriteria<'a> {
    FormSelectionCriteria {
        operation,
        content_type: criteria.content_type,
        subprotocol: criteria.subprotocol,
    }
}
