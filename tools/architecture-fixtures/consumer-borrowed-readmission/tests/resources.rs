#![cfg(feature = "validated-thing")]
use clinkz_wot_foundation::{
    AdmissionLedger, BenchmarkStaticReferenceV1, GatewayDefaultV1, Generation, ResourceAccount,
    ResourceKind as R, SlotIndex, StaticResourceProfile,
};
use consumer_borrowed_readmission_probe::{data_type::BaseUri, td::*, thing::Thing};
#[path = "support/case.rs"]
mod case;
mod support;
use case::*;
#[global_allocator]
static ALLOC: support::Allocator = support::Allocator;
fn nested() -> Thing {
    let mut t = source();
    let mut v = serde_json::json!({"λ": "a".repeat(128)});
    for _ in 0..14 {
        v = serde_json::json!([v]);
    }
    t._extra_fields.insert("deep".into(), v);
    t.base = Some(BaseUri::parse("https://example.test/a/b/").unwrap());
    t
}
#[test]
fn all_real_layout_requests_overlap_and_terminal_release() {
    let t = nested();
    let l = GatewayDefaultV1::limits();
    let g = support::start(None);
    let proof = validate(&t, l, 4096).unwrap();
    let trace = proof.trace();
    let (requests, n) = support::requests();
    let events = proof.allocation_events();
    let mut j = 0;
    let mut live = 0;
    let mut peak = 0;
    for e in events {
        match e.kind {
            AllocationEventKind::Allocated => {
                assert_eq!(requests[j], (e.bytes as usize, e.alignment));
                j += 1;
                live += e.bytes;
                peak = peak.max(live);
            }
            AllocationEventKind::BeforeRelease => assert_eq!(e.local_live, live),
            AllocationEventKind::Released => {
                live -= e.bytes;
                assert_eq!(e.local_live, live)
            }
            _ => {}
        }
    }
    assert_eq!(j, n);
    assert_eq!(live, 0);
    assert_eq!(peak, trace.peak);
    let observed = support::counts();
    assert_eq!(observed.allocations as u64, trace.allocations);
    assert_eq!(observed.peak as u64, trace.peak);
    assert_eq!(observed.largest as u64, trace.largest);
    assert!(trace.frame_moves > 0);
    let mut read = proof.into_property_read();
    drive(&mut read, 4).unwrap();
    let trace = read.trace();
    let frame = layout_catalog()[4];
    let config = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    let (requests, n) = support::requests();
    for &(bytes, alignment) in &requests[..n] {
        if alignment == frame.alignment {
            assert_eq!(bytes % frame.size, 0);
            assert!(bytes / frame.size <= config.frame_capacity());
        } else {
            assert_eq!(alignment, 1);
            assert_eq!(bytes, l.get(R::UriTemplateSourceBytesMax).unwrap() as usize);
        }
    }
    assert!(trace.peak <= config.controlled_heap_envelope());
    drop(read);
    let c = support::counts();
    drop(g);
    assert_eq!(c.live, 0);
    assert_eq!(c.allocations, c.releases);
    println!(
        "TD layouts={:?}; heap peak={} largest={} allocations={} moves={}",
        layout_catalog(),
        trace.peak,
        trace.largest,
        trace.allocations,
        trace.frame_moves
    );
}
#[test]
fn every_td_allocation_failure_is_fixed_and_source_free() {
    let t = nested();
    let l = GatewayDefaultV1::limits();
    let g = support::start(None);
    let mut read = validate(&t, l, 4096).unwrap().into_property_read();
    drive(&mut read, 4).unwrap();
    drop(read);
    let total = support::counts().attempts;
    drop(g);
    for fail in 1..=total {
        let g = support::start(Some(fail));
        let e = match validate(&t, l, 4096) {
            Err(e) => e,
            Ok(v) => {
                let mut read = v.into_property_read();
                let e = drive(&mut read, 4).unwrap_err();
                assert_eq!(read.step(&mut budget(0), true).err(), Some(e));
                drop(read);
                e
            }
        };
        assert!(
            matches!(e,ValidatedThingCause::Failed(f) if f.kind()==ValidatedThingFailureKind::AllocationFailed)
        );
        let c = support::counts();
        drop(g);
        assert_eq!(c.live, 0, "failure {fail}");
        assert_eq!(c.allocations, c.releases);
    }
}
#[test]
fn every_inspect_basic_and_semantic_suspension_can_cancel_or_drop() {
    let t = nested();
    let l = GatewayDefaultV1::limits();
    let cfg = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    let mut terminal = 0;
    let mut c = ValidatedThingCursor::from_thing(&t, &cfg, ledger(l));
    loop {
        terminal += 1;
        match c.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(n) => c = n,
            ValidatedThingProgress::Complete(v) => {
                drop(v);
                break;
            }
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        }
    }
    for position in 0..terminal {
        for cancel in [false, true] {
            let g = support::start(None);
            let mut c = ValidatedThingCursor::from_thing(&t, &cfg, ledger(l));
            for _ in 0..position {
                let ValidatedThingProgress::Pending(n) = c.step(&mut budget(4096), false) else {
                    panic!()
                };
                c = n;
            }
            if cancel {
                assert!(matches!(
                    c.step(&mut budget(0), true),
                    ValidatedThingProgress::Failed(ValidatedThingCause::Cancelled { .. })
                ));
            } else {
                drop(c);
            }
            let n = support::counts();
            drop(g);
            assert_eq!(n.live, 0);
            assert_eq!(n.allocations, n.releases);
        }
    }
    let proof = validate(&t, l, 4096).unwrap();
    let mut read = proof.into_property_read();
    let mut terminal_read = 0;
    loop {
        terminal_read += 1;
        match read.step(&mut budget(4), false).unwrap() {
            ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Done => break,
        }
    }
    drop(read);
    for position in 0..terminal_read {
        for cancel in [false, true] {
            let g = support::start(None);
            let mut read = validate(&t, l, 4096).unwrap().into_property_read();
            for _ in 0..position {
                match read.step(&mut budget(4), false).unwrap() {
                    ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
                    _ => {}
                }
            }
            if cancel {
                let first = read.step(&mut budget(0), true).err().unwrap();
                assert!(matches!(first, ValidatedThingCause::Cancelled { .. }));
                assert_eq!(read.step(&mut budget(4096), false).err(), Some(first));
            }
            drop(read);
            let c = support::counts();
            drop(g);
            assert_eq!(c.live, 0);
            assert_eq!(c.allocations, c.releases);
        }
    }
    println!(
        "exhaustive cancellation/drop positions: inspect/Basic={} semantic={}",
        terminal, terminal_read
    );
}
#[test]
fn every_normalization_suspension_and_allocation_failure_preserves_terminal_ownership() {
    let t = normalization_source(40);
    let l = GatewayDefaultV1::limits()
        .clone()
        .with_limit(R::UriTemplateSourceBytesMax, Some(64));
    let g = support::start(None);
    let mut read = validate(&t, &l, 4096).unwrap().into_property_read();
    let mut positions = 0;
    loop {
        positions += 1;
        match read.step(&mut budget(4), false).unwrap() {
            ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Done => break,
        }
    }
    assert!(read.uri_observations().pop_bytes >= 40);
    assert_eq!(read.uri_observations().reversed_bytes, 2);
    drop(read);
    let attempts = support::counts().attempts;
    assert_eq!(support::counts().live, 0);
    drop(g);
    for fail in 1..=attempts {
        let g = support::start(Some(fail));
        let e = match validate(&t, &l, 4096) {
            Err(e) => e,
            Ok(proof) => {
                let mut read = proof.into_property_read();
                let e = drive(&mut read, 4).unwrap_err();
                assert_eq!(read.step(&mut budget(0), true).err(), Some(e));
                drop(read);
                e
            }
        };
        assert!(
            matches!(e, ValidatedThingCause::Failed(x) if x.kind() == ValidatedThingFailureKind::AllocationFailed)
        );
        assert_eq!(support::counts().live, 0);
        assert_eq!(support::counts().allocations, support::counts().releases);
        drop(g);
    }
    for position in 0..positions {
        for cancel in [false, true] {
            let g = support::start(None);
            let mut read = validate(&t, &l, 4096).unwrap().into_property_read();
            for _ in 0..position {
                match read.step(&mut budget(4), false).unwrap() {
                    ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
                    _ => {}
                }
            }
            if cancel {
                let e = read.step(&mut budget(0), true).err().unwrap();
                assert!(matches!(e, ValidatedThingCause::Cancelled { .. }));
                assert_eq!(read.step(&mut budget(4096), false).err(), Some(e));
            }
            drop(read);
            assert_eq!(support::counts().live, 0);
            assert_eq!(support::counts().allocations, support::counts().releases);
            drop(g);
        }
    }
    println!(
        "normalization cancellation/drop positions={positions}; allocation failures={attempts}"
    );
}
#[test]
fn allocation_limits_reject_before_allocator_entry() {
    let t = source();
    let l = GatewayDefaultV1::LIMITS;
    for r in [
        R::AdmissionTemporaryBytesPerOperationMax,
        R::AdmissionTemporaryBytesGlobalMax,
        R::PeakLiveBytesPerAdmissionMax,
        R::AdmissionPeakLiveBytesGlobalMax,
        R::EngineLiveBytesGlobalMax,
        R::LargestContiguousAllocationBytesMax,
    ] {
        let l = l.clone().with_limit(r, Some(0));
        let g = support::start(None);
        let e = validate(&t, &l, 4096).err().unwrap();
        let c = support::counts();
        drop(g);
        assert!(matches!(e, ValidatedThingCause::Limit(_)));
        assert_eq!(c.attempts, 0);
    }
}
#[test]
fn actual_upstream_source_keeps_original_parent_charge_until_physical_drop() {
    let l = GatewayDefaultV1::limits();
    let cfg = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    let mut upstream = AdmissionLedger::new(
        SlotIndex::new(9),
        Generation::INITIAL,
        1_000_000,
        0,
        0,
        0,
        0,
        0,
    );
    let mut parent = ResourceAccount::new(
        SlotIndex::new(1),
        Generation::INITIAL,
        R::EngineLiveBytesGlobalMax,
        4_000_000,
    );
    let mut g = support::source_start(&mut upstream, &mut parent);
    // The engine-owned root itself has an observed physical Layout as well as
    // its nested allocations; no uncharged inline Thing owner is assumed.
    let t = Box::new(nested());
    g.lend();
    let (requests, count) = support::requests();
    assert_eq!(
        requests[count - 1],
        (
            core::mem::size_of::<Thing>(),
            core::mem::align_of::<Thing>()
        )
    );
    let original = g.source_live();
    assert!(original > 0);
    assert_eq!(original, support::counts().live as u64);
    let envelope = cfg.controlled_heap_envelope()
        + 2 * layout_catalog()
            .iter()
            .take(4)
            .map(|r| r.size as u64)
            .max()
            .unwrap();
    assert!(g.reserve_children(envelope));
    let child = AdmissionLedger::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        0,
        cfg.controlled_heap_envelope(),
        0,
        0,
        0,
        0,
    );
    let mut cursor = ValidatedThingCursor::from_thing(&t, &cfg, child);
    let proof = loop {
        match cursor.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(next) => cursor = next,
            ValidatedThingProgress::Complete(proof) => break proof,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        }
    };
    let mut read = proof.into_property_read();
    drive(&mut read, 4).unwrap();
    assert_eq!(g.source_live(), original);
    assert_eq!(g.parent_live(), original + envelope);
    drop(read);
    assert_eq!(support::counts().live as u64, original);
    g.release_children(envelope);
    assert_eq!(g.source_live(), original);
    drop(t);
    assert_eq!(support::counts().live, 0);
    assert_eq!(g.source_live(), 0);
    drop(g);
    assert_eq!(upstream.live_bytes(), 0);
    assert_eq!(parent.used(), 0);
    println!(
        "original physical upstream source={} paired TD capacity={}",
        original, envelope
    );
}
#[test]
fn named_static_profile_has_its_own_checked_envelope() {
    let l = BenchmarkStaticReferenceV1::limits();
    let c = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    assert_eq!(l.get(R::NumberLexemeBytesMax), Some(64));
    validate(&source(), l, 4096).unwrap();
    println!(
        "static TD reserved heap envelope={}",
        c.controlled_heap_envelope()
    );
}
#[test]
fn all_local_and_global_parent_capacities_cover_inline_return_overlap_before_entry() {
    use clinkz_wot_foundation::ResourceLimits;
    let l = GatewayDefaultV1::limits();
    let cfg = ValidatedThingAdmissionConfig::try_from_limits(l).unwrap();
    // Reserve two maximum TD owner slots: current plus movement/return. The
    // borrowed step event and checked policy fit inside the unused slot. These
    // are logical caller-owned capacity, never one allocator request.
    let inline = 2 * layout_catalog()
        .iter()
        .take(4)
        .map(|r| r.size as u64)
        .max()
        .unwrap();
    let heap = cfg.controlled_heap_envelope();
    let child = inline + heap;
    let kinds = [
        R::AdmissionTemporaryBytesPerOperationMax,
        R::AdmissionTemporaryBytesGlobalMax,
        R::PeakLiveBytesPerAdmissionMax,
        R::AdmissionPeakLiveBytesGlobalMax,
        R::EngineLiveBytesGlobalMax,
    ];
    let mut parents = kinds.map(|r| {
        ResourceAccount::new(SlotIndex::new(0), Generation::INITIAL, r, l.get(r).unwrap())
    });
    for parent in &mut parents {
        parent.try_reserve(child).unwrap().commit();
    }
    let t = nested();
    let g = support::start(None);
    let ledger = AdmissionLedger::new(SlotIndex::new(0), Generation::INITIAL, 0, heap, 0, 0, 0, 0);
    let mut c = ValidatedThingCursor::from_thing(&t, &cfg, ledger);
    let proof = loop {
        assert!(support::counts().live as u64 <= heap);
        for p in &parents {
            assert_eq!(p.used(), child);
        }
        match c.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(n) => c = n,
            ValidatedThingProgress::Complete(v) => break v,
            _ => panic!(),
        }
    };
    let mut read = proof.into_property_read();
    loop {
        assert!(support::counts().live as u64 <= heap);
        match read.step(&mut budget(4), false).unwrap() {
            ValidatedPropertyReadStep::Ready(_) => read.acknowledge(),
            ValidatedPropertyReadStep::Pending => {}
            ValidatedPropertyReadStep::Done => break,
        }
    }
    drop(read);
    assert_eq!(support::counts().live, 0);
    for parent in &mut parents {
        assert!(parent.release_committed(child));
        assert_eq!(parent.used(), 0);
    }
    drop(g);
    for r in kinds {
        let mut p = ResourceAccount::new(SlotIndex::new(0), Generation::INITIAL, r, child - 1);
        assert!(p.try_reserve(child).is_none());
        assert_eq!(p.used(), 0);
    }
    // A concurrently occupied parent rejects another complete child allowance
    // before either input inspection or allocation, then accepts after release.
    let mut p = ResourceAccount::new(
        SlotIndex::new(0),
        Generation::INITIAL,
        R::EngineLiveBytesGlobalMax,
        child,
    );
    p.try_reserve(child).unwrap().commit();
    assert!(p.try_reserve(child).is_none());
    assert!(p.release_committed(child));
    p.try_reserve(child).unwrap().commit();
    assert!(p.release_committed(child));
    let _ = core::mem::size_of::<ResourceLimits>();
    println!(
        "logical TD current/return slots={inline}; capped heap={heap}; paired five scopes={child}"
    );
}

