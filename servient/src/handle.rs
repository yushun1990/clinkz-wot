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

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    string::ToString,
    sync::Arc,
    vec::Vec,
};

use clinkz_wot_core::{
    AffordanceTarget, CoreError, EventBroker, EventName, EventSink, InteractionInput,
    InteractionOutput, Payload, Subscription, ThingId,
};
use clinkz_wot_protocol_bindings::{AffordanceRef, FormSelectionCriteria};
use clinkz_wot_td::{
    data_type::Operation,
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
};

use crate::{
    ExposedThingRegistry, ServientResult, consumed::ConsumedThingEntry, consumed::SubscriptionKey,
    servient::Servient,
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
    registry: Arc<ExposedThingRegistry>,
    event_broker: EventBroker,
    id: Arc<str>,
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
            id: Arc::clone(&self.id),
        }
    }
}

impl<D> ExposedThingHandle<D> {
    pub(crate) fn new(
        servient: Servient<D>,
        registry: Arc<ExposedThingRegistry>,
        event_broker: EventBroker,
        id: String,
    ) -> Self {
        Self {
            servient,
            registry,
            event_broker,
            id: Arc::from(id),
        }
    }

    /// Returns the exposed Thing identity.
    pub fn thing_id(&self) -> &str {
        self.id.as_ref()
    }
}

