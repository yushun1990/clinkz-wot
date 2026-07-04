//! TD and TM version metadata (`VersionInfo`, `ThingModelVersionInfo`).

use alloc::string::String;

use crate::data_type::ExtensionMap;

/// Metadata of a Thing that provides version information about the TD document.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct VersionInfo {
    /// Provides a version indicator of this TD.
    pub instance: String,
    /// Provides a version indicator of underlying TM.
    pub model: Option<String>,

    pub _extra_fields: ExtensionMap,
}

impl VersionInfo {
    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self._extra_fields.insert(key.into(), value);
        self
    }
}

impl<'de> serde::Deserialize<'de> for VersionInfo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let instance = crate::flat::take_required(&mut map, "instance")?;
        let model = crate::flat::take(&mut map, "model")?;
        Ok(VersionInfo {
            instance,
            model,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl serde::Serialize for VersionInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("instance", &self.instance)?;
        if let Some(model) = &self.model {
            map.serialize_entry("model", model)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// Thing Model version metadata.
///
/// Thing Model versioning uses the `model` term and must not include an
/// `instance` term.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ThingModelVersionInfo {
    /// Provides a version indicator of the underlying Thing Model.
    pub model: Option<String>,

    pub _extra_fields: ExtensionMap,
}

impl ThingModelVersionInfo {
    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self._extra_fields.insert(key.into(), value);
        self
    }
}

impl<'de> serde::Deserialize<'de> for ThingModelVersionInfo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let model = crate::flat::take(&mut map, "model")?;
        Ok(ThingModelVersionInfo {
            model,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl serde::Serialize for ThingModelVersionInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if let Some(model) = &self.model {
            map.serialize_entry("model", model)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}
