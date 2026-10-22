#![cfg(feature = "std")]
//! External non-HTTP author, using production pairing/funding without stages.
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{
    ProducerPropertyReadAdmission, ProducerPropertyReadStep, ResolvedProducerInput,
    producer_execution::*,
};
use clinkz_wot_servient::producer_installation::*;
use clinkz_wot_td::{
    ValidatedThing, ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress,
    thing::Thing,
};
use serde_json::json;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    },
    task::{Context, Poll, Waker},
};

fn accepted<T, I>(r: Result<T, BindingInputRejection<I>>) -> T {
    r.unwrap_or_else(|r| panic!("{:?}", r.error()))
}
fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, n))
}
fn instance(n: u32) -> ThingSlotId {
    ThingSlotId::new(SlotIndex::new(n), Generation::INITIAL)
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 10)
}
fn td() -> Thing {
    serde_json::from_value(json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"native", "securityDefinitions":{"n":{"scheme":"nosec"}}, "security":["n"], "properties":{"":{"type":"number", "readOnly":true, "forms":[{"href":"native://device/value"}]}}})).unwrap()
}
fn proof(td: &Thing) -> ValidatedThing<'_> {
    let config = ValidatedThingAdmissionConfig::try_from_limits(&GatewayDefaultV1::LIMITS).unwrap();
    let mut c = ValidatedThingCursor::from_thing(
        td,
        &config,
        AdmissionLedger::new(
            SlotIndex::new(0),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            0,
            0,
            0,
        ),
    );
    for _ in 0..100_000 {
        c = match c.step(&mut work(1_000_000), false) {
            ValidatedThingProgress::Pending(c) => c,
            ValidatedThingProgress::Complete(p) => return p,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        };
    }
    panic!("validation stalled")
}
fn coverage(td: &Thing) -> ProducerPropertyReadAdmission<'_, 'static> {
    let mut c = ProducerPropertyReadAdmission::new(proof(td), "");
    for _ in 0..100_000 {
        if c.step(&mut work(1_000_000), false).unwrap() == ProducerPropertyReadStep::Covered {
            return c;
        }
    }
    panic!("coverage stalled")
}
#[derive(Default)]
struct Probe {
    policy: AtomicUsize,
    created: AtomicUsize,
    polled: AtomicUsize,
    installed: AtomicUsize,
    live: AtomicUsize,
    dropped: AtomicUsize,
    handler_dropped: AtomicUsize,
    compiler_dropped: AtomicUsize,
    pending_install: AtomicBool,
    fail_install: AtomicBool,
    pending_close: AtomicBool,
    fail_close: AtomicBool,
}
struct Handler(Arc<Probe>);
impl ReadPropertyHandler for Handler {
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        panic!("unpublished installation invoked a handler")
    }
}
impl Drop for Handler {
    fn drop(&mut self) {
        self.0.handler_dropped.fetch_add(1, SeqCst);
    }
}
struct Program(Vec<u8>); // deliberately owned, non-Copy native backing
struct Executor(Arc<Probe>);
struct Compiler {
    probe: Arc<Probe>,
    config: usize,
}
impl Drop for Compiler {
    fn drop(&mut self) {
        self.probe.compiler_dropped.fetch_add(1, SeqCst);
    }
}
impl ProducerCompiler<Executor> for Compiler {
    fn configuration(&self) -> &usize {
        &self.config
    }
    fn compile(
        &self,
        input: ResolvedProducerInput<'_>,
        _: &mut WorkBudget,
    ) -> CoreResult<Option<Program>> {
        Ok(Some(Program(input.target().as_bytes().to_vec())))
    }
}
impl ProducerExecutor for Executor {
    type Program = Program;
    type Configuration = usize;
    type Installation = Installation;
    fn configuration_matches(&self, captured: &usize) -> bool {
        self.0.policy.load(SeqCst) == *captured
    }
    fn identity(&self) -> BindingRegistrationIdentity {
        BindingRegistrationIdentity::new(
            BindingId::new(900),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([1; 32]),
            BindingArtifactCompatibility::new([1; 16]),
            0,
        )
    }
    fn install_policy(&self) -> ProducerInstallPolicy {
        ProducerInstallPolicy {
            executor_bytes: 0,
            program_bytes: 64,
            installation_bytes: 0,
            temporary_bytes: 0,
            largest_indirect_allocation: 64,
            local_drop_work: 1,
        }
    }
    fn check_program(&self, p: &Program) -> CoreResult<()> {
        if p.0 == b"native://device/value" {
            Ok(())
        } else {
            Err(failure(1))
        }
    }
    fn create_installation(&self, _: &ProducerExecutionEntry<Program>) -> CoreResult<Installation> {
        self.0.created.fetch_add(1, SeqCst);
        Ok(Installation {
            p: self.0.clone(),
            live: false,
        })
    }
}
struct Installation {
    p: Arc<Probe>,
    live: bool,
}
impl ProducerInstallation for Installation {
    fn poll_install(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.p.polled.fetch_add(1, SeqCst);
        if !self.live {
            self.live = true;
            self.p.live.fetch_add(1, SeqCst);
        }
        if self.p.fail_install.load(SeqCst) {
            return Poll::Ready(Err(failure(2)));
        }
        if self.p.pending_install.load(SeqCst) {
            return Poll::Pending;
        }
        self.p.installed.fetch_add(1, SeqCst);
        Poll::Ready(Ok(()))
    }
    fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        if self.p.fail_close.load(SeqCst) {
            return Poll::Ready(Err(failure(3)));
        }
        if self.p.pending_close.load(SeqCst) {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}
impl Drop for Installation {
    fn drop(&mut self) {
        if self.live {
            self.p.live.fetch_sub(1, SeqCst);
        }
        self.p.dropped.fetch_add(1, SeqCst);
    }
}
fn failure(code: u16) -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Prepare, RetryClass::Never)
            .with_redacted_cause(code, "native failure"),
    )
}
type Runtime = ProducerInstallationRuntime<Executor, Handler>;
fn registered(
    p: &Arc<Probe>,
    name: &str,
    authority: &ProducerInstanceAuthority,
) -> ProducerHandlerRegistration<Handler> {
    ProducerHandlerRegistration::new(
        authority,
        name.into(),
        Handler(p.clone()),
        HandlerFootprint::new(std::mem::size_of::<Handler>() as u64, 0, 0),
    )
}
fn pair(p: &Arc<Probe>) -> ProducerBindingPreparation<Compiler, Executor> {
    accepted(ProducerBindingPreparation::try_new(
        Compiler {
            probe: p.clone(),
            config: p.policy.load(SeqCst),
        },
        Executor(p.clone()),
        p.policy.load(SeqCst),
    ))
}
fn prepared(
    p: &Arc<Probe>,
    authority: &ProducerInstanceAuthority,
) -> PreparedProducer<Executor, Handler> {
    let td = td();
    let mut c = coverage(&td);
    accepted(pair(p).prepare_property_read(
        &mut c,
        authority,
        registered(p, "", authority),
        &mut work(1_000_000),
    ))
}
fn bank(limits: &ResourceLimits) -> Arc<ProducerInstallationAccounts> {
    Arc::new(producer_installation_accounts(limits))
}
fn empty(bank: &Arc<ProducerInstallationAccounts>) {
    assert!(bank.snapshot().iter().all(|a| a.used() == 0));
}
fn authority(instance: ThingSlotId) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(instance).unwrap()
}
fn runtime(p: &Arc<Probe>) -> (Runtime, Arc<ProducerInstallationAccounts>) {
    let limits = GatewayDefaultV1::LIMITS.clone();
    let bank = bank(&limits);
    let authority = authority(instance(0));
    (
        accepted(Runtime::try_new(
            prepared(p, &authority),
            authority,
            limits,
            bank.clone(),
            Deadline::NONE,
        )),
        bank,
    )
}
fn step(r: &mut Runtime, n: u64) -> Poll<ProducerInstallationState> {
    r.step(&mut Context::from_waker(Waker::noop()), &mut work(n), now())
}
#[test]
fn source_compiler_and_id_are_absent_at_funded_installation() {
    let p = Arc::new(Probe::default());
    let (mut r, b) = runtime(&p);
    assert_eq!(p.compiler_dropped.load(SeqCst), 1);
    assert_eq!(r.entry().unwrap().property(), "");
    for _ in 0..3 {
        assert!(step(&mut r, 0).is_pending());
    }
    assert_eq!(p.created.load(SeqCst), 0);
    assert_eq!(
        step(&mut r, 1),
        Poll::Ready(ProducerInstallationState::Installing)
    );
    p.pending_install.store(true, SeqCst);
    assert!(step(&mut r, 1).is_pending());
    assert_eq!(p.live.load(SeqCst), 1);
    p.pending_install.store(false, SeqCst);
    assert_eq!(
        step(&mut r, 1),
        Poll::Ready(ProducerInstallationState::InstalledClosed)
    );
    let polls = p.polled.load(SeqCst);
    assert_eq!(
        step(&mut r, 0),
        Poll::Ready(ProducerInstallationState::InstalledClosed)
    );
    assert_eq!(p.polled.load(SeqCst), polls);
    r.begin_close();
    assert!(step(&mut r, 0).is_pending());
    assert_eq!(p.live.load(SeqCst), 1);
    assert_eq!(
        step(&mut r, 1),
        Poll::Ready(ProducerInstallationState::Closed)
    );
    assert_eq!(p.live.load(SeqCst), 0);
    assert_eq!(p.handler_dropped.load(SeqCst), 1);
    assert!(r.entry().is_none());
    assert!(
        b.snapshot().iter().any(|a| a.used() > 0),
        "terminal status still funded"
    );
    drop(r);
    empty(&b);
    assert_eq!(p.dropped.load(SeqCst), 1);
}
#[test]
fn actual_configuration_registered_key_and_current_instance_refuse_before_construction() {
    let p = Arc::new(Probe::default());
    let refused = ProducerBindingPreparation::try_new(
        Compiler {
            probe: p.clone(),
            config: 1,
        },
        Executor(p.clone()),
        1,
    )
    .err()
    .unwrap();
    let (c, e, snapshot) = refused.into_input();
    assert_eq!(c.config, 1);
    assert_eq!(snapshot, 1);
    assert!(e.configuration_matches(&0));
    drop((c, e));
    let td = td();
    let mut a = coverage(&td);
    let authority = authority(instance(0));
    let rejected = pair(&p)
        .prepare_property_read(
            &mut a,
            &authority,
            registered(&p, "wrong", &authority),
            &mut work(1_000_000),
        )
        .err()
        .unwrap();
    assert_eq!(rejected.input().1.property(), "wrong");
    drop(rejected);
    let replacement = ProducerInstanceAuthority::try_new(instance(0)).unwrap();
    let mut coverage = coverage(&td);
    let old_handler = registered(&p, "", &authority);
    let refused = pair(&p)
        .prepare_property_read(
            &mut coverage,
            &replacement,
            old_handler,
            &mut work(1_000_000),
        )
        .err()
        .unwrap();
    assert_eq!(refused.input().1.instance_key(), authority.key());
    drop(refused);
    let limits = GatewayDefaultV1::LIMITS.clone();
    let b = bank(&limits);
    let prepared = prepared(&p, &authority);
    let replacement = ProducerInstanceAuthority::try_new(instance(0)).unwrap();
    assert_ne!(prepared.entry().instance_key(), replacement.key());
    let rejected = Runtime::try_new(prepared, replacement, limits, b.clone(), Deadline::NONE)
        .err()
        .unwrap();
    assert_eq!(rejected.input().0.entry().instance(), instance(0));
    empty(&b);
    assert_eq!(p.created.load(SeqCst), 0);
}
#[test]
fn uncovered_draft_and_zero_preparation_credit_cannot_mint_a_completed_entry() {
    let authority = authority(instance(0));
    let p = Arc::new(Probe::default());
    let td = td();
    let mut a = ProducerPropertyReadAdmission::new(proof(&td), "");
    assert!(
        pair(&p)
            .prepare_property_read(
                &mut a,
                &authority,
                registered(&p, "", &authority),
                &mut work(1_000_000)
            )
            .is_err()
    );
    let mut a = coverage(&td);
    let refused = pair(&p)
        .prepare_property_read(
            &mut a,
            &authority,
            registered(&p, "", &authority),
            &mut work(0),
        )
        .err()
        .unwrap();
    let (pair, handler) = refused.into_input();
    assert_eq!(handler.property(), "");
    drop(accepted(pair.prepare_property_read(
        &mut a,
        &authority,
        handler,
        &mut work(1_000_000),
    )));
    assert_eq!(p.created.load(SeqCst), 0);
}
#[test]
fn local_and_original_parent_limits_rollback_without_native_effects() {
    let attempt = |p: &Arc<Probe>, limits, b: Arc<ProducerInstallationAccounts>| {
        let authority = authority(instance(0));
        Runtime::try_new(
            prepared(p, &authority),
            authority,
            limits,
            b,
            Deadline::NONE,
        )
    };
    let p = Arc::new(Probe::default());
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::BindingSlotStateBytesPerItemMax,
        ResourceKind::BindingArtifactBytesPerItemMax,
        ResourceKind::HandlerSlotsPerThingMax,
        ResourceKind::CleanupItemsMax,
        ResourceKind::CleanupItemBytesMax,
        ResourceKind::CleanupWorkItemsPerStepMax,
    ] {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .try_with_limit(kind, Some(0))
            .unwrap();
        let b = bank(&limits);
        assert!(attempt(&p, limits, b.clone()).is_err(), "{kind:?}");
        empty(&b);
    }
    let limits = GatewayDefaultV1::LIMITS.clone();
    let parent_limits = limits
        .clone()
        .try_with_limit(
            ResourceKind::DurableStatusBytesGlobalMax,
            Some(std::mem::size_of::<Option<CoreError>>() as u64),
        )
        .unwrap();
    let b = bank(&parent_limits);
    let sibling = accepted(attempt(&p, limits.clone(), b.clone()));
    let before = b.snapshot().map(|a| a.used());
    assert!(attempt(&p, limits, b.clone()).is_err());
    assert_eq!(b.snapshot().map(|a| a.used()), before);
    drop(sibling);
    empty(&b);
    assert_eq!(p.created.load(SeqCst), 0);
}
#[test]
fn partial_install_and_cleanup_errors_retain_one_owner_and_the_first_cause() {
    let p = Arc::new(Probe::default());
    p.fail_install.store(true, SeqCst);
    let (mut r, b) = runtime(&p);
    let _ = step(&mut r, 1);
    let _ = step(&mut r, 1);
    assert_eq!(r.state(), ProducerInstallationState::Closing);
    assert_eq!(p.live.load(SeqCst), 1);
    let cause = r.first_cause().unwrap().clone();
    assert_eq!(cause.context().cause_code(), Some(2));
    p.fail_close.store(true, SeqCst);
    let _ = step(&mut r, 1);
    r.cancel();
    assert_eq!(r.first_cause(), Some(&cause));
    assert_eq!(p.dropped.load(SeqCst), 0);
    p.fail_close.store(false, SeqCst);
    p.pending_close.store(true, SeqCst);
    assert!(step(&mut r, 1).is_pending());
    p.pending_close.store(false, SeqCst);
    let _ = step(&mut r, 1);
    assert_eq!(p.dropped.load(SeqCst), 1);
    assert_eq!(r.first_cause(), Some(&cause));
    drop(r);
    empty(&b);
}
#[test]
fn cancel_and_last_owner_loss_dispose_every_legal_installation_stage() {
    for stage in 0..4 {
        let p = Arc::new(Probe::default());
        let (mut r, b) = runtime(&p);
        if stage > 0 {
            let _ = step(&mut r, 1);
        }
        if stage == 2 {
            p.pending_install.store(true, SeqCst);
            let _ = step(&mut r, 1);
        }
        if stage == 3 {
            let _ = step(&mut r, 1);
        }
        r.cancel();
        assert!(matches!(r.first_cause(), Some(CoreError::Cancelled(_))));
        assert!(step(&mut r, 0).is_pending());
        drop(r);
        empty(&b);
        assert_eq!(p.live.load(SeqCst), 0);
        assert_eq!(p.handler_dropped.load(SeqCst), 1);
        assert_eq!(p.dropped.load(SeqCst), usize::from(stage > 0));
    }
}
#[test]
fn expired_deadline_and_policy_drift_refuse_installation_then_drain() {
    let p = Arc::new(Probe::default());
    let limits = GatewayDefaultV1::LIMITS.clone();
    let b = bank(&limits);
    let authority = authority(instance(0));
    let mut r = accepted(Runtime::try_new(
        prepared(&p, &authority),
        authority,
        limits,
        b.clone(),
        Deadline::at(now()),
    ));
    let _ = step(&mut r, 0);
    assert!(matches!(r.first_cause(), Some(CoreError::TimedOut(_))));
    assert_eq!(p.created.load(SeqCst), 0);
    let _ = step(&mut r, 1);
    drop(r);
    empty(&b);
    let (mut r, b) = runtime(&p);
    let _ = step(&mut r, 1);
    p.policy.store(1, SeqCst);
    let _ = step(&mut r, 1);
    assert!(matches!(r.first_cause(), Some(CoreError::Validation(_))));
    assert_eq!(p.live.load(SeqCst), 0);
    let _ = step(&mut r, 1);
    drop(r);
    empty(&b);
}
#[test]
fn rejected_complete_object_transfer_and_sibling_installation_preserve_ownership() {
    struct Refuse;
    impl<T> CleanupTransferTarget<T> for Refuse {
        fn try_accept(&mut self, t: CleanupTransferEnvelope<T>) -> CleanupTransferAcceptance<T> {
            CleanupTransferAcceptance::Rejected(t)
        }
    }
    let p = Arc::new(Probe::default());
    let (mut first, b1) = runtime(&p);
    let (mut sibling, b2) = runtime(&p);
    let _ = step(&mut first, 1);
    let _ = step(&mut first, 1);
    let _ = step(&mut sibling, 1);
    let _ = step(&mut sibling, 1);
    assert_eq!(p.live.load(SeqCst), 2);
    first.cancel();
    let subject = CleanupSlotId::new(SlotIndex::new(1), Generation::INITIAL);
    let destination = CleanupSlotId::new(SlotIndex::new(2), Generation::INITIAL);
    let request = CleanupPhaseContext::bind_with_transfer_owner(
        CleanupReservation::new(subject, BindingLifetimeFootprint::new(1, 8192), 1, work(8)),
        destination,
        CleanupOperation::CancelProcess,
        first.first_cause().unwrap().clone(),
        Deadline::NONE,
    )
    .try_into_transfer_request()
    .unwrap();
    let CleanupTransferAcceptance::Rejected(original) =
        Refuse.try_accept(CleanupTransferEnvelope::new(request, first))
    else {
        panic!("refused")
    };
    assert_eq!(p.dropped.load(SeqCst), 0);
    let (_, mut first) = original.into_parts();
    let _ = step(&mut first, 1);
    drop(first);
    empty(&b1);
    assert_eq!(p.live.load(SeqCst), 1);
    assert_eq!(sibling.state(), ProducerInstallationState::InstalledClosed);
    drop(sibling);
    empty(&b2);
    assert_eq!(p.live.load(SeqCst), 0);
}

