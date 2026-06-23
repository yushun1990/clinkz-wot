//! Typed interaction handles (baseline v3.0 §6 / addendum §4).
//!
//! [`Servient::expose`](crate::Servient::expose) returns an
//! [`ExposedThingHandle`] and [`Servient::consume`](crate::Servient::consume)
//! returns a [`ConsumedThingHandle`]. Both hold a cheap [`Servient`](crate::Servient)
//! clone plus the relevant identity/TD, are [`Clone`], and expose the WoT
//! Scripting API interaction surface (`read_property` / `write_property` /
//! `invoke_action` / `subscribe_event`).
//!
//! Local in-process interactions go directly to the handler without form
//! selection or transport security (baseline §6). The
//! [`ExposedThingHandle`] dispatches through its own `Arc` clone of the
//! exposed-Thing registry, bypassing the outer Servient `StateLock`, so a
//! handler calling `destroy(own_id)` does not self-deadlock (baseline §7).
//! Remote interactions select a form, apply transport security, and invoke a
//! binding through shared runtime registries snapshot from the Servient.

use alloc::{collections::BTreeMap, string::String, sync::Arc};

use clinkz_wot_core::{
    EventBroker, EventName, EventSink, ExposedThing, InteractionInput, InteractionOutput, Payload,
    Subscription, ThingId,
};
use clinkz_wot_protocol_bindings::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_td::{data_type::Operation, thing::Thing};

use crate::{
    InMemoryExposedThingRegistry, ServientResult, consumed::ConsumedThingEntry,
    consumed::SubscriptionKey, servient::Servient,
};

/// Handle for a locally exposed Thing (baseline v3.0 §6 / addendum §4).
///
/// Returned by [`Servient::expose`](crate::Servient::expose). Holds a cheap
/// `Servient` clone, an `Arc` clone of the exposed-Thing registry, and the
/// exposed Thing identity. Local interactions dispatch directly to the attached
/// handler through the registry's two-level locking (baseline §7); handlers are
/// attached after `expose` through
/// [`set_property_handler`](Self::set_property_handler) and friends.
pub struct ExposedThingHandle<D> {
    servient: Servient<D>,
    registry: Arc<InMemoryExposedThingRegistry>,
    event_broker: EventBroker,
    id: String,
}

impl<D> core::fmt::Debug for ExposedThingHandle<D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExposedThingHandle")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<D> Clone for ExposedThingHandle<D> {
    fn clone(&self) -> Self {
        Self {
            servient: self.servient.clone(),
            registry: Arc::clone(&self.registry),
            event_broker: self.event_broker.clone(),
            id: self.id.clone(),
        }
    }
}

impl<D> ExposedThingHandle<D> {
    pub(crate) fn new(
        servient: Servient<D>,
        registry: Arc<InMemoryExposedThingRegistry>,
        event_broker: EventBroker,
        id: String,
    ) -> Self {
        Self {
            servient,
            registry,
            event_broker,
            id,
        }
    }

    /// Returns the exposed Thing identity.
    pub fn thing_id(&self) -> &str {
        &self.id
    }
}

