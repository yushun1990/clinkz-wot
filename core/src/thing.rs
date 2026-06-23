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
pub trait PropertyReadHandler {
    /// Reads the current property value.
    fn read(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for writing a local property affordance (W3C Scripting API
/// `setPropertyWriteHandler`).
pub trait PropertyWriteHandler {
    /// Writes a new property value.
    fn write(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for observing a local property affordance (W3C Scripting API
/// `setPropertyObserveHandler`).
///
/// Called once when a remote consumer starts observing. The handler may emit
/// the initial value through `sink`. Subsequent value changes are pushed by
/// the application via
/// [`ExposedThingHandle::emit_event`](crate::ExposedThingHandle::emit_event)
/// or a dedicated property-change emission path.
pub trait PropertyObserveHandler {
    /// Called when observation starts; may emit initial values through `sink`.
    fn observe(
        &mut self,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;
}

/// Handler for a local action affordance.
pub trait ActionHandler {
    /// Invokes the action.
    fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for event subscription on a local event affordance (W3C Scripting
/// API `setEventSubscribeHandler`).
pub trait EventSubscribeHandler {
    /// Called when a consumer subscribes; may emit initial event payloads.
    fn subscribe(
        &mut self,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;
}

/// Handler for event unsubscription on a local event affordance (W3C Scripting
/// API `setEventUnsubscribeHandler`).
pub trait EventUnsubscribeHandler {
    /// Called when a consumer unsubscribes; allows cleanup of per-subscriber
    /// state.
    fn unsubscribe(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
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
    async fn read(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Async handler for writing a property (M9).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncPropertyWriteHandler: Send {
    /// Writes a new property value.
    async fn write(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Async handler for invoking an action (M9).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncActionHandler: Send {
    /// Invokes the action.
    async fn invoke(&mut self, input: InteractionInput) -> CoreResult<InteractionOutput>;
}

/// All handlers registered for a single property affordance.
#[derive(Default)]
struct PropertyHandlerSet {
    read: Option<Box<dyn PropertyReadHandler>>,
    write: Option<Box<dyn PropertyWriteHandler>>,
    observe: Option<Box<dyn PropertyObserveHandler>>,
    #[cfg(feature = "async")]
    async_read: Option<Box<dyn AsyncPropertyReadHandler + Send>>,
    #[cfg(feature = "async")]
    async_write: Option<Box<dyn AsyncPropertyWriteHandler + Send>>,
}

/// All handlers registered for a single event affordance.
#[derive(Default)]
struct EventHandlerSet {
    subscribe: Option<Box<dyn EventSubscribeHandler>>,
    unsubscribe: Option<Box<dyn EventUnsubscribeHandler>>,
}

/// Protocol-neutral local Thing dispatcher.
pub struct LocalThing {
    thing: Thing,
    property_handlers: BTreeMap<String, PropertyHandlerSet>,
    action_handlers: BTreeMap<String, Box<dyn ActionHandler>>,
    #[cfg(feature = "async")]
    async_action_handlers: BTreeMap<String, Box<dyn AsyncActionHandler + Send>>,
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
        let selected = form_set
            .forms
            .iter()
            .find(|candidate| *candidate == form)
            .ok_or_else(|| {
                CoreError::InvalidInteraction(
                    "Selected form does not belong to the requested affordance".into(),
                )
            })?;

        if effective_form_operations(form_set.context, selected).contains(&operation) {
            Ok(())
        } else {
            Err(CoreError::UnsupportedOperation(format!(
                "Form does not support {:?}",
                operation
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
        form: &Form,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.validate_selected_form(&target, operation, form)?;

        let binding = self
            .bindings
            .iter()
            .find(|binding| binding.supports_with_thing(&self.thing, form, operation))
            .ok_or_else(|| {
                CoreError::UnsupportedBinding(format!(
                    "No binding supports {:?} for {}",
                    operation,
                    form.href.as_str()
                ))
            })?;

        binding.invoke(BindingRequest {
            thing: Arc::clone(&self.thing),
            target,
            operation,
            form: Arc::new(form.clone()),
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
            #[cfg(feature = "async")]
            async_action_handlers: BTreeMap::new(),
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
        self.property_handlers.entry(name.into()).or_default().read = Some(Box::new(handler));
    }

    /// Registers a property write handler by affordance name.
    pub fn register_property_write_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyWriteHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().write = Some(Box::new(handler));
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
            .observe = Some(Box::new(handler));
    }

    /// Registers an action handler by affordance name.
    pub fn register_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionHandler + 'static,
    ) -> Option<Box<dyn ActionHandler>> {
        self.action_handlers.insert(name.into(), Box::new(handler))
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
            .async_read = Some(Box::new(handler));
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
            .async_write = Some(Box::new(handler));
    }

    /// Registers an async action handler (M9).
    #[cfg(feature = "async")]
    pub fn register_async_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncActionHandler + 'static,
    ) {
        self.async_action_handlers
            .insert(name.into(), Box::new(handler));
    }

    /// Takes the async read handler out of the set (for async dispatch).
    #[cfg(feature = "async")]
    pub fn take_async_read_handler(
        &mut self,
        name: &str,
    ) -> Option<Box<dyn AsyncPropertyReadHandler + Send>> {
        self.property_handlers
            .get_mut(name)
            .and_then(|set| set.async_read.take())
    }

    /// Returns an async read handler to the set after dispatch.
    #[cfg(feature = "async")]
    pub fn return_async_read_handler(
        &mut self,
        name: &str,
        handler: Box<dyn AsyncPropertyReadHandler + Send>,
    ) {
        if let Some(set) = self.property_handlers.get_mut(name) {
            set.async_read = Some(handler);
        }
    }

    /// Takes the async write handler out of the set (for async dispatch).
    #[cfg(feature = "async")]
    pub fn take_async_write_handler(
        &mut self,
        name: &str,
    ) -> Option<Box<dyn AsyncPropertyWriteHandler + Send>> {
        self.property_handlers
            .get_mut(name)
            .and_then(|set| set.async_write.take())
    }

    /// Returns an async write handler to the set after dispatch.
    #[cfg(feature = "async")]
    pub fn return_async_write_handler(
        &mut self,
        name: &str,
        handler: Box<dyn AsyncPropertyWriteHandler + Send>,
    ) {
        if let Some(set) = self.property_handlers.get_mut(name) {
            set.async_write = Some(handler);
        }
    }

    /// Takes the async action handler out (for async dispatch).
    #[cfg(feature = "async")]
    pub fn take_async_action_handler(
        &mut self,
        name: &str,
    ) -> Option<Box<dyn AsyncActionHandler + Send>> {
        self.async_action_handlers.remove(name)
    }

    /// Returns an async action handler after dispatch.
    #[cfg(feature = "async")]
    pub fn return_async_action_handler(
        &mut self,
        name: &str,
        handler: Box<dyn AsyncActionHandler + Send>,
    ) {
        self.async_action_handlers.insert(name.into(), handler);
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
            .subscribe = Some(Box::new(handler));
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
            .unsubscribe = Some(Box::new(handler));
    }

    fn ensure_property_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("property", name, &self.thing.properties)
    }

    fn ensure_action_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance("action", name, &self.thing.actions)
    }

    fn ensure_event_affordance(&self, name: &str) -> CoreResult<()> {
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
        #[cfg(feature = "async")]
        self.async_action_handlers.remove(name);
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

    fn read_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handlers = self.property_handlers.get_mut(name);
        let handler = handlers
            .and_then(|h| h.read.as_deref_mut())
            .ok_or(CoreError::MissingHandler)?;
        handler.read(input)
    }

    fn write_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handlers = self.property_handlers.get_mut(name);
        let handler = handlers
            .and_then(|h| h.write.as_deref_mut())
            .ok_or(CoreError::MissingHandler)?;
        handler.write(input)
    }

    fn observe_property(
        &mut self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_property_affordance(name)?;
        let handlers = self.property_handlers.get_mut(name);
        let handler = handlers
            .and_then(|h| h.observe.as_deref_mut())
            .ok_or(CoreError::MissingHandler)?;
        handler.observe(input, sink)
    }

    fn invoke_action(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_action_affordance(name)?;
        let handler = self
            .action_handlers
            .get_mut(name)
            .ok_or(CoreError::MissingHandler)?;
        handler.invoke(input)
    }

    fn subscribe_event(
        &mut self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_event_affordance(name)?;
        let handlers = self.event_handlers.get_mut(name);
        let handler = handlers
            .and_then(|h| h.subscribe.as_deref_mut())
            .ok_or(CoreError::MissingHandler)?;
        handler.subscribe(input, sink)
    }

    fn unsubscribe_event(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.ensure_event_affordance(name)?;
        let handlers = self.event_handlers.get_mut(name);
        let handler = handlers
            .and_then(|h| h.unsubscribe.as_deref_mut())
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
    fn read_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Writes a property.
    fn write_property(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Starts observing a property; the handler may emit the initial value
    /// through `sink`.
    fn observe_property(
        &mut self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;

    /// Invokes an action.
    fn invoke_action(
        &mut self,
        name: &str,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;

    /// Subscribes to an event source.
    fn subscribe_event(
        &mut self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> CoreResult<InteractionOutput>;

    /// Unsubscribes from an event source.
    fn unsubscribe_event(
        &mut self,
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
        form: &Form,
        input: InteractionInput,
    ) -> CoreResult<InteractionOutput>;
}
