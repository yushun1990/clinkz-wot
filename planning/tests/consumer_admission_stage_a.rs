#![allow(dead_code)]

use core::cell::Cell;
use core::mem::{align_of, offset_of, size_of, ManuallyDrop};

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingArtifactFootprint, BindingCompilerBounds,
    BindingConfigurationDigest, BindingGeneration, BindingId, BindingRegistrationIdentity,
    CoreError, PlanId, PlanSetGeneration,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::{
    affordance::{ActionAffordance, EventAffordance, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    security_scheme::SecurityScheme,
    thing::Thing,
};
use std::{collections::btree_map::Iter as BTreeIter, slice::Iter as SliceIter};

/// Non-production Stage-A model only.
///
/// It intentionally proves ownership and substitution resistance without
/// creating the future public Consumer admission API.
mod stage_a {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ValidationIssue {
        kind: u16,
        location: u32,
    }

    /// Concrete fixed slot capable of holding the largest currently modeled
    /// admission failure carrier without a second allocation.
    #[repr(C)]
    pub union FailureSlot {
        validation: ManuallyDrop<ValidationIssue>,
        core: ManuallyDrop<CoreError>,
    }

    /// Existing TD storage is external. The cursor stores iterators that borrow
    /// that stable source; it never owns/moves the source it traverses.
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

        /// Representative one-item progress over every top-level borrowed
        /// iterator. Nested schema/extension frames use the same external-borrow
        /// rule in the future TD implementation; this model proves the lifetime
        /// topology rather than reimplementing Basic validation.
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
    }

    /// Opaque build authority supplied by the eventual Servient/0062 plan-set
    /// identity owner. There is intentionally no public constructor from raw
    /// PlanId or PlanSetGeneration.
    pub struct UnpublishedPlanBuildLease {
        identity: BuildIdentity,
    }

    impl UnpublishedPlanBuildLease {
        pub fn plan_id(&self) -> PlanId {
            self.identity.plan_id
        }

        pub fn plan_set_generation(&self) -> PlanSetGeneration {
            self.identity.plan_set_generation
        }
    }

    pub struct PlanSetIdentityAuthority {
        plan_set_generation: PlanSetGeneration,
        next_plan_slot: u32,
        next_plan_generation: Generation,
    }

    impl PlanSetIdentityAuthority {
        pub fn new(
            plan_set_generation: PlanSetGeneration,
            next_plan_slot: u32,
            next_plan_generation: Generation,
        ) -> Self {
            Self {
                plan_set_generation,
                next_plan_slot,
                next_plan_generation,
            }
        }

        pub fn reserve(&mut self) -> UnpublishedPlanBuildLease {
            let identity = BuildIdentity {
                plan_id: PlanId::new(
                    SlotIndex::new(self.next_plan_slot),
                    self.next_plan_generation,
                ),
                plan_set_generation: self.plan_set_generation,
            };
            self.next_plan_slot = self
                .next_plan_slot
                .checked_add(1)
                .expect("Stage-A fixture plan slots stay bounded");
            self.next_plan_generation = self
                .next_plan_generation
                .checked_next()
                .expect("Stage-A fixture generations stay bounded");
            UnpublishedPlanBuildLease { identity }
        }
    }

    #[derive(Clone, Copy)]
    pub struct CheckedPolicy {
        admission_temporary_bytes_per_operation: u64,
        largest_contiguous_allocation_bytes: u64,
        compiled_runtime_bytes_per_thing: u64,
        typed_td_admission_work_units: u64,
    }

    impl CheckedPolicy {
        pub fn fixture() -> Self {
            Self {
                admission_temporary_bytes_per_operation: 4096,
                largest_contiguous_allocation_bytes: 4096,
                compiled_runtime_bytes_per_thing: 4096,
                typed_td_admission_work_units: 1024,
            }
        }
    }

    pub struct CompleteRegistration {
        identity: BindingRegistrationIdentity,
        compiler_compatibility: BindingArtifactCompatibility,
        artifact_bytes: u64,
        cursor_bytes: u64,
        temporary_bytes: u64,
        compiler_work: u64,
        bounds_calls: Cell<u32>,
        start_calls: Cell<u32>,
    }

    impl CompleteRegistration {
        pub fn new(
            identity: BindingRegistrationIdentity,
            compiler_compatibility: BindingArtifactCompatibility,
            artifact_bytes: u64,
            cursor_bytes: u64,
            temporary_bytes: u64,
            compiler_work: u64,
        ) -> Result<Self, ()> {
            if identity.artifact_compatibility() != compiler_compatibility {
                return Err(());
            }
            Ok(Self {
                identity,
                compiler_compatibility,
                artifact_bytes,
                cursor_bytes,
                temporary_bytes,
                compiler_work,
                bounds_calls: Cell::new(0),
                start_calls: Cell::new(0),
            })
        }

        pub fn identity(&self) -> BindingRegistrationIdentity {
            self.identity
        }

        pub fn diagnostic_ordinal(&self) -> u32 {
            self.identity.diagnostic_ordinal()
        }

        fn bounds(&self) -> BindingCompilerBounds {
            self.bounds_calls.set(self.bounds_calls.get() + 1);
            BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, self.artifact_bytes),
                self.cursor_bytes,
                self.temporary_bytes,
                WorkBudget::new().with_remaining(WorkClass::BindingPolls, self.compiler_work),
            )
        }

        fn start(&self) {
            self.start_calls.set(self.start_calls.get() + 1);
        }

        pub fn bounds_calls(&self) -> u32 {
            self.bounds_calls.get()
        }

        pub fn start_calls(&self) -> u32 {
            self.start_calls.get()
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
            self.entries[ordinal].bounds_calls()
        }

        pub fn start_calls(&self, ordinal: usize) -> u32 {
            self.entries[ordinal].start_calls()
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
            let artifact = bounds.artifact();
            let cursor_bytes = bounds.cursor_bytes();
            let temporary_bytes = bounds.temporary_bytes();
            let lifetime_work = bounds.into_work();
            Self {
                artifact,
                cursor_bytes,
                temporary_bytes,
                lifetime_work,
            }
        }

        pub fn lifetime_remaining(&self, class: WorkClass) -> u64 {
            self.lifetime_work.remaining(class)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct CompilerReservation {
        temporary_bytes: u64,
        runtime_bytes: u64,
    }

    pub struct AdmissionAccounting {
        temporary_limit: u64,
        runtime_limit: u64,
        contiguous_limit: u64,
    }

    impl AdmissionAccounting {
        pub fn from_policy(policy: CheckedPolicy) -> Self {
            Self {
                temporary_limit: policy.admission_temporary_bytes_per_operation,
                runtime_limit: policy.compiled_runtime_bytes_per_thing,
                contiguous_limit: policy.largest_contiguous_allocation_bytes,
            }
        }

        pub fn with_limits(
            temporary_limit: u64,
            runtime_limit: u64,
            contiguous_limit: u64,
        ) -> Self {
            Self {
                temporary_limit,
                runtime_limit,
                contiguous_limit,
            }
        }

        fn reserve_compiler(
            &self,
            bounds: &CapturedCompilerBounds,
        ) -> Result<CompilerReservation, ()> {
            let temporary = bounds
                .cursor_bytes
                .checked_add(bounds.temporary_bytes)
                .ok_or(())?;
            let runtime = bounds.artifact.retained_bytes();
            if temporary > self.temporary_limit
                || runtime > self.runtime_limit
                || bounds.cursor_bytes > self.contiguous_limit
                || bounds.temporary_bytes > self.contiguous_limit
                || runtime > self.contiguous_limit
            {
                return Err(());
            }
            Ok(CompilerReservation {
                temporary_bytes: temporary,
                runtime_bytes: runtime,
            })
        }
    }

    struct EphemeralPlanBuildInput<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        plan_id: PlanId,
        plan_set_generation: PlanSetGeneration,
        registration_snapshot_ordinal: usize,
    }

    pub struct Planning<'td, 'reg> {
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        registration: &'reg CompleteRegistration,
        registration_snapshot_ordinal: usize,
        cancellation_generation: u64,
        policy: CheckedPolicy,
        lease: UnpublishedPlanBuildLease,
        bounds: CapturedCompilerBounds,
        _reservation: CompilerReservation,
    }

    impl<'td, 'reg> Validated<'td, 'reg> {
        pub fn enter_planning(
            self,
            lease: UnpublishedPlanBuildLease,
            accounting: AdmissionAccounting,
        ) -> Result<Planning<'td, 'reg>, ()> {
            let registration = self.snapshot.get(self.snapshot_ordinal).ok_or(())?;

            // Full identity is sourced from this one complete registration.
            // No separate BindingRegistrationIdentity/compiler argument exists.
            if registration.identity().artifact_compatibility()
                != registration.compiler_compatibility
            {
                return Err(());
            }

            // Exact bounds are obtained once and all memory is admitted before start.
            let bounds = CapturedCompilerBounds::capture(registration.bounds());
            let reservation = accounting.reserve_compiler(&bounds)?;
            registration.start();

            Ok(Planning {
                source: self.source,
                snapshot: self.snapshot,
                registration,
                registration_snapshot_ordinal: self.snapshot_ordinal,
                cancellation_generation: self.cancellation_generation,
                policy: self.policy,
                lease,
                bounds,
                _reservation: reservation,
            })
        }
    }

    impl Planning<'_, '_> {
        fn ephemeral_input(&self) -> EphemeralPlanBuildInput<'_, '_> {
            EphemeralPlanBuildInput {
                source: self.source,
                snapshot: self.snapshot,
                plan_id: self.lease.plan_id(),
                plan_set_generation: self.lease.plan_set_generation(),
                registration_snapshot_ordinal: self.registration_snapshot_ordinal,
            }
        }

        pub fn selected_identity(&self) -> BindingRegistrationIdentity {
            self.registration.identity()
        }

        pub fn selected_snapshot_ordinal(&self) -> usize {
            self.registration_snapshot_ordinal
        }

        pub fn plan_id(&self) -> PlanId {
            self.ephemeral_input().plan_id
        }

        pub fn plan_set_generation(&self) -> PlanSetGeneration {
            self.ephemeral_input().plan_set_generation
        }

        pub fn consume_compiler_work(
            &mut self,
            step: &mut WorkBudget,
            class: WorkClass,
            units: u64,
        ) -> Result<(), ()> {
            consume_work_pair(&mut self.bounds.lifetime_work, step, class, units)
        }

        pub fn compiler_lifetime_remaining(&self, class: WorkClass) -> u64 {
            self.bounds.lifetime_remaining(class)
        }
    }

    /// Stage-A model for the Foundation pair-commit primitive.
    pub fn consume_work_pair(
        lifetime: &mut WorkBudget,
        step: &mut WorkBudget,
        class: WorkClass,
        units: u64,
    ) -> Result<(), ()> {
        if lifetime.remaining(class) < units || step.remaining(class) < units {
            return Err(());
        }

        // Both preflights succeeded under unique mutable ownership; these two
        // commits are now non-failing for the observed counters.
        lifetime
            .consume(class, units)
            .expect("lifetime preflight must make commit infallible");
        step.consume(class, units)
            .expect("step preflight must make commit infallible");
        Ok(())
    }

    /// Proposed append-only typed-admission work domain. Production Foundation
    /// migration adds the equivalent WorkClass only after 0063 is accepted.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum TypedTdWorkClass {
        TypedTdAdmissionItems,
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
        cursor_bytes: u64,
        temporary_bytes: u64,
        artifact_bytes: u64,
        lifetime_work: WorkBudget,
    }

    /// Concrete Stage-A Host layout model.
    #[repr(C)]
    pub struct HostAdmissionStorage<'td, 'reg> {
        tag: u8,
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        cancellation_generation: u64,
        state_words: [u64; 12],
        failure: FailureSlot,
        accounting: AccountingStorage,
        compiler: CompilerStorage,
    }

    /// Concrete Stage-A application-static layout model.
    #[repr(C)]
    pub struct StaticAdmissionStorage<'td, 'reg> {
        tag: u8,
        source: &'td Thing,
        snapshot: &'reg RegistrationSnapshot,
        cancellation_generation: u64,
        state_words: [u64; 8],
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
        let state = offset_of!(Storage, state_words);
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
        let state = offset_of!(Storage, state_words);
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
        CompleteRegistration::new(identity, compatibility, 64, 32, 48, 3)
            .expect("fixture registration must be self-consistent")
    }
}

