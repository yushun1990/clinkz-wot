#![allow(dead_code)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint, BindingArtifactRole,
    BindingCandidate, BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput,
    BindingCompilerOutput, BindingCompilerStep, BindingConfigurationDigest, BindingGeneration,
    BindingId, LogicalInteractionPlan, PlanId, PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::thing::Thing;

/// Non-production Stage-A constructibility model for workspace/0063.
///
/// This fixture intentionally does not define the future public API. It proves
/// only the disputed topology against current Core compiler contracts:
///
/// - one Servient-owned aggregate transaction retains plan-set authority;
/// - exact unpublished PlanIds are assigned before current compiler `bounds`;
/// - resource reservation happens after all bounds and before compiler `start`;
/// - Planning receives copied, non-authoritative identity assignments only;
/// - Pending resumes without replacement source/snapshot/identity inputs;
/// - a live compiler cursor is aborted through the real SPI exactly once; and
/// - identity/resource reservations settle on abort or successful freeze.
mod stage_a {
    use super::*;

    const COORDINATE_COUNT: usize = 2;
    const PER_STEP_BINDING_POLLS: u64 = 1;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct PlanIdentityAssignment {
        plan_set_generation: PlanSetGeneration,
        plan_id: PlanId,
    }

    #[derive(Clone)]
    struct ReservationTracker {
        identity_outstanding: Rc<Cell<u32>>,
        resource_outstanding: Rc<Cell<u32>>,
    }

    impl ReservationTracker {
        fn new() -> Self {
            Self {
                identity_outstanding: Rc::new(Cell::new(0)),
                resource_outstanding: Rc::new(Cell::new(0)),
            }
        }

        fn identities(&self) -> u32 {
            self.identity_outstanding.get()
        }

        fn resources(&self) -> u32 {
            self.resource_outstanding.get()
        }
    }

    struct ServientPlanIdentityAuthority {
        generation: PlanSetGeneration,
        next_slot: u32,
        tracker: ReservationTracker,
    }

    impl ServientPlanIdentityAuthority {
        fn new(generation: PlanSetGeneration, tracker: ReservationTracker) -> Self {
            Self {
                generation,
                next_slot: 0,
                tracker,
            }
        }

        fn assign(mut self, count: usize) -> PlanIdentityLease {
            let mut assignments = Vec::with_capacity(count);
            for _ in 0..count {
                assignments.push(PlanIdentityAssignment {
                    plan_set_generation: self.generation,
                    plan_id: PlanId::new(SlotIndex::new(self.next_slot), Generation::INITIAL),
                });
                self.next_slot += 1;
            }
            self.tracker
                .identity_outstanding
                .set(self.tracker.identity_outstanding.get() + 1);
            PlanIdentityLease {
                assignments,
                tracker: self.tracker,
                active: true,
            }
        }
    }

    #[must_use]
    struct PlanIdentityLease {
        assignments: Vec<PlanIdentityAssignment>,
        tracker: ReservationTracker,
        active: bool,
    }

    impl PlanIdentityLease {
        fn assignments(&self) -> &[PlanIdentityAssignment] {
            &self.assignments
        }

        fn settle(&mut self) {
            if self.active {
                self.tracker.identity_outstanding.set(
                    self.tracker
                        .identity_outstanding
                        .get()
                        .checked_sub(1)
                        .expect("fixture identity lease must be outstanding"),
                );
                self.active = false;
            }
        }

        fn commit(mut self) -> Vec<PlanIdentityAssignment> {
            self.settle();
            std::mem::take(&mut self.assignments)
        }
    }

