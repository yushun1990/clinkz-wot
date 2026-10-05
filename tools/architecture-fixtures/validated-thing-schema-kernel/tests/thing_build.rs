#![cfg(feature = "validated-thing")]
#![allow(clippy::result_large_err)]
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use td_crate::{
    basic_kernel as basic,
    basic_typed::TypedBasicAccess,
    schema_build::{Cause, ConstructionCause, ConstructionTrace, Cursor, Pass, Progress, Stage},
    thing::Thing,
    thing_build as build,
    validate::{Validate, ValidationLevel},
};
use validated_thing_schema_kernel_probe as td_crate;
use validated_thing_value_construction_probe::{Cause as Resource, Limits, OwnedValue};
#[path = "support/allocator.rs"]
mod allocation;
#[path = "../../../../td/tests/support/basic_corpus_shared.rs"]
#[allow(dead_code)]
mod basic_corpus;
#[path = "support/canonical_fields.rs"]
mod canonical_fields;
#[path = "../../../../td/tests/support/typed_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;
#[path = "support/thing_fields.rs"]
mod thing_fields;

const CLASSES: [W; 6] = [
    W::DocumentNodes,
    W::CodecInputBytes,
    W::CodecOutputBytes,
    W::JsonSchemaNodes,
    W::CleanupItems,
    W::UriBytes,
];
fn budget(n: u64) -> WorkBudget {
    CLASSES
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, n))
}
fn total(trace: ConstructionTrace) -> u64 {
    trace.canonical.work.into_iter().sum::<u64>() + trace.seal.into_iter().sum::<u64>()
}
fn schedule(n: u64) -> impl FnMut(Stage) -> (WorkBudget, bool) {
    let mut polls = 0;
    move |_| {
        polls += 1;
        assert!(polls < 1_000_000);
        (budget(if polls % 17 == 0 { n.max(2) } else { n }), false)
    }
}

#[test]
fn typed_envelope_fields_equivalence_basic_and_input_drop_compose() {
    for thing in [
        corpus::typed_corpus(),
        corpus::nested_schema_corpus(),
        corpus::serializer_failure_thing(),
    ] {
        let expected = basic::validate(&TypedBasicAccess(Some(&thing)), &basic::InlineSink);
        let mut baseline = None;
        for n in [1, 7, 4096] {
            let (normalized, trace) =
                build::from_thing(&thing, Limits::default(), schedule(n)).unwrap();
            thing_fields::assert_thing(normalized.view(), &thing);
            assert_eq!(
                basic::validate(&build::Basic::new(&normalized), &basic::InlineSink),
                expected
            );
            assert_eq!(
                normalized.lifetime_remaining(),
                Limits::default().lifetime - total(trace)
            );
            let a = normalized.arenas_for_fixture();
            assert_eq!(trace.canonical.compared_nodes, a.nodes().len() as u64);
            assert_eq!(trace.canonical.compared_edges, a.edges().len() as u64);
            assert_eq!(trace.canonical.compared_bytes, a.bytes().len() as u64);
            assert_eq!(trace.canonical.schemas[0], trace.canonical.schemas[1]);
            assert!(trace.canonical.schemas[0] > 10);
            assert_eq!(
                *baseline.get_or_insert((trace, normalized.footprint())),
                (trace, normalized.footprint())
            );
        }
    }
    let thing = corpus::serializer_failure_thing();
    assert!(thing.validate_with_level(ValidationLevel::Basic).is_ok());
    assert!(serde_json::to_vec(&thing).is_err());
    let normalized = build::from_thing(&thing, Limits::default(), schedule(7))
        .unwrap()
        .0;
    drop(thing);
    fn require_static<T: 'static>(_: &T) {}
    require_static(&normalized);
    assert!(basic::validate(&build::Basic::new(&normalized), &basic::InlineSink).is_ok());
    assert_eq!(
        normalized
            .view()
            .root_field("context")
            .unwrap()
            .child(0)
            .unwrap()
            .text(),
        Some("https://example.org/extension-only")
    );
}

#[test]
fn existing_whole_basic_first_cause_corpus_runs_on_constructed_owner() {
    let cases = basic_corpus::cases();
    assert!(cases.len() > 170);
    for case in cases {
        let expected = basic::validate(&TypedBasicAccess(Some(&case.thing)), &basic::InlineSink);
        let normalized = build::from_thing(&case.thing, Limits::default(), schedule(31))
            .unwrap()
            .0;
        assert_eq!(
            basic::validate(&build::Basic::new(&normalized), &basic::InlineSink),
            expected,
            "{}",
            case.label
        );
        assert_eq!(expected.is_ok(), case.valid, "{}", case.label);
    }
}

