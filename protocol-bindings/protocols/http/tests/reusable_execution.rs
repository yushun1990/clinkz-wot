#![cfg(feature = "prepare")]
#![allow(dead_code)] // Shared real-HTTP fixture also serves other capability tests.
//! Reusable Engine content, specialized native mappings, independent instances.
include!("support/collection.rs");
#[path = "support/allocation_meter.rs"]
mod allocation_meter;
#[path = "support/native_occupancy.rs"]
mod native_occupancy;
#[path = "support/service_ownership.rs"]
mod service_ownership;
#[global_allocator]
static ALLOCATOR: allocation_meter::Allocator = allocation_meter::Allocator;

fn mapping(
    td: &Value,
    backing: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> Arc<ConsumerDefinition> {
    mapping_cost(td, backing, reuse).0
}

struct PreparationCost {
    content_constructions: usize,
    work: [u64; WorkClass::ALL.len()],
    native_jobs: usize,
}
fn mapping_cost(
    td: &Value,
    backing: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> (Arc<ConsumerDefinition>, PreparationCost) {
    let registry = ConsumerRegistry::try_new(vec![registration()], backing)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let thing: Thing = serde_json::from_value(td.clone()).unwrap();
    let mut native_jobs = 0;
    let mut budget = work();
    let mut drive = |job: &mut ConsumerCompilation<'_>| {
        native_jobs += 1;
        while !job.is_finished() {
            job.step(&mut work(), false);
        }
    };
    let p = if let Some(content) = reuse {
        registry.prepare_with_content(
            &thing,
            GatewayDefaultV1::LIMITS,
            backing.clone(),
            ConsumerSelector::default(),
            content,
            &mut budget,
            &mut drive,
        )
    } else {
        registry.prepare(
            &thing,
            GatewayDefaultV1::LIMITS,
            backing.clone(),
            ConsumerSelector::default(),
            &mut budget,
            &mut drive,
        )
    }
    .unwrap();
    let content_constructions = p.content_constructions();
    let (bindings, funding) = p.into_execution();
    let result = bindings
        .iter()
        .find(|d| d.operation() == Operation::ReadAllProperties)
        .unwrap()
        .definition()
        .clone();
    drop(bindings);
    drop(funding);
    // Neither source, arena, compiler nor registry escapes this function.
    (
        result,
        PreparationCost {
            content_constructions,
            work: WorkClass::ALL.map(|k| 1_000_000 - budget.remaining(k)),
            native_jobs,
        },
    )
}

struct InstancePolicy {
    expected: ConsumerInstanceRef,
    denied: Arc<AtomicBool>,
}
impl ConsumerAccessService for InstancePolicy {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        consumer: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        if consumer != &self.expected || self.denied.load(Ordering::Relaxed) {
            Err(denied())
        } else {
            Ok(())
        }
    }
    fn retained_bytes(&self) -> u64 {
        128
    }
    fn work(&self) -> u64 {
        2
    }
}
fn bind(
    definition: Arc<ConsumerDefinition>,
    owner: &ConsumerInstance,
    accounts: &Arc<HostResourceAccounts>,
    denied: Arc<AtomicBool>,
) -> (ConsumerAssociation, ConsumerCalls) {
    let policy = ConsumerServiceOwner::retain_access(
        Arc::new(InstancePolicy {
            expected: owner.reference(),
            denied,
        }),
        accounts,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let binding = definition
        .bind(policy, accounts.clone(), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    ConsumerAssociation::try_new(
        owner.reference(),
        vec![binding],
        vec![],
        accounts.clone(),
    )
    .map(|association| {
        (
            association,
            ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
        )
    })
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn members(content: &PreparedConsumerContent) -> &collection::PropertyCollectionSchema {
    let ConsumerFiniteSchemas::ReadAllProperties(s) = content.schemas() else {
        panic!()
    };
    s
}
fn live(accounts: &HostResourceAccounts) -> u64 {
    accounts
        .account(ResourceKind::EngineLiveBytesGlobalMax)
        .used()
}

#[test]
fn independent_endpoints_share_preparation_but_keep_policy_authority_and_original_funding() {
    io().block_on(async {
        let td_a = description(address());
        let td_b = description(address());
        let mut server_a = Fixture::new(td_a.clone());
        let mut server_b = Fixture::new(td_b.clone());
        *server_b.app.value.borrow_mut() = br#"{"level":19,"on":false}"#.to_vec();
        let backing_a = bank();
        let backing_b = bank();
        let definition_a = mapping(&td_a, &backing_a, None);
        let content = definition_a.content().clone();
        let weak_content = Arc::downgrade(&content);
        let before_failed_preparation = live(&backing_a);
        let failed_bank = bank();
        let failed_registry = ConsumerRegistry::try_new(vec![registration()], &failed_bank)
            .unwrap_or_else(|_| panic!());
        let source: Thing = serde_json::from_value(td_b.clone()).unwrap();
        assert!(
            failed_registry
                .prepare_with_content(
                    &source,
                    GatewayDefaultV1::LIMITS,
                    failed_bank.clone(),
                    ConsumerSelector::default(),
                    &content,
                    &mut work(),
                    |job| {
                        job.step(&mut WorkBudget::new(), true);
                    }
                )
                .is_err()
        );
        drop(failed_registry);
        drop(source);
        clean(&failed_bank);
        assert_eq!(live(&backing_a), before_failed_preparation);
        let definition_b = mapping(&td_b, &backing_b, Some(&content));
        assert!(Arc::ptr_eq(definition_a.content(), definition_b.content()));
        assert_eq!(
            members(definition_a.content()).members().as_ptr(),
            members(definition_b.content()).members().as_ptr()
        );
        assert_eq!(
            members(definition_a.content()).members()[0].name.as_ptr(),
            members(definition_b.content()).members()[0].name.as_ptr()
        );
        assert_ne!(definition_a.target(), definition_b.target());
        assert!(!std::ptr::eq(
            definition_a.artifact(),
            definition_b.artifact()
        ));
        let weak_a = Arc::downgrade(&definition_a);
        let weak_b = Arc::downgrade(&definition_b);
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        assert_eq!(owner_a.reference().slot(), owner_b.reference().slot());
        assert_ne!(owner_a.reference(), owner_b.reference());
        let account_a = bank();
        let account_b = bank();
        let deny_a = Arc::new(AtomicBool::new(false));
        let deny_b = Arc::new(AtomicBool::new(false));
        let client_a = bind(definition_a.clone(), &owner_a, &account_a, deny_a.clone());
        let client_b = bind(definition_b.clone(), &owner_b, &account_b, deny_b.clone());
        drop(td_a);
        drop(td_b);
        drop(definition_a);
        drop(definition_b);
        drop(content);

        deny_a.store(true, Ordering::Relaxed);
        assert!(
            client_a
                .0
                .read_all_properties(&client_a.1, Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(server_a.controls.requests.get(), 0);
        let mut b = start_read(&client_b);
        let (result_b, _) = server_b.exchange(&mut b).await;
        assert_eq!(
            result_b.result().unwrap().data().unwrap().body,
            br#"{"level":19,"on":false}"#
        );
        assert_eq!(result_b.consumer(), &owner_b.reference());
        drop(result_b);
        drop(b);
        deny_a.store(false, Ordering::Relaxed);
        let mut a = start_read(&client_a);
        let (result_a, _) = server_a.exchange(&mut a).await;
        assert_eq!(
            result_a.result().unwrap().data().unwrap().body,
            br#"{"level":7,"on":true}"#
        );
        assert_eq!(result_a.consumer(), &owner_a.reference());
        // Native occupancy retired with the exchange, independently of result_a.
        // This unpolled accepted call has no effects and retires with client_a.
        let unpolled = start_read(&client_a);
        drop(unpolled);
        let a_identity = owner_a.reference();
        drop(a);
        drop(client_a);
        drop(owner_a);
        assert!(!a_identity.is_current());
        assert!(weak_a.upgrade().is_some());
        assert!(live(&account_a) > 0 && live(&backing_a) > 0);
        // The sibling remains usable while the original unread result owns all
        // original backing, even after the preparation and instance roots go.
        let mut b = start_read(&client_b);
        let (result_b, _) = server_b.exchange(&mut b).await;
        assert!(result_b.result().is_ok());
        assert_eq!(server_b.app.roots.get(), 2);
        drop(result_b);
        drop(b);
        drop(result_a);
        clean(&account_a);
        assert!(weak_a.upgrade().is_none());
        assert!(
            live(&backing_a) > 0,
            "original common backing still belongs to A's preparation bank"
        );
        drop(client_b);
        drop(owner_b);
        assert!(weak_b.upgrade().is_none());
        assert!(weak_content.upgrade().is_none());
        clean(&account_b);
        clean(&backing_a);
        clean(&backing_b);
        assert_eq!(server_a.app.reads.get() + server_b.app.reads.get(), 0);
        assert_eq!(server_a.controls.requests.get(), 1);
        assert_eq!(server_b.controls.requests.get(), 2);
        server_a.close();
        server_b.close();
    });
}

#[test]
fn source_free_rebinding_preserves_bank_guards_and_returns_failed_binding_owners() {
    let backing = bank();
    let definition = mapping(&description(address()), &backing, None);
    let weak = Arc::downgrade(&definition);
    let account_a = bank();
    let account_b = bank();
    let owner = consumer_instance();
    let policy = ConsumerServiceOwner::retain_access(
        Arc::new(InstancePolicy {
            expected: owner.reference(),
            denied: Arc::new(AtomicBool::new(false)),
        }),
        &account_a,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let before_a = live(&account_a);
    let before_backing = live(&backing);
    let saturated = account_b
        .reserve(
            ResourceKind::EngineLiveBytesGlobalMax,
            account_b
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .limit(),
        )
        .unwrap();
    let refusal = definition
        .clone()
        .bind(policy.clone(), account_b.clone(), &mut work())
        .err()
        .unwrap();
    let (returned_definition, returned_policy) = refusal.into_input();
    assert!(Arc::ptr_eq(&returned_definition, &definition));
    assert!(Arc::ptr_eq(&returned_policy, &policy));
    assert_eq!(live(&account_a), before_a);
    assert_eq!(live(&backing), before_backing);
    drop(saturated);
    drop(returned_definition);
    drop(returned_policy);
    let binding = definition
        .clone()
        .bind(policy.clone(), account_b.clone(), &mut work())
        .unwrap_or_else(|_| panic!());
    let refused = ConsumerAssociation::try_new(
        owner.reference(),
        vec![binding],
        vec![],
        account_a.clone(),
    )
    .map(|association| {
        (
            association,
            ConsumerCalls::try_new(account_a.clone(), 1).unwrap(),
        )
    })
    .err()
    .expect("foreign instance bank is still refused");
    let (bindings, funding) = refused.into_input();
    let old_identity = owner.reference();
    drop(owner);
    let stale =
        ConsumerAssociation::try_new(old_identity, bindings, funding, account_b.clone())
            .map(|association| {
                (
                    association,
                    ConsumerCalls::try_new(account_b.clone(), 1).unwrap(),
                )
            })
            .err()
            .expect("stale original authority cannot be installed");
    let (bindings, funding) = stale.into_input();
    let replacement = consumer_instance();
    let runtime = ConsumerAssociation::try_new(
        replacement.reference(),
        bindings,
        funding,
        account_b.clone(),
    )
    .map(|association| {
        (
            association,
            ConsumerCalls::try_new(account_b.clone(), 1).unwrap(),
        )
    })
    .unwrap_or_else(|_| panic!());
    // An equal numeric slot cannot assume the old policy's identity.
    assert!(
        runtime
            .0
            .read_all_properties(&runtime.1, Deadline::NONE, now(0), &mut work())
            .is_err()
    );
    drop(runtime);
    drop(policy);
    drop(definition);
    assert!(weak.upgrade().is_none());
    clean(&account_a);
    clean(&account_b);
    clean(&backing);
}

#[test]
fn shared_actual_registration_and_data_backing_keep_their_original_owner() {
    let services_bank = bank();
    let mapping_bank_a = bank();
    let mapping_bank_b = bank();
    let registry = ConsumerRegistry::try_new(vec![registration()], &services_bank)
        .unwrap_or_else(|_| panic!());
    let prepare = |td: &Thing,
                   bank: &Arc<HostResourceAccounts>,
                   content: Option<&Arc<PreparedConsumerContent>>| {
        let mut budget = work();
        let drive = |job: &mut ConsumerCompilation<'_>| {
            while !job.is_finished() {
                job.step(&mut work(), false);
            }
        };
        let p = if let Some(content) = content {
            registry.prepare_with_content(
                td,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                content,
                &mut budget,
                drive,
            )
        } else {
            registry.prepare(
                td,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                ConsumerSelector::default(),
                &mut budget,
                drive,
            )
        }
        .unwrap();
        let (bindings, funding) = p.into_execution();
        let definition = bindings
            .iter()
            .find(|b| b.operation() == Operation::ReadAllProperties)
            .unwrap()
            .definition()
            .clone();
        drop(bindings);
        drop(funding);
        definition
    };
    let a = prepare(
        &serde_json::from_value(description(address())).unwrap(),
        &mapping_bank_a,
        None,
    );
    let content = a.content().clone();
    let b = prepare(
        &serde_json::from_value(description(address())).unwrap(),
        &mapping_bank_b,
        Some(&content),
    );
    assert!(std::ptr::eq(a.execution(), b.execution()));
    assert!(std::ptr::eq(a.data(), b.data()));
    drop(registry);
    let original_services = live(&services_bank);
    drop(a);
    assert_eq!(
        live(&services_bank),
        original_services,
        "native/data owners remain at the original bank"
    );
    drop(b);
    clean(&services_bank);
    clean(&mapping_bank_b);
    assert!(
        live(&mapping_bank_a) > 0,
        "the retained semantic root still owns its original funding"
    );
    drop(content);
    clean(&mapping_bank_a);
}

#[test]
fn changed_path_and_headers_are_compiled_for_the_actual_endpoint() {
    io().block_on(async {
        let backing = bank();
        let original = mapping(&description(address()), &backing, None);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut td = description(listener.local_addr().unwrap());
        td["forms"][0]["href"] = json!(format!("http://{}/second-root", listener.local_addr().unwrap()));
        td["forms"][0]["htv:headers"] = json!([{"htv:fieldName":"X-Instance", "htv:fieldValue":"second"}]);
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new(); let mut buf = [0; 1024];
            while !bytes.ends_with(b"\r\n\r\n") {
                let n = socket.read(&mut buf).unwrap(); assert!(n > 0); bytes.extend_from_slice(&buf[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.starts_with("GET /second-root HTTP/1.1\r\n"));
            assert!(request.to_ascii_lowercase().contains("x-instance: second\r\n"));
            let body = br#"{"level":23,"on":false}"#;
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).unwrap();
            socket.write_all(body).unwrap();
        });
        let actual = mapping(&td, &backing, Some(original.content()));
        drop(td); drop(original);
        let account = bank(); let owner = consumer_instance();
        let client = bind(actual, &owner, &account, Arc::new(AtomicBool::new(false)));
        let mut call = start_read(&client); let started = Instant::now();
        let result = loop {
            if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) { break result; }
            assert!(started.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert_eq!(result.result().unwrap().data().unwrap().body, br#"{"level":23,"on":false}"#);
        peer.join().unwrap();
        drop(result); drop(call); drop(client); clean(&account); clean(&backing);
    });
}

#[test]
fn source_free_binding_and_warm_admission_cannot_refresh_changed_service_bounds() {
    struct ChangingData {
        inner: Arc<dyn ConsumerDataService>,
        mode: Arc<std::sync::atomic::AtomicUsize>,
    }
    impl ConsumerDataService for ChangingData {
        fn content_type(&self) -> &str {
            self.inner.content_type()
        }
        fn validate(
            &self,
            schema: &data_schema::PreparedSchema,
            payload: &Payload,
        ) -> CoreResult<()> {
            self.inner.validate(schema, payload)
        }
        fn policy(&self) -> ConsumerDataPolicy {
            let mut p = self.inner.policy();
            p.retained_bytes += 128;
            if self.mode.load(Ordering::Relaxed) == 2 {
                p.validation_work += 1;
            }
            p
        }
        fn available(&self) -> bool {
            self.mode.load(Ordering::Relaxed) != 1
        }
        fn supports_property_map(&self, s: &collection::PropertyCollectionSchema) -> bool {
            self.mode.load(Ordering::Relaxed) != 3 && self.inner.supports_property_map(s)
        }
        fn validate_property_map(
            &self,
            s: &collection::PropertyCollectionSchema,
            p: &Payload,
        ) -> CoreResult<()> {
            self.inner.validate_property_map(s, p)
        }
    }
    let backing = bank();
    let original = mapping(&description(address()), &backing, None);
    for state in 1..=3 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let thing: Thing =
            serde_json::from_value(description(listener.local_addr().unwrap())).unwrap();
        let preparation_bank = bank();
        let account = bank();
        let owner = consumer_instance();
        let mode = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut reg = registration();
        reg.services.data = Arc::new(ChangingData {
            inner: reg.services.data,
            mode: mode.clone(),
        });
        let registry =
            ConsumerRegistry::try_new(vec![reg], &preparation_bank).unwrap_or_else(|_| panic!());
        let p = registry
            .prepare_with_content(
                &thing,
                GatewayDefaultV1::LIMITS,
                preparation_bank.clone(),
                ConsumerSelector::default(),
                original.content(),
                &mut work(),
                |job| {
                    while !job.is_finished() {
                        job.step(&mut work(), false);
                    }
                },
            )
            .unwrap();
        let (bindings, funding) = p.into_execution();
        let definition = bindings[0].definition().clone();
        drop(bindings);
        drop(funding);
        drop(registry);
        drop(thing);
        let client = bind(
            definition.clone(),
            &owner,
            &account,
            Arc::new(AtomicBool::new(false)),
        );
        mode.store(state, Ordering::Relaxed);
        assert!(
            client
                .0
                .read_all_properties(&client.1, Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        let policy = ConsumerServiceOwner::retain_access(
            Arc::new(InstancePolicy {
                expected: owner.reference(),
                denied: Arc::new(AtomicBool::new(false)),
            }),
            &account,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        assert!(
            definition
                .clone()
                .bind(policy, account.clone(), &mut work())
                .is_err()
        );
        listener.set_nonblocking(true).unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        drop(client);
        drop(definition);
        clean(&account);
        clean(&preparation_bank);
    }
    drop(original);
    clean(&backing);
}

#[test]
fn reuse_requalifies_all_actual_facts_and_never_relabels_an_exact_native_mapping() {
    let backing = bank();
    let original = description(address());
    let definition = mapping(&original, &backing, None);
    let content = definition.content().clone();
    for bad in [
        "name",
        "type",
        "assertion",
        "security",
        "media",
        "response",
        "method",
        "extension",
        "variables",
        "endpoint",
        "operation",
    ] {
        let mut td = description(address());
        match bad {
            "name" => {
                let member = td["properties"]
                    .as_object_mut()
                    .unwrap()
                    .remove("on")
                    .unwrap();
                td["properties"]["enabled"] = member;
            }
            "type" => td["properties"]["level"]["type"] = json!("number"),
            "assertion" => td["properties"]["level"]["minimum"] = json!(0),
            "security" => {
                td["securityDefinitions"]["n"] = json!({"scheme":"basic"});
            }
            "media" => td["forms"][0]["contentType"] = json!("text/plain"),
            "response" => td["forms"][0]["response"] = json!({"contentType":"text/plain"}),
            "method" => td["forms"][0]["htv:methodName"] = json!("POST"),
            "extension" => {
                td["forms"][0]["htv:headers"] =
                    json!([{"htv:fieldName":"Host","htv:fieldValue":"foreign"}])
            }
            "variables" => td["uriVariables"] = json!({"q":{"type":"integer"}}),
            "endpoint" => td["forms"][0]["href"] = json!("https://127.0.0.1/properties"),
            _ => td["forms"][0]["op"] = json!("readmultipleproperties"),
        }
        let accounts = bank();
        let registry =
            ConsumerRegistry::try_new(vec![registration()], &accounts).unwrap_or_else(|_| panic!());
        let thing: Thing = serde_json::from_value(td).unwrap();
        let mut jobs = 0;
        let result = registry.prepare_with_content(
            &thing,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &content,
            &mut work(),
            |j| {
                jobs += 1;
                while !j.is_finished() {
                    j.step(&mut work(), false);
                }
            },
        );
        assert!(result.is_err(), "{bad}");
        if matches!(
            bad,
            "name"
                | "type"
                | "assertion"
                | "security"
                | "media"
                | "response"
                | "variables"
                | "operation"
        ) {
            assert_eq!(
                jobs, 0,
                "{bad}: incompatible meaning must refuse before native construction"
            );
        }
        drop(result);
        drop(registry);
        clean(&accounts);
    }
    // Core's source-free carrier still rejects a foreign actual registration.
    let prepared = mapping(&original, &backing, Some(&content));
    let mut input = Arc::try_unwrap(prepared).ok().unwrap().into_input();
    input.execution = Arc::new(
        try_consumer_execution(
            HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(999),
            BindingGeneration::INITIAL,
            0,
        )
        .unwrap(),
    );
    let rejected = ConsumerDefinition::try_new(input)
        .err()
        .expect("foreign actual provider");
    drop(rejected);
    let prepared = mapping(&original, &backing, Some(&content));
    let mut input = Arc::try_unwrap(prepared).ok().unwrap().into_input();
    input.configuration = ConsumerBindingConfiguration::for_semantic_host(
        64,
        BindingConfigurationDigest::new([71; 32]),
        input.artifact.identity().compatibility(),
        4096,
        64,
    )
    .unwrap();
    assert!(
        ConsumerDefinition::try_new(input).is_err(),
        "changed native configuration"
    );
    drop(definition);
    drop(content);
    clean(&backing);
}

#[test]
fn replacement_and_late_results_retain_the_original_shared_meaning() {
    io().block_on(async {
        for cancelled in [false, true] {
            let td_a = description(address());
            let td_b = description(address());
            let mut server_a = Fixture::new(td_a.clone());
            let mut server_b = Fixture::new(td_b.clone());
            let backing_a = bank();
            let backing_b = bank();
            let definition_a = mapping(&td_a, &backing_a, None);
            let content = definition_a.content().clone();
            let weak = Arc::downgrade(&content);
            let definition_b = mapping(&td_b, &backing_b, Some(&content));
            let owner_a = consumer_instance();
            let owner_b = consumer_instance();
            let accounts_a = bank();
            let accounts_b = bank();
            let client_a = bind(
                definition_a,
                &owner_a,
                &accounts_a,
                Arc::new(AtomicBool::new(false)),
            );
            let client_b = bind(
                definition_b.clone(),
                &owner_b,
                &accounts_b,
                Arc::new(AtomicBool::new(false)),
            );
            drop(td_a);
            drop(td_b);
            drop(content);
            let mut a = start_read(&client_a);
            // Retain a real native output awaiting validation, then replace only A.
            let started = Instant::now();
            while server_a.app.roots.get() == 0 {
                server_a.step();
                assert!(
                    a.poll(
                        &mut cx(),
                        now(0),
                        &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
                    )
                    .is_pending()
                );
                assert!(started.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            for _ in 0..20 {
                server_a.step();
                assert!(
                    a.poll(
                        &mut cx(),
                        now(0),
                        &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
                    )
                    .is_pending()
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            if cancelled {
                a.cancel();
            }
            let identity = owner_a.reference();
            drop(owner_a);
            drop(client_a);
            let replacement = consumer_instance();
            assert_ne!(identity, replacement.reference());
            let (late, _) = server_a.exchange(&mut a).await;
            assert!(late.result().is_err());
            assert_eq!(late.consumer(), &identity);
            assert!(matches!(
                late.native_outcome(),
                ConsumerNativeOutcome::Returned(Ok(_))
            ));
            drop(a);
            let mut b = start_read(&client_b);
            let (result_b, _) = server_b.exchange(&mut b).await;
            assert!(result_b.result().is_ok());
            drop(result_b);
            drop(b);
            drop(client_b);
            drop(owner_b);
            drop(definition_b);
            clean(&accounts_b);
            clean(&backing_b);
            assert!(weak.upgrade().is_some());
            assert!(live(&backing_a) > 0 && live(&accounts_a) > 0);
            drop(late);
            assert!(weak.upgrade().is_none());
            clean(&accounts_a);
            clean(&backing_a);
            server_a.close();
            server_b.close();
        }
    });
}

#[test]
fn characterize_shared_preparation_instance_and_active_backing_over_supported_counts() {
    io().block_on(async {
        for shared in [true, false] {
        for count in [1, 2, 4, 8, 16, 32] {
            // One root mapping, sixteen nontrivial named schemas. No leaf Form
            // or individual Property work is involved in this measurement.
            let tds: Vec<_> = (0..count).map(|_| {
                let mut td = description(address());
                for p in td["properties"].as_object_mut().unwrap().values_mut() {
                    p["forms"] = json!([]);
                }
                for i in 0..14 {
                    td["properties"][format!("calibrated_measurement_{i:02}_with_a_retained_name")] =
                        json!({"type":"integer", "readOnly":true, "forms":[]});
                }
                td
            }).collect();
            let mut servers: Vec<_> = tds.iter().map(|td| {
                let server = Fixture::new(td.clone());
                let mut value = json!({"level":7,"on":true});
                for i in 0..14 { value[format!("calibrated_measurement_{i:02}_with_a_retained_name")] = json!(i); }
                *server.app.value.borrow_mut() = serde_json::to_vec(&value).unwrap();
                server
            }).collect();
            // External account provisioning, peer/source storage and these
            // measurement vectors are outside the measured Engine phases.
            let backing: Vec<_> = (0..count).map(|_| bank()).collect();
            let accounts: Vec<_> = (0..count).map(|_| bank()).collect();
            let mut definitions = Vec::with_capacity(count);
            let mut owners = Vec::with_capacity(count);
            let mut clients = Vec::with_capacity(count);
            let mut calls = Vec::with_capacity(count);
            let mut results = Vec::with_capacity(count);
            let baseline = allocation_meter::read();
            let started = Instant::now();
            let (first, first_cost) = mapping_cost(&tds[0], &backing[0], None);
            let content = first.content().clone();
            let weak = Arc::downgrade(&content);
            definitions.push(first);
            let mut jobs = first_cost.native_jobs;
            let mut content_builds = first_cost.content_constructions;
            let mut total_work = first_cost.work;
            for i in 1..count {
                let (definition, cost) = mapping_cost(&tds[i], &backing[i], shared.then_some(&content));
                assert_eq!(Arc::ptr_eq(definition.content(), &content), shared);
                assert_eq!(members(definition.content()).members().as_ptr() == members(&content).members().as_ptr(), shared);
                definitions.push(definition);
                jobs += cost.native_jobs;
                content_builds += cost.content_constructions;
                for (total, n) in total_work.iter_mut().zip(cost.work) { *total += n; }
            }
            let prepare_us = started.elapsed().as_micros();
            assert_eq!(jobs, count, "exact native compilation remains per endpoint");
            let after_preparation = allocation_meter::read();
            let prepare_reservations: u64 = backing.iter().map(|b| live(b)).sum();
            let started = Instant::now();
            for (i, definition) in definitions.iter().enumerate() {
                let owner = consumer_instance();
                clients.push(bind(definition.clone(), &owner, &accounts[i], Arc::new(AtomicBool::new(false))));
                owners.push(owner);
            }
            let bind_us = started.elapsed().as_micros();
            let idle = allocation_meter::read();
            let instance_reservations: u64 = accounts.iter().map(|b| live(b)).sum();
            for client in &clients { calls.push(start_read(client)); }
            let admitted = allocation_meter::read();
            let active_reservations: u64 = accounts.iter().map(|b| live(b)).sum();
            for (server, call) in servers.iter_mut().zip(&mut calls) {
                let (local, remote) = server.exchange(call).await;
                assert!(local.result().is_ok()); assert!(remote.result.is_ok());
                assert_eq!(server.app.roots.get(), 1); assert_eq!(server.app.reads.get(), 0);
                results.push(local);
            }
            // Retire every instance and endpoint root while keeping all unread
            // results. Their actual shared/mapping/service/call owners survive.
            drop(calls); drop(clients); drop(owners); drop(definitions);
            let semantic_bytes = members(&content).retained_bytes();
            drop(content);
            assert!(weak.upgrade().is_some());
            for b in &accounts { assert!(live(b) > 0); }
            for b in &backing { assert!(live(b) > 0); }
            drop(results);
            assert!(weak.upgrade().is_none()); drop(weak);
            for b in accounts.iter().chain(&backing) { clean(b); }
            assert_eq!(content_builds, if shared { 1 } else { count });
            println!("reuse-cost shared={shared} n={count} content_builds={content_builds} native_jobs={jobs} member_backing={semantic_bytes} prepare_live={} prepare_allocs={} prepare_requested={} instance_live={} instance_allocs={} admitted_call_live={} prepare_reserved={prepare_reservations} instance_reserved={instance_reservations} active_reserved={active_reservations} prepare_us={prepare_us} bind_us={bind_us} first_work={:?} total_work={total_work:?}",
                after_preparation.live-baseline.live,
                after_preparation.allocations-baseline.allocations,
                after_preparation.allocated_bytes-baseline.allocated_bytes,
                idle.live-after_preparation.live,
                idle.allocations-after_preparation.allocations,
                admitted.live-idle.live, first_cost.work);
            for server in servers { server.close(); }
        }
        }
    });
}
