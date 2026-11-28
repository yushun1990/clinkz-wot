//! Public continuing Action ownership under deterministic linked-code reentry.
//! Actual HTTP and downstream datagram lifecycle tests supply transport evidence.
use clinkz_wot_core::execution::ValueValidator;
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
    AccessDrop,
    Validate,
    Destination,
    Start,
    Poll,
    Close,
    Dispose,
    TokenDrop,
    Wake,
    WakerClone,
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
impl ValueValidator for Data {
    fn content_type(&self) -> &str {
        ConsumerDataService::content_type(self)
    }
    fn supports(&self, schema: &PreparedSchema) -> bool {
        ConsumerDataService::supports_schema(self, schema)
    }
    fn body_bytes(&self) -> usize {
        self.policy().body_bytes as usize
    }
    fn available(&self) -> bool {
        ConsumerDataService::available(self)
    }
    fn validate(&self, _: &PreparedSchema, p: PayloadView<'_>) -> CoreResult<()> {
        self.0.validates.fetch_add(1, Ordering::Relaxed);
        fire(Edge::Validate);
        if p.body == b"12" || p.body == b"24" {
            Ok(())
        } else {
            Err(invalid())
        }
    }
}
impl ConsumerDataService for Data {
    fn action_validator(&self) -> Option<&dyn ValueValidator> {
        Some(self)
    }

