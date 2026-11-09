#![cfg(feature = "prepare")]
#![allow(dead_code)]
//! Structured DataSchema through actual qualified Engine HTTP execution.
include!("support/collection.rs");

fn structured_description(addr: SocketAddr) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"structured configuration",
    "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},"forms":[],
    "properties":{"on":{"type":"object","readOnly":false,
        "properties":{"config":{"type":"object","properties":{
            "level":{"type":"integer","minimum":1,"exclusiveMaximum":100},
            "label":{"type":"string","minLength":1,"maxLength":2},
            "enabled":{"type":"boolean"}},"required":["level","label"]}},
        "required":["config"],"forms":[{"href":format!("http://{addr}/on")}]
    }}})
}
fn valid(level: u8) -> Vec<u8> {
    serde_json::to_vec(&json!({"config":{"level":level,"label":"ok"}})).unwrap()
}
fn mappings(
    td: &Value,
    backing: &Arc<HostResourceAccounts>,
    reuse: Option<&Arc<PreparedConsumerContent>>,
) -> Vec<Arc<ConsumerDefinition>> {
    let registry = ConsumerRegistry::try_new(vec![registration()], backing)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let source: Thing = serde_json::from_value(td.clone()).unwrap();
    let drive = |job: &mut ConsumerCompilation<'_>| {
        while !job.is_finished() {
            job.step(&mut work(), false);
        }
    };
    let p = if let Some(content) = reuse {
        let p = registry
            .prepare_with_content(
                &source,
                GatewayDefaultV1::LIMITS,
                backing.clone(),
                ConsumerSelector::default(),
                content,
                &mut work(),
                drive,
            )
            .unwrap();
        assert_eq!(
            p.content_constructions(),
            0,
            "qualification must not reconstruct immutable constraints"
        );
        p
    } else {
        registry
            .prepare(
                &source,
                GatewayDefaultV1::LIMITS,
                backing.clone(),
                ConsumerSelector::default(),
                &mut work(),
                drive,
            )
            .unwrap()
    };
    let (bindings, funding) = p.into_execution();
    let result = bindings.iter().map(|b| b.definition().clone()).collect();
    drop(bindings);
    drop(funding);
    result
}
fn bind_mappings(
    definitions: Vec<Arc<ConsumerDefinition>>,
    owner: &ConsumerInstance,
    bank: &Arc<HostResourceAccounts>,
    denied: Arc<AtomicBool>,
) -> (ConsumerAssociation, ConsumerCalls) {
    let access = ConsumerServiceOwner::retain_access(Arc::new(Access(denied)), bank)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let bindings = definitions
        .into_iter()
        .map(|d| {
            d.bind(access.clone(), bank.clone(), &mut work())
                .unwrap_or_else(|e| panic!("{:?}", e.error()))
        })
        .collect();
    ConsumerAssociation::try_new(owner.reference(), bindings, vec![], bank.clone())
        .map(|association| {
            (
                association,
                ConsumerCalls::try_new(bank.clone(), 1).unwrap(),
            )
        })
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn live(bank: &HostResourceAccounts) -> u64 {
    bank.account(ResourceKind::EngineLiveBytesGlobalMax).used()
}
fn value_schema(content: &PreparedConsumerContent) -> &data_schema::PreparedSchema {
    let ConsumerFiniteSchemas::Value(s) = content.schemas() else {
        panic!()
    };
    s.input.as_ref().or(s.output.as_ref()).unwrap()
}

#[test]
fn shared_constraints_survive_source_retirement_and_keep_instances_independent() {
    io().block_on(async {
        let td_a = structured_description(address());
        let mut td_b = structured_description(address());
        td_b["properties"]["on"]["properties"]["config"]["required"] = json!(["label", "level"]);
        let mut a = Fixture::new(td_a.clone());
        let mut b = Fixture::new(td_b.clone());
        *a.app.scalar.borrow_mut() = Some(valid(10));
        *b.app.scalar.borrow_mut() = Some(valid(20));
        let backing_a = bank();
        let backing_b = bank();
        let accounts_a = bank();
        let accounts_b = bank();
        let definitions_a = mappings(&td_a, &backing_a, None);
        assert_eq!(definitions_a.len(), 2);
        let mut definitions_b = Vec::new();
        let mut weak = Vec::new();
        for d in &definitions_a {
            assert!(value_schema(d.content()).retained_bytes() > 0);
            let mut second = mappings(&td_b, &backing_b, Some(d.content()));
            assert_eq!(second.len(), 1);
            assert!(Arc::ptr_eq(d.content(), second[0].content()));
            let data_schema::PreparedSchema::Object(schema) = value_schema(d.content()) else {
                panic!()
            };
            let data_schema::PreparedSchema::Object(other) = value_schema(second[0].content())
            else {
                panic!()
            };
            assert_eq!(schema.properties().as_ptr(), other.properties().as_ptr());
            assert_eq!(
                schema.properties()[0].name.as_ptr(),
                other.properties()[0].name.as_ptr()
            );
            weak.push((d.operation(), Arc::downgrade(d.content())));
            definitions_b.append(&mut second);
        }
        drop(td_a);
        drop(td_b);
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        assert_eq!(owner_a.reference().slot(), owner_b.reference().slot());
        assert_ne!(owner_a.reference(), owner_b.reference());
        let denied_a = Arc::new(AtomicBool::new(false));
        let client_a = bind_mappings(definitions_a, &owner_a, &accounts_a, denied_a.clone());
        let client_b = bind_mappings(
            definitions_b,
            &owner_b,
            &accounts_b,
            Arc::new(AtomicBool::new(false)),
        );
        let invalid = payload(br#"{"config":{"level":100,"label":"ok"}}"#);
        let original = invalid.body.clone();
        let rejected = client_a
            .0
            .write_property(
                &client_a.1,
                "on",
                invalid,
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .err()
            .unwrap()
            .into_input();
        assert!(Arc::ptr_eq(&original, &rejected.body));
        drop(rejected);
        drop(original);
        assert_eq!(a.app.writes.get(), 0);
        assert_eq!(a.controls.requests.get(), 0);
        let mut write = client_a
            .0
            .write_property(
                &client_a.1,
                "on",
                payload(&valid(42)),
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let (written, produced) = a.exchange(&mut write).await;
        assert!(written.result().is_ok());
        assert!(produced.result.is_ok());
        drop(written);
        drop(produced);
        drop(write);
        assert_eq!(a.app.writes.get(), 1);
        assert_eq!(a.app.scalar.borrow().as_ref().unwrap(), &valid(42));
        assert_eq!(b.app.writes.get(), 0);
        assert_eq!(b.app.scalar.borrow().as_ref().unwrap(), &valid(20));
        denied_a.store(true, Ordering::Relaxed);
        assert!(
            client_a
                .0
                .read_property(&client_a.1, "on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        let mut read_b = client_b
            .0
            .read_property(&client_b.1, "on", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (result_b, producer_b) = b.exchange(&mut read_b).await;
        assert_eq!(result_b.result().unwrap().data().unwrap().body, valid(20));
        drop(result_b);
        drop(producer_b);
        drop(read_b);
        denied_a.store(false, Ordering::Relaxed);
        let mut read_a = client_a
            .0
            .read_property(&client_a.1, "on", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (result_a, producer_a) = a.exchange(&mut read_a).await;
        assert_eq!(result_a.result().unwrap().data().unwrap().body, valid(42));
        drop(producer_a);
        drop(read_a);
        drop(client_a);
        drop(owner_a);
        drop(client_b);
        drop(owner_b);
        assert!(live(&accounts_a) > 0 && live(&backing_a) > 0);
        assert!(
            weak.iter()
                .all(|(operation, w)| w.upgrade().is_some()
                    == (*operation == Operation::ReadProperty))
        );
        drop(result_a);
        assert!(weak.iter().all(|(_, w)| w.upgrade().is_none()));
        for bank in [&backing_a, &backing_b, &accounts_a, &accounts_b] {
            clean(bank);
        }
        a.close();
        b.close();
    });
}

fn raw_request(server: &mut Fixture, body: &[u8]) -> RawFiniteSettlement {
    let mut peer = TcpStream::connect(server.addr).unwrap();
    write!(peer,"PUT /on HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",server.addr,body.len()).unwrap();
    peer.write_all(body).unwrap();
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    peer.set_nonblocking(true).unwrap();
    let start = Instant::now();
    loop {
        server.step();
        if let Some(s) = server.server().take_settlement() {
            return s;
        }
        let mut buffer = [0; 4096];
        let _ = peer.read(&mut buffer);
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}

#[test]
fn producer_rejects_invalid_structured_input_before_permission_and_application_effects() {
    io().block_on(async {
        for invalid in [
            br#"{}"#.as_slice(),
            br#"{"config":{"level":0,"label":"ok"}}"#,
            br#"{"config":{"level":7,"label":"too long"}}"#,
        ] {
            let mut server = Fixture::new(structured_description(address()));
            *server.app.scalar.borrow_mut() = Some(valid(10));
            let result = raw_request(&mut server, invalid);
            assert!(result.result.is_err());
            assert_eq!(result.effect, FiniteEffect::NotInvoked);
            assert_eq!(server.app.authorizations.get(), 0);
            assert_eq!(server.app.writes.get(), 0);
            assert_eq!(server.app.scalar.borrow().as_ref().unwrap(), &valid(10));
            drop(result);
            server.close();
        }
    });
}

#[test]
fn producer_validates_declared_nested_output_before_delivery() {
    io().block_on(async {
        let td = structured_description(address());
        let mut server = Fixture::new(td.clone());
        *server.app.scalar.borrow_mut() =
            Some(br#"{"config":{"level":100,"label":"ok"}}"#.to_vec());
        let backing = bank();
        let accounts = bank();
        let owner = consumer_instance();
        let client = bind_mappings(
            mappings(&td, &backing, None),
            &owner,
            &accounts,
            Arc::new(AtomicBool::new(false)),
        );
        let mut call = client
            .0
            .read_property(&client.1, "on", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let (result, produced) = server.exchange(&mut call).await;
        assert_eq!(server.app.reads.get(), 1);
        assert!(produced.result.is_err());
        assert!(result.result().is_err());
        drop(result);
        drop(produced);
        drop(call);
        drop(client);
        drop(owner);
        clean(&accounts);
        clean(&backing);
        server.close();
    });
}

struct TypeOnly(Arc<dyn ConsumerDataService>);
impl ConsumerDataService for TypeOnly {
    fn content_type(&self) -> &str {
        self.0.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut p = self.0.policy();
        p.retained_bytes += 64;
        p
    }
    fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
        self.0.validate(s, p)
    }
    // The default supports_schema only qualifies primitive type assertions.
}

#[test]
fn changed_constraints_and_incompatible_actual_validator_refuse_preparation() {
    let td = structured_description(address());
    let original_bank = bank();
    let original = mappings(&td, &original_bank, None);
    let content = original[0].content();
    for change in ["bound", "required", "type", "unsupported", "validator"] {
        let accounts = bank();
        let mut source = td.clone();
        let mut input = registration();
        match change {
            "bound" => {
                source["properties"]["on"]["properties"]["config"]["properties"]["level"]["minimum"] =
                    json!(2)
            }
            "required" => {
                source["properties"]["on"]["properties"]["config"]["required"] =
                    json!(["level", "label", "enabled"])
            }
            "type" => {
                source["properties"]["on"]["properties"]["config"]["properties"]["enabled"]["type"] =
                    json!("string")
            }
            "unsupported" => {
                source["properties"]["on"]["properties"]["config"]["properties"]["level"]["multipleOf"] =
                    json!(2)
            }
            "validator" => input.services.data = Arc::new(TypeOnly(input.services.data)),
            _ => unreachable!(),
        }
        let registry = ConsumerRegistry::try_new(vec![input], &accounts)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let source: Thing = serde_json::from_value(source).unwrap();
        assert!(
            registry
                .prepare_with_content(
                    &source,
                    GatewayDefaultV1::LIMITS,
                    accounts.clone(),
                    ConsumerSelector::default(),
                    content,
                    &mut work(),
                    |job| while !job.is_finished() {
                        job.step(&mut work(), false);
                    }
                )
                .is_err(),
            "{change}"
        );
        drop(source);
        drop(registry);
        clean(&accounts);
    }
    drop(original);
    clean(&original_bank);
}

#[test]
fn unsupported_nested_assertions_are_not_weakened_for_either_role() {
    for assertion in [
        json!({"pattern":"a"}),
        json!({"format":"email"}),
        json!({"const":"ok"}),
        json!({"enum":["ok"]}),
        json!({"contentEncoding":"base64"}),
        json!({"oneOf":[{"type":"string"}]}),
        json!({"type":"number","minimum":0.1}),
        json!({"type":"integer","multipleOf":2}),
        json!({"type":"array","items":{"type":"integer"}}),
        json!({"type":"object","additionalProperties":false}),
    ] {
        let mut td = structured_description(address());
        let label = &mut td["properties"]["on"]["properties"]["config"]["properties"]["label"];
        if assertion.get("type").is_some() {
            *label = assertion;
        } else {
            label
                .as_object_mut()
                .unwrap()
                .extend(assertion.as_object().unwrap().clone());
        }
        let accounts = bank();
        let registry = ConsumerRegistry::try_new(vec![registration()], &accounts)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let source: Thing = serde_json::from_value(td.clone()).unwrap();
        assert!(
            registry
                .prepare(
                    &source,
                    GatewayDefaultV1::LIMITS,
                    accounts.clone(),
                    ConsumerSelector::default(),
                    &mut work(),
                    |j| while !j.is_finished() {
                        j.step(&mut work(), false);
                    }
                )
                .is_err()
        );
        let addr = td["properties"]["on"]["forms"][0]["href"]
            .as_str()
            .unwrap()
            .strip_prefix("http://")
            .unwrap()
            .strip_suffix("/on")
            .unwrap()
            .parse()
            .unwrap();
        let producer = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap();
        assert!(
            prepare_thing(
                &td.to_string(),
                authority(Generation::INITIAL).key(),
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&producer],
                |_, _| true
            )
            .is_err()
        );
        drop(registry);
        drop(source);
        clean(&accounts);
        drop(TcpListener::bind(addr).unwrap());
    }
}

#[test]
fn consumer_rejects_invalid_wire_values_and_preserves_late_constraint_evidence() {
    io().block_on(async {
        for mode in ["normal", "cancel", "replace"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let sent = Arc::new(AtomicBool::new(false));
            let done = sent.clone();
            let wire = std::thread::spawn(move || {
                let (mut peer, _) = listener.accept().unwrap();
                peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request = Vec::new();
                let mut bytes = [0; 512];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = peer.read(&mut bytes).unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&bytes[..n]);
                }
                let body = br#"{"config":{"level":100,"label":"ok"}}"#;
                write!(
                    peer,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                peer.write_all(body).unwrap();
                done.store(true, Ordering::Release);
            });
            let td = structured_description(addr);
            let backing = bank();
            let accounts = bank();
            let owner = consumer_instance();
            let definitions = mappings(&td, &backing, None);
            let content = Arc::downgrade(
                definitions
                    .iter()
                    .find(|d| d.operation() == Operation::ReadProperty)
                    .unwrap()
                    .content(),
            );
            let client = bind_mappings(
                definitions,
                &owner,
                &accounts,
                Arc::new(AtomicBool::new(false)),
            );
            drop(td);
            let mut call = client.0.read_property(&client.1, "on", Deadline::NONE, now(0), &mut work())
                .unwrap();
            let start = Instant::now();
            while !sent.load(Ordering::Acquire) {
                assert!(
                    call.poll(
                        &mut cx(),
                        now(0),
                        &mut work().with_remaining(WorkClass::CodecInputBytes, 0),
                    )
                    .is_pending()
                );
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            for _ in 0..4 {
                assert!(
                    call.poll(
                        &mut cx(),
                        now(0),
                        &mut work().with_remaining(WorkClass::CodecInputBytes, 0),
                    )
                    .is_pending()
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            match mode {
                "cancel" => call.cancel(),
                "replace" => owner.revoke(),
                _ => {}
            }
            let settlement = loop {
                if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            wire.join().unwrap();
            assert!(settlement.result().is_err(), "{mode}");
            assert!(
                settlement.rejected_output().is_some(),
                "original schema must validate normal and late output: {mode}"
            );
            if mode == "cancel" {
                assert!(matches!(
                    settlement.first_cause(),
                    Some(CoreError::Cancelled(_))
                ));
            }
            drop(call);
            drop(client);
            drop(owner);
            assert!(content.upgrade().is_some());
            assert!(live(&backing) > 0 && live(&accounts) > 0);
            drop(settlement);
            assert!(content.upgrade().is_none());
            clean(&backing);
            clean(&accounts);
        }
    });
}

#[test]
fn structured_collection_content_reuses_nested_constraints_without_leaf_fallback() {
    io().block_on(async {
        let make = |addr| {
            let mut td = structured_description(addr);
            td["forms"] =
                json!([{"op":"readallproperties","href":format!("http://{addr}/properties")}]);
            td["properties"]["on"]["readOnly"] = json!(true);
            td["properties"]["on"]["forms"] = json!([]);
            td
        };
        let td_a = make(address());
        let td_b = make(address());
        let mut a = Fixture::new(td_a.clone());
        let mut b = Fixture::new(td_b.clone());
        *a.app.value.borrow_mut() =
            serde_json::to_vec(&json!({"on":serde_json::from_slice::<Value>(&valid(7)).unwrap()}))
                .unwrap();
        *b.app.value.borrow_mut() =
            serde_json::to_vec(&json!({"on":serde_json::from_slice::<Value>(&valid(8)).unwrap()}))
                .unwrap();
        let backing_a = bank();
        let backing_b = bank();
        let accounts_a = bank();
        let accounts_b = bank();
        let first = mappings(&td_a, &backing_a, None);
        let second = mappings(&td_b, &backing_b, Some(first[0].content()));
        assert!(Arc::ptr_eq(first[0].content(), second[0].content()));
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        let client_a = bind_mappings(
            first,
            &owner_a,
            &accounts_a,
            Arc::new(AtomicBool::new(false)),
        );
        let client_b = bind_mappings(
            second,
            &owner_b,
            &accounts_b,
            Arc::new(AtomicBool::new(false)),
        );
        drop(td_a);
        drop(td_b);
        let mut call_a = start_read(&client_a);
        let mut call_b = start_read(&client_b);
        let (ra, pa) = a.exchange(&mut call_a).await;
        let (rb, pb) = b.exchange(&mut call_b).await;
        assert!(ra.result().is_ok() && rb.result().is_ok());
        assert_eq!(a.app.roots.get(), 1);
        assert_eq!(b.app.roots.get(), 1);
        assert_eq!(a.app.reads.get() + b.app.reads.get(), 0);
        drop(ra);
        drop(rb);
        drop(pa);
        drop(pb);
        drop(call_a);
        drop(call_b);
        drop(client_a);
        drop(client_b);
        drop(owner_a);
        drop(owner_b);
        for bank in [&backing_a, &backing_b, &accounts_a, &accounts_b] {
            clean(bank);
        }
        a.close();
        b.close();
    });
}

#[test]
fn structured_preparation_cancellation_and_binding_refusal_preserve_funded_owners() {
    let td = structured_description(address());
    let accounts = bank();
    let registry = ConsumerRegistry::try_new(vec![registration()], &accounts)
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let source: Thing = serde_json::from_value(td.clone()).unwrap();
    let cancelled = registry.prepare(
        &source,
        GatewayDefaultV1::LIMITS,
        accounts.clone(),
        ConsumerSelector::default(),
        &mut work(),
        |job| {
            job.step(&mut WorkBudget::new(), true);
        },
    );
    assert!(matches!(cancelled, Err(CoreError::Cancelled(_))));
    drop(source);
    drop(registry);
    clean(&accounts);

    let mut definitions = mappings(&td, &accounts, None);
    let definition = definitions.pop().unwrap();
    drop(definitions);
    let policy = ConsumerServiceOwner::retain_access(
        Arc::new(Access(Arc::new(AtomicBool::new(false)))),
        &accounts,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    for saturated in [false, true] {
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        if saturated {
            assert!(limits.set(ResourceKind::EngineLiveBytesGlobalMax, Some(1)));
        }
        let bank = Arc::new(HostResourceAccounts::new(&limits));
        let mut budget = if saturated { work() } else { WorkBudget::new() };
        let refused = definition
            .clone()
            .bind(policy.clone(), bank.clone(), &mut budget)
            .err()
            .unwrap();
        let (returned_definition, returned_policy) = refused.into_input();
        assert!(Arc::ptr_eq(&definition, &returned_definition));
        assert!(Arc::ptr_eq(&policy, &returned_policy));
        drop(returned_definition);
        drop(returned_policy);
        clean(&bank);
    }
    drop(definition);
    drop(policy);
    clean(&accounts);
}
