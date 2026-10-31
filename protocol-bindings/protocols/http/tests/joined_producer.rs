#![cfg(feature = "prepare")]
//! Public joined v6 path with independent real TCP/HTTP peers. Controlled
//! wrappers retain the real native owner; they do not replace socket execution.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{ResolvedProducerInput, finite_producer::FiniteProducerCompiler};
use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer::*};
use clinkz_wot_servient_finite::RawFiniteState;
use serde_json::{Value, json};
use std::{
    cell::Cell,
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
fn now(t: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(91), t)
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
fn authority() -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL))
        .unwrap()
}
fn document(addr: SocketAddr) -> Value {
    json!({"@context":["https://www.w3.org/2022/wot/td/v1.1",{"htv":"http://www.w3.org/2011/http#"}],
        "title":"joined", "securityDefinitions":{"n":{"scheme":"nosec"},"b":{"scheme":"basic"}}, "security":["n"],
        "properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{addr}/value")}]}}})
}
fn compiler(addr: SocketAddr) -> HttpFiniteProducer {
    HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap()
}
fn accounts() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
}

#[derive(Default)]
struct Application {
    calls: Cell<usize>,
    drops: Cell<usize>,
    denied: Cell<bool>,
    body: Cell<&'static [u8]>,
}
struct Handler(Rc<Application>);
impl Drop for Handler {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl RawFiniteHandler for Handler {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        if self.0.denied.get() {
            Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )))
        } else {
            Ok(())
        }
    }
    fn handle(
        &self,
        context: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert!(context.thing_id().is_none());
        self.0.calls.set(self.0.calls.get() + 1);
        Ok(InteractionOutput::with_data(Payload::new(
            self.0.body.get().to_vec(),
            "application/json",
        )))
    }
}
fn app() -> Rc<Application> {
    Rc::new(Application {
        body: Cell::new(b"42"),
        ..Default::default()
    })
}
fn root<C: FiniteProducerCompiler>(c: C, bank: Arc<HostResourceAccounts>) -> ProducerServient<C> {
    ProducerServient::try_new(c, GatewayDefaultV1::LIMITS.clone(), bank).unwrap()
}
fn prepare<C: FiniteProducerCompiler>(
    s: &ProducerServient<C>,
    value: Value,
    a: ProducerInstanceAuthority,
    application: Rc<Application>,
    expected: Option<ProducerInstanceKey>,
) -> CoreResult<ProducerExposure<C::Binding>> {
    let text: Arc<str> = Arc::from(value.to_string());
    s.prepare(
        text,
        Arc::from("level"),
        a,
        Handler(application),
        0,
        expected,
        Deadline::NONE,
        &mut work(1_000_000),
    )
}
fn step<C: FiniteProducerCompiler>(
    s: &mut ProducerServient<C>,
    e: &mut ProducerExposure<C::Binding>,
) {
    let _ = s
        .step(e, &mut cx(), &mut work(1_000_000), now(0), false)
        .unwrap();
}
fn until<C: FiniteProducerCompiler>(
    s: &mut ProducerServient<C>,
    e: &mut ProducerExposure<C::Binding>,
    state: RawFiniteState,
) {
    let end = Instant::now() + Duration::from_secs(5);
    while e.state() != state {
        assert!(
            Instant::now() < end,
            "stalled at {:?}, {:?}",
            e.state(),
            e.first_cause()
        );
        step(s, e);
    }
}
fn publish<C: FiniteProducerCompiler>(
    s: &mut ProducerServient<C>,
    e: &mut ProducerExposure<C::Binding>,
) {
    until(s, e, RawFiniteState::InstalledClosed);
    assert!(s.description(e.key()).is_none());
    step(s, e);
    assert_eq!(e.state(), RawFiniteState::Serving, "{:?}", e.first_cause());
    assert!(s.description(e.key()).is_some());
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
fn response<C: FiniteProducerCompiler>(
    s: &mut ProducerServient<C>,
    e: &mut ProducerExposure<C::Binding>,
    peer: &mut TcpStream,
) -> Vec<u8> {
    let end = Instant::now() + Duration::from_secs(5);
    let mut wire = Vec::new();
    loop {
        assert!(Instant::now() < end, "stalled reply: {:?}", e.state());
        step(s, e);
        let mut bytes = [0; 8192];
        match peer.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => wire.extend_from_slice(&bytes[..n]),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionReset => break,
            Err(err) => panic!("{err}"),
        }
    }
    wire
}
fn close<C: FiniteProducerCompiler>(
    s: &mut ProducerServient<C>,
    e: &mut ProducerExposure<C::Binding>,
) {
    e.begin_close();
    until(s, e, RawFiniteState::Closed);
}
fn assert_released(addr: SocketAddr) {
    TcpListener::bind(addr).unwrap();
}

