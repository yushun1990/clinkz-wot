use alloc::{boxed::Box, collections::BTreeMap, format, string::String, sync::Arc, vec::Vec};

use clinkz_wot_td::{
    data_type::Operation,
    form::Form,
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
};

use crate::{BindingRequest, ClientBinding, CoreError, CoreResult, Payload, Principal};

/// Location of an affordance within a Thing Description.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AffordanceTarget {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(String),
    /// An action affordance by name.
    Action(String),
    /// An event affordance by name.
    Event(String),
}

/// Input provided to an interaction handler.
///
/// For inbound (exposed) interactions, `principal` carries the verified caller
/// identity (or `None` for NoSec / local dispatch). For outbound (consumed)
/// interactions, `principal` is always `None` and `security_metadata` carries
/// transport-level auth material (e.g. `Authorization` headers).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionInput {
    /// Optional encoded payload for write, action, subscription, or cancellation flows.
    pub payload: Option<Payload>,
    /// URI template or protocol binding parameters supplied by the caller.
    pub parameters: BTreeMap<String, String>,
    /// Verified caller identity for inbound interactions; `None` for outbound
    /// or local in-process calls.
    pub principal: Option<Principal>,
    /// Security metadata (e.g. `Authorization` headers, API keys) applied by
    /// [`SecurityProvider::apply`](crate::SecurityProvider::apply) for outbound
    /// interactions. Bindings SHOULD send these as protocol-level auth headers
    /// or attachments rather than URI parameters.
    pub security_metadata: BTreeMap<String, String>,
}

impl InteractionInput {
    /// Creates an empty interaction input.
    pub fn empty() -> Self {
        Self {
            payload: None,
            parameters: BTreeMap::new(),
            principal: None,
            security_metadata: BTreeMap::new(),
        }
    }

    /// Creates an interaction input containing a payload.
    pub fn with_payload(payload: Payload) -> Self {
        Self {
            payload: Some(payload),
            parameters: BTreeMap::new(),
            principal: None,
            security_metadata: BTreeMap::new(),
        }
    }
}

/// Output returned by an interaction handler or consumed Thing call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionOutput {
    /// Optional encoded response payload.
    pub payload: Option<Payload>,
}

impl InteractionOutput {
    /// Creates an empty output.
    pub fn empty() -> Self {
        Self { payload: None }
    }

    /// Creates an output containing a payload.
    pub fn with_payload(payload: Payload) -> Self {
        Self {
            payload: Some(payload),
        }
    }
}

/// Sink used by event subscriptions without requiring an async runtime.
pub trait EventSink {
    /// Accepts the next event payload.
    fn emit(&mut self, payload: Payload) -> CoreResult<()>;
}

