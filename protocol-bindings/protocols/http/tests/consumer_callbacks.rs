#![cfg(feature = "prepare")]
//! Callback isolation through public TD preparation and finite Consumer APIs.
//! Local native work makes adversarial scheduling deterministic; real HTTP
//! evidence uses the unmodified executor in the final tests below.
#[path = "support/prepared_read_callbacks.rs"]
mod prepared_read;
#[path = "support/qualification_adoption.rs"]
mod qualification_adoption;
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{consumer::*, data_schema::PreparedSchema, host_accounts::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{HostConsumerBindingPreparation, open_consumer::*};
use clinkz_wot_servient_finite::consumer::*;
use serde_json::json;
use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Resolve,
    Access,
    Issue,
    Validate,
    Start,
    Poll,
    Cancel,
    Dispose,
    Wake,
    WakerDrop,
    ServiceDrop,
}
type Hook = (Edge, Box<dyn FnOnce()>);
thread_local! { static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) }; }
fn hook(edge: Edge, callback: impl FnOnce() + 'static) {
    HOOK.with(|h| assert!(h.borrow_mut().replace((edge, Box::new(callback))).is_none()));
}
fn fire(edge: Edge) {
    let callback = HOOK.with(|h| {
        let mut hook = h.borrow_mut();
        if hook.as_ref().is_some_and(|(at, _)| *at == edge) {
            hook.take()
        } else {
            None
        }
    });
    if let Some((_, callback)) = callback {
        callback();
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
fn now() -> MonotonicInstant {
    at(1)
}
fn at(ticks: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(106), ticks)
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn clean(bank: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(bank.account(k).used(), 0, "{k:?}");
    }
}
fn usage(bank: &HostResourceAccounts) -> Vec<u64> {
    ResourceKind::ALL
        .into_iter()
        .map(|k| bank.account(k).used())
        .collect()
}
struct Data(Arc<dyn ConsumerDataService>, Arc<Probe>);
impl ConsumerDataService for Data {
    fn qualification_revision(&self) -> u64 {
        self.1.data_revision.load(Ordering::Acquire)
    }
    fn content_type(&self) -> &str {
        self.0.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut policy = self.0.policy();
        if self.1.changed_bounds.load(Ordering::Acquire) {
            policy.temporary_bytes += 1;
        }
        policy
    }
    fn supports_media(&self, media: &str) -> bool {
        self.1.qualifications.fetch_add(1, Ordering::Relaxed);
        self.0.supports_media(media)
    }
    fn supports_schema(&self, s: &PreparedSchema) -> bool {
        self.1.qualifications.fetch_add(1, Ordering::Relaxed);
        self.0.supports_schema(s)
    }
    fn validate(&self, s: &PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.1.validations.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Validate);
        self.0.validate(s, p)
    }
}
struct Access(Arc<dyn ConsumerAccessService>, Arc<Probe>);
impl Drop for Access {
    fn drop(&mut self) {
        fire(Edge::ServiceDrop);
    }
}
impl ConsumerAccessService for Access {
    fn qualification_revision(&self) -> u64 {
        self.1.access_revision.load(Ordering::Acquire)
    }
    fn supports_security(&self, requirement: &security::SecurityRequirement) -> bool {
        self.1.qualifications.fetch_add(1, Ordering::Relaxed);
        self.0.supports_security(requirement)
    }
    fn authorize(&self, b: BindingRegistrationIdentity, t: &str, o: Operation) -> CoreResult<()> {
        self.0.authorize(b, t, o)
    }
    fn authorize_instance(
        &self,
        c: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        t: &str,
        o: Operation,
    ) -> CoreResult<()> {
        self.1
            .permission_destination
            .store(t.as_ptr() as usize, Ordering::Relaxed);
        fire(Edge::Access);
        self.0.authorize_instance(c, b, t, o)
    }
    fn outbound_security(
        &self,
        c: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        t: &str,
        o: Operation,
        requirement: &security::SecurityRequirement,
    ) -> CoreResult<Option<security::OutboundSecurity>> {
        fire(Edge::Issue);
        self.0.outbound_security(c, b, t, o, requirement)
    }
    fn retained_bytes(&self) -> u64 {
        self.0.retained_bytes()
    }
    fn work(&self) -> u64 {
        self.0.work()
    }
}
#[derive(Default)]
struct Probe {
    resolved_destination: AtomicUsize,
    native_destination: AtomicUsize,
    permission_destination: AtomicUsize,
    qualifications: AtomicUsize,
    resolutions: AtomicUsize,
    validations: AtomicUsize,
    data_revision: AtomicU64,
    native_revision: AtomicU64,
    access_revision: AtomicU64,
    changed_bounds: AtomicBool,
    starts: AtomicUsize,
    polls: AtomicUsize,
    cancels: AtomicUsize,
    drops: AtomicUsize,
    pending: AtomicBool,
}
struct LocalClient {
    configuration: ConsumerBindingConfiguration,
    compatibility: BindingArtifactCompatibility,
    probe: Arc<Probe>,
}
impl binding::ClientBinding for LocalClient {
    fn qualification_revision(&self) -> u64 {
        self.probe.native_revision.load(Ordering::Acquire)
    }
    fn supports_operation(&self, o: Operation) -> bool {
        self.probe.qualifications.fetch_add(1, Ordering::Relaxed);
        matches!(o, Operation::ReadProperty | Operation::WriteProperty)
    }
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        Some(self.configuration)
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn resolve_target(
        &self,
        target: &str,
        variables: &[(&str, &str)],
        _: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> CoreResult<Box<str>> {
        self.probe.resolutions.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Resolve);
        if !variables.is_empty() || target.len() > self.configuration.target_capacity() {
            return Err(invalid());
        }
        let destination: Box<str> = target.into();
        self.probe
            .resolved_destination
            .store(destination.as_ptr() as usize, Ordering::Relaxed);
        Ok(destination)
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        _: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        if let Some(destination) = request.destination() {
            self.probe
                .native_destination
                .store(destination.as_ptr() as usize, Ordering::Relaxed);
        }
        self.probe.starts.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Start);
        HostBindingCallBox::try_new(LocalCall {
            request: Some(request),
            probe: self.probe.clone(),
            waker: None,
        })
        .map_err(|mut call| {
            BindingInputRejection::new(
                call.request.take().unwrap(),
                BindingOperationalError::new(busy()),
            )
        })
    }
}
struct LocalCall {
    request: Option<OutboundRequest>,
    probe: Arc<Probe>,
    waker: Option<Waker>,
}
impl Drop for LocalCall {
    fn drop(&mut self) {
        self.probe.drops.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Dispose);
    }
}
impl HostBindingCall<CoreResult<InteractionOutput>> for LocalCall {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        BindingLifetimeFootprint::new(4, 4096)
    }
    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
    fn poll_result(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.probe.polls.fetch_add(1, Ordering::Relaxed);
        self.waker = Some(cx.waker().clone());
        fire(Edge::Poll);
        if self.probe.pending.load(Ordering::Relaxed) {
            return Poll::Pending;
        }
        let r = self.request.as_ref().unwrap();
        let output = if r.operation() == Operation::ReadProperty {
            InteractionOutput::with_data(Payload::new(b"7".as_slice(), "application/json"))
        } else {
            InteractionOutput::empty()
        };
        Poll::Ready(Ok(output
            .try_with_metadata(
                InteractionOutputMetadata::default().with_untrusted_binding_response(
                    BindingResponseMetadata::primary(
                        r.binding_id(),
                        r.binding_generation(),
                        r.plan_id(),
                        if r.operation() == Operation::ReadProperty {
                            200
                        } else {
                            204
                        },
                    ),
                ),
            )
            .unwrap()))
    }
    fn start_cancel(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.probe.cancels.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Cancel);
        Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: NoCleanupSuccessor,
            },
        }))
    }
    fn poll_cancel(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        let cleanup = CleanupPhaseContext::bind(
            CleanupReservation::new(
                CleanupSlotId::new(SlotIndex::new(0), Generation::INITIAL),
                self.lifetime_footprint(),
                0,
            ),
            CleanupOperation::CancelRequest,
            busy(),
            Deadline::NONE,
        );
        match self.start_cancel(cx, cleanup)? {
            StartStatus::Pending => Poll::Pending,
            StartStatus::Ready(s) => Poll::Ready(Ok(s)),
        }
    }
}
struct Harness {
    rt: Rc<(StandaloneConsumerAssociation, ConsumerCalls)>,
    owner: ConsumerInstance,
    bank: Arc<HostResourceAccounts>,
    probe: Arc<Probe>,
}
fn harness(calls: usize, address: Option<std::net::SocketAddr>) -> Harness {
    harness_slot(calls, address, 0)
}
fn harness_slot(calls: usize, address: Option<std::net::SocketAddr>, slot: u32) -> Harness {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL));
    let (rt, probe) = association_harness(calls, address, bank.clone(), owner.reference());
    Harness {
        rt,
        owner,
        bank,
        probe,
    }
}
fn association_harness(
    calls: usize,
    address: Option<std::net::SocketAddr>,
    bank: Arc<HostResourceAccounts>,
    consumer: ConsumerInstanceRef,
) -> (
    Rc<(StandaloneConsumerAssociation, ConsumerCalls)>,
    Arc<Probe>,
) {
    let probe = Arc::new(Probe::default());
    let mut input = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(106),
        BindingGeneration::INITIAL,
        106,
    )
    .unwrap();
    let services = input.services;
    input.services = ConsumerServices {
        data: Arc::new(Data(services.data, probe.clone())),
        access: Arc::new(Access(services.access, probe.clone())),
    };
    if address.is_none() {
        let (runtime, compiler) = input.preparation.into_parts();
        let native = LocalClient {
            configuration: runtime.consumer_configuration().unwrap(),
            compatibility: runtime.identity().artifact_compatibility(),
            probe: probe.clone(),
        };
        let local =
            HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
                runtime.identity(),
                runtime.execution(),
                Box::new(native),
                runtime.resources(),
                runtime.status(),
            ))
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        input.preparation = HostConsumerBindingPreparation::try_pair(local, compiler)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
    }
    let registry =
        ConsumerRegistry::try_new(vec![input], &bank).unwrap_or_else(|e| panic!("{:?}", e.error()));
    let addr = address.unwrap_or_else(|| "127.0.0.1:10600".parse().unwrap());
    let thing = serde_json::from_value(json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"callback ownership",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer","forms":[{"href":format!("http://{addr}/level")}]}}})).unwrap();
    let prepared = registry
        .prepare(
            &thing,
            GatewayDefaultV1::LIMITS,
            bank.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        )
        .unwrap();
    drop(thing);
    drop(registry);
    let (definitions, funding) = prepared.into_execution();
    let rt = Rc::new(
        StandaloneConsumerAssociation::try_new(consumer, definitions, funding, bank.clone())
            .map(|association| {
                (
                    association,
                    ConsumerCalls::try_new(bank.clone(), calls).unwrap(),
                )
            })
            .unwrap_or_else(|e| panic!("{:?}", e.error())),
    );
    (rt, probe)
}
fn read(rt: &(StandaloneConsumerAssociation, ConsumerCalls)) -> CoreResult<ConsumerCall> {
    rt.0.read_property(&rt.1, "level", Deadline::NONE, now())
}
fn settle(call: &mut ConsumerCall) -> ConsumerSettlement {
    for _ in 0..32 {
        if let Poll::Ready(Ok(s)) = call.poll(&mut cx(), now()) {
            return s;
        }
    }
    panic!("local call did not retire")
}
fn dispose(h: Harness) {
    let Harness {
        rt, owner, bank, ..
    } = h;
    drop(rt);
    drop(owner);
    clean(&bank);
    HOOK.with(|h| assert!(h.borrow().is_none(), "hook was not exercised"));
}