#[test]
fn complete_final_td_closed_setup_atomic_publication_and_repeated_real_io() {
    let addr = address();
    let bank = accounts();
    let application = app();
    let mut s = root(compiler(addr), bank.clone());
    let value = document(addr);
    let expected = value.clone();
    let mut e = prepare(&s, value, authority(), application.clone(), None).unwrap();
    // No caller TD/semantic loan/legacy runtime is retained. Before first paid
    // setup no listener exists, and zero work cannot create one.
    let _ = s
        .step(&mut e, &mut cx(), &mut work(0), now(0), false)
        .unwrap();
    assert_eq!(e.state(), RawFiniteState::Starting);
    assert!(TcpStream::connect(addr).is_err());
    until(&mut s, &mut e, RawFiniteState::InstalledClosed);
    let mut peer = request(addr);
    for _ in 0..5 {
        let _ = s
            .step(&mut e, &mut cx(), &mut work(0), now(0), false)
            .unwrap();
        assert_eq!(application.calls.get(), 0);
        assert!(s.description(e.key()).is_none());
    }
    let mut byte = [0];
    assert_eq!(
        peer.read(&mut byte).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    step(&mut s, &mut e); // one final qualification/publication boundary
    assert_eq!(e.state(), RawFiniteState::Serving);
    assert_eq!(
        serde_json::from_str::<Value>(s.description(e.key()).unwrap()).unwrap(),
        expected
    );
    let wire = response(&mut s, &mut e, &mut peer);
    assert!(wire.starts_with(b"HTTP/1.1 200"));
    assert!(wire.ends_with(b"42"));
    let settlement = e.take_settlement().unwrap();
    assert_eq!(settlement.recipient, Some(e.key()));
    assert_eq!(settlement.binding.0, BindingId::new(7));
    assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
    assert_eq!(settlement.effect, FiniteEffect::Unknown);
    assert_eq!(
        settlement.result.unwrap().data().unwrap().body.as_ref(),
        b"42"
    );
    for _ in 0..100 {
        let mut peer = request(addr);
        assert!(response(&mut s, &mut e, &mut peer).ends_with(b"42"));
        assert_eq!(
            e.take_settlement().unwrap().delivery,
            FiniteDelivery::Delivered
        );
    }
    assert_eq!(application.calls.get(), 101);
    close(&mut s, &mut e);
    assert!(s.description(e.key()).is_none());
    s.retire(&mut e, &mut work(1)).unwrap();
    drop(e);
    drop(s);
    assert_eq!(application.drops.get(), 1);
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::AdmissionTemporaryBytesGlobalMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::CleanupItemsMax,
        ResourceKind::ThingsGlobalMax,
        ResourceKind::PendingHandlerCallsGlobalMax,
        ResourceKind::InFlightResponsesGlobalMax,
        ResourceKind::BindingIngressItemsGlobalMax,
    ] {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
    assert_released(addr);
}

#[test]
fn every_advertised_alternative_refuses_before_native_setup() {
    for mutation in 0..19 {
        let addr = address();
        let application = app();
        let bank = accounts();
        let s = root(compiler(addr), bank.clone());
        let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        let mut v = document(addr);
        match mutation {
            0 => {
                let form = v["properties"]["level"]["forms"][0].clone();
                v["properties"]["level"]["forms"]
                    .as_array_mut()
                    .unwrap()
                    .push(form);
            }
            1 => v["properties"]["other"] = json!({"type":"integer","readOnly":true,"forms":[]}),
            2 => v["properties"]["level"]
                .as_object_mut()
                .unwrap()
                .remove("readOnly")
                .map(|_| ())
                .unwrap(),
            3 => {
                v["properties"]["level"]["forms"][0]["op"] =
                    json!(["readproperty", "writeproperty"])
            }
            4 => v["actions"] = json!({"run":{"forms":[]}}),
            5 => v["events"] = json!({"alarm":{"forms":[]}}),
            6 => {
                v["forms"] = json!([{"href":format!("http://{addr}/all"),"op":"readallproperties"}])
            }
            7 => v["properties"]["level"]["minimum"] = json!(0),
            8 => v["security"] = json!(["b"]),
            9 => v["properties"]["level"]["forms"][0]["security"] = json!(["n", "b"]),
            10 => v["properties"]["level"]["forms"][0]["contentType"] = json!("text/plain"),
            11 => v["properties"]["level"]["forms"][0]["subprotocol"] = json!("longpoll"),
            12 => v["properties"]["level"]["forms"][0]["htv:methodName"] = json!("POST"),
            13 => {
                v["properties"]["level"]["forms"][0]["response"] =
                    json!({"htv:statusCodeValue":201})
            }
            14 => v["properties"]["level"]["forms"][0]["additionalResponses"] = json!([]),
            15 => v["properties"]["level"]["observable"] = json!(true),
            16 => v["https://vendor.example/required"] = json!(true),
            17 => v["properties"]["level"]["forms"][0]["href"] = json!("http://127.0.0.1:1/wrong"),
            _ => v["profile"] = json!("urn:unsupported:profile"),
        }
        assert!(
            prepare(&s, v, authority(), application.clone(), None).is_err(),
            "mutation {mutation}"
        );
        assert_eq!(application.calls.get(), 0);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            baseline
        );
        assert_released(addr);
    }
}

