//! Common WoT facts over the admitted loan. No raw TD, context or schema parser
//! is exposed. All recursive schema handles point into the prepared arena.
use super::*;
use crate::{
    data_schema::DataSchema,
    data_type::MultiLanguage,
    security_scheme::{Qop, SecurityLocation, SecurityScheme},
};

#[cfg(any(feature = "validated-thing", test))]
const JS: &str = "https://www.w3.org/2019/wot/json-schema#";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticAccessError {
    OutsidePreparedScope,
    UnsupportedDependency,
}

#[derive(Clone, Copy)]
pub struct MetadataFacts<'a> {
    pub title: Option<&'a str>,
    pub titles: Option<&'a MultiLanguage>,
    pub description: Option<&'a str>,
    pub descriptions: Option<&'a MultiLanguage>,
}
impl<'a> From<&'a crate::data_type::Metadata> for MetadataFacts<'a> {
    fn from(value: &'a crate::data_type::Metadata) -> Self {
        Self {
            title: value.title.as_deref(),
            titles: value.titles.as_ref(),
            description: value.description.as_deref(),
            descriptions: value.descriptions.as_ref(),
        }
    }
}

/// Values in const/default/enum are JSON payload literals. In particular an
/// object member named @context in a literal is data, never a context to expand.
#[derive(Clone, Copy)]
pub struct SchemaFacts<'a> {
    pub declared_type: Option<&'a str>,
    pub read_only: bool,
    pub write_only: bool,
    pub unit: Option<&'a str>,
    pub format: Option<&'a str>,
    pub constant: Option<&'a Value>,
    pub default: Option<&'a Value>,
    pub enumeration: Option<&'a [Value]>,
    pub metadata: MetadataFacts<'a>,
    pub constraints: SchemaConstraints<'a>,
}

/// Typed standard facets; an omitted type remains None in declared_type.
/// Facets retained by the upstream generic-schema representation in its extras
/// remain available by expanded name through SchemaView::semantics().
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SchemaConstraints<'a> {
    Array {
        min_items: Option<u32>,
        max_items: Option<u32>,
    },
    Boolean,
    Number {
        minimum: Option<f64>,
        exclusive_minimum: Option<f64>,
        maximum: Option<f64>,
        exclusive_maximum: Option<f64>,
        multiple_of: Option<f64>,
    },
    Integer {
        minimum: Option<i64>,
        exclusive_minimum: Option<i64>,
        maximum: Option<i64>,
        exclusive_maximum: Option<i64>,
        multiple_of: Option<i64>,
    },
    Object {
        required: Option<&'a [String]>,
    },
    String {
        min_length: Option<u32>,
        max_length: Option<u32>,
        pattern: Option<&'a str>,
        content_encoding: Option<&'a str>,
        content_media_type: Option<&'a str>,
    },
    Null,
}

