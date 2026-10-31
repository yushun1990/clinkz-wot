#![cfg(feature = "prepare")]
//! Thing-root ReadAllProperties over one native Engine-to-Engine HTTP request.
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
    "forms":[{"op":"readallproperties","href":format!("http://{addr}/properties")}],
    "properties":{
        "level":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{addr}/level")}]},
        "on":{"type":"boolean","readOnly":true,"forms":[{"href":format!("http://{addr}/on")}]},
        "secret":{"type":"string","writeOnly":true,"forms":[{"href":format!("http://{addr}/secret")}]}
    }})
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
    roots: Cell<usize>,
    reads: Cell<usize>,
    writes: Cell<usize>,
    denies: Cell<bool>,
    fails: Cell<bool>,
    drops: Cell<usize>,
}
struct Behavior(Rc<Application>);
impl Drop for Behavior {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl RawFiniteHandler for Behavior {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        if self.0.denies.get() {
            Err(denied())
        } else {
            Ok(())
        }
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        match ctx.operation() {
            Operation::ReadAllProperties => {
                assert_eq!(ctx.target(), &AffordanceTarget::Thing);
                assert!(input.data.is_none());
                self.0.roots.set(self.0.roots.get() + 1);
                if self.0.fails.get() {
                    return Err(CoreError::Binding(ErrorContext::new(
                        ErrorPhase::Handler,
                        RetryClass::Never,
                    )));
                }
                Ok(InteractionOutput::with_data(payload(
                    &self.0.value.borrow(),
                )))
            }
            Operation::ReadProperty => {
                self.0.reads.set(self.0.reads.get() + 1);
                Ok(InteractionOutput::with_data(payload(
                    if ctx.target().name() == Some("level") {
                        b"7"
                    } else {
                        b"true"
                    },
                )))
            }
            Operation::WriteProperty => {
                self.0.writes.set(self.0.writes.get() + 1);
                Ok(InteractionOutput::empty())
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
    requests: Cell<usize>,
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
        let result = self.actual.poll_receive(cx, now);
        if matches!(&result, Poll::Ready(Ok(_))) {
            self.controls.requests.set(self.controls.requests.get() + 1);
        }
        result
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
    let addr = td["forms"][0]["href"]
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
    let behaviors = vec![
        RawFiniteBehavior::new(
            AffordanceTarget::Thing,
            Operation::ReadAllProperties,
            Behavior(app.clone()),
        ),
        RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("level")),
            Operation::ReadProperty,
            Behavior(app.clone()),
        ),
        RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("on")),
            Operation::ReadProperty,
            Behavior(app.clone()),
        ),
        RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("secret")),
            Operation::WriteProperty,
            Behavior(app.clone()),
        ),
    ];
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
    let prepared = prepare_thing(
        instance.description().unwrap(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[provider.as_ref()],
        |scope, op| match scope {
            ValidatedProducerScope::Thing => instance.supports(&AffordanceTarget::Thing, op),
            ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
            _ => false,
        },
    )
    .unwrap();
    let mappings = prepared
        .operations()
        .iter()
        .map(|o| {
            (
                match &o.site {
                    FormSite::Thing => AffordanceTarget::Thing,
                    FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
                    _ => unreachable!(),
                },
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
        let mut result = Self::closed(td);
        result
            .exposure
            .publish(&mut result.owner, &mut work())
            .unwrap();
        result
    }
    fn closed(td: Value) -> Self {
        let bank = bank();
        let owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
            instances: 1,
            behaviors_per_instance: 4,
            target_name_bytes: 256,
        })
        .unwrap();
        let app = Rc::new(Application::default());
        *app.value.borrow_mut() = br#"{"level":7,"on":true}"#.to_vec();
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
        Self {
            owner,
            exposure,
            bank,
            app,
            controls,
            addr,
        }
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
    let prepared = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration],
        ConsumerSelector::default(),
    )
    .unwrap();
    let (definitions, funding) = prepared.into_execution();
    let root = definitions
        .iter()
        .find(|d| d.operation() == Operation::ReadAllProperties)
        .unwrap();
    assert_eq!(root.affordance(), &AffordanceTarget::Thing);
    assert_eq!(root.name(), None);
    assert_eq!(root.binding().binding_id(), BindingId::new(98));
    drop(td);
    ConsumerRuntime::try_new(owner.reference(), definitions, funding, accounts, 2)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn prepare_consumer(
    td: &Value,
    accounts: Arc<HostResourceAccounts>,
    registrations: Vec<ConsumerRegistrationInput>,
    selector: ConsumerSelector,
) -> CoreResult<PreparedConsumer> {
    let registry =
        ConsumerRegistry::try_new(registrations).unwrap_or_else(|e| panic!("{:?}", e.error()));
    let td: Thing = serde_json::from_value(td.clone()).unwrap();
    registry.prepare(
        &td,
        GatewayDefaultV1::LIMITS,
        accounts.clone(),
        selector,
        &mut work(),
        |job| {
            while !job.is_finished() {
                job.step(&mut work(), false);
            }
        },
    )
}

