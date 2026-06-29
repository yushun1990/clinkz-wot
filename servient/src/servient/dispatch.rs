use alloc::{format, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ActionCancelHandler, ActionHandler, ActionQueryHandler, CoreError, CoreResult, EventBroker,
    EventName, EventSink, EventSubscribeHandler, EventUnsubscribeHandler, InboundRequest,
    InboundResponse, InteractionInput, InteractionOutput, LocalThing, Payload,
    PropertyObserveHandler, PropertyReadHandler, PropertyUnobserveHandler, PropertyWriteHandler,
    ThingId,
};
use clinkz_wot_td::data_type::Operation;

use super::security::verify_inbound;
use super::{Servient, bulk};

/// A handler cloned out under the brief slot lock, ready to run with the slot
/// lock released (C7 reentrancy fix).
///
/// [`PreparedDispatch::prepare`] runs under the per-Thing `thing` lock and
/// captures the handler `Arc` plus its input; [`PreparedDispatch::run`] invokes
/// the handler outside that lock (only the driving-loop serialization lock may
/// remain held), so handler code may re-enter the Servient without self-deadlock.
pub(super) enum PreparedDispatch {
    Read(Arc<dyn PropertyReadHandler>, InteractionInput),
    Write(Arc<dyn PropertyWriteHandler>, InteractionInput),
    Invoke(Arc<dyn ActionHandler>, InteractionInput),
    Subscribe(Arc<dyn EventSubscribeHandler>, InteractionInput),
    Unsubscribe(Arc<dyn EventUnsubscribeHandler>, InteractionInput),
    Observe(Arc<dyn PropertyObserveHandler>, InteractionInput),
    Unobserve(Arc<dyn PropertyUnobserveHandler>, InteractionInput),
    /// Action query (W3C TD `queryaction`).
    ActionQuery(Arc<dyn ActionQueryHandler>, InteractionInput),
    /// Action cancel with a registered handler (W3C TD `cancelaction`).
    ActionCancel(Arc<dyn ActionCancelHandler>, InteractionInput),
    /// No unsubscribe handler registered — acknowledge the request inline.
    UnsubscribeAck,
    /// No observe handler registered — acknowledge the request inline.
    UnobserveAck,
    /// No cancel handler registered — acknowledge the request inline.
    ActionCancelAck,
    /// No observe handler registered — fall back to read + emit initial value.
    ObserveFallbackRead(Arc<dyn PropertyReadHandler>, InteractionInput),
    /// Fan out a `readallproperties` / `readmultipleproperties` request across
    /// the listed property read handlers and combine the results into a single
    /// JSON-object payload (W3C TD §6.3.3).
    BulkReadProperties(
        Vec<(alloc::string::String, Arc<dyn PropertyReadHandler>)>,
        InteractionInput,
    ),
    /// Fan out a `writeallproperties` / `writemultipleproperties` request
    /// across the listed property write handlers, each fed its slice of the
    /// JSON-object request payload (W3C TD §6.3.3).
    BulkWriteProperties(
        Vec<(
            alloc::string::String,
            Arc<dyn PropertyWriteHandler>,
            InteractionInput,
        )>,
    ),
    /// Fan out an `observeallproperties` request across the listed property
    /// observe handlers. Each handler emits through a per-property buffering
    /// sink so emissions route to the correct broker key (W3C TD §6.3.3).
    BulkObserveProperties(
        Vec<(alloc::string::String, Arc<dyn PropertyObserveHandler>)>,
        InteractionInput,
    ),
    /// Fan out an `unobserveallproperties` request across the listed property
    /// unobserve handlers (side-effect only, no streaming output).
    BulkUnobserveProperties(
        Vec<(
            alloc::string::String,
            Arc<dyn PropertyUnobserveHandler>,
            InteractionInput,
        )>,
    ),
    /// Fan out a `subscribeallevents` request across the listed event
    /// subscribe handlers. Each handler emits through a per-event buffering
    /// sink (W3C TD `subscribeallevents`).
    BulkSubscribeEvents(
        Vec<(alloc::string::String, Arc<dyn EventSubscribeHandler>)>,
        InteractionInput,
    ),
    /// Fan out an `unsubscribeallevents` request across the listed event
    /// unsubscribe handlers (side-effect only, W3C TD `unsubscribeallevents`).
    BulkUnsubscribeEvents(
        Vec<(
            alloc::string::String,
            Arc<dyn EventUnsubscribeHandler>,
            InteractionInput,
        )>,
    ),
    /// Fan out a `queryallactions` request across the listed action query
    /// handlers and combine the results into a single JSON-object payload
    /// (W3C TD §6.3.3).
    BulkQueryActions(
        Vec<(alloc::string::String, Arc<dyn ActionQueryHandler>)>,
        InteractionInput,
    ),
}

