use alloc::{borrow::Cow, string::String, vec::Vec};
use fluent_uri::ParseError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{OneOrMany, serde_as};

use crate::data_type::{
    AdditionalExpectedResponse, ExpectedResponse, ExtensionMap, FormHref, Operation,
};

/// Deserialize adapter carrying the `serde_as(Option<OneOrMany<_>>)` decoder for
/// `Form::security`, `Form::scopes`, and `Form::op`.
#[serde_as]
#[derive(Deserialize)]
struct StringListField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<String>>);

#[serde_as]
#[derive(Deserialize)]
struct OperationListField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<Operation>>);

/// A form can be viewed as a statement of "To perform an operation type
/// operation on form context, make a request method request to submission
/// target" where the optional form fields may further describe the
/// required request. In Thing Descriptions, the form context is the
/// surrounding Object, such as Properties, Actions, and Events or the Thing
/// itself for meta-interactions.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    /// Target IRI of the resource or service.
    pub href: FormHref,

    /// Media type of data sent/received (e.g., "application/json").
    pub content_type: String,

    /// Content coding (e.g., "gzip").
    pub content_coding: Option<String>,

    /// Reference to a security scheme definition by its name.
    pub security: Option<Vec<String>>,

    /// Scope names required for OAuth2.
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
    pub op: Option<Vec<Operation>>,

    pub _extra_fields: ExtensionMap,
}

const DEFAULT_CONTENT_TYPE: &str = "application/json";

fn default_content_type() -> String {
    String::from(DEFAULT_CONTENT_TYPE)
}

fn is_default_content_type(content_type: &str) -> bool {
    content_type == DEFAULT_CONTENT_TYPE
}

impl<'de> Deserialize<'de> for Form {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let href = crate::flat::take_required(&mut map, "href")?;
        let content_type = crate::flat::take::<String, D::Error>(&mut map, "contentType")?
            .unwrap_or_else(default_content_type);
        let content_coding = crate::flat::take(&mut map, "contentCoding")?;
        let security = crate::flat::take::<StringListField, D::Error>(&mut map, "security")?
            .and_then(|field| field.0);
        let scopes = crate::flat::take::<StringListField, D::Error>(&mut map, "scopes")?
            .and_then(|field| field.0);
        let response = crate::flat::take(&mut map, "response")?;
        let additional_responses = crate::flat::take(&mut map, "additionalResponses")?;
        let subprotocol = crate::flat::take(&mut map, "subprotocol")?;
        let op = crate::flat::take::<OperationListField, D::Error>(&mut map, "op")?
            .and_then(|field| field.0);
        Ok(Form {
            href,
            content_type,
            content_coding,
            security,
            scopes,
            response,
            additional_responses,
            subprotocol,
            op,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for Form {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("href", &self.href)?;
        if !is_default_content_type(&self.content_type) {
            map.serialize_entry("contentType", &self.content_type)?;
        }
        if let Some(content_coding) = &self.content_coding {
            map.serialize_entry("contentCoding", content_coding)?;
        }
        if let Some(security) = &self.security {
            map.serialize_entry("security", &crate::flat::OneOrManyRef(security))?;
        }
        if let Some(scopes) = &self.scopes {
            map.serialize_entry("scopes", &crate::flat::OneOrManyRef(scopes))?;
        }
        if let Some(response) = &self.response {
            map.serialize_entry("response", response)?;
        }
        if let Some(additional_responses) = &self.additional_responses {
            map.serialize_entry("additionalResponses", additional_responses)?;
        }
        if let Some(subprotocol) = &self.subprotocol {
            map.serialize_entry("subprotocol", subprotocol)?;
        }
        if let Some(op) = &self.op {
            map.serialize_entry("op", &crate::flat::OneOrManyRef(op))?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl Default for Form {
    fn default() -> Self {
        Self {
            href: Default::default(),
            content_type: default_content_type(),
            content_coding: None,
            security: None,
            scopes: None,
            response: None,
            additional_responses: None,
            subprotocol: None,
            op: None,
            _extra_fields: Default::default(),
        }
    }
}

impl Form {
    pub fn builder<'a>(href: impl Into<Cow<'a, str>>) -> FormBuilder<'a> {
        FormBuilder::new(href)
    }

    /// Creates a form builder with `readproperty` operation metadata.
    pub fn read_property<'a>(href: impl Into<Cow<'a, str>>) -> FormBuilder<'a> {
        Self::builder(href).read_property()
    }

    /// Creates a form builder with `writeproperty` operation metadata.
    pub fn write_property<'a>(href: impl Into<Cow<'a, str>>) -> FormBuilder<'a> {
        Self::builder(href).write_property()
    }

    /// Creates a form builder with `invokeaction` operation metadata.
    pub fn invoke_action<'a>(href: impl Into<Cow<'a, str>>) -> FormBuilder<'a> {
        Self::builder(href).invoke_action()
    }

    /// Creates a form builder with `subscribeevent` operation metadata.
    pub fn subscribe_event<'a>(href: impl Into<Cow<'a, str>>) -> FormBuilder<'a> {
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

    /// Set the subprotocol (e.g., "sse").
    pub fn subprotocol(mut self, subprotocol: impl Into<String>) -> Self {
        self.form.subprotocol = Some(subprotocol.into());
        self
    }

    /// Add security.
    pub fn security<I, S>(mut self, security: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.form
            .security
            .get_or_insert_with(Vec::new)
            .extend(security.into_iter().map(|s| s.into()));
        self
    }

    /// Assign scopes
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.form
            .scopes
            .get_or_insert_with(Vec::new)
            .extend(scopes.into_iter().map(|s| s.into()));
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
    pub fn additional_responses(
        mut self,
        responses: impl IntoIterator<Item = AdditionalExpectedResponse>,
    ) -> Self {
        self.form
            .additional_responses
            .get_or_insert_with(Vec::new)
            .extend(responses);
        self
    }

    /// Add operations.
    pub fn op(mut self, op: impl IntoIterator<Item = Operation>) -> Self {
        self.form.op.get_or_insert_with(Vec::new).extend(op);
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

    /// Adds `cancelaction` operation metadata (TD 1.1).
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

    /// Adds `subscribeallevents` operation metadata (TD 1.1).
    pub fn subscribe_all_events(self) -> Self {
        self.single_op(Operation::SubscribeAllEvents)
    }

    /// Adds `unsubscribeallevents` operation metadata (TD 1.1).
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
