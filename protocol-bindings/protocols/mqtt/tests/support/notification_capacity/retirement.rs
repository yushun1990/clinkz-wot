//! Cleanup against the original publication's profile and resources.
use super::*;

fn policy(quantum: u64) -> Arc<ResourceLimits> {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(K::CleanupWorkItemsPerStepMax, Some(quantum)));
    Arc::new(limits)
}

#[test]
fn four_credit_turns_retire_an_event_even_when_the_registry_is_full() {
    runtime().block_on(async {
        let broker = Broker::start();
        let (provider, service) = mqtt_service(&broker);
        let sibling_td = support::td(broker.address, "other/alarm").to_string();
        let sibling = support::instance(authority(1), &sibling_td);
        let sibling_key = sibling.key();
        let mut owner = RawProducerServient::try_new(
            vec![sibling],
            RawProducerRegistryLimits {
                instances: 2,
                behaviors_per_instance: 1,
                target_name_bytes: 64,
            },
        )
        .ok()
        .unwrap();
        let input = event_input(
            &owner,
            event_instance(broker.address, 1, false),
            &provider,
            &service,
            1,
            &mut budget(),
        )
        .unwrap();
        let gate = input.participants[0].ingress_gate().clone();
        let small_bank = Arc::new(HostResourceAccounts::new(&policy(3)));
        let refused = ComposedProducerExposure::try_new(
            &owner,
            input,
            policy(3),
            small_bank.clone(),
            &mut budget(),
        )
        .err()
        .unwrap();
        assert_eq!(
            refused.error().error().limit_details(),
            Some((K::CleanupWorkItemsPerStepMax, 3, Some(4), None))
        );
        assert!(gate.is_staged() && !service.connected());
        clean(&small_bank);
        let accounts = Arc::new(HostResourceAccounts::new(&policy(4)));
        let mut exposure = ComposedProducerExposure::try_new(
            &owner,
            refused.into_input(),
            policy(4),
            accounts.clone(),
            &mut budget(),
        )
        .unwrap_or_else(|r| panic!("{:?}", r.error().error()));
        publish_members(&mut owner, &mut exposure).await;
        exposure.begin_close();
        assert!(
            owner
                .step_notifications(support::native(&mut exposure), &mut cx(), &mut cleanup(4))
                .unwrap()
                .is_ready()
        );
        assert!(!service.connected());
        let mut first = cleanup(4);
        assert!(matches!(
            exposure.retire(&mut owner, &mut first),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(first.remaining(WorkClass::CleanupItems), 0);
        exposure.retire(&mut owner, &mut cleanup(4)).unwrap();
        assert_eq!(owner.description(sibling_key), Some(sibling_td.as_str()));
        drop(exposure);
        clean(&accounts);
    });
}

#[test]
fn unrelated_action_entries_do_not_charge_an_orphaned_notification_thing() {
    runtime().block_on(async {
        let broker = Broker::start();
        let (provider, service) = mqtt_service(&broker);
        let authority = authority(1);
        let key = authority.key();
        let app = Rc::new(App::default());
        let action =
            RawProducerInstance::new(authority, vec![]).with_actions(vec![RawActionBehavior::new(
                "hold".into(),
                Action(app.clone()),
                32,
                false,
                false,
            )]);
        let mut owner = RawProducerServient::try_new(
            vec![action],
            RawProducerRegistryLimits {
                instances: 2,
                behaviors_per_instance: 32,
                target_name_bytes: 64,
            },
        )
        .ok()
        .unwrap();
        owner
            .enable_raw_actions(RawActionLimits {
                invocations: 128,
                application_work_bytes: 8192,
                input_fields: 8,
                input_bytes: 512,
                principal_id_bytes: 32,
                output_body_bytes: 64,
                output_media_capacity: 64,
                output_retained_bytes: 2048,
            })
            .unwrap();
        let target = AffordanceTarget::Action("hold".into());
        let input = InteractionInput::empty();
        let handles: Vec<_> = (0..128)
            .map(|_| {
                owner
                    .invoke_raw_action(
                        RawActionRequest {
                            instance: key,
                            target: &target,
                            input: &input,
                            invoke: ActionRoute {
                                binding: (BindingId::new(2), BindingGeneration::INITIAL),
                                mapping: ActionMappingKey(1),
                            },
                            query: None,
                            cancel: None,
                        },
                        &mut budget(),
                    )
                    .unwrap()
            })
            .collect();
        let accounts = publication_bank(32, 32);
        let input = event_input(
            &owner,
            event_instance(broker.address, 32, false),
            &provider,
            &service,
            1,
            &mut budget(),
        )
        .unwrap();
        let mut exposure = support::stage(&owner, input, &accounts);
        publish_members(&mut owner, &mut exposure).await;
        let orphan = exposure.key();
        drop(exposure);
        assert!(!service.connected());
        let mut turn = cleanup(gateway_cleanup());
        owner.retire_revoked_publication(orphan, &mut turn).unwrap();
        // Two registry positions, 32 original identities, 32 routes, one removal.
        assert_eq!(
            gateway_cleanup() - turn.remaining(WorkClass::CleanupItems),
            67
        );
        clean(&accounts);
        assert_eq!(app.cancels.get(), 0);
        app.complete.set(true);
        for handle in handles {
            assert!(
                owner
                    .step_raw_action(&handle, &mut budget())
                    .unwrap()
                    .is_ready()
            );
            assert_eq!(
                owner
                    .take_raw_action_settlement(&handle)
                    .unwrap()
                    .unwrap()
                    .instance,
                key
            );
        }
    });
}

fn many_forms(
    owner: &RawProducerServient,
    broker: &Broker,
    provider: &MqttNotificationProducer,
    service: &MqttNotificationService,
) -> ComposedProducerInput {
    let mut td = support::td(broker.address, "unused");
    let mut events = serde_json::Map::new();
    let mut sources = Vec::new();
    for i in 1..=9 {
        let name = format!("event{i:02}");
        let forms: Vec<_> = (0..if i == 9 {25} else {29}).map(|form| serde_json::json!({
            "href": format!("mqtt://{}", broker.address), "mqv:filter": format!("r/{i}/{form}")
        })).collect();
        events.insert(
            name.clone(),
            serde_json::json!({"data": {"type":"integer"}, "forms":forms}),
        );
        sources.push(ProducerNotificationSource::event(name));
    }
    td["events"] = events.into();
    let instance = RawProducerInstance::new(authority(2), vec![])
        .with_description(Arc::from(td.to_string()))
        .with_notification_sources(sources, &mut budget())
        .unwrap_or_else(|r| panic!("{:?}", r.error().error()));
    let mut work = WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 4_000_000));
    let prepared = prepare_thing(instance.description().unwrap(), instance.key(), GatewayDefaultV1::LIMITS, &mut work,
        &[provider], |scope, op| matches!(scope, ValidatedProducerScope::Event(name) if instance.supports_event(name,op))).unwrap();
    assert_eq!(
        (
            prepared.notification_mapping_count(),
            prepared.operations().len()
        ),
        (257, 514)
    );
    let mut groups = vec![Vec::new(); 29];
    for op in prepared.operations() {
        groups[op.form as usize].push(
            op.definition::<MqttNotificationDefinition>()
                .unwrap()
                .clone(),
        );
    }
    let participants = groups
        .into_iter()
        .map(|mut defs| {
            OwnedProducerParticipation::notifications(
                service
                    .participation(
                        &owner.participation_token(),
                        instance.key(),
                        &mut defs,
                        &mut budget(),
                    )
                    .unwrap(),
            )
        })
        .collect();
    ComposedProducerInput {
        instance,
        prepared,
        providers: ProducerRegistrations::new(vec![Rc::new(provider.clone())]),
        participants,
        expected: None,
        handler_backing_bytes: 0,
    }
}

