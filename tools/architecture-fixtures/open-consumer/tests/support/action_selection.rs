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
                            configuration().native_compatibility(),
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
        registrations.extend(datagram_registrations(
            udp,
            &native,
            &datagram_services,
            &TOKEN_PROVIDERS[..3],
        ));
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

// (Binding ID, Invoke compiler, ASCII token). The two control compilers
// accept their respective native token types on a shared control endpoint.
const TOKEN_PROVIDERS: [(u32, bool, bool); 4] = [
    (2, true, false),
    (3, false, false),
    (4, true, true),
    (5, false, true),
];

fn datagram_registrations(
    peer: SocketAddr,
    native: &Arc<NativeProbe>,
    services: &RetainedConsumerServices,
    providers: &[(u32, bool, bool)],
) -> Vec<ConsumerActionRegistration> {
    providers
        .iter()
        .map(
            |&(id, invoke, ascii_continuation)| ConsumerActionRegistration {
                preparation: Box::new(Compiler {
                    id: BindingRegistrationIdentity::new(
                        BindingId::new(id),
                        BindingGeneration::INITIAL,
                        configuration().configuration(),
                        configuration().native_compatibility(),
                        id,
                    ),
                    invoke,
                    ascii_continuation,
                    peer,
                    media: "application/json",
                    probe: native.clone(),
                }),
                services: services.clone(),
            },
        )
        .collect()
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

fn prepare_with_credit(
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
        prepare_with_credit(
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
    execute_datagram_action(chosen, &bank);
    server.join().unwrap();
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
        prepare_with_credit(
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
    let selected = prepare_with_credit(
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
        let result = prepare_with_credit(&registry, &td, &bank, selectors, &limits, 1_000_000);
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

fn execute_datagram_action(chosen: ConsumerActionDefinition, bank: &Arc<HostResourceAccounts>) {
    let has_cancel = chosen.input().cancel.is_some();
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
    if has_cancel {
        handle.request_cancel(now(), &mut work()).unwrap();
        drive(&handle, ConsumerActionState::Pending);
    }
    handle.query(now(), &mut work()).unwrap();
    drive(&handle, ConsumerActionState::Completed);
    let settlement = handle.take_settlement(now(), &mut work()).unwrap();
    assert_eq!(settlement.result().unwrap().data().unwrap().body, b"24");
    drop((settlement, handle, calls, association));
    clean(bank);
}

fn control_forms(peer: SocketAddr, forms: &[(&str, &str)]) -> Thing {
    let forms: Vec<_> = forms
        .iter()
        .map(|(op, path)| {
            serde_json::json!({
                "href": format!("linkedudp://{peer}/{path}"), "op":op,
            })
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"competing datagram mappings",
        "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"input":{"type":"integer"},"output":{"type":"integer"},"forms":forms}}
    }))
    .unwrap()
}

fn selection_services(service_bank: &Arc<HostResourceAccounts>) -> RetainedConsumerServices {
    RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(
            services(Arc::new(Probe::default())).data,
            service_bank,
        )
        .ok()
        .unwrap(),
        access: ConsumerServiceOwner::retain_access(Arc::new(Access), service_bank)
            .ok()
            .unwrap(),
    }
}

fn competing_registry(
    peer: SocketAddr,
    native: &Arc<NativeProbe>,
    service_bank: &Arc<HostResourceAccounts>,
    providers: &[(u32, bool, bool)],
    reverse: bool,
) -> ConsumerActionRegistry {
    let services = selection_services(service_bank);
    let mut registrations = datagram_registrations(peer, native, &services, providers);
    if reverse {
        registrations.reverse();
    }
    ConsumerActionRegistry::try_new_fixed(registrations).unwrap()
}

#[test]
fn preferred_query_executes_before_a_later_failing_query_in_either_registration_order() {
    let (address, server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(NativeProbe {
        fail_compilation: Some((2, Operation::QueryAction)),
        ..Default::default()
    });
    *probe.bank.lock().unwrap() = Some(bank.clone());
    let td = control_forms(
        address,
        &[
            ("invokeaction", "run"),
            ("queryaction", "numeric"),
            ("queryaction", "late"),
        ],
    );
    for reverse in [false, true] {
        let registry =
            competing_registry(address, &probe, &services, &TOKEN_PROVIDERS[..3], reverse);
        let select = |form, credit| {
            prepare_with_credit(
                &registry,
                &td,
                &bank,
                ConsumerActionSelectors {
                    query: ConsumerSelector {
                        form,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                GatewayDefaultV1::LIMITS,
                credit,
            )
        };
        let check_pair = |d: &ConsumerActionDefinition| {
            assert_eq!(d.input().invoke.identity.binding_id(), BindingId::new(2));
            assert_eq!(d.input().query.as_ref().unwrap().form, 1);
        };
        let credit = 2 * 3 * 32;
        let pinned = select(Some(1), credit).unwrap();
        check_pair(&pinned);
        drop(pinned);
        clean(&bank);
        assert!(matches!(
            select(Some(2), 1_000_000),
            Err(CoreError::Binding(_))
        ));
        clean(&bank);
        let chosen = select(None, 1_000_000).expect("later Query cannot change the preferred pair");
        check_pair(&chosen);
        drop(chosen);
        clean(&bank);
        assert!(matches!(
            select(None, credit - 1),
            Err(CoreError::Backpressure(_))
        ));
        clean(&bank);
        execute_datagram_action(select(None, credit).unwrap(), &bank);
    }
    server.join().unwrap();
    clean(&services);
}

#[test]
fn query_preference_precedes_cancel_preference_across_invoke_providers() {
    let (address, server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"CANCEL 7", b"STOP REQUESTED"),
        (b"QUERY 7", b"OUTPUT 24"),
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"CANCEL 7", b"STOP REQUESTED"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let td = control_forms(
        address,
        &[
            ("invokeaction", "run"),
            ("queryaction", "ascii"),
            ("queryaction", "numeric"),
            ("cancelaction", "numeric"),
            ("cancelaction", "ascii"),
        ],
    );
    for reverse in [false, true] {
        // Query 1 with Cancel 4 outranks Query 2 with Cancel 3.
        let probe = Arc::new(NativeProbe {
            fail_compilation: Some((2, Operation::QueryAction)),
            ..Default::default()
        });
        *probe.bank.lock().unwrap() = Some(bank.clone());
        let registry = competing_registry(address, &probe, &services, &TOKEN_PROVIDERS, reverse);
        let chosen = prepare_with_credit(
            &registry,
            &td,
            &bank,
            Default::default(),
            GatewayDefaultV1::LIMITS,
            4 * 4 * 32,
        )
        .unwrap();
        assert_eq!(
            chosen.input().invoke.identity.binding_id(),
            BindingId::new(4)
        );
        assert_eq!(chosen.input().query.as_ref().unwrap().form, 1);
        assert_eq!(chosen.input().cancel.as_ref().unwrap().form, 4);
        execute_datagram_action(chosen, &bank);
        drop(registry);

        // A hard error at Cancel 4 can invalidate the earlier Query prefix.
        // Pinning the later Query establishes a viable lower-ranked combination.
        let probe = Arc::new(NativeProbe {
            fail_compilation: Some((4, Operation::CancelAction)),
            ..Default::default()
        });
        let registry = competing_registry(address, &probe, &services, &TOKEN_PROVIDERS, reverse);
        let prepare = |selectors| {
            registry.prepare_action(
                &td,
                "run",
                selectors,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut work(),
            )
        };
        let pinned = prepare(ConsumerActionSelectors {
            query: ConsumerSelector {
                form: Some(2),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
        assert_eq!(pinned.input().cancel.as_ref().unwrap().form, 3);
        drop(pinned);
        clean(&bank);
        assert!(matches!(
            prepare(Default::default()),
            Err(CoreError::Binding(_))
        ));
        assert!(probe.ports.lock().unwrap().is_empty());
        clean(&bank);
    }
    server.join().unwrap();
    clean(&services);
}

#[test]
fn only_complete_assignments_at_the_preferred_forms_count_for_ambiguity() {
    let address = "127.0.0.1:9".parse().unwrap();
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(NativeProbe::default());
    for reverse in [false, true] {
        let registry = competing_registry(address, &probe, &services, &TOKEN_PROVIDERS, reverse);
        let prepare = |registry: &ConsumerActionRegistry, forms: &[(&str, &str)], selectors| {
            registry.prepare_action(
                &control_forms(address, forms),
                "run",
                selectors,
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut work(),
            )
        };
        let both = [
            ("invokeaction", "run"),
            ("queryaction", "both"),
            ("cancelaction", "both"),
        ];
        let ambiguous = |result| {
            assert!(matches!(
                result,
                Err(CoreError::Selection {
                    reason: SelectionFailureReason::AmbiguousBindingOwner,
                    ..
                })
            ))
        };
        // Two Invoke providers each give exactly one complete assignment.
        ambiguous(prepare(&registry, &both, Default::default()));
        clean(&bank);
        // With Query absent or tied, the first compatible Cancel decides.
        // Exactly enough compiler credit for that tuple excludes a standby Cancel.
        for with_query in [false, true] {
            let mut forms = vec![("invokeaction", "run")];
            if with_query {
                forms.push(("queryaction", "both"));
            }
            let first_cancel = forms.len() as u32;
            forms.extend([("cancelaction", "ascii"), ("cancelaction", "numeric")]);
            let selected = prepare_with_credit(
                &registry,
                &control_forms(address, &forms),
                &bank,
                Default::default(),
                GatewayDefaultV1::LIMITS,
                u64::from(first_cancel + 1) * 4 * 32,
            )
            .unwrap();
            assert_eq!(
                selected.input().invoke.identity.binding_id(),
                BindingId::new(4)
            );
            assert_eq!(selected.input().query.is_some(), with_query);
            assert_eq!(selected.input().cancel.as_ref().unwrap().form, first_cancel);
            drop(selected);
            clean(&bank);
        }
        drop(registry);

        let mut providers = TOKEN_PROVIDERS.to_vec();
        providers.push((6, false, true)); // A second independent ASCII control provider.
        let registry = competing_registry(address, &probe, &services, &providers, reverse);
        // Two ASCII Query mappings are immaterial when only the numeric Invoke
        // can finish. Try both a dead-end early Query and a shared Query Form.
        for (queries, expected) in [(["ascii", "numeric"], 2), (["both", "ascii"], 1)] {
            let forms = [
                ("invokeaction", "run"),
                ("queryaction", queries[0]),
                ("queryaction", queries[1]),
                ("cancelaction", "numeric"),
            ];
            let chosen = prepare(&registry, &forms, Default::default()).unwrap();
            assert_eq!(
                chosen.input().invoke.identity.binding_id(),
                BindingId::new(2)
            );
            assert_eq!(chosen.input().query.as_ref().unwrap().form, expected);
            assert_eq!(chosen.input().cancel.as_ref().unwrap().form, 3);
            drop(chosen);
            clean(&bank);
        }
        // Fix both Invoke and Cancel. The two Query mappings now create a tie
        // at the complete tuple. Pinning one of them resolves it.
        let ascii = ConsumerSelector {
            binding: Some(BindingId::new(5)),
            ..Default::default()
        };
        let mut selectors = ConsumerActionSelectors {
            invoke: ConsumerSelector {
                binding: Some(BindingId::new(4)),
                ..Default::default()
            },
            cancel: ascii,
            ..Default::default()
        };
        ambiguous(prepare(&registry, &both, selectors));
        clean(&bank);
        selectors.query = ascii;
        let unique = prepare(&registry, &both, selectors).unwrap();
        assert_eq!(
            unique.input().query.as_ref().unwrap().identity.binding_id(),
            BindingId::new(5)
        );
        drop(unique);
        clean(&bank);
    }
    assert!(probe.ports.lock().unwrap().is_empty());
    clean(&services);
}

#[test]
fn a_compatibility_error_from_a_possible_equal_assignment_reaches_the_caller() {
    let address = "127.0.0.1:9".parse().unwrap();
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(NativeProbe {
        fail_compatibility: Some((Operation::QueryAction, BindingId::new(4))),
        ..Default::default()
    });
    let td = control_forms(address, &[("invokeaction", "run"), ("queryaction", "both")]);
    for reverse in [false, true] {
        let registry = competing_registry(address, &probe, &services, &TOKEN_PROVIDERS, reverse);
        let select = |binding| {
            registry.prepare_action(
                &td,
                "run",
                ConsumerActionSelectors {
                    invoke: ConsumerSelector {
                        binding,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut work(),
            )
        };
        drop(select(Some(BindingId::new(2))).unwrap());
        clean(&bank);
        assert!(matches!(select(None), Err(CoreError::Binding(_))));
        clean(&bank);
    }
    assert!(probe.ports.lock().unwrap().is_empty());
    clean(&services);
}

// Both operations belong to this registration. The endpoint selects the Invoke
// token format and the Query endpoint accepts only the numeric token.
struct EndpointTokenCompiler(Compiler);
impl ConsumerActionPreparation for EndpointTokenCompiler {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.0.identity()
    }
    fn bounds(&self) -> ConsumerPreparationBounds {
        self.0.bounds()
    }
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        Compiler {
            invoke: input.operation == Operation::InvokeAction,
            ascii_continuation: input.endpoint.ends_with("/ascii"),
            ..self.0.clone()
        }
        .compile_action(input)
    }
}

fn endpoint_token_registry(
    address: SocketAddr,
    probe: &Arc<NativeProbe>,
    services: &Arc<HostResourceAccounts>,
) -> ConsumerActionRegistry {
    let compiler = EndpointTokenCompiler(Compiler {
        id: BindingRegistrationIdentity::new(
            BindingId::new(2),
            BindingGeneration::INITIAL,
            configuration().configuration(),
            configuration().native_compatibility(),
            0,
        ),
        invoke: true,
        ascii_continuation: false,
        peer: address,
        media: "application/json",
        probe: probe.clone(),
    });
    ConsumerActionRegistry::try_new_fixed(vec![ConsumerActionRegistration {
        preparation: Box::new(compiler),
        services: selection_services(services),
    }])
    .unwrap()
}

#[test]
fn exhausted_invoke_releases_its_cleanup_slot_and_reuses_its_candidate_storage() {
    let (address, server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(ResourceKind::CleanupItemsMax, Some(2))
        .with_limit(ResourceKind::PeakLiveBytesPerAdmissionMax, Some(108 << 10));
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let probe = Arc::new(NativeProbe {
        mapping_bytes: 36 << 10,
        ..Default::default()
    });
    let registry = endpoint_token_registry(address, &probe, &services);
    let td = control_forms(
        address,
        &[
            ("invokeaction", "ascii"),
            ("invokeaction", "numeric"),
            ("queryaction", "numeric"),
        ],
    );
    let kind = ResourceKind::AdmissionTemporaryBytesPerOperationMax;
    let prepare = |pin, bytes| {
        prepare_with_credit(
            &registry,
            &td,
            &bank,
            ConsumerActionSelectors {
                invoke: ConsumerSelector {
                    form: pin,
                    ..Default::default()
                },
                ..Default::default()
            },
            &limits.clone().with_limit(kind, Some(bytes)),
            3 * 32,
        )
    };
    // Calibrate the pinned pair's storage on this target. Automatic selection
    // must fit that same allowance and reuse Query 2 after rejecting Invoke 0.
    let (mut low, mut high) = (0, limits.get(kind).unwrap());
    drop(prepare(Some(1), high).unwrap());
    while low < high {
        let mid = low + (high - low) / 2;
        match prepare(Some(1), mid) {
            Ok(_) => high = mid,
            Err(CoreError::LimitExceeded { .. } | CoreError::Backpressure(_)) => low = mid + 1,
            Err(e) => panic!("unexpected preparation error {e:?}"),
        }
        clean(&bank);
    }
    let selected = prepare(None, low).unwrap();
    assert_eq!(
        (
            selected.input().invoke.form,
            selected.input().query.as_ref().unwrap().form
        ),
        (1, 2)
    );
    assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 2);
    assert_eq!(probe.mapping_peak.load(Ordering::Relaxed), 72 << 10);
    // Execution has its own source of capacity after preparation.
    let calls_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    *probe.bank.lock().unwrap() = Some(calls_bank.clone());
    execute_datagram_action(selected, &calls_bank);
    server.join().unwrap();
    clean(&bank);
    drop(registry);
    clean(&services);
}

#[test]
fn two_complete_assignments_end_native_probing_at_the_deciding_operation() {
    let address = "127.0.0.1:9".parse().unwrap();
    let services_bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let services = selection_services(&services_bank);
    let good = Arc::new(NativeProbe::default());
    let forms = [
        ("invokeaction", "run"),
        ("queryaction", "numeric"),
        ("cancelaction", "numeric"),
    ];
    for (last, operation) in [
        Operation::InvokeAction,
        Operation::QueryAction,
        Operation::CancelAction,
    ]
    .into_iter()
    .enumerate()
    {
        let td = control_forms(address, &forms[..=last]);
        let bad = Arc::new(NativeProbe {
            fail_compilation: Some((last as u32, operation)),
            ..Default::default()
        });
        let providers: &[(u32, bool, bool)] = if last == 0 {
            &[(2, true, false), (3, true, false)]
        } else {
            &[(2, true, false), (3, false, false), (4, false, false)]
        };
        let mut registrations = datagram_registrations(address, &good, &services, providers);
        registrations.extend(datagram_registrations(
            address,
            &bad,
            &services,
            &[(9, last == 0, false)],
        ));
        let registry = ConsumerActionRegistry::try_new_fixed(registrations).unwrap();
        let binding = |id| ConsumerSelector {
            binding: Some(BindingId::new(id)),
            ..Default::default()
        };
        let mut selectors = ConsumerActionSelectors::default();
        if last > 0 {
            selectors.invoke = binding(2);
        }
        if last == 2 {
            selectors.query = binding(3);
        }
        let needed = [64, 128, 160][last];
        let prepare = |selectors, credit| {
            let mut budget = work().with_remaining(WorkClass::BindingPolls, credit);
            let result = registry
                .prepare_action(
                    &td,
                    "run",
                    selectors,
                    GatewayDefaultV1::LIMITS,
                    bank.clone(),
                    &mut budget,
                )
                .map(drop);
            clean(&bank);
            (result, credit - budget.remaining(WorkClass::BindingPolls))
        };
        assert!(matches!(
            prepare(selectors, needed - 1).0,
            Err(CoreError::Backpressure(_))
        ));
        for credit in [needed, needed + 32] {
            assert!(
                matches!(prepare(selectors,credit),(Err(CoreError::Selection{reason: SelectionFailureReason::AmbiguousBindingOwner,..}),spent) if spent==needed)
            );
        }
        // The same hard compiler error is observable when it can decide the tuple.
        match operation {
            Operation::InvokeAction => selectors.invoke = binding(9),
            Operation::QueryAction => selectors.query = binding(9),
            Operation::CancelAction => selectors.cancel = binding(9),
            _ => unreachable!(),
        }
        assert!(matches!(
            prepare(selectors, needed + 32).0,
            Err(CoreError::Binding(_))
        ));
        assert!(bad.ports.lock().unwrap().is_empty());
    }
    drop(services);
    clean(&services_bank);
}

#[test]
fn impossible_form_and_binding_pins_spend_zero_compiler_work() {
    let address = "127.0.0.1:9".parse().unwrap();
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let td = control_forms(
        address,
        &[("invokeaction", "run"), ("queryaction", "numeric")],
    );
    let probe = Arc::new(NativeProbe {
        fail_compilation: Some((0, Operation::InvokeAction)),
        ..Default::default()
    });
    let registry = competing_registry(address, &probe, &services, &TOKEN_PROVIDERS[..3], false);
    for (form, binding) in [
        (Some(0), None),
        (Some(2), None),
        (Some(1), Some(BindingId::new(89))),
    ] {
        for credit in [0, 128] {
            let mut budget = work().with_remaining(WorkClass::BindingPolls, credit);
            let result = registry.prepare_action(
                &td,
                "run",
                ConsumerActionSelectors {
                    query: ConsumerSelector { form, binding },
                    ..Default::default()
                },
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut budget,
            );
            assert!(matches!(result, Err(CoreError::UnsupportedOperation(_))));
            assert_eq!(budget.remaining(WorkClass::BindingPolls), credit);
            clean(&bank);
        }
    }
    assert!(probe.ports.lock().unwrap().is_empty());
    drop(registry);
    clean(&services);
}

#[test]
fn empty_provider_sets_for_a_required_control_rule_out_every_later_invoke() {
    let address = "127.0.0.1:9".parse().unwrap();
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    for (control, count) in [("queryaction", 1), ("cancelaction", 2)] {
        let mut forms = vec![("invokeaction", "first")];
        forms.extend(std::iter::repeat_n((control, "numeric"), count));
        let last = forms.len();
        forms.push(("invokeaction", "later"));
        let td = control_forms(address, &forms);
        let probe = Arc::new(NativeProbe {
            fail_compilation: Some((last as u32, Operation::InvokeAction)),
            ..Default::default()
        });
        let registry = competing_registry(address, &probe, &services, &[(2, true, false)], false);
        let decisive = 32 * last as u64;
        for credits in [decisive - 1, decisive, decisive + 32] {
            let mut budget = work().with_remaining(WorkClass::BindingPolls, credits);
            let result = registry.prepare_action(
                &td,
                "run",
                Default::default(),
                GatewayDefaultV1::LIMITS,
                bank.clone(),
                &mut budget,
            );
            if credits < decisive {
                assert!(matches!(result, Err(CoreError::Backpressure(_))));
            } else {
                assert!(matches!(result, Err(CoreError::UnsupportedOperation(_))));
                assert_eq!(
                    budget.remaining(WorkClass::BindingPolls),
                    credits - decisive
                );
            }
            clean(&bank);
        }
        assert!(probe.ports.lock().unwrap().is_empty());
    }
    clean(&services);
}

#[test]
fn executable_mappings_fit_the_admission_peak_together() {
    use ResourceKind::*;
    let (address, server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let services = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let probe = Arc::new(NativeProbe {
        mapping_bytes: 36 << 10,
        ..Default::default()
    });
    *probe.bank.lock().unwrap() = Some(calls.clone());
    let registry = endpoint_token_registry(address, &probe, &services);
    let forms = [("invokeaction", "first"), ("queryaction", "numeric")];
    for (kind, ceiling, count) in [
        (PeakLiveBytesPerAdmissionMax, 64 << 10, 1),
        (PeakLiveBytesPerAdmissionMax, 64 << 10, 2),
        (AdmissionPeakLiveBytesGlobalMax, 64 << 10, 2),
        (AdmissionPeakLiveBytesGlobalMax, 192 << 10, 2),
    ] {
        probe.mapping_peak.store(0, Ordering::Relaxed);
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .with_limit(kind, Some(ceiling))
            .with_limit(AdmissionTemporaryBytesPerOperationMax, Some(48 << 10));
        let bank = Arc::new(HostResourceAccounts::new(&limits));
        let td = control_forms(address, &forms[..count]);
        let prepared = registry.prepare_action(
            &td,
            "run",
            Default::default(),
            &limits,
            bank.clone(),
            &mut work(),
        );
        assert_eq!(bank.account(AdmissionPeakLiveBytesGlobalMax).used(), 0);
        if count == 2 && ceiling == 64 << 10 {
            assert!(
                matches!(prepared, Err(CoreError::LimitExceeded {resource, ..}) if resource == kind)
            );
            assert_eq!(probe.mapping_peak.load(Ordering::Relaxed), 36 << 10);
        } else {
            let definition = prepared.unwrap();
            let physical = count * probe.mapping_bytes;
            assert_eq!(probe.mapping_live.load(Ordering::Relaxed), physical);
            assert!(bank.account(EngineLiveBytesGlobalMax).used() >= physical as u64);
            if count == 2 {
                execute_datagram_action(definition, &calls);
            }
        }
        assert_eq!(probe.mapping_live.load(Ordering::Relaxed), 0);
        clean(&bank);
    }
    server.join().unwrap();
    drop(registry);
    clean(&services);
}

#[test]
fn selected_mappings_and_schema_stay_funded_through_the_closing_revision() {
    use ResourceKind::{AdmissionPeakLiveBytesGlobalMax, EngineLiveBytesGlobalMax};
    use std::cell::Cell;
    let address = "127.0.0.1:9".parse().unwrap();
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .with_limit(AdmissionPeakLiveBytesGlobalMax, Some(128 << 10));
    let services_bank = Arc::new(HostResourceAccounts::new(&limits));
    let json = clinkz_wot_binding_http::HttpConsumerConfig::for_cleartext_ip()
        .unwrap()
        .consumer_services()
        .data;
    let services = RetainedConsumerServices {
        data: ConsumerServiceOwner::retain_data(json, &services_bank)
            .ok()
            .unwrap(),
        access: ConsumerServiceOwner::retain_access(Arc::new(Access), &services_bank)
            .ok()
            .unwrap(),
    };
    let mut td = control_forms(
        address,
        &[
            ("invokeaction", "run"),
            ("queryaction", "numeric"),
            ("cancelaction", "numeric"),
        ],
    );
    td.actions.as_mut().unwrap().get_mut("run").unwrap().output = Some(
        serde_json::from_value(serde_json::json!({
            "type":"object","properties":{"reading":{"type":"integer"}},"required":["reading"]
        }))
        .unwrap(),
    );
    for closing_revision in [Some(7), Some(8), None] {
        let bank = Arc::new(HostResourceAccounts::new(&limits));
        let native = Arc::new(NativeProbe {
            mapping_bytes: 12 << 10,
            ..Default::default()
        });
        *native.bank.lock().unwrap() = Some(bank.clone());
        let observation = Rc::new(Cell::new(None));
        let registry = ConsumerActionRegistry::try_new_revisioned(
            datagram_registrations(
                address,
                &native,
                &services,
                &[(2, true, false), (3, false, false)],
            ),
            {
                let bank = bank.clone();
                let native = native.clone();
                let observation = observation.clone();
                let opened = Cell::new(false);
                move || {
                    if !opened.replace(true) {
                        return Some(7);
                    }
                    observation.set(Some((
                        native.mapping_live.load(Ordering::Relaxed),
                        bank.account(AdmissionPeakLiveBytesGlobalMax).used(),
                        bank.account(EngineLiveBytesGlobalMax).used(),
                    )));
                    closing_revision
                }
            },
        )
        .unwrap();
        let result = registry.prepare_action(
            &td,
            "run",
            Default::default(),
            &limits,
            bank.clone(),
            &mut work(),
        );
        let (physical, peak, engine) = observation.get().unwrap();
        assert_eq!(physical, 36 << 10);
        assert!(engine > physical as u64);
        assert_eq!(
            peak, engine,
            "the closing observer sees every provisional Engine byte"
        );
        assert_eq!(bank.account(AdmissionPeakLiveBytesGlobalMax).used(), 0);
        if closing_revision == Some(7) {
            let definition = result.unwrap();
            assert_eq!(bank.account(EngineLiveBytesGlobalMax).used(), engine);
            drop(definition);
            assert_eq!(native.mapping_drop_charge.load(Ordering::Relaxed), 0);
        } else {
            assert!(matches!(result, Err(CoreError::StaleHandle(_))));
            assert_eq!(
                native.mapping_drop_charge.load(Ordering::Relaxed) as u64,
                peak
            );
        }
        assert_eq!(native.mapping_live.load(Ordering::Relaxed), 0);
        clean(&bank);
    }
    drop(services);
    clean(&services_bank);
}

#[test]
fn empty_control_forms_reuse_the_pinned_choices_temporary_capacity() {
    let (address, server) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"CANCEL 7", b"STOP REQUESTED"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let limits = GatewayDefaultV1::LIMITS;
    let services = Arc::new(HostResourceAccounts::new(limits));
    let bank = Arc::new(HostResourceAccounts::new(limits));
    let execution = Arc::new(HostResourceAccounts::new(limits));
    let native = Arc::new(NativeProbe::default());
    *native.bank.lock().unwrap() = Some(execution.clone());
    let registry = endpoint_token_registry(address, &native, &services);
    let mut forms = vec![("invokeaction", "ascii"), ("invokeaction", "numeric")];
    for role in ["queryaction", "cancelaction"] {
        forms.extend(std::iter::repeat_n((role, "absent"), 8));
        forms.push((role, "numeric"));
    }
    let mut td = control_forms(address, &forms);
    // The only registration declines the Forms directed to a different peer.
    let absent =
        clinkz_wot_td::data_type::FormHref::parse("linkedudp://127.0.0.1:0/absent").unwrap();
    for form in &mut td
        .actions
        .as_mut()
        .unwrap()
        .get_mut("run")
        .unwrap()
        ._interaction
        .forms
    {
        if form.href.as_str().ends_with("/absent") {
            form.href = absent.clone();
        }
    }
    let pin = |form| ConsumerSelector {
        form: Some(form),
        ..Default::default()
    };
    let pinned = ConsumerActionSelectors {
        query: pin(10),
        cancel: pin(19),
        ..Default::default()
    };
    drop(
        registry
            .prepare_action(&td, "run", pinned, limits, bank.clone(), &mut work())
            .unwrap(),
    );
    let paid = bank
        .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
        .peak();
    clean(&bank);
    let limits = limits.clone().with_limit(
        ResourceKind::AdmissionTemporaryBytesPerOperationMax,
        Some(paid),
    );
    // The first Invoke rejects the numeric Query. Trying the second Invoke
    // reuses the completed Query probes, including all eight empty results.
    let mut budget = work().with_remaining(WorkClass::BindingPolls, forms.len() as u64 * 32);
    let definition = registry
        .prepare_action(
            &td,
            "run",
            Default::default(),
            &limits,
            bank.clone(),
            &mut budget,
        )
        .unwrap();
    assert_eq!(budget.remaining(WorkClass::BindingPolls), 0);
    let selected = definition.input();
    assert_eq!(
        (
            selected.invoke.form,
            selected.query.as_ref().unwrap().form,
            selected.cancel.as_ref().unwrap().form
        ),
        (1, 10, 19)
    );
    execute_datagram_action(definition, &execution);
    server.join().unwrap();
    clean(&bank);
    drop(registry);
    clean(&services);
}
