//! Production Basic semantics. Resource/work admission is a separate boundary.
use clinkz_wot_td as td_crate;
use td_crate::{
    data_schema::{DataSchema, IntegerSchema, NullSchema, NumberSchema},
    thing::Thing,
    validate::{Validate, ValidateError, ValidationLevel},
};

#[path = "support/basic_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;
#[path = "support/typed_corpus_shared.rs"]
#[allow(dead_code)]
mod typed;

#[test]
fn production_basic_covers_the_complete_fixed_boundary_corpus() {
    for case in corpus::cases() {
        assert_eq!(case.thing.validate().is_ok(), case.valid, "{}", case.label);
        assert!(
            case.thing
                .validate_with_level(ValidationLevel::Minimal)
                .is_ok()
        );
    }
}

#[test]
fn basic_accepts_absent_id_and_valid_input_that_cannot_serialize() {
    let mut thing = typed::serializer_failure_thing();
    thing.id = None;
    thing.validate().unwrap();
    assert!(serde_json::to_string(&thing).is_err());
}

fn schema_error(input: &str) -> String {
    let schema: DataSchema = serde_json::from_str(input).unwrap();
    let Err(ValidateError::InvalidSchema(message)) = schema.validate() else {
        panic!("expected InvalidSchema for {input}");
    };
    message
}

#[test]
fn schema_first_error_preserves_one_of_flags_bounds_and_native_child_order() {
    assert_eq!(
        schema_error(
            r#"{"type":"string","readOnly":true,"writeOnly":true,"oneOf":[{"type":"integer","multipleOf":0}]}"#
        ),
        "[0]: multipleOf must be greater than 0"
    );
    assert_eq!(
        schema_error(r#"{"type":"null","readOnly":true,"writeOnly":true,"minimum":1e309}"#),
        "readOnly and writeOnly must not both be true"
    );
    assert_eq!(
        schema_error(
            r#"{"type":"array","minItems":5,"maxItems":1,"items":[{"type":"integer","multipleOf":0}]}"#
        ),
        "minItems must be less than or equal to maxItems"
    );
    assert_eq!(
        schema_error(
            r#"{"type":"object","properties":{"z":{"type":"integer","multipleOf":0},"a":{"type":"string","minLength":2,"maxLength":1}}}"#
        ),
        "properties.a: minLength must be less than or equal to maxLength"
    );
    assert_eq!(
        schema_error(r#"{"type":"string","minimum":2,"maximum":1,"multipleOf":1e309}"#),
        "minimum must be less than or equal to maximum"
    );
    let mut schema = NullSchema::default();
    schema._context.data_type = Some("string".into());
    schema._context.read_only = true;
    schema._context.write_only = true;
    assert_eq!(
        DataSchema::Null(schema).validate(),
        Err(ValidateError::InvalidSchema(
            "type 'string' does not match null schema".into()
        ))
    );
}

#[test]
fn every_extension_predicate_rejects_failed_projection_at_all_schema_locations() {
    for field in [
        "minimum",
        "exclusiveMinimum",
        "maximum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        let schema = format!(r#"{{"type":"string","{field}":1e309}}"#);
        let direct: DataSchema = serde_json::from_str(&schema).unwrap();
        for level in [
            ValidationLevel::Basic,
            ValidationLevel::Profile,
            ValidationLevel::Full,
        ] {
            assert_eq!(
                direct.validate_with_level(level),
                Err(ValidateError::InvalidSchema(format!(
                    "{field} Number has no finite binary64 projection"
                )))
            );
        }
        for placement in [
            format!(r#""schemaDefinitions":{{"p":{schema}}}"#),
            format!(r#""uriVariables":{{"p":{schema}}}"#),
            format!(r#""properties":{{"p":{{"type":"string","{field}":1e309,"forms":[]}}}}"#),
            format!(
                r#""properties":{{"p":{{"type":"object","properties":{{"x":{schema}}},"forms":[]}}}}"#
            ),
            format!(r#""properties":{{"p":{{"type":"array","items":[{schema}],"forms":[]}}}}"#),
            format!(r#""properties":{{"p":{{"type":"null","oneOf":[{schema}],"forms":[]}}}}"#),
            format!(
                r#""properties":{{"p":{{"type":"null","uriVariables":{{"x":{schema}}},"forms":[]}}}}"#
            ),
            format!(r#""actions":{{"p":{{"input":{schema},"forms":[]}}}}"#),
            format!(r#""actions":{{"p":{{"output":{schema},"forms":[]}}}}"#),
            format!(r#""actions":{{"p":{{"uriVariables":{{"x":{schema}}},"forms":[]}}}}"#),
            format!(r#""events":{{"p":{{"subscription":{schema},"forms":[]}}}}"#),
            format!(r#""events":{{"p":{{"data":{schema},"forms":[]}}}}"#),
            format!(r#""events":{{"p":{{"dataResponse":{schema},"forms":[]}}}}"#),
            format!(r#""events":{{"p":{{"cancellation":{schema},"forms":[]}}}}"#),
            format!(r#""events":{{"p":{{"uriVariables":{{"x":{schema}}},"forms":[]}}}}"#),
        ] {
            let input = format!(
                r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Basic program","security":["none"],"securityDefinitions":{{"none":{{"scheme":"nosec"}}}},{placement}}}"#
            );
            let thing: Thing = serde_json::from_str(&input).unwrap();
            assert!(
                matches!(thing.validate(), Err(ValidateError::InvalidSchema(_))),
                "{input}"
            );
            assert!(thing.validate_with_level(ValidationLevel::Minimal).is_ok());
        }
    }
}

#[test]
fn opaque_numbers_non_numbers_binary64_rounding_and_typed_fields_keep_their_meaning() {
    for input in [
        r#"{"type":"string","const":1e309,"default":1e309,"enum":[1e309],"opaque":{"minimum":1e309}}"#,
        r#"{"type":"string","minimum":"ignored","maximum":null,"multipleOf":true}"#,
        r#"{"type":"string","minimum":9007199254740993,"maximum":9007199254740992}"#,
        r#"{"type":"number","multipleOf":2.5}"#,
    ] {
        let schema: DataSchema = serde_json::from_str(input).unwrap();
        schema.validate().unwrap();
    }
    assert_eq!(
        schema_error(r#"{"type":"string","multipleOf":1e-4000}"#),
        "multipleOf must be greater than 0"
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        DataSchema::Number(NumberSchema {
            minimum: Some(value),
            maximum: Some(f64::INFINITY),
            ..Default::default()
        })
        .validate()
        .unwrap();
    }
    assert!(
        DataSchema::Integer(IntegerSchema {
            minimum: Some(9_007_199_254_740_993),
            maximum: Some(9_007_199_254_740_992),
            ..Default::default()
        })
        .validate()
        .is_err()
    );
}
