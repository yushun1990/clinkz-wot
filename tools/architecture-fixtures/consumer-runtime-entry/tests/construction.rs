#![cfg(feature = "std")]
extern crate alloc;

use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, GatewayDefaultV1, Generation, ResourceAccount, ResourceKind as K, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use consumer_runtime_entry_probe::production::{self, Stats};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    collections::BTreeMap,
    sync::Arc,
    task::{Context, Poll, Waker},
};

std::thread_local! {
    static ALLOCATIONS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
    static ACCEPTED: Cell<bool> = const { Cell::new(false) };
    static WATCHED: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}
struct Observer;
#[global_allocator]
static ALLOCATOR: Observer = Observer;
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = ALLOCATIONS
            .try_with(|count| {
                let Some((requests, fail)) = count.get() else {
                    return false;
                };
                count.set(Some((requests + 1, fail)));
                requests == fail
            })
            .unwrap_or(false);
        if fail {
            // Disarm before diagnostic I/O. Only a subprocess arms failure.
            ALLOCATIONS.with(|count| count.set(None));
            if ACCEPTED.with(Cell::get) {
                eprintln!("FAIL_AFTER_RAW_ACCEPTANCE");
            } else {
                eprintln!("FAIL_BEFORE_RAW_ACCEPTANCE");
            }
            return core::ptr::null_mut();
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let _ = WATCHED.try_with(|watched| {
            let (address, released) = watched.get();
            if address != 0 && address == pointer as usize {
                watched.set((address, released + 1));
            }
        });
        unsafe { System.dealloc(pointer, layout) }
    }
}
fn observe(fail: usize) {
    ALLOCATIONS.with(|count| count.set(Some((0, fail))));
    ACCEPTED.with(|accepted| accepted.set(false));
}
fn disarm() -> usize {
    ALLOCATIONS.with(|count| count.replace(None).unwrap().0)
}

struct ObservedClient(production::Client);
impl binding::ClientBinding for ObservedClient {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.0.compatibility
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        let raw = binding::ClientBinding::invoke(&self.0, request, artifact)?;
        ACCEPTED.with(|accepted| accepted.set(true));
        Ok(raw)
    }
}

#[allow(dead_code)]
mod exact {
    include!(concat!(env!("OUT_DIR"), "/quiescent.rs"));
}
use exact::*;

fn budget(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn ledger(runtime: u64) -> AdmissionLedger {
    AdmissionLedger::new(
        SlotIndex::new(7),
        Generation::INITIAL,
        0,
        8 << 20,
        0,
        runtime,
        0,
        8 << 20,
    )
}
fn prepare() -> (
    ConsumerPropertyReadDraft<HostBindingArtifact>,
    ConsumerPropertyReadCleanupSlot<HostBindingArtifact>,
    ResourceAccount,
    HostConsumerBindingRegistration,
) {
    prepare_with_resources(production::resources())
}
fn prepare_with_resources(
    resources: BindingResourceDeclarations,
) -> (
    ConsumerPropertyReadDraft<HostBindingArtifact>,
    ConsumerPropertyReadCleanupSlot<HostBindingArtifact>,
    ResourceAccount,
    HostConsumerBindingRegistration,
) {
    let source: Thing = serde_json::from_str(r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"transfer","id":"urn:transfer","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"properties":{"value":{"forms":[{"href":"https://peer.test/value"}]}}}"#).unwrap();
    let td_config =
        ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut td = ValidatedThingCursor::from_thing(&source, &td_config, ledger(0));
    let proof = 'td: {
        for _ in 0..100_000 {
            match td.step(&mut budget(4096), false) {
                ValidatedThingProgress::Pending(next) => td = next,
                ValidatedThingProgress::Complete(proof) => break 'td proof,
                ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
            }
        }
        panic!("TD stalled")
    };
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let identity = production::identity(&compiler);
    let client = ObservedClient(production::Client {
        compatibility: compiler.compatibility(),
        stats: Arc::new(Stats::default()),
    });
    let registration =
        HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
            identity,
            BindingExecutionSupport::host_erased(),
            HostBindingCompilerRegistration::try_new_consumer(compiler).unwrap(),
            Box::new(client),
            resources,
            BindingStatusPolicy::new(2, 17),
        ))
        .unwrap_or_else(|_| panic!("registration"))
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("capture"));
    let config =
        ConsumerPropertyReadBuildConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let bytes = ConsumerPropertyReadCleanupSlot::<HostBindingArtifact>::required_capacity();
    let mut account = ResourceAccount::new(
        SlotIndex::new(8),
        Generation::INITIAL,
        K::CleanupBytesMax,
        bytes,
    );
    let mut slot =
        ConsumerPropertyReadCleanupSlot::try_new(&config, account.try_reserve(bytes).unwrap())
            .unwrap_or_else(|_| panic!("slot"));
    let mut build = build_consumer_property_read_aggregate(
        proof,
        &registration,
        &config,
        PlanSetGeneration::INITIAL,
        ledger(0),
        &mut slot,
    )
    .unwrap();
    let draft = 'planning: {
        for _ in 0..100_000 {
            match build.step(&mut budget(4096), false) {
                ConsumerPropertyReadBuildStep::Pending(next) => build = next,
                ConsumerPropertyReadBuildStep::Preflight {
                    build: next,
                    requirements,
                } => {
                    build = next
                        .admit_runtime(ledger(requirements.output_capacity_bytes))
                        .unwrap_or_else(|_| panic!("runtime"));
                }
                ConsumerPropertyReadBuildStep::Complete(draft) => break 'planning draft,
                ConsumerPropertyReadBuildStep::Failed { cause, .. } => panic!("{cause:?}"),
            }
        }
        panic!("Planning stalled")
    };
    drop(source);
    (draft, slot, account, registration.into_registration())
}

