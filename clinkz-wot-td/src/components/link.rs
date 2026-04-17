use alloc::{vec::Vec, string::String, borrow::Cow};
use fluent_uri::ParseError;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::data_type::AnyUri;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Target IRI of the link.
    pub href: AnyUri,

    /// Target media type of the link.
    #[serde(rename = "type")]
    pub content_type: Option<String>,

    /// Relation type between the current Thing and the target resource.
    /// Common values: "service-doc", "item", "parent", "collection".
    pub rel: Option<String>,

    /// The anchor should be used as the context of the link.
    pub anchor: Option<AnyUri>,

    /// Target attributes that specifies one or more sizes for the
    /// referenced icon.
    pub sizes: Option<String>,

    /// Language of the target resource (BCP47).
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub hreflang: Option<Vec<String>>,
}

impl Link {
    pub fn builder(href: &str) -> LinkBuilder<'_> {
        LinkBuilder::new(href)
    }
}

/// Builder for creating `Link` instances.
pub struct LinkBuilder<'a> {
    href: Cow<'a, str>,
    link: Link,
}

impl<'a> LinkBuilder<'a> {
    /// Creates a new `LinkBuilder` with the required `href` field.
    pub fn new(href: impl Into<Cow<'a, str>>) -> Self {
        Self {
            href: href.into(),
            link: Default::default(),
        }
    }

    /// Sets the `content_type` field.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.link.content_type = Some(content_type.into());
        self
    }

    /// Sets the `rel` field.
    pub fn rel(mut self, rel: impl Into<String>) -> Self {
        self.link.rel = Some(rel.into());
        self
    }

    /// Sets the `anchor` field.
    pub fn anchor(mut self, anchor: impl Into<String>) -> Self {
        match AnyUri::parse(anchor.into().as_str()) {
            Ok(uri) => self.link.anchor = Some(uri),
            Err(_) => {},
        }
        self
    }

    /// Sets the `sizes` field.
    pub fn sizes(mut self, sizes: impl Into<String>) -> Self {
        self.link.sizes = Some(sizes.into());
        self
    }

    /// Adds a single `hreflang`.
    pub fn hreflang(mut self, hreflang: impl Into<String>) -> Self {
        self.link.hreflang.get_or_insert_with(Vec::new).push(hreflang.into());
        self
    }

    /// Adds multiple `hreflang` values.
    pub fn hreflangs<I, S>(mut self, hreflangs: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String> {
        let mut items: Vec<String> = hreflangs.into_iter().map(|s| s.into()).collect();
        self.link.hreflang.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Builds and returns the `Link` instance.
    pub fn build(mut self) -> Result<Link, ParseError> {
        self.link.href = AnyUri::parse(&self.href)?;
        Ok(self.link)
    }
}
