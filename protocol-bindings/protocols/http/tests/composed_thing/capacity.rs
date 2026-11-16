//! Real shared listener, fully qualified independent publications, finite work
//! pressure and original retained result ownership. No alternate native driver.
use super::*;

fn exchange_limits() -> RawFiniteLimits {
    RawFiniteLimits {
        candidates: 1,
        input_body_bytes: 0,
        output_body_bytes: 4096,
        output_media_capacity: 64,
        output_retained_bytes: 8192,
    }
}
fn capacity(
    owner: &RawProducerServient,
    accounts: &Arc<HostResourceAccounts>,
    retained: usize,
) -> ProducerFiniteCapacity {
    ProducerFiniteCapacity::try_new(owner, exchange_limits(), 1, retained, accounts.clone())
        .unwrap()
}
#[allow(clippy::too_many_arguments)]
fn publication(
    owner: &RawProducerServient,
    address: SocketAddr,
    native: &HttpFiniteListener,
    capacity: &ProducerFiniteCapacity,
    provider: Rc<HttpFiniteProducer>,
    accounts: &Arc<HostResourceAccounts>,
    authority: ProducerInstanceAuthority,
    name: &'static str,
    value: u8,
    app: Rc<App>,
) -> ComposedProducerExposure {
    admit(
        owner,
        input_with_capacity(
            owner,
            authority,
            &td(address, name, &[&format!("/{name}")]),
            provider,
            name,
            value,
            app,
            Some(native),
            Some(capacity),
        ),
        accounts,
    )
}
fn take(exposure: &mut ComposedProducerExposure) -> Box<ProducerFiniteSettlement> {
    let result = exposure
        .participant_mut::<HttpRuntime>(0)
        .unwrap()
        .take_retained_settlement()
        .unwrap();
    assert_eq!(result.recipient(), Some(exposure.key()));
    result
}
fn delivering(owner: &mut RawProducerServient, exposure: &mut ComposedProducerExposure) {
    let end = Instant::now() + Duration::from_secs(5);
    while exposure.participant_mut::<HttpRuntime>(0).unwrap().state() != RawFiniteState::Delivering
    {
        drive(owner, &mut [exposure]);
        assert!(Instant::now() < end, "dispatch stalled");
    }
}
fn assert_work_empty(accounts: &HostResourceAccounts) {
    for kind in ResourceKind::ALL {
        assert_eq!(accounts.account(kind).used(), 0, "{kind:?}");
    }
}
fn assert_result(result: &ProducerFiniteSettlement, key: ProducerInstanceKey, value: u8) {
    assert_eq!(result.recipient(), Some(key));
    assert_eq!(result.operation(), Some(Operation::ReadProperty));
    assert_eq!(result.effect(), FiniteEffect::Unknown);
    assert_eq!(
        result.result().unwrap().data().unwrap().body,
        format!("{{\"value\":{value}}}").as_bytes()
    );
}