#[test]
fn borrowed_td_cursor_is_constructible_without_source_ownership() {
    let thing = Thing::default();
    let mut cursor = stage_a::BorrowedTdCursor::new(&thing);
    assert_eq!(cursor.progress_once(), 0);
}

#[test]
fn build_lease_binds_plan_id_and_plan_set_generation_together() {
    let plan_set_generation =
        PlanSetGeneration::new(Generation::new(9).expect("nonzero generation"));
    let mut authority = stage_a::PlanSetIdentityAuthority::new(
        plan_set_generation,
        4,
        Generation::new(7).expect("nonzero generation"),
    );
    let lease = authority.reserve();

    assert_eq!(lease.plan_id().slot(), SlotIndex::new(4));
    assert_eq!(lease.plan_id().generation().get(), 7);
    assert_eq!(lease.plan_set_generation(), plan_set_generation);
}

#[test]
fn ordinal_domains_remain_distinct_and_same_entry_derives_compiler_identity() {
    let compatibility = 5;
    let registrations = vec![
        stage_a::registration(10, 1, 10, compatibility, 101),
        stage_a::registration(11, 2, 11, compatibility, 102),
        stage_a::registration(12, 3, 12, compatibility, 103),
        stage_a::registration(13, 4, 13, compatibility, 17),
    ];
    // Make the equal-compatibility competing identity explicit.
    assert_ne!(
        registrations[0].identity().binding_id(),
        registrations[3].identity().binding_id()
    );

    let snapshot = stage_a::RegistrationSnapshot::new(registrations);
    let thing = Thing::default();
    let validating = stage_a::Validating::new(
        &thing,
        &snapshot,
        3,
        1,
        stage_a::CheckedPolicy::fixture(),
    );
    let validated = validating.validated();

    let mut identity_authority = stage_a::PlanSetIdentityAuthority::new(
        PlanSetGeneration::new(Generation::new(8).expect("nonzero generation")),
        2,
        Generation::new(6).expect("nonzero generation"),
    );
    let lease = identity_authority.reserve();
    let accounting = stage_a::AdmissionAccounting::from_policy(stage_a::CheckedPolicy::fixture());
    let planning = validated
        .enter_planning(lease, accounting)
        .expect("same-entry construction must succeed");

    assert_eq!(planning.selected_snapshot_ordinal(), 3);
    assert_eq!(planning.selected_identity().diagnostic_ordinal(), 17);
    assert_eq!(planning.selected_identity().binding_id(), BindingId::new(13));
    assert_eq!(snapshot.bounds_calls(0), 0);
    assert_eq!(snapshot.bounds_calls(3), 1);
    assert_eq!(snapshot.start_calls(0), 0);
    assert_eq!(snapshot.start_calls(3), 1);
}

