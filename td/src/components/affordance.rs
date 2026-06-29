use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::skip_serializing_none;

use crate::{
    data_type::{ExtensionMap, METADATA_KEYS, Metadata, MetadataHelper, Operation},
    validate::{Validate, ValidateError, ValidationLevel, schema_error_message},
};

#[cfg(feature = "td2-preview")]
use super::util::deserialize_option_bool_flexible;
use super::{data_schema::DataSchema, form::Form, util::deserialize_bool_flexible};

/// Deserialize adapter carrying the flexible-bool decoder used by `observable`,
/// `safe`, and `idempotent`.
#[derive(Deserialize)]
struct FlexBoolField(#[serde(deserialize_with = "deserialize_bool_flexible")] bool);

/// Deserialize adapter carrying the optional flexible-bool decoder used by the
/// TD 2.0 `synchronous` field.
#[cfg(feature = "td2-preview")]
#[derive(Deserialize)]
struct OptionFlexBoolField(
    #[serde(deserialize_with = "deserialize_option_bool_flexible")] Option<bool>,
);

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
    ///   for error reporting.
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
        self.interaction().forms.extend(forms);
        self
    }

    /// Sets the URI variables.
    fn uri_variables(mut self, uri_variables: BTreeMap<String, DataSchema>) -> Self {
        self.interaction().uri_variables = Some(uri_variables);
        self
    }

    /// Adds a URI variable.
    fn uri_variable(mut self, name: impl Into<String>, schema: impl Into<DataSchema>) -> Self {
        let uri_variables = self
            .interaction()
            .uri_variables
            .get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema.into());
        self
    }
}

/// An Interaction Affordance that exposes state of the Thing.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyAffordance {
    pub _schema: DataSchema,

    pub _interaction: InteractionAffordance,

    /// A hint that indicates whether Servients hosting the Thing and
    /// Intermediaries should provide a Protocol Binding that supports
    /// the observeproperty and unobserveproperty.
    pub observable: bool,
}

impl<'de> Deserialize<'de> for PropertyAffordance {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let forms = crate::flat::take_required::<Vec<Form>, D::Error>(&mut map, "forms")?;
        let uri_variables = crate::flat::take(&mut map, "uriVariables")?;
        let observable = match crate::flat::take::<FlexBoolField, D::Error>(&mut map, "observable")?
        {
            Some(field) => field.0,
            None => false,
        };
        let schema = crate::flat::from_remaining::<DataSchema, D::Error>(map)?;
        Ok(PropertyAffordance {
            _schema: schema,
            _interaction: InteractionAffordance {
                forms,
                uri_variables,
            },
            observable,
        })
    }
}

impl Serialize for PropertyAffordance {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._schema.serialize_into(&mut map)?;
        map.serialize_entry("forms", &self._interaction.forms)?;
        if let Some(uri_variables) = &self._interaction.uri_variables {
            map.serialize_entry("uriVariables", uri_variables)?;
        }
        if self.observable {
            map.serialize_entry("observable", &self.observable)?;
        }
        map.end()
    }
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
#[derive(Clone, Default, Debug, PartialEq)]
pub struct ActionAffordance {
    pub _metadata: Metadata,

    pub _interaction: InteractionAffordance,

    /// Used to define the input data schema of the Action.
    pub input: Option<DataSchema>,

    /// Used to define the output data schema of the Action.
    pub output: Option<DataSchema>,

    /// Signals if the Action is safe(=true) or not.
    /// Used to signal if there is no internal state is changed
    /// when invoking an Action.
    pub safe: bool,

    /// Indicates whether the Action is idempotent(=true) or not.
    pub idempotent: bool,

    /// Indicates whether the Action is synchronous(=true) or not.
    ///
    /// TD 2.0 field; gated behind the `td2-preview` feature. TD 1.1 actions are
    /// implicitly synchronous by default and do not carry this term.
    #[cfg(feature = "td2-preview")]
    pub synchronous: Option<bool>,

    pub _extra_fields: ExtensionMap,
}

