#![cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;
mod support;
const KIND: clinkz_wot_core::notification::NotificationKind =
    clinkz_wot_core::notification::NotificationKind::PropertyObservation;
const NAME: &str = "temperature";
include!("support/consumer_notifications.rs");

#[path = "support/services.rs"]
mod services;
fn property_data(bytes: &[u8]) -> PublicationData {
    PublicationData::PropertyValue(Payload::new(bytes.to_vec(), "application/json"))
}
fn publication(
    o: &mut RawProducerServient,
    e: &mut ComposedProducerExposure,
    bytes: &[u8],
) -> ProducerNotificationPublication<MqttPublication> {
    let key = e.key();
    o.publish_notification(
        native(e),
        key,
        target(),
        property_data(bytes),
        Deadline::NONE,
        now(),
        &mut budget(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}

#[test]
fn producer_property_validation_policy_and_family_refusals_return_original_values() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let p = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(b.address, 8192).unwrap(),
            BindingId::new(7),
            BindingGeneration::INITIAL,
        );
        let s = MqttNotificationService::new(&p).unwrap();
        let mut o = owner();
        let text = property_td(b.address, "validate/temperature").to_string();
        let (input, probe) = services::intercept_with_instance(
            &o,
            property_instance(authority(1), &text),
            &p,
            &s,
            false,
        );
        let mut e = stage(&o, input, &bank);
        let key = e.key();
        // Staging has no publication authority or socket.
        assert!(
            o.publish_notification(
                &mut e
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                target(),
                property_data(b"1"),
                Deadline::NONE,
                now(),
                &mut budget()
            )
            .is_err()
        );
        assert!(!s.connected());
        e.publish(&mut o, &mut budget()).unwrap_err();
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let _ = e
                .participant_mut::<services::ServicesParticipant>(0)
                .unwrap()
                .native
                .step(&mut o, &mut cx(), &mut budget())
                .unwrap();
            if e.publish(&mut o, &mut budget()).is_ok() {
                break;
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let mut peer = Peer::new(&b);
        peer.subscribe("validate/temperature");
        for data in [
            data(b"1"),
            property_data(b"true"),
            property_data(b"{"),
            property_data(b"1.5"),
            property_data(&vec![b'0'; MAX_NOTIFICATION_BYTES + 1]),
            PublicationData::PropertyValue(Payload::new(b"1".to_vec(), "text/plain")),
            PublicationData::PropertyValue(
                Payload::new(b"1".to_vec(), "application/json").with_content_coding("gzip"),
            ),
        ] {
            let original = data.payload().body.clone();
            let refused = o
                .publish_notification(
                    &mut e
                        .participant_mut::<services::ServicesParticipant>(0)
                        .unwrap()
                        .native,
                    key,
                    target(),
                    data,
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .err()
                .unwrap()
                .into_input();
            assert!(Arc::ptr_eq(&original, &refused.payload().body));
        }
        assert_eq!(s.locally_published(), 0);
        probe.denied.set(true);
        let validated = probe.validated.get();
        let refused = o
            .publish_notification(
                &mut e
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                target(),
                property_data(b"2"),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .err()
            .unwrap();
        assert!(matches!(
            refused.error().error(),
            CoreError::Security { .. }
        ));
        assert_eq!(probe.validated.get(), validated);
        assert_eq!(s.locally_published(), 0);
        probe.denied.set(false);
        let mut work = o
            .publish_notification(
                &mut e
                    .participant_mut::<services::ServicesParticipant>(0)
                    .unwrap()
                    .native,
                key,
                target(),
                property_data(b"23"),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        finish(&mut work).await;
        peer.event("validate/temperature", b"23");
        work.acknowledge().ok().unwrap();
        e.begin_close();
        let _ = e
            .participant_mut::<services::ServicesParticipant>(0)
            .unwrap()
            .native
            .step(&mut o, &mut cx(), &mut budget())
            .unwrap();
        e.retire(&mut o, &mut budget()).unwrap();
        drop(e);
        assert!(!s.connected());
        assert_empty(&bank);
    });
}

#[test]
fn property_publication_cancel_deadline_replacement_and_unread_settlement_keep_original_owners() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let p = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(b.address, 8192).unwrap(),
            BindingId::new(7),
            BindingGeneration::INITIAL,
        );
        let s = MqttNotificationService::new(&p).unwrap();
        let mut o = owner();
        let text = property_td(b.address, "replace/temperature").to_string();
        let mut old = stage(
            &o,
            input_with_instance(&o, property_instance(authority(1), &text), &p, &s, None),
            &bank,
        );
        publish(&mut o, &mut old).await;
        let mut cancelled = publication(&mut o, &mut old, b"1");
        cancelled.cancel();
        assert!(
            cancelled
                .step(&mut cx(), &mut WorkBudget::new(), now())
                .is_pending()
        );
        finish(&mut cancelled).await;
        let report = cancelled.acknowledge().ok().unwrap();
        assert!(matches!(report.cause, Some(CoreError::Cancelled(_))));
        assert_eq!(report.native.effect, PublicationEffect::NotPublished);
        let key = old.key();
        let at = now().checked_add_ticks(1).unwrap();
        let mut timed = o
            .publish_notification(
                native(&mut old),
                key,
                target(),
                property_data(b"2"),
                Deadline::at(at),
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        assert!(timed.step(&mut cx(), &mut budget(), at).is_ready());
        let report = timed.acknowledge().ok().unwrap();
        assert!(matches!(report.cause, Some(CoreError::TimedOut(_))));
        assert_eq!(report.native.effect, PublicationEffect::NotPublished);
        let mut pending = publication(&mut o, &mut old, b"3");
        let mut next = stage(
            &o,
            input_with_instance(
                &o,
                property_instance(authority(1), &text),
                &p,
                &s,
                Some(key),
            ),
            &bank,
        );
        publish(&mut o, &mut next).await;
        finish(&mut pending).await;
        let report = pending.acknowledge().ok().unwrap();
        assert_eq!(report.instance, key);
        assert_eq!(report.target, target());
        assert!(matches!(report.cause, Some(CoreError::StaleHandle(_))));
        assert_eq!(report.native.effect, PublicationEffect::NotPublished);
        close(&mut o, &mut old);
        drop(old);
        assert_eq!(s.locally_published(), 0);
        let mut peer = Peer::new(&b);
        peer.subscribe("replace/temperature");
        let mut value = publication(&mut o, &mut next, b"4");
        finish(&mut value).await;
        peer.event("replace/temperature", b"4");
        next.begin_close();
        assert!(
            native(&mut next)
                .step(&mut o, &mut cx(), &mut budget())
                .unwrap()
                .is_pending()
        );
        assert!(next.retire(&mut o, &mut budget()).is_err());
        assert!(s.connected());
        assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
        let report = value.acknowledge().ok().unwrap();
        assert_eq!(report.native.effect, PublicationEffect::LocallyPublished);
        close(&mut o, &mut next);
        drop(next);
        assert!(!s.connected());
        assert_empty(&bank);
    });
}

