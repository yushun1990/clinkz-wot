//! Review 10 diagnostics at f6773277a1a3f854465fe94f14b60765451e400c.
//!
//! Assertions preserve reviewed defects, not desired behavior. Retire or invert
//! them when the relevant correction lands. See the parent assessment for the
//! temporary Cargo manifest recipe. This is not a workspace member or runtime.
//!
//! The service deliberately changes declared backing without allocating that
//! backing. This tests capture/accounting, not allocator use or memory safety.
//! No native progress, socket acquisition or broker process is required.

use clinkz_wot_binding_http::{HttpActionConsumer, HttpConsumerConfig};
use clinkz_wot_binding_mqtt::{MqttConsumerConfig, MqttConsumerService};
use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, BindingRegistrationIdentity, CoreResult,
    Operation, Payload, ThingSlotId,
    consumer::{
        ConsumerAccessService, ConsumerDataPolicy, ConsumerDataService, ConsumerInstance,
        ConsumerInstanceRef, ConsumerServices,
    },
    data_schema::PreparedSchema,
    host_accounts::HostResourceAccounts,
    notification::NotificationKind,
};
use clinkz_wot_foundation::{
    GatewayDefaultV1, Generation, ResourceKind, SlotIndex, StaticResourceProfile, WorkBudget,
    WorkClass,
};
use clinkz_wot_planning::open_consumer::{
    ConsumerActionRegistration, ConsumerActionRegistry, ConsumerActionSelectors,
    ConsumerNotificationRegistration, ConsumerNotificationRegistry, ConsumerSelector,
};
use clinkz_wot_servient_finite::{
    consumer_action::ConsumerActionRuntime, consumer_notification::ConsumerNotificationRuntime,
    notification::RawNotificationLimits,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

struct Data {
    base: Arc<dyn ConsumerDataService>,
    added: AtomicU64,
}
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        self.base.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut policy = self.base.policy();
        policy.retained_bytes += self.added.load(Ordering::Relaxed);
        policy
    }
    fn supports_schema(&self, schema: &PreparedSchema) -> bool {
        self.base.supports_schema(schema)
    }
    fn validate(&self, schema: &PreparedSchema, payload: &Payload) -> CoreResult<()> {
        self.base.validate(schema, payload)
    }
}

struct Allow;
impl ConsumerAccessService for Allow {
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

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}

fn assert_released(bank: &HostResourceAccounts) {
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}

