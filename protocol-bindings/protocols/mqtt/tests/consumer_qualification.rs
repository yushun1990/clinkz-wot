#![cfg(feature = "prepare")]
mod support;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{consumer_notification::*, notification::*};
use std::sync::atomic::{AtomicBool, Ordering};
use support::*;

struct Access;
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
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
struct Data(Arc<AtomicBool>);
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn validate(&self, schema: value_type::TypeSchema, payload: &Payload) -> CoreResult<()> {
        MqttJsonData.validate(schema, payload)
    }
    fn available(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        MqttJsonData.policy()
    }
}
fn services() -> ConsumerServices {
    ConsumerServices {
        data: Arc::new(MqttJsonData),
        access: Arc::new(Access),
    }
}
fn service(address: SocketAddr, id: u32) -> MqttConsumerService {
    MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(address, 8192).unwrap(),
        BindingId::new(id),
        BindingGeneration::INITIAL,
    )
    .unwrap()
}
fn prepare(
    value: serde_json::Value,
    inputs: Vec<ConsumerNotificationRegistration<MqttNotificationConsumer>>,
    selector: ConsumerSelector,
    bank: &Arc<HostResourceAccounts>,
) -> CoreResult<ConsumerNotificationDefinition<MqttNotificationConsumer>> {
    let registry = ConsumerNotificationRegistry::try_new(inputs)?;
    let thing = serde_json::from_value(value).unwrap();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    registry.prepare_notification(
        &thing,
        &AffordanceTarget::Event(Arc::from("alarm")),
        selector,
        &limits,
        bank.clone(),
        &mut budget(),
    )
}
fn registration(
    s: &MqttConsumerService,
) -> ConsumerNotificationRegistration<MqttNotificationConsumer> {
    ConsumerNotificationRegistration {
        binding: s.binding(),
        services: services(),
    }
}
fn owner() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
}
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
fn common_schema_security_response_and_protocol_assertions_refuse_without_native_effects() {
    let address = address();
    let s = service(address, 8);
    let bank = bank();
    let base = td(address, "qualification/alarm");
    let mut variants = Vec::new();
    for role in ["subscription", "cancellation", "dataResponse"] {
        let mut v = base.clone();
        v["events"]["alarm"][role] = serde_json::json!({"type":"integer"});
        variants.push(v);
    }
    for (key, value) in [
        ("mqv:qos", serde_json::json!(1)),
        ("mqv:retain", serde_json::json!(true)),
        ("mqv:filter", serde_json::json!("qualification/+")),
        ("mqv:topic", serde_json::json!("other/topic")),
        ("mqv:controlPacket", serde_json::json!("publish")),
        (
            "response",
            serde_json::json!({"contentType":"application/json"}),
        ),
        ("contentType", serde_json::json!("text/plain")),
        ("contentCoding", serde_json::json!("gzip")),
        ("mqv:unknown", serde_json::json!(true)),
    ] {
        let mut v = base.clone();
        v["events"]["alarm"]["forms"][0][key] = value;
        variants.push(v);
    }
    let mut v = base.clone();
    v["events"]["alarm"]["data"]["minimum"] = serde_json::json!(1);
    variants.push(v);
    let mut v = base.clone();
    v["securityDefinitions"]["n"] = serde_json::json!({"scheme":"basic"});
    variants.push(v);
    let mut v = base.clone();
    v["profile"] = serde_json::json!(["urn:unsupported"]);
    variants.push(v);
    for (i, v) in variants.into_iter().enumerate() {
        assert!(
            prepare(
                v,
                vec![registration(&s)],
                ConsumerSelector::default(),
                &bank
            )
            .is_err(),
            "variant {i}"
        );
        assert!(!s.connected());
        assert_empty(&bank);
    }
    let good = prepare(
        base,
        vec![registration(&s)],
        ConsumerSelector::default(),
        &bank,
    )
    .unwrap();
    assert!(good.input().schemas.data().is_some());
    assert!(good.input().schemas.subscription().is_none());
    assert!(good.input().schemas.cancellation().is_none());
    assert!(good.input().schemas.data_response().is_none());
    drop(good);
    assert_empty(&bank);
}

