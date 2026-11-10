use super::*;

fn limited_bank(outstanding: u64) -> Arc<HostResourceAccounts> {
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(
        ResourceKind::PendingHandlerCallsGlobalMax,
        Some(outstanding)
    ));
    Arc::new(HostResourceAccounts::new(&limits))
}
fn outstanding(bank: &HostResourceAccounts) -> u64 {
    bank.account(ResourceKind::PendingHandlerCallsGlobalMax)
        .used()
}
fn associate(
    definition: &Arc<ConsumerDefinition>,
    owner: &ConsumerInstance,
    accounts: &Arc<HostResourceAccounts>,
    peers: &[&Peer],
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
    ConsumerAssociation::try_new(owner.reference(), vec![binding], vec![], accounts.clone())
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn start(
    association: &ConsumerAssociation,
    calls: &ConsumerCalls,
    peer: &Peer,
) -> CoreResult<ConsumerCall> {
    let port = peer.address.port().to_string();
    association.read_property_with_variables(
        calls,
        "level",
        &[("port", &port), ("item", "x"), ("q", "q")],
        Deadline::NONE,
        now(0),
        &mut work(),
    )
}

#[test]
fn eight_independent_associations_share_one_call_slot_and_keep_both_quota_domains() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owners: Vec<_> = (0..8).map(|_| consumer_instance()).collect();
        let accounts: Vec<_> = (0..8).map(|_| limited_bank(1)).collect();
        let associations: Vec<_> = owners
            .iter()
            .zip(&accounts)
            .map(|(owner, account)| associate(&definition, owner, account, &[&a, &b]))
            .collect();
        // Construction did not reserve any call, response or cleanup capacity.
        // The exact same prepared meaning and qualified mapping serve all eight.
        for account in &accounts {
            for resource in [
                ResourceKind::PendingHandlerCallsGlobalMax,
                ResourceKind::CleanupItemsMax,
                ResourceKind::CleanupBytesMax,
                ResourceKind::BindingResponseBufferBytesGlobalMax,
            ] {
                assert_eq!(account.account(resource).used(), 0, "idle {resource:?}");
            }
        }
        let work_bank = limited_bank(2);
        let other_bank = limited_bank(2);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        let mut first = start(&associations[0], &calls, &a).unwrap();
        assert!(matches!(
            start(&associations[1], &calls, &b),
            Err(CoreError::Backpressure(_))
        ));
        assert_eq!(
            (a.count(), b.count()),
            (0, 0),
            "saturation occurs before I/O"
        );
        let retained = finish(&mut first).await;
        assert_eq!(retained.consumer(), &owners[0].reference());
        assert_eq!(retained.result().unwrap().data().unwrap().body, b"7");
        drop(first);
        assert!(
            calls.is_idle(),
            "settlement transfers the complete funded call"
        );
        assert_eq!((outstanding(&work_bank), outstanding(&accounts[0])), (1, 1));
        let other_calls = ConsumerCalls::try_new(other_bank.clone(), 1).unwrap();
        // A different capacity domain cannot evade this association's quota.
        assert!(matches!(
            start(&associations[0], &other_calls, &b),
            Err(CoreError::LimitExceeded {
                resource: ResourceKind::PendingHandlerCallsGlobalMax,
                ..
            })
        ));
        assert_eq!(
            outstanding(&other_bank),
            0,
            "refusal rolls back work funding"
        );
        let mut second = start(&associations[1], &calls, &b).unwrap();
        let retained_second = finish(&mut second).await;
        drop(second);
        assert_eq!((outstanding(&work_bank), outstanding(&accounts[1])), (2, 1));
        // Idle storage alone cannot fund a third retained obligation.
        assert!(matches!(
            start(&associations[2], &calls, &a),
            Err(CoreError::LimitExceeded {
                resource: ResourceKind::PendingHandlerCallsGlobalMax,
                ..
            })
        ));
        assert_eq!(outstanding(&accounts[2]), 0);
        drop(retained);
        assert_eq!((outstanding(&work_bank), outstanding(&accounts[0])), (1, 0));
        // One association can use another capacity owner and another qualified
        // dynamic destination, without preparing or binding again.
        let mut third = start(&associations[0], &other_calls, &b).unwrap();
        let result = finish(&mut third).await;
        assert_eq!(result.consumer(), &owners[0].reference());
        drop((third, result));
        drop(retained_second);
        for (association, owner) in associations.iter().zip(&owners).skip(2) {
            let mut call = start(association, &calls, &a).unwrap();
            let result = finish(&mut call).await;
            assert_eq!(result.consumer(), &owner.reference());
            assert!(result.result().is_ok());
        }
        calls.begin_close();
        assert!(matches!(
            start(&associations[7], &calls, &a),
            Err(CoreError::StaleHandle(_))
        ));
        drop(calls);
        // Capacity-root close/Drop does not close the persistent associations.
        let mut call = start(&associations[7], &other_calls, &b).unwrap();
        let result = finish(&mut call).await;
        assert_eq!(result.consumer(), &owners[7].reference());
        assert!(result.result().is_ok());
        drop((call, result));
        assert_eq!((a.count(), b.count()), (7, 3));
        drop((associations, owners, other_calls, definition));
        for account in accounts.iter().chain([&backing, &work_bank, &other_bank]) {
            clean(account);
        }
    });
}

