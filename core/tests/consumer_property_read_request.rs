use std::{collections::BTreeMap, sync::Arc};

use clinkz_wot_core::{
    ActionInvocationRef, AffordanceTarget, BindingArtifactCompatibility, BindingArtifactIdentity,
    BindingArtifactRef, BindingArtifactRole, BindingConfigurationDigest, BindingGeneration,
    BindingId, BindingResponseMetadata, CoreError, Deadline, InteractionOutput,
    InteractionOutputMetadata, InteractionStatus, Payload, PlanId, PlanSetGeneration,
    ResponsePayloadRole, ThingId, validate_untrusted_binding_output,
};
use clinkz_wot_foundation::{
    ClockId, GatewayDefaultV1, Generation, MonotonicInstant, ResourceKind, SlotIndex,
    StaticResourceProfile,
};
use clinkz_wot_td::data_type::Operation;

const COMPATIBILITY: BindingArtifactCompatibility = BindingArtifactCompatibility::new([7; 16]);
const BINDING_ID: BindingId = BindingId::new(11);
const BINDING_GENERATION: BindingGeneration = BindingGeneration::INITIAL;

fn plan_id(slot: u32) -> PlanId {
    PlanId::new(SlotIndex::new(slot), Generation::INITIAL)
}

fn artifact_ref(role: BindingArtifactRole) -> BindingArtifactRef {
    BindingArtifactRef::new(
        BindingArtifactIdentity::new(
            PlanSetGeneration::INITIAL,
            plan_id(13),
            BINDING_ID,
            BINDING_GENERATION,
            BindingConfigurationDigest::new([17; 32]),
            COMPATIBILITY,
            role,
        ),
        SlotIndex::new(19),
    )
}

fn request() -> clinkz_wot_core::OutboundRequest {
    let mut uri_variables = BTreeMap::new();
    uri_variables.insert(String::from("room"), String::from("west"));
    clinkz_wot_core::OutboundRequest::property_read(
        ThingId::from("urn:test:consumer"),
        AffordanceTarget::Property(Arc::from("temperature")),
        artifact_ref(BindingArtifactRole::ConsumerCall),
        uri_variables,
        Some(Deadline::at(MonotonicInstant::new(ClockId::new(3), 29))),
    )
    .expect("the exact Property Read request is admitted")
}

fn payload() -> Payload {
    Payload::new(Vec::from(&b"23.5"[..]), "application/json")
}

fn output_with(metadata: InteractionOutputMetadata) -> InteractionOutput {
    InteractionOutput::with_data(payload())
        .try_with_metadata(metadata)
        .expect("generic output accepts untrusted binding metadata")
}

fn valid_output(status_code: u16) -> InteractionOutput {
    output_with(
        InteractionOutputMetadata::default().with_untrusted_binding_response(
            BindingResponseMetadata::primary(
                BINDING_ID,
                BINDING_GENERATION,
                plan_id(13),
                status_code,
            ),
        ),
    )
}

fn assert_validation_failure(
    request: &clinkz_wot_core::OutboundRequest,
    output: InteractionOutput,
) {
    let error = validate_untrusted_binding_output(request, output)
        .expect_err("invalid untrusted output must be rejected");
    let CoreError::Validation(context) = error else {
        panic!("wrapper must retain the Core validation category")
    };
    assert_eq!(context.operation(), Some(Operation::ReadProperty));
    assert_eq!(context.plan(), Some(request.plan_id()));
    assert_eq!(
        context.binding(),
        Some((request.binding_id(), request.binding_generation()))
    );
}

#[test]
fn property_read_request_owns_only_selected_and_call_varying_facts() {
    let request = request();

    assert_eq!(request.thing_id().as_str(), "urn:test:consumer");
    assert_eq!(
        request.target(),
        &AffordanceTarget::Property(Arc::from("temperature"))
    );
    assert_eq!(request.operation(), Operation::ReadProperty);
    assert_eq!(
        request.artifact(),
        artifact_ref(BindingArtifactRole::ConsumerCall)
    );
    assert_eq!(request.binding_id(), BINDING_ID);
    assert_eq!(request.binding_generation(), BINDING_GENERATION);
    assert_eq!(request.plan_set_generation(), PlanSetGeneration::INITIAL);
    assert_eq!(request.plan_id(), plan_id(13));
    assert_eq!(request.uri_variables().get("room").unwrap(), "west");
    assert_eq!(
        request.deadline(),
        Some(Deadline::at(MonotonicInstant::new(ClockId::new(3), 29)))
    );
}

