//! Production boundary discriminator for the schema-2 admission.
//!
//! These tests characterize the unchanged strict compiler. They do not model
//! owned registration, supply a binding-key encoding, or prove schema-2 support.
use clinkz_wot_core::*;
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
use consumer_runtime_entry_probe::production::{self, Client, Stats};
use std::sync::{Arc, atomic::Ordering};

fn distinguishing_key() -> BindingConfigurationDigest {
    // Any key distinct from the capacity-only compiler key has the same issue.
    // This is a witness, not the proposed encoding of binding configuration.
    BindingConfigurationDigest::new([0xa5; 32])
}

fn identity(c: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    let original = production::identity(c);
    BindingRegistrationIdentity::new(
        original.binding_id(),
        original.binding_generation(),
        distinguishing_key(),
        original.artifact_compatibility(),
        original.diagnostic_ordinal(),
    )
}

fn plan() -> LogicalInteractionPlan {
    LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(7), Generation::INITIAL),
        ThingId::from("urn:resource-configuration"),
        "x".into(),
        0,
        "x".into(),
        None,
        None,
    )
    .unwrap()
}

fn candidate(
    c: &ResolvedTargetCompiler,
    configuration: BindingConfigurationDigest,
) -> BindingCandidate {
    BindingCandidate::new(
        BindingId::new(223),
        BindingGeneration::INITIAL,
        configuration,
        c.compatibility(),
        0,
        0,
    )
}

#[test]
fn native_configuration_has_only_capacity_and_rejects_a_distinguishing_binding_key() {
    let plan = plan();
    for capacity in 1..=64 {
        let c = ResolvedTargetCompiler::try_new(capacity).unwrap();
        let mut expected = [0; 32];
        expected[..4].copy_from_slice(&[0x63, 0x74, 1, capacity as u8]);
        assert_eq!(c.configuration(), BindingConfigurationDigest::new(expected));
        let strict = BindingCompilerInput::new(
            &plan,
            candidate(&c, c.configuration()),
            BindingArtifactRole::ConsumerCall,
        );
        assert!(c.bounds(&strict).is_ok());
        assert!(c.start(&strict).is_ok());
        let distinct = BindingCompilerInput::new(
            &plan,
            candidate(&c, distinguishing_key()),
            BindingArtifactRole::ConsumerCall,
        );
        assert!(matches!(c.bounds(&distinct), Err(CoreError::Validation(_))));
        assert!(matches!(c.start(&distinct), Err(CoreError::Validation(_))));
        let cursor = c.start(&strict).unwrap();
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2);
        let BindingCompilerStep::Failed(failure) = c.step(&distinct, cursor, &mut budget) else {
            panic!("configuration cannot change during a paid continuation");
        };
        let (_, cursor) = failure.into_parts();
        c.abort(cursor);
    }
}

#[test]
fn configuration_eligibility_does_not_identify_content_or_generation_authority() {
    let registration = production::static_registration(
        ResolvedTargetCompiler::try_new(64).unwrap(),
        Arc::new(Stats::default()),
    );
    let compiler = registration.registration().compiler().compiler();
    let id = registration.registration().identity();
    let mut outputs = Vec::new();
    for (slot, thing, target) in [
        (11, "urn:meter:a", "https://peer/a"),
        (12, "urn:meter:b", "https://peer/b"),
        (13, "urn:meter:c", "https://peer/a"),
    ] {
        let plan = LogicalInteractionPlan::try_property_read(
            PlanId::new(SlotIndex::new(slot), Generation::INITIAL),
            ThingId::from(thing),
            "x".into(),
            0,
            target.into(),
            None,
            None,
        )
        .unwrap();
        let input = BindingCompilerInput::new(
            &plan,
            BindingCandidate::new(
                id.binding_id(),
                id.binding_generation(),
                id.configuration(),
                id.artifact_compatibility(),
                id.diagnostic_ordinal(),
                0,
            ),
            BindingArtifactRole::ConsumerCall,
        );
        let bounds = compiler.bounds(&input).unwrap();
        let admitted = bounds.artifact();
        let mut budget = bounds.into_work();
        let cursor = compiler.start(&input).unwrap();
        let BindingCompilerStep::Pending(cursor) = compiler.step(&input, cursor, &mut budget)
        else {
            panic!("first paid step is pending");
        };
        let BindingCompilerStep::Complete(output) = compiler.step(&input, cursor, &mut budget)
        else {
            panic!("second paid step completes");
        };
        let identity = BindingArtifactIdentity::new(
            PlanSetGeneration::new(Generation::INITIAL),
            plan.plan_id(),
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            BindingArtifactRole::ConsumerCall,
        );
        outputs.push(
            BindingArtifactEnvelope::try_new(identity, admitted, output.into_artifact()).unwrap(),
        );
    }
    // One actual complete owner compiles three values. Equality of the key and
    // format neither identifies content nor merges distinct admitted uses.
    assert_eq!(
        outputs[0].identity().configuration(),
        outputs[1].identity().configuration()
    );
    assert_ne!(
        outputs[0].artifact().payload(),
        outputs[1].artifact().payload()
    );
    assert_eq!(
        outputs[0].artifact().payload(),
        outputs[2].artifact().payload()
    );
    assert_ne!(outputs[0].identity(), outputs[2].identity());
    let original = outputs[0].identity();
    let fresh_generation = BindingArtifactIdentity::new(
        PlanSetGeneration::new(Generation::INITIAL.checked_next().unwrap()),
        original.plan_id(),
        original.binding_id(),
        original.binding_generation(),
        original.configuration(),
        original.compatibility(),
        original.role(),
    );
    assert_ne!(original, fresh_generation);
    // This proves distinct identity values, not live leases, cross-root
    // physical sharing, reusable-program support or owned compiler admission.
}

