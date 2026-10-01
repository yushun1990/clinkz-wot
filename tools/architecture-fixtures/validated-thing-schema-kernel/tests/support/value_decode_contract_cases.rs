// Identical decision probes against both actual public TD source models.
use serde_json::Value;
use td::{
    data_schema::{DataSchema, ObjectSchema},
    thing::Thing,
    validate::{Validate, ValidationLevel},
};

fn document(fields: &str) -> String {
    format!(
        r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"contract","security":"none","securityDefinitions":{{"none":{{"scheme":"nosec"}}}},{fields}}}"#
    )
}

fn decode(fields: &str) -> Result<Thing, serde_json::Error> {
    serde_json::from_str(&document(fields))
}

fn wrapper(key: &str, payload: &str) -> String {
    format!(
        "{{{}:{}}}",
        serde_json::to_string(key).unwrap(),
        serde_json::to_string(payload).unwrap(),
    )
}

fn schema(thing: &Thing) -> &ObjectSchema {
    match &thing.properties.as_ref().unwrap()["p"]._schema {
        DataSchema::Object(schema) => schema,
        other => panic!("unexpected schema: {other:?}"),
    }
}

#[test]
fn literal_reference_classifies_the_collision_family_without_dispatch() {
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
        "ordinary",
    ] {
        for payload in ["true", "null", "[true,false]", "1e309", "true trailing"] {
            let object = super::literal(&wrapper(key, payload)).unwrap();
            assert_eq!(object[key], Value::String(payload.into()));
        }
    }
    let first = r#"{"$serde_json::private::RawValue":"true","ordinary":true}"#;
    let last = r#"{"ordinary":true,"\u0024serde_json::private::RawValue":"true"}"#;
    assert_eq!(
        super::literal(first).unwrap(),
        super::literal(last).unwrap()
    );
    let source = wrapper("$serde_json::private::RawValue", "[true,false]");
    let mut nested = source.clone();
    for _ in 0..3 {
        let payload = nested;
        nested = wrapper("$serde_json::private::RawValue", &payload);
        assert_eq!(
            super::literal(&nested).unwrap()["$serde_json::private::RawValue"],
            Value::String(payload),
        );
    }
    let payload = format!("[{}]", vec!["true"; 257].join(","));
    let value = super::literal(&wrapper("$serde_json::private::RawValue", &payload)).unwrap();
    assert_eq!(
        value["$serde_json::private::RawValue"],
        Value::String(payload)
    );
    assert!(super::literal("true trailing").is_err());
}

