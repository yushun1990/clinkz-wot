#![cfg(feature = "prepare")]
//! Real native collection selection, original-input validation and retirement.
include!("support/collection.rs");

fn selected_description(addr: SocketAddr) -> Value {
    let mut td = description(addr);
    td["forms"] = json!([{"op":"readmultipleproperties", "href":format!("http://{addr}/properties"), "htv:methodName":"POST"}]);
    td
}
fn start_selected(client: &(ConsumerAssociation, ConsumerCalls), names: &[&str]) -> ConsumerCall {
    client
        .0
        .read_multiple_properties(&client.1, names, Deadline::NONE, now(0), &mut work())
        .unwrap()
}
fn body(result: &ConsumerSettlement) -> Value {
    serde_json::from_slice(result.result().unwrap().data().unwrap().body).unwrap()
}

#[test]
fn producer_rejects_undeclared_selected_read_status_and_branch_before_native_encoding() {
    io().block_on(async {
        let addr = address();
        let td = selected_description(addr);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        for (status, selection) in [
            (
                InteractionStatus::ApplicationError,
                ResponseSelection::Primary,
            ),
            (InteractionStatus::Ok, ResponseSelection::Additional(0)),
            (
                InteractionStatus::ApplicationError,
                ResponseSelection::Additional(0),
            ),
        ] {
            server.app.selected_status.set(status);
            server.app.selected_response.set(selection);
            let mut call = start_selected(&client, &["on"]);
            let (local, remote) = server.exchange(&mut call).await;
            assert!(
                matches!(remote.result, Err(CoreError::Validation(context))
                if context.phase() == ErrorPhase::Validate),
                "valid selected data cannot certify an undeclared response status/branch"
            );
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert!(local.result().is_err());
        }
        assert_eq!(server.app.roots.get(), 3);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn same_td_supports_different_per_call_subsets_and_distinct_read_all() {
    io().block_on(async {
        let addr = address();
        let mut td = description(addr);
        // Same native path is valid: the Binding distinguishes GET and POST.
        td["forms"]
            .as_array_mut()
            .unwrap()
            .push(selected_description(addr)["forms"][0].clone());
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let prepared = prepare_consumer(
            &td,
            accounts.clone(),
            vec![registration()],
            ConsumerSelector::default(),
        )
        .unwrap();
        let (definitions, funding) = prepared.into_execution();
        let root = definitions
            .iter()
            .find(|d| d.operation() == Operation::ReadMultipleProperties)
            .unwrap();
        assert_eq!(root.affordance(), &AffordanceTarget::Thing);
        assert_eq!(root.form(), 1);
        assert_eq!(root.binding().binding_id(), BindingId::new(98));
        let client = ConsumerAssociation::try_new(
            owner.reference(),
            definitions,
            funding,
            accounts.clone(),
        )
        .map(|association| {
            (
                association,
                ConsumerCalls::try_new(accounts.clone(), 2).unwrap(),
            )
        })
        .unwrap_or_else(|_| panic!());
        drop(td);
        for (names, expected) in [
            (vec!["level"], json!({"level":7})),
            (vec!["on"], json!({"on":true})),
            (vec!["on", "level"], json!({"level":7,"on":true})),
            (vec![], json!({})),
        ] {
            let mut call = start_selected(&client, &names);
            drop(names); // invocation input is independent of caller storage
            let (local, remote) = server.exchange(&mut call).await;
            assert_eq!(body(&local), expected);
            assert_eq!(local.operation(), Operation::ReadMultipleProperties);
            assert_eq!(local.consumer(), &owner.reference());
            assert_eq!(
                local.affordance(),
                clinkz_wot_core::thing::AffordanceSite::Thing
            );
            assert_eq!(remote.recipient, Some(server.exposure.key()));
            assert_eq!(remote.operation, Some(Operation::ReadMultipleProperties));
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert_eq!(remote.delivery, FiniteDelivery::Delivered);
            assert_eq!(
                serde_json::from_slice::<Value>(&remote.result.unwrap().data().unwrap().body)
                    .unwrap(),
                expected
            );
        }
        assert_eq!(server.app.selections.borrow().len(), 4);
        let mut all = start_read(&client);
        let (local, remote) = server.exchange(&mut all).await;
        assert_eq!(body(&local), json!({"level":7,"on":true}));
        assert_eq!(remote.operation, Some(Operation::ReadAllProperties));
        assert_eq!(server.controls.requests.get(), 5);
        assert_eq!(server.app.roots.get(), 5);
        assert_eq!(server.app.reads.get(), 0);
        assert_eq!(server.app.writes.get(), 0);
        drop(local);
        drop(all);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn unsupported_selections_and_unpaid_encoding_have_no_native_effect() {
    io().block_on(async {
        let addr = address();
        let td = selected_description(addr);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
        let long = "x".repeat(257);
        for names in [
            vec!["absent"],
            vec!["secret"],
            vec!["level", "level"],
            vec![" level"],
            vec![long.as_str()],
            vec!["on"; 33],
        ] {
            assert!(
                client
                    .0
                    .read_multiple_properties(
                        &client.1,
                        &names,
                        Deadline::NONE,
                        now(0),
                        &mut work()
                    )
                    .is_err()
            );
            assert_eq!(
                baseline,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
        }
        for class in [
            WorkClass::CodecInputBytes,
            WorkClass::CleanupItems,
            WorkClass::BindingPolls,
            WorkClass::PlanningItems,
        ] {
            let mut unpaid = work().with_remaining(class, 0);
            assert!(
                client
                    .0
                    .read_multiple_properties(
                        &client.1,
                        &["on"],
                        Deadline::NONE,
                        now(0),
                        &mut unpaid
                    )
                    .is_err()
            );
            assert_eq!(
                baseline,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
        }
        server.step();
        assert_eq!(server.controls.requests.get(), 0);
        assert_eq!(server.app.roots.get(), 0);
        let mut call = start_selected(&client, &["on"]);
        let (local, remote) = server.exchange(&mut call).await;
        assert_eq!(body(&local), json!({"on":true}));
        assert!(remote.result.is_ok());
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn producer_rejects_malformed_and_unsupported_request_arrays_before_dispatch() {
    let addr = address();
    let mut server = Fixture::new(selected_description(addr));
    for bytes in [
        b"null".as_slice(),
        b"{}",
        br#"["absent"]"#,
        br#"["secret"]"#,
        br#"["level","\u006cevel"]"#,
        br#"[7]"#,
        br#"["on",]"#,
        br#"["on"] []"#,
        br#"["\uD800"]"#,
    ] {
        let mut peer = TcpStream::connect(addr).unwrap();
        write!(peer, "POST /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
        peer.write_all(bytes).unwrap();
        let start = Instant::now();
        let settled = loop {
            server.step();
            if let Some(s) = server.server().take_settlement() {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
        };
        assert!(settled.result.is_err());
        assert_eq!(settled.effect, FiniteEffect::NotInvoked);
        assert_eq!(settled.operation, Some(Operation::ReadMultipleProperties));
        assert_eq!(server.app.roots.get(), 0);
        assert_eq!(server.app.reads.get(), 0);
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut response = String::new();
        peer.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 500"));
    }
    server.close();
}

#[test]
fn producer_validates_response_against_original_selection_and_member_schemas() {
    io().block_on(async {
        let addr = address();
        let td = selected_description(addr);
        let mut server = Fixture::new(td.clone());
        server.app.fixed_selection_response.set(true);
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        for bytes in [
            br#"{"on":true,"level":7}"#.as_slice(),
            b"{}",
            br#"{"on":true,"on":false}"#,
            br#"{"on":true,"\u006fn":false}"#,
            br#"{"on":7}"#,
            br#"{"on":{"error":"unavailable"}}"#,
            br#"{"on":true,"secret":"leak"}"#,
            br#"{"level":7}"#,
        ] {
            *server.app.value.borrow_mut() = bytes.to_vec();
            let mut call = start_selected(&client, &["on"]);
            let (local, remote) = server.exchange(&mut call).await;
            assert!(local.result().is_err(), "{bytes:?}");
            assert!(matches!(remote.result, Err(CoreError::Validation(_))));
            assert_eq!(remote.effect, FiniteEffect::Unknown);
            assert_eq!(remote.operation, Some(Operation::ReadMultipleProperties));
        }
        server.app.fails.set(true);
        let mut call = start_selected(&client, &["on", "level"]);
        let (local, remote) = server.exchange(&mut call).await;
        assert!(local.result().is_err());
        assert!(remote.result.is_err());
        assert_eq!(remote.effect, FiniteEffect::Unknown);
        assert_eq!(server.app.reads.get(), 0);
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

// An independent peer must receive one POST and its default TD array. Its
// malformed/partial result cannot rely on Producer-side validation to be caught.
fn peer_response(
    listener: TcpListener,
    status: u16,
    bytes: Vec<u8>,
    truncated: bool,
) -> std::thread::JoinHandle<TcpListener> {
    std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 1024];
        let end = loop {
            let n = socket.read(&mut buf).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buf[..n]);
            if let Some(pos) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let head = std::str::from_utf8(&request[..end]).unwrap();
        assert!(head.starts_with("POST /properties HTTP/1.1\r\n"));
        let length: usize = head
            .lines()
            .find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .unwrap()
            .parse()
            .unwrap();
        while request.len() < end + length {
            let n = socket.read(&mut buf).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buf[..n]);
        }
        assert_eq!(
            serde_json::from_slice::<Value>(&request[end..]).unwrap(),
            json!(["on"])
        );
        write!(socket,"HTTP/1.1 {status} Reply\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()+usize::from(truncated)).unwrap();
        socket.write_all(&bytes).unwrap();
        listener
    })
}

#[test]
fn independent_peer_cannot_return_unrequested_missing_duplicate_invalid_or_partial_members() {
    io().block_on(async {
        for (status, bytes, truncated, valid) in [
            (200, br#"{"on":true}"#.as_slice(), false, true),
            (200, br#"{"on":true,"level":7}"#, false, false),
            (200, b"{}", false, false),
            (200, br#"{"on":true,"on":false}"#, false, false),
            (200, br#"{"on":true,"\u006fn":false}"#, false, false),
            (200, br#"{"on":7}"#, false, false),
            (200, br#"{"on":{"error":"failed"}}"#, false, false),
            (200, br#"{"on":true,"secret":"leak"}"#, false, false),
            (200, br#"{"on":true}"#, true, false),
            (206, br#"{"on":true}"#, false, false),
            (207, br#"{"on":true}"#, false, false),
            (500, br#"{"on":true}"#, false, false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let peer = peer_response(listener, status, bytes.to_vec(), truncated);
            let accounts = bank();
            let owner = consumer_instance();
            let client = client(
                selected_description(addr),
                Arc::new(AtomicBool::new(false)),
                accounts.clone(),
                &owner,
            );
            let mut call = start_selected(&client, &["on"]);
            let start = Instant::now();
            let settled = loop {
                if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                    break s;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            assert_eq!(
                settled.result().is_ok(),
                valid,
                "status={status}, {bytes:?}"
            );
            if status == 200 && !truncated && !valid {
                assert_eq!(
                    settled
                        .rejected_output()
                        .unwrap()
                        .data()
                        .unwrap()
                        .body
                        .as_ref(),
                    bytes
                );
            }
            assert_eq!(settled.consumer(), &owner.reference());
            drop(settled);
            drop(call);
            drop(client);
            clean(&accounts);
            let listener = peer.join().unwrap();
            listener.set_nonblocking(true).unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
        }
    });
}

#[test]
fn selected_lifecycle() {
    verify_collection_lifecycle(Operation::ReadMultipleProperties);
}

#[test]
fn selected_replacement() {
    verify_collection_replacement(Operation::ReadMultipleProperties);
}

#[test]
fn selected_publication() {
    verify_collection_publication(Operation::ReadMultipleProperties);
}

#[test]
fn selected_authorization() {
    verify_collection_authorization(Operation::ReadMultipleProperties);
}

#[test]
fn root_requires_actual_selection_mapping_and_complete_static_member_meaning() {
    let addr = address();
    for fault in [
        "no-root",
        "default-get",
        "get",
        "put",
        "template",
        "schema",
        "readability",
        "extension",
        "context",
    ] {
        let mut td = selected_description(addr);
        match fault {
            "no-root" => td["forms"] = json!([]),
            "default-get" => {
                td["forms"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("htv:methodName");
            }
            "get" => td["forms"][0]["htv:methodName"] = json!("GET"),
            "put" => td["forms"][0]["htv:methodName"] = json!("PUT"),
            "template" => {
                td["forms"][0]["href"] =
                    json!(format!("http://{addr}/properties{{?propertyNames}}"));
                td["uriVariables"] = json!({"propertyNames":{"type":"string"}});
            }
            "schema" => td["properties"]["level"]["multipleOf"] = json!(2),
            "readability" => {
                td["properties"]["level"]["writeOnly"] = json!(true);
                // This is contradictory with readOnly and must not be accepted.
            }
            "extension" => td["forms"][0]["urn:vendor:selection"] = json!("query"),
            "context" => {
                td["properties"]["level"]["@context"] = json!({"limit":"urn:vendor:minimum"});
                td["properties"]["level"]["limit"] = json!(0);
            }
            _ => unreachable!(),
        }
        let accounts = bank();
        if let Ok(prepared) = prepare_consumer(
            &td,
            accounts.clone(),
            vec![registration()],
            ConsumerSelector::default(),
        ) {
            assert!(
                !prepared
                    .support()
                    .iter()
                    .any(|s| s.selected && s.operation == Operation::ReadMultipleProperties),
                "{fault}"
            );
        }
        clean(&accounts);
        let provider = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            BindingId::new(97),
            BindingGeneration::INITIAL,
            1000,
        )
        .unwrap();
        if fault != "no-root" {
            assert!(
                prepare_thing(
                    &td.to_string(),
                    authority(Generation::INITIAL).key(),
                    GatewayDefaultV1::LIMITS,
                    &mut work(),
                    &[&provider],
                    |_, _| true
                )
                .is_err(),
                "{fault}"
            );
        }
        drop(TcpListener::bind(addr).unwrap());
    }
    let td = selected_description(addr);
    let accounts = bank();
    let wrong_binding = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector {
            form: Some(0),
            binding: Some(BindingId::new(7)),
        },
    );
    assert!(wrong_binding.is_err());
    clean(&accounts);
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
            authority(Generation::INITIAL).key(),
            GatewayDefaultV1::LIMITS,
            &mut work(),
            &[&provider],
            |scope, _| scope != ValidatedProducerScope::Thing
        )
        .is_err()
    );
}

struct SelectedService {
    actual: Arc<dyn ConsumerDataService>,
    available: Arc<AtomicBool>,
    encodes: Arc<std::sync::atomic::AtomicUsize>,
    selected_support: bool,
    lose_on_encode: bool,
    lose_on_result: bool,
}
impl ConsumerDataService for SelectedService {
    fn content_type(&self) -> &str {
        self.actual.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut p = self.actual.policy();
        p.retained_bytes += 128;
        p
    }
    fn validate(&self, s: &data_schema::PreparedSchema, p: &Payload) -> CoreResult<()> {
        assert!(
            self.available(),
            "unavailable service entered input validation"
        );
        self.actual.validate(s, p)
    }
    fn available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
    fn supports_property_map(&self, s: &collection::PropertyCollectionSchema) -> bool {
        self.actual.supports_property_map(s)
    }
    fn validate_property_map(
        &self,
        s: &collection::PropertyCollectionSchema,
        p: &Payload,
    ) -> CoreResult<()> {
        self.actual.validate_property_map(s, p)
    }
    fn supports_property_selection(&self, s: &collection::PropertyCollectionSchema) -> bool {
        self.selected_support && self.actual.supports_property_selection(s)
    }
    fn encode_property_selection(
        &self,
        s: &collection::PropertyCollectionSchema,
        selected: collection::PropertySelection,
    ) -> CoreResult<Payload> {
        self.encodes.fetch_add(1, Ordering::Relaxed);
        let result = self.actual.encode_property_selection(s, selected);
        if self.lose_on_encode {
            self.available.store(false, Ordering::Relaxed);
        }
        result
    }
    fn validate_selected_property_map(
        &self,
        s: &collection::PropertyCollectionSchema,
        selected: collection::PropertySelection,
        p: &Payload,
    ) -> CoreResult<()> {
        let result = self.actual.validate_selected_property_map(s, selected, p);
        if self.lose_on_result {
            self.available.store(false, Ordering::Relaxed);
        }
        result
    }
}

#[test]
fn selected_service_is_actual_static_preparation_never_encodes_and_callback_loss_fences_handoff() {
    io().block_on(async {
        for fault in ["unsupported", "encode", "result"] {
            let addr = address();
            let td = selected_description(addr);
            let mut server = Fixture::new(td.clone());
            let accounts = bank();
            let owner = consumer_instance();
            let available = Arc::new(AtomicBool::new(true));
            let encodes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let mut reg = registration();
            reg.services.data = Arc::new(SelectedService {
                actual: reg.services.data.clone(),
                available: available.clone(),
                encodes: encodes.clone(),
                selected_support: fault != "unsupported",
                lose_on_encode: fault == "encode",
                lose_on_result: fault == "result",
            });
            let prepared = prepare_consumer(
                &td,
                accounts.clone(),
                vec![reg],
                ConsumerSelector::default(),
            )
            .unwrap();
            assert_eq!(encodes.load(Ordering::Relaxed), 0);
            if fault == "unsupported" {
                assert!(
                    !prepared
                        .support()
                        .iter()
                        .any(|s| s.selected && s.operation == Operation::ReadMultipleProperties)
                );
                drop(prepared);
                clean(&accounts);
                server.close();
                continue;
            }
            let (d, f) = prepared.into_execution();
            let client =
                ConsumerAssociation::try_new(owner.reference(), d, f, accounts.clone())
                    .map(|association| {
                        (
                            association,
                            ConsumerCalls::try_new(accounts.clone(), 2).unwrap(),
                        )
                    })
                    .unwrap_or_else(|_| panic!());
            let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
            let result = client.0.read_multiple_properties(
                &client.1,
                &["on"],
                Deadline::NONE,
                now(0),
                &mut work(),
            );
            assert_eq!(encodes.load(Ordering::Relaxed), 1);
            if fault == "encode" {
                assert!(result.is_err());
                assert_eq!(
                    baseline,
                    ResourceKind::ALL.map(|k| accounts.account(k).used())
                );
                assert_eq!(server.controls.requests.get(), 0);
            } else {
                let mut call = result.unwrap();
                let (local, remote) = server.exchange(&mut call).await;
                assert!(local.result().is_err());
                assert!(matches!(
                    local.native_outcome(),
                    ConsumerNativeOutcome::Returned(Ok(_))
                ));
                assert!(remote.result.is_ok());
                assert_eq!(local.consumer(), &owner.reference());
            }
            assert!(!available.load(Ordering::Relaxed));
            assert!(
                client
                    .0
                    .read_multiple_properties(
                        &client.1,
                        &["level"],
                        Deadline::NONE,
                        now(0),
                        &mut work()
                    )
                    .is_err()
            );
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn pending_output_keeps_original_selection_and_funding_across_failed_sibling_admission() {
    io().block_on(async {
        let addr = address();
        let td = selected_description(addr);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let mut call = start_selected(&client, &["on"]);
        let start = Instant::now();
        let held = loop {
            server.step();
            let mut unpaid = work().with_remaining(WorkClass::CodecInputBytes, 0);
            assert!(call.poll(&mut cx(), now(0), &mut unpaid).is_pending());
            if let Some(remote) = server.server().take_settlement() {
                assert!(remote.result.is_ok());
                break ResourceKind::ALL.map(|k| accounts.account(k).used());
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        // The original raw call keeps native occupancy through unread output.
        assert!(
            client
                .0
                .read_multiple_properties(
                    &client.1,
                    &["level"],
                    Deadline::NONE,
                    now(0),
                    &mut work()
                )
                .is_err()
        );
        assert_eq!(held, ResourceKind::ALL.map(|k| accounts.account(k).used()));
        let local = loop {
            if let Poll::Ready(s) = call.poll(&mut cx(), now(0), &mut work()) {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert_eq!(body(&local), json!({"on":true}));
        assert_eq!(server.controls.requests.get(), 1);
        assert_eq!(
            server.app.selections.borrow().as_slice(),
            &[vec!["on".to_owned()]]
        );
        drop(local);
        drop(call);
        let mut call = start_selected(&client, &["level"]);
        let (local, remote) = server.exchange(&mut call).await;
        assert_eq!(body(&local), json!({"level":7}));
        assert!(remote.result.is_ok());
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn selected_schema_cleanup_is_prepaid_for_temporary_retained_and_rollback_owners() {
    verify_collection_cleanup(Operation::ReadMultipleProperties);
}

#[test]
fn selected_validation_bound_change_fences_callback_and_retains_original_input() {
    verify_collection_validator_bound(Operation::ReadMultipleProperties);
}

#[test]
fn escaping_expansion_refuses_with_funded_cleanup_before_native_start() {
    io().block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let mut td = selected_description(addr);
        td["properties"] = json!({});
        let names: Vec<_> = (0..3).map(|i| format!("{i}{}", "\u{0001}".repeat(240))).collect();
        for (i,name) in names.iter().enumerate() {
            td["properties"][name] = json!({"type":"boolean", "readOnly":true, "forms":[{"href":format!("http://{addr}/p{i}")}]});
        }
        let accounts = bank(); let owner = consumer_instance();
        let client = client(td, Arc::new(AtomicBool::new(false)), accounts.clone(), &owner);
        let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
        assert!(client.0.read_multiple_properties(&client.1, &names.iter().map(String::as_str).collect::<Vec<_>>(), Deadline::NONE, now(0), &mut work()).is_err());
        assert_eq!(baseline, ResourceKind::ALL.map(|k| accounts.account(k).used()));
        assert!(client.1.is_idle());
        listener.set_nonblocking(true).unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        drop(client); clean(&accounts);
    });
}

#[test]
fn authorization_service_loss_fences_each_admission_boundary() {
    struct AccessChangesData(Arc<AtomicBool>, std::sync::atomic::AtomicUsize, usize);
    impl ConsumerAccessService for AccessChangesData {
        fn authorize(
            &self,
            _: BindingRegistrationIdentity,
            _: &str,
            _: Operation,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn authorize_instance(
            &self,
            _: &ConsumerInstanceRef,
            _: BindingRegistrationIdentity,
            _: &str,
            _: Operation,
        ) -> CoreResult<()> {
            // First permission precedes encoding; the second follows encoding
            // and precedes generic input validation; the third precedes native
            // admission. Each callback must leave the dependencies current.
            if self.1.fetch_add(1, Ordering::Relaxed) + 1 == self.2 {
                self.0.store(false, Ordering::Relaxed);
            }
            Ok(())
        }
        fn retained_bytes(&self) -> u64 {
            64
        }
        fn work(&self) -> u64 {
            1
        }
    }
    // A non-effecting permission callback is allowed to change policy through
    // interior state. No codec callback may rely on its earlier availability.
    io().block_on(async {
        for permission in [1, 2, 3] {
            let addr = address();
            let td = selected_description(addr);
            let mut server = Fixture::new(td.clone());
            let accounts = bank();
            let owner = consumer_instance();
            let available = Arc::new(AtomicBool::new(true));
            let encodes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let mut reg = registration();
            reg.services.data = Arc::new(SelectedService {
                actual: reg.services.data.clone(),
                available: available.clone(),
                encodes: encodes.clone(),
                selected_support: true,
                lose_on_encode: false,
                lose_on_result: false,
            });
            reg.services.access = Arc::new(AccessChangesData(
                available.clone(),
                std::sync::atomic::AtomicUsize::new(0),
                permission,
            ));
            let prepared = prepare_consumer(
                &td,
                accounts.clone(),
                vec![reg],
                ConsumerSelector::default(),
            )
            .unwrap();
            let (d, f) = prepared.into_execution();
            let client =
                ConsumerAssociation::try_new(owner.reference(), d, f, accounts.clone())
                    .map(|association| {
                        (
                            association,
                            ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
                        )
                    })
                    .unwrap_or_else(|_| panic!());
            let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
            assert!(
                client
                    .0
                    .read_multiple_properties(
                        &client.1,
                        &["on"],
                        Deadline::NONE,
                        now(0),
                        &mut work()
                    )
                    .is_err()
            );
            assert_eq!(
                encodes.load(Ordering::Relaxed),
                usize::from(permission >= 2)
            );
            assert_eq!(
                baseline,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
            server.step();
            assert_eq!(server.controls.requests.get(), 0);
            assert_eq!(server.app.roots.get(), 0);
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}

// These wrappers change only captured service metadata. All encoding, decoding
// and native I/O still use the actual HTTP participant and JSON service.
struct ReviewData {
    actual: Arc<dyn ConsumerDataService>,
    fault: &'static str,
    changed: Arc<AtomicBool>,
    validated: Arc<std::sync::atomic::AtomicUsize>,
}
impl ConsumerDataService for ReviewData {
    fn content_type(&self) -> &str {
        self.actual.content_type()
    }
    fn policy(&self) -> ConsumerDataPolicy {
        let mut policy = self.actual.policy();
        policy.retained_bytes += 256;
        if self.fault == "policy" && self.changed.load(Ordering::Relaxed) {
            policy.validation_work += 1;
        }
        policy
    }
    fn available(&self) -> bool {
        self.actual.available()
            && !(self.fault == "availability" && self.changed.load(Ordering::Relaxed))
    }
    fn supports_schema(&self, schema: &data_schema::PreparedSchema) -> bool {
        self.actual.supports_schema(schema)
            && !(self.fault == "array"
                && self.changed.load(Ordering::Relaxed)
                && schema == &data_schema::PreparedSchema::Type(data_schema::ValueType::Array))
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
        assert!(
            self.available(),
            "unavailable service entered input validation"
        );
        if !self.supports_schema(schema) {
            return Err(CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            )));
        }
        self.actual.validate(schema, payload)
    }
    fn supports_property_map(&self, schema: &collection::PropertyCollectionSchema) -> bool {
        self.actual.supports_property_map(schema)
    }
    fn validate_property_map(
        &self,
        schema: &collection::PropertyCollectionSchema,
        payload: &Payload,
    ) -> CoreResult<()> {
        self.actual.validate_property_map(schema, payload)
    }
    fn supports_property_selection(&self, schema: &collection::PropertyCollectionSchema) -> bool {
        self.actual.supports_property_selection(schema)
    }
    fn encode_property_selection(
        &self,
        schema: &collection::PropertyCollectionSchema,
        selected: collection::PropertySelection,
    ) -> CoreResult<Payload> {
        self.actual.encode_property_selection(schema, selected)
    }
    fn validate_selected_property_map(
        &self,
        schema: &collection::PropertyCollectionSchema,
        selected: collection::PropertySelection,
        payload: &Payload,
    ) -> CoreResult<()> {
        self.actual
            .validate_selected_property_map(schema, selected, payload)?;
        self.validated.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
struct ReviewAccess {
    fault: &'static str,
    armed: Arc<AtomicBool>,
    changed: Arc<AtomicBool>,
    validated: Arc<std::sync::atomic::AtomicUsize>,
}
impl ConsumerAccessService for ReviewAccess {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        if self.armed.load(Ordering::Relaxed) && self.validated.load(Ordering::Relaxed) != 0 {
            self.changed.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        128
    }
    fn work(&self) -> u64 {
        if self.fault == "access" && self.changed.load(Ordering::Relaxed) {
            2
        } else {
            1
        }
    }
}

#[test]
fn review_authorization_changes_fence_immediate_and_unread_validated_result_handoff() {
    io().block_on(async {
        for unread in [false, true] {
            for fault in ["availability", "policy", "access", "array", "none"] {
                let addr = address();
                let td = selected_description(addr);
                let mut server = Fixture::new(td.clone());
                let accounts = bank();
                let owner = consumer_instance();
                let changed = Arc::new(AtomicBool::new(false));
                let armed = Arc::new(AtomicBool::new(!unread));
                let validated = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let mut reg = registration();
                reg.services.data = Arc::new(ReviewData {
                    actual: reg.services.data.clone(),
                    fault,
                    changed: changed.clone(),
                    validated: validated.clone(),
                });
                reg.services.access = Arc::new(ReviewAccess {
                    fault,
                    armed: armed.clone(),
                    changed: changed.clone(),
                    validated: validated.clone(),
                });
                let prepared = prepare_consumer(
                    &td,
                    accounts.clone(),
                    vec![reg],
                    ConsumerSelector::default(),
                )
                .unwrap();
                let (d, f) = prepared.into_execution();
                let client = ConsumerAssociation::try_new(
                    owner.reference(),
                    d,
                    f,
                    accounts.clone(),
                )
                .map(|association| {
                    (
                        association,
                        ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
                    )
                })
                .unwrap_or_else(|_| panic!());
                let mut call = start_selected(&client, &["on"]);
                let start = Instant::now();
                let (local, remote) = if unread {
                    // Complete output validation but withhold credit for the
                    // final permission callback, retaining an unread result.
                    while validated.load(Ordering::Relaxed) == 0 {
                        server.step();
                        client.1.step(
                            &mut cx(),
                            now(0),
                            &mut work().with_remaining(WorkClass::ProviderProbes, 1),
                        );
                        assert!(start.elapsed() < Duration::from_secs(5));
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    assert!(!changed.load(Ordering::Relaxed));
                    let retained = ResourceKind::ALL.map(|k| accounts.account(k).used());
                    armed.store(true, Ordering::Relaxed);
                    assert!(
                        call.poll(&mut cx(), now(0), &mut WorkBudget::new())
                            .is_pending()
                    );
                    assert!(
                        !changed.load(Ordering::Relaxed),
                        "no unpaid permission callback"
                    );
                    assert_eq!(
                        retained,
                        ResourceKind::ALL.map(|k| accounts.account(k).used())
                    );
                    let Poll::Ready(local) = call.poll(
                        &mut cx(),
                        now(0),
                        &mut WorkBudget::new().with_remaining(WorkClass::ProviderProbes, 1),
                    ) else {
                        panic!("validated result requires only the paid current check");
                    };
                    assert_eq!(
                        retained,
                        ResourceKind::ALL.map(|k| accounts.account(k).used())
                    );
                    (local, server.server().take_settlement().unwrap())
                } else {
                    server.exchange(&mut call).await
                };
                assert!(changed.load(Ordering::Relaxed));
                assert_eq!(validated.load(Ordering::Relaxed), 1);
                assert_eq!(
                    local.result().is_ok(),
                    fault == "none",
                    "unread={unread}, fault={fault}"
                );
                if fault != "none" {
                    assert!(matches!(
                        local.first_cause(),
                        Some(CoreError::Validation(_))
                    ));
                }
                let ConsumerNativeOutcome::Returned(Ok(original)) = local.native_outcome() else {
                    panic!("retain the original validated native output");
                };
                assert_eq!(
                    serde_json::from_slice::<Value>(&original.data().unwrap().body).unwrap(),
                    json!({"on":true})
                );
                assert_eq!(local.consumer(), &owner.reference());
                assert_eq!(
                    local.affordance(),
                    clinkz_wot_core::thing::AffordanceSite::Thing
                );
                assert_eq!(local.operation(), Operation::ReadMultipleProperties);
                assert_eq!(local.effect(), ConsumerEffect::Unknown);
                assert!(local.rejected_output().is_none());
                assert!(remote.result.is_ok());
                assert_eq!(server.controls.requests.get(), 1);
                assert_eq!(server.app.reads.get(), 0);
                drop(local);
                drop(call);
                drop(client);
                clean(&accounts);
                server.close();
            }
        }
    });
}

#[test]
fn review_preparation_qualifies_generic_array_input_independently_of_selection_facets() {
    io().block_on(async {
        for array_supported in [false, true] {
            let addr = address();
            let mut td = selected_description(addr);
            td["forms"]
                .as_array_mut()
                .unwrap()
                .push(description(addr)["forms"][0].clone());
            let mut server = Fixture::new(td.clone());
            let accounts = bank();
            let owner = consumer_instance();
            let mut reg = registration();
            reg.services.data = Arc::new(ReviewData {
                actual: reg.services.data.clone(),
                fault: "array",
                changed: Arc::new(AtomicBool::new(!array_supported)),
                validated: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            });
            let prepared = prepare_consumer(
                &td,
                accounts.clone(),
                vec![reg],
                ConsumerSelector::default(),
            )
            .unwrap();
            let selected = |op| {
                prepared
                    .support()
                    .iter()
                    .any(|s| s.selected && s.operation == op)
            };
            assert_eq!(selected(Operation::ReadMultipleProperties), array_supported);
            assert!(
                selected(Operation::ReadAllProperties),
                "all-read has no Array input dependency"
            );
            let (d, f) = prepared.into_execution();
            let client =
                ConsumerAssociation::try_new(owner.reference(), d, f, accounts.clone())
                    .map(|association| {
                        (
                            association,
                            ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
                        )
                    })
                    .unwrap_or_else(|_| panic!());
            let result = client.0.read_multiple_properties(
                &client.1,
                &["on"],
                Deadline::NONE,
                now(0),
                &mut work(),
            );
            if array_supported {
                let mut call = result.unwrap();
                let (local, remote) = server.exchange(&mut call).await;
                assert_eq!(body(&local), json!({"on":true}));
                assert!(remote.result.is_ok());
            } else {
                assert!(matches!(result, Err(CoreError::UnsupportedOperation(_))));
                server.step();
                assert_eq!(server.controls.requests.get(), 0);
            }
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}
