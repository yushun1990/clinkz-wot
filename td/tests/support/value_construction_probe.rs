//! Connect the existing actual typed corpus to the budgeted value layer.
//! Test enumeration is outside the engine cursor and is not TD traversal proof.
use super::{
    td_crate::{data_schema::DataSchema, data_type::ExtensionMap},
    typed_corpus_shared,
};
use validated_thing_value_construction_probe::{Cursor, Limits};

#[path = "../../../tools/architecture-fixtures/validated-thing-value-construction/tests/support/mod.rs"]
#[allow(dead_code)]
mod support;

fn extensions(fields: &ExtensionMap, count: &mut usize) {
    for value in fields.values() {
        let output = support::drive(Cursor::from_value(value, Limits::default()), 1).unwrap();
        support::equivalent(output.view(), value);
        *count += 1;
    }
}

fn schema(schema: &DataSchema, count: &mut usize) {
    let context = schema.context();
    extensions(&context._extra_fields, count);
    for value in context
        .constant
        .iter()
        .chain(context.default.iter())
        .chain(context.enumerate.iter().flatten())
    {
        let output = support::drive(Cursor::from_value(value, Limits::default()), 1).unwrap();
        support::equivalent(output.view(), value);
        *count += 1;
    }
    for child in context.one_of.iter().flatten() {
        self::schema(child, count);
    }
    match schema {
        DataSchema::Array(array) => {
            for child in array.items.iter().flatten() {
                self::schema(child, count);
            }
        }
        DataSchema::Object(object) => {
            for child in object.properties.iter().flat_map(|map| map.values()) {
                self::schema(child, count);
            }
        }
        _ => {}
    }
}

#[test]
fn budgeted_value_cursor_consumes_existing_typed_and_serializer_failure_corpora() {
    let mut count = 0;
    for thing in [
        typed_corpus_shared::typed_corpus(),
        typed_corpus_shared::nested_schema_corpus(),
        typed_corpus_shared::serializer_failure_thing(),
    ] {
        extensions(&thing._extra_fields, &mut count);
        for property in thing.properties.iter().flat_map(|map| map.values()) {
            schema(&property._schema, &mut count);
        }
    }
    assert!(count > 0);
}
