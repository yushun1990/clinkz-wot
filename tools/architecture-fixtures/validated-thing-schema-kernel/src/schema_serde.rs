//! Established owning serde representations around the shared field policy.
use super::schema_fields::{self as policy, Context, Decoded, Field, Metadata, Shape};
use crate::{
    data_schema::*,
    data_type::{ExtensionMap, METADATA_KEYS, Metadata as TdMetadata, MultiLanguage},
};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use serde::Deserialize;
use serde_json::{Value, from_value};

pub struct Source {
    map: crate::flat::JsonMap,
}
impl Source {
    pub fn new(map: crate::flat::JsonMap) -> Self {
        Self { map }
    }
}

#[derive(Deserialize)]
struct Flexible(
    #[serde(deserialize_with = "crate::components::util::deserialize_bool_flexible")] bool,
);

impl policy::Source for Source {
    type Value = Value;
    type Text = String;
    type Values = Vec<Value>;
    type Strings = Vec<String>;
    type Languages = MultiLanguage;
    type Schemas = Vec<DataSchema>;
    type SchemaMap = BTreeMap<String, DataSchema>;
    type Extras = ExtensionMap;
    type Error = serde_json::Error;
    fn take(&mut self, field: Field) -> Option<Value> {
        self.map.remove(field.name())
    }
    fn is_null(value: &Value) -> bool {
        value.is_null()
    }
    fn literal(&mut self, value: Value) -> Result<Value, Self::Error> {
        from_value(value)
    }
    fn text(&mut self, value: Value) -> Result<String, Self::Error> {
        from_value(value)
    }
    fn values(&mut self, value: Value) -> Result<Vec<Value>, Self::Error> {
        from_value(value)
    }
    fn strings(&mut self, value: Value) -> Result<Vec<String>, Self::Error> {
        from_value(value)
    }
    fn strings_many(&mut self, value: Value) -> Result<Vec<String>, Self::Error> {
        #[serde_with::serde_as]
        #[derive(Deserialize)]
        struct Many(#[serde_as(as = "serde_with::OneOrMany<_>")] Vec<String>);
        from_value::<Many>(value).map(|many| many.0)
    }
    fn languages(&mut self, value: Value) -> Result<MultiLanguage, Self::Error> {
        from_value(value)
    }
    fn schemas(&mut self, value: Value) -> Result<Vec<DataSchema>, Self::Error> {
        from_value(value)
    }
    fn schemas_many(&mut self, value: Value) -> Result<Vec<DataSchema>, Self::Error> {
        // Preserve take_one_or_many's per-element from_value boundary rather
        // than substituting serde_with's intermediate Content representation.
        match value {
            Value::Array(values) => values.into_iter().map(from_value).collect(),
            other => from_value(other).map(|value| alloc::vec![value]),
        }
    }
    fn schema_map(&mut self, value: Value) -> Result<Self::SchemaMap, Self::Error> {
        from_value(value)
    }
    fn boolean(&mut self, value: Value) -> Result<bool, Self::Error> {
        from_value::<Flexible>(value).map(|v| v.0)
    }
    fn unsigned(&mut self, value: Value) -> Result<u32, Self::Error> {
        from_value(value)
    }
    fn float(&mut self, value: Value) -> Result<f64, Self::Error> {
        from_value(value)
    }
    fn integer(&mut self, value: Value) -> Result<i64, Self::Error> {
        from_value(value)
    }
    fn metadata(&mut self) -> Result<Metadata<Self>, Self::Error> {
        let metadata =
            crate::flat::drain_substruct::<TdMetadata, Self::Error>(&mut self.map, METADATA_KEYS)?;
        Ok(meta_from_td(metadata))
    }
    fn remaining_context(self) -> Result<Context<Self>, Self::Error> {
        // This re-entry is intentional: ordinary Value/RawValue interpretation
        // at flattened composition boundaries remains unchanged.
        crate::flat::from_remaining::<DataSchemaContext, Self::Error>(self.map).map(context_from_td)
    }
    fn extras(self) -> ExtensionMap {
        crate::flat::into_extras(self.map)
    }
}

fn meta_from_td(value: TdMetadata) -> Metadata<Source> {
    Metadata {
        tags: value.tags,
        title: value.title,
        titles: value.titles,
        description: value.description,
        descriptions: value.descriptions,
    }
}
fn meta_into_td(value: Metadata<Source>) -> TdMetadata {
    TdMetadata {
        tags: value.tags,
        title: value.title,
        titles: value.titles,
        description: value.description,
        descriptions: value.descriptions,
    }
}
fn context_from_td(value: DataSchemaContext) -> Context<Source> {
    Context {
        metadata: meta_from_td(value._metadata),
        constant: value.constant,
        default: value.default,
        unit: value.unit,
        one_of: value.one_of,
        enumerate: value.enumerate,
        read_only: value.read_only,
        write_only: value.write_only,
        format: value.format,
        data_type: value.data_type,
        extras: value._extra_fields,
    }
}
pub fn context_into_td(value: Context<Source>) -> DataSchemaContext {
    DataSchemaContext {
        _metadata: meta_into_td(value.metadata),
        constant: value.constant,
        default: value.default,
        unit: value.unit,
        one_of: value.one_of,
        enumerate: value.enumerate,
        read_only: value.read_only,
        write_only: value.write_only,
        format: value.format,
        data_type: value.data_type,
        _extra_fields: value.extras,
    }
}

pub fn into_td(decoded: Decoded<Source>) -> DataSchema {
    let context = context_into_td(decoded.context);
    match decoded.shape {
        Shape::Array { items, min, max } => DataSchema::Array(ArraySchema {
            _context: context,
            items,
            min_items: min,
            max_items: max,
        }),
        Shape::Boolean => DataSchema::Boolean(BooleanSchema { _context: context }),
        Shape::Number(
            [
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            ],
        ) => DataSchema::Number(NumberSchema {
            _context: context,
            minimum,
            exclusive_minimum,
            maximum,
            exclusive_maximum,
            multiple_of,
        }),
        Shape::Integer(
            [
                minimum,
                exclusive_minimum,
                maximum,
                exclusive_maximum,
                multiple_of,
            ],
        ) => DataSchema::Integer(IntegerSchema {
            _context: context,
            minimum,
            exclusive_minimum,
            maximum,
            exclusive_maximum,
            multiple_of,
        }),
        Shape::Object {
            properties,
            required,
        } => DataSchema::Object(ObjectSchema {
            _context: context,
            properties,
            required,
        }),
        Shape::String {
            min,
            max,
            pattern,
            encoding,
            media_type,
        } => DataSchema::String(StringSchema {
            _context: context,
            min_length: min,
            max_length: max,
            pattern,
            content_encoding: encoding,
            content_media_type: media_type,
        }),
        Shape::Null => DataSchema::Null(NullSchema { _context: context }),
    }
}
