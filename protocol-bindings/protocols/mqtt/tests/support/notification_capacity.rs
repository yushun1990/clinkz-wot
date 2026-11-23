//! Complete notification publication, attribution and original-owner retirement.
use super::*;
use clinkz_wot_foundation::ResourceKind as K;

const EVENTS: usize = 33;

fn events_owner(count: usize) -> RawProducerServient {
    RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: count,
        target_name_bytes: 64,
    })
    .unwrap()
}

fn event_td(broker: SocketAddr, count: usize, duplicate_last_topic: bool) -> String {
    let mut td = support::td(broker, "unused");
    let mut events = serde_json::Map::new();
    for i in 1..=count {
        let name = format!("event{i:02}");
        let topic = if duplicate_last_topic && i == count {
            i - 1
        } else {
            i
        };
        let schema = if i == count { "string" } else { "integer" };
        events.insert(name.clone(), serde_json::json!({
            "data": {"type": schema},
            "forms": [{"href": format!("mqtt://{broker}"), "mqv:filter": format!("capacity/event{topic:02}")}]
        }));
    }
    td["events"] = events.into();
    td.to_string()
}

fn event_instance(
    broker: SocketAddr,
    count: usize,
    duplicate_last_topic: bool,
) -> RawProducerInstance {
    let sources = (1..=count)
        .map(|i| ProducerNotificationSource::event(format!("event{i:02}")))
        .collect();
    RawProducerInstance::new(authority(2), vec![])
        .with_notification_sources(sources, &mut budget())
        .unwrap_or_else(|r| panic!("{:?}", r.error().error()))
        .with_description(Arc::from(event_td(broker, count, duplicate_last_topic)))
}

fn event_input(
    owner: &RawProducerServient,
    instance: RawProducerInstance,
    provider: &MqttNotificationProducer,
    service: &MqttNotificationService,
    events_per_member: usize,
    lifetime: &mut WorkBudget,
) -> CoreResult<ComposedProducerInput> {
    let prepared = support::prepare(&instance, &[provider])?;
    let count = instance.behavior_count();
    assert_eq!(prepared.operations().len(), 2 * count);
    assert_eq!(prepared.notification_mapping_count(), count);
    let mut groups = vec![Vec::new(); count.div_ceil(events_per_member)];
    for op in prepared.operations() {
        let clinkz_wot_core::thing::AffordanceSite::Event(name) = op.site.site() else {
            panic!("unexpected Event operation")
        };
        let number: usize = name.strip_prefix("event").unwrap().parse().unwrap();
        groups[(number - 1) / events_per_member].push(
            op.definition::<MqttNotificationDefinition>()
                .unwrap()
                .clone(),
        );
    }
    let mut participants = Vec::new();
    for mut definitions in groups {
        let native = service.participation(
            &owner.participation_token(),
            instance.key(),
            &mut definitions,
            lifetime,
        )?;
        participants.push(OwnedProducerParticipation::notifications(native));
    }
    Ok(ComposedProducerInput {
        instance,
        prepared,
        providers: ProducerRegistrations::new(vec![Rc::new(provider.clone())]),
        participants,
        expected: None,
        handler_backing_bytes: 0,
    })
}

fn cleanup(items: u64) -> WorkBudget {
    WorkBudget::new().with_remaining(WorkClass::CleanupItems, items)
}

fn gateway_cleanup() -> u64 {
    GatewayDefaultV1::LIMITS
        .get(K::CleanupWorkItemsPerStepMax)
        .unwrap()
}

