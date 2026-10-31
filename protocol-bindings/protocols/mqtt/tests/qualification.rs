#![cfg(feature = "prepare")]
#[path = "support/services.rs"]
mod services;
mod support;
use services::*;
use support::*;

#[test]
fn final_qualification_rejects_foreign_definitions_with_matching_numeric_identity() {
    use clinkz_wot_binding_http::{HttpActionProducer, HttpFiniteProducer, HttpProducerConfig};

    fn behavior(scope: ValidatedProducerScope<'_>, operation: Operation) -> bool {
        matches!(scope, ValidatedProducerScope::Property("level"))
            && operation == Operation::ReadProperty
    }

    let addr = "127.0.0.1:18883".parse().unwrap();
    let identity = (BindingId::new(7), BindingGeneration::INITIAL);
    let config = HttpProducerConfig::for_loopback(addr).unwrap();
    let http = HttpFiniteProducer::try_new(config, identity.0, identity.1, 100).unwrap();
    let mqtt = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        identity.0,
        identity.1,
    );
    let action = HttpActionProducer::new(config, identity.0, identity.1);
    let instance = authority(1);
    let original = serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"Final qualification",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer","readOnly":true,
            "forms":[{"href":format!("http://{addr}/level")}]}}
    })
    .to_string();
    let prepared = prepare_thing(
        &original,
        instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[&http],
        behavior,
    )
    .unwrap();
    check_final_thing(
        &original,
        instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[&http],
        &prepared,
        behavior,
    )
    .unwrap();
    let changed = original
        .replace("integer", "string")
        .replace("/level", "/changed");
    assert!(
        check_final_thing(
            &changed,
            instance.key(),
            GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[&http],
            &prepared,
            behavior,
        )
        .is_err(),
        "the original registration must reject changed schema and path"
    );

    let mut accepted = Vec::new();
    for (name, substitute) in [
        ("MQTT Event", &mqtt as &dyn ProducerCapability),
        ("HTTP Action", &action as &dyn ProducerCapability),
    ] {
        assert_eq!(substitute.identity(), http.identity());
        assert!(
            prepare_thing(
                &original,
                instance.key(),
                GatewayDefaultV1::LIMITS,
                &mut budget(),
                &[substitute],
                behavior,
            )
            .is_err(),
            "{name} cannot prepare the original HTTP Property Read"
        );
        for (revision, text) in [("original", &original), ("changed", &changed)] {
            if check_final_thing(
                text,
                instance.key(),
                GatewayDefaultV1::LIMITS,
                &mut budget(),
                &[substitute],
                &prepared,
                behavior,
            )
            .is_ok()
            {
                accepted.push((name, revision));
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "foreign definitions passed final qualification: {accepted:?}"
    );
}

#[test]
fn same_numeric_slot_in_another_servient_is_not_topic_replacement_authority() {
    let addr = address();
    let provider = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let service = MqttNotificationService::new(&provider).unwrap();
    let first = owner();
    let second = owner();
    let original = input(
        &first,
        authority(1),
        &provider,
        &service,
        &td(addr, "owned/alarm").to_string(),
        None,
    );
    let foreign = instance(authority(1), &td(addr, "owned/alarm").to_string());
    let prepared = prepare(&foreign, &[&provider]).unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|o| {
            o.definition::<MqttNotificationDefinition>()
                .unwrap()
                .clone()
        })
        .collect();
    assert!(
        service
            .participation(&second, foreign.key(), definitions)
            .is_err()
    );
    // Explicit service sharing for a distinct source is still supported.
    let sibling = input(
        &second,
        authority(1),
        &provider,
        &service,
        &td(addr, "other/alarm").to_string(),
        None,
    );
    assert!(!service.connected());
    drop(original);
    drop(sibling);
    assert!(!service.connected());
}

#[test]
fn cancellation_before_native_setup_closes_qualification_and_releases_all_funding() {
    let addr = address();
    let provider = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let service = MqttNotificationService::new(&provider).unwrap();
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
    let provider = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let service = MqttNotificationService::new(&provider).unwrap();
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
    let provider = MqttNotificationProducer::new(
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
    let service = MqttNotificationService::new(&provider).unwrap();
    assert!(!service.connected());
    let other = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(addr, 8192).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let foreign_service = MqttNotificationService::new(&other).unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|p| {
            p.definition::<MqttNotificationDefinition>()
                .unwrap()
                .clone()
        })
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
    let provider = MqttNotificationProducer::new(
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
    let service = MqttNotificationService::new(&provider).unwrap();
    assert!(!service.connected());
    assert_eq!(service.locally_published(), 0);
}

#[test]
fn final_description_cannot_silently_change_mapping_or_schema() {
    let addr = address();
    let provider = MqttNotificationProducer::new(
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
                |s, op| matches!(s,ValidatedProducerScope::Event(n) if i.supports_notification(&AffordanceTarget::Event(Arc::from(n)),op))
            )
            .is_err()
        );
    }
}
