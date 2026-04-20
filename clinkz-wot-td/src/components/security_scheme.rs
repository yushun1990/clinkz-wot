use alloc::{string::{String, ToString}, vec::Vec};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::data_type::{AnyUri, MultiLanguage};

#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct SecuritySchemeContext {
    /// JSON-LD keyword to label the object with semantic tags.
    #[serde(rename = "@type")]
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub tags: Option<Vec<String>>,

    /// Provides additional (human-readable) information based on a
    /// default language.
    pub description: Option<String>,

    /// Multi-language descriptions.
    pub descriptions: Option<MultiLanguage>,

    /// URI of the proxy server this security configuration provides
    /// access to.
    pub proxy: Option<AnyUri>,

    /// Identification of the security mechanism being configured.
    pub scheme: String,
}

impl SecuritySchemeContext {
    pub fn new(scheme: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            ..Default::default()
        }
    }
}

pub trait ContextHelper: Sized {
    fn context(&mut self) -> &mut SecuritySchemeContext;

    /// Adds tags.
    fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String>
    {
        let mut items: Vec<String> = tags.into_iter().map(|s| s.into()).collect();
        self.context().tags.get_or_insert_with(Vec::new).append(&mut items);
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
        let descriptions = self.context().descriptions.get_or_insert_with(MultiLanguage::new);
        descriptions.add(lang, description);
        self
    }

    /// Sets the proxy URI.
    fn proxy(mut self, proxy: impl Into<String>) -> Self {
        match AnyUri::parse(proxy.into().as_str()) {
            Ok(uri) => self.context().proxy = Some(uri),
            Err(_) => {},
        }
        self
    }

}


/// A security configuration corresponding to identified by the
/// Vocabulary Term `nosec`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NoSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,
}

impl NoSecurityScheme {
    pub fn builder() -> NoSecuritySchemeBuilder {
        NoSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `NoSecurityScheme` instances.
pub struct NoSecuritySchemeBuilder {
    scheme: NoSecurityScheme,
}

impl NoSecuritySchemeBuilder {
    /// Creates a new `NoSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: NoSecurityScheme {
                _context: SecuritySchemeContext::new("nosec"),
            },
        }
    }

    /// Builds and returns the `NoSecurityScheme` instance.
    pub fn build(self) -> NoSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for NoSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `auto`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AutoSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,
}

impl AutoSecurityScheme {
    pub fn builder() -> AutoSecuritySchemeBuilder {
        AutoSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `AutoSecurityScheme` instances.
pub struct AutoSecuritySchemeBuilder {
    scheme: AutoSecurityScheme,
}

impl AutoSecuritySchemeBuilder {
    /// Creates a new `AutoSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: AutoSecurityScheme {
                _context: SecuritySchemeContext::new("auto"),
            },
        }
    }

    /// Builds and returns the `AutoSecurityScheme` instance.
    pub fn build(self) -> AutoSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for AutoSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `combo`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ComboSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// Array of two or more strings identifying other named security
    /// scheme definitions, any one of which, when satisfied, will
    /// allow access.
    pub one_of: Vec<String>,

    /// Array of two or more strings identifying other named security
    /// scheme definitions, all of which must be satisfied for access.
    pub all_of: Vec<String>,
}

impl ComboSecurityScheme {
    pub fn builder() -> ComboSecuritySchemeBuilder {
        ComboSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `ComboSecurityScheme` instances.
pub struct ComboSecuritySchemeBuilder {
    scheme: ComboSecurityScheme,
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
        }
    }

