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

    /// Returns `true` when the context contains at least one standard WoT
    /// context URI (TD 1.0 or TD 1.1).
    ///
    /// This check mirrors the serialization-time validation in
    /// [`Serialize for Context`](Self#impl-Serialize-for-Context) but is
    /// available for explicit validation at
    /// [`ValidationLevel::Profile`](crate::validate::ValidationLevel::Profile)
    /// or stricter levels.
    pub fn has_wot_context(&self) -> bool {
        self.entries.iter().any(|entry| match entry {
            ContextEntry::Uri(uri) => {
                uri.as_str() == WOT_CONTEXT_1_0 || uri.as_str() == WOT_CONTEXT_1_1
            }
            ContextEntry::Object(_) => false,
        })
    }

    /// Returns `true` when the first `@context` entry is a standard WoT context
    /// URI (TD 1.0 or TD 1.1).
    ///
    /// TD 1.1 / JSON-LD require the standard TD context URI to be the first
    /// value of `@context` so that consumers (and JSON-LD processors) resolve
    /// the WoT vocabulary before any extension namespace. Extension-only first
    /// entries or object entries first violate this rule.
    pub fn is_wot_context_first(&self) -> bool {
        self.entries.first().is_some_and(|entry| match entry {
            ContextEntry::Uri(uri) => {
                uri.as_str() == WOT_CONTEXT_1_0 || uri.as_str() == WOT_CONTEXT_1_1
            }
            ContextEntry::Object(_) => false,
        })
    }

    /// Returns the standard WoT TD version URI present in this context, if any.
    ///
    /// Prefers TD 1.1 when both 1.0 and 1.1 are declared.
    pub fn wot_version_uri(&self) -> Option<&'static str> {
        let mut has_v10 = false;
        let mut has_v11 = false;
        for entry in &self.entries {
            if let ContextEntry::Uri(u) = entry {
                if u.as_str() == WOT_CONTEXT_1_1 {
                    has_v11 = true;
                } else if u.as_str() == WOT_CONTEXT_1_0 {
                    has_v10 = true;
                }
            }
        }
        if has_v11 {
            Some(WOT_CONTEXT_1_1)
        } else if has_v10 {
            Some(WOT_CONTEXT_1_0)
        } else {
            None
        }
    }

    /// Enable compability with WoT 1.0 consumers.
    /// According to the spec, 1.0 URI MUST be the first entry,
    /// and 1.1 URI MUST be the second entry in this case.
    pub fn with_1_0_compatibility(mut self) -> Self {
        let already_v1_first = self
            .entries
            .first()
            .is_some_and(|e| matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_0));

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
        crate::validate::collected_errors(self.errors)?;
        Ok(self.context)
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;

        // Single-pass guard: ensure at least one official WoT URI is present.
        let mut has_v1 = false;
        let mut has_v11 = false;
        for entry in &self.entries {
            if let ContextEntry::Uri(u) = entry {
                if u.as_str() == WOT_CONTEXT_1_0 {
                    has_v1 = true;
                } else if u.as_str() == WOT_CONTEXT_1_1 {
                    has_v11 = true;
                }
            }
        }

        if !has_v1 && !has_v11 {
            return Err(S::Error::custom(
                "Context must contain at least one official WoT URI",
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