#[derive(Clone, Copy)]
/// Schema handles cannot outlive the prepared semantic input.
/// ```compile_fail
/// use clinkz_wot_td::semantic::{ResolvedSemanticDocument, SchemaView};
/// fn escape(document: &ResolvedSemanticDocument<'_>) -> SchemaView<'static> {
///     document.property().schema
/// }
/// ```
pub struct SchemaView<'a>(SemanticNode<'a>);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaItemsMode {
    Uniform,
    Tuple,
}
impl<'a> SchemaView<'a> {
    /// Declared scoped extensions and class assertions, excluding the typed
    /// schema branches appended by TD. An extension alias cannot masquerade as
    /// one of those branches. This query borrows already resolved nodes.
    pub fn extension_semantics(self) -> impl Iterator<Item = SemanticNode<'a>> + 'a {
        let context = self.0.nodes[self.0.index].schema.unwrap().context();
        let count = context._extra_fields.len()
            - usize::from(context._extra_fields.contains_key("@context"))
            + usize::from(context._metadata.tags.is_some());
        self.0.children().take(count)
    }
    pub fn semantics(self) -> SemanticNode<'a> {
        self.0
    }
    pub fn facts(self) -> SchemaFacts<'a> {
        let schema = self.0.nodes[self.0.index].schema.unwrap();
        let common = schema.context();
        let constraints = match schema {
            DataSchema::Array(s) => SchemaConstraints::Array {
                min_items: s.min_items,
                max_items: s.max_items,
            },
            DataSchema::Boolean(_) => SchemaConstraints::Boolean,
            DataSchema::Number(s) => SchemaConstraints::Number {
                minimum: s.minimum,
                exclusive_minimum: s.exclusive_minimum,
                maximum: s.maximum,
                exclusive_maximum: s.exclusive_maximum,
                multiple_of: s.multiple_of,
            },
            DataSchema::Integer(s) => SchemaConstraints::Integer {
                minimum: s.minimum,
                exclusive_minimum: s.exclusive_minimum,
                maximum: s.maximum,
                exclusive_maximum: s.exclusive_maximum,
                multiple_of: s.multiple_of,
            },
            DataSchema::Object(s) => SchemaConstraints::Object {
                required: s.required.as_deref(),
            },
            DataSchema::String(s) => SchemaConstraints::String {
                min_length: s.min_length,
                max_length: s.max_length,
                pattern: s.pattern.as_deref(),
                content_encoding: s.content_encoding.as_deref(),
                content_media_type: s.content_media_type.as_deref(),
            },
            DataSchema::Null(_) => SchemaConstraints::Null,
        };
        SchemaFacts {
            declared_type: common.data_type.as_deref(),
            read_only: common.read_only,
            write_only: common.write_only,
            unit: common.unit.as_deref(),
            format: common.format.as_deref(),
            constant: common.constant.as_ref(),
            default: common.default.as_ref(),
            enumeration: common.enumerate.as_deref(),
            metadata: (&common._metadata).into(),
            constraints,
        }
    }
    /// None means the field was omitted; Some(empty iterator) is explicit empty.
    pub fn properties(self) -> Option<impl Iterator<Item = (&'a str, Self)> + 'a> {
        self.branch("properties").map(|root| {
            root.children().map(|child| {
                let SemanticName::Unmapped(name) = child.name() else {
                    unreachable!()
                };
                (name, Self(child))
            })
        })
    }
    pub fn items(self) -> Option<impl Iterator<Item = Self> + 'a> {
        self.branch("items").map(|root| root.children().map(Self))
    }
    pub fn items_mode(self) -> Option<SchemaItemsMode> {
        let DataSchema::Array(schema) = self.0.nodes[self.0.index].schema.unwrap() else {
            return None;
        };
        let items = schema.items.as_ref()?;
        Some(if schema.items_are_tuple || items.len() != 1 {
            SchemaItemsMode::Tuple
        } else {
            SchemaItemsMode::Uniform
        })
    }
    pub fn one_of(self) -> Option<impl Iterator<Item = Self> + 'a> {
        self.branch("oneOf").map(|root| root.children().map(Self))
    }
    fn branch(self, field: &str) -> Option<SemanticNode<'a>> {
        let schema = self.0.nodes[self.0.index].schema.unwrap();
        let present = match (field, schema) {
            ("properties", DataSchema::Object(s)) => s.properties.is_some(),
            ("items", DataSchema::Array(s)) => s.items.is_some(),
            ("oneOf", _) => schema.context().one_of.is_some(),
            _ => false,
        };
        if !present {
            return None;
        }
        // Structural branches are emitted after extensions. An extension alias
        // with the same IRI cannot replace the admitted typed schema branch.
        self.0
            .children()
            .filter(|node| node.name().is(format_iri(field)))
            .last()
    }
}
// Static strings avoid allocation in queries, including child lookup.
fn format_iri(field: &str) -> &'static str {
    match field {
        "properties" => "https://www.w3.org/2019/wot/json-schema#properties",
        "items" => "https://www.w3.org/2019/wot/json-schema#items",
        "oneOf" => "https://www.w3.org/2019/wot/json-schema#oneOf",
        _ => unreachable!(),
    }
}

