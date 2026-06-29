use alloc::{
    borrow::ToOwned,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use crate::{components_util::deserialize_bool_flexible, validate::Validate};
use fluent_uri::{ParseError, Uri, UriRef, resolve::ResolveError};
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
///
/// The parsed [`Uri`] is retained alongside the textual form so that callers
/// resolving relative references against this base can reuse the cached parse
/// instead of paying for re-parsing on every resolution (a per-request hot
/// path for protocol bindings).
#[derive(Debug, Clone, PartialEq)]
pub struct AbsoluteUri(Uri<String>);

impl AbsoluteUri {
    /// Creates an absolute URI from a static string. Panics if the input is invalid.
    /// Internal use only for known-good constants.
    pub(crate) fn from_static(s: &'static str) -> Self {
        Self::parse(s).expect("Invalid static absolute URI")
    }

    /// Parses an absolute URI. Relative references and URI templates are rejected.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let uri = Uri::parse(s)?;
        Ok(Self(uri.to_owned()))
    }

    /// Returns the string representation of the URI.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the cached parsed URI for cheap resolution against this base.
    ///
    /// Protocol bindings call [`UriReference::resolve_against`] with this
    /// reference instead of re-parsing the base on every request.
    pub(crate) fn as_uri(&self) -> &Uri<String> {
        &self.0
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
        match scan_template_expressions(s) {
            TemplateScan::Valid(static_uri) => {
                Uri::parse(static_uri.as_str())?;
                Ok(Self::Template(s.to_owned()))
            }
            // No template expressions, or malformed braces: fall back to strict
            // absolute-URI parsing so malformed inputs surface a parse error.
            _ => AbsoluteUri::parse(s).map(Self::Absolute),
        }
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

/// Result of a single-pass scan for URI template expressions.
enum TemplateScan {
    /// No template braces were found.
    None,
    /// At least one balanced `{...}` expression was found; carries the
    /// brace-stripped static portion for validation.
    Valid(String),
    /// Braces were present but unbalanced or nested.
    Invalid,
}

/// Walks `s` once, classifying it as a plain URI, a valid URI template, or a
/// malformed template. Replaces the previous two `contains` scans plus a third
/// strip pass with a single allocation-free walk (the static portion is only
/// built when the input is a valid template).
fn scan_template_expressions(s: &str) -> TemplateScan {
    let mut stripped = String::new();
    let mut in_expression = false;
    let mut has_expression = false;

    for c in s.chars() {
        match (c, in_expression) {
            ('{', false) => in_expression = true,
            ('{', true) => return TemplateScan::Invalid,
            ('}', true) => {
                in_expression = false;
                has_expression = true;
            }
            ('}', false) => return TemplateScan::Invalid,
            (_, false) => stripped.push(c),
            (_, true) => {}
        }
    }

    if in_expression {
        return TemplateScan::Invalid;
    }

    if has_expression {
        TemplateScan::Valid(stripped)
    } else {
        TemplateScan::None
    }
}

impl FormHref {
    /// Parses a form target.
    ///
    /// It first checks for URI Template characters ('{' and '}').
    /// If found, it treats the string as a Template. Otherwise, it attempts
    /// to parse it as a standard URI Reference using fluent-uri.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        // Single pass: a template requires both '{' and '}' to be present.
        let mut has_open = false;
        let mut has_close = false;
        for c in s.chars() {
            if c == '{' {
                has_open = true;
            } else if c == '}' {
                has_close = true;
            }
            if has_open && has_close {
                return Ok(Self::Template(s.to_owned()));
            }
        }

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

/// A protocol-neutral form target after applying the Thing-level `base` when
/// that can be done without URI template expansion.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedFormHref {
    /// A URI reference or absolute URI.
    Reference(UriReference),
    /// A URI template that must be expanded by a later binding/runtime step.
    Template(String),
}

impl ResolvedFormHref {
    /// Returns the string representation of the resolved target.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Reference(reference) => reference.as_str(),
            Self::Template(template) => template.as_str(),
        }
    }

    /// Returns true when the target is still a URI template.
    pub fn is_template(&self) -> bool {
        matches!(self, Self::Template(_))
    }
}

impl PartialEq<str> for ResolvedFormHref {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

/// Errors returned while resolving a form target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveFormHrefError {
    /// `base` contains URI template expressions and therefore cannot be used
    /// for concrete URI resolution without variable values.
    TemplateBase(String),
    /// RFC 3986 reference resolution failed.
    Resolve(String),
}

impl fmt::Display for ResolveFormHrefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemplateBase(base) => {
                write!(
                    f,
                    "Cannot resolve form href against URI template base: {}",
                    base
                )
            }
            Self::Resolve(message) => write!(f, "Failed to resolve form href: {}", message),
        }
    }
}

impl From<ResolveError> for ResolveFormHrefError {
    fn from(value: ResolveError) -> Self {
        Self::Resolve(value.to_string())
    }
}

