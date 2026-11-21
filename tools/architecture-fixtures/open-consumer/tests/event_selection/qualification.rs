use super::*;

struct Media {
    primary: &'static str,
    json: AtomicBool,
}

impl ConsumerDataService for Media {
    fn qualification_revision(&self) -> u64 {
        u64::from(self.json.load(SeqCst))
    }
    fn content_type(&self) -> &str {
        self.primary
    }
    fn supports_media(&self, media: &str) -> bool {
        media == "application/json" && self.json.load(SeqCst)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 64,
            temporary_bytes: 4096,
            body_bytes: 16,
            encoded_selection_bytes: 0,
            validation_work: 64,
        }
    }
    fn validate(&self, schema: &data_schema::PreparedSchema, payload: &Payload) -> CoreResult<()> {
        if !self.supports_media(&payload.content_type) {
            return Err(invalid());
        }
        clinkz_wot_codec_json::validate_payload(schema, payload)
    }
}

async fn exchange_digit(definition: ConsumerNotificationDefinition<Datagram>, peers: &Peers) {
    let active = bank();
    let digit: &[u8] = if definition.input().program.hex {
        b"35"
    } else {
        b"5"
    };
    let mut conversation = Conversation::start(definition, &active);
    conversation.until(NotificationState::Active).await;
    let port = peers.packet(b"LISTEN");
    assert_eq!(port.ip(), peers.addr.ip());
    peers.datagram.send_to(digit, port).unwrap();
    conversation.receive(b"5").await;
    assert_eq!(
        conversation.finish(true).await.remote_cancellation,
        RemoteCancellation::Unknown
    );
    assert_eq!(peers.packet(b"UNLISTEN"), port);
    drop(UdpSocket::bind(port).expect("recipient port released"));
    clean(&active);
}

#[test]
fn executable_scalar_media_qualifies_selection_and_the_surviving_definition() {
    let peers = Peers::new();
    let mut deployment = Deployment::new(peers.addr);
    let incompatible = Arc::new(Media {
        primary: "application/json",
        json: false.into(),
    });
    let compatible = Arc::new(Media {
        primary: "application/cbor",
        json: true.into(),
    });
    deployment.stream_services.data =
        ConsumerServiceOwner::retain_data(incompatible, &deployment.services)
            .ok()
            .unwrap();
    deployment.datagram_services.data =
        ConsumerServiceOwner::retain_data(compatible.clone(), &deployment.services)
            .ok()
            .unwrap();
    let registry = deployment.registry(false);
    let plans = bank();
    let shared = td(peers.addr, &[(None, None)]);
    assert!(matches!(
        select(&registry, &shared, pin(None, Some(71)), &plans, &mut work()),
        Err(CoreError::UnsupportedOperation(_))
    ));
    let SelectedEvent::Datagram(selected) =
        select(&registry, &shared, pin(None, None), &plans, &mut work()).unwrap()
    else {
        panic!("only the datagram service validates JSON")
    };
    peers.quiet();
    runtime().block_on(exchange_digit(selected, &peers));
    let SelectedEvent::Datagram(qualified) =
        select(&registry, &shared, pin(None, Some(72)), &plans, &mut work()).unwrap()
    else {
        panic!()
    };
    compatible.json.store(false, SeqCst);
    assert!(qualified.check_dependencies().is_err());
    let mut input = qualified.into_input();
    input.data = deployment.stream_services.data.clone();
    assert!(ConsumerNotificationDefinition::try_new(input).is_err());
    clean(&plans);
}

