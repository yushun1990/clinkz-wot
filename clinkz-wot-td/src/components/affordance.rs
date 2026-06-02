use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    data_type::{ExtensionMap, Metadata, MetadataHelper, Operation},
    validate::{Validate, ValidateError, ValidationLevel},
};

use super::{
    data_schema::DataSchema,
    form::Form,
    util::{deserialize_bool_flexible, deserialize_option_bool_flexible},
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
}

pub trait InteractionHelper: Sized {
    fn interaction(&mut self) -> &mut InteractionAffordance;

    /// Adds a form to the interaction affordance.
    fn form(mut self, form: Form) -> Self {
        self.interaction().forms.push(form);
        self
    }

    /// Adds multiple forms to the interaction affordance.
    fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item = Form>,
    {
        let mut items: Vec<Form> = forms.into_iter().collect();
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
        let uri_variables = self
            .interaction()
            .uri_variables
            .get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }
}

/// An Interaction Affordance that exposes state of the Thing.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyAffordance {
    #[serde(flatten)]
    pub _schema: DataSchema,

    #[serde(flatten)]
    pub _interaction: InteractionAffordance,

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

impl Validate for PropertyAffordance {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        self._schema.validate_with_level(level)?;
        validate_interaction_schemas(&self._interaction, level)?;

        self._interaction.validate_ops("PropertyAffordance", |op| {
            matches!(
                op,
                Operation::ReadProperty
                    | Operation::WriteProperty
                    | Operation::ObserveProperty
                    | Operation::UnobserveProperty
            )
        })
    }
}

impl PropertyAffordance {
    /// Creates a builder for `PropertyAffordance`.
    pub fn builder(schema: impl Into<DataSchema>) -> PropertyAffordanceBuilder {
        PropertyAffordanceBuilder::new(schema.into())
    }
}

/// Builder for creating `PropertyAffordance` instances.
pub struct PropertyAffordanceBuilder {
    affordance: PropertyAffordance,
}

impl PropertyAffordanceBuilder {
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
    pub fn build(self) -> Result<PropertyAffordance, ValidateError> {
        self.affordance.validate()?;
        Ok(self.affordance)
    }
}

impl InteractionHelper for PropertyAffordanceBuilder {
    fn interaction(&mut self) -> &mut InteractionAffordance {
        &mut self.affordance._interaction
    }
}

/// An Interaction Affordance that allows to invoke a function of
/// the Thing.
#[skip_serializing_none]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
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
    pub _extra_fields: ExtensionMap,
}

impl Validate for ActionAffordance {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        validate_interaction_schemas(&self._interaction, level)?;
        if let Some(input) = &self.input {
            input.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!("input: {}", schema_error_message(err)))
            })?;
        }
        if let Some(output) = &self.output {
            output.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!("output: {}", schema_error_message(err)))
            })?;
        }

        self._interaction.validate_ops("ActionAffordance", |op| {
            matches!(
                op,
                Operation::InvokeAction | Operation::QueryAction | Operation::CancelAction
            )
        })
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
            affordance: Default::default(),
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

    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.affordance._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.affordance._extra_fields.insert(key.into(), value);
        self
    }

    /// Builds and returns the `ActionAffordance` instance.
    pub fn build(self) -> Result<ActionAffordance, ValidateError> {
        self.affordance.validate()?;
        Ok(self.affordance)
    }
}

impl MetadataHelper for ActionAffordanceBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.affordance._metadata
    }
}

impl InteractionHelper for ActionAffordanceBuilder {
    fn interaction(&mut self) -> &mut InteractionAffordance {
        &mut self.affordance._interaction
    }
}

/// An interaction Affordance that describes an event source, which
/// asynchronously pushes event data to Consumers.
#[skip_serializing_none]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
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

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
}

impl Validate for EventAffordance {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        validate_interaction_schemas(&self._interaction, level)?;
        if let Some(subscription) = &self.subscription {
            subscription.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!("subscription: {}", schema_error_message(err)))
            })?;
        }
        if let Some(data) = &self.data {
            data.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!("data: {}", schema_error_message(err)))
            })?;
        }
        if let Some(data_response) = &self.data_response {
            data_response.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!("dataResponse: {}", schema_error_message(err)))
            })?;
        }
        if let Some(cancellation) = &self.cancellation {
            cancellation.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!("cancellation: {}", schema_error_message(err)))
            })?;
        }

        self._interaction.validate_ops("EventAffordance", |op| {
            matches!(op, Operation::SubscribeEvent | Operation::UnsubscribeEvent)
        })
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

    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.affordance._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.affordance._extra_fields.insert(key.into(), value);
        self
    }

    /// Builds and returns the `EventAffordance` instance.
    pub fn build(self) -> Result<EventAffordance, ValidateError> {
        self.affordance.validate()?;
        Ok(self.affordance)
    }
}

impl MetadataHelper for EventAffordanceBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.affordance._metadata
    }
}

impl InteractionHelper for EventAffordanceBuilder {
    fn interaction(&mut self) -> &mut InteractionAffordance {
        &mut self.affordance._interaction
    }
}

fn validate_interaction_schemas(
    interaction: &InteractionAffordance,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    if let Some(uri_variables) = &interaction.uri_variables {
        for (name, schema) in uri_variables {
            schema.validate_with_level(level).map_err(|err| {
                ValidateError::InvalidSchema(format!(
                    "uriVariables.{}: {}",
                    name,
                    schema_error_message(err)
                ))
            })?;
        }
    }

    Ok(())
}

fn schema_error_message(err: ValidateError) -> String {
    match err {
        ValidateError::InvalidSchema(message) => message,
        other => other.to_string(),
    }
}