#[test]
fn losing_the_work_owner_during_native_poll_disposes_abandoned_siblings_and_restores_the_live_call()
{
    let Harness {
        rt,
        owner,
        bank,
        probe,
    } = harness(2, None);
    let (association, calls) = Rc::try_unwrap(rt).ok().unwrap();
    let association = Rc::new(association);
    probe.pending.store(true, Ordering::Release);
    let abandoned = association
        .read_property(&calls, "level", Deadline::NONE, now())
        .unwrap();
    let mut survivor = association
        .read_property(&calls, "level", Deadline::NONE, now())
        .unwrap();
    drop(abandoned);
    let controller = Rc::new(RefCell::new(Some(calls)));
    let (losing, closing, native, original) = (
        controller.clone(),
        association.clone(),
        probe.clone(),
        bank.clone(),
    );
    hook(Edge::Poll, move || {
        hook(Edge::Dispose, move || {
            // The abandoned slot is already vacant; the survivor remains
            // checked out and retains its separately reachable passive control.
            closing.begin_close();
            assert_eq!(native.drops.load(Ordering::Acquire), 1);
            assert_eq!(
                original
                    .account(ResourceKind::PendingHandlerCallsGlobalMax)
                    .used(),
                2
            );
        });
        drop(losing.borrow_mut().take());
    });
    assert!(survivor.poll(&mut cx(), now()).is_pending());
    assert!(controller.borrow().is_none());
    assert_eq!(probe.drops.load(Ordering::Acquire), 1);
    assert_eq!(
        bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used(),
        1
    );
    survivor.cancel(); // cannot replace the earlier loss of original authority
    let result = settle(&mut survivor);
    assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(result.consumer(), &owner.reference());
    assert_eq!(probe.cancels.load(Ordering::Acquire), 1);
    drop((result, survivor, association, controller, owner));
    assert_eq!(probe.drops.load(Ordering::Acquire), 2);
    clean(&bank);
}

