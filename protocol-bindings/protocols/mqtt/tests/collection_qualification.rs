#![cfg(feature = "prepare")]
mod support;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use support::*;

struct Access;
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, op: Operation) -> CoreResult<()> {
        assert_eq!(op, Operation::ObserveAllProperties);
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
        bank,
    )?;
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    registry.prepare_notification(
        &serde_json::from_value(td).unwrap(),
        &AffordanceTarget::Thing,
        NotificationKind::ObserveAllProperties,
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
fn root_form_and_complete_readable_member_schemas_are_required_in_both_roles() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = provider(addr);
    let s = service(addr, 8);
    let bank = bank();
    let base = collection_td(addr, "qualification/all");
    let mut bad = Vec::new();
    let mut no_root = base.clone();
    no_root.as_object_mut().unwrap().remove("forms");
    bad.push(no_root);
    for (field, value) in [
        ("op", serde_json::json!(["observeallproperties"])),
        ("op", serde_json::json!(["unobserveallproperties"])),
        (
            "op",
            serde_json::json!([
                "readallproperties",
                "observeallproperties",
                "unobserveallproperties"
            ]),
        ),
        ("mqv:filter", serde_json::json!("qualification/+")),
        (
            "mqv:filter",
            serde_json::json!("$share/group/qualification/all"),
        ),
        ("mqv:topic", serde_json::json!("another/topic")),
        ("mqv:controlPacket", serde_json::json!("subscribe")),
        ("mqv:qos", serde_json::json!(1)),
        ("mqv:retain", serde_json::json!(true)),
        ("contentType", serde_json::json!("application/cbor")),
        ("contentCoding", serde_json::json!("gzip")),
        ("subprotocol", serde_json::json!("custom")),
        ("mqv:unknown", serde_json::json!(true)),
        (
            "response",
            serde_json::json!({"contentType":"application/json"}),
        ),
        ("additionalResponses", serde_json::json!([{"success":true}])),
    ] {
        let mut v = base.clone();
        v["forms"][0][field] = value;
        bad.push(v);
    }
    for (field, value) in [
        ("minimum", serde_json::json!(0)),
        ("enum", serde_json::json!([0, 1])),
        ("format", serde_json::json!("int32")),
        (
            "properties",
            serde_json::json!({"nested":{"type":"string"}}),
        ),
        ("mqv:assertion", serde_json::json!(true)),
    ] {
        let mut v = base.clone();
        v["properties"]["temperature"][field] = value;
        bad.push(v);
    }
    let mut secured = base.clone();
    secured["securityDefinitions"]["n"]["scheme"] = serde_json::json!("basic");
    bad.push(secured);
    let mut variable = base.clone();
    variable["uriVariables"] = serde_json::json!({"x":{"type":"integer"}});
    bad.push(variable);
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
        // Without a root Form the Producer has no advertised root capability;
        // its registered root source must still refuse complete qualification.
        let instance = collection_instance(authority(1), &td.to_string());
        if td.get("forms").is_some() {
            assert!(prepare(&instance, &[&p]).is_err(), "Producer accepted {td}");
        }
        assert!(!s.connected());
        assert_empty(&bank);
    }
}

#[test]
fn root_selection_is_independent_of_leaf_mappings_and_observable_hint() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let s = service(addr, 8);
    let bank = bank();
    let mut td = collection_td(addr, "meaning/all");
    td["properties"]["temperature"]["observable"] = serde_json::json!(false);
    td["properties"]["temperature"]["forms"] =
        serde_json::json!([{"href":"https://unsupported.example/value","op":"readproperty"}]);
    td["properties"][""] = td["properties"]["temperature"].take();
    td["properties"]
        .as_object_mut()
        .unwrap()
        .remove("temperature");
    td["properties"]["secret"] = serde_json::json!({"type":"string","writeOnly":true,"forms":[{"href":"https://unsupported.example/secret","op":"writeproperty"}]});
    // Empty names, Unicode and punctuation are exact data keys, independent
    // of the root's own identity or any MQTT topic extraction rule.
    td["properties"]["开/关"] = td["properties"]["enabled"].take();
    td["properties"].as_object_mut().unwrap().remove("enabled");
    let d = selected(
        td,
        &[&s],
        ConsumerSelector::default(),
        &bank,
        Arc::new(MqttJsonData),
        &mut budget(),
    )
    .unwrap();
    assert_eq!(d.input().target, AffordanceTarget::Thing);
    assert_eq!(
        d.input().schemas.kind(),
        NotificationKind::ObserveAllProperties
    );
    let NotificationSchemas::ObserveAllProperties(schema) = &d.input().schemas else {
        panic!()
    };
    assert_eq!(
        schema
            .members()
            .iter()
            .map(|m| m.name.as_ref())
            .collect::<Vec<_>>(),
        ["", "开/关"]
    );
    assert!(schema.members().iter().all(|m| m.name.as_ref() != "secret"));
    MqttJsonData
        .validate_property_map(
            schema,
            &Payload::new("{\"\":7,\"开/关\":true}".as_bytes(), "application/json"),
        )
        .unwrap();
    drop(d);
    assert_empty(&bank);
    assert!(!s.connected());
}

