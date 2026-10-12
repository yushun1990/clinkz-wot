//! Production aggregate evidence through the public Planning boundary.
extern crate alloc;
use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceAccount, ResourceKind as K, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass as W,
};
use clinkz_wot_planning::*;
use clinkz_wot_td::{
    ValidatedThing, ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress,
    thing::Thing,
};
use consumer_compiler_support_probe::production_static_registration;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[allow(dead_code)]
#[path = "../../../../planning/src/consumer_property_read.rs"]
mod checked_program;

#[derive(Clone, Copy, Default)]
struct Record {
    pointer: usize,
    bytes: usize,
}
#[derive(Clone, Copy)]
struct Trace {
    armed: bool,
    requests: usize,
    fail: usize,
    live: usize,
    peak: usize,
    largest: usize,
    records: [Record; 128],
}
const EMPTY: Trace = Trace {
    armed: false,
    requests: 0,
    fail: usize::MAX,
    live: 0,
    peak: 0,
    largest: 0,
    records: [Record {
        pointer: 0,
        bytes: 0,
    }; 128],
};
std::thread_local! {static TRACE:Cell<Trace>=const{Cell::new(EMPTY)};}
struct Observer;
#[global_allocator]
static ALLOCATOR: Observer = Observer;
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let fail = TRACE
            .try_with(|t| {
                let mut s = t.get();
                if !s.armed {
                    return false;
                }
                let index = s.requests;
                s.requests += 1;
                s.largest = s.largest.max(l.size());
                t.set(s);
                index == s.fail
            })
            .unwrap_or(false);
        if fail {
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            let _ = TRACE.try_with(|t| {
                let mut s = t.get();
                if s.armed {
                    let i = s
                        .records
                        .iter()
                        .position(|r| r.pointer == 0)
                        .expect("bounded trace");
                    s.records[i] = Record {
                        pointer: p as usize,
                        bytes: l.size(),
                    };
                    s.live += l.size();
                    s.peak = s.peak.max(s.live);
                    t.set(s);
                }
            });
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = TRACE.try_with(|t| {
            let mut s = t.get();
            if let Some(i) = s.records.iter().position(|r| r.pointer == p as usize) {
                s.live -= s.records[i].bytes;
                s.records[i] = Record::default();
                t.set(s);
            }
        });
        unsafe { System.dealloc(p, l) };
    }
}
fn observe(fail: usize) {
    TRACE.with(|t| {
        t.set(Trace {
            armed: true,
            fail,
            ..EMPTY
        })
    });
}
fn trace() -> Trace {
    TRACE.with(Cell::get)
}
fn disarm() {
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = false;
        t.set(s);
    });
}
fn budget(n: u64) -> WorkBudget {
    W::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn source() -> Thing {
    serde_json::from_str(r#"{
 "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"aggregate","id":"urn:aggregate:id","base":"https://peer.test/a/",
 "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
 "properties":{
  "a-read":{"forms":[{"href":"skip","op":[]},{"href":"../first","scopes":["二",""]},{"href":"second","contentCoding":"gzip","subprotocol":"one"}]},
  "b-empty":{"forms":[{"href":"skip","op":[]}]},
  "c-温度":{"forms":[{"href":"skip","op":["writeproperty"]},{"href":"last","scopes":["three"]}]},
  "z-empty":{"forms":[]}
 }}"#).unwrap()
}
fn proof(source: &Thing) -> ValidatedThing<'_> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut c = ValidatedThingCursor::from_thing(source, &config, ledger(0));
    for _ in 0..100000 {
        match c.step(&mut budget(4096), false) {
            ValidatedThingProgress::Pending(next) => c = next,
            ValidatedThingProgress::Complete(p) => return p,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        }
    }
    panic!("TD stalled")
}
fn ledger(runtime: u64) -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(30),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        runtime,
        0,
        8 << 20,
    )
}
fn config() -> ConsumerPropertyReadBuildConfig {
    ConsumerPropertyReadBuildConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap()
}
fn identity(c: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(203),
        BindingGeneration::INITIAL.checked_next().unwrap(),
        c.configuration(),
        c.compatibility(),
        19,
    )
}
fn slot<A: 'static>(
    c: &ConsumerPropertyReadBuildConfig,
) -> (ConsumerPropertyReadCleanupSlot<A>, ResourceAccount) {
    let bytes = ConsumerPropertyReadCleanupSlot::<A>::required_capacity();
    let mut account = ResourceAccount::new(
        SlotIndex::new(80),
        Generation::INITIAL,
        K::CleanupBytesMax,
        bytes,
    );
    let slot = ConsumerPropertyReadCleanupSlot::try_new(c, account.try_reserve(bytes).unwrap())
        .unwrap_or_else(|_| panic!("slot"));
    (slot, account)
}
fn finish<'td, 'env, R: ConsumerPropertyReadRegistration>(
    mut build: ConsumerPropertyReadBuild<'td, 'env, R>,
    runtime: u64,
    fail: usize,
) -> Result<
    ConsumerPropertyReadDraft<R::Artifact>,
    (CoreError, ConsumerPropertyReadRollback<R::Artifact>),
