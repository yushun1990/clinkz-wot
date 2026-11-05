#![cfg(feature = "prepare")]
//! Thing-root ReadAllProperties over one native Engine-to-Engine HTTP request.
include!("support/collection.rs");

#[test]
fn engine_read_all_uses_one_root_request_and_keeps_individual_operations_distinct() {
    io().block_on(async {
        let addr = address();
        let mut td = description(addr);
        td["forms"][0]["htv:methodName"] = json!("GET");
        td["forms"][0]["response"] =
            json!({"contentType":"application/json", "htv:statusCodeValue":200});
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let mut call = start_read(&client);
        let (result, remote) = server.exchange(&mut call).await;
        assert_eq!(result.operation(), Operation::ReadAllProperties);
        assert_eq!(
            result.affordance(),
            clinkz_wot_core::thing::AffordanceSite::Thing
        );
        assert_eq!(result.consumer(), &owner.reference());
        assert_eq!(
            result.result().unwrap().data().unwrap().body.as_ref(),
            br#"{"level":7,"on":true}"#
        );
        assert_eq!(remote.target, Some(AffordanceTarget::Thing));
        assert_eq!(remote.recipient, Some(server.exposure.key()));
        assert_eq!(remote.operation, Some(Operation::ReadAllProperties));
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        assert_eq!(server.controls.requests.get(), 1);
        assert_eq!(server.app.roots.get(), 1);
        assert_eq!(server.app.reads.get(), 0);
        assert_eq!(server.app.writes.get(), 0);
        // Unread results retain original call capacity, mapping and schema.
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(result);
        drop(call);
        let mut single = client
            .read_property("level", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (result, _) = server.exchange(&mut single).await;
        assert_eq!(
            result.affordance(),
            clinkz_wot_core::thing::AffordanceSite::Property("level")
        );
        assert_eq!(result.result().unwrap().data().unwrap().body.as_ref(), b"7");
        assert_eq!(server.app.reads.get(), 1);
        assert_eq!(server.app.roots.get(), 1);
        drop(result);
        drop(single);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn review_collection_cleanup_work_is_actually_prepaid() {
    verify_collection_cleanup(Operation::ReadAllProperties);
}

#[test]
fn review_changed_validation_bound_cannot_run_unfunded_output_callback() {
    verify_collection_validator_bound(Operation::ReadAllProperties);
}

#[test]
fn producer_rejects_missing_wrong_duplicate_unknown_and_write_only_members_without_fanout() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let bodies: &[&[u8]] = &[
            br#"{"level":7}"#,
            br#"{"on":true}"#,
            br#"{"level":true,"on":true}"#,
            br#"{"level":1.5,"on":true}"#,
            br#"{"level":7,"on":true,"secret":"leak"}"#,
            br#"{"level":7,"on":true,"other":0}"#,
            br#"{"level":7,"level":8,"on":true}"#,
            br#"{"level":7,"\u006cevel":8,"on":true}"#,
            br#"{"level":7,"on":{"error":"failed"}}"#,
            br#"{"level":7,"on":true}{}"#,
            b"[]",
            b"null",
            b"{broken",
        ];
        for body in bodies {
            *server.app.value.borrow_mut() = body.to_vec();
            let mut call = start_read(&client);
            let (result, remote) = server.exchange(&mut call).await;
            assert!(result.result().is_err(), "{body:?}");
            assert!(
                matches!(remote.result, Err(CoreError::Validation(_))),
                "{body:?}"
            );
            assert_eq!(remote.target, Some(AffordanceTarget::Thing));
            drop(result);
            drop(call);
        }
        server.app.fails.set(true);
        let mut call = start_read(&client);
        let (result, remote) = server.exchange(&mut call).await;
        assert!(result.result().is_err());
        assert!(matches!(remote.result, Err(CoreError::Binding(_))));
        assert_eq!(server.app.roots.get(), bodies.len() + 1);
        assert_eq!(server.controls.requests.get(), bodies.len() + 1);
        assert_eq!(server.app.reads.get(), 0);
        drop(result);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn consumer_validates_independent_native_response_against_captured_member_schemas() {
    io().block_on(async {
        for (status, body, valid) in [
            (200, br#"{"on":true,"level":7.0}"#.as_slice(), true),
            (200, br#"{"level":70e-1,"on":true}"#, true),
            (200, br#"{"level":7e-100,"on":true}"#, false),
            (200, br#"{"level":7}"#, false),
            (200, br#"{"level":7,"on":true,"secret":"leak"}"#, false),
            (200, br#"{"level":7,"on":true,"on":false}"#, false),
            (200, br#"{"level":7,"on":true,"extra":null}"#, false),
            (200, br#"{"level":"7","on":true}"#, false),
            (200, br#"{"level":7,"on":{"error":"unavailable"}}"#, false),
            (206, br#"{"level":7,"on":true}"#, false),
            (207, br#"{"level":7,"on":true}"#, false),
            (500, br#"{"level":7,"on":true}"#, false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let response = body.to_vec();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new(); let mut buf = [0; 1024];
                while !bytes.ends_with(b"\r\n\r\n") {
                    let n = socket.read(&mut buf).unwrap(); assert!(n > 0); bytes.extend_from_slice(&buf[..n]);
                }
                assert!(bytes.starts_with(b"GET /properties HTTP/1.1\r\n"));
                let mut reply = format!("HTTP/1.1 {status} Reply\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).into_bytes();
                reply.extend_from_slice(&response);
                socket.write_all(&reply).unwrap();
                listener
            });
            let accounts = bank(); let owner = consumer_instance();
            let client = client(description(addr), Arc::new(AtomicBool::new(false)), accounts.clone(), &owner);
            let mut call = start_read(&client); let start = Instant::now();
            let result = loop {
                if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) { break result; }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(result.result().is_ok(), valid, "{body:?}");
            if !valid && status == 200 { assert_eq!(result.rejected_output().unwrap().data().unwrap().body.as_ref(), body); }
            drop(result); drop(call); drop(client); clean(&accounts);
            let listener = peer.join().unwrap(); listener.set_nonblocking(true).unwrap();
            assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        }
    });
}

#[test]
fn consumer_root_selection_requires_actual_form_binding_services_and_complete_schema_support() {
    let addr = address();
    for bad in [
        "absent",
        "https",
        "method",
        "schema",
        "security",
        "response",
        "multiple",
        "variables",
    ] {
        let mut td = description(addr);
        match bad {
            "absent" => {
                td.as_object_mut().unwrap().remove("forms");
            }
            "https" => td["forms"][0]["href"] = json!(format!("https://{addr}/properties")),
            "method" => td["forms"][0]["htv:methodName"] = json!("POST"),
            "schema" => td["properties"]["on"]["const"] = json!(true),
            "security" => {
                td["securityDefinitions"]["b"] = json!({"scheme":"basic"});
                td["forms"][0]["security"] = json!(["b"]);
            }
            "response" => td["forms"][0]["response"] = json!({"contentType":"text/plain"}),
            "multiple" => td["forms"][0]["op"] = json!("readmultipleproperties"),
            _ => {
                td["uriVariables"] = json!({"unit":{"type":"string"}});
                td["forms"][0]["href"] = json!(format!("http://{addr}/properties{{?unit}}"));
            }
        }
        let accounts = bank();
        if let Ok(p) = prepare_consumer(
            &td,
            accounts.clone(),
            vec![registration()],
            ConsumerSelector::default(),
        ) {
            assert!(
                !p.support()
                    .iter()
                    .any(|d| d.selected && d.operation == Operation::ReadAllProperties),
                "{bad}"
            );
            let owner = consumer_instance();
            let (d, f) = p.into_execution();
            let client = ConsumerRuntime::try_new(owner.reference(), d, f, accounts.clone(), 1)
                .unwrap_or_else(|_| panic!());
            assert!(
                client
                    .read_all_properties(Deadline::NONE, now(0), &mut work())
                    .is_err()
            );
            drop(client);
        }
        clean(&accounts);
        drop(TcpListener::bind(addr).unwrap());
    }
    let mut grouped_only = description(addr);
    for n in ["on", "level"] {
        grouped_only["properties"][n]["forms"][0]["href"] =
            json!(format!("mqtt://127.0.0.1:1883/{n}"));
    }
    grouped_only["properties"]["secret"]["const"] = json!("unsupported writable assertion");
    let accounts = bank();
    let p = prepare_consumer(
        &grouped_only,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let (defs, funding) = p.into_execution();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].affordance(), &AffordanceTarget::Thing);
    drop(defs);
    drop(funding);
    clean(&accounts);
    let mut td = description(addr);
    let supported = td["forms"][0].clone();
    td["forms"] =
        json!([{"op":"readallproperties","href":format!("https://{addr}/bad")}, supported]);
    let accounts = bank();
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let root = p
        .support()
        .iter()
        .find(|d| d.selected && d.operation == Operation::ReadAllProperties)
        .unwrap();
    assert_eq!(root.form, 1);
    assert_eq!(root.site, FormSite::Thing);
    drop(p);
    clean(&accounts);
    let other = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(99),
        BindingGeneration::INITIAL,
        1,
    )
    .unwrap();
    assert!(
        prepare_consumer(
            &td,
            accounts.clone(),
            vec![registration(), other],
            ConsumerSelector::default()
        )
        .is_err()
    );
    clean(&accounts);
    // An explicit Form selector cannot reinterpret an unsupported root Form.
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector {
            form: Some(0),
            binding: None,
        },
    )
    .unwrap();
    assert!(
        !p.support()
            .iter()
            .any(|d| d.selected && d.operation == Operation::ReadAllProperties)
    );
    drop(p);
    clean(&accounts);
}

#[test]
fn full_producer_qualification_requires_root_behavior_every_form_and_original_member_meaning() {
    let addr = address();
    let td = description(addr);
    let key = authority(Generation::INITIAL).key();
    let provider = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        BindingId::new(97),
        BindingGeneration::INITIAL,
        1000,
    )
    .unwrap();
    assert!(
        prepare_thing(
            &td.to_string(),
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |scope, _| scope != ValidatedProducerScope::Thing
        )
        .is_err()
    );
    for fault in [
        "schema",
        "root-op",
        "root-protocol",
        "property",
        "root-response",
        "root-method",
    ] {
        let mut bad = td.clone();
        match fault {
            "schema" => bad["properties"]["on"]["const"] = json!(true),
            "root-op" => {
                bad["forms"][0]["op"] = json!(["readallproperties", "observeallproperties"])
            }
            "root-protocol" => bad["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"op":"readallproperties","href":"mqtt://127.0.0.1:1883/group"})),
            "property" => {
                bad["properties"]["level"]["forms"][0]["href"] = json!("https://127.0.0.1/level")
            }
            "root-response" => {
                bad["forms"][0]["response"] =
                    json!({"contentType":"application/json", "htv:statusCodeValue":207})
            }
            _ => bad["forms"][0]["htv:methodName"] = json!("POST"),
        }
        assert!(
            prepare_thing(
                &bad.to_string(),
                key,
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |_, _| true
            )
            .is_err(),
            "{fault}"
        );
        drop(TcpListener::bind(addr).unwrap());
    }
    let prepared = prepare_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    for fault in ["schema", "writeOnly", "new-member"] {
        let mut changed = td.clone();
        match fault {
            "schema" => changed["properties"]["level"]["type"] = json!("string"),
            "writeOnly" => {
                changed["properties"]["level"]["readOnly"] = json!(false);
                changed["properties"]["level"]["writeOnly"] = json!(true);
            }
            _ => {
                changed["properties"]["extra"] = json!({"type":"null", "readOnly":true,"forms":[{"href":format!("http://{addr}/extra")}]})
            }
        }
        assert!(
            check_final_thing(
                &changed.to_string(),
                key,
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                &prepared,
                |_, _| true
            )
            .is_err(),
            "{fault}"
        );
    }
    assert!(
        check_final_thing(
            &td.to_string(),
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            &prepared,
            |_, _| true
        )
        .is_ok()
    );
    // Same native GET path is a collision even though logical operations differ.
    let mut same = td.clone();
    same["properties"]["level"]["forms"][0]["href"] = td["forms"][0]["href"].clone();
    let p = prepare_thing(
        &same.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    let mappings = p
        .operations()
        .iter()
        .map(|o| {
            (
                match &o.site {
                    FormSite::Thing => AffordanceTarget::Thing,
                    FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
                    _ => unreachable!(),
                },
                o.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    assert!(
        HttpFiniteConfig::try_from_mappings(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            mappings,
            key,
            1000
        )
        .is_err()
    );
    drop(TcpListener::bind(addr).unwrap());
}

#[test]
fn root_qualification_cannot_be_satisfied_by_only_individual_native_mappings() {
    let addr = address();
    let td = description(addr);
    let accounts = bank();
    let owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 4,
        target_name_bytes: 256,
    })
    .unwrap();
    let a = authority(Generation::INITIAL);
    let key = a.key();
    let provider: Rc<dyn ProducerCapability> = Rc::new(
        HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap(),
    );
    let prepared = prepare_thing(
        &td.to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[provider.as_ref()],
        |_, _| true,
    )
    .unwrap();
    let mappings = prepared
        .operations()
        .iter()
        .filter(|o| o.site != FormSite::Thing)
        .map(|o| {
            let FormSite::Property(n) = &o.site else {
                unreachable!()
            };
            (
                AffordanceTarget::Property(Arc::from(n.as_str())),
                o.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    let config = HttpFiniteConfig::try_from_mappings(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        mappings,
        key,
        1000,
    )
    .unwrap_or_else(|_| panic!());
    let server = RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(
            config.clone(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
        ),
        config,
        &owner,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 4096,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .unwrap_or_else(|_| panic!());
    let app = Rc::new(Application::default());
    let instance = RawProducerInstance::new(
        a,
        vec![
            RawFiniteBehavior::new(
                AffordanceTarget::Thing,
                Operation::ReadAllProperties,
                Behavior(app.clone()),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Behavior(app.clone()),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("on")),
                Operation::ReadProperty,
                Behavior(app.clone()),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("secret")),
                Operation::WriteProperty,
                Behavior(app.clone()),
            ),
        ],
    )
    .with_description(Arc::from(td.to_string()));
    let returned = ComposedProducerExposure::try_new(
        &owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![provider],
            participants: vec![Box::new(server)],
            expected: None,
            handler_backing_bytes: 8192,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
        &mut work(),
    )
    .err()
    .unwrap()
    .into_input();
    assert!(owner.description(key).is_none());
    assert_eq!(returned.prepared.operations().len(), 4);
    drop(returned);
    clean(&accounts);
    drop(TcpListener::bind(addr).unwrap());
    assert_eq!(app.roots.get(), 0);
    assert_eq!(app.drops.get(), 4);
}

#[test]
fn cancellation_replacement_deadline_and_policy_loss_retain_original_root_owners() {
    verify_collection_lifecycle(Operation::ReadAllProperties);
}

#[test]
fn producer_replacement_preserves_original_root_attribution_before_and_after_dispatch() {
    verify_collection_replacement(Operation::ReadAllProperties);
}

// The actual selected service can refuse collection meaning independently of
// scalar type support, or disappear during its own validation callback.
struct Data {
    inner: Arc<dyn ConsumerDataService>,
    available: Arc<AtomicBool>,
    maps: bool,
    lose_on_validate: bool,
}
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        self.inner.content_type()
    }
    fn validate(
        &self,
        schema: clinkz_wot_core::value_type::TypeSchema,
        p: &Payload,
    ) -> CoreResult<()> {
        self.inner.validate(schema, p)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut p = self.inner.policy();
        p.retained_bytes += 128;
        p
    }
    fn available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
    fn supports_property_map(
        &self,
        schema: &clinkz_wot_core::collection::PropertyCollectionSchema,
    ) -> bool {
        self.maps && self.inner.supports_property_map(schema)
    }
    fn validate_property_map(
        &self,
        schema: &clinkz_wot_core::collection::PropertyCollectionSchema,
        p: &Payload,
    ) -> CoreResult<()> {
        let result = self.inner.validate_property_map(schema, p);
        if self.lose_on_validate {
            self.available.store(false, Ordering::Relaxed);
        }
        result
    }
}

#[test]
fn captured_collection_service_is_required_and_freshly_checked_after_validation() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let accounts = bank();
        let mut reg = registration();
        reg.services.data = Arc::new(Data {
            inner: reg.services.data.clone(),
            available: Arc::new(AtomicBool::new(true)),
            maps: false,
            lose_on_validate: false,
        });
        let prepared = prepare_consumer(
            &td,
            accounts.clone(),
            vec![reg],
            ConsumerSelector::default(),
        )
        .unwrap();
        assert!(
            !prepared
                .support()
                .iter()
                .any(|s| s.selected && s.operation == Operation::ReadAllProperties)
        );
        drop(prepared);
        clean(&accounts);
        let mut server = Fixture::new(td.clone());
        let available = Arc::new(AtomicBool::new(true));
        let mut reg = registration();
        reg.services.data = Arc::new(Data {
            inner: reg.services.data.clone(),
            available: available.clone(),
            maps: true,
            lose_on_validate: true,
        });
        let prepared = prepare_consumer(
            &td,
            accounts.clone(),
            vec![reg],
            ConsumerSelector::default(),
        )
        .unwrap();
        let owner = consumer_instance();
        let (d, f) = prepared.into_execution();
        let client = ConsumerRuntime::try_new(owner.reference(), d, f, accounts.clone(), 1)
            .unwrap_or_else(|_| panic!());
        let mut call = start_read(&client);
        let (local, remote) = server.exchange(&mut call).await;
        assert!(!available.load(Ordering::Relaxed));
        assert!(local.result().is_err());
        assert!(matches!(
            local.native_outcome(),
            ConsumerNativeOutcome::Returned(Ok(_))
        ));
        assert!(remote.result.is_ok());
        assert_eq!(server.app.roots.get(), 1);
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn preparation_saturation_and_unpaid_member_traversal_preserve_all_owners_without_effects() {
    io().block_on(async {
        let addr = address();
        let td = description(addr);
        let accounts = bank();
        let registry = ConsumerRegistry::try_new(vec![registration()]).unwrap_or_else(|_| panic!());
        let thing: Thing = serde_json::from_value(td.clone()).unwrap();
        assert!(
            registry
                .prepare(
                    &thing,
                    GatewayDefaultV1::LIMITS,
                    accounts.clone(),
                    ConsumerSelector::default(),
                    &mut WorkBudget::new(),
                    |_| panic!("unpaid native compiler")
                )
                .is_err()
        );
        clean(&accounts);
        drop(registry);
        drop(thing);
        let owner = consumer_instance();
        let client = client(
            td.clone(),
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let before = ResourceKind::ALL.map(|k| accounts.account(k).used());
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut WorkBudget::new())
                .is_err()
        );
        assert_eq!(
            before,
            ResourceKind::ALL.map(|k| accounts.account(k).used())
        );
        let a = accounts.account(ResourceKind::EngineLiveBytesGlobalMax);
        let saturated = accounts
            .reserve(ResourceKind::EngineLiveBytesGlobalMax, a.limit() - a.used())
            .unwrap();
        assert!(
            client
                .read_all_properties(Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(saturated);
        assert_eq!(
            before,
            ResourceKind::ALL.map(|k| accounts.account(k).used())
        );
        // Refused starts have not acquired a native listener/connection.
        let mut server = Fixture::new(td);
        let mut call = start_read(&client);
        let start = Instant::now();
        while server.server().state() != RawFiniteState::Dispatching {
            server.step();
            assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        for _ in 0..3 {
            let mut short = work().with_remaining(WorkClass::HandlerSteps, 20_000);
            assert!(
                server
                    .owner
                    .step(
                        server.exposure.participant_mut::<Server>(0).unwrap(),
                        &mut cx(),
                        &mut short,
                        now(0)
                    )
                    .unwrap()
                    .is_pending()
            );
            assert_eq!(server.app.roots.get(), 0);
        }
        // Once admitted, an unread native collection result waits for the whole
        // captured validation allowance; small credits do not invoke it piecemeal.
        for _ in 0..20 {
            server.step();
            assert!(
                call.poll(
                    &mut cx(),
                    now(0),
                    &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
                )
                .is_pending()
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(server.controls.requests.get(), 1);
        assert_eq!(server.app.roots.get(), 1);
        let (result, remote) = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        assert!(remote.result.is_ok());
        drop(client);
        assert!(
            accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop(result);
        drop(call);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn prepublication_ingress_and_lost_producer_validation_cannot_dispatch_root_behavior() {
    verify_collection_publication(Operation::ReadAllProperties);
}

#[test]
fn root_authorization_refuses_before_effects_without_individual_fallback() {
    verify_collection_authorization(Operation::ReadAllProperties);
}
