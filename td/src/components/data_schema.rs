use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::util::deserialize_bool_flexible;
use crate::data_type::{ExtensionMap, METADATA_KEYS, Metadata, MetadataHelper};
use crate::validate::{Validate, ValidateError, ValidationLevel};

/// Deserialize adapter carrying the flexible-bool decoder used by `readOnly`
/// and `writeOnly`.
#[derive(Deserialize)]
struct FlexBoolField(#[serde(deserialize_with = "deserialize_bool_flexible")] bool);

/// Shared base for every data schema variant: flattened metadata, the JSON
/// Schema core constraints, and preserved extension fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataSchemaContext {
    pub _metadata: Metadata,

    /// Provides a const value.
    pub constant: Option<serde_json::Value>,

    /// Supply a default value.
    pub default: Option<serde_json::Value>,

    /// Provides unit information that is used.
    pub unit: Option<String>,

    /// Used to ensure that the data is valid against one of the
    /// specified schemas in the array.
    pub one_of: Option<Vec<DataSchema>>,

    /// Restricted set of values provided as an array.
    pub enumerate: Option<Vec<serde_json::Value>>,

    /// Indicate whether a property value is read only.
    pub read_only: bool,

    /// Indicate whether a property value is write only.
    pub write_only: bool,

    /// Allows validation based on a format pattern such as
    /// "date-time", "email", "uri", etc.
    pub format: Option<String>,

    /// Assignment of JSON based data types compatible with JSON schema.
    pub data_type: Option<String>,

    pub _extra_fields: ExtensionMap,
}

impl<'de> Deserialize<'de> for DataSchemaContext {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let metadata = crate::flat::drain_substruct::<Metadata, D::Error>(&mut map, METADATA_KEYS)?;
        let constant = crate::flat::take(&mut map, "const")?;
        let default = crate::flat::take(&mut map, "default")?;
        let unit = crate::flat::take(&mut map, "unit")?;
        let one_of = crate::flat::take(&mut map, "oneOf")?;
        let enumerate = crate::flat::take(&mut map, "enum")?;
        let read_only = match crate::flat::take::<FlexBoolField, D::Error>(&mut map, "readOnly")? {
            Some(field) => field.0,
            None => false,
        };
        let write_only = match crate::flat::take::<FlexBoolField, D::Error>(&mut map, "writeOnly")?
        {
            Some(field) => field.0,
            None => false,
        };
        let format = crate::flat::take(&mut map, "format")?;
        let data_type = crate::flat::take(&mut map, "type")?;
        Ok(DataSchemaContext {
            _metadata: metadata,
            constant,
            default,
            unit,
            one_of,
            enumerate,
            read_only,
            write_only,
            format,
            data_type,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for DataSchemaContext {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self.serialize_into(&mut map)?;
        map.end()
    }
}

impl DataSchemaContext {
    /// Emits this context's fields inline into an in-progress serialized map.
    /// Used by schema variants that previously `#[serde(flatten)]`-ed the
    /// context, so its fields appear at the variant level without nesting.
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        self._metadata.serialize_into(map)?;
        if let Some(constant) = &self.constant {
            map.serialize_entry("const", constant)?;
        }
        if let Some(default) = &self.default {
            map.serialize_entry("default", default)?;
        }
        if let Some(unit) = &self.unit {
            map.serialize_entry("unit", unit)?;
        }
        if let Some(one_of) = &self.one_of {
            map.serialize_entry("oneOf", one_of)?;
        }
        if let Some(enumerate) = &self.enumerate {
            map.serialize_entry("enum", enumerate)?;
        }
        if self.read_only {
            map.serialize_entry("readOnly", &self.read_only)?;
        }
        if self.write_only {
            map.serialize_entry("writeOnly", &self.write_only)?;
        }
        if let Some(format) = &self.format {
            map.serialize_entry("format", format)?;
        }
        if let Some(data_type) = &self.data_type {
            map.serialize_entry("type", data_type)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        Ok(())
    }
}

pub trait ContextHelper: MetadataHelper {
    fn context(&mut self) -> &mut DataSchemaContext;

    /// Sets the const value.
    fn constant(mut self, constant: serde_json::Value) -> Self {
        self.context().constant = Some(constant);
        self
    }

    /// Sets the default value.
    fn default(mut self, default: serde_json::Value) -> Self {
        self.context().default = Some(default);
        self
    }

    /// Sets the unit.
    fn unit(mut self, unit: impl Into<String>) -> Self {
        self.context().unit = Some(unit.into());
        self
    }

    /// Adds schemas to one_of.
    fn one_of<I>(mut self, schemas: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<DataSchema>,
    {
        self.context()
            .one_of
            .get_or_insert_with(Vec::new)
            .extend(schemas.into_iter().map(Into::into));
        self
    }

    /// Adds values to enum.
    fn enumerate<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = serde_json::Value>,
    {
        self.context()
            .enumerate
            .get_or_insert_with(Vec::new)
            .extend(values);
        self
    }

    /// Sets read_only.
    fn read_only(mut self, read_only: bool) -> Self {
        self.context().read_only = read_only;
        self
    }

    /// Sets write_only.
    fn write_only(mut self, write_only: bool) -> Self {
        self.context().write_only = write_only;
        self
    }

    /// Sets the format.
    fn format(mut self, format: impl Into<String>) -> Self {
        self.context().format = Some(format.into());
        self
    }

    /// Sets the type.
    fn data_type(mut self, data_type: impl Into<String>) -> Self {
        self.context().data_type = Some(data_type.into());
        self
    }

    /// Sets extension fields.
    fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.context()._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context()._extra_fields.insert(key.into(), value);
        self
    }
}

/// Metadata describing data of type array.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArraySchema {
    pub _context: DataSchemaContext,

    /// Used to define the characteristic of an array.
    pub items: Option<Vec<DataSchema>>,

    /// Define the minimum number of items that have to be in the array.
    pub min_items: Option<u32>,

    /// Define the maximum number of items that have to be in the array.
    pub max_items: Option<u32>,
}

impl<'de> Deserialize<'de> for ArraySchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        // `items` uses the W3C one-or-many convention (a single DataSchema or an
        // array). It is read via `take_one_or_many` rather than
        // `serde_with::OneOrMany` because the latter buffers through serde's
        // `Content` layer, which would block the `Box<RawValue>` capture that
        // `DataSchema` now relies on.
        let items = crate::flat::take_one_or_many::<DataSchema, D::Error>(&mut map, "items")?;
        let min_items = crate::flat::take(&mut map, "minItems")?;
        let max_items = crate::flat::take(&mut map, "maxItems")?;
        let context = crate::flat::from_remaining::<DataSchemaContext, D::Error>(map)?;
        Ok(ArraySchema {
            _context: context,
            items,
            min_items,
            max_items,
        })
    }
}

