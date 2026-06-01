use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};

use crate::{components_util::deserialize_bool_flexible, validate::Validate};
use fluent_uri::{ParseError, Uri, UriRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

/// URI reference compliant with RFC 3986.
#[derive(Debug, Clone, PartialEq)]
pub struct UriReference(UriRef<String>);

impl UriReference {
    /// Parses an absolute URI or relative URI reference.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let uri = UriRef::parse(s)?;
        Ok(Self(uri.into()))
    }

    /// Returns the string representation of the URI reference.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<str> for UriReference {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Serialize for UriReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UriReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        UriReference::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl Default for UriReference {
    fn default() -> Self {
        Self(UriRef::parse("").unwrap().to_owned())
    }
}

/// Form submission target.
///
/// Form `href` can be a URI reference or a URI template. This type should not
/// be reused for fields that only permit plain URI references.
#[derive(Debug, Clone, PartialEq)]
pub enum FormHref {
    /// A URI reference compliant with RFC 3986.
    Reference(UriReference),
    /// A URI Template compliant with RFC 6570 containing placeholders.
    Template(String),
}

/// Absolute URI value for TD fields that cannot be relative references.
///
/// WoT TD uses the JSON Schema `anyURI` lexical type in several places, but
/// individual fields narrow that range differently. Use `AbsoluteUri` for
/// fields that must identify an absolute resource, and `FormHref` for form
/// targets that may be relative references or URI templates.
#[derive(Debug, Clone, PartialEq)]
pub struct AbsoluteUri(String);

impl AbsoluteUri {
    /// Creates an absolute URI from a static string. Panics if the input is invalid.
    /// Internal use only for known-good constants.
    pub(crate) fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("Invalid static absolute URI")
    }

    /// Parses an absolute URI. Relative references and URI templates are rejected.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        Uri::parse(s)?;
        Ok(Self(s.to_owned()))
    }

    /// Returns the string representation of the URI.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<str> for AbsoluteUri {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Serialize for AbsoluteUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AbsoluteUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AbsoluteUri::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Thing-level base URI.
///
/// A base URI must provide an absolute base for resolving relative form
/// targets. Some real TDs use URI template expressions in `base`, so this type
/// permits absolute URI templates while still rejecting relative references.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseUri {
    /// Absolute URI without template expressions.
    Absolute(AbsoluteUri),
    /// Absolute URI template.
    Template(String),
}

impl BaseUri {
    /// Parses a Thing-level base URI.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        if s.contains('{') && s.contains('}') {
            if let Some(static_uri) = strip_uri_template_expressions(s) {
                Uri::parse(static_uri.as_str())?;
                return Ok(Self::Template(s.to_owned()));
            }
        }

        AbsoluteUri::parse(s).map(Self::Absolute)
    }

    /// Returns the string representation of the base URI.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Absolute(uri) => uri.as_str(),
            Self::Template(template) => template.as_str(),
        }
    }

    /// Returns true when this base contains URI template expressions.
    pub fn is_template(&self) -> bool {
        matches!(self, Self::Template(_))
    }
}

impl PartialEq<str> for BaseUri {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Serialize for BaseUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BaseUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        BaseUri::parse(&s).map_err(serde::de::Error::custom)
    }
}

fn strip_uri_template_expressions(s: &str) -> Option<String> {
    let mut stripped = String::new();
    let mut in_expression = false;

    for c in s.chars() {
        match (c, in_expression) {
            ('{', false) => in_expression = true,
            ('{', true) => return None,
            ('}', true) => in_expression = false,
            ('}', false) => return None,
            (_, false) => stripped.push(c),
            (_, true) => {}
        }
    }

    if in_expression {
        return None;
    }

    Some(stripped)
}

impl FormHref {
    /// Parses a form target.
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
        UriReference::parse(s).map(Self::Reference)
    }

    /// Returns the string representation of the target.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Reference(u) => u.as_str(),
            Self::Template(s) => s.as_str(),
        }
    }

    /// Checks if the URI is a template (RFC 6570).
    pub fn is_template(&self) -> bool {
        matches!(self, Self::Template(_))
    }
}

impl PartialEq<str> for FormHref {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Serialize for FormHref {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FormHref {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FormHref::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl Default for FormHref {
    fn default() -> Self {
        Self::Reference(UriReference::default())
    }
}

/// Extension fields preserved from unknown TD terms.
pub type ExtensionMap = BTreeMap<String, serde_json::Value>;

impl Validate for ExtensionMap {
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
        I: IntoIterator<Item = (String, String)>,
    {
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
        self.0
            .extend(other.0.iter().map(|(k, v)| (k.clone(), v.clone())));
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
pub struct VersionInfo {
    /// Provides a version indicator of this TD.
    pub instance: String,
    /// Provides a version indicator of underlying TM.
    pub model: Option<String>,

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
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
pub struct ExpectedResponse {
    /// Media type of the response payload (e.g., "application/json").
    pub content_type: String,

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
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
        self._extra_fields = extra_fields.into();
        self
    }
}

/// Communication metadata describing the expected response message for
/// additional responses.
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalExpectedResponse {
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
    pub _extra_fields: ExtensionMap,
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
        self._extra_fields = extra_fields.into();
        self
    }
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

pub trait MetadataHelper: Sized {
    fn metadata(&mut self) -> &mut Metadata;

    /// Adds tags.
    fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        Self: Sized,
    {
        let mut items: Vec<String> = tags.into_iter().map(|s| s.into()).collect();
        self.metadata()
            .tags
            .get_or_insert_with(Vec::new)
            .append(&mut items);
        self
    }

    /// Sets the title.
    fn title(mut self, title: impl Into<String>) -> Self
    where
        Self: Sized,
    {
        self.metadata().title = Some(title.into());
        self
    }

    /// Sets the multi-language titles.
    fn titles(mut self, titles: impl Into<MultiLanguage>) -> Self
    where
        Self: Sized,
    {
        self.metadata().titles = Some(titles.into());
        self
    }

    /// Adds a title for a specific language.
    fn title_with_lang(mut self, lang: &str, title: &str) -> Self
    where
        Self: Sized,
    {
        let titles = self
            .metadata()
            .titles
            .get_or_insert_with(MultiLanguage::new);
        titles.add(lang, title);
        self
    }

    /// Sets the description.
    fn description(mut self, description: impl Into<String>) -> Self
    where
        Self: Sized,
    {
        self.metadata().description = Some(description.into());
        self
    }

    /// Sets the multi-language descriptions.
    fn descriptions(mut self, descriptions: impl Into<MultiLanguage>) -> Self
    where
        Self: Sized,
    {
        self.metadata().descriptions = Some(descriptions.into());
        self
    }

    /// Adds a description for a specific language.
    fn description_with_lang(mut self, lang: &str, description: &str) -> Self
    where
        Self: Sized,
    {
        let descriptions = self
            .metadata()
            .descriptions
            .get_or_insert_with(MultiLanguage::new);
        descriptions.add(lang, description);
        self
    }
}
