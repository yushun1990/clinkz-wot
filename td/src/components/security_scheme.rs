use alloc::{
    borrow::Cow,
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{OneOrMany, serde_as};

use crate::{
    data_type::{AbsoluteUri, ExtensionMap, MultiLanguage},
    validate::{Validate, ValidateError, ValidationLevel, parse_uri_field},
};

/// Deserialize adapter carrying the `serde_as(Option<OneOrMany<_>>)` decoder
/// for `SecuritySchemeContext::tags`.
#[serde_as]
#[derive(Deserialize)]
struct TagsField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<String>>);

/// Shared base for every security scheme variant: semantic tags, description,
/// proxy, the mandatory `scheme` discriminator, and preserved extension fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SecuritySchemeContext {
    /// JSON-LD keyword to label the object with semantic tags.
    pub tags: Option<Vec<String>>,

    /// Provides additional (human-readable) information based on a
    /// default language.
    pub description: Option<String>,

    /// Multi-language descriptions.
    pub descriptions: Option<MultiLanguage>,

    /// URI of the proxy server this security configuration provides
    /// access to.
    pub proxy: Option<AbsoluteUri>,

    /// Identification of the security mechanism being configured.
    pub scheme: String,

    pub _extra_fields: ExtensionMap,
}

impl SecuritySchemeContext {
    pub fn new(scheme: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            ..Default::default()
        }
    }

    /// Emits this context's fields inline into an in-progress serialized map.
    /// Used by security scheme variants that previously `#[serde(flatten)]`-ed
    /// the context, so its fields appear at the variant level without nesting.
    pub(crate) fn serialize_into<S: serde::ser::SerializeMap>(
        &self,
        map: &mut S,
    ) -> Result<(), S::Error> {
        if let Some(tags) = &self.tags {
            map.serialize_entry("@type", &crate::flat::OneOrManyRef(tags))?;
        }
        if let Some(description) = &self.description {
            map.serialize_entry("description", description)?;
        }
        if let Some(descriptions) = &self.descriptions {
            map.serialize_entry("descriptions", descriptions)?;
        }
        if let Some(proxy) = &self.proxy {
            map.serialize_entry("proxy", proxy)?;
        }
        map.serialize_entry("scheme", &self.scheme)?;
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for SecuritySchemeContext {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let tags =
            crate::flat::take::<TagsField, D::Error>(&mut map, "@type")?.and_then(|field| field.0);
        let description = crate::flat::take(&mut map, "description")?;
        let descriptions = crate::flat::take(&mut map, "descriptions")?;
        let proxy = crate::flat::take(&mut map, "proxy")?;
        let scheme = crate::flat::take_required(&mut map, "scheme")?;
        Ok(SecuritySchemeContext {
            tags,
            description,
            descriptions,
            proxy,
            scheme,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for SecuritySchemeContext {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self.serialize_into(&mut map)?;
        map.end()
    }
}

pub trait ContextHelper: Sized {
    fn context(&mut self) -> &mut SecuritySchemeContext;

    /// Returns a mutable reference to the builder's accumulated validation
    /// errors. Provided methods that can fail (e.g. [`proxy`](Self::proxy))
    /// push errors here so they are surfaced by `build()`.
    fn builder_errors(&mut self) -> &mut Vec<ValidateError>;

    /// Adds tags.
    fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.context()
            .tags
            .get_or_insert_with(Vec::new)
            .extend(tags.into_iter().map(|s| s.into()));
        self
    }

    /// Sets the description.
    fn description(mut self, description: impl Into<String>) -> Self {
        self.context().description = Some(description.into());
        self
    }

    /// Sets the multi-language descriptions.
    fn descriptions(mut self, descriptions: impl Into<MultiLanguage>) -> Self {
        self.context().descriptions = Some(descriptions.into());
        self
    }

    /// Adds a description for a specific language.
    fn description_with_lang(mut self, lang: &str, description: &str) -> Self {
        let descriptions = self
            .context()
            .descriptions
            .get_or_insert_with(MultiLanguage::new);
        descriptions.add(lang, description);
        self
    }

    /// Sets the proxy URI.
    fn proxy(mut self, proxy: impl Into<String>) -> Self {
        let proxy = proxy.into();
        if let Some(uri) = parse_uri_field(
            "proxy",
            proxy.as_str(),
            AbsoluteUri::parse,
            self.builder_errors(),
        ) {
            self.context().proxy = Some(uri);
        }
        self
    }

    fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.context()._extra_fields.extend(extra_fields.into());
        self
    }

    fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context()._extra_fields.insert(key.into(), value);
        self
    }
}

fn check_builder_errors(errors: Vec<ValidateError>) -> Result<(), ValidateError> {
    crate::validate::collected_errors(errors)
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `nosec`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NoSecurityScheme {
    pub _context: SecuritySchemeContext,
}

impl<'de> Deserialize<'de> for NoSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = crate::flat::deserialize_map(deserializer)?;
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(NoSecurityScheme { _context: context })
    }
}