pub struct PropertyFacts<'a> {
    pub name: &'a str,
    pub observable: bool,
    pub schema: SchemaView<'a>,
}
pub struct ActionFacts<'a> {
    pub name: &'a str,
    pub metadata: MetadataFacts<'a>,
    pub safe: bool,
    pub idempotent: bool,
    /// None makes no claim of synchronous completion.
    pub synchronous: Option<bool>,
}
pub struct EventFacts<'a> {
    pub name: &'a str,
    pub metadata: MetadataFacts<'a>,
}
#[derive(Clone, Copy)]
pub enum UriVariableScope {
    Thing,
    Property,
    Action,
    Event,
}
pub struct ResponseFacts<'a> {
    pub declared_content_type: Option<&'a str>,
    pub effective_content_type: &'a str,
    pub success: bool,
    pub schema: Option<&'a str>,
    pub semantics: SemanticNode<'a>,
}
/// Standard shared NoSec facts, borrowing the prepared admitted source.
/// Semantic tags and resolved extensions are retained in `semantics`.
pub struct SecurityFacts<'a> {
    pub scheme: &'a str,
    /// Server this configuration applies to; absence means the endpoint.
    /// Preserving this target does not confer proxy execution support.
    pub proxy: Option<&'a str>,
    pub description: Option<&'a str>,
    pub descriptions: Option<&'a MultiLanguage>,
    pub semantics: SemanticNode<'a>,
    pub details: SecurityDetails<'a>,
}

/// All typed TD 1.1 security operands, including defaulted fields. The
/// selected provider interprets these shared facts; credentials and current
/// authorization never enter the semantic loan. Unknown extension schemes
/// retain their resolved associations in `SecurityFacts::semantics`.
#[derive(Clone, Copy, Debug)]
pub enum SecurityDetails<'a> {
    NoSec,
    Auto,
    Combo {
        one_of: &'a [String],
        all_of: &'a [String],
    },
    Basic {
        name: Option<&'a str>,
        location: &'a SecurityLocation,
    },
    Digest {
        name: Option<&'a str>,
        location: &'a SecurityLocation,
        qop: Qop,
    },
    ApiKey {
        name: Option<&'a str>,
        location: &'a SecurityLocation,
    },
    Bearer {
        name: Option<&'a str>,
        location: &'a SecurityLocation,
        authorization: Option<&'a str>,
        alg: &'a str,
        format: &'a str,
    },
    Psk {
        identity: Option<&'a str>,
    },
    OAuth2 {
        flow: &'a str,
        authorization: Option<&'a str>,
        token: Option<&'a str>,
        refresh: Option<&'a str>,
        scopes: Option<&'a [String]>,
    },
    Extension,
}

