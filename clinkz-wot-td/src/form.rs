use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::data_type::{AdditionalExpectedResponse, AnyUri, ExpectedResponse, Operation};

/// A form can be viewed as a statement of "To perform an operation type
/// operation on form context, make a request method request to submission
/// target" where the optional form fields may further describe the
/// required request. In Thing Descriptions, the form context is the
/// surrounding Object, such as Properties, Actions, and Events or the Thing
/// itself for meta-interactions.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    /// Target IRI of the resource or service.
    pub href: AnyUri,

    #[serde(default="default_content_type")]
    /// Media type of data sent/received (e.g., "application/json").
    pub content_type: Option<String>,

    /// Content coding (e.g., "gzip").
    pub content_coding: Option<String>,

    /// Reference to a security scheme definition by its name.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub security: Option<Vec<String>>,

    /// Scope names required for OAuth2.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub scopes: Option<Vec<String>>,

    /// This optional term can be used if, e.g., the output communication
    /// metadata differ from input metadata(e.g., output contentType differ
    /// from the input contentType). The response name contains metadata
    /// that is only valid for the primary response messages.
    pub response: Option<ExpectedResponse>,

    /// This optional term can be used if additional expected responses are
    /// possible, e.g., for error reporting. Each additional response needs
    /// to be distinguished from others in some way(for example, by specifying
    /// a protocol-specific error code), and may also have its own data schema.
    pub additional_responses: Option<Vec<AdditionalExpectedResponse>>,

    /// Indicates the exact mechanism will be accomplished for a given protocol
    /// when there are multiple options. For example, for HTPP and Events, it
    /// indicates which of several available mechanisms should be used for
    /// asynchronous notifications such as long pulling, WebSub, Server-Sent
    /// Events.
    pub subprotocol: Option<String>,

    /// Indicates the semantic intention of performing the operations decribed
    /// by the form.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub op: Option<Vec<Operation>>,
}

fn default_content_type() -> Option<String> {
    Some(String::from("application/form"))
}
