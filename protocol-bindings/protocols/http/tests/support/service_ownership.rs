use super::*;
use std::sync::atomic::{AtomicU64, AtomicUsize};

struct ReleaseEvidence {
    bank: Arc<HostResourceAccounts>,
    owner_charge: AtomicU64,
    drops: AtomicUsize,
    funded_at_drop: AtomicU64,
}
impl ReleaseEvidence {
    fn new(bank: &Arc<HostResourceAccounts>) -> Arc<Self> {
        Arc::new(Self {
            bank: bank.clone(),
            owner_charge: AtomicU64::new(0),
            drops: AtomicUsize::new(0),
            funded_at_drop: AtomicU64::new(0),
        })
    }
    fn record_drop(&self) {
        self.funded_at_drop
            .store(live(&self.bank), Ordering::Relaxed);
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
    fn assert_released_once(&self) {
        assert_eq!(self.drops.load(Ordering::Relaxed), 1);
        assert!(
            self.funded_at_drop.load(Ordering::Relaxed)
                >= self.owner_charge.load(Ordering::Relaxed)
        );
    }
}

struct BackedData {
    inner: Arc<dyn ConsumerDataService>,
    backing: Vec<u8>,
    release: Option<Arc<ReleaseEvidence>>,
}
impl Drop for BackedData {
    fn drop(&mut self) {
        if let Some(release) = &self.release {
            release.record_drop();
        }
    }
}
impl ConsumerDataService for BackedData {
    fn content_type(&self) -> &str {
        self.inner.content_type()
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
        self.inner.validate(schema, payload)
    }
    fn supports_property_map(&self, schema: &collection::PropertyCollectionSchema) -> bool {
        self.inner.supports_property_map(schema)
    }
    fn validate_property_map(
        &self,
        schema: &collection::PropertyCollectionSchema,
        payload: &Payload,
    ) -> CoreResult<()> {
        self.inner.validate_property_map(schema, payload)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut policy = self.inner.policy();
        policy.retained_bytes +=
            std::mem::size_of::<Self>() as u64 + self.backing.capacity() as u64;
        policy
    }
}
struct BackedAccess {
    inner: Arc<dyn ConsumerAccessService>,
    backing: Vec<u8>,
    release: Option<Arc<ReleaseEvidence>>,
}
impl Drop for BackedAccess {
    fn drop(&mut self) {
        if let Some(release) = &self.release {
            release.record_drop();
        }
    }
}
impl ConsumerAccessService for BackedAccess {
    fn security_revision(&self) -> u64 {
        self.inner.security_revision()
    }
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        self.inner.authorize(binding, target, operation)
    }
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        self.inner
            .authorize_instance(consumer, binding, target, operation)
    }
    fn retained_bytes(&self) -> u64 {
        self.inner.retained_bytes()
            + std::mem::size_of::<Self>() as u64
            + self.backing.capacity() as u64
    }
    fn work(&self) -> u64 {
        self.inner.work()
    }
}

fn backed_registration(
    id: u32,
    data_bytes: usize,
    access_bytes: usize,
) -> ConsumerRegistrationInput {
    let mut input = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(id),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap();
    input.services.data = Arc::new(BackedData {
        inner: input.services.data,
        backing: vec![0; data_bytes],
        release: None,
    });
    input.services.access = Arc::new(BackedAccess {
        inner: input.services.access,
        backing: vec![0; access_bytes],
        release: None,
    });
    input
}
fn registrations(
    data_bytes: usize,
    access_bytes: usize,
    share_data: bool,
    share_access: bool,
) -> Vec<ConsumerRegistrationInput> {
    let mut inputs: Vec<_> = (98..100)
        .map(|id| backed_registration(id, data_bytes, access_bytes))
        .collect();
    if share_data {
        inputs[1].services.data = inputs[0].services.data.clone();
    }
    if share_access {
        inputs[1].services.access = inputs[0].services.access.clone();
    }
    inputs
}
fn limited_bank(bytes: u64) -> Arc<HostResourceAccounts> {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::EngineLiveBytesGlobalMax, Some(bytes)));
    Arc::new(HostResourceAccounts::new(&limits))
}

