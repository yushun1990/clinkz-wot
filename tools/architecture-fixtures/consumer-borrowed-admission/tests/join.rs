use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingCandidate, BindingConfigurationDigest, BindingGeneration,
    BindingId,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use clinkz_wot_property_read_binding_fixture::MockCompiler;
use consumer_borrowed_admission_probe::{
    planning::{Build, Calls, Cause, Draft, Phase, Registry, Step},
    td::{self, Limits, Policy, Progress, Validated, Validation},
};
use td_candidate::thing::Thing;
#[path = "../../validated-thing-planning-handoff/src/registration.rs"]
mod registration;
mod support;
#[global_allocator]
static ALLOC: support::Allocator = support::Allocator;
fn budget(n: u64) -> WorkBudget {
    let mut b = WorkBudget::new();
    for c in W::ALL {
        b.set_remaining(c, n);
    }
    b
}
fn source() -> Thing {
    serde_json::from_str(r#"{
    "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"borrowed", "id":"urn:test:borrowed",
    "base":"https://example.test/a/b/", "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
    "properties":{"empty":{"type":"null","forms":[{"href":"ignored","op":[]}]},
      "zeta":{"type":"null","forms":[{"href":"unused","op":["writeproperty"]},{"href":"../c/./value?q=1#f","contentCoding":"gzip","scopes":["first","second"]},{"href":"final"}]}},
    "actions":{"unrelated":{"forms":[{"href":"action"}]}}, "events":{"unrelated":{"forms":[{"href":"event"}]}},
    "opaque":{"nested":[{"const":1e309}]}
}"#).unwrap()
}
fn validate(thing: &Thing) -> Validated<'_> {
    let mut c = Validation::new(thing, Policy::check(Limits::default()).unwrap());
    loop {
        match c.step(&mut budget(100_000), false) {
            Progress::Pending(n) => c = n,
            Progress::Complete(v) => return v,
            Progress::Failed(c, t) => panic!("{c:?} {t:?}"),
        }
    }
}
fn candidate() -> BindingCandidate {
    BindingCandidate::new(
        BindingId::new(0),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([3; 32]),
        BindingArtifactCompatibility::new([2; 16]),
        0,
        0,
    )
}
fn complete(
    proof: Validated<'_>,
    compiler: &MockCompiler,
    calls: &Calls,
    fail: Option<usize>,
) -> Result<Draft, Cause> {
    complete_with(proof, compiler, calls, candidate(), fail)
}
fn complete_with(
    proof: Validated<'_>,
    compiler: &MockCompiler,
    calls: &Calls,
    candidate: BindingCandidate,
    fail: Option<usize>,
) -> Result<Draft, Cause> {
    let mut c = Build::new(proof, compiler, calls, candidate, 1_000_000, 100_000, fail)?;
    for _ in 0..100_000 {
        if matches!(
            c.phase(),
            Phase::Preflight | Phase::Materialize | Phase::Bounds
        ) {
            assert_eq!(calls.starts.get(), 0);
        }
        // Move the continuation with ready derived URI bytes, not a retained
        // event borrow. The boxes are harness owners outside allocator scopes.
        match c.step(&mut budget(100_000), false) {
            Step::Pending => {}
            Step::Complete(d) => return Ok(d),
            Step::Failed(e) => return Err(e),
        }
    }
    panic!("stalled");
}
#[test]
fn real_owned_plans_and_mock_artifacts_survive_source_and_registration_destruction() {
    let thing = source();
    let registration = registration::registration();
    assert!(
        registration
            .capabilities()
            .supports_consumer_property_read()
    );
    let identity = registration.identity();
    let candidate = BindingCandidate::new(
        identity.binding_id(),
        identity.binding_generation(),
        identity.configuration(),
        identity.artifact_compatibility(),
        0,
        0,
    );
    let calls = Calls::default();
    let proof = validate(&thing);
    let guard = support::start(None);
    let draft = complete_with(
        proof,
        registration.compiler().compiler(),
        &calls,
        candidate,
        None,
    )
    .unwrap();
    let observed = support::counts();
    let footprint = draft.footprint();
    assert_eq!(observed.live as u64, footprint.requested);
    assert_eq!(observed.peak as u64, footprint.peak);
    assert_eq!(observed.largest as u64, footprint.largest);
    assert_eq!(observed.allocations as u64, footprint.allocations);
    assert_eq!((calls.bounds.get(), calls.starts.get()), (2, 2));
    drop(thing);
    drop(registration);
    assert_eq!(draft.counts(), (2, 2));
    assert!(draft.select("empty", None).is_none());
    let (plan, artifact) = draft.select("zeta", Some(1)).unwrap();
    assert_eq!(
        plan.resolved_target(),
        "https://example.test/a/c/value?q=1#f"
    );
    assert_eq!(
        artifact.artifact().payload().target(),
        Some(plan.resolved_target())
    );
    assert_eq!(
        draft.raw_and_metadata(0),
        ("../c/./value?q=1#f", Some("gzip"), 2)
    );
    fn require_owned<T: 'static>(_: &T) {}
    require_owned(&draft);
    let mut registry = Registry::default();
    let before = support::counts();
    registry.publish(draft, false, true).unwrap();
    assert_eq!(support::counts(), before);
    assert!(registry.draft().unwrap().select("zeta", Some(2)).is_some());
    drop(registry);
    let final_counts = support::counts();
    drop(guard);
    assert_eq!(final_counts.live, 0);
    assert_eq!(final_counts.allocations, final_counts.releases);
    println!(
        "owned join: heap={} peak={} largest={} allocations={} inline_draft={}",
        footprint.requested,
        footprint.peak,
        footprint.largest,
        footprint.allocations,
        footprint.inline
    );
    println!(
        "fixed owners (each contains its nested fields): validation={} read={} build={} result={}",
        core::mem::size_of::<Validation<'static>>(),
        core::mem::size_of::<td::Read<'static>>(),
        core::mem::size_of::<Build<'static, 'static>>(),
        core::mem::size_of::<Step>(),
    );
}
#[test]
fn later_materialization_and_bound_failures_have_zero_starts() {
    let mut thing = source();
    thing.base = Some(td_candidate::data_type::BaseUri::parse("https://{host}/").unwrap());
    let forms = &mut thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms;
    forms[1].href = td_candidate::data_type::FormHref::parse("https://example.test/first").unwrap();
    let compiler = MockCompiler::new(candidate().compatibility());
    let calls = Calls::default();
    assert_eq!(
        complete(validate(&thing), &compiler, &calls, None).err(),
        Some(Cause::Td(td::Cause::Uri))
    );
    assert_eq!(calls.starts.get(), 0);
    let thing = source();
    let calls = Calls::default();
    let guard = support::start(None);
    assert_eq!(
        complete(validate(&thing), &compiler, &calls, Some(1)).err(),
        Some(Cause::LaterBound)
    );
    assert_eq!(calls.bounds.get(), 2);
    assert_eq!(calls.starts.get(), 0);
    assert_eq!(support::counts().live, 0);
    drop(guard);
}
#[test]
fn missing_id_explicit_empty_security_and_unrelated_invalidity_do_not_start_a_compiler() {
    let mut thing = source();
    thing.id = None;
    let compiler = MockCompiler::new(candidate().compatibility());
    let calls = Calls::default();
    assert_eq!(
        complete(validate(&thing), &compiler, &calls, None).err(),
        Some(Cause::Td(td::Cause::MissingId))
    );
    let mut thing = source();
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[2]
        .security = Some(vec![]);
    assert_eq!(
        complete(validate(&thing), &compiler, &calls, None).err(),
        Some(Cause::Td(td::Cause::Security))
    );
    assert_eq!(calls.bounds.get(), 0);
    let mut thing = source();
    thing
        .actions
        .as_mut()
        .unwrap()
        .get_mut("unrelated")
        .unwrap()
        ._interaction
        .forms[0]
        .op = Some(vec![td_candidate::data_type::Operation::ReadProperty]);
    let mut c = Validation::new(&thing, Policy::check(Limits::default()).unwrap());
    loop {
        match c.step(&mut budget(100_000), false) {
            Progress::Pending(n) => c = n,
            Progress::Failed(td::Cause::Invalid(_), t) => {
                assert_eq!(t.live, 0);
                break;
            }
            _ => panic!("unrelated Basic was skipped"),
        }
    }
    assert_eq!(calls.starts.get(), 0);
}
#[test]
fn scope_sizing_waits_for_the_complete_semantic_debit() {
    let mut thing = source();
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[1]
        .scopes = Some((0..16).map(|n| "é".repeat(n)).collect());
    let mut read = td::Read::new(validate(&thing));
    let mut no_uri = budget(100_000);
    no_uri.set_remaining(W::UriBytes, 0);
    loop {
        match read.step(&mut no_uri, false).unwrap() {
            td::Event::Property { .. } => read.acknowledge(),
            td::Event::Pending => break,
            _ => panic!("URI projection must suspend"),
        }
    }
    let before = read.trace();
    let remaining = read.lifetime_remaining();
    assert_eq!(read.scope_visits(), 0);
    assert_eq!(read.uri_validation_bytes(), 0);
    // One projection action plus sixteen scope-length visits must be paid
    // together. Short calls cannot accumulate credit or start either scan.
    for nodes in [0, 16, 16] {
        let mut short = budget(100_000);
        short.set_remaining(W::DocumentNodes, nodes);
        assert!(matches!(
            read.step(&mut short, false).unwrap(),
            td::Event::Pending
        ));
        assert_eq!(read.scope_visits(), 0);
        assert_eq!(read.uri_validation_bytes(), 0);
        assert_eq!(read.trace(), before);
        assert_eq!(read.lifetime_remaining(), remaining);
        assert_eq!(short.remaining(W::DocumentNodes), nodes);
        assert_eq!(short.remaining(W::UriBytes), 100_000);
    }
    let mut exact = budget(100_000);
    exact.set_remaining(W::DocumentNodes, 17);
    let td::Event::Form(f) = read.step(&mut exact, false).unwrap() else {
        panic!("a complete debit should produce the Form");
    };
    assert_eq!(f.scopes.len(), 16);
    assert_eq!(f.scopes.byte_len(), 240);
    assert_eq!(exact.remaining(W::DocumentNodes), 0);
    assert_eq!(read.scope_visits(), 16);
    assert!(read.uri_validation_bytes() > 0);
}

