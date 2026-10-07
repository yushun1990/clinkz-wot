#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile, WorkClass as W};
use consumer_borrowed_readmission_probe::{
    planning::{self, Build, Calls, Cause, Draft, Phase, Step},
    thing::Thing,
};
#[path = "support/case.rs"]
#[allow(dead_code)]
mod case;
#[allow(dead_code)]
mod support;
use case::*;
#[global_allocator]
static ALLOC: support::Allocator = support::Allocator;
fn input() -> Thing {
    serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"join","id":"urn:join","base":"https://example.test/a/b/","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"empty":{"type":"null","forms":[{"href":"unused","op":[]}]},"zeta":{"type":"null","forms":[{"href":"write","op":["writeproperty"]},{"href":"../c/./value?q=1#f","scopes":["first","second"],"contentCoding":"gzip"},{"href":"last"}]}},"actions":{"other":{"forms":[{"href":"action"}]}},"events":{"other":{"forms":[{"href":"event"}]}}}"#).unwrap()
}
fn complete<'a, 'r>(mut c: Build<'a, 'r>, calls: &Calls) -> Result<Draft, Cause> {
    for _ in 0..100_000 {
        if matches!(
            c.phase(),
            Phase::Preflight | Phase::Materialize | Phase::Bounds
        ) {
            assert_eq!(calls.starts.get(), 0);
        }
        match c.step(&mut budget(4096), false) {
            Step::Pending(n) => c = n,
            Step::Complete(d) => return Ok(d),
            Step::Failed(e) => return Err(e),
        }
    }
    panic!("stalled")
}
fn build<'a, 'r>(
    t: &'a Thing,
    r: &'r planning::registration::Registration,
    calls: &'r Calls,
    fail: Option<usize>,
    request: u64,
) -> Build<'a, 'r> {
    Build::new(
        validate(t, GatewayDefaultV1::limits(), 4096).unwrap(),
        r,
        calls,
        1_000_000,
        request,
        fail,
    )
    .unwrap()
}
#[test]
fn consuming_complete_owns_all_coordinates_after_real_source_and_registration_destruction() {
    let t = input();
    let registration = planning::registration::registration();
    assert!(
        registration
            .capabilities()
            .supports_consumer_property_read()
    );
    let calls = Calls::default();
    let proof = validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
    let g = support::start(None);
    let d = complete(
        Build::new(proof, &registration, &calls, 1_000_000, 100_000, None).unwrap(),
        &calls,
    )
    .unwrap();
    let footprint = d.footprint();
    assert_eq!(d.counts(), (2, 2));
    assert_eq!((calls.bounds.get(), calls.starts.get()), (2, 2));
    let c = support::counts();
    assert_eq!(c.live as u64, footprint.requested);
    let (requests, n) = support::requests();
    let runtime_largest = requests[..n]
        .iter()
        .filter(|r| r.0 != 16384)
        .map(|r| r.0)
        .max()
        .unwrap();
    assert_eq!(runtime_largest as u64, footprint.largest);
    assert_eq!(c.peak as u64, footprint.peak + 16384);
    assert_eq!(c.allocations as u64, footprint.allocations + 1);
    drop(t);
    drop(registration);
    assert_eq!(d.lookup_range("empty"), Some((0, 0)));
    assert_eq!(d.lookup_range("absent"), None);
    assert!(d.select("zeta", Some(0)).is_none());
    let (p, a) = d.select("zeta", Some(1)).unwrap();
    assert_eq!(p.resolved_target(), "https://example.test/a/c/value?q=1#f");
    assert_eq!(a.artifact().payload().target(), Some(p.resolved_target()));
    assert_eq!(
        d.raw_and_metadata(0),
        ("../c/./value?q=1#f", Some("gzip"), 2)
    );
    assert!(d.select("zeta", Some(2)).is_some());
    fn owned<T: 'static>(_: &T) {}
    owned(&d);
    drop(d);
    let c = support::counts();
    drop(g);
    assert_eq!(c.live, 0);
    assert_eq!(c.allocations, c.releases);
    println!(
        "concrete owned join: {footprint:?}; build={} result={}",
        core::mem::size_of::<Build<'static, 'static>>(),
        core::mem::size_of::<Step<'static, 'static>>()
    );
}