    /// Adds security schemes to one_of.
    pub fn one_of<I, S>(mut self, schemes: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String>
    {
        self.scheme.one_of.extend(schemes.into_iter().map(|s| s.into()));
        self
    }

    /// Adds security schemes to all_of.
    pub fn all_of<I, S>(mut self, schemes: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String>
    {
        self.scheme.all_of.extend(schemes.into_iter().map(|s| s.into()));
        self
    }

    /// Builds and returns the `ComboSecurityScheme` instance.
    pub fn build(self) -> ComboSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for ComboSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLocation {
    Header,
    Query,
    Body,
    Cookie,
    Auto,
}

impl Default for SecurityLocation {
    fn default() -> Self {
        SecurityLocation::Header
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `basic`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BasicSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    #[serde(default, rename = "in")]
    pub location: SecurityLocation,
}

impl BasicSecurityScheme {
    pub fn builder() -> BasicSecuritySchemeBuilder {
        BasicSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `BasicSecurityScheme` instances.
pub struct BasicSecuritySchemeBuilder {
    scheme: BasicSecurityScheme,
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
    pub fn build(self) -> BasicSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for BasicSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Qop {
    /// Authentication only
    Auth,
    /// Authentication with integrity protection
    AuthInt,
}

impl Default for Qop {
    fn default() -> Self {
        Self::Auth
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `digest`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct DigestSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    #[serde(default, rename = "in")]
    pub location: SecurityLocation,

    /// Quality of protection.
    #[serde(default)]
    pub qop: Qop
}

impl DigestSecurityScheme {
    pub fn builder() -> DigestSecuritySchemeBuilder {
        DigestSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `DigestSecurityScheme` instances.
pub struct DigestSecuritySchemeBuilder {
    scheme: DigestSecurityScheme,
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
    pub fn build(self) -> DigestSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for DigestSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `apikey`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct APIKeySecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    #[serde(default, rename = "in")]
    pub location: SecurityLocation,
}

impl APIKeySecurityScheme {
    pub fn builder() -> APIKeySecuritySchemeBuilder {
        APIKeySecuritySchemeBuilder::new()
    }
}

/// Builder for creating `APIKeySecurityScheme` instances.
pub struct APIKeySecuritySchemeBuilder {
    scheme: APIKeySecurityScheme,
}

impl APIKeySecuritySchemeBuilder {
    /// Creates a new `APIKeySecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: APIKeySecurityScheme {
                _context: SecuritySchemeContext::new("apikey"),
                name: None,
                location: SecurityLocation::default(),
            },
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
    pub fn build(self) -> APIKeySecurityScheme {
        self.scheme
    }
}

impl ContextHelper for APIKeySecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

/// Helper function to provide the default algorithm "ES256"
fn default_alg() -> String {
    "ES256".to_string()
}

/// Helper function to provide the default format "jwt"
fn default_format() -> String {
    "jwt".to_string()
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `bearer`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BearerSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// URI of the authorization server.
    pub authorization: Option<AnyUri>,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Encoding, encryption, or digest algorithm.
    #[serde(default = "default_alg")]
    pub alg: String,

    /// Specifies format of security authentication information.
    #[serde(default = "default_format")]
    pub format: String,

    /// Specifies the location of security authentication information.
    #[serde(default, rename = "in")]
    pub location: SecurityLocation,
}

impl BearerSecurityScheme {
    pub fn builder() -> BearerSecuritySchemeBuilder {
        BearerSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `BearerSecurityScheme` instances.
pub struct BearerSecuritySchemeBuilder {
    scheme: BearerSecurityScheme,
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
        }
    }

    /// Sets the authorization URI.
    pub fn authorization(mut self, authorization: impl Into<String>) -> Self {
        match AnyUri::parse(authorization.into().as_str()) {
            Ok(uri) => self.scheme.authorization = Some(uri),
            Err(_) => {},
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
    pub fn build(self) -> BearerSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for BearerSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `psk`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PSKSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// Identifier providing information which can be used for
    /// selection or confirmation.
    pub identity: Option<String>,
}

impl PSKSecurityScheme {
    pub fn builder() -> PSKSecuritySchemeBuilder {
        PSKSecuritySchemeBuilder::new()
    }
}

/// Builder for creating `PSKSecurityScheme` instances.
pub struct PSKSecuritySchemeBuilder {
    scheme: PSKSecurityScheme,
}

impl PSKSecuritySchemeBuilder {
    /// Creates a new `PSKSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: PSKSecurityScheme {
                _context: SecuritySchemeContext::new("psk"),
                identity: None,
            },
        }
    }

    /// Sets the identity.
    pub fn identity(mut self, identity: impl Into<String>) -> Self {
        self.scheme.identity = Some(identity.into());
        self
    }

    /// Builds and returns the `PSKSecurityScheme` instance.
    pub fn build(self) -> PSKSecurityScheme {
        self.scheme
    }
}

impl ContextHelper for PSKSecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `oauth2`.
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct OAuth2SecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,

    /// URI of the authorization server.
    pub authorization: Option<AnyUri>,

    /// URI of the token server.
    pub token: Option<AnyUri>,

    /// URI of the refresh server.
    pub refresh: Option<AnyUri>,

    /// Set of authorization scope identifier provided as an array.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub scopes: Option<Vec<String>>,

    /// Authorization flow, e.g., code, client.
    pub flow: String,
}

impl OAuth2SecurityScheme {
    pub fn builder(flow: impl Into<String>) -> OAuth2SecuritySchemeBuilder {
        OAuth2SecuritySchemeBuilder::new(flow)
    }
}

/// Builder for creating `OAuth2SecurityScheme` instances.
pub struct OAuth2SecuritySchemeBuilder {
    scheme: OAuth2SecurityScheme,
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
        }
    }

    /// Sets the authorization URI.
    pub fn authorization(mut self, authorization: impl Into<String>) -> Self {
        match AnyUri::parse(authorization.into().as_str()) {
            Ok(uri) => self.scheme.authorization = Some(uri),
            Err(_) => {},
        }
        self
    }

    /// Sets the token URI.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        match AnyUri::parse(token.into().as_str()) {
            Ok(uri) => self.scheme.token = Some(uri),
            Err(_) => {},
        }
        self
    }

    /// Sets the refresh URI.
    pub fn refresh(mut self, refresh: impl Into<String>) -> Self {
        match AnyUri::parse(refresh.into().as_str()) {
            Ok(uri) => self.scheme.refresh = Some(uri),
            Err(_) => {},
        }
        self
    }

    /// Adds scopes.
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String> {
        let mut items: Vec<String> = scopes.into_iter().map(|s| s.into()).collect();
        self.scheme.scopes.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Builds and returns the `OAuth2SecurityScheme` instance.
    pub fn build(self) -> OAuth2SecurityScheme {
        self.scheme
    }
}

impl ContextHelper for OAuth2SecuritySchemeBuilder {
    fn context(&mut self) -> &mut SecuritySchemeContext {
        &mut self.scheme._context
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
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

impl SecurityScheme {
    pub fn scheme(&self) -> &str {
        macro_rules! get_scheme {
            ($($variant:ident),*) => {
                match self {
                    $(Self::$variant(s) => s._context.scheme.as_ref(),)*
                }
            };
        }

        get_scheme!(NoSec, Auto, Combo, Basic, Digest, APIKey, Bearer, PSK, OAuth2)
    }
}
