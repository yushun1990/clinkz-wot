use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::data_type::AnyUri;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
