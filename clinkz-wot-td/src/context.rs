use alloc::vec::Vec;
use alloc::string::String;

use fluent_uri::Uri;
use serde::{Deserialize, Serialize};

pub const WOT_CONTEXT_1_0: &str = "https://www.w3c.org/2019/wot/td/v1";
pub const WOT_CONTEXT_1_1: &str = "https://www.w3c.org/2022/wot/td/v1.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AnyUri(Uri<String>);

impl AnyUri {
    /// Creates an AnyUri from a static string, panicking on invalid input.
    /// Internal use only for known-good constants.
    pub(crate) fn from_static(s: &'static str) -> Self {
        Self(Uri::parse(s).expect("Invalid static URI").to_owned())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn parse(s: &str) -> Result<Self, fluent_uri::ParseError> {
        Ok(Self(Uri::parse(s)?.to_owned()))
    }
}

impl PartialEq<str> for AnyUri {
    fn eq(&self, other: &str) -> bool {
        self.0.as_str() == other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextEntry {
    Uri(AnyUri),
    Object(serde_json::Map<String, serde_json::Value>)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    entries: Vec<ContextEntry>
}

impl Context {
    /// Create a standard WoT 1.1 Context.
    /// By default, it contains only the 1.1 URI.
    pub fn new() -> Self {
        let uri = Uri::parse(WOT_CONTEXT_1_1).unwrap().to_owned();
        Self {
            entries: alloc::vec![ContextEntry::Uri(AnyUri(uri))]
        }
    }

    /// Enable compability with WoT 1.0 consumers.
    /// According to the spec, 1.0 URI MUST be the first entry,
    /// and 1.1 URI MUST be the second entry in this case.
    pub fn with_1_0_compatibility(mut self) -> Self {
        let already_v1_first = self.entries.first().map_or(
            false, |e| {
                matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_0)
            }
        );

        if !already_v1_first {
            self.entries.retain(|e| {
                !matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_1)
            });

            let uri_v1 = AnyUri::from_static(WOT_CONTEXT_1_0);
            let uri_v11 = AnyUri::from_static(WOT_CONTEXT_1_1);

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

impl Serialize for Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            use serde::ser::Error;

            // Validation: Enusure at least one WoT standard URI is present
            let has_v1 = self.entries.iter().any(
                |e| matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_0));
            let has_v11 = self.entries.iter().any(
                |e| matches!(e, ContextEntry::Uri(u) if u == WOT_CONTEXT_1_1));

            if !has_v1 && !has_v11 {
                return Err(S::Error::custom("Context must contain at least on official WoT URI"));
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
        D: serde::Deserializer<'de> {
            use serde::de::Error;

            // Internal helper to catch both formats in JSON-LD
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum RawContext {
                Single(ContextEntry),
                Multi(Vec<ContextEntry>)
            }

            let raw = RawContext::deserialize(deserializer)?;
            let entries = match raw {
                RawContext::Single(entry) => alloc::vec![entry],
                RawContext::Multi(vec) => vec
            };

            // Semantic Validation: Check if entries are empty.
            if entries.is_empty() {
                return Err(D::Error::custom("@context cannot be empty."));
            }

            Ok(Self { entries })
    }
}