#[test]
fn compiler_bounds_are_reserved_before_start_and_owned_after_entry() {
    let registration = stage_a::registration(20, 1, 1, 1, 7);
    let snapshot = stage_a::RegistrationSnapshot::new(vec![registration]);
    let thing = Thing::default();

    let rejected = stage_a::Validating::new(
        &thing,
        &snapshot,
        0,
        1,
        stage_a::CheckedPolicy::fixture(),
    )
    .validated();
    let mut rejected_identity_authority = stage_a::PlanSetIdentityAuthority::new(
        PlanSetGeneration::new(Generation::new(4).expect("nonzero generation")),
        0,
        Generation::new(2).expect("nonzero generation"),
    );
    assert!(
        rejected
            .enter_planning(
                rejected_identity_authority.reserve(),
                stage_a::AdmissionAccounting::with_limits(16, 4096, 4096),
            )
            .is_err()
    );
    assert_eq!(snapshot.bounds_calls(0), 1);
    assert_eq!(snapshot.start_calls(0), 0);

    let validated = stage_a::Validating::new(
        &thing,
        &snapshot,
        0,
        1,
        stage_a::CheckedPolicy::fixture(),
    )
    .validated();

    let mut identity_authority = stage_a::PlanSetIdentityAuthority::new(
        PlanSetGeneration::new(Generation::new(5).expect("nonzero generation")),
        1,
        Generation::new(3).expect("nonzero generation"),
    );
    let planning = validated
        .enter_planning(
            identity_authority.reserve(),
            stage_a::AdmissionAccounting::from_policy(stage_a::CheckedPolicy::fixture()),
        )
        .expect("bounds fit fixture accounting");

    assert_eq!(snapshot.bounds_calls(0), 2);
    assert_eq!(snapshot.start_calls(0), 1);
    assert_eq!(planning.compiler_lifetime_remaining(WorkClass::BindingPolls), 3);
}

