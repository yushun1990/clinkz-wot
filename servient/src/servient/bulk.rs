use alloc::{borrow::Cow, format, string::String, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ActionQueryHandler, AffordanceTarget, CoreError, CoreResult, EventSink, EventSubscribeHandler,
    EventUnsubscribeHandler, InteractionInput, InteractionOutput, LocalThing, Payload,
    PropertyObserveHandler, PropertyReadHandler, PropertyUnobserveHandler, PropertyWriteHandler,
};
use clinkz_wot_td::{data_type::Operation, thing::Thing};

use super::dispatch::{BufferingEventSink, DispatchResult};

/// Default content type used when assembling or parsing bulk payloads.
const BULK_CONTENT_TYPE: &str = "application/json";

/// A single `(name, write handler, per-property input)` entry prepared for a
/// bulk write dispatch.
pub(super) type PreparedBulkWriteEntry = (String, Arc<dyn PropertyWriteHandler>, InteractionInput);

/// Returns the content type carried by the bulk request payload, falling back
/// to the WoT default when none is declared.
pub(super) fn bulk_content_type<'a>(input: &'a InteractionInput) -> Cow<'a, str> {
    match input.payload.as_ref() {
        Some(payload) if !payload.content_type.is_empty() => {
            Cow::Borrowed(payload.content_type.as_str())
        }
        _ => Cow::Borrowed(BULK_CONTENT_TYPE),
    }
}

/// Collects `(name, read handler)` pairs for the listed property names.
///
/// A property without a registered read handler is skipped rather than failing
/// the whole bulk request, matching the tolerant fan-out semantics of
/// [`ExposedThingHandle::read_all_properties`]. Returns `MissingHandler` only
/// when no listed property has a handler at all.
pub(super) fn collect_read_handlers(
    thing: &LocalThing,
    names: &[String],
    operation: Operation,
) -> CoreResult<Vec<(String, Arc<dyn PropertyReadHandler>)>> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.read_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    if entries.is_empty() {
        return Err(CoreError::MissingHandler {
            target: AffordanceTarget::Thing,
            operation,
        });
    }
    Ok(entries)
}

/// Parses a `readmultipleproperties` request payload (a JSON array of property
/// names, e.g. `["temp","hum"]`) into an owned name list.
///
/// When the payload is missing or not a JSON array, an empty name list is
/// returned so `collect_read_handlers` surfaces a clear `MissingHandler` error
/// instead of a confusing deserialization failure.
pub(super) fn parse_read_multiple_names(input: &InteractionInput) -> CoreResult<Vec<String>> {
    let Some(payload) = input.payload.as_ref() else {
        return Ok(Vec::new());
    };
    let names: Vec<String> = serde_json::from_slice(payload.body.as_ref()).map_err(|err| {
        CoreError::InvalidInteraction(format!(
            "readmultipleproperties request payload is not a JSON array of names: {err}"
        ))
    })?;
    Ok(names)
}

/// Collects `(name, write handler, per-property input)` triples for a bulk
/// write request.
///
/// The request payload is a JSON object mapping property names to their new
/// values. Each value is re-serialized into a standalone
/// [`clinkz_wot_core::Payload`] and wrapped in an [`InteractionInput`] that
/// preserves the caller's URI variables and security metadata.
pub(super) fn collect_write_handlers(
    thing: &LocalThing,
    mut input: InteractionInput,
    operation: Operation,
) -> CoreResult<Vec<PreparedBulkWriteEntry>> {
    let content_type = bulk_content_type(&input);
    let map: serde_json::Map<String, serde_json::Value> = {
        let Some(payload) = input.payload.as_ref() else {
            return Err(CoreError::InvalidInteraction(format!(
                "{} request payload is missing",
                "writeallproperties/writemultipleproperties"
            )));
        };
        serde_json::from_slice(payload.body.as_ref()).map_err(|err| {
            CoreError::InvalidInteraction(format!(
                "bulk write request payload is not a JSON object: {err}"
            ))
        })?
    };

    // Collect (name, handler, body) tuples without touching the shared
    // `parameters` / `principal` / `security_metadata` fields. This decouples
    // payload serialization from the per-entry `InteractionInput` construction
    // so the shared fields can be *moved* into the last entry below.
    let mut pairs: Vec<(String, Arc<dyn PropertyWriteHandler>, Vec<u8>)> = Vec::new();
    for (name, value) in map {
        let Some(handler) = thing.write_handler(&name) else {
            continue;
        };
        let body = serde_json::to_vec(&value).map_err(|err| {
            CoreError::InvalidInteraction(format!(
                "failed to serialize bulk write value for '{name}': {err}"
            ))
        })?;
        pairs.push((name, handler, body));
    }

    if pairs.is_empty() {
        return Err(CoreError::MissingHandler {
            target: AffordanceTarget::Thing,
            operation,
        });
    }

    // Build the per-entry `InteractionInput`s. The shared fields
    // (`parameters`, `principal`, `security_metadata`) are identical across
    // every entry, so for all but the last entry we clone them, and for the
    // last entry we *move* them out of `input` — saving one full
    // `Principal::clone` (which clones `PrincipalId` + a `scopes` `Vec`) and
    // two `BTreeMap::clone`s that would otherwise run N times instead of N-1.
    let last = pairs.pop();
    let mut entries = Vec::with_capacity(pairs.len() + 1);
    for (name, handler, body) in pairs {
        entries.push((
            name,
            handler,
            InteractionInput {
                payload: Some(Payload::new(body, content_type.as_ref())),
                parameters: input.parameters.clone(),
                principal: input.principal.clone(),
                security_metadata: input.security_metadata.clone(),
            },
        ));
    }
    if let Some((name, handler, body)) = last {
        // Move the shared fields into the final entry instead of cloning.
        input.payload = Some(Payload::new(body, content_type.as_ref()));
        entries.push((name, handler, input));
    }

    Ok(entries)
}