    fn qualification_revision(&self) -> u64 {
        0
    }

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
    fn validate(&self, schema: &PreparedSchema, p: &Payload) -> CoreResult<()> {
        ValueValidator::validate(self, schema, p.view())
    }
}
struct Access;
impl Drop for Access {
    fn drop(&mut self) {
        fire(Edge::AccessDrop);
    }
}
impl ConsumerAccessService for Access {
    fn qualification_revision(&self) -> u64 {
        0
    }

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
        _destination: &str,
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
        ConsumerActionReply::Complete(
            InteractionOutput::with_data(Payload::new(
                if self.probe.invalid_output.load(Ordering::Relaxed) {
                    b"bad".as_slice()
                } else {
                    b"24".as_slice()
                },
                "application/json",
            ))
            .into(),
        )
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
struct TestClient {
    calls: ConsumerActions,
    association: ConsumerActionAssociation,
}
struct Harness {
    rt: Rc<TestClient>,
    owner: ConsumerInstance,
    probe: Arc<Probe>,
    codecs: [Arc<Probe>; 3],
    bank: Arc<HostResourceAccounts>,
}
fn harness(slots: usize) -> Harness {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(Probe::default());
    let codecs = [
        probe.clone(),
        Arc::new(Probe::default()),
        Arc::new(Probe::default()),
    ];
    let data = ConsumerServiceOwner::retain_data(Arc::new(Data(probe.clone())), &bank)
        .ok()
        .unwrap();
    let access = ConsumerServiceOwner::retain_access(Arc::new(Access), &bank)
        .ok()
        .unwrap();
    let op = |operation| {
        let mapping = Box::new(Mapping {
            operation,
            probe: probe.clone(),
            bank: bank.clone(),
        });
        let bounds = mapping.bounds();
        let identity = mapping.identity();
        // Each representation has an independent actual validation dependency.
        let data = if operation == Operation::InvokeAction {
            data.clone()
        } else {
            let i = if operation == Operation::QueryAction {
                1
            } else {
                2
            };
            ConsumerServiceOwner::retain_data(Arc::new(Data(codecs[i].clone())), &bank)
                .ok()
                .unwrap()
        };
        ConsumerActionOperation {
            mapping,
            endpoint: "test://action".into(),
            form: 0,
            data: data.clone(),
            body_bytes: data.policy().body_bytes,
            validation_scratch_bytes: data.policy().temporary_bytes,
            content_type: "application/json".into(),
            bounds,
            identity,
            operation,
        }
    };
    let d = ConsumerActionDefinition::try_new(ConsumerActionDefinitionInput {
        target: AffordanceTarget::Action("run".into()),
        schemas: ValueSchemas {
            input: Some(PreparedSchema::Any),
            output: Some(PreparedSchema::Any),
        },
        invoke: op(Operation::InvokeAction),
        query: Some(op(Operation::QueryAction)),
        cancel: Some(op(Operation::CancelAction)),
        funding: vec![],
    })
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
    let association = ConsumerActionAssociation::try_new(
        owner.reference(),
        Rc::new(d),
        ConsumerActionAccess {
            invoke: access.clone(),
            query: Some(access.clone()),
            cancel: Some(access),
        },
        bank.clone(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let rt = Rc::new(TestClient {
        association,
        calls: ConsumerActions::try_new(slots, bank.clone()).unwrap(),
    });
    Harness {
        rt,
        owner,
        probe,
        codecs,
        bank,
    }
}
fn invoke(rt: &TestClient) -> Result<ConsumerActionHandle, BindingInputRejection<Option<Payload>>> {
    rt.calls.invoke_action(
        &rt.association,
        Some(Payload::new(b"12".as_slice(), "application/json")),
        Deadline::NONE,
        at(1),
    )
}
fn start(h: &Harness) -> Rc<ConsumerActionHandle> {
    Rc::new(invoke(&h.rt).unwrap_or_else(|e| panic!("{:?}", e.error())))
}
fn progress(h: &ConsumerActionHandle) -> ConsumerActionState {
    for _ in 0..4 {
        if h.progress(&mut cx(), at(1)).unwrap() != ActionTurn::Advanced {
            break;
        }
        let state = h.state().unwrap();
        if state == ConsumerActionState::Pending {
            return state;
        }
    }
    h.state().unwrap()
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
fn last_idle_association_view_disposes_access_without_polling_a_sibling() {
    let idle = harness(1);
    let TestClient { association, calls } = Rc::try_unwrap(idle.rt).ok().unwrap();
    let view = association.clone();
    let sibling = harness(1);
    let call = start(&sibling);
    let rt = sibling.rt.clone();
    hook(Edge::AccessDrop, move || {
        assert!(matches!(
            rt.calls.step(&mut cx(), at(1)),
            Err(CoreError::Lifecycle(_))
        ))
    });
    drop(association);
    assert!(view.consumer().is_current());
    drop(view);
    fired();
    assert!(calls.is_idle());
    assert_eq!(sibling.probe.polls.load(Ordering::Relaxed), 0);
    assert_eq!(progress(&call), ConsumerActionState::Pending);
    drop((call, calls, idle.owner));
    clean(&idle.bank);
    dispose(sibling);
}

#[test]
fn admission_callbacks_reserve_capacity_and_observe_close_before_effects() {
    for edge in [Edge::Access, Edge::Validate, Edge::Destination, Edge::Start] {
        let h = harness(1);
        let rt = h.rt.clone();
        hook(edge, move || {
            assert!(!rt.calls.is_idle());
            assert!(matches!(
                invoke(&rt).err().unwrap().error().error(),
                CoreError::Lifecycle(_)
            ));
            assert!(matches!(
                rt.calls.step(&mut cx(), at(1)),
                Err(CoreError::Lifecycle(_))
            ));
            rt.calls.begin_close();
        });
        let result = invoke(&h.rt);
        fired();
        if edge == Edge::Start {
            let call = result.ok().unwrap();
            progress(&call);
            let s = call.take_settlement(at(1)).unwrap();
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
            call.request_cancel(at(1))
        } else {
            call.query(at(1))
        };
        fired();
        assert!(result.is_err());
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
        assert_eq!(progress(&call), ConsumerActionState::Closed);
        let s = call.take_settlement(at(1)).unwrap();
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
        assert!(!rt.calls.is_idle());
        assert!(matches!(nested.query(at(1)), Err(CoreError::Lifecycle(_))));
        assert!(matches!(
            nested.take_settlement(at(1)),
            Err(CoreError::Lifecycle(_))
        ));
        nested.cancel_local();
        assert!(matches!(
            nested.progress(&mut cx(), at(1)),
            Err(CoreError::Lifecycle(_))
        ));
        assert!(matches!(
            rt.calls.step(&mut cx(), at(1)),
            Err(CoreError::Lifecycle(_))
        ));
    });
    progress(&call);
    fired();
    let s = call.take_settlement(at(1)).unwrap();
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
    hook(Edge::Access, move || rt.calls.begin_close());
    let s = call.take_settlement(at(1)).unwrap();
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
        assert!(!rt.calls.is_idle());
        rt.calls.begin_close();
    });
    h.rt.calls.step(&mut cx(), at(1)).unwrap();
    assert!(h.rt.calls.is_idle());
    fired();
    dispose(h);
}

#[test]
fn callback_admission_returns_its_input_and_the_next_turn_can_use_the_free_slot() {
    let h = harness(2);
    h.probe.complete_invoke.store(true, Ordering::Relaxed);
    let rt = h.rt.clone();
    hook(Edge::Start, move || {
        let (input, error) = invoke(&rt).err().unwrap().into_parts();
        assert!(matches!(error.error(), CoreError::Lifecycle(_)));
        assert_eq!(input.unwrap().body.as_ref(), b"12");
        assert!(matches!(
            rt.calls.step(&mut cx(), at(1)),
            Err(CoreError::Lifecycle(_))
        ));
    });
    let call = start(&h);
    fired();
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 0);
    let sibling = start(&h);
    assert_ne!(call.identity().unwrap(), sibling.identity().unwrap());
    assert_ne!(
        call.identity().unwrap().sequence(),
        sibling.identity().unwrap().sequence()
    );
    assert!(invoke(&h.rt).is_err());
    assert_eq!(progress(&call), ConsumerActionState::Completed);
    assert_eq!(progress(&sibling), ConsumerActionState::Completed);
    let s = call.take_settlement(at(1)).unwrap();
    let other = sibling.take_settlement(at(1)).unwrap();
    assert!(s.result().is_ok());
    assert!(other.result().is_ok());
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 2);
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 2);
    // A stale handle never gains control of the next use of its former slot.
    let replacement = start(&h);
    call.cancel_local();
    drop(call);
    assert_eq!(progress(&replacement), ConsumerActionState::Completed);
    drop(replacement.take_settlement(at(1)).unwrap());
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
            assert!(nested.request_cancel(at(1)).is_err());
            nested.cancel_local();
        });
        if cancel {
            call.request_cancel(at(1)).unwrap();
        } else {
            call.query(at(1)).unwrap();
        }
        assert_eq!(progress(&call), ConsumerActionState::Closed);
        let s = call.take_settlement(at(1)).unwrap();
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
    call.query(at(1)).unwrap();
    h.probe.fail_poll.store(true, Ordering::Relaxed);
    h.probe.fail_close.store(true, Ordering::Relaxed);
    h.probe.close_pending.store(true, Ordering::Relaxed);
    let nested = call.clone();
    hook(Edge::Poll, move || nested.cancel_local());
    assert_eq!(progress(&call), ConsumerActionState::Closing);
    assert!(call.cleanup_error().unwrap().is_some());
    assert!(call.take_settlement(at(1)).is_err());
    assert!(invoke(&h.rt).is_err());
    assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
    h.probe.late.store(true, Ordering::Relaxed);
    h.probe.invalid_output.store(true, Ordering::Relaxed);
    let counts = (
        h.probe.closes.load(Ordering::Relaxed),
        h.codecs[1].validates.load(Ordering::Relaxed),
    );
    call.progress(&mut cx(), at(1)).unwrap();
    assert_eq!(
        (counts.0 + 1, counts.1),
        (
            h.probe.closes.load(Ordering::Relaxed),
            h.codecs[1].validates.load(Ordering::Relaxed)
        )
    );
    h.probe.close_pending.store(false, Ordering::Relaxed);
    assert_eq!(progress(&call), ConsumerActionState::Closed);
    let s = call.take_settlement(at(1)).unwrap();
    assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
    assert!(s.native_error().is_some());
    assert!(s.cleanup_error().is_some());
    assert!(s.validation_error().is_none());
    assert!(s.native_result().unwrap().is_ok());
    assert!(s.result().is_err());
    assert_eq!(s.effect(), ConsumerEffect::Unknown);
    assert_eq!(h.probe.validates.load(Ordering::Relaxed), 1);
    assert_eq!(h.codecs[1].validates.load(Ordering::Relaxed), 0);
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