impl Serialize for ArraySchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self.serialize_into(&mut map)?;
        map.end()
    }
}

impl ArraySchema {
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        self._context.serialize_into(map)?;
        if let Some(items) = &self.items {
            map.serialize_entry("items", &crate::flat::OneOrManyRef(items))?;
        }
        if let Some(min_items) = self.min_items {
            map.serialize_entry("minItems", &min_items)?;
        }
        if let Some(max_items) = self.max_items {
            map.serialize_entry("maxItems", &max_items)?;
        }
        Ok(())
    }
}

impl ArraySchema {
    pub fn builder() -> ArraySchemaBuilder {
        ArraySchemaBuilder::new()
    }
}

/// Builder for creating `ArraySchema` instances.
pub struct ArraySchemaBuilder {
    schema: ArraySchema,
}

impl ArraySchemaBuilder {
    /// Creates a new `ArraySchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: ArraySchema {
                _context: DataSchemaContext::default(),
                items: None,
                min_items: None,
                max_items: None,
            },
        }
    }

    /// Adds items schemas.
    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<DataSchema>,
    {
        self.schema
            .items
            .get_or_insert_with(Vec::new)
            .extend(items.into_iter().map(Into::into));
        self
    }

    /// Sets the minimum number of items.
    pub fn min_items(mut self, min_items: u32) -> Self {
        self.schema.min_items = Some(min_items);
        self
    }

    /// Sets the maximum number of items.
    pub fn max_items(mut self, max_items: u32) -> Self {
        self.schema.max_items = Some(max_items);
        self
    }

    /// Builds and returns the `ArraySchema` instance.
    pub fn build(self) -> ArraySchema {
        self.schema
    }
}

