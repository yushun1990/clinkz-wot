//! Public continuing Action ownership under deterministic linked-code reentry.
//! Actual HTTP and downstream datagram lifecycle tests supply transport evidence.
use clinkz_wot_core::{
    consumer::*, consumer_action::*, data_schema::PreparedSchema, host_accounts::*, *,
};
use clinkz_wot_foundation::*;
use clinkz_wot_servient_finite::consumer_action::*;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Access,
    Validate,
    Destination,
    Start,
    Poll,
    Close,
    Dispose,
    TokenDrop,
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
        let mut h = h.borrow_mut();
        if h.as_ref().is_some_and(|(e, _)| *e == edge) {
            h.take()
        } else {
            None
        }
    });
    if let Some((_, callback)) = callback {
        callback();
    }
}
fn fired() {
    HOOK.with(|h| assert!(h.borrow().is_none()));
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
fn at(t: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(109), t)
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
fn usage(b: &HostResourceAccounts) -> Vec<u64> {
    ResourceKind::ALL
        .into_iter()
        .map(|k| b.account(k).used())
        .collect()
}
fn clean(b: &HostResourceAccounts) {
    assert!(usage(b).iter().all(|n| *n == 0), "{:?}", usage(b));
}

#[derive(Default)]
struct Probe {
    starts: AtomicUsize,
    polls: AtomicUsize,
    closes: AtomicUsize,
    drops: AtomicUsize,
    validates: AtomicUsize,
    pending: AtomicBool,
    close_pending: AtomicBool,
    fail_poll: AtomicBool,
    fail_close: AtomicBool,
    complete_invoke: AtomicBool,
    late: AtomicBool,
    invalid_output: AtomicBool,
    unavailable: AtomicBool,
    wake_in_poll: AtomicBool,
}
struct Data(Arc<Probe>);
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 64,
            temporary_bytes: 64,
            body_bytes: 32,
            encoded_selection_bytes: 0,
            validation_work: 4,
        }
    }
    fn available(&self) -> bool {
        !self.0.unavailable.load(Ordering::Relaxed)
    }
    fn validate(&self, _: &PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.0.validates.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Validate);
        if p.body.as_ref() == b"12" || p.body.as_ref() == b"24" {
            Ok(())
        } else {
            Err(invalid())
        }
    }
}
struct Access;
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        fire(Edge::Access);
        Ok(())
    }
    fn work(&self) -> u64 {
        2
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
}
struct Token(u64);
impl Drop for Token {
    fn drop(&mut self) {
        fire(Edge::TokenDrop);
    }
}
struct Mapping {
    operation: Operation,
    probe: Arc<Probe>,
    bank: Arc<HostResourceAccounts>,
}
impl ConsumerActionMapping for Mapping {
    fn identity(&self) -> BindingRegistrationIdentity {
        BindingRegistrationIdentity::new(
            BindingId::new(1),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([1; 32]),
            BindingArtifactCompatibility::new(*b"action-callbacks"),
            0,
        )
    }
    fn operation(&self) -> Operation {
        self.operation
    }
    fn bounds(&self) -> ConsumerActionBounds {
        ConsumerActionBounds {
            exchange_bytes: 512,
            temporary_bytes: 64,
            continuation_bytes: 64,
            response_bytes: 512,
            poll_work: 4,
        }
    }
    fn native_definition(&self) -> &dyn Any {
        self
    }
    fn supports_continuation(&self, _: &dyn ConsumerActionMapping) -> CoreResult<()> {
        Ok(())
    }
    fn destination(&self, _: Option<&dyn Any>) -> CoreResult<Box<str>> {
        fire(Edge::Destination);
        Ok("test://action".into())
    }
    fn start(
        &self,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        if self.operation == Operation::InvokeAction {
            assert!(input.is_some());
            assert!(continuation.is_none());
        } else {
            assert!(input.is_none());
            assert_eq!(continuation.unwrap().downcast_ref::<Token>().unwrap().0, 7);
        }
        self.probe.starts.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Start);
        Ok(Box::new(Native {
            operation: self.operation,
            probe: self.probe.clone(),
            bank: self.bank.clone(),
        }))
    }
}
struct Native {
    operation: Operation,
    probe: Arc<Probe>,
    bank: Arc<HostResourceAccounts>,
}
impl Native {
    fn output(&self) -> ConsumerActionReply {
        ConsumerActionReply::Complete(InteractionOutput::with_data(Payload::new(
            if self.probe.invalid_output.load(Ordering::Relaxed) {
                b"bad".as_slice()
            } else {
                b"24".as_slice()
            },
            "application/json",
        )))
    }
}
impl ConsumerActionExchange for Native {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>> {
        self.probe.polls.fetch_add(1, Ordering::Relaxed);
        if self.probe.wake_in_poll.swap(false, Ordering::Relaxed) {
            cx.waker().wake_by_ref();
        }
        fire(Edge::Poll);
        if self.probe.pending.load(Ordering::Relaxed) {
            return Poll::Pending;
        }
        if self.probe.fail_poll.swap(false, Ordering::Relaxed) {
            return Poll::Ready(Err(invalid()));
        }
        Poll::Ready(Ok(match self.operation {
            Operation::InvokeAction if !self.probe.complete_invoke.load(Ordering::Relaxed) => {
                ConsumerActionReply::Accepted(Box::new(Token(7)))
            }
            Operation::CancelAction => ConsumerActionReply::CancelRequested,
            _ => self.output(),
        }))
    }
    fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<Option<ConsumerActionReply>>> {
        self.probe.closes.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Close);
        if self.probe.fail_close.swap(false, Ordering::Relaxed) {
            return Poll::Ready(Err(invalid()));
        }
        if self.probe.close_pending.load(Ordering::Relaxed) {
            return Poll::Pending;
        }
        Poll::Ready(Ok(self
            .probe
            .late
            .swap(false, Ordering::Relaxed)
            .then(|| self.output())))
    }
    fn abort_local(self: Box<Self>) {
        drop(self);
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        assert!(self.bank.account(ResourceKind::CleanupItemsMax).used() > 0);
        self.probe.drops.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Dispose);
    }
}
struct Harness {
    rt: Rc<ConsumerActionRuntime>,
    owner: ConsumerInstance,
    probe: Arc<Probe>,
    bank: Arc<HostResourceAccounts>,
}
fn harness(slots: usize) -> Harness {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(Probe::default());
    let op = |operation| ConsumerActionOperation {
        mapping: Box::new(Mapping {
            operation,
            probe: probe.clone(),
            bank: bank.clone(),
        }),
        endpoint: "test://action".into(),
        form: 0,
        services: ConsumerServices {
            data: Arc::new(Data(probe.clone())),
            access: Arc::new(Access),
        },
    };
    let d = ConsumerActionDefinition::try_new(ConsumerActionDefinitionInput {
        target: AffordanceTarget::Action("run".into()),
        schemas: ConsumerSchemas {
            input: Some(PreparedSchema::Any),
            output: Some(PreparedSchema::Any),
        },
        invoke: op(Operation::InvokeAction),
        query: Some(op(Operation::QueryAction)),
        cancel: Some(op(Operation::CancelAction)),
        bank: bank.clone(),
        funding: vec![],
    })
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let rt = Rc::new(
        ConsumerActionRuntime::try_new(owner.reference(), vec![d], slots)
            .unwrap_or_else(|e| panic!("{:?}", e.error())),
    );
    Harness {
        rt,
        owner,
        probe,
        bank,
    }
}
fn invoke(
    rt: &ConsumerActionRuntime,
) -> Result<ConsumerActionHandle, BindingInputRejection<Option<Payload>>> {
    rt.invoke_action(
        "run",
        Some(Payload::new(b"12".as_slice(), "application/json")),
        Deadline::NONE,
        at(1),
        &mut work(),
    )
}
fn start(h: &Harness) -> Rc<ConsumerActionHandle> {
    Rc::new(invoke(&h.rt).unwrap_or_else(|e| panic!("{:?}", e.error())))
}
fn progress(h: &ConsumerActionHandle) -> ConsumerActionState {
    h.progress(&mut cx(), at(1), &mut work()).unwrap()
}
fn dispose(h: Harness) {
    let Harness {
        rt, owner, bank, ..
    } = h;
    drop(rt);
    drop(owner);
    clean(&bank);
}

