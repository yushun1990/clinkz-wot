//! First bounded Property Read plan-build algorithm.

use alloc::{boxed::Box, vec};
use core::convert::TryFrom;

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCandidate, BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput,
    BindingCompilerStep, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingRegistrationIdentity, CoreError, CoreResult, ErrorContext, ErrorPhase,
    InteractionOptions, LogicalInteractionPlan, PlanId, RetryClass,
    StaticBindingCompilerRegistration, ThingId,
};
#[cfg(feature = "std")]
use clinkz_wot_core::{
    HostBindingArtifact, HostBindingCompilerCursor, HostBindingCompilerRegistration,
};
use clinkz_wot_foundation::{SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::{
    data_type::{Operation, resolve_form_href},
    td_defaults::{FormContext, effective_form_operations},
    thing::Thing,
};

use crate::{PlanBuildFailure, PlanBuildInput, PlanBuildOutput, PlanBuildStep, PlanCompiler};

trait PropertyReadCompilerRegistration {
    type Cursor;
    type Artifact;

    fn compatibility(&self) -> BindingArtifactCompatibility;
    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds>;
    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor>;
    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact>;
}

impl<C> PropertyReadCompilerRegistration for StaticBindingCompilerRegistration<C>
where
    C: BindingCompilerExtension,
{
    type Cursor = C::Cursor;
    type Artifact = C::Artifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compiler().compatibility()
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.compiler().bounds(input)
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.compiler().start(input)
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.compiler().step(input, cursor, budget)
    }
}

impl<T> PropertyReadCompilerRegistration for &T
where
    T: PropertyReadCompilerRegistration + ?Sized,
{
    type Cursor = T::Cursor;
    type Artifact = T::Artifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        <T as PropertyReadCompilerRegistration>::compatibility(*self)
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        <T as PropertyReadCompilerRegistration>::bounds(*self, input)
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        <T as PropertyReadCompilerRegistration>::start(*self, input)
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        <T as PropertyReadCompilerRegistration>::step(*self, input, cursor, budget)
    }
}

#[cfg(feature = "std")]
impl PropertyReadCompilerRegistration for HostBindingCompilerRegistration {
    type Cursor = HostBindingCompilerCursor;
    type Artifact = HostBindingArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility()
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        self.bounds(input)
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        self.start(input)
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        self.step(input, cursor, budget)
    }
}

#[derive(Debug, Eq, PartialEq)]
enum PropertyReadBuildState<C, A> {
    Start,
    Compiling {
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
        admitted: BindingArtifactFootprint,
        compiler_cursor: C,
    },
    ArtifactReady {
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
        admitted: BindingArtifactFootprint,
        artifact: BindingArtifact<A>,
    },
}

/// Opaque resumable state for one bounded Property Read plan build.
///
/// Callers may retain and return the cursor, but cannot inspect or forge its
/// state.
///
/// ```compile_fail
/// use clinkz_wot_planning::PropertyReadBuildCursor;
/// fn inspect<C, A>(cursor: PropertyReadBuildCursor<C, A>) {
///     let _ = cursor.state;
/// }
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct PropertyReadBuildCursor<C, A> {
    state: PropertyReadBuildState<C, A>,
}

/// Exact bounded compiler for the reviewed Property Read projection.
pub struct PropertyReadPlanCompiler {
    plan_id: PlanId,
    consumer_target: Option<(Box<str>, u32)>,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    registration_index: u32,
    candidate_order: u32,
    role: BindingArtifactRole,
}

impl PropertyReadPlanCompiler {
    /// Creates the exact eager Consumer-call projection for one target coordinate.
    pub fn consumer_call(
        plan_id: PlanId,
        property_name: Box<str>,
        form_index: u32,
        registration: BindingRegistrationIdentity,
        registration_index: u32,
        candidate_order: u32,
    ) -> Self {
        Self {
            plan_id,
            consumer_target: Some((property_name, form_index)),
            binding_id: registration.binding_id(),
            binding_generation: registration.binding_generation(),
            configuration: registration.configuration(),
            compatibility: registration.artifact_compatibility(),
            registration_index,
            candidate_order,
            role: BindingArtifactRole::ConsumerCall,
        }
    }

    /// Creates the exact Producer-route projection for one complete registration.
    pub const fn producer_route(
        plan_id: PlanId,
        registration: BindingRegistrationIdentity,
        registration_index: u32,
        candidate_order: u32,
    ) -> Self {
        Self {
            plan_id,
            consumer_target: None,
            binding_id: registration.binding_id(),
            binding_generation: registration.binding_generation(),
            configuration: registration.configuration(),
            compatibility: registration.artifact_compatibility(),
            registration_index,
            candidate_order,
            role: BindingArtifactRole::ProducerRoute,
        }
    }
}

