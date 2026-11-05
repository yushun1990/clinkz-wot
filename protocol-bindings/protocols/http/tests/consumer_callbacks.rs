#![cfg(feature = "prepare")]
//! Callback isolation through public TD preparation and finite Consumer APIs.
//! Local native work makes adversarial scheduling deterministic; real HTTP
//! evidence uses the unmodified executor in the final tests below.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{consumer::*, host_accounts::*, value_type::TypeSchema, *};
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
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Access,
    Validate,
    Start,
    Poll,
    Cancel,
    Dispose,
    Wake,
    WakerDrop,
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
    MonotonicInstant::new(ClockId::new(106), 1)
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
struct Data(Arc<dyn ConsumerDataService>);
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        self.0.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        self.0.policy()
    }
    fn supports_schema(&self, s: TypeSchema) -> bool {
        self.0.supports_schema(s)
    }
    fn validate(&self, s: TypeSchema, p: &Payload) -> CoreResult<()> {
        fire(Edge::Validate);
        self.0.validate(s, p)
    }
}
struct Access(Arc<dyn ConsumerAccessService>);
impl ConsumerAccessService for Access {
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
        fire(Edge::Access);
        self.0.authorize_instance(c, b, t, o)
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
    starts: AtomicUsize,
    polls: AtomicUsize,
    cancels: AtomicUsize,
    drops: AtomicUsize,
}
struct LocalClient {
    configuration: ConsumerBindingConfiguration,
    compatibility: BindingArtifactCompatibility,
    probe: Arc<Probe>,
}
impl binding::ClientBinding for LocalClient {
    fn supports_operation(&self, o: Operation) -> bool {
        matches!(o, Operation::ReadProperty | Operation::WriteProperty)
    }
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        Some(self.configuration)
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        _: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        self.probe.starts.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Start);
        HostBindingCallBox::try_new(LocalCall {
            request: Some(request),
            probe: self.probe.clone(),
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
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.probe.polls.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Poll);
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
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
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
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        let cleanup = CleanupPhaseContext::bind(
            CleanupReservation::new(
                CleanupSlotId::new(SlotIndex::new(0), Generation::INITIAL),
                self.lifetime_footprint(),
                0,
                WorkBudget::new(),
            ),
            CleanupOperation::CancelRequest,
            busy(),
            Deadline::NONE,
        );
        match self.start_cancel(cx, cleanup, budget)? {
            StartStatus::Pending => Poll::Pending,
            StartStatus::Ready(s) => Poll::Ready(Ok(s)),
        }
    }
}
struct Harness {
    rt: Rc<ConsumerRuntime>,
    owner: ConsumerInstance,
    bank: Arc<HostResourceAccounts>,
    probe: Arc<Probe>,
}
fn harness(calls: usize, address: Option<std::net::SocketAddr>) -> Harness {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
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
        data: Arc::new(Data(services.data)),
        access: Arc::new(Access(services.access)),
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
        ConsumerRegistry::try_new(vec![input]).unwrap_or_else(|e| panic!("{:?}", e.error()));
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
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
    let (definitions, funding) = prepared.into_execution();
    let rt = Rc::new(
        ConsumerRuntime::try_new(owner.reference(), definitions, funding, bank.clone(), calls)
            .unwrap_or_else(|e| panic!("{:?}", e.error())),
    );
    Harness {
        rt,
        owner,
        bank,
        probe,
    }
}
fn read(rt: &ConsumerRuntime) -> CoreResult<ConsumerCall> {
    rt.read_property("level", Deadline::NONE, now(), &mut work())
}
fn settle(call: &mut ConsumerCall) -> ConsumerSettlement {
    for _ in 0..32 {
        if let Poll::Ready(s) = call.poll(&mut cx(), now(), &mut work()) {
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
fn authorization_can_close_and_refuse_the_original_write_before_native_construction() {
    let h = harness(1, None);
    let before = usage(&h.bank);
    let rt = h.rt.clone();
    hook(Edge::Access, move || {
        assert!(!rt.is_idle());
        rt.begin_close();
    });
    let input = Payload::new(b"8".as_slice(), "application/json");
    let original = input.body.clone();
    let rejected =
        h.rt.write_property("level", input, Deadline::NONE, now(), &mut work())
            .err()
            .unwrap();
    assert!(matches!(
        rejected.error().error(),
        CoreError::StaleHandle(_)
    ));
    let (input, _) = rejected.into_parts();
    assert!(Arc::ptr_eq(&original, &input.body));
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
    assert!(h.rt.is_idle());
    assert_eq!(before, usage(&h.bank));
    dispose(h);
}

#[test]
fn nested_admission_sees_reserved_capacity_and_cannot_overwrite_the_outer_call() {
    for capacity in [1, 2] {
        let h = harness(capacity, None);
        let rt = h.rt.clone();
        let nested = Rc::new(RefCell::new(None));
        let capture = nested.clone();
        hook(Edge::Access, move || {
            assert!(!rt.is_idle());
            rt.step(&mut cx(), now(), &mut work());
            match read(&rt) {
                Ok(call) => {
                    assert_eq!(capacity, 2);
                    *capture.borrow_mut() = Some(call);
                }
                Err(error) => {
                    assert_eq!(capacity, 1);
                    assert!(matches!(error, CoreError::Backpressure(_)));
                }
            }
        });
        let mut outer = read(&h.rt).unwrap();
        let result = settle(&mut outer);
        assert!(result.result().is_ok());
        drop(result);
        drop(outer);
        if let Some(mut call) = nested.borrow_mut().take() {
            drop(settle(&mut call));
            drop(call);
        }
        assert!(h.rt.is_idle());
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), capacity);
        dispose(h);
    }
}

#[test]
fn native_construction_can_close_but_the_new_work_only_enters_cancellation() {
    let h = harness(1, None);
    let rt = h.rt.clone();
    hook(Edge::Start, move || rt.begin_close());
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
        rt.begin_close();
        rt.step(&mut cx(), now(), &mut work());
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
    h.rt.step(&mut cx(), now(), &mut work());
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
    assert!(h.rt.is_idle());
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
        rt.step(&mut cx(), now(), &mut work());
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
fn validation_and_unread_handoff_recheck_close_after_callbacks() {
    for edge in [Edge::Validate, Edge::Access] {
        let h = harness(1, None);
        let mut call = read(&h.rt).unwrap();
        if edge == Edge::Access {
            h.rt.step(&mut cx(), now(), &mut work());
        }
        let rt = h.rt.clone();
        hook(edge, move || rt.begin_close());
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
    let replacement = Rc::new(RefCell::new(None));
    let next = replacement.clone();
    let rt = h.rt.clone();
    hook(Edge::Poll, move || {
        drop(capture.borrow_mut().take());
        hook(Edge::Dispose, move || {
            assert!(rt.is_idle());
            *next.borrow_mut() = Some(read(&rt).unwrap());
        });
    });
    h.rt.step(&mut cx(), now(), &mut work());
    assert!(old.borrow().is_none());
    let mut next = replacement.borrow_mut().take().unwrap();
    assert!(settle(&mut next).result().is_ok());
    drop(next);
    drop(old);
    drop(replacement);
    dispose(h);
}

#[test]
fn unwinding_restores_the_call_and_failed_admission_releases_its_reserved_slot() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let h = harness(1, None);
    hook(Edge::Access, || panic!("admission callback"));
    assert!(catch_unwind(AssertUnwindSafe(|| read(&h.rt))).is_err());
    assert!(h.rt.is_idle());
    let mut call = read(&h.rt).unwrap();
    hook(Edge::Poll, || panic!("native callback"));
    assert!(
        catch_unwind(AssertUnwindSafe(|| h.rt.step(
            &mut cx(),
            now(),
            &mut work()
        )))
        .is_err()
    );
    assert!(!h.rt.is_idle());
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
                .poll(&mut Context::from_waker(&waker), now(), &mut work())
                .is_pending()
        );
    });
    h.rt.step(&mut cx(), now(), &mut work());
    assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    let mut call = handle.borrow_mut().take().unwrap();
    assert!(settle(&mut call).result().is_ok());
    drop(call);
    drop(handle);
    drop(counter);
    dispose(h);
}

