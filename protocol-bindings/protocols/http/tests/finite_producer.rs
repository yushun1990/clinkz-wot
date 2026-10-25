#![cfg(feature = "prepare")]
//! Actual HTTP I/O through the shared logical owner, not the installation ABI.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{ProducerPropertyReadAdmission, ProducerPropertyReadStep};
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use std::{
    cell::{Cell, RefCell},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, n))
}
fn now(ticks: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(91), ticks)
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn accepted<T, I>(value: Result<T, BindingInputRejection<I>>) -> T {
    value.unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn new_authority() -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL))
        .unwrap()
}
fn limits() -> RawFiniteLimits {
    RawFiniteLimits {
        candidates: 1,
        input_body_bytes: 0,
        output_body_bytes: 4096,
        output_media_capacity: 64,
        output_retained_bytes: 8192,
    }
}

// The real whole-description cursor and HTTP compiler run only here. The TD,
// semantic loan, admission and compiler all die before the first socket poll.
fn native(addr: SocketAddr, key: ProducerInstanceKey, id: u32, schema: &str) -> HttpFiniteBinding {
    let td: Thing = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"finite HTTP",
        "securityDefinitions":{"n":{"scheme":"nosec"}}, "security":["n"],
        "properties":{"level":{"type":schema,"readOnly":true,
            "forms":[{"href":format!("http://{addr}/value")}]}}
    }))
    .unwrap();
    let admission_config =
        ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut cursor = ValidatedThingCursor::from_thing(
        &td,
        &admission_config,
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
    let proof = loop {
        cursor = match cursor.step(&mut work(1_000_000), false) {
            ValidatedThingProgress::Complete(p) => break p,
            ValidatedThingProgress::Pending(c) => c,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        };
    };
    let mut admission = ProducerPropertyReadAdmission::new(proof, "level");
    while admission.step(&mut work(1_000_000), false).unwrap() != ProducerPropertyReadStep::Covered
    {
    }
    let listen = HttpProducerConfig::for_loopback(addr).unwrap();
    let program = admission
        .with_input(&mut work(1_000_000), |input, budget| {
            listen.compile_property_read(input, budget)
        })
        .unwrap()
        .unwrap();
    drop(admission);
    drop(td);
    HttpFiniteBinding::new(
        accepted(HttpFiniteConfig::try_new(
            listen,
            program,
            key,
            Arc::from("level"),
            10,
        )),
        BindingId::new(id),
        BindingGeneration::INITIAL,
    )
}

