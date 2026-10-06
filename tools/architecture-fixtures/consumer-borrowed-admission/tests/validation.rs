use clinkz_wot_foundation::{WorkBudget, WorkClass};
use consumer_borrowed_admission_probe::td::{
    Cause, Limits, Policy, Progress, Trace, Validated, Validation,
};
use td_candidate as td_crate;
#[path = "../../../../td/tests/support/basic_corpus_shared.rs"]
#[allow(dead_code)]
mod corpus;

fn budget(n: u64) -> WorkBudget {
    let mut b = WorkBudget::new();
    for c in WorkClass::ALL {
        b.set_remaining(c, n);
    }
    b
}
// Keep observations inline: a harness allocation must not look like TD work.
#[allow(clippy::result_large_err)]
fn run(
    thing: &td_crate::thing::Thing,
    limits: Limits,
    quantum: u64,
) -> Result<Validated<'_>, (Cause, Trace)> {
    let mut c = Validation::new(thing, Policy::check(limits).unwrap());
    for _ in 0..200_000 {
        // Moving into and out of a new owner attacks input/iterator self-borrows.
        c = *Box::new(c);
        match c.step(&mut budget(quantum), false) {
            Progress::Pending(next) => c = next,
            Progress::Complete(v) => return Ok(v),
            Progress::Failed(e, t) => return Err((e, t)),
        }
    }
    panic!("no progress with sufficient primitive credit");
}

#[test]
fn complete_basic_corpus_and_first_inline_error_use_the_same_rules() {
    let cases = corpus::cases();
    assert!(cases.len() > 170);
    println!("shared whole-Thing Basic corpus: {} cases", cases.len());
    for case in cases {
        let expected = td_crate::basic_kernel::validate(
            &td_crate::basic_typed::TypedBasicAccess(Some(&case.thing)),
            &td_crate::basic_kernel::InlineSink,
        );
        let actual = run(&case.thing, Limits::default(), 4096);
        match (expected, actual) {
            (Ok(()), Ok(v)) => {
                assert!(case.valid, "{}", case.label);
                assert_eq!(v.trace().live, 0);
            }
            (Err(expected), Err((Cause::Invalid(actual), trace))) => {
                assert_eq!(actual, expected, "{}", case.label);
                assert_eq!(trace.live, 0);
                assert_eq!(trace.allocations, trace.releases);
            }
            (expected, actual) => panic!(
                "{} expected {:?}, actual {:?}",
                case.label,
                expected,
                actual.err()
            ),
        }
    }
}

#[test]
fn chunking_zero_budget_and_moves_do_not_restart_traversal() {
    let thing = corpus::cases().into_iter().find(|c| c.valid).unwrap().thing;
    let a = run(&thing, Limits::default(), 100_000)
        .unwrap_or_else(|e| panic!("{e:?}"))
        .trace();
    let b = run(&thing, Limits::default(), 4096)
        .unwrap_or_else(|e| panic!("{e:?}"))
        .trace();
    assert_eq!(a, b);
    let c = Validation::new(&thing, Policy::check(Limits::default()).unwrap());
    let before = c.trace();
    let remaining = c.lifetime_remaining();
    let Progress::Pending(c) = c.step(&mut budget(0), false) else {
        panic!()
    };
    assert_eq!(c.trace(), before);
    assert_eq!(c.lifetime_remaining(), remaining);
    let Progress::Failed(Cause::Cancelled, t) = c.step(&mut budget(0), true) else {
        panic!()
    };
    assert_eq!(t.live, 0);
}

#[test]
fn lifetime_is_not_refilled_and_structural_limits_visit_opaque_input() {
    let mut thing = corpus::cases().into_iter().find(|c| c.valid).unwrap().thing;
    thing
        ._extra_fields
        .insert("opaque".into(), serde_json::json!([[[[[1]]]]]));
    let e = run(
        &thing,
        Limits {
            lifetime: 100,
            ..Limits::default()
        },
        4096,
    )
    .err()
    .unwrap();
    assert_eq!(e.0, Cause::Lifetime);
    assert_eq!(e.1.live, 0);
    let e = run(
        &thing,
        Limits {
            depth: 5,
            ..Limits::default()
        },
        4096,
    )
    .err()
    .unwrap();
    assert_eq!(e.0, Cause::Depth);
    assert_eq!(e.1.live, 0);
    thing
        ._extra_fields
        .insert("number".into(), serde_json::from_str("1e309").unwrap());
    assert!(run(&thing, Limits::default(), 4096).is_ok());
    assert_eq!(
        run(
            &thing,
            Limits {
                number: 4,
                ..Limits::default()
            },
            4096
        )
        .err()
        .unwrap()
        .0,
        Cause::Number
    );
}

#[test]
fn filtered_combo_fallback_keeps_logical_reference_error_indices() {
    let mut thing = corpus::cases().into_iter().find(|c| c.valid).unwrap().thing;
    let definition = thing.security_definitions.values_mut().next().unwrap();
    let td_crate::security_scheme::SecurityScheme::NoSec(v) = definition else {
        panic!()
    };
    v._context.scheme = "combo".into();
    v._context._extra_fields.insert(
        "oneOf".into(),
        serde_json::json!([false, "none", 17, "missing", null]),
    );
    let expected = td_crate::basic_kernel::validate(
        &td_crate::basic_typed::TypedBasicAccess(Some(&thing)),
        &td_crate::basic_kernel::InlineSink,
    )
    .err()
    .unwrap();
    let (Cause::Invalid(actual), _) = run(&thing, Limits::default(), 4096).err().unwrap() else {
        panic!()
    };
    assert_eq!(actual, expected);
}