#[test]
fn successor_refuses_before_setup_if_the_serving_incumbent_exceeds_the_cutover_quantum() {
    runtime().block_on(async {
        for keep_serving in [true, false] {
            let broker = Broker::start();
            let (provider, service) = mqtt_service(&broker);
            let mut owner = events_owner(9);
            let accounts = bank();
            let mut setup = WorkClass::ALL
                .into_iter()
                .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 8_000_000));
            let mut old = ComposedProducerExposure::try_new(
                &owner,
                many_forms(&owner, &broker, &provider, &service),
                limits(),
                accounts.clone(),
                &mut setup,
            )
            .unwrap_or_else(|r| panic!("{:?}", r.error().error()));
            publish_members(&mut owner, &mut old).await;
            let key = old.key();
            let input = event_input(
                &owner,
                event_instance(broker.address, 1, false),
                &provider,
                &service,
                1,
                &mut budget(),
            )
            .unwrap();
            let mut input = ComposedProducerInput {
                expected: Some(key),
                ..input
            };
            let successor = input.instance.key();
            let gate = input.participants[0].ingress_gate().clone();
            let next_bank = bank();
            let refused = ComposedProducerExposure::try_new(
                &owner,
                input,
                limits(),
                next_bank.clone(),
                &mut budget(),
            )
            .err()
            .unwrap();
            assert_eq!(
                refused.error().error().limit_details(),
                Some((K::CleanupWorkItemsPerStepMax, 256, Some(257), None))
            );
            assert_eq!(
                refused.error().error().context().thing(),
                Some(successor.slot())
            );
            input = refused.into_input();
            assert!(Rc::ptr_eq(input.participants[0].ingress_gate(), &gate) && gate.is_staged());
            assert!(owner.description(successor).is_none());
            assert_eq!(accounts.account(K::BindingRoutesGlobalMax).used(), 257);
            clean(&next_bank);
            let mut peer = Peer::new(&broker);
            peer.subscribe("r/1/0");
            let mut sample = owner
                .publish_notification(
                    support::native(&mut old),
                    key,
                    AffordanceTarget::Event("event01".into()),
                    data(b"51"),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            finish(&mut sample).await;
            peer.event("r/1/0", b"51");
            sample.acknowledge().ok().unwrap();
            let allowed = if keep_serving {
                257
            } else {
                old.begin_close();
                for index in 0..29 {
                    let member = old
                        .participant_mut::<MqttNotificationRuntime>(index)
                        .unwrap();
                    assert!(
                        owner
                            .step_notifications(member, &mut cx(), &mut cleanup(gateway_cleanup()))
                            .unwrap()
                            .is_ready()
                    );
                }
                // Fence the original incarnation and drain exactly one route under
                // its own policy. The returned successor can now use Gateway too.
                let mut drain_one = cleanup(29 + 1 + 1);
                assert!(matches!(
                    old.retire(&mut owner, &mut drain_one),
                    Err(CoreError::Backpressure(_))
                ));
                assert_eq!(drain_one.remaining(WorkClass::CleanupItems), 0);
                gateway_cleanup()
            };
            let next_bank = Arc::new(HostResourceAccounts::new(&policy(allowed)));
            let mut next = ComposedProducerExposure::try_new(
                &owner,
                input,
                policy(allowed),
                next_bank.clone(),
                &mut budget(),
            )
            .unwrap_or_else(|r| panic!("{:?}", r.error().error()));
            next.step(&mut owner, &mut cx(), now(), |_, _| budget())
                .unwrap();
            assert!(matches!(
                next.publish(
                    &mut owner,
                    &mut budget().with_remaining(WorkClass::CleanupItems, allowed - 1)
                ),
                Err(CoreError::Backpressure(_))
            ));
            assert!(gate.is_staged());
            assert_eq!(owner.description(key).is_some(), keep_serving);
            let mut cutover = budget().with_remaining(WorkClass::CleanupItems, allowed);
            next.publish(&mut owner, &mut cutover).unwrap();
            assert_eq!(cutover.remaining(WorkClass::CleanupItems), 0);
            assert!(
                gate.is_open()
                    && owner.description(key).is_none()
                    && owner.description(successor).is_some()
            );
            drop((old, next));
            owner
                .retire_revoked_publication(successor, &mut cleanup(257))
                .unwrap();
            clean(&accounts);
            clean(&next_bank);
        }
    });
}

