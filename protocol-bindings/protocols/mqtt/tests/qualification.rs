#![cfg(feature = "prepare")]
#[path = "support/services.rs"]
mod services;
mod support;
use services::*;
use support::*;

#[test]
fn cancellation_before_native_setup_closes_qualification_and_releases_all_funding() {
    let addr = address();
    let provider = MqttEventProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let service = MqttEventService::new(&provider).unwrap();
    let accounts = bank();
    let mut owner = owner();
    let mut exposure = stage(
        &owner,
        input(
            &owner,
            authority(1),
            &provider,
            &service,
            &td(addr, "alarm").to_string(),
            None,
        ),
        &accounts,
    );
    let key = exposure.key();
    exposure.begin_close();
    assert!(exposure.publish(&mut owner, &mut budget()).is_err());
    assert!(
        native(&mut exposure)
            .step(&mut owner, &mut cx(), &mut WorkBudget::new())
            .unwrap()
            .is_pending()
    );
    assert!(
        native(&mut exposure)
            .step(&mut owner, &mut cx(), &mut budget())
            .unwrap()
            .is_ready()
    );
    exposure.retire(&mut owner, &mut budget()).unwrap();
    assert!(owner.description(key).is_none());
    assert!(!service.connected());
    assert_eq!(service.locally_published(), 0);
    drop(exposure);
    assert_empty(&accounts);
}

#[test]
fn missing_actual_event_services_and_unfunded_handoffs_refuse_before_setup() {
    let addr = address();
    let provider = MqttEventProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let service = MqttEventService::new(&provider).unwrap();
    let owner = owner();
    let accounts = bank();
    let (missing, _) = intercept(
        &owner,
        authority(1),
        &provider,
        &service,
        &td(addr, "alarm").to_string(),
        true,
    );
    assert!(
        ComposedProducerExposure::try_new(
            &owner,
            missing,
            limits(),
            Arc::clone(&accounts),
            &mut budget()
        )
        .is_err()
    );
    assert!(!service.connected());
    assert_empty(&accounts);
    let mut tiny = GatewayDefaultV1::LIMITS.clone();
    assert!(tiny.set(ResourceKind::EngineLiveBytesGlobalMax, Some(1)));
    let accounts = Arc::new(HostResourceAccounts::new(&tiny));
    let input = input(
        &owner,
        authority(1),
        &provider,
        &service,
        &td(addr, "alarm").to_string(),
        None,
    );
    let key = input.instance.key();
    let rejected = ComposedProducerExposure::try_new(
        &owner,
        input,
        limits(),
        Arc::clone(&accounts),
        &mut budget(),
    )
    .err()
    .unwrap();
    assert_eq!(rejected.into_input().instance.key(), key);
    assert!(!service.connected());
    assert_empty(&accounts);
}

#[test]
fn default_operations_aliases_and_actual_registration_are_prepared_without_io() {
    let addr = address();
    let provider = MqttEventProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let mut json = td(addr, "thing/alarm");
    json["@context"][1]["topicFilter"] =
        serde_json::json!("https://www.w3.org/2019/wot/mqtt#filter");
    let form = json["events"]["alarm"]["forms"][0].as_object_mut().unwrap();
    let topic = form.remove("mqv:filter").unwrap();
    form.insert("topicFilter".into(), topic);
    let i = instance(authority(1), &json.to_string());
    let prepared = prepare(&i, &[&provider]).unwrap();
    assert_eq!(prepared.operations().len(), 2);
    assert_eq!(
        prepared.operations()[0].operation,
        Operation::SubscribeEvent
    );
    assert_eq!(
        prepared.operations()[1].operation,
        Operation::UnsubscribeEvent
    );
    let service = MqttEventService::new(&provider).unwrap();
    assert!(!service.connected());
    let other = MqttEventProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let foreign_service = MqttEventService::new(&other).unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|p| p.definition::<MqttEventDefinition>().unwrap().clone())
        .collect();
    assert!(
        foreign_service
            .participation(&owner(), i.key(), definitions)
            .is_err(),
        "equal tags cannot pair different registrations"
    );
    assert!(!foreign_service.connected());
}