#[derive(Default)]
struct Application {
    calls: Cell<u32>,
    body: RefCell<Option<Vec<u8>>>,
    allowed: Cell<bool>,
    drops: Cell<u32>,
    bindings: RefCell<Vec<BindingId>>,
    lose_on_authorize: RefCell<Option<Rc<Cell<bool>>>>,
    lose_on_handle: RefCell<Option<Rc<Cell<bool>>>>,
}
struct Handler(Rc<Application>);
impl Drop for Handler {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl RawFiniteHandler for Handler {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        if let Some(service) = self.0.lose_on_authorize.borrow().as_ref() {
            service.set(false);
        }
        if self.0.allowed.get() {
            Ok(())
        } else {
            Err(CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )))
        }
    }
    fn handle(
        &self,
        context: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        if let Some(service) = self.0.lose_on_handle.borrow().as_ref() {
            service.set(false);
        }
        self.0.calls.set(self.0.calls.get() + 1);
        self.0
            .bindings
            .borrow_mut()
            .push(context.binding().unwrap().0);
        let body = self
            .0
            .body
            .borrow()
            .clone()
            .unwrap_or_else(|| self.0.calls.get().to_string().into_bytes());
        Ok(InteractionOutput::with_data(Payload::new(
            body,
            "application/json",
        )))
    }
}
fn instance(authority: ProducerInstanceAuthority, app: Rc<Application>) -> RawFiniteInstance {
    RawFiniteInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("level")),
            Operation::ReadProperty,
            Handler(app),
        )],
    )
}
fn owner(authority: ProducerInstanceAuthority, app: Rc<Application>) -> RawFiniteServient {
    app.allowed.set(true);
    accepted(RawFiniteServient::try_new(
        vec![instance(authority, app)],
        RawFiniteRegistryLimits {
            instances: 1,
            behaviors_per_instance: 1,
            target_name_bytes: 5,
        },
    ))
}
fn runtime<B: FiniteIngressBinding>(
    binding: B,
    owner: &RawFiniteServient,
    validation: FiniteOutputValidation,
) -> RawFiniteRuntime<B>
where
    B::Configuration: Clone,
{
    let config = binding.configuration().clone();
    accepted(RawFiniteRuntime::try_new(
        binding,
        config,
        owner,
        limits(),
        validation,
        Deadline::NONE,
    ))
}
fn step<B: FiniteIngressBinding>(owner: &mut RawFiniteServient, runtime: &mut RawFiniteRuntime<B>) {
    let _ = owner
        .step(runtime, &mut cx(), &mut work(1_000_000), now(0))
        .unwrap();
}
fn ready<B: FiniteIngressBinding>(
    owner: &mut RawFiniteServient,
    runtime: &mut RawFiniteRuntime<B>,
) {
    step(owner, runtime);
    assert_eq!(runtime.state(), RawFiniteState::InstalledClosed);
    runtime.activate_raw().unwrap(); // Deliberately no TD-publication claim.
}
fn request(addr: SocketAddr) -> TcpStream {
    let mut peer = TcpStream::connect(addr).unwrap();
    peer.write_all(
        format!("GET /value HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .unwrap();
    peer.set_nonblocking(true).unwrap();
    peer
}
fn until<B: FiniteIngressBinding>(
    owner: &mut RawFiniteServient,
    runtime: &mut RawFiniteRuntime<B>,
    state: RawFiniteState,
) {
    let start = Instant::now();
    while runtime.state() != state {
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "stalled at {:?}",
            runtime.state()
        );
        step(owner, runtime);
        std::thread::yield_now();
    }
}
fn response<B: FiniteIngressBinding>(
    owner: &mut RawFiniteServient,
    runtime: &mut RawFiniteRuntime<B>,
    mut peer: TcpStream,
) -> (String, RawFiniteSettlement) {
    let start = Instant::now();
    let mut bytes = Vec::new();
    let mut settlement = None;
    loop {
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "HTTP exchange stalled"
        );
        step(owner, runtime);
        settlement = settlement.or_else(|| runtime.take_settlement());
        match peer.read_to_end(&mut bytes) {
            Ok(_) if settlement.is_some() => break,
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("HTTP peer: {e:?}"),
        }
        std::thread::yield_now();
    }
    (String::from_utf8(bytes).unwrap(), settlement.unwrap())
}
fn close<B: FiniteIngressBinding>(runtime: &mut RawFiniteRuntime<B>) {
    runtime.begin_close();
    assert_eq!(runtime.step(&mut cx(), &mut work(0), now(0)), Poll::Pending);
    assert_eq!(
        runtime.step(&mut cx(), &mut work(1), now(0)),
        Poll::Ready(RawFiniteState::Closed)
    );
}

// Test control around the actual HTTP owner, for deterministic service-loss,
// pending-delivery and failed-retirement edges. No fake socket or reply path.
struct ControlledHttp {
    native: HttpFiniteBinding,
    live: Rc<Cell<bool>>,
    stall_send: Rc<Cell<bool>>,
    close_failures: Rc<Cell<u32>>,
    validations: Rc<Cell<u32>>,
}
impl ControlledHttp {
    fn new(native: HttpFiniteBinding) -> Self {
        Self {
            native,
            live: Rc::new(Cell::new(true)),
            stall_send: Rc::new(Cell::new(false)),
            close_failures: Rc::new(Cell::new(0)),
            validations: Rc::new(Cell::new(0)),
        }
    }
}
impl ProducerOutputValidator for ControlledHttp {
    fn validate(&self, output: &InteractionOutput) -> CoreResult<()> {
        self.validations.set(self.validations.get() + 1);
        self.native.output_validator().unwrap().validate(output)
    }
}
impl FiniteIngressBinding for ControlledHttp {
    type Configuration = HttpFiniteConfig;
    type Reply = HttpFiniteReply;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.native.identity()
    }
    fn configuration(&self) -> &Self::Configuration {
        self.native.configuration()
    }
    fn output_validator(&self) -> Option<&dyn ProducerOutputValidator> {
        self.live.get().then_some(self)
    }
    fn request_deadline(&self) -> Deadline {
        self.native.request_deadline()
    }
    fn poll_start(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.native.poll_start(cx)
    }
    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        self.native.poll_receive(cx, now)
    }
    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        if self.stall_send.get() {
            Poll::Pending
        } else {
            self.native.poll_send(cx, reply)
        }
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        ingress: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        cause: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        if self.close_failures.get() > 0 {
            self.close_failures.set(self.close_failures.get() - 1);
            Poll::Ready(Err(CoreError::Backpressure(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))))
        } else {
            self.native.poll_close(cx, ingress, reply, cause)
        }
    }
}