impl ContextHelper for ArraySchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for ArraySchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type boolean.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BooleanSchema {
    pub _context: DataSchemaContext,
}

impl<'de> Deserialize<'de> for BooleanSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = crate::flat::deserialize_map(deserializer)?;
        let context = crate::flat::from_remaining::<DataSchemaContext, D::Error>(map)?;
        Ok(BooleanSchema { _context: context })
    }
}

impl Serialize for BooleanSchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        map.end()
    }
}

impl BooleanSchema {
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        self._context.serialize_into(map)
    }
}

impl BooleanSchema {
    pub fn builder() -> BooleanSchemaBuilder {
        BooleanSchemaBuilder::new()
    }
}

/// Builder for creating `BooleanSchema` instances.
pub struct BooleanSchemaBuilder {
    schema: BooleanSchema,
}

impl BooleanSchemaBuilder {
    /// Creates a new `BooleanSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: BooleanSchema {
                _context: DataSchemaContext::default(),
            },
        }
    }

    /// Builds and returns the `BooleanSchema` instance.
    pub fn build(self) -> BooleanSchema {
        self.schema
    }
}

impl ContextHelper for BooleanSchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for BooleanSchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Generates a numeric DataSchema variant (`NumberSchema` / `IntegerSchema`)
/// together with its manual `Deserialize` / `Serialize` (preserving extension
/// fields via `from_remaining`), `serialize_into`, builder, `ContextHelper` /
/// `MetadataHelper`, and `From` impls. The two variants are structurally
/// identical except for the numeric type (`f64` vs `i64`), so the macro
/// eliminates ~150 lines of pure duplication.
macro_rules! impl_numeric_schema {
    (
        $name:ident, $builder:ident, $variant:ident, $ty:ty, $doc:expr
    ) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct $name {
            pub _context: DataSchemaContext,

            /// Specifies a minimum numeric value, representing an inclusive
            /// lower limit.
            pub minimum: Option<$ty>,

            /// Specifies a minimum numeric value, representing an exclusive
            /// lower limit.
            pub exclusive_minimum: Option<$ty>,

            /// Specifies a maximum numeric value, representing an inclusive
            /// upper limit.
            pub maximum: Option<$ty>,

            /// Specifies a maximum numeric value, representing an exclusive
            /// upper limit.
            pub exclusive_maximum: Option<$ty>,

            /// Specifies the `multipleOf` value.
            ///
            /// Basic validation requires this value to be strictly greater than 0.
            pub multiple_of: Option<$ty>,
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let mut map = crate::flat::deserialize_map(deserializer)?;
                let minimum = crate::flat::take(&mut map, "minimum")?;
                let exclusive_minimum = crate::flat::take(&mut map, "exclusiveMinimum")?;
                let maximum = crate::flat::take(&mut map, "maximum")?;
                let exclusive_maximum = crate::flat::take(&mut map, "exclusiveMaximum")?;
                let multiple_of = crate::flat::take(&mut map, "multipleOf")?;
                let context = crate::flat::from_remaining::<DataSchemaContext, D::Error>(map)?;
                Ok($name {
                    _context: context,
                    minimum,
                    exclusive_minimum,
                    maximum,
                    exclusive_maximum,
                    multiple_of,
                })
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(None)?;
                self.serialize_into(&mut map)?;
                map.end()
            }
        }

        impl $name {
            pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
                &self,
                map: &mut S,
            ) -> Result<(), S::Error> {
                self._context.serialize_into(map)?;
                if let Some(v) = self.minimum {
                    map.serialize_entry("minimum", &v)?;
                }
                if let Some(v) = self.exclusive_minimum {
                    map.serialize_entry("exclusiveMinimum", &v)?;
                }
                if let Some(v) = self.maximum {
                    map.serialize_entry("maximum", &v)?;
                }
                if let Some(v) = self.exclusive_maximum {
                    map.serialize_entry("exclusiveMaximum", &v)?;
                }
                if let Some(v) = self.multiple_of {
                    map.serialize_entry("multipleOf", &v)?;
                }
                Ok(())
            }

            pub fn builder() -> $builder {
                $builder::new()
            }
        }

        /// Builder for creating a `$name` instance.
        pub struct $builder {
            schema: $name,
        }

        impl $builder {
            pub fn new() -> Self {
                Self {
                    schema: $name::default(),
                }
            }

            pub fn minimum(mut self, v: $ty) -> Self {
                self.schema.minimum = Some(v);
                self
            }

            pub fn exclusive_minimum(mut self, v: $ty) -> Self {
                self.schema.exclusive_minimum = Some(v);
                self
            }

            pub fn maximum(mut self, v: $ty) -> Self {
                self.schema.maximum = Some(v);
                self
            }

            pub fn exclusive_maximum(mut self, v: $ty) -> Self {
                self.schema.exclusive_maximum = Some(v);
                self
            }

            pub fn multiple_of(mut self, v: $ty) -> Self {
                self.schema.multiple_of = Some(v);
                self
            }

            pub fn build(self) -> $name {
                self.schema
            }
        }

        impl ContextHelper for $builder {
            fn context(&mut self) -> &mut DataSchemaContext {
                &mut self.schema._context
            }
        }

        impl MetadataHelper for $builder {
            fn metadata(&mut self) -> &mut Metadata {
                &mut self.context()._metadata
            }
        }

        impl From<$name> for DataSchema {
            fn from(schema: $name) -> Self {
                Self::$variant(schema)
            }
        }

        impl From<$builder> for DataSchema {
            fn from(builder: $builder) -> Self {
                builder.build().into()
            }
        }
    };
}

