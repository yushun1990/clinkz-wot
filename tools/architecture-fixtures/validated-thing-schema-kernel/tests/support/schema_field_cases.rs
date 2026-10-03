use super::{Shape, arena, construction, literal};
use td_crate::data_schema::DataSchema;
use td_crate::validate::{Validate, ValidationLevel};
use validated_thing_value_construction_probe::View;

fn text_list(actual: Option<arena::List<'_>>, expected: Option<&[String]>) {
    assert_eq!(actual.is_some(), expected.is_some());
    if let (Some(actual), Some(expected)) = (actual, expected) {
        assert_eq!(actual.len(), expected.len());
        for (i, value) in expected.iter().enumerate() {
            assert_eq!(actual.get(i).unwrap().text(), Some(value.as_str()));
        }
    }
}
fn languages(actual: Option<View<'_>>, expected: Option<&td_crate::data_type::MultiLanguage>) {
    assert_eq!(actual.is_some(), expected.is_some());
    if let (Some(actual), Some(expected)) = (actual, expected) {
        assert_eq!(actual.len(), expected.len());
        for (i, (key, value)) in expected.as_map().iter().enumerate() {
            let (name, child) = actual.member(i).unwrap();
            assert_eq!(name, key);
            assert_eq!(child.text(), Some(value.as_str()));
        }
    }
}
pub(super) fn fields(view: View<'_>, expected: &DataSchema) {
    let actual = arena::decode(view).unwrap();
    let context = match expected {
        DataSchema::Array(v) => &v._context,
        DataSchema::Boolean(v) => &v._context,
        DataSchema::Number(v) => &v._context,
        DataSchema::Integer(v) => &v._context,
        DataSchema::Object(v) => &v._context,
        DataSchema::String(v) => &v._context,
        DataSchema::Null(v) => &v._context,
    };
    text_list(
        actual.context.metadata.tags,
        context._metadata.tags.as_deref(),
    );
    assert_eq!(
        actual.context.metadata.title,
        context._metadata.title.as_deref()
    );
    assert_eq!(
        actual.context.metadata.description,
        context._metadata.description.as_deref()
    );
    languages(
        actual.context.metadata.titles,
        context._metadata.titles.as_ref(),
    );
    languages(
        actual.context.metadata.descriptions,
        context._metadata.descriptions.as_ref(),
    );
    assert_eq!(
        actual.context.constant.is_some(),
        context.constant.is_some()
    );
    if let (Some(a), Some(b)) = (actual.context.constant, &context.constant) {
        construction::equivalent(a, b);
    }
    assert_eq!(actual.context.default.is_some(), context.default.is_some());
    if let (Some(a), Some(b)) = (actual.context.default, &context.default) {
        construction::equivalent(a, b);
    }
    assert_eq!(
        actual.context.enumerate.is_some(),
        context.enumerate.is_some()
    );
    if let (Some(a), Some(b)) = (actual.context.enumerate, &context.enumerate) {
        assert_eq!(a.len(), b.len());
        for (i, value) in b.iter().enumerate() {
            construction::equivalent(a.child(i).unwrap(), value);
        }
    }
    assert_eq!(actual.context.unit, context.unit.as_deref());
    assert_eq!(actual.context.format, context.format.as_deref());
    assert_eq!(actual.context.data_type, context.data_type.as_deref());
    assert_eq!(actual.context.read_only, context.read_only);
    assert_eq!(actual.context.write_only, context.write_only);
    assert_eq!(actual.context.one_of.is_some(), context.one_of.is_some());
    if let (Some(a), Some(b)) = (actual.context.one_of, &context.one_of) {
        assert_eq!(a.len(), b.len());
        for (i, child) in b.iter().enumerate() {
            fields(a.get(i).unwrap(), child);
        }
    }
    assert_eq!(
        actual.context.extras.members().count(),
        context._extra_fields.len()
    );
    for (key, value) in &context._extra_fields {
        construction::equivalent(actual.context.extras.get(key).unwrap(), value);
    }
    match (actual.shape, expected) {
        (Shape::Array { items, min, max }, DataSchema::Array(schema)) => {
            assert_eq!((min, max), (schema.min_items, schema.max_items));
            assert_eq!(items.is_some(), schema.items.is_some());
            if let (Some(a), Some(b)) = (items, &schema.items) {
                assert_eq!(a.len(), b.len());
                for (i, child) in b.iter().enumerate() {
                    fields(a.get(i).unwrap(), child);
                }
            }
        }
        (Shape::Boolean, DataSchema::Boolean(_)) | (Shape::Null, DataSchema::Null(_)) => {}
        (Shape::Number(values), DataSchema::Number(schema)) => {
            for (a, b) in values.into_iter().zip([
                schema.minimum,
                schema.exclusive_minimum,
                schema.maximum,
                schema.exclusive_maximum,
                schema.multiple_of,
            ]) {
                assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
            }
        }
        (Shape::Integer(values), DataSchema::Integer(schema)) => assert_eq!(
            values,
            [
                schema.minimum,
                schema.exclusive_minimum,
                schema.maximum,
                schema.exclusive_maximum,
                schema.multiple_of
            ]
        ),
        (
            Shape::Object {
                properties,
                required,
            },
            DataSchema::Object(schema),
        ) => {
            text_list(required, schema.required.as_deref());
            assert_eq!(properties.is_some(), schema.properties.is_some());
            if let (Some(a), Some(b)) = (properties, &schema.properties) {
                assert_eq!(a.len(), b.len());
                for (i, (key, child)) in b.iter().enumerate() {
                    let (name, value) = a.member(i).unwrap();
                    assert_eq!(name, key);
                    fields(value, child);
                }
            }
        }
        (
            Shape::String {
                min,
                max,
                pattern,
                encoding,
                media_type,
            },
            DataSchema::String(schema),
        ) => {
            assert_eq!((min, max), (schema.min_length, schema.max_length));
            assert_eq!(pattern, schema.pattern.as_deref());
            assert_eq!(encoding, schema.content_encoding.as_deref());
            assert_eq!(media_type, schema.content_media_type.as_deref());
        }
        _ => panic!("variant mismatch"),
    }
}