#[test]
fn static_capture_rejects_a_distinguishing_key_and_recovers_the_complete_owner() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let stats = Arc::new(Stats::default());
    let registration =
        StaticConsumerBindingPreparation::new(StaticConsumerBindingPreparationInput::new(
            id,
            BindingExecutionSupport::application_static(),
            StaticBindingCompilerRegistration::new(c),
            Client {
                compatibility: id.artifact_compatibility(),
                stats: stats.clone(),
            },
            production::resources(),
            BindingStatusPolicy::new(2, 17),
        ))
        .unwrap_or_else(|_| panic!("complete registration accepts consistent compatibility"));
    let rejected = match registration.try_into_consumer_compiler() {
        Err(e) => e,
        Ok(_) => panic!("capacity-only support accepted a distinct binding configuration"),
    };
    assert_eq!(rejected.error().error().context().cause_code(), Some(323));
    let registration = rejected.into_input();
    assert_eq!(registration.identity(), id);
    assert_eq!(
        registration.compiler().compiler().configuration(),
        ResolvedTargetCompiler::try_new(64).unwrap().configuration(),
    );
    let input = registration.into_input();
    assert!(Arc::ptr_eq(&input.client().stats, &stats));
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
    drop(input);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);

    // A positive production control; this test is not just rejecting all input.
    let strict = production::static_registration(
        ResolvedTargetCompiler::try_new(64).unwrap(),
        stats.clone(),
    );
    assert_eq!(strict.support().target_capacity(), 64);
    drop(strict);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 2);
}

#[cfg(feature = "std")]
#[test]
fn host_capture_and_supported_adapter_reject_a_distinguishing_binding_key() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let plan = plan();
    let distinct = BindingCompilerInput::new(
        &plan,
        candidate(&c, distinguishing_key()),
        BindingArtifactRole::ConsumerCall,
    );
    let compiler = HostBindingCompilerRegistration::try_new_consumer(c).unwrap();
    assert!(matches!(
        compiler.bounds(&distinct),
        Err(CoreError::Validation(_))
    ));
    assert!(matches!(
        compiler.start(&distinct),
        Err(CoreError::Validation(_))
    ));
    let stats = Arc::new(Stats::default());
    let input = HostConsumerBindingPreparationInput::new(
        id,
        BindingExecutionSupport::host_erased(),
        compiler,
        Box::new(Client {
            compatibility: id.artifact_compatibility(),
            stats: stats.clone(),
        }),
        production::resources(),
        BindingStatusPolicy::new(2, 17),
    );
    let original_client = input.client() as *const dyn binding::ClientBinding as *const () as usize;
    let registration = HostConsumerBindingPreparation::new(input)
        .unwrap_or_else(|_| panic!("complete Host registration"));
    let rejected = match registration.try_into_consumer_compiler() {
        Err(e) => e,
        Ok(_) => panic!("supported Host adapter accepted a distinct binding configuration"),
    };
    assert_eq!(rejected.error().error().context().cause_code(), Some(323));
    let registration = rejected.into_input();
    assert_eq!(registration.identity(), id);
    let input = registration.into_input();
    assert_eq!(
        input.client() as *const dyn binding::ClientBinding as *const () as usize,
        original_client,
    );
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
    drop(input);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);

    let strict =
        production::host_registration(ResolvedTargetCompiler::try_new(64).unwrap(), stats.clone());
    assert_eq!(strict.support().target_capacity(), 64);
    drop(strict);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 2);
}
