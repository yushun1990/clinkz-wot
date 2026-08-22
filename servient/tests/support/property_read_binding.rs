pub use clinkz_wot_property_read_binding_fixture::*;

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_core::binding::BindingRouteKey;
    use clinkz_wot_core::{
        BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
        BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRole,
        BindingConfigurationDigest, BindingExecutionSupport, BindingGeneration, BindingId,
        BindingIngressPolicy, BindingLifetimeFootprint, BindingRegistrationCapabilities,
        BindingRegistrationIdentity, BindingResourceDeclarations, BindingStatusPolicy,
        CollisionDomainId, EndpointReservationKey, PlanId, PlanSetGeneration, PollServerBinding,
        PrepareInput, RouteReservationIdentity, ServerRouteSlot,
    };
    use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget};

    #[test]
    fn static_variant_mismatch_returns_input_before_route_state_or_side_effect() {
        let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
        let registration_identity = BindingRegistrationIdentity::new(
            BindingId::new(7),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([0x52; 32]),
            compatibility,
            0,
        );
        let resources = BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(4, 256),
            BindingLifetimeFootprint::new(4, 256),
        );
        let mut registration = match static_registration(
            registration_identity,
            compatibility,
            BindingRegistrationCapabilities::producer_property_read(),
            BindingExecutionSupport::application_static(),
            resources,
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(1, 64),
            0,
        ) {
            Ok(registration) => registration,
            Err(_) => panic!("complete static registration was rejected"),
        };
        let plan_id = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
        let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
        let artifact_identity = BindingArtifactIdentity::new(
            plan_set_generation,
            plan_id,
            registration_identity.binding_id(),
            registration_identity.binding_generation(),
            registration_identity.configuration(),
            compatibility,
            BindingArtifactRole::ProducerRoute,
        );
        let reservation = RouteReservationIdentity::new(
            CollisionDomainId::new([0x61; 16]),
            EndpointReservationKey::new([0x62; 32]),
        );
        let artifact_footprint = BindingArtifactFootprint::new(1, 1);
        let envelope = BindingArtifactEnvelope::try_new(
            artifact_identity,
            artifact_footprint,
            BindingArtifact::producer_route(
                compatibility,
                artifact_footprint,
                reservation,
                MockArtifact::unsupported_variant(),
            ),
        )
        .expect("admitted wrong static variant");
        let artifact_ref =
            clinkz_wot_core::BindingArtifactRef::new(artifact_identity, SlotIndex::new(0));
        let route = BindingRouteKey::new(
            registration_identity.binding_id(),
            registration_identity.binding_generation(),
            Generation::INITIAL,
            plan_set_generation,
            plan_id,
            reservation,
        );
        let prepare = PrepareInput::new(route, artifact_ref, resources.route_state());
        let mut route_slot = ServerRouteSlot::new();
        let rejection = registration
            .server_mut()
            .start_prepare(prepare, &envelope, &mut route_slot, &mut WorkBudget::new())
            .expect_err("unsupported static artifact variant must be rejected");

        assert!(route_slot.is_vacant());
        assert_eq!(rejection.input().route(), &route);
        assert_eq!(rejection.input().artifact(), artifact_ref);
        assert_eq!(
            rejection.into_input().admitted_footprint(),
            resources.route_state()
        );
    }
}
