use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as, skip_serializing_none};

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::Context,
    data_schema::DataSchema,
    data_type::{AbsoluteUri, BaseUri, ExtensionMap, Metadata, MetadataHelper},
    form::Form,
    link::Link,
    security_scheme::SecurityScheme,
    validate::{Validate, ValidateError, ValidationLevel},
};

/// A reusable WoT Thing Model template.
///
/// Thing Models describe a class of Things and can be instantiated into concrete
/// Thing Descriptions by later tooling. This crate stores TM structure and
/// validates local constraints, but does not fetch referenced models or generate
/// protocol-specific forms.
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThingModel {
    /// JSON-LD context for the model.
    #[serde(rename = "@context")]
    pub context: Context,

    /// Model identifier.
    pub id: Option<AbsoluteUri>,

    /// Shared metadata, including the `@type` entry that identifies a TM.
    #[serde(flatten)]
    pub _metadata: Metadata,

    /// Support contact or documentation URI.
    pub support: Option<AbsoluteUri>,

    /// Optional base URI template used by instantiated TDs.
    pub base: Option<BaseUri>,

    /// Property interaction models.
    pub properties: Option<BTreeMap<String, PropertyAffordance>>,

    /// Action interaction models.
    pub actions: Option<BTreeMap<String, ActionAffordance>>,

    /// Event interaction models.
    pub events: Option<BTreeMap<String, EventAffordance>>,

    /// Links, including `rel: "tm:extends"` relationships to parent models.
    pub links: Option<Vec<Link>>,

    /// Optional form templates.
    pub forms: Option<Vec<Form>>,

    /// Optional model-level security names.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub security: Option<Vec<String>>,

    /// Optional named security configurations.
    pub security_definitions: Option<BTreeMap<String, SecurityScheme>>,

    /// WoT Profile identifiers associated with generated TDs.
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub profile: Option<Vec<AbsoluteUri>>,

    /// Named data schemas reusable by the model.
    pub schema_definitions: Option<BTreeMap<String, DataSchema>>,

    /// URI template variables declared by the model.
    pub uri_variables: Option<BTreeMap<String, DataSchema>>,

    /// JSON Pointer references to interaction models that are optional in TD
    /// instances generated from this model.
    #[serde(rename = "tm:optional")]
    pub tm_optional: Option<Vec<String>>,

    #[serde(flatten)]
    pub _extra_fields: ExtensionMap,
}

impl ThingModel {
    /// Creates a new ThingModel builder with `@type` set to `tm:ThingModel`.
    pub fn builder(title: impl Into<String>) -> ThingModelBuilder {
        ThingModelBuilder::new(title)
    }
}

impl Validate for ThingModel {
    fn validate_with_level(&self, level: ValidationLevel) -> Result<(), ValidateError> {
        if matches!(level, ValidationLevel::Minimal) {
            return Ok(());
        }

        if self._metadata.title.as_deref().unwrap_or("").is_empty() {
            return Err(ValidateError::MissingRequiredField("title".to_string()));
        }

        validate_thing_model_type(&self._metadata.tags)?;

        if let Some(security) = &self.security {
            if security.is_empty() {
                return Err(ValidateError::MissingRequiredField("security".to_string()));
            }

            let Some(security_definitions) = &self.security_definitions else {
                return Err(ValidateError::MissingRequiredField(
                    "securityDefinitions".to_string(),
                ));
            };

            validate_security_references("ThingModel.security", security, security_definitions)?;
        }

        if let Some(security_definitions) = &self.security_definitions {
            for (name, scheme) in security_definitions {
                scheme.validate_with_level(level).map_err(|err| {
                    ValidateError::InvalidSecurity(format!("securityDefinitions.{}: {}", name, err))
                })?;
                scheme.validate_references(
                    format!("securityDefinitions.{}", name).as_str(),
                    security_definitions,
                )?;
            }
        }

        validate_schema_map("schemaDefinitions", self.schema_definitions.as_ref(), level)?;
        validate_schema_map("uriVariables", self.uri_variables.as_ref(), level)?;

        if let Some(properties) = &self.properties {
            for (name, property) in properties {
                property.validate_with_level(level).map_err(|err| {
                    ValidateError::InvalidSchema(format!("properties.{}: {}", name, err))
                })?;
            }
        }

        if let Some(actions) = &self.actions {
            for (name, action) in actions {
                action.validate_with_level(level).map_err(|err| {
                    ValidateError::InvalidSchema(format!("actions.{}: {}", name, err))
                })?;
            }
        }

        if let Some(events) = &self.events {
            for (name, event) in events {
                event.validate_with_level(level).map_err(|err| {
                    ValidateError::InvalidSchema(format!("events.{}: {}", name, err))
                })?;
            }
        }

        if let Some(tm_optional) = &self.tm_optional {
            validate_tm_optional(self, tm_optional)?;
        }

        self._extra_fields.validate_with_level(level)?;

        Ok(())
    }
}

