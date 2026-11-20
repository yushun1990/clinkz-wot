use super::*;
use clinkz_wot_td::thing::Thing;

// Three independent registrations use one effective deployment configuration.
// A always qualifies, B qualifies in odd revisions, C in positive even ones.
// B can deliver a reload between its negative answer and C's probe.
struct ReloadableCompiler {
    compiler: Compiler,
    ordinal: usize,
    revision: Rc<std::cell::Cell<Option<u64>>>,
    change_at_b: Rc<std::cell::Cell<bool>>,
}
impl ConsumerActionPreparation for ReloadableCompiler {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.compiler.identity()
    }
    fn bounds(&self) -> ConsumerPreparationBounds {
        self.compiler.bounds()
    }
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        let revision = self
            .revision
            .get()
            .expect("compiler entered during a reload");
        let eligible = match self.ordinal {
            0 => true,
            1 => revision % 2 == 1,
            _ => revision > 0 && revision % 2 == 0,
        };
        if self.ordinal == 1 && self.change_at_b.replace(false) {
            self.revision.set(Some(revision.checked_add(1).unwrap()));
        }
        if eligible {
            self.compiler.compile_action(input)
        } else {
            Ok(None)
        }
    }
}

#[test]
fn selection_fences_a_reload_and_releases_competitors_after_the_decision() {
    use std::cell::Cell;
    let (address, server) = peer(vec![
        (b"INVOKE 12", b"OUTPUT 24"),
        (b"INVOKE 12", b"OUTPUT 24"),
    ]);
    let competitors = Arc::new(NativeProbe::default());
    let revision = Rc::new(Cell::new(Some(0u64)));
    let change_at_b = Rc::new(Cell::new(false));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let native = Arc::new(NativeProbe::default());
    *native.bank.lock().unwrap() = Some(bank.clone());
    let data = services(Arc::new(Probe::default())).data;
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(data, &bank).ok().unwrap(),
        access: ConsumerServiceOwner::retain_access(Arc::new(Access), &bank)
            .ok()
            .unwrap(),
    };
    let registry = ConsumerActionRegistry::try_new_revisioned(
        (0..3)
            .map(|i| ConsumerActionRegistration {
                preparation: Box::new(ReloadableCompiler {
                    compiler: Compiler {
                        id: BindingRegistrationIdentity::new(
                            BindingId::new(70 + i),
                            BindingGeneration::INITIAL,
                            configuration().configuration(),
                            configuration().native_compatibility().unwrap(),
                            i,
                        ),
                        invoke: true,
                        media: "application/json",
                        peer: address,
                        ascii_continuation: false,
                        probe: if i == 0 {
                            native.clone()
                        } else {
                            competitors.clone()
                        },
                    },
                    ordinal: i as usize,
                    revision: revision.clone(),
                    change_at_b: change_at_b.clone(),
                }),
                services: services.clone(),
            })
            .collect(),
        {
            let revision = revision.clone();
            move || revision.get()
        },
    )
    .unwrap();
    let td = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"changing candidates",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"input":{"type":"integer"},"output":{"type":"integer"},
            "forms":[{"op":"invokeaction","href":format!("linkedudp://{address}/run")}]}}
    }))
    .unwrap();
    let select = |binding, budget: &mut WorkBudget| {
        registry.prepare_action(
            &td,
            "run",
            ConsumerActionSelectors {
                invoke: ConsumerSelector {
                    form: None,
                    binding,
                },
                ..Default::default()
            },
            GatewayDefaultV1::LIMITS,
            bank.clone(),
            budget,
        )
    };
    let original = Rc::new(select(None, &mut work()).unwrap());
    let associate = |definition, owner: &ConsumerInstance| {
        ConsumerActionAssociation::try_new(
            owner.reference(),
            definition,
            ConsumerActionAccess {
                invoke: services.access.clone(),
                query: None,
                cancel: None,
            },
            bank.clone(),
            &mut work(),
        )
        .ok()
        .unwrap()
    };
    let owner = instance();
    let live = associate(original.clone(), &owner);
    let retired_owner = instance();
    let retired = associate(original, &retired_owner);
    retired_owner.revoke();
    revision.set(None);
    assert!(matches!(
        select(None, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    for generation in [1, 2] {
        revision.set(Some(generation));
        assert!(matches!(
            select(None, &mut work()),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::AmbiguousBindingOwner,
                ..
            })
        ));
    }
    change_at_b.set(true);
    assert!(matches!(
        select(None, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    let mut short = work().with_remaining(WorkClass::ProviderProbes, 3);
    assert!(matches!(
        select(None, &mut short),
        Err(CoreError::Backpressure(_))
    ));
    let pinned_owner = instance();
    let pinned = associate(
        Rc::new(select(Some(BindingId::new(70)), &mut work()).unwrap()),
        &pinned_owner,
    );
    drop(registry);
    let calls = ConsumerActions::try_new(1, bank.clone()).unwrap();
    let invoke = |association| {
        calls.invoke_action(
            association,
            Some(Payload::new(b"12".as_slice(), "application/json")),
            Deadline::NONE,
            now(),
            &mut work(),
        )
    };
    assert!(invoke(&retired).is_err());
    assert!(native.ports.lock().unwrap().is_empty());
    for association in [&live, &pinned] {
        let h = invoke(association).ok().unwrap();
        drive(&h, ConsumerActionState::Completed);
        let result = h.take_settlement(now(), &mut work()).unwrap();
        assert_eq!(result.result().unwrap().data().unwrap().body, b"24");
    }
    server.join().unwrap();
    assert!(competitors.ports.lock().unwrap().is_empty());
    drop((calls, live, retired, pinned, services));
    clean(&bank);
}

fn http_peer() -> (SocketAddr, std::thread::JoinHandle<()>) {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            for (request, code, body) in [
                (
                    "POST /run HTTP/1.1",
                    202,
                    r#"{"status":"accepted","id":"0123456789abcdef-7"}"#,
                ),
                (
                    "GET /status?id=0123456789abcdef-7 HTTP/1.1",
                    200,
                    r#"{"status":"completed","output":42}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert_eq!(line.trim_end(), request);
                loop {
                    line.clear();
                    assert!(reader.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                if code == 202 {
                    let mut input = [0; 2];
                    reader.read_exact(&mut input).unwrap();
                    assert_eq!(&input, b"12");
                }
                write!(stream, "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        }
    });
    (address, server)
}

async fn drive_with_reactor(handle: &ConsumerActionHandle, expected: ConsumerActionState) {
    let until = Instant::now() + Duration::from_secs(3);
    while handle.progress(&mut cx(), now(), &mut work()).unwrap() != expected {
        assert!(
            Instant::now() < until,
            "expected {expected:?}, got {:?}",
            handle.state()
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn http_and_independent_datagram_bindings_choose_the_first_complete_family() {
    let (http, http_server) = http_peer();
    let (udp, udp_server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let reactor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    reactor.block_on(exercise_mixed_selection(http, udp));
    http_server.join().unwrap();
    udp_server.join().unwrap();
}

async fn exercise_mixed_selection(http: SocketAddr, udp: SocketAddr) {
    use clinkz_wot_binding_http::{HttpActionConsumer, HttpConsumerConfig};
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::PlanCompileWorkUnitsPerStepMax, Some(4096)));
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let native = Arc::new(NativeProbe::default());
    *native.bank.lock().unwrap() = Some(bank.clone());
    let config = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let retain = |data, access| RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(data, &bank).ok().unwrap(),
        access: ConsumerServiceOwner::retain_access(access, &bank)
            .ok()
            .unwrap(),
    };
    let http_services = config.consumer_services();
    let http_services = retain(http_services.data, http_services.access);
    let datagram_services = retain(services(Arc::new(Probe::default())).data, Arc::new(Access));
    for reverse in [false, true] {
        let mut registrations = vec![ConsumerActionRegistration {
            preparation: Box::new(HttpActionConsumer::new(
                config,
                BindingId::new(1),
                BindingGeneration::INITIAL,
                0,
            )),
            services: http_services.clone(),
        }];
        // Both Invoke providers qualify for the same datagram Form. Only the
        // numeric-token provider can pair with this control implementation.
        for (id, invoke, ascii_continuation) in
            [(2, true, false), (3, false, false), (4, true, true)]
        {
            registrations.push(ConsumerActionRegistration {
                preparation: Box::new(Compiler {
                    id: BindingRegistrationIdentity::new(
                        BindingId::new(id),
                        BindingGeneration::INITIAL,
                        configuration().configuration(),
                        configuration().native_compatibility().unwrap(),
                        id,
                    ),
                    invoke,
                    media: "application/json",
                    ascii_continuation,
                    peer: udp,
                    probe: native.clone(),
                }),
                services: datagram_services.clone(),
            });
        }
        if reverse {
            registrations.reverse();
        }
        let registry = ConsumerActionRegistry::try_new_fixed(registrations).unwrap();
        for http_control in [true, false] {
            // Controls can precede Invoke. Removing only the HTTP control
            // leaves its Invoke individually supported but unusable. The UDP Invoke
            // Form has already been visited as a Query alternative for that HTTP Invoke.
            let mut forms = vec![
                serde_json::json!({"op":"queryaction","href":format!("linkedudp://{udp}/query")}),
                serde_json::json!({"op":"invokeaction","href":format!("http://{http}/run"),"htv:methodName":"POST"}),
                serde_json::json!({"op":["invokeaction","queryaction"],"href":format!("linkedudp://{udp}/run")}),
            ];
            if http_control {
                forms.push(serde_json::json!({"op":"queryaction","href":format!("http://{http}/status{{?id}}"),"htv:methodName":"GET"}));
            }
            let td = serde_json::from_value(serde_json::json!({
                "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"mixed Action",
                "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
                "actions":{"run":{"input":{"type":"integer"},"output":{"type":"integer"},
                    "uriVariables":{"id":{"type":"string"}},"forms":forms}}
            }))
            .unwrap();
            let definition = registry
                .prepare_action(
                    &td,
                    "run",
                    Default::default(),
                    &limits,
                    bank.clone(),
                    &mut work(),
                )
                .unwrap();
            assert_eq!(
                definition.input().invoke.identity.binding_id(),
                BindingId::new(if http_control { 1 } else { 2 })
            );
            let access_for = |op: &ConsumerActionOperation| {
                if op.identity.binding_id() == BindingId::new(1) {
                    http_services.access.clone()
                } else {
                    datagram_services.access.clone()
                }
            };
            let access = ConsumerActionAccess {
                invoke: access_for(&definition.input().invoke),
                query: definition.input().query.as_ref().map(access_for),
                cancel: None,
            };
            let owner = instance();
            let association = ConsumerActionAssociation::try_new(
                owner.reference(),
                Rc::new(definition),
                access,
                bank.clone(),
                &mut work(),
            )
            .ok()
            .unwrap();
            let calls = ConsumerActions::try_new(1, bank.clone()).unwrap();
            let handle = calls
                .invoke_action(
                    &association,
                    Some(Payload::new(b"12".as_slice(), "application/json")),
                    Deadline::NONE,
                    now(),
                    &mut work(),
                )
                .ok()
                .unwrap();
            drive_with_reactor(&handle, ConsumerActionState::Pending).await;
            handle.query(now(), &mut work()).unwrap();
            drive_with_reactor(&handle, ConsumerActionState::Completed).await;
            let settlement = handle.take_settlement(now(), &mut work()).unwrap();
            assert_eq!(
                settlement.result().unwrap().data().unwrap().body,
                if http_control { b"42" } else { b"24" }
            );
        }
    }
    drop((http_services, datagram_services));
    clean(&bank);
}

fn two_form_action(peer_address: SocketAddr, operations: [&[&str]; 2]) -> Thing {
    serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"two Action Forms",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"input":{"type":"integer"},"output":{"type":"integer"},"forms":[
            {"href":format!("linkedudp://{peer_address}/run"), "op":operations[0]},
            {"href":format!("linkedudp://{peer_address}/control"), "op":operations[1]}
        ]}}
    }))
    .unwrap()
}

fn prepare_two_forms(
    registry: &ConsumerActionRegistry,
    td: &Thing,
    bank: &Arc<HostResourceAccounts>,
    selectors: ConsumerActionSelectors,
    limits: &ResourceLimits,
    credit: u64,
) -> CoreResult<ConsumerActionDefinition> {
    registry.prepare_action(
        td,
        "run",
        selectors,
        limits,
        bank.clone(),
        &mut work().with_remaining(WorkClass::BindingPolls, credit),
    )
}

#[test]
fn query_on_a_second_invoke_form_uses_only_the_selected_operations_credit() {
    let (peer_address, server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let services_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(NativeProbe::default());
    let registry = datagram_registry(
        peer_address,
        &services_bank,
        &probe,
        &Arc::new(Probe::default()),
    );
    *probe.bank.lock().unwrap() = Some(bank.clone());
    let td = two_form_action(
        peer_address,
        [&["invokeaction"], &["invokeaction", "queryaction"]],
    );
    let prepare = |form, credit| {
        prepare_two_forms(
            &registry,
            &td,
            &bank,
            ConsumerActionSelectors {
                invoke: ConsumerSelector {
                    form,
                    ..Default::default()
                },
                ..Default::default()
            },
            GatewayDefaultV1::LIMITS,
            credit,
        )
    };
    // Two operations, two registered compilers, 32 units per callback.
    let credit = 2 * 2 * 32;
    let pinned = prepare(Some(0), credit).expect("pinned Invoke 0 and Query 1 fit");
    assert_eq!(
        (
            pinned.input().invoke.form,
            pinned.input().query.as_ref().unwrap().form
        ),
        (0, 1)
    );
    drop(pinned);
    clean(&bank);
    assert!(matches!(
        prepare(None, credit - 1),
        Err(CoreError::Backpressure(_))
    ));
    clean(&bank);
    assert!(probe.ports.lock().unwrap().is_empty());

    let chosen =
        prepare(None, credit).expect("automatic Invoke 0 and Query 1 fit the same allowance");
    assert_eq!(
        (
            chosen.input().invoke.form,
            chosen.input().query.as_ref().unwrap().form
        ),
        (0, 1)
    );
    drop(registry);
    let owner = instance();
    let (association, calls) = bind_datagram_action(chosen, &owner, bank.clone());
    let handle = calls
        .invoke_action(
            &association,
            Some(Payload::new(b"12".as_slice(), "application/json")),
            Deadline::NONE,
            now(),
            &mut work(),
        )
        .ok()
        .unwrap();
    drive(&handle, ConsumerActionState::Pending);
    handle.query(now(), &mut work()).unwrap();
    drive(&handle, ConsumerActionState::Completed);
    let settlement = handle.take_settlement(now(), &mut work()).unwrap();
    assert_eq!(settlement.result().unwrap().data().unwrap().body, b"24");
    server.join().unwrap();
    drop((settlement, handle, calls, association));
    clean(&bank);
    clean(&services_bank);
}

#[test]
fn compiler_failure_propagates_only_when_its_form_and_operation_are_requested() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let services_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(NativeProbe {
        fail_compilation: Some((1, Operation::CancelAction)),
        ..Default::default()
    });
    let address = "127.0.0.1:9".parse().unwrap();
    let registry = datagram_registry(address, &services_bank, &probe, &Arc::new(Probe::default()));
    let td = two_form_action(
        address,
        [
            &["invokeaction", "cancelaction"],
            &["queryaction", "cancelaction"],
        ],
    );
    let pin_cancel = ConsumerActionSelectors {
        cancel: ConsumerSelector {
            form: Some(1),
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(matches!(
        prepare_two_forms(
            &registry,
            &td,
            &bank,
            pin_cancel,
            GatewayDefaultV1::LIMITS,
            1_000_000
        ),
        Err(CoreError::Binding(_))
    ));
    clean(&bank);
    let selected = prepare_two_forms(
        &registry,
        &td,
        &bank,
        Default::default(),
        GatewayDefaultV1::LIMITS,
        1_000_000,
    )
    .expect("Form 1 Query remains usable when its Cancel compiler fails");
    assert_eq!(selected.input().invoke.form, 0);
    assert_eq!(selected.input().query.as_ref().unwrap().form, 1);
    assert_eq!(selected.input().cancel.as_ref().unwrap().form, 0);
    assert!(probe.ports.lock().unwrap().is_empty());
    drop((selected, registry));
    clean(&bank);
    clean(&services_bank);
}

#[test]
fn an_unused_form_operation_reserves_no_candidate_storage() {
    let address = "127.0.0.1:9".parse().unwrap();
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let registry = datagram_registry(
        address,
        &services,
        &Arc::new(NativeProbe::default()),
        &Arc::new(Probe::default()),
    );
    let td = two_form_action(
        address,
        [&["invokeaction"], &["invokeaction", "queryaction"]],
    );
    let kind = ResourceKind::AdmissionTemporaryBytesPerOperationMax;
    let fits = |invoke_form, bytes| {
        let mut limits = GatewayDefaultV1::LIMITS.clone();
        assert!(limits.set(kind, Some(bytes)));
        let selectors = ConsumerActionSelectors {
            invoke: ConsumerSelector {
                form: invoke_form,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = prepare_two_forms(&registry, &td, &bank, selectors, &limits, 1_000_000);
        let fits = match result {
            Ok(definition) => {
                drop(definition);
                true
            }
            Err(CoreError::LimitExceeded { .. } | CoreError::Backpressure(_)) => false,
            Err(e) => panic!("unexpected preparation error {e:?}"),
        };
        clean(&bank);
        fits
    };
    // Discover the threshold through the public API so it follows the target's layouts.
    let (mut low, mut high) = (0, GatewayDefaultV1::LIMITS.get(kind).unwrap());
    assert!(fits(Some(0), high));
    while low < high {
        let mid = low + (high - low) / 2;
        if fits(Some(0), mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(
        fits(None, low),
        "automatic selection must fit the pinned {low}-byte allowance"
    );
    assert!(!fits(None, low - 1));
    drop(registry);
    clean(&services);
}