#[test]
fn control_only_events_require_scalar_media_and_collections_keep_their_exact_representation() {
    let service = Media {
        primary: "application/cbor",
        json: true.into(),
    };
    let event = |cancellation| NotificationSchemas::Event {
        subscription: None,
        data: None,
        data_response: None,
        cancellation,
    };
    assert!(event(None).supports_data(&service, "text/plain"));
    let cancel = event(Some(data_schema::PreparedSchema::Type(
        data_schema::ValueType::Integer,
    )));
    assert!(cancel.supports_data(&service, "application/json"));
    assert!(!cancel.supports_data(&service, "application/cbor"));

    let collection = NotificationSchemas::SubscribeAllEvents(Arc::new(
        collection::EventCollectionSchema::try_new(Vec::new()).unwrap(),
    ));
    assert!(collection.supports_data(&service, "application/cbor"));
    assert!(!collection.supports_data(&service, "application/json"));

    let integer = data_schema::PreparedSchema::Type(data_schema::ValueType::Integer);
    for (data, data_response) in [(Some(integer.clone()), None), (None, Some(integer))] {
        let member = collection::EventMemberSchema {
            name: "alarm".into(),
            subscription: None,
            data,
            data_response,
            cancellation: None,
        };
        let collection = NotificationSchemas::SubscribeAllEvents(Arc::new(
            collection::EventCollectionSchema::try_new(vec![member]).unwrap(),
        ));
        assert!(
            !collection.supports_data(&service, "application/cbor"),
            "member needs a scalar codec"
        );
        assert!(
            !collection.supports_data(&service, "application/json"),
            "collection needs exact primary media"
        );
    }
}

#[test]
fn only_a_resolved_datagram_destination_spends_the_single_native_probe() {
    use clinkz_wot_td::data_type::{BaseUri, FormHref};

    let peers = Peers::new();
    let d = Deployment::new(peers.addr);
    d.stream_access.missing.store(true, SeqCst);
    let registry = d.registry(false);
    let plans = bank();
    let base = BaseUri::parse(&format!("testevent://{}/", peers.addr)).unwrap();
    let endpoint = format!("testevent://{}/alarm", peers.addr);
    for (href, base, expected) in [
        ("alarm", None, 1),
        ("alarm", Some(base), 0),
        ("testevent://source/{channel}", None, 1),
    ] {
        let mut source = td(peers.addr, &[(Some("datagram"), None); 2]);
        source.base = base;
        let alarm = source.events.as_mut().unwrap().get_mut("alarm").unwrap();
        alarm._interaction.forms[0].href = FormHref::parse(href).unwrap();

        let before = d.datagram.compilations.get();
        for form in [None, Some(expected)] {
            let mut budget = work().with_remaining(
                WorkClass::BindingPolls,
                d.datagram.preparation_bounds().work,
            );
            let SelectedEvent::Datagram(selected) =
                select(&registry, &source, pin(form, None), &plans, &mut budget).unwrap()
            else {
                panic!("the concrete datagram Form must win")
            };
            assert_eq!(selected.input().form, expected);
            assert_eq!(selected.input().endpoint.as_ref(), endpoint);
            runtime().block_on(exchange_digit(selected, &peers));
            clean(&plans);
        }
        if expected == 1 {
            let mut no_native_work = work().with_remaining(WorkClass::BindingPolls, 0);
            assert!(matches!(
                select(
                    &registry,
                    &source,
                    pin(Some(0), None),
                    &plans,
                    &mut no_native_work
                ),
                Err(CoreError::UnsupportedOperation(_))
            ));
            clean(&plans);
        }
        assert_eq!(d.datagram.compilations.get(), before + 2);
    }
}

#[test]
fn a_denial_only_describes_an_otherwise_executable_candidate() {
    let peers = Peers::new();
    let deployment = Deployment::new(peers.addr);
    deployment.stream_access.denied.store(true, SeqCst);
    let plans = bank();
    for reverse in [false, true] {
        let registry = deployment.registry(reverse);
        let prepare = |forms: &[(Option<&str>, Option<&str>)]| {
            select(
                &registry,
                &td(peers.addr, forms),
                pin(None, None),
                &plans,
                &mut work(),
            )
        };
        assert!(matches!(
            prepare(&[(Some("unknown"), None)]),
            Err(CoreError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            prepare(&[(Some("stream"), None)]),
            Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                ..
            })
        ));
        let Ok(SelectedEvent::Datagram(selected)) =
            prepare(&[(Some("stream"), None), (Some("datagram"), None)])
        else {
            panic!("the authorized fallback must win")
        };
        assert_eq!(selected.input().form, 1);
    }
    peers.quiet();
    clean(&plans);
}