> {
    let mut emitted = false;
    for tick in 0..100000 {
        // Every move and fresh zero budget preserves the continuation.
        if tick % 7 == 0 {
            build = match build.step(&mut budget(0), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => next,
                _ => panic!("zero budget advanced"),
            };
        }
        match build.step(&mut budget(4096), false) {
            ConsumerPropertyReadBuildStep::Pending(next) => build = next,
            ConsumerPropertyReadBuildStep::Preflight {
                build: mut next,
                requirements,
            } => {
                assert!(!emitted);
                emitted = true;
                assert_eq!(requirements.registration.binding_id(), BindingId::new(203));
                for _ in 0..3 {
                    next = match next.step(&mut budget(4096), false) {
                        ConsumerPropertyReadBuildStep::Pending(next) => next,
                        _ => panic!("waiting replay"),
                    };
                }
                observe(fail);
                build = next
                    .admit_runtime(ledger(runtime))
                    .unwrap_or_else(|_| panic!("admit"));
            }
            ConsumerPropertyReadBuildStep::Complete(draft) => return Ok(draft),
            ConsumerPropertyReadBuildStep::Failed { cause, cleanup } => {
                return Err((cause, cleanup.unwrap()));
            }
        }
    }
    panic!("aggregate stalled")
}
fn reclaim<A: 'static>(mut owner: ConsumerPropertyReadRollback<A>) -> CoreError {
    let first = owner.cause().clone();
    for tick in 0..100000 {
        if trace().armed {
            assert_eq!(owner.live_bytes(), trace().live as u64);
            if let Some((live, peak, largest)) = owner.observations() {
                assert_eq!(live, owner.live_bytes());
                assert!(peak >= live);
                assert!(largest <= peak);
            }
        }
        let mut b = budget(if tick % 5 == 0 { 0 } else { 31 });
        let before = b.remaining(W::CodecInputBytes);
        match owner.step(&mut b) {
            ConsumerPropertyReadRollbackStep::Pending(next) => owner = next,
            ConsumerPropertyReadRollbackStep::Complete(cause) => {
                assert_eq!(cause, first);
                return cause;
            }
        }
        assert!(before - b.remaining(W::CodecInputBytes) <= 31);
    }
    panic!("cleanup stalled")
}
fn success<R: ConsumerPropertyReadRegistration>(
    r: R,
    target: impl Fn(&BindingArtifact<R::Artifact>) -> &str,
) {
    let source = source();
    let c = config();
    let (mut root, mut account) = slot::<R::Artifact>(&c);
    let held = Box::new([7u8; 7]);
    let build = build_consumer_property_read_aggregate(
        proof(&source),
        &r,
        &c,
        PlanSetGeneration::INITIAL.checked_next().unwrap(),
        ledger(0),
        &mut root,
    )
    .unwrap();
    let draft = finish(build, 8 << 20, usize::MAX).unwrap_or_else(|(e, _)| panic!("{e:?}"));
    let memory = trace();
    disarm();
    assert_eq!(draft.registration_identity(), r.identity());
    drop(r);
    drop(source);
    assert_eq!(draft.property_count(), 4);
    assert_eq!(draft.coordinate_count(), 3);
    let footprint = draft.footprint();
    assert_eq!(footprint.retained_bytes(), memory.live as u64);
    assert_eq!(footprint.largest_allocation, memory.largest as u64);
    assert!(footprint.peak_live_bytes >= memory.peak as u64);
    assert_eq!(footprint.retained_source_bytes, 0);
    assert_eq!(footprint.lazy_bytes, 0);
    for (name, index, expected) in [
        ("a-read", 1, "https://peer.test/first"),
        ("a-read", 2, "https://peer.test/a/second"),
        ("c-温度", 1, "https://peer.test/a/last"),
    ] {
        let selected = select_consumer_property_read_aggregate(
            &draft,
            name,
            &InteractionOptions::default().with_form_index(index),
        )
        .unwrap();
        assert_eq!(target(selected.artifact().artifact()), expected);
        assert_eq!(selected.plan().thing_id().as_str(), "urn:aggregate:id");
        assert_eq!(selected.candidate().registration_ordinal(), 0);
        assert_eq!(selected.candidate().candidate_order(), 0);
        assert_eq!(
            selected.artifact_ref().identity(),
            selected.artifact().identity()
        );
        assert_eq!(selected.security_name(), "none");
    }
    let selected =
        select_consumer_property_read_aggregate(&draft, "a-read", &InteractionOptions::default())
            .unwrap();
    assert_eq!(selected.plan().form_index(), 1);
    assert_eq!(selected.scopes()[0].as_ref(), "二");
    for (name, reason) in [
        ("missing", SelectionFailureReason::AffordanceMissing),
        ("b-empty", SelectionFailureReason::NoFormSupportsOperation),
        ("z-empty", SelectionFailureReason::NoFormSupportsOperation),
    ] {
        assert!(
            matches!(select_consumer_property_read_aggregate(&draft,name,&InteractionOptions::default()),Err(CoreError::Selection{reason:r,..}) if r==reason)
        );
    }
    assert!(matches!(
        select_consumer_property_read_aggregate(
            &draft,
            "a-read",
            &InteractionOptions::default().with_form_index(0)
        ),
        Err(CoreError::Selection {
            reason: SelectionFailureReason::StrictSelectionMismatch,
            ..
        })
    ));
    // Returning a source-free draft does not borrow the movable root. Its Drop
    // returns the cleanup lease even after the root has moved.
    let mut relocated = Box::new(root);
    drop(draft);
    let owner = relocated.take().unwrap();
    drop(owner);
    let owner = relocated.take().unwrap();
    let history = owner.observations().unwrap();
    assert_eq!(history.0, footprint.retained_bytes());
    reclaim(owner);
    let child_terminal_live = trace().live;
    assert_eq!(
        child_terminal_live,
        checked_program::test_support::layouts::<R::Artifact>()[0][0]
    );
    assert!(relocated.take().is_none());
    assert_eq!(trace().live, 0);
    assert_eq!(*held, [7; 7]);
    let bytes = account.used();
    assert!(bytes > 0);
    drop(relocated);
    assert!(account.release_committed(bytes));
    println!(
        "footprint={footprint:?}; heap_peak={}; requests={}",
        memory.peak, memory.requests
    );
    println!(
        "layouts[node,data,property,coordinate,reclaim,slot,draft]={:?}; cleanup_capacity={bytes}; cleanup_peak={}",
        checked_program::test_support::layouts::<R::Artifact>(),
        account.peak()
    );
    println!(
        "output_ledger_live_peak_largest={history:?}; after_children_live={child_terminal_live}; after_root_live=0"
    );
}
#[test]
fn static_public_aggregate_owns_every_coordinate_and_bounded_cleanup() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    success(production_static_registration(c, id), |a| {
        a.payload().target()
    });
}
#[cfg(feature = "std")]
#[test]
fn host_public_aggregate_owns_every_coordinate_and_bounded_cleanup() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    success(
        consumer_compiler_support_probe::production_host_registration(c, id),
        |a| {
            a.try_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                .unwrap()
                .target()
        },
    );
}

