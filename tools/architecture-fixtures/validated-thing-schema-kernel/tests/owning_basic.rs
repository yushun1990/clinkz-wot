#![cfg(feature = "validated-thing")]
#![allow(clippy::result_large_err)]
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use td as td_crate;
use td::{
    schema_build as build,
    thing_step::{Cause, Output, OwningCursor, OwningProgress, Trace},
};
use validated_thing_schema_kernel_probe as td;
use validated_thing_value_construction_probe::Limits;
#[path = "support/allocator.rs"]
mod allocation;
#[path = "../../../../td/tests/support/typed_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;
#[path = "support/canonical_fields.rs"]
mod fields;

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
fn control(_: build::Stage) -> (WorkBudget, bool) {
    (budget(4096), false)
}
fn total(t: build::ConstructionTrace) -> u64 {
    t.literal.work.into_iter().sum::<u64>()
        + t.basic.work.into_iter().sum::<u64>()
        + t.canonical.work.into_iter().sum::<u64>()
        + t.seal.into_iter().sum::<u64>()
        + t.thing_basic.work.into_iter().sum::<u64>()
        + t.schema_basic.work.into_iter().sum::<u64>()
}
fn static_owner<T: 'static>(value: T) -> T {
    value
}

// Separate, address-observable inline slots move the complete owner on EVERY
// Pending. This is test storage, not a heap owner, Pin, or an engine allocation.
fn drive(
    cursor: OwningCursor,
    step: u64,
) -> Result<(Output, Trace, usize), td::thing_step::OwningFailure> {
    let initial = cursor.lifetime_remaining() + cursor.trace().work.into_iter().sum::<u64>();
    let mut slots = [Some(static_owner(cursor)), None];
    let mut slot = 0;
    for poll in 0..1_000_000 {
        let address = slots[slot].as_ref().unwrap() as *const OwningCursor as usize;
        let c = slots[slot].take().unwrap();
        let before = c.trace();
        let phase = c.phase();
        let lifetime = c.lifetime_remaining();
        let OwningProgress::Pending(c) = c.step(&mut WorkBudget::new(), || false) else {
            panic!("zero allowance at {phase:?}")
        };
        assert_eq!(c.trace(), before);
        assert_eq!(c.phase(), phase);
        assert_eq!(c.lifetime_remaining(), lifetime);
        let amount = if poll % 23 == 22 { step.max(512) } else { step };
        let mut work = budget(amount);
        let progress = c.step(&mut work, || false);
        let after = match &progress {
            OwningProgress::Pending(c) => c.trace(),
            OwningProgress::Complete { trace, .. } => *trace,
            OwningProgress::Failed(f) => f.failure.trace,
        };
        for (i, class) in CLASSES.into_iter().enumerate() {
            assert_eq!(
                after.work[i] - before.work[i],
                amount - work.remaining(class)
            );
        }
        match progress {
            OwningProgress::Pending(c) => {
                assert_eq!(
                    c.lifetime_remaining(),
                    initial - after.work.into_iter().sum::<u64>()
                );
                slot = 1 - slot;
                slots[slot] = Some(c);
                let moved = std::hint::black_box(&slots[slot]).as_ref().unwrap()
                    as *const OwningCursor as usize;
                assert_ne!(address, moved);
            }
            OwningProgress::Complete { output, trace } => return Ok((output, trace, poll)),
            OwningProgress::Failed(f) => return Err(f),
        }
    }
    panic!("no completion")
}

const STRICT: &str = r#"{"type":"bogus","type":"array","title":"discarded","ti\u0074le":"kept","readOnly":true,"readOnly":"false","items":[{"type":"null","minimum":1.00,"maximum":2,"minLength":0,"maxLength":8},{"type":"object","properties":{"z":{"type":"string","pattern":"λ+"},"a":{"type":"boolean"}}}],"default":{"z":[1E0,1e309,"[true,false]"],"a":{"ordinary-key":"1e309"}},"const":null}"#;