#[test]
fn local_disposal_can_reentrantly_abandon_another_original_slot_after_control_loss() {
    let Harness {
        rt,
        owner,
        bank,
        probe,
    } = harness(2, None);
    let (association, calls) = Rc::try_unwrap(rt).ok().unwrap();
    let a = association
        .read_property(&calls, "level", Deadline::NONE, now())
        .unwrap();
    let b = Rc::new(RefCell::new(Some(
        association
            .read_property(&calls, "level", Deadline::NONE, now())
            .unwrap(),
    )));
    drop(a);
    let (other, original) = (b.clone(), bank.clone());
    hook(Edge::Dispose, move || {
        assert_eq!(
            original
                .account(ResourceKind::PendingHandlerCallsGlobalMax)
                .used(),
            2
        );
        drop(other.borrow_mut().take());
        assert_eq!(
            original
                .account(ResourceKind::PendingHandlerCallsGlobalMax)
                .used(),
            1
        );
    });
    drop(calls);
    assert!(b.borrow().is_none());
    assert_eq!(probe.drops.load(Ordering::Acquire), 2);
    assert_eq!(
        bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used(),
        0
    );
    drop((b, association, owner));
    clean(&bank);
}

#[test]
fn disposal_unwind_keeps_the_remaining_abandoned_sweep_despite_an_unrelated_retained_handle() {
    let Harness {
        rt,
        owner,
        bank,
        probe,
    } = harness(3, None);
    let (association, calls) = Rc::try_unwrap(rt).ok().unwrap();
    let mut retained = association
        .read_property(&calls, "level", Deadline::NONE, now())
        .unwrap();
    let result = settle(&mut retained);
    let a = association
        .read_property(&calls, "level", Deadline::NONE, now())
        .unwrap();
    let b = association
        .read_property(&calls, "level", Deadline::NONE, now())
        .unwrap();
    drop((a, b));
    hook(Edge::Dispose, || panic!("single linked disposal panic"));
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(calls)));
    assert!(failed.is_err());
    assert_eq!(probe.drops.load(Ordering::Acquire), 3);
    assert_eq!(
        bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used(),
        1
    );
    assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
    // The retained handle still holds State: its final Drop must not be needed
    // to dispose either abandoned sibling after the supported unwind exit.
    drop((result, retained, association, owner));
    assert_eq!(probe.drops.load(Ordering::Acquire), 3);
    clean(&bank);
}

#[test]
fn authorization_can_close_and_refuse_the_original_write_before_native_construction() {
    let h = harness(1, None);
    let before = usage(&h.bank);
    let rt = h.rt.clone();
    hook(Edge::Access, move || {
        assert!(!rt.1.is_idle());
        rt.1.begin_close();
    });
    let input = Payload::new(b"8".as_slice(), "application/json");
    let original = input.body.clone();
    let rejected =
        h.rt.0
            .write_property(&h.rt.1, "level", input, Deadline::NONE, now())
            .err()
            .unwrap();
    assert!(matches!(
        rejected.error().error(),
        CoreError::StaleHandle(_)
    ));
    let (input, _) = rejected.into_parts();
    assert!(Arc::ptr_eq(&original, &input.body));
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
    assert!(h.rt.1.is_idle());
    assert_eq!(before, usage(&h.bank));
    dispose(h);
}

#[test]
fn credential_callback_fences_refuse_native_acceptance_and_recover_original_input() {
    for http in [false, true] {
        for change in 0..5 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let h = harness(1, http.then(|| listener.local_addr().unwrap()));
            let before = usage(&h.bank);
            let rt = h.rt.clone();
            hook(Edge::Issue, move || {
                assert!(!rt.1.is_idle(), "admission must retain its reserved slot");
                match change {
                    0 => rt.0.begin_close(),
                    1 => rt.1.begin_close(),
                    2 => rt.1.step(&mut cx(), at(2)).unwrap(),
                    3 => rt.1.step(&mut cx(), at(0)).unwrap(),
                    _ => {
                        rt.1.begin_close();
                        rt.1.step(&mut cx(), at(0)).unwrap();
                    }
                }
            });
            let input = Payload::new(b"8".as_slice(), "application/json");
            let original = input.body.clone();
            let refused =
                h.rt.0
                    .write_property(&h.rt.1, "level", input, Deadline::at(at(2)), now())
                    .err()
                    .expect("credential callback fenced the original admission");
            match change {
                0 | 1 | 4 => assert!(matches!(refused.error().error(), CoreError::StaleHandle(_))),
                2 => assert!(matches!(refused.error().error(), CoreError::TimedOut(_))),
                _ => assert!(matches!(
                    refused.error().error(),
                    CoreError::InternalInvariant(_)
                )),
            }
            assert!(h.owner.reference().is_current(), "the parent stays current");
            let (input, _) = refused.into_parts();
            assert!(Arc::ptr_eq(&original, &input.body));
            assert_eq!(input.body.as_ref(), b"8");
            if !http {
                assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
                assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
                assert_eq!(h.probe.cancels.load(Ordering::Relaxed), 0);
                assert_eq!(h.probe.drops.load(Ordering::Relaxed), 0);
            }
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
            assert!(h.rt.1.is_idle());
            if change == 0 {
                // Closing releases unselected mapping/service backing now;
                // refusal must add no retained call or scratch debit.
                assert!(
                    usage(&h.bank)
                        .iter()
                        .zip(&before)
                        .all(|(after, before)| after <= before)
                );
            } else {
                assert_eq!(before, usage(&h.bank));
            }
            drop((input, original));
            dispose(h);
        }
    }
}

#[test]
fn nested_admission_defers_even_with_a_free_sibling_slot() {
    for capacity in [1, 2] {
        let h = harness(capacity, None);
        let rt = h.rt.clone();
        hook(Edge::Access, move || {
            rt.1.step(&mut cx(), now()).unwrap();
            assert!(matches!(read(&rt), Err(CoreError::Backpressure(_))));
        });
        let mut outer = read(&h.rt).unwrap();
        assert!(settle(&mut outer).result().is_ok());
        drop(outer);
        assert!(h.rt.1.is_idle());
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
        dispose(h);
    }
}

#[test]
fn advancing_nested_sweep_does_not_regress_a_siblings_call_time() {
    let h = harness(2, None);
    let mut first = read(&h.rt).unwrap();
    let mut sibling = read(&h.rt).unwrap();
    let rt = h.rt.clone();
    hook(Edge::Poll, move || rt.1.step(&mut cx(), at(2)).unwrap());
    h.rt.1.step(&mut cx(), at(1)).unwrap();
    for call in [&mut first, &mut sibling] {
        let Poll::Ready(Ok(s)) = call.poll(&mut cx(), at(3)) else {
            panic!("not ready")
        };
        assert!(s.result().is_ok(), "{:?}", s.result());
        drop(s);
    }
    drop(first);
    drop(sibling);
    dispose(h);
}

