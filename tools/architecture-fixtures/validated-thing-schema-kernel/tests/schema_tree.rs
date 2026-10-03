#![cfg(feature = "validated-thing")]
// The measured helper carries its full fixed trace inline; boxing errors would
// contaminate the allocator interval. These are test records, not diagnostics.
#![allow(clippy::result_large_err)]
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
use validated_thing_schema_kernel_probe::{
    data_schema::DataSchema,
    schema_arena as reference,
    schema_fields::Field as DecodeField,
    schema_kernel::{Field, Rule},
    schema_tree::{Cause, Cursor, Failure, Phase, Progress, Trace},
    validate::{Validate, ValidationLevel},
};
use validated_thing_value_construction_probe::{Cursor as ValueCursor, Limits, OwnedValue};
#[path = "../../validated-thing-value-construction/tests/support/mod.rs"]
#[allow(dead_code)]
mod construction;

#[derive(Clone, Copy, Debug, Default)]
struct Observation {
    attempts: usize,
    allocations: usize,
    releases: usize,
    reallocations: usize,
    live: i64,
    peak: usize,
    largest: usize,
    fail_at: usize,
}
thread_local! { static OBSERVER: Cell<Option<Observation>> = const { Cell::new(None) }; }
struct Allocator;
// SAFETY: unchanged pointer/Layout pairs are forwarded. Fixed thread-local
// scalars track only this test thread; failed requests preserve live truth.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = OBSERVER.with(|cell| {
            let Some(mut record) = cell.get() else {
                return false;
            };
            record.attempts += 1;
            let fail = record.attempts == record.fail_at;
            cell.set(Some(record));
            fail
        });
        if fail {
            return std::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            OBSERVER.with(|cell| {
                if let Some(mut record) = cell.get() {
                    record.allocations += 1;
                    record.live += layout.size() as i64;
                    record.peak = record.peak.max(record.live.max(0) as usize);
                    record.largest = record.largest.max(layout.size());
                    cell.set(Some(record));
                }
            });
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        OBSERVER.with(|cell| {
            if let Some(mut record) = cell.get() {
                record.releases += 1;
                record.live -= layout.size() as i64;
                cell.set(Some(record));
            }
        });
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        OBSERVER.with(|cell| {
            if let Some(mut record) = cell.get() {
                record.reallocations += 1;
                cell.set(Some(record));
            }
        });
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn observe<T>(fail_at: usize, operation: impl FnOnce() -> T) -> (T, Observation) {
    OBSERVER.with(|cell| {
        assert!(cell.get().is_none());
        cell.set(Some(Observation {
            fail_at,
            ..Observation::default()
        }));
    });
    let result = operation();
    (result, OBSERVER.with(|cell| cell.replace(None).unwrap()))
}
const CLASSES: [W; 4] = [
    W::DocumentNodes,
    W::CodecInputBytes,
    W::JsonSchemaNodes,
    W::CleanupItems,
];
fn budget(amounts: [u64; 4]) -> WorkBudget {
    CLASSES
        .into_iter()
        .zip(amounts)
        .fold(WorkBudget::new(), |budget, (class, amount)| {
            budget.with_remaining(class, amount)
        })
}
fn trace(progress: &Progress<'_>) -> Trace {
    match progress {
        Progress::Pending(cursor) => cursor.trace(),
        Progress::Complete(trace) => *trace,
        Progress::Failed(failure) => failure.trace,
    }
}
fn spent(trace: Trace) -> u64 {
    trace.work.into_iter().sum()
}
fn literal(input: &str, limits: Limits) -> OwnedValue {
    construction::drive(ValueCursor::from_json(input.as_bytes(), limits), 4096).unwrap()
}
fn drive(mut cursor: Cursor<'_>, schedule: &[[u64; 4]]) -> Result<Trace, Failure> {
    let initial = cursor.lifetime_remaining();
    for step in 0..10_000_000 {
        let amounts = schedule[step % schedule.len()];
        let before = cursor.trace();
        let mut work = budget(amounts);
        let progress = cursor.step(&mut work, || false);
        let after = trace(&progress);
        for i in 0..4 {
            assert_eq!(
                after.work[i] - before.work[i],
                amounts[i] - work.remaining(CLASSES[i])
            );
        }
        match progress {
            Progress::Pending(next) => {
                assert_eq!(next.lifetime_remaining(), initial - spent(after));
                if amounts == [0; 4] {
                    assert_eq!(before, after);
                }
                cursor = next;
            }
            Progress::Complete(trace) => return Ok(trace),
            Progress::Failed(failure) => return Err(failure),
        }
    }
    panic!("subtree did not make bounded progress")
}
fn run(owner: &mut OwnedValue, schedule: &[[u64; 4]]) -> Result<Trace, Failure> {
    drive(Cursor::new(owner, 256, 4096), schedule)
}
fn first_cause(input: &str) -> Cause {
    let mut owner = literal(input, Limits::default());
    let failure = run(&mut owner, &[[7; 4]]).unwrap_err();
    assert_eq!(failure.trace.resources.unwrap().temporary_live, 0);
    failure.cause
}

#[test]
fn complete_decode_precedes_basic_and_the_shared_discovery_order_survives_suspension() {
    for input in [
        r#"{"readOnly":true,"writeOnly":true,"oneOf":[{"unit":null}]}"#,
        r#"{"minimum":2,"maximum":1,"properties":{"later":{"unit":null}}}"#,
        r#"{"oneOf":[{"multipleOf":0}],"properties":{"later":{"unit":null}}}"#,
        r#"{"type":"array","minItems":2,"maxItems":1,"items":{"unit":null}}"#,
    ] {
        assert!(matches!(
            first_cause(input),
            Cause::Field {
                error: reference::DecodeError {
                    field: Some(DecodeField::Unit)
                },
                ..
            }
        ));
    }
    for input in [
        r#"{"oneOf":[0]}"#,
        r#"{"type":"array","items":[0]}"#,
        r#"{"properties":{"nested":0}}"#,
    ] {
        assert_eq!(
            first_cause(input),
            Cause::Field {
                ordinal: 1,
                error: reference::DecodeError { field: None }
            }
        );
    }
    for (input, ordinal, rule) in [
        (
            r#"{"type":"string","readOnly":true,"writeOnly":true,"oneOf":[{"type":"integer","multipleOf":0}]}"#,
            1,
            Rule::Positive(Field::MultipleOf),
        ),
        (
            r#"{"type":"array","items":{"multipleOf":0},"minItems":2,"maxItems":1}"#,
            0,
            Rule::Ordered(Field::MinItems, Field::MaxItems),
        ),
        (
            r#"{"properties":{"z":{"multipleOf":0},"a":{"readOnly":true,"writeOnly":true}}}"#,
            1,
            Rule::ReadWrite,
        ),
        (
            r#"{"oneOf":[{}, {"oneOf":[{"maximum":1,"minimum":2}]}],"readOnly":true,"writeOnly":true}"#,
            3,
            Rule::Ordered(Field::Minimum, Field::Maximum),
        ),
        (
            r#"{"type":"unknown","oneOf":[{"multipleOf":0}]}"#,
            0,
            Rule::TypeMismatch,
        ),
    ] {
        let cause = first_cause(input);
        assert_eq!(
            cause,
            Cause::Basic(
                validated_thing_schema_kernel_probe::schema_kernel::InlineInvalid { ordinal, rule }
            )
        );
        for schedule in [&[[1; 4], [17; 4]][..], &[[4096; 4]][..]] {
            let mut owner = literal(input, Limits::default());
            assert_eq!(run(&mut owner, schedule).unwrap_err().cause, cause);
            let Err(reference::Invalid::Basic(expected)) = reference::validate(owner.view()) else {
                panic!("oracle");
            };
            assert_eq!(cause, Cause::Basic(expected));
        }
    }
}

#[test]
fn all_schema_shapes_and_numeric_predicates_compose_through_every_child_boundary() {
    let cases = [
        r#"{}"#,
        r#"{"type":"null"}"#,
        r#"{"type":"boolean"}"#,
        r#"{"type":"array","items":[],"minItems":1,"maxItems":2}"#,
        r#"{"type":"object","required":["undefined"],"properties":{}}"#,
        r#"{"type":"string","pattern":"λ+","contentEncoding":"base64","contentMediaType":"text/plain"}"#,
        r#"{"type":"number","minimum":1e309,"maximum":1e309}"#,
        r#"{"type":"integer","minimum":9007199254740993,"maximum":9007199254740992}"#,
        r#"{"type":"string","minimum":9007199254740993,"maximum":9007199254740992}"#,
        r#"{"type":"string","multipleOf":1e-4000}"#,
        r#"{"minItems":18446744073709551615,"maxItems":18446744073709551614}"#,
        r#"{"minItems":18446744073709551616,"maxItems":1}"#,
        r#"{"minLength":1e0,"maxLength":0}"#,
        r#"{"const":1e309,"default":{"minimum":1e309},"enum":[1e309]}"#,
        r#"{"oneOf":[{"properties":{"z":{"type":"array","items":[{"type":"string"},{}]}}}]}"#,
    ];
    let mut checked = 0;
    for schema in cases.into_iter().map(str::to_owned).chain(
        [
            "minimum",
            "exclusiveMinimum",
            "maximum",
            "exclusiveMaximum",
            "multipleOf",
        ]
        .into_iter()
        .flat_map(|field| {
            ["1e309", "0", "null", "false", "{}", r#""[true,false]""#]
                .into_iter()
                .map(move |value| format!(r#"{{"type":"string","{field}":{value}}}"#))
        }),
    ) {
        for input in [
            schema.clone(),
            format!(r#"{{"oneOf":[{schema}]}}"#),
            format!(r#"{{"type":"array","items":{schema}}}"#),
            format!(r#"{{"properties":{{"nested":{schema}}}}}"#),
        ] {
            let typed: DataSchema = serde_json::from_str(&input).unwrap();
            let mut owner = literal(&input, Limits::default());
            let expected = reference::validate(owner.view());
            let actual = run(&mut owner, &[[0; 4], [1; 4], [2; 4], [17; 4], [64; 4]]);
            assert_eq!(
                actual.is_ok(),
                typed.validate_with_level(ValidationLevel::Basic).is_ok(),
                "{input}"
            );
            match (actual, expected) {
                (Ok(_), Ok(())) => {}
                (Err(actual), Err(reference::Invalid::Basic(expected))) => {
                    assert_eq!(actual.cause, Cause::Basic(expected), "{input}")
                }
                other => panic!("{input}: {other:?}"),
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 180);
}

const RICH: &str = r#"{"type":"array","minimum":1.25,"maximum":4,"items":[{"oneOf":[{"oneOf":[{"type":"string","multipleOf":0.5}]}]},{"properties":{"z":{},"a":{"type":"integer","minimum":1}}}]}"#;
#[test]
fn exact_traces_and_one_nonresettable_lifetime_cover_decode_basic_and_frame_transfers() {
    let mut baseline = None;
    for schedule in [&[[4096; 4]][..], &[[0; 4], [1; 4], [2; 4], [17; 4]][..]] {
        let mut owner = literal(RICH, Limits::default());
        let initial = owner.lifetime_remaining();
        let source = owner.footprint().retained_requested_bytes;
        let trace = run(&mut owner, schedule).unwrap();
        assert_eq!(owner.lifetime_remaining(), initial - spent(trace));
        assert_eq!(trace.nodes, [7, 7]);
        assert_eq!(trace.maximum_depth, 4);
        assert_eq!(trace.frame_requests, 3);
        assert_eq!(trace.frame_copies, 3);
        assert_eq!(trace.field_parses, [2, 0, 0]);
        assert_eq!(trace.numeric_parses, 3);
        assert_eq!(trace.unsigned_parses, 0);
        assert_eq!(trace.work[3], trace.frame_requests);
        assert_eq!(trace.resources.unwrap().temporary_live, 0);
        assert_eq!(trace.resources.unwrap().live, source);
        if let Some(expected) = baseline {
            assert_eq!(trace, expected);
        } else {
            baseline = Some(trace);
        }
        let repeated = run(&mut owner, schedule).unwrap();
        assert_eq!(repeated.work, trace.work);
        assert_eq!(owner.lifetime_remaining(), initial - 2 * spent(trace));
    }
    let mut owner = literal(RICH, Limits::default());
    let construction_work: u64 = owner.trace().work.into_iter().sum();
    let full = run(&mut owner, &[[4096; 4]]).unwrap();
    for remainder in [spent(full) - 1, spent(full)] {
        let limits = Limits {
            lifetime: construction_work + remainder,
            ..Limits::default()
        };
        let mut owner = literal(RICH, limits);
        assert_eq!(owner.lifetime_remaining(), remainder);
        let result = run(&mut owner, &[[4096; 4]]);
        assert_eq!(result.is_ok(), remainder == spent(full));
        if let Err(failure) = result {
            assert_eq!(failure.cause, Cause::Lifetime);
        }
        assert_eq!(owner.lifetime_remaining(), 0);
    }
}

#[test]
fn the_actual_frame_requests_overlap_the_actual_source_ledger_and_release_before_terminal() {
    let mut owner = literal(RICH, Limits::default());
    let baseline = owner.footprint();
    let allocations = owner.allocations();
    let releases = owner.releases();
    let (trace, observed) = observe(0, || run(&mut owner, &[[1; 4], [17; 4]]).unwrap());
    let resources = trace.resources.unwrap();
    assert_eq!(observed.allocations as u64, trace.frame_requests);
    assert_eq!(observed.allocations, observed.releases);
    assert_eq!(observed.live, 0);
    assert_eq!(observed.reallocations, 0);
    assert_eq!(
        resources.temporary_peak,
        baseline.temporary_peak_bytes.max(observed.peak as u64)
    );
    assert_eq!(
        resources.conversion_peak,
        baseline
            .conversion_peak_bytes
            .max(baseline.retained_requested_bytes + observed.peak as u64)
    );
    assert_eq!(
        resources.largest_request,
        baseline.largest_request_bytes.max(observed.largest as u64)
    );
    assert_eq!(owner.allocations() - allocations, trace.frame_requests);
    assert_eq!(owner.releases() - releases, trace.frame_requests);
    assert_eq!(
        owner.footprint().retained_requested_bytes,
        baseline.retained_requested_bytes
    );
    println!(
        "subtree resource witness: source={}, frame peak={}, largest frame request={}, source/frames peak={}",
        baseline.retained_requested_bytes,
        observed.peak,
        observed.largest,
        baseline.retained_requested_bytes + observed.peak as u64
    );
    let (_, dropped) = observe(0, || drop(owner));
    assert_eq!(dropped.allocations, 0);
    assert_eq!(dropped.releases as u64, baseline.retained_allocation_count);
    assert_eq!(dropped.live, -(baseline.retained_requested_bytes as i64));
    // Source allocations began before this interval; observe their actual sizes
    // without including them in the interval's new-allocation live count.
}

#[test]
fn every_frame_allocation_failure_and_memory_rejection_leave_only_the_original_source() {
    let mut owner = literal(RICH, Limits::default());
    let trace = run(&mut owner, &[[4096; 4]]).unwrap();
    for request in 1..=trace.frame_requests as usize {
        let mut owner = literal(RICH, Limits::default());
        let source = owner.footprint().retained_requested_bytes;
        let (failure, observed) =
            observe(request, || run(&mut owner, &[[1; 4], [17; 4]]).unwrap_err());
        assert_eq!(failure.cause, Cause::Allocation);
        assert_eq!(observed.attempts, request);
        assert_eq!(observed.allocations, observed.releases);
        assert_eq!(observed.live, 0);
        assert_eq!(failure.trace.resources.unwrap().temporary_live, 0);
        assert_eq!(failure.trace.resources.unwrap().live, source);
    }
    // A deeper semantic stack is larger than the constructor's literal frames.
    let input = format!("{}{{}}{}", r#"{"oneOf":["#.repeat(40), "]}".repeat(40));
    let mut owner = literal(&input, Limits::default());
    let (_, observed) = observe(0, || run(&mut owner, &[[4096; 4]]).unwrap());
    let source = owner.footprint().retained_requested_bytes;
    for (axis, delta) in [
        (0, -1_i64),
        (0, 0),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
        (2, -1),
        (2, 0),
        (2, 1),
    ] {
        let mut limits = Limits::default();
        match axis {
            0 => limits.temporary = (observed.peak as u64).checked_add_signed(delta).unwrap(),
            1 => limits.contiguous = (observed.largest as u64).checked_add_signed(delta).unwrap(),
            2 => {
                limits.peak = (source + observed.peak as u64)
                    .checked_add_signed(delta)
                    .unwrap()
            }
            _ => unreachable!(),
        }
        let mut owner = literal(&input, limits);
        let (result, calls) = observe(0, || run(&mut owner, &[[4096; 4]]));
        assert_eq!(result.is_ok(), delta >= 0);
        assert_eq!(calls.live, 0);
        if let Err(failure) = result {
            assert_eq!(failure.cause, Cause::Memory);
            assert_eq!(calls.attempts as u64, failure.trace.frame_requests);
        }
    }
}

fn at_numeric(owner: &mut OwnedValue) -> Cursor<'_> {
    let mut cursor = Cursor::new(owner, 256, 4096);
    loop {
        if cursor.phase() == Phase::Numeric {
            return cursor;
        }
        let Progress::Pending(next) = cursor.step(&mut budget([1; 4]), || false) else {
            panic!("numeric suspension");
        };
        cursor = next;
    }
}
#[test]
fn composed_numeric_work_retains_atomic_precharge_and_pre_post_cancellation() {
    let mut owner = literal(
        r#"{"type":"boolean","minItems":2,"maxItems":1,"minLength":18446744073709551615}"#,
        Limits::default(),
    );
    let failure = run(&mut owner, &[[1; 4]]).unwrap_err();
    assert_eq!(failure.trace.unsigned_parses, 2);
    assert_eq!(
        failure.cause,
        Cause::Basic(
            validated_thing_schema_kernel_probe::schema_kernel::InlineInvalid {
                ordinal: 0,
                rule: Rule::Ordered(Field::MinItems, Field::MaxItems)
            }
        )
    );

    for cancelled_at in [2, 3] {
        let mut owner = literal(r#"{"type":"string","minimum":1.25}"#, Limits::default());
        let cursor = at_numeric(&mut owner);
        let before = cursor.trace();
        let mut checks = 0;
        let mut work = budget([0, 4, 0, 0]);
        let Progress::Failed(failure) = cursor.step(&mut work, || {
            checks += 1;
            checks == cancelled_at
        }) else {
            panic!("cancel");
        };
        assert_eq!(failure.cause, Cause::Cancelled);
        let parsed = u64::from(cancelled_at == 3);
        assert_eq!(failure.trace.numeric_parses - before.numeric_parses, parsed);
        assert_eq!(failure.trace.work[1] - before.work[1], 4 * parsed);
        assert_eq!(failure.trace.resources.unwrap().temporary_live, 0);
    }
    let mut owner = literal(
        r#"{"type":"string","minimum":1.25,"maximum":1e309}"#,
        Limits::default(),
    );
    let cursor = at_numeric(&mut owner);
    let before = cursor.trace();
    let Progress::Pending(cursor) = cursor.step(&mut budget([0, 3, 0, 0]), || false) else {
        panic!("shortage");
    };
    assert_eq!(cursor.trace(), before);
    let Progress::Pending(cursor) = cursor.step(&mut budget([0, 4, 0, 0]), || false) else {
        panic!("second projection shortage");
    };
    assert_eq!(cursor.trace().numeric_parses, 1);
    // #119 preserves AP's decoded positive exponent spelling: 1e+309.
    let Progress::Failed(failure) = cursor.step(&mut budget([0, 6, 0, 0]), || false) else {
        panic!("failed projection");
    };
    assert_eq!(failure.trace.numeric_parses, 2);
    assert_eq!(
        failure.cause,
        Cause::Basic(
            validated_thing_schema_kernel_probe::schema_kernel::InlineInvalid {
                ordinal: 0,
                rule: Rule::FailedProjection(Field::Maximum)
            }
        )
    );

    let input = r#"{"type":"string","minimum":1.25}"#;
    let mut owner = literal(input, Limits::default());
    let construction_work: u64 = owner.trace().work.into_iter().sum();
    let cursor = at_numeric(&mut owner);
    let before_numeric = spent(cursor.trace());
    drop(cursor);
    let mut owner = literal(
        input,
        Limits {
            lifetime: construction_work + before_numeric + 3,
            ..Limits::default()
        },
    );
    let cursor = at_numeric(&mut owner);
    let before = cursor.trace();
    assert_eq!(cursor.lifetime_remaining(), 3);
    let Progress::Failed(failure) = cursor.step(&mut budget([0, 4, 0, 0]), || false) else {
        panic!("shared lifetime shortage");
    };
    assert_eq!(failure.cause, Cause::Lifetime);
    assert_eq!(failure.trace.numeric_parses, 0);
    assert_eq!(failure.trace.work, before.work);

    // Once Basic fixes an invalidity, rollback must not check a later cancel.
    let mut owner = literal(r#"{"minimum":1e309}"#, Limits::default());
    let cursor = at_numeric(&mut owner);
    let mut checks = 0;
    let Progress::Failed(failure) = cursor.step(&mut budget([0, 6, 0, 0]), || {
        checks += 1;
        checks == 4
    }) else {
        panic!("fixed Basic cause");
    };
    assert!(matches!(failure.cause, Cause::Basic(_)));
    assert_eq!(checks, 3);
}

#[test]
fn every_pending_boundary_preserves_zero_budget_and_bounded_cancellation_or_abandonment() {
    let input = r#"{"type":"array","items":[{}, {"oneOf":[{}]}]}"#;
    let mut owner = literal(input, Limits::default());
    let mut cursor = Cursor::new(&mut owner, 256, 4096);
    let mut pending = 0;
    loop {
        let before = cursor.trace();
        let phase = cursor.phase();
        let lifetime = cursor.lifetime_remaining();
        let Progress::Pending(next) = cursor.step(&mut WorkBudget::new(), || false) else {
            panic!("zero must suspend");
        };
        assert_eq!(next.trace(), before);
        assert_eq!(next.phase(), phase);
        assert_eq!(next.lifetime_remaining(), lifetime);
        match next.step(&mut budget([1; 4]), || false) {
            Progress::Pending(next) => {
                cursor = next;
                pending += 1;
            }
            Progress::Complete(_) => break,
            Progress::Failed(failure) => panic!("{failure:?}"),
        }
    }
    for boundary in 0..=pending {
        for cancel in [false, true] {
            let mut owner = literal(input, Limits::default());
            let source = owner.footprint().retained_requested_bytes;
            let ((), observed) = observe(0, || {
                let mut cursor = Cursor::new(&mut owner, 256, 4096);
                for _ in 0..boundary {
                    let Progress::Pending(next) = cursor.step(&mut budget([1; 4]), || false) else {
                        panic!("boundary");
                    };
                    cursor = next;
                }
                let before_cleanup = OBSERVER.with(|cell| cell.get().unwrap());
                if cancel {
                    let before = cursor.trace();
                    let Progress::Failed(failure) = cursor.step(&mut WorkBudget::new(), || true)
                    else {
                        panic!("cancel");
                    };
                    assert_eq!(failure.cause, Cause::Cancelled);
                    assert_eq!(failure.trace.work, before.work);
                    assert_eq!(failure.trace.resources.unwrap().live, source);
                    assert_eq!(failure.trace.resources.unwrap().temporary_live, 0);
                } else {
                    drop(cursor);
                }
                let after_cleanup = OBSERVER.with(|cell| cell.get().unwrap());
                assert_eq!(after_cleanup.attempts, before_cleanup.attempts);
                assert!(after_cleanup.releases - before_cleanup.releases <= 2);
            });
            assert_eq!(observed.allocations, observed.releases);
            assert_eq!(observed.live, 0);
            assert_eq!(observed.reallocations, 0);
        }
    }
    assert!(pending > 100);
}

#[test]
fn deep_schemas_use_the_frame_arena_and_opaque_values_never_become_schema_work() {
    let depth = 512;
    let input = format!(
        "{}{{}}{}",
        r#"{"type":"array","items":["#.repeat(depth - 1),
        "]}".repeat(depth - 1)
    );
    let mut owner = literal(&input, Limits::default());
    let (trace, observed) = observe(0, || run(&mut owner, &[[1; 4], [4096; 4]]).unwrap());
    assert_eq!(trace.nodes, [depth as u64; 2]);
    assert_eq!(trace.maximum_depth, depth);
    assert_eq!(observed.allocations, observed.releases);
    let failure = drive(Cursor::new(&mut owner, 256, depth - 1), &[[4096; 4]]).unwrap_err();
    assert_eq!(
        failure.cause,
        Cause::Depth {
            observed: depth,
            ceiling: depth - 1
        }
    );
    assert_eq!(failure.trace.resources.unwrap().temporary_live, 0);
    let wrapper = r#"{"$serde_json::private::Number":"1e309","$serde_json::private::RawValue":"[true,false]","minimum":1e309}"#;
    let text = "λ".repeat(24_576);
    for input in [
        format!(r#"{{"const":{wrapper},"default":{wrapper},"opaque":{wrapper}}}"#),
        format!(
            r#"{{"title":{},"const":{wrapper},"default":{wrapper},"opaque":{wrapper}}}"#,
            serde_json::to_string(&text).unwrap()
        ),
    ] {
        let mut owner = literal(&input, Limits::default());
        let trace = run(&mut owner, &[[1; 4]]).unwrap();
        assert_eq!(trace.nodes, [1, 1]);
        assert_eq!(trace.field_parses, [0; 3]);
        assert_eq!(trace.numeric_parses, 0);
    }
    println!(
        "subtree cursor: {} bytes",
        std::mem::size_of::<Cursor<'static>>()
    );
}