#[test]
fn partially_written_property_value_is_uncertain_and_fences_native_siblings() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let p = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(b.address, 1).unwrap(),
            BindingId::new(7),
            BindingGeneration::INITIAL,
        );
        let s = MqttNotificationService::new(&p).unwrap();
        let mut o = owner();
        let mut a = stage(
            &o,
            input_with_instance(
                &o,
                property_instance(
                    authority(1),
                    &property_td(b.address, "partial/a").to_string(),
                ),
                &p,
                &s,
                None,
            ),
            &bank,
        );
        let mut z = stage(
            &o,
            input_with_instance(
                &o,
                property_instance(
                    authority(2),
                    &property_td(b.address, "partial/z").to_string(),
                ),
                &p,
                &s,
                None,
            ),
            &bank,
        );
        publish(&mut o, &mut a).await;
        publish(&mut o, &mut z).await;
        let mut work = publication(&mut o, &mut a, b"1");
        assert!(work.step(&mut cx(), &mut budget(), now()).is_pending());
        work.cancel();
        finish(&mut work).await;
        let report = work.acknowledge().ok().unwrap();
        assert_eq!(report.native.effect, PublicationEffect::Uncertain);
        assert!(matches!(report.cause, Some(CoreError::Cancelled(_))));
        assert_eq!(report.target, target());
        let _ = native(&mut z)
            .step(&mut o, &mut cx(), &mut budget())
            .unwrap();
        assert!(o.description(z.key()).is_none());
        assert!(s.first_failure().is_some());
        assert!(!s.connected());
        close(&mut o, &mut a);
        close(&mut o, &mut z);
        drop((a, z));
        assert_empty(&bank);
    });
}

