#![no_std]
// Each graph is checked independently. In negative graphs serde AP alone must
// not enable the TD capability. Positive graphs are ordinary downstream edges.
#[cfg(any(feature = "positive", feature = "negative"))]
pub use td::{
    ValidatedPropertyReadCursor, ValidatedPropertyReadEvent, ValidatedPropertyReadForm,
    ValidatedPropertyReadStep, ValidatedTextSequence, ValidatedThing,
    ValidatedThingAdmissionConfig, ValidatedThingCause, ValidatedThingCursor, ValidatedThingPhase,
    ValidatedThingProgress,
};
#[cfg(feature = "positive")]
pub fn checked(
    limits: &foundation::ResourceLimits,
) -> Result<ValidatedThingAdmissionConfig, td::ValidatedThingConfigError> {
    ValidatedThingAdmissionConfig::try_from_limits(limits)
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::{format, string::ToString};
    use td::{
        data_schema::DataSchema,
        thing::Thing,
        validate::{Validate, ValidateError},
    };

    fn basic(fields: &str, valid: bool) {
        let schema: DataSchema =
            serde_json::from_str(&format!(r#"{{"type":"string",{fields}}}"#)).unwrap();
        let t: Thing = serde_json::from_str(&format!(r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"graph","security":["none"],"securityDefinitions":{{"none":{{"scheme":"nosec"}}}},"schemaDefinitions":{{"s":{{"type":"string",{fields}}}}}}}"#)).unwrap();
        for result in [schema.validate(), t.validate()] {
            assert_eq!(result.is_ok(), valid, "{fields}: {result:?}");
            if !valid {
                assert!(matches!(result, Err(ValidateError::InvalidSchema(_))));
            }
        }
    }
    #[test]
    fn public_basic_number_meaning_in_the_resolved_normal_dependency_graph() {
        let lexical = "1.0000000000000000001";
        let number: serde_json::Number = lexical.parse().unwrap();
        assert_eq!(
            number.to_string() == lexical,
            cfg!(any(
                feature = "ap",
                feature = "capability",
                feature = "sibling"
            ))
        );
        for lower in ["minimum", "exclusiveMinimum"] {
            for upper in ["maximum", "exclusiveMaximum"] {
                basic(
                    &format!("\"{lower}\":9007199254740993,\"{upper}\":9007199254740992"),
                    true,
                );
                basic(&format!("\"{lower}\":2,\"{upper}\":1"), false);
            }
        }
        basic(r#""multipleOf":2.5"#, true);
        basic(r#""multipleOf":1e-4000"#, false);
        basic(
            r#""minimum":"ignored","maximum":null,"multipleOf":true"#,
            true,
        );
        #[cfg(any(feature = "ap", feature = "capability", feature = "sibling"))]
        {
            for field in [
                "minimum",
                "exclusiveMinimum",
                "maximum",
                "exclusiveMaximum",
                "multipleOf",
            ] {
                basic(&format!("\"{field}\":1e309"), false);
            }
            basic(r#""const":1e309,"opaque":{"minimum":1e309}"#, true);
        }
    }
}