impl_numeric_schema!(
    NumberSchema,
    NumberSchemaBuilder,
    Number,
    f64,
    "Metadata describing data of type number."
);
impl_numeric_schema!(
    IntegerSchema,
    IntegerSchemaBuilder,
    Integer,
    i64,
    "Metadata describing data of type integer."
);
/// Metadata describing data of type Object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectSchema {
    pub _context: DataSchemaContext,

    /// Data schema nested definitions.
    pub properties: Option<BTreeMap<String, DataSchema>>,

    /// Defines which members of the object type are mandatory.
    pub required: Option<Vec<String>>,
}

impl<'de> Deserialize<'de> for ObjectSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let properties = crate::flat::take(&mut map, "properties")?;
        let required = crate::flat::take(&mut map, "required")?;
        let context = crate::flat::from_remaining::<DataSchemaContext, D::Error>(map)?;
        Ok(ObjectSchema {
            _context: context,
            properties,
            required,
        })
    }
}

impl Serialize for ObjectSchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self.serialize_into(&mut map)?;
        map.end()
    }
}

impl ObjectSchema {
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        self._context.serialize_into(map)?;
        if let Some(properties) = &self.properties {
            map.serialize_entry("properties", properties)?;
        }
        if let Some(required) = &self.required {
            map.serialize_entry("required", required)?;
        }
        Ok(())
    }
}

impl ObjectSchema {
    pub fn builder() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }
}

/// Builder for creating `ObjectSchema` instances.
pub struct ObjectSchemaBuilder {
    schema: ObjectSchema,
}

impl ObjectSchemaBuilder {
    /// Creates a new `ObjectSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: ObjectSchema {
                _context: DataSchemaContext::default(),
                properties: None,
                required: None,
            },
        }
    }

    /// Adds a property.
    pub fn property(mut self, name: impl Into<String>, schema: impl Into<DataSchema>) -> Self {
        let properties = self.schema.properties.get_or_insert_with(BTreeMap::new);
        properties.insert(name.into(), schema.into());
        self
    }

    /// Adds multiple properties.
    pub fn properties<I, S, D>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item = (S, D)>,
        S: Into<String>,
        D: Into<DataSchema>,
    {
        let map = self.schema.properties.get_or_insert_with(BTreeMap::new);
        for (name, schema) in properties {
            map.insert(name.into(), schema.into());
        }
        self
    }

    /// Adds required fields.
    pub fn required<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.schema
            .required
            .get_or_insert_with(Vec::new)
            .extend(fields.into_iter().map(|s| s.into()));
        self
    }

    /// Builds and returns the `ObjectSchema` instance.
    pub fn build(self) -> ObjectSchema {
        self.schema
    }
}