#[test]
fn late_http_result_survives_association_and_capacity_roots_and_cannot_move_to_replacement() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let original = consumer_instance();
        let identity = original.reference();
        let sibling = consumer_instance();
        let original_bank = limited_bank(1);
        let sibling_bank = limited_bank(1);
        let original_work = limited_bank(1);
        let sibling_work = limited_bank(1);
        let association = associate(&definition, &original, &original_bank, &[&a]);
        let other = associate(&definition, &sibling, &sibling_bank, &[&b]);
        let calls = ConsumerCalls::try_new(original_work.clone(), 1).unwrap();
        let other_calls = ConsumerCalls::try_new(sibling_work.clone(), 1).unwrap();
        let mut call = start(&association, &calls, &a).unwrap();
        // Stalled Engine validation retains its own call and accounts, but the
        // actual HTTP lane retires. Acquiring that lane proves native settlement.
        let stop = Instant::now() + Duration::from_secs(5);
        let mut sibling_call = loop {
            assert!(
                call.poll(
                    &mut cx(),
                    now(0),
                    &mut work().with_remaining(WorkClass::CodecInputBytes, 0)
                )
                .is_pending()
            );
            if let Ok(next) = start(&other, &other_calls, &b) {
                break next;
            }
            assert!(Instant::now() < stop);
            tokio::time::sleep(Duration::from_millis(1)).await;
        };
        assert_eq!(a.count(), 1);
        drop((association, original, calls));
        // Even a later cancellation cannot replace the earlier root/authority loss.
        call.cancel();
        let late = finish(&mut call).await;
        assert!(matches!(late.result(), Err(CoreError::StaleHandle(_))));
        assert_eq!(late.consumer(), &identity);
        assert!(
            matches!(late.native_outcome(), ConsumerNativeOutcome::Returned(Ok(output))
            if output.data().unwrap().body == b"7")
        );
        drop(call);
        assert!(original_work.account(ResourceKind::CleanupBytesMax).used() > 0);
        assert_eq!(outstanding(&original_bank), 1);
        let sibling_result = finish(&mut sibling_call).await;
        assert_eq!(sibling_result.consumer(), &sibling.reference());
        assert!(sibling_result.result().is_ok());
        drop((sibling_call, sibling_result));
        let replacement = consumer_instance();
        assert_eq!(replacement.reference().slot(), identity.slot());
        assert_ne!(replacement.reference(), identity);
        let replacement_bank = limited_bank(1);
        let replacement_association =
            associate(&definition, &replacement, &replacement_bank, &[&a]);
        let mut replacement_call = start(&replacement_association, &other_calls, &a).unwrap();
        let result = finish(&mut replacement_call).await;
        assert_eq!(result.consumer(), &replacement.reference());
        assert_eq!(late.consumer(), &identity);
        drop((result, replacement_call, late));
        clean(&original_work);
        clean(&original_bank);
        drop((
            other,
            sibling,
            replacement_association,
            replacement,
            other_calls,
            definition,
        ));
        for account in [&backing, &sibling_bank, &sibling_work, &replacement_bank] {
            clean(account);
        }
    });
}

thread_local! { static REENTER: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) }; }
struct ReenterPolicy;
impl ConsumerAccessService for ReenterPolicy {
    fn authorize(&self, _: BindingRegistrationIdentity, _: &str, _: Operation) -> CoreResult<()> {
        Ok(())
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        _: BindingRegistrationIdentity,
        _: &str,
        _: Operation,
    ) -> CoreResult<()> {
        let callback = REENTER.with(|pending| pending.borrow_mut().take());
        if let Some(callback) = callback {
            callback();
        }
        Ok(())
    }
    fn retained_bytes(&self) -> u64 {
        32
    }
    fn work(&self) -> u64 {
        1
    }
}

