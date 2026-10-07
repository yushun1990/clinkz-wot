//! Connect the existing typed storage corpus to the shared schema rule source.
//! This adapter creates no Number/Value/Thing and copies no validation rule.
use super::{Kind, ROOT_SCHEMA_DEFINITIONS, Snapshot, typed_corpus_shared};
use crate::{
    data_schema::{DataSchema, DataSchemaContext},
    thing::Thing,
    validate::{Validate, ValidationLevel},
};
use alloc::{format, string::String, vec::Vec};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

#[path = "../../../tools/architecture-fixtures/validated-thing-schema-kernel/src/kernel.rs"]
pub(super) mod schema_kernel;
#[path = "../../../tools/architecture-fixtures/validated-thing-schema-kernel/src/typed_access.rs"]
pub(super) mod typed_access;
use schema_kernel::{
    ChildSite, Field, InlineInvalid, InlineSink, NumericCursor, Rule, SchemaAccess, SchemaKind,
    projection_step::{self, ProjectionProgress},
};
use typed_access::TypedAccess;

pub(super) struct SnapshotAccess<'a>(pub(super) &'a Snapshot);

impl SnapshotAccess<'_> {
    fn context(&self, node: u32, field: &str) -> u32 {
        self.0.map_get(self.0.child(node, 1), field).unwrap()
    }
    fn variant(&self, node: u32) -> u32 {
        self.0.child(node, 0)
    }
    fn count(&self, node: u32) -> usize {
        if self.0.kind(node) == Kind::Absent {
            0
        } else {
            self.0.node(node).edge_count as usize
        }
    }
    fn extension(&self, node: u32, field: Field) -> Option<u32> {
        self.0
            .map_get(self.context(node, "extensions"), field.name())
    }
    fn bits(&self, node: u32) -> Option<u64> {
        if self.0.kind(node) == Kind::Absent {
            return None;
        }
        let node = self.0.node(node);
        Some(u64::from_be_bytes(
            self.0.arena.bytes()
                [node.first_byte as usize..(node.first_byte + node.byte_count) as usize]
                .try_into()
                .unwrap(),
        ))
    }
    fn children(&self, node: u32) -> u32 {
        self.0.child(self.variant(node), 0)
    }
}

impl<'a> SchemaAccess<'a> for SnapshotAccess<'a> {
    type Node = u32;
    type Number = &'a str;
    fn kind(&self, node: u32) -> SchemaKind {
        match self.0.kind(self.variant(node)) {
            Kind::SchemaArray => SchemaKind::Array,
            Kind::SchemaBoolean => SchemaKind::Boolean,
            Kind::SchemaNumber => SchemaKind::Number,
            Kind::SchemaInteger => SchemaKind::Integer,
            Kind::SchemaObject => SchemaKind::Object,
            Kind::SchemaString => SchemaKind::String,
            Kind::SchemaNull => SchemaKind::Null,
            _ => unreachable!("not a stored schema"),
        }
    }
    fn data_type(&self, node: u32) -> Option<&'a str> {
        let value = self.context(node, "type");
        (self.0.kind(value) != Kind::Absent).then(|| self.0.text(value))
    }
    fn flags(&self, node: u32) -> (bool, bool) {
        (
            self.0.kind(self.context(node, "readOnly")) == Kind::True,
            self.0.kind(self.context(node, "writeOnly")) == Kind::True,
        )
    }
    fn one_of_count(&self, node: u32) -> usize {
        self.count(self.context(node, "oneOf"))
    }
    fn one_of(&self, node: u32, index: usize) -> u32 {
        self.0.child(self.context(node, "oneOf"), index)
    }
    fn child_count(&self, node: u32) -> usize {
        match SchemaAccess::kind(self, node) {
            SchemaKind::Array | SchemaKind::Object => self.count(self.children(node)),
            _ => 0,
        }
    }
    fn child(&self, node: u32, index: usize) -> (ChildSite<'a>, u32) {
        let child = self.0.child(self.children(node), index);
        if SchemaAccess::kind(self, node) == SchemaKind::Object {
            (
                ChildSite::Property(self.0.text(self.0.child(child, 0))),
                self.0.child(child, 1),
            )
        } else {
            (ChildSite::Indexed(index), child)
        }
    }
    fn unsigned_extension(&self, node: u32, field: Field) -> Option<u64> {
        let value = self.extension(node, field)?;
        (self.0.kind(value) == Kind::Number)
            .then(|| self.0.text(value).parse().ok())
            .flatten()
    }
    fn number_extension(&self, node: u32, field: Field) -> Option<&'a str> {
        let value = self.extension(node, field)?;
        (self.0.kind(value) == Kind::Number).then(|| self.0.text(value))
    }
    fn project_number(&self, number: &'a str) -> Option<f64> {
        // Public core parsing of AP text; no owned serde Number, serializer,
        // dependency-private parser, or alternate arithmetic model. Its parity
        // with public Number::as_f64 is independently tested below.
        number.parse::<f64>().ok().filter(|value| value.is_finite())
    }
    fn typed_unsigned(&self, node: u32) -> (Option<u32>, Option<u32>) {
        let first = if SchemaAccess::kind(self, node) == SchemaKind::Array {
            1
        } else {
            0
        };
        (
            self.bits(self.0.child(self.variant(node), first))
                .map(|v| v as u32),
            self.bits(self.0.child(self.variant(node), first + 1))
                .map(|v| v as u32),
        )
    }
    fn typed_float(&self, node: u32) -> [Option<f64>; 5] {
        core::array::from_fn(|index| {
            self.bits(self.0.child(self.variant(node), index))
                .map(f64::from_bits)
        })
    }
    fn typed_integer(&self, node: u32) -> [Option<i64>; 5] {
        core::array::from_fn(|index| {
            self.bits(self.0.child(self.variant(node), index))
                .map(|v| v as i64)
        })
    }
}