#[test]
fn nested_drives_across_three_capacities_defer_io_but_preserve_cancellation() {
    let first = harness(1, None);
    let second = harness(1, None);
    let third = harness(1, None);
    let mut a = read(&first.rt).unwrap();
    let mut b = read(&second.rt).unwrap();
    let c = Rc::new(read(&third.rt).unwrap());
    let second_driver = second.rt.clone();
    let third_driver = third.rt.clone();
    let cancel = c.clone();
    hook(Edge::Poll, move || {
        hook(Edge::Poll, move || {
            third_driver.1.step(&mut cx(), at(3)).unwrap()
        });
        second_driver.1.step(&mut cx(), at(2)).unwrap();
        cancel.cancel();
    });
    first.rt.1.step(&mut cx(), at(1)).unwrap();
    assert_eq!(first.probe.polls.load(Ordering::Relaxed), 1);
    assert_eq!(second.probe.polls.load(Ordering::Relaxed), 0);
    assert_eq!(third.probe.cancels.load(Ordering::Relaxed), 0);
    second.rt.1.step(&mut cx(), at(3)).unwrap();
    assert_eq!(second.probe.polls.load(Ordering::Relaxed), 1);
    assert_eq!(third.probe.cancels.load(Ordering::Relaxed), 0);
    let mut c = Rc::try_unwrap(c).ok().unwrap();
    assert!(
        matches!(c.poll(&mut cx(), at(4)), Poll::Ready(Ok(s)) if matches!(s.result(), Err(CoreError::Cancelled(_))))
    );
    assert_eq!(third.probe.polls.load(Ordering::Relaxed), 0);
    assert_eq!(third.probe.cancels.load(Ordering::Relaxed), 1);
    assert!(matches!(b.poll(&mut cx(), at(4)), Poll::Ready(Ok(s)) if s.result().is_ok()));
    assert!(matches!(a.poll(&mut cx(), at(4)), Poll::Ready(Ok(s)) if s.result().is_ok()));
    drop((a, b, c));
    dispose(first);
    dispose(second);
    dispose(third);
}

#[test]
fn genuine_caller_clock_regressions_and_domain_changes_still_fence_success() {
    for direct in [false, true] {
        for bad in [at(1), MonotonicInstant::new(ClockId::new(107), 3)] {
            let h = harness(1, None);
            let mut call = read(&h.rt).unwrap();
            h.rt.1.step(&mut cx(), at(2)).unwrap();
            if !direct {
                h.rt.1.step(&mut cx(), bad).unwrap();
            }
            let Poll::Ready(Ok(s)) = call.poll(&mut cx(), if direct { bad } else { at(3) }) else {
                panic!("not ready")
            };
            assert!(matches!(s.result(), Err(CoreError::InternalInvariant(_))));
            assert!(matches!(
                s.native_outcome(),
                ConsumerNativeOutcome::Returned(Ok(_))
            ));
            drop(s);
            drop(call);
            dispose(h);
        }
    }
}

#[test]
fn expiry_observed_on_a_sibling_wins_over_later_cancel_or_close() {
    for close in [false, true] {
        let h = harness(2, None);
        let first = read(&h.rt).unwrap();
        let mut second =
            h.rt.0
                .read_property(&h.rt.1, "level", Deadline::at(at(2)), at(1))
                .unwrap();
        h.rt.1.step(&mut cx(), at(2)).unwrap();
        assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
        if close {
            h.rt.1.begin_close();
        } else {
            second.cancel();
        }
        let Poll::Ready(Ok(result)) = second.poll(&mut cx(), at(3)) else {
            panic!("original local cleanup")
        };
        assert!(matches!(result.first_cause(), Some(CoreError::TimedOut(_))));
        assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
        drop((result, second, first));
        dispose(h);
    }
}