impl Serialize for NoSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        map.end()
    }
}

impl NoSecurityScheme {
    pub fn builder() -> NoSecuritySchemeBuilder {
        NoSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `NoSecurityScheme` instances.
pub struct NoSecuritySchemeBuilder {
    scheme: NoSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl NoSecuritySchemeBuilder {
    /// Creates a new `NoSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: NoSecurityScheme {
                _context: SecuritySchemeContext::new("nosec"),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Builds and returns the `NoSecurityScheme` instance.
    pub fn build(mut self) -> Result<NoSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for NoSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `auto`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AutoSecurityScheme {
    pub _context: SecuritySchemeContext,
}

impl<'de> Deserialize<'de> for AutoSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = crate::flat::deserialize_map(deserializer)?;
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(AutoSecurityScheme { _context: context })
    }
}

impl Serialize for AutoSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        map.end()
    }
}

impl AutoSecurityScheme {
    pub fn builder() -> AutoSecuritySchemeBuilder {
        AutoSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `AutoSecurityScheme` instances.
pub struct AutoSecuritySchemeBuilder {
    scheme: AutoSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl AutoSecuritySchemeBuilder {
    /// Creates a new `AutoSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: AutoSecurityScheme {
                _context: SecuritySchemeContext::new("auto"),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Builds and returns the `AutoSecurityScheme` instance.
    pub fn build(mut self) -> Result<AutoSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for AutoSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `combo`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComboSecurityScheme {
    pub _context: SecuritySchemeContext,

    /// Array of two or more strings identifying other named security
    /// scheme definitions, any one of which, when satisfied, will
    /// allow access.
    pub one_of: Vec<String>,

    /// Array of two or more strings identifying other named security
    /// scheme definitions, all of which must be satisfied for access.
    pub all_of: Vec<String>,
}

impl<'de> Deserialize<'de> for ComboSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let one_of =
            crate::flat::take::<Vec<String>, D::Error>(&mut map, "oneOf")?.unwrap_or_default();
        let all_of =
            crate::flat::take::<Vec<String>, D::Error>(&mut map, "allOf")?.unwrap_or_default();
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(ComboSecurityScheme {
            _context: context,
            one_of,
            all_of,
        })
    }
}

impl Serialize for ComboSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if !self.one_of.is_empty() {
            map.serialize_entry("oneOf", &self.one_of)?;
        }
        if !self.all_of.is_empty() {
            map.serialize_entry("allOf", &self.all_of)?;
        }
        map.end()
    }
}

