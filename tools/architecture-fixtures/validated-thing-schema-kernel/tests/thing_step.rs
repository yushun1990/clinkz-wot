#![cfg(feature = "validated-thing")]
#![allow(clippy::result_large_err)]
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use td_crate::{
    basic_kernel as basic,
    basic_typed::TypedBasicAccess,
    schema_build::{ConstructionCause, ConstructionTrace, Stage},
    thing_build as build,
    thing_step::{Cause, Cursor, Failure, Progress, Trace},
};
use validated_thing_schema_kernel_probe as td_crate;
use validated_thing_value_construction_probe::Limits;
#[path = "support/allocator.rs"]
mod allocation;
#[path = "../../../../td/tests/support/basic_corpus_shared.rs"]
#[allow(dead_code)]
mod cases;
#[path = "../../../../td/tests/support/typed_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;

const CLASSES: [W; 5] = [
    W::DocumentNodes,
    W::CodecInputBytes,
    W::JsonSchemaNodes,
    W::SecurityBranches,
    W::CleanupItems,
];
fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn total(trace: ConstructionTrace) -> u64 {
    trace.canonical.work.into_iter().sum::<u64>()
        + trace.seal.into_iter().sum::<u64>()
        + trace.thing_basic.work.into_iter().sum::<u64>()
}
fn schedule(n: u64) -> impl FnMut(Stage) -> (WorkBudget, bool) {
    let mut poll = 0;
    move |_| {
        poll += 1;
        assert!(poll < 1000000);
        (budget(if poll % 23 == 0 { n.max(512) } else { n }), false)
    }
}
fn trace(progress: &Progress<'_>) -> Trace {
    match progress {
        Progress::Pending(c) => c.trace(),
        Progress::Complete(t) => *t,
        Progress::Failed(f) => f.trace,
    }
}
fn drive(mut cursor: Cursor<'_>, n: u64) -> Result<Trace, Failure> {
    let initial = cursor.lifetime_remaining();
    for poll in 0..1000000 {
        let before = cursor.trace();
        let phase = cursor.phase();
        let lifetime = cursor.lifetime_remaining();
        let Progress::Pending(next) = cursor.step(&mut WorkBudget::new(), || false) else {
            panic!("zero budget at {phase:?}")
        };
        assert_eq!(next.trace(), before);
        assert_eq!(next.phase(), phase);
        assert_eq!(next.lifetime_remaining(), lifetime);
        let amount = if poll % 23 == 0 { n.max(512) } else { n };
        let mut work = budget(amount);
        let progress = next.step(&mut work, || false);
        let after = trace(&progress);
        for (i, class) in CLASSES.into_iter().enumerate() {
            assert_eq!(
                after.work[i] - before.work[i],
                amount - work.remaining(class)
            );
        }
        match progress {
            Progress::Pending(next) => {
                assert_eq!(
                    next.lifetime_remaining(),
                    initial - after.work.into_iter().sum::<u64>()
                );
                cursor = next;
            }
            Progress::Complete(trace) => return Ok(trace),
            Progress::Failed(f) => return Err(f),
        }
    }
    panic!("Basic did not finish");
}

fn projection_thing(number: &str) -> td_crate::thing::Thing {
    let mut thing: td_crate::thing::Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"projection","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}}}"#).unwrap();
    // Direct typed facts pin the actual public Number content. Full TD field
    // deserialization can change spelling through repeated Value conversion.
    let mut schema = td_crate::data_schema::NullSchema::default();
    schema
        ._context
        ._extra_fields
        .insert("minimum".into(), serde_json::from_str(number).unwrap());
    thing.schema_definitions = Some(std::collections::BTreeMap::from([(
        "number".into(),
        td_crate::data_schema::DataSchema::Null(schema),
    )]));
    thing
}

