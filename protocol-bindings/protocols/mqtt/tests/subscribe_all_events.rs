#![cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;
#[path = "support/services.rs"]
mod services;
mod support;
use broker::*;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{consumer_notification::*, notification::*};
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use support::*;

type Consumer = ConsumerNotificationRuntime<MqttNotificationConsumer>;
const ONE: &[u8] = b"7";
const TWO: &[u8] = b"8";

#[derive(Default)]
struct Access {
    deny: AtomicBool,
    deny_cancel: AtomicBool,
    cancels: AtomicUsize,
}
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, op: Operation) -> CoreResult<()> {
        assert_eq!(op, Operation::SubscribeAllEvents);
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        op: Operation,
    ) -> CoreResult<()> {
        assert!(matches!(
            op,
            Operation::SubscribeAllEvents | Operation::UnsubscribeAllEvents
        ));
        if op == Operation::UnsubscribeAllEvents {
            self.cancels.fetch_add(1, Ordering::Relaxed);
        }
        if self.deny.load(Ordering::Relaxed)
            || op == Operation::UnsubscribeAllEvents && self.deny_cancel.load(Ordering::Relaxed)
        {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Delivery, RetryClass::Never),
            })
        } else {
            Ok(())
        }
    }
    fn retained_bytes(&self) -> u64 {
        64
    }
    fn work(&self) -> u64 {
        1
    }
}
fn consumer_owner() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
}
fn service(b: &Broker, chunk: usize) -> MqttConsumerService {
    MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(b.address, chunk).unwrap(),
        BindingId::new(8),
        BindingGeneration::INITIAL,
    )
    .unwrap()
}
fn definition(
    s: &MqttConsumerService,
    td: serde_json::Value,
    bank: &Arc<HostResourceAccounts>,
    access: Arc<Access>,
    data: Arc<dyn ConsumerDataService>,
) -> ConsumerNotificationDefinition<MqttNotificationConsumer> {
    try_definition(s, td, bank, access, data).unwrap()
}
fn try_definition(
    s: &MqttConsumerService,
    td: serde_json::Value,
    bank: &Arc<HostResourceAccounts>,
    access: Arc<Access>,
    data: Arc<dyn ConsumerDataService>,
) -> CoreResult<ConsumerNotificationDefinition<MqttNotificationConsumer>> {
    let registry = ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
        binding: s.binding(),
        services: ConsumerServices { data, access },
    }])?;
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    registry.prepare_notification(
        &serde_json::from_value(td).unwrap(),
        &AffordanceTarget::Thing,
        NotificationKind::SubscribeAllEvents,
        ConsumerSelector::default(),
        &limits,
        bank.clone(),
        &mut budget(),
    )
}
fn from_definition(
    d: ConsumerNotificationDefinition<MqttNotificationConsumer>,
    owner: &ConsumerInstance,
    items: usize,
) -> Consumer {
    Consumer::try_new(
        owner.reference(),
        d,
        RawNotificationLimits {
            subscriptions: 4,
            queue_items: items,
            queue_payload_bytes: 24_000,
            payload_backing_bytes: 4608,
            control_backing_bytes: 4608,
            target_name_bytes: 64,
            engine_bytes: 128_000,
        },
    )
    .unwrap_or_else(|e| panic!("runtime: {:?}", e.error()))
}
fn consumer(
    s: &MqttConsumerService,
    b: &Broker,
    topic: &str,
    owner: &ConsumerInstance,
    bank: &Arc<HostResourceAccounts>,
    access: Arc<Access>,
    items: usize,
) -> Consumer {
    from_definition(
        definition(
            s,
            event_collection_td(b.address, topic),
            bank,
            access,
            Arc::new(MqttJsonData),
        ),
        owner,
        items,
    )
}
fn subscribe(c: &mut Consumer, policy: NotificationOverflow) -> NotificationCursor {
    c.subscribe(
        SubscriptionInput(None),
        ResponseRequirement::None,
        policy,
        &mut budget(),
    )
    .unwrap_or_else(|e| panic!("subscribe: {:?}", e.error()))
}
async fn active(c: &mut Consumer, id: &NotificationCursor) {
    let end = Instant::now() + Duration::from_secs(3);
    while c.progress(id.identity(), &mut cx(), &mut budget()).unwrap() != NotificationState::Active
    {
        assert!(
            Instant::now() < end,
            "establishment: {:?}",
            c.status(id.identity()).unwrap().first_cause
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn accounted(c: &mut Consumer, id: &NotificationCursor, n: u64) {
    let end = Instant::now() + Duration::from_secs(3);
    while {
        let s = c.status(id.identity()).unwrap();
        s.queued_items as u64 + s.loss.items < n
    } {
        c.progress(id.identity(), &mut cx(), &mut budget()).unwrap();
        assert!(Instant::now() < end, "missing collection samples");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn value(c: &mut Consumer, id: &mut NotificationCursor, expected: &[u8]) {
    accounted(c, id, 1).await;
    let v = c.front(id, &mut budget()).unwrap().unwrap();
    assert_eq!(v.target, &AffordanceTarget::Event(Arc::from("alarm")));
    assert_eq!(&*v.data.unwrap().body, expected);
    assert_eq!(v.response, EventResponseState::Unavailable);
    c.advance(id, &mut budget()).unwrap();
}
async fn settled(c: &mut Consumer, id: &NotificationCursor) -> NotificationSettlement {
    let end = Instant::now() + Duration::from_secs(3);
    while c.progress(id.identity(), &mut cx(), &mut budget()).unwrap() != NotificationState::Closed
    {
        assert!(Instant::now() < end, "cleanup timeout");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let s = c.take_settlement(id.identity()).unwrap();
    assert_eq!(s.target, AffordanceTarget::Thing);
    s
}
async fn fenced(c: &mut Consumer, id: &NotificationCursor) {
    let end = Instant::now() + Duration::from_secs(3);
    while matches!(
        c.status(id.identity()).unwrap().state,
        NotificationState::Establishing | NotificationState::Active
    ) {
        c.progress(id.identity(), &mut cx(), &mut budget()).unwrap();
        assert!(Instant::now() < end, "expected collection refusal");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
fn data(bytes: &[u8]) -> PublicationData {
    PublicationData::EventOccurrence {
        event: Arc::from("alarm"),
        data: Payload::new(bytes, "application/json"),
    }
}
fn emission(
    o: &mut RawProducerServient,
    e: &mut ComposedProducerExposure,
    bytes: &[u8],
) -> ProducerNotificationPublication<MqttPublication> {
    let key = e.key();
    o.publish_notification(
        native(e),
        key,
        AffordanceTarget::Thing,
        data(bytes),
        Deadline::NONE,
        now(),
        &mut budget(),
    )
    .unwrap_or_else(|e| panic!("publish: {:?}", e.error()))
}
async fn send(o: &mut RawProducerServient, e: &mut ComposedProducerExposure, bytes: &[u8]) {
    let mut work = emission(o, e, bytes);
    finish(&mut work).await;
    assert_eq!(
        work.settlement().unwrap().native.effect,
        PublicationEffect::LocallyPublished
    );
    work.acknowledge().unwrap_or_else(|_| panic!());
}
async fn producer(
    b: &Broker,
    topic: &str,
    bank: &Arc<HostResourceAccounts>,
    chunk: usize,
) -> (
    RawProducerServient,
    ComposedProducerExposure,
    MqttNotificationProducer,
    MqttNotificationService,
) {
    let p = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(b.address, chunk).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let s = MqttNotificationService::new(&p).unwrap();
    let mut o = owner();
    let input = input_with_instance(
        &o,
        event_collection_instance(
            authority(1),
            &event_collection_td(b.address, topic).to_string(),
        ),
        &p,
        &s,
        None,
    );
    let mut e = stage(&o, input, bank);
    let key = e.key();
    assert!(
        o.publish_notification(
            native(&mut e),
            key,
            AffordanceTarget::Thing,
            data(ONE),
            Deadline::NONE,
            now(),
            &mut budget()
        )
        .is_err()
    );
    assert!(!s.connected(), "staging grants no publication or socket");
    publish(&mut o, &mut e).await;
    (o, e, p, s)
}

#[test]
fn joined_attributed_events_use_one_native_subscription_and_independent_logical_stop() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "joined/all";
        let (mut po, mut pe, _, ps) = producer(&b, topic, &bank, 8192).await;
        let s = service(&b, 8192);
        let one = consumer_owner();
        let two = consumer_owner();
        let mut a = consumer(&s, &b, topic, &one, &bank, Arc::new(Access::default()), 4);
        let mut z = consumer(&s, &b, topic, &two, &bank, Arc::new(Access::default()), 4);
        let mut ca = subscribe(&mut a, NotificationOverflow::Terminate);
        let mut cz = subscribe(&mut z, NotificationOverflow::Terminate);
        assert_ne!(ca.identity(), cz.identity());
        active(&mut a, &ca).await;
        active(&mut z, &cz).await;
        assert!(a.front(&mut ca, &mut budget()).unwrap().is_none());
        assert!(z.front(&mut cz, &mut budget()).unwrap().is_none());
        let mut peer = Peer::new(&b);
        peer.subscribe(&format!("{topic}/+"));
        send(&mut po, &mut pe, ONE).await;
        peer.event(&format!("{topic}/alarm"), ONE);
        value(&mut a, &mut ca, ONE).await;
        value(&mut z, &mut cz, ONE).await;
        assert_eq!(
            b.logs()
                .lines()
                .filter(|l| l.contains("Received SUBSCRIBE from cr"))
                .count(),
            1,
            "one collection, no N leaf subscriptions"
        );
        a.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut a, &ca).await.remote_cancellation,
            RemoteCancellation::NotAttempted
        );
        assert!(s.connected());
        send(&mut po, &mut pe, TWO).await;
        peer.event(&format!("{topic}/alarm"), TWO);
        value(&mut z, &mut cz, TWO).await;
        z.stop(&cz, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut z, &cz).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        assert!(!s.connected());
        assert_eq!(
            b.logs()
                .lines()
                .filter(|l| l.contains("Received UNSUBSCRIBE from cr"))
                .count(),
            1
        );
        close(&mut po, &mut pe);
        assert!(!ps.connected());
        for prefix in ["Client cr", "Client cz"] {
            let end = Instant::now() + Duration::from_secs(1);
            while !b.logs().lines().any(|l| {
                l.contains(prefix)
                    && (l.contains("closed its connection") || l.contains("disconnected"))
            }) {
                assert!(
                    Instant::now() < end,
                    "broker did not observe {prefix} physical close"
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
        drop((a, z, pe));
        assert_empty(&bank);
    });
}

#[test]
fn malformed_event_data_refuses_before_delivery_and_with_original_backing_retained() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "invalid/all";
        let mut peer = Peer::new(&b);
        for invalid in [
            b"true".as_slice(),
            b"7.5",
            b"null",
            b"{}",
            b"[7]",
            b"7 trailing",
            br#""\uD800""#,
        ] {
            let s = service(&b, 8192);
            let owner = consumer_owner();
            let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 4);
            let mut id = subscribe(&mut c, NotificationOverflow::Terminate);
            active(&mut c, &id).await;
            peer.publish_raw(&format!("{topic}/alarm"), invalid, false);
            fenced(&mut c, &id).await;
            let status = c.status(id.identity()).unwrap();
            assert_eq!(status.queued_items, 0);
            assert!(status.has_retained_handoff);
            assert!(c.front(&mut id, &mut budget()).is_err());
            let used = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
            assert!(c.take_settlement(id.identity()).is_err());
            assert!(matches!(
                c.progress(id.identity(), &mut cx(), &mut WorkBudget::new()),
                Err(CoreError::Backpressure(_))
            ));
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                used
            );
            let report = settled(&mut c, &id).await;
            assert!(matches!(report.first_cause, Some(CoreError::Validation(_))));
            assert_eq!(report.delivered_items, 0);
            assert!(!s.connected());
            drop(c);
            assert_empty(&bank);
        }
    });
}

#[test]
fn independently_captured_collection_schemas_cannot_be_replaced_by_shared_native_data() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "schema/all";
        let (mut po, mut pe, _, _) = producer(&b, topic, &bank, 8192).await;
        let s = service(&b, 8192);
        let one = consumer_owner();
        let two = consumer_owner();
        let mut a = consumer(&s, &b, topic, &one, &bank, Arc::new(Access::default()), 4);
        let mut td = event_collection_td(b.address, topic);
        td["events"]["alarm"]["data"]["type"] = serde_json::json!("string");
        let mut z = from_definition(
            definition(
                &s,
                td,
                &bank,
                Arc::new(Access::default()),
                Arc::new(MqttJsonData),
            ),
            &two,
            4,
        );
        let mut ca = subscribe(&mut a, NotificationOverflow::Terminate);
        let cz = subscribe(&mut z, NotificationOverflow::Terminate);
        active(&mut a, &ca).await;
        active(&mut z, &cz).await;
        send(&mut po, &mut pe, ONE).await;
        value(&mut a, &mut ca, ONE).await;
        fenced(&mut z, &cz).await;
        assert!(matches!(
            settled(&mut z, &cz).await.first_cause,
            Some(CoreError::Validation(_))
        ));
        assert!(s.connected());
        a.stop(&ca, None, &mut budget())
            .unwrap_or_else(|_| panic!());
        settled(&mut a, &ca).await;
        close(&mut po, &mut pe);
        drop((a, z, pe));
        assert_empty(&bank);
    });
}

