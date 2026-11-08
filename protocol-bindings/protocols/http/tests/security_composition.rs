#![cfg(feature = "prepare")]
#![allow(dead_code)]
//! Qualified composed security over actual Engine-to-Engine and independent TLS I/O.
include!("support/security.rs");
use std::sync::atomic::{AtomicU64, AtomicUsize};

const API_KEY: &[u8] = b"independent-api-credential";
thread_local! { static KEY_HOOK: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) }; }
struct KeyVerifier {
    revision: AtomicU64,
    revoked: AtomicBool,
    different_identity: AtomicBool,
    changed_work: AtomicBool,
    calls: AtomicUsize,
}
impl HttpApiKeyVerifier for KeyVerifier {
    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    fn work(&self) -> u64 {
        if self.changed_work.load(Ordering::Acquire) {
            2048
        } else {
            1024
        }
    }
    fn verify(&self, key: &[u8]) -> CoreResult<Principal> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let hook = KEY_HOOK.with(|h| h.borrow_mut().take());
        if let Some(hook) = hook {
            hook();
        }
        if self.revoked.load(Ordering::Acquire) || key != API_KEY {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::InvalidCredentials,
                context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never),
            });
        }
        Ok(Principal {
            id: PrincipalId::from(if self.different_identity.load(Ordering::Acquire) {
                "other-account"
            } else {
                "verified-account"
            }),
            scopes: vec!["property-access".into()],
        })
    }
}
fn key_verifier() -> Arc<KeyVerifier> {
    Arc::new(KeyVerifier {
        revision: AtomicU64::new(0),
        revoked: AtomicBool::new(false),
        different_identity: AtomicBool::new(false),
        changed_work: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
    })
}
fn server_security(
    keys: &Keys,
    basic: Option<Arc<Verifier>>,
    key: Option<Arc<KeyVerifier>>,
) -> HttpServerSecurity {
    HttpServerSecurity::try_composed(
        vec![keys.cert.clone()],
        keys.key.clone_key(),
        basic.map(|v| v as Arc<dyn HttpBasicVerifier>),
        key.map(|v| v as Arc<dyn HttpApiKeyVerifier>),
    )
    .unwrap()
}
fn description_for(addr: SocketAddr, security: Value) -> Value {
    let mut td = secure_description(addr);
    td["securityDefinitions"]["key"] = json!({"scheme":"apikey","in":"header","name":"X-Api-Key"});
    td["securityDefinitions"]["none"] = json!({"scheme":"nosec"});
    td["securityDefinitions"]["either"] = json!({"scheme":"combo","oneOf":["auth","key"]});
    td["securityDefinitions"]["both"] = json!({"scheme":"combo","allOf":["auth","key"]});
    td["security"] = security;
    td
}
fn target(td: &Value) -> &str {
    td["properties"]["on"]["forms"][0]["href"].as_str().unwrap()
}
fn client_access(
    owner: &ConsumerInstance,
    td: &Value,
    basic: bool,
    key: bool,
) -> Arc<HttpClientAccess> {
    Arc::new(
        HttpClientAccess::try_composed(
            owner.reference(),
            target(td),
            Arc::new(Access(Arc::new(AtomicBool::new(false)))),
            basic.then(credentials),
            key.then(|| HttpApiKey::try_new(API_KEY).unwrap()),
        )
        .unwrap(),
    )
}
fn prepare_producer(td: &Value, service: HttpServerSecurity) -> CoreResult<PreparedThing> {
    let addr = target(td)
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let provider = HttpFiniteProducer::try_https(
        HttpProducerConfig::for_https_loopback(addr).unwrap(),
        BindingId::new(197),
        BindingGeneration::INITIAL,
        1000,
        service,
    )
    .unwrap();
    prepare_thing(
        &td.to_string(),
        authority(Generation::INITIAL).key(),
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
}

#[test]
fn complete_conjunctions_alternatives_and_form_override_execute_with_actual_services() {
    io().block_on(async {
        for case in 0..7 {
            let keys = Keys::for_name("127.0.0.1");
            let basic = verifier();
            let key = key_verifier();
            let addr = address();
            let mut td = description_for(
                addr,
                match case {
                    0 => json!(["auth", "key"]),
                    1 => json!(["both"]),
                    2 | 3 => json!(["either"]),
                    4 => json!(["nested"]),
                    5 => json!(["key"]),
                    _ => json!(["auth"]),
                },
            );
            if case == 4 {
                td["securityDefinitions"]["nested"] =
                    json!({"scheme":"combo","allOf":["none","either"]});
            }
            if case == 6 {
                td["securityDefinitions"]["unsupported"] = json!({"scheme":"digest"});
                td["security"] = json!(["unsupported"]);
                td["properties"]["on"]["forms"][0]["security"] = json!(["both"]);
            }
            let uses_basic = matches!(case, 0 | 1 | 2 | 4 | 6);
            let uses_key = matches!(case, 0 | 1 | 3 | 5 | 6);
            // API-key-only execution has no dummy Basic verifier or credentials.
            let service =
                server_security(&keys, (case != 5).then(|| basic.clone()), Some(key.clone()));
            let mut server =
                SecureFixture::from_security(service, addr, td, &[Operation::ReadProperty]);
            let owner = consumer_instance();
            let accounts = bank();
            let access = client_access(&owner, &server.td, uses_basic, uses_key);
            let (client, content) =
                secure_client(&server.td, &keys, &owner, access, &accounts, None);
            if case == 2 {
                assert_eq!(content.security(), &security::SecurityRequirement::scheme("basic", &[("in", "header")]),
                    "shared meaning must preserve an absent credential name; HTTP owns its header default");
            }
            server.td = Value::Null;
            let mut call = read_secure(&client);
            let result = server.exchange(&mut call).await;
            assert!(
                result.result().is_ok(),
                "case {case}: {:?}",
                result.result()
            );
            assert_eq!(basic.calls.load(Ordering::Relaxed) > 0, uses_basic);
            assert_eq!(key.calls.load(Ordering::Relaxed) > 0, uses_key);
            assert_eq!(server.app.reads.get(), 1);
            drop((result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn supported_consumer_choice_does_not_certify_the_producers_other_alternatives() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let mut server = SecureFixture::new(&keys, verifier());
        let mut td = server.td.clone();
        td["securityDefinitions"]["unsupported"] = json!({"scheme":"digest"});
        td["securityDefinitions"]["choice"] =
            json!({"scheme":"combo","oneOf":["unsupported","auth"]});
        td["security"] = json!(["choice"]);
        let owner = consumer_instance();
        let accounts = bank();
        let access = client_access(&owner, &td, true, false);
        let (client, content) = secure_client(&td, &keys, &owner, access, &accounts, None);
        let mut call = read_secure(&client);
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        assert!(prepare_producer(&td, keys.server(verifier())).is_err());
        td["securityDefinitions"]["choice"] =
            json!({"scheme":"combo","allOf":["unsupported","auth"]});
        assert!(
            secure_prepare(
                &td,
                &keys,
                client_access(&owner, &td, true, false),
                &accounts,
                None
            )
            .is_err()
        );
        assert!(prepare_producer(&td, keys.server(verifier())).is_err());
        drop((result, call, client, content, owner));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn producer_qualification_requires_every_component_and_distinguishable_placements() {
    let keys = Keys::for_name("127.0.0.1");
    let td = description_for(address(), json!(["both"]));
    assert!(prepare_producer(&td, keys.server(verifier())).is_err());
    assert!(prepare_producer(&td, server_security(&keys, None, Some(key_verifier()))).is_err());
    for expression in [json!(["auth", "another"]), json!(["choice"])] {
        let mut td = td.clone();
        td["securityDefinitions"]["another"] = json!({"scheme":"basic"});
        td["securityDefinitions"]["choice"] = json!({"scheme":"combo","oneOf":["auth","another"]});
        td["security"] = expression;
        assert!(
            prepare_producer(
                &td,
                server_security(&keys, Some(verifier()), Some(key_verifier()))
            )
            .is_err()
        );
    }
    for name in ["Authorization", "X-Api-Key", "x-API-kEy"] {
        let mut changed = td.clone();
        changed["securityDefinitions"]["key"]["name"] = json!(name);
        if name == "Authorization" {
            assert!(
                prepare_producer(
                    &changed,
                    server_security(&keys, Some(verifier()), Some(key_verifier()))
                )
                .is_err()
            );
        }
        changed["@context"] =
            json!(["https://www.w3.org/2022/wot/td/v1.1", {"h":"http://www.w3.org/2011/http#"}]);
        changed["properties"]["on"]["forms"][0]["h:headers"] =
            json!([{"h:fieldName":name,"h:fieldValue":"literal-must-not-be-a-credential"}]);
        let owner = consumer_instance();
        let accounts = bank();
        assert!(
            secure_prepare(
                &changed,
                &keys,
                client_access(&owner, &changed, true, true),
                &accounts,
                None
            )
            .is_err()
        );
        assert!(
            prepare_producer(
                &changed,
                server_security(&keys, Some(verifier()), Some(key_verifier()))
            )
            .is_err()
        );
        clean(&accounts);
    }
}

fn peer(
    addr: SocketAddr,
    config: Arc<rustls::ClientConfig>,
    headers: &str,
) -> std::thread::JoinHandle<Vec<u8>> {
    let headers = headers.to_owned();
    std::thread::spawn(move || {
        let socket = TcpStream::connect(addr).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let name = rustls::pki_types::ServerName::IpAddress(addr.ip().into());
        let connection = rustls::ClientConnection::new(config, name).unwrap();
        let mut stream = rustls::StreamOwned::new(connection, socket);
        write!(
            stream,
            "GET /on HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n{headers}\r\n"
        )
        .unwrap();
        stream.flush().unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        bytes
    })
}
async fn peer_result(
    server: &mut SecureFixture,
    peer: std::thread::JoinHandle<Vec<u8>>,
) -> Vec<u8> {
    let start = Instant::now();
    while !peer.is_finished() {
        server.step();
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert!(start.elapsed() < Duration::from_secs(8));
    }
    peer.join().unwrap()
}
#[test]
fn invalid_partial_extra_or_conflicting_evidence_never_falls_back_to_another_branch() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    io().block_on(async {
        for case in 0..5 {
            let keys = Keys::for_name("127.0.0.1");
            let addr = address();
            let basic = verifier();
            let key = key_verifier();
            let mut td = description_for(addr, json!(["choice"]));
            td["securityDefinitions"]["choice"] = json!({"scheme":"combo","oneOf":["none","both"]});
            if case == 3 {
                td["security"] = json!(["either"]);
            }
            if case == 4 {
                key.different_identity.store(true, Ordering::Release);
            }
            let mut server = SecureFixture::from_security(
                server_security(&keys, Some(basic), Some(key)),
                addr,
                td,
                &[Operation::ReadProperty],
            );
            server.app.allow_anonymous.set(true);
            let encoded =
                STANDARD.encode([USER, b":", if case == 1 { b"wrong" } else { PASSWORD }].concat());
            let headers = match case {
                0 => format!("Authorization: Basic {encoded}\r\n"),
                2 => format!("Authorization: Basic {encoded}\r\nX-Api-Key: wrong\r\n"),
                _ => format!(
                    "Authorization: Basic {encoded}\r\nX-Api-Key: {}\r\n",
                    std::str::from_utf8(API_KEY).unwrap()
                ),
            };
            let wire =
                peer_result(&mut server, peer(addr, keys.independent_client(), &headers)).await;
            assert!(
                wire.starts_with(b"HTTP/1.1 401"),
                "case {case}: {}",
                String::from_utf8_lossy(&wire)
            );
            assert_eq!(server.app.reads.get(), 0);
            server.close();
        }
    });
}

#[test]
fn https_nosec_has_no_authentication_service_but_still_applies_application_policy() {
    io().block_on(async {
        for allowed in [false, true] {
            let keys = Keys::for_name("127.0.0.1");
            let addr = address();
            let td = description_for(addr, json!(["none"]));
            let mut server = SecureFixture::from_security(
                server_security(&keys, None, None),
                addr,
                td,
                &[Operation::ReadProperty],
            );
            server.app.allow_anonymous.set(allowed);
            let owner = consumer_instance();
            let accounts = bank();
            let (client, content) = secure_client(
                &server.td,
                &keys,
                &owner,
                client_access(&owner, &server.td, false, false),
                &accounts,
                None,
            );
            let mut call = read_secure(&client);
            let result = server.exchange(&mut call).await;
            assert_eq!(result.result().is_ok(), allowed);
            assert_eq!(server.app.reads.get(), usize::from(allowed));
            drop((result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn whole_conjunction_revision_checks_fence_cross_verifier_changes_before_dispatch_and_delivery() {
    io().block_on(async {
        for accepted in [false, true] {
            let keys = Keys::for_name("127.0.0.1");
            let addr = address();
            let basic = verifier();
            let key = key_verifier();
            let td = description_for(addr, json!(["both"]));
            let mut server = SecureFixture::from_security(
                server_security(&keys, Some(basic.clone()), Some(key)),
                addr,
                td,
                &[Operation::ReadProperty],
            );
            let owner = consumer_instance();
            let accounts = bank();
            let (client, content) = secure_client(
                &server.td,
                &keys,
                &owner,
                client_access(&owner, &server.td, true, true),
                &accounts,
                None,
            );
            let mut call = read_secure(&client);
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
            KEY_HOOK.with(|h| {
                *h.borrow_mut() = Some(Box::new(move || {
                    basic.revision.fetch_add(1, Ordering::AcqRel);
                }))
            });
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            assert_eq!(server.app.reads.get(), usize::from(accepted));
            let settlement = server.server().take_settlement().unwrap();
            if accepted {
                assert_ne!(settlement.delivery, FiniteDelivery::Delivered);
            }
            if !accepted {
                assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
            }
            drop((settlement, result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn conjunction_write_preserves_effects_when_a_required_verifier_is_revoked_before_delivery() {
    io().block_on(async {
        for revoked in [false, true] {
            let keys = Keys::for_name("127.0.0.1");
            let addr = address();
            let key = key_verifier();
            let mut td = description_for(addr, json!(["both"]));
            td["properties"]["on"]["readOnly"] = json!(false);
            let mut server = SecureFixture::from_security(
                server_security(&keys, Some(verifier()), Some(key.clone())),
                addr,
                td,
                &[Operation::ReadProperty, Operation::WriteProperty],
            );
            let owner = consumer_instance();
            let accounts = bank();
            let (client, content) = secure_client(
                &server.td,
                &keys,
                &owner,
                client_access(&owner, &server.td, true, true),
                &accounts,
                None,
            );
            let mut call = client
                .write_property("on", payload(b"42"), Deadline::NONE, now(0), &mut work())
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
            if revoked {
                until(&mut server, &mut call, RawFiniteState::Delivering).await;
                key.revoked.store(true, Ordering::Release);
                key.revision.fetch_add(1, Ordering::AcqRel);
            }
            let result = server.exchange(&mut call).await;
            assert_eq!(result.result().is_ok(), !revoked);
            assert_eq!(server.app.writes.get(), 1);
            assert_eq!(&*server.app.value.borrow(), b"42");
            let settlement = server.server().take_settlement().unwrap();
            assert_eq!(settlement.effect, FiniteEffect::Completed);
            assert_eq!(settlement.delivery == FiniteDelivery::Delivered, !revoked);
            drop((settlement, result, call, client, content, owner));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn api_key_rotation_cancellation_and_shared_meaning_keep_original_instance_funding() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let addr = address();
        let td = description_for(addr, json!(["both"]));
        let mut server = SecureFixture::from_security(
            server_security(&keys, Some(verifier()), Some(key_verifier())),
            addr,
            td,
            &[Operation::ReadProperty],
        );
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        let bank_a = bank();
        let bank_b = bank();
        let access_a = client_access(&owner_a, &server.td, true, true);
        let access_b = client_access(&owner_b, &server.td, true, true);
        let (client_a, content) =
            secure_client(&server.td, &keys, &owner_a, access_a.clone(), &bank_a, None);
        let (client_b, shared) = secure_client(
            &server.td,
            &keys,
            &owner_b,
            access_b.clone(),
            &bank_b,
            Some(&content),
        );
        assert!(Arc::ptr_eq(&content, &shared));
        let revision_b = access_b.security_revision();
        let mut call_a = read_secure(&client_a);
        until(&mut server, &mut call_a, RawFiniteState::Delivering).await;
        access_a.replace_api_key(None).unwrap();
        let result_a = server.exchange(&mut call_a).await;
        assert!(result_a.result().is_err());
        assert_eq!(access_b.security_revision(), revision_b);
        drop(server.server().take_settlement());
        let mut call_b = read_secure(&client_b);
        let result_b = server.exchange(&mut call_b).await;
        assert!(result_b.result().is_ok());
        // Grants are audience-, registration-, operation- and incarnation-bound.
        owner_a.revoke();
        assert!(
            client_a
                .read_property("on", Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop(server.server().take_settlement());
        let mut cancelled = read_secure(&client_b);
        cancelled.cancel();
        let cancelled_result = server.exchange(&mut cancelled).await;
        assert!(cancelled_result.result().is_err());
        drop((
            call_a,
            call_b,
            cancelled,
            cancelled_result,
            client_a,
            client_b,
            owner_a,
            owner_b,
            access_a,
            access_b,
            content,
            shared,
        ));
        assert!(
            bank_a
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop((result_a, result_b));
        clean(&bank_a);
        clean(&bank_b);
        server.close();
    });
}

#[test]
fn cyclic_or_explosive_composition_refuses_the_whole_expression_without_truncation() {
    for explosive in [false, true] {
        let mut td = description_for(address(), json!(["root"]));
        if explosive {
            for i in 0..5 {
                td["securityDefinitions"][format!("c{i}")] =
                    json!({"scheme":"combo","oneOf":["none","auth"]});
            }
            td["securityDefinitions"]["root"] =
                json!({"scheme":"combo","allOf":["c0","c1","c2","c3","c4"]});
        } else {
            td["securityDefinitions"]["root"] = json!({"scheme":"combo","oneOf":["none","root"]});
        }
        let keys = Keys::for_name("127.0.0.1");
        let owner = consumer_instance();
        let accounts = bank();
        assert!(
            secure_prepare(
                &td,
                &keys,
                client_access(&owner, &td, true, false),
                &accounts,
                None
            )
            .is_err()
        );
        clean(&accounts);
    }
}

#[test]
fn mutated_typed_discriminators_cannot_turn_advertised_authentication_into_nosec() {
    use clinkz_wot_td::security_scheme::SecurityScheme;
    for combo in [false, true] {
        let keys = Keys::for_name("127.0.0.1");
        let td = description_for(address(), json!(["none"]));
        let mut source: Thing = serde_json::from_value(td.clone()).unwrap();
        let mut definition = if combo {
            SecurityScheme::combo_all_of(["none", "none"])
        } else {
            SecurityScheme::nosec()
        };
        match &mut definition {
            SecurityScheme::Combo(s) => s._context.scheme = "basic".into(),
            SecurityScheme::NoSec(s) => s._context.scheme = "basic".into(),
            _ => unreachable!(),
        }
        source
            .security_definitions
            .insert("mutated".into(), definition);
        source.security = vec!["mutated".into()];
        let owner = consumer_instance();
        let accounts = bank();
        let registration = try_https_consumer_registration(
            keys.client(),
            client_access(&owner, &td, false, false),
            BindingId::new(198),
            BindingGeneration::INITIAL,
            0,
        )
        .unwrap();
        let registry = ConsumerRegistry::try_new(vec![registration], &accounts)
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        assert!(
            registry
                .prepare(
                    &source,
                    GatewayDefaultV1::LIMITS,
                    accounts.clone(),
                    ConsumerSelector::default(),
                    &mut work(),
                    |job| {
                        while !job.is_finished() {
                            job.step(&mut work(), false);
                        }
                    }
                )
                .is_err()
        );
        drop(registry);
        clean(&accounts);
    }
}

#[test]
fn a_verifier_cannot_increase_the_next_verifiers_work_and_enter_its_callback() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let addr = address();
        let basic = verifier();
        let key = key_verifier();
        let td = description_for(addr, json!(["both"]));
        let mut server = SecureFixture::from_security(
            server_security(&keys, Some(basic.clone()), Some(key.clone())),
            addr,
            td,
            &[Operation::ReadProperty],
        );
        let owner = consumer_instance();
        let accounts = bank();
        let (client, content) = secure_client(
            &server.td,
            &keys,
            &owner,
            client_access(&owner, &server.td, true, true),
            &accounts,
            None,
        );
        let mut call = read_secure(&client);
        until(&mut server, &mut call, RawFiniteState::Dispatching).await;
        let changed = key.clone();
        VERIFICATION_HOOK.with(|h| {
            *h.borrow_mut() = Some((
                1,
                Box::new(move || {
                    changed.changed_work.store(true, Ordering::Release);
                }),
            ))
        });
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_err());
        assert_eq!(server.app.reads.get(), 0);
        assert_eq!(basic.calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            key.calls.load(Ordering::Relaxed),
            0,
            "changed verifier has no original work credit"
        );
        drop((result, call, client, content, owner));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn unsupported_combo_metadata_cannot_conceal_structural_cycles() {
    for proxy in [false, true] {
        let keys = Keys::for_name("127.0.0.1");
        let mut td = description_for(address(), json!(["root"]));
        td["securityDefinitions"]["root"] = json!({"scheme":"combo","oneOf":["none","loop"]});
        td["securityDefinitions"]["loop"] = json!({"scheme":"combo","allOf":["none","loop"]});
        if proxy {
            td["securityDefinitions"]["loop"]["proxy"] = json!("https://127.0.0.1/proxy");
        } else {
            td["securityDefinitions"]["loop"]["urn:uninterpreted"] = json!(true);
        }
        let owner = consumer_instance();
        let accounts = bank();
        assert!(
            secure_prepare(
                &td,
                &keys,
                client_access(&owner, &td, false, false),
                &accounts,
                None
            )
            .is_err(),
            "an unavailable cyclic alternative cannot grant the earlier anonymous branch"
        );
        clean(&accounts);
    }
}

#[test]
fn unavailable_containers_preserve_full_expansion_bounds_and_remain_unqualified() {
    for proxy in [false, true] {
        for overflow in [false, true] {
            let keys = Keys::for_name("127.0.0.1");
            let mut td = description_for(address(), json!(["root"]));
            td["securityDefinitions"]["root"] = json!({"scheme":"combo","oneOf":["none","joined"]});
            td["securityDefinitions"]["joined"] =
                json!({"scheme":"combo","allOf":["hidden","either"]});
            td["securityDefinitions"]["hidden"] = json!({"scheme":"combo","oneOf":if overflow { vec!["none";16] } else { vec!["none","auth"] }});
            if proxy {
                td["securityDefinitions"]["hidden"]["proxy"] = json!("https://127.0.0.1/proxy");
            } else {
                td["securityDefinitions"]["hidden"]["urn:uninterpreted"] = json!(true);
            }
            let owner = consumer_instance();
            let accounts = bank();
            let prepared = secure_prepare(
                &td,
                &keys,
                client_access(&owner, &td, false, false),
                &accounts,
                None,
            );
            assert_eq!(
                prepared.is_ok(),
                !overflow,
                "unsupported metadata cannot hide a 32-branch dependency"
            );
            assert!(prepare_producer(&td, server_security(&keys, None, None)).is_err());
            drop(prepared);
            clean(&accounts);
        }
    }
}
