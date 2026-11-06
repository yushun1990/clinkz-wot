// Shared real-wire and contract cases for all/selected writes.
fn shared_untrusted_producer_input_is_fully_validated_before_group_authorization_or_effects(
    operation: Operation,
    invalid_inputs: Vec<Vec<u8>>,
) {
    let write_description = |addr| collection_description(addr, operation);
    let addr = address();
    let mut server = Fixture::new(write_description(addr));
    for body in invalid_inputs.into_iter().filter(|b| b.len() <= 4096) {
        let mut peer = TcpStream::connect(addr).unwrap();
        write!(peer, "PUT /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
        peer.write_all(&body).unwrap();
        peer.shutdown(std::net::Shutdown::Write).unwrap();
        let start = Instant::now();
        let settled = loop {
            server.step();
            if let Some(s) = server.server().take_settlement() {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
        };
        assert!(matches!(settled.result, Err(CoreError::Validation(_))));
        assert_eq!(settled.effect, FiniteEffect::NotInvoked);
        assert_eq!(settled.operation, Some(operation));
        assert_eq!(server.app.authorizations.get(), 0);
        assert_eq!(server.app.roots.get(), 0);
        assert!(server.app.applied.borrow().is_null());
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut response = String::new();
        peer.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 500"));
    }
    server.close();
}

fn shared_failure_before_during_and_after_application_never_implies_rollback_or_member_success(
    operation: Operation,
) {
    let write_description = |addr| collection_description(addr, operation);
    let start_write =
        |client: &ConsumerRuntime| start_collection_at(client, operation, Deadline::NONE).unwrap();
    io().block_on(async {
        for fault in 1..=5 {
            let addr = address();
            let td = write_description(addr);
            let mut server = Fixture::new(td.clone());
            server.app.write_fault.set(fault);
            let accounts = bank();
            let owner = consumer_instance();
            let client = client(
                td,
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &owner,
            );
            let mut call = start_write(&client);
            let (local, remote) = server.exchange(&mut call).await;
            assert!(local.result().is_err());
            assert!(remote.result.is_err());
            assert_eq!(local.effect(), ConsumerEffect::Unknown);
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert_eq!(remote.operation, Some(operation));
            assert_eq!(
                *server.app.applied.borrow(),
                match fault {
                    1 => json!({"on":false}), // A real partial application effect.
                    5 => Value::Null,         // Handler entry alone cannot certify no effects.
                    _ => json!({"on":false,"secret":"updated"}),
                }
            );
            for _ in 0..10 {
                server.step();
                client.step(&mut cx(), now(0), &mut work());
            }
            assert_eq!(server.controls.requests.get(), 1);
            assert_eq!(server.app.roots.get(), 1);
            assert_eq!(server.app.writes.get(), 0);
            drop(local);
            drop(call);
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}

fn shared_ambiguous_or_partial_native_completion_never_confirms_or_replays_the_group(
    operation: Operation,
) {
    let write_description = |addr| collection_description(addr, operation);
    let start_write =
        |client: &ConsumerRuntime| start_collection_at(client, operation, Deadline::NONE).unwrap();
    for reply in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".as_slice(),
        b"HTTP/1.1 202 Accepted\r\n\r\n",
        b"HTTP/1.1 206 Partial Content\r\nContent-Length: 2\r\n\r\n{}",
        b"HTTP/1.1 207 Multi-Status\r\nContent-Length: 2\r\n\r\n{}",
        b"HTTP/1.1 500 Internal Server Error\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 204 No Content\r\n\r\n{}",
        b"HTTP/1.1 204 No Content\r\n",
        b"",
        b"HTTP/1.1 204 No Content\r\n\r\n",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let peer = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0; 4096];
            while !bytes.ends_with(VALUES) {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
            }
            assert!(bytes.starts_with(b"PUT /properties HTTP/1.1\r\n"));
            stream.write_all(reply).unwrap();
            drop(stream);
            listener
        });
        io().block_on(async {
            let accounts = bank();
            let owner = consumer_instance();
            let client = client(
                write_description(addr),
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &owner,
            );
            let mut call = start_write(&client);
            let start = Instant::now();
            let result = loop {
                if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            let confirmed = reply == b"HTTP/1.1 204 No Content\r\n\r\n";
            assert_eq!(result.result().is_ok(), confirmed);
            assert_eq!(
                result.effect(),
                if confirmed {
                    ConsumerEffect::Completed
                } else {
                    ConsumerEffect::Unknown
                }
            );
            for _ in 0..10 {
                client.step(&mut cx(), now(0), &mut work());
            }
            drop(result);
            drop(call);
            drop(client);
            clean(&accounts);
        });
        let listener = peer.join().unwrap();
        listener.set_nonblocking(true).unwrap();
        assert!(
            matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
            "effect-uncertain collection cannot replay"
        );
    }
}

fn shared_complete_producer_qualification_refuses_missing_root_behavior_and_unsupported_advertisement(
    operation: Operation,
) {
    let write_description = |addr| collection_description(addr, operation);
    for fault in 0..16 {
        let addr = address();
        let mut td = write_description(addr);
        match fault {
            0 => td["properties"]["on"]["const"] = json!(false),
            1 => td["properties"]["secret"]["pattern"] = json!("^x$"),
            2 => td["properties"]["on"]["enum"] = json!([true, false]),
            3 => td["forms"][0]["htv:methodName"] = json!("POST"),
            4 => {
                td["forms"][0]["response"] =
                    json!({"contentType":"application/json","htv:statusCodeValue":200})
            }
            5 => td["forms"][0]["contentType"] = json!("text/plain"),
            6 => td["forms"][0]["contentCoding"] = json!("gzip"),
            7 => td["securityDefinitions"]["n"] = json!({"scheme":"basic","in":"header"}),
            8 => td["forms"][0]["href"] = json!(format!("http://{addr}/properties{{?unit}}")),
            9 => td["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"href":"unknown:group","op":"observeallproperties"})),
            10 => td["properties"]["level"]["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"href":"unknown:level","op":"readproperty"})),
            11 => td["properties"]["level"]["forms"][0]["op"] = json!("writeproperty"),
            12 => {
                td["forms"][0]["op"] = json!(["readallproperties", operation.as_str()]);
                td["forms"][0]["htv:methodName"] = json!("PUT");
            }
            13 => td["forms"][0]["htv:statusCodeValue"] = json!(204),
            14 => td["uriVariables"] = json!({"unit":{"type":"string"}}),
            15 => {} // Native support alone cannot replace the actual root behavior.
            _ => unreachable!(),
        }
        let auth = authority(Generation::INITIAL);
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
                auth.key(),
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |scope, op| !(fault == 15
                    && scope == ValidatedProducerScope::Thing
                    && op == operation)
            )
            .is_err(),
            "fault={fault}"
        );
        // Unsupported Producer siblings do not narrow a Consumer root choice.
        if !matches!(fault, 9..=11 | 15) {
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
                        .any(|s| s.selected && s.operation == operation),
                    "fault={fault}"
                );
            }
            clean(&accounts);
        }
        drop(TcpListener::bind(addr).unwrap());
    }
}

