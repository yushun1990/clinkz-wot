use alloc::{vec::Vec, string::String, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use super::util::deserialize_bool_flexible;
use crate::data_type::{Metadata, MetadataHelper, DefaultExt};

#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct DataSchemaContext<Ext=DefaultExt> {
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
    pub _extra_fields: Ext,
}

pub trait ContextHelper: MetadataHelper {
    type Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext>;

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
        I: IntoIterator<Item=DataSchema>
    {
        let mut items: Vec<DataSchema> = schemas.into_iter().collect();
        self.context().one_of.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Adds values to enum.
    fn enumerate<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item=serde_json::Value>
    {
        let mut items: Vec<serde_json::Value> = values.into_iter().collect();
        self.context().enumerate.get_or_insert_with(Vec::new).append(&mut items);
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

}


/// Metadata describing data of type array.
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ArraySchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>,

    /// Used to define the characteristic of an array.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub items: Option<Vec<DataSchema>>,

    /// Define the minimum number of items that have to be in the array.
    pub min_items: Option<u32>,

    /// Define the maximum number of items that have to be in the array.
    pub max_items: Option<u32>,
}

impl<Ext> ArraySchema<Ext>
where
    Ext: Default
{
    pub fn builder() -> ArraySchemaBuilder<Ext> {
        ArraySchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `ArraySchema` instances.
pub struct ArraySchemaBuilder<Ext> {
    schema: ArraySchema<Ext>,
}

impl <Ext> ArraySchemaBuilder <Ext>
where
    Ext: Default
{
    /// Creates a new `ArraySchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: ArraySchema {
                _context: DataSchemaContext::<Ext>::default(),
                items: None,
                min_items: None,
                max_items: None,
            },
        }
    }

