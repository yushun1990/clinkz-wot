#![cfg(feature = "prepare")]
#[path = "support/services.rs"]
mod services;
mod support;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use services::*;
use support::*;

struct PropertyAccess;
impl ConsumerAccessService for PropertyAccess {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, op: Operation) -> CoreResult<()> {
        assert_eq!(op, Operation::ObserveProperty);
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
            Operation::ObserveProperty | Operation::UnobserveProperty
        ));
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
fn target() -> AffordanceTarget {
    AffordanceTarget::Property(Arc::from("temperature"))
}
fn prepare_consumer(
    value: serde_json::Value,
    registrations: &[&MqttConsumerService],
    selector: ConsumerSelector,
    bank: &Arc<HostResourceAccounts>,
) -> CoreResult<ConsumerNotificationDefinition<MqttNotificationConsumer>> {
    let registry = ConsumerNotificationRegistry::try_new(
        registrations
            .iter()
            .map(|s| ConsumerNotificationRegistration {
                binding: s.binding(),
                services: ConsumerServices {
                    data: Arc::new(MqttJsonData),
                    access: Arc::new(PropertyAccess),
                },
            })
            .collect(),
    )?;
    let thing = serde_json::from_value(value).unwrap();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    registry.prepare_notification(
        &thing,
        &target(),
        selector,
        &limits,
        bank.clone(),
        &mut budget(),
    )
}
fn producer(address: SocketAddr) -> MqttNotificationProducer {
    MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(address, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    )
}
fn consumer(address: SocketAddr, id: u32) -> MqttConsumerService {
    MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(address, 8192).unwrap(),
        BindingId::new(id),
        BindingGeneration::INITIAL,
    )
    .unwrap()
}

#[test]
fn property_observability_value_schema_and_protocol_limits_are_qualified_in_both_roles() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = producer(addr);
    let s = consumer(addr, 8);
    let bank = bank();
    let base = property_td(addr, "qualified/temperature");
    let mut bad = Vec::new();
    for (key, value) in [
        ("observable", serde_json::json!(false)),
        ("minimum", serde_json::json!(0)),
        ("enum", serde_json::json!([0, 1])),
        ("format", serde_json::json!("int32")),
        ("mqv:unknown", serde_json::json!(true)),
        // Event application roles on a Property must not be consumed as value schemas.
        ("data", serde_json::json!({"type":"integer"})),
        ("subscription", serde_json::json!({"type":"integer"})),
        ("cancellation", serde_json::json!({"type":"integer"})),
        ("dataResponse", serde_json::json!({"type":"integer"})),
    ] {
        let mut v = base.clone();
        v["properties"]["temperature"][key] = value;
        bad.push(v);
    }
    let mut v = base.clone();
    v["properties"]["temperature"]
        .as_object_mut()
        .unwrap()
        .remove("observable");
    bad.push(v);
    for (key, value) in [
        ("mqv:retain", serde_json::json!(true)),
        ("mqv:qos", serde_json::json!(1)),
        ("mqv:filter", serde_json::json!("qualified/+")),
        ("mqv:topic", serde_json::json!("different/topic")),
        ("mqv:controlPacket", serde_json::json!("publish")),
        (
            "response",
            serde_json::json!({"contentType":"application/json"}),
        ),
        ("contentCoding", serde_json::json!("gzip")),
        ("contentType", serde_json::json!("text/plain")),
        ("mqv:unknown", serde_json::json!(true)),
    ] {
        let mut v = base.clone();
        v["properties"]["temperature"]["forms"][0][key] = value;
        bad.push(v);
    }
    let mut v = base.clone();
    v["securityDefinitions"]["n"] = serde_json::json!({"scheme":"basic"});
    bad.push(v);
    for (i, v) in bad.into_iter().enumerate() {
        let instance = property_instance(authority(1), &v.to_string());
        assert!(prepare(&instance, &[&p]).is_err(), "Producer variant {i}");
        assert!(
            prepare_consumer(v, &[&s], ConsumerSelector::default(), &bank).is_err(),
            "Consumer variant {i}"
        );
        assert!(!s.connected());
        assert_empty(&bank);
    }
    let definition = prepare_consumer(base, &[&s], ConsumerSelector::default(), &bank).unwrap();
    assert_eq!(definition.input().target, target());
    assert_eq!(
        definition.input().schemas,
        ConsumerNotificationSchemas::PropertyValue(value_type::TypeSchema::Type(
            value_type::ValueType::Integer
        ))
    );
    assert!(
        definition
            .input()
            .binding
            .support(&definition.input().program, NotificationKind::Event)
            .is_err()
    );
    drop(definition);
    assert_empty(&bank);
}