#[test]
fn supported_aliases_defaults_security_override_and_anonymous_empty_key() {
    let addr = address();
    let application = app();
    let mut s = root(compiler(addr), accounts());
    let mut v = document(addr);
    v["@context"][1]["method"] = json!("htv:methodName");
    v["security"] = json!(["b"]);
    v["properties"]["level"]["forms"][0]["security"] = json!(["n"]);
    v["properties"]["level"]["forms"][0]["method"] = json!("GET");
    v["base"] = json!(format!("http://{addr}/base/"));
    v["properties"]["level"]["forms"][0]["href"] = json!("../value");
    let property = v["properties"]
        .as_object_mut()
        .unwrap()
        .remove("level")
        .unwrap();
    v["properties"][""] = property;
    let mut e = s
        .prepare(
            Arc::from(v.to_string()),
            Arc::from(""),
            authority(),
            Handler(application.clone()),
            0,
            None,
            Deadline::NONE,
            &mut work(1_000_000),
        )
        .unwrap();
    publish(&mut s, &mut e);
    assert!(response(&mut s, &mut e, &mut request(addr)).ends_with(b"42"));
    assert_eq!(
        e.take_settlement().unwrap().delivery,
        FiniteDelivery::Delivered
    );
    close(&mut s, &mut e);
}

#[test]
fn cancellation_before_setup_after_readiness_and_after_dispatch_retains_truth() {
    for boundary in 0..3 {
        let addr = address();
        let application = app();
        let mut s = root(compiler(addr), accounts());
        let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
        if boundary > 0 {
            until(&mut s, &mut e, RawFiniteState::InstalledClosed);
        }
        let mut peer = None;
        if boundary == 2 {
            publish(&mut s, &mut e);
            peer = Some(request(addr));
            until(&mut s, &mut e, RawFiniteState::Delivering);
            assert_eq!(application.calls.get(), 1);
        }
        let _ = s
            .step(&mut e, &mut cx(), &mut work(0), now(0), true)
            .unwrap();
        assert_eq!(e.state(), RawFiniteState::Closing);
        assert!(s.description(e.key()).is_none());
        assert!(matches!(e.first_cause(), Some(CoreError::Cancelled(_))));
        until(&mut s, &mut e, RawFiniteState::Closed);
        if boundary == 2 {
            let settlement = e.take_settlement().unwrap();
            assert_eq!(settlement.recipient, Some(e.key()));
            assert_eq!(settlement.delivery, FiniteDelivery::NotDelivered);
            assert_eq!(settlement.effect, FiniteEffect::Unknown);
            assert!(settlement.result.is_ok());
        } else {
            assert_eq!(application.calls.get(), 0);
        }
        drop(peer);
        assert_released(addr);
    }
}

#[test]
fn fresh_currentness_and_authorization_checked_between_ingress_and_dispatch() {
    for denied in [false, true] {
        let addr = address();
        let application = app();
        let mut s = root(compiler(addr), accounts());
        let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
        publish(&mut s, &mut e);
        let mut peer = request(addr);
        until(&mut s, &mut e, RawFiniteState::Dispatching);
        if denied {
            application.denied.set(true);
        } else {
            assert!(s.revoke(e.key()));
        }
        assert!(response(&mut s, &mut e, &mut peer).starts_with(b"HTTP/1.1 500"));
        let settlement = e.take_settlement().unwrap();
        assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
        assert_eq!(application.calls.get(), 0);
        if !denied {
            assert!(matches!(settlement.result, Err(CoreError::StaleHandle(_))));
        }
        close(&mut s, &mut e);
    }
}

#[test]
fn invalid_application_output_cannot_be_delivered_as_schema_success() {
    let addr = address();
    let application = app();
    application.body.set(b"\"wrong type\"");
    let mut s = root(compiler(addr), accounts());
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    publish(&mut s, &mut e);
    let wire = response(&mut s, &mut e, &mut request(addr));
    assert!(wire.starts_with(b"HTTP/1.1 500"));
    let settlement = e.take_settlement().unwrap();
    assert_eq!(settlement.effect, FiniteEffect::Unknown);
    assert!(matches!(settlement.result, Err(CoreError::Validation(_))));
    assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
    close(&mut s, &mut e);
}

#[test]
fn replacement_preserves_original_result_identity_and_last_owner_funding() {
    let addr = address();
    let bank = accounts();
    let old_app = app();
    let new_app = app();
    new_app.body.set(b"73");
    let mut s = root(compiler(addr), bank.clone());
    let mut old = prepare(&s, document(addr), authority(), old_app.clone(), None).unwrap();
    publish(&mut s, &mut old);
    let peer = request(addr);
    until(&mut s, &mut old, RawFiniteState::Delivering);
    let old_key = old.key();
    old.cancel();
    until(&mut s, &mut old, RawFiniteState::Closed);
    drop(peer);
    assert!(
        s.retire(&mut old, &mut work(1)).is_err(),
        "unacknowledged original result"
    );
    let mut successor = prepare(
        &s,
        document(addr),
        authority(),
        new_app.clone(),
        Some(old_key),
    )
    .unwrap();
    publish(&mut s, &mut successor);
    assert!(s.description(old_key).is_none());
    assert_ne!(successor.key(), old_key);
    assert!(response(&mut s, &mut successor, &mut request(addr)).ends_with(b"73"));
    successor.take_settlement().unwrap();
    assert_eq!(old_app.drops.get(), 0);
    let held = bank.account(ResourceKind::CleanupItemsMax).used();
    assert!(held >= 2);
    let settlement = old.take_settlement().unwrap();
    assert_eq!(settlement.recipient, Some(old_key));
    assert_eq!(
        settlement.result.unwrap().data().unwrap().body.as_ref(),
        b"42"
    );
    s.retire(&mut old, &mut work(1)).unwrap();
    drop(old);
    assert_eq!(old_app.drops.get(), 1);
    assert!(s.description(successor.key()).is_some());
    close(&mut s, &mut successor);
    s.retire(&mut successor, &mut work(1)).unwrap();
    drop(successor);
    drop(s);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    assert_released(addr);
}

