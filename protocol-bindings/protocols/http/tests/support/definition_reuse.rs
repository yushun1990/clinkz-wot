use super::*;

fn config() -> HttpConsumerConfig {
    HttpConsumerConfig::for_cleartext_ip()
        .unwrap()
        .with_target_capacity(256)
        .unwrap()
}
fn root_only(mut td: Value) -> Value {
    for property in td["properties"].as_object_mut().unwrap().values_mut() {
        property["forms"] = json!([]);
    }
    td
}
fn prepared(
    td: &Value,
    compiler: &HttpDefinitionCompiler,
    accounts: &Arc<HostResourceAccounts>,
    content: Option<&Arc<PreparedConsumerContent>>,
    id: u32,
) -> CoreResult<Arc<ConsumerDefinition>> {
    prepared_input(
        td,
        compiler.registration(BindingId::new(id), BindingGeneration::INITIAL, 0)?,
        accounts,
        content,
    )
    .map(|(mapping, _)| mapping)
}
fn prepared_input(
    td: &Value,
    registration: ConsumerRegistrationInput,
    accounts: &Arc<HostResourceAccounts>,
    content: Option<&Arc<PreparedConsumerContent>>,
) -> CoreResult<(Arc<ConsumerDefinition>, usize)> {
    let registry = ConsumerRegistry::try_new(vec![registration], accounts)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let source: Thing = serde_json::from_value(td.clone()).unwrap();
    let mut jobs = 0;
    let drive = |job: &mut ConsumerCompilation<'_>| {
        jobs += 1;
        while !job.is_finished() {
            job.step(&mut work(), false);
        }
    };
    let output = if let Some(content) = content {
        registry.prepare_with_content(
            &source,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            content,
            &mut work(),
            drive,
        )
    } else {
        registry.prepare(
            &source,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            &mut work(),
            drive,
        )
    }?;
    let (bindings, funding) = output.into_execution();
    assert_eq!(bindings.len(), 1);
    let definition = bindings[0].definition().clone();
    drop(bindings);
    drop(funding);
    Ok((definition, jobs))
}
fn program(mapping: &ConsumerDefinition) -> HttpRequestDefinition {
    HttpRequestDefinition::from_prepared(mapping).expect("explicit HTTP definition")
}

