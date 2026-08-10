#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Profile {
    Static,
    Host,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provenance {
    UpstreamOutput,
    LegalRoot,
    ServientDerived,
    FixtureRestatement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GenerationSet {
    binding: u32,
    produced: u32,
    plan_set: u32,
    plan: u32,
    route: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FirstEntryClosure {
    profile: Profile,
    logical_route: Provenance,
    artifact_metadata: Provenance,
    complete_registration: Provenance,
    produced_thing: Provenance,
    handler_coverage: Provenance,
    admission_policy: Provenance,
    plan_set_ownership: Provenance,
    route_assembly: Provenance,
    activation_cleanup: Provenance,
    artifact_reservation: [u8; 4],
    route_reservation: [u8; 4],
    metadata_survived_erasure: bool,
    registration_complete: bool,
    capacities_reserved: bool,
    binding_side_effect_started: bool,
    host_zero_argument_constructor_preserved: bool,
    manifest_lockfile_transition_complete: bool,
    host_resource_policy_root_constructible: bool,
    generations: GenerationSet,
}

impl FirstEntryClosure {
    fn valid(profile: Profile) -> Self {
        Self {
            profile,
            logical_route: Provenance::UpstreamOutput,
            artifact_metadata: Provenance::UpstreamOutput,
            complete_registration: Provenance::LegalRoot,
            produced_thing: Provenance::ServientDerived,
            handler_coverage: Provenance::LegalRoot,
            admission_policy: Provenance::LegalRoot,
            plan_set_ownership: Provenance::ServientDerived,
            route_assembly: Provenance::ServientDerived,
            activation_cleanup: Provenance::ServientDerived,
            artifact_reservation: [7, 11, 13, 17],
            route_reservation: [7, 11, 13, 17],
            metadata_survived_erasure: true,
            registration_complete: true,
            capacities_reserved: true,
            binding_side_effect_started: false,
            host_zero_argument_constructor_preserved: true,
            manifest_lockfile_transition_complete: true,
            host_resource_policy_root_constructible: true,
            generations: GenerationSet {
                binding: 3,
                produced: 5,
                plan_set: 7,
                plan: 11,
                route: 13,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rejection {
    FixtureAuthority,
    GenerationMismatch,
    ReservationReconstruction,
    HostMetadataLoss,
    ReservationMismatch,
    PartialRegistration,
    SideEffectBeforeReservation,
    ProfileDivergence,
    HostConstructorCompatibility,
    ManifestLockfileTopology,
    HostResourcePolicyRoot,
}

fn validate(
    closure: FirstEntryClosure,
    counterpart: Option<FirstEntryClosure>,
) -> Result<(), Rejection> {
    let required = [
        closure.logical_route,
        closure.artifact_metadata,
        closure.complete_registration,
        closure.produced_thing,
        closure.handler_coverage,
        closure.admission_policy,
        closure.plan_set_ownership,
        closure.route_assembly,
        closure.activation_cleanup,
    ];
    if required.contains(&Provenance::FixtureRestatement) {
        return Err(Rejection::FixtureAuthority);
    }
    if closure.generations.binding == 0
        || closure.generations.produced == 0
        || closure.generations.plan_set == 0
        || closure.generations.plan == 0
        || closure.generations.route == 0
    {
        return Err(Rejection::GenerationMismatch);
    }
    if closure.artifact_metadata != Provenance::UpstreamOutput
        || closure.route_assembly != Provenance::ServientDerived
    {
        return Err(Rejection::ReservationReconstruction);
    }
    if closure.profile == Profile::Host && !closure.metadata_survived_erasure {
        return Err(Rejection::HostMetadataLoss);
    }
    if closure.artifact_reservation != closure.route_reservation {
        return Err(Rejection::ReservationMismatch);
    }
    if closure.complete_registration != Provenance::LegalRoot || !closure.registration_complete {
        return Err(Rejection::PartialRegistration);
    }
    if closure.binding_side_effect_started && !closure.capacities_reserved {
        return Err(Rejection::SideEffectBeforeReservation);
    }
    if closure.profile == Profile::Host && !closure.host_zero_argument_constructor_preserved {
        return Err(Rejection::HostConstructorCompatibility);
    }
    if !closure.manifest_lockfile_transition_complete {
        return Err(Rejection::ManifestLockfileTopology);
    }
    if closure.profile == Profile::Host && !closure.host_resource_policy_root_constructible {
        return Err(Rejection::HostResourcePolicyRoot);
    }
    if let Some(counterpart) = counterpart {
        let same_semantics = closure.logical_route == counterpart.logical_route
            && closure.artifact_metadata == counterpart.artifact_metadata
            && closure.complete_registration == counterpart.complete_registration
            && closure.produced_thing == counterpart.produced_thing
            && closure.handler_coverage == counterpart.handler_coverage
            && closure.admission_policy == counterpart.admission_policy
            && closure.plan_set_ownership == counterpart.plan_set_ownership
            && closure.route_assembly == counterpart.route_assembly
            && closure.activation_cleanup == counterpart.activation_cleanup
            && closure.artifact_reservation == counterpart.artifact_reservation
            && closure.route_reservation == counterpart.route_reservation
            && closure.registration_complete == counterpart.registration_complete
            && closure.capacities_reserved == counterpart.capacities_reserved
            && closure.host_resource_policy_root_constructible
                == counterpart.host_resource_policy_root_constructible
            && closure.generations == counterpart.generations;
        if closure.profile == counterpart.profile || !same_semantics {
            return Err(Rejection::ProfileDivergence);
        }
    }
    Ok(())
}

#[test]
fn all_nine_production_inputs_close_before_prepare_in_both_profiles() {
    let static_closure = FirstEntryClosure::valid(Profile::Static);
    let host_closure = FirstEntryClosure::valid(Profile::Host);
    assert_eq!(validate(static_closure, Some(host_closure)), Ok(()));
    assert_eq!(validate(host_closure, Some(static_closure)), Ok(()));
}

#[test]
fn fixture_restated_artifact_or_reservation_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.artifact_metadata = Provenance::FixtureRestatement;
    assert_eq!(validate(closure, None), Err(Rejection::FixtureAuthority));
}

#[test]
fn dropped_or_mismatched_generation_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.generations.route = 0;
    assert_eq!(validate(closure, None), Err(Rejection::GenerationMismatch));
}

#[test]
fn planning_or_servient_reservation_reconstruction_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.route_assembly = Provenance::LegalRoot;
    assert_eq!(
        validate(closure, None),
        Err(Rejection::ReservationReconstruction)
    );
}

#[test]
fn host_erasure_metadata_loss_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Host);
    closure.metadata_survived_erasure = false;
    assert_eq!(validate(closure, None), Err(Rejection::HostMetadataLoss));
}

#[test]
fn unrelated_reservation_with_real_artifact_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.route_reservation = [19, 23, 29, 31];
    assert_eq!(validate(closure, None), Err(Rejection::ReservationMismatch));
}

