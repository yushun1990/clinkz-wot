#![cfg(feature = "prepare")]
#[cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;
mod support;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{consumer_notification::*, notification::*};
use std::sync::atomic::{AtomicBool, Ordering};
use support::*;

type Consumer = ConsumerNotificationRuntime<MqttNotificationConsumer>;
#[derive(Default)]
struct Access(AtomicBool);
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        if self.0.load(Ordering::Relaxed) {
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Delivery, RetryClass::Never),
            })
        } else {
            Ok(())
        }
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
fn schema() -> serde_json::Value {
    serde_json::json!({"type":"object", "required":["reading"], "properties":{
        "reading":{"type":"object", "required":["level","label"], "properties":{
            "level":{"type":"integer", "minimum":1, "exclusiveMaximum":100},
            "label":{"type":"string", "minLength":1, "maxLength":2},
            "enabled":{"type":"boolean"}
        }}
    }})
}
fn target(kind: NotificationKind) -> AffordanceTarget {
    match kind {
        NotificationKind::PropertyObservation => {
            AffordanceTarget::Property(Arc::from("temperature"))
        }
        NotificationKind::Event => AffordanceTarget::Event(Arc::from("alarm")),
        _ => unreachable!(),
    }
}
fn description(address: SocketAddr, topic: &str, kind: NotificationKind) -> serde_json::Value {
    if kind == NotificationKind::Event {
        let mut v = td(address, topic);
        v["events"]["alarm"]["data"] = schema();
        v
    } else {
        let mut v = property_td(address, topic);
        v["properties"]["temperature"]
            .as_object_mut()
            .unwrap()
            .extend(schema().as_object().unwrap().clone());
        v
    }
}
fn instance_for(kind: NotificationKind, text: &str) -> RawProducerInstance {
    if kind == NotificationKind::Event {
        instance(authority(1), text)
    } else {
        property_instance(authority(1), text)
    }
}
fn publication(kind: NotificationKind, body: &[u8]) -> PublicationData {
    let p = Payload::new(body.to_vec(), "application/json");
    if kind == NotificationKind::Event {
        PublicationData::EventData(p)
    } else {
        PublicationData::PropertyValue(p)
    }
}
#[allow(clippy::too_many_arguments)]
fn definition(
    service: &MqttConsumerService,
    value: serde_json::Value,
    kind: NotificationKind,
    bank: &Arc<HostResourceAccounts>,
    access: Arc<Access>,
    data: Arc<dyn ConsumerDataService>,
    limits: &ResourceLimits,
    work: &mut WorkBudget,
) -> CoreResult<ConsumerNotificationDefinition<MqttNotificationConsumer>> {
    let source = serde_json::from_value(value).unwrap();
    let registry = ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
        binding: service.binding(),
        services: ConsumerServices { data, access },
    }])?;
    registry.prepare_notification(
        &source,
        &target(kind),
        kind,
        ConsumerSelector::default(),
        limits,
        bank.clone(),
        work,
    )
} // Source and registry retire before execution.
fn preparation_limits() -> ResourceLimits {
    let mut l = GatewayDefaultV1::LIMITS.clone();
    assert!(l.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    l
}
#[cfg(feature = "broker-tests")]
fn raw_limits() -> RawNotificationLimits {
    RawNotificationLimits {
        subscriptions: 2,
        queue_items: 2,
        queue_payload_bytes: 10_000,
        payload_backing_bytes: 4608,
        control_backing_bytes: 4608,
        target_name_bytes: 64,
        engine_bytes: 100_000,
    }
}

#[test]
fn actual_services_work_schema_backing_and_complete_final_advertisement_are_qualified() {
    struct TypeOnly;
    impl ConsumerDataService for TypeOnly {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
            MqttJsonData.validate(s, p)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            MqttJsonData.policy()
        }
    }
    let address = address();
    let p = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(address, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let s = MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(address, 8192).unwrap(),
        BindingId::new(8),
        BindingGeneration::INITIAL,
    )
    .unwrap();
    let bank = bank();
    for kind in [
        NotificationKind::PropertyObservation,
        NotificationKind::Event,
    ] {
        let v = description(address, "structured/value", kind);
        let access = Arc::new(Access::default());
        assert!(
            definition(
                &s,
                v.clone(),
                kind,
                &bank,
                access.clone(),
                Arc::new(TypeOnly),
                &preparation_limits(),
                &mut budget()
            )
            .is_err()
        );
        assert_empty(&bank);
        let mut short = preparation_limits();
        assert!(short.set(ResourceKind::CleanupWorkItemsPerStepMax, Some(1)));
        assert!(
            definition(
                &s,
                v.clone(),
                kind,
                &bank,
                access.clone(),
                Arc::new(MqttJsonData),
                &short,
                &mut budget()
            )
            .is_err()
        );
        let mut short_work = budget().with_remaining(WorkClass::CleanupItems, 0);
        assert!(
            definition(
                &s,
                v.clone(),
                kind,
                &bank,
                access.clone(),
                Arc::new(MqttJsonData),
                &preparation_limits(),
                &mut short_work
            )
            .is_err()
        );
        assert_empty(&bank);
        let primitive = if kind == NotificationKind::Event {
            td(address, "structured/value")
        } else {
            property_td(address, "structured/value")
        };
        let primitive = definition(
            &s,
            primitive,
            kind,
            &bank,
            access.clone(),
            Arc::new(MqttJsonData),
            &preparation_limits(),
            &mut budget(),
        )
        .unwrap();
        let primitive_bytes = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        drop(primitive);
        let d = definition(
            &s,
            v.clone(),
            kind,
            &bank,
            access,
            Arc::new(MqttJsonData),
            &preparation_limits(),
            &mut budget(),
        )
        .unwrap();
        assert!(d.input().schemas.retained_bytes() > 0);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            primitive_bytes + d.input().schemas.retained_bytes() as u64
        );
        drop(d);
        assert_empty(&bank);
        let original = instance_for(kind, &v.to_string());
        let prepared = prepare(&original, &[&p]).unwrap();
        let check = |v: &serde_json::Value| {
            check_final_thing(
                &v.to_string(),
                original.key(),
                GatewayDefaultV1::LIMITS,
                &mut budget(),
                &[&p],
                &prepared,
                |scope, op| match scope {
                    ValidatedProducerScope::Property(n) => original.supports_property(n, op),
                    ValidatedProducerScope::Event(n) => {
                        original.supports_notification(&AffordanceTarget::Event(Arc::from(n)), op)
                    }
                    _ => false,
                },
            )
        };
        check(&v).unwrap();
        for change in [
            "bound",
            "required",
            "assertion",
            "operation",
            "form",
            "security",
            "response",
            "collection",
        ] {
            let mut changed = v.clone();
            let schema = if kind == NotificationKind::Event {
                &mut changed["events"]["alarm"]["data"]
            } else {
                &mut changed["properties"]["temperature"]
            };
            match change {
                "bound" => {
                    schema["properties"]["reading"]["properties"]["level"]["minimum"] =
                        serde_json::json!(2)
                }
                "required" => {
                    schema["properties"]["reading"]["required"] = serde_json::json!(["level"])
                }
                "assertion" => {
                    schema["properties"]["reading"]["properties"]["label"]["pattern"] =
                        serde_json::json!("x")
                }
                "collection" => {
                    changed["forms"] = serde_json::json!([{"href":format!("mqtt://{address}"), "mqv:filter":"structured/+", "op":["subscribeallevents","unsubscribeallevents"]}])
                }
                _ => {
                    let scope = if kind == NotificationKind::Event {
                        &mut changed["events"]["alarm"]
                    } else {
                        &mut changed["properties"]["temperature"]
                    };
                    match change {
                        "operation" => {
                            scope["forms"][0]["op"] = serde_json::json!([
                                "readproperty",
                                "observeproperty",
                                "unobserveproperty"
                            ])
                        }
                        "form" => scope["forms"]
                            .as_array_mut()
                            .unwrap()
                            .push(serde_json::json!({"href":"https://unsupported.example/value"})),
                        "security" => {
                            changed["securityDefinitions"]["n"] =
                                serde_json::json!({"scheme":"basic"})
                        }
                        "response" => scope["dataResponse"] = schema_for_response(),
                        _ => unreachable!(),
                    }
                }
            }
            assert!(check(&changed).is_err(), "{kind:?} {change}");
        }
    }
    assert!(!s.connected());
    assert_empty(&bank);
}
fn schema_for_response() -> serde_json::Value {
    serde_json::json!({"type":"object", "required":["accepted"], "properties":{"accepted":{"type":"boolean"}}})
}

