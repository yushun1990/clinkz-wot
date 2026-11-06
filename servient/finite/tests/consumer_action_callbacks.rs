//! Public continuing Action ownership under deterministic linked-code reentry.
//! Actual HTTP and downstream datagram lifecycle tests supply transport evidence.
use clinkz_wot_core::{
    consumer::*, consumer_action::*, data_schema::PreparedSchema, host_accounts::*, *,
};
use clinkz_wot_foundation::*;
use clinkz_wot_servient_finite::consumer_action::*;
use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
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
    fn poll(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>> {
        self.probe.polls.fetch_add(1, Ordering::Relaxed);
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

#[test]
fn progress_native_validation_and_disposal_callbacks_can_reenter_without_repolling() {
    for edge in [
        Edge::Access,
        Edge::Poll,
        Edge::Close,
        Edge::Validate,
        Edge::Dispose,
    ] {
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