#[test]
fn partial_or_bare_registration_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.registration_complete = false;
    assert_eq!(validate(closure, None), Err(Rejection::PartialRegistration));
}

#[test]
fn binding_side_effect_before_reservations_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.capacities_reserved = false;
    closure.binding_side_effect_started = true;
    assert_eq!(
        validate(closure, None),
        Err(Rejection::SideEffectBeforeReservation)
    );
}

#[test]
fn host_static_semantic_divergence_is_rejected() {
    let static_closure = FirstEntryClosure::valid(Profile::Static);
    let mut host_closure = FirstEntryClosure::valid(Profile::Host);
    host_closure.generations.plan_set = 17;
    assert_eq!(
        validate(static_closure, Some(host_closure)),
        Err(Rejection::ProfileDivergence)
    );
}

#[test]
fn one_argument_host_constructor_replacement_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Host);
    closure.host_zero_argument_constructor_preserved = false;
    assert_eq!(
        validate(closure, None),
        Err(Rejection::HostConstructorCompatibility)
    );
}

#[test]
fn manifest_lockfile_transition_omission_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Static);
    closure.manifest_lockfile_transition_complete = false;
    assert_eq!(
        validate(closure, None),
        Err(Rejection::ManifestLockfileTopology)
    );
}

#[test]
fn nonexistent_foundation_default_assumption_is_rejected() {
    let mut closure = FirstEntryClosure::valid(Profile::Host);
    closure.host_resource_policy_root_constructible = false;
    assert_eq!(
        validate(closure, None),
        Err(Rejection::HostResourcePolicyRoot)
    );
}
