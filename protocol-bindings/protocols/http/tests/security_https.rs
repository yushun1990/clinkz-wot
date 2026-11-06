#![cfg(feature = "prepare")]
#![allow(dead_code)]
//! Real TLS, qualified TD Producer, Engine Consumer and an independent TLS peer.
include!("support/security.rs");

#[test]
fn real_https_verifies_credentials_then_principal_and_reuses_only_immutable_meaning() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verify_a = verifier();
        let verify_b = verifier_password(b"independent-instance-password");
        let mut a = SecureFixture::new(&keys, verify_a.clone());
        let mut b = SecureFixture::new(&keys, verify_b.clone());
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        let bank_a = bank();
        let bank_b = bank();
        let access_a = access(
            &owner_a,
            a.td["properties"]["on"]["forms"][0]["href"]
                .as_str()
                .unwrap(),
            true,
            Arc::new(AtomicBool::new(false)),
        );
        let access_b = Arc::new(
            HttpBasicClientAccess::try_new(
                owner_b.reference(),
                b.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                Arc::new(Access(Arc::new(AtomicBool::new(false)))),
                Some(
                    HttpBasicCredentials::try_new(USER, b"independent-instance-password").unwrap(),
                ),
            )
            .unwrap(),
        );
        let (client_a, content) =
            secure_client(&a.td, &keys, &owner_a, access_a.clone(), &bank_a, None);
        let (client_b, content_b) =
            secure_client(&b.td, &keys, &owner_b, access_b, &bank_b, Some(&content));
        assert!(Arc::ptr_eq(&content, &content_b));
        let mut call_a = read_secure(&client_a);
        let mut call_b = read_secure(&client_b);
        let result_a = a.exchange(&mut call_a).await;
        let result_b = b.exchange(&mut call_b).await;
        assert_eq!(result_a.result().unwrap().data().unwrap().body, b"17");
        assert!(result_b.result().is_ok());
        assert_eq!(
            a.app.principal.borrow().as_ref().unwrap().id.as_str(),
            "verified-account"
        );
        assert!(verify_a.calls.load(Ordering::Relaxed) >= 3);
        owner_a.revoke();
        assert!(
            client_a
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop((call_b, result_b));
        drop(b.server().take_settlement());
        let mut next_b = read_secure(&client_b);
        let result_next_b = b.exchange(&mut next_b).await;
        assert!(
            result_next_b.result().is_ok(),
            "sibling credentials and authority remain independent"
        );
        drop((next_b, result_next_b));
        drop((
            call_a, client_a, client_b, owner_a, owner_b, content_b, access_a,
        ));
        assert!(
            bank_a
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop((result_a, content));
        clean(&bank_a);
        clean(&bank_b);
        a.close();
        b.close();
    });
}