#[test]
fn root_authority_is_fresh_on_handoff_and_denied_or_invalid_unsubscribe_still_stops_locally() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "policy/all";
        let (mut po, mut pe, _, _) = producer(&b, topic, &bank, 8192).await;
        for invalid in [false, true] {
            let s = service(&b, 8192);
            let owner = consumer_owner();
            let access = Arc::new(Access::default());
            let mut c = consumer(&s, &b, topic, &owner, &bank, access.clone(), 4);
            // No Event response right or application establishment data exists.
            assert!(
                c.subscribe(
                    SubscriptionInput(None),
                    ResponseRequirement::Required,
                    NotificationOverflow::Terminate,
                    &mut budget()
                )
                .is_err()
            );
            assert!(
                c.subscribe(
                    SubscriptionInput(Some(Payload::new(b"1".as_slice(), "application/json"))),
                    ResponseRequirement::None,
                    NotificationOverflow::Terminate,
                    &mut budget()
                )
                .is_err()
            );
            assert!(!s.connected());
            let mut id = subscribe(&mut c, NotificationOverflow::Terminate);
            active(&mut c, &id).await;
            send(&mut po, &mut pe, ONE).await;
            accounted(&mut c, &id, 1).await;
            if invalid {
                let p = Payload::new(b"{}".as_slice(), "application/json");
                let returned = c
                    .stop(&id, Some(CancellationInput(Some(p.clone()))), &mut budget())
                    .err()
                    .unwrap()
                    .into_input()
                    .unwrap();
                assert_eq!(returned.0, Some(p));
            } else {
                c.stop(&id, Some(CancellationInput(None)), &mut budget())
                    .unwrap_or_else(|_| panic!());
                access.deny_cancel.store(true, Ordering::Relaxed);
            }
            assert!(c.front(&mut id, &mut budget()).is_err());
            let report = settled(&mut c, &id).await;
            assert!(matches!(report.first_cause, Some(CoreError::Cancelled(_))));
            assert!(report.cancellation_error.is_some());
            assert_eq!(report.remote_cancellation, RemoteCancellation::NotAttempted);
            assert_eq!(report.delivered_items, 0);
            assert!(!s.connected());
            drop(c);
        }
        let s = service(&b, 8192);
        let owner = consumer_owner();
        let access = Arc::new(Access::default());
        let mut c = consumer(&s, &b, topic, &owner, &bank, access.clone(), 4);
        let mut id = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &id).await;
        send(&mut po, &mut pe, ONE).await;
        accounted(&mut c, &id, 1).await;
        access.deny.store(true, Ordering::Relaxed);
        assert!(c.front(&mut id, &mut budget()).is_err());
        assert_eq!(settled(&mut c, &id).await.delivered_items, 0);
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

