//! Observe existing compiler callbacks; implement no aggregate or TD semantics.
mod inline;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    mem,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
};

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactRole, BindingCandidate, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerStep, BindingConfigurationDigest, BindingGeneration,
    BindingId, HostBindingCompilerRegistration, LogicalInteractionPlan, PlanId, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_property_read_binding_fixture::{MockArtifact, MockCompiler, MockCompilerCursor};

// Single-threaded binary. No formatting, allocation or fallible injection inside
// the observer. Each bounded window records the actual request Layouts.
static OBSERVE: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_SIZES: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static ALLOC_ALIGNS: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static FREE_SIZES: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static FREE_ALIGNS: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];

struct Observer;
#[global_allocator]
static ALLOCATOR: Observer = Observer;

// SAFETY: delegate unchanged Layouts and pointers to System. Observation uses
// only atomics and never alters allocator results or ownership.
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if OBSERVE.load(SeqCst) {
            let i = ALLOCS.fetch_add(1, SeqCst);
            if i < ALLOC_SIZES.len() {
                ALLOC_SIZES[i].store(layout.size(), SeqCst);
                ALLOC_ALIGNS[i].store(layout.align(), SeqCst);
            }
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if OBSERVE.load(SeqCst) {
            let i = FREES.fetch_add(1, SeqCst);
            if i < FREE_SIZES.len() {
                FREE_SIZES[i].store(layout.size(), SeqCst);
                FREE_ALIGNS[i].store(layout.align(), SeqCst);
            }
        }
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Requests {
    allocations: Vec<(usize, usize)>,
    releases: Vec<(usize, usize)>,
}

fn observe<T>(callback: impl FnOnce() -> T) -> (T, Requests) {
    ALLOCS.store(0, SeqCst);
    FREES.store(0, SeqCst);
    OBSERVE.store(true, SeqCst);
    let result = callback();
    OBSERVE.store(false, SeqCst);
    let allocations = ALLOCS.load(SeqCst);
    let releases = FREES.load(SeqCst);
    assert!(allocations <= ALLOC_SIZES.len() && releases <= FREE_SIZES.len());
    let requests = Requests {
        allocations: (0..allocations)
            .map(|i| (ALLOC_SIZES[i].load(SeqCst), ALLOC_ALIGNS[i].load(SeqCst)))
            .collect(),
        releases: (0..releases)
            .map(|i| (FREE_SIZES[i].load(SeqCst), FREE_ALIGNS[i].load(SeqCst)))
            .collect(),
    };
    (result, requests)
}

fn layout<T>() -> (usize, usize) {
    (mem::size_of::<T>(), mem::align_of::<T>())
}

fn main() {
    let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(1), Generation::INITIAL),
        ThingId::from("urn:test:compiler-admission"),
        "temperature".into(),
        1,
        "mock://sensor/temperature".into(),
        Some("application/json".into()),
        None,
    )
    .unwrap();
    let candidate = BindingCandidate::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([0x52; 32]),
        compatibility,
        0,
        0,
    );
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
    let target_bytes = plan.resolved_target().len();
    // Provisioning the plan and compiler registration is outside observation.
    let portable = MockCompiler::new(compatibility);
    let host = HostBindingCompilerRegistration::new(MockCompiler::new(compatibility));
    let (static_bounds, static_bounds_requests) = observe(|| portable.bounds(&input).unwrap());
    let (host_bounds, host_bounds_requests) = observe(|| host.bounds(&input).unwrap());
    assert_eq!(static_bounds, host_bounds);
    assert!(static_bounds_requests.allocations.is_empty());
    assert!(host_bounds_requests.allocations.is_empty());
    assert_eq!(host_bounds.artifact().retained_bytes(), target_bytes as u64);
    assert_eq!(
        host_bounds.cursor_bytes(),
        mem::size_of::<MockCompilerCursor>() as u64
    );
    assert_eq!(host_bounds.temporary_bytes(), 0);

    let (static_cursor, static_start) = observe(|| portable.start(&input).unwrap());
    assert!(static_start.allocations.is_empty());
    let (host_cursor, host_start) = observe(|| host.start(&input).unwrap());
    assert_eq!(host_start.allocations, [layout::<MockCompilerCursor>()]);
    assert!(host_start.releases.is_empty());

    // Direct Core observation, not a claimed aggregate zero-budget violation:
    // the underlying compiler does no work, but the erased adapter reboxes it.
    let mut zero = WorkBudget::new();
    let (pending, host_pending) = observe(|| host.step(&input, host_cursor, &mut zero));
    let BindingCompilerStep::Pending(host_cursor) = pending else {
        panic!("mock must suspend without compiler credit")
    };
    assert_eq!(host_pending.allocations, [layout::<MockCompilerCursor>()]);
    assert_eq!(host_pending.releases, host_pending.allocations);
    assert_eq!(zero.remaining(WorkClass::BindingPolls), 0);

    let mut static_credit = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    let (static_result, static_step) =
        observe(|| portable.step(&input, static_cursor, &mut static_credit));
    let BindingCompilerStep::Complete(static_output) = static_result else {
        panic!("static mock must complete with one compiler unit")
    };
    assert_eq!(static_step.allocations, [(target_bytes, 1)]);
    assert!(static_step.releases.is_empty());

    let mut host_credit = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    let (host_result, host_step) = observe(|| host.step(&input, host_cursor, &mut host_credit));
    let BindingCompilerStep::Complete(host_output) = host_result else {
        panic!("erased mock must complete with one compiler unit")
    };
    assert_eq!(
        host_step.allocations,
        [(target_bytes, 1), layout::<MockArtifact>()]
    );
    assert_eq!(host_step.releases, [layout::<MockCompilerCursor>()]);
    assert_eq!(host_output.artifact().footprint(), host_bounds.artifact());
    assert_eq!(static_output.artifact().footprint(), host_bounds.artifact());
    assert_eq!(
        static_output.artifact().payload().target(),
        Some(plan.resolved_target())
    );
    assert_eq!(
        host_output
            .artifact()
            .try_payload::<MockArtifact>(compatibility)
            .unwrap()
            .target(),
        Some(plan.resolved_target()),
    );

    let (_, static_drop) = observe(|| drop(static_output));
    let (_, host_drop) = observe(|| drop(host_output));
    assert!(static_drop.allocations.is_empty() && host_drop.allocations.is_empty());
    assert_eq!(static_drop.releases, static_step.allocations);
    assert_eq!(host_drop.releases, host_step.allocations);
    println!(
        "bounds: target={target_bytes}, cursor={}, temporary=0",
        host_bounds.cursor_bytes()
    );
    println!("static start: {static_start:?}; static complete: {static_step:?}");
    println!("Host start: {host_start:?}; Host Pending: {host_pending:?}");
    println!("Host complete: {host_step:?}; Host output release: {host_drop:?}");
    println!(
        "Host artifact erasure delta: {} requested bytes; no aggregate or admission claim",
        mem::size_of::<MockArtifact>()
    );
    inline::compare(plan, candidate);
}
