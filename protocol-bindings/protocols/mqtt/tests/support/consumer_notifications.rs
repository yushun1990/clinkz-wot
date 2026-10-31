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
#[derive(Default)]
struct Access {
    deny_start: AtomicBool,
    deny_deliver: AtomicBool,
    deny_cancel: AtomicBool,
    calls: AtomicUsize,
}
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.deny_start.load(Ordering::Relaxed)
            || self.deny_deliver.load(Ordering::Relaxed)
            || op == KIND.stop_operation() && self.deny_cancel.load(Ordering::Relaxed)
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
        128
    }
    fn work(&self) -> u64 {
        1
    }
}
fn consumer_owner(slot: u32) -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL))
}
fn service(b: &Broker, chunk: usize) -> MqttConsumerService {
    MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(b.address, chunk).unwrap(),
        BindingId::new(8),
        BindingGeneration::INITIAL,
    )
    .unwrap()
}
fn engine_limits(items: usize) -> RawNotificationLimits {
    RawNotificationLimits {
        subscriptions: 4,
        queue_items: items,
        queue_payload_bytes: 24_000,
        payload_backing_bytes: 4608,
        control_backing_bytes: 4608,
        target_name_bytes: 64,
        engine_bytes: 128_000,
    }
}
fn prepare_consumer(
    s: &MqttConsumerService,
    value: serde_json::Value,
    bank: &Arc<HostResourceAccounts>,
    access: Arc<Access>,
) -> CoreResult<ConsumerNotificationDefinition<MqttNotificationConsumer>> {
    let thing = serde_json::from_value(value).unwrap();
    let registry = ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
        binding: s.binding(),
        services: ConsumerServices {
            data: Arc::new(MqttJsonData),
            access,
        },
    }])?;
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    let definition = registry.prepare_notification(
        &thing,
        &target(),
        ConsumerSelector::default(),
        &limits,
        bank.clone(),
        &mut budget(),
    )?;
    assert_eq!(definition.input().target, target());
    assert_eq!(definition.input().schemas.kind(), KIND);
    Ok(definition)
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
    let definition = prepare_consumer(s, notification_td(b.address, topic), bank, access).unwrap_or_else(|e| {
        if let CoreError::LimitExceeded {
            resource,
            limit,
            requested,
            ..
        } = &e
        {
            panic!("{resource:?}: {requested:?} > {limit}");
        }
        panic!("{e:?}")
    });
    // The source TD, semantic arena and preparation registry are already gone.
    Consumer::try_new(owner.reference(), definition, engine_limits(items))
        .unwrap_or_else(|e| panic!("runtime: {:?}", e.error()))
}
fn subscribe(c: &mut Consumer, overflow: NotificationOverflow) -> NotificationCursor {
    c.subscribe(
        SubscriptionInput(None),
        ResponseRequirement::None,
        overflow,
        &mut budget(),
    )
    .unwrap_or_else(|e| panic!("subscribe: {:?}", e.error()))
}
async fn active(c: &mut Consumer, cursor: &NotificationCursor) {
    let end = Instant::now() + Duration::from_secs(3);
    while c
        .progress(cursor.identity(), &mut cx(), &mut budget())
        .unwrap()
        != NotificationState::Active
    {
        assert!(
            Instant::now() < end,
            "establish: {:?}",
            c.status(cursor.identity()).unwrap().first_cause
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn pump(c: &mut Consumer, cursor: &NotificationCursor, n: usize) {
    for _ in 0..n {
        let _ = c.progress(cursor.identity(), &mut cx(), &mut budget());
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn accounted(c: &mut Consumer, cursor: &NotificationCursor, count: u64) {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let status = c.status(cursor.identity()).unwrap();
        if status.queued_items as u64 + status.loss.items >= count {
            return;
        }
        c.progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap();
        assert!(Instant::now() < end, "notification accounting timeout");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn value(c: &mut Consumer, cursor: &mut NotificationCursor, expected: &[u8]) {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(view) = c.front(cursor, &mut budget()).unwrap() {
            assert_eq!(&*view.data.unwrap().body, expected);
            assert_eq!(view.response, EventResponseState::Unavailable);
            c.advance(cursor, &mut budget()).unwrap();
            return;
        }
        c.progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap();
        assert!(Instant::now() < end, "notification timeout");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn settled(c: &mut Consumer, cursor: &NotificationCursor) -> NotificationSettlement {
    let end = Instant::now() + Duration::from_secs(3);
    while c
        .progress(cursor.identity(), &mut cx(), &mut budget())
        .unwrap()
        != NotificationState::Closed
    {
        assert!(Instant::now() < end, "cleanup timeout");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    c.take_settlement(cursor.identity()).unwrap()
}
async fn fenced(c: &mut Consumer, cursor: &NotificationCursor) {
    let end = Instant::now() + Duration::from_secs(3);
    while matches!(
        c.status(cursor.identity()).unwrap().state,
        NotificationState::Active | NotificationState::Establishing
    ) {
        let _ = c.progress(cursor.identity(), &mut cx(), &mut budget());
        assert!(Instant::now() < end, "fence timeout");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn producer(
    b: &Broker,
    topic: &str,
    bank: &Arc<HostResourceAccounts>,
) -> (
    RawProducerServient,
    ComposedProducerExposure,
    MqttNotificationService,
) {
    let p = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(b.address, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let s = MqttNotificationService::new(&p).unwrap();
    let mut o = owner();
    let mut e = stage(
        &o,
        input_with_instance(&o, notification_instance(authority(1), &notification_td(b.address, topic).to_string()), &p, &s, None),
        bank,
    );
    publish(&mut o, &mut e).await;
    (o, e, s)
}
async fn send(o: &mut RawProducerServient, e: &mut ComposedProducerExposure, bytes: &[u8]) {
    let key = e.key();
    let data = match KIND {
        NotificationKind::PropertyObservation => PublicationData::PropertyValue(Payload::new(bytes.to_vec(), "application/json")),
        NotificationKind::Event => data(bytes),
    };
    let mut emission = o.publish_notification(native(e), key, target(), data, Deadline::NONE, now(), &mut budget()).unwrap_or_else(|e| panic!("{:?}", e.error()));
    finish(&mut emission).await;
    assert_eq!(
        emission.settlement().unwrap().native.effect,
        PublicationEffect::LocallyPublished
    );
    emission
        .acknowledge()
        .unwrap_or_else(|_| panic!("settlement"));
}

#[test]
fn engine_producer_to_engine_consumers_shares_native_subscription_and_releases_last_socket() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let (mut po, mut pe, ps) = producer(&b, "joined/alarm", &bank).await;
        let s = service(&b, 8192);
        let one = consumer_owner(1);
        let two = consumer_owner(2);
        let mut a = consumer(
            &s,
            &b,
            "joined/alarm",
            &one,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut z = consumer(
            &s,
            &b,
            "joined/alarm",
            &two,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut ca = subscribe(&mut a, NotificationOverflow::DropNewest);
        active(&mut a, &ca).await;
        let mut cz = subscribe(&mut z, NotificationOverflow::DropNewest);
        active(&mut z, &cz).await;
        assert!(a.front(&mut ca, &mut budget()).unwrap().is_none());
        assert!(z.front(&mut cz, &mut budget()).unwrap().is_none());
        send(&mut po, &mut pe, b"41").await;
        value(&mut a, &mut ca, b"41").await;
        value(&mut z, &mut cz, b"41").await;
        a.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert!(a.front(&mut ca, &mut budget()).is_err());
        assert_eq!(
            settled(&mut a, &ca).await.remote_cancellation,
            RemoteCancellation::NotAttempted
        );
        assert!(s.connected(), "sibling retains shared connection");
        send(&mut po, &mut pe, b"42").await;
        value(&mut z, &mut cz, b"42").await;
        z.stop(&cz, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut z, &cz).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        assert!(
            !s.connected(),
            "retained registration/definition must not retain idle socket"
        );
        assert_eq!(b.logs().matches("Received SUBSCRIBE from cr").count(), 1);
        assert_eq!(b.logs().matches("Received UNSUBSCRIBE from cr").count(), 1);
        close(&mut po, &mut pe);
        assert!(!ps.connected());
        drop((a, z, pe));
        assert_empty(&bank);
    });
}

#[test]
fn required_responses_invalid_controls_and_unpaid_or_denied_start_refuse_before_native_effects() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let owner = consumer_owner(1);
        let access = Arc::new(Access::default());
        let mut c = consumer(&s, &b, "refusal/alarm", &owner, &bank, access.clone(), 2);
        assert!(
            c.subscribe(
                SubscriptionInput(None),
                ResponseRequirement::Required,
                NotificationOverflow::Terminate,
                &mut budget()
            )
            .is_err()
        );
        let invalid = Payload::new(b"1".to_vec(), "application/json");
        let refused = c
            .subscribe(
                SubscriptionInput(Some(invalid.clone())),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
                &mut budget(),
            )
            .err()
            .unwrap()
            .into_input();
        assert_eq!(refused.0, Some(invalid));
        assert!(
            c.subscribe(
                SubscriptionInput(None),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
                &mut WorkBudget::new()
            )
            .is_err()
        );
        access.deny_start.store(true, Ordering::Relaxed);
        assert!(
            c.subscribe(
                SubscriptionInput(None),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
                &mut budget()
            )
            .is_err()
        );
        assert!(!s.connected());
        access.deny_start.store(false, Ordering::Relaxed);
        let ca = subscribe(&mut c, NotificationOverflow::Terminate);
        c.stop(&ca, None, &mut budget())
            .unwrap_or_else(|_| panic!());
        let result = settled(&mut c, &ca).await;
        assert!(!result.native_started);
        assert_eq!(result.remote_cancellation, RemoteCancellation::NotAttempted);
        assert!(!s.connected());
        drop(c);
        assert_empty(&bank);
    });
}

#[test]
fn replacement_preserves_original_queued_identity_and_sibling_native_participation() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let (mut po, mut pe, _ps) = producer(&b, "replace/alarm", &bank).await;
        let old = consumer_owner(1);
        let new = consumer_owner(1);
        let mut a = consumer(
            &s,
            &b,
            "replace/alarm",
            &old,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let ca = subscribe(&mut a, NotificationOverflow::DropNewest);
        active(&mut a, &ca).await;
        send(&mut po, &mut pe, b"11").await;
        accounted(&mut a, &ca, 1).await;
        assert_eq!(a.status(ca.identity()).unwrap().queued_items, 1);
        let mut z = consumer(
            &s,
            &b,
            "replace/alarm",
            &new,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut cz = subscribe(&mut z, NotificationOverflow::DropNewest);
        active(&mut z, &cz).await;
        drop(old);
        assert_eq!(
            a.status(ca.identity()).unwrap().state,
            NotificationState::Closing
        );
        assert!(
            z.status(ca.identity()).is_err(),
            "same numeric slot cannot retarget original work"
        );
        assert!(
            z.front(&mut cz, &mut budget()).unwrap().is_none(),
            "no replay of sibling's queued Event"
        );
        let result = settled(&mut a, &ca).await;
        assert!(matches!(
            result.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        assert_eq!(result.delivered_items, 0);
        assert!(s.connected());
        send(&mut po, &mut pe, b"12").await;
        value(&mut z, &mut cz, b"12").await;
        drop(cz);
        z.begin_close();
        let end = Instant::now() + Duration::from_secs(3);
        while z.step_cleanup(&mut cx(), &mut budget()).unwrap().is_none() {
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(!s.connected());
        close(&mut po, &mut pe);
        drop((a, z, pe));
        assert_empty(&bank);
    });
}

#[test]
fn fresh_authorization_fences_buffered_delivery_and_denied_cancel_still_releases_locally() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let owner = consumer_owner(1);
        let access = Arc::new(Access::default());
        let mut c = consumer(&s, &b, "policy/alarm", &owner, &bank, access.clone(), 4);
        let mut ca = subscribe(&mut c, NotificationOverflow::DropNewest);
        active(&mut c, &ca).await;
        let sibling = consumer_owner(2);
        let mut z = consumer(
            &s,
            &b,
            "policy/alarm",
            &sibling,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut cz = subscribe(&mut z, NotificationOverflow::DropNewest);
        active(&mut z, &cz).await;
        let mut peer = Peer::new(&b);
        peer.publish_raw("policy/alarm", b"1", false);
        accounted(&mut c, &ca, 1).await;
        assert_eq!(c.status(ca.identity()).unwrap().queued_items, 1);
        access.deny_deliver.store(true, Ordering::Relaxed);
        assert!(c.front(&mut ca, &mut budget()).is_err());
        let result = settled(&mut c, &ca).await;
        assert!(matches!(
            result.first_cause,
            Some(CoreError::Security { .. })
        ));
        assert_eq!(result.delivered_items, 0);
        assert!(s.connected());
        value(&mut z, &mut cz, b"1").await;
        drop((c, z));
        assert!(!s.connected());
        assert_empty(&bank);

        let s = service(&b, 8192);
        access.deny_deliver.store(false, Ordering::Relaxed);
        let mut c = consumer(&s, &b, "policy/alarm", &owner, &bank, access.clone(), 4);
        let ca = subscribe(&mut c, NotificationOverflow::DropNewest);
        active(&mut c, &ca).await;
        access.deny_cancel.store(true, Ordering::Relaxed);
        assert!(
            c.stop(&ca, Some(CancellationInput(None)), &mut budget())
                .is_err()
        );
        let result = settled(&mut c, &ca).await;
        assert!(matches!(
            result.cancellation_error,
            Some(CoreError::Security { .. })
        ));
        assert_eq!(result.remote_cancellation, RemoteCancellation::NotAttempted);
        assert!(!s.connected());
        drop(c);
        assert_empty(&bank);
    });
}

#[test]
fn malformed_json_wrong_schema_and_unpaid_validation_never_escape_to_application() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let owner = consumer_owner(1);
        for bytes in [b"false".as_slice(), b"{bad", b"\"\\uD800\"", b"0.1"] {
            let s = service(&b, 8192);
            let mut c = consumer(
                &s,
                &b,
                "invalid/alarm",
                &owner,
                &bank,
                Arc::new(Access::default()),
                4,
            );
            let mut ca = subscribe(&mut c, NotificationOverflow::DropNewest);
            active(&mut c, &ca).await;
            let mut peer = Peer::new(&b);
            peer.publish_raw("invalid/alarm", bytes, false);
            let mut short = budget();
            short.set_remaining(WorkClass::CodecInputBytes, 65535);
            assert!(c.progress(ca.identity(), &mut cx(), &mut short).is_err());
            assert_eq!(
                c.status(ca.identity()).unwrap().state,
                NotificationState::Active
            );
            fenced(&mut c, &ca).await;
            assert!(c.front(&mut ca, &mut budget()).is_err());
            let result = settled(&mut c, &ca).await;
            assert!(matches!(result.first_cause, Some(CoreError::Validation(_))));
            assert_eq!(result.delivered_items, 0);
            assert!(!s.connected());
            drop(c);
            assert_empty(&bank);
        }
        // An Engine Producer's valid integer is invalid for the captured
        // Consumer boolean schema; native QoS ACKs never validate application data.
        let (mut po, mut pe, _) = producer(&b, "invalid/alarm", &bank).await;
        let s = service(&b, 8192);
        let mut description = notification_td(b.address, "invalid/alarm");
        match KIND {
            NotificationKind::PropertyObservation => description["properties"][NAME]["type"] = serde_json::json!("boolean"),
            NotificationKind::Event => description["events"][NAME]["data"]["type"] = serde_json::json!("boolean"),
        }
        let definition =
            prepare_consumer(&s, description, &bank, Arc::new(Access::default())).unwrap();
        let mut c = Consumer::try_new(owner.reference(), definition, engine_limits(4))
            .unwrap_or_else(|_| panic!());
        let ca = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &ca).await;
        send(&mut po, &mut pe, b"1").await;
        fenced(&mut c, &ca).await;
        let result = settled(&mut c, &ca).await;
        assert!(matches!(result.first_cause, Some(CoreError::Validation(_))));
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

#[test]
fn retained_delivery_is_visible_loss_and_does_not_create_an_initial_or_replayed_data() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let owner = consumer_owner(1);
        let mut peer = Peer::new(&b);
        peer.publish_raw("retain/alarm", b"10", true);
        // A round trip on the publishing connection proves the retained write
        // reached the real broker before this Consumer's subscription.
        peer.subscribe("retain/barrier");
        let mut c = consumer(
            &s,
            &b,
            "retain/alarm",
            &owner,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut ca = subscribe(&mut c, NotificationOverflow::DropNewest);
        active(&mut c, &ca).await;
        pump(&mut c, &ca, 30).await;
        assert!(c.front(&mut ca, &mut budget()).unwrap().is_none());
        assert_eq!(c.status(ca.identity()).unwrap().loss.items, 1);
        let (mut po, mut pe, _) = producer(&b, "retain/alarm", &bank).await;
        send(&mut po, &mut pe, b"11").await;
        value(&mut c, &mut ca, b"11").await;
        c.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(settled(&mut c, &ca).await.loss.items, 1);
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

#[test]
fn logical_and_native_overflow_report_loss_without_coalescing() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let mut s = service(&b, 8192);
        let owner = consumer_owner(1);
        for native in [false, true] {
            let mut c = consumer(
                &s,
                &b,
                "overflow/alarm",
                &owner,
                &bank,
                Arc::new(Access::default()),
                1,
            );
            let mut ca = subscribe(&mut c, NotificationOverflow::DropNewest);
            active(&mut c, &ca).await;
            let (mut po, mut pe, _) = producer(&b, "overflow/alarm", &bank).await;
            for n in 0..7 {
                send(&mut po, &mut pe, n.to_string().as_bytes()).await;
                if !native {
                    accounted(&mut c, &ca, n + 1).await;
                }
            }
            if native {
                for _ in 0..120 {
                    s.step(&mut cx());
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                pump(&mut c, &ca, 10).await;
            }
            assert_eq!(c.status(ca.identity()).unwrap().loss.items, 6);
            value(&mut c, &mut ca, b"0").await;
            c.stop(&ca, None, &mut budget())
                .unwrap_or_else(|_| panic!());
            assert_eq!(settled(&mut c, &ca).await.loss.items, 6);
            close(&mut po, &mut pe);
            drop((c, pe));
            assert_empty(&bank);
            s = service(&b, 8192);
        }
        let mut c = consumer(
            &s,
            &b,
            "overflow/alarm",
            &owner,
            &bank,
            Arc::new(Access::default()),
            1,
        );
        let ca = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &ca).await;
        let mut peer = Peer::new(&b);
        peer.publish_raw("overflow/alarm", b"0", false);
        pump(&mut c, &ca, 20).await;
        peer.publish_raw("overflow/alarm", b"1", false);
        pump(&mut c, &ca, 20).await;
        let result = settled(&mut c, &ca).await;
        assert_eq!(result.loss.items, 1);
        assert!(matches!(
            result.first_cause,
            Some(CoreError::Backpressure(_))
        ));
        drop(c);
        assert_empty(&bank);
    });
}

#[test]
fn broker_failure_is_terminal_shared_failure_and_final_cleanup_is_local() {
    runtime().block_on(async {
        let mut b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let one = consumer_owner(1);
        let two = consumer_owner(2);
        let mut a = consumer(
            &s,
            &b,
            "failure/a",
            &one,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut z = consumer(
            &s,
            &b,
            "failure/z",
            &two,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let ca = subscribe(&mut a, NotificationOverflow::Terminate);
        active(&mut a, &ca).await;
        let cz = subscribe(&mut z, NotificationOverflow::Terminate);
        active(&mut z, &cz).await;
        b.stop();
        pump(&mut a, &ca, 20).await;
        pump(&mut z, &cz, 2).await;
        for (c, cursor) in [(&mut a, &ca), (&mut z, &cz)] {
            let result = settled(c, cursor).await;
            assert!(matches!(result.first_cause, Some(CoreError::Binding(_))));
            assert!(result.cleanup_error.is_none());
            assert_eq!(result.delivered_items, 0);
        }
        assert!(!s.connected());
        assert!(s.first_failure().is_some());
        assert!(
            a.subscribe(
                SubscriptionInput(None),
                ResponseRequirement::None,
                NotificationOverflow::Terminate,
                &mut budget()
            )
            .is_err(),
            "no implicit reconnect"
        );
        drop((a, z));
        assert_empty(&bank);
    });
}

#[test]
fn cancelling_during_shared_connection_setup_does_not_abort_the_sibling_handshake() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 1);
        let one = consumer_owner(1);
        let two = consumer_owner(2);
        let mut a = consumer(
            &s,
            &b,
            "setup/a",
            &one,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut z = consumer(
            &s,
            &b,
            "setup/z",
            &two,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let ca = subscribe(&mut a, NotificationOverflow::Terminate);
        a.progress(ca.identity(), &mut cx(), &mut budget()).unwrap();
        let mut cz = subscribe(&mut z, NotificationOverflow::Terminate);
        a.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut a, &ca).await.remote_cancellation,
            RemoteCancellation::NotAttempted
        );
        active(&mut z, &cz).await;
        assert!(s.first_failure().is_none());
        let (mut po, mut pe, _) = producer(&b, "setup/z", &bank).await;
        send(&mut po, &mut pe, b"99").await;
        value(&mut z, &mut cz, b"99").await;
        drop(cz);
        z.begin_close();
        let end = Instant::now() + Duration::from_secs(3);
        while z.step_cleanup(&mut cx(), &mut budget()).unwrap().is_none() {
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(!s.connected());
        close(&mut po, &mut pe);
        drop((a, z, pe));
        assert_empty(&bank);
    });
}

#[test]
fn cancellation_is_reauthorized_and_invalid_control_returns_its_original_payload() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let owner = consumer_owner(1);
        for invalid_data in [true, false] {
            let s = service(&b, 8192);
            let access = Arc::new(Access::default());
            let mut c = consumer(&s, &b, "cancel/alarm", &owner, &bank, access.clone(), 4);
            let ca = subscribe(&mut c, NotificationOverflow::Terminate);
            active(&mut c, &ca).await;
            if invalid_data {
                let data = Payload::new(b"7".to_vec(), "application/json");
                let returned = c
                    .stop(
                        &ca,
                        Some(CancellationInput(Some(data.clone()))),
                        &mut budget(),
                    )
                    .err()
                    .unwrap()
                    .into_input()
                    .unwrap();
                assert_eq!(returned.0, Some(data));
            } else {
                c.stop(&ca, Some(CancellationInput(None)), &mut budget())
                    .unwrap_or_else(|_| panic!());
                access.deny_cancel.store(true, Ordering::Relaxed);
            }
            let result = settled(&mut c, &ca).await;
            assert!(matches!(result.first_cause, Some(CoreError::Cancelled(_))));
            assert!(result.cancellation_error.is_some());
            assert_eq!(result.remote_cancellation, RemoteCancellation::NotAttempted);
            assert!(!s.connected());
            drop(c);
            assert_empty(&bank);
        }
    });
}

