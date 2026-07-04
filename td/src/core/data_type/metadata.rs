//! Human-readable metadata vocabulary shared by Thing, Thing Model, and
//! affordance definitions (`@type`, `title`, `titles`, `description`,
//! `descriptions`), plus the multi-language map type.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

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
