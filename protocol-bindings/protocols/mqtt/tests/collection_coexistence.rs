#![cfg(feature = "prepare")]
mod support;
use clinkz_wot_binding_http::*;
use clinkz_wot_core::action::*;
use serde_json::{Value, json};
use support::*;

struct Behavior;
impl RawActionHandler for Behavior {
    fn authorize(&self, _: Operation, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn invoke(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<RawActionStart> {
        Ok(RawActionStart::Complete(InteractionOutput::with_data(
            Payload::new(b"23".as_slice(), "application/json"),
        )))
    }
}
struct Query;
impl ActionQueryPolicy for Query {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
}
fn description(mqtt: SocketAddr, http: SocketAddr) -> Value {
    let mut td = event_collection_td(mqtt, "coexist/events");
    // The same name intentionally denotes different data in each family.
    td["events"]["alarm"]["data"]["type"] = json!("boolean");
    td["actions"] = json!({"alarm":{"synchronous":false,
    "input":{"type":"integer"},"output":{"type":"integer"},
    "uriVariables":{"id":{"type":"string"}},"forms":[
        {"href":format!("http://{http}/alarm"),"op":"invokeaction","htv:methodName":"POST"},
        {"href":format!("http://{http}/status{{?id}}"),"op":"queryaction","htv:methodName":"GET"},
        {"href":format!("http://{http}/stop{{?id}}"),"op":"cancelaction","htv:methodName":"DELETE"}
    ]}});
    td["forms"].as_array_mut().unwrap().push(json!({
        "href":format!("http://{http}/actions"),"op":"queryallactions","htv:methodName":"GET",
        "response":{"contentType":"application/json","htv:statusCodeValue":200}
    }));
    td
}
fn logical(td: &Value, root_action: bool, root_events: bool) -> RawProducerInstance {
    let mut sources = vec![
        ProducerNotificationSource::event("alarm"),
        ProducerNotificationSource::event("ready"),
    ];
    if root_events {
        sources.push(ProducerNotificationSource::AllEvents);
    }
    let instance = RawProducerInstance::new(authority(1), vec![])
        .with_description(Arc::from(td.to_string()))
        .with_notification_sources(sources)
        .with_actions(vec![RawActionBehavior::new(
            Arc::from("alarm"),
            Behavior,
            0,
            true,
            true,
        )]);
    if root_action {
        instance.with_action_query(Query)
    } else {
        instance
    }
}
fn supports(
    instance: &RawProducerInstance,
    scope: ValidatedProducerScope<'_>,
    op: Operation,
) -> bool {
    match scope {
        ValidatedProducerScope::Thing => instance.supports(&AffordanceTarget::Thing, op),
        ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
        ValidatedProducerScope::Event(n) => instance.supports_event(n, op),
        ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
    }
}
fn prepare_both(
    instance: &RawProducerInstance,
    providers: &[&dyn ProducerCapability],
) -> CoreResult<PreparedThing> {
    prepare_thing(
        instance.description().unwrap(),
        instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        providers,
        |s, op| supports(instance, s, op),
    )
}

#[test]
fn mixed_collection_preparation_and_final_qualification_require_both_families() {
    let mqtt_addr = address();
    let http_addr = address();
    let td = description(mqtt_addr, http_addr);
    let mqtt = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(mqtt_addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let http = HttpActionProducer::new(
        HttpProducerConfig::for_loopback(http_addr).unwrap(),
        BindingId::new(8),
        BindingGeneration::INITIAL,
    );
    let providers: &[&dyn ProducerCapability] = &[&mqtt, &http];
    let instance = logical(&td, true, true);
    let prepared = prepare_both(&instance, providers).unwrap();
    assert!(prepare_both(&instance, &[&mqtt]).is_err());
    assert!(prepare_both(&instance, &[&http]).is_err());
    assert!(prepare_both(&logical(&td, false, true), providers).is_err());
    assert!(prepare_both(&logical(&td, true, false), providers).is_err());
    for op in [
        Operation::QueryAllActions,
        Operation::SubscribeAllEvents,
        Operation::UnsubscribeAllEvents,
    ] {
        assert!(
            prepared
                .operations()
                .iter()
                .any(|o| o.site == FormSite::Thing && o.operation == op)
        );
    }
    let check = |value: &Value| {
        check_final_thing(
            &value.to_string(),
            instance.key(),
            GatewayDefaultV1::LIMITS,
            &mut budget(),
            providers,
            &prepared,
            |s, op| supports(&instance, s, op),
        )
    };
    check(&td).unwrap();
    for family in ["actions", "events"] {
        let role = if family == "actions" {
            "output"
        } else {
            "data"
        };
        let mut changed = td.clone();
        changed[family]["alarm"][role]["type"] = json!("string");
        assert!(check(&changed).is_err(), "changed {family} schema");
        let mut duplicate = td.clone();
        duplicate[family]["alarm"][format!("https://www.w3.org/2019/wot/td#{role}")] =
            json!({"type":"string"});
        assert!(
            prepare_both(&logical(&duplicate, true, true), providers).is_err(),
            "duplicate {family} role"
        );
        assert!(check(&duplicate).is_err(), "final duplicate {family} role");
    }
    for index in 0..2 {
        let mut missing = td.clone();
        missing["forms"].as_array_mut().unwrap().remove(index);
        assert!(check(&missing).is_err(), "missing root {index}");
    }
    TcpListener::bind(http_addr).unwrap();
    TcpListener::bind(mqtt_addr).unwrap();
}

#[cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;

#[cfg(feature = "broker-tests")]
mod live {
    use super::*;
    use broker::*;
    use clinkz_wot_core::{consumer::*, notification::*};
    use clinkz_wot_planning::open_consumer::*;
    use clinkz_wot_servient_finite::{consumer::*, consumer_notification::*, notification::*};
    use std::{
        io::{Read, Write},
        net::TcpStream,
        task::Poll,
        time::{Duration, Instant},
    };

    struct Access;
    impl ConsumerAccessService for Access {
        fn authorize_instance(
            &self,
            instance: &ConsumerInstanceRef,
            binding: BindingRegistrationIdentity,
            target: &str,
            operation: Operation,
        ) -> CoreResult<()> {
            assert!(instance.is_current());
            self.authorize(binding, target, operation)
        }
        fn authorize(
            &self,
            _: BindingRegistrationIdentity,
            _: &str,
            _: Operation,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn retained_bytes(&self) -> u64 {
            0
        }
        fn work(&self) -> u64 {
            1
        }
    }
    fn clean(bank: &HostResourceAccounts) {
        for kind in ResourceKind::ALL {
            assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
        }
    }
    async fn invoke(
        owner: &mut RawProducerServient,
        exposure: &mut ComposedProducerExposure,
        addr: SocketAddr,
    ) -> String {
        let mut peer = TcpStream::connect(addr).unwrap();
        peer.write_all(format!("POST /alarm HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 1\r\nConnection: close\r\n\r\n7").as_bytes()).unwrap();
        peer.set_nonblocking(true).unwrap();
        let end = Instant::now() + Duration::from_secs(3);
        let mut bytes = Vec::new();
        let mut delivered = false;
        loop {
            let http = exposure.participant_mut::<HttpActionRuntime>(1).unwrap();
            let _ = http.step(owner, &mut budget()).unwrap();
            if let Some(d) = http.take_delivery() {
                assert!(d.locally_written);
                delivered = true;
            }
            let mut buf = [0; 1024];
            match peer.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => bytes.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("{e}"),
            }
            assert!(Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(delivered);
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("HTTP/1.1 202"), "{text}");
        serde_json::from_str::<Value>(text.split_once("\r\n\r\n").unwrap().1).unwrap()["id"]
            .as_str()
            .unwrap()
            .into()
    }

    #[test]
    fn one_qualified_thing_keeps_http_actions_and_mqtt_event_lifecycles_and_funding_independent() {
        runtime().block_on(async {
            let broker = Broker::start();
            let http_addr = address();
            let td = description(broker.address, http_addr);
            let producer_bank = bank();
            let consumer_bank = bank();
            let mqtt = MqttNotificationProducer::new(
                MqttProducerConfig::for_loopback(broker.address, 8192).unwrap(),
                BindingId::new(7),
                BindingGeneration::INITIAL,
            );
            let service = MqttNotificationService::new(&mqtt).unwrap();
            let http = HttpActionProducer::new(
                HttpProducerConfig::for_loopback(http_addr).unwrap(),
                BindingId::new(8),
                BindingGeneration::INITIAL,
            );
            let mut owner = owner();
            owner
                .enable_raw_actions(RawActionLimits {
                    invocations: 8,
                    application_work_bytes: 64,
                    input_fields: 8,
                    input_bytes: 4160,
                    principal_id_bytes: 32,
                    output_body_bytes: 4096,
                    output_media_capacity: 64,
                    output_retained_bytes: 8192,
                })
                .unwrap();
            let instance = logical(&td, true, true);
            let prepared = prepare_both(&instance, &[&mqtt, &http]).unwrap();
            let mqtt_definitions = prepared
                .operations()
                .iter()
                .filter_map(|o| o.definition::<MqttNotificationDefinition>().cloned())
                .collect();
            let http_definitions = prepared
                .operations()
                .iter()
                .filter_map(|o| o.definition::<HttpActionDefinition>().cloned())
                .collect();
            let mqtt_runtime = service
                .participation(&owner, instance.key(), mqtt_definitions)
                .unwrap();
            let http_runtime =
                HttpActionRuntime::try_new(&owner, instance.key(), http_definitions, 8).unwrap();
            let mut exposure = stage(
                &owner,
                ComposedProducerInput {
                    instance,
                    prepared,
                    providers: vec![Rc::new(mqtt), Rc::new(http)],
                    participants: vec![Box::new(mqtt_runtime), Box::new(http_runtime)],
                    expected: None,
                    handler_backing_bytes: 0,
                },
                &producer_bank,
            );
            // Both actual participants must be ready before the one logical
            // publication; staging itself acquires no listener or connection.
            assert!(!service.connected());
            exposure
                .participant_mut::<HttpActionRuntime>(1)
                .unwrap()
                .install_closed()
                .unwrap();
            publish(&mut owner, &mut exposure).await;

            let consumer_owner =
                ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(4), Generation::INITIAL));
            let mqtt_consumer = MqttConsumerService::new(
                MqttConsumerConfig::for_loopback(broker.address, 8192).unwrap(),
                BindingId::new(10),
                BindingGeneration::INITIAL,
            )
            .unwrap();
            let registry =
                ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
                    binding: mqtt_consumer.binding(),
                    services: ConsumerServices {
                        data: Arc::new(MqttJsonData),
                        access: Arc::new(Access),
                    },
                }])
                .unwrap();
            let thing = serde_json::from_value(td).unwrap();
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
            let event_definition = registry
                .prepare_notification(
                    &thing,
                    &AffordanceTarget::Thing,
                    NotificationKind::SubscribeAllEvents,
                    ConsumerSelector::default(),
                    &limits,
                    consumer_bank.clone(),
                    &mut budget(),
                )
                .unwrap();
            let ConsumerNotificationSchemas::SubscribeAllEvents(schema) =
                &event_definition.input().schemas
            else {
                panic!()
            };
            assert_eq!(
                schema.member("alarm").unwrap().data,
                Some(value_type::TypeSchema::Type(value_type::ValueType::Boolean))
            );
            let finite_registry = ConsumerRegistry::try_new(
                vec![
                    try_consumer_registration(
                        HttpConsumerConfig::for_cleartext_ip().unwrap(),
                        BindingId::new(11),
                        BindingGeneration::INITIAL,
                        0,
                    )
                    .unwrap(),
                ],
                &consumer_bank,
            )
            .unwrap_or_else(|_| panic!());
            let prepared = finite_registry
                .prepare(
                    &thing,
                    GatewayDefaultV1::LIMITS,
                    consumer_bank.clone(),
                    ConsumerSelector::default(),
                    &mut budget(),
                    |job| {
                        while !job.is_finished() {
                            job.step(&mut budget(), false);
                        }
                    },
                )
                .unwrap();
            assert!(
                prepared
                    .support()
                    .iter()
                    .any(|s| s.operation == Operation::QueryAllActions && s.selected)
            );
            assert!(
                !prepared
                    .support()
                    .iter()
                    .any(|s| s.operation == Operation::SubscribeAllEvents && s.selected)
            );
            let (definitions, funding) = prepared.into_execution();
            let finite = ConsumerRuntime::try_new(
                consumer_owner.reference(),
                definitions,
                funding,
                consumer_bank.clone(),
                1,
            )
            .unwrap_or_else(|_| panic!());
            let mut events = ConsumerNotificationRuntime::try_new(
                consumer_owner.reference(),
                event_definition,
                RawNotificationLimits {
                    subscriptions: 2,
                    queue_items: 4,
                    queue_payload_bytes: 24_000,
                    payload_backing_bytes: 4608,
                    control_backing_bytes: 4608,
                    target_name_bytes: 64,
                    engine_bytes: 128_000,
                },
            )
            .unwrap_or_else(|_| panic!());
            drop(thing);
            drop(registry);
            drop(finite_registry);
            let mut cursor = events
                .subscribe(
                    SubscriptionInput(None),
                    ResponseRequirement::None,
                    NotificationOverflow::Terminate,
                    &mut budget(),
                )
                .unwrap_or_else(|_| panic!());
            let end = Instant::now() + Duration::from_secs(5);
            while events
                .progress(cursor.identity(), &mut cx(), &mut budget())
                .unwrap()
                != NotificationState::Active
            {
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let token = invoke(&mut owner, &mut exposure, http_addr).await;
            let key = exposure.key();
            // An integer valid for the same-named Action is invalid Event data.
            let before = ResourceKind::ALL.map(|k| producer_bank.account(k).used());
            assert!(
                owner
                    .publish_notification(
                        native(&mut exposure),
                        key,
                        AffordanceTarget::Thing,
                        PublicationData::EventOccurrence {
                            event: Arc::from("alarm"),
                            data: Payload::new(b"23".as_slice(), "application/json")
                        },
                        Deadline::NONE,
                        now(),
                        &mut budget()
                    )
                    .is_err()
            );
            assert_eq!(service.locally_published(), 0);
            assert_eq!(
                before,
                ResourceKind::ALL.map(|k| producer_bank.account(k).used())
            );
            let mut emission = owner
                .publish_notification(
                    native(&mut exposure),
                    key,
                    AffordanceTarget::Thing,
                    PublicationData::EventOccurrence {
                        event: Arc::from("alarm"),
                        data: Payload::new(b"true".as_slice(), "application/json"),
                    },
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .unwrap_or_else(|_| panic!());
            finish(&mut emission).await;
            emission.acknowledge().unwrap_or_else(|_| panic!());
            while events.status(cursor.identity()).unwrap().queued_items == 0 {
                events
                    .progress(cursor.identity(), &mut cx(), &mut budget())
                    .unwrap();
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let event = events.front(&mut cursor, &mut budget()).unwrap().unwrap();
            assert_eq!(
                event.target,
                clinkz_wot_core::thing::AffordanceSite::Event("alarm")
            );
            assert_eq!(event.data.unwrap().body, b"true");
            events.advance(&mut cursor, &mut budget()).unwrap();
            events
                .stop(&cursor, Some(CancellationInput(None)), &mut budget())
                .unwrap_or_else(|_| panic!());
            while events
                .progress(cursor.identity(), &mut cx(), &mut budget())
                .unwrap()
                != NotificationState::Closed
            {
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let stopped = events.take_settlement(cursor.identity()).unwrap();
            assert_eq!(stopped.remote_cancellation, RemoteCancellation::Confirmed);
            drop(stopped);
            assert!(!mqtt_consumer.connected());
            assert!(service.connected());
            // Stopping the Event interaction did not revoke this Consumer or
            // the Producer's independently retained Action inventory.
            let mut query = finite
                .query_all_actions(Deadline::NONE, now(), &mut budget())
                .unwrap();
            let result = loop {
                if let Poll::Ready(result) = query.poll(&mut cx(), now(), &mut budget()) {
                    break result;
                }
                let http = exposure.participant_mut::<HttpActionRuntime>(1).unwrap();
                let _ = http.step(&mut owner, &mut budget()).unwrap();
                if let Some(d) = http.take_delivery() {
                    assert!(d.locally_written);
                }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            let data = result.result().unwrap().data().unwrap();
            let statuses: Value = serde_json::from_slice(data.body).unwrap();
            assert_eq!(statuses["alarm"][0]["status"], "completed");
            assert_eq!(statuses["alarm"][0]["output"], 23);
            assert!(
                statuses["alarm"][0]["href"]
                    .as_str()
                    .unwrap()
                    .ends_with(&token)
            );
            assert_eq!(result.consumer(), &consumer_owner.reference());
            let http = exposure.participant_mut::<HttpActionRuntime>(1).unwrap();
            let id = http.action_ids().next().unwrap();
            let retained = http.acknowledge(&mut owner, id).unwrap().unwrap();
            assert_eq!(retained.result().unwrap().data().unwrap().body, b"23");
            close(&mut owner, &mut exposure);
            assert!(!service.connected());
            drop(exposure);
            drop(owner);
            TcpListener::bind(http_addr).unwrap();
            drop(cursor);
            drop(events);
            drop(query);
            drop(finite);
            assert!(
                producer_bank
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .used()
                    > 0
            );
            assert!(producer_bank.account(ResourceKind::CleanupItemsMax).used() > 0);
            assert!(
                consumer_bank
                    .account(ResourceKind::BindingResponseBufferBytesGlobalMax)
                    .used()
                    > 0
            );
            assert_eq!(retained.result().unwrap().data().unwrap().body, b"23");
            assert_eq!(
                serde_json::from_slice::<Value>(result.result().unwrap().data().unwrap().body)
                    .unwrap(),
                statuses
            );
            drop(retained);
            drop(result);
            clean(&producer_bank);
            clean(&consumer_bank);
            assert!(
                broker
                    .logs()
                    .lines()
                    .any(|l| l.contains("Received UNSUBSCRIBE from cr"))
            );
        });
    }
}