fn mqtt_service(broker: &Broker) -> (MqttNotificationProducer, MqttNotificationService) {
    let provider = MqttNotificationProducer::new(
        MqttProducerConfig::for_loopback(broker.address, 8192).unwrap(),
        BindingId::new(602),
        BindingGeneration::INITIAL,
    );
    let service = MqttNotificationService::new(&provider).unwrap();
    (provider, service)
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
        let (provider, service) = mqtt_service(&broker);
        let mut owner = events_owner(EVENTS);
        let mut peer = Peer::new(&broker);
        peer.subscribe("capacity/+");
        for (mappings, split, kind, limit, needed) in [
            (32, false, K::BindingRoutesGlobalMax, 32, 33),
            (33, true, K::RouteReadinessTokensGlobalMax, 1, 2),
        ] {
            let bank = publication_bank(mappings, 1);
            let input = event_input(
                &owner,
                event_instance(broker.address, EVENTS, false),
                &provider,
                &service,
                if split { 32 } else { EVENTS },
                &mut budget(),
            )
            .unwrap();
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
        let mut owner = events_owner(EVENTS);
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
            event_instance(broker.address, EVENTS, false),
            &transport.mqtt,
            &transport.service,
            EVENTS,
            &mut budget(),
        )
        .unwrap();
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
        for (credit, ready) in [(native_items - 1, false), (1, true)] {
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

#[test]
fn same_incarnation_topic_collision_refuses_for_one_or_two_members() {
    runtime().block_on(async {
        let broker = Broker::start();
        let (provider, service) = mqtt_service(&broker);
        let owner = events_owner(EVENTS);
        let mut peer = Peer::new(&broker);
        peer.subscribe("capacity/event32");
        for group in [33, 32] {
            let instance = event_instance(broker.address, EVENTS, true);
            let key = instance.key();
            assert!(matches!(
                event_input(&owner, instance, &provider, &service, group, &mut budget()),
                Err(CoreError::UnsupportedOperation(_))
            ));
            assert!(owner.description(key).is_none());
            assert!(!service.connected());
            assert_eq!(service.locally_published(), 0);
            peer.absent();
        }
    });
}

// Measure only the calling thread and only the synchronous operation under test.
struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
std::thread_local! {
    static ALLOCATION_COUNTS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let _ = ALLOCATION_COUNTS.try_with(|counts| {
            if let Some((a, d)) = counts.get() {
                counts.set(Some((a + 1, d)));
            }
        });
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        let _ = ALLOCATION_COUNTS.try_with(|counts| {
            if let Some((a, d)) = counts.get() {
                counts.set(Some((a, d + 1)));
            }
        });
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}
fn allocations<T>(operation: impl FnOnce() -> T) -> (T, (usize, usize)) {
    ALLOCATION_COUNTS.with(|counts| counts.set(Some((0, 0))));
    let result = operation();
    (
        result,
        ALLOCATION_COUNTS.with(|counts| counts.take().unwrap()),
    )
}

#[test]
fn large_native_inventory_drains_across_gateway_turns_and_orphaned_rows_stay_at_the_owner() {
    runtime().block_on(async {
        let broker = Broker::start();
        let (provider, service) = mqtt_service(&broker);
        let mut owner = events_owner(86);
        let accounts = publication_bank(86, 1);
        let instance = event_instance(broker.address, 86, false);
        let prepared = support::prepare(&instance, &[&provider]).unwrap();
        let mut definitions: Vec<_> = prepared
            .operations()
            .iter()
            .map(|op| {
                op.definition::<MqttNotificationDefinition>()
                    .unwrap()
                    .clone()
            })
            .collect();
        let mut short = cleanup(257);
        assert!(matches!(
            service.participation(
                &owner.participation_token(),
                instance.key(),
                &mut definitions,
                &mut short
            ),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(definitions.len(), 172);
        assert_eq!(short.remaining(WorkClass::CleanupItems), 257);
        assert!(!service.connected());
        drop((definitions, prepared));
        let mut lifetime = cleanup(258);
        let input = event_input(&owner, instance, &provider, &service, 86, &mut lifetime).unwrap();
        assert_eq!(lifetime.remaining(WorkClass::CleanupItems), 0);
        let mut exposure = support::stage(&owner, input, &accounts);
        publish_members(&mut owner, &mut exposure).await;
        let key = exposure.key();
        exposure.begin_close();
        let quantum = gateway_cleanup();
        let mut first = cleanup(quantum);
        assert!(
            owner
                .step_notifications(support::native(&mut exposure), &mut cx(), &mut first)
                .unwrap()
                .is_pending()
        );
        assert_eq!(first.remaining(WorkClass::CleanupItems), 0);
        assert!(service.connected());
        let mut second = cleanup(quantum);
        assert!(
            owner
                .step_notifications(support::native(&mut exposure), &mut cx(), &mut second)
                .unwrap()
                .is_ready()
        );
        assert_eq!(2 * quantum - second.remaining(WorkClass::CleanupItems), 259);
        assert!(!service.connected());
        let mut partial = cleanup(16);
        assert!(matches!(
            exposure.retire(&mut owner, &mut partial),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(partial.remaining(WorkClass::CleanupItems), 0);
        drop(exposure);
        assert_eq!(accounts.account(K::BindingRoutesGlobalMax).used(), 86);
        owner
            .retire_revoked_publication(key, &mut cleanup(quantum))
            .unwrap();
        clean(&accounts);
    });
}

#[test]
fn eleven_original_members_retire_within_the_gateway_quantum_and_zero_credit_allocates_nothing() {
    runtime().block_on(async {
        let broker = Broker::start();
        let (provider, service) = mqtt_service(&broker);
        let mut owner = events_owner(EVENTS);
        let accounts = publication_bank(EVENTS as u64, 11);
        let input = event_input(
            &owner,
            event_instance(broker.address, EVENTS, false),
            &provider,
            &service,
            3,
            &mut budget(),
        )
        .unwrap();
        let mut exposure = support::stage(&owner, input, &accounts);
        publish_members(&mut owner, &mut exposure).await;
        exposure.begin_close();
        for _ in 0..2 {
            let (refused, counts) = allocations(|| exposure.retire(&mut owner, &mut cleanup(0)));
            assert!(matches!(refused, Err(CoreError::Backpressure(_))));
            assert_eq!(counts, (0, 0));
        }
        for index in 0..11 {
            let member = exposure
                .participant_mut::<MqttNotificationRuntime>(index)
                .unwrap();
            assert!(
                owner
                    .step_notifications(member, &mut cx(), &mut cleanup(10))
                    .unwrap()
                    .is_ready()
            );
        }
        assert!(!service.connected());
        exposure
            .retire(&mut owner, &mut cleanup(gateway_cleanup()))
            .unwrap();
        drop(exposure);
        clean(&accounts);
    });
}

#[test]
fn cutover_charges_displaced_routes_and_allows_the_successor_to_reuse_its_topic() {
    runtime().block_on(async {
        for count in [1, 86] {
            let broker = Broker::start();
            let (provider, service) = mqtt_service(&broker);
            let mut owner = events_owner(count);
            let accounts = publication_bank(count as u64 + 1, 2);
            let input = event_input(
                &owner,
                event_instance(broker.address, count, false),
                &provider,
                &service,
                count,
                &mut budget(),
            )
            .unwrap();
            let mut incumbent = support::stage(&owner, input, &accounts);
            publish_members(&mut owner, &mut incumbent).await;
            let mut next = event_input(
                &owner,
                event_instance(broker.address, 1, false),
                &provider,
                &service,
                1,
                &mut budget(),
            )
            .unwrap();
            next.expected = Some(incumbent.key());
            let mut next = support::stage(&owner, next, &accounts);
            next.step(&mut owner, &mut cx(), now(), |_, _| budget())
                .unwrap();
            let mut short = budget().with_remaining(WorkClass::CleanupItems, count as u64 - 1);
            assert!(matches!(
                next.publish(&mut owner, &mut short),
                Err(CoreError::Backpressure(_))
            ));
            assert!(owner.description(incumbent.key()).is_some());
            assert!(support::native(&mut incumbent).ingress_gate().is_open());
            assert!(support::native(&mut next).ingress_gate().is_staged());
            let mut peer = Peer::new(&broker);
            peer.subscribe("capacity/event01");
            let key = incumbent.key();
            let payload: &[u8] = if count == 1 { br#""incumbent""# } else { b"41" };
            let mut before = owner
                .publish_notification(
                    support::native(&mut incumbent),
                    key,
                    AffordanceTarget::Event("event01".into()),
                    data(payload),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            finish(&mut before).await;
            peer.event("capacity/event01", payload);
            assert_eq!(before.acknowledge().ok().unwrap().instance, key);
            let mut exact = budget().with_remaining(WorkClass::CleanupItems, count as u64);
            next.publish(&mut owner, &mut exact).unwrap();
            assert_eq!(exact.remaining(WorkClass::CleanupItems), 0);
            assert!(owner.description(incumbent.key()).is_none());
            assert!(
                support::native(&mut incumbent)
                    .ingress_gate()
                    .is_withdrawn()
            );
            let key = next.key();
            let mut emission = owner
                .publish_notification(
                    support::native(&mut next),
                    key,
                    AffordanceTarget::Event("event01".into()),
                    data(br#""successor""#),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            finish(&mut emission).await;
            peer.event("capacity/event01", br#""successor""#);
            assert_eq!(emission.acknowledge().ok().unwrap().instance, key);
            drop((incumbent, next));
            owner
                .retire_revoked_publication(key, &mut cleanup(gateway_cleanup()))
                .unwrap();
            clean(&accounts);
        }
    });
}

#[test]
fn last_settlement_releases_only_its_fixed_lease_for_one_or_86_events() {
    runtime().block_on(async {
        let mut acknowledgements = Vec::new();
        for count in [1, 86] {
            let broker = Broker::start();
            let (provider, service) = mqtt_service(&broker);
            let mut owner = events_owner(count);
            let accounts = publication_bank(count as u64, 1);
            let mut lifetime = cleanup(3 * count as u64);
            let input = event_input(
                &owner,
                event_instance(broker.address, count, false),
                &provider,
                &service,
                count,
                &mut lifetime,
            )
            .unwrap();
            assert_eq!(lifetime.remaining(WorkClass::CleanupItems), 0);
            let mut exposure = support::stage(&owner, input, &accounts);
            publish_members(&mut owner, &mut exposure).await;
            let key = exposure.key();
            let mut emission = owner
                .publish_notification(
                    support::native(&mut exposure),
                    key,
                    AffordanceTarget::Event(format!("event{count:02}").into()),
                    data(br#""last""#),
                    Deadline::NONE,
                    now(),
                    &mut budget(),
                )
                .ok()
                .unwrap();
            finish(&mut emission).await;
            drop(exposure);
            assert!(service.connected());
            assert_eq!(
                accounts.account(K::BindingRoutesGlobalMax).used(),
                count as u64
            );
            let (result, counts) = allocations(|| emission.acknowledge().ok().unwrap());
            assert_eq!(result.instance, key);
            assert!(!service.connected());
            acknowledgements.push(counts.1);
            owner
                .retire_revoked_publication(key, &mut cleanup(gateway_cleanup()))
                .unwrap();
            clean(&accounts);
        }
        assert_eq!(acknowledgements[0], acknowledgements[1]);
    });
}

#[path = "notification_capacity/retirement.rs"]
mod retirement;