#[test]
fn admission_callbacks_reserve_capacity_and_observe_close_before_effects() {
    for edge in [Edge::Access, Edge::Validate, Edge::Destination, Edge::Start] {
        let h = harness(1);
        let rt = h.rt.clone();
        hook(edge, move || {
            assert!(!rt.is_idle());
            assert!(matches!(
                invoke(&rt).err().unwrap().error().error(),
                CoreError::Backpressure(_)
            ));
            rt.step(&mut cx(), at(1), &mut work());
            rt.begin_close();
        });
        let result = invoke(&h.rt);
        fired();
        if edge == Edge::Start {
            let call = result.ok().unwrap();
            progress(&call);
            let s = call.take_settlement(at(1), &mut work()).unwrap();
            assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
            assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
            drop(s);
            drop(call);
        } else {
            assert!(matches!(
                result.err().unwrap().error().error(),
                CoreError::StaleHandle(_)
            ));
            assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
        }
        dispose(h);
    }
}

#[test]
fn query_and_cancel_authorization_reentry_cannot_start_after_local_cancellation() {
    for cancel in [false, true] {
        let h = harness(1);
        let call = start(&h);
        assert_eq!(progress(&call), ConsumerActionState::Pending);
        let nested = call.clone();
        hook(Edge::Access, move || {
            nested.cancel_local();
        });
        let result = if cancel {
            call.request_cancel(at(1), &mut work())
        } else {
            call.query(at(1), &mut work())
        };
        fired();
        assert!(result.is_err());
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
        let s = call.take_settlement(at(1), &mut work()).unwrap();
        assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::NotRequested
        );
        drop(s);
        drop(call);
        dispose(h);
    }
}

