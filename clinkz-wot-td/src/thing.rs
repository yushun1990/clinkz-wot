use alloc::string::String;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

use crate::{context::Context, data_type::{AnyUri, Metadata, Nil, VersionInfo}};


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

    /// Unique identifier of the Thing (optional by recommended).
    pub id: Option<String>,

    /// metadata
    #[serde(flatten)]
    pub _metadata: Metadata,

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
