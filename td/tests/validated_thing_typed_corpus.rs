//! Non-production input corpus for WP-100-CONSUMER-VALIDATED-THING item 3.
//!
//! These tests establish typed input semantics using the current Thing model.
//! They do not stand in for a future normalized snapshot or prove that the
//! compatibility cursor accepts the corpus; that cursor is not admitted yet.

use std::collections::BTreeMap;

use clinkz_wot_td as td_crate;
use clinkz_wot_td::{
    context::Context,
    data_schema::DataSchema,
    data_type::Operation,
    validate::{Validate, ValidationLevel},
};
#[path = "support/typed_corpus_shared.rs"]
mod typed_corpus_shared;
use serde_json::{Value, json};
use typed_corpus_shared::{nested_schema_corpus, serializer_failure_thing, typed_corpus};

#[test]
fn nested_schema_corpus_is_typed_and_basic_valid() {
    let thing = nested_schema_corpus();
    let DataSchema::Object(root) = &thing.properties.as_ref().unwrap()["alpha"]._schema else {
        panic!("alpha must retain its Object variant");
    };
    assert_eq!(root.required.as_deref().unwrap(), ["samples"]);
    let DataSchema::Array(samples) = &root.properties.as_ref().unwrap()["samples"] else {
        panic!("samples must retain its Array variant");
    };
    let DataSchema::Object(reading) = &samples.items.as_ref().unwrap()[0] else {
        panic!("first item must retain its Object variant");
    };
    assert_eq!(reading.required.as_deref().unwrap(), ["label", "level"]);
    assert_eq!(
        root._context._extra_fields["ex:opaque"]
            .as_number()
            .unwrap()
            .as_str(),
        "1e+309"
    );
    thing.validate_with_level(ValidationLevel::Basic).unwrap();
}