#[test]
fn replacing_a_waker_may_close_without_borrow_conflicts_or_successful_handoff() {
    let h = harness(1, None);
    let mut call = read(&h.rt).unwrap();
    let waker = Waker::from(Arc::new(WakeProbe(AtomicUsize::new(0))));
    assert!(
        call.poll(
            &mut Context::from_waker(&waker),
            now(),
            &mut WorkBudget::new()
        )
        .is_pending()
    );
    drop(waker);
    let rt = h.rt.clone();
    hook(Edge::WakerDrop, move || rt.begin_close());
    let s = settle(&mut call);
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
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
    h.rt.step(&mut Context::from_waker(&waker), now(), &mut work());
    let sibling = second.clone();
    let rt = h.rt.clone();
    hook(Edge::Wake, move || {
        assert!(!rt.is_idle());
        sibling.borrow().as_ref().unwrap().cancel();
    });
    h.rt.begin_close();
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
                    rt.begin_close();
                } else {
                    capture.borrow().as_ref().unwrap().cancel();
                }
                rt.step(&mut cx(), now(), &mut work());
            });
            let start = Instant::now();
            loop {
                h.rt.step(&mut cx(), now(), &mut work());
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
            if o.data().unwrap().body == b"7" && o.metadata().binding_response().unwrap().status_code() == 200)
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