#[test]
fn occupied_native_endpoint_never_publishes_and_first_cause_survives_cancel() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let application = app();
    let mut s = root(compiler(addr), accounts());
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    step(&mut s, &mut e);
    assert_eq!(e.state(), RawFiniteState::Closing);
    let first = e.first_cause().unwrap().clone();
    e.cancel();
    until(&mut s, &mut e, RawFiniteState::Closed);
    assert_eq!(e.first_cause(), Some(&first));
    assert!(s.description(e.key()).is_none());
    assert_eq!(application.calls.get(), 0);
    drop(listener);
    assert_released(addr);
}

#[test]
fn loss_of_exposure_or_root_closes_authority_and_native_owners_without_waiting() {
    for lose_root in [false, true] {
        let addr = address();
        let application = app();
        let bank = accounts();
        let mut s = root(compiler(addr), bank.clone());
        let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
        publish(&mut s, &mut e);
        let _peer = request(addr);
        until(&mut s, &mut e, RawFiniteState::Delivering);
        let key = e.key();
        if lose_root {
            drop(s);
            assert_eq!(application.drops.get(), 0);
            while e.state() != RawFiniteState::Closed {
                let _ = e.poll_cleanup(&mut cx(), &mut work(1_000_000), now(0));
            }
            assert!(e.take_settlement().unwrap().result.is_ok());
            drop(e);
        } else {
            drop(e);
            assert!(s.description(key).is_none());
            drop(s);
        }
        assert_eq!(application.drops.get(), 1);
        assert_released(addr);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            0
        );
    }
}

#[derive(Default)]
struct Controls {
    lose_service: Cell<bool>,
    refuse_final: Cell<bool>,
    fail_send: Cell<bool>,
    final_calls: Cell<usize>,
    close_polls: Cell<usize>,
    hold_delivery: Cell<bool>,
}
struct ControlledCompiler {
    actual: HttpFiniteProducer,
    controls: Rc<Controls>,
}
struct ControlledBinding {
    actual: HttpFiniteBinding,
    controls: Rc<Controls>,
    delivered: bool,
}
impl FiniteIngressBinding for ControlledBinding {
    type Configuration = HttpFiniteConfig;
    type Reply = HttpFiniteReply;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.actual.identity()
    }
    fn configuration(&self) -> &Self::Configuration {
        self.actual.configuration()
    }
    fn prepared_for_instance(&self, key: ProducerInstanceKey) -> bool {
        self.actual.prepared_for_instance(key)
    }
    fn validator(&self) -> Option<&dyn FiniteValidator> {
        if self.controls.lose_service.get() {
            None
        } else {
            self.actual.validator()
        }
    }
    fn request_deadline(&self) -> Deadline {
        self.actual.request_deadline()
    }
    fn poll_start(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.actual.poll_start(cx)
    }
    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
        n: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        self.actual.poll_receive(cx, n)
    }
    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        if self.delivered {
            return Poll::Pending;
        }
        if self.controls.fail_send.get() {
            return Poll::Ready(Err(CoreError::Binding(ErrorContext::new(
                ErrorPhase::Binding,
                RetryClass::Never,
            ))));
        }
        let result = self.actual.poll_send(cx, reply);
        if matches!(result, Poll::Ready(Ok(()))) && self.controls.hold_delivery.get() {
            self.delivered = true;
            Poll::Pending
        } else {
            result
        }
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        input: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        cause: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        let count = self.controls.close_polls.get();
        self.controls.close_polls.set(count + 1);
        if count == 0 {
            return Poll::Ready(Err(CoreError::Binding(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))));
        }
        if count == 1 {
            return Poll::Pending;
        }
        self.actual.poll_close(cx, input, reply, cause).map(|r| {
            r.map(|delivery| {
                if self.delivered {
                    FiniteDelivery::Delivered
                } else {
                    delivery
                }
            })
        })
    }
}
impl FiniteProducerCompiler for ControlledCompiler {
    type Binding = ControlledBinding;
    fn prepare(
        &self,
        input: ResolvedProducerInput<'_>,
        key: ProducerInstanceKey,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<(Self::Binding, HttpFiniteConfig)>> {
        Ok(self
            .actual
            .prepare(input, key, budget)?
            .map(|(actual, configuration)| {
                (
                    ControlledBinding {
                        actual,
                        controls: self.controls.clone(),
                        delivered: false,
                    },
                    configuration,
                )
            }))
    }
    fn check_final(
        &self,
        input: ResolvedProducerInput<'_>,
        key: ProducerInstanceKey,
        binding: &Self::Binding,
        budget: &mut WorkBudget,
    ) -> CoreResult<Option<()>> {
        self.controls
            .final_calls
            .set(self.controls.final_calls.get() + 1);
        if self.controls.refuse_final.get() {
            return Err(CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            )));
        }
        self.actual.check_final(input, key, &binding.actual, budget)
    }
    fn retained_bytes(&self) -> usize {
        self.actual.retained_bytes()
            + std::mem::size_of::<Controls>()
            + 2 * std::mem::size_of::<usize>()
    }
}

