#![cfg(feature = "std")]
use clinkz_wot_core::{host_accounts::*, *};
use clinkz_wot_foundation::*;
use std::sync::Arc;

#[test]
fn destination_bytes_and_original_reservation_follow_the_last_native_request() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let text: Box<str> = "opaque-destination".into();
    let original_pointer = text.as_ptr();
    let bytes = OutboundDestination::backing_bytes(text.len()).unwrap();
    let funding = bank
        .reserve(ResourceKind::EngineLiveBytesGlobalMax, bytes)
        .unwrap();
    let destination = OutboundDestination::try_with_funding(text, funding)
        .unwrap_or_else(|rejection| panic!("{:?}", rejection.error()));
    let identity = BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL,
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        BindingId::new(1),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([1; 32]),
        BindingArtifactCompatibility::new([1; 16]),
        BindingArtifactRole::ConsumerCall,
    );
    let request = OutboundRequest::finite(
        BindingArtifactRef::new(identity, SlotIndex::new(0)),
        Operation::ReadProperty,
        None,
        Default::default(),
        None,
    )
    .unwrap()
    .with_destination(destination.clone());
    let settlement_operand = destination.clone();
    assert_eq!(request.destination().unwrap().as_ptr(), original_pointer);
    assert_eq!(settlement_operand.as_ptr(), original_pointer);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        bytes
    );
    drop(destination);
    drop(settlement_operand);
    assert_eq!(request.destination().unwrap().as_ptr(), original_pointer);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        bytes
    );
    drop(request);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
}

#[test]
fn invalid_funding_returns_the_identical_value_and_unchanged_reservation_for_retry() {
    for text in ["", "opaque-destination"] {
        let bytes = OutboundDestination::backing_bytes(text.len()).unwrap();
        assert!(bytes > text.len() as u64, "shared owner also needs backing");
        for (kind, amount) in [
            (ResourceKind::EngineLiveBytesGlobalMax, 0),
            (ResourceKind::EngineLiveBytesGlobalMax, bytes - 1),
            (ResourceKind::AdmissionTemporaryBytesGlobalMax, bytes),
            (ResourceKind::CleanupBytesMax, bytes + 17),
        ] {
            let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
            let value: Box<str> = text.into();
            let original_pointer = value.as_ptr();
            let funding = bank.reserve(kind, amount).unwrap();
            let rejection = OutboundDestination::try_with_funding(value, funding)
                .expect_err("invalid funding must refuse construction");
            assert!(matches!(
                rejection.error().error(),
                CoreError::Validation(_)
            ));
            assert_eq!(rejection.input().0.as_ptr(), original_pointer);
            assert_eq!(rejection.input().0.as_ref(), text);
            assert_eq!(rejection.input().1.amount(), amount);
            for resource in ResourceKind::ALL {
                assert_eq!(
                    bank.account(resource).used(),
                    if resource == kind { amount } else { 0 }
                );
            }
            let (value, mut funding) = rejection.into_input();
            if kind == ResourceKind::EngineLiveBytesGlobalMax {
                // Repair the returned lease in place at its original bank.
                assert!(funding.try_grow(bytes - amount));
            } else {
                drop(funding);
                assert_eq!(bank.account(kind).used(), 0);
                funding = bank
                    .reserve(ResourceKind::EngineLiveBytesGlobalMax, bytes)
                    .unwrap();
            }
            let destination = OutboundDestination::try_with_funding(value, funding)
                .unwrap_or_else(|rejection| panic!("{:?}", rejection.error()));
            assert_eq!(destination.as_ptr(), original_pointer);
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                bytes
            );
            let retained = destination.clone();
            drop(destination);
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                bytes
            );
            drop(retained);
            for resource in ResourceKind::ALL {
                assert_eq!(bank.account(resource).used(), 0);
            }
        }
    }
}

#[test]
fn excess_capacity_stays_at_the_original_bank_until_the_last_owner_drops() {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let value: Box<str> = "opaque-destination".into();
    let bytes = OutboundDestination::backing_bytes(value.len()).unwrap() + 17;
    let funding = bank
        .reserve(ResourceKind::EngineLiveBytesGlobalMax, bytes)
        .unwrap();
    let destination = OutboundDestination::try_with_funding(value, funding)
        .unwrap_or_else(|rejection| panic!("{:?}", rejection.error()));
    let other = destination.clone();
    assert_eq!(other.as_ptr(), destination.as_ptr());
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        bytes
    );
    drop(destination);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        bytes
    );
    drop(other);
    assert_eq!(
        bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
        0
    );
}