#[test]
fn complete_producer_description_requires_real_property_source_and_all_advertised_operations() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = producer(addr);
    let base = property_td(addr, "source/temperature");
    let event_source = RawProducerInstance::new(authority(1), vec![])
        .with_notification_sources(vec![ProducerNotificationSource::event("temperature")])
        .with_description(Arc::from(base.to_string()));
    assert!(prepare(&event_source, &[&p]).is_err());
    for operations in [
        serde_json::json!(["readproperty", "observeproperty", "unobserveproperty"]),
        serde_json::json!(["observeproperty", "unobserveproperty", "writeproperty"]),
    ] {
        let mut v = base.clone();
        v["properties"]["temperature"]["forms"][0]["op"] = operations;
        assert!(prepare(&property_instance(authority(1), &v.to_string()), &[&p]).is_err());
    }
    let instance = property_instance(authority(1), &base.to_string());
    let prepared = prepare(&instance, &[&p]).unwrap();
    assert_eq!(prepared.operations().len(), 2);
    assert!(prepared.operations().iter().all(|o| matches!(
        o.operation,
        Operation::ObserveProperty | Operation::UnobserveProperty
    )));
}

#[test]
fn property_form_selection_keeps_order_actual_registration_and_stop_capability() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let s = consumer(addr, 8);
    let other = consumer(addr, 9);
    let bank = bank();
    let mut td = property_td(addr, "forms/temperature");
    let good = td["properties"]["temperature"]["forms"][0].clone();
    let mut bad = good.clone();
    bad["mqv:qos"] = serde_json::json!(1);
    td["properties"]["temperature"]["forms"] = serde_json::json!([bad, good]);
    let d = prepare_consumer(td.clone(), &[&s], ConsumerSelector::default(), &bank).unwrap();
    assert_eq!(d.input().form, 1);
    drop(d);
    assert!(
        prepare_consumer(
            td.clone(),
            &[&s],
            ConsumerSelector {
                form: Some(0),
                binding: None
            },
            &bank
        )
        .is_err()
    );
    assert!(
        prepare_consumer(
            td.clone(),
            &[&s, &other],
            ConsumerSelector::default(),
            &bank
        )
        .is_err()
    );
    let d = prepare_consumer(
        td.clone(),
        &[&s, &other],
        ConsumerSelector {
            form: Some(1),
            binding: Some(BindingId::new(9)),
        },
        &bank,
    )
    .unwrap();
    assert_eq!(d.input().identity.binding_id(), BindingId::new(9));
    drop(d);
    td["properties"]["temperature"]["forms"][1]["op"] = serde_json::json!("observeproperty");
    let d = prepare_consumer(td, &[&s], ConsumerSelector::default(), &bank).unwrap();
    assert!(
        !d.input()
            .binding
            .support(&d.input().program, NotificationKind::PropertyObservation)
            .unwrap()
            .cancellation
    );
    // Even a trusted source-free caller cannot pair a Property program with Event schemas.
    let mut input = d.into_input();
    input.schemas = ConsumerNotificationSchemas::Event {
        subscription: None,
        data: Some(value_type::TypeSchema::Any),
        data_response: None,
        cancellation: None,
    };
    let refused = ConsumerNotificationDefinition::try_new(input)
        .err()
        .unwrap()
        .into_input();
    drop(refused);
    assert_empty(&bank);
    assert!(!s.connected());
    assert!(!other.connected());
}

#[test]
fn final_property_description_cannot_change_value_schema_mapping_or_observability() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = producer(addr);
    let base = property_td(addr, "final/temperature");
    let instance = property_instance(authority(1), &base.to_string());
    let prepared = prepare(&instance, &[&p]).unwrap();
    let check = |text: &str| {
        check_final_thing(
            text,
            instance.key(),
            GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[&p],
            &prepared,
            |scope, op| matches!(scope, ValidatedProducerScope::Property(n) if instance.supports_property(n, op)),
        )
    };
    check(&base.to_string()).unwrap();
    for (key, value) in [
        ("type", serde_json::json!("string")),
        ("observable", serde_json::json!(false)),
    ] {
        let mut changed = base.clone();
        changed["properties"]["temperature"][key] = value;
        assert!(check(&changed.to_string()).is_err());
    }
    let mut changed = base;
    changed["properties"]["temperature"]["forms"][0]["mqv:filter"] =
        serde_json::json!("final/changed");
    assert!(check(&changed.to_string()).is_err());
}

#[test]
fn missing_property_services_and_cancelled_staging_release_original_capacity_without_io() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = producer(addr);
    let s = MqttNotificationService::new(&p).unwrap();
    let owner = owner();
    let bank = bank();
    let text = property_td(addr, "services/temperature").to_string();
    let (input, probe) =
        intercept_with_instance(&owner, property_instance(authority(1), &text), &p, &s, true);
    let invalid = CapturedNotificationRoute {
        target: AffordanceTarget::Action(Arc::from("not-a-notification")),
        route: NotificationRoute::new(p.identity()),
        services: probe,
        data_bytes: 1,
    };
    assert!(CapturedNotificationRoute::backing_bytes(&[invalid], 1).is_none());
    let refused =
        ComposedProducerExposure::try_new(&owner, input, limits(), bank.clone(), &mut budget())
            .err()
            .unwrap()
            .into_input();
    drop(refused);
    assert_empty(&bank);
    assert!(!s.connected());
    let input = input_with_instance(&owner, property_instance(authority(1), &text), &p, &s, None);
    let mut exposure = stage(&owner, input, &bank);
    exposure.begin_close();
    drop(exposure);
    assert_empty(&bank);
    assert!(!s.connected());
}
