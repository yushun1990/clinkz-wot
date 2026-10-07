//! The real public Thing/component call graph in the generated candidate,
//! compared with current production source under the same resolved graph.
use clinkz_wot_td as original;
#[cfg(any(feature = "ap", feature = "validated-thing"))]
use td_crate::thing::Thing;
use td_crate::{
    data_schema::DataSchema,
    validate::{Validate, ValidationLevel},
};
use validated_thing_schema_kernel_probe as td_crate;

#[path = "../../../../td/tests/support/typed_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;

fn schema_parity(input: &str) {
    let before: original::data_schema::DataSchema = serde_json::from_str(input).unwrap();
    let after: DataSchema = serde_json::from_str(input).unwrap();
    for level in [
        ValidationLevel::Minimal,
        ValidationLevel::Basic,
        ValidationLevel::Profile,
        ValidationLevel::Full,
    ] {
        let old_level = match level {
            ValidationLevel::Minimal => original::validate::ValidationLevel::Minimal,
            ValidationLevel::Basic => original::validate::ValidationLevel::Basic,
            ValidationLevel::Profile => original::validate::ValidationLevel::Profile,
            ValidationLevel::Full => original::validate::ValidationLevel::Full,
        };
        use original::validate::Validate as _;
        assert_eq!(
            before
                .validate_with_level(old_level)
                .map_err(|err| err.to_string()),
            after
                .validate_with_level(level)
                .map_err(|err| err.to_string()),
            "{input}"
        );
    }
}

#[test]
fn every_schema_variant_and_unaffected_first_error_match_real_public_source() {
    for input in [
        r#"{"type":"array","items":[{"type":"string","minLength":2,"maxLength":1}],"minItems":5,"maxItems":1}"#,
        r#"{"type":"array","items":[{"type":"string","minLength":2,"maxLength":1}]}"#,
        r#"{"type":"object","properties":{"z":{"type":"integer","multipleOf":0},"a":{"type":"boolean","readOnly":true,"writeOnly":true}}}"#,
        r#"{"type":"number","minimum":2,"maximum":1,"multipleOf":0}"#,
        r#"{"type":"integer","exclusiveMinimum":5,"exclusiveMaximum":1}"#,
        r#"{"type":"null","minLength":2,"maxLength":1}"#,
        r#"{"type":"boolean","minItems":2,"maxItems":1}"#,
        r#"{"type":"string","readOnly":true,"writeOnly":true,"oneOf":[{"type":"integer","multipleOf":0}]}"#,
        r#"{"type":"string","minimum":"ignore","maximum":null,"multipleOf":true}"#,
        r#"{"type":"string","minimum":9007199254740993,"maximum":9007199254740992}"#,
        r#"{"type":"string","multipleOf":1e-4000}"#,
        r#"{"type":"number","multipleOf":2.5}"#,
        r#"{"type":"object","required":["undefined"],"properties":{}}"#,
        r#"{"type":"array","items":[]}"#,
    ] {
        schema_parity(input);
    }
    let thing = corpus::typed_corpus();
    thing.validate_with_level(ValidationLevel::Basic).unwrap();
    corpus::assert_complete_security_corpus(&thing);
    // That existing corpus contains opaque 1e309, constructible only in AP.
    // Capability-off base/order keep their real resolved parsing behavior.
    #[cfg(any(feature = "ap", feature = "validated-thing"))]
    corpus::nested_schema_corpus()
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    let serializer_failure = corpus::serializer_failure_thing();
    serializer_failure
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    assert!(serde_json::to_string(&serializer_failure).is_err());
}