#[test]
fn actual_support_refuses_complete_unsupported_expressions_and_incompatible_reuse() {
    let keys = Keys::for_name("127.0.0.1");
    let td = secure_description(address());
    let owner = consumer_instance();
    let accounts = bank();
    let target = td["properties"]["on"]["forms"][0]["href"].as_str().unwrap();
    let access = access(&owner, target, true, Arc::new(AtomicBool::new(false)));
    let prepared = secure_prepare(&td, &keys, access.clone(), &accounts, None).unwrap();
    let (bindings, funding) = prepared.into_execution();
    let definition = bindings[0].definition().clone();
    let content = definition.content().clone();
    let denied_service = ConsumerServiceOwner::retain_access(
        Arc::new(Access(Arc::new(AtomicBool::new(false)))),
        &accounts,
    )
    .unwrap();
    assert!(
        definition
            .clone()
            .bind(denied_service, accounts.clone(), &mut work())
            .is_err(),
        "NoSec-only service cannot bind Basic meaning"
    );
    let registration = keys.server(verifier());
    let address = target
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let provider = HttpFiniteProducer::try_https_basic(
        HttpProducerConfig::for_https_loopback(address).unwrap(),
        BindingId::new(197),
        BindingGeneration::INITIAL,
        1000,
        registration,
    )
    .unwrap();
    let supported = prepare_thing(
        &td.to_string(),
        authority(Generation::INITIAL).key(),
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    let program = supported.operations()[0]
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let wrong_service = keys.server(verifier());
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_https_loopback(address).unwrap(),
        program.clone(),
        authority(Generation::INITIAL).key(),
        AffordanceTarget::Property(Arc::from("on")),
        1000,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(
        config.with_security(wrong_service).is_err(),
        "equal scheme tags cannot substitute the actual verifier"
    );
    use clinkz_wot_core::producer::ProducerExecutor;
    let legacy = producer_execution(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(197),
        BindingGeneration::INITIAL,
        0,
    );
    assert!(
        legacy.check_program(&program).is_err(),
        "legacy cleartext executor must refuse a TLS program"
    );
    let mut changes = Vec::new();
    for scheme in [
        "nosec",
        "digest",
        "apikey",
        "bearer",
        "psk",
        "auto",
        "vendor:unknown",
    ] {
        let mut changed = td.clone();
        changed["securityDefinitions"]["auth"] = json!({"scheme":scheme});
        changes.push(changed);
    }
    for expression in [
        json!({"scheme":"combo","oneOf":["none","b"]}),
        json!({"scheme":"combo","allOf":["none","b"]}),
    ] {
        let mut changed = td.clone();
        changed["securityDefinitions"] =
            json!({"auth":expression,"none":{"scheme":"nosec"},"b":{"scheme":"basic"}});
        changes.push(changed);
    }
    for location in ["query", "body", "cookie", "uri"] {
        let mut changed = td.clone();
        changed["securityDefinitions"]["auth"]["in"] = json!(location);
        changes.push(changed);
    }
    for changed in [
        json!({"scheme":"basic","name":"X-Secret"}),
        json!({"scheme":"basic","proxy":"https://127.0.0.1/"}),
        json!({"scheme":"basic","extra":"unimplemented"}),
    ] {
        let mut change = td.clone();
        change["securityDefinitions"]["auth"] = changed;
        changes.push(change);
    }
    let mut simultaneous = td.clone();
    simultaneous["securityDefinitions"]["another"] = json!({"scheme":"basic"});
    simultaneous["security"] = json!(["auth", "another"]);
    changes.push(simultaneous);
    let mut empty = td.clone();
    empty["properties"]["on"]["forms"][0]["security"] = json!([]);
    changes.push(empty);
    let mut scope = td.clone();
    scope["properties"]["on"]["forms"][0]["scopes"] = json!(["admin"]);
    changes.push(scope);
    let mut clear = td.clone();
    clear["properties"]["on"]["forms"][0]["href"] = json!(target.replacen("https:", "http:", 1));
    changes.push(clear);
    let mut observe = td.clone();
    observe["properties"]["on"]["observable"] = json!(true);
    observe["properties"]["on"]["forms"][0]["op"] =
        json!(["readproperty", "observeproperty", "unobserveproperty"]);
    changes.push(observe);
    for changed in changes {
        // Consumer partial support does not grant complete Producer coverage.
        if changed["properties"]["on"]["observable"] != true {
            assert!(
                secure_prepare(&changed, &keys, access.clone(), &accounts, Some(&content)).is_err()
            );
        }
        assert!(
            prepare_thing(
                &changed.to_string(),
                authority(Generation::INITIAL).key(),
                GatewayDefaultV1::LIMITS,
                &mut work(),
                &[&provider],
                |_, _| true
            )
            .is_err()
        );
    }
    // Form override replaces inherited NoSec, rather than combining or ignoring it.
    let mut override_td = td.clone();
    override_td["securityDefinitions"]["none"] = json!({"scheme":"nosec"});
    override_td["security"] = json!(["none"]);
    override_td["properties"]["on"]["forms"][0]["security"] = json!(["auth"]);
    let overridden =
        secure_prepare(&override_td, &keys, access, &accounts, Some(&content)).unwrap();
    assert_eq!(overridden.content_constructions(), 0);
    drop((overridden, bindings, funding, content, definition));
    clean(&accounts);
}

#[test]
fn missing_credentials_and_wrong_instance_refuse_before_network_and_rotation_fences_unread_result()
{
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verify = verifier();
        let mut server = SecureFixture::new(&keys, verify.clone());
        let owner = consumer_instance();
        let accounts = bank();
        let target = server.td["properties"]["on"]["forms"][0]["href"]
            .as_str()
            .unwrap()
            .to_owned();
        let missing = access(&owner, &target, false, Arc::new(AtomicBool::new(false)));
        let (client, content) =
            secure_client(&server.td, &keys, &owner, missing.clone(), &accounts, None);
        assert!(
            client
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
        drop(client);
        missing.replace_credentials(Some(credentials())).unwrap();
        let (client, _) = secure_client(
            &server.td,
            &keys,
            &owner,
            missing.clone(),
            &accounts,
            Some(&content),
        );
        let mut call = read_secure(&client);
        until(&mut server, &mut call, RawFiniteState::Delivering).await;
        // Let the native operation complete without taking the Consumer result.
        let start = Instant::now();
        loop {
            server.step();
            client.step(&mut cx(), now(0), &mut work());
            if server.server().take_settlement().is_some() {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        missing.replace_credentials(None).unwrap();
        let result = loop {
            server.step();
            if let Poll::Ready(r) = call.poll(&mut cx(), now(0), &mut work()) {
                break r;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert!(result.result().is_err());
        drop((result, call, client));
        let replacement = ConsumerInstance::new(ThingSlotId::new(
            SlotIndex::new(0),
            Generation::new(2).unwrap(),
        ));
        missing.replace_credentials(Some(credentials())).unwrap();
        let (wrong, _) = secure_client(
            &server.td,
            &keys,
            &replacement,
            missing.clone(),
            &accounts,
            Some(&content),
        );
        assert!(
            wrong
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(wrong);
        let fresh = access(
            &replacement,
            &target,
            true,
            Arc::new(AtomicBool::new(false)),
        );
        let (client, _) = secure_client(
            &server.td,
            &keys,
            &replacement,
            fresh,
            &accounts,
            Some(&content),
        );
        let mut call = read_secure(&client);
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        drop((result, call, client, owner, replacement, missing, content));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn changed_verifier_application_policy_and_instance_authority_fence_original_delivery() {
    io().block_on(async {
        for case in 0..9 {
            let keys = Keys::for_name("127.0.0.1");
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify.clone());
            let owner = consumer_instance();
            let accounts = bank();
            let access = access(
                &owner,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                Arc::new(AtomicBool::new(false)),
            );
            let (client, content) =
                secure_client(&server.td, &keys, &owner, access, &accounts, None);
            let mut call = read_secure(&client);
            let accepted = case != 0;
            until(
                &mut server,
                &mut call,
                if accepted {
                    RawFiniteState::Delivering
                } else {
                    RawFiniteState::Dispatching
                },
            )
            .await;
            match case {
                0 | 1 => verify.revoked.store(true, Ordering::Release),
                2 => server.app.denied.set(true),
                3 => {
                    assert!(server.owner.revoke(server.exposure.key()));
                }
                4 => {
                    let successor = RawProducerInstance::new(
                        authority(Generation::new(2).unwrap()),
                        vec![RawFiniteBehavior::new(
                            AffordanceTarget::Property(Arc::from("on")),
                            Operation::ReadProperty,
                            SecureBehavior(Rc::new(SecureApp::default())),
                        )],
                    );
                    server
                        .owner
                        .replace(server.exposure.key(), successor)
                        .unwrap_or_else(|e| panic!("{:?}", e.error()));
                }
                5 => server.exposure.begin_close(),
                6 => verify.replacement_identity.store(true, Ordering::Release),
                7 => verify.changed_work.store(true, Ordering::Release),
                8 => *server.app.revoke_in_permission.borrow_mut() = Some(verify.clone()),
                _ => unreachable!(),
            }
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            assert_eq!(server.app.reads.get(), usize::from(accepted));
            let settlement = server.server().take_settlement().unwrap();
            if accepted {
                assert!(settlement.result.is_ok(), "retain original late output");
                assert_ne!(settlement.delivery, FiniteDelivery::Delivered);
            } else {
                assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
            }
            drop((settlement, result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

fn independent_request(
    addr: SocketAddr,
    config: Arc<rustls::ClientConfig>,
    authorization: Option<&str>,
) -> std::thread::JoinHandle<Vec<u8>> {
    let authorization = authorization.map(str::to_owned);
    std::thread::spawn(move || {
        let socket = TcpStream::connect(addr).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let connection = rustls::ClientConnection::new(
            config,
            rustls::pki_types::ServerName::IpAddress(addr.ip().into()),
        )
        .unwrap();
        let mut stream = rustls::StreamOwned::new(connection, socket);
        let mut request = format!("GET /on HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
        if let Some(header) = authorization {
            request.push_str("Authorization: ");
            request.push_str(&header);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut result = Vec::new();
        match stream.read_to_end(&mut result) {
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => (),
            Err(error) => panic!("independent TLS receive: {error:?}"),
        }
        result
    })
}

#[test]
fn independent_tls_peer_observes_401_403_and_verified_success_without_credential_diagnostics() {
    io().block_on(async {
        use base64::Engine;
        let keys = Keys::for_name("127.0.0.1");
        let valid = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode([USER, b":", PASSWORD].concat())
        );
        for case in 0..4 {
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify);
            server.app.denied.set(case == 2);
            let peer = independent_request(
                server.addr,
                keys.independent_client(),
                match case {
                    0 => None,
                    1 => Some("Basic ZGV2aWNlOndyb25n"),
                    _ => Some(&valid),
                },
            );
            let start = Instant::now();
            let settlement = loop {
                server.step();
                if let Some(s) = server.server().take_settlement() {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            let bytes = peer.join().unwrap();
            let expected = match case {
                0 | 1 => 401,
                2 => 403,
                _ => 200,
            };
            assert!(
                std::str::from_utf8(&bytes)
                    .unwrap()
                    .starts_with(&format!("HTTP/1.1 {expected}"))
            );
            if expected == 401 {
                assert!(
                    std::str::from_utf8(&bytes)
                        .unwrap()
                        .to_lowercase()
                        .contains("www-authenticate: basic realm=\"wot\"")
                );
            }
            assert!(!bytes.windows(PASSWORD.len()).any(|w| w == PASSWORD));
            let diagnostic = format!(
                "{settlement:?} {:?} {:?}",
                credentials(),
                AuthMaterial::Basic {
                    username: "private-user".into(),
                    password: "private-password".into()
                }
            );
            for secret in [
                std::str::from_utf8(PASSWORD).unwrap(),
                "private-user",
                "private-password",
                "ZGV2aWNlOndyb25n",
            ] {
                assert!(!diagnostic.contains(secret));
            }
            assert_eq!(server.app.reads.get(), usize::from(case == 3));
            drop(settlement);
            server.close();
        }
    });
}

#[test]
fn destination_trust_failures_never_reach_verifier_or_behavior() {
    io().block_on(async {
        for case in 0..3 {
            let keys = if case == 1 {
                Keys::for_name("127.0.0.2")
            } else if case == 2 {
                let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".into()]).unwrap();
                params.not_before = rcgen::date_time_ymd(2000, 1, 1);
                params.not_after = rcgen::date_time_ymd(2001, 1, 1);
                let key = rcgen::KeyPair::generate().unwrap();
                let cert = params.self_signed(&key).unwrap();
                Keys {
                    cert: cert.der().clone(),
                    key: PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
                }
            } else {
                Keys::for_name("127.0.0.1")
            };
            let wrong_roots = Keys::for_name("127.0.0.1");
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify.clone());
            let owner = consumer_instance();
            let accounts = bank();
            let access = access(
                &owner,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                Arc::new(AtomicBool::new(false)),
            );
            let (client, content) = secure_client(
                &server.td,
                if case == 0 { &wrong_roots } else { &keys },
                &owner,
                access,
                &accounts,
                None,
            );
            let mut call = read_secure(&client);
            let result = server.exchange(&mut call).await;
            assert_eq!(
                result.result().unwrap_err().security_reason(),
                Some(SecurityFailureReason::ProviderFailure)
            );
            assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
            assert_eq!(server.app.reads.get(), 0);
            drop((result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn cancellation_policy_change_and_zero_verification_credit_retain_original_owners_until_cleanup() {
    io().block_on(async {
        for case in 0..5 {
            let keys = Keys::for_name("127.0.0.1");
            let verify = verifier();
            let mut server = SecureFixture::new(&keys, verify.clone());
            let owner = consumer_instance();
            let accounts = bank();
            let deny = Arc::new(AtomicBool::new(false));
            let access = access(
                &owner,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                deny.clone(),
            );
            let (client, content) =
                secure_client(&server.td, &keys, &owner, access.clone(), &accounts, None);
            let mut call = read_secure(&client);
            if case != 0 {
                until(
                    &mut server,
                    &mut call,
                    if case == 1 {
                        RawFiniteState::Dispatching
                    } else {
                        RawFiniteState::Delivering
                    },
                )
                .await;
            }
            if case == 1 {
                let mut no_verification = work().with_remaining(WorkClass::HandlerSteps, 0);
                let _ = server
                    .owner
                    .step(
                        server.exposure.participant_mut::<SecureServer>(0).unwrap(),
                        &mut cx(),
                        &mut no_verification,
                        now(0),
                    )
                    .unwrap();
                assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
                assert_eq!(server.app.reads.get(), 0);
            }
            match case {
                0..=2 => call.cancel(),
                3 => deny.store(true, Ordering::Release),
                4 => access
                    .replace_credentials(Some(
                        HttpBasicCredentials::try_new(USER, b"replacement-password").unwrap(),
                    ))
                    .unwrap(),
                _ => unreachable!(),
            }
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            drop((call, client, owner, access, content));
            assert!(
                accounts
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .used()
                    > 0,
                "unread result retains original funding"
            );
            drop(result);
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn delivery_permission_callback_cannot_revoke_authority_or_publication_and_still_send() {
    io().block_on(async {
        for (in_final_verification, revoke_instance) in
            [(false, true), (false, false), (true, true), (true, false)]
        {
            let keys = Keys::for_name("127.0.0.1");
            let verifier = verifier();
            let mut server = SecureFixture::new(&keys, verifier.clone());
            let instance = consumer_instance();
            let accounts = bank();
            let access = access(
                &instance,
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                true,
                Arc::new(AtomicBool::new(false)),
            );
            let (client, content) =
                secure_client(&server.td, &keys, &instance, access, &accounts, None);
            let mut call = read_secure(&client);
            until(&mut server, &mut call, RawFiniteState::Delivering).await;
            let gate = ClosedProducerParticipation::ingress_gate(server.server()).clone();
            let SecureFixture {
                owner,
                mut exposure,
                app,
                accounts: producer_accounts,
                ..
            } = server;
            let owner = Rc::new(RefCell::new(owner));
            let key = exposure.key();
            let changed = Rc::new(Cell::new(false));
            let changed_in_hook = changed.clone();
            let callback_owner = owner.clone();
            let callback: Box<dyn FnOnce()> = Box::new(move || {
                if revoke_instance {
                    assert!(callback_owner.borrow_mut().revoke(key));
                } else {
                    gate.set(false);
                }
                changed_in_hook.set(true);
            });
            if in_final_verification {
                // Each send performs verification before and after permission.
                let at = verifier.calls.load(Ordering::Relaxed) + 2;
                VERIFICATION_HOOK.with(|pending| {
                    assert!(pending.borrow_mut().replace((at, callback)).is_none())
                });
            } else {
                *app.authority_in_permission.borrow_mut() = Some(callback);
            }
            // Direct native progress leaves the logical owner available to the
            // callback, exactly as supported independent ownership permits.
            let start = Instant::now();
            let result = loop {
                let _ = exposure.participant_mut::<SecureServer>(0).unwrap().step(
                    &mut cx(),
                    &mut work(),
                    now(0),
                );
                if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) {
                    break result;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert!(changed.get());
            VERIFICATION_HOOK.with(|pending| assert!(pending.borrow().is_none()));
            assert!(
                result.result().is_err(),
                "callback-time revocation must fence application delivery"
            );
            let runtime = exposure.participant_mut::<SecureServer>(0).unwrap();
            let settlement = runtime.take_settlement().unwrap();
            assert_eq!(settlement.delivery, FiniteDelivery::NotDelivered);
            assert_eq!(
                settlement
                    .result
                    .as_ref()
                    .unwrap()
                    .data()
                    .unwrap()
                    .body
                    .as_ref(),
                b"17"
            );
            assert_eq!(app.reads.get(), 1);
            drop(settlement);
            exposure.begin_close();
            for _ in 0..100 {
                let _ = exposure.participant_mut::<SecureServer>(0).unwrap().step(
                    &mut cx(),
                    &mut work(),
                    now(0),
                );
            }
            exposure
                .retire(&mut owner.borrow_mut(), &mut work())
                .unwrap();
            drop((exposure, owner, app, client, call, instance, content));
            assert!(
                accounts
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .used()
                    > 0
            );
            drop(result);
            clean(&accounts);
            clean(&producer_accounts);
        }
    });
}

#[derive(Default)]
struct ReviewPolicy {
    revision: std::sync::atomic::AtomicU64,
    calls: std::sync::atomic::AtomicUsize,
    change_in_authorize: AtomicBool,
    change_in_instance: AtomicBool,
}
impl ConsumerAccessService for ReviewPolicy {
    fn security_revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.change_in_authorize.swap(false, Ordering::AcqRel) {
            self.revision.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.change_in_instance.swap(false, Ordering::AcqRel) {
            self.revision.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        64
    }
    fn work(&self) -> u64 {
        4096
    }
}

#[test]
fn basic_credential_issuance_prepays_its_own_complete_callback_work() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verify = verifier();
        let mut server = SecureFixture::new(&keys, verify.clone());
        let instance = consumer_instance();
        let accounts = bank();
        let policy = Arc::new(ReviewPolicy::default());
        let access = Arc::new(
            HttpBasicClientAccess::try_new(
                instance.reference(),
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                policy.clone(),
                Some(credentials()),
            )
            .unwrap(),
        );
        let callback_work = access.work();
        let (client, content) =
            secure_client(&server.td, &keys, &instance, access, &accounts, None);
        let baseline: Vec<_> = ResourceKind::ALL.map(|k| accounts.account(k).used()).into();
        for credit in [callback_work, 2 * callback_work - 1] {
            policy.calls.store(0, Ordering::Relaxed);
            let mut budget = work().with_remaining(WorkClass::ProviderProbes, credit);
            assert!(
                client
                    .read_property("on", Deadline::NONE, now(0), &mut budget)
                    .is_err(),
                "one permission payment cannot also fund credential issuance"
            );
            assert_eq!(policy.calls.load(Ordering::Relaxed), 2);
            assert_eq!(
                budget.remaining(WorkClass::ProviderProbes),
                credit - callback_work
            );
            assert_eq!(
                ResourceKind::ALL
                    .map(|k| accounts.account(k).used())
                    .as_slice(),
                baseline
            );
            assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
        }
        policy.calls.store(0, Ordering::Relaxed);
        let mut budget = work().with_remaining(WorkClass::ProviderProbes, 2 * callback_work);
        let mut call = client
            .read_property("on", Deadline::NONE, now(0), &mut budget)
            .unwrap();
        assert_eq!(budget.remaining(WorkClass::ProviderProbes), 0);
        assert_eq!(policy.calls.load(Ordering::Relaxed), 4);
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        drop((call, result, client, content, instance));
        clean(&accounts);
        server.close();
    });
}

struct ReviewData {
    actual: Arc<dyn ConsumerDataService>,
    validations: Arc<std::sync::atomic::AtomicUsize>,
}
impl ConsumerDataService for ReviewData {
    fn content_type(&self) -> &str {
        self.actual.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut policy = self.actual.policy();
        policy.retained_bytes += 64;
        policy
    }
    fn supports_schema(&self, schema: &data_schema::PreparedSchema) -> bool {
        self.actual.supports_schema(schema)
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
        self.actual.validate(schema, payload)?;
        self.validations.fetch_add(1, Ordering::Release);
        Ok(())
    }
}

#[test]
fn wrapped_policy_revision_fences_a_validated_unread_tls_result_and_rebinds_fresh_state() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let mut server = SecureFixture::new(&keys, verifier());
        let instance = consumer_instance();
        let accounts = bank();
        let policy = Arc::new(ReviewPolicy::default());
        let access = Arc::new(
            HttpBasicClientAccess::try_new(
                instance.reference(),
                server.td["properties"]["on"]["forms"][0]["href"]
                    .as_str()
                    .unwrap(),
                policy.clone(),
                Some(credentials()),
            )
            .unwrap(),
        );
        let validations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut registration = try_https_basic_consumer_registration(
            keys.client(),
            access.clone(),
            BindingId::new(198),
            BindingGeneration::INITIAL,
            0,
        )
        .unwrap();
        registration.services.data = Arc::new(ReviewData {
            actual: registration.services.data,
            validations: validations.clone(),
        });
        let prepared = prepare_consumer(
            &server.td,
            accounts.clone(),
            vec![registration],
            ConsumerSelector::default(),
        )
        .unwrap();
        let (bindings, funding) = prepared.into_execution();
        let definition = bindings[0].definition().clone();
        let content = definition.content().clone();
        let client =
            ConsumerRuntime::try_new(instance.reference(), bindings, funding, accounts.clone(), 1)
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let mut call = read_secure(&client);
        let start = Instant::now();
        while validations.load(Ordering::Acquire) == 0 {
            server.step();
            client.step(&mut cx(), now(0), &mut work());
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        // Validation has finished, while the original outcome remains unread.
        let before = access.security_revision();
        policy.revision.fetch_add(1, Ordering::AcqRel);
        let result = server.exchange(&mut call).await;
        assert!(
            result.result().is_err(),
            "wrapped revision change must invalidate the already validated result"
        );
        assert!(matches!(
            result.native_outcome(),
            ConsumerNativeOutcome::Returned(Ok(_))
        ));
        assert_ne!(before, access.security_revision());
        drop((call, result));
        drop(server.server().take_settlement());
        policy.calls.store(0, Ordering::Relaxed);
        assert!(
            client
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(
            policy.calls.load(Ordering::Relaxed),
            0,
            "stale binding cannot authorize a new call"
        );
        drop(client);
        let owner = ConsumerServiceOwner::retain_access(access.clone(), &accounts).unwrap();
        let binding = definition
            .bind(owner, accounts.clone(), &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let client = ConsumerRuntime::try_new(
            instance.reference(),
            vec![binding],
            vec![],
            accounts.clone(),
            1,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let mut call = read_secure(&client);
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        drop((call, result, client, content, instance, access));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn wrapped_policy_changes_inside_permission_and_issuance_cannot_grant_original_work() {
    let keys = Keys::for_name("127.0.0.1");
    let td = secure_description(address());
    let instance = consumer_instance();
    let accounts = bank();
    let policy = Arc::new(ReviewPolicy::default());
    let access = Arc::new(
        HttpBasicClientAccess::try_new(
            instance.reference(),
            td["properties"]["on"]["forms"][0]["href"].as_str().unwrap(),
            policy.clone(),
            Some(credentials()),
        )
        .unwrap(),
    );
    let prepared = secure_prepare(&td, &keys, access.clone(), &accounts, None).unwrap();
    let (bindings, funding) = prepared.into_execution();
    let definition = bindings[0].definition().clone();
    let binding = definition.binding();
    let target = definition.target();
    for change_in_first_callback in [false, true] {
        let revision = access.security_revision();
        if change_in_first_callback {
            policy.change_in_authorize.store(true, Ordering::Release);
        } else {
            policy.change_in_instance.store(true, Ordering::Release);
        }
        assert!(
            access
                .outbound_security(
                    &instance.reference(),
                    binding,
                    target,
                    Operation::ReadProperty,
                    definition.content().security()
                )
                .is_err()
        );
        assert_ne!(access.security_revision(), revision);
    }
    let grant = access
        .outbound_security(
            &instance.reference(),
            binding,
            target,
            Operation::ReadProperty,
            definition.content().security(),
        )
        .unwrap();
    assert!(
        grant.is_some(),
        "freshly qualified state still permits a new grant"
    );
    drop((bindings, funding, definition, instance));
    clean(&accounts);
}

#[derive(Clone, Copy, Debug)]
enum DispatchInvalidation {
    BeforeDispatch,
    Authorization,
    FirstVerification,
    FinalVerification,
    FinalVerificationWork,
}

fn verify_dispatch_refuses_callback_invalidation(edge: DispatchInvalidation) {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verify = verifier();
        let mut server = SecureFixture::new(&keys, verify.clone());
        let instance = consumer_instance();
        let accounts = bank();
        let access = access(
            &instance,
            server.td["properties"]["on"]["forms"][0]["href"]
                .as_str()
                .unwrap(),
            true,
            Arc::new(AtomicBool::new(false)),
        );
        let (client, content) =
            secure_client(&server.td, &keys, &instance, access, &accounts, None);
        let key = server.exposure.key();
        let mut call = read_secure(&client);
        until(&mut server, &mut call, RawFiniteState::Dispatching).await;
        assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
        let gate = ClosedProducerParticipation::ingress_gate(server.server()).clone();
        let changed = Rc::new(Cell::new(false));
        let changed_in_hook = changed.clone();
        let verify_in_hook = verify.clone();
        let invalidate = move || {
            if matches!(edge, DispatchInvalidation::FinalVerificationWork) {
                verify_in_hook.changed_work.store(true, Ordering::Release);
            } else {
                gate.set(false);
            }
            changed_in_hook.set(true);
        };
        match edge {
            DispatchInvalidation::BeforeDispatch => invalidate(),
            DispatchInvalidation::Authorization => {
                *server.app.authority_in_permission.borrow_mut() = Some(Box::new(invalidate));
            }
            DispatchInvalidation::FirstVerification
            | DispatchInvalidation::FinalVerification
            | DispatchInvalidation::FinalVerificationWork => {
                let at = if matches!(edge, DispatchInvalidation::FirstVerification) {
                    1
                } else {
                    2
                };
                VERIFICATION_HOOK.with(|pending| {
                    assert!(
                        pending
                            .borrow_mut()
                            .replace((at, Box::new(invalidate)))
                            .is_none()
                    );
                });
            }
        }
        if matches!(edge, DispatchInvalidation::BeforeDispatch) {
            let mut no_work = WorkBudget::new();
            let progress = server
                .owner
                .step(
                    server.exposure.participant_mut::<SecureServer>(0).unwrap(),
                    &mut cx(),
                    &mut no_work,
                    now(0),
                )
                .unwrap();
            assert_eq!(progress, Poll::Ready(RawFiniteState::Closing));
            assert_eq!(verify.calls.load(Ordering::Relaxed), 0);
            assert_eq!(server.app.reads.get(), 0);
            // Fencing is immediate; cleanup still waits for its own credit
            // while retaining the full original unaccepted native exchange.
            assert!(
                server
                    .server()
                    .step(&mut cx(), &mut no_work, now(0))
                    .is_pending()
            );
            assert!(server.server().take_settlement().is_none());
        }
        let result = server.exchange(&mut call).await;
        let settlement = server.server().take_settlement().unwrap();
        assert!(changed.get(), "{edge:?} hook must run");
        VERIFICATION_HOOK.with(|pending| assert!(pending.borrow().is_none()));
        assert_eq!(
            server.app.reads.get(),
            0,
            "{edge:?}: stale acceptance cannot enter behavior"
        );
        assert!(result.result().is_err(), "{edge:?}");
        assert_eq!(settlement.effect, FiniteEffect::NotInvoked, "{edge:?}");
        assert_eq!(
            settlement.delivery,
            FiniteDelivery::NotDelivered,
            "{edge:?}"
        );
        assert_eq!(settlement.recipient, Some(key));
        assert_eq!(settlement.operation, Some(Operation::ReadProperty));
        assert_eq!(
            settlement.binding,
            (BindingId::new(197), BindingGeneration::INITIAL)
        );
        assert!(settlement.result.is_err());
        drop((settlement, client, call, instance, content));
        assert!(
            accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop(result);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn dispatch_refuses_preclosed_publication() {
    verify_dispatch_refuses_callback_invalidation(DispatchInvalidation::BeforeDispatch);
}
#[test]
fn dispatch_refuses_publication_closed_during_authorization() {
    verify_dispatch_refuses_callback_invalidation(DispatchInvalidation::Authorization);
}
#[test]
fn dispatch_refuses_publication_closed_during_first_verification() {
    verify_dispatch_refuses_callback_invalidation(DispatchInvalidation::FirstVerification);
}
#[test]
fn dispatch_refuses_publication_closed_during_final_verification() {
    verify_dispatch_refuses_callback_invalidation(DispatchInvalidation::FinalVerification);
}
#[test]
fn dispatch_refuses_service_work_changed_during_final_verification() {
    verify_dispatch_refuses_callback_invalidation(DispatchInvalidation::FinalVerificationWork);
}
