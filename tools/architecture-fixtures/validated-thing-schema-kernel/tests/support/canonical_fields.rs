use serde_json::Value;
use validated_thing_schema_kernel_probe::{
    data_schema::DataSchema, data_type::MultiLanguage, schema_build::View,
    schema_fields::Field as F, schema_kernel::SchemaKind,
};
use validated_thing_value_construction_probe::Kind;
pub fn optional<T: std::fmt::Debug>(
    actual: Option<View<'_>>,
    expected: Option<T>,
    check: impl FnOnce(View<'_>, T),
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) => check(actual, expected),
        (None, None) => {}
        _ => panic!("optional distinction"),
    }
}
pub fn strings(view: View<'_>, expected: &[String]) {
    assert_eq!(view.literal_kind(), Some(Kind::Array));
    assert_eq!(view.len(), expected.len());
    for (i, text) in expected.iter().enumerate() {
        assert_eq!(view.child(i).unwrap().text(), Some(text.as_str()));
    }
}
pub fn languages(view: View<'_>, expected: &MultiLanguage) {
    assert_eq!(view.len(), expected.len());
    for i in 0..view.len() {
        let (key, value) = view.member(i).unwrap();
        assert_eq!(value.text(), expected.get(key).map(String::as_str));
    }
}
pub fn literal(view: View<'_>, expected: &Value) {
    match expected {
        Value::Null => assert_eq!(view.literal_kind(), Some(Kind::Null)),
        Value::Bool(v) => assert_eq!(
            view.literal_kind(),
            Some(if *v { Kind::True } else { Kind::False })
        ),
        Value::String(v) => {
            assert_eq!(view.literal_kind(), Some(Kind::String));
            assert_eq!(view.text(), Some(v.as_str()));
        }
        Value::Number(v) => {
            assert_eq!(view.literal_kind(), Some(Kind::Number));
            assert_eq!(view.text(), Some(v.as_str()));
        }
        Value::Array(values) => {
            assert_eq!(view.literal_kind(), Some(Kind::Array));
            assert_eq!(view.len(), values.len());
            for (i, value) in values.iter().enumerate() {
                literal(view.child(i).unwrap(), value);
            }
        }
        Value::Object(values) => {
            assert_eq!(view.literal_kind(), Some(Kind::Object));
            assert_eq!(view.len(), values.len());
            let mut names = values.keys().collect::<Vec<_>>();
            names.sort();
            for (i, key) in names.into_iter().enumerate() {
                let (name, value) = view.member(i).unwrap();
                assert_eq!(name, key);
                literal(value, &values[key]);
            }
        }
    }
}
pub fn fields(view: View<'_>, source: &DataSchema) {
    let context = match source {
        DataSchema::Array(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::Array));
            optional(view.field(F::Items), v.items.as_deref(), schemas);
            optional(view.field(F::MinItems), v.min_items, |view, value| {
                assert_eq!(view.unsigned(), Some(value))
            });
            optional(view.field(F::MaxItems), v.max_items, |view, value| {
                assert_eq!(view.unsigned(), Some(value))
            });
            &v._context
        }
        DataSchema::Object(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::Object));
            optional(
                view.field(F::Properties),
                v.properties.as_ref(),
                |view, values| {
                    assert_eq!(view.len(), values.len());
                    for (i, (key, schema)) in values.iter().enumerate() {
                        let (name, value) = view.member(i).unwrap();
                        assert_eq!(name, key);
                        fields(value, schema);
                    }
                },
            );
            optional(view.field(F::Required), v.required.as_deref(), strings);
            &v._context
        }
        DataSchema::String(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::String));
            for (field, value) in [(F::MinLength, v.min_length), (F::MaxLength, v.max_length)] {
                optional(view.field(field), value, |view, value| {
                    assert_eq!(view.unsigned(), Some(value))
                });
            }
            for (field, value) in [
                (F::Pattern, v.pattern.as_deref()),
                (F::ContentEncoding, v.content_encoding.as_deref()),
                (F::ContentMediaType, v.content_media_type.as_deref()),
            ] {
                optional(view.field(field), value, |view, value| {
                    assert_eq!(view.text(), Some(value))
                });
            }
            &v._context
        }
        DataSchema::Number(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::Number));
            for (field, value) in [
                F::Minimum,
                F::ExclusiveMinimum,
                F::Maximum,
                F::ExclusiveMaximum,
                F::MultipleOf,
            ]
            .into_iter()
            .zip([
                v.minimum,
                v.exclusive_minimum,
                v.maximum,
                v.exclusive_maximum,
                v.multiple_of,
            ]) {
                optional(view.field(field), value, |view, value| {
                    assert_eq!(view.float().unwrap().to_bits(), value.to_bits())
                });
            }
            &v._context
        }
        DataSchema::Integer(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::Integer));
            for (field, value) in [
                F::Minimum,
                F::ExclusiveMinimum,
                F::Maximum,
                F::ExclusiveMaximum,
                F::MultipleOf,
            ]
            .into_iter()
            .zip([
                v.minimum,
                v.exclusive_minimum,
                v.maximum,
                v.exclusive_maximum,
                v.multiple_of,
            ]) {
                optional(view.field(field), value, |view, value| {
                    assert_eq!(view.integer(), Some(value))
                });
            }
            &v._context
        }
        DataSchema::Boolean(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::Boolean));
            &v._context
        }
        DataSchema::Null(v) => {
            assert_eq!(view.schema_kind(), Some(SchemaKind::Null));
            &v._context
        }
    };
    let metadata = &context._metadata;
    optional(view.field(F::Tags), metadata.tags.as_deref(), strings);
    optional(
        view.field(F::Title),
        metadata.title.as_deref(),
        |view, value| assert_eq!(view.text(), Some(value)),
    );
    optional(
        view.field(F::Description),
        metadata.description.as_deref(),
        |view, value| assert_eq!(view.text(), Some(value)),
    );
    optional(view.field(F::Titles), metadata.titles.as_ref(), languages);
    optional(
        view.field(F::Descriptions),
        metadata.descriptions.as_ref(),
        languages,
    );
    optional(view.field(F::Const), context.constant.as_ref(), literal);
    optional(view.field(F::Default), context.default.as_ref(), literal);
    optional(view.field(F::OneOf), context.one_of.as_deref(), schemas);
    optional(
        view.field(F::Enum),
        context.enumerate.as_deref(),
        |view, values| {
            assert_eq!(view.len(), values.len());
            for (i, value) in values.iter().enumerate() {
                literal(view.child(i).unwrap(), value);
            }
        },
    );
    for (field, value) in [
        (F::Unit, context.unit.as_deref()),
        (F::Format, context.format.as_deref()),
        (F::Type, context.data_type.as_deref()),
    ] {
        optional(view.field(field), value, |view, value| {
            assert_eq!(view.text(), Some(value))
        });
    }
    for (field, value) in [
        (F::ReadOnly, context.read_only),
        (F::WriteOnly, context.write_only),
    ] {
        assert_eq!(
            view.field(field).unwrap().literal_kind(),
            Some(if value { Kind::True } else { Kind::False })
        );
    }
    assert_eq!(view.extras().len(), context._extra_fields.len());
    for (i, (key, value)) in context._extra_fields.iter().enumerate() {
        let (name, child) = view.extras().member(i).unwrap();
        assert_eq!(name, key);
        literal(child, value);
    }
}
pub fn schemas(view: View<'_>, source: &[DataSchema]) {
    assert_eq!(view.literal_kind(), Some(Kind::Array));
    assert_eq!(view.len(), source.len());
    for (i, schema) in source.iter().enumerate() {
        fields(view.child(i).unwrap(), schema);
    }
}