// The safe Wake adapter clones an Arc internally. A stateless raw waker also
// lets the fixture exercise a callback from clone itself.
fn linked_waker() -> Waker {
    use std::task::{RawWaker, RawWakerVTable};
    fn raw() -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| {
            fire(Edge::WakerClone);
            raw()
        },
        |_| fire(Edge::Wake),
        |_| fire(Edge::Wake),
        |_| {},
    );
    // SAFETY: the vtable is static, ignores its data pointer and uses only
    // thread-local hooks. Every clone is independently valid on any thread.
    unsafe { Waker::from_raw(raw()) }
}

#[test]
fn close_and_cancellation_during_waker_replacement_reach_the_new_waiter() {
    for close in [false, true] {
        let h = harness(1);
        h.probe.pending.store(true, Ordering::Relaxed);
        let call = start(&h);
        let (old, wakes) = wake_counter();
        assert_eq!(
            call.progress(&mut Context::from_waker(&old), at(1))
                .unwrap(),
            ActionTurn::Waiting
        );

        let cancel = call.clone();
        let runtime = h.rt.clone();
        let notified = Rc::new(Cell::new(false));
        let seen = notified.clone();
        hook(Edge::WakerClone, move || {
            if close {
                runtime.calls.begin_close();
            } else {
                cancel.cancel_local();
            }
            hook(Edge::Wake, move || seen.set(true));
        });
        assert_eq!(
            call.progress(&mut Context::from_waker(&linked_waker()), at(1),)
                .unwrap(),
            ActionTurn::Settled
        );
        fired();
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        assert!(notified.get());
        assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
        let result = call.take_settlement(at(1)).unwrap();
        if close {
            assert!(matches!(
                result.first_cause(),
                Some(CoreError::StaleHandle(_))
            ));
        } else {
            assert!(matches!(
                result.first_cause(),
                Some(CoreError::Cancelled(_))
            ));
        }
        drop((result, call));
        dispose(h);
    }
}