#[test]
fn collection_loss_and_saturation_preserve_whole_samples_and_never_coalesce() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "loss/all";
        let (mut po, mut pe, _, _) = producer(&b, topic, &bank, 8192).await;
        for policy in [
            NotificationOverflow::DropNewest,
            NotificationOverflow::Terminate,
        ] {
            let s = service(&b, 8192);
            let owner = consumer_owner();
            let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 1);
            let mut id = subscribe(&mut c, policy);
            active(&mut c, &id).await;
            send(&mut po, &mut pe, ONE).await;
            accounted(&mut c, &id, 1).await;
            assert_eq!(
                &*c.front(&mut id, &mut budget())
                    .unwrap()
                    .unwrap()
                    .data
                    .unwrap()
                    .body,
                ONE
            );
            send(&mut po, &mut pe, TWO).await;
            accounted(&mut c, &id, 2).await;
            assert_eq!(c.status(id.identity()).unwrap().loss.items, 1);
            if policy == NotificationOverflow::DropNewest {
                assert_eq!(
                    &*c.front(&mut id, &mut budget())
                        .unwrap()
                        .unwrap()
                        .data
                        .unwrap()
                        .body,
                    ONE
                );
                c.advance(&mut id, &mut budget()).unwrap();
                assert!(c.front(&mut id, &mut budget()).unwrap().is_none());
                c.stop(&id, None, &mut budget())
                    .unwrap_or_else(|_| panic!());
            }
            let report = settled(&mut c, &id).await;
            assert_eq!(report.loss.items, 1);
            drop(c);
        }
        // Overflow the separate bounded native queue while no Engine cursor
        // progresses. Loss is reported for complete native samples too.
        let mut s = service(&b, 8192);
        let owner = consumer_owner();
        let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 4);
        let id = subscribe(&mut c, NotificationOverflow::DropNewest);
        active(&mut c, &id).await;
        let mut peer = Peer::new(&b);
        for _ in 0..12 {
            peer.publish_raw(&format!("{topic}/alarm"), ONE, false);
        }
        peer.subscribe("loss/barrier");
        for _ in 0..100 {
            s.step(&mut cx());
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        accounted(&mut c, &id, 12).await;
        assert_eq!(c.status(id.identity()).unwrap().queued_items, 4);
        assert_eq!(c.status(id.identity()).unwrap().loss.items, 8);
        c.stop(&id, None, &mut budget())
            .unwrap_or_else(|_| panic!());
        settled(&mut c, &id).await;
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

#[test]
fn replacement_cannot_retarget_an_old_collection_queue_or_inherit_it() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "replacement/all";
        let (mut po, mut pe, _, _) = producer(&b, topic, &bank, 8192).await;
        let s = service(&b, 8192);
        let one = consumer_owner();
        let two = consumer_owner();
        let mut a = consumer(&s, &b, topic, &one, &bank, Arc::new(Access::default()), 4);
        let mut ca = subscribe(&mut a, NotificationOverflow::Terminate);
        active(&mut a, &ca).await;
        send(&mut po, &mut pe, ONE).await;
        accounted(&mut a, &ca, 1).await;
        let original = ca.identity().clone();
        one.revoke();
        let mut z = consumer(&s, &b, topic, &two, &bank, Arc::new(Access::default()), 4);
        let mut cz = subscribe(&mut z, NotificationOverflow::Terminate);
        active(&mut z, &cz).await;
        assert_ne!(&original, cz.identity());
        assert!(z.front(&mut cz, &mut budget()).unwrap().is_none());
        assert!(a.front(&mut ca, &mut budget()).is_err());
        let report = settled(&mut a, &ca).await;
        assert_eq!(report.identity, original);
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        assert_eq!(report.delivered_items, 0);
        assert!(s.connected());
        send(&mut po, &mut pe, TWO).await;
        value(&mut z, &mut cz, TWO).await;
        drop(z);
        assert!(!s.connected());
        close(&mut po, &mut pe);
        drop((a, pe));
        assert_empty(&bank);
    });
}