impl<D> ExposedThingHandle<D>
where
    D: clinkz_wot_discovery::ThingDirectory,
{
    /// Reads a property, dispatching directly to the attached handler.
    pub fn read_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.registry
            .dispatch(&self.id, |thing| thing.read_property(name, input))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Writes a property, dispatching directly to the attached handler.
    pub fn write_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.registry
            .dispatch(&self.id, |thing| thing.write_property(name, input))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Reads multiple properties in one call (W3C Scripting API
    /// `readMultipleProperties`).
    ///
    /// Each property is read individually; partial failures return the first
    /// error.
    pub fn read_multiple_properties(
        &self,
        names: &[&str],
    ) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        let mut results = BTreeMap::new();
        for &name in names {
            let output = self.read_property(name, InteractionInput::empty())?;
            results.insert(name.into(), output);
        }
        Ok(results)
    }

    /// Reads all properties declared in the TD (W3C Scripting API
    /// `readAllProperties`).
    pub fn read_all_properties(&self) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        let td = self.thing_description()?;
        let mut results = BTreeMap::new();
        if let Some(properties) = td.properties.as_ref() {
            for name in properties.keys() {
                let output = self.read_property(name, InteractionInput::empty())?;
                results.insert(name.clone(), output);
            }
        }
        Ok(results)
    }

    /// Writes multiple properties in one call (W3C Scripting API
    /// `writeMultipleProperties`).
    pub fn write_multiple_properties(
        &self,
        values: &BTreeMap<String, InteractionInput>,
    ) -> ServientResult<()> {
        for (name, input) in values {
            self.write_property(name, input.clone())?;
        }
        Ok(())
    }

    /// Invokes an action, dispatching directly to the attached handler.
    pub fn invoke_action(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.registry
            .dispatch(&self.id, |thing| thing.invoke_action(name, input))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Subscribes to an event, dispatching directly to the attached handler.
    pub fn subscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> ServientResult<InteractionOutput> {
        self.registry
            .dispatch(&self.id, |thing| thing.subscribe_event(name, input, sink))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Emits an event payload to all subscribers (W3C Scripting API
    /// `emitEvent`).
    ///
    /// The payload is fanned out through the [`EventBroker`] to every
    /// registered [`PublisherSink`](clinkz_wot_core::PublisherSink), each of
    /// which publishes to its remote subscriber via the server binding.
    /// Publishing to a Thing or event with no subscribers is a no-op.
    pub fn emit_event(&self, name: &str, payload: Payload) -> ServientResult<()> {
        self.event_broker
            .publish(
                &ThingId::from(self.id.as_str()),
                &EventName::from(name),
                &payload,
            )
            .map_err(Into::into)
    }

    /// Emits a property change notification to all observers of the named
    /// property (W3C Scripting API convenience alias).
    ///
    /// Observable properties share the same [`EventBroker`] fan-out path as
    /// events: the binding registers a `PublisherSink` under the property
    /// name during `register_thing`. This method is therefore equivalent to
    /// [`emit_event`](Self::emit_event) with the property name.
    pub fn emit_property_change(&self, name: &str, payload: Payload) -> ServientResult<()> {
        self.emit_event(name, payload)
    }

    // -----------------------------------------------------------------------
    // Handler attachment (addendum §4 / W3C Scripting API §7).
    // -----------------------------------------------------------------------

    /// Attaches a property read handler (W3C Scripting API
    /// `setPropertyReadHandler`).
    pub fn set_property_read_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::PropertyReadHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_property_read_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches a property write handler (W3C Scripting API
    /// `setPropertyWriteHandler`).
    pub fn set_property_write_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::PropertyWriteHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_property_write_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an async property read handler (M9, behind `async` feature).
    ///
    /// When registered, the async driving loop calls this handler instead of
    /// the sync [`PropertyReadHandler`](clinkz_wot_core::PropertyReadHandler),
    /// allowing async I/O without blocking the driving loop.
    #[cfg(feature = "async")]
    pub fn set_async_property_read_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::AsyncPropertyReadHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_async_property_read_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an async property write handler (M9).
    #[cfg(feature = "async")]
    pub fn set_async_property_write_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::AsyncPropertyWriteHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_async_property_write_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an async action handler (M9).
    #[cfg(feature = "async")]
    pub fn set_async_action_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::AsyncActionHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_async_action_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches a property observe handler (W3C Scripting API
    /// `setPropertyObserveHandler`).
    pub fn set_property_observe_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::PropertyObserveHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_property_observe_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an action handler.
    pub fn set_action_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::ActionHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_action_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an event subscribe handler (W3C Scripting API
    /// `setEventSubscribeHandler`).
    pub fn set_event_subscribe_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::EventSubscribeHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_event_subscribe_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an event unsubscribe handler (W3C Scripting API
    /// `setEventUnsubscribeHandler`).
    pub fn set_event_unsubscribe_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::EventUnsubscribeHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_event_unsubscribe_handler(name, handler)
        });
        Ok(())
    }

    /// Returns the exposed Thing Description.
    pub fn thing_description(&self) -> ServientResult<Thing> {
        self.registry
            .thing_description(&self.id)
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))
    }

    /// Adds a property affordance to the exposed Thing at runtime (W3C
    /// Scripting API `addProperty`).
    pub fn add_property(
        &self,
        name: impl Into<String>,
        property: clinkz_wot_td::affordance::PropertyAffordance,
    ) -> ServientResult<()> {
        self.registry
            .mutate(&self.id, |thing| thing.add_property(name, property))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Removes a property affordance from the exposed Thing.
    pub fn remove_property(&self, name: &str) -> ServientResult<()> {
        self.registry
            .mutate(&self.id, |thing| thing.remove_property(name));
        Ok(())
    }

    /// Adds an action affordance to the exposed Thing at runtime.
    pub fn add_action(
        &self,
        name: impl Into<String>,
        action: clinkz_wot_td::affordance::ActionAffordance,
    ) -> ServientResult<()> {
        self.registry
            .mutate(&self.id, |thing| thing.add_action(name, action))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Removes an action affordance from the exposed Thing.
    pub fn remove_action(&self, name: &str) -> ServientResult<()> {
        self.registry
            .mutate(&self.id, |thing| thing.remove_action(name));
        Ok(())
    }

    /// Adds an event affordance to the exposed Thing at runtime.
    pub fn add_event(
        &self,
        name: impl Into<String>,
        event: clinkz_wot_td::affordance::EventAffordance,
    ) -> ServientResult<()> {
        self.registry
            .mutate(&self.id, |thing| thing.add_event(name, event))
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.clone()))?
            .map_err(Into::into)
    }

    /// Removes an event affordance from the exposed Thing.
    pub fn remove_event(&self, name: &str) -> ServientResult<()> {
        self.registry
            .mutate(&self.id, |thing| thing.remove_event(name));
        Ok(())
    }
}

/// Handle for a consumed (remote) Thing (baseline v3.0 §5.1 / §6).
///
/// Returned by [`Servient::consume`](crate::Servient::consume). Holds a cheap
/// `Servient` clone, an `Arc` clone of the consumed-Thing registry, the Thing
/// identity, and an `Arc` clone of the interned entry. Multiple `consume()`
/// calls for the same Thing id share one canonical live entry, so form
/// selections and binding plans are computed once and reused (baseline §5.1
/// identity interning).
///
/// Remote interactions select a form, apply transport security, and invoke a
/// protocol binding through the interned entry's caches.
pub struct ConsumedThingHandle<D> {
    servient: Servient<D>,
    id: String,
    entry: Arc<ConsumedThingEntry>,
}

impl<D> core::fmt::Debug for ConsumedThingHandle<D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConsumedThingHandle")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<D> Clone for ConsumedThingHandle<D> {
    fn clone(&self) -> Self {
        Self {
            servient: self.servient.clone(),
            id: self.id.clone(),
            entry: Arc::clone(&self.entry),
        }
    }
}