impl ComboSecurityScheme {
    pub fn builder() -> ComboSecuritySchemeBuilder {
        ComboSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `ComboSecurityScheme` instances.
pub struct ComboSecuritySchemeBuilder {
    scheme: ComboSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl ComboSecuritySchemeBuilder {
    /// Creates a new `ComboSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: ComboSecurityScheme {
                _context: SecuritySchemeContext::new("combo"),
                one_of: Vec::new(),
                all_of: Vec::new(),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Adds security schemes to one_of.
    pub fn one_of<I, S>(mut self, schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scheme
            .one_of
            .extend(schemes.into_iter().map(|s| s.into()));
        self
    }

    /// Adds security schemes to all_of.
    pub fn all_of<I, S>(mut self, schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scheme
            .all_of
            .extend(schemes.into_iter().map(|s| s.into()));
        self
    }

    /// Builds and returns the `ComboSecurityScheme` instance.
    pub fn build(mut self) -> Result<ComboSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for ComboSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLocation {
    #[default]
    Header,
    Query,
    Uri,
    Body,
    Cookie,
    Auto,
}

fn is_default_location(location: &SecurityLocation) -> bool {
    location == &SecurityLocation::Header
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `basic`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BasicSecurityScheme {
    pub _context: SecuritySchemeContext,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    pub location: SecurityLocation,
}

impl<'de> Deserialize<'de> for BasicSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let name = crate::flat::take(&mut map, "name")?;
        let location =
            crate::flat::take::<SecurityLocation, D::Error>(&mut map, "in")?.unwrap_or_default();
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(BasicSecurityScheme {
            _context: context,
            name,
            location,
        })
    }
}

impl Serialize for BasicSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if let Some(name) = &self.name {
            map.serialize_entry("name", name)?;
        }
        if !is_default_location(&self.location) {
            map.serialize_entry("in", &self.location)?;
        }
        map.end()
    }
}

impl BasicSecurityScheme {
    pub fn builder() -> BasicSecuritySchemeBuilder {
        BasicSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `BasicSecurityScheme` instances.
pub struct BasicSecuritySchemeBuilder {
    scheme: BasicSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl BasicSecuritySchemeBuilder {
    /// Creates a new `BasicSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: BasicSecurityScheme {
                _context: SecuritySchemeContext::new("basic"),
                name: None,
                location: SecurityLocation::default(),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Sets the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.scheme.name = Some(name.into());
        self
    }

    /// Sets the location.
    pub fn location(mut self, location: SecurityLocation) -> Self {
        self.scheme.location = location;
        self
    }

    /// Builds and returns the `BasicSecurityScheme` instance.
    pub fn build(mut self) -> Result<BasicSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for BasicSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Qop {
    /// Authentication only
    #[default]
    Auth,
    /// Authentication with integrity protection
    AuthInt,
}

fn is_default_qop(qop: &Qop) -> bool {
    qop == &Qop::Auth
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `digest`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DigestSecurityScheme {
    pub _context: SecuritySchemeContext,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    pub location: SecurityLocation,

    /// Quality of protection.
    pub qop: Qop,
}

impl<'de> Deserialize<'de> for DigestSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let name = crate::flat::take(&mut map, "name")?;
        let location =
            crate::flat::take::<SecurityLocation, D::Error>(&mut map, "in")?.unwrap_or_default();
        let qop = crate::flat::take::<Qop, D::Error>(&mut map, "qop")?.unwrap_or_default();
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(DigestSecurityScheme {
            _context: context,
            name,
            location,
            qop,
        })
    }
}

impl Serialize for DigestSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if let Some(name) = &self.name {
            map.serialize_entry("name", name)?;
        }
        if !is_default_location(&self.location) {
            map.serialize_entry("in", &self.location)?;
        }
        if !is_default_qop(&self.qop) {
            map.serialize_entry("qop", &self.qop)?;
        }
        map.end()
    }
}

impl DigestSecurityScheme {
    pub fn builder() -> DigestSecuritySchemeBuilder {
        DigestSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `DigestSecurityScheme` instances.
pub struct DigestSecuritySchemeBuilder {
    scheme: DigestSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl DigestSecuritySchemeBuilder {
    /// Creates a new `DigestSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: DigestSecurityScheme {
                _context: SecuritySchemeContext::new("digest"),
                name: None,
                location: SecurityLocation::default(),
                qop: Qop::default(),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Sets the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.scheme.name = Some(name.into());
        self
    }

    /// Sets the location.
    pub fn location(mut self, location: SecurityLocation) -> Self {
        self.scheme.location = location;
        self
    }

    /// Sets the quality of protection.
    pub fn qop(mut self, qop: Qop) -> Self {
        self.scheme.qop = qop;
        self
    }

    /// Builds and returns the `DigestSecurityScheme` instance.
    pub fn build(mut self) -> Result<DigestSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for DigestSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `apikey`.
#[derive(Clone, Debug, PartialEq)]
pub struct APIKeySecurityScheme {
    pub _context: SecuritySchemeContext,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    ///
    /// Per TD 1.1 §5.4 the default `in` for an API key scheme is `query`
    /// (unlike basic/digest/bearer, which default to `header`).
    pub location: SecurityLocation,
}

impl<'de> Deserialize<'de> for APIKeySecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let name = crate::flat::take(&mut map, "name")?;
        let location = crate::flat::take::<SecurityLocation, D::Error>(&mut map, "in")?
            .unwrap_or_else(default_apikey_location);
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(APIKeySecurityScheme {
            _context: context,
            name,
            location,
        })
    }
}

impl Serialize for APIKeySecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if let Some(name) = &self.name {
            map.serialize_entry("name", name)?;
        }
        if !is_default_apikey_location(&self.location) {
            map.serialize_entry("in", &self.location)?;
        }
        map.end()
    }
}

// TD 1.1 §5.4: `APIKeySecurityScheme.in` defaults to `query`.
fn default_apikey_location() -> SecurityLocation {
    SecurityLocation::Query
}

fn is_default_apikey_location(location: &SecurityLocation) -> bool {
    location == &SecurityLocation::Query
}

impl Default for APIKeySecurityScheme {
    fn default() -> Self {
        Self {
            _context: SecuritySchemeContext::new("apikey"),
            name: None,
            location: SecurityLocation::Query,
        }
    }
}

impl APIKeySecurityScheme {
    pub fn builder() -> APIKeySecuritySchemeBuilder {
        APIKeySecuritySchemeBuilder::new()
    }
}

/// Builder for creating `APIKeySecurityScheme` instances.
pub struct APIKeySecuritySchemeBuilder {
    scheme: APIKeySecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl APIKeySecuritySchemeBuilder {
    /// Creates a new `APIKeySecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: APIKeySecurityScheme {
                _context: SecuritySchemeContext::new("apikey"),
                name: None,
                location: SecurityLocation::Query,
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Sets the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.scheme.name = Some(name.into());
        self
    }

