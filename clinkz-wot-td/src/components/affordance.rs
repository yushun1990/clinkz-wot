use alloc::{string::{String, ToString}, collections::BTreeMap, format, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    data_schema::{DataSchema, DataSchemaContext},
    data_type::{Metadata, Operation},
    form::Form,
    components_util::{deserialize_bool_flexible, deserialize_option_bool_flexible},
    validate::{Validate, ValidateError}
};

/// Metadata of a Thing that shows the possible choices to Consumers,
/// thereby suggesting how Consumers may interact with the Thing.
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionAffordance {
    /// Set of form hypermedia controls that describe how an operation
    /// can be performed.
    pub forms: Vec<Form>,

    /// Define URI template variables according to a collection based on
    /// DataSchema declarations.
    pub uri_variables: Option<BTreeMap<String, DataSchema>>,
}

impl InteractionAffordance {
    /// Generic helper to validate that all operations in all forms satisfy
    /// a predicate.
    ///
    /// # Arguments
    /// * `context` - A string describing the caller (e.g., "PropertyAffordance")
    ///    for error reporting.
    /// * `f` - A closure that returns true if the specific Operation is allowed.
    pub fn validate_ops<F>(&self, context: &str, f: F) -> Result<(), ValidateError>
    where
        F: Fn(&Operation) -> bool,
    {
        for form in &self.forms {
            if let Some(ops) = &form.op {
                for op in ops {
                    if !f(op) {
                        return Err(ValidateError::InvalidOperation {
                            context: context.to_string(),
                            found: format!("{:?}", op),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Creates a builder for `InteractionAffordance`.
    pub fn builder() -> InteractionAffordanceBuilder {
        InteractionAffordanceBuilder::new()
    }
}

/// Builder for creating `InteractionAffordance` instances.
pub struct InteractionAffordanceBuilder {
    affordance: InteractionAffordance,
}

impl InteractionAffordanceBuilder {
    /// Creates a new `InteractionAffordanceBuilder`.
    pub fn new() -> Self {
        Self {
            affordance: InteractionAffordance::default(),
        }
    }

    /// Adds a form to the interaction affordance.
    pub fn form(mut self, form: Form) -> Self {
        self.affordance.forms.push(form);
        self
    }

    /// Adds multiple forms to the interaction affordance.
    pub fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item=Form> {
        let mut items: Vec<Form> = forms.into_iter().collect();
        self.affordance.forms.append(&mut items);
        self
    }

    /// Sets the URI variables.
    pub fn uri_variables(mut self, uri_variables: BTreeMap<String, DataSchema>) -> Self {
        self.affordance.uri_variables = Some(uri_variables);
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let uri_variables = self.affordance.uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }

    /// Builds and returns the `InteractionAffordance` instance.
    pub fn build(self) -> InteractionAffordance {
        self.affordance
    }
}

/// An Interaction Affordance that exposes state of the Thing.
#[skip_serializing_none]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyAffordance {
    #[serde(flatten)]
    pub _schema: DataSchemaContext,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance,

    /// A hint that indicates whether Servients hosting the Thing and
    /// Intermediaries should provide a Protocol Binding that supports
    /// the observeproperty and unobserveproperty.
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub observable: bool,
}

impl Validate for PropertyAffordance {
    fn validate(&self) -> Result<(), ValidateError> {
        self._interaction.validate_ops("PropertyAffordance", |op| matches!(
            op,
            Operation::ReadProperty |
            Operation::WriteProperty |
            Operation::ObserveProperty |
            Operation::UnobserveProperty
        ))
    }
}

impl PropertyAffordance {
    /// Creates a builder for `PropertyAffordance`.
    pub fn builder() -> PropertyAffordanceBuilder {
        PropertyAffordanceBuilder::new()
    }
}

/// Builder for creating `PropertyAffordance` instances.
pub struct PropertyAffordanceBuilder {
    affordance: PropertyAffordance,
}

impl PropertyAffordanceBuilder {
    /// Creates a new `PropertyAffordanceBuilder`.
    pub fn new() -> Self {
        Self {
            affordance: PropertyAffordance::default(),
        }
    }

    /// Sets the schema context.
    pub fn schema(mut self, schema: impl Into<DataSchemaContext>) -> Self {
        self.affordance._schema = schema.into();
        self
    }

    /// Sets the interaction affordance.
    pub fn interaction(mut self, interaction: impl Into<InteractionAffordance>) -> Self {
        self.affordance._interaction = interaction.into();
        self
    }

    /// Adds a form to the interaction affordance.
    pub fn form(mut self, form: Form) -> Self {
        self.affordance._interaction.forms.push(form);
        self
    }

    /// Adds multiple forms to the interaction affordance.
    pub fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item=Form> {
        let mut items: Vec<Form> = forms.into_iter().collect();
        self.affordance._interaction.forms.append(&mut items);
        self
    }

    /// Sets the URI variables.
    pub fn uri_variables(mut self, uri_variables: BTreeMap<String, DataSchema>) -> Self {
        self.affordance._interaction.uri_variables = Some(uri_variables);
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let uri_variables = self.affordance._interaction.uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }

    /// Sets the observable flag.
    pub fn observable(mut self, observable: bool) -> Self {
        self.affordance.observable = observable;
        self
    }

    /// Builds and returns the `PropertyAffordance` instance.
    pub fn build(self) -> PropertyAffordance {
        self.affordance
    }
}

/// An Interaction Affordance that allows to invoke a function of
/// the Thing.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionAffordance {
    #[serde(flatten)]
    pub _metadata: Metadata,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance,

    /// Used to define the input data schema of the Action.
    pub input: Option<DataSchema>,

    /// Used to define the output data schema of the Action.
    pub output: Option<DataSchema>,

    /// Signals if the Action is safe(=true) or not.
    /// Used to signal if there is no internal state is changed
    /// when invoking an Action.
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub safe: bool,

    /// Indicates whether the Action is idempotent(=true) or not.
    #[serde(default, deserialize_with = "deserialize_bool_flexible")]
    pub idempotent: bool,

    /// Indicates whether the Action is synchronous(=true) or not.
    #[serde(default, deserialize_with = "deserialize_option_bool_flexible")]
    pub synchronous: Option<bool>,
}

impl Validate for ActionAffordance {
    fn validate(&self) -> Result<(), ValidateError> {
        self._interaction.validate_ops("ActionAffordance", |op| matches!(
            op,
            Operation::InvokeAction |
            Operation::QueryAction |
            Operation::CancelAction
        ))
    }
}

impl ActionAffordance {
    /// Creates a builder for `ActionAffordance`.
    pub fn builder() -> ActionAffordanceBuilder {
        ActionAffordanceBuilder::new()
    }
}

/// Builder for creating `ActionAffordance` instances.
pub struct ActionAffordanceBuilder {
    affordance: ActionAffordance,
}

impl ActionAffordanceBuilder {
    /// Creates a new `ActionAffordanceBuilder`.
    pub fn new() -> Self {
        Self {
            affordance: ActionAffordance {
                _metadata: Metadata::default(),
                _interaction: InteractionAffordance::default(),
                input: None,
                output: None,
                safe: false,
                idempotent: false,
                synchronous: None,
            },
        }
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: impl Into<Metadata>) -> Self {
        self.affordance._metadata = metadata.into();
        self
    }

    /// Sets the interaction affordance.
    pub fn interaction(mut self, interaction: impl Into<InteractionAffordance>) -> Self {
        self.affordance._interaction = interaction.into();
        self
    }

    /// Adds a form to the interaction affordance.
    pub fn form(mut self, form: Form) -> Self {
        self.affordance._interaction.forms.push(form);
        self
    }

    /// Adds multiple forms to the interaction affordance.
    pub fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item=Form> {
        let mut items: Vec<Form> = forms.into_iter().collect();
        self.affordance._interaction.forms.append(&mut items);
        self
    }

    /// Sets the URI variables.
    pub fn uri_variables(mut self, uri_variables: BTreeMap<String, DataSchema>) -> Self {
        self.affordance._interaction.uri_variables = Some(uri_variables);
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let uri_variables = self.affordance._interaction.uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }

    /// Sets the input data schema.
    pub fn input(mut self, input: impl Into<DataSchema>) -> Self {
        self.affordance.input = Some(input.into());
        self
    }

    /// Sets the output data schema.
    pub fn output(mut self, output: impl Into<DataSchema>) -> Self {
        self.affordance.output = Some(output.into());
        self
    }

    /// Sets the safe flag.
    pub fn safe(mut self, safe: bool) -> Self {
        self.affordance.safe = safe;
        self
    }

    /// Sets the idempotent flag.
    pub fn idempotent(mut self, idempotent: bool) -> Self {
        self.affordance.idempotent = idempotent;
        self
    }

    /// Sets the synchronous flag.
    pub fn synchronous(mut self, synchronous: bool) -> Self {
        self.affordance.synchronous = Some(synchronous);
        self
    }

    /// Builds and returns the `ActionAffordance` instance.
    pub fn build(self) -> ActionAffordance {
        self.affordance
    }
}

/// An interaction Affordance that describes an event source, which
/// asynchronously pushes event data to Consumers.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAffordance {
    #[serde(flatten)]
    pub _metadata: Metadata,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance,

    /// Defines data that needs to be passed upon subscription,
    /// e.g., filters or message format for setting up Webhooks.
    pub subscription: Option<DataSchema>,

    /// Defines the data schema of the Event instance messages
    /// pushed by the Thing.
    pub data: Option<DataSchema>,

    /// Defines the data schema of the Event response messages
    /// sent by the consumer in a response to a data message.
    pub data_response: Option<DataSchema>,

    /// Defines any data that needs to be passed to cancel a
    /// subscription, e.g., a specific message to remove
    /// a Webhook.
    pub cancellation: Option<DataSchema>,
}

impl Validate for EventAffordance {
    fn validate(&self) -> Result<(), ValidateError> {
        self._interaction.validate_ops("EventAffordance", |op| matches!(
            op,
            Operation::SubscribeEvent |
            Operation::UnsubscribeEvent
        ))
    }
}

impl EventAffordance {
    /// Creates a builder for `EventAffordance`.
    pub fn builder() -> EventAffordanceBuilder {
        EventAffordanceBuilder::new()
    }
}

/// Builder for creating `EventAffordance` instances.
pub struct EventAffordanceBuilder {
    affordance: EventAffordance,
}

impl EventAffordanceBuilder {
    /// Creates a new `EventAffordanceBuilder`.
    pub fn new() -> Self {
        Self {
            affordance: EventAffordance {
                _metadata: Metadata::default(),
                _interaction: InteractionAffordance::default(),
                subscription: None,
                data: None,
                data_response: None,
                cancellation: None,
            },
        }
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: impl Into<Metadata>) -> Self {
        self.affordance._metadata = metadata.into();
        self
    }

    /// Sets the interaction affordance.
    pub fn interaction(mut self, interaction: impl Into<InteractionAffordance>) -> Self {
        self.affordance._interaction = interaction.into();
        self
    }

    /// Adds a form to the interaction affordance.
    pub fn form(mut self, form: Form) -> Self {
        self.affordance._interaction.forms.push(form);
        self
    }

    /// Adds multiple forms to the interaction affordance.
    pub fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item=Form> {
        let mut items: Vec<Form> = forms.into_iter().collect();
        self.affordance._interaction.forms.append(&mut items);
        self
    }

    /// Sets the URI variables.
    pub fn uri_variables(mut self, uri_variables: BTreeMap<String, DataSchema>) -> Self {
        self.affordance._interaction.uri_variables = Some(uri_variables);
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let uri_variables = self.affordance._interaction.uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }

    /// Sets the subscription data schema.
    pub fn subscription(mut self, subscription: impl Into<DataSchema>) -> Self {
        self.affordance.subscription = Some(subscription.into());
        self
    }

    /// Sets the data schema.
    pub fn data(mut self, data: impl Into<DataSchema>) -> Self {
        self.affordance.data = Some(data.into());
        self
    }

    /// Sets the data response schema.
    pub fn data_response(mut self, data_response: impl Into<DataSchema>) -> Self {
        self.affordance.data_response = Some(data_response.into());
        self
    }

    /// Sets the cancellation schema.
    pub fn cancellation(mut self, cancellation: impl Into<DataSchema>) -> Self {
        self.affordance.cancellation = Some(cancellation.into());
        self
    }

    /// Builds and returns the `EventAffordance` instance.
    pub fn build(self) -> EventAffordance {
        self.affordance
    }
}