fn reclaim(
    draft: ConsumerPropertyReadDraft<HostBindingArtifact>,
    mut slot: ConsumerPropertyReadCleanupSlot<HostBindingArtifact>,
    account: ResourceAccount,
) {
    // Exercise both original abandonment returns, on the receiving thread.
    drop(draft);
    let cleanup = slot.take().unwrap();
    drop(cleanup);
    let mut cleanup = slot.take().unwrap();
    assert!(matches!(
        cleanup.step(&mut budget(0)),
        ConsumerPropertyReadRollbackStep::Pending(_)
    ));
    cleanup = slot.take().unwrap();
    finish_cleanup(cleanup, slot, account);
}
fn finish_cleanup(
    mut cleanup: ConsumerPropertyReadRollback<HostBindingArtifact>,
    mut slot: ConsumerPropertyReadCleanupSlot<HostBindingArtifact>,
    mut account: ResourceAccount,
) {
    let bytes = account.used();
    for _ in 0..100_000 {
        assert_eq!(account.used(), bytes);
        match cleanup.step(&mut budget(31)) {
            ConsumerPropertyReadRollbackStep::Pending(next) => cleanup = next,
            ConsumerPropertyReadRollbackStep::Complete(_) => {
                assert!(slot.take().is_none());
                assert!(slot.is_vacant());
                drop(slot);
                assert!(account.release_committed(bytes));
                assert_eq!(account.used(), 0);
                return;
            }
        }
    }
    panic!("reclaim stalled")
}

#[test]
fn unique_completed_graph_moves_and_uses_original_selection_and_reclaim() {
    fn movable<T: Send>() {}
    movable::<UniqueOutput<HostBindingArtifact>>();
    movable::<UniqueCleanup<HostBindingArtifact>>();
    movable::<Arc<std::sync::Mutex<UniqueOutput<HostBindingArtifact>>>>();
    let original_pair = Layout::new::<(
        ConsumerPropertyReadDraft<HostBindingArtifact>,
        ConsumerPropertyReadCleanupSlot<HostBindingArtifact>,
    )>();
    let output_owner = Layout::new::<UniqueOutput<HostBindingArtifact>>();
    let cleanup_owner = Layout::new::<UniqueCleanup<HostBindingArtifact>>();
    assert!(
        (original_pair.size() + output_owner.size()) as u64
            <= ConsumerPropertyReadCleanupSlot::<HostBindingArtifact>::required_capacity()
    );
    println!(
        "handoff layouts: original_pair={original_pair:?}, output={output_owner:?}, rollback={cleanup_owner:?}, original_node={:?}",
        exact::test_support::layouts::<HostBindingArtifact>()[0]
    );
    let (draft, slot, account, registration) = prepare();
    let reference =
        select_consumer_property_read_aggregate(&draft, "value", &InteractionOptions::default())
            .unwrap()
            .artifact_ref();
    let reservation = slot.reservation();
    let artifact_address =
        select_consumer_property_read_aggregate(&draft, "value", &InteractionOptions::default())
            .unwrap()
            .artifact() as *const _ as usize;
    observe(usize::MAX);
    let output =
        UniqueOutput::try_take(draft, slot, &mut budget(1)).unwrap_or_else(|_| panic!("take"));
    assert_eq!(disarm(), 0, "handoff moves the existing backing");
    std::thread::spawn(move || {
        WATCHED.with(|watched| watched.set((output.address(), 0)));
        observe(usize::MAX);
        let (draft, slot) = output.into_local();
        assert_eq!(disarm(), 0, "reattachment allocates nothing");
        assert_eq!(slot.reservation(), reservation);
        let selected = select_consumer_property_read_aggregate(
            &draft,
            "value",
            &InteractionOptions::default(),
        )
        .unwrap();
        assert_eq!(selected.artifact_ref(), reference);
        assert_eq!(selected.artifact() as *const _ as usize, artifact_address);
        let request =
            OutboundRequest::property_read(selected.artifact_ref(), BTreeMap::new(), None).unwrap();
        let mut call = registration
            .start_consumer_property_read(request, selected.artifact())
            .unwrap();
        let Poll::Ready(Ok(result)) = call
            .as_pin_mut()
            .poll_result(&mut Context::from_waker(Waker::noop()), &mut budget(1))
        else {
            panic!("call")
        };
        assert_eq!(
            result.data().unwrap().body.as_ref(),
            b"https://peer.test/value"
        );
        drop(result);
        drop(call);
        drop(registration);
        reclaim(draft, slot, account);
        assert_eq!(
            WATCHED.with(Cell::get).1,
            1,
            "original backing released exactly once"
        );
    })
    .join()
    .unwrap();
}