#[test]
fn final_actual_service_loss_and_failed_pending_cleanup_retain_full_native_owner() {
    let addr = address();
    let application = app();
    let controls = Rc::new(Controls::default());
    let mut s = root(
        ControlledCompiler {
            actual: compiler(addr),
            controls: controls.clone(),
        },
        accounts(),
    );
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    until(&mut s, &mut e, RawFiniteState::InstalledClosed);
    let _peer = request(addr);
    controls.lose_service.set(true);
    step(&mut s, &mut e);
    assert_eq!(e.state(), RawFiniteState::Closing);
    assert!(s.description(e.key()).is_none());
    assert_eq!(application.calls.get(), 0);
    let first = e.first_cause().unwrap().clone();
    for _ in 0..2 {
        step(&mut s, &mut e);
        assert_eq!(e.state(), RawFiniteState::Closing);
        assert!(TcpListener::bind(addr).is_err());
    }
    step(&mut s, &mut e);
    assert_eq!(e.state(), RawFiniteState::Closed);
    assert_eq!(e.first_cause(), Some(&first));
    assert_released(addr);
}

#[test]
fn late_native_delivery_after_cancel_keeps_original_result_and_delivery() {
    let addr = address();
    let application = app();
    let controls = Rc::new(Controls::default());
    controls.hold_delivery.set(true);
    let mut s = root(
        ControlledCompiler {
            actual: compiler(addr),
            controls: controls.clone(),
        },
        accounts(),
    );
    let mut e = prepare(&s, document(addr), authority(), application, None).unwrap();
    publish(&mut s, &mut e);
    let mut peer = request(addr);
    let wire = response(&mut s, &mut e, &mut peer);
    assert!(wire.ends_with(b"42"));
    assert_eq!(e.state(), RawFiniteState::Delivering);
    e.cancel();
    let first = e.first_cause().unwrap().clone();
    until(&mut s, &mut e, RawFiniteState::Closed);
    assert_eq!(e.first_cause(), Some(&first));
    let result = e.take_settlement().unwrap();
    assert_eq!(result.delivery, FiniteDelivery::Delivered);
    assert_eq!(result.effect, FiniteEffect::Unknown);
    assert_eq!(result.result.unwrap().data().unwrap().body.as_ref(), b"42");
    assert_released(addr);
}

#[test]
fn deadline_fences_closed_readiness_before_publication_and_native_request_before_dispatch() {
    let addr = address();
    let application = app();
    let mut s = root(compiler(addr), accounts());
    let mut e = s
        .prepare(
            Arc::from(document(addr).to_string()),
            Arc::from("level"),
            authority(),
            Handler(application.clone()),
            0,
            None,
            Deadline::at(now(2)),
            &mut work(1_000_000),
        )
        .unwrap();
    until(&mut s, &mut e, RawFiniteState::InstalledClosed);
    let _ = s
        .step(&mut e, &mut cx(), &mut work(0), now(2), false)
        .unwrap();
    assert_eq!(e.state(), RawFiniteState::Closing);
    assert!(matches!(e.first_cause(), Some(CoreError::TimedOut(_))));
    let _ = s
        .step(&mut e, &mut cx(), &mut work(1_000_000), now(2), false)
        .unwrap();
    assert_released(addr);
    drop(e);
    drop(s);
    let mut s = root(compiler(addr), accounts());
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    publish(&mut s, &mut e);
    let mut peer = TcpStream::connect(addr).unwrap();
    peer.write_all(b"GET /value HTTP/1.1\r\n").unwrap();
    step(&mut s, &mut e);
    let _ = s
        .step(&mut e, &mut cx(), &mut work(0), now(100), false)
        .unwrap();
    assert_eq!(e.state(), RawFiniteState::Closing);
    assert_eq!(application.calls.get(), 0);
    assert!(s.description(e.key()).is_none());
    let _ = s
        .step(&mut e, &mut cx(), &mut work(1_000_000), now(100), false)
        .unwrap();
    assert_released(addr);
}

#[test]
fn final_support_refusal_retains_closed_setup_and_never_publishes() {
    let addr = address();
    let application = app();
    let controls = Rc::new(Controls::default());
    let mut s = root(
        ControlledCompiler {
            actual: compiler(addr),
            controls: controls.clone(),
        },
        accounts(),
    );
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    until(&mut s, &mut e, RawFiniteState::InstalledClosed);
    let _peer = request(addr);
    controls.refuse_final.set(true);
    step(&mut s, &mut e);
    assert_eq!(controls.final_calls.get(), 1);
    assert_eq!(e.state(), RawFiniteState::Closing);
    assert!(matches!(
        e.first_cause(),
        Some(CoreError::UnsupportedOperation(_))
    ));
    assert!(s.description(e.key()).is_none());
    assert_eq!(application.calls.get(), 0);
    until(&mut s, &mut e, RawFiniteState::Closed);
    assert_released(addr);
}