#[test]
fn collision_effects_reach_known_fields_and_are_not_only_opaque_extensions() {
    let title = wrapper("$serde_json::private::RawValue", r#""converted""#);
    let security = wrapper("$serde_json::private::RawValue", r#"["none"]"#);
    let observable = wrapper("$serde_json::private::RawValue", "true");
    let thing = decode(&format!(
        r#""title":{title},"security":{security},"properties":{{"p":{{"forms":[],"observable":{observable}}}}}"#
    )).unwrap();
    assert_eq!(thing._metadata.title.as_deref(), Some("converted"));
    assert_eq!(thing.security, ["none"]);
    assert!(thing.properties.as_ref().unwrap()["p"].observable);
    for source in [&title, &security, &observable] {
        assert!(super::literal(source).unwrap().is_object());
    }
    // The proposed literal rule leaves these as Objects. Existing string,
    // string-list and flexible-bool field policies then reject their kinds.
    // This probe does not pretend to implement that full field decoder.
}

#[test]
fn literal_numeric_collision_uses_existing_basic_as_non_number() {
    let mut thing = decode(r#""properties":{"p":{"forms":[]}}"#).unwrap();
    let property = &mut thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("p")
        .unwrap()
        ._schema;
    let DataSchema::Object(object) = property else {
        panic!("generic schema")
    };
    object._context._extra_fields.insert(
        "minimum".into(),
        super::literal(r#"{"$serde_json::private::RawValue":"2"}"#).unwrap(),
    );
    object
        ._context
        ._extra_fields
        .insert("maximum".into(), Value::from(1));
    object._context._extra_fields.insert(
        "multipleOf".into(),
        super::literal(r#"{"$serde_json::private::RawValue":"0"}"#).unwrap(),
    );
    thing.validate_with_level(ValidationLevel::Basic).unwrap();
    // Compatibility keeps the supplied typed Object too. No modified Basic
    // rule or serializer re-decode is needed to accept this typed value.
    assert!(schema(&thing)._context._extra_fields["minimum"].is_object());
}

#[test]
fn last_decoded_duplicate_key_wins_before_known_field_projection() {
    let thing = decode(
        r#""title":17,"ti\u0074le":"last","properties":{"p":{"forms":17,"forms":[],"type":"number","type":"integer","minimum":"wrong kind","minimum":1,"const":true,"const":null}},"x":{"a":true,"\u0061":false}"#,
    ).unwrap();
    assert_eq!(thing._metadata.title.as_deref(), Some("last"));
    let property = &thing.properties.as_ref().unwrap()["p"];
    assert!(property._interaction.forms.is_empty());
    let DataSchema::Integer(schema) = &property._schema else {
        panic!("last discriminator")
    };
    assert_eq!(schema.minimum, Some(1));
    assert_eq!(schema._context.constant, Some(Value::Null));
    assert_eq!(thing._extra_fields["x"]["a"], Value::Bool(false));
    assert_eq!(
        super::literal(r#"{"a":true,"\u0061":false}"#).unwrap(),
        thing._extra_fields["x"],
    );
}

#[test]
fn null_and_defaults_follow_the_actual_field_policy() {
    let thing = decode(
        r#""title":null,"profile":null,"properties":{"p":{"forms":[{"href":"/p","op":null,"security":null}],"const":null}}"#,
    ).unwrap();
    assert_eq!(thing._metadata.title, None);
    assert_eq!(thing.profile, None);
    assert_eq!(schema(&thing)._context.constant, Some(Value::Null));
    let property = &thing.properties.as_ref().unwrap()["p"];
    assert!(!property.observable);
    assert!(!schema(&thing)._context.read_only);
    let form = &property._interaction.forms[0];
    assert_eq!(form.content_type, "application/json");
    assert_eq!(form.op, None);
    assert_eq!(form.security, None);
    for fields in [
        r#""id":null"#,
        r#""properties":null"#,
        r#""properties":{"p":{"forms":[],"observable":null}}"#,
        r#""properties":{"p":{"forms":[],"unit":null}}"#,
        r#""properties":{"p":{"forms":[{"href":"/p","contentType":null}]}}"#,
    ] {
        assert!(decode(fields).is_err(), "{fields}");
    }
    assert_eq!(
        schema(&decode(r#""properties":{"p":{"forms":[]}}"#).unwrap())
            ._context
            .constant,
        None
    );
}

#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[test]
fn direct_number_token_content_is_public_ap_content_not_raw_spelling() {
    for (token, expected) in [
        ("-0", "0"),
        ("1E0", "1e+0"),
        ("1e309", "1e+309"),
        ("1.00", "1.00"),
    ] {
        let reference = super::literal(token).unwrap();
        let thing = decode(&format!(
            r#""x":{token},"properties":{{"p":{{"forms":[],"const":{token}}}}}"#
        ))
        .unwrap();
        assert_eq!(reference.as_number().unwrap().as_str(), expected);
        assert_eq!(reference, thing._extra_fields["x"]);
        assert_eq!(schema(&thing)._context.constant.as_ref(), Some(&reference));
    }
    // An inserted exponent sign can cross the configured lexical ceiling even
    // when the wire token itself fits. Future construction must guard both.
    assert_eq!("1e0".len(), 3);
    assert_eq!(
        super::literal("1e0")
            .unwrap()
            .as_number()
            .unwrap()
            .as_str()
            .len(),
        4
    );
}
