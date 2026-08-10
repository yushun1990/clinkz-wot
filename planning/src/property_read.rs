//! First bounded Property Read plan-build algorithm.

use alloc::{boxed::Box, vec};
use core::convert::TryFrom;

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCandidate, BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput,
    BindingCompilerStep, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingRegistrationIdentity, CoreError, CoreResult, ErrorContext, ErrorPhase,
    LogicalInteractionPlan, PlanId, RetryClass, StaticBindingCompilerRegistration, ThingId,
};
#[cfg(feature = "std")]
use clinkz_wot_core::{
    HostBindingArtifact, HostBindingCompilerCursor, HostBindingCompilerRegistration,
};
use clinkz_wot_foundation::{SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::{
    data_type::{Operation, resolve_form_href},
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
#[derive(Debug, Eq, PartialEq)]
pub struct PropertyReadBuildCursor<C, A> {
    state: PropertyReadBuildState<C, A>,
}

/// Exact bounded compiler for the reviewed Property Read projection.
pub struct PropertyReadPlanCompiler {
    plan_id: PlanId,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    configuration: BindingConfigurationDigest,
    compatibility: BindingArtifactCompatibility,
    registration_index: u32,
    candidate_order: u32,
    role: BindingArtifactRole,
}

impl PropertyReadPlanCompiler {
    const fn new(
        plan_id: PlanId,
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        registration_index: u32,
        candidate_order: u32,
    ) -> Self {
        Self {
            plan_id,
            binding_id,
            binding_generation,
            configuration,
            compatibility,
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
                let plan = match property_read_plan(input.validated_td(), self.plan_id) {
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

fn property_read_plan(td: &Thing, plan_id: PlanId) -> CoreResult<LogicalInteractionPlan> {
    let thing_id = td
        .id
        .as_ref()
        .map(|id| ThingId::from(id.as_str()))
        .ok_or_else(document_error)?;
    let properties = td.properties.as_ref().ok_or_else(|| {
        selection_error(clinkz_wot_core::SelectionFailureReason::AffordanceMissing)
    })?;

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
            let resolved = resolve_form_href(td.base.as_ref(), &form.href).map_err(|_| {
                selection_error(clinkz_wot_core::SelectionFailureReason::TargetResolutionFailed)
            })?;
            return LogicalInteractionPlan::try_property_read(
                plan_id,
                thing_id,
                Box::from(property_name.as_str()),
                form_index,
                Box::from(resolved.as_str()),
                Some(Box::from(form.content_type.as_str())),
                form.subprotocol.as_deref().map(Box::from),
            );
        }
    }

    Err(selection_error(
        clinkz_wot_core::SelectionFailureReason::NoFormSupportsOperation,
    ))
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
    use clinkz_wot_core::{BindingCompilerOutput, PlanSetGeneration};
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
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                self.compatibility,
                footprint,
                MockArtifact { target },
            )))
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

    fn plan_id() -> PlanId {
        PlanId::new(SlotIndex::new(3), Generation::INITIAL)
    }

    fn compiler<R>() -> PropertyReadPlanCompiler {
        PropertyReadPlanCompiler::new(
            plan_id(),
            BindingId::new(11),
            BindingGeneration::new(Generation::INITIAL),
            BindingConfigurationDigest::new([12; 32]),
            BindingArtifactCompatibility::new([13; 16]),
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
        let compiler = compiler::<StaticBindingCompilerRegistration<MockCompiler>>();
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
            let compiler = compiler::<StaticBindingCompilerRegistration<MockCompiler>>();
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
            let compiler = compiler::<HostBindingCompilerRegistration>();
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
        let error = compiler::<StaticBindingCompilerRegistration<MockCompiler>>()
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