#[test]
fn request_rejects_non_property_targets_and_non_consumer_artifacts() {
    for target in [
        AffordanceTarget::Thing,
        AffordanceTarget::Action(Arc::from("calibrate")),
        AffordanceTarget::Event(Arc::from("alarm")),
    ] {
        let error = clinkz_wot_core::OutboundRequest::property_read(
            ThingId::from("urn:test:consumer"),
            target,
            artifact_ref(BindingArtifactRole::ConsumerCall),
            BTreeMap::new(),
            None,
        )
        .expect_err("a non-property target is structurally invalid");
        assert!(matches!(error, CoreError::Validation(_)));
    }

    for role in [
        BindingArtifactRole::ConsumerSubscription,
        BindingArtifactRole::ProducerRoute,
        BindingArtifactRole::ProducerPublication,
    ] {
        let error = clinkz_wot_core::OutboundRequest::property_read(
            ThingId::from("urn:test:consumer"),
            AffordanceTarget::Property(Arc::from("temperature")),
            artifact_ref(role),
            BTreeMap::new(),
            None,
        )
        .expect_err("only ConsumerCall artifacts are admitted");
        assert!(matches!(error, CoreError::Validation(_)));
    }
}

#[test]
fn public_wrapper_preserves_valid_output_and_opaque_native_status() {
    let request = request();
    for native_status in [0, 100, 204, 418, 599, u16::MAX] {
        let output = valid_output(native_status);
        let validated = validate_untrusted_binding_output(&request, output)
            .expect("the exact selected primary response is valid");
        assert_eq!(
            validated
                .metadata()
                .binding_response()
                .expect("validated provenance remains present")
                .status_code(),
            native_status
        );
    }
}

#[test]
fn public_wrapper_rejects_every_identity_and_response_shape_negative() {
    let request = request();
    let different_generation = BINDING_GENERATION.checked_next().unwrap();
    let identities = [
        BindingResponseMetadata::primary(BindingId::new(12), BINDING_GENERATION, plan_id(13), 200),
        BindingResponseMetadata::primary(BINDING_ID, different_generation, plan_id(13), 200),
        BindingResponseMetadata::primary(BINDING_ID, BINDING_GENERATION, plan_id(14), 200),
    ];
    for response in identities {
        assert_validation_failure(
            &request,
            output_with(
                InteractionOutputMetadata::default().with_untrusted_binding_response(response),
            ),
        );
    }

    assert_validation_failure(&request, InteractionOutput::with_data(payload()));

    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(ResourceKind::AdditionalResponsesPerFormMax, Some(1))
        .unwrap();
    let additional = BindingResponseMetadata::try_additional(
        BINDING_ID,
        BINDING_GENERATION,
        plan_id(13),
        0,
        200,
        &limits,
    )
    .unwrap();
    assert_validation_failure(
        &request,
        output_with(
            InteractionOutputMetadata::default().with_untrusted_binding_response(additional),
        ),
    );

    let primary_metadata = InteractionOutputMetadata::default().with_untrusted_binding_response(
        BindingResponseMetadata::primary(BINDING_ID, BINDING_GENERATION, plan_id(13), 200),
    );
    let missing_payload = InteractionOutput::empty()
        .try_with_metadata(primary_metadata)
        .unwrap();
    assert_validation_failure(&request, missing_payload);
    assert_validation_failure(
        &request,
        valid_output(200).with_status(InteractionStatus::Created),
    );
    assert_validation_failure(
        &request,
        valid_output(200).with_status(InteractionStatus::Accepted),
    );

    let response = valid_output(200).metadata().binding_response().unwrap();
    assert_validation_failure(
        &request,
        output_with(
            InteractionOutputMetadata::default()
                .with_untrusted_binding_response(response)
                .with_payload_role(ResponsePayloadRole::OperationStatus),
        ),
    );
    assert_validation_failure(
        &request,
        output_with(
            InteractionOutputMetadata::default()
                .with_untrusted_binding_response(response)
                .with_action_invocation(ActionInvocationRef::new(
                    SlotIndex::new(31),
                    Generation::INITIAL,
                )),
        ),
    );
}