#[test]
fn typed_nonfinite_and_integer_fields_keep_existing_comparisons() {
    use original::validate::Validate as _;
    for (minimum, maximum, multiple_of) in [
        (f64::NAN, 1.0, f64::NAN),
        (f64::INFINITY, f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::INFINITY, 2.5),
        (2.0, 1.0, 0.0),
    ] {
        let before = original::data_schema::NumberSchema {
            minimum: Some(minimum),
            maximum: Some(maximum),
            multiple_of: Some(multiple_of),
            ..Default::default()
        };
        let after = td_crate::data_schema::NumberSchema {
            minimum: Some(minimum),
            maximum: Some(maximum),
            multiple_of: Some(multiple_of),
            ..Default::default()
        };
        assert_eq!(
            original::data_schema::DataSchema::Number(before)
                .validate()
                .map_err(|err| err.to_string()),
            DataSchema::Number(after)
                .validate()
                .map_err(|err| err.to_string())
        );
    }
    schema_parity(r#"{"type":"integer","minimum":9007199254740993,"maximum":9007199254740992}"#);
}

#[cfg(any(feature = "ap", feature = "validated-thing"))]
#[test]
fn five_predicate_rule_reaches_production_and_candidate_at_every_schema_location() {
    use original::validate::Validate as _;
    for field in [
        "minimum",
        "exclusiveMinimum",
        "maximum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        let schema = format!(r#"{{"type":"string","{field}":1e309}}"#);
        for placement in [
            format!(r#""schemaDefinitions":{{"probe":{schema}}}"#),
            format!(r#""uriVariables":{{"probe":{schema}}}"#),
            format!(r#""properties":{{"probe":{schema}}}"#),
            format!(
                r#""properties":{{"probe":{{"type":"object","properties":{{"nested":{schema}}}}}}}"#
            ),
            format!(r#""properties":{{"probe":{{"type":"array","items":[{schema}]}}}}"#),
            format!(r#""properties":{{"probe":{{"type":"null","oneOf":[{schema}]}}}}"#),
            format!(
                r#""properties":{{"probe":{{"type":"null","uriVariables":{{"var":{schema}}}}}}}"#
            ),
            format!(r#""actions":{{"probe":{{"input":{schema}}}}}"#),
            format!(r#""actions":{{"probe":{{"output":{schema}}}}}"#),
            format!(r#""actions":{{"probe":{{"uriVariables":{{"var":{schema}}}}}}}"#),
            format!(r#""events":{{"probe":{{"subscription":{schema}}}}}"#),
            format!(r#""events":{{"probe":{{"data":{schema}}}}}"#),
            format!(r#""events":{{"probe":{{"dataResponse":{schema}}}}}"#),
            format!(r#""events":{{"probe":{{"cancellation":{schema}}}}}"#),
            format!(r#""events":{{"probe":{{"uriVariables":{{"var":{schema}}}}}}}"#),
        ] {
            let input = format!(
                r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"public delta","security":["none"],"securityDefinitions":{{"none":{{"scheme":"nosec"}}}},{placement}}}"#
            );
            // Ordinary JSON corpus scaffolding, not Thing normalization. Both
            // public source graphs decode exactly this same input.
            let mut document: serde_json::Value = serde_json::from_str(&input).unwrap();
            for collection in ["properties", "actions", "events"] {
                if let Some(affordances) = document.get_mut(collection) {
                    for affordance in affordances.as_object_mut().unwrap().values_mut() {
                        affordance
                            .as_object_mut()
                            .unwrap()
                            .insert("forms".into(), serde_json::json!([]));
                    }
                }
            }
            let input = serde_json::to_string(&document).unwrap();
            let before: original::thing::Thing = serde_json::from_str(&input).unwrap();
            let after: Thing = serde_json::from_str(&input).unwrap();
            assert!(
                matches!(
                    before.validate(),
                    Err(original::validate::ValidateError::InvalidSchema(_))
                ),
                "{input}"
            );
            assert!(
                matches!(
                    after.validate(),
                    Err(td_crate::validate::ValidateError::InvalidSchema(_))
                ),
                "{input}"
            );
            assert_eq!(
                before.validate().map_err(|e| e.to_string()),
                after.validate().map_err(|e| e.to_string()),
                "{input}"
            );
            assert!(after.validate_with_level(ValidationLevel::Minimal).is_ok());
        }
    }
    schema_parity(
        r#"{"type":"string","const":1e309,"default":1e309,"enum":[1e309],"opaque":{"minimum":1e309}}"#,
    );
}
