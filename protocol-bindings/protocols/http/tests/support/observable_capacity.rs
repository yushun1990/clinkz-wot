//! Actual HTTP execution with application-owned event-driven control readiness.
use super::*;
use clinkz_wot_core::host_change::HostChangeDriver;
use std::future::poll_fn;
use tokio::io::unix::AsyncFd;

fn observable_owner(bank: &Arc<HostResourceAccounts>, drivers: usize) -> ConsumerInstance {
    ConsumerInstance::try_new_observable(
        ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL),
        bank,
        drivers,
    )
    .unwrap()
}
fn observable_association(
    definition: &Arc<ConsumerDefinition>,
    owner: &ConsumerInstance,
    accounts: &Arc<HostResourceAccounts>,
    peers: &[&Peer],
    drivers: usize,
) -> ConsumerAssociation {
    let access = ConsumerServiceOwner::retain_access(
        Arc::new(DestinationPolicy {
            instance: owner.reference(),
            allowed: peers
                .iter()
                .map(|p| format!("http://{}/v/x?q=q", p.address))
                .collect(),
            denied: Arc::new(AtomicBool::new(false)),
            seen: Default::default(),
        }),
        accounts,
    )
    .unwrap_or_else(|_| panic!());
    let binding = definition
        .clone()
        .bind(access, accounts.clone(), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(Arc::ptr_eq(binding.definition(), definition));
    ConsumerAssociation::try_new_observable(
        owner.reference(),
        vec![binding],
        vec![],
        accounts.clone(),
        drivers,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn driver(bank: &Arc<HostResourceAccounts>) -> AsyncFd<HostChangeDriver> {
    AsyncFd::new(HostChangeDriver::try_new(bank).unwrap()).unwrap()
}
async fn changed(driver: &AsyncFd<HostChangeDriver>) {
    loop {
        let mut ready = driver.readable().await.unwrap();
        match ready.try_io(|fd| {
            if fd.get_ref().acknowledge()? {
                Ok(())
            } else {
                Err(std::io::ErrorKind::WouldBlock.into())
            }
        }) {
            Ok(Ok(())) => return,
            Ok(Err(error)) => panic!("control backend failed: {error}"),
            Err(_) => continue,
        }
    }
}
async fn change(driver: &AsyncFd<HostChangeDriver>) {
    tokio::time::timeout(Duration::from_secs(5), changed(driver))
        .await
        .expect("revocation did not make the parked driver runnable");
}
async fn driven_finish(
    call: &mut ConsumerCall,
    driver: &AsyncFd<HostChangeDriver>,
) -> ConsumerSettlement {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut notification = std::pin::pin!(changed(driver));
        poll_fn(|cx| {
            if notification.as_mut().poll(cx).is_ready() {
                notification.set(changed(driver));
                // Retain a runnable turn until the replacement readiness future
                // has registered interest. Each turn still surveys original work.
                cx.waker().wake_by_ref();
            }
            call.poll(cx, now(0), &mut work())
        })
        .await
    })
    .await
    .expect("event-driven cleanup stalled")
}
async fn closed(peer: &Peer) {
    let stop = Instant::now() + Duration::from_secs(5);
    while peer.closed.load(Ordering::Acquire) == 0 {
        assert!(
            Instant::now() < stop,
            "local retirement retained the native socket"
        );
        // This observes the remote fixture; it never polls Engine work.
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn silent_http_revocation_wakes_without_periodic_driver_polling_for_raii_and_explicit_release() {
    io().block_on(async {
        // Removal and replacement drop the old association through normal Rust
        // collection operations. Parent release also works across threads.
        for how in 0..4 {
            let peer = Peer::new();
            peer.paused.store(true, Ordering::Release);
            let backing = bank();
            let definition = definition(&dynamic_td(false), &backing).unwrap();
            let mut owner = Some(observable_owner(&backing, 2));
            let identity = owner.as_ref().unwrap().reference();
            let (account, capacity) = (limited_bank(2), limited_bank(2));
            let driver = driver(&backing);
            let calls =
                ConsumerCalls::try_new_driven(capacity.clone(), 1, driver.get_ref().signal())
                    .unwrap();
            let mut associations = std::collections::BTreeMap::new();
            associations.insert(
                0,
                observable_association(&definition, owner.as_ref().unwrap(), &account, &[&peer], 2),
            );
            let mut call = start(&associations[&0], &calls, &peer).unwrap();
            received(&peer, &mut call).await;
            // Register both the native-I/O task and control readiness before
            // revocation. There is no timer/periodic Engine poll after this point.
            let result = {
                let settled = driven_finish(&mut call, &driver);
                tokio::pin!(settled);
                assert!(
                    std::future::poll_fn(|cx| Poll::Ready(settled.as_mut().poll(cx).is_pending()))
                        .await
                );
                let notification = changed(&driver);
                tokio::pin!(notification);
                assert!(
                    std::future::poll_fn(|cx| Poll::Ready(
                        notification.as_mut().poll(cx).is_pending()
                    ))
                    .await
                );
                match how {
                    0 => {
                        drop(associations.remove(&0));
                    }
                    1 => {
                        let replacement = observable_association(
                            &definition,
                            owner.as_ref().unwrap(),
                            &account,
                            &[&peer],
                            2,
                        );
                        drop(associations.insert(0, replacement));
                    }
                    2 => owner.as_ref().unwrap().revoke(),
                    _ => {
                        std::thread::spawn(move || drop(owner.take()))
                            .join()
                            .unwrap();
                        owner = None;
                    }
                }
                if how < 2 {
                    assert!(identity.is_current());
                } else {
                    assert!(!identity.is_current());
                }
                tokio::time::timeout(Duration::from_secs(5), notification)
                    .await
                    .unwrap();
                settled.await
            };
            assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
            assert_eq!(result.consumer(), &identity);
            assert!(matches!(
                result.native_outcome(),
                ConsumerNativeOutcome::Cancelled { .. }
            ));
            assert_eq!(
                result.destination(),
                format!("http://{}/v/x?q=q", peer.address)
            );
            closed(&peer).await;
            assert_eq!(peer.count(), 1);
            drop((result, call));
            assert_eq!((outstanding(&account), outstanding(&capacity)), (0, 0));
            if how == 1 {
                peer.paused.store(false, Ordering::Release);
                let mut successor = start(&associations[&0], &calls, &peer).unwrap();
                assert!(
                    driven_finish(&mut successor, &driver)
                        .await
                        .result()
                        .is_ok()
                );
            }
            drop((associations, calls, owner, definition, identity, driver));
            for account in [&backing, &account, &capacity] {
                clean(account);
            }
        }
    });
}

#[test]
fn the_existing_association_later_enrolls_d2_and_notifies_both_active_drivers() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        b.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = observable_owner(&backing, 2);
        let identity = owner.reference();
        let (account, capacity1, capacity2) = (limited_bank(2), limited_bank(2), limited_bank(2));
        let association = observable_association(&definition, &owner, &account, &[&a, &b], 2);
        let source = association.change_source().unwrap();
        let d1 = driver(&backing);
        let calls1 = ConsumerCalls::try_new_driven(capacity1.clone(), 1, d1.get_ref().signal()).unwrap();
        let mut first = start(&association, &calls1, &a).unwrap();
        // Provision D2 after the original authority and mapping already exist.
        let d2 = driver(&backing);
        let calls2 = ConsumerCalls::try_new_driven(capacity2.clone(), 1, d2.get_ref().signal()).unwrap();
        // Keep D1's original returned native result in Engine validation while
        // reusing its retired HTTP lane for D2. Neither result changes identity.
        let stop = Instant::now() + Duration::from_secs(5);
        let mut second = loop {
            assert!(first.poll(&mut cx(), now(0), &mut work().with_remaining(WorkClass::CodecInputBytes, 0)).is_pending());
            if let Ok(next) = start(&association, &calls2, &b) { break next; }
            assert!(Instant::now() < stop);
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        received(&b, &mut second).await;
        assert_eq!(source.retained_relationships(), 2);
        drop(association);
        change(&d1).await;
        change(&d2).await;
        let late = driven_finish(&mut first, &d1).await;
        assert!(matches!(late.result(), Err(CoreError::StaleHandle(_))));
        assert!(matches!(late.native_outcome(), ConsumerNativeOutcome::Returned(Ok(value)) if value.data().unwrap().body == b"7"));
        let cancelled = driven_finish(&mut second, &d2).await;
        assert!(matches!(cancelled.result(), Err(CoreError::StaleHandle(_))));
        assert_eq!((late.consumer(), cancelled.consumer()), (&identity, &identity));
        assert_eq!(source.retained_relationships(), 0, "handoff ends dependencies even while settlements remain funded");
        closed(&b).await;
        drop((late, cancelled, first, second, calls1, calls2, d1, d2, source, definition, identity, owner));
        for account in [&backing, &account, &capacity1, &capacity2] { clean(account); }
    });
}

#[test]
fn notification_capacity_refuses_before_native_effects_and_zero_credit_keeps_the_original_owner() {
    io().block_on(async {
        let peer = Peer::new();
        peer.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = observable_owner(&backing, 1);
        let (account, capacity) = (limited_bank(2), limited_bank(2));
        let association = observable_association(&definition, &owner, &account, &[&peer], 1);
        let source = association.change_source().unwrap();
        let (d1, d2) = (driver(&backing), driver(&backing));
        let calls =
            ConsumerCalls::try_new_driven(capacity.clone(), 2, d1.get_ref().signal()).unwrap();
        // Different poll and cleanup recipients require two funded records per
        // source. Refusal rolls back the already acquired cleanup relationship.
        assert!(matches!(
            start(
                &association,
                &calls.using_driver(&d2.get_ref().signal()),
                &peer
            ),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(source.retained_relationships(), 0);
        assert_eq!(
            (outstanding(&account), outstanding(&capacity), peer.count()),
            (0, 0, 0)
        );
        let mut call = start(&association, &calls, &peer).unwrap();
        received(&peer, &mut call).await;
        assert_eq!(source.retained_relationships(), 1);
        association.begin_close();
        change(&d1).await;
        let held = ResourceKind::ALL.map(|k| capacity.account(k).used());
        assert!(
            call.poll(&mut cx(), now(0), &mut WorkBudget::new())
                .is_pending()
        );
        calls.step(&mut cx(), now(0), &mut WorkBudget::new());
        assert_eq!(ResourceKind::ALL.map(|k| capacity.account(k).used()), held);
        assert_eq!(peer.closed.load(Ordering::Acquire), 0);
        assert_eq!((outstanding(&account), outstanding(&capacity)), (1, 1));
        // A receipt isn't execution credit. The credit owner retains runnable
        // debt and explicitly requests another turn when supplying new credit.
        d1.get_ref().signal().request_turn().unwrap();
        change(&d1).await;
        let result = driven_finish(&mut call, &d1).await;
        assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
        closed(&peer).await;
        drop((
            result,
            call,
            calls,
            association,
            source,
            definition,
            owner,
            d1,
            d2,
        ));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}

#[test]
fn a_separate_poll_driver_retires_while_the_prepaid_cleanup_recipient_remains_reachable() {
    io().block_on(async {
        let peer = Peer::new();
        peer.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = observable_owner(&backing, 2);
        let (account, capacity) = (limited_bank(1), limited_bank(1));
        let association = observable_association(&definition, &owner, &account, &[&peer], 2);
        let source = association.change_source().unwrap();
        let (cleanup, polling) = (driver(&backing), driver(&backing));
        let calls =
            ConsumerCalls::try_new_driven(capacity.clone(), 1, cleanup.get_ref().signal()).unwrap();
        let mut call = start(
            &association,
            &calls.using_driver(&polling.get_ref().signal()),
            &peer,
        )
        .unwrap();
        received(&peer, &mut call).await;
        assert_eq!(source.retained_relationships(), 2);
        drop(call);
        change(&cleanup).await;
        assert_eq!(source.retained_relationships(), 1);
        assert!(!polling.get_ref().acknowledge().unwrap());
        calls.step(&mut cx(), now(0), &mut WorkBudget::new());
        assert!(!calls.is_idle());
        drop(association);
        change(&cleanup).await;
        assert!(
            !polling.get_ref().acknowledge().unwrap(),
            "retired polling dependency received a source notice"
        );
        calls.step(&mut cx(), now(0), &mut work());
        assert!(calls.is_idle());
        closed(&peer).await;
        assert_eq!(source.retained_relationships(), 0);
        drop((calls, source, definition, owner, cleanup, polling));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}

#[test]
fn aggregate_native_progress_notifies_the_distinct_original_result_driver() {
    io().block_on(async {
        let peer = Peer::new();
        peer.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = observable_owner(&backing, 2);
        let (account, capacity) = (limited_bank(1), limited_bank(1));
        let association = observable_association(&definition, &owner, &account, &[&peer], 2);
        let source = association.change_source().unwrap();
        let (cleanup, polling) = (driver(&backing), driver(&backing));
        let calls =
            ConsumerCalls::try_new_driven(capacity.clone(), 1, cleanup.get_ref().signal()).unwrap();
        let mut call = start(
            &association,
            &calls.using_driver(&polling.get_ref().signal()),
            &peer,
        )
        .unwrap();
        received(&peer, &mut call).await;
        assert!(!polling.get_ref().acknowledge().unwrap());
        peer.paused.store(false, Ordering::Release);
        {
            let notification = changed(&polling);
            tokio::pin!(notification);
            // Only the aggregate driver polls native I/O now. A native wake can
            // move to it; the original result recipient still needs a terminal turn.
            tokio::time::timeout(
                Duration::from_secs(5),
                poll_fn(|cx| {
                    calls.step(cx, now(0), &mut work());
                    notification.as_mut().poll(cx)
                }),
            )
            .await
            .expect("aggregate completion hid the original result driver");
        }
        assert!(source.is_current());
        assert_eq!(
            source.retained_relationships(),
            2,
            "unread terminal work still owns its dependencies"
        );
        calls.step(&mut cx(), now(0), &mut work());
        calls.step(&mut cx(), now(0), &mut work());
        assert!(
            !polling.get_ref().acknowledge().unwrap(),
            "terminal progress repeatedly posted without a transition"
        );
        let result = driven_finish(&mut call, &polling).await;
        assert!(result.result().is_ok());
        assert_eq!(result.consumer(), &owner.reference());
        assert_eq!(source.retained_relationships(), 0);
        drop((
            result,
            call,
            calls,
            association,
            source,
            definition,
            owner,
            cleanup,
            polling,
        ));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}

#[test]
fn abandoned_a_is_locally_disposed_with_b_retained_in_both_control_loss_orders() {
    io().block_on(async {
        for abandon_first in [false, true] {
            let peer = Peer::new();
            let backing = bank();
            let definition = definition(&dynamic_td(false), &backing).unwrap();
            let owner = observable_owner(&backing, 1);
            let identity = owner.reference();
            let (account, capacity) = (limited_bank(2), limited_bank(2));
            let association = observable_association(&definition, &owner, &account, &[&peer], 1);
            let driver = driver(&backing);
            let calls =
                ConsumerCalls::try_new_driven(capacity.clone(), 2, driver.get_ref().signal())
                    .unwrap();
            let mut b = start(&association, &calls, &peer).unwrap();
            let result_b = driven_finish(&mut b, &driver).await;
            assert_eq!(result_b.result().unwrap().data().unwrap().body, b"7");
            peer.paused.store(true, Ordering::Release);
            let mut a = start(&association, &calls, &peer).unwrap();
            // The peer has already seen B. Wait specifically for A's request.
            let stop = Instant::now() + Duration::from_secs(5);
            while peer.count() < 2 {
                assert!(a.poll(&mut cx(), now(0), &mut work()).is_pending());
                assert!(Instant::now() < stop);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            assert_eq!((outstanding(&account), outstanding(&capacity)), (2, 2));
            if abandon_first {
                drop(a);
                drop(calls);
            } else {
                drop(calls);
                drop(a);
            }
            // B's handle still retains the shared State. No incremental driver
            // remains for A; qualified local disposal must have retired it now.
            assert_eq!((outstanding(&account), outstanding(&capacity)), (1, 1));
            closed(&peer).await;
            assert_eq!(result_b.consumer(), &identity);
            assert_eq!(result_b.result().unwrap().data().unwrap().body, b"7");
            assert_eq!(peer.count(), 2, "abandoned work was replayed");
            drop((b, result_b));
            assert_eq!((outstanding(&account), outstanding(&capacity)), (0, 0));
            drop((association, definition, identity, owner, driver));
            for account in [&backing, &account, &capacity] {
                clean(account);
            }
        }
    });
}

#[test]
fn abandoned_http_a_retires_while_live_http_b_keeps_reachable_original_cleanup() {
    io().block_on(async {
        for abandon_first in [false, true] {
            let (a, b) = (Peer::new(), Peer::new());
            a.paused.store(true, Ordering::Release);
            b.paused.store(true, Ordering::Release);
            let backing = bank();
            // Independent native owners can both have I/O pending; Engine
            // capacity does not create or duplicate either physical lane.
            let definitions = [
                definition(&dynamic_td(false), &backing).unwrap(),
                definition(&dynamic_td(false), &backing).unwrap(),
            ];
            let owner = observable_owner(&backing, 1);
            let identity = owner.reference();
            let (account_a, account_b, capacity) =
                (limited_bank(1), limited_bank(1), limited_bank(2));
            let association_a =
                observable_association(&definitions[0], &owner, &account_a, &[&a], 1);
            let association_b =
                observable_association(&definitions[1], &owner, &account_b, &[&b], 1);
            let driver = driver(&backing);
            let calls =
                ConsumerCalls::try_new_driven(capacity.clone(), 2, driver.get_ref().signal())
                    .unwrap();
            let mut call_b = start(&association_b, &calls, &b).unwrap();
            received(&b, &mut call_b).await;
            let mut call_a = start(&association_a, &calls, &a).unwrap();
            received(&a, &mut call_a).await;
            if abandon_first {
                drop(call_a);
                drop(calls);
            } else {
                drop(calls);
                drop(call_a);
            }
            assert_eq!(
                (
                    outstanding(&account_a),
                    outstanding(&account_b),
                    outstanding(&capacity)
                ),
                (0, 1, 1)
            );
            closed(&a).await;
            assert_eq!(
                b.closed.load(Ordering::Acquire),
                0,
                "surviving B was disposed instead of retained"
            );
            // A's native occupancy, not just its socket, was retired before the
            // original work accounts could fund another admission.
            let successor_capacity =
                ConsumerCalls::try_new_driven(capacity.clone(), 1, driver.get_ref().signal())
                    .unwrap();
            let successor = start(&association_a, &successor_capacity, &a).unwrap();
            drop((successor, successor_capacity));
            assert_eq!(outstanding(&capacity), 1);
            let result_b = driven_finish(&mut call_b, &driver).await;
            assert!(matches!(result_b.result(), Err(CoreError::StaleHandle(_))));
            assert_eq!(result_b.consumer(), &identity);
            assert_eq!(
                result_b.destination(),
                format!("http://{}/v/x?q=q", b.address)
            );
            closed(&b).await;
            drop((call_b, result_b));
            assert_eq!((a.count(), b.count()), (1, 1));
            drop((
                association_a,
                association_b,
                definitions,
                identity,
                owner,
                driver,
            ));
            for account in [&backing, &account_a, &account_b, &capacity] {
                clean(account);
            }
        }
    });
}