#[test]
fn broker_loss_during_partial_unsubscribe_retains_stop_cause_and_reports_unknown_remote_removal() {
    runtime().block_on(async {
        let mut b = Broker::start();
        let bank = bank();
        let s = service(&b, 1);
        let owner = consumer_owner(1);
        let mut c = consumer(
            &s,
            &b,
            "unsubscribe/alarm",
            &owner,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let ca = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &ca).await;
        c.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            c.progress(ca.identity(), &mut cx(), &mut budget()).unwrap(),
            NotificationState::Closing
        );
        assert!(
            c.take_settlement(ca.identity()).is_err(),
            "cannot acknowledge pending cleanup"
        );
        let funded = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        b.stop();
        let result = settled(&mut c, &ca).await;
        assert!(matches!(result.first_cause, Some(CoreError::Cancelled(_))));
        assert_eq!(result.remote_cancellation, RemoteCancellation::Unknown);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            funded
        );
        assert!(!s.connected());
        drop(c);
        assert_empty(&bank);
    });
}

#[test]
fn root_drop_aborts_local_receive_owners_and_preserves_other_topic_until_final_disposal() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 1);
        let one = consumer_owner(1);
        let two = consumer_owner(2);
        let mut a = consumer(
            &s,
            &b,
            "drop/a",
            &one,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut z = consumer(
            &s,
            &b,
            "drop/z",
            &two,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let ca = subscribe(&mut a, NotificationOverflow::Terminate);
        active(&mut a, &ca).await;
        let mut cz = subscribe(&mut z, NotificationOverflow::Terminate);
        active(&mut z, &cz).await;
        a.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        a.progress(ca.identity(), &mut cx(), &mut budget()).unwrap(); // partial control frame
        drop(a);
        assert!(s.connected());
        let (mut po, mut pe, _) = producer(&b, "drop/z", &bank).await;
        send(&mut po, &mut pe, b"2").await;
        value(&mut z, &mut cz, b"2").await;
        assert!(
            s.first_failure().is_none(),
            "shared owner finishes original frame"
        );
        drop(z);
        assert!(!s.connected());
        close(&mut po, &mut pe);
        drop(pe);
        assert_empty(&bank);
        let end = Instant::now() + Duration::from_secs(1);
        while !b.logs().lines().any(|line| {
            line.contains("Client cr")
                && (line.contains("closed its connection") || line.contains("disconnected"))
        }) {
            assert!(
                Instant::now() < end,
                "broker did not observe physical close"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
}

#[test]
fn a_new_interest_during_partial_unsubscribe_keeps_original_remote_uncertainty() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 1);
        let one = consumer_owner(1);
        let two = consumer_owner(2);
        let mut a = consumer(
            &s,
            &b,
            "handoff/alarm",
            &one,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let ca = subscribe(&mut a, NotificationOverflow::Terminate);
        active(&mut a, &ca).await;
        a.stop(&ca, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        a.progress(ca.identity(), &mut cx(), &mut budget()).unwrap();
        let mut z = consumer(
            &s,
            &b,
            "handoff/alarm",
            &two,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let mut cz = subscribe(&mut z, NotificationOverflow::Terminate);
        z.progress(cz.identity(), &mut cx(), &mut budget()).unwrap();
        let report = settled(&mut a, &ca).await;
        assert_eq!(report.remote_cancellation, RemoteCancellation::Unknown);
        active(&mut z, &cz).await;
        let (mut po, mut pe, _) = producer(&b, "handoff/alarm", &bank).await;
        send(&mut po, &mut pe, b"1").await;
        value(&mut z, &mut cz, b"1").await;
        close(&mut po, &mut pe);
        drop((a, z, pe));
        assert!(!s.connected());
        assert_empty(&bank);
    });
}

#[test]
fn explicit_resubscribe_uses_a_fresh_clean_connection_and_never_replays_old_work() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let owner = consumer_owner(1);
        let mut c = consumer(
            &s,
            &b,
            "fresh/alarm",
            &owner,
            &bank,
            Arc::new(Access::default()),
            4,
        );
        let before_io = subscribe(&mut c, NotificationOverflow::Terminate);
        c.stop(&before_io, None, &mut budget())
            .unwrap_or_else(|_| panic!());
        assert!(!settled(&mut c, &before_io).await.native_started);
        let mut first = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &first).await;
        assert_ne!(before_io.identity(), first.identity());
        let (mut po, mut pe, _) = producer(&b, "fresh/alarm", &bank).await;
        send(&mut po, &mut pe, b"1").await;
        value(&mut c, &mut first, b"1").await;
        c.stop(&first, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        settled(&mut c, &first).await;
        assert!(!s.connected());
        send(&mut po, &mut pe, b"2").await;
        let mut second = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &second).await;
        assert!(c.status(first.identity()).is_err());
        assert!(c.front(&mut second, &mut budget()).unwrap().is_none());
        send(&mut po, &mut pe, b"3").await;
        value(&mut c, &mut second, b"3").await;
        c.stop(&second, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        settled(&mut c, &second).await;
        assert!(!s.connected());
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

#[test]
fn revocation_during_captured_validation_fences_the_owned_native_candidate() {
    struct RevokingData(Arc<ConsumerInstance>);
    impl ConsumerDataService for RevokingData {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn policy(&self) -> ConsumerDataPolicy {
            MqttJsonData.policy()
        }
        fn validate(&self, schema: value_type::TypeSchema, payload: &Payload) -> CoreResult<()> {
            let result = MqttJsonData.validate(schema, payload);
            self.0.revoke();
            result
        }
    }
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let owner = Arc::new(consumer_owner(1));
        let definition = {
            let registry =
                ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
                    binding: s.binding(),
                    services: ConsumerServices {
                        data: Arc::new(RevokingData(owner.clone())),
                        access: Arc::new(Access::default()),
                    },
                }])
                .unwrap();
            let thing = serde_json::from_value(notification_td(b.address, "callback/alarm")).unwrap();
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
            registry
                .prepare_notification(
                    &thing,
                    &target(),
                    ConsumerSelector::default(),
                    &limits,
                    bank.clone(),
                    &mut budget(),
                )
                .unwrap()
        };
        let mut c = Consumer::try_new(owner.reference(), definition, engine_limits(4))
            .unwrap_or_else(|_| panic!());
        let ca = subscribe(&mut c, NotificationOverflow::Terminate);
        active(&mut c, &ca).await;
        let (mut po, mut pe, _) = producer(&b, "callback/alarm", &bank).await;
        send(&mut po, &mut pe, b"1").await;
        fenced(&mut c, &ca).await;
        assert!(c.status(ca.identity()).unwrap().has_retained_handoff);
        assert_eq!(c.status(ca.identity()).unwrap().queued_items, 0);
        let report = settled(&mut c, &ca).await;
        assert!(matches!(
            report.first_cause,
            Some(CoreError::StaleHandle(_))
        ));
        assert_eq!(report.delivered_items, 0);
        assert!(!s.connected());
        close(&mut po, &mut pe);
        drop((c, pe));
        assert_empty(&bank);
    });
}

fn target() -> AffordanceTarget {
    match KIND {
        NotificationKind::PropertyObservation => AffordanceTarget::Property(Arc::from(NAME)),
        NotificationKind::Event => AffordanceTarget::Event(Arc::from(NAME)),
    }
}
fn notification_td(address: SocketAddr, topic: &str) -> serde_json::Value {
    match KIND {
        NotificationKind::PropertyObservation => property_td(address, topic),
        NotificationKind::Event => td(address, topic),
    }
}
fn notification_instance(authority: ProducerInstanceAuthority, text: &str) -> RawProducerInstance {
    RawProducerInstance::new(authority, vec![]).with_description(Arc::from(text))
        .with_notification_sources(vec![match KIND {
            NotificationKind::PropertyObservation => ProducerNotificationSource::property(NAME),
            NotificationKind::Event => ProducerNotificationSource::event(NAME),
        }])
}