impl ContextHelper for ObjectSchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for ObjectSchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type string.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StringSchema {
    pub _context: DataSchemaContext,

    /// Specifies the minimum length of a string.
    pub min_length: Option<u32>,

    /// Specifies the maximum length of a string.
    pub max_length: Option<u32>,

    /// Provides a regular expression to express constraints of
    /// the string value.
    pub pattern: Option<String>,

    /// Specifies the encoding used to store the contents.
    pub content_encoding: Option<String>,

    /// Specifies the MIME type of the contents of a string value.
    pub content_media_type: Option<String>,
}

impl<'de> Deserialize<'de> for StringSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let min_length = crate::flat::take(&mut map, "minLength")?;
        let max_length = crate::flat::take(&mut map, "maxLength")?;
        let pattern = crate::flat::take(&mut map, "pattern")?;
        let content_encoding = crate::flat::take(&mut map, "contentEncoding")?;
        let content_media_type = crate::flat::take(&mut map, "contentMediaType")?;
        let context = crate::flat::from_remaining::<DataSchemaContext, D::Error>(map)?;
        Ok(StringSchema {
            _context: context,
            min_length,
            max_length,
            pattern,
            content_encoding,
            content_media_type,
        })
    }
}

impl Serialize for StringSchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self.serialize_into(&mut map)?;
        map.end()
    }
}

impl StringSchema {
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        self._context.serialize_into(map)?;
        if let Some(min_length) = self.min_length {
            map.serialize_entry("minLength", &min_length)?;
        }
        if let Some(max_length) = self.max_length {
            map.serialize_entry("maxLength", &max_length)?;
        }
        if let Some(pattern) = &self.pattern {
            map.serialize_entry("pattern", pattern)?;
        }
        if let Some(content_encoding) = &self.content_encoding {
            map.serialize_entry("contentEncoding", content_encoding)?;
        }
        if let Some(content_media_type) = &self.content_media_type {
            map.serialize_entry("contentMediaType", content_media_type)?;
        }
        Ok(())
    }
}

impl StringSchema {
    pub fn builder() -> StringSchemaBuilder {
        StringSchemaBuilder::new()
    }
}

/// Builder for creating `StringSchema` instances.
pub struct StringSchemaBuilder {
    schema: StringSchema,
}

impl StringSchemaBuilder {
    /// Creates a new `StringSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: StringSchema {
                _context: DataSchemaContext::default(),
                min_length: None,
                max_length: None,
                pattern: None,
                content_encoding: None,
                content_media_type: None,
            },
        }
    }

    /// Sets the minimum length.
    pub fn min_length(mut self, min_length: u32) -> Self {
        self.schema.min_length = Some(min_length);
        self
    }

    /// Sets the maximum length.
    pub fn max_length(mut self, max_length: u32) -> Self {
        self.schema.max_length = Some(max_length);
        self
    }

    /// Sets the pattern.
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.schema.pattern = Some(pattern.into());
        self
    }

    /// Sets the content encoding.
    pub fn content_encoding(mut self, content_encoding: impl Into<String>) -> Self {
        self.schema.content_encoding = Some(content_encoding.into());
        self
    }

    /// Sets the content media type.
    pub fn content_media_type(mut self, content_media_type: impl Into<String>) -> Self {
        self.schema.content_media_type = Some(content_media_type.into());
        self
    }

    /// Builds and returns the `StringSchema` instance.
    pub fn build(self) -> StringSchema {
        self.schema
    }
}

impl ContextHelper for StringSchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for StringSchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type string.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NullSchema {
    pub _context: DataSchemaContext,
}

impl<'de> Deserialize<'de> for NullSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = crate::flat::deserialize_map(deserializer)?;
        let context = crate::flat::from_remaining::<DataSchemaContext, D::Error>(map)?;
        Ok(NullSchema { _context: context })
    }
}

impl Serialize for NullSchema {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        map.end()
    }
}

impl NullSchema {
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        self._context.serialize_into(map)
    }
}

impl NullSchema {
    pub fn builder() -> NullSchemaBuilder {
        NullSchemaBuilder::new()
    }
}

/// Builder for creating `NullSchema` instances.
pub struct NullSchemaBuilder {
    schema: NullSchema,
}