#[test]
fn shared_admission_reentry_reserves_capacity_and_closes_only_the_original_association() {
    io().block_on(async {
        let peer = Rc::new(Peer::new());
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let original = consumer_instance();
        let sibling = consumer_instance();
        let account = limited_bank(1);
        let sibling_account = limited_bank(1);
        let work_bank = limited_bank(2);
        let calls = Rc::new(ConsumerCalls::try_new(work_bank.clone(), 1).unwrap());
        let sibling_association =
            Rc::new(associate(&definition, &sibling, &sibling_account, &[&peer]));
        let access = ConsumerServiceOwner::retain_access(Arc::new(ReenterPolicy), &account)
            .unwrap_or_else(|_| panic!());
        let binding = definition
            .clone()
            .bind(access, account.clone(), &mut work())
            .unwrap_or_else(|_| panic!());
        let association = Rc::new(
            ConsumerAssociation::try_new(
                original.reference(),
                vec![binding],
                vec![],
                account.clone(),
            )
            .unwrap_or_else(|_| panic!()),
        );
        let (nested, other, closing, destination) = (
            calls.clone(),
            sibling_association.clone(),
            association.clone(),
            peer.clone(),
        );
        REENTER.with(|pending| {
            *pending.borrow_mut() = Some(Box::new(move || {
                assert!(!nested.is_idle());
                nested.step(&mut cx(), now(0), &mut work());
                assert!(matches!(
                    start(&other, &nested, &destination),
                    Err(CoreError::Backpressure(_))
                ));
                closing.begin_close();
            }))
        });
        assert!(matches!(
            start(&association, &calls, &peer),
            Err(CoreError::StaleHandle(_))
        ));
        assert!(calls.is_idle());
        assert_eq!((outstanding(&account), outstanding(&work_bank)), (0, 0));
        assert_eq!(peer.count(), 0);
        let call = start(&sibling_association, &calls, &peer).unwrap();
        drop(call); // Abandonment still leaves a complete owner for manual cleanup.
        calls.step(&mut cx(), now(0), &mut work());
        assert!(calls.is_idle());
        let mut call = start(&sibling_association, &calls, &peer).unwrap();
        let result = finish(&mut call).await;
        assert!(result.result().is_ok());
        assert_eq!(result.consumer(), &sibling.reference());
        drop((
            result,
            call,
            association,
            sibling_association,
            calls,
            definition,
            original,
            sibling,
        ));
        for account in [&backing, &account, &sibling_account, &work_bank] {
            clean(account);
        }
    });
}

#[test]
fn dropping_both_roots_with_abandoned_real_http_work_disposes_the_original_socket_and_funding() {
    io().block_on(async {
        let peer = Peer::new();
        peer.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = consumer_instance();
        let account = limited_bank(1);
        let work_bank = limited_bank(1);
        let association = associate(&definition, &owner, &account, &[&peer]);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        let mut call = start(&association, &calls, &peer).unwrap();
        received(&peer, &mut call).await;
        assert!(start(&association, &calls, &peer).is_err());
        drop((call, association, calls, owner, definition));
        clean(&work_bank);
        clean(&account);
        clean(&backing);
        let stop = Instant::now() + Duration::from_secs(5);
        while peer.closed.load(Ordering::Acquire) == 0 {
            assert!(
                Instant::now() < stop,
                "finite local disposal retained its socket"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(peer.count(), 1, "uncertain work was never replayed");
    });
}

#[test]
fn admitted_write_retains_original_operands_and_input_after_http_retirement_and_root_loss() {
    io().block_on(async {
        let peer = Peer::new();
        let backing = bank();
        let definition = definition(&dynamic_td(true), &backing).unwrap();
        let owner = consumer_instance();
        let identity = owner.reference();
        let account = limited_bank(1);
        let work_bank = limited_bank(1);
        let association = associate(&definition, &owner, &account, &[&peer]);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        let input = payload(b"7");
        let original_body = Arc::downgrade(&input.body);
        let port = peer.address.port().to_string();
        let mut item = String::from("x");
        let mut call = association
            .write_property_with_variables(
                &calls,
                "level",
                input,
                &[("port", &port), ("item", &item), ("q", "q")],
                Deadline::NONE,
                now(0),
                &mut work(),
            )
            .unwrap_or_else(|_| panic!());
        item.replace_range(.., "successor");
        let result = finish(&mut call).await;
        assert!(result.result().is_ok());
        drop((call, association, calls, definition, owner));
        assert_eq!(result.consumer(), &identity);
        assert_eq!(
            result.destination(),
            format!("http://{}/v/x?q=q", peer.address)
        );
        assert_eq!(
            result.uri_variables().collect::<Vec<_>>(),
            vec![("port", port.as_str()), ("item", "x"), ("q", "q")]
        );
        assert_eq!(result.input().unwrap().body, b"7");
        assert!(original_body.upgrade().is_some());
        assert_eq!(peer.count(), 1);
        assert_eq!(outstanding(&account), 1);
        drop(result);
        assert!(original_body.upgrade().is_none());
        for account in [&backing, &account, &work_bank] {
            clean(account);
        }
    });
}

#[test]
fn a_specialized_mapping_does_not_gain_dynamic_destination_authority_from_shared_capacity() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let backing = bank();
        let td = json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"fixed target",
            "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
            "properties":{"level":{"type":"integer","readOnly":true,
                "forms":[{"href":format!("http://{}/v/x?q=q", a.address)}]}}});
        let definition = definition(&td, &backing).unwrap();
        let owner = consumer_instance();
        let account = limited_bank(1);
        let work_bank = limited_bank(1);
        let association = associate(&definition, &owner, &account, &[&a, &b]);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        assert!(start(&association, &calls, &b).is_err());
        assert_eq!((a.count(), b.count()), (0, 0));
        assert_eq!((outstanding(&account), outstanding(&work_bank)), (0, 0));
        let mut call = association
            .read_property(&calls, "level", Deadline::NONE, now(0), &mut work())
            .unwrap();
        let result = finish(&mut call).await;
        assert_eq!(
            result.destination(),
            format!("http://{}/v/x?q=q", a.address)
        );
        assert!(result.result().is_ok());
        assert_eq!((a.count(), b.count()), (1, 0));
        drop((result, call, association, calls, owner, definition));
        for account in [&backing, &account, &work_bank] {
            clean(account);
        }
    });
}

