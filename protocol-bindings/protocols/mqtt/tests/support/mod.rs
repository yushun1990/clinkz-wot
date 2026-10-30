#![allow(dead_code)]
pub use clinkz_wot_binding_mqtt::*;
pub use clinkz_wot_core::{event::*, producer::*, *};
pub use clinkz_wot_foundation::*;
pub use clinkz_wot_planning::producer_composition::*;
pub use clinkz_wot_servient::{host_accounts::HostResourceAccounts, producer_composition::*};
pub use clinkz_wot_servient_finite::*;
pub use clinkz_wot_td::ValidatedProducerScope;
pub use std::{
    net::{SocketAddr, TcpListener},
    rc::Rc,
    sync::Arc,
    task::{Context, Waker},
};

pub fn budget() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
pub fn address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
pub fn authority(slot: u32) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL))
        .unwrap()
}
pub fn owner() -> RawProducerServient {
    RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 8,
        behaviors_per_instance: 16,
        target_name_bytes: 64,
    })
    .unwrap()
}
pub fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 0)
}
pub fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
pub fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
}
pub fn limits() -> Arc<ResourceLimits> {
    Arc::new(GatewayDefaultV1::LIMITS.clone())
}
pub fn assert_empty(bank: &HostResourceAccounts) {
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::AdmissionTemporaryBytesGlobalMax,
        ResourceKind::CleanupItemsMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::ThingsGlobalMax,
    ] {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}
pub fn data(bytes: &[u8]) -> EventData {
    EventData(Payload::new(bytes.to_vec(), "application/json"))
}
pub fn td(address: SocketAddr, topic: &str) -> serde_json::Value {
    serde_json::json!({
        "@context":["https://www.w3.org/2022/wot/td/v1.1", {"mqv":"https://www.w3.org/2019/wot/mqtt#"}],
        "title":"Producer Events", "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "events":{"alarm":{"data":{"type":"integer"}, "forms":[{
            "href":format!("mqtt://{address}"), "mqv:filter":topic,
            "mqv:qos":"0", "mqv:retain":false
        }]}}
    })
}
pub fn instance(authority: ProducerInstanceAuthority, text: &str) -> RawProducerInstance {
    RawProducerInstance::new(authority, vec![])
        .with_events(vec![ProducerEventSource::new("alarm")])
        .with_description(Arc::from(text))
}
pub fn prepare(
    instance: &RawProducerInstance,
    providers: &[&dyn ProducerCapability],
) -> CoreResult<PreparedThing> {
    prepare_thing(
        instance.description().unwrap(),
        instance.key(),
        GatewayDefaultV1::LIMITS,
        &mut budget(),
        providers,
        |s, op| match s {
            ValidatedProducerScope::Event(n) => instance.supports_event(n, op),
            _ => false,
        },
    )
}
pub fn input(
    owner: &RawProducerServient,
    authority: ProducerInstanceAuthority,
    provider: &MqttEventProducer,
    service: &MqttEventService,
    text: &str,
    expected: Option<ProducerInstanceKey>,
) -> ComposedProducerInput {
    let instance = instance(authority, text);
    let prepared = prepare(&instance, &[provider]).unwrap();
    let definitions = prepared
        .operations()
        .iter()
        .map(|p| p.definition::<MqttEventDefinition>().unwrap().clone())
        .collect();
    let runtime = service
        .participation(owner, instance.key(), definitions)
        .unwrap();
    ComposedProducerInput {
        instance,
        prepared,
        providers: vec![Rc::new(provider.clone())],
        participants: vec![Box::new(runtime)],
        expected,
        handler_backing_bytes: 0,
    }
}
pub fn stage(
    owner: &RawProducerServient,
    input: ComposedProducerInput,
    bank: &Arc<HostResourceAccounts>,
) -> ComposedProducerExposure {
    ComposedProducerExposure::try_new(owner, input, limits(), Arc::clone(bank), &mut budget())
        .unwrap_or_else(|r| panic!("admission: {:?}", r.error().error()))
}
pub fn native(exposure: &mut ComposedProducerExposure) -> &mut MqttEventRuntime {
    exposure.participant_mut::<MqttEventRuntime>(0).unwrap()
}
pub fn emit(
    owner: &mut RawProducerServient,
    exposure: &mut ComposedProducerExposure,
    bytes: &[u8],
) -> ProducerEventEmission<MqttPublication> {
    let key = exposure.key();
    owner
        .emit_event(
            native(exposure),
            key,
            "alarm",
            data(bytes),
            Deadline::NONE,
            now(),
            &mut budget(),
        )
        .unwrap_or_else(|r| panic!("emission: {:?}", r.error().error()))
}
