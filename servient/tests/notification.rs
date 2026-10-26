#![cfg(feature = "std")]
//! Deterministic native capability tests. The in-memory Binding, not the
//! Engine, matches sources, distributes data and owns physical retirement.
use clinkz_wot_core::{notification::*, *};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_servient::notification::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::{Rc, Weak},
    sync::Arc,
    task::{Context, Poll, Waker},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Ready,
    Pending,
    Fail,
}
struct Program {
    source: &'static str,
    tenant: u8,
    responses: bool,
    cancellation: bool,
    start: Mode,
}
impl Program {
    fn property() -> Self {
        Self {
            source: "sensor",
            tenant: 1,
            responses: false,
            cancellation: true,
            start: Mode::Ready,
        }
    }
    fn event() -> Self {
        Self {
            responses: true,
            ..Self::property()
        }
    }
    fn physical(&self) -> (&'static str, u8) {
        (self.source, self.tenant)
    }
}
#[derive(Default)]
struct Physical {
    participants: usize,
    jobs: usize,
}
struct Endpoint {
    identity: NotificationIdentity,
    physical: (&'static str, u8),
    responses: bool,
    joined: bool,
    queue: VecDeque<NativeNotification<Reply>>,
    bytes: usize,
    loss: u64,
    wake: Option<Waker>,
}
struct Bus {
    endpoints: Vec<Weak<RefCell<Endpoint>>>,
    physical: BTreeMap<(&'static str, u8), Physical>,
    prepare_calls: usize,
    start_polls: usize,
    receive_polls: usize,
    subscriptions: usize,
    native_acks: usize,
    close_polls: usize,
    completed_closes: usize,
    aborts: usize,
    reply_aborts: usize,
    reply_retire_polls: usize,
    reply_send_polls: usize,
    control_inputs: Vec<Vec<u8>>,
    response_inputs: Vec<Vec<u8>>,
    response_bodies: Vec<usize>,
    close: Mode,
    send: Mode,
    retire: Mode,
    retain_idle: bool,
    on_next: Option<Box<dyn FnOnce()>>,
    on_send: Option<Box<dyn FnOnce()>>,
    receive_error: bool,
    late_response_delivered: bool,
}
impl Bus {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            endpoints: vec![],
            physical: BTreeMap::new(),
            prepare_calls: 0,
            start_polls: 0,
            receive_polls: 0,
            subscriptions: 0,
            native_acks: 0,
            close_polls: 0,
            completed_closes: 0,
            aborts: 0,
            reply_aborts: 0,
            reply_retire_polls: 0,
            reply_send_polls: 0,
            control_inputs: vec![],
            response_inputs: vec![],
            response_bodies: vec![],
            close: Mode::Ready,
            send: Mode::Ready,
            retire: Mode::Ready,
            retain_idle: false,
            on_next: None,
            on_send: None,
            receive_error: false,
            late_response_delivered: false,
        }))
    }
    fn retire_physical(&mut self, key: (&'static str, u8)) {
        if !self.retain_idle
            && self
                .physical
                .get(&key)
                .is_some_and(|p| p.participants == 0 && p.jobs == 0)
        {
            self.physical.remove(&key);
        }
    }
    // Only the application-owned infrastructure service has this capability.
    fn shutdown_idle(&mut self) {
        self.physical
            .retain(|_, p| p.participants > 0 || p.jobs > 0);
    }
    fn publish(bus: &Rc<RefCell<Self>>, key: (&'static str, u8), data: Option<Payload>) {
        let endpoints = bus
            .borrow()
            .endpoints
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>();
        for endpoint in endpoints {
            let mut endpoint = endpoint.borrow_mut();
            if !endpoint.joined || endpoint.physical != key {
                continue;
            }
            let bytes = notification_payload_backing(data.as_ref()).unwrap();
            bus.borrow_mut().native_acks += 1; // Native ACK never creates a dataResponse.
            // Explicit native item AND byte limits, independent of Engine credit.
            if endpoint.queue.len() == 4 || bytes > 4096 || endpoint.bytes + bytes > 8192 {
                endpoint.loss = endpoint.loss.saturating_add(1);
            } else {
                let response = if endpoint.responses {
                    bus.borrow_mut().physical.get_mut(&key).unwrap().jobs += 1;
                    Some(Reply {
                        bus: bus.clone(),
                        physical: key,
                        attempted: false,
                        sent: false,
                        released: false,
                        event_body: data.as_ref().map(|p| p.body.clone()),
                    })
                } else {
                    None
                };
                let recipient = endpoint.identity.clone();
                endpoint.queue.push_back(NativeNotification {
                    recipient,
                    data: data.clone(),
                    response,
                });
                endpoint.bytes += bytes;
            }
            if let Some(wake) = endpoint.wake.take() {
                wake.wake();
            }
        }
    }
}
struct Reply {
    bus: Rc<RefCell<Bus>>,
    physical: (&'static str, u8),
    attempted: bool,
    sent: bool,
    released: bool,
    event_body: Option<Arc<[u8]>>,
}
impl Reply {
    fn release(&mut self) {
        if !self.released {
            let mut bus = self.bus.borrow_mut();
            bus.physical.get_mut(&self.physical).unwrap().jobs -= 1;
            bus.retire_physical(self.physical);
            self.released = true;
            self.event_body.take();
        }
    }
}
impl NotificationResponse for Reply {
    fn poll_deliver(
        &mut self,
        _: &mut Context<'_>,
        input: &EventDataResponse,
    ) -> Poll<CoreResult<()>> {
        self.attempted = true;
        let hook = self.bus.borrow_mut().on_send.take();
        if let Some(hook) = hook {
            hook();
        }
        let mut bus = self.bus.borrow_mut();
        bus.reply_send_polls += 1;
        bus.response_inputs
            .push(input.0.as_ref().map_or(vec![], |p| p.body.to_vec()));
        bus.response_bodies
            .push(input.0.as_ref().map_or(0, |p| p.body.as_ptr() as usize));
        match bus.send {
            Mode::Pending => Poll::Pending,
            Mode::Fail => Poll::Ready(Err(binding_error())),
            Mode::Ready => {
                self.sent = true;
                Poll::Ready(Ok(()))
            }
        }
    }
    fn poll_retire(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<EventResponseDelivery>> {
        let mode = {
            let mut bus = self.bus.borrow_mut();
            bus.reply_retire_polls += 1;
            bus.retire
        };
        match mode {
            Mode::Pending => Poll::Pending,
            Mode::Fail => Poll::Ready(Err(cleanup_error())),
            Mode::Ready => {
                self.release();
                Poll::Ready(Ok(
                    if self.sent || (self.attempted && self.bus.borrow().late_response_delivered) {
                        EventResponseDelivery::Delivered
                    } else if self.attempted {
                        EventResponseDelivery::Unknown
                    } else {
                        EventResponseDelivery::NotAttempted
                    },
                ))
            }
        }
    }
    fn abort_local(mut self) {
        self.bus.borrow_mut().reply_aborts += 1;
        self.release();
    }
}

struct Participation {
    bus: Rc<RefCell<Bus>>,
    endpoint: Rc<RefCell<Endpoint>>,
    program: Program,
    began: bool,
}
impl Participation {
    fn release(&mut self) {
        let mut endpoint = self.endpoint.borrow_mut();
        if endpoint.joined {
            endpoint.joined = false;
            let mut bus = self.bus.borrow_mut();
            bus.physical
                .get_mut(&endpoint.physical)
                .unwrap()
                .participants -= 1;
            bus.retire_physical(endpoint.physical);
        }
        for mut item in endpoint.queue.drain(..) {
            if let Some(reply) = item.response.take() {
                reply.abort_local();
            }
        }
        endpoint.bytes = 0;
    }
}
impl NotificationParticipation for Participation {
    type Response = Reply;
    fn poll_start(
        &mut self,
        _: &mut Context<'_>,
        input: &SubscriptionInput,
    ) -> Poll<CoreResult<()>> {
        self.bus.borrow_mut().start_polls += 1;
        if !self.began {
            self.began = true;
            let key = self.program.physical();
            let mut bus = self.bus.borrow_mut();
            // One actual response opportunity cannot be shared among logical
            // recipients. This Binding refuses that topology, without an Engine
            // unique-replier or aggregation service.
            if self.program.responses && bus.physical.get(&key).is_some_and(|p| p.participants > 0)
            {
                return Poll::Ready(Err(binding_error()));
            }
            let first = !bus.physical.contains_key(&key);
            bus.physical.entry(key).or_default().participants += 1;
            if first {
                bus.subscriptions += 1;
            }
            self.endpoint.borrow_mut().joined = true;
            bus.control_inputs
                .push(input.0.as_ref().map_or(vec![], |p| p.body.to_vec()));
        }
        match self.program.start {
            Mode::Ready => Poll::Ready(Ok(())),
            Mode::Pending => Poll::Pending,
            Mode::Fail => Poll::Ready(Err(binding_error())),
        }
    }
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        max_bytes: usize,
    ) -> Poll<CoreResult<NotificationSignal<Reply>>> {
        self.bus.borrow_mut().receive_polls += 1;
        let mut endpoint = self.endpoint.borrow_mut();
        // Single-threaded register-then-recheck, with bounded wake storage.
        endpoint.wake = Some(cx.waker().clone());
        if endpoint.loss > 0 {
            let loss = std::mem::take(&mut endpoint.loss);
            return Poll::Ready(Ok(NotificationSignal::Loss(loss)));
        }
        let Some(mut item) = endpoint.queue.pop_front() else {
            return Poll::Pending;
        };
        let bytes = notification_payload_backing(item.data.as_ref()).unwrap();
        endpoint.bytes -= bytes;
        drop(endpoint);
        if bytes > max_bytes {
            if let Some(reply) = item.response.take() {
                reply.abort_local();
            }
            return Poll::Ready(Ok(NotificationSignal::Loss(1)));
        }
        let hook = self.bus.borrow_mut().on_next.take();
        if let Some(hook) = hook {
            hook();
        }
        if self.bus.borrow().receive_error {
            // Operational failure retains the complete original native input.
            let mut endpoint = self.endpoint.borrow_mut();
            endpoint.queue.push_front(item);
            endpoint.bytes += bytes;
            return Poll::Ready(Err(binding_error()));
        }
        Poll::Ready(Ok(NotificationSignal::Data(item)))
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        input: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>> {
        let mode = {
            let mut bus = self.bus.borrow_mut();
            bus.close_polls += 1;
            bus.close
        };
        match mode {
            Mode::Pending => Poll::Pending,
            Mode::Fail => Poll::Ready(Err(cleanup_error())),
            Mode::Ready => {
                self.bus.borrow_mut().completed_closes += 1;
                if let Some(input) = input {
                    self.bus
                        .borrow_mut()
                        .control_inputs
                        .push(input.0.as_ref().map_or(vec![], |p| p.body.to_vec()));
                }
                self.release();
                Poll::Ready(Ok(if !self.program.cancellation {
                    RemoteCancellation::Unsupported
                } else if input.is_some() {
                    RemoteCancellation::Confirmed
                } else {
                    RemoteCancellation::Unknown
                }))
            }
        }
    }
    fn abort_local(mut self) {
        self.bus.borrow_mut().aborts += 1;
        self.release();
    }
}

struct Binding {
    bus: Rc<RefCell<Bus>>,
    configuration: Rc<Cell<u64>>,
}
impl ConsumerNotificationBinding for Binding {
    type Program = Program;
    type Configuration = Cell<u64>;
    type Participation = Participation;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(7), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &Cell<u64> {
        &self.configuration
    }
    fn support(&self, program: &Program, _: NotificationKind) -> CoreResult<NotificationSupport> {
        Ok(NotificationSupport {
            event_responses: program.responses,
            cancellation: program.cancellation,
        })
    }
    fn prepare(
        &mut self,
        program: Program,
        identity: &NotificationIdentity,
    ) -> Result<Participation, BindingInputRejection<Program>> {
        let mut bus = self.bus.borrow_mut();
        // Bound local private construction, independent of Engine item credit.
        bus.endpoints.retain(|e| e.strong_count() > 0);
        if bus.endpoints.len() == 16 {
            return Err(BindingInputRejection::new(
                program,
                BindingOperationalError::new(binding_error()),
            ));
        }
        bus.prepare_calls += 1;
        let endpoint = Rc::new(RefCell::new(Endpoint {
            identity: identity.clone(),
            physical: program.physical(),
            responses: program.responses,
            joined: false,
            queue: VecDeque::with_capacity(4),
            bytes: 0,
            loss: 0,
            wake: None,
        }));
        bus.endpoints.push(Rc::downgrade(&endpoint));
        Ok(Participation {
            bus: self.bus.clone(),
            endpoint,
            program,
            began: false,
        })
    }
}
#[derive(Clone)]
struct Policy {
    denied: Rc<Cell<u8>>,
    probes: Rc<Cell<usize>>,
    on_deliver: PolicyHook,
}
type PolicyHook = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;
impl Policy {
    fn new() -> Self {
        Self {
            denied: Rc::new(Cell::new(0)),
            probes: Rc::new(Cell::new(0)),
            on_deliver: Rc::new(RefCell::new(None)),
        }
    }
}
fn bit(p: NotificationPermission) -> u8 {
    match p {
        NotificationPermission::Establish => 1,
        NotificationPermission::Deliver => 2,
        NotificationPermission::Respond => 4,
        NotificationPermission::Cancel => 8,
    }
}
impl NotificationAuthorizer for Policy {
    fn authorize(
        &self,
        _: &NotificationIdentity,
        _: &AffordanceTarget,
        p: NotificationPermission,
    ) -> CoreResult<()> {
        self.probes.set(self.probes.get() + 1);
        if p == NotificationPermission::Deliver {
            let hook = self.on_deliver.borrow_mut().take();
            if let Some(hook) = hook {
                hook();
            }
        }
        if self.denied.get() & bit(p) == 0 {
            Ok(())
        } else {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never),
            })
        }
    }
}
type Root = RawNotificationServient<Binding, Policy>;
fn binding_error() -> CoreError {
    CoreError::Binding(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
fn cleanup_error() -> CoreError {
    CoreError::Cleanup(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never))
}
fn budget() -> WorkBudget {
    let mut budget = WorkBudget::new();
    for class in WorkClass::ALL {
        budget.set_remaining(class, 1024);
    }
    budget
}
fn limits() -> RawNotificationLimits {
    RawNotificationLimits {
        subscriptions: 2,
        queue_items: 2,
        queue_payload_bytes: 4096,
        payload_backing_bytes: 1024,
        control_backing_bytes: 1024,
        target_name_bytes: 64,
        engine_bytes: 64 * 1024,
    }
}
fn root(bus: &Rc<RefCell<Bus>>, limits: RawNotificationLimits) -> (Root, Policy, Rc<Cell<u64>>) {
    let policy = Policy::new();
    let configuration = Rc::new(Cell::new(1));
    let root = Root::try_new(
        Binding {
            bus: bus.clone(),
            configuration: configuration.clone(),
        },
        Cell::new(1),
        policy.clone(),
        ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL),
        limits,
    )
    .unwrap_or_else(|_| panic!("root capacity"));
    (root, policy, configuration)
}
fn payload(s: &str) -> Payload {
    Payload::new(s.as_bytes(), "application/test")
}
fn property(overflow: NotificationOverflow) -> RawNotificationRequest {
    RawNotificationRequest {
        target: AffordanceTarget::Property(Arc::from("temperature")),
        subscription: SubscriptionInput(None),
        responses: ResponseRequirement::None,
        overflow,
    }
}
fn event(required: bool) -> RawNotificationRequest {
    RawNotificationRequest {
        target: AffordanceTarget::Event(Arc::from("alarm")),
        subscription: SubscriptionInput(Some(payload("filter"))),
        responses: if required {
            ResponseRequirement::Required
        } else {
            ResponseRequirement::None
        },
        overflow: NotificationOverflow::Terminate,
    }
}
fn subscribe(
    root: &mut Root,
    request: RawNotificationRequest,
    program: Program,
) -> NotificationCursor {
    root.subscribe(request, program, &mut budget())
        .unwrap_or_else(|_| panic!("subscribe rejected"))
}
fn step(root: &mut Root, id: &NotificationIdentity) -> NotificationState {
    root.progress(id, &mut Context::from_waker(Waker::noop()), &mut budget())
        .unwrap()
}
fn settle(root: &mut Root, id: &NotificationIdentity) -> NotificationSettlement {
    for _ in 0..32 {
        if root.status(id).unwrap().state == NotificationState::Closed {
            return root.take_settlement(id).unwrap();
        }
        step(root, id);
    }
    panic!("cleanup did not converge")
}
fn data(root: &mut Root, cursor: &mut NotificationCursor) -> Vec<u8> {
    root.front(cursor, &mut budget())
        .unwrap()
        .unwrap()
        .data
        .unwrap()
        .body
        .to_vec()
}