#[test]
fn ready_uri_lending_does_not_repeat_utf8_validation() {
    let mut thing = source();
    let path = "x".repeat(180);
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[1]
        .href = td_candidate::data_type::FormHref::parse(&format!("../c/./{path}?q=1#f")).unwrap();
    let expected = format!("https://example.test/a/c/{path}?q=1#f");
    let mut read = td::Read::new(validate(&thing));
    loop {
        match read.step(&mut budget(2_000_000), false).unwrap() {
            td::Event::Property { .. } => read.acknowledge(),
            td::Event::Form(f) => {
                assert_eq!(f.original, 1);
                assert_eq!(f.resolved, expected);
                break;
            }
            _ => panic!("expected the first readable Form"),
        }
    }
    let scanned = read.uri_validation_bytes();
    assert_eq!(scanned, expected.len() as u64);
    let before = read.trace();
    let remaining = read.lifetime_remaining();
    for _ in 0..128 {
        read = *Box::new(read);
        let td::Event::Form(f) = read.step(&mut WorkBudget::new(), false).unwrap() else {
            panic!("ready Form was lost");
        };
        assert_eq!(f.resolved, expected);
        assert_eq!(read.uri_validation_bytes(), scanned);
        assert_eq!(read.trace(), before);
        assert_eq!(read.lifetime_remaining(), remaining);
    }
}