#[test]
fn fixed_allocator_executes_td_and_concrete_join_with_prepaid_startup_storage() {
    use clinkz_wot_foundation::{Generation, ResourceAccount, ResourceKind as R, SlotIndex};
    let t = input();
    let registration = planning::registration::registration();
    let calls = Calls::default();
    let pool = support::pool_owner_layout();
    // Includes all fixed Draft tables, the one owned compiler cursor, cleanup
    // state, and a complete current/return slot. Config/budget fit the spare.
    let inline = 2 * core::mem::size_of::<Step<'static, 'static>>() as u64;
    let mut parent = ResourceAccount::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        R::EngineLiveBytesGlobalMax,
        GatewayDefaultV1::limits()
            .get(R::EngineLiveBytesGlobalMax)
            .unwrap(),
    );
    parent.try_reserve(pool.size() as u64).unwrap().commit();
    parent.try_reserve(inline).unwrap().commit();
    let g = support::start_pool();
    let proof = validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
    let d = complete(
        Build::new(proof, &registration, &calls, 1_000_000, 100_000, None).unwrap(),
        &calls,
    )
    .unwrap();
    assert_eq!(support::counts().live as u64, d.footprint().requested);
    assert_eq!(support::pool_observations().live, support::counts().live);
    assert_eq!((calls.bounds.get(), calls.starts.get()), (2, 2));
    drop(t);
    drop(registration);
    let (plan, artifact) = d.select("zeta", Some(1)).unwrap();
    assert_eq!(
        artifact.artifact().payload().target(),
        Some(plan.resolved_target())
    );
    let observed = support::pool_observations();
    drop(d);
    assert_eq!(support::counts().live, 0);
    assert_eq!(support::pool_observations().live, 0);
    drop(g);
    assert!(parent.release_committed(inline));
    assert_eq!(parent.used(), pool.size() as u64); // allocator root stays reserved
    println!(
        "fixed allocator concrete join: {observed:?}; startup={pool:?}; inline current/return={inline}"
    );
}
#[test]
fn later_materialization_and_bounds_failure_have_zero_starts() {
    let t = input();
    let r = planning::registration::registration();
    let calls = Calls::default();
    let g = support::start(None);
    let e = complete(build(&t, &r, &calls, Some(1), 100_000), &calls)
        .err()
        .unwrap();
    assert_eq!(e, Cause::LaterBound);
    assert_eq!(calls.bounds.get(), 2);
    assert_eq!(calls.starts.get(), 0);
    assert_eq!(support::counts().live, 0);
    drop(g);
    // Property/name/id copies fit. The later coordinate's resolved target
    // rejects its request during materialization, before any bound/start.
    let mut t = input();
    let forms = &mut t
        .properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms;
    forms[1].href =
        consumer_borrowed_readmission_probe::data_type::FormHref::parse("urn:first").unwrap();
    forms[2].href = consumer_borrowed_readmission_probe::data_type::FormHref::parse(
        "https://example.test/second/long/target",
    )
    .unwrap();
    let calls = Calls::default();
    let g = support::start(None);
    let e = complete(build(&t, &r, &calls, None, 20), &calls)
        .err()
        .unwrap();
    assert_eq!(e, Cause::Capacity);
    assert_eq!((calls.bounds.get(), calls.starts.get()), (0, 0));
    assert_eq!(support::counts().live, 0);
    drop(g);
}
#[test]
fn empty_rootless_base_rejection_at_a_later_coordinate_has_zero_bounds_and_starts() {
    use consumer_borrowed_readmission_probe::{
        data_type::{BaseUri, FormHref, resolve_form_href},
        td::{ValidatedThingCause, ValidatedThingInvalidKind, ValidatedThingPhase},
    };
    for raw in ["x", "/x", "?q", "//h/x"] {
        let mut t = input();
        t.base = Some(BaseUri::parse("foo:").unwrap());
        let forms = &mut t
            .properties
            .as_mut()
            .unwrap()
            .get_mut("zeta")
            .unwrap()
            ._interaction
            .forms;
        // The first readable coordinate is valid; failure is at original index 2.
        forms[1].href = FormHref::parse("urn:first").unwrap();
        forms[2].href = FormHref::parse(raw).unwrap();
        assert!(resolve_form_href(t.base.as_ref(), &forms[2].href).is_err());
        let r = planning::registration::registration();
        let calls = Calls::default();
        let g = support::start(None);
        let e = complete(build(&t, &r, &calls, None, 100_000), &calls)
            .err()
            .unwrap();
        assert!(
            matches!(e, Cause::Td(ValidatedThingCause::Invalid(x)) if x.kind() == ValidatedThingInvalidKind::InvalidUri && x.phase() == ValidatedThingPhase::Semantics)
        );
        assert_eq!(
            (calls.bounds.get(), calls.starts.get(), calls.aborts.get()),
            (0, 0, 0)
        );
        assert_eq!(support::counts().live, 0);
        drop(g);
    }
}
#[test]
fn normalized_gateway_and_small_targets_survive_real_owned_handoff() {
    use clinkz_wot_foundation::ResourceKind as R;
    for (ceiling, base_segment) in [(16384, 16360), (64, 40)] {
        let limits = GatewayDefaultV1::limits()
            .clone()
            .with_limit(R::UriTemplateSourceBytesMax, Some(ceiling));
        let t = normalization_source(base_segment);
        let registration = planning::registration::registration();
        let calls = Calls::default();
        let proof = validate(&t, &limits, 4096).unwrap();
        let g = support::start(None);
        let d = complete(
            Build::new(proof, &registration, &calls, 1_000_000, 100_000, None).unwrap(),
            &calls,
        )
        .unwrap();
        assert_eq!((calls.bounds.get(), calls.starts.get()), (1, 1));
        assert_eq!(d.counts(), (1, 1));
        assert_eq!(support::counts().live as u64, d.footprint().requested);
        drop(t);
        drop(registration);
        let (plan, artifact) = d.select("p", Some(0)).unwrap();
        assert_eq!(plan.resolved_target(), "https://h/x");
        assert_eq!(
            artifact.artifact().payload().target(),
            Some(plan.resolved_target())
        );
        assert_eq!(d.raw_and_metadata(0).0, "bbbbbbbbbbbbbbbbbbbb/../../x");
        drop(d);
        assert_eq!(support::counts().live, 0);
        assert_eq!(support::counts().allocations, support::counts().releases);
        drop(g);
    }
}
#[test]
fn copy_and_cleanup_retry_never_revalidates_ready_uri_or_resizes_scopes() {
    let t = input();
    let r = planning::registration::registration();
    let calls = Calls::default();
    let mut c = build(&t, &r, &calls, None, 100_000);
    loop {
        if c.phase() == Phase::Materialize
            && c.uri_validation_bytes() > 0
            && c.scope_visits() == 2
            && c.ready()
        {
            break;
        }
        let mut b = budget(4096);
        b.set_remaining(W::CodecOutputBytes, 12);
        match c.step(&mut b, false) {
            Step::Pending(n) => c = n,
            Step::Failed(e) => panic!("{e:?}"),
            Step::Complete(_) => panic!(),
        }
    }
    let td = c.trace();
    let utf = c.uri_validation_bytes();
    let scopes = c.scope_visits();
    let remaining = c.remaining();
    assert!(utf > 0);
    assert!(scopes > 0);
    for (bytes, cleanup) in [(0, 100), (12, 100), (4096, 0), (0, 0)] {
        for _ in 0..4 {
            let mut b = budget(4096);
            b.set_remaining(W::CodecOutputBytes, bytes);
            b.set_remaining(W::CleanupItems, cleanup);
            let Step::Pending(n) = c.step(&mut b, false) else {
                panic!()
            };
            c = n;
            assert_eq!(c.trace(), td);
            assert_eq!(c.uri_validation_bytes(), utf);
            assert_eq!(c.scope_visits(), scopes);
            assert_eq!(c.remaining(), remaining);
        }
    }
    let d = complete(c, &calls).unwrap();
    assert_eq!(d.counts(), (2, 2));
}
#[test]
fn every_fixed_build_position_has_prepaid_drop_and_exactly_once_abort() {
    let t = input();
    let r = planning::registration::registration();
    let mut positions = 0;
    let calls = Calls::default();
    let mut c = build(&t, &r, &calls, None, 100_000);
    loop {
        positions += 1;
        match c.step(&mut budget(4096), false) {
            Step::Pending(n) => c = n,
            Step::Complete(d) => {
                drop(d);
                break;
            }
            _ => panic!(),
        }
    }
    for n in 0..positions {
        for cancel in [false, true] {
            let calls = Calls::default();
            let g = support::start(None);
            let mut c = build(&t, &r, &calls, None, 100_000);
            for _ in 0..n {
                let Step::Pending(next) = c.step(&mut budget(4096), false) else {
                    panic!()
                };
                c = next;
            }
            let pending = c.compiler_pending();
            if cancel {
                assert!(matches!(
                    c.step(&mut budget(0), true),
                    Step::Failed(Cause::Cancelled)
                ));
            } else {
                drop(c);
            }
            assert_eq!(calls.aborts.get(), usize::from(pending));
            let counts = support::counts();
            drop(g);
            assert_eq!(counts.live, 0);
            assert_eq!(counts.allocations, counts.releases);
        }
    }
    println!("fixed external build cancellation/drop positions={positions}");
}
#[test]
fn missing_id_and_explicit_empty_security_do_not_compile() {
    let mut t = input();
    t.id = None;
    let r = planning::registration::registration();
    let calls = Calls::default();
    let proof = validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
    assert!(matches!(
        Build::new(proof, &r, &calls, 1_000_000, 100_000, None),
        Err(Cause::MissingId)
    ));
    let mut t = input();
    t.properties
        .as_mut()
        .unwrap()
        .get_mut("zeta")
        .unwrap()
        ._interaction
        .forms[1]
        .security = Some(vec![]);
    assert!(matches!(
        complete(build(&t, &r, &calls, None, 100_000), &calls),
        Err(Cause::Capacity)
    ));
    assert_eq!(calls.starts.get(), 0);
}
#[test]
fn every_fallible_td_and_materialization_request_before_bounds_rolls_back_without_start() {
    let t = input();
    let r = planning::registration::registration();
    let calls = Calls::default();
    let proof = validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
    let g = support::start(None);
    let mut c = Build::new(proof, &r, &calls, 1_000_000, 100_000, None).unwrap();
    while c.phase() != Phase::Bounds {
        let Step::Pending(n) = c.step(&mut budget(4096), false) else {
            panic!()
        };
        c = n;
    }
    let count = support::counts().attempts;
    drop(c);
    assert_eq!(support::counts().live, 0);
    drop(g);
    for fail in 1..=count {
        let calls = Calls::default();
        let proof = validate(&t, GatewayDefaultV1::limits(), 4096).unwrap();
        let g = support::start(Some(fail));
        let e = complete(
            Build::new(proof, &r, &calls, 1_000_000, 100_000, None).unwrap(),
            &calls,
        )
        .err()
        .unwrap();
        assert!(matches!(
            e,
            Cause::Allocation
                | Cause::Td(
                    consumer_borrowed_readmission_probe::td::ValidatedThingCause::Failed(_)
                )
        ));
        assert_eq!((calls.bounds.get(), calls.starts.get()), (0, 0));
        let n = support::counts();
        drop(g);
        assert_eq!(n.live, 0);
        assert_eq!(n.allocations, n.releases);
    }
}
