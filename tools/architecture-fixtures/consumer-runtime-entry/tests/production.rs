//! Completion candidate: actual downstream Core/Planning API execution.
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
use consumer_runtime_entry_probe::production::{self, Client, State, Stats};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    collections::BTreeMap,
    sync::{Arc, atomic::Ordering},
    task::{Context, Poll, Waker},
};
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

fn cx() -> Context<'static> {
    // The standard library supplies a static no-op waker.
    Context::from_waker(Waker::noop())
}
struct LocalOwner<A: 'static> {
    draft: ConsumerPropertyReadDraft<A>,
    root: ConsumerPropertyReadCleanupSlot<A>,
    account: ResourceAccount,
}
fn prepare<R: ConsumerPropertyReadRegistration>(
    source: &Thing,
    registration: &R,
    fail: usize,
) -> Result<
    LocalOwner<R::Artifact>,
    (
        CoreError,
        ConsumerPropertyReadRollback<R::Artifact>,
        ConsumerPropertyReadCleanupSlot<R::Artifact>,
        ResourceAccount,
    ),
> {
    let config = config();
    let capacity = ConsumerPropertyReadCleanupSlot::<R::Artifact>::required_capacity();
    let mut account = ResourceAccount::new(
        SlotIndex::new(73),
        Generation::INITIAL,
        K::CleanupBytesMax,
        capacity,
    );
    let mut root =
        ConsumerPropertyReadCleanupSlot::try_new(&config, account.try_reserve(capacity).unwrap())
            .unwrap_or_else(|_| panic!("cleanup root"));
    let reservation = root.reservation();
    let mut build = build_consumer_property_read_aggregate(
        proof(source),
        registration,
        &config,
        PlanSetGeneration::INITIAL.checked_next().unwrap(),
        ledger(0),
        &mut root,
    )
    .unwrap();
    let mut preflight = false;
    let result = 'drive: {
        for _ in 0..100000 {
            match build.step(&mut budget(4096), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => build = next,
                ConsumerPropertyReadBuildStep::Preflight {
                    build: mut next,
                    requirements,
                } => {
                    assert!(!preflight);
                    preflight = true;
                    assert_eq!(requirements.registration, registration.identity());
                    assert_eq!(
                        (requirements.property_count, requirements.coordinate_count),
                        (4, 3)
                    );
                    for _ in 0..3 {
                        next = match next.step(&mut budget(4096), false) {
                            ConsumerPropertyReadBuildStep::Pending(next) => next,
                            _ => panic!("preflight replay"),
                        };
                    }
                    observe(fail);
                    build = next
                        .admit_runtime(ledger(requirements.output_capacity_bytes))
                        .unwrap_or_else(|_| panic!("runtime"));
                }
                ConsumerPropertyReadBuildStep::Complete(draft) => {
                    assert!(preflight);
                    break 'drive Ok(draft);
                }
                ConsumerPropertyReadBuildStep::Failed { cause, cleanup } => {
                    break 'drive Err((cause, cleanup.unwrap()));
                }
            }
        }
        panic!("build stalled")
    };
    assert_eq!(root.reservation(), reservation);
    match result {
        Ok(draft) => Ok(LocalOwner {
            draft,
            root,
            account,
        }),
        Err((cause, cleanup)) => Err((cause, cleanup, root, account)),
    }
}
fn reclaim<A: 'static>(
    mut cleanup: ConsumerPropertyReadRollback<A>,
    mut root: ConsumerPropertyReadCleanupSlot<A>,
    account: ResourceAccount,
) {
    let initial = cleanup.observations().unwrap();
    let capacity = account.used();
    assert!(capacity > 0);
    cleanup = match cleanup.step(&mut budget(0)) {
        ConsumerPropertyReadRollbackStep::Pending(next) => next,
        ConsumerPropertyReadRollbackStep::Complete(_) => {
            release_root(root, account);
            return;
        }
    };
    // Abandoning this lease returns it to the original destination. Neither
    // that Drop nor scope exit substitutes for driving cleanup to terminal.
    drop(cleanup);
    cleanup = root.take().unwrap();
    for _ in 0..100000 {
        assert_eq!(account.used(), capacity);
        if let Some((live, peak, largest)) = cleanup.observations() {
            assert_eq!((peak, largest), (initial.1, initial.2));
            assert!(live <= initial.0);
        }
        match cleanup.step(&mut budget(31)) {
            ConsumerPropertyReadRollbackStep::Pending(next) => cleanup = next,
            ConsumerPropertyReadRollbackStep::Complete(_) => {
                release_root(root, account);
                return;
            }
        }
    }
    panic!("cleanup stalled")
}
fn release_root<A: 'static>(
    mut root: ConsumerPropertyReadCleanupSlot<A>,
    mut account: ResourceAccount,
) {
    let capacity = account.used();
    assert!(root.take().is_none()); // refresh destroys final backing
    assert!(root.is_vacant());
    assert_eq!(trace().live, 0, "zero retained artifact/root bytes");
    drop(root);
    assert!(account.release_committed(capacity));
    assert_eq!(account.used(), 0, "original cleanup account released last");
}
fn check_output<A: 'static>(draft: &ConsumerPropertyReadDraft<A>, id: BindingRegistrationIdentity) {
    assert_eq!(draft.registration_identity(), id);
    assert_eq!((draft.property_count(), draft.coordinate_count()), (4, 3));
    let footprint = draft.footprint();
    assert_eq!(footprint.retained_source_bytes, 0);
    assert_eq!(footprint.registration_delta_bytes, 0);
    assert_eq!(footprint.retained_bytes(), trace().live as u64);
    for (property, index, raw) in [
        ("a-read", 1, "../first"),
        ("a-read", 2, "second"),
        ("c-温度", 1, "last"),
    ] {
        let selected = select_consumer_property_read_aggregate(
            draft,
            property,
            &InteractionOptions::default().with_form_index(index),
        )
        .unwrap();
        assert_eq!(selected.plan().form_index(), index as u32);
        assert_eq!(selected.raw_href(), raw);
        let poisoned = InteractionOptions::with_data(Payload::new(
            b"poisoned-legacy-data".to_vec(),
            "application/octet-stream",
        ))
        .with_uri_variable("poison", "must-not-replan")
        .with_timeout(core::time::Duration::from_millis(1))
        .with_form_index(index);
        assert_eq!(
            select_consumer_property_read_aggregate(draft, property, &poisoned)
                .unwrap()
                .artifact_ref(),
            selected.artifact_ref()
        );
        assert_eq!(
            selected.artifact_ref().identity(),
            selected.artifact().identity()
        );
        assert_eq!(
            selected.artifact().identity().binding_generation(),
            id.binding_generation()
        );
    }
    for empty in ["b-empty", "z-empty"] {
        assert!(matches!(
            select_consumer_property_read_aggregate(draft, empty, &InteractionOptions::default()),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::NoFormSupportsOperation,
                ..
            })
        ));
    }
    assert!(matches!(
        select_consumer_property_read_aggregate(
            draft,
            "a-read",
            &InteractionOptions::default().with_form_index(0)
        ),
        Err(CoreError::Selection {
            reason: SelectionFailureReason::StrictSelectionMismatch,
            ..
        })
    ));
}
#[test]
fn static_client_only_aggregate_executes_after_source_destruction_and_explicitly_reclaims() {
    println!(
        "static target layouts={:?}",
        production::CONSUMER_ONLY_STATIC_LAYOUTS
    );
    let source = source();
    let stats = Arc::new(Stats::default());
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = production::identity(&compiler);
    observe(usize::MAX);
    let registration = production::static_registration(compiler, stats.clone());
    assert_eq!((trace().requests, trace().live), (0, 0));
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    assert_eq!(
        registration.registration().execution(),
        BindingExecutionSupport::application_static()
    );
    assert_eq!(
        registration.registration().resources(),
        production::resources()
    );
    assert_eq!(
        registration.registration().status(),
        BindingStatusPolicy::new(2, 17)
    );
    disarm();
    let owner = prepare(&source, &registration, usize::MAX).unwrap_or_else(|_| panic!("prepare"));
    let memory = trace();
    disarm();
    drop(source);
    check_output(&owner.draft, id);
    let mut registration = registration.into_registration();
    let mut slot = StaticConsumerPropertyReadSlot::new();
    for (name, index, expected) in [
        ("a-read", 1, "https://peer.test/first"),
        ("a-read", 2, "https://peer.test/a/second"),
        ("c-温度", 1, "https://peer.test/a/last"),
    ] {
        let selected = select_consumer_property_read_aggregate(
            &owner.draft,
            name,
            &InteractionOptions::default().with_form_index(index),
        )
        .unwrap();
        assert_eq!(selected.artifact().artifact().payload().target(), expected);
        let aggregate_trace = trace();
        observe(usize::MAX);
        let request =
            OutboundRequest::property_read(selected.artifact_ref(), BTreeMap::new(), None).unwrap();
        assert_eq!(
            registration
                .start_consumer_property_read(
                    request,
                    selected.artifact(),
                    &mut slot,
                    &mut budget(1)
                )
                .unwrap(),
            StartStatus::Pending
        );
        assert!(
            registration
                .acknowledge_consumer_property_read(&mut slot)
                .is_err()
        );
        let Poll::Ready(Ok(output)) =
            registration.poll_consumer_property_read(&mut cx(), &mut slot, &mut budget(1))
        else {
            panic!("result")
        };
        assert_eq!(
            output.metadata().binding_response().unwrap().status_code(),
            599
        );
        assert_eq!(output.data().unwrap().body.as_ref(), expected.as_bytes());
        drop(output);
        registration
            .acknowledge_consumer_property_read(&mut slot)
            .unwrap();
        assert!(slot.is_vacant());
        assert_eq!(
            trace().live,
            0,
            "static request allocations released at acknowledgement"
        );
        TRACE.with(|t| t.set(aggregate_trace));
    }
    assert_eq!(stats.starts.load(Ordering::SeqCst), 3);
    assert_eq!(stats.requests_destroyed.load(Ordering::SeqCst), 3);
    drop(slot);
    drop(registration);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    println!(
        "static: registration={:?}, checked={:?}, slot={:?}, raw_state={:?}, retained_output={}, output_peak={}, largest={}, requests={}",
        Layout::new::<StaticConsumerBindingRegistration<Client>>(),
        Layout::new::<ConsumerCompilerRegistration<StaticConsumerBindingRegistration<Client>>>(),
        Layout::new::<StaticConsumerPropertyReadSlot<State>>(),
        Layout::new::<State>(),
        memory.live,
        memory.peak,
        memory.largest,
        memory.requests
    );
    let LocalOwner {
        draft,
        root,
        account,
    } = owner;
    reclaim(draft.into_rollback(), root, account);
}
#[cfg(feature = "std")]
#[test]
fn host_client_only_aggregate_executes_after_source_destruction_and_explicitly_reclaims() {
    let source = source();
    let stats = Arc::new(Stats::default());
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = production::identity(&compiler);
    observe(usize::MAX);
    let registration = production::host_registration(compiler, stats.clone());
    let provision = trace();
    disarm();
    assert_eq!(provision.requests, 2); // supported compiler adapter + actual client, no server
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    let owner = prepare(&source, &registration, usize::MAX).unwrap_or_else(|_| panic!("prepare"));
    let memory = trace();
    disarm();
    drop(source);
    check_output(&owner.draft, id);
    let registration = registration.into_registration();
    for (name, index, expected) in [
        ("a-read", 1, "https://peer.test/first"),
        ("a-read", 2, "https://peer.test/a/second"),
        ("c-温度", 1, "https://peer.test/a/last"),
    ] {
        let selected = select_consumer_property_read_aggregate(
            &owner.draft,
            name,
            &InteractionOptions::default().with_form_index(index),
        )
        .unwrap();
        assert_eq!(
            selected
                .artifact()
                .artifact()
                .try_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                .unwrap()
                .target(),
            expected
        );
        let aggregate_trace = trace();
        observe(usize::MAX);
        let request =
            OutboundRequest::property_read(selected.artifact_ref(), BTreeMap::new(), None).unwrap();
        let mut call = registration
            .start_consumer_property_read(request, selected.artifact())
            .unwrap();
        let footprint = call.as_pin_mut().lifetime_footprint();
        assert_eq!(footprint.retained_items(), 4);
        let Poll::Ready(Ok(output)) = call.as_pin_mut().poll_result(&mut cx(), &mut budget(1))
        else {
            panic!("result")
        };
        assert_eq!(
            output.metadata().binding_response().unwrap().status_code(),
            599
        );
        assert_eq!(output.data().unwrap().body.as_ref(), expected.as_bytes());
        drop(output);
        drop(call);
        let call_memory = trace();
        assert_eq!(
            call_memory.live, 0,
            "Host raw call, decorator and payload released once"
        );
        TRACE.with(|t| t.set(aggregate_trace));
        println!(
            "Host call: heap_peak={}, requests={}, declared_complete_bytes={}",
            call_memory.peak,
            call_memory.requests,
            footprint.retained_bytes()
        );
    }
    assert_eq!(stats.starts.load(Ordering::SeqCst), 3);
    assert_eq!(stats.requests_destroyed.load(Ordering::SeqCst), 3);
    let aggregate_trace = trace();
    TRACE.with(|t| t.set(provision));
    disarm();
    drop(registration);
    assert_eq!(
        trace().live,
        0,
        "actual client and supported adapter backing released"
    );
    TRACE.with(|t| t.set(aggregate_trace));
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    println!(
        "Host: registration={:?}, checked={:?}, client={:?}, adapter={:?}, provisioning_bytes={}, output_bytes={}, output_peak={}, largest={}, requests={}",
        Layout::new::<HostConsumerBindingRegistration>(),
        Layout::new::<ConsumerCompilerRegistration<HostConsumerBindingRegistration>>(),
        Layout::new::<Client>(),
        Layout::new::<ResolvedTargetCompiler>(),
        provision.live,
        memory.live,
        memory.peak,
        memory.largest,
        memory.requests
    );
    let LocalOwner {
        draft,
        root,
        account,
    } = owner;
    reclaim(draft.into_rollback(), root, account);
}
fn failure_cases<R: ConsumerPropertyReadRegistration>(r: R, stats: Arc<Stats>) {
    let source = source();
    let owner = prepare(&source, &r, usize::MAX).unwrap_or_else(|_| panic!("baseline"));
    let requests = trace().requests;
    disarm();
    let LocalOwner {
        draft,
        root,
        account,
    } = owner;
    reclaim(draft.into_rollback(), root, account);
    for failure in 0..requests {
        let (cause, cleanup, root, account) = prepare(&source, &r, failure)
            .err()
            .expect("actual allocator failure");
        disarm();
        assert!(matches!(cause, CoreError::Backpressure(_)));
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        reclaim(cleanup, root, account);
    }
    let long_source: Thing = serde_json::from_value({
        let mut value = serde_json::to_value(&source).unwrap();
        value["properties"]["c-温度"]["forms"][1]["href"] =
            serde_json::Value::String("x".repeat(65));
        value
    })
    .unwrap();
    let (cause, cleanup, root, account) = prepare(&long_source, &r, usize::MAX)
        .err()
        .expect("last bound exceeds actual compiler capacity");
    disarm();
    assert!(matches!(cause, CoreError::Validation(_)));
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    reclaim(cleanup, root, account);
    drop(r);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
}