#[test]
fn nested_time_advancement_expires_checked_out_work_before_effects_or_handoff() {
    for edge in [Edge::Access, Edge::Poll, Edge::Validate] {
        let h = harness(1, None);
        let mut call =
            h.rt.0
                .read_property(&h.rt.1, "level", Deadline::at(at(2)), at(1))
                .unwrap();
        let rt = h.rt.clone();
        hook(edge, move || rt.1.step(&mut cx(), at(2)).unwrap());
        let Poll::Ready(Ok(s)) = call.poll(&mut cx(), at(1)) else {
            panic!("not ready")
        };
        assert!(
            matches!(s.result(), Err(CoreError::TimedOut(_))),
            "{:?}",
            s.result()
        );
        assert_eq!(
            h.probe.polls.load(Ordering::Relaxed),
            usize::from(edge != Edge::Access)
        );
        if edge != Edge::Access {
            assert!(matches!(
                s.native_outcome(),
                ConsumerNativeOutcome::Returned(Ok(_))
            ));
        }
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn native_construction_can_close_but_the_new_work_only_enters_cancellation() {
    let h = harness(1, None);
    let rt = h.rt.clone();
    hook(Edge::Start, move || rt.1.begin_close());
    let mut call = read(&h.rt).unwrap();
    let s = settle(&mut call);
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
    assert_eq!(h.probe.cancels.load(Ordering::Relaxed), 1);
    drop(s);
    drop(call);
    dispose(h);
}

#[test]
fn cancellation_callback_can_close_without_replacing_the_first_cause() {
    let h = harness(1, None);
    let mut call = read(&h.rt).unwrap();
    call.cancel();
    let rt = h.rt.clone();
    hook(Edge::Cancel, move || {
        rt.1.begin_close();
        rt.1.step(&mut cx(), now()).unwrap();
    });
    let s = settle(&mut call);
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
    drop(s);
    drop(call);
    dispose(h);
}

#[test]
fn checked_out_native_work_can_be_cancelled_and_its_late_result_stays_funded() {
    let h = harness(1, None);
    let handle = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
    let capture = handle.clone();
    hook(Edge::Poll, move || {
        capture.borrow().as_ref().unwrap().cancel()
    });
    h.rt.1.step(&mut cx(), now()).unwrap();
    let mut call = handle.borrow_mut().take().unwrap();
    let s = settle(&mut call);
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    assert!(
        matches!(s.native_outcome(), ConsumerNativeOutcome::Returned(Ok(o)) if o.data().unwrap().body == b"7")
    );
    assert!(h.bank.account(ResourceKind::CleanupBytesMax).used() > 0);
    drop(call);
    drop(handle);
    let Harness {
        rt, owner, bank, ..
    } = h;
    drop(rt);
    drop(owner);
    assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
    drop(s);
    clean(&bank);
}

#[test]
fn retained_old_handle_keeps_its_control_funded_and_cannot_cancel_a_reused_slot() {
    let h = harness(1, None);
    let baseline = usage(&h.bank);
    let mut old = read(&h.rt).unwrap();
    drop(settle(&mut old));
    assert!(h.rt.1.is_idle());
    assert_eq!(
        h.bank.account(ResourceKind::CleanupBytesMax).used(),
        baseline[ResourceKind::CleanupBytesMax.index()]
    );
    assert!(
        h.bank
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > baseline[ResourceKind::EngineLiveBytesGlobalMax.index()]
    );
    let mut current = read(&h.rt).unwrap();
    old.cancel();
    drop(old);
    let s = settle(&mut current);
    assert!(s.result().is_ok());
    drop(s);
    drop(current);
    assert_eq!(baseline, usage(&h.bank));
    dispose(h);
}

#[test]
fn reentrant_progress_can_cancel_a_sibling_without_polling_the_checked_out_call_again() {
    let h = harness(2, None);
    let mut first = read(&h.rt).unwrap();
    let second = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
    let rt = h.rt.clone();
    let capture = second.clone();
    hook(Edge::Poll, move || {
        assert!(matches!(read(&rt), Err(CoreError::Backpressure(_))));
        capture.borrow().as_ref().unwrap().cancel();
        rt.1.step(&mut cx(), now()).unwrap();
    });
    let s = settle(&mut first);
    assert!(s.result().is_ok());
    drop(s);
    let mut sibling = second.borrow_mut().take().unwrap();
    let s = settle(&mut sibling);
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    assert_eq!(h.probe.cancels.load(Ordering::Relaxed), 1);
    drop(s);
    drop(sibling);
    drop(first);
    drop(second);
    dispose(h);
}

#[test]
fn dependency_loss_during_native_disposal_keeps_raw_evidence_without_decoding() {
    let h = harness(1, None);
    let mut call = read(&h.rt).unwrap();
    let probe = h.probe.clone();
    hook(Edge::Dispose, move || {
        probe.access_revision.fetch_add(1, Ordering::AcqRel);
    });
    let result = settle(&mut call);
    assert!(result.result().is_err());
    assert!(
        matches!(result.native_outcome(), ConsumerNativeOutcome::Returned(Ok(o)) if o.data().unwrap().body == b"7")
    );
    assert_eq!(h.probe.validations.load(Ordering::Relaxed), 0);
    drop((result, call));
    dispose(h);
}

#[test]
fn validation_and_unread_handoff_recheck_close_after_callbacks() {
    for edge in [Edge::Validate, Edge::Access] {
        let h = harness(1, None);
        let mut call = read(&h.rt).unwrap();
        if edge == Edge::Access {
            h.rt.1.step(&mut cx(), now()).unwrap();
        }
        let rt = h.rt.clone();
        hook(edge, move || rt.1.begin_close());
        let s = settle(&mut call);
        assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
        assert!(
            matches!(s.native_outcome(), ConsumerNativeOutcome::Returned(Ok(o)) if o.data().unwrap().body == b"7")
        );
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn abandonment_and_native_destruction_can_reenter_after_the_original_slot_is_vacated() {
    let h = harness(1, None);
    let old = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
    let capture = old.clone();
    let rt = h.rt.clone();
    hook(Edge::Poll, move || {
        drop(capture.borrow_mut().take());
        hook(Edge::Dispose, move || {
            assert!(rt.1.is_idle());
            assert!(matches!(read(&rt), Err(CoreError::Backpressure(_))));
        });
    });
    h.rt.1.step(&mut cx(), now()).unwrap();
    assert!(old.borrow().is_none());
    let mut next = read(&h.rt).unwrap();
    assert!(settle(&mut next).result().is_ok());
    drop(next);
    drop(old);
    dispose(h);
}

#[test]
fn unwinding_restores_the_call_and_failed_admission_releases_its_reserved_slot() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let h = harness(1, None);
    hook(Edge::Access, || panic!("admission callback"));
    assert!(catch_unwind(AssertUnwindSafe(|| read(&h.rt))).is_err());
    assert!(h.rt.1.is_idle());
    let mut call = read(&h.rt).unwrap();
    hook(Edge::Poll, || panic!("native callback"));
    assert!(catch_unwind(AssertUnwindSafe(|| h.rt.1.step(&mut cx(), now()))).is_err());
    assert!(!h.rt.1.is_idle());
    call.cancel();
    assert!(matches!(
        settle(&mut call).result(),
        Err(CoreError::Cancelled(_))
    ));
    drop(call);
    dispose(h);
}

struct WakeProbe(AtomicUsize);
impl Wake for WakeProbe {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Wake);
    }
}
impl Drop for WakeProbe {
    fn drop(&mut self) {
        fire(Edge::WakerDrop);
    }
}

#[test]
fn inline_retry_registers_another_reads_close_waker_and_reports_exhausted_interest() {
    let outer = harness(1, None);
    outer.probe.pending.store(true, Ordering::Relaxed);
    let trigger = read(&outer.rt).unwrap();
    let replacements = harness(2, None);
    let first = Rc::new(read(&replacements.rt).unwrap());
    let second = Rc::new(read(&replacements.rt).unwrap());
    let (a, b) = (first.clone(), second.clone());
    hook(Edge::Poll, move || {
        for call in [a, b] {
            let old = Waker::from(Arc::new(WakeProbe(AtomicUsize::new(0))));
            assert!(
                call.progress(&mut Context::from_waker(&old), now())
                    .is_pending()
            );
        }
    });
    assert!(trigger.progress(&mut cx(), now()).is_pending());

    let retry_owner = harness(1, None);
    let retry_call = Rc::new(read(&retry_owner.rt).unwrap());
    let other = harness(1, None);
    let other_call = Rc::new(read(&other.rt).unwrap());
    let overflow = harness(1, None);
    let overflow_call = Rc::new(RefCell::new(read(&overflow.rt).unwrap()));
    let retry_count = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let retry_waker = Waker::from(retry_count.clone());
    let other_count = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let other_waker = Waker::from(other_count.clone());
    let overflow_handle = overflow_call.clone();
    let nested = other_call.clone();
    let retry = retry_call.clone();
    hook(Edge::Poll, move || {
        hook(Edge::WakerDrop, move || {
            hook(Edge::WakerDrop, move || {
                hook(Edge::Wake, move || {
                    assert!(
                        nested
                            .progress(&mut Context::from_waker(&other_waker), now())
                            .is_pending()
                    );
                });
                for _ in 0..2 {
                    assert!(
                        retry
                            .progress(&mut Context::from_waker(&retry_waker), now())
                            .is_pending()
                    );
                }
                // The one retry already belongs to another task. A caller can
                // retry this exact accepted Read without waiting for a signal.
                let Poll::Ready(Err(CoreError::Backpressure(error))) =
                    overflow_handle.borrow_mut().poll(&mut cx(), now())
                else {
                    panic!("unregistered observer needs an explicit retry");
                };
                assert_eq!(error.retry_class(), RetryClass::Safe);
            });
            assert!(second.progress(&mut cx(), now()).is_pending());
        });
        assert!(first.progress(&mut cx(), now()).is_pending());
    });
    assert!(trigger.progress(&mut cx(), now()).is_pending());
    HOOK.with(|h| assert!(h.borrow().is_none()));
    assert_eq!(retry_count.0.load(Ordering::Relaxed), 1);
    assert_eq!(other_count.0.load(Ordering::Relaxed), 0);
    for h in [&replacements, &retry_owner, &other, &overflow] {
        assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
    }
    other.rt.1.begin_close();
    assert_eq!(other_count.0.load(Ordering::Relaxed), 1);
    assert!(settle(&mut overflow_call.borrow_mut()).result().is_ok());
    assert_eq!(overflow.probe.polls.load(Ordering::Relaxed), 1);
    drop((trigger, retry_call, other_call, overflow_call));
    for h in [outer, replacements, retry_owner, other, overflow] {
        dispose(h);
    }
}

#[test]
fn dropping_a_bound_property_read_guards_its_last_selected_service() {
    let original = harness(1, None);
    let read_view = original.rt.0.bind_property_read("level").unwrap();
    drop((original.rt, original.owner));
    let sibling = harness(1, None);
    let mut next = read(&sibling.rt).unwrap();
    let rt = sibling.rt.clone();
    hook(Edge::ServiceDrop, move || {
        rt.1.step(&mut cx(), now()).unwrap()
    });
    drop(read_view);
    HOOK.with(|h| assert!(h.borrow().is_none()));
    assert_eq!(sibling.probe.polls.load(Ordering::Relaxed), 0);
    clean(&original.bank);
    assert!(settle(&mut next).result().is_ok());
    drop(next);
    dispose(sibling);
}

#[test]
fn collected_read_guards_its_last_service_destructor() {
    let original = harness(1, None);
    let mut call = read(&original.rt).unwrap();
    let result = settle(&mut call);
    assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
    drop((call, original.rt, original.owner));

    let sibling = harness(1, None);
    let mut next = read(&sibling.rt).unwrap();
    let rt = sibling.rt.clone();
    hook(Edge::ServiceDrop, move || {
        rt.1.step(&mut cx(), now()).unwrap()
    });
    drop(result);
    HOOK.with(|h| assert!(h.borrow().is_none()));
    assert_eq!(sibling.probe.polls.load(Ordering::Relaxed), 0);
    clean(&original.bank);
    assert_eq!(
        settle(&mut next).result().unwrap().data().unwrap().body,
        b"7"
    );
    drop(next);
    dispose(sibling);
}

#[test]
fn checked_out_read_wakes_only_its_latest_observer_on_restoration() {
    let h = harness(1, None);
    h.probe.pending.store(true, Ordering::Relaxed);
    let call = Rc::new(read(&h.rt).unwrap());
    let nested = call.clone();
    let rt = h.rt.clone();
    let first = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let latest = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let first_waker = Waker::from(first.clone());
    let latest_waker = Waker::from(latest.clone());
    hook(Edge::Poll, move || {
        rt.1.step(&mut Context::from_waker(&first_waker), now())
            .unwrap();
        let mut cx = Context::from_waker(&latest_waker);
        assert!(nested.progress(&mut cx, now()).is_pending());
        assert!(nested.progress(&mut cx, now()).is_pending());
    });
    assert!(call.progress(&mut cx(), now()).is_pending());
    assert_eq!(first.0.load(Ordering::Relaxed), 0);
    assert_eq!(latest.0.load(Ordering::Relaxed), 1);
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    drop(call);
    dispose(h);
}

#[test]
fn deferred_ready_read_registers_only_its_live_handle_for_close_or_cancel() {
    for close in [true, false] {
        let outer = harness(1, None);
        let trigger = read(&outer.rt).unwrap();
        let sibling = harness(1, None);
        let mut retired = read(&sibling.rt).unwrap();
        drop(settle(&mut retired));
        let call = Rc::new(read(&sibling.rt).unwrap());
        sibling.probe.pending.store(true, Ordering::Relaxed);
        assert!(call.progress(&mut cx(), now()).is_pending());
        let polls = sibling.probe.polls.load(Ordering::Relaxed);
        let latest = Arc::new(WakeProbe(AtomicUsize::new(0)));
        let waker = Waker::from(latest.clone());
        let nested = call.clone();
        hook(Edge::Poll, move || {
            assert!(
                nested
                    .progress(&mut Context::from_waker(&waker), now())
                    .is_pending()
            );
            assert!(retired.progress(&mut cx(), now()).is_pending());
        });
        assert!(trigger.progress(&mut cx(), now()).is_ready());
        assert_eq!(sibling.probe.polls.load(Ordering::Relaxed), polls);
        assert_eq!(latest.0.load(Ordering::Relaxed), 0);
        if close {
            sibling.rt.1.begin_close();
        } else {
            call.cancel();
        }
        assert_eq!(latest.0.load(Ordering::Relaxed), 1);
        drop((trigger, call));
        dispose(outer);
        dispose(sibling);
    }
}

#[test]
fn nested_poll_registers_interest_and_wakes_after_the_call_is_restored() {
    let h = harness(1, None);
    let handle = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
    let capture = handle.clone();
    let counter = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let count = counter.clone();
    hook(Edge::Poll, move || {
        let waker = Waker::from(count);
        assert!(
            capture
                .borrow_mut()
                .as_mut()
                .unwrap()
                .poll(&mut Context::from_waker(&waker), now())
                .is_pending()
        );
    });
    h.rt.1.step(&mut cx(), now()).unwrap();
    assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    let mut call = handle.borrow_mut().take().unwrap();
    assert!(settle(&mut call).result().is_ok());
    drop(call);
    drop(handle);
    drop(counter);
    dispose(h);
}

#[test]
fn nested_runtime_driver_keeps_interest_while_the_original_native_poll_is_pending() {
    let h = harness(1, None);
    let mut call = read(&h.rt).unwrap();
    let counter = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let capture = counter.clone();
    let rt = h.rt.clone();
    hook(Edge::Access, move || {
        let waker = Waker::from(capture);
        rt.1.step(&mut Context::from_waker(&waker), now()).unwrap();
    });
    h.probe.pending.store(true, Ordering::Relaxed);
    h.rt.1.step(&mut cx(), now()).unwrap();
    call.cancel();
    assert!(
        counter.0.load(Ordering::Relaxed) > 0,
        "nested driver lost its wake registration"
    );
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    drop(settle(&mut call));
    drop(call);
    drop(counter);
    dispose(h);
}

#[test]
fn consumed_inline_cancel_wake_is_rearmed_after_pending_native_work_is_restored() {
    let h = harness(1, None);
    h.probe.pending.store(true, Ordering::Relaxed);
    let handle = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
    let capture = handle.clone();
    let counter = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let rt = h.rt.clone();
    let nested_waker = waker.clone();
    hook(Edge::Poll, move || {
        hook(Edge::Wake, move || {
            rt.1.step(&mut Context::from_waker(&nested_waker), now())
                .unwrap()
        });
        capture.borrow().as_ref().unwrap().cancel();
    });
    h.rt.1
        .step(&mut Context::from_waker(&waker), now())
        .unwrap();
    assert!(
        counter.0.load(Ordering::Relaxed) >= 2,
        "busy-slot restoration lost the inline cancellation wake"
    );
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    // The restoration wake supplies the next driver turn; no native I/O or
    // unrelated event is needed to enter cancellation and retire the owner.
    h.rt.1
        .step(&mut Context::from_waker(&waker), now())
        .unwrap();
    assert_eq!(h.probe.cancels.load(Ordering::Relaxed), 1);
    let mut call = handle.borrow_mut().take().unwrap();
    let s = settle(&mut call);
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    drop(s);
    drop(call);
    drop(handle);
    drop(waker);
    drop(counter);
    dispose(h);
}

#[test]
fn replacing_a_waker_may_close_without_borrow_conflicts_or_successful_handoff() {
    let h = harness(1, None);
    let mut call = read(&h.rt).unwrap();
    let waker = Waker::from(Arc::new(WakeProbe(AtomicUsize::new(0))));
    h.probe.pending.store(true, Ordering::Relaxed);
    assert!(
        call.poll(&mut Context::from_waker(&waker), now())
            .is_pending()
    );
    drop(waker);
    let rt = h.rt.clone();
    hook(Edge::WakerDrop, move || rt.1.begin_close());
    let s = settle(&mut call);
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 2);
    assert_eq!(h.probe.cancels.load(Ordering::Relaxed), 1);
    drop(s);
    drop(call);
    dispose(h);
}