#[test]
fn property_observation_has_one_cursor_and_no_fabricated_initial_value() {
    let bus = Bus::new();
    Bus::publish(&bus, ("sensor", 1), Some(payload("no subscribers")));
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    assert_eq!(step(&mut root, &id), NotificationState::Active);
    step(&mut root, &id);
    assert!(root.front(&mut cursor, &mut budget()).unwrap().is_none());
    Bus::publish(&bus, ("sensor", 1), Some(payload("21")));
    step(&mut root, &id);
    assert_eq!(data(&mut root, &mut cursor), b"21");
    assert_eq!(data(&mut root, &mut cursor), b"21"); // Same cursor, same retained value.
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .sequence,
        0
    );
    root.advance(&mut cursor, &mut budget()).unwrap();
    assert!(root.front(&mut cursor, &mut budget()).unwrap().is_none());
    assert_eq!(bus.borrow().reply_send_polls, 0);
    root.stop(&cursor, None, &mut budget())
        .unwrap_or_else(|_| panic!());
    let report = settle(&mut root, &id);
    assert_eq!(report.delivered_items, 1);
    assert_eq!(report.remote_cancellation, RemoteCancellation::Unknown);
}

#[test]
fn binding_shares_physical_subscription_across_consumers_with_independent_stop_and_idle_retirement()
{
    let bus = Bus::new();
    bus.borrow_mut().retain_idle = true;
    let (mut a, _, _) = root(&bus, limits());
    let (mut b, _, _) = root(&bus, limits());
    let mut ca = subscribe(
        &mut a,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let mut cb = subscribe(
        &mut b,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let ia = ca.identity().clone();
    let ib = cb.identity().clone();
    assert_ne!(ia, ib); // Same numeric consumer/subscription coordinates, different issuers.
    step(&mut a, &ia);
    step(&mut b, &ib);
    assert_eq!(bus.borrow().subscriptions, 1);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].participants, 2);
    assert!(b.front(&mut ca, &mut budget()).is_err());
    Bus::publish(&bus, ("sensor", 1), Some(payload("shared")));
    step(&mut a, &ia);
    step(&mut b, &ib);
    let pa = a
        .front(&mut ca, &mut budget())
        .unwrap()
        .unwrap()
        .data
        .unwrap()
        .body
        .as_ptr();
    let pb = b
        .front(&mut cb, &mut budget())
        .unwrap()
        .unwrap()
        .data
        .unwrap()
        .body
        .as_ptr();
    assert_eq!(pa, pb); // Actual immutable body sharing; independent cursor records.
    a.stop(&ca, Some(CancellationInput(None)), &mut budget())
        .unwrap_or_else(|_| panic!());
    settle(&mut a, &ia);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].participants, 1);
    b.advance(&mut cb, &mut budget()).unwrap();
    Bus::publish(&bus, ("sensor", 1), Some(payload("sibling")));
    step(&mut b, &ib);
    assert_eq!(data(&mut b, &mut cb), b"sibling");
    b.stop(&cb, None, &mut budget())
        .unwrap_or_else(|_| panic!());
    settle(&mut b, &ib);
    assert_eq!(bus.borrow().physical.len(), 1); // Last logical close is not shutdown.
    bus.borrow_mut().shutdown_idle();
    assert!(bus.borrow().physical.is_empty());
}