#[test]
fn one_logical_behavior_serves_independent_http_owners_and_survives_retirement() {
    let authority = new_authority();
    let key = authority.key();
    let app = Rc::new(Application::default());
    let mut owner = owner(authority, app.clone());
    let a = address();
    let b = address();
    let mut first = runtime(
        native(a, key, 1, "integer"),
        &owner,
        FiniteOutputValidation::Required,
    );
    let mut second = runtime(
        ControlledHttp::new(native(b, key, 2, "integer")),
        &owner,
        FiniteOutputValidation::Required,
    );
    assert_eq!(first.step(&mut cx(), &mut work(0), now(0)), Poll::Pending);
    drop(TcpListener::bind(a).unwrap()); // Zero credit did not bind.
    ready(&mut owner, &mut first);
    ready(&mut owner, &mut second);
    for n in 1..=100 {
        let (wire, settlement) = if n % 2 == 1 {
            response(&mut owner, &mut first, request(a))
        } else {
            response(&mut owner, &mut second, request(b))
        };
        assert!(wire.starts_with("HTTP/1.1 200"));
        assert!(wire.ends_with(&format!("\r\n\r\n{n}")));
        assert_eq!(settlement.recipient, Some(key));
        assert_eq!(settlement.effect, FiniteEffect::Unknown);
        assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
        assert!(settlement.result.is_ok());
    }
    close(&mut first);
    drop(TcpListener::bind(a).unwrap());
    let (wire, _) = response(&mut owner, &mut second, request(b));
    assert!(wire.ends_with("\r\n\r\n101"));
    assert_eq!(app.calls.get(), 101);
    assert_eq!(app.drops.get(), 0);
    assert!(
        app.bindings
            .borrow()
            .iter()
            .enumerate()
            .all(|(i, id)| *id == BindingId::new(if i % 2 == 0 && i < 100 { 1 } else { 2 }))
    );
    app.allowed.set(false); // NoSec still checks current application policy.
    let (wire, refused) = response(&mut owner, &mut second, request(b));
    assert!(wire.starts_with("HTTP/1.1 500"));
    assert_eq!(refused.effect, FiniteEffect::NotInvoked);
    assert_eq!(app.calls.get(), 101);
    close(&mut second);
    drop(owner);
    assert_eq!(app.drops.get(), 1);
}

#[test]
fn prepared_schema_is_enforced_on_real_http_without_claiming_raw_activation_is_publication() {
    for (schema, body, validation, status) in [
        (
            "integer",
            b"\"not a number\"".as_slice(),
            FiniteOutputValidation::Raw,
            200,
        ),
        (
            "integer",
            b"\"not a number\"".as_slice(),
            FiniteOutputValidation::Required,
            500,
        ),
        (
            "integer",
            b"1.00000000000000000000000001".as_slice(),
            FiniteOutputValidation::Required,
            500,
        ),
        (
            "integer",
            b"100e-2".as_slice(),
            FiniteOutputValidation::Required,
            200,
        ),
        (
            "number",
            b"-12.25e1".as_slice(),
            FiniteOutputValidation::Required,
            200,
        ),
        (
            "string",
            b"\"\\uD800\"".as_slice(),
            FiniteOutputValidation::Required,
            500,
        ),
    ] {
        let authority = new_authority();
        let key = authority.key();
        let addr = address();
        let app = Rc::new(Application::default());
        *app.body.borrow_mut() = Some(body.to_vec());
        let mut owner = owner(authority, app);
        let mut runtime = runtime(native(addr, key, 1, schema), &owner, validation);
        ready(&mut owner, &mut runtime);
        let (wire, settlement) = response(&mut owner, &mut runtime, request(addr));
        assert!(wire.starts_with(&format!("HTTP/1.1 {status}")), "{wire}");
        if status == 200 {
            assert!(wire.as_bytes().ends_with(body));
            assert!(settlement.result.is_ok());
        } else {
            assert!(wire.ends_with("\r\n\r\nnull"));
            assert!(settlement.result.is_err());
        }
        close(&mut runtime);
    }
}