#[test]
fn partial_unsubscribe_and_broker_failure_preserve_stop_uncertainty_and_funding() {
    runtime().block_on(async {
        let mut b = Broker::start();
        let bank = bank();
        let s = service(&b, 1);
        let owner = consumer_owner();
        let mut c = consumer(
            &s,
            &b,
            "cleanup/all",
            &owner,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let id = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &id).await;
        c.stop(&id, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            c.progress(id.identity(), &mut cx(), &mut budget()).unwrap(),
            NotificationState::Closing
        );
        assert!(c.take_settlement(id.identity()).is_err());
        let used = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        b.stop();
        let report = settled(&mut c, &id).await;
        assert!(matches!(report.first_cause, Some(CoreError::Cancelled(_))));
        assert_eq!(report.remote_cancellation, RemoteCancellation::Unknown);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            used
        );
        assert!(!s.connected());
        drop(c);
        assert_empty(&bank);
    });
}

#[test]
fn producer_replacement_retains_original_root_work_until_retirement_and_acknowledgement() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "producer/all";
        let (mut po, mut pe, p, ps) = producer(&b, topic, &bank, 8192).await;
        let mut pending = emission(&mut po, &mut pe, ONE);
        let old = pe.key();
        let auth = ProducerInstanceAuthority::try_new(ThingSlotId::new(
            old.slot().slot(),
            old.slot().generation().checked_next().unwrap(),
        ))
        .unwrap();
        let input = input_with_instance(
            &po,
            event_collection_instance(auth, &event_collection_td(b.address, topic).to_string()),
            &p,
            &ps,
            Some(old),
        );
        let mut fresh = stage(&po, input, &bank);
        publish(&mut po, &mut fresh).await;
        pe.begin_close();
        assert!(
            native(&mut pe)
                .step(&mut po, &mut cx(), &mut budget())
                .unwrap()
                .is_pending()
        );
        assert!(pe.retire(&mut po, &mut budget()).is_err());
        assert!(ps.connected());
        finish(&mut pending).await;
        let result = pending.settlement().unwrap();
        assert_eq!(result.instance, old);
        assert_eq!(result.target, AffordanceTarget::Thing);
        assert!(matches!(result.cause, Some(CoreError::StaleHandle(_))));
        assert_eq!(result.native.effect, PublicationEffect::NotPublished);
        assert!(
            native(&mut pe)
                .step(&mut po, &mut cx(), &mut budget())
                .unwrap()
                .is_pending(),
            "unread original settlement retains its work lease"
        );
        pending.acknowledge().unwrap_or_else(|_| panic!());
        close(&mut po, &mut pe);
        assert!(ps.connected());
        let mut peer = Peer::new(&b);
        peer.subscribe(&format!("{topic}/+"));
        send(&mut po, &mut fresh, TWO).await;
        peer.event(&format!("{topic}/alarm"), TWO);
        close(&mut po, &mut fresh);
        assert!(!ps.connected());
        drop((pe, fresh));
        assert_empty(&bank);
    });
}