#[test]
fn continuation_retires_under_the_turn_before_result_disposal() {
    let original = harness(1);
    let call = start(&original);
    assert_eq!(progress(&call), ConsumerActionState::Pending);
    call.query(at(1)).unwrap();

    let sibling = harness(1);
    let next = start(&sibling);
    let rt = sibling.rt.clone();
    hook(Edge::TokenDrop, move || {
        assert!(matches!(
            rt.calls.step(&mut cx(), at(1)),
            Err(CoreError::Lifecycle(_))
        ));
    });
    assert_eq!(progress(&call), ConsumerActionState::Completed);
    fired();
    let result = call.take_settlement(at(1)).unwrap();
    drop((call, original.rt, original.owner));
    assert_eq!(sibling.probe.polls.load(Ordering::Relaxed), 0);
    assert_eq!(result.result().unwrap().data().unwrap().body, b"24");
    drop(result);
    clean(&original.bank);
    assert_eq!(progress(&next), ConsumerActionState::Pending);
    drop(next);
    dispose(sibling);
}

#[test]
fn close_fences_all_siblings_before_inline_wake_and_drop_never_wakes() {
    let h = harness(2);
    h.probe.pending.store(true, Ordering::Relaxed);
    let a = start(&h);
    let b = start(&h);
    let (waker, count) = wake_counter();
    for _ in 0..2 {
        h.rt.calls
            .step(&mut Context::from_waker(&waker), at(1))
            .unwrap();
    }
    let sibling = b.clone();
    hook(Edge::Wake, move || {
        sibling.cancel_local();
    });
    h.rt.calls.begin_close();
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
    h.rt.calls.step(&mut cx(), at(1)).unwrap();
    drop(waker);
    dispose(h);

    let h = harness(1);
    h.probe.pending.store(true, Ordering::Relaxed);
    let call = start(&h);
    let (waker, count) = wake_counter();
    call.progress(&mut Context::from_waker(&waker), at(1))
        .unwrap();
    let Harness {
        rt,
        owner,
        bank,
        probe,
        ..
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
    call.progress(&mut Context::from_waker(&waker), at(1))
        .unwrap();
    drop(waker);
    let rt = h.rt.clone();
    hook(Edge::WakerDrop, move || {
        rt.calls.begin_close();
        assert!(matches!(
            rt.calls.step(&mut cx(), at(1)),
            Err(CoreError::Lifecycle(_))
        ));
    });
    assert_eq!(progress(&call), ConsumerActionState::Closed);
    fired();
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    drop(call.take_settlement(at(1)).unwrap());
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
                ..
            } = h;
            hook(Edge::Poll, move || drop(rt));
            assert_eq!(progress(&call), ConsumerActionState::Closed);
            let s = call.take_settlement(at(1)).unwrap();
            assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
            assert!(s.native_result().unwrap().is_ok());
            drop(s);
            drop(call);
            drop(owner);
            assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
            clean(&bank);
        } else {
            hook(Edge::Poll, move || drop(call));
            assert_eq!(
                h.rt.calls.step(&mut cx(), at(1)).unwrap(),
                ActionTurn::Advanced
            );
            assert!(!h.rt.calls.is_idle());
            assert_eq!(
                h.rt.calls.step(&mut cx(), at(1)).unwrap(),
                ActionTurn::Settled
            );
            assert!(h.rt.calls.is_idle());
            assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
            dispose(h);
        }
        fired();
    }
}