#[test]
fn cleanup_saturation_refuses_before_native_effects_and_preserves_original_bank() {
    let addr = address();
    let bank = accounts();
    let application = app();
    let s = root(compiler(addr), bank.clone());
    let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let full = bank
        .reserve(
            ResourceKind::CleanupItemsMax,
            bank.account(ResourceKind::CleanupItemsMax).limit(),
        )
        .unwrap();
    assert!(matches!(
        prepare(&s, document(addr), authority(), application.clone(), None),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        baseline
    );
    assert_released(addr);
    drop(full);
    let e = prepare(&s, document(addr), authority(), application, None).unwrap();
    drop(e);
    drop(s);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
}

#[test]
fn publication_releases_qualification_scratch_for_another_producer() {
    let first_addr = address();
    let first_app = app();
    let second_app = app();
    let bank = accounts();
    let mut first = root(compiler(first_addr), bank.clone());
    let mut exposed = prepare(
        &first,
        document(first_addr),
        authority(),
        first_app.clone(),
        None,
    )
    .unwrap();
    until(&mut first, &mut exposed, RawFiniteState::InstalledClosed);
    // The first listener now occupies its endpoint, so the second is distinct.
    let second_addr = address();
    let mut second = root(compiler(second_addr), bank.clone());
    let scratch = bank
        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .used();
    assert!(scratch > 0);
    let other_bytes = bank
        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .limit()
        - scratch;
    let other_work = bank
        .reserve(ResourceKind::AdmissionTemporaryBytesGlobalMax, other_bytes)
        .unwrap();
    let engine_before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let cleanup_before = bank.account(ResourceKind::CleanupBytesMax).used();

    publish(&mut first, &mut exposed);
    let completed_scratch = bank
        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .used()
        - other_bytes;
    let engine_after = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let cleanup_after = bank.account(ResourceKind::CleanupBytesMax).used();
    let prepared = prepare(
        &second,
        document(second_addr),
        authority(),
        second_app.clone(),
        None,
    );
    assert_eq!(
        completed_scratch, 0,
        "completed qualification still holds {completed_scratch} temporary bytes"
    );
    assert_eq!(engine_after, engine_before - scratch);
    assert_eq!(cleanup_after, cleanup_before);
    let mut prepared = prepared.unwrap_or_else(|error| {
        panic!("second Producer refused after completed qualification: {error:?}")
    });
    assert_eq!(exposed.state(), RawFiniteState::Serving);
    assert_eq!(first_app.drops.get(), 0);
    assert!(bank.account(ResourceKind::CleanupBytesMax).used() > cleanup_before);
    // Warm I/O still works while the second preparation uses the released
    // scratch and unrelated work occupies the rest of the temporary bank.
    assert!(response(&mut first, &mut exposed, &mut request(first_addr)).ends_with(b"42"));
    assert_eq!(
        exposed.take_settlement().unwrap().delivery,
        FiniteDelivery::Delivered
    );
    assert_eq!(first_app.calls.get(), 1);
    close(&mut second, &mut prepared);
    second.retire(&mut prepared, &mut work(1)).unwrap();
    close(&mut first, &mut exposed);
    first.retire(&mut exposed, &mut work(1)).unwrap();
    drop((prepared, exposed, first, second, other_work));
    assert_eq!(
        bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
            .used(),
        0
    );
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), 0);
    assert_released(first_addr);
    assert_released(second_addr);
}

#[test]
fn qualification_scratch_stays_reserved_while_setup_can_retry() {
    let addr = address();
    let bank = accounts();
    let application = app();
    let mut s = root(compiler(addr), bank.clone());
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    let scratch = bank
        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .used();
    let engine = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let cleanup = bank.account(ResourceKind::CleanupBytesMax).used();
    assert!(scratch > 0);
    let _ = s
        .step(&mut e, &mut cx(), &mut work(0), now(0), false)
        .unwrap();
    assert_eq!(e.state(), RawFiniteState::Starting);
    assert_eq!(
        bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
            .used(),
        scratch
    );
    until(&mut s, &mut e, RawFiniteState::InstalledClosed);
    // One attempt cannot decode; another completes TD interpretation but
    // cannot pay the actual Binding compiler's complete lowering envelope.
    for mut budget in [
        work(0),
        work(1_000_000).with_remaining(WorkClass::BindingPolls, 4095),
    ] {
        assert_eq!(
            s.step(&mut e, &mut cx(), &mut budget, now(0), false)
                .unwrap(),
            Poll::Pending
        );
        assert_eq!(e.state(), RawFiniteState::InstalledClosed);
        assert_eq!(
            bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                .used(),
            scratch
        );
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            engine
        );
        assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), cleanup);
        assert_eq!(application.calls.get(), 0);
    }
    publish(&mut s, &mut e);
    assert_eq!(
        bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
            .used(),
        0
    );
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        engine - scratch
    );
    assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), cleanup);
    close(&mut s, &mut e);
    s.retire(&mut e, &mut work(1)).unwrap();
    drop((e, s));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    assert_released(addr);
}

