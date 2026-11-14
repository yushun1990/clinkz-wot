#![cfg(feature = "prepare")]
//! Independent association/capacity and original service-owner evidence.
//! MQTT occurrence coordinates below carry no authenticated publisher identity.
#[cfg(feature = "broker-tests")]
#[path = "support/broker.rs"]
mod broker;
mod support;
use clinkz_wot_core::{consumer::*, notification::*};
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::{consumer_notification::*, notification::*};
use std::{
    cell::Cell,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};
use support::*;

#[derive(Clone)]
struct Counted {
    binding: MqttNotificationConsumer,
    lowerings: Rc<Cell<usize>>,
    changed_support: Rc<Cell<bool>>,
}
impl ConsumerNotificationBinding for Counted {
    type Program = <MqttNotificationConsumer as ConsumerNotificationBinding>::Program;
    type Configuration = MqttConsumerConfig;
    type Participation = <MqttNotificationConsumer as ConsumerNotificationBinding>::Participation;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.binding.identity()
    }
    fn configuration(&self) -> &Self::Configuration {
        self.binding.configuration()
    }
    fn support(&self, p: &Self::Program, k: NotificationKind) -> CoreResult<NotificationSupport> {
        let mut support = self.binding.support(p, k)?;
        if self.changed_support.get() {
            support.cancellation = !support.cancellation;
        }
        Ok(support)
    }
    fn prepare(
        &mut self,
        p: Self::Program,
        id: &NotificationIdentity,
    ) -> Result<Self::Participation, BindingInputRejection<Self::Program>> {
        self.binding.prepare(p, id)
    }
}
impl ConsumerNotificationPreparation for Counted {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        self.binding.registration_identity()
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        self.binding.preparation_bounds()
    }
    fn compile_notification(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
        self.lowerings.set(self.lowerings.get() + 1);
        self.binding.compile_notification(input)
    }
}
struct Data {
    policy: Mutex<ConsumerDataPolicy>,
    available: AtomicBool,
    supported: AtomicBool,
    validations: AtomicUsize,
    _backing: Vec<u8>,
}
impl Data {
    fn new(bytes: usize) -> Self {
        Self {
            policy: Mutex::new(ConsumerDataPolicy {
                retained_bytes: bytes as u64,
                ..MqttJsonData.policy()
            }),
            available: AtomicBool::new(true),
            supported: AtomicBool::new(true),
            validations: AtomicUsize::new(0),
            _backing: vec![0; bytes],
        }
    }
}
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn policy(&self) -> ConsumerDataPolicy {
        *self.policy.lock().unwrap()
    }
    fn available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
    fn supports_schema(&self, _: &data_schema::PreparedSchema) -> bool {
        self.supported.load(Ordering::Relaxed)
    }
    fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.validations.fetch_add(1, Ordering::Relaxed);
        MqttJsonData.validate(s, p)
    }
}
struct Access {
    work: AtomicU64,
    bytes: AtomicU64,
    revision: AtomicU64,
    deny_alarm: AtomicBool,
    deny_cancel: AtomicBool,
    expected: Mutex<Option<ConsumerInstanceRef>>,
    alarm_seen: AtomicBool,
    ready_seen: AtomicBool,
}
impl Default for Access {
    fn default() -> Self {
        Self {
            work: AtomicU64::new(1),
            bytes: AtomicU64::new(256),
            revision: AtomicU64::new(0),
            deny_alarm: AtomicBool::new(false),
            deny_cancel: AtomicBool::new(false),
            expected: Mutex::new(None),
            alarm_seen: AtomicBool::new(false),
            ready_seen: AtomicBool::new(false),
        }
    }
}
impl ConsumerAccessService for Access {
    fn security_revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }
    fn work(&self) -> u64 {
        self.work.load(Ordering::Relaxed)
    }
    fn retained_bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }
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
    fn authorize_notification(
        &self,
        _: BindingRegistrationIdentity,
        _: &str,
        r: NotificationAuthorization<'_>,
    ) -> CoreResult<()> {
        assert_eq!(r.target, &AffordanceTarget::Thing);
        if let Some(AffordanceTarget::Event(name)) = r.occurrence {
            if let Some(expected) = &*self.expected.lock().unwrap() {
                assert_eq!(&r.recipient.consumer, expected);
            }
            match &**name {
                "alarm" => self.alarm_seen.store(true, Ordering::Relaxed),
                "ready" => self.ready_seen.store(true, Ordering::Relaxed),
                _ => panic!("undeclared occurrence reached policy"),
            }
            if &**name == "alarm" && self.deny_alarm.load(Ordering::Relaxed) {
                return Err(denied());
            }
        }
        if r.permission == NotificationPermission::Cancel
            && self.deny_cancel.load(Ordering::Relaxed)
        {
            return Err(denied());
        }
        Ok(())
    }
}
fn denied() -> CoreError {
    CoreError::Security {
        reason: SecurityFailureReason::AuthorizationDenied,
        context: ErrorContext::new(ErrorPhase::Delivery, RetryClass::Never),
    }
}
fn retained(
    data: Arc<dyn ConsumerDataService>,
    access: Arc<dyn ConsumerAccessService>,
    bank: &Arc<HostResourceAccounts>,
) -> RetainedConsumerServices {
    RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(data, bank).unwrap_or_else(|_| panic!()),
        access: ConsumerServiceOwner::retain_access(access, bank).unwrap_or_else(|_| panic!()),
    }
}
fn binding(s: &MqttConsumerService) -> Counted {
    Counted {
        binding: s.binding(),
        lowerings: Rc::new(Cell::new(0)),
        changed_support: Rc::new(Cell::new(false)),
    }
}
fn service(address: SocketAddr) -> MqttConsumerService {
    MqttConsumerService::new(
        MqttConsumerConfig::for_loopback(address, 1).unwrap(),
        BindingId::new(8),
        BindingGeneration::INITIAL,
    )
    .unwrap()
}
fn owner(slot: u32) -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL))
}
fn description(address: SocketAddr) -> clinkz_wot_td::thing::Thing {
    serde_json::from_value(event_collection_td(address, "composition/events")).unwrap()
}
fn definition(
    b: &Counted,
    services: RetainedConsumerServices,
    map_bank: &Arc<HostResourceAccounts>,
    address: SocketAddr,
) -> Rc<ConsumerNotificationDefinition<Counted>> {
    let r = ConsumerNotificationRegistry::try_from_retained(vec![
        RetainedConsumerNotificationRegistration {
            binding: b.clone(),
            services,
        },
    ])
    .unwrap();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    Rc::new(
        r.prepare_notification(
            &description(address),
            &AffordanceTarget::Thing,
            NotificationKind::SubscribeAllEvents,
            ConsumerSelector::default(),
            &limits,
            map_bank.clone(),
            &mut budget(),
        )
        .unwrap(),
    )
}
fn associate(
    consumer: &ConsumerInstance,
    def: Rc<ConsumerNotificationDefinition<Counted>>,
    access: Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>,
    instance_bank: &Arc<HostResourceAccounts>,
) -> ConsumerNotificationAssociation<Counted> {
    ConsumerNotificationAssociation::try_new(
        consumer.reference(),
        def,
        access,
        instance_bank.clone(),
        &mut budget(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn limits(slots: usize) -> RawNotificationLimits {
    RawNotificationLimits {
        subscriptions: slots,
        queue_items: 2,
        queue_payload_bytes: 12_000,
        payload_backing_bytes: 4608,
        control_backing_bytes: 0,
        target_name_bytes: 64,
        engine_bytes: 128_000,
    }
}
fn subscribe(
    work: &mut ConsumerNotifications<Counted>,
    a: &ConsumerNotificationAssociation<Counted>,
) -> Result<NotificationCursor, BindingInputRejection<SubscriptionInput>> {
    work.subscribe(
        a,
        SubscriptionInput(None),
        ResponseRequirement::None,
        NotificationOverflow::DropNewest,
        &mut budget(),
    )
}

#[test]
fn original_services_are_charged_once_across_definitions_and_banks() {
    let address = address();
    let s = service(address);
    let b = binding(&s);
    let original = bank();
    let maps = bank();
    let instances = bank();
    let capacity = bank();
    let data = Arc::new(Data::new(60_000));
    let access = Arc::new(Access::default());
    let services = retained(data.clone(), access, &original);
    let charged = original
        .account(ResourceKind::EngineLiveBytesGlobalMax)
        .used();
    assert!(charged >= 60_000);
    let d1 = definition(&b, services.clone(), &maps, address);
    let first = maps.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let d2 = definition(&b, services.clone(), &maps, address);
    assert_eq!(
        maps.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        first * 2
    );
    assert!(
        first < 60_000,
        "physical service backing must not be charged per mapping"
    );
    assert_eq!(
        original
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        charged
    );
    let consumer = owner(1);
    let a = associate(&consumer, d1, services.access.clone(), &instances);
    let mut work = ConsumerNotifications::try_new(limits(1), capacity.clone()).unwrap();
    let cursor = subscribe(&mut work, &a).unwrap_or_else(|_| panic!());
    drop((services, d2, a));
    assert_eq!(
        original
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        charged
    );
    drop(cursor);
    let report = work
        .step_cleanup(&mut cx(), &mut budget())
        .unwrap()
        .unwrap();
    assert!(!report.native_started);
    drop(work);
    assert_empty(&capacity);
    assert!(
        original
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used()
            > 0,
        "retained report keeps original prepared owners"
    );
    drop((report, data));
    assert_empty(&original);
    assert_empty(&maps);
    assert_empty(&instances);
    assert!(!s.connected());
}

#[test]
fn prepared_data_and_native_declarations_cannot_be_refreshed_by_binding_or_admission() {
    let address = address();
    let s = service(address);
    let b = binding(&s);
    let original = bank();
    let maps = bank();
    let instances = bank();
    let capacity = bank();
    let data = Arc::new(Data::new(32));
    let access = Arc::new(Access::default());
    let services = retained(data.clone(), access.clone(), &original);
    let def = definition(&b, services.clone(), &maps, address);
    let original_policy = data.policy();
    let consumer = owner(2);
    let a = associate(&consumer, def.clone(), services.access.clone(), &instances);
    let mut work = ConsumerNotifications::try_new(limits(1), capacity.clone()).unwrap();
    let unchanged = ResourceKind::ALL.map(|k| capacity.account(k).used());
    for field in 0..5 {
        let mut changed = original_policy;
        match field {
            0 => changed.retained_bytes += 123_456,
            1 => changed.temporary_bytes += 1,
            2 => changed.body_bytes += 1,
            3 => changed.validation_work += 1,
            _ => changed.encoded_selection_bytes += 1,
        }
        *data.policy.lock().unwrap() = changed;
        assert!(def.check_dependencies().is_err());
        assert!(
            ConsumerNotificationAssociation::try_new(
                consumer.reference(),
                def.clone(),
                services.access.clone(),
                instances.clone(),
                &mut budget()
            )
            .is_err()
        );
        assert!(subscribe(&mut work, &a).is_err());
        assert_eq!(
            ResourceKind::ALL.map(|k| capacity.account(k).used()),
            unchanged
        );
        *data.policy.lock().unwrap() = original_policy;
    }
    b.changed_support.set(true);
    assert!(subscribe(&mut work, &a).is_err());
    b.changed_support.set(false);
    data.available.store(false, Ordering::Relaxed);
    assert!(subscribe(&mut work, &a).is_err());
    data.available.store(true, Ordering::Relaxed);
    data.supported.store(false, Ordering::Relaxed);
    assert!(subscribe(&mut work, &a).is_err());
    data.supported.store(true, Ordering::Relaxed);
    for field in 0..2 {
        match field {
            0 => access.work.store(2, Ordering::Relaxed),
            _ => access.bytes.store(257, Ordering::Relaxed),
        }
        assert!(subscribe(&mut work, &a).is_err());
        access.work.store(1, Ordering::Relaxed);
        access.bytes.store(256, Ordering::Relaxed);
    }
    let cursor = subscribe(&mut work, &a).unwrap_or_else(|_| panic!());
    data.policy.lock().unwrap().validation_work += 1;
    assert!(
        work.progress(cursor.identity(), &mut cx(), &mut budget())
            .is_err()
    );
    *data.policy.lock().unwrap() = original_policy;
    assert_eq!(
        work.status(cursor.identity()).unwrap().state,
        NotificationState::Closing
    );
    assert_eq!(
        work.progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap(),
        NotificationState::Closed
    );
    let report = work.take_settlement(cursor.identity()).unwrap();
    assert!(!report.native_started);
    drop(report);
    // A live security dependency revision is never reused by this service.
    access.revision.store(1, Ordering::Relaxed);
    assert!(subscribe(&mut work, &a).is_err());
    assert_eq!(data.validations.load(Ordering::Relaxed), 0);
    assert!(!s.connected());
    assert_eq!(b.lowerings.get(), 1);
    drop((work, a, def, services));
    assert_empty(&original);
    assert_empty(&maps);
    assert_empty(&instances);
    assert_empty(&capacity);
}

#[cfg(feature = "broker-tests")]
async fn drive(
    work: &mut ConsumerNotifications<Counted>,
    cursor: &NotificationCursor,
    active: bool,
) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let state = work
            .progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap();
        if (active && state == NotificationState::Active)
            || (!active && state == NotificationState::Closed)
        {
            return;
        }
        assert!(std::time::Instant::now() < end, "{state:?}");
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
#[cfg(feature = "broker-tests")]
async fn queued(
    work: &mut ConsumerNotifications<Counted>,
    cursor: &NotificationCursor,
    count: usize,
) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while work.status(cursor.identity()).unwrap().queued_items < count {
        work.progress(cursor.identity(), &mut cx(), &mut budget())
            .unwrap();
        assert!(std::time::Instant::now() < end);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
#[cfg(feature = "broker-tests")]
#[test]
fn one_compilation_many_associations_two_slots_and_one_native_interest_keep_independent_rights() {
    broker::runtime().block_on(async {
        let broker = broker::Broker::start();
        let service = service(broker.address);
        let b = binding(&service);
        let original = bank();
        let maps = bank();
        let capacity = bank();
        let data = Arc::new(Data::new(60_000));
        let first_access = Arc::new(Access::default());
        let second_access = Arc::new(Access::default());
        let services = retained(data, first_access.clone(), &original);
        let second = ConsumerServiceOwner::retain_access(second_access.clone(), &original)
            .unwrap_or_else(|_| panic!());
        let def = definition(&b, services.clone(), &maps, broker.address);
        let consumers: Vec<_> = (0..8).map(owner).collect();
        *second_access.expected.lock().unwrap() = Some(consumers[1].reference());
        let banks: Vec<_> = (0..8)
            .map(|_| {
                let mut l = GatewayDefaultV1::LIMITS.clone();
                assert!(l.set(ResourceKind::SubscriptionsPerThingMax, Some(1)));
                Arc::new(HostResourceAccounts::new(&l))
            })
            .collect();
        let associations: Vec<_> = consumers
            .iter()
            .enumerate()
            .map(|(i, c)| {
                associate(
                    c,
                    def.clone(),
                    if i == 1 {
                        second.clone()
                    } else {
                        services.access.clone()
                    },
                    &banks[i],
                )
            })
            .collect();
        assert_eq!(b.lowerings.get(), 1);
        assert!(!service.connected());
        for bank in &banks {
            assert_eq!(
                bank.account(ResourceKind::SubscriptionsPerThingMax).used(),
                0
            );
            assert_eq!(
                bank.account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                    .used(),
                0
            );
        }
        let mut work = ConsumerNotifications::try_new(limits(2), capacity.clone()).unwrap();
        let mut a = subscribe(&mut work, &associations[0]).unwrap_or_else(|_| panic!());
        assert!(
            subscribe(&mut work, &associations[0]).is_err(),
            "per-instance quota with global capacity still available"
        );
        assert_eq!(work.retained_subscriptions(), 1);
        let mut z = subscribe(&mut work, &associations[1]).unwrap_or_else(|_| panic!());
        assert!(subscribe(&mut work, &associations[2]).is_err());
        assert!(
            subscribe(&mut work, &associations[0]).is_err(),
            "per-instance outstanding quota"
        );
        drive(&mut work, &a, true).await;
        drive(&mut work, &z, true).await;
        assert_eq!(
            broker.logs().matches("Received SUBSCRIBE from cr").count(),
            1
        );
        let mut peer = broker::Peer::new(&broker);
        peer.publish_raw("composition/events/alarm", b"41", false);
        queued(&mut work, &a, 1).await;
        queued(&mut work, &z, 1).await;
        first_access.deny_alarm.store(true, Ordering::Relaxed);
        assert!(
            work.front(&mut a, &mut budget()).is_err(),
            "fresh queued occurrence policy"
        );
        let view = work.front(&mut z, &mut budget()).unwrap().unwrap();
        assert_eq!(
            view.target,
            clinkz_wot_core::thing::AffordanceSite::Event("alarm")
        );
        assert_eq!(view.data.unwrap().body, b"41");
        work.advance(&mut z, &mut budget()).unwrap();
        drive(&mut work, &a, false).await;
        assert!(
            subscribe(&mut work, &associations[2]).is_err(),
            "unacknowledged closed work retains slot"
        );
        let report = work.take_settlement(a.identity()).unwrap();
        assert!(matches!(
            report.first_cause,
            Some(CoreError::Security { .. })
        ));
        drop(report);
        let third = subscribe(&mut work, &associations[2]).unwrap_or_else(|_| panic!());
        drive(&mut work, &third, true).await;
        // Application view loss grants no revocation and strands no cleanup.
        drop((associations, def, services, second));
        peer.publish_raw("composition/events/ready", b"true", false);
        queued(&mut work, &z, 1).await;
        queued(&mut work, &third, 1).await;
        assert!(work.front(&mut z, &mut budget()).unwrap().is_some());
        second_access.deny_cancel.store(true, Ordering::Relaxed);
        assert!(
            work.stop(&z, Some(CancellationInput(None)), &mut budget())
                .is_err()
        );
        drive(&mut work, &z, false).await;
        drop(work.take_settlement(z.identity()).unwrap());
        assert!(service.connected(), "sibling retains physical lease");
        consumers[2].revoke();
        assert!(work.front(&mut { third }, &mut budget()).is_err());
        // Cleanup remains reachable after every public cursor is lost.
        drop((a, z));
        let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while work.retained_subscriptions() > 0 {
            drop(work.step_cleanup(&mut cx(), &mut budget()).unwrap());
            assert!(std::time::Instant::now() < end);
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert!(!service.connected());
        assert_eq!(b.lowerings.get(), 1);
        assert_eq!(
            broker.logs().matches("Received SUBSCRIBE from cr").count(),
            1
        );
        assert_eq!(
            broker
                .logs()
                .matches("Received UNSUBSCRIBE from cr")
                .count(),
            1
        );
        assert!(second_access.alarm_seen.load(Ordering::Relaxed));
        assert!(second_access.ready_seen.load(Ordering::Relaxed));
        drop(work);
        assert_empty(&original);
        assert_empty(&maps);
        assert_empty(&capacity);
        for bank in &banks {
            assert_empty(bank);
        }
    });
}
