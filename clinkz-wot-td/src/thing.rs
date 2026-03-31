use alloc::vec::Vec;

use alloc::string::String;
use time::format_description::well_known::Rfc3339;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};
use time::OffsetDateTime;

use crate::{context::Context, data_type::{AnyUri, MultiLanguage, Nil, VersionInfo}};


/// An abstraction of a physical or virtual entity whose metadata and interfaces are
/// described by a WoT Thing Description, whereas a virtual entity is the composition
/// os one or more Things.
#[serde_as]
#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(
    serialize = "Ext: Serialize",
    deserialize = "Ext: Deserialize<'de>"
))]
pub struct Thing<Ext = Nil> {
    /// JSON-LD keyword to define short-hand names called terms that are used throughout
    /// a TD document.
    #[serde(rename = "@context")]
    pub context: Context,

    /// JSON-LD keyword to label the object with semantic tags(or types).
    #[serde(rename = "@type")]
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub tags: Option<Vec<String>>,

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

    /// Provides a version information.
    pub version: Option<VersionInfo>,

    /// Provides information when the TD instance was created.
    #[serde_as(as = "Option<Rfc3339>")]
    pub created: Option<OffsetDateTime>,

    /// Provides information when the TD instance was last modified.
    #[serde_as(as = "Option<Rfc3339>")]
    pub modified: Option<OffsetDateTime>,

    /// Provides information about the TD maintainer as URI scheme.
    pub support: Option<AnyUri>,

    /// Define the base URI that is used for all relative URI references
    /// throughout a TD document. In TD instances, all relative URIs
    /// are resovled relative to the base URI using the algorithm defnied
    /// in [RFC3986]
    pub base: Option<AnyUri>,

    #[serde(flatten)]
    pub _extra_fields: Ext
}
