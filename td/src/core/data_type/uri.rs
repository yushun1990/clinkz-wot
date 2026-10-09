//! URI reference, form target, and base-URI types used across TD/TM documents.
//!
//! These types wrap [`fluent_uri`] so that parsed representations can be cached
//! and reused on the per-request resolution hot path of protocol bindings.

use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
};
use core::fmt;

use fluent_uri::{ParseError, Uri, UriRef, resolve::ResolveError};
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

#[cfg(feature = "std")]
impl std::error::Error for ResolveFormHrefError {}

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
    let target = lending::Target::new(base, href).map_err(|error| match error {
        lending::Error::TemplateBase => {
            ResolveFormHrefError::TemplateBase(base.unwrap().as_str().to_owned())
        }
        lending::Error::Resolve(error) => error.into(),
    })?;
    let mut resolver = match target {
        lending::Target::Alias(_) => {
            return Ok(match href {
                FormHref::Template(value) => ResolvedFormHref::Template(value.clone()),
                FormHref::Reference(value) => ResolvedFormHref::Reference(value.clone()),
            });
        }
        lending::Target::Resolve(resolver) => resolver,
    };
    let mut bytes = alloc::vec::Vec::new();
    while !resolver.tick(&mut bytes) {}
    // The synchronous adapter owns parsing/allocation. Bounded lending uses
    // the program's paid ASCII validation and never invokes this parser.
    let text = String::from_utf8(bytes).expect("URI program emits ASCII");
    Ok(ResolvedFormHref::Reference(
        UriReference::parse(&text).expect("valid resolved URI"),
    ))
}

/// One production URI meaning, driven either synchronously above or under the
/// proof's work/account owner. All source handles refer to immutable caller
/// storage; every transition reads/writes at most sixteen bytes. Cached public
/// fluent-uri component access is fixed work, never a reparsing shortcut.
pub(crate) mod lending {
    use super::*;