#[test]
fn partially_paid_cleanup_moves_with_its_anchor_and_original_account() {
    let (draft, mut slot, account, registration) = prepare();
    drop(registration);
    let mut cleanup = draft.into_rollback();
    for _ in 0..3 {
        cleanup = match cleanup.step(&mut budget(1)) {
            ConsumerPropertyReadRollbackStep::Pending(next) => next,
            ConsumerPropertyReadRollbackStep::Complete(_) => panic!("unexpected terminal"),
        };
    }
    let observations = cleanup.observations();
    let paid_bytes = rollback_paid_bytes(&cleanup);
    assert!(paid_bytes > 0, "transfer an actually suspended byte cursor");
    let reservation = slot.reservation();
    // A short handoff preserves the exact already-paid cursor and both owners.
    let (cleanup, returned_slot) = UniqueCleanup::try_take(cleanup, slot, &mut budget(0))
        .err()
        .unwrap();
    slot = returned_slot;
    assert_eq!(cleanup.observations(), observations);
    observe(usize::MAX);
    let unique = UniqueCleanup::try_take(cleanup, slot, &mut budget(1))
        .unwrap_or_else(|_| panic!("cleanup handoff"));
    assert_eq!(disarm(), 0);
    assert_eq!(unique.paid_bytes(), paid_bytes);
    std::thread::spawn(move || {
        WATCHED.with(|watched| watched.set((unique.address(), 0)));
        observe(usize::MAX);
        let (cleanup, mut slot) = unique.into_local();
        assert_eq!(disarm(), 0);
        assert_eq!(slot.reservation(), reservation);
        assert_eq!(cleanup.observations(), observations);
        assert_eq!(rollback_paid_bytes(&cleanup), paid_bytes);
        drop(cleanup);
        let cleanup = slot.take().unwrap();
        // Reuse the original reclaimer, preserving all partial byte credit.
        finish_cleanup(cleanup, slot, account);
        assert_eq!(WATCHED.with(Cell::get).1, 1);
    })
    .join()
    .unwrap();
}

#[test]
fn wrong_root_and_short_work_return_both_original_graphs() {
    let (a, a_slot, a_account, a_registration) = prepare();
    let (b, b_slot, b_account, b_registration) = prepare();
    // Equal public identity/bytes conceal two different physical roots.
    assert_eq!(a_slot.reservation(), b_slot.reservation());
    let mut work = budget(1);
    let (a, b_slot) = UniqueOutput::try_take(a, b_slot, &mut work).err().unwrap();
    assert_eq!(work.remaining(WorkClass::PlanningItems), 1);
    let (a, a_slot) = UniqueOutput::try_take(a, a_slot, &mut budget(0))
        .err()
        .unwrap();
    assert_eq!(a.registration_identity(), a_registration.identity());
    drop(a_registration);
    drop(b_registration);
    reclaim(a, a_slot, a_account);
    reclaim(b, b_slot, b_account);
}