#[test]
fn sharing_never_crosses_binding_tenant_compatibility() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut a = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let mut b = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program {
            tenant: 2,
            ..Program::property()
        },
    );
    step(&mut root, a.identity());
    step(&mut root, b.identity());
    Bus::publish(&bus, ("sensor", 1), Some(payload("tenant1")));
    step(&mut root, a.identity());
    step(&mut root, b.identity());
    assert_eq!(data(&mut root, &mut a), b"tenant1");
    assert!(root.front(&mut b, &mut budget()).unwrap().is_none());
    assert_eq!(bus.borrow().subscriptions, 2);
}

#[test]
fn unsupported_required_response_and_denied_establishment_have_no_native_effects() {
    let bus = Bus::new();
    let (mut root, policy, _) = root(&bus, limits());
    let request = event(true);
    let original = request.subscription.0.as_ref().unwrap().body.clone();
    let rejection = root
        .subscribe(request, Program::property(), &mut budget())
        .err()
        .unwrap();
    assert!(matches!(
        rejection.error().error(),
        CoreError::UnsupportedOperation(_)
    ));
    assert!(Arc::ptr_eq(
        &original,
        &rejection.input().0.subscription.0.as_ref().unwrap().body
    ));
    policy.denied.set(bit(NotificationPermission::Establish));
    assert!(
        root.subscribe(
            property(NotificationOverflow::Terminate),
            Program::property(),
            &mut budget()
        )
        .is_err()
    );
    assert_eq!(bus.borrow().prepare_calls, 0);
    assert_eq!(bus.borrow().start_polls, 0);
}

