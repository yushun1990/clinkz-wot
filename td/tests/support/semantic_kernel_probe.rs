//! Non-production storage-neutral semantic-access prototype for the existing
//! typed `Thing` and the three-arena [`Snapshot`].
//!
//! The adapters expose representation facts only. Property operation defaults,
//! effective Thing/Form security, and security-name reference checks live once
//! in this module and are exercised against both storage forms. The production
//! `td_defaults` helpers and current Basic validator remain unchanged and serve
//! as the typed-Thing oracle.

extern crate std;

use super::{
    ACTION_FORMS, EVENT_FORMS, FORM_CONTENT_CODING, FORM_CONTENT_TYPE, FORM_OPERATIONS,
    FORM_SCOPES, FORM_SECURITY, FORM_SUBPROTOCOL, Kind, PROPERTY_FORMS, PROPERTY_SCHEMA,
    ROOT_ACTIONS, ROOT_EVENTS, ROOT_FORMS, ROOT_ID, ROOT_PROPERTIES, ROOT_SECURITY,
    ROOT_SECURITY_DEFINITIONS, SECURITY_CONTEXT_EXTENSIONS, SECURITY_CONTEXT_SCHEME,
    SECURITY_SCHEME_CONTEXT, SECURITY_SCHEME_VARIANT, Snapshot,
};
use crate::{
    affordance::PropertyAffordance,
    data_schema::{DataSchema, DataSchemaContext},
    data_type::Operation,
    form::Form,
    security_scheme::SecurityScheme,
    td_defaults::{self, FormContext},
    thing::Thing,
    validate::{Validate, ValidateError, ValidationLevel},
};
use alloc::{string::String, vec, vec::Vec};
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::Cell,
};
use std::{alloc::System, thread_local};

const NO_OPERATIONS: &[Operation] = &[];
const PROPERTY_READ_WRITE_OPERATIONS: &[Operation] =
    &[Operation::ReadProperty, Operation::WriteProperty];