    /// Sets the location.
    pub fn location(mut self, location: SecurityLocation) -> Self {
        self.scheme.location = location;
        self
    }

    /// Builds and returns the `APIKeySecurityScheme` instance.
    pub fn build(mut self) -> Result<APIKeySecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for APIKeySecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

/// Helper function to provide the default algorithm "ES256"
const DEFAULT_ALG: &str = "ES256";

fn default_alg() -> String {
    String::from(DEFAULT_ALG)
}

fn is_default_alg(alg: &str) -> bool {
    alg == DEFAULT_ALG
}

/// Helper function to provide the default format "jwt"
const DEFAULT_FORMAT: &str = "jwt";

fn default_format() -> String {
    String::from(DEFAULT_FORMAT)
}

fn is_default_format(format: &str) -> bool {
    format == DEFAULT_FORMAT
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `bearer`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BearerSecurityScheme {
    pub _context: SecuritySchemeContext,

    /// URI of the authorization server.
    pub authorization: Option<AbsoluteUri>,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Encoding, encryption, or digest algorithm.
    pub alg: String,

    /// Specifies format of security authentication information.
    pub format: String,

    /// Specifies the location of security authentication information.
    pub location: SecurityLocation,
}

impl<'de> Deserialize<'de> for BearerSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let authorization = crate::flat::take(&mut map, "authorization")?;
        let name = crate::flat::take(&mut map, "name")?;
        let alg =
            crate::flat::take::<String, D::Error>(&mut map, "alg")?.unwrap_or_else(default_alg);
        let format = crate::flat::take::<String, D::Error>(&mut map, "format")?
            .unwrap_or_else(default_format);
        let location =
            crate::flat::take::<SecurityLocation, D::Error>(&mut map, "in")?.unwrap_or_default();
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(BearerSecurityScheme {
            _context: context,
            authorization,
            name,
            alg,
            format,
            location,
        })
    }
}

impl Serialize for BearerSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if let Some(authorization) = &self.authorization {
            map.serialize_entry("authorization", authorization)?;
        }
        if let Some(name) = &self.name {
            map.serialize_entry("name", name)?;
        }
        if !is_default_alg(&self.alg) {
            map.serialize_entry("alg", &self.alg)?;
        }
        if !is_default_format(&self.format) {
            map.serialize_entry("format", &self.format)?;
        }
        if !is_default_location(&self.location) {
            map.serialize_entry("in", &self.location)?;
        }
        map.end()
    }
}

impl BearerSecurityScheme {
    pub fn builder() -> BearerSecuritySchemeBuilder {
        BearerSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `BearerSecurityScheme` instances.
pub struct BearerSecuritySchemeBuilder {
    scheme: BearerSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl BearerSecuritySchemeBuilder {
    /// Creates a new `BearerSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: BearerSecurityScheme {
                _context: SecuritySchemeContext::new("bearer"),
                authorization: None,
                name: None,
                alg: default_alg(),
                format: default_format(),
                location: SecurityLocation::default(),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Sets the authorization URI.
    pub fn authorization(mut self, authorization: impl Into<String>) -> Self {
        let authorization = authorization.into();
        if let Some(uri) = parse_uri_field(
            "authorization",
            authorization.as_str(),
            AbsoluteUri::parse,
            &mut self._builder_errors,
        ) {
            self.scheme.authorization = Some(uri);
        }
        self
    }

    /// Sets the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.scheme.name = Some(name.into());
        self
    }

    /// Sets the algorithm.
    pub fn alg(mut self, alg: impl Into<String>) -> Self {
        self.scheme.alg = alg.into();
        self
    }

    /// Sets the format.
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.scheme.format = format.into();
        self
    }

    /// Sets the location.
    pub fn location(mut self, location: SecurityLocation) -> Self {
        self.scheme.location = location;
        self
    }

    /// Builds and returns the `BearerSecurityScheme` instance.
    pub fn build(mut self) -> Result<BearerSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for BearerSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `psk`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PSKSecurityScheme {
    pub _context: SecuritySchemeContext,

    /// Identifier providing information which can be used for
    /// selection or confirmation.
    pub identity: Option<String>,
}

impl<'de> Deserialize<'de> for PSKSecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let identity = crate::flat::take(&mut map, "identity")?;
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(PSKSecurityScheme {
            _context: context,
            identity,
        })
    }
}

impl Serialize for PSKSecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if let Some(identity) = &self.identity {
            map.serialize_entry("identity", identity)?;
        }
        map.end()
    }
}

