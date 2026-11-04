#![cfg(feature = "prepare")]
//! Selected writable intent, native group effects and original evidence.
include!("support/collection.rs");
include!("support/collection_write.rs");

const WRITE: Operation = Operation::WriteMultipleProperties;
const VALUES: &[u8] = br#"{"on":false,"secret":"updated"}"#;
fn write_description(addr: SocketAddr) -> Value {
    collection_description(addr, WRITE)
}
fn start_write(client: &ConsumerRuntime) -> ConsumerCall {
    start_collection_at(client, WRITE, Deadline::NONE).unwrap()
}
fn invalid_inputs() -> Vec<Vec<u8>> {
    [
        b"null".as_slice(),
        b"[]",
        br#"{"level":7}"#,
        br#"{"on":false,"secret":"updated","unknown":0}"#,
        br#"{"on":0,"secret":"updated"}"#,
        br#"{"on":false,"secret":7}"#,
        br#"{"on":false,"on":true}"#,
        br#"{"on":false,"\u006fn":true}"#,
        br#"{"on":false,"secret":"\uD800"}"#,
        br#"{"on":false} {}"#,
        br#"{"on":false,}"#,
    ]
    .into_iter()
    .map(Vec::from)
    .chain([vec![b' '; 4097]])
    .collect()
}