impl<'a> ResolvedSemanticDocument<'a> {
    pub fn thing_id(&self) -> Option<&str> {
        self.selected
            .as_ref()
            .unwrap()
            .thing
            .id
            .as_ref()
            .map(|id| id.as_str())
    }
    pub fn thing_metadata(&self) -> MetadataFacts<'_> {
        (&self.selected.as_ref().unwrap().thing._metadata).into()
    }
    pub fn property(&self) -> PropertyFacts<'_> {
        let selected = self.selected.as_ref().unwrap();
        let FormOwner::Property(name) = selected.owner else {
            panic!("not a Property selection")
        };
        let property = &selected.thing.properties.as_ref().unwrap()[name];
        PropertyFacts {
            name,
            observable: property.observable,
            schema: SchemaView(self.scope(SemanticScope::Property(name)).unwrap()),
        }
    }
    pub fn action(&self) -> Option<ActionFacts<'_>> {
        let selected = self.selected.as_ref()?;
        let FormOwner::Action(name) = selected.owner else {
            return None;
        };
        let action = &selected.thing.actions.as_ref()?[name];
        Some(ActionFacts {
            name,
            metadata: (&action._metadata).into(),
            safe: action.safe,
            idempotent: action.idempotent,
            synchronous: action.synchronous,
        })
    }
    pub fn event(&self) -> Option<EventFacts<'_>> {
        let selected = self.selected.as_ref()?;
        let FormOwner::Event(name) = selected.owner else {
            return None;
        };
        let event = &selected.thing.events.as_ref()?[name];
        Some(EventFacts {
            name,
            metadata: (&event._metadata).into(),
        })
    }
    /// Typed schema at an Action/Event declaration. The role remains distinct:
    /// Action input/output and Event subscription/data/dataResponse/cancellation
    /// cannot be mistaken for a Property value or an initiating reply.
    pub fn affordance_schema(&self, owner: FormOwner<'_>, role: &str) -> Option<SchemaView<'_>> {
        if self.selected.as_ref()?.owner != owner {
            return None;
        }
        let suffix = match (owner, role) {
            (FormOwner::Action(_), "input" | "output") => role,
            (FormOwner::Event(_), "subscription" | "data" | "dataResponse" | "cancellation") => {
                role
            }
            _ => return None,
        };
        let scope = match owner {
            FormOwner::Action(name) => SemanticScope::Action(name),
            FormOwner::Event(name) => SemanticScope::Event(name),
            _ => return None,
        };
        self.scope(scope)?
            .children()
            .find(|n| {
                // Extension nodes precede the typed schema branches. An
                // extension with the same expanded name must not be returned
                // as a SchemaView without a typed DataSchema backing.
                n.nodes[n.index].schema.is_some()
                    && n.name().is(match suffix {
                        "input" => "https://www.w3.org/2019/wot/td#input",
                        "output" => "https://www.w3.org/2019/wot/td#output",
                        "subscription" => "https://www.w3.org/2019/wot/td#subscription",
                        "data" => "https://www.w3.org/2019/wot/td#data",
                        "dataResponse" => "https://www.w3.org/2019/wot/td#dataResponse",
                        _ => "https://www.w3.org/2019/wot/td#cancellation",
                    })
            })
            .map(SchemaView)
    }
    pub fn action_synchronous(&self, name: &str) -> Option<Option<bool>> {
        let selected = self.selected.as_ref()?;
        (selected.owner == FormOwner::Action(name))
            .then(|| selected.thing.actions.as_ref().unwrap()[name].synchronous)
    }
    pub fn schema_definition(&self, name: &str) -> Result<SchemaView<'_>, SemanticAccessError> {
        self.scope(SemanticScope::SchemaDefinition(name))
            .map(SchemaView)
            .ok_or(SemanticAccessError::OutsidePreparedScope)
    }
    /// Own-scope URI variables. No implicit merge of Thing and affordance maps.
    pub fn uri_variables(
        &self,
        scope: UriVariableScope,
    ) -> Option<impl Iterator<Item = (&str, SchemaView<'_>)>> {
        let selected = self.selected.as_ref().unwrap();
        let owner = match scope {
            UriVariableScope::Thing => FormOwner::Thing,
            UriVariableScope::Property => selected.owner,
            UriVariableScope::Action => selected.owner,
            UriVariableScope::Event => selected.owner,
        };
        let map = match (scope, owner) {
            (UriVariableScope::Thing, _) => selected.thing.uri_variables.as_ref(),
            (UriVariableScope::Property, FormOwner::Property(name)) => {
                selected.thing.properties.as_ref().unwrap()[name]
                    ._interaction
                    .uri_variables
                    .as_ref()
            }
            (UriVariableScope::Action, FormOwner::Action(name)) => {
                selected.thing.actions.as_ref().unwrap()[name]
                    ._interaction
                    .uri_variables
                    .as_ref()
            }
            (UriVariableScope::Event, FormOwner::Event(name)) => {
                selected.thing.events.as_ref().unwrap()[name]
                    ._interaction
                    .uri_variables
                    .as_ref()
            }
            _ => None,
        }?;
        Some(map.keys().map(move |name| {
            (
                name.as_str(),
                SchemaView(
                    self.scope(match owner {
                        FormOwner::Thing => SemanticScope::UriVariable {
                            property: None,
                            name,
                        },
                        FormOwner::Property(property) => SemanticScope::UriVariable {
                            property: Some(property),
                            name,
                        },
                        _ => SemanticScope::AffordanceUriVariable { owner, name },
                    })
                    .unwrap(),
                ),
            )
        }))
    }
    pub fn additional_response(
        &self,
        property: &str,
        form: u32,
        index: u32,
    ) -> Result<Option<ResponseFacts<'_>>, SemanticAccessError> {
        self.additional_form_response(FormOwner::Property(property), form, index)
    }

    pub fn primary_response(
        &self,
        owner: FormOwner<'_>,
        form: u32,
    ) -> Result<Option<(&str, SemanticNode<'_>)>, SemanticAccessError> {
        if self.selected.as_ref().map(|s| s.owner) != Some(owner) {
            return Err(SemanticAccessError::OutsidePreparedScope);
        }
        let parent = self
            .scopes
            .iter()
            .find(|s| s.key == form_scope(owner, form))
            .and_then(|s| s.form)
            .ok_or(SemanticAccessError::OutsidePreparedScope)?;
        Ok(parent.response.as_ref().map(|response| {
            (
                response.content_type.as_str(),
                self.scope(response_scope(owner, form)).unwrap(),
            )
        }))
    }

    pub fn additional_form_response(
        &self,
        owner: FormOwner<'_>,
        form: u32,
        index: u32,
    ) -> Result<Option<ResponseFacts<'_>>, SemanticAccessError> {
        if self.selected.as_ref().map(|s| s.owner) != Some(owner) {
            return Err(SemanticAccessError::OutsidePreparedScope);
        }
        let parent = self
            .scopes
            .iter()
            .find(|s| s.key == form_scope(owner, form))
            .and_then(|s| s.form)
            .ok_or(SemanticAccessError::OutsidePreparedScope)?;
        Ok(
            parent
                .additional_responses
                .as_ref()
                .and_then(|responses| responses.get(index as usize))
                .map(|response| ResponseFacts {
                    declared_content_type: response.content_type.as_deref(),
                    effective_content_type:
                        crate::td_defaults::effective_additional_response_content_type(
                            parent, response,
                        ),
                    success: response.success,
                    schema: response.schema.as_deref(),
                    semantics: self
                        .scope(additional_response_scope(owner, form, index))
                        .unwrap(),
                }),
        )
    }
    /// Security meaning is preserved independently of a particular Binding's
    /// support. A selected security provider, not TD, decides whether a scheme
    /// and its extension associations can be executed.
    pub fn security(&self, name: &str) -> Result<SecurityFacts<'_>, SemanticAccessError> {
        let semantics = self
            .scope(SemanticScope::Security(name))
            .ok_or(SemanticAccessError::OutsidePreparedScope)?;
        let definition = &self.selected.as_ref().unwrap().thing.security_definitions[name];
        let context = crate::validate::basic_typed::security_context(definition);
        Ok(SecurityFacts {
            scheme: definition.scheme(),
            proxy: context.proxy.as_ref().map(|uri| uri.as_str()),
            description: context.description.as_deref(),
            descriptions: context.descriptions.as_ref(),
            semantics,
            details: match definition {
                SecurityScheme::NoSec(_) => SecurityDetails::NoSec,
                SecurityScheme::Auto(_) => SecurityDetails::Auto,
                SecurityScheme::Combo(s) => SecurityDetails::Combo {
                    one_of: &s.one_of,
                    all_of: &s.all_of,
                },
                SecurityScheme::Basic(s) => SecurityDetails::Basic {
                    name: s.name.as_deref(),
                    location: &s.location,
                },
                SecurityScheme::Digest(s) => SecurityDetails::Digest {
                    name: s.name.as_deref(),
                    location: &s.location,
                    qop: s.qop,
                },
                SecurityScheme::APIKey(s) => SecurityDetails::ApiKey {
                    name: s.name.as_deref(),
                    location: &s.location,
                },
                SecurityScheme::Bearer(s) => SecurityDetails::Bearer {
                    name: s.name.as_deref(),
                    location: &s.location,
                    authorization: s.authorization.as_ref().map(|u| u.as_str()),
                    alg: &s.alg,
                    format: &s.format,
                },
                SecurityScheme::PSK(s) => SecurityDetails::Psk {
                    identity: s.identity.as_deref(),
                },
                SecurityScheme::OAuth2(s) => SecurityDetails::OAuth2 {
                    flow: &s.flow,
                    authorization: s.authorization.as_ref().map(|u| u.as_str()),
                    token: s.token.as_ref().map(|u| u.as_str()),
                    refresh: s.refresh.as_ref().map(|u| u.as_str()),
                    scopes: s.scopes.as_deref(),
                },
                SecurityScheme::Extension(_) => SecurityDetails::Extension,
            },
        })
    }
}