fn failures<R: ConsumerPropertyReadRegistration>(r: &R) {
    let source = source();
    let c = config();
    let (mut root, mut account) = slot::<R::Artifact>(&c);
    let draft = finish(
        build_consumer_property_read_aggregate(
            proof(&source),
            r,
            &c,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        8 << 20,
        usize::MAX,
    )
    .unwrap_or_else(|(e, _)| panic!("baseline {e:?}"));
    let requests = trace().requests;
    drop(draft);
    reclaim(root.take().unwrap());
    assert!(root.take().is_none());
    assert_eq!(trace().live, 0);
    disarm();
    for index in 0..requests {
        let held = Box::new([9u8; 17]);
        let result = finish(
            build_consumer_property_read_aggregate(
                proof(&source),
                r,
                &c,
                PlanSetGeneration::INITIAL,
                ledger(0),
                &mut root,
            )
            .unwrap(),
            8 << 20,
            index,
        );
        let (cause, owner) = result
            .err()
            .unwrap_or_else(|| panic!("allocation {index} unexpectedly succeeded"));
        assert!(matches!(
            cause,
            CoreError::Backpressure(_) | CoreError::Validation(_)
        ));
        reclaim(owner);
        assert!(root.take().is_none());
        assert_eq!(trace().live, 0, "leaked allocation at {index}");
        assert_eq!(*held, [9; 17]);
        assert!(account.used() > 0);
        disarm();
    }
    // A capped child rejects before its unauthorized first allocator request.
    let (_, owner) = finish(
        build_consumer_property_read_aggregate(
            proof(&source),
            r,
            &c,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        0,
        usize::MAX,
    )
    .err()
    .unwrap();
    assert_eq!(trace().requests, 0);
    reclaim(owner);
    assert!(root.take().is_none());
    disarm();
    let bytes = account.used();
    assert_eq!(account.peak(), bytes);
    drop(root);
    assert!(account.release_committed(bytes));
    println!("allocation_failures={requests}; committed_cleanup_peak={bytes}");
}
#[test]
fn static_every_actual_allocation_failure_keeps_other_owner_and_reclaims() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    failures(&production_static_registration(c, id));
}
#[cfg(feature = "std")]
#[test]
fn host_every_actual_allocation_failure_keeps_other_owner_and_reclaims() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    failures(&consumer_compiler_support_probe::production_host_registration(c, id));
}

