//! Immutable protocol-neutral interaction-plan values.

use alloc::boxed::Box;

use clinkz_wot_td::data_type::Operation;

use crate::{
    BindingConfigurationDigest, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext,
    ErrorPhase, PlanId, RetryClass, ThingId,
};

/// Protocol-neutral immutable plan for one effective interaction form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalInteractionPlan {
    plan_id: PlanId,
    thing_id: ThingId,
    property_name: Box<str>,
    form_index: u32,
    resolved_target: Box<str>,
    content_type: Option<Box<str>>,
    subprotocol: Option<Box<str>>,
}

impl LogicalInteractionPlan {
    /// Constructs the owned Property Read projection.
    ///
    /// The result retains no TD, form, source envelope, or input lifetime.
    pub fn try_property_read(
        plan_id: PlanId,
        thing_id: ThingId,
        property_name: Box<str>,
        form_index: u32,
        resolved_target: Box<str>,
        content_type: Option<Box<str>>,
        subprotocol: Option<Box<str>>,
    ) -> CoreResult<Self> {
        if property_name.is_empty() || resolved_target.is_empty() {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                    .with_operation(Operation::ReadProperty)
                    .with_form_index(form_index)
                    .with_plan(plan_id),
            ));
        }
        Ok(Self {
            plan_id,
            thing_id,
            property_name,
            form_index,
            resolved_target,
            content_type,
            subprotocol,
        })
    }

    /// Returns the generation-bearing plan identity.
    pub const fn plan_id(&self) -> PlanId {
        self.plan_id
    }

    /// Returns the owned Thing identity.
    pub const fn thing_id(&self) -> &ThingId {
        &self.thing_id
    }

    /// Returns the frozen interaction operation.
    pub const fn operation(&self) -> Operation {
        Operation::ReadProperty
    }

    /// Returns the owned property name.
    pub const fn property_name(&self) -> &str {
        &self.property_name
    }

    /// Returns the original property-form index.
    pub const fn form_index(&self) -> u32 {
        self.form_index
    }

    /// Returns the resolved target captured during planning.
    pub const fn resolved_target(&self) -> &str {
        &self.resolved_target
    }

    /// Returns the effective content type when one was captured.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Returns the effective subprotocol when one was captured.
    pub fn subprotocol(&self) -> Option<&str> {
        self.subprotocol.as_deref()
    }
}

/// Immutable association between one logical plan and one compiler candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingCandidate {
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: crate::BindingArtifactCompatibility,
    registration_ordinal: u32,
    candidate_order: u32,
}

impl BindingCandidate {
    /// Constructs the complete candidate identity captured by Planning.
    pub const fn new(
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: crate::BindingArtifactCompatibility,
        registration_ordinal: u32,
        candidate_order: u32,
    ) -> Self {
        Self {
            binding_id,
            binding_generation,
            configuration,
            compatibility,
            registration_ordinal,
            candidate_order,
        }
    }

    /// Returns the stable binding identity.
    pub const fn binding_id(&self) -> BindingId {
        self.binding_id
    }

    /// Returns the captured binding generation.
    pub const fn binding_generation(&self) -> BindingGeneration {
        self.binding_generation
    }

    /// Returns the captured configuration digest.
    pub const fn configuration(&self) -> BindingConfigurationDigest {
        self.configuration
    }

    /// Returns the compiler/artifact compatibility identity.
    pub const fn compatibility(&self) -> crate::BindingArtifactCompatibility {
        self.compatibility
    }

    /// Returns the immutable registration-snapshot ordinal.
    pub const fn registration_ordinal(&self) -> u32 {
        self.registration_ordinal
    }

    /// Returns the deterministic candidate-order position.
    pub const fn candidate_order(&self) -> u32 {
        self.candidate_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_foundation::{Generation, SlotIndex};

    fn plan_id() -> PlanId {
        PlanId::new(SlotIndex::new(4), Generation::INITIAL)
    }

    #[test]
    fn property_read_plan_owns_exact_projection() {
        let plan = LogicalInteractionPlan::try_property_read(
            plan_id(),
            ThingId::from("urn:test:owned"),
            Box::from("temperature"),
            3,
            Box::from("mock://sensor/temperature"),
            Some(Box::from("application/json")),
            Some(Box::from("mock-subprotocol")),
        )
        .expect("valid plan");

        assert_eq!(plan.operation(), Operation::ReadProperty);
        assert_eq!(plan.thing_id().as_str(), "urn:test:owned");
        assert_eq!(plan.property_name(), "temperature");
        assert_eq!(plan.form_index(), 3);
        assert_eq!(plan.resolved_target(), "mock://sensor/temperature");
        assert_eq!(plan.content_type(), Some("application/json"));
        assert_eq!(plan.subprotocol(), Some("mock-subprotocol"));
    }

    #[test]
    fn property_read_plan_rejects_empty_required_fields() {
        let error = LogicalInteractionPlan::try_property_read(
            plan_id(),
            ThingId::from("urn:test:invalid"),
            Box::from(""),
            0,
            Box::from("mock://target"),
            None,
            None,
        )
        .expect_err("empty property name must fail");
        assert!(matches!(error, CoreError::Validation(_)));

        let error = LogicalInteractionPlan::try_property_read(
            plan_id(),
            ThingId::from("urn:test:invalid"),
            Box::from("temperature"),
            0,
            Box::from(""),
            None,
            None,
        )
        .expect_err("empty target must fail");
        assert!(matches!(error, CoreError::Validation(_)));
    }
}
