//! Single-threaded downstream witness; observation is never allocation policy.
use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, Generation, ResourceKind, SlotIndex, WorkBudget, WorkClass,
};
use consumer_compiler_support_probe::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
};

static OBSERVE: AtomicBool = AtomicBool::new(false);
static FAIL_NEXT: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static SIZES: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static ALIGNS: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static FREE_SIZES: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static FREE_ALIGNS: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
struct Observer;
#[global_allocator]
static ALLOCATOR: Observer = Observer;
// SAFETY: unchanged System allocation/free, except one explicitly armed null
// return in the known reservation call. No formatting or allocation in hooks.
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if OBSERVE.load(SeqCst) {
            let n = ALLOCS.fetch_add(1, SeqCst);
            assert!(n < 8);
            SIZES[n].store(layout.size(), SeqCst);
            ALIGNS[n].store(layout.align(), SeqCst);
        }
        if FAIL_NEXT.swap(false, SeqCst) {
            return ptr::null_mut();
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if OBSERVE.load(SeqCst) {
            let n = FREES.fetch_add(1, SeqCst);
            assert!(n < 8);
            FREE_SIZES[n].store(layout.size(), SeqCst);
            FREE_ALIGNS[n].store(layout.align(), SeqCst);
        }
        unsafe {
            System.dealloc(pointer, layout);
        }
    }
}
#[derive(Debug, Eq, PartialEq)]
struct Requests {
    allocations: Vec<(usize, usize)>,
    releases: Vec<(usize, usize)>,
}
fn observe<T>(f: impl FnOnce() -> T) -> (T, Requests) {
    ALLOCS.store(0, SeqCst);
    FREES.store(0, SeqCst);
    OBSERVE.store(true, SeqCst);
    let result = f();
    OBSERVE.store(false, SeqCst);
    assert!(
        !FAIL_NEXT.load(SeqCst),
        "failure was not injected at the intended site"
    );
    (
        result,
        Requests {
            allocations: (0..ALLOCS.load(SeqCst))
                .map(|n| (SIZES[n].load(SeqCst), ALIGNS[n].load(SeqCst)))
                .collect(),
            releases: (0..FREES.load(SeqCst))
                .map(|n| (FREE_SIZES[n].load(SeqCst), FREE_ALIGNS[n].load(SeqCst)))
                .collect(),
        },
    )
}
fn none(r: Requests) {
    assert!(r.allocations.is_empty() && r.releases.is_empty(), "{r:?}");
}
fn budget(polls: u64, cleanup: u64) -> WorkBudget {
    WorkBudget::new()
        .with_remaining(WorkClass::BindingPolls, polls)
        .with_remaining(WorkClass::CleanupItems, cleanup)
}
fn ledger(bytes: u64) -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(3),
        Generation::INITIAL,
        100,
        100,
        0,
        bytes,
        0,
        100,
    )
}
fn plan(id: u32, target: &str) -> LogicalInteractionPlan {
    LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(id), Generation::INITIAL),
        ThingId::from("urn:compiler:support"),
        "temperature".into(),
        1,
        target.into(),
        None,
        None,
    )
    .unwrap()
}
fn candidate(id: BindingRegistrationIdentity) -> BindingCandidate {
    BindingCandidate::new(
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        0,
        0,
    )
}
fn make_checked(scenario: Scenario, representation: Representation) -> CheckedRegistration {
    let complete = registration(CAPACITY, scenario).unwrap();
    let id = complete.identity();
    capture(Some(complete), id, representation)
        .unwrap_or_else(|_| panic!("closed capture rejected"))
}
fn identity_negatives() {
    let complete = registration(CAPACITY, Scenario::Success).unwrap();
    let id = complete.identity();
    drop(complete);
    let mismatches = [
        BindingRegistrationIdentity::new(
            BindingId::new(8),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            0,
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation().checked_next().unwrap(),
            id.configuration(),
            id.artifact_compatibility(),
            0,
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation(),
            BindingConfigurationDigest::new([9; 32]),
            id.artifact_compatibility(),
            0,
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            BindingArtifactCompatibility::new([9; 16]),
            0,
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            1,
        ),
    ];
    for expected in mismatches {
        let complete = registration(CAPACITY, Scenario::Success).unwrap();
        let before = CALLBACKS.load(SeqCst);
        let (result, r) = observe(|| capture(Some(complete), expected, Representation::Static));
        none(r);
        let (original, error) = result.err().expect("identity mismatch accepted");
        assert_eq!(error, CaptureError::Mismatch);
        assert_eq!(original.unwrap().identity(), id);
        assert_eq!(CALLBACKS.load(SeqCst), before);
    }
    let before = CALLBACKS.load(SeqCst);
    assert_eq!(
        capture(None, id, Representation::Static).err().unwrap().1,
        CaptureError::Absent
    );
    assert_eq!(CALLBACKS.load(SeqCst), before);
    for (complete, expected_error) in [
        (producer_only_registration(), CaptureError::Capability),
        (
            unsupported_registration(),
            CaptureError::UnsupportedImplementation,
        ),
        (
            mismatched_configuration_registration(),
            CaptureError::Configuration,
        ),
        (
            legacy_host_registration(),
            CaptureError::UnsupportedRepresentation,
        ),
    ] {
        let expected = complete.identity();
        let before = CALLBACKS.load(SeqCst);
        let (result, r) = observe(|| {
            capture(
                Some(complete),
                expected,
                Representation::ReservedHostPrototype,
            )
        });
        none(r);
        let (original, error) = result
            .err()
            .expect("unsupported compiler/configuration/transport accepted");
        assert_eq!(error, expected_error);
        assert_eq!(original.unwrap().identity(), expected);
        assert_eq!(CALLBACKS.load(SeqCst), before);
    }
    let owner = make_checked(Scenario::Success, Representation::Static);
    let other = registration(CAPACITY / 2, Scenario::Success).unwrap();
    let source = plan(99, "mock://sensor/temperature");
    let borrowed_from_other = BindingCompilerInput::new(
        &source,
        candidate(other.identity()),
        BindingArtifactRole::ConsumerCall,
    );
    let before = CALLBACKS.load(SeqCst);
    let (result, r) = observe(|| owner.bounds(&borrowed_from_other, &mut budget(1, 0)));
    none(r);
    assert!(result.is_err());
    assert_eq!(CALLBACKS.load(SeqCst), before);
    assert_eq!(other.identity().binding_id(), owner.identity().binding_id());
    for capacity in [0, CAPACITY + 1] {
        assert!(registration(capacity, Scenario::Success).is_err());
    }
}
fn static_success() -> BindingArtifactIdentity {
    let checked = make_checked(Scenario::Success, Representation::Static);
    let plan = plan(1, "mock://sensor/temperature");
    let envelope = {
        let input = BindingCompilerInput::new(
            &plan,
            candidate(checked.identity()),
            BindingArtifactRole::ConsumerCall,
        );
        let before = CALLBACKS.load(SeqCst);
        assert!(checked.bounds(&input, &mut budget(0, 0)).is_err());
        assert_eq!(CALLBACKS.load(SeqCst), before);
        let (bounds, r) = observe(|| checked.bounds(&input, &mut budget(1, 0)).unwrap());
        none(r);
        let (cursor, r) = observe(|| checked.start_static(&input, &mut budget(1, 1)).unwrap());
        none(r);
        let before = CALLBACKS.load(SeqCst);
        let mut cursor = cursor;
        for polls in [0, 1, 0, 1] {
            let (step, r) = observe(|| cursor.step(&input, &mut budget(polls, 0)));
            none(r);
            cursor = match step {
                StaticStep::Pending(next) => next,
                _ => panic!("unpaid static progress"),
            };
        }
        assert_eq!(CALLBACKS.load(SeqCst), before);
        let (step, r) = observe(|| cursor.step(&input, &mut budget(2, 0)));
        none(r);
        let cursor = match step {
            StaticStep::Pending(cursor) => cursor,
            _ => panic!("first static step"),
        };
        let (step, r) = observe(|| cursor.step(&input, &mut budget(2, 0)));
        none(r);
        let output = match step {
            StaticStep::Complete(output) => output,
            _ => panic!("static completion"),
        };
        assert_eq!(output.artifact().footprint(), bounds.artifact());
        let c = candidate(checked.identity());
        let identity = BindingArtifactIdentity::new(
            PlanSetGeneration::INITIAL,
            plan.plan_id(),
            c.binding_id(),
            c.binding_generation(),
            c.configuration(),
            c.compatibility(),
            BindingArtifactRole::ConsumerCall,
        );
        let envelope =
            BindingArtifactEnvelope::try_new(identity, bounds.artifact(), output.into_artifact())
                .unwrap_or_else(|_| panic!("envelope rejected"));
        envelope
    };
    drop(checked);
    drop(plan);
    assert_eq!(
        envelope.artifact().payload().target(),
        "mock://sensor/temperature"
    );
    let identity = envelope.identity();
    let before = ARTIFACT_DROPS.load(SeqCst);
    let (_, r) = observe(|| drop(envelope));
    none(r);
    assert_eq!(ARTIFACT_DROPS.load(SeqCst), before + 1);
    identity
}
fn host_success() -> BindingArtifactIdentity {
    let checked = make_checked(Scenario::Success, Representation::ReservedHostPrototype);
    let plan = plan(1, "mock://sensor/temperature");
    let (output, bounds, layout) = {
        let input = BindingCompilerInput::new(
            &plan,
            candidate(checked.identity()),
            BindingArtifactRole::ConsumerCall,
        );
        let bounds = checked.bounds(&input, &mut budget(1, 0)).unwrap();
        let layout = ReservedCursor::slab_layout();
        let (reserved, r) = observe(|| {
            reserve(
                &checked,
                &input,
                ledger(layout.size() as u64),
                layout.size() as u64,
                &mut budget(0, 1),
            )
            .unwrap_or_else(|_| panic!("reserve failed"))
        });
        assert_eq!(r.allocations, [(layout.size(), layout.align())]);
        assert!(r.releases.is_empty());
        let address = reserved.allocation_address();
        assert_eq!(reserved.live_bytes(), layout.size() as u64);
        let (cursor, r) = observe(|| {
            reserved
                .start(&input, &mut budget(1, 1))
                .unwrap_or_else(|_| panic!("start failed"))
        });
        none(r);
        let before = CALLBACKS.load(SeqCst);
        let mut cursor = cursor;
        for polls in [0, 1, 0, 1] {
            let (step, r) = observe(|| cursor.step(&input, &mut budget(polls, 0)));
            none(r);
            cursor = match step {
                HostStep::Pending(next) => next,
                _ => panic!("unpaid Host progress"),
            };
            assert_eq!(cursor.allocation_address(), address);
        }
        assert_eq!(CALLBACKS.load(SeqCst), before);
        let (step, r) = observe(|| cursor.step(&input, &mut budget(2, 0)));
        none(r);
        let cursor = match step {
            HostStep::Pending(cursor) => cursor,
            _ => panic!("first Host step"),
        };
        assert_eq!(cursor.allocation_address(), address);
        let (step, r) = observe(|| cursor.step(&input, &mut budget(2, 0)));
        none(r);
        let output = match step {
            HostStep::Complete(output) => output,
            _ => panic!("Host completion"),
        };
        assert_eq!(output.allocation_address(), address);
        assert_eq!(output.binding_footprint(), bounds.artifact());
        let expected = candidate(checked.identity());
        assert_eq!(output.identity().configuration(), expected.configuration());
        (output, bounds, layout)
    };
    drop(checked);
    drop(plan);
    assert!(output.try_payload::<u8>(COMPATIBILITY).is_none());
    assert!(
        output
            .try_payload::<InlineArtifact>(BindingArtifactCompatibility::new([0; 16]))
            .is_none()
    );
    assert_eq!(
        output
            .try_payload::<InlineArtifact>(COMPATIBILITY)
            .unwrap()
            .target(),
        "mock://sensor/temperature"
    );
    let identity = output.identity();
    let before = ARTIFACT_DROPS.load(SeqCst);
    let (ledger, r) = observe(|| output.reclaim());
    assert!(r.allocations.is_empty());
    assert_eq!(r.releases, [(layout.size(), layout.align())]);
    assert_eq!(ledger.live_bytes(), 0);
    assert_eq!(ledger.peak_live_bytes(), layout.size() as u64);
    assert_eq!(ledger.largest_contiguous_allocation(), layout.size() as u64);
    assert_eq!(ARTIFACT_DROPS.load(SeqCst), before + 1);
    println!(
        "reserved Host: slab=({}, {}), owner=({}, {}), binding payload={}; one allocation retained through Pending and Complete",
        layout.size(),
        layout.align(),
        ReservedCursor::owner_layout().size(),
        ReservedCursor::owner_layout().align(),
        bounds.artifact().retained_bytes()
    );
    identity
}
fn host_negatives() {
    let checked = make_checked(Scenario::Success, Representation::ReservedHostPrototype);
    let plan = plan(3, "mock://sensor/temperature");
    let input = BindingCompilerInput::new(
        &plan,
        candidate(checked.identity()),
        BindingArtifactRole::ConsumerCall,
    );
    let size = ReservedCursor::slab_layout().size() as u64;
    for (capacity, largest, cleanup) in [(size - 1, size, 1), (size, size - 1, 1), (size, size, 0)]
    {
        let before = CALLBACKS.load(SeqCst);
        let (result, r) = observe(|| {
            reserve(
                &checked,
                &input,
                ledger(capacity),
                largest,
                &mut budget(0, cleanup),
            )
        });
        none(r);
        assert_eq!(
            result
                .err()
                .expect("negative reservation accepted")
                .0
                .live_bytes(),
            0
        );
        assert_eq!(CALLBACKS.load(SeqCst), before);
    }
    // A second acquisition can fail while the first complete owner remains held.
    let retained = reserve(&checked, &input, ledger(size), size, &mut budget(0, 1))
        .unwrap_or_else(|_| panic!("retained reserve"));
    let address = retained.allocation_address();
    let before = CALLBACKS.load(SeqCst);
    let mut original = ledger(size);
    original
        .try_reserve_source(ResourceKind::DocumentBytesMax, 7)
        .unwrap()
        .commit();
    let (result, r) = observe(|| {
        FAIL_NEXT.store(true, SeqCst);
        reserve(&checked, &input, original, size, &mut budget(0, 1))
    });
    assert_eq!(
        r.allocations,
        [(size as usize, ReservedCursor::slab_layout().align())]
    );
    assert!(r.releases.is_empty());
    assert_eq!(
        result
            .err()
            .expect("null allocation accepted")
            .0
            .live_bytes(),
        7
    );
    assert_eq!(retained.allocation_address(), address);
    assert_eq!(retained.live_bytes(), size);
    assert_eq!(CALLBACKS.load(SeqCst), before);
    drop(retained);
    let cursor = reserve(&checked, &input, ledger(size), size, &mut budget(0, 1))
        .unwrap_or_else(|_| panic!("abort reserve"));
    let cursor = cursor
        .start(&input, &mut budget(1, 1))
        .unwrap_or_else(|_| panic!("abort start"));
    let before = ABORTS.load(SeqCst);
    let (_, r) = observe(|| drop(cursor));
    assert!(r.allocations.is_empty());
    assert_eq!(r.releases.len(), 1);
    assert_eq!(ABORTS.load(SeqCst), before + 1);
    for scenario in [Scenario::StartFailure, Scenario::StepFailure] {
        let checked = make_checked(scenario, Representation::ReservedHostPrototype);
        let input = BindingCompilerInput::new(
            &plan,
            candidate(checked.identity()),
            BindingArtifactRole::ConsumerCall,
        );
        let cursor = reserve(&checked, &input, ledger(size), size, &mut budget(0, 1))
            .unwrap_or_else(|_| panic!("failure reserve"));
        let before = ABORTS.load(SeqCst);
        let cursor = if scenario == Scenario::StartFailure {
            let (result, r) = observe(|| cursor.start(&input, &mut budget(1, 1)));
            none(r);
            result.err().expect("start failure disappeared").0
        } else {
            let cursor = cursor
                .start(&input, &mut budget(1, 1))
                .unwrap_or_else(|_| panic!("failure start"));
            let HostStep::Pending(cursor) = cursor.step(&input, &mut budget(2, 0)) else {
                panic!("failure first step")
            };
            let address = cursor.allocation_address();
            let (step, r) = observe(|| cursor.step(&input, &mut budget(2, 0)));
            none(r);
            let HostStep::Failed(_, cursor) = step else {
                panic!("expected failure")
            };
            assert_eq!(cursor.allocation_address(), address);
            let calls = CALLBACKS.load(SeqCst);
            let (step, r) = observe(|| cursor.step(&input, &mut budget(2, 0)));
            none(r);
            let HostStep::Failed(_, cursor) = step else {
                panic!("exhausted remainder refilled")
            };
            assert_eq!(CALLBACKS.load(SeqCst), calls);
            cursor
        };
        let (_, r) = observe(|| drop(cursor));
        assert!(r.allocations.is_empty());
        assert_eq!(r.releases.len(), 1);
        assert_eq!(
            ABORTS.load(SeqCst),
            before + usize::from(scenario == Scenario::StepFailure)
        );
    }
}
fn main() {
    identity_negatives();
    assert_eq!(static_success(), host_success());
    host_negatives();
    println!(
        "passed: complete-registration support capture, configuration/impostor/legacy negatives, paid callbacks, static source independence, held Host storage, null acquisition and retained-owner cleanup"
    );
}