impl<D> ExposedThingHandle<D>
where
    D: clinkz_wot_discovery::ThingDirectory,
{
    /// Starts inbound serving for this Thing: registers inbound routes on all
    /// server bindings and publishes the TD to the directory (W3C WoT Scripting
    /// API `ExposedThing.expose`).
    ///
    /// Called automatically by [`Servient::expose`](crate::Servient::expose).
    /// Use [`Servient::produce`](crate::Servient::produce) + this method when
    /// you need to register handlers *before* the Thing becomes
    /// network-reachable, eliminating the window where the Thing is remotely
    /// addressable but has no handlers.
    ///
    /// Route-registration failure is fatal and returns `Err` (partially
    /// registered routes are rolled back); directory-publish failure is
    /// best-effort.
    pub fn expose(&self) -> ServientResult<()> {
        self.servient.start_serving(self.id.as_ref())
    }

    /// Reads a property, dispatching directly to the attached handler.
    ///
    /// The handler `Arc` is cloned out under a brief slot lock and invoked with
    /// the lock released, so a handler may re-enter the Servient (C7).
    pub fn read_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let handler = self
            .registry
            .dispatch(&self.id, |thing| {
                thing.ensure_property_affordance(name)?;
                thing.read_handler(name).ok_or(CoreError::MissingHandler {
                    target: AffordanceTarget::Property(name.into()),
                    operation: Operation::ReadProperty,
                })
            })
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.to_string()))?
            .map_err(crate::ServientError::from)?;
        handler.read(input).map_err(Into::into)
    }

    /// Writes a property, dispatching directly to the attached handler.
    pub fn write_property(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        let handler = self
            .registry
            .dispatch(&self.id, |thing| {
                thing.ensure_property_affordance(name)?;
                thing.write_handler(name).ok_or(CoreError::MissingHandler {
                    target: AffordanceTarget::Property(name.into()),
                    operation: Operation::WriteProperty,
                })
            })
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.to_string()))?
            .map_err(crate::ServientError::from)?;
        handler.write(input).map_err(Into::into)
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
        let names = self
            .with_thing_description(|td| {
                td.properties
                    .as_ref()
                    .map(|props| props.keys().cloned().collect::<Vec<String>>())
                    .unwrap_or_default()
            })
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.to_string()))?;
        let mut results = BTreeMap::new();
        for name in &names {
            let output = self.read_property(name, InteractionInput::empty())?;
            results.insert(name.clone(), output);
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

    /// Writes all writable properties declared in the TD (W3C Scripting API
    /// `writeAllProperties`; W3C TD `writeallproperties` meta-operation).
    ///
    /// Each entry in `values` is written to its property handler. Properties
    /// absent from `values` are left unchanged. This is the local in-process
    /// counterpart of the inbound `writeallproperties` dispatch; the semantic
    /// distinction from [`write_multiple_properties`](Self::write_multiple_properties)
    /// is only meaningful at the protocol level (different TD meta-operation).
    pub fn write_all_properties(
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
        let handler = self
            .registry
            .dispatch(&self.id, |thing| {
                thing.ensure_action_affordance(name)?;
                thing.action_handler(name).ok_or(CoreError::MissingHandler {
                    target: AffordanceTarget::Action(name.into()),
                    operation: Operation::InvokeAction,
                })
            })
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.to_string()))?
            .map_err(crate::ServientError::from)?;
        handler.invoke(input).map_err(Into::into)
    }

    /// Subscribes to an event, dispatching directly to the attached handler.
    pub fn subscribe_event(
        &self,
        name: &str,
        input: InteractionInput,
        sink: &mut dyn EventSink,
    ) -> ServientResult<InteractionOutput> {
        let handler = self
            .registry
            .dispatch(&self.id, |thing| {
                thing.ensure_event_affordance(name)?;
                thing
                    .subscribe_handler(name)
                    .ok_or(CoreError::MissingHandler {
                        target: AffordanceTarget::Event(name.into()),
                        operation: Operation::SubscribeEvent,
                    })
            })
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.to_string()))?
            .map_err(crate::ServientError::from)?;
        handler.subscribe(input, sink).map_err(Into::into)
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
                &ThingId::from(self.id.as_ref()),
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

    /// Attaches a property unobserve handler (W3C Scripting API
    /// `setPropertyUnobserveHandler`).
    pub fn set_property_unobserve_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::PropertyUnobserveHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_property_unobserve_handler(name, handler)
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

    /// Attaches an action query handler (W3C TD `queryaction` operation).
    pub fn set_action_query_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::ActionQueryHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_action_query_handler(name, handler)
        });
        Ok(())
    }

    /// Attaches an action cancel handler (W3C TD `cancelaction` operation).
    pub fn set_action_cancel_handler(
        &self,
        name: impl Into<String>,
        handler: impl clinkz_wot_core::ActionCancelHandler + 'static,
    ) -> ServientResult<()> {
        self.registry.dispatch(&self.id, |thing| {
            thing.register_action_cancel_handler(name, handler)
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
            .ok_or_else(|| crate::ServientError::ExposedThingNotFound(self.id.to_string()))
    }

    /// Borrows the exposed Thing Description read-only under the per-Thing lock
    /// and runs `f` without cloning the TD (W3C Scripting API efficiency
    /// helper).
    ///
    /// The closure must not call back into this handle's interaction methods
    /// (they re-acquire the per-Thing lock). Extract any owned data needed
    /// before returning.
    ///
    /// Returns `None` when the Thing is no longer registered (drained or
    /// destroyed), matching [`thing_description`](Self::thing_description)'s
    /// not-found case.
    pub fn with_thing_description<R>(&self, f: impl FnOnce(&Thing) -> R) -> Option<R> {
        self.registry.with_thing_description(&self.id, f)
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
    id: Arc<str>,
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
            id: Arc::clone(&self.id),
            entry: Arc::clone(&self.entry),
        }
    }
}

impl<D> ConsumedThingHandle<D> {
    pub(crate) fn new(servient: Servient<D>, id: String, entry: Arc<ConsumedThingEntry>) -> Self {
        Self {
            servient,
            id: Arc::from(id),
            entry,
        }
    }

