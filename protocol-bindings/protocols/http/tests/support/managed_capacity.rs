//! Managed lifecycle through real HTTP, independently provisioned work owners.
use super::*;
use clinkz_wot_servient_finite::consumer::{
    ConsumerAssociation, ConsumerInstanceHandle, ConsumerOwner,
};

fn instance(owner: &ConsumerOwner, slot: u32) -> ConsumerInstanceHandle {
    owner
        .register_thing(&format!("urn:managed:{slot}"), &mut work())
        .unwrap()
}
fn association(
    owner: &ConsumerOwner,
    instance: &ConsumerInstanceHandle,
    definition: &Arc<ConsumerDefinition>,
    account: &Arc<HostResourceAccounts>,
    peers: &[&Peer],
) -> ConsumerAssociation {
    let standalone = associate_ref(definition, instance.consumer(), account, peers);
    owner
        .set_configuration(instance.thing_id(), "default", standalone, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
async fn closed(peer: &Peer) {
    let stop = Instant::now() + Duration::from_secs(5);
    while peer.closed.load(Ordering::Acquire) == 0 {
        assert!(
            Instant::now() < stop,
            "original native socket did not retire"
        );
        // Observe only the peer, never manufacture another Engine progress turn.
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn association_refusal_preserves_the_original_owner_and_removal_releases_capacity() {
    io().block_on(async {
        let peer = Peer::new();
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 2, 1).unwrap();
        let first = instance(&owner, 0);
        let second = instance(&owner, 1);
        let account = limited_bank(1);
        let original = associate_ref(&definition, first.consumer(), &account, &[&peer]);
        let view = original.handle();
        let before = backing
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used();
        let refused = owner
            .set_configuration(second.thing_id(), "default", original, &mut work())
            .err()
            .expect("a sibling cannot adopt the wrong original parent");
        assert!(matches!(refused.error().error(), CoreError::Validation(_)));
        let (original, _) = refused.into_parts();
        assert!(view.is_current());
        let refused = owner
            .set_configuration(
                first.thing_id(),
                "default",
                original,
                &mut WorkBudget::new(),
            )
            .err()
            .expect("lookup needs work credit");
        let (original, _) = refused.into_parts();
        let mut no_retirement_credit =
            WorkBudget::new().with_remaining(WorkClass::QueueOperations, 10000);
        let refused = owner
            .set_configuration(
                first.thing_id(),
                "default",
                original,
                &mut no_retirement_credit,
            )
            .err()
            .expect("adoption must prepay retirement");
        let (original, _) = refused.into_parts();
        assert!(view.is_current() && first.is_current() && second.is_current());
        assert_eq!(
            backing
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used(),
            before
        );
        let registered = owner
            .set_configuration(first.thing_id(), "default", original, &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let candidate = associate_ref(&definition, second.consumer(), &account, &[&peer]);
        let candidate_view = candidate.handle();
        let refused = owner
            .set_configuration(second.thing_id(), "default", candidate, &mut work())
            .err()
            .expect("the configured association limit is global to this owner");
        assert!(matches!(
            refused.error().error(),
            CoreError::Backpressure(_)
        ));
        let (candidate, _) = refused.into_parts();
        assert!(registered.is_current() && candidate_view.is_current());
        assert!(matches!(
            owner.remove_configuration(first.thing_id(), "default", &mut WorkBudget::new()),
            Err(CoreError::Backpressure(_))
        ));
        assert!(registered.is_current(), "refused removal has no effect");
        owner
            .remove_configuration(first.thing_id(), "default", &mut work())
            .unwrap();
        assert!(!view.is_current() && first.is_current());
        let current = owner
            .set_configuration(second.thing_id(), "default", candidate, &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        assert!(matches!(
            owner.remove_configuration_if_current(second.thing_id(), "default", &view, &mut work()),
            Err(CoreError::StaleHandle(_))
        ));
        let capacity = limited_bank(1);
        let calls = ConsumerCalls::try_new(capacity.clone(), 1).unwrap();
        let mut call = start(&current, &calls, &peer).unwrap();
        assert!(finish(&mut call).await.result().is_ok());
        drop((
            call,
            calls,
            current,
            registered,
            view,
            candidate_view,
            first,
            second,
            owner,
            definition,
        ));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}

#[test]
fn managed_handle_drop_keeps_eight_registrations_usable_with_one_shared_call_slot() {
    io().block_on(async {
        let peer = Peer::new();
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 8, 8).unwrap();
        let accounts: Vec<_> = (0..8).map(|_| limited_bank(1)).collect();
        let mut views = Vec::new();
        let mut identities = Vec::new();
        for (slot, account) in accounts.iter().enumerate() {
            let id = instance(&owner, slot as u32);
            let standalone = associate_ref(&definition, id.consumer(), account, &[&peer]);
            // Even a view issued before adoption cannot retain revocation power.
            let view = standalone.handle();
            let returned = owner
                .set_configuration(id.thing_id(), "default", standalone, &mut work())
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
            identities.push(id.consumer());
            drop(returned);
            drop(id); // the sole application instance handle
            assert!(view.is_current());
            assert_eq!(
                outstanding(account),
                0,
                "registration prepays no idle calls"
            );
            views.push(view);
        }
        let work_bank = limited_bank(1);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        for (view, identity) in views.iter().zip(&identities) {
            let mut call = start(view, &calls, &peer).unwrap();
            let result = finish(&mut call).await;
            assert_eq!(result.consumer(), identity);
            assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
            drop((result, call));
            assert!(calls.is_idle());
        }
        let other_bank = limited_bank(1);
        let other = ConsumerCalls::try_new(other_bank.clone(), 1).unwrap();
        drop(calls); // capacity loss does not remove managed registrations
        let mut call = start(&views[0], &other, &peer).unwrap();
        let result = finish(&mut call).await;
        assert_eq!(result.consumer(), &identities[0]);
        drop((call, result));
        assert_eq!(peer.count(), 9);
        owner.begin_close();
        assert!(views.iter().all(|v| !v.is_current()));
        assert!(identities.iter().all(|id| !id.is_current()));
        assert!(matches!(
            start(&views[0], &other, &peer),
            Err(CoreError::StaleHandle(_))
        ));
        // Retained stale views keep funded tombstone headers, not permission.
        assert_ne!(
            accounts[0]
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used(),
            0
        );
        drop((views, identities, owner, other, definition));
        for account in accounts.iter().chain([&backing, &work_bank, &other_bank]) {
            clean(account);
        }
    });
}

#[test]
fn explicit_facet_and_instance_replacement_preserve_siblings_and_reject_stale_handles() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 1, 2).unwrap();
        let old = instance(&owner, 0);
        let account = limited_bank(2);
        let first = association(&owner, &old, &definition, &account, &[&a, &b]);
        let sibling = configure(
            &owner,
            old.thing_id(),
            "sibling",
            &definition,
            &account,
            &[&a, &b],
        );
        let replacement = associate_ref(&definition, old.consumer(), &account, &[&a, &b]);
        let current = owner
            .replace_configuration(old.thing_id(), "default", &first, replacement, &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        assert!(!first.is_current());
        assert!(sibling.is_current() && current.is_current() && old.is_current());
        assert!(matches!(
            owner.remove_configuration_if_current(old.thing_id(), "default", &first, &mut work()),
            Err(CoreError::StaleHandle(_))
        ));
        owner
            .remove_configuration(old.thing_id(), "sibling", &mut work())
            .unwrap();
        assert!(current.is_current() && old.is_current());
        let work_bank = limited_bank(2);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        a.paused.store(true, Ordering::Release);
        let mut call = start(&current, &calls, &a).unwrap();
        received(&a, &mut call).await;
        let new = owner.replace_instance(&old, &mut work()).unwrap();
        assert!(!old.is_current() && !current.is_current());
        assert!(new.is_current());
        assert_eq!(new.consumer().slot().generation().get(), 2);
        assert!(matches!(
            owner.remove_instance(&old, &mut work()),
            Err(CoreError::StaleHandle(_))
        ));
        let fresh = association(&owner, &new, &definition, &account, &[&b]);
        assert!(matches!(
            start(&first, &calls, &b),
            Err(CoreError::StaleHandle(_))
        ));
        assert!(matches!(
            start(&current, &calls, &b),
            Err(CoreError::StaleHandle(_))
        ));
        let retired = finish(&mut call).await;
        assert_eq!(retired.consumer(), &old.consumer());
        assert!(matches!(retired.result(), Err(CoreError::StaleHandle(_))));
        closed(&a).await;
        drop((retired, call));
        let mut call = start(&fresh, &calls, &b).unwrap();
        let retained = finish(&mut call).await;
        owner.remove_instance(&new, &mut work()).unwrap();
        assert!(!new.is_current() && !fresh.is_current());
        assert_eq!(retained.consumer(), &new.consumer());
        assert_eq!(retained.result().unwrap().data().unwrap().body, b"7");
        assert_eq!(
            outstanding(&account),
            1,
            "settlement retains its original quota"
        );
        drop((
            retained, call, calls, first, sibling, current, fresh, old, new, owner, definition,
        ));
        for account in [&backing, &account, &work_bank] {
            clean(account);
        }
    });
}

#[test]
fn silent_peer_removal_fences_without_a_wake_and_keeps_funded_cleanup_reachable() {
    io().block_on(async {
        let peer = Peer::new();
        peer.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 1, 1).unwrap();
        let id = instance(&owner, 0);
        let account = limited_bank(1);
        let view = association(&owner, &id, &definition, &account, &[&peer]);
        let work_bank = limited_bank(1);
        let calls = ConsumerCalls::try_new(work_bank.clone(), 1).unwrap();
        let mut call = start(&view, &calls, &peer).unwrap();
        received(&peer, &mut call).await;
        let wakes = Arc::new(ProgressWake(std::sync::atomic::AtomicUsize::new(0)));
        let waker = Waker::from(wakes.clone());
        let mut driver = Context::from_waker(&waker);
        assert!(call.poll(&mut driver, now(0), &mut work()).is_pending());
        wakes.0.store(0, Ordering::Release);
        owner.remove_instance(&id, &mut work()).unwrap();
        assert_eq!(wakes.0.load(Ordering::Acquire), 0);
        assert!(matches!(
            start(&view, &calls, &peer),
            Err(CoreError::StaleHandle(_))
        ));
        assert!(
            call.poll(&mut driver, now(0), &mut WorkBudget::new())
                .is_pending()
        );
        assert!(!calls.is_idle());
        assert_eq!((outstanding(&account), outstanding(&work_bank)), (1, 1));
        assert_eq!(
            peer.closed.load(Ordering::Acquire),
            0,
            "removal is not drain"
        );
        // The application supplies this lifecycle turn; no peer response occurs.
        calls.step(&mut driver, now(0), &mut work());
        let Poll::Ready(result) = call.poll(&mut driver, now(0), &mut work()) else {
            panic!("funded original cleanup was not reachable");
        };
        assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
        assert_eq!(result.consumer(), &id.consumer());
        closed(&peer).await;
        assert_eq!(outstanding(&account), 1);
        drop((result, call, calls, view, id, owner, definition));
        for account in [&backing, &account, &work_bank] {
            clean(account);
        }
    });
}

#[test]
fn abandoned_a_is_locally_disposed_with_active_or_settled_b_in_both_loss_orders() {
    io().block_on(async {
        for settled_b in [false, true] {
            for abandon_first in [false, true] {
                let (a, b) = (Peer::new(), Peer::new());
                a.paused.store(true, Ordering::Release);
                b.paused.store(!settled_b, Ordering::Release);
                let backing = bank();
                let da = definition(&dynamic_td(false), &backing).unwrap();
                let db = definition(&dynamic_td(false), &backing).unwrap();
                let owner = ConsumerOwner::try_new(backing.clone(), 2, 2).unwrap();
                let ia = instance(&owner, 0);
                let ib = instance(&owner, 1);
                let (ba, bb, capacity) = (limited_bank(1), limited_bank(1), limited_bank(2));
                let va = association(&owner, &ia, &da, &ba, &[&a]);
                let vb = association(&owner, &ib, &db, &bb, &[&b]);
                let calls = ConsumerCalls::try_new(capacity.clone(), 2).unwrap();
                let mut ca = start(&va, &calls, &a).unwrap();
                let mut cb = start(&vb, &calls, &b).unwrap();
                received(&a, &mut ca).await;
                let retained_b = if settled_b {
                    Some(finish(&mut cb).await)
                } else {
                    received(&b, &mut cb).await;
                    None
                };
                owner.remove_instance(&ia, &mut work()).unwrap();
                if abandon_first {
                    drop(ca);
                    drop(calls);
                } else {
                    drop(calls);
                    drop(ca);
                }
                closed(&a).await;
                assert_eq!(outstanding(&ba), 0, "B must not retain abandoned A");
                assert_eq!(outstanding(&capacity), 1);
                if let Some(result) = &retained_b {
                    assert_eq!(result.result().unwrap().data().unwrap().body, b"7");
                } else {
                    assert_eq!(b.closed.load(Ordering::Acquire), 0);
                    let result = finish(&mut cb).await;
                    assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
                    assert_eq!(result.consumer(), &ib.consumer());
                    closed(&b).await;
                    drop(result);
                }
                assert!(
                    ib.is_current(),
                    "work loss does not remove the managed instance"
                );
                drop((cb, retained_b, va, vb, ia, ib, owner, da, db));
                for account in [&backing, &ba, &bb, &capacity] {
                    clean(account);
                }
            }
        }
    });
}

#[test]
fn owner_drop_fences_views_while_original_calls_still_have_their_own_progress() {
    io().block_on(async {
        let peer = Peer::new();
        peer.paused.store(true, Ordering::Release);
        let backing = bank();
        let definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 1, 1).unwrap();
        let id = instance(&owner, 0);
        let account = limited_bank(1);
        let view = association(&owner, &id, &definition, &account, &[&peer]);
        let capacity = limited_bank(1);
        let calls = ConsumerCalls::try_new(capacity.clone(), 1).unwrap();
        let mut call = start(&view, &calls, &peer).unwrap();
        received(&peer, &mut call).await;
        drop(owner);
        assert!(!view.is_current() && !id.is_current());
        let result = finish(&mut call).await;
        assert_eq!(result.consumer(), &id.consumer());
        assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
        closed(&peer).await;
        drop((call, result, calls, view, id, definition));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}

fn configure(
    owner: &ConsumerOwner,
    id: &str,
    configuration: &str,
    definition: &Arc<ConsumerDefinition>,
    account: &Arc<HostResourceAccounts>,
    peers: &[&Peer],
) -> ConsumerAssociation {
    let original = owner.instance(id, &mut work()).unwrap().consumer();
    let raw = associate_ref(definition, original, account, peers);
    owner
        .set_configuration(id, configuration, raw, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn read_by_id(
    owner: &ConsumerOwner,
    id: &str,
    configuration: &str,
    calls: &ConsumerCalls,
    peer: &Peer,
) -> CoreResult<ConsumerCall> {
    let port = peer.address.port().to_string();
    owner.read_property_with_variables(
        id,
        configuration,
        calls,
        "level",
        &[("port", &port), ("item", "x"), ("q", "q")],
        Deadline::NONE,
        now(0),
        &mut work(),
    )
}

#[test]
fn td_id_alone_recovers_read_and_targeted_removal_after_every_thing_view_is_dropped() {
    io().block_on(async {
        let peer = Peer::new();
        let backing = bank();
        let mut source = dynamic_td(false);
        source["id"] = json!("urn:pump:primary");
        let definition = definition(&source, &backing).unwrap();
        let source: Thing = serde_json::from_value(source).unwrap();
        let id = source.id.as_ref().unwrap().as_str().to_owned();
        let owner = ConsumerOwner::try_new(backing.clone(), 1, 1).unwrap();
        drop(clinkz_wot_servient::consumer::register_thing(&owner, &source, &mut work()).unwrap());
        let account = limited_bank(1);
        drop(configure(
            &owner,
            &id,
            "readonly",
            &definition,
            &account,
            &[&peer],
        ));
        drop(source);
        // No Thing snapshot, association view, internal slot or external handle
        // table is kept. The library's own owner resolves every operation.
        let capacity = limited_bank(1);
        let calls = ConsumerCalls::try_new(capacity.clone(), 1).unwrap();
        assert!(matches!(
            read_by_id(&owner, &id, "misspelled", &calls, &peer),
            Err(CoreError::NotFound(_))
        ));
        assert_eq!(peer.count(), 0, "no configuration fallback");
        let mut call = read_by_id(&owner, &id, "readonly", &calls, &peer).unwrap();
        assert_eq!(
            finish(&mut call)
                .await
                .result()
                .unwrap()
                .data()
                .unwrap()
                .body,
            b"7"
        );
        drop(call);
        owner.remove_thing(&id, &mut work()).unwrap();
        assert!(matches!(
            read_by_id(&owner, &id, "readonly", &calls, &peer),
            Err(CoreError::NotFound(_))
        ));
        drop((owner, calls, definition));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}

#[test]
fn same_thing_configurations_keep_permissions_and_accepted_context_across_update() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        let backing = bank();
        let shared = definition(&dynamic_td(false), &backing).unwrap();
        let updated = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 1, 2).unwrap();
        let id = "urn:pump";
        let original = owner.register_thing(id, &mut work()).unwrap().consumer();
        let (readonly_bank, admin_bank, updated_bank, capacity) = (
            limited_bank(1),
            limited_bank(1),
            limited_bank(1),
            limited_bank(2),
        );
        let readonly = configure(&owner, id, "readonly", &shared, &readonly_bank, &[&a]);
        let admin = configure(&owner, id, "admin", &shared, &admin_bank, &[&b]);
        assert_eq!(readonly.consumer(), admin.consumer());
        let calls = ConsumerCalls::try_new(capacity.clone(), 2).unwrap();
        assert!(matches!(
            read_by_id(&owner, id, "readonly", &calls, &b),
            Err(CoreError::Security { .. })
        ));
        assert!(matches!(
            read_by_id(&owner, id, "admin", &calls, &a),
            Err(CoreError::Security { .. })
        ));
        assert_eq!((a.count(), b.count()), (0, 0));
        b.paused.store(true, Ordering::Release);
        let mut old = read_by_id(&owner, id, "admin", &calls, &b).unwrap();
        received(&b, &mut old).await;
        // Replacement fits at full association capacity and changes only this
        // selected configuration. Already accepted work retains b and its bank.
        drop(configure(
            &owner,
            id,
            "admin",
            &updated,
            &updated_bank,
            &[&a],
        ));
        assert!(!admin.is_current() && readonly.is_current() && original.is_current());
        assert_eq!(
            owner.instance(id, &mut work()).unwrap().consumer(),
            original
        );
        assert!(matches!(
            start(&admin, &calls, &b),
            Err(CoreError::StaleHandle(_))
        ));
        let result = finish(&mut old).await;
        assert!(matches!(result.result(), Err(CoreError::StaleHandle(_))));
        assert_eq!(result.consumer(), &original);
        assert_eq!(
            result.destination(),
            format!("http://{}/v/x?q=q", b.address)
        );
        closed(&b).await;
        assert_eq!(
            (outstanding(&admin_bank), outstanding(&updated_bank)),
            (1, 0)
        );
        drop((result, old));
        let mut current = read_by_id(&owner, id, "admin", &calls, &a).unwrap();
        assert!(finish(&mut current).await.result().is_ok());
        drop(current);
        owner
            .remove_configuration(id, "admin", &mut work())
            .unwrap();
        assert!(matches!(
            owner.association(id, "admin", &mut work()),
            Err(CoreError::NotFound(_))
        ));
        assert!(readonly.is_current());
        let mut call = read_by_id(&owner, id, "readonly", &calls, &a).unwrap();
        assert!(finish(&mut call).await.result().is_ok());
        drop((
            call, readonly, admin, original, owner, calls, shared, updated,
        ));
        for account in [
            &backing,
            &readonly_bank,
            &admin_bank,
            &updated_bank,
            &capacity,
        ] {
            clean(account);
        }
    });
}

#[test]
fn same_id_recreation_does_not_retarget_old_calls_views_or_configuration_inputs() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        a.paused.store(true, Ordering::Release);
        let backing = bank();
        let first_definition = definition(&dynamic_td(false), &backing).unwrap();
        let next_definition = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 1, 1).unwrap();
        let id = "urn:pump";
        let first = owner.register_thing(id, &mut work()).unwrap();
        let (first_bank, next_bank, capacity) = (limited_bank(1), limited_bank(1), limited_bank(2));
        let stale = configure(&owner, id, "default", &first_definition, &first_bank, &[&a]);
        let prepared_for_old = associate_ref(&next_definition, first.consumer(), &next_bank, &[&b]);
        let calls = ConsumerCalls::try_new(capacity.clone(), 2).unwrap();
        let mut old = read_by_id(&owner, id, "default", &calls, &a).unwrap();
        received(&a, &mut old).await;
        owner.remove_thing(id, &mut work()).unwrap();
        let next = owner.register_thing(id, &mut work()).unwrap();
        assert_ne!(next.consumer(), first.consumer());
        assert!(matches!(
            owner.remove_instance(&first, &mut work()),
            Err(CoreError::StaleHandle(_))
        ));
        let refused = owner
            .set_configuration(id, "default", prepared_for_old, &mut work())
            .err()
            .unwrap();
        assert!(matches!(refused.error().error(), CoreError::Validation(_)));
        drop(refused);
        drop(configure(
            &owner,
            id,
            "default",
            &next_definition,
            &next_bank,
            &[&b],
        ));
        assert!(matches!(
            start(&stale, &calls, &b),
            Err(CoreError::StaleHandle(_))
        ));
        let mut new = read_by_id(&owner, id, "default", &calls, &b).unwrap();
        let retained = finish(&mut new).await;
        assert!(retained.result().is_ok());
        let retired = finish(&mut old).await;
        assert!(matches!(retired.result(), Err(CoreError::StaleHandle(_))));
        assert_eq!(retired.consumer(), &first.consumer());
        closed(&a).await;
        drop((retired, old));
        assert_eq!(
            (
                outstanding(&first_bank),
                outstanding(&next_bank),
                outstanding(&capacity)
            ),
            (0, 1, 1)
        );
        assert!(next.is_current());
        assert_eq!(retained.consumer(), &next.consumer());
        owner.remove_thing(id, &mut work()).unwrap();
        assert!(
            retained.result().is_ok(),
            "returned results retain original evidence"
        );
        drop((
            retained,
            new,
            stale,
            first,
            next,
            owner,
            calls,
            first_definition,
            next_definition,
        ));
        for account in [&backing, &first_bank, &next_bank, &capacity] {
            clean(account);
        }
    });
}

