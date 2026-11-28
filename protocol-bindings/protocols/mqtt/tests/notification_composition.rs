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
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<Self::Program>> {
        self.lowerings.set(self.lowerings.get() + 1);
        self.binding.compile_notification_form(input)
    }
}
struct Data {
    policy: Mutex<ConsumerDataPolicy>,
    available: AtomicBool,
    supported: AtomicBool,
    revision: AtomicU64,
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
            revision: AtomicU64::new(0),
            validations: AtomicUsize::new(0),
            _backing: vec![0; bytes],
        }
    }
}
impl ConsumerDataService for Data {
    fn qualification_revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

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
    deny_subscription: AtomicBool,
    expected: Mutex<Option<ConsumerInstanceRef>>,
    alarm_seen: AtomicBool,
    ready_seen: AtomicBool,
    missing_occurrences: AtomicUsize,
}
impl Default for Access {
    fn default() -> Self {
        Self {
            work: AtomicU64::new(1),
            bytes: AtomicU64::new(256),
            revision: AtomicU64::new(0),
            deny_alarm: AtomicBool::new(false),
            deny_cancel: AtomicBool::new(false),
            deny_subscription: AtomicBool::new(false),
            expected: Mutex::new(None),
            alarm_seen: AtomicBool::new(false),
            ready_seen: AtomicBool::new(false),
            missing_occurrences: AtomicUsize::new(0),
        }
    }
}
impl ConsumerAccessService for Access {
    fn qualification_revision(&self) -> u64 {
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
    ) -> CoreResult<NotificationDecision> {
        assert_eq!(r.target, &AffordanceTarget::Thing);
        if self.deny_subscription.load(Ordering::Relaxed) {
            return Err(denied());
        }
        if matches!(
            r.permission,
            NotificationPermission::Deliver | NotificationPermission::Respond
        ) && r.occurrence.is_none()
        {
            self.missing_occurrences.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(AffordanceTarget::Event(name)) = r.occurrence {
            if let Some(expected) = &*self.expected.lock().unwrap() {
                assert_eq!(r.recipient.consumer(), expected);
            }
            match &**name {
                "alarm" => self.alarm_seen.store(true, Ordering::Relaxed),
                "ready" => self.ready_seen.store(true, Ordering::Relaxed),
                _ => panic!("undeclared occurrence reached policy"),
            }
            if &**name == "alarm" && self.deny_alarm.load(Ordering::Relaxed) {
                return Ok(NotificationDecision::RejectOccurrence);
            }
        }
        if r.permission == NotificationPermission::Cancel
            && self.deny_cancel.load(Ordering::Relaxed)
        {
            return Err(denied());
        }
        Ok(NotificationDecision::Allow)
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
) -> ConsumerNotificationAssociation<Counted> {
    ConsumerNotificationAssociation::try_new(consumer.reference(), def, access)
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
        reports: 8,
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
    )
}

#[test]
fn original_services_are_charged_once_across_definitions_and_banks() {
    let address = address();
    let s = service(address);
    let b = binding(&s);
    let original = bank();
    let maps = bank();
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
    let a = associate(&consumer, d1, services.access.clone());
    let mut work = ConsumerNotifications::try_new(limits(1)).unwrap();
    let cursor = subscribe(&mut work, &a).unwrap_or_else(|_| panic!());
    drop((services, d2, a));
    assert_eq!(
        original
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        charged
    );
    drop(cursor);
    let report = work.step_cleanup(&mut cx()).unwrap().unwrap();
    assert!(!report.native_started);
    drop(work);
    assert_empty(&original);
    assert_empty(&maps);
    assert_eq!(
        report.target(),
        clinkz_wot_core::thing::AffordanceSite::Thing
    );
    assert!(!s.connected());
}

#[test]
fn prepared_data_and_native_declarations_cannot_be_refreshed_by_binding_or_admission() {
    let address = address();
    let s = service(address);
    let b = binding(&s);
    let original = bank();
    let maps = bank();
    let data = Arc::new(Data::new(32));
    let access = Arc::new(Access::default());
    let services = retained(data.clone(), access.clone(), &original);
    let def = definition(&b, services.clone(), &maps, address);
    let original_policy = data.policy();
    let consumer = owner(2);
    let a = associate(&consumer, def.clone(), services.access.clone());
    let mut work = ConsumerNotifications::try_new(limits(1)).unwrap();
    for field in 0..3 {
        let mut changed = original_policy;
        match field {
            0 => changed.retained_bytes += 123_456,
            1 => changed.temporary_bytes += 1,
            _ => changed.body_bytes += 1,
        }
        *data.policy.lock().unwrap() = changed;
        assert!(def.check_dependencies().is_err());
        assert!(
            ConsumerNotificationAssociation::try_new(
                consumer.reference(),
                def.clone(),
                services.access.clone()
            )
            .is_err()
        );
        assert!(subscribe(&mut work, &a).is_err());
        assert_eq!(work.retained_subscriptions(), 0);
        *data.policy.lock().unwrap() = original_policy;
    }
    b.changed_support.set(true);
    assert!(subscribe(&mut work, &a).is_err());
    b.changed_support.set(false);
    data.available.store(false, Ordering::Relaxed);
    assert!(subscribe(&mut work, &a).is_err());
    data.available.store(true, Ordering::Relaxed);
    access.bytes.store(257, Ordering::Relaxed);
    assert!(subscribe(&mut work, &a).is_err());
    access.bytes.store(256, Ordering::Relaxed);
    let cursor = subscribe(&mut work, &a).unwrap_or_else(|_| panic!());
    data.policy.lock().unwrap().body_bytes -= 1;
    assert!(work.progress(cursor.identity(), &mut cx()).is_err());
    *data.policy.lock().unwrap() = original_policy;
    assert_eq!(
        work.status(cursor.identity()).unwrap().state,
        NotificationState::Closing
    );
    assert_eq!(
        work.progress(cursor.identity(), &mut cx()).unwrap(),
        NotificationTurn::Closed
    );
    let report = work.take_settlement(cursor.identity()).unwrap();
    assert!(!report.native_started);
    drop(report);
    // A live security dependency revision is never reused by this service.
    access.revision.store(1, Ordering::Relaxed);
    assert!(subscribe(&mut work, &a).is_err());
    assert_eq!(data.validations.load(Ordering::Relaxed), 0);
    assert!(!s.connected());
    data.revision.fetch_add(1, Ordering::Relaxed);
    assert!(def.check_dependencies().is_err());
    assert_eq!(b.lowerings.get(), 1);
    drop((work, a, def, services));
    assert_empty(&original);
    assert_empty(&maps);
}

#[cfg(feature = "broker-tests")]
async fn drive(
    work: &mut ConsumerNotifications<Counted>,
    cursor: &NotificationCursor,
    active: bool,
) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        work.progress(cursor.identity(), &mut cx()).unwrap();
        let state = work.status(cursor.identity()).unwrap().state;
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
        work.progress(cursor.identity(), &mut cx()).unwrap();
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
        let data = Arc::new(Data::new(60_000));
        let first_access = Arc::new(Access::default());
        let second_access = Arc::new(Access::default());
        let services = retained(data, first_access.clone(), &original);
        let second = ConsumerServiceOwner::retain_access(second_access.clone(), &original)
            .unwrap_or_else(|_| panic!());
        let def = definition(&b, services.clone(), &maps, broker.address);
        let consumers: Vec<_> = (0..8).map(owner).collect();
        *second_access.expected.lock().unwrap() = Some(consumers[1].reference());
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
                )
            })
            .collect();
        assert_eq!(b.lowerings.get(), 1);
        assert!(!service.connected());
        let mut work = ConsumerNotifications::try_new(RawNotificationLimits {
            reports: 2,
            ..limits(2)
        })
        .unwrap();
        let mut a = subscribe(&mut work, &associations[0]).unwrap_or_else(|_| panic!());
        let mut z = subscribe(&mut work, &associations[1]).unwrap_or_else(|_| panic!());
        assert!(subscribe(&mut work, &associations[2]).is_err());
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
        first_access
            .deny_subscription
            .store(true, Ordering::Relaxed);
        assert!(
            work.front(&mut a).is_err(),
            "fresh queued occurrence policy"
        );
        let view = work.front(&mut z).unwrap().unwrap();
        assert_eq!(
            view.target,
            clinkz_wot_core::thing::AffordanceSite::Event("alarm")
        );
        assert_eq!(view.data.unwrap().body, b"41");
        work.advance(&mut z).unwrap();
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
        assert_eq!(work.retained_subscriptions(), 1);
        assert!(
            subscribe(&mut work, &associations[2]).is_err(),
            "held report owns the last report slot"
        );
        drop(report);
        first_access
            .deny_subscription
            .store(false, Ordering::Relaxed);
        let third = subscribe(&mut work, &associations[2]).unwrap_or_else(|_| panic!());
        drive(&mut work, &third, true).await;
        // Application view loss grants no revocation and strands no cleanup.
        drop((associations, def, services, second));
        peer.publish_raw("composition/events/ready", b"true", false);
        queued(&mut work, &z, 1).await;
        queued(&mut work, &third, 1).await;
        assert!(work.front(&mut z).unwrap().is_some());
        second_access.deny_cancel.store(true, Ordering::Relaxed);
        assert!(work.stop(&z, Some(CancellationInput(None))).is_err());
        drive(&mut work, &z, false).await;
        drop(work.take_settlement(z.identity()).unwrap());
        assert!(service.connected(), "sibling retains physical lease");
        consumers[2].revoke();
        assert!(work.front(&mut { third }).is_err());
        // Cleanup remains reachable after every public cursor is lost.
        drop((a, z));
        let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while work.retained_subscriptions() > 0 {
            drop(work.step_cleanup(&mut cx()).unwrap());
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
    });
}

