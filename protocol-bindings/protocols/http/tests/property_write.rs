#![cfg(feature = "prepare")]
//! Real TD-prepared Engine Consumer -> completely qualified Engine Producer.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{consumer::*, finite::*, host_accounts::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{open_consumer::*, producer_composition::*};
use clinkz_wot_servient::producer_composition::*;
use clinkz_wot_servient_finite::{consumer::*, *};
use clinkz_wot_td::{ValidatedProducerScope, thing::Thing};
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
fn now(t: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(97), t)
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
fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
}
fn clean(bank: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(bank.account(k).used(), 0, "{k:?}");
    }
}
fn payload(bytes: &[u8]) -> Payload {
    Payload::new(bytes, "application/json")
}
fn description(addr: SocketAddr) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"writable level",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer","forms":[{"href":format!("http://{addr}/level")}]}}})
}
fn authority(generation: Generation) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(0), generation)).unwrap()
}
fn consumer_instance() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL))
}
fn io() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
}
#[derive(Default)]
struct Application {
    value: RefCell<Vec<u8>>,
    reads: Cell<usize>,
    writes: Cell<usize>,
    denies: Cell<bool>,
    output: Cell<u8>,
    drops: Cell<usize>,
}
struct Behavior(Rc<Application>);
impl Drop for Behavior {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl RawFiniteHandler for Behavior {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        if input.data.is_some() && self.0.denies.get() {
            return Err(denied());
        }
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        match ctx.operation() {
            Operation::ReadProperty => {
                self.0.reads.set(self.0.reads.get() + 1);
                Ok(InteractionOutput::with_data(payload(
                    &self.0.value.borrow(),
                )))
            }
            Operation::WriteProperty => {
                self.0.writes.set(self.0.writes.get() + 1);
                *self.0.value.borrow_mut() = input.data.as_ref().unwrap().body.to_vec();
                match self.0.output.get() {
                    1 => Err(CoreError::Binding(ErrorContext::new(
                        ErrorPhase::Handler,
                        RetryClass::Never,
                    ))),
                    2 => Ok(InteractionOutput::with_data(payload(b"7"))),
                    3 => Ok(InteractionOutput::empty().with_status(InteractionStatus::Accepted)),
                    _ => Ok(InteractionOutput::empty()),
                }
            }
            _ => unreachable!(),
        }
    }
}
fn denied() -> CoreError {
    CoreError::Security {
        reason: SecurityFailureReason::AuthorizationDenied,
        context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never),
    }
}

