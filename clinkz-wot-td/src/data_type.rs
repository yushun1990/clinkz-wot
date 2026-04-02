use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;

use fluent_uri::Uri;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};
use crate::util::deserialize_bool_flexible;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct Nil;

/// A map of language tags to strings (e.g., {"en": "Light", "zh": "灯"})
///
/// Using BTreeMap instead of HashMap to ensure deterministic serialization order.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
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
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct VersionInfo {
    /// Provides a version indicator of this TD.
    instance: String,
    /// Provides a version indicator of underlying TM.
    model: Option<String>
}

/// Operation types of form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    ReadProperty,
    WriteProperty,
    ObserveProperty,
    UnobserveProperty,
    InvokeAction,
    QueryAction,
    CancelAction,
    SubscribeEvent,
    UnsubscribeEvent,
    ReadAllProperties,
    WriteAllProperties,
    ObserveAllProperties,
    UnobserveAllProperties,
    SubscribeAllEvents,
    UnsubscribeAllEvents,
}

/// Communication metadata describing the expected response message for the
/// primary response.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedResponse {
    /// Media type of the response payload (e.g., "application/json").
    pub content_type: String,
}

/// Communication metadata describing the expected response message for
/// additional responses.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalExpectedResponse {
    /// Flatten the core fields into this struct.
    #[serde(flatten)]
    pub _expected_response: ExpectedResponse,

    /// For HTTP, this might be "Content-Range".
    pub schema: Option<String>,

    /// Indicates if this response is for an error case.
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub success: bool,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// JSON-LD keyword to label the object with semantic tags.
    #[serde(rename = "@type")]
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub tags: Option<Vec<String>>,

    /// Provides a human-readable title.
    pub title: Option<String>,

    /// Provides multi-language human-readable titles.
    pub titles: Option<MultiLanguage>,

    /// Provides additional (human-readable) information based on a
    /// default language.
    pub description: Option<String>,

    /// Multi-language descriptions.
    pub descriptions: Option<MultiLanguage>,
}
