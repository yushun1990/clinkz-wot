use alloc::{vec::Vec, string::String, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::{data_type::Metadata, components_util::deserialize_bool_flexible};

#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct DataSchemaContext {
    #[serde(flatten)]
    pub _metabase: Metadata,

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
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub read_only: bool,

    /// Indicate whether a property value is write only.
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub write_only: bool,

    /// Allows validation based on a format pattern such as
    /// "date-time", "email", "uri", etc.
    pub format: Option<String>,

    /// Assignment of JSON based data types compatible with JSON schema.
    #[serde(rename = "type")]
    pub data_type: Option<String>
}

/// Metadata describing data of type array.
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
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

/// Metadata describing data of type boolean.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BooleanSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,
}

/// Metadata describing data of type number.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
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

    /// Specifies the multipleOf value number. The value must strictly
    /// greater than 0.
    pub multiple_of: Option<f64>,
}

/// Metadata describing data of type integer.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
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

    /// Specifies the multipleOf value number. The value must strictly
    /// greater than 0.
    pub multiple_of: Option<i64>,
}

/// Metadata describing data of type Object.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ObjectSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext,

    /// Data schema nested definitions.
    pub properties: Option<BTreeMap<String, DataSchema>>,

    /// Defines which members of the object type are mandatory.
    pub required: Option<Vec<String>>,
}

/// Metadata describing data of type string.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
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


/// Metadata describing data of type string.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NullSchema {
    #[serde(flatten)]
    pub _context: DataSchemaContext
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DataSchema {
    Array(ArraySchema),
    Boolean(BooleanSchema),
    Number(NumberSchema),
    Integer(IntegerSchema),
    Object(ObjectSchema),
    String(StringSchema),
    Null(NullSchema)
}
