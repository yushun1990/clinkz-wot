#![no_std]

#[cfg(feature = "std")]
use clinkz_wot_core::HostBindingRegistration;
use clinkz_wot_core::binding::BindingRouteKey;
use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactRole, BindingConfigurationDigest,
    BindingGeneration, BindingId, BindingLifetimeFootprint, BindingRegistrationCapabilities,
    BindingRegistrationIdentity, BindingResourceDeclarations, BindingStatusPolicy,
    CollisionDomainId, EndpointReservationKey, PlanId, PlanSetGeneration, PollServerBinding,
    PrepareInput, RoutePreparationVisibility, RoutePrepareOutcome, RouteReservationIdentity,
    ServerRouteSlot, StartStatus,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_planning::{PlanBuildInput, PlanBuildStep, PlanCompiler, PropertyReadPlanCompiler};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    thing::Thing,
};
use clinkz_wot_wp300_property_read_binding_slice_contract::static_registration;

fn thing() -> Thing {
    Thing::builder("Tank")
        .id("urn:fixture:producer-route")
        .nosec()
        .property(
            "level",
            PropertyAffordance::builder(DataSchema::number())
                .form(
                    Form::read_property("mock://tank/level")
                        .build()
                        .expect("valid form"),
                )
                .build()
                .expect("valid property"),
        )
        .build()
        .expect("valid thing")
}

/// The resumable cursor is public, but its state is intentionally unreadable
/// and cannot be forged by an external caller.
///
/// ```compile_fail
/// use clinkz_wot_planning::PropertyReadBuildCursor;
///
/// fn inspect<C, A>(cursor: PropertyReadBuildCursor<C, A>) {
///     let _ = cursor.state;
/// }
/// ```
pub struct CursorOpacityContract;

/// Forces the std-host projection borrowed from one complete registration to
/// implement the same real Planning contract as the static projection.
#[cfg(feature = "std")]
pub fn typecheck_borrowed_host_projection(
    registration: &HostBindingRegistration,
    td: &Thing,
    plan_id: PlanId,
    plan_set_generation: PlanSetGeneration,
) {
    let compiler_registrations = [registration.compiler()];
    let input = PlanBuildInput::new(td, &compiler_registrations[..], plan_set_generation);
    let compiler = PropertyReadPlanCompiler::producer_route(plan_id, registration.identity(), 0, 0);
    let _ = compiler
        .start(&input)
        .expect("borrowed host Producer-route build cursor");
}

/// Exercises the real Producer-route plan-to-binding handoff.
pub fn verify_producer_route_projection() {
    let compatibility = BindingArtifactCompatibility::new([7; 16]);
    let identity = BindingRegistrationIdentity::new(
        BindingId::new(11),
        BindingGeneration::new(Generation::INITIAL),
        BindingConfigurationDigest::new([12; 32]),
        compatibility,
        0,
    );
    let state_footprint = BindingLifetimeFootprint::new(1, 8);
    let registration_footprint = BindingLifetimeFootprint::new(3, 64);
    let resources =
        BindingResourceDeclarations::new(registration_footprint, registration_footprint)
            .with_state_footprints(state_footprint, state_footprint, state_footprint);
    let mut registration = match static_registration(
        identity,
        compatibility,
        BindingRegistrationCapabilities::producer_property_read(),
        clinkz_wot_core::BindingExecutionSupport::application_static(),
        resources,
        clinkz_wot_core::BindingIngressPolicy::new(
            RoutePreparationVisibility::Hidden,
            clinkz_wot_core::BindingIngressLimits::new(0, 0),
            clinkz_wot_core::BindingIngressLimits::new(0, 0),
            clinkz_wot_core::BindingIngressLimits::new(0, 0),
        ),
        BindingStatusPolicy::new(1, 32),
        0,
    ) {
        Ok(registration) => registration,
        Err(_) => panic!("complete Producer Property Read registration"),
    };

    let plan_id = PlanId::new(SlotIndex::new(3), Generation::INITIAL);
    let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
    let artifact_ref = {
        let td = thing();
        let compiler_registrations = [registration.compiler()];
        let input = PlanBuildInput::new(&td, &compiler_registrations[..], plan_set_generation);
        let compiler =
            PropertyReadPlanCompiler::producer_route(plan_id, registration.identity(), 0, 0);
        let cursor = compiler.start(&input).expect("Producer-route build cursor");
        let mut zero = WorkBudget::new();
        let cursor = match compiler.step(&input, cursor, &mut zero) {
            PlanBuildStep::Pending(cursor) => cursor,
            _ => panic!("zero budget advanced Producer-route planning"),
        };
        let mut work = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        let output = match compiler.step(&input, cursor, &mut work) {
            PlanBuildStep::Complete(output) => output,
            PlanBuildStep::Pending(_) => panic!("one-step mock compiler remained pending"),
            PlanBuildStep::Failed(failure) => {
                panic!("Producer-route planning failed: {:?}", failure.error())
            }
        };

        assert_eq!(output.logical_plans().len(), 1);
        assert_eq!(output.artifacts().len(), 1);
        assert_eq!(output.artifact_refs().len(), 1);
        let artifact_ref = output.artifact_refs()[0];
        let artifact_identity = artifact_ref.identity();
        assert_eq!(artifact_identity.role(), BindingArtifactRole::ProducerRoute);
        assert_eq!(
            artifact_identity.binding_id(),
            registration.identity().binding_id()
        );
        assert_eq!(
            artifact_identity.binding_generation(),
            registration.identity().binding_generation()
        );
        assert_eq!(
            artifact_identity.configuration(),
            registration.identity().configuration()
        );
        assert_eq!(
            artifact_identity.compatibility(),
            registration.identity().artifact_compatibility()
        );
        assert_eq!(artifact_identity.plan_set_generation(), plan_set_generation);
        assert_eq!(artifact_identity.plan_id(), plan_id);
        assert_eq!(output.artifacts()[0].identity(), artifact_identity);
        artifact_ref
    };

    let route = BindingRouteKey::new(
        registration.identity().binding_id(),
        registration.identity().binding_generation(),
        Generation::INITIAL,
        plan_set_generation,
        plan_id,
        RouteReservationIdentity::new(
            CollisionDomainId::new([21; 16]),
            EndpointReservationKey::new([22; 32]),
        ),
    );
    let artifact_identity = artifact_ref.identity();
    assert_eq!(route.binding_id(), artifact_identity.binding_id());
    assert_eq!(
        route.binding_generation(),
        artifact_identity.binding_generation()
    );
    assert_eq!(
        route.plan_set_generation(),
        artifact_identity.plan_set_generation()
    );
    assert_eq!(route.plan_id(), artifact_identity.plan_id());
    let prepare = PrepareInput::new(route, artifact_ref, state_footprint);
    let mut route_slot = ServerRouteSlot::new();
    let mut prepare_budget = WorkBudget::new();
    let outcome = registration
        .server_mut()
        .start_prepare(prepare, &mut route_slot, &mut prepare_budget)
        .expect("real Producer server accepted plan-derived preparation input");
    assert!(matches!(
        outcome,
        StartStatus::Ready(RoutePrepareOutcome::Prepared(()))
    ));
    assert_eq!(
        route_slot.input().artifact().identity().role(),
        BindingArtifactRole::ProducerRoute
    );
}