#[test]
fn real_http_id_and_configuration_admission_uses_small_credit_at_different_populations() {
    io().block_on(async {
        for population in [64, 4096] {
            let peer = Peer::new();
            let backing = bank();
            let definition = definition(&dynamic_td(false), &backing).unwrap();
            let owner = ConsumerOwner::try_new(backing.clone(), 1_000_000, 64).unwrap();
            let key = |i| format!("urn:factory:meter:{i:08}");
            for i in (0..population).rev() {
                drop(owner.register_thing(&key(i), &mut work()).unwrap());
            }
            let id = key(population / 2);
            let account = limited_bank(1);
            for i in (0..32).rev() {
                drop(configure(
                    &owner,
                    &id,
                    &format!("access-{i:03}"),
                    &definition,
                    &account,
                    &[&peer],
                ));
            }
            owner
                .remove_configuration(&id, "access-016", &mut work())
                .unwrap();
            assert!(matches!(
                owner.association(&id, "access-016", &mut work()),
                Err(CoreError::NotFound(_))
            ));
            let capacity = limited_bank(1);
            let calls = ConsumerCalls::try_new(capacity.clone(), 1).unwrap();
            for configuration in ["access-000", "access-015", "access-031"] {
                // The same fixed credit serves both populations. It is far
                // below the previous full-population reservation, with no
                // adjustment to policy/native/cleanup work or configured limits.
                let mut credit = work().with_remaining(WorkClass::QueueOperations, 1024);
                let port = peer.address.port().to_string();
                let mut call = owner
                    .read_property_with_variables(
                        &id,
                        configuration,
                        &calls,
                        "level",
                        &[("port", &port), ("item", "x"), ("q", "q")],
                        Deadline::NONE,
                        now(0),
                        &mut credit,
                    )
                    .unwrap();
                assert!(credit.remaining(WorkClass::QueueOperations) > 0);
                assert_eq!(
                    finish(&mut call)
                        .await
                        .result()
                        .unwrap()
                        .data()
                        .unwrap()
                        .body,
                    b"7"
                );
                drop(call);
            }
            assert_eq!(peer.count(), 3);
            drop((calls, owner, definition));
            for account in [&backing, &account, &capacity] {
                clean(account);
            }
        }
    });
}