#[test]
fn native_lowering_count_and_total_retained_cost_follow_definitions_and_instances() {
    for k in [1, 2] {
        for n in [1, 2, 8, 32] {
            if k > n {
                continue;
            }
            // Equal source documents, common semantic owner, policy and native
            // capacity in both controls. Account/fixture provisioning is excluded.
            let common_bank = bank();
            let initial = root_only(description(address()));
            let (common_mapping, _) = prepared_input(
                &initial,
                try_consumer_registration(
                    config(),
                    BindingId::new(98),
                    BindingGeneration::INITIAL,
                    0,
                )
                .unwrap(),
                &common_bank,
                None,
            )
            .unwrap();
            let content = common_mapping.content().clone();
            drop(common_mapping);
            let sources: Vec<_> = (0..n)
                .map(|i| {
                    let mut td = root_only(description(address()));
                    td["id"] = json!(format!("urn:meter:{i}"));
                    td["forms"][0]["htv:headers"] = json!([
                    {"htv:fieldName":"X-Model", "htv:fieldValue":format!("group-{}", i % k)}]);
                    td
                })
                .collect();
            let compilation_bank = bank();
            let mapping_bank = bank();
            let control_bank = bank();
            let shared_start = allocation_meter::read();
            let started = Instant::now();
            let compiler = HttpDefinitionCompiler::new(config(), compilation_bank.clone()).unwrap();
            let mut binders = Vec::new();
            let mut shared: Vec<Arc<ConsumerDefinition>> = Vec::new();
            for (i, td) in sources.iter().enumerate() {
                let mapping = if i < k {
                    let mapping =
                        prepared(td, &compiler, &mapping_bank, Some(&content), 98).unwrap();
                    binders.push(
                        HttpDefinitionCompiler::with_definition(
                            config(),
                            &program(&mapping),
                            mapping_bank.clone(),
                        )
                        .unwrap(),
                    );
                    mapping
                } else {
                    prepared(td, &binders[i % k], &mapping_bank, Some(&content), 98).unwrap()
                };
                if i >= k {
                    assert!(program(&shared[i % k]).shares_compilation_with(&program(&mapping)));
                }
                shared.push(mapping);
            }
            assert_eq!(compiler.native_compilations(), k);
            assert_eq!(
                binders
                    .iter()
                    .map(|b| b.native_compilations())
                    .sum::<usize>(),
                0
            );
            assert_eq!(
                binders.iter().map(|b| b.dependency_checks()).sum::<usize>(),
                n - k
            );
            assert_eq!(
                compiler.instance_bindings()
                    + binders.iter().map(|b| b.instance_bindings()).sum::<usize>(),
                n
            );
            // Runtime retained cost includes K instruction/input owners and N
            // mappings, after compilation-only owners and counters are destroyed.
            drop(compiler);
            drop(binders);
            let shared_elapsed = started.elapsed();
            let shared_end = allocation_meter::read();
            let shared_heap = shared_end.live - shared_start.live;
            let shared_reserved = live(&compilation_bank) + live(&mapping_bank);
            let definition_reserved = live(&compilation_bank);
            let control_start = allocation_meter::read();
            let started = Instant::now();
            let mut controls = Vec::new();
            let mut control_jobs = 0;
            for td in &sources {
                let (mapping, jobs) = prepared_input(
                    td,
                    try_consumer_registration(
                        config(),
                        BindingId::new(98),
                        BindingGeneration::INITIAL,
                        0,
                    )
                    .unwrap(),
                    &control_bank,
                    Some(&content),
                )
                .unwrap();
                control_jobs += jobs;
                controls.push(mapping);
            }
            let control_elapsed = started.elapsed();
            let control_end = allocation_meter::read();
            let control_heap = control_end.live - control_start.live;
            let control_reserved = live(&control_bank);
            assert_eq!(control_jobs, n);
            if n >= 8 {
                assert!(shared_heap < control_heap, "actual retained heap benefit");
            }
            assert!(
                definition_reserved < k as u64 * 4096,
                "no fixed 8 KiB dependency buffer"
            );
            drop(controls);
            assert_eq!(
                allocation_meter::read().live,
                control_start.live,
                "physical control release"
            );
            drop(shared);
            assert_eq!(
                allocation_meter::read().live,
                shared_start.live,
                "physical shared release"
            );
            clean(&mapping_bank);
            clean(&compilation_bank);
            clean(&control_bank);
            eprintln!(
                "HTTP K={k} N={n}: lowerings={k}/{control_jobs}, checks={}, shared/independent heap={shared_heap}/{control_heap}, reserved={shared_reserved}/{control_reserved}, definition reserved={definition_reserved}, preparation including compile={shared_elapsed:?}/{control_elapsed:?}, allocation calls={}/{}, requested bytes={}/{}",
                n - k,
                shared_end.allocations - shared_start.allocations,
                control_end.allocations - control_start.allocations,
                shared_end.allocated_bytes - shared_start.allocated_bytes,
                control_end.allocated_bytes - control_start.allocated_bytes
            );
            drop(content);
            clean(&common_bank);
        }
    }
}

