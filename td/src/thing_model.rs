use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{OneOrMany, serde_as};

use crate::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    context::Context,
    data_schema::DataSchema,
    data_type::{
        AbsoluteUri, AdditionalExpectedResponse, BaseUri, ExtensionMap, FormHref, METADATA_KEYS,
        Metadata, MetadataHelper, ThingModelVersionInfo,
    },
    form::Form,
    link::Link,
    security_scheme::SecurityScheme,
    validate::{
        HasAdditionalResponses, Validate, ValidateError, ValidationLevel, parse_uri_field,
        validate_context_at_profile_level, validate_form_response_references, validate_schema_map,
        validate_security_references,
    },
};

/// Deserialize adapter for one-or-many string lists (`security`, `scopes`).
#[serde_as]
#[derive(Deserialize)]
struct StringListField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<String>>);

/// Deserialize adapter for one-or-many operation lists (`op`).
#[serde_as]
#[derive(Deserialize)]
struct OperationListField(
    #[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<crate::data_type::Operation>>,
);

/// Deserialize adapter for one-or-many profile URIs.
#[serde_as]
#[derive(Deserialize)]
struct ProfileField(#[serde_as(as = "Option<OneOrMany<_>>")] Option<Vec<AbsoluteUri>>);

/// A reusable WoT Thing Model template.
///
/// Thing Models describe a class of Things and can be instantiated into concrete
/// Thing Descriptions by later tooling. This crate stores TM structure and
/// validates local constraints, but does not fetch referenced models or generate
/// protocol-specific forms.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThingModel {
    /// JSON-LD context for the model.
    pub context: Context,

    /// Model identifier.
    pub id: Option<AbsoluteUri>,

    /// Shared metadata, including the `@type` entry that identifies a TM.
    pub _metadata: Metadata,

    /// Thing Model version information.
    pub version: Option<ThingModelVersionInfo>,

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
    pub forms: Option<Vec<ThingModelForm>>,

    /// Optional model-level security names.
    pub security: Option<Vec<String>>,

    /// Optional named security configurations.
    pub security_definitions: Option<BTreeMap<String, SecurityScheme>>,

    /// WoT Profile identifiers associated with generated TDs.
    pub profile: Option<Vec<AbsoluteUri>>,

    /// Named data schemas reusable by the model.
    pub schema_definitions: Option<BTreeMap<String, DataSchema>>,

    /// URI template variables declared by the model.
    pub uri_variables: Option<BTreeMap<String, DataSchema>>,

    /// JSON Pointer references to interaction models that are optional in TD
    /// instances generated from this model.
    pub tm_optional: Option<Vec<String>>,

    pub _extra_fields: ExtensionMap,
}

