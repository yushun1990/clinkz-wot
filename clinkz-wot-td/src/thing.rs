use alloc::{string::{String, ToString}, format, collections::BTreeMap, vec::Vec};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::Context,
    data_schema::DataSchema,
    data_type::{AnyUri, Metadata, Nil, VersionInfo},
    form::Form,
    link::Link,
    security_scheme::SecurityScheme, validate::{Validate, ValidateError}
};


/// An abstraction of a physical or virtual entity whose metadata and interfaces are
/// described by a WoT Thing Description, whereas a virtual entity is the composition
/// of one or more Things.
#[serde_as]
#[skip_serializing_none]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(
    serialize = "Ext: Serialize",
    deserialize = "Ext: Deserialize<'de>"
))]
pub struct Thing<Ext = Nil> {
    /// JSON-LD keyword to define short-hand names called terms that are used throughout
    /// a TD document.
    #[serde(rename = "@context")]
    pub context: Context,

    /// Unique identifier of the Thing (optional by recommended).
    pub id: Option<AnyUri>,

    /// metadata
    #[serde(flatten)]
    pub _metadata: Metadata,

    /// Provides a version information.
    pub version: Option<VersionInfo>,

    /// Provides information when the TD instance was created.
    #[serde_as(as = "Option<Rfc3339>")]
    pub created: Option<OffsetDateTime>,

    /// Provides information when the TD instance was last modified.
    #[serde_as(as = "Option<Rfc3339>")]
    pub modified: Option<OffsetDateTime>,

    /// Provides information about the TD maintainer as URI scheme.
    pub support: Option<AnyUri>,

    /// Define the base URI that is used for all relative URI references
    /// throughout a TD document. In TD instances, all relative URIs
    /// are resovled relative to the base URI using the algorithm defnied
    /// in [RFC3986]
    pub base: Option<AnyUri>,

    /// All Property-based Interaction Affordances of the Thing.
    pub properties: Option<BTreeMap<String, PropertyAffordance>>,

    /// All Action-based Interaction Affordances of the Thing.
    pub actions: Option<BTreeMap<String, ActionAffordance>>,

    /// All Event-based Interaction Affordances of the Thing.
    pub events: Option<BTreeMap<String, EventAffordance>>,

    /// Provides Web links to arbitrary resources that relate to the
    /// specified Thing Description.
    pub links: Option<Vec<Link>>,

    /// Set of form hypermedia controls that describe how an operation
    /// can be performed.
    pub forms: Option<Vec<Form>>,

    /// Set of security definition names, chosen from those defined in
    /// securityDefinitions.
    #[serde_as(as = "OneOrMany<_>")]
    pub security: Vec<String>,

    /// Set of named security configurations(definitions only).
    pub security_definitions: BTreeMap<String, SecurityScheme>,

    /// Indicates the WoT Profile mechanisms followed by this
    /// Thing Description and the corresponding Thing  implementation.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub profile: Option<Vec<AnyUri>>,

    /// Set of named data schemas.
    ///
    /// To be used in a schema name-value pair inside an
    /// AdditionalExpectedResponse object.
    pub schema_definitions: Option<BTreeMap<String, DataSchema>>,

    /// Define URI template variables according to [RFC6570]
    /// as collection based on DataSchema declarations.
    pub uri_variables: Option<BTreeMap<String, DataSchema>>,

    #[serde(flatten)]
    pub _extra_fields: Ext
}

impl <Ext> Validate for Thing<Ext>
where
    Ext: Serialize + Validate,
{
    fn validate(&self) -> Result<(), crate::validate::ValidateError> {
        // title is mandatory
        if self._metadata.title.as_deref().unwrap_or("").is_empty() {
            return Err(ValidateError::MissingRequiredField("title".to_string()));
        }

        if self.security.is_empty() {
            return Err(ValidateError::MissingRequiredField("security".to_string()));
        }

        // Validate Properties
        if let Some(properties) = &self.properties {
            for (name, property) in properties {
                property.validate().map_err(|e| ValidateError::InvalidOperation {
                    context: format!("Property '{}'", name),
                    found: e.to_string(),
                })?;
            }
        }

        // Validate Actions
        if let Some(actions) = &self.actions {
            for (name, action) in actions {
                action.validate().map_err(|e| ValidateError::InvalidOperation {
                    context: format!("Action '{}'", name),
                    found: e.to_string(),
                })?;
            }
        }

        // Validate Events
        if let Some(events) = &self.events {
            for (name, event) in events {
                event.validate().map_err(|e| ValidateError::InvalidOperation {
                    context: format!("Event '{}'", name),
                    found: e.to_string(),
                })?;
            }
        }

        self._extra_fields.validate()?;

        Ok(())
    }
}