#[test]
fn compiled_instructions_survive_independent_authority_policy_results_and_retirement() {
    io().block_on(async {
        let td_a = description(address());
        let td_b = description(address());
        let mut a_peer = Fixture::new(td_a.clone());
        let mut b_peer = Fixture::new(td_b.clone());
        *b_peer.app.value.borrow_mut() = br#"{"level":19,"on":false}"#.to_vec();
        let compilation_bank = bank();
        let mapping_bank_a = bank();
        let mapping_bank_b = bank();
        let compiler = HttpDefinitionCompiler::new(config(), compilation_bank.clone()).unwrap();
        let a = prepared(&root_only(td_a), &compiler, &mapping_bank_a, None, 98).unwrap();
        let definition = program(&a);
        let binder =
            HttpDefinitionCompiler::with_definition(config(), &definition, mapping_bank_b.clone())
                .unwrap();
        let b = prepared(
            &root_only(td_b),
            &binder,
            &mapping_bank_b,
            Some(a.content()),
            99,
        )
        .unwrap();
        assert!(definition.shares_compilation_with(&program(&b)));
        assert_ne!(a.target(), b.target());
        assert_ne!(a.binding(), b.binding());
        assert_eq!(
            (compiler.native_compilations(), binder.native_compilations()),
            (1, 0)
        );
        let (a_accounts, b_accounts) = (bank(), bank());
        let (a_owner, b_owner) = (consumer_instance(), consumer_instance());
        assert_ne!(a_owner.reference(), b_owner.reference());
        let a_denied = Arc::new(AtomicBool::new(true));
        let a_client = bind(a.clone(), &a_owner, &a_accounts, a_denied.clone());
        let b_client = bind(
            b.clone(),
            &b_owner,
            &b_accounts,
            Arc::new(AtomicBool::new(false)),
        );
        drop(a);
        drop(b);
        drop(definition);
        drop(compiler);
        drop(binder);
        let original_bytes = live(&compilation_bank);
        assert_eq!(
            compilation_bank
                .account(ResourceKind::BindingArtifactsGlobalMax)
                .used(),
            1
        );
        assert!(
            a_client
                .0
                .read_all_properties(&a_client.1, Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(a_peer.controls.requests.get(), 0);
        let mut b = start_read(&b_client);
        let (b_result, _) = b_peer.exchange(&mut b).await;
        assert_eq!(
            b_result.result().unwrap().data().unwrap().body,
            br#"{"level":19,"on":false}"#
        );
        assert_eq!(b_result.consumer(), &b_owner.reference());
        drop(b_result);
        drop(b);
        a_denied.store(false, Ordering::Relaxed);
        let mut a = start_read(&a_client);
        // Retain a real returned output before validation, then cancel and
        // revoke A. The original compilation must survive that late result.
        let started = Instant::now();
        while a_peer.app.roots.get() == 0 {
            a_peer.step();
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
            a_peer.step();
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
        let a_identity = a_owner.reference();
        a.cancel();
        drop(a_client);
        drop(a_owner);
        let (a_result, _) = a_peer.exchange(&mut a).await;
        assert_eq!(a_result.consumer(), &a_identity);
        assert!(a_result.result().is_err());
        assert!(matches!(
            a_result.native_outcome(),
            ConsumerNativeOutcome::Returned(Ok(_))
        ));
        drop(a);
        assert_eq!(live(&compilation_bank), original_bytes);
        let mut b = start_read(&b_client);
        let (b_result, _) = b_peer.exchange(&mut b).await;
        assert!(b_result.result().is_ok());
        drop(b);
        drop(b_client);
        drop(b_owner);
        assert_eq!(live(&compilation_bank), original_bytes);
        drop(a_result);
        clean(&a_accounts);
        assert!(
            live(&mapping_bank_a) > 0,
            "B retains A's common semantic content"
        );
        assert_eq!(
            live(&compilation_bank),
            original_bytes,
            "B's result retains the original compilation"
        );
        drop(b_result);
        clean(&mapping_bank_a);
        clean(&b_accounts);
        clean(&mapping_bank_b);
        clean(&compilation_bank);
        assert_eq!(
            (
                a_peer.controls.requests.get(),
                b_peer.controls.requests.get()
            ),
            (1, 2)
        );
        a_peer.close();
        b_peer.close();
    });
}

#[test]
fn changed_dependencies_refuse_before_lowering_and_fresh_compilation_can_recover() {
    let compiler_bank = bank();
    let mapping_bank = bank();
    let compiler = HttpDefinitionCompiler::new(config(), compiler_bank.clone()).unwrap();
    let mut original = root_only(description(address()));
    original["forms"][0]["htv:headers"] = json!([
        {"htv:fieldName":"X-Model", "htv:fieldValue":"old"}]);
    let mapping = prepared(&original, &compiler, &mapping_bank, None, 98).unwrap();
    let binder =
        HttpDefinitionCompiler::with_definition(config(), &program(&mapping), mapping_bank.clone())
            .unwrap();
    let mut aliased = original.clone();
    let form = aliased["forms"][0].as_object_mut().unwrap();
    let headers = form.remove("htv:headers").unwrap();
    form.insert(
        "@context".into(),
        json!({"h":"http://www.w3.org/2011/http#"}),
    );
    form.insert("h:headers".into(), headers);
    let equivalent = prepared(
        &aliased,
        &binder,
        &mapping_bank,
        Some(mapping.content()),
        99,
    )
    .unwrap();
    assert!(program(&mapping).shares_compilation_with(&program(&equivalent)));
    drop(equivalent);
    // These declarations affect neither support nor the normalized native
    // instructions. They must not enter a second, overbroad dependency key.
    let mut explicit_defaults = original.clone();
    explicit_defaults["forms"][0]["htv:methodName"] = json!("GET");
    explicit_defaults["forms"][0]["response"] = json!({
        "contentType":"application/json", "htv:statusCodeValue":200});
    explicit_defaults["forms"][0]["urn:vendor:annotation"] = json!("x".repeat(6000));
    let equivalent = prepared(
        &explicit_defaults,
        &binder,
        &mapping_bank,
        Some(mapping.content()),
        99,
    )
    .unwrap();
    assert!(program(&mapping).shares_compilation_with(&program(&equivalent)));
    drop(equivalent);
    let mut shifted = original.clone();
    shifted["forms"].as_array_mut().unwrap().insert(
        0,
        json!({"op":"readallproperties", "href":"ftp://127.0.0.1/unsupported"}),
    );
    let shifted = prepared(
        &shifted,
        &binder,
        &mapping_bank,
        Some(mapping.content()),
        99,
    )
    .unwrap();
    assert!(program(&mapping).shares_compilation_with(&program(&shifted)));
    assert_ne!(
        mapping.artifact().identity().plan_id(),
        shifted.artifact().identity().plan_id()
    );
    drop(shifted);
    let held = live(&mapping_bank);
    let mut changed = original.clone();
    changed["forms"][0]["htv:headers"][0]["htv:fieldValue"] = json!("new");
    assert!(
        prepared(
            &changed,
            &binder,
            &mapping_bank,
            Some(mapping.content()),
            99
        )
        .is_err()
    );
    assert_eq!(binder.native_compilations(), 0);
    assert_eq!(live(&mapping_bank), held);
    let new_mapping = prepared(
        &changed,
        &compiler,
        &mapping_bank,
        Some(mapping.content()),
        99,
    )
    .unwrap();
    assert_eq!(compiler.native_compilations(), 2);
    assert!(!program(&mapping).shares_compilation_with(&program(&new_mapping)));
    // Changed common assertions are independently checked, before native reuse.
    let mut schema = original.clone();
    schema["properties"]["level"]["minimum"] = json!(3);
    let before = binder.dependency_checks();
    assert!(prepared(&schema, &binder, &mapping_bank, Some(mapping.content()), 99).is_err());
    assert_eq!(binder.dependency_checks(), before);
    for (field, value) in [
        ("htv:methodName", json!("POST")),
        ("contentCoding", json!("gzip")),
        ("contentType", json!("text/plain")),
        ("subprotocol", json!("other")),
        ("htv:statusCodeValue", json!(200)), // unsupported at the Form itself
    ] {
        let mut td = original.clone();
        td["forms"][0][field] = value;
        assert!(
            prepared(&td, &binder, &mapping_bank, Some(mapping.content()), 99).is_err(),
            "{field}"
        );
        assert!(
            prepared(&td, &compiler, &mapping_bank, Some(mapping.content()), 99).is_err(),
            "fresh and reused extraction must reject the same unsupported {field}"
        );
    }
    for target in [
        "http://example.com/no-dns",
        "https://127.0.0.1/secure",
        "http://127.0.0.1:0/invalid",
        "http://user@127.0.0.1/path",
        "http://127.0.0.1/{item}",
    ] {
        let mut td = original.clone();
        td["forms"][0]["href"] = json!(target);
        assert!(
            prepared(&td, &binder, &mapping_bank, Some(mapping.content()), 99).is_err(),
            "{target}"
        );
    }
    assert!(
        HttpDefinitionCompiler::with_definition(
            config().with_target_capacity(128).unwrap(),
            &program(&mapping),
            mapping_bank.clone()
        )
        .is_err()
    );
    assert!(
        HttpDefinitionCompiler::with_definition(
            HttpConsumerConfig::for_https_ip().unwrap(),
            &program(&mapping),
            mapping_bank.clone()
        )
        .is_err()
    );
    assert_eq!(binder.native_compilations(), 0);
    drop(binder);
    drop(mapping);
    drop(new_mapping);
    drop(compiler);
    clean(&mapping_bank);
    clean(&compiler_bank);
}

#[test]
fn zero_credit_cancellation_and_capacity_refusal_preserve_original_definition() {
    let compilation_bank = bank();
    let mapping_bank = bank();
    let compiler = HttpDefinitionCompiler::new(config(), compilation_bank.clone()).unwrap();
    let td = root_only(description(address()));
    let mapping = prepared(&td, &compiler, &mapping_bank, None, 98).unwrap();
    let binder =
        HttpDefinitionCompiler::with_definition(config(), &program(&mapping), mapping_bank.clone())
            .unwrap();
    let registry = ConsumerRegistry::try_new(
        vec![
            binder
                .registration(BindingId::new(99), BindingGeneration::INITIAL, 0)
                .unwrap(),
        ],
        &mapping_bank,
    )
    .unwrap_or_else(|_| panic!());
    let original = live(&compilation_bank);
    let held = live(&mapping_bank);
    let source: Thing = serde_json::from_value(td).unwrap();
    for cancel in [false, true] {
        assert!(
            registry
                .prepare_with_content(
                    &source,
                    GatewayDefaultV1::LIMITS,
                    mapping_bank.clone(),
                    ConsumerSelector::default(),
                    mapping.content(),
                    &mut work(),
                    |job| {
                        job.step(&mut WorkBudget::new(), cancel);
                    }
                )
                .is_err()
        );
        assert_eq!(binder.dependency_checks(), 0);
        assert_eq!(live(&mapping_bank), held);
        assert_eq!(live(&compilation_bank), original);
    }
    let saturated = mapping_bank
        .reserve(
            ResourceKind::EngineLiveBytesGlobalMax,
            mapping_bank
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .limit()
                - held,
        )
        .unwrap();
    assert!(
        registry
            .prepare(
                &source,
                GatewayDefaultV1::LIMITS,
                mapping_bank.clone(),
                ConsumerSelector::default(),
                &mut work(),
                |_| panic!("no capacity before native callback")
            )
            .is_err()
    );
    drop(saturated);
    drop(registry);
    drop(binder);
    drop(mapping);
    drop(compiler);
    clean(&mapping_bank);
    clean(&compilation_bank);
}

fn leaf(addr: SocketAddr, path: &str) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"meter",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "properties":{"level":{"type":"integer", "readOnly":true,
            "forms":[{"href":format!("http://{addr}{path}"), "htv:methodName":"POST",
                "htv:headers":[{"htv:fieldName":"X-Model", "htv:fieldValue":"meter-v1"}],
                "additionalResponses":[{"success":false, "htv:statusCodeValue":409}]}]}}})
}