impl PSKSecurityScheme {
    pub fn builder() -> PSKSecuritySchemeBuilder {
        PSKSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `PSKSecurityScheme` instances.
pub struct PSKSecuritySchemeBuilder {
    scheme: PSKSecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl PSKSecuritySchemeBuilder {
    /// Creates a new `PSKSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: PSKSecurityScheme {
                _context: SecuritySchemeContext::new("psk"),
                identity: None,
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Sets the identity.
    pub fn identity(mut self, identity: impl Into<String>) -> Self {
        self.scheme.identity = Some(identity.into());
        self
    }

    /// Builds and returns the `PSKSecurityScheme` instance.
    pub fn build(mut self) -> Result<PSKSecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for PSKSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

/// Deserialize adapter carrying the `serde_as(Option<OneOrMany<_>>)` decoder
/// for `OAuth2SecurityScheme::scopes`.
#[serde_as]
#[derive(Deserialize)]
struct ScopesField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<String>>);

/// A security configuration corresponding to identified by the
/// Vocabulary Term `oauth2`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OAuth2SecurityScheme {
    pub _context: SecuritySchemeContext,

    /// URI of the authorization server.
    pub authorization: Option<AbsoluteUri>,

    /// URI of the token server.
    pub token: Option<AbsoluteUri>,

    /// URI of the refresh server.
    pub refresh: Option<AbsoluteUri>,

    /// Set of authorization scope identifier provided as an array.
    pub scopes: Option<Vec<String>>,

    /// Authorization flow, e.g., code, client.
    pub flow: String,
}

impl<'de> Deserialize<'de> for OAuth2SecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let authorization = crate::flat::take(&mut map, "authorization")?;
        let token = crate::flat::take(&mut map, "token")?;
        let refresh = crate::flat::take(&mut map, "refresh")?;
        let scopes = crate::flat::take::<ScopesField, D::Error>(&mut map, "scopes")?
            .and_then(|field| field.0);
        let flow = crate::flat::take_required(&mut map, "flow")?;
        let context = crate::flat::from_remaining::<SecuritySchemeContext, D::Error>(map)?;
        Ok(OAuth2SecurityScheme {
            _context: context,
            authorization,
            token,
            refresh,
            scopes,
            flow,
        })
    }
}

impl Serialize for OAuth2SecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._context.serialize_into(&mut map)?;
        if let Some(authorization) = &self.authorization {
            map.serialize_entry("authorization", authorization)?;
        }
        if let Some(token) = &self.token {
            map.serialize_entry("token", token)?;
        }
        if let Some(refresh) = &self.refresh {
            map.serialize_entry("refresh", refresh)?;
        }
        if let Some(scopes) = &self.scopes {
            map.serialize_entry("scopes", &crate::flat::OneOrManyRef(scopes))?;
        }
        map.serialize_entry("flow", &self.flow)?;
        map.end()
    }
}

impl OAuth2SecurityScheme {
    pub fn builder(flow: impl Into<String>) -> OAuth2SecuritySchemeBuilder {
        OAuth2SecuritySchemeBuilder::new(flow)
    }
}

/// Builder for creating `OAuth2SecurityScheme` instances.
pub struct OAuth2SecuritySchemeBuilder {
    scheme: OAuth2SecurityScheme,
    _builder_errors: Vec<ValidateError>,
}

impl OAuth2SecuritySchemeBuilder {
    /// Creates a new `OAuth2SecuritySchemeBuilder` with the required `flow` field.
    pub fn new(flow: impl Into<String>) -> Self {
        Self {
            scheme: OAuth2SecurityScheme {
                _context: SecuritySchemeContext::new("oauth2"),
                authorization: None,
                token: None,
                refresh: None,
                scopes: None,
                flow: flow.into(),
            },
            _builder_errors: Vec::new(),
        }
    }

    /// Sets the authorization URI.
    pub fn authorization(mut self, authorization: impl Into<String>) -> Self {
        let authorization = authorization.into();
        if let Some(uri) = parse_uri_field(
            "authorization",
            authorization.as_str(),
            AbsoluteUri::parse,
            &mut self._builder_errors,
        ) {
            self.scheme.authorization = Some(uri);
        }
        self
    }

    /// Sets the token URI.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        if let Some(uri) = parse_uri_field(
            "token",
            token.as_str(),
            AbsoluteUri::parse,
            &mut self._builder_errors,
        ) {
            self.scheme.token = Some(uri);
        }
        self
    }

    /// Sets the refresh URI.
    pub fn refresh(mut self, refresh: impl Into<String>) -> Self {
        let refresh = refresh.into();
        if let Some(uri) = parse_uri_field(
            "refresh",
            refresh.as_str(),
            AbsoluteUri::parse,
            &mut self._builder_errors,
        ) {
            self.scheme.refresh = Some(uri);
        }
        self
    }

    /// Adds scopes.
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scheme
            .scopes
            .get_or_insert_with(Vec::new)
            .extend(scopes.into_iter().map(|s| s.into()));
        self
    }

    /// Builds and returns the `OAuth2SecurityScheme` instance.
    pub fn build(mut self) -> Result<OAuth2SecurityScheme, ValidateError> {
        check_builder_errors(core::mem::take(&mut self._builder_errors))?;
        Ok(self.scheme)
    }
}