#[cfg(feature = "broker-tests")]
#[test]
fn held_thing_report_allows_preparation_at_the_original_peak() {
    broker::runtime().block_on(async {
        let broker = broker::Broker::start();
        let service = service(broker.address);
        let b = binding(&service);
        let preparation = |bank: &Arc<HostResourceAccounts>, access: Arc<Access>| {
            let services = retained(Arc::new(Data::new(32)), access, bank);
            let mapping = definition(&b, services.clone(), bank, broker.address);
            (mapping, services)
        };
        let measurement = bank();
        drop(preparation(&measurement, Arc::new(Access::default())));
        let kind = ResourceKind::EngineLiveBytesGlobalMax;
        let peak = measurement.account(kind).peak();
        assert!(peak > 0);
        let mut budget_limits = GatewayDefaultV1::LIMITS.clone();
        assert!(budget_limits.set(kind, Some(peak)));
        let bank = Arc::new(HostResourceAccounts::new(&budget_limits));

        let access = Arc::new(Access::default());
        let original_access = Arc::downgrade(&access);
        let (mapping, services) = preparation(&bank, access);
        let instance = owner(1);
        let association = associate(&instance, mapping, services.access.clone());
        let mut work = ConsumerNotifications::try_new(limits(1)).unwrap();
        let cursor = subscribe(&mut work, &association).unwrap_or_else(|_| panic!());
        drive(&mut work, &cursor, true).await;
        work.stop(&cursor, Some(CancellationInput(None)))
            .unwrap_or_else(|_| panic!());
        drive(&mut work, &cursor, false).await;
        let report = work.take_settlement(cursor.identity()).unwrap();
        drop((cursor, instance, association, services));
        assert_eq!(work.retained_subscriptions(), 0);
        assert!(!service.connected());

        let successor = preparation(&bank, Arc::new(Access::default()));
        assert!(original_access.upgrade().is_none());
        assert_eq!(
            report.target(),
            clinkz_wot_core::thing::AffordanceSite::Thing
        );
        assert_eq!(report.remote_cancellation, RemoteCancellation::Confirmed);
        drop(successor);
        assert_empty(&bank);
    });
}

