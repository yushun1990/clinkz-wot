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
