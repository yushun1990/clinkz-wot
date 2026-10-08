use clinkz_wot_core as old;
use clinkz_wot_foundation::{Generation, SlotIndex};
use clinkz_wot_planning::{PlanBuildOutput, select_consumer_property_read};
use clinkz_wot_property_read_binding_fixture::MockArtifact;
use runtime_contracts_probe as new;

fn project_identity(id: old::BindingArtifactIdentity) -> new::BindingArtifactIdentity {
    new::BindingArtifactIdentity::new(
        new::PlanSetGeneration::new(id.plan_set_generation().get()),
        new::PlanId::new(id.plan_id().slot(), id.plan_id().generation()),
        new::BindingId::new(id.binding_id().get()),
        new::BindingGeneration::new(id.binding_generation().get()),
        new::BindingConfigurationDigest::new(*id.configuration().as_bytes()),
        new::BindingArtifactCompatibility::new(*id.compatibility().as_bytes()),
        match id.role() {
            old::BindingArtifactRole::ConsumerCall => new::BindingArtifactRole::ConsumerCall,
            old::BindingArtifactRole::ConsumerSubscription => {
                new::BindingArtifactRole::ConsumerSubscription
            }
            old::BindingArtifactRole::ProducerRoute => new::BindingArtifactRole::ProducerRoute,
            old::BindingArtifactRole::ProducerPublication => {
                new::BindingArtifactRole::ProducerPublication
            }
        },
    )
}

fn compare(
    output: &PlanBuildOutput<MockArtifact>,
    name: &str,
    form: Option<usize>,
    expected_error: Option<old::SelectionFailureReason>,
) {
    // Observe each production collection independently, including malformed
    // outputs. Never repair envelope identity, role, cardinality or reference slot.
    let plans: Vec<_> = output
        .logical_plans()
        .iter()
        .map(|plan| new::PlanFact {
            id: new::PlanId::new(plan.plan_id().slot(), plan.plan_id().generation()),
            name: plan.property_name(),
            form: plan.form_index(),
        })
        .collect();
    let envelopes: Vec<_> = output
        .artifacts()
        .iter()
        .map(|envelope| new::EnvelopeFact {
            id: project_identity(envelope.identity()),
            artifact: new::ArtifactFact {
                compatibility: new::BindingArtifactCompatibility::new(
                    *envelope.artifact().compatibility().as_bytes(),
                ),
            },
            route: envelope.route_reservation().is_some(),
        })
        .collect();
    let refs: Vec<_> = output
        .artifact_refs()
        .iter()
        .map(|reference| {
            new::BindingArtifactRef::new(
                project_identity(reference.identity()),
                reference.artifact_slot(),
            )
        })
        .collect();
    let view = new::FrozenPlanView {
        plans: &plans,
        envelopes: &envelopes,
        refs: &refs,
    };
    let options = match form {
        None => old::InteractionOptions::default(),
        Some(index) => old::InteractionOptions::default().with_form_index(index),
    };
    let production = select_consumer_property_read(output, name, &options);
    let projected = new::select_consumer_property_read(&view, name, &new::InteractionOptions(form));
    match expected_error {
        None => assert_eq!(
            production
                .as_ref()
                .expect("valid frozen selection must succeed"),
            &output.artifact_refs()[0],
        ),
        Some(expected) => {
            let a = production
                .as_ref()
                .expect_err("invalid frozen selection must fail");
            let b = projected
                .as_ref()
                .expect_err("invalid projected selection must fail");
            assert_eq!(a.selection_reason(), Some(expected));
            // CoreError/ ErrorContext Debug omit reason, operation and retry
            // advice. Compare the reason and complete typed contexts explicitly.
            let expected_projected = match expected {
                old::SelectionFailureReason::AffordanceMissing => {
                    new::SelectionFailureReason::AffordanceMissing
                }
                old::SelectionFailureReason::StrictSelectionMismatch => {
                    new::SelectionFailureReason::StrictSelectionMismatch
                }
                _ => unreachable!("unexpected fixture reason"),
            };
            assert_eq!(b.selection_reason(), Some(expected_projected));
            assert_eq!(
                a.context(),
                &old::ErrorContext::new(old::ErrorPhase::Selection, old::RetryClass::Never)
                    .with_operation(clinkz_wot_td::data_type::Operation::ReadProperty)
            );
            assert_eq!(
                b.context(),
                &new::ErrorContext::new(new::ErrorPhase::Selection, new::RetryClass::Never)
                    .with_operation(new::Operation::ReadProperty)
            );
        }
    }
    assert_eq!(format!("{production:?}"), format!("{projected:?}"));
}

pub fn run(output: &PlanBuildOutput<MockArtifact>) {
    let mut cases = 0;
    let mut successes = 0;
    for name in ["temperature", "missing", ""] {
        for form in [None, Some(0), Some(1), Some(2), Some(usize::MAX)] {
            let expected_error = match (name, form) {
                ("temperature", None | Some(1)) => None,
                ("temperature", _) => Some(old::SelectionFailureReason::StrictSelectionMismatch),
                _ => Some(old::SelectionFailureReason::AffordanceMissing),
            };
            compare(output, name, form, expected_error);
            cases += 1;
            successes += usize::from(expected_error.is_none());
        }
    }
    assert_eq!((cases, successes), (15, 2));

    // Independent malformed production inputs, then project their actual facts.
    // Separate generation changes ensure one identity check cannot mask another.
    for corruption in 0..8 {
        let (mut plans, mut artifacts, mut refs) = crate::plan_builder::build().into_parts();
        let id = refs[0].identity();
        let next = Generation::INITIAL.checked_next().unwrap();
        match corruption {
            0 => plans.clear(),
            1 => artifacts.clear(),
            2 => refs.clear(),
            3 => refs[0] = old::BindingArtifactRef::new(id, SlotIndex::new(1)),
            4..=7 => {
                let changed = old::BindingArtifactIdentity::new(
                    if corruption == 4 {
                        old::PlanSetGeneration::new(next)
                    } else {
                        id.plan_set_generation()
                    },
                    if corruption == 5 {
                        old::PlanId::new(id.plan_id().slot(), next)
                    } else {
                        id.plan_id()
                    },
                    if corruption == 6 {
                        old::BindingId::new(99)
                    } else {
                        id.binding_id()
                    },
                    id.binding_generation(),
                    id.configuration(),
                    id.compatibility(),
                    if corruption == 7 {
                        old::BindingArtifactRole::ConsumerSubscription
                    } else {
                        id.role()
                    },
                );
                refs[0] = old::BindingArtifactRef::new(changed, SlotIndex::new(0));
                if corruption == 7 {
                    let envelope = artifacts.pop().unwrap();
                    artifacts.push(
                        old::BindingArtifactEnvelope::try_new(
                            changed,
                            envelope.admitted(),
                            envelope.into_artifact(),
                        )
                        .unwrap(),
                    );
                }
            }
            _ => unreachable!(),
        }
        let bad_output = PlanBuildOutput::new(plans, artifacts, refs);
        compare(
            &bad_output,
            "temperature",
            None,
            Some(old::SelectionFailureReason::StrictSelectionMismatch),
        );
    }
    println!(
        "frozen selection: {cases} cases match expected outcomes (2 successes); 8 malformed outputs rejected"
    );
}
