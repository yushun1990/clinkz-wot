use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactRole, BindingConfigurationDigest,
    BindingGeneration, BindingId, PlanSetGeneration,
};
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceLimits, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass as W,
};
use clinkz_wot_td::{
    ValidatedPropertyReadCursor, ValidatedThingAdmissionConfig, ValidatedThingCause,
    ValidatedThingCursor, ValidatedThingPhase, ValidatedThingProgress, thing::Thing,
};
use clinkz_wot_td_semantic_join::{Build, Calls, Cause, Draft, Limits, Phase, Step, registration};
mod support;
#[global_allocator]
static ALLOCATOR: support::Allocator = support::Allocator;

fn input() -> Thing {
    serde_json::from_str(r#"{
      "@context":"https://www.w3.org/2022/wot/td/v1.1",
      "title":"production join", "id":"urn:join", "base":"https://example.test/a/b/",
      "security":["none"],
      "securityDefinitions":{"basic":{"scheme":"basic"},"none":{"scheme":"nosec"}},
      "properties":{
        "a-empty":{"type":"null","forms":[{"href":"ignored","op":[]}]},
        "b-write":{"type":"null","writeOnly":true,"forms":[{"href":"ignored"}]},
        "c-zero":{"type":"null","forms":[]},
        "zeta":{"type":"null","forms":[
          {"href":"write","op":["writeproperty"]},
          {"href":"../c/./value?q=1#f","scopes":["first","二", ""],"contentCoding":"gzip","subprotocol":"sub"},
          {"href":"last/longer/target","op":["readproperty"],"security":["none"],"contentType":"text/plain"}
        ]},
        "zz-tail":{"type":"null","forms":[]}
      },
      "actions":{"unrelated":{"forms":[{"href":"action"}]}},
      "events":{"unrelated":{"forms":[{"href":"event"}]}}
    }"#).unwrap()
}
fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn validate<'a>(
    source: &'a Thing,
    limits: &ResourceLimits,
) -> Result<ValidatedPropertyReadCursor<'a>, Cause> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(limits).unwrap();
    let ledger = AdmissionLedger::new(
        SlotIndex::new(1),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        0,
        0,
        0,
    );
    let mut cursor = ValidatedThingCursor::from_thing(source, &config, ledger);
    for _ in 0..100_000 {
        match cursor.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => return Ok(proof.into_property_read()),
            ValidatedThingProgress::Failed(cause) => return Err(Cause::Td(cause)),
        }
    }
    panic!("validation stalled")
}
fn build<'a, 'r>(
    source: &'a Thing,
    r: &'r registration::Registration,
    calls: &'r Calls,
    limits: Limits,
) -> Result<Build<'a, 'r>, Cause> {
    Build::new(
        validate(source, GatewayDefaultV1::LIMITS)?,
        r,
        calls,
        limits,
    )
}
fn complete(mut build: Build<'_, '_>, calls: &Calls) -> Result<Draft, Cause> {
    for _ in 0..100_000 {
        if build.phase() != Phase::Compile {
            assert_eq!(calls.starts.get(), 0);
        }
        if calls.starts.get() > 0 {
            assert_eq!(
                calls.bounds.get(),
                2,
                "start crossed the all-bounds barrier"
            );
        }
        match build.step(&mut budget(4096), false) {
            Step::Pending(next) => build = next,
            Step::Complete(draft) => return Ok(draft),
            Step::Failed(cause) => return Err(cause),
        }
    }
    panic!("join stalled")
}
fn assert_output(draft: &Draft) {
    assert_eq!(draft.counts(), (5, 2));
    for name in ["a-empty", "b-write", "c-zero"] {
        assert_eq!(draft.lookup_range(name), Some((0, 0)));
        assert!(draft.select(name, None).is_none());
    }
    assert_eq!(draft.lookup_range("absent"), None);
    assert_eq!(draft.lookup_range("zeta"), Some((0, 2)));
    assert_eq!(draft.lookup_range("zz-tail"), Some((2, 2)));
    assert!(draft.select("zeta", Some(0)).is_none());
    assert!(draft.select("zeta", Some(3)).is_none());
    for (index, target, content) in [
        (
            1,
            "https://example.test/a/c/value?q=1#f",
            "application/json",
        ),
        (
            2,
            "https://example.test/a/b/last/longer/target",
            "text/plain",
        ),
    ] {
        let row = draft.select("zeta", Some(index)).unwrap();
        assert_eq!(row.property_ordinal, 3);
        assert_eq!(row.plan.form_index(), index);
        assert_eq!(row.plan.property_name(), "zeta");
        assert_eq!(row.plan.thing_id().as_str(), "urn:join");
        assert_eq!(row.plan.resolved_target(), target);
        assert_eq!(row.plan.content_type(), Some(content));
        let artifact = row.artifact.as_ref().unwrap();
        assert_eq!(artifact.artifact().payload().target(), Some(target));
        let identity = artifact.identity();
        assert_eq!(identity.binding_id(), BindingId::new(7));
        assert_eq!(identity.binding_generation(), BindingGeneration::INITIAL);
        assert_eq!(
            identity.configuration(),
            BindingConfigurationDigest::new([0x52; 32])
        );
        assert_eq!(
            identity.compatibility(),
            BindingArtifactCompatibility::new([0x41; 16])
        );
        assert_eq!(identity.plan_set_generation(), PlanSetGeneration::INITIAL);
        assert_eq!(identity.plan_id(), row.plan.plan_id());
        assert_eq!(identity.role(), BindingArtifactRole::ConsumerCall);
        assert!(artifact.artifact().route_reservation().is_none());
    }
    let first = draft.select("zeta", None).unwrap();
    assert_eq!(first.raw.as_ref(), "../c/./value?q=1#f");
    assert_eq!(first.coding.as_deref(), Some("gzip"));
    assert_eq!(first.plan.subprotocol(), Some("sub"));
    assert_eq!(first.scopes[0].as_deref(), Some("first"));
    assert_eq!(first.scopes[1].as_deref(), Some("二"));
    assert_eq!(first.scopes[2].as_deref(), Some(""));
    assert!(first.scopes[3].is_none());
}
#[test]
fn readable_properties_have_disjoint_ranges_and_property_specific_selection() {
    let mut source = input();
    source.properties = Some(serde_json::from_str(r#"{
      "a-read":{"type":"null","forms":[{"href":"ignored","op":[]},{"href":"first"}]},
      "b-empty":{"type":"null","forms":[]},
      "c-read":{"type":"null","forms":[{"href":"second"},{"href":"ignored","op":[]},{"href":"third"}]},
      "d-tail":{"type":"null","forms":[]}
    }"#).unwrap());
    let r = registration::registration();
    let calls = Calls::default();
    let mut b = build(&source, &r, &calls, Limits::default()).unwrap();
    let draft = loop {
        if calls.starts.get() != 0 {
            assert_eq!(calls.bounds.get(), 3);
        }
        match b.step(&mut budget(4096), false) {
            Step::Pending(next) => b = next,
            Step::Complete(draft) => break draft,
            Step::Failed(cause) => panic!("{cause:?}"),
        }
    };
    drop(source);
    drop(r);
    assert_eq!(draft.counts(), (4, 3));
    for (name, range) in [
        ("a-read", (0, 1)),
        ("b-empty", (1, 1)),
        ("c-read", (1, 3)),
        ("d-tail", (3, 3)),
    ] {
        assert_eq!(draft.lookup_range(name), Some(range));
    }
    for (name, ordinal, index, target) in [
        ("a-read", 0, 1, "https://example.test/a/b/first"),
        ("c-read", 2, 0, "https://example.test/a/b/second"),
        ("c-read", 2, 2, "https://example.test/a/b/third"),
    ] {
        let row = draft.select(name, Some(index)).unwrap();
        assert_eq!(row.property_ordinal, ordinal);
        assert_eq!(row.plan.property_name(), name);
        assert_eq!(row.plan.form_index(), index);
        assert_eq!(row.plan.resolved_target(), target);
        assert_eq!(
            row.artifact.as_ref().unwrap().artifact().payload().target(),
            Some(target)
        );
    }
    assert_eq!(draft.select("a-read", None).unwrap().plan.form_index(), 1);
    assert_eq!(draft.select("c-read", None).unwrap().plan.form_index(), 0);
    for (name, index) in [
        ("a-read", 0),
        ("a-read", 2),
        ("c-read", 1),
        ("b-empty", 1),
        ("d-tail", 2),
    ] {
        assert!(draft.select(name, Some(index)).is_none());
    }
    assert_eq!((calls.bounds.get(), calls.starts.get()), (3, 3));
}

#[test]
fn production_cursor_yields_owned_core_plans_and_concrete_artifacts() {
    let source = input();
    let registration = registration::registration();
    assert!(
        registration
            .capabilities()
            .supports_consumer_property_read()
    );
    let calls = Calls::default();
    let observation = support::start(None);
    let draft = complete(
        build(&source, &registration, &calls, Limits::default()).unwrap(),
        &calls,
    )
    .unwrap();
    assert_eq!(
        (
            calls.bounds.get(),
            calls.starts.get(),
            calls.steps.get(),
            calls.aborts.get()
        ),
        (2, 2, 2, 0)
    );
    assert_eq!(support::counts().live as u64, draft.heap_bytes());
    fn owned<T: 'static>(_: &T) {}
    owned(&draft);
    drop(source);
    drop(registration);
    assert_output(&draft);
    drop(draft);
    drop(observation);
}
#[test]
fn completely_empty_readable_output_preserves_every_lookup() {
    let mut source = input();
    for property in source.properties.as_mut().unwrap().values_mut() {
        for form in &mut property._interaction.forms {
            form.op = Some(vec![]);
        }
    }
    let registration = registration::registration();
    let calls = Calls::default();
    let observation = support::start(None);
    let draft = complete(
        build(&source, &registration, &calls, Limits::default()).unwrap(),
        &calls,
    )
    .unwrap();
    drop(source);
    drop(registration);
    assert_eq!(draft.counts(), (5, 0));
    for name in ["a-empty", "b-write", "c-zero", "zeta", "zz-tail"] {
        assert_eq!(draft.lookup_range(name), Some((0, 0)));
    }
    assert_eq!((calls.bounds.get(), calls.starts.get()), (0, 0));
    assert_eq!(support::counts().live as u64, draft.heap_bytes());
    drop(draft);
    drop(observation);
}
#[test]
fn compiler_cursor_retries_need_fresh_work_and_abandonment_aborts_once() {
    let source = input();
    let registration = registration::registration();
    let calls = Calls::default();
    let observation = support::start(None);
    let mut b = build(&source, &registration, &calls, Limits::default()).unwrap();
    while !b.compiler_pending() {
        b = match b.step(&mut budget(4096), false) {
            Step::Pending(b) => b,
            _ => panic!("early exit"),
        };
    }
    assert_eq!(
        (calls.bounds.get(), calls.starts.get(), calls.steps.get()),
        (2, 1, 0)
    );
    let state = support::counts();
    let remaining = b.remaining();
    for _ in 0..4 {
        b = match b.step(&mut budget(0), false) {
            Step::Pending(b) => b,
            _ => panic!("zero work"),
        };
        assert_eq!(support::counts(), state);
        assert_eq!(b.remaining(), remaining);
        assert_eq!(calls.steps.get(), 0);
    }
    drop(b);
    drop(source);
    drop(registration);
    assert_eq!(calls.aborts.get(), 1);
    drop(observation);
}
#[test]
fn fixed_catalog_and_allowances_reject_without_starting_compilers() {
    for variant in 0..5 {
        let mut source = input();
        let mut limits = Limits::default();
        match variant {
            0 => {
                let properties = source.properties.as_mut().unwrap();
                let empty = properties["c-zero"].clone();
                for n in 0..4 {
                    properties.insert(format!("zz-extra-{n}"), empty.clone());
                }
            }
            1 => {
                source
                    .properties
                    .as_mut()
                    .unwrap()
                    .get_mut("zeta")
                    .unwrap()
                    ._interaction
                    .forms[1]
                    .scopes = Some(vec!["scope".into(); 5])
            }
            2 => limits.output_bytes = 0,
            3 => limits.copy_request_bytes = 10,
            4 => limits.planning_work = 0,
            _ => unreachable!(),
        }
        let registration = registration::registration();
        let calls = Calls::default();
        let observation = support::start(None);
        let result =
            build(&source, &registration, &calls, limits).and_then(|b| complete(b, &calls));
        assert!(matches!(result, Err(Cause::Capacity) | Err(Cause::Work)));
        assert_eq!((calls.bounds.get(), calls.starts.get()), (0, 0));
        drop(observation);
    }
}
#[test]
fn later_actual_compiler_bound_rejects_before_any_start() {
    let source = input();
    let registration = registration::registration();
    let calls = Calls::default();
    let observation = support::start(None);
    let limits = Limits {
        artifact_bytes: "https://example.test/a/c/value?q=1#f".len() as u64,
        ..Limits::default()
    };
    assert_eq!(
        complete(
            build(&source, &registration, &calls, limits).unwrap(),
            &calls
        )
        .err(),
        Some(Cause::Bounds)
    );
    assert_eq!(
        (calls.bounds.get(), calls.starts.get(), calls.steps.get()),
        (2, 0, 0)
    );
    drop(observation);
}
#[test]
fn later_materialization_allocation_failure_has_no_bounds_or_starts() {
    let source = input();
    let registration = registration::registration();
    let calls = Calls::default();
    let observation = support::start(None);
    let mut b = build(&source, &registration, &calls, Limits::default()).unwrap();
    while b.counts().1 != 1 {
        b = match b.step(&mut budget(4096), false) {
            Step::Pending(b) => b,
            _ => panic!("early exit"),
        };
    }
    support::fail_next();
    assert_eq!(complete(b, &calls).err(), Some(Cause::Allocation));
    assert_eq!((calls.bounds.get(), calls.starts.get()), (0, 0));
    drop(observation);
}
#[test]
fn every_fallible_validation_semantic_and_output_request_rolls_back() {
    let source = input();
    let registration = registration::registration();
    let calls = Calls::default();
    let observation = support::start(None);
    let mut b = build(&source, &registration, &calls, Limits::default()).unwrap();
    while b.phase() != Phase::Bounds {
        b = match b.step(&mut budget(4096), false) {
            Step::Pending(b) => b,
            _ => panic!("early exit"),
        };
    }
    let attempts = support::counts().attempts;
    drop(b);
    drop(observation);
    assert!(attempts > 20);
    for fail in 1..=attempts {
        let calls = Calls::default();
        let observation = support::start(Some(fail));
        let result = build(&source, &registration, &calls, Limits::default())
            .and_then(|b| complete(b, &calls));
        assert!(
            matches!(
                result,
                Err(Cause::Allocation) | Err(Cause::Td(ValidatedThingCause::Failed(_)))
            ),
            "request {fail}"
        );
        assert_eq!(
            (calls.bounds.get(), calls.starts.get(), calls.aborts.get()),
            (0, 0, 0)
        );
        assert_eq!(support::counts().attempts, fail);
        drop(observation);
    }
}
#[test]
fn ready_copy_shortages_preserve_work_memory_and_coordinate() {
    let source = input();
    let registration = registration::registration();
    let calls = Calls::default();
    let observation = support::start(None);
    let mut b = build(&source, &registration, &calls, Limits::default()).unwrap();
    // One copy byte permits TD's byte-resumable URI program to reach Ready,
    // while preventing this entire Form's atomic Planning copy.
    for _ in 0..100_000 {
        if b.phase() == Phase::Materialize && b.counts() == (4, 0) && b.copy_wait() {
            break;
        }
        let n = if b.phase() == Phase::Materialize && b.counts().0 == 4 {
            1
        } else {
            4096
        };
        b = match b.step(
            &mut budget(4096).with_remaining(W::CodecOutputBytes, n),
            false,
        ) {
            Step::Pending(b) => b,
            _ => panic!("early exit"),
        };
    }
    assert!(b.copy_wait());
    // Independent byte oracle for the first output coordinate; no TD debit,
    // copy_bytes getter or private implementation trace supplies this total.
    let copy: u64 = [
        "urn:join",
        "zeta",
        "../c/./value?q=1#f",
        "https://example.test/a/c/value?q=1#f",
        "application/json",
        "gzip",
        "sub",
        "first",
        "二",
        "",
    ]
    .iter()
    .map(|s| s.len() as u64)
    .sum();
    let state = support::counts();
    let remaining = b.remaining();
    for (bytes, items, cleanup) in [
        (0, 4096, 4096),
        (copy - 1, 4096, 4096),
        (4096, 3, 4096),
        (4096, 4096, 9),
    ] {
        for _ in 0..3 {
            let mut work = budget(4096)
                .with_remaining(W::CodecOutputBytes, bytes)
                .with_remaining(W::PlanningItems, items)
                .with_remaining(W::CleanupItems, cleanup);
            b = match b.step(&mut work, false) {
                Step::Pending(b) => b,
                _ => panic!("partial event"),
            };
            assert_eq!(b.counts(), (4, 0));
            assert_eq!(b.remaining(), remaining);
            assert_eq!(support::counts(), state);
            for w in W::ALL {
                assert_eq!(
                    work.remaining(w),
                    match w {
                        W::CodecOutputBytes => bytes,
                        W::PlanningItems => items,
                        W::CleanupItems => cleanup,
                        _ => 4096,
                    }
                );
            }
        }
    }
    let draft = complete(b, &calls).unwrap();
    assert_output(&draft);
    drop(draft);
    drop(observation);
}
#[test]
fn every_join_suspension_drops_or_cancels_with_bounded_exact_abort() {
    let source = input();
    let registration = registration::registration();
    let calls = Calls::default();
    let mut b = build(&source, &registration, &calls, Limits::default()).unwrap();
    let mut positions = 0;
    loop {
        positions += 1;
        match b.step(&mut budget(4096), false) {
            Step::Pending(next) => b = next,
            Step::Complete(d) => {
                drop(d);
                break;
            }
            Step::Failed(e) => panic!("{e:?}"),
        }
    }
    assert!(positions > 20);
    for position in 0..positions {
        for cancel in [false, true] {
            let calls = Calls::default();
            let observation = support::start(None);
            let mut b = build(&source, &registration, &calls, Limits::default()).unwrap();
            for _ in 0..position {
                b = match b.step(&mut budget(4096), false) {
                    Step::Pending(b) => b,
                    _ => panic!("position"),
                };
            }
            let pending = b.compiler_pending();
            if cancel {
                assert!(matches!(
                    b.step(&mut budget(0), true),
                    Step::Failed(Cause::Cancelled)
                ));
            } else {
                drop(b);
            }
            assert_eq!(calls.aborts.get(), usize::from(pending));
            drop(observation);
        }
    }
}
#[test]
fn missing_id_empty_security_and_late_semantic_failure_never_start() {
    for variant in 0..4 {
        let mut source = input();
        match variant {
            0 => source.id = None,
            1 => {
                source
                    .properties
                    .as_mut()
                    .unwrap()
                    .get_mut("zeta")
                    .unwrap()
                    ._interaction
                    .forms[2]
                    .security = Some(vec![])
            }
            2 => {
                source
                    .properties
                    .as_mut()
                    .unwrap()
                    .get_mut("zeta")
                    .unwrap()
                    ._interaction
                    .forms[2]
                    .security = Some(vec!["basic".into()])
            }
            3 => {
                source.base = Some(clinkz_wot_td::data_type::BaseUri::parse("foo:").unwrap());
                let forms = &mut source
                    .properties
                    .as_mut()
                    .unwrap()
                    .get_mut("zeta")
                    .unwrap()
                    ._interaction
                    .forms;
                forms[1].href =
                    clinkz_wot_td::data_type::FormHref::parse("urn:valid-first").unwrap();
                forms[2].href =
                    clinkz_wot_td::data_type::FormHref::parse("later-relative").unwrap();
            }
            _ => unreachable!(),
        }
        let registration = registration::registration();
        let calls = Calls::default();
        let observation = support::start(None);
        let result = build(&source, &registration, &calls, Limits::default())
            .and_then(|b| complete(b, &calls));
        match variant {
            0 => assert!(matches!(result, Err(Cause::MissingId))),
            1 | 2 => assert!(matches!(result, Err(Cause::IneligibleSecurity))),
            3 => assert!(
                matches!(result, Err(Cause::Td(ValidatedThingCause::Invalid(x))) if x.phase() == ValidatedThingPhase::Semantics)
            ),
            _ => unreachable!(),
        }
        assert_eq!((calls.bounds.get(), calls.starts.get()), (0, 0));
        drop(observation);
    }
}