type DataOwner = Arc<ConsumerServiceOwner<dyn ConsumerDataService>>;
type AccessOwner = Arc<ConsumerServiceOwner<dyn ConsumerAccessService>>;

fn retained_services(
    bank: &Arc<HostResourceAccounts>,
    data_bytes: usize,
    access_bytes: usize,
) -> (
    DataOwner,
    AccessOwner,
    Arc<ReleaseEvidence>,
    Arc<ReleaseEvidence>,
) {
    let input = registration();
    let data_release = ReleaseEvidence::new(bank);
    let access_release = ReleaseEvidence::new(bank);
    let before = live(bank);
    let data = ConsumerServiceOwner::retain_data(
        Arc::new(BackedData {
            inner: input.services.data,
            backing: vec![0; data_bytes],
            release: Some(data_release.clone()),
        }),
        bank,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    data_release
        .owner_charge
        .store(live(bank) - before, Ordering::Relaxed);
    let before = live(bank);
    let access = ConsumerServiceOwner::retain_access(
        Arc::new(BackedAccess {
            inner: input.services.access,
            backing: vec![0; access_bytes],
            release: Some(access_release.clone()),
        }),
        bank,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    access_release
        .owner_charge
        .store(live(bank) - before, Ordering::Relaxed);
    (data, access, data_release, access_release)
}

fn retained_registration(
    id: u32,
    data: &DataOwner,
    access: &AccessOwner,
) -> RetainedConsumerRegistrationInput {
    let mut input = registration();
    if id != 98 {
        input = try_consumer_registration(
            HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(id),
            BindingGeneration::INITIAL,
            0,
        )
        .unwrap();
    }
    RetainedConsumerRegistrationInput {
        preparation: input.preparation,
        data: data.clone(),
        access: access.clone(),
    }
}

#[test]
fn retained_import_preserves_one_physical_owner_and_separate_registry_costs() {
    for same_bank in [true, false] {
        for reverse in [false, true] {
            for (data_bytes, access_bytes) in [(60_000, 0), (0, 60_000), (30_000, 30_000)] {
                let origin = limited_bank(100_000);
                let second_bank = if same_bank {
                    origin.clone()
                } else {
                    limited_bank(100_000)
                };
                let (data, access, data_drop, access_drop) =
                    retained_services(&origin, data_bytes, access_bytes);
                let backing = live(&origin);
                let first = ConsumerRegistry::try_from_retained(
                    vec![retained_registration(98, &data, &access)],
                    &origin,
                )
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
                let local = live(&origin) - backing;
                let before = live(&second_bank);
                let second = ConsumerRegistry::try_from_retained(
                    vec![retained_registration(99, &data, &access)],
                    &second_bank,
                )
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
                assert_eq!(live(&second_bank) - before, local);
                assert!(local > 0 && local < 60_000);
                assert_eq!(
                    live(&origin),
                    backing + local * if same_bank { 2 } else { 1 }
                );
                eprintln!(
                    "retained import: same_bank={same_bank}, reverse={reverse}, backing={backing}, registry={local}"
                );
                drop((data, access));
                if reverse {
                    drop(second);
                    assert_eq!(live(&origin), backing + local);
                    if !same_bank {
                        clean(&second_bank);
                    }
                    drop(first);
                } else {
                    drop(first);
                    assert_eq!(live(&origin), backing + if same_bank { local } else { 0 });
                    assert_eq!(
                        live(&second_bank),
                        if same_bank { backing + local } else { local }
                    );
                    drop(second);
                }
                let at_final_destruction = backing + if reverse || same_bank { local } else { 0 };
                assert_eq!(
                    data_drop.funded_at_drop.load(Ordering::Relaxed),
                    at_final_destruction
                );
                assert_eq!(
                    access_drop.funded_at_drop.load(Ordering::Relaxed),
                    at_final_destruction - data_drop.owner_charge.load(Ordering::Relaxed)
                );
                data_drop.assert_released_once();
                access_drop.assert_released_once();
                clean(&origin);
                clean(&second_bank);
            }
        }
    }
}

#[test]
fn retained_registry_refusal_preserves_original_owners_and_rolls_back_local_acquisitions() {
    let origin = limited_bank(100_000);
    let (data, access, data_drop, access_drop) = retained_services(&origin, 30_000, 30_000);
    let backing = live(&origin);
    let probe = limited_bank(100_000);
    let first = ConsumerRegistry::try_from_retained(
        vec![retained_registration(98, &data, &access)],
        &probe,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let local = live(&probe);
    drop(first);
    clean(&probe);
    for limit in [0, local + 2048] {
        let local_bank = limited_bank(limit);
        let inputs = vec![
            retained_registration(98, &data, &access),
            retained_registration(99, &data, &access),
        ];
        let rejected = ConsumerRegistry::try_from_retained(inputs, &local_bank)
            .err()
            .expect("two registrations exceed the local bank");
        assert!(matches!(
            rejected.error().error(),
            CoreError::LimitExceeded { .. }
        ));
        clean(&local_bank);
        if limit > 0 {
            assert!(
                local_bank
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .peak()
                    > local,
                "refusal followed the first native registration acquisition"
            );
        }
        assert_eq!(live(&origin), backing);
        let inputs = rejected.into_input();
        assert_eq!(inputs.len(), 2);
        for input in &inputs {
            assert!(Arc::ptr_eq(&input.data, &data));
            assert!(Arc::ptr_eq(&input.access, &access));
        }
        let retry = ConsumerRegistry::try_from_retained(inputs, &probe)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        drop(retry);
        clean(&probe);
    }
    drop((data, access));
    data_drop.assert_released_once();
    access_drop.assert_released_once();
    clean(&origin);
}

#[test]
fn retained_import_combines_account_domains_without_merging_equal_distinct_services() {
    let origin_a = limited_bank(100_000);
    let origin_b = limited_bank(100_000);
    let local_bank = limited_bank(100_000);
    let (data_a, access_a, drop_da, drop_aa) = retained_services(&origin_a, 30_000, 30_000);
    let (data_b, access_b, drop_db, drop_ab) = retained_services(&origin_b, 30_000, 30_000);
    assert!(!Arc::ptr_eq(&data_a, &data_b));
    assert!(!Arc::ptr_eq(&access_a, &access_b));
    let backing = live(&origin_a);
    assert_eq!(live(&origin_b), backing);
    let first = ConsumerRegistry::try_from_retained(
        vec![retained_registration(98, &data_a, &access_b)],
        &local_bank,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let local = live(&local_bank);
    let second = ConsumerRegistry::try_from_retained(
        vec![retained_registration(99, &data_b, &access_a)],
        &local_bank,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(live(&local_bank), local * 2);
    assert_eq!(live(&origin_a) + live(&origin_b), backing * 2);
    drop((data_a, access_a, data_b, access_b));
    drop(first);
    assert_eq!(
        live(&origin_a),
        drop_aa.owner_charge.load(Ordering::Relaxed)
    );
    assert_eq!(
        live(&origin_b),
        drop_db.owner_charge.load(Ordering::Relaxed)
    );
    drop_da.assert_released_once();
    drop_ab.assert_released_once();
    assert_eq!(drop_aa.drops.load(Ordering::Relaxed), 0);
    assert_eq!(drop_db.drops.load(Ordering::Relaxed), 0);
    assert_eq!(live(&local_bank), local);
    drop(second);
    drop_aa.assert_released_once();
    drop_db.assert_released_once();
    clean(&origin_a);
    clean(&origin_b);
    clean(&local_bank);
}

#[test]
fn service_adoption_refusal_returns_the_last_raw_input_for_retry() {
    let empty = limited_bank(0);
    let origin = limited_bank(100_000);
    let input = backed_registration(98, 30_000, 30_000);
    let data_weak = Arc::downgrade(&input.services.data);
    let access_weak = Arc::downgrade(&input.services.access);
    let rejected_data = ConsumerServiceOwner::retain_data(input.services.data, &empty)
        .err()
        .expect("unfunded data service must return its input");
    let rejected_access = ConsumerServiceOwner::retain_access(input.services.access, &empty)
        .err()
        .expect("unfunded access service must return its input");
    assert!(matches!(
        rejected_data.error().error(),
        CoreError::LimitExceeded { .. }
    ));
    assert!(matches!(
        rejected_access.error().error(),
        CoreError::LimitExceeded { .. }
    ));
    clean(&empty);
    let data = rejected_data.into_input();
    let access = rejected_access.into_input();
    assert!(Arc::ptr_eq(&data, &data_weak.upgrade().unwrap()));
    assert!(Arc::ptr_eq(&access, &access_weak.upgrade().unwrap()));
    let data = ConsumerServiceOwner::retain_data(data, &origin)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let access = ConsumerServiceOwner::retain_access(access, &origin)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(live(&origin) >= 60_000);
    drop((data, access));
    assert!(data_weak.upgrade().is_none() && access_weak.upgrade().is_none());
    clean(&origin);
}

// A passive declaration-change fixture. Runtime validation is exercised by the
// actual HTTP services above; these services are never prepared or executed.
struct ChangingBacking(Arc<AtomicU64>);
impl ConsumerDataService for ChangingBacking {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn validate(&self, _: &data_schema::PreparedSchema, _: &Payload) -> CoreResult<()> {
        Ok(())
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: std::mem::size_of::<Self>() as u64 + self.0.load(Ordering::Relaxed),
            temporary_bytes: 0,
            body_bytes: 0,
            encoded_selection_bytes: 0,
            validation_work: 1,
        }
    }
}
impl ConsumerAccessService for ChangingBacking {
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
        std::mem::size_of::<Self>() as u64 + self.0.load(Ordering::Relaxed)
    }
    fn work(&self) -> u64 {
        1
    }
}

#[test]
fn changed_service_backing_refuses_import_and_returns_the_last_retained_owners() {
    for change_data in [true, false] {
        let origin = limited_bank(100_000);
        let local = limited_bank(100_000);
        let extra_data = Arc::new(AtomicU64::new(0));
        let extra_access = Arc::new(AtomicU64::new(0));
        let data = ConsumerServiceOwner::retain_data(
            Arc::new(ChangingBacking(extra_data.clone())),
            &origin,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let access = ConsumerServiceOwner::retain_access(
            Arc::new(ChangingBacking(extra_access.clone())),
            &origin,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let weak_data = Arc::downgrade(&data);
        let weak_access = Arc::downgrade(&access);
        let inputs = vec![retained_registration(98, &data, &access)];
        let backing = live(&origin);
        drop((data, access));
        let changed = if change_data {
            extra_data
        } else {
            extra_access
        };
        changed.store(1, Ordering::Relaxed);
        let rejected = ConsumerRegistry::try_from_retained(inputs, &local)
            .err()
            .expect("no refinancing of changed backing");
        assert!(!matches!(
            rejected.error().error(),
            CoreError::LimitExceeded { .. }
        ));
        clean(&local);
        assert_eq!(live(&origin), backing);
        let inputs = rejected.into_input();
        assert!(Arc::ptr_eq(&inputs[0].data, &weak_data.upgrade().unwrap()));
        assert!(Arc::ptr_eq(
            &inputs[0].access,
            &weak_access.upgrade().unwrap()
        ));
        changed.store(0, Ordering::Relaxed);
        let retry = ConsumerRegistry::try_from_retained(inputs, &local)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        drop(retry);
        assert!(weak_data.upgrade().is_none() && weak_access.upgrade().is_none());
        clean(&origin);
        clean(&local);
    }
}

fn registry_client(
    registry: &ConsumerRegistry,
    td: &Value,
    accounts: &Arc<HostResourceAccounts>,
    owner: &ConsumerInstance,
) -> ConsumerRuntime {
    let (bindings, funding) = registry
        .prepare(
            &serde_json::from_value(td.clone()).unwrap(),
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        )
        .unwrap()
        .into_execution();
    ConsumerRuntime::try_new(owner.reference(), bindings, funding, accounts.clone(), 1)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}

#[test]
fn independent_registries_retain_original_service_funding_through_normal_and_late_http_results() {
    io().block_on(async {
        for same_bank in [true, false] {
            for reverse in [false, true] {
                for cancelled in [false, true] {
                    let origin = limited_bank(100_000);
                    let second_bank = if same_bank { origin.clone() } else { limited_bank(100_000) };
                    let (data, access, data_drop, access_drop) = retained_services(&origin, 30_000, 30_000);
                    let backing = live(&origin);
                    let input_a = retained_registration(98, &data, &access);
                    let input_b = retained_registration(99, &data, &access);
                    // Native backing is independently owned by each actual
                    // registration. Service ownership never merges executors.
                    let native = input_a.preparation.resources().admitted().retained_bytes()
                        + std::mem::size_of::<HostConsumerBindingRegistration>() as u64 + 128;
                    assert_eq!(input_a.preparation.resources(), input_b.preparation.resources());
                    let registry_a = ConsumerRegistry::try_from_retained(vec![input_a], &origin)
                        .unwrap_or_else(|e| panic!("{:?}", e.error()));
                    let registry_b = ConsumerRegistry::try_from_retained(vec![input_b], &second_bank)
                        .unwrap_or_else(|e| panic!("{:?}", e.error()));
                    drop((data, access));
                    let mut server_a = Fixture::new(description(address()));
                    let mut server_b = Fixture::new(description(address()));
                    let owner_a = consumer_instance();
                    let owner_b = consumer_instance();
                    assert_eq!(owner_a.reference().slot(), owner_b.reference().slot());
                    assert_ne!(owner_a.reference(), owner_b.reference());
                    let accounts_a = bank();
                    let accounts_b = bank();
                    let client_a = registry_client(&registry_a, &description(server_a.addr), &accounts_a, &owner_a);
                    let client_b = registry_client(&registry_b, &description(server_b.addr), &accounts_b, &owner_b);
                    if reverse { drop(registry_b); drop(registry_a); }
                    else { drop(registry_a); drop(registry_b); }
                    assert_eq!(live(&origin), backing + native * if same_bank { 2 } else { 1 });
                    if !same_bank { assert_eq!(live(&second_bank), native); }

                    let mut normal_call = start_read(&client_a);
                    let (normal, _) = server_a.exchange(&mut normal_call).await;
                    assert!(normal.result().is_ok());
                    let mut late_call = start_read(&client_b);
                    // Retain an actual returned HTTP output without paying for
                    // validation, then retire only its original authority.
                    let started = Instant::now();
                    while server_b.app.roots.get() == 0 {
                        server_b.step();
                        assert!(late_call.poll(&mut cx(), now(0),
                            &mut work().with_remaining(WorkClass::CodecInputBytes, 0)).is_pending());
                        assert!(started.elapsed() < Duration::from_secs(5));
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    for _ in 0..20 {
                        server_b.step();
                        assert!(late_call.poll(&mut cx(), now(0),
                            &mut work().with_remaining(WorkClass::CodecInputBytes, 0)).is_pending());
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    let identity_b = owner_b.reference();
                    if cancelled { late_call.cancel(); }
                    drop((client_b, owner_b));
                    let successor = consumer_instance();
                    assert_ne!(identity_b, successor.reference());
                    let (late, _) = server_b.exchange(&mut late_call).await;
                    assert!(late.result().is_err());
                    assert_eq!(late.consumer(), &identity_b);
                    assert!(matches!(late.native_outcome(), ConsumerNativeOutcome::Returned(Ok(_))));
                    assert!(normal.result().is_ok(), "sibling authority is independent");
                    drop((normal_call, late_call, client_a, owner_a, successor));
                    assert!(live(&accounts_a) > 0 && live(&accounts_b) > 0);
                    assert_eq!(live(&origin), backing + native * if same_bank { 2 } else { 1 });
                    assert_eq!(data_drop.drops.load(Ordering::Relaxed), 0);
                    assert_eq!(access_drop.drops.load(Ordering::Relaxed), 0);
                    if reverse {
                        drop(late);
                        clean(&accounts_b);
                        assert_eq!(live(&origin), backing + native);
                        if !same_bank { clean(&second_bank); }
                        drop(normal);
                    } else {
                        drop(normal);
                        clean(&accounts_a);
                        assert_eq!(live(&origin), backing + if same_bank { native } else { 0 });
                        if !same_bank { assert_eq!(live(&second_bank), native); }
                        drop(late);
                    }
                    eprintln!("HTTP final release: same_bank={same_bank}, reverse={reverse}, cancelled={cancelled}, backing={backing}, native_each={native}, drops=1+1");
                    data_drop.assert_released_once();
                    access_drop.assert_released_once();
                    clean(&origin);
                    clean(&second_bank);
                    clean(&accounts_a);
                    clean(&accounts_b);
                    server_a.close();
                    server_b.close();
                }
            }
        }
    });
}

#[test]
fn raw_services_in_independent_registries_repeat_the_physical_charge() {
    for same_bank in [true, false] {
        for (data_bytes, access_bytes) in [(60_000, 0), (0, 60_000), (30_000, 30_000)] {
            let a = limited_bank(200_000);
            let b = if same_bank {
                a.clone()
            } else {
                limited_bank(200_000)
            };
            let mut inputs = registrations(data_bytes, access_bytes, true, true);
            let second = inputs.pop().unwrap();
            let first =
                ConsumerRegistry::try_new(inputs, &a).unwrap_or_else(|e| panic!("{:?}", e.error()));
            let first_charge = live(&a);
            let before_second = live(&b);
            let second = ConsumerRegistry::try_new(vec![second], &b)
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
            let second_charge = live(&b) - before_second;
            eprintln!(
                "raw independent registries: same_bank={same_bank}, data={data_bytes}, access={access_bytes}, first={first_charge}, second={second_charge}"
            );
            assert_eq!(first_charge, second_charge);
            assert!(second_charge >= 60_000);
            drop(first);
            assert_eq!(live(&b), second_charge);
            drop(second);
            clean(&a);
            clean(&b);
        }
    }
    let bank = limited_bank(100_000);
    let mut inputs = registrations(30_000, 30_000, true, true);
    let second = inputs.pop().unwrap();
    let first =
        ConsumerRegistry::try_new(inputs, &bank).unwrap_or_else(|e| panic!("{:?}", e.error()));
    let original = live(&bank);
    let rejected = ConsumerRegistry::try_new(vec![second], &bank)
        .err()
        .expect("a second charge refuses despite one 60KB backing");
    assert!(matches!(
        rejected.error().error(),
        CoreError::LimitExceeded { .. }
    ));
    assert_eq!(rejected.into_input().len(), 1);
    assert_eq!(live(&bank), original);
    drop(first);
    clean(&bank);
}

#[test]
fn aliased_services_are_charged_once_and_equal_distinct_services_stay_separate() {
    for (data_bytes, access_bytes, share_data, share_access) in [
        (60_000, 0, true, false),
        (0, 60_000, false, true),
        (30_000, 30_000, true, true),
    ] {
        let bank = limited_bank(100_000);
        let small = ConsumerRegistry::try_new(registrations(0, 0, share_data, share_access), &bank)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let small_bytes = live(&bank);
        drop(small);
        clean(&bank);

        let mut single = registrations(data_bytes, access_bytes, share_data, share_access);
        single.pop();
        let single =
            ConsumerRegistry::try_new(single, &bank).unwrap_or_else(|e| panic!("{:?}", e.error()));
        let single_bytes = live(&bank);
        drop(single);
        clean(&bank);

        let inputs = registrations(data_bytes, access_bytes, share_data, share_access);
        assert_eq!(
            Arc::ptr_eq(&inputs[0].services.data, &inputs[1].services.data),
            share_data
        );
        assert_eq!(
            Arc::ptr_eq(&inputs[0].services.access, &inputs[1].services.access),
            share_access
        );
        let second_registration_bytes = inputs[1]
            .preparation
            .resources()
            .admitted()
            .retained_bytes();
        let shared = ConsumerRegistry::try_new(inputs, &bank)
            .unwrap_or_else(|e| panic!("60KB of shared backing must fit: {:?}", e.error()));
        assert_eq!(live(&bank), small_bytes + 60_000);
        assert!(live(&bank) >= single_bytes + second_registration_bytes);
        drop(shared);
        clean(&bank);

        let distinct = match ConsumerRegistry::try_new(
            registrations(data_bytes, access_bytes, false, false),
            &bank,
        ) {
            Ok(_) => panic!("two distinct 60KB service backings must exceed the bank"),
            Err(rejected) => rejected,
        };
        assert!(matches!(
            distinct.error().error(),
            CoreError::LimitExceeded { .. }
        ));
        let inputs = distinct.into_input();
        assert_eq!(inputs.len(), 2);
        assert!(!Arc::ptr_eq(
            &inputs[0].services.data,
            &inputs[1].services.data
        ));
        assert!(!Arc::ptr_eq(
            &inputs[0].services.access,
            &inputs[1].services.access
        ));
        clean(&bank);
    }
}

#[test]
fn refusal_returns_aliased_inputs_and_releases_partial_registry_funding() {
    let bank = limited_bank(100_000);
    let mut inputs = registrations(30_000, 30_000, true, true);
    inputs.push(backed_registration(100, 60_000, 0));
    let data = Arc::downgrade(&inputs[0].services.data);
    let access = Arc::downgrade(&inputs[0].services.access);
    let rejected = match ConsumerRegistry::try_new(inputs, &bank) {
        Ok(_) => panic!("the additional distinct backing must exceed the bank"),
        Err(rejected) => rejected,
    };
    assert!(matches!(
        rejected.error().error(),
        CoreError::LimitExceeded { .. }
    ));
    clean(&bank);
    let inputs = rejected.into_input();
    assert_eq!(inputs.len(), 3);
    assert!(Arc::ptr_eq(
        &inputs[0].services.data,
        &data.upgrade().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &inputs[0].services.access,
        &access.upgrade().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &inputs[0].services.data,
        &inputs[1].services.data
    ));
    assert!(Arc::ptr_eq(
        &inputs[0].services.access,
        &inputs[1].services.access
    ));
    let retry_bank = limited_bank(200_000);
    let registry = ConsumerRegistry::try_new(inputs, &retry_bank)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    drop(registry);
    assert!(data.upgrade().is_none());
    assert!(access.upgrade().is_none());
    clean(&retry_bank);
}

#[test]
fn aliased_service_funding_survives_registry_and_instances_until_final_http_result() {
    io().block_on(async {
        let services_bank = limited_bank(100_000);
        let inputs = registrations(30_000, 30_000, true, true);
        let data = Arc::downgrade(&inputs[0].services.data);
        let access = Arc::downgrade(&inputs[0].services.access);
        let registry = ConsumerRegistry::try_new(inputs, &services_bank)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let mut server_a = Fixture::new(description(address()));
        let mut server_b = Fixture::new(description(address()));
        let accounts_a = bank();
        let accounts_b = bank();
        let prepare = |td: &Value, id, accounts: &Arc<HostResourceAccounts>| {
            registry
                .prepare(
                    &serde_json::from_value(td.clone()).unwrap(),
                    GatewayDefaultV1::LIMITS,
                    accounts.clone(),
                    ConsumerSelector {
                        binding: Some(BindingId::new(id)),
                        form: None,
                    },
                    &mut work(),
                    |job| {
                        while !job.is_finished() {
                            job.step(&mut work(), false);
                        }
                    },
                )
                .unwrap()
                .into_execution()
        };
        let (bindings_a, funding_a) = prepare(&description(server_a.addr), 98, &accounts_a);
        let (bindings_b, funding_b) = prepare(&description(server_b.addr), 99, &accounts_b);
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        let client_a = ConsumerRuntime::try_new(
            owner_a.reference(),
            bindings_a,
            funding_a,
            accounts_a.clone(),
            1,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let client_b = ConsumerRuntime::try_new(
            owner_b.reference(),
            bindings_b,
            funding_b,
            accounts_b.clone(),
            1,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let registered_services = live(&services_bank);
        drop(registry);
        let original_services = live(&services_bank);
        assert!(
            original_services < registered_services,
            "registry records retire"
        );

        let mut call_a = start_read(&client_a);
        let mut call_b = start_read(&client_b);
        let (result_a, _) = server_a.exchange(&mut call_a).await;
        let (result_b, _) = server_b.exchange(&mut call_b).await;
        assert!(result_a.result().is_ok() && result_b.result().is_ok());
        drop(call_a);
        drop(call_b);
        drop(client_a);
        drop(client_b);
        drop(owner_a);
        drop(owner_b);
        assert_eq!(live(&services_bank), original_services);
        drop(result_a);
        assert!(data.upgrade().is_some() && access.upgrade().is_some());
        assert!(live(&services_bank) >= 60_000);
        clean(&accounts_a);
        drop(result_b);
        assert!(data.upgrade().is_none() && access.upgrade().is_none());
        clean(&services_bank);
        clean(&accounts_b);
        server_a.close();
        server_b.close();
    });
}
