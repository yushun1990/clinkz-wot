#![cfg(feature = "prepare")]
mod support;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use support::*;

struct Access;
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, op: Operation) -> CoreResult<()> {
        assert_eq!(op, Operation::SubscribeAllEvents);
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        t: &str,
        op: Operation,
    ) -> CoreResult<()> {
        self.authorize(b, t, op)
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
fn service(addr: SocketAddr, id: u32) -> MqttConsumerService {
    MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(id),
        BindingGeneration::INITIAL,
    )
    .unwrap()
}
fn selected(
    td: serde_json::Value,
    services: &[&MqttConsumerService],
    selector: ConsumerSelector,
    bank: &Arc<HostResourceAccounts>,
    data: Arc<dyn ConsumerDataService>,
    work: &mut WorkBudget,
) -> CoreResult<ConsumerNotificationDefinition<MqttNotificationConsumer>> {
    let registry = ConsumerNotificationRegistry::try_new(
        services
            .iter()
            .map(|s| ConsumerNotificationRegistration {
                binding: s.binding(),
                services: ConsumerServices {
                    data: data.clone(),
                    access: Arc::new(Access),
                },
            })
            .collect(),
    )?;
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    registry.prepare_notification(
        &serde_json::from_value(td).unwrap(),
        &AffordanceTarget::Thing,
        NotificationKind::SubscribeAllEvents,
        selector,
        &limits,
        bank.clone(),
        work,
    )
}
fn provider(addr: SocketAddr) -> MqttNotificationProducer {
    MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    )
}
#[test]
fn duplicate_expanded_event_data_roles_refuse_collection_qualification_in_both_roles() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = provider(addr);
    let s = service(addr, 8);
    let bank = bank();
    for event in ["alarm", "ready"] {
        let mut base = event_collection_td(addr, "duplicate/events");
        base["events"][event]["@context"] = serde_json::json!({
            "td": "https://www.w3.org/2019/wot/td#",
            "extraData": "https://www.w3.org/2019/wot/td#data"
        });
        let supported = selected(
            base.clone(),
            &[&s],
            ConsumerSelector::default(),
            &bank,
            Arc::new(MqttJsonData),
            &mut budget(),
        )
        .unwrap();
        drop(supported);
        assert!(
            prepare(
                &event_collection_instance(authority(1), &base.to_string()),
                &[&p]
            )
            .is_ok()
        );
        for key in [
            "https://www.w3.org/2019/wot/td#data",
            "td:data",
            "extraData",
        ] {
            let mut td = base.clone();
            td["events"][event][key] = serde_json::json!({"type":"string","minLength":100});
            assert!(
                selected(
                    td.clone(),
                    &[&s],
                    ConsumerSelector::default(),
                    &bank,
                    Arc::new(MqttJsonData),
                    &mut budget()
                )
                .is_err(),
                "Consumer ignored extra {key} meaning on {event}"
            );
            assert!(
                prepare(
                    &event_collection_instance(authority(1), &td.to_string()),
                    &[&p]
                )
                .is_err(),
                "Producer ignored extra {key} meaning on {event}"
            );
            assert!(!s.connected());
            assert_empty(&bank);
        }
    }
}
#[test]
fn both_roles_refuse_incomplete_mapping_unknown_semantics_and_every_unsupported_event_role() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = provider(addr);
    let s = service(addr, 8);
    let bank = bank();
    let base = event_collection_td(addr, "qualification/events");
    let mut bad = Vec::new();
    for (field, value) in [
        ("op", serde_json::json!(["subscribeallevents"])),
        ("op", serde_json::json!(["unsubscribeallevents"])),
        (
            "op",
            serde_json::json!([
                "subscribeallevents",
                "unsubscribeallevents",
                "observeallproperties"
            ]),
        ),
        ("mqv:filter", serde_json::json!("events/#")),
        ("mqv:filter", serde_json::json!("events")),
        ("mqv:filter", serde_json::json!("events/+/nested/+")),
        ("mqv:filter", serde_json::json!("$share/group/events/+")),
        ("mqv:topic", serde_json::json!("qualification/events/alarm")),
        ("mqv:controlPacket", serde_json::json!("subscribe")),
        ("mqv:qos", serde_json::json!(1)),
        ("mqv:retain", serde_json::json!(true)),
        ("contentType", serde_json::json!("application/cbor")),
        ("contentCoding", serde_json::json!("gzip")),
        ("subprotocol", serde_json::json!("made-up")),
        ("mqv:unknown", serde_json::json!(true)),
        (
            "response",
            serde_json::json!({"contentType":"application/json"}),
        ),
        ("additionalResponses", serde_json::json!([{"success":true}])),
    ] {
        let mut td = base.clone();
        td["forms"][0][field] = value;
        bad.push(td);
    }
    for role in ["subscription", "dataResponse", "cancellation"] {
        for name in ["alarm", "ready"] {
            let mut td = base.clone();
            td["events"][name][role] = serde_json::json!({"type":"integer"});
            bad.push(td);
        }
    }
    for (field, value) in [
        ("minimum", serde_json::json!(0)),
        ("enum", serde_json::json!([0, 1])),
        ("format", serde_json::json!("int32")),
        ("mqv:assertion", serde_json::json!(true)),
        (
            "properties",
            serde_json::json!({"nested":{"type":"string"}}),
        ),
    ] {
        let mut td = base.clone();
        td["events"]["alarm"]["data"][field] = value;
        bad.push(td);
    }
    for name in ["", "警报", "nested/alarm", "a+", "a#", "$system"] {
        let mut td = base.clone();
        let event = td["events"]
            .as_object_mut()
            .unwrap()
            .remove("alarm")
            .unwrap();
        td["events"][name] = event;
        bad.push(td);
    }
    let mut no_data = base.clone();
    no_data["events"]["alarm"]
        .as_object_mut()
        .unwrap()
        .remove("data");
    bad.push(no_data);
    let mut empty = base.clone();
    empty["events"] = serde_json::json!({});
    bad.push(empty);
    let mut secured = base.clone();
    secured["securityDefinitions"]["n"]["scheme"] = serde_json::json!("basic");
    bad.push(secured);
    let mut variable = base.clone();
    variable["uriVariables"] = serde_json::json!({"x":{"type":"string"}});
    bad.push(variable);
    let mut extension = base.clone();
    extension["events"]["ready"]["mqv:unknown"] = serde_json::json!(true);
    bad.push(extension);
    for td in bad {
        assert!(
            selected(
                td.clone(),
                &[&s],
                ConsumerSelector::default(),
                &bank,
                Arc::new(MqttJsonData),
                &mut budget()
            )
            .is_err(),
            "Consumer accepted {td}"
        );
        assert!(
            prepare(
                &event_collection_instance(authority(1), &td.to_string()),
                &[&p]
            )
            .is_err(),
            "Producer accepted {td}"
        );
        assert!(!s.connected());
        assert_empty(&bank);
    }
}
#[test]
fn root_retains_exact_per_event_schemas_independently_of_leaf_forms_and_source_td() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let s = service(addr, 8);
    let bank = bank();
    let mut td = event_collection_td(addr, "meaning/events");
    td["events"]["alarm"]["forms"] =
        serde_json::json!([{"href":"https://unsupported.example/alarm"}]);
    let d = selected(
        td,
        &[&s],
        ConsumerSelector::default(),
        &bank,
        Arc::new(MqttJsonData),
        &mut budget(),
    )
    .unwrap();
    let ConsumerNotificationSchemas::SubscribeAllEvents(schema) = &d.input().schemas else {
        panic!()
    };
    assert_eq!(schema.members().len(), 2);
    assert_eq!(
        schema.member("alarm").unwrap().data,
        Some(data_schema::PreparedSchema::Type(
            data_schema::ValueType::Integer
        ))
    );
    assert_eq!(
        schema.member("ready").unwrap().data,
        Some(data_schema::PreparedSchema::Type(
            data_schema::ValueType::Boolean
        ))
    );
    assert!(schema.members().iter().all(|m| m.subscription.is_none()
        && m.cancellation.is_none()
        && m.data_response.is_none()));
    assert!(!s.connected());
    drop(d);
    assert_empty(&bank);
}
#[test]
fn actual_services_ambiguity_short_credit_and_root_source_coverage_are_checked_without_io() {
    struct NoBoolean;
    impl ConsumerDataService for NoBoolean {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
            MqttJsonData.validate(s, p)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            MqttJsonData.policy()
        }
        fn supports_schema(&self, s: &data_schema::PreparedSchema) -> bool {
            s != &data_schema::PreparedSchema::Type(data_schema::ValueType::Boolean)
        }
    }
    let addr = "127.0.0.1:18883".parse().unwrap();
    let s = service(addr, 8);
    let other = service(addr, 9);
    let p = provider(addr);
    let bank = bank();
    let td = event_collection_td(addr, "selection/events");
    assert!(
        selected(
            td.clone(),
            &[&s],
            ConsumerSelector::default(),
            &bank,
            Arc::new(NoBoolean),
            &mut budget()
        )
        .is_err()
    );
    assert!(
        selected(
            td.clone(),
            &[&s, &other],
            ConsumerSelector::default(),
            &bank,
            Arc::new(MqttJsonData),
            &mut budget()
        )
        .is_err()
    );
    assert!(
        selected(
            td.clone(),
            &[&s],
            ConsumerSelector::default(),
            &bank,
            Arc::new(MqttJsonData),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    let d = selected(
        td.clone(),
        &[&s, &other],
        ConsumerSelector {
            binding: Some(BindingId::new(9)),
            ..ConsumerSelector::default()
        },
        &bank,
        Arc::new(MqttJsonData),
        &mut budget(),
    )
    .unwrap();
    drop(d);
    let no_root_source = RawProducerInstance::new(authority(1), vec![])
        .with_description(Arc::from(td.to_string()))
        .with_notification_sources(vec![
            ProducerNotificationSource::event("alarm"),
            ProducerNotificationSource::event("ready"),
        ]);
    assert!(prepare(&no_root_source, &[&p]).is_err());
    let no_leaf_source = RawProducerInstance::new(authority(1), vec![])
        .with_description(Arc::from(td.to_string()))
        .with_notification_sources(vec![ProducerNotificationSource::AllEvents]);
    assert!(
        prepare(&no_leaf_source, &[&p]).is_err(),
        "root source cannot satisfy advertised leaf operations"
    );
    let mut no_root = td.clone();
    no_root.as_object_mut().unwrap().remove("forms");
    assert!(
        selected(
            no_root,
            &[&s],
            ConsumerSelector::default(),
            &bank,
            Arc::new(MqttJsonData),
            &mut budget()
        )
        .is_err()
    );
    assert!(!s.connected());
    assert_empty(&bank);
}

#[test]
fn final_td_requalifies_all_event_roles_root_mapping_and_every_advertised_operation() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = provider(addr);
    let td = event_collection_td(addr, "final/events");
    let instance = event_collection_instance(authority(1), &td.to_string());
    let prepared = prepare(&instance, &[&p]).unwrap();
    assert_eq!(
        prepared
            .operations()
            .iter()
            .filter(|o| o.site == FormSite::Thing)
            .count(),
        2
    );
    for change in 0..7 {
        let mut changed = td.clone();
        match change {
            0=>changed["events"]["alarm"]["data"]["type"]=serde_json::json!("boolean"),
            1=>changed["events"]["ready"]["dataResponse"]=serde_json::json!({"type":"string"}),
            2=>changed["forms"][0]["mqv:filter"]=serde_json::json!("other/events/+"),
            3=>changed["forms"].as_array_mut().unwrap().push(serde_json::json!({"href":"http://127.0.0.1/unsupported","op":"observeallproperties"})),
            4=>changed["events"]["ready"]["forms"][0]["href"]=serde_json::json!("https://unsupported.example/ready"),
            5=>changed["events"]["alarm"]["subscription"]=serde_json::json!({"type":"integer"}),
            _=>changed["events"]["alarm"]["cancellation"]=serde_json::json!({"type":"integer"})
        }
        assert!(
            check_final_thing(
                &changed.to_string(),
                instance.key(),
                GatewayDefaultV1::LIMITS,
                &mut budget(),
                &[&p],
                &prepared,
                |scope, op| match scope {
                    ValidatedProducerScope::Thing =>
                        instance.supports(&AffordanceTarget::Thing, op),
                    ValidatedProducerScope::Event(n) => instance.supports_event(n, op),
                    _ => false,
                }
            )
            .is_err(),
            "accepted changed TD {changed}"
        );
    }
}