impl<D> ConsumedThingHandle<D> {
    pub(crate) fn new(servient: Servient<D>, id: String, entry: Arc<ConsumedThingEntry>) -> Self {
        Self {
            servient,
            id,
            entry,
        }
    }

    /// Returns the consumed Thing identity.
    pub fn thing_id(&self) -> &str {
        &self.id
    }

    /// Returns the consumed Thing Description.
    pub fn thing_description(&self) -> &Thing {
        self.entry.thing()
    }
}

impl<D> ConsumedThingHandle<D>
where
    D: clinkz_wot_discovery::ThingDirectory,
{
    /// Reads a remote property, selecting a form by default criteria.
    pub fn read_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.read_property_with_criteria(
            name,
            FormSelectionCriteria::new(Operation::ReadProperty),
            input,
        )
    }

    /// Reads a remote property, selecting a form by explicit criteria.
    pub fn read_property_with_criteria(
        &self,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient.consumed_request(
            &self.entry,
            &self.id,
            clinkz_wot_core::AffordanceTarget::Property(name.into()),
            AffordanceRef::Property(name),
            criteria,
            input,
        )
    }

    /// Writes a remote property, selecting a form by default criteria.
    pub fn write_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.write_property_with_criteria(
            name,
            FormSelectionCriteria::new(Operation::WriteProperty),
            input,
        )
    }

    /// Writes a remote property, selecting a form by explicit criteria.
    pub fn write_property_with_criteria(
        &self,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient.consumed_request(
            &self.entry,
            &self.id,
            clinkz_wot_core::AffordanceTarget::Property(name.into()),
            AffordanceRef::Property(name),
            criteria_for_operation(criteria, Operation::WriteProperty),
            input,
        )
    }

    /// Invokes a remote action, selecting a form by default criteria.
    pub fn invoke_action(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.invoke_action_with_criteria(
            name,
            FormSelectionCriteria::new(Operation::InvokeAction),
            input,
        )
    }

    /// Invokes a remote action, selecting a form by explicit criteria.
    pub fn invoke_action_with_criteria(
        &self,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient.consumed_request(
            &self.entry,
            &self.id,
            clinkz_wot_core::AffordanceTarget::Action(name.into()),
            AffordanceRef::Action(name),
            criteria_for_operation(criteria, Operation::InvokeAction),
            input,
        )
    }

    /// Subscribes to a remote event, opening a long-lived streaming
    /// subscription.
    ///
    /// Returns a [`Subscription`] for draining event payloads pushed by the
    /// remote Thing. The caller should poll the subscription via
    /// [`Subscription::poll_next`] and call
    /// [`unsubscribe_event`](Self::unsubscribe_event) when done to release wire
    /// resources.
    pub fn subscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.subscribe_event_with_criteria(
            name,
            FormSelectionCriteria::new(Operation::SubscribeEvent),
            input,
        )
    }

    /// Subscribes to a remote event with explicit form selection criteria.
    pub fn subscribe_event_with_criteria(
        &self,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.servient.consumed_subscribe(
            &self.entry,
            &self.id,
            clinkz_wot_core::AffordanceTarget::Event(name.into()),
            AffordanceRef::Event(name),
            criteria_for_operation(criteria, Operation::SubscribeEvent),
            input,
        )
    }

    /// Stops all active streaming subscriptions for the named event and
    /// releases wire resources (W3C Scripting API `unsubscribeEvent`).
    pub fn unsubscribe_event(&self, name: &str) {
        let key = SubscriptionKey::new(
            &clinkz_wot_core::AffordanceTarget::Event(name.into()),
            Operation::SubscribeEvent.as_str(),
        );
        self.entry.stop_subscriptions(&key);
    }

    /// Observes a remote property, opening a long-lived streaming subscription
    /// for property value changes (W3C Scripting API `observeProperty`).
    ///
    /// Returns a [`Subscription`] for draining property change payloads pushed
    /// by the remote Thing. Call
    /// [`unobserve_property`](Self::unobserve_property) when done.
    pub fn observe_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.observe_property_with_criteria(
            name,
            FormSelectionCriteria::new(Operation::ObserveProperty),
            input,
        )
    }

    /// Observes a remote property with explicit form selection criteria.
    pub fn observe_property_with_criteria(
        &self,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.servient.consumed_subscribe(
            &self.entry,
            &self.id,
            clinkz_wot_core::AffordanceTarget::Property(name.into()),
            AffordanceRef::Property(name),
            criteria_for_operation(criteria, Operation::ObserveProperty),
            input,
        )
    }

    /// Stops all active property observation subscriptions and releases wire
    /// resources (W3C Scripting API `unobserveProperty`).
    pub fn unobserve_property(&self, name: &str) {
        let key = SubscriptionKey::new(
            &clinkz_wot_core::AffordanceTarget::Property(name.into()),
            Operation::ObserveProperty.as_str(),
        );
        self.entry.stop_subscriptions(&key);
    }

    /// Reads multiple remote properties (W3C Scripting API
    /// `readMultipleProperties`).
    ///
    /// Each property is read individually through its own form selection and
    /// binding invocation.
    pub fn read_multiple_properties(
        &self,
        names: &[&str],
    ) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        let mut results = BTreeMap::new();
        for &name in names {
            let output = self.read_property(name, InteractionInput::empty())?;
            results.insert(name.into(), output);
        }
        Ok(results)
    }

    /// Reads all properties declared in the consumed TD (W3C Scripting API
    /// `readAllProperties`).
    pub fn read_all_properties(&self) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        let mut results = BTreeMap::new();
        if let Some(properties) = self.entry.thing().properties.as_ref() {
            for name in properties.keys() {
                let output = self.read_property(name, InteractionInput::empty())?;
                results.insert(name.clone(), output);
            }
        }
        Ok(results)
    }

    /// Writes multiple remote properties (W3C Scripting API
    /// `writeMultipleProperties`).
    pub fn write_multiple_properties(
        &self,
        values: &BTreeMap<String, InteractionInput>,
    ) -> ServientResult<()> {
        for (name, input) in values {
            self.write_property(name, input.clone())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Async consumer methods (behind `async` feature — M8).
//
// These provide an async API surface for consumers in async contexts. The
// current implementation delegates to the synchronous path, resolving
// immediately. Future native async bindings can replace the delegation with
// async I/O without changing the API.
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
impl<D> ConsumedThingHandle<D>
where
    D: clinkz_wot_discovery::ThingDirectory,
{
    /// Async variant of [`read_property`](Self::read_property).
    pub async fn read_property_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.read_property(name, input)
    }

    /// Async variant of [`write_property`](Self::write_property).
    pub async fn write_property_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.write_property(name, input)
    }

    /// Async variant of [`invoke_action`](Self::invoke_action).
    pub async fn invoke_action_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.invoke_action(name, input)
    }

    /// Async variant of [`subscribe_event`](Self::subscribe_event).
    pub async fn subscribe_event_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.subscribe_event(name, input)
    }

    /// Async variant of [`observe_property`](Self::observe_property).
    pub async fn observe_property_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.observe_property(name, input)
    }
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