#[test]
fn two_logical_instances_can_retain_one_physical_native_owner() {
    struct Session(Arc<AtomicUsize>);
    impl Drop for Session {
        fn drop(&mut self) {
            self.0.fetch_add(1, SeqCst);
        }
    }
    struct SharedExecutor(Arc<Session>);
    struct SharedInstallation(Option<Arc<Session>>);
    impl ProducerInstallation for SharedInstallation {
        fn poll_install(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
            self.0 = None;
            Poll::Ready(Ok(()))
        }
    }
    impl ProducerExecutor for SharedExecutor {
        type Configuration = ();
        type Program = ();
        type Installation = SharedInstallation;
        fn configuration_matches(&self, _: &()) -> bool {
            true
        }
        fn identity(&self) -> BindingRegistrationIdentity {
            BindingRegistrationIdentity::new(
                BindingId::new(900),
                BindingGeneration::INITIAL,
                BindingConfigurationDigest::new([1; 32]),
                BindingArtifactCompatibility::new([1; 16]),
                0,
            )
        }
        fn install_policy(&self) -> ProducerInstallPolicy {
            ProducerInstallPolicy {
                executor_bytes: 0,
                program_bytes: 0,
                installation_bytes: 0,
                temporary_bytes: 0,
                largest_indirect_allocation: 0,
                local_drop_work: 1,
            }
        }
        fn check_program(&self, _: &()) -> CoreResult<()> {
            Ok(())
        }
        fn create_installation(
            &self,
            _: &ProducerExecutionEntry<()>,
        ) -> CoreResult<SharedInstallation> {
            Ok(SharedInstallation(Some(self.0.clone())))
        }
    }
    struct SharedCompiler;
    impl ProducerCompiler<SharedExecutor> for SharedCompiler {
        fn configuration(&self) -> &() {
            &()
        }
        fn compile(
            &self,
            _: ResolvedProducerInput<'_>,
            _: &mut WorkBudget,
        ) -> CoreResult<Option<()>> {
            Ok(Some(()))
        }
    }
    let closed = Arc::new(AtomicUsize::new(0));
    let physical = Arc::new(Session(closed.clone()));
    let limits = GatewayDefaultV1::LIMITS.clone();
    let b = bank(&limits);
    let construct = |instance| {
        let authority = authority(instance);
        let td = td();
        let mut a = coverage(&td);
        let pair = accepted(ProducerBindingPreparation::try_new(
            SharedCompiler,
            SharedExecutor(physical.clone()),
            (),
        ));
        let capability = ProducerHandlerRegistration::new(
            &authority,
            "".into(),
            Handler(Arc::new(Probe::default())),
            HandlerFootprint::new(std::mem::size_of::<Handler>() as u64, 0, 0),
        );
        let prepared = accepted(pair.prepare_property_read(
            &mut a,
            &authority,
            capability,
            &mut work(1_000_000),
        ));
        accepted(ProducerInstallationRuntime::try_new(
            prepared,
            authority,
            limits.clone(),
            b.clone(),
            Deadline::NONE,
        ))
    };
    let mut first = construct(instance(0));
    let mut sibling = construct(instance(1));
    drop(physical);
    for r in [&mut first, &mut sibling] {
        let mut cx = Context::from_waker(Waker::noop());
        let _ = r.step(&mut cx, &mut work(1), now());
        let _ = r.step(&mut cx, &mut work(1), now());
    }
    first.begin_close();
    let _ = first.step(&mut Context::from_waker(Waker::noop()), &mut work(1), now());
    drop(first);
    assert_eq!(
        closed.load(SeqCst),
        0,
        "one instance released a sibling's native session"
    );
    assert_eq!(sibling.state(), ProducerInstallationState::InstalledClosed);
    drop(sibling);
    assert_eq!(closed.load(SeqCst), 1);
    empty(&b);
}

