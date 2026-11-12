#![cfg(all(feature = "std", target_has_atomic = "ptr"))]
use clinkz_wot_core::{
    CoreError, ThingSlotId, consumer::ConsumerInstance, host_accounts::HostResourceAccounts,
};
use clinkz_wot_foundation::*;
use clinkz_wot_servient_finite::consumer::ConsumerOwner;
use std::sync::Arc;
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 10000))
}
fn root(slot: u32, generation: Generation) -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(slot), generation))
}
fn clean(bank: &HostResourceAccounts) {
    for kind in ResourceKind::ALL {
        assert_eq!(bank.account(kind).used(), 0, "{kind:?}");
    }
}
#[test]
fn managed_registration_refusal_and_wrong_owner_snapshot_preserve_current_authority() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let a = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let b = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let id = a.register_thing("urn:pump", &mut work()).unwrap();
    let other = b.register_thing("urn:pump", &mut work()).unwrap();
    assert_ne!(id.consumer(), other.consumer());
    assert!(matches!(
        a.remove_instance(&other, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(matches!(
        a.register_thing("urn:other", &mut WorkBudget::new()),
        Err(CoreError::Backpressure(_))
    ));
    assert!(matches!(
        a.register_thing("urn:other", &mut work()),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    assert!(id.is_current() && other.is_current());
    a.remove_thing("urn:pump", &mut work()).unwrap();
    let fresh = a.register_thing("urn:pump", &mut work()).unwrap();
    assert_ne!(fresh.consumer(), id.consumer());
    assert!(matches!(
        a.remove_instance(&id, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    drop((a, b, id, other, fresh));
    clean(&bank);
}

#[test]
fn owner_generates_replacements_and_refusal_cannot_invalidate_the_original() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let id = owner.register_thing("urn:pump", &mut work()).unwrap();
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(matches!(
        owner.replace_instance(&id, &mut WorkBudget::new()),
        Err(CoreError::Backpressure(_))
    ));
    assert!(id.is_current());
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    let next = owner.replace_thing("urn:pump", &mut work()).unwrap();
    assert!(!id.is_current() && next.is_current());
    assert_eq!(next.consumer().slot().slot(), id.consumer().slot().slot());
    assert_eq!(
        next.consumer().slot().generation(),
        id.consumer().slot().generation().checked_next().unwrap()
    );
    assert!(matches!(
        owner.replace_instance(&id, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    drop((id, next, owner));
    clean(&bank);
}

#[test]
fn id_lookup_recovers_management_and_distinguishes_current_from_conditional_removal() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let a = ConsumerOwner::try_new(bank.clone(), 2, 0).unwrap();
    let b = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    for invalid in ["", &"x".repeat(2049)] {
        assert!(matches!(
            a.register_thing(invalid, &mut work()),
            Err(CoreError::Validation(_))
        ));
    }
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(matches!(
        a.register_thing("urn:pump", &mut WorkBudget::new()),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    drop(a.register_thing("urn:pump", &mut work()).unwrap());
    drop(b.register_thing("urn:pump", &mut work()).unwrap());
    let current = a.instance("urn:pump", &mut work()).unwrap();
    assert_eq!(current.thing_id(), "urn:pump");
    assert_ne!(
        current.consumer(),
        b.instance("urn:pump", &mut work()).unwrap().consumer()
    );
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(matches!(
        a.register_thing("urn:pump", &mut work()),
        Err(CoreError::Validation(_))
    ));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    assert!(matches!(
        a.remove_thing("urn:missing", &mut work()),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        a.remove_thing("urn:pump", &mut WorkBudget::new()),
        Err(CoreError::Backpressure(_))
    ));
    assert!(current.is_current());
    let replaced = a.replace_instance(&current, &mut work()).unwrap();
    assert!(!current.is_current());
    assert_eq!(
        a.instance("urn:pump", &mut work()).unwrap().consumer(),
        replaced.consumer()
    );
    a.remove_thing("urn:pump", &mut work()).unwrap();
    assert!(!replaced.is_current());
    drop(a.register_thing("urn:pump", &mut work()).unwrap());
    assert!(matches!(
        a.remove_instance(&replaced, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    // ID removal deliberately targets the current registration, unlike removal
    // conditioned on the stale snapshot. No generation guesses command intent.
    a.remove_thing("urn:pump", &mut work()).unwrap();
    assert!(b.instance("urn:pump", &mut work()).unwrap().is_current());
    drop((a, b, current, replaced));
    clean(&bank);
}

#[test]
fn internally_funded_authority_outlives_registration_while_a_passive_reference_exists() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let handle = owner.register_thing("urn:pump", &mut work()).unwrap();
    let reference = handle.consumer();
    owner.remove_thing("urn:pump", &mut work()).unwrap();
    drop((handle, owner));
    assert!(!reference.is_current());
    assert_ne!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    drop(reference);
    clean(&bank);
}

#[test]
fn internal_slot_tickets_recycle_without_reviving_stale_work_or_requiring_caller_coordinates() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 2, 0).unwrap();
    let first = owner.register_thing("urn:one", &mut work()).unwrap();
    let other = owner.register_thing("urn:two", &mut work()).unwrap();
    owner.remove_thing("urn:one", &mut work()).unwrap();
    let replacement = owner.register_thing("urn:new", &mut work()).unwrap();
    assert_eq!(
        replacement.consumer().slot().slot(),
        first.consumer().slot().slot()
    );
    assert_eq!(
        replacement.consumer().slot().generation(),
        first.consumer().slot().generation().checked_next().unwrap()
    );
    assert_ne!(replacement.consumer(), first.consumer());
    assert!(!first.is_current() && other.is_current());
    assert!(matches!(
        owner.remove_instance(&first, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    // Explicit coordinates still work for a standalone representation, independently
    // of the managed owner's private allocator and authority.
    let standalone = root(u32::MAX, Generation::INITIAL);
    let passive = standalone.reference();
    assert!(passive.is_current());
    drop(standalone);
    assert!(!passive.is_current());
    assert!(replacement.is_current());
    drop((owner, first, other, replacement, passive));
    clean(&bank);
}

#[test]
fn empty_owner_does_not_preallocate_its_configured_population_limit() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1_000_000, 1_000_000).unwrap();
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
    let first = owner.register_thing("urn:meter:1", &mut work()).unwrap();
    let used = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(used > 0 && used < 4096);
    drop((first, owner));
    clean(&bank);
}

#[test]
fn long_common_prefix_ids_stay_indexed_through_out_of_order_insert_remove_and_reuse() {
    const COUNT: usize = 4096;
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1_000_000, 0).unwrap();
    let key = |i| format!("urn:factory:common-prefix:device:{i:08}");
    // A permutation spreads insertion through the table rather than appending
    // already-sorted IDs. Both the ID and slot indexes must remain consistent.
    for i in 0..COUNT {
        drop(
            owner
                .register_thing(&key(i * 2053 % COUNT), &mut work())
                .unwrap(),
        );
    }
    for i in [0, 1, COUNT / 2, COUNT - 2, COUNT - 1] {
        let mut small = WorkBudget::new().with_remaining(WorkClass::QueueOperations, 1024);
        let handle = owner.instance(&key(i), &mut small).unwrap();
        assert_eq!(handle.thing_id(), key(i).as_str());
        assert!(small.remaining(WorkClass::QueueOperations) > 0);
    }
    let stale = owner.instance(&key(17), &mut work()).unwrap();
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    assert!(matches!(
        owner.register_thing(&key(17), &mut work()),
        Err(CoreError::Validation(_))
    ));
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    for i in (0..COUNT).step_by(17) {
        owner.remove_thing(&key(i), &mut work()).unwrap();
    }
    for i in (0..COUNT).step_by(17) {
        drop(owner.register_thing(&key(i), &mut work()).unwrap());
    }
    assert!(matches!(
        owner.remove_instance(&stale, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    for i in 0..COUNT {
        let mut small = WorkBudget::new().with_remaining(WorkClass::QueueOperations, 1024);
        assert!(owner.instance(&key(i), &mut small).unwrap().is_current());
    }
    drop((stale, owner));
    clean(&bank);
}

#[test]
fn failed_node_admission_refunds_partial_acquisition_and_does_not_consume_a_free_ticket() {
    let kind = ResourceKind::EngineLiveBytesGlobalMax;
    let mut saw_partial = false;
    for extra in (0..=1024).step_by(8) {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let owner = ConsumerOwner::try_new(bank.clone(), 2, 0).unwrap();
        let old = owner.register_thing("urn:old", &mut work()).unwrap();
        let other = owner.register_thing("urn:other", &mut work()).unwrap();
        owner.remove_thing("urn:old", &mut work()).unwrap();
        let account = bank.account(kind);
        let hold = bank
            .reserve(kind, account.limit() - account.used() - extra)
            .unwrap();
        let before = bank.account(kind).used();
        let new = match owner.register_thing("urn:new", &mut work()) {
            Ok(h) => {
                drop(hold);
                h
            }
            Err(e) => {
                assert!(matches!(e, CoreError::Backpressure(_)));
                assert!(other.is_current());
                assert_eq!(bank.account(kind).used(), before);
                saw_partial |= bank.account(kind).peak() > before;
                drop(hold);
                owner.register_thing("urn:new", &mut work()).unwrap()
            }
        };
        assert_eq!(new.consumer().slot().slot(), old.consumer().slot().slot());
        assert_eq!(
            new.consumer().slot().generation(),
            old.consumer().slot().generation().checked_next().unwrap()
        );
        drop((new, old, other, owner));
        clean(&bank);
    }
    assert!(saw_partial);
}

#[test]
fn mixed_public_lifecycle_has_the_same_fixed_credit_at_small_and_large_populations() {
    use std::collections::BTreeMap;
    for population in [64, 4096] {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let owner = ConsumerOwner::try_new(bank.clone(), 1_000_000, 0).unwrap();
        let key = |i| format!("urn:mixed:{i:05}");
        let lifecycle = || work().with_remaining(WorkClass::QueueOperations, 4096);
        let mut model = BTreeMap::new();
        let mut retired = Vec::new();
        for i in 0..population {
            let id = key(i);
            model.insert(
                id.clone(),
                owner.register_thing(&id, &mut lifecycle()).unwrap(),
            );
        }
        let mut seed = 13u64;
        for turn in 0..6000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let id = key((seed >> 32) as usize % (population * 2));
            match seed & 3 {
                0 => {
                    if model.contains_key(&id) {
                        assert!(matches!(
                            owner.register_thing(&id, &mut lifecycle()),
                            Err(CoreError::Validation(_))
                        ));
                    } else {
                        model.insert(
                            id.clone(),
                            owner.register_thing(&id, &mut lifecycle()).unwrap(),
                        );
                    }
                }
                1 => {
                    if let Some(old) = model.remove(&id) {
                        owner.remove_thing(&id, &mut lifecycle()).unwrap();
                        assert!(!old.is_current());
                        // Recreate immediately as well as leaving holes during
                        // other turns. Old snapshots may never target the new root.
                        if turn % 3 == 0 {
                            let new = owner.register_thing(&id, &mut lifecycle()).unwrap();
                            assert_ne!(old.consumer(), new.consumer());
                            model.insert(id.clone(), new);
                        }
                        if retired.len() == 32 {
                            retired.remove(0);
                        }
                        retired.push(old);
                    } else {
                        assert!(matches!(
                            owner.remove_thing(&id, &mut lifecycle()),
                            Err(CoreError::NotFound(_))
                        ));
                    }
                }
                2 => {
                    let mut credit =
                        WorkBudget::new().with_remaining(WorkClass::QueueOperations, 1024);
                    match (model.get(&id), owner.instance(&id, &mut credit)) {
                        (Some(expected), Ok(actual)) => {
                            assert_eq!(expected.consumer(), actual.consumer())
                        }
                        (None, Err(CoreError::NotFound(_))) => {}
                        _ => panic!("model disagrees with lookup"),
                    }
                }
                _ => {
                    if let Some(old) = model.get(&id) {
                        assert!(matches!(
                            owner.remove_thing(&id, &mut WorkBudget::new()),
                            Err(CoreError::Backpressure(_))
                        ));
                        assert!(old.is_current());
                        let new = owner.replace_instance(old, &mut lifecycle()).unwrap();
                        assert!(!old.is_current());
                        model.insert(id, new);
                    }
                }
            }
        }
        for old in &retired {
            assert!(matches!(
                owner.remove_instance(old, &mut lifecycle()),
                Err(CoreError::StaleHandle(_))
            ));
        }
        owner.begin_close();
        assert!(model.values().all(|h| !h.is_current()));
        drop((retired, model, owner));
        clean(&bank);
    }
}
