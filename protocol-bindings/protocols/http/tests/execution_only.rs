//! The runtime facet constructs and rejects untrusted data without TD/Planning.
use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_execution};
use clinkz_wot_core::*;
use clinkz_wot_foundation::{Generation, SlotIndex};
use std::any::Any;

struct ForeignStorage;
impl HostArtifactStorage for ForeignStorage {
    fn storage_overhead(&self) -> u64 {
        0
    }
    fn payload(&self) -> &dyn Any {
        &()
    }
    fn take_payload(&mut self, destination: &mut dyn Any) {
        *destination.downcast_mut::<Option<()>>().unwrap() = Some(());
    }
}

#[test]
fn runtime_registration_rejects_foreign_native_data_without_an_io_runtime() {
    let runtime = try_consumer_execution(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(601),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap();
    let registration = runtime.identity();
    let identity = BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL,
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        registration.binding_id(),
        registration.binding_generation(),
        registration.configuration(),
        registration.artifact_compatibility(),
        BindingArtifactRole::ConsumerCall,
    );
    let artifact = BindingArtifactEnvelope::try_new(
        identity,
        BindingArtifactFootprint::new(1, 0),
        BindingArtifact::new(
            registration.artifact_compatibility(),
            BindingArtifactFootprint::new(1, 0),
            HostBindingArtifact::from_storage(Box::new(ForeignStorage)),
        ),
    )
    .unwrap();
    let request = OutboundRequest::property_read(
        BindingArtifactRef::new(identity, SlotIndex::new(0)),
        Default::default(),
        None,
    )
    .unwrap();
    let original = OutboundRequest::property_read(
        BindingArtifactRef::new(identity, SlotIndex::new(0)),
        Default::default(),
        None,
    )
    .unwrap();
    // The metadata matches, but it cannot turn foreign bytes into an admitted
    // native program. No Tokio runtime or network service is installed.
    let refused = runtime
        .start_consumer_property_read(request, &artifact)
        .unwrap_err();
    assert_eq!(refused.into_input(), original);
}
