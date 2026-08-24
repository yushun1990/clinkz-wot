use clinkz_wot_core::binding::BindingRouteKey;
use clinkz_wot_core::{
    ActionInvocationRef, BindingGeneration, BindingId, BindingResponseMetadata, CollisionDomainId,
    CoreError, CorrelationId, EndpointReservationKey, ErrorContext, ErrorPhase, InteractionOutput,
    InteractionOutputMetadata, InteractionStatus, Payload, PlanId, PlanSetGeneration,
    ResponsePayloadRole, RetryClass, RouteInboundResponse, RouteReservationIdentity,
    RouteResponseOpportunity,
};
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_td::data_type::Operation;

fn route() -> BindingRouteKey {
    BindingRouteKey::new(
        BindingId::new(37),
        BindingGeneration::INITIAL,
        Generation::INITIAL,
        PlanSetGeneration::INITIAL,
        PlanId::new(SlotIndex::new(3), Generation::INITIAL),
        RouteReservationIdentity::new(
            CollisionDomainId::new([0x37; 16]),
            EndpointReservationKey::new([0x5a; 32]),
        ),
    )
}

fn opportunity(correlation: u64) -> RouteResponseOpportunity {
    RouteResponseOpportunity::new(route(), CorrelationId::new(correlation))
}

fn payload() -> Payload {
    Payload::new(b"property-value".to_vec(), "application/octet-stream")
}

fn assert_validation_failure(response: RouteInboundResponse, expected_correlation: CorrelationId) {
    let (returned_opportunity, result) = response.into_parts();
    assert_eq!(returned_opportunity.route(), &route());
    assert_eq!(returned_opportunity.correlation(), expected_correlation);

    let error = result.expect_err("invalid successful output must be deliverable as failure");
    let CoreError::Validation(context) = error else {
        panic!("invalid successful output must become CoreError::Validation");
    };
    assert_eq!(context.phase(), ErrorPhase::Validate);
    assert_eq!(context.retry_class(), RetryClass::Never);
    assert_eq!(context.operation(), Some(Operation::ReadProperty));
    assert_eq!(context.plan(), Some(route().plan_id()));
    assert_eq!(
        context.binding(),
        Some((route().binding_id(), route().binding_generation()))
    );
    assert_eq!(context.correlation(), Some(expected_correlation));
}

#[test]
fn payload_bearing_ok_application_output_is_sealed() {
    let correlation = CorrelationId::new(11);
    let response = RouteInboundResponse::seal_property_read_handler_result(
        opportunity(11),
        Ok(InteractionOutput::with_data(payload())),
    );

    assert_eq!(response.opportunity().route(), &route());
    assert_eq!(response.opportunity().correlation(), correlation);
    let output = response.result().expect("valid Property Read success");
    assert_eq!(output.data(), Some(&payload()));
    assert_eq!(output.status(), InteractionStatus::Ok);
    assert_eq!(
        output.metadata().payload_role(),
        ResponsePayloadRole::Application
    );
}

#[test]
fn binding_response_metadata_is_rejected_at_handler_origin() {
    let metadata = InteractionOutputMetadata::default().with_untrusted_binding_response(
        BindingResponseMetadata::primary(
            route().binding_id(),
            route().binding_generation(),
            route().plan_id(),
            200,
        ),
    );
    let output = InteractionOutput::with_data(payload())
        .try_with_metadata(metadata)
        .expect("generic interaction value accepts untrusted binding metadata");

    assert_validation_failure(
        RouteInboundResponse::seal_property_read_handler_result(opportunity(12), Ok(output)),
        CorrelationId::new(12),
    );
}

#[test]
fn created_and_accepted_statuses_are_rejected_for_property_read() {
    for (correlation, status) in [
        (13, InteractionStatus::Created),
        (14, InteractionStatus::Accepted),
    ] {
        assert_validation_failure(
            RouteInboundResponse::seal_property_read_handler_result(
                opportunity(correlation),
                Ok(InteractionOutput::with_data(payload()).with_status(status)),
            ),
            CorrelationId::new(correlation),
        );
    }
}

#[test]
fn operation_status_payload_role_is_rejected_for_property_read() {
    let metadata = InteractionOutputMetadata::default()
        .with_payload_role(ResponsePayloadRole::OperationStatus);
    let output = InteractionOutput::with_data(payload())
        .try_with_metadata(metadata)
        .expect("operation-status output has a payload");

    assert_validation_failure(
        RouteInboundResponse::seal_property_read_handler_result(opportunity(15), Ok(output)),
        CorrelationId::new(15),
    );
}

#[test]
fn action_invocation_reference_is_rejected_for_property_read() {
    let metadata = InteractionOutputMetadata::default().with_action_invocation(
        ActionInvocationRef::new(SlotIndex::new(9), Generation::INITIAL),
    );
    let output = InteractionOutput::with_data(payload())
        .try_with_metadata(metadata)
        .expect("generic interaction value accepts an action reference");

    assert_validation_failure(
        RouteInboundResponse::seal_property_read_handler_result(opportunity(16), Ok(output)),
        CorrelationId::new(16),
    );
}

#[test]
fn missing_payload_is_rejected_for_property_read() {
    assert_validation_failure(
        RouteInboundResponse::seal_property_read_handler_result(
            opportunity(17),
            Ok(InteractionOutput::empty()),
        ),
        CorrelationId::new(17),
    );
}

#[test]
fn handler_error_is_preserved_unchanged_with_the_original_opportunity() {
    let correlation = CorrelationId::new(18);
    let handler_error = CoreError::Application(
        ErrorContext::new(ErrorPhase::Handler, RetryClass::CallerDecision)
            .with_operation(Operation::ReadProperty)
            .with_correlation(correlation),
    );
    let response = RouteInboundResponse::seal_property_read_handler_result(
        opportunity(18),
        Err(handler_error.clone()),
    );
    let (returned_opportunity, returned_result) = response.into_parts();

    assert_eq!(returned_opportunity.route(), &route());
    assert_eq!(returned_opportunity.correlation(), correlation);
    assert_eq!(returned_result, Err(handler_error));
}