    impl Drop for PlanIdentityLease {
        fn drop(&mut self) {
            self.settle();
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct AggregateRequirements {
        plan_count: usize,
        artifact_bytes: u64,
        compiler_cursor_peak: u64,
        compiler_temporary_peak: u64,
        compiler_lifetime_polls: u64,
    }

    #[must_use]
    struct ResourceLease {
        tracker: ReservationTracker,
        active: bool,
    }

    impl ResourceLease {
        fn reserve(tracker: ReservationTracker) -> Self {
            tracker
                .resource_outstanding
                .set(tracker.resource_outstanding.get() + 1);
            Self {
                tracker,
                active: true,
            }
        }

        fn settle(&mut self) {
            if self.active {
                self.tracker.resource_outstanding.set(
                    self.tracker
                        .resource_outstanding
                        .get()
                        .checked_sub(1)
                        .expect("fixture resource lease must be outstanding"),
                );
                self.active = false;
            }
        }

        fn commit(mut self) {
            self.settle();
        }
    }

    impl Drop for ResourceLease {
        fn drop(&mut self) {
            self.settle();
        }
    }

    #[must_use]
    struct PlanSetBuildLease {
        identities: PlanIdentityLease,
        resources: ResourceLease,
        requirements: AggregateRequirements,
    }

    impl PlanSetBuildLease {
        fn new(identities: PlanIdentityLease, requirements: AggregateRequirements) -> Self {
            let resources = ResourceLease::reserve(identities.tracker.clone());
            Self {
                identities,
                resources,
                requirements,
            }
        }

        fn assignments(&self) -> &[PlanIdentityAssignment] {
            self.identities.assignments()
        }

        fn requirements(&self) -> AggregateRequirements {
            self.requirements
        }

        fn commit(self) -> FrozenAuthority {
            let PlanSetBuildLease {
                identities,
                resources,
                requirements,
            } = self;
            resources.commit();
            let assignments = identities.commit();
            FrozenAuthority {
                assignments,
                requirements,
            }
        }
    }

    struct FrozenAuthority {
        assignments: Vec<PlanIdentityAssignment>,
        requirements: AggregateRequirements,
    }

    #[derive(Clone, Copy)]
    struct ValidatedThingRef<'td> {
        thing: &'td Thing,
        census_items: u64,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FixtureCursor {
        progress: u8,
    }

    struct FixtureCompiler {
        compatibility: BindingArtifactCompatibility,
        bounds_calls: Cell<u32>,
        start_calls: Cell<u32>,
        step_calls: Cell<u32>,
        abort_calls: Cell<u32>,
        bound_plan_ids: RefCell<Vec<PlanId>>,
        started_plan_ids: RefCell<Vec<PlanId>>,
    }

    impl FixtureCompiler {
        fn new(compatibility: BindingArtifactCompatibility) -> Self {
            Self {
                compatibility,
                bounds_calls: Cell::new(0),
                start_calls: Cell::new(0),
                step_calls: Cell::new(0),
                abort_calls: Cell::new(0),
                bound_plan_ids: RefCell::new(Vec::new()),
                started_plan_ids: RefCell::new(Vec::new()),
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
            self.bounds_calls.set(self.bounds_calls.get() + 1);
            self.bound_plan_ids
                .borrow_mut()
                .push(input.logical_plan().plan_id());
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
            self.start_calls.set(self.start_calls.get() + 1);
            self.started_plan_ids
                .borrow_mut()
                .push(input.logical_plan().plan_id());
            Ok(FixtureCursor { progress: 0 })
        }

        fn step(
            &self,
            _input: &BindingCompilerInput<'_>,
            mut cursor: Self::Cursor,
            budget: &mut WorkBudget,
        ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return BindingCompilerStep::Pending(cursor);
            }
            self.step_calls.set(self.step_calls.get() + 1);
            cursor.progress += 1;
            if cursor.progress < 2 {
                BindingCompilerStep::Pending(cursor)
            } else {
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    self.compatibility,
                    BindingArtifactFootprint::new(1, 64),
                    7,
                )))
            }
        }

