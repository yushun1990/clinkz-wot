use alloc::{collections::BTreeMap, format, string::String, vec::Vec};

use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

use super::util::deserialize_bool_flexible;
use crate::data_type::{ExtensionMap, Metadata, MetadataHelper};
use crate::validate::{Validate, ValidateError, ValidationLevel};

#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSchemaContext {
    #[serde(flatten)]
    pub _metadata: Metadata,

    /// Provides a const value.
    #[serde(rename = "const")]
    pub constant: Option<serde_json::Value>,

    /// Supply a default value.
    pub default: Option<serde_json::Value>,

    /// Provides unit information that is used.
    pub unit: Option<String>,

    /// Used to ensure that the data is valid against one of the
    /// specified schemas in the array.
    pub one_of: Option<Vec<DataSchema>>,

    /// Restricted set of values provided as an array.
    #[serde(rename = "enum")]
    pub enumerate: Option<Vec<serde_json::Value>>,

    /// Indicate whether a property value is read only.
    #[serde(
        default,
        deserialize_with = "deserialize_bool_flexible",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub read_only: bool,

    /// Indicate whether a property value is write only.
    #[serde(
        default,
        deserialize_with = "deserialize_bool_flexible",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub write_only: bool,

    /// Allows validation based on a format pattern such as
    /// "date-time", "email", "uri", etc.
    pub format: Option<String>,

    /// Assignment of JSON based data types compatible with JSON schema.
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
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
        let mut items: Vec<DataSchema> = schemas.into_iter().map(Into::into).collect();
        self.context()
            .one_of
            .get_or_insert_with(Vec::new)
            .append(&mut items);
        self
    }

    /// Adds values to enum.
    fn enumerate<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = serde_json::Value>,
    {
        let mut items: Vec<serde_json::Value> = values.into_iter().collect();
        self.context()
            .enumerate
            .get_or_insert_with(Vec::new)
            .append(&mut items);
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
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArraySchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,

    /// Used to define the characteristic of an array.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub items: Option<Vec<DataSchema>>,

    /// Define the minimum number of items that have to be in the array.
    pub min_items: Option<u32>,

    /// Define the maximum number of items that have to be in the array.
    pub max_items: Option<u32>,
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
        let mut schemas: Vec<DataSchema> = items.into_iter().map(Into::into).collect();
        self.schema
            .items
            .get_or_insert_with(Vec::new)
            .append(&mut schemas);
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
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,
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

/// Metadata describing data of type number.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,

    /// Specifies a minimum numeric value, representing an inclusive
    /// lower limit.
    pub minimum: Option<f64>,

    /// Specifies a minimum numeric value, representing an exclusive
    /// lower limit.
    pub exclusive_minimum: Option<f64>,

    /// Specifies a maximum numeric value, representing an inclusive
    /// upper limit.
    pub maximum: Option<f64>,

    /// Specifies a maximum numeric value, representing an exclusive
    /// upper limit.
    pub exclusive_maximum: Option<f64>,

    /// Specifies the `multipleOf` value.
    ///
    /// Basic validation requires this value to be strictly greater than 0.
    pub multiple_of: Option<f64>,
}

impl NumberSchema {
    pub fn builder() -> NumberSchemaBuilder {
        NumberSchemaBuilder::new()
    }
}

/// Builder for creating `NumberSchema` instances.
pub struct NumberSchemaBuilder {
    schema: NumberSchema,
}