#[test]
fn checked_config_and_guard_preserving_slot_rejections() {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(K::PlanCompileWorkUnitsPerStepMax, None));
    assert!(ConsumerPropertyReadBuildConfig::try_from_limits(&limits).is_err());
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(K::FormsPerThingMax, Some(u64::MAX));
    assert!(ConsumerPropertyReadBuildConfig::try_from_limits(&limits).is_err());
    let c = config();
    let bytes = ConsumerPropertyReadCleanupSlot::<ResolvedTargetArtifact>::required_capacity();
    let mut account = ResourceAccount::new(
        SlotIndex::new(80),
        Generation::INITIAL,
        K::CleanupBytesMax,
        bytes,
    );
    let guard = account.try_reserve(bytes - 1).unwrap();
    let (guard, _) = ConsumerPropertyReadCleanupSlot::<ResolvedTargetArtifact>::try_new(&c, guard)
        .err()
        .unwrap();
    assert_eq!(guard.bytes(), bytes - 1);
    drop(guard);
    assert_eq!(account.used(), 0);
    assert_eq!(account.peak(), bytes - 1);
    let mut wrong_kind = ResourceAccount::new(
        SlotIndex::new(80),
        Generation::INITIAL,
        K::CompiledRuntimeBytesPerThingMax,
        bytes,
    );
    let (guard, _) = ConsumerPropertyReadCleanupSlot::<ResolvedTargetArtifact>::try_new(
        &c,
        wrong_kind.try_reserve(bytes).unwrap(),
    )
    .err()
    .unwrap();
    assert_eq!(guard.kind(), K::CompiledRuntimeBytesPerThingMax);
    assert_eq!(guard.bytes(), bytes);
    drop(guard);
    assert_eq!(wrong_kind.used(), 0);
}

#[test]
fn wrong_phase_admission_returns_unchanged_build_and_child() {
    let source = source();
    let cfg = config();
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let r = production_static_registration(c, id);
    let (mut root, mut account) = slot::<ResolvedTargetArtifact>(&cfg);
    let build = build_consumer_property_read_aggregate(
        proof(&source),
        &r,
        &cfg,
        PlanSetGeneration::INITIAL,
        ledger(0),
        &mut root,
    )
    .unwrap();
    let (build, mut child, cause) = build.admit_runtime(ledger(7)).err().unwrap();
    assert!(matches!(cause, CoreError::Validation(_)));
    assert_eq!(child.live_bytes(), 0);
    assert_eq!(child.peak_live_bytes(), 0);
    child
        .try_reserve_persistent_runtime(K::CompiledRuntimeBytesPerThingMax, 7)
        .unwrap()
        .commit();
    assert!(child.release_persistent_runtime(7));
    assert!(
        child
            .try_reserve_persistent_runtime(K::CompiledRuntimeBytesPerThingMax, 8)
            .is_none()
    );
    let draft = finish(build, 8 << 20, usize::MAX).unwrap_or_else(|(e, _)| panic!("{e:?}"));
    drop(draft);
    reclaim(root.take().unwrap());
    assert!(root.take().is_none());
    disarm();
    let bytes = account.used();
    drop(root);
    assert!(account.release_committed(bytes));
}