/// Handler for reading a local property affordance (W3C Scripting API
/// `setPropertyReadHandler`).
///
/// # Reentrancy
///
/// Handlers run with the per-Thing slot lock **released** (the engine clones
/// the handler out under a brief lock, then invokes it outside). Implementations
/// may therefore freely re-enter the Servient — call
/// [`ExposedThingHandle`](crate::ExposedThingHandle) methods, `emit_event`,
/// `add_property`, etc. on the same or other Things — without self-deadlock.
///
/// Mutating internal handler state across **concurrent** invocations (e.g. when
/// multiple application threads drive the same Thing) requires internal
/// synchronization (`Arc<Mutex<…>>`, `Cell`, etc.); the engine serializes
/// driving-loop interactions within a Thing but does not serialize
/// application-facing handle calls against an in-flight driving handler.
pub trait PropertyReadHandler {
    /// Reads the current property value.
    fn read(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for writing a local property affordance (W3C Scripting API
/// `setPropertyWriteHandler`).
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait PropertyWriteHandler {
    /// Writes a new property value.
    fn write(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for observing a local property affordance (W3C Scripting API
/// `setPropertyObserveHandler`).
///
/// Called once when a remote consumer starts observing. The handler may emit
/// the initial value through `sink`. Subsequent value changes are pushed by
/// the application via
/// [`ExposedThingHandle::emit_event`](crate::ExposedThingHandle::emit_event)
/// or a dedicated property-change emission path.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait PropertyObserveHandler {
    /// Called when observation starts; may emit initial values through `sink`.
    fn observe(
        &self,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;
}

/// Handler for unobserving a local property affordance (W3C Scripting API
/// `setPropertyUnobserveHandler`).
///
/// Called when a remote consumer stops observing. Allows cleanup of
/// per-observer state.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait PropertyUnobserveHandler {
    /// Called when observation stops; allows cleanup of per-observer state.
    fn unobserve(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for a local action affordance.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait ActionHandler {
    /// Invokes the action.
    fn invoke(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for querying the status of a local action affordance (W3C TD
/// `queryaction` operation).
///
/// Called when a remote consumer queries an action's status. When no query
/// handler is registered the inbound dispatcher returns `MissingHandler` so
/// callers receive a clear error instead of a silent empty reply.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait ActionQueryHandler {
    /// Queries the action status.
    fn query(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for cancelling a local action affordance (W3C TD `cancelaction`
/// operation).
///
/// Called when a remote consumer cancels an ongoing action invocation. When no
/// cancel handler is registered the inbound dispatcher acknowledges the
/// request with an empty reply.
///
/// `cancelaction` is a TD 2.0 operation; this handler is only available under
/// the `td2-preview` feature.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
#[cfg(feature = "td2-preview")]
pub trait ActionCancelHandler {
    /// Cancels the ongoing action invocation.
    fn cancel(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for event subscription on a local event affordance (W3C Scripting
/// API `setEventSubscribeHandler`).
///
/// Called when a consumer subscribes; may emit initial event payloads.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait EventSubscribeHandler {
    /// Called when a consumer subscribes; may emit initial event payloads.
    fn subscribe(
        &self,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;
}

/// Handler for event unsubscription on a local event affordance (W3C Scripting
/// API `setEventUnsubscribeHandler`).
///
/// Called when a consumer unsubscribes; allows cleanup of per-subscriber
/// state.
///
/// # Reentrancy
///
/// See [`PropertyReadHandler`] — handlers run with the slot lock released and
/// may re-enter the Servient.
pub trait EventUnsubscribeHandler {
    /// Called when a consumer unsubscribes; allows cleanup of per-subscriber
    /// state.
    fn unsubscribe(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

// ---------------------------------------------------------------------------
// Async handler traits (behind `async` feature — M9).
// ---------------------------------------------------------------------------

/// Async handler for reading a property (M9).
///
/// When registered, the async driving loop calls this handler instead of the
/// sync [`PropertyReadHandler`], allowing async I/O without blocking the
/// driving loop.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyReadHandler: Send {
    /// Reads the current property value.
    async fn read(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Async handler for writing a property (M9).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyWriteHandler: Send {
    /// Writes a new property value.
    async fn write(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Async handler for invoking an action (M9).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncActionHandler: Send {
    /// Invokes the action.
    async fn invoke(&self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// All handlers registered for a single property affordance.
#[derive(Default)]
struct PropertyHandlerSet {
    read: Option<Arc<dyn PropertyReadHandler>>,
    write: Option<Arc<dyn PropertyWriteHandler>>,
    observe: Option<Arc<dyn PropertyObserveHandler>>,
    unobserve: Option<Arc<dyn PropertyUnobserveHandler>>,
    #[cfg(feature = "async")]
    async_read: Option<Arc<dyn AsyncPropertyReadHandler + Send>>,
    #[cfg(feature = "async")]
    async_write: Option<Arc<dyn AsyncPropertyWriteHandler + Send>>,
}

/// All handlers registered for a single event affordance.
#[derive(Default)]
struct EventHandlerSet {
    subscribe: Option<Arc<dyn EventSubscribeHandler>>,
    unsubscribe: Option<Arc<dyn EventUnsubscribeHandler>>,
}

/// All handlers registered for a single action affordance.
#[derive(Default)]
struct ActionHandlerSet {
    invoke: Option<Arc<dyn ActionHandler>>,
    query: Option<Arc<dyn ActionQueryHandler>>,
    #[cfg(feature = "td2-preview")]
    cancel: Option<Arc<dyn ActionCancelHandler>>,
    #[cfg(feature = "async")]
    async_invoke: Option<Arc<dyn AsyncActionHandler + Send>>,
}

/// Protocol-neutral local Thing dispatcher.
pub struct LocalThing {
    thing: Thing,
    property_handlers: BTreeMap<String, PropertyHandlerSet>,
    action_handlers: BTreeMap<String, ActionHandlerSet>,
    event_handlers: BTreeMap<String, EventHandlerSet>,
}

/// Protocol-neutral dispatcher for consuming a remote Thing through bindings.
pub struct BoundConsumedThing {
    thing: Arc<Thing>,
    bindings: Vec<Box<dyn ClientBinding>>,
}

impl BoundConsumedThing {
    /// Creates a consumed Thing dispatcher for a Thing Description.
    pub fn new(thing: Thing) -> Self {
        Self {
            thing: Arc::new(thing),
            bindings: Vec::new(),
        }
    }

    /// Creates a consumed Thing dispatcher from an already-shared `Arc<Thing>`.
    ///
    /// Use this when the caller already holds a shared Thing (e.g. from an
    /// interned consumed-Thing entry) to avoid a full TD deep-clone followed
    /// immediately by re-wrapping it in a fresh `Arc`.
    pub fn from_arc(thing: Arc<Thing>) -> Self {
        Self {
            thing,
            bindings: Vec::new(),
        }
    }

    /// Returns the Thing Description owned by this dispatcher.
    pub fn thing_description(&self) -> &Thing {
        &self.thing
    }

    /// Registers a protocol binding.
    pub fn register_binding(&mut self, binding: impl ClientBinding + 'static) {
        self.bindings.push(Box::new(binding));
    }

    fn forms_for_target(&self, target: &AffordanceTarget) -> CoreResult<FormSet<'_>> {
        match target {
            AffordanceTarget::Thing => Ok(FormSet {
                context: FormContext::Thing,
                forms: self.thing.forms.as_deref().unwrap_or(&[]),
            }),
            AffordanceTarget::Property(name) => {
                let property = find_affordance("property", name, &self.thing.properties)?;
                Ok(FormSet {
                    context: FormContext::Property(property),
                    forms: property._interaction.forms.as_slice(),
                })
            }
            AffordanceTarget::Action(name) => {
                let action = find_affordance("action", name, &self.thing.actions)?;
                Ok(FormSet {
                    context: FormContext::Action(action),
                    forms: action._interaction.forms.as_slice(),
                })
            }
            AffordanceTarget::Event(name) => {
                let event = find_affordance("event", name, &self.thing.events)?;
                Ok(FormSet {
                    context: FormContext::Event(event),
                    forms: event._interaction.forms.as_slice(),
                })
            }
        }
    }

    fn validate_selected_form(
        &self,
        target: &AffordanceTarget,
        operation: Operation,
        form: &Form,
    ) -> CoreResult<()> {
        let form_set = self.forms_for_target(target)?;

        // The public `ConsumedThing::request` entry point accepts an arbitrary
        // caller-supplied `Arc<Form>`. Verify the form actually belongs to this
        // affordance; otherwise a foreign form (or a no-`op` form) could pass
        // by falling back to the affordance's default operations and dispatch
        // to the wrong binding.
        if !form_set.forms.iter().any(|candidate| candidate == form) {
            return Err(CoreError::InvalidInteraction(format!(
                "Selected form does not belong to the {:?} affordance",
                target
            )));
        }

        if effective_form_operations(form_set.context, form).contains(&operation) {
            Ok(())
        } else {
            Err(CoreError::UnsupportedOperation(format!(
                "Form does not support {}",
                operation.as_str()
            )))
        }
    }
}

struct FormSet<'a> {
    context: FormContext<'a>,
    forms: &'a [Form],
}

fn find_affordance<'a, T>(
    kind: &'static str,
    name: &str,
    affordances: &'a Option<BTreeMap<String, T>>,
) -> CoreResult<&'a T> {
    affordances
        .as_ref()
        .and_then(|affordances| affordances.get(name))
        .ok_or_else(|| CoreError::UnknownAffordance {
            kind,
            name: name.into(),
        })
}

impl ConsumedThing for BoundConsumedThing {
    fn thing_description(&self) -> &Thing {
        &self.thing
    }

    fn request(
        &mut self,
        target: AffordanceTarget,
        operation: Operation,
        form: Arc<Form>,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.validate_selected_form(&target, operation, &form)?;

        let binding = self
            .bindings
            .iter()
            .find(|binding| binding.supports_with_thing(&self.thing, &form, operation))
            .ok_or_else(|| {
                CoreError::UnsupportedBinding(format!(
                    "No binding supports {} for {}",
                    operation.as_str(),
                    form.href.as_str()
                ))
            })?;

        binding.invoke(BindingRequest {
            thing: Arc::clone(&self.thing),
            target,
            operation,
            form: Arc::clone(&form),
            input,
        })
    }
}

impl LocalThing {
    /// Creates a local dispatcher for a Thing Description.
    pub fn new(thing: Thing) -> Self {
        Self {
            thing,
            property_handlers: BTreeMap::new(),
            action_handlers: BTreeMap::new(),
            event_handlers: BTreeMap::new(),
        }
    }

    /// Returns the Thing Description owned by this dispatcher.
    pub fn thing_description(&self) -> &Thing {
        &self.thing
    }

    /// Registers a property read handler by affordance name.
    pub fn register_property_read_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyReadHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().read = Some(Arc::new(handler));
    }

    /// Registers a property write handler by affordance name.
    pub fn register_property_write_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyWriteHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().write = Some(Arc::new(handler));
    }

    /// Registers a property observe handler by affordance name.
    pub fn register_property_observe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyObserveHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .observe = Some(Arc::new(handler));
    }

    /// Registers a property unobserve handler by affordance name (W3C Scripting
    /// API `setPropertyUnobserveHandler`).
    pub fn register_property_unobserve_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyUnobserveHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .unobserve = Some(Arc::new(handler));
    }

    /// Registers an action handler by affordance name.
    pub fn register_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionHandler + 'static,
    ) -> Option<Arc<dyn ActionHandler>> {
        self.action_handlers
            .entry(name.into())
            .or_default()
            .invoke
            .replace(Arc::new(handler))
    }

    /// Registers an action query handler by affordance name (W3C TD
    /// `queryaction` operation).
    pub fn register_action_query_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionQueryHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().query = Some(Arc::new(handler));
    }

    /// Registers an action cancel handler by affordance name (W3C TD
    /// `cancelaction` operation; TD 2.0, requires `td2-preview`).
    #[cfg(feature = "td2-preview")]
    pub fn register_action_cancel_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionCancelHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().cancel = Some(Arc::new(handler));
    }

    /// Registers an async property read handler (M9, behind `async` feature).
    #[cfg(feature = "async")]
    pub fn register_async_property_read_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncPropertyReadHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .async_read = Some(Arc::new(handler));
    }

    /// Registers an async property write handler (M9).
    #[cfg(feature = "async")]
    pub fn register_async_property_write_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncPropertyWriteHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .async_write = Some(Arc::new(handler));
    }

    /// Registers an async action handler (M9).
    #[cfg(feature = "async")]
    pub fn register_async_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncActionHandler + 'static,
    ) {
        self.action_handlers
            .entry(name.into())
            .or_default()
            .async_invoke = Some(Arc::new(handler));
    }

    /// Clones the async read handler for dispatch without removing it.
    ///
    /// The handler stays registered, so concurrent requests for the same
    /// affordance each get their own `Arc` clone instead of observing
    /// `MissingHandler`.
    #[cfg(feature = "async")]
    pub fn async_read_handler(
        &self,
        name: &str,
    ) -> Option<Arc<dyn AsyncPropertyReadHandler + Send>> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.async_read.clone())
    }

    /// Clones the async write handler for dispatch without removing it.
    #[cfg(feature = "async")]
    pub fn async_write_handler(
        &self,
        name: &str,
    ) -> Option<Arc<dyn AsyncPropertyWriteHandler + Send>> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.async_write.clone())
    }

    /// Clones the async action handler for dispatch without removing it.
    #[cfg(feature = "async")]
    pub fn async_action_handler(&self, name: &str) -> Option<Arc<dyn AsyncActionHandler + Send>> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.async_invoke.clone())
    }

    /// Clones the sync read handler for dispatch without removing it.
    ///
    /// The handler stays registered; concurrent requests each get their own
    /// `Arc` clone. Dispatch clones the handler out under the per-Thing slot
    /// lock and invokes it with the lock released (reentrancy-safe — see the
    /// [`PropertyReadHandler`] docs).
    pub fn read_handler(&self, name: &str) -> Option<Arc<dyn PropertyReadHandler>> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.read.clone())
    }