#[test]
fn every_unsupported_advertised_semantic_refuses_before_native_effects() {
    let addr = address();
    let provider = MqttEventProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let original = td(addr, "thing/alarm");
    let mut variants = Vec::new();
    for role in ["subscription", "cancellation", "dataResponse"] {
        let mut j = original.clone();
        j["events"]["alarm"][role] = serde_json::json!({"type":"integer"});
        variants.push((role, j));
    }
    for (key, value) in [
        ("minimum", serde_json::json!(0)),
        ("enum", serde_json::json!([1, 2])),
        ("oneOf", serde_json::json!([{"type":"integer"}])),
    ] {
        let mut j = original.clone();
        j["events"]["alarm"]["data"][key] = value;
        variants.push((key, j));
    }
    for (key, value) in [
        ("mqv:qos", serde_json::json!("1")),
        ("mqv:retain", serde_json::json!(true)),
        ("mqv:filter", serde_json::json!("thing/+")),
        ("mqv:filter", serde_json::json!(["thing/alarm"])),
        ("mqv:topic", serde_json::json!("other")),
        ("mqv:controlPacket", serde_json::json!("publish")),
        ("mqv:unknown", serde_json::json!(true)),
        (
            "response",
            serde_json::json!({"contentType":"application/json"}),
        ),
        (
            "additionalResponses",
            serde_json::json!([{"contentType":"application/json"}]),
        ),
        ("contentType", serde_json::json!("text/plain")),
        ("subprotocol", serde_json::json!("custom")),
        ("op", serde_json::json!("readproperty")),
        ("security", serde_json::json!([])),
    ] {
        let mut j = original.clone();
        j["events"]["alarm"]["forms"][0][key] = value;
        variants.push((key, j));
    }
    let mut j = original.clone();
    j["securityDefinitions"]["n"]["scheme"] = serde_json::json!("basic");
    variants.push(("security", j));
    let mut j = original.clone();
    j["profile"] = serde_json::json!(["https://example.com/profile"]);
    variants.push(("profile", j));
    let mut j = original.clone();
    j["events"]["alarm"]["forms"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"href":"http://127.0.0.1/unsupported", "op":"subscribeevent"}));
    variants.push(("extra Form", j));
    let mut j = original.clone();
    j["forms"] = serde_json::json!([{"href":format!("mqtt://{addr}"),"op":"subscribeallevents","mqv:filter":"thing/#"}]);
    variants.push(("collection", j));
    for (case, j) in variants {
        let i = instance(authority(1), &j.to_string());
        assert!(prepare(&i, &[&provider]).is_err(), "accepted {case}: {j}");
    }
    let service = MqttEventService::new(&provider).unwrap();
    assert!(!service.connected());
    assert_eq!(service.locally_published(), 0);
}

#[test]
fn final_description_cannot_silently_change_mapping_or_schema() {
    let addr = address();
    let provider = MqttEventProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let original = td(addr, "original");
    let i = instance(authority(1), &original.to_string());
    let prepared = prepare(&i, &[&provider]).unwrap();
    for topic in [true, false] {
        let mut j = original.clone();
        if topic {
            j["events"]["alarm"]["forms"][0]["mqv:filter"] = serde_json::json!("changed");
        } else {
            j["events"]["alarm"]["data"]["type"] = serde_json::json!("string");
        }
        assert!(
            check_final_thing(
                &j.to_string(),
                i.key(),
                GatewayDefaultV1::LIMITS,
                &mut budget(),
                &[&provider],
                &prepared,
                |s, op| matches!(s,ValidatedProducerScope::Event(n) if i.supports_event(n,op))
            )
            .is_err()
        );
    }
}