impl NumberSchemaBuilder {
    /// Creates a new `NumberSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: NumberSchema {
                _context: DataSchemaContext::default(),
                minimum: None,
                exclusive_minimum: None,
                maximum: None,
                exclusive_maximum: None,
                multiple_of: None,
            },
        }
    }

    /// Sets the minimum value.
    pub fn minimum(mut self, minimum: f64) -> Self {
        self.schema.minimum = Some(minimum);
        self
    }

    /// Sets the exclusive minimum value.
    pub fn exclusive_minimum(mut self, exclusive_minimum: f64) -> Self {
        self.schema.exclusive_minimum = Some(exclusive_minimum);
        self
    }

    /// Sets the maximum value.
    pub fn maximum(mut self, maximum: f64) -> Self {
        self.schema.maximum = Some(maximum);
        self
    }

    /// Sets the exclusive maximum value.
    pub fn exclusive_maximum(mut self, exclusive_maximum: f64) -> Self {
        self.schema.exclusive_maximum = Some(exclusive_maximum);
        self
    }

    /// Sets the `multipleOf` value.
    pub fn multiple_of(mut self, multiple_of: f64) -> Self {
        self.schema.multiple_of = Some(multiple_of);
        self
    }

    /// Builds and returns the `NumberSchema` instance.
    pub fn build(self) -> NumberSchema {
        self.schema
    }
}

impl ContextHelper for NumberSchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for NumberSchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type integer.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegerSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,

    /// Specifies a minimum numeric value, representing an inclusive
    /// lower limit.
    pub minimum: Option<i64>,

    /// Specifies a minimum numeric value, representing an exclusive
    /// lower limit.
    pub exclusive_minimum: Option<i64>,

    /// Specifies a maximum numeric value, representing an inclusive
    /// upper limit.
    pub maximum: Option<i64>,

    /// Specifies a maximum numeric value, representing an exclusive
    /// upper limit.
    pub exclusive_maximum: Option<i64>,

    /// Specifies the `multipleOf` value.
    ///
    /// Basic validation requires this value to be strictly greater than 0.
    pub multiple_of: Option<i64>,
}

impl IntegerSchema {
    pub fn builder() -> IntegerSchemaBuilder {
        IntegerSchemaBuilder::new()
    }
}

/// Builder for creating `IntegerSchema` instances.
pub struct IntegerSchemaBuilder {
    schema: IntegerSchema,
}

impl IntegerSchemaBuilder {
    /// Creates a new `IntegerSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: IntegerSchema {
                _context: DataSchemaContext::default(),
                minimum: None,
                exclusive_minimum: None,
                maximum: None,
                exclusive_maximum: None,
                multiple_of: None,
            },
        }
    }

    /// Sets the minimum value.
    pub fn minimum(mut self, minimum: i64) -> Self {
        self.schema.minimum = Some(minimum);
        self
    }

    /// Sets the exclusive minimum value.
    pub fn exclusive_minimum(mut self, exclusive_minimum: i64) -> Self {
        self.schema.exclusive_minimum = Some(exclusive_minimum);
        self
    }

    /// Sets the maximum value.
    pub fn maximum(mut self, maximum: i64) -> Self {
        self.schema.maximum = Some(maximum);
        self
    }

    /// Sets the exclusive maximum value.
    pub fn exclusive_maximum(mut self, exclusive_maximum: i64) -> Self {
        self.schema.exclusive_maximum = Some(exclusive_maximum);
        self
    }

    /// Sets the `multipleOf` value.
    pub fn multiple_of(mut self, multiple_of: i64) -> Self {
        self.schema.multiple_of = Some(multiple_of);
        self
    }

    /// Builds and returns the `IntegerSchema` instance.
    pub fn build(self) -> IntegerSchema {
        self.schema
    }
}

impl ContextHelper for IntegerSchemaBuilder {
    fn context(&mut self) -> &mut DataSchemaContext {
        &mut self.schema._context
    }
}

impl MetadataHelper for IntegerSchemaBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type Object.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,

    /// Data schema nested definitions.
    pub properties: Option<BTreeMap<String, DataSchema>>,

    /// Defines which members of the object type are mandatory.
    pub required: Option<Vec<String>>,
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
        let mut items: Vec<String> = fields.into_iter().map(|s| s.into()).collect();
        self.schema
            .required
            .get_or_insert_with(Vec::new)
            .append(&mut items);
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
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringSchema {
    #[serde(flatten)]
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
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NullSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,
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

impl<'de> Deserialize<'de> for DataSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        // Peek at "type" to decide whether a typed schema applies, without
        // cloning the entire value tree.
        let has_known_type = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|t| {
                matches!(
                    t,
                    "array" | "boolean" | "number" | "integer" | "object" | "string" | "null"
                )
            });

        if has_known_type {
            return deserialize_typed_data_schema(value).map_err(serde::de::Error::custom);
        }

        deserialize_untyped_data_schema(value).map_err(serde::de::Error::custom)
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

