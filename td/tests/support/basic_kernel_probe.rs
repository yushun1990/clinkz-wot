//! Complete Basic composition over the existing three-arena storage witness.
//! No new storage, decoding, production API, or admission transition.
use super::schema_kernel_probe::typed_access as schema_access;
use super::semantic_kernel_probe::{
    SemanticAccess, SnapshotSecurityDefinition, SnapshotSecurityNames,
};
use super::{
    ACTION_FORMS, ACTION_INPUT, ACTION_OUTPUT, ACTION_URI_VARIABLES, EVENT_CANCELLATION,
    EVENT_DATA, EVENT_DATA_RESPONSE, EVENT_FORMS, EVENT_SUBSCRIPTION, EVENT_URI_VARIABLES, Kind,
    PROPERTY_FORMS, PROPERTY_SCHEMA, PROPERTY_URI_VARIABLES, ROOT_ACTIONS, ROOT_EVENTS, ROOT_FORMS,
    ROOT_PROPERTIES, ROOT_SCHEMA_DEFINITIONS, ROOT_TITLE, ROOT_URI_VARIABLES,
    SECURITY_CONTEXT_EXTENSIONS, SECURITY_SCHEME_CONTEXT, SECURITY_SCHEME_VARIANT, Snapshot,
    schema_kernel_probe::{SnapshotAccess as SchemaSnapshotAccess, schema_kernel},
    semantic_kernel_probe, typed_corpus_shared,
};
use crate as td_crate;
use crate::{
    thing::Thing,
    validate::{Validate, ValidationLevel},
};

#[path = "../../../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_diagnostics.rs"]
mod basic_diagnostics;
#[path = "../../../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_kernel.rs"]
pub(super) mod basic_kernel;
#[path = "../../../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_typed.rs"]
mod basic_typed;
#[path = "basic_corpus_shared.rs"]
mod corpus;
#[path = "../../../tools/architecture-fixtures/validated-thing-schema-kernel/src/schema_diagnostics.rs"]
mod schema_diagnostics;
use alloc::{collections::BTreeMap, format};
use basic_kernel::{BasicAccess, Field, InlineInvalid, InlineSink, Owner, OwnerKind};

#[derive(Clone, Copy)]
struct StoredAffordance {
    kind: OwnerKind,
    node: u32,
}

struct SnapshotAccess<'a> {
    snapshot: &'a Snapshot,
    schemas: SchemaSnapshotAccess<'a>,
}

impl<'a> SnapshotAccess<'a> {
    fn new(snapshot: &'a Snapshot) -> Self {
        Self {
            snapshot,
            schemas: SchemaSnapshotAccess(snapshot),
        }
    }
    fn optional(&self, node: u32) -> Option<u32> {
        (self.snapshot.kind(node) != Kind::Absent).then_some(node)
    }
    fn text(&self, node: u32) -> Option<&'a str> {
        self.optional(node).map(|node| self.snapshot.text(node))
    }
    fn map(&self, kind: OwnerKind) -> u32 {
        self.snapshot.child(
            self.snapshot.root,
            match kind {
                OwnerKind::Property => ROOT_PROPERTIES,
                OwnerKind::Action => ROOT_ACTIONS,
                OwnerKind::Event => ROOT_EVENTS,
                _ => unreachable!(),
            },
        )
    }
    fn extension_string(
        &self,
        definition: SnapshotSecurityDefinition,
        field: Field,
    ) -> Option<&'a str> {
        let s = self.snapshot;
        let context = s.child(definition.definition, SECURITY_SCHEME_CONTEXT);
        let node = s.map_get(s.child(context, SECURITY_CONTEXT_EXTENSIONS), field.name())?;
        (s.kind(node) == Kind::Str).then(|| s.text(node))
    }
}

