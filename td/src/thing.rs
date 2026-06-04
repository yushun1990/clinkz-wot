use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use time::OffsetDateTime;

use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::Context,
    data_schema::DataSchema,
    data_type::{AbsoluteUri, BaseUri, ExtensionMap, Metadata, MetadataHelper, VersionInfo},
    form::Form,
    link::Link,
    security_scheme::SecurityScheme,
    validate::{Validate, ValidateError, ValidationLevel},
};

/// An abstraction of a physical or virtual entity whose metadata and interfaces are
/// described by a WoT Thing Description, whereas a virtual entity is the composition
/// of one or more Things.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thing {
    /// JSON-LD keyword to define short-hand names called terms that are used throughout
    /// a TD document.
    #[serde(rename = "@context")]
    pub context: Context,

    /// Unique identifier of the Thing (optional by recommended).
    pub id: Option<AbsoluteUri>,

    /// metadata
    #[serde(flatten)]
    pub _metadata: Metadata,

    /// Provides a version information.
    pub version: Option<VersionInfo>,

    /// Provides information when the TD instance was created.
    #[serde(with = "time::serde::rfc3339::option")]
    #[serde(default)]
    pub created: Option<OffsetDateTime>,

    /// Provides information when the TD instance was last modified.
    #[serde(with = "time::serde::rfc3339::option")]
    #[serde(default)]
    pub modified: Option<OffsetDateTime>,

    /// Provides information about the TD maintainer as URI scheme.
    pub support: Option<AbsoluteUri>,

    /// Define the base URI that is used for all relative URI references
    /// throughout a TD document. In TD instances, all relative URIs
    /// are resovled relative to the base URI using the algorithm defnied
    /// in [RFC3986]
    pub base: Option<BaseUri>,

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
    pub profile: Option<Vec<AbsoluteUri>>,

    /// Set of named data schemas.
    ///
    /// To be used in a schema name-value pair inside an
    /// AdditionalExpectedResponse object.
    pub schema_definitions: Option<BTreeMap<String, DataSchema>>,

    /// Define URI template variables according to [RFC6570]
    /// as collection based on DataSchema declarations.
    pub uri_variables: Option<BTreeMap<String, DataSchema>>,

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
}

impl Thing {
    /// Creates a new ThingBuilder with a default "nosec" security configuration.
    pub fn builder(title: impl Into<String>) -> ThingBuilder {
        ThingBuilder::new(title)
    }
}

impl Validate for Thing {
    fn validate_with_level(
        &self,
        level: ValidationLevel,
    ) -> Result<(), crate::validate::ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        // title is mandatory
        if self._metadata.title.as_deref().unwrap_or("").is_empty() {
            return Err(ValidateError::MissingRequiredField("title".to_string()));
        }

        if self.security.is_empty() {
            return Err(ValidateError::MissingRequiredField("security".to_string()));
        }
        validate_security_references("Thing.security", &self.security, &self.security_definitions)?;
        validate_security_definitions(&self.security_definitions, level)?;

        if let Some(schema_definitions) = &self.schema_definitions {
            validate_schema_map("schemaDefinitions", schema_definitions, level)?;
        }
        if let Some(uri_variables) = &self.uri_variables {
            validate_schema_map("uriVariables", uri_variables, level)?;
        }

        // Validate Properties
        if let Some(properties) = &self.properties {
            for (name, property) in properties {
                property.validate_with_level(level).map_err(|e| {
                    contextualize_affordance_error(format!("Property '{}'", name), e)
                })?;
                validate_form_security_references(
                    format!("Property '{}'", name),
                    &property._interaction.forms,
                    &self.security_definitions,
                )?;
            }
        }

        // Validate Actions
        if let Some(actions) = &self.actions {
            for (name, action) in actions {
                action
                    .validate_with_level(level)
                    .map_err(|e| contextualize_affordance_error(format!("Action '{}'", name), e))?;
                validate_form_security_references(
                    format!("Action '{}'", name),
                    &action._interaction.forms,
                    &self.security_definitions,
                )?;
            }
        }

        // Validate Events
        if let Some(events) = &self.events {
            for (name, event) in events {
                event
                    .validate_with_level(level)
                    .map_err(|e| contextualize_affordance_error(format!("Event '{}'", name), e))?;
                validate_form_security_references(
                    format!("Event '{}'", name),
                    &event._interaction.forms,
                    &self.security_definitions,
                )?;
            }
        }

        if let Some(forms) = &self.forms {
            validate_form_security_references(
                "Thing.forms".to_string(),
                forms,
                &self.security_definitions,
            )?;
        }

        self._extra_fields.validate_with_level(level)?;

        Ok(())
    }
}