#[test]
fn compiler_pair_charge_is_atomic_and_step_replenishment_cannot_reset_lifetime() {
    let mut lifetime = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    let mut empty_step = WorkBudget::new();

    assert!(stage_a::consume_work_pair(
        &mut lifetime,
        &mut empty_step,
        WorkClass::BindingPolls,
        1
    )
    .is_err());
    assert_eq!(lifetime.remaining(WorkClass::BindingPolls), 1);
    assert_eq!(empty_step.remaining(WorkClass::BindingPolls), 0);

    let mut funded_step = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    stage_a::consume_work_pair(
        &mut lifetime,
        &mut funded_step,
        WorkClass::BindingPolls,
        1,
    )
    .expect("joint preflight succeeds");
    assert_eq!(lifetime.remaining(WorkClass::BindingPolls), 0);
    assert_eq!(funded_step.remaining(WorkClass::BindingPolls), 0);

    let mut replenished = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
    assert!(stage_a::consume_work_pair(
        &mut lifetime,
        &mut replenished,
        WorkClass::BindingPolls,
        1
    )
    .is_err());
    assert_eq!(replenished.remaining(WorkClass::BindingPolls), 1);
}

#[test]
fn typed_td_pair_charge_has_the_same_failure_atomicity() {
    let mut lifetime = stage_a::TypedTdWorkMeter::new(1);
    let mut empty_step = stage_a::TypedTdWorkMeter::new(0);
    assert!(stage_a::consume_typed_td_pair(&mut lifetime, &mut empty_step, 1).is_err());
    assert_eq!(lifetime.remaining(), 1);
    assert_eq!(empty_step.remaining(), 0);

    let mut step = stage_a::TypedTdWorkMeter::new(1);
    stage_a::consume_typed_td_pair(&mut lifetime, &mut step, 1)
        .expect("joint typed-TD charge succeeds");
    assert_eq!(lifetime.remaining(), 0);
    assert_eq!(step.remaining(), 0);
}

#[test]
fn host_and_static_layouts_cover_one_enclosing_allocation_without_overlap() {
    let host = stage_a::host_layout();
    let static_layout = stage_a::static_layout();

    assert!(host.total_size >= size_of::<stage_a::FailureSlot>());
    assert!(static_layout.total_size >= size_of::<stage_a::FailureSlot>());
    assert!(size_of::<stage_a::FailureSlot>() >= size_of::<CoreError>());
    assert!(size_of::<stage_a::FailureSlot>() >= size_of::<stage_a::ValidationIssue>());
    assert!(host.diagnostic.1 - host.diagnostic.0 >= size_of::<stage_a::FailureSlot>());
    assert!(
        static_layout.diagnostic.1 - static_layout.diagnostic.0
            >= size_of::<stage_a::FailureSlot>()
    );
    assert!(host.alignment >= align_of::<stage_a::FailureSlot>());
    assert!(static_layout.alignment >= align_of::<stage_a::FailureSlot>());
}
