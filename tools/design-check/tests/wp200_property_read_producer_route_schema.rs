#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArtifactRole {
    ConsumerCall,
    ProducerRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegistrationIdentity {
    binding: u64,
    generation: u64,
    configuration: [u8; 4],
    compatibility: [u8; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ArtifactRef {
    registration: RegistrationIdentity,
    plan_set_generation: u64,
    plan: u64,
    role: ArtifactRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Cursor {
    registration: RegistrationIdentity,
    plan_set_generation: u64,
    plan: u64,
    role: ArtifactRole,
}

struct PropertyReadPlanCompiler {
    registration: RegistrationIdentity,
    plan: u64,
}

impl PropertyReadPlanCompiler {
    fn producer_route(registration: RegistrationIdentity, plan: u64) -> Self {
        Self { registration, plan }
    }

    fn start(&self, plan_set_generation: u64) -> Cursor {
        Cursor {
            registration: self.registration,
            plan_set_generation,
            plan: self.plan,
            role: ArtifactRole::ProducerRoute,
        }
    }

    fn step(&self, cursor: Cursor, binding_polls: u64) -> Result<ArtifactRef, Cursor> {
        if binding_polls == 0 {
            return Err(cursor);
        }
        Ok(ArtifactRef {
            registration: cursor.registration,
            plan_set_generation: cursor.plan_set_generation,
            plan: cursor.plan,
            role: cursor.role,
        })
    }
}

#[derive(Debug)]
struct PrepareInput(ArtifactRef);

impl PrepareInput {
    fn producer_route(artifact: ArtifactRef) -> Result<Self, ArtifactRef> {
        if artifact.role != ArtifactRole::ProducerRoute {
            return Err(artifact);
        }
        Ok(Self(artifact))
    }
}

fn identity() -> RegistrationIdentity {
    RegistrationIdentity {
        binding: 7,
        generation: 11,
        configuration: [13; 4],
        compatibility: [17; 4],
    }
}

#[test]
fn complete_registration_identity_and_producer_role_reach_prepare_input() {
    let compiler = PropertyReadPlanCompiler::producer_route(identity(), 19);
    let cursor = compiler.start(23);
    let artifact = compiler.step(cursor, 1).expect("bounded completion");
    let prepare = PrepareInput::producer_route(artifact).expect("Producer route");
    assert_eq!(prepare.0.registration, identity());
    assert_eq!(prepare.0.plan_set_generation, 23);
    assert_eq!(prepare.0.plan, 19);
    assert_eq!(prepare.0.role, ArtifactRole::ProducerRoute);
}

#[test]
fn zero_budget_returns_the_identical_opaque_cursor() {
    let compiler = PropertyReadPlanCompiler::producer_route(identity(), 19);
    let cursor = compiler.start(23);
    assert_eq!(compiler.step(cursor, 0), Err(cursor));
}

#[test]
fn consumer_call_mutation_cannot_prepare_a_producer_route() {
    let artifact = ArtifactRef {
        registration: identity(),
        plan_set_generation: 23,
        plan: 19,
        role: ArtifactRole::ConsumerCall,
    };
    assert_eq!(
        PrepareInput::producer_route(artifact).unwrap_err(),
        artifact
    );
}