#[test]
fn td_form_order_explicit_selectors_and_actual_provider_ambiguity_are_preserved() {
    let address = address();
    let s = service(address, 8);
    let second = service(address, 9);
    let bank = bank();
    let mut value = td(address, "forms/alarm");
    let mut unsupported = value["events"]["alarm"]["forms"][0].clone();
    unsupported["mqv:qos"] = serde_json::json!(1);
    let selected = value["events"]["alarm"]["forms"][0].clone();
    value["events"]["alarm"]["forms"] = serde_json::json!([unsupported, selected]);
    let definition = prepare(
        value.clone(),
        vec![registration(&s)],
        ConsumerSelector::default(),
        &bank,
    )
    .unwrap();
    assert_eq!(definition.input().form, 1);
    drop(definition);
    assert_empty(&bank);
    assert!(
        prepare(
            value.clone(),
            vec![registration(&s)],
            ConsumerSelector {
                form: Some(0),
                binding: None
            },
            &bank
        )
        .is_err()
    );
    assert!(
        prepare(
            value.clone(),
            vec![registration(&s), registration(&second)],
            ConsumerSelector::default(),
            &bank
        )
        .is_err()
    );
    assert_empty(&bank);
    let definition = prepare(
        value,
        vec![registration(&s), registration(&second)],
        ConsumerSelector {
            form: Some(1),
            binding: Some(BindingId::new(9)),
        },
        &bank,
    )
    .unwrap();
    assert_eq!(definition.input().identity.binding_id(), BindingId::new(9));
    drop(definition);
    assert_empty(&bank);
    assert!(!s.connected());
    assert!(!second.connected());
}

#[test]
fn equal_numeric_registration_and_configuration_cannot_substitute_an_actual_native_service() {
    let address = address();
    let s = service(address, 8);
    let second = service(address, 8);
    let bank = bank();
    let definition = prepare(
        td(address, "actual/alarm"),
        vec![registration(&s)],
        ConsumerSelector::default(),
        &bank,
    )
    .unwrap();
    let mut input = definition.into_input();
    input.binding = second.binding();
    let refused = ConsumerNotificationDefinition::try_new(input)
        .err()
        .unwrap()
        .into_input();
    assert!(!s.connected());
    assert!(!second.connected());
    drop(refused);
    assert_empty(&bank);
}

#[test]
fn captured_data_service_availability_and_capacity_are_checked_before_runtime_installation() {
    let address = address();
    let s = service(address, 8);
    let bank = bank();
    let available = Arc::new(AtomicBool::new(true));
    let input = ConsumerNotificationRegistration {
        binding: s.binding(),
        services: ConsumerServices {
            data: Arc::new(Data(available.clone())),
            access: Arc::new(Access),
        },
    };
    let definition = prepare(
        td(address, "services/alarm"),
        vec![input],
        ConsumerSelector::default(),
        &bank,
    )
    .unwrap();
    available.store(false, Ordering::Relaxed);
    let owner = owner();
    let refused = ConsumerNotificationRuntime::try_new(owner.reference(), definition, raw_limits())
        .err()
        .unwrap()
        .into_input();
    available.store(true, Ordering::Relaxed);
    let mut short = raw_limits();
    short.engine_bytes = 1;
    let refused = ConsumerNotificationRuntime::try_new(owner.reference(), refused, short)
        .err()
        .unwrap()
        .into_input();
    let runtime = ConsumerNotificationRuntime::try_new(owner.reference(), refused, raw_limits())
        .unwrap_or_else(|_| panic!());
    drop(runtime);
    assert_empty(&bank);
    assert!(!s.connected());
}

#[test]
fn subscribe_only_form_allows_local_stop_and_explicitly_refuses_mapped_cancellation() {
    let address = address();
    let s = service(address, 8);
    let bank = bank();
    let mut value = td(address, "control/alarm");
    value["events"]["alarm"]["forms"][0]["op"] = serde_json::json!("subscribeevent");
    let definition = prepare(
        value,
        vec![registration(&s)],
        ConsumerSelector::default(),
        &bank,
    )
    .unwrap();
    assert!(
        !definition
            .input()
            .binding
            .support(&definition.input().program, NotificationKind::Event)
            .unwrap()
            .cancellation
    );
    let owner = owner();
    let mut runtime =
        ConsumerNotificationRuntime::try_new(owner.reference(), definition, raw_limits())
            .unwrap_or_else(|_| panic!());
    let cursor = runtime
        .subscribe(
            SubscriptionInput(None),
            ResponseRequirement::None,
            NotificationOverflow::Terminate,
            &mut budget(),
        )
        .unwrap_or_else(|_| panic!());
    assert!(
        runtime
            .stop(&cursor, Some(CancellationInput(None)), &mut budget())
            .is_err()
    );
    assert_eq!(
        runtime
            .progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap(),
        NotificationState::Closed
    );
    let report = runtime.take_settlement(cursor.identity()).unwrap();
    assert!(!report.native_started);
    assert_eq!(report.remote_cancellation, RemoteCancellation::NotAttempted);
    drop(runtime);
    assert_empty(&bank);
}
