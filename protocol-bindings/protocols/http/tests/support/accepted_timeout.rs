use super::*;

fn put(address: SocketAddr, path: &str, value: u8) -> TcpStream {
    let mut peer = TcpStream::connect(address).unwrap();
    let body = value.to_string();
    write!(peer, "PUT {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    peer
}

fn drive_to(
    exposure: &mut ComposedProducerExposure,
    owner: &mut RawProducerServient,
    tick: u64,
    state: RawFiniteState,
) {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        exposure
            .step(owner, &mut cx(), now(tick), |_, _| work())
            .unwrap();
        let current = exposure.participant_mut::<Server>(0).unwrap().state();
        assert!(
            exposure.progress_failure(0).is_none(),
            "{current:?}: {:?}",
            exposure.progress_failure(0)
        );
        if current == state {
            return;
        }
        assert!(Instant::now() < end, "stalled at {current:?}");
    }
}

fn wire(mut peer: TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes).unwrap();
    bytes
}

fn take(exposure: &mut ComposedProducerExposure) -> ProducerFiniteSettlement {
    exposure
        .participant_mut::<Server>(0)
        .unwrap()
        .take_retained_settlement()
        .unwrap()
}

fn write_owner() -> RawProducerServient {
    RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: 2,
        target_name_bytes: 256,
    })
    .unwrap()
}

fn publish_writes(
    owner: &mut RawProducerServient,
    addr: SocketAddr,
    accounts: &[Arc<HostResourceAccounts>; 2],
    apps: &[Rc<Application>; 2],
    controls: &[Rc<Controls>; 2],
    capacities: [Option<&ProducerFiniteCapacity>; 2],
) -> [ComposedProducerExposure; 2] {
    let listener =
        HttpFiniteListener::new(HttpProducerConfig::for_loopback(addr).unwrap(), 1000).unwrap();
    [0, 1].map(|i| {
        let mut td = description(addr);
        let path = ["/level", "/sibling"][i];
        td["properties"]["level"]["forms"][0]["href"] = json!(format!("http://{addr}{path}"));
        let key = ProducerInstanceAuthority::try_new(ThingSlotId::new(
            SlotIndex::new(i as u32),
            Generation::INITIAL,
        ))
        .unwrap();
        let mut exposure = exposure_with_capacity(
            owner,
            accounts[i].clone(),
            td,
            apps[i].clone(),
            controls[i].clone(),
            key,
            None,
            capacities[i],
            Some(&listener),
        );
        drive_to(&mut exposure, owner, 0, RawFiniteState::InstalledClosed);
        exposure.publish(owner, &mut work()).unwrap();
        exposure
    })
}

#[test]
fn withdrawal_during_pending_reply_retirement_preserves_the_timeout() {
    for explicit_close in [false, true] {
        let addr = address();
        let mut server = Fixture::new(description(addr));
        let peer = put(addr, "/level", 41);
        let key = server.exposure.key();
        drive_to(
            &mut server.exposure,
            &mut server.owner,
            0,
            RawFiniteState::Delivering,
        );
        server.controls.hold_reply_retirement.set(true);
        drive_to(
            &mut server.exposure,
            &mut server.owner,
            1000,
            RawFiniteState::RetiringReply,
        );
        if explicit_close {
            server.controls.hold_close.set(true);
            server.exposure.begin_close();
        } else {
            assert!(server.owner.revoke(key));
        }
        server
            .exposure
            .step(&mut server.owner, &mut cx(), now(1000), |_, _| work())
            .unwrap();
        assert!(server.owner.description(key).is_none());
        assert!(server.server().take_settlement().is_none());
        assert_eq!(
            server.server().state(),
            if explicit_close {
                RawFiniteState::Closing
            } else {
                RawFiniteState::RetiringReply
            }
        );
        server.controls.hold_reply_retirement.set(false);
        server.controls.hold_close.set(false);
        for _ in 0..2 {
            server
                .exposure
                .step(&mut server.owner, &mut cx(), now(1000), |_, _| work())
                .unwrap();
        }
        assert_eq!(server.server().state(), RawFiniteState::Closed);
        assert!(wire(peer).is_empty());
        assert_eq!(server.app.writes.get(), 1);
        assert!(matches!(
            (explicit_close, server.exposure.progress_failure(0)),
            (true, Some(CoreError::Cancelled(_))) | (false, None)
        ));
        let original = server.server().take_settlement().unwrap();
        assert_eq!(original.recipient, Some(key));
        assert!(
            matches!(original.first_cause, Some(CoreError::TimedOut(c)) if c.phase() == ErrorPhase::Binding)
        );
        assert_eq!(original.effect, FiniteEffect::Completed);
        assert_eq!(original.delivery, FiniteDelivery::NotDelivered);
        assert!(original.result.is_ok());
        server.close();
    }
}

