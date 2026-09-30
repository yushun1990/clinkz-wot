//! Borrowed typed facts, compiled against both real TD and the source candidate.
use super::{
    basic_kernel::{BasicAccess, Field, Owner, OwnerKind},
    schema_access::TypedAccess,
};
use crate::{
    affordance::{ActionAffordance, EventAffordance, InteractionAffordance, PropertyAffordance},
    data_schema::DataSchema,
    data_type::Operation,
    form::Form,
    security_scheme::{SecurityScheme, SecuritySchemeContext},
    thing::Thing,
};
use alloc::{collections::BTreeMap, string::String};
use serde_json::Value;

#[derive(Clone, Copy)]
pub enum Affordance<'a> {
    Property(&'a PropertyAffordance),
    Action(&'a ActionAffordance),
    Event(&'a EventAffordance),
}

impl<'a> Affordance<'a> {
    fn interaction(self) -> &'a InteractionAffordance {
        match self {
            Self::Property(value) => &value._interaction,
            Self::Action(value) => &value._interaction,
            Self::Event(value) => &value._interaction,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Names<'a> {
    Strings(&'a [String]),
    JsonArray(&'a [Value]),
    JsonString(&'a str),
    Empty,
}

impl<'a> Names<'a> {
    pub fn count(self) -> usize {
        match self {
            Self::Strings(values) => values.len(),
            Self::JsonArray(values) => values.iter().filter(|value| value.is_string()).count(),
            Self::JsonString(_) => 1,
            Self::Empty => 0,
        }
    }
    pub fn at(self, index: usize) -> &'a str {
        match self {
            Self::Strings(values) => &values[index],
            Self::JsonArray(values) => values.iter().filter_map(Value::as_str).nth(index).unwrap(),
            Self::JsonString(value) => {
                assert_eq!(index, 0);
                value
            }
            Self::Empty => unreachable!(),
        }
    }
}

pub fn security_context(value: &SecurityScheme) -> &SecuritySchemeContext {
    match value {
        SecurityScheme::NoSec(v) => &v._context,
        SecurityScheme::Auto(v) => &v._context,
        SecurityScheme::Combo(v) => &v._context,
        SecurityScheme::Basic(v) => &v._context,
        SecurityScheme::Digest(v) => &v._context,
        SecurityScheme::APIKey(v) => &v._context,
        SecurityScheme::Bearer(v) => &v._context,
        SecurityScheme::PSK(v) => &v._context,
        SecurityScheme::OAuth2(v) => &v._context,
    }
}

/// Whole-document methods require a root; component methods borrow only the
/// passed component. No dummy Thing is constructed for standalone validation.
pub struct TypedBasicAccess<'a>(pub Option<&'a Thing>);
static SCHEMAS: TypedAccess = TypedAccess;

impl<'a> BasicAccess<'a> for TypedBasicAccess<'a> {
    type Schemas = TypedAccess;
    type SchemaMap = &'a BTreeMap<String, DataSchema>;
    type Affordance = Affordance<'a>;
    type Forms = &'a [Form];
    type Form = &'a Form;
    type Operations = &'a [Operation];
    type Names = Names<'a>;
    type Definition = &'a SecurityScheme;
    fn schemas(&self) -> &TypedAccess {
        &SCHEMAS
    }
    fn title(&self) -> Option<&'a str> {
        self.0.unwrap()._metadata.title.as_deref()
    }
    fn root_security(&self) -> Names<'a> {
        Names::Strings(&self.0.unwrap().security)
    }
    fn names_count(&self, names: Names<'a>) -> usize {
        names.count()
    }
    fn name_at(&self, names: Names<'a>, index: usize) -> &'a str {
        names.at(index)
    }
    fn definition_count(&self) -> usize {
        self.0.unwrap().security_definitions.len()
    }
    fn definition_at(&self, index: usize) -> Self::Definition {
        self.0
            .unwrap()
            .security_definitions
            .values()
            .nth(index)
            .unwrap()
    }
    fn definition_name(&self, index: usize) -> &'a str {
        self.0
            .unwrap()
            .security_definitions
            .keys()
            .nth(index)
            .unwrap()
    }
    fn definition_exists(&self, name: &str) -> bool {
        self.0.unwrap().security_definitions.contains_key(name)
    }
    fn scheme(&self, definition: Self::Definition) -> &'a str {
        definition.scheme()
    }
    fn combo_names(&self, definition: Self::Definition, field: Field) -> Names<'a> {
        if let SecurityScheme::Combo(value) = definition {
            return Names::Strings(match field {
                Field::OneOf => &value.one_of,
                Field::AllOf => &value.all_of,
                _ => unreachable!(),
            });
        }
        match security_context(definition)._extra_fields.get(field.name()) {
            Some(Value::Array(values)) => Names::JsonArray(values),
            Some(Value::String(value)) => Names::JsonString(value),
            _ => Names::Empty,
        }
    }
    fn security_string(&self, definition: Self::Definition, field: Field) -> Option<&'a str> {
        match (definition, field) {
            (SecurityScheme::APIKey(value), Field::Name) => value.name.as_deref(),
            (SecurityScheme::OAuth2(value), Field::Flow) => Some(&value.flow),
            _ => security_context(definition)
                ._extra_fields
                .get(field.name())
                .and_then(Value::as_str),
        }
    }
    fn endpoint_present(&self, definition: Self::Definition, field: Field) -> bool {
        match (definition, field) {
            (SecurityScheme::OAuth2(value), Field::Authorization) => value.authorization.is_some(),
            (SecurityScheme::OAuth2(value), Field::Token) => value.token.is_some(),
            _ => self
                .security_string(definition, field)
                .is_some_and(|value| !value.is_empty()),
        }
    }
    fn root_schema_map(&self, field: Field) -> Option<Self::SchemaMap> {
        let thing = self.0.unwrap();
        match field {
            Field::SchemaDefinitions => thing.schema_definitions.as_ref(),
            Field::UriVariables => thing.uri_variables.as_ref(),
            _ => unreachable!(),
        }
    }
    fn schema_count(&self, map: Self::SchemaMap) -> usize {
        map.len()
    }
    fn schema_at(&self, map: Self::SchemaMap, index: usize) -> &'a DataSchema {
        map.values().nth(index).unwrap()
    }
    fn schema_name(&self, map: Self::SchemaMap, index: usize) -> &'a str {
        map.keys().nth(index).unwrap()
    }
    fn affordance_count(&self, kind: OwnerKind) -> usize {
        let thing = self.0.unwrap();
        match kind {
            OwnerKind::Property => thing.properties.as_ref().map_or(0, |map| map.len()),
            OwnerKind::Action => thing.actions.as_ref().map_or(0, |map| map.len()),
            OwnerKind::Event => thing.events.as_ref().map_or(0, |map| map.len()),
            _ => unreachable!(),
        }
    }
    fn affordance_at(&self, kind: OwnerKind, index: usize) -> Affordance<'a> {
        let thing = self.0.unwrap();
        match kind {
            OwnerKind::Property => Affordance::Property(
                thing
                    .properties
                    .as_ref()
                    .unwrap()
                    .values()
                    .nth(index)
                    .unwrap(),
            ),
            OwnerKind::Action => {
                Affordance::Action(thing.actions.as_ref().unwrap().values().nth(index).unwrap())
            }
            OwnerKind::Event => {
                Affordance::Event(thing.events.as_ref().unwrap().values().nth(index).unwrap())
            }
            _ => unreachable!(),
        }
    }
    fn affordance_name(&self, owner: Owner) -> &'a str {
        let thing = self.0.unwrap();
        let index = owner.ordinal;
        match owner.kind {
            OwnerKind::Property => thing
                .properties
                .as_ref()
                .unwrap()
                .keys()
                .nth(index)
                .unwrap(),
            OwnerKind::Action => thing.actions.as_ref().unwrap().keys().nth(index).unwrap(),
            OwnerKind::Event => thing.events.as_ref().unwrap().keys().nth(index).unwrap(),
            _ => unreachable!(),
        }
    }
    fn uri_variables(&self, affordance: Affordance<'a>) -> Option<Self::SchemaMap> {
        affordance.interaction().uri_variables.as_ref()
    }
    fn affordance_schema(
        &self,
        affordance: Affordance<'a>,
        field: Field,
    ) -> Option<&'a DataSchema> {
        match (affordance, field) {
            (Affordance::Property(value), Field::PropertySchema) => Some(&value._schema),
            (Affordance::Action(value), Field::Input) => value.input.as_ref(),
            (Affordance::Action(value), Field::Output) => value.output.as_ref(),
            (Affordance::Event(value), Field::Subscription) => value.subscription.as_ref(),
            (Affordance::Event(value), Field::Data) => value.data.as_ref(),
            (Affordance::Event(value), Field::DataResponse) => value.data_response.as_ref(),
            (Affordance::Event(value), Field::Cancellation) => value.cancellation.as_ref(),
            _ => unreachable!(),
        }
    }
    fn forms(&self, affordance: Option<Affordance<'a>>) -> Option<Self::Forms> {
        match affordance {
            Some(value) => Some(&value.interaction().forms),
            None => self.0.unwrap().forms.as_deref(),
        }
    }
    fn form_count(&self, forms: Self::Forms) -> usize {
        forms.len()
    }
    fn form_at(&self, forms: Self::Forms, index: usize) -> Self::Form {
        &forms[index]
    }
    fn operations(&self, form: Self::Form) -> Option<Self::Operations> {
        form.op.as_deref()
    }
    fn operation_count(&self, ops: Self::Operations) -> usize {
        ops.len()
    }
    fn operation_at(&self, ops: Self::Operations, index: usize) -> Operation {
        ops[index]
    }
    fn form_security(&self, form: Self::Form) -> Option<Names<'a>> {
        form.security.as_deref().map(Names::Strings)
    }
}