#[test]
fn strict_and_typed_results_share_one_movable_basic_engine_after_input_drop() {
    let expected: td::data_schema::DataSchema =
        serde_json::from_value(serde_json::from_str::<serde_json::Value>(STRICT).unwrap()).unwrap();
    for step in [1, 7, 4096] {
        let c = {
            let input = STRICT.to_owned();
            let (schema, _) =
                build::from_json(input.as_bytes(), Limits::default(), control).unwrap();
            static_owner(OwningCursor::from_schema(schema, Limits::default()))
        };
        let (result, observed) = allocation::observe(0, || drive(c, step));
        let (Output::Schema(schema), trace, moves) = result.unwrap() else {
            panic!("wrong scope")
        };
        fields::fields(schema.view(), &expected);
        assert_eq!(trace.schemas, 5);
        assert_eq!(trace.projections, [2, 2]);
        assert!(trace.key_bytes > 0);
        assert_eq!(observed.live, 0); // only semantic frames allocated here
        assert_eq!(observed.allocations, observed.releases);
        assert_eq!(observed.reallocations, 0);
        if step < 10 {
            assert!(moves > 10);
        }
    }
    let c = {
        let input = corpus::nested_schema_corpus();
        let (owner, _) = td::thing_build::from_thing(&input, Limits::default(), control).unwrap();
        OwningCursor::new(owner, Limits::default())
    };
    let (result, observed) = allocation::observe(0, || drive(c, 1));
    let (Output::Thing(owner), trace, moves) = result.unwrap() else {
        panic!("wrong scope")
    };
    assert!(trace.schemas > 20);
    assert!(moves > 10);
    assert_eq!(observed.live, 0);
    assert_eq!(observed.allocations, observed.releases);
    assert!(owner.view().root_field("properties").is_some());
}

#[test]
fn strict_result_basic_is_part_of_the_original_work_and_allocation_transaction() {
    let limits = Limits::default();
    let (result, observed) = allocation::observe(0, || {
        build::from_json_basic(STRICT.as_bytes(), limits, control)
    });
    let (owner, trace) = result.unwrap();
    fields::fields(
        owner.view(),
        &serde_json::from_value(serde_json::from_str::<serde_json::Value>(STRICT).unwrap())
            .unwrap(),
    );
    let spent = total(trace);
    assert_eq!(owner.lifetime_remaining(), limits.lifetime - spent);
    assert_eq!(
        observed.live as u64,
        owner.footprint().retained_requested_bytes
    );
    assert_eq!(
        observed.peak as u64,
        owner.footprint().conversion_peak_bytes
    );
    assert_eq!(
        observed.largest as u64,
        owner.footprint().largest_request_bytes
    );
    assert_eq!(observed.allocations - observed.releases, 3);
    assert_eq!(observed.reallocations, 0);
    drop(owner);
    for step in [1, 17, 4096] {
        let mut polls = 0;
        let (owner, t) = build::from_json_basic(
            STRICT.as_bytes(),
            Limits {
                lifetime: spent,
                ..limits
            },
            |_| {
                polls += 1;
                (
                    budget(if polls % 23 == 0 { step.max(512) } else { step }),
                    false,
                )
            },
        )
        .unwrap();
        assert_eq!(total(t), spent);
        assert_eq!(owner.lifetime_remaining(), 0);
    }
    let failure = build::from_json_basic(
        STRICT.as_bytes(),
        Limits {
            lifetime: spent - 1,
            ..limits
        },
        control,
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.cause,
        build::ConstructionCause::SchemaBasic(Cause::Lifetime)
    );
    assert_eq!(failure.allocations, failure.releases);
    assert_eq!(failure.live_after_rollback, 0);
    for fail_at in 1..=observed.attempts {
        let (failure, observed) = allocation::observe(fail_at, || {
            build::from_json_basic(STRICT.as_bytes(), limits, control)
        });
        let failure = failure.err().unwrap();
        assert_eq!(failure.allocations, failure.releases);
        assert_eq!(failure.live_after_rollback, 0);
        assert_eq!(observed.allocations, observed.releases);
        assert_eq!(observed.live, 0);
        assert_eq!(
            observed.peak as u64,
            failure.resources.conversion_peak_bytes
        );
        assert_eq!(
            observed.largest as u64,
            failure.resources.largest_request_bytes
        );
    }
    println!(
        "strict transaction: attempts={} spent={} footprint={:?}; owning cursor={} frame trace={:?}",
        observed.attempts,
        spent,
        trace.schema_basic.resources,
        std::mem::size_of::<OwningCursor>(),
        trace.schema_basic
    );
}

