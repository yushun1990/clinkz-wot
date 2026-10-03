#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
use validated_thing_schema_kernel_probe::{
    schema_arena as synchronous,
    schema_fields::{Field, Shape},
    schema_step::{self as step, Cause, Cursor, Fields, Progress, Trace},
};
use validated_thing_value_construction_probe::{Cursor as ValueCursor, Limits, OwnedValue, View};
#[path = "../../validated-thing-value-construction/tests/support/mod.rs"]
#[allow(dead_code)]
mod construction;

thread_local! { static CALLS: Cell<Option<[usize; 3]>> = const { Cell::new(None) }; }
struct Observer;
// SAFETY: original pointer/Layout pairs are forwarded to System. The observer
// owns only fixed thread-local scalars and ignores other test threads.
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(0);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(1);
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(2);
        unsafe { System.realloc(pointer, layout, size) }
    }
}
fn record(kind: usize) {
    let _ = CALLS.try_with(|calls| {
        if let Some(mut counts) = calls.get() {
            counts[kind] += 1;
            calls.set(Some(counts));
        }
    });
}
#[global_allocator]
static ALLOCATOR: Observer = Observer;
fn observed<T>(operation: impl FnOnce() -> T) -> (T, [usize; 3]) {
    CALLS.with(|calls| {
        assert!(calls.get().is_none());
        calls.set(Some([0; 3]));
    });
    let result = operation();
    (result, CALLS.with(|calls| calls.replace(None).unwrap()))
}
fn literal(input: &str, allowance: u64) -> OwnedValue {
    construction::drive(
        ValueCursor::from_json(input.as_bytes(), Limits::default()),
        allowance,
    )
    .unwrap()
}
fn budget(amounts: [u64; 3]) -> WorkBudget {
    WorkBudget::new()
        .with_remaining(W::DocumentNodes, amounts[0])
        .with_remaining(W::CodecInputBytes, amounts[1])
        .with_remaining(W::JsonSchemaNodes, amounts[2])
}
fn trace(progress: &Progress<'_, '_>) -> Trace {
    match progress {
        Progress::Pending(cursor) => cursor.trace(),
        Progress::Complete { trace, .. } => *trace,
        Progress::Failed(failure) => failure.trace,
    }
}
fn sum(trace: Trace) -> u64 {
    trace.work.into_iter().sum()
}
fn drive<'a>(
    view: View<'a>,
    lifetime: &mut u64,
    ceiling: usize,
    schedule: &[[u64; 3]],
) -> Result<(Fields<'a>, Trace), step::Failure> {
    let mut cursor = Cursor::new(view, lifetime, ceiling);
    let initial = cursor.lifetime_remaining();
    for tick in 0..10_000_000 {
        let amounts = schedule[tick % schedule.len()];
        let before = cursor.trace();
        let remainder = cursor.lifetime_remaining();
        let mut allowance = budget(amounts);
        let progress = cursor.step(&mut allowance, || false);
        let after = trace(&progress);
        let spent = [W::DocumentNodes, W::CodecInputBytes, W::JsonSchemaNodes]
            .map(|class| allowance.remaining(class));
        for i in 0..3 {
            assert_eq!(after.work[i] - before.work[i], amounts[i] - spent[i]);
        }
        assert!(after.list_elements - before.list_elements <= after.work[0] - before.work[0]);
        assert!(after.key_bytes - before.key_bytes <= after.work[1] - before.work[1]);
        assert!(after.policy_runs <= Field::ALL.len() as u64 + 1);
        match progress {
            Progress::Pending(next) => {
                assert_eq!(
                    next.lifetime_remaining(),
                    remainder - (sum(after) - sum(before))
                );
                assert_eq!(next.lifetime_remaining(), initial - sum(after));
                if amounts == [0; 3] {
                    assert_eq!(before, after);
                }
                cursor = next;
            }
            Progress::Complete { fields, trace } => {
                assert_eq!(*lifetime, initial - sum(trace));
                return Ok((fields, trace));
            }
            Progress::Failed(failure) => {
                assert_eq!(*lifetime, initial - sum(failure.trace));
                return Err(failure);
            }
        }
    }
    panic!("field projection did not finish")
}

