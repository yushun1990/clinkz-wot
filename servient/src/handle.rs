//! `ExposedThingHandle` / `ConsumedThingHandle` — non-generic, holding a
//! `Servient` clone (baseline v4.0 §7.3–§7.4 / phase-p3 §3.3–§3.4, §3.7).

use alloc::sync::Arc;

use clinkz_wot_core::{
    ActionCancelHandler, ActionHandler, ActionQueryHandler, AffordanceTarget, ConsumedThing,
    CoreError, CoreResult, EventSubscribeHandler, EventUnsubscribeHandler, InteractionInput,
    InteractionOptions, InteractionOutput, Payload, PropertyObserveHandler, PropertyReadHandler,
    PropertyUnobserveHandler, PropertyWriteHandler, ThingId, WotLock,
};
use clinkz_wot_td::{data_type::Operation, thing::Thing};

use crate::ServientResult;
use crate::registry::ExposedThingSlot;
use crate::servient::Servient;

// ---------------------------------------------------------------------------
// ExposedThingHandle — produced Thing + handler slots; frozen TD at expose.
// ---------------------------------------------------------------------------

/// A handle to a locally produced Thing. Draft after `produce()` (state owned
/// by the handle, not remotely servable); exposed after `expose()` (inserted
/// into the servable registry + routes registered); removed after `destroy()`.
///
/// Handlers may be attached or replaced throughout the exposed lifetime (AD14).
/// The TD affordance set is frozen at `expose()` (decision 2): there is no
/// `add_*`/`remove_*` post-produce on this handle.
pub struct ExposedThingHandle {
    servient: Servient,
    slot: Arc<WotLock<ExposedThingSlot>>,
    id: ThingId,
}

impl ExposedThingHandle {
    pub(crate) fn new(
        servient: Servient,
        slot: Arc<WotLock<ExposedThingSlot>>,
        id: ThingId,
    ) -> Self {
        Self { servient, slot, id }
    }

    /// Returns the Thing id.
    pub fn id(&self) -> &ThingId {
        &self.id
    }

    /// Returns a clone of the Thing Description.
    pub fn thing_description(&self) -> Thing {
        self.slot.with_read(|s| s.thing.thing_description().clone())
    }

    // --- handler attachment (replaceable throughout produce→expose→destroy) ---

    pub fn set_property_read_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl PropertyReadHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_property_read_handler(name, handler));
    }

    pub fn set_property_write_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl PropertyWriteHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_property_write_handler(name, handler));
    }

    pub fn set_property_observe_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl PropertyObserveHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_property_observe_handler(name, handler));
    }

    pub fn set_property_unobserve_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl PropertyUnobserveHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_property_unobserve_handler(name, handler));
    }

    pub fn set_action_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl ActionHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_action_handler(name, handler));
    }

    pub fn set_action_query_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl ActionQueryHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_action_query_handler(name, handler));
    }

    pub fn set_action_cancel_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl ActionCancelHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_action_cancel_handler(name, handler));
    }

    pub fn set_event_subscribe_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl EventSubscribeHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_event_subscribe_handler(name, handler));
    }

    pub fn set_event_unsubscribe_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl EventUnsubscribeHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_event_unsubscribe_handler(name, handler));
    }

    // --- lifecycle ---

    /// Registers routes on every server binding, inserts into the servable
    /// registry, and publishes the TD. Multi-binding rollback on failure
    /// (E12/AD27). The TD affordance set is frozen after this.
    pub async fn expose(&self) -> ServientResult<()> {
        self.servient
            .expose_thing(self.id.clone(), self.slot.clone())
            .await
    }

    /// Quiescing teardown (AD15): unregisters routes (no new requests), drains
    /// / rejects in-flight, removes the registry entry, unpublishes. Idempotent
    /// (AD27/E13). The Thing is gone afterwards — re-`produce` to re-expose.
    pub async fn destroy(&self) -> ServientResult<()> {
        self.servient.destroy_thing(&self.id).await
    }

    // --- local (server-side) interaction ---

    /// Reads a property locally via its registered handler.
    pub fn read_property(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot.with_read(|s| s.thing.read_property(name, input))
    }

    /// Writes a property locally via its registered write handler.
    pub fn write_property(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot.with(|s| s.thing.write_property(name, input))
    }

    /// Invokes an action locally via its registered invoke handler.
    pub fn invoke_action(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot.with(|s| s.thing.invoke_action(name, input))
    }

    /// Fans an event payload out to registered subscribers via the broker.
    pub fn emit_event(&self, name: &str, payload: Payload) -> CoreResult<()> {
        self.servient.emit_event(&self.id, name, payload)
    }

    /// Fans a property-change payload out via the broker (same path as events).
    pub fn emit_property_change(&self, name: &str, payload: Payload) -> CoreResult<()> {
        self.servient.emit_event(&self.id, name, payload)
    }
}

