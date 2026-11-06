use clinkz_wot_codec_json::validate_json;
use clinkz_wot_core::data_schema::{PreparedSchema as S, SchemaProperty, ValueType};

fn field(name: &str, required: bool, schema: S) -> SchemaProperty {
    SchemaProperty {
        name: name.into(),
        required,
        schema,
    }
}
fn integer(minimum: Option<i64>, maximum: Option<i64>) -> S {
    S::Integer {
        minimum,
        maximum,
        exclusive_minimum: None,
        exclusive_maximum: None,
    }
}
fn schema() -> S {
    S::object(vec![field(
        "settings",
        true,
        S::object(vec![
            field("level", true, integer(Some(1), Some(99))),
            field(
                "label",
                true,
                S::String {
                    min_length: Some(1),
                    max_length: Some(2),
                },
            ),
            field("enabled", false, S::Type(ValueType::Boolean)),
        ])
        .unwrap(),
    )])
    .unwrap()
}

#[test]
fn nested_required_constraints_unicode_and_optional_unknown_members() {
    let s = schema();
    for value in [
        r#"{"settings":{"level":42,"label":"a"}}"#,
        r#"{"settings":{"level":1e1,"label":"\ud83d\ude00","enabled":true},"extra":[1,null]}"#,
        r#"{"settings":{"level":99.00,"label":"e\u0301","other":{}}}"#,
    ] {
        assert!(validate_json(&s, value.as_bytes()).is_ok(), "{value}");
    }
    for value in [
        r#"{}"#,
        r#"{"settings":null}"#,
        r#"{"settings":{}}"#,
        r#"{"settings":{"level":0,"label":"a"}}"#,
        r#"{"settings":{"level":100,"label":"a"}}"#,
        r#"{"settings":{"level":1.00000000000000000001,"label":"a"}}"#,
        r#"{"settings":{"level":10,"label":""}}"#,
        r#"{"settings":{"level":10,"label":"abc"}}"#,
        r#"{"settings":{"level":10,"label":"a","enabled":0}}"#,
        r#"{"settings":{"level":10,"le\u0076el":11,"label":"a"}}"#,
        r#"{"settings":{"level":10,"label":"\ud800"}}"#,
    ] {
        assert!(validate_json(&s, value.as_bytes()).is_err(), "{value}");
    }
}

#[test]
fn exact_integer_boundaries_do_not_round_through_binary_float_or_i64() {
    let exact = integer(Some(9_007_199_254_740_993), Some(9_007_199_254_740_993));
    for valid in [
        "9007199254740993",
        "9007199254740993.0",
        "900719925474099300e-2",
    ] {
        assert!(validate_json(&exact, valid.as_bytes()).is_ok(), "{valid}");
    }
    for bad in [
        "9007199254740992",
        "9007199254740994",
        "9007199254740993.000000001",
    ] {
        assert!(validate_json(&exact, bad.as_bytes()).is_err(), "{bad}");
    }
    let all = integer(Some(i64::MIN), Some(i64::MAX));
    for valid in ["-9223372036854775808", "9223372036854775807.0", "-0e-99999"] {
        assert!(validate_json(&all, valid.as_bytes()).is_ok(), "{valid}");
    }
    for bad in [
        "-9223372036854775809",
        "9223372036854775808",
        "1e99999",
        "-1e99999",
        "1e-99999",
    ] {
        assert!(validate_json(&all, bad.as_bytes()).is_err(), "{bad}");
    }
    assert!(validate_json(&integer(Some(0), None), b"1e99999999999999999999").is_ok());
    assert!(validate_json(&integer(None, Some(0)), b"-1e99999999999999999999").is_ok());
    let exclusive = S::Integer {
        minimum: None,
        maximum: None,
        exclusive_minimum: Some(-1),
        exclusive_maximum: Some(1),
    };
    assert!(validate_json(&exclusive, b"-0.0").is_ok());
    assert!(validate_json(&exclusive, b"1").is_err());
    assert!(validate_json(&exclusive, b"-1").is_err());
}

#[test]
fn ignored_subtrees_and_schema_construction_have_explicit_depth_bounds() {
    let deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
    assert!(validate_json(&S::Any, deep.as_bytes()).is_err());
    let mut s = S::Any;
    for _ in 1..S::MAX_DEPTH {
        s = S::object(vec![field("x", true, s)]).unwrap();
    }
    assert!(S::object(vec![field("x", true, s)]).is_err());
    assert!(
        S::object(vec![
            field("same", true, S::Any),
            field("same", false, S::Any)
        ])
        .is_err()
    );
}
