//! Historical policy operands are fixed digests, never replacement behavior.
use super::*;

fn work() -> WorkBudget {
    WorkBudget::new().with_remaining(WorkClass::BindingPolls, 10000)
}
fn registration(
    config: HttpConsumerConfig,
    generation: BindingGeneration,
) -> ConsumerCompilerRegistration<HostConsumerBindingRegistration> {
    try_consumer_registration(config, BindingId::new(600), generation, 600).unwrap()
}
fn plan(target: String) -> LogicalInteractionPlan {
    LogicalInteractionPlan::try_property_read(
        PlanId::new(SlotIndex::new(0), Generation::INITIAL),
        ThingId::from("urn:http-identity"),
        "a".into(),
        0,
        target.into(),
        Some("application/json".into()),
        None,
    )
    .unwrap()
    .with_additional_form_metadata(false)
}
fn candidate(id: BindingRegistrationIdentity) -> BindingCandidate {
    BindingCandidate::new(
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        600,
        0,
    )
}
fn artifact(
    owner: &HostConsumerBindingRegistration,
    plan: &LogicalInteractionPlan,
) -> BindingArtifactEnvelope<HostBindingArtifact> {
    let id = owner.identity();
    let compiler = owner.compiler();
    let thing: clinkz_wot_td::thing::Thing = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"identity", "id":"urn:http-identity",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties":{"a":{"forms":[{"href":plan.resolved_target()}]}}
    })).unwrap();
    let config =
        clinkz_wot_td::ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS)
            .unwrap();
    let mut cursor = clinkz_wot_td::ValidatedThingCursor::from_thing(
        &thing,
        &config,
        AdmissionLedger::new(
            SlotIndex::new(1),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            0,
            0,
            0,
        ),
    );
    let proof = loop {
        let mut budget = WorkClass::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 100_000));
        match cursor.step(&mut budget, false) {
            clinkz_wot_td::ValidatedThingProgress::Pending(next) => cursor = next,
            clinkz_wot_td::ValidatedThingProgress::Complete(proof) => break proof,
            _ => panic!("TD admission failed"),
        }
    };
    let mut resolution_work = WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 100_000));
    let semantics = proof
        .into_property_read()
        .resolve_extensions(&mut resolution_work)
        .unwrap()
        .unwrap();
    let input = BindingCompilerInput::new(plan, candidate(id), BindingArtifactRole::ConsumerCall)
        .with_semantics(&semantics);
    let mut cursor = compiler.start(&input).unwrap();
    loop {
        match compiler.step(&input, cursor, &mut work()) {
            BindingCompilerStep::Pending(next) => cursor = next,
            BindingCompilerStep::Complete(output) => {
                return BindingArtifactEnvelope::try_new(
                    BindingArtifactIdentity::new(
                        PlanSetGeneration::INITIAL,
                        plan.plan_id(),
                        id.binding_id(),
                        id.binding_generation(),
                        id.configuration(),
                        id.artifact_compatibility(),
                        BindingArtifactRole::ConsumerCall,
                    ),
                    output.artifact().footprint(),
                    output.into_artifact(),
                )
                .unwrap();
            }
            _ => panic!("pure compiler failed"),
        }
    }
}
fn request(artifact: &BindingArtifactEnvelope<HostBindingArtifact>) -> OutboundRequest {
    OutboundRequest::property_read(
        BindingArtifactRef::new(artifact.identity(), SlotIndex::new(0)),
        Default::default(),
        None,
    )
    .unwrap()
}
fn config(private: [u8; 32], restricted: bool) -> HttpConsumerConfig {
    assert!(restricted);
    HttpConsumerConfig {
        configuration: ConsumerBindingConfiguration::for_semantic_host(
            64,
            BindingConfigurationDigest::new(private),
            COMPATIBILITY,
            BODY,
            MEDIA,
        )
        .unwrap(),
    }
}
fn digest(hex: &str) -> [u8; 32] {
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

#[test]
fn v6_configuration_support_client_compiler_artifact_and_diagnostics_agree() {
    let current = HttpConsumerConfig::for_cleartext_ip().unwrap();
    let captured = registration(current, BindingGeneration::INITIAL);
    assert_eq!(
        captured.support().consumer_configuration(),
        Some(current.configuration)
    );
    assert!(!current.configuration.requires_default_json_forms());
    assert!(captured.support().uses_semantics());
    let owner = captured.into_registration();
    assert_eq!(
        owner.identity().configuration(),
        current.configuration.configuration()
    );
    let input = owner.into_input();
    assert_eq!(
        input.client().consumer_configuration(),
        Some(current.configuration)
    );
    let owner = HostConsumerBindingRegistration::new(input).unwrap_or_else(|_| panic!());
    let plan = plan("http://127.0.0.1:1/a".into());
    let artifact = artifact(&owner, &plan);
    assert_eq!(
        artifact.identity().configuration(),
        current.configuration.configuration()
    );
    // No runtime is entered. Reaching the actual client rejects with the v6
    // diagnostic before raw acceptance, never opening a connection.
    let rejection = owner
        .start_consumer_property_read(request(&artifact), &artifact)
        .err()
        .unwrap();
    assert_eq!(
        rejection
            .error()
            .error()
            .context()
            .resource_failure()
            .unwrap()
            .0,
        current.configuration.private_policy()
    );
    assert_eq!(
        current.configuration.private_policy(),
        BindingConfigurationDigest::new(PRIVATE)
    );
    println!(
        "v6 private={:02x?} configuration={:02x?}",
        PRIVATE,
        current.configuration.configuration().as_bytes()
    );
}

#[test]
fn historical_policy_operand_configuration_and_artifact_substitution_reject_before_io() {
    let current = HttpConsumerConfig::for_cleartext_ip().unwrap();
    // Historical private-policy operands, rehashed under the new native format.
    // These owners exercise identity substitution, not old native executors.
    let historical = [
        config(
            digest("0abd19e612b06697f1b4f7a562dd8715c25146fe6e6165d166db4e01db361a4f"),
            true,
        ),
        config(
            digest("6f61140140fd464a2a331fec03f3fe236823b9591d32a5f8b05bf8a9b90800dd"),
            true,
        ),
        config(
            digest("ea559362386b3f8682d4fade93ea0e3099e9682cc32783034b0504df03cf52c2"),
            true,
        ),
    ];
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let plan = plan(format!("http://{}/a", listener.local_addr().unwrap()));
    for old in historical {
        assert_ne!(
            old.configuration.configuration(),
            current.configuration.configuration()
        );
        for (actual, foreign) in [(current, old), (old, current)] {
            let owner = registration(actual, BindingGeneration::INITIAL).into_registration();
            let foreign_owner =
                registration(foreign, BindingGeneration::INITIAL).into_registration();
            let input = BindingCompilerInput::new(
                &plan,
                candidate(foreign_owner.identity()),
                BindingArtifactRole::ConsumerCall,
            );
            assert!(matches!(
                owner.compiler().bounds(&input),
                Err(CoreError::Validation(_))
            ));
            assert!(matches!(
                owner.compiler().start(&input),
                Err(CoreError::Validation(_))
            ));
            let foreign_artifact = artifact(&foreign_owner, &plan);
            let original = request(&foreign_artifact);
            let rejected = owner
                .start_consumer_property_read(original, &foreign_artifact)
                .err()
                .unwrap();
            assert_eq!(rejected.input(), &request(&foreign_artifact));
            assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
            let own_artifact = artifact(&owner, &plan);
            let rejected = owner
                .start_consumer_property_read(request(&own_artifact), &foreign_artifact)
                .err()
                .unwrap();
            assert_eq!(rejected.input(), &request(&own_artifact));
            assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
            // Mixing a foreign client into a complete registration returns that
            // same input; a compiler mismatch is caught during support capture.
            let original = owner.into_input();
            let client = Box::new(Client {
                config: foreign,
                busy: Arc::new(AtomicBool::new(false)),
            });
            let ptr = &*client as *const Client;
            let input = HostConsumerBindingRegistrationInput::new(
                original.identity(),
                BindingExecutionSupport::host_erased(),
                HostBindingCompilerRegistration::try_new_semantic(HttpCompiler(actual)).unwrap(),
                client,
                BindingResourceDeclarations::new(
                    BindingLifetimeFootprint::new(3, 1024),
                    BindingLifetimeFootprint::new(16, 8192),
                )
                .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2)
                .with_consumer_result_limits(BODY, MEDIA),
                BindingStatusPolicy::new(2, 17),
            );
            let rejected = HostConsumerBindingRegistration::new(input).err().unwrap();
            assert_eq!(
                rejected.input().client() as *const dyn binding::ClientBinding as *const (),
                ptr.cast()
            );
            assert_eq!(
                rejected.input().client().consumer_configuration(),
                Some(foreign.configuration)
            );
            let input = HostConsumerBindingRegistrationInput::new(
                original.identity(),
                BindingExecutionSupport::host_erased(),
                HostBindingCompilerRegistration::try_new_semantic(HttpCompiler(foreign)).unwrap(),
                Box::new(Client {
                    config: actual,
                    busy: Arc::new(AtomicBool::new(false)),
                }),
                BindingResourceDeclarations::new(
                    BindingLifetimeFootprint::new(3, 1024),
                    BindingLifetimeFootprint::new(16, 8192),
                )
                .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2)
                .with_consumer_result_limits(BODY, MEDIA),
                BindingStatusPolicy::new(2, 17),
            );
            let owner = HostConsumerBindingRegistration::new(input).unwrap_or_else(|_| panic!());
            let rejected = owner
                .try_into_consumer_compiler()
                .err()
                .expect("foreign compiler support capture");
            assert_eq!(rejected.input().identity(), original.identity());
        }
    }
    let current_owner = registration(current, BindingGeneration::INITIAL).into_registration();
    let next_owner = registration(current, BindingGeneration::INITIAL.checked_next().unwrap())
        .into_registration();
    let foreign = artifact(&next_owner, &plan);
    let rejected = current_owner
        .start_consumer_property_read(request(&foreign), &foreign)
        .err()
        .unwrap();
    assert_eq!(rejected.input(), &request(&foreign));
    assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
    assert!(
        listener.accept().is_err(),
        "identity rejection performed I/O"
    );
}