fn traced_failures<R: checked_program::ConsumerPropertyReadRegistration>(
    factory: impl Fn() -> R,
    host: bool,
) {
    let source = source();
    let counts = checked_program::test_support::traced_after_admit(
        proof(&source),
        factory(),
        None,
        false,
        || observe(usize::MAX),
    );
    let requests = trace().requests;
    assert!(!counts.failed);
    assert_eq!(trace().live, 0);
    disarm();
    for index in 0..requests {
        let counts = checked_program::test_support::traced_after_admit(
            proof(&source),
            factory(),
            None,
            false,
            || observe(index),
        );
        assert!(counts.failed);
        if index < requests - if host { 3 } else { 0 } {
            assert_eq!(counts.starts, 0, "materialization allocation {index}");
        } else {
            assert_eq!(counts.bounds, 3);
            assert_eq!(counts.aborts, 0, "failed start never produced a cursor");
        }
        assert_eq!(trace().live, 0);
        disarm();
    }
}
#[test]
fn static_materialization_allocation_failures_have_zero_starts() {
    traced_failures(
        || {
            let c = ResolvedTargetCompiler::try_new(64).unwrap();
            let id = identity(&c);
            production_static_registration(c, id)
        },
        false,
    );
}
#[cfg(feature = "std")]
#[test]
fn host_materialization_allocation_failures_have_zero_starts() {
    traced_failures(
        || {
            let c = ResolvedTargetCompiler::try_new(64).unwrap();
            let id = identity(&c);
            consumer_compiler_support_probe::production_host_registration(c, id)
        },
        true,
    );
}

#[test]
fn all_bounds_barrier_and_compiler_remainders_use_the_checked_production_source() {
    let source = source();
    for spare in [0, 1] {
        let c = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&c);
        checked_program::test_support::ready_retries(
            proof(&source),
            production_static_registration(c, id),
            spare,
        );
        #[cfg(feature = "std")]
        {
            let c = ResolvedTargetCompiler::try_new(64).unwrap();
            let id = identity(&c);
            checked_program::test_support::ready_retries(
                proof(&source),
                consumer_compiler_support_probe::production_host_registration(c, id),
                spare,
            );
        }
    }
    for bad in [None, Some(1), Some(3)] {
        let c = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&c);
        let counts = checked_program::test_support::traced(
            proof(&source),
            production_static_registration(c, id),
            bad,
            false,
        );
        if bad.is_some() {
            assert!(counts.failed);
            assert_eq!(counts.starts, 0);
            assert_eq!(counts.bounds, bad.unwrap());
        } else {
            assert!(!counts.failed);
            assert_eq!(
                (counts.bounds, counts.starts, counts.steps, counts.aborts),
                (3, 3, 6, 0)
            );
        }
    }
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let counts = checked_program::test_support::traced(
        proof(&source),
        production_static_registration(c, id),
        None,
        true,
    );
    assert!(counts.failed);
    assert_eq!(
        (counts.bounds, counts.starts, counts.steps, counts.aborts),
        (3, 1, 1, 1)
    );
    #[cfg(feature = "std")]
    {
        let c = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&c);
        let counts = checked_program::test_support::traced(
            proof(&source),
            consumer_compiler_support_probe::production_host_registration(c, id),
            None,
            false,
        );
        assert_eq!(
            (counts.bounds, counts.starts, counts.steps, counts.aborts),
            (3, 3, 6, 0)
        );
        let c = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&c);
        let counts = checked_program::test_support::traced(
            proof(&source),
            consumer_compiler_support_probe::production_host_registration(c, id),
            None,
            true,
        );
        assert_eq!(
            (counts.bounds, counts.starts, counts.steps, counts.aborts),
            (3, 1, 1, 1)
        );
    }
}