#[test]
fn physical_definition_capacity_is_acquired_before_lowering_and_refusal_is_retryable() {
    let compilation_bank = bank();
    let mapping_bank = bank();
    let compiler = HttpDefinitionCompiler::new(config(), compilation_bank.clone()).unwrap();
    let held = live(&compilation_bank);
    let saturated = compilation_bank
        .reserve(
            ResourceKind::EngineLiveBytesGlobalMax,
            compilation_bank
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .limit()
                - held,
        )
        .unwrap();
    let td = leaf(address(), "/value");
    assert!(prepared(&td, &compiler, &mapping_bank, None, 98).is_err());
    assert_eq!(compiler.native_compilations(), 0);
    clean(&mapping_bank);
    drop(saturated);
    assert_eq!(live(&compilation_bank), held);
    let mapping = prepared(&td, &compiler, &mapping_bank, None, 98).unwrap();
    assert_eq!(compiler.native_compilations(), 1);
    drop(mapping);
    drop(compiler);
    clean(&mapping_bank);
    clean(&compilation_bank);
}

#[test]
fn checked_binding_reuses_method_headers_and_response_discrimination_on_the_wire() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let peer = std::thread::spawn(move || {
        for path in ["/meter/a%2Fb?q=a%26b", "/meter/c%20d?q=x%3Fy"] {
            let started = Instant::now();
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(started.elapsed() < Duration::from_secs(5));
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|s| s == b"\r\n\r\n") {
                let mut bytes = [0; 1024];
                let n = socket.read(&mut bytes).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&bytes[..n]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(
                request.starts_with(&format!("POST {path} HTTP/1.1\r\n")),
                "{request}"
            );
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("\r\nx-model: meter-v1\r\n"),
                "{request}"
            );
            socket.write_all(b"HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nContent-Length: 1\r\nConnection: close\r\n\r\n7").unwrap();
        }
    });
    io().block_on(async {
        let compiler_bank = bank();
        let mapping_bank = bank();
        let compiler = HttpDefinitionCompiler::new(config(), compiler_bank.clone()).unwrap();
        let first_source = leaf(addr, "/meter/a%2Fb?q=a%26b");
        let first = prepared(&first_source, &compiler, &mapping_bank, None, 98).unwrap();
        let binder = HttpDefinitionCompiler::with_definition(
            config(),
            &program(&first),
            mapping_bank.clone(),
        )
        .unwrap();
        let second_source = leaf(addr, "/meter/c%20d?q=x%3Fy");
        let second = prepared(
            &second_source,
            &binder,
            &mapping_bank,
            Some(first.content()),
            99,
        )
        .unwrap();
        let mut another_supported_method = second_source.clone();
        another_supported_method["properties"]["level"]["forms"][0]["htv:methodName"] =
            json!("GET");
        assert!(prepared(&another_supported_method, &binder, &mapping_bank, None, 99).is_err());
        assert!(
            prepared_input(
                &another_supported_method,
                try_consumer_registration(
                    config(),
                    BindingId::new(99),
                    BindingGeneration::INITIAL,
                    0
                )
                .unwrap(),
                &mapping_bank,
                None
            )
            .is_ok(),
            "changed supported input can be freshly compiled"
        );
        for (field, value) in [
            ("htv:statusCodeValue", json!(410)),
            ("success", json!(true)),
            ("contentType", json!("application/json; charset=utf-8")),
        ] {
            let mut changed = second_source.clone();
            changed["properties"]["level"]["forms"][0]["additionalResponses"][0][field] = value;
            assert!(
                prepared(&changed, &binder, &mapping_bank, None, 99).is_err(),
                "{field}"
            );
        }
        assert_eq!(binder.native_compilations(), 0);
        for definition in [first, second] {
            let accounts = bank();
            let owner = consumer_instance();
            let client = bind(
                definition,
                &owner,
                &accounts,
                Arc::new(AtomicBool::new(false)),
            );
            let mut call = client
                .0
                .read_property(&client.1, "level", Deadline::NONE, now(0), &mut work())
                .unwrap();
            let started = Instant::now();
            let result = loop {
                if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) {
                    break result;
                }
                assert!(started.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            let output = result.result().unwrap();
            assert_eq!(output.status(), InteractionStatus::ApplicationError);
            assert_eq!(
                output.metadata().response_selection(),
                ResponseSelection::Additional(0)
            );
            assert_eq!(output.data().unwrap().body, b"7");
            drop(result);
            drop(call);
            drop(client);
            drop(owner);
            clean(&accounts);
        }
        assert_eq!(
            (compiler.native_compilations(), binder.instance_bindings()),
            (1, 1)
        );
        drop(compiler);
        drop(binder);
        clean(&mapping_bank);
        clean(&compiler_bank);
    });
    peer.join().unwrap();
}