impl<'de> Deserialize<'de> for ThingModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let context = crate::flat::take_required(&mut map, "@context")?;
        let id = crate::flat::take(&mut map, "id")?;
        let metadata = crate::flat::drain_substruct::<Metadata, D::Error>(&mut map, METADATA_KEYS)?;
        let version = crate::flat::take(&mut map, "version")?;
        let support = crate::flat::take(&mut map, "support")?;
        let base = crate::flat::take(&mut map, "base")?;
        let properties = crate::flat::take(&mut map, "properties")?;
        let actions = crate::flat::take(&mut map, "actions")?;
        let events = crate::flat::take(&mut map, "events")?;
        let links = crate::flat::take(&mut map, "links")?;
        let forms = crate::flat::take(&mut map, "forms")?;
        let security = crate::flat::take::<StringListField, D::Error>(&mut map, "security")?
            .and_then(|field| field.0);
        let security_definitions = crate::flat::take(&mut map, "securityDefinitions")?;
        let profile = crate::flat::take::<ProfileField, D::Error>(&mut map, "profile")?
            .and_then(|field| field.0);
        let schema_definitions = crate::flat::take(&mut map, "schemaDefinitions")?;
        let uri_variables = crate::flat::take(&mut map, "uriVariables")?;
        let tm_optional = crate::flat::take(&mut map, "tm:optional")?;
        Ok(ThingModel {
            context,
            id,
            _metadata: metadata,
            version,
            support,
            base,
            properties,
            actions,
            events,
            links,
            forms,
            security,
            security_definitions,
            profile,
            schema_definitions,
            uri_variables,
            tm_optional,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for ThingModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("@context", &self.context)?;
        if let Some(id) = &self.id {
            map.serialize_entry("id", id)?;
        }
        self._metadata.serialize_into(&mut map)?;
        if let Some(version) = &self.version {
            map.serialize_entry("version", version)?;
        }
        if let Some(support) = &self.support {
            map.serialize_entry("support", support)?;
        }
        if let Some(base) = &self.base {
            map.serialize_entry("base", base)?;
        }
        if let Some(properties) = &self.properties {
            map.serialize_entry("properties", properties)?;
        }
        if let Some(actions) = &self.actions {
            map.serialize_entry("actions", actions)?;
        }
        if let Some(events) = &self.events {
            map.serialize_entry("events", events)?;
        }
        if let Some(links) = &self.links {
            map.serialize_entry("links", links)?;
        }
        if let Some(forms) = &self.forms {
            map.serialize_entry("forms", forms)?;
        }
        if let Some(security) = &self.security {
            map.serialize_entry("security", &crate::flat::OneOrManyRef(security))?;
        }
        if let Some(security_definitions) = &self.security_definitions {
            map.serialize_entry("securityDefinitions", security_definitions)?;
        }
        if let Some(profile) = &self.profile {
            map.serialize_entry("profile", &crate::flat::OneOrManyRef(profile))?;
        }
        if let Some(schema_definitions) = &self.schema_definitions {
            map.serialize_entry("schemaDefinitions", schema_definitions)?;
        }
        if let Some(uri_variables) = &self.uri_variables {
            map.serialize_entry("uriVariables", uri_variables)?;
        }
        if let Some(tm_optional) = &self.tm_optional {
            map.serialize_entry("tm:optional", tm_optional)?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// A Thing Model form template.
///
/// Thing Model forms may omit `href` because they describe reusable templates
/// that are instantiated into concrete Thing Description forms later.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThingModelForm {
    /// Optional target IRI of the resource or service.
    pub href: Option<FormHref>,

    /// Media type of data sent or received.
    pub content_type: String,

    /// Content coding, such as `gzip`.
    pub content_coding: Option<String>,

    /// Reference to a security scheme definition by name.
    pub security: Option<Vec<String>>,

    /// Scope names required for OAuth2.
    pub scopes: Option<Vec<String>>,

    /// Metadata of the primary response.
    pub response: Option<crate::data_type::ExpectedResponse>,

    /// Additional expected responses.
    pub additional_responses: Option<Vec<crate::data_type::AdditionalExpectedResponse>>,

    /// Protocol-specific subprotocol hint.
    pub subprotocol: Option<String>,

    /// Intended operations for the form template.
    pub op: Option<Vec<crate::data_type::Operation>>,

    pub _extra_fields: ExtensionMap,
}

impl<'de> Deserialize<'de> for ThingModelForm {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = crate::flat::deserialize_map(deserializer)?;
        let href = crate::flat::take(&mut map, "href")?;
        let content_type = crate::flat::take::<String, D::Error>(&mut map, "contentType")?
            .unwrap_or_else(default_content_type);
        let content_coding = crate::flat::take(&mut map, "contentCoding")?;
        let security = crate::flat::take::<StringListField, D::Error>(&mut map, "security")?
            .and_then(|field| field.0);
        let scopes = crate::flat::take::<StringListField, D::Error>(&mut map, "scopes")?
            .and_then(|field| field.0);
        let response = crate::flat::take(&mut map, "response")?;
        let additional_responses = crate::flat::take(&mut map, "additionalResponses")?;
        let subprotocol = crate::flat::take(&mut map, "subprotocol")?;
        let op = crate::flat::take::<OperationListField, D::Error>(&mut map, "op")?
            .and_then(|field| field.0);
        Ok(ThingModelForm {
            href,
            content_type,
            content_coding,
            security,
            scopes,
            response,
            additional_responses,
            subprotocol,
            op,
            _extra_fields: crate::flat::into_extras(map),
        })
    }
}

impl Serialize for ThingModelForm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if let Some(href) = &self.href {
            map.serialize_entry("href", href)?;
        }
        if !is_default_content_type(&self.content_type) {
            map.serialize_entry("contentType", &self.content_type)?;
        }
        if let Some(content_coding) = &self.content_coding {
            map.serialize_entry("contentCoding", content_coding)?;
        }
        if let Some(security) = &self.security {
            map.serialize_entry("security", &crate::flat::OneOrManyRef(security))?;
        }
        if let Some(scopes) = &self.scopes {
            map.serialize_entry("scopes", &crate::flat::OneOrManyRef(scopes))?;
        }
        if let Some(response) = &self.response {
            map.serialize_entry("response", response)?;
        }
        if let Some(additional_responses) = &self.additional_responses {
            map.serialize_entry("additionalResponses", additional_responses)?;
        }
        if let Some(subprotocol) = &self.subprotocol {
            map.serialize_entry("subprotocol", subprotocol)?;
        }
        if let Some(op) = &self.op {
            map.serialize_entry("op", &crate::flat::OneOrManyRef(op))?;
        }
        for (key, value) in &self._extra_fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl From<Form> for ThingModelForm {
    fn from(value: Form) -> Self {
        Self {
            href: Some(value.href),
            content_type: value.content_type,
            content_coding: value.content_coding,
            security: value.security,
            scopes: value.scopes,
            response: value.response,
            additional_responses: value.additional_responses,
            subprotocol: value.subprotocol,
            op: value.op,
            _extra_fields: value._extra_fields,
        }
    }
}

const DEFAULT_CONTENT_TYPE: &str = "application/json";

fn default_content_type() -> String {
    String::from(DEFAULT_CONTENT_TYPE)
}

fn is_default_content_type(content_type: &str) -> bool {
    content_type == DEFAULT_CONTENT_TYPE
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

        // Profile/Full: @context must contain a standard WoT context URI.
        validate_context_at_profile_level(&self.context, level)?;

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
                validate_form_response_references(
                    format!("properties.{}.forms", name).as_str(),
                    &property._interaction.forms,
                    self.schema_definitions.as_ref(),
                    level,
                )?;
            }
        }

        if let Some(actions) = &self.actions {
            for (name, action) in actions {
                action.validate_with_level(level).map_err(|err| {
                    ValidateError::InvalidSchema(format!("actions.{}: {}", name, err))
                })?;
                validate_form_response_references(
                    format!("actions.{}.forms", name).as_str(),
                    &action._interaction.forms,
                    self.schema_definitions.as_ref(),
                    level,
                )?;
            }
        }

        if let Some(events) = &self.events {
            for (name, event) in events {
                event.validate_with_level(level).map_err(|err| {
                    ValidateError::InvalidSchema(format!("events.{}: {}", name, err))
                })?;
                validate_form_response_references(
                    format!("events.{}.forms", name).as_str(),
                    &event._interaction.forms,
                    self.schema_definitions.as_ref(),
                    level,
                )?;
            }
        }

        if let Some(forms) = &self.forms {
            validate_form_response_references(
                "forms",
                forms,
                self.schema_definitions.as_ref(),
                level,
            )?;
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

impl HasAdditionalResponses for ThingModelForm {
    fn additional_responses(&self) -> Option<&[AdditionalExpectedResponse]> {
        self.additional_responses.as_deref()
    }
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
            .is_some_and(|items| items.contains_key(name)),
        "actions" => model
            .actions
            .as_ref()
            .is_some_and(|items| items.contains_key(name)),
        "events" => model
            .events
            .as_ref()
            .is_some_and(|items| items.contains_key(name)),
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
                version: None,
                _extra_fields: ExtensionMap::default(),
                ..Default::default()
            },
            errors: Vec::new(),
        }
    }

    /// Sets the model identifier.
    pub fn id(mut self, id: &str) -> Self {
        if let Some(uri) = parse_uri_field("id", id, AbsoluteUri::parse, &mut self.errors) {
            self.model.id = Some(uri);
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
        if let Some(uri) = parse_uri_field("support", support, AbsoluteUri::parse, &mut self.errors)
        {
            self.model.support = Some(uri);
        }
        self
    }

    /// Sets the Thing Model version information.
    pub fn version(mut self, version: ThingModelVersionInfo) -> Self {
        self.model.version = Some(version);
        self
    }

    /// Sets the base URI.
    pub fn base(mut self, base: &str) -> Self {
        if let Some(uri) = parse_uri_field("base", base, BaseUri::parse, &mut self.errors) {
            self.model.base = Some(uri);
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
    pub fn form(mut self, form: impl Into<ThingModelForm>) -> Self {
        self.model
            .forms
            .get_or_insert_with(Vec::new)
            .push(form.into());
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
        crate::validate::collected_errors(self.errors)?;
        self.model.validate()?;
        Ok(self.model)
    }
}

impl MetadataHelper for ThingModelBuilder {
    fn metadata(&mut self) -> &mut Metadata {
        &mut self.model._metadata
    }
}