    /// Clones the sync write handler for dispatch without removing it.
    pub fn write_handler(&self, name: &str) -> Option<Arc<dyn PropertyWriteHandler>> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.write.clone())
    }

    /// Clones the sync observe handler for dispatch without removing it.
    pub fn observe_handler(&self, name: &str) -> Option<Arc<dyn PropertyObserveHandler>> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.observe.clone())
    }

    /// Clones the sync unobserve handler for dispatch without removing it.
    pub fn unobserve_handler(&self, name: &str) -> Option<Arc<dyn PropertyUnobserveHandler>> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.unobserve.clone())
    }

    /// Clones the sync action handler for dispatch without removing it.
    pub fn action_handler(&self, name: &str) -> Option<Arc<dyn ActionHandler>> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.invoke.clone())
    }

    /// Clones the sync action query handler for dispatch without removing it.
    pub fn action_query_handler(&self, name: &str) -> Option<Arc<dyn ActionQueryHandler>> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.query.clone())
    }

    /// Clones the sync action cancel handler for dispatch without removing it.
    #[cfg(feature = "td2-preview")]
    pub fn action_cancel_handler(&self, name: &str) -> Option<Arc<dyn ActionCancelHandler>> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.cancel.clone())
    }

    /// Clones the sync event subscribe handler for dispatch without removing it.
    pub fn subscribe_handler(&self, name: &str) -> Option<Arc<dyn EventSubscribeHandler>> {
        self.event_handlers
            .get(name)
            .and_then(|set| set.subscribe.clone())
    }

    /// Clones the sync event unsubscribe handler for dispatch without removing it.
    pub fn unsubscribe_handler(&self, name: &str) -> Option<Arc<dyn EventUnsubscribeHandler>> {
        self.event_handlers
            .get(name)
            .and_then(|set| set.unsubscribe.clone())
    }

    /// Registers an event subscribe handler by affordance name.
    pub fn register_event_subscribe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl EventSubscribeHandler + 'static,
    ) {
        self.event_handlers
            .entry(name.into())
            .or_default()
            .subscribe = Some(Arc::new(handler));
    }

    /// Registers an event unsubscribe handler by affordance name.
    pub fn register_event_unsubscribe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl EventUnsubscribeHandler + 'static,
    ) {
        self.event_handlers
            .entry(name.into())
            .or_default()
            .unsubscribe = Some(Arc::new(handler));
    }

    /// Returns `Err(UnknownAffordance)` if no property with `name` is declared.
    pub fn ensure_property_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("property", name, &self.thing.properties)
    }

    /// Returns `Err(UnknownAffordance)` if no action with `name` is declared.
    pub fn ensure_action_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("action", name, &self.thing.actions)
    }

    /// Returns `Err(UnknownAffordance)` if no event with `name` is declared.
    pub fn ensure_event_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("event", name, &self.thing.events)
    }

    /// Adds a property affordance to the TD at runtime (W3C Scripting API
    /// `addProperty`).
    ///
    /// Returns `Err` if a property with the same name already exists.
    pub fn add_property(
        &mut self,
        name: impl Into<String>,
        property: clinkz_wot_td::affordance::PropertyAffordance,
    ) -> CoreResult<()> {
        let name = name.into();
        let properties = self.thing.properties.get_or_insert_with(BTreeMap::new);
        if properties.contains_key(&name) {
            return Err(CoreError::InvalidInteraction(format!(
                "Property '{}' already exists",
                name
            )));
        }
        properties.insert(name, property);
        Ok(())
    }

    /// Removes a property affordance from the TD at runtime (W3C Scripting API
    /// `removeProperty`).
    pub fn remove_property(&mut self, name: &str) {
        if let Some(properties) = &mut self.thing.properties {
            properties.remove(name);
        }
        self.property_handlers.remove(name);
    }

    /// Adds an action affordance to the TD at runtime (W3C Scripting API
    /// `addAction`).
    pub fn add_action(
        &mut self,
        name: impl Into<String>,
        action: clinkz_wot_td::affordance::ActionAffordance,
    ) -> CoreResult<()> {
        let name = name.into();
        let actions = self.thing.actions.get_or_insert_with(BTreeMap::new);
        if actions.contains_key(&name) {
            return Err(CoreError::InvalidInteraction(format!(
                "Action '{}' already exists",
                name
            )));
        }
        actions.insert(name, action);
        Ok(())
    }

    /// Removes an action affordance from the TD at runtime.
    pub fn remove_action(&mut self, name: &str) {
        if let Some(actions) = &mut self.thing.actions {
            actions.remove(name);
        }
        self.action_handlers.remove(name);
    }

    /// Adds an event affordance to the TD at runtime (W3C Scripting API
    /// `addEvent`).
    pub fn add_event(
        &mut self,
        name: impl Into<String>,
        event: clinkz_wot_td::affordance::EventAffordance,
    ) -> CoreResult<()> {
        let name = name.into();
        let events = self.thing.events.get_or_insert_with(BTreeMap::new);
        if events.contains_key(&name) {
            return Err(CoreError::InvalidInteraction(format!(
                "Event '{}' already exists",
                name
            )));
        }
        events.insert(name, event);
        Ok(())
    }

    /// Removes an event affordance from the TD at runtime.
    pub fn remove_event(&mut self, name: &str) {
        if let Some(events) = &mut self.thing.events {
            events.remove(name);
        }
        self.event_handlers.remove(name);
    }
}

