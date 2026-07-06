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
    ActionCancelHandler, ActionHandler, ActionQueryHandler, AffordanceTarget, ConsumedThing,
    CoreError, CoreResult, EventName, EventStream, EventSubscribeHandler, EventUnsubscribeHandler,
    InteractionInput, InteractionOptions, InteractionOutput, Payload, PropertyObserveHandler,
    PropertyReadHandler, PropertyUnobserveHandler, PropertyWriteHandler, Subscription,
    SubscriptionGuard, ThingId, WotLock,
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
        Err(CoreError::UnsupportedOperation(format!(
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
        let (subscription, guard) = self.consumed.subscribe(target.clone(), operation, form, input).await?;
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
        self.consumed.subscribe(target, operation, form, input).await
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
    // cancellation ack payload. Recorded in docs/wot-compliance.md §9.

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
    /// Fail-fast: if any declared event has no compatible form or no
    /// matching binding, this method returns an `UnsupportedOperation`
    /// error and opens no subscriptions. Existing subscriptions opened in
    /// the same call before the failure are cleaned up before returning.
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
                .subscribe_op_with_guard(
                    target.clone(),
                    Operation::SubscribeEvent,
                    options.clone(),
                )
                .await
            {
                Ok((sub, guard)) => {
                    self.guards.with(|g| {
                        g.insert(target, guard);
                    });
                    opened.push((EventName::new(name.clone()), sub));
                }
                Err(err) => {
                    // Fail-fast: clean up everything we just opened so the
                    // caller doesn't have to handle a partial result.
                    for (cleanup_name, _) in &opened {
                        let _ = self.drop_guard(&AffordanceTarget::Event(
                            cleanup_name.as_str().to_string().into(),
                        ));
                    }
                    return Err(CoreError::UnsupportedOperation(format!(
                        "subscribe_all_events: event {name:?} could not be subscribed: {err}"
                    )));
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
    /// Each individual read runs sequentially against the selected form.
    /// The aggregated body is a JSON object `{"<name>": "<value>"}` where
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
        self.read_multiple_properties(&names.iter().map(String::as_str).collect::<Vec<_>>(), options)
            .await
    }

    /// Reads the named subset of properties and returns a single
    /// [`InteractionOutput`] in the same aggregated JSON format as
    /// [`read_all_properties`](Self::read_all_properties).
    pub async fn read_multiple_properties(
        &self,
        names: &[&str],
        options: InteractionOptions,
    ) -> CoreResult<InteractionOutput> {
        let mut entries: Vec<(String, Payload)> = Vec::with_capacity(names.len());
        for name in names {
            let out = self
                .invoke_op(
                    AffordanceTarget::Property((*name).into()),
                    Operation::ReadProperty,
                    options.clone(),
                )
                .await?;
            if let Some(payload) = out.data {
                entries.push(((*name).to_string(), payload));
            }
        }
        Ok(InteractionOutput::with_data(aggregate_payloads(entries)))
    }

    /// Writes the named properties. Each write runs sequentially against
    /// the selected form for that property. Returns `Ok(())` only after
    /// every write succeeds; on the first error, the remaining writes are
    /// skipped and the error is returned.
    pub async fn write_multiple_properties(
        &self,
        entries: &BTreeMap<&str, Payload>,
        options: InteractionOptions,
    ) -> CoreResult<()> {
        for (name, payload) in entries {
            let mut opts = options.clone();
            opts.data = Some(payload.clone());
            self.invoke_op(
                AffordanceTarget::Property((*name).into()),
                Operation::WriteProperty,
                opts,
            )
            .await?;
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
    const ALPHA: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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