#[test]
fn real_http_mixed_removal_recreation_and_configuration_update_preserves_original_work() {
    io().block_on(async {
        let (a, b) = (Peer::new(), Peer::new());
        a.paused.store(true, Ordering::Release);
        let backing = bank();
        let da = definition(&dynamic_td(false), &backing).unwrap();
        let db = definition(&dynamic_td(false), &backing).unwrap();
        let owner = ConsumerOwner::try_new(backing.clone(), 1_000_000, 2).unwrap();
        for i in 0..2048 {
            drop(
                owner
                    .register_thing(&format!("urn:mixed:{i:05}"), &mut work())
                    .unwrap(),
            );
        }
        let id = "urn:mixed:01024";
        let lifecycle = || work().with_remaining(WorkClass::QueueOperations, 4096);
        let account = limited_bank(2);
        let capacity = limited_bank(2);
        let calls = ConsumerCalls::try_new(capacity.clone(), 2).unwrap();
        for _ in 0..12 {
            let original = owner.instance(id, &mut lifecycle()).unwrap();
            let old_view = configure(&owner, id, "default", &da, &account, &[&a]);
            let closed_before = a.closed.load(Ordering::Acquire);
            let received_before = a.count();
            let mut old = read_by_id(&owner, id, "default", &calls, &a).unwrap();
            let stop = Instant::now() + Duration::from_secs(5);
            while a.count() == received_before {
                assert!(old.poll(&mut cx(), now(0), &mut work()).is_pending());
                assert!(Instant::now() < stop, "no new HTTP request");
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            owner.remove_thing(id, &mut lifecycle()).unwrap();
            let current = owner.register_thing(id, &mut lifecycle()).unwrap();
            assert!(!original.is_current() && !old_view.is_current());
            assert_ne!(original.consumer(), current.consumer());
            drop(configure(&owner, id, "default", &db, &account, &[&b]));
            let mut new = read_by_id(&owner, id, "default", &calls, &b).unwrap();
            let retained = finish(&mut new).await;
            assert!(retained.result().is_ok());
            let retired = finish(&mut old).await;
            assert_eq!(retired.consumer(), &original.consumer());
            assert!(matches!(retired.result(), Err(CoreError::StaleHandle(_))));
            let stop = Instant::now() + Duration::from_secs(5);
            while a.closed.load(Ordering::Acquire) == closed_before {
                assert!(Instant::now() < stop);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            drop((retired, old));
            assert_eq!((outstanding(&account), outstanding(&capacity)), (1, 1));
            // Reconfigure and remove the new facet while its already-returned
            // result stays owned. Its evidence/quota cannot move to that facet.
            drop(configure(&owner, id, "default", &da, &account, &[&a]));
            owner
                .remove_configuration(id, "default", &mut lifecycle())
                .unwrap();
            assert!(retained.result().is_ok());
            drop((new, retained, original, current, old_view));
            assert!(calls.is_idle());
        }
        assert_eq!((a.count(), b.count()), (12, 12));
        drop((owner, calls, da, db));
        for account in [&backing, &account, &capacity] {
            clean(account);
        }
    });
}
