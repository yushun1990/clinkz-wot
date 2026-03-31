use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{data_type::{Metadata, Nil}, form::Form};

/// Metadata of a Thing that shows the possible choices to Consumers,
/// thereby suggesting how Consumers may interact with the Thing.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionAffordance<Ext = Nil> {
    #[serde(flatten)]
    pub _metadata: Metadata,
    /// Set of form hypermedia controls that describe how an operation
    /// can be performed.
    pub forms: Vec<Form>,

    /// Define URI template variables according to as collection based on
    /// DataSchema declarations.
    pub uri_variables: Option<serde_json::Map<String, serde_json::Value>>,

    #[serde(flatten)]
    pub _extra_fields: Ext,
}