// ---------------------------------------------------------------------------
// ConsumedThingHandle — consumed Thing; real async via ClientBinding.
// ---------------------------------------------------------------------------

/// A handle to a consumed Thing. All interaction methods drive the real async
/// [`ClientBinding::invoke`] / `subscribe` through the underlying
/// [`ConsumedThing`].
pub struct ConsumedThingHandle {
    #[allow(dead_code)]
    servient: Servient,
    consumed: ConsumedThing,
    id: ThingId,
}

impl ConsumedThingHandle {
    pub(crate) fn new(servient: Servient, consumed: ConsumedThing, id: ThingId) -> Self {
        // The Servient pre-registers client bindings into `consumed` before
        // wrapping it (see `Servient::consume`).
        let _ = &servient;
        Self {
            servient,
            consumed,
            id,
        }
    }

    /// Returns the Thing id.
    pub fn id(&self) -> &ThingId {
        &self.id
    }

    /// Returns the Thing Description.
    pub fn thing_description(&self) -> &Thing {
        self.consumed.thing_description()
    }

    fn affordance_form(
        &self,
        target: &AffordanceTarget,
        operation: Operation,
    ) -> CoreResult<Arc<clinkz_wot_td::form::Form>> {
        let thing = self.consumed.thing_description();
        let forms = match target {
            AffordanceTarget::Thing => thing.forms.as_deref().unwrap_or(&[]),
            AffordanceTarget::Property(name) => thing
                .properties
                .as_ref()
                .and_then(|m| m.get(&**name))
                .map(|p| p._interaction.forms.as_slice())
                .unwrap_or(&[]),
            AffordanceTarget::Action(name) => thing
                .actions
                .as_ref()
                .and_then(|m| m.get(&**name))
                .map(|a| a._interaction.forms.as_slice())
                .unwrap_or(&[]),
            AffordanceTarget::Event(name) => thing
                .events
                .as_ref()
                .and_then(|m| m.get(&**name))
                .map(|e| e._interaction.forms.as_slice())
                .unwrap_or(&[]),
        };
        // First form whose effective operations contain `operation`.
        for form in forms {
            let ctx = match target {
                AffordanceTarget::Thing => clinkz_wot_td::td_defaults::FormContext::Thing,
                AffordanceTarget::Property(_) => clinkz_wot_td::td_defaults::FormContext::Property(
                    thing
                        .properties
                        .as_ref()
                        .and_then(|m| m.get(target.name().unwrap_or("")))
                        .unwrap(),
                ),
                AffordanceTarget::Action(_) => clinkz_wot_td::td_defaults::FormContext::Action(
                    thing
                        .actions
                        .as_ref()
                        .and_then(|m| m.get(target.name().unwrap_or("")))
                        .unwrap(),
                ),
                AffordanceTarget::Event(_) => clinkz_wot_td::td_defaults::FormContext::Event(
                    thing
                        .events
                        .as_ref()
                        .and_then(|m| m.get(target.name().unwrap_or("")))
                        .unwrap(),
                ),
            };
            if clinkz_wot_td::td_defaults::effective_form_operations(ctx, form).contains(&operation)
            {
                return Ok(Arc::new(form.clone()));
            }
        }
        Err(CoreError::UnsupportedOperation(alloc::format!(
            "no form for {} on {:?}",
            operation.as_str(),
            target
        )))
    }

    async fn invoke_op(
        &self,
        target: AffordanceTarget,
        operation: Operation,
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        let form = self.affordance_form(&target, operation)?;
        let input = InteractionInput {
            data: options.data,
            uri_variables: options.uri_variables,
            principal: None,
            accept: None,
        };
        self.consumed.request(target, operation, form, input).await
    }

    pub async fn read_property(
        &self,
        name: &str,
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        self.invoke_op(
            AffordanceTarget::Property(name.into()),
            Operation::ReadProperty,
            options,
        )
        .await
    }

    pub async fn write_property(
        &self,
        name: &str,
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        self.invoke_op(
            AffordanceTarget::Property(name.into()),
            Operation::WriteProperty,
            options,
        )
        .await
    }

    pub async fn invoke_action(
        &self,
        name: &str,
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        self.invoke_op(
            AffordanceTarget::Action(name.into()),
            Operation::InvokeAction,
            options,
        )
        .await
    }
}