fn invalid_inputs<R: ConsumerPropertyReadRegistration>(r: R) {
    let cfg = config();
    for case in 0..4 {
        let mut source = source();
        match case {
            0 => source.id = None,
            1 => {
                source
                    .properties
                    .as_mut()
                    .unwrap()
                    .get_mut("a-read")
                    .unwrap()
                    ._interaction
                    .forms[1]
                    .security = Some(Vec::new())
            }
            2 => {
                source
                    .properties
                    .as_mut()
                    .unwrap()
                    .get_mut("a-read")
                    .unwrap()
                    ._interaction
                    .forms[1]
                    .security = Some(vec!["none".into(), "none".into()])
            }
            _ => {
                source.security_definitions.insert(
                    "none".into(),
                    serde_json::from_str(r#"{"scheme":"basic"}"#).unwrap(),
                );
            }
        }
        let (mut root, mut account) = slot::<R::Artifact>(&cfg);
        // No runtime admission is reached by these valid typed-TD negatives.
        let mut build = build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap();
        let owner = loop {
            match build.step(&mut budget(4096), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => build = next,
                ConsumerPropertyReadBuildStep::Failed { cleanup, .. } => break cleanup.unwrap(),
                _ => panic!("bad preflight accepted"),
            }
        };
        drop(source);
        reclaim(owner);
        assert!(root.take().is_none());
        let bytes = account.used();
        drop(root);
        assert!(account.release_committed(bytes));
    }
}

#[test]
fn static_missing_id_and_effective_security_fail_before_output_allocation() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    invalid_inputs(production_static_registration(c, id));
}
#[cfg(feature = "std")]
#[test]
fn host_missing_id_and_effective_security_fail_before_output_allocation() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    invalid_inputs(consumer_compiler_support_probe::production_host_registration(c, id));
}

fn output_boundaries<R: ConsumerPropertyReadRegistration>(r: R) {
    let cfg = config();
    let source = source();
    let (mut root, mut account) = slot::<R::Artifact>(&cfg);
    let draft = finish(
        build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        8 << 20,
        usize::MAX,
    )
    .unwrap_or_else(|(e, _)| panic!("{e:?}"));
    let exact = draft.footprint().retained_bytes();
    drop(draft);
    reclaim(root.take().unwrap());
    assert!(root.take().is_none());
    disarm();
    for bytes in [exact - 1, exact, exact + 1] {
        let result = finish(
            build_consumer_property_read_aggregate(
                proof(&source),
                &r,
                &cfg,
                PlanSetGeneration::INITIAL,
                ledger(0),
                &mut root,
            )
            .unwrap(),
            bytes,
            usize::MAX,
        );
        match result {
            Ok(draft) => {
                assert!(bytes >= exact);
                assert_eq!(draft.footprint().retained_bytes(), exact);
                drop(draft);
                reclaim(root.take().unwrap());
            }
            Err((_, owner)) => {
                assert_eq!(bytes, exact - 1);
                reclaim(owner);
            }
        }
        assert!(root.take().is_none());
        assert_eq!(trace().live, 0);
        disarm();
    }
    let mut empty = source.clone();
    for property in empty.properties.as_mut().unwrap().values_mut() {
        for form in &mut property._interaction.forms {
            form.op = Some(Vec::new());
        }
    }
    let draft = finish(
        build_consumer_property_read_aggregate(
            proof(&empty),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        8 << 20,
        usize::MAX,
    )
    .unwrap_or_else(|(e, _)| panic!("{e:?}"));
    assert_eq!(draft.coordinate_count(), 0);
    assert_eq!(draft.property_count(), 4);
    assert!(matches!(
        select_consumer_property_read_aggregate(&draft, "a-read", &InteractionOptions::default()),
        Err(CoreError::Selection {
            reason: SelectionFailureReason::NoFormSupportsOperation,
            ..
        })
    ));
    drop(draft);
    reclaim(root.take().unwrap());
    assert!(root.take().is_none());
    assert_eq!(trace().live, 0);
    disarm();
    let bytes = account.used();
    drop(root);
    assert!(account.release_committed(bytes));
}
#[test]
fn static_empty_aggregate_and_output_capacity_boundaries_are_exact() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    output_boundaries(production_static_registration(c, id));
}
#[cfg(feature = "std")]
#[test]
fn host_empty_aggregate_and_output_capacity_boundaries_are_exact() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    output_boundaries(consumer_compiler_support_probe::production_host_registration(c, id));
}

