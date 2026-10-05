#![cfg(feature = "validated-thing")]
#![allow(clippy::result_large_err)]
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use validated_thing_schema_kernel_probe::{
    data_schema::DataSchema,
    schema_build::{self as build, ConstructionCause, ConstructionTrace, Pass, Stage},
    schema_fields::Field as F,
    schema_kernel::SchemaKind,
};
use validated_thing_value_construction_probe::{Cause, Cursor as LiteralCursor, Kind, Limits};
#[path = "support/allocator.rs"]
mod allocation;
#[path = "../../validated-thing-value-construction/tests/support/mod.rs"]
#[allow(dead_code)]
mod construction;
use allocation::{OBSERVER, observe};

const CLASSES: [W; 5] = [
    W::DocumentNodes,
    W::CodecInputBytes,
    W::CodecOutputBytes,
    W::JsonSchemaNodes,
    W::CleanupItems,
];
fn budget(n: u64) -> WorkBudget {
    CLASSES
        .into_iter()
        .fold(WorkBudget::new(), |budget, class| {
            budget.with_remaining(class, n)
        })
}
fn schedule(n: u64) -> impl FnMut(Stage) -> (WorkBudget, bool) {
    let mut polls = 0;
    move |_| {
        polls += 1;
        assert!(polls < 1_000_000);
        (budget(if polls % 17 == 0 { n.max(256) } else { n }), false)
    }
}
fn total(trace: ConstructionTrace) -> u64 {
    trace.literal.work.into_iter().sum::<u64>()
        + trace.basic.work.into_iter().sum::<u64>()
        + trace.canonical.work.into_iter().sum::<u64>()
        + trace.seal.into_iter().sum::<u64>()
}
#[path = "support/canonical_fields.rs"]
mod canonical_fields;
use canonical_fields::fields;

const RICH: &str = r#"{"type":"array","@type":"tag","title":"root","titles":{"zh":"根","en":"root"},"description":"desc","descriptions":{},"const":null,"default":{"z":[1e309,"[true,false]"],"a":true},"unit":"x","enum":[null,{},1e309],"readOnly":"false","writeOnly":0,"format":"custom","minimum":1.25,"maximum":4,"minItems":1,"maxItems":3,"oneOf":[{"type":"boolean","const":true}],"items":[{"type":"string","minLength":0,"maxLength":8,"pattern":"λ+","contentEncoding":"base64","contentMediaType":"text/plain"},{"type":"number","minimum":1e309,"maximum":1e309},{"type":"object","properties":{"z":{"type":"null"},"a":{"type":"integer","minimum":-9223372036854775808,"exclusiveMinimum":-9,"maximum":9223372036854775807,"exclusiveMaximum":9,"multipleOf":2}},"required":[]}],"ex:opaque":{"nested":{"minimum":1e309}}}"#;

