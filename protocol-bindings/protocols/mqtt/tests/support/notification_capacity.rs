//! One complete TD, with either one native owner or two owners of disjoint Events.
use super::*;
use clinkz_wot_foundation::ResourceKind as K;

const EVENTS: usize = 33;

fn events_owner() -> RawProducerServient {
    RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: EVENTS,
        target_name_bytes: 64,
    })
    .unwrap()
}

fn event_input(
    owner: &RawProducerServient,
    broker: SocketAddr,
    provider: &MqttNotificationProducer,
    service: &MqttNotificationService,
    split: bool,
) -> ComposedProducerInput {
    let mut td = support::td(broker, "unused");
    let template = td["events"]["alarm"].take();
    let mut events = serde_json::Map::new();
    let mut sources = Vec::new();
    for i in 1..=EVENTS {
        let name = format!("event{i:02}");
        let mut event = template.clone();
        event["forms"][0]["mqv:filter"] = format!("capacity/{name}").into();
        if i == EVENTS {
            event["data"]["type"] = "string".into();
        }
        sources.push(ProducerNotificationSource::event(name.as_str()));
        events.insert(name, event);
    }
    td["events"] = events.into();
    let instance = RawProducerInstance::new(authority(2), vec![])
        .with_notification_sources(sources)
        .with_description(Arc::from(td.to_string()));
    let prepared = support::prepare(&instance, &[provider]).unwrap();
    assert_eq!(prepared.operations().len(), 2 * EVENTS);
    assert_eq!(prepared.notification_mapping_count(), EVENTS);
    let (last, first): (Vec<_>, Vec<_>) = prepared.operations().iter().partition(|op| {
        split && op.site.site() == clinkz_wot_core::thing::AffordanceSite::Event("event33")
    });
    let participants = [first, last]
        .into_iter()
        .filter(|ops| !ops.is_empty())
        .map(|ops| {
            let definitions = ops
                .into_iter()
                .map(|op| {
                    op.definition::<MqttNotificationDefinition>()
                        .unwrap()
                        .clone()
                })
                .collect();
            OwnedProducerParticipation::notifications(
                service
                    .participation(&owner.participation_token(), instance.key(), definitions)
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

fn emit_last(
    owner: &mut RawProducerServient,
    binding: &mut MqttNotificationRuntime,
    key: ProducerInstanceKey,
    bytes: &[u8],
    work: &mut WorkBudget,
) -> Result<ProducerNotificationPublication<MqttPublication>, BindingInputRejection<PublicationData>>
{
    owner.publish_notification(
        binding,
        key,
        AffordanceTarget::Event("event33".into()),
        data(bytes),
        Deadline::NONE,
        now(),
        work,
    )
}

fn publication_bank(mappings: u64, participants: u64) -> Arc<HostResourceAccounts> {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    for (kind, count) in [
        (K::BindingRoutesGlobalMax, mappings),
        (K::RouteReadinessTokensGlobalMax, participants),
        (K::PendingHandlerCallsGlobalMax, 1),
        (K::InFlightResponsesGlobalMax, 1),
    ] {
        assert!(limits.set(kind, Some(count)));
    }
    Arc::new(HostResourceAccounts::new(&limits))
}

async fn publish_members(owner: &mut RawProducerServient, exposure: &mut ComposedProducerExposure) {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        exposure
            .step(owner, &mut cx(), now(), |_, _| budget())
            .unwrap();
        match exposure.publish(owner, &mut budget()) {
            Ok(()) => return,
            Err(CoreError::Backpressure(_)) => (),
            Err(error) => panic!("{error:?}"),
        }
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn notification_mapping_and_required_participant_shortages_return_the_original_closed_owners() {
    runtime().block_on(async {
        let broker = Broker::start();
        let provider = MqttNotificationProducer::new(
            MqttProducerConfig::for_loopback(broker.address, 8192).unwrap(),
            BindingId::new(602),
            BindingGeneration::INITIAL,
        );
        let service = MqttNotificationService::new(&provider).unwrap();
        let mut owner = events_owner();
        let mut peer = Peer::new(&broker);
        peer.subscribe("capacity/+");
        for (mappings, split, kind, limit, needed) in [
            (32, false, K::BindingRoutesGlobalMax, 32, 33),
            (33, true, K::RouteReadinessTokensGlobalMax, 1, 2),
        ] {
            let bank = publication_bank(mappings, 1);
            let input = event_input(&owner, broker.address, &provider, &service, split);
            let key = input.instance.key();
            let gates: Vec<_> = input
                .participants
                .iter()
                .map(|p| p.ingress_gate().clone())
                .collect();
            let refusal = ComposedProducerExposure::try_new(
                &owner,
                input,
                limits(),
                bank.clone(),
                &mut budget(),
            )
            .err()
            .expect("insufficient capacity");
            assert_eq!(
                refusal.error().error().limit_details(),
                Some((kind, limit, Some(needed), None))
            );
            assert_eq!(refusal.error().error().context().thing(), Some(key.slot()));
            let mut input = refusal.into_input();
            assert_eq!(input.instance.key(), key);
            assert_eq!(input.participants.len(), gates.len());
            for (p, gate) in input.participants.iter().zip(&gates) {
                assert!(Rc::ptr_eq(p.ingress_gate(), gate) && gate.is_staged());
            }
            assert!(owner.description(key).is_none());
            let binding = input
                .participants
                .last_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<MqttNotificationRuntime>()
                .unwrap();
            assert!(emit_last(&mut owner, binding, key, br#""refused""#, &mut budget()).is_err());
            assert!(!service.connected());
            assert_eq!(service.locally_published(), 0);
            peer.absent();
            clean(&bank);
            if split {
                // Omitting the second member loses coverage of event33 even
                // with enough funding. Restore that exact owner and retry.
                let funded = publication_bank(33, 2);
                let required = input.participants.pop().unwrap();
                let missing = ComposedProducerExposure::try_new(
                    &owner,
                    input,
                    limits(),
                    funded.clone(),
                    &mut budget(),
                )
                .err()
                .unwrap();
                assert!(matches!(
                    missing.error().error(),
                    CoreError::UnsupportedOperation(_)
                ));
                clean(&funded);
                let mut input = missing.into_input();
                input.participants.push(required);
                let mut exposure = support::stage(&owner, input, &funded);
                publish_members(&mut owner, &mut exposure).await;
                assert!(gates.iter().all(|g| g.is_open()));
                let last = exposure
                    .participant_mut::<MqttNotificationRuntime>(1)
                    .unwrap();
                let mut emission =
                    emit_last(&mut owner, last, key, br#""restored""#, &mut budget())
                        .ok()
                        .unwrap();
                finish(&mut emission).await;
                peer.event("capacity/event33", br#""restored""#);
                drop(exposure);
                assert!(
                    owner
                        .retire_revoked_publication(key, &mut budget())
                        .is_err()
                );
                assert_eq!(funded.account(K::BindingRoutesGlobalMax).used(), 33);
                assert!(service.connected());
                emission.acknowledge().ok().unwrap();
                assert!(!service.connected());
                assert!(
                    owner
                        .retire_revoked_publication(
                            key,
                            &mut WorkBudget::new().with_remaining(WorkClass::CleanupItems, 32)
                        )
                        .is_err()
                );
                owner
                    .retire_revoked_publication(key, &mut budget())
                    .unwrap();
                clean(&funded);
            }
        }
    });
}

#[test]
fn full_finite_pool_allows_complete_publication_and_delivery_on_mapping_33() {
    runtime().block_on(async {
        let broker = Broker::start();
        let [http, action] = http_addresses();
        let mut owner = events_owner();
        let accounts = publication_bank(33, 1);
        let http_accounts = bank();
        let transport = Native::new(
            &owner,
            HttpProducerConfig::for_loopback(http).unwrap(),
            action,
            broker.address,
            &http_accounts,
            &accounts,
            1,
        );
        let (mut finite, definition) = transport.stage(
            &owner,
            authority(1),
            &description(http, action, broker.address, false, "held"),
            Rc::new(App::default()),
            &http_accounts,
            None,
        );
        publish_members(&mut owner, &mut finite).await;
        let until = Instant::now() + Duration::from_secs(5);
        let mut http_peer = connect(http, "held");
        while finite.participant_mut::<Finite>(0).unwrap().state() != RawFiniteState::Delivering {
            finite
                .step(&mut owner, &mut cx(), now(), |_, _| budget())
                .unwrap();
            assert!(Instant::now() < until);
        }
        assert_eq!(
            (
                transport.capacity.active_work(),
                transport.capacity.retained_work()
            ),
            (1, 1)
        );
        assert_eq!(accounts.account(K::PendingHandlerCallsGlobalMax).used(), 1);
        let input = event_input(
            &owner,
            broker.address,
            &transport.mqtt,
            &transport.service,
            false,
        );
        assert_eq!(input.participants.len(), 1);
        let td = input.instance.description().unwrap().to_owned();
        let mut events = support::stage(&owner, input, &accounts);
        let key = events.key();
        let mut intended = Peer::new(&broker);
        intended.subscribe("capacity/event33");
        let mut other = Peer::new(&broker);
        other.subscribe("capacity/event32");
        broker::publish(&mut owner, &mut events).await;
        assert_eq!(owner.description(key), Some(td.as_str()));
        assert_eq!(accounts.account(K::BindingRoutesGlobalMax).used(), 33);
        assert_eq!(accounts.account(K::RouteReadinessTokensGlobalMax).used(), 1);
        let binding = support::native(&mut events);
        let short = emit_last(
            &mut owner,
            binding,
            key,
            br#""short""#,
            &mut budget().with_remaining(WorkClass::HandlerSteps, 32),
        )
        .err()
        .expect("short route lookup credit");
        assert!(matches!(short.error().error(), CoreError::Backpressure(_)));
        assert!(emit_last(&mut owner, binding, key, b"33", &mut budget()).is_err());
        assert_eq!(transport.service.locally_published(), 0);
        let mut emission = emit_last(&mut owner, binding, key, br#""last""#, &mut budget())
            .ok()
            .unwrap();
        finish(&mut emission).await;
        intended.event("capacity/event33", br#""last""#);
        other.absent();
        assert_eq!(
            (
                transport.capacity.active_work(),
                transport.capacity.retained_work()
            ),
            (1, 1)
        );
        let result = emission.settlement().unwrap();
        assert_eq!(result.instance, key);
        assert_eq!(result.target, AffordanceTarget::Event("event33".into()));
        assert_eq!(result.native.effect, PublicationEffect::LocallyPublished);
        assert!(result.cause.is_none());
        events.begin_close();
        assert!(events.retire(&mut owner, &mut budget()).is_err());
        assert!(owner.description(key).is_none());
        assert!(transport.service.connected());
        assert_eq!(accounts.account(K::BindingRoutesGlobalMax).used(), 33);
        emission.acknowledge().ok().unwrap();
        // The original native lease owns 66 definitions, 33 routes and the
        // retirement operation itself. One missing credit keeps it connected.
        let native_items = (3 * EVENTS + 1) as u64;
        for (credit, ready) in [(native_items - 1, false), (native_items, true)] {
            let progress = owner
                .step_notifications(
                    support::native(&mut events),
                    &mut cx(),
                    &mut WorkBudget::new().with_remaining(WorkClass::CleanupItems, credit),
                )
                .unwrap();
            assert_eq!(progress.is_ready(), ready);
            assert_eq!(transport.service.connected(), !ready);
        }
        assert!(matches!(
            events.retire(
                &mut owner,
                &mut WorkBudget::new().with_remaining(WorkClass::CleanupItems, 32)
            ),
            Err(CoreError::Backpressure(_))
        ));
        events.retire(&mut owner, &mut budget()).unwrap();
        drop(events);
        assert!(!transport.service.connected());
        assert_eq!(accounts.account(K::BindingRoutesGlobalMax).used(), 0);
        assert_eq!(accounts.account(K::RouteReadinessTokensGlobalMax).used(), 0);
        assert_eq!(transport.capacity.active_work(), 1);
        let mut response = Vec::new();
        let until = Instant::now() + Duration::from_secs(3);
        while !read(&mut http_peer, &mut response) || transport.capacity.active_work() != 0 {
            finite
                .step(&mut owner, &mut cx(), now(), |_, _| budget())
                .unwrap();
            assert!(Instant::now() < until);
        }
        assert!(response.starts_with(b"HTTP/1.1 200"));
        let result = take(&mut finite);
        assert_eq!(result.recipient(), Some(finite.key()));
        finite.begin_close();
        while finite.retire(&mut owner, &mut budget()).is_err() {
            finite
                .step(&mut owner, &mut cx(), now(), |_, _| budget())
                .unwrap();
            assert!(Instant::now() < until);
        }
        drop((result, finite, definition, transport));
        clean(&accounts);
        clean(&http_accounts);
    });
}