#[test]
fn ready_derived_event_moves_and_insufficient_copy_credit_do_not_rescan() {
    let mut thing = source();
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[1]
        .scopes = Some((0..16).map(|n| "scope".repeat(n)).collect());
    let compiler = MockCompiler::new(candidate().compatibility());
    let calls = Calls::default();
    let mut c = Build::new(
        validate(&thing),
        &compiler,
        &calls,
        candidate(),
        1_000_000,
        100_000,
        None,
    )
    .unwrap();
    while c.phase() == Phase::Preflight {
        assert!(matches!(c.step(&mut budget(100_000), false), Step::Pending));
    }
    let mut small = budget(100_000);
    small.set_remaining(W::PlanningItems, 1);
    assert!(matches!(c.step(&mut small, false), Step::Pending));
    // Property event has been projected but cannot yet be copied.
    let before = c.trace();
    let remaining = c.remaining();
    c = *Box::new(c);
    assert!(matches!(
        c.step(&mut WorkBudget::new(), false),
        Step::Pending
    ));
    assert_eq!(c.trace(), before);
    assert_eq!(c.remaining(), remaining);
    // Copy each ready Property, then stop at the first derived Form with copy
    // work withheld. Observe the scans themselves, including Planning access.
    for _ in 0..8 {
        assert!(matches!(c.step(&mut budget(100_000), false), Step::Pending));
        let mut small = budget(100_000);
        small.set_remaining(W::PlanningItems, 1);
        assert!(matches!(c.step(&mut small, false), Step::Pending));
        if c.uri_validation_bytes() > 0 {
            break;
        }
    }
    assert!(c.uri_validation_bytes() > 0, "never reached a derived Form");
    assert_eq!(c.phase(), Phase::Materialize);
    let before = c.trace();
    let remaining = c.remaining();
    let scanned = c.uri_validation_bytes();
    let visited = c.scope_visits();
    assert_eq!(visited, 16);
    for _ in 0..32 {
        for (planning, cleanup) in [(0, 0), (1, 100_000), (64, 100_000), (100_000, 22)] {
            c = *Box::new(c);
            let mut short = budget(100_000);
            short.set_remaining(W::PlanningItems, planning);
            short.set_remaining(W::CleanupItems, cleanup);
            assert!(matches!(c.step(&mut short, false), Step::Pending));
            assert_eq!(c.scope_visits(), visited);
            assert_eq!(c.uri_validation_bytes(), scanned);
            assert_eq!(c.trace(), before);
            assert_eq!(c.remaining(), remaining);
            assert_eq!(short.remaining(W::PlanningItems), planning);
            assert_eq!(short.remaining(W::CleanupItems), cleanup);
        }
    }
    assert!(matches!(c.step(&mut budget(100_000), false), Step::Pending));
    assert_eq!(
        c.scope_visits(),
        visited + 16,
        "copy visits each scope once"
    );
    let draft = loop {
        match c.step(&mut budget(100_000), false) {
            Step::Pending => {}
            Step::Complete(d) => break d,
            Step::Failed(e) => panic!("{e:?}"),
        }
    };
    assert_eq!(
        draft.select("zeta", Some(1)).unwrap().0.resolved_target(),
        "https://example.test/a/c/value?q=1#f"
    );
}