#[test]
fn listener_loss_after_revocation_closes_a_pending_reply_and_releases_its_slot() {
    let addr = address();
    let mut owner = write_owner();
    let work_bank = bank();
    let capacity =
        ProducerFiniteCapacity::try_new(&owner, exchange_limits(), 1, 1, Some(work_bank.clone()))
            .unwrap();
    let accounts = [bank(), bank()];
    let apps = Default::default();
    let controls = Default::default();
    let [mut first, mut sibling] = publish_writes(
        &mut owner,
        addr,
        &accounts,
        &apps,
        &controls,
        [Some(&capacity), None],
    );
    let key = first.key();
    let peer = put(addr, "/level", 41);
    drive_to(&mut first, &mut owner, 10, RawFiniteState::Delivering);
    controls[0].hold_reply_retirement.set(true);
    drive_to(&mut first, &mut owner, 1010, RawFiniteState::RetiringReply);
    assert!(owner.revoke(key));
    drive_to(&mut first, &mut owner, 1010, RawFiniteState::RetiringReply);
    assert!(owner.description(key).is_none());
    assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));

    // This route's first receive is valid on its own clock, but precedes
    // the tick 10 receive already observed by the shared listener.
    sibling
        .step(&mut owner, &mut cx(), now(0), |_, _| work())
        .unwrap();
    assert!(matches!(
        sibling.progress_failure(0),
        Some(CoreError::Binding(_))
    ));
    // The cleanup turn can observe loss even before the result queue is funded.
    first
        .step(&mut owner, &mut cx(), now(1010), |_, _| {
            WorkBudget::new().with_remaining(WorkClass::CleanupItems, 1)
        })
        .unwrap();
    let member = first.participant_mut::<Server>(0).unwrap();
    assert_eq!(member.state(), RawFiniteState::Closing);
    assert!(matches!(member.first_cause(), Some(CoreError::Binding(_))));
    assert!(member.take_retained_settlement().is_none());
    assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));

    // Only whole-participation close is available. It still owns the original PUT.
    first
        .step(&mut owner, &mut cx(), now(1010), |_, _| {
            WorkBudget::new()
                .with_remaining(WorkClass::CleanupItems, 1)
                .with_remaining(WorkClass::QueueOperations, 1)
        })
        .unwrap();
    assert_eq!(
        first.participant_mut::<Server>(0).unwrap().state(),
        RawFiniteState::Closed
    );
    assert!(matches!(
        first.progress_failure(0),
        Some(CoreError::Binding(_))
    ));
    assert!(wire(peer).is_empty());
    assert_eq!((apps[0].writes.get(), apps[1].writes.get()), (1, 0));
    assert_eq!(*apps[0].value.borrow(), b"41");
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 1));
    let original = take(&mut first);
    assert_eq!(original.recipient(), Some(key));
    assert_eq!(original.operation(), Some(Operation::WriteProperty));
    assert!(
        matches!(original.first_cause(), Some(CoreError::TimedOut(c)) if c.phase() == ErrorPhase::Binding)
    );
    assert_eq!(original.effect(), FiniteEffect::Completed);
    assert_eq!(original.delivery(), FiniteDelivery::NotDelivered);
    assert_eq!(original.result().unwrap().status(), InteractionStatus::Ok);
    first.retire(&mut owner, &mut work()).unwrap();
    sibling
        .step(&mut owner, &mut cx(), now(0), |_, _| work())
        .unwrap();
    sibling.retire(&mut owner, &mut work()).unwrap();
    drop((first, sibling, owner));
    TcpListener::bind(addr).unwrap();
    assert_eq!(capacity.retained_work(), 1);
    drop(original);
    assert_eq!(capacity.retained_work(), 0);
    drop(capacity);
    clean(&work_bank);
    for account in accounts {
        clean(&account);
    }
}

