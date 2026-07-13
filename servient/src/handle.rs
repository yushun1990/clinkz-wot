//! `ExposedThingHandle` / `ConsumedThingHandle` — non-generic, holding a
//! `Servient` clone (baseline v4.0 §7.3–§7.4 / phase-p3 §3.3–§3.4, §3.7).

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use clinkz_wot_core::{
    ActionCancelHandler, ActionHandler, ActionQueryHandler, AffordanceTarget, CancelSlot,
    ConsumedThing, CoreError, CoreResult, ErrorContext, ErrorPhase, EventName, EventStream,
    EventSubscribeHandler, EventUnsubscribeHandler, InteractionInput, InteractionOptions,
    InteractionOutput, InvokeSlot, ObserveSlot, Payload, PropertyObserveHandler,
    PropertyReadHandler, PropertyUnobserveHandler, PropertyWriteHandler, PushFn, QuerySlot,
    ReadSlot, RetryClass, SelectionFailureReason, SubscribeSlot, Subscription, SubscriptionGuard,
    ThingId, UnobserveSlot, UnsubscribeSlot, WotLock, WriteSlot,
};
#[cfg(feature = "async")]
use clinkz_wot_core::{
    AsyncActionCancelHandler, AsyncActionHandler, AsyncActionQueryHandler,
    AsyncEventSubscribeHandler, AsyncEventUnsubscribeHandler, AsyncPropertyObserveHandler,
    AsyncPropertyReadHandler, AsyncPropertyUnobserveHandler, AsyncPropertyWriteHandler,
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
    server_bindings: Arc<[Arc<dyn clinkz_wot_core::ServerBinding>]>,
}

