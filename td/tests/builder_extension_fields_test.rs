use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance},
    data_schema::ContextHelper as DataSchemaContextHelper,
    data_type::{AdditionalExpectedResponse, ExpectedResponse, ExtensionMap, VersionInfo},
    form::Form,
    link::Link,
    security_scheme::{ContextHelper as SecurityContextHelper, NoSecurityScheme, SecurityScheme},
    thing::Thing,
};
use serde_json::{json, Value};

fn extension_map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> ExtensionMap {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn field(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("missing field {key}"))
        .clone()
}

#[test]
fn thing_builder_sets_extension_fields() {
    let thing = Thing::builder("Demo Thing")
        .security(SecurityScheme::NoSec(
            NoSecurityScheme::builder()
                .build()
                .expect("security should build"),
        ))
        .extra_field("cz:binding", json!({ "transport": "zenoh" }))
        .extra_fields(extension_map([("cz:owner", json!("platform"))]))
        .version(
            VersionInfo {
                instance: "1.0.0".to_string(),
                model: None,
                _extra_fields: Default::default(),
            }
            .extra_field("cz:versionTag", json!("stable")),
        )
        .build()
        .expect("thing should build");

    let value = serde_json::to_value(thing).expect("thing should serialize");
    assert_eq!(field(&value, "cz:binding"), json!({ "transport": "zenoh" }));
    assert_eq!(field(&value, "cz:owner"), json!("platform"));
    assert_eq!(field(&value["version"], "cz:versionTag"), json!("stable"));
}

#[test]
fn form_and_response_builders_set_extension_fields() {
    let form = Form::builder("/properties/temperature")
        .extra_field("cz:testHint", json!("application/json"))
        .response(
            ExpectedResponse::new("application/cbor".to_string())
                .extra_field("cz:responseHint", json!("compact")),
        )
        .additional_response(
            AdditionalExpectedResponse::new("application/json".to_string())
                .extra_field("cz:errorCode", json!(400)),
        )
        .build()
        .expect("form should build");

    let value = serde_json::to_value(form).expect("form should serialize");
    assert_eq!(field(&value, "cz:testHint"), json!("application/json"));
    assert_eq!(
        field(&value["response"], "cz:responseHint"),
        json!("compact")
    );
    assert_eq!(
        field(&value["additionalResponses"][0], "cz:errorCode"),
        json!(400)
    );
}

#[test]
fn link_and_schema_builders_set_extension_fields() {
    let link = Link::builder("/things/parent")
        .extra_field("cz:linkRole", json!("registry-parent"))
        .build()
        .expect("link should build");
    let link_value = serde_json::to_value(link).expect("link should serialize");
    assert_eq!(field(&link_value, "cz:linkRole"), json!("registry-parent"));

    let schema = clinkz_wot_td::data_schema::DataSchema::string()
        .data_type("string")
        .extra_field("cz:semanticType", json!("temperature-unit"))
        .build();
    let schema_value = serde_json::to_value(schema).expect("schema should serialize");
    assert_eq!(
        field(&schema_value, "cz:semanticType"),
        json!("temperature-unit")
    );
}

#[test]
fn security_and_affordance_builders_set_extension_fields() {
    let security = NoSecurityScheme::builder()
        .extra_field("cz:authProfile", json!("local"))
        .build()
        .expect("security should build");
    let security_value = serde_json::to_value(security).expect("security should serialize");
    assert_eq!(field(&security_value, "cz:authProfile"), json!("local"));

    let action = ActionAffordance::builder()
        .extra_field("cz:computePlacement", json!("edge"))
        .build()
        .expect("action should build");
    let action_value = serde_json::to_value(action).expect("action should serialize");
    assert_eq!(field(&action_value, "cz:computePlacement"), json!("edge"));

    let event = EventAffordance::builder()
        .extra_field("cz:eventStream", json!("alerts"))
        .build()
        .expect("event should build");
    let event_value = serde_json::to_value(event).expect("event should serialize");
    assert_eq!(field(&event_value, "cz:eventStream"), json!("alerts"));
}
