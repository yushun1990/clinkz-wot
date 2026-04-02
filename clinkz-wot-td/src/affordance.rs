use alloc::{string::{String, ToString}, collections::BTreeMap, format, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    data_schema::{DataSchema, DataSchemaContext},
    data_type::{Metadata, Operation},
    form::Form,
    util::{deserialize_bool_flexible, deserialize_option_bool_flexible},
    validate::{Validate, ValidateError}
};

/// Metadata of a Thing that shows the possible choices to Consumers,
/// thereby suggesting how Consumers may interact with the Thing.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionAffordance {
    /// Set of form hypermedia controls that describe how an operation
    /// can be performed.
    pub forms: Vec<Form>,

    /// Define URI template variables according to as collection based on
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

/// An Interaction Affordance that exposes state of the Thing.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
