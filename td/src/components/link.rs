use alloc::{borrow::Cow, string::String, vec::Vec};
use fluent_uri::ParseError;
use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

use crate::data_type::{ExtensionMap, UriReference};

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Target IRI of the link.
    pub href: UriReference,

    /// Target media type of the link.
    #[serde(rename = "type")]
    pub content_type: Option<String>,

    /// Relation type between the current Thing and the target resource.
    /// Common values: "service-doc", "item", "parent", "collection".
    pub rel: Option<String>,

    /// The anchor should be used as the context of the link.
    pub anchor: Option<UriReference>,

    /// Target attributes that specifies one or more sizes for the
    /// referenced icon.
    pub sizes: Option<String>,

    /// Language of the target resource (BCP47).
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub hreflang: Option<Vec<String>>,

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
}

impl Link {
    pub fn builder<'a>(href: impl Into<Cow<'a, str>>) -> LinkBuilder<'a> {
        LinkBuilder::new(href)
    }
}

/// Builder for creating `Link` instances.
pub struct LinkBuilder<'a> {
    href: Cow<'a, str>,
    anchor: Option<Cow<'a, str>>,
    link: Link,
}

impl<'a> LinkBuilder<'a> {
    /// Creates a new `LinkBuilder` with the required `href` field.
    pub fn new(href: impl Into<Cow<'a, str>>) -> Self {
        Self {
            href: href.into(),
            anchor: None,
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
    pub fn anchor(mut self, anchor: impl Into<Cow<'a, str>>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    /// Sets the `sizes` field.
    pub fn sizes(mut self, sizes: impl Into<String>) -> Self {
        self.link.sizes = Some(sizes.into());
        self
    }

    /// Adds a single `hreflang`.
    pub fn hreflang(mut self, hreflang: impl Into<String>) -> Self {
        self.link
            .hreflang
            .get_or_insert_with(Vec::new)
            .push(hreflang.into());
        self
    }

    /// Adds multiple `hreflang` values.
    pub fn hreflangs<I, S>(mut self, hreflangs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut items: Vec<String> = hreflangs.into_iter().map(|s| s.into()).collect();
        self.link
            .hreflang
            .get_or_insert_with(Vec::new)
            .append(&mut items);
        self
    }

    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.link._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.link._extra_fields.insert(key.into(), value);
        self
    }

    /// Builds and returns the `Link` instance.
    pub fn build(mut self) -> Result<Link, ParseError> {
        self.link.href = UriReference::parse(&self.href)?;
        if let Some(anchor) = self.anchor {
            self.link.anchor = Some(UriReference::parse(&anchor)?);
        }
        Ok(self.link)
    }
}