#[test]
fn manual_progress_registers_control_wakes_and_close_records_sibling_causes_before_waking() {
    let h = harness(2, None);
    let mut first = read(&h.rt).unwrap();
    let second = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
    let counter = Arc::new(WakeProbe(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    h.rt.1
        .step(&mut Context::from_waker(&waker), now())
        .unwrap();
    let sibling = second.clone();
    let rt = h.rt.clone();
    hook(Edge::Wake, move || {
        assert!(!rt.1.is_idle());
        sibling.borrow().as_ref().unwrap().cancel();
    });
    h.rt.1.begin_close();
    assert!(counter.0.load(Ordering::Relaxed) >= 1);
    let a = settle(&mut first);
    let mut next = second.borrow_mut().take().unwrap();
    let b = settle(&mut next);
    assert!(matches!(a.result(), Err(CoreError::StaleHandle(_))));
    assert!(matches!(b.result(), Err(CoreError::StaleHandle(_))));
    drop(a);
    drop(b);
    drop(first);
    drop(next);
    drop(second);
    drop(waker);
    drop(counter);
    dispose(h);
}

#[test]
fn real_http_retains_one_original_result_when_validation_reenters_close_or_cancel() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    for close in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let h = harness(1, Some(listener.local_addr().unwrap()));
        let peer = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 1);
                request.push(byte[0]);
                assert!(request.len() < 4096);
            }
            assert!(request.starts_with(b"GET /level HTTP/1.1\r\n"));
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7").unwrap();
            let mut byte = [0];
            assert_eq!(
                stream.read(&mut byte).unwrap(),
                0,
                "original connection retires"
            );
            listener.set_nonblocking(true).unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
                "no implicit replay"
            );
        });
        let io = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        let settlement = io.block_on(async {
            let handle = Rc::new(RefCell::new(Some(read(&h.rt).unwrap())));
            let capture = handle.clone();
            let rt = h.rt.clone();
            hook(Edge::Validate, move || {
                if close {
                    rt.1.begin_close();
                } else {
                    capture.borrow().as_ref().unwrap().cancel();
                }
                rt.1.step(&mut cx(), now()).unwrap();
            });
            let start = Instant::now();
            loop {
                h.rt.1.step(&mut cx(), now()).unwrap();
                // The handle remains outside progress, allowing a callback to
                // cancel that same checked-out call through the public API.
                if HOOK.with(|h| h.borrow().is_none()) {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let mut call = handle.borrow_mut().take().unwrap();
            let s = settle(&mut call);
            drop(call);
            drop(handle);
            s
        });
        if close {
            assert!(matches!(
                settlement.result(),
                Err(CoreError::StaleHandle(_))
            ));
        } else {
            assert!(matches!(settlement.result(), Err(CoreError::Cancelled(_))));
        }
        assert!(
            matches!(settlement.native_outcome(), ConsumerNativeOutcome::Returned(Ok(o))
            if o.data().unwrap().body == b"7" && o.metadata().binding_response().unwrap().status_code().unwrap() == 200)
        );
        assert_eq!(settlement.consumer(), &h.owner.reference());
        let Harness {
            rt, owner, bank, ..
        } = h;
        drop(rt);
        drop(owner);
        assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
        drop(settlement);
        clean(&bank);
        peer.join().unwrap();
    }
}

