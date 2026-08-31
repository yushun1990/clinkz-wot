//! Core-owned validation of untrusted binding-origin interaction output.

use clinkz_wot_td::data_type::Operation;

use crate::{
    BindingArtifactRef, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext,
    ErrorPhase, InteractionOutput, InteractionStatus, OutboundRequest, PlanId, ResponsePayloadRole,
    ResponseSelection, RetryClass,
};

/// Validates untrusted binding output against one live selected request.
///
/// Expected identities cannot be supplied independently: they are derived
/// from the request's single immutable binding-artifact reference.
pub fn validate_untrusted_binding_output(
    request: &OutboundRequest,
    output: InteractionOutput,
) -> CoreResult<InteractionOutput> {
    validate_property_read_binding_output(
        request.binding_id(),
        request.binding_generation(),
        request.plan_id(),
        output,
    )
}

/// Private single-use authority for sealing one accepted Consumer result.
///
/// This is deliberately a projection of the selected call identity rather
/// than a clone of [`OutboundRequest`]. Complete binding registrations create
/// it immediately before transferring the request and never expose it to a
/// binding author or runtime caller.
pub(crate) struct ConsumerResultSeal {
    artifact: BindingArtifactRef,
}

impl ConsumerResultSeal {
    pub(crate) const fn from_request(request: &OutboundRequest) -> Self {
        Self {
            artifact: request.artifact(),
        }
    }

    pub(crate) fn matches_request(&self, request: &OutboundRequest) -> bool {
        self.artifact == request.artifact()
    }

    pub(crate) fn validate(self, output: InteractionOutput) -> CoreResult<InteractionOutput> {
        let identity = self.artifact.identity();
        validate_property_read_binding_output(
            identity.binding_id(),
            identity.binding_generation(),
            identity.plan_id(),
            output,
        )
    }

    pub(crate) fn validate_against_request(
        self,
        request: &OutboundRequest,
        output: InteractionOutput,
    ) -> CoreResult<InteractionOutput> {
        if self.matches_request(request) {
            validate_untrusted_binding_output(request, output)
        } else {
            Err(self.validation_error(320, "accepted request identity changed"))
        }
    }

    pub(crate) fn validation_error(&self, code: u16, cause: &str) -> CoreError {
        let identity = self.artifact.identity();
        CoreError::Validation(
            ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                .with_operation(Operation::ReadProperty)
                .with_plan(identity.plan_id())
                .with_binding(identity.binding_id(), identity.binding_generation())
                .with_redacted_cause(code, cause),
        )
    }
}

