#![allow(dead_code)]

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    mem::size_of,
    rc::Rc,
};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCandidate, BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput,
    BindingCompilerOutput, BindingCompilerStep, BindingConfigurationDigest, BindingGeneration,
    BindingId, LogicalInteractionPlan, PlanId, PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::thing::Thing;

/// Composite non-production constructibility model for workspace/0063.
///
/// Unlike the earlier phase-order fixture, this model proves one complete
/// Servient-owned transaction. The same owned logical-plan values are created
/// once after identity assignment, borrowed by compiler `bounds`, then moved
/// through resource reservation into compiler `start/step`, aggregate material,
/// reconcile, and Frozen ownership. Candidate/runtime joins, independent
/// PlanId generations, target-operation projection, persistent accounting, and
/// execution pinning are all carried by the same transaction.
mod stage_a {
    use super::*;

    const READABLE_COORDINATES: usize = 2;
    const PER_STEP_BINDING_POLLS: u64 = 1;

    fn generation(value: u32) -> Generation {
        Generation::new(value).expect("fixture generation must be nonzero")
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct PlanIdentityAssignment {
        plan_set_generation: PlanSetGeneration,
        plan_id: PlanId,
    }

    #[derive(Debug)]
    struct IdentityArenaState {
        next_plan_set_generation: PlanSetGeneration,
        next_plan_generations: Vec<Generation>,
        unpublished_outstanding: bool,
        frozen_outstanding: bool,
    }

    #[derive(Clone)]
    struct IdentityArena(Rc<RefCell<IdentityArenaState>>);

    impl IdentityArena {
        fn fixture() -> Self {
            Self(Rc::new(RefCell::new(IdentityArenaState {
                // Deliberately unrelated values: no equality invariant exists
                // between the plan-set generation and any PlanId generation.
                next_plan_set_generation: PlanSetGeneration::new(generation(3)),
                next_plan_generations: vec![generation(7), generation(11)],
                unpublished_outstanding: false,
                frozen_outstanding: false,
            })))
        }

        fn reserve(&self, count: usize) -> PlanIdentityLease {
            let mut state = self.0.borrow_mut();
            assert!(!state.unpublished_outstanding);
            assert!(!state.frozen_outstanding);
            assert!(count <= state.next_plan_generations.len());
            state.unpublished_outstanding = true;
            let plan_set_generation = state.next_plan_set_generation;
            let assignments = (0..count)
                .map(|slot| PlanIdentityAssignment {
                    plan_set_generation,
                    plan_id: PlanId::new(
                        SlotIndex::new(slot as u32),
                        state.next_plan_generations[slot],
                    ),
                })
                .collect();
            drop(state);
            PlanIdentityLease {
                arena: self.clone(),
                assignments,
                active: true,
            }
        }

        fn snapshot_generations(&self) -> (PlanSetGeneration, Vec<Generation>) {
            let state = self.0.borrow();
            (
                state.next_plan_set_generation,
                state.next_plan_generations.clone(),
            )
        }

        fn frozen_outstanding(&self) -> bool {
            self.0.borrow().frozen_outstanding
        }
    }

    #[must_use]
    struct PlanIdentityLease {
        arena: IdentityArena,
        assignments: Vec<PlanIdentityAssignment>,
        active: bool,
    }

    impl PlanIdentityLease {
        fn assignments(&self) -> &[PlanIdentityAssignment] {
            &self.assignments
        }

        fn abort(mut self) {
            self.abort_in_place();
        }

        fn abort_in_place(&mut self) {
            if !self.active {
                return;
            }
            let mut state = self.arena.0.borrow_mut();
            assert!(state.unpublished_outstanding);
            state.unpublished_outstanding = false;
            state.next_plan_set_generation = state
                .next_plan_set_generation
                .checked_next()
                .expect("fixture plan-set generation cannot wrap");
            for assignment in &self.assignments {
                let slot = assignment.plan_id.slot().get() as usize;
                state.next_plan_generations[slot] = state.next_plan_generations[slot]
                    .checked_next()
                    .expect("fixture plan generation cannot wrap");
            }
            self.active = false;
        }

        fn commit(mut self) -> FrozenIdentityOwner {
            let mut state = self.arena.0.borrow_mut();
            assert!(state.unpublished_outstanding);
            assert!(!state.frozen_outstanding);
            state.unpublished_outstanding = false;
            state.frozen_outstanding = true;
            drop(state);
            self.active = false;
            FrozenIdentityOwner {
                arena: self.arena.clone(),
                assignments: std::mem::take(&mut self.assignments),
                active: true,
            }
        }
    }

    impl Drop for PlanIdentityLease {
        fn drop(&mut self) {
            self.abort_in_place();
        }
    }

    #[must_use]
    struct FrozenIdentityOwner {
        arena: IdentityArena,
        assignments: Vec<PlanIdentityAssignment>,
        active: bool,
    }

    impl FrozenIdentityOwner {
        fn assignments(&self) -> &[PlanIdentityAssignment] {
            &self.assignments
        }

        fn reclaim(mut self) {
            self.reclaim_in_place();
        }

        fn reclaim_in_place(&mut self) {
            if !self.active {
                return;
            }
            let mut state = self.arena.0.borrow_mut();
            assert!(state.frozen_outstanding);
            state.frozen_outstanding = false;
            state.next_plan_set_generation = state
                .next_plan_set_generation
                .checked_next()
                .expect("fixture plan-set generation cannot wrap");
            for assignment in &self.assignments {
                let slot = assignment.plan_id.slot().get() as usize;
                state.next_plan_generations[slot] = state.next_plan_generations[slot]
                    .checked_next()
                    .expect("fixture plan generation cannot wrap");
            }
            self.active = false;
        }
    }

    impl Drop for FrozenIdentityOwner {
        fn drop(&mut self) {
            self.reclaim_in_place();
        }
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct MeasuredLedger {
        logical_plan_bytes: u64,
        candidate_bytes: u64,
        artifact_bytes: u64,
        artifact_ref_bytes: u64,
        binding_plan_ref_bytes: u64,
        target_index_bytes: u64,
        diagnostic_bytes: u64,
    }

    impl MeasuredLedger {
        fn persistent_bytes(self) -> u64 {
            self.logical_plan_bytes
                + self.candidate_bytes
                + self.artifact_bytes
                + self.artifact_ref_bytes
                + self.binding_plan_ref_bytes
                + self.target_index_bytes
                + self.diagnostic_bytes
        }

        fn fits_within(self, reserved: Self) -> bool {
            self.logical_plan_bytes <= reserved.logical_plan_bytes
                && self.candidate_bytes <= reserved.candidate_bytes
                && self.artifact_bytes <= reserved.artifact_bytes
                && self.artifact_ref_bytes <= reserved.artifact_ref_bytes
                && self.binding_plan_ref_bytes <= reserved.binding_plan_ref_bytes
                && self.target_index_bytes <= reserved.target_index_bytes
                && self.diagnostic_bytes <= reserved.diagnostic_bytes
        }
    }

    #[derive(Debug, Default)]
    struct ResourceAccountState {
        temporary_reserved: u64,
        persistent_reserved: u64,
        persistent_committed: u64,
    }

    #[derive(Clone)]
    struct ResourceAccount(Rc<RefCell<ResourceAccountState>>);

    impl ResourceAccount {
        fn new() -> Self {
            Self(Rc::new(RefCell::new(ResourceAccountState::default())))
        }

        fn snapshot(&self) -> ResourceAccountState {
            let state = self.0.borrow();
            ResourceAccountState {
                temporary_reserved: state.temporary_reserved,
                persistent_reserved: state.persistent_reserved,
                persistent_committed: state.persistent_committed,
            }
        }
    }

    #[must_use]
    struct ResourceLease {
        account: ResourceAccount,
        reserved: MeasuredLedger,
        temporary_bytes: u64,
        active: bool,
    }

    impl ResourceLease {
        fn reserve(account: ResourceAccount, reserved: MeasuredLedger, temporary_bytes: u64) -> Self {
            {
                let mut state = account.0.borrow_mut();
                state.temporary_reserved += temporary_bytes;
                state.persistent_reserved += reserved.persistent_bytes();
            }
            Self {
                account,
                reserved,
                temporary_bytes,
                active: true,
            }
        }

        fn reserved(&self) -> MeasuredLedger {
            self.reserved
        }

        fn abort_in_place(&mut self) {
            if !self.active {
                return;
            }
            let mut state = self.account.0.borrow_mut();
            state.temporary_reserved -= self.temporary_bytes;
            state.persistent_reserved -= self.reserved.persistent_bytes();
            self.active = false;
        }

        fn commit(mut self, measured: MeasuredLedger) -> FrozenResourceAccount {
            assert!(measured.fits_within(self.reserved));
            let committed = measured.persistent_bytes();
            {
                let mut state = self.account.0.borrow_mut();
                state.temporary_reserved -= self.temporary_bytes;
                state.persistent_reserved -= self.reserved.persistent_bytes();
                state.persistent_committed += committed;
            }
            self.active = false;
            FrozenResourceAccount {
                account: self.account.clone(),
                committed,
                active: true,
            }
        }
    }

    impl Drop for ResourceLease {
        fn drop(&mut self) {
            self.abort_in_place();
        }
    }

    #[must_use]
    struct FrozenResourceAccount {
        account: ResourceAccount,
        committed: u64,
        active: bool,
    }

    impl FrozenResourceAccount {
        fn committed(&self) -> u64 {
            self.committed
        }

        fn reclaim(mut self) {
            if self.active {
                self.account.0.borrow_mut().persistent_committed -= self.committed;
                self.active = false;
            }
        }
    }

    impl Drop for FrozenResourceAccount {
        fn drop(&mut self) {
            if self.active {
                self.account.0.borrow_mut().persistent_committed -= self.committed;
                self.active = false;
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ExecutionPin {
        binding_id: BindingId,
        binding_generation: BindingGeneration,
        configuration: BindingConfigurationDigest,
        compatibility: BindingArtifactCompatibility,
        registration_ordinal: u32,
    }

    impl ExecutionPin {
        fn from_candidate(candidate: BindingCandidate) -> Self {
            Self {
                binding_id: candidate.binding_id(),
                binding_generation: candidate.binding_generation(),
                configuration: candidate.configuration(),
                compatibility: candidate.compatibility(),
                registration_ordinal: candidate.registration_ordinal(),
            }
        }

        fn matches(self, candidate: BindingCandidate) -> bool {
            self.binding_id == candidate.binding_id()
                && self.binding_generation == candidate.binding_generation()
                && self.configuration == candidate.configuration()
                && self.compatibility == candidate.compatibility()
                && self.registration_ordinal == candidate.registration_ordinal()
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct InputFingerprint {
        plan_id: PlanId,
        property_name: String,
        form_index: u32,
        resolved_target: String,
        candidate: BindingCandidate,
    }

    impl InputFingerprint {
        fn from_input(input: &BindingCompilerInput<'_>) -> Self {
            Self {
                plan_id: input.logical_plan().plan_id(),
                property_name: input.logical_plan().property_name().to_owned(),
                form_index: input.logical_plan().form_index(),
                resolved_target: input.logical_plan().resolved_target().to_owned(),
                candidate: input.candidate(),
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FixtureCursor {
        progress: u8,
    }

    struct FixtureCompiler {
        compatibility: BindingArtifactCompatibility,
        fail_form_index: Cell<Option<u32>>,
        bounds_inputs: RefCell<Vec<InputFingerprint>>,
        start_inputs: RefCell<Vec<InputFingerprint>>,
        step_inputs: RefCell<Vec<InputFingerprint>>,
        abort_calls: Cell<u32>,
    }

    impl FixtureCompiler {
        fn new(compatibility: BindingArtifactCompatibility) -> Self {
            Self {
                compatibility,
                fail_form_index: Cell::new(None),
                bounds_inputs: RefCell::new(Vec::new()),
                start_inputs: RefCell::new(Vec::new()),
                step_inputs: RefCell::new(Vec::new()),
                abort_calls: Cell::new(0),
            }
        }
    }

    impl BindingCompilerExtension for FixtureCompiler {
        type Cursor = FixtureCursor;
        type Artifact = u8;

        fn compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
        }

        fn bounds(
            &self,
            input: &BindingCompilerInput<'_>,
        ) -> Result<BindingCompilerBounds, clinkz_wot_core::CoreError> {
            self.bounds_inputs
                .borrow_mut()
                .push(InputFingerprint::from_input(input));
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, 64),
                48,
                32,
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2),
            ))
        }

        fn start(
            &self,
            input: &BindingCompilerInput<'_>,
        ) -> Result<Self::Cursor, clinkz_wot_core::CoreError> {
            self.start_inputs
                .borrow_mut()
                .push(InputFingerprint::from_input(input));
            Ok(FixtureCursor { progress: 0 })
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
            self.step_inputs
                .borrow_mut()
                .push(InputFingerprint::from_input(input));
            cursor.progress += 1;
            if cursor.progress < 2 {
                return BindingCompilerStep::Pending(cursor);
            }
            if self.fail_form_index.get() == Some(input.logical_plan().form_index()) {
                return BindingCompilerStep::Failed(clinkz_wot_core::BindingCompilerFailure::new(
                    clinkz_wot_core::CoreError::Validation(
                        clinkz_wot_core::ErrorContext::new(
                            clinkz_wot_core::ErrorPhase::Binding,
                            clinkz_wot_core::RetryClass::Never,
                        )
                        .with_plan(input.logical_plan().plan_id()),
                    ),
                    cursor,
                ));
            }
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                self.compatibility,
                BindingArtifactFootprint::new(1, 64),
                input.logical_plan().form_index() as u8,
            )))
        }

        fn abort(&self, _cursor: Self::Cursor) {
            self.abort_calls.set(self.abort_calls.get() + 1);
        }
    }

    struct RegistrationSnapshot {
        compiler: FixtureCompiler,
        candidate: BindingCandidate,
        execution_pin: ExecutionPin,
    }

    impl RegistrationSnapshot {
        fn fixture() -> Self {
            let compatibility = BindingArtifactCompatibility::new([9; 16]);
            let candidate = BindingCandidate::new(
                BindingId::new(11),
                BindingGeneration::INITIAL,
                BindingConfigurationDigest::new([3; 32]),
                compatibility,
                4,
                0,
            );
            Self {
                compiler: FixtureCompiler::new(compatibility),
                candidate,
                execution_pin: ExecutionPin::from_candidate(candidate),
            }
        }
    }

    #[derive(Clone, Copy)]
    struct ValidatedThingRef<'td> {
        thing: &'td Thing,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TargetProjection {
        property_name: Box<str>,
        first_binding_plan_ref: u32,
        binding_plan_ref_count: u32,
    }

    #[derive(Clone, Debug)]
    struct CoordinateSeed {
        target_index: usize,
        property_name: Box<str>,
        form_index: u32,
        resolved_target: Box<str>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BindingPlanRefEquivalent {
        logical_plan_slot: u32,
        candidate_slot: u32,
        artifact_slot: u32,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct AggregateDiagnostic {
        selected_registration_ordinal: u32,
        declared_target_count: u32,
        readable_coordinate_count: u32,
    }

    struct OwnedCoordinate {
        target_index: usize,
        plan_set_generation: PlanSetGeneration,
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
    }

    struct BoundedCoordinate {
        target_index: usize,
        plan_set_generation: PlanSetGeneration,
        plan: LogicalInteractionPlan,
        candidate: BindingCandidate,
        bounds: BindingCompilerBounds,
    }

    struct AggregateDraft {
        logical_plans: Vec<LogicalInteractionPlan>,
        candidates: Vec<BindingCandidate>,
        artifacts: Vec<BindingArtifactEnvelope<u8>>,
        artifact_refs: Vec<BindingArtifactRef>,
        binding_plan_refs: Vec<BindingPlanRefEquivalent>,
        target_index: Vec<TargetProjection>,
        diagnostic: AggregateDiagnostic,
    }

    impl AggregateDraft {
        fn with_targets(target_index: Vec<TargetProjection>, diagnostic: AggregateDiagnostic) -> Self {
            Self {
                logical_plans: Vec::new(),
                candidates: Vec::new(),
                artifacts: Vec::new(),
                artifact_refs: Vec::new(),
                binding_plan_refs: Vec::new(),
                target_index,
                diagnostic,
            }
        }

        fn measured(&self) -> MeasuredLedger {
            let logical_plan_bytes = self
                .logical_plans
                .iter()
                .map(|plan| {
                    (size_of::<LogicalInteractionPlan>()
                        + plan.property_name().len()
                        + plan.resolved_target().len()
                        + plan.content_type().map_or(0, str::len)
                        + plan.subprotocol().map_or(0, str::len)) as u64
                })
                .sum();
            let target_index_bytes = self
                .target_index
                .iter()
                .map(|target| (size_of::<TargetProjection>() + target.property_name.len()) as u64)
                .sum();
            MeasuredLedger {
                logical_plan_bytes,
                candidate_bytes: (self.candidates.len() * size_of::<BindingCandidate>()) as u64,
                artifact_bytes: self
                    .artifacts
                    .iter()
                    .map(|artifact| artifact.artifact().footprint().retained_bytes())
                    .sum(),
                artifact_ref_bytes: (self.artifact_refs.len() * size_of::<BindingArtifactRef>())
                    as u64,
                binding_plan_ref_bytes: (self.binding_plan_refs.len()
                    * size_of::<BindingPlanRefEquivalent>()) as u64,
                target_index_bytes,
                diagnostic_bytes: size_of::<AggregateDiagnostic>() as u64,
            }
        }

        fn target(&self, property_name: &str) -> Option<&TargetProjection> {
            self.target_index
                .iter()
                .find(|target| target.property_name.as_ref() == property_name)
        }

        fn validate_join(&self, expected_pin: ExecutionPin) {
            assert_eq!(self.logical_plans.len(), self.candidates.len());
            assert_eq!(self.logical_plans.len(), self.artifacts.len());
            assert_eq!(self.logical_plans.len(), self.artifact_refs.len());
            assert_eq!(self.logical_plans.len(), self.binding_plan_refs.len());
            assert_eq!(
                self.diagnostic.selected_registration_ordinal,
                expected_pin.registration_ordinal
            );
            assert_eq!(
                self.diagnostic.declared_target_count as usize,
                self.target_index.len()
            );
            assert_eq!(
                self.diagnostic.readable_coordinate_count as usize,
                self.binding_plan_refs.len()
            );

            for join in &self.binding_plan_refs {
                let plan = &self.logical_plans[join.logical_plan_slot as usize];
                let candidate = self.candidates[join.candidate_slot as usize];
                let envelope = &self.artifacts[join.artifact_slot as usize];
                let artifact_ref = self.artifact_refs[join.artifact_slot as usize];
                let identity = envelope.identity();

                assert_eq!(identity.plan_id(), plan.plan_id());
                assert_eq!(identity.binding_id(), candidate.binding_id());
                assert_eq!(identity.binding_generation(), candidate.binding_generation());
                assert_eq!(identity.configuration(), candidate.configuration());
                assert_eq!(identity.compatibility(), candidate.compatibility());
                assert_eq!(identity.role(), BindingArtifactRole::ConsumerCall);
                assert_eq!(artifact_ref.identity(), identity);
                assert_eq!(
                    artifact_ref.artifact_slot(),
                    SlotIndex::new(join.artifact_slot)
                );
                assert!(expected_pin.matches(candidate));
                assert_eq!(candidate.registration_ordinal(), 4);
                assert_eq!(candidate.candidate_order(), 0);
            }

            for target in &self.target_index {
                let end = target
                    .first_binding_plan_ref
                    .checked_add(target.binding_plan_ref_count)
                    .expect("fixture target range cannot overflow");
                assert!(end as usize <= self.binding_plan_refs.len());
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct AggregateRequirements {
        reserved: MeasuredLedger,
        compiler_cursor_peak: u64,
        compiler_temporary_peak: u64,
        compiler_lifetime_polls: u64,
    }

    struct PlanSetBuildLease {
        identities: PlanIdentityLease,
        resources: ResourceLease,
        execution_pin: ExecutionPin,
        requirements: AggregateRequirements,
    }

    impl PlanSetBuildLease {
        fn assignments(&self) -> &[PlanIdentityAssignment] {
            self.identities.assignments()
        }

        fn commit(self, measured: MeasuredLedger) -> FrozenAuthority {
            let Self {
                identities,
                resources,
                execution_pin,
                requirements,
            } = self;
            assert!(measured.fits_within(requirements.reserved));
            FrozenAuthority {
                identities: identities.commit(),
                account: resources.commit(measured),
                execution_pin,
            }
        }

        fn abort(self) {
            let Self {
                identities,
                resources,
                execution_pin: _,
                requirements: _,
            } = self;
            drop(resources);
            identities.abort();
        }
    }

    struct FrozenAuthority {
        identities: FrozenIdentityOwner,
        account: FrozenResourceAccount,
        execution_pin: ExecutionPin,
    }

    impl FrozenAuthority {
        fn reclaim(self) {
            let Self {
                identities,
                account,
                execution_pin: _,
            } = self;
            account.reclaim();
            identities.reclaim();
        }
    }

    struct Captured<'td> {
        thing: &'td Thing,
        identity_arena: IdentityArena,
        resource_account: ResourceAccount,
    }

    struct Validated<'td> {
        validated: ValidatedThingRef<'td>,
        identity_arena: IdentityArena,
        resource_account: ResourceAccount,
    }

    struct Selected<'td> {
        validated: ValidatedThingRef<'td>,
        identity_arena: IdentityArena,
        resource_account: ResourceAccount,
        candidate: BindingCandidate,
        execution_pin: ExecutionPin,
    }

    struct Enumerated<'td> {
        validated: ValidatedThingRef<'td>,
        identity_arena: IdentityArena,
        resource_account: ResourceAccount,
        candidate: BindingCandidate,
        execution_pin: ExecutionPin,
        targets: Vec<TargetProjection>,
        coordinates: Vec<CoordinateSeed>,
    }

    struct Identified<'td> {
        validated: ValidatedThingRef<'td>,
        resource_account: ResourceAccount,
        identities: PlanIdentityLease,
        execution_pin: ExecutionPin,
        targets: Vec<TargetProjection>,
        coordinates: Vec<OwnedCoordinate>,
    }

    struct Bounded<'td> {
        validated: ValidatedThingRef<'td>,
        resource_account: ResourceAccount,
        identities: PlanIdentityLease,
        execution_pin: ExecutionPin,
        targets: Vec<TargetProjection>,
        coordinates: Vec<BoundedCoordinate>,
        requirements: AggregateRequirements,
    }

