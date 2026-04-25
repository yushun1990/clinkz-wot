use alloc::{string::{String, ToString}, vec::Vec};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::data_type::{AnyUri, DefaultExt, MultiLanguage};

#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct SecuritySchemeContext<Ext> {
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

    #[serde(flatten)]
    pub _extra_fields: Ext,
}

impl <Ext> SecuritySchemeContext<Ext>
where
    Ext: Default
{
    pub fn new(scheme: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            ..Default::default()
        }
    }
}

pub trait ContextHelper: Sized {
    type Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext>;

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

    fn extra_fields(mut self, extra_fields: impl Into<Self::Ext>) -> Self {
        self.context()._extra_fields =  extra_fields.into();
        self
    }
}


/// A security configuration corresponding to identified by the
/// Vocabulary Term `nosec`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NoSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,
}

impl <Ext> NoSecurityScheme<Ext>
where
    Ext: Default
{
    pub fn builder() -> NoSecuritySchemeBuilder<Ext> {
        NoSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `NoSecurityScheme` instances.
pub struct NoSecuritySchemeBuilder<Ext> {
    scheme: NoSecurityScheme<Ext>,
}

impl <Ext> NoSecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `NoSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: NoSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("nosec"),
            },
        }
    }

    /// Builds and returns the `NoSecurityScheme` instance.
    pub fn build(self) -> NoSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for NoSecuritySchemeBuilder<Ext> {
    type Ext = Ext;
    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `auto`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AutoSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,
}

impl <Ext> AutoSecurityScheme<Ext>
where
    Ext: Default {
    pub fn builder() -> AutoSecuritySchemeBuilder<Ext> {
        AutoSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `AutoSecurityScheme` instances.
pub struct AutoSecuritySchemeBuilder<Ext> {
    scheme: AutoSecurityScheme<Ext>,
}

impl <Ext> AutoSecuritySchemeBuilder<Ext>
where
    Ext: Default {
    /// Creates a new `AutoSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: AutoSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("auto"),
            },
        }
    }

    /// Builds and returns the `AutoSecurityScheme` instance.
    pub fn build(self) -> AutoSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for AutoSecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `combo`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ComboSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

    /// Array of two or more strings identifying other named security
    /// scheme definitions, any one of which, when satisfied, will
    /// allow access.
    pub one_of: Vec<String>,

    /// Array of two or more strings identifying other named security
    /// scheme definitions, all of which must be satisfied for access.
    pub all_of: Vec<String>,
}

