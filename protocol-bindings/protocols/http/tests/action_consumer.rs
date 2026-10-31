#![cfg(feature = "prepare")]
//! The public TD-prepared Consumer against the qualified HTTP Producer.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{
    consumer::*, consumer_action::*, host_accounts::*, value_type::TypeSchema, *,
};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient::producer_composition::*;
use clinkz_wot_servient_finite::{consumer_action::*, *};
use clinkz_wot_td::thing::Thing;
use serde_json::{Value, json};
use std::{
    net::TcpListener,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Waker},
    time::Duration,
};
#[path = "support/action.rs"]
mod fixture;
use fixture::{Counters, addr, budget, key, now, td};
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
    fn validate(&self, schema: TypeSchema, p: &Payload) -> CoreResult<()> {
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
        let bank = bank();
        let owner = fixture::owner();
        let counts = Rc::new(Counters::default());
        let mut exposure = fixture::create(
            &owner,
            key(Generation::INITIAL),
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
        assert!(h.take_settlement().is_err());
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
        let settlement = h.take_settlement().unwrap();
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
        assert!(h.take_settlement().is_err());
        h.query(now(), &mut budget()).unwrap();
        producer.drive(&h, ConsumerActionState::Completed).await;
        let s = h.take_settlement().unwrap();
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
            let s = h.take_settlement().unwrap();
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
    constrained["actions"]["run"]["input"]["minimum"] = json!(0);
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
            let s = h.take_settlement().unwrap();
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
            let s = h.take_settlement().unwrap();
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
        let s = old.take_settlement().unwrap();
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
        assert!(new.take_settlement().unwrap().result().is_ok());
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
        drop(h.take_settlement().unwrap());
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
            let s = h.take_settlement().unwrap();
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
        assert!(h.take_settlement().unwrap().result().is_ok());
        drop(h);
        drop(runtime);
        drop(owner);
        clean(&b);
    });
    peer.join().unwrap();
}