#[test]
fn typed_fields_define_order_presence_and_content_without_json_roundtrip() {
    let thing = typed_corpus();
    assert_eq!(
        thing.id.as_ref().unwrap().as_str(),
        "urn:example:typed-corpus"
    );
    assert_eq!(
        thing.support.as_ref().unwrap().as_str(),
        "https://example.org/support"
    );
    assert_eq!(
        thing.base.as_ref().unwrap().as_str(),
        "https://example.org/things/"
    );
    assert_eq!(
        thing._metadata.titles.as_ref().unwrap().get("fr").unwrap(),
        "Corpus typé"
    );
    assert_eq!(
        thing._metadata.tags.as_deref(),
        Some(["Sensor".into(), "Thermometer".into()].as_slice())
    );
    assert_eq!(
        thing._metadata.description.as_deref(),
        Some("A fixed typed semantic corpus")
    );
    assert_eq!(
        thing.version.as_ref().unwrap()._extra_fields["ex:build"]["channel"],
        json!("evidence")
    );
    assert_eq!(thing.profile.as_ref().unwrap().len(), 2);
    assert_eq!(thing.schema_definitions.as_ref().unwrap().len(), 2);
    assert_eq!(thing.uri_variables.as_ref().unwrap().len(), 1);
    assert_eq!(thing.forms.as_ref().unwrap().len(), 0);

    let properties = thing.properties.as_ref().unwrap();
    assert_eq!(
        properties.keys().map(String::as_str).collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    assert!(properties["zeta"].observable);
    assert_eq!(
        properties["zeta"]
            ._interaction
            .uri_variables
            .as_ref()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["locale", "sample"]
    );
    assert!(!properties["alpha"].observable);
    assert!(properties["alpha"]._interaction.uri_variables.is_none());
    let forms = &properties["zeta"]._interaction.forms;
    assert_eq!(forms[0].href.as_str(), "zeta/first");
    assert_eq!(forms[1].href.as_str(), "zeta/second");
    assert_eq!(forms[0].content_type, "text/plain");
    assert_eq!(forms[1].content_type, "application/json");
    assert_eq!(forms[0].content_coding.as_deref(), Some("identity"));
    assert_eq!(
        forms[0].security.as_deref(),
        Some(["none".into(), "none_alt".into()].as_slice())
    );
    assert_eq!(
        forms[0].scopes.as_deref(),
        Some(["things.read".into(), "things.audit".into()].as_slice())
    );
    assert_eq!(
        forms[0].response.as_ref().unwrap().content_type,
        "application/cbor"
    );
    assert_eq!(
        forms[0].response.as_ref().unwrap()._extra_fields["ex:responseHint"]["compact"],
        json!(true)
    );
    let additional = forms[0].additional_responses.as_ref().unwrap();
    assert_eq!(additional.len(), 2);
    assert_eq!(
        additional[0].content_type.as_deref(),
        Some("application/problem+json")
    );
    assert_eq!(additional[0].schema.as_deref(), Some("mode"));
    assert!(additional[0].success);
    assert_eq!(additional[1].content_type, None);
    assert_eq!(additional[1].schema.as_deref(), Some("threshold"));
    assert!(!additional[1].success);
    assert_eq!(forms[0].subprotocol.as_deref(), Some("longpoll"));
    assert_eq!(forms[0]._extra_fields["ex:formHint"]["priority"], json!(1));
    assert_eq!(
        forms[0].op.as_deref(),
        Some([Operation::ReadProperty, Operation::WriteProperty].as_slice())
    );

    let actions = thing.actions.as_ref().unwrap();
    assert_eq!(
        actions.keys().map(String::as_str).collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    let rich_action = &actions["zeta"];
    assert_eq!(rich_action._metadata.title.as_deref(), Some("Calibrate"));
    assert_eq!(
        rich_action._metadata.tags.as_deref(),
        Some(["CalibrateAction".into(), "MaintenanceAction".into()].as_slice())
    );
    assert_eq!(
        rich_action
            ._interaction
            .uri_variables
            .as_ref()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["attempt", "channel"]
    );
    assert!(rich_action.input.is_some());
    assert!(rich_action.output.is_some());
    assert!(rich_action.safe);
    assert!(rich_action.idempotent);
    #[cfg(feature = "td2-preview")]
    assert_eq!(rich_action.synchronous, Some(true));
    assert_eq!(
        rich_action._interaction.forms[0].op.as_deref(),
        Some([Operation::InvokeAction, Operation::QueryAction].as_slice())
    );
    assert_eq!(
        rich_action._extra_fields["ex:actionHint"]["priority"],
        json!(2)
    );
    let minimal_action = &actions["alpha"];
    assert!(minimal_action._metadata.title.is_none());
    assert!(minimal_action._interaction.uri_variables.is_none());
    assert!(minimal_action.input.is_none());
    assert!(minimal_action.output.is_none());
    assert!(!minimal_action.safe);
    assert!(!minimal_action.idempotent);
    #[cfg(feature = "td2-preview")]
    assert_eq!(minimal_action.synchronous, None);

    let events = thing.events.as_ref().unwrap();
    assert_eq!(
        events.keys().map(String::as_str).collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    let rich_event = &events["zeta"];
    assert_eq!(rich_event._metadata.title.as_deref(), Some("Alarm"));
    assert_eq!(
        rich_event._metadata.tags.as_deref(),
        Some(["AlarmEvent".into(), "TelemetryEvent".into()].as_slice())
    );
    assert_eq!(
        rich_event
            ._interaction
            .uri_variables
            .as_ref()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["severity", "window"]
    );
    assert!(rich_event.subscription.is_some());
    assert!(rich_event.data.is_some());
    assert!(rich_event.data_response.is_some());
    assert!(rich_event.cancellation.is_some());
    assert_eq!(
        rich_event._interaction.forms[0].op.as_deref(),
        Some([Operation::SubscribeEvent, Operation::UnsubscribeEvent].as_slice())
    );
    assert_eq!(
        rich_event._extra_fields["ex:eventHint"]["priority"],
        json!(3)
    );
    let minimal_event = &events["alpha"];
    assert!(minimal_event._metadata.title.is_none());
    assert!(minimal_event._interaction.uri_variables.is_none());
    assert!(minimal_event.subscription.is_none());
    assert!(minimal_event.data.is_none());
    assert!(minimal_event.data_response.is_none());
    assert!(minimal_event.cancellation.is_none());

    let links = thing.links.as_ref().unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].href.as_str(), "docs/manual?edition=2");
    assert_eq!(links[0].content_type.as_deref(), Some("application/pdf"));
    assert_eq!(links[0].rel.as_deref(), Some("service-doc"));
    assert_eq!(
        links[0].anchor.as_ref().unwrap().as_str(),
        "https://example.org/things/typed-corpus"
    );
    assert_eq!(links[0].sizes.as_deref(), Some("16x16 32x32"));
    assert_eq!(
        links[0].hreflang.as_deref(),
        Some(["en".into(), "fr".into()].as_slice())
    );
    assert_eq!(links[0]._extra_fields["ex:linkHint"]["priority"], json!(4));
    assert_eq!(links[1].href.as_str(), "related/item");
    assert!(links[1].content_type.is_none());
    assert!(links[1].rel.is_none());
    assert!(links[1].anchor.is_none());
    assert!(links[1].sizes.is_none());
    assert!(links[1].hreflang.is_none());
    assert!(links[1]._extra_fields.is_empty());

    let payload = &thing._extra_fields["ex:payload"];
    assert_eq!(payload["flag"], json!(true));
    assert!(payload["empty"].is_null());
    assert_eq!(payload["items"][0], json!("first"));
    assert_eq!(payload["items"][1]["right"], json!("R"));
    assert_eq!(
        thing._extra_fields["ex:long"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        2_048
    );

    let mut same_map = thing.clone();
    let original = same_map.properties.take().unwrap();
    let mut reinserted = BTreeMap::new();
    for (name, property) in original.into_iter().rev() {
        reinserted.insert(name, property);
    }
    same_map.properties = Some(reinserted);
    assert_eq!(
        same_map, thing,
        "map insertion history is not typed meaning"
    );
}

#[test]
fn typed_mutations_distinguish_order_presence_and_map_association() {
    let thing = typed_corpus();

    let mut context_order = thing.clone();
    context_order.context = serde_json::from_str::<Context>(
        r##"[
        { "ex": "https://example.org/ns#" },
        "https://www.w3.org/2022/wot/td/v1.1",
        "https://example.org/extra-context"
    ]"##,
    )
    .unwrap();
    context_order
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(context_order.context, thing.context);
    assert_ne!(context_order, thing);

    let mut form_order = thing.clone();
    form_order
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms
        .swap(0, 1);
    form_order
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(form_order, thing, "Form original indices are semantic");

    let mut form_security_order = thing.clone();
    form_security_order
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[0]
        .security
        .as_mut()
        .unwrap()
        .swap(0, 1);
    form_security_order
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(
        form_security_order, thing,
        "Form security order is semantic"
    );

    let mut additional_response_order = thing.clone();
    additional_response_order
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[0]
        .additional_responses
        .as_mut()
        .unwrap()
        .swap(0, 1);
    additional_response_order
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(
        additional_response_order, thing,
        "additional response order is semantic"
    );

    let mut property_observable = thing.clone();
    property_observable
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        .observable = false;
    property_observable
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(
        property_observable, thing,
        "Property observable state is semantic"
    );

    let mut action_safe = thing.clone();
    action_safe
        .actions
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        .safe = false;
    action_safe
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(action_safe, thing, "Action safe state is semantic");

    let mut event_data = thing.clone();
    event_data
        .events
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        .data = None;
    event_data
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(event_data, thing, "Event data presence is semantic");

    let mut link_order = thing.clone();
    link_order.links.as_mut().unwrap().swap(0, 1);
    link_order
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(link_order, thing, "Link order is semantic");

    let mut absent_forms = thing.clone();
    absent_forms.forms = None;
    absent_forms
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(absent_forms, thing, "None differs from Some(empty)");

    let mut array_order = thing.clone();
    array_order._extra_fields.get_mut("ex:payload").unwrap()["items"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert_ne!(array_order, thing);

    let mut map_association = thing.clone();
    map_association._extra_fields.get_mut("ex:payload").unwrap()["flag"] = json!(false);
    assert_ne!(map_association, thing);

    let mut long_content = thing.clone();
    long_content
        ._extra_fields
        .insert("ex:long".into(), Value::String("λ".repeat(2_047)));
    assert_ne!(long_content, thing);

    let mut profile_order = thing.clone();
    profile_order.profile.as_mut().unwrap().swap(0, 1);
    profile_order
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(profile_order, thing, "profile order is semantic");

    let mut absent_version = thing.clone();
    absent_version.version = None;
    absent_version
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(absent_version, thing, "None differs from Some(version)");

    let mut schema_association = thing.clone();
    let definitions = schema_association.schema_definitions.as_mut().unwrap();
    let mode = definitions.remove("mode").unwrap();
    let threshold = definitions.remove("threshold").unwrap();
    definitions.insert("mode".into(), threshold);
    definitions.insert("threshold".into(), mode);
    schema_association
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert_ne!(
        schema_association, thing,
        "schema map associations are semantic"
    );
}

#[test]
fn basic_valid_typed_thing_can_fail_its_serializer() {
    let thing = serializer_failure_thing();
    assert!(!thing.context.has_wot_context());
    let error =
        serde_json::to_vec(&thing).expect_err("Context serializer requires an official URI");
    assert!(
        error
            .to_string()
            .contains("Context must contain at least one official WoT URI")
    );

    // The typed fields remain available after failed serialization. A future
    // compatibility normalizer must inspect these directly and accept this
    // Basic-valid input without trying to serialize it first.
    assert_eq!(
        thing.properties.as_ref().unwrap()["zeta"]
            ._interaction
            .forms[1]
            .href
            .as_str(),
        "zeta/second"
    );
    assert_eq!(
        thing._extra_fields["ex:payload"]["items"][0],
        json!("first")
    );
}