// Controls only pause/fence the actual native participant. All matching, body
// decoding, I/O and delivery still run through the production HTTP Binding.
#[derive(Default)]
struct Controls {
    hold_send: Cell<bool>,
    hold_close: Cell<bool>,
    lose_service: Cell<bool>,
}
struct Native {
    actual: HttpFiniteBinding,
    controls: Rc<Controls>,
}
impl FiniteIngressBinding for Native {
    type Configuration = HttpFiniteConfig;
    type Reply = HttpFiniteReply;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.actual.identity()
    }
    fn configuration(&self) -> &HttpFiniteConfig {
        self.actual.configuration()
    }
    fn engine_backing_bytes(&self) -> Option<usize> {
        self.actual
            .engine_backing_bytes()?
            .checked_add(size_of::<Controls>() + 2 * size_of::<usize>())
    }
    fn prepared_for_instance(&self, key: ProducerInstanceKey) -> bool {
        self.actual.prepared_for_instance(key)
    }
    fn covers_definition(
        &self,
        key: ProducerInstanceKey,
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        op: Operation,
        d: &dyn Any,
    ) -> bool {
        self.actual.covers_definition(key, site, form, op, d)
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
        now: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        self.actual.poll_receive(cx, now)
    }
    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        if self.controls.hold_send.get() {
            Poll::Pending
        } else {
            self.actual.poll_send(cx, reply)
        }
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        ingress: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        cause: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        if self.controls.hold_close.get() {
            Poll::Pending
        } else {
            self.actual.poll_close(cx, ingress, reply, cause)
        }
    }
}
type Server = RawFiniteRuntime<Native>;
fn exposure(
    owner: &RawProducerServient,
    accounts: Arc<HostResourceAccounts>,
    td: Value,
    app: Rc<Application>,
    controls: Rc<Controls>,
    authority: ProducerInstanceAuthority,
    expected: Option<ProducerInstanceKey>,
) -> ComposedProducerExposure {
    let addr = td["properties"]["level"]["forms"][0]["href"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap()
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let key = authority.key();
    let read = td["properties"]["level"]["writeOnly"] != true;
    let write = td["properties"]["level"]["readOnly"] != true;
    let behaviors = [
        (Operation::ReadProperty, read),
        (Operation::WriteProperty, write),
    ]
    .into_iter()
    .filter(|(_, yes)| *yes)
    .map(|(op, _)| {
        RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("level")),
            op,
            Behavior(app.clone()),
        )
    })
    .collect();
    let instance =
        RawProducerInstance::new(authority, behaviors).with_description(Arc::from(td.to_string()));
    let provider: Rc<dyn ProducerCapability> = Rc::new(
        HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap(),
    );
    let prepared = prepare_thing(instance.description().unwrap(), key, GatewayDefaultV1::LIMITS,
        &mut work(), &[provider.as_ref()], |scope, op| matches!(scope, ValidatedProducerScope::Property(n) if instance.supports_property(n, op))).unwrap();
    let mappings = prepared
        .operations()
        .iter()
        .map(|o| {
            (
                Arc::from("level"),
                o.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    let config = HttpFiniteConfig::try_from_mappings(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        mappings,
        key,
        1000,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let server = RawFiniteRuntime::try_new(
        Native {
            actual: HttpFiniteBinding::new(
                config.clone(),
                BindingId::new(97),
                BindingGeneration::INITIAL,
            ),
            controls,
        },
        config,
        owner,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 4096,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let result = ComposedProducerExposure::try_new(
        owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![provider],
            participants: vec![Box::new(server)],
            expected,
            handler_backing_bytes: size_of::<Application>() + 8192,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts,
        &mut work(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop(td);
    result
}
struct Fixture {
    owner: RawProducerServient,
    exposure: ComposedProducerExposure,
    bank: Arc<HostResourceAccounts>,
    app: Rc<Application>,
    controls: Rc<Controls>,
    addr: SocketAddr,
}
impl Fixture {
    fn new(td: Value) -> Self {
        let bank = bank();
        let owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
            instances: 1,
            behaviors_per_instance: 2,
            target_name_bytes: 256,
        })
        .unwrap();
        let app = Rc::new(Application::default());
        *app.value.borrow_mut() = b"0".to_vec();
        let controls = Rc::new(Controls::default());
        let mut exposure = exposure(
            &owner,
            bank.clone(),
            td,
            app.clone(),
            controls.clone(),
            authority(Generation::INITIAL),
            None,
        );
        let server = exposure.participant_mut::<Server>(0).unwrap();
        let _ = server.step(&mut cx(), &mut work(), now(0));
        let addr = server
            .closed_binding()
            .unwrap()
            .actual
            .bound_address()
            .unwrap();
        // Closed native setup has not published the TD or granted dispatch.
        assert!(owner.description(exposure.key()).is_none());
        let mut result = Self {
            owner,
            exposure,
            bank,
            app,
            controls,
            addr,
        };
        result
            .exposure
            .publish(&mut result.owner, &mut work())
            .unwrap();
        result
    }
    fn server(&mut self) -> &mut Server {
        self.exposure.participant_mut::<Server>(0).unwrap()
    }
    fn step(&mut self) {
        let _ = self
            .owner
            .step(
                self.exposure.participant_mut::<Server>(0).unwrap(),
                &mut cx(),
                &mut work(),
                now(0),
            )
            .unwrap();
    }
    async fn exchange(
        &mut self,
        call: &mut ConsumerCall,
    ) -> (ConsumerSettlement, RawFiniteSettlement) {
        let start = Instant::now();
        let result = loop {
            assert!(start.elapsed() < Duration::from_secs(5), "exchange stalled");
            self.step();
            if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        while self.server().state() == RawFiniteState::Closing {
            self.step();
        }
        (
            result,
            self.server()
                .take_settlement()
                .expect("retained Producer settlement"),
        )
    }
    fn close(mut self) {
        self.exposure.begin_close();
        for _ in 0..100 {
            self.step();
            if self.server().state() == RawFiniteState::Closed {
                break;
            }
        }
        assert_eq!(self.server().state(), RawFiniteState::Closed);
        assert!(self.server().take_settlement().is_none());
        self.exposure.retire(&mut self.owner, &mut work()).unwrap();
        drop(self.exposure);
        drop(self.owner);
        clean(&self.bank);
        drop(TcpListener::bind(self.addr).unwrap());
    }
}
struct Access(Arc<AtomicBool>);
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        if self.0.load(Ordering::Relaxed) {
            Err(denied())
        } else {
            Ok(())
        }
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        t: &str,
        o: Operation,
    ) -> CoreResult<()> {
        self.authorize(b, t, o)
    }
    fn retained_bytes(&self) -> u64 {
        64
    }
    fn work(&self) -> u64 {
        1
    }
}
fn client(
    td: Value,
    denied: Arc<AtomicBool>,
    accounts: Arc<HostResourceAccounts>,
    owner: &ConsumerInstance,
) -> ConsumerRuntime {
    let mut registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(98),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap();
    registration.services.access = Arc::new(Access(denied));
    let registry =
        ConsumerRegistry::try_new(vec![registration]).unwrap_or_else(|e| panic!("{:?}", e.error()));
    let td: Thing = serde_json::from_value(td).unwrap();
    let prepared = registry
        .prepare(
            &td,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        )
        .unwrap();
    let (definitions, funding) = prepared.into_execution();
    drop(td);
    drop(registry);
    ConsumerRuntime::try_new(owner.reference(), definitions, funding, accounts, 2)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn start_write(client: &ConsumerRuntime, bytes: &[u8]) -> ConsumerCall {
    client
        .write_property("level", payload(bytes), Deadline::NONE, now(0), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}

#[test]
fn default_and_explicit_read_write_definitions_coexist_on_one_property_and_listener() {
    io().block_on(async {
        for layout in 0..3 {
            let addr = address(); let mut td = description(addr);
            match layout {
                1 => td["properties"]["level"]["forms"][0]["op"] = json!(["readproperty","writeproperty"]),
                2 => td["properties"]["level"]["forms"] = json!([
                    {"href":format!("http://{addr}/level"),"op":"readproperty","htv:methodName":"GET","response":{"contentType":"application/json","htv:statusCodeValue":200}},
                    {"href":format!("http://{addr}/level"),"op":"writeproperty","htv:methodName":"PUT","response":{"contentType":"application/json","htv:statusCodeValue":204}}]),
                _ => {}
            }
            let mut server = Fixture::new(td.clone());
            let bank = bank(); let owner = consumer_instance();
            let client = client(td, Arc::new(AtomicBool::new(false)), bank.clone(), &owner);
            for value in [b"12".as_slice(),b"13.0",b"14e0"] {
                let mut write = start_write(&client, value);
                let (result, remote) = server.exchange(&mut write).await;
                assert_eq!(result.operation(), Operation::WriteProperty);
                assert!(result.result().unwrap().data().is_none());
                assert_eq!(result.result().unwrap().metadata().binding_response().unwrap().status_code(),204);
                assert_eq!(result.effect(), ConsumerEffect::Unknown, "remote certainty remains independent of reported completion");
                assert_eq!(remote.effect, FiniteEffect::Unknown);
                assert_eq!(remote.delivery, FiniteDelivery::Delivered);
                assert!(remote.result.unwrap().data().is_none());
                drop(result); drop(write);
                let mut read = client.read_property("level", Deadline::NONE, now(0), &mut work()).unwrap();
                let (result, remote) = server.exchange(&mut read).await;
                assert_eq!(result.operation(), Operation::ReadProperty);
                assert_eq!(result.result().unwrap().data().unwrap().body.as_ref(),value);
                assert_eq!(remote.operation,Some(Operation::ReadProperty));
            }
            assert_eq!(server.app.writes.get(),3); assert_eq!(server.app.reads.get(),3);
            drop(client); clean(&bank); server.close();
        }
    });
}

#[test]
fn invalid_unpaid_and_denied_consumer_input_returns_original_before_io() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let mut server = Fixture::new(td.clone());
        let bank = bank();
        let owner = consumer_instance();
        let denied = Arc::new(AtomicBool::new(false));
        let client = client(td, denied.clone(), bank.clone(), &owner);
        let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        for value in [
            payload(b"true"),
            payload(b"not-json"),
            payload(&vec![b'1'; 4097]),
            Payload::new(b"7".as_slice(), "text/plain"),
            payload(b"7").with_content_coding("gzip"),
        ] {
            let original = value.clone();
            let error = client
                .write_property("level", value, Deadline::NONE, now(0), &mut work())
                .err()
                .unwrap();
            assert_eq!(
                error.error().error().context().retry_class(),
                RetryClass::Never
            );
            let returned = error.into_input();
            assert!(Arc::ptr_eq(&returned.body, &original.body));
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                baseline
            );
        }
        let mut short = work();
        short.set_remaining(WorkClass::CodecInputBytes, 0);
        assert!(
            client
                .write_property("level", payload(b"7"), Deadline::NONE, now(0), &mut short)
                .is_err()
        );
        denied.store(true, Ordering::Relaxed);
        assert!(
            client
                .write_property("level", payload(b"7"), Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        for _ in 0..10 {
            server.step();
        }
        assert_eq!(server.app.writes.get(), 0);
        assert!(server.server().take_settlement().is_none());
        denied.store(false, Ordering::Relaxed);
        server.app.denies.set(true);
        let mut call = start_write(&client, b"7");
        let (local, remote) = server.exchange(&mut call).await;
        assert!(local.result().is_err());
        assert_eq!(local.effect(), ConsumerEffect::Unknown);
        assert!(matches!(remote.result, Err(CoreError::Security { .. })));
        assert_eq!(remote.effect, FiniteEffect::NotInvoked);
        assert_eq!(server.app.writes.get(), 0);
        drop(local);
        drop(call);
        drop(client);
        clean(&bank);
        server.close();
    });
}

#[test]
fn handler_failure_and_invalid_success_do_not_erase_committed_write_or_trigger_replay() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let mut server = Fixture::new(td.clone());
        let bank = bank();
        let owner = consumer_instance();
        let client = client(td, Arc::new(AtomicBool::new(false)), bank.clone(), &owner);
        for mode in 1..=3 {
            server.app.output.set(mode);
            let mut call = start_write(&client, b"22");
            let (local, remote) = server.exchange(&mut call).await;
            assert!(local.result().is_err());
            assert_eq!(local.effect(), ConsumerEffect::Unknown);
            assert!(remote.result.is_err());
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert_eq!(*server.app.value.borrow(), b"22");
            assert_eq!(server.app.writes.get(), usize::from(mode));
            for _ in 0..10 {
                server.step();
            }
            assert_eq!(server.app.writes.get(), usize::from(mode));
        }
        drop(client);
        clean(&bank);
        server.close();
    });
}

