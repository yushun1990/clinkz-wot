use alloc::{vec::Vec, string::String, borrow::Cow};
use fluent_uri::ParseError;
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
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form<Ext> {
    /// Target IRI of the resource or service.
    pub href: AnyUri,

    /// Media type of data sent/received (e.g., "application/json").
    #[serde(default="default_content_type", skip_serializing_if = "is_default_content_type")]
    pub content_type: String,

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

    /// Indicates the semantic intention of performing the operations described
    /// by the form.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub op: Option<Vec<Operation>>,

    #[serde(flatten)]
    pub _extra_fields: Ext,
}

fn default_content_type() -> String {
    String::from("application/json")
}

fn is_default_content_type(content_type: &String) -> bool {
    content_type == &default_content_type()
}

impl <Ext> Form<Ext>
where
    Ext: Default,
{
    pub fn builder(href: &str) -> FormBuilder<'_, Ext> {
        FormBuilder::new(href)
    }
}


pub struct FormBuilder<'a, Ext> {
    href: Cow<'a, str>,
    form: Form<Ext>,
}


impl <'a, Ext> FormBuilder <'a, Ext>
where
    Ext: Default,
{
    pub fn new(href: impl Into<Cow<'a, str>>) -> Self {
        Self {
            href: href.into(),
            form: Default::default()
        }
    }

    /// Set the content type (e.g., "application/cbor").
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.form.content_type = content_type.into();
        self
    }

    /// Set the content encoding (e.g., "gzip")
    pub fn content_coding(mut self, coding: impl Into<String>) -> Self {
        self.form.content_coding = Some(coding.into());
        self
    }

    /// Add security.
    pub fn security<I, S>(mut self, security: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String> {

            let mut items: Vec<String> = security.into_iter().map(|s| s.into()).collect();
            self.form.security.get_or_insert_with(Vec::new).append(&mut items);
            self
    }

    /// Assign scopes
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<String> {

            let mut items: Vec<String> = scopes.into_iter().map(|s| s.into()).collect();
            self.form.scopes.get_or_insert_with(Vec::new).append(&mut items);
            self
    }

    /// Set the response (e.g., "application/json")
    pub fn response(mut self, response: impl Into<ExpectedResponse>) -> Self {
        self.form.response = Some(response.into());
        self
    }

    /// Add additional response with schema as null.
    pub fn additional_response(mut self, response: impl Into<AdditionalExpectedResponse>) -> Self {
        self.form.additional_responses.get_or_insert_with(Vec::new)
            .push(response.into());
        self
    }

    /// Add multiple additional responses.
    pub fn additonal_responses(
        mut self,
        responses: impl IntoIterator<Item=AdditionalExpectedResponse>) -> Self {
            let mut items: Vec<_> = responses.into_iter().collect();
            self.form.additional_responses.get_or_insert_with(Vec::new).append(&mut items);

            self
        }

    /// Add operations.
    pub fn op(mut self, op: impl IntoIterator<Item=Operation>) -> Self {
        let mut items: Vec<Operation> = op.into_iter().collect();
        self.form.op.get_or_insert_with(Vec::new).append(&mut items);

        self
    }

    /// Build the form.
    pub fn build(mut self) -> Result<Form<Ext>, ParseError> {
        self.form.href = AnyUri::parse(&self.href)?;

        Ok(self.form)
    }
}