impl PropertyReadPlanCompiler {
    fn start_impl<R>(
        &self,
        input: &PlanBuildInput<'_, [R]>,
    ) -> CoreResult<PropertyReadBuildCursor<R::Cursor, R::Artifact>>
    where
        R: PropertyReadCompilerRegistration,
    {
        self.registration(input.registrations())?;
        Ok(PropertyReadBuildCursor {
            state: PropertyReadBuildState::Start,
        })
    }

    fn step_impl<R>(
        &self,
        input: &PlanBuildInput<'_, [R]>,
        cursor: PropertyReadBuildCursor<R::Cursor, R::Artifact>,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<PropertyReadBuildCursor<R::Cursor, R::Artifact>, R::Artifact>
    where
        R: PropertyReadCompilerRegistration,
    {
        if budget.remaining(WorkClass::BindingPolls) == 0 {
            return PlanBuildStep::Pending(cursor);
        }

        let (plan, candidate, admitted, compiler_cursor) = match cursor.state {
            PropertyReadBuildState::Start => {
                let plan = match property_read_plan(
                    input.validated_td(),
                    self.plan_id,
                    self.consumer_target
                        .as_ref()
                        .map(|(property_name, form_index)| (property_name.as_ref(), *form_index)),
                ) {
                    Ok(plan) => plan,
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure::new(
                            error,
                            PropertyReadBuildCursor {
                                state: PropertyReadBuildState::Start,
                            },
                        ));
                    }
                };
                let registration = match self.registration(input.registrations()) {
                    Ok(registration) => registration,
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure::new(
                            error,
                            PropertyReadBuildCursor {
                                state: PropertyReadBuildState::Start,
                            },
                        ));
                    }
                };
                let candidate = BindingCandidate::new(
                    self.binding_id,
                    self.binding_generation,
                    self.configuration,
                    self.compatibility,
                    self.registration_index,
                    self.candidate_order,
                );
                let compiler_input = BindingCompilerInput::new(&plan, candidate, self.role);
                let bounds = match registration.bounds(&compiler_input) {
                    Ok(bounds) => bounds,
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure::new(
                            error,
                            PropertyReadBuildCursor {
                                state: PropertyReadBuildState::Start,
                            },
                        ));
                    }
                };
                if !compiler_work_is_portable(&bounds) {
                    return PlanBuildStep::Failed(PlanBuildFailure::new(
                        compiler_contract_error(&plan, candidate),
                        PropertyReadBuildCursor {
                            state: PropertyReadBuildState::Start,
                        },
                    ));
                }
                let admitted = bounds.artifact();
                let compiler_cursor = match registration.start(&compiler_input) {
                    Ok(cursor) => cursor,
                    Err(error) => {
                        return PlanBuildStep::Failed(PlanBuildFailure::new(
                            error,
                            PropertyReadBuildCursor {
                                state: PropertyReadBuildState::Start,
                            },
                        ));
                    }
                };
                (plan, candidate, admitted, compiler_cursor)
            }
            PropertyReadBuildState::Compiling {
                plan,
                candidate,
                admitted,
                compiler_cursor,
            } => (plan, candidate, admitted, compiler_cursor),
            PropertyReadBuildState::ArtifactReady {
                plan,
                candidate,
                admitted,
                artifact,
            } => {
                return finish_property_read_build(
                    input.plan_set_generation(),
                    plan,
                    candidate,
                    admitted,
                    artifact,
                    self.role,
                );
            }
        };

        let registration = match self.registration(input.registrations()) {
            Ok(registration) => registration,
            Err(error) => {
                return PlanBuildStep::Failed(PlanBuildFailure::new(
                    error,
                    PropertyReadBuildCursor {
                        state: PropertyReadBuildState::Compiling {
                            plan,
                            candidate,
                            admitted,
                            compiler_cursor,
                        },
                    },
                ));
            }
        };
        let compiler_input = BindingCompilerInput::new(&plan, candidate, self.role);
        match registration.step(&compiler_input, compiler_cursor, budget) {
            BindingCompilerStep::Pending(compiler_cursor) => {
                PlanBuildStep::Pending(PropertyReadBuildCursor {
                    state: PropertyReadBuildState::Compiling {
                        plan,
                        candidate,
                        admitted,
                        compiler_cursor,
                    },
                })
            }
            BindingCompilerStep::Complete(output) => finish_property_read_build(
                input.plan_set_generation(),
                plan,
                candidate,
                admitted,
                output.into_artifact(),
                self.role,
            ),
            BindingCompilerStep::Failed(failure) => {
                let (error, compiler_cursor) = failure.into_parts();
                PlanBuildStep::Failed(PlanBuildFailure::new(
                    error,
                    PropertyReadBuildCursor {
                        state: PropertyReadBuildState::Compiling {
                            plan,
                            candidate,
                            admitted,
                            compiler_cursor,
                        },
                    },
                ))
            }
        }
    }

    fn abort_impl<C, A>(&self, _cursor: PropertyReadBuildCursor<C, A>) {}
}