#[test]
fn optional_envelope_fields_distinguish_absent_and_explicit_empty() {
    let mut sparse = corpus::typed_corpus();
    sparse.id = None;
    sparse._metadata = Default::default();
    sparse.version = None;
    sparse.created = None;
    sparse.modified = None;
    sparse.support = None;
    sparse.base = None;
    sparse.properties = None;
    sparse.actions = None;
    sparse.events = None;
    sparse.links = None;
    sparse.forms = None;
    sparse.profile = None;
    sparse.schema_definitions = None;
    sparse.uri_variables = None;
    sparse.security.clear();
    sparse.security_definitions.clear();
    sparse._extra_fields.clear();
    let absent = build::from_thing(&sparse, Limits::default(), schedule(7))
        .unwrap()
        .0;
    thing_fields::assert_thing(absent.view(), &sparse);
    sparse._metadata.tags = Some(vec![]);
    sparse._metadata.titles = Some(Default::default());
    sparse._metadata.descriptions = Some(Default::default());
    sparse.properties = Some(Default::default());
    sparse.actions = Some(Default::default());
    sparse.events = Some(Default::default());
    sparse.links = Some(vec![]);
    sparse.forms = Some(vec![]);
    sparse.profile = Some(vec![]);
    sparse.schema_definitions = Some(Default::default());
    sparse.uri_variables = Some(Default::default());
    let empty = build::from_thing(&sparse, Limits::default(), schedule(7))
        .unwrap()
        .0;
    thing_fields::assert_thing(empty.view(), &sparse);
    assert!(absent.view().root_field("properties").is_none());
    assert!(empty.view().root_field("properties").unwrap().is_empty());
    assert!(
        empty.footprint().retained_requested_bytes > absent.footprint().retained_requested_bytes
    );

    let mut thing = corpus::typed_corpus();
    let form = &mut thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[0];
    form.content_type.clear();
    form.content_coding = None;
    form.security = None;
    form.scopes = None;
    form.response = None;
    form.additional_responses = None;
    form.subprotocol = None;
    form.op = None;
    let absent = build::from_thing(&thing, Limits::default(), schedule(7))
        .unwrap()
        .0;
    thing_fields::assert_thing(absent.view(), &thing);
    let form = &mut thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[0];
    form.security = Some(vec![]);
    form.scopes = Some(vec![]);
    form.additional_responses = Some(vec![]);
    form.op = Some(vec![]);
    let empty = build::from_thing(&thing, Limits::default(), schedule(7))
        .unwrap()
        .0;
    thing_fields::assert_thing(empty.view(), &thing);
    assert_eq!(
        basic::validate(&build::Basic::new(&empty), &basic::InlineSink),
        basic::validate(&TypedBasicAccess(Some(&thing)), &basic::InlineSink)
    );
}

#[test]
fn all_actual_allocations_failure_peaks_and_fixed_owner_release_are_observed() {
    let thing = corpus::nested_schema_corpus();
    let ((normalized, trace), observed) = allocation::observe(0, || {
        build::from_thing(&thing, Limits::default(), schedule(31)).unwrap()
    });
    let f = normalized.footprint();
    assert_eq!(observed.live as u64, f.retained_requested_bytes);
    assert_eq!(observed.peak as u64, f.conversion_peak_bytes);
    assert_eq!(observed.largest as u64, f.largest_request_bytes);
    assert_eq!(observed.reallocations, 0);
    let (_, released) = allocation::observe(0, || drop(normalized));
    assert_eq!(released.allocations, 0);
    assert_eq!(released.releases, 3);
    assert_eq!(-released.live as u64, f.retained_requested_bytes);
    for fail_at in 1..=observed.attempts {
        let (failed, observed) = allocation::observe(fail_at, || {
            build::from_thing(&thing, Limits::default(), schedule(31))
        });
        let failed = failed.err().unwrap();
        assert!(matches!(
            failed.cause,
            ConstructionCause::Canonical(Cause::Resource(Resource::Allocation))
                | ConstructionCause::Seal(Resource::Allocation)
        ));
        assert_eq!(observed.live, 0);
        assert_eq!(observed.reallocations, 0);
        assert_eq!(failed.live_after_rollback, 0);
        assert_eq!(failed.allocations, failed.releases);
        assert_eq!(observed.allocations, observed.releases);
        assert_eq!(failed.resources.conversion_peak_bytes, observed.peak as u64);
    }
    println!(
        "typed Thing: retained={}, peak={}, temporary={}, largest={}, lifetime={}, allocations={}, cursor_inline={}",
        f.retained_requested_bytes,
        f.conversion_peak_bytes,
        f.temporary_peak_bytes,
        f.largest_request_bytes,
        total(trace),
        observed.attempts,
        core::mem::size_of::<Cursor<'_>>()
    );
}