#[cfg(feature = "broker-tests")]
#[test]
fn native_lease_limit_refuses_a_spare_conversation_until_original_close() {
    broker::runtime().block_on(async {
        let broker = broker::Broker::start();
        let service = service(broker.address);
        let original = bank();
        let services = retained(
            Arc::new(Data::new(32)),
            Arc::new(Access::default()),
            &original,
        );
        let def = definition(
            &binding(&service),
            services.clone(),
            &original,
            broker.address,
        );
        let instance = owner(1);
        let association = associate(&instance, def, services.access.clone());
        let mut work = ConsumerNotifications::try_new(RawNotificationLimits {
            reports: 32,
            ..limits(32)
        })
        .unwrap();
        let original_cursor = subscribe(&mut work, &association).unwrap_or_else(|_| panic!());
        drive(&mut work, &original_cursor, true).await;
        // Prepared native interests already own their slots before any poll starts them.
        let provisional = (1..32)
            .map(|_| subscribe(&mut work, &association).unwrap_or_else(|_| panic!()))
            .collect::<Vec<_>>();
        let mut contender = ConsumerNotifications::try_new(RawNotificationLimits {
            reports: 1,
            ..limits(1)
        })
        .unwrap();
        let refused = subscribe(&mut contender, &association).err().unwrap();
        assert!(matches!(
            refused.error().error(),
            CoreError::Backpressure(_)
        ));
        assert_eq!(contender.retained_subscriptions(), 0);

        original_cursor.stop_handle().request_stop();
        drive(&mut work, &original_cursor, false).await;
        let original_report = work.take_settlement(original_cursor.identity()).unwrap();
        let successor = subscribe(&mut contender, &association).unwrap_or_else(|_| panic!());
        drive(&mut contender, &successor, true).await;
        assert_ne!(original_report.identity, *successor.identity());

        drop((provisional, work, contender, association, services));
        assert!(!service.connected());
        drop(original_report);
        assert_empty(&original);
    });
}