impl From<NumberSchema> for DataSchema {
    fn from(schema: NumberSchema) -> Self {
        Self::Number(schema)
    }
}

impl From<NumberSchemaBuilder> for DataSchema {
    fn from(builder: NumberSchemaBuilder) -> Self {
        builder.build().into()
    }
}

impl From<IntegerSchema> for DataSchema {
    fn from(schema: IntegerSchema) -> Self {
        Self::Integer(schema)
    }
}

impl From<IntegerSchemaBuilder> for DataSchema {
    fn from(builder: IntegerSchemaBuilder) -> Self {
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

    fn context(&self) -> &DataSchemaContext {
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
                validate_ordered_u32("minItems", schema.min_items, "maxItems", schema.max_items)?;
                validate_nested_schemas(schema.items.as_deref(), level)?;
            }
            Self::Number(schema) => {
                validate_number_bounds(
                    schema.minimum,
                    schema.exclusive_minimum,
                    schema.maximum,
                    schema.exclusive_maximum,
                )?;
                validate_positive_f64("multipleOf", schema.multiple_of)?;
            }
            Self::Integer(schema) => {
                validate_integer_bounds(
                    schema.minimum,
                    schema.exclusive_minimum,
                    schema.maximum,
                    schema.exclusive_maximum,
                )?;
                validate_positive_i64("multipleOf", schema.multiple_of)?;
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
                validate_ordered_u32(
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

fn deserialize_typed_data_schema(
    value: serde_json::Value,
) -> Result<DataSchema, serde_json::Error> {
    let data_type = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    match data_type {
        "array" => serde_json::from_value::<ArraySchema>(value).map(DataSchema::Array),
        "boolean" => serde_json::from_value::<BooleanSchema>(value).map(DataSchema::Boolean),
        "number" => serde_json::from_value::<NumberSchema>(value).map(DataSchema::Number),
        "integer" => serde_json::from_value::<IntegerSchema>(value).map(DataSchema::Integer),
        "object" => serde_json::from_value::<ObjectSchema>(value).map(DataSchema::Object),
        "string" => serde_json::from_value::<StringSchema>(value).map(DataSchema::String),
        "null" => serde_json::from_value::<NullSchema>(value).map(DataSchema::Null),
        // Defensive: the `has_known_type` pre-check in `deserialize()`
        // guarantees one of the known types above. Surface a deserialization
        // error instead of panicking if that invariant ever breaks (for
        // example if a new type name is introduced without updating this
        // match).
        other => Err(serde::de::Error::custom(format!(
            "unknown data schema type '{other}'"
        ))),
    }
}

fn deserialize_untyped_data_schema(
    value: serde_json::Value,
) -> Result<DataSchema, serde_json::Error> {
    // A DataSchema without a recognized `type` is a generic schema (W3C TD
    // permits omitting `type`). There is no dedicated "untyped" variant, so
    // deserialize it as the most permissive canonical variant (`Object`)
    // deterministically. The previous `#[serde(untagged)]` first-match logic
    // arbitrarily picked `Array` (the first variant) for inputs like `{}`,
    // misclassifying generic schemas as arrays (and letting array-only
    // constraints apply). Any type-specific fields the input carries (e.g.
    // `minLength`, `minimum`) are preserved via the schema's extension map.
    serde_json::from_value::<ObjectSchema>(value).map(DataSchema::Object)
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
    validate_ordered_u64(
        "minItems",
        value_as_u64(fields.get("minItems")),
        "maxItems",
        value_as_u64(fields.get("maxItems")),
    )?;
    validate_ordered_u64(
        "minLength",
        value_as_u64(fields.get("minLength")),
        "maxLength",
        value_as_u64(fields.get("maxLength")),
    )?;
    validate_json_number_bounds(fields)?;

    if let Some(multiple_of) = value_as_f64(fields.get("multipleOf")) {
        validate_positive_f64("multipleOf", Some(multiple_of))?;
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

fn validate_ordered_u32(
    min_name: &str,
    min: Option<u32>,
    max_name: &str,
    max: Option<u32>,
) -> Result<(), ValidateError> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => Err(ValidateError::InvalidSchema(format!(
            "{} must be less than or equal to {}",
            min_name, max_name
        ))),
        _ => Ok(()),
    }
}

fn validate_ordered_u64(
    min_name: &str,
    min: Option<u64>,
    max_name: &str,
    max: Option<u64>,
) -> Result<(), ValidateError> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => Err(ValidateError::InvalidSchema(format!(
            "{} must be less than or equal to {}",
            min_name, max_name
        ))),
        _ => Ok(()),
    }
}

fn validate_number_bounds(
    minimum: Option<f64>,
    exclusive_minimum: Option<f64>,
    maximum: Option<f64>,
    exclusive_maximum: Option<f64>,
) -> Result<(), ValidateError> {
    validate_ordered_f64("minimum", minimum, "maximum", maximum)?;
    validate_ordered_f64("minimum", minimum, "exclusiveMaximum", exclusive_maximum)?;
    validate_ordered_f64("exclusiveMinimum", exclusive_minimum, "maximum", maximum)?;
    validate_ordered_f64(
        "exclusiveMinimum",
        exclusive_minimum,
        "exclusiveMaximum",
        exclusive_maximum,
    )
}

fn validate_json_number_bounds(fields: &ExtensionMap) -> Result<(), ValidateError> {
    validate_number_bounds(
        value_as_f64(fields.get("minimum")),
        value_as_f64(fields.get("exclusiveMinimum")),
        value_as_f64(fields.get("maximum")),
        value_as_f64(fields.get("exclusiveMaximum")),
    )
}

fn validate_ordered_f64(
    min_name: &str,
    min: Option<f64>,
    max_name: &str,
    max: Option<f64>,
) -> Result<(), ValidateError> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => Err(ValidateError::InvalidSchema(format!(
            "{} must be less than or equal to {}",
            min_name, max_name
        ))),
        _ => Ok(()),
    }
}