#[test]
fn complete_producer_qualification_refuses_unsupported_or_contradictory_advertisement_before_setup()
{
    for case in 0..18 {
        let addr = address();
        let mut td = description(addr);
        let p = &mut td["properties"]["level"];
        match case {
            0 => p["minimum"] = json!(0),
            1 => p["enum"] = json!([1, 2]),
            2 => p["const"] = json!(1),
            3 => {
                p["readOnly"] = json!(true);
                p["forms"][0]["op"] = json!("writeproperty");
            }
            4 => {
                p["writeOnly"] = json!(true);
                p["forms"][0]["op"] = json!("readproperty");
            }
            5 => p["forms"][0]["op"] = json!(["readproperty", "writeproperty", "observeproperty"]),
            6 => p["forms"][0]["contentType"] = json!("application/cbor"),
            7 => p["forms"][0]["contentCoding"] = json!("gzip"),
            8 => p["forms"][0]["htv:methodName"] = json!("POST"),
            9 => p["forms"][0]["htv:methodName"] = json!("GET"),
            10 => {
                p["forms"][0]["response"] =
                    json!({"contentType":"application/json","htv:statusCodeValue":202})
            }
            11 => p["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"href":"unknown:unsupported","op":"writeproperty"})),
            12 => p["forms"][0]["op"] = json!(["readproperty", "writeproperty"]),
            13 => {
                td["securityDefinitions"]["n"] = json!({"scheme":"basic","in":"header"});
            }
            14 => {
                td["securityDefinitions"]["b"] = json!({"scheme":"basic","in":"header"});
                td["properties"]["level"]["forms"][0]["security"] = json!(["b"]);
            }
            15 => p["format"] = json!("int32"),
            16 => p["oneOf"] = json!([{"type":"integer"},{"type":"string"}]),
            17 => {
                p["type"] = json!("object");
                p["required"] = json!(["x"]);
            }
            _ => unreachable!(),
        }
        if case == 12 {
            td["properties"]["level"]["forms"][0]["htv:methodName"] = json!("PUT");
        }
        let a = authority(Generation::INITIAL);
        let provider = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap();
        assert!(
            prepare_thing(
                &td.to_string(),
                a.key(),
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |_, _| true
            )
            .is_err(),
            "case {case}"
        );
        drop(TcpListener::bind(addr).unwrap());
    }
}

#[test]
fn read_only_write_only_and_explicit_operation_applicability_gate_consumer_calls() {
    io().block_on(async {
        for write_only in [false, true] {
            let addr = address();
            let mut td = description(addr);
            td["properties"]["level"][if write_only { "writeOnly" } else { "readOnly" }] =
                json!(true);
            let mut server = Fixture::new(td.clone());
            let bank = bank();
            let owner = consumer_instance();
            let client = client(td, Arc::new(AtomicBool::new(false)), bank.clone(), &owner);
            if write_only {
                assert!(
                    client
                        .read_property("level", Deadline::NONE, now(0), &mut work())
                        .is_err()
                );
                let mut call = start_write(&client, b"8");
                let (local, remote) = server.exchange(&mut call).await;
                assert!(local.result().is_ok());
                assert!(remote.result.is_ok());
            } else {
                assert!(
                    client
                        .write_property("level", payload(b"8"), Deadline::NONE, now(0), &mut work())
                        .is_err()
                );
                let mut call = client
                    .read_property("level", Deadline::NONE, now(0), &mut work())
                    .unwrap();
                let (local, remote) = server.exchange(&mut call).await;
                assert!(local.result().is_ok());
                assert!(remote.result.is_ok());
            }
            drop(client);
            clean(&bank);
            server.close();
        }
        for case in 0..6 {
            let addr = address();
            let mut td = description(addr);
            let mut write = json!({"href":format!("http://{addr}/level"),"op":"writeproperty"});
            match case {
                0 => td["properties"]["level"]["readOnly"] = json!(true),
                1 => write["op"] = json!("readproperty"),
                2 => write["htv:methodName"] = json!("GET"),
                3 => {
                    write["response"] =
                        json!({"contentType":"application/json","htv:statusCodeValue":200})
                }
                4 => {
                    td["securityDefinitions"]["b"] = json!({"scheme":"basic","in":"header"});
                    write["security"] = json!(["b"]);
                }
                5 => write["contentType"] = json!("text/plain"),
                _ => unreachable!(),
            }
            td["properties"]["level"]["forms"] = json!([
                {"href":format!("http://{addr}/level"),"op":"readproperty"},write]);
            let bank = bank();
            let owner = consumer_instance();
            let client = client(td, Arc::new(AtomicBool::new(false)), bank.clone(), &owner);
            assert!(
                client
                    .write_property("level", payload(b"8"), Deadline::NONE, now(0), &mut work())
                    .is_err(),
                "case {case}"
            );
            drop(client);
            clean(&bank);
            drop(TcpListener::bind(addr).unwrap());
        }
    });
}

#[test]
fn consumer_requires_the_prepared_native_write_completion_and_never_replays_uncertain_effects() {
    for reply in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7",
        b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\n\r\n7",
        b"HTTP/1.1 204 No Content\r\nContent-Type: text/plain\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\nContent-Encoding: gzip\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\nContent-Type: application/json\r\nContent-Type: application/json\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\n",
        b"", b"HTTP/1.1 204 No Content\r\n\r\n",
    ] {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap(); let addr=listener.local_addr().unwrap();
        let peer=std::thread::spawn(move || {
            let (mut stream,_)=listener.accept().unwrap(); stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut bytes=Vec::new();let mut buf=[0;4096];
            while !bytes.ends_with(b"\r\n\r\n7") { let n=stream.read(&mut buf).unwrap();assert!(n>0);bytes.extend_from_slice(&buf[..n]); }
            assert!(bytes.starts_with(b"PUT /level HTTP/1.1\r\n"));
            stream.write_all(reply).unwrap(); drop(stream);listener
        });
        let bank=bank();let owner=consumer_instance();
        let client=client(description(addr),Arc::new(AtomicBool::new(false)),bank.clone(),&owner);
        io().block_on(async {
            let mut call=start_write(&client,b"7");let start=Instant::now();
            let result=loop {
                if let Poll::Ready(s)=call.poll(&mut cx(),now(0),&mut work()) {break s;}
                assert!(start.elapsed()<Duration::from_secs(5));tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(result.result().is_ok(),reply==b"HTTP/1.1 204 No Content\r\n\r\n","{:?}",String::from_utf8_lossy(reply));
            assert_eq!(result.effect(),ConsumerEffect::Unknown);
            for _ in 0..10 { client.step(&mut cx(),now(0),&mut work()); }
        });
        let listener=peer.join().unwrap(); listener.set_nonblocking(true).unwrap();
        assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock),"uncertain write must not replay");
        drop(client);clean(&bank);
    }
}