impl ExposedThing for LocalThing {
    fn thing_description(&self) -> &Thing {
        &self.thing
    }

    fn read_property(&self, name: &str, input: InteractionInput) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handler = self.read_handler(name).ok_or(CoreError::MissingHandler)?;
        handler.read(input)
    }

    fn write_property(&self, name: &str, input: InteractionInput) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handler = self.write_handler(name).ok_or(CoreError::MissingHandler)?;
        handler.write(input)
    }

    fn observe_property(
        &self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handler = self
            .observe_handler(name)
            .ok_or(CoreError::MissingHandler)?;
        handler.observe(input, sink)
    }

    fn unobserve_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handler = self
            .unobserve_handler(name)
            .ok_or(CoreError::MissingHandler)?;
        handler.unobserve(input)
    }

    fn invoke_action(&self, name: &str, input: InteractionInput) -> CoreResult<InteractionOutput> {
        self.ensure_action_affordance(name)?;
        let handler = self.action_handler(name).ok_or(CoreError::MissingHandler)?;
        handler.invoke(input)
    }

    fn subscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_event_affordance(name)?;
        let handler = self
            .subscribe_handler(name)
            .ok_or(CoreError::MissingHandler)?;
        handler.subscribe(input, sink)
    }

    fn unsubscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_event_affordance(name)?;
        let handler = self
            .unsubscribe_handler(name)
            .ok_or(CoreError::MissingHandler)?;
        handler.unsubscribe(input)
    }
}

