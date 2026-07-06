//! Affordance addressing, handler model, and concrete exposed/consumed Thing
//! types (baseline v4.0 §4.1–§4.4).
//!
//! The single-impl `ExposedThing` / `ConsumedThing` traits are gone. Core owns
//! two concrete types: [`ExposedThing`] (a produced Thing plus its handler
//! sets, driven by the protocol-neutral dispatcher) and [`ConsumedThing`]
//! (a consumed Thing plus its resolved binding plan). [`LocalThing`] is retained
//! as a produce-time TD affordance builder (audit F9).

use alloc::{collections::BTreeMap, string::String, sync::Arc};

use clinkz_wot_td::{data_type::Operation, thing::Thing};

use crate::{
    CoreError, CoreResult,
    interaction::{InteractionInput, InteractionOutput},
};

// ---------------------------------------------------------------------------
// Affordance addressing (baseline §4.4 — retained from v3.1 §1/§2).
// ---------------------------------------------------------------------------

/// Location of an affordance within a Thing Description.
///
/// Affordance names are stored as [`Arc<str>`] rather than [`String`] so that
/// cloning an `AffordanceTarget` is a single atomic refcount bump. This matters
/// on the inbound and consumed-interaction hot paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AffordanceTarget {
    /// A form declared at Thing level.
    Thing,
    /// A property affordance by name.
    Property(Arc<str>),
    /// An action affordance by name.
    Action(Arc<str>),
    /// An event affordance by name.
    Event(Arc<str>),
}

impl AffordanceTarget {
    /// Returns the affordance name when this target points at a property,
    /// action, or event, or `None` for the Thing-level target.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Thing => None,
            Self::Property(name) | Self::Action(name) | Self::Event(name) => Some(&**name),
        }
    }

    /// Returns the [`AffordanceKind`] discriminant of this target, or `None`
    /// for the Thing-level target.
    pub fn kind(&self) -> Option<AffordanceKind> {
        match self {
            Self::Thing => None,
            Self::Property(_) => Some(AffordanceKind::Property),
            Self::Action(_) => Some(AffordanceKind::Action),
            Self::Event(_) => Some(AffordanceKind::Event),
        }
    }
}

/// Closed discriminant for the three interaction affordance kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AffordanceKind {
    Property,
    Action,
    Event,
}

impl AffordanceKind {
    /// Returns the canonical lowercase kind name (`"property"`, `"action"`,
    /// `"event"`), matching the W3C TD interaction affordance collection names.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Property => "property",
            Self::Action => "action",
            Self::Event => "event",
        }
    }
}

impl core::fmt::Display for AffordanceKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Synchronous handler traits — the primary, zero-allocation inbound path
// (baseline §4.2). Sync handler trait objects are `Send + Sync`.
// ---------------------------------------------------------------------------

/// Ephemeral push callback handed to a streaming (observe/subscribe) handler
/// so it can push initial [`Payload`]s into the stream at establishment.
///
/// This is a closure, not a named trait: the "initial push" is a one-shot,
/// handler-scoped concern, so a `FnMut` callback is the minimal abstraction
/// (no implementor/impl indirection needed). It is distinct from ongoing
/// emission, which uses the Servient-level `emit_event` / `emit_property_change`
/// API (driving [`EventBroker::publish`](crate::EventBroker::publish) directly,
/// available for the lifetime of the exposed Thing — not handler-scoped).
pub type PushFn<'a> = &'a mut dyn FnMut(crate::Payload) -> CoreResult<()>;