// Additional extraction regression, outside the measured engine interval. The
// derived Debug includes all owning typed fields; this is not serialization or
// a normalization/equivalence implementation. Literal parity uses fields above.
pub(super) fn ordinary_observation(input: &str) -> Result<String, String> {
    let document = format!(
        r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"probe","security":"none","securityDefinitions":{{"none":{{"scheme":"nosec"}}}},"properties":{{"p":{{"forms":[],{input}}}}}}}"#
    );
    serde_json::from_str::<td_crate::thing::Thing>(&document)
        .map(|thing| format!("{:?}", thing.properties.unwrap()["p"]._schema))
        .map_err(|error| error.to_string())
}

#[test]
fn all_variant_and_context_fields_are_equal_through_real_arena_construction() {
    let common = r#""@type":["z","a"],"title":"title","titles":{"zh":"标题","en":"title"},"description":"description","descriptions":{"en":"description"},"const":null,"default":{"nested":[1e309,"[true,false]",false]},"enum":[null,1.00,{"opaque":true}],"unit":"unit","oneOf":[{"type":"null","const":null},{"type":"boolean","readOnly":"true"}],"readOnly":"1","writeOnly":0,"format":"format","opaque":{"n":1E0},"minItems":1,"maxItems":2,"minLength":1,"maxLength":2"#;
    for specific in [
        r#""type":"array","items":{"type":"object","required":["z","a"],"properties":{"z":{"type":"string","pattern":"p","contentEncoding":"base64","contentMediaType":"text/plain"},"a":{"type":"integer","minimum":-9223372036854775808,"maximum":9223372036854775807}}}"#,
        r#""type":"array","items":[]"#,
        r#""type":"array","items":null"#,
        r#""type":"boolean""#,
        r#""type":"number","minimum":-1.25,"exclusiveMinimum":-2,"maximum":3.5,"exclusiveMaximum":4,"multipleOf":0.5"#,
        r#""type":"integer","minimum":-3,"exclusiveMinimum":-2,"maximum":9007199254740993,"exclusiveMaximum":9007199254740994,"multipleOf":1"#,
        r#""type":"object","properties":{},"required":[]"#,
        r#""type":"object","properties":{"z":{"type":"array","items":[{"type":"string"},{"type":"null"}]},"a":{"type":"number","minimum":1e309}},"required":["z","a"]"#,
        r#""type":"string","pattern":"λ+","contentEncoding":"base64","contentMediaType":"text/plain""#,
        r#""type":"null""#,
        r#""type":"unknown""#,
        r#""extraType":"absent""#,
    ] {
        let input = format!("{{{common},{specific}}}");
        let typed: DataSchema =
            serde_json::from_str(&input).unwrap_or_else(|error| panic!("{input}: {error}"));
        for step in [1, 17, 4096] {
            let owner = literal(&input, step);
            arena::inspect(owner.view()).unwrap();
            fields(owner.view(), &typed);
            assert_eq!(
                arena::validate(owner.view()).is_ok(),
                typed.validate_with_level(ValidationLevel::Basic).is_ok(),
                "{input}"
            );
        }
    }
}

