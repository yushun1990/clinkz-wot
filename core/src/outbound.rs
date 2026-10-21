//! Selected execution requests; no TD or selection authority.
use crate::*;
use alloc::{collections::BTreeMap, string::String};
/// Owned execution envelope for one already-selected Consumer Property Read.
///
/// Static target and protocol facts remain in the immutable binding artifact.
/// This request is name-free: it carries no `ThingId`, `ThingSlotId`, or
/// `AffordanceTarget`. It holds only the selected artifact reference plus
/// call-varying URI variables and deadline intent; it contains no TD, Form,
/// options, security provider, candidate list, or fallback authority.
/// Human-readable Thing/property identity remains only at API/admission
/// boundaries or in immutable plan/diagnostic storage.
///
/// ```compile_fail
/// # use clinkz_wot_core::OutboundRequest;
/// # fn no_thing_identity(request: &OutboundRequest) {
/// let _ = request.thing_id();
/// # }
/// ```
///
/// ```compile_fail
/// # use clinkz_wot_core::OutboundRequest;
/// # fn no_affordance_target(request: &OutboundRequest) {
/// let _ = request.target();
/// # }
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct OutboundRequest {
    artifact: BindingArtifactRef,
    uri_variables: BTreeMap<String, String>,
    deadline: Option<Deadline>,
}

impl OutboundRequest {
    /// Constructs one structurally valid selected name-free Property Read
    /// request.
    ///
    /// Binding, binding-generation, configuration, plan-set, plan,
    /// compatibility, and role identities derive from the captured artifact
    /// reference; they are not independently supplied fields that may
    /// disagree. Construction fails when the artifact is not a `ConsumerCall`
    /// artifact or when the plan id's generation differs from the artifact's
    /// plan-set generation.
    ///
    /// ```compile_fail
    /// # use std::collections::BTreeMap;
    /// # use clinkz_wot_core::{AffordanceTarget, BindingArtifactRef, OutboundRequest, ThingId};
    /// # fn no_legacy_constructor_shape(
    /// #     thing_id: ThingId,
    /// #     target: AffordanceTarget,
    /// #     artifact: BindingArtifactRef,
    /// # ) -> clinkz_wot_core::CoreResult<OutboundRequest> {
    /// OutboundRequest::property_read(
    ///     thing_id,
    ///     target,
    ///     artifact,
    ///     BTreeMap::new(),
    ///     None,
    /// )
    /// # }
    /// ```
    pub fn property_read(
        artifact: BindingArtifactRef,
        uri_variables: BTreeMap<String, String>,
        deadline: Option<Deadline>,
    ) -> CoreResult<Self> {
        let identity = artifact.identity();
        if identity.role() != BindingArtifactRole::ConsumerCall
            || identity.plan_id().generation() != identity.plan_set_generation().get()
        {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_operation(Operation::ReadProperty)
                    .with_plan(identity.plan_id())
                    .with_binding(identity.binding_id(), identity.binding_generation()),
            ));
        }

        Ok(Self {
            artifact,
            uri_variables,
            deadline,
        })
    }

    /// Returns the operation frozen by this request constructor.
    pub const fn operation(&self) -> Operation {
        Operation::ReadProperty
    }

    /// Returns the exact immutable artifact reference selected by Planning.
    pub const fn artifact(&self) -> BindingArtifactRef {
        self.artifact
    }

    /// Derives the binding id from the selected artifact reference.
    pub const fn binding_id(&self) -> BindingId {
        self.artifact.identity().binding_id()
    }

    /// Derives the binding generation from the selected artifact reference.
    pub const fn binding_generation(&self) -> BindingGeneration {
        self.artifact.identity().binding_generation()
    }

    /// Derives the plan-set generation from the selected artifact reference.
    pub const fn plan_set_generation(&self) -> PlanSetGeneration {
        self.artifact.identity().plan_set_generation()
    }

    /// Derives the selected logical-plan id from the artifact reference.
    pub const fn plan_id(&self) -> PlanId {
        self.artifact.identity().plan_id()
    }

    /// Returns the owned resolved URI-variable values for this call.
    pub fn uri_variables(&self) -> &BTreeMap<String, String> {
        &self.uri_variables
    }

    /// Returns the optional call deadline.
    pub const fn deadline(&self) -> Option<Deadline> {
        self.deadline
    }
}
