use alloc::{collections::BTreeMap, format, string::String, vec::Vec};

use serde::{Deserialize, Serialize};

use crate::{data_type::AbsoluteUri, validate::ValidateError};

pub const WOT_CONTEXT_1_0: &str = "https://www.w3.org/2019/wot/td/v1";
pub const WOT_CONTEXT_1_1: &str = "https://www.w3.org/2022/wot/td/v1.1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextEntry {
    Uri(AbsoluteUri),
    Object(BTreeMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    entries: Vec<ContextEntry>,
}

impl Context {
    /// Create a standard WoT 1.1 Context.
    /// By default, it contains only the 1.1 URI.
    pub fn new() -> Self {
        Self {
            entries: alloc::vec![ContextEntry::Uri(AbsoluteUri::from_static(WOT_CONTEXT_1_1))],
        }
    }

    /// Enable compability with WoT 1.0 consumers.
    /// According to the spec, 1.0 URI MUST be the first entry,
    /// and 1.1 URI MUST be the second entry in this case.
    pub fn with_1_0_compatibility(mut self) -> Self {
        let already_v1_first = self.entries.first().map_or(
            false,
            |e| matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_0),
        );

        if !already_v1_first {
            self.entries
                .retain(|e| !matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_1));

            let uri_v1 = AbsoluteUri::from_static(WOT_CONTEXT_1_0);
            let uri_v11 = AbsoluteUri::from_static(WOT_CONTEXT_1_1);

            self.entries.insert(0, ContextEntry::Uri(uri_v11));
            self.entries.insert(0, ContextEntry::Uri(uri_v1));
        }

        self
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Creates a builder for `Context`.
    pub fn builder() -> ContextBuilder {
        ContextBuilder::new()
    }
}

/// Builder for creating `Context` instances.
pub struct ContextBuilder {
    context: Context,
    errors: Vec<ValidateError>,
}

impl ContextBuilder {
    /// Creates a new `ContextBuilder` with the default WoT 1.1 context.
    pub fn new() -> Self {
        Self {
            context: Context::new(),
            errors: Vec::new(),
        }
    }

    /// Adds a URI to the context.
    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        let uri = uri.into();
        match AbsoluteUri::parse(uri.as_str()) {
            Ok(uri) => self.context.entries.push(ContextEntry::Uri(uri)),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("@context: {}", uri))),
        }
        self
    }

    /// Adds an object to the context.
    pub fn object(mut self, object: BTreeMap<String, serde_json::Value>) -> Self {
        self.context.entries.push(ContextEntry::Object(object));
        self
    }

    /// Adds a key-value pair to the context as an object entry.
    pub fn pair(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let mut object = BTreeMap::new();
        object.insert(key.into(), value);
        self.context.entries.push(ContextEntry::Object(object));
        self
    }

    /// Enables compatibility with WoT 1.0 consumers.
    pub fn with_1_0_compatibility(mut self) -> Self {
        self.context = self.context.with_1_0_compatibility();
        self
    }

    /// Builds and returns the `Context` instance.
    pub fn build(self) -> Result<Context, ValidateError> {
        if let Some(error) = self.errors.into_iter().next() {
            return Err(error);
        }
        Ok(self.context)
    }
}

impl Serialize for Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;

        // Validation: Enusure at least one WoT standard URI is present
        let has_v1 = self
            .entries
            .iter()
            .any(|e| matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_0));
        let has_v11 = self
            .entries
            .iter()
            .any(|e| matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_1));

        if !has_v1 && !has_v11 {
            return Err(S::Error::custom(
                "Context must contain at least on official WoT URI",
            ));
        }

        // Logic: Serialize as a single string if only one entry exists.
        if self.entries.len() == 1 {
            return self.entries[0].serialize(serializer);
        }

        self.entries.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        // Internal helper to catch both formats in JSON-LD
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawContext {
            Single(ContextEntry),
            Multi(Vec<ContextEntry>),
        }

        let raw = RawContext::deserialize(deserializer)?;
        let entries = match raw {
            RawContext::Single(entry) => alloc::vec![entry],
            RawContext::Multi(vec) => vec,
        };

        // Semantic Validation: Check if entries are empty.
        if entries.is_empty() {
            return Err(D::Error::custom("@context cannot be empty."));
        }

        Ok(Self { entries })
    }
}