#[test]
fn stop_before_native_start_and_cursor_drop_retain_prepaid_cleanup_and_capacity() {
    let bus = Bus::new();
    let (mut root, _, _) = root(
        &bus,
        RawNotificationLimits {
            subscriptions: 1,
            ..limits()
        },
    );
    let baseline = root.reserved_engine_bytes();
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    let reservation = root.reserved_engine_bytes();
    assert!(reservation > baseline);
    drop(cursor);
    assert_eq!(root.status(&id).unwrap().state, NotificationState::Closing);
    assert!(
        root.progress(
            &id,
            &mut Context::from_waker(Waker::noop()),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    assert_eq!(root.reserved_engine_bytes(), reservation);
    assert!(
        root.subscribe(
            property(NotificationOverflow::Terminate),
            Program::property(),
            &mut budget()
        )
        .is_err()
    );
    step(&mut root, &id);
    assert_eq!(root.reserved_engine_bytes(), reservation); // Terminal report still owns the slot.
    let report = root.take_settlement(&id).unwrap();
    assert!(!report.native_started);
    assert_eq!(report.remote_cancellation, RemoteCancellation::NotAttempted);
    assert_eq!(root.reserved_engine_bytes(), baseline);
    assert_eq!(bus.borrow().start_polls, 0);
    let replacement = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    assert_ne!(replacement.identity(), &id);
    assert!(root.status(&id).is_err());
}

#[test]
fn event_control_data_response_and_delivery_have_distinct_owners() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(true), Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("alarm data")));
    step(&mut root, &id);
    assert_eq!(data(&mut root, &mut cursor), b"alarm data");
    assert_eq!(bus.borrow().native_acks, 1);
    assert_eq!(bus.borrow().reply_send_polls, 0);
    assert!(root.advance(&mut cursor, &mut budget()).is_err());
    let response = payload("consumer response");
    let body = response.body.as_ptr() as usize;
    root.respond(
        &mut cursor,
        EventDataResponse(Some(response)),
        &mut budget(),
    )
    .unwrap_or_else(|_| panic!());
    bus.borrow_mut().send = Mode::Pending;
    step(&mut root, &id);
    assert!(root.advance(&mut cursor, &mut budget()).is_err());
    bus.borrow_mut().send = Mode::Ready;
    step(&mut root, &id);
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Delivered
    );
    step(&mut root, &id);
    root.advance(&mut cursor, &mut budget()).unwrap();
    assert_eq!(bus.borrow().response_bodies, vec![body, body]);
    root.stop(
        &cursor,
        Some(CancellationInput(Some(payload("remove webhook")))),
        &mut budget(),
    )
    .unwrap_or_else(|_| panic!());
    let report = settle(&mut root, &id);
    assert_eq!(report.responses_delivered, 1);
    assert_eq!(report.responses_uncertain, 0);
    assert_eq!(report.remote_cancellation, RemoteCancellation::Confirmed);
    assert_eq!(
        bus.borrow().control_inputs,
        vec![b"filter".to_vec(), b"remove webhook".to_vec()]
    );
    assert!(
        bus.borrow()
            .response_inputs
            .iter()
            .all(|v| v == b"consumer response")
    );
}

#[test]
fn empty_event_data_is_supported_without_fabricating_a_property_value_or_response() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(false), Program::property());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), None);
    step(&mut root, &id);
    let view = root.front(&mut cursor, &mut budget()).unwrap().unwrap();
    assert!(view.data.is_none());
    assert_eq!(view.response, EventResponseState::Unavailable);
    assert!(root.decline_response(&mut cursor, &mut budget()).is_err());
    assert!(
        root.respond(&mut cursor, EventDataResponse(None), &mut budget())
            .is_err()
    );
    root.advance(&mut cursor, &mut budget()).unwrap();
}

#[test]
fn required_event_reply_cannot_be_broadcast_or_synthesized() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let a = subscribe(&mut root, event(true), Program::event());
    let ia = a.identity().clone();
    step(&mut root, &ia);
    let b = subscribe(&mut root, event(true), Program::event());
    let ib = b.identity().clone();
    assert_eq!(step(&mut root, &ib), NotificationState::Closing);
    settle(&mut root, &ib);
    assert_eq!(root.status(&ia).unwrap().state, NotificationState::Active);
    Bus::publish(&bus, ("sensor", 1), Some(payload("one reply")));
    // Corrupt the test Binding's advertised guarantee: Engine refuses the data,
    // retaining its original input rather than inventing an opportunity.
    let endpoint = bus.borrow().endpoints[0].upgrade().unwrap();
    endpoint
        .borrow_mut()
        .queue
        .front_mut()
        .unwrap()
        .response
        .take()
        .unwrap()
        .abort_local();
    assert_eq!(step(&mut root, &ia), NotificationState::Closing);
    assert!(matches!(
        root.status(&ia).unwrap().first_cause,
        Some(CoreError::Payload(_))
    ));
}

#[test]
fn late_delivery_after_stop_is_owned_but_never_exposed() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(true), Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    let p = payload("late native data");
    let weak = Arc::downgrade(&p.body);
    Bus::publish(&bus, ("sensor", 1), Some(p));
    let stop = cursor.stop_handle();
    bus.borrow_mut().on_next = Some(Box::new(move || stop.request_stop()));
    assert_eq!(step(&mut root, &id), NotificationState::Closing);
    assert!(root.front(&mut cursor, &mut budget()).is_err());
    assert!(root.status(&id).unwrap().has_retained_handoff);
    assert!(weak.upgrade().is_some());
    bus.borrow_mut().retire = Mode::Pending;
    step(&mut root, &id);
    assert!(weak.upgrade().is_some());
    assert!(root.take_settlement(&id).is_err());
    bus.borrow_mut().retire = Mode::Ready;
    let report = settle(&mut root, &id);
    assert_eq!(report.delivered_items, 0);
    assert_eq!(report.responses_delivered, 0);
    assert!(weak.upgrade().is_none());
    assert!(bus.borrow().physical.is_empty());
}

#[test]
fn policy_revocation_during_native_progress_fences_each_original_recipient() {
    let bus = Bus::new();
    let (mut root, policy, _) = root(&bus, limits());
    let mut cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("denied late data")));
    bus.borrow_mut().on_next = Some(Box::new(move || {
        policy.denied.set(bit(NotificationPermission::Deliver))
    }));
    assert_eq!(step(&mut root, &id), NotificationState::Closing);
    assert!(root.front(&mut cursor, &mut budget()).is_err());
    let report = settle(&mut root, &id);
    assert!(matches!(
        report.first_cause,
        Some(CoreError::Security { .. })
    ));
    assert_eq!(report.delivered_items, 0);
}

#[test]
fn full_queue_drop_policy_reports_event_loss_without_replacing_unread_data() {
    let bus = Bus::new();
    let (mut root, _, _) = root(
        &bus,
        RawNotificationLimits {
            queue_items: 1,
            ..limits()
        },
    );
    let mut request = event(false);
    request.overflow = NotificationOverflow::DropNewest;
    let mut cursor = subscribe(&mut root, request, Program::property());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("first event")));
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("second event")));
    step(&mut root, &id);
    assert_eq!(root.status(&id).unwrap().loss.items, 1);
    assert_eq!(root.status(&id).unwrap().state, NotificationState::Active);
    assert_eq!(data(&mut root, &mut cursor), b"first event");
    root.advance(&mut cursor, &mut budget()).unwrap();
    Bus::publish(&bus, ("sensor", 1), Some(payload("third event")));
    step(&mut root, &id);
    assert_eq!(data(&mut root, &mut cursor), b"third event");
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .sequence,
        1
    );
}