#[test]
fn typed_number_ceiling_precedes_copy_and_preserves_opaque_overflow() {
    for length in [0, 63, 64, 65, 255, 256, 257] {
        let mut thing = corpus::typed_corpus();
        // Tests construct the public AP Number before observing admission.
        let n = if length == 0 {
            "1".to_owned()
        } else {
            format!("1{}", "0".repeat(length - 1))
        };
        thing
            ._extra_fields
            .insert("number".into(), serde_json::from_str(&n).unwrap());
        let limits = Limits {
            number: if length < 100 { 64 } else { 256 },
            ..Limits::default()
        };
        let result = build::from_thing(&thing, limits, schedule(7));
        if n.len() <= limits.number {
            let owner = result.unwrap().0;
            assert_eq!(
                build::map_get(owner.view().root_field("extras").unwrap(), "number")
                    .unwrap()
                    .text(),
                Some(n.as_str())
            );
        } else {
            let failed = result.err().unwrap();
            assert_eq!(
                failed.cause,
                ConstructionCause::Canonical(Cause::Resource(Resource::DecodedNumber))
            );
            assert_eq!(failed.live_after_rollback, 0);
        }
    }
    let mut thing = Thing::default();
    thing
        ._extra_fields
        .insert("n".into(), serde_json::from_str("1e309").unwrap());
    let failed = build::from_thing(
        &thing,
        Limits {
            number: 0,
            ..Limits::default()
        },
        schedule(1),
    )
    .err()
    .unwrap();
    assert_eq!(
        failed.cause,
        ConstructionCause::Canonical(Cause::Resource(Resource::DecodedNumber))
    );
}

#[test]
fn combined_lifetime_partial_credit_and_cancellation_cover_the_whole_envelope() {
    let thing = corpus::typed_corpus();
    let (owner, trace) = build::from_thing(&thing, Limits::default(), schedule(31)).unwrap();
    drop(owner);
    let exact = Limits {
        lifetime: total(trace),
        ..Limits::default()
    };
    for withheld in CLASSES {
        let mut polls = 0;
        let (owner, actual) = build::from_thing(&thing, exact, |_| {
            polls += 1;
            assert!(polls < 1_000_000);
            (
                if polls % 3 == 0 {
                    budget(31)
                } else {
                    budget(31).with_remaining(withheld, 0)
                },
                false,
            )
        })
        .unwrap();
        assert_eq!(owner.lifetime_remaining(), 0);
        assert_eq!(actual, trace);
    }
    let failed = build::from_thing(
        &thing,
        Limits {
            lifetime: total(trace) - 1,
            ..exact
        },
        schedule(31),
    )
    .err()
    .unwrap();
    assert_eq!(failed.cause, ConstructionCause::Seal(Resource::Lifetime));
    assert_eq!(failed.live_after_rollback, 0);
    for target in [
        Stage::Canonical(Pass::Construction),
        Stage::Canonical(Pass::Equivalence),
        Stage::Seal,
    ] {
        let failed = build::from_thing(&thing, Limits::default(), |stage| {
            (budget(7), stage == target)
        })
        .err()
        .unwrap();
        assert_eq!(failed.stage, target);
        assert_eq!(failed.live_after_rollback, 0);
        assert_eq!(failed.allocations, failed.releases);
    }
}