impl<C> PlanCompiler<[StaticBindingCompilerRegistration<C>]> for PropertyReadPlanCompiler
where
    C: BindingCompilerExtension,
{
    type Cursor = PropertyReadBuildCursor<C::Cursor, C::Artifact>;
    type Artifact = C::Artifact;

    fn start(
        &self,
        input: &PlanBuildInput<'_, [StaticBindingCompilerRegistration<C>]>,
    ) -> CoreResult<Self::Cursor> {
        self.start_impl(input)
    }

    fn step(
        &self,
        input: &PlanBuildInput<'_, [StaticBindingCompilerRegistration<C>]>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact> {
        self.step_impl(input, cursor, budget)
    }

    fn abort(&self, cursor: Self::Cursor) {
        self.abort_impl(cursor);
    }
}

impl<'a, C> PlanCompiler<[&'a StaticBindingCompilerRegistration<C>]> for PropertyReadPlanCompiler
where
    C: BindingCompilerExtension,
{
    type Cursor = PropertyReadBuildCursor<C::Cursor, C::Artifact>;
    type Artifact = C::Artifact;

    fn start(
        &self,
        input: &PlanBuildInput<'_, [&'a StaticBindingCompilerRegistration<C>]>,
    ) -> CoreResult<Self::Cursor> {
        self.start_impl(input)
    }

    fn step(
        &self,
        input: &PlanBuildInput<'_, [&'a StaticBindingCompilerRegistration<C>]>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact> {
        self.step_impl(input, cursor, budget)
    }

    fn abort(&self, cursor: Self::Cursor) {
        self.abort_impl(cursor);
    }
}

#[cfg(feature = "std")]
impl PlanCompiler<[HostBindingCompilerRegistration]> for PropertyReadPlanCompiler {
    type Cursor = PropertyReadBuildCursor<HostBindingCompilerCursor, HostBindingArtifact>;
    type Artifact = HostBindingArtifact;

    fn start(
        &self,
        input: &PlanBuildInput<'_, [HostBindingCompilerRegistration]>,
    ) -> CoreResult<Self::Cursor> {
        self.start_impl(input)
    }

    fn step(
        &self,
        input: &PlanBuildInput<'_, [HostBindingCompilerRegistration]>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact> {
        self.step_impl(input, cursor, budget)
    }

    fn abort(&self, cursor: Self::Cursor) {
        self.abort_impl(cursor);
    }
}

#[cfg(feature = "std")]
impl<'a> PlanCompiler<[&'a HostBindingCompilerRegistration]> for PropertyReadPlanCompiler {
    type Cursor = PropertyReadBuildCursor<HostBindingCompilerCursor, HostBindingArtifact>;
    type Artifact = HostBindingArtifact;

    fn start(
        &self,
        input: &PlanBuildInput<'_, [&'a HostBindingCompilerRegistration]>,
    ) -> CoreResult<Self::Cursor> {
        self.start_impl(input)
    }

    fn step(
        &self,
        input: &PlanBuildInput<'_, [&'a HostBindingCompilerRegistration]>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact> {
        self.step_impl(input, cursor, budget)
    }

    fn abort(&self, cursor: Self::Cursor) {
        self.abort_impl(cursor);
    }
}

impl PropertyReadPlanCompiler {
    fn registration<'a, R>(&self, registrations: &'a [R]) -> CoreResult<&'a R>
    where
        R: PropertyReadCompilerRegistration,
    {
        let index = usize::try_from(self.registration_index).map_err(|_| {
            selection_error(clinkz_wot_core::SelectionFailureReason::NoSupportingBinding)
        })?;
        let registration = registrations.get(index).ok_or_else(|| {
            selection_error(clinkz_wot_core::SelectionFailureReason::NoSupportingBinding)
        })?;
        if registration.compatibility() != self.compatibility {
            return Err(selection_error(
                clinkz_wot_core::SelectionFailureReason::NoSupportingBinding,
            ));
        }
        Ok(registration)
    }
}

fn compiler_work_is_portable(bounds: &BindingCompilerBounds) -> bool {
    WorkClass::ALL
        .into_iter()
        .all(|class| class == WorkClass::BindingPolls || bounds.work().remaining(class) == 0)
}

