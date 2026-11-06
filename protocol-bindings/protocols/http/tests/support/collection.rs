#[allow(unused_imports)]
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
    // TD preparation needs the endpoint before native installation. Avoid
    // assigning the same kernel-selected port to parallel staged fixtures.
    static USED: std::sync::Mutex<Vec<SocketAddr>> = std::sync::Mutex::new(Vec::new());
    let mut used = USED.lock().unwrap();
    loop {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        if !used.contains(&addr) {
            used.push(addr);
            return addr;
        }
    }
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
    fixed_selection_response: Cell<bool>,
    selections: RefCell<Vec<Vec<String>>>,
    authorizations: Cell<usize>,
    write_fault: Cell<u8>,
    applied: RefCell<Value>,
    denied_member: RefCell<Option<String>>,
    write_report: Cell<Option<collection::PropertyWriteReport>>,
}
struct Behavior(Rc<Application>);
impl Drop for Behavior {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl RawFiniteHandler for Behavior {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        if let Some(name) = self.0.denied_member.borrow().as_ref() {
            let values: Value = serde_json::from_slice(&input.data.as_ref().unwrap().body).unwrap();
            if values.get(name).is_some() {
                return Err(denied());
            }
        }
        self.0.authorizations.set(self.0.authorizations.get() + 1);
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
            Operation::ReadAllProperties | Operation::ReadMultipleProperties => {
                assert_eq!(ctx.target(), &AffordanceTarget::Thing);
                assert_eq!(
                    input.data.is_none(),
                    ctx.operation() == Operation::ReadAllProperties
                );
                self.0.roots.set(self.0.roots.get() + 1);
                if self.0.fails.get() {
                    return Err(CoreError::Binding(ErrorContext::new(
                        ErrorPhase::Handler,
                        RetryClass::Never,
                    )));
                }
                if ctx.operation() == Operation::ReadMultipleProperties {
                    let names: Vec<String> =
                        serde_json::from_slice(&input.data.as_ref().unwrap().body).unwrap();
                    self.0.selections.borrow_mut().push(names.clone());
                    if !self.0.fixed_selection_response.get() {
                        let values: Value = serde_json::from_slice(&self.0.value.borrow()).unwrap();
                        let selected: serde_json::Map<String, Value> = names
                            .into_iter()
                            .map(|n| {
                                let value = values[&n].clone();
                                (n, value)
                            })
                            .collect();
                        return Ok(InteractionOutput::with_data(payload(
                            &serde_json::to_vec(&selected).unwrap(),
                        )));
                    }
                }
                Ok(InteractionOutput::with_data(payload(
                    &self.0.value.borrow(),
                )))
            }
            Operation::WriteAllProperties | Operation::WriteMultipleProperties => {
                assert_eq!(ctx.target(), &AffordanceTarget::Thing);
                self.0.roots.set(self.0.roots.get() + 1);
                let values: Value =
                    serde_json::from_slice(&input.data.as_ref().unwrap().body).unwrap();
                // Real application effects are deliberately sequential. A root
                // operation does not acquire atomicity from one network request.
                if self.0.write_fault.get() == 5 {
                    return Err(denied());
                }
                if let Some(value) = values.get("on") {
                    self.0.applied.borrow_mut()["on"] = value.clone();
                }
                if self.0.write_fault.get() == 1 {
                    return Err(denied());
                }
                for (name, value) in values.as_object().unwrap() {
                    self.0.applied.borrow_mut()[name] = value.clone();
                }
                match self.0.write_fault.get() {
                    2 => Err(denied()),
                    3 => Ok(InteractionOutput::with_data(payload(b"{}"))),
                    4 => Ok(InteractionOutput::empty().with_status(InteractionStatus::Accepted)),
                    _ => Ok(if let Some(report) = self.0.write_report.get() {
                        InteractionOutput::empty()
                            .try_with_metadata(
                                InteractionOutputMetadata::default()
                                    .with_property_write_report(report),
                            )
                            .unwrap()
                    } else {
                        InteractionOutput::empty()
                    }),
                }
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
    let mut behaviors = Vec::new();
    for (name, operation) in [("level", Operation::ReadProperty),
        ("on", Operation::ReadProperty), ("secret", Operation::WriteProperty)] {
        if td["properties"][name]["forms"].as_array().is_some_and(|f| !f.is_empty()) {
            behaviors.push(RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from(name)), operation, Behavior(app.clone()),
            ));
        }
    }
    if td["properties"]["on"]["readOnly"] == false {
        behaviors.push(RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("on")),
            Operation::WriteProperty,
            Behavior(app.clone()),
        ));
    }
    for (name, operation) in [
        ("readallproperties", Operation::ReadAllProperties),
        ("readmultipleproperties", Operation::ReadMultipleProperties),
        ("writeallproperties", Operation::WriteAllProperties),
        (
            "writemultipleproperties",
            Operation::WriteMultipleProperties,
        ),
    ] {
        if td["forms"].as_array().unwrap().iter().any(|f| {
            f["op"] == name
                || f["op"]
                    .as_array()
                    .is_some_and(|ops| ops.iter().any(|op| op == name))
        }) {
            behaviors.push(RawFiniteBehavior::new(
                AffordanceTarget::Thing,
                operation,
                Behavior(app.clone()),
            ));
        }
    }
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
            behaviors_per_instance: 8,
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
        assert_eq!(
            server.state(),
            RawFiniteState::InstalledClosed,
            "native setup failed: {:?}",
            server.first_cause()
        );
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
        .find(|d| {
            matches!(
                d.operation(),
                Operation::ReadAllProperties
                    | Operation::ReadMultipleProperties
                    | Operation::WriteAllProperties
                    | Operation::WriteMultipleProperties
            )
        })
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
        ConsumerRegistry::try_new(registrations, &accounts).unwrap_or_else(|e| panic!("{:?}", e.error()));
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