fn context_mut(schema: &mut DataSchema) -> &mut DataSchemaContext {
    match schema {
        DataSchema::Array(v) => &mut v._context,
        DataSchema::Boolean(v) => &mut v._context,
        DataSchema::Number(v) => &mut v._context,
        DataSchema::Integer(v) => &mut v._context,
        DataSchema::Object(v) => &mut v._context,
        DataSchema::String(v) => &mut v._context,
        DataSchema::Null(v) => &mut v._context,
    }
}

fn store(schema: DataSchema) -> (Thing, Snapshot, u32) {
    let mut thing = Thing::builder("schema evidence").nosec().build().unwrap();
    thing.schema_definitions = Some([("probe".into(), schema)].into());
    let snapshot = Snapshot::normalize(&thing);
    let root = snapshot
        .map_get(
            snapshot.child(snapshot.root, ROOT_SCHEMA_DEFINITIONS),
            "probe",
        )
        .unwrap();
    (thing, snapshot, root)
}

fn assert_parity(schema: DataSchema, current_oracle: bool) -> Result<(), InlineInvalid> {
    let (thing, snapshot, root) = store(schema);
    let typed = &thing.schema_definitions.as_ref().unwrap()["probe"];
    snapshot.assert_schema(root, typed);
    let (a, alloc_a) = super::semantic_kernel_probe::count_allocations(|| {
        schema_kernel::validate(&TypedAccess, typed, &InlineSink)
    });
    let (b, alloc_b) = super::semantic_kernel_probe::count_allocations(|| {
        schema_kernel::validate(&SnapshotAccess(&snapshot), root, &InlineSink)
    });
    assert_eq!(a, b);
    assert_eq!((alloc_a, alloc_b), (0, 0));
    assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
    if current_oracle {
        assert_eq!(
            a.is_ok(),
            typed.validate_with_level(ValidationLevel::Basic).is_ok()
        );
    }
    a
}