#[test]
fn field_conversion_acceptance_matches_current_public_source_for_logically_equal_values() {
    let placements = [
        ("@type", ""),
        ("title", ""),
        ("titles", ""),
        ("description", ""),
        ("descriptions", ""),
        ("const", ""),
        ("default", ""),
        ("unit", ""),
        ("oneOf", ""),
        ("enum", ""),
        ("readOnly", ""),
        ("writeOnly", ""),
        ("format", ""),
        ("type", ""),
        ("items", r#""type":"array","#),
        ("minItems", r#""type":"array","#),
        ("maxItems", r#""type":"array","#),
        ("properties", ""),
        ("required", ""),
        ("minLength", r#""type":"string","#),
        ("maxLength", r#""type":"string","#),
        ("pattern", r#""type":"string","#),
        ("contentEncoding", r#""type":"string","#),
        ("contentMediaType", r#""type":"string","#),
    ];
    let values = [
        "null",
        "true",
        "false",
        "0",
        "1",
        "-0",
        "2",
        "1.0",
        "1e0",
        "-1",
        "4294967296",
        r#""true""#,
        r#""false""#,
        r#""1""#,
        r#""0""#,
        r#""plain""#,
        r#""[true,false]""#,
        "[]",
        "{}",
        r#"["a","z"]"#,
        r#"{"en":"text"}"#,
        r#"[{},{}]"#,
        r#"{"nested":{}}"#,
    ];
    let mut checked = 0;
    for (field, prefix) in placements {
        for value in values {
            let input = format!(r#"{{{prefix}"{field}":{value}}}"#);
            let typed = serde_json::from_str::<DataSchema>(&input);
            let owner = literal(&input, 7);
            let strict = arena::inspect(owner.view());
            assert_eq!(
                strict.is_ok(),
                typed.is_ok(),
                "{input}: {typed:?} {strict:?}"
            );
            if let Ok(typed) = typed {
                fields(owner.view(), &typed);
                assert_eq!(
                    arena::validate(owner.view()).is_ok(),
                    typed.validate_with_level(ValidationLevel::Basic).is_ok(),
                    "{input}"
                );
            }
            checked += 1;
        }
    }
    for kind in ["number", "integer"] {
        for field in [
            "minimum",
            "exclusiveMinimum",
            "maximum",
            "exclusiveMaximum",
            "multipleOf",
        ] {
            for value in values.into_iter().chain([
                "1e309",
                "1e-4000",
                "9007199254740993",
                "18446744073709551616",
            ]) {
                let input = format!(r#"{{"type":"{kind}","{field}":{value}}}"#);
                let typed = serde_json::from_str::<DataSchema>(&input);
                let owner = literal(&input, 7);
                let strict = arena::inspect(owner.view());
                assert_eq!(
                    strict.is_ok(),
                    typed.is_ok(),
                    "{input}: {typed:?} {strict:?}"
                );
                if let Ok(typed) = typed {
                    fields(owner.view(), &typed);
                    assert_eq!(
                        arena::validate(owner.view()).is_ok(),
                        typed.validate_with_level(ValidationLevel::Basic).is_ok(),
                        "{input}"
                    );
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 822);
}