#[test]
fn missing_or_lost_service_fences_start_dispatch_and_delivery_with_retained_cleanup() {
    for phase in 0..4 {
        let authority = new_authority();
        let key = authority.key();
        let addr = address();
        let app = Rc::new(Application::default());
        let mut owner = owner(authority, app.clone());
        let binding = ControlledHttp::new(native(addr, key, 1, "integer"));
        let live = binding.live.clone();
        let close_failures = binding.close_failures.clone();
        if phase == 0 {
            live.set(false);
            let config = binding.configuration().clone();
            assert!(
                RawFiniteRuntime::try_new(
                    binding,
                    config,
                    &owner,
                    limits(),
                    FiniteOutputValidation::Required,
                    Deadline::NONE
                )
                .is_err()
            );
            drop(TcpListener::bind(addr).unwrap());
            continue;
        }
        let mut runtime = runtime(binding, &owner, FiniteOutputValidation::Required);
        ready(&mut owner, &mut runtime);
        let peer = if phase >= 2 {
            let peer = request(addr);
            until(&mut owner, &mut runtime, RawFiniteState::Dispatching);
            Some(peer)
        } else {
            None
        };
        if phase == 3 {
            step(&mut owner, &mut runtime);
            assert_eq!(runtime.state(), RawFiniteState::Delivering);
        }
        live.set(false);
        close_failures.set(1);
        assert_eq!(
            owner
                .step(&mut runtime, &mut cx(), &mut work(0), now(0))
                .unwrap(),
            Poll::Pending
        );
        step(&mut owner, &mut runtime);
        assert_eq!(runtime.state(), RawFiniteState::Closing);
        assert!(matches!(
            runtime.first_cause(),
            Some(CoreError::Validation(_))
        ));
        assert_eq!(app.calls.get(), u32::from(phase == 3));
        step(&mut owner, &mut runtime); // Failed native close retains the owner.
        assert!(TcpListener::bind(addr).is_err());
        runtime.begin_close();
        step(&mut owner, &mut runtime);
        assert_eq!(runtime.state(), RawFiniteState::Closed);
        assert!(matches!(
            runtime.first_cause(),
            Some(CoreError::Validation(_))
        ));
        drop(TcpListener::bind(addr).unwrap());
        if phase >= 2 {
            let settlement = runtime.take_settlement().unwrap();
            assert_eq!(settlement.delivery, FiniteDelivery::NotDelivered);
            if phase == 3 {
                assert!(settlement.result.is_ok());
            } else {
                assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
            }
        }
        drop(peer);
    }
}

#[test]
fn callback_dependency_loss_fences_entry_and_preserves_the_original_validated_late_result() {
    for before_entry in [true, false] {
        let authority = new_authority();
        let key = authority.key();
        let addr = address();
        let app = Rc::new(Application::default());
        let mut owner = owner(authority, app.clone());
        let binding = ControlledHttp::new(native(addr, key, 1, "integer"));
        let validations = binding.validations.clone();
        if before_entry {
            *app.lose_on_authorize.borrow_mut() = Some(binding.live.clone());
        } else {
            *app.lose_on_handle.borrow_mut() = Some(binding.live.clone());
        }
        let mut runtime = runtime(binding, &owner, FiniteOutputValidation::Required);
        ready(&mut owner, &mut runtime);
        let _peer = request(addr);
        until(&mut owner, &mut runtime, RawFiniteState::Dispatching);
        step(&mut owner, &mut runtime); // Actual synchronous application callback.
        assert_eq!(app.calls.get(), u32::from(!before_entry));
        assert_eq!(validations.get(), u32::from(!before_entry));
        step(&mut owner, &mut runtime); // Lost service fences native delivery.
        assert_eq!(runtime.state(), RawFiniteState::Closing);
        assert!(matches!(
            runtime.first_cause(),
            Some(CoreError::Validation(_))
        ));
        close(&mut runtime);
        let settlement = runtime.take_settlement().unwrap();
        assert_eq!(settlement.delivery, FiniteDelivery::NotDelivered);
        if before_entry {
            assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
        } else {
            assert_eq!(
                settlement.result.unwrap().data().unwrap().body.as_ref(),
                b"1"
            );
        }
    }
}