#[test]
fn schema_kernel_all_variants_context_rules_and_typed_constraints_share_one_body() {
    for kind in [
        "array", "boolean", "number", "integer", "object", "string", "null",
    ] {
        let schema: DataSchema = serde_json::from_str(&format!(r#"{{"type":"{kind}"}}"#)).unwrap();
        assert!(assert_parity(schema.clone(), true).is_ok());
        for type_name in [None, Some(kind), Some("mismatch")] {
            let mut changed = schema.clone();
            context_mut(&mut changed).data_type = type_name.map(String::from);
            assert_eq!(
                assert_parity(changed, true).err().map(|e| e.rule),
                (type_name == Some("mismatch")).then_some(Rule::TypeMismatch)
            );
        }
        for (read, write) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut changed = schema.clone();
            let context = context_mut(&mut changed);
            context.read_only = read;
            context.write_only = write;
            assert_eq!(
                assert_parity(changed, true).err().map(|e| e.rule),
                (read && write).then_some(Rule::ReadWrite)
            );
        }
        for (lo, hi) in [("minItems", "maxItems"), ("minLength", "maxLength")] {
            for (a, b) in [(1_u64, 2_u64), (2, 2), (3, 2)] {
                let mut changed = schema.clone();
                let fields = &mut context_mut(&mut changed)._extra_fields;
                fields.insert(lo.into(), a.into());
                fields.insert(hi.into(), b.into());
                assert_eq!(assert_parity(changed, true).is_ok(), a <= b);
            }
        }
    }
    for input in [
        r#"{"type":"array","minItems":2,"maxItems":1}"#,
        r#"{"type":"string","minLength":2,"maxLength":1}"#,
        r#"{"type":"number","minimum":2,"maximum":1}"#,
        r#"{"type":"integer","exclusiveMinimum":2,"exclusiveMaximum":1}"#,
        r#"{"type":"integer","minimum":9007199254740993,"maximum":9007199254740992}"#,
        r#"{"type":"number","multipleOf":0}"#,
        r#"{"type":"integer","multipleOf":-1}"#,
    ] {
        assert!(assert_parity(serde_json::from_str(input).unwrap(), true).is_err());
    }
    for (minimum, maximum, multiple_of) in [
        (f64::NAN, 1.0, f64::NAN),
        (f64::INFINITY, f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::INFINITY, 2.5),
    ] {
        assert!(
            assert_parity(
                DataSchema::Number(crate::data_schema::NumberSchema {
                    minimum: Some(minimum),
                    maximum: Some(maximum),
                    multiple_of: Some(multiple_of),
                    ..Default::default()
                }),
                true
            )
            .is_ok()
        );
    }
}

#[test]
fn schema_kernel_preserves_nested_first_error_and_nonsemantic_map_history() {
    let input = r#"{"type":"object","readOnly":true,"writeOnly":true,"oneOf":[{"type":"null"},{"type":"string","minLength":2,"maxLength":1}],"properties":{"z":{"type":"integer","multipleOf":0},"a":{"type":"boolean","readOnly":true,"writeOnly":true}}}"#;
    let mut schema: DataSchema = serde_json::from_str(input).unwrap();
    assert_eq!(
        assert_parity(schema.clone(), true).unwrap_err(),
        InlineInvalid {
            ordinal: 2,
            rule: Rule::Ordered(Field::MinLength, Field::MaxLength)
        }
    );
    context_mut(&mut schema).data_type = Some("wrong".into());
    assert_eq!(
        assert_parity(schema.clone(), true).unwrap_err(),
        InlineInvalid {
            ordinal: 0,
            rule: Rule::TypeMismatch
        }
    );
    context_mut(&mut schema).data_type = None;
    context_mut(&mut schema).one_of = None;
    assert_eq!(
        assert_parity(schema.clone(), true).unwrap_err(),
        InlineInvalid {
            ordinal: 0,
            rule: Rule::ReadWrite
        }
    );
    context_mut(&mut schema).write_only = false;
    let before = assert_parity(schema.clone(), true).unwrap_err();
    assert_eq!(
        before,
        InlineInvalid {
            ordinal: 1,
            rule: Rule::ReadWrite
        }
    );
    let DataSchema::Object(value) = &mut schema else {
        panic!()
    };
    value.properties = Some(value.properties.take().unwrap().into_iter().rev().collect());
    assert_eq!(assert_parity(schema, true).unwrap_err(), before);
    assert!(assert_parity(serde_json::from_str(r#"{"type":"array","items":[{"type":"object","properties":{"x":{"type":"integer","minimum":5,"maximum":1}}}]}"#).unwrap(), true).is_err());
}

#[test]
fn schema_kernel_numeric_rule_preserves_non_numbers_rounding_and_opaque_numbers() {
    for field in [
        "minimum",
        "exclusiveMinimum",
        "maximum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        for nonnumber in ["null", "true", "\"1e309\"", "[]", "{}"] {
            assert!(
                assert_parity(
                    serde_json::from_str(&format!(r#"{{"type":"string","{field}":{nonnumber}}}"#))
                        .unwrap(),
                    true
                )
                .is_ok()
            );
        }
        let overflow: DataSchema =
            serde_json::from_str(&format!(r#"{{"type":"string","{field}":1e309}}"#)).unwrap();
        // The admitted correction now also runs through production Basic.
        assert!(overflow.validate().is_err());
        assert_eq!(
            assert_parity(overflow, true).unwrap_err().rule,
            Rule::FailedProjection(match field {
                "minimum" => Field::Minimum,
                "exclusiveMinimum" => Field::ExclusiveMinimum,
                "maximum" => Field::Maximum,
                "exclusiveMaximum" => Field::ExclusiveMaximum,
                _ => Field::MultipleOf,
            })
        );
    }
    for lower in ["minimum", "exclusiveMinimum"] {
        for upper in ["maximum", "exclusiveMaximum"] {
            assert!(assert_parity(serde_json::from_str(&format!(r#"{{"type":"string","{lower}":9007199254740993,"{upper}":9007199254740992}}"#)).unwrap(), true).is_ok());
        }
    }
    for input in [
        r#"{"type":"string","minimum":1e-4000,"maximum":0}"#,
        r#"{"type":"string","const":1e309,"default":1e309,"enum":[1e309],"opaque":{"minimum":1e309}}"#,
    ] {
        assert!(assert_parity(serde_json::from_str(input).unwrap(), true).is_ok());
    }
}

#[test]
fn schema_kernel_existing_corpus_all_schema_locations_and_serializer_failure() {
    // Reuse each existing schema root, including nested children. Whole Thing
    // validation still includes other components and is not claimed here.
    let things = [
        typed_corpus_shared::nested_schema_corpus(),
        typed_corpus_shared::serializer_failure_thing(),
    ];
    let mut roots = Vec::new();
    for thing in &things {
        roots.extend(thing.schema_definitions.iter().flat_map(|v| v.values()));
        roots.extend(thing.uri_variables.iter().flat_map(|v| v.values()));
        for property in thing.properties.iter().flat_map(|v| v.values()) {
            roots.push(&property._schema);
            roots.extend(
                property
                    ._interaction
                    .uri_variables
                    .iter()
                    .flat_map(|v| v.values()),
            );
        }
        for action in thing.actions.iter().flat_map(|v| v.values()) {
            roots.extend(action.input.iter());
            roots.extend(action.output.iter());
            roots.extend(
                action
                    ._interaction
                    .uri_variables
                    .iter()
                    .flat_map(|v| v.values()),
            );
        }
        for event in thing.events.iter().flat_map(|v| v.values()) {
            roots.extend(event.subscription.iter());
            roots.extend(event.data.iter());
            roots.extend(event.data_response.iter());
            roots.extend(event.cancellation.iter());
            roots.extend(
                event
                    ._interaction
                    .uri_variables
                    .iter()
                    .flat_map(|v| v.values()),
            );
        }
    }
    assert!(roots.len() > 20);
    for root in roots {
        assert!(assert_parity(root.clone(), true).is_ok());
    }
    assert!(serde_json::to_string(&things[1]).is_err());
}

#[path = "../../../tools/architecture-fixtures/bounded-atomic-number/src/lib.rs"]
#[allow(unused_attributes, dead_code)]
mod number_workload;

#[test]
fn schema_kernel_borrowed_ap_projection_matches_the_existing_public_workload() {
    let (_, snapshot, _) = store(serde_json::from_str(r#"{"type":"string"}"#).unwrap());
    let mut count = 0;
    number_workload::for_each_case(|_, text| {
        let number: serde_json::Number = serde_json::from_str(text).unwrap();
        assert_eq!(number.as_str(), text);
        let (values, allocations) = super::semantic_kernel_probe::count_allocations(|| {
            (
                TypedAccess.project_number(&number).map(f64::to_bits),
                SnapshotAccess(&snapshot)
                    .project_number(number.as_str())
                    .map(f64::to_bits),
            )
        });
        assert_eq!(values.0, values.1, "{text}");
        assert_eq!(allocations, 0, "{text}");
        count += 1;
    });
    assert!(count > 10_000);
}

fn numeric_step<'a, A: SchemaAccess<'a>>(
    cursor: &mut NumericCursor,
    access: &A,
    node: A::Node,
    lexeme: impl Fn(A::Number) -> &'a str,
    ceiling: usize,
    allowance: u64,
    lifetime: &mut u64,
) -> (ProjectionProgress<Result<(), Rule>>, u64, usize) {
    let mut budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, allowance);
    let mut projections = 0;
    let inputs = schema_kernel::numeric_inputs(access, node);
    let (result, allocations) = super::semantic_kernel_probe::count_allocations(|| {
        cursor.step(inputs, |number| {
            projection_step::project(
                lexeme(number),
                ceiling,
                &mut budget,
                lifetime,
                || false,
                || {
                    projections += 1;
                    access.project_number(number)
                },
            )
        })
    });
    assert_eq!(allocations, 0);
    (
        result,
        budget.remaining(WorkClass::CodecInputBytes),
        projections,
    )
}

#[test]
fn schema_kernel_numeric_continuation_composes_shared_atomic_budget_and_snapshot() {
    let (thing, snapshot, root) = store(
        serde_json::from_str(r#"{"type":"string","minimum":1.25,"maximum":2.50,"multipleOf":0.5}"#)
            .unwrap(),
    );
    let typed = &thing.schema_definitions.as_ref().unwrap()["probe"];
    let mut a = NumericCursor::default();
    let mut b = NumericCursor::default();
    let mut life_a = 20;
    let mut life_b = 20;
    let initial = a;
    for allowance in [0, 1, 3, 4, 0, 3, 4, 3] {
        let left = numeric_step(
            &mut a,
            &TypedAccess,
            typed,
            |number| number.as_str(),
            64,
            allowance,
            &mut life_a,
        );
        let right = numeric_step(
            &mut b,
            &SnapshotAccess(&snapshot),
            root,
            |text| text,
            64,
            allowance,
            &mut life_b,
        );
        assert_eq!(left, right);
        assert_eq!(a, b);
        assert_eq!(life_a, life_b);
        if allowance == 0 && a == initial {
            assert_eq!(life_a, 20);
        }
    }
    assert_eq!(life_a, 9); // 4 + 4 + 3 once, never retained step credit
    let terminal = a;
    assert_eq!(
        numeric_step(
            &mut a,
            &TypedAccess,
            typed,
            |number| number.as_str(),
            64,
            100,
            &mut life_a
        ),
        (ProjectionProgress::Complete(Ok(())), 100, 0)
    );
    assert_eq!(a, terminal);
    // A separate validation really projects again and pays the same full cost.
    assert_eq!(
        numeric_step(
            &mut NumericCursor::default(),
            &TypedAccess,
            typed,
            |number| number.as_str(),
            64,
            100,
            &mut life_a
        )
        .0,
        ProjectionProgress::Limit
    );
    assert_eq!(life_a, 1); // first two projections charged; third cannot start
    assert!(!core::mem::needs_drop::<NumericCursor>());
    assert!(!core::mem::needs_drop::<InlineInvalid>());
    assert!(core::mem::size_of::<NumericCursor>() <= 128);
    assert!(core::mem::size_of::<InlineInvalid>() <= 16);
}

#[test]
fn schema_kernel_numeric_thresholds_lifetime_and_first_failure_match_both_stores() {
    for ceiling in [0_usize, 64, 256, 257] {
        let lengths = if ceiling == 0 {
            [1, 2, 5]
        } else {
            [ceiling - 1, ceiling, ceiling + 1]
        };
        for length in lengths {
            let text = if length < 4 {
                "1".repeat(length)
            } else {
                format!("1e+{}1", "0".repeat(length - 4))
            };
            let (thing, snapshot, root) = store(
                serde_json::from_str(&format!(r#"{{"type":"string","minimum":{text}}}"#)).unwrap(),
            );
            let typed = &thing.schema_definitions.as_ref().unwrap()["probe"];
            for allowance in [0, length as u64 - 1, length as u64] {
                let mut life_a = length as u64;
                let mut life_b = life_a;
                let mut a = NumericCursor::default();
                let mut b = a;
                let left = numeric_step(
                    &mut a,
                    &TypedAccess,
                    typed,
                    |v| v.as_str(),
                    ceiling,
                    allowance,
                    &mut life_a,
                );
                let right = numeric_step(
                    &mut b,
                    &SnapshotAccess(&snapshot),
                    root,
                    |v| v,
                    ceiling,
                    allowance,
                    &mut life_b,
                );
                assert_eq!((left, a, life_a), (right, b, life_b));
                let expected = if length > ceiling {
                    ProjectionProgress::Limit
                } else if allowance < length as u64 {
                    ProjectionProgress::Pending
                } else {
                    ProjectionProgress::Complete(Ok(()))
                };
                assert_eq!(left.0, expected);
                assert_eq!(
                    left.2,
                    usize::from(length <= ceiling && allowance >= length as u64)
                );
            }
        }
    }
    let (thing, snapshot, root) = store(
        serde_json::from_str(r#"{"type":"string","minimum":2,"maximum":1,"multipleOf":1e309}"#)
            .unwrap(),
    );
    let typed = &thing.schema_definitions.as_ref().unwrap()["probe"];
    for lifetime in [1, 2, 10] {
        let mut a = lifetime;
        let mut b = lifetime;
        let left = numeric_step(
            &mut NumericCursor::default(),
            &TypedAccess,
            typed,
            |v| v.as_str(),
            64,
            20,
            &mut a,
        );
        let right = numeric_step(
            &mut NumericCursor::default(),
            &SnapshotAccess(&snapshot),
            root,
            |v| v,
            64,
            20,
            &mut b,
        );
        assert_eq!((left, a), (right, b));
        assert_eq!(
            left.0,
            if lifetime < 2 {
                ProjectionProgress::Limit
            } else {
                ProjectionProgress::Complete(Err(Rule::Ordered(Field::Minimum, Field::Maximum)))
            }
        );
        assert_eq!(left.2, lifetime.min(2) as usize); // multipleOf never observed
    }
}

#[test]
fn schema_kernel_cancellation_brackets_projection_without_allocating() {
    for text in [
        "1e309",
        "1.00000000000000011102230246251565404236316680908203125",
    ] {
        let (thing, snapshot, root) = store(
            serde_json::from_str(&format!(r#"{{"type":"string","minimum":{text}}}"#)).unwrap(),
        );
        let typed = &thing.schema_definitions.as_ref().unwrap()["probe"];
        let number = TypedAccess.number_extension(typed, Field::Minimum).unwrap();
        let stored = SnapshotAccess(&snapshot)
            .number_extension(root, Field::Minimum)
            .unwrap();
        for cancel_on in [1, 2, 3] {
            let mut checkpoints = 0;
            let mut projections = 0;
            let mut lifetime = 256;
            let mut budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 256);
            let mut cursor = NumericCursor::default();
            let inputs = schema_kernel::numeric_inputs(&TypedAccess, typed);
            let (left, allocations) = super::semantic_kernel_probe::count_allocations(|| {
                cursor.step(inputs, |_| {
                    projection_step::project(
                        number.as_str(),
                        256,
                        &mut budget,
                        &mut lifetime,
                        || {
                            checkpoints += 1;
                            checkpoints == cancel_on
                        },
                        || {
                            projections += 1;
                            number.as_f64()
                        },
                    )
                })
            });
            assert_eq!(allocations, 0);
            let typed_trace = (
                left,
                checkpoints,
                projections,
                lifetime,
                budget.remaining(WorkClass::CodecInputBytes),
            );
            checkpoints = 0;
            projections = 0;
            lifetime = 256;
            budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 256);
            let inputs = schema_kernel::numeric_inputs(&SnapshotAccess(&snapshot), root);
            let (right, allocations) = super::semantic_kernel_probe::count_allocations(|| {
                NumericCursor::default().step(inputs, |_| {
                    projection_step::project(
                        stored,
                        256,
                        &mut budget,
                        &mut lifetime,
                        || {
                            checkpoints += 1;
                            checkpoints == cancel_on
                        },
                        || {
                            projections += 1;
                            SnapshotAccess(&snapshot).project_number(stored)
                        },
                    )
                })
            });
            assert_eq!(allocations, 0);
            assert_eq!(
                typed_trace,
                (
                    right,
                    checkpoints,
                    projections,
                    lifetime,
                    budget.remaining(WorkClass::CodecInputBytes)
                )
            );
            if cancel_on <= 2 {
                assert_eq!(right, ProjectionProgress::Cancelled);
            }
            assert_eq!(projections, usize::from(cancel_on != 1));
            assert_eq!(
                lifetime,
                256 - if cancel_on == 1 {
                    0
                } else {
                    number.as_str().len() as u64
                }
            );
        }
    }
}