    pub(crate) enum Error {
        TemplateBase,
        Resolve(ResolveError),
    }
    pub(crate) enum Target<'a> {
        Alias(&'a str),
        Resolve(Resolution<'a>),
    }
    pub(crate) trait Output {
        fn len(&self) -> usize;
        fn byte(&self, index: usize) -> u8;
        fn push(&mut self, byte: u8);
        fn set(&mut self, index: usize, byte: u8);
        fn truncate(&mut self, len: usize);
    }
    impl Output for alloc::vec::Vec<u8> {
        fn len(&self) -> usize {
            self.len()
        }
        fn byte(&self, index: usize) -> u8 {
            self[index]
        }
        fn push(&mut self, byte: u8) {
            self.push(byte);
        }
        fn set(&mut self, index: usize, byte: u8) {
            self[index] = byte;
        }
        fn truncate(&mut self, len: usize) {
            self.truncate(len);
        }
    }
    #[derive(Clone, Copy)]
    enum State {
        Merge,
        MergeClassify,
        Configure,
        Prefix,
        Segment,
        Classify,
        Emit,
        Pop,
        Fix,
        Shift,
        Insert,
        Tail,
        Validate,
        Done,
    }
    pub(crate) struct Resolution<'a> {
        prefix: [&'a str; 4],
        paths: [&'a str; 2],
        tail: [&'a str; 4],
        state: State,
        part: usize,
        pos: usize,
        start: usize,
        path_start: usize,
        normalize: bool,
        authority: bool,
        merge: usize,
        validate: usize,
        streaming: bool,
    }
    impl<'a> Target<'a> {
        pub(crate) fn new(base: Option<&'a BaseUri>, href: &'a FormHref) -> Result<Self, Error> {
            let FormHref::Reference(reference) = href else {
                return Ok(Self::Alias(href.as_str()));
            };
            let r = &reference.0;
            if r.has_scheme() || base.is_none() {
                return Ok(Self::Alias(href.as_str()));
            }
            let base = match base.unwrap() {
                BaseUri::Absolute(base) => base.as_uri(),
                BaseUri::Template(_) => return Err(Error::TemplateBase),
            };
            if base.has_fragment() {
                return Err(Error::Resolve(ResolveError::BaseWithFragment));
            }
            if !base.has_authority()
                && base.path().is_rootless()
                && !matches!(r.as_str().as_bytes().first(), None | Some(b'#'))
            {
                return Err(Error::Resolve(
                    ResolveError::InvalidReferenceAgainstOpaqueBase,
                ));
            }
            let authority = r.authority().or_else(|| base.authority());
            let mut paths = [r.path().as_str(), ""];
            let mut query = r.query();
            let mut state = State::Configure;
            if !r.has_authority() {
                if r.path().is_empty() {
                    paths[0] = base.path().as_str();
                    query = query.or_else(|| base.query());
                } else if !r.path().is_absolute() {
                    paths = [
                        if base.path().is_empty() {
                            "/"
                        } else {
                            base.path().as_str()
                        },
                        r.path().as_str(),
                    ];
                    state = State::Merge;
                }
            }
            Ok(Self::Resolve(Resolution {
                prefix: [
                    base.scheme().as_str(),
                    ":",
                    if authority.is_some() { "//" } else { "" },
                    authority.map_or("", |a| a.as_str()),
                ],
                paths,
                tail: [
                    if query.is_some() { "?" } else { "" },
                    query.map_or("", |q| q.as_str()),
                    if r.has_fragment() { "#" } else { "" },
                    r.fragment().map_or("", |f| f.as_str()),
                ],
                state,
                part: 0,
                pos: 0,
                start: 0,
                path_start: 0,
                normalize: false,
                authority: authority.is_some(),
                merge: paths[0].len(),
                validate: 0,
                streaming: false,
            }))
        }
    }
    // Percent-encoded dots follow the resolved fluent-uri behavior, including
    // mixed literal/encoded spellings. At most six bytes are inspected.
    fn dots(mut segment: &[u8]) -> u8 {
        let mut count = 0;
        while !segment.is_empty() && count < 2 {
            if segment[0] == b'.' {
                segment = &segment[1..];
            } else if segment.len() >= 3
                && segment[0] == b'%'
                && segment[1] == b'2'
                && matches!(segment[2], b'e' | b'E')
            {
                segment = &segment[3..];
            } else {
                return 0;
            }
            count += 1;
        }
        if segment.is_empty() { count } else { 0 }
    }
    impl Resolution<'_> {
        /// Conservative per-transition UriBytes envelope, including source and
        /// destination accesses. No text-length-dependent atomic action.
        #[cfg(feature = "validated-thing")]
        pub(crate) const WORK: u64 = 16;
        #[cfg(feature = "validated-thing")]
        pub(crate) fn work(&self) -> u64 {
            match self.state {
                State::MergeClassify | State::Classify => Self::WORK,
                State::Fix => 4,
                State::Insert => 2,
                _ => 1,
            }
        }
        pub(crate) fn tick(&mut self, output: &mut impl Output) -> bool {
            match self.state {
                State::Merge => {
                    if self.merge > 0 && self.paths[0].as_bytes()[self.merge - 1] != b'/' {
                        self.merge -= 1;
                    } else {
                        self.state = State::MergeClassify;
                    }
                }
                State::MergeClassify => {
                    let suffix = &self.paths[0].as_bytes()[self.merge..];
                    if dots(suffix) != 2 {
                        self.paths[0] = &self.paths[0][..self.merge];
                    }
                    self.state = State::Configure;
                }
                State::Configure => {
                    self.normalize = self.paths[0].starts_with('/');
                    self.state = State::Prefix;
                }
                State::Prefix | State::Tail => {
                    let tail = matches!(self.state, State::Tail);
                    let text = if tail {
                        self.tail[self.part]
                    } else {
                        self.prefix[self.part]
                    };
                    if self.pos < text.len() {
                        output.push(text.as_bytes()[self.pos]);
                        self.pos += 1;
                    } else {
                        self.part += 1;
                        self.pos = 0;
                        if self.part == 4 {
                            self.part = 0;
                            if tail {
                                self.validate = 0;
                                self.state = State::Validate;
                            } else {
                                self.path_start = output.len();
                                self.state = State::Segment;
                            }
                        }
                    }
                }
                State::Segment => {
                    let path = self.paths[self.part];
                    // A dot segment has at most six bytes. Once seven bytes
                    // have been inspected, stream the rest directly: a long
                    // ordinary segment needs no complete scan before copying.
                    if self.pos - self.start == 7 {
                        self.streaming = true;
                        self.merge = self.start;
                        self.state = State::Emit;
                    } else if self.pos < path.len() && path.as_bytes()[self.pos] != b'/' {
                        self.pos += 1;
                    } else if self.pos > self.start || self.pos < path.len() {
                        self.state = State::Classify;
                    } else {
                        self.part += 1;
                        self.pos = 0;
                        self.start = 0;
                        if self.part == 2 {
                            self.state = State::Fix;
                        }
                    }
                }
                State::Classify => {
                    let path = self.paths[self.part];
                    let end = self.pos;
                    if self.pos < path.len() {
                        self.pos += 1;
                    }
                    let kind = if self.normalize {
                        dots(&path.as_bytes()[self.start..end])
                    } else {
                        0
                    };
                    match kind {
                        1 => self.start = self.pos,
                        2 if output.len() > self.path_start + 1 => {
                            self.state = State::Pop;
                            self.validate = output.len() - 1;
                        }
                        2 => self.start = self.pos,
                        _ => {
                            self.merge = self.start;
                            self.state = State::Emit;
                        }
                    }
                    if kind != 0 && !matches!(self.state, State::Pop) {
                        self.state = State::Segment;
                    }
                }
                State::Emit => {
                    if self.merge < self.pos {
                        output.push(self.paths[self.part].as_bytes()[self.merge]);
                        self.merge += 1;
                    } else if self.streaming && self.pos < self.paths[self.part].len() {
                        let byte = self.paths[self.part].as_bytes()[self.pos];
                        output.push(byte);
                        self.pos += 1;
                        self.merge = self.pos;
                        if byte == b'/' {
                            self.streaming = false;
                            self.start = self.pos;
                            self.state = State::Segment;
                        }
                    } else {
                        self.streaming = false;
                        self.start = self.pos;
                        self.state = State::Segment;
                    }
                }
                State::Pop => {
                    self.validate -= 1;
                    if output.byte(self.validate) == b'/' {
                        output.truncate(self.validate + 1);
                        self.start = self.pos;
                        self.state = State::Segment;
                    }
                }
                State::Fix => {
                    if !self.authority
                        && output.len() >= self.path_start + 2
                        && output.byte(self.path_start) == b'/'
                        && output.byte(self.path_start + 1) == b'/'
                    {
                        self.merge = output.len();
                        output.push(0);
                        output.push(0);
                        self.state = State::Shift;
                    } else {
                        self.part = 0;
                        self.pos = 0;
                        self.state = State::Tail;
                    }
                }
                State::Shift => {
                    if self.merge > self.path_start {
                        self.merge -= 1;
                        let byte = output.byte(self.merge);
                        output.set(self.merge + 2, byte);
                    } else {
                        self.state = State::Insert;
                    }
                }
                State::Insert => {
                    output.set(self.path_start, b'/');
                    output.set(self.path_start + 1, b'.');
                    self.part = 0;
                    self.pos = 0;
                    self.state = State::Tail;
                }
                State::Validate => {
                    if self.validate < output.len() {
                        // Parsed URI (not IRI) components and punctuation are
                        // ASCII. This charged pass certifies the fresh buffer.
                        assert!(output.byte(self.validate).is_ascii());
                        self.validate += 1;
                    } else {
                        self.state = State::Done;
                    }
                }
                State::Done => return true,
            }
            matches!(self.state, State::Done)
        }
    }
}