fn validate_schema_map(
    context: &str,
    schemas: &BTreeMap<String, DataSchema>,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    for (name, schema) in schemas {
        schema.validate_with_level(level).map_err(|err| {
            ValidateError::InvalidSchema(format!(
                "{}.{}: {}",
                context,
                name,
                schema_error_message(err)
            ))
        })?;
    }

    Ok(())
}

fn validate_security_definitions(
    security_definitions: &BTreeMap<String, SecurityScheme>,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    for (name, scheme) in security_definitions {
        let context = format!("securityDefinitions.{}", name);
        scheme
            .validate_with_level(level)
            .map_err(|err| contextualize_security_error(context.clone(), err))?;
        scheme.validate_references(context.as_str(), security_definitions)?;
    }

    Ok(())
}

fn contextualize_affordance_error(context: String, err: ValidateError) -> ValidateError {
    match err {
        ValidateError::InvalidSchema(message) => {
            ValidateError::InvalidSchema(format!("{}: {}", context, message))
        }
        other => ValidateError::InvalidOperation {
            context,
            found: other.to_string(),
        },
    }
}

fn contextualize_security_error(context: String, err: ValidateError) -> ValidateError {
    match err {
        ValidateError::InvalidSecurity(message) => {
            ValidateError::InvalidSecurity(format!("{}: {}", context, message))
        }
        ValidateError::MissingRequiredField(field) => {
            ValidateError::MissingRequiredField(format!("{}: {}", context, field))
        }
        ValidateError::InvalidReference {
            context: nested,
            reference,
        } => ValidateError::InvalidReference {
            context: format!("{}: {}", context, nested),
            reference,
        },
        other => ValidateError::InvalidSecurity(format!("{}: {}", context, other)),
    }
}

fn schema_error_message(err: ValidateError) -> String {
    match err {
        ValidateError::InvalidSchema(message) => message,
        other => other.to_string(),
    }
}

fn validate_form_security_references(
    context: String,
    forms: &[Form],
    security_definitions: &BTreeMap<String, SecurityScheme>,
) -> Result<(), ValidateError> {
    for (index, form) in forms.iter().enumerate() {
        if let Some(security) = &form.security {
            validate_security_references(
                format!("{}.forms[{}].security", context, index).as_str(),
                security,
                security_definitions,
            )?;
        }
    }

    Ok(())
}

fn validate_security_references(
    context: &str,
    security: &[String],
    security_definitions: &BTreeMap<String, SecurityScheme>,
) -> Result<(), ValidateError> {
    for reference in security {
        if !security_definitions.contains_key(reference) {
            return Err(ValidateError::InvalidReference {
                context: context.to_string(),
                reference: reference.clone(),
            });
        }
    }

    Ok(())
}

pub struct ThingBuilder {
    thing: Thing,
    errors: Vec<ValidateError>,
}