#[test]
fn byte_overflow_terminates_and_cleanup_survives_saturation_and_failure() {
    let p = payload("bounded data");
    let bytes = notification_payload_backing(Some(&p)).unwrap();
    let bus = Bus::new();
    let (mut root, _, _) = root(
        &bus,
        RawNotificationLimits {
            subscriptions: 1,
            queue_payload_bytes: bytes,
            ..limits()
        },
    );
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(p));
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("overflow data")));
    assert_eq!(step(&mut root, &id), NotificationState::Closing);
    assert_eq!(root.status(&id).unwrap().loss.items, 1);
    assert!(root.status(&id).unwrap().has_retained_handoff);
    let reservation = root.reserved_engine_bytes();
    bus.borrow_mut().close = Mode::Fail;
    step(&mut root, &id);
    step(&mut root, &id);
    step(&mut root, &id);
    assert_eq!(root.reserved_engine_bytes(), reservation);
    assert!(matches!(
        root.status(&id).unwrap().first_cause,
        Some(CoreError::Backpressure(_))
    ));
    assert!(matches!(
        root.status(&id).unwrap().cleanup_error,
        Some(CoreError::Cleanup(_))
    ));
    assert!(root.take_settlement(&id).is_err());
    assert!(
        root.subscribe(
            property(NotificationOverflow::Terminate),
            Program::property(),
            &mut budget()
        )
        .is_err()
    );
    bus.borrow_mut().close = Mode::Ready;
    let report = settle(&mut root, &id);
    assert!(report.cleanup_error.is_some());
    assert_eq!(report.loss.items, 1);
}

#[test]
fn response_overflow_keeps_all_reply_jobs_even_under_drop_newest_policy() {
    let bus = Bus::new();
    let (mut root, _, _) = root(
        &bus,
        RawNotificationLimits {
            queue_items: 1,
            ..limits()
        },
    );
    let mut request = event(true);
    request.overflow = NotificationOverflow::DropNewest;
    let cursor = subscribe(&mut root, request, Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("one")));
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("two")));
    step(&mut root, &id);
    assert_eq!(root.status(&id).unwrap().state, NotificationState::Closing);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 2);
    bus.borrow_mut().retire = Mode::Fail;
    step(&mut root, &id);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 2);
    bus.borrow_mut().retire = Mode::Ready;
    settle(&mut root, &id);
    assert!(bus.borrow().physical.is_empty());
    assert_eq!(bus.borrow().reply_aborts, 2);
}

#[test]
fn native_loss_has_bounded_out_of_band_saturating_status() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::DropNewest),
        Program::property(),
    );
    let id = cursor.identity().clone();
    step(&mut root, &id);
    let endpoint = bus.borrow().endpoints[0].upgrade().unwrap();
    endpoint.borrow_mut().loss = u64::MAX;
    step(&mut root, &id);
    endpoint.borrow_mut().loss = 1;
    step(&mut root, &id);
    assert_eq!(
        root.status(&id).unwrap().loss,
        NotificationLoss {
            items: u64::MAX,
            saturated: true
        }
    );
    for _ in 0..5 {
        Bus::publish(&bus, ("sensor", 1), Some(payload("native overflow")));
    }
    assert_eq!(endpoint.borrow().queue.len(), 4);
    assert_eq!(endpoint.borrow().loss, 1);
    step(&mut root, &id);
    assert_eq!(root.status(&id).unwrap().state, NotificationState::Active);
    root.stop(&cursor, None, &mut budget())
        .unwrap_or_else(|_| panic!());
    settle(&mut root, &id);
}

#[test]
fn stop_is_independent_of_remote_cancellation_support_and_permission() {
    for denied in [false, true] {
        let bus = Bus::new();
        let (mut root, policy, _) = root(&bus, limits());
        let mut program = Program::property();
        program.cancellation = denied;
        let mut cursor = subscribe(&mut root, event(false), program);
        let id = cursor.identity().clone();
        step(&mut root, &id);
        if denied {
            policy.denied.set(bit(NotificationPermission::Cancel));
        }
        let rejection = root
            .stop(
                &cursor,
                Some(CancellationInput(Some(payload("cancel input")))),
                &mut budget(),
            )
            .err()
            .unwrap();
        assert_eq!(
            rejection
                .input()
                .as_ref()
                .unwrap()
                .0
                .as_ref()
                .unwrap()
                .body
                .as_ref(),
            b"cancel input"
        );
        assert!(root.front(&mut cursor, &mut budget()).is_err());
        let report = settle(&mut root, &id);
        assert!(report.cancellation_error.is_some());
        assert_eq!(
            report.remote_cancellation,
            if denied {
                RemoteCancellation::Unknown
            } else {
                RemoteCancellation::Unsupported
            }
        );
    }
}

#[test]
fn cancellation_is_reauthorized_at_native_control_acceptance() {
    let bus = Bus::new();
    let (mut root, policy, _) = root(&bus, limits());
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    step(&mut root, &id);
    root.stop(&cursor, Some(CancellationInput(None)), &mut budget())
        .unwrap_or_else(|_| panic!());
    policy.denied.set(bit(NotificationPermission::Cancel));
    let report = settle(&mut root, &id);
    assert!(report.cancellation_error.is_some());
    assert_eq!(report.remote_cancellation, RemoteCancellation::Unknown);
    assert_eq!(bus.borrow().control_inputs, vec![Vec::<u8>::new()]);
}

#[test]
fn failed_response_retains_original_bytes_and_reports_uncertain_delivery_after_stop() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(true), Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("alarm")));
    step(&mut root, &id);
    data(&mut root, &mut cursor);
    let p = payload("original response");
    let weak = Arc::downgrade(&p.body);
    root.respond(&mut cursor, EventDataResponse(Some(p)), &mut budget())
        .unwrap_or_else(|_| panic!());
    bus.borrow_mut().send = Mode::Fail;
    step(&mut root, &id);
    assert!(weak.upgrade().is_some());
    bus.borrow_mut().retire = Mode::Pending;
    step(&mut root, &id);
    assert!(weak.upgrade().is_some());
    bus.borrow_mut().retire = Mode::Ready;
    let report = settle(&mut root, &id);
    assert!(matches!(report.first_cause, Some(CoreError::Binding(_))));
    assert_eq!(report.responses_uncertain, 1);
    assert_eq!(report.responses_delivered, 0);
    assert!(weak.upgrade().is_none());
}

#[test]
fn stale_native_delivery_cannot_retarget_reused_logical_slot() {
    let bus = Bus::new();
    let (mut root, _, _) = root(
        &bus,
        RawNotificationLimits {
            subscriptions: 1,
            ..limits()
        },
    );
    let old = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let old_id = old.identity().clone();
    drop(old);
    settle(&mut root, &old_id);
    let mut new = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = new.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("old callback")));
    let endpoint = bus.borrow().endpoints.last().unwrap().upgrade().unwrap();
    endpoint.borrow_mut().queue.front_mut().unwrap().recipient = old_id;
    step(&mut root, &id);
    assert!(root.front(&mut new, &mut budget()).is_err());
    let report = settle(&mut root, &id);
    assert!(matches!(
        report.first_cause,
        Some(CoreError::StaleHandle(_))
    ));
}

#[test]
fn mutable_configuration_and_consumer_revocation_fence_original_work() {
    for revoke in [false, true] {
        let bus = Bus::new();
        let (mut root, _, configuration) = root(&bus, limits());
        let mut cursor = subscribe(
            &mut root,
            property(NotificationOverflow::Terminate),
            Program::property(),
        );
        let id = cursor.identity().clone();
        step(&mut root, &id);
        Bus::publish(&bus, ("sensor", 1), Some(payload("original owner")));
        step(&mut root, &id);
        if revoke {
            root.revoke();
        } else {
            configuration.set(2);
        }
        assert!(root.front(&mut cursor, &mut budget()).is_err());
        let report = settle(&mut root, &id);
        assert_eq!(report.delivered_items, 0);
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
    }
}