fn property_read_plan(
    td: &Thing,
    plan_id: PlanId,
    consumer_target: Option<(&str, u32)>,
) -> CoreResult<LogicalInteractionPlan> {
    let thing_id = td
        .id
        .as_ref()
        .map(|id| ThingId::from(id.as_str()))
        .ok_or_else(document_error)?;
    let properties = td.properties.as_ref().ok_or_else(|| {
        selection_error(clinkz_wot_core::SelectionFailureReason::AffordanceMissing)
    })?;

    if let Some((property_name, form_index)) = consumer_target {
        let property = properties.get(property_name).ok_or_else(|| {
            selection_error(clinkz_wot_core::SelectionFailureReason::AffordanceMissing)
        })?;
        let form = usize::try_from(form_index)
            .ok()
            .and_then(|index| property._interaction.forms.get(index))
            .ok_or_else(|| {
                selection_error(clinkz_wot_core::SelectionFailureReason::StrictSelectionMismatch)
            })?;
        if !effective_form_operations(FormContext::Property(property), form)
            .contains(&Operation::ReadProperty)
        {
            return Err(selection_error(
                clinkz_wot_core::SelectionFailureReason::NoFormSupportsOperation,
            ));
        }
        return make_property_read_plan(td, plan_id, thing_id, property_name, form_index, form);
    }

    for (property_name, property) in properties {
        for (form_index, form) in property._interaction.forms.iter().enumerate() {
            let supports_read = match &form.op {
                Some(operations) => operations.contains(&Operation::ReadProperty),
                None => true,
            };
            if !supports_read {
                continue;
            }
            let form_index = u32::try_from(form_index).map_err(|_| {
                CoreError::InternalInvariant(
                    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                        .with_operation(Operation::ReadProperty)
                        .with_plan(plan_id),
                )
            })?;
            return make_property_read_plan(td, plan_id, thing_id, property_name, form_index, form);
        }
    }

    Err(selection_error(
        clinkz_wot_core::SelectionFailureReason::NoFormSupportsOperation,
    ))
}

fn make_property_read_plan(
    td: &Thing,
    plan_id: PlanId,
    thing_id: ThingId,
    property_name: &str,
    form_index: u32,
    form: &clinkz_wot_td::form::Form,
) -> CoreResult<LogicalInteractionPlan> {
    let resolved = resolve_form_href(td.base.as_ref(), &form.href).map_err(|_| {
        selection_error(clinkz_wot_core::SelectionFailureReason::TargetResolutionFailed)
    })?;
    LogicalInteractionPlan::try_property_read(
        plan_id,
        thing_id,
        Box::from(property_name),
        form_index,
        Box::from(resolved.as_str()),
        Some(Box::from(form.content_type.as_str())),
        form.subprotocol.as_deref().map(Box::from),
    )
}

/// Selects the one eager Consumer Property Read artifact from immutable plan data.
pub fn select_consumer_property_read<A>(
    output: &PlanBuildOutput<A>,
    property_name: &str,
    options: &InteractionOptions,
) -> CoreResult<BindingArtifactRef> {
    let [plan] = output.logical_plans() else {
        return Err(selection_error(
            clinkz_wot_core::SelectionFailureReason::StrictSelectionMismatch,
        ));
    };
    if plan.property_name() != property_name {
        return Err(selection_error(
            clinkz_wot_core::SelectionFailureReason::AffordanceMissing,
        ));
    }
    if options
        .form_index()
        .is_some_and(|form_index| u32::try_from(form_index).ok() != Some(plan.form_index()))
    {
        return Err(selection_error(
            clinkz_wot_core::SelectionFailureReason::StrictSelectionMismatch,
        ));
    }

    let [envelope] = output.artifacts() else {
        return Err(selection_error(
            clinkz_wot_core::SelectionFailureReason::StrictSelectionMismatch,
        ));
    };
    let [artifact_ref] = output.artifact_refs() else {
        return Err(selection_error(
            clinkz_wot_core::SelectionFailureReason::StrictSelectionMismatch,
        ));
    };
    let identity = artifact_ref.identity();
    if artifact_ref.artifact_slot() != SlotIndex::new(0)
        || envelope.identity() != identity
        || plan.plan_id() != identity.plan_id()
        || identity.role() != BindingArtifactRole::ConsumerCall
        || envelope.artifact().compatibility() != identity.compatibility()
        || envelope.route_reservation().is_some()
    {
        return Err(selection_error(
            clinkz_wot_core::SelectionFailureReason::StrictSelectionMismatch,
        ));
    }

    Ok(*artifact_ref)
}