#[test]
fn consumer_cancellation_replacement_timeout_and_policy_loss_preserve_original_remote_effect() {
    io().block_on(async {
        for fault in ["cancel", "replace", "timeout", "policy"] {
            let addr = address();
            let td = description(addr);
            let mut server = Fixture::new(td.clone());
            server.controls.hold_send.set(true);
            let bank = bank();
            let owner = consumer_instance();
            let access = Arc::new(AtomicBool::new(false));
            let client = client(td, access.clone(), bank.clone(), &owner);
            let mut call = client
                .write_property(
                    "level",
                    payload(b"33"),
                    Deadline::at(now(10)),
                    now(0),
                    &mut work(),
                )
                .unwrap_or_else(|_| panic!());
            let start = Instant::now();
            while server.app.writes.get() == 0 {
                server.step();
                assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            match fault {
                "cancel" => call.cancel(),
                "replace" => owner.revoke(),
                "policy" => access.store(true, Ordering::Relaxed),
                _ => {}
            }
            let successor = consumer_instance();
            assert_ne!(successor.reference(), owner.reference());
            let result = loop {
                if let Poll::Ready(s) = call.poll(
                    &mut cx(),
                    now(if fault == "timeout" { 10 } else { 0 }),
                    &mut work(),
                ) {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert!(result.result().is_err());
            assert_eq!(result.consumer(), &owner.reference());
            assert_eq!(result.effect(), ConsumerEffect::Unknown);
            assert_eq!(server.app.writes.get(), 1);
            assert_eq!(*server.app.value.borrow(), b"33");
            // Native close and the unread late Producer result keep all original
            // accepted owners funded; cancellation does not erase handler success.
            server.controls.hold_close.set(true);
            server.exposure.begin_close();
            for _ in 0..5 {
                server.step();
            }
            assert_eq!(server.server().state(), RawFiniteState::Closing);
            assert!(TcpListener::bind(addr).is_err());
            server.controls.hold_close.set(false);
            server.step();
            let remote = server.server().take_settlement().unwrap();
            assert!(remote.result.is_ok());
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert_eq!(remote.delivery, FiniteDelivery::NotDelivered);
            drop(result);
            drop(call);
            drop(client);
            clean(&bank);
            server.close();
        }
    });
}

#[test]
fn missing_write_behavior_native_coverage_and_collisions_refuse_complete_publication() {
    let addr = address();
    let td = description(addr).to_string();
    let accounts = bank();
    let owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 2,
        target_name_bytes: 256,
    })
    .unwrap();
    let authority = authority(Generation::INITIAL);
    let key = authority.key();
    let provider: Rc<dyn ProducerCapability> = Rc::new(
        HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap(),
    );
    assert!(
        prepare_thing(
            &td,
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[provider.as_ref()],
            |_, op| op == Operation::ReadProperty
        )
        .is_err()
    );
    let prepared = prepare_thing(
        &td,
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[provider.as_ref()],
        |_, _| true,
    )
    .unwrap();
    let read = prepared
        .operations()
        .iter()
        .find(|o| o.operation == Operation::ReadProperty)
        .unwrap()
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let mappings = vec![
        (Arc::from("level"), read.clone()),
        (Arc::from("level"), read.clone()),
    ];
    let returned = HttpFiniteConfig::try_from_mappings(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        mappings,
        key,
        1000,
    )
    .err()
    .unwrap()
    .into_input();
    assert_eq!(returned.len(), 2);
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        read,
        key,
        Arc::from("level"),
        1000,
    )
    .unwrap();
    let server = RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(
            config.clone(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
        ),
        config,
        &owner,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 4096,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .unwrap_or_else(|_| panic!());
    let app = Rc::new(Application::default());
    let instance = RawProducerInstance::new(
        authority,
        [Operation::ReadProperty, Operation::WriteProperty]
            .into_iter()
            .map(|op| {
                RawFiniteBehavior::new(
                    AffordanceTarget::Property(Arc::from("level")),
                    op,
                    Behavior(app.clone()),
                )
            })
            .collect(),
    )
    .with_description(Arc::from(td));
    let original = ComposedProducerExposure::try_new(
        &owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![provider],
            participants: vec![Box::new(server)],
            expected: None,
            handler_backing_bytes: 8192,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
        &mut work(),
    )
    .err()
    .unwrap()
    .into_input();
    assert!(owner.description(key).is_none());
    assert_eq!(original.prepared.operations().len(), 2);
    drop(original);
    clean(&accounts);
    drop(TcpListener::bind(addr).unwrap());
    assert_eq!(app.writes.get(), 0);
    assert_eq!(app.drops.get(), 2);
}

fn raw_reply(server: &mut Fixture, bytes: &[u8]) -> (Vec<u8>, Option<RawFiniteSettlement>) {
    let mut peer = TcpStream::connect(server.addr).unwrap();
    peer.write_all(bytes).unwrap();
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    peer.set_nonblocking(true).unwrap();
    let mut reply = Vec::new();
    let start = Instant::now();
    loop {
        server.step();
        let mut buf = [0; 4096];
        match peer.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => reply.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
            Err(e) => panic!("{e}"),
        }
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    (reply, server.server().take_settlement())
}
#[test]
fn producer_validates_untrusted_write_value_and_framing_before_any_application_effect() {
    let addr = address();
    let mut server = Fixture::new(description(addr));
    for (extra, body) in [
        ("", "true"),
        ("", "invalid"),
        ("", "1.5"),
        ("Content-Encoding: gzip\r\n", "7"),
        ("Transfer-Encoding: chunked\r\n", "7"),
        ("Expect: 100-continue\r\n", "7"),
        ("Authorization: Basic c2VjcmV0\r\n", "7"),
        ("Content-Length: 1\r\n", "7"),
        ("Trailer: x-extra\r\n", "7"),
    ] {
        let request = format!(
            "PUT /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n{body}",
            body.len()
        );
        let (_, settlement) = raw_reply(&mut server, request.as_bytes());
        if extra.is_empty() {
            let settlement =
                settlement.expect("bounded valid native input reaches captured validation");
            assert!(matches!(settlement.result, Err(CoreError::Validation(_))));
            assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
        } else {
            assert!(settlement.is_none());
        }
        assert_eq!(server.app.writes.get(), 0);
    }
    for request in [
        format!(
            "PUT /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: text/plain\r\nContent-Length: 1\r\n\r\n7"
        ),
        format!("PUT /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\r\n"),
        format!(
            "PUT /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 4\r\n\r\n7"
        ),
        format!(
            "PUT /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 4097\r\n\r\n{}",
            "1".repeat(4097)
        ),
        format!("GET /level HTTP/1.1\r\nHost: {addr}\r\nContent-Length: 1\r\n\r\n7"),
        format!(
            "POST /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n7"
        ),
    ] {
        assert!(raw_reply(&mut server, request.as_bytes()).1.is_none());
        assert_eq!(server.app.writes.get(), 0);
        assert_eq!(server.app.reads.get(), 0);
    }
    // Maximal supported input arrives through several actual native polls.
    let body = format!("7{}", " ".repeat(4095));
    let request = format!(
        "PUT /level HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 4096\r\n\r\n{body}"
    );
    let (reply, settlement) = raw_reply(&mut server, request.as_bytes());
    assert!(reply.starts_with(b"HTTP/1.1 204"));
    assert!(settlement.unwrap().result.is_ok());
    assert_eq!(server.app.writes.get(), 1);
    server.close();
}

#[test]
fn producer_replacement_fences_pending_input_and_preserves_already_accepted_write_identity() {
    io().block_on(async {
        for accepted in [false, true] {
            let addr = address();
            let td = description(addr);
            let mut server = Fixture::new(td.clone());
            server.controls.hold_send.set(true);
            let old = server.exposure.key();
            let bank = bank();
            let owner = consumer_instance();
            let client = client(td, Arc::new(AtomicBool::new(false)), bank.clone(), &owner);
            let mut call = start_write(&client, b"45");
            let start = Instant::now();
            loop {
                server.step();
                assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
                if server.server().state()
                    == if accepted {
                        RawFiniteState::Delivering
                    } else {
                        RawFiniteState::Dispatching
                    }
                {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let next_addr = address();
            let next_app = Rc::new(Application::default());
            let mut successor = exposure(
                &server.owner,
                server.bank.clone(),
                description(next_addr),
                next_app.clone(),
                Rc::new(Controls::default()),
                authority(Generation::new(2).unwrap()),
                Some(old),
            );
            let _ = successor.participant_mut::<Server>(0).unwrap().step(
                &mut cx(),
                &mut work(),
                now(0),
            );
            successor.publish(&mut server.owner, &mut work()).unwrap();
            assert_ne!(server.exposure.key(), successor.key());
            server.controls.hold_send.set(false);
            let (local, remote) = server.exchange(&mut call).await;
            assert_eq!(local.result().is_ok(), accepted);
            assert_eq!(remote.recipient, Some(old));
            assert_eq!(remote.operation, Some(Operation::WriteProperty));
            assert_eq!(
                remote.effect,
                if accepted {
                    FiniteEffect::Unknown
                } else {
                    FiniteEffect::NotInvoked
                }
            );
            assert_eq!(server.app.writes.get(), usize::from(accepted));
            assert_eq!(next_app.writes.get(), 0);
            successor.begin_close();
            let _ = successor.participant_mut::<Server>(0).unwrap().step(
                &mut cx(),
                &mut work(),
                now(0),
            );
            successor.retire(&mut server.owner, &mut work()).unwrap();
            drop(successor);
            drop(local);
            drop(call);
            drop(client);
            clean(&bank);
            server.close();
            assert_eq!(next_app.drops.get(), 2);
        }
    });
}

#[test]
fn unread_results_and_service_loss_keep_bounded_original_owners_until_acknowledgement() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let mut server = Fixture::new(td.clone());
        let bank = bank();
        let owner = consumer_instance();
        let client = client(td, Arc::new(AtomicBool::new(false)), bank.clone(), &owner);
        let mut call = start_write(&client, b"56");
        let start = Instant::now();
        let local = loop {
            server.step();
            if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert!(local.result().is_ok());
        // The actual captured HTTP runtime remains occupied until this result
        // is released; refusal recovers the new value and cannot trigger I/O.
        let original = payload(b"57");
        let returned = client
            .write_property(
                "level",
                original.clone(),
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .err()
            .unwrap()
            .into_input();
        assert!(Arc::ptr_eq(&returned.body, &original.body));
        // A successful unread Producer settlement stops new receives.
        drop(local);
        drop(call);
        let mut pending = start_write(&client, b"57");
        for _ in 0..10 {
            server.step();
            assert!(pending.poll(&mut cx(), now(0), &mut work()).is_pending());
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(server.app.writes.get(), 1);
        let prior = server.server().take_settlement().unwrap();
        assert!(prior.result.is_ok());
        // Service loss after native receive fences dispatch and preserves the
        // original input/reply for native disposal, with no second handler call.
        while server.server().state() != RawFiniteState::Dispatching {
            server.step();
            assert!(pending.poll(&mut cx(), now(0), &mut work()).is_pending());
        }
        server.controls.lose_service.set(true);
        server.controls.hold_close.set(true);
        server.step();
        assert_eq!(server.server().state(), RawFiniteState::Closing);
        server.exposure.begin_close();
        assert!(
            server
                .exposure
                .retire(&mut server.owner, &mut work())
                .is_err()
        );
        assert!(TcpListener::bind(addr).is_err());
        assert!(server.bank.account(ResourceKind::CleanupBytesMax).used() > 0);
        server.controls.hold_close.set(false);
        server.step();
        let remote = server.server().take_settlement().unwrap();
        assert_eq!(remote.effect, FiniteEffect::NotInvoked);
        assert!(remote.result.is_err());
        assert_eq!(server.app.writes.get(), 1);
        drop(client);
        let result = loop {
            if let Poll::Ready(s) = pending.poll(&mut cx(), now(0), &mut work()) {
                break s;
            }
        };
        assert!(result.result().is_err());
        drop(result);
        drop(pending);
        clean(&bank);
        server.close();
    });
}