impl ContextHelper for OAuth2SecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }

    fn builder_errors(&mut self) -> &mut Vec<ValidateError> {
        &mut self._builder_errors
    }
}

impl_builder_default!(
    NoSecuritySchemeBuilder,
    AutoSecuritySchemeBuilder,
    ComboSecuritySchemeBuilder,
    BasicSecuritySchemeBuilder,
    DigestSecuritySchemeBuilder,
    APIKeySecuritySchemeBuilder,
    BearerSecuritySchemeBuilder,
    PSKSecuritySchemeBuilder,
);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SecurityScheme {
    NoSec(NoSecurityScheme),
    Auto(AutoSecurityScheme),
    Combo(ComboSecurityScheme),
    Basic(BasicSecurityScheme),
    Digest(DigestSecurityScheme),
    APIKey(APIKeySecurityScheme),
    Bearer(BearerSecurityScheme),
    PSK(PSKSecurityScheme),
    OAuth2(OAuth2SecurityScheme),
}

/// Lightweight discriminator probe used to dispatch [`SecurityScheme`] variants
/// without materializing the full `serde_json::Value` tree.
#[derive(Deserialize)]
struct SchemePeek {
    scheme: String,
}

impl<'de> Deserialize<'de> for SecurityScheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Buffer the object once into a `Box<RawValue>` (a single byte-buffer
        // allocation) instead of a full `serde_json::Value` tree (one
        // allocation per node). See `DataSchema::deserialize` for why this is
        // only possible now that the surrounding TD structs buffer through
        // `serde_json::Map` rather than serde's `Content` layer.
        let raw: Box<serde_json::value::RawValue> = Deserialize::deserialize(deserializer)?;
        let peek: SchemePeek = serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
        match peek.scheme.as_str() {
            "nosec" => serde_json::from_str::<NoSecurityScheme>(raw.get()).map(Self::NoSec),
            "auto" => serde_json::from_str::<AutoSecurityScheme>(raw.get()).map(Self::Auto),
            "combo" => serde_json::from_str::<ComboSecurityScheme>(raw.get()).map(Self::Combo),
            "basic" => serde_json::from_str::<BasicSecurityScheme>(raw.get()).map(Self::Basic),
            "digest" => serde_json::from_str::<DigestSecurityScheme>(raw.get()).map(Self::Digest),
            "apikey" => serde_json::from_str::<APIKeySecurityScheme>(raw.get()).map(Self::APIKey),
            "bearer" => serde_json::from_str::<BearerSecurityScheme>(raw.get()).map(Self::Bearer),
            "psk" => serde_json::from_str::<PSKSecurityScheme>(raw.get()).map(Self::PSK),
            "oauth2" => serde_json::from_str::<OAuth2SecurityScheme>(raw.get()).map(Self::OAuth2),
            other => Err(serde::de::Error::custom(format!(
                "unsupported security scheme '{}'",
                other
            ))),
        }
        .map_err(serde::de::Error::custom)
    }
}

impl From<NoSecurityScheme> for SecurityScheme {
    fn from(scheme: NoSecurityScheme) -> Self {
        Self::NoSec(scheme)
    }
}

impl From<AutoSecurityScheme> for SecurityScheme {
    fn from(scheme: AutoSecurityScheme) -> Self {
        Self::Auto(scheme)
    }
}

impl From<ComboSecurityScheme> for SecurityScheme {
    fn from(scheme: ComboSecurityScheme) -> Self {
        Self::Combo(scheme)
    }
}

impl From<BasicSecurityScheme> for SecurityScheme {
    fn from(scheme: BasicSecurityScheme) -> Self {
        Self::Basic(scheme)
    }
}

impl From<DigestSecurityScheme> for SecurityScheme {
    fn from(scheme: DigestSecurityScheme) -> Self {
        Self::Digest(scheme)
    }
}

impl From<APIKeySecurityScheme> for SecurityScheme {
    fn from(scheme: APIKeySecurityScheme) -> Self {
        Self::APIKey(scheme)
    }
}

impl From<BearerSecurityScheme> for SecurityScheme {
    fn from(scheme: BearerSecurityScheme) -> Self {
        Self::Bearer(scheme)
    }
}

impl From<PSKSecurityScheme> for SecurityScheme {
    fn from(scheme: PSKSecurityScheme) -> Self {
        Self::PSK(scheme)
    }
}

impl From<OAuth2SecurityScheme> for SecurityScheme {
    fn from(scheme: OAuth2SecurityScheme) -> Self {
        Self::OAuth2(scheme)
    }
}