#[test]
fn fixed_pool_runtime_prepays_alignment_metadata_and_unused_allocator_capacity() {
    let t = nested(); // external typed source, provisioned before bounded entry
    let small = normalization_source(40);
    let gateway = normalization_source(16360);
    let small_limits = GatewayDefaultV1::limits()
        .clone()
        .with_limit(R::UriTemplateSourceBytesMax, Some(64));
    for (t, limits) in [
        (&t, GatewayDefaultV1::limits()),
        (&t, BenchmarkStaticReferenceV1::limits()),
        (&small, &small_limits),
        (&gateway, GatewayDefaultV1::limits()),
    ] {
        let cfg = ValidatedThingAdmissionConfig::try_from_limits(limits).unwrap();
        let allocator = support::pool_owner_layout();
        let inline = 2 * layout_catalog()
            .iter()
            .take(4)
            .map(|r| r.size as u64)
            .max()
            .unwrap();
        let kinds = [
            R::AdmissionTemporaryBytesPerOperationMax,
            R::AdmissionTemporaryBytesGlobalMax,
            R::PeakLiveBytesPerAdmissionMax,
            R::AdmissionPeakLiveBytesGlobalMax,
            R::EngineLiveBytesGlobalMax,
        ];
        let mut parents = kinds.map(|r| {
            ResourceAccount::new(
                SlotIndex::new(0),
                Generation::INITIAL,
                r,
                limits.get(r).unwrap(),
            )
        });
        // Caller-reserved pool backing plus metadata is startup storage. The
        // whole arena (including gaps/unused capacity) is charged once; child
        // Layout reservations delegate its capacity instead of adding it again.
        for p in &mut parents {
            p.try_reserve(allocator.size() as u64).unwrap().commit();
            p.try_reserve(inline).unwrap().commit();
        }
        let g = support::start_pool();
        let proof = validate(t, limits, 4096).unwrap();
        let mut read = proof.into_property_read();
        drive(&mut read, 4).unwrap();
        let trace = read.trace();
        let actual = support::counts();
        let pool = support::pool_observations();
        assert_eq!(pool.live, actual.live);
        assert_eq!(pool.peak, actual.peak);
        assert_eq!(pool.allocations, actual.allocations);
        assert!(pool.occupied_span <= allocator.size());
        assert!(actual.peak as u64 <= cfg.controlled_heap_envelope());
        let (requests, n) = support::requests();
        assert_eq!(n as u64, trace.allocations);
        assert!(
            requests[..n]
                .iter()
                .all(|&(bytes, align)| bytes != 0 && align <= allocator.align())
        );
        drop(read);
        assert_eq!(support::counts().live, 0);
        assert_eq!(support::pool_observations().live, 0);
        drop(g);
        for p in &mut parents {
            assert!(p.release_committed(inline));
            // The pre-existing allocator root remains physically reserved.
            assert_eq!(p.used(), allocator.size() as u64);
        }
        println!(
            "fixed allocator startup layout={allocator:?}; child peak={}; arena span={}; alignment padding={}",
            pool.peak, pool.occupied_span, pool.alignment_padding
        );
    }
}