fn ensure_affordance<T>(
    kind: &'static str,
    name: &str,
    affordances: &Option<BTreeMap<String, T>>,
) -> CoreResult<()> {
    if affordances
        .as_ref()
        .is_some_and(|affordances| affordances.contains_key(name))
    {
        Ok(())
    } else {
        Err(CoreError::UnknownAffordance {
            kind,
            name: name.into(),
        })
    }
}

/// Protocol-neutral interface implemented by locally exposed Things.
pub trait ExposedThing {
    /// Returns the Thing Description that describes this exposed Thing.
    fn thing_description(&self) -> &Thing;

    /// Reads a property.
    fn read_property(&self, name: &str, input: InteractionInput) -> CoreResult<InteractionOutput>;

    /// Writes a property.
    fn write_property(&self, name: &str, input: InteractionInput) -> CoreResult<InteractionOutput>;

    /// Starts observing a property; the handler may emit the initial value
    /// through `sink`.
    fn observe_property(
        &self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;

    /// Stops observing a property; allows cleanup of per-observer state.
    fn unobserve_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Invokes an action.
    fn invoke_action(&self, name: &str, input: InteractionInput) -> CoreResult<InteractionOutput>;

    /// Subscribes to an event source.
    fn subscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;

    /// Unsubscribes from an event source.
    fn unsubscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}

/// Protocol-neutral interface for consuming a remote Thing through bindings.
pub trait ConsumedThing {
    /// Returns the Thing Description used by this consumed Thing.
    fn thing_description(&self) -> &Thing;

    /// Performs an operation against a selected affordance form.
    fn request(
        &mut self,
        target: AffordanceTarget,
        operation: Operation,
        form: Arc<Form>,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