impl ExposedThingHandle {
    pub(crate) fn new(
        servient: Servient,
        slot: Arc<WotLock<ExposedThingSlot>>,
        id: ThingId,
        server_bindings: Arc<[Arc<dyn clinkz_wot_core::ServerBinding>]>,
    ) -> Self {
        Self {
            servient,
            slot,
            id,
            server_bindings,
        }
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

    // --- async handler setters (mirror the sync set; for I/O-bound handlers) ---
    //
    // Both flavors registerable on the same handle. Per-affordance the last
    // setter wins (the underlying `core::ExposedThing` stores handlers as
    // `Option<XSlot>` where `XSlot` is a `Sync(..) | Async(..)` enum, last
    // write replaces). Gated on `#[cfg(feature = "async")]`.

    /// Registers an async property-read handler for `name` (replaces any
    /// prior handler, sync or async).
    #[cfg(feature = "async")]
    pub fn set_async_property_read_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncPropertyReadHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_property_read_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_property_write_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncPropertyWriteHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_property_write_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_property_observe_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncPropertyObserveHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_property_observe_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_property_unobserve_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncPropertyUnobserveHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_property_unobserve_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_action_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncActionHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_action_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_action_query_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncActionQueryHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_action_query_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_action_cancel_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncActionCancelHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_action_cancel_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_event_subscribe_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncEventSubscribeHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_event_subscribe_handler(name, handler));
    }

    #[cfg(feature = "async")]
    pub fn set_async_event_unsubscribe_handler(
        &self,
        name: impl Into<alloc::string::String>,
        handler: impl AsyncEventUnsubscribeHandler + 'static,
    ) {
        self.slot
            .with(|s| s.thing.set_async_event_unsubscribe_handler(name, handler));
    }

    // --- lifecycle ---

    /// Registers routes on every server binding, inserts into the servable
    /// registry, and publishes the TD. Multi-binding rollback on failure
    /// (E12/AD27). The TD affordance set is frozen after this.
    pub async fn expose(&self) -> ServientResult<()> {
        self.servient
            .expose_thing(self.id.clone(), self.slot.clone(), &self.server_bindings)
            .await
    }

    /// Quiescing teardown (AD15): unregisters routes (no new requests), drains
    /// / rejects in-flight, removes the registry entry, unpublishes. Idempotent
    /// (AD27/E13). The Thing is gone afterwards — re-`produce` to re-expose.
    pub async fn destroy(&self) -> ServientResult<()> {
        self.servient
            .destroy_thing(&self.id, &self.server_bindings)
            .await
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

    /// Queries an action's state locally (TD 1.1 `queryaction`). Returns
    /// `UnsupportedOperation` when no query handler is registered for `name` or
    /// when only an async handler is registered (sync dispatch cannot drive
    /// async handlers — use [`Self::query_action_async`] for those).
    pub fn query_action(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot.with_read(|s| s.thing.query_action(name, input))
    }

    /// Cancels an in-flight action locally (TD 1.1 `cancelaction`). Returns
    /// `UnsupportedOperation` for the same reasons as [`Self::query_action`].
    pub fn cancel_action(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot.with(|s| s.thing.cancel_action(name, input))
    }

    /// Triggers the observe handler locally, fanning its first push out via
    /// `push`. Returns `UnsupportedOperation` for the same reasons as
    /// [`Self::query_action`].
    pub fn observe_property(
        &self,
        name: &str,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput> {
        self.slot
            .with_read(|s| s.thing.observe_property(name, input, push))
    }

    /// Triggers the unobserve handler locally.
    pub fn unobserve_property(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot
            .with_read(|s| s.thing.unobserve_property(name, input))
    }

    /// Triggers the event subscribe handler locally.
    pub fn subscribe_event(
        &self,
        name: &str,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput> {
        self.slot
            .with_read(|s| s.thing.subscribe_event(name, input, push))
    }

    /// Triggers the event unsubscribe handler locally.
    pub fn unsubscribe_event(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.slot
            .with_read(|s| s.thing.unsubscribe_event(name, input))
    }

    // --- async local dispatch (drives sync OR async handlers) ---
    //
    // For each op, lock the slot, clone the handler Arc out, drop the lock,
    // and `.await` outside the critical section. The `WotLock::with_read`
    // closure is sync so it cannot host an `.await` itself; cloning the Arc
    // out keeps the lock guard's scope short. Sync handlers run inline
    // inside the closure; async handlers run after the lock drops.

    /// Async local read; drives sync handlers inline or awaits async ones.
    #[cfg(feature = "async")]
    pub async fn read_property_async(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self.slot.with_read(|s| s.thing.property_handler_read(name));
        match slot {
            Some(ReadSlot::Sync(h)) => h.read(input),
            Some(ReadSlot::Async(h)) => h.read(input).await,
            None => Err(missing_local_handler(Operation::ReadProperty)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn write_property_async(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self
            .slot
            .with_read(|s| s.thing.property_handler_write(name));
        match slot {
            Some(WriteSlot::Sync(h)) => h.write(input),
            Some(WriteSlot::Async(h)) => h.write(input).await,
            None => Err(missing_local_handler(Operation::WriteProperty)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn invoke_action_async(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self.slot.with_read(|s| s.thing.action_handler_invoke(name));
        match slot {
            Some(InvokeSlot::Sync(h)) => h.invoke(input),
            Some(InvokeSlot::Async(h)) => h.invoke(input).await,
            None => Err(missing_local_handler(Operation::InvokeAction)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn query_action_async(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self.slot.with_read(|s| s.thing.action_handler_query(name));
        match slot {
            Some(QuerySlot::Sync(h)) => h.query(input),
            Some(QuerySlot::Async(h)) => h.query(input).await,
            None => Err(missing_local_handler(Operation::QueryAction)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn cancel_action_async(
        &self,
        name: &str,
        input: &mut InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self.slot.with_read(|s| s.thing.action_handler_cancel(name));
        match slot {
            Some(CancelSlot::Sync(h)) => h.cancel(input),
            Some(CancelSlot::Async(h)) => h.cancel(input).await,
            None => Err(missing_local_handler(Operation::CancelAction)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn observe_property_async(
        &self,
        name: &str,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput> {
        let slot = self
            .slot
            .with_read(|s| s.thing.property_handler_observe(name));
        match slot {
            Some(ObserveSlot::Sync(h)) => h.observe(input, push),
            Some(ObserveSlot::Async(h)) => h.observe(input, push).await,
            None => Err(missing_local_handler(Operation::ObserveProperty)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn unobserve_property_async(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self
            .slot
            .with_read(|s| s.thing.property_handler_unobserve(name));
        match slot {
            Some(UnobserveSlot::Sync(h)) => h.unobserve(input),
            Some(UnobserveSlot::Async(h)) => h.unobserve(input).await,
            None => Err(missing_local_handler(Operation::UnobserveProperty)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn subscribe_event_async(
        &self,
        name: &str,
        input: &InteractionInput,
        push: PushFn<'_>,
    ) -> CoreResult<InteractionOutput> {
        let slot = self
            .slot
            .with_read(|s| s.thing.event_handler_subscribe(name));
        match slot {
            Some(SubscribeSlot::Sync(h)) => h.subscribe(input, push),
            Some(SubscribeSlot::Async(h)) => h.subscribe(input, push).await,
            None => Err(missing_local_handler(Operation::SubscribeEvent)),
        }
    }

    #[cfg(feature = "async")]
    pub async fn unsubscribe_event_async(
        &self,
        name: &str,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        let slot = self
            .slot
            .with_read(|s| s.thing.event_handler_unsubscribe(name));
        match slot {
            Some(UnsubscribeSlot::Sync(h)) => h.unsubscribe(input),
            Some(UnsubscribeSlot::Async(h)) => h.unsubscribe(input).await,
            None => Err(missing_local_handler(Operation::UnsubscribeEvent)),
        }
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

/// Constructs the canonical "no handler registered for this operation" error.
fn missing_local_handler(operation: Operation) -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Handler, RetryClass::Never).with_operation(operation),
    )
}

// ---------------------------------------------------------------------------
// ConsumedThingHandle — consumed Thing; real async via ClientBinding.
// ---------------------------------------------------------------------------

/// A handle to a consumed Thing. All interaction methods drive the real async
/// [`ClientBinding::invoke`] / `subscribe` through the underlying
/// [`ConsumedThing`].
///
/// Streaming ops (`observe_property`, `subscribe_event`, `subscribe_all_events`)
/// hand back a [`Subscription`] / [`EventStream`] for the caller to drain.
/// The wire-side [`SubscriptionGuard`] for each active subscription is owned
/// by the handle under a `guards` map keyed by affordance target, so the
/// caller never has to manage the guard explicitly. Calling the matching
/// `unobserve_*` / `unsubscribe_*` method drops the guard; dropping the
/// handle drops every still-active guard.
pub struct ConsumedThingHandle {
    #[allow(dead_code)]
    servient: Servient,
    consumed: ConsumedThing,
    id: ThingId,
    guards: WotLock<BTreeMap<AffordanceTarget, Box<dyn SubscriptionGuard>>>,
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
            guards: WotLock::new(BTreeMap::new()),
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
        let (forms, affordance_exists): (&[clinkz_wot_td::form::Form], bool) = match target {
            AffordanceTarget::Thing => (thing.forms.as_deref().unwrap_or(&[]), true),
            AffordanceTarget::Property(name) => match thing
                .properties
                .as_ref()
                .and_then(|affordances| affordances.get(&**name))
            {
                Some(property) => (property._interaction.forms.as_slice(), true),
                None => (&[], false),
            },
            AffordanceTarget::Action(name) => match thing
                .actions
                .as_ref()
                .and_then(|affordances| affordances.get(&**name))
            {
                Some(action) => (action._interaction.forms.as_slice(), true),
                None => (&[], false),
            },
            AffordanceTarget::Event(name) => match thing
                .events
                .as_ref()
                .and_then(|affordances| affordances.get(&**name))
            {
                Some(event) => (event._interaction.forms.as_slice(), true),
                None => (&[], false),
            },
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
        Err(CoreError::Selection {
            reason: if affordance_exists {
                SelectionFailureReason::NoFormSupportsOperation
            } else {
                SelectionFailureReason::AffordanceMissing
            },
            context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never)
                .with_operation(operation),
        })
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

    /// Opens a streaming subscription against the selected affordance form,
    /// returning the consumer-side [`Subscription`]. The wire-side guard is
    /// stored in the handle's `guards` map keyed by `target`.
    async fn subscribe_op(
        &self,
        target: AffordanceTarget,
        operation: Operation,
        options: InteractionOptions,
    ) -> CoreResult<Subscription> {
        let form = self.affordance_form(&target, operation)?;
        let input = InteractionInput {
            data: options.data,
            uri_variables: options.uri_variables,
            principal: None,
            accept: None,
        };
        let (subscription, guard) = self
            .consumed
            .subscribe(target.clone(), operation, form, input)
            .await?;
        self.guards.with(|g| {
            g.insert(target, guard);
        });
        Ok(subscription)
    }

    /// Opens a streaming subscription for event `name` without storing the
    /// guard — used by `subscribe_all_events` where the caller manages N
    /// guards at once and merges the subscriptions into a single stream.
    async fn subscribe_op_with_guard(
        &self,
        target: AffordanceTarget,
        operation: Operation,
        options: InteractionOptions,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        let form = self.affordance_form(&target, operation)?;
        let input = InteractionInput {
            data: options.data,
            uri_variables: options.uri_variables,
            principal: None,
            accept: None,
        };
        self.consumed
            .subscribe(target, operation, form, input)
            .await
    }

    /// Drops the guard stored under `target`, releasing the wire-side
    /// subscription. Returns `Ok(())` even if no guard was registered (the
    /// caller may have already dropped the subscription, or never opened
    /// one). Idempotent.
    fn drop_guard(&self, target: &AffordanceTarget) -> CoreResult<()> {
        self.guards.with(|g| {
            g.remove(target);
        });
        Ok(())
    }

    // --- one-shot property ops (Scripting API §6.4) ---

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

    // --- observable property ops (Scripting API §6.6) ---
    //
    // Deviation from Scripting API §6.6: `observe_property` returns a
    // pull-queue `Subscription` rather than registering a push-callback,
    // and `unobserve_property` returns `CoreResult<()>` rather than the
    // cancellation ack payload. Recorded in docs/design.md.

    /// Opens a long-lived subscription to property changes and returns a
    /// [`Subscription`] implementing
    /// [`futures_core::Stream<Item = Payload>`](futures_core::Stream). Drain
    /// it from a `while let Some(payload) = stream.next().await { ... }`
    /// loop. The wire-side guard is owned by the handle; call
    /// [`unobserve_property`](Self::unobserve_property) to release it
    /// explicitly, or drop the handle to release all of them.
    pub async fn observe_property(
        &self,
        name: &str,
        options: InteractionOptions,
    ) -> CoreResult<Subscription> {
        self.subscribe_op(
            AffordanceTarget::Property(name.into()),
            Operation::ObserveProperty,
            options,
        )
        .await
    }

    /// Releases the wire-side subscription for `name`. The previously
    /// returned [`Subscription`] stops receiving new samples but remains
    /// drainable for already-buffered ones. Idempotent: returns `Ok(())`
    /// even if no observation was active for `name`.
    pub async fn unobserve_property(
        &self,
        name: &str,
        _options: InteractionOptions,
    ) -> CoreResult<()> {
        self.drop_guard(&AffordanceTarget::Property(name.into()))
    }

    // --- action ops (Scripting API §6.5) ---

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

    // --- event ops (Scripting API §6.7) ---

    /// Opens a long-lived subscription to event `name` and returns a
    /// [`Subscription`] for draining pushed payloads. The wire-side guard is
    /// owned by the handle.
    pub async fn subscribe_event(
        &self,
        name: &str,
        options: InteractionOptions,
    ) -> CoreResult<Subscription> {
        self.subscribe_op(
            AffordanceTarget::Event(name.into()),
            Operation::SubscribeEvent,
            options,
        )
        .await
    }

    /// Releases the wire-side subscription for event `name`. Idempotent.
    pub async fn unsubscribe_event(
        &self,
        name: &str,
        _options: InteractionOptions,
    ) -> CoreResult<()> {
        self.drop_guard(&AffordanceTarget::Event(name.into()))
    }

    /// Subscribes to every event declared by the consumed Thing and returns
    /// a merged [`EventStream`] whose `Stream::Item` is
    /// `(EventName, Payload)`. The wire-side guards for every successfully
    /// opened event subscription are owned by the handle; dropping the
    /// handle releases them all.
    ///
    /// If the Thing declares no events, returns an empty `EventStream` that
    /// immediately yields `None`.
    ///
    /// Fail-fast: if any declared event has no compatible form or no matching
    /// binding, this method returns a structured selection error and opens no
    /// subscriptions. Existing subscriptions opened in the same call before
    /// the failure are cleaned up before returning.
    pub async fn subscribe_all_events(
        &self,
        options: InteractionOptions,
    ) -> CoreResult<EventStream> {
        let event_names: Vec<String> = self
            .consumed
            .thing_description()
            .events
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();

        let mut opened: Vec<(EventName, Subscription)> = Vec::with_capacity(event_names.len());
        for name in &event_names {
            let target = AffordanceTarget::Event(name.clone().into());
            match self
                .subscribe_op_with_guard(target.clone(), Operation::SubscribeEvent, options.clone())
                .await
            {
                Ok((sub, guard)) => {
                    self.guards.with(|g| {
                        g.insert(target, guard);
                    });
                    opened.push((EventName::new(name.clone()), sub));
                }
                Err(error) => {
                    // Fail-fast: clean up everything we just opened so the
                    // caller doesn't have to handle a partial result.
                    for (cleanup_name, _) in &opened {
                        let _ = self.drop_guard(&AffordanceTarget::Event(
                            cleanup_name.as_str().to_string().into(),
                        ));
                    }
                    return Err(error);
                }
            }
        }
        Ok(EventStream::new(opened))
    }

    // --- bulk property ops (Scripting API §6.5) ---

    /// Reads every declared property and returns a single
    /// [`InteractionOutput`] whose payload is a JSON object mapping
    /// property name to its raw payload.
    ///
    /// All individual reads fan out in parallel via `join_all`. The
    /// aggregated body is a JSON object `{"<name>": "<value>"}` where
    /// the value is the per-property payload body interpreted as a UTF-8
    /// string when valid, otherwise base64-encoded. The aggregated
    /// `content_type` is `application/json`.
    pub async fn read_all_properties(
        &self,
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        let names: Vec<String> = self
            .consumed
            .thing_description()
            .properties
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        self.read_multiple_properties(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
            options,
        )
        .await
    }

    /// Reads the named subset of properties in parallel and returns a single
    /// [`InteractionOutput`] in the same aggregated JSON format as
    /// [`read_all_properties`](Self::read_all_properties).
    ///
    /// All reads are issued concurrently via `join_all`; the first error
    /// short-circuits the aggregate (remaining in-flight reads are still
    /// driven to completion by `join_all`'s semantics — they're not
    /// cancelled, but their results are discarded on error).
    pub async fn read_multiple_properties(
        &self,
        names: &[&str],
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        use futures_util::future::join_all;

        // Build one future per named property. Each clone of `options` is
        // cheap (Payload is Arc-backed, BTreeMap is the only real copy).
        let reads: Vec<_> = names
            .iter()
            .map(|name| {
                let name = (*name).to_string();
                let opts = options.clone();
                async move {
                    let out = self
                        .invoke_op(
                            AffordanceTarget::Property(name.clone().into()),
                            Operation::ReadProperty,
                            opts,
                        )
                        .await?;
                    Ok::<_, CoreError>((name, out.data))
                }
            })
            .collect();

        let results = join_all(reads).await;
        // Surface the first error if any; otherwise build the aggregate.
        let mut entries: Vec<(String, Payload)> = Vec::with_capacity(results.len());
        for r in results {
            match r {
                Ok((name, Some(payload))) => entries.push((name, payload)),
                Ok((_, None)) => {} // empty payload → no entry
                Err(err) => return Err(err),
            }
        }
        Ok(InteractionOutput::with_data(aggregate_payloads(entries)))
    }

    /// Writes the named properties in parallel. All writes are issued
    /// concurrently via `join_all`; the method returns the first error
    /// encountered (remaining writes still complete — they're not
    /// cancelled, but their outcomes are folded into the aggregate result).
    ///
    /// For strict sequential semantics (write N only after N-1 succeeds),
    /// issue individual [`write_property`](Self::write_property) calls
    /// from application code.
    pub async fn write_multiple_properties(
        &self,
        entries: &BTreeMap<&str, Payload>,
        options: InteractionOptions,
    ) -> CoreResult<()> {
        use futures_util::future::join_all;

        let writes: Vec<_> = entries
            .iter()
            .map(|(name, payload)| {
                let name = (*name).to_string();
                let mut opts = options.clone();
                opts.data = Some(payload.clone());
                async move {
                    self.invoke_op(
                        AffordanceTarget::Property(name.into()),
                        Operation::WriteProperty,
                        opts,
                    )
                    .await
                }
            })
            .collect();

        let results = join_all(writes).await;
        for result in results {
            result?;
        }
        Ok(())
    }
}

impl Drop for ConsumedThingHandle {
    fn drop(&mut self) {
        // Drop every still-active wire-side guard so the underlying
        // transport subscriptions are torn down promptly. Without this,
        // bindings that don't watch the paired `SubscriptionSender`'s
        // lifetime would leak the wire resource until their owning session
        // is dropped.
        self.guards.with(|g| g.clear());
    }
}

/// Aggregates per-property payloads into a single JSON payload.
///
/// Each value is encoded as a JSON string when its body is valid UTF-8,
/// otherwise as a base64 string (prefixed with `"base64:"` so the receiver
/// can disambiguate). The output content type is `application/json`.
fn aggregate_payloads(entries: Vec<(String, Payload)>) -> Payload {
    let mut obj = String::from("{");
    for (i, (name, payload)) in entries.iter().enumerate() {
        if i > 0 {
            obj.push(',');
        }
        obj.push('"');
        json_escape_into(&mut obj, name);
        obj.push_str("\":\"");
        let body = payload.body.as_ref();
        match core::str::from_utf8(body) {
            Ok(text) => json_escape_into(&mut obj, text),
            Err(_) => {
                // Base64 (RFC 4648) over the standard alphabet. Inline so
                // we don't pull a dependency just for this aggregation.
                obj.push_str("base64:");
                base64_encode_into(&mut obj, body);
            }
        }
        obj.push('"');
    }
    obj.push('}');
    Payload::new(obj.into_bytes(), "application/json")
}

fn json_escape_into(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

fn base64_encode_into(out: &mut String, bytes: &[u8]) {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i] as usize;
        let b1 = bytes[i + 1] as usize;
        let b2 = bytes[i + 2] as usize;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHA[(n >> 18) & 0x3f] as char);
        out.push(ALPHA[(n >> 12) & 0x3f] as char);
        out.push(ALPHA[(n >> 6) & 0x3f] as char);
        out.push(ALPHA[n & 0x3f] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i] as usize;
        let n = b0 << 16;
        out.push(ALPHA[(n >> 18) & 0x3f] as char);
        out.push(ALPHA[(n >> 12) & 0x3f] as char);
        out.push_str("==");
    } else if rem == 2 {
        let b0 = bytes[i] as usize;
        let b1 = bytes[i + 1] as usize;
        let n = (b0 << 16) | (b1 << 8);
        out.push(ALPHA[(n >> 18) & 0x3f] as char);
        out.push(ALPHA[(n >> 12) & 0x3f] as char);
        out.push(ALPHA[(n >> 6) & 0x3f] as char);
        out.push('=');
    }
}