#[test]
fn saturated_call_capacity_recovers_original_input_before_capability_or_native_start() {
    let h = harness(1, None);
    let before = usage(&h.bank);
    let input = Payload::new(b"8".as_slice(), "application/json");
    let original = input.body.clone();
    let issued = Rc::new(std::cell::Cell::new(false));
    let issued_in_hook = issued.clone();
    hook(Edge::Issue, move || issued_in_hook.set(true));
    let quota = h.bank.account(ResourceKind::PendingHandlerCallsGlobalMax);
    let pressure = h
        .bank
        .reserve(
            ResourceKind::PendingHandlerCallsGlobalMax,
            quota.limit() - quota.used(),
        )
        .unwrap();
    let refused =
        h.rt.0
            .write_property(&h.rt.1, "level", input, Deadline::NONE, now())
            .err()
            .expect("saturated admission must return the input");
    assert!(matches!(
        refused.error().error(),
        CoreError::LimitExceeded { .. }
    ));
    let (input, _) = refused.into_parts();
    assert!(Arc::ptr_eq(&original, &input.body));
    assert!(!issued.get());
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
    assert!(h.rt.1.is_idle());
    drop(pressure);
    assert_eq!(before, usage(&h.bank));
    let mut retry =
        h.rt.0
            .write_property(&h.rt.1, "level", input, Deadline::NONE, now())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(issued.get());
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
    drop(settle(&mut retry));
    drop(retry);
    drop(original);
    dispose(h);
}

#[test]
fn managed_credential_callbacks_remove_or_shutdown_before_native_acceptance() {
    for http in [false, true] {
        for change in 0..3 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
            let managed = Rc::new(ConsumerOwner::try_new(bank.clone(), 1, 1).unwrap());
            let instance = managed
                .register_thing("urn:callbacks", &mut work())
                .unwrap();
            let (rt, probe) = association_harness(
                1,
                http.then(|| listener.local_addr().unwrap()),
                bank.clone(),
                instance.consumer(),
            );
            let (standalone, calls) = Rc::try_unwrap(rt).ok().unwrap();
            let association = managed
                .set_configuration(instance.thing_id(), "default", standalone, &mut work())
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
            let control = managed.clone();
            let original_instance = instance.clone();
            let original_association = association.clone();
            hook(Edge::Issue, move || match change {
                0 => control
                    .remove_configuration_if_current(
                        original_instance.thing_id(),
                        "default",
                        &original_association,
                        &mut work(),
                    )
                    .unwrap(),
                1 => control
                    .remove_instance(&original_instance, &mut work())
                    .unwrap(),
                _ => control.begin_close(),
            });
            let input = Payload::new(b"8".as_slice(), "application/json");
            let allocation = input.body.clone();
            let refused = association
                .write_property(&calls, "level", input, Deadline::NONE, now())
                .err()
                .expect("removed admission must refuse");
            assert!(matches!(refused.error().error(), CoreError::StaleHandle(_)));
            let (input, _) = refused.into_parts();
            assert!(Arc::ptr_eq(&allocation, &input.body));
            assert!(calls.is_idle());
            assert_eq!(probe.starts.load(Ordering::Relaxed), 0);
            assert_eq!(probe.polls.load(Ordering::Relaxed), 0);
            assert_eq!(probe.cancels.load(Ordering::Relaxed), 0);
            assert_eq!(probe.drops.load(Ordering::Relaxed), 0);
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
            assert_eq!(
                bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
                    .used(),
                0
            );
            assert_eq!(instance.is_current(), change == 0);
            drop((input, allocation, association, instance, managed, calls));
            clean(&bank);
            HOOK.with(|h| assert!(h.borrow().is_none()));
        }
    }
}