#[test]
fn public_aggregate_keeps_one_td_lifetime_across_both_passes() {
    let source = source();
    let cfg = config();
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let r = production_static_registration(c, id);
    let (mut root, mut account) = slot::<ResolvedTargetArtifact>(&cfg);
    let mut run = |units: u64| {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .with_limit(K::JsonMembersPerObjectMax, Some(64))
            .with_limit(K::NumberLexemeBytesMax, Some(64))
            .with_limit(K::DocumentValidationWorkUnitsMax, Some(units));
        let Ok(td_config) = ValidatedThingAdmissionConfig::try_from_limits(&limits) else {
            return (false, false, None);
        };
        let mut td = ValidatedThingCursor::from_thing(&source, &td_config, ledger(0));
        let proof = loop {
            match td.step(&mut budget(4096), false) {
                ValidatedThingProgress::Pending(next) => td = next,
                ValidatedThingProgress::Complete(proof) => break proof,
                ValidatedThingProgress::Failed(_) => return (false, false, None),
            }
        };
        let mut build = build_consumer_property_read_aggregate(
            proof,
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap();
        let mut emitted = false;
        let result = loop {
            build = match build.step(&mut budget(0), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => next,
                _ => panic!("zero work advanced"),
            };
            match build.step(&mut budget(4096), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => build = next,
                ConsumerPropertyReadBuildStep::Preflight { build: next, .. } => {
                    assert!(!emitted);
                    emitted = true;
                    build = next
                        .admit_runtime(ledger(8 << 20))
                        .unwrap_or_else(|_| panic!("admit"));
                }
                ConsumerPropertyReadBuildStep::Complete(draft) => {
                    reclaim(draft.into_rollback());
                    break (true, emitted, None);
                }
                ConsumerPropertyReadBuildStep::Failed { cause, cleanup } => {
                    reclaim(cleanup.unwrap());
                    break (false, emitted, Some(cause));
                }
            }
        };
        assert!(root.take().is_none());
        result
    };
    let (mut low, mut high) = (0, 8 << 20);
    assert!(run(high).0);
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if run(middle).0 {
            high = middle;
        } else {
            low = middle;
        }
    }
    let (complete, preflight, cause) = run(high - 1);
    assert!(
        !complete && preflight,
        "exhaustion must occur in the second pass: exact={high}, complete={complete}, preflight={preflight}, cause={cause:?}"
    );
    assert!(matches!(
        cause,
        Some(CoreError::LimitExceeded {
            resource: K::DocumentValidationWorkUnitsMax,
            ..
        })
    ));
    assert!(run(high).0);
    assert!(run(high + 1).0);
    let bytes = account.used();
    drop(root);
    assert!(account.release_committed(bytes));
    println!("two_pass_td_lifetime_exact={high}; one_short_fails_after_preflight=true");
}

type Terminal<A> =
    Result<ConsumerPropertyReadDraft<A>, (CoreError, ConsumerPropertyReadRollback<A>)>;