fn progress_callback(edge: Edge) {
    let h = harness(1);
    h.probe.complete_invoke.store(true, Ordering::Relaxed);
    let call = start(&h);
    let nested = call.clone();
    let rt = h.rt.clone();
    hook(edge, move || {
        assert!(nested.identity().is_ok());
        assert!(!rt.is_idle());
        assert!(nested.query(at(1), &mut work()).is_err());
        assert!(nested.take_settlement(at(1), &mut work()).is_err());
        nested.cancel_local();
        let _ = nested.progress(&mut cx(), at(1), &mut work());
        rt.step(&mut cx(), at(1), &mut work());
    });
    progress(&call);
    fired();
    let s = call.take_settlement(at(1), &mut work()).unwrap();
    assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
    assert_eq!(
        h.probe.polls.load(Ordering::Relaxed),
        usize::from(edge != Edge::Access)
    );
    if edge != Edge::Access {
        assert!(s.native_result().unwrap().is_ok());
    }
    assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
    drop(s);
    drop(call);
    dispose(h);
}

#[test]
fn progress_access_reentry() {
    progress_callback(Edge::Access);
}

#[test]
fn native_poll_reentry() {
    progress_callback(Edge::Poll);
}

#[test]
fn native_retirement_reentry() {
    progress_callback(Edge::Close);
}

#[test]
fn output_validator_reentry() {
    progress_callback(Edge::Validate);
}

#[test]
fn native_disposal_reentry() {
    progress_callback(Edge::Dispose);
}

#[test]
fn settlement_access_close_fences_validated_unread_success() {
    let h = harness(1);
    h.probe.complete_invoke.store(true, Ordering::Relaxed);
    let call = start(&h);
    assert_eq!(progress(&call), ConsumerActionState::Completed);
    let rt = h.rt.clone();
    hook(Edge::Access, move || rt.begin_close());
    let s = call.take_settlement(at(1), &mut work()).unwrap();
    fired();
    assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
    assert!(s.native_result().unwrap().is_ok());
    drop(s);
    drop(call);
    dispose(h);
}

#[test]
fn abandoned_token_destruction_runs_outside_registry() {
    let h = harness(1);
    let call = start(&h);
    assert_eq!(progress(&call), ConsumerActionState::Pending);
    drop(call);
    let rt = h.rt.clone();
    hook(Edge::TokenDrop, move || {
        assert!(rt.is_idle());
        rt.begin_close();
    });
    h.rt.step(&mut cx(), at(1), &mut work());
    fired();
    dispose(h);
}