impl<'de> Deserialize<'de> for ActionAffordance {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let metadata = crate::flat::drain_substruct::<Metadata, D::Error>(&mut map, METADATA_KEYS)?;
        let forms = crate::flat::take_required::<Vec<Form>, D::Error>(&mut map, "forms")?;
        let uri_variables = crate::flat::take(&mut map, "uriVariables")?;
        let input = crate::flat::take(&mut map, "input")?;
        let output = crate::flat::take(&mut map, "output")?;
        let safe = match crate::flat::take::<FlexBoolField, D::Error>(&mut map, "safe")? {
            Some(field) => field.0,
            None => false,
        };
        let idempotent = match crate::flat::take::<FlexBoolField, D::Error>(&mut map, "idempotent")?
        {
            Some(field) => field.0,
            None => false,
        };
        #[cfg(feature = "td2-preview")]
        let synchronous =
            crate::flat::take::<OptionFlexBoolField, D::Error>(&mut map, "synchronous")?
                .and_then(|field| field.0);
        Ok(ActionAffordance {
            _metadata: metadata,
            _interaction: InteractionAffordance {
                forms,
                uri_variables,
            },
            input,
            output,
            safe,
            idempotent,
            #[cfg(feature = "td2-preview")]
            synchronous,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for ActionAffordance {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._metadata.serialize_into(&mut map)?;
        map.serialize_entry("forms", &self._interaction.forms)?;
        if let Some(uri_variables) = &self._interaction.uri_variables {
            map.serialize_entry("uriVariables", uri_variables)?;
        }
        if let Some(input) = &self.input {
            map.serialize_entry("input", input)?;
        }
        if let Some(output) = &self.output {
            map.serialize_entry("output", output)?;
        }
        if self.safe {
            map.serialize_entry("safe", &self.safe)?;
        }
        if self.idempotent {
            map.serialize_entry("idempotent", &self.idempotent)?;
        }
        #[cfg(feature = "td2-preview")]
        if let Some(synchronous) = self.synchronous {
            map.serialize_entry("synchronous", &synchronous)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
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

    /// Sets the synchronous flag (TD 2.0; requires `td2-preview`).
    #[cfg(feature = "td2-preview")]
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

impl Default for ActionAffordanceBuilder {
    fn default() -> Self {
        Self::new()
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
#[derive(Clone, Default, Debug, PartialEq)]
pub struct EventAffordance {
    pub _metadata: Metadata,

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

    pub _extra_fields: ExtensionMap,
}

impl<'de> Deserialize<'de> for EventAffordance {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let metadata = crate::flat::drain_substruct::<Metadata, D::Error>(&mut map, METADATA_KEYS)?;
        let forms = crate::flat::take_required::<Vec<Form>, D::Error>(&mut map, "forms")?;
        let uri_variables = crate::flat::take(&mut map, "uriVariables")?;
        let subscription = crate::flat::take(&mut map, "subscription")?;
        let data = crate::flat::take(&mut map, "data")?;
        let data_response = crate::flat::take(&mut map, "dataResponse")?;
        let cancellation = crate::flat::take(&mut map, "cancellation")?;
        Ok(EventAffordance {
            _metadata: metadata,
            _interaction: InteractionAffordance {
                forms,
                uri_variables,
            },
            subscription,
            data,
            data_response,
            cancellation,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for EventAffordance {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        self._metadata.serialize_into(&mut map)?;
        map.serialize_entry("forms", &self._interaction.forms)?;
        if let Some(uri_variables) = &self._interaction.uri_variables {
            map.serialize_entry("uriVariables", uri_variables)?;
        }
        if let Some(subscription) = &self.subscription {
            map.serialize_entry("subscription", subscription)?;
        }
        if let Some(data) = &self.data {
            map.serialize_entry("data", data)?;
        }
        if let Some(data_response) = &self.data_response {
            map.serialize_entry("dataResponse", data_response)?;
        }
        if let Some(cancellation) = &self.cancellation {
            map.serialize_entry("cancellation", cancellation)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
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

impl Default for EventAffordanceBuilder {
    fn default() -> Self {
        Self::new()
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
