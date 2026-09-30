// Compile this identical corpus against unchanged production and the existing
// public-source Basic candidate. All decode observations use public TD/serde
// calls. These adversarial keys are input data, never parser dispatch rules.
use serde_json::{Map, Value};
use td::{
    data_schema::{DataSchema, ObjectSchema},
    thing::Thing,
    validate::{Validate, ValidateError, ValidationLevel},
};

const NUMBER_KEY: &str = "$serde_json::private::Number";
const RAW_KEY: &str = "$serde_json::private::RawValue";

fn document(fields: &str) -> String {
    format!(
        r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"decode boundary","security":"none","securityDefinitions":{{"none":{{"scheme":"nosec"}}}},{fields}}}"#
    )
}

fn wrapper(key: &str, payload: &str) -> String {
    // Only JSON corpus scaffolding; never serialize a Thing or normalize it.
    format!(
        "{{{}:{}}}",
        serde_json::to_string(key).unwrap(),
        serde_json::to_string(payload).unwrap()
    )
}

fn decode(fields: &str) -> Result<Thing, serde_json::Error> {
    serde_json::from_str(&document(fields))
}

fn opaque(body: &str) -> Value {
    let mut thing = decode(&format!(r#""x":{body}"#)).unwrap();
    thing.validate_with_level(ValidationLevel::Basic).unwrap();
    thing._extra_fields.remove("x").unwrap()
}

fn object(key: &str, value: Value) -> Value {
    let mut map = Map::new();
    map.insert(key.into(), value);
    Value::Object(map)
}

fn property_schema(thing: &Thing) -> &ObjectSchema {
    match &thing.properties.as_ref().unwrap()["p"]._schema {
        DataSchema::Object(schema) => schema,
        other => panic!("expected the unchanged generic Object dispatch: {other:?}"),
    }
}

#[test]
fn ordinary_extension_objects_are_the_control_for_member_order() {
    let left = opaque(r#"{"left":"true","right":[true,false]}"#);
    let right = opaque(r#"{"right":[true,false],"left":"true"}"#);
    assert_eq!(left, right);
    assert_eq!(left["left"], Value::String("true".into()));
    assert_eq!(
        left["right"],
        Value::Array(vec![Value::Bool(true), Value::Bool(false)])
    );
    assert_eq!(
        opaque(r#""[true,false]""#),
        Value::String("[true,false]".into())
    );
}

#[test]
fn raw_key_string_is_reinterpreted_by_the_actual_thing_decoder() {
    for (payload, expected) in [
        ("null", Value::Null),
        ("true", Value::Bool(true)),
        (
            "[true,false]",
            Value::Array(vec![Value::Bool(true), Value::Bool(false)]),
        ),
        (r#"{"inside":true}"#, object("inside", Value::Bool(true))),
        ("2", Value::from(2)),
    ] {
        let source = wrapper(RAW_KEY, payload);
        // The directly constructed typed object retains its key and string.
        // The public wire decoder instead returns the embedded JSON value.
        assert_ne!(
            opaque(&source),
            object(RAW_KEY, Value::String(payload.into()))
        );
        assert_eq!(opaque(&source), expected, "{source}");
    }
    // Escaping the input member name does not avoid the behavior. No raw-byte
    // blacklist would establish a decoded-key policy.
    assert_eq!(
        opaque(r#"{"\u0024serde_json::private::RawValue":"true"}"#),
        Value::Bool(true)
    );
}

#[test]
fn number_key_behavior_depends_on_the_resolved_ap_graph() {
    let source = wrapper(NUMBER_KEY, "2");
    let value = opaque(&source);
    #[cfg(any(feature = "ap", feature = "validated-thing"))]
    assert_eq!(value, Value::from(2));
    #[cfg(not(any(feature = "ap", feature = "validated-thing")))]
    assert_eq!(value, object(NUMBER_KEY, Value::String("2".into())));
}

#[test]
fn raw_key_member_order_changes_wire_acceptance_in_all_graphs() {
    let marker_first = r#"{"$serde_json::private::RawValue":"true","ordinary":true}"#;
    let marker_last = r#"{"ordinary":true,"$serde_json::private::RawValue":"true"}"#;
    // Both texts are valid JSON according to the dependency's public syntax
    // validator; their ordinary object key/value associations are identical.
    for source in [marker_first, marker_last] {
        serde_json::value::RawValue::from_string(source.into()).unwrap();
    }
    assert!(decode(&format!(r#""x":{marker_first}"#)).is_err());
    let kept = opaque(marker_last);
    assert_eq!(kept[RAW_KEY], Value::String("true".into()));
    assert_eq!(kept["ordinary"], Value::Bool(true));
}

#[test]
fn number_key_member_order_changes_wire_acceptance_only_with_ap() {
    let first = r#"{"$serde_json::private::Number":"2","ordinary":true}"#;
    let last = r#"{"ordinary":true,"$serde_json::private::Number":"2"}"#;
    #[cfg(any(feature = "ap", feature = "validated-thing"))]
    {
        assert!(decode(&format!(r#""x":{first}"#)).is_err());
        assert_eq!(opaque(last)[NUMBER_KEY], Value::String("2".into()));
    }
    #[cfg(not(any(feature = "ap", feature = "validated-thing")))]
    assert_eq!(opaque(first), opaque(last));
}

#[test]
fn repeated_td_field_conversion_makes_map_backend_relevant() {
    let source = r#"{"ordinary":true,"$serde_json::private::RawValue":"true"}"#;
    let root = opaque(source);
    let property = decode(&format!(
        r#""properties":{{"p":{{"forms":[],"const":{source}}}}}"#
    ));
    // Thing -> flat::take/from_value -> DataSchema/RawValue -> context ->
    // flat::take::<Value> crosses Value decoding again. Sorted and insertion-
    // order Maps put different keys first on that later pass.
    #[cfg(feature = "order")]
    {
        let property = property.unwrap();
        property
            .validate_with_level(ValidationLevel::Basic)
            .unwrap();
        assert_eq!(
            property_schema(&property)._context.constant.as_ref(),
            Some(&root)
        );
    }
    #[cfg(not(feature = "order"))]
    {
        assert!(property.is_err());
        assert_eq!(root[RAW_KEY], Value::String("true".into()));
    }
}

#[test]
fn embedded_json_is_recursively_interpreted_and_can_introduce_structure() {
    let mut source = "true".to_owned();
    for _ in 0..3 {
        source = wrapper(RAW_KEY, &source);
        assert_eq!(opaque(&source), Value::Bool(true));
    }
    let payload = format!("[{}]", vec!["true"; 257].join(","));
    let value = opaque(&wrapper(RAW_KEY, &payload));
    let array = value.as_array().unwrap();
    assert_eq!(array.len(), 257);
    assert!(array.iter().all(|value| value == &Value::Bool(true)));
    // The source x is an object with one string value, whereas decoded x has
    // 258 semantic nodes. This is no work/resource or bounded decoder proof.
}

#[test]
fn valid_outer_json_can_fail_only_when_the_string_is_reparsed() {
    let source = wrapper(RAW_KEY, "true trailing");
    serde_json::value::RawValue::from_string(source.clone()).unwrap();
    assert!(decode(&format!(r#""x":{source}"#)).is_err());
    assert_eq!(
        opaque(r#""true trailing""#),
        Value::String("true trailing".into())
    );
}

#[test]
fn decoded_numbers_change_unaffected_basic_predicates() {
    let hidden = wrapper(RAW_KEY, "2");
    let thing = decode(&format!(
        r#""properties":{{"p":{{"forms":[],"minimum":{hidden},"maximum":1}}}}"#
    ))
    .unwrap();
    assert!(property_schema(&thing)._context._extra_fields["minimum"].is_number());
    assert!(matches!(
        thing.validate_with_level(ValidationLevel::Basic),
        Err(ValidateError::InvalidSchema(_))
    ));

    let mut typed = ObjectSchema::default();
    typed
        ._context
        ._extra_fields
        .insert("minimum".into(), object(RAW_KEY, Value::String("2".into())));
    typed
        ._context
        ._extra_fields
        .insert("maximum".into(), Value::from(1));
    // A literal object retained by a grammar-only direct decoder would be a
    // non-Number and would bypass the same shared bound predicate.
    DataSchema::Object(typed)
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();

    let hidden = wrapper(RAW_KEY, "0");
    let thing = decode(&format!(
        r#""properties":{{"p":{{"forms":[],"multipleOf":{hidden}}}}}"#
    ))
    .unwrap();
    assert!(matches!(
        thing.validate_with_level(ValidationLevel::Basic),
        Err(ValidateError::InvalidSchema(_))
    ));
}

#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[test]
fn decoded_overflow_reaches_all_five_amended_predicates() {
    for key in [NUMBER_KEY, RAW_KEY] {
        for field in [
            "minimum",
            "exclusiveMinimum",
            "maximum",
            "exclusiveMaximum",
            "multipleOf",
        ] {
            let hidden = wrapper(key, "1e309");
            let thing = decode(&format!(
                r#""properties":{{"p":{{"forms":[],"{field}":{hidden}}}}}"#
            ))
            .unwrap();
            assert!(property_schema(&thing)._context._extra_fields[field].is_number());
            let result = thing.validate_with_level(ValidationLevel::Basic);
            if AMENDED_BASIC {
                assert!(
                    matches!(result, Err(ValidateError::InvalidSchema(_))),
                    "{field}: {key}"
                );
            } else {
                assert!(
                    result.is_ok(),
                    "unchanged failed-projection-as-absent oracle"
                );
            }
        }
    }
}

#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[test]
fn string_origin_numbers_require_the_existing_post_decode_lexical_guard() {
    use super::atomic::{ProjectionProgress, project};
    use clinkz_wot_foundation::{WorkBudget, WorkClass};

    for ceiling in [0, 64, 256] {
        let lengths = if ceiling == 0 {
            vec![1]
        } else {
            vec![ceiling - 1, ceiling, ceiling + 1, 4096]
        };
        for length in lengths {
            let lexeme = if length == 1 {
                "0".into()
            } else {
                format!("1e+{}1", "0".repeat(length - 4))
            };
            for key in [NUMBER_KEY, RAW_KEY] {
                let source = wrapper(key, &lexeme);
                // This fixed source contains no Number token: all digits are
                // in the member's JSON string. Ordinary decode owns its graph
                // before this interval; no pre-copy/strict guarantee is claimed.
                let value = opaque(&source);
                let number = value.as_number().unwrap();
                assert_eq!(number.as_str(), lexeme);
                let mut budget =
                    WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 10_000);
                let mut lifetime = 10_000;
                let mut calls = 0;
                let result = project(
                    number.as_str(),
                    ceiling,
                    &mut budget,
                    &mut lifetime,
                    || false,
                    || {
                        calls += 1;
                        number.as_f64()
                    },
                );
                if length > ceiling {
                    assert_eq!(result, ProjectionProgress::Limit);
                    assert_eq!(calls, 0);
                    assert_eq!(lifetime, 10_000);
                    assert_eq!(budget.remaining(WorkClass::CodecInputBytes), 10_000);
                } else {
                    assert!(matches!(result, ProjectionProgress::Complete(Some(_))));
                    assert_eq!(calls, 1);
                    assert_eq!(lifetime, 10_000 - length as u64);
                }
            }
        }
    }
}
