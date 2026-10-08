use clinkz_wot_core as old;
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_planning::{PlanBuildOutput, select_consumer_property_read};
use clinkz_wot_property_read_binding_fixture::MockArtifact;
use runtime_contracts_probe as new;
pub fn run(output: &PlanBuildOutput<MockArtifact>) -> usize {
    let plan = &output.logical_plans()[0];
    let source_id = output.artifact_refs()[0].identity();
    let id = new::BindingArtifactIdentity::new(
        new::PlanSetGeneration::new(source_id.plan_set_generation().get()),
        new::PlanId::new(source_id.plan_id().slot(), source_id.plan_id().generation()),
        new::BindingId::new(source_id.binding_id().get()),
        new::BindingGeneration::new(source_id.binding_generation().get()),
        new::BindingConfigurationDigest::new(*source_id.configuration().as_bytes()),
        new::BindingArtifactCompatibility::new(*source_id.compatibility().as_bytes()),
        new::BindingArtifactRole::ConsumerCall,
    );
    let plans = [new::PlanFact {
        id: new::PlanId::new(plan.plan_id().slot(), plan.plan_id().generation()),
        name: plan.property_name(),
        form: plan.form_index(),
    }];
    let envelopes = [new::EnvelopeFact {
        id,
        artifact: new::ArtifactFact {
            compatibility: id.compatibility(),
        },
        route: false,
    }];
    let refs = [new::BindingArtifactRef::new(id, SlotIndex::new(0))];
    let view = new::FrozenPlanView {
        plans: &plans,
        envelopes: &envelopes,
        refs: &refs,
    };
    let mut cases = 0;
    for name in ["temperature", "missing", ""] {
        for form in [None, Some(0), Some(1), Some(2), Some(usize::MAX)] {
            let options = match form {
                None => old::InteractionOptions::default(),
                Some(index) => old::InteractionOptions::default().with_form_index(index),
            };
            let old = select_consumer_property_read(output, name, &options);
            let new =
                new::select_consumer_property_read(&view, name, &new::InteractionOptions(form));
            assert_eq!(old.is_ok(), new.is_ok());
            match (old, new) {
                (Ok(a), Ok(b)) => assert_eq!(format!("{a:?}"), format!("{b:?}")),
                (Err(a), Err(b)) => assert_eq!(format!("{a:?}"), format!("{b:?}")),
                _ => unreachable!(),
            }
            cases += 1;
        }
    }
    let changed = new::BindingArtifactIdentity::new(
        new::PlanSetGeneration::new(Generation::INITIAL.checked_next().unwrap()),
        new::PlanId::new(
            SlotIndex::new(0),
            Generation::INITIAL.checked_next().unwrap(),
        ),
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.compatibility(),
        id.role(),
    );
    let mismatched_refs = [new::BindingArtifactRef::new(changed, SlotIndex::new(0))];
    let bad_view = new::FrozenPlanView {
        plans: &plans,
        envelopes: &envelopes,
        refs: &mismatched_refs,
    };
    let projected_error = new::select_consumer_property_read(
        &bad_view,
        "temperature",
        &new::InteractionOptions(None),
    )
    .err()
    .expect("mismatched generation reference must be rejected");
    // Strengthen the scratch experiment: submit the same inconsistent
    // reference to the actual production selector, not just the projection.
    let (plans, artifacts, _) = crate::plan_builder::build().into_parts();
    let changed_old = old::BindingArtifactIdentity::new(
        old::PlanSetGeneration::new(changed.plan_set_generation().get()),
        old::PlanId::new(changed.plan_id().slot(), changed.plan_id().generation()),
        source_id.binding_id(),
        source_id.binding_generation(),
        source_id.configuration(),
        source_id.compatibility(),
        source_id.role(),
    );
    let bad_output = PlanBuildOutput::new(
        plans,
        artifacts,
        vec![old::BindingArtifactRef::new(changed_old, SlotIndex::new(0))],
    );
    let production_error = select_consumer_property_read(
        &bad_output,
        "temperature",
        &old::InteractionOptions::default(),
    )
    .err()
    .expect("production must reject the same mismatch");
    assert_eq!(
        format!("{production_error:?}"),
        format!("{projected_error:?}")
    );
    assert_eq!(cases, 15);
    println!(
        "frozen selection: {cases} cases match owned production output; generation mismatch rejected"
    );
    cases
}
