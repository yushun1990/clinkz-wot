#![cfg(feature = "prepare")]
//! One real native write and one root behavior; effects are not transactional.
include!("support/collection.rs");
include!("support/collection_write.rs");

const WRITE: Operation = Operation::WriteAllProperties;
const VALUES: &[u8] = br#"{"on":false,"secret":"updated"}"#;
fn write_description(addr: SocketAddr) -> Value {
    collection_description(addr, WRITE)
}
fn start_write(client: &(StandaloneConsumerAssociation, ConsumerCalls)) -> ConsumerCall {
    start_collection_at(client, WRITE, Deadline::NONE).unwrap()
}

#[test]
fn complete_writable_map_uses_actual_root_form_services_and_one_native_put() {
    io().block_on(async {
        for explicit in [false, true] {
            let addr = address();
            let mut td = write_description(addr);
            if explicit {
                td["forms"][0]["htv:methodName"] = json!("PUT");
                td["forms"][0]["response"] =
                    json!({"contentType":"application/json","htv:statusCodeValue":204});
            }
            td["forms"]
                .as_array_mut()
                .unwrap()
                .push(description(addr)["forms"][0].clone());
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
            let root = definitions.iter().find(|d| d.operation() == WRITE).unwrap();
            assert_eq!(root.affordance(), &AffordanceTarget::Thing);
            assert_eq!(root.form(), 0);
            assert_eq!(root.binding().binding_id(), BindingId::new(98));
            let client = StandaloneConsumerAssociation::try_new(
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
            let mut call = start_write(&client);
            let (local, remote) = server.exchange(&mut call).await;
            assert!(local.result().unwrap().data().is_none());
            assert_eq!(local.operation(), WRITE);
            assert_eq!(local.consumer(), &owner.reference());
            assert_eq!(
                local.affordance(),
                clinkz_wot_core::thing::AffordanceSite::Thing
            );
            assert_eq!(local.effect(), ConsumerEffect::Completed);
            assert_eq!(remote.recipient, Some(server.exposure.key()));
            assert_eq!(remote.operation, Some(WRITE));
            assert_eq!(remote.effect, FiniteEffect::Completed);
            assert_eq!(remote.delivery, FiniteDelivery::Delivered);
            assert_eq!(
                *server.app.applied.borrow(),
                json!({"on":false,"secret":"updated"})
            );
            assert_eq!(server.controls.requests.get(), 1);
            assert_eq!(server.app.roots.get(), 1);
            assert_eq!((server.app.reads.get(), server.app.writes.get()), (0, 0));
            drop(local);
            drop(call);
            // Individual write and all-read remain distinct on this listener.
            let mut single = client
                .0
                .write_property(
                    &client.1,
                    "on",
                    payload(b"true"),
                    Deadline::NONE,
                    now(0),
                    &mut work(),
                )
                .unwrap_or_else(|_| panic!());
            let (local, remote) = server.exchange(&mut single).await;
            assert!(local.result().is_ok());
            assert_eq!(remote.operation, Some(Operation::WriteProperty));
            drop(local);
            drop(single);
            let mut read = start_read(&client);
            let (local, _) = server.exchange(&mut read).await;
            assert_eq!(
                serde_json::from_slice::<Value>(&local.result().unwrap().data().unwrap().body)
                    .unwrap(),
                json!({"level":7,"on":true})
            );
            assert_eq!(server.app.writes.get(), 1);
            assert_eq!(server.app.roots.get(), 2);
            drop(local);
            drop(read);
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}

fn invalid_inputs() -> Vec<Vec<u8>> {
    [
        b"null".as_slice(),
        b"[]",
        b"{}",
        br#"{"on":false}"#,
        br#"{"secret":"updated"}"#,
        br#"{"on":false,"secret":"updated","level":8}"#,
        br#"{"on":false,"secret":"updated","unknown":0}"#,
        br#"{"on":0,"secret":"updated"}"#,
        br#"{"on":false,"secret":7}"#,
        br#"{"on":false,"on":true,"secret":"updated"}"#,
        br#"{"on":false,"\u006fn":true,"secret":"updated"}"#,
        br#"{"on":false,"secret":"\uD800"}"#,
        br#"{"on":false,"secret":"updated"} {}"#,
        br#"{"on":false,"secret":"updated",}"#,
    ]
    .into_iter()
    .map(Vec::from)
    .chain([vec![b' '; 4097]])
    .collect()
}

#[test]
fn invalid_missing_duplicate_readonly_and_unpaid_input_returns_original_without_io() {
    io().block_on(async {
        let addr = address();
        let td = write_description(addr);
        let mut server = Fixture::new(td.clone());
        let accounts = bank();
        let owner = consumer_instance();
        let denied = Arc::new(AtomicBool::new(false));
        let client = client(td, denied.clone(), accounts.clone(), &owner);
        let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
        let mut inputs: Vec<_> = invalid_inputs().iter().map(|b| payload(b)).collect();
        inputs.push(Payload::new(VALUES, "text/plain"));
        inputs.push(payload(VALUES).with_content_coding("gzip"));
        for input in inputs {
            let rejected = client
                .0
                .write_all_properties(
                    &client.1,
                    input.clone(),
                    Deadline::NONE,
                    now(0),
                    &mut work(),
                )
                .err()
                .unwrap()
                .into_input();
            assert!(Arc::ptr_eq(&input.body, &rejected.body));
            assert_eq!(
                baseline,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
        }
        for class in [
            WorkClass::CodecInputBytes,
            WorkClass::ProviderProbes,
            WorkClass::PlanningItems,
            WorkClass::CleanupItems,
            WorkClass::BindingPolls,
        ] {
            let input = payload(VALUES);
            let rejected = client
                .0
                .write_all_properties(
                    &client.1,
                    input.clone(),
                    Deadline::NONE,
                    now(0),
                    &mut work().with_remaining(class, 0),
                )
                .err()
                .unwrap()
                .into_input();
            assert!(Arc::ptr_eq(&input.body, &rejected.body));
            assert_eq!(
                baseline,
                ResourceKind::ALL.map(|k| accounts.account(k).used())
            );
        }
        denied.store(true, Ordering::Relaxed);
        assert!(
            client
                .0
                .write_all_properties(
                    &client.1,
                    payload(VALUES),
                    Deadline::NONE,
                    now(0),
                    &mut work()
                )
                .is_err()
        );
        server.step();
        assert_eq!(server.controls.requests.get(), 0);
        assert_eq!(server.app.roots.get(), 0);
        assert!(server.app.applied.borrow().is_null());
        drop(client);
        clean(&accounts);
        server.close();
    });
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
fn consumer_cancellation_replacement_timeout_abandon_and_policy_loss_retain_original_effects() {
    verify_collection_lifecycle(WRITE);
}
#[test]
fn producer_replacement_before_and_after_dispatch_never_redirects_the_write() {
    verify_collection_replacement(WRITE);
}
#[test]
fn prepublication_and_lost_validation_service_cannot_dispatch_and_retain_cleanup() {
    verify_collection_publication(WRITE);
}
#[test]
fn group_authorization_precedes_any_application_effect() {
    verify_collection_authorization(WRITE);
}
#[test]
fn collection_schema_construction_and_failed_admission_keep_prepaid_destruction() {
    verify_collection_cleanup(WRITE);
}
#[test]
fn changed_captured_validation_work_fences_success_after_effects() {
    verify_collection_validator_bound(WRITE);
}

#[test]
fn complete_producer_qualification_refuses_missing_root_behavior_and_unsupported_advertisement() {
    shared_complete_producer_qualification_refuses_missing_root_behavior_and_unsupported_advertisement(WRITE);
}

#[test]
fn writable_input_schema_is_independent_of_readable_members_and_individual_forms() {
    let addr = address();
    let mut td = write_description(addr);
    td["properties"]["level"]["minimum"] = json!(0); // Excluded from the input role.
    for name in ["on", "secret", "level"] {
        td["properties"][name]["forms"][0]["href"] = json!(format!("unknown:{name}"));
    }
    // Consumer can select a supported root alternative, never a Property fallback.
    let supported = td["forms"][0].clone();
    td["forms"] = json!([{"href":"unknown:properties","op":"writeallproperties"},supported]);
    let accounts = bank();
    let prepared = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let (definitions, funding) = prepared.into_execution();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].form(), 1);
    let (input, _, _) = definitions.into_iter().next().unwrap().into_parts();
    match input.definition.content().schemas() {
        ConsumerFiniteSchemas::WriteAllProperties(s) => assert_eq!(
            s.members()
                .iter()
                .map(|m| m.name.as_ref())
                .collect::<Vec<_>>(),
            ["on", "secret"]
        ),
        _ => panic!("writable input role required"),
    }
    assert!(
        input
            .definition
            .content()
            .schemas()
            .validate_output(input.definition.data(), None, None)
            .is_ok()
    );
    assert!(
        input
            .definition
            .content()
            .schemas()
            .validate_input(input.definition.data(), Some(&payload(VALUES)))
            .is_ok()
    );
    assert!(
        input
            .definition
            .content()
            .schemas()
            .validate_input(
                input.definition.data(),
                Some(&payload(br#"{"on":false,"secret":"updated","level":1}"#))
            )
            .is_err()
    );
    drop(input);
    drop(funding);
    clean(&accounts);

    td["forms"] = json!([supported]);
    td["properties"]["on"]["readOnly"] = json!(true);
    td["properties"]["secret"]
        .as_object_mut()
        .unwrap()
        .remove("writeOnly");
    td["properties"]["secret"]["readOnly"] = json!(true);
    let p = prepare_consumer(
        &td,
        accounts.clone(),
        vec![registration()],
        ConsumerSelector::default(),
    )
    .unwrap();
    let (d, f) = p.into_execution();
    let (input, _, _) = d.into_iter().next().unwrap().into_parts();
    assert!(
        input
            .definition
            .content()
            .schemas()
            .validate_input(input.definition.data(), Some(&payload(b"{}")))
            .is_ok()
    );
    assert!(
        input
            .definition
            .content()
            .schemas()
            .validate_input(input.definition.data(), Some(&payload(VALUES)))
            .is_err()
    );
    drop(input);
    drop(f);
    clean(&accounts);
}

#[test]
fn captured_map_service_validates_the_whole_input_and_service_loss_fences_native_admission() {
    use std::sync::atomic::AtomicUsize;
    struct Data {
        actual: Arc<dyn ConsumerDataService>,
        calls: Arc<AtomicUsize>,
        lost: Arc<AtomicBool>,
        lose_on_validate: bool,
    }
    impl ConsumerDataService for Data {
        fn content_type(&self) -> &str {
            self.actual.content_type()
        }
        fn policy(&self) -> ConsumerDataPolicy {
            let mut p = self.actual.policy();
            p.retained_bytes += 128;
            p
        }
        fn available(&self) -> bool {
            !self.lost.load(Ordering::Relaxed)
        }
        fn validate(&self, _: &data_schema::PreparedSchema, _: &Payload) -> CoreResult<()> {
            panic!("collection input must use its member schema")
        }
        fn supports_schema(&self, _: &data_schema::PreparedSchema) -> bool {
            false
        }
        fn supports_property_map(&self, s: &collection::PropertyCollectionSchema) -> bool {
            self.actual.supports_property_map(s)
        }
        fn validate_property_map(
            &self,
            s: &collection::PropertyCollectionSchema,
            p: &Payload,
        ) -> CoreResult<()> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.actual.validate_property_map(s, p)?;
            self.lost.store(self.lose_on_validate, Ordering::Relaxed);
            Ok(())
        }
    }
    io().block_on(async {
        for lost in [false, true] {
            let addr = address();
            let td = write_description(addr);
            let mut server = Fixture::new(td.clone());
            let accounts = bank();
            let owner = consumer_instance();
            let calls = Arc::new(AtomicUsize::new(0));
            let mut reg = registration();
            reg.services.data = Arc::new(Data {
                actual: reg.services.data,
                calls: calls.clone(),
                lost: Arc::new(AtomicBool::new(false)),
                lose_on_validate: lost,
            });
            let prepared = prepare_consumer(
                &td,
                accounts.clone(),
                vec![reg],
                ConsumerSelector::default(),
            )
            .unwrap();
            let (d, f) = prepared.into_execution();
            assert_eq!(d.len(), 1);
            let client =
                StandaloneConsumerAssociation::try_new(owner.reference(), d, f, accounts.clone())
                    .map(|association| {
                        (
                            association,
                            ConsumerCalls::try_new(accounts.clone(), 1).unwrap(),
                        )
                    })
                    .unwrap_or_else(|_| panic!());
            let original = payload(VALUES);
            let call = client.0.write_all_properties(
                &client.1,
                original.clone(),
                Deadline::NONE,
                now(0),
                &mut work(),
            );
            assert_eq!(calls.load(Ordering::Relaxed), 1);
            if lost {
                assert!(Arc::ptr_eq(
                    &call.err().unwrap().into_input().body,
                    &original.body
                ));
                server.step();
                assert_eq!(server.controls.requests.get(), 0);
                assert!(server.app.applied.borrow().is_null());
            } else {
                let mut call = call.unwrap_or_else(|_| panic!());
                let (local, remote) = server.exchange(&mut call).await;
                assert!(local.result().is_ok());
                assert_eq!(remote.effect, FiniteEffect::Completed);
                assert_eq!(
                    calls.load(Ordering::Relaxed),
                    1,
                    "empty completion has no collection output schema"
                );
            }
            drop(client);
            clean(&accounts);
            server.close();
        }
    });
}

#[test]
fn valid_head_without_eof_is_not_completion_and_retains_funding_until_settlement() {
    shared_valid_head_without_eof_is_not_completion_and_retains_funding_until_settlement(WRITE);
}

#[test]
fn final_qualification_requires_original_writable_members_and_actual_root_native_coverage() {
    shared_final_qualification_requires_original_writable_members_and_actual_root_native_coverage(
        WRITE,
    );
}

#[test]
fn unread_confirmed_completion_keeps_original_evidence_when_local_handoff_is_revoked() {
    shared_unread_confirmed_completion_keeps_original_evidence_when_local_handoff_is_revoked(WRITE);
}