#[test]
fn the_original_instance_prepays_its_sources_through_retirement_and_displaced_settlement() {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum LastOwner {
        Publication,
        Registry,
        Settlement,
    }
    runtime().block_on(async {
        for count in [1, 86] {
            for last_owner in [
                LastOwner::Publication,
                LastOwner::Registry,
                LastOwner::Settlement,
            ] {
                let broker = Broker::start();
                let (provider, service) = mqtt_service(&broker);
                let mut owner = events_owner(count);
                let accounts = publication_bank(count as u64 + 1, 2);
                let base = RawProducerInstance::new(authority(2), vec![])
                    .with_description(Arc::from(event_td(broker.address, count, false)));
                let key = base.key();
                let names: Vec<Arc<str>> =
                    (1..=count).map(|i| format!("event{i:02}").into()).collect();
                let original_names: Vec<_> = names.iter().map(Arc::downgrade).collect();
                let sources = names
                    .into_iter()
                    .map(ProducerNotificationSource::event)
                    .collect();
                let mut short = cleanup(count as u64 - 1);
                let refusal = base
                    .with_notification_sources(sources, &mut short)
                    .err()
                    .unwrap();
                assert!(matches!(
                    refusal.error().error(),
                    CoreError::Backpressure(_)
                ));
                assert_eq!(short.remaining(WorkClass::CleanupItems), count as u64 - 1);
                let (instance, sources) = refusal.into_input();
                assert_eq!(instance.key(), key);
                assert_eq!(sources.len(), count);
                let mut lifetime = cleanup(count as u64);
                let instance = instance
                    .with_notification_sources(sources, &mut lifetime)
                    .unwrap_or_else(|r| panic!("{:?}", r.error().error()));
                assert_eq!(lifetime.remaining(WorkClass::CleanupItems), 0);
                let input =
                    event_input(&owner, instance, &provider, &service, count, &mut budget())
                        .unwrap();
                let mut original = support::stage(&owner, input, &accounts);
                publish_members(&mut owner, &mut original).await;
                if last_owner == LastOwner::Settlement {
                    let mut emission = owner
                        .publish_notification(
                            support::native(&mut original),
                            key,
                            AffordanceTarget::Event(format!("event{count:02}").into()),
                            data(br#""terminal""#),
                            Deadline::NONE,
                            now(),
                            &mut budget(),
                        )
                        .ok()
                        .unwrap();
                    finish(&mut emission).await;
                    let mut next = event_input(
                        &owner,
                        event_instance(broker.address, 1, false),
                        &provider,
                        &service,
                        1,
                        &mut budget(),
                    )
                    .unwrap();
                    next.expected = Some(key);
                    let mut successor = support::stage(&owner, next, &accounts);
                    publish_members(&mut owner, &mut successor).await;
                    drop(original);
                    assert!(original_names.iter().all(|name| name.strong_count() != 0));
                    assert_eq!(
                        accounts.account(K::BindingRoutesGlobalMax).used(),
                        count as u64 + 1
                    );
                    assert_eq!(emission.acknowledge().ok().unwrap().instance, key);
                    assert_eq!(accounts.account(K::BindingRoutesGlobalMax).used(), 1);
                    let new_key = successor.key();
                    drop(successor);
                    owner
                        .retire_revoked_publication(new_key, &mut cleanup(gateway_cleanup()))
                        .unwrap();
                } else {
                    original.begin_close();
                    let turns = (3 * count as u64 + 1).div_ceil(gateway_cleanup());
                    for turn in 1..=turns {
                        let progress = owner
                            .step_notifications(
                                support::native(&mut original),
                                &mut cx(),
                                &mut cleanup(gateway_cleanup()),
                            )
                            .unwrap();
                        assert_eq!(progress.is_ready(), turn == turns);
                    }
                    assert!(original_names.iter().all(|name| name.strong_count() != 0));
                    // One lookup, one membership check and all but the final route.
                    let mut prefix = cleanup(count as u64 + 1);
                    if last_owner == LastOwner::Registry {
                        drop(original);
                        assert!(owner.retire_revoked_publication(key, &mut prefix).is_err());
                        assert!(original_names.iter().all(|name| name.strong_count() != 0));
                        owner
                            .retire_revoked_publication(key, &mut cleanup(4))
                            .unwrap();
                    } else {
                        assert!(original.retire(&mut owner, &mut prefix).is_err());
                        assert!(original_names.iter().all(|name| name.strong_count() != 0));
                        original.retire(&mut owner, &mut cleanup(4)).unwrap();
                        drop(original);
                    }
                }
                assert!(original_names.iter().all(|name| name.strong_count() == 0));
                clean(&accounts);
            }
        }
    });
}