#[test]
fn shared_work_saturated_receive_cannot_starve_sibling_with_one_native_credit() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let definition_bank = bank();
    let banks = [bank(), bank()];
    let work_bank = bank();
    let capacity = capacity(&owner, &work_bank, 2);
    let provider = compiler(address, &definition_bank);
    let apps = [Rc::new(App::default()), Rc::new(App::default())];
    let mut active = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &banks[0],
        key(1, Generation::INITIAL),
        "alpha",
        11,
        apps[0].clone(),
    );
    let mut blocked = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &banks[1],
        key(2, Generation::INITIAL),
        "beta",
        22,
        apps[1].clone(),
    );
    for exposure in [&mut active, &mut blocked] {
        start(exposure);
        exposure.publish(&mut owner, &mut budget()).unwrap();
    }
    let active_peer = connect(address, "/alpha");
    delivering(&mut owner, &mut active);
    let blocked_peer = connect(address, "/beta");
    assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));

    // Fixed caller order, one shared native credit per turn. The second
    // participant cannot receive until the first completes its original send.
    // No socket timing or deadline determines the capacity/budget deadlock.
    let mut blocked_remainders = Vec::new();
    for _ in 0..4 {
        let mut turn = WorkBudget::new()
            .with_remaining(WorkClass::BindingPolls, 1)
            .with_remaining(WorkClass::QueueOperations, 32);
        assert!(
            owner
                .step(
                    blocked.participant_mut::<HttpRuntime>(0).unwrap(),
                    &mut cx(),
                    &mut turn,
                    now()
                )
                .unwrap()
                .is_pending()
        );
        blocked_remainders.push(turn.remaining(WorkClass::BindingPolls));
        let _ = owner
            .step(
                active.participant_mut::<HttpRuntime>(0).unwrap(),
                &mut cx(),
                &mut turn,
                now(),
            )
            .unwrap();
        if active.participant_mut::<HttpRuntime>(0).unwrap().state() == RawFiniteState::Serving {
            break;
        }
    }
    assert_eq!(
        active.participant_mut::<HttpRuntime>(0).unwrap().state(),
        RawFiniteState::Serving,
        "saturated receive stole the releasing sibling's credit in every turn: {blocked_remainders:?}"
    );
    assert!(blocked_remainders.iter().all(|remaining| *remaining == 1));
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 1));
    assert_eq!(apps[1].calls.get(), 0);
    assert_value(&response(active_peer, &mut owner, &mut []), 11);
    assert_value(&response(blocked_peer, &mut owner, &mut [&mut blocked]), 22);
    let original = take(&mut active);
    let sibling = take(&mut blocked);
    assert_result(&original, active.key(), 11);
    assert_result(&sibling, blocked.key(), 22);
    assert!(original.sequence() < sibling.sequence());
    for exposure in [&mut active, &mut blocked] {
        close(exposure);
        exposure.retire(&mut owner, &mut budget()).unwrap();
    }
    drop(active);
    drop(blocked);
    drop(owner);
    drop(provider);
    TcpListener::bind(address).unwrap();
    drop(original);
    drop(sibling);
    drop(capacity);
    for bank in [&banks[0], &banks[1], &definition_bank, &work_bank] {
        assert_work_empty(bank);
    }
}