#[test]
fn actual_static_bounds_and_allocation_failures_reclaim_original_root() {
    let stats = Arc::new(Stats::default());
    let r = production::static_registration(
        ResolvedTargetCompiler::try_new(64).unwrap(),
        stats.clone(),
    );
    failure_cases(r, stats);
}
#[cfg(feature = "std")]
#[test]
fn actual_host_bounds_and_allocation_failures_reclaim_original_root() {
    let stats = Arc::new(Stats::default());
    let r =
        production::host_registration(ResolvedTargetCompiler::try_new(64).unwrap(), stats.clone());
    failure_cases(r, stats);
}

// The exact production program's existing test observer corroborates the
// public failure above: every bound precedes any compiler start in both forms.
#[allow(dead_code)]
#[path = "../../../../planning/src/consumer_property_read.rs"]
mod checked_program;
#[test]
fn new_checked_adapters_preserve_the_single_all_bounds_barrier() {
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
    let stats = Arc::new(Stats::default());
    let r = production::static_registration(
        ResolvedTargetCompiler::try_new(64).unwrap(),
        stats.clone(),
    );
    let counts = checked_program::test_support::traced(proof(&source), r, None, false);
    assert!(counts.failed);
    assert_eq!((counts.bounds, counts.starts), (3, 0));
    #[cfg(feature = "std")]
    {
        let r = production::host_registration(
            ResolvedTargetCompiler::try_new(64).unwrap(),
            stats.clone(),
        );
        let counts = checked_program::test_support::traced(proof(&source), r, None, false);
        assert!(counts.failed);
        assert_eq!((counts.bounds, counts.starts), (3, 0));
    }
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
}