#[test]
fn one_retained_td_writes_different_subsets_without_touching_omitted_members() {
    io().block_on(async {
        let addr = address();
        let mut td = write_description(addr);
        td["forms"]
            .as_array_mut()
            .unwrap()
            .push(description(addr)["forms"][0].clone());
        let mut server = Fixture::new(td.clone());
        *server.app.applied.borrow_mut() = json!({"on":true,"secret":"original","level":7});
        let accounts = bank();
        let owner = consumer_instance();
        let client = client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        for (input, expected) in [
            (
                br#"{"on":false}"#.as_slice(),
                json!({"on":false,"secret":"original","level":7}),
            ),
            (
                br#"{"secret":"updated"}"#,
                json!({"on":false,"secret":"updated","level":7}),
            ),
            (br#"{}"#, json!({"on":false,"secret":"updated","level":7})),
            (
                br#"{"on":true,"secret":"last"}"#,
                json!({"on":true,"secret":"last","level":7}),
            ),
        ] {
            let input = payload(input);
            let mut call = client
                .write_multiple_properties(input, Deadline::NONE, now(0), &mut work())
                .unwrap_or_else(|_| panic!());
            let (local, remote) = server.exchange(&mut call).await;
            assert!(local.result().unwrap().data().is_none());
            assert!(
                local
                    .result()
                    .unwrap()
                    .metadata()
                    .property_write_report()
                    .is_none(),
                "HTTP invents no member receipts"
            );
            assert_eq!(local.operation(), WRITE);
            assert_eq!(local.affordance(), &AffordanceTarget::Thing);
            assert_eq!(local.binding().binding_id(), BindingId::new(98));
            assert_eq!(local.effect(), ConsumerEffect::Completed);
            assert_eq!(remote.effect, FiniteEffect::Completed);
            assert_eq!(remote.delivery, FiniteDelivery::Delivered);
            assert_eq!(*server.app.applied.borrow(), expected);
        }
        assert_eq!(server.controls.requests.get(), 4);
        assert_eq!(server.app.roots.get(), 4);
        assert_eq!((server.app.reads.get(), server.app.writes.get()), (0, 0));
        assert!(
            client
                .write_all_properties(payload(VALUES), Deadline::NONE, now(0), &mut work())
                .is_err(),
            "root operations do not acquire one another"
        );
        let mut read = start_read(&client);
        let (local, _) = server.exchange(&mut read).await;
        assert!(local.result().is_ok());
        drop(local);
        drop(read);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn invalid_input_and_short_work_or_capacity_return_original_before_native_effects() {
    let addr = address();
    let accounts = bank();
    let owner = consumer_instance();
    let client = client(
        write_description(addr),
        Arc::new(AtomicBool::new(false)),
        accounts.clone(),
        &owner,
    );
    let retained = ResourceKind::ALL.map(|k| accounts.account(k).used());
    for input in invalid_inputs().into_iter().map(|b| payload(&b)).chain([
        Payload::new(VALUES, "text/plain"),
        {
            let mut p = payload(VALUES);
            p.content_coding = Some("gzip".into());
            p
        },
    ]) {
        let original = input.clone();
        let error = client
            .write_multiple_properties(input, Deadline::NONE, now(0), &mut work())
            .err()
            .unwrap();
        assert!(Arc::ptr_eq(&original.body, &error.into_input().body));
        assert_eq!(
            retained,
            ResourceKind::ALL.map(|k| accounts.account(k).used())
        );
    }
    for class in [
        WorkClass::CodecInputBytes,
        WorkClass::ProviderProbes,
        WorkClass::BindingPolls,
        WorkClass::CleanupItems,
    ] {
        let input = payload(VALUES);
        let original = input.clone();
        let error = client
            .write_multiple_properties(
                input,
                Deadline::NONE,
                now(0),
                &mut work().with_remaining(class, 0),
            )
            .err()
            .unwrap();
        assert!(Arc::ptr_eq(&original.body, &error.into_input().body));
        assert_eq!(
            retained,
            ResourceKind::ALL.map(|k| accounts.account(k).used())
        );
    }
    io().block_on(async {
        let first = client
            .write_multiple_properties(
                payload(br#"{"on":false}"#),
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap_or_else(|_| panic!());
        let input = payload(VALUES);
        let original = input.clone();
        let error = client
            .write_multiple_properties(input, Deadline::NONE, now(0), &mut work())
            .err()
            .unwrap();
        assert!(
            Arc::ptr_eq(&original.body, &error.into_input().body),
            "native admission refusal retains a selected-write input"
        );
        drop(first);
    });
    drop(client);
    clean(&accounts);
    // Nothing acquired a listener or attempted a per-Property fallback.
    drop(TcpListener::bind(addr).unwrap());
}

#[test]
fn consumer_and_producer_member_permission_refuse_the_entire_group_before_effects() {
    struct AccessMembers;
    impl ConsumerAccessService for AccessMembers {
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
            Ok(())
        }
        fn authorize_property_write(
            &self,
            _: &ConsumerInstanceRef,
            _: BindingRegistrationIdentity,
            _: &str,
            _: Operation,
            (schema, selected): (
                &collection::PropertyCollectionSchema,
                collection::PropertySelection,
            ),
        ) -> CoreResult<()> {
            if schema
                .members()
                .iter()
                .enumerate()
                .any(|(i, m)| selected.contains(i) && m.name.as_ref() == "secret")
            {
                Err(denied())
            } else {
                Ok(())
            }
        }
        fn retained_bytes(&self) -> u64 {
            32
        }
        fn work(&self) -> u64 {
            4096
        }
    }
    io().block_on(async {
        let addr = address();
        let td = write_description(addr);
        let accounts = bank();
        let owner = consumer_instance();
        let mut reg = registration();
        reg.services.access = Arc::new(AccessMembers);
        let p = prepare_consumer(
            &td,
            accounts.clone(),
            vec![reg],
            ConsumerSelector::default(),
        )
        .unwrap();
        let (d, f) = p.into_execution();
        let client = ConsumerRuntime::try_new(owner.reference(), d, f, accounts.clone(), 2)
            .unwrap_or_else(|_| panic!());
        let mut server = Fixture::new(td);
        assert!(
            client
                .write_multiple_properties(payload(VALUES), Deadline::NONE, now(0), &mut work())
                .is_err()
        );
        assert_eq!(server.controls.requests.get(), 0);
        assert!(server.app.applied.borrow().is_null());
        let mut allowed = client
            .write_multiple_properties(
                payload(br#"{"on":false}"#),
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap_or_else(|_| panic!());
        let (local, _) = server.exchange(&mut allowed).await;
        assert!(local.result().is_ok());
        drop(local);
        drop(allowed);
        drop(client);
        clean(&accounts);
        server.close();

        let addr = address();
        let td = write_description(addr);
        let mut server = Fixture::new(td.clone());
        *server.app.denied_member.borrow_mut() = Some("secret".into());
        let client = crate::client(
            td,
            Arc::new(AtomicBool::new(false)),
            accounts.clone(),
            &owner,
        );
        let mut denied_call = start_write(&client);
        let (local, remote) = server.exchange(&mut denied_call).await;
        assert!(local.result().is_err());
        assert_eq!(remote.effect, FiniteEffect::NotInvoked);
        assert_eq!(server.app.roots.get(), 0);
        assert!(
            server.app.applied.borrow().is_null(),
            "an earlier authorized member cannot be applied"
        );
        drop(local);
        drop(denied_call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn selected_write_requires_its_own_data_facet_and_prepares_all_eligible_schemas() {
    struct AllMapOnly(Arc<dyn ConsumerDataService>);
    impl ConsumerDataService for AllMapOnly {
        fn content_type(&self) -> &str {
            self.0.content_type()
        }
        fn policy(&self) -> ConsumerDataPolicy {
            self.0.policy()
        }
        fn validate(&self, s: value_type::TypeSchema, p: &Payload) -> CoreResult<()> {
            self.0.validate(s, p)
        }
        fn supports_property_map(&self, s: &collection::PropertyCollectionSchema) -> bool {
            self.0.supports_property_map(s)
        }
        fn validate_property_map(
            &self,
            s: &collection::PropertyCollectionSchema,
            p: &Payload,
        ) -> CoreResult<()> {
            self.0.validate_property_map(s, p)
        }
    }
    let addr = address();
    let mut td = write_description(addr);
    // Unsupported individual native alternatives do not prevent Consumer root selection.
    td["properties"]["on"]["forms"][0]["href"] = json!("unknown:on");
    let accounts = bank();
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let (d, f) = p.into_execution();
    assert_eq!(d.iter().filter(|d| d.operation() == WRITE).count(), 1);
    let (input, _, _) = d
        .into_iter()
        .find(|d| d.operation() == WRITE)
        .unwrap()
        .into_parts();
    let ConsumerFiniteSchemas::WriteMultipleProperties(s) = &input.schemas else {
        panic!()
    };
    assert_eq!(
        s.members()
            .iter()
            .map(|m| m.name.as_ref())
            .collect::<Vec<_>>(),
        ["on", "secret"]
    );
    let selected = input
        .schemas
        .validate_input_selection(
            input.services.data.as_ref(),
            Some(&payload(br#"{"secret":"x"}"#)),
        )
        .unwrap()
        .unwrap();
    assert_eq!(selected, s.select(&["secret"]).unwrap());
    drop(input);
    drop(f);
    clean(&accounts);
    // Even an unselected writable member's required schema must be supported at preparation.
    td["properties"]["secret"]["minLength"] = json!(1);
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    assert!(
        !p.support()
            .iter()
            .any(|s| s.selected && s.operation == WRITE)
    );
    drop(p);
    clean(&accounts);
    td["properties"]["secret"]
        .as_object_mut()
        .unwrap()
        .remove("minLength");
    let mut reg = registration();
    reg.services.data = Arc::new(AllMapOnly(reg.services.data));
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![reg],
        ConsumerSelector::default(),
    )
    .unwrap();
    assert!(
        !p.support()
            .iter()
            .any(|s| s.selected && s.operation == WRITE),
        "all-map support cannot qualify selected-map validation"
    );
    drop(p);
    clean(&accounts);
}

#[test]
fn a_multi_operation_root_form_uses_independent_http_defaults_on_one_listener() {
    io().block_on(async {
        let addr = address();
        let mut td = write_description(addr);
        td["forms"][0]["op"] = json!(["readallproperties", "writemultipleproperties"]);
        let mut server = Fixture::new(td.clone());
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
        assert!(local.result().is_ok());
        assert_eq!(remote.operation, Some(WRITE));
        drop(local);
        drop(call);
        let mut read = start_read(&client);
        let (local, remote) = server.exchange(&mut read).await;
        assert!(local.result().is_ok());
        assert_eq!(remote.operation, Some(Operation::ReadAllProperties));
        assert_eq!(server.controls.requests.get(), 2);
        assert_eq!(server.app.roots.get(), 2);
        drop(local);
        drop(read);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn http_refuses_an_unencodable_member_report_instead_of_replacing_it_with_204() {
    io().block_on(async {
        let addr = address();
        let td = write_description(addr);
        let mut server = Fixture::new(td.clone());
        let schema = collection::PropertyCollectionSchema::try_new(
            ["on", "secret"]
                .into_iter()
                .map(|name| collection::PropertyMemberSchema {
                    name: name.into(),
                    schema: value_type::TypeSchema::Any,
                })
                .collect(),
        )
        .unwrap();
        server.app.write_report.set(Some(
            collection::PropertyWriteReport::try_new(
                schema.all(),
                3,
                0,
                collection::WriteAtomicity::Unspecified,
            )
            .unwrap(),
        ));
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
        assert_eq!(server.app.roots.get(), 1);
        assert_eq!(server.controls.requests.get(), 1);
        drop(local);
        drop(call);
        drop(client);
        clean(&accounts);
        server.close();
    });
}

#[test]
fn raw_http_delivery_also_refuses_to_erase_member_evidence() {
    let addr = address();
    let auth = authority(Generation::INITIAL);
    let key = auth.key();
    let listen = HttpProducerConfig::for_loopback(addr).unwrap();
    let provider =
        HttpFiniteProducer::try_new(listen, BindingId::new(97), BindingGeneration::INITIAL, 1000)
            .unwrap();
    let prepared = prepare_thing(
        &write_description(addr).to_string(),
        key,
        GatewayDefaultV1::LIMITS,
        &mut work(),
        &[&provider],
        |_, _| true,
    )
    .unwrap();
    let program = prepared
        .operations()
        .iter()
        .find(|o| o.site == FormSite::Thing)
        .unwrap()
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let config = HttpFiniteConfig::try_new(listen, program, key, AffordanceTarget::Thing, 1000)
        .unwrap_or_else(|_| panic!());
    let mut native = HttpFiniteBinding::new(config, BindingId::new(97), BindingGeneration::INITIAL);
    assert!(matches!(native.poll_start(&mut cx()), Poll::Ready(Ok(()))));
    let mut peer = TcpStream::connect(addr).unwrap();
    write!(peer,"PUT /properties HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",VALUES.len()).unwrap();
    peer.write_all(VALUES).unwrap();
    peer.shutdown(std::net::Shutdown::Write).unwrap();
    let start = Instant::now();
    let ingress = loop {
        if let Poll::Ready(r) = native.poll_receive(&mut cx(), now(0)) {
            break r.unwrap();
        }
        assert!(start.elapsed() < Duration::from_secs(3));
    };
    let schema = collection::PropertyCollectionSchema::try_new(
        ["on", "secret"]
            .into_iter()
            .map(|name| collection::PropertyMemberSchema {
                name: name.into(),
                schema: value_type::TypeSchema::Any,
            })
            .collect(),
    )
    .unwrap();
    let report = collection::PropertyWriteReport::try_new(
        schema.all(),
        1,
        2,
        collection::WriteAtomicity::NonAtomic,
    )
    .unwrap();
    let reply = RawFiniteReply {
        ingress,
        result: Ok(InteractionOutput::empty()
            .try_with_metadata(
                InteractionOutputMetadata::default().with_property_write_report(report),
            )
            .unwrap()),
        effect: FiniteEffect::Reported,
        recipient: Some(key),
        operation: Some(WRITE),
    };
    assert!(matches!(
        native.poll_send(&mut cx(), &reply),
        Poll::Ready(Err(_))
    ));
    assert_eq!(
        reply
            .result
            .as_ref()
            .unwrap()
            .metadata()
            .property_write_report(),
        Some(&report)
    );
    assert!(matches!(
        native.poll_close(&mut cx(), None, Some(&reply), None),
        Poll::Ready(Ok(FiniteDelivery::NotDelivered))
    ));
    drop(native);
    drop(peer);
    drop(TcpListener::bind(addr).unwrap());
}

#[test]
fn consumer_cancellation_replacement_timeout_abandon_and_policy_loss_keep_original_effects() {
    verify_collection_lifecycle(WRITE);
}
#[test]
fn producer_replacement_never_redirects_the_group() {
    verify_collection_replacement(WRITE);
}
#[test]
fn prepublication_and_lost_services_cannot_dispatch_and_keep_retirement_funded() {
    verify_collection_publication(WRITE);
}
#[test]
fn fresh_group_authorization_precedes_effects() {
    verify_collection_authorization(WRITE);
}
#[test]
fn temporary_and_retained_schema_cleanup_is_prepaid() {
    verify_collection_cleanup(WRITE);
}
#[test]
fn changed_validation_work_fences_results_after_effects() {
    verify_collection_validator_bound(WRITE);
}

#[test]
fn untrusted_producer_input_is_fully_validated_before_group_authorization_or_effects() {
    shared_untrusted_producer_input_is_fully_validated_before_group_authorization_or_effects(
        WRITE,
        invalid_inputs(),
    );
}

#[test]
fn failure_before_during_and_after_application_never_implies_rollback_or_member_success() {
    shared_failure_before_during_and_after_application_never_implies_rollback_or_member_success(
        WRITE,
    );
}

#[test]
fn ambiguous_or_partial_native_completion_never_confirms_or_replays_the_group() {
    shared_ambiguous_or_partial_native_completion_never_confirms_or_replays_the_group(WRITE);
}

#[test]
fn complete_producer_qualification_refuses_missing_root_behavior_and_unsupported_advertisement() {
    shared_complete_producer_qualification_refuses_missing_root_behavior_and_unsupported_advertisement(WRITE);
}

#[test]
fn valid_head_without_eof_is_not_completion_and_retains_funding_until_settlement() {
    shared_valid_head_without_eof_is_not_completion_and_retains_funding_until_settlement(WRITE);
}

#[test]
fn unread_confirmed_completion_keeps_original_evidence_when_local_handoff_is_revoked() {
    shared_unread_confirmed_completion_keeps_original_evidence_when_local_handoff_is_revoked(WRITE);
}

#[test]
fn final_qualification_requires_original_writable_members_and_actual_root_native_coverage() {
    shared_final_qualification_requires_original_writable_members_and_actual_root_native_coverage(
        WRITE,
    );
}