// Compile the EXACT #120 fieldwise corpus against both public models again,
// replacing only its literal decode/inspection driver with the charged pass.
// Recursive orchestration below is TEST-ONLY and makes no whole-cursor claim.
mod arena {
    pub use super::synchronous::{DecodeError, List, validate};
    use super::*;
    pub fn decode(view: View<'_>) -> Result<Fields<'_>, DecodeError> {
        drive(
            view,
            &mut 100_000_000,
            256,
            &[[0; 3], [1; 3], [2; 3], [17; 3], [4096; 3]],
        )
        .map(|(fields, _)| fields)
        .map_err(|failure| match failure.cause {
            Cause::Field(error) => error,
            other => panic!("unexpected corpus limit: {other:?}"),
        })
    }
    pub fn inspect(view: View<'_>) -> Result<(), DecodeError> {
        let fields = decode(view)?;
        if let Some(list) = fields.context.one_of {
            for i in 0..list.len() {
                inspect(list.get(i).unwrap())?;
            }
        }
        match fields.shape {
            Shape::Array {
                items: Some(list), ..
            } => {
                for i in 0..list.len() {
                    inspect(list.get(i).unwrap())?;
                }
            }
            Shape::Object {
                properties: Some(map),
                ..
            } => {
                for i in 0..map.len() {
                    inspect(map.member(i).unwrap().1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
#[allow(dead_code)] // shared corpus also supplies the old ordinary-wire observer
mod candidate {
    use validated_thing_schema_kernel_probe as td_crate;
    include!("support/schema_field_cases.rs");
}
#[allow(dead_code)]
mod unchanged {
    use clinkz_wot_td as td_crate;
    include!("support/schema_field_cases.rs");
}

#[test]
fn long_and_wide_input_has_exact_paid_units_and_small_non_numeric_steps_finish() {
    let long = "λ🦀".repeat(4096);
    let keys = (0..128)
        .map(|i| format!(r#""opaque{i}":null"#))
        .collect::<Vec<_>>()
        .join(",");
    let names = (0..512)
        .map(|i| format!(r#""tag{i}""#))
        .collect::<Vec<_>>()
        .join(",");
    let input = format!(
        r#"{{{keys},"{long}":true,"@type":[{names}],"title":"{long}","type":"{long}","required":["a","z"],"titles":{{"en":"{long}","zh":"标题"}},"readOnly":"false","const":{{"nested":[1e309,"[true,false]"]}}}}"#
    );
    let owner = literal(&input, 4096);
    let (fields, reference) = drive(owner.view(), &mut 100_000_000, 256, &[[4096; 3]]).unwrap();
    assert_eq!(fields.context.metadata.title, Some(long.as_str()));
    assert_eq!(fields.context.data_type, Some(long.as_str()));
    assert!(!fields.context.read_only);
    assert_eq!(reference.selector_bytes, 0); // long unrecognized type is NOT capped
    assert_eq!(reference.boolean_bytes, 5);
    assert_eq!(reference.list_elements, 516);
    assert_eq!(reference.parses, [0; 3]); // opaque Number is not projected
    for schedule in [
        &[[1; 3]][..],
        &[[0; 3], [1; 3], [2; 3]][..],
        &[[1, 0, 1], [0, 1, 0], [1; 3]][..],
    ] {
        let (_, actual) = drive(owner.view(), &mut 100_000_000, 256, schedule).unwrap();
        assert_eq!(actual, reference); // no credit accumulation or unpaid rescans
    }
    assert!(reference.key_bytes < 2048); // unknown long key is rejected by length
}

fn at_number<'a, 'w>(view: View<'a>, lifetime: &'w mut u64, ceiling: usize) -> Cursor<'a, 'w> {
    let mut cursor = Cursor::new(view, lifetime, ceiling);
    for _ in 0..100_000 {
        if cursor.phase() == step::Phase::Number {
            return cursor;
        }
        let Progress::Pending(next) = cursor.step(&mut budget([1; 3]), || false) else {
            panic!("expected Number pause")
        };
        cursor = next;
    }
    panic!("Number not reached")
}

#[test]
fn every_actual_scalar_parse_is_precharged_and_failed_attempts_do_not_restart() {
    // Generic integer conversion tries i64, u64, then f64 as #120 did. An
    // exponent/fraction remains a float event, rejected by the i64 visitor.
    let owner = literal(r#"{"type":"integer","minimum":1.00}"#, 7);
    let mut lifetime = 100_000;
    let cursor = at_number(owner.view(), &mut lifetime, 256);
    let before = cursor.trace();
    let remainder = cursor.lifetime_remaining();
    let mut too_small = budget([100, 3, 100]);
    let Progress::Pending(cursor) = cursor.step(&mut too_small, || false) else {
        panic!("precharge must pause")
    };
    assert_eq!(cursor.trace(), before);
    assert_eq!(cursor.lifetime_remaining(), remainder);
    assert_eq!(too_small.remaining(W::CodecInputBytes), 3);
    let mut cursor = cursor;
    for count in 1..=2 {
        let Progress::Pending(next) = cursor.step(&mut budget([100, 4, 100]), || false) else {
            panic!("next parse requires another full debit")
        };
        assert_eq!(
            next.trace().parses,
            if count == 1 { [1, 0, 0] } else { [1, 1, 0] }
        );
        assert_eq!(next.trace().work[1] - before.work[1], 4 * count);
        assert_eq!(next.lifetime_remaining(), remainder - 4 * count);
        cursor = next;
    }
    let Progress::Failed(failure) = cursor.step(&mut budget([100, 4, 100]), || false) else {
        panic!("float event is not i64")
    };
    assert_eq!(failure.trace.parses, [1; 3]);
    assert_eq!(failure.trace.work[1] - before.work[1], 12);
    assert_eq!(
        failure.cause,
        Cause::Field(synchronous::DecodeError {
            field: Some(Field::Minimum)
        })
    );
    assert!(synchronous::decode(owner.view()).is_err());

    let owner = literal(r#"{"type":"integer","minimum":18446744073709551615}"#, 7);
    let failure = drive(owner.view(), &mut 100_000, 256, &[[4096; 3]])
        .err()
        .unwrap();
    assert_eq!(failure.trace.parses, [1, 1, 0]); // real u64 visitor, not f64 coercion
    let owner = literal(r#"{"type":"number","minimum":1e309}"#, 7);
    let (fields, trace) = drive(owner.view(), &mut 100_000, 256, &[[4096; 3]]).unwrap();
    let Shape::Number(values) = fields.shape else {
        panic!("number")
    };
    assert_eq!(values[0], Some(f64::INFINITY)); // no unauthorized finite typed rule
    assert_eq!(trace.parses, [0, 0, 1]);
}

#[test]
fn lexical_ceiling_precedes_conversion_and_atomic_lifetime_cannot_reset() {
    for ceiling in [64, 256] {
        for length in [ceiling - 1, ceiling, ceiling + 1] {
            let number = format!("0.{}1", "0".repeat(length - 3));
            let input = format!(r#"{{"type":"number","minimum":{number}}}"#);
            let limits = Limits {
                number: 512,
                ..Limits::default()
            };
            let owner = construction::drive(ValueCursor::from_json(input.as_bytes(), limits), 4096)
                .unwrap();
            let result = drive(
                owner.view(),
                &mut 100_000,
                ceiling,
                &[[0; 3], [1; 3], [4096; 3]],
            );
            if length <= ceiling {
                assert_eq!(result.unwrap().1.parses, [0, 0, 1]);
            } else {
                let failure = result.err().unwrap();
                assert_eq!(
                    failure.cause,
                    Cause::NumberLimit {
                        observed: length,
                        ceiling
                    }
                );
                assert_eq!(failure.trace.parses, [0; 3]);
            }
        }
    }
    for (input, ceiling, observed) in [
        (r#"{"type":"number","minimum":0}"#, 0, 1),
        (r#"{"type":"number","minimum":1e0}"#, 3, 4),
    ] {
        let owner = literal(input, 7);
        let failure = drive(owner.view(), &mut 100_000, ceiling, &[[4096; 3]])
            .err()
            .unwrap();
        assert_eq!(failure.cause, Cause::NumberLimit { observed, ceiling });
        assert_eq!(failure.trace.parses, [0; 3]);
    }
    let owner = literal(r#"{"type":"integer","minimum":1.00}"#, 7);
    let prefix = {
        let mut lifetime = 100_000;
        sum(at_number(owner.view(), &mut lifetime, 256).trace())
    };
    let mut lifetime = prefix + 8;
    let cursor = at_number(owner.view(), &mut lifetime, 256);
    let Progress::Pending(cursor) = cursor.step(&mut budget([100, 8, 100]), || false) else {
        panic!("third attempt cannot fit current step")
    };
    assert_eq!(cursor.trace().parses, [1, 1, 0]);
    let Progress::Pending(cursor) = cursor.step(&mut budget([100, 3, 100]), || false) else {
        panic!("step shortage before lifetime")
    };
    let mut sufficient = budget([100; 3]);
    let Progress::Failed(failure) = cursor.step(&mut sufficient, || false) else {
        panic!("fresh step cannot reset lifetime")
    };
    assert_eq!(failure.cause, Cause::Lifetime);
    assert_eq!(failure.trace.parses, [1, 1, 0]);
    assert_eq!(sufficient.remaining(W::CodecInputBytes), 100);
    assert_eq!(lifetime, 0);
}

#[test]
fn value_construction_and_projection_share_one_actual_remainder() {
    let input = r#"{"title":"probe","@type":["z","a"],"readOnly":"1","const":1e309}"#;
    let limits = Limits::default();
    let mut owner =
        construction::drive(ValueCursor::from_json(input.as_bytes(), limits), 1).unwrap();
    let build_work: u64 = owner.trace().work.into_iter().sum();
    assert_eq!(owner.lifetime_remaining(), limits.lifetime - build_work);
    let footprint = owner.footprint();
    let field_work = {
        let (view, remainder) = owner.admission_parts();
        let (_, trace) = drive(view, remainder, limits.number, &[[1; 3]]).unwrap();
        sum(trace)
    };
    assert_eq!(
        owner.lifetime_remaining(),
        limits.lifetime - build_work - field_work
    );
    {
        let (view, remainder) = owner.admission_parts();
        let (_, trace) = drive(view, remainder, limits.number, &[[1; 3]]).unwrap();
        assert_eq!(sum(trace), field_work);
    }
    assert_eq!(
        owner.lifetime_remaining(),
        limits.lifetime - build_work - 2 * field_work
    );
    assert_eq!(owner.footprint(), footprint);

    let limits = Limits {
        lifetime: build_work + field_work - 1,
        ..limits
    };
    let mut owner =
        construction::drive(ValueCursor::from_json(input.as_bytes(), limits), 1).unwrap();
    let (view, remainder) = owner.admission_parts();
    let failure = drive(view, remainder, limits.number, &[[1; 3]])
        .err()
        .unwrap();
    assert_eq!(failure.cause, Cause::Lifetime);
    assert_eq!(owner.lifetime_remaining(), 0);
    // Field failure has no new owner to release. Outer rollback drops the actual
    // three sealed arenas; physical release is independently observed here.
    let ((), calls) = observed(|| drop(owner));
    assert_eq!(calls, [0, 3, 0]);
}

#[test]
fn cancellation_brackets_each_scalar_projection_without_reclassifying_first_cause() {
    let owner = literal(r#"{"type":"integer","minimum":1.00}"#, 7);
    for cancel_call in [1, 2, 3] {
        let mut lifetime = 100_000;
        let cursor = at_number(owner.view(), &mut lifetime, 256);
        let before = cursor.trace();
        let remainder = cursor.lifetime_remaining();
        let mut calls = 0;
        let mut allowance = budget([100, 4, 100]);
        let Progress::Failed(failure) = cursor.step(&mut allowance, || {
            calls += 1;
            calls == cancel_call
        }) else {
            panic!("cancellation")
        };
        assert_eq!(failure.cause, Cause::Cancelled);
        let ran = u64::from(cancel_call == 3);
        assert_eq!(failure.trace.parses, [ran, 0, 0]);
        assert_eq!(failure.trace.work[1] - before.work[1], ran * 4);
        assert_eq!(lifetime, remainder - ran * 4);
        assert_eq!(allowance.remaining(W::CodecInputBytes), 4 - ran * 4);
    }
}

#[test]
fn zero_and_cancellation_preserve_every_pending_boundary() {
    let owner = literal(
        r#"{"type":"integer","minimum":1000,"@type":["a","z"],"titles":{"en":"title"},"readOnly":"true"}"#,
        7,
    );
    let mut phases = [false; 5];
    for boundary in 0..1000 {
        let mut lifetime = 100_000;
        let mut cursor = Cursor::new(owner.view(), &mut lifetime, 256);
        let mut finished = false;
        for i in 0..=boundary {
            let input = if cursor.phase() == step::Phase::Number {
                4
            } else {
                1
            };
            match cursor.step(&mut budget([1, input, 1]), || false) {
                Progress::Pending(next) if i == boundary => {
                    let before = next.trace();
                    let remainder = next.lifetime_remaining();
                    let phase = next.phase();
                    phases[phase as usize] = true;
                    let Progress::Pending(next) = next.step(&mut budget([0; 3]), || false) else {
                        panic!("zero must pause")
                    };
                    assert_eq!(next.trace(), before);
                    assert_eq!(next.lifetime_remaining(), remainder);
                    assert_eq!(next.phase(), phase);
                    let mut allowance = budget([100; 3]);
                    let Progress::Failed(failure) = next.step(&mut allowance, || true) else {
                        panic!("cancel must terminate")
                    };
                    assert_eq!(failure.cause, Cause::Cancelled);
                    assert_eq!(failure.trace, before);
                    assert_eq!(lifetime, remainder);
                    assert_eq!(
                        [W::DocumentNodes, W::CodecInputBytes, W::JsonSchemaNodes]
                            .map(|w| allowance.remaining(w)),
                        [100; 3]
                    );
                    break;
                }
                Progress::Pending(next) => cursor = next,
                Progress::Complete { .. } => {
                    finished = true;
                    break;
                }
                Progress::Failed(failure) => panic!("unexpected failure: {failure:?}"),
            }
        }
        if finished {
            break;
        }
        assert!(boundary < 999);
    }
    assert_eq!(phases, [true; 5]);
}

#[test]
fn one_node_does_not_hide_recursive_schema_or_basic_work() {
    let input = r#"{"type":"array","items":{"unit":null},"oneOf":[{"minimum":1e309}],"readOnly":true,"writeOnly":true}"#;
    let mut owner = literal(input, 7);
    let (view, lifetime) = owner.admission_parts();
    let (fields, trace) = drive(view, lifetime, 256, &[[1; 3]]).unwrap();
    assert_eq!(trace.parses, [0; 3]);
    assert_eq!(trace.work[2], 1); // exactly one schema node, not the nested ones
    assert!(fields.context.read_only && fields.context.write_only);
    let Shape::Array {
        items: Some(items), ..
    } = fields.shape
    else {
        panic!("items")
    };
    let failure = drive(items.get(0).unwrap(), lifetime, 256, &[[1; 3]])
        .err()
        .unwrap();
    assert_eq!(
        failure.cause,
        Cause::Field(synchronous::DecodeError {
            field: Some(Field::Unit)
        })
    );
    // Basic rejection and whole conversion precedence remain in existing tests.
}

#[test]
fn duplicate_and_collision_inputs_reach_the_same_policy_without_hidden_parses() {
    for input in [
        r#"{"type":17,"type":"integer","minimum":"wrong","minimum":1,"unit":null,"unit":"last","const":17,"const":null,"ti\u0074le":"last"}"#,
        r#"{"const":{"$serde_json::private::RawValue":"[true,false]"},"default":{"$serde_json::private::Number":"1e309"},"minimum":{"$serde_json::private::Number":"1e309"}}"#,
        r#"{"type":"array","items":null,"@type":null,"title":null,"profile":null}"#,
    ] {
        let owner = literal(input, 1);
        let (fields, trace) = drive(owner.view(), &mut 100_000, 256, &[[1; 3], [4096; 3]]).unwrap();
        let original = synchronous::decode(owner.view()).unwrap();
        assert_eq!(
            fields.context.metadata.title,
            original.context.metadata.title
        );
        assert_eq!(fields.context.data_type, original.context.data_type);
        assert_eq!(fields.context.unit, original.context.unit);
        if input.contains("RawValue") {
            assert_eq!(trace.parses, [0; 3]);
            assert_eq!(
                fields.context.constant.unwrap().kind(),
                validated_thing_value_construction_probe::Kind::Object
            );
        } else if input.contains("wrong") {
            assert_eq!(trace.parses, [1, 0, 0]);
        }
    }
    let input = r#"{"type":"integer","minimum":null,"unit":null}"#;
    let owner = literal(input, 7);
    let failure = drive(owner.view(), &mut 100_000, 256, &[[1; 3]])
        .err()
        .unwrap();
    assert_eq!(
        failure.cause,
        Cause::Field(synchronous::DecodeError {
            field: Some(Field::Minimum)
        })
    );
}

#[test]
fn projection_and_every_pending_abandonment_add_no_heap_or_drop_obligation() {
    for input in [
        r#"{"title":"λ🦀","@type":["a","z"],"readOnly":"true","minimum":1e309,"const":1e309}"#,
        r#"{"type":"number","minimum":1e309}"#,
        r#"{"type":"integer","minimum":1.00}"#,
        r#"{"type":"array","items":{},"unit":null}"#,
        r#"{"readOnly":true,"writeOnly":true}"#,
    ] {
        let owner = literal(input, 7);
        let footprint = owner.footprint();
        let (_result, calls) = observed(|| {
            drive(
                owner.view(),
                &mut 100_000,
                256,
                &[[0; 3], [1; 3], [4096; 3]],
            )
        });
        assert_eq!(calls, [0; 3]);
        assert_eq!(owner.footprint(), footprint);
        // Reach and abandon every Pending boundary under this fixed schedule.
        for boundary in 0..1000 {
            let mut lifetime = 100_000;
            let (finished, calls) = observed(|| {
                let mut cursor = Cursor::new(owner.view(), &mut lifetime, 256);
                for i in 0..=boundary {
                    match cursor.step(
                        &mut budget(if i % 5 == 4 { [4096; 3] } else { [1; 3] }),
                        || false,
                    ) {
                        Progress::Pending(next) if i == boundary => {
                            let _abandoned = next;
                            return false;
                        }
                        Progress::Pending(next) => cursor = next,
                        Progress::Complete { .. } | Progress::Failed(_) => return true,
                    }
                }
                unreachable!()
            });
            assert_eq!(calls, [0; 3]);
            if finished {
                break;
            }
            assert!(boundary < 999);
        }
        let (_, calls) = observed(|| {
            let mut remainder = 0;
            assert!(matches!(
                Cursor::new(owner.view(), &mut remainder, 256).step(&mut budget([1; 3]), || false),
                Progress::Failed(step::Failure {
                    cause: Cause::Lifetime,
                    ..
                })
            ));
        });
        assert_eq!(calls, [0; 3]);
        let (_, calls) = observed(|| {
            let mut remainder = 100_000;
            assert!(matches!(
                Cursor::new(owner.view(), &mut remainder, 256).step(&mut budget([0; 3]), || true),
                Progress::Failed(step::Failure {
                    cause: Cause::Cancelled,
                    ..
                })
            ));
        });
        assert_eq!(calls, [0; 3]);
    }
    assert!(!std::mem::needs_drop::<Cursor<'static, 'static>>());
    assert!(!std::mem::needs_drop::<Fields<'static>>());
    println!(
        "field cursor {} / fields {} inline Host bytes",
        std::mem::size_of::<Cursor<'static, 'static>>(),
        std::mem::size_of::<Fields<'static>>()
    );
}

#[test]
fn trusted_text_ranges_retain_utf8_and_do_not_admit_malformed_input() {
    for input in [
        r#"{"title":"λ🦀","unit":"\uD83E\uDD80","const":"\u0000λ"}"#,
        r#"{"type":"λ","readOnly":"λ"}"#,
    ] {
        let owner = literal(input, 1);
        let _ = drive(owner.view(), &mut 100_000, 256, &[[1; 3]]);
        assert!(
            owner
                .view()
                .get("title")
                .is_none_or(|v| v.text() == Some("λ🦀"))
        );
        assert!(
            owner
                .view()
                .get("unit")
                .is_none_or(|v| v.text() == Some("🦀"))
        );
    }
    for input in [
        b"{\"title\":\"\xff\"}".as_slice(),
        br#"{"unit":"\uD800"}"#,
        br#"{"title":"\uDE00"}"#,
    ] {
        assert!(construction::drive(ValueCursor::from_json(input, Limits::default()), 1).is_err());
    }
}