#[test]
fn independent_action_turns_fence_clock_regressions_and_domain_changes() {
    let h = harness(2);
    let a = start(&h);
    let b = start(&h);
    h.rt.calls.step(&mut cx(), at(2)).unwrap();
    assert!(a.first_cause().unwrap().is_none());
    assert!(b.first_cause().unwrap().is_none());
    a.progress(&mut cx(), at(1)).unwrap();
    assert!(matches!(
        a.first_cause().unwrap(),
        Some(CoreError::Validation(_))
    ));
    b.progress(&mut cx(), MonotonicInstant::new(ClockId::new(110), 3))
        .unwrap();
    assert!(matches!(
        b.first_cause().unwrap(),
        Some(CoreError::Validation(_))
    ));
    drop(a.take_settlement(at(2)).unwrap());
    drop(b.take_settlement(at(2)).unwrap());
    drop(a);
    drop(b);
    dispose(h);
}

#[test]
fn full_host_quota_refuses_before_callbacks_and_preserves_input_and_slot() {
    let h = harness(1);
    let entered = Rc::new(Cell::new(false));
    let seen = entered.clone();
    hook(Edge::Access, move || seen.set(true));
    let k = ResourceKind::EngineLiveBytesGlobalMax;
    let account = h.bank.account(k);
    let held = h.bank.reserve(k, account.limit() - account.used()).unwrap();
    let before = usage(&h.bank);
    let input = Payload::new(b"12".as_slice(), "application/json");
    let original = input.body.clone();
    let (input, error) =
        h.rt.calls
            .invoke_action(&h.rt.association, Some(input), Deadline::NONE, at(1))
            .err()
            .unwrap()
            .into_parts();
    assert!(matches!(error.error(), CoreError::LimitExceeded { resource, .. } if *resource == k));
    assert!(Arc::ptr_eq(&original, &input.unwrap().body));
    assert!(!entered.get());
    assert!(h.rt.calls.is_idle());
    assert_eq!(before, usage(&h.bank));
    drop(held);
    h.probe.complete_invoke.store(true, Ordering::Relaxed);
    let call = start(&h);
    assert!(entered.get());
    assert_eq!(
        call.progress(&mut cx(), at(1)).unwrap(),
        ActionTurn::Advanced
    );
    assert!(call.take_settlement(at(1)).is_err());
    assert!(invoke(&h.rt).is_err());
    assert_eq!(progress(&call), ConsumerActionState::Completed);
    assert!(call.take_settlement(at(1)).unwrap().result().is_ok());
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
        let s = call.take_settlement(at(1)).unwrap();
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
        ..
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
    assert!(rt.calls.is_idle());
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
fn retired_invoke_codec_loss_does_not_revoke_independent_control_or_query_output() {
    for cancel in [false, true] {
        let h = harness(2);
        let call = start(&h);
        assert_eq!(progress(&call), ConsumerActionState::Pending);
        let p = h.probe.clone();
        hook(Edge::Access, move || {
            p.unavailable.store(true, Ordering::Relaxed)
        });
        let result = if cancel {
            call.request_cancel(at(1))
        } else {
            call.query(at(1))
        };
        result.unwrap();
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 2);
        // Full qualification is still required for new work using this definition.
        let refused = invoke(&h.rt).err().unwrap();
        assert!(matches!(refused.error().error(), CoreError::Validation(_)));
        drop(refused);
        if cancel {
            assert_eq!(progress(&call), ConsumerActionState::Pending);
            call.query(at(1)).unwrap();
        }
        assert_eq!(progress(&call), ConsumerActionState::Completed);
        let s = call.take_settlement(at(1)).unwrap();
        assert!(s.result().is_ok());
        assert_eq!(h.probe.validates.load(Ordering::Relaxed), 1);
        assert_eq!(h.codecs[1].validates.load(Ordering::Relaxed), 1);
        assert_eq!(h.codecs[2].validates.load(Ordering::Relaxed), 0);
        assert_eq!(
            s.remote_cancellation(),
            if cancel {
                ConsumerActionCancellation::Requested
            } else {
                ConsumerActionCancellation::NotRequested
            }
        );
        drop(s);
        drop(call);
        dispose(h);
    }
}