fn cut<'td, 'env, R: ConsumerPropertyReadRegistration>(
    mut build: ConsumerPropertyReadBuild<'td, 'env, R>,
    at: usize,
    cancel: bool,
) -> (usize, Option<Terminal<R::Artifact>>) {
    for step in 0..100000 {
        if step == at {
            if !cancel {
                drop(build);
                return (step, None);
            }
            return match build.step(&mut budget(4096), true) {
                ConsumerPropertyReadBuildStep::Failed { cause, cleanup } => {
                    (step, Some(Err((cause, cleanup.unwrap()))))
                }
                _ => panic!("cancel advanced"),
            };
        }
        match build.step(&mut budget(4096), false) {
            ConsumerPropertyReadBuildStep::Pending(next) => build = next,
            ConsumerPropertyReadBuildStep::Preflight { build: next, .. } => {
                build = next
                    .admit_runtime(ledger(8 << 20))
                    .unwrap_or_else(|_| panic!("admit"))
            }
            ConsumerPropertyReadBuildStep::Complete(draft) => return (step, Some(Ok(draft))),
            ConsumerPropertyReadBuildStep::Failed { cause, cleanup } => {
                return (step, Some(Err((cause, cleanup.unwrap()))));
            }
        }
    }
    panic!("stalled")
}
fn all_boundaries<R: ConsumerPropertyReadRegistration>(r: R) {
    let source = source();
    let cfg = config();
    let (mut root, mut account) = slot::<R::Artifact>(&cfg);
    // A baseline establishes every actual public poll boundary, not guessed
    // phase offsets. Subsequent runs cancel and abandon at each one.
    let (steps, result) = cut(
        build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        usize::MAX,
        false,
    );
    drop(result.unwrap().unwrap_or_else(|(e, _)| panic!("{e:?}")));
    reclaim(root.take().unwrap());
    assert!(root.take().is_none());
    for cancel in [false, true] {
        for at in 0..=steps {
            let proof = proof(&source);
            observe(usize::MAX);
            let (_, result) = cut(
                build_consumer_property_read_aggregate(
                    proof,
                    &r,
                    &cfg,
                    PlanSetGeneration::INITIAL,
                    ledger(0),
                    &mut root,
                )
                .unwrap(),
                at,
                cancel,
            );
            match result {
                None => {
                    let owner = root.take().unwrap();
                    reclaim(owner);
                }
                Some(Err((cause, owner))) => {
                    assert!(matches!(cause, CoreError::Cancelled(_)));
                    reclaim(owner);
                }
                Some(Ok(_)) => panic!("cut missed"),
            }
            assert!(root.take().is_none());
            assert_eq!(trace().live, 0, "boundary {at}, cancel={cancel}");
            disarm();
            assert!(account.used() > 0);
        }
    }
    // The returned failure really ends both loans before cleanup is driven.
    let (_, result) = cut(
        build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        steps - 2,
        true,
    );
    let (_, owner) = result.unwrap().err().unwrap();
    drop(source);
    drop(r);
    reclaim(owner);
    assert!(root.take().is_none());
    let bytes = account.used();
    drop(root);
    assert!(account.release_committed(bytes));
}
#[test]
fn static_cancel_and_abandon_every_public_poll_boundary() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    all_boundaries(production_static_registration(c, id));
}
#[cfg(feature = "std")]
#[test]
fn host_cancel_and_abandon_every_public_poll_boundary() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    all_boundaries(consumer_compiler_support_probe::production_host_registration(c, id));
}

#[test]
fn actual_last_bound_failure_and_occupied_slot_reject_without_start() {
    let mut source = source();
    source
        .properties
        .as_mut()
        .unwrap()
        .get_mut("c-温度")
        .unwrap()
        ._interaction
        .forms[1] =
        clinkz_wot_td::form::Form::read_property(format!("https://peer.test/{}", "x".repeat(65)))
            .build()
            .unwrap();
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let counts = checked_program::test_support::traced(
        proof(&source),
        production_static_registration(c, id),
        None,
        false,
    );
    assert!(counts.failed);
    assert_eq!((counts.bounds, counts.starts), (3, 0));
    #[cfg(feature = "std")]
    {
        let c = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&c);
        let counts = checked_program::test_support::traced(
            proof(&source),
            consumer_compiler_support_probe::production_host_registration(c, id),
            None,
            false,
        );
        assert_eq!((counts.bounds, counts.starts), (3, 0));
    }
    let source = self::source();
    let cfg = config();
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let r = production_static_registration(c, id);
    let (mut root, mut account) = slot::<ResolvedTargetArtifact>(&cfg);
    let draft = finish(
        build_consumer_property_read_aggregate(
            proof(&source),
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root,
        )
        .unwrap(),
        8 << 20,
        usize::MAX,
    )
    .unwrap_or_else(|(e, _)| panic!("{e:?}"));
    disarm();
    let proof = proof(&source);
    observe(usize::MAX);
    assert!(matches!(
        build_consumer_property_read_aggregate(
            proof,
            &r,
            &cfg,
            PlanSetGeneration::INITIAL,
            ledger(0),
            &mut root
        ),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(trace().requests, 0);
    disarm();
    drop(draft);
    reclaim(root.take().unwrap());
    assert!(root.take().is_none());
    let bytes = account.used();
    drop(root);
    assert!(account.release_committed(bytes));
}
