use alloc::{string::{String, ToString}, collections::BTreeMap, format, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    data_type::{Metadata, MetadataHelper, Operation},
    validate::{Validate, ValidateError}
};

use super::{
    form::Form,
    util::{deserialize_bool_flexible, deserialize_option_bool_flexible},
    data_schema::DataSchema,
};

/// Metadata of a Thing that shows the possible choices to Consumers,
/// thereby suggesting how Consumers may interact with the Thing.
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionAffordance<Ext> {
    /// Set of form hypermedia controls that describe how an operation
    /// can be performed.
    pub forms: Vec<Form<Ext>>,

    /// Define URI template variables according to a collection based on
    /// DataSchema declarations.
    pub uri_variables: Option<BTreeMap<String, DataSchema>>,
}

impl <Ext> InteractionAffordance<Ext> {
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
}

pub trait InteractionHelper: Sized {
    type Ext;
    fn interaction(&mut self) -> &mut InteractionAffordance<Self::Ext>;

    /// Adds a form to the interaction affordance.
    fn form(mut self, form: Form<Self::Ext>) -> Self {
        self.interaction().forms.push(form);
        self
    }

    /// Adds multiple forms to the interaction affordance.
    fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item=Form<Self::Ext>> {
        let mut items: Vec<Form<Self::Ext>> = forms.into_iter().collect();
        self.interaction().forms.append(&mut items);
        self
    }

    /// Sets the URI variables.
    fn uri_variables(mut self, uri_variables: BTreeMap<String, DataSchema>) -> Self {
        self.interaction().uri_variables = Some(uri_variables);
        self
    }

    /// Adds a URI variable.
    fn uri_variable(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let uri_variables = self.interaction().uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }
}


/// An Interaction Affordance that exposes state of the Thing.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyAffordance<Ext> {
    #[serde(flatten)]
    pub _schema: DataSchema,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance<Ext>,

    /// A hint that indicates whether Servients hosting the Thing and
    /// Intermediaries should provide a Protocol Binding that supports
    /// the observeproperty and unobserveproperty.
    #[serde(
        default,
        deserialize_with = "deserialize_bool_flexible",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub observable: bool,
}

impl <Ext> Validate for PropertyAffordance<Ext> {
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

impl<Ext> PropertyAffordance<Ext>
where
    Ext: Default
{
    /// Creates a builder for `PropertyAffordance`.
    pub fn builder(schema: impl Into<DataSchema>) -> PropertyAffordanceBuilder<Ext> {
        PropertyAffordanceBuilder::<Ext>::new(schema.into())
    }
}

/// Builder for creating `PropertyAffordance` instances.
pub struct PropertyAffordanceBuilder<Ext> {
    affordance: PropertyAffordance<Ext>,
}

impl <Ext> PropertyAffordanceBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `PropertyAffordanceBuilder`.
    pub fn new(schema: DataSchema) -> Self {
        Self {
            affordance: PropertyAffordance {
                _schema: schema,
                _interaction: Default::default(),
                observable: Default::default(),
            },
        }
    }

    /// Sets the observable flag.
    pub fn observable(mut self, observable: bool) -> Self {
        self.affordance.observable = observable;
        self
    }

    /// Builds and returns the `PropertyAffordance` instance.
    pub fn build(self) -> Result<PropertyAffordance<Ext>, ValidateError> {
        self.affordance.validate()?;
        Ok(self.affordance)
    }
}

impl <Ext> InteractionHelper for PropertyAffordanceBuilder<Ext> {
    type Ext = Ext;
    fn interaction(&mut self) -> &mut InteractionAffordance<Ext> {
        &mut self.affordance._interaction
    }
}


