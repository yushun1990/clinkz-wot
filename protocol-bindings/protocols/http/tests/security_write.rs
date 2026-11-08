#![cfg(feature = "prepare")]
#![allow(dead_code)]
//! Engine-to-Engine structured Property Write, plus independent TLS peers.
include!("support/security.rs");

const VALUE: &[u8] = br#"{"level":42,"meta":{"label":"lamp"}}"#;
fn write_td(addr: SocketAddr) -> Value {
    let mut td = secure_description(addr);
    td["properties"]["on"] = json!({
        "type":"object", "required":["level","meta"],
        "properties":{
            "level":{"type":"integer","minimum":0,"maximum":100},
            "meta":{"type":"object","required":["label"],
                "properties":{"label":{"type":"string","minLength":2,"maxLength":8}}}
        },
        "forms":[{"href":format!("https://{addr}/on")}]
    });
    td
}
fn producer(keys: &Keys, verifier: Arc<Verifier>) -> SecureFixture {
    let addr = address();
    SecureFixture::from_td(
        keys,
        verifier,
        addr,
        write_td(addr),
        &[Operation::ReadProperty, Operation::WriteProperty],
    )
}
fn write(client: &ConsumerRuntime, value: &[u8]) -> ConsumerCall {
    client
        .write_property("on", payload(value), Deadline::NONE, now(0), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn target(td: &Value) -> &str {
    td["properties"]["on"]["forms"][0]["href"].as_str().unwrap()
}
fn consumer(
    server: &SecureFixture,
    keys: &Keys,
    instance: &ConsumerInstance,
    accounts: &Arc<HostResourceAccounts>,
) -> (ConsumerRuntime, Arc<PreparedConsumerContent>) {
    secure_client(
        &server.td,
        keys,
        instance,
        access(
            instance,
            target(&server.td),
            true,
            Arc::new(AtomicBool::new(false)),
        ),
        accounts,
        None,
    )
}
fn assert_remote(remote: &RawFiniteSettlement, key: ProducerInstanceKey, effect: FiniteEffect) {
    assert_eq!(remote.recipient, Some(key));
    assert_eq!(remote.operation, Some(Operation::WriteProperty));
    assert_eq!(
        remote.binding,
        (BindingId::new(197), BindingGeneration::INITIAL)
    );
    assert_eq!(remote.effect, effect);
}

#[test]
fn structured_https_write_and_read_share_a_listener_with_independent_operation_definitions() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let mut server = producer(&keys, verifier());
        let instance = consumer_instance();
        let accounts = bank();
        let (client, content) = consumer(&server, &keys, &instance, &accounts);
        // Source TD/compiler owners are no longer needed by either execution path.
        server.td = Value::Null;
        let mut call = write(&client, VALUE);
        let result = server.exchange(&mut call).await;
        assert!(result.result().unwrap().data().is_none());
        assert_eq!(result.effect(), ConsumerEffect::Completed);
        assert_eq!(result.operation(), Operation::WriteProperty);
        assert_eq!(*server.app.value.borrow(), VALUE);
        assert_eq!(server.app.writes.get(), 1);
        assert_eq!(
            server.app.principal.borrow().as_ref().unwrap().id.as_str(),
            "verified-account"
        );
        let key = server.exposure.key();
        assert_eq!(
            *server.app.identity.borrow(),
            Some((
                key,
                Operation::WriteProperty,
                (BindingId::new(197), BindingGeneration::INITIAL)
            ))
        );
        let remote = server.server().take_settlement().unwrap();
        assert_remote(&remote, key, FiniteEffect::Completed);
        assert_eq!(remote.delivery, FiniteDelivery::Delivered);
        drop((result, call, remote));
        let mut read = read_secure(&client);
        let result = server.exchange(&mut read).await;
        assert_eq!(result.result().unwrap().data().unwrap().body, VALUE);
        assert_eq!(server.app.reads.get(), 1);
        drop(server.server().take_settlement());
        drop((client, read, instance, content));
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

// Deliberately bypasses Engine Consumer validation: a separately driven rustls
// client supplies untrusted wire input to the real, fully qualified Producer.
fn peer_put(
    keys: &Keys,
    addr: SocketAddr,
    body: &[u8],
    authorization: Option<&str>,
) -> std::thread::JoinHandle<Vec<u8>> {
    let config = keys.independent_client();
    let mut request = format!(
        "PUT /on HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(header) = authorization {
        request.push_str(&format!("Authorization: {header}\r\n"));
    }
    request.push_str("\r\n");
    let mut request = request.into_bytes();
    request.extend_from_slice(body);
    std::thread::spawn(move || {
        let socket = TcpStream::connect(addr).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let conn = rustls::ClientConnection::new(
            config,
            rustls::pki_types::ServerName::IpAddress(addr.ip().into()),
        )
        .unwrap();
        let mut stream = rustls::StreamOwned::new(conn, socket);
        stream.write_all(&request).unwrap();
        stream.flush().unwrap();
        let mut response = Vec::new();
        match stream.read_to_end(&mut response) {
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => (),
            Err(e) => panic!("TLS peer: {e}"),
        }
        response
    })
}
fn basic_header() -> String {
    use base64::Engine;
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode([USER, b":", PASSWORD].concat())
    )
}
async fn drive_peer(server: &mut SecureFixture, peer: std::thread::JoinHandle<Vec<u8>>) -> Vec<u8> {
    let start = Instant::now();
    while !peer.is_finished() {
        server.step();
        assert!(start.elapsed() < Duration::from_secs(6));
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    // Finish native settlement bookkeeping after the peer has observed EOF.
    for _ in 0..4 {
        server.step();
    }
    peer.join().unwrap()
}

#[test]
fn invalid_structured_values_refuse_both_engine_admission_and_independent_tls_ingress() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let mut server = producer(&keys, verifier());
        let instance = consumer_instance();
        let accounts = bank();
        let (client, content) = consumer(&server, &keys, &instance, &accounts);
        for value in [
            br#"{"level":101,"meta":{"label":"lamp"}}"#.as_slice(),
            br#"{"level":1,"meta":{"label":"x"}}"#,
            br#"{"level":1,"meta":{}}"#,
            br#"{"meta":{"label":"lamp"}}"#,
            br#"{"level":"1","meta":{"label":"lamp"}}"#,
            br#"{"level":1,"level":2,"meta":{"label":"lamp"}}"#,
        ] {
            let input = payload(value);
            let ptr = input.body.as_ptr();
            let rejection = client
                .write_property("on", input, Deadline::NONE, now(0), &mut work())
                .err()
                .expect("invalid value admitted");
            let (returned, _) = rejection.into_parts();
            assert_eq!(returned.body.as_ptr(), ptr);
            let peer = peer_put(&keys, server.addr, value, Some(&basic_header()));
            let response = drive_peer(&mut server, peer).await;
            assert!(response.starts_with(b"HTTP/1.1 500"));
            let key = server.exposure.key();
            let remote = server.server().take_settlement().unwrap();
            assert_remote(&remote, key, FiniteEffect::NotInvoked);
            assert_eq!(server.app.writes.get(), 0);
        }
        drop((client, content, instance));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn authentication_application_policy_and_destination_trust_refuse_before_behavior() {
    io().block_on(async {
        for case in 0..6 {
            let keys = Keys::for_name("127.0.0.1");
            let verifier = verifier();
            if case == 2 {
                verifier.replacement_identity.store(true, Ordering::Release);
            }
            let mut server = producer(&keys, verifier.clone());
            if case == 3 {
                server.app.deny();
            }
            let instance = consumer_instance();
            let accounts = bank();
            let credentials = if case == 0 {
                HttpBasicCredentials::try_new(USER, b"wrong").unwrap()
            } else {
                credentials()
            };
            let access = Arc::new(
                HttpClientAccess::try_new(
                    instance.reference(),
                    target(&server.td),
                    Arc::new(Access(Arc::new(AtomicBool::new(false)))),
                    Some(credentials),
                )
                .unwrap(),
            );
            let wrong_roots = Keys::for_name("127.0.0.1");
            let (client, content) = secure_client(
                &server.td,
                if case == 1 { &wrong_roots } else { &keys },
                &instance,
                access.clone(),
                &accounts,
                None,
            );
            if case == 4 {
                access.replace_credentials(None).unwrap();
            }
            if case == 5 {
                instance.revoke();
            }
            if case >= 4 {
                assert!(
                    client
                        .write_property("on", payload(VALUE), Deadline::NONE, now(0), &mut work())
                        .is_err()
                );
                assert_eq!(verifier.calls.load(Ordering::Relaxed), 0);
            } else {
                let mut call = write(&client, VALUE);
                let result = server.exchange(&mut call).await;
                assert!(result.result().is_err());
                assert_eq!(result.effect(), ConsumerEffect::Unknown);
                if case != 1 {
                    let key = server.exposure.key();
                    let remote = server.server().take_settlement().unwrap();
                    assert_remote(&remote, key, FiniteEffect::NotInvoked);
                } else {
                    assert_eq!(verifier.calls.load(Ordering::Relaxed), 0);
                }
            }
            assert_eq!(server.app.writes.get(), 0);
            drop((client, content, access, instance));
            clean(&accounts);
            server.close();
        }
        // Independent peer verifies the actual wire challenge/denial distinction.
        for (header, denied, status) in [
            (None, false, 401),
            (Some("Basic ZGV2aWNlOndyb25n"), false, 401),
            (Some(basic_header().as_str()), true, 403),
        ] {
            let keys = Keys::for_name("127.0.0.1");
            let mut server = producer(&keys, verifier());
            if denied {
                server.app.deny();
            }
            let peer = peer_put(&keys, server.addr, VALUE, header);
            let response = drive_peer(&mut server, peer).await;
            assert!(response.starts_with(format!("HTTP/1.1 {status}").as_bytes()));
            if status == 401 {
                assert!(
                    String::from_utf8_lossy(&response)
                        .to_lowercase()
                        .contains("www-authenticate: basic")
                );
            }
            assert_eq!(server.app.writes.get(), 0);
            let key = server.exposure.key();
            assert_remote(
                &server.server().take_settlement().unwrap(),
                key,
                FiniteEffect::NotInvoked,
            );
            server.close();
        }
    });
}

#[test]
fn forbidden_destination_refuses_preparation_and_current_policy_returns_original_input() {
    io().block_on(async {
        for wrong_destination in [false, true] {
            let keys = Keys::for_name("127.0.0.1");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let td = write_td(listener.local_addr().unwrap());
            let instance = consumer_instance();
            let accounts = bank();
            let denied = Arc::new(AtomicBool::new(false));
            let access = Arc::new(
                HttpClientAccess::try_new(
                    instance.reference(),
                    if wrong_destination {
                        "https://127.0.0.1:1/on"
                    } else {
                        target(&td)
                    },
                    Arc::new(Access(denied.clone())),
                    Some(credentials()),
                )
                .unwrap(),
            );
            if wrong_destination {
                assert!(secure_prepare(&td, &keys, access, &accounts, None).is_err());
            } else {
                let (client, content) =
                    secure_client(&td, &keys, &instance, access, &accounts, None);
                denied.store(true, Ordering::Release);
                let input = payload(VALUE);
                let ptr = input.body.as_ptr();
                let rejection = client
                    .write_property("on", input, Deadline::NONE, now(0), &mut work())
                    .err()
                    .unwrap();
                assert_eq!(rejection.into_parts().0.body.as_ptr(), ptr);
                drop((client, content));
            }
            listener.set_nonblocking(true).unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
            );
            drop(instance);
            clean(&accounts);
        }
    });
}

#[test]
fn callback_changes_before_dispatch_cannot_enter_write_behavior() {
    io().block_on(async {
        // A final verifier changing policy but returning the same identity is
        // the counterexample to verification/authorization/verification alone.
        for case in 0..8 {
            let keys = Keys::for_name("127.0.0.1");
            let verifier = verifier();
            let mut server = producer(&keys, verifier.clone());
            let instance = consumer_instance();
            let accounts = bank();
            let (client, content) = consumer(&server, &keys, &instance, &accounts);
            let mut call = write(&client, VALUE);
            until(&mut server, &mut call, RawFiniteState::Dispatching).await;
            let key = server.exposure.key();
            let app = server.app.clone();
            let gate = ClosedProducerParticipation::ingress_gate(server.server()).clone();
            match case {
                0 => {
                    server.owner.revoke(key);
                }
                1 => {
                    *server.app.authority_in_permission.borrow_mut() =
                        Some(Box::new(move || gate.set(false)))
                }
                2 | 3 | 4 => {
                    let verifier_hook = verifier.clone();
                    VERIFICATION_HOOK.with(|pending| {
                        *pending.borrow_mut() = Some((
                            2,
                            Box::new(move || match case {
                                2 => app.deny(),
                                3 => gate.set(false),
                                _ => {
                                    verifier_hook.revision.fetch_add(1, Ordering::AcqRel);
                                }
                            }),
                        ))
                    });
                }
                5 => *server.app.revoke_in_permission.borrow_mut() = Some(verifier.clone()),
                6 | 7 => {
                    let revocation = server.owner.revocation(key).unwrap();
                    assert_eq!(revocation.key(), key);
                    if case == 6 {
                        *server.app.authority_in_permission.borrow_mut() =
                            Some(Box::new(move || revocation.revoke()));
                    } else {
                        VERIFICATION_HOOK.with(|pending| {
                            *pending.borrow_mut() = Some((2, Box::new(move || revocation.revoke())))
                        });
                    }
                }
                _ => unreachable!(),
            }
            server.step();
            assert_eq!(server.app.writes.get(), 0, "case {case}");
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            let remote = server.server().take_settlement().unwrap();
            assert_remote(&remote, key, FiniteEffect::NotInvoked);
            drop((result, remote, call, client, content, instance));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn effects_and_original_identity_survive_lost_delivery_and_callback_revocation() {
    io().block_on(async {
        for case in 0..8 {
            let keys = Keys::for_name("127.0.0.1");
            let verifier = verifier();
            let mut server = producer(&keys, verifier.clone());
            let instance = consumer_instance();
            let accounts = bank();
            let (client, content) = consumer(&server, &keys, &instance, &accounts);
            let mut call = write(&client, VALUE);
            until(&mut server, &mut call, RawFiniteState::Delivering).await;
            assert_eq!(server.app.writes.get(), 1);
            let key = server.exposure.key();
            match case {
                0 => server.server().begin_close(), // response lost after effects
                1 => server.app.deny(),
                2 => {
                    server.owner.revoke(key);
                }
                3 => {
                    verifier.revoked.store(true, Ordering::Release);
                }
                4 | 5 => {
                    let app = server.app.clone();
                    let verifier_hook = verifier.clone();
                    let at = verifier.calls.load(Ordering::Relaxed) + 2;
                    VERIFICATION_HOOK.with(|pending| {
                        *pending.borrow_mut() = Some((
                            at,
                            Box::new(move || {
                                if case == 4 {
                                    app.deny();
                                } else {
                                    verifier_hook.revision.fetch_add(1, Ordering::AcqRel);
                                }
                            }),
                        ))
                    });
                }
                6 | 7 => {
                    let revocation = server.owner.revocation(key).unwrap();
                    if case == 6 {
                        *server.app.authority_in_permission.borrow_mut() =
                            Some(Box::new(move || revocation.revoke()));
                    } else {
                        let at = verifier.calls.load(Ordering::Relaxed) + 2;
                        VERIFICATION_HOOK.with(|pending| {
                            *pending.borrow_mut() =
                                Some((at, Box::new(move || revocation.revoke())))
                        });
                    }
                }
                _ => unreachable!(),
            }
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err(), "case {case}");
            assert_eq!(result.effect(), ConsumerEffect::Unknown);
            let remote = server.server().take_settlement().unwrap();
            assert_remote(&remote, key, FiniteEffect::Completed);
            assert_eq!(remote.delivery, FiniteDelivery::NotDelivered);
            assert!(remote.result.is_ok());
            for _ in 0..5 {
                client.step(&mut cx(), now(0), &mut work());
            }
            assert_eq!(server.app.writes.get(), 1, "never replay uncertain writes");
            drop((result, remote, call, client, content, instance));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn handler_error_or_invalid_output_after_effects_never_proves_rollback() {
    io().block_on(async {
        for fault in [1, 2] {
            let keys = Keys::for_name("127.0.0.1");
            let mut server = producer(&keys, verifier());
            server.app.write_fault.set(fault);
            let instance = consumer_instance();
            let accounts = bank();
            let (client, content) = consumer(&server, &keys, &instance, &accounts);
            let mut call = write(&client, VALUE);
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            assert_eq!(result.effect(), ConsumerEffect::Unknown);
            let key = server.exposure.key();
            assert_remote(
                &server.server().take_settlement().unwrap(),
                key,
                FiniteEffect::Unknown,
            );
            assert_eq!(*server.app.value.borrow(), VALUE);
            assert_eq!(server.app.writes.get(), 1);
            drop((result, call, client, content, instance));
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn cancellation_before_network_or_after_effects_releases_original_resources() {
    io().block_on(async {
        for after_effect in [false, true] {
            let keys = Keys::for_name("127.0.0.1");
            let mut server = producer(&keys, verifier());
            let instance = consumer_instance();
            let accounts = bank();
            let (client, content) = consumer(&server, &keys, &instance, &accounts);
            let mut call = write(&client, VALUE);
            if after_effect {
                until(&mut server, &mut call, RawFiniteState::Delivering).await;
            }
            call.cancel();
            let result = server.exchange(&mut call).await;
            assert!(matches!(result.result(), Err(CoreError::Cancelled(_))));
            assert_eq!(server.app.writes.get(), usize::from(after_effect));
            if after_effect {
                // Local cancellation grants no remote cancellation/rollback.
                for _ in 0..10 {
                    server.step();
                }
                let key = server.exposure.key();
                let remote = server.server().take_settlement().unwrap();
                assert_remote(&remote, key, FiniteEffect::Completed);
            }
            drop((call, client, content, instance));
            assert!(
                accounts
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .used()
                    > 0
            );
            drop(result);
            clean(&accounts);
            server.close();
        }
    });
}

thread_local! { static ACCESS_HOOK: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) }; }
struct CallbackPolicy;
impl ConsumerAccessService for CallbackPolicy {
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        self.authorize(binding, target, operation)
    }
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        let hook = ACCESS_HOOK.with(|p| p.borrow_mut().take());
        if let Some(hook) = hook {
            hook();
        }
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
    fn work(&self) -> u64 {
        1
    }
}
#[test]
fn reentrant_policy_cancellation_fences_native_start_and_preserves_completed_unread_write() {
    io().block_on(async {
        for mode in 0..4 {
            let unread = mode != 0;
            let keys = Keys::for_name("127.0.0.1");
            let mut server = producer(&keys, verifier());
            let instance = Rc::new(consumer_instance());
            let accounts = bank();
            let access = Arc::new(
                HttpClientAccess::try_new(
                    instance.reference(),
                    target(&server.td),
                    Arc::new(CallbackPolicy),
                    Some(credentials()),
                )
                .unwrap(),
            );
            let (client, content) = secure_client(
                &server.td,
                &keys,
                &instance,
                access.clone(),
                &accounts,
                None,
            );
            let client = Rc::new(client);
            let call = Rc::new(RefCell::new(write(&client, VALUE)));
            if unread {
                let start = Instant::now();
                // Progress without taking the original settlement.
                while server.app.writes.get() == 0
                    || server.server().state() != RawFiniteState::Serving
                {
                    server.step();
                    client.step(&mut cx(), now(0), &mut work());
                    assert!(start.elapsed() < Duration::from_secs(5));
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                for _ in 0..10 {
                    client.step(&mut cx(), now(0), &mut work());
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            let nested_call = call.clone();
            let nested_client = client.clone();
            let nested_instance = instance.clone();
            ACCESS_HOOK.with(|p| {
                *p.borrow_mut() = Some(Box::new(move || {
                    match mode {
                        2 => access.replace_credentials(None).unwrap(),
                        3 => nested_instance.revoke(),
                        _ => nested_call.borrow().cancel(),
                    }
                    // The checked-out original slot remains occupied. No RefCell
                    // registry borrow surrounds the access callback.
                    nested_client.step(&mut cx(), now(0), &mut work());
                    assert!(!nested_client.is_idle());
                }))
            });
            client.step(&mut cx(), now(0), &mut work());
            let mut call = Rc::try_unwrap(call).ok().unwrap().into_inner();
            let result = server.exchange(&mut call).await;
            assert!(result.result().is_err());
            if mode <= 1 {
                assert!(matches!(result.result(), Err(CoreError::Cancelled(_))));
            }
            assert_eq!(server.app.writes.get(), usize::from(unread));
            if unread {
                assert_eq!(result.effect(), ConsumerEffect::Completed);
            }
            drop((call, client, content, instance));
            assert!(
                accounts
                    .account(ResourceKind::EngineLiveBytesGlobalMax)
                    .used()
                    > 0
            );
            drop(result);
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn independent_tls_server_requires_complete_write_evidence_and_never_receives_a_replay() {
    io().block_on(async {
        for response in [
            b"HTTP/1.1 204 No Content\r\n\r\n".as_slice(),
            b"HTTP/1.1 202 Accepted\r\n\r\n",
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n",
            b"HTTP/1.1 204 No Content\r\n\r\nx",
            b"",
        ] {
            let keys = Keys::for_name("127.0.0.1");
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![keys.cert.clone()], keys.key.clone_key())
            .unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let peer = std::thread::spawn(move || {
                let (socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut stream = rustls::StreamOwned::new(
                    rustls::ServerConnection::new(Arc::new(config)).unwrap(),
                    socket,
                );
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.ends_with(VALUE) {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&buffer[..n]);
                    assert!(request.len() <= 8192);
                }
                assert!(request.starts_with(b"PUT /on HTTP/1.1\r\n"));
                assert!(String::from_utf8_lossy(&request).contains(&basic_header()));
                // The peer has accepted/applied the complete value. Every error
                // response, including 403, remains ambiguous to the Consumer.
                stream.write_all(response).unwrap();
                stream.conn.send_close_notify();
                stream.flush().unwrap();
                drop(stream);
                listener
            });
            let accounts = bank();
            let instance = consumer_instance();
            let td = write_td(addr);
            let (client, content) = secure_client(
                &td,
                &keys,
                &instance,
                access(
                    &instance,
                    target(&td),
                    true,
                    Arc::new(AtomicBool::new(false)),
                ),
                &accounts,
                None,
            );
            let mut call = write(&client, VALUE);
            let start = Instant::now();
            let result = loop {
                if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) {
                    break result;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            let complete = response == b"HTTP/1.1 204 No Content\r\n\r\n";
            assert_eq!(result.result().is_ok(), complete);
            assert_eq!(
                result.effect(),
                if complete {
                    ConsumerEffect::Completed
                } else {
                    ConsumerEffect::Unknown
                }
            );
            for _ in 0..10 {
                client.step(&mut cx(), now(0), &mut work());
            }
            let listener = peer.join().unwrap();
            listener.set_nonblocking(true).unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
            );
            drop((result, call, client, content, instance));
            clean(&accounts);
        }
    });
}

#[test]
fn basic_support_still_qualifies_every_operation_form_schema_and_actual_server_service() {
    let keys = Keys::for_name("127.0.0.1");
    let addr = address();
    let td = write_td(addr);
    let provider = HttpFiniteProducer::try_https(
        HttpProducerConfig::for_https_loopback(addr).unwrap(),
        BindingId::new(197),
        BindingGeneration::INITIAL,
        1000,
        keys.server(verifier()),
    )
    .unwrap();
    let qualify = |td: &Value| {
        prepare_thing(
            &td.to_string(),
            authority(Generation::INITIAL).key(),
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |_, _| true,
        )
    };
    let supported = qualify(&td).unwrap();
    assert_eq!(supported.operations().len(), 2);
    let wrong_service = HttpFiniteConfig::try_from_mappings(
        HttpProducerConfig::for_https_loopback(addr).unwrap(),
        supported
            .operations()
            .iter()
            .map(|op| {
                (
                    AffordanceTarget::Property(Arc::from("on")),
                    op.definition::<HttpProducerProgram>().unwrap().clone(),
                )
            })
            .collect(),
        authority(Generation::INITIAL).key(),
        1000,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(
        wrong_service
            .with_security(keys.server(verifier()))
            .is_err()
    );
    for case in 0..8 {
        let mut changed = td.clone();
        match case {
            0 => {
                changed["forms"] =
                    json!([{"op":"writeallproperties", "href":format!("https://{addr}/on")}])
            }
            1 => {
                changed["actions"] = json!({"run":{"synchronous":true,"forms":[{"op":"invokeaction","href":format!("https://{addr}/on")} ]}})
            }
            2 => {
                changed["properties"]["on"]["observable"] = json!(true);
                changed["properties"]["on"]["forms"][0]["op"] = json!([
                    "readproperty",
                    "writeproperty",
                    "observeproperty",
                    "unobserveproperty"
                ]);
            }
            3 => changed["properties"]["on"]["forms"]
                .as_array_mut()
                .unwrap()
                .push(json!({"op":"writeproperty", "href":format!("http://{addr}/other")})),
            4 => {
                changed["properties"]["on"]["properties"]["meta"]["properties"]["label"]["pattern"] =
                    json!("[a-z]+")
            }
            5 => {
                changed["properties"]["on"]["forms"][0]["response"] =
                    json!({"htv:statusCodeValue":202})
            }
            6 => changed["properties"]["on"]["forms"][0]["security"] = json!(["auth", "auth"]),
            _ => {
                changed["properties"]["on"]["readOnly"] = json!(true);
                changed["properties"]["on"]["forms"][0]["op"] =
                    json!(["readproperty", "writeproperty"]);
            }
        }
        assert!(qualify(&changed).is_err(), "case {case}");
        if case <= 2 {
            let instance = consumer_instance();
            let accounts = bank();
            let prepared = secure_prepare(
                &changed,
                &keys,
                access(
                    &instance,
                    target(&changed),
                    true,
                    Arc::new(AtomicBool::new(false)),
                ),
                &accounts,
                None,
            )
            .unwrap();
            for support in prepared.support() {
                assert_eq!(
                    support.selected,
                    matches!(
                        support.operation,
                        Operation::ReadProperty | Operation::WriteProperty
                    )
                );
            }
            drop(prepared);
            clean(&accounts);
        }
    }
    drop(TcpListener::bind(addr).unwrap()); // preparation created no listener
}

#[test]
fn write_content_reuse_keeps_credentials_policy_and_authority_with_each_instance() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let make = |password| {
            let addr = address();
            let mut td = write_td(addr);
            td["properties"]["on"]["writeOnly"] = json!(true);
            SecureFixture::from_td(
                &keys,
                verifier_password(password),
                addr,
                td,
                &[Operation::WriteProperty],
            )
        };
        let mut a = make(PASSWORD);
        let mut b = make(b"second-password");
        let owner_a = consumer_instance();
        let owner_b = consumer_instance();
        let bank_a = bank();
        let bank_b = bank();
        let access_a = access(
            &owner_a,
            target(&a.td),
            true,
            Arc::new(AtomicBool::new(false)),
        );
        let access_b = Arc::new(
            HttpClientAccess::try_new(
                owner_b.reference(),
                target(&b.td),
                Arc::new(Access(Arc::new(AtomicBool::new(false)))),
                Some(HttpBasicCredentials::try_new(USER, b"second-password").unwrap()),
            )
            .unwrap(),
        );
        let (client_a, content_a) =
            secure_client(&a.td, &keys, &owner_a, access_a.clone(), &bank_a, None);
        let (client_b, content_b) =
            secure_client(&b.td, &keys, &owner_b, access_b, &bank_b, Some(&content_a));
        assert!(Arc::ptr_eq(&content_a, &content_b));
        let mut call_a = write(&client_a, VALUE);
        let mut call_b = write(&client_b, VALUE);
        let result_a = a.exchange(&mut call_a).await;
        let result_b = b.exchange(&mut call_b).await;
        assert_eq!(result_a.effect(), ConsumerEffect::Completed);
        assert_eq!(result_b.effect(), ConsumerEffect::Completed);
        drop(a.server().take_settlement());
        drop(b.server().take_settlement());
        access_a.replace_credentials(None).unwrap();
        owner_a.revoke();
        assert!(
            client_a
                .write_property("on", payload(VALUE), Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        drop((result_b, call_b));
        let mut call_b = write(&client_b, VALUE);
        let result_b = b.exchange(&mut call_b).await;
        assert!(result_b.result().is_ok());
        assert_eq!(a.app.writes.get(), 1);
        assert_eq!(b.app.writes.get(), 2);
        // A fresh instance/credential service binds the same immutable content.
        let fresh = consumer_instance();
        let (fresh_client, fresh_content) = secure_client(
            &a.td,
            &keys,
            &fresh,
            access(
                &fresh,
                target(&a.td),
                true,
                Arc::new(AtomicBool::new(false)),
            ),
            &bank_a,
            Some(&content_a),
        );
        let mut fresh_call = write(&fresh_client, VALUE);
        let fresh_result = a.exchange(&mut fresh_call).await;
        assert!(fresh_result.result().is_ok());
        drop((
            fresh_call,
            fresh_result,
            fresh_client,
            fresh_content,
            fresh,
            client_a,
            client_b,
            call_a,
            call_b,
            result_b,
            content_b,
            owner_a,
            owner_b,
            access_a,
        ));
        clean(&bank_b);
        assert!(
            bank_a
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop((result_a, content_a));
        clean(&bank_a);
        a.close();
        b.close();
    });
}

#[test]
fn retained_revocation_handle_keeps_original_capacity_until_final_control_release() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let mut server = producer(&keys, verifier());
        let instance = consumer_instance();
        let accounts = bank();
        let (client, content) = consumer(&server, &keys, &instance, &accounts);
        let revocation = server.owner.revocation(server.exposure.key()).unwrap();
        let mut call = write(&client, VALUE);
        let result = server.exchange(&mut call).await;
        assert_eq!(result.effect(), ConsumerEffect::Completed);
        drop((result, call, client, content, instance));
        clean(&accounts);
        revocation.revoke();
        server.exposure.begin_close();
        for _ in 0..100 {
            server.step();
            if server.server().state() == RawFiniteState::Closed {
                break;
            }
        }
        assert_eq!(server.server().state(), RawFiniteState::Closed);
        drop(server.server().take_settlement());
        server
            .exposure
            .retire(&mut server.owner, &mut work())
            .unwrap();
        let producer_accounts = server.accounts.clone();
        let addr = server.addr;
        drop(server);
        drop(TcpListener::bind(addr).unwrap());
        assert!(
            producer_accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        revocation.revoke(); // no live old instance, no lookup of a successor
        drop(revocation);
        clean(&producer_accounts);
    });
}

#[test]
fn secure_write_prepays_input_admission_and_verification_before_effects() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let verifier = verifier();
        let mut server = producer(&keys, verifier.clone());
        let instance = consumer_instance();
        let accounts = bank();
        let (client, content) = consumer(&server, &keys, &instance, &accounts);
        let input = payload(VALUE);
        let ptr = input.body.as_ptr();
        let rejected = client
            .write_property("on", input, Deadline::NONE, now(0), &mut WorkBudget::new())
            .err()
            .unwrap();
        assert_eq!(rejected.into_parts().0.body.as_ptr(), ptr);
        assert_eq!(verifier.calls.load(Ordering::Relaxed), 0);
        let mut call = write(&client, VALUE);
        until(&mut server, &mut call, RawFiniteState::Dispatching).await;
        let mut unpaid = work();
        unpaid.set_remaining(WorkClass::HandlerSteps, 0);
        assert!(
            server
                .owner
                .step(
                    server.exposure.participant_mut::<SecureServer>(0).unwrap(),
                    &mut cx(),
                    &mut unpaid,
                    now(0)
                )
                .unwrap()
                .is_pending()
        );
        assert_eq!(verifier.calls.load(Ordering::Relaxed), 0);
        assert_eq!(server.app.writes.get(), 0);
        let result = server.exchange(&mut call).await;
        assert_eq!(result.effect(), ConsumerEffect::Completed);
        assert_eq!(server.app.writes.get(), 1);
        drop((result, call, client, content, instance));
        clean(&accounts);
        server.close();
    });
}
