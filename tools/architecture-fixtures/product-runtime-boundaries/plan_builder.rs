use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingRegistrationIdentity, PlanId, PlanSetGeneration, StaticBindingCompilerRegistration,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_planning::{
    PlanBuildInput, PlanBuildOutput, PlanBuildStep, PlanCompiler, PropertyReadPlanCompiler,
};
use clinkz_wot_property_read_binding_fixture::{MockArtifact, MockCompiler};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    thing::Thing,
    validate::Validate,
};
pub fn build() -> PlanBuildOutput<MockArtifact> {
    let td = Thing::builder("temperature sensor")
        .id("urn:probe:sensor")
        .base("zenoh+tcp://127.0.0.1:7447/base/")
        .nosec()
        .property(
            "temperature",
            PropertyAffordance::builder(DataSchema::number())
                .form(Form::read_property("/ignored").build().unwrap())
                .form(
                    Form::read_property("../sensor/temperature?unit=C")
                        .content_type("application/json")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    td.validate().unwrap();
    let compatibility = BindingArtifactCompatibility::new([3; 16]);
    let identity = BindingRegistrationIdentity::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([2; 32]),
        compatibility,
        0,
    );
    let registrations = [StaticBindingCompilerRegistration::new(MockCompiler::new(
        compatibility,
    ))];
    let compiler = PropertyReadPlanCompiler::consumer_call(
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        "temperature".into(),
        1,
        identity,
        0,
        0,
    );
    let input = PlanBuildInput::new(&td, &registrations[..], PlanSetGeneration::INITIAL);
    let mut cursor = compiler.start(&input).unwrap();
    // A changed leaf that makes no progress should fail this probe, not hang
    // an unrelated architecture-investigation build indefinitely.
    for _ in 0..32 {
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        match compiler.step(&input, cursor, &mut budget) {
            PlanBuildStep::Pending(next) => cursor = next,
            PlanBuildStep::Complete(result) => return result,
            PlanBuildStep::Failed(error) => panic!("planning failed: {:?}", error.error()),
        }
    }
    panic!("Property Read leaf did not complete within the fixture step envelope");
}