#[test]
fn terminal_setup_releases_scratch_but_failed_cleanup_keeps_lifetime_leases() {
    for case in 0..6 {
        let occupied = (case == 3).then(|| TcpListener::bind("127.0.0.1:0").unwrap());
        let addr = occupied
            .as_ref()
            .map_or_else(address, |listener| listener.local_addr().unwrap());
        let bank = accounts();
        let application = app();
        let controls = Rc::new(Controls::default());
        let mut s = root(
            ControlledCompiler {
                actual: compiler(addr),
                controls: controls.clone(),
            },
            bank.clone(),
        );
        let deadline = if case == 2 {
            Deadline::at(now(1))
        } else {
            Deadline::NONE
        };
        let mut e = s
            .prepare(
                Arc::from(document(addr).to_string()),
                Arc::from("level"),
                authority(),
                Handler(application.clone()),
                0,
                None,
                deadline,
                &mut work(1_000_000),
            )
            .unwrap();
        let scratch = bank
            .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
            .used();
        let engine = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        let cleanup = bank.account(ResourceKind::CleanupBytesMax).used();
        assert!(scratch > 0);
        if matches!(case, 1 | 4 | 5) {
            until(&mut s, &mut e, RawFiniteState::InstalledClosed);
        }
        match case {
            0 => e.begin_close(),
            1 => e.cancel(),
            2 => {
                let _ = s
                    .step(&mut e, &mut cx(), &mut work(0), now(1), false)
                    .unwrap();
            }
            3 => step(&mut s, &mut e), // Real native bind failure.
            4 => {
                controls.refuse_final.set(true);
                step(&mut s, &mut e);
            }
            _ => {
                controls.lose_service.set(true);
                step(&mut s, &mut e);
            }
        }
        assert_eq!(e.state(), RawFiniteState::Closing, "case {case}");
        assert!(s.description(e.key()).is_none());
        assert_eq!(
            bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                .used(),
            0,
            "case {case}"
        );
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            engine - scratch,
            "case {case}"
        );
        assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), cleanup);
        assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 1);
        assert_eq!(application.drops.get(), 0);
        assert_eq!(application.calls.get(), 0);
        let instant = now(u64::from(case == 2));
        let mut cause = e.first_cause().cloned();
        for attempt in 0..3 {
            let _ = s
                .step(&mut e, &mut cx(), &mut work(1_000_000), instant, false)
                .unwrap();
            if cause.is_none() {
                cause = e.first_cause().cloned();
            }
            assert_eq!(e.first_cause(), cause.as_ref());
            assert_eq!(
                e.state(),
                if attempt < 2 {
                    RawFiniteState::Closing
                } else {
                    RawFiniteState::Closed
                }
            );
            assert_eq!(
                bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                    .used(),
                0
            );
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                engine - scratch
            );
            assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), cleanup);
            assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 1);
        }
        s.retire(&mut e, &mut work(1)).unwrap();
        drop((e, s, occupied));
        assert_eq!(application.drops.get(), 1);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            0
        );
        assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), 0);
        assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 0);
        assert_released(addr);
    }
}

#[test]
fn staged_root_loss_cleanup_and_exposure_drop_release_scratch_once() {
    for drop_exposure in [false, true] {
        let addr = address();
        let bank = accounts();
        let application = app();
        let controls = Rc::new(Controls::default());
        let mut s = root(
            ControlledCompiler {
                actual: compiler(addr),
                controls: controls.clone(),
            },
            bank.clone(),
        );
        let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
        until(&mut s, &mut e, RawFiniteState::InstalledClosed);
        assert!(
            bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                .used()
                > 0
        );
        if drop_exposure {
            drop(e);
            assert_eq!(
                bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                    .used(),
                0
            );
            assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 0);
            drop(s);
        } else {
            drop(s);
            // An unfunded cleanup poll still ends qualification, without
            // polling native retirement or refunding its remaining owners.
            let _ = e.poll_cleanup(&mut cx(), &mut work(0), now(0));
            assert_eq!(e.state(), RawFiniteState::Closing);
            assert_eq!(controls.close_polls.get(), 0);
            assert_eq!(
                bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                    .used(),
                0
            );
            assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 1);
            assert_eq!(application.drops.get(), 0);
            for _ in 0..3 {
                let _ = e.poll_cleanup(&mut cx(), &mut work(1_000_000), now(0));
            }
            assert_eq!(e.state(), RawFiniteState::Closed);
            drop(e);
        }
        assert_eq!(application.drops.get(), 1);
        assert_eq!(
            bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                .used(),
            0
        );
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            0
        );
        assert_eq!(bank.account(ResourceKind::CleanupBytesMax).used(), 0);
        assert_released(addr);
    }
}

#[test]
fn unread_settlement_blocks_next_exchange_until_acknowledgement() {
    let addr = address();
    let application = app();
    let mut s = root(compiler(addr), accounts());
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    publish(&mut s, &mut e);
    assert!(response(&mut s, &mut e, &mut request(addr)).ends_with(b"42"));
    let mut second = request(addr);
    for _ in 0..10 {
        step(&mut s, &mut e);
    }
    assert_eq!(application.calls.get(), 1);
    e.take_settlement().unwrap();
    assert!(response(&mut s, &mut e, &mut second).ends_with(b"42"));
    assert_eq!(application.calls.get(), 2);
    e.take_settlement().unwrap();
    close(&mut s, &mut e);
}

#[test]
fn delivery_failure_after_dispatch_preserves_original_value_and_funded_cleanup() {
    let addr = address();
    let bank = accounts();
    let application = app();
    let controls = Rc::new(Controls::default());
    let mut s = root(
        ControlledCompiler {
            actual: compiler(addr),
            controls: controls.clone(),
        },
        bank.clone(),
    );
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    publish(&mut s, &mut e);
    let _peer = request(addr);
    until(&mut s, &mut e, RawFiniteState::Delivering);
    controls.fail_send.set(true);
    step(&mut s, &mut e);
    assert!(s.description(e.key()).is_none());
    let first = e.first_cause().unwrap().clone();
    let retained = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    for _ in 0..2 {
        step(&mut s, &mut e);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            retained
        );
    }
    step(&mut s, &mut e);
    assert_eq!(e.state(), RawFiniteState::Closed);
    assert_eq!(e.first_cause(), Some(&first));
    let result = e.take_settlement().unwrap();
    assert_eq!(result.effect, FiniteEffect::Unknown);
    assert_eq!(result.delivery, FiniteDelivery::NotDelivered);
    assert_eq!(result.result.unwrap().data().unwrap().body.as_ref(), b"42");
    assert_eq!(application.calls.get(), 1);
    assert_released(addr);
}

