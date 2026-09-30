//! The same representation-only adapter is compiled against real and candidate TD.
use super::schema_kernel::{ChildSite, Field, SchemaAccess, SchemaKind};
use crate::data_schema::DataSchema;

pub struct TypedAccess;

impl<'a> SchemaAccess<'a> for TypedAccess {
    type Node = &'a DataSchema;
    type Number = &'a serde_json::Number;

    fn kind(&self, node: Self::Node) -> SchemaKind {
        match node {
            DataSchema::Array(_) => SchemaKind::Array,
            DataSchema::Boolean(_) => SchemaKind::Boolean,
            DataSchema::Number(_) => SchemaKind::Number,
            DataSchema::Integer(_) => SchemaKind::Integer,
            DataSchema::Object(_) => SchemaKind::Object,
            DataSchema::String(_) => SchemaKind::String,
            DataSchema::Null(_) => SchemaKind::Null,
        }
    }
    fn data_type(&self, node: Self::Node) -> Option<&'a str> {
        node.context().data_type.as_deref()
    }
    fn flags(&self, node: Self::Node) -> (bool, bool) {
        (node.context().read_only, node.context().write_only)
    }
    fn one_of_count(&self, node: Self::Node) -> usize {
        node.context()
            .one_of
            .as_ref()
            .map_or(0, |values| values.len())
    }
    fn one_of(&self, node: Self::Node, index: usize) -> Self::Node {
        &node.context().one_of.as_ref().unwrap()[index]
    }
    fn child_count(&self, node: Self::Node) -> usize {
        match node {
            DataSchema::Array(value) => value.items.as_ref().map_or(0, |values| values.len()),
            DataSchema::Object(value) => value.properties.as_ref().map_or(0, |values| values.len()),
            _ => 0,
        }
    }
    fn child(&self, node: Self::Node, index: usize) -> (ChildSite<'a>, Self::Node) {
        match node {
            DataSchema::Array(value) => (
                ChildSite::Indexed(index),
                &value.items.as_ref().unwrap()[index],
            ),
            DataSchema::Object(value) => {
                let (key, child) = value
                    .properties
                    .as_ref()
                    .unwrap()
                    .iter()
                    .nth(index)
                    .unwrap();
                (ChildSite::Property(key), child)
            }
            _ => unreachable!("no child on scalar schema"),
        }
    }
    fn unsigned_extension(&self, node: Self::Node, field: Field) -> Option<u64> {
        node.context()
            ._extra_fields
            .get(field.name())
            .and_then(serde_json::Value::as_u64)
    }
    fn number_extension(&self, node: Self::Node, field: Field) -> Option<Self::Number> {
        node.context()
            ._extra_fields
            .get(field.name())
            .and_then(serde_json::Value::as_number)
    }
    fn project_number(&self, number: Self::Number) -> Option<f64> {
        number.as_f64()
    }
    fn typed_unsigned(&self, node: Self::Node) -> (Option<u32>, Option<u32>) {
        match node {
            DataSchema::Array(value) => (value.min_items, value.max_items),
            DataSchema::String(value) => (value.min_length, value.max_length),
            _ => (None, None),
        }
    }
    fn typed_float(&self, node: Self::Node) -> [Option<f64>; 5] {
        let DataSchema::Number(value) = node else {
            unreachable!()
        };
        [
            value.minimum,
            value.exclusive_minimum,
            value.maximum,
            value.exclusive_maximum,
            value.multiple_of,
        ]
    }
    fn typed_integer(&self, node: Self::Node) -> [Option<i64>; 5] {
        let DataSchema::Integer(value) = node else {
            unreachable!()
        };
        [
            value.minimum,
            value.exclusive_minimum,
            value.maximum,
            value.exclusive_maximum,
            value.multiple_of,
        ]
    }
}