    struct Reserved<'td> {
        validated: ValidatedThingRef<'td>,
        targets: Vec<TargetProjection>,
        coordinates: Vec<BoundedCoordinate>,
        lease: PlanSetBuildLease,
    }

    struct ActiveCoordinate {
        coordinate: BoundedCoordinate,
        cursor: FixtureCursor,
    }

    struct AggregatePlanning<'td, 'reg> {
        _validated: ValidatedThingRef<'td>,
        snapshot: &'reg RegistrationSnapshot,
        plan_set_generation: PlanSetGeneration,
        remaining: VecDeque<BoundedCoordinate>,
        current: Option<ActiveCoordinate>,
        draft: AggregateDraft,
        compiler_lifetime_remaining: u64,
    }

    impl<'td, 'reg> AggregatePlanning<'td, 'reg> {
        fn new(
            validated: ValidatedThingRef<'td>,
            snapshot: &'reg RegistrationSnapshot,
            targets: Vec<TargetProjection>,
            coordinates: Vec<BoundedCoordinate>,
            compiler_lifetime_remaining: u64,
        ) -> Result<Self, clinkz_wot_core::CoreError> {
            let plan_set_generation = coordinates
                .first()
                .map(|coordinate| coordinate.plan_set_generation)
                .expect("fixture has readable coordinates");
            let diagnostic = AggregateDiagnostic {
                selected_registration_ordinal: snapshot.candidate.registration_ordinal(),
                declared_target_count: targets.len() as u32,
                readable_coordinate_count: coordinates.len() as u32,
            };
            let mut planning = Self {
                _validated: validated,
                snapshot,
                plan_set_generation,
                remaining: coordinates.into(),
                current: None,
                draft: AggregateDraft::with_targets(targets, diagnostic),
                compiler_lifetime_remaining,
            };
            planning.start_next()?;
            Ok(planning)
        }

        fn start_next(&mut self) -> Result<(), clinkz_wot_core::CoreError> {
            let coordinate = self
                .remaining
                .pop_front()
                .expect("fixture starts only with remaining coordinates");
            let input = BindingCompilerInput::new(
                &coordinate.plan,
                coordinate.candidate,
                BindingArtifactRole::ConsumerCall,
            );
            let cursor = self.snapshot.compiler.start(&input)?;
            self.current = Some(ActiveCoordinate { coordinate, cursor });
            Ok(())
        }

        fn provisional_count(&self) -> usize {
            self.draft.logical_plans.len()
        }

        fn step(mut self, caller: &mut WorkBudget) -> PlanningProgress<'td, 'reg> {
            let available = self
                .compiler_lifetime_remaining
                .min(caller.remaining(WorkClass::BindingPolls))
                .min(PER_STEP_BINDING_POLLS);
            if available == 0 {
                return PlanningProgress::Pending(self);
            }

            self.compiler_lifetime_remaining -= available;
            caller
                .consume(WorkClass::BindingPolls, available)
                .expect("paired debit availability was checked");
            let mut child =
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, available);

            let active = self.current.take().expect("building owns one cursor");
            let input = BindingCompilerInput::new(
                &active.coordinate.plan,
                active.coordinate.candidate,
                BindingArtifactRole::ConsumerCall,
            );
            let result = self.snapshot.compiler.step(&input, active.cursor, &mut child);

            let unused = child.remaining(WorkClass::BindingPolls);
            self.compiler_lifetime_remaining += unused;
            caller.set_remaining(
                WorkClass::BindingPolls,
                caller.remaining(WorkClass::BindingPolls) + unused,
            );

            match result {
                BindingCompilerStep::Pending(cursor) => {
                    self.current = Some(ActiveCoordinate {
                        coordinate: active.coordinate,
                        cursor,
                    });
                    PlanningProgress::Pending(self)
                }
                BindingCompilerStep::Failed(failure) => {
                    let (_error, cursor) = failure.into_parts();
                    self.current = Some(ActiveCoordinate {
                        coordinate: active.coordinate,
                        cursor,
                    });
                    PlanningProgress::Failed(self)
                }
                BindingCompilerStep::Complete(output) => {
                    let coordinate = active.coordinate;
                    let artifact_slot = self.draft.artifacts.len() as u32;
                    let plan_slot = self.draft.logical_plans.len() as u32;
                    let candidate_slot = self.draft.candidates.len() as u32;
                    let identity = BindingArtifactIdentity::new(
                        coordinate.plan_set_generation,
                        coordinate.plan.plan_id(),
                        coordinate.candidate.binding_id(),
                        coordinate.candidate.binding_generation(),
                        coordinate.candidate.configuration(),
                        coordinate.candidate.compatibility(),
                        BindingArtifactRole::ConsumerCall,
                    );
                    let envelope = BindingArtifactEnvelope::try_new(
                        identity,
                        coordinate.bounds.artifact(),
                        output.into_artifact(),
                    )
                    .expect("fixture artifact fits admitted bounds");
                    let artifact_ref = BindingArtifactRef::new(identity, SlotIndex::new(artifact_slot));
                    self.draft.logical_plans.push(coordinate.plan);
                    self.draft.candidates.push(coordinate.candidate);
                    self.draft.artifacts.push(envelope);
                    self.draft.artifact_refs.push(artifact_ref);
                    self.draft.binding_plan_refs.push(BindingPlanRefEquivalent {
                        logical_plan_slot: plan_slot,
                        candidate_slot,
                        artifact_slot,
                    });

                    if self.remaining.is_empty() {
                        PlanningProgress::Complete(self.draft)
                    } else {
                        self.start_next()
                            .expect("fixture next start uses retained owned plan");
                        PlanningProgress::Pending(self)
                    }
                }
            }
        }

        fn abort(mut self) -> AggregateDraft {
            if let Some(active) = self.current.take() {
                self.snapshot.compiler.abort(active.cursor);
            }
            self.draft
        }
    }

    enum PlanningProgress<'td, 'reg> {
        Pending(AggregatePlanning<'td, 'reg>),
        Complete(AggregateDraft),
        Failed(AggregatePlanning<'td, 'reg>),
    }

    struct Building<'td, 'reg> {
        lease: PlanSetBuildLease,
        planning: AggregatePlanning<'td, 'reg>,
    }

    struct Reconciling {
        lease: PlanSetBuildLease,
        draft: AggregateDraft,
    }

    struct Frozen {
        authority: FrozenAuthority,
        draft: AggregateDraft,
    }

    struct FailedSettled;

    struct ConsumerAdmissionTxn<'td, 'reg, S> {
        snapshot: &'reg RegistrationSnapshot,
        state: S,
        _source_lifetime: std::marker::PhantomData<&'td Thing>,
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Captured<'td>> {
        fn capture(
            thing: &'td Thing,
            snapshot: &'reg RegistrationSnapshot,
            identity_arena: IdentityArena,
            resource_account: ResourceAccount,
        ) -> Self {
            Self {
                snapshot,
                state: Captured {
                    thing,
                    identity_arena,
                    resource_account,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }

        fn validate(self) -> ConsumerAdmissionTxn<'td, 'reg, Validated<'td>> {
            let Captured {
                thing,
                identity_arena,
                resource_account,
            } = self.state;
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Validated {
                    validated: ValidatedThingRef { thing },
                    identity_arena,
                    resource_account,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Validated<'td>> {
        fn select_registration(self) -> ConsumerAdmissionTxn<'td, 'reg, Selected<'td>> {
            let Validated {
                validated,
                identity_arena,
                resource_account,
            } = self.state;
            assert!(self.snapshot.execution_pin.matches(self.snapshot.candidate));
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Selected {
                    validated,
                    identity_arena,
                    resource_account,
                    candidate: self.snapshot.candidate,
                    execution_pin: self.snapshot.execution_pin,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Selected<'td>> {
        fn enumerate(self) -> ConsumerAdmissionTxn<'td, 'reg, Enumerated<'td>> {
            let Selected {
                validated,
                identity_arena,
                resource_account,
                candidate,
                execution_pin,
            } = self.state;

            // Deterministic target projection includes one declared property
            // with no readable Form. Its zero-length range survives source TD
            // release and remains distinguishable from an absent property.
            let targets = vec![
                TargetProjection {
                    property_name: Box::from("humidity"),
                    first_binding_plan_ref: 0,
                    binding_plan_ref_count: 0,
                },
                TargetProjection {
                    property_name: Box::from("pressure"),
                    first_binding_plan_ref: 0,
                    binding_plan_ref_count: 1,
                },
                TargetProjection {
                    property_name: Box::from("temperature"),
                    first_binding_plan_ref: 1,
                    binding_plan_ref_count: 1,
                },
            ];
            let coordinates = vec![
                CoordinateSeed {
                    target_index: 1,
                    property_name: Box::from("pressure"),
                    form_index: 0,
                    resolved_target: Box::from("mock://thing/pressure"),
                },
                CoordinateSeed {
                    target_index: 2,
                    property_name: Box::from("temperature"),
                    form_index: 1,
                    resolved_target: Box::from("mock://thing/temperature"),
                },
            ];

            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Enumerated {
                    validated,
                    identity_arena,
                    resource_account,
                    candidate,
                    execution_pin,
                    targets,
                    coordinates,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Enumerated<'td>> {
        fn assign_identities(self) -> ConsumerAdmissionTxn<'td, 'reg, Identified<'td>> {
            let Enumerated {
                validated,
                identity_arena,
                resource_account,
                candidate,
                execution_pin,
                targets,
                coordinates,
            } = self.state;
            let identities = identity_arena.reserve(coordinates.len());
            let owned_coordinates = coordinates
                .into_iter()
                .zip(identities.assignments().iter().copied())
                .map(|(seed, assignment)| OwnedCoordinate {
                    target_index: seed.target_index,
                    plan_set_generation: assignment.plan_set_generation,
                    plan: LogicalInteractionPlan::try_property_read(
                        assignment.plan_id,
                        ThingId::from("urn:test:aggregate-consumer"),
                        seed.property_name,
                        seed.form_index,
                        seed.resolved_target,
                        Some(Box::from("application/json")),
                        None,
                    )
                    .expect("fixture plan is valid"),
                    candidate,
                })
                .collect();
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Identified {
                    validated,
                    resource_account,
                    identities,
                    execution_pin,
                    targets,
                    coordinates: owned_coordinates,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Identified<'td>> {
        fn collect_bounds(self) -> ConsumerAdmissionTxn<'td, 'reg, Bounded<'td>> {
            let Identified {
                validated,
                resource_account,
                identities,
                execution_pin,
                targets,
                coordinates,
            } = self.state;
            let mut bounded = Vec::with_capacity(coordinates.len());
            let mut reserved = MeasuredLedger {
                candidate_bytes: (coordinates.len() * size_of::<BindingCandidate>()) as u64,
                artifact_ref_bytes: (coordinates.len() * size_of::<BindingArtifactRef>()) as u64,
                binding_plan_ref_bytes: (coordinates.len() * size_of::<BindingPlanRefEquivalent>())
                    as u64,
                target_index_bytes: targets
                    .iter()
                    .map(|target| (size_of::<TargetProjection>() + target.property_name.len()) as u64)
                    .sum(),
                diagnostic_bytes: size_of::<AggregateDiagnostic>() as u64,
                ..MeasuredLedger::default()
            };
            let mut cursor_peak = 0;
            let mut temporary_peak = 0;
            let mut lifetime_polls = 0;

            for coordinate in coordinates {
                let input = BindingCompilerInput::new(
                    &coordinate.plan,
                    coordinate.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                let bounds = self
                    .snapshot
                    .compiler
                    .bounds(&input)
                    .expect("fixture bounds succeed");
                reserved.logical_plan_bytes += (size_of::<LogicalInteractionPlan>()
                    + coordinate.plan.property_name().len()
                    + coordinate.plan.resolved_target().len()
                    + coordinate.plan.content_type().map_or(0, str::len)
                    + coordinate.plan.subprotocol().map_or(0, str::len)) as u64;
                reserved.artifact_bytes += bounds.artifact().retained_bytes();
                cursor_peak = cursor_peak.max(bounds.cursor_bytes());
                temporary_peak = temporary_peak.max(bounds.temporary_bytes());
                lifetime_polls += bounds.work().remaining(WorkClass::BindingPolls);
                bounded.push(BoundedCoordinate {
                    target_index: coordinate.target_index,
                    plan_set_generation: coordinate.plan_set_generation,
                    plan: coordinate.plan,
                    candidate: coordinate.candidate,
                    bounds,
                });
            }

            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Bounded {
                    validated,
                    resource_account,
                    identities,
                    execution_pin,
                    targets,
                    coordinates: bounded,
                    requirements: AggregateRequirements {
                        reserved,
                        compiler_cursor_peak: cursor_peak,
                        compiler_temporary_peak: temporary_peak,
                        compiler_lifetime_polls: lifetime_polls,
                    },
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Bounded<'td>> {
        fn reserve_resources(self) -> ConsumerAdmissionTxn<'td, 'reg, Reserved<'td>> {
            let Bounded {
                validated,
                resource_account,
                identities,
                execution_pin,
                targets,
                coordinates,
                requirements,
            } = self.state;
            let temporary = requirements
                .compiler_cursor_peak
                .checked_add(requirements.compiler_temporary_peak)
                .expect("fixture temporary bound cannot overflow");
            let resources =
                ResourceLease::reserve(resource_account, requirements.reserved, temporary);
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Reserved {
                    validated,
                    targets,
                    coordinates,
                    lease: PlanSetBuildLease {
                        identities,
                        resources,
                        execution_pin,
                        requirements,
                    },
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Reserved<'td>> {
        fn start_build(
            self,
        ) -> Result<ConsumerAdmissionTxn<'td, 'reg, Building<'td, 'reg>>, clinkz_wot_core::CoreError>
        {
            let Reserved {
                validated,
                targets,
                coordinates,
                lease,
            } = self.state;
            let planning = AggregatePlanning::new(
                validated,
                self.snapshot,
                targets,
                coordinates,
                lease.requirements.compiler_lifetime_polls,
            )?;
            Ok(ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Building { lease, planning },
                _source_lifetime: std::marker::PhantomData,
            })
        }
    }

    enum BuildProgress<'td, 'reg> {
        Pending(ConsumerAdmissionTxn<'td, 'reg, Building<'td, 'reg>>),
        Reconciling(ConsumerAdmissionTxn<'td, 'reg, Reconciling>),
        Failed(ConsumerAdmissionTxn<'td, 'reg, Building<'td, 'reg>>),
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Building<'td, 'reg>> {
        fn step(self, caller: &mut WorkBudget) -> BuildProgress<'td, 'reg> {
            let Building { lease, planning } = self.state;
            match planning.step(caller) {
                PlanningProgress::Pending(planning) => BuildProgress::Pending(ConsumerAdmissionTxn {
                    snapshot: self.snapshot,
                    state: Building { lease, planning },
                    _source_lifetime: std::marker::PhantomData,
                }),
                PlanningProgress::Complete(draft) => {
                    BuildProgress::Reconciling(ConsumerAdmissionTxn {
                        snapshot: self.snapshot,
                        state: Reconciling { lease, draft },
                        _source_lifetime: std::marker::PhantomData,
                    })
                }
                PlanningProgress::Failed(planning) => BuildProgress::Failed(ConsumerAdmissionTxn {
                    snapshot: self.snapshot,
                    state: Building { lease, planning },
                    _source_lifetime: std::marker::PhantomData,
                }),
            }
        }

        fn provisional_count(&self) -> usize {
            self.state.planning.provisional_count()
        }

        fn abort(self) -> ConsumerAdmissionTxn<'td, 'reg, FailedSettled> {
            let Building { lease, planning } = self.state;
            let _provisional_material = planning.abort();
            lease.abort();
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: FailedSettled,
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Reconciling> {
        fn freeze(self) -> ConsumerAdmissionTxn<'td, 'reg, Frozen> {
            let Reconciling { lease, draft } = self.state;
            draft.validate_join(lease.execution_pin);
            let measured = draft.measured();
            assert!(measured.fits_within(lease.resources.reserved()));
            assert_eq!(draft.logical_plans.len(), READABLE_COORDINATES);
            assert_eq!(draft.candidates.len(), READABLE_COORDINATES);
            assert_eq!(draft.artifacts.len(), READABLE_COORDINATES);
            assert_eq!(draft.binding_plan_refs.len(), READABLE_COORDINATES);
            assert_eq!(draft.target("humidity").unwrap().binding_plan_ref_count, 0);
            assert!(draft.target("absent-property").is_none());
            let authority = lease.commit(measured);
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Frozen { authority, draft },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    fn fixture() -> (
        Thing,
        RegistrationSnapshot,
        IdentityArena,
        ResourceAccount,
    ) {
        (
            Thing::default(),
            RegistrationSnapshot::fixture(),
            IdentityArena::fixture(),
            ResourceAccount::new(),
        )
    }

    fn advance_to_building<'td, 'reg>(
        thing: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        identities: IdentityArena,
        resources: ResourceAccount,
    ) -> ConsumerAdmissionTxn<'td, 'reg, Building<'td, 'reg>> {
        ConsumerAdmissionTxn::capture(thing, snapshot, identities, resources)
            .validate()
            .select_registration()
            .enumerate()
            .assign_identities()
            .collect_bounds()
            .reserve_resources()
            .start_build()
            .expect("fixture build starts")
    }

    #[test]
    fn one_owned_plan_value_flows_from_bounds_through_frozen_runtime_join() {
        let (thing, snapshot, identities, resources) = fixture();
        let mut txn = advance_to_building(
            &thing,
            &snapshot,
            identities.clone(),
            resources.clone(),
        );
        let mut caller =
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 16);

        let frozen = loop {
            txn = match txn.step(&mut caller) {
                BuildProgress::Pending(txn) => txn,
                BuildProgress::Reconciling(txn) => break txn.freeze(),
                BuildProgress::Failed(_) => panic!("success fixture must not fail"),
            };
        };

        let bounds = snapshot.compiler.bounds_inputs.borrow();
        let starts = snapshot.compiler.start_inputs.borrow();
        let steps = snapshot.compiler.step_inputs.borrow();
        assert_eq!(bounds.len(), READABLE_COORDINATES);
        assert_eq!(starts.len(), READABLE_COORDINATES);
        for input in bounds.iter() {
            assert!(starts.contains(input));
            assert!(steps.contains(input));
        }
        drop(bounds);
        drop(starts);
        drop(steps);

        // PlanId generations are deliberately independent from the aggregate
        // PlanSetGeneration while remaining exact across bounds/build/Frozen.
        let assignments = frozen.state.authority.identities.assignments();
        assert_eq!(assignments[0].plan_set_generation.get().get(), 3);
        assert_eq!(assignments[0].plan_id.generation().get(), 7);
        assert_eq!(assignments[1].plan_id.generation().get(), 11);
        assert_ne!(
            assignments[0].plan_set_generation.get(),
            assignments[0].plan_id.generation()
        );

        assert!(frozen
            .state
            .authority
            .execution_pin
            .matches(frozen.state.draft.candidates[0]));
        assert!(identities.frozen_outstanding());
        let account = resources.snapshot();
        assert_eq!(account.temporary_reserved, 0);
        assert_eq!(account.persistent_reserved, 0);
        assert!(account.persistent_committed > 0);
        assert_eq!(
            account.persistent_committed,
            frozen.state.authority.account.committed()
        );

        frozen.state.authority.reclaim();
        assert!(!identities.frozen_outstanding());
        assert_eq!(resources.snapshot().persistent_committed, 0);
    }

    #[test]
    fn partial_success_abort_releases_material_and_advances_both_generation_domains() {
        let (thing, snapshot, identities, resources) = fixture();
        snapshot.compiler.fail_form_index.set(Some(1));
        let before = identities.snapshot_generations();
        let mut txn = advance_to_building(
            &thing,
            &snapshot,
            identities.clone(),
            resources.clone(),
        );
        let mut caller =
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 16);

        let failed = loop {
            txn = match txn.step(&mut caller) {
                BuildProgress::Pending(txn) => txn,
                BuildProgress::Reconciling(_) => panic!("failure fixture must not freeze"),
                BuildProgress::Failed(txn) => break txn,
            };
        };

        assert_eq!(failed.provisional_count(), 1);
        let settled = failed.abort();
        let _ = settled;
        assert_eq!(snapshot.compiler.abort_calls.get(), 1);
        let account = resources.snapshot();
        assert_eq!(account.temporary_reserved, 0);
        assert_eq!(account.persistent_reserved, 0);
        assert_eq!(account.persistent_committed, 0);

        let after = identities.snapshot_generations();
        assert_eq!(
            after.0,
            before.0.checked_next().expect("plan-set generation advances")
        );
        assert_eq!(
            after.1[0],
            before.1[0].checked_next().expect("plan generation advances")
        );
        assert_eq!(
            after.1[1],
            before.1[1].checked_next().expect("plan generation advances")
        );

        // Reuse of the same dense plan slots receives the advanced per-slot
        // PlanId generations, independently of the advanced PlanSetGeneration.
        let reused = identities.reserve(READABLE_COORDINATES);
        assert_eq!(reused.assignments()[0].plan_set_generation, after.0);
        assert_eq!(reused.assignments()[0].plan_id.generation(), after.1[0]);
        assert_eq!(reused.assignments()[1].plan_id.generation(), after.1[1]);
        reused.abort();
    }
}