impl SecurityScheme {
    /// Creates a `nosec` security scheme.
    pub fn nosec() -> Self {
        NoSecurityScheme {
            _context: SecuritySchemeContext::new("nosec"),
        }
        .into()
    }

    /// Creates an `auto` security scheme.
    pub fn auto() -> Self {
        AutoSecurityScheme {
            _context: SecuritySchemeContext::new("auto"),
        }
        .into()
    }

    /// Creates a `basic` security scheme with the required name.
    pub fn basic(name: impl Into<String>) -> Self {
        BasicSecurityScheme {
            _context: SecuritySchemeContext::new("basic"),
            name: Some(name.into()),
            location: SecurityLocation::default(),
        }
        .into()
    }

    /// Creates a `digest` security scheme with the required name.
    pub fn digest(name: impl Into<String>) -> Self {
        DigestSecurityScheme {
            _context: SecuritySchemeContext::new("digest"),
            name: Some(name.into()),
            location: SecurityLocation::default(),
            qop: Qop::default(),
        }
        .into()
    }

    /// Creates an `apikey` security scheme with the required name.
    pub fn apikey(name: impl Into<String>) -> Self {
        APIKeySecurityScheme {
            _context: SecuritySchemeContext::new("apikey"),
            name: Some(name.into()),
            location: SecurityLocation::Query,
        }
        .into()
    }

    /// Creates a `bearer` security scheme with the required name.
    pub fn bearer(name: impl Into<String>) -> Self {
        BearerSecurityScheme {
            _context: SecuritySchemeContext::new("bearer"),
            authorization: None,
            name: Some(name.into()),
            alg: default_alg(),
            format: default_format(),
            location: SecurityLocation::default(),
        }
        .into()
    }

    /// Creates a `bearer` security scheme with an authorization endpoint.
    pub fn bearer_authorization(
        name: impl Into<String>,
        authorization: impl Into<String>,
    ) -> Result<Self, ValidateError> {
        Ok(BearerSecurityScheme::builder()
            .name(name)
            .authorization(authorization)
            .build()?
            .into())
    }

    /// Creates a `psk` security scheme with an identity hint.
    pub fn psk(identity: impl Into<String>) -> Self {
        PSKSecurityScheme {
            _context: SecuritySchemeContext::new("psk"),
            identity: Some(identity.into()),
        }
        .into()
    }

    /// Creates a `combo` security scheme where any referenced scheme may satisfy access.
    pub fn combo_one_of<I, S>(schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        ComboSecurityScheme {
            _context: SecuritySchemeContext::new("combo"),
            one_of: schemes.into_iter().map(Into::into).collect(),
            all_of: Vec::new(),
        }
        .into()
    }

    /// Creates a `combo` security scheme where all referenced schemes must satisfy access.
    pub fn combo_all_of<I, S>(schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        ComboSecurityScheme {
            _context: SecuritySchemeContext::new("combo"),
            one_of: Vec::new(),
            all_of: schemes.into_iter().map(Into::into).collect(),
        }
        .into()
    }

    /// Creates an `oauth2` security scheme with an explicit flow.
    pub fn oauth2(flow: impl Into<String>) -> Self {
        OAuth2SecurityScheme {
            _context: SecuritySchemeContext::new("oauth2"),
            authorization: None,
            token: None,
            refresh: None,
            scopes: None,
            flow: flow.into(),
        }
        .into()
    }

    /// Creates an OAuth2 authorization-code flow security scheme.
    pub fn oauth2_code(
        authorization: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, ValidateError> {
        Ok(OAuth2SecurityScheme::builder("code")
            .authorization(authorization)
            .token(token)
            .build()?
            .into())
    }

    /// Creates an OAuth2 client credentials flow security scheme.
    pub fn oauth2_client() -> Self {
        Self::oauth2("client")
    }

    /// Creates an OAuth2 device flow security scheme.
    pub fn oauth2_device() -> Self {
        Self::oauth2("device")
    }

    pub fn scheme(&self) -> &str {
        macro_rules! get_scheme {
            ($($variant:ident),*) => {
                match self {
                    $(Self::$variant(s) => s._context.scheme.as_ref(),)*
                }
            };
        }

        get_scheme!(
            NoSec, Auto, Combo, Basic, Digest, APIKey, Bearer, PSK, OAuth2
        )
    }

    pub(crate) fn validate_references(
        &self,
        context: &str,
        security_definitions: &BTreeMap<String, SecurityScheme>,
    ) -> Result<(), ValidateError> {
        if self.scheme() != "combo" {
            return Ok(());
        }

        validate_combo_references(
            format!("{}.oneOf", context).as_str(),
            &self.one_of_references(),
            security_definitions,
        )?;
        validate_combo_references(
            format!("{}.allOf", context).as_str(),
            &self.all_of_references(),
            security_definitions,
        )
    }

    fn context(&self) -> &SecuritySchemeContext {
        match self {
            Self::NoSec(scheme) => &scheme._context,
            Self::Auto(scheme) => &scheme._context,
            Self::Combo(scheme) => &scheme._context,
            Self::Basic(scheme) => &scheme._context,
            Self::Digest(scheme) => &scheme._context,
            Self::APIKey(scheme) => &scheme._context,
            Self::Bearer(scheme) => &scheme._context,
            Self::PSK(scheme) => &scheme._context,
            Self::OAuth2(scheme) => &scheme._context,
        }
    }

    fn string_field(&self, name: &str) -> Option<&str> {
        self.context()
            ._extra_fields
            .get(name)
            .and_then(serde_json::Value::as_str)
    }

    fn one_of_references(&self) -> Cow<'_, [String]> {
        match self {
            Self::Combo(scheme) => Cow::Borrowed(&scheme.one_of),
            _ => Cow::Owned(string_array_field(
                self.context()._extra_fields.get("oneOf"),
            )),
        }
    }

