use alloc::{string::{String, ToString}, format, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    data_schema::DataSchemaContext, data_type::Operation, form::Form, util::deserialize_bool_flexible, validate::{Validate, ValidateError}
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
    pub uri_variables: Option<serde_json::Map<String, serde_json::Value>>,
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
    fn validate(&self) -> Result<(), crate::validate::ValidateError> {
        // 1. Validate the underlying DataSchema constraints if any
        // (Assuming DataSchemaContext or Metadata will have their own Validate impl)

        // 2. Validate operations in forms according to W3C WoT spec.
        // For PropertyAffordance, 'op' MUST be one of:
        // readproperty, writeproperty, observeproperty, unobserveproperty.
        for form in &self._interaction.forms {
            if let Some(ops) = &form.op {
                for op in ops {
                    match op {
                        Operation::ReadProperty |
                        Operation::WriteProperty |
                        Operation::ObserveProperty |
                        Operation::UnobserveProperty => continue,
                        _ => return Err(ValidateError::InvalidOperation {
                            context: "PropertyAffordance".to_string(),
                            found: format!("{:?}", op),
                        }),
                    }
                }
            }
        }

        Ok(())
    }
}