#[test]
fn an_original_http_reply_cannot_be_substituted_at_a_sibling_with_the_same_numeric_identity() {
    let authority = new_authority();
    let key = authority.key();
    let a = address();
    let b = address();
    let mut first = native(a, key, 1, "integer");
    let mut second = native(b, key, 1, "integer");
    assert!(matches!(first.poll_start(&mut cx()), Poll::Ready(Ok(()))));
    assert!(matches!(second.poll_start(&mut cx()), Poll::Ready(Ok(()))));
    let mut peer_a = request(a);
    let mut peer_b = request(b);
    let receive = |binding: &mut HttpFiniteBinding| {
        let start = Instant::now();
        loop {
            assert!(start.elapsed() < Duration::from_secs(3));
            if let Poll::Ready(ingress) = binding.poll_receive(&mut cx(), now(0)) {
                break ingress.unwrap();
            }
        }
    };
    let reply = |ingress| RawFiniteReply {
        ingress,
        result: Ok(InteractionOutput::with_data(Payload::new(
            b"1".as_slice(),
            "application/json",
        ))),
        effect: FiniteEffect::NotInvoked,
        recipient: Some(key),
        operation: Some(Operation::ReadProperty),
    };
    let first_reply = reply(receive(&mut first));
    let second_reply = reply(receive(&mut second));
    assert!(matches!(
        second.poll_send(&mut cx(), &first_reply),
        Poll::Ready(Err(_))
    ));
    for (binding, reply, peer) in [
        (&mut first, &first_reply, &mut peer_a),
        (&mut second, &second_reply, &mut peer_b),
    ] {
        let start = Instant::now();
        loop {
            assert!(start.elapsed() < Duration::from_secs(3));
            if let Poll::Ready(result) = binding.poll_send(&mut cx(), reply) {
                result.unwrap();
                break;
            }
        }
        peer.set_nonblocking(false).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut bytes = String::new();
        peer.read_to_string(&mut bytes).unwrap();
        assert!(bytes.starts_with("HTTP/1.1 200"));
        assert!(matches!(
            binding.poll_close(&mut cx(), None, None, None),
            Poll::Ready(Ok(_))
        ));
    }
}

#[test]
fn incomplete_native_request_and_accepted_reply_keep_original_deadline_and_owner() {
    for accepted in [false, true] {
        let authority = new_authority();
        let key = authority.key();
        let addr = address();
        let app = Rc::new(Application::default());
        let mut owner = owner(authority, app.clone());
        let binding = ControlledHttp::new(native(addr, key, 1, "integer"));
        binding.stall_send.set(true);
        let mut runtime = runtime(binding, &owner, FiniteOutputValidation::Required);
        ready(&mut owner, &mut runtime);
        let mut peer = if accepted {
            request(addr)
        } else {
            let mut peer = TcpStream::connect(addr).unwrap();
            peer.write_all(b"GET /value HTTP/1.1\r\n").unwrap();
            peer
        };
        if accepted {
            until(&mut owner, &mut runtime, RawFiniteState::Delivering);
        } else {
            for _ in 0..20 {
                step(&mut owner, &mut runtime);
            }
        }
        // Polls at later times do not renew the request's tick-10 deadline.
        let _ = owner
            .step(&mut runtime, &mut cx(), &mut work(1_000_000), now(9))
            .unwrap();
        assert_ne!(runtime.state(), RawFiniteState::Closing);
        assert_eq!(
            runtime.step(&mut cx(), &mut work(0), now(10)),
            Poll::Pending
        );
        assert!(matches!(
            runtime.first_cause(),
            Some(CoreError::TimedOut(_))
        ));
        drop(owner);
        assert_eq!(app.drops.get(), u32::from(!accepted));
        assert_eq!(
            runtime.step(&mut cx(), &mut work(1), now(10)),
            Poll::Ready(RawFiniteState::Closed)
        );
        if accepted {
            assert_eq!(app.drops.get(), 0);
            let settlement = runtime.take_settlement().unwrap();
            assert_eq!(settlement.recipient, Some(key));
            assert_eq!(
                settlement.result.unwrap().data().unwrap().body.as_ref(),
                b"1"
            );
            assert_eq!(settlement.delivery, FiniteDelivery::NotDelivered);
            assert_eq!(app.drops.get(), 1);
        }
        let _ = peer.write(b"late data");
        drop(peer);
        drop(TcpListener::bind(addr).unwrap());
    }
}