#[test]
fn producer_root_validation_policy_and_saturation_return_original_complete_carriers() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "producer-validation/all";
        let p = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(b.address, 8192).unwrap(),
            BindingId::new(7),
            BindingGeneration::INITIAL,
        );
        let ps = MqttNotificationService::new(&p).unwrap();
        let mut po = owner();
        let text = event_collection_td(b.address, topic).to_string();
        let (input, probe) = services::intercept_with_instance(
            &po,
            event_collection_instance(authority(1), &text),
            &p,
            &ps,
            false,
        );
        let mut pe = stage(&po, input, &bank);
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let _ = pe
                .participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .step(&mut po, &mut cx(), &mut budget())
                .unwrap();
            match pe.publish(&mut po, &mut budget()) {
                Ok(()) => break,
                Err(CoreError::Backpressure(_)) => {}
                Err(e) => panic!("{e:?}"),
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let key = pe.key();
        assert!(
            pe.participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .notification_route(&AffordanceTarget::Thing, NotificationKind::Event)
                .is_none()
        );
        assert!(
            pe.participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .notification_route(
                    &AffordanceTarget::Thing,
                    NotificationKind::SubscribeAllEvents
                )
                .is_some()
        );
        for invalid in [
            data(b"true"),
            data(b"7.5"),
            PublicationData::EventOccurrence {
                event: Arc::from("ready"),
                data: Payload::new(ONE, "application/json"),
            },
            PublicationData::EventOccurrence {
                event: Arc::from("unknown"),
                data: Payload::new(ONE, "application/json"),
            },
            PublicationData::PropertyValue(Payload::new(ONE, "application/json")),
            PublicationData::PropertyValues(Payload::new(ONE, "application/json")),
            PublicationData::EventData(Payload::new(ONE, "application/json")),
            PublicationData::EventOccurrence {
                event: Arc::from("alarm"),
                data: Payload::new(ONE, "text/plain"),
            },
            data(&vec![b' '; MAX_NOTIFICATION_BYTES + 1]),
        ] {
            let payload = invalid.payload().clone();
            let returned = po
                .publish_notification(
                    &mut pe
                        .participant_mut::<services::ServicesParticipant>(0)
                        .unwrap()
                        .native,
                    key,
                    AffordanceTarget::Thing,
                    invalid,
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .err()
                .unwrap()
                .into_input();
            assert_eq!(returned.payload(), &payload);
            assert!(Arc::ptr_eq(&returned.payload().body, &payload.body));
            assert_eq!(ps.locally_published(), 0);
        }
        probe.denied.set(true);
        assert!(
            po.publish_notification(
                &mut pe
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                AffordanceTarget::Thing,
                data(ONE),
                Deadline::NONE,
                now(),
                &mut budget()
            )
            .is_err()
        );
        probe.denied.set(false);
        let mut pending = po
            .publish_notification(
                &mut pe
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                AffordanceTarget::Thing,
                data(ONE),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .unwrap_or_else(|_| panic!());
        finish(&mut pending).await;
        assert_eq!(ps.locally_published(), 1);
        assert!(
            po.publish_notification(
                &mut pe
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                AffordanceTarget::Thing,
                data(TWO),
                Deadline::NONE,
                now(),
                &mut budget()
            )
            .is_err(),
            "unread settlement retains native slot"
        );
        pending.acknowledge().unwrap_or_else(|_| panic!());
        pe.begin_close();
        assert!(
            pe.participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .step(&mut po, &mut cx(), &mut budget())
                .unwrap()
                .is_ready()
        );
        pe.retire(&mut po, &mut budget()).unwrap();
        assert!(!ps.connected());
        drop(pe);
        assert_empty(&bank);
    });
}