    fn all_of_references(&self) -> Cow<'_, [String]> {
        match self {
            Self::Combo(scheme) => Cow::Borrowed(&scheme.all_of),
            _ => Cow::Owned(string_array_field(
                self.context()._extra_fields.get("allOf"),
            )),
        }
    }

    fn apikey_name(&self) -> Option<&str> {
        match self {
            Self::APIKey(scheme) => scheme.name.as_deref(),
            _ => self.string_field("name"),
        }
    }

    fn oauth2_flow(&self) -> Option<&str> {
        match self {
            Self::OAuth2(scheme) => Some(scheme.flow.as_str()),
            _ => self.string_field("flow"),
        }
    }

    fn oauth2_has_endpoint(&self, name: &str) -> bool {
        match (self, name) {
            (Self::OAuth2(scheme), "authorization") => scheme.authorization.is_some(),
            (Self::OAuth2(scheme), "token") => scheme.token.is_some(),
            _ => self
                .string_field(name)
                .is_some_and(|value| !value.is_empty()),
        }
    }
}

impl Validate for SecurityScheme {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        match self.scheme() {
            "combo" => {
                let one_of = self.one_of_references();
                let all_of = self.all_of_references();
                if one_of.is_empty() && all_of.is_empty() {
                    return Err(invalid_security(
                        "combo schemes must define at least one of oneOf or allOf",
                    ));
                }
                validate_combo_members("oneOf", &one_of)?;
                validate_combo_members("allOf", &all_of)?;
            }
            "apikey" => {
                if self.apikey_name().unwrap_or("").is_empty() {
                    return Err(ValidateError::MissingRequiredField("name".to_string()));
                }
            }
            "oauth2" => validate_oauth2_scheme(self)?,
            "nosec" | "auto" | "basic" | "digest" | "bearer" | "psk" => {}
            scheme => return Err(invalid_security(format!("unsupported scheme '{}'", scheme))),
        }

        Ok(())
    }
}

fn validate_combo_members(context: &str, references: &[String]) -> Result<(), ValidateError> {
    if !references.is_empty() && references.len() < 2 {
        return Err(invalid_security(format!(
            "{} must contain at least two references",
            context
        )));
    }

    for reference in references {
        if reference.is_empty() {
            return Err(invalid_security(format!(
                "{} must not contain empty references",
                context
            )));
        }
    }

    Ok(())
}

fn validate_combo_references(
    context: &str,
    references: &[String],
    security_definitions: &BTreeMap<String, SecurityScheme>,
) -> Result<(), ValidateError> {
    for reference in references {
        if !security_definitions.contains_key(reference) {
            return Err(ValidateError::InvalidReference {
                context: context.to_string(),
                reference: reference.clone(),
            });
        }
    }

    Ok(())
}

fn validate_oauth2_scheme(scheme: &SecurityScheme) -> Result<(), ValidateError> {
    match scheme.oauth2_flow().unwrap_or("") {
        "code" => {
            if !scheme.oauth2_has_endpoint("authorization") {
                return Err(ValidateError::MissingRequiredField(
                    "authorization".to_string(),
                ));
            }
            if !scheme.oauth2_has_endpoint("token") {
                return Err(ValidateError::MissingRequiredField("token".to_string()));
            }
        }
        "client" | "device" => {}
        flow => {
            return Err(invalid_security(format!(
                "unsupported OAuth2 flow '{}'",
                flow
            )));
        }
    }

    Ok(())
}

fn invalid_security(message: impl Into<String>) -> ValidateError {
    ValidateError::InvalidSecurity(message.into())
}

fn string_array_field(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect(),
        Some(serde_json::Value::String(value)) => alloc::vec![value.clone()],
        _ => Vec::new(),
    }
}