#[test]
fn nested_admission_owns_a_distinct_slot_and_cannot_overwrite_the_outer_invocation() {
    let h = harness(2);
    h.probe.complete_invoke.store(true, Ordering::Relaxed);
    let sibling = Rc::new(RefCell::new(None));
    let save = sibling.clone();
    let rt = h.rt.clone();
    hook(Edge::Start, move || {
        *save.borrow_mut() = Some(invoke(&rt).ok().unwrap());
        rt.step(&mut cx(), at(1), &mut work());
    });
    let call = start(&h);
    let sibling = sibling.borrow_mut().take().unwrap();
    assert_ne!(call.identity().unwrap(), sibling.identity().unwrap());
    assert_ne!(
        call.identity().unwrap().sequence(),
        sibling.identity().unwrap().sequence()
    );
    assert!(invoke(&h.rt).is_err());
    assert_eq!(progress(&call), ConsumerActionState::Completed);
    let s = call.take_settlement(at(1), &mut work()).unwrap();
    let other = sibling.take_settlement(at(1), &mut work()).unwrap();
    assert!(s.result().is_ok());
    assert!(other.result().is_ok());
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 2);
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 2);
    // A stale handle never gains control of the next use of its former slot.
    let replacement = start(&h);
    call.cancel_local();
    drop(call);
    assert_eq!(progress(&replacement), ConsumerActionState::Completed);
    drop(replacement.take_settlement(at(1), &mut work()).unwrap());
    drop(replacement);
    drop(sibling);
    drop(s);
    drop(other);
    dispose(h);
}

#[test]
fn control_start_reentry_preserves_returned_exchange_and_remote_cancel_uncertainty() {
    for cancel in [false, true] {
        let h = harness(1);
        let call = start(&h);
        assert_eq!(progress(&call), ConsumerActionState::Pending);
        let nested = call.clone();
        hook(Edge::Start, move || {
            assert!(nested.request_cancel(at(1), &mut work()).is_err());
            nested.cancel_local();
        });
        if cancel {
            call.request_cancel(at(1), &mut work()).unwrap();
        } else {
            call.query(at(1), &mut work()).unwrap();
        }
        assert_eq!(progress(&call), ConsumerActionState::Closed);
        let s = call.take_settlement(at(1), &mut work()).unwrap();
        assert_eq!(
            s.remote_cancellation(),
            if cancel {
                ConsumerActionCancellation::Uncertain
            } else {
                ConsumerActionCancellation::NotRequested
            }
        );
        assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
        assert_eq!(s.effect(), ConsumerEffect::Unknown);
        assert!(s.accepted());
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 2);
        assert_eq!(
            h.probe.polls.load(Ordering::Relaxed),
            1,
            "no control poll after cancellation in start"
        );
        assert_eq!(h.probe.drops.load(Ordering::Relaxed), 2);
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn cancelled_failed_query_retains_late_invalid_output_and_failed_retirement_under_original_funding()
{
    let h = harness(1);
    let call = start(&h);
    assert_eq!(progress(&call), ConsumerActionState::Pending);
    call.query(at(1), &mut work()).unwrap();
    h.probe.fail_poll.store(true, Ordering::Relaxed);
    h.probe.fail_close.store(true, Ordering::Relaxed);
    h.probe.close_pending.store(true, Ordering::Relaxed);
    let nested = call.clone();
    hook(Edge::Poll, move || nested.cancel_local());
    assert_eq!(progress(&call), ConsumerActionState::Closing);
    assert!(call.cleanup_error().unwrap().is_some());
    assert!(call.take_settlement(at(1), &mut work()).is_err());
    assert!(invoke(&h.rt).is_err());
    assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
    h.probe.late.store(true, Ordering::Relaxed);
    h.probe.invalid_output.store(true, Ordering::Relaxed);
    let counts = (
        h.probe.closes.load(Ordering::Relaxed),
        h.probe.validates.load(Ordering::Relaxed),
    );
    call.progress(&mut cx(), at(1), &mut WorkBudget::new())
        .unwrap();
    assert_eq!(
        counts,
        (
            h.probe.closes.load(Ordering::Relaxed),
            h.probe.validates.load(Ordering::Relaxed)
        )
    );
    h.probe.close_pending.store(false, Ordering::Relaxed);
    assert_eq!(progress(&call), ConsumerActionState::Closed);
    let s = call.take_settlement(at(1), &mut work()).unwrap();
    assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
    assert!(s.native_error().is_some());
    assert!(s.cleanup_error().is_some());
    assert!(s.validation_error().is_some());
    assert!(s.native_result().unwrap().is_ok());
    assert!(s.result().is_err());
    assert_eq!(s.effect(), ConsumerEffect::Unknown);
    assert_eq!(h.probe.validates.load(Ordering::Relaxed), 2);
    assert_eq!(h.probe.drops.load(Ordering::Relaxed), 2);
    let Harness {
        rt, owner, bank, ..
    } = h;
    drop(rt);
    drop(owner);
    assert!(bank.account(ResourceKind::CleanupBytesMax).used() > 0);
    drop(s);
    assert!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0,
        "original handle/control remains funded"
    );
    drop(call);
    clean(&bank);
}

