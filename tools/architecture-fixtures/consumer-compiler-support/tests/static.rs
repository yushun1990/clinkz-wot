use clinkz_wot_core::*;
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use consumer_compiler_support_probe::*;

#[test]
fn complete_static_registration_and_owned_output_in_each_feature_cell() {
    let complete = registration(CAPACITY, Scenario::Success).unwrap();
    let id = complete.identity();
    let checked =
        capture(Some(complete), id, Representation::Static).unwrap_or_else(|_| panic!("capture"));
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(1), Generation::INITIAL),
        ThingId::from("urn:static:compiler"),
        "temperature".into(),
        1,
        "mock://sensor/temperature".into(),
        None,
        None,
    )
    .unwrap();
    let candidate = BindingCandidate::new(
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        0,
        0,
    );
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
    let mut work = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    checked.bounds(&input, &mut work).unwrap();
    let mut work = WorkBudget::new()
        .with_remaining(WorkClass::BindingPolls, 1)
        .with_remaining(WorkClass::CleanupItems, 1);
    let cursor = checked.start_static(&input, &mut work).unwrap();
    let mut work = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2);
    let cursor = match cursor.step(&input, &mut work) {
        StaticStep::Pending(cursor) => cursor,
        _ => panic!("pending"),
    };
    let mut work = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2);
    let output = match cursor.step(&input, &mut work) {
        StaticStep::Complete(output) => output,
        _ => panic!("complete"),
    };
    drop(checked);
    drop(plan);
    assert_eq!(
        output.artifact().payload().target(),
        "mock://sensor/temperature"
    );
}