/// Runs a bulk read, combining each handler's output payload into a single
/// JSON-object response keyed by property name.
///
/// Each handler output is parsed as a JSON value; non-JSON payloads are wrapped
/// in a JSON string so the combined object stays valid JSON. An empty handler
/// output contributes a JSON `null` entry.
pub(super) fn run_bulk_read(
    mut entries: Vec<(String, Arc<dyn PropertyReadHandler>)>,
    input: InteractionInput,
) -> CoreResult<InteractionOutput> {
    let mut combined = serde_json::Map::new();
    // Move the last input instead of cloning it (saves one full
    // InteractionInput clone per bulk read).
    let last_entry = entries.pop();
    for (name, handler) in entries {
        let output = handler.read(input.clone())?;
        let value = match output.payload {
            Some(payload) if !payload.body.is_empty() => {
                serde_json::from_slice::<serde_json::Value>(payload.body.as_ref()).unwrap_or_else(
                    |_| {
                        serde_json::Value::String(
                            alloc::string::String::from_utf8_lossy(payload.body.as_ref())
                                .into_owned(),
                        )
                    },
                )
            }
            _ => serde_json::Value::Null,
        };
        combined.insert(name, value);
    }
    if let Some((name, handler)) = last_entry {
        let output = handler.read(input)?;
        let value = match output.payload {
            Some(payload) if !payload.body.is_empty() => {
                serde_json::from_slice::<serde_json::Value>(payload.body.as_ref()).unwrap_or_else(
                    |_| {
                        serde_json::Value::String(
                            alloc::string::String::from_utf8_lossy(payload.body.as_ref())
                                .into_owned(),
                        )
                    },
                )
            }
            _ => serde_json::Value::Null,
        };
        combined.insert(name, value);
    }

    let body = serde_json::to_vec(&serde_json::Value::Object(combined)).map_err(|err| {
        CoreError::InvalidInteraction(format!("failed to serialize bulk read response: {err}"))
    })?;
    Ok(InteractionOutput::with_payload(Payload::new(
        body,
        BULK_CONTENT_TYPE,
    )))
}

/// Returns the names of all observable properties declared in the TD.
pub(super) fn observable_property_names(thing: &Thing) -> Vec<String> {
    thing
        .properties
        .as_ref()
        .map(|props| {
            props
                .iter()
                .filter(|(_, p)| p.observable)
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the names of all events declared in the TD.
pub(super) fn event_names(thing: &Thing) -> Vec<String> {
    thing
        .events
        .as_ref()
        .map(|events| events.keys().cloned().collect())
        .unwrap_or_default()
}

/// Returns the names of all actions declared in the TD.
pub(super) fn action_names(thing: &Thing) -> Vec<String> {
    thing
        .actions
        .as_ref()
        .map(|actions| actions.keys().cloned().collect())
        .unwrap_or_default()
}

/// Collects `(name, observe handler)` pairs for the listed property names.
///
/// A property without a registered observe handler is skipped. Returns
/// `MissingHandler` only when no listed property has a handler at all.
pub(super) fn collect_observe_handlers(
    thing: &LocalThing,
    names: &[String],
    operation: Operation,
) -> CoreResult<Vec<(String, Arc<dyn PropertyObserveHandler>)>> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.observe_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    if entries.is_empty() {
        return Err(CoreError::MissingHandler {
            target: AffordanceTarget::Thing,
            operation,
        });
    }
    Ok(entries)
}

/// Collects `(name, unobserve handler, input)` triples for the listed property
/// names. Properties without an unobserve handler produce no entry (the inbound
/// dispatcher acks those inline).
pub(super) fn collect_unobserve_handlers(
    thing: &LocalThing,
    names: &[String],
    input: &InteractionInput,
) -> Vec<(String, Arc<dyn PropertyUnobserveHandler>, InteractionInput)> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.unobserve_handler(name) {
            entries.push((name.clone(), handler, input.clone()));
        }
    }
    entries
}