#[test]
fn local_worker_is_constructible_but_must_retain_and_drain_its_root() {
    let (command, commands) = std::sync::mpsc::sync_channel(1);
    let (reply, replies) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        // Nothing local crosses a thread: build the actual graph here.
        let (draft, slot, account, registration) = prepare();
        commands.recv().unwrap();
        let selected = select_consumer_property_read_aggregate(
            &draft,
            "value",
            &InteractionOptions::default(),
        )
        .unwrap();
        let request =
            OutboundRequest::property_read(selected.artifact_ref(), BTreeMap::new(), None).unwrap();
        let mut call = registration
            .start_consumer_property_read(request, selected.artifact())
            .unwrap();
        let mut cx = Context::from_waker(Waker::noop());
        assert!(
            call.as_pin_mut()
                .poll_result(&mut cx, &mut budget(0))
                .is_pending()
        );
        let Poll::Ready(result) = call.as_pin_mut().poll_result(&mut cx, &mut budget(1)) else {
            panic!("call")
        };
        assert!(result.is_ok());
        // Lost caller interest does not destroy the accepted call. The driver
        // still owns it, settles it and handles rejection by the reply lane.
        assert!(reply.send(result).is_err());
        drop(call);
        drop(registration);
        reclaim(draft, slot, account);
    });
    drop(replies);
    command.send(()).unwrap();
    worker.join().unwrap();
}

#[test]
fn existing_parent_account_can_cover_the_original_cleanup_owner_until_terminal() {
    let bytes = ConsumerPropertyReadCleanupSlot::<HostBindingArtifact>::required_capacity();
    let mut parent = ResourceAccount::new(
        SlotIndex::new(91),
        Generation::INITIAL,
        K::CleanupBytesMax,
        bytes,
    );
    parent.try_reserve(bytes).unwrap().commit();
    assert!(
        parent.try_reserve(1).is_none(),
        "another child must be rejected before construction"
    );
    let (draft, slot, account, registration) = prepare();
    assert_eq!(account.used(), bytes);
    assert_eq!(parent.used(), bytes);
    drop(registration);
    reclaim(draft, slot, account);
    assert_eq!(
        parent.used(),
        bytes,
        "child terminal does not silently release parent"
    );
    assert!(parent.release_committed(bytes));
    assert_eq!(parent.used(), 0);
    assert_eq!(parent.peak(), bytes);
    // This covers the cleanup owner only, not a global build/runtime governor.
}

#[test]
fn host_backing_failure_child() {
    let Ok(failure) = std::env::var("CLINKZ_HOST_BACKING_FAIL") else {
        return;
    };
    let fail = failure.parse().unwrap();
    let resources = if std::env::var_os("CLINKZ_HOST_BACKING_OVERRUN").is_some() {
        BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(0, 0),
            BindingLifetimeFootprint::new(1, 1),
        )
    } else {
        production::resources()
    };
    let (draft, slot, account, registration) = prepare_with_resources(resources);
    let selected =
        select_consumer_property_read_aggregate(&draft, "value", &InteractionOptions::default())
            .unwrap();
    let request =
        OutboundRequest::property_read(selected.artifact_ref(), BTreeMap::new(), None).unwrap();
    observe(fail);
    let call = registration
        .start_consumer_property_read(request, selected.artifact())
        .unwrap();
    let allocations = disarm();
    eprintln!("CALL_ALLOCATION_REQUESTS={allocations}");
    drop(call);
    drop(registration);
    reclaim(draft, slot, account);
}

#[test]
fn host_call_allocation_failure_is_fatal_including_after_raw_acceptance() {
    fn child(fail: usize, overrun: bool) -> std::process::Output {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "host_backing_failure_child", "--nocapture"])
            .env("CLINKZ_HOST_BACKING_FAIL", fail.to_string());
        if overrun {
            command.env("CLINKZ_HOST_BACKING_OVERRUN", "1");
        } else {
            command.env_remove("CLINKZ_HOST_BACKING_OVERRUN");
        }
        command.output().unwrap()
    }
    for overrun in [false, true] {
        let success = child(usize::MAX, overrun);
        assert!(
            success.status.success(),
            "{}",
            String::from_utf8_lossy(&success.stderr)
        );
        let stderr = String::from_utf8(success.stderr).unwrap();
        let allocations: usize = stderr
            .lines()
            .find_map(|line| line.strip_prefix("CALL_ALLOCATION_REQUESTS="))
            .unwrap()
            .parse()
            .unwrap();
        assert!(allocations >= 2);
        for fail in 0..allocations {
            let output = child(fail, overrun);
            assert!(
                !output.status.success(),
                "infallible allocation unexpectedly recovered"
            );
            let diagnostic = String::from_utf8_lossy(&output.stderr);
            let marker = if fail == allocations - 1 {
                "FAIL_AFTER_RAW_ACCEPTANCE"
            } else {
                "FAIL_BEFORE_RAW_ACCEPTANCE"
            };
            assert!(diagnostic.contains(marker), "index={fail}: {diagnostic}");
            assert!(diagnostic.contains("memory allocation of"), "{diagnostic}");
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(output.status.signal(), Some(6));
            }
        }
        println!(
            "Host construction (overrun={overrun}): {allocations} allocator requests; final request follows raw acceptance; every injected failure aborts"
        );
    }
}