const PROPERTY_READ_OPERATIONS: &[Operation] = &[Operation::ReadProperty];
const PROPERTY_WRITE_OPERATIONS: &[Operation] = &[Operation::WriteProperty];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FormOwner {
    Property(u32),
    Action(u32),
    Event(u32),
    Thing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FormSite {
    owner: FormOwner,
    original_index: u32,
}

#[derive(Clone, Copy)]
struct LocatedForm<F> {
    form: F,
    site: FormSite,
}

/// Storage facts required by this semantic slice. No method decides defaults,
/// inheritance, reference validity, or Planning eligibility.
trait SemanticAccess<'a> {
    type Property: Copy;
    type Form: Copy;
    type Operations: Copy;
    type SecurityNames: Copy;
    type SecurityDefinition: Copy;

    fn property_count(&'a self) -> usize;
    fn property_at(&'a self, index: usize) -> Option<Self::Property>;
    fn property_name(&'a self, property: Self::Property) -> &'a str;
    fn property_flags(&'a self, property: Self::Property) -> (bool, bool);
    fn property_form_count(&'a self, property: Self::Property) -> usize;
    fn property_form_at(&'a self, property: Self::Property, index: usize) -> Option<Self::Form>;

    fn explicit_operations(&'a self, form: Self::Form) -> Option<Self::Operations>;
    fn operation_count(&'a self, operations: Self::Operations) -> usize;
    fn operation_at(&'a self, operations: Self::Operations, index: usize) -> Option<Operation>;

    fn thing_security(&'a self) -> Self::SecurityNames;
    fn explicit_form_security(&'a self, form: Self::Form) -> Option<Self::SecurityNames>;
    fn security_name_count(&'a self, names: Self::SecurityNames) -> usize;
    fn security_name_at(&'a self, names: Self::SecurityNames, index: usize) -> Option<&'a str>;

    fn security_definition_count(&'a self) -> usize;
    fn security_definition_at(&'a self, index: usize) -> Option<Self::SecurityDefinition>;
    fn security_definition_by_name(&'a self, name: &str) -> Option<Self::SecurityDefinition>;
    fn security_definition_name(&'a self, definition: Self::SecurityDefinition) -> &'a str;
    fn security_definition_scheme(&'a self, definition: Self::SecurityDefinition) -> &'a str;
    fn security_definition_one_of(
        &'a self,
        definition: Self::SecurityDefinition,
    ) -> Option<Self::SecurityNames>;
    fn security_definition_all_of(
        &'a self,
        definition: Self::SecurityDefinition,
    ) -> Option<Self::SecurityNames>;

    /// Forms are visited in current Basic order: Property, Action, Event, then
    /// Thing-level forms. The site is fixed inline and owns no diagnostic text.
    fn all_form_count(&'a self) -> usize;
    fn all_form_at(&'a self, index: usize) -> Option<LocatedForm<Self::Form>>;
}

#[derive(Clone, Copy)]
struct ThingProperty<'a> {
    name: &'a str,
    property: &'a PropertyAffordance,
}

#[derive(Clone, Copy)]
struct ThingSecurityDefinition<'a> {
    name: &'a str,
    definition: &'a SecurityScheme,
}

#[derive(Clone, Copy)]
enum ThingSecurityNames<'a> {
    Strings(&'a [String]),
    JsonArray(&'a [serde_json::Value]),
    JsonString(&'a str),
    Empty,
}

fn thing_security_context(
    definition: &SecurityScheme,
) -> &crate::security_scheme::SecuritySchemeContext {
    match definition {
        SecurityScheme::NoSec(value) => &value._context,
        SecurityScheme::Auto(value) => &value._context,
        SecurityScheme::Combo(value) => &value._context,
        SecurityScheme::Basic(value) => &value._context,
        SecurityScheme::Digest(value) => &value._context,
        SecurityScheme::APIKey(value) => &value._context,
        SecurityScheme::Bearer(value) => &value._context,
        SecurityScheme::PSK(value) => &value._context,
        SecurityScheme::OAuth2(value) => &value._context,
    }
}

fn thing_extension_security_names<'a>(
    definition: &'a SecurityScheme,
    field: &str,
) -> ThingSecurityNames<'a> {
    match thing_security_context(definition)._extra_fields.get(field) {
        Some(serde_json::Value::Array(values)) => ThingSecurityNames::JsonArray(values),
        Some(serde_json::Value::String(value)) => ThingSecurityNames::JsonString(value),
        _ => ThingSecurityNames::Empty,
    }
}

impl<'a> SemanticAccess<'a> for Thing {
    type Property = ThingProperty<'a>;
    type Form = &'a Form;
    type Operations = &'a [Operation];
    type SecurityNames = ThingSecurityNames<'a>;
    type SecurityDefinition = ThingSecurityDefinition<'a>;

    fn property_count(&'a self) -> usize {
        self.properties.as_ref().map_or(0, |values| values.len())
    }

    fn property_at(&'a self, index: usize) -> Option<Self::Property> {
        self.properties
            .as_ref()?
            .iter()
            .nth(index)
            .map(|(name, property)| ThingProperty { name, property })
    }

    fn property_name(&'a self, property: Self::Property) -> &'a str {
        property.name
    }

    fn property_flags(&'a self, property: Self::Property) -> (bool, bool) {
        let context = property.property._schema.context();
        (context.read_only, context.write_only)
    }

    fn property_form_count(&'a self, property: Self::Property) -> usize {
        property.property._interaction.forms.len()
    }

    fn property_form_at(&'a self, property: Self::Property, index: usize) -> Option<Self::Form> {
        property.property._interaction.forms.get(index)
    }

    fn explicit_operations(&'a self, form: Self::Form) -> Option<Self::Operations> {
        form.op.as_deref()
    }

    fn operation_count(&'a self, operations: Self::Operations) -> usize {
        operations.len()
    }

    fn operation_at(&'a self, operations: Self::Operations, index: usize) -> Option<Operation> {
        operations.get(index).copied()
    }

    fn thing_security(&'a self) -> Self::SecurityNames {
        ThingSecurityNames::Strings(&self.security)
    }

    fn explicit_form_security(&'a self, form: Self::Form) -> Option<Self::SecurityNames> {
        form.security.as_deref().map(ThingSecurityNames::Strings)
    }

    fn security_name_count(&'a self, names: Self::SecurityNames) -> usize {
        match names {
            ThingSecurityNames::Strings(values) => values.len(),
            ThingSecurityNames::JsonArray(values) => {
                values.iter().filter(|value| value.is_string()).count()
            }
            ThingSecurityNames::JsonString(_) => 1,
            ThingSecurityNames::Empty => 0,
        }
    }

    fn security_name_at(&'a self, names: Self::SecurityNames, index: usize) -> Option<&'a str> {
        match names {
            ThingSecurityNames::Strings(values) => values.get(index).map(String::as_str),
            ThingSecurityNames::JsonArray(values) => values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .nth(index),
            ThingSecurityNames::JsonString(value) => (index == 0).then_some(value),
            ThingSecurityNames::Empty => None,
        }
    }

    fn security_definition_count(&'a self) -> usize {
        self.security_definitions.len()
    }

    fn security_definition_at(&'a self, index: usize) -> Option<Self::SecurityDefinition> {
        self.security_definitions
            .iter()
            .nth(index)
            .map(|(name, definition)| ThingSecurityDefinition { name, definition })
    }

    fn security_definition_by_name(&'a self, name: &str) -> Option<Self::SecurityDefinition> {
        self.security_definitions
            .get_key_value(name)
            .map(|(name, definition)| ThingSecurityDefinition { name, definition })
    }

    fn security_definition_name(&'a self, definition: Self::SecurityDefinition) -> &'a str {
        definition.name
    }

    fn security_definition_scheme(&'a self, definition: Self::SecurityDefinition) -> &'a str {
        definition.definition.scheme()
    }

    fn security_definition_one_of(
        &'a self,
        definition: Self::SecurityDefinition,
    ) -> Option<Self::SecurityNames> {
        Some(match definition.definition {
            SecurityScheme::Combo(combo) => ThingSecurityNames::Strings(&combo.one_of),
            definition => thing_extension_security_names(definition, "oneOf"),
        })
    }

    fn security_definition_all_of(
        &'a self,
        definition: Self::SecurityDefinition,
    ) -> Option<Self::SecurityNames> {
        Some(match definition.definition {
            SecurityScheme::Combo(combo) => ThingSecurityNames::Strings(&combo.all_of),
            definition => thing_extension_security_names(definition, "allOf"),
        })
    }

    fn all_form_count(&'a self) -> usize {
        let properties = self.properties.as_ref().map_or(0, |values| {
            values
                .values()
                .map(|property| property._interaction.forms.len())
                .sum()
        });
        let actions = self.actions.as_ref().map_or(0, |values| {
            values
                .values()
                .map(|action| action._interaction.forms.len())
                .sum()
        });
        let events = self.events.as_ref().map_or(0, |values| {
            values
                .values()
                .map(|event| event._interaction.forms.len())
                .sum()
        });
        let thing = self.forms.as_deref().map_or(0, <[Form]>::len);
        properties + actions + events + thing
    }

    fn all_form_at(&'a self, mut index: usize) -> Option<LocatedForm<Self::Form>> {
        if let Some(properties) = &self.properties {
            for (ordinal, property) in properties.values().enumerate() {
                let forms = &property._interaction.forms;
                if index < forms.len() {
                    return Some(LocatedForm {
                        form: &forms[index],
                        site: FormSite {
                            owner: FormOwner::Property(ordinal.try_into().ok()?),
                            original_index: index.try_into().ok()?,
                        },
                    });
                }
                index -= forms.len();
            }
        }
        if let Some(actions) = &self.actions {
            for (ordinal, action) in actions.values().enumerate() {
                let forms = &action._interaction.forms;
                if index < forms.len() {
                    return Some(LocatedForm {
                        form: &forms[index],
                        site: FormSite {
                            owner: FormOwner::Action(ordinal.try_into().ok()?),
                            original_index: index.try_into().ok()?,
                        },
                    });
                }
                index -= forms.len();
            }
        }
        if let Some(events) = &self.events {
            for (ordinal, event) in events.values().enumerate() {
                let forms = &event._interaction.forms;
                if index < forms.len() {
                    return Some(LocatedForm {
                        form: &forms[index],
                        site: FormSite {
                            owner: FormOwner::Event(ordinal.try_into().ok()?),
                            original_index: index.try_into().ok()?,
                        },
                    });
                }
                index -= forms.len();
            }
        }
        let forms = self.forms.as_deref().unwrap_or_default();
        forms.get(index).map(|form| LocatedForm {
            form,
            site: FormSite {
                owner: FormOwner::Thing,
                original_index: index.try_into().unwrap(),
            },
        })
    }
}

#[derive(Clone, Copy)]
struct SnapshotProperty {
    name: u32,
    property: u32,
}

#[derive(Clone, Copy)]
struct SnapshotSecurityDefinition {
    name: u32,
    definition: u32,
}

#[derive(Clone, Copy)]
enum SnapshotSecurityNames {
    Array(u32),
    String(u32),
    Empty,
}

impl Snapshot {
    fn optional_sequence(&self, node: u32) -> Option<u32> {
        match self.kind(node) {
            Kind::Absent => None,
            Kind::Array => Some(node),
            kind => panic!("expected optional sequence, found {kind:?}"),
        }
    }

    fn sequence_len(&self, node: u32) -> usize {
        assert_eq!(self.kind(node), Kind::Array);
        self.node(node).edge_count as usize
    }

    fn optional_map_len(&self, node: u32) -> usize {
        match self.kind(node) {
            Kind::Absent => 0,
            Kind::Map => self.node(node).edge_count as usize,
            kind => panic!("expected optional map, found {kind:?}"),
        }
    }

    fn map_pair(&self, map: u32, index: usize) -> Option<(u32, u32)> {
        if self.kind(map) != Kind::Map || index >= self.node(map).edge_count as usize {
            return None;
        }
        let entry = self.child(map, index);
        Some((self.child(entry, 0), self.child(entry, 1)))
    }

    fn definition_handle(&self, index: usize) -> Option<SnapshotSecurityDefinition> {
        let definitions = self.child(self.root, ROOT_SECURITY_DEFINITIONS);
        let (name, definition) = self.map_pair(definitions, index)?;
        Some(SnapshotSecurityDefinition { name, definition })
    }

    fn extension_security_names(
        &self,
        definition: SnapshotSecurityDefinition,
        field: &str,
    ) -> SnapshotSecurityNames {
        let context = self.child(definition.definition, SECURITY_SCHEME_CONTEXT);
        let extensions = self.child(context, SECURITY_CONTEXT_EXTENSIONS);
        match self.map_get(extensions, field) {
            Some(node) if self.kind(node) == Kind::Array => SnapshotSecurityNames::Array(node),
            Some(node) if self.kind(node) == Kind::Str => SnapshotSecurityNames::String(node),
            _ => SnapshotSecurityNames::Empty,
        }
    }

    fn forms_in_map_count(&self, map: u32, forms_field: usize) -> usize {
        (0..self.optional_map_len(map))
            .map(|index| {
                let (_, value) = self.map_pair(map, index).unwrap();
                self.sequence_len(self.child(value, forms_field))
            })
            .sum()
    }

    fn form_in_map(
        &self,
        map: u32,
        forms_field: usize,
        owner: fn(u32) -> FormOwner,
        mut index: usize,
    ) -> Result<LocatedForm<u32>, usize> {
        for ordinal in 0..self.optional_map_len(map) {
            let (_, value) = self.map_pair(map, ordinal).unwrap();
            let forms = self.child(value, forms_field);
            let count = self.sequence_len(forms);
            if index < count {
                return Ok(LocatedForm {
                    form: self.child(forms, index),
                    site: FormSite {
                        owner: owner(ordinal.try_into().unwrap()),
                        original_index: index.try_into().unwrap(),
                    },
                });
            }
            index -= count;
        }
        Err(index)
    }
}

impl<'a> SemanticAccess<'a> for Snapshot {
    type Property = SnapshotProperty;
    type Form = u32;
    type Operations = u32;
    type SecurityNames = SnapshotSecurityNames;
    type SecurityDefinition = SnapshotSecurityDefinition;

    fn property_count(&'a self) -> usize {
        self.optional_map_len(self.child(self.root, ROOT_PROPERTIES))
    }

    fn property_at(&'a self, index: usize) -> Option<Self::Property> {
        let (name, property) = self.map_pair(self.child(self.root, ROOT_PROPERTIES), index)?;
        Some(SnapshotProperty { name, property })
    }

    fn property_name(&'a self, property: Self::Property) -> &'a str {
        self.text(property.name)
    }

    fn property_flags(&'a self, property: Self::Property) -> (bool, bool) {
        let schema = self.child(property.property, PROPERTY_SCHEMA);
        let context = self.child(schema, 1);
        let read_only = self.map_get(context, "readOnly").unwrap();
        let write_only = self.map_get(context, "writeOnly").unwrap();
        (
            self.kind(read_only) == Kind::True,
            self.kind(write_only) == Kind::True,
        )
    }

    fn property_form_count(&'a self, property: Self::Property) -> usize {
        self.sequence_len(self.child(property.property, PROPERTY_FORMS))
    }

    fn property_form_at(&'a self, property: Self::Property, index: usize) -> Option<Self::Form> {
        let forms = self.child(property.property, PROPERTY_FORMS);
        (index < self.sequence_len(forms)).then(|| self.child(forms, index))
    }

    fn explicit_operations(&'a self, form: Self::Form) -> Option<Self::Operations> {
        self.optional_sequence(self.child(form, FORM_OPERATIONS))
    }

    fn operation_count(&'a self, operations: Self::Operations) -> usize {
        self.sequence_len(operations)
    }

    fn operation_at(&'a self, operations: Self::Operations, index: usize) -> Option<Operation> {
        (index < self.sequence_len(operations))
            .then(|| self.text(self.child(operations, index)))
            .and_then(operation_from_text)
    }

    fn thing_security(&'a self) -> Self::SecurityNames {
        SnapshotSecurityNames::Array(self.child(self.root, ROOT_SECURITY))
    }

    fn explicit_form_security(&'a self, form: Self::Form) -> Option<Self::SecurityNames> {
        self.optional_sequence(self.child(form, FORM_SECURITY))
            .map(SnapshotSecurityNames::Array)
    }

    fn security_name_count(&'a self, names: Self::SecurityNames) -> usize {
        match names {
            SnapshotSecurityNames::Array(node) => (0..self.sequence_len(node))
                .filter(|&index| self.kind(self.child(node, index)) == Kind::Str)
                .count(),
            SnapshotSecurityNames::String(_) => 1,
            SnapshotSecurityNames::Empty => 0,
        }
    }

    fn security_name_at(&'a self, names: Self::SecurityNames, index: usize) -> Option<&'a str> {
        match names {
            SnapshotSecurityNames::Array(node) => (0..self.sequence_len(node))
                .map(|item| self.child(node, item))
                .filter(|&item| self.kind(item) == Kind::Str)
                .nth(index)
                .map(|item| self.text(item)),
            SnapshotSecurityNames::String(node) => (index == 0).then(|| self.text(node)),
            SnapshotSecurityNames::Empty => None,
        }
    }

    fn security_definition_count(&'a self) -> usize {
        self.node(self.child(self.root, ROOT_SECURITY_DEFINITIONS))
            .edge_count as usize
    }

    fn security_definition_at(&'a self, index: usize) -> Option<Self::SecurityDefinition> {
        self.definition_handle(index)
    }

    fn security_definition_by_name(&'a self, name: &str) -> Option<Self::SecurityDefinition> {
        (0..self.security_definition_count()).find_map(|index| {
            let definition = self.definition_handle(index).unwrap();
            (self.text(definition.name) == name).then_some(definition)
        })
    }

    fn security_definition_name(&'a self, definition: Self::SecurityDefinition) -> &'a str {
        self.text(definition.name)
    }

    fn security_definition_scheme(&'a self, definition: Self::SecurityDefinition) -> &'a str {
        let context = self.child(definition.definition, SECURITY_SCHEME_CONTEXT);
        self.text(self.child(context, SECURITY_CONTEXT_SCHEME))
    }

    fn security_definition_one_of(
        &'a self,
        definition: Self::SecurityDefinition,
    ) -> Option<Self::SecurityNames> {
        let variant = self.child(definition.definition, SECURITY_SCHEME_VARIANT);
        Some(if self.kind(variant) == Kind::SecurityCombo {
            SnapshotSecurityNames::Array(self.child(variant, 0))
        } else {
            self.extension_security_names(definition, "oneOf")
        })
    }

    fn security_definition_all_of(
        &'a self,
        definition: Self::SecurityDefinition,
    ) -> Option<Self::SecurityNames> {
        let variant = self.child(definition.definition, SECURITY_SCHEME_VARIANT);
        Some(if self.kind(variant) == Kind::SecurityCombo {
            SnapshotSecurityNames::Array(self.child(variant, 1))
        } else {
            self.extension_security_names(definition, "allOf")
        })
    }

    fn all_form_count(&'a self) -> usize {
        let properties =
            self.forms_in_map_count(self.child(self.root, ROOT_PROPERTIES), PROPERTY_FORMS);
        let actions = self.forms_in_map_count(self.child(self.root, ROOT_ACTIONS), ACTION_FORMS);
        let events = self.forms_in_map_count(self.child(self.root, ROOT_EVENTS), EVENT_FORMS);
        let root_forms = self.child(self.root, ROOT_FORMS);
        properties
            + actions
            + events
            + self
                .optional_sequence(root_forms)
                .map_or(0, |forms| self.sequence_len(forms))
    }

    fn all_form_at(&'a self, index: usize) -> Option<LocatedForm<Self::Form>> {
        let index = match self.form_in_map(
            self.child(self.root, ROOT_PROPERTIES),
            PROPERTY_FORMS,
            FormOwner::Property,
            index,
        ) {
            Ok(form) => return Some(form),
            Err(index) => index,
        };
        let index = match self.form_in_map(
            self.child(self.root, ROOT_ACTIONS),
            ACTION_FORMS,
            FormOwner::Action,
            index,
        ) {
            Ok(form) => return Some(form),
            Err(index) => index,
        };
        let index = match self.form_in_map(
            self.child(self.root, ROOT_EVENTS),
            EVENT_FORMS,
            FormOwner::Event,
            index,
        ) {
            Ok(form) => return Some(form),
            Err(index) => index,
        };
        let forms = self.optional_sequence(self.child(self.root, ROOT_FORMS))?;
        (index < self.sequence_len(forms)).then(|| LocatedForm {
            form: self.child(forms, index),
            site: FormSite {
                owner: FormOwner::Thing,
                original_index: index.try_into().unwrap(),
            },
        })
    }
}

fn operation_from_text(value: &str) -> Option<Operation> {
    Some(match value {
        "readproperty" => Operation::ReadProperty,
        "writeproperty" => Operation::WriteProperty,
        "observeproperty" => Operation::ObserveProperty,
        "unobserveproperty" => Operation::UnobserveProperty,
        "invokeaction" => Operation::InvokeAction,
        "queryaction" => Operation::QueryAction,
        "cancelaction" => Operation::CancelAction,
        "subscribeevent" => Operation::SubscribeEvent,
        "unsubscribeevent" => Operation::UnsubscribeEvent,
        "readallproperties" => Operation::ReadAllProperties,
        "writeallproperties" => Operation::WriteAllProperties,
        "readmultipleproperties" => Operation::ReadMultipleProperties,
        "writemultipleproperties" => Operation::WriteMultipleProperties,
        "observeallproperties" => Operation::ObserveAllProperties,
        "unobserveallproperties" => Operation::UnobserveAllProperties,
        "queryallactions" => Operation::QueryAllActions,
        "subscribeallevents" => Operation::SubscribeAllEvents,
        "unsubscribeallevents" => Operation::UnsubscribeAllEvents,
        _ => return None,
    })
}

#[derive(Clone, Copy)]
enum OperationSource<O> {
    Explicit(O),
    Default(&'static [Operation]),
}

struct EffectiveOperations<'a, A: SemanticAccess<'a> + ?Sized> {
    access: &'a A,
    source: OperationSource<A::Operations>,
}

impl<'a, A: SemanticAccess<'a> + ?Sized> EffectiveOperations<'a, A> {
    fn len(&self) -> usize {
        match self.source {
            OperationSource::Explicit(operations) => self.access.operation_count(operations),
            OperationSource::Default(operations) => operations.len(),
        }
    }

    fn get(&self, index: usize) -> Option<Operation> {
        match self.source {
            OperationSource::Explicit(operations) => self.access.operation_at(operations, index),
            OperationSource::Default(operations) => operations.get(index).copied(),
        }
    }

    fn is_explicit(&self) -> bool {
        matches!(self.source, OperationSource::Explicit(_))
    }

    fn iter(&self) -> OperationIter<'_, 'a, A> {
        OperationIter {
            operations: self,
            next: 0,
        }
    }
}

struct OperationIter<'list, 'a, A: SemanticAccess<'a> + ?Sized> {
    operations: &'list EffectiveOperations<'a, A>,
    next: usize,
}

impl<'a, A: SemanticAccess<'a> + ?Sized> Clone for OperationIter<'_, 'a, A> {
    fn clone(&self) -> Self {
        Self {
            operations: self.operations,
            next: self.next,
        }
    }
}

impl<'a, A: SemanticAccess<'a> + ?Sized> Iterator for OperationIter<'_, 'a, A> {
    type Item = Operation;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.operations.get(self.next)?;
        self.next += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.operations.len() - self.next;
        (remaining, Some(remaining))
    }
}

impl<'a, A: SemanticAccess<'a> + ?Sized> ExactSizeIterator for OperationIter<'_, 'a, A> {}

fn effective_property_operations<'a, A: SemanticAccess<'a> + ?Sized>(
    access: &'a A,
    property: A::Property,
    form: A::Form,
) -> EffectiveOperations<'a, A> {
    if let Some(operations) = access.explicit_operations(form) {
        return EffectiveOperations {
            access,
            source: OperationSource::Explicit(operations),
        };
    }

    let (read_only, write_only) = access.property_flags(property);
    let defaults = match (read_only, write_only) {
        (true, false) => PROPERTY_READ_OPERATIONS,
        (false, true) => PROPERTY_WRITE_OPERATIONS,
        (true, true) | (false, false) => PROPERTY_READ_WRITE_OPERATIONS,
    };
    EffectiveOperations {
        access,
        source: OperationSource::Default(defaults),
    }
}

struct EffectiveSecurity<'a, A: SemanticAccess<'a> + ?Sized> {
    access: &'a A,
    names: A::SecurityNames,
    inherited: bool,
}

impl<'a, A: SemanticAccess<'a> + ?Sized> EffectiveSecurity<'a, A> {
    fn len(&self) -> usize {
        self.access.security_name_count(self.names)
    }

    fn get(&self, index: usize) -> Option<&'a str> {
        self.access.security_name_at(self.names, index)
    }

    fn is_inherited(&self) -> bool {
        self.inherited
    }

    fn iter(&self) -> SecurityIter<'_, 'a, A> {
        SecurityIter {
            security: self,
            next: 0,
        }
    }
}

struct SecurityIter<'list, 'a, A: SemanticAccess<'a> + ?Sized> {
    security: &'list EffectiveSecurity<'a, A>,
    next: usize,
}

impl<'a, A: SemanticAccess<'a> + ?Sized> Clone for SecurityIter<'_, 'a, A> {
    fn clone(&self) -> Self {
        Self {
            security: self.security,
            next: self.next,
        }
    }
}

impl<'a, A: SemanticAccess<'a> + ?Sized> Iterator for SecurityIter<'_, 'a, A> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.security.get(self.next)?;
        self.next += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.security.len() - self.next;
        (remaining, Some(remaining))
    }
}

impl<'a, A: SemanticAccess<'a> + ?Sized> ExactSizeIterator for SecurityIter<'_, 'a, A> {}

fn effective_form_security<'a, A: SemanticAccess<'a> + ?Sized>(
    access: &'a A,
    form: A::Form,
) -> EffectiveSecurity<'a, A> {
    match access.explicit_form_security(form) {
        Some(names) => EffectiveSecurity {
            access,
            names,
            inherited: false,
        },
        None => EffectiveSecurity {
            access,
            names: access.thing_security(),
            inherited: true,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DefinitionGroup {
    OneOf,
    AllOf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SecurityReferenceSite {
    Thing,
    Form(FormSite),
    SecurityDefinition {
        ordinal: u32,
        group: DefinitionGroup,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SecurityRuleError<'a> {
    MissingThingSecurity,
    Undefined {
        site: SecurityReferenceSite,
        reference: &'a str,
    },
}

fn validate_name_sequence<'a, A: SemanticAccess<'a> + ?Sized>(
    access: &'a A,
    names: A::SecurityNames,
    site: SecurityReferenceSite,
) -> Result<(), SecurityRuleError<'a>> {
    for index in 0..access.security_name_count(names) {
        let reference = access.security_name_at(names, index).unwrap();
        if access.security_definition_by_name(reference).is_none() {
            return Err(SecurityRuleError::Undefined { site, reference });
        }
    }
    Ok(())
}

fn validate_security_references<'a, A: SemanticAccess<'a> + ?Sized>(
    access: &'a A,
) -> Result<(), SecurityRuleError<'a>> {
    let root = access.thing_security();
    if access.security_name_count(root) == 0 {
        return Err(SecurityRuleError::MissingThingSecurity);
    }
    validate_name_sequence(access, root, SecurityReferenceSite::Thing)?;

    for index in 0..access.security_definition_count() {
        let definition = access.security_definition_at(index).unwrap();
        if access.security_definition_scheme(definition) != "combo" {
            continue;
        }
        let ordinal = index.try_into().unwrap();
        if let Some(one_of) = access.security_definition_one_of(definition) {
            validate_name_sequence(
                access,
                one_of,
                SecurityReferenceSite::SecurityDefinition {
                    ordinal,
                    group: DefinitionGroup::OneOf,
                },
            )?;
        }
        if let Some(all_of) = access.security_definition_all_of(definition) {
            validate_name_sequence(
                access,
                all_of,
                SecurityReferenceSite::SecurityDefinition {
                    ordinal,
                    group: DefinitionGroup::AllOf,
                },
            )?;
        }
    }

    for index in 0..access.all_form_count() {
        let located = access.all_form_at(index).unwrap();
        if let Some(names) = access.explicit_form_security(located.form) {
            validate_name_sequence(access, names, SecurityReferenceSite::Form(located.site))?;
        }
    }
    Ok(())
}

fn property_named<'a, A: SemanticAccess<'a> + ?Sized>(access: &'a A, name: &str) -> A::Property {
    (0..access.property_count())
        .find_map(|index| {
            let property = access.property_at(index).unwrap();
            (access.property_name(property) == name).then_some(property)
        })
        .unwrap()
}

fn assert_sequence_eq<T: Copy + core::fmt::Debug + Eq>(
    left_len: usize,
    mut left: impl FnMut(usize) -> Option<T>,
    right_len: usize,
    mut right: impl FnMut(usize) -> Option<T>,
) {
    assert_eq!(left_len, right_len);
    for index in 0..left_len {
        assert_eq!(left(index), right(index));
    }
}

fn assert_semantic_parity<'a, L, R>(left: &'a L, right: &'a R)
where
    L: SemanticAccess<'a> + ?Sized,
    R: SemanticAccess<'a> + ?Sized,
{
    assert_eq!(left.property_count(), right.property_count());
    for property_index in 0..left.property_count() {
        let left_property = left.property_at(property_index).unwrap();
        let right_property = right.property_at(property_index).unwrap();
        assert_eq!(
            left.property_name(left_property),
            right.property_name(right_property)
        );
        assert_eq!(
            left.property_flags(left_property),
            right.property_flags(right_property)
        );
        assert_eq!(
            left.property_form_count(left_property),
            right.property_form_count(right_property)
        );
        for form_index in 0..left.property_form_count(left_property) {
            let left_form = left.property_form_at(left_property, form_index).unwrap();
            let right_form = right.property_form_at(right_property, form_index).unwrap();
            let left_operations = effective_property_operations(left, left_property, left_form);
            let right_operations = effective_property_operations(right, right_property, right_form);
            assert_eq!(
                left_operations.is_explicit(),
                right_operations.is_explicit()
            );
            assert_sequence_eq(
                left_operations.len(),
                |index| left_operations.get(index),
                right_operations.len(),
                |index| right_operations.get(index),
            );

            let left_security = effective_form_security(left, left_form);
            let right_security = effective_form_security(right, right_form);
            assert_eq!(left_security.is_inherited(), right_security.is_inherited());
            assert_sequence_eq(
                left_security.len(),
                |index| left_security.get(index),
                right_security.len(),
                |index| right_security.get(index),
            );
        }
    }

    assert_eq!(left.all_form_count(), right.all_form_count());
    for index in 0..left.all_form_count() {
        let left_form = left.all_form_at(index).unwrap();
        let right_form = right.all_form_at(index).unwrap();
        assert_eq!(left_form.site, right_form.site);
        let left_security = effective_form_security(left, left_form.form);
        let right_security = effective_form_security(right, right_form.form);
        assert_eq!(left_security.is_inherited(), right_security.is_inherited());
        assert_sequence_eq(
            left_security.len(),
            |index| left_security.get(index),
            right_security.len(),
            |index| right_security.get(index),
        );
    }

    assert_eq!(
        left.security_definition_count(),
        right.security_definition_count()
    );
    for index in 0..left.security_definition_count() {
        let left_definition = left.security_definition_at(index).unwrap();
        let right_definition = right.security_definition_at(index).unwrap();
        assert_eq!(
            left.security_definition_name(left_definition),
            right.security_definition_name(right_definition)
        );
        assert_eq!(
            left.security_definition_scheme(left_definition),
            right.security_definition_scheme(right_definition)
        );
    }
    assert_eq!(
        validate_security_references(left),
        validate_security_references(right)
    );
}

fn assert_current_thing_oracle(thing: &Thing) {
    if let Some(properties) = &thing.properties {
        for property in properties.values() {
            let property_handle = ThingProperty {
                name: "oracle",
                property,
            };
            for form in &property._interaction.forms {
                let shared = effective_property_operations(thing, property_handle, form);
                let current =
                    td_defaults::effective_form_operations(FormContext::Property(property), form);
                assert_sequence_eq(
                    shared.len(),
                    |index| shared.get(index),
                    current.len(),
                    |index| current.get(index).copied(),
                );

                let shared = effective_form_security(thing, form);
                let current = td_defaults::effective_form_security(thing, form);
                assert_sequence_eq(
                    shared.len(),
                    |index| shared.get(index),
                    current.len(),
                    |index| current.get(index).map(String::as_str),
                );
            }
        }
    }
}

fn schema_context_mut(schema: &mut DataSchema) -> &mut DataSchemaContext {
    match schema {
        DataSchema::Array(value) => &mut value._context,
        DataSchema::Boolean(value) => &mut value._context,
        DataSchema::Number(value) => &mut value._context,
        DataSchema::Integer(value) => &mut value._context,
        DataSchema::Object(value) => &mut value._context,
        DataSchema::String(value) => &mut value._context,
        DataSchema::Null(value) => &mut value._context,
    }
}

fn alpha_property_mut(thing: &mut Thing) -> &mut PropertyAffordance {
    thing.properties.as_mut().unwrap().get_mut("alpha").unwrap()
}

fn zeta_form_mut(thing: &mut Thing, index: usize) -> &mut Form {
    &mut thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[index]
}

fn assert_effective_operations<A: for<'a> SemanticAccess<'a> + ?Sized>(
    access: &A,
    property_name: &str,
    form_index: usize,
    explicit: bool,
    expected: &[Operation],
) {
    let property = property_named(access, property_name);
    let form = access.property_form_at(property, form_index).unwrap();
    let operations = effective_property_operations(access, property, form);
    assert_eq!(operations.is_explicit(), explicit);
    assert_eq!(operations.len(), expected.len());
    assert_eq!(operations.iter().len(), expected.len());
    let mut cloned = operations.iter().clone();
    for expected in expected {
        assert_eq!(cloned.next(), Some(*expected));
    }
    assert_eq!(cloned.next(), None);
}

fn assert_effective_security<A: for<'a> SemanticAccess<'a> + ?Sized>(
    access: &A,
    property_name: &str,
    form_index: usize,
    inherited: bool,
    expected: &[&str],
) {
    let property = property_named(access, property_name);
    let form = access.property_form_at(property, form_index).unwrap();
    let security = effective_form_security(access, form);
    assert_eq!(security.is_inherited(), inherited);
    assert_eq!(security.len(), expected.len());
    assert_eq!(security.iter().len(), expected.len());
    let mut cloned = security.iter().clone();
    for expected in expected {
        assert_eq!(cloned.next(), Some(*expected));
    }
    assert_eq!(cloned.next(), None);
}

fn assert_basic_invalid_reference(thing: &Thing, expected: &str) {
    assert!(matches!(
        thing.validate_with_level(ValidationLevel::Basic),
        Err(ValidateError::InvalidReference { reference, .. }) if reference == expected
    ));
}

// Frozen-API-shaped borrowed views over the test-only Snapshot. These wrappers
// deliberately expose no node, edge, byte range, Thing, or semantic adapter.
#[derive(Clone, Copy)]
pub(super) struct ValidatedThingView<'a> {
    snapshot: &'a Snapshot,
}

#[derive(Clone, Copy)]
pub(super) struct ValidatedPropertyView<'a> {
    snapshot: &'a Snapshot,
    property: SnapshotProperty,
    ordinal: u32,
}

#[derive(Clone, Copy)]
pub(super) struct ValidatedFormView<'a> {
    snapshot: &'a Snapshot,
    property: SnapshotProperty,
    form: u32,
    original_index: u32,
}

#[derive(Clone, Copy)]
pub(super) struct ValidatedSecuritySchemeView<'a> {
    snapshot: &'a Snapshot,
    definition: SnapshotSecurityDefinition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ValidatedFormHref<'a> {
    Reference(&'a str),
    Template(&'a str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ValidatedFormHrefError {
    TemplateBase,
    Resolution,
}

#[derive(Clone, Copy)]
struct PropertyViews<'a> {
    view: ValidatedThingView<'a>,
    next: usize,
}

impl<'a> Iterator for PropertyViews<'a> {
    type Item = ValidatedPropertyView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let ordinal = self.next;
        let property = self.view.snapshot.property_at(ordinal)?;
        self.next += 1;
        Some(ValidatedPropertyView {
            snapshot: self.view.snapshot,
            property,
            ordinal: ordinal.try_into().ok()?,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.view.snapshot.property_count() - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PropertyViews<'_> {}

#[derive(Clone, Copy)]
struct FormViews<'a> {
    property: ValidatedPropertyView<'a>,
    next: usize,
}

impl<'a> Iterator for FormViews<'a> {
    type Item = ValidatedFormView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let original_index = self.next;
        let form = self
            .property
            .snapshot
            .property_form_at(self.property.property, original_index)?;
        self.next += 1;
        Some(ValidatedFormView {
            snapshot: self.property.snapshot,
            property: self.property.property,
            form,
            original_index: original_index.try_into().ok()?,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self
            .property
            .snapshot
            .property_form_count(self.property.property)
            - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for FormViews<'_> {}

#[derive(Clone, Copy)]
struct TextSequence<'a> {
    snapshot: &'a Snapshot,
    sequence: Option<u32>,
    next: usize,
    length: usize,
}

impl<'a> Iterator for TextSequence<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let sequence = self.sequence?;
        if self.next == self.length {
            return None;
        }
        let value = self.snapshot.text(self.snapshot.child(sequence, self.next));
        self.next += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.length - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TextSequence<'_> {}

#[derive(Clone, Copy)]
enum ViewOperationSource {
    Explicit(u32),
    Default(&'static [Operation]),
}

#[derive(Clone, Copy)]
struct ViewOperations<'a> {
    snapshot: &'a Snapshot,
    source: ViewOperationSource,
    next: usize,
    length: usize,
}

impl Iterator for ViewOperations<'_> {
    type Item = Operation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.length {
            return None;
        }
        let value = match self.source {
            ViewOperationSource::Explicit(operations) => {
                self.snapshot.operation_at(operations, self.next)
            }
            ViewOperationSource::Default(operations) => operations.get(self.next).copied(),
        }?;
        self.next += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.length - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ViewOperations<'_> {}

#[derive(Clone, Copy)]
struct ViewSecurity<'a> {
    snapshot: &'a Snapshot,
    names: SnapshotSecurityNames,
    next: usize,
    length: usize,
}

impl<'a> Iterator for ViewSecurity<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.length {
            return None;
        }
        let value = self.snapshot.security_name_at(self.names, self.next)?;
        self.next += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.length - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ViewSecurity<'_> {}

impl<'a> ValidatedThingView<'a> {
    fn new(snapshot: &'a Snapshot) -> Self {
        Self { snapshot }
    }

    pub(super) fn id(self) -> Option<&'a str> {
        let id = self.snapshot.child(self.snapshot.root, ROOT_ID);
        (self.snapshot.kind(id) != Kind::Absent).then(|| self.snapshot.text(id))
    }

    pub(super) fn property(self, name: &str) -> Option<ValidatedPropertyView<'a>> {
        self.properties().find(|property| property.name() == name)
    }

    pub(super) fn properties(
        self,
    ) -> impl ExactSizeIterator<Item = ValidatedPropertyView<'a>> + Clone + 'a {
        PropertyViews {
            view: self,
            next: 0,
        }
    }

    pub(super) fn security_definition(self, name: &str) -> Option<ValidatedSecuritySchemeView<'a>> {
        self.snapshot
            .security_definition_by_name(name)
            .map(|definition| ValidatedSecuritySchemeView {
                snapshot: self.snapshot,
                definition,
            })
    }
}

impl<'a> ValidatedPropertyView<'a> {
    pub(super) const fn ordinal(self) -> u32 {
        self.ordinal
    }

    pub(super) fn name(self) -> &'a str {
        self.snapshot.property_name(self.property)
    }

    pub(super) fn form(self, original_index: u32) -> Option<ValidatedFormView<'a>> {
        let index: usize = original_index.try_into().ok()?;
        let form = self.snapshot.property_form_at(self.property, index)?;
        Some(ValidatedFormView {
            snapshot: self.snapshot,
            property: self.property,
            form,
            original_index,
        })
    }

    pub(super) fn forms(self) -> impl ExactSizeIterator<Item = ValidatedFormView<'a>> + Clone + 'a {
        FormViews {
            property: self,
            next: 0,
        }
    }
}

impl<'a> ValidatedFormView<'a> {
    pub(super) const fn original_index(self) -> u32 {
        self.original_index
    }

    pub(super) fn href(self) -> ValidatedFormHref<'a> {
        match self.snapshot.form_href(self.form) {
            super::BorrowedHref::Reference(value) => ValidatedFormHref::Reference(value),
            super::BorrowedHref::Template(value) => ValidatedFormHref::Template(value),
        }
    }

    pub(super) fn resolved_href(self) -> Result<ValidatedFormHref<'a>, ValidatedFormHrefError> {
        match self.snapshot.resolved_form_href(self.form) {
            Ok(super::BorrowedHref::Reference(value)) => Ok(ValidatedFormHref::Reference(value)),
            Ok(super::BorrowedHref::Template(value)) => Ok(ValidatedFormHref::Template(value)),
            Err(super::ResolveIntoError::TemplateBase) => Err(ValidatedFormHrefError::TemplateBase),
            Err(super::ResolveIntoError::Resolution | super::ResolveIntoError::Capacity) => {
                Err(ValidatedFormHrefError::Resolution)
            }
        }
    }

    pub(super) fn content_type(self) -> &'a str {
        self.snapshot
            .text(self.snapshot.child(self.form, FORM_CONTENT_TYPE))
    }

    pub(super) fn content_coding(self) -> Option<&'a str> {
        let value = self.snapshot.child(self.form, FORM_CONTENT_CODING);
        (self.snapshot.kind(value) != Kind::Absent).then(|| self.snapshot.text(value))
    }

    pub(super) fn subprotocol(self) -> Option<&'a str> {
        let value = self.snapshot.child(self.form, FORM_SUBPROTOCOL);
        (self.snapshot.kind(value) != Kind::Absent).then(|| self.snapshot.text(value))
    }

    pub(super) fn scopes(self) -> impl ExactSizeIterator<Item = &'a str> + Clone + 'a {
        let sequence = self
            .snapshot
            .optional_sequence(self.snapshot.child(self.form, FORM_SCOPES));
        let length = sequence.map_or(0, |sequence| self.snapshot.sequence_len(sequence));
        TextSequence {
            snapshot: self.snapshot,
            sequence,
            next: 0,
            length,
        }
    }

    pub(super) fn effective_operations(
        self,
    ) -> impl ExactSizeIterator<Item = Operation> + Clone + 'a {
        let effective = effective_property_operations(self.snapshot, self.property, self.form);
        let source = match effective.source {
            OperationSource::Explicit(operations) => ViewOperationSource::Explicit(operations),
            OperationSource::Default(operations) => ViewOperationSource::Default(operations),
        };
        ViewOperations {
            snapshot: self.snapshot,
            source,
            next: 0,
            length: effective.len(),
        }
    }

    pub(super) fn effective_security(self) -> impl ExactSizeIterator<Item = &'a str> + Clone + 'a {
        let effective = effective_form_security(self.snapshot, self.form);
        ViewSecurity {
            snapshot: self.snapshot,
            names: effective.names,
            next: 0,
            length: effective.len(),
        }
    }
}

impl<'a> ValidatedSecuritySchemeView<'a> {
    pub(super) fn name(self) -> &'a str {
        self.snapshot.security_definition_name(self.definition)
    }

    pub(super) fn scheme(self) -> &'a str {
        self.snapshot.security_definition_scheme(self.definition)
    }
}

#[path = "../../../tools/architecture-fixtures/validated-thing-arena-layout/planning_view_consumer.rs"]
mod planning_view_consumer;

#[path = "planning_handoff_probe.rs"]
mod planning_handoff_probe;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlanningProbe {
    properties: u32,
    readable_forms: u32,
    all_readable_forms_have_one_nosec: bool,
    non_first_coordinate: Option<(u32, u32)>,
}

/// Local stand-in for the future external Planning consumer. It only uses the
/// storage-neutral access contract and TD-owned effective results.
fn planning_probe<'a, A: SemanticAccess<'a> + ?Sized>(access: &'a A) -> PlanningProbe {
    let mut readable_forms = 0_u32;
    let mut all_have_one_nosec = true;
    let mut non_first_coordinate = None;
    for property_index in 0..access.property_count() {
        let property = access.property_at(property_index).unwrap();
        for form_index in 0..access.property_form_count(property) {
            let form = access.property_form_at(property, form_index).unwrap();
            let operations = effective_property_operations(access, property, form);
            if !operations
                .iter()
                .any(|operation| operation == Operation::ReadProperty)
            {
                continue;
            }
            readable_forms += 1;
            let security = effective_form_security(access, form);
            let one_nosec = security.len() == 1
                && security.get(0).is_some_and(|name| {
                    access
                        .security_definition_by_name(name)
                        .is_some_and(|definition| {
                            access.security_definition_scheme(definition) == "nosec"
                        })
                });
            all_have_one_nosec &= one_nosec;
            if property_index > 0 && form_index > 0 {
                non_first_coordinate = Some((
                    property_index.try_into().unwrap(),
                    form_index.try_into().unwrap(),
                ));
            }
        }
    }
    PlanningProbe {
        properties: access.property_count().try_into().unwrap(),
        readable_forms,
        all_readable_forms_have_one_nosec: all_have_one_nosec,
        non_first_coordinate,
    }
}

fn hash_text(mut hash: u64, value: &str) -> u64 {
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn semantic_fingerprint<'a, A: SemanticAccess<'a> + ?Sized>(access: &'a A) -> u64 {
    let mut hash = 14_695_981_039_346_656_037;
    for property_index in 0..access.property_count() {
        let property = access.property_at(property_index).unwrap();
        hash = hash_text(hash, access.property_name(property));
        let (read_only, write_only) = access.property_flags(property);
        hash ^= u64::from(read_only) | (u64::from(write_only) << 1);
        for form_index in 0..access.property_form_count(property) {
            let form = access.property_form_at(property, form_index).unwrap();
            let operations = effective_property_operations(access, property, form);
            for operation in operations.iter() {
                hash = hash_text(hash, operation.as_str());
            }
            let security = effective_form_security(access, form);
            hash ^= u64::from(security.is_inherited());
            for name in security.iter() {
                hash = hash_text(hash, name);
                let definition = access.security_definition_by_name(name).unwrap();
                hash = hash_text(hash, access.security_definition_scheme(definition));
            }
        }
    }
    hash
}

thread_local! {
    static COUNT_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_TRACE: Cell<AllocationTrace> = const { Cell::new(AllocationTrace::EMPTY) };
    static WATCHED_ALLOCATIONS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

#[derive(Clone, Copy, Debug)]
struct AllocationTrace {
    allocation_calls: usize,
    deallocation_calls: usize,
    deallocated_bytes: usize,
    live_change: i64,
    peak_additional_bytes: usize,
    watched_deallocations: [usize; 3],
}

impl AllocationTrace {
    const EMPTY: Self = Self {
        allocation_calls: 0,
        deallocation_calls: 0,
        deallocated_bytes: 0,
        live_change: 0,
        peak_additional_bytes: 0,
        watched_deallocations: [0; 3],
    };
}

fn record_allocation(bytes: usize, previous: Option<usize>, succeeded: bool) {
    COUNT_ALLOCATIONS.with(|enabled| {
        if enabled.get() {
            ALLOCATION_TRACE.with(|trace| {
                let mut value = trace.get();
                value.allocation_calls += 1;
                if succeeded {
                    value.live_change += bytes as i64 - previous.unwrap_or(0) as i64;
                }
                value.peak_additional_bytes = value
                    .peak_additional_bytes
                    .max(value.live_change.max(0) as usize);
                trace.set(value);
            });
        }
    });
}

fn record_deallocation(bytes: usize) {
    COUNT_ALLOCATIONS.with(|enabled| {
        if enabled.get() {
            ALLOCATION_TRACE.with(|trace| {
                let mut value = trace.get();
                value.deallocation_calls += 1;
                value.deallocated_bytes += bytes;
                value.live_change -= bytes as i64;
                trace.set(value);
            });
        }
    });
}

struct ThreadCountingAllocator;

unsafe impl GlobalAlloc for ThreadCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the caller-provided layout to the system allocator.
        let pointer = unsafe { System.alloc(layout) };
        record_allocation(layout.size(), None, !pointer.is_null());
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the caller-provided layout to the system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        record_allocation(layout.size(), None, !pointer.is_null());
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        COUNT_ALLOCATIONS.with(|enabled| {
            if enabled.get() {
                WATCHED_ALLOCATIONS.with(|sites| {
                    ALLOCATION_TRACE.with(|trace| {
                        let mut value = trace.get();
                        for (index, site) in sites.get().into_iter().enumerate() {
                            if site != 0 && site == pointer as usize {
                                value.watched_deallocations[index] += 1;
                            }
                        }
                        trace.set(value);
                    });
                });
            }
        });
        // SAFETY: `pointer` and `layout` came from the forwarded system allocation.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarding the original allocation and requested size.
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        // A failed realloc leaves the old request live. A successful realloc
        // changes its requested size at this API boundary; allocator-internal
        // storage/overlap is outside this portable observation.
        record_allocation(new_size, Some(layout.size()), !replacement.is_null());
        replacement
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: ThreadCountingAllocator = ThreadCountingAllocator;

struct AllocationCountGuard;

impl Drop for AllocationCountGuard {
    fn drop(&mut self) {
        COUNT_ALLOCATIONS.with(|enabled| enabled.set(false));
    }
}

pub(super) fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    let (output, trace) = trace_allocations(operation);
    (output, trace.allocation_calls)
}

fn trace_allocations<T>(operation: impl FnOnce() -> T) -> (T, AllocationTrace) {
    trace_allocations_watching([0; 3], operation)
}

fn trace_allocations_watching<T>(
    sites: [usize; 3],
    operation: impl FnOnce() -> T,
) -> (T, AllocationTrace) {
    ALLOCATION_TRACE.with(|trace| trace.set(AllocationTrace::EMPTY));
    WATCHED_ALLOCATIONS.with(|watched| watched.set(sites));
    COUNT_ALLOCATIONS
        .with(|enabled| assert!(!enabled.replace(true), "allocation counting must not nest"));
    let guard = AllocationCountGuard;
    let output = operation();
    drop(guard);
    let trace = ALLOCATION_TRACE.with(Cell::get);
    (output, trace)
}

#[test]
fn semantic_kernel_typed_corpus_matches_thing_snapshot_and_current_rules() {
    let thing = super::typed_corpus_shared::typed_corpus();
    let snapshot = Snapshot::normalize(&thing);

    assert_semantic_parity(&thing, &snapshot);
    assert_current_thing_oracle(&thing);
    assert_eq!(validate_security_references(&thing), Ok(()));
    assert_eq!(validate_security_references(&snapshot), Ok(()));

    assert_effective_operations(
        &thing,
        "zeta",
        0,
        true,
        &[Operation::ReadProperty, Operation::WriteProperty],
    );
    assert_effective_operations(
        &snapshot,
        "zeta",
        0,
        true,
        &[Operation::ReadProperty, Operation::WriteProperty],
    );
    assert_effective_security(&thing, "zeta", 1, true, &["none"]);
    assert_effective_security(&snapshot, "zeta", 1, true, &["none"]);
    assert_effective_security(&thing, "zeta", 0, false, &["none", "none_alt"]);
    assert_effective_security(&snapshot, "zeta", 0, false, &["none", "none_alt"]);
}

#[test]
fn semantic_kernel_property_default_and_explicit_operation_mutations() {
    for (read_only, write_only, expected) in [
        (false, false, PROPERTY_READ_WRITE_OPERATIONS),
        (true, false, PROPERTY_READ_OPERATIONS),
        (false, true, PROPERTY_WRITE_OPERATIONS),
        // Current Minimal/default helper deliberately keeps this neutral
        // fallback even though Basic rejects both flags being true.
        (true, true, PROPERTY_READ_WRITE_OPERATIONS),
    ] {
        let mut thing = super::typed_corpus_shared::typed_corpus();
        let alpha = alpha_property_mut(&mut thing);
        alpha._interaction.forms[0].op = None;
        let context = schema_context_mut(&mut alpha._schema);
        context.read_only = read_only;
        context.write_only = write_only;
        let snapshot = Snapshot::normalize(&thing);

        assert_semantic_parity(&thing, &snapshot);
        assert_current_thing_oracle(&thing);
        assert_effective_operations(&thing, "alpha", 0, false, expected);
        assert_effective_operations(&snapshot, "alpha", 0, false, expected);
        if read_only && write_only {
            assert!(thing.validate_with_level(ValidationLevel::Basic).is_err());
        } else {
            thing.validate_with_level(ValidationLevel::Basic).unwrap();
        }
    }

    let mut thing = super::typed_corpus_shared::typed_corpus();
    alpha_property_mut(&mut thing)._interaction.forms[0].op =
        Some(vec![Operation::WriteProperty, Operation::ReadProperty]);
    let snapshot = Snapshot::normalize(&thing);
    assert_semantic_parity(&thing, &snapshot);
    assert_effective_operations(
        &snapshot,
        "alpha",
        0,
        true,
        &[Operation::WriteProperty, Operation::ReadProperty],
    );

    alpha_property_mut(&mut thing)._interaction.forms[0].op = Some(Vec::new());
    let snapshot = Snapshot::normalize(&thing);
    assert_semantic_parity(&thing, &snapshot);
    assert_effective_operations(&thing, "alpha", 0, true, NO_OPERATIONS);
    assert_effective_operations(&snapshot, "alpha", 0, true, NO_OPERATIONS);
}

#[test]
fn semantic_kernel_security_inheritance_empty_override_and_reference_negatives() {
    let mut thing = super::typed_corpus_shared::typed_corpus();
    zeta_form_mut(&mut thing, 1).security = Some(Vec::new());
    thing.validate_with_level(ValidationLevel::Basic).unwrap();
    let snapshot = Snapshot::normalize(&thing);
    assert_semantic_parity(&thing, &snapshot);
    assert_effective_security(&thing, "zeta", 1, false, &[]);
    assert_effective_security(&snapshot, "zeta", 1, false, &[]);

    let mut missing_root = super::typed_corpus_shared::typed_corpus();
    missing_root.security[0] = "missing-root".into();
    let snapshot = Snapshot::normalize(&missing_root);
    assert_semantic_parity(&missing_root, &snapshot);
    assert_eq!(
        validate_security_references(&snapshot),
        Err(SecurityRuleError::Undefined {
            site: SecurityReferenceSite::Thing,
            reference: "missing-root",
        })
    );
    assert_basic_invalid_reference(&missing_root, "missing-root");

    let mut missing_form = super::typed_corpus_shared::typed_corpus();
    zeta_form_mut(&mut missing_form, 1).security = Some(vec!["missing-form".into()]);
    let snapshot = Snapshot::normalize(&missing_form);
    assert_semantic_parity(&missing_form, &snapshot);
    assert!(matches!(
        validate_security_references(&snapshot),
        Err(SecurityRuleError::Undefined {
            site: SecurityReferenceSite::Form(FormSite {
                owner: FormOwner::Property(1),
                original_index: 1,
            }),
            reference: "missing-form",
        })
    ));
    assert_basic_invalid_reference(&missing_form, "missing-form");

    let mut missing_combo = super::typed_corpus_shared::typed_corpus();
    let SecurityScheme::Combo(combo) = missing_combo
        .security_definitions
        .get_mut("combined")
        .unwrap()
    else {
        panic!("combined must be combo");
    };
    combo.one_of[0] = "missing-combo".into();
    let snapshot = Snapshot::normalize(&missing_combo);
    assert_semantic_parity(&missing_combo, &snapshot);
    assert!(matches!(
        validate_security_references(&snapshot),
        Err(SecurityRuleError::Undefined {
            site: SecurityReferenceSite::SecurityDefinition {
                group: DefinitionGroup::OneOf,
                ..
            },
            reference: "missing-combo",
        })
    ));
    assert_basic_invalid_reference(&missing_combo, "missing-combo");

    let mut empty_root = super::typed_corpus_shared::typed_corpus();
    empty_root.security.clear();
    let snapshot = Snapshot::normalize(&empty_root);
    assert_semantic_parity(&empty_root, &snapshot);
    assert_eq!(
        validate_security_references(&snapshot),
        Err(SecurityRuleError::MissingThingSecurity)
    );
    assert!(matches!(
        empty_root.validate_with_level(ValidationLevel::Basic),
        Err(ValidateError::MissingRequiredField(field)) if field == "security"
    ));
}

#[test]
fn semantic_kernel_security_reference_dispatch_follows_mutable_scheme_discriminator() {
    // Current Basic dispatches combo behavior from the mutable `scheme`
    // discriminator. For a non-Combo Rust variant it reads oneOf/allOf from
    // extension fields, so the storage adapters must not dispatch only on the
    // enum variant that happened to carry the typed value.
    let mut disguised_combo = super::typed_corpus_shared::typed_corpus();
    let SecurityScheme::Auto(disguised) = disguised_combo
        .security_definitions
        .get_mut("automatic")
        .unwrap()
    else {
        panic!("automatic must retain its Auto variant");
    };
    disguised._context.scheme = "combo".into();
    disguised
        ._context
        ._extra_fields
        .insert("oneOf".into(), serde_json::json!(["none", "none_alt"]));
    disguised_combo
        .validate_with_level(ValidationLevel::Basic)
        .unwrap();
    let snapshot = Snapshot::normalize(&disguised_combo);
    assert_semantic_parity(&disguised_combo, &snapshot);
    assert_eq!(validate_security_references(&snapshot), Ok(()));

    let SecurityScheme::Auto(disguised) = disguised_combo
        .security_definitions
        .get_mut("automatic")
        .unwrap()
    else {
        panic!("automatic must retain its Auto variant");
    };
    disguised._context._extra_fields.insert(
        "oneOf".into(),
        serde_json::json!(["none", "missing-disguised-combo"]),
    );
    let snapshot = Snapshot::normalize(&disguised_combo);
    assert_semantic_parity(&disguised_combo, &snapshot);
    assert!(matches!(
        validate_security_references(&snapshot),
        Err(SecurityRuleError::Undefined {
            site: SecurityReferenceSite::SecurityDefinition {
                group: DefinitionGroup::OneOf,
                ..
            },
            reference: "missing-disguised-combo",
        })
    ));
    assert_basic_invalid_reference(&disguised_combo, "missing-disguised-combo");

    let (thing_rejected, thing_allocations) =
        count_allocations(|| validate_security_references(&disguised_combo).is_err());
    let (snapshot_rejected, snapshot_allocations) =
        count_allocations(|| validate_security_references(&snapshot).is_err());
    assert!(thing_rejected && snapshot_rejected);
    assert_eq!(thing_allocations, 0);
    assert_eq!(snapshot_allocations, 0);
}

#[test]
fn semantic_kernel_queries_and_planning_probe_allocate_nothing() {
    let mut thing = super::typed_corpus_shared::typed_corpus();
    zeta_form_mut(&mut thing, 0).security = Some(vec!["none".into()]);
    alpha_property_mut(&mut thing)._interaction.forms[0].op = None;
    let snapshot = Snapshot::normalize(&thing);
    assert_semantic_parity(&thing, &snapshot);

    let ((thing_fingerprint, thing_valid, thing_planning), thing_allocations) =
        count_allocations(|| {
            (
                semantic_fingerprint(&thing),
                validate_security_references(&thing).is_ok(),
                planning_probe(&thing),
            )
        });
    let ((snapshot_fingerprint, snapshot_valid, snapshot_planning), snapshot_allocations) =
        count_allocations(|| {
            (
                semantic_fingerprint(&snapshot),
                validate_security_references(&snapshot).is_ok(),
                planning_probe(&snapshot),
            )
        });

    assert_eq!(thing_allocations, 0);
    assert_eq!(snapshot_allocations, 0);
    assert_eq!(thing_fingerprint, snapshot_fingerprint);
    assert!(thing_valid && snapshot_valid);
    assert_eq!(thing_planning, snapshot_planning);
    assert_eq!(
        snapshot_planning,
        PlanningProbe {
            properties: 2,
            readable_forms: 3,
            all_readable_forms_have_one_nosec: true,
            non_first_coordinate: Some((1, 1)),
        }
    );
}

#[test]
fn frozen_borrowed_view_supplies_all_property_read_planning_queries_without_allocation() {
    let mut thing = super::typed_corpus_shared::typed_corpus();
    zeta_form_mut(&mut thing, 1).op = None;
    let (snapshot, preprocessing_allocations) = count_allocations(|| Snapshot::normalize(&thing));
    assert_eq!(preprocessing_allocations, 6);
    assert_eq!(snapshot.uri_cache_cost.derived_bytes, 284);
    assert_eq!(snapshot.uri_cache_cost.retained_requested_bytes(), 496);
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);

    let view = ValidatedThingView::new(&snapshot);
    let (selection, query_allocations) =
        count_allocations(|| planning_view_consumer::query_non_first_property_form(view));

    assert_eq!(query_allocations, 0);
    assert_eq!(selection.thing_id, "urn:example:typed-corpus");
    assert_eq!(selection.property_count, 2);
    assert_eq!(selection.property_name, "zeta");
    assert_eq!(selection.property_ordinal, 1);
    assert_eq!(selection.form_original_index, 1);
    assert_eq!(
        selection.raw_href,
        ValidatedFormHref::Reference("zeta/second")
    );
    assert_eq!(
        selection.resolved_href,
        ValidatedFormHref::Reference("https://example.org/things/zeta/second")
    );
    assert_eq!(selection.content_type, "application/json");
    assert_eq!(selection.content_coding, None);
    assert_eq!(selection.subprotocol, None);
    assert_eq!(selection.scope_count, 0);
    assert_eq!(selection.security_name, "none");
    assert_eq!(selection.security_scheme_name, "none");
    assert_eq!(selection.security_scheme, "nosec");
}