    /// Adds items schemas.
    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item=DataSchema> {
        let mut schemas: Vec<DataSchema> = items.into_iter().collect();
        self.schema.items.get_or_insert_with(Vec::new).append(&mut schemas);
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
    pub fn build(self) -> ArraySchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for ArraySchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for ArraySchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type boolean.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BooleanSchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>,
}

impl <Ext> BooleanSchema<Ext>
where
    Ext: Default
{
    pub fn builder() -> BooleanSchemaBuilder<Ext> {
        BooleanSchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `BooleanSchema` instances.
pub struct BooleanSchemaBuilder<Ext> {
    schema: BooleanSchema<Ext>,
}

impl <Ext> BooleanSchemaBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `BooleanSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: BooleanSchema {
                _context: DataSchemaContext::<Ext>::default(),
            },
        }
    }

    /// Builds and returns the `BooleanSchema` instance.
    pub fn build(self) -> BooleanSchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for BooleanSchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for BooleanSchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type number.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NumberSchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>,

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

    /// Specifies the multipleOf value number. The value must strictly
    /// greater than 0.
    pub multiple_of: Option<f64>,
}

impl <Ext> NumberSchema<Ext>
where
    Ext: Default
{
    pub fn builder() -> NumberSchemaBuilder<Ext> {
        NumberSchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `NumberSchema` instances.
pub struct NumberSchemaBuilder<Ext> {
    schema: NumberSchema<Ext>,
}

impl <Ext> NumberSchemaBuilder<Ext>
where
    Ext: Default {
    /// Creates a new `NumberSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: NumberSchema {
                _context: DataSchemaContext::<Ext>::default(),
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

    /// Sets the multiple of value.
    pub fn multiple_of(mut self, multiple_of: f64) -> Self {
        if multiple_of > 0.0 {
            self.schema.multiple_of = Some(multiple_of);
        }
        self
    }

    /// Builds and returns the `NumberSchema` instance.
    pub fn build(self) -> NumberSchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for NumberSchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for NumberSchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type integer.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct IntegerSchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>,

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

    /// Specifies the multipleOf value number. The value must strictly
    /// greater than 0.
    pub multiple_of: Option<i64>,
}

impl <Ext> IntegerSchema<Ext>
where
    Ext: Default
{
    pub fn builder() -> IntegerSchemaBuilder<Ext> {
        IntegerSchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `IntegerSchema` instances.
pub struct IntegerSchemaBuilder<Ext> {
    schema: IntegerSchema<Ext>,
}

impl <Ext> IntegerSchemaBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `IntegerSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: IntegerSchema {
                _context: DataSchemaContext::<Ext>::default(),
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

    /// Sets the multiple of value.
    pub fn multiple_of(mut self, multiple_of: i64) -> Self {
        if multiple_of > 0 {
            self.schema.multiple_of = Some(multiple_of);
        }
        self
    }

    /// Builds and returns the `IntegerSchema` instance.
    pub fn build(self) -> IntegerSchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for IntegerSchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for IntegerSchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type Object.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ObjectSchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>,

    /// Data schema nested definitions.
    pub properties: Option<BTreeMap<String, DataSchema>>,

    /// Defines which members of the object type are mandatory.
    pub required: Option<Vec<String>>,
}

impl <Ext> ObjectSchema<Ext>
where
    Ext: Default
{
    pub fn builder() -> ObjectSchemaBuilder<Ext> {
        ObjectSchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `ObjectSchema` instances.
pub struct ObjectSchemaBuilder<Ext> {
    schema: ObjectSchema<Ext>,
}

impl <Ext> ObjectSchemaBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `ObjectSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: ObjectSchema {
                _context: DataSchemaContext::<Ext>::default(),
                properties: None,
                required: None,
            },
        }
    }

    /// Adds a property.
    pub fn property(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let properties = self.schema.properties.get_or_insert_with(BTreeMap::new);
        properties.insert(name.into(), schema);
        self
    }

    /// Adds multiple properties.
    pub fn properties<I, S>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item=(S, DataSchema)>,
        S: Into<String> {
        let map = self.schema.properties.get_or_insert_with(BTreeMap::new);
        for (name, schema) in properties {
            map.insert(name.into(), schema);
        }
        self
    }

    /// Adds required fields.
    pub fn required<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String> {
        let mut items: Vec<String> = fields.into_iter().map(|s| s.into()).collect();
        self.schema.required.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Builds and returns the `ObjectSchema` instance.
    pub fn build(self) -> ObjectSchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for ObjectSchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for ObjectSchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}

/// Metadata describing data of type string.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct StringSchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>,

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

impl <Ext> StringSchema<Ext>
where
    Ext: Default {
    pub fn builder() -> StringSchemaBuilder<Ext> {
        StringSchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `StringSchema` instances.
pub struct StringSchemaBuilder<Ext> {
    schema: StringSchema<Ext>,
}

impl <Ext> StringSchemaBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `StringSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: StringSchema {
                _context: DataSchemaContext::<Ext>::default(),
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
    pub fn build(self) -> StringSchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for StringSchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for StringSchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}


/// Metadata describing data of type string.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NullSchema<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: DataSchemaContext<Ext>
}

impl <Ext> NullSchema<Ext>
where
    Ext: Default
{
    pub fn builder() -> NullSchemaBuilder<Ext> {
        NullSchemaBuilder::<Ext>::new()
    }
}

/// Builder for creating `NullSchema` instances.
pub struct NullSchemaBuilder<Ext> {
    schema: NullSchema<Ext>,
}

impl <Ext> NullSchemaBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `NullSchemaBuilder`.
    pub fn new() -> Self {
        Self {
            schema: NullSchema {
                _context: DataSchemaContext::default(),
            },
        }
    }

    /// Builds and returns the `NullSchema` instance.
    pub fn build(self) -> NullSchema<Ext> {
        self.schema
    }
}

impl <Ext> ContextHelper for NullSchemaBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut DataSchemaContext<Self::Ext> {
        &mut self.schema._context
    }
}

impl <Ext> MetadataHelper for NullSchemaBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.context()._metadata
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataSchema<Ext=DefaultExt> {
    Array(ArraySchema<Ext>),
    Boolean(BooleanSchema<Ext>),
    Number(NumberSchema<Ext>),
    Integer(IntegerSchema<Ext>),
    Object(ObjectSchema<Ext>),
    String(StringSchema<Ext>),
    Null(NullSchema<Ext>)
}

impl <Ext> DataSchema<Ext>
where
    Ext: Default + Clone
{
    /// Creates an ArraySchema using the builder pattern.
    pub fn array() -> ArraySchemaBuilder<Ext> {
        ArraySchema::<Ext>::builder()
    }

    /// Creates a BooleanSchema using the builder pattern.
    pub fn boolean() -> BooleanSchemaBuilder<Ext> {
        BooleanSchema::<Ext>::builder()
    }

    /// Creates a NumberSchema using the builder pattern.
    pub fn number() -> NumberSchemaBuilder<Ext> {
        NumberSchema::<Ext>::builder()
    }

    /// Creates an IntegerSchema using the builder pattern.
    pub fn integer() -> IntegerSchemaBuilder<Ext> {
        IntegerSchema::<Ext>::builder()
    }

    /// Creates an ObjectSchema using the builder pattern.
    pub fn object() -> ObjectSchemaBuilder<Ext> {
        ObjectSchema::<Ext>::builder()
    }

    /// Creates a StringSchema using the builder pattern.
    pub fn string() -> StringSchemaBuilder<Ext> {
        StringSchema::<Ext>::builder()
    }

    /// Creates a NullSchema using the builder pattern.
    pub fn null() -> NullSchemaBuilder<Ext> {
        NullSchema::<Ext>::builder()
    }
}