#[test]
fn typed_equivalence_fault_zero_work_and_pending_abandonment_are_falsifiable() {
    let thing = corpus::typed_corpus();
    let mut owner = OwnedValue::empty_for_fixture(Limits::default());
    let mut cursor = Cursor::from_thing(&mut owner, &thing, Limits::default());
    let before = cursor.trace();
    let lifetime = cursor.lifetime_remaining();
    let Progress::Pending(next) = cursor.step(&mut WorkBudget::new(), || false) else {
        panic!()
    };
    cursor = next;
    assert_eq!(cursor.trace(), before);
    assert_eq!(cursor.lifetime_remaining(), lifetime);
    loop {
        if cursor.pass() == Pass::Equivalence {
            cursor.corrupt_root_for_test();
            break;
        }
        let Progress::Pending(next) = cursor.step(&mut budget(2), || false) else {
            panic!()
        };
        cursor = next;
    }
    assert!(
        matches!(cursor.step(&mut budget(4096),|| false),Progress::Failed(f) if f.cause == Cause::SemanticMismatch)
    );
    owner.rollback_for_fixture();
    assert_eq!(owner.allocations(), owner.releases());
    // Stop at every mixed-schedule construction/equivalence Pending and prove
    // that drop pays no semantic work, adds no allocation, and frees all sites.
    for stop in 0..1000 {
        let ((complete, allocations, releases), observed) = allocation::observe(0, || {
            let mut owner = OwnedValue::empty_for_fixture(Limits::default());
            let mut complete = false;
            {
                let mut cursor = Cursor::from_thing(&mut owner, &thing, Limits::default());
                for poll in 0..=stop {
                    match cursor.step(&mut budget(127), || false) {
                        Progress::Pending(next) if poll == stop => {
                            drop(next);
                            break;
                        }
                        Progress::Pending(next) => cursor = next,
                        Progress::Complete(_) => {
                            complete = true;
                            break;
                        }
                        Progress::Failed(f) => panic!("{f:?}"),
                    }
                }
            }
            owner.rollback_for_fixture();
            (complete, owner.allocations(), owner.releases())
        });
        assert_eq!(allocations, releases);
        assert_eq!(observed.live, 0);
        if complete {
            return;
        }
    }
    panic!("complete boundary not reached");
}

#[test]
fn whole_owner_memory_and_structure_boundaries_are_not_separate_schema_allowances() {
    let thing = corpus::nested_schema_corpus();
    let (owner, _) = build::from_thing(&thing, Limits::default(), schedule(31)).unwrap();
    let f = owner.footprint();
    let a = owner.arenas_for_fixture();
    let sizes = [a.nodes().len(), a.edges().len(), a.bytes().len()];
    drop(owner);
    for axis in 0..7 {
        let limit = match axis {
            0 => f.retained_requested_bytes,
            1 => f.temporary_peak_bytes,
            2 => f.conversion_peak_bytes,
            3 => f.largest_request_bytes,
            _ => sizes[axis - 4] as u64,
        };
        for delta in [-1i64, 0, 1] {
            let value = (limit as i64 + delta) as u64;
            let mut limits = Limits::default();
            match axis {
                0 => limits.source = value,
                1 => limits.temporary = value,
                2 => limits.peak = value,
                3 => limits.contiguous = value,
                4 => limits.nodes = value as usize,
                5 => limits.edges = value as usize,
                6 => limits.bytes = value as usize,
                _ => unreachable!(),
            }
            let result = build::from_thing(&thing, limits, schedule(31));
            if delta < 0 {
                let f = result.err().unwrap();
                assert_eq!(f.live_after_rollback, 0);
                assert_eq!(f.allocations, f.releases);
            } else {
                assert!(result.is_ok(), "axis {axis}, delta {delta}");
            }
        }
    }
    let failed = build::from_thing(
        &thing,
        Limits {
            frames: 1,
            ..Limits::default()
        },
        schedule(31),
    )
    .err()
    .unwrap();
    assert_eq!(
        failed.cause,
        ConstructionCause::Canonical(Cause::Resource(Resource::Frames))
    );
    assert_eq!(failed.live_after_rollback, 0);
}

