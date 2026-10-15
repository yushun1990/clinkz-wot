//! Production boundary discriminator for the schema-2 admission.
//!
//! These tests characterize the unchanged strict compiler. They do not model
//! owned registration, supply a binding-key encoding, or prove schema-2 support.
use clinkz_wot_core::*;
use clinkz_wot_foundation::{Generation, SlotIndex};
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
    }
}

#[test]
fn static_capture_rejects_a_distinguishing_key_and_recovers_the_complete_owner() {
    let c = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&c);
    let stats = Arc::new(Stats::default());
    let registration =
        StaticConsumerBindingRegistration::new(StaticConsumerBindingRegistrationInput::new(
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
    let input = HostConsumerBindingRegistrationInput::new(
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
    let registration = HostConsumerBindingRegistration::new(input)
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
