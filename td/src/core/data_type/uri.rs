//! URI reference, form target, and base-URI types used across TD/TM documents.
//!
//! These types wrap [`fluent_uri`] so that parsed representations can be cached
//! and reused on the per-request resolution hot path of protocol bindings.

use alloc::{borrow::ToOwned, string::{String, ToString}};
use core::fmt;

use fluent_uri::{
    ParseError, Uri, UriRef,
    resolve::ResolveError,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    /// A URI-reference compliant with RFC 3986.
    Reference(UriReference),
    /// A URI Template compliant with RFC 6570 containing placeholders.
    Template(String),
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