#[cfg(feature = "broker-tests")]
fn rejected_alarm_then_ready_on_original_conversation(queued_change: bool) {
    broker::runtime().block_on(async {
        let broker = broker::Broker::start();
        let service = service(broker.address);
        let b = binding(&service);
        let original = bank();
        let maps = bank();
        let policy = Arc::new(Access::default());
        policy.deny_alarm.store(!queued_change, Ordering::Relaxed);
        let services = retained(Arc::new(Data::new(32)), policy.clone(), &original);
        let def = definition(&b, services.clone(), &maps, broker.address);
        let consumers = [owner(1), owner(2)];
        let sibling = ConsumerServiceOwner::retain_access(Arc::new(Access::default()), &original)
            .unwrap_or_else(|_| panic!());
        let associations = [
            associate(&consumers[0], def.clone(), services.access.clone()),
            associate(&consumers[1], def.clone(), sibling),
        ];
        let mut work = ConsumerNotifications::try_new(limits(2)).unwrap();
        let mut a = subscribe(&mut work, &associations[0]).unwrap_or_else(|_| panic!());
        let mut z = subscribe(&mut work, &associations[1]).unwrap_or_else(|_| panic!());
        let original_id = a.identity().clone();
        drive(&mut work, &a, true).await;
        drive(&mut work, &z, true).await;
        if !queued_change {
            work.progress(a.identity(), &mut cx()).unwrap();
            assert_eq!(
                policy.missing_occurrences.load(Ordering::Relaxed),
                0,
                "native polling must not require a Deliver decision without an occurrence"
            );
            assert!(work.front(&mut a).unwrap().is_none());
        }
        let mut peer = broker::Peer::new(&broker);
        peer.publish_raw("composition/events/alarm", b"41", false);
        peer.publish_raw("composition/events/ready", b"true", false);
        queued(&mut work, &a, if queued_change { 2 } else { 1 }).await;
        queued(&mut work, &z, 2).await;
        if queued_change {
            policy.deny_alarm.store(true, Ordering::Relaxed);
            assert!(
                work.front(&mut a).unwrap().is_none(),
                "deny the original queued alarm without closing the conversation"
            );
            // Once refused, changing policy back cannot resurrect this item.
            policy.deny_alarm.store(false, Ordering::Relaxed);
            assert!(work.front(&mut a).unwrap().is_none());
            work.progress(a.identity(), &mut cx()).unwrap();
        }
        let view = work.front(&mut a).unwrap().unwrap();
        assert_eq!(
            view.target,
            clinkz_wot_core::thing::AffordanceSite::Event("ready")
        );
        assert_eq!(view.data.unwrap().body, b"true");
        work.advance(&mut a).unwrap();
        assert_eq!(a.identity(), &original_id);
        assert_eq!(
            work.status(a.identity()).unwrap().state,
            NotificationState::Active
        );
        assert!(work.status(a.identity()).unwrap().first_cause.is_none());
        assert_eq!(policy.missing_occurrences.load(Ordering::Relaxed), 0);
        for (name, body) in [("alarm", b"41".as_slice()), ("ready", b"true".as_slice())] {
            let view = work.front(&mut z).unwrap().unwrap();
            assert_eq!(
                view.target,
                clinkz_wot_core::thing::AffordanceSite::Event(name)
            );
            assert_eq!(view.data.unwrap().body, body);
            work.advance(&mut z).unwrap();
        }
        assert!(service.connected());
        assert_eq!(
            broker.logs().matches("Received SUBSCRIBE from cr").count(),
            1
        );
        assert_eq!(
            broker
                .logs()
                .matches("Received UNSUBSCRIBE from cr")
                .count(),
            0
        );
        for cursor in [&a, &z] {
            work.stop(cursor, None).unwrap_or_else(|_| panic!());
            drive(&mut work, cursor, false).await;
            drop(work.take_settlement(cursor.identity()).unwrap());
        }
        assert!(!service.connected());
        assert_eq!(
            broker
                .logs()
                .matches("Received UNSUBSCRIBE from cr")
                .count(),
            1
        );
        drop((work, associations, def, services));
        assert_empty(&original);
        assert_empty(&maps);
    });
}
#[cfg(feature = "broker-tests")]
#[test]
fn native_poll_has_no_occurrence_delivery_preflight_and_denied_alarm_allows_next_ready() {
    rejected_alarm_then_ready_on_original_conversation(false);
}
#[cfg(feature = "broker-tests")]
#[test]
fn queued_alarm_denial_does_not_fence_ready_or_a_sibling_consumer() {
    rejected_alarm_then_ready_on_original_conversation(true);
}