fn projection_pause(mut cursor: Cursor<'_>) -> Cursor<'_> {
    for _ in 0..10000 {
        let before = cursor.trace().work;
        let Progress::Pending(next) = cursor.step(&mut budget(1), || false) else {
            panic!("projection not reached")
        };
        if next.phase() == td_crate::thing_step::Phase::Numeric && next.trace().work == before {
            return next;
        }
        cursor = next;
    }
    panic!("projection not reached");
}

#[test]
fn numeric_atomic_work_and_terminal_causes_compose_on_the_result_owner() {
    for ceiling in [64, 256] {
        for length in [ceiling - 1, ceiling, ceiling + 1] {
            let text = format!("0.{}1", "0".repeat(length - 3));
            let input = projection_thing(&text);
            let mut owner = build::from_thing(
                &input,
                Limits {
                    number: 512,
                    ..Limits::default()
                },
                schedule(17),
            )
            .unwrap()
            .0;
            let (result, observed) = allocation::observe(0, || {
                drive(
                    Cursor::new(
                        &mut owner,
                        Limits {
                            number: ceiling,
                            ..Limits::default()
                        },
                    ),
                    17,
                )
            });
            assert_eq!(observed.attempts, 1); // the single schema frame
            assert_eq!(observed.allocations, observed.releases);
            assert_eq!(observed.live, 0);
            assert_eq!(observed.reallocations, 0);
            if length > ceiling {
                let f = result.unwrap_err();
                assert_eq!(
                    f.cause,
                    Cause::Number {
                        observed: length,
                        ceiling
                    }
                );
                assert_eq!(f.trace.projections, [0; 2]);
            } else {
                assert_eq!(result.unwrap().projections, [0, 1]);
                let (owner, trace) = build::from_thing_basic(
                    &input,
                    Limits {
                        number: ceiling,
                        ..Limits::default()
                    },
                    schedule(1),
                )
                .unwrap();
                assert_eq!(trace.thing_basic.projections, [0, 1]);
                assert_eq!(
                    owner.lifetime_remaining(),
                    Limits::default().lifetime - total(trace)
                );
            }
        }
    }
    let input = projection_thing("1.00");
    for cancel_at in [2, 3] {
        // after the outer check: immediately before / after parse
        let mut owner = build::from_thing(&input, Limits::default(), schedule(17))
            .unwrap()
            .0;
        let mut cursor = projection_pause(Cursor::new(&mut owner, Limits::default()));
        let before = cursor.trace();
        let lifetime = cursor.lifetime_remaining();
        for _ in 0..3 {
            let mut work = budget(4096).with_remaining(W::CodecInputBytes, 3);
            let Progress::Pending(next) = cursor.step(&mut work, || false) else {
                panic!("atomic shortage")
            };
            assert_eq!(next.trace(), before);
            assert_eq!(next.lifetime_remaining(), lifetime);
            for class in W::ALL {
                assert_eq!(
                    work.remaining(class),
                    if class == W::CodecInputBytes { 3 } else { 4096 }
                );
            }
            cursor = next;
        }
        let mut checks = 0;
        let Progress::Failed(f) = cursor.step(&mut budget(4096), || {
            checks += 1;
            checks == cancel_at
        }) else {
            panic!("projection cancellation")
        };
        assert_eq!(f.cause, Cause::Cancelled);
        let parsed = u64::from(cancel_at == 3);
        assert_eq!(f.trace.projections, [0, parsed]);
        assert_eq!(f.trace.work[1] - before.work[1], 4 * parsed);
        assert_eq!(f.trace.resources.unwrap().temporary_live, 0);
        assert_eq!(lifetime - owner.lifetime_remaining(), 4 * parsed);
    }
    let mut owner = build::from_thing(&input, Limits::default(), schedule(17))
        .unwrap()
        .0;
    for _ in 0..2 {
        assert_eq!(
            drive(Cursor::new(&mut owner, Limits::default()), 1)
                .unwrap()
                .projections,
            [0, 1]
        );
    }
    let mut owner = build::from_thing(&input, Limits::default(), schedule(17))
        .unwrap()
        .0;
    let cursor = projection_pause(Cursor::new(&mut owner, Limits::default()));
    let prefix = Limits::default().lifetime - cursor.lifetime_remaining();
    drop(cursor);
    let f = build::from_thing_basic(
        &input,
        Limits {
            lifetime: prefix + 3,
            ..Limits::default()
        },
        schedule(17),
    )
    .err()
    .unwrap();
    assert_eq!(f.cause, ConstructionCause::ThingBasic(Cause::Lifetime));
    assert_eq!(
        f.stage,
        Stage::ThingBasic(td_crate::thing_step::Phase::Numeric)
    );
    assert_eq!(f.trace.thing_basic.projections, [0; 2]);
    assert_eq!(total(f.trace), prefix);
    assert_eq!(f.lifetime_remaining, 3);
    assert_eq!(f.live_after_rollback, 0);
    assert_eq!(f.allocations, f.releases);
    let mut owner = build::from_thing(&input, Limits::default(), schedule(17))
        .unwrap()
        .0;
    let f = drive(
        Cursor::new(
            &mut owner,
            Limits {
                number: 0,
                ..Limits::default()
            },
        ),
        17,
    )
    .unwrap_err();
    assert_eq!(
        f.cause,
        Cause::Number {
            observed: 4,
            ceiling: 0
        }
    );
    assert_eq!(f.trace.projections, [0; 2]);
    let f = build::from_thing_basic(&projection_thing("1e309"), Limits::default(), schedule(17))
        .err()
        .unwrap();
    assert!(matches!(
        f.cause,
        ConstructionCause::ThingBasic(Cause::Invalid(basic::InlineInvalid {
            rule: basic::InlineRule::Schema(td_crate::schema_kernel::Rule::FailedProjection(_)),
            ..
        }))
    ));
    assert_eq!(f.trace.thing_basic.projections, [0, 1]);
    assert_eq!(f.live_after_rollback, 0);
    assert_eq!(f.allocations, f.releases);
    let (_, trace) =
        build::from_thing_basic(&projection_thing("true"), Limits::default(), schedule(17))
            .unwrap();
    assert_eq!(trace.thing_basic.projections, [0; 2]);
}

#[test]
fn long_security_lookup_charges_each_actual_operand_without_new_storage() {
    for length in [1, 128, 512] {
        let mut input = projection_thing("true");
        input.schema_definitions = None;
        let name = "x".repeat(length);
        let definition = input.security_definitions.remove("none").unwrap();
        input.security_definitions.insert(name.clone(), definition);
        input.security = vec![name];
        let mut owner = build::from_thing(&input, Limits::default(), schedule(17))
            .unwrap()
            .0;
        let (result, physical) =
            allocation::observe(0, || drive(Cursor::new(&mut owner, Limits::default()), 1));
        let trace = result.unwrap();
        assert_eq!(trace.key_bytes, 2 * length as u64);
        assert_eq!(trace.work[1], trace.key_bytes + 5); // nosec discriminator
        assert_eq!(trace.frame_requests, 0);
        assert_eq!(physical.attempts, 0);
    }
}

#[test]
fn known_combo_shape_failure_precedes_unrelated_group_work_and_lifetime() {
    use td_crate::security_scheme::SecurityScheme;
    let mut baseline = None;
    for later in [0, 1, 4096] {
        let mut input = projection_thing("true");
        let mut definition = SecurityScheme::combo_one_of(["none"]);
        let SecurityScheme::Combo(ref mut combo) = definition else {
            unreachable!()
        };
        combo.all_of = vec!["none".into(); later];
        input.security_definitions.insert("z".into(), definition);
        let f = build::from_thing_basic(&input, Limits::default(), schedule(1))
            .err()
            .unwrap();
        assert!(matches!(
            f.cause,
            ConstructionCause::ThingBasic(Cause::Invalid(basic::InlineInvalid {
                site: basic::Site {
                    field: basic::Field::OneOf,
                    ..
                },
                rule: basic::InlineRule::Basic(_),
                ..
            }))
        ));
        assert_eq!(
            *baseline.get_or_insert(f.trace.thing_basic.work),
            f.trace.thing_basic.work
        );
        // Give precisely enough total lifetime for construction plus the known
        // oneOf failure; a later group cannot replace it with Lifetime.
        let f2 = build::from_thing_basic(
            &input,
            Limits {
                lifetime: total(f.trace),
                ..Limits::default()
            },
            schedule(17),
        )
        .err()
        .unwrap();
        assert_eq!(f2.cause, f.cause);
        assert_eq!(f2.lifetime_remaining, 0);
        assert_eq!(f2.live_after_rollback, 0);
        assert_eq!(f2.allocations, f2.releases);
    }
}

#[test]
fn full_first_error_corpus_uses_paid_canonical_basic_in_the_owning_transaction() {
    let cases = cases::cases();
    assert!(cases.len() > 170);
    for case in cases {
        let expected = basic::validate(&TypedBasicAccess(Some(&case.thing)), &basic::InlineSink);
        let mut baseline = None;
        for n in [1, 17, 4096] {
            let result = build::from_thing_basic(&case.thing, Limits::default(), schedule(n));
            let actual = match result {
                Ok((owner, trace)) => {
                    assert_eq!(
                        owner.lifetime_remaining(),
                        Limits::default().lifetime - total(trace)
                    );
                    assert_eq!(
                        basic::validate(&build::Basic::new(&owner), &basic::InlineSink),
                        expected,
                        "{}",
                        case.label
                    );
                    assert_eq!(*baseline.get_or_insert(trace), trace, "{}", case.label);
                    Ok(())
                }
                Err(f) => {
                    assert_eq!(f.live_after_rollback, 0);
                    assert_eq!(f.allocations, f.releases);
                    assert!(matches!(f.stage, Stage::ThingBasic(_)));
                    assert_eq!(
                        f.lifetime_remaining,
                        Limits::default().lifetime - total(f.trace)
                    );
                    assert_eq!(*baseline.get_or_insert(f.trace), f.trace, "{}", case.label);
                    let ConstructionCause::ThingBasic(Cause::Invalid(error)) = f.cause else {
                        panic!("{}: {f:?}", case.label)
                    };
                    Err(error)
                }
            };
            assert_eq!(actual, expected, "{} step={n}", case.label);
        }
    }
}

#[test]
fn canonical_cursor_reads_owned_input_and_every_class_debit_is_observable() {
    let mut owner = {
        let input = corpus::serializer_failure_thing();
        assert!(serde_json::to_vec(&input).is_err());
        build::from_thing(&input, Limits::default(), schedule(17))
            .unwrap()
            .0
    };
    let source = owner.footprint().retained_requested_bytes;
    let mut baseline = None;
    for n in [1, 17, 4096] {
        let before = owner.lifetime_remaining();
        let (result, observed) =
            allocation::observe(0, || drive(Cursor::new(&mut owner, Limits::default()), n));
        let result = result.unwrap();
        assert_eq!(observed.live, 0);
        assert_eq!(observed.allocations, observed.releases);
        assert_eq!(result.resources.unwrap().source_live, source);
        assert_eq!(result.resources.unwrap().temporary_live, 0);
        assert_eq!(
            before - owner.lifetime_remaining(),
            result.work.into_iter().sum::<u64>()
        );
        assert_eq!(*baseline.get_or_insert(result.work), result.work);
        assert!(result.work[2] > 10 && result.work[3] > 0 && result.key_bytes > 0);
    }
}

#[test]
fn lifetime_and_partial_class_credit_cover_construction_seal_and_result_basic() {
    let input = corpus::typed_corpus();
    let (owner, trace) = build::from_thing_basic(&input, Limits::default(), schedule(17)).unwrap();
    let spent = total(trace);
    drop(owner);
    assert!(trace.thing_basic.work.into_iter().sum::<u64>() > 0);
    let exact = Limits {
        lifetime: spent,
        ..Limits::default()
    };
    for withheld in [
        W::DocumentNodes,
        W::CodecInputBytes,
        W::CodecOutputBytes,
        W::JsonSchemaNodes,
        W::CleanupItems,
        W::UriBytes,
        W::SecurityBranches,
    ] {
        let mut polls = 0;
        let (owner, actual) = build::from_thing_basic(&input, exact, |_| {
            polls += 1;
            assert!(polls < 1000000);
            (
                if polls % 3 == 0 {
                    budget(4096)
                } else {
                    budget(4096).with_remaining(withheld, 0)
                },
                false,
            )
        })
        .unwrap();
        assert_eq!(actual, trace);
        assert_eq!(owner.lifetime_remaining(), 0);
    }
    let f = build::from_thing_basic(
        &input,
        Limits {
            lifetime: spent - 1,
            ..exact
        },
        schedule(17),
    )
    .err()
    .unwrap();
    assert_eq!(f.cause, ConstructionCause::ThingBasic(Cause::Lifetime));
    assert_eq!(f.live_after_rollback, 0);
    assert_eq!(f.allocations, f.releases);
}

#[test]
fn all_requests_and_result_basic_failures_preserve_physical_peak_and_full_rollback() {
    let input = corpus::nested_schema_corpus();
    let ((owner, trace), observed) = allocation::observe(0, || {
        build::from_thing_basic(&input, Limits::default(), schedule(17)).unwrap()
    });
    let footprint = owner.footprint();
    assert_eq!(observed.live as u64, footprint.retained_requested_bytes);
    assert_eq!(observed.peak as u64, footprint.conversion_peak_bytes);
    assert_eq!(observed.largest as u64, footprint.largest_request_bytes);
    assert_eq!(observed.reallocations, 0);
    let (_, released) = allocation::observe(0, || drop(owner));
    assert_eq!(released.releases, 3);
    assert_eq!(released.allocations, 0);
    let mut basic_failures = 0;
    for fail_at in 1..=observed.attempts {
        let (result, observed) = allocation::observe(fail_at, || {
            build::from_thing_basic(&input, Limits::default(), schedule(17))
        });
        let f = result.err().unwrap();
        if f.cause == ConstructionCause::ThingBasic(Cause::Allocation) {
            basic_failures += 1;
        }
        assert_eq!(f.live_after_rollback, 0);
        assert_eq!(f.allocations, f.releases);
        assert_eq!(observed.live, 0);
        assert_eq!(f.resources.conversion_peak_bytes, observed.peak as u64);
        assert_eq!(f.resources.largest_request_bytes, observed.largest as u64);
        assert!(f.resources.reservation_peak_bytes >= f.resources.conversion_peak_bytes);
    }
    assert!(basic_failures > 0);
    println!(
        "whole typed Basic: work={}, basic_work={:?}, schemas={}, projections={:?}, frame_requests={}, retained={}, peak={}, allocations={}, cursor_inline={}",
        total(trace),
        trace.thing_basic.work,
        trace.thing_basic.schemas,
        trace.thing_basic.projections,
        trace.thing_basic.frame_requests,
        footprint.retained_requested_bytes,
        footprint.conversion_peak_bytes,
        observed.attempts,
        core::mem::size_of::<Cursor<'_>>()
    );
}

#[test]
fn each_basic_pending_boundary_can_cancel_or_abandon_with_prepaid_frame_release() {
    for cancel in [false, true] {
        for stop in 0..10000 {
            let mut owner =
                build::from_thing(&corpus::typed_corpus(), Limits::default(), schedule(17))
                    .unwrap()
                    .0;
            let before = owner.lifetime_remaining();
            let ((done, spent), observed) = allocation::observe(0, || {
                let mut cursor = Cursor::new(&mut owner, Limits::default());
                for poll in 0..=stop {
                    match cursor.step(&mut budget(127), || false) {
                        Progress::Pending(next) if poll == stop => {
                            let trace = next.trace();
                            if cancel {
                                let Progress::Failed(f) =
                                    next.step(&mut WorkBudget::new(), || true)
                                else {
                                    panic!("cancel")
                                };
                                assert_eq!(f.cause, Cause::Cancelled);
                                assert_eq!(f.trace.work, trace.work);
                            } else {
                                drop(next);
                            }
                            return (false, trace.work.into_iter().sum::<u64>());
                        }
                        Progress::Pending(next) => cursor = next,
                        Progress::Complete(t) => return (true, t.work.into_iter().sum()),
                        Progress::Failed(f) => panic!("{f:?}"),
                    }
                }
                unreachable!()
            });
            assert_eq!(observed.live, 0);
            assert_eq!(observed.allocations, observed.releases);
            assert_eq!(before - owner.lifetime_remaining(), spent);
            if done {
                break;
            }
            assert!(stop < 9999);
        }
    }
    let input = corpus::typed_corpus();
    let f = build::from_thing_basic(&input, Limits::default(), |stage| {
        (budget(4096), matches!(stage, Stage::ThingBasic(_)))
    })
    .err()
    .unwrap();
    assert_eq!(f.cause, ConstructionCause::ThingBasic(Cause::Cancelled));
    assert_eq!(f.live_after_rollback, 0);
    assert_eq!(f.allocations, f.releases);
}

#[test]
fn deep_canonical_schemas_and_numeric_projection_compose_without_recursive_basic() {
    use td_crate::data_schema::DataSchema;
    let mut input = corpus::typed_corpus();
    let mut node: DataSchema = DataSchema::null().build().into();
    for _ in 0..256 {
        node = DataSchema::array().items([node]).build().into();
    }
    input
        .schema_definitions
        .as_mut()
        .unwrap()
        .insert("deep".into(), node);
    input.schema_definitions.as_mut().unwrap().insert(
        "project".into(),
        serde_json::from_str(
            r#"{"minItems":1,"maxItems":2,"minimum":0.125,"maximum":1,"multipleOf":0.5}"#,
        )
        .unwrap(),
    );
    let (owner, trace) = build::from_thing_basic(&input, Limits::default(), schedule(17)).unwrap();
    assert!(trace.thing_basic.maximum_depth >= 257);
    assert!(trace.thing_basic.projections.into_iter().sum::<u64>() > 0);
    assert!(basic::validate(&build::Basic::new(&owner), &basic::InlineSink).is_ok());
    let mut owner = owner;
    let f = drive(
        Cursor::new(
            &mut owner,
            Limits {
                frames: 128,
                ..Limits::default()
            },
        ),
        17,
    )
    .unwrap_err();
    assert_eq!(
        f.cause,
        Cause::Depth {
            observed: 129,
            ceiling: 128
        }
    );
    assert_eq!(f.trace.resources.unwrap().temporary_live, 0);
}