#[test]
fn replacement_cannot_redirect_a_received_http_request() {
    let authority = new_authority();
    let key = authority.key();
    let addr = address();
    let app = Rc::new(Application::default());
    let mut owner = owner(authority, app.clone());
    let mut runtime = runtime(
        native(addr, key, 1, "integer"),
        &owner,
        FiniteOutputValidation::Required,
    );
    ready(&mut owner, &mut runtime);
    let peer = request(addr);
    until(&mut owner, &mut runtime, RawFiniteState::Dispatching);
    let successor = new_authority();
    let new_key = successor.key();
    accepted(owner.replace(key, instance(successor, app.clone())));
    let (wire, settlement) = response(&mut owner, &mut runtime, peer);
    assert!(wire.starts_with("HTTP/1.1 500"));
    assert_eq!(settlement.recipient, Some(key));
    assert_ne!(settlement.recipient, Some(new_key));
    assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
    assert_eq!(app.calls.get(), 0);
    close(&mut runtime);
}

#[test]
fn native_setup_failure_and_last_runtime_drop_release_real_sockets() {
    let authority = new_authority();
    let key = authority.key();
    let app = Rc::new(Application::default());
    let mut owner = owner(authority, app);
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = occupied.local_addr().unwrap();
    let mut failed = runtime(
        native(addr, key, 1, "integer"),
        &owner,
        FiniteOutputValidation::Required,
    );
    step(&mut owner, &mut failed);
    assert_eq!(failed.state(), RawFiniteState::Closing);
    assert!(matches!(failed.first_cause(), Some(CoreError::Binding(_))));
    close(&mut failed);
    drop(occupied);
    let mut live = runtime(
        native(addr, key, 2, "integer"),
        &owner,
        FiniteOutputValidation::Required,
    );
    ready(&mut owner, &mut live);
    let mut peer = TcpStream::connect(addr).unwrap();
    peer.write_all(b"GET /value HTTP/1.1\r\n").unwrap();
    for _ in 0..10 {
        step(&mut owner, &mut live);
    }
    drop(live);
    drop(peer);
    drop(TcpListener::bind(addr).unwrap());
}

#[test]
fn validation_traversal_is_prepaid_and_oversize_never_enters_the_codec() {
    for size in [4096, 4097] {
        let authority = new_authority();
        let key = authority.key();
        let addr = address();
        let app = Rc::new(Application::default());
        let mut body = vec![b' '; size];
        body[0] = b'1';
        *app.body.borrow_mut() = Some(body);
        let mut owner = owner(authority, app.clone());
        let binding = ControlledHttp::new(native(addr, key, 1, "integer"));
        let validations = binding.validations.clone();
        let mut runtime = runtime(binding, &owner, FiniteOutputValidation::Required);
        ready(&mut owner, &mut runtime);
        let peer = request(addr);
        until(&mut owner, &mut runtime, RawFiniteState::Dispatching);
        let required = 1 + (5 + 1) + 1 + 4096 + 2 * 64;
        assert_eq!(
            owner
                .step(&mut runtime, &mut cx(), &mut work(required - 1), now(0))
                .unwrap(),
            Poll::Pending
        );
        assert_eq!(app.calls.get(), 0);
        assert_eq!(validations.get(), 0);
        assert_eq!(
            owner
                .step(&mut runtime, &mut cx(), &mut work(required), now(0))
                .unwrap(),
            Poll::Ready(RawFiniteState::Delivering)
        );
        assert_eq!(app.calls.get(), 1);
        assert_eq!(validations.get(), u32::from(size == 4096));
        let (wire, _) = response(&mut owner, &mut runtime, peer);
        assert!(wire.starts_with(if size == 4096 {
            "HTTP/1.1 200"
        } else {
            "HTTP/1.1 500"
        }));
        close(&mut runtime);
    }
}