#[test]
fn accepted_write_timeout_preserves_both_publications_and_the_funded_original() {
    for handler_error in [false, true] {
        let addr = address();
        let mut owner = write_owner();
        let work_bank = bank();
        let capacity = ProducerFiniteCapacity::try_new(
            &owner,
            exchange_limits(),
            1,
            3,
            Some(work_bank.clone()),
        )
        .unwrap();
        let accounts = [bank(), bank()];
        let apps = Default::default();
        let controls = Default::default();
        let [mut first, mut sibling] = publish_writes(
            &mut owner,
            addr,
            &accounts,
            &apps,
            &controls,
            [Some(&capacity); 2],
        );
        let original_key = first.key();
        apps[0].output.set(u8::from(handler_error));
        let peer = put(addr, "/level", 41);
        drive_to(&mut first, &mut owner, 0, RawFiniteState::Delivering);
        assert_eq!(apps[0].writes.get(), 1);
        assert_eq!(*apps[0].value.borrow(), b"41");
        assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));

        // The request deadline falls between behavior entry and the first native send.
        if handler_error {
            controls[0].hold_reply_retirement.set(true);
            drive_to(&mut first, &mut owner, 1000, RawFiniteState::RetiringReply);
            assert!(
                first
                    .participant_mut::<Server>(0)
                    .unwrap()
                    .take_retained_settlement()
                    .is_none()
            );
            assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));
            controls[0].hold_reply_retirement.set(false);
        }
        drive_to(&mut first, &mut owner, 1000, RawFiniteState::Serving);
        assert!(wire(peer).is_empty());
        assert!(owner.description(original_key).is_some());
        assert!(owner.description(sibling.key()).is_some());
        assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 1));

        apps[0].output.set(0);
        let peer = put(addr, "/level", 42);
        drive_to(&mut first, &mut owner, 1000, RawFiniteState::Delivering);
        drive_to(&mut first, &mut owner, 1000, RawFiniteState::Serving);
        assert!(wire(peer).starts_with(b"HTTP/1.1 204"));
        let original = take(&mut first);
        let next = take(&mut first);

        let peer = put(addr, "/sibling", 73);
        drive_to(&mut sibling, &mut owner, 1000, RawFiniteState::Delivering);
        drive_to(&mut sibling, &mut owner, 1000, RawFiniteState::Serving);
        assert!(wire(peer).starts_with(b"HTTP/1.1 204"));
        let other = take(&mut sibling);
        assert!(original.sequence() < next.sequence() && next.sequence() < other.sequence());
        assert_eq!(next.recipient(), Some(original_key));
        assert_eq!(other.recipient(), Some(sibling.key()));
        for delivered in [next, other] {
            assert!(delivered.first_cause().is_none());
            assert_eq!(delivered.delivery(), FiniteDelivery::Delivered);
            assert_eq!(delivered.effect(), FiniteEffect::Completed);
            assert!(delivered.result().is_ok());
        }
        apps[0].denies.set(true);
        let peer = put(addr, "/level", 99);
        drive_to(&mut first, &mut owner, 1000, RawFiniteState::Delivering);
        drive_to(&mut first, &mut owner, 1000, RawFiniteState::Serving);
        assert!(wire(peer).starts_with(b"HTTP/1.1 403"));
        let refused = take(&mut first);
        assert_eq!(refused.effect(), FiniteEffect::NotInvoked);
        drop(refused);
        assert_eq!((apps[0].writes.get(), apps[1].writes.get()), (2, 1));
        assert_eq!(*apps[0].value.borrow(), b"42");
        assert_eq!(*apps[1].value.borrow(), b"73");
        for mut exposure in [first, sibling] {
            exposure.begin_close();
            exposure
                .step(&mut owner, &mut cx(), now(1000), |_, _| work())
                .unwrap();
            exposure.retire(&mut owner, &mut work()).unwrap();
        }
        drop(owner);
        TcpListener::bind(addr).unwrap();
        assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 1));
        assert_eq!(
            work_bank
                .account(ResourceKind::BindingIngressItemsGlobalMax)
                .used(),
            0
        );
        assert_eq!(
            work_bank
                .account(ResourceKind::PendingHandlerCallsGlobalMax)
                .used(),
            1
        );
        assert!(
            accounts[0]
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                > 0
        );
        assert_eq!((apps[0].drops.get(), apps[1].drops.get()), (0, 2));
        assert_eq!(original.recipient(), Some(original_key));
        assert_eq!(
            original.binding(),
            (BindingId::new(97), BindingGeneration::INITIAL)
        );
        assert_eq!(
            original.target(),
            Some(clinkz_wot_core::thing::AffordanceSite::Property("level"))
        );
        assert_eq!(original.operation(), Some(Operation::WriteProperty));
        assert_eq!(original.delivery(), FiniteDelivery::NotDelivered);
        assert!(
            matches!(original.first_cause(), Some(CoreError::TimedOut(c)) if c.phase() == ErrorPhase::Binding)
        );
        if handler_error {
            assert_eq!(original.effect(), FiniteEffect::Unknown);
            assert!(
                matches!(original.result(), Err(CoreError::Binding(c)) if c.phase() == ErrorPhase::Handler)
            );
        } else {
            assert_eq!(original.effect(), FiniteEffect::Completed);
            assert_eq!(original.result().unwrap().status(), InteractionStatus::Ok);
        }
        clean(&accounts[1]);
        drop(original);
        assert_eq!(apps[0].drops.get(), 2);
        assert_eq!(capacity.retained_work(), 0);
        drop(capacity);
        clean(&accounts[0]);
        clean(&work_bank);
    }
}
