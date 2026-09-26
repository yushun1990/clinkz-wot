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
    let forms = &properties["zeta"]._interaction.forms;
    assert_eq!(forms[0].href.as_str(), "zeta/first");
    assert_eq!(forms[1].href.as_str(), "zeta/second");
    assert_eq!(forms[0].content_type, "text/plain");
    assert_eq!(forms[1].content_type, "application/json");
    assert_eq!(
        forms[0].op.as_deref(),
        Some([Operation::ReadProperty, Operation::WriteProperty].as_slice())
    );

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