#[test]
fn shared_work_publication_active_pressure_unread_progress_and_retained_pressure() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let definition_bank = bank();
    let accounts = [bank(), bank()];
    let work_bank = bank();
    let capacity = capacity(&owner, &work_bank, 3);
    let idle_bytes = live(&work_bank);
    let provider = compiler(address, &definition_bank);
    let apps = [Rc::new(App::default()), Rc::new(App::default())];
    let mut first = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &accounts[0],
        key(1, Generation::INITIAL),
        "alpha",
        11,
        apps[0].clone(),
    );
    start(&mut first);
    first.publish(&mut owner, &mut budget()).unwrap();
    for _ in 0..5 {
        drive(&mut owner, &mut [&mut first]);
    }
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 0));
    assert_eq!(
        live(&work_bank),
        idle_bytes,
        "idle native receive retains no Engine work"
    );

    let first_peer = connect(address, "/alpha");
    delivering(&mut owner, &mut first);
    assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));
    assert!(
        owner
            .step(
                first.participant_mut::<HttpRuntime>(0).unwrap(),
                &mut cx(),
                &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
                now()
            )
            .unwrap()
            .is_pending()
    );
    assert_eq!(
        first.participant_mut::<HttpRuntime>(0).unwrap().state(),
        RawFiniteState::Delivering
    );
    // Qualification, closed setup and publication do not require a vacant slot.
    let mut second = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &accounts[1],
        key(2, Generation::INITIAL),
        "beta",
        22,
        apps[1].clone(),
    );
    start(&mut second);
    second.publish(&mut owner, &mut budget()).unwrap();
    let second_peer = connect(address, "/beta");
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut second]);
    }
    assert_eq!(
        apps[1].calls.get(),
        0,
        "active limit prevents another dispatch"
    );
    assert!(owner.description(second.key()).is_some());

    assert_value(&response(first_peer, &mut owner, &mut [&mut first]), 11);
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 1));
    assert_value(&response(second_peer, &mut owner, &mut [&mut second]), 22);
    // Another request on the same continuously published Thing also progresses.
    assert_value(&request(address, "/alpha", &mut owner, &mut first), 11);
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 3));
    assert_eq!(apps[0].calls.get(), 2);
    assert_eq!(
        work_bank
            .account(ResourceKind::PendingHandlerCallsGlobalMax)
            .used(),
        3
    );
    assert_eq!(
        work_bank
            .account(ResourceKind::BindingIngressItemsGlobalMax)
            .used(),
        0
    );
    let blocked = connect(address, "/beta");
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut first, &mut second]);
    }
    assert_eq!(apps[1].calls.get(), 1);
    let original = take(&mut first);
    assert_result(&original, first.key(), 11);
    assert_eq!(original.delivery(), FiniteDelivery::Delivered);
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut first, &mut second]);
    }
    assert_eq!(
        apps[1].calls.get(),
        1,
        "acknowledged ownership still consumes retained quota"
    );
    assert_eq!(capacity.retained_work(), 3);
    let old_sequence = original.sequence();
    drop(original);
    assert_eq!(capacity.retained_work(), 2);
    assert_value(
        &response(blocked, &mut owner, &mut [&mut first, &mut second]),
        22,
    );
    let later_first = take(&mut first);
    let first_second = take(&mut second);
    let later_second = take(&mut second);
    assert!(old_sequence < first_second.sequence());
    assert!(first_second.sequence() < later_first.sequence());
    assert!(later_first.sequence() < later_second.sequence());
    assert_result(&first_second, second.key(), 22);
    assert_result(&later_first, first.key(), 11);
    assert_eq!(later_first.binding(), first_second.binding()); // IDs alone cannot conflate requests.
    assert!(
        first
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .take_retained_settlement()
            .is_none()
    );
    for exposure in [&mut first, &mut second] {
        close(exposure);
        exposure.retire(&mut owner, &mut budget()).unwrap();
    }
    drop(first);
    drop(second);
    drop(owner);
    drop(provider);
    TcpListener::bind(address).unwrap(); // Retained outcomes do not retain native socket occupancy.
    assert!(live(&accounts[0]) > 0 && live(&accounts[1]) > 0);
    drop(later_first);
    assert_empty(&accounts[0]);
    drop(first_second);
    assert!(live(&accounts[1]) > 0);
    drop(later_second);
    assert_empty(&accounts[1]);
    assert_empty(&definition_bank);
    assert_eq!(capacity.retained_work(), 0);
    assert_eq!(live(&work_bank), idle_bytes);
    drop(capacity);
    assert_work_empty(&work_bank);
}

