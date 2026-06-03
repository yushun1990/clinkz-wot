use alloc::{borrow::Cow, string::String, vec::Vec};
use fluent_uri::ParseError;
use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

use crate::data_type::{
    AdditionalExpectedResponse, ExpectedResponse, ExtensionMap, FormHref, Operation,
};

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
pub struct Form {
    /// Target IRI of the resource or service.
    pub href: FormHref,

    /// Media type of data sent/received (e.g., "application/json").
    #[serde(
        default = "default_content_type",
        skip_serializing_if = "is_default_content_type"
    )]
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
    pub _extra_fields: ExtensionMap,
}

fn default_content_type() -> String {
    String::from("application/json")
}

fn is_default_content_type(content_type: &String) -> bool {
    content_type == &default_content_type()
}

impl Form {
    pub fn builder(href: &str) -> FormBuilder<'_> {
        FormBuilder::new(href)
    }

    /// Creates a form builder with `readproperty` operation metadata.
    pub fn read_property(href: &str) -> FormBuilder<'_> {
        Self::builder(href).read_property()
    }

    /// Creates a form builder with `writeproperty` operation metadata.
    pub fn write_property(href: &str) -> FormBuilder<'_> {
        Self::builder(href).write_property()
    }

    /// Creates a form builder with `invokeaction` operation metadata.
    pub fn invoke_action(href: &str) -> FormBuilder<'_> {
        Self::builder(href).invoke_action()
    }

    /// Creates a form builder with `subscribeevent` operation metadata.
    pub fn subscribe_event(href: &str) -> FormBuilder<'_> {
        Self::builder(href).subscribe_event()
    }
}

pub struct FormBuilder<'a> {
    href: Cow<'a, str>,
    form: Form,
}

impl<'a> FormBuilder<'a> {
    pub fn new(href: impl Into<Cow<'a, str>>) -> Self {
        Self {
            href: href.into(),
            form: Default::default(),
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
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut items: Vec<String> = security.into_iter().map(|s| s.into()).collect();
        self.form
            .security
            .get_or_insert_with(Vec::new)
            .append(&mut items);
        self
    }

    /// Assign scopes
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut items: Vec<String> = scopes.into_iter().map(|s| s.into()).collect();
        self.form
            .scopes
            .get_or_insert_with(Vec::new)
            .append(&mut items);
        self
    }

    /// Set the response (e.g., "application/json")
    pub fn response(mut self, response: impl Into<ExpectedResponse>) -> Self {
        self.form.response = Some(response.into());
        self
    }

    /// Add additional response with schema as null.
    pub fn additional_response(mut self, response: impl Into<AdditionalExpectedResponse>) -> Self {
        self.form
            .additional_responses
            .get_or_insert_with(Vec::new)
            .push(response.into());
        self
    }

    /// Add multiple additional responses.
    pub fn additonal_responses(
        mut self,
        responses: impl IntoIterator<Item = AdditionalExpectedResponse>,
    ) -> Self {
        let mut items: Vec<_> = responses.into_iter().collect();
        self.form
            .additional_responses
            .get_or_insert_with(Vec::new)
            .append(&mut items);

        self
    }

    /// Add operations.
    pub fn op(mut self, op: impl IntoIterator<Item = Operation>) -> Self {
        let mut items: Vec<Operation> = op.into_iter().collect();
        self.form.op.get_or_insert_with(Vec::new).append(&mut items);

        self
    }

    fn single_op(self, operation: Operation) -> Self {
        self.op([operation])
    }

    /// Adds `readproperty` operation metadata.
    pub fn read_property(self) -> Self {
        self.single_op(Operation::ReadProperty)
    }

    /// Adds `writeproperty` operation metadata.
    pub fn write_property(self) -> Self {
        self.single_op(Operation::WriteProperty)
    }

    /// Adds `observeproperty` operation metadata.
    pub fn observe_property(self) -> Self {
        self.single_op(Operation::ObserveProperty)
    }

    /// Adds `unobserveproperty` operation metadata.
    pub fn unobserve_property(self) -> Self {
        self.single_op(Operation::UnobserveProperty)
    }

    /// Adds `invokeaction` operation metadata.
    pub fn invoke_action(self) -> Self {
        self.single_op(Operation::InvokeAction)
    }

    /// Adds `queryaction` operation metadata.
    pub fn query_action(self) -> Self {
        self.single_op(Operation::QueryAction)
    }

    /// Adds `cancelaction` operation metadata.
    pub fn cancel_action(self) -> Self {
        self.single_op(Operation::CancelAction)
    }

    /// Adds `subscribeevent` operation metadata.
    pub fn subscribe_event(self) -> Self {
        self.single_op(Operation::SubscribeEvent)
    }

    /// Adds `unsubscribeevent` operation metadata.
    pub fn unsubscribe_event(self) -> Self {
        self.single_op(Operation::UnsubscribeEvent)
    }

    /// Adds `readallproperties` operation metadata.
    pub fn read_all_properties(self) -> Self {
        self.single_op(Operation::ReadAllProperties)
    }

    /// Adds `writeallproperties` operation metadata.
    pub fn write_all_properties(self) -> Self {
        self.single_op(Operation::WriteAllProperties)
    }

    /// Adds `readmultipleproperties` operation metadata.
    pub fn read_multiple_properties(self) -> Self {
        self.single_op(Operation::ReadMultipleProperties)
    }

    /// Adds `writemultipleproperties` operation metadata.
    pub fn write_multiple_properties(self) -> Self {
        self.single_op(Operation::WriteMultipleProperties)
    }

    /// Adds `observeallproperties` operation metadata.
    pub fn observe_all_properties(self) -> Self {
        self.single_op(Operation::ObserveAllProperties)
    }

    /// Adds `unobserveallproperties` operation metadata.
    pub fn unobserve_all_properties(self) -> Self {
        self.single_op(Operation::UnobserveAllProperties)
    }

    /// Adds `queryallactions` operation metadata.
    pub fn query_all_actions(self) -> Self {
        self.single_op(Operation::QueryAllActions)
    }

    /// Adds `subscribeallevents` operation metadata.
    pub fn subscribe_all_events(self) -> Self {
        self.single_op(Operation::SubscribeAllEvents)
    }

    /// Adds `unsubscribeallevents` operation metadata.
    pub fn unsubscribe_all_events(self) -> Self {
        self.single_op(Operation::UnsubscribeAllEvents)
    }

    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.form._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.form._extra_fields.insert(key.into(), value);
        self
    }

    /// Build the form.
    pub fn build(mut self) -> Result<Form, ParseError> {
        self.form.href = FormHref::parse(&self.href)?;

        Ok(self.form)
    }
}