impl NullSchemaBuilder {
    /// Creates a new `NullSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: NullSchema {
                _context: DataSchemaContext::default(),
            },
        }
    }

    /// Builds and returns the `NullSchema` instance.
    pub fn build(self) -> NullSchema {
        self.schema
    }
}

impl ContextHelper for NullSchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for NullSchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

impl_builder_default!(
    ArraySchemaBuilder,
    BooleanSchemaBuilder,
    NumberSchemaBuilder,
    IntegerSchemaBuilder,
    ObjectSchemaBuilder,
    StringSchemaBuilder,
    NullSchemaBuilder,
);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DataSchema {
    Array(ArraySchema),
    Boolean(BooleanSchema),
    Number(NumberSchema),
    Integer(IntegerSchema),
    Object(ObjectSchema),
    String(StringSchema),
    Null(NullSchema),
}

/// Lightweight discriminator probe used to dispatch [`DataSchema`] variants
/// without materializing the full `serde_json::Value` tree.
#[derive(Deserialize)]
struct TypePeek {
    #[serde(rename = "type")]
    r#type: Option<String>,
}

impl<'de> Deserialize<'de> for DataSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Buffer the object once into a `Box<RawValue>` (a single byte-buffer
        // allocation) instead of a full `serde_json::Value` tree (one
        // allocation per node). This is possible now that every TD struct that
        // used to `#[serde(flatten)]` extension/composition fields buffers
        // through `serde_json::Map` instead of serde's `Content` layer, so the
        // `RawValue` survives the ancestor deserializer and is re-parsed by
        // serde_json's native (raw-capable) deserializer.
        let raw: Box<serde_json::value::RawValue> = Deserialize::deserialize(deserializer)?;
        let peek: TypePeek = serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
        match peek.r#type.as_deref() {
            Some("array") => serde_json::from_str::<ArraySchema>(raw.get()).map(Self::Array),
            Some("boolean") => serde_json::from_str::<BooleanSchema>(raw.get()).map(Self::Boolean),
            Some("number") => serde_json::from_str::<NumberSchema>(raw.get()).map(Self::Number),
            Some("integer") => serde_json::from_str::<IntegerSchema>(raw.get()).map(Self::Integer),
            Some("object") => serde_json::from_str::<ObjectSchema>(raw.get()).map(Self::Object),
            Some("string") => serde_json::from_str::<StringSchema>(raw.get()).map(Self::String),
            Some("null") => serde_json::from_str::<NullSchema>(raw.get()).map(Self::Null),
            // A DataSchema without a recognized `type` is a generic schema
            // (W3C TD permits omitting `type`). There is no dedicated
            // "untyped" variant, so deserialize it as the most permissive
            // canonical variant (`Object`) deterministically. The previous
            // `#[serde(untagged)]` first-match logic arbitrarily picked
            // `Array` (the first variant) for inputs like `{}`,
            // misclassifying generic schemas as arrays. Any type-specific
            // fields the input carries (e.g. `minLength`, `minimum`) are
            // preserved via the schema's extension map. An unrecognized
            // `type` string follows the same path defensively rather than
            // panicking.
            _ => serde_json::from_str::<ObjectSchema>(raw.get()).map(Self::Object),
        }
        .map_err(serde::de::Error::custom)
    }
}

impl From<ArraySchema> for DataSchema {
    fn from(schema: ArraySchema) -> Self {
        Self::Array(schema)
    }
}

impl From<ArraySchemaBuilder> for DataSchema {
    fn from(builder: ArraySchemaBuilder) -> Self {
        builder.build().into()
    }
}

impl From<BooleanSchema> for DataSchema {
    fn from(schema: BooleanSchema) -> Self {
        Self::Boolean(schema)
    }
}

impl From<BooleanSchemaBuilder> for DataSchema {
    fn from(builder: BooleanSchemaBuilder) -> Self {
        builder.build().into()
    }
}

impl From<ObjectSchema> for DataSchema {
    fn from(schema: ObjectSchema) -> Self {
        Self::Object(schema)
    }
}

impl From<ObjectSchemaBuilder> for DataSchema {
    fn from(builder: ObjectSchemaBuilder) -> Self {
        builder.build().into()
    }
}