/// Handler for reading a local property affordance (Scripting API
/// `setPropertyReadHandler`).
pub trait PropertyReadHandler: Send + Sync {
    /// Reads the current property value.
    fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for writing a local property affordance (Scripting API
/// `setPropertyWriteHandler`). Takes `&mut` so the handler may move the
/// payload out (via `take`) instead of cloning.
pub trait PropertyWriteHandler: Send + Sync {
    /// Writes a new property value.
    fn write(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for observing a local property affordance (Scripting API
/// `setPropertyObserveHandler`). Called once when a remote consumer starts
/// observing; may push the initial value through `push`.
pub trait PropertyObserveHandler: Send + Sync {
    /// Called when observation starts; may push initial values through `push`.
    fn observe(&self, input: &InteractionInput, push: PushFn<'_>) -> CoreResult<InteractionOutput>;
}

/// Handler for unobserving a local property affordance (Scripting API
/// `setPropertyUnobserveHandler`).
pub trait PropertyUnobserveHandler: Send + Sync {
    /// Called when observation stops; allows cleanup of per-observer state.
    fn unobserve(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for invoking a local action affordance (Scripting API
/// `setActionHandler`). Takes `&mut` so the handler may consume the input args.
pub trait ActionHandler: Send + Sync {
    /// Invokes the action.
    fn invoke(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for querying the status of a local action affordance (TD 1.1
/// `queryaction` operation).
pub trait ActionQueryHandler: Send + Sync {
    /// Queries the action status.
    fn query(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for cancelling a local action affordance (TD 1.1 `cancelaction`
/// operation).
pub trait ActionCancelHandler: Send + Sync {
    /// Cancels the ongoing action invocation.
    fn cancel(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
}

/// Handler for event subscription on a local event affordance (Scripting API
/// `setEventSubscribeHandler`).
pub trait EventSubscribeHandler: Send + Sync {
    /// Called when a consumer subscribes; may push initial event payloads.
    fn subscribe(
        &self,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput>;
}

/// Handler for event unsubscription on a local event affordance (Scripting API
/// `setEventUnsubscribeHandler`).
pub trait EventUnsubscribeHandler: Send + Sync {
    /// Called when a consumer unsubscribes; allows cleanup of per-subscriber
    /// state.
    fn unsubscribe(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
}

// ---------------------------------------------------------------------------
// Opt-in async handler twins (all nine operations) — behind `async` feature
// (baseline §4.2 / AD4). For I/O-bound cloud/gateway handlers. The async path
// pays one `async_trait` `Box` per call, acceptable because the handler is
// I/O-bound.
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
mod async_handlers {
    use alloc::boxed::Box;

    use super::{InteractionInput, InteractionOutput, PushFn};
    use crate::CoreResult;

    #[async_trait::async_trait]
    pub trait AsyncPropertyReadHandler: Send + Sync {
        async fn read(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncPropertyWriteHandler: Send + Sync {
        async fn write(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncPropertyObserveHandler: Send + Sync {
        async fn observe(
            &self,
            input: &InteractionInput,
            push: PushFn<'_>,
        ) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncPropertyUnobserveHandler: Send + Sync {
        async fn unobserve(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncActionHandler: Send + Sync {
        async fn invoke(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncActionQueryHandler: Send + Sync {
        async fn query(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncActionCancelHandler: Send + Sync {
        async fn cancel(&self, input: &mut InteractionInput) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncEventSubscribeHandler: Send + Sync {
        async fn subscribe(
            &self,
            input: &InteractionInput,
            push: PushFn<'_>,
        ) -> CoreResult<InteractionOutput>;
    }

    #[async_trait::async_trait]
    pub trait AsyncEventUnsubscribeHandler: Send + Sync {
        async fn unsubscribe(&self, input: &InteractionInput) -> CoreResult<InteractionOutput>;
    }
}

#[cfg(feature = "async")]
pub use async_handlers::{
    AsyncActionCancelHandler, AsyncActionHandler, AsyncActionQueryHandler,
    AsyncEventSubscribeHandler, AsyncEventUnsubscribeHandler, AsyncPropertyObserveHandler,
    AsyncPropertyReadHandler, AsyncPropertyUnobserveHandler, AsyncPropertyWriteHandler,
};

// ---------------------------------------------------------------------------
// Consolidated handler-set storage (baseline §4.2 / AD51). One HandlerSet per
// affordance; each slot is `Option<Sync(..) | Async(..)>`. Engine-internal —
// not part of the public extension surface (AD24).
// ---------------------------------------------------------------------------

/// Slot holding whichever flavor (sync primary / opt-in async twin) was
/// registered for a single property-read operation.
#[derive(Clone)]
pub enum ReadSlot {
    Sync(Arc<dyn PropertyReadHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncPropertyReadHandler>),
}

#[derive(Clone)]
pub enum WriteSlot {
    Sync(Arc<dyn PropertyWriteHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncPropertyWriteHandler>),
}

#[derive(Clone)]
pub enum ObserveSlot {
    Sync(Arc<dyn PropertyObserveHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncPropertyObserveHandler>),
}

#[derive(Clone)]
pub enum UnobserveSlot {
    Sync(Arc<dyn PropertyUnobserveHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncPropertyUnobserveHandler>),
}

#[derive(Clone)]
pub enum InvokeSlot {
    Sync(Arc<dyn ActionHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncActionHandler>),
}

#[derive(Clone)]
pub enum QuerySlot {
    Sync(Arc<dyn ActionQueryHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncActionQueryHandler>),
}

#[derive(Clone)]
pub enum CancelSlot {
    Sync(Arc<dyn ActionCancelHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncActionCancelHandler>),
}

#[derive(Clone)]
pub enum SubscribeSlot {
    Sync(Arc<dyn EventSubscribeHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncEventSubscribeHandler>),
}

#[derive(Clone)]
pub enum UnsubscribeSlot {
    Sync(Arc<dyn EventUnsubscribeHandler>),
    #[cfg(feature = "async")]
    Async(Arc<dyn AsyncEventUnsubscribeHandler>),
}

#[derive(Clone, Default)]
pub(crate) struct PropertyHandlerSet {
    pub(crate) read: Option<ReadSlot>,
    pub(crate) write: Option<WriteSlot>,
    pub(crate) observe: Option<ObserveSlot>,
    pub(crate) unobserve: Option<UnobserveSlot>,
}

#[derive(Clone, Default)]
pub(crate) struct ActionHandlerSet {
    pub(crate) invoke: Option<InvokeSlot>,
    pub(crate) query: Option<QuerySlot>,
    pub(crate) cancel: Option<CancelSlot>,
}

#[derive(Clone, Default)]
pub(crate) struct EventHandlerSet {
    pub(crate) subscribe: Option<SubscribeSlot>,
    pub(crate) unsubscribe: Option<UnsubscribeSlot>,
}

// ---------------------------------------------------------------------------
// LocalThing — produce-time TD affordance builder (audit F9). The TD affordance
// set is frozen at expose(); pre-expose mutation (produce → mutate → expose)
// legitimately needs these primitives. They are not reachable from an exposed
// handle post-expose.
// ---------------------------------------------------------------------------

/// A local Thing Description plus affordance-mutation primitives used between
/// `produce` and `expose` (audit F9). Once exposed, the affordance set is frozen
/// (decision 2).
pub struct LocalThing {
    pub(crate) thing: Thing,
}

impl LocalThing {
    /// Creates a local TD builder for a Thing Description.
    pub fn new(thing: Thing) -> Self {
        Self { thing }
    }

    /// Returns the Thing Description.
    pub fn thing_description(&self) -> &Thing {
        &self.thing
    }

    /// Returns `Err(UnknownAffordance)` if no property with `name` is declared.
    pub fn ensure_property_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance(AffordanceKind::Property, name, &self.thing.properties)
    }

    /// Returns `Err(UnknownAffordance)` if no action with `name` is declared.
    pub fn ensure_action_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance(AffordanceKind::Action, name, &self.thing.actions)
    }

    /// Returns `Err(UnknownAffordance)` if no event with `name` is declared.
    pub fn ensure_event_affordance(&self, name: &str) -> CoreResult<()> {
        ensure_affordance(AffordanceKind::Event, name, &self.thing.events)
    }
}

fn ensure_affordance<T>(
    kind: AffordanceKind,
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

/// Builds a [`CoreError::MissingHandler`] carrying the offending target and
/// operation so diagnostics (e.g. HTTP 501 response bodies) are actionable.
fn missing_handler(target: AffordanceTarget, operation: Operation) -> CoreError {
    CoreError::MissingHandler { target, operation }
}

// ---------------------------------------------------------------------------
// ExposedThing — produced Thing plus handler sets (baseline §4.1). Lives
// in core so the protocol-neutral dispatcher can drive it. Handlers may be
// attached or replaced throughout the exposed lifetime (AD14); an unwired
// affordance yields `MissingHandler`.
// ---------------------------------------------------------------------------

/// A produced Thing plus its per-affordance handler sets.
///
/// Holds a [`LocalThing`] (the mutable TD + affordance-mutation primitives) and
/// three handler-set maps (property/action/event). Dispatch clones a handler
/// `Arc` out of the relevant set and invokes it outside any lock.
pub struct ExposedThing {
    pub(crate) local: LocalThing,
    pub(crate) property_handlers: BTreeMap<String, PropertyHandlerSet>,
    pub(crate) action_handlers: BTreeMap<String, ActionHandlerSet>,
    pub(crate) event_handlers: BTreeMap<String, EventHandlerSet>,
}

impl ExposedThing {
    /// Creates an exposed Thing from a Thing Description with no handlers
    /// attached.
    pub fn new(thing: Thing) -> Self {
        Self {
            local: LocalThing::new(thing),
            property_handlers: BTreeMap::new(),
            action_handlers: BTreeMap::new(),
            event_handlers: BTreeMap::new(),
        }
    }

    /// Returns the Thing Description.
    pub fn thing_description(&self) -> &Thing {
        self.local.thing_description()
    }

    // --- property handler registration ------------------------------------

    /// Sets the property read handler for `name` (replaces any prior handler).
    pub fn set_property_read_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyReadHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().read =
            Some(ReadSlot::Sync(Arc::new(handler)));
    }

    /// Sets the property write handler for `name`.
    pub fn set_property_write_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyWriteHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().write =
            Some(WriteSlot::Sync(Arc::new(handler)));
    }

    /// Sets the property observe handler for `name`.
    pub fn set_property_observe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyObserveHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .observe = Some(ObserveSlot::Sync(Arc::new(handler)));
    }

    /// Sets the property unobserve handler for `name`.
    pub fn set_property_unobserve_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl PropertyUnobserveHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .unobserve = Some(UnobserveSlot::Sync(Arc::new(handler)));
    }

    // --- action handler registration --------------------------------------

    /// Sets the action invoke handler for `name`.
    pub fn set_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().invoke =
            Some(InvokeSlot::Sync(Arc::new(handler)));
    }

    /// Sets the action query handler for `name` (TD 1.1 `queryaction`).
    pub fn set_action_query_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionQueryHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().query =
            Some(QuerySlot::Sync(Arc::new(handler)));
    }

    /// Sets the action cancel handler for `name` (TD 1.1 `cancelaction`).
    pub fn set_action_cancel_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl ActionCancelHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().cancel =
            Some(CancelSlot::Sync(Arc::new(handler)));
    }

    // --- event handler registration ---------------------------------------

    /// Sets the event subscribe handler for `name`.
    pub fn set_event_subscribe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl EventSubscribeHandler + 'static,
    ) {
        self.event_handlers
            .entry(name.into())
            .or_default()
            .subscribe = Some(SubscribeSlot::Sync(Arc::new(handler)));
    }

    /// Sets the event unsubscribe handler for `name`.
    pub fn set_event_unsubscribe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl EventUnsubscribeHandler + 'static,
    ) {
        self.event_handlers
            .entry(name.into())
            .or_default()
            .unsubscribe = Some(UnsubscribeSlot::Sync(Arc::new(handler)));
    }

    // --- async handler registration (behind `async` feature) --------------

    #[cfg(feature = "async")]
    pub fn set_async_property_read_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncPropertyReadHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().read =
            Some(ReadSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_property_write_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncPropertyWriteHandler + 'static,
    ) {
        self.property_handlers.entry(name.into()).or_default().write =
            Some(WriteSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_action_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncActionHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().invoke =
            Some(InvokeSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_property_observe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncPropertyObserveHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .observe = Some(ObserveSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_property_unobserve_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncPropertyUnobserveHandler + 'static,
    ) {
        self.property_handlers
            .entry(name.into())
            .or_default()
            .unobserve = Some(UnobserveSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_action_query_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncActionQueryHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().query =
            Some(QuerySlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_action_cancel_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncActionCancelHandler + 'static,
    ) {
        self.action_handlers.entry(name.into()).or_default().cancel =
            Some(CancelSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_event_subscribe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncEventSubscribeHandler + 'static,
    ) {
        self.event_handlers
            .entry(name.into())
            .or_default()
            .subscribe = Some(SubscribeSlot::Async(Arc::new(handler)));
    }

    #[cfg(feature = "async")]
    pub fn set_async_event_unsubscribe_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl AsyncEventUnsubscribeHandler + 'static,
    ) {
        self.event_handlers
            .entry(name.into())
            .or_default()
            .unsubscribe = Some(UnsubscribeSlot::Async(Arc::new(handler)));
    }

    // --- synchronous dispatch (primary inbound path for sync handlers) ----

    /// Reads a property via its registered read handler.
    pub fn read_property(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_property_affordance(name)?;
        let slot = self.property_handler_read(name).ok_or(missing_handler(
            AffordanceTarget::Property(name.into()),
            Operation::ReadProperty,
        ))?;
        match slot {
            ReadSlot::Sync(handler) => handler.read(input),
            #[cfg(feature = "async")]
            ReadSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Property(name.into()),
                Operation::ReadProperty,
            )),
        }
    }

    /// Writes a property via its registered write handler.
    pub fn write_property(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_property_affordance(name)?;
        let slot = self.property_handler_write(name).ok_or(missing_handler(
            AffordanceTarget::Property(name.into()),
            Operation::WriteProperty,
        ))?;
        match slot {
            WriteSlot::Sync(handler) => handler.write(input),
            #[cfg(feature = "async")]
            WriteSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Property(name.into()),
                Operation::WriteProperty,
            )),
        }
    }

    /// Starts observing a property via its registered observe handler.
    pub fn observe_property(
        &self,
        name: &str,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_property_affordance(name)?;
        let slot = self.property_handler_observe(name).ok_or(missing_handler(
            AffordanceTarget::Property(name.into()),
            Operation::ObserveProperty,
        ))?;
        match slot {
            ObserveSlot::Sync(handler) => handler.observe(input, push),
            #[cfg(feature = "async")]
            ObserveSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Property(name.into()),
                Operation::ObserveProperty,
            )),
        }
    }

    /// Stops observing a property via its registered unobserve handler.
    pub fn unobserve_property(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_property_affordance(name)?;
        let slot = self
            .property_handler_unobserve(name)
            .ok_or(missing_handler(
                AffordanceTarget::Property(name.into()),
                Operation::UnobserveProperty,
            ))?;
        match slot {
            UnobserveSlot::Sync(handler) => handler.unobserve(input),
            #[cfg(feature = "async")]
            UnobserveSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Property(name.into()),
                Operation::UnobserveProperty,
            )),
        }
    }

    /// Invokes an action via its registered invoke handler.
    pub fn invoke_action(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_action_affordance(name)?;
        let slot = self.action_handler_invoke(name).ok_or(missing_handler(
            AffordanceTarget::Action(name.into()),
            Operation::InvokeAction,
        ))?;
        match slot {
            InvokeSlot::Sync(handler) => handler.invoke(input),
            #[cfg(feature = "async")]
            InvokeSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Action(name.into()),
                Operation::InvokeAction,
            )),
        }
    }

    /// Queries an action via its registered query handler.
    pub fn query_action(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_action_affordance(name)?;
        let slot = self.action_handler_query(name).ok_or(missing_handler(
            AffordanceTarget::Action(name.into()),
            Operation::QueryAction,
        ))?;
        match slot {
            QuerySlot::Sync(handler) => handler.query(input),
            #[cfg(feature = "async")]
            QuerySlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Action(name.into()),
                Operation::QueryAction,
            )),
        }
    }

    /// Cancels an action via its registered cancel handler.
    pub fn cancel_action(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_action_affordance(name)?;
        let slot = self.action_handler_cancel(name).ok_or(missing_handler(
            AffordanceTarget::Action(name.into()),
            Operation::CancelAction,
        ))?;
        match slot {
            CancelSlot::Sync(handler) => handler.cancel(input),
            #[cfg(feature = "async")]
            CancelSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Action(name.into()),
                Operation::CancelAction,
            )),
        }
    }

    /// Subscribes to an event via its registered subscribe handler.
    pub fn subscribe_event(
        &self,
        name: &str,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_event_affordance(name)?;
        let slot = self.event_handler_subscribe(name).ok_or(missing_handler(
            AffordanceTarget::Event(name.into()),
            Operation::SubscribeEvent,
        ))?;
        match slot {
            SubscribeSlot::Sync(handler) => handler.subscribe(input, push),
            #[cfg(feature = "async")]
            SubscribeSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Event(name.into()),
                Operation::SubscribeEvent,
            )),
        }
    }

    /// Unsubscribes from an event via its registered unsubscribe handler.
    pub fn unsubscribe_event(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.local.ensure_event_affordance(name)?;
        let slot = self.event_handler_unsubscribe(name).ok_or(missing_handler(
            AffordanceTarget::Event(name.into()),
            Operation::UnsubscribeEvent,
        ))?;
        match slot {
            UnsubscribeSlot::Sync(handler) => handler.unsubscribe(input),
            #[cfg(feature = "async")]
            UnsubscribeSlot::Async(_) => Err(missing_handler(
                AffordanceTarget::Event(name.into()),
                Operation::UnsubscribeEvent,
            )),
        }
    }

    // --- handler-lookup (clone slot out for dispatch) ---------------------

    pub fn property_handler_read(&self, name: &str) -> Option<ReadSlot> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.read.clone())
    }

    pub fn property_handler_write(&self, name: &str) -> Option<WriteSlot> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.write.clone())
    }

    pub fn property_handler_observe(&self, name: &str) -> Option<ObserveSlot> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.observe.clone())
    }

    pub fn property_handler_unobserve(&self, name: &str) -> Option<UnobserveSlot> {
        self.property_handlers
            .get(name)
            .and_then(|set| set.unobserve.clone())
    }

    pub fn action_handler_invoke(&self, name: &str) -> Option<InvokeSlot> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.invoke.clone())
    }

    pub fn action_handler_query(&self, name: &str) -> Option<QuerySlot> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.query.clone())
    }

    pub fn action_handler_cancel(&self, name: &str) -> Option<CancelSlot> {
        self.action_handlers
            .get(name)
            .and_then(|set| set.cancel.clone())
    }

    pub fn event_handler_subscribe(&self, name: &str) -> Option<SubscribeSlot> {
        self.event_handlers
            .get(name)
            .and_then(|set| set.subscribe.clone())
    }

    pub fn event_handler_unsubscribe(&self, name: &str) -> Option<UnsubscribeSlot> {
        self.event_handlers
            .get(name)
            .and_then(|set| set.unsubscribe.clone())
    }
}