#[test]
fn managed_shutdown_fences_all_parents_before_reentrant_service_destruction() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let other_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let managed = Rc::new(ConsumerOwner::try_new(bank.clone(), 2, 2).unwrap());
    let ia = managed.register_thing("urn:a", &mut work()).unwrap();
    let ib = managed.register_thing("urn:b", &mut work()).unwrap();
    let (a, _) = association_harness(1, None, bank.clone(), ia.consumer());
    let (b, _) = association_harness(1, None, other_bank.clone(), ib.consumer());
    let (sa, ca) = Rc::try_unwrap(a).ok().unwrap();
    let (sb, cb) = Rc::try_unwrap(b).ok().unwrap();
    let va = managed
        .set_configuration(ia.thing_id(), "default", sa, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let vb = managed
        .set_configuration(ib.thing_id(), "default", sb, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let control = managed.clone();
    let first = va.clone();
    let sibling_scope = vb.clone();
    hook(Edge::ServiceDrop, move || {
        assert!(control.is_closed());
        assert!(!first.is_current());
        assert!(!sibling_scope.is_current());
        let rejected = control
            .register_thing("urn:replacement", &mut work())
            .err()
            .expect("closed owner");
        assert!(matches!(rejected, CoreError::StaleHandle(_)));
    });
    managed.begin_close();
    assert!(!ib.is_current());
    drop((va, vb, ia, ib, managed, ca, cb));
    clean(&bank);
    clean(&other_bank);
    HOOK.with(|h| assert!(h.borrow().is_none()));
}

#[test]
fn id_resolved_admission_never_retries_a_replacement_created_during_credentials() {
    for http in [false, true] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let owner = Rc::new(ConsumerOwner::try_new(bank.clone(), 1, 1).unwrap());
        let id = "urn:pump";
        let original = owner.register_thing(id, &mut work()).unwrap().consumer();
        let (rt, probe) = association_harness(
            1,
            http.then(|| listener.local_addr().unwrap()),
            bank.clone(),
            original.clone(),
        );
        let (standalone, calls) = Rc::try_unwrap(rt).ok().unwrap();
        drop(
            owner
                .set_configuration(id, "default", standalone, &mut work())
                .unwrap_or_else(|e| panic!("{:?}", e.error())),
        );
        let control = owner.clone();
        hook(Edge::Issue, move || {
            control.remove_thing(id, &mut work()).unwrap();
            drop(control.register_thing(id, &mut work()).unwrap());
        });
        let input = Payload::new(b"8".as_slice(), "application/json");
        let allocation = input.body.clone();
        let refused = owner
            .association(id, "default")
            .unwrap()
            .write_property(&calls, "level", input, Deadline::NONE, now())
            .err()
            .unwrap();
        assert!(matches!(refused.error().error(), CoreError::StaleHandle(_)));
        let (input, _) = refused.into_parts();
        assert!(Arc::ptr_eq(&allocation, &input.body));
        assert!(!original.is_current());
        let current = owner.instance(id).unwrap();
        assert!(current.is_current());
        assert_ne!(current.consumer(), original);
        assert_eq!(probe.starts.load(Ordering::Relaxed), 0);
        assert!(calls.is_idle());
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        drop((input, allocation, current, original, owner, calls));
        clean(&bank);
        HOOK.with(|h| assert!(h.borrow().is_none()));
    }
}

#[test]
fn indexed_replacement_is_committed_before_destructor_reentry_reorders_both_tables() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = Rc::new(ConsumerOwner::try_new(bank.clone(), 4, 2).unwrap());
    for id in ["urn:z", "urn:a", "urn:m"] {
        drop(owner.register_thing(id, &mut work()).unwrap());
    }
    let original = owner.instance("urn:m").unwrap().consumer();
    let mut calls = Vec::new();
    let mut views = Vec::new();
    for name in ["omega", "alpha"] {
        let (rt, _) = association_harness(1, None, bank.clone(), original.clone());
        let (raw, store) = Rc::try_unwrap(rt).ok().unwrap();
        views.push(
            owner
                .set_configuration("urn:m", name, raw, &mut work())
                .unwrap_or_else(|e| panic!("{:?}", e.error())),
        );
        calls.push(store);
    }
    let (rt, _) = association_harness(1, None, bank.clone(), original.clone());
    let (replacement, store) = Rc::try_unwrap(rt).ok().unwrap();
    calls.push(store);
    let control = owner.clone();
    let old = views[1].clone();
    hook(Edge::ServiceDrop, move || {
        assert!(!old.is_current());
        assert!(control.association("urn:m", "alpha").unwrap().is_current());
        control.remove_thing("urn:m", &mut work()).unwrap();
        drop(control.register_thing("urn:m", &mut work()).unwrap());
        drop(control.register_thing("urn:b", &mut work()).unwrap());
    });
    let replacement = owner
        .set_configuration("urn:m", "alpha", replacement, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(!original.is_current() && !replacement.is_current());
    assert!(views.iter().all(|v| !v.is_current()));
    assert_ne!(owner.instance("urn:m").unwrap().consumer(), original);
    for id in ["urn:a", "urn:b", "urn:m", "urn:z"] {
        assert!(owner.instance(id).unwrap().is_current());
    }
    assert!(matches!(
        owner.association("urn:m", "alpha"),
        Err(CoreError::NotFound(_))
    ));
    drop((replacement, views, original, calls, owner));
    clean(&bank);
    HOOK.with(|h| assert!(h.borrow().is_none()));
}

#[path = "../../../../servient/finite/tests/support/worker_exit.rs"]
mod worker_exit;

#[test]
fn worker_exit_retires_a_parked_read() {
    worker_exit::check(
        || {
            let h = harness(1, None);
            h.probe.pending.store(true, Ordering::Relaxed);
            let call = read(&h.rt).unwrap();
            (h, call)
        },
        |(_, call)| assert!(call.progress(&mut cx(), now()).is_pending()),
        |(h, mut call)| {
            call.cancel();
            assert!(matches!(
                settle(&mut call).result(),
                Err(CoreError::Cancelled(_))
            ));
            assert_eq!(h.probe.cancels.load(Ordering::Relaxed), 1);
            assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
            drop(call);
            dispose(h);
        },
    );
}