/// Resolves a TD form `href` against an optional Thing-level `base`.
///
/// Absolute references are returned unchanged. Relative references are resolved
/// when `base` is a concrete absolute URI. URI templates are preserved because
/// this crate does not know the runtime variable values needed for expansion.
/// Relative references without a base are preserved for callers that use a
/// document URL or another protocol binding base outside the TD document.
pub fn resolve_form_href(
    base: Option<&BaseUri>,
    href: &FormHref,
) -> Result<ResolvedFormHref, ResolveFormHrefError> {
    let reference = match href {
        FormHref::Template(template) => {
            return Ok(ResolvedFormHref::Template(template.clone()));
        }
        FormHref::Reference(reference) => reference,
    };

    if reference.0.has_scheme() {
        return Ok(ResolvedFormHref::Reference(reference.clone()));
    }

    let Some(base) = base else {
        return Ok(ResolvedFormHref::Reference(reference.clone()));
    };

    let base = match base {
        BaseUri::Absolute(base) => base.as_uri(),
        BaseUri::Template(template) => {
            return Err(ResolveFormHrefError::TemplateBase(template.clone()));
        }
    };

    let resolved = reference.0.resolve_against(base)?;
    Ok(ResolvedFormHref::Reference(UriReference(resolved.into())))
}

/// Extension fields preserved from unknown TD terms.
pub type ExtensionMap = BTreeMap<String, serde_json::Value>;

impl Validate for ExtensionMap {
    fn validate_with_level(
        &self,
        _level: crate::validate::ValidationLevel,
    ) -> Result<(), crate::validate::ValidateError> {
        Ok(())
    }
}

/// A map of language tags to strings (e.g., {"en": "Light", "fr": "Lampe"}).
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
    pub fn from_pairs<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        iter.into_iter().collect()
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

    /// Merges another MultiLanguage into this one, moving entries instead of
    /// cloning them.
    pub fn merge_owned(&mut self, other: MultiLanguage) {
        self.0.extend(other.0);
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

impl FromIterator<(String, String)> for MultiLanguage {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (String, String)>,
    {
        Self(BTreeMap::from_iter(iter))
    }
}

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

/// Operation types of form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    ReadProperty,
    WriteProperty,
    ObserveProperty,
    UnobserveProperty,
    InvokeAction,
    QueryAction,
    /// Action cancellation (TD 1.1 `cancelaction`).
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
    /// Subscribe to all events (TD 1.1 `subscribeallevents`).
    SubscribeAllEvents,
    /// Unsubscribe from all events (TD 1.1 `unsubscribeallevents`).
    UnsubscribeAllEvents,
}

impl Operation {
    /// Returns the canonical lowercase operation name matching the W3C WoT
    /// TD serialization (`#[serde(rename_all = "lowercase")]`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadProperty => "readproperty",
            Self::WriteProperty => "writeproperty",
            Self::ObserveProperty => "observeproperty",
            Self::UnobserveProperty => "unobserveproperty",
            Self::InvokeAction => "invokeaction",
            Self::QueryAction => "queryaction",
            Self::CancelAction => "cancelaction",
            Self::SubscribeEvent => "subscribeevent",
            Self::UnsubscribeEvent => "unsubscribeevent",
            Self::ReadAllProperties => "readallproperties",
            Self::WriteAllProperties => "writeallproperties",
            Self::ReadMultipleProperties => "readmultipleproperties",
            Self::WriteMultipleProperties => "writemultipleproperties",
            Self::ObserveAllProperties => "observeallproperties",
            Self::UnobserveAllProperties => "unobserveallproperties",
            Self::QueryAllActions => "queryallactions",
            Self::SubscribeAllEvents => "subscribeallevents",
            Self::UnsubscribeAllEvents => "unsubscribeallevents",
        }
    }
}

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

/// JSON object keys owned by [`Metadata`] when flattened into a parent struct.
/// Drained out of the parent's buffer before the parent's own fields are read.
pub(crate) const METADATA_KEYS: &[&str] =
    &["@type", "title", "titles", "description", "descriptions"];

impl Metadata {
    /// Emits this metadata's fields inline into an in-progress serialized map.
    /// Used by structs that previously `#[serde(flatten)]`-ed a `Metadata`
    /// field, so the fields appear at the parent level without a nested object.
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        if let Some(tags) = &self.tags {
            map.serialize_entry("@type", &crate::flat::OneOrManyRef(tags))?;
        }
        if let Some(title) = &self.title {
            map.serialize_entry("title", title)?;
        }
        if let Some(titles) = &self.titles {
            map.serialize_entry("titles", titles)?;
        }
        if let Some(description) = &self.description {
            map.serialize_entry("description", description)?;
        }
        if let Some(descriptions) = &self.descriptions {
            map.serialize_entry("descriptions", descriptions)?;
        }
        Ok(())
    }
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
        self.metadata()
            .tags
            .get_or_insert_with(Vec::new)
            .extend(tags.into_iter().map(|s| s.into()));
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