impl From<StringSchema> for DataSchema {
    fn from(schema: StringSchema) -> Self {
        Self::String(schema)
    }
}

impl From<StringSchemaBuilder> for DataSchema {
    fn from(builder: StringSchemaBuilder) -> Self {
        builder.build().into()
    }
}

impl From<NullSchema> for DataSchema {
    fn from(schema: NullSchema) -> Self {
        Self::Null(schema)
    }
}

impl From<NullSchemaBuilder> for DataSchema {
    fn from(builder: NullSchemaBuilder) -> Self {
        builder.build().into()
    }
}

impl DataSchema {
    /// Creates an ArraySchema using the builder pattern.
    pub fn array() -> ArraySchemaBuilder {
        ArraySchema::builder()
    }

    /// Creates a BooleanSchema using the builder pattern.
    pub fn boolean() -> BooleanSchemaBuilder {
        BooleanSchema::builder()
    }

    /// Creates a NumberSchema using the builder pattern.
    pub fn number() -> NumberSchemaBuilder {
        NumberSchema::builder()
    }

    /// Creates an IntegerSchema using the builder pattern.
    pub fn integer() -> IntegerSchemaBuilder {
        IntegerSchema::builder()
    }

    /// Creates an ObjectSchema using the builder pattern.
    pub fn object() -> ObjectSchemaBuilder {
        ObjectSchema::builder()
    }

    /// Creates a StringSchema using the builder pattern.
    pub fn string() -> StringSchemaBuilder {
        StringSchema::builder()
    }

    /// Creates a NullSchema using the builder pattern.
    pub fn null() -> NullSchemaBuilder {
        NullSchema::builder()
    }

    /// Emits this schema's fields inline into an in-progress serialized map.
    /// Used by interaction affordances that previously `#[serde(flatten)]`-ed a
    /// `DataSchema`, so the schema fields appear at the affordance level.
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        match self {
            Self::Array(schema) => schema.serialize_into(map),
            Self::Boolean(schema) => schema.serialize_into(map),
            Self::Number(schema) => schema.serialize_into(map),
            Self::Integer(schema) => schema.serialize_into(map),
            Self::Object(schema) => schema.serialize_into(map),
            Self::String(schema) => schema.serialize_into(map),
            Self::Null(schema) => schema.serialize_into(map),
        }
    }

    pub(crate) fn context(&self) -> &DataSchemaContext {
        match self {
            Self::Array(schema) => &schema._context,
            Self::Boolean(schema) => &schema._context,
            Self::Number(schema) => &schema._context,
            Self::Integer(schema) => &schema._context,
            Self::Object(schema) => &schema._context,
            Self::String(schema) => &schema._context,
            Self::Null(schema) => &schema._context,
        }
    }

    fn expected_data_type(&self) -> &'static str {
        match self {
            Self::Array(_) => "array",
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            Self::Integer(_) => "integer",
            Self::Object(_) => "object",
            Self::String(_) => "string",
            Self::Null(_) => "null",
        }
    }
}

impl Validate for DataSchema {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        validate_schema_type_consistency(self)?;
        validate_schema_context(self.context(), level)?;

        match self {
            Self::Array(schema) => {
                validate_ordered("minItems", schema.min_items, "maxItems", schema.max_items)?;
                validate_nested_schemas(schema.items.as_deref(), level)?;
            }
            Self::Number(schema) => {
                validate_numeric_bounds(
                    schema.minimum,
                    schema.exclusive_minimum,
                    schema.maximum,
                    schema.exclusive_maximum,
                )?;
                validate_positive("multipleOf", schema.multiple_of)?;
            }
            Self::Integer(schema) => {
                validate_numeric_bounds(
                    schema.minimum,
                    schema.exclusive_minimum,
                    schema.maximum,
                    schema.exclusive_maximum,
                )?;
                validate_positive("multipleOf", schema.multiple_of)?;
            }
            Self::Object(schema) => {
                if let Some(properties) = &schema.properties {
                    for (name, schema) in properties {
                        schema.validate_with_level(level).map_err(|err| {
                            ValidateError::InvalidSchema(format_schema_path(
                                format_args!("properties.{}", name),
                                err,
                            ))
                        })?;
                    }
                }
            }
            Self::String(schema) => {
                validate_ordered(
                    "minLength",
                    schema.min_length,
                    "maxLength",
                    schema.max_length,
                )?;
            }
            Self::Boolean(_) | Self::Null(_) => {}
        }