#[test]
fn strict_to_typed_schema_equivalence_seal_and_owned_handoff_are_one_executable_path() {
    for input in [
        "{}",
        r#"{"type":"array","items":{},"@type":[],"title":null}"#,
        r#"{"type":"array","items":null}"#,
        r#"{"type":"object","properties":{},"required":[]}"#,
        r#"{"const":null,"default":null,"oneOf":[],"enum":[]}"#,
        r#"{"type":"number","minimum":-0.0,"exclusiveMinimum":-0.0,"maximum":9007199254740993,"exclusiveMaximum":9007199254740994,"multipleOf":0.5}"#,
        RICH,
    ] {
        let typed: DataSchema = serde_json::from_str(input).unwrap();
        let mut footprint = None;
        for step in [1, 7, 4096] {
            let (schema, trace) =
                build::from_json(input.as_bytes(), Limits::default(), schedule(step)).unwrap();
            fields(schema.view(), &typed);
            assert!(
                validated_thing_schema_kernel_probe::schema_kernel::validate(
                    &build::Access,
                    schema.view(),
                    &validated_thing_schema_kernel_probe::schema_kernel::InlineSink
                )
                .is_ok()
            );
            assert_eq!(trace.canonical.schemas[0], trace.canonical.schemas[1]);
            assert_eq!(trace.canonical.schemas[0], trace.basic.nodes[0]);
            let arenas = schema.arenas_for_fixture();
            assert_eq!(trace.canonical.compared_nodes, arenas.nodes().len() as u64);
            assert_eq!(trace.canonical.compared_edges, arenas.edges().len() as u64);
            assert_eq!(trace.canonical.compared_bytes, arenas.bytes().len() as u64);
            let source = (std::mem::size_of_val(arenas.nodes())
                + std::mem::size_of_val(arenas.edges())
                + arenas.bytes().len()) as u64;
            assert_eq!(source, schema.footprint().retained_requested_bytes);
            assert_eq!(
                schema.lifetime_remaining(),
                Limits::default().lifetime - total(trace)
            );
            assert_eq!(*footprint.get_or_insert(source), source);
        }
    }
    // Neither original input nor a typed oracle is part of the returned owner.
    let schema = {
        let input = RICH.to_owned();
        build::from_json(input.as_bytes(), Limits::default(), schedule(7))
            .unwrap()
            .0
    };
    assert_eq!(schema.view().field(F::Title).unwrap().text(), Some("root"));
}

#[test]
fn malformed_children_basic_failure_and_cancellation_release_the_whole_source() {
    for input in [
        r#"{"oneOf":[{"unit":null}]}"#,
        r#"{"minimum":1e309}"#,
        r#"{"type":"array","items":[{"readOnly":true,"writeOnly":true}]}"#,
    ] {
        let failure = build::from_json(input.as_bytes(), Limits::default(), schedule(1))
            .err()
            .unwrap();
        assert!(matches!(failure.cause, ConstructionCause::Basic(_)));
        assert_eq!(failure.live_after_rollback, 0);
        assert_eq!(failure.allocations, failure.releases);
    }
    for phase in 0..5 {
        let mut work = schedule(1);
        let failure = build::from_json(RICH.as_bytes(), Limits::default(), |stage| {
            let cancel = match stage {
                Stage::Literal(_) => phase == 0,
                Stage::Basic(_) => phase == 1,
                Stage::Canonical(Pass::Construction) => phase == 2,
                Stage::Canonical(Pass::Equivalence) => phase == 3,
                Stage::Seal => phase == 4,
                Stage::ThingBasic(_) => unreachable!("schema-only construction"),
            };
            (work(stage).0, cancel)
        })
        .err()
        .unwrap();
        assert_eq!(failure.live_after_rollback, 0);
        assert_eq!(failure.allocations, failure.releases);
        assert!(matches!(
            failure.cause,
            ConstructionCause::Literal(Cause::Cancelled)
                | ConstructionCause::Basic(
                    validated_thing_schema_kernel_probe::schema_tree::Cause::Cancelled
                )
                | ConstructionCause::Canonical(build::Cause::Resource(Cause::Cancelled))
                | ConstructionCause::Seal(Cause::Cancelled)
        ));
    }
}

#[test]
fn equivalence_is_paid_can_falsify_output_and_has_zero_budget_no_progress() {
    let mut owner = construction::drive(
        LiteralCursor::from_json(RICH.as_bytes(), Limits::default()),
        1,
    )
    .unwrap();
    let mut cursor = build::Cursor::new(&mut owner, Limits::default());
    let mut work = schedule(1);
    loop {
        let before = cursor.trace();
        let lifetime = cursor.lifetime_remaining();
        let build::Progress::Pending(next) = cursor.step(&mut WorkBudget::new(), || false) else {
            panic!("zero");
        };
        assert_eq!(before, next.trace());
        assert_eq!(lifetime, next.lifetime_remaining());
        if next.pass() == Pass::Equivalence {
            cursor = next;
            break;
        }
        let build::Progress::Pending(next) =
            next.step(&mut work(Stage::Canonical(Pass::Construction)).0, || false)
        else {
            panic!("construction must enter paid equivalence");
        };
        cursor = next;
    }
    cursor.corrupt_root_for_test();
    let failure = loop {
        match cursor.step(&mut work(Stage::Canonical(Pass::Equivalence)).0, || false) {
            build::Progress::Pending(next) => cursor = next,
            build::Progress::Failed(failure) => break failure,
            build::Progress::Complete(_) => panic!("corruption"),
        }
    };
    assert_eq!(failure.cause, build::Cause::SemanticMismatch);
    assert_eq!(failure.pass, Pass::Equivalence);
    owner.rollback_for_fixture();
    assert_eq!(owner.allocations(), owner.releases());
}