#[test]
fn shared_work_bank_refusal_zero_credit_and_fresh_policy_precede_effects() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let definition_bank = bank();
    let exposure_bank = bank();
    let work_bank = bank();
    let capacity = capacity(&owner, &work_bank, 2);
    let idle_bytes = live(&work_bank);
    let provider = compiler(address, &definition_bank);
    let app = Rc::new(App::default());
    let mut exposure = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &exposure_bank,
        key(1, Generation::INITIAL),
        "alpha",
        11,
        app.clone(),
    );
    start(&mut exposure);
    exposure.publish(&mut owner, &mut budget()).unwrap();
    let peer = connect(address, "/alpha");
    for _ in 0..5 {
        assert!(
            owner
                .step(
                    exposure.participant_mut::<HttpRuntime>(0).unwrap(),
                    &mut cx(),
                    &mut WorkBudget::new(),
                    now()
                )
                .unwrap()
                .is_pending()
        );
    }
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 0));
    for kind in [
        ResourceKind::EngineLiveBytesGlobalMax,
        ResourceKind::CleanupBytesMax,
        ResourceKind::CleanupItemsMax,
        ResourceKind::InFlightResponsesGlobalMax,
        ResourceKind::PendingHandlerCallsGlobalMax,
        ResourceKind::BindingIngressItemsGlobalMax,
    ] {
        let account = work_bank.account(kind);
        let pressure = work_bank
            .reserve(kind, account.limit() - account.used())
            .unwrap();
        let before = work_bank.snapshot(ResourceKind::ALL);
        for _ in 0..20 {
            let mut turn = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
            assert!(
                owner
                    .step(
                        exposure.participant_mut::<HttpRuntime>(0).unwrap(),
                        &mut cx(),
                        &mut turn,
                        now()
                    )
                    .unwrap()
                    .is_pending()
            );
            assert_eq!(
                turn.remaining(WorkClass::BindingPolls),
                1,
                "{kind:?} refusal performed no native poll"
            );
        }
        assert_eq!(app.calls.get(), 0);
        assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 0));
        for (before, after) in before
            .into_iter()
            .zip(work_bank.snapshot(ResourceKind::ALL))
        {
            assert_eq!(
                before.used(),
                after.used(),
                "partial reservation leaked under {kind:?}"
            );
        }
        assert!(owner.description(exposure.key()).is_some());
        drop(pressure);
    }
    app.denied.set(true);
    let denied = response(peer, &mut owner, &mut [&mut exposure]);
    assert!(!denied.starts_with(b"HTTP/1.1 200"));
    let refused = take(&mut exposure);
    assert_eq!(refused.effect(), FiniteEffect::NotInvoked);
    assert!(matches!(refused.result(), Err(CoreError::Security { .. })));
    assert_eq!(app.calls.get(), 0);
    app.denied.set(false);
    assert_value(&request(address, "/alpha", &mut owner, &mut exposure), 11);
    let normal = take(&mut exposure);
    assert_result(&normal, exposure.key(), 11);
    assert_ne!(refused.sequence(), normal.sequence());
    close(&mut exposure);
    exposure.retire(&mut owner, &mut budget()).unwrap();
    drop(exposure);
    drop(owner);
    drop(provider);
    drop(refused);
    drop(normal);
    assert_eq!(live(&work_bank), idle_bytes);
    drop(capacity);
    for bank in [&work_bank, &definition_bank, &exposure_bank] {
        assert_work_empty(bank);
    }
    TcpListener::bind(address).unwrap();
}

#[test]
fn shared_work_replacement_keeps_pending_reply_unread_result_and_original_funding() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let definition_bank = bank();
    let banks = [bank(), bank()];
    let work_bank = bank();
    let capacity =
        ProducerFiniteCapacity::try_new(&owner, exchange_limits(), 2, 2, work_bank.clone())
            .unwrap();
    let provider = compiler(address, &definition_bank);
    let original_app = Rc::new(App::default());
    let mut original = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &banks[0],
        key(1, Generation::INITIAL),
        "alpha",
        11,
        original_app.clone(),
    );
    start(&mut original);
    original.publish(&mut owner, &mut budget()).unwrap();
    let peer = connect(address, "/alpha");
    delivering(&mut owner, &mut original);
    let old = original.key();
    let successor_app = Rc::new(App::default());
    let mut input = input_with_capacity(
        &owner,
        key(1, Generation::INITIAL.checked_next().unwrap()),
        &td(address, "successor", &["/successor"]),
        provider.clone(),
        "successor",
        33,
        successor_app.clone(),
        Some(&native),
        Some(&capacity),
    );
    input.expected = Some(old);
    let mut successor = admit(&owner, input, &banks[1]);
    start(&mut successor);
    successor.publish(&mut owner, &mut budget()).unwrap();
    assert!(owner.description(old).is_none());
    assert_eq!(capacity.active_work(), 1);
    let successor_peer = connect(address, "/successor");
    delivering(&mut owner, &mut successor);
    assert_eq!((capacity.active_work(), capacity.retained_work()), (2, 2));
    assert_value(
        &response(successor_peer, &mut owner, &mut [&mut successor]),
        33,
    );
    assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 2));
    // Revocation/close with zero credits cannot discard accepted work.
    original.begin_close();
    assert!(
        original
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .step(&mut cx(), &mut WorkBudget::new(), now())
            .is_pending()
    );
    assert_eq!(capacity.active_work(), 1);
    close(&mut original);
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 2));
    assert!(original.retire(&mut owner, &mut budget()).is_err());
    assert!(response(peer, &mut owner, &mut [&mut successor]).is_empty());
    assert_eq!(original_app.calls.get(), 1);
    assert_eq!(successor_app.calls.get(), 1);
    let old_result = take(&mut original);
    assert_result(&old_result, old, 11);
    assert_eq!(old_result.delivery(), FiniteDelivery::NotDelivered);
    assert!(matches!(
        original
            .participant_mut::<HttpRuntime>(0)
            .unwrap()
            .first_cause(),
        Some(CoreError::Cancelled(_))
    ));
    let new_result = take(&mut successor);
    assert_result(&new_result, successor.key(), 33);
    assert!(old_result.sequence() < new_result.sequence());
    original.retire(&mut owner, &mut budget()).unwrap();
    close(&mut successor);
    successor.retire(&mut owner, &mut budget()).unwrap();
    drop(original);
    drop(successor);
    drop(owner);
    drop(provider);
    drop(capacity); // Original leases remain reachable solely through outcomes.
    assert!(live(&banks[0]) > 0 && live(&banks[1]) > 0 && live(&work_bank) > 0);
    TcpListener::bind(address).unwrap();
    let old_result = Rc::new(old_result);
    let shared_result = old_result.clone();
    drop(old_result);
    assert_result(&shared_result, old, 11);
    drop(shared_result);
    assert_empty(&banks[0]);
    assert!(live(&banks[1]) > 0 && live(&work_bank) > 0);
    assert_result(&new_result, new_result.recipient().unwrap(), 33);
    drop(new_result);
    for bank in [&banks[1], &definition_bank, &work_bank] {
        assert_work_empty(bank);
    }
}