        Ok(())
    }
}

fn validate_schema_type_consistency(schema: &DataSchema) -> Result<(), ValidateError> {
    let Some(data_type) = schema.context().data_type.as_deref() else {
        return Ok(());
    };

    let expected = schema.expected_data_type();
    if data_type == expected {
        return Ok(());
    }

    Err(ValidateError::InvalidSchema(format!(
        "type '{}' does not match {} schema",
        data_type, expected
    )))
}

fn validate_schema_context(
    context: &DataSchemaContext,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    validate_nested_schemas(context.one_of.as_deref(), level)?;

    // JSON Schema / TD 1.1: a schema MUST NOT be both readOnly and writeOnly.
    if context.read_only && context.write_only {
        return Err(ValidateError::InvalidSchema(String::from(
            "readOnly and writeOnly must not both be true",
        )));
    }

    let fields = &context._extra_fields;
    validate_ordered(
        "minItems",
        value_as_u64(fields.get("minItems")),
        "maxItems",
        value_as_u64(fields.get("maxItems")),
    )?;
    validate_ordered(
        "minLength",
        value_as_u64(fields.get("minLength")),
        "maxLength",
        value_as_u64(fields.get("maxLength")),
    )?;
    validate_json_number_bounds(fields)?;

    if let Some(multiple_of) = value_as_f64(fields.get("multipleOf")) {
        validate_positive("multipleOf", Some(multiple_of))?;
    }

    Ok(())
}

fn validate_nested_schemas(
    schemas: Option<&[DataSchema]>,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    if let Some(schemas) = schemas {
        for (index, schema) in schemas.iter().enumerate() {
            schema.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format_schema_path(format_args!("[{}]", index), err))
            })?;
        }
    }

    Ok(())
}

fn validate_ordered<T: PartialOrd>(
    min_name: &str,
    min: Option<T>,
    max_name: &str,
    max: Option<T>,
) -> Result<(), ValidateError> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => Err(ValidateError::InvalidSchema(format!(
            "{} must be less than or equal to {}",
            min_name, max_name
        ))),
        _ => Ok(()),
    }
}

fn validate_numeric_bounds<T: PartialOrd + Copy>(
    minimum: Option<T>,
    exclusive_minimum: Option<T>,
    maximum: Option<T>,
    exclusive_maximum: Option<T>,
) -> Result<(), ValidateError> {
    validate_ordered("minimum", minimum, "maximum", maximum)?;
    validate_ordered("minimum", minimum, "exclusiveMaximum", exclusive_maximum)?;
    validate_ordered("exclusiveMinimum", exclusive_minimum, "maximum", maximum)?;
    validate_ordered(
        "exclusiveMinimum",
        exclusive_minimum,
        "exclusiveMaximum",
        exclusive_maximum,
    )
}

fn validate_json_number_bounds(fields: &ExtensionMap) -> Result<(), ValidateError> {
    validate_numeric_bounds(
        value_as_f64(fields.get("minimum")),
        value_as_f64(fields.get("exclusiveMinimum")),
        value_as_f64(fields.get("maximum")),
        value_as_f64(fields.get("exclusiveMaximum")),
    )
}

fn validate_positive<T: PartialOrd + Default>(
    name: &str,
    value: Option<T>,
) -> Result<(), ValidateError> {
    match value {
        Some(value) if value <= T::default() => Err(ValidateError::InvalidSchema(format!(
            "{} must be greater than 0",
            name
        ))),
        _ => Ok(()),
    }
}

fn value_as_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(serde_json::Value::as_u64)
}

fn value_as_f64(value: Option<&serde_json::Value>) -> Option<f64> {
    value.and_then(serde_json::Value::as_f64)
}

fn format_schema_path(path: core::fmt::Arguments<'_>, err: ValidateError) -> String {
    match err {
        ValidateError::InvalidSchema(message) => alloc::format!("{}: {}", path, message),
        other => alloc::format!("{}: {}", path, other),
    }
}
