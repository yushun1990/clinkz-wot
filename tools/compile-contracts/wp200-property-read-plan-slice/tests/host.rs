use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactRole, BindingCandidate, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerStep, BindingConfigurationDigest, BindingGeneration,
    BindingId, HostBindingCompilerRegistration, LogicalInteractionPlan, PlanId, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_wp200_property_read_plan_slice_contract::{
    FixedArtifact, FixedCompiler, ThirdPartyArtifact, ThirdPartyCompiler,
};

fn plan(compatibility: BindingArtifactCompatibility) -> (LogicalInteractionPlan, BindingCandidate) {
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(1), Generation::INITIAL),
        ThingId::from("urn:fixture:property-read"),
        "temperature".into(),
        0,
        "mock://sensor/temperature".into(),
        Some("application/json".into()),
        None,
    )
    .expect("property-read plan must be constructible");
    let candidate = BindingCandidate::new(
        BindingId::new(2),
        BindingGeneration::new(Generation::INITIAL),
        BindingConfigurationDigest::new([3; 32]),
        compatibility,
        0,
        0,
    );
    (plan, candidate)
}

#[test]
fn core_host_erasure_preserves_mismatched_cursor_and_payload() {
    let third_party_compatibility = BindingArtifactCompatibility::new([4; 16]);
    let fixed_compatibility = BindingArtifactCompatibility::new([5; 16]);
    let third_party =
        HostBindingCompilerRegistration::new(ThirdPartyCompiler::new(third_party_compatibility));
    let fixed = HostBindingCompilerRegistration::new(FixedCompiler::new(fixed_compatibility));
    let (plan, candidate) = plan(third_party_compatibility);
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);

    let fixed_cursor = fixed.start(&input).expect("fixed cursor");
    let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 3);
    let fixed_cursor = match third_party.step(&input, fixed_cursor, &mut budget) {
        BindingCompilerStep::Failed(failure) => failure.into_parts().1,
        _ => panic!("host cursor mismatch did not return the original cursor"),
    };
    let fixed_artifact = match fixed.step(&input, fixed_cursor, &mut budget) {
        BindingCompilerStep::Complete(output) => output.into_artifact(),
        _ => panic!("returned cursor no longer worked with its owner"),
    };
    assert_eq!(
        fixed_artifact
            .try_payload::<FixedArtifact>(fixed_compatibility)
            .copied(),
        Some(FixedArtifact::from_raw(7))
    );

    let cursor = third_party.start(&input).expect("third-party cursor");
    let cursor = match third_party.step(&input, cursor, &mut budget) {
        BindingCompilerStep::Pending(cursor) => cursor,
        _ => panic!("third-party cursor must remain pending after one step"),
    };
    let artifact = match third_party.step(&input, cursor, &mut budget) {
        BindingCompilerStep::Complete(output) => output.into_artifact(),
        _ => panic!("third-party compiler must complete after two steps"),
    };
    assert!(
        artifact
            .try_payload::<ThirdPartyArtifact>(fixed_compatibility)
            .is_none()
    );
    let artifact = match artifact.try_into_payload::<ThirdPartyArtifact>(fixed_compatibility) {
        Ok(_) => panic!("compatibility mismatch was accepted"),
        Err(artifact) => artifact,
    };
    let artifact = match artifact.try_into_payload::<FixedArtifact>(third_party_compatibility) {
        Ok(_) => panic!("concrete payload mismatch was accepted"),
        Err(artifact) => artifact,
    };
    let payload = match artifact.try_into_payload::<ThirdPartyArtifact>(third_party_compatibility) {
        Ok(payload) => payload,
        Err(_) => panic!("matching payload could not be recovered"),
    };
    assert_eq!(payload.target(), "mock://sensor/temperature");
}
