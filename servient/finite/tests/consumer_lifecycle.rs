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
fn managed_refusal_returns_the_same_authority_and_wrong_owner_cannot_remove_it() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let a = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let b = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let id = a
        .register_instance(root(0, Generation::INITIAL), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let other = b
        .register_instance(root(0, Generation::INITIAL), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_ne!(id.consumer(), other.consumer());
    assert!(matches!(
        a.remove_instance(&other, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    assert!(id.is_current() && other.is_current());
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    let original = root(1, Generation::INITIAL);
    let reference = original.reference();
    let error = a
        .register_instance(original, &mut WorkBudget::new())
        .err()
        .unwrap();
    let (returned, _) = error.into_parts();
    assert_eq!(reference, returned.reference());
    assert!(reference.is_current());
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        before
    );
    let error = a.register_instance(returned, &mut work()).err().unwrap();
    assert!(matches!(error.error().error(), CoreError::Backpressure(_)));
    drop(error);
    assert!(!reference.is_current());
    a.remove_instance(&id, &mut work()).unwrap();
    assert!(!id.is_current());
    assert!(other.is_current());
    // A fresh issuer cannot be reached by an old handle even if the caller
    // deliberately reuses the same numeric coordinate after full removal.
    let fresh = a
        .register_instance(root(0, Generation::INITIAL), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_ne!(fresh.consumer(), id.consumer());
    assert!(matches!(
        a.remove_instance(&id, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    assert!(fresh.is_current());
    drop((a, b));
    assert!(!fresh.is_current() && !other.is_current());
    drop((fresh, id, other, reference));
    clean(&bank);
}
#[test]
fn replacement_requires_checked_next_generation_and_cannot_wrap() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 1, 0).unwrap();
    let id = owner
        .register_instance(root(0, Generation::INITIAL), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
    for invalid in [
        root(0, Generation::INITIAL),
        root(1, Generation::INITIAL.checked_next().unwrap()),
    ] {
        let original = invalid.reference();
        let error = owner
            .replace_instance(&id, invalid, &mut work())
            .err()
            .unwrap();
        let (returned, _) = error.into_parts();
        assert_eq!(returned.reference(), original);
        assert!(original.is_current() && id.is_current());
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            before
        );
    }
    owner.remove_instance(&id, &mut work()).unwrap();
    let maximum = owner
        .register_instance(root(0, Generation::new(u32::MAX).unwrap()), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let error = owner
        .replace_instance(&maximum, root(0, Generation::INITIAL), &mut work())
        .err()
        .unwrap();
    assert!(matches!(error.error().error(), CoreError::Validation(_)));
    assert!(maximum.is_current());
    drop((error, owner, id, maximum));
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
    assert_eq!(current.thing_id(), Some("urn:pump"));
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
    let next = root(
        current.consumer().slot().slot().get(),
        Generation::INITIAL.checked_next().unwrap(),
    );
    let replaced = a
        .replace_instance(&current, next, &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
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
fn automatic_slots_reuse_free_coordinates_even_with_a_live_maximum_raw_slot() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = ConsumerOwner::try_new(bank.clone(), 3, 0).unwrap();
    let high = owner
        .register_instance(root(u32::MAX, Generation::INITIAL), &mut work())
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let first = owner.register_thing("urn:meter:1", &mut work()).unwrap();
    assert_eq!(first.consumer().slot().slot().get(), 0);
    let second = owner.register_thing("urn:meter:2", &mut work()).unwrap();
    assert_eq!(second.consumer().slot().slot().get(), 1);
    owner.remove_thing("urn:meter:1", &mut work()).unwrap();
    let replacement = owner.register_thing("urn:meter:1", &mut work()).unwrap();
    assert_eq!(
        replacement.consumer().slot().slot(),
        first.consumer().slot().slot()
    );
    assert_ne!(replacement.consumer(), first.consumer());
    assert!(matches!(
        owner.remove_instance(&first, &mut work()),
        Err(CoreError::StaleHandle(_))
    ));
    assert!(high.is_current() && replacement.is_current());
    drop((owner, high, first, second, replacement));
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
        assert_eq!(handle.thing_id(), Some(key(i).as_str()));
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
fn staged_index_growth_refusal_restores_both_indexes_and_original_input_funding() {
    let kind = ResourceKind::EngineLiveBytesGlobalMax;
    let mut saw_partial_growth_refusal = false;
    // Sweep real remaining byte limits instead of depending on private layouts.
    // Some refusals happen after one index has acquired its replacement buffer.
    for extra in (0..=2048).step_by(8) {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let owner = ConsumerOwner::try_new(bank.clone(), 8, 0).unwrap();
        let a = owner
            .register_instance(root(0, Generation::INITIAL), &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let b = owner
            .register_instance(root(u32::MAX, Generation::INITIAL), &mut work())
            .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let account = bank.account(kind);
        let hold = bank
            .reserve(kind, account.limit() - account.used() - extra)
            .unwrap();
        let before = bank.account(kind).used();
        let original = root(1, Generation::INITIAL);
        let reference = original.reference();
        match owner.register_instance(original, &mut work()) {
            Ok(handle) => {
                assert_eq!(handle.consumer(), reference);
                assert!(handle.is_current());
            }
            Err(rejected) => {
                assert!(matches!(
                    rejected.error().error(),
                    CoreError::Backpressure(_)
                ));
                let (returned, _) = rejected.into_parts();
                assert_eq!(returned.reference(), reference);
                assert!(reference.is_current() && a.is_current() && b.is_current());
                assert_eq!(bank.account(kind).used(), before);
                saw_partial_growth_refusal |= bank.account(kind).peak() > before;
                drop(returned);
            }
        }
        owner.remove_instance(&a, &mut work()).unwrap();
        owner.remove_instance(&b, &mut work()).unwrap();
        drop((reference, a, b, owner, hold));
        clean(&bank);
    }
    assert!(
        saw_partial_growth_refusal,
        "exercise rollback after a staged allocation"
    );
}