#[test]
fn scalar_only_data_services_short_work_and_ambiguous_registrations_refuse_without_io() {
    struct ScalarOnly;
    impl ConsumerDataService for ScalarOnly {
        fn content_type(&self) -> &str {
            "application/json"
        }
        fn policy(&self) -> ConsumerDataPolicy {
            MqttJsonData.policy()
        }
        fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
            MqttJsonData.validate(&s, p)
        }
    }
    let addr = "127.0.0.1:18883".parse().unwrap();
    let a = service(addr, 8);
    let z = service(addr, 9);
    let bank = bank();
    let td = collection_td(addr, "selection/all");
    assert!(
        selected(
            td.clone(),
            &[&a],
            ConsumerSelector::default(),
            &bank,
            Arc::new(ScalarOnly),
            &mut budget()
        )
        .is_err()
    );
    assert!(
        selected(
            td.clone(),
            &[&a, &z],
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
            &[&a],
            ConsumerSelector::default(),
            &bank,
            Arc::new(MqttJsonData),
            &mut WorkBudget::new()
        )
        .is_err()
    );
    let d = selected(
        td.clone(),
        &[&a, &z],
        ConsumerSelector {
            binding: Some(BindingId::new(9)),
            form: Some(0),
        },
        &bank,
        Arc::new(MqttJsonData),
        &mut budget(),
    )
    .unwrap();
    assert_eq!(d.input().identity.binding_id(), BindingId::new(9));
    drop(d);
    let mut alternatives = td;
    alternatives["forms"].as_array_mut().unwrap().insert(0, serde_json::json!({"href":"http://127.0.0.1/properties","op":["observeallproperties","unobserveallproperties"]}));
    let d = selected(
        alternatives,
        &[&a],
        ConsumerSelector::default(),
        &bank,
        Arc::new(MqttJsonData),
        &mut budget(),
    )
    .unwrap();
    assert_eq!(d.input().form, 1);
    drop(d);
    assert!(!a.connected() && !z.connected());
    assert_empty(&bank);
}

#[test]
fn producer_requires_a_distinct_root_source_and_requalifies_all_final_meaning() {
    let addr = "127.0.0.1:18883".parse().unwrap();
    let p = provider(addr);
    let td = collection_td(addr, "final/all");
    let leaves = RawProducerInstance::new(authority(1), vec![])
        .with_description(Arc::from(td.to_string()))
        .with_notification_sources(vec![
            ProducerNotificationSource::property("temperature"),
            ProducerNotificationSource::property("enabled"),
        ]);
    assert!(
        prepare(&leaves, &[&p]).is_err(),
        "leaf observation is not root support"
    );
    let instance = collection_instance(authority(1), &td.to_string());
    let prepared = prepare(&instance, &[&p]).unwrap();
    assert_eq!(
        prepared
            .operations()
            .iter()
            .filter(|o| o.site == FormSite::Thing)
            .count(),
        2
    );
    for change in 0..5 {
        let mut changed = td.clone();
        match change {
            0 => changed["properties"]["temperature"]["type"]=serde_json::json!("string"),
            1 => changed["forms"][0]["mqv:filter"]=serde_json::json!("final/another"),
            2 => changed["forms"].as_array_mut().unwrap().push(serde_json::json!({"href":"http://127.0.0.1/unsupported","op":"subscribeallevents"})),
            3 => changed["properties"]["enabled"]["forms"][0]["op"]=serde_json::json!("readproperty"),
            _ => changed["properties"]["temperature"]["writeOnly"]=serde_json::json!(true),
        }
        assert!(
            check_final_thing(
                &changed.to_string(),
                instance.key(),
                GatewayDefaultV1::LIMITS,
                &mut budget(),
                &[&p],
                &prepared,
                |s, op| match s {
                    ValidatedProducerScope::Thing =>
                        instance.supports(&AffordanceTarget::Thing, op),
                    ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
                    _ => false,
                }
            )
            .is_err()
        );
    }
}
