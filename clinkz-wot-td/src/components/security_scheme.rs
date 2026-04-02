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
    pub schema: String,
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

/// A security configuration corresponding to identified by the
/// Vocabulary Term `auto`.
#[skip_serializing_none]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AutoSecurityScheme {
    #[serde(flatten)]
    pub _context: SecuritySchemeContext,
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