/// Output of [`PreparedDispatch::run`].
pub(super) struct DispatchResult {
    /// Interaction output (empty for ack / side-effect-only operations).
    pub(super) output: InteractionOutput,
    /// Per-affordance emissions for bulk streaming fan-out operations
    /// (`observeallproperties`, `subscribeallevents`). Each `(name, payloads)`
    /// pair is drained through the broker keyed by `(thing_id, name)`. Empty
    /// for single-affordance operations whose emissions go through the passed-in
    /// `sink`.
    pub(super) tagged_emissions: Vec<(alloc::string::String, Vec<Payload>)>,
}

/// [`EventSink`] that buffers emitted payloads for deferred fan-out.
///
/// Used by the inbound dispatch path to collect payloads emitted while the
/// per-Thing slot lock is held, so they can be pushed through the
/// [`EventBroker`] after the lock is released.
pub(super) struct BufferingEventSink<'a> {
    pub(super) buffer: &'a mut Vec<Payload>,
}

impl<'a> EventSink for BufferingEventSink<'a> {
    fn emit(&mut self, payload: Payload) -> CoreResult<()> {
        self.buffer.push(payload);
        Ok(())
    }
}

impl PreparedDispatch {
    /// Resolves the affordance + clones the handler `Arc` under the slot lock.
    pub(super) fn prepare(
        thing: &LocalThing,
        target: &clinkz_wot_core::AffordanceTarget,
        operation: Operation,
        input: InteractionInput,
    ) -> CoreResult<Self> {
        match (target, operation) {
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::ReadProperty) => {
                thing.ensure_property_affordance(name)?;
                let handler = thing.read_handler(name).ok_or(CoreError::MissingHandler {
                    target: target.clone(),
                    operation,
                })?;
                Ok(Self::Read(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::WriteProperty) => {
                thing.ensure_property_affordance(name)?;
                let handler = thing.write_handler(name).ok_or(CoreError::MissingHandler {
                    target: target.clone(),
                    operation,
                })?;
                Ok(Self::Write(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Action(name), Operation::InvokeAction) => {
                thing.ensure_action_affordance(name)?;
                let handler = thing
                    .action_handler(name)
                    .ok_or(CoreError::MissingHandler {
                        target: target.clone(),
                        operation,
                    })?;
                Ok(Self::Invoke(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Event(name), Operation::SubscribeEvent) => {
                thing.ensure_event_affordance(name)?;
                let handler = thing
                    .subscribe_handler(name)
                    .ok_or(CoreError::MissingHandler {
                        target: target.clone(),
                        operation,
                    })?;
                Ok(Self::Subscribe(handler, input))
            }
            (clinkz_wot_core::AffordanceTarget::Event(name), Operation::UnsubscribeEvent) => {
                thing.ensure_event_affordance(name)?;
                match thing.unsubscribe_handler(name) {
                    Some(handler) => Ok(Self::Unsubscribe(handler, input)),
                    None => Ok(Self::UnsubscribeAck),
                }
            }
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::ObserveProperty) => {
                thing.ensure_property_affordance(name)?;
                match thing.observe_handler(name) {
                    Some(handler) => Ok(Self::Observe(handler, input)),
                    None => {
                        let handler =
                            thing.read_handler(name).ok_or(CoreError::MissingHandler {
                                target: target.clone(),
                                operation,
                            })?;
                        Ok(Self::ObserveFallbackRead(handler, input))
                    }
                }
            }
            (clinkz_wot_core::AffordanceTarget::Property(name), Operation::UnobserveProperty) => {
                thing.ensure_property_affordance(name)?;
                match thing.unobserve_handler(name) {
                    Some(handler) => Ok(Self::Unobserve(handler, input)),
                    None => Ok(Self::UnobserveAck),
                }
            }
            (clinkz_wot_core::AffordanceTarget::Action(name), Operation::QueryAction) => {
                thing.ensure_action_affordance(name)?;
                let handler =
                    thing
                        .action_query_handler(name)
                        .ok_or(CoreError::MissingHandler {
                            target: target.clone(),
                            operation,
                        })?;
                Ok(Self::ActionQuery(handler, input))
            }
            // Action cancel (W3C TD `cancelaction`).
            (clinkz_wot_core::AffordanceTarget::Action(name), Operation::CancelAction) => {
                thing.ensure_action_affordance(name)?;
                match thing.action_cancel_handler(name) {
                    Some(handler) => Ok(Self::ActionCancel(handler, input)),
                    None => Ok(Self::ActionCancelAck),
                }
            }
            // Bulk property reads (W3C TD §6.3.3). Fan out across the property
            // read handlers and combine the results into a single JSON-object
            // payload. `readallproperties` targets every declared property;
            // `readmultipleproperties` targets the names carried by the request
            // payload as a JSON array (e.g. `["temp","hum"]`).
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::ReadAllProperties) => {
                let names = thing
                    .thing_description()
                    .properties
                    .as_ref()
                    .map(|props| props.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                Ok(Self::BulkReadProperties(
                    bulk::collect_read_handlers(thing, &names, Operation::ReadAllProperties)?,
                    input,
                ))
            }
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::ReadMultipleProperties) => {
                let names = bulk::parse_read_multiple_names(&input)?;
                Ok(Self::BulkReadProperties(
                    bulk::collect_read_handlers(thing, &names, Operation::ReadMultipleProperties)?,
                    input,
                ))
            }
            // Bulk property writes (W3C TD §6.3.3). The request payload is a
            // JSON object mapping property names to their new values. Each
            // (name, value) pair is dispatched to its write handler with the
            // value serialized as the per-property interaction input payload.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::WriteAllProperties)
            | (clinkz_wot_core::AffordanceTarget::Thing, Operation::WriteMultipleProperties) => Ok(
                Self::BulkWriteProperties(bulk::collect_write_handlers(thing, input, operation)?),
            ),
            // Bulk observe (W3C TD §6.3.3). Fan out across the property
            // observe handlers for every observable property.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::ObserveAllProperties) => {
                let names = bulk::observable_property_names(thing.thing_description());
                Ok(Self::BulkObserveProperties(
                    bulk::collect_observe_handlers(thing, &names, Operation::ObserveAllProperties)?,
                    input,
                ))
            }
            // Bulk unobserve (W3C TD §6.3.3). Fan out across the property
            // unobserve handlers; unobserved properties without a handler are
            // acked.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::UnobserveAllProperties) => {
                let names = bulk::observable_property_names(thing.thing_description());
                Ok(Self::BulkUnobserveProperties(
                    bulk::collect_unobserve_handlers(thing, &names, &input),
                ))
            }
            // Bulk subscribe (`subscribeallevents`) and unsubscribe
            // (`unsubscribeallevents`) event meta-operations.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::SubscribeAllEvents) => {
                let names = bulk::event_names(thing.thing_description());
                Ok(Self::BulkSubscribeEvents(
                    bulk::collect_subscribe_handlers(thing, &names, Operation::SubscribeAllEvents)?,
                    input,
                ))
            }
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::UnsubscribeAllEvents) => {
                let names = bulk::event_names(thing.thing_description());
                Ok(Self::BulkUnsubscribeEvents(
                    bulk::collect_unsubscribe_handlers(thing, &names, &input),
                ))
            }
            // Bulk query actions (W3C TD §6.3.3). Fan out across action query
            // handlers and combine results. Actions without a query handler are
            // skipped.
            (clinkz_wot_core::AffordanceTarget::Thing, Operation::QueryAllActions) => {
                let names = bulk::action_names(thing.thing_description());
                Ok(Self::BulkQueryActions(
                    bulk::collect_action_query_handlers(thing, &names),
                    input,
                ))
            }
            _ => Err(CoreError::UnsupportedOperation(format!(
                "Inbound dispatch does not support {:?} on {:?}",
                operation, target
            ))),
        }
    }

    /// Invokes the handler outside the slot lock. Single-affordance emissions go
    /// through `sink`; bulk streaming fan-out emissions are returned as tagged
    /// `(affordance_name, payloads)` pairs in [`DispatchResult`].
    pub(super) fn run(self, sink: &mut dyn EventSink) -> CoreResult<DispatchResult> {
        let empty_emissions = Vec::new();
        match self {
            Self::Read(handler, input) => Ok(DispatchResult {
                output: handler.read(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Write(handler, input) => Ok(DispatchResult {
                output: handler.write(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Invoke(handler, input) => Ok(DispatchResult {
                output: handler.invoke(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Subscribe(handler, input) => Ok(DispatchResult {
                output: handler.subscribe(input, sink)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Unsubscribe(handler, input) => Ok(DispatchResult {
                output: handler.unsubscribe(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Observe(handler, input) => Ok(DispatchResult {
                output: handler.observe(input, sink)?,
                tagged_emissions: empty_emissions,
            }),
            Self::Unobserve(handler, input) => Ok(DispatchResult {
                output: handler.unobserve(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::ActionQuery(handler, input) => Ok(DispatchResult {
                output: handler.query(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::ActionCancel(handler, input) => Ok(DispatchResult {
                output: handler.cancel(input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::UnsubscribeAck | Self::UnobserveAck => Ok(DispatchResult {
                output: InteractionOutput::empty(),
                tagged_emissions: empty_emissions,
            }),
            Self::ActionCancelAck => Ok(DispatchResult {
                output: InteractionOutput::empty(),
                tagged_emissions: empty_emissions,
            }),
            Self::ObserveFallbackRead(handler, input) => {
                let output = handler.read(input)?;
                if let Some(ref payload) = output.payload {
                    let _ = sink.emit(payload.clone());
                }
                Ok(DispatchResult {
                    output,
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkReadProperties(entries, input) => Ok(DispatchResult {
                output: bulk::run_bulk_read(entries, input)?,
                tagged_emissions: empty_emissions,
            }),
            Self::BulkWriteProperties(entries) => {
                for (_name, handler, value_input) in entries {
                    handler.write(value_input)?;
                }
                Ok(DispatchResult {
                    output: InteractionOutput::empty(),
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkObserveProperties(entries, input) => {
                bulk::run_bulk_streaming(entries, input, |handler, input, sink| {
                    handler.observe(input, sink)
                })
            }
            Self::BulkUnobserveProperties(entries) => {
                for (_name, handler, value_input) in entries {
                    handler.unobserve(value_input)?;
                }
                Ok(DispatchResult {
                    output: InteractionOutput::empty(),
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkSubscribeEvents(entries, input) => {
                bulk::run_bulk_streaming(entries, input, |handler, input, sink| {
                    handler.subscribe(input, sink)
                })
            }
            Self::BulkUnsubscribeEvents(entries) => {
                for (_name, handler, value_input) in entries {
                    handler.unsubscribe(value_input)?;
                }
                Ok(DispatchResult {
                    output: InteractionOutput::empty(),
                    tagged_emissions: empty_emissions,
                })
            }
            Self::BulkQueryActions(entries, input) => Ok(DispatchResult {
                output: bulk::run_bulk_query_actions(entries, input)?,
                tagged_emissions: empty_emissions,
            }),
        }
    }
}

impl<D> Servient<D> {
    /// Synchronous inbound dispatch (baseline §4).
    ///
    /// Reads lock-free shared state (registries, broker) directly; the handler
    /// runs through `registry.dispatch`, which holds only its own per-Thing slot
    /// lock — never the outer Servient lock.
    pub(super) fn dispatch_inbound(&self, request: InboundRequest) -> InboundResponse {
        let correlation = request.correlation.clone();

        let resolved_security = match self.shared.exposed_registry.resolve_inbound_security(
            request.thing_id.as_str(),
            &request.target,
            request.operation,
        ) {
            Some(Ok(resolved_security)) => resolved_security,
            Some(Err(core_error)) => {
                return InboundResponse::error(correlation, core_error);
            }
            None => {
                return InboundResponse::error(
                    correlation,
                    CoreError::InboundDispatch(format!("Unknown Thing id '{}'", request.thing_id)),
                );
            }
        };

        let principal = match verify_inbound(
            &self.shared.security_providers,
            &request,
            &resolved_security,
        ) {
            Ok(principal) => principal,
            Err(core_err) => return InboundResponse::error(correlation, core_err),
        };

        // Move the dispatch-relevant fields out of `request` so `input` is
        // moved (not cloned) into the handler. `correlation` was already
        // retained above; `auth` is no longer needed after verification.
        let thing_id = request.thing_id;
        let target = request.target;
        let operation = request.operation;
        let mut input = request.input;
        input.principal = Some(principal);

        let registry = Arc::clone(&self.shared.exposed_registry);
        let broker = &self.shared.event_broker;

        // Clone the handler `Arc` out under a brief slot lock and invoke it
        // with the slot lock released (held only by the driving-loop
        // serialization lock `sync_lock`), so the handler may re-enter the
        // Servient for the same Thing without self-deadlock (C7). Emitted
        // payloads are buffered under the run and drained through the broker
        // afterwards.
        let output: Option<CoreResult<InteractionOutput>> =
            registry.slot_for(thing_id.as_str()).map(|slot| {
                slot.with_sync_serialization(|| {
                    let mut emitted: Vec<Payload> = Vec::new();
                    let prepared = slot
                        .with_thing(|thing| {
                            PreparedDispatch::prepare(thing, &target, operation, input)
                        })
                        .ok_or(CoreError::MissingHandler {
                            target: target.clone(),
                            operation,
                        })?;
                    let prepared = prepared?;
                    let result = prepared.run(&mut BufferingEventSink {
                        buffer: &mut emitted,
                    })?;
                    drain_emitted(broker, &thing_id, &target, emitted);
                    drain_tagged_emissions(broker, &thing_id, result.tagged_emissions);
                    Ok(result.output)
                })
            });

        match output {
            Some(Ok(out)) => InboundResponse::new(out, correlation),
            Some(Err(core_err)) => InboundResponse::error(correlation, core_err),
            None => InboundResponse::error(
                correlation,
                CoreError::MissingHandler {
                    target: target.clone(),
                    operation,
                },
            ),
        }
    }
}

/// Returns the broker event-name key for an affordance target, if it is one
/// that emits through the broker (events and observed properties).
pub(super) fn event_name_for_target(target: &clinkz_wot_core::AffordanceTarget) -> Option<&str> {
    match target {
        clinkz_wot_core::AffordanceTarget::Event(name)
        | clinkz_wot_core::AffordanceTarget::Property(name) => Some(&**name),
        _ => None,
    }
}

/// Drains buffered payloads through the broker, keyed by the request target.
///
/// Only subscribe/observe operations emit; for any other target the buffer is
/// expected to be empty and this is a no-op.
pub(super) fn drain_emitted(
    broker: &EventBroker,
    thing_id: &ThingId,
    target: &clinkz_wot_core::AffordanceTarget,
    emitted: Vec<Payload>,
) {
    if emitted.is_empty() {
        return;
    }
    let Some(name) = event_name_for_target(target) else {
        return;
    };
    let event = EventName::from(name);
    for payload in emitted {
        let _ = broker.publish(thing_id, &event, &payload);
    }
}

/// Drains per-affordance tagged emissions through the broker.
///
/// Each `(name, payloads)` pair is published to the broker under
/// `(thing_id, name)` so the correct per-affordance `PublisherSink` receives
/// the payloads.
pub(super) fn drain_tagged_emissions(
    broker: &EventBroker,
    thing_id: &ThingId,
    tagged: Vec<(alloc::string::String, Vec<Payload>)>,
) {
    for (name, payloads) in tagged {
        let event = EventName::from(name);
        for payload in payloads {
            let _ = broker.publish(thing_id, &event, &payload);
        }
    }
}
