#![allow(dead_code)]

use core::cell::Cell;
use core::mem::{align_of, offset_of, size_of, ManuallyDrop};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint, BindingArtifactRole,
    BindingCandidate, BindingCompilerBounds, BindingCompilerExtension, BindingCompilerInput,
    BindingCompilerOutput, BindingCompilerStep, BindingConfigurationDigest, BindingGeneration,
    BindingId, BindingRegistrationIdentity, CoreError, LogicalInteractionPlan, PlanId,
    PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    security_scheme::SecurityScheme,
    thing::Thing,
};
use std::{
    collections::btree_map::Iter as BTreeIter,
    rc::Rc,
    slice::Iter as SliceIter,
};

/// Non-production Stage-A constructibility model only.
///
/// This file deliberately exercises the current binding-compiler SPI while
/// proving the proposed admission ownership/resource topology. It creates no
/// admitted production API.
mod stage_a {
    use super::*;

    const FIXTURE_CURSOR_CAPACITY: usize = 64;
    const FIXTURE_TEMPORARY_CAPACITY: usize = 64;
    const FIXTURE_ARTIFACT_CAPACITY: usize = 128;
    const FIXTURE_LOGICAL_PLAN_BYTES: u64 = 128;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ValidationIssue {
        kind: u16,
        location: u32,
    }

    #[repr(C)]
    pub union FailureSlot {
        validation: ManuallyDrop<ValidationIssue>,
        core: ManuallyDrop<CoreError>,
    }

