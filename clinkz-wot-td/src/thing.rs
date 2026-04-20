use alloc::{string::{String, ToString}, format, collections::BTreeMap, vec::Vec};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::Context,
    data_schema::DataSchema,
    data_type::{AnyUri, Metadata, MetadataHelper, Nil, VersionInfo},
    form::Form,
    link::Link,
    security_scheme::SecurityScheme, validate::{Validate, ValidateError}
};


/// An abstraction of a physical or virtual entity whose metadata and interfaces are
/// described by a WoT Thing Description, whereas a virtual entity is the composition
/// of one or more Things.
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Deserialize, Serialize)]
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

impl <Ext> Thing<Ext>
where
    Ext: Default + Serialize + Validate
{
    /// Creates a new ThingBuilder with a default "nosec" security configuration.
    pub fn builder(title: impl Into<String>) -> ThingBuilder<Ext> {
        ThingBuilder::new(title)
    }

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

pub struct ThingBuilder<Ext> {
    thing: Thing<Ext>
}

impl <Ext> ThingBuilder<Ext>
where
    Ext: Default + Serialize + Validate
{
    pub fn new(title: impl Into<String>) -> Self{
        Self {
            thing: Thing {
                _metadata: Metadata {
                    title: Some(title.into()),
                    ..Default::default()
                },
                security: alloc::vec![],
                security_definitions: BTreeMap::new(),
                _extra_fields: Ext::default(),
                ..Default::default()
            }
        }
    }

    /// Sets the Things's unique identifier
    pub fn id(mut self, id: &str) -> Self {
        self.thing.id = AnyUri::parse(id).ok();
        self
    }

    /// Sets the context.
    pub fn context(mut self, context: impl Into<Context>) -> Self {
        self.thing.context = context.into();
        self
    }

    /// Sets the version information.
    pub fn version(mut self, version: VersionInfo) -> Self {
        self.thing.version = Some(version);
        self
    }

    /// Sets the creation time.
    pub fn created(mut self, created: OffsetDateTime) -> Self {
        self.thing.created = Some(created);
        self
    }

    /// Sets the modification time.
    pub fn modified(mut self, modified: OffsetDateTime) -> Self {
        self.thing.modified = Some(modified);
        self
    }

    /// Sets the support URI.
    pub fn support(mut self, support: &str) -> Self {
        self.thing.support = AnyUri::parse(support).ok();
        self
    }

    /// Sets the base URI.
    pub fn base(mut self, base: &str) -> Self {
        self.thing.base = AnyUri::parse(base).ok();
        self
    }

    /// Adds a profile URI.
    pub fn profile(mut self, profile: &str) -> Self {
        if let Some(profile) = AnyUri::parse(profile).ok() {
            self.thing.profile.get_or_insert_with(Vec::new).push(profile);
        }
        self
    }

    /// Adds multiple profile URIs.
    pub fn profiles<I>(mut self, profiles: I) -> Self
    where
        I: IntoIterator<Item=&'static str> {
        let mut items: Vec<AnyUri> = profiles.into_iter()
            .filter_map(|p| AnyUri::parse(p).ok())
            .collect();
        self.thing.profile.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Adds a property affordance.
    pub fn property(mut self, name: impl Into<String>, property: PropertyAffordance) -> Self {
        let properties = self.thing.properties.get_or_insert_with(BTreeMap::new);
        properties.insert(name.into(), property);
        self
    }

    /// Adds an action affordance.
    pub fn action(mut self, name: impl Into<String>, action: ActionAffordance) -> Self {
        let actions = self.thing.actions.get_or_insert_with(BTreeMap::new);
        actions.insert(name.into(), action);
        self
    }

    /// Adds an event affordance.
    pub fn event(mut self, name: impl Into<String>, event: EventAffordance) -> Self {
        let events = self.thing.events.get_or_insert_with(BTreeMap::new);
        events.insert(name.into(), event);
        self
    }

    /// Adds a link.
    pub fn link(mut self, link: Link) -> Self {
        self.thing.links.get_or_insert_with(Vec::new).push(link);
        self
    }

    /// Adds multiple links.
    pub fn links<I>(mut self, links: I) -> Self
    where
        I: IntoIterator<Item=Link> {
        let mut items: Vec<Link> = links.into_iter().collect();
        self.thing.links.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Adds a form.
    pub fn form(mut self, form: Form) -> Self {
        self.thing.forms.get_or_insert_with(Vec::new).push(form);
        self
    }

    /// Adds multiple forms.
    pub fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item=Form> {
        let mut items: Vec<Form> = forms.into_iter().collect();
        self.thing.forms.get_or_insert_with(Vec::new).append(&mut items);
        self
    }

    /// Adds a security name.
    pub fn security(mut self, security: impl Into<SecurityScheme>) -> Self {
        let security = security.into();
        let scheme = security.scheme().to_string();
        self.thing.security.push(scheme.clone());
        self.thing.security_definitions.insert(scheme, security);
        self
    }

    /// Adds multiple security names.
    pub fn securities<I, S>(mut self, securities: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: Into<SecurityScheme>
    {
        for s in securities {
            self = self.security(s);
        }
        self
    }

    /// Adds a schema definition.
    pub fn schema_definition(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let schema_definitions = self.thing.schema_definitions.get_or_insert_with(BTreeMap::new);
        schema_definitions.insert(name.into(), schema);
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: DataSchema) -> Self {
        let uri_variables = self.thing.uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema);
        self
    }

    /// Builds and returns the `Thing` instance.
    pub fn build(self) -> Result<Thing<Ext>, ValidateError> {
        self.thing.validate()?;
        Ok(self.thing)
    }
}

impl <Ext> MetadataHelper for Thing<Ext>
where
    Ext: Default + Serialize
{
    fn metadata(&mut self) -> &mut Metadata {
        &mut self._metadata
    }
}
