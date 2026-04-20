use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;

use fluent_uri::Uri;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};
use crate::{components_util::deserialize_bool_flexible, validate::Validate};


#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
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

impl Validate for Nil {
    fn validate(&self) -> Result<(), crate::validate::ValidateError> {
        Ok(())
    }
}

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

    /// Adds a language-text pair and returns self for method chaining.
    pub fn with(mut self, lang: &str, text: &str) -> Self {
        self.add(lang, text);
        self
    }

    /// Creates a MultiLanguage from a BTreeMap.
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        Self(map)
    }

    /// Creates a MultiLanguage from an iterator of (lang, text) pairs.
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item=(String, String)> {
        Self(BTreeMap::from_iter(iter))
    }

    /// Checks if a language is present.
    pub fn contains(&self, lang: &str) -> bool {
        self.0.contains_key(lang)
    }

    /// Gets the text for a specific language, or None if not present.
    pub fn get(&self, lang: &str) -> Option<&String> {
        self.0.get(lang)
    }

    /// Merges another MultiLanguage into this one.
    pub fn merge(&mut self, other: &MultiLanguage) {
        self.0.extend(other.0.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    /// Returns a reference to the underlying BTreeMap.
    pub fn as_map(&self) -> &BTreeMap<String, String> {
        &self.0
    }

    /// Converts self into the underlying BTreeMap.
    pub fn into_map(self) -> BTreeMap<String, String> {
        self.0
    }

    /// Returns the number of language-text pairs.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no language-text pairs.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
    #[serde(default)]
    pub content_type: String,
}

impl From<String> for ExpectedResponse {
    fn from(value: String) -> Self {
        Self {
            content_type: value
        }
    }
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

    /// Used to define the output data schema for an additional response
    /// if it differs from the default output data schema.
    /// Rather than a DataSchema object, the name of a previous definition
    /// given in a schemaDefinitions map must be used.
    pub schema: Option<String>,

    /// Indicates if this response is for an error case.
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub success: bool,
}

impl AdditionalExpectedResponse {
    pub fn new (response: impl Into<ExpectedResponse>, success: bool) -> Self {
        Self {
            _expected_response: response.into(),
            schema: None,
            success,
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
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

pub trait MetadataHelper: Sized {
    fn metadata(&mut self) -> &mut Metadata;

     /// Adds tags.
    fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String>,
        Self: Sized
    {
        let mut items: Vec<String> = tags.into_iter().map(|s| s.into()).collect();
        self.metadata().tags.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Sets the title.
    fn title(mut self, title: impl Into<String>) -> Self
    where
        Self: Sized
    {
        self.metadata().title = Some(title.into());
        self
    }

    /// Sets the multi-language titles.
    fn titles(mut self, titles: impl Into<MultiLanguage>) -> Self
    where
        Self: Sized
    {
        self.metadata().titles = Some(titles.into());
        self
    }

    /// Adds a title for a specific language.
    fn title_with_lang(mut self, lang: &str, title: &str) -> Self
    where
        Self: Sized
    {
        let titles = self.metadata().titles.get_or_insert_with(MultiLanguage::new);
        titles.add(lang, title);
        self
    }

    /// Sets the description.
    fn description(mut self, description: impl Into<String>) -> Self
    where
        Self: Sized
    {
        self.metadata().description = Some(description.into());
        self
    }

    /// Sets the multi-language descriptions.
    fn descriptions(mut self, descriptions: impl Into<MultiLanguage>) -> Self
    where
        Self: Sized
    {
        self.metadata().descriptions = Some(descriptions.into());
        self
    }

    /// Adds a description for a specific language.
    fn description_with_lang(mut self, lang: &str, description: &str) -> Self
    where
        Self: Sized
    {
        let descriptions = self.metadata().descriptions.get_or_insert_with(MultiLanguage::new);
        descriptions.add(lang, description);
        self
    }
}