#[test]
fn selected_query_codec_loss_refuses_before_effects_without_revoking_original_work() {
    let h = harness(1);
    let call = start(&h);
    assert_eq!(progress(&call), ConsumerActionState::Pending);
    let query = h.codecs[1].clone();
    hook(Edge::Access, move || {
        query.unavailable.store(true, Ordering::Relaxed)
    });
    assert!(call.query(at(1)).is_err());
    assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
    assert!(call.query(at(1)).is_err());
    call.request_cancel(at(1)).unwrap();
    assert_eq!(progress(&call), ConsumerActionState::Pending);
    h.codecs[1].unavailable.store(false, Ordering::Relaxed);
    call.query(at(1)).unwrap();
    assert_eq!(progress(&call), ConsumerActionState::Completed);
    let s = call.take_settlement(at(1)).unwrap();
    assert!(s.result().is_ok());
    assert_eq!(
        s.remote_cancellation(),
        ConsumerActionCancellation::Requested
    );
    drop((s, call));
    dispose(h);
}

#[test]
fn native_inline_wake_cannot_repoll_or_advance_the_action_clock() {
    let h = harness(1);
    let call = start(&h);
    h.probe.pending.store(true, Ordering::Relaxed);
    h.probe.wake_in_poll.store(true, Ordering::Relaxed);
    let (waker, count) = wake_counter();
    let nested = call.clone();
    let cloned = Rc::new(Cell::new(false));
    let seen = cloned.clone();
    hook(Edge::Wake, move || {
        hook(Edge::WakerClone, move || seen.set(true));
        let result = nested.progress(&mut Context::from_waker(&linked_waker()), at(9));
        assert!(matches!(result, Err(CoreError::Lifecycle(error))
            if error.phase() == ErrorPhase::Readiness && error.retry_class() == RetryClass::Never));
    });
    call.progress(&mut Context::from_waker(&waker), at(1))
        .unwrap();
    assert!(!cloned.get());
    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 1);
    assert_eq!(
        call.progress(&mut Context::from_waker(&linked_waker()), at(1),)
            .unwrap(),
        ActionTurn::Waiting
    );
    assert!(cloned.get());
    fired();
    assert_eq!(h.probe.polls.load(Ordering::Relaxed), 2);
    call.cancel_local();
    progress(&call);
    drop(call.take_settlement(at(1)).unwrap());
    drop(waker);
    drop(call);
    dispose(h);
}