struct ProgressWake(std::sync::atomic::AtomicUsize);
impl std::task::Wake for ProgressWake {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

#[test]
fn quiet_http_peer_needs_application_progress_after_passive_association_loss() {
    io().block_on(async {
        for (drop_association, abandon_handle) in [(false, false), (true, false), (false, true)] {
            let peer = Peer::new();
            peer.paused.store(true, Ordering::Release);
            let backing = bank();
            let definition = definition(&dynamic_td(false), &backing).unwrap();
            let owner = consumer_instance();
            let identity = owner.reference();
            let account = limited_bank(1);
            let work_bank = limited_bank(1);
            let mut association = Some(associate(&definition, &owner, &account, &[&peer]));
            let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
            let mut call = start(association.as_ref().unwrap(), &calls, &peer).unwrap();
            received(&peer, &mut call).await;
            let wakes = Arc::new(ProgressWake(std::sync::atomic::AtomicUsize::new(0)));
            let waker = Waker::from(wakes.clone());
            let mut driver = Context::from_waker(&waker);
            assert!(call.poll(&mut driver, now(0), &mut work()).is_pending());
            let funded = ResourceKind::ALL.map(|k| work_bank.account(k).used());
            wakes.0.store(0, Ordering::Release);
            if drop_association {
                drop(association.take());
            } else {
                association.as_ref().unwrap().begin_close();
            }
            // Closing this association does not revoke the underlying Consumer
            // or run a registered driver. The next driver turn is the caller's
            // obligation even if the native peer never sends another byte.
            assert!(identity.is_current());
            assert_eq!(wakes.0.load(Ordering::Acquire), 0);
            assert!(!calls.is_idle());
            assert_eq!(peer.closed.load(Ordering::Acquire), 0);
            assert!(
                call.poll(&mut driver, now(0), &mut WorkBudget::new())
                    .is_pending()
            );
            assert_eq!(
                ResourceKind::ALL.map(|k| work_bank.account(k).used()),
                funded
            );
            assert_eq!((outstanding(&account), outstanding(&work_bank)), (1, 1));
            if abandon_handle {
                drop(call);
                calls.step(&mut driver, now(0), &mut WorkBudget::new());
                assert!(!calls.is_idle());
                assert_eq!(outstanding(&work_bank), 1);
                calls.step(&mut driver, now(0), &mut work());
            } else {
                // The same cleanup owner can advance a retained handle. Native
                // response activity is not needed to cancel the HTTP exchange.
                calls.step(&mut driver, now(0), &mut work());
                let Poll::Ready(result) = call.poll(&mut driver, now(0), &mut work()) else {
                    panic!("funded HTTP local cancellation did not settle");
                };
                assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
                assert_eq!(result.consumer(), &identity);
                assert!(matches!(
                    result.native_outcome(),
                    ConsumerNativeOutcome::Cancelled { .. }
                ));
                drop(call);
                assert_eq!((outstanding(&account), outstanding(&work_bank)), (1, 1));
                drop(result);
            }
            assert!(calls.is_idle());
            assert_eq!((outstanding(&account), outstanding(&work_bank)), (0, 0));
            let stop = Instant::now() + Duration::from_secs(5);
            while peer.closed.load(Ordering::Acquire) == 0 {
                assert!(
                    Instant::now() < stop,
                    "explicit progress retained the silent socket"
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            assert_eq!(peer.count(), 1);
            drop((association, calls, definition, owner));
            for account in [&backing, &account, &work_bank] {
                clean(account);
            }
        }
    });
}

#[cfg(target_os = "linux")]
#[path = "observable_capacity.rs"]
mod observable;
