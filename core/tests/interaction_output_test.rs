use core::fmt::Debug;
use core::hash::Hash;

use clinkz_wot_core::{
    ActionInvocationRef, BindingGeneration, BindingId, BindingResponseMetadata, InteractionOutput,
    InteractionOutputMetadata, InteractionStatus, Payload, PlanId, ResponsePayloadRole,
    ResponseSelection,
};
use clinkz_wot_foundation::{
    GatewayDefaultV1, Generation, ResourceKind, ResourceLimits, SlotIndex, StaticResourceProfile,
};

fn plan_id() -> PlanId {
    PlanId::new(SlotIndex::new(3), Generation::INITIAL)
}

fn action_invocation() -> ActionInvocationRef {
    ActionInvocationRef::new(SlotIndex::new(4), Generation::INITIAL)
}

fn limits_with_additional_max(limit: Option<u64>) -> ResourceLimits {
    GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(ResourceKind::AdditionalResponsesPerFormMax, limit)
        .expect("test limits satisfy the frozen resource schema")
}

fn assert_copy_value_traits<T>()
where
    T: Clone + Copy + Debug + Eq + Hash + Ord + PartialEq + PartialOrd,
{
}

#[test]
fn interaction_value_types_expose_the_frozen_traits() {
    assert_copy_value_traits::<InteractionStatus>();
    assert_copy_value_traits::<ResponsePayloadRole>();
    assert_copy_value_traits::<ResponseSelection>();
    assert_copy_value_traits::<BindingResponseMetadata>();
    assert_copy_value_traits::<InteractionOutputMetadata>();

    fn assert_output_traits<T>()
    where
        T: Clone + Debug + Default + Eq + PartialEq,
    {
    }
    assert_output_traits::<InteractionOutput>();
}

#[test]
fn primary_binding_response_metadata_round_trips() {
    let metadata = BindingResponseMetadata::primary(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        plan_id(),
        204,
    );

    assert_eq!(metadata.binding_id(), BindingId::new(7));
    assert_eq!(metadata.binding_generation(), BindingGeneration::INITIAL);
    assert_eq!(metadata.plan_id(), plan_id());
    assert_eq!(metadata.selection(), ResponseSelection::Primary);
    assert_eq!(metadata.status_code(), 204);
}

#[test]
fn additional_binding_response_metadata_enforces_every_limit_boundary() {
    let binding_id = BindingId::new(9);
    let binding_generation = BindingGeneration::INITIAL;
    let plan_id = plan_id();

    for limit in [None, Some(0)] {
        assert_eq!(
            BindingResponseMetadata::try_additional(
                binding_id,
                binding_generation,
                plan_id,
                0,
                200,
                &limits_with_additional_max(limit),
            ),
            None
        );
    }

    let one = limits_with_additional_max(Some(1));
    let accepted = BindingResponseMetadata::try_additional(
        binding_id,
        binding_generation,
        plan_id,
        0,
        206,
        &one,
    )
    .expect("index zero is below a limit of one");
    assert_eq!(accepted.binding_id(), binding_id);
    assert_eq!(accepted.binding_generation(), binding_generation);
    assert_eq!(accepted.plan_id(), plan_id);
    assert_eq!(accepted.selection(), ResponseSelection::Additional(0));
    assert_eq!(accepted.status_code(), 206);
    assert_eq!(
        BindingResponseMetadata::try_additional(
            binding_id,
            binding_generation,
            plan_id,
            1,
            206,
            &one,
        ),
        None
    );

    let maximum = limits_with_additional_max(Some(65_536));
    let accepted = BindingResponseMetadata::try_additional(
        binding_id,
        binding_generation,
        plan_id,
        u16::MAX,
        299,
        &maximum,
    )
    .expect("the maximum u16 index is below the maximum valid limit");
    assert_eq!(
        accepted.selection(),
        ResponseSelection::Additional(u16::MAX)
    );
}

#[test]
fn interaction_output_metadata_builders_round_trip() {
    let binding_response = BindingResponseMetadata::primary(
        BindingId::new(11),
        BindingGeneration::INITIAL,
        plan_id(),
        201,
    );
    let metadata = InteractionOutputMetadata::default()
        .with_action_invocation(action_invocation())
        .with_payload_role(ResponsePayloadRole::OperationStatus)
        .with_untrusted_binding_response(binding_response);

    assert_eq!(metadata.action_invocation(), Some(action_invocation()));
    assert_eq!(metadata.binding_response(), Some(binding_response));
    assert_eq!(
        metadata.payload_role(),
        ResponsePayloadRole::OperationStatus
    );

    let default = InteractionOutputMetadata::default();
    assert_eq!(default.action_invocation(), None);
    assert_eq!(default.binding_response(), None);
    assert_eq!(default.payload_role(), ResponsePayloadRole::Application);
}

#[test]
fn interaction_output_defaults_and_parts_match_the_frozen_surface() {
    const EMPTY: InteractionOutput = InteractionOutput::empty();

    assert_eq!(EMPTY, InteractionOutput::default());
    assert_eq!(EMPTY.data(), None);
    assert_eq!(EMPTY.status(), InteractionStatus::Ok);
    assert_eq!(EMPTY.metadata(), &InteractionOutputMetadata::default());

    let metadata = InteractionOutputMetadata::default().with_action_invocation(action_invocation());
    let output = InteractionOutput::with_data(Payload::new(b"state".to_vec(), "text/plain"))
        .with_status(InteractionStatus::Created)
        .try_with_metadata(metadata)
        .expect("application payload metadata is valid");

    assert_eq!(output.data().unwrap().body.as_ref(), b"state");
    assert_eq!(output.status(), InteractionStatus::Created);
    assert_eq!(output.metadata(), &metadata);

    let (data, status, returned_metadata) = output.into_parts();
    assert_eq!(data.unwrap().body.as_ref(), b"state");
    assert_eq!(status, InteractionStatus::Created);
    assert_eq!(returned_metadata, metadata);
}

#[test]
fn operation_status_metadata_requires_a_payload() {
    let metadata = InteractionOutputMetadata::default()
        .with_action_invocation(action_invocation())
        .with_payload_role(ResponsePayloadRole::OperationStatus);

    assert_eq!(InteractionOutput::empty().try_with_metadata(metadata), None);

    let output = InteractionOutput::with_data(Payload::new(b"running".to_vec(), "text/plain"))
        .try_with_metadata(metadata)
        .expect("operation status metadata has one payload");
    assert_eq!(
        output.metadata().payload_role(),
        ResponsePayloadRole::OperationStatus
    );

    assert!(
        InteractionOutput::empty()
            .try_with_metadata(InteractionOutputMetadata::default())
            .is_some()
    );
}

#[test]
fn into_data_moves_the_payload_without_exposing_metadata_fields() {
    let payload = Payload::new(b"value".to_vec(), "application/octet-stream");
    let output = InteractionOutput::with_data(payload.clone());

    assert_eq!(output.into_data(), Some(payload));
}
