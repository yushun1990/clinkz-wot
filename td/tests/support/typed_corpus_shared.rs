//! Fixed typed input shared by the #96 tests and the snapshot prototype.

extern crate alloc;
use self::alloc::vec;

use super::td_crate::{
    context::Context,
    data_schema::{
        ArraySchema, DataSchema, DataSchemaContext, IntegerSchema, NumberSchema, ObjectSchema,
        StringSchema,
    },
    data_type::MultiLanguage,
    thing::Thing,
    validate::{Validate, ValidationLevel},
};
use serde_json::Value;

pub const CORPUS: &str = r##"{
    "@context": [
        "https://www.w3.org/2022/wot/td/v1.1",
        { "ex": "https://example.org/ns#" },
        "https://example.org/extra-context"
    ],
    "id": "urn:example:typed-corpus",
    "title": "Typed corpus",
    "titles": { "en": "Typed corpus", "fr": "Corpus typé" },
    "support": "https://example.org/support",
    "base": "https://example.org/things/",
    "security": ["none"],
    "securityDefinitions": { "none": { "scheme": "nosec" } },
    "properties": {
        "zeta": {
            "type": "string",
            "forms": [
                { "href": "zeta/first", "op": ["readproperty", "writeproperty"], "contentType": "text/plain" },
                { "href": "zeta/second", "op": "readproperty" }
            ]
        },
        "alpha": {
            "type": "boolean",
            "forms": [{ "href": "alpha", "op": "readproperty" }]
        }
    },
    "forms": [],
    "ex:payload": {
        "flag": true,
        "count": 123456789012345678901234567890,
        "empty": null,
        "items": ["first", { "left": "L", "right": "R" }]
    }
}"##;

pub fn typed_corpus() -> Thing {
    let mut thing: Thing = serde_json::from_str(CORPUS).expect("fixed TD input must decode");
    thing
        ._extra_fields
        .insert("ex:long".into(), Value::String("λ".repeat(2_048)));
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("fixed typed corpus must pass Basic");
    thing
}

pub fn serializer_failure_thing() -> Thing {
    let mut thing = typed_corpus();
    thing.context =
        serde_json::from_str::<Context>(r#"["https://example.org/extension-only"]"#).unwrap();
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("Basic accepts extension-only Context");
    thing
}

pub fn nested_schema_corpus() -> Thing {
    let mut thing = typed_corpus();
    let label = DataSchema::String(StringSchema {
        _context: DataSchemaContext {
            data_type: Some("string".into()),
            enumerate: Some(vec![
                Value::String("warm".into()),
                Value::String("cold".into()),
            ]),
            ..Default::default()
        },
        min_length: Some(1),
        max_length: Some(24),
        pattern: Some("^[a-z]+$".into()),
        content_encoding: Some("utf-8".into()),
        content_media_type: Some("text/plain".into()),
    });
    let reading = DataSchema::Object(ObjectSchema {
        _context: DataSchemaContext::default(),
        properties: Some(
            [
                ("label".into(), label),
                (
                    "level".into(),
                    DataSchema::Integer(IntegerSchema {
                        minimum: Some(-10),
                        maximum: Some(10),
                        multiple_of: Some(2),
                        ..Default::default()
                    }),
                ),
                (
                    "ratio".into(),
                    DataSchema::Number(NumberSchema {
                        minimum: Some(0.25),
                        exclusive_maximum: Some(9.5),
                        ..Default::default()
                    }),
                ),
            ]
            .into(),
        ),
        required: Some(vec!["label".into(), "level".into()]),
    });
    let mut context = DataSchemaContext {
        data_type: Some("object".into()),
        constant: Some(serde_json::json!({"nested": [null, true]})),
        default: Some(serde_json::json!({"nested": []})),
        unit: Some("sample".into()),
        one_of: Some(vec![
            DataSchema::Boolean(Default::default()),
            DataSchema::Null(Default::default()),
        ]),
        enumerate: Some(vec![
            serde_json::json!({"a": 1}),
            serde_json::json!(["x", "y"]),
        ]),
        read_only: true,
        format: Some("custom-object".into()),
        ..Default::default()
    };
    context._metadata.tags = Some(vec!["Sensor".into(), "Sample".into()]);
    context._metadata.title = Some("Readings".into());
    context._metadata.titles = Some(
        MultiLanguage::new()
            .with("en", "Readings")
            .with("fr", "Mesures"),
    );
    context._metadata.description = Some("Nested readings".into());
    context._metadata.descriptions = Some(MultiLanguage::new().with("en", "Nested readings"));
    context
        ._extra_fields
        .insert("ex:opaque".into(), serde_json::from_str("1e309").unwrap());
    context._extra_fields.insert(
        "ex:deep".into(),
        serde_json::json!({"items": [false, {"key": "value"}]}),
    );
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema = DataSchema::Object(ObjectSchema {
        _context: context,
        properties: Some(
            [(
                "samples".into(),
                DataSchema::Array(ArraySchema {
                    items: Some(vec![reading, DataSchema::Null(Default::default())]),
                    min_items: Some(1),
                    max_items: Some(8),
                    ..Default::default()
                }),
            )]
            .into(),
        ),
        required: Some(vec!["samples".into()]),
    });
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("nested typed schema must pass Basic");
    thing
}