fn start_read(client: &ConsumerRuntime) -> ConsumerCall {
    client
        .read_all_properties(Deadline::NONE, now(0), &mut work())
        .unwrap()
}
fn registration() -> ConsumerRegistrationInput {
    try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(98),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap()
}

#[test]
fn engine_read_all_uses_one_root_request_and_keeps_individual_operations_distinct() {
    io().block_on(async {
        let addr = address();
        let mut td = description(addr);
        td["forms"][0]["htv:methodName"] = json!("GET");
        td["forms"][0]["response"] =
            json!({"contentType":"application/json", "htv:statusCodeValue":200});
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let mut call = start_read(&client);
        let (result, remote) = server.exchange(&mut call).await;
        assert_eq!(result.operation(), Operation::ReadAllProperties);
        assert_eq!(result.affordance(), &AffordanceTarget::Thing);
        assert_eq!(result.consumer(), &owner.reference());
        assert_eq!(
            result.result().unwrap().data().unwrap().body.as_ref(),
            br#"{"level":7,"on":true}"#
        );
        assert_eq!(remote.target, Some(AffordanceTarget::Thing));
        assert_eq!(remote.recipient, Some(server.exposure.key()));
        assert_eq!(remote.operation, Some(Operation::ReadAllProperties));
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        assert_eq!(server.controls.requests.get(), 1);
        assert_eq!(server.app.roots.get(), 1);
        assert_eq!(server.app.reads.get(), 0);
        assert_eq!(server.app.writes.get(), 0);
        // Unread results retain original call capacity, mapping and schema.
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(result);
        drop(call);
        let mut single = client
            .read_property("level", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (result, _) = server.exchange(&mut single).await;
        assert_eq!(result.affordance().name(), Some("level"));
        assert_eq!(result.result().unwrap().data().unwrap().body.as_ref(), b"7");
        assert_eq!(server.app.reads.get(), 1);
        assert_eq!(server.app.roots.get(), 1);
        drop(result);
        drop(single);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn producer_rejects_missing_wrong_duplicate_unknown_and_write_only_members_without_fanout() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let bodies: &[&[u8]] = &[
            br#"{"level":7}"#,
            br#"{"on":true}"#,
            br#"{"level":true,"on":true}"#,
            br#"{"level":1.5,"on":true}"#,
            br#"{"level":7,"on":true,"secret":"leak"}"#,
            br#"{"level":7,"on":true,"other":0}"#,
            br#"{"level":7,"level":8,"on":true}"#,
            br#"{"level":7,"\u006cevel":8,"on":true}"#,
            br#"{"level":7,"on":{"error":"failed"}}"#,
            br#"{"level":7,"on":true}{}"#,
            b"[]",
            b"null",
            b"{broken",
        ];
        for body in bodies {
            *server.app.value.borrow_mut() = body.to_vec();
            let mut call = start_read(&client);
            let (result, remote) = server.exchange(&mut call).await;
            assert!(result.result().is_err(), "{body:?}");
            assert!(
                matches!(remote.result, Err(CoreError::Validation(_))),
                "{body:?}"
            );
            assert_eq!(remote.target, Some(AffordanceTarget::Thing));
            drop(result);
            drop(call);
        }
        server.app.fails.set(true);
        let mut call = start_read(&client);
        let (result, remote) = server.exchange(&mut call).await;
        assert!(result.result().is_err());
        assert!(matches!(remote.result, Err(CoreError::Binding(_))));
        assert_eq!(server.app.roots.get(), bodies.len() + 1);
        assert_eq!(server.controls.requests.get(), bodies.len() + 1);
        assert_eq!(server.app.reads.get(), 0);
        drop(result);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn consumer_validates_independent_native_response_against_captured_member_schemas() {
    io().block_on(async {
        for (status, body, valid) in [
            (200, br#"{"on":true,"level":7.0}"#.as_slice(), true),
            (200, br#"{"level":70e-1,"on":true}"#, true),
            (200, br#"{"level":7e-100,"on":true}"#, false),
            (200, br#"{"level":7}"#, false),
            (200, br#"{"level":7,"on":true,"secret":"leak"}"#, false),
            (200, br#"{"level":7,"on":true,"on":false}"#, false),
            (200, br#"{"level":7,"on":true,"extra":null}"#, false),
            (200, br#"{"level":"7","on":true}"#, false),
            (200, br#"{"level":7,"on":{"error":"unavailable"}}"#, false),
            (206, br#"{"level":7,"on":true}"#, false),
            (207, br#"{"level":7,"on":true}"#, false),
            (500, br#"{"level":7,"on":true}"#, false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let response = body.to_vec();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new(); let mut buf = [0; 1024];
                while !bytes.ends_with(b"\r\n\r\n") {
                    let n = socket.read(&mut buf).unwrap(); assert!(n > 0); bytes.extend_from_slice(&buf[..n]);
                }
                assert!(bytes.starts_with(b"GET /properties HTTP/1.1\r\n"));
                let mut reply = format!("HTTP/1.1 {status} Reply\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).into_bytes();
                reply.extend_from_slice(&response);
                socket.write_all(&reply).unwrap();
                listener
            });
            let accounts = bank(); let owner = consumer_instance();
            let client = client(description(addr), Arc::new(AtomicBool::new(false)), accounts.clone(), &owner);
            let mut call = start_read(&client); let start = Instant::now();
            let result = loop {
                if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) { break result; }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(result.result().is_ok(), valid, "{body:?}");
            if !valid && status == 200 { assert_eq!(result.rejected_output().unwrap().data().unwrap().body.as_ref(), body); }
            drop(result); drop(call); drop(client); clean(&accounts);
            let listener = peer.join().unwrap(); listener.set_nonblocking(true).unwrap();
            assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        }
    });
}

#[test]
fn consumer_root_selection_requires_actual_form_binding_services_and_complete_schema_support() {
    let addr = address();
    for bad in [
        "absent",
        "https",
        "method",
        "schema",
        "security",
        "response",
        "multiple",
        "variables",
    ] {
        let mut td = description(addr);
        match bad {
            "absent" => {
                td.as_object_mut().unwrap().remove("forms");
            }
            "https" => td["forms"][0]["href"] = json!(format!("https://{addr}/properties")),
            "method" => td["forms"][0]["htv:methodName"] = json!("POST"),
            "schema" => td["properties"]["on"]["const"] = json!(true),
            "security" => {
                td["securityDefinitions"]["b"] = json!({"scheme":"basic"});
                td["forms"][0]["security"] = json!(["b"]);
            }
            "response" => td["forms"][0]["response"] = json!({"contentType":"text/plain"}),
            "multiple" => td["forms"][0]["op"] = json!("readmultipleproperties"),
            _ => {
                td["uriVariables"] = json!({"unit":{"type":"string"}});
                td["forms"][0]["href"] = json!(format!("http://{addr}/properties{{?unit}}"));
            }
        }
        let accounts = bank();
        if let Ok(p) = prepare_consumer(
            &td,
            accounts.clone(),
            vec![registration()],
            ConsumerSelector::default(),
        ) {
            assert!(
                !p.support()
                    .iter()
                    .any(|d| d.selected && d.operation == Operation::ReadAllProperties),
                "{bad}"
            );
            let owner = consumer_instance();
            let (d, f) = p.into_execution();
            let client = ConsumerRuntime::try_new(owner.reference(), d, f, accounts.clone(), 1)
                .unwrap_or_else(|_| panic!());
            assert!(
                client
                    .read_all_properties(Deadline::NONE, now(0), &mut work())
                    .is_err()
            );
            drop(client);
        }
        clean(&accounts);
        drop(TcpListener::bind(addr).unwrap());
    }
    let mut grouped_only = description(addr);
    for n in ["on", "level"] {
        grouped_only["properties"][n]["forms"][0]["href"] =
            json!(format!("mqtt://127.0.0.1:1883/{n}"));
    }
    grouped_only["properties"]["secret"]["const"] = json!("unsupported writable assertion");
    let accounts = bank();
    let p = prepare_consumer(
        &grouped_only,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let (defs, funding) = p.into_execution();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].affordance(), &AffordanceTarget::Thing);
    drop(defs);
    drop(funding);
    clean(&accounts);
    let mut td = description(addr);
    let supported = td["forms"][0].clone();
    td["forms"] =
        json!([{"op":"readallproperties","href":format!("https://{addr}/bad")}, supported]);
    let accounts = bank();
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let root = p
        .support()
        .iter()
        .find(|d| d.selected && d.operation == Operation::ReadAllProperties)
        .unwrap();
    assert_eq!(root.form, 1);
    assert_eq!(root.site, FormSite::Thing);
    drop(p);
    clean(&accounts);
    let other = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(99),
        BindingGeneration::INITIAL,
        1,
    )
    .unwrap();
    assert!(
        prepare_consumer(
            &td,
            accounts.clone(),
            vec![registration(), other],
            ConsumerSelector::default()
        )
        .is_err()
    );
    clean(&accounts);
    // An explicit Form selector cannot reinterpret an unsupported root Form.
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector {
            form: Some(0),
            binding: None,
        },
    )
    .unwrap();
    assert!(
        !p.support()
            .iter()
            .any(|d| d.selected && d.operation == Operation::ReadAllProperties)
    );
    drop(p);
    clean(&accounts);
}

#[test]
fn full_producer_qualification_requires_root_behavior_every_form_and_original_member_meaning() {
    let addr = address();
    let td = description(addr);
    let key = authority(Generation::INITIAL).key();
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
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |scope, _| scope != ValidatedProducerScope::Thing
        )
        .is_err()
    );
    for fault in [
        "schema",
        "root-op",
        "root-protocol",
        "property",
        "root-response",
        "root-method",
    ] {
        let mut bad = td.clone();
        match fault {
            "schema" => bad["properties"]["on"]["const"] = json!(true),
            "root-op" => bad["forms"][0]["op"] = json!(["readallproperties", "writeallproperties"]),
            "root-protocol" => bad["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"op":"readallproperties","href":"mqtt://127.0.0.1:1883/group"})),
            "property" => {
                bad["properties"]["level"]["forms"][0]["href"] = json!("https://127.0.0.1/level")
            }
            "root-response" => {
                bad["forms"][0]["response"] =
                    json!({"contentType":"application/json", "htv:statusCodeValue":207})
            }
            _ => bad["forms"][0]["htv:methodName"] = json!("POST"),
        }
        assert!(
            prepare_thing(
                &bad.to_string(),
                key,
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |_, _| true
            )
            .is_err(),
            "{fault}"
        );
        drop(TcpListener::bind(addr).unwrap());
    }
    let prepared = prepare_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    for fault in ["schema", "writeOnly", "new-member"] {
        let mut changed = td.clone();
        match fault {
            "schema" => changed["properties"]["level"]["type"] = json!("string"),
            "writeOnly" => {
                changed["properties"]["level"]["readOnly"] = json!(false);
                changed["properties"]["level"]["writeOnly"] = json!(true);
            }
            _ => {
                changed["properties"]["extra"] = json!({"type":"null", "readOnly":true,"forms":[{"href":format!("http://{addr}/extra")}]})
            }
        }
        assert!(
            check_final_thing(
                &changed.to_string(),
                key,
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                &prepared,
                |_, _| true
            )
            .is_err(),
            "{fault}"
        );
    }
    assert!(
        check_final_thing(
            &td.to_string(),
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            &prepared,
            |_, _| true
        )
        .is_ok()
    );
    // Same native GET path is a collision even though logical operations differ.
    let mut same = td.clone();
    same["properties"]["level"]["forms"][0]["href"] = td["forms"][0]["href"].clone();
    let p = prepare_thing(
        &same.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    let mappings = p
        .operations()
        .iter()
        .map(|o| {
            (
                match &o.site {
                    FormSite::Thing => AffordanceTarget::Thing,
                    FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
                    _ => unreachable!(),
                },
                o.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    assert!(
        HttpFiniteConfig::try_from_mappings(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            mappings,
            key,
            1000
        )
        .is_err()
    );
    drop(TcpListener::bind(addr).unwrap());
}

#[test]
fn root_qualification_cannot_be_satisfied_by_only_individual_native_mappings() {
    let addr = address();
    let td = description(addr);
    let accounts = bank();
    let owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 4,
        target_name_bytes: 256,
    })
    .unwrap();
    let a = authority(Generation::INITIAL);
    let key = a.key();
    let provider: Rc<dyn ProducerCapability> = Rc::new(
        HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap(),
    );
    let prepared = prepare_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[provider.as_ref()],
        |_, _| true,
    )
    .unwrap();
    let mappings = prepared
        .operations()
        .iter()
        .filter(|o| o.site != FormSite::Thing)
        .map(|o| {
            let FormSite::Property(n) = &o.site else {
                unreachable!()
            };
            (
                AffordanceTarget::Property(Arc::from(n.as_str())),
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
    .unwrap_or_else(|_| panic!());
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
        a,
        vec![
            RawFiniteBehavior::new(
                AffordanceTarget::Thing,
                Operation::ReadAllProperties,
                Behavior(app.clone()),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Behavior(app.clone()),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("on")),
                Operation::ReadProperty,
                Behavior(app.clone()),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("secret")),
                Operation::WriteProperty,
                Behavior(app.clone()),
            ),
        ],
    )
    .with_description(Arc::from(td.to_string()));
    let returned = ComposedProducerExposure::try_new(
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
    assert_eq!(returned.prepared.operations().len(), 4);
    drop(returned);
    clean(&accounts);
    drop(TcpListener::bind(addr).unwrap());
    assert_eq!(app.roots.get(), 0);
    assert_eq!(app.drops.get(), 4);
}

#[test]
fn cancellation_replacement_deadline_and_policy_loss_retain_original_root_owners() {
    io().block_on(async {
        for fault in ["cancel", "replace", "timeout", "policy", "abandon"] {
            let addr = address();
            let td = description(addr);
            let mut server = Fixture::new(td.clone());
            server.controls.hold_send.set(true);
            let accounts = bank();
            let owner = consumer_instance();
            let denied = Arc::new(AtomicBool::new(false));
            let client = client(td, denied.clone(), accounts.clone(), &owner);
            let mut call = Some(
                client
                    .read_all_properties(Deadline::at(now(10)), now(0), &mut work())
                    .unwrap(),
            );
            let start = Instant::now();
            while server.app.roots.get() == 0 {
                server.step();
                assert!(
                    call.as_mut()
                        .unwrap()
                        .poll(&mut cx(), now(0), &mut work())
                        .is_pending()
                );
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            match fault {
                "cancel" => call.as_mut().unwrap().cancel(),
                "replace" => owner.revoke(),
                "policy" => denied.store(true, Ordering::Relaxed),
                "abandon" => {
                    drop(call.take());
                }
                _ => {}
            }
            let instant = now(if fault == "timeout" { 10 } else { 0 });
            let retained = ResourceKind::ALL.map(|k| accounts.account(k).used());
            if let Some(c) = &mut call {
                assert!(
                    c.poll(&mut cx(), instant, &mut WorkBudget::new())
                        .is_pending()
                );
            } else {
                client.step(&mut cx(), instant, &mut WorkBudget::new());
            }
            assert_eq!(
                retained,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
            if let Some(c) = &mut call {
                let result = loop {
                    if let Poll::Ready(s) = c.poll(&mut cx(), instant, &mut work()) {
                        break s;
                    }
                    assert!(start.elapsed() < Duration::from_secs(5));
                    tokio::time::sleep(Duration::from_millis(1)).await;
                };
                assert!(result.result().is_err());
                assert_eq!(result.consumer(), &owner.reference());
                assert_eq!(result.affordance(), &AffordanceTarget::Thing);
                assert!(result.first_cause().is_some());
                drop(result);
            } else {
                while !client.is_idle() {
                    client.step(&mut cx(), instant, &mut work());
                    assert!(start.elapsed() < Duration::from_secs(5));
                }
            }
            server.controls.hold_close.set(true);
            server.exposure.begin_close();
            server.step();
            assert_eq!(server.server().state(), RawFiniteState::Closing);
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
            assert!(remote.result.is_ok());
            assert_eq!(remote.target, Some(AffordanceTarget::Thing));
            assert_eq!(remote.delivery, FiniteDelivery::NotDelivered);
            assert_eq!(server.controls.requests.get(), 1);
            assert_eq!(server.app.roots.get(), 1);
            assert_eq!(server.app.reads.get(), 0);
            drop(call);
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn producer_replacement_preserves_original_root_attribution_before_and_after_dispatch() {
    io().block_on(async {
        for accepted in [false, true] {
            let addr = address();
            let td = description(addr);
            let mut server = Fixture::new(td.clone());
            server.controls.hold_send.set(true);
            let old = server.exposure.key();
            let accounts = bank();
            let owner = consumer_instance();
            let client = client(
                td,
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &owner,
            );
            let mut call = start_read(&client);
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
            server.controls.hold_send.set(false);
            let (local, remote) = server.exchange(&mut call).await;
            assert_eq!(local.result().is_ok(), accepted);
            assert_eq!(remote.recipient, Some(old));
            assert_eq!(
                remote.target,
                if accepted {
                    Some(AffordanceTarget::Thing)
                } else {
                    None
                }
            );
            assert_eq!(server.app.roots.get(), usize::from(accepted));
            assert_eq!(next_app.roots.get(), 0);
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
            clean(&accounts);
            server.close();
            assert_eq!(next_app.drops.get(), 4);
        }
    });
}

// The actual selected service can refuse collection meaning independently of
// scalar type support, or disappear during its own validation callback.
struct Data {
    inner: Arc<dyn ConsumerDataService>,
    available: Arc<AtomicBool>,
    maps: bool,
    lose_on_validate: bool,
}
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        self.inner.content_type()
    }
    fn validate(
        &self,
        schema: clinkz_wot_core::value_type::TypeSchema,
        p: &Payload,
    ) -> CoreResult<()> {
        self.inner.validate(schema, p)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut p = self.inner.policy();
        p.retained_bytes += 128;
        p
    }
    fn available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
    fn supports_property_map(
        &self,
        schema: &clinkz_wot_core::collection::PropertyCollectionSchema,
    ) -> bool {
        self.maps && self.inner.supports_property_map(schema)
    }
    fn validate_property_map(
        &self,
        schema: &clinkz_wot_core::collection::PropertyCollectionSchema,
        p: &Payload,
    ) -> CoreResult<()> {
        let result = self.inner.validate_property_map(schema, p);
        if self.lose_on_validate {
            self.available.store(false, Ordering::Relaxed);
        }
        result
    }
}

#[test]
fn captured_collection_service_is_required_and_freshly_checked_after_validation() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let accounts = bank();
        let mut reg = registration();
        reg.services.data = Arc::new(Data {
            inner: reg.services.data.clone(),
            available: Arc::new(AtomicBool::new(true)),
            maps: false,
            lose_on_validate: false,
        });
        let prepared = prepare_consumer(
            &td,
            accounts.clone(),
            vec![reg],
            ConsumerSelector::default(),
        )
        .unwrap();
        assert!(
            !prepared
                .support()
                .iter()
                .any(|s| s.selected && s.operation == Operation::ReadAllProperties)
        );
        drop(prepared);
        clean(&accounts);
        let mut server = Fixture::new(td.clone());
        let available = Arc::new(AtomicBool::new(true));
        let mut reg = registration();
        reg.services.data = Arc::new(Data {
            inner: reg.services.data.clone(),
            available: available.clone(),
            maps: true,
            lose_on_validate: true,
        });
        let prepared = prepare_consumer(
            &td,
            accounts.clone(),
            vec![reg],
            ConsumerSelector::default(),
        )
        .unwrap();
        let owner = consumer_instance();
        let (d, f) = prepared.into_execution();
        let client = ConsumerRuntime::try_new(owner.reference(), d, f, accounts.clone(), 1)
            .unwrap_or_else(|_| panic!());
        let mut call = start_read(&client);
        let (local, remote) = server.exchange(&mut call).await;
        assert!(!available.load(Ordering::Relaxed));
        assert!(local.result().is_err());
        assert!(matches!(
            local.native_outcome(),
            BindingCallSettlement::Returned(Ok(_))
        ));
        assert!(remote.result.is_ok());
        assert_eq!(server.app.roots.get(), 1);
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn preparation_saturation_and_unpaid_member_traversal_preserve_all_owners_without_effects() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let accounts = bank();
        let registry = ConsumerRegistry::try_new(vec![registration()]).unwrap_or_else(|_| panic!());
        let thing: Thing = serde_json::from_value(td.clone()).unwrap();
        assert!(
            registry
                .prepare(
                    &thing,
                    GatewayDefaultV1::LIMITS,
                    accounts.clone(),
                    ConsumerSelector::default(),
                    &mut WorkBudget::new(),
                    |_| panic!("unpaid native compiler")
                )
                .is_err()
        );
        clean(&accounts);
        drop(registry);
        drop(thing);
        let owner = consumer_instance();
        let client = client(
            td.clone(),
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let before = ResourceKind::ALL.map(|k| accounts.account(k).used());
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut WorkBudget::new())
                .is_err()
        );
        assert_eq!(
            before,
            ResourceKind::ALL.map(|k| accounts.account(k).used())
        );
        let a = accounts.account(ResourceKind::EngineLiveBytesGlobalMax);
        let saturated = accounts
            .reserve(ResourceKind::EngineLiveBytesGlobalMax, a.limit() - a.used())
            .unwrap();
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(saturated);
        assert_eq!(
            before,
            ResourceKind::ALL.map(|k| accounts.account(k).used())
        );
        // Refused starts have not acquired a native listener/connection.
        let mut server = Fixture::new(td);
        let mut call = start_read(&client);
        let start = Instant::now();
        while server.server().state() != RawFiniteState::Dispatching {
            server.step();
            assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        for _ in 0..3 {
            let mut short = work().with_remaining(WorkClass::HandlerSteps, 20_000);
            assert!(
                server
                    .owner
                    .step(
                        server.exposure.participant_mut::<Server>(0).unwrap(),
                        &mut cx(),
                        &mut short,
                        now(0)
                    )
                    .unwrap()
                    .is_pending()
            );
            assert_eq!(server.app.roots.get(), 0);
        }
        // Once admitted, an unread native collection result waits for the whole
        // captured validation allowance; small credits do not invoke it piecemeal.
        for _ in 0..20 {
            server.step();
            assert!(
                call.poll(
                    &mut cx(),
                    now(0),
                    &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
                )
                .is_pending()
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(server.controls.requests.get(), 1);
        assert_eq!(server.app.roots.get(), 1);
        let (result, remote) = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        assert!(remote.result.is_ok());
        drop(client);
        assert!(
            accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop(result);
        drop(call);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn prepublication_ingress_and_lost_producer_validation_cannot_dispatch_root_behavior() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let mut server = Fixture::closed(td.clone());
        let mut peer = TcpStream::connect(addr).unwrap();
        write!(peer, "GET /properties HTTP/1.1\r\nHost: {addr}\r\n\r\n").unwrap();
        peer.shutdown(std::net::Shutdown::Write).unwrap();
        peer.set_nonblocking(true).unwrap();
        for _ in 0..5 {
            server.step();
        }
        assert_eq!(server.app.roots.get(), 0);
        assert_eq!(server.controls.requests.get(), 0);
        assert!(server.owner.description(server.exposure.key()).is_none());
        server
            .exposure
            .publish(&mut server.owner, &mut work())
            .unwrap();
        let mut bytes = Vec::new();
        let start = Instant::now();
        loop {
            server.step();
            let mut buf = [0; 4096];
            match peer.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => bytes.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("{e}"),
            }
            assert!(start.elapsed() < Duration::from_secs(5));
        }
        assert!(bytes.starts_with(b"HTTP/1.1 200"));
        assert_eq!(server.app.roots.get(), 1);
        drop(server.server().take_settlement().unwrap());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let mut call = start_read(&client);
        while server.server().state() != RawFiniteState::Dispatching {
            server.step();
            assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        server.controls.lose_service.set(true);
        server.controls.hold_close.set(true);
        server.step();
        assert_eq!(server.server().state(), RawFiniteState::Closing);
        assert!(TcpListener::bind(addr).is_err());
        server.exposure.begin_close();
        assert!(
            server
                .exposure
                .retire(&mut server.owner, &mut work())
                .is_err()
        );
        server.controls.hold_close.set(false);
        server.step();
        let remote = server.server().take_settlement().unwrap();
        assert!(remote.result.is_err());
        assert_eq!(remote.effect, FiniteEffect::NotInvoked);
        let result = loop {
            if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert!(result.result().is_err());
        assert_eq!(server.app.roots.get(), 1);
        drop(result);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn root_authorization_refuses_before_effects_without_individual_fallback() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let accounts = bank();
        let owner = consumer_instance();
        let denied = Arc::new(AtomicBool::new(false));
        let client = client(td.clone(), denied.clone(), accounts.clone(), &owner);
        denied.store(true, Ordering::Relaxed);
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        let mut server = Fixture::new(td);
        assert_eq!(server.controls.requests.get(), 0);
        denied.store(false, Ordering::Relaxed);
        server.app.denies.set(true);
        let mut call = start_read(&client);
        let (result, remote) = server.exchange(&mut call).await;
        assert!(result.result().is_err());
        assert!(matches!(remote.result, Err(CoreError::Security { .. })));
        assert_eq!(remote.effect, FiniteEffect::NotInvoked);
        assert_eq!(server.controls.requests.get(), 1);
        assert_eq!(server.app.roots.get(), 0);
        assert_eq!(server.app.reads.get(), 0);
        drop(result);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}