    /// Borrowed traversal state. The source itself stays caller-owned and
    /// immutably borrowed for the complete validation/Planning admission.
    pub struct BorrowedTdCursor<'a> {
        properties: Option<BTreeIter<'a, String, PropertyAffordance>>,
        actions: Option<BTreeIter<'a, String, ActionAffordance>>,
        events: Option<BTreeIter<'a, String, EventAffordance>>,
        forms: Option<SliceIter<'a, Form>>,
        security: SliceIter<'a, String>,
        security_definitions: BTreeIter<'a, String, SecurityScheme>,
        schema_definitions: Option<BTreeIter<'a, String, DataSchema>>,
        uri_variables: Option<BTreeIter<'a, String, DataSchema>>,
    }

    impl<'a> BorrowedTdCursor<'a> {
        pub fn new(thing: &'a Thing) -> Self {
            Self {
                properties: thing.properties.as_ref().map(|values| values.iter()),
                actions: thing.actions.as_ref().map(|values| values.iter()),
                events: thing.events.as_ref().map(|values| values.iter()),
                forms: thing.forms.as_ref().map(|values| values.iter()),
                security: thing.security.iter(),
                security_definitions: thing.security_definitions.iter(),
                schema_definitions: thing.schema_definitions.as_ref().map(|values| values.iter()),
                uri_variables: thing.uri_variables.as_ref().map(|values| values.iter()),
            }
        }

        pub fn progress_once(&mut self) -> usize {
            let mut progressed = 0;
            progressed += self
                .properties
                .as_mut()
                .is_some_and(|values| values.next().is_some()) as usize;
            progressed += self
                .actions
                .as_mut()
                .is_some_and(|values| values.next().is_some()) as usize;
            progressed += self
                .events
                .as_mut()
                .is_some_and(|values| values.next().is_some()) as usize;
            progressed += self
                .forms
                .as_mut()
                .is_some_and(|values| values.next().is_some()) as usize;
            progressed += self.security.next().is_some() as usize;
            progressed += self.security_definitions.next().is_some() as usize;
            progressed += self
                .schema_definitions
                .as_mut()
                .is_some_and(|values| values.next().is_some()) as usize;
            progressed += self
                .uri_variables
                .as_mut()
                .is_some_and(|values| values.next().is_some()) as usize;
            progressed
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BuildIdentity {
        plan_id: PlanId,
        plan_set_generation: PlanSetGeneration,
        owner_id: u64,
    }

    /// Move-only reservation issued by the upstream plan-set identity owner.
    /// The tracker provides a model RAII release fallback; explicit rejection
    /// and abort paths return the lease so ownership is observable.
    #[must_use]
    pub struct UnpublishedPlanBuildLease {
        identity: BuildIdentity,
        outstanding: Rc<Cell<u32>>,
        active: bool,
    }

    impl UnpublishedPlanBuildLease {
        pub fn plan_id(&self) -> PlanId {
            self.identity.plan_id
        }

        pub fn plan_set_generation(&self) -> PlanSetGeneration {
            self.identity.plan_set_generation
        }

        fn settle(&mut self) {
            if self.active {
                self.outstanding.set(
                    self.outstanding
                        .get()
                        .checked_sub(1)
                        .expect("active lease must have an outstanding reservation"),
                );
                self.active = false;
            }
        }

        pub fn release(mut self) {
            self.settle();
        }

        fn commit(mut self) -> BuildIdentity {
            self.settle();
            self.identity
        }
    }

    impl Drop for UnpublishedPlanBuildLease {
        fn drop(&mut self) {
            self.settle();
        }
    }

    pub struct PlanSetIdentityAuthority {
        owner_id: u64,
        plan_set_generation: PlanSetGeneration,
        next_plan_slot: u32,
        next_plan_generation: Generation,
        outstanding: Rc<Cell<u32>>,
    }

    impl PlanSetIdentityAuthority {
        pub fn new(
            owner_id: u64,
            plan_set_generation: PlanSetGeneration,
            next_plan_slot: u32,
            next_plan_generation: Generation,
        ) -> Self {
            Self {
                owner_id,
                plan_set_generation,
                next_plan_slot,
                next_plan_generation,
                outstanding: Rc::new(Cell::new(0)),
            }
        }

        pub fn reserve(&mut self) -> UnpublishedPlanBuildLease {
            let identity = BuildIdentity {
                plan_id: PlanId::new(
                    SlotIndex::new(self.next_plan_slot),
                    self.next_plan_generation,
                ),
                plan_set_generation: self.plan_set_generation,
                owner_id: self.owner_id,
            };
            self.next_plan_slot = self
                .next_plan_slot
                .checked_add(1)
                .expect("fixture plan slots stay bounded");
            self.next_plan_generation = self
                .next_plan_generation
                .checked_next()
                .expect("fixture plan generations stay bounded");
            self.outstanding.set(self.outstanding.get() + 1);
            UnpublishedPlanBuildLease {
                identity,
                outstanding: Rc::clone(&self.outstanding),
                active: true,
            }
        }

        pub fn outstanding(&self) -> u32 {
            self.outstanding.get()
        }
    }

    #[derive(Clone, Copy)]
    struct TypedTdPolicy {
        nesting_depth_max: u64,
        members_per_map_max: u64,
        items_per_sequence_max: u64,
        value_nodes_per_thing_max: u64,
        string_bytes_per_thing_max: u64,
        admission_work_units_max: u64,
        affordances_per_thing_max: u64,
        forms_per_context_max: u64,
        forms_per_thing_max: u64,
        additional_responses_per_form_max: u64,
        uri_variables_per_form_max: u64,
        schema_nodes_per_document_max: u64,
        schema_composition_depth_max: u64,
        schema_reference_edges_per_document_max: u64,
        uri_template_source_bytes_max: u64,
        uri_template_variables_max: u64,
        form_binding_candidates_per_operation_max: u64,
    }

    #[derive(Clone, Copy)]
    struct MemoryPolicy {
        retained_source_bytes_per_owner_max: u64,
        retained_source_bytes_global_max: u64,
        admission_temporary_bytes_per_operation_max: u64,
        admission_temporary_bytes_global_max: u64,
        peak_live_bytes_per_admission_max: u64,
        admission_peak_live_bytes_global_max: u64,
        engine_live_bytes_global_max: u64,
        largest_contiguous_allocation_bytes_max: u64,
    }

    #[derive(Clone, Copy)]
    struct PlanningPolicy {
        compiled_plan_bytes_max: u64,
        logical_plan_bytes_per_thing_max: u64,
        compiled_runtime_bytes_per_thing_max: u64,
        compiled_runtime_bytes_global_max: u64,
        plan_sets_per_thing_max: u64,
        plan_sets_global_max: u64,
        plan_pins_per_plan_set_max: u64,
        plan_pins_global_max: u64,
        binding_artifacts_per_thing_max: u64,
        binding_artifacts_global_max: u64,
        binding_artifact_bytes_per_item_max: u64,
        binding_artifact_bytes_per_thing_max: u64,
        binding_artifact_bytes_global_max: u64,
        binding_compiler_cursor_bytes_per_item_max: u64,
        binding_compiler_cursor_bytes_global_max: u64,
        plan_compile_work_units_per_step_max: u64,
        plan_reclaim_bytes_per_step_max: u64,
    }

    /// Non-optional checked projection. Raw/document-only fields do not appear;
    /// every field that applies to borrowed typed admission + first-proof plan
    /// compilation is represented as a concrete value rather than Option<u64>.
    #[derive(Clone, Copy)]
    pub struct CheckedPolicy {
        typed_td: TypedTdPolicy,
        memory: MemoryPolicy,
        planning: PlanningPolicy,
    }

    impl CheckedPolicy {
        pub fn fixture() -> Self {
            Self {
                typed_td: TypedTdPolicy {
                    nesting_depth_max: 64,
                    members_per_map_max: 8192,
                    items_per_sequence_max: 8192,
                    value_nodes_per_thing_max: 65536,
                    string_bytes_per_thing_max: 262144,
                    admission_work_units_max: 1024,
                    affordances_per_thing_max: 1024,
                    forms_per_context_max: 32,
                    forms_per_thing_max: 4096,
                    additional_responses_per_form_max: 32,
                    uri_variables_per_form_max: 64,
                    schema_nodes_per_document_max: 65536,
                    schema_composition_depth_max: 32,
                    schema_reference_edges_per_document_max: 131072,
                    uri_template_source_bytes_max: 16384,
                    uri_template_variables_max: 64,
                    form_binding_candidates_per_operation_max: 32,
                },
                memory: MemoryPolicy {
                    retained_source_bytes_per_owner_max: 1048576,
                    retained_source_bytes_global_max: 536870912,
                    admission_temporary_bytes_per_operation_max: 4096,
                    admission_temporary_bytes_global_max: 4096,
                    peak_live_bytes_per_admission_max: 4096,
                    admission_peak_live_bytes_global_max: 4096,
                    engine_live_bytes_global_max: 4096,
                    largest_contiguous_allocation_bytes_max: 4096,
                },
                planning: PlanningPolicy {
                    compiled_plan_bytes_max: 4096,
                    logical_plan_bytes_per_thing_max: 4096,
                    compiled_runtime_bytes_per_thing_max: 4096,
                    compiled_runtime_bytes_global_max: 4096,
                    plan_sets_per_thing_max: 4,
                    plan_sets_global_max: 32,
                    plan_pins_per_plan_set_max: 256,
                    plan_pins_global_max: 1024,
                    binding_artifacts_per_thing_max: 256,
                    binding_artifacts_global_max: 1024,
                    binding_artifact_bytes_per_item_max: 4096,
                    binding_artifact_bytes_per_thing_max: 4096,
                    binding_artifact_bytes_global_max: 4096,
                    binding_compiler_cursor_bytes_per_item_max: 4096,
                    binding_compiler_cursor_bytes_global_max: 4096,
                    plan_compile_work_units_per_step_max: 1,
                    plan_reclaim_bytes_per_step_max: 4096,
                },
            }
        }
    }

    pub struct FixtureCompiler {
        compatibility: BindingArtifactCompatibility,
        bounds_calls: Cell<u32>,
        start_calls: Cell<u32>,
        step_calls: Cell<u32>,
    }

    #[derive(Debug, Eq, PartialEq)]
    pub struct FixtureCursor {
        step: u8,
    }

    impl FixtureCompiler {
        fn new(compatibility: BindingArtifactCompatibility) -> Self {
            Self {
                compatibility,
                bounds_calls: Cell::new(0),
                start_calls: Cell::new(0),
                step_calls: Cell::new(0),
            }
        }

        fn bounds_calls(&self) -> u32 {
            self.bounds_calls.get()
        }

        fn start_calls(&self) -> u32 {
            self.start_calls.get()
        }

        fn step_calls(&self) -> u32 {
            self.step_calls.get()
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
            _input: &BindingCompilerInput<'_>,
        ) -> Result<BindingCompilerBounds, CoreError> {
            self.bounds_calls.set(self.bounds_calls.get() + 1);
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, 64),
                32,
                48,
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, 3),
            ))
        }

        fn start(&self, _input: &BindingCompilerInput<'_>) -> Result<Self::Cursor, CoreError> {
            self.start_calls.set(self.start_calls.get() + 1);
            Ok(FixtureCursor { step: 0 })
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
            cursor.step += 1;
            if cursor.step < 2 {
                BindingCompilerStep::Pending(cursor)
            } else {
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    self.compatibility,
                    BindingArtifactFootprint::new(1, 64),
                    7,
                )))
            }
        }

        fn abort(&self, _cursor: Self::Cursor) {}
    }

    pub struct CompleteRegistration {
        identity: BindingRegistrationIdentity,
        compiler: FixtureCompiler,
    }

    impl CompleteRegistration {
        pub fn identity(&self) -> BindingRegistrationIdentity {
            self.identity
        }
    }

    pub struct RegistrationSnapshot {
        entries: Vec<CompleteRegistration>,
    }

    impl RegistrationSnapshot {
        pub fn new(entries: Vec<CompleteRegistration>) -> Self {
            Self { entries }
        }

        fn get(&self, ordinal: usize) -> Option<&CompleteRegistration> {
            self.entries.get(ordinal)
        }

        pub fn bounds_calls(&self, ordinal: usize) -> u32 {
            self.entries[ordinal].compiler.bounds_calls()
        }

        pub fn start_calls(&self, ordinal: usize) -> u32 {
            self.entries[ordinal].compiler.start_calls()
        }

        pub fn step_calls(&self, ordinal: usize) -> u32 {
            self.entries[ordinal].compiler.step_calls()
        }
    }

    pub struct Validating<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        snapshot_ordinal: usize,
        cancellation_generation: u64,
        policy: CheckedPolicy,
        cursor: BorrowedTdCursor<'td>,
    }

    pub struct Validated<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        snapshot_ordinal: usize,
        cancellation_generation: u64,
        policy: CheckedPolicy,
    }

    impl<'td, 'reg> Validating<'td, 'reg> {
        pub fn new(
            source: &'td Thing,
            snapshot: &'reg RegistrationSnapshot,
            snapshot_ordinal: usize,
            cancellation_generation: u64,
            policy: CheckedPolicy,
        ) -> Self {
            Self {
                source,
                snapshot,
                snapshot_ordinal,
                cancellation_generation,
                policy,
                cursor: BorrowedTdCursor::new(source),
            }
        }

        pub fn progress_once(&mut self) -> usize {
            self.cursor.progress_once()
        }

        pub fn validated(self) -> Validated<'td, 'reg> {
            Validated {
                source: self.source,
                snapshot: self.snapshot,
                snapshot_ordinal: self.snapshot_ordinal,
                cancellation_generation: self.cancellation_generation,
                policy: self.policy,
            }
        }
    }

    pub struct CapturedCompilerBounds {
        artifact: BindingArtifactFootprint,
        cursor_bytes: u64,
        temporary_bytes: u64,
        lifetime_work: WorkBudget,
    }

    impl CapturedCompilerBounds {
        fn capture(bounds: BindingCompilerBounds) -> Self {
            Self {
                artifact: bounds.artifact(),
                cursor_bytes: bounds.cursor_bytes(),
                temporary_bytes: bounds.temporary_bytes(),
                lifetime_work: bounds.into_work(),
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct CompilerReservation {
        logical_plan_bytes: u64,
        temporary_bytes: u64,
        runtime_bytes: u64,
    }

    #[derive(Clone, Copy)]
    pub struct AdmissionAccounting {
        policy: CheckedPolicy,
    }

    impl AdmissionAccounting {
        pub fn from_policy(policy: CheckedPolicy) -> Self {
            Self { policy }
        }

        pub fn too_small(policy: CheckedPolicy) -> Self {
            let mut value = policy;
            value.memory.admission_temporary_bytes_per_operation_max = 16;
            value.memory.admission_temporary_bytes_global_max = 16;
            Self { policy: value }
        }

        fn preflight_logical_plan(&self, bytes: u64) -> Result<(), ()> {
            let planning = self.policy.planning;
            if bytes > planning.logical_plan_bytes_per_thing_max
                || bytes > planning.compiled_plan_bytes_max
                || bytes > self.policy.memory.largest_contiguous_allocation_bytes_max
            {
                return Err(());
            }
            Ok(())
        }

        fn reserve_compiler(
            &self,
            bounds: &CapturedCompilerBounds,
            logical_plan_bytes: u64,
        ) -> Result<CompilerReservation, ()> {
            let memory = self.policy.memory;
            let planning = self.policy.planning;
            let temporary = bounds
                .cursor_bytes
                .checked_add(bounds.temporary_bytes)
                .ok_or(())?;
            let runtime = bounds.artifact.retained_bytes();
            let compiled_plan = logical_plan_bytes.checked_add(runtime).ok_or(())?;
            let artifact_count = 1_u64;

            if temporary > memory.admission_temporary_bytes_per_operation_max
                || temporary > memory.admission_temporary_bytes_global_max
                || compiled_plan > memory.peak_live_bytes_per_admission_max
                || compiled_plan > memory.admission_peak_live_bytes_global_max
                || runtime > memory.engine_live_bytes_global_max
                || bounds.cursor_bytes > memory.largest_contiguous_allocation_bytes_max
                || bounds.temporary_bytes > memory.largest_contiguous_allocation_bytes_max
                || runtime > memory.largest_contiguous_allocation_bytes_max
                || compiled_plan > planning.compiled_plan_bytes_max
                || logical_plan_bytes > planning.logical_plan_bytes_per_thing_max
                || runtime > planning.compiled_runtime_bytes_per_thing_max
                || runtime > planning.compiled_runtime_bytes_global_max
                || artifact_count > planning.binding_artifacts_per_thing_max
                || artifact_count > planning.binding_artifacts_global_max
                || runtime > planning.binding_artifact_bytes_per_item_max
                || runtime > planning.binding_artifact_bytes_per_thing_max
                || runtime > planning.binding_artifact_bytes_global_max
                || bounds.cursor_bytes > planning.binding_compiler_cursor_bytes_per_item_max
                || bounds.cursor_bytes > planning.binding_compiler_cursor_bytes_global_max
            {
                return Err(());
            }

            Ok(CompilerReservation {
                logical_plan_bytes,
                temporary_bytes: temporary,
                runtime_bytes: runtime,
            })
        }
    }

    /// Reservation adapter around the existing SPI. Parent budgets are charged
    /// before the compiler callback. A child WorkBudget containing only the
    /// jointly reserved allowance is passed to BindingCompilerExtension::step.
    /// Unused reservation is reconciled afterwards while exclusive parent
    /// borrows remain held by this value.
    struct PairedWorkReservation<'a> {
        lifetime: &'a mut WorkBudget,
        caller_step: &'a mut WorkBudget,
        granted: [u64; WorkClass::ALL.len()],
        child: WorkBudget,
    }

    impl<'a> PairedWorkReservation<'a> {
        fn reserve(
            lifetime: &'a mut WorkBudget,
            caller_step: &'a mut WorkBudget,
            per_step_total_max: u64,
        ) -> Result<Self, ()> {
            let mut granted = [0_u64; WorkClass::ALL.len()];
            let mut child = WorkBudget::new();
            let mut total_remaining = per_step_total_max;

            for class in WorkClass::ALL {
                if total_remaining == 0 {
                    break;
                }
                let units = lifetime
                    .remaining(class)
                    .min(caller_step.remaining(class))
                    .min(total_remaining);
                if units == 0 {
                    continue;
                }
                lifetime.consume(class, units).map_err(|_| ())?;
                caller_step.consume(class, units).map_err(|_| ())?;
                child.set_remaining(class, units);
                granted[class as usize] = units;
                total_remaining -= units;
            }

            if granted.iter().all(|units| *units == 0) {
                return Err(());
            }

            Ok(Self {
                lifetime,
                caller_step,
                granted,
                child,
            })
        }

        fn child_mut(&mut self) -> &mut WorkBudget {
            &mut self.child
        }

        fn finish(self) -> u64 {
            let mut used_total = 0_u64;
            for class in WorkClass::ALL {
                let granted = self.granted[class as usize];
                let unused = self.child.remaining(class);
                let used = granted
                    .checked_sub(unused)
                    .expect("compiler cannot exceed its child reservation");
                used_total += used;
                if unused != 0 {
                    self.lifetime.set_remaining(
                        class,
                        self.lifetime
                            .remaining(class)
                            .checked_add(unused)
                            .expect("reservation reconciliation is bounded"),
                    );
                    self.caller_step.set_remaining(
                        class,
                        self.caller_step
                            .remaining(class)
                            .checked_add(unused)
                            .expect("reservation reconciliation is bounded"),
                    );
                }
            }
            used_total
        }
    }

    pub struct PlanningEntryRejection<'td, 'reg> {
        validated: Validated<'td, 'reg>,
        lease: UnpublishedPlanBuildLease,
    }

    impl<'td, 'reg> PlanningEntryRejection<'td, 'reg> {
        pub fn into_parts(self) -> (Validated<'td, 'reg>, UnpublishedPlanBuildLease) {
            (self.validated, self.lease)
        }
    }

    pub struct Planning<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        registration: &'reg CompleteRegistration,
        registration_snapshot_ordinal: usize,
        cancellation_generation: u64,
        policy: CheckedPolicy,
        lease: UnpublishedPlanBuildLease,
        logical_plan: LogicalInteractionPlan,
        bounds: CapturedCompilerBounds,
        reservation: CompilerReservation,
        compiler_cursor: Option<FixtureCursor>,
    }

    impl<'td, 'reg> Validated<'td, 'reg> {
        pub fn enter_planning(
            self,
            lease: UnpublishedPlanBuildLease,
            accounting: AdmissionAccounting,
        ) -> Result<Planning<'td, 'reg>, PlanningEntryRejection<'td, 'reg>> {
            let reject = |validated: Self, lease| PlanningEntryRejection { validated, lease };

            let registration = match self.snapshot.get(self.snapshot_ordinal) {
                Some(registration) => registration,
                None => return Err(reject(self, lease)),
            };
            if registration.identity().artifact_compatibility()
                != registration.compiler.compatibility()
            {
                return Err(reject(self, lease));
            }

            if accounting
                .preflight_logical_plan(FIXTURE_LOGICAL_PLAN_BYTES)
                .is_err()
            {
                return Err(reject(self, lease));
            }

            let logical_plan = LogicalInteractionPlan::try_property_read(
                lease.plan_id(),
                ThingId::from("urn:stage-a:thing"),
                Box::from("temperature"),
                0,
                Box::from("mock://temperature"),
                None,
                None,
            )
            .expect("fixture logical plan is structurally valid");
            let identity = registration.identity();
            let candidate = BindingCandidate::new(
                identity.binding_id(),
                identity.binding_generation(),
                identity.configuration(),
                identity.artifact_compatibility(),
                self.snapshot_ordinal as u32,
                0,
            );
            let input = BindingCompilerInput::new(
                &logical_plan,
                candidate,
                BindingArtifactRole::ConsumerCall,
            );
            let bounds = CapturedCompilerBounds::capture(
                registration
                    .compiler
                    .bounds(&input)
                    .expect("fixture bounds are infallible"),
            );
            let reservation = match accounting.reserve_compiler(&bounds, FIXTURE_LOGICAL_PLAN_BYTES)
            {
                Ok(reservation) => reservation,
                Err(()) => return Err(reject(self, lease)),
            };
            let compiler_cursor = registration
                .compiler
                .start(&input)
                .expect("fixture start is infallible");

            Ok(Planning {
                source: self.source,
                snapshot: self.snapshot,
                registration,
                registration_snapshot_ordinal: self.snapshot_ordinal,
                cancellation_generation: self.cancellation_generation,
                policy: self.policy,
                lease,
                logical_plan,
                bounds,
                reservation,
                compiler_cursor: Some(compiler_cursor),
            })
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum CompilerProgress {
        Pending { work_used: u64 },
        Complete { work_used: u64, artifact_bytes: u64 },
    }

    impl Planning<'_, '_> {
        fn candidate(&self) -> BindingCandidate {
            let identity = self.registration.identity();
            BindingCandidate::new(
                identity.binding_id(),
                identity.binding_generation(),
                identity.configuration(),
                identity.artifact_compatibility(),
                self.registration_snapshot_ordinal as u32,
                0,
            )
        }

        pub fn step_compiler(
            &mut self,
            caller_step: &mut WorkBudget,
        ) -> Result<CompilerProgress, ()> {
            let cursor = self.compiler_cursor.take().ok_or(())?;
            let candidate = self.candidate();
            let input = BindingCompilerInput::new(
                &self.logical_plan,
                candidate,
                BindingArtifactRole::ConsumerCall,
            );
            let mut work = match PairedWorkReservation::reserve(
                &mut self.bounds.lifetime_work,
                caller_step,
                self.policy.planning.plan_compile_work_units_per_step_max,
            ) {
                Ok(work) => work,
                Err(()) => {
                    self.compiler_cursor = Some(cursor);
                    return Err(());
                }
            };

            // This is the real current SPI shape: exactly one &mut WorkBudget
            // is passed to the compiler step. It is the bounded child budget.
            let step = self
                .registration
                .compiler
                .step(&input, cursor, work.child_mut());
            let work_used = work.finish();

            match step {
                BindingCompilerStep::Pending(cursor) => {
                    self.compiler_cursor = Some(cursor);
                    Ok(CompilerProgress::Pending { work_used })
                }
                BindingCompilerStep::Complete(output) => {
                    let bytes = output.artifact().footprint().retained_bytes();
                    self.compiler_cursor = None;
                    Ok(CompilerProgress::Complete {
                        work_used,
                        artifact_bytes: bytes,
                    })
                }
                BindingCompilerStep::Failed(failure) => {
                    let (_, cursor) = failure.into_parts();
                    self.compiler_cursor = Some(cursor);
                    Err(())
                }
            }
        }

        pub fn selected_identity(&self) -> BindingRegistrationIdentity {
            self.registration.identity()
        }

        pub fn selected_snapshot_ordinal(&self) -> usize {
            self.registration_snapshot_ordinal
        }

        pub fn plan_id(&self) -> PlanId {
            self.lease.plan_id()
        }

        pub fn plan_set_generation(&self) -> PlanSetGeneration {
            self.lease.plan_set_generation()
        }

        pub fn compiler_lifetime_remaining(&self, class: WorkClass) -> u64 {
            self.bounds.lifetime_work.remaining(class)
        }

        pub fn abort(self) -> UnpublishedPlanBuildLease {
            self.lease
        }

        pub fn freeze(self) -> (PlanId, PlanSetGeneration) {
            let identity = self.lease.commit();
            (identity.plan_id, identity.plan_set_generation)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct TypedTdWorkMeter {
        remaining: u64,
    }

    impl TypedTdWorkMeter {
        pub fn new(remaining: u64) -> Self {
            Self { remaining }
        }

        pub fn remaining(&self) -> u64 {
            self.remaining
        }
    }

    pub fn consume_typed_td_pair(
        lifetime: &mut TypedTdWorkMeter,
        step: &mut TypedTdWorkMeter,
        units: u64,
    ) -> Result<(), ()> {
        if lifetime.remaining < units || step.remaining < units {
            return Err(());
        }
        lifetime.remaining -= units;
        step.remaining -= units;
        Ok(())
    }

    #[repr(C)]
    pub struct AccountingStorage {
        local_temporary: u64,
        global_temporary: u64,
        local_peak: u64,
        global_peak: u64,
        engine_live: u64,
        runtime: u64,
        contiguous: u64,
    }

    #[repr(C)]
    pub struct CompilerStorage {
        cursor_region: [u8; FIXTURE_CURSOR_CAPACITY],
        temporary_region: [u8; FIXTURE_TEMPORARY_CAPACITY],
        artifact_region: [u8; FIXTURE_ARTIFACT_CAPACITY],
        lifetime_work: WorkBudget,
    }

    /// Real inline typestate slot for every state modeled in this fixture.
    #[repr(C)]
    pub union AdmissionStateSlot<'td, 'reg> {
        cursor: ManuallyDrop<BorrowedTdCursor<'td>>,
        validating: ManuallyDrop<Validating<'td, 'reg>>,
        validated: ManuallyDrop<Validated<'td, 'reg>>,
        planning: ManuallyDrop<Planning<'td, 'reg>>,
    }

    #[repr(C)]
    pub struct HostAdmissionStorage<'td, 'reg> {
        tag: u8,
        host_generation: u64,
        state: AdmissionStateSlot<'td, 'reg>,
        failure: FailureSlot,
        accounting: AccountingStorage,
        compiler: CompilerStorage,
    }

    #[repr(C)]
    pub struct StaticAdmissionStorage<'td, 'reg> {
        tag: u8,
        static_slot: u32,
        state: AdmissionStateSlot<'td, 'reg>,
        failure: FailureSlot,
        accounting: AccountingStorage,
        compiler: CompilerStorage,
    }

    #[derive(Debug, Eq, PartialEq)]
    pub struct LayoutRecord {
        pub total_size: usize,
        pub alignment: usize,
        pub structural: (usize, usize),
        pub state: (usize, usize),
        pub diagnostic: (usize, usize),
        pub accounting: (usize, usize),
        pub compiler: (usize, usize),
    }

    fn assert_partition(record: &LayoutRecord) {
        let ranges = [
            record.structural,
            record.state,
            record.diagnostic,
            record.accounting,
            record.compiler,
        ];
        assert_eq!(ranges[0].0, 0);
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
        assert_eq!(ranges.last().expect("five ranges").1, record.total_size);
    }

    pub fn host_layout() -> LayoutRecord {
        type Storage = HostAdmissionStorage<'static, 'static>;
        let state = offset_of!(Storage, state);
        let diagnostic = offset_of!(Storage, failure);
        let accounting = offset_of!(Storage, accounting);
        let compiler = offset_of!(Storage, compiler);
        let record = LayoutRecord {
            total_size: size_of::<Storage>(),
            alignment: align_of::<Storage>(),
            structural: (0, state),
            state: (state, diagnostic),
            diagnostic: (diagnostic, accounting),
            accounting: (accounting, compiler),
            compiler: (compiler, size_of::<Storage>()),
        };
        assert_partition(&record);
        record
    }

    pub fn static_layout() -> LayoutRecord {
        type Storage = StaticAdmissionStorage<'static, 'static>;
        let state = offset_of!(Storage, state);
        let diagnostic = offset_of!(Storage, failure);
        let accounting = offset_of!(Storage, accounting);
        let compiler = offset_of!(Storage, compiler);
        let record = LayoutRecord {
            total_size: size_of::<Storage>(),
            alignment: align_of::<Storage>(),
            structural: (0, state),
            state: (state, diagnostic),
            diagnostic: (diagnostic, accounting),
            accounting: (accounting, compiler),
            compiler: (compiler, size_of::<Storage>()),
        };
        assert_partition(&record);
        record
    }

    pub fn registration(
        binding_id: u32,
        binding_generation: u32,
        configuration_byte: u8,
        compatibility_byte: u8,
        diagnostic_ordinal: u32,
    ) -> CompleteRegistration {
        let generation =
            Generation::new(binding_generation).expect("fixture generation must be nonzero");
        let compatibility = BindingArtifactCompatibility::new([compatibility_byte; 16]);
        let identity = BindingRegistrationIdentity::new(
            BindingId::new(binding_id),
            BindingGeneration::new(generation),
            BindingConfigurationDigest::new([configuration_byte; 32]),
            compatibility,
            diagnostic_ordinal,
        );
        CompleteRegistration {
            identity,
            compiler: FixtureCompiler::new(compatibility),
        }
    }

    pub fn state_slot_size() -> usize {
        size_of::<AdmissionStateSlot<'static, 'static>>()
    }

    pub fn compiler_storage_size() -> usize {
        size_of::<CompilerStorage>()
    }
}

fn plan_authority(owner_id: u64, generation: u32, slot: u32) -> stage_a::PlanSetIdentityAuthority {
    stage_a::PlanSetIdentityAuthority::new(
        owner_id,
        PlanSetGeneration::new(Generation::new(generation).expect("nonzero generation")),
        slot,
        Generation::new(7).expect("nonzero generation"),
    )
}

#[test]
fn borrowed_td_cursor_is_constructible_without_source_ownership() {
    let thing = Thing::default();
    let mut cursor = stage_a::BorrowedTdCursor::new(&thing);
    assert_eq!(cursor.progress_once(), 0);
}

#[test]
fn build_lease_binds_both_identities_and_releases_on_rejection_abort_or_drop() {
    let registration = stage_a::registration(20, 1, 1, 1, 7);
    let snapshot = stage_a::RegistrationSnapshot::new(vec![registration]);
    let thing = Thing::default();
    let policy = stage_a::CheckedPolicy::fixture();
    let mut authority = plan_authority(44, 9, 4);

    let rejected = stage_a::Validating::new(&thing, &snapshot, 0, 1, policy).validated();
    let lease = authority.reserve();
    assert_eq!(authority.outstanding(), 1);
    let rejection = match rejected.enter_planning(
        lease,
        stage_a::AdmissionAccounting::too_small(policy),
    ) {
        Err(rejection) => rejection,
        Ok(_) => panic!("compiler reservation must fail before start"),
    };
    let (_validated, lease) = rejection.into_parts();
    assert_eq!(authority.outstanding(), 1);
    lease.release();
    assert_eq!(authority.outstanding(), 0);
    assert_eq!(snapshot.start_calls(0), 0);

    let validated = stage_a::Validating::new(&thing, &snapshot, 0, 1, policy).validated();
    let lease = authority.reserve();
    let planning = match validated.enter_planning(
        lease,
        stage_a::AdmissionAccounting::from_policy(policy),
    ) {
        Ok(planning) => planning,
        Err(_) => panic!("valid reservation must enter Planning"),
    };
    assert_eq!(authority.outstanding(), 1);
    planning.abort().release();
    assert_eq!(authority.outstanding(), 0);

    let dropped = authority.reserve();
    assert_eq!(authority.outstanding(), 1);
    drop(dropped);
    assert_eq!(authority.outstanding(), 0);
}

#[test]
fn ordinal_domains_and_same_registration_compiler_source_remain_distinct() {
    let compatibility = 5;
    let registrations = vec![
        stage_a::registration(10, 1, 10, compatibility, 101),
        stage_a::registration(11, 2, 11, compatibility, 102),
        stage_a::registration(12, 3, 12, compatibility, 103),
        stage_a::registration(13, 4, 13, compatibility, 17),
    ];
    let snapshot = stage_a::RegistrationSnapshot::new(registrations);
    let thing = Thing::default();
    let policy = stage_a::CheckedPolicy::fixture();
    let validated = stage_a::Validating::new(&thing, &snapshot, 3, 1, policy).validated();
    let mut authority = plan_authority(1, 8, 2);
    let planning = match validated.enter_planning(
        authority.reserve(),
        stage_a::AdmissionAccounting::from_policy(policy),
    ) {
        Ok(planning) => planning,
        Err(_) => panic!("same-entry construction must succeed"),
    };

    assert_eq!(planning.selected_snapshot_ordinal(), 3);
    assert_eq!(planning.selected_identity().diagnostic_ordinal(), 17);
    assert_eq!(planning.selected_identity().binding_id(), BindingId::new(13));
    assert_eq!(snapshot.bounds_calls(0), 0);
    assert_eq!(snapshot.bounds_calls(3), 1);
    assert_eq!(snapshot.start_calls(0), 0);
    assert_eq!(snapshot.start_calls(3), 1);
    assert_eq!(
        planning.plan_set_generation(),
        PlanSetGeneration::new(Generation::new(8).unwrap())
    );
    assert_eq!(planning.plan_id().slot(), SlotIndex::new(2));
    planning.abort().release();
    assert_eq!(authority.outstanding(), 0);
}

#[test]
fn compiler_work_reservation_wraps_the_real_spi_step_budget() {
    let registration = stage_a::registration(30, 1, 1, 1, 7);
    let snapshot = stage_a::RegistrationSnapshot::new(vec![registration]);
    let thing = Thing::default();
    let policy = stage_a::CheckedPolicy::fixture();
    let validated = stage_a::Validating::new(&thing, &snapshot, 0, 1, policy).validated();
    let mut authority = plan_authority(2, 5, 1);
    let mut planning = match validated.enter_planning(
        authority.reserve(),
        stage_a::AdmissionAccounting::from_policy(policy),
    ) {
        Ok(planning) => planning,
        Err(_) => panic!("Planning entry must succeed"),
    };

    let mut empty = WorkBudget::new();
    assert!(planning.step_compiler(&mut empty).is_err());
    assert_eq!(snapshot.step_calls(0), 0);
    assert_eq!(planning.compiler_lifetime_remaining(WorkClass::BindingPolls), 3);

    let mut first_step = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 5);
    assert_eq!(
        planning.step_compiler(&mut first_step).unwrap(),
        stage_a::CompilerProgress::Pending { work_used: 1 }
    );
    assert_eq!(snapshot.step_calls(0), 1);
    assert_eq!(planning.compiler_lifetime_remaining(WorkClass::BindingPolls), 2);
    assert_eq!(first_step.remaining(WorkClass::BindingPolls), 4);

    let mut second_step = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 5);
    assert_eq!(
        planning.step_compiler(&mut second_step).unwrap(),
        stage_a::CompilerProgress::Complete {
            work_used: 1,
            artifact_bytes: 64,
        }
    );
    assert_eq!(snapshot.step_calls(0), 2);
    assert_eq!(planning.compiler_lifetime_remaining(WorkClass::BindingPolls), 1);
    assert_eq!(second_step.remaining(WorkClass::BindingPolls), 4);

    let (_plan_id, _plan_set_generation) = planning.freeze();
    assert_eq!(authority.outstanding(), 0);
}

#[test]
fn typed_td_pair_charge_remains_failure_atomic() {
    let mut lifetime = stage_a::TypedTdWorkMeter::new(1);
    let mut empty_step = stage_a::TypedTdWorkMeter::new(0);
    assert!(stage_a::consume_typed_td_pair(&mut lifetime, &mut empty_step, 1).is_err());
    assert_eq!(lifetime.remaining(), 1);
    assert_eq!(empty_step.remaining(), 0);

    let mut step = stage_a::TypedTdWorkMeter::new(1);
    stage_a::consume_typed_td_pair(&mut lifetime, &mut step, 1).unwrap();
    assert_eq!(lifetime.remaining(), 0);
    assert_eq!(step.remaining(), 0);
}

#[test]
fn concrete_state_union_and_compiler_regions_fit_every_modeled_state() {
    type Slot = stage_a::AdmissionStateSlot<'static, 'static>;
    type Cursor = stage_a::BorrowedTdCursor<'static>;
    type Validating = stage_a::Validating<'static, 'static>;
    type Validated = stage_a::Validated<'static, 'static>;
    type Planning = stage_a::Planning<'static, 'static>;

    assert!(size_of::<Slot>() >= size_of::<Cursor>());
    assert!(size_of::<Slot>() >= size_of::<Validating>());
    assert!(size_of::<Slot>() >= size_of::<Validated>());
    assert!(size_of::<Slot>() >= size_of::<Planning>());
    assert!(align_of::<Slot>() >= align_of::<Cursor>());
    assert!(align_of::<Slot>() >= align_of::<Validating>());
    assert!(align_of::<Slot>() >= align_of::<Validated>());
    assert!(align_of::<Slot>() >= align_of::<Planning>());

    let host = stage_a::host_layout();
    let static_layout = stage_a::static_layout();
    assert!(host.state.1 - host.state.0 >= stage_a::state_slot_size());
    assert!(static_layout.state.1 - static_layout.state.0 >= stage_a::state_slot_size());
    assert!(host.compiler.1 - host.compiler.0 >= stage_a::compiler_storage_size());
    assert!(static_layout.compiler.1 - static_layout.compiler.0 >= stage_a::compiler_storage_size());
    assert!(host.diagnostic.1 - host.diagnostic.0 >= size_of::<stage_a::FailureSlot>());
    assert!(static_layout.diagnostic.1 - static_layout.diagnostic.0 >= size_of::<stage_a::FailureSlot>());
    assert!(host.alignment >= align_of::<Slot>());
    assert!(static_layout.alignment >= align_of::<Slot>());
}