fn validate_thing_model_type(tags: &Option<Vec<String>>) -> Result<(), ValidateError> {
    let has_tm_type = tags
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .any(|tag| tag == "tm:ThingModel");

    if has_tm_type {
        return Ok(());
    }

    Err(ValidateError::MissingRequiredField(
        "@type: tm:ThingModel".to_string(),
    ))
}

fn validate_schema_map(
    context: &str,
    schemas: Option<&BTreeMap<String, DataSchema>>,
    level: ValidationLevel,
) -> Result<(), ValidateError> {
    let Some(schemas) = schemas else {
        return Ok(());
    };

    for (name, schema) in schemas {
        schema.validate_with_level(level).map_err(|err| {
            ValidateError::InvalidSchema(format!("{}.{}: {}", context, name, err))
        })?;
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

fn validate_tm_optional(model: &ThingModel, pointers: &[String]) -> Result<(), ValidateError> {
    for pointer in pointers {
        let trimmed = pointer.strip_prefix('#').unwrap_or(pointer.as_str());
        let segments =
            trimmed
                .strip_prefix('/')
                .ok_or_else(|| ValidateError::InvalidReference {
                    context: "ThingModel.tm:optional".to_string(),
                    reference: pointer.clone(),
                })?;

        let mut parts = segments.split('/');
        let Some(collection) = parts.next() else {
            return Err(ValidateError::InvalidReference {
                context: "ThingModel.tm:optional".to_string(),
                reference: pointer.clone(),
            });
        };
        let Some(name) = parts.next() else {
            return Err(ValidateError::InvalidReference {
                context: "ThingModel.tm:optional".to_string(),
                reference: pointer.clone(),
            });
        };

        if parts.next().is_some() || !has_interaction(model, collection, name) {
            return Err(ValidateError::InvalidReference {
                context: "ThingModel.tm:optional".to_string(),
                reference: pointer.clone(),
            });
        }
    }

    Ok(())
}

fn has_interaction(model: &ThingModel, collection: &str, name: &str) -> bool {
    match collection {
        "properties" => model
            .properties
            .as_ref()
            .map_or(false, |items| items.contains_key(name)),
        "actions" => model
            .actions
            .as_ref()
            .map_or(false, |items| items.contains_key(name)),
        "events" => model
            .events
            .as_ref()
            .map_or(false, |items| items.contains_key(name)),
        _ => false,
    }
}

pub struct ThingModelBuilder {
    model: ThingModel,
    errors: Vec<ValidateError>,
}

impl ThingModelBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            model: ThingModel {
                _metadata: Metadata {
                    tags: Some(alloc::vec!["tm:ThingModel".to_string()]),
                    title: Some(title.into()),
                    ..Default::default()
                },
                _extra_fields: ExtensionMap::default(),
                ..Default::default()
            },
            errors: Vec::new(),
        }
    }

    /// Sets the model identifier.
    pub fn id(mut self, id: &str) -> Self {
        match AbsoluteUri::parse(id) {
            Ok(id) => self.model.id = Some(id),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("id: {}", id))),
        }
        self
    }

    /// Sets the context.
    pub fn context(mut self, context: impl Into<Context>) -> Self {
        self.model.context = context.into();
        self
    }

    /// Sets the support URI.
    pub fn support(mut self, support: &str) -> Self {
        match AbsoluteUri::parse(support) {
            Ok(support) => self.model.support = Some(support),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("support: {}", support))),
        }
        self
    }

    /// Sets the base URI.
    pub fn base(mut self, base: &str) -> Self {
        match BaseUri::parse(base) {
            Ok(base) => self.model.base = Some(base),
            Err(_) => self
                .errors
                .push(ValidateError::InvalidUri(format!("base: {}", base))),
        }
        self
    }

    /// Adds a property model.
    pub fn property(mut self, name: impl Into<String>, property: PropertyAffordance) -> Self {
        self.model
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), property);
        self
    }

    /// Adds an action model.
    pub fn action(mut self, name: impl Into<String>, action: ActionAffordance) -> Self {
        self.model
            .actions
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), action);
        self
    }

    /// Adds an event model.
    pub fn event(mut self, name: impl Into<String>, event: EventAffordance) -> Self {
        self.model
            .events
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), event);
        self
    }

    /// Adds a link.
    pub fn link(mut self, link: Link) -> Self {
        self.model.links.get_or_insert_with(Vec::new).push(link);
        self
    }

    /// Adds a form template.
    pub fn form(mut self, form: Form) -> Self {
        self.model.forms.get_or_insert_with(Vec::new).push(form);
        self
    }

    /// Adds a security name.
    pub fn security_name(mut self, name: impl Into<String>) -> Self {
        self.model
            .security
            .get_or_insert_with(Vec::new)
            .push(name.into());
        self
    }

    /// Adds a named security definition.
    pub fn security_definition(
        mut self,
        name: impl Into<String>,
        scheme: impl Into<SecurityScheme>,
    ) -> Self {
        self.model
            .security_definitions
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), scheme.into());
        self
    }

    /// Adds a named security definition and references it from `security`.
    pub fn security_named(
        self,
        name: impl Into<String>,
        scheme: impl Into<SecurityScheme>,
    ) -> Self {
        let name = name.into();
        self.security_definition(name.clone(), scheme)
            .security_name(name)
    }

    /// Adds the default `nosec` security scheme and reference.
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

    /// Adds a schema definition.
    pub fn schema_definition(
        mut self,
        name: impl Into<String>,
        schema: impl Into<DataSchema>,
    ) -> Self {
        self.model
            .schema_definitions
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), schema.into());
        self
    }

    /// Adds a URI variable.
    pub fn uri_variable(mut self, name: impl Into<String>, schema: impl Into<DataSchema>) -> Self {
        self.model
            .uri_variables
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), schema.into());
        self
    }

    /// Marks an interaction model as optional using a JSON Pointer.
    pub fn optional(mut self, pointer: impl Into<String>) -> Self {
        self.model
            .tm_optional
            .get_or_insert_with(Vec::new)
            .push(pointer.into());
        self
    }

    /// Sets extension fields.
    pub fn extra_fields(mut self, extra_fields: impl Into<ExtensionMap>) -> Self {
        self.model._extra_fields.extend(extra_fields.into());
        self
    }

    /// Adds an extension field.
    pub fn extra_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.model._extra_fields.insert(key.into(), value);
        self
    }

    /// Builds and returns the `ThingModel` instance.
    pub fn build(self) -> Result<ThingModel, ValidateError> {
        if let Some(error) = self.errors.into_iter().next() {
            return Err(error);
        }
        self.model.validate()?;
        Ok(self.model)
    }
}

impl MetadataHelper for ThingModelBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.model._metadata
    }
}
