#![cfg(feature = "prepare")]
#![allow(dead_code)]
//! Dynamic destinations through real sockets, shared preparation and live policy.
include!("support/security.rs");

struct Peer {
    address: SocketAddr,
    requests: Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
    paused: Arc<AtomicBool>,
    closed: Arc<std::sync::atomic::AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Peer {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let paused = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (seen, wait, done) = (requests.clone(), paused.clone(), stop.clone());
        let remote_closed = closed.clone();
        let thread = std::thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                let (mut socket, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                loop {
                    let mut bytes = [0; 1024];
                    let n = socket.read(&mut bytes).unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&bytes[..n]);
                    if let Some(head) = request.windows(4).position(|b| b == b"\r\n\r\n") {
                        let text = String::from_utf8_lossy(&request[..head]);
                        let size = text
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length: ")
                                    .map(|v| v.parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if request.len() >= head + 4 + size {
                            break;
                        }
                    }
                }
                if request.is_empty() {
                    continue;
                }
                let write = request.starts_with(b"PUT ");
                seen.lock().unwrap().push(request);
                socket.set_nonblocking(true).unwrap();
                let mut cancelled = false;
                while wait.load(Ordering::Acquire) && !done.load(Ordering::Acquire) {
                    if matches!(socket.peek(&mut [0; 1]), Ok(0)) {
                        remote_closed.fetch_add(1, Ordering::AcqRel);
                        cancelled = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                if cancelled {
                    continue;
                }
                socket.set_nonblocking(false).unwrap();
                let reply: &[u8] = if write {
                    b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n"
                } else {
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\nConnection: close\r\n\r\n7"
                };
                let _ = socket.write_all(reply);
            }
        });
        Self {
            address,
            requests,
            paused,
            closed,
            stop,
            thread: Some(thread),
        }
    }
    fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn dynamic_td(write: bool) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"logical multi-target interface",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "uriVariables":{
            "port":{"type":"integer","minimum":1,"maximum":65535},
            "q":{"type":"string","minLength":1,"maxLength":32},
            "item":{"type":"string","minLength":50},
            "unused":{"type":"boolean"}
        },
        "properties":{"level":{"type":"integer","minimum":0,"maximum":100,
            "readOnly":!write,"writeOnly":write,
            "uriVariables":{"item":{"type":"string","minLength":1,"maxLength":16}},
            "forms":[{"href":"http://127.0.0.1:{port}/v/{item}{?q}"}]}}})
}
fn definition(
    td: &Value,
    accounts: &Arc<HostResourceAccounts>,
) -> CoreResult<Arc<ConsumerDefinition>> {
    let registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip()
            .unwrap()
            .with_target_capacity(256)
            .unwrap(),
        BindingId::new(98),
        BindingGeneration::INITIAL,
        0,
    )?;
    let registry =
        ConsumerRegistry::try_new(vec![registration], accounts).unwrap_or_else(|_| panic!());
    let source: Thing = serde_json::from_value(td.clone()).map_err(|_| {
        CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
    })?;
    let prepared = registry.prepare(
        &source,
        GatewayDefaultV1::LIMITS,
        accounts.clone(),
        ConsumerSelector::default(),
        &mut work(),
        |job| {
            while !job.is_finished() {
                job.step(&mut work(), false);
            }
        },
    )?;
    let (bindings, funding) = prepared.into_execution();
    let result = bindings
        .first()
        .map(|b| b.definition().clone())
        .ok_or_else(|| {
            CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            ))
        });
    drop((bindings, funding));
    result
}
struct DestinationPolicy {
    instance: ConsumerInstanceRef,
    allowed: Vec<String>,
    denied: Arc<AtomicBool>,
    seen: std::sync::Mutex<Vec<String>>,
}
impl ConsumerAccessService for DestinationPolicy {
    fn authorize(
        &self,
        _: BindingRegistrationIdentity,
        target: &str,
        _: Operation,
    ) -> CoreResult<()> {
        if self.denied.load(Ordering::Acquire) || !self.allowed.iter().any(|t| t == target) {
            Err(denied())
        } else {
            Ok(())
        }
    }
    fn authorize_instance(
        &self,
        instance: &ConsumerInstanceRef,
        id: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        assert!(
            !target.contains('{'),
            "policy must receive the resolved destination"
        );
        self.seen.lock().unwrap().push(target.into());
        if instance != &self.instance {
            return Err(denied());
        }
        self.authorize(id, target, operation)
    }
    fn retained_bytes(&self) -> u64 {
        8192
    }
    fn work(&self) -> u64 {
        32
    }
}
fn bind_dynamic(
    definition: Arc<ConsumerDefinition>,
    owner: &ConsumerInstance,
    accounts: &Arc<HostResourceAccounts>,
    allowed: Vec<String>,
    denied: Arc<AtomicBool>,
) -> (StandaloneConsumerAssociation, ConsumerCalls) {
    let access = ConsumerServiceOwner::retain_access(
        Arc::new(DestinationPolicy {
            instance: owner.reference(),
            allowed,
            denied,
            seen: Default::default(),
        }),
        accounts,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let binding = definition
        .bind(access, accounts.clone(), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    StandaloneConsumerAssociation::try_new(
        owner.reference(),
        vec![binding],
        vec![],
        accounts.clone(),
    )
    .map(|association| {
        (
            association,
            ConsumerCalls::try_new(accounts.clone(), 2).unwrap(),
        )
    })
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
async fn finish(call: &mut ConsumerCall) -> ConsumerSettlement {
    let stop = Instant::now() + Duration::from_secs(5);
    loop {
        if let Poll::Ready(result) = call.poll(&mut cx(), now(0), &mut work()) {
            return result;
        }
        assert!(Instant::now() < stop, "dynamic call stalled");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn received(peer: &Peer, call: &mut ConsumerCall) {
    let stop = Instant::now() + Duration::from_secs(5);
    while peer.count() == 0 {
        assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
        assert!(Instant::now() < stop, "no native HTTP request");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn boolean_query_continuation_executes_and_relative_templates_refuse() {
    io().block_on(async {
        let peer = Peer::new();
        let accounts = bank();
        let owner = consumer_instance();
        let mut td = dynamic_td(false);
        td["base"] = json!(format!("http://{}/root/", peer.address));
        td["uriVariables"]["q"] = json!({"type":"boolean"});
        td["properties"]["level"]["forms"][0]["href"] = json!("v/{item}?fixed=1{&q}");
        assert!(definition(&td, &accounts).is_err());
        clean(&accounts);
        td["properties"]["level"]["forms"][0]["href"] = json!(format!(
            "http://{}/root/v/{{item}}?fixed=1{{&q}}",
            peer.address
        ));
        let definition = definition(&td, &accounts).unwrap();
        let client = bind_dynamic(
            definition.clone(),
            &owner,
            &accounts,
            vec![format!("http://{}/root/v/x?fixed=1&q=true", peer.address)],
            Arc::new(AtomicBool::new(false)),
        );
        drop(td);
        assert!(
            client
                .0
                .read_property_with_variables(
                    &client.1,
                    "level",
                    &[("item", "x"), ("q", "1")],
                    Deadline::NONE,
                    now(0),
                    &mut work()
                )
                .is_err()
        );
        let mut call = client
            .0
            .read_property_with_variables(
                &client.1,
                "level",
                &[("item", "x"), ("q", "true")],
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap();
        let result = finish(&mut call).await;
        assert!(result.result().is_ok());
        assert!(
            String::from_utf8_lossy(&peer.requests.lock().unwrap()[0])
                .starts_with("GET /root/v/x?fixed=1&q=true HTTP/1.1\r\n")
        );
        drop((client, definition, owner, call, result));
        clean(&accounts);
    });
}

#[test]
fn root_query_templates_execute_with_fixed_and_dynamic_ports() {
    io().block_on(async {
        let peer = Peer::new();
        let accounts = bank();
        for dynamic_port in [false, true] {
            let owner = consumer_instance();
            let mut td = dynamic_td(false);
            td["properties"]["level"]["forms"][0]["href"] = if dynamic_port {
                json!("http://127.0.0.1:{port}{?q}")
            } else {
                json!(format!("http://{}{{?q}}", peer.address))
            };
            let definition = definition(&td, &accounts).unwrap();
            let client = bind_dynamic(
                definition.clone(),
                &owner,
                &accounts,
                vec![format!("http://{}?q=root", peer.address)],
                Arc::new(AtomicBool::new(false)),
            );
            let port = peer.address.port().to_string();
            let mut values = vec![("q", "root")];
            if dynamic_port {
                values.push(("port", port.as_str()));
            }
            let mut call = client
                .0
                .read_property_with_variables(
                    &client.1,
                    "level",
                    &values,
                    Deadline::NONE,
                    now(0),
                    &mut work(),
                )
                .unwrap();
            let result = finish(&mut call).await;
            assert!(
                result.result().is_ok(),
                "{:?}; peer requests: {:?}",
                result.result().err(),
                peer.requests.lock().unwrap()
            );
            assert!(
                String::from_utf8_lossy(peer.requests.lock().unwrap().last().unwrap())
                    .starts_with("GET /?q=root HTTP/1.1\r\n")
            );
            drop((client, definition, owner, call, result));
            clean(&accounts);
        }
        assert_eq!(peer.count(), 2);
    });
}

#[test]
fn one_definition_changes_native_destinations_and_encodes_values_with_independent_instances() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let accounts = bank();
        let definition = definition(&dynamic_td(false), &accounts).unwrap();
        let content = definition.content().clone();
        assert_eq!(content.uri_variables().declarations().len(), 4);
        let (one, two) = (consumer_instance(), consumer_instance());
        assert_ne!(one.reference(), two.reference());
        let encoded = "north%2Fa%252F%3F?q=%E9%9B%AA%20%26%2F%3F%25%23";
        let allowed = vec![
            format!("http://{}/v/{encoded}", a.address),
            format!("http://{}/v/{encoded}", b.address),
        ];
        let first = bind_dynamic(
            definition.clone(),
            &one,
            &accounts,
            allowed.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        let second = bind_dynamic(
            definition.clone(),
            &two,
            &accounts,
            allowed,
            Arc::new(AtomicBool::new(false)),
        );
        let mut retained = Vec::new();
        for (runtime, peer) in [(&first, &a), (&first, &b), (&second, &b)] {
            let port = peer.address.port().to_string();
            let mut call = runtime
                .0
                .read_property_with_variables(
                    &runtime.1,
                    "level",
                    &[("port", &port), ("item", "north/a%2F?"), ("q", "雪 &/?%#")],
                    Deadline::NONE,
                    now(0),
                    &mut work(),
                )
                .unwrap();
            let result = finish(&mut call).await;
            assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
            assert!(
                String::from_utf8_lossy(peer.requests.lock().unwrap().last().unwrap())
                    .starts_with(&format!("GET /v/{encoded} HTTP/1.1\r\n"))
            );
            retained.push(result);
        }
        one.revoke();
        let port = b.address.port().to_string();
        assert!(
            first
                .0
                .read_property_with_variables(
                    &first.1,
                    "level",
                    &[("port", &port), ("item", "x"), ("q", "q")],
                    Deadline::NONE,
                    now(0),
                    &mut work()
                )
                .is_err()
        );
        let mut surviving = second
            .0
            .read_property_with_variables(
                &second.1,
                "level",
                &[("port", &port), ("item", "north/a%2F?"), ("q", "雪 &/?%#")],
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap();
        let result = finish(&mut surviving).await;
        assert!(result.result().is_ok());
        retained.push(result);
        drop(surviving);
        assert_eq!((a.count(), b.count()), (1, 3));
        drop((first, second, definition, content, one, two));
        assert!(
            accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        drop(retained);
        clean(&accounts);
    });
}

#[test]
fn malformed_missing_unsupported_and_denied_values_have_no_io_and_return_write_input() {
    let peer = Peer::new();
    let accounts = bank();
    let definition = definition(&dynamic_td(true), &accounts).unwrap();
    let owner = consumer_instance();
    let client = bind_dynamic(
        definition.clone(),
        &owner,
        &accounts,
        vec![],
        Arc::new(AtomicBool::new(false)),
    );
    let port = peer.address.port().to_string();
    let long = "x".repeat(129);
    for values in [
        vec![],
        vec![("port", port.as_str()), ("item", "x")],
        vec![("port", "01"), ("item", "x"), ("q", "q")],
        vec![("port", "65536"), ("item", "x"), ("q", "q")],
        vec![("port", "+1"), ("item", "x"), ("q", "q")],
        vec![("port", port.as_str()), ("item", ""), ("q", "q")],
        vec![("port", port.as_str()), ("item", "x"), ("q", long.as_str())],
        vec![
            ("port", port.as_str()),
            ("item", "x"),
            ("q", "q"),
            ("q", "other"),
        ],
        vec![
            ("port", port.as_str()),
            ("item", "x"),
            ("q", "q"),
            ("unused", "true"),
        ],
        vec![
            ("port", port.as_str()),
            ("item", "x"),
            ("q", "q"),
            ("unknown", "x"),
        ],
        // Well-formed and schema-valid, but actual destination permission denies.
        vec![("port", port.as_str()), ("item", "x"), ("q", "q")],
    ] {
        let input = payload(b"7");
        let rejected = client
            .0
            .write_property_with_variables(
                &client.1,
                "level",
                input,
                &values,
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .err()
            .expect("refused before I/O");
        assert_eq!(rejected.input().body.as_ref(), b"7");
        assert_eq!(peer.count(), 0);
    }
    drop((client, definition, owner));
    clean(&accounts);
}

#[test]
fn selected_assertions_templates_and_unsupported_producer_advertisement_refuse() {
    let accounts = bank();
    for (field, value) in [
        ("href", json!("http://127.0.0.1/{missing}")),
        ("href", json!("http://127.0.0.1/{+item}")),
        ("href", json!("http://127.0.0.1/{item:2}")),
        ("href", json!("http://127.0.0.1/{item*}")),
        ("href", json!("http://{item}/value")),
        ("href", json!("http://127.0.0.1/{item")),
        ("href", json!("http://127.0.0.1/%GG/{item}")),
    ] {
        let mut td = dynamic_td(false);
        td["properties"]["level"]["forms"][0][field] = value;
        assert!(definition(&td, &accounts).is_err(), "{td}");
        clean(&accounts);
    }
    for schema in [
        json!({"type":"string","pattern":"x"}),
        json!({"type":"integer","multipleOf":2}),
        json!({"type":"object"}),
        json!({"type":"array"}),
        json!({"type":"string","enum":["x"]}),
    ] {
        let mut td = dynamic_td(false);
        td["uriVariables"]["unused"] = schema;
        assert!(
            definition(&td, &accounts).is_err(),
            "unused selected assertions must qualify"
        );
        clean(&accounts);
    }
    let addr = address();
    let provider = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(addr).unwrap(),
        BindingId::new(97),
        BindingGeneration::INITIAL,
        1000,
    )
    .unwrap();
    let mut td = dynamic_td(false);
    td["properties"]["level"]["forms"][0]["href"] = json!(format!("http://{addr}/{{item}}"));
    let owner = authority(Generation::INITIAL);
    assert!(
        prepare_thing(
            &td.to_string(),
            owner.key(),
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |_, _| true
        )
        .is_err()
    );
    // Even an unused dynamic declaration cannot escape final Producer qualification.
    td["properties"]["level"]["forms"][0]["href"] = json!(format!("http://{addr}/value"));
    assert!(
        prepare_thing(
            &td.to_string(),
            owner.key(),
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |_, _| true
        )
        .is_err()
    );
}

#[test]
fn reuse_compares_complete_variable_meaning_without_rebuilding_shared_content() {
    let accounts = bank();
    let original = definition(&dynamic_td(false), &accounts).unwrap();
    let registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip()
            .unwrap()
            .with_target_capacity(256)
            .unwrap(),
        BindingId::new(98),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap();
    let registry =
        ConsumerRegistry::try_new(vec![registration], &accounts).unwrap_or_else(|_| panic!());
    for changed in [false, true] {
        let mut td = dynamic_td(false);
        td["properties"]["level"]["forms"][0]["href"] =
            json!("http://127.0.0.1:{port}/other/{item}?fixed=1{&q}");
        if changed {
            td["uriVariables"]["q"]["maxLength"] = json!(2);
        }
        let source: Thing = serde_json::from_value(td).unwrap();
        let prepared = registry.prepare_with_content(
            &source,
            GatewayDefaultV1::LIMITS,
            accounts.clone(),
            ConsumerSelector::default(),
            original.content(),
            &mut work(),
            |job| {
                while !job.is_finished() {
                    job.step(&mut work(), false);
                }
            },
        );
        if changed {
            assert!(prepared.is_err());
            continue;
        }
        let prepared = prepared.unwrap();
        assert_eq!(prepared.content_constructions(), 0);
        let (bindings, funding) = prepared.into_execution();
        assert!(Arc::ptr_eq(
            bindings[0].definition().content(),
            original.content()
        ));
        drop((bindings, funding));
    }
    drop((registry, original));
    clean(&accounts);
}

#[test]
fn actual_http_compatibility_and_expansion_capacity_refuse_before_network() {
    let peer = Peer::new();
    let accounts = bank();
    let owner = consumer_instance();
    let mut td = dynamic_td(false);
    td["uriVariables"]["port"] = json!({"type":"string"});
    let definition = definition(&td, &accounts).unwrap();
    let client = bind_dynamic(
        definition.clone(),
        &owner,
        &accounts,
        vec![],
        Arc::new(AtomicBool::new(false)),
    );
    for port in ["0", "65536", "bad"] {
        let error = client
            .0
            .read_property_with_variables(
                &client.1,
                "level",
                &[("port", port), ("item", "x"), ("q", "q")],
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Binding(_)));
    }
    let port = peer.address.port().to_string();
    let unicode = "😀".repeat(32); // 128 source bytes, 384 encoded bytes.
    assert!(
        client
            .0
            .read_property_with_variables(
                &client.1,
                "level",
                &[("port", &port), ("item", "x"), ("q", &unicode)],
                Deadline::NONE,
                now(0),
                &mut work()
            )
            .is_err()
    );
    assert_eq!(peer.count(), 0);
    drop((client, definition, owner));
    clean(&accounts);
}

#[test]
fn https_dynamic_resolution_uses_actual_audience_and_rejects_substituted_native_grants() {
    io().block_on(async {
        let keys = Keys::for_name("127.0.0.1");
        let v = verifier();
        let addr = address();
        let mut server = SecureFixture::from_td(
            &keys,
            v.clone(),
            addr,
            secure_description(addr),
            &[Operation::ReadProperty],
        );
        let accounts = bank();
        let owner = consumer_instance();
        let destination = format!("https://{addr}/on");
        let access = access(&owner, &destination, true, Arc::new(AtomicBool::new(false)));
        let mut td = server.td.clone();
        td["properties"]["on"]["uriVariables"] =
            json!({"resource":{"type":"string","minLength":1,"maxLength":4}});
        td["properties"]["on"]["forms"][0]["href"] = json!(format!("https://{addr}/{{resource}}"));
        let prepared = secure_prepare(&td, &keys, access.clone(), &accounts, None).unwrap();
        let (bindings, funding) = prepared.into_execution();
        assert_eq!(bindings.len(), 1);
        let definition = bindings[0].definition().clone();
        let client = StandaloneConsumerAssociation::try_new(
            owner.reference(),
            bindings,
            funding,
            accounts.clone(),
        )
        .map(|association| {
            (
                association,
                ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
            )
        })
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        drop(td);
        let mut call = client
            .0
            .read_property_with_variables(
                &client.1,
                "on",
                &[("resource", "on")],
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap();
        let result = server.exchange(&mut call).await;
        assert!(result.result().is_ok());
        drop((result, call, server.server().take_settlement()));
        let before = v.calls.load(Ordering::Acquire);
        assert!(matches!(
            client
                .0
                .read_property_with_variables(
                    &client.1,
                    "on",
                    &[("resource", "off")],
                    Deadline::NONE,
                    now(0),
                    &mut work()
                )
                .err()
                .unwrap(),
            CoreError::Security { .. }
        ));
        assert_eq!(v.calls.load(Ordering::Acquire), before);
        let artifact = definition.artifact();
        for actual in [destination.clone(), format!("https://{addr}/off")] {
            let grant = access
                .outbound_security(
                    &owner.reference(),
                    definition.binding(),
                    &destination,
                    Operation::ReadProperty,
                    definition.content().security(),
                )
                .unwrap();
            let raw = OutboundRequest::property_read(
                BindingArtifactRef::new(artifact.identity(), SlotIndex::new(0)),
                [("resource".into(), "off".into())].into_iter().collect(),
                None,
            )
            .unwrap()
            .with_destination(actual.into_boxed_str())
            .with_security(grant);
            assert!(
                definition.execution().start_finite(raw, artifact).is_err(),
                "native start must reject substituted destination or credential audience"
            );
        }
        assert_eq!(v.calls.load(Ordering::Acquire), before);
        drop((client, definition, access, owner));
        clean(&accounts);
        server.close();
    });
}

#[test]
fn a_late_native_value_retains_original_destination_and_funding_after_permission_loss() {
    io().block_on(async {
        let peer = Peer::new();
        let accounts = bank();
        let owner = consumer_instance();
        let definition = definition(&dynamic_td(false), &accounts).unwrap();
        let deny = Arc::new(AtomicBool::new(false));
        let client = bind_dynamic(
            definition.clone(),
            &owner,
            &accounts,
            vec![format!("http://{}/v/x?q=q", peer.address)],
            deny.clone(),
        );
        let port = peer.address.port().to_string();
        let mut call = client
            .0
            .read_property_with_variables(
                &client.1,
                "level",
                &[("port", &port), ("item", "x"), ("q", "q")],
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap();
        // Let native completion happen while shared output validation has no
        // credit. Revocation must preserve that returned value as late evidence.
        for _ in 0..50 {
            let mut budget = work().with_remaining(WorkClass::CodecInputBytes, 0);
            assert!(call.poll(&mut cx(), now(0), &mut budget).is_pending());
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(peer.count(), 1);
        deny.store(true, Ordering::Release);
        let result = finish(&mut call).await;
        assert!(matches!(result.result(), Err(CoreError::Security { .. })));
        assert!(matches!(
            result.native_outcome(),
            ConsumerNativeOutcome::Returned(Ok(_))
        ));
        drop((client, definition, owner, call));
        assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
        drop(result);
        clean(&accounts);
    });
}

#[test]
fn dynamic_write_executes_once_and_revoked_pending_work_keeps_original_cleanup() {
    io().block_on(async {
        let peer = Peer::new();
        let accounts = bank();
        let owner = consumer_instance();
        let definition = definition(&dynamic_td(true), &accounts).unwrap();
        let destination = format!("http://{}/v/lamp?q=on", peer.address);
        let deny = Arc::new(AtomicBool::new(false));
        let client = bind_dynamic(
            definition.clone(),
            &owner,
            &accounts,
            vec![destination],
            deny.clone(),
        );
        let port = peer.address.port().to_string();
        let values = [("port", port.as_str()), ("item", "lamp"), ("q", "on")];
        let mut call = client
            .0
            .write_property_with_variables(
                &client.1,
                "level",
                payload(b"7"),
                &values,
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let result = finish(&mut call).await;
        assert_eq!(result.effect(), ConsumerEffect::Completed);
        assert_eq!(peer.count(), 1);
        drop((result, call));
        peer.paused.store(true, Ordering::Release);
        let mut call = client
            .0
            .write_property_with_variables(
                &client.1,
                "level",
                payload(b"8"),
                &values,
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let stop = Instant::now() + Duration::from_secs(5);
        while peer.count() < 2 {
            assert!(call.poll(&mut cx(), now(0), &mut work()).is_pending());
            assert!(Instant::now() < stop);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        deny.store(true, Ordering::Release);
        let result = finish(&mut call).await;
        assert!(matches!(result.result(), Err(CoreError::Security { .. })));
        assert_eq!(result.effect(), ConsumerEffect::Unknown);
        assert_eq!(peer.count(), 2);
        assert!(matches!(
            result.native_outcome(),
            ConsumerNativeOutcome::Cancelled { .. }
        ));
        let stop = Instant::now() + Duration::from_secs(5);
        while peer.closed.load(Ordering::Acquire) == 0 {
            assert!(
                Instant::now() < stop,
                "cancelled dynamic socket was not released"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        drop((client, definition, owner, call));
        assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
        drop(result);
        clean(&accounts);
    });
}

#[path = "support/independent_capacity.rs"]
mod independent_capacity;