#[test]
fn stalled_establishment_does_not_block_sibling_and_partial_start_still_needs_cleanup() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let slow = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program {
            start: Mode::Pending,
            ..Program::property()
        },
    );
    let mut fast = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    assert_eq!(
        step(&mut root, slow.identity()),
        NotificationState::Establishing
    );
    step(&mut root, fast.identity());
    Bus::publish(&bus, ("sensor", 1), Some(payload("live sibling")));
    step(&mut root, fast.identity());
    assert_eq!(data(&mut root, &mut fast), b"live sibling");
    let id = slow.identity().clone();
    drop(slow);
    bus.borrow_mut().close = Mode::Pending;
    assert_eq!(step(&mut root, &id), NotificationState::Closing);
    assert!(root.status(&id).unwrap().native_started);
    bus.borrow_mut().close = Mode::Ready;
    settle(&mut root, &id);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].participants, 1);
}

#[test]
fn zero_work_and_backing_limits_refuse_before_native_acquisition() {
    let bus = Bus::new();
    let (mut root, policy, _) = root(&bus, limits());
    assert!(
        root.subscribe(
            property(NotificationOverflow::Terminate),
            Program::property(),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    assert_eq!(policy.probes.get(), 0);
    assert_eq!(bus.borrow().prepare_calls, 0);
    let mut request = event(false);
    let mut p = payload("x");
    p.content_type.reserve_exact(2048);
    request.subscription = SubscriptionInput(Some(p));
    assert!(
        root.subscribe(request, Program::property(), &mut budget())
            .is_err()
    );
    assert_eq!(bus.borrow().prepare_calls, 0);
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    assert!(
        root.progress(
            &id,
            &mut Context::from_waker(Waker::noop()),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    assert_eq!(bus.borrow().start_polls, 0);
    step(&mut root, &id);
    let mut short = WorkBudget::new()
        .with_remaining(WorkClass::ProviderProbes, 1)
        .with_remaining(WorkClass::BindingPolls, 1)
        .with_remaining(WorkClass::QueueOperations, 1);
    assert!(
        root.progress(&id, &mut Context::from_waker(Waker::noop()), &mut short)
            .is_err()
    );
    assert_eq!(bus.borrow().receive_polls, 0);
    assert_eq!(root.status(&id).unwrap().state, NotificationState::Active);
}

#[test]
fn host_root_drop_aborts_complete_local_owners_without_claiming_remote_drain() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(true), Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    let p = payload("held native body");
    let weak = Arc::downgrade(&p.body);
    Bus::publish(&bus, ("sensor", 1), Some(p));
    step(&mut root, &id);
    data(&mut root, &mut cursor);
    root.respond(
        &mut cursor,
        EventDataResponse(Some(payload("pending response"))),
        &mut budget(),
    )
    .unwrap_or_else(|_| panic!());
    bus.borrow_mut().send = Mode::Pending;
    bus.borrow_mut().close = Mode::Pending;
    bus.borrow_mut().retire = Mode::Pending;
    step(&mut root, &id);
    drop(root);
    assert!(weak.upgrade().is_none());
    assert!(bus.borrow().physical.is_empty());
    assert_eq!(bus.borrow().completed_closes, 0);
    assert_eq!(bus.borrow().close_polls, 0);
    assert_eq!(bus.borrow().aborts, 1);
    assert_eq!(bus.borrow().reply_aborts, 1);
}

#[test]
fn owner_cleanup_progresses_fairly_after_every_cursor_and_identity_is_lost() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let slow = subscribe(
        &mut root,
        event(true),
        Program {
            source: "events",
            ..Program::event()
        },
    );
    let fast = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    step(&mut root, slow.identity());
    step(&mut root, fast.identity());
    Bus::publish(&bus, ("events", 1), Some(payload("retained response job")));
    step(&mut root, slow.identity());
    drop(slow);
    drop(fast); // No retained application identity/cursor.
    bus.borrow_mut().retire = Mode::Pending;
    let mut reports = vec![];
    for _ in 0..8 {
        if let Some(report) = root
            .step_cleanup(&mut Context::from_waker(Waker::noop()), &mut budget())
            .unwrap()
        {
            reports.push(report);
        }
    }
    assert_eq!(reports.len(), 1);
    assert!(matches!(reports[0].target, AffordanceTarget::Property(_)));
    assert_eq!(root.retained_subscriptions(), 1);
    assert_eq!(bus.borrow().physical.len(), 1);
    bus.borrow_mut().retire = Mode::Fail;
    for _ in 0..2 {
        root.step_cleanup(&mut Context::from_waker(Waker::noop()), &mut budget())
            .unwrap();
    }
    bus.borrow_mut().retire = Mode::Ready;
    for _ in 0..12 {
        if let Some(report) = root
            .step_cleanup(&mut Context::from_waker(Waker::noop()), &mut budget())
            .unwrap()
        {
            reports.push(report);
        }
    }
    assert_eq!(reports.len(), 2);
    assert!(reports[1].cleanup_error.is_some());
    assert_eq!(root.retained_subscriptions(), 0);
    assert!(bus.borrow().physical.is_empty());
}

#[test]
fn whole_owner_close_fences_live_cursors_without_native_shutdown_authority() {
    let bus = Bus::new();
    bus.borrow_mut().retain_idle = true;
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    step(&mut root, cursor.identity());
    root.begin_close();
    assert!(root.front(&mut cursor, &mut budget()).is_err());
    assert!(
        root.subscribe(
            property(NotificationOverflow::Terminate),
            Program::property(),
            &mut budget()
        )
        .is_err()
    );
    for _ in 0..4 {
        root.step_cleanup(&mut Context::from_waker(Waker::noop()), &mut budget())
            .unwrap();
    }
    assert_eq!(root.retained_subscriptions(), 0);
    assert_eq!(bus.borrow().physical.len(), 1);
    bus.borrow_mut().shutdown_idle();
    assert!(bus.borrow().physical.is_empty());
}

#[test]
fn stop_during_native_error_wins_first_cause_and_never_becomes_completed_cleanup() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), Some(payload("late error input")));
    let stop = cursor.stop_handle();
    bus.borrow_mut().on_next = Some(Box::new(move || stop.request_stop()));
    bus.borrow_mut().receive_error = true;
    step(&mut root, &id);
    assert!(matches!(
        root.status(&id).unwrap().first_cause,
        Some(CoreError::Cancelled(_))
    ));
    bus.borrow_mut().close = Mode::Pending;
    step(&mut root, &id);
    assert!(root.take_settlement(&id).is_err());
    assert_eq!(bus.borrow().completed_closes, 0);
    bus.borrow_mut().close = Mode::Ready;
    let report = settle(&mut root, &id);
    assert!(matches!(report.first_cause, Some(CoreError::Cancelled(_))));
}

#[test]
fn late_response_success_is_preserved_independently_of_local_stop() {
    for delivered_in_retirement in [false, true] {
        let bus = Bus::new();
        let (mut root, _, _) = root(&bus, limits());
        let mut cursor = subscribe(&mut root, event(true), Program::event());
        let id = cursor.identity().clone();
        step(&mut root, &id);
        Bus::publish(&bus, ("sensor", 1), Some(payload("event")));
        step(&mut root, &id);
        data(&mut root, &mut cursor);
        root.respond(
            &mut cursor,
            EventDataResponse(Some(payload("original reply"))),
            &mut budget(),
        )
        .unwrap_or_else(|_| panic!());
        let stop = cursor.stop_handle();
        bus.borrow_mut().on_send = Some(Box::new(move || stop.request_stop()));
        if delivered_in_retirement {
            bus.borrow_mut().send = Mode::Pending;
            bus.borrow_mut().late_response_delivered = true;
        }
        step(&mut root, &id);
        assert_eq!(root.status(&id).unwrap().state, NotificationState::Closing);
        let report = settle(&mut root, &id);
        assert!(matches!(report.first_cause, Some(CoreError::Cancelled(_))));
        assert_eq!(report.responses_delivered, 1);
        assert_eq!(report.responses_uncertain, 0);
        assert_eq!(
            bus.borrow().response_inputs,
            vec![b"original reply".to_vec()]
        );
    }
}