#[test]
fn shared_work_rejects_foreign_or_insufficient_capacity_without_losing_participation() {
    let owner = owner();
    let foreign_owner = super::owner();
    let address = tcp_addr();
    let native = listener(address);
    let accounts = bank();
    let work_bank = bank();
    assert!(
        ProducerFiniteCapacity::try_new(&owner, exchange_limits(), 0, 1, work_bank.clone())
            .is_err()
    );
    assert!(
        ProducerFiniteCapacity::try_new(&owner, exchange_limits(), 2, 1, work_bank.clone())
            .is_err()
    );
    assert_work_empty(&work_bank);
    let foreign = capacity(&foreign_owner, &work_bank, 1);
    let mut small_limits = exchange_limits();
    small_limits.output_body_bytes = 1;
    let small =
        ProducerFiniteCapacity::try_new(&owner, small_limits, 1, 1, work_bank.clone()).unwrap();
    let provider = compiler(address, &accounts);
    let mut input = input_with_listener(
        &owner,
        key(1, Generation::INITIAL),
        &td(address, "alpha", &["/alpha"]),
        provider.clone(),
        "alpha",
        11,
        Rc::new(App::default()),
        Some(&native),
    );
    // Construct the original native pair again from its actual prepared definition.
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        input.prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone(),
        input.instance.key(),
        AffordanceTarget::Property(Arc::from("alpha")),
        100,
    )
    .unwrap()
    .with_listener(&native)
    .unwrap();
    let runtime = RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(
            config.clone(),
            BindingId::new(999),
            BindingGeneration::INITIAL,
        ),
        config,
        &owner,
        exchange_limits(),
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    let runtime = runtime
        .with_capacity(foreign.clone())
        .err()
        .unwrap()
        .into_input();
    let runtime = runtime
        .with_capacity(small.clone())
        .err()
        .unwrap()
        .into_input();
    assert_eq!(runtime.state(), RawFiniteState::Starting);
    assert_eq!((foreign.active_work(), small.active_work()), (0, 0));
    TcpListener::bind(address).unwrap();
    let good = capacity(&owner, &work_bank, 1);
    let mut runtime = runtime.with_capacity(good.clone()).ok().unwrap();
    assert_eq!(
        runtime.step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    // Capacity cannot be substituted after native acquisition.
    let mut runtime = runtime
        .with_capacity(good.clone())
        .err()
        .unwrap()
        .into_input();
    runtime.begin_close();
    assert_eq!(
        runtime.step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::Closed)
    );
    drop(runtime);
    input.participants.clear();
    drop(input);
    drop(provider);
    drop(foreign);
    drop(small);
    drop(good);
    assert_work_empty(&work_bank);
    assert_work_empty(&accounts);
    TcpListener::bind(address).unwrap();
}

