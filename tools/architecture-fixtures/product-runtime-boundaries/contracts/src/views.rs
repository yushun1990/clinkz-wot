use crate::*;

// Handwritten observations only. No TD semantics, selection, validation or
// admission rules live here. The borrowed bytes outlive the predicate call;
// this is not a slot lease, result seal or complete registration.
pub struct InteractionOutput<'p> {
    data: Option<&'p [u8]>,
    status: InteractionStatus,
    metadata: InteractionOutputMetadata,
}

impl<'p> InteractionOutput<'p> {
    pub fn new(
        data: Option<&'p [u8]>,
        status: InteractionStatus,
        metadata: InteractionOutputMetadata,
    ) -> Self {
        Self {
            data,
            status,
            metadata,
        }
    }
    pub fn data(&self) -> Option<&[u8]> {
        self.data
    }
    pub fn status(&self) -> InteractionStatus {
        self.status
    }
    pub fn metadata(&self) -> &InteractionOutputMetadata {
        &self.metadata
    }
}

// Deliberately just six logical/mock facts, not an executable firmware image.
pub struct PlanImage {
    pub operation: Operation,
    pub property: &'static str,
    pub form_index: u32,
    pub resolved_target: &'static str,
    pub binding_target: &'static str,
    pub content_type: Option<&'static str>,
}

pub struct PlanFact<'a> {
    pub id: PlanId,
    pub name: &'a str,
    pub form: u32,
}
impl PlanFact<'_> {
    pub fn plan_id(&self) -> PlanId {
        self.id
    }
    pub fn property_name(&self) -> &str {
        self.name
    }
    pub fn form_index(&self) -> u32 {
        self.form
    }
}

pub struct ArtifactFact {
    pub compatibility: BindingArtifactCompatibility,
}
impl ArtifactFact {
    pub fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
}

pub struct EnvelopeFact {
    pub id: BindingArtifactIdentity,
    pub artifact: ArtifactFact,
    pub route: bool,
}
impl EnvelopeFact {
    pub fn identity(&self) -> BindingArtifactIdentity {
        self.id
    }
    pub fn artifact(&self) -> &ArtifactFact {
        &self.artifact
    }
    pub fn route_reservation(&self) -> Option<()> {
        self.route.then_some(())
    }
}

pub struct FrozenPlanView<'a> {
    pub plans: &'a [PlanFact<'a>],
    pub envelopes: &'a [EnvelopeFact],
    pub refs: &'a [BindingArtifactRef],
}
impl<'a> FrozenPlanView<'a> {
    pub fn logical_plans(&self) -> &[PlanFact<'a>] {
        self.plans
    }
    pub fn artifacts(&self) -> &[EnvelopeFact] {
        self.envelopes
    }
    pub fn artifact_refs(&self) -> &[BindingArtifactRef] {
        self.refs
    }
}

pub struct InteractionOptions(pub Option<usize>);
impl InteractionOptions {
    pub fn form_index(&self) -> Option<usize> {
        self.0
    }
}