fn shared_valid_head_without_eof_is_not_completion_and_retains_funding_until_settlement(
    operation: Operation,
) {
    let write_description = |addr| collection_description(addr, operation);
    for fault in ["finish", "extra", "cancel", "timeout"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (sent, received) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0; 4096];
            while !bytes.ends_with(VALUES) {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
            }
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\n\r\n")
                .unwrap();
            sent.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            if fault == "extra" {
                stream.write_all(b"{}").unwrap();
            }
            drop(stream);
            listener
        });
        io().block_on(async {
            let accounts = bank();
            let owner = consumer_instance();
            let client = client(
                write_description(addr),
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &owner,
            );
            let mut call = start_collection_at(&client, operation, Deadline::at(now(10))).unwrap();
            let start = Instant::now();
            loop {
                assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
                if received.try_recv().is_ok() {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            let retained = ResourceKind::ALL.map(|k| accounts.account(k).used());
            for _ in 0..10 {
                assert!(
                    call.poll(&mut cx(), now(0), &mut work()).is_pending(),
                    "head alone cannot confirm a write"
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
            assert_eq!(
                retained,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
            if fault == "cancel" {
                call.cancel();
            }
            release.send(()).unwrap();
            let instant = now(if fault == "timeout" { 10 } else { 0 });
            assert!(
                call.poll(&mut cx(), instant, &mut WorkBudget::new())
                    .is_pending()
            );
            let result = loop {
                if let Poll::Ready(s) = call.poll(&mut cx(), instant, &mut work()) {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(result.result().is_ok(), fault == "finish");
            assert_eq!(
                result.effect(),
                if fault == "finish" {
                    ConsumerEffect::Completed
                } else {
                    ConsumerEffect::Unknown
                }
            );
            assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
            drop(result);
            drop(call);
            drop(client);
            clean(&accounts);
        });
        let listener = peer.join().unwrap();
        listener.set_nonblocking(true).unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

fn shared_unread_confirmed_completion_keeps_original_evidence_when_local_handoff_is_revoked(
    operation: Operation,
) {
    let write_description = |addr| collection_description(addr, operation);
    let start_write =
        |client: &ConsumerRuntime| start_collection_at(client, operation, Deadline::NONE).unwrap();
    use std::sync::atomic::AtomicUsize;
    struct CountAccess(Access, Arc<AtomicUsize>);
    impl ConsumerAccessService for CountAccess {
        fn authorize(
            &self,
            b: BindingRegistrationIdentity,
            t: &str,
            o: Operation,
        ) -> CoreResult<()> {
            self.0.authorize(b, t, o)
        }
        fn authorize_instance(
            &self,
            i: &ConsumerInstanceRef,
            b: BindingRegistrationIdentity,
            t: &str,
            o: Operation,
        ) -> CoreResult<()> {
            self.1.fetch_add(1, Ordering::Relaxed);
            self.0.authorize_instance(i, b, t, o)
        }
        fn retained_bytes(&self) -> u64 {
            128
        }
        fn work(&self) -> u64 {
            1
        }
    }
    io().block_on(async {
        for fault in ["cancel", "replace", "policy"] {
            let addr = address();
            let td = write_description(addr);
            let mut server = Fixture::new(td.clone());
            let accounts = bank();
            let owner = consumer_instance();
            let denied = Arc::new(AtomicBool::new(false));
            let checks = Arc::new(AtomicUsize::new(0));
            let mut reg = registration();
            reg.services.access = Arc::new(CountAccess(Access(denied.clone()), checks.clone()));
            let p = prepare_consumer(&td, accounts.clone(), vec![reg], ConsumerSelector::default()).unwrap();
            let (d, f) = p.into_execution();
            let client = ConsumerRuntime::try_new(owner.reference(), d, f, accounts.clone(), 2).unwrap_or_else(|_| panic!());
            let mut call = start_write(&client);
            let start = Instant::now();
            // Withhold handoff. Two authorization callbacks in one step witness
            // the post-validation completion check without private-state access.
            let mut remote = None;
            let mut completed = false;
            while !completed || remote.is_none() {
                server.step();
                let before = checks.load(Ordering::Relaxed);
                client.step(&mut cx(), now(0), &mut work());
                completed |= checks.load(Ordering::Relaxed) - before == 2;
                if let Some(s) = server.server().take_settlement() { remote = Some(s); }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            assert_eq!(remote.unwrap().effect, FiniteEffect::Completed);
            assert!(client.write_all_properties(payload(VALUES), Deadline::NONE, now(0), &mut work()).is_err(), "unread result retains native occupancy");
            match fault { "cancel" => call.cancel(), "replace" => owner.revoke(), _ => denied.store(true, Ordering::Relaxed) }
            let retained = ResourceKind::ALL.map(|k| accounts.account(k).used());
            assert!(call.poll(&mut cx(), now(0), &mut WorkBudget::new()).is_pending());
            let Poll::Ready(local) = call.poll(&mut cx(), now(0), &mut work()) else { panic!("original completed result retained") };
            assert!(local.result().is_err());
            assert_eq!(local.effect(), ConsumerEffect::Completed);
            assert!(matches!(local.native_outcome(), ConsumerNativeOutcome::Returned(Ok(o)) if o.data().is_none()));
            assert_eq!(local.consumer(), &owner.reference());
            assert_eq!(retained, ResourceKind::ALL.map(|k| accounts.account(k).used()));
            assert_eq!(server.controls.requests.get(), 1);
            assert_eq!(server.app.roots.get(), 1);
            assert_eq!(*server.app.applied.borrow(), json!({"on":false,"secret":"updated"}));
            drop(local); drop(call); drop(client); clean(&accounts); server.close();
        }
    });
}

fn shared_final_qualification_requires_original_writable_members_and_actual_root_native_coverage(
    operation: Operation,
) {
    let write_description = |addr| collection_description(addr, operation);
    let addr = address();
    let td = write_description(addr);
    let auth = authority(Generation::INITIAL);
    let key = auth.key();
    let accounts = bank();
    let owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 8,
        target_name_bytes: 256,
    })
    .unwrap();
    let listen = HttpProducerConfig::for_loopback(addr).unwrap();
    let provider: Rc<dyn ProducerCapability> = Rc::new(
        HttpFiniteProducer::try_new(listen, BindingId::new(97), BindingGeneration::INITIAL, 1000)
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
    for fault in ["eligibility", "type", "new-member"] {
        let mut changed = td.clone();
        match fault {
            "eligibility" => changed["properties"]["on"]["readOnly"] = json!(true),
            "type" => changed["properties"]["secret"]["type"] = json!("integer"),
            _ => {
                changed["properties"]["extra"] = json!({"type":"null","writeOnly":true,"forms":[{"href":format!("http://{addr}/extra")}]})
            }
        }
        assert!(
            check_final_thing(
                &changed.to_string(),
                key,
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[provider.as_ref()],
                &prepared,
                |_, _| true
            )
            .is_err()
        );
    }
    assert!(
        check_final_thing(
            &td.to_string(),
            key,
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[provider.as_ref()],
            &prepared,
            |_, _| true
        )
        .is_ok()
    );
    let mappings = prepared
        .operations()
        .iter()
        .filter(|o| o.site != FormSite::Thing)
        .map(|o| {
            let FormSite::Property(name) = &o.site else {
                unreachable!()
            };
            (
                AffordanceTarget::Property(Arc::from(name.as_str())),
                o.definition::<HttpProducerProgram>().unwrap().clone(),
            )
        })
        .collect();
    let config = HttpFiniteConfig::try_from_mappings(listen, mappings, key, 1000)
        .unwrap_or_else(|_| panic!());
    let native = RawFiniteRuntime::try_new(
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
    let behaviors = prepared
        .operations()
        .iter()
        .map(|o| {
            RawFiniteBehavior::new(
                match &o.site {
                    FormSite::Thing => AffordanceTarget::Thing,
                    FormSite::Property(n) => AffordanceTarget::Property(Arc::from(n.as_str())),
                    _ => unreachable!(),
                },
                o.operation,
                Behavior(app.clone()),
            )
        })
        .collect();
    let instance =
        RawProducerInstance::new(auth, behaviors).with_description(Arc::from(td.to_string()));
    let original = ComposedProducerExposure::try_new(
        &owner,
        ComposedProducerInput {
            instance,
            prepared,
            providers: vec![provider],
            participants: vec![Box::new(native)],
            expected: None,
            handler_backing_bytes: 8192,
        },
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
        &mut work(),
    )
    .err()
    .expect("individual mappings cannot qualify root write")
    .into_input();
    assert!(owner.description(key).is_none());
    assert_eq!(app.roots.get(), 0);
    assert!(app.applied.borrow().is_null());
    drop(original);
    clean(&accounts);
    drop(TcpListener::bind(addr).unwrap());
}