fn validate_integer_bounds(
    minimum: Option<i64>,
    exclusive_minimum: Option<i64>,
    maximum: Option<i64>,
    exclusive_maximum: Option<i64>,
) -> Result<(), ValidateError> {
    validate_ordered_i64("minimum", minimum, "maximum", maximum)?;
    validate_ordered_i64("minimum", minimum, "exclusiveMaximum", exclusive_maximum)?;
    validate_ordered_i64("exclusiveMinimum", exclusive_minimum, "maximum", maximum)?;
    validate_ordered_i64(
        "exclusiveMinimum",
        exclusive_minimum,
        "exclusiveMaximum",
        exclusive_maximum,
    )
}

fn validate_ordered_i64(
    min_name: &str,
    min: Option<i64>,
    max_name: &str,
    max: Option<i64>,
) -> Result<(), ValidateError> {
    match (min, max) {
        (Some(min), Some(max)) if min > max => Err(ValidateError::InvalidSchema(format!(
            "{} must be less than or equal to {}",
            min_name, max_name
        ))),
        _ => Ok(()),
    }
}

fn validate_positive_f64(name: &str, value: Option<f64>) -> Result<(), ValidateError> {
    match value {
        Some(value) if value <= 0.0 => Err(ValidateError::InvalidSchema(format!(
            "{} must be greater than 0",
            name
        ))),
        _ => Ok(()),
    }
}

fn validate_positive_i64(name: &str, value: Option<i64>) -> Result<(), ValidateError> {
    match value {
        Some(value) if value <= 0 => Err(ValidateError::InvalidSchema(format!(
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