#[test]
fn collection_validator_rechecks_original_authority_after_callback_and_requires_prepaid_work() {
    struct RevokingData {
        owner: Arc<ConsumerInstance>,
        calls: Arc<AtomicUsize>,
    }
    impl ConsumerDataService for RevokingData {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn policy(&self) -> ConsumerDataPolicy {
            MqttJsonData.policy()
        }
        fn validate(&self, s: value_type::TypeSchema, p: &Payload) -> CoreResult<()> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let result = MqttJsonData.validate(s, p);
            self.owner.revoke();
            result
        }
    }
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "callback/all";
        let (mut po, mut pe, _, _) = producer(&b, topic, &bank, 8192).await;
        let s = service(&b, 8192);
        let owner = Arc::new(consumer_owner());
        let calls = Arc::new(AtomicUsize::new(0));
        let d = definition(
            &s,
            event_collection_td(b.address, topic),
            &bank,
            Arc::new(Access::default()),
            Arc::new(RevokingData {
                owner: owner.clone(),
                calls: calls.clone(),
            }),
        );
        let mut c = from_definition(d, &owner, 4);
        let id = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &id).await;
        send(&mut po, &mut pe, ONE).await;
        let short = MqttJsonData.policy().validation_work - 1;
        for _ in 0..10 {
            let mut work = budget().with_remaining(WorkClass::CodecInputBytes, short);
            assert!(matches!(
                c.progress(id.identity(), &mut cx(), &mut work),
                Err(CoreError::Backpressure(_))
            ));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(
            calls.load(Ordering::Relaxed),
            0,
            "short work cannot acquire or validate a native sample"
        );
        fenced(&mut c, &id).await;
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(c.status(id.identity()).unwrap().has_retained_handoff);
        let report = settled(&mut c, &id).await;
        assert_eq!(report.delivered_items, 0);
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

#[test]
fn retained_root_samples_are_visible_loss_and_not_initial_values_or_replay() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "retained/all";
        let mut peer = Peer::new(&b);
        peer.publish_raw(&format!("{topic}/alarm"), ONE, true);
        peer.subscribe("retained/barrier");
        let s = service(&b, 8192);
        let owner = consumer_owner();
        let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 4);
        let mut id = subscribe(&mut c, NotificationOverflow::DropNewest);
        active(&mut c, &id).await;
        accounted(&mut c, &id, 1).await;
        assert_eq!(c.status(id.identity()).unwrap().loss.items, 1);
        assert!(c.front(&mut id, &mut budget()).unwrap().is_none());
        peer.publish_raw(&format!("{topic}/alarm"), TWO, false);
        accounted(&mut c, &id, 2).await;
        value(&mut c, &mut id, TWO).await;
        c.stop(&id, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        settled(&mut c, &id).await;
        assert!(!s.connected());
        drop(c);
        assert_empty(&bank);
    });
}

#[test]
fn partial_root_publication_cancellation_is_uncertain_and_releases_physical_service() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "partial/all";
        let (mut po, mut pe, _, ps) = producer(&b, topic, &bank, 1).await;
        let mut work = emission(&mut po, &mut pe, ONE);
        for _ in 0..3 {
            assert!(work.step(&mut cx(), &mut budget(), now()).is_pending());
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(work.settlement().is_none());
        work.cancel();
        finish(&mut work).await;
        let report = work.acknowledge().unwrap_or_else(|_| panic!());
        assert_eq!(report.target, AffordanceTarget::Thing);
        assert!(matches!(report.cause, Some(CoreError::Cancelled(_))));
        assert_eq!(report.native.effect, PublicationEffect::Uncertain);
        assert!(!ps.connected());
        assert!(ps.first_failure().is_some());
        assert_eq!(ps.locally_published(), 0);
        close(&mut po, &mut pe);
        drop(pe);
        assert_empty(&bank);
    });
}