fn finish_property_read_build<C, A>(
    plan_set_generation: clinkz_wot_core::PlanSetGeneration,
    plan: LogicalInteractionPlan,
    candidate: BindingCandidate,
    admitted: BindingArtifactFootprint,
    artifact: BindingArtifact<A>,
    role: BindingArtifactRole,
) -> PlanBuildStep<PropertyReadBuildCursor<C, A>, A> {
    let route_reservation = artifact.route_reservation();
    let identity = BindingArtifactIdentity::new(
        plan_set_generation,
        plan.plan_id(),
        candidate.binding_id(),
        candidate.binding_generation(),
        candidate.configuration(),
        candidate.compatibility(),
        role,
    );
    let envelope = match BindingArtifactEnvelope::try_new(identity, admitted, artifact) {
        Ok(envelope) => envelope,
        Err(rejection) => {
            return PlanBuildStep::Failed(PlanBuildFailure::new(
                compiler_contract_error(&plan, candidate),
                PropertyReadBuildCursor {
                    state: PropertyReadBuildState::ArtifactReady {
                        plan,
                        candidate,
                        admitted,
                        artifact: rejection.into_artifact(),
                    },
                },
            ));
        }
    };
    debug_assert_eq!(envelope.route_reservation(), route_reservation);
    PlanBuildStep::Complete(PlanBuildOutput::new(
        vec![plan],
        vec![envelope],
        vec![BindingArtifactRef::new(identity, SlotIndex::new(0))],
    ))
}

fn document_error() -> CoreError {
    CoreError::Validation(
        ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
            .with_operation(Operation::ReadProperty),
    )
}

fn selection_error(reason: clinkz_wot_core::SelectionFailureReason) -> CoreError {
    CoreError::Selection {
        reason,
        context: ErrorContext::new(ErrorPhase::Selection, RetryClass::Never)
            .with_operation(Operation::ReadProperty),
    }
}