fn nested(depth: usize) -> String {
    let mut input = String::new();
    for _ in 0..depth {
        input.push_str(r#"{"type":"array","items":["#);
    }
    input.push_str(r#"{"type":"null","minimum":1.00,"maximum":2,"minItems":0,"maxItems":3}"#);
    for _ in 0..depth {
        input.push_str("]}");
    }
    input
}
#[test]
fn pending_frame_growth_cancellation_and_abandonment_release_the_whole_owner() {
    let input = nested(8);
    let mut polls = 0;
    let mut transfers = 0;
    let baseline = allocation::observe(0, || {
        let (schema, _) = build::from_json(input.as_bytes(), Limits::default(), control).unwrap();
        let mut cursor = OwningCursor::from_schema(schema, Limits::default());
        loop {
            match cursor.step(&mut budget(if polls % 23 == 22 { 256 } else { 1 }), || {
                false
            }) {
                OwningProgress::Pending(mut c) => {
                    polls += 1;
                    transfers += usize::from(c.frame_transfer());
                    cursor = c;
                }
                OwningProgress::Complete { output, .. } => {
                    drop(output);
                    break;
                }
                OwningProgress::Failed(f) => panic!("{f:?}"),
            }
        }
    })
    .1;
    assert!(transfers > 0);
    assert_eq!(baseline.live, 0);
    assert_eq!(baseline.allocations, baseline.releases);
    for stop in 0..=polls {
        for cancel in [false, true] {
            let ((), observed) = allocation::observe(0, || {
                let (schema, _) =
                    build::from_json(input.as_bytes(), Limits::default(), control).unwrap();
                let mut c = OwningCursor::from_schema(schema, Limits::default());
                for poll in 0..stop {
                    let OwningProgress::Pending(next) =
                        c.step(&mut budget(if poll % 23 == 22 { 256 } else { 1 }), || false)
                    else {
                        panic!("early terminal")
                    };
                    c = next;
                }
                if cancel {
                    let OwningProgress::Failed(f) = c.step(&mut WorkBudget::new(), || true) else {
                        panic!("cancel ignored")
                    };
                    assert_eq!(f.failure.cause, Cause::Cancelled);
                    assert_eq!(f.allocations, f.releases);
                    assert_eq!(f.live_after_rollback, 0);
                } else {
                    drop(c);
                }
            });
            assert_eq!(observed.live, 0, "stop={stop} cancel={cancel}");
            assert_eq!(observed.allocations, observed.releases);
            assert_eq!(observed.reallocations, 0);
        }
    }
}

#[test]
fn deep_strict_owner_and_number_projection_resume_without_a_source_borrow() {
    let input = nested(256);
    let (schema, _) = build::from_json(input.as_bytes(), Limits::default(), control).unwrap();
    drop(input);
    let (Output::Schema(_), trace, moves) =
        drive(OwningCursor::from_schema(schema, Limits::default()), 1).unwrap()
    else {
        panic!("scope")
    };
    assert_eq!(trace.maximum_depth, 257);
    assert_eq!(trace.schemas, 257);
    assert_eq!(trace.projections, [2, 2]);
    assert!(trace.frame_copies > 0 && moves > 100);
}

#[test]
fn literal_collisions_and_discarded_discriminators_reach_the_movable_result_unchanged() {
    // The ordinary adapter is an observation, not the strict value authority.
    // This Number-looking object is an Object for strict fields and therefore
    // absent for Basic's numeric predicate, including after canonical rebuild.
    let input = r#"{"type":"null","minimum":{"$serde_json::private::Number":"1e309"},"default":{"$serde_json::private::RawValue":"[true,false]"},"const":"[true,false]"}"#;
    let (schema, trace) =
        build::from_json_basic(input.as_bytes(), Limits::default(), control).unwrap();
    assert_eq!(trace.schema_basic.projections, [0, 0]);
    let default = schema
        .view()
        .field(td::schema_fields::Field::Default)
        .unwrap();
    assert_eq!(
        default.literal_kind(),
        Some(validated_thing_value_construction_probe::Kind::Object)
    );
    assert_eq!(default.member(0).unwrap().1.text(), Some("[true,false]"));
    assert_eq!(
        schema
            .view()
            .field(td::schema_fields::Field::Const)
            .unwrap()
            .text(),
        Some("[true,false]")
    );
    let ordinary: td::data_schema::DataSchema = serde_json::from_str(input).unwrap();
    use td::validate::{Validate, ValidationLevel};
    assert!(
        ordinary
            .validate_with_level(ValidationLevel::Basic)
            .is_err()
    );
    // Earlier fields still undergo literal syntax/resource processing, but a
    // later equal decoded name fixes the dispatch and value before field policy.
    for wire in [
        r#"{"unit":null,"unit":"m","type":"unknown","ty\u0070e":"null"}"#,
        r#"{"minimum":1e309,"minimum":1,"type":"null"}"#,
    ] {
        let (schema, trace) =
            build::from_json_basic(wire.as_bytes(), Limits::default(), control).unwrap();
        assert_eq!(
            schema.view().schema_kind(),
            Some(td::schema_kernel::SchemaKind::Null)
        );
        assert_eq!(trace.schema_basic.schemas, 1);
    }
    // Malformed overwritten bytes cannot be made valid by a later duplicate.
    let failure = build::from_json_basic(
        br#"{"minimum":1e+,"minimum":1}"#,
        Limits::default(),
        control,
    )
    .err()
    .unwrap();
    assert_eq!(
        failure.cause,
        build::ConstructionCause::Literal(validated_thing_value_construction_probe::Cause::Syntax)
    );
    assert_eq!(failure.live_after_rollback, 0);
}

#[test]
fn owning_atomic_projection_preserves_partial_credit_and_lifetime_across_moves() {
    for ceiling in [64, 256] {
        for length in [ceiling - 1, ceiling, ceiling + 1] {
            let text = format!("0.{}1", "0".repeat(length - 3));
            let wire = format!(r#"{{"type":"null","minimum":{text}}}"#);
            let limits = Limits {
                number: ceiling,
                ..Limits::default()
            };
            if length > ceiling {
                let f = build::from_json_basic(wire.as_bytes(), limits, control)
                    .err()
                    .unwrap();
                assert_eq!(
                    f.cause,
                    build::ConstructionCause::Literal(
                        validated_thing_value_construction_probe::Cause::RawNumber
                    )
                );
                assert_eq!(f.live_after_rollback, 0);
                continue;
            }
            let (schema, _) = build::from_json(wire.as_bytes(), limits, control).unwrap();
            let mut c = OwningCursor::from_schema(schema, limits);
            for poll in 0..10000 {
                let before = c.trace();
                let OwningProgress::Pending(next) = c.step(&mut budget(1), || false) else {
                    panic!("atomic stop missed")
                };
                let stopped =
                    next.phase() == td::thing_step::Phase::Numeric && next.trace() == before;
                c = static_owner(next);
                if stopped {
                    break;
                }
                assert!(poll < 9999);
            }
            let before = c.trace();
            let lifetime = c.lifetime_remaining();
            for _ in 0..3 {
                let mut partial = budget(0).with_remaining(W::CodecInputBytes, (length - 1) as u64);
                let OwningProgress::Pending(next) = c.step(&mut partial, || false) else {
                    panic!("partial projection")
                };
                assert_eq!(next.trace(), before);
                assert_eq!(next.lifetime_remaining(), lifetime);
                assert_eq!(partial.remaining(W::CodecInputBytes), (length - 1) as u64);
                c = static_owner(next);
            }
            let mut exact = WorkBudget::new().with_remaining(W::CodecInputBytes, length as u64);
            let OwningProgress::Pending(next) = c.step(&mut exact, || false) else {
                panic!("unpaid structural progress")
            };
            assert_eq!(next.trace().projections[1], before.projections[1] + 1);
            assert_eq!(next.trace().work[1] - before.work[1], length as u64);
            assert_eq!(next.lifetime_remaining(), lifetime - length as u64);
            assert_eq!(exact.remaining(W::CodecInputBytes), 0);
            drive(next, 1).unwrap();
        }
    }
}