struct OperationAccess {
    operation: Operation,
    revision: AtomicUsize,
    authorizations: AtomicUsize,
}
impl ConsumerAccessService for OperationAccess {
    fn qualification_revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed) as u64
    }
    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        _: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        assert_eq!(operation, self.operation);
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        assert_eq!(operation, self.operation);
        self.authorizations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        64
    }
    fn work(&self) -> u64 {
        1
    }
}
fn operation_association(h: &Harness) -> (ConsumerActionAssociation, [Arc<OperationAccess>; 3]) {
    let services = [
        Operation::InvokeAction,
        Operation::QueryAction,
        Operation::CancelAction,
    ]
    .map(|operation| {
        Arc::new(OperationAccess {
            operation,
            revision: AtomicUsize::new(0),
            authorizations: AtomicUsize::new(0),
        })
    });
    let owners = services.each_ref().map(|s| {
        ConsumerServiceOwner::retain_access(s.clone(), &h.bank)
            .ok()
            .unwrap()
    });
    let [invoke, query, cancel] = owners;
    let association = ConsumerActionAssociation::try_new(
        h.owner.reference(),
        h.rt.association.definition().clone(),
        ConsumerActionAccess {
            invoke,
            query: Some(query),
            cancel: Some(cancel),
        },
        h.bank.clone(),
    )
    .ok()
    .unwrap();
    (association, services)
}

#[test]
fn unused_query_revision_cannot_revoke_original_invoke_completion_or_handoff() {
    for after_completion in [false, true] {
        let h = harness(1);
        h.probe.complete_invoke.store(true, Ordering::Relaxed);
        let (association, services) = operation_association(&h);
        let call =
            h.rt.calls
                .invoke_action(
                    &association,
                    Some(Payload::new(b"12".as_slice(), "application/json")),
                    Deadline::NONE,
                    at(1),
                )
                .ok()
                .unwrap();
        let identity = call.identity().unwrap();
        if after_completion {
            assert_eq!(progress(&call), ConsumerActionState::Completed);
        }
        services[1].revision.store(1, Ordering::Relaxed);
        assert_eq!(progress(&call), ConsumerActionState::Completed);
        assert!(call.first_cause().unwrap().is_none());
        let settlement = call.take_settlement(at(1)).unwrap();
        assert_eq!(settlement.identity(), &identity);
        assert_eq!(settlement.result().unwrap().data().unwrap().body, b"24");
        assert_eq!(services[1].authorizations.load(Ordering::Relaxed), 0);
        assert_eq!(services[2].authorizations.load(Ordering::Relaxed), 0);
        assert_eq!(h.probe.starts.load(Ordering::Relaxed), 1);
        assert!(h.rt.calls.is_idle());
        assert_eq!(
            h.bank
                .account(ResourceKind::PendingHandlerCallsGlobalMax)
                .used(),
            1
        );
        drop((call, association, services));
        assert_eq!(
            h.bank
                .account(ResourceKind::PendingHandlerCallsGlobalMax)
                .used(),
            1
        );
        drop((settlement, identity));
        dispose(h);
    }
}

