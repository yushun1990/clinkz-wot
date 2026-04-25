use alloc::{borrow::ToOwned, vec::Vec, string::String, collections::BTreeMap};

use fluent_uri::{ParseError, UriRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};
use crate::{components_util::deserialize_bool_flexible, validate::Validate};

/// Represents a WoT AnyUri which can be either a standard URI reference or a URI template.
///
/// This enum ensures fidelity during round-trip serialization by preserving the original
/// string representation while providing structured access for standard URIs.
#[derive(Debug, Clone, PartialEq)]
pub enum AnyUri {
    /// A standard URI or URI Reference compliant with RFC 3986 (e.g., "http://example.com" or "#td").
    Standard(UriRef<String>),
    /// A URI Template compliant with RFC 6570 containing placeholders (e.g., "/count{?fill}").
    Template(String),
}

impl AnyUri {
    /// Creates an AnyUri from a static string. Panics if the input is invalid.
    /// Internal use only for known-good constants.
    pub(crate) fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("Invalid static URI")
    }

    /// Parses a string into an AnyUri.
    ///
    /// It first checks for URI Template characters ('{' and '}').
    /// If found, it treats the string as a Template. Otherwise, it attempts
    /// to parse it as a standard URI Reference using fluent-uri.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        // Rule 1: Check for URI Template indicators
        if s.contains('{') && s.contains('}') {
            return Ok(Self::Template(s.to_owned()));
        }

        // Rule 2: Attempt strict URI Reference parsing
        let uri = UriRef::parse(s)?;
        Ok(Self::Standard(uri.into()))
    }

    /// Returns the string representation of the URI.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Standard(u) => u.as_str(),
            Self::Template(s) => s.as_str(),
        }
    }

    /// Checks if the URI is a template (RFC 6570).
    pub fn is_template(&self) -> bool {
        matches!(self, Self::Template(_))
    }
}

/// Allows comparison between AnyUri and string slices.
impl PartialEq<str> for AnyUri {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

/// Serializes AnyUri back into its original string form for transparent JSON output.
impl Serialize for AnyUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Deserializes a string into AnyUri by identifying its type (Standard vs Template).
impl<'de> Deserialize<'de> for AnyUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AnyUri::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Provides an empty standard URI as the default value.
impl Default for AnyUri {
    fn default() -> Self {
        Self::Standard(UriRef::parse("").unwrap().to_owned())
    }
}

/// Empty extended type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nil;

impl Validate for Nil {
    fn validate(&self) -> Result<(), crate::validate::ValidateError> {
        Ok(())
    }
}

/// Default extra fields type.
pub type DefaultExt = BTreeMap<String, serde_json::Value>;

impl Validate for DefaultExt {
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
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct VersionInfo<Ext=DefaultExt> {
    /// Provides a version indicator of this TD.
    pub instance: String,
    /// Provides a version indicator of underlying TM.
    pub model: Option<String>,

    #[serde(flatten)]
    pub _extra_fields: Ext
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
    ReadMultipleProperties,
    WriteMultipleProperties,
    ObserveAllProperties,
    UnobserveAllProperties,
    QueryAllActions,
    SubscribeAllEvents,
    UnsubscribeAllEvents,
}

/// Communication metadata describing the expected response message for the
/// primary response.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedResponse<Ext=DefaultExt> {
    /// Media type of the response payload (e.g., "application/json").
    pub content_type: String,

    #[serde(flatten)]
    pub _extra_fields: Ext,
}

impl <Ext> From<String> for ExpectedResponse<Ext>
where
   Ext: Default
{
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl <Ext> ExpectedResponse<Ext>
where
    Ext: Default
{
    pub fn new(value: String) -> Self {
        Self {
            content_type: value,
            _extra_fields: Default::default()
        }
    }

    pub fn extra_fields(mut self, extra_fields: impl Into<Ext>) -> Self {
        self._extra_fields = extra_fields.into();
        self
    }
}

/// Communication metadata describing the expected response message for
/// additional responses.
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalExpectedResponse<Ext=DefaultExt> {
    /// Mandatory, default to value of the contentType of the Form element it belongs to.
    #[serde(flatten)]
    pub content_type: Option<String>,

    /// Used to define the output data schema for an additional response
    /// if it differs from the default output data schema.
    /// Rather than a DataSchema object, the name of a previous definition
    /// given in a schemaDefinitions map must be used.
    pub schema: Option<String>,

    /// Indicates if this response is for an error case.
    #[serde(
        default,
        deserialize_with = "deserialize_bool_flexible",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub success: bool,

    #[serde(flatten)]
    pub _extra_fields: Ext,
}

impl <Ext> AdditionalExpectedResponse<Ext>
where
    Ext: Default
{
    pub fn new (content_type: String) -> Self {
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

    pub fn extra_fields(mut self, extra_fields: impl Into<Ext>) -> Self {
        self._extra_fields = extra_fields.into();
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