#[test]
fn denied_competitors_leave_the_only_cleanup_slot_and_compiler_credit_for_authorized_forms() {
    let peers = Peers::new();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::CleanupItemsMax, Some(1)));
    let plans = Arc::new(HostResourceAccounts::new(&limits));
    for reverse in [true, false] {
        let d = Deployment::new(peers.addr);
        d.stream_access.denied.store(true, SeqCst);
        let registry = d.registry(reverse);
        let shared = [(None, None)];
        let fallback = [(Some("stream"), None), (Some("datagram"), None)];
        for forms in [shared.as_slice(), fallback.as_slice()] {
            let thing = td(peers.addr, forms);
            let units = d.datagram.preparation_bounds().work * forms.len() as u64;
            for mut budget in [
                work(),
                work().with_remaining(WorkClass::BindingPolls, units),
            ] {
                let SelectedEvent::Datagram(selected) =
                    select(&registry, &thing, pin(None, None), &plans, &mut budget).unwrap()
                else {
                    panic!("only UDP is authorized")
                };
                assert_eq!(selected.input().form as usize, forms.len() - 1);
                assert_eq!(plans.account(ResourceKind::CleanupItemsMax).used(), 1);
                runtime().block_on(exchange_digit(selected, &peers));
                clean(&plans);
            }
        }
        assert_eq!(d.stream.compilations.load(SeqCst), 0);
        let both = td(peers.addr, &shared);
        for form in [None, Some(0)] {
            assert!(matches!(
                select(&registry, &both, pin(form, Some(71)), &plans, &mut work()),
                Err(CoreError::Security {
                    reason: SecurityFailureReason::AuthorizationDenied,
                    ..
                })
            ));
        }
        assert_eq!(d.stream.compilations.load(SeqCst), 2);
        clean(&plans);
    }
}