fn occurrence(event: &str, bytes: &[u8]) -> PublicationData {
    PublicationData::EventOccurrence {
        event: Arc::from(event),
        data: Payload::new(bytes, "application/json"),
    }
}
async fn send_named(
    o: &mut RawProducerServient,
    e: &mut ComposedProducerExposure,
    event: &str,
    bytes: &[u8],
) {
    let key = e.key();
    let mut publication = o
        .publish_notification(
            native(e),
            key,
            AffordanceTarget::Thing,
            occurrence(event, bytes),
            Deadline::NONE,
            now(),
            &mut budget(),
        )
        .unwrap_or_else(|r| panic!("{:?}", r.error()));
    finish(&mut publication).await;
    assert_eq!(
        publication
            .acknowledge()
            .unwrap_or_else(|_| panic!())
            .native
            .effect,
        PublicationEffect::LocallyPublished
    );
}
#[test]
fn native_topics_preserve_heterogeneous_occurrences_and_two_cursors_stop_independently() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "identity/events";
        let (mut po, mut pe, _, ps) = producer(&b, topic, &bank, 8192).await;
        let s = service(&b, 8192);
        let owner = consumer_owner();
        let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 4);
        let mut a = subscribe(&mut c, NotificationOverflow::Terminate);
        let mut z = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &a).await;
        active(&mut c, &z).await;
        assert_ne!(a.identity(), z.identity());
        let mut peer = Peer::new(&b);
        peer.subscribe(&format!("{topic}/+"));
        for (event, bytes) in [
            ("ready", b"true".as_slice()),
            ("alarm", b"7"),
            ("ready", b"false"),
        ] {
            send_named(&mut po, &mut pe, event, bytes).await;
            peer.event(&format!("{topic}/{event}"), bytes);
            for cursor in [&mut a, &mut z] {
                accounted(&mut c, cursor, 1).await;
                let item = c.front(cursor, &mut budget()).unwrap().unwrap();
                assert_eq!(item.target, &AffordanceTarget::Event(Arc::from(event)));
                assert_eq!(&*item.data.unwrap().body, bytes);
                assert_eq!(item.response, EventResponseState::Unavailable);
                assert!(
                    c.respond(
                        cursor,
                        EventDataResponse(Some(Payload::new(b"1".as_slice(), "application/json"))),
                        &mut budget()
                    )
                    .is_err()
                );
                c.advance(cursor, &mut budget()).unwrap();
            }
        }
        c.stop(&a, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut c, &a).await.remote_cancellation,
            RemoteCancellation::NotAttempted
        );
        send_named(&mut po, &mut pe, "ready", b"true").await;
        accounted(&mut c, &z, 1).await;
        assert_eq!(
            c.front(&mut z, &mut budget()).unwrap().unwrap().target,
            &AffordanceTarget::Event(Arc::from("ready"))
        );
        c.stop(&z, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut c, &z).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        assert!(!s.connected());
        assert_eq!(
            b.logs()
                .lines()
                .filter(|l| l.contains("Received SUBSCRIBE from cr"))
                .count(),
            1
        );
        assert!(
            b.logs().contains(&format!("{topic}/+")),
            "broker saw native wildcard"
        );
        close(&mut po, &mut pe);
        assert!(!ps.connected());
        drop((c, pe));
        assert_empty(&bank);
    });
}
#[test]
fn unknown_event_and_cross_event_schema_data_are_owned_and_refused_before_logical_admission() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "adversarial/events";
        let mut peer = Peer::new(&b);
        for (event, data) in [
            ("unknown", b"7".as_slice()),
            ("ready", b"7"),
            ("alarm", b"true"),
            ("", b"7"),
        ] {
            let s = service(&b, 8192);
            let owner = consumer_owner();
            let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 4);
            let mut cursor = subscribe(&mut c, NotificationOverflow::Terminate);
            active(&mut c, &cursor).await;
            peer.publish_raw(&format!("{topic}/{event}"), data, false);
            fenced(&mut c, &cursor).await;
            let status = c.status(cursor.identity()).unwrap();
            assert_eq!(status.queued_items, 0);
            assert!(status.has_retained_handoff);
            assert!(c.front(&mut cursor, &mut budget()).is_err());
            let report = settled(&mut c, &cursor).await;
            assert!(matches!(report.first_cause, Some(CoreError::Validation(_))));
            assert_eq!(report.delivered_items, 0);
            assert!(!s.connected());
            drop(c);
            assert_empty(&bank);
        }
    });
}
#[test]
fn cursor_loss_and_stop_before_start_release_collection_without_native_effects() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let owner = consumer_owner();
        let mut c = consumer(
            &s,
            &b,
            "before/events",
            &owner,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let a = subscribe(&mut c, NotificationOverflow::Terminate);
        let z = subscribe(&mut c, NotificationOverflow::Terminate);
        let id = a.identity().clone();
        drop(a);
        c.stop(&z, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        let first = c.progress(&id, &mut cx(), &mut budget()).unwrap();
        assert!(matches!(
            first,
            NotificationState::Closing | NotificationState::Closed
        ));
        for _ in 0..4 {
            if c.progress(&id, &mut cx(), &mut budget()).unwrap() == NotificationState::Closed {
                break;
            }
        }
        let result = c.take_settlement(&id).unwrap();
        assert!(!result.native_started);
        assert!(!settled(&mut c, &z).await.native_started);
        assert!(!s.connected());
        assert!(!b.logs().contains("Received SUBSCRIBE from cr"));
        drop(c);
        assert_empty(&bank);
    });
}
#[test]
fn overlapping_distinct_filters_refuse_without_revoking_original_collection_interest() {
    struct LeafAccess;
    impl ConsumerAccessService for LeafAccess {
        fn authorize(
            &self,
            _: BindingRegistrationIdentity,
            _: &str,
            op: Operation,
        ) -> CoreResult<()> {
            assert_eq!(op, Operation::SubscribeEvent);
            Ok(())
        }
        fn authorize_instance(
            &self,
            _: &ConsumerInstanceRef,
            id: BindingRegistrationIdentity,
            t: &str,
            op: Operation,
        ) -> CoreResult<()> {
            self.authorize(id, t, op)
        }
        fn retained_bytes(&self) -> u64 {
            0
        }
        fn work(&self) -> u64 {
            1
        }
    }
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let one = consumer_owner();
        let two = consumer_owner();
        let prefix = "overlap/events";
        let mut root = consumer(&s, &b, prefix, &one, &bank, Arc::new(Access::default()), 4);
        let mut a = subscribe(&mut root, NotificationOverflow::Terminate);
        active(&mut root, &a).await;
        let registry =
            ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
                binding: s.binding(),
                services: ConsumerServices {
                    data: Arc::new(MqttJsonData),
                    access: Arc::new(LeafAccess),
                },
            }])
            .unwrap();
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
        let d = registry
            .prepare_notification(
                &serde_json::from_value(event_collection_td(b.address, prefix)).unwrap(),
                &AffordanceTarget::Event(Arc::from("alarm")),
                NotificationKind::Event,
                ConsumerSelector::default(),
                &limits,
                bank.clone(),
                &mut budget(),
            )
            .unwrap();
        let mut leaf = from_definition(d, &two, 4);
        assert!(
            leaf.subscribe(
                SubscriptionInput(None),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
                &mut budget()
            )
            .is_err()
        );
        let mut peer = Peer::new(&b);
        peer.publish_raw(&format!("{prefix}/alarm"), ONE, false);
        value(&mut root, &mut a, ONE).await;
        root.stop(&a, None, &mut budget())
            .unwrap_or_else(|_| panic!());
        settled(&mut root, &a).await;
        assert!(!s.connected());
        drop((root, leaf));
        assert_empty(&bank);
    });
}