#[cfg(feature = "broker-tests")]
async fn active(c: &mut Consumer, cursor: &NotificationCursor) {
    use std::time::{Duration, Instant};
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if c.progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap()
            == NotificationState::Active
        {
            return;
        }
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
#[cfg(feature = "broker-tests")]
async fn queued(c: &mut Consumer, cursor: &NotificationCursor) {
    use std::time::{Duration, Instant};
    let end = Instant::now() + Duration::from_secs(3);
    while c.status(cursor.identity()).unwrap().queued_items == 0 {
        c.progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap();
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
#[cfg(feature = "broker-tests")]
async fn settled(c: &mut Consumer, cursor: &NotificationCursor) -> NotificationSettlement {
    use std::time::{Duration, Instant};
    let end = Instant::now() + Duration::from_secs(3);
    while c.status(cursor.identity()).unwrap().state != NotificationState::Closed {
        let _ = c.progress(cursor.identity(), &mut cx(), &mut budget());
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    c.take_settlement(cursor.identity()).unwrap()
}

#[cfg(feature = "broker-tests")]
#[test]
fn structured_engine_producer_to_consumers_preserves_sharing_and_original_retained_owners() {
    use broker::*;
    runtime().block_on(async {
        for kind in [NotificationKind::PropertyObservation, NotificationKind::Event] {
            let broker = Broker::start();
            let bank = bank();
            let mut o = owner();
            let p = MqttNotificationProducer::new(MqttProducerConfig::for_loopback(broker.address, 8192).unwrap(), BindingId::new(7), BindingGeneration::INITIAL);
            let ps = MqttNotificationService::new(&p).unwrap();
            let v = description(broker.address, "structured/value", kind);
            let mut exposure = stage(&o, input_with_instance(&o, instance_for(kind, &v.to_string()), &p, &ps, None), &bank);
            publish(&mut o, &mut exposure).await;
            let service = MqttConsumerService::new(MqttConsumerConfig::for_loopback(broker.address, 8192).unwrap(), BindingId::new(8), BindingGeneration::INITIAL).unwrap();
            let one = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
            let two = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(2), Generation::INITIAL));
            let make = |consumer: &ConsumerInstance| Consumer::try_new(consumer.reference(),
                definition(&service, v.clone(), kind, &bank, Arc::new(Access::default()), Arc::new(MqttJsonData), &preparation_limits(), &mut budget()).unwrap(), raw_limits())
                .unwrap_or_else(|_| panic!("runtime"));
            let mut first = make(&one);
            let mut second = make(&two);
            drop(v);
            let mut a = first.subscribe(SubscriptionInput(None), ResponseRequirement::None, NotificationOverflow::DropNewest, &mut budget()).ok().unwrap();
            let mut b = second.subscribe(SubscriptionInput(None), ResponseRequirement::None, NotificationOverflow::DropNewest, &mut budget()).ok().unwrap();
            active(&mut first, &a).await;
            active(&mut second, &b).await;
            assert_eq!(broker.logs().matches("Received SUBSCRIBE from cr").count(), 1);
            assert!(first.front(&mut a, &mut budget()).unwrap().is_none());
            let key = exposure.key();
            let valid = br#"{"reading":{"level":42.00,"label":"\ud83d\ude00","enabled":true},"unknown":null}"#;
            let before = ps.locally_published();
            for invalid in [br#"{}"#.as_slice(), br#"{"reading":{"level":100,"label":"a"}}"#,
                br#"{"reading":{"level":1.1,"label":"a"}}"#, br#"{"reading":{"level":42,"label":"abc"}}"#,
                br#"{"reading":{"level":42,"le\u0076el":1,"label":"a"}}"#] {
                let original = publication(kind, invalid);
                let pointer = original.payload().body.as_ptr();
                let refused = o.publish_notification(native(&mut exposure), key, target(kind), original, Deadline::NONE, now(), &mut budget()).err().unwrap().into_input();
                assert_eq!(refused.payload().body.as_ptr(), pointer);
            }
            let mut short = budget().with_remaining(WorkClass::CodecInputBytes, MqttJsonData.policy().validation_work - 1);
            assert!(o.publish_notification(native(&mut exposure), key, target(kind), publication(kind, valid), Deadline::NONE, now(), &mut short).is_err());
            assert_eq!(ps.locally_published(), before);
            let mut work = o.publish_notification(native(&mut exposure), key, target(kind), publication(kind, valid), Deadline::NONE, now(), &mut budget()).ok().unwrap();
            finish(&mut work).await;
            assert_eq!(work.settlement().unwrap().native.effect, PublicationEffect::LocallyPublished);
            let mut short = budget().with_remaining(WorkClass::CodecInputBytes, MqttJsonData.policy().validation_work - 1);
            assert!(first.progress(a.identity(), &mut cx(), &mut short).is_err());
            assert_eq!(first.status(a.identity()).unwrap().queued_items, 0);
            queued(&mut first, &a).await;
            queued(&mut second, &b).await;
            let view = first.front(&mut a, &mut budget()).unwrap().unwrap();
            assert_eq!(view.data.unwrap().body, valid);
            assert_eq!(view.response, EventResponseState::Unavailable);
            first.advance(&mut a, &mut budget()).unwrap();
            first.stop(&a, Some(CancellationInput(None)), &mut budget()).ok().unwrap();
            settled(&mut first, &a).await;
            assert!(service.connected());
            assert_eq!(broker.logs().matches("Received UNSUBSCRIBE from cr").count(), 0);
            // Replacement/revocation cannot hand a buffered original value to a successor.
            two.revoke();
            assert!(second.front(&mut b, &mut budget()).is_err());
            let s = settled(&mut second, &b).await;
            assert_eq!(s.identity.consumer, two.reference());
            assert!(!service.connected());
            exposure.begin_close();
            drop((exposure, first, second, o));
            assert!(bank.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0, "unacknowledged original publication still owns its schema/services");
            drop(work.acknowledge().ok().unwrap());
            assert!(!ps.connected());
            assert_empty(&bank);
        }
    });
}

#[cfg(feature = "broker-tests")]
#[test]
fn independent_native_peer_cannot_deliver_invalid_nested_data_or_bypass_fresh_policy() {
    use broker::*;
    struct ChangingPolicy(AtomicBool);
    impl ConsumerDataService for ChangingPolicy {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn supports_schema(&self, _: &data_schema::PreparedSchema) -> bool {
            true
        }
        fn validate(
            &self,
            schema: &data_schema::PreparedSchema,
            payload: &Payload,
        ) -> CoreResult<()> {
            let result = MqttJsonData.validate(schema, payload);
            self.0.store(true, Ordering::Relaxed);
            result
        }
        fn policy(&self) -> ConsumerDataPolicy {
            let mut p = MqttJsonData.policy();
            p.retained_bytes = std::mem::size_of::<Self>() as u64;
            if self.0.load(Ordering::Relaxed) {
                p.validation_work += 1;
            }
            p
        }
    }
    runtime().block_on(async {
        let broker = Broker::start();
        let mut wire = Peer::new(&broker);
        for kind in [
            NotificationKind::PropertyObservation,
            NotificationKind::Event,
        ] {
            for mode in ["invalid", "permission", "service"] {
                let bank = bank();
                let s = MqttConsumerService::new(
                    MqttConsumerConfig::for_loopback(broker.address, 8192).unwrap(),
                    BindingId::new(8),
                    BindingGeneration::INITIAL,
                )
                .unwrap();
                let owner =
                    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL));
                let access = Arc::new(Access::default());
                let data: Arc<dyn ConsumerDataService> = if mode == "service" {
                    Arc::new(ChangingPolicy(AtomicBool::new(false)))
                } else {
                    Arc::new(MqttJsonData)
                };
                let d = definition(
                    &s,
                    description(broker.address, "structured/adversary", kind),
                    kind,
                    &bank,
                    access.clone(),
                    data,
                    &preparation_limits(),
                    &mut budget(),
                )
                .unwrap();
                let mut c = Consumer::try_new(owner.reference(), d, raw_limits())
                    .unwrap_or_else(|_| panic!());
                let mut cursor = c
                    .subscribe(
                        SubscriptionInput(None),
                        ResponseRequirement::None,
                        NotificationOverflow::Terminate,
                        &mut budget(),
                    )
                    .ok()
                    .unwrap();
                active(&mut c, &cursor).await;
                access.0.store(mode == "permission", Ordering::Relaxed);
                let bytes = if mode == "invalid" {
                    br#"{"reading":{"level":0,"label":"ok"}}"#.as_slice()
                } else {
                    br#"{"reading":{"level":42,"label":"ok"}}"#
                };
                wire.publish_raw("structured/adversary", bytes, false);
                let result = settled(&mut c, &cursor).await;
                if mode == "permission" {
                    assert!(matches!(
                        result.first_cause,
                        Some(CoreError::Security { .. })
                    ));
                } else {
                    assert!(matches!(result.first_cause, Some(CoreError::Validation(_))));
                }
                assert!(c.front(&mut cursor, &mut budget()).is_err());
                drop(c);
                assert!(!s.connected());
                assert_empty(&bank);
            }
        }
    });
}