#[test]
fn closed_gate_and_malformed_native_input_never_dispatch_and_listener_recovers() {
    let authority = new_authority();
    let key = authority.key();
    let addr = address();
    let app = Rc::new(Application::default());
    let mut owner = owner(authority, app.clone());
    let mut runtime = runtime(
        native(addr, key, 1, "integer"),
        &owner,
        FiniteOutputValidation::Required,
    );
    step(&mut owner, &mut runtime);
    let peer = request(addr);
    for _ in 0..20 {
        step(&mut owner, &mut runtime);
    }
    assert_eq!(runtime.state(), RawFiniteState::InstalledClosed);
    assert_eq!(app.calls.get(), 0);
    runtime.activate_raw().unwrap();
    let (wire, _) = response(&mut owner, &mut runtime, peer);
    assert!(wire.starts_with("HTTP/1.1 200"));
    for malformed in [
        format!("POST /value HTTP/1.1\r\nHost: {addr}\r\n\r\n"),
        format!("GET /other HTTP/1.1\r\nHost: {addr}\r\n\r\n"),
        "GET /value HTTP/1.1\r\nHost: wrong\r\n\r\n".into(),
        format!("GET /value HTTP/1.1\r\nHost: {addr}\r\nContent-Length: 1\r\n\r\nx"),
        format!(
            "GET /value HTTP/1.1\r\nHost: {addr}\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
        ),
    ] {
        let mut peer = TcpStream::connect(addr).unwrap();
        peer.write_all(malformed.as_bytes()).unwrap();
        peer.set_nonblocking(true).unwrap();
        let start = Instant::now();
        let mut bytes = Vec::new();
        loop {
            assert!(start.elapsed() < Duration::from_secs(3));
            step(&mut owner, &mut runtime);
            match peer.read_to_end(&mut bytes) {
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => panic!("{e:?}"),
            }
        }
        assert_eq!(app.calls.get(), 1);
        assert!(runtime.take_settlement().is_none());
    }
    let (wire, _) = response(&mut owner, &mut runtime, request(addr));
    assert!(wire.ends_with("\r\n\r\n2"));
    close(&mut runtime);
}

#[test]
fn deadline_and_clock_fencing_before_start_require_only_funded_native_cleanup() {
    for (deadline, later, timeout) in [
        (Deadline::at(now(5)), now(5), true),
        (Deadline::NONE, now(0), false),
        (
            Deadline::NONE,
            MonotonicInstant::new(ClockId::new(92), 2),
            false,
        ),
    ] {
        let authority = new_authority();
        let key = authority.key();
        let addr = address();
        let mut owner = owner(authority, Rc::new(Application::default()));
        let binding = native(addr, key, 1, "integer");
        let config = binding.configuration().clone();
        let mut runtime = accepted(RawFiniteRuntime::try_new(
            binding,
            config,
            &owner,
            limits(),
            FiniteOutputValidation::Required,
            deadline,
        ));
        assert_eq!(
            owner
                .step(&mut runtime, &mut cx(), &mut work(0), now(1))
                .unwrap(),
            Poll::Pending
        );
        assert_eq!(
            owner
                .step(&mut runtime, &mut cx(), &mut work(0), later)
                .unwrap(),
            Poll::Pending
        );
        assert_eq!(runtime.state(), RawFiniteState::Closing);
        assert_eq!(
            matches!(runtime.first_cause(), Some(CoreError::TimedOut(_))),
            timeout
        );
        assert_eq!(
            runtime.step(&mut cx(), &mut work(1), later),
            Poll::Ready(RawFiniteState::Closed)
        );
        drop(TcpListener::bind(addr).unwrap());
    }
    let authority = new_authority();
    let key = authority.key();
    let addr = address();
    let mut owner = owner(authority, Rc::new(Application::default()));
    let binding = ControlledHttp::new(native(addr, key, 1, "integer"));
    let live = binding.live.clone();
    let mut runtime = runtime(binding, &owner, FiniteOutputValidation::Required);
    step(&mut owner, &mut runtime);
    live.set(false);
    assert!(runtime.activate_raw().is_err());
    assert_eq!(runtime.state(), RawFiniteState::Closing);
    close(&mut runtime);
}