#[cfg(feature = "async")]
mod async_dispatch {
    use super::*;

    impl ExposedThing {
        /// Async read dispatch: awaits an async read handler, or runs a sync
        /// read handler inline.
        pub async fn read_property_async(
            &self,
            name: &str,
            input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_property_affordance(name)?;
            match self.property_handler_read(name) {
                Some(ReadSlot::Sync(handler)) => handler.read(input),
                Some(ReadSlot::Async(handler)) => handler.read(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Property(name.into()),
                    Operation::ReadProperty,
                )),
            }
        }

        /// Async write dispatch.
        pub async fn write_property_async(
            &self,
            name: &str,
            input: &mut InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_property_affordance(name)?;
            match self.property_handler_write(name) {
                Some(WriteSlot::Sync(handler)) => handler.write(input),
                Some(WriteSlot::Async(handler)) => handler.write(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Property(name.into()),
                    Operation::WriteProperty,
                )),
            }
        }

        /// Async action invoke dispatch.
        pub async fn invoke_action_async(
            &self,
            name: &str,
            input: &mut InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_action_affordance(name)?;
            match self.action_handler_invoke(name) {
                Some(InvokeSlot::Sync(handler)) => handler.invoke(input),
                Some(InvokeSlot::Async(handler)) => handler.invoke(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Action(name.into()),
                    Operation::InvokeAction,
                )),
            }
        }

        /// Async property observe dispatch.
        pub async fn observe_property_async(
            &self,
            name: &str,
            input: &InteractionInput,
            push: PushFn<'_>,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_property_affordance(name)?;
            match self.property_handler_observe(name) {
                Some(ObserveSlot::Sync(handler)) => handler.observe(input, push),
                Some(ObserveSlot::Async(handler)) => handler.observe(input, push).await,
                None => Err(missing_handler(
                    AffordanceTarget::Property(name.into()),
                    Operation::ObserveProperty,
                )),
            }
        }

        /// Async property unobserve dispatch.
        pub async fn unobserve_property_async(
            &self,
            name: &str,
            input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_property_affordance(name)?;
            match self.property_handler_unobserve(name) {
                Some(UnobserveSlot::Sync(handler)) => handler.unobserve(input),
                Some(UnobserveSlot::Async(handler)) => handler.unobserve(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Property(name.into()),
                    Operation::UnobserveProperty,
                )),
            }
        }

        /// Async action query dispatch.
        pub async fn query_action_async(
            &self,
            name: &str,
            input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_action_affordance(name)?;
            match self.action_handler_query(name) {
                Some(QuerySlot::Sync(handler)) => handler.query(input),
                Some(QuerySlot::Async(handler)) => handler.query(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Action(name.into()),
                    Operation::QueryAction,
                )),
            }
        }

        /// Async action cancel dispatch.
        pub async fn cancel_action_async(
            &self,
            name: &str,
            input: &mut InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_action_affordance(name)?;
            match self.action_handler_cancel(name) {
                Some(CancelSlot::Sync(handler)) => handler.cancel(input),
                Some(CancelSlot::Async(handler)) => handler.cancel(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Action(name.into()),
                    Operation::CancelAction,
                )),
            }
        }

        /// Async event subscribe dispatch.
        pub async fn subscribe_event_async(
            &self,
            name: &str,
            input: &InteractionInput,
            push: PushFn<'_>,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_event_affordance(name)?;
            match self.event_handler_subscribe(name) {
                Some(SubscribeSlot::Sync(handler)) => handler.subscribe(input, push),
                Some(SubscribeSlot::Async(handler)) => handler.subscribe(input, push).await,
                None => Err(missing_handler(
                    AffordanceTarget::Event(name.into()),
                    Operation::SubscribeEvent,
                )),
            }
        }

        /// Async event unsubscribe dispatch.
        pub async fn unsubscribe_event_async(
            &self,
            name: &str,
            input: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.local.ensure_event_affordance(name)?;
            match self.event_handler_unsubscribe(name) {
                Some(UnsubscribeSlot::Sync(handler)) => handler.unsubscribe(input),
                Some(UnsubscribeSlot::Async(handler)) => handler.unsubscribe(input).await,
                None => Err(missing_handler(
                    AffordanceTarget::Event(name.into()),
                    Operation::UnsubscribeEvent,
                )),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ConsumedThing — consumed Thing plus resolved binding plan (baseline
// §4.1). The consumed (outbound) path is network-bound and therefore async
// (resolved A1): ClientBinding::invoke is `async fn`, so the consumed dispatch
// is gated behind the `async` feature.
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
mod consumed {
    use alloc::{boxed::Box, collections::BTreeMap, format, string::String, sync::Arc, vec::Vec};

    use clinkz_wot_td::td_defaults::{FormContext, effective_form_operations};
    use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

    use crate::interaction::{InteractionInput, InteractionOutput};
    use crate::{
        AffordanceKind, AffordanceTarget, BindingRequest, ClientBinding, CoreError, CoreResult,
        Subscription, SubscriptionGuard,
    };

    /// A consumed Thing plus its registered client bindings.
    ///
    /// Dispatch resolves the selected form against the affordance, picks a
    /// binding whose [`ClientBinding::supports`] holds, and drives
    /// [`ClientBinding::invoke`] asynchronously.
    pub struct ConsumedThing {
        pub(crate) thing: Arc<Thing>,
        pub(crate) bindings: Vec<Box<dyn ClientBinding>>,
    }

    impl ConsumedThing {
        /// Creates a consumed Thing dispatcher for a Thing Description.
        pub fn new(thing: Thing) -> Self {
            Self {
                thing: Arc::new(thing),
                bindings: Vec::new(),
            }
        }

        /// Creates a consumed Thing dispatcher from an already-shared `Arc<Thing>`.
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

        /// Performs an operation against a caller-selected affordance form.
        ///
        /// The form must belong to the target affordance and support the
        /// operation; otherwise an `InvalidInteraction` / `UnsupportedOperation`
        /// error is returned before any binding is consulted.
        pub async fn request(
            &self,
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

            binding
                .invoke(BindingRequest {
                    thing: Arc::clone(&self.thing),
                    target,
                    operation,
                    form: Arc::clone(&form),
                    input,
                })
                .await
        }

        /// Opens a long-lived streaming subscription against a caller-selected
        /// affordance form (P2).
        ///
        /// Mirrors [`Self::request`] but drives
        /// [`ClientBinding::subscribe`] instead of [`ClientBinding::invoke`].
        /// Used by `observe_property`, `subscribe_event`, and their `*_all`
        /// fan-outs on `ConsumedThingHandle`. Form validation and binding
        /// selection are identical to `request`.
        ///
        /// Returns the consumer-side [`Subscription`] (for draining pushed
        /// samples) and the protocol-specific [`SubscriptionGuard`] (for
        /// wire-side cleanup). The caller is responsible for keeping the
        /// guard alive for the desired subscription lifetime; dropping it
        /// releases the wire-side resources.
        pub async fn subscribe(
            &self,
            target: AffordanceTarget,
            operation: Operation,
            form: Arc<Form>,
            input: InteractionInput,
        ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
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

            binding
                .subscribe(BindingRequest {
                    thing: Arc::clone(&self.thing),
                    target,
                    operation,
                    form: Arc::clone(&form),
                    input,
                })
                .await
        }

        fn validate_selected_form(
            &self,
            target: &AffordanceTarget,
            operation: Operation,
            form: &Form,
        ) -> CoreResult<()> {
            let form_set = self.forms_for_target(target)?;

            // The public entry point accepts a caller-supplied `Arc<Form>`;
            // verify the form actually belongs to this affordance, otherwise a
            // foreign form (or a no-`op` form) could pass by falling back to the
            // affordance's default operations and dispatch to the wrong binding.
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

        fn forms_for_target(&self, target: &AffordanceTarget) -> CoreResult<FormSet<'_>> {
            match target {
                AffordanceTarget::Thing => Ok(FormSet {
                    context: FormContext::Thing,
                    forms: self.thing.forms.as_deref().unwrap_or(&[]),
                }),
                AffordanceTarget::Property(name) => {
                    let property =
                        find_affordance(AffordanceKind::Property, name, &self.thing.properties)?;
                    Ok(FormSet {
                        context: FormContext::Property(property),
                        forms: property._interaction.forms.as_slice(),
                    })
                }
                AffordanceTarget::Action(name) => {
                    let action =
                        find_affordance(AffordanceKind::Action, name, &self.thing.actions)?;
                    Ok(FormSet {
                        context: FormContext::Action(action),
                        forms: action._interaction.forms.as_slice(),
                    })
                }
                AffordanceTarget::Event(name) => {
                    let event = find_affordance(AffordanceKind::Event, name, &self.thing.events)?;
                    Ok(FormSet {
                        context: FormContext::Event(event),
                        forms: event._interaction.forms.as_slice(),
                    })
                }
            }
        }
    }

    struct FormSet<'a> {
        context: FormContext<'a>,
        forms: &'a [Form],
    }

    fn find_affordance<'a, T>(
        kind: AffordanceKind,
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
}

#[cfg(feature = "async")]
pub use consumed::ConsumedThing;