impl<'a> BasicAccess<'a> for SnapshotAccess<'a> {
    type Schemas = SchemaSnapshotAccess<'a>;
    type SchemaMap = u32;
    type Affordance = StoredAffordance;
    type Forms = u32;
    type Form = u32;
    type Operations = u32;
    type Names = SnapshotSecurityNames;
    type Definition = SnapshotSecurityDefinition;
    fn schemas(&self) -> &Self::Schemas {
        &self.schemas
    }
    fn title(&self) -> Option<&'a str> {
        self.text(self.snapshot.child(self.snapshot.root, ROOT_TITLE))
    }
    fn root_security(&self) -> Self::Names {
        self.snapshot.thing_security()
    }
    fn names_count(&self, names: Self::Names) -> usize {
        self.snapshot.security_name_count(names)
    }
    fn name_at(&self, names: Self::Names, index: usize) -> &'a str {
        self.snapshot.security_name_at(names, index).unwrap()
    }
    fn definition_count(&self) -> usize {
        self.snapshot.security_definition_count()
    }
    fn definition_at(&self, index: usize) -> Self::Definition {
        self.snapshot.security_definition_at(index).unwrap()
    }
    fn definition_name(&self, index: usize) -> &'a str {
        self.snapshot
            .security_definition_name(self.definition_at(index))
    }
    fn definition_exists(&self, name: &str) -> bool {
        self.snapshot.security_definition_by_name(name).is_some()
    }
    fn scheme(&self, definition: Self::Definition) -> &'a str {
        self.snapshot.security_definition_scheme(definition)
    }
    fn combo_names(&self, definition: Self::Definition, field: Field) -> Self::Names {
        match field {
            Field::OneOf => self
                .snapshot
                .security_definition_one_of(definition)
                .unwrap(),
            Field::AllOf => self
                .snapshot
                .security_definition_all_of(definition)
                .unwrap(),
            _ => unreachable!(),
        }
    }
    fn security_string(&self, definition: Self::Definition, field: Field) -> Option<&'a str> {
        let s = self.snapshot;
        let variant = s.child(definition.definition, SECURITY_SCHEME_VARIANT);
        match (s.kind(variant), field) {
            (Kind::SecurityApiKey, Field::Name) => self.text(s.child(variant, 0)),
            (Kind::SecurityOAuth2, Field::Flow) => self.text(s.child(variant, 4)),
            _ => self.extension_string(definition, field),
        }
    }
    fn endpoint_present(&self, definition: Self::Definition, field: Field) -> bool {
        let s = self.snapshot;
        let variant = s.child(definition.definition, SECURITY_SCHEME_VARIANT);
        if s.kind(variant) == Kind::SecurityOAuth2 {
            self.optional(s.child(
                variant,
                match field {
                    Field::Authorization => 0,
                    Field::Token => 1,
                    _ => unreachable!(),
                },
            ))
            .is_some()
        } else {
            self.extension_string(definition, field)
                .is_some_and(|value| !value.is_empty())
        }
    }
    fn root_schema_map(&self, field: Field) -> Option<u32> {
        self.optional(self.snapshot.child(
            self.snapshot.root,
            match field {
                Field::SchemaDefinitions => ROOT_SCHEMA_DEFINITIONS,
                Field::UriVariables => ROOT_URI_VARIABLES,
                _ => unreachable!(),
            },
        ))
    }
    fn schema_count(&self, map: u32) -> usize {
        self.snapshot.node(map).edge_count as usize
    }
    fn schema_at(&self, map: u32, index: usize) -> u32 {
        self.snapshot.map_pair(map, index).unwrap().1
    }
    fn schema_name(&self, map: u32, index: usize) -> &'a str {
        self.snapshot
            .text(self.snapshot.map_pair(map, index).unwrap().0)
    }
    fn affordance_count(&self, kind: OwnerKind) -> usize {
        self.optional(self.map(kind))
            .map_or(0, |map| self.schema_count(map))
    }
    fn affordance_at(&self, kind: OwnerKind, index: usize) -> StoredAffordance {
        StoredAffordance {
            kind,
            node: self.schema_at(self.map(kind), index),
        }
    }
    fn affordance_name(&self, owner: Owner) -> &'a str {
        self.schema_name(self.map(owner.kind), owner.ordinal)
    }
    fn uri_variables(&self, affordance: StoredAffordance) -> Option<u32> {
        self.optional(self.snapshot.child(
            affordance.node,
            match affordance.kind {
                OwnerKind::Property => PROPERTY_URI_VARIABLES,
                OwnerKind::Action => ACTION_URI_VARIABLES,
                OwnerKind::Event => EVENT_URI_VARIABLES,
                _ => unreachable!(),
            },
        ))
    }
    fn affordance_schema(&self, affordance: StoredAffordance, field: Field) -> Option<u32> {
        self.optional(self.snapshot.child(
            affordance.node,
            match field {
                Field::PropertySchema => PROPERTY_SCHEMA,
                Field::Input => ACTION_INPUT,
                Field::Output => ACTION_OUTPUT,
                Field::Subscription => EVENT_SUBSCRIPTION,
                Field::Data => EVENT_DATA,
                Field::DataResponse => EVENT_DATA_RESPONSE,
                Field::Cancellation => EVENT_CANCELLATION,
                _ => unreachable!(),
            },
        ))
    }
    fn forms(&self, affordance: Option<StoredAffordance>) -> Option<u32> {
        let s = self.snapshot;
        self.optional(match affordance {
            Some(value) => s.child(
                value.node,
                match value.kind {
                    OwnerKind::Property => PROPERTY_FORMS,
                    OwnerKind::Action => ACTION_FORMS,
                    OwnerKind::Event => EVENT_FORMS,
                    _ => unreachable!(),
                },
            ),
            None => s.child(s.root, ROOT_FORMS),
        })
    }
    fn form_count(&self, forms: u32) -> usize {
        self.snapshot.node(forms).edge_count as usize
    }
    fn form_at(&self, forms: u32, index: usize) -> u32 {
        self.snapshot.child(forms, index)
    }
    fn operations(&self, form: u32) -> Option<u32> {
        self.snapshot.explicit_operations(form)
    }
    fn operation_count(&self, ops: u32) -> usize {
        self.snapshot.operation_count(ops)
    }
    fn operation_at(&self, ops: u32, index: usize) -> crate::data_type::Operation {
        self.snapshot.operation_at(ops, index).unwrap()
    }
    fn form_security(&self, form: u32) -> Option<Self::Names> {
        self.snapshot.explicit_form_security(form)
    }
}

