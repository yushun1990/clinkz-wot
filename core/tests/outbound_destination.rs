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
    let destination = OutboundDestination::with_funding(text, funding);
    let identity = BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL,
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        BindingId::new(1),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([1; 32]),
        BindingArtifactCompatibility::new([1; 16]),
        BindingArtifactRole::ConsumerCall,
    );
    let request = OutboundRequest::property_read(
        BindingArtifactRef::new(identity, SlotIndex::new(0)),
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