#[test]
fn frame_capacity_growth_precedence_and_cancellation_match_actual_requests() {
    let mut thing = source();
    let mut value = serde_json::json!(0);
    for _ in 0..24 {
        value = serde_json::json!([value]);
    }
    thing._extra_fields.insert("deep".into(), value);
    let guard = support::start(None);
    let proof = validate(&thing);
    let trace = proof.trace();
    let actual = support::counts();
    drop(guard);
    assert_eq!(actual.live, 0);
    assert_eq!(actual.allocations as u64, trace.allocations);
    assert_eq!(actual.releases as u64, trace.releases);
    assert_eq!(actual.peak as u64, trace.peak);
    assert_eq!(actual.largest as u64, trace.largest);
    assert!(trace.frame_moves > 0);
    for fail in 1..=actual.allocations {
        let guard = support::start(Some(fail));
        let mut c = Validation::new(&thing, Policy::check(Limits::default()).unwrap());
        loop {
            match c.step(&mut budget(100_000), false) {
                Progress::Pending(n) => c = n,
                Progress::Failed(td::Cause::Memory, t) => {
                    assert_eq!(t.live, 0);
                    break;
                }
                _ => panic!("allocation failure was lost"),
            }
        }
        let count = support::counts();
        drop(guard);
        assert_eq!(count.live, 0);
        assert_eq!(count.allocations, count.releases);
    }
    let guard = support::start(None);
    let c = Validation::new(
        &thing,
        Policy::check(Limits {
            request: 1,
            ..Limits::default()
        })
        .unwrap(),
    );
    assert!(matches!(
        c.step(&mut budget(100_000), false),
        Progress::Failed(td::Cause::Memory, _)
    ));
    assert_eq!(support::counts().attempts, 0);
    drop(guard);
    for cancel in [false, true] {
        let guard = support::start(None);
        let mut c = Validation::new(&thing, Policy::check(Limits::default()).unwrap());
        for _ in 0..100 {
            let mut b = budget(100_000);
            b.set_remaining(W::CleanupItems, 1);
            let Progress::Pending(n) = c.step(&mut b, false) else {
                panic!()
            };
            c = n;
            if c.frame_request_pending() && c.trace().allocations >= 1 {
                break;
            }
        }
        assert!(c.frame_request_pending());
        let mut b = budget(0);
        b.set_remaining(W::CleanupItems, 1);
        let Progress::Pending(n) = c.step(&mut b, false) else {
            panic!()
        };
        c = n;
        assert!(c.transferring());
        if cancel {
            assert!(matches!(
                c.step(&mut WorkBudget::new(), true),
                Progress::Failed(td::Cause::Cancelled, _)
            ));
        } else {
            drop(c);
        }
        let count = support::counts();
        drop(guard);
        assert_eq!(count.live, 0);
        assert_eq!(count.allocations, count.releases);
    }
    println!(
        "deep borrowed traversal: allocations={} peak={} largest={} moved_frames={}",
        actual.allocations, actual.peak, actual.largest, trace.frame_moves
    );
}