fn main() {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let base = config.consumer_services();
    let data = Arc::new(Data {
        base: base.data,
        added: AtomicU64::new(0),
    });
    let services = ConsumerServices {
        data: data.clone(),
        access: base.access,
    };
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let registry = ConsumerActionRegistry::try_new(vec![ConsumerActionRegistration {
        preparation: Box::new(HttpActionConsumer::new(
            config,
            BindingId::new(601),
            BindingGeneration::INITIAL,
            0,
        )),
        services: services.clone(),
    }])
    .unwrap();
    let thing = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"capture probe",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"synchronous":false,"input":{"type":"integer"},"output":{"type":"integer"},
            "forms":[{"href":"http://127.0.0.1:9/run","op":"invokeaction","htv:methodName":"POST"}]}}
    }))
    .unwrap();
    let definition = registry
        .prepare_action(
            &thing,
            "run",
            ConsumerActionSelectors::default(),
            &limits,
            bank.clone(),
            &mut work(),
        )
        .unwrap();
    let prepared_charge = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let before = data.policy().retained_bytes;
    data.added.store(123_456, Ordering::Relaxed);
    let owner = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
    let runtime = ConsumerActionRuntime::try_new(owner.reference(), vec![definition], 1);
    let total_charge = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(
        runtime.is_ok(),
        "reviewed defect: changed declaration is accepted"
    );
    assert!(total_charge < data.policy().retained_bytes);
    println!(
        "Action: prepared service bytes={before}, new service bytes={}, prepared total charge={prepared_charge}, runtime accepted={}, runtime extra charge={}",
        data.policy().retained_bytes,
        runtime.is_ok(),
        total_charge - prepared_charge
    );
    drop(runtime);
    assert_released(&bank);

    // The exact same service object serves invoke/query/cancel in both builds.
    let full = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"same service, three maps",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"synchronous":false,"input":{"type":"integer"},"output":{"type":"integer"},
        "uriVariables":{"id":{"type":"string"}},"forms":[
            {"href":"http://127.0.0.1:9/run","op":"invokeaction","htv:methodName":"POST"},
            {"href":"http://127.0.0.1:9/status{?id}","op":"queryaction","htv:methodName":"GET"},
            {"href":"http://127.0.0.1:9/stop{?id}","op":"cancelaction","htv:methodName":"DELETE"}
        ]}}
    }))
    .unwrap();
    let measure = |added| {
        data.added.store(added, Ordering::Relaxed);
        let bank = Arc::new(HostResourceAccounts::new(&limits));
        let definition = registry
            .prepare_action(
                &full,
                "run",
                ConsumerActionSelectors::default(),
                &limits,
                bank.clone(),
                &mut work(),
            )
            .unwrap();
        let cost = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        drop(definition);
        assert_released(&bank);
        cost
    };
    let small = measure(0);
    let large = measure(60_000);
    assert_eq!(
        large - small,
        180_000,
        "reviewed defect: three backing charges"
    );
    println!(
        "Action: one aliased service declares 60000 extra bytes; three-operation charge increases by {} bytes",
        large - small
    );
    drop(registry);

    data.added.store(0, Ordering::Relaxed);
    let address = "127.0.0.1:9".parse().unwrap();
    let native = MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(address, 8192).unwrap(),
        BindingId::new(602),
        BindingGeneration::INITIAL,
    )
    .unwrap();
    let registry = ConsumerNotificationRegistry::try_new(vec![ConsumerNotificationRegistration {
        binding: native.binding(),
        services: ConsumerServices {
            data: data.clone(),
            access: Arc::new(Allow),
        },
    }])
    .unwrap();
    let thing = serde_json::from_value(serde_json::json!({
        "@context":["https://www.w3.org/2022/wot/td/v1.1",{"mqv":"https://www.w3.org/2019/wot/mqtt#"}],"title":"capture probe",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "events":{"alarm":{"data":{"type":"integer"},"forms":[{"href":"mqtt://127.0.0.1:9","mqv:filter":"probe/alarm","mqv:qos":"0","mqv:retain":false}]}}
    }))
    .unwrap();
    let definition = registry
        .prepare_notification(
            &thing,
            &AffordanceTarget::Event(Arc::from("alarm")),
            NotificationKind::Event,
            ConsumerSelector::default(),
            &limits,
            bank.clone(),
            &mut work(),
        )
        .unwrap();
    let prepared_charge = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert_eq!(data.policy().retained_bytes, before);
    data.added.store(123_456, Ordering::Relaxed);
    let runtime = ConsumerNotificationRuntime::try_new(
        owner.reference(),
        definition,
        RawNotificationLimits {
            subscriptions: 1,
            queue_items: 1,
            queue_payload_bytes: 10_000,
            payload_backing_bytes: 4608,
            control_backing_bytes: 4608,
            target_name_bytes: 64,
            engine_bytes: 100_000,
        },
    );
    assert!(
        runtime.is_ok(),
        "reviewed defect: changed declaration is accepted"
    );
    assert!(!native.connected());
    println!(
        "Notification: prepared service bytes={before}, new service bytes={}, prepared total charge={prepared_charge}, runtime accepted={}, native connected={}",
        data.policy().retained_bytes,
        runtime.is_ok(),
        native.connected()
    );
    drop(runtime);
    drop(registry);
    drop(services);
    drop(data);
    drop(native);
    drop(owner);
    assert_released(&bank);
}