#[test]
fn final_publication_refuses_missing_actual_collection_services_after_closed_native_setup() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let p = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(b.address, 8192).unwrap(),
            BindingId::new(7),
            BindingGeneration::INITIAL,
        );
        let s = MqttNotificationService::new(&p).unwrap();
        let mut po = owner();
        let (input, _) = services::intercept_with_instance(
            &po,
            event_collection_instance(
                authority(1),
                &event_collection_td(b.address, "missing/events").to_string(),
            ),
            &p,
            &s,
            false,
        );
        let mut pe = stage(&po, input, &bank);
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let ready = pe
                .participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .step(&mut po, &mut cx(), &mut budget())
                .unwrap();
            if ready.is_ready() {
                break;
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        // Services existed at admission. Losing them after real closed setup
        // must still refuse the final publication boundary.
        pe.participant_mut::<services::ServicesParticipant>(0)
            .unwrap()
            .services = None;
        assert!(pe.publish(&mut po, &mut budget()).is_err());
        let key = pe.key();
        assert!(
            po.publish_notification(
                &mut pe
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                AffordanceTarget::Thing,
                occurrence("alarm", ONE),
                Deadline::NONE,
                now(),
                &mut budget()
            )
            .is_err()
        );
        assert_eq!(s.locally_published(), 0);
        pe.begin_close();
        assert!(
            pe.participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .step(&mut po, &mut cx(), &mut budget())
                .unwrap()
                .is_ready()
        );
        pe.retire(&mut po, &mut budget()).unwrap();
        assert!(!s.connected());
        drop(pe);
        assert_empty(&bank);
    });
}

#[test]
fn extra_expanded_event_data_refuses_before_native_start_and_supported_td_still_executes() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let topic = "duplicate/events";
        let (mut po, mut pe, _, ps) = producer(&b, topic, &bank, 8192).await;
        let s = service(&b, 8192);
        let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        for key in [
            "https://www.w3.org/2019/wot/td#data",
            "td:data",
            "extraData",
        ] {
            let mut td = event_collection_td(b.address, topic);
            td["events"]["alarm"]["@context"] = serde_json::json!({
                "td": "https://www.w3.org/2019/wot/td#",
                "extraData": "https://www.w3.org/2019/wot/td#data"
            });
            td["events"]["alarm"][key] = serde_json::json!({"type":"string","minLength":100});
            assert!(
                try_definition(
                    &s,
                    td,
                    &bank,
                    Arc::new(Access::default()),
                    Arc::new(MqttJsonData)
                )
                .is_err(),
                "extra {key} meaning must refuse before a native subscription can be created"
            );
            assert!(!s.connected());
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                baseline
            );
        }
        assert!(!b.logs().contains("Received SUBSCRIBE from cr"));
        let owner = consumer_owner();
        let mut c = consumer(&s, &b, topic, &owner, &bank, Arc::new(Access::default()), 4);
        let mut cursor = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &cursor).await;
        send(&mut po, &mut pe, ONE).await;
        value(&mut c, &mut cursor, ONE).await;
        c.stop(&cursor, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut c, &cursor).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        assert!(!s.connected());
        close(&mut po, &mut pe);
        assert!(!ps.connected());
        drop((c, pe));
        assert_empty(&bank);
    });
}
