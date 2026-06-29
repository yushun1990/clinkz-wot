use alloc::{borrow::Cow, string::String, vec::Vec};
use fluent_uri::ParseError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{OneOrMany, serde_as};

use crate::data_type::{ExtensionMap, UriReference};

/// Deserialize adapter carrying the `serde_as(Option<OneOrMany<_>>)` decoder
/// used by `Link::hreflang`.
#[serde_as]
#[derive(Deserialize)]
struct HrefLangField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<String>>);

/// A link relation to an arbitrary resource.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Link {
    /// Target IRI of the link.
    pub href: UriReference,

    /// Target media type of the link.
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
    pub hreflang: Option<Vec<String>>,

    pub _extra_fields: ExtensionMap,
}

impl<'de> Deserialize<'de> for Link {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let href = crate::flat::take_required(&mut map, "href")?;
        let content_type = crate::flat::take(&mut map, "type")?;
        let rel = crate::flat::take(&mut map, "rel")?;
        let anchor = crate::flat::take(&mut map, "anchor")?;
        let sizes = crate::flat::take(&mut map, "sizes")?;
        let hreflang = crate::flat::take::<HrefLangField, D::Error>(&mut map, "hreflang")?
            .and_then(|field| field.0);
        Ok(Link {
            href,
            content_type,
            rel,
            anchor,
            sizes,
            hreflang,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for Link {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("href", &self.href)?;
        if let Some(content_type) = &self.content_type {
            map.serialize_entry("type", content_type)?;
        }
        if let Some(rel) = &self.rel {
            map.serialize_entry("rel", rel)?;
        }
        if let Some(anchor) = &self.anchor {
            map.serialize_entry("anchor", anchor)?;
        }
        if let Some(sizes) = &self.sizes {
            map.serialize_entry("sizes", sizes)?;
        }
        if let Some(hreflang) = &self.hreflang {
            map.serialize_entry("hreflang", &crate::flat::OneOrManyRef(hreflang))?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
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
        let target = self.link.hreflang.get_or_insert_with(Vec::new);
        target.extend(hreflangs.into_iter().map(|s| s.into()));
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