fn assert_parity(thing: &Thing, oracle: bool) -> Result<(), InlineInvalid> {
    let snapshot = Snapshot::normalize(thing);
    let typed = basic_typed::TypedBasicAccess(Some(thing));
    let stored = SnapshotAccess::new(&snapshot);
    let (left, left_alloc) =
        semantic_kernel_probe::count_allocations(|| basic_kernel::validate(&typed, &InlineSink));
    let (right, right_alloc) =
        semantic_kernel_probe::count_allocations(|| basic_kernel::validate(&stored, &InlineSink));
    assert_eq!(left, right);
    assert_eq!((left_alloc, right_alloc), (0, 0));
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
    let sink = basic_diagnostics::PublicSink { document: true };
    let typed_public = basic_kernel::validate(&typed, &sink);
    let snapshot_public = basic_kernel::validate(&stored, &sink);
    assert_eq!(typed_public, snapshot_public);
    assert_eq!(left.is_ok(), typed_public.is_ok());
    if oracle {
        assert_eq!(
            typed_public,
            thing.validate_with_level(ValidationLevel::Basic)
        );
    }
    left
}

#[test]
fn basic_kernel_reuses_complete_typed_nested_and_serializer_failure_corpus() {
    for thing in [
        typed_corpus_shared::typed_corpus(),
        typed_corpus_shared::nested_schema_corpus(),
        typed_corpus_shared::serializer_failure_thing(),
    ] {
        assert!(assert_parity(&thing, true).is_ok());
    }
    assert!(serde_json::to_string(&typed_corpus_shared::serializer_failure_thing()).is_err());
}

#[test]
fn basic_kernel_complete_boundary_and_first_error_corpus_matches_both_stores() {
    for case in corpus::cases() {
        let result = assert_parity(&case.thing, true);
        assert_eq!(result.is_ok(), case.valid, "{}", case.label);
        if let Some(expected) = case.first {
            let site = result.unwrap_err().site;
            assert_eq!(
                format!("{:?}", site.owner.kind),
                expected.owner,
                "{}",
                case.label
            );
            assert_eq!(
                format!("{:?}", site.field),
                expected.field,
                "{}",
                case.label
            );
            assert_eq!(
                (site.owner.ordinal, site.index, site.member),
                (expected.ordinal, expected.index, expected.member),
                "{}",
                case.label
            );
        }
    }
    assert!(core::mem::size_of::<InlineInvalid>() <= 64);
    assert!(!core::mem::needs_drop::<InlineInvalid>());
}

#[test]
fn basic_kernel_numeric_amendment_participates_in_whole_document_order() {
    let mut thing = typed_corpus_shared::typed_corpus();
    thing.schema_definitions = Some(BTreeMap::from([(
        "probe".into(),
        serde_json::from_str(r#"{"type":"string","minimum":1e309}"#).unwrap(),
    )]));
    let error = assert_parity(&thing, false).unwrap_err();
    assert_eq!(error.site.field, Field::SchemaDefinitions);
    assert_eq!(
        error.rule,
        basic_kernel::InlineRule::Schema(schema_kernel::Rule::FailedProjection(
            schema_kernel::Field::Minimum
        ))
    );
    thing.security[0] = "missing".into();
    let error = assert_parity(&thing, true).unwrap_err();
    assert_eq!(error.site.field, Field::Security);
    thing._metadata.title = None;
    assert_eq!(
        assert_parity(&thing, true).unwrap_err().site.field,
        Field::Title
    );
}
