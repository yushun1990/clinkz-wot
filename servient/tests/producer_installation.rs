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
    fn configuration(&self) -> usize {
        self.config
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
    fn configuration(&self) -> usize {
        self.0.policy.load(SeqCst)
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
fn bank(limits: &ResourceLimits) -> Arc<Mutex<ProducerInstallationAccounts>> {
    Arc::new(Mutex::new(producer_installation_accounts(limits)))
}
fn empty(bank: &Arc<Mutex<ProducerInstallationAccounts>>) {
    assert!(bank.lock().unwrap().iter().all(|a| a.used() == 0));
}
fn authority(instance: ThingSlotId) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(instance).unwrap()
}
fn runtime(p: &Arc<Probe>) -> (Runtime, Arc<Mutex<ProducerInstallationAccounts>>) {
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
        b.lock().unwrap().iter().any(|a| a.used() > 0),
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
    )
    .err()
    .unwrap();
    let (c, e) = refused.into_input();
    assert_eq!(c.config, 1);
    assert_eq!(e.configuration(), 0);
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
    let attempt = |p: &Arc<Probe>, limits, b: Arc<Mutex<ProducerInstallationAccounts>>| {
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
    let b = bank(&limits);
    {
        let mut bank = b.lock().unwrap();
        let i = bank.len() - 1;
        let n = bank[i].limit();
        bank[i].try_reserve(n).unwrap().commit();
    }
    let before: Vec<_> = b.lock().unwrap().iter().map(|a| a.used()).collect();
    assert!(attempt(&p, limits, b.clone()).is_err());
    assert_eq!(
        b.lock()
            .unwrap()
            .iter()
            .map(|a| a.used())
            .collect::<Vec<_>>(),
        before
    );
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
        fn configuration(&self) {}
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
        fn configuration(&self) {}
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
