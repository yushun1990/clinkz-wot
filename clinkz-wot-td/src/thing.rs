use alloc::{collections::BTreeMap, string::String};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use crate::context::Context;


#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct Nil;

/// A map of language tags to strings (e.g., {"en": "Light", "zh": "灯"})
///
/// Using BTreeMap instead of HashMap to ensure daterministic serialization order.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct MultiLanguage(BTreeMap<String, String>);

impl MultiLanguage {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn add(&mut self, lang: &str, text: &str) {
        self.0.insert(String::from(lang), String::from(text));
    }
}


#[serde_as]
#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(
    serialize = "Ext: Serialize",
    deserialize = "Ext: Deserialize<'de>"
))]
pub struct Thing<Ext = Nil> {
    /// The JONS-LD Context.
    #[serde(rename = "@context")]
    pub context: Context,

    /// Unique identifier of the Thing (optional by recommended).
    pub id: Option<String>,

    /// Human-readable title.
    pub title: String,

    /// Multi-language titles (optional).
    pub titles: Option<MultiLanguage>,

    /// Human-readable additional information (optional).
    pub description: Option<String>,

    /// Multi-language descriptions (optional).
    pub descriptions: Option<MultiLanguage>,

    #[serde(flatten)]
    pub _extra_fields: Ext
}