#[test]
fn real_partial_write_then_cancel_reports_uncertainty_with_complete_original_value() {
    static BODY: [u8; 4096] = {
        let mut body = [b' '; 4096];
        body[0] = b'1';
        body
    };
    let addr = address();
    let application = app();
    application.body.set(&BODY);
    let mut s = root(compiler(addr), accounts());
    let mut e = prepare(&s, document(addr), authority(), application.clone(), None).unwrap();
    publish(&mut s, &mut e);
    let mut peer = request(addr);
    until(&mut s, &mut e, RawFiniteState::Delivering);
    step(&mut s, &mut e);
    assert_eq!(e.state(), RawFiniteState::Delivering);
    e.cancel();
    until(&mut s, &mut e, RawFiniteState::Closed);
    let result = e.take_settlement().unwrap();
    assert_eq!(result.delivery, FiniteDelivery::Unknown);
    assert_eq!(result.effect, FiniteEffect::Unknown);
    assert_eq!(result.result.unwrap().data().unwrap().body.as_ref(), &BODY);
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes).unwrap();
    assert!(!bytes.is_empty());
    assert!(!bytes.ends_with(&BODY));
    assert_eq!(application.calls.get(), 1);
    assert_released(addr);
}

#[test]
fn public_consumer_uses_exact_published_description_and_real_native_peer() {
    use clinkz_wot_servient::consumer_property_read::*;
    struct Clock;
    impl RuntimeClock for Clock {
        fn now(&self) -> MonotonicInstant {
            now(0)
        }
        fn ticks_per_second(&self) -> std::num::NonZeroU64 {
            std::num::NonZeroU64::new(1000).unwrap()
        }
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let addr = address();
            let application = app();
            let mut server = root(compiler(addr), accounts());
            let mut value = document(addr);
            value["id"] = json!("urn:joined:producer");
            let mut exposure =
                prepare(&server, value, authority(), application.clone(), None).unwrap();
            publish(&mut server, &mut exposure);
            let policy = ConsumerPropertyReadPolicy::try_new_owned(
                BindingResourceContract::ConsumerOwnedHostV2,
                ResourceProfileId::GATEWAY_DEFAULT_V1,
                GatewayDefaultV1::LIMITS.clone(),
                BindingLifetimeFootprint::new(8, 8192),
                BindingLifetimeFootprint::new(8, 4096),
                [1, 64, 64, 64],
                [4096, 16],
                [2, 2, 1, 1, 1, 1, 1],
                [30_000, 1000],
                65536,
            )
            .unwrap();
            let bank = Arc::new(
                HostResourceAccounts::try_from_accounts(policy.parent_accounts()).unwrap(),
            );
            let registration = try_inline_consumer_registration(
                HttpConsumerConfig::for_cleartext_ip().unwrap(),
                BindingId::new(2),
                BindingGeneration::INITIAL,
                0,
            )
            .unwrap();
            let (client, observer) = ConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                Arc::new(Clock),
                bank.clone(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
            let td: clinkz_wot_td::thing::Thing =
                serde_json::from_str(server.description(exposure.key()).unwrap()).unwrap();
            let handle = match client
                .consume(&td, &mut work(1_000_000), |build| {
                    for _ in 0..10000 {
                        build.step(&mut work(1_000_000));
                        if build.is_finished() {
                            return;
                        }
                    }
                    panic!("Consumer preparation stalled");
                })
                .unwrap()
            {
                StartStatus::Ready(handle) => handle,
                _ => panic!("Consumer not ready"),
            };
            drop(td);
            for _ in 0..3 {
                let mut call = handle
                    .read_property("level", InteractionOptions::new(), &mut work(1_000_000))
                    .unwrap();
                let end = Instant::now() + Duration::from_secs(5);
                let output = loop {
                    assert!(Instant::now() < end);
                    step(&mut server, &mut exposure);
                    call.step(&mut cx(), &mut work(1_000_000));
                    if let Some(result) = call.try_take(&mut work(1_000_000)).unwrap() {
                        break result.unwrap();
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                };
                assert_eq!(output.data().unwrap().body.as_ref(), b"42");
                assert_eq!(
                    exposure.take_settlement().unwrap().delivery,
                    FiniteDelivery::Delivered
                );
            }
            drop(handle);
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                if let StartStatus::Ready(outcome) =
                    client.shutdown(&mut cx(), &mut work(1_000_000))
                {
                    assert!(outcome.is_complete());
                    break;
                }
                assert!(Instant::now() < end);
            }
            drop(client);
            drop(observer);
            assert!(
                bank.snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                    .iter()
                    .all(|a| a.used() == 0)
            );
            close(&mut server, &mut exposure);
            assert_eq!(application.calls.get(), 3);
            assert_released(addr);
        });
}