#[test]
fn configuration_changes_during_permission_callback_precede_native_effects_and_delivery() {
    for already_queued in [false, true] {
        let bus = Bus::new();
        let (mut root, policy, configuration) = root(&bus, limits());
        let mut cursor = subscribe(
            &mut root,
            property(NotificationOverflow::Terminate),
            Program::property(),
        );
        let id = cursor.identity().clone();
        step(&mut root, &id);
        Bus::publish(&bus, ("sensor", 1), Some(payload("old configuration data")));
        if already_queued {
            step(&mut root, &id);
        }
        let polls = bus.borrow().receive_polls;
        *policy.on_deliver.borrow_mut() = Some(Box::new(move || configuration.set(2)));
        if already_queued {
            assert!(root.front(&mut cursor, &mut budget()).is_err());
        } else {
            assert!(
                root.progress(&id, &mut Context::from_waker(Waker::noop()), &mut budget())
                    .is_err()
            );
        }
        assert_eq!(bus.borrow().receive_polls, polls);
        assert_eq!(settle(&mut root, &id).delivered_items, 0);
    }
}

#[test]
fn property_requires_a_value_and_cannot_acquire_event_control_or_response_roles() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut request = property(NotificationOverflow::Terminate);
    request.subscription = SubscriptionInput(Some(payload("event-only filter")));
    assert!(
        root.subscribe(request, Program::property(), &mut budget())
            .is_err()
    );
    let mut request = property(NotificationOverflow::Terminate);
    request.responses = ResponseRequirement::Required;
    assert!(
        root.subscribe(request, Program::event(), &mut budget())
            .is_err()
    );
    assert_eq!(bus.borrow().prepare_calls, 0);
    let mut cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    step(&mut root, &id);
    Bus::publish(&bus, ("sensor", 1), None);
    step(&mut root, &id);
    assert!(root.front(&mut cursor, &mut budget()).is_err());
    assert!(matches!(
        settle(&mut root, &id).first_cause,
        Some(CoreError::Payload(_))
    ));
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    let rejected = root
        .stop(
            &cursor,
            Some(CancellationInput(Some(payload(
                "Event-only cancellation data",
            )))),
            &mut budget(),
        )
        .err()
        .unwrap();
    assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
    assert!(!settle(&mut root, &id).native_started);
}

#[test]
fn engine_reservation_and_zero_queue_limit_refuse_before_local_native_construction() {
    for zero_queue in [false, true] {
        let bus = Bus::new();
        let (probe, _, _) = root(&bus, limits());
        let root_bytes = probe.reserved_engine_bytes();
        drop(probe);
        let limits = if zero_queue {
            RawNotificationLimits {
                queue_items: 0,
                ..limits()
            }
        } else {
            RawNotificationLimits {
                engine_bytes: root_bytes,
                ..limits()
            }
        };
        let (mut root, _, _) = root(&bus, limits);
        assert!(
            root.subscribe(
                property(NotificationOverflow::Terminate),
                Program::property(),
                &mut budget()
            )
            .is_err()
        );
        assert_eq!(bus.borrow().prepare_calls, 0);
        assert_eq!(bus.borrow().start_polls, 0);
    }
}

#[test]
fn stop_intent_precedes_later_owner_revocation_and_pairing_errors_are_not_capacity() {
    let bus = Bus::new();
    let configuration = Rc::new(Cell::new(1));
    let result = Root::try_new(
        Binding {
            bus: bus.clone(),
            configuration,
        },
        Cell::new(2),
        Policy::new(),
        ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL),
        limits(),
    );
    assert!(matches!(
        result.err().unwrap().error().error(),
        CoreError::Validation(_)
    ));
    assert_eq!(bus.borrow().prepare_calls, 0);
    let (mut root, _, _) = root(&bus, limits());
    let cursor = subscribe(
        &mut root,
        property(NotificationOverflow::Terminate),
        Program::property(),
    );
    let id = cursor.identity().clone();
    cursor.stop_handle().request_stop();
    root.revoke();
    assert!(matches!(
        settle(&mut root, &id).first_cause,
        Some(CoreError::Cancelled(_))
    ));
}

#[test]
fn optional_event_response_can_be_declined_and_later_notifications_consumed_without_a_send() {
    let bus = Bus::new();
    let (mut root, policy, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(false), Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    let first = payload("first");
    let weak = Arc::downgrade(&first.body);
    Bus::publish(&bus, ("sensor", 1), Some(first));
    step(&mut root, &id);
    // An unexposed opportunity cannot be skipped through the receive cursor.
    assert!(root.decline_response(&mut cursor, &mut budget()).is_err());
    assert_eq!(data(&mut root, &mut cursor), b"first");
    Bus::publish(&bus, ("sensor", 1), Some(payload("second")));
    step(&mut root, &id);
    let reserved = root.reserved_engine_bytes();
    let bytes = root.status(&id).unwrap().queued_payload_bytes;
    assert_eq!(root.status(&id).unwrap().queued_items, 2);
    assert!(
        root.decline_response(&mut cursor, &mut WorkBudget::new())
            .is_err()
    );
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Awaiting
    );
    assert_eq!(bus.borrow().reply_retire_polls, 0);
    // Declining is release of accepted work, not permission to send a new
    // application response. A denied Respond permission must not force a send.
    policy.denied.set(bit(NotificationPermission::Respond));
    root.decline_response(&mut cursor, &mut budget()).unwrap();
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Declining
    );
    assert!(root.advance(&mut cursor, &mut budget()).is_err());
    assert!(
        root.progress(
            &id,
            &mut Context::from_waker(Waker::noop()),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    assert_eq!(bus.borrow().reply_retire_polls, 0);
    bus.borrow_mut().retire = Mode::Pending;
    for _ in 0..3 {
        assert_eq!(step(&mut root, &id), NotificationState::Active);
    }
    assert_eq!(data(&mut root, &mut cursor), b"first");
    assert_eq!(root.reserved_engine_bytes(), reserved);
    assert_eq!(root.status(&id).unwrap().queued_payload_bytes, bytes);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 2);
    assert_eq!(bus.borrow().reply_aborts, 0);
    assert!(root.advance(&mut cursor, &mut budget()).is_err());
    assert!(weak.upgrade().is_some());
    bus.borrow_mut().retire = Mode::Ready;
    assert_eq!(step(&mut root, &id), NotificationState::Active);
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .response,
        EventResponseState::Declined
    );
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 1);
    assert!(weak.upgrade().is_some()); // Cursor still owns the original data.
    root.advance(&mut cursor, &mut budget()).unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(data(&mut root, &mut cursor), b"second");
    assert_eq!(
        root.front(&mut cursor, &mut budget())
            .unwrap()
            .unwrap()
            .sequence,
        1
    );
    root.decline_response(&mut cursor, &mut budget()).unwrap();
    step(&mut root, &id);
    root.advance(&mut cursor, &mut budget()).unwrap();
    assert!(root.front(&mut cursor, &mut budget()).unwrap().is_none());
    assert_eq!(bus.borrow().reply_send_polls, 0);
    assert!(bus.borrow().response_inputs.is_empty());
    assert_eq!(bus.borrow().close_polls, 0);
    root.stop(&cursor, None, &mut budget())
        .unwrap_or_else(|_| panic!());
    let report = settle(&mut root, &id);
    assert_eq!(report.delivered_items, 2);
    assert_eq!(report.responses_delivered, 0);
    assert_eq!(report.responses_uncertain, 0);
}