#[test]
fn maps_caller_capacity_and_deep_opaque_values_share_the_typed_envelope() {
    let mut a = corpus::typed_corpus();
    let mut b = a.clone();
    a._metadata.title.as_mut().unwrap().reserve(4096);
    a.security.reserve(64);
    a.properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms
        .reserve(64);
    let entries = [
        ("z", serde_json::json!([1, 2])),
        ("a", serde_json::json!({"k":true})),
        ("same-prefix-a", serde_json::json!(1)),
        ("same-prefix-b", serde_json::json!(2)),
    ];
    for (thing, reverse) in [(&mut a, false), (&mut b, true)] {
        let mut map = serde_json::Map::new();
        for i in 0..entries.len() {
            let (k, v) = &entries[if reverse { entries.len() - i - 1 } else { i }];
            map.insert((*k).into(), v.clone());
        }
        thing
            ._extra_fields
            .insert("ordered".into(), serde_json::Value::Object(map));
        let mut deep = serde_json::json!("leaf");
        for _ in 0..256 {
            deep = serde_json::Value::Array(vec![deep]);
        }
        thing._extra_fields.insert("deep".into(), deep);
    }
    let (aa, at) = build::from_thing(&a, Limits::default(), schedule(31)).unwrap();
    let (bb, bt) = build::from_thing(&b, Limits::default(), schedule(31)).unwrap();
    assert_eq!(aa.footprint(), bb.footprint());
    // Selection pays actual comparator work; insertion order may change it.
    assert!(total(at) > 0 && total(bt) > 0);
    assert_eq!(
        aa.arenas_for_fixture().nodes(),
        bb.arenas_for_fixture().nodes()
    );
    assert_eq!(
        aa.arenas_for_fixture().edges(),
        bb.arenas_for_fixture().edges()
    );
    assert_eq!(
        aa.arenas_for_fixture().bytes(),
        bb.arenas_for_fixture().bytes()
    );
    let mut v = build::map_get(aa.view().root_field("extras").unwrap(), "deep").unwrap();
    for _ in 0..256 {
        assert_eq!(v.len(), 1);
        v = v.child(0).unwrap();
    }
    assert_eq!(v.text(), Some("leaf"));
    let failure = build::from_thing(
        &a,
        Limits {
            frames: 128,
            ..Limits::default()
        },
        schedule(31),
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.cause,
        ConstructionCause::Canonical(Cause::Resource(Resource::Frames))
    );
    assert_eq!(failure.live_after_rollback, 0);
}

#[test]
fn external_query_and_owned_selection_use_the_constructed_owner_after_input_drop() {
    use td_crate::{basic_kernel::BasicAccess, data_type::Operation, schema_build::View};
    fn query(root: View<'_>) -> (&str, u32, &str, &str) {
        let properties = root.root_field("properties").unwrap();
        let (name, property) = properties.member(1).unwrap();
        let forms = property.child(1).unwrap();
        let form = forms.child(1).unwrap();
        assert_eq!(
            root.effective_property_operations(property),
            [Operation::ReadProperty, Operation::WriteProperty]
        );
        assert!(form.form_field("op").is_none());
        assert_eq!(
            form.form_field("contentCoding").unwrap().text(),
            Some("identity")
        );
        assert_eq!(
            form.form_field("subprotocol").unwrap().text(),
            Some("longpoll")
        );
        assert_eq!(
            form.form_field("scopes").unwrap().child(0).unwrap().text(),
            Some("read")
        );
        (
            name,
            forms.original_index(1).unwrap(),
            form.form_field("href").unwrap().text().unwrap(),
            form.form_field("contentType").unwrap().text().unwrap(),
        )
    }
    let normalized = {
        let mut thing = corpus::typed_corpus();
        let form = &mut thing
            .properties
            .as_mut()
            .unwrap()
            .get_mut("zeta")
            .unwrap()
            ._interaction
            .forms[1];
        form.op = None;
        form.security = None;
        form.content_coding = Some("identity".into());
        form.subprotocol = Some("longpoll".into());
        form.scopes = Some(vec!["read".into()]);
        build::from_thing(&thing, Limits::default(), schedule(7))
            .unwrap()
            .0
    };
    let (selection, observed) = allocation::observe(0, || {
        let result = query(normalized.view());
        let access = build::Basic::new(&normalized);
        let form = normalized
            .view()
            .root_field("properties")
            .unwrap()
            .member(1)
            .unwrap()
            .1
            .child(1)
            .unwrap()
            .child(1)
            .unwrap();
        let names = normalized.view().effective_security(form);
        assert_eq!(access.names_count(names), 1);
        assert_eq!(access.name_at(names, 0), "none");
        assert_eq!(
            access.scheme(
                build::map_get(
                    normalized.view().root_field("securityDefinitions").unwrap(),
                    "none"
                )
                .unwrap()
            ),
            "nosec"
        );
        result
    });
    assert_eq!(observed.allocations, 0);
    assert_eq!(selection.0, "zeta");
    assert_eq!(selection.1, 1);
    let owned = (
        selection.0.to_owned(),
        selection.1,
        selection.2.to_owned(),
        selection.3.to_owned(),
    );
    drop(normalized);
    assert_eq!(
        owned,
        (
            "zeta".into(),
            1,
            "zeta/second".into(),
            "application/json".into()
        )
    );
}
