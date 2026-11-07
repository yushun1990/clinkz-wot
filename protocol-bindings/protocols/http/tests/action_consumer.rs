#![cfg(feature = "prepare")]
//! The public TD-prepared Consumer against the qualified HTTP Producer.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{
    consumer::*, consumer_action::*, data_schema::PreparedSchema, host_accounts::*, *,
};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient::producer_composition::*;
use clinkz_wot_servient_finite::{consumer_action::*, *};
use clinkz_wot_td::thing::Thing;
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::RefCell,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
#[path = "support/action.rs"]
mod fixture;
use fixture::{Counters, addr, budget, key, now, td};
const STRUCTURED_INPUT: &[u8] = br#"{"request":{"level":12.0e0,"label":"\u706f\u5149"}}"#;
const STRUCTURED_OUTPUT: &[u8] = br#"{"result":{"level":2.4e1,"label":"\u5b8c\u6210"}}"#;
fn structured_td(address: std::net::SocketAddr) -> Arc<str> {
    let mut text: Value = serde_json::from_str(&td(address, None)).unwrap();
    let role = |name: &str, minimum, maximum| {
        json!({
            "type":"object", "required":[name], "properties":{
                name:{"type":"object", "required":["level","label"], "properties":{
                    "level":{"type":"integer", "minimum":minimum, "maximum":maximum},
                    "label":{"type":"string", "minLength":2, "maxLength":3}
                }}
            }
        })
    };
    text["actions"]["run"]["input"] = role("request", 1, 12);
    text["actions"]["run"]["output"] = role("result", 20, 30);
    Arc::from(text.to_string())
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn limits() -> ResourceLimits {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    limits
}
fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(&limits()))
}
fn clean(b: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(b.account(k).used(), 0, "{k:?}");
    }
}
fn instance() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
}
fn io() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
}
fn payload(bytes: &[u8]) -> Option<Payload> {
    Some(Payload::new(bytes, "application/json"))
}
#[derive(Default)]
struct Policy {
    denied: AtomicBool,
    reject_output: AtomicBool,
    unavailable: AtomicBool,
    lose_data_on_query_access: AtomicBool,
    validations: AtomicUsize,
    log: Mutex<Vec<(BindingId, Operation, String)>>,
}
struct Data(Arc<Policy>, Arc<dyn ConsumerDataService>);
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn policy(&self) -> ConsumerDataPolicy {
        self.1.policy()
    }
    fn available(&self) -> bool {
        !self.0.unavailable.load(Ordering::Relaxed)
    }
    fn supports_schema(&self, schema: &PreparedSchema) -> bool {
        self.1.supports_schema(schema)
    }
    fn validate(&self, schema: &PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.0.validations.fetch_add(1, Ordering::Relaxed);
        callback(CallbackEdge::Validate);
        if self.0.reject_output.load(Ordering::Relaxed) && p.body.as_ref() == b"24" {
            return Err(invalid());
        }
        self.1.validate(schema, p)
    }
}
struct Access(Arc<Policy>, Arc<dyn ConsumerAccessService>);
impl ConsumerAccessService for Access {
    fn authorize(&self, b: BindingRegistrationIdentity, t: &str, o: Operation) -> CoreResult<()> {
        self.1.authorize(b, t, o)
    }
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.1.authorize_instance(consumer, b, target, op)?;
        callback(CallbackEdge::Access);
        if op == Operation::QueryAction
            && self
                .0
                .lose_data_on_query_access
                .swap(false, Ordering::Relaxed)
        {
            self.0.unavailable.store(true, Ordering::Relaxed);
        }
        self.0
            .log
            .lock()
            .unwrap()
            .push((b.binding_id(), op, target.to_owned()));
        if self.0.denied.load(Ordering::Relaxed) {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Binding, RetryClass::Never),
            });
        }
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        64 + self.1.retained_bytes()
    }
    fn work(&self) -> u64 {
        1 + self.1.work()
    }
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
}
fn definition(
    text: &str,
    b: Arc<HostResourceAccounts>,
    p: Arc<Policy>,
    separate: bool,
) -> CoreResult<ConsumerActionDefinition> {
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let actual = config.consumer_services();
    let services = ConsumerServices {
        data: Arc::new(Data(p.clone(), actual.data)),
        access: Arc::new(Access(p, actual.access)),
    };
    let count = if separate { 3 } else { 1 };
    let registry = ConsumerActionRegistry::try_new(
        (0..count)
            .map(|i| ConsumerActionRegistration {
                preparation: Box::new(HttpActionConsumer::new(
                    config,
                    BindingId::new(90 + i),
                    BindingGeneration::INITIAL,
                    i,
                )),
                services: services.clone(),
            })
            .collect(),
    )?;
    let selector = |n| ConsumerSelector {
        binding: Some(BindingId::new(n)),
        ..Default::default()
    };
    let selectors = if separate {
        ConsumerActionSelectors {
            invoke: selector(90),
            query: selector(91),
            cancel: selector(92),
        }
    } else {
        ConsumerActionSelectors::default()
    };
    let thing: Thing = serde_json::from_str(text).unwrap();
    registry.prepare_action(&thing, "run", selectors, &limits(), b, &mut budget())
}
fn client(
    text: &str,
    b: Arc<HostResourceAccounts>,
    p: Arc<Policy>,
    owner: &ConsumerInstance,
    separate: bool,
    slots: usize,
) -> ConsumerActionRuntime {
    let d = definition(text, b, p, separate).unwrap_or_else(|e| match e {
        CoreError::LimitExceeded {
            resource,
            limit,
            requested,
            observed,
            ..
        } => panic!("{resource:?} limit={limit} requested={requested:?} used={observed:?}"),
        e => panic!("{e:?}"),
    });
    ConsumerActionRuntime::try_new(owner.reference(), vec![d], slots)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
struct Producer {
    owner: RawProducerServient,
    exposure: ComposedProducerExposure,
    address: std::net::SocketAddr,
    counts: Rc<Counters>,
    bank: Arc<HostResourceAccounts>,
}
impl Producer {
    fn replace(&mut self) -> ComposedProducerExposure {
        let old = self.exposure.key();
        self.exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .begin_close();
        let counts = Rc::new(Counters::default());
        let mut next = fixture::create(
            &self.owner,
            key(Generation::INITIAL.checked_next().unwrap()),
            self.address,
            None,
            counts.clone(),
            Some(old),
            self.bank.clone(),
        );
        next.participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .install_closed()
            .unwrap();
        next.publish(&mut self.owner, &mut budget()).unwrap();
        self.counts = counts;
        std::mem::replace(&mut self.exposure, next)
    }
    fn new() -> Self {
        let address = addr();
        Self::from_text(address, td(address, None), Rc::new(Counters::default()))
    }
    fn structured() -> Self {
        let address = addr();
        let counts = Rc::new(Counters::default());
        *counts.input.borrow_mut() = Some(STRUCTURED_INPUT.to_vec());
        *counts.output.borrow_mut() = Some(STRUCTURED_OUTPUT.to_vec());
        Self::from_text(address, structured_td(address), counts)
    }
    fn from_text(address: std::net::SocketAddr, text: Arc<str>, counts: Rc<Counters>) -> Self {
        let bank = bank();
        let owner = fixture::owner();
        let mut exposure = fixture::create_from_text(
            &owner,
            key(Generation::INITIAL),
            text,
            address,
            None,
            counts.clone(),
            None,
            bank.clone(),
        );
        exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap()
            .install_closed()
            .unwrap();
        let mut result = Self {
            owner,
            exposure,
            address,
            counts,
            bank,
        };
        result
            .exposure
            .publish(&mut result.owner, &mut budget())
            .unwrap();
        result
    }
    fn step(&mut self) {
        let native = self
            .exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap();
        let _ = native.step(&mut self.owner, &mut budget()).unwrap();
        native.take_delivery();
        let _ = native
            .progress_actions(&mut self.owner, &mut budget())
            .unwrap();
    }
    fn raw_invoke(&mut self, body: &str) -> String {
        let mut peer = TcpStream::connect(self.address).unwrap();
        peer.set_nonblocking(true).unwrap();
        peer.write_all(format!("POST /run HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.address, body.len()).as_bytes()).unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        let mut bytes = Vec::new();
        loop {
            self.step();
            let mut buffer = [0; 512];
            match peer.read(&mut buffer) {
                Ok(0) => return String::from_utf8(bytes).unwrap(),
                Ok(n) => bytes.extend_from_slice(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("{e}"),
            }
            assert!(Instant::now() < end);
        }
    }
    async fn drive(&mut self, h: &ConsumerActionHandle, expected: ConsumerActionState) {
        for _ in 0..1000 {
            self.step();
            let s = h.progress(&mut cx(), now(), &mut budget()).unwrap();
            if s == expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("expected {expected:?}, got {:?}", h.state());
    }
    fn finish(mut self) {
        self.counts.complete.set(true);
        for _ in 0..16 {
            self.step();
        }
        let native = self
            .exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap();
        while let Some(s) = native
            .reap_completed(&mut self.owner, Duration::ZERO)
            .unwrap()
        {
            drop(s);
        }
        native.begin_close();
        self.exposure
            .retire(&mut self.owner, &mut budget())
            .unwrap();
        drop(self.exposure);
        drop(self.owner);
        clean(&self.bank);
        drop(TcpListener::bind(self.address).unwrap());
    }
}
#[test]
fn accepted_pending_query_completed_uses_selected_forms_and_independent_identity() {
    io().block_on(async {
        let mut producer = Producer::new();
        let b = bank();
        let owner = instance();
        let policy = Arc::new(Policy::default());
        // Unspecified synchronicity is a continuing possibility, not a finite guarantee.
        let mut text: Value = serde_json::from_str(&td(producer.address, None)).unwrap();
        text["actions"]["run"]
            .as_object_mut()
            .unwrap()
            .remove("synchronous");
        let runtime = client(
            &text.to_string(),
            b.clone(),
            policy.clone(),
            &owner,
            true,
            1,
        );
        drop(text);
        let h = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        let identity = h.identity().unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        assert_eq!(producer.counts.invoked.get(), 1);
        assert!(h.take_settlement(now(), &mut budget()).is_err());
        h.query(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        assert_eq!(h.identity().unwrap(), identity);
        producer.counts.complete.set(true);
        h.query(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Completed).await;
        // Unread completion retains its slot. No implicit reinvocation on pressure.
        assert!(
            runtime
                .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                .is_err()
        );
        let settlement = h.take_settlement(now(), &mut budget()).unwrap();
        assert_eq!(settlement.identity(), &identity);
        assert_eq!(
            settlement.result().unwrap().data().unwrap().body.as_ref(),
            b"24"
        );
        assert_eq!(settlement.effect(), ConsumerEffect::Unknown);
        assert_eq!(
            settlement.selected_form(Operation::QueryAction).unwrap(),
            (definition_identity(91), 1)
        );
        assert!(h.query(now(), &mut budget()).is_err());
        let log = policy.log.lock().unwrap();
        assert!(log.iter().any(|(b, o, t)| *b == BindingId::new(91)
            && *o == Operation::QueryAction
            && t.contains("/status?id=")));
        drop(log);
        drop(h);
        drop(runtime);
        drop(owner);
        assert!(b.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop(settlement);
        clean(&b);
        producer.finish();
    });
}
fn definition_identity(id: u32) -> BindingRegistrationIdentity {
    let input = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(id),
        BindingGeneration::INITIAL,
        id - 90,
    )
    .unwrap();
    input.preparation.identity()
}

#[test]
fn structured_actions_validate_both_roles_before_effects_and_keep_original_content_funded() {
    io().block_on(async {
        let mut producer = Producer::structured();
        let b = bank();
        let owner = instance();
        let runtime = client(
            &structured_td(producer.address),
            b.clone(),
            Arc::new(Policy::default()),
            &owner,
            true,
            2,
        );
        for invalid in [
            r#"{}"#,
            r#"{"request":{"level":12}}"#,
            r#"{"request":{"level":13,"label":"ok"}}"#,
            r#"{"request":{"level":12,"label":"a"}}"#,
            r#"{"request":{"level":12,"label":"ok","\u006cevel":11}}"#,
        ] {
            assert!(
                runtime
                    .invoke_action(
                        "run",
                        payload(invalid.as_bytes()),
                        Deadline::NONE,
                        now(),
                        &mut budget()
                    )
                    .is_err()
            );
            // An independent peer bypasses the Consumer: the qualified Producer
            // must reject this same nested value before handler authorization.
            assert!(producer.raw_invoke(invalid).starts_with("HTTP/1.1 400"));
        }
        assert_eq!(producer.counts.invoked.get(), 0);
        assert_eq!(producer.counts.authorized.get(), 0);
        let first = runtime
            .invoke_action(
                "run",
                payload(STRUCTURED_INPUT),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        producer.drive(&first, ConsumerActionState::Pending).await;
        let second = runtime
            .invoke_action(
                "run",
                payload(STRUCTURED_INPUT),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        producer.drive(&second, ConsumerActionState::Pending).await;
        assert_ne!(first.identity().unwrap(), second.identity().unwrap());
        assert_eq!(producer.counts.invoked.get(), 2);
        // Remote cancellation is a request; a validated result can still follow.
        first.request_cancel(now(), &mut budget()).unwrap();
        producer.drive(&first, ConsumerActionState::Pending).await;
        assert_eq!(producer.counts.cancelled.get(), 1);
        producer.counts.complete.set(true);
        for h in [&first, &second] {
            h.query(now(), &mut budget()).unwrap();
            producer.drive(h, ConsumerActionState::Completed).await;
        }
        let a = first.take_settlement(now(), &mut budget()).unwrap();
        let c = second.take_settlement(now(), &mut budget()).unwrap();
        for s in [&a, &c] {
            assert_eq!(
                s.result().unwrap().data().unwrap().body.as_ref(),
                STRUCTURED_OUTPUT
            );
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
        }
        drop((first, second, runtime, owner));
        assert!(b.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop(a);
        assert!(b.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        drop(c);
        clean(&b);
        producer.finish();
    });
}

#[test]
fn structured_producer_completion_is_checked_against_its_distinct_output_role() {
    io().block_on(async {
        let mut producer = Producer::structured();
        let b = bank();
        let owner = instance();
        let runtime = client(
            &structured_td(producer.address),
            b.clone(),
            Arc::new(Policy::default()),
            &owner,
            false,
            1,
        );
        let h = runtime
            .invoke_action(
                "run",
                payload(STRUCTURED_INPUT),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        // Valid input shape and bounds do not qualify an output: the result
        // must have the output role's required names and integer boundaries.
        *producer.counts.output.borrow_mut() = Some(STRUCTURED_INPUT.to_vec());
        producer.counts.complete.set(true);
        h.query(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Failed).await;
        let s = h.take_settlement(now(), &mut budget()).unwrap();
        assert!(s.native_result().unwrap().is_err());
        let native = producer
            .exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap();
        let id = native.action_ids().next().unwrap();
        let produced = native
            .acknowledge(&mut producer.owner, id)
            .unwrap()
            .unwrap();
        assert!(matches!(produced.result(), Err(CoreError::Validation(_))));
        drop(produced);
        assert_eq!(s.effect(), ConsumerEffect::Unknown);
        assert_eq!(producer.counts.invoked.get(), 1);
        drop((s, h, runtime, owner));
        clean(&b);
        producer.finish();
    });
}

struct TracedPreparation(HttpActionConsumer, Arc<Mutex<Vec<usize>>>);
impl ConsumerActionPreparation for TracedPreparation {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.0.identity()
    }
    fn bounds(&self) -> ConsumerPreparationBounds {
        self.0.bounds()
    }
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        let Some(PreparedSchema::Object(object)) = &input.schemas.input else {
            panic!()
        };
        self.1
            .lock()
            .unwrap()
            .push(object.properties().as_ptr() as usize);
        self.0.compile_action(input)
    }
}
struct TypesOnly(Arc<dyn ConsumerDataService>);
impl ConsumerDataService for TypesOnly {
    fn content_type(&self) -> &str {
        self.0.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        self.0.policy()
    }
    fn validate(&self, s: &PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.0.validate(s, p)
    }
    // Default support intentionally qualifies only type assertions.
}
#[test]
fn action_meaning_is_built_once_and_actual_support_work_and_schema_bounds_still_qualify() {
    let address = addr();
    let text = structured_td(address);
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    for case in 0..6 {
        let b = bank();
        let traces = Arc::new(Mutex::new(Vec::new()));
        let mut services = config.consumer_services();
        if case == 1 {
            services.data = Arc::new(TypesOnly(services.data));
        }
        let registry = ConsumerActionRegistry::try_new(vec![ConsumerActionRegistration {
            preparation: Box::new(TracedPreparation(
                HttpActionConsumer::new(config, BindingId::new(90), BindingGeneration::INITIAL, 0),
                traces.clone(),
            )),
            services,
        }])
        .unwrap();
        let mut source: Value = serde_json::from_str(&text).unwrap();
        let mut limits = limits();
        match case {
            2 => {
                assert!(limits.set(ResourceKind::CleanupWorkItemsPerStepMax, Some(1)));
            }
            3 => {
                source["actions"]["run"]["output"]["properties"]["result"]["properties"]["label"]
                    ["pattern"] = json!(".*")
            }
            4 => {
                let mut schema = json!({"type":"integer"});
                for _ in 0..9 {
                    schema = json!({"type":"object","properties":{"x":schema}});
                }
                source["actions"]["run"]["input"] = schema;
            }
            _ => {}
        }
        let thing: Thing = serde_json::from_value(source).unwrap();
        let mut work = budget();
        if case == 5 {
            work = work.with_remaining(WorkClass::ProviderProbes, 0);
        }
        let result = registry.prepare_action(
            &thing,
            "run",
            ConsumerActionSelectors::default(),
            &limits,
            b.clone(),
            &mut work,
        );
        if case == 0 {
            let definition = result.unwrap();
            let trace = traces.lock().unwrap();
            assert_eq!(trace.len(), 3);
            assert!(
                trace.iter().all(|p| *p == trace[0]),
                "each native operation sees the same immutable backing"
            );
            assert!(definition.input().schemas.retained_bytes() > 0);
            assert!(
                b.account(ResourceKind::EngineLiveBytesGlobalMax).used()
                    >= definition.input().schemas.retained_bytes() as u64
            );
            drop(definition);
        } else {
            assert!(result.is_err(), "case {case}");
            assert!(
                traces.lock().unwrap().is_empty(),
                "refusal before native compilation"
            );
        }
        drop((registry, thing));
        clean(&b);
    }
    TcpListener::bind(address).unwrap();
}
#[test]
fn described_cancel_acknowledges_request_and_query_can_still_complete() {
    io().block_on(async {
        let mut producer = Producer::new();
        let b = bank();
        let owner = instance();
        let p = Arc::new(Policy::default());
        let runtime = client(
            &td(producer.address, None),
            b.clone(),
            p.clone(),
            &owner,
            true,
            2,
        );
        let h = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        h.request_cancel(now(), &mut budget()).unwrap();
        assert_eq!(
            h.remote_cancellation().unwrap(),
            ConsumerActionCancellation::Uncertain
        );
        producer.drive(&h, ConsumerActionState::Pending).await;
        assert_eq!(
            h.remote_cancellation().unwrap(),
            ConsumerActionCancellation::Requested
        );
        assert_eq!(producer.counts.cancelled.get(), 1);
        assert!(
            p.log
                .lock()
                .unwrap()
                .iter()
                .any(|(b, op, t)| *b == BindingId::new(92)
                    && *op == Operation::CancelAction
                    && t.contains("/stop?id="))
        );
        h.query(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        producer.counts.complete.set(true);
        producer.step();
        h.request_cancel(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        assert_eq!(
            h.remote_cancellation().unwrap(),
            ConsumerActionCancellation::AlreadyTerminal
        );
        assert!(h.take_settlement(now(), &mut budget()).is_err());
        h.query(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Completed).await;
        let s = h.take_settlement(now(), &mut budget()).unwrap();
        assert!(s.result().is_ok());
        assert_eq!(
            s.remote_cancellation(),
            ConsumerActionCancellation::AlreadyTerminal
        );
        drop(s);
        drop(h);
        drop(runtime);
        drop(owner);
        clean(&b);
        producer.finish();
    });
}
#[test]
fn input_and_completion_validation_and_remote_failure_keep_original_evidence() {
    io().block_on(async {
        for remote_invalid in [false, true] {
            let mut producer = Producer::new();
            let b = bank();
            let owner = instance();
            let p = Arc::new(Policy::default());
            let runtime = client(
                &td(producer.address, None),
                b.clone(),
                p.clone(),
                &owner,
                false,
                2,
            );
            let rejection = runtime
                .invoke_action(
                    "run",
                    payload(b"\"bad\""),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .err()
                .unwrap();
            assert_eq!(rejection.into_parts().0.unwrap().body.as_ref(), b"\"bad\"");
            assert_eq!(producer.counts.invoked.get(), 0);
            let h = runtime
                .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                .ok()
                .unwrap();
            producer.drive(&h, ConsumerActionState::Pending).await;
            p.reject_output.store(!remote_invalid, Ordering::Relaxed);
            producer.counts.invalid.set(remote_invalid);
            producer.counts.complete.set(true);
            h.query(now(), &mut budget()).unwrap();
            producer
                .drive(
                    &h,
                    if remote_invalid {
                        ConsumerActionState::Failed
                    } else {
                        ConsumerActionState::Closed
                    },
                )
                .await;
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert!(s.result().is_err());
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
            assert_eq!(s.native_result().unwrap().is_err(), remote_invalid);
            if !remote_invalid {
                assert!(matches!(s.first_cause(), Some(CoreError::Validation(_))));
            }
            assert_eq!(producer.counts.invoked.get(), 1);
            drop(s);
            drop(h);
            drop(runtime);
            drop(owner);
            clean(&b);
            producer.finish();
        }
    });
}
#[test]
fn unsupported_described_mappings_and_semantics_refuse_before_io() {
    let address = addr();
    let original: Value = serde_json::from_str(&td(address, None)).unwrap();
    let variants = [
        ("/actions/run/synchronous", json!(true)),
        (
            "/actions/run/forms/1/href",
            json!(format!("mqtt://{address}/status")),
        ),
        ("/actions/run/forms/1/htv:methodName", json!("POST")),
        (
            "/actions/run/forms/2/href",
            json!(format!("http://{address}/invented")),
        ),
        ("/actions/run/uriVariables/id/type", json!("integer")),
    ];
    for (path, replacement) in variants {
        let mut value = original.clone();
        *value.pointer_mut(path).unwrap() = replacement;
        let b = bank();
        assert!(
            definition(
                &value.to_string(),
                b.clone(),
                Arc::new(Policy::default()),
                false
            )
            .is_err()
        );
        clean(&b);
    }
    let mut constrained = original.clone();
    constrained["actions"]["run"]["input"]["multipleOf"] = json!(2);
    let b = bank();
    assert!(
        definition(
            &constrained.to_string(),
            b.clone(),
            Arc::new(Policy::default()),
            false
        )
        .is_err()
    );
    clean(&b);
    let mut security = original;
    security["securityDefinitions"]["n"]["scheme"] = json!("basic");
    let b = bank();
    assert!(
        definition(
            &security.to_string(),
            b.clone(),
            Arc::new(Policy::default()),
            false
        )
        .is_err()
    );
    clean(&b);
    drop(TcpListener::bind(address).unwrap());
}
#[test]
fn local_cancellation_and_cancel_response_loss_do_not_claim_remote_rollback() {
    io().block_on(async {
        for remote_cancel in [false, true] {
            let mut producer = Producer::new();
            let b = bank();
            let owner = instance();
            let runtime = client(
                &td(producer.address, None),
                b.clone(),
                Arc::new(Policy::default()),
                &owner,
                false,
                1,
            );
            let h = runtime
                .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                .ok()
                .unwrap();
            producer.drive(&h, ConsumerActionState::Pending).await;
            if remote_cancel {
                h.request_cancel(now(), &mut budget()).unwrap();
                for _ in 0..1000 {
                    h.progress(&mut cx(), now(), &mut budget()).unwrap();
                    producer.step();
                    if producer.counts.cancelled.get() == 1 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                assert_eq!(producer.counts.cancelled.get(), 1);
            }
            // The Producer accepted cancellation but has not delivered its reply.
            h.cancel_local();
            producer.drive(&h, ConsumerActionState::Closed).await;
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
            assert_eq!(
                s.remote_cancellation(),
                if remote_cancel {
                    ConsumerActionCancellation::Uncertain
                } else {
                    ConsumerActionCancellation::NotRequested
                }
            );
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
            assert_eq!(producer.counts.invoked.get(), 1);
            drop(s);
            drop(h);
            drop(runtime);
            drop(owner);
            clean(&b);
            producer.finish();
        }
    });
}
#[test]
fn capacity_replacement_policy_deadline_and_abandonment_retire_original_work() {
    io().block_on(async {
        for reason in 0..5 {
            let mut producer = Producer::new();
            let b = bank();
            let owner = instance();
            let p = Arc::new(Policy::default());
            let runtime = client(
                &td(producer.address, None),
                b.clone(),
                p.clone(),
                &owner,
                false,
                2,
            );
            let account = b.account(ResourceKind::CleanupItemsMax);
            let remaining = account.limit() - account.used();
            let pressure = b.reserve(ResourceKind::CleanupItemsMax, remaining).unwrap();
            assert!(
                runtime
                    .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                    .is_err()
            );
            assert_eq!(producer.counts.invoked.get(), 0);
            drop(pressure);
            let deadline = if reason == 3 {
                Deadline::at(MonotonicInstant::new(ClockId::new(1), 1))
            } else {
                Deadline::NONE
            };
            let h = runtime
                .invoke_action("run", payload(b"12"), deadline, now(), &mut budget())
                .ok()
                .unwrap();
            // Native exchange concurrency is separate from retained invocation capacity.
            assert!(
                runtime
                    .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                    .is_err()
            );
            producer.drive(&h, ConsumerActionState::Pending).await;
            match reason {
                0 => owner.revoke(),
                1 => p.denied.store(true, Ordering::Relaxed),
                2 => p.unavailable.store(true, Ordering::Relaxed),
                3 => {
                    h.progress(
                        &mut cx(),
                        MonotonicInstant::new(ClockId::new(1), 1),
                        &mut budget(),
                    )
                    .unwrap();
                }
                _ => runtime.begin_close(),
            }
            if reason != 3 {
                producer.drive(&h, ConsumerActionState::Closed).await;
            }
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert!(s.result().is_err());
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
            assert_eq!(producer.counts.invoked.get(), 1);
            // A new incarnation at the same slot cannot inherit the old handle.
            let replacement = instance();
            assert_ne!(replacement.reference(), owner.reference());
            assert!(h.query(now(), &mut budget()).is_err());
            drop(s);
            drop(h);
            drop(runtime);
            drop(owner);
            drop(replacement);
            clean(&b);
            producer.finish();
        }
        let mut producer = Producer::new();
        let b = bank();
        let owner = instance();
        let runtime = client(
            &td(producer.address, None),
            b.clone(),
            Arc::new(Policy::default()),
            &owner,
            false,
            1,
        );
        let h = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        drop(h);
        runtime.step(&mut cx(), now(), &mut budget());
        assert!(runtime.is_idle());
        drop(runtime);
        drop(owner);
        clean(&b);
        producer.finish();
    });
}
#[test]
fn producer_replacement_rejects_original_token_and_cannot_transfer_or_replay_invocation() {
    io().block_on(async {
        let mut producer = Producer::new();
        let b = bank();
        let owner = instance();
        let runtime = client(
            &td(producer.address, None),
            b.clone(),
            Arc::new(Policy::default()),
            &owner,
            false,
            1,
        );
        let old = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        producer.drive(&old, ConsumerActionState::Pending).await;
        let old_counts = producer.counts.clone();
        let old_identity = old.identity().unwrap();
        let mut old_exposure = producer.replace();
        old.query(now(), &mut budget()).unwrap();
        producer.drive(&old, ConsumerActionState::Closed).await;
        let s = old.take_settlement(now(), &mut budget()).unwrap();
        assert!(s.result().is_err());
        assert_eq!(s.effect(), ConsumerEffect::Unknown);
        assert_eq!(producer.counts.invoked.get(), 0);
        assert_eq!(old_counts.invoked.get(), 1);
        // The vacated local slot can be reused only by an explicit new invocation.
        let new = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        assert_ne!(new.identity().unwrap(), old_identity);
        assert!(old.request_cancel(now(), &mut budget()).is_err());
        producer.drive(&new, ConsumerActionState::Pending).await;
        producer.counts.complete.set(true);
        new.query(now(), &mut budget()).unwrap();
        producer.drive(&new, ConsumerActionState::Completed).await;
        new.request_cancel(now(), &mut budget()).unwrap_err();
        assert!(
            new.take_settlement(now(), &mut budget())
                .unwrap()
                .result()
                .is_ok()
        );
        old_counts.complete.set(true);
        let native = old_exposure
            .participant_mut::<HttpActionRuntime>(0)
            .unwrap();
        let _ = native
            .progress_actions(&mut producer.owner, &mut budget())
            .unwrap();
        drop(
            native
                .reap_completed(&mut producer.owner, Duration::ZERO)
                .unwrap()
                .unwrap(),
        );
        old_exposure
            .retire(&mut producer.owner, &mut budget())
            .unwrap();
        drop(old_exposure);
        drop(s);
        drop(old);
        drop(new);
        drop(runtime);
        drop(owner);
        clean(&b);
        producer.finish();
    });
}
#[test]
fn absent_controls_are_unsupported_and_never_synthesized() {
    io().block_on(async {
        let mut producer = Producer::new();
        let b = bank();
        let owner = instance();
        let mut text: Value = serde_json::from_str(&td(producer.address, None)).unwrap();
        text["actions"]["run"]["forms"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        let runtime = client(
            &text.to_string(),
            b.clone(),
            Arc::new(Policy::default()),
            &owner,
            false,
            1,
        );
        let h = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        producer.drive(&h, ConsumerActionState::Pending).await;
        assert!(matches!(
            h.query(now(), &mut budget()),
            Err(CoreError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            h.request_cancel(now(), &mut budget()),
            Err(CoreError::UnsupportedOperation(_))
        ));
        h.cancel_local();
        producer.drive(&h, ConsumerActionState::Closed).await;
        assert_eq!(producer.counts.cancelled.get(), 0);
        drop(h.take_settlement(now(), &mut budget()).unwrap());
        drop(h);
        drop(runtime);
        drop(owner);
        clean(&b);
        producer.finish();
    });
}
#[test]
fn malformed_native_acceptance_and_framing_never_grant_completion_or_replay() {
    use std::io::{Read, Write};
    let bodies = [
        "{\"id\":\"0123456789abcdef-1\",\"status\":\"accepted\",\"output\":24}",
        "{\"id\":\"http://example.test/other\",\"status\":\"accepted\"}",
        "{\"id\":\"0123456789abcdef-1\",\"status\":\"accepted\",\"status\":\"accepted\"}",
        "{\"status\":\"completed\",\"output\":24}",
        "{\"id\":\"0123456789abcdef-1\",\"status\":\"accepted\"}",
    ];
    for (i, body) in bodies.iter().enumerate() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mut response = format!("HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            if i == 3 { "200 OK" } else { "202 Accepted" }, body.len()).into_bytes();
        if i == 4 {
            response.extend_from_slice(b"unexpected-trailing-bytes");
        }
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = [0; 8192];
            let _ = socket.read(&mut bytes).unwrap();
            for fragment in response.chunks(17) {
                socket.write_all(fragment).unwrap();
            }
            drop(socket);
            listener.set_nonblocking(true).unwrap();
            assert!(listener.accept().is_err());
        });
        io().block_on(async {
            let b = bank();
            let owner = instance();
            let runtime = client(
                &td(address, None),
                b.clone(),
                Arc::new(Policy::default()),
                &owner,
                false,
                1,
            );
            let h = runtime
                .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                .ok()
                .unwrap();
            for _ in 0..1000 {
                if h.progress(&mut cx(), now(), &mut budget()).unwrap()
                    == ConsumerActionState::Closed
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert!(s.result().is_err());
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
            assert!(h.query(now(), &mut budget()).is_err());
            drop(s);
            drop(h);
            drop(runtime);
            drop(owner);
            clean(&b);
        });
        peer.join().unwrap();
    }
}

#[test]
fn wire_location_cannot_replace_the_selected_query_form() {
    use std::io::{Read, Write};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        for (expected, code, body) in [
            (
                "POST /run HTTP/1.1",
                202,
                "{\"id\":\"0123456789abcdef-7\",\"status\":\"accepted\"}",
            ),
            (
                "GET /status?id=0123456789abcdef-7 HTTP/1.1",
                200,
                "{\"status\":\"completed\",\"output\":24}",
            ),
        ] {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            loop {
                let mut buf = [0; 1024];
                let n = socket.read(&mut buf).unwrap();
                assert_ne!(n, 0);
                request.extend_from_slice(&buf[..n]);
                let mut headers = [httparse::EMPTY_HEADER; 16];
                let mut parsed = httparse::Request::new(&mut headers);
                if let httparse::Status::Complete(end) = parsed.parse(&request).unwrap() {
                    let length = parsed
                        .headers
                        .iter()
                        .find(|h| h.name.eq_ignore_ascii_case("content-length"))
                        .map_or(0, |h| {
                            std::str::from_utf8(h.value)
                                .unwrap()
                                .parse::<usize>()
                                .unwrap()
                        });
                    if request.len() >= end + length {
                        break;
                    }
                }
            }
            assert!(request.starts_with(expected.as_bytes()));
            let location = if code == 202 {
                "Location: http://127.0.0.1:9/unselected\r\n"
            } else {
                ""
            };
            write!(socket, "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{location}Connection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    io().block_on(async {
        let b = bank();
        let owner = instance();
        let runtime = client(
            &td(address, None),
            b.clone(),
            Arc::new(Policy::default()),
            &owner,
            false,
            1,
        );
        let h = runtime
            .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
            .ok()
            .unwrap();
        for expected in [ConsumerActionState::Pending, ConsumerActionState::Completed] {
            for _ in 0..1000 {
                if h.progress(&mut cx(), now(), &mut budget()).unwrap() == expected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            assert_eq!(h.state().unwrap(), expected);
            if expected == ConsumerActionState::Pending {
                h.query(now(), &mut budget()).unwrap();
            }
        }
        assert!(
            h.take_settlement(now(), &mut budget())
                .unwrap()
                .result()
                .is_ok()
        );
        drop(h);
        drop(runtime);
        drop(owner);
        clean(&b);
    });
    peer.join().unwrap();
}

#[test]
fn unread_completion_remains_revocable_until_settlement_handoff() {
    io().block_on(async {
        for progress_before_handoff in [false, true] {
            for cause in 0..9 {
                let mut producer = Producer::new();
                let b = bank();
                let owner = instance();
                let p = Arc::new(Policy::default());
                let mut runtime = Some(client(
                    &td(producer.address, None),
                    b.clone(),
                    p.clone(),
                    &owner,
                    true,
                    1,
                ));
                let h = runtime
                    .as_ref()
                    .unwrap()
                    .invoke_action(
                        "run",
                        payload(b"12"),
                        Deadline::at(MonotonicInstant::new(ClockId::new(1), 1)),
                        now(),
                        &mut budget(),
                    )
                    .ok()
                    .unwrap();
                producer.drive(&h, ConsumerActionState::Pending).await;
                producer.counts.complete.set(true);
                h.query(now(), &mut budget()).unwrap();
                producer.drive(&h, ConsumerActionState::Completed).await;
                match cause {
                    0 => owner.revoke(),
                    1 => p.denied.store(true, Ordering::Relaxed),
                    2 => p.unavailable.store(true, Ordering::Relaxed),
                    3 => runtime.as_ref().unwrap().begin_close(),
                    4 => h.cancel_local(),
                    6 => drop(runtime.take()),
                    8 => p.lose_data_on_query_access.store(true, Ordering::Relaxed),
                    _ => {}
                }
                let time = match cause {
                    5 => MonotonicInstant::new(ClockId::new(1), 1),
                    7 => MonotonicInstant::new(ClockId::new(2), 0),
                    _ => now(),
                };
                if progress_before_handoff {
                    h.progress(&mut cx(), time, &mut budget()).unwrap();
                }
                let s = h.take_settlement(time, &mut budget()).unwrap();
                assert!(
                    s.result().is_err(),
                    "unread completion escaped cause {cause}"
                );
                match (cause, s.first_cause()) {
                    (0 | 3 | 6, Some(CoreError::StaleHandle(_)))
                    | (1, Some(CoreError::Security { .. }))
                    | (2 | 7 | 8, Some(CoreError::Validation(_)))
                    | (4, Some(CoreError::Cancelled(_)))
                    | (5, Some(CoreError::TimedOut(_))) => {}
                    _ => panic!("wrong first cause {cause}: {:?}", s.first_cause()),
                }
                assert_eq!(s.state(), ConsumerActionState::Closed);
                assert_eq!(
                    s.native_result()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .data()
                        .unwrap()
                        .body
                        .as_ref(),
                    b"24"
                );
                assert_eq!(s.effect(), ConsumerEffect::Unknown);
                assert_eq!(producer.counts.invoked.get(), 1);
                drop(s);
                drop(h);
                drop(runtime);
                drop(owner);
                clean(&b);
                producer.finish();
            }
        }
    });
}

#[test]
fn unread_success_requires_paid_fresh_handoff_and_keeps_its_slot_on_short_credit() {
    io().block_on(async {
        for deny_handoff in [false, true] {
            let mut producer = Producer::new();
            let b = bank();
            let owner = instance();
            let p = Arc::new(Policy::default());
            let d = definition(&td(producer.address, None), b.clone(), p.clone(), true).unwrap();
            let selected = d.input().query.as_ref().unwrap();
            let access_work = selected.services.access.work();
            let destination_work = selected.mapping.bounds().poll_work;
            let runtime = ConsumerActionRuntime::try_new(owner.reference(), vec![d], 1)
                .ok()
                .unwrap();
            let h = runtime
                .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                .ok()
                .unwrap();
            producer.drive(&h, ConsumerActionState::Pending).await;
            producer.counts.complete.set(true);
            h.query(now(), &mut budget()).unwrap();
            producer.drive(&h, ConsumerActionState::Completed).await;
            let before = b.account(ResourceKind::EngineLiveBytesGlobalMax).used();
            let access_calls = p.log.lock().unwrap().len();
            assert_eq!(p.validations.load(Ordering::Relaxed), 2);
            p.denied.store(deny_handoff, Ordering::Relaxed);
            assert!(matches!(
                h.take_settlement(now(), &mut WorkBudget::new()),
                Err(CoreError::Backpressure(_))
            ));
            for (class, required) in [
                (WorkClass::ProviderProbes, access_work),
                (WorkClass::BindingPolls, destination_work),
                (WorkClass::CleanupItems, 1),
            ] {
                let mut short = budget();
                short.set_remaining(class, required - 1);
                assert!(matches!(
                    h.take_settlement(now(), &mut short),
                    Err(CoreError::Backpressure(_))
                ));
                assert_eq!(h.state().unwrap(), ConsumerActionState::Completed);
                assert_eq!(p.log.lock().unwrap().len(), access_calls);
                assert_eq!(
                    b.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                    before
                );
                assert!(
                    runtime
                        .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                        .is_err()
                );
            }
            // A completed value needs a fresh access check, not another codec
            // callback or native exchange. Recent progress cannot cache permission.
            let mut handoff = WorkBudget::new()
                .with_remaining(WorkClass::ProviderProbes, access_work)
                .with_remaining(WorkClass::BindingPolls, destination_work)
                .with_remaining(WorkClass::CleanupItems, 1);
            let s = h.take_settlement(now(), &mut handoff).unwrap();
            assert_eq!(p.log.lock().unwrap().len(), access_calls + 1);
            assert_eq!(p.validations.load(Ordering::Relaxed), 2);
            assert_eq!(s.result().is_err(), deny_handoff);
            if deny_handoff {
                assert!(matches!(s.first_cause(), Some(CoreError::Security { .. })));
            }
            assert!(s.native_result().unwrap().is_ok());
            assert!(h.take_settlement(now(), &mut budget()).is_err());
            owner.revoke();
            drop(h);
            drop(runtime);
            drop(owner);
            assert_eq!(
                s.result().is_err(),
                deny_handoff,
                "the successful handoff is the acceptance boundary"
            );
            assert!(b.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
            drop(s);
            clean(&b);
            producer.finish();
        }
    });
}

#[derive(Default)]
struct RetirementProbe {
    blocked: AtomicBool,
    late_replies: AtomicUsize,
}
struct RetirementMapping {
    original: Box<dyn ConsumerActionMapping>,
    probe: Arc<RetirementProbe>,
}
impl ConsumerActionMapping for RetirementMapping {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.original.identity()
    }
    fn operation(&self) -> Operation {
        self.original.operation()
    }
    fn bounds(&self) -> ConsumerActionBounds {
        let mut b = self.original.bounds();
        b.exchange_bytes += std::mem::size_of::<RetirementExchange>() as u64;
        b.poll_work += 16;
        b
    }
    fn native_definition(&self) -> &dyn Any {
        self.original.native_definition()
    }
    fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()> {
        self.original.supports_continuation(invoke)
    }
    fn destination(&self, continuation: Option<&dyn Any>) -> CoreResult<Box<str>> {
        self.original.destination(continuation)
    }
    fn start(
        &self,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        Ok(Box::new(RetirementExchange {
            original: self.original.start(input, continuation)?,
            reply: None,
            polled: false,
            probe: self.probe.clone(),
        }))
    }
}
struct RetirementExchange {
    original: Box<dyn ConsumerActionExchange>,
    reply: Option<ConsumerActionReply>,
    polled: bool,
    probe: Arc<RetirementProbe>,
}
impl ConsumerActionExchange for RetirementExchange {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>> {
        assert!(
            !self.polled,
            "normal poll must not replay a failed exchange"
        );
        self.polled = true;
        if let Poll::Ready(result) = self.original.poll(cx) {
            self.reply = Some(result.unwrap());
        }
        Poll::Ready(Err(query_poll_error()))
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<Option<ConsumerActionReply>>> {
        if self.probe.blocked.load(Ordering::Relaxed) {
            return Poll::Pending;
        }
        if self.reply.is_none() {
            match self.original.poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => self.reply = Some(result?),
            }
        }
        match self.original.poll_close(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                assert!(result?.is_none());
                self.probe.late_replies.fetch_add(1, Ordering::Relaxed);
                Poll::Ready(Ok(self.reply.take()))
            }
        }
    }
    fn abort_local(self: Box<Self>) {
        self.original.abort_local();
    }
}
fn query_poll_error() -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(660, "query poll failed before retirement response"),
    )
}
#[test]
fn poll_error_preserves_and_validates_actual_http_output_arriving_during_retirement() {
    io().block_on(async {
        for (structured, invalid_output) in
            [(false, false), (false, true), (true, false), (true, true)]
        {
            let mut producer = if structured {
                Producer::structured()
            } else {
                Producer::new()
            };
            let b = bank();
            let owner = instance();
            let p = Arc::new(Policy::default());
            let probe = Arc::new(RetirementProbe::default());
            let text = if structured {
                let mut value: Value =
                    serde_json::from_str(&structured_td(producer.address)).unwrap();
                if invalid_output {
                    // The Producer legitimately completes with 24. The original
                    // Consumer's stricter nested bound must reject even a late
                    // result, using the actual shared evaluator and codec.
                    value["actions"]["run"]["output"]["properties"]["result"]["properties"]["level"]
                        ["maximum"] = json!(23);
                }
                Arc::from(value.to_string())
            } else {
                td(producer.address, None)
            };
            let mut input = definition(&text, b.clone(), p.clone(), false)
                .unwrap()
                .into_input();
            let mut query = input.query.take().unwrap();
            // Retain the actual selected HTTP mapping; only its exchange result
            // timing changes, as permitted by the native retirement contract.
            query.mapping = Box::new(RetirementMapping {
                original: query.mapping,
                probe: probe.clone(),
            });
            input.query = Some(query);
            let d = ConsumerActionDefinition::try_new(input).ok().unwrap();
            let runtime = ConsumerActionRuntime::try_new(owner.reference(), vec![d], 1)
                .ok()
                .unwrap();
            let h = runtime
                .invoke_action(
                    "run",
                    payload(if structured { STRUCTURED_INPUT } else { b"12" }),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            producer.drive(&h, ConsumerActionState::Pending).await;
            producer.counts.complete.set(true);
            p.reject_output
                .store(invalid_output && !structured, Ordering::Relaxed);
            probe.blocked.store(true, Ordering::Relaxed);
            h.query(now(), &mut budget()).unwrap();
            h.progress(&mut cx(), now(), &mut budget()).unwrap();
            assert!(h.take_settlement(now(), &mut budget()).is_err());
            assert!(b.account(ResourceKind::CleanupBytesMax).used() > 0);
            assert_eq!(h.first_cause().unwrap(), Some(query_poll_error()));
            if structured {
                owner.revoke();
            }
            h.cancel_local();
            h.progress(&mut cx(), now(), &mut WorkBudget::new())
                .unwrap();
            assert_eq!(probe.late_replies.load(Ordering::Relaxed), 0);
            probe.blocked.store(false, Ordering::Relaxed);
            producer.drive(&h, ConsumerActionState::Closed).await;
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            assert_eq!(s.first_cause(), Some(&query_poll_error()));
            assert_eq!(s.native_error(), Some(&query_poll_error()));
            assert_eq!(p.validations.load(Ordering::Relaxed), 2);
            assert_eq!(
                s.native_result()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .data()
                    .unwrap()
                    .body
                    .as_ref(),
                if structured { STRUCTURED_OUTPUT } else { b"24" }
            );
            assert!(s.native_result().unwrap().is_ok(), "late output was lost");
            assert_eq!(s.validation_error().is_some(), invalid_output);
            assert!(s.result().is_err());
            assert_eq!(probe.late_replies.load(Ordering::Relaxed), 1);
            assert_eq!(producer.counts.invoked.get(), 1);
            drop(s);
            drop(h);
            drop(runtime);
            drop(owner);
            clean(&b);
            producer.finish();
        }
    });
}

// Hooks are local to this test thread; services remain the actual captured HTTP
// services, and all exchanges below perform real Consumer-to-Producer TCP I/O.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CallbackEdge {
    Access,
    Validate,
    Poll,
    Close,
}
type Callback = (CallbackEdge, Box<dyn FnOnce()>);
thread_local! { static CALLBACK: RefCell<Option<Callback>> = const { RefCell::new(None) }; }
fn on_callback(edge: CallbackEdge, f: impl FnOnce() + 'static) {
    CALLBACK.with(|h| assert!(h.borrow_mut().replace((edge, Box::new(f))).is_none()));
}
fn callback(edge: CallbackEdge) {
    let f = CALLBACK.with(|h| {
        let mut h = h.borrow_mut();
        if h.as_ref().is_some_and(|(e, _)| *e == edge) {
            h.take()
        } else {
            None
        }
    });
    if let Some((_, f)) = f {
        f();
    }
}
struct CallbackMapping(Box<dyn ConsumerActionMapping>);
impl ConsumerActionMapping for CallbackMapping {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.0.identity()
    }
    fn operation(&self) -> Operation {
        self.0.operation()
    }
    fn bounds(&self) -> ConsumerActionBounds {
        let mut b = self.0.bounds();
        b.exchange_bytes += std::mem::size_of::<CallbackExchange>() as u64;
        b.poll_work += 16;
        b
    }
    fn native_definition(&self) -> &dyn Any {
        self.0.native_definition()
    }
    fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()> {
        self.0.supports_continuation(invoke)
    }
    fn destination(&self, continuation: Option<&dyn Any>) -> CoreResult<Box<str>> {
        self.0.destination(continuation)
    }
    fn start(
        &self,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        Ok(Box::new(CallbackExchange(
            self.0.start(input, continuation)?,
        )))
    }
}
struct CallbackExchange(Box<dyn ConsumerActionExchange>);
impl ConsumerActionExchange for CallbackExchange {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>> {
        let result = self.0.poll(cx);
        if result.is_ready() {
            callback(CallbackEdge::Poll);
        }
        result
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<CoreResult<Option<ConsumerActionReply>>> {
        let result = self.0.poll_close(cx);
        if result.is_ready() {
            callback(CallbackEdge::Close);
        }
        result
    }
    fn abort_local(self: Box<Self>) {
        self.0.abort_local();
    }
}

#[test]
fn actual_http_action_callbacks_reenter_original_owner_and_retain_fenced_completion() {
    io().block_on(async {
        for edge in [
            CallbackEdge::Validate,
            CallbackEdge::Poll,
            CallbackEdge::Close,
            CallbackEdge::Access,
        ] {
            let mut producer = Producer::new();
            let b = bank();
            let owner = instance();
            let p = Arc::new(Policy::default());
            let mut input = definition(&td(producer.address, None), b.clone(), p.clone(), false)
                .unwrap()
                .into_input();
            // Decorate only native timing; tokens, destination, socket ownership,
            // serialization and response parsing remain in the HTTP Binding.
            let original = input.query.take().unwrap();
            input.query = Some(ConsumerActionOperation {
                mapping: Box::new(CallbackMapping(original.mapping)),
                ..original
            });
            let d = ConsumerActionDefinition::try_new(input).ok().unwrap();
            let runtime = Rc::new(
                ConsumerActionRuntime::try_new(owner.reference(), vec![d], 1)
                    .ok()
                    .unwrap(),
            );
            let h = Rc::new(
                runtime
                    .invoke_action("run", payload(b"12"), Deadline::NONE, now(), &mut budget())
                    .ok()
                    .unwrap(),
            );
            producer.drive(&h, ConsumerActionState::Pending).await;
            producer.counts.complete.set(true);
            h.query(now(), &mut budget()).unwrap();
            if edge == CallbackEdge::Access {
                producer.drive(&h, ConsumerActionState::Completed).await;
            }
            let nested = h.clone();
            let rt = runtime.clone();
            on_callback(edge, move || {
                assert!(!rt.is_idle());
                assert!(nested.identity().is_ok());
                assert!(nested.query(now(), &mut budget()).is_err());
                assert!(nested.take_settlement(now(), &mut budget()).is_err());
                if edge == CallbackEdge::Access || edge == CallbackEdge::Close {
                    rt.begin_close();
                } else {
                    nested.cancel_local();
                }
                rt.step(&mut cx(), now(), &mut budget());
            });
            if edge != CallbackEdge::Access {
                producer.drive(&h, ConsumerActionState::Closed).await;
            }
            let s = h.take_settlement(now(), &mut budget()).unwrap();
            CALLBACK.with(|h| assert!(h.borrow().is_none()));
            if edge == CallbackEdge::Access || edge == CallbackEdge::Close {
                assert!(matches!(s.result(), Err(CoreError::StaleHandle(_))));
            } else {
                assert!(matches!(s.result(), Err(CoreError::Cancelled(_))));
            }
            assert!(s.accepted());
            assert_eq!(s.effect(), ConsumerEffect::Unknown);
            assert_eq!(
                s.remote_cancellation(),
                ConsumerActionCancellation::NotRequested
            );
            assert_eq!(
                s.native_result()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .data()
                    .unwrap()
                    .body
                    .as_ref(),
                b"24"
            );
            assert_eq!(p.validations.load(Ordering::Relaxed), 2);
            assert_eq!(producer.counts.invoked.get(), 1);
            drop(h);
            drop(runtime);
            drop(owner);
            assert!(b.account(ResourceKind::CleanupBytesMax).used() > 0);
            drop(s);
            clean(&b);
            producer.finish();
        }
    });
}