#[test]
fn optional_decline_retirement_failure_retains_exact_owner_payload_and_capacity_until_cleanup() {
    let bus = Bus::new();
    let (mut root, _, _) = root(
        &bus,
        RawNotificationLimits {
            subscriptions: 1,
            ..limits()
        },
    );
    let baseline = root.reserved_engine_bytes();
    let mut cursor = subscribe(&mut root, event(false), Program::event());
    let id = cursor.identity().clone();
    step(&mut root, &id);
    let original = payload("retained optional response data");
    let weak = Arc::downgrade(&original.body);
    Bus::publish(&bus, ("sensor", 1), Some(original));
    step(&mut root, &id);
    assert_eq!(
        data(&mut root, &mut cursor),
        b"retained optional response data"
    );
    let reserved = root.reserved_engine_bytes();
    let bytes = root.status(&id).unwrap().queued_payload_bytes;
    root.decline_response(&mut cursor, &mut budget()).unwrap();
    bus.borrow_mut().retire = Mode::Fail;
    assert_eq!(step(&mut root, &id), NotificationState::Closing);
    assert!(matches!(
        root.status(&id).unwrap().first_cause,
        Some(CoreError::Cleanup(_))
    ));
    assert!(root.status(&id).unwrap().cleanup_error.is_some());
    assert_eq!(root.status(&id).unwrap().queued_payload_bytes, bytes);
    assert_eq!(root.reserved_engine_bytes(), reserved);
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 1);
    assert_eq!(bus.borrow().reply_aborts, 0);
    assert!(weak.upgrade().is_some());
    assert!(root.advance(&mut cursor, &mut budget()).is_err());
    assert!(root.take_settlement(&id).is_err());
    assert!(
        root.subscribe(event(false), Program::event(), &mut budget())
            .is_err()
    );
    bus.borrow_mut().retire = Mode::Pending;
    step(&mut root, &id);
    assert_eq!(root.reserved_engine_bytes(), reserved);
    assert!(weak.upgrade().is_some());
    bus.borrow_mut().retire = Mode::Ready;
    bus.borrow_mut().close = Mode::Pending;
    step(&mut root, &id); // Only native response retirement completed.
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 0);
    assert!(weak.upgrade().is_some());
    step(&mut root, &id); // Original data is now disposed.
    assert!(weak.upgrade().is_none());
    step(&mut root, &id); // Native participation is still pending.
    assert_eq!(root.reserved_engine_bytes(), reserved);
    assert!(root.take_settlement(&id).is_err());
    bus.borrow_mut().close = Mode::Ready;
    let report = settle(&mut root, &id);
    assert_eq!(report.responses_delivered, 0);
    assert_eq!(report.responses_uncertain, 0);
    assert!(report.cleanup_error.is_some());
    assert_eq!(bus.borrow().reply_send_polls, 0);
    assert_eq!(bus.borrow().reply_aborts, 1);
    assert!(bus.borrow().physical.is_empty());
    assert_eq!(root.reserved_engine_bytes(), baseline);
}

#[test]
fn required_or_already_submitted_event_responses_cannot_be_declined() {
    for required in [false, true] {
        let bus = Bus::new();
        let (mut root, _, _) = root(&bus, limits());
        let mut cursor = subscribe(&mut root, event(required), Program::event());
        let id = cursor.identity().clone();
        step(&mut root, &id);
        Bus::publish(&bus, ("sensor", 1), Some(payload("event")));
        step(&mut root, &id);
        data(&mut root, &mut cursor);
        if required {
            assert!(root.decline_response(&mut cursor, &mut budget()).is_err());
            assert_eq!(
                root.front(&mut cursor, &mut budget())
                    .unwrap()
                    .unwrap()
                    .response,
                EventResponseState::Awaiting
            );
            for _ in 0..3 {
                step(&mut root, &id);
            }
            assert_eq!(bus.borrow().reply_retire_polls, 0);
            assert!(root.advance(&mut cursor, &mut budget()).is_err());
        }
        root.respond(
            &mut cursor,
            EventDataResponse(Some(payload("admitted original response"))),
            &mut budget(),
        )
        .unwrap_or_else(|_| panic!());
        assert!(root.decline_response(&mut cursor, &mut budget()).is_err());
        bus.borrow_mut().send = Mode::Pending;
        step(&mut root, &id);
        assert!(root.decline_response(&mut cursor, &mut budget()).is_err());
        assert_eq!(
            root.front(&mut cursor, &mut budget())
                .unwrap()
                .unwrap()
                .response,
            EventResponseState::Pending
        );
        assert_eq!(bus.borrow().reply_retire_polls, 0);
        assert!(root.advance(&mut cursor, &mut budget()).is_err());
        bus.borrow_mut().send = Mode::Ready;
        step(&mut root, &id);
        step(&mut root, &id);
        assert!(root.decline_response(&mut cursor, &mut budget()).is_err());
        root.advance(&mut cursor, &mut budget()).unwrap();
        assert!(
            bus.borrow()
                .response_inputs
                .iter()
                .all(|p| p == b"admitted original response")
        );
        root.stop(&cursor, None, &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(settle(&mut root, &id).responses_delivered, 1);
    }
}

#[test]
fn cursor_loss_during_optional_decline_leaves_prepaid_owner_cleanup_usable() {
    let bus = Bus::new();
    let (mut root, _, _) = root(&bus, limits());
    let mut cursor = subscribe(&mut root, event(false), Program::event());
    step(&mut root, cursor.identity());
    let original = payload("abandoned decline");
    let weak = Arc::downgrade(&original.body);
    Bus::publish(&bus, ("sensor", 1), Some(original));
    step(&mut root, cursor.identity());
    data(&mut root, &mut cursor);
    root.decline_response(&mut cursor, &mut budget()).unwrap();
    bus.borrow_mut().retire = Mode::Pending;
    step(&mut root, cursor.identity());
    let reserved = root.reserved_engine_bytes();
    drop(cursor);
    for _ in 0..4 {
        assert!(
            root.step_cleanup(&mut Context::from_waker(Waker::noop()), &mut budget())
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(root.reserved_engine_bytes(), reserved);
    assert!(weak.upgrade().is_some());
    assert_eq!(bus.borrow().physical[&("sensor", 1)].jobs, 1);
    bus.borrow_mut().retire = Mode::Ready;
    let mut reports = vec![];
    for _ in 0..8 {
        if let Some(report) = root
            .step_cleanup(&mut Context::from_waker(Waker::noop()), &mut budget())
            .unwrap()
        {
            reports.push(report);
        }
    }
    assert_eq!(reports.len(), 1);
    assert_eq!(root.retained_subscriptions(), 0);
    assert_eq!(reports[0].responses_delivered, 0);
    assert_eq!(reports[0].responses_uncertain, 0);
    assert_eq!(bus.borrow().reply_send_polls, 0);
    assert!(bus.borrow().physical.is_empty());
    assert!(weak.upgrade().is_none());
}