fn compiler_contract_error(
    plan: &LogicalInteractionPlan,
    candidate: BindingCandidate,
) -> CoreError {
    CoreError::InternalInvariant(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_operation(Operation::ReadProperty)
            .with_form_index(plan.form_index())
            .with_plan(plan.plan_id())
            .with_binding(candidate.binding_id(), candidate.binding_generation()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_core::binding::BindingRouteKey;
    use clinkz_wot_core::{
        BindingCompilerOutput, BindingLifetimeFootprint, BindingRegistrationIdentity,
        CollisionDomainId, EndpointReservationKey, InteractionOptions, Payload, PlanSetGeneration,
        PrepareInput, RouteReservationIdentity, SelectionFailureReason,
    };
    use clinkz_wot_foundation::Generation;
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        form::Form,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct MockArtifact {
        target: Box<str>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct MockCursor {
        remaining: u8,
    }

    #[derive(Clone, Copy)]
    struct MockCompiler {
        compatibility: BindingArtifactCompatibility,
    }

    fn route_reservation() -> RouteReservationIdentity {
        RouteReservationIdentity::new(
            CollisionDomainId::new([0x21; 16]),
            EndpointReservationKey::new([0x22; 32]),
        )
    }

    impl BindingCompilerExtension for MockCompiler {
        type Cursor = MockCursor;
        type Artifact = MockArtifact;

        fn compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
        }

        fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(
                    1,
                    input.logical_plan().resolved_target().len() as u64,
                ),
                core::mem::size_of::<MockCursor>() as u64,
                0,
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2),
            ))
        }

        fn start(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
            Ok(MockCursor { remaining: 2 })
        }

        fn step(
            &self,
            input: &BindingCompilerInput<'_>,
            mut cursor: Self::Cursor,
            budget: &mut WorkBudget,
        ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return BindingCompilerStep::Pending(cursor);
            }
            cursor.remaining -= 1;
            if cursor.remaining != 0 {
                return BindingCompilerStep::Pending(cursor);
            }
            let target: Box<str> = input.logical_plan().resolved_target().into();
            let footprint = BindingArtifactFootprint::new(1, target.len() as u64);
            let artifact = if input.role() == BindingArtifactRole::ProducerRoute {
                BindingArtifact::producer_route(
                    self.compatibility,
                    footprint,
                    route_reservation(),
                    MockArtifact { target },
                )
            } else {
                BindingArtifact::new(self.compatibility, footprint, MockArtifact { target })
            };
            BindingCompilerStep::Complete(BindingCompilerOutput::new(artifact))
        }

        fn abort(&self, _cursor: Self::Cursor) {}
    }

    fn thing() -> Thing {
        Thing::builder("Tank")
            .id("urn:test:tank")
            .nosec()
            .property(
                "level",
                PropertyAffordance::builder(DataSchema::number())
                    .form(
                        Form::read_property("mock://tank/level")
                            .build()
                            .expect("valid form"),
                    )
                    .build()
                    .expect("valid property"),
            )
            .build()
            .expect("valid thing")
    }

    fn competing_thing() -> Thing {
        Thing::builder("Consumer target fixture")
            .id("urn:test:consumer-target")
            .nosec()
            .property(
                "alpha",
                PropertyAffordance::builder(DataSchema::number())
                    .form(
                        Form::read_property("mock://alpha/read")
                            .build()
                            .expect("valid competing form"),
                    )
                    .build()
                    .expect("valid competing property"),
            )
            .property(
                "target",
                PropertyAffordance::builder(DataSchema::number())
                    .form(
                        Form::read_property("mock://target/first")
                            .build()
                            .expect("valid first target form"),
                    )
                    .form(
                        Form::read_property("mock://target/selected")
                            .build()
                            .expect("valid selected target form"),
                    )
                    .form(
                        Form::write_property("mock://target/write-only")
                            .build()
                            .expect("valid non-read target form"),
                    )
                    .build()
                    .expect("valid target property"),
            )
            .build()
            .expect("valid competing thing")
    }

    fn plan_id() -> PlanId {
        PlanId::new(SlotIndex::new(3), Generation::INITIAL)
    }

    fn registration_identity() -> BindingRegistrationIdentity {
        BindingRegistrationIdentity::new(
            BindingId::new(11),
            BindingGeneration::new(Generation::INITIAL),
            BindingConfigurationDigest::new([12; 32]),
            BindingArtifactCompatibility::new([13; 16]),
            7,
        )
    }

    fn compiler(property_name: &str, form_index: u32) -> PropertyReadPlanCompiler {
        PropertyReadPlanCompiler::consumer_call(
            plan_id(),
            Box::from(property_name),
            form_index,
            registration_identity(),
            0,
            0,
        )
    }

    fn build_static(step_budget: u64) -> PlanBuildOutput<MockArtifact> {
        let td = thing();
        let compatibility = BindingArtifactCompatibility::new([13; 16]);
        let registrations = [StaticBindingCompilerRegistration::new(MockCompiler {
            compatibility,
        })];
        let input = PlanBuildInput::new(
            &td,
            &registrations[..],
            PlanSetGeneration::new(Generation::INITIAL),
        );
        let compiler = compiler("level", 0);
        let mut cursor = compiler.start(&input).expect("build cursor");
        loop {
            let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, step_budget);
            cursor = match compiler.step(&input, cursor, &mut budget) {
                PlanBuildStep::Pending(cursor) => cursor,
                PlanBuildStep::Complete(output) => return output,
                PlanBuildStep::Failed(failure) => {
                    panic!("property-read build failed: {:?}", failure.error())
                }
            };
        }
    }

    #[test]
    fn zero_budget_preserves_start_and_owned_output_survives_inputs() {
        let output = {
            let td = thing();
            let compatibility = BindingArtifactCompatibility::new([13; 16]);
            let registrations = [StaticBindingCompilerRegistration::new(MockCompiler {
                compatibility,
            })];
            let input = PlanBuildInput::new(
                &td,
                &registrations[..],
                PlanSetGeneration::new(Generation::INITIAL),
            );
            let compiler = compiler("level", 0);
            let cursor = compiler.start(&input).expect("build cursor");
            let mut zero = WorkBudget::new();
            let cursor = match compiler.step(&input, cursor, &mut zero) {
                PlanBuildStep::Pending(cursor)
                    if matches!(&cursor.state, PropertyReadBuildState::Start) =>
                {
                    cursor
                }
                _ => panic!("zero budget advanced the planner"),
            };
            let mut first = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
            let cursor = match compiler.step(&input, cursor, &mut first) {
                PlanBuildStep::Pending(cursor) => cursor,
                _ => panic!("first compiler step must remain pending"),
            };
            let mut second = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
            match compiler.step(&input, cursor, &mut second) {
                PlanBuildStep::Complete(output) => output,
                _ => panic!("second compiler step must complete"),
            }
        };

        assert_eq!(
            output.logical_plans()[0].thing_id().as_str(),
            "urn:test:tank"
        );
        assert_eq!(output.logical_plans()[0].property_name(), "level");
        assert_eq!(
            output.artifacts()[0].artifact().payload().target.as_ref(),
            "mock://tank/level"
        );
        assert_eq!(output.artifact_refs()[0].artifact_slot(), SlotIndex::new(0));
        assert_eq!(
            select_consumer_property_read(&output, "level", &InteractionOptions::new())
                .expect("owned output selects after every build input is dropped"),
            output.artifact_refs()[0]
        );
    }

    #[test]
    fn step_budget_partition_does_not_change_output() {
        assert_eq!(build_static(1), build_static(2));
    }

    #[cfg(feature = "std")]
    #[test]
    fn static_and_host_outputs_share_identity_and_footprint() {
        let static_output = build_static(1);
        let host_output = {
            let td = thing();
            let compatibility = BindingArtifactCompatibility::new([13; 16]);
            let registrations = [HostBindingCompilerRegistration::new(MockCompiler {
                compatibility,
            })];
            let input = PlanBuildInput::new(
                &td,
                &registrations[..],
                PlanSetGeneration::new(Generation::INITIAL),
            );
            let compiler = compiler("level", 0);
            let mut cursor = compiler.start(&input).expect("host build cursor");
            loop {
                let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
                cursor = match compiler.step(&input, cursor, &mut budget) {
                    PlanBuildStep::Pending(cursor) => cursor,
                    PlanBuildStep::Complete(output) => break output,
                    PlanBuildStep::Failed(failure) => {
                        panic!("host property-read build failed: {:?}", failure.error())
                    }
                };
            }
        };

        assert_eq!(
            static_output.artifacts()[0].identity(),
            host_output.artifacts()[0].identity()
        );
        assert_eq!(
            static_output.artifacts()[0].artifact().footprint(),
            host_output.artifacts()[0].artifact().footprint()
        );
        assert_eq!(
            host_output.artifacts()[0]
                .artifact()
                .try_payload::<MockArtifact>(BindingArtifactCompatibility::new([13; 16]))
                .map(|artifact| artifact.target.as_ref()),
            Some("mock://tank/level")
        );
        assert_eq!(
            select_consumer_property_read(&static_output, "level", &InteractionOptions::new())
                .expect("static selection"),
            select_consumer_property_read(&host_output, "level", &InteractionOptions::new())
                .expect("Host selection")
        );
    }

    #[test]
    fn consumer_call_compiles_only_the_exact_non_first_coordinate() {
        let td = competing_thing();
        let compatibility = BindingArtifactCompatibility::new([13; 16]);
        let registrations = [StaticBindingCompilerRegistration::new(MockCompiler {
            compatibility,
        })];
        let input = PlanBuildInput::new(
            &td,
            &registrations[..],
            PlanSetGeneration::new(Generation::INITIAL),
        );
        let compiler = compiler("target", 1);
        let mut cursor = compiler.start(&input).expect("exact target cursor");
        let output = loop {
            let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
            cursor = match compiler.step(&input, cursor, &mut budget) {
                PlanBuildStep::Pending(cursor) => cursor,
                PlanBuildStep::Complete(output) => break output,
                PlanBuildStep::Failed(failure) => {
                    panic!("exact target build failed: {:?}", failure.error())
                }
            };
        };

        let plan = &output.logical_plans()[0];
        assert_eq!(plan.property_name(), "target");
        assert_eq!(plan.form_index(), 1);
        assert_eq!(plan.resolved_target(), "mock://target/selected");
        let selected = select_consumer_property_read(
            &output,
            "target",
            &InteractionOptions::new().with_form_index(1),
        )
        .expect("matching strict selection");
        assert_eq!(selected, output.artifact_refs()[0]);
        assert_eq!(
            selected.identity().binding_id(),
            registration_identity().binding_id()
        );
        assert_eq!(
            selected.identity().binding_generation(),
            registration_identity().binding_generation()
        );
        assert_eq!(
            selected.identity().configuration(),
            registration_identity().configuration()
        );
        assert_eq!(
            selected.identity().compatibility(),
            registration_identity().artifact_compatibility()
        );
        assert_eq!(
            selected.identity().role(),
            BindingArtifactRole::ConsumerCall
        );
        assert!(output.artifacts()[0].route_reservation().is_none());
    }

    fn build_failure(property_name: &str, form_index: u32) -> CoreError {
        let td = competing_thing();
        let registrations = [StaticBindingCompilerRegistration::new(MockCompiler {
            compatibility: BindingArtifactCompatibility::new([13; 16]),
        })];
        let input = PlanBuildInput::new(
            &td,
            &registrations[..],
            PlanSetGeneration::new(Generation::INITIAL),
        );
        let compiler = compiler(property_name, form_index);
        let cursor = compiler.start(&input).expect("failure-case cursor");
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        match compiler.step(&input, cursor, &mut budget) {
            PlanBuildStep::Failed(failure) => failure.into_parts().0,
            other => panic!("invalid exact target unexpectedly progressed: {other:?}"),
        }
    }

    #[test]
    fn exact_target_failures_never_fall_back_to_readable_competitors() {
        assert!(matches!(
            build_failure("missing", 0),
            CoreError::Selection {
                reason: SelectionFailureReason::AffordanceMissing,
                ..
            }
        ));
        assert!(matches!(
            build_failure("target", 99),
            CoreError::Selection {
                reason: SelectionFailureReason::StrictSelectionMismatch,
                ..
            }
        ));
        assert!(matches!(
            build_failure("target", 2),
            CoreError::Selection {
                reason: SelectionFailureReason::NoFormSupportsOperation,
                ..
            }
        ));
    }

    #[test]
    fn immutable_selector_uses_only_narrow_options_and_rejects_mismatch() {
        let output = build_static(1);
        let omitted = select_consumer_property_read(&output, "level", &InteractionOptions::new())
            .expect("omitted form selection");
        let call_varying = InteractionOptions::with_data(Payload::new(
            b"poisoned-legacy-data".to_vec(),
            "application/octet-stream",
        ))
        .with_uri_variable("poison", "must-not-replan")
        .with_timeout(core::time::Duration::from_millis(1))
        .with_form_index(0);
        assert_eq!(
            select_consumer_property_read(&output, "level", &call_varying)
                .expect("call-varying values do not alter the static reference"),
            omitted
        );
        assert!(matches!(
            select_consumer_property_read(&output, "other", &InteractionOptions::new()),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::AffordanceMissing,
                ..
            })
        ));
        assert!(matches!(
            select_consumer_property_read(
                &output,
                "level",
                &InteractionOptions::new().with_form_index(1),
            ),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::StrictSelectionMismatch,
                ..
            })
        ));
    }

    #[test]
    fn immutable_selector_rejects_forged_identity_and_artifact_slot() {
        let output = build_static(1);
        let (plans, artifacts, refs) = output.into_parts();
        let identity = refs[0].identity();
        let forged_identity = BindingArtifactIdentity::new(
            identity.plan_set_generation(),
            identity.plan_id(),
            BindingId::new(99),
            identity.binding_generation(),
            identity.configuration(),
            identity.compatibility(),
            identity.role(),
        );
        let forged = PlanBuildOutput::new(
            plans,
            artifacts,
            vec![BindingArtifactRef::new(forged_identity, SlotIndex::new(1))],
        );
        assert!(matches!(
            select_consumer_property_read(&forged, "level", &InteractionOptions::new()),
            Err(CoreError::Selection {
                reason: SelectionFailureReason::StrictSelectionMismatch,
                ..
            })
        ));
    }

    #[test]
    fn producer_route_artifact_reaches_prepare_input_with_compiler_reservation() {
        let td = thing();
        let compatibility = BindingArtifactCompatibility::new([13; 16]);
        let registration = BindingRegistrationIdentity::new(
            BindingId::new(11),
            BindingGeneration::new(Generation::INITIAL),
            BindingConfigurationDigest::new([12; 32]),
            compatibility,
            0,
        );
        let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
        let registrations = [StaticBindingCompilerRegistration::new(MockCompiler {
            compatibility,
        })];
        let input = PlanBuildInput::new(&td, &registrations[..], plan_set_generation);
        let compiler = PropertyReadPlanCompiler::producer_route(plan_id(), registration, 0, 0);
        let cursor = compiler.start(&input).expect("Producer-route cursor");
        let mut budget = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2);
        let output = match compiler.step(&input, cursor, &mut budget) {
            PlanBuildStep::Complete(output) => output,
            PlanBuildStep::Pending(cursor) => match compiler.step(&input, cursor, &mut budget) {
                PlanBuildStep::Complete(output) => output,
                other => panic!("Producer-route compiler did not complete: {other:?}"),
            },
            PlanBuildStep::Failed(failure) => {
                panic!("Producer-route compiler failed: {:?}", failure.error())
            }
        };

        let artifact_ref = output.artifact_refs()[0];
        let envelope = &output.artifacts()[artifact_ref.artifact_slot().get() as usize];
        assert_eq!(
            artifact_ref.identity().role(),
            BindingArtifactRole::ProducerRoute
        );
        assert_eq!(
            artifact_ref.identity().binding_id(),
            registration.binding_id()
        );
        assert_eq!(
            artifact_ref.identity().binding_generation(),
            registration.binding_generation(),
        );
        assert_eq!(
            artifact_ref.identity().plan_set_generation(),
            plan_set_generation
        );
        assert_eq!(envelope.identity(), artifact_ref.identity());

        let reservation = envelope
            .route_reservation()
            .expect("Producer-route compiler owns canonical reservation identity");
        assert_eq!(reservation, route_reservation());
        let route = BindingRouteKey::new(
            artifact_ref.identity().binding_id(),
            artifact_ref.identity().binding_generation(),
            Generation::INITIAL,
            artifact_ref.identity().plan_set_generation(),
            artifact_ref.identity().plan_id(),
            reservation,
        );
        let prepare = PrepareInput::new(route, artifact_ref, BindingLifetimeFootprint::new(2, 128));
        assert_eq!(prepare.artifact(), artifact_ref);
        assert_eq!(prepare.route().reservation(), reservation);
        assert_eq!(prepare.route().plan_id(), artifact_ref.identity().plan_id());
    }

    #[test]
    fn missing_registration_fails_before_td_progress() {
        let td = thing();
        let registrations: [StaticBindingCompilerRegistration<MockCompiler>; 0] = [];
        let input = PlanBuildInput::new(
            &td,
            &registrations[..],
            PlanSetGeneration::new(Generation::INITIAL),
        );
        let error = compiler("level", 0)
            .start(&input)
            .expect_err("empty table must fail");
        assert!(matches!(
            error,
            CoreError::Selection {
                reason: clinkz_wot_core::SelectionFailureReason::NoSupportingBinding,
                ..
            }
        ));
    }
}