#[test]
fn selected_control_revision_fences_owned_exchange_and_retains_late_cleanup() {
    for operation in [1, 2] {
        let h = harness(1);
        let (association, services) = operation_association(&h);
        let call =
            h.rt.calls
                .invoke_action(
                    &association,
                    Some(Payload::new(b"12".as_slice(), "application/json")),
                    Deadline::NONE,
                    at(1),
                )
                .ok()
                .unwrap();
        let identity = call.identity().unwrap();
        assert_eq!(progress(&call), ConsumerActionState::Pending);
        if operation == 1 {
            call.query(at(1)).unwrap();
        } else {
            call.request_cancel(at(1)).unwrap();
        }
        h.probe.close_pending.store(true, Ordering::Relaxed);
        h.probe.late.store(operation == 1, Ordering::Relaxed);
        services[operation].revision.store(1, Ordering::Relaxed);
        assert_eq!(progress(&call), ConsumerActionState::Closing);
        assert!(matches!(
            call.first_cause().unwrap(),
            Some(CoreError::Validation(_))
        ));
        assert_eq!(
            h.probe.polls.load(Ordering::Relaxed),
            1,
            "invalid selected work can only retire"
        );
        assert_eq!(
            h.probe.drops.load(Ordering::Relaxed),
            1,
            "the control owner remains funded"
        );
        let closes = h.probe.closes.load(Ordering::Relaxed);
        assert_eq!(
            call.progress(&mut cx(), at(1)).unwrap(),
            ActionTurn::Waiting
        );
        assert_eq!(h.probe.closes.load(Ordering::Relaxed), closes + 1);
        // Adversarial restoration cannot erase the violation already observed.
        services[operation].revision.store(0, Ordering::Relaxed);
        assert!(call.query(at(1)).is_err());
        h.probe.close_pending.store(false, Ordering::Relaxed);
        assert_eq!(progress(&call), ConsumerActionState::Closed);
        let settlement = call.take_settlement(at(1)).unwrap();
        assert_eq!(settlement.identity(), &identity);
        assert!(matches!(settlement.result(), Err(CoreError::Validation(_))));
        assert_eq!(
            settlement.remote_cancellation(),
            if operation == 2 {
                ConsumerActionCancellation::Uncertain
            } else {
                ConsumerActionCancellation::NotRequested
            }
        );
        if operation == 1 {
            assert_eq!(
                settlement
                    .native_result()
                    .unwrap()
                    .unwrap()
                    .data()
                    .unwrap()
                    .body,
                b"24"
            );
            assert_eq!(h.probe.validates.load(Ordering::Relaxed), 1);
            assert_eq!(h.codecs[1].validates.load(Ordering::Relaxed), 0);
        }
        assert_eq!(h.probe.drops.load(Ordering::Relaxed), 2);
        drop((call, association, services, settlement, identity));
        dispose(h);
    }
}

#[path = "support/worker_exit.rs"]
mod worker_exit;

#[test]
fn worker_exit_retires_a_parked_action() {
    worker_exit::check(
        || {
            let h = harness(1);
            h.probe.pending.store(true, Ordering::Relaxed);
            let call = start(&h);
            (h, call)
        },
        |(_, call)| assert_eq!(progress(call), ConsumerActionState::Invoking),
        |(h, call)| {
            call.cancel_local();
            assert_eq!(progress(&call), ConsumerActionState::Closed);
            let report = call.take_settlement(at(1)).unwrap();
            assert!(matches!(report.result(), Err(CoreError::Cancelled(_))));
            assert_eq!(h.probe.closes.load(Ordering::Relaxed), 1);
            assert_eq!(h.probe.drops.load(Ordering::Relaxed), 1);
            drop((report, call));
            dispose(h);
        },
    );
}