#[test]
fn all_stages_consume_one_lifetime_and_exact_exhaustion_rolls_back() {
    let (_, baseline) = build::from_json(RICH.as_bytes(), Limits::default(), schedule(7)).unwrap();
    let required = total(baseline);
    for lifetime in [required - 1, required] {
        let result = build::from_json(
            RICH.as_bytes(),
            Limits {
                lifetime,
                ..Limits::default()
            },
            schedule(7),
        );
        if lifetime == required {
            assert_eq!(result.unwrap().0.lifetime_remaining(), 0);
        } else {
            let failure = result.err().unwrap();
            assert_eq!(failure.cause, ConstructionCause::Seal(Cause::Lifetime));
            assert_eq!(failure.allocations, failure.releases);
            assert_eq!(failure.live_after_rollback, 0);
        }
    }
}

#[test]
fn blocked_canonical_byte_work_preserves_every_credit_and_lifetime() {
    let mut owner = construction::drive(
        LiteralCursor::from_json(br#"{"const":"hello"}"#, Limits::default()),
        4096,
    )
    .unwrap();
    let mut cursor = build::Cursor::new(&mut owner, Limits::default());
    for pass in [Pass::Construction, Pass::Equivalence] {
        while cursor.pass() != pass {
            let build::Progress::Pending(next) = cursor.step(&mut budget(1), || false) else {
                panic!("construction must enter equivalence");
            };
            cursor = next;
        }
        // Finish the bounded structural/allocation prefix without permitting a
        // text byte to be emitted or compared. Later polls have no payable work.
        for _ in 0..32 {
            let build::Progress::Pending(next) = cursor.step(
                &mut budget(4096).with_remaining(W::CodecOutputBytes, 0),
                || false,
            ) else {
                panic!("text bytes are blocked");
            };
            cursor = next;
        }
        assert_eq!(cursor.pass(), pass);
        let trace = cursor.trace();
        if pass == Pass::Construction {
            assert_eq!(trace.work[2], 0);
        } else {
            assert_eq!(trace.compared_bytes, 0);
        }
        let lifetime = cursor.lifetime_remaining();
        for blocked in [W::CodecInputBytes, W::CodecOutputBytes] {
            for _ in 0..4 {
                let mut credit = budget(1).with_remaining(blocked, 0);
                let before = CLASSES.map(|class| credit.remaining(class));
                let build::Progress::Pending(next) = cursor.step(&mut credit, || false) else {
                    panic!("one required byte credit is missing");
                };
                assert_eq!(trace, next.trace());
                assert_eq!(lifetime, next.lifetime_remaining());
                assert_eq!(before, CLASSES.map(|class| credit.remaining(class)));
                cursor = next;
            }
        }
    }
}

#[test]
fn partial_credit_polls_preserve_the_exact_transaction_lifetime() {
    let input = br#"{"const":"hello"}"#;
    let (baseline, trace) =
        build::from_json(input, Limits::default(), |_| (budget(4096), false)).unwrap();
    let required = total(trace);
    assert_eq!(trace.canonical.compared_bytes, 5);
    assert!(trace.canonical.work[2] > trace.canonical.compared_bytes);
    for blocked in CLASSES {
        let mut polls = 0;
        let (schema, actual) = build::from_json(
            input,
            Limits {
                lifetime: required,
                ..Limits::default()
            },
            |stage| {
                let mut credit = budget(4096);
                if matches!(stage, Stage::Canonical(_)) {
                    polls += 1;
                    assert!(polls < 4096);
                    if polls % 5 != 0 {
                        credit = credit.with_remaining(blocked, 0);
                    }
                }
                (credit, false)
            },
        )
        .unwrap_or_else(|failure| panic!("blocked {blocked:?}: {failure:?}"));
        assert_eq!(trace, actual, "blocked {blocked:?}");
        assert_eq!(schema.lifetime_remaining(), 0);
        assert_eq!(schema.footprint(), baseline.footprint());
        assert_eq!(schema.view().field(F::Const).unwrap().text(), Some("hello"));
    }
}

#[test]
fn terminal_diagnostics_use_the_failure_phase_after_step_advances() {
    use validated_thing_schema_kernel_probe::schema_tree::{
        Pass as BasicPass, Phase as BasicPhase,
    };
    use validated_thing_value_construction_probe::Phase as LiteralPhase;
    for step in [1, 4096] {
        let failure = build::from_json(
            b"{}",
            Limits {
                source: 0,
                ..Limits::default()
            },
            |_| (budget(step), false),
        )
        .err()
        .unwrap();
        assert_eq!(failure.cause, ConstructionCause::Literal(Cause::Memory));
        assert_eq!(failure.stage, Stage::Literal(LiteralPhase::Seal));
        let failure = build::from_json(
            br#"{"readOnly":true,"writeOnly":true}"#,
            Limits::default(),
            |_| (budget(step), false),
        )
        .err()
        .unwrap();
        assert_eq!(
            failure.stage,
            Stage::Basic(BasicPhase::Walk(BasicPass::Basic))
        );
        assert_eq!(failure.live_after_rollback, 0);
        assert_eq!(failure.allocations, failure.releases);
    }

    let mut owner =
        construction::drive(LiteralCursor::from_json(b"{}", Limits::default()), 4096).unwrap();
    let construction_work = {
        let mut cursor = build::Cursor::new(&mut owner, Limits::default());
        loop {
            let build::Progress::Pending(next) = cursor.step(&mut budget(1), || false) else {
                panic!("construction must enter equivalence");
            };
            if next.pass() == Pass::Equivalence {
                assert_eq!(next.trace().schemas[1], 0);
                break next.trace().work.into_iter().sum::<u64>();
            }
            cursor = next;
        }
    };
    let (_, baseline) =
        build::from_json(b"{}", Limits::default(), |_| (budget(4096), false)).unwrap();
    let lifetime = baseline.literal.work.into_iter().sum::<u64>()
        + baseline.basic.work.into_iter().sum::<u64>()
        + construction_work;
    for step in [1, 4096] {
        let failure = build::from_json(
            b"{}",
            Limits {
                lifetime,
                ..Limits::default()
            },
            |_| (budget(step), false),
        )
        .err()
        .unwrap();
        assert_eq!(
            failure.cause,
            ConstructionCause::Canonical(build::Cause::Resource(Cause::Lifetime))
        );
        assert_eq!(failure.stage, Stage::Canonical(Pass::Equivalence));
        assert_eq!(failure.live_after_rollback, 0);
        assert_eq!(failure.allocations, failure.releases);
    }
}

#[test]
fn nonrecursive_canonical_emission_preserves_deep_and_opaque_graphs() {
    let depth = 256;
    let input = format!(
        "{}{{}}{}",
        r#"{"type":"array","items":["#.repeat(depth - 1),
        "]}".repeat(depth - 1)
    );
    let (schema, trace) = build::from_json(input.as_bytes(), Limits::default(), |_| {
        (budget(4096), false)
    })
    .unwrap();
    assert_eq!(trace.canonical.schemas, [depth as u64; 2]);
    let mut view = schema.view();
    for _ in 1..depth {
        view = view.field(F::Items).unwrap().child(0).unwrap();
    }
    assert_eq!(view.schema_kind(), Some(SchemaKind::Object));
    let input = r#"{"const":{"$serde_json::private::Number":"1e309","$serde_json::private::RawValue":"[true,false]"},"default":1e309,"items":{"unit":null}}"#;
    let schema = build::from_json(input.as_bytes(), Limits::default(), schedule(1))
        .unwrap()
        .0;
    assert_eq!(
        schema.view().field(F::Const).unwrap().literal_kind(),
        Some(Kind::Object)
    );
    assert_eq!(
        schema.view().field(F::Default).unwrap().text(),
        Some("1e+309")
    );
    assert_eq!(schema.view().extras().member(0).unwrap().0, "items");
}

#[test]
fn every_request_in_the_complete_transaction_has_real_peak_and_terminal_release_evidence() {
    let ((report, trace, retained), observed) = observe(0, || {
        let (schema, trace) =
            build::from_json(RICH.as_bytes(), Limits::default(), schedule(7)).unwrap();
        let report = schema.footprint();
        let retained = OBSERVER.with(|cell| cell.get().unwrap().live);
        drop(schema);
        (report, trace, retained)
    });
    assert_eq!(observed.live, 0);
    assert_eq!(observed.allocations, observed.releases);
    assert_eq!(report.retained_requested_bytes, retained as u64);
    assert_eq!(report.conversion_peak_bytes, observed.peak as u64);
    assert_eq!(report.reservation_peak_bytes, report.conversion_peak_bytes);
    assert_eq!(report.largest_request_bytes, observed.largest as u64);
    assert_eq!(observed.reallocations, 0);
    let cleanup = |trace: ConstructionTrace| {
        trace.literal.work[3] + trace.basic.work[3] + trace.canonical.work[4] + trace.seal[3]
    };
    assert_eq!(cleanup(trace), observed.attempts as u64);
    for request in 1..=observed.attempts {
        let (result, calls) = observe(request, || {
            build::from_json(RICH.as_bytes(), Limits::default(), schedule(7))
        });
        let failure = result.err().unwrap();
        assert!(matches!(
            failure.cause,
            ConstructionCause::Literal(Cause::Allocation)
                | ConstructionCause::Basic(
                    validated_thing_schema_kernel_probe::schema_tree::Cause::Allocation
                )
                | ConstructionCause::Canonical(build::Cause::Resource(Cause::Allocation))
                | ConstructionCause::Seal(Cause::Allocation)
        ));
        assert_eq!(calls.live, 0);
        assert_eq!(calls.allocations, calls.releases);
        assert_eq!(failure.allocations, failure.releases);
        assert_eq!(failure.resources.conversion_peak_bytes, calls.peak as u64);
        assert_eq!(
            failure.resources.largest_request_bytes,
            calls.largest as u64
        );
        assert!(
            failure.resources.reservation_peak_bytes >= failure.resources.conversion_peak_bytes
        );
        assert_eq!(cleanup(failure.trace), calls.attempts as u64);
    }
    println!(
        "complete schema path: retained={}, physical_peak={}, largest_request={}, requests={}, work={}, builder_inline={}",
        report.retained_requested_bytes,
        report.conversion_peak_bytes,
        report.largest_request_bytes,
        observed.attempts,
        total(trace),
        std::mem::size_of::<build::Cursor<'static>>()
    );
}

#[test]
fn canonical_boundaries_are_checked_with_literal_and_output_resident_together() {
    let ((report, _), _) = observe(0, || {
        let (schema, trace) =
            build::from_json(RICH.as_bytes(), Limits::default(), schedule(7)).unwrap();
        let report = schema.footprint();
        drop(schema);
        (report, trace)
    });
    for axis in 0..3 {
        for delta in [-1, 0, 1] {
            let mut limits = Limits::default();
            match axis {
                0 => {
                    limits.peak = report
                        .conversion_peak_bytes
                        .checked_add_signed(delta)
                        .unwrap()
                }
                1 => {
                    limits.contiguous = report
                        .largest_request_bytes
                        .checked_add_signed(delta)
                        .unwrap()
                }
                2 => {
                    limits.temporary = report
                        .temporary_peak_bytes
                        .checked_add_signed(delta)
                        .unwrap()
                }
                _ => unreachable!(),
            }
            let (result, observed) =
                observe(0, || build::from_json(RICH.as_bytes(), limits, schedule(7)));
            assert_eq!(result.is_ok(), delta >= 0);
            if let Err(failure) = result {
                assert!(matches!(
                    failure.cause,
                    ConstructionCause::Literal(Cause::Memory)
                        | ConstructionCause::Basic(
                            validated_thing_schema_kernel_probe::schema_tree::Cause::Memory
                        )
                        | ConstructionCause::Canonical(build::Cause::Resource(Cause::Memory))
                        | ConstructionCause::Seal(Cause::Memory)
                ));
                assert_eq!(failure.allocations, failure.releases);
                assert_eq!(observed.live, 0);
                assert_eq!(
                    failure.trace.literal.work[3]
                        + failure.trace.basic.work[3]
                        + failure.trace.canonical.work[4]
                        + failure.trace.seal[3],
                    observed.attempts as u64
                );
            }
        }
    }
}

#[test]
fn typed_default_equivalence_erases_literal_spelling_and_duplicate_history() {
    for group in [
        &[
            "{}",
            r#"{"title":null,"readOnly":false,"writeOnly":false}"#,
            r#"{"title":"discard","title":null,"readOnly":0,"writeOnly":"false"}"#,
        ][..],
        &[
            r#"{"type":"array"}"#,
            r#"{"type":"array","items":null,"@type":null}"#,
        ][..],
    ] {
        let expected: DataSchema = serde_json::from_str(group[0]).unwrap();
        let mut source_bytes = None;
        for input in group {
            let (schema, _) =
                build::from_json(input.as_bytes(), Limits::default(), schedule(1)).unwrap();
            fields(schema.view(), &expected);
            assert_eq!(
                *source_bytes.get_or_insert(schema.footprint().retained_requested_bytes),
                schema.footprint().retained_requested_bytes
            );
        }
    }
}

#[test]
fn each_canonical_pending_boundary_can_abandon_with_bounded_prepaid_owner_cleanup() {
    let input = r#"{"type":"array","items":[{}, {"oneOf":[{}]}],"const":["λ",true]}"#.as_bytes();
    let boundaries = {
        let mut owner =
            construction::drive(LiteralCursor::from_json(input, Limits::default()), 1).unwrap();
        let mut cursor = build::Cursor::new(&mut owner, Limits::default());
        let mut boundaries = 0;
        loop {
            match cursor.step(&mut budget(1), || false) {
                build::Progress::Pending(next) => {
                    cursor = next;
                    boundaries += 1;
                }
                build::Progress::Complete(_) => break boundaries,
                build::Progress::Failed(failure) => panic!("{failure:?}"),
            }
        }
    };
    for boundary in 0..boundaries {
        let ((), observed) = observe(0, || {
            let mut owner =
                construction::drive(LiteralCursor::from_json(input, Limits::default()), 1).unwrap();
            let mut cursor = build::Cursor::new(&mut owner, Limits::default());
            for _ in 0..boundary {
                let build::Progress::Pending(next) = cursor.step(&mut budget(1), || false) else {
                    panic!("boundary");
                };
                cursor = next;
            }
            let before = OBSERVER.with(|cell| cell.get().unwrap());
            drop(cursor);
            drop(owner);
            let after = OBSERVER.with(|cell| cell.get().unwrap());
            assert_eq!(before.attempts, after.attempts);
            assert!(after.releases - before.releases <= 8);
        });
        assert_eq!(observed.live, 0);
        assert_eq!(observed.allocations, observed.releases);
    }
    assert!(boundaries > 100);
}