impl ThingBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            thing: Thing {
                _metadata: Metadata {
                    title: Some(title.into()),
                    ..Default::default()
                },
                security: alloc::vec![],
                security_definitions: BTreeMap::new(),
                _extra_fields: ExtensionMap::default(),
                ..Default::default()
            },
            errors: Vec::new(),
        }
    }

    /// Sets the Things's unique identifier
    pub fn id(mut self, id: &str) -> Self {
        match AbsoluteUri::parse(id) {
            Ok(id) => self.thing.id = Some(id),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("id: {}", id))),
        }
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
        match AbsoluteUri::parse(support) {
            Ok(support) => self.thing.support = Some(support),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("support: {}", support))),
        }
        self
    }

    /// Sets the base URI.
    pub fn base(mut self, base: &str) -> Self {
        match BaseUri::parse(base) {
            Ok(base) => self.thing.base = Some(base),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("base: {}", base))),
        }
        self
    }

    /// Adds a profile URI.
    pub fn profile(mut self, profile: &str) -> Self {
        match AbsoluteUri::parse(profile) {
            Ok(profile) => self
                .thing
                .profile
                .get_or_insert_with(Vec::new)
                .push(profile),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("profile: {}", profile))),
        }
        self
    }

    /// Adds multiple profile URIs.
    pub fn profiles<I, S>(mut self, profiles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for profile in profiles {
            let profile = profile.as_ref();
            match AbsoluteUri::parse(profile) {
                Ok(profile) => self
                    .thing
                    .profile
                    .get_or_insert_with(Vec::new)
                    .push(profile),
                Err(_) => self
                    .errors
                    .push(ValidateError::InvalidUri(format!("profile: {}", profile))),
            }
        }
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
        I: IntoIterator<Item = Link>,
    {
        let mut items: Vec<Link> = links.into_iter().collect();
        self.thing
            .links
            .get_or_insert_with(Vec::new)
            .append(&mut items);
        self
    }

    /// Adds a form.
    pub fn form(mut self, form: impl Into<Form>) -> Self {
        self.thing
            .forms
            .get_or_insert_with(Vec::new)
            .push(form.into());
        self
    }

    /// Adds multiple forms.
    pub fn forms<I>(mut self, forms: I) -> Self
    where
        I: IntoIterator<Item = Form>,
    {
        let mut items: Vec<Form> = forms.into_iter().collect();
        self.thing
            .forms
            .get_or_insert_with(Vec::new)
            .append(&mut items);
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

    /// Adds a security definition reference name.
    pub fn security_name(mut self, name: impl Into<String>) -> Self {
        self.thing.security.push(name.into());
        self
    }

    /// Adds a named security definition.
    pub fn security_definition(
        mut self,
        name: impl Into<String>,
        security: impl Into<SecurityScheme>,
    ) -> Self {
        self.thing
            .security_definitions
            .insert(name.into(), security.into());
        self
    }

    /// Adds a named security definition and references it from `security`.
    pub fn security_named(
        self,
        name: impl Into<String>,
        security: impl Into<SecurityScheme>,
    ) -> Self {
        let name = name.into();
        self.security_definition(name.clone(), security)
            .security_name(name)
    }

    /// Adds the default `nosec` security scheme.
    pub fn nosec(self) -> Self {
        self.security_named("nosec", SecurityScheme::nosec())
    }

    /// Adds a named `basic` security scheme and references it from `security`.
    pub fn basic_security(self, name: impl Into<String>, parameter: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::basic(parameter))
    }

    /// Adds a named `apikey` security scheme and references it from `security`.
    pub fn apikey_security(self, name: impl Into<String>, parameter: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::apikey(parameter))
    }

    /// Adds a named `digest` security scheme and references it from `security`.
    pub fn digest_security(self, name: impl Into<String>, parameter: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::digest(parameter))
    }

    /// Adds a named `bearer` security scheme and references it from `security`.
    pub fn bearer_security(self, name: impl Into<String>, parameter: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::bearer(parameter))
    }

    /// Adds a named `bearer` security scheme with an authorization endpoint.
    pub fn bearer_authorization_security(
        mut self,
        name: impl Into<String>,
        parameter: impl Into<String>,
        authorization: impl Into<String>,
    ) -> Self {
        let name = name.into();
        match SecurityScheme::bearer_authorization(parameter, authorization) {
            Ok(security) => self = self.security_named(name, security),
            Err(err) => self.errors.push(err),
        }
        self
    }

    /// Adds a named `psk` security scheme and references it from `security`.
    pub fn psk_security(self, name: impl Into<String>, identity: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::psk(identity))
    }

    /// Adds a named `combo` security scheme with `oneOf` references.
    pub fn combo_one_of_security<I, S>(self, name: impl Into<String>, schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.security_named(name, SecurityScheme::combo_one_of(schemes))
    }

    /// Adds a named `combo` security scheme with `allOf` references.
    pub fn combo_all_of_security<I, S>(self, name: impl Into<String>, schemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.security_named(name, SecurityScheme::combo_all_of(schemes))
    }

    /// Adds a named OAuth2 authorization-code flow security scheme.
    pub fn oauth2_code_security(
        mut self,
        name: impl Into<String>,
        authorization: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        let name = name.into();
        match SecurityScheme::oauth2_code(authorization, token) {
            Ok(security) => self = self.security_named(name, security),
            Err(err) => self.errors.push(err),
        }
        self
    }

    /// Adds a named OAuth2 client credentials flow security scheme.
    pub fn oauth2_client_security(self, name: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::oauth2_client())
    }

    /// Adds a named OAuth2 device flow security scheme.
    pub fn oauth2_device_security(self, name: impl Into<String>) -> Self {
        self.security_named(name, SecurityScheme::oauth2_device())
    }

    /// Adds multiple security names.
    pub fn securities<I, S>(mut self, securities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SecurityScheme>,
    {
        for s in securities {
            self = self.security(s);
        }
        self
    }

    /// Adds a schema definition.
    pub fn schema_definition(
        mut self,
        name: impl Into<String>,
        schema: impl Into<DataSchema>,
    ) -> Self {
        let schema_definitions = self
            .thing
            .schema_definitions
            .get_or_insert_with(BTreeMap::new);
        schema_definitions.insert(name.into(), schema.into());
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: impl Into<DataSchema>) -> Self {
        let uri_variables = self.thing.uri_variables.get_or_insert_with(BTreeMap::new);
        uri_variables.insert(name.into(), schema.into());
        self
    }

    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.thing._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.thing._extra_fields.insert(key.into(), value);
        self
    }

    /// Builds and returns the `Thing` instance.
    pub fn build(self) -> Result<Thing, ValidateError> {
        if let Some(error) = self.errors.into_iter().next() {
            return Err(error);
        }
        self.thing.validate()?;
        Ok(self.thing)
    }
}

impl MetadataHelper for Thing {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self._metadata
    }
}