    /// Returns the consumed Thing identity.
    pub fn thing_id(&self) -> &str {
        self.id.as_ref()
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
            clinkz_wot_core::AffordanceTarget::Action(name.into()),
            AffordanceRef::Action(name),
            criteria_for_operation(criteria, Operation::InvokeAction),
            input,
        )
    }

    /// Queries the status of a remote action, selecting a form by default
    /// criteria (W3C TD `queryaction` operation).
    pub fn query_action(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.query_action_with_criteria(
            name,
            FormSelectionCriteria::new(Operation::QueryAction),
            input,
        )
    }

    /// Queries the status of a remote action, selecting a form by explicit
    /// criteria (W3C TD `queryaction` operation).
    pub fn query_action_with_criteria(
        &self,
        name: &str,
        criteria: FormSelectionCriteria<'_>,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient.consumed_request(
            &self.entry,
            clinkz_wot_core::AffordanceTarget::Action(name.into()),
            AffordanceRef::Action(name),
            criteria_for_operation(criteria, Operation::QueryAction),
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
    /// Prefers a single Thing-level form declaring the `readmultipleproperties`
    /// operation (one round trip, W3C TD §6.3.3) when the consumed TD advertises
    /// one; otherwise falls back to one read per property.
    pub fn read_multiple_properties(
        &self,
        names: &[&str],
    ) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        // Membership set for O(1) filtering of the bulk response; avoids the
        // previous O(N×M) `names_owned.iter().any(...)` scan and the throwaway
        // `Vec<String>` allocation (serde_json serializes `&[&str]` directly).
        let requested: BTreeSet<&str> = names.iter().copied().collect();
        if let Some(result) = self.thing_level_bulk(Operation::ReadMultipleProperties, {
            let body = serde_json::to_vec(names).map_err(|err| {
                crate::ServientError::from(CoreError::InvalidInteraction(alloc::format!(
                    "failed to serialize readmultipleproperties names: {err}"
                )))
            })?;
            InteractionInput::with_payload(Payload::new(body, BULK_CONTENT_TYPE))
        }) {
            let output = result?;
            if let Some(map) = split_bulk_object_output(output) {
                // Keep only the requested names, in case the remote returned
                // extra properties.
                let requested: BTreeMap<String, InteractionOutput> = map
                    .into_iter()
                    .filter(|(name, _)| requested.contains(name.as_str()))
                    .collect();
                if !requested.is_empty() {
                    return Ok(requested);
                }
            }
            // Split failed or returned nothing — fall through to fan-out.
        }

        let mut results = BTreeMap::new();
        for &name in names {
            let output = self.read_property(name, InteractionInput::empty())?;
            results.insert(name.into(), output);
        }
        Ok(results)
    }

    /// Reads all properties declared in the consumed TD (W3C Scripting API
    /// `readAllProperties`).
    ///
    /// Prefers a single Thing-level form declaring the `readallproperties`
    /// operation (one round trip, W3C TD §6.3.3) when the consumed TD advertises
    /// one; otherwise falls back to one read per property.
    pub fn read_all_properties(&self) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        if let Some(result) =
            self.thing_level_bulk(Operation::ReadAllProperties, InteractionInput::empty())
        {
            let output = result?;
            if let Some(map) = split_bulk_object_output(output) {
                return Ok(map);
            }
            // Split failed — fall through to fan-out.
        }

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
    ///
    /// Prefers a single Thing-level form declaring the `writemultipleproperties`
    /// operation (one round trip, W3C TD §6.3.3) when the consumed TD advertises
    /// one; otherwise falls back to one write per property.
    pub fn write_multiple_properties(
        &self,
        values: &BTreeMap<String, InteractionInput>,
    ) -> ServientResult<()> {
        if let Some(result) = self.write_via_thing_level(Operation::WriteMultipleProperties, values)
        {
            return result.map(|_| ());
        }

        for (name, input) in values {
            self.write_property(name, input.clone())?;
        }
        Ok(())
    }

    /// Writes all writable properties declared in the consumed TD (W3C
    /// Scripting API `writeAllProperties`).
    ///
    /// Prefers a single Thing-level form declaring the `writeallproperties`
    /// operation (one round trip, W3C TD §6.3.3) when the consumed TD advertises
    /// one; otherwise falls back to one write per property.
    pub fn write_all_properties(
        &self,
        values: &BTreeMap<String, InteractionInput>,
    ) -> ServientResult<()> {
        if let Some(result) = self.write_via_thing_level(Operation::WriteAllProperties, values) {
            return result.map(|_| ());
        }

        for (name, input) in values {
            self.write_property(name, input.clone())?;
        }
        Ok(())
    }

    /// Observes all observable properties declared in the consumed TD (W3C
    /// Scripting API `observeAllProperties`).
    ///
    /// Prefers a single Thing-level form declaring the `observeallproperties`
    /// operation (one subscription) when the consumed TD advertises one;
    /// otherwise fans out across individual property observations and returns a
    /// merged [`Subscription`] that multiplexes them.
    pub fn observe_all_properties(&self, input: InteractionInput) -> ServientResult<Subscription> {
        if let Some(sub) =
            self.thing_level_subscribe(Operation::ObserveAllProperties, input.clone())
        {
            return sub;
        }

        let names = observable_property_names(self.entry.thing());
        let mut subs = Vec::new();
        for name in names {
            subs.push(self.observe_property(name, input.clone())?);
        }
        Ok(Subscription::merge(subs))
    }

    /// Stops all active property observations (W3C Scripting API
    /// `unobserveAllProperties`).
    ///
    /// Prefers a single Thing-level form declaring the
    /// `unobserveallproperties` operation (one round trip) when available;
    /// otherwise stops each active property observation individually.
    pub fn unobserve_all_properties(&self) -> ServientResult<()> {
        if self.has_thing_level_form_for(Operation::UnobserveAllProperties) {
            let _ = self.servient.consumed_request(
                &self.entry,
                AffordanceTarget::Thing,
                AffordanceRef::Thing,
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::UnobserveAllProperties),
                    Operation::UnobserveAllProperties,
                ),
                InteractionInput::empty(),
            )?;
            return Ok(());
        }

        for name in observable_property_names(self.entry.thing()) {
            self.unobserve_property(name);
        }
        Ok(())
    }

    /// Subscribes to all events declared in the consumed TD (W3C Scripting API
    /// `subscribeAllEvents`; W3C TD `subscribeallevents`).
    ///
    /// Prefers a single Thing-level form declaring the `subscribeallevents`
    /// operation (one subscription) when the consumed TD advertises one;
    /// otherwise fans out across individual event subscriptions and returns a
    /// merged [`Subscription`] that multiplexes them.
    pub fn subscribe_all_events(&self, input: InteractionInput) -> ServientResult<Subscription> {
        if let Some(sub) = self.thing_level_subscribe(Operation::SubscribeAllEvents, input.clone())
        {
            return sub;
        }

        let names = event_names(self.entry.thing());
        let mut subs = Vec::new();
        for name in names {
            subs.push(self.subscribe_event(name, input.clone())?);
        }
        Ok(Subscription::merge(subs))
    }

    /// Stops all active event subscriptions (W3C Scripting API
    /// `unsubscribeAllEvents`; W3C TD `unsubscribeallevents`).
    ///
    /// Prefers a single Thing-level form declaring the
    /// `unsubscribeallevents` operation (one round trip) when available;
    /// otherwise stops each active event subscription individually.
    pub fn unsubscribe_all_events(&self) -> ServientResult<()> {
        if self.has_thing_level_form_for(Operation::UnsubscribeAllEvents) {
            let _ = self.servient.consumed_request(
                &self.entry,
                AffordanceTarget::Thing,
                AffordanceRef::Thing,
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::UnsubscribeAllEvents),
                    Operation::UnsubscribeAllEvents,
                ),
                InteractionInput::empty(),
            )?;
            return Ok(());
        }

        for name in event_names(self.entry.thing()) {
            self.unsubscribe_event(name);
        }
        Ok(())
    }

    /// Queries the status of all actions declared in the consumed TD (W3C TD
    /// `queryallactions` meta-operation; W3C TD §6.3.3).
    ///
    /// Prefers a single Thing-level form declaring the `queryallactions`
    /// operation (one round trip) when the consumed TD advertises one, splitting
    /// the combined JSON-object response into a per-action map; otherwise falls
    /// back to one query per action.
    pub fn query_all_actions(
        &self,
        input: InteractionInput,
    ) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        if let Some(result) = self.thing_level_bulk(Operation::QueryAllActions, input.clone()) {
            let output = result?;
            if let Some(map) = split_bulk_object_output(output) {
                return Ok(map);
            }
            // Split failed — fall through to fan-out.
        }

        let mut results = BTreeMap::new();
        for name in action_names(self.entry.thing()) {
            let output = self.query_action(name, input.clone())?;
            results.insert(name.into(), output);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Thing-level bulk form preference (W3C TD §6.3.3).
    //
    // When the consumed TD advertises a Thing-level form for a bulk
    // meta-operation, route the request through it (one round trip) instead of
    // fanning out across N per-property forms. Fall back to fan-out when no such
    // form exists or the bulk response cannot be interpreted.
    // -----------------------------------------------------------------------

    /// Returns `true` when the consumed TD declares a Thing-level form that
    /// supports `operation`.
    fn has_thing_level_form_for(&self, operation: Operation) -> bool {
        let thing = self.entry.thing();
        thing
            .forms
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .any(|form| effective_form_operations(FormContext::Thing, form).contains(&operation))
    }

    /// Issues a Thing-level bulk read/write request when a matching form exists.
    ///
    /// Returns `None` when no Thing-level form supports `operation`, so callers
    /// can fall back to per-property fan-out.
    fn thing_level_bulk(
        &self,
        operation: Operation,
        input: InteractionInput,
    ) -> Option<ServientResult<InteractionOutput>> {
        if !self.has_thing_level_form_for(operation) {
            return None;
        }
        Some(self.servient.consumed_request(
            &self.entry,
            AffordanceTarget::Thing,
            AffordanceRef::Thing,
            criteria_for_operation(FormSelectionCriteria::new(operation), operation),
            input,
        ))
    }

    /// Opens a Thing-level streaming subscription when a matching form exists.
    ///
    /// Returns `None` when no Thing-level form supports `operation`, so callers
    /// can fall back to per-affordance fan-out.
    fn thing_level_subscribe(
        &self,
        operation: Operation,
        input: InteractionInput,
    ) -> Option<ServientResult<Subscription>> {
        if !self.has_thing_level_form_for(operation) {
            return None;
        }
        Some(self.servient.consumed_subscribe(
            &self.entry,
            AffordanceTarget::Thing,
            AffordanceRef::Thing,
            criteria_for_operation(FormSelectionCriteria::new(operation), operation),
            input,
        ))
    }

    /// Combines per-property write inputs into a single JSON-object payload and
    /// issues a Thing-level bulk write when a matching form exists.
    ///
    /// Each value's payload body is parsed as a JSON value and assembled into a
    /// `{name: value}` object. Returns `None` when no Thing-level form supports
    /// `operation`.
    fn write_via_thing_level(
        &self,
        operation: Operation,
        values: &BTreeMap<String, InteractionInput>,
    ) -> Option<ServientResult<InteractionOutput>> {
        if !self.has_thing_level_form_for(operation) {
            return None;
        }

        let mut combined = serde_json::Map::new();
        for (name, input) in values {
            match input.payload.as_ref() {
                Some(payload) if !payload.body.is_empty() => {
                    match serde_json::from_slice::<serde_json::Value>(payload.body.as_ref()) {
                        Ok(value) => {
                            combined.insert(name.clone(), value);
                        }
                        Err(_) => {
                            // Treat as an opaque JSON string when the payload
                            // is not valid JSON so no value is silently lost.
                            combined.insert(
                                name.clone(),
                                serde_json::Value::String(
                                    String::from_utf8_lossy(payload.body.as_ref()).into_owned(),
                                ),
                            );
                        }
                    }
                }
                _ => {
                    combined.insert(name.clone(), serde_json::Value::Null);
                }
            }
        }

        let body = match serde_json::to_vec(&serde_json::Value::Object(combined)) {
            Ok(body) => body,
            Err(err) => {
                return Some(Err(crate::ServientError::from(
                    CoreError::InvalidInteraction(alloc::format!(
                        "failed to serialize bulk write payload: {err}"
                    )),
                )));
            }
        };

        let input = InteractionInput::with_payload(Payload::new(body, BULK_CONTENT_TYPE));
        Some(self.servient.consumed_request(
            &self.entry,
            AffordanceTarget::Thing,
            AffordanceRef::Thing,
            criteria_for_operation(FormSelectionCriteria::new(operation), operation),
            input,
        ))
    }
}

