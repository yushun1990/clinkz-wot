use alloc::string::String;
use alloc::collections::BTreeMap;

use fluent_uri::Uri;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AnyUri(pub Uri<String>);

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

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct Nil;

/// A map of language tags to strings (e.g., {"en": "Light", "zh": "灯"})
///
/// Using BTreeMap instead of HashMap to ensure daterministic serialization order.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct MultiLanguage(BTreeMap<String, String>);

impl MultiLanguage {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn add(&mut self, lang: &str, text: &str) {
        self.0.insert(String::from(lang), String::from(text));
    }
}

/// Metadata of a Thing that provides version information about the TD document.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    /// Provides a version indicator of this TD.
    instance: String,
    /// Provides a version indicator of underlying TM.
    model: Option<String>
}