#[test]
fn property_and_event_with_same_name_keep_schemas_targets_and_stop_independent() {
    runtime().block_on(async {
        let b = Broker::start();
        let bank = bank();
        let s = service(&b, 8192);
        let p = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(b.address, 8192).unwrap(),
            BindingId::new(7),
            BindingGeneration::INITIAL,
        );
        let ps = MqttNotificationService::new(&p).unwrap();
        let mut o = owner();
        let mut td = property_td(b.address, "families/property");
        let mut event = support::td(b.address, "families/event");
        event["events"]["alarm"]["data"]["type"] = serde_json::json!("string");
        td["events"] = serde_json::json!({ "temperature": event["events"]["alarm"].clone() });
        let instance = RawProducerInstance::new(authority(1), vec![])
            .with_description(Arc::from(td.to_string()))
            .with_notification_sources(vec![
                ProducerNotificationSource::property(NAME),
                ProducerNotificationSource::event(NAME),
            ]);
        let mut e = stage(&o, input_with_instance(&o, instance, &p, &ps, None), &bank);
        publish(&mut o, &mut e).await;
        let one = consumer_owner(1);
        let two = consumer_owner(2);
        let definition =
            prepare_consumer(&s, td.clone(), &bank, Arc::new(Access::default())).unwrap();
        let mut property = Consumer::try_new(one.reference(), definition, engine_limits(4))
            .unwrap_or_else(|_| panic!());
        let event_target = AffordanceTarget::Event(Arc::from(NAME));
        let event_definition = {
            let thing = serde_json::from_value(td).unwrap();
            let registry =
                ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
                    binding: s.binding(),
                    services: ConsumerServices {
                        data: Arc::new(MqttJsonData),
                        access: Arc::new(Access::default()),
                    },
                }])
                .unwrap();
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
            registry
                .prepare_notification(
                    &thing,
                    &event_target,
                    ConsumerSelector::default(),
                    &limits,
                    bank.clone(),
                    &mut budget(),
                )
                .unwrap()
        };
        assert_eq!(
            event_definition.input().schemas.kind(),
            NotificationKind::Event
        );
        let mut event = Consumer::try_new(two.reference(), event_definition, engine_limits(4))
            .unwrap_or_else(|_| panic!());
        let mut cp = subscribe(&mut property, NotificationOverflow::Terminate);
        active(&mut property, &cp).await;
        let mut ce = subscribe(&mut event, NotificationOverflow::Terminate);
        active(&mut event, &ce).await;
        assert!(property.front(&mut cp, &mut budget()).unwrap().is_none());
        assert!(event.front(&mut ce, &mut budget()).unwrap().is_none());
        send(&mut o, &mut e, b"7").await;
        value(&mut property, &mut cp, b"7").await;
        assert!(event.front(&mut ce, &mut budget()).unwrap().is_none());
        let key = e.key();
        assert!(
            o.publish_notification(
                native(&mut e),
                key,
                target(),
                data(b"7"),
                Deadline::NONE,
                now(),
                &mut budget()
            )
            .is_err()
        );
        let mut work = o
            .publish_notification(
                native(&mut e),
                key,
                event_target,
                data(b"\"alarm\""),
                Deadline::NONE,
                now(),
                &mut budget(),
            )
            .ok()
            .unwrap();
        finish(&mut work).await;
        work.acknowledge().ok().unwrap();
        value(&mut event, &mut ce, b"\"alarm\"").await;
        event
            .stop(&ce, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut event, &ce).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        assert!(s.connected());
        send(&mut o, &mut e, b"8").await;
        value(&mut property, &mut cp, b"8").await;
        property
            .stop(&cp, Some(CancellationInput(None)), &mut budget())
            .unwrap_or_else(|_| panic!());
        assert_eq!(
            settled(&mut property, &cp).await.remote_cancellation,
            RemoteCancellation::Confirmed
        );
        assert!(!s.connected());
        close(&mut o, &mut e);
        drop((e, property, event));
        assert_empty(&bank);
    });
}