#[test]
fn a_deferred_denial_spends_the_remaining_work_and_is_fenced_at_the_original_revision() {
    let peers = Peers::new();
    let d = Deployment::new(peers.addr);
    d.stream_access.denied.store(true, SeqCst);
    let thing = td(
        peers.addr,
        &[(Some("unknown"), None), (Some("stream"), None)],
    );
    let plans = bank();
    let selector = pin(None, Some(71));
    let mut short =
        work().with_remaining(WorkClass::BindingPolls, d.stream.preparation_bounds().work);
    assert!(matches!(
        select(&d.registry(false), &thing, selector, &plans, &mut short),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(d.stream.compilations.load(SeqCst), 1);
    clean(&plans);
    // Inject a configuration change during each native diagnostic compilation.
    let compilations = d.stream.compilations.clone();
    let registry =
        ConsumerNotificationRegistry::try_new_revisioned(d.providers(false), move || {
            Some(compilations.load(SeqCst) as u64)
        })
        .unwrap();
    assert!(matches!(
        select(&registry, &thing, selector, &plans, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    assert_eq!(d.stream.compilations.load(SeqCst), 3);
    peers.quiet();
    clean(&plans);
}

#[test]
fn coded_datagram_delivers_and_releases_its_ipv6_recipient() {
    let peers = Peers::on(std::net::Ipv6Addr::LOCALHOST.into());
    let deployment = Deployment::new(peers.addr);
    let plans = bank();
    let SelectedEvent::Datagram(definition) = select(
        &deployment.registry(false),
        &td(peers.addr, &[(Some("datagram"), Some("x-test-hex"))]),
        pin(None, None),
        &plans,
        &mut work(),
    )
    .unwrap() else {
        panic!()
    };
    runtime().block_on(exchange_digit(definition, &peers));
    clean(&plans);
}

#[test]
fn homogeneous_registries_can_move_to_a_worker_with_their_native_binding() {
    fn share<T: Send + Sync>(registry: T) -> T {
        registry
    }
    let peers = Peers::new();
    let deployment = Deployment::new(peers.addr);
    let raw = share(
        ConsumerNotificationRegistry::try_new(
            vec![ConsumerNotificationRegistration {
                binding: deployment.stream.clone(),
                services: ConsumerServices {
                    data: independent_open_consumer::services(Arc::default()).data,
                    access: deployment.stream_access.clone(),
                },
            }],
            &deployment.services,
        )
        .unwrap(),
    );
    let retained = share(
        ConsumerNotificationRegistry::try_from_retained(vec![
            RetainedConsumerNotificationRegistration {
                binding: deployment.stream.clone(),
                services: deployment.stream_services.clone(),
            },
        ])
        .unwrap(),
    );
    let addr = peers.addr;
    std::thread::spawn(move || {
        let plans = bank();
        for registry in [raw, retained] {
            let definition = registry
                .prepare_notification(
                    &td(addr, &[(Some("stream"), None)]),
                    &alarm(),
                    NotificationKind::Event,
                    ConsumerSelector::default(),
                    GatewayDefaultV1::LIMITS,
                    plans.clone(),
                    &mut work(),
                )
                .unwrap();
            assert_eq!(definition.input().program, addr);
        }
        clean(&plans);
    })
    .join()
    .unwrap();
    peers.quiet();
}

const ROLLBACK: u64 = 4096;

struct DropAudit {
    bank: Arc<HostResourceAccounts>,
    entry: AtomicU64,
    disposal: AtomicU64,
}
impl DropAudit {
    fn temporary(&self) -> u64 {
        self.bank
            .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
            .used()
    }
}
struct CountedProgram {
    peer: SocketAddr,
    audit: Arc<DropAudit>,
}
impl Drop for CountedProgram {
    fn drop(&mut self) {
        self.audit.disposal.store(self.audit.temporary(), SeqCst);
    }
}
#[derive(Clone)]
struct AuditedStream {
    stream: Stream,
    audit: Arc<DropAudit>,
    accepts: bool,
}
impl ConsumerNotificationBinding for AuditedStream {
    type Configuration = SocketAddr;
    type Program = CountedProgram;
    type Participation = StreamSubscription;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.stream.identity()
    }
    fn configuration(&self) -> &SocketAddr {
        self.stream.configuration()
    }
    fn support(
        &self,
        program: &CountedProgram,
        kind: NotificationKind,
    ) -> CoreResult<NotificationSupport> {
        if !self.accepts {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        self.stream.support(&program.peer, kind)
    }
    fn prepare(
        &mut self,
        program: CountedProgram,
        recipient: &NotificationIdentity,
    ) -> Result<StreamSubscription, BindingInputRejection<CountedProgram>> {
        if let Err(error) = self.support(&program, NotificationKind::Event) {
            return Err(BindingInputRejection::new(
                program,
                BindingOperationalError::new(error),
            ));
        }
        self.stream
            .prepare(program.peer, recipient)
            .map_err(|e| BindingInputRejection::new(program, e.into_parts().1))
    }
}
impl ConsumerNotificationPreparation for AuditedStream {
    fn registration_identity(&self) -> BindingRegistrationIdentity {
        self.stream.registration_identity()
    }
    fn preparation_bounds(&self) -> ConsumerPreparationBounds {
        ConsumerPreparationBounds {
            temporary_bytes: ROLLBACK,
            ..self.stream.preparation_bounds()
        }
    }
    fn compile_notification_form(
        &self,
        input: ConsumerNotificationInput<'_>,
    ) -> CoreResult<Option<CountedProgram>> {
        self.audit.entry.store(self.audit.temporary(), SeqCst);
        Ok(self
            .stream
            .compile_notification_form(input)?
            .map(|peer| CountedProgram {
                peer,
                audit: self.audit.clone(),
            }))
    }
}

#[test]
fn native_program_disposal_observes_rollback_funding_at_refusal_and_at_revision_close() {
    let peers = Peers::new();
    let deployment = Deployment::new(peers.addr);
    let plans = bank();
    let audit = Arc::new(DropAudit {
        bank: plans.clone(),
        entry: 0.into(),
        disposal: u64::MAX.into(),
    });
    let thing = td(peers.addr, &[(Some("stream"), None)]);
    for accepts in [false, true] {
        audit.disposal.store(u64::MAX, SeqCst);
        let registration = RetainedConsumerNotificationRegistration {
            binding: AuditedStream {
                stream: deployment.stream.clone(),
                audit: audit.clone(),
                accepts,
            },
            services: deployment.stream_services.clone(),
        };
        let revision = Cell::new(0);
        let registry = ConsumerNotificationRegistry::try_new_revisioned(
            vec![registration.into_provider(std::convert::identity)],
            move || {
                let observed = revision.get();
                revision.set(observed + u64::from(accepts));
                Some(observed)
            },
        )
        .unwrap();
        let result = registry.prepare_notification(
            &thing,
            &alarm(),
            NotificationKind::Event,
            ConsumerSelector::default(),
            GatewayDefaultV1::LIMITS,
            plans.clone(),
            &mut work(),
        );
        let entry = audit.entry.load(SeqCst);
        assert!(entry >= ROLLBACK);
        if accepts {
            assert!(matches!(result, Err(CoreError::StaleHandle(_))));
            // The TD walk and arena are gone by the time the provisional winner rolls back.
            assert_eq!(audit.disposal.load(SeqCst), ROLLBACK);
        } else {
            assert!(matches!(result, Err(CoreError::UnsupportedOperation(_))));
            assert_eq!(audit.disposal.load(SeqCst), entry);
        }
        clean(&plans);
    }
}

#[test]
fn competing_native_probes_reuse_idle_rollback_capacity_in_either_order() {
    let peers = Peers::new();
    let deployment = Deployment::new(peers.addr);
    let plans = bank();
    let audit = Arc::new(DropAudit {
        bank: plans.clone(),
        entry: 0.into(),
        disposal: u64::MAX.into(),
    });
    let thing = td(peers.addr, &[(None, None)]);
    let prepare = |reverse, selector, limits: &ResourceLimits| {
        let mut providers = vec![
            RetainedConsumerNotificationRegistration {
                binding: AuditedStream {
                    stream: deployment.stream.clone(),
                    audit: audit.clone(),
                    accepts: true,
                },
                services: deployment.stream_services.clone(),
            }
            .into_provider(|_| ()),
            RetainedConsumerNotificationRegistration {
                binding: deployment.datagram.clone(),
                services: deployment.datagram_services.clone(),
            }
            .into_provider(|_| ()),
        ];
        if reverse {
            providers.reverse();
        }
        ConsumerNotificationRegistry::try_new_fixed(providers)
            .unwrap()
            .prepare_notification(
                &thing,
                &alarm(),
                NotificationKind::Event,
                selector,
                limits,
                plans.clone(),
                &mut work(),
            )
    };
    prepare(false, pin(None, Some(71)), GatewayDefaultV1::LIMITS).unwrap();
    // The deployment's output constructor discarded the committed definition.
    assert_eq!(audit.disposal.load(SeqCst), 0);
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    let compiler_entry = audit.entry.load(SeqCst);
    assert!(limits.set(
        ResourceKind::AdmissionTemporaryBytesPerOperationMax,
        Some(compiler_entry)
    ));
    for reverse in [false, true] {
        assert!(matches!(
            prepare(reverse, pin(None, None), &limits),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::AmbiguousBindingOwner,
                ..
            })
        ));
        assert_eq!(audit.entry.load(SeqCst), compiler_entry);
        assert_eq!(audit.disposal.load(SeqCst), compiler_entry);
        clean(&plans);
    }
}