/// An Interaction Affordance that allows to invoke a function of
/// the Thing.
#[skip_serializing_none]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionAffordance<Ext> {
    #[serde(flatten)]
    pub _metadata: Metadata,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance<Ext>,

    /// Used to define the input data schema of the Action.
    pub input: Option<DataSchema>,

    /// Used to define the output data schema of the Action.
    pub output: Option<DataSchema>,

    /// Signals if the Action is safe(=true) or not.
    /// Used to signal if there is no internal state is changed
    /// when invoking an Action.
    #[serde(
        default,
        deserialize_with = "deserialize_bool_flexible",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub safe: bool,

    /// Indicates whether the Action is idempotent(=true) or not.
    #[serde(
        default,
        deserialize_with = "deserialize_bool_flexible",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub idempotent: bool,

    /// Indicates whether the Action is synchronous(=true) or not.
    #[serde(default, deserialize_with = "deserialize_option_bool_flexible")]
    pub synchronous: Option<bool>,

    #[serde(flatten)]
    pub _extra_fields: Ext,
}

impl <Ext> Validate for ActionAffordance<Ext> {
    fn validate(&self) -> Result<(), ValidateError> {
        self._interaction.validate_ops("ActionAffordance", |op| matches!(
            op,
            Operation::InvokeAction |
            Operation::QueryAction |
            Operation::CancelAction
        ))
    }
}

impl <Ext> ActionAffordance<Ext>
where
    Ext: Default
{
    /// Creates a builder for `ActionAffordance`.
    pub fn builder() -> ActionAffordanceBuilder<Ext> {
        ActionAffordanceBuilder::<Ext>::new()
    }
}

/// Builder for creating `ActionAffordance` instances.
pub struct ActionAffordanceBuilder<Ext> {
    affordance: ActionAffordance<Ext>,
}

impl <Ext> ActionAffordanceBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `ActionAffordanceBuilder`.
    pub fn new() -> Self {
        Self {
            affordance: Default::default()
        }
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
    pub fn build(self) -> Result<ActionAffordance<Ext>, ValidateError> {
        self.affordance.validate()?;
        Ok(self.affordance)
    }
}

impl <Ext> MetadataHelper for ActionAffordanceBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.affordance._metadata
    }
}

impl <Ext> InteractionHelper for ActionAffordanceBuilder<Ext> {
    type Ext = Ext;

    fn interaction(&mut self) -> &mut InteractionAffordance<Ext> {
        &mut self.affordance._interaction
    }
}

/// An interaction Affordance that describes an event source, which
/// asynchronously pushes event data to Consumers.
#[skip_serializing_none]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAffordance<Ext> {
    #[serde(flatten)]
    pub _metadata: Metadata,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance<Ext>,

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

    #[serde(flatten)]
    pub _extra_fields: Ext,
}

impl <Ext> Validate for EventAffordance<Ext> {
    fn validate(&self) -> Result<(), ValidateError> {
        self._interaction.validate_ops("EventAffordance", |op| matches!(
            op,
            Operation::SubscribeEvent |
            Operation::UnsubscribeEvent
        ))
    }
}

impl <Ext> EventAffordance<Ext>
where
    Ext: Default
{
    /// Creates a builder for `EventAffordance`.
    pub fn builder() -> EventAffordanceBuilder<Ext> {
        EventAffordanceBuilder::<Ext>::new()
    }
}

/// Builder for creating `EventAffordance` instances.
pub struct EventAffordanceBuilder<Ext> {
    affordance: EventAffordance<Ext>,
}

impl <Ext> EventAffordanceBuilder<Ext>
where
    Ext: Default
{
    /// Creates a new `EventAffordanceBuilder`.
    pub fn new() -> Self {
        Self {
            affordance: Default::default(),
        }
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
    pub fn build(self) -> Result<EventAffordance<Ext>, ValidateError> {
        self.affordance.validate()?;
        Ok(self.affordance)
    }
}

impl <Ext> MetadataHelper for EventAffordanceBuilder<Ext> {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.affordance._metadata
    }
}

impl <Ext> InteractionHelper for EventAffordanceBuilder<Ext> {
    type Ext = Ext;

    fn interaction(&mut self) -> &mut InteractionAffordance<Ext> {
        &mut self.affordance._interaction
    }
}
