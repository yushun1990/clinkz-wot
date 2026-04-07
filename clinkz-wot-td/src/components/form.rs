use alloc::{vec::Vec, string::String, borrow::Cow};
use fluent_uri::ParseError;
use serde::{Deserialize, Deserializer, Serialize};
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
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    /// Target IRI of the resource or service.
    pub href: AnyUri,

    /// Media type of data sent/received (e.g., "application/json").
    #[serde(default="default_content_type")]
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
}

fn default_content_type() -> String {
    String::from("application/json")
}

impl<'de> Deserialize<'de> for Form {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Internal shadow struct to capture raw JSON data.
        #[serde_as]
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FormShadow {
            pub href: AnyUri,
            #[serde(default = "default_content_type")]
            pub content_type: String,
            pub content_coding: Option<String>,
            #[serde_as(as = "Option<OneOrMany<_>>")]
            pub security: Option<Vec<String>>,
            #[serde_as(as = "Option<OneOrMany<_>>")]
            pub scopes: Option<Vec<String>>,
            pub response: Option<ExpectedResponse>,
            pub additional_responses: Option<Vec<AdditionalExpectedResponse>>,
            pub subprotocol: Option<String>,
            #[serde_as(as = "Option<OneOrMany<_>>")]
            pub op: Option<Vec<Operation>>,
        }

        let raw = FormShadow::deserialize(deserializer)?;

        // Logic: If response exists but lacks contentType, inherit from the parent Form.
        let mut processed_response = raw.response;
        if let Some(ref mut resp) = processed_response {
            if resp.content_type.is_empty() {
                resp.content_type = raw.content_type.clone()
            }
        }

        Ok(Form {
            href: raw.href,
            content_type: raw.content_type,
            content_coding: raw.content_coding,
            security: raw.security,
            scopes: raw.scopes,
            response: processed_response,
            additional_responses: raw.additional_responses,
            subprotocol: raw.subprotocol,
            op: raw.op,
        })
    }
}

impl Form {
    pub fn builder(href: &str) -> FormBuilder {
        FormBuilder::new(href)
    }
}


pub struct FormBuilder<'a> {
    href: Cow<'a, str>,
    form: Form,
}


impl <'a> FormBuilder <'a> {
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
    pub fn additional_response(mut self, response: impl Into<ExpectedResponse>, success: bool) -> Self {
        self.form.additional_responses.get_or_insert_with(Vec::new)
            .push(AdditionalExpectedResponse::new(response, success));
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
    pub fn build(mut self) -> Result<Form, ParseError> {
        self.form.href = AnyUri::parse(&self.href)?;

        Ok(self.form)
    }
}
