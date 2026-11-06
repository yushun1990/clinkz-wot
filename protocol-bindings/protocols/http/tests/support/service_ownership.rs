use super::*;

struct BackedData {
    inner: Arc<dyn ConsumerDataService>,
    backing: Vec<u8>,
}
impl ConsumerDataService for BackedData {
    fn content_type(&self) -> &str {
        self.inner.content_type()
    }
    fn validate(&self, schema: value_type::TypeSchema, payload: &Payload) -> CoreResult<()> {
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
}
impl ConsumerAccessService for BackedAccess {
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
    });
    input.services.access = Arc::new(BackedAccess {
        inner: input.services.access,
        backing: vec![0; access_bytes],
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
        let original_services = live(&services_bank);
        drop(registry);
        assert_eq!(live(&services_bank), original_services);

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