        fn abort(&self, _cursor: Self::Cursor) {
            self.abort_calls.set(self.abort_calls.get() + 1);
        }
    }

    struct RegistrationSnapshot {
        compiler: FixtureCompiler,
        candidate: BindingCandidate,
    }

    impl RegistrationSnapshot {
        fn fixture() -> Self {
            let compatibility = BindingArtifactCompatibility::new([9; 16]);
            Self {
                compiler: FixtureCompiler::new(compatibility),
                candidate: BindingCandidate::new(
                    BindingId::new(11),
                    BindingGeneration::INITIAL,
                    BindingConfigurationDigest::new([3; 32]),
                    compatibility,
                    4,
                    0,
                ),
            }
        }
    }

    struct Captured<'td> {
        thing: &'td Thing,
        identity_authority: ServientPlanIdentityAuthority,
    }

    struct Validated<'td> {
        validated: ValidatedThingRef<'td>,
        identity_authority: ServientPlanIdentityAuthority,
    }

    struct Enumerated<'td> {
        validated: ValidatedThingRef<'td>,
        identity_authority: ServientPlanIdentityAuthority,
        coordinate_count: usize,
    }

    struct Identified<'td> {
        validated: ValidatedThingRef<'td>,
        identities: PlanIdentityLease,
    }

    struct Bounded<'td> {
        validated: ValidatedThingRef<'td>,
        identities: PlanIdentityLease,
        requirements: AggregateRequirements,
    }

    struct Reserved<'td> {
        validated: ValidatedThingRef<'td>,
        lease: PlanSetBuildLease,
    }

    struct CoordinateBuild {
        plan: LogicalInteractionPlan,
        cursor: FixtureCursor,
    }

    struct AggregatePlanning<'td, 'reg> {
        validated: ValidatedThingRef<'td>,
        snapshot: &'reg RegistrationSnapshot,
        assignments: Vec<PlanIdentityAssignment>,
        coordinate_index: usize,
        current: Option<CoordinateBuild>,
        artifact_count: usize,
        compiler_lifetime_remaining: u64,
    }

    impl<'td, 'reg> AggregatePlanning<'td, 'reg> {
        fn new(
            validated: ValidatedThingRef<'td>,
            snapshot: &'reg RegistrationSnapshot,
            assignments: Vec<PlanIdentityAssignment>,
            compiler_lifetime_remaining: u64,
        ) -> Result<Self, clinkz_wot_core::CoreError> {
            let mut planning = Self {
                validated,
                snapshot,
                assignments,
                coordinate_index: 0,
                current: None,
                artifact_count: 0,
                compiler_lifetime_remaining,
            };
            planning.start_current()?;
            Ok(planning)
        }

        fn plan_for(&self, index: usize) -> LogicalInteractionPlan {
            let assignment = self.assignments[index];
            LogicalInteractionPlan::try_property_read(
                assignment.plan_id,
                ThingId::from("urn:test:aggregate-consumer"),
                format!("property-{index}").into_boxed_str(),
                index as u32,
                format!("mock://thing/property-{index}").into_boxed_str(),
                Some(Box::from("application/json")),
                None,
            )
            .expect("fixture logical plan is valid")
        }

        fn start_current(&mut self) -> Result<(), clinkz_wot_core::CoreError> {
            let plan = self.plan_for(self.coordinate_index);
            let input = BindingCompilerInput::new(
                &plan,
                self.snapshot.candidate,
                BindingArtifactRole::ConsumerCall,
            );
            let cursor = self.snapshot.compiler.start(&input)?;
            self.current = Some(CoordinateBuild { plan, cursor });
            Ok(())
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
                .expect("availability was checked before paired debit");

            let mut child =
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, available);
            let current = self.current.take().expect("building owns one compiler cursor");
            let input = BindingCompilerInput::new(
                &current.plan,
                self.snapshot.candidate,
                BindingArtifactRole::ConsumerCall,
            );
            let result = self.snapshot.compiler.step(&input, current.cursor, &mut child);

            let unused = child.remaining(WorkClass::BindingPolls);
            self.compiler_lifetime_remaining = self
                .compiler_lifetime_remaining
                .checked_add(unused)
                .expect("fixture lifetime work cannot overflow");
            caller.set_remaining(
                WorkClass::BindingPolls,
                caller
                    .remaining(WorkClass::BindingPolls)
                    .checked_add(unused)
                    .expect("fixture caller work cannot overflow"),
            );

            match result {
                BindingCompilerStep::Pending(cursor) => {
                    self.current = Some(CoordinateBuild {
                        plan: current.plan,
                        cursor,
                    });
                    PlanningProgress::Pending(self)
                }
                BindingCompilerStep::Complete(output) => {
                    let artifact = output.into_artifact();
                    assert_eq!(artifact.footprint().retained_bytes(), 64);
                    self.artifact_count += 1;
                    self.coordinate_index += 1;
                    if self.coordinate_index == self.assignments.len() {
                        PlanningProgress::Complete(AggregateDraft {
                            assignments: self.assignments,
                            artifact_count: self.artifact_count,
                        })
                    } else {
                        self.start_current()
                            .expect("fixture next compiler start remains valid");
                        PlanningProgress::Pending(self)
                    }
                }
                BindingCompilerStep::Failed(failure) => {
                    let (_error, cursor) = failure.into_parts();
                    self.current = Some(CoordinateBuild {
                        plan: current.plan,
                        cursor,
                    });
                    PlanningProgress::Failed(self)
                }
            }
        }

        fn abort(mut self) {
            if let Some(current) = self.current.take() {
                self.snapshot.compiler.abort(current.cursor);
            }
        }
    }

    enum PlanningProgress<'td, 'reg> {
        Pending(AggregatePlanning<'td, 'reg>),
        Complete(AggregateDraft),
        Failed(AggregatePlanning<'td, 'reg>),
    }

    struct AggregateDraft {
        assignments: Vec<PlanIdentityAssignment>,
        artifact_count: usize,
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
            identity_authority: ServientPlanIdentityAuthority,
        ) -> Self {
            Self {
                snapshot,
                state: Captured {
                    thing,
                    identity_authority,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }

        fn validate(self) -> ConsumerAdmissionTxn<'td, 'reg, Validated<'td>> {
            let Captured {
                thing,
                identity_authority,
            } = self.state;
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Validated {
                    validated: ValidatedThingRef {
                        thing,
                        census_items: 1,
                    },
                    identity_authority,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Validated<'td>> {
        fn enumerate(self) -> ConsumerAdmissionTxn<'td, 'reg, Enumerated<'td>> {
            let Validated {
                validated,
                identity_authority,
            } = self.state;
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Enumerated {
                    validated,
                    identity_authority,
                    coordinate_count: COORDINATE_COUNT,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Enumerated<'td>> {
        fn assign_identities(self) -> ConsumerAdmissionTxn<'td, 'reg, Identified<'td>> {
            let Enumerated {
                validated,
                identity_authority,
                coordinate_count,
            } = self.state;
            let identities = identity_authority.assign(coordinate_count);
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Identified {
                    validated,
                    identities,
                },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'td, 'reg> ConsumerAdmissionTxn<'td, 'reg, Identified<'td>> {
        fn collect_bounds(self) -> ConsumerAdmissionTxn<'td, 'reg, Bounded<'td>> {
            let Identified {
                validated,
                identities,
            } = self.state;

            let mut artifact_bytes = 0;
            let mut cursor_peak = 0;
            let mut temporary_peak = 0;
            let mut compiler_lifetime_polls = 0;

            for (index, assignment) in identities.assignments().iter().copied().enumerate() {
                let plan = LogicalInteractionPlan::try_property_read(
                    assignment.plan_id,
                    ThingId::from("urn:test:aggregate-consumer"),
                    format!("property-{index}").into_boxed_str(),
                    index as u32,
                    format!("mock://thing/property-{index}").into_boxed_str(),
                    Some(Box::from("application/json")),
                    None,
                )
                .expect("fixture bounds plan is valid");
                let input = BindingCompilerInput::new(
                    &plan,
                    self.snapshot.candidate,
                    BindingArtifactRole::ConsumerCall,
                );
                let bounds = self
                    .snapshot
                    .compiler
                    .bounds(&input)
                    .expect("fixture compiler bounds succeed");
                artifact_bytes += bounds.artifact().retained_bytes();
                cursor_peak = cursor_peak.max(bounds.cursor_bytes());
                temporary_peak = temporary_peak.max(bounds.temporary_bytes());
                compiler_lifetime_polls += bounds.work().remaining(WorkClass::BindingPolls);
            }

            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Bounded {
                    validated,
                    identities,
                    requirements: AggregateRequirements {
                        plan_count: COORDINATE_COUNT,
                        artifact_bytes,
                        compiler_cursor_peak: cursor_peak,
                        compiler_temporary_peak: temporary_peak,
                        compiler_lifetime_polls,
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
                identities,
                requirements,
            } = self.state;
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Reserved {
                    validated,
                    lease: PlanSetBuildLease::new(identities, requirements),
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
            let Reserved { validated, lease } = self.state;
            let planning = AggregatePlanning::new(
                validated,
                self.snapshot,
                lease.assignments().to_vec(),
                lease.requirements().compiler_lifetime_polls,
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

        fn abort(self) -> ConsumerAdmissionTxn<'td, 'reg, FailedSettled> {
            let Building { lease, planning } = self.state;
            planning.abort();
            drop(lease);
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
            let requirements = lease.requirements();
            assert_eq!(draft.assignments.len(), requirements.plan_count);
            assert_eq!(draft.artifact_count, requirements.plan_count);
            assert_eq!(requirements.artifact_bytes, 64 * requirements.plan_count as u64);
            assert_eq!(draft.assignments, lease.assignments());
            let authority = lease.commit();
            ConsumerAdmissionTxn {
                snapshot: self.snapshot,
                state: Frozen { authority, draft },
                _source_lifetime: std::marker::PhantomData,
            }
        }
    }

    fn fixture(
    ) -> (
        Thing,
        RegistrationSnapshot,
        ReservationTracker,
        ServientPlanIdentityAuthority,
    ) {
        let tracker = ReservationTracker::new();
        let authority =
            ServientPlanIdentityAuthority::new(PlanSetGeneration::INITIAL, tracker.clone());
        (Thing::default(), RegistrationSnapshot::fixture(), tracker, authority)
    }

    #[test]
    fn exact_identities_exist_before_bounds_and_resources_before_start() {
        let (thing, snapshot, tracker, authority) = fixture();
        let captured = ConsumerAdmissionTxn::capture(&thing, &snapshot, authority);
        let enumerated = captured.validate().enumerate();
        assert_eq!(snapshot.compiler.bounds_calls.get(), 0);
        assert_eq!(snapshot.compiler.start_calls.get(), 0);
        assert_eq!(tracker.identities(), 0);
        assert_eq!(tracker.resources(), 0);

        let identified = enumerated.assign_identities();
        assert_eq!(tracker.identities(), 1);
        assert_eq!(tracker.resources(), 0);
        assert_eq!(snapshot.compiler.bounds_calls.get(), 0);

        let bounded = identified.collect_bounds();
        assert_eq!(snapshot.compiler.bounds_calls.get(), COORDINATE_COUNT as u32);
        assert_eq!(snapshot.compiler.start_calls.get(), 0);
        assert_eq!(tracker.resources(), 0);

        let expected_ids = bounded
            .state
            .identities
            .assignments()
            .iter()
            .map(|assignment| assignment.plan_id)
            .collect::<Vec<_>>();
        assert_eq!(*snapshot.compiler.bound_plan_ids.borrow(), expected_ids);

        let reserved = bounded.reserve_resources();
        assert_eq!(tracker.identities(), 1);
        assert_eq!(tracker.resources(), 1);
        assert_eq!(snapshot.compiler.start_calls.get(), 0);

        let building = reserved.start_build().expect("build starts after reservation");
        assert_eq!(snapshot.compiler.start_calls.get(), 1);
        assert_eq!(snapshot.compiler.started_plan_ids.borrow()[0], expected_ids[0]);
        building.abort();
        assert_eq!(tracker.identities(), 0);
        assert_eq!(tracker.resources(), 0);
    }

    #[test]
    fn pending_keeps_servient_lease_outside_planning_and_zero_budget_calls_nothing() {
        let (thing, snapshot, tracker, authority) = fixture();
        let building = ConsumerAdmissionTxn::capture(&thing, &snapshot, authority)
            .validate()
            .enumerate()
            .assign_identities()
            .collect_bounds()
            .reserve_resources()
            .start_build()
            .expect("fixture build starts");

        assert_eq!(tracker.identities(), 1);
        assert_eq!(tracker.resources(), 1);
        let calls_before = snapshot.compiler.step_calls.get();
        let mut zero = WorkBudget::new();
        let building = match building.step(&mut zero) {
            BuildProgress::Pending(building) => building,
            _ => panic!("zero budget must keep the same admission transaction pending"),
        };
        assert_eq!(snapshot.compiler.step_calls.get(), calls_before);
        assert_eq!(tracker.identities(), 1);
        assert_eq!(tracker.resources(), 1);

        building.abort();
        assert_eq!(snapshot.compiler.abort_calls.get(), 1);
        assert_eq!(tracker.identities(), 0);
        assert_eq!(tracker.resources(), 0);
    }

    #[test]
    fn abort_after_real_pending_compiler_step_settles_every_owner_once() {
        let (thing, snapshot, tracker, authority) = fixture();
        let building = ConsumerAdmissionTxn::capture(&thing, &snapshot, authority)
            .validate()
            .enumerate()
            .assign_identities()
            .collect_bounds()
            .reserve_resources()
            .start_build()
            .expect("fixture build starts");

        let mut caller = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        let building = match building.step(&mut caller) {
            BuildProgress::Pending(building) => building,
            _ => panic!("first compiler step is deliberately pending"),
        };
        assert_eq!(snapshot.compiler.step_calls.get(), 1);
        assert_eq!(snapshot.compiler.abort_calls.get(), 0);

        let _failed = building.abort();
        assert_eq!(snapshot.compiler.abort_calls.get(), 1);
        assert_eq!(tracker.identities(), 0);
        assert_eq!(tracker.resources(), 0);
    }

    #[test]
    fn complete_aggregate_reconciles_two_plans_and_commits_servient_authority() {
        let (thing, snapshot, tracker, authority) = fixture();
        let mut building = ConsumerAdmissionTxn::capture(&thing, &snapshot, authority)
            .validate()
            .enumerate()
            .assign_identities()
            .collect_bounds()
            .reserve_resources()
            .start_build()
            .expect("fixture build starts");

        let mut caller = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 8);
        let reconciling = loop {
            match building.step(&mut caller) {
                BuildProgress::Pending(next) => building = next,
                BuildProgress::Reconciling(done) => break done,
                BuildProgress::Failed(failed) => {
                    failed.abort();
                    panic!("fixture compiler must not fail")
                }
            }
        };

        assert_eq!(snapshot.compiler.bounds_calls.get(), COORDINATE_COUNT as u32);
        assert_eq!(snapshot.compiler.start_calls.get(), COORDINATE_COUNT as u32);
        assert_eq!(snapshot.compiler.step_calls.get(), (COORDINATE_COUNT * 2) as u32);
        assert_eq!(snapshot.compiler.abort_calls.get(), 0);
        assert_eq!(
            *snapshot.compiler.bound_plan_ids.borrow(),
            *snapshot.compiler.started_plan_ids.borrow()
        );

        let frozen = reconciling.freeze();
        assert_eq!(frozen.state.draft.artifact_count, COORDINATE_COUNT);
        assert_eq!(frozen.state.authority.assignments.len(), COORDINATE_COUNT);
        assert_eq!(frozen.state.authority.requirements.plan_count, COORDINATE_COUNT);
        assert_eq!(tracker.identities(), 0);
        assert_eq!(tracker.resources(), 0);
    }
}