fn collection_description(addr: SocketAddr, operation: Operation) -> Value {
    let mut td = description(addr);
    if operation == Operation::ReadMultipleProperties {
        td["forms"][0]["op"] = json!("readmultipleproperties");
        td["forms"][0]["htv:methodName"] = json!("POST");
    }
    if matches!(
        operation,
        Operation::WriteAllProperties | Operation::WriteMultipleProperties
    ) {
        td["forms"][0]["op"] = json!(operation.as_str());
        td["properties"]["on"]["readOnly"] = json!(false);
    }
    td
}
fn start_collection_at(
    client: &ConsumerRuntime,
    operation: Operation,
    deadline: Deadline,
) -> CoreResult<ConsumerCall> {
    if operation == Operation::ReadMultipleProperties {
        client.read_multiple_properties(&["on"], deadline, now(0), &mut work())
    } else if operation == Operation::WriteAllProperties {
        client
            .write_all_properties(
                payload(br#"{"on":false,"secret":"updated"}"#),
                deadline,
                now(0),
                &mut work(),
            )
            .map_err(|e| e.error().error().clone())
    } else if operation == Operation::WriteMultipleProperties {
        client
            .write_multiple_properties(
                payload(br#"{"on":false,"secret":"updated"}"#),
                deadline,
                now(0),
                &mut work(),
            )
            .map_err(|e| e.error().error().clone())
    } else {
        client.read_all_properties(deadline, now(0), &mut work())
    }
}

fn verify_collection_lifecycle(operation: Operation) {
    io().block_on(async {
        for fault in ["cancel", "replace", "timeout", "policy", "abandon"] {
            let addr = address();
            let td = collection_description(addr, operation);
            let mut server = Fixture::new(td.clone());
            server.controls.hold_send.set(true);
            let accounts = bank();
            let owner = consumer_instance();
            let denied = Arc::new(AtomicBool::new(false));
            let client = client(td, denied.clone(), accounts.clone(), &owner);
            let mut call =
                Some(start_collection_at(&client, operation, Deadline::at(now(10))).unwrap());
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
                assert_eq!(result.affordance(), clinkz_wot_core::thing::AffordanceSite::Thing);
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
            if matches!(
                operation,
                Operation::WriteAllProperties | Operation::WriteMultipleProperties
            ) {
                assert_eq!(remote.effect, FiniteEffect::Completed);
                assert_eq!(
                    *server.app.applied.borrow(),
                    json!({"on":false,"secret":"updated"})
                );
            }
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

fn verify_collection_replacement(operation: Operation) {
    io().block_on(async {
        for accepted in [false, true] {
            let addr = address();
            let td = collection_description(addr, operation);
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
            let mut call = start_collection_at(&client, operation, Deadline::NONE).unwrap();
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
                collection_description(next_addr, operation),
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
            if matches!(
                operation,
                Operation::WriteAllProperties | Operation::WriteMultipleProperties
            ) {
                assert_eq!(
                    remote.effect,
                    if accepted {
                        FiniteEffect::Completed
                    } else {
                        FiniteEffect::NotInvoked
                    }
                );
                assert_eq!(
                    local.effect(),
                    if accepted {
                        ConsumerEffect::Completed
                    } else {
                        ConsumerEffect::Unknown
                    }
                );
                assert_eq!(server.app.applied.borrow().is_null(), !accepted);
                assert!(next_app.applied.borrow().is_null());
            }
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
            assert_eq!(
                next_app.drops.get(),
                if matches!(
                    operation,
                    Operation::WriteAllProperties | Operation::WriteMultipleProperties
                ) {
                    5
                } else {
                    4
                }
            );
        }
    });
}

fn verify_collection_publication(operation: Operation) {
    io().block_on(async {
        let addr = address();
        let td = collection_description(addr, operation);
        let mut server = Fixture::closed(td.clone());
        let mut peer = TcpStream::connect(addr).unwrap();
        if operation == Operation::ReadMultipleProperties {
            write!(peer, "POST /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 6\r\n\r\n[\"on\"]").unwrap();
        } else if matches!(operation, Operation::WriteAllProperties | Operation::WriteMultipleProperties) {
            let body = br#"{"on":false,"secret":"updated"}"#;
            write!(peer, "PUT /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", body.len()).unwrap();
            peer.write_all(body).unwrap();
        } else {
            write!(peer, "GET /properties HTTP/1.1\r\nHost: {addr}\r\n\r\n").unwrap();
        }
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
        assert!(bytes.starts_with(if matches!(operation, Operation::WriteAllProperties | Operation::WriteMultipleProperties) { b"HTTP/1.1 204" } else { b"HTTP/1.1 200" }));
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
        let mut call = start_collection_at(&client, operation, Deadline::NONE).unwrap();
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

fn verify_collection_authorization(operation: Operation) {
    io().block_on(async {
        let addr = address();
        let td = collection_description(addr, operation);
        let accounts = bank();
        let owner = consumer_instance();
        let denied = Arc::new(AtomicBool::new(false));
        let client = client(td.clone(), denied.clone(), accounts.clone(), &owner);
        denied.store(true, Ordering::Relaxed);
        assert!(start_collection_at(&client, operation, Deadline::NONE).is_err());
        let mut server = Fixture::new(td);
        assert_eq!(server.controls.requests.get(), 0);
        denied.store(false, Ordering::Relaxed);
        server.app.denies.set(true);
        let mut call = start_collection_at(&client, operation, Deadline::NONE).unwrap();
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

fn verify_collection_cleanup(operation: Operation) {
    use std::sync::atomic::AtomicUsize;
    #[derive(Clone, Copy)]
    enum Mode {
        Unselected,
        UnsupportedData,
        Selected,
        Cancel,
    }
    struct DataProbe {
        actual: Arc<dyn ConsumerDataService>,
        queries: Arc<AtomicUsize>,
        supported: bool,
    }
    impl ConsumerDataService for DataProbe {
        fn content_type(&self) -> &str {
            self.actual.content_type()
        }
        fn validate(
            &self,
            s: clinkz_wot_core::value_type::TypeSchema,
            p: &Payload,
        ) -> CoreResult<()> {
            self.actual.validate(s, p)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            let mut p = self.actual.policy();
            p.retained_bytes += 128;
            p
        }
        fn supports_property_map(
            &self,
            s: &clinkz_wot_core::collection::PropertyCollectionSchema,
        ) -> bool {
            self.queries.fetch_add(1, Ordering::Relaxed);
            self.supported && self.actual.supports_property_map(s)
        }
        fn validate_property_map(
            &self,
            s: &clinkz_wot_core::collection::PropertyCollectionSchema,
            p: &Payload,
        ) -> CoreResult<()> {
            self.actual.validate_property_map(s, p)
        }
        fn supports_property_selection(&self, s: &collection::PropertyCollectionSchema) -> bool {
            self.queries.fetch_add(1, Ordering::Relaxed);
            self.supported && self.actual.supports_property_selection(s)
        }
        fn supports_property_write_selection(
            &self,
            s: &collection::PropertyCollectionSchema,
        ) -> bool {
            self.queries.fetch_add(1, Ordering::Relaxed);
            self.supported && self.actual.supports_property_write_selection(s)
        }
        fn validate_property_write_map(
            &self,
            s: &collection::PropertyCollectionSchema,
            p: &Payload,
        ) -> CoreResult<collection::PropertySelection> {
            self.actual.validate_property_write_map(s, p)
        }
        fn encode_property_selection(
            &self,
            s: &collection::PropertyCollectionSchema,
            selected: collection::PropertySelection,
        ) -> CoreResult<Payload> {
            self.actual.encode_property_selection(s, selected)
        }
        fn validate_selected_property_map(
            &self,
            s: &collection::PropertyCollectionSchema,
            selected: collection::PropertySelection,
            p: &Payload,
        ) -> CoreResult<()> {
            self.actual.validate_selected_property_map(s, selected, p)
        }
    }
    struct Probe {
        used: u64,
        queries: usize,
        jobs: usize,
        error: Option<CoreError>,
        selected: usize,
    }
    fn td_with_members(count: usize, operation: Operation) -> Value {
        let addr = address();
        let mut td = description(addr);
        td["forms"] = json!([
            {"op":operation.as_str(),"href":format!("http://{addr}/skip"),"htv:methodName":if operation == Operation::ReadMultipleProperties {"POST"} else if matches!(operation, Operation::WriteAllProperties | Operation::WriteMultipleProperties) {"PUT"} else {"GET"}},
            {"op":operation.as_str(),"href":format!("http://{addr}/properties"),"htv:methodName":if operation == Operation::ReadMultipleProperties {"POST"} else if matches!(operation, Operation::WriteAllProperties | Operation::WriteMultipleProperties) {"PUT"} else {"GET"}}
        ]);
        td["properties"] = json!({});
        for i in 0..count {
            td["properties"][format!("p{i}")] = json!({"type":"integer", "readOnly": !matches!(operation, Operation::WriteAllProperties | Operation::WriteMultipleProperties),
                "forms":[{"href":format!("http://{addr}/p{i}")}]});
        }
        td
    }
    fn attempt(td: &Value, mode: Mode, cleanup: u64) -> Probe {
        let thing: Thing = serde_json::from_value(td.clone()).unwrap();
        let mut reg = registration();
        let queries = Arc::new(AtomicUsize::new(0));
        reg.services.data = Arc::new(DataProbe {
            actual: reg.services.data,
            queries: queries.clone(),
            supported: !matches!(mode, Mode::UnsupportedData),
        });
        let accounts = bank();
        let registry = ConsumerRegistry::try_new(vec![reg], &accounts).unwrap_or_else(|_| panic!());
        let mut budget = work().with_remaining(WorkClass::CleanupItems, cleanup);
        let mut jobs = 0;
        let prepared = registry.prepare(
            &thing,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector {
                form: Some(if matches!(mode, Mode::Unselected) {
                    99
                } else {
                    1
                }),
                binding: None,
            },
            &mut budget,
            |job| {
                jobs += 1;
                if matches!(mode, Mode::Cancel) {
                    job.step(&mut WorkBudget::new(), true);
                } else {
                    while !job.is_finished() {
                        job.step(&mut work(), false);
                    }
                }
            },
        );
        let (error, selected) = match &prepared {
            Ok(p) => (None, p.support().iter().filter(|s| s.selected).count()),
            Err(e) => (Some(e.clone()), 0),
        };
        drop(prepared);
        drop(registry);
        clean(&accounts);
        Probe {
            used: cleanup - budget.remaining(WorkClass::CleanupItems),
            queries: queries.load(Ordering::Relaxed),
            jobs,
            error,
            selected,
        }
    }
    let mut previous = None;
    for count in [1, 10] {
        let td = td_with_members(count, operation);
        let walk = attempt(&td, Mode::Unselected, 1_000_000);
        let temporary = attempt(&td, Mode::UnsupportedData, 1_000_000);
        let compiled = attempt(&td, Mode::Selected, 1_000_000);
        let schema_drop = count as u64 + 1;
        assert!(matches!(
            walk.error,
            Some(CoreError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            temporary.error,
            Some(CoreError::UnsupportedOperation(_))
        ));
        assert_eq!((temporary.queries, temporary.jobs), (1, 0));
        assert_eq!(
            temporary.used - walk.used,
            schema_drop,
            "temporary owner needs its own credit"
        );
        assert!(compiled.error.is_none());
        assert_eq!(compiled.selected, 1);
        let native_cleanup = compiled.used - temporary.used;
        let total = compiled.used - walk.used;
        if let Some(one) = previous {
            assert_eq!(total - one, 9, "one prepared content owner grows by nine members");
        }
        previous = Some(total);
        println!(
            "{count} members: one schema owner cleanup={schema_drop}, native cleanup={native_cleanup}, beyond walk={total}"
        );

        // Cancelling the native job fixes the credit spent up to its start,
        // independently of later TD/Form walking. Use that observed boundary to
        // test one-unit shortages before common construction and native preparation.
        let cancelled = attempt(&td, Mode::Cancel, 1_000_000);
        assert!(matches!(cancelled.error, Some(CoreError::Cancelled(_))));
        assert_eq!(cancelled.jobs, 1);
        for (credit, expected_queries) in [
            (cancelled.used - native_cleanup - 1, 0),
            (cancelled.used - 1, 1),
        ] {
            let refused = attempt(&td, Mode::Selected, credit);
            assert!(matches!(refused.error, Some(CoreError::Backpressure(_))));
            assert_eq!(refused.queries, expected_queries);
            assert_eq!(
                refused.jobs, 0,
                "short schema credit cannot start a native job"
            );
            assert!(
                refused.used < credit,
                "failed atomic payment keeps the short credit"
            );
        }
        let exact = attempt(&td, Mode::Selected, compiled.used);
        assert!(exact.error.is_none());
        assert_eq!(exact.selected, 1);
        assert_eq!(exact.used, compiled.used);
        let rollback = attempt(&td, Mode::Selected, compiled.used - 1);
        assert!(rollback.error.is_some());
        assert_eq!(rollback.jobs, 1, "late refusal releases the retained content");
    }
    // A supported first member is acquired before a later unsupported schema;
    // partial construction has the same prepaid temporary disposal bound.
    let mut td = td_with_members(2, operation);
    td["properties"]["p1"]["minimum"] = json!(0);
    let walk = attempt(&td, Mode::Unselected, 1_000_000);
    let partial = attempt(&td, Mode::Selected, 1_000_000);
    assert!(matches!(
        partial.error,
        Some(CoreError::UnsupportedOperation(_))
    ));
    assert_eq!((partial.queries, partial.jobs), (0, 0));
    assert_eq!(partial.used - walk.used, 3);
}

fn verify_collection_validator_bound(operation: Operation) {
    struct ChangingNative {
        actual: HttpFiniteBinding,
        allowance: Rc<Cell<u64>>,
        validated: Rc<Cell<bool>>,
        hold_close: Rc<Cell<bool>>,
        sends: Rc<Cell<usize>>,
    }
    impl FiniteValidator for ChangingNative {
        fn additional_validation_work(&self) -> u64 {
            self.allowance.get()
        }
        fn validate_input(&self, c: &FiniteCandidate, i: &InteractionInput) -> CoreResult<()> {
            self.actual.validator().unwrap().validate_input(c, i)
        }
        fn validate_output(
            &self,
            c: &FiniteCandidate,
            i: &InteractionInput,
            o: &InteractionOutput,
        ) -> CoreResult<()> {
            self.validated.set(true);
            self.actual.validator().unwrap().validate_output(c, i, o)
        }
    }
    impl FiniteIngressBinding for ChangingNative {
        type Configuration = HttpFiniteConfig;
        type Reply = HttpFiniteReply;
        fn identity(&self) -> (BindingId, BindingGeneration) {
            self.actual.identity()
        }
        fn configuration(&self) -> &HttpFiniteConfig {
            self.actual.configuration()
        }
        fn validator(&self) -> Option<&dyn FiniteValidator> {
            Some(self)
        }
        fn poll_start(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
            self.actual.poll_start(cx)
        }
        fn poll_receive(
            &mut self,
            cx: &mut Context<'_>,
            t: MonotonicInstant,
        ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
            self.actual.poll_receive(cx, t)
        }
        fn poll_send(
            &mut self,
            cx: &mut Context<'_>,
            r: &RawFiniteReply<Self::Reply>,
        ) -> Poll<CoreResult<()>> {
            self.sends.set(self.sends.get() + 1);
            self.actual.poll_send(cx, r)
        }
        fn poll_close(
            &mut self,
            cx: &mut Context<'_>,
            i: Option<&FiniteIngress<Self::Reply>>,
            r: Option<&RawFiniteReply<Self::Reply>>,
            e: Option<&CoreError>,
        ) -> Poll<CoreResult<FiniteDelivery>> {
            let reply = r.expect("original accepted exchange stays owned");
            assert!(i.is_none());
            assert_eq!(reply.effect, FiniteEffect::Unknown);
            assert_eq!(reply.operation, Some(reply.ingress.candidates[0].operation));
            assert_eq!(reply.ingress.candidates[0].target, AffordanceTarget::Thing);
            assert!(matches!(reply.result, Err(CoreError::Validation(_))));
            assert!(matches!(e, Some(CoreError::Validation(_))));
            if self.hold_close.get() {
                Poll::Pending
            } else {
                self.actual.poll_close(cx, i, r, e)
            }
        }
    }
    struct ChangingHandler(Rc<Cell<u64>>, u64, Operation);
    impl RawFiniteHandler for ChangingHandler {
        fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
            Ok(())
        }
        fn handle(
            &self,
            _: HandlerContext<'_>,
            _: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            self.0.set(self.1);
            if matches!(
                self.2,
                Operation::WriteAllProperties | Operation::WriteMultipleProperties
            ) {
                return Ok(InteractionOutput::empty());
            }
            Ok(InteractionOutput::with_data(payload(
                br#"{"level":7,"on":true}"#,
            )))
        }
    }
    for increase in [false, true] {
        let addr = address();
        let auth = authority(Generation::INITIAL);
        let key = auth.key();
        let listen = HttpProducerConfig::for_loopback(addr).unwrap();
        let provider = HttpFiniteProducer::try_new(
            listen,
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap();
        let prepared = prepare_thing(
            &collection_description(addr, operation).to_string(),
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |_, _| true,
        )
        .unwrap();
        let program = prepared
            .operations()
            .iter()
            .find(|o| o.site == FormSite::Thing)
            .unwrap()
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone();
        let config = HttpFiniteConfig::try_new(listen, program, key, AffordanceTarget::Thing, 1000)
            .unwrap_or_else(|_| panic!());
        let initial = config.additional_validation_work();
        let next = if increase { initial + 1 } else { initial - 1 };
        let allowance = Rc::new(Cell::new(initial));
        let hold_close = Rc::new(Cell::new(true));
        let sends = Rc::new(Cell::new(0));
        let validated = Rc::new(Cell::new(false));
        let mut owner = RawProducerServient::try_new(
            vec![RawProducerInstance::new(
                auth,
                vec![RawFiniteBehavior::new(
                    AffordanceTarget::Thing,
                    operation,
                    ChangingHandler(allowance.clone(), next, operation),
                )],
            )],
            RawProducerRegistryLimits {
                instances: 1,
                behaviors_per_instance: 1,
                target_name_bytes: 256,
            },
        )
        .unwrap_or_else(|_| panic!());
        let mut native = RawFiniteRuntime::try_new(
            ChangingNative {
                actual: HttpFiniteBinding::new(
                    config.clone(),
                    BindingId::new(97),
                    BindingGeneration::INITIAL,
                ),
                allowance: allowance.clone(),
                validated: validated.clone(),
                hold_close: hold_close.clone(),
                sends: sends.clone(),
            },
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
        assert_eq!(
            native.step(&mut cx(), &mut work(), now(0)),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
        native.activate_raw().unwrap();
        let mut peer = TcpStream::connect(addr).unwrap();
        if operation == Operation::ReadMultipleProperties {
            write!(peer, "POST /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 14\r\n\r\n[\"level\",\"on\"]").unwrap();
        } else if matches!(
            operation,
            Operation::WriteAllProperties | Operation::WriteMultipleProperties
        ) {
            let body = br#"{"on":false,"secret":"updated"}"#;
            write!(peer, "PUT /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", body.len()).unwrap();
            peer.write_all(body).unwrap();
        } else {
            write!(peer, "GET /properties HTTP/1.1\r\nHost: {addr}\r\n\r\n").unwrap();
        }
        let start = Instant::now();
        while native.state() != RawFiniteState::Dispatching {
            let _ = owner
                .step(&mut native, &mut cx(), &mut work(), now(0))
                .unwrap();
            assert!(start.elapsed() < Duration::from_secs(3));
        }
        let prepaid = 1 + (256 + 1) + 1 + 4096 + 64 * 2 + 4096 + initial;
        let mut budget = work().with_remaining(WorkClass::HandlerSteps, prepaid);
        let _ = owner
            .step(&mut native, &mut cx(), &mut budget, now(0))
            .unwrap();
        assert_eq!(budget.remaining(WorkClass::HandlerSteps), 0);
        assert_eq!(allowance.get(), next);
        let output_ran = validated.get();
        println!(
            "Output validation ran={output_ran}; captured bound={initial}; current bound={}",
            allowance.get()
        );
        assert_eq!(native.state(), RawFiniteState::Closing);
        assert!(
            matches!(native.first_cause(), Some(CoreError::Validation(c)) if c.phase() == ErrorPhase::Validate)
        );
        assert_eq!(
            native.step(&mut cx(), &mut WorkBudget::new(), now(0)),
            Poll::Pending
        );
        assert_eq!(native.step(&mut cx(), &mut work(), now(0)), Poll::Pending);
        assert!(
            TcpListener::bind(addr).is_err(),
            "pending retirement retains the socket"
        );
        allowance.set(initial); // Restoring metadata cannot reopen a fenced exchange.
        drop(owner);
        assert_eq!(native.step(&mut cx(), &mut work(), now(0)), Poll::Pending);
        assert_eq!(sends.get(), 0);
        hold_close.set(false);
        assert_eq!(
            native.step(&mut cx(), &mut work(), now(0)),
            Poll::Ready(RawFiniteState::Closed)
        );
        let result = native.take_settlement().unwrap();
        assert_eq!(result.recipient, Some(key));
        assert_eq!(result.target, Some(AffordanceTarget::Thing));
        assert_eq!(result.operation, Some(operation));
        assert_eq!(result.effect, FiniteEffect::Unknown);
        assert_eq!(result.delivery, FiniteDelivery::NotDelivered);
        assert!(matches!(result.result, Err(CoreError::Validation(_))));
        drop(native);
        drop(peer);
        drop(TcpListener::bind(addr).unwrap());
        assert!(
            !output_ran,
            "output validation ran after its bound changed, with no credit for the increased allowance"
        );
    }
}