#[test]
fn long_common_prefixes_pay_actual_bytes_and_do_not_depend_on_poll_chunking() {
    let mut thing = source();
    let definition = thing.security_definitions.values().next().unwrap().clone();
    thing.security_definitions.clear();
    let prefix = "p".repeat(96);
    for n in 0..32 {
        thing
            .security_definitions
            .insert(format!("{prefix}{n:04}"), definition.clone());
    }
    let target = format!("{prefix}{:04}", 31);
    thing.security = vec![target.clone()];
    let expected: usize = thing
        .security_definitions
        .keys()
        .map(|key| {
            key.bytes()
                .zip(target.bytes())
                .position(|(a, b)| a != b)
                .map_or(key.len(), |n| n + 1)
        })
        .sum();
    let a = validate(&thing).trace();
    assert_eq!(a.compared_bytes as usize, expected);
    let mut c = Validation::new(&thing, Policy::check(Limits::default()).unwrap());
    let b = loop {
        c = *Box::new(c);
        match c.step(&mut budget(4096), false) {
            Progress::Pending(n) => c = n,
            Progress::Complete(v) => break v.trace(),
            Progress::Failed(e, t) => panic!("{e:?} {t:?}"),
        }
    };
    assert_eq!(a, b);
    assert!(expected > 3000);
    println!(
        "security reference: actual compared bytes={expected}, iterator advances={}",
        a.next_calls
    );
}

#[test]
fn no_projection_or_partial_credit_before_complete_numeric_debit() {
    let mut thing = source();
    let lexeme = format!("0.{}1", "0".repeat(197));
    assert_eq!(lexeme.len(), 200);
    let schema = &mut thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._schema;
    let td_candidate::data_schema::DataSchema::Null(schema) = schema else {
        panic!()
    };
    schema
        ._context
        ._extra_fields
        .insert("minimum".into(), serde_json::from_str(&lexeme).unwrap());
    let mut c = Validation::new(&thing, Policy::check(Limits::default()).unwrap());
    loop {
        if c.numeric_charge() == Some(200) {
            break;
        }
        match c.step(&mut budget(400), false) {
            Progress::Pending(n) => c = n,
            _ => panic!("missed numeric suspension"),
        }
    }
    let before = c.trace();
    let left = c.lifetime_remaining();
    let mut short = budget(100_000);
    short.set_remaining(W::CodecInputBytes, 199);
    let Progress::Pending(next) = c.step(&mut short, false) else {
        panic!()
    };
    c = next;
    assert_eq!(c.trace(), before);
    assert_eq!(c.lifetime_remaining(), left);
    assert_eq!(short.remaining(W::CodecInputBytes), 199);
    let mut exact = budget(100_000);
    exact.set_remaining(W::CodecInputBytes, 200);
    match c.step(&mut exact, false) {
        Progress::Pending(n) => assert_eq!(n.trace().projections, before.projections + 1),
        Progress::Complete(v) => assert_eq!(v.trace().projections, before.projections + 1),
        Progress::Failed(e, t) => panic!("{e:?} {t:?}"),
    }
}