#[cfg(any(feature = "validated-thing", test))]
impl<'a> Resolver<'a> {
    pub(super) fn security_details_cost(
        &mut self,
        definition: &SecurityScheme,
    ) -> Result<(), SemanticError> {
        let mut pay_text = |value: &str| self.pay(value.len() as u64 + 1);
        match definition {
            SecurityScheme::Basic(s) => {
                if let Some(name) = &s.name {
                    pay_text(name)?;
                }
            }
            SecurityScheme::Digest(s) => {
                if let Some(name) = &s.name {
                    pay_text(name)?;
                }
            }
            SecurityScheme::APIKey(s) => {
                if let Some(name) = &s.name {
                    pay_text(name)?;
                }
            }
            SecurityScheme::Bearer(s) => {
                if let Some(name) = &s.name {
                    pay_text(name)?;
                }
                if let Some(uri) = &s.authorization {
                    pay_text(uri.as_str())?;
                }
                pay_text(&s.alg)?;
                pay_text(&s.format)?;
            }
            SecurityScheme::PSK(s) => {
                if let Some(identity) = &s.identity {
                    pay_text(identity)?;
                }
            }
            SecurityScheme::OAuth2(s) => {
                pay_text(&s.flow)?;
                for uri in [&s.authorization, &s.token, &s.refresh]
                    .into_iter()
                    .flatten()
                {
                    pay_text(uri.as_str())?;
                }
                if let Some(scopes) = &s.scopes {
                    for scope in scopes {
                        pay_text(scope)?;
                    }
                }
            }
            // Combo member names are paid as each reference is resolved.
            SecurityScheme::Combo(_)
            | SecurityScheme::NoSec(_)
            | SecurityScheme::Auto(_)
            | SecurityScheme::Extension(_) => {}
        }
        Ok(())
    }
    pub(super) fn security_facts_cost(
        &mut self,
        security: &crate::security_scheme::SecuritySchemeContext,
    ) -> Result<(), SemanticError> {
        for text in [
            Some(security.scheme.as_str()),
            security.proxy.as_ref().map(|uri| uri.as_str()),
            security.description.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            self.pay(text.len() as u64 + 1)?;
        }
        if let Some(map) = &security.descriptions {
            for (key, value) in map.as_map() {
                self.pay((key.len() + value.len() + 1) as u64)?;
            }
        }
        Ok(())
    }
    pub(super) fn metadata_cost(
        &mut self,
        metadata: &crate::data_type::Metadata,
    ) -> Result<(), SemanticError> {
        for text in [metadata.title.as_deref(), metadata.description.as_deref()]
            .into_iter()
            .flatten()
        {
            self.pay(text.len() as u64 + 1)?;
        }
        for map in [metadata.titles.as_ref(), metadata.descriptions.as_ref()]
            .into_iter()
            .flatten()
        {
            for (key, value) in map.as_map() {
                self.pay((key.len() + value.len() + 1) as u64)?;
            }
        }
        Ok(())
    }
    pub(super) fn form_facts_cost(
        &mut self,
        form: &crate::form::Form,
    ) -> Result<(), SemanticError> {
        self.pay((form.href.as_str().len() + form.content_type.len() + 1) as u64)?;
        for text in [
            form.content_coding.as_deref(),
            form.subprotocol.as_deref(),
            form.response.as_ref().map(|r| r.content_type.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            self.pay(text.len() as u64 + 1)?;
        }
        self.pay(form.op.as_ref().map_or(1, |ops| ops.len()) as u64)?;
        for list in [form.security.as_ref(), form.scopes.as_ref()]
            .into_iter()
            .flatten()
        {
            for name in list {
                self.pay(name.len() as u64 + 1)?;
            }
        }
        if let Some(responses) = &form.additional_responses {
            for response in responses {
                self.pay(1)?;
                if let Some(media) = &response.content_type {
                    self.pay(media.len() as u64)?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn schema_scope(
        &mut self,
        scope: SemanticScope<'a>,
        schema: &'a DataSchema,
        context: ActiveContext<'a>,
    ) -> Result<(), SemanticError> {
        let context = self.local(context, &schema.context()._extra_fields)?;
        self.scope(
            scope,
            &schema.context()._extra_fields,
            schema.context()._metadata.tags.as_deref(),
            context,
        )?;
        let root = self.document.scopes.last().unwrap().root;
        self.document.nodes[root].schema = Some(schema);
        self.schema_facts(schema, 0, context)?;
        self.schema_children(schema, context, 0)?;
        self.finish(root);
        Ok(())
    }
    pub(super) fn uri_variables(
        &mut self,
        map: Option<&'a BTreeMap<String, DataSchema>>,
        property: Option<&'a str>,
        mut context: ActiveContext<'a>,
    ) -> Result<(), SemanticError> {
        context.vocab = JS;
        if let Some(map) = map {
            for (name, schema) in map {
                self.pay(name.len() as u64 + 1)?;
                self.schema_scope(
                    SemanticScope::UriVariable { property, name },
                    schema,
                    context,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn affordance_uri_variables(
        &mut self,
        map: Option<&'a BTreeMap<String, DataSchema>>,
        owner: FormOwner<'a>,
        mut context: ActiveContext<'a>,
    ) -> Result<(), SemanticError> {
        context.vocab = JS;
        if let Some(map) = map {
            for (name, schema) in map {
                self.pay(name.len() as u64 + 1)?;
                self.schema_scope(
                    SemanticScope::AffordanceUriVariable { owner, name },
                    schema,
                    context,
                )?;
            }
        }
        Ok(())
    }
    fn literal_cost(&mut self, value: &Value, depth: usize) -> Result<(), SemanticError> {
        if depth == DEPTH {
            return Err(SemanticError::Limit);
        }
        self.pay(1)?;
        match value {
            Value::String(value) => self.pay(value.len() as u64)?,
            Value::Array(values) => {
                for value in values {
                    self.literal_cost(value, depth + 1)?;
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    self.pay(key.len() as u64)?;
                    self.literal_cost(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn schema_facts(
        &mut self,
        schema: &'a DataSchema,
        depth: usize,
        context: ActiveContext<'a>,
    ) -> Result<(), SemanticError> {
        let facts = schema.context();
        // Fixed getters do not copy strings, numbers or literal backing. Charge
        // the reachable borrowed content as well as constructing its handle.
        self.pay(32)?;
        self.metadata_cost(&facts._metadata)?;
        for text in [
            facts.data_type.as_deref(),
            facts.unit.as_deref(),
            facts.format.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            self.pay(text.len() as u64)?;
        }
        for value in [facts.constant.as_ref(), facts.default.as_ref()]
            .into_iter()
            .flatten()
        {
            self.literal_cost(value, depth)?;
        }
        if let Some(values) = &facts.enumerate {
            for value in values {
                self.literal_cost(value, depth)?;
            }
        }
        match schema {
            DataSchema::String(s) => {
                for value in [
                    s.pattern.as_deref(),
                    s.content_encoding.as_deref(),
                    s.content_media_type.as_deref(),
                ]
                .into_iter()
                .flatten()
                {
                    self.pay(value.len() as u64)?;
                }
            }
            DataSchema::Object(s) => {
                if let Some(required) = &s.required {
                    for name in required {
                        self.pay(name.len() as u64 + 1)?;
                    }
                }
            }
            _ => {}
        }
        // JSON Schema / Thing Model references are not in this TD schema
        // projection. Refuse an executable dependency rather than losing it.
        for key in facts._extra_fields.keys() {
            let name = self.name(key, context, 0)?;
            if key == "$ref" || name.is("https://www.w3.org/2019/wot/tm#ref") {
                return Err(SemanticError::UnsupportedDependency);
            }
            if facts.data_type.is_none()
                && !matches!(schema, DataSchema::Array(_))
                && name.is("https://www.w3.org/2019/wot/json-schema#items")
            {
                // The upstream generic representation did not validate these
                // children as DataSchemas. Do not report them as absent or
                // hand out a supposedly validated child handle.
                return Err(SemanticError::UnsupportedDependency);
            }
        }
        Ok(())
    }
}