impl <Ext> ComboSecurityScheme<Ext>
where
    Ext: Default {
    pub fn builder() -> ComboSecuritySchemeBuilder<Ext> {
        ComboSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `ComboSecurityScheme` instances.
pub struct ComboSecuritySchemeBuilder<Ext> {
    scheme: ComboSecurityScheme<Ext>,
}

impl <Ext> ComboSecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `ComboSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: ComboSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("combo"),
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
    pub fn build(self) -> ComboSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for ComboSecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
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

fn is_default_location(location: &SecurityLocation) -> bool {
    location == &SecurityLocation::Header
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `basic`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BasicSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    #[serde(
        default,
        rename = "in",
        skip_serializing_if = "is_default_location"
    )]
    pub location: SecurityLocation,
}

impl <Ext> BasicSecurityScheme<Ext>
where
    Ext: Default
{
    pub fn builder() -> BasicSecuritySchemeBuilder<Ext> {
        BasicSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `BasicSecurityScheme` instances.
pub struct BasicSecuritySchemeBuilder<Ext> {
    scheme: BasicSecurityScheme<Ext>,
}

impl <Ext> BasicSecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `BasicSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: BasicSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("basic"),
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
    pub fn build(self) -> BasicSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for BasicSecuritySchemeBuilder<Ext> {
    type Ext = Ext;
    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
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

fn is_default_qop(qop: &Qop) -> bool {
    qop == &Qop::Auth
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `digest`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct DigestSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    #[serde(
        default,
        rename = "in",
        skip_serializing_if = "is_default_location"
    )]
    pub location: SecurityLocation,

    /// Quality of protection.
    #[serde(default, skip_serializing_if = "is_default_qop")]
    pub qop: Qop
}

impl <Ext> DigestSecurityScheme<Ext>
where
    Ext: Default
{
    pub fn builder() -> DigestSecuritySchemeBuilder<Ext> {
        DigestSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `DigestSecurityScheme` instances.
pub struct DigestSecuritySchemeBuilder<Ext> {
    scheme: DigestSecurityScheme<Ext>,
}

impl <Ext> DigestSecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `DigestSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: DigestSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("digest"),
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
    pub fn build(self) -> DigestSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for DigestSecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `apikey`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct APIKeySecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Specifies the location of security authentication information.
    #[serde(
        default,
        rename = "in",
        skip_serializing_if = "is_default_location"
    )]
    pub location: SecurityLocation,
}

impl <Ext> APIKeySecurityScheme<Ext>
where
    Ext: Default
{
    pub fn builder() -> APIKeySecuritySchemeBuilder<Ext> {
        APIKeySecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `APIKeySecurityScheme` instances.
pub struct APIKeySecuritySchemeBuilder<Ext> {
    scheme: APIKeySecurityScheme<Ext>,
}

impl <Ext> APIKeySecuritySchemeBuilder<Ext>
where
    Ext: Default {
    /// Creates a new `APIKeySecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: APIKeySecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("apikey"),
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
    pub fn build(self) -> APIKeySecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for APIKeySecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
        &mut self.scheme._context
    }
}

/// Helper function to provide the default algorithm "ES256"
fn default_alg() -> String {
    "ES256".to_string()
}

fn is_default_alg(alg: &String) -> bool {
    alg == &default_alg()
}

/// Helper function to provide the default format "jwt"
fn default_format() -> String {
    "jwt".to_string()
}

fn is_default_format(format: &String) -> bool {
    format == &default_format()
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `bearer`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BearerSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

    /// URI of the authorization server.
    pub authorization: Option<AnyUri>,

    /// Name for query, header, cookie, or uri parameters.
    pub name: Option<String>,

    /// Encoding, encryption, or digest algorithm.
    #[serde(default = "default_alg", skip_serializing_if = "is_default_alg")]
    pub alg: String,

    /// Specifies format of security authentication information.
    #[serde(default = "default_format", skip_serializing_if = "is_default_format")]
    pub format: String,

    /// Specifies the location of security authentication information.
    #[serde(default, rename = "in", skip_serializing_if = "is_default_location")]
    pub location: SecurityLocation,
}

impl <Ext> BearerSecurityScheme<Ext>
where
    Ext: Default
{
    pub fn builder() -> BearerSecuritySchemeBuilder<Ext> {
        BearerSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `BearerSecurityScheme` instances.
pub struct BearerSecuritySchemeBuilder<Ext> {
    scheme: BearerSecurityScheme<Ext>,
}

impl <Ext> BearerSecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `BearerSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: BearerSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("bearer"),
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
    pub fn build(self) -> BearerSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for BearerSecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `psk`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PSKSecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

    /// Identifier providing information which can be used for
    /// selection or confirmation.
    pub identity: Option<String>,
}

impl <Ext> PSKSecurityScheme<Ext>
where
   Ext: Default
{
    pub fn builder() -> PSKSecuritySchemeBuilder<Ext> {
        PSKSecuritySchemeBuilder::<Ext>::new()
    }
}

/// Builder for creating `PSKSecurityScheme` instances.
pub struct PSKSecuritySchemeBuilder<Ext> {
    scheme: PSKSecurityScheme<Ext>,
}

impl<Ext> PSKSecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `PSKSecuritySchemeBuilder`.
    pub fn new() -> Self {
        Self {
            scheme: PSKSecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("psk"),
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
    pub fn build(self) -> PSKSecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for PSKSecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Ext> {
        &mut self.scheme._context
    }
}

/// A security configuration corresponding to identified by the
/// Vocabulary Term `oauth2`.
#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct OAuth2SecurityScheme<Ext=DefaultExt> {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext<Ext>,

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

impl <Ext> OAuth2SecurityScheme<Ext>
where
    Ext: Default
{
    pub fn builder(flow: impl Into<String>) -> OAuth2SecuritySchemeBuilder<Ext> {
        OAuth2SecuritySchemeBuilder::<Ext>::new(flow)
    }
}

/// Builder for creating `OAuth2SecurityScheme` instances.
pub struct OAuth2SecuritySchemeBuilder<Ext> {
    scheme: OAuth2SecurityScheme<Ext>,
}

impl <Ext> OAuth2SecuritySchemeBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `OAuth2SecuritySchemeBuilder` with the required `flow` field.
    pub fn new(flow: impl Into<String>) -> Self {
        Self {
            scheme: OAuth2SecurityScheme {
                _context: SecuritySchemeContext::<Ext>::new("oauth2"),
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
    pub fn build(self) -> OAuth2SecurityScheme<Ext> {
        self.scheme
    }
}

impl <Ext> ContextHelper for OAuth2SecuritySchemeBuilder<Ext> {
    type Ext = Ext;

    fn context(&mut self) -> &mut SecuritySchemeContext<Self::Ext> {
        &mut self.scheme._context
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecurityScheme<Ext=DefaultExt> {
    NoSec(NoSecurityScheme<Ext>),
    Auto(AutoSecurityScheme<Ext>),
    Combo(ComboSecurityScheme<Ext>),
    Basic(BasicSecurityScheme<Ext>),
    Digest(DigestSecurityScheme<Ext>),
    APIKey(APIKeySecurityScheme<Ext>),
    Bearer(BearerSecurityScheme<Ext>),
    PSK(PSKSecurityScheme<Ext>),
    OAuth2(OAuth2SecurityScheme<Ext>),
}

impl <Ext> SecurityScheme<Ext>
where
    Ext: Default {

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