#[test]
fn cancelling_an_actual_partial_http_write_reports_unknown_delivery_and_keeps_the_result() {
    let authority = new_authority();
    let key = authority.key();
    let addr = address();
    let app = Rc::new(Application::default());
    let mut body = vec![b' '; 4096];
    body[0] = b'1';
    *app.body.borrow_mut() = Some(body.clone());
    let mut owner = owner(authority, app.clone());
    let mut runtime = runtime(
        native(addr, key, 1, "integer"),
        &owner,
        FiniteOutputValidation::Required,
    );
    ready(&mut owner, &mut runtime);
    let mut peer = request(addr);
    until(&mut owner, &mut runtime, RawFiniteState::Delivering);
    // Header + 4096-byte body cannot fit the one-poll 4096-byte write allowance.
    assert_eq!(runtime.step(&mut cx(), &mut work(1), now(0)), Poll::Pending);
    assert_eq!(runtime.state(), RawFiniteState::Delivering);
    close(&mut runtime);
    assert!(matches!(
        runtime.first_cause(),
        Some(CoreError::Cancelled(_))
    ));
    let settlement = runtime.take_settlement().unwrap();
    assert_eq!(settlement.delivery, FiniteDelivery::Unknown);
    assert_eq!(settlement.effect, FiniteEffect::Unknown);
    assert_eq!(
        settlement.result.unwrap().data().unwrap().body.as_ref(),
        body.as_slice()
    );
    peer.set_nonblocking(false).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let mut received = Vec::new();
    peer.read_to_end(&mut received).unwrap();
    assert!(!received.is_empty());
    assert!(!received.ends_with(&body));
    assert_eq!(app.calls.get(), 1);
}

#[test]
fn public_consumer_reads_the_shared_http_participant_after_sources_are_dropped() {
    use clinkz_wot_servient::{consumer_property_read::*, host_accounts::HostResourceAccounts};
    struct Clock;
    impl RuntimeClock for Clock {
        fn now(&self) -> MonotonicInstant {
            now(0)
        }
        fn ticks_per_second(&self) -> std::num::NonZeroU64 {
            std::num::NonZeroU64::new(1000).unwrap()
        }
    }
    let io = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    io.block_on(async {
        let authority = new_authority(); let key = authority.key(); let addr = address();
        let mut owner = owner(authority, Rc::new(Application::default()));
        let mut server = runtime(native(addr, key, 1, "integer"), &owner, FiniteOutputValidation::Required);
        ready(&mut owner, &mut server);
        let policy = ConsumerPropertyReadPolicy::try_new_owned(
            BindingResourceContract::ConsumerOwnedHostV2, ResourceProfileId::GATEWAY_DEFAULT_V1,
            GatewayDefaultV1::LIMITS.clone(), BindingLifetimeFootprint::new(8, 8192),
            BindingLifetimeFootprint::new(8, 4096), [1, 64, 64, 64], [4096, 16],
            [2, 2, 1, 1, 1, 1, 1], [30_000, 1000], 65536).unwrap();
        let bank = Arc::new(HostResourceAccounts::try_from_accounts(policy.parent_accounts()).unwrap());
        let registration = try_consumer_registration(HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(2), BindingGeneration::INITIAL, 0).unwrap();
        let (client, observer) = accepted(ConsumerPropertyReadRuntime::try_new(registration, policy, Arc::new(Clock), bank.clone()));
        let td: Thing = serde_json::from_value(serde_json::json!({
            "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"client", "id":"urn:finite:client",
            "securityDefinitions":{"n":{"scheme":"nosec"}}, "security":["n"],
            "properties":{"level":{"forms":[{"href":format!("http://{addr}/value")}]}}
        })).unwrap();
        let handle = match client.consume(&td, &mut work(1_000_000), |build| {
            for _ in 0..10000 { build.step(&mut work(1_000_000)); if build.is_finished() { return; } }
            panic!("consume stalled");
        }).unwrap() { StartStatus::Ready(handle) => handle, _ => panic!("client not ready") };
        drop(td);
        for value in 1..=3 {
            let mut call = handle.read_property("level", InteractionOptions::new(), &mut work(1_000_000)).unwrap();
            let start = Instant::now();
            let output = loop {
                assert!(start.elapsed() < Duration::from_secs(3));
                step(&mut owner, &mut server);
                call.step(&mut cx(), &mut work(1_000_000));
                if let Some(result) = call.try_take(&mut work(1_000_000)).unwrap() { break result.unwrap(); }
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(output.data().unwrap().body.as_ref(), value.to_string().as_bytes());
            assert!(server.take_settlement().unwrap().result.is_ok());
        }
        drop(handle);
        let start = Instant::now();
        loop {
            if let StartStatus::Ready(outcome) = client.shutdown(&mut cx(), &mut work(1_000_000)) {
                assert!(outcome.is_complete()); break;
            }
            assert!(start.elapsed() < Duration::from_secs(3), "Consumer cleanup stalled");
        }
        drop(client); drop(observer);
        assert!(bank.snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS).iter().all(|a| a.used() == 0));
        close(&mut server);
    });
}
