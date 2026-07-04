//! Communication metadata describing expected response messages
//! (`ExpectedResponse`, `AdditionalExpectedResponse`).

use alloc::string::String;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{components_util::deserialize_bool_flexible, data_type::ExtensionMap};

/// Communication metadata describing the expected response message for the
/// primary response.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedResponse {
    /// Media type of the response payload (e.g., "application/json").
    pub content_type: String,

    pub _extra_fields: ExtensionMap,
}

impl<'de> serde::Deserialize<'de> for ExpectedResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let content_type = crate::flat::take_required(&mut map, "contentType")?;
        Ok(ExpectedResponse {
            content_type,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl serde::Serialize for ExpectedResponse {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("contentType", &self.content_type)?;
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl From<String> for ExpectedResponse {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl ExpectedResponse {
    pub fn new(value: String) -> Self {
        Self {
            content_type: value,
            _extra_fields: Default::default(),
        }
    }

    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self._extra_fields.extend(extra_fields.into());
        self
    }

    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self._extra_fields.insert(key.into(), value);
        self
    }
}

/// Communication metadata describing the expected response message for
/// additional responses.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AdditionalExpectedResponse {
    /// Mandatory, default to value of the contentType of the Form element it belongs to.
    pub content_type: Option<String>,

    /// Used to define the output data schema for an additional response
    /// if it differs from the default output data schema.
    /// Rather than a DataSchema object, the name of a previous definition
    /// given in a schemaDefinitions map must be used.
    pub schema: Option<String>,

    /// Indicates if this response is for an error case.
    pub success: bool,

    pub _extra_fields: ExtensionMap,
}

/// Deserialize adapter carrying the flexible-bool decoder used by `success`.
#[derive(Deserialize)]
struct FlexBoolField(#[serde(deserialize_with = "deserialize_bool_flexible")] bool);

impl<'de> Deserialize<'de> for AdditionalExpectedResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let content_type = crate::flat::take(&mut map, "contentType")?;
        let schema = crate::flat::take(&mut map, "schema")?;
        let success = match crate::flat::take::<FlexBoolField, D::Error>(&mut map, "success")? {
            Some(field) => field.0,
            None => false,
        };
        Ok(AdditionalExpectedResponse {
            content_type,
            schema,
            success,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for AdditionalExpectedResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if let Some(content_type) = &self.content_type {
            map.serialize_entry("contentType", content_type)?;
        }
        if let Some(schema) = &self.schema {
            map.serialize_entry("schema", schema)?;
        }
        if self.success {
            map.serialize_entry("success", &self.success)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl AdditionalExpectedResponse {
    pub fn new(content_type: String) -> Self {
        Self {
            content_type: Some(content_type),
            ..Default::default()
        }
    }

    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self._extra_fields.extend(extra_fields.into());
        self
    }

    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self._extra_fields.insert(key.into(), value);
        self
    }
}