// ---------------------------------------------------------------------------
// Async consumer methods (behind `async` feature — M8).
//
// These provide an async API surface for consumers in async contexts. When
// the underlying binding implements `AsyncClientBinding`, the call routes
// through native async I/O (e.g., `session.get().await` for zenoh). When it
// does not, the sync path is used as a fallback (which may block the async
// executor).
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
impl<D> ConsumedThingHandle<D>
where
    D: clinkz_wot_discovery::ThingDirectory,
{
    /// Async variant of [`read_property`](Self::read_property).
    ///
    /// Routes through `AsyncClientBinding::invoke_async` when the concrete
    /// binding implements it, giving true non-blocking I/O. Otherwise falls
    /// back to the synchronous `invoke` path.
    pub async fn read_property_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient
            .consumed_request_async(
                &self.entry,
                clinkz_wot_core::AffordanceTarget::Property(name.into()),
                AffordanceRef::Property(name),
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::ReadProperty),
                    Operation::ReadProperty,
                ),
                input,
            )
            .await
    }

    /// Async variant of [`write_property`](Self::write_property).
    pub async fn write_property_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient
            .consumed_request_async(
                &self.entry,
                clinkz_wot_core::AffordanceTarget::Property(name.into()),
                AffordanceRef::Property(name),
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::WriteProperty),
                    Operation::WriteProperty,
                ),
                input,
            )
            .await
    }

    /// Async variant of [`invoke_action`](Self::invoke_action).
    pub async fn invoke_action_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient
            .consumed_request_async(
                &self.entry,
                clinkz_wot_core::AffordanceTarget::Action(name.into()),
                AffordanceRef::Action(name),
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::InvokeAction),
                    Operation::InvokeAction,
                ),
                input,
            )
            .await
    }

    /// Async variant of [`query_action`](Self::query_action).
    pub async fn query_action_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<InteractionOutput> {
        self.servient
            .consumed_request_async(
                &self.entry,
                clinkz_wot_core::AffordanceTarget::Action(name.into()),
                AffordanceRef::Action(name),
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::QueryAction),
                    Operation::QueryAction,
                ),
                input,
            )
            .await
    }

    /// Async variant of [`subscribe_event`](Self::subscribe_event).
    pub async fn subscribe_event_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.servient
            .consumed_subscribe_async(
                &self.entry,
                clinkz_wot_core::AffordanceTarget::Event(name.into()),
                AffordanceRef::Event(name),
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::SubscribeEvent),
                    Operation::SubscribeEvent,
                ),
                input,
            )
            .await
    }

    /// Async variant of [`observe_property`](Self::observe_property).
    pub async fn observe_property_async(
        &self,
        name: &str,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        self.servient
            .consumed_subscribe_async(
                &self.entry,
                clinkz_wot_core::AffordanceTarget::Property(name.into()),
                AffordanceRef::Property(name),
                criteria_for_operation(
                    FormSelectionCriteria::new(Operation::ObserveProperty),
                    Operation::ObserveProperty,
                ),
                input,
            )
            .await
    }

    /// Async variant of [`write_all_properties`](Self::write_all_properties).
    pub async fn write_all_properties_async(
        &self,
        values: &BTreeMap<String, InteractionInput>,
    ) -> ServientResult<()> {
        if let Some(result) = self.write_via_thing_level(Operation::WriteAllProperties, values) {
            return result.map(|_| ());
        }
        for (name, input) in values {
            self.write_property_async(name, input.clone()).await?;
        }
        Ok(())
    }

    /// Async variant of [`observe_all_properties`](Self::observe_all_properties).
    pub async fn observe_all_properties_async(
        &self,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        if let Some(sub) =
            self.thing_level_subscribe(Operation::ObserveAllProperties, input.clone())
        {
            return sub;
        }
        let names = observable_property_names(self.entry.thing());
        let mut subs = Vec::new();
        for name in names {
            subs.push(self.observe_property_async(name, input.clone()).await?);
        }
        Ok(Subscription::merge(subs))
    }

    /// Async variant of
    /// [`unobserve_all_properties`](Self::unobserve_all_properties).
    pub async fn unobserve_all_properties_async(&self) -> ServientResult<()> {
        if self.has_thing_level_form_for(Operation::UnobserveAllProperties) {
            let _ = self
                .servient
                .consumed_request_async(
                    &self.entry,
                    AffordanceTarget::Thing,
                    AffordanceRef::Thing,
                    criteria_for_operation(
                        FormSelectionCriteria::new(Operation::UnobserveAllProperties),
                        Operation::UnobserveAllProperties,
                    ),
                    InteractionInput::empty(),
                )
                .await?;
            return Ok(());
        }
        for name in observable_property_names(self.entry.thing()) {
            self.unobserve_property(name);
        }
        Ok(())
    }

    /// Async variant of [`subscribe_all_events`](Self::subscribe_all_events).
    pub async fn subscribe_all_events_async(
        &self,
        input: InteractionInput,
    ) -> ServientResult<Subscription> {
        if let Some(sub) = self.thing_level_subscribe(Operation::SubscribeAllEvents, input.clone())
        {
            return sub;
        }
        let names = event_names(self.entry.thing());
        let mut subs = Vec::new();
        for name in names {
            subs.push(self.subscribe_event_async(name, input.clone()).await?);
        }
        Ok(Subscription::merge(subs))
    }

    /// Async variant of
    /// [`unsubscribe_all_events`](Self::unsubscribe_all_events).
    pub async fn unsubscribe_all_events_async(&self) -> ServientResult<()> {
        if self.has_thing_level_form_for(Operation::UnsubscribeAllEvents) {
            let _ = self
                .servient
                .consumed_request_async(
                    &self.entry,
                    AffordanceTarget::Thing,
                    AffordanceRef::Thing,
                    criteria_for_operation(
                        FormSelectionCriteria::new(Operation::UnsubscribeAllEvents),
                        Operation::UnsubscribeAllEvents,
                    ),
                    InteractionInput::empty(),
                )
                .await?;
            return Ok(());
        }
        for name in event_names(self.entry.thing()) {
            self.unsubscribe_event(name);
        }
        Ok(())
    }

    /// Async variant of [`query_all_actions`](Self::query_all_actions).
    pub async fn query_all_actions_async(
        &self,
        input: InteractionInput,
    ) -> ServientResult<BTreeMap<String, InteractionOutput>> {
        if let Some(result) = self.thing_level_bulk(Operation::QueryAllActions, input.clone()) {
            let output = result?;
            if let Some(map) = split_bulk_object_output(output) {
                return Ok(map);
            }
        }

        let mut results = BTreeMap::new();
        for name in action_names(self.entry.thing()) {
            let output = self.query_action_async(name, input.clone()).await?;
            results.insert(name.into(), output);
        }
        Ok(results)
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

/// Content type used for assembled and parsed bulk payloads (the WoT default).
const BULK_CONTENT_TYPE: &str = "application/json";

/// Splits a bulk read response payload (a JSON object `{name: value}`) into a
/// per-property [`InteractionOutput`] map.
///
/// Returns `None` when the payload is missing or not a JSON object, so callers
/// can fall back to per-property fan-out instead of failing the whole bulk
/// operation.
fn split_bulk_object_output(
    output: InteractionOutput,
) -> Option<BTreeMap<String, InteractionOutput>> {
    let payload = output.payload?;
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(payload.body.as_ref()).ok()?;
    let content_type = if payload.content_type.is_empty() {
        BULK_CONTENT_TYPE
    } else {
        payload.content_type.as_str()
    };
    let mut results = BTreeMap::new();
    for (name, value) in map {
        let body = serde_json::to_vec(&value).ok()?;
        results.insert(
            name,
            InteractionOutput::with_payload(Payload::new(body, content_type)),
        );
    }
    Some(results)
}

/// Returns the names of all observable properties declared in the TD.
fn observable_property_names(thing: &Thing) -> Vec<&str> {
    thing
        .properties
        .as_ref()
        .map(|props| {
            props
                .iter()
                .filter(|(_, p)| p.observable)
                .map(|(name, _)| name.as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the names of all events declared in the TD.
fn event_names(thing: &Thing) -> Vec<&str> {
    thing
        .events
        .as_ref()
        .map(|events| events.keys().map(|name| name.as_str()).collect())
        .unwrap_or_default()
}

/// Returns the names of all actions declared in the TD.
fn action_names(thing: &Thing) -> Vec<&str> {
    thing
        .actions
        .as_ref()
        .map(|actions| actions.keys().map(|name| name.as_str()).collect())
        .unwrap_or_default()
}