/// Validates the narrow Property Read response produced by a selected binding call.
///
/// The expected identities come from the selected call owner. This kernel stays
/// private until the admitted WP-300 `OutboundRequest` wrapper can derive them
/// from that trusted live request.
pub(crate) fn validate_property_read_binding_output(
    expected_binding_id: BindingId,
    expected_binding_generation: BindingGeneration,
    expected_plan_id: PlanId,
    output: InteractionOutput,
) -> CoreResult<InteractionOutput> {
    let metadata = output.metadata();
    let valid_binding_response = metadata.binding_response().is_some_and(|response| {
        response.binding_id() == expected_binding_id
            && response.binding_generation() == expected_binding_generation
            && response.plan_id() == expected_plan_id
            && response.selection() == ResponseSelection::Primary
    });

    if valid_binding_response
        && output.data().is_some()
        && output.status() == InteractionStatus::Ok
        && metadata.payload_role() == ResponsePayloadRole::Application
        && metadata.action_invocation().is_none()
    {
        return Ok(output);
    }

    Err(CoreError::Validation(
        ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
            .with_operation(Operation::ReadProperty)
            .with_plan(expected_plan_id)
            .with_binding(expected_binding_id, expected_binding_generation),
    ))
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use clinkz_wot_foundation::{
        GatewayDefaultV1, Generation, ResourceKind, SlotIndex, StaticResourceProfile,
    };
    use clinkz_wot_td::data_type::Operation;

    use super::validate_property_read_binding_output;
    use crate::{
        ActionInvocationRef, BindingGeneration, BindingId, BindingResponseMetadata, CoreError,
        ErrorPhase, InteractionOutput, InteractionOutputMetadata, InteractionStatus, Payload,
        PlanId, ResponsePayloadRole, RetryClass,
    };

    const EXPECTED_BINDING_ID: BindingId = BindingId::new(17);
    const EXPECTED_BINDING_GENERATION: BindingGeneration = BindingGeneration::INITIAL;

    fn expected_plan_id() -> PlanId {
        PlanId::new(SlotIndex::new(23), Generation::INITIAL)
    }

    fn payload() -> Payload {
        Payload::new(
            Vec::from(&b"property-value"[..]),
            "application/octet-stream",
        )
    }

    fn valid_output(status_code: u16) -> InteractionOutput {
        let metadata = InteractionOutputMetadata::default().with_untrusted_binding_response(
            BindingResponseMetadata::primary(
                EXPECTED_BINDING_ID,
                EXPECTED_BINDING_GENERATION,
                expected_plan_id(),
                status_code,
            ),
        );
        InteractionOutput::with_data(payload())
            .try_with_metadata(metadata)
            .expect("application output with one payload accepts binding metadata")
    }

    fn validate(output: InteractionOutput) -> crate::CoreResult<InteractionOutput> {
        validate_property_read_binding_output(
            EXPECTED_BINDING_ID,
            EXPECTED_BINDING_GENERATION,
            expected_plan_id(),
            output,
        )
    }

    fn assert_validation_failure(output: InteractionOutput) {
        let error = validate(output).expect_err("invalid binding output must be rejected");
        let CoreError::Validation(context) = error else {
            panic!("invalid binding output must become CoreError::Validation");
        };
        assert_eq!(context.phase(), ErrorPhase::Validate);
        assert_eq!(context.retry_class(), RetryClass::Never);
        assert_eq!(context.operation(), Some(Operation::ReadProperty));
        assert_eq!(context.plan(), Some(expected_plan_id()));
        assert_eq!(
            context.binding(),
            Some((EXPECTED_BINDING_ID, EXPECTED_BINDING_GENERATION))
        );
    }

    #[test]
    fn selected_primary_property_read_output_is_returned_unchanged() {
        let output = valid_output(200);

        assert_eq!(
            validate(output.clone()).expect("exact selected output is valid"),
            output
        );
    }

    #[test]
    fn binding_native_numeric_status_is_opaque_provenance() {
        for status_code in [0, 100, 204, 418, 599, u16::MAX] {
            let output = valid_output(status_code);
            let validated = validate(output.clone())
                .expect("native numeric status never changes Core validation semantics");

            assert_eq!(validated, output);
            assert_eq!(
                validated
                    .metadata()
                    .binding_response()
                    .expect("validated metadata remains present")
                    .status_code(),
                status_code
            );
        }
    }

    #[test]
    fn every_selected_call_identity_mismatch_is_rejected() {
        let mismatched_binding = InteractionOutputMetadata::default()
            .with_untrusted_binding_response(BindingResponseMetadata::primary(
                BindingId::new(18),
                EXPECTED_BINDING_GENERATION,
                expected_plan_id(),
                200,
            ));
        let mismatched_generation = InteractionOutputMetadata::default()
            .with_untrusted_binding_response(BindingResponseMetadata::primary(
                EXPECTED_BINDING_ID,
                EXPECTED_BINDING_GENERATION
                    .checked_next()
                    .expect("a second generation exists"),
                expected_plan_id(),
                200,
            ));
        let mismatched_plan = InteractionOutputMetadata::default().with_untrusted_binding_response(
            BindingResponseMetadata::primary(
                EXPECTED_BINDING_ID,
                EXPECTED_BINDING_GENERATION,
                PlanId::new(SlotIndex::new(24), Generation::INITIAL),
                200,
            ),
        );

        for metadata in [mismatched_binding, mismatched_generation, mismatched_plan] {
            let output = InteractionOutput::with_data(payload())
                .try_with_metadata(metadata)
                .expect("generic output accepts untrusted identity metadata");
            assert_validation_failure(output);
        }
    }

    #[test]
    fn missing_binding_metadata_is_rejected() {
        assert_validation_failure(InteractionOutput::with_data(payload()));
    }

    #[test]
    fn additional_response_selection_is_rejected() {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .try_with_limit(ResourceKind::AdditionalResponsesPerFormMax, Some(1))
            .expect("test limit is within the frozen schema");
        let response = BindingResponseMetadata::try_additional(
            EXPECTED_BINDING_ID,
            EXPECTED_BINDING_GENERATION,
            expected_plan_id(),
            0,
            200,
            &limits,
        )
        .expect("the first additional response is within the test limit");
        let metadata =
            InteractionOutputMetadata::default().with_untrusted_binding_response(response);
        let output = InteractionOutput::with_data(payload())
            .try_with_metadata(metadata)
            .expect("generic output accepts an additional response branch");

        assert_validation_failure(output);
    }

    #[test]
    fn missing_payload_is_rejected() {
        let metadata = InteractionOutputMetadata::default().with_untrusted_binding_response(
            BindingResponseMetadata::primary(
                EXPECTED_BINDING_ID,
                EXPECTED_BINDING_GENERATION,
                expected_plan_id(),
                200,
            ),
        );
        let output = InteractionOutput::empty()
            .try_with_metadata(metadata)
            .expect("generic output accepts binding metadata without a payload");

        assert_validation_failure(output);
    }

    #[test]
    fn every_non_ok_status_is_rejected() {
        for status in [InteractionStatus::Created, InteractionStatus::Accepted] {
            assert_validation_failure(valid_output(200).with_status(status));
        }
    }

    #[test]
    fn non_application_payload_role_is_rejected() {
        let binding_response = valid_output(200)
            .metadata()
            .binding_response()
            .expect("valid helper installs binding response metadata");
        let metadata = InteractionOutputMetadata::default()
            .with_untrusted_binding_response(binding_response)
            .with_payload_role(ResponsePayloadRole::OperationStatus);
        let output = InteractionOutput::with_data(payload())
            .try_with_metadata(metadata)
            .expect("operation-status output has a payload");

        assert_validation_failure(output);
    }

    #[test]
    fn action_invocation_reference_is_rejected() {
        let binding_response = valid_output(200)
            .metadata()
            .binding_response()
            .expect("valid helper installs binding response metadata");
        let metadata = InteractionOutputMetadata::default()
            .with_untrusted_binding_response(binding_response)
            .with_action_invocation(ActionInvocationRef::new(
                SlotIndex::new(31),
                Generation::INITIAL,
            ));
        let output = InteractionOutput::with_data(payload())
            .try_with_metadata(metadata)
            .expect("generic output accepts an action invocation reference");

        assert_validation_failure(output);
    }
}
