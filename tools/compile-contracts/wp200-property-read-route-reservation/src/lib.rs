#![no_std]

#[cfg(feature = "std")]
use clinkz_wot_core::HostBindingRegistration;
use clinkz_wot_core::binding::BindingRouteKey;
use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactRole, BindingConfigurationDigest,
    BindingGeneration, BindingId, BindingLifetimeFootprint, BindingRegistrationCapabilities,
    BindingRegistrationIdentity, BindingResourceDeclarations, BindingStatusPolicy, PlanId,
    PlanSetGeneration, PollServerBinding, PrepareInput, RoutePreparationVisibility,
    RoutePrepareOutcome, ServerRouteSlot, StartStatus,
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
        .id("urn:fixture:producer-route-reservation")
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

/// Forces the borrowed host compiler projection to retain the same portable
/// Producer-route artifact-metadata contract.
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
        .expect("borrowed host route-reservation build cursor");
}

/// Exercises the real compiler-to-route-reservation handoff.
pub fn verify_route_reservation_projection() {
    let compatibility = BindingArtifactCompatibility::new([7; 16]);
    let identity = BindingRegistrationIdentity::new(
        BindingId::new(11),
        BindingGeneration::new(Generation::INITIAL),
        BindingConfigurationDigest::new([12; 32]),
        compatibility,
        0,
    );
    let state_footprint = BindingLifetimeFootprint::new(2, 128);
    let registration_footprint = BindingLifetimeFootprint::new(4, 256);
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
    let output = {
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

        output
    };

    let artifact_ref = output.artifact_refs()[0];
    let envelope = &output.artifacts()[artifact_ref.artifact_slot().get() as usize];
    assert_eq!(envelope.identity(), artifact_ref.identity());
    assert_eq!(
        artifact_ref.identity().role(),
        BindingArtifactRole::ProducerRoute
    );
    let reservation = envelope
        .route_reservation()
        .expect("Producer-route compiler supplied canonical reservation identity");

    let artifact_identity = artifact_ref.identity();
    let route = BindingRouteKey::new(
        artifact_identity.binding_id(),
        artifact_identity.binding_generation(),
        Generation::INITIAL,
        artifact_identity.plan_set_generation(),
        artifact_identity.plan_id(),
        reservation,
    );
    assert_eq!(route.reservation(), reservation);
    let prepare = PrepareInput::new(route, artifact_ref, state_footprint);
    let mut route_slot = ServerRouteSlot::new();
    let mut prepare_budget = WorkBudget::new();
    let outcome = registration
        .server_mut()
        .start_prepare(prepare, envelope, &mut route_slot, &mut prepare_budget)
        .expect("real server accepted compiler-derived route identity");
    assert!(matches!(
        outcome,
        StartStatus::Ready(RoutePrepareOutcome::Prepared(()))
    ));
    assert_eq!(route_slot.input().route().reservation(), reservation);
}