#[test]
fn concurrent_admissions_preserve_original_capacity_and_release_without_a_poller() {
    const OWNERS: usize = 8;
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(ResourceKind::HandlerSlotsGlobalMax, Some(1))
        .unwrap();
    let parent = bank(&limits);
    let start = Arc::new(std::sync::Barrier::new(OWNERS));
    let admitted = Arc::new(std::sync::Barrier::new(OWNERS));
    let p = Arc::new(Probe::default());
    let inputs: Vec<_> = (0..OWNERS)
        .map(|i| {
            let authority = authority(instance(i as u32));
            (prepared(&p, &authority), authority)
        })
        .collect();
    let successes = std::thread::scope(|scope| {
        let workers: Vec<_> = inputs
            .into_iter()
            .map(|(prepared, authority)| {
                let parent = parent.clone();
                let start = start.clone();
                let admitted = admitted.clone();
                let limits = limits.clone();
                scope.spawn(move || {
                    start.wait();
                    let result = Runtime::try_new(
                        prepared,
                        authority,
                        limits,
                        parent.clone(),
                        Deadline::NONE,
                    );
                    let success = result.is_ok();
                    admitted.wait();
                    assert!(parent.snapshot().iter().all(|a| a.used() <= a.limit()));
                    drop(result); // includes partial-reservation rollback on refusal
                    success as usize
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .sum::<usize>()
    });
    assert_eq!(successes, 1);
    empty(&parent);
    let slots = parent
        .snapshot()
        .into_iter()
        .find(|a| a.kind() == ResourceKind::HandlerSlotsGlobalMax)
        .unwrap();
    assert_eq!(slots.peak(), 1);
    assert_eq!(p.created.load(SeqCst), 0);
}

mod last_root {
    use super::*;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    #[test]
    fn parent_observation_cannot_block_reserved_or_closed_drop() {
        const MODE: &str = "CLINKZ_PRODUCER_DROP_MODE";
        const READY: &str = "CLINKZ_PRODUCER_DROP_READY";
        if let Some(mode) = std::env::var_os(MODE) {
            let p = Arc::new(Probe::default());
            let (mut runtime, bank) = runtime(&p);
            if mode == "closed" {
                runtime.begin_close();
                assert_eq!(
                    step(&mut runtime, 1),
                    Poll::Ready(ProducerInstallationState::Closed)
                );
            }
            // Inspection has no guard on the bank. A caller can retain the
            // snapshot and even its own application mutex across disposal.
            let mut snapshot = bank.snapshot();
            assert!(snapshot.iter().any(|a| a.used() > 0));
            for account in &mut snapshot {
                assert!(account.release_committed(account.used()));
            }
            assert!(
                bank.snapshot().iter().any(|a| a.used() > 0),
                "copies cannot release original charges"
            );
            let application_lock = Mutex::new(bank.clone());
            let held = if mode == "unlocked" {
                None
            } else {
                Some(application_lock.lock().unwrap())
            };
            // The parent starts its Drop watchdog only after this signal.
            std::fs::write(std::env::var_os(READY).unwrap(), b"ready").unwrap();
            drop(runtime);
            empty(&bank);
            drop(held);
            return;
        }

        let mut outcomes = Vec::new();
        for mode in ["unlocked", "reserved", "closed"] {
            let directory = std::env::temp_dir().join(format!(
                "clinkz-producer-drop-{}-{mode}",
                std::process::id(),
            ));
            std::fs::create_dir(&directory).unwrap();
            let ready = directory.join("ready");
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "last_root::parent_observation_cannot_block_reserved_or_closed_drop",
                    "--nocapture",
                ])
                .env(MODE, mode)
                .env(READY, &ready)
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let startup = Instant::now();
            while !ready.exists() && startup.elapsed() < Duration::from_secs(15) {
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let signalled = ready.exists();
            let drop_started = Instant::now();
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break Some(status);
                }
                if !signalled || drop_started.elapsed() >= Duration::from_millis(1500) {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            std::fs::remove_dir_all(directory).unwrap();
            let success = signalled && status.is_some_and(|s| s.success());
            eprintln!(
                "{mode}: signalled={signalled}; Drop returned and accounts released={success}"
            );
            outcomes.push((mode, success));
        }
        assert!(outcomes.iter().all(|(_, success)| *success), "{outcomes:?}");
    }
}

mod funding {
    use super::*;
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
    };

    const SCRATCH: usize = 8192;
    thread_local! {
        static TRACE: Cell<bool> = const { Cell::new(false) };
        static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    }
    struct AllocationObserver;
    // Forward every allocation unchanged; count only the opted-in test thread.
    unsafe impl GlobalAlloc for AllocationObserver {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if TRACE.try_with(Cell::get).unwrap_or(false) {
                let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
            }
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            unsafe { System.dealloc(pointer, layout) }
        }
    }
    #[global_allocator]
    static ALLOCATOR: AllocationObserver = AllocationObserver;
    fn allocations<T>(f: impl FnOnce() -> T) -> (T, usize) {
        ALLOCATIONS.with(|n| n.set(0));
        TRACE.with(|n| n.set(true));
        let result = f();
        TRACE.with(|n| n.set(false));
        (result, ALLOCATIONS.with(Cell::get))
    }

    struct FundingProbe {
        bank: Arc<ProducerInstallationAccounts>,
        observed: Mutex<[u64; 3]>,
        pending_close: AtomicBool,
        fail_close: AtomicBool,
        compiler_queries: AtomicUsize,
        executor_queries: AtomicUsize,
        compiler_calls: AtomicUsize,
    }
    impl FundingProbe {
        fn new(bank: Arc<ProducerInstallationAccounts>) -> Arc<Self> {
            Arc::new(Self {
                bank,
                observed: Mutex::new([0; 3]),
                pending_close: AtomicBool::new(false),
                fail_close: AtomicBool::new(false),
                compiler_queries: AtomicUsize::new(0),
                executor_queries: AtomicUsize::new(0),
                compiler_calls: AtomicUsize::new(0),
            })
        }
        fn scratch(&self) {
            // Real declared controlled scratch, simultaneously live with the
            // retained owner in install, close and finite local destruction.
            let scratch = vec![0x5au8; SCRATCH];
            std::hint::black_box(&scratch);
            *self.observed.lock().unwrap() = used(&self.bank);
            drop(scratch);
        }
    }
    fn used(bank: &Arc<ProducerInstallationAccounts>) -> [u64; 3] {
        let bank = bank.snapshot();
        [
            ResourceKind::EngineLiveBytesGlobalMax,
            ResourceKind::CompiledRuntimeBytesGlobalMax,
            ResourceKind::BindingPollTemporaryBytesGlobalMax,
        ]
        .map(|kind| bank.iter().find(|a| a.kind() == kind).unwrap().used())
    }
    // Deliberately move-only: pairing and runtime must never need Clone.
    #[derive(Eq, PartialEq)]
    struct FundingConfiguration(Vec<u8>);
    struct FundingExecutor {
        configuration: FundingConfiguration,
        probe: Arc<FundingProbe>,
    }
    struct FundingCompiler(FundingConfiguration, Arc<FundingProbe>);
    struct FundingInstallation(Arc<FundingProbe>);
    impl ProducerCompiler<FundingExecutor> for FundingCompiler {
        fn configuration(&self) -> &FundingConfiguration {
            self.1.compiler_queries.fetch_add(1, SeqCst);
            &self.0
        }
        fn compile(
            &self,
            _: ResolvedProducerInput<'_>,
            _: &mut WorkBudget,
        ) -> CoreResult<Option<()>> {
            self.1.compiler_calls.fetch_add(1, SeqCst);
            Ok(Some(()))
        }
    }
    impl ProducerExecutor for FundingExecutor {
        type Configuration = FundingConfiguration;
        type Program = ();
        type Installation = FundingInstallation;
        fn configuration_matches(&self, captured: &FundingConfiguration) -> bool {
            self.probe.executor_queries.fetch_add(1, SeqCst);
            self.configuration == *captured
        }
        fn identity(&self) -> BindingRegistrationIdentity {
            BindingRegistrationIdentity::new(
                BindingId::new(901),
                BindingGeneration::INITIAL,
                BindingConfigurationDigest::new([2; 32]),
                BindingArtifactCompatibility::new([2; 16]),
                0,
            )
        }
        fn install_policy(&self) -> ProducerInstallPolicy {
            ProducerInstallPolicy {
                // Both actual executor and captured configuration vectors.
                executor_bytes: (2 * SCRATCH) as u64,
                program_bytes: 0,
                installation_bytes: 0,
                temporary_bytes: SCRATCH as u64,
                largest_indirect_allocation: SCRATCH as u64,
                local_drop_work: 1,
            }
        }
        fn check_program(&self, _: &()) -> CoreResult<()> {
            Ok(())
        }
        fn create_installation(
            &self,
            _: &ProducerExecutionEntry<()>,
        ) -> CoreResult<FundingInstallation> {
            Ok(FundingInstallation(self.probe.clone()))
        }
    }
    impl ProducerInstallation for FundingInstallation {
        fn poll_install(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
            self.0.scratch();
            Poll::Ready(Ok(()))
        }
        fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
            self.0.scratch();
            if self.0.fail_close.load(SeqCst) {
                Poll::Ready(Err(failure(3)))
            } else if self.0.pending_close.load(SeqCst) {
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }
    impl Drop for FundingInstallation {
        fn drop(&mut self) {
            self.0.scratch();
        }
    }
    type FundingRuntime = ProducerInstallationRuntime<FundingExecutor, Handler>;
    fn prepare(
        probe: &Arc<FundingProbe>,
    ) -> (
        PreparedProducer<FundingExecutor, Handler>,
        ProducerInstanceAuthority,
    ) {
        let authority = authority(instance(0));
        let td = td();
        let mut admission = coverage(&td);
        let pair = accepted(ProducerBindingPreparation::try_new(
            FundingCompiler(FundingConfiguration(vec![7; SCRATCH]), probe.clone()),
            FundingExecutor {
                configuration: FundingConfiguration(vec![7; SCRATCH]),
                probe: probe.clone(),
            },
            FundingConfiguration(vec![7; SCRATCH]),
        ));
        let prepared = accepted(pair.prepare_property_read(
            &mut admission,
            &authority,
            registered(&Arc::new(Probe::default()), "", &authority),
            &mut work(1_000_000),
        ));
        (prepared, authority)
    }
    fn poll(runtime: &mut FundingRuntime) -> Poll<ProducerInstallationState> {
        runtime.step(&mut Context::from_waker(Waker::noop()), &mut work(1), now())
    }

    #[test]
    fn pairing_moves_provisioned_configuration_without_allocating_on_mismatch_or_zero_capacity() {
        let generous = GatewayDefaultV1::LIMITS.clone();
        let parent = bank(&generous);
        let probe = FundingProbe::new(parent.clone());
        let handler_probe = Arc::new(Probe::default());
        for (compiler_value, executor_value) in [(9, 7), (7, 9), (9, 9), (7, 7)] {
            let configuration = FundingConfiguration(vec![7u8; SCRATCH]);
            let snapshot_pointer = configuration.0.as_ptr();
            let compiler = FundingCompiler(
                FundingConfiguration(vec![compiler_value; SCRATCH]),
                probe.clone(),
            );
            let compiler_pointer = compiler.0.0.as_ptr();
            let executor = FundingExecutor {
                configuration: FundingConfiguration(vec![executor_value; SCRATCH]),
                probe: probe.clone(),
            };
            let executor_pointer = executor.configuration.0.as_ptr();
            let (result, count) = allocations(|| {
                ProducerBindingPreparation::try_new(compiler, executor, configuration)
            });
            assert_eq!(
                count, 0,
                "pairing must not create an unfunded configuration clone"
            );
            if compiler_value != 7 || executor_value != 7 {
                let refusal = result.err().expect("complete configuration mismatch");
                assert!(matches!(refusal.error().error(), CoreError::Validation(_)));
                let (compiler, executor, configuration) = refusal.into_input();
                assert_eq!(compiler.0.0.as_ptr(), compiler_pointer);
                assert_eq!(executor.configuration.0.as_ptr(), executor_pointer);
                assert_eq!(configuration.0.as_ptr(), snapshot_pointer);
            } else {
                let authority = authority(instance(0));
                let td = td();
                let mut admission = coverage(&td);
                let prepared = accepted(accepted(result).prepare_property_read(
                    &mut admission,
                    &authority,
                    registered(&handler_probe, "", &authority),
                    &mut work(1_000_000),
                ));
                assert_eq!(prepared.configuration().0.as_ptr(), snapshot_pointer);
                let zero = generous
                    .clone()
                    .try_with_limit(ResourceKind::EngineLiveBytesGlobalMax, Some(0))
                    .unwrap();
                let (result, count) = allocations(|| {
                    FundingRuntime::try_new(
                        prepared,
                        authority,
                        zero,
                        parent.clone(),
                        Deadline::NONE,
                    )
                });
                assert_eq!(
                    count, 0,
                    "zero-capacity runtime admission must not copy the snapshot"
                );
                let refusal = result.err().expect("zero live capacity");
                assert!(matches!(
                    refusal.error().error(),
                    CoreError::LimitExceeded {
                        resource: ResourceKind::EngineLiveBytesGlobalMax,
                        limit: 0,
                        ..
                    }
                ));
                assert_eq!(
                    refusal.input().0.configuration().0.as_ptr(),
                    snapshot_pointer
                );
                assert_eq!(
                    refusal.input().0.entry().instance_key(),
                    refusal.input().1.key()
                );
                assert!(Arc::ptr_eq(&refusal.input().3, &parent));
                assert_eq!(handler_probe.handler_dropped.load(SeqCst), 0);
                drop(refusal);
                assert_eq!(handler_probe.handler_dropped.load(SeqCst), 1);
            }
            empty(&parent);
        }
    }

    #[test]
    fn zero_binding_credit_preparation_retries_keep_owned_configuration_without_allocating() {
        let generous = GatewayDefaultV1::LIMITS.clone();
        let parent = bank(&generous);
        let probe = FundingProbe::new(parent.clone());
        let authority = authority(instance(0));
        let td = td();
        let mut admission = coverage(&td);
        let configuration = FundingConfiguration(vec![7u8; SCRATCH]);
        let (copy, control) = allocations(|| configuration.0.clone());
        assert_eq!(
            control, 1,
            "an owned configuration clone must be observable"
        );
        drop(copy);
        let mut pair = accepted(ProducerBindingPreparation::try_new(
            FundingCompiler(FundingConfiguration(vec![7; SCRATCH]), probe.clone()),
            FundingExecutor {
                configuration: FundingConfiguration(vec![7; SCRATCH]),
                probe: probe.clone(),
            },
            configuration,
        ));
        let handler_probe = Arc::new(Probe::default());
        let mut handler = registered(&handler_probe, "", &authority);
        probe.compiler_queries.store(0, SeqCst);
        probe.executor_queries.store(0, SeqCst);
        for _ in 0..4 {
            // Give TD traversal/allocation ample credit, isolating the absent
            // BindingPolls allowance rather than exhausting every work class.
            let mut budget = work(1_000_000).with_remaining(WorkClass::BindingPolls, 0);
            let before = WorkClass::ALL.map(|class| budget.remaining(class));
            let (result, count) = allocations(|| {
                pair.prepare_property_read(&mut admission, &authority, handler, &mut budget)
            });
            assert_eq!(
                count, 0,
                "zero-credit retry must not clone compiler configuration"
            );
            assert_eq!(probe.compiler_queries.load(SeqCst), 0);
            assert_eq!(probe.executor_queries.load(SeqCst), 0);
            assert_eq!(probe.compiler_calls.load(SeqCst), 0);
            let refusal = result.err().expect("insufficient BindingPolls credit");
            assert!(matches!(
                refusal.error().error(),
                CoreError::Backpressure(_)
            ));
            assert_eq!(WorkClass::ALL.map(|class| budget.remaining(class)), before);
            (pair, handler) = refusal.into_input();
            assert_eq!(handler.instance_key(), authority.key());
            assert_eq!(handler_probe.handler_dropped.load(SeqCst), 0);
            empty(&parent);
        }
        let prepared = accepted(pair.prepare_property_read(
            &mut admission,
            &authority,
            handler,
            &mut work(1_000_000),
        ));
        assert_eq!(prepared.configuration().0.len(), SCRATCH);
        assert_eq!(prepared.entry().instance_key(), authority.key());
        assert_eq!(probe.compiler_queries.load(SeqCst), 1);
        assert_eq!(probe.executor_queries.load(SeqCst), 1);
        assert_eq!(probe.compiler_calls.load(SeqCst), 1);
        assert_eq!(handler_probe.handler_dropped.load(SeqCst), 0);
        drop(prepared);
        assert_eq!(handler_probe.handler_dropped.load(SeqCst), 1);
        empty(&parent);
    }

    #[test]
    fn controlled_scratch_participates_in_exact_local_and_original_parent_live_peak() {
        let generous = GatewayDefaultV1::LIMITS.clone();
        let b = bank(&generous);
        let probe = FundingProbe::new(b.clone());
        let (prepared, authority) = prepare(&probe);
        let runtime = accepted(FundingRuntime::try_new(
            prepared,
            authority,
            generous.clone(),
            b.clone(),
            Deadline::NONE,
        ));
        let retained = used(&b)[1];
        let peak = retained.checked_add(SCRATCH as u64).unwrap();
        assert_eq!(used(&b), [peak, retained, SCRATCH as u64]);
        drop(runtime);
        empty(&b);

        let sibling_probe = Arc::new(Probe::default());
        let (sibling, sibling_bank) = super::runtime(&sibling_probe);
        let sibling_live = used(&sibling_bank)[0];
        drop(sibling);
        empty(&sibling_bank);
        for original_parent in [false, true] {
            for available in [peak - 1, peak] {
                let inherited = if original_parent { sibling_live } else { 0 };
                let restricted = generous
                    .clone()
                    .try_with_limit(
                        ResourceKind::EngineLiveBytesGlobalMax,
                        Some(available + inherited),
                    )
                    .unwrap();
                let (local, b) = if original_parent {
                    (generous.clone(), bank(&restricted))
                } else {
                    (restricted, bank(&generous))
                };
                let sibling = if original_parent {
                    let authority = super::authority(instance(1));
                    Some(accepted(Runtime::try_new(
                        super::prepared(&sibling_probe, &authority),
                        authority,
                        generous.clone(),
                        b.clone(),
                        Deadline::NONE,
                    )))
                } else {
                    None
                };
                let baseline = used(&b);
                let probe = FundingProbe::new(b.clone());
                let (prepared, authority) = prepare(&probe);
                let key = authority.key();
                let result =
                    FundingRuntime::try_new(prepared, authority, local, b.clone(), Deadline::NONE);
                if available < peak {
                    let refusal = result.err().expect("one below the simultaneous peak");
                    assert_eq!(refusal.input().1.key(), key);
                    assert!(Arc::ptr_eq(&refusal.input().3, &b));
                    assert!(matches!(refusal.error().error(), CoreError::LimitExceeded {
                        resource: ResourceKind::EngineLiveBytesGlobalMax,
                        requested: Some(n), ..
                    } if *n == peak));
                    assert_eq!(used(&b), baseline);
                } else {
                    let mut runtime = accepted(result);
                    let funded = [
                        inherited + peak,
                        baseline[1] + retained,
                        baseline[2] + SCRATCH as u64,
                    ];
                    assert_eq!(used(&b), funded);
                    assert_eq!(
                        poll(&mut runtime),
                        Poll::Ready(ProducerInstallationState::Installing)
                    );
                    assert_eq!(
                        poll(&mut runtime),
                        Poll::Ready(ProducerInstallationState::InstalledClosed)
                    );
                    assert_eq!(*probe.observed.lock().unwrap(), funded);
                    runtime.cancel();
                    probe.pending_close.store(true, SeqCst);
                    assert!(poll(&mut runtime).is_pending());
                    probe.pending_close.store(false, SeqCst);
                    probe.fail_close.store(true, SeqCst);
                    assert_eq!(
                        poll(&mut runtime),
                        Poll::Ready(ProducerInstallationState::Closing)
                    );
                    assert_eq!(used(&b), funded);
                    assert!(matches!(
                        runtime.first_cause(),
                        Some(CoreError::Cancelled(_))
                    ));
                    probe.fail_close.store(false, SeqCst);
                    assert_eq!(
                        poll(&mut runtime),
                        Poll::Ready(ProducerInstallationState::Closed)
                    );
                    assert_eq!(
                        *probe.observed.lock().unwrap(),
                        funded,
                        "scratch in native Drop stays funded"
                    );
                    assert_eq!(
                        used(&b),
                        [
                            inherited + std::mem::size_of::<FundingRuntime>() as u64,
                            baseline[1],
                            baseline[2]
                        ]
                    );
                    drop(runtime);
                    assert_eq!(used(&b), baseline);
                }
                drop(sibling);
                empty(&b);
            }
        }
    }

    #[test]
    fn allocating_configuration_is_not_cloned_before_zero_capacity_refusal_or_at_runtime() {
        let generous = GatewayDefaultV1::LIMITS.clone();
        let b = bank(&generous);
        let probe = FundingProbe::new(b.clone());
        let (prepared, authority) = prepare(&probe);
        let (copy, control) = allocations(|| prepared.configuration().0.clone());
        assert_eq!(
            control, 1,
            "the observer must see an owned configuration clone"
        );
        assert_eq!(copy.len(), SCRATCH);
        drop(copy);
        let zero = [
            ResourceKind::EngineLiveBytesGlobalMax,
            ResourceKind::BindingPollTemporaryBytesPerCallMax,
            ResourceKind::BindingPollTemporaryBytesGlobalMax,
        ]
        .into_iter()
        .fold(generous.clone(), |limits, kind| {
            limits.try_with_limit(kind, Some(0)).unwrap()
        });
        let key = authority.key();
        let (result, count) = allocations(|| {
            FundingRuntime::try_new(prepared, authority, zero, b.clone(), Deadline::NONE)
        });
        assert_eq!(
            count, 0,
            "capacity refusal must not allocate a configuration snapshot"
        );
        let refusal = result.err().expect("zero capacity refusal");
        assert_eq!(refusal.input().1.key(), key);
        assert_eq!(refusal.input().0.configuration().0.len(), SCRATCH);
        empty(&b);
        let ((prepared, authority, _, original), _) = refusal.into_parts();
        let (result, count) = allocations(|| {
            FundingRuntime::try_new(prepared, authority, generous, original, Deadline::NONE)
        });
        assert_eq!(count, 1, "only the funded fallible owner is allocated");
        let mut runtime = accepted(result);
        let (state, count) = allocations(|| poll(&mut runtime));
        assert_eq!(state, Poll::Ready(ProducerInstallationState::Installing));
        assert_eq!(
            count, 0,
            "construction checks borrow the captured configuration"
        );
        let (state, count) = allocations(|| poll(&mut runtime));
        assert_eq!(
            state,
            Poll::Ready(ProducerInstallationState::InstalledClosed)
        );
        assert_eq!(count, 1, "the only callback allocation is declared scratch");
        drop(runtime);
        empty(&b);
    }
}