struct InlineWake(Arc<AtomicUsize>);
impl Wake for InlineWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Wake);
    }
}
impl Drop for InlineWake {
    fn drop(&mut self) {
        fire(Edge::WakerDrop);
    }
}
fn wake_counter() -> (Waker, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    (Waker::from(Arc::new(InlineWake(count.clone()))), count)
}

#[test]
fn synchronous_cancellation_wake_registers_busy_progress_and_retries_after_restoration() {
    for driver in [false, true] {
        let h = harness(1);
        h.probe.pending.store(true, Ordering::Relaxed);
        let call = start(&h);
        let (waker, count) = wake_counter();
        let nested = call.clone();
        let rt = h.rt.clone();
        let retry_waker = waker.clone();
        hook(Edge::Poll, move || {
            let on_wake = nested.clone();
            hook(Edge::Wake, move || {
                let mut cx = Context::from_waker(&retry_waker);
                if driver {
                    rt.step(&mut cx, at(1), &mut work());
                } else {
                    assert!(matches!(
                        on_wake.progress(&mut cx, at(1), &mut work()),
                        Err(CoreError::Backpressure(_))
                    ));
                }
            });
            nested.cancel_local();
        });
        call.progress(&mut Context::from_waker(&waker), at(1), &mut work())
            .unwrap();
        fired();
        assert_eq!(
            count.load(Ordering::Relaxed),
            2,
            "cancellation and restoration retry"
        );
        assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
        assert_eq!(h.probe.closes.load(Ordering::Relaxed), 1);
        let s = call.take_settlement(at(1), &mut work()).unwrap();
        assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
        drop(waker);
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn close_fences_all_siblings_before_inline_wake_and_drop_never_wakes() {
    let h = harness(2);
    h.probe.pending.store(true, Ordering::Relaxed);
    let a = start(&h);
    let b = start(&h);
    let (waker, count) = wake_counter();
    h.rt.step(&mut Context::from_waker(&waker), at(1), &mut work());
    let sibling = b.clone();
    hook(Edge::Wake, move || {
        sibling.cancel_local();
    });
    h.rt.begin_close();
    fired();
    assert!(matches!(
        a.first_cause().unwrap(),
        Some(CoreError::StaleHandle(_))
    ));
    assert!(matches!(
        b.first_cause().unwrap(),
        Some(CoreError::StaleHandle(_))
    ));
    assert_eq!(count.load(Ordering::Relaxed), 2);
    drop(a);
    drop(b);
    h.rt.step(&mut cx(), at(1), &mut work());
    drop(waker);
    dispose(h);

    let h = harness(1);
    h.probe.pending.store(true, Ordering::Relaxed);
    let call = start(&h);
    let (waker, count) = wake_counter();
    call.progress(&mut Context::from_waker(&waker), at(1), &mut work())
        .unwrap();
    let Harness {
        rt,
        owner,
        bank,
        probe,
    } = h;
    drop(rt);
    drop(call);
    drop(owner);
    drop(waker);
    assert_eq!(count.load(Ordering::Relaxed), 0);
    assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
    clean(&bank);
}

#[test]
fn replacing_a_registered_waker_may_reenter_and_close_the_checked_out_invocation() {
    let h = harness(1);
    h.probe.pending.store(true, Ordering::Relaxed);
    let call = start(&h);
    let (waker, _) = wake_counter();
    call.progress(&mut Context::from_waker(&waker), at(1), &mut work())
        .unwrap();
    drop(waker);
    let rt = h.rt.clone();
    hook(Edge::WakerDrop, move || {
        rt.begin_close();
        rt.step(&mut cx(), at(1), &mut work());
    });
    assert_eq!(progress(&call), ConsumerActionState::Closed);
    fired();
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    drop(call.take_settlement(at(1), &mut work()).unwrap());
    drop(call);
    dispose(h);
}

#[test]
fn callback_can_drop_the_original_handle_or_runtime_without_losing_native_ownership() {
    for root in [false, true] {
        let h = harness(1);
        h.probe.complete_invoke.store(true, Ordering::Relaxed);
        let call = start(&h);
        if root {
            let Harness {
                rt,
                owner,
                bank,
                probe,
            } = h;
            hook(Edge::Poll, move || drop(rt));
            assert_eq!(progress(&call), ConsumerActionState::Closed);
            let s = call.take_settlement(at(1), &mut work()).unwrap();
            assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
            assert!(s.native_result().unwrap().is_ok());
            drop(s);
            drop(call);
            drop(owner);
            assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
            clean(&bank);
        } else {
            hook(Edge::Poll, move || drop(call));
            h.rt.step(&mut cx(), at(1), &mut work());
            assert!(h.rt.is_idle());
            assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
            dispose(h);
        }
        fired();
    }
}

#[test]
fn nested_progress_time_fences_admission_and_unread_handoff_without_false_regression() {
    for admission in [false, true] {
        let h = harness(1);
        h.probe.complete_invoke.store(true, Ordering::Relaxed);
        if admission {
            let rt = h.rt.clone();
            hook(Edge::Access, move || rt.step(&mut cx(), at(5), &mut work()));
        }
        let result = h.rt.invoke_action(
            "run",
            Some(Payload::new(b"12".as_slice(), "application/json")),
            Deadline::at(at(5)),
            at(1),
            &mut work(),
        );
        if admission {
            assert!(matches!(
                result.err().unwrap().error().error(),
                CoreError::TimedOut(_)
            ));
            assert_eq!(h.probe.starts.load(Ordering::Relaxed), 0);
        } else {
            let call = Rc::new(result.ok().unwrap());
            let nested = call.clone();
            hook(Edge::Validate, move || {
                let _ = nested.progress(&mut cx(), at(5), &mut work());
            });
            h.rt.step(&mut cx(), at(1), &mut work());
            let s = call.take_settlement(at(5), &mut work()).unwrap();
            assert!(matches!(s.first_cause(), Some(CoreError::TimedOut(_))));
            assert!(s.native_result().unwrap().is_ok());
            drop(s);
            drop(call);
        }
        fired();
        dispose(h);
    }
    let h = harness(2);
    let a = start(&h);
    let b = start(&h);
    let rt = h.rt.clone();
    hook(Edge::Access, move || rt.step(&mut cx(), at(2), &mut work()));
    h.rt.step(&mut cx(), at(1), &mut work());
    assert!(a.first_cause().unwrap().is_none());
    assert!(b.first_cause().unwrap().is_none());
    // A real regression, unlike resuming the outer sweep, is still a fault.
    a.progress(&mut cx(), at(1), &mut work()).unwrap();
    assert!(matches!(
        a.first_cause().unwrap(),
        Some(CoreError::Validation(_))
    ));
    b.progress(
        &mut cx(),
        MonotonicInstant::new(ClockId::new(110), 3),
        &mut work(),
    )
    .unwrap();
    assert!(matches!(
        b.first_cause().unwrap(),
        Some(CoreError::Validation(_))
    ));
    drop(a.take_settlement(at(2), &mut work()).unwrap());
    drop(b.take_settlement(at(2), &mut work()).unwrap());
    drop(a);
    drop(b);
    dispose(h);
}

#[test]
fn short_work_or_capacity_refuses_before_callbacks_and_preserves_input_and_slot() {
    let h = harness(1);
    let before = usage(&h.bank);
    let entered = Rc::new(Cell::new(false));
    let seen = entered.clone();
    hook(Edge::Access, move || seen.set(true));
    let input = Payload::new(b"12".as_slice(), "application/json");
    let original = input.body.clone();
    let refusal =
        h.rt.invoke_action(
            "run",
            Some(input),
            Deadline::NONE,
            at(1),
            &mut work().with_remaining(WorkClass::CodecInputBytes, 0),
        )
        .err()
        .unwrap();
    let (input, _) = refusal.into_parts();
    assert!(Arc::ptr_eq(&original, &input.unwrap().body));
    assert!(!entered.get());
    assert!(h.rt.is_idle());
    assert_eq!(before, usage(&h.bank));
    let k = ResourceKind::EngineLiveBytesGlobalMax;
    let account = h.bank.account(k);
    let held = h.bank.reserve(k, account.limit() - account.used()).unwrap();
    assert!(invoke(&h.rt).is_err());
    assert!(!entered.get());
    assert!(h.rt.is_idle());
    drop(held);
    let call = start(&h);
    assert!(entered.get());
    h.probe.complete_invoke.store(true, Ordering::Relaxed);
    progress(&call);
    assert!(call.take_settlement(at(1), &mut WorkBudget::new()).is_err());
    assert!(invoke(&h.rt).is_err());
    assert!(
        call.take_settlement(at(1), &mut work())
            .unwrap()
            .result()
            .is_ok()
    );
    drop(call);
    dispose(h);
}

#[test]
fn unwind_restores_occupied_owner_and_retains_native_evidence_before_validator_entry() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    for edge in [Edge::Poll, Edge::Validate] {
        let h = harness(1);
        h.probe.complete_invoke.store(true, Ordering::Relaxed);
        let call = start(&h);
        hook(edge, || panic!("injected linked callback panic"));
        assert!(catch_unwind(AssertUnwindSafe(|| progress(&call))).is_err());
        assert!(invoke(&h.rt).is_err());
        call.cancel_local();
        progress(&call);
        let s = call.take_settlement(at(1), &mut work()).unwrap();
        assert!(s.result().is_err());
        if edge == Edge::Validate {
            assert!(s.native_result().unwrap().is_ok());
        }
        assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
        assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn replacement_during_authorization_already_refuses_before_native_start() {
    // Positive control on the original implementation: revoking the independent
    // ConsumerInstance is atomic and does not itself reenter the runtime registry.
    let Harness {
        rt,
        owner,
        probe,
        bank,
    } = harness(1);
    let owner = Rc::new(owner);
    let original = owner.clone();
    hook(Edge::Access, move || original.revoke());
    let rejected = invoke(&rt).err().unwrap();
    assert!(matches!(
        rejected.error().error(),
        CoreError::StaleHandle(_)
    ));
    assert_eq!(probe.starts.load(Ordering::Relaxed), 0);
    assert!(rt.is_idle());
    let replacement =
        ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    assert!(replacement.reference().is_current());
    assert!(!owner.reference().is_current());
    drop(rejected);
    drop(rt);
    drop(owner);
    drop(replacement);
    clean(&bank);
}

#[test]
fn service_loss_during_control_access_stays_fenced_after_service_recovery() {
    for cancel in [false, true] {
        let h = harness(1);
        let call = start(&h);
        assert_eq!(progress(&call), ConsumerActionState::Pending);
        let p = h.probe.clone();
        hook(Edge::Access, move || {
            p.unavailable.store(true, Ordering::Relaxed)
        });
        let result = if cancel {
            call.request_cancel(at(1), &mut work())
        } else {
            call.query(at(1), &mut work())
        };
        assert!(result.is_err());
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
        h.probe.unavailable.store(false, Ordering::Relaxed);
        assert!(call.query(at(1), &mut work()).is_err());
        let s = call.take_settlement(at(1), &mut work()).unwrap();
        assert!(matches!(s.result(), Err(CoreError::Validation(_))));
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::NotRequested
        );
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn native_inline_wake_cannot_repoll_and_preserves_driver_retry_interest() {
    let h = harness(1);
    let call = start(&h);
    h.probe.pending.store(true, Ordering::Relaxed);
    h.probe.wake_in_poll.store(true, Ordering::Relaxed);
    let (waker, count) = wake_counter();
    let nested_waker = waker.clone();
    let rt = h.rt.clone();
    let nested = call.clone();
    let restored = Rc::new(Cell::new(false));
    let seen = restored.clone();
    hook(Edge::Wake, move || {
        rt.step(&mut Context::from_waker(&nested_waker), at(1), &mut work());
        hook(Edge::Wake, move || {
            // The restoration wake can execute synchronously too. A second
            // poll is legal only after the original owner has been restored.
            assert_eq!(progress(&nested), ConsumerActionState::Invoking);
            seen.set(true);
        });
    });
    call.progress(&mut Context::from_waker(&waker), at(1), &mut work())
        .unwrap();
    fired();
    assert!(restored.get());
    assert_eq!(count.load(Ordering::Relaxed), 2);
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 2);
    call.cancel_local();
    progress(&call);
    drop(call.take_settlement(at(1), &mut work()).unwrap());
    drop(waker);
    drop(call);
    dispose(h);
}