/// Collects `(name, subscribe handler)` pairs for the listed event names.
pub(super) fn collect_subscribe_handlers(
    thing: &LocalThing,
    names: &[String],
    operation: Operation,
) -> CoreResult<Vec<(String, Arc<dyn EventSubscribeHandler>)>> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.subscribe_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    if entries.is_empty() {
        return Err(CoreError::MissingHandler {
            target: AffordanceTarget::Thing,
            operation,
        });
    }
    Ok(entries)
}

/// Collects `(name, unsubscribe handler, input)` triples for the listed event
/// names.
pub(super) fn collect_unsubscribe_handlers(
    thing: &LocalThing,
    names: &[String],
    input: &InteractionInput,
) -> Vec<(String, Arc<dyn EventUnsubscribeHandler>, InteractionInput)> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.unsubscribe_handler(name) {
            entries.push((name.clone(), handler, input.clone()));
        }
    }
    entries
}

/// Collects `(name, query handler)` pairs for the listed action names.
pub(super) fn collect_action_query_handlers(
    thing: &LocalThing,
    names: &[String],
) -> Vec<(String, Arc<dyn ActionQueryHandler>)> {
    let mut entries = Vec::new();
    for name in names {
        if let Some(handler) = thing.action_query_handler(name) {
            entries.push((name.clone(), handler));
        }
    }
    entries
}

/// Runs a bulk streaming fan-out (`observeallproperties` /
/// `subscribeallevents`), invoking each handler through a per-affordance
/// buffering sink so emissions are tagged with the correct affordance name for
/// broker routing.
pub(super) fn run_bulk_streaming<H>(
    mut entries: Vec<(String, Arc<H>)>,
    input: InteractionInput,
    invoke: fn(&Arc<H>, InteractionInput, &mut dyn EventSink) -> CoreResult<InteractionOutput>,
) -> CoreResult<DispatchResult>
where
    H: ?Sized,
{
    let mut tagged_emissions: Vec<(String, Vec<Payload>)> = Vec::new();
    let last_entry = entries.pop();
    for (name, handler) in entries {
        let mut emitted: Vec<Payload> = Vec::new();
        invoke(
            &handler,
            input.clone(),
            &mut BufferingEventSink {
                buffer: &mut emitted,
            },
        )?;
        if !emitted.is_empty() {
            tagged_emissions.push((name, emitted));
        }
    }
    if let Some((name, handler)) = last_entry {
        let mut emitted: Vec<Payload> = Vec::new();
        invoke(
            &handler,
            input,
            &mut BufferingEventSink {
                buffer: &mut emitted,
            },
        )?;
        if !emitted.is_empty() {
            tagged_emissions.push((name, emitted));
        }
    }
    Ok(DispatchResult {
        output: InteractionOutput::empty(),
        tagged_emissions,
    })
}

/// Runs a bulk action query (`queryallactions`), combining each handler's
/// output payload into a single JSON-object response keyed by action name.
/// When no query handlers are registered, returns an empty JSON object.
pub(super) fn run_bulk_query_actions(
    mut entries: Vec<(String, Arc<dyn ActionQueryHandler>)>,
    input: InteractionInput,
) -> CoreResult<InteractionOutput> {
    let mut combined = serde_json::Map::new();
    let last_entry = entries.pop();
    for (name, handler) in entries {
        let output = handler.query(input.clone())?;
        let value = match output.payload {
            Some(payload) if !payload.body.is_empty() => {
                serde_json::from_slice::<serde_json::Value>(payload.body.as_ref()).unwrap_or_else(
                    |_| {
                        serde_json::Value::String(
                            alloc::string::String::from_utf8_lossy(payload.body.as_ref())
                                .into_owned(),
                        )
                    },
                )
            }
            _ => serde_json::Value::Null,
        };
        combined.insert(name, value);
    }
    if let Some((name, handler)) = last_entry {
        let output = handler.query(input)?;
        let value = match output.payload {
            Some(payload) if !payload.body.is_empty() => {
                serde_json::from_slice::<serde_json::Value>(payload.body.as_ref()).unwrap_or_else(
                    |_| {
                        serde_json::Value::String(
                            alloc::string::String::from_utf8_lossy(payload.body.as_ref())
                                .into_owned(),
                        )
                    },
                )
            }
            _ => serde_json::Value::Null,
        };
        combined.insert(name, value);
    }

    let body = serde_json::to_vec(&serde_json::Value::Object(combined)).map_err(|err| {
        CoreError::InvalidInteraction(format!(
            "failed to serialize queryallactions response: {err}"
        ))
    })?;
    Ok(InteractionOutput::with_payload(Payload::new(
        body,
        BULK_CONTENT_TYPE,
    )))
}
