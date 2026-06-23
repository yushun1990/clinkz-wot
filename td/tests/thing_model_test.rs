use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::MetadataHelper,
    data_type::ThingModelVersionInfo,
    form::Form,
    link::Link,
    security_scheme::SecurityScheme,
    thing_model::ThingModel,
    thing_model::ThingModelForm,
    validate::{Validate, ValidateError, ValidationLevel},
};

#[test]
fn thing_model_round_trips_extensions_and_tm_terms() {
    let raw = r##"{
        "@context": [
            "https://www.w3.org/2022/wot/td/v1.1",
            { "tm": "https://www.w3.org/2022/wot/tm#", "cz": "https://clinkz.io/ns/wot#" }
        ],
        "@type": "tm:ThingModel",
        "title": "Lamp Model",
        "version": {
            "model": "1.0.0"
        },
        "description": "Reusable lamp capabilities",
        "links": [
            {
                "href": "https://models.example.com/base-lamp.tm.jsonld",
                "rel": "tm:extends",
                "cz:catalog": "clinkz"
            }
        ],
        "properties": {
            "brightness": {
                "type": "integer",
                "minimum": 0,
                "maximum": 100,
                "forms": [
                    { "href": "properties/brightness", "op": ["readproperty", "writeproperty"] }
                ],
                "cz:storage": "shadow"
            }
        },
        "actions": {
            "toggle": {
                "forms": [
                    { "href": "actions/toggle", "op": "invokeaction" }
                ]
            }
        },
        "tm:optional": ["/actions/toggle"],
        "cz:binding": { "protocol": "zenoh-template" }
    }"##;

    let model: ThingModel = serde_json::from_str(raw).expect("TM should deserialize");
    model.validate().expect("TM should validate");

    let original: serde_json::Value = serde_json::from_str(raw).expect("raw JSON should parse");
    let serialized: serde_json::Value =
        serde_json::to_value(&model).expect("TM should serialize to JSON");

    assert_eq!(original, serialized);
}

#[test]
fn thing_model_builder_sets_version_metadata() {
    let model = ThingModel::builder("Versioned Model")
        .version(ThingModelVersionInfo {
            model: Some("1.2.3".to_string()),
            _extra_fields: Default::default(),
        })
        .build()
        .expect("model should build");

    let value = serde_json::to_value(model).expect("TM should serialize");
    assert_eq!(value["version"]["model"], "1.2.3");
    assert!(value["version"].get("instance").is_none());
}

#[test]
fn thing_model_form_can_omit_href() {
    let form = ThingModelForm {
        content_type: "application/json".to_string(),
        op: Some(vec![clinkz_wot_td::data_type::Operation::InvokeAction]),
        ..Default::default()
    };

    let value = serde_json::to_value(&form).expect("TM form should serialize");
    assert!(value.get("href").is_none());

    let round_tripped: ThingModelForm =
        serde_json::from_value(value).expect("TM form should deserialize");
    assert_eq!(round_tripped, form);
}

#[test]
fn thing_model_builder_creates_valid_model() {
    let brightness = PropertyAffordance::builder(DataSchema::Integer(
        DataSchema::integer().minimum(0).maximum(100).build(),
    ))
    .form(
        Form::builder("properties/brightness")
            .build()
            .expect("form should build"),
    )
    .build()
    .expect("property should build");

    let model = ThingModel::builder("Lamp Model")
        .id("https://models.example.com/lamp")
        .title("Updated Lamp Model")
        .property("brightness", brightness)
        .link(
            Link::builder("https://models.example.com/base-lamp.tm.jsonld")
                .rel("tm:extends")
                .build()
                .expect("link should build"),
        )
        .optional("/properties/brightness")
        .extra_field("cz:modelVersion", serde_json::json!("1.0.0"))
        .build()
        .expect("model should build");

    assert_eq!(model._metadata.title.as_deref(), Some("Updated Lamp Model"));
    assert_eq!(
        model._metadata.tags.as_deref(),
        Some(&["tm:ThingModel".to_string()][..])
    );
    assert!(
        model
            .properties
            .as_ref()
            .unwrap()
            .contains_key("brightness")
    );
}

#[test]
fn basic_validation_rejects_missing_thing_model_type() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Missing Type Model"
    }"#;

    let model: ThingModel = serde_json::from_str(raw).expect("TM shape should deserialize");
    model
        .validate_with_level(ValidationLevel::Minimal)
        .expect("minimal validation should accept serde-valid TM shape");

    let err = model
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject missing tm:ThingModel type");

    assert!(
        matches!(err, ValidateError::MissingRequiredField(field) if field == "@type: tm:ThingModel")
    );
}

#[test]
fn basic_validation_rejects_unknown_tm_optional_pointer() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "@type": "tm:ThingModel",
        "title": "Invalid Optional Model",
        "tm:optional": ["/properties/missing"]
    }"#;

    let model: ThingModel = serde_json::from_str(raw).expect("TM shape should deserialize");
    let err = model
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("unknown tm:optional pointer should fail basic validation");

    assert!(matches!(
        err,
        ValidateError::InvalidReference { context, reference }
            if context == "ThingModel.tm:optional" && reference == "/properties/missing"
    ));
}

#[test]
fn basic_validation_checks_optional_security_references() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "@type": "tm:ThingModel",
        "title": "Security Model",
        "security": "nosec_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        }
    }"#;

    let model: ThingModel = serde_json::from_str(raw).expect("TM shape should deserialize");
    model
        .validate_with_level(ValidationLevel::Basic)
        .expect("known model security reference should validate");

    let scheme = serde_json::from_value::<SecurityScheme>(serde_json::json!({ "scheme": "nosec" }))
        .expect("security scheme should deserialize");
    let built = ThingModel::builder("Security Model")
        .security_name("nosec_sc")
        .security_definition("nosec_sc", scheme)
        .build()
        .expect("model with security should build");

    built.validate().expect("built model should validate");
}

#[test]
fn profile_validation_rejects_unknown_additional_response_schema_reference_in_thing_model() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "@type": "tm:ThingModel",
        "title": "Template With Invalid Response Schema",
        "actions": {
            "reboot": {
                "forms": [
                    {
                        "href": "actions/reboot",
                        "additionalResponses": [
                            {
                                "schema": "problem"
                            }
                        ]
                    }
                ]
            }
        }
    }"#;

    let model: ThingModel = serde_json::from_str(raw).expect("TM shape should deserialize");
    model
        .validate_with_level(ValidationLevel::Basic)
        .expect("basic validation should keep tolerant additional response schema references");

    let err = model
        .validate_with_level(ValidationLevel::Profile)
        .expect_err(
            "profile validation should reject unknown additional response schema references",
        );

    assert!(matches!(
        err,
        ValidateError::InvalidReference { context, reference }
            if context == "actions.reboot.forms[0].additionalResponses[0].schema"
                && reference == "problem"
    ));
}