#[test]
fn shared_work_incomplete_native_ingress_does_not_reserve_engine_execution() {
    let mut owner = owner();
    let address = tcp_addr();
    let native = listener(address);
    let definition_bank = bank();
    let accounts = [bank(), bank()];
    let work_bank = bank();
    let capacity = capacity(&owner, &work_bank, 2);
    let provider = compiler(address, &definition_bank);
    let apps = [Rc::new(App::default()), Rc::new(App::default())];
    let mut first = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &accounts[0],
        key(1, Generation::INITIAL),
        "alpha",
        11,
        apps[0].clone(),
    );
    let mut second = publication(
        &owner,
        address,
        &native,
        &capacity,
        provider.clone(),
        &accounts[1],
        key(2, Generation::INITIAL),
        "beta",
        22,
        apps[1].clone(),
    );
    for exposure in [&mut first, &mut second] {
        start(exposure);
        exposure.publish(&mut owner, &mut budget()).unwrap();
    }
    let mut partial = TcpStream::connect(address).unwrap();
    partial.write_all(b"GET /alpha HTTP/1.1\r\n").unwrap();
    partial.set_nonblocking(true).unwrap();
    for _ in 0..20 {
        drive(&mut owner, &mut [&mut first, &mut second]);
    }
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 0));
    assert_eq!(apps[0].calls.get(), 0);
    assert_value(
        &response(
            connect(address, "/beta"),
            &mut owner,
            &mut [&mut first, &mut second],
        ),
        22,
    );
    assert_eq!(capacity.retained_work(), 1);
    partial
        .write_all(format!("Host: {address}\r\nConnection: close\r\n\r\n").as_bytes())
        .unwrap();
    assert_value(
        &response(partial, &mut owner, &mut [&mut first, &mut second]),
        11,
    );
    assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 2));
    for exposure in [&mut first, &mut second] {
        close(exposure);
        assert!(exposure.retire(&mut owner, &mut budget()).is_err());
        drop(take(exposure));
        exposure.retire(&mut owner, &mut budget()).unwrap();
    }
    drop(first);
    drop(second);
    drop(owner);
    drop(provider);
    drop(capacity);
    for bank in [&accounts[0], &accounts[1], &definition_bank, &work_bank] {
        assert_work_empty(bank);
    }
    TcpListener::bind(address).unwrap();
}

#[test]
fn shared_work_lost_participant_locally_aborts_original_admitted_work() {
    for dispatch in [false, true] {
        let mut owner = owner();
        let address = tcp_addr();
        let native = listener(address);
        let definition_bank = bank();
        let exposure_bank = bank();
        let work_bank = bank();
        let capacity = capacity(&owner, &work_bank, 1);
        let provider = compiler(address, &definition_bank);
        let app = Rc::new(App::default());
        let mut exposure = publication(
            &owner,
            address,
            &native,
            &capacity,
            provider.clone(),
            &exposure_bank,
            key(1, Generation::INITIAL),
            "alpha",
            11,
            app.clone(),
        );
        start(&mut exposure);
        exposure.publish(&mut owner, &mut budget()).unwrap();
        let peer = connect(address, "/alpha");
        let end = Instant::now() + Duration::from_secs(5);
        let expected = if dispatch {
            RawFiniteState::Delivering
        } else {
            RawFiniteState::Dispatching
        };
        while exposure.participant_mut::<HttpRuntime>(0).unwrap().state() != expected {
            drive(&mut owner, &mut [&mut exposure]);
            assert!(Instant::now() < end);
        }
        let key = exposure.key();
        assert_eq!((capacity.active_work(), capacity.retained_work()), (1, 1));
        drop(exposure);
        assert_eq!((capacity.active_work(), capacity.retained_work()), (0, 0));
        assert_eq!(app.calls.get(), usize::from(dispatch));
        assert!(owner.description(key).is_none());
        TcpListener::bind(address).unwrap();
        owner
            .retire_revoked_publication(key, &mut budget())
            .unwrap();
        drop(owner);
        drop(provider);
        drop(capacity);
        drop(peer);
        for bank in [&exposure_bank, &definition_bank, &work_bank] {
            assert_work_empty(bank);
        }
    }
}