#[test]
fn cancellation_abandonment_and_publication_rejection_release_owned_output() {
    let thing = source();
    let compiler = MockCompiler::new(candidate().compatibility());
    for terminal in 0..3 {
        let calls = Calls::default();
        let guard = support::start(None);
        let mut c = Build::new(
            validate(&thing),
            &compiler,
            &calls,
            candidate(),
            1_000_000,
            100_000,
            None,
        )
        .unwrap();
        while c.phase() != Phase::Compile {
            assert!(matches!(c.step(&mut budget(100_000), false), Step::Pending));
        }
        assert_eq!(calls.starts.get(), 0);
        assert!(support::counts().live > 0);
        match terminal {
            0 => {
                assert!(matches!(
                    c.step(&mut WorkBudget::new(), true),
                    Step::Failed(Cause::Cancelled)
                ));
                assert!(matches!(
                    c.step(&mut WorkBudget::new(), false),
                    Step::Failed(Cause::Cancelled)
                ));
            }
            1 => drop(c),
            _ => {
                let draft = loop {
                    match c.step(&mut budget(100_000), false) {
                        Step::Pending => {}
                        Step::Complete(d) => break d,
                        _ => panic!(),
                    }
                };
                let mut registry = Registry::default();
                assert_eq!(registry.publish(draft, false, false), Err(Cause::Identity));
                assert!(registry.draft().is_none());
            }
        }
        let observed = support::counts();
        drop(guard);
        assert_eq!(observed.live, 0);
        assert_eq!(observed.allocations, observed.releases);
    }
}

#[test]
fn every_checked_materialization_allocation_failure_rolls_back_before_bounds_or_start() {
    let thing = source();
    let compiler = MockCompiler::new(candidate().compatibility());
    // The measured successful witness has fifteen logical/lookup/metadata
    // requests followed by two existing mock artifact requests. The mock's
    // infallible Box allocation is explicitly outside this fault-injection proof.
    for fail in 1..=15 {
        let calls = Calls::default();
        let proof = validate(&thing);
        let guard = support::start(Some(fail));
        let result = complete(proof, &compiler, &calls, None);
        let count = support::counts();
        drop(guard);
        assert_eq!(result.err(), Some(Cause::Allocation));
        assert_eq!(count.attempts, fail);
        assert_eq!(calls.bounds.get(), 0);
        assert_eq!(calls.starts.get(), 0);
        assert_eq!(count.live, 0);
        assert_eq!(count.allocations, count.releases);
    }
}

#[test]
fn serializer_failure_and_unused_content_do_not_force_a_copy_or_change_runtime_output() {
    let mut thing = source();
    thing.context = serde_json::from_str(r#""https://example.test/private-context""#).unwrap();
    assert!(serde_json::to_string(&thing).is_err());
    let plain = validate(&thing).trace();
    thing._metadata.description = Some("x".repeat(4096));
    let rich = validate(&thing).trace();
    assert_eq!(rich.content, plain.content + 4096);
    assert_eq!(rich.allocations, plain.allocations);
    assert_eq!(rich.peak, plain.peak);
    let compiler = MockCompiler::new(candidate().compatibility());
    let calls = Calls::default();
    let draft = complete(validate(&thing), &compiler, &calls, None).unwrap();
    assert_eq!(draft.footprint().requested, 253);
    assert_eq!(draft.counts(), (2, 2));
}
