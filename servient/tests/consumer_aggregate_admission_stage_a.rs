#![allow(dead_code)]

//! Non-production constructibility evidence for workspace/0063.
//!
//! The fixture deliberately composes current public TD, identity, candidate,
//! compiler, artifact, and work-budget values. It does not pre-admit a
//! production aggregate API or WP-400 implementation.

use std::{
    cell::{Cell, RefCell},
    mem::size_of,
    rc::Rc,
};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactEnvelope, BindingArtifactFootprint, BindingArtifactIdentity,
    BindingArtifactRef, BindingArtifactRole, BindingCandidate, BindingCompilerBounds,
    BindingCompilerExtension, BindingCompilerFailure, BindingCompilerInput, BindingCompilerOutput,
    BindingCompilerStep, BindingConfigurationDigest, BindingGeneration, BindingId,
    BindingRegistrationCapabilities, BindingRegistrationIdentity, LogicalInteractionPlan, PlanId,
    PlanSetGeneration, ThingId,
};
use clinkz_wot_foundation::{SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    data_type::{Operation, resolve_form_href},
    form::Form,
    security_scheme::SecurityScheme,
    td_defaults::{FormContext, effective_form_operations, effective_form_security},
    thing::Thing,
    validate::Validate,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceEvent {
    SourceTransferred,
    EnumerationReserved,
    Enumerated,
    ShapeReserved,
    PlanMaterialized(PlanId),
    BoundsObserved(PlanId),
    EnumerationReleased,
    CompilerReserved,
    StartObserved(PlanId),
    StepObserved(PlanId),
    CursorAborted(PlanId),
    CompilerTemporaryReleased,
    SourceReleased,
    Frozen,
    FailedSettled,
    Reclaimed,
}

#[derive(Clone, Default)]
struct Trace(Rc<RefCell<Vec<TraceEvent>>>);

impl Trace {
    fn push(&self, event: TraceEvent) {
        self.0.borrow_mut().push(event);
    }

    fn snapshot(&self) -> Vec<TraceEvent> {
        self.0.borrow().clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InputObservation {
    plan_id: PlanId,
    plan_address: usize,
    property_name: String,
    form_index: u32,
    resolved_target: String,
    candidate: BindingCandidate,
}

impl InputObservation {
    fn capture(input: &BindingCompilerInput<'_>) -> Self {
        Self {
            plan_id: input.logical_plan().plan_id(),
            plan_address: input.logical_plan() as *const LogicalInteractionPlan as usize,
            property_name: input.logical_plan().property_name().to_owned(),
            form_index: input.logical_plan().form_index(),
            resolved_target: input.logical_plan().resolved_target().to_owned(),
            candidate: input.candidate(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixtureCursor {
    plan_id: PlanId,
    progress: u8,
}

struct FixtureCompiler {
    compatibility: clinkz_wot_core::BindingArtifactCompatibility,
    fail_slot: Cell<Option<u32>>,
    bounds: RefCell<Vec<InputObservation>>,
    starts: RefCell<Vec<InputObservation>>,
    steps: RefCell<Vec<InputObservation>>,
    aborts: Cell<u32>,
    artifact_drops: Rc<Cell<u32>>,
    trace: Trace,
}

impl FixtureCompiler {
    fn new(trace: Trace) -> Self {
        Self {
            compatibility: clinkz_wot_core::BindingArtifactCompatibility::new([0x63; 16]),
            fail_slot: Cell::new(None),
            bounds: RefCell::new(Vec::new()),
            starts: RefCell::new(Vec::new()),
            steps: RefCell::new(Vec::new()),
            aborts: Cell::new(0),
            artifact_drops: Rc::new(Cell::new(0)),
            trace,
        }
    }
}

impl BindingCompilerExtension for FixtureCompiler {
    type Cursor = FixtureCursor;
    type Artifact = FixtureArtifact;

    fn compatibility(&self) -> clinkz_wot_core::BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> Result<BindingCompilerBounds, clinkz_wot_core::CoreError> {
        let observation = InputObservation::capture(input);
        self.trace
            .push(TraceEvent::BoundsObserved(observation.plan_id));
        self.bounds.borrow_mut().push(observation);
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, 64),
            size_of::<FixtureCursor>() as u64,
            32,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2),
        ))
    }

    fn start(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> Result<Self::Cursor, clinkz_wot_core::CoreError> {
        let observation = InputObservation::capture(input);
        self.trace
            .push(TraceEvent::StartObserved(observation.plan_id));
        self.starts.borrow_mut().push(observation);
        Ok(FixtureCursor {
            plan_id: input.logical_plan().plan_id(),
            progress: 0,
        })
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
        let observation = InputObservation::capture(input);
        self.trace
            .push(TraceEvent::StepObserved(observation.plan_id));
        self.steps.borrow_mut().push(observation);
        cursor.progress += 1;
        if cursor.progress == 1 {
            return BindingCompilerStep::Pending(cursor);
        }
        if self.fail_slot.get() == Some(input.logical_plan().plan_id().slot().get()) {
            let error = clinkz_wot_core::CoreError::Validation(
                clinkz_wot_core::ErrorContext::new(
                    clinkz_wot_core::ErrorPhase::Binding,
                    clinkz_wot_core::RetryClass::Never,
                )
                .with_operation(Operation::ReadProperty)
                .with_plan(input.logical_plan().plan_id()),
            );
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(error, cursor));
        }
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            self.compatibility,
            BindingArtifactFootprint::new(1, 64),
            FixtureArtifact {
                form_index: input.logical_plan().form_index(),
                drops: self.artifact_drops.clone(),
            },
        )))
    }

    fn abort(&self, cursor: Self::Cursor) {
        self.aborts.set(self.aborts.get() + 1);
        self.trace.push(TraceEvent::CursorAborted(cursor.plan_id));
    }
}

#[derive(Debug)]
struct FixtureArtifact {
    form_index: u32,
    drops: Rc<Cell<u32>>,
}

impl Drop for FixtureArtifact {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

struct Registration {
    identity: BindingRegistrationIdentity,
    capabilities: BindingRegistrationCapabilities,
    compiler: Rc<FixtureCompiler>,
    client_name: &'static str,
}

impl Registration {
    fn matches(&self, candidate: BindingCandidate) -> bool {
        self.identity.binding_id() == candidate.binding_id()
            && self.identity.binding_generation() == candidate.binding_generation()
            && self.identity.configuration() == candidate.configuration()
            && self.identity.artifact_compatibility() == candidate.compatibility()
    }
}

struct RegistrationSnapshot {
    registrations: Vec<Registration>,
}

impl RegistrationSnapshot {
    fn first_proof(consumer_compiler: Rc<FixtureCompiler>) -> Self {
        let producer_compiler = Rc::new(FixtureCompiler::new(consumer_compiler.trace.clone()));
        Self {
            registrations: vec![
                registration(
                    3,
                    80,
                    BindingRegistrationCapabilities::producer_property_read(),
                    producer_compiler,
                    "producer-only",
                ),
                registration(
                    9,
                    17,
                    BindingRegistrationCapabilities::producer_and_consumer_property_read(),
                    consumer_compiler,
                    "consumer-client",
                ),
            ],
        }
    }

    fn producer_only(trace: Trace) -> Self {
        Self {
            registrations: vec![registration(
                3,
                80,
                BindingRegistrationCapabilities::producer_property_read(),
                Rc::new(FixtureCompiler::new(trace)),
                "producer-only",
            )],
        }
    }

    fn with_second_consumer(mut self) -> Self {
        let trace = self.registrations[1].compiler.trace.clone();
        self.registrations.push(registration(
            12,
            41,
            BindingRegistrationCapabilities::producer_and_consumer_property_read(),
            Rc::new(FixtureCompiler::new(trace)),
            "other-consumer-client",
        ));
        self
    }

    fn select_first_proof_consumer(&self) -> Result<usize, AdmissionCause> {
        let mut eligible = self
            .registrations
            .iter()
            .enumerate()
            .filter(|(_, registration)| {
                registration.capabilities.supports_consumer_property_read()
            });
        let Some((ordinal, _)) = eligible.next() else {
            return Err(AdmissionCause::NoConsumerRegistration);
        };
        if eligible.next().is_some() {
            return Err(AdmissionCause::AmbiguousFirstProofRegistrations);
        }
        Ok(ordinal)
    }
}

fn registration(
    id: u32,
    diagnostic_ordinal: u32,
    capabilities: BindingRegistrationCapabilities,
    compiler: Rc<FixtureCompiler>,
    client_name: &'static str,
) -> Registration {
    Registration {
        identity: BindingRegistrationIdentity::new(
            BindingId::new(id),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([id as u8; 32]),
            compiler.compatibility(),
            diagnostic_ordinal,
        ),
        capabilities,
        compiler,
        client_name,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TypedCensus {
    properties: u32,
    forms: u32,
    owned_text_bytes: u64,
    admitted_source_bytes: u64,
}

struct ValidatedConsumerThing {
    thing: Thing,
    census: TypedCensus,
}

impl ValidatedConsumerThing {
    fn new(thing: Thing) -> Result<Self, String> {
        thing.validate().map_err(|error| error.to_string())?;
        let properties = thing.properties.as_ref().map_or(0, |items| items.len());
        let forms = thing.properties.as_ref().map_or(0, |items| {
            items
                .values()
                .map(|property| property._interaction.forms.len())
                .sum()
        });
        let owned_text_bytes = thing.id.as_ref().map_or(0, |id| id.as_str().len())
            + thing.properties.as_ref().map_or(0, |items| {
                items
                    .iter()
                    .map(|(name, property)| {
                        name.len()
                            + property
                                ._interaction
                                .forms
                                .iter()
                                .map(|form| {
                                    form.href.as_str().len()
                                        + form.content_type.len()
                                        + form.subprotocol.as_deref().map_or(0, str::len)
                                })
                                .sum::<usize>()
                    })
                    .sum::<usize>()
            });
        let admitted_source_bytes = size_of::<Thing>()
            .checked_add(properties * size_of::<PropertyAffordance>())
            .and_then(|value| value.checked_add(forms * size_of::<Form>()))
            .and_then(|value| value.checked_add(owned_text_bytes))
            .ok_or_else(|| "fixture typed census overflow".to_owned())?;
        Ok(Self {
            thing,
            census: TypedCensus {
                properties: u32::try_from(properties)
                    .map_err(|_| "fixture property census overflow".to_owned())?,
                forms: u32::try_from(forms)
                    .map_err(|_| "fixture form census overflow".to_owned())?,
                owned_text_bytes: owned_text_bytes as u64,
                admitted_source_bytes: admitted_source_bytes as u64,
            },
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct AdmissionAccounts {
    retained_source: u64,
    temporary_build: u64,
    persistent_effective_document: u64,
    persistent_runtime_reserved: u64,
    persistent_runtime_committed: u64,
    diagnostic_reserved: u64,
    diagnostic_committed: u64,
    cleanup_state: u64,
    peak_live: u64,
    largest_allocation: u64,
}

impl AdmissionAccounts {
    fn live(self) -> u64 {
        self.retained_source
            + self.temporary_build
            + self.persistent_effective_document
            + self.persistent_runtime_reserved
            + self.persistent_runtime_committed
            + self.diagnostic_reserved
            + self.diagnostic_committed
            + self.cleanup_state
    }

    fn settled_zero(self) -> bool {
        self.live() == 0
    }
}

#[derive(Default)]
struct ResourceState {
    accounts: AdmissionAccounts,
    building: bool,
    frozen: bool,
}

#[derive(Clone)]
struct ResourceArena {
    state: Rc<RefCell<ResourceState>>,
    trace: Trace,
}

impl ResourceArena {
    fn new(trace: Trace) -> Self {
        Self {
            state: Rc::new(RefCell::new(ResourceState::default())),
            trace,
        }
    }

    fn transfer_source(&self, bytes: u64) -> ResourceBuildLease {
        assert!(bytes > 0);
        let mut state = self.state.borrow_mut();
        assert!(!state.building && !state.frozen && state.accounts.settled_zero());
        state.building = true;
        state.accounts.retained_source = bytes;
        update_peaks(&mut state.accounts, bytes);
        drop(state);
        self.trace.push(TraceEvent::SourceTransferred);
        ResourceBuildLease {
            arena: self.clone(),
            active: true,
        }
    }

    fn snapshot(&self) -> AdmissionAccounts {
        self.state.borrow().accounts
    }
}

fn update_peaks(accounts: &mut AdmissionAccounts, allocation: u64) {
    accounts.largest_allocation = accounts.largest_allocation.max(allocation);
    accounts.peak_live = accounts.peak_live.max(accounts.live());
}

struct ResourceBuildLease {
    arena: ResourceArena,
    active: bool,
}

impl ResourceBuildLease {
    fn reserve_enumeration(&mut self, temporary_bytes: u64) {
        let mut state = self.arena.state.borrow_mut();
        state.accounts.temporary_build = temporary_bytes;
        update_peaks(&mut state.accounts, temporary_bytes);
        drop(state);
        self.arena.trace.push(TraceEvent::EnumerationReserved);
    }

    fn enumerated(&self) {
        self.arena.trace.push(TraceEvent::Enumerated);
    }

    fn reserve_shape(&mut self, runtime_bytes: u64, diagnostic_bytes: u64) {
        let mut state = self.arena.state.borrow_mut();
        state.accounts.persistent_runtime_reserved = runtime_bytes;
        state.accounts.diagnostic_reserved = diagnostic_bytes;
        update_peaks(&mut state.accounts, runtime_bytes.max(diagnostic_bytes));
        drop(state);
        self.arena.trace.push(TraceEvent::ShapeReserved);
    }

    fn release_enumeration(&mut self) {
        self.arena.state.borrow_mut().accounts.temporary_build = 0;
        self.arena.trace.push(TraceEvent::EnumerationReleased);
    }

    fn reserve_compiler(&mut self, artifact_bytes: u64, temporary_bytes: u64) {
        let mut state = self.arena.state.borrow_mut();
        state.accounts.persistent_runtime_reserved += artifact_bytes;
        state.accounts.temporary_build = temporary_bytes;
        update_peaks(&mut state.accounts, artifact_bytes.max(temporary_bytes));
        drop(state);
        self.arena.trace.push(TraceEvent::CompilerReserved);
    }

    fn release_compiler_temporary(&mut self) {
        self.arena.state.borrow_mut().accounts.temporary_build = 0;
        self.arena.trace.push(TraceEvent::CompilerTemporaryReleased);
    }

    fn release_source(&mut self) {
        self.arena.state.borrow_mut().accounts.retained_source = 0;
        self.arena.trace.push(TraceEvent::SourceReleased);
    }

    fn fail(mut self) {
        self.settle_failed();
    }

    fn settle_failed(&mut self) {
        if !self.active {
            return;
        }
        let mut state = self.arena.state.borrow_mut();
        state.accounts = AdmissionAccounts::default();
        state.building = false;
        state.frozen = false;
        drop(state);
        self.active = false;
        self.arena.trace.push(TraceEvent::FailedSettled);
    }

    fn commit(mut self, measured_runtime: u64, measured_diagnostic: u64) -> FrozenResourceLease {
        let mut state = self.arena.state.borrow_mut();
        assert_eq!(state.accounts.retained_source, 0);
        assert_eq!(state.accounts.temporary_build, 0);
        assert_eq!(state.accounts.persistent_effective_document, 0);
        assert!(measured_runtime <= state.accounts.persistent_runtime_reserved);
        assert!(measured_diagnostic <= state.accounts.diagnostic_reserved);
        state.accounts.persistent_runtime_reserved = 0;
        state.accounts.diagnostic_reserved = 0;
        state.accounts.persistent_runtime_committed = measured_runtime;
        state.accounts.diagnostic_committed = measured_diagnostic;
        state.building = false;
        state.frozen = true;
        drop(state);
        self.active = false;
        self.arena.trace.push(TraceEvent::Frozen);
        FrozenResourceLease {
            arena: self.arena.clone(),
            active: true,
        }
    }
}

impl Drop for ResourceBuildLease {
    fn drop(&mut self) {
        self.settle_failed();
    }
}

struct FrozenResourceLease {
    arena: ResourceArena,
    active: bool,
}

impl FrozenResourceLease {
    fn reclaim(mut self) {
        self.reclaim_in_place();
    }

    fn reclaim_in_place(&mut self) {
        if !self.active {
            return;
        }
        let mut state = self.arena.state.borrow_mut();
        assert!(state.frozen);
        state.accounts = AdmissionAccounts::default();
        state.frozen = false;
        drop(state);
        self.active = false;
        self.arena.trace.push(TraceEvent::Reclaimed);
    }
}

impl Drop for FrozenResourceLease {
    fn drop(&mut self) {
        self.reclaim_in_place();
    }
}

#[derive(Default)]
struct PlanSetState {
    next_generation: PlanSetGeneration,
    unpublished: bool,
    frozen: bool,
}

#[derive(Clone, Default)]
struct PlanSetArena(Rc<RefCell<PlanSetState>>);

impl PlanSetArena {
    fn reserve(&self) -> PlanSetBuildLease {
        let mut state = self.0.borrow_mut();
        assert!(!state.unpublished && !state.frozen);
        state.unpublished = true;
        let generation = state.next_generation;
        drop(state);
        PlanSetBuildLease {
            arena: self.clone(),
            generation,
            active: true,
        }
    }

    fn next_generation(&self) -> PlanSetGeneration {
        self.0.borrow().next_generation
    }
}

struct PlanSetBuildLease {
    arena: PlanSetArena,
    generation: PlanSetGeneration,
    active: bool,
}

impl PlanSetBuildLease {
    fn plan_id(&self, slot: u32) -> PlanId {
        PlanId::new(SlotIndex::new(slot), self.generation.get())
    }

    fn abort(mut self) {
        self.abort_in_place();
    }

    fn abort_in_place(&mut self) {
        if !self.active {
            return;
        }
        let mut state = self.arena.0.borrow_mut();
        assert!(state.unpublished);
        state.unpublished = false;
        state.next_generation = state
            .next_generation
            .checked_next()
            .expect("fixture generation cannot wrap");
        self.active = false;
    }

    fn commit(mut self) -> FrozenGenerationLease {
        let mut state = self.arena.0.borrow_mut();
        assert!(state.unpublished && !state.frozen);
        state.unpublished = false;
        state.frozen = true;
        drop(state);
        self.active = false;
        FrozenGenerationLease {
            arena: self.arena.clone(),
            generation: self.generation,
            active: true,
        }
    }
}

impl Drop for PlanSetBuildLease {
    fn drop(&mut self) {
        self.abort_in_place();
    }
}

struct FrozenGenerationLease {
    arena: PlanSetArena,
    generation: PlanSetGeneration,
    active: bool,
}

impl FrozenGenerationLease {
    fn reclaim(mut self) {
        self.reclaim_in_place();
    }

    fn reclaim_in_place(&mut self) {
        if !self.active {
            return;
        }
        let mut state = self.arena.0.borrow_mut();
        assert!(state.frozen);
        state.frozen = false;
        state.next_generation = state
            .next_generation
            .checked_next()
            .expect("fixture generation cannot wrap");
        self.active = false;
    }
}

impl Drop for FrozenGenerationLease {
    fn drop(&mut self) {
        self.reclaim_in_place();
    }
}

struct CoordinateSeed<'a> {
    target_slot: u32,
    property_name: &'a str,
    form_index: u32,
    resolved_target: String,
    content_type: &'a str,
    subprotocol: Option<&'a str>,
}

struct TargetSeed<'a> {
    property_name: &'a str,
    first_join: u32,
    join_count: u32,
}

struct Enumeration<'a> {
    targets: Vec<TargetSeed<'a>>,
    coordinates: Vec<CoordinateSeed<'a>>,
}

fn enumerate(thing: &Thing) -> Result<Enumeration<'_>, AdmissionCause> {
    let mut targets = Vec::new();
    let mut coordinates = Vec::new();
    for (property_name, property) in thing
        .properties
        .as_ref()
        .expect("validated fixture properties")
    {
        let first_join = u32::try_from(coordinates.len()).expect("fixture coordinate bound");
        for (form_index, form) in property._interaction.forms.iter().enumerate() {
            if !effective_form_operations(FormContext::Property(property), form)
                .contains(&Operation::ReadProperty)
            {
                continue;
            }
            let [security_name] = effective_form_security(thing, form) else {
                return Err(AdmissionCause::UnsupportedFirstProofSecurity);
            };
            if !matches!(
                thing.security_definitions.get(security_name),
                Some(SecurityScheme::NoSec(_))
            ) {
                return Err(AdmissionCause::UnsupportedFirstProofSecurity);
            }
            let resolved = resolve_form_href(thing.base.as_ref(), &form.href)
                .expect("validated fixture target resolves");
            coordinates.push(CoordinateSeed {
                target_slot: u32::try_from(targets.len()).expect("fixture target bound"),
                property_name,
                form_index: u32::try_from(form_index).expect("fixture form bound"),
                resolved_target: resolved.as_str().to_owned(),
                content_type: form.content_type.as_str(),
                subprotocol: form.subprotocol.as_deref(),
            });
        }
        targets.push(TargetSeed {
            property_name,
            first_join,
            join_count: u32::try_from(coordinates.len()).expect("fixture coordinate bound")
                - first_join,
        });
    }
    Ok(Enumeration {
        targets,
        coordinates,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TargetProjection {
    property_name: Box<str>,
    first_join: u32,
    join_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BindingPlanRefEquivalent {
    target_slot: u32,
    plan_slot: u32,
    candidate_slot: u32,
    artifact_slot: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AggregateDiagnostic {
    snapshot_ordinal: u32,
    registration_diagnostic_ordinal: u32,
    target_count: u32,
    readable_coordinate_count: u32,
}

struct MaterializedCoordinate {
    target_slot: u32,
    plan: Box<LogicalInteractionPlan>,
    candidate: BindingCandidate,
}

struct BoundedCoordinate {
    target_slot: u32,
    plan: Box<LogicalInteractionPlan>,
    candidate: BindingCandidate,
    admitted: BindingArtifactFootprint,
    work: WorkBudget,
}

struct AggregateDraft {
    plans: Vec<Box<LogicalInteractionPlan>>,
    candidates: Vec<BindingCandidate>,
    artifacts: Vec<BindingArtifactEnvelope<FixtureArtifact>>,
    artifact_refs: Vec<BindingArtifactRef>,
    runtime_joins: Vec<BindingPlanRefEquivalent>,
    targets: Vec<TargetProjection>,
    diagnostic: AggregateDiagnostic,
}

impl AggregateDraft {
    fn measured_runtime_bytes(&self) -> u64 {
        let plans: usize = self
            .plans
            .iter()
            .map(|plan| {
                size_of::<LogicalInteractionPlan>()
                    + plan.thing_id().as_str().len()
                    + plan.property_name().len()
                    + plan.resolved_target().len()
                    + plan.content_type().map_or(0, str::len)
                    + plan.subprotocol().map_or(0, str::len)
            })
            .sum();
        let targets: usize = self
            .targets
            .iter()
            .map(|target| size_of::<TargetProjection>() + target.property_name.len())
            .sum();
        let artifacts: u64 = self
            .artifacts
            .iter()
            .map(|artifact| {
                size_of::<BindingArtifactEnvelope<FixtureArtifact>>() as u64
                    + artifact.artifact().footprint().retained_bytes()
            })
            .sum();
        plans as u64
            + targets as u64
            + artifacts
            + (self.candidates.len() * size_of::<BindingCandidate>()) as u64
            + (self.artifact_refs.len() * size_of::<BindingArtifactRef>()) as u64
            + (self.runtime_joins.len() * size_of::<BindingPlanRefEquivalent>()) as u64
    }

    fn validate(&self, generation: PlanSetGeneration, snapshot: &RegistrationSnapshot) {
        let count = self.plans.len();
        assert_eq!(count, self.candidates.len());
        assert_eq!(count, self.artifacts.len());
        assert_eq!(count, self.artifact_refs.len());
        assert_eq!(count, self.runtime_joins.len());
        assert_eq!(self.diagnostic.target_count as usize, self.targets.len());
        assert_eq!(self.diagnostic.readable_coordinate_count as usize, count);
        for (slot, plan) in self.plans.iter().enumerate() {
            assert_eq!(plan.plan_id().slot(), SlotIndex::new(slot as u32));
            assert_eq!(plan.plan_id().generation(), generation.get());
        }
        for join in &self.runtime_joins {
            let plan = &self.plans[join.plan_slot as usize];
            let candidate = self.candidates[join.candidate_slot as usize];
            let artifact = &self.artifacts[join.artifact_slot as usize];
            let artifact_ref = self.artifact_refs[join.artifact_slot as usize];
            let registration = &snapshot.registrations[candidate.registration_ordinal() as usize];
            assert!(registration.matches(candidate));
            assert_eq!(artifact.identity().plan_set_generation(), generation);
            assert_eq!(artifact.identity().plan_id(), plan.plan_id());
            assert_eq!(artifact.identity().binding_id(), candidate.binding_id());
            assert_eq!(
                artifact.identity().binding_generation(),
                candidate.binding_generation()
            );
            assert_eq!(
                artifact.identity().configuration(),
                candidate.configuration()
            );
            assert_eq!(
                artifact.identity().compatibility(),
                candidate.compatibility()
            );
            assert_eq!(
                artifact.identity().role(),
                BindingArtifactRole::ConsumerCall
            );
            assert_eq!(artifact.artifact().payload().form_index, plan.form_index());
            assert_eq!(artifact_ref.identity(), artifact.identity());
            assert_eq!(
                artifact_ref.artifact_slot(),
                SlotIndex::new(join.artifact_slot)
            );
        }
        let mut expected_first_join = 0u32;
        for (target_slot, target) in self.targets.iter().enumerate() {
            assert_eq!(target.first_join, expected_first_join);
            let end = target
                .first_join
                .checked_add(target.join_count)
                .expect("fixture target range");
            assert!(end as usize <= self.runtime_joins.len());
            for join in &self.runtime_joins[target.first_join as usize..end as usize] {
                assert_eq!(join.target_slot, target_slot as u32);
                assert_eq!(
                    self.plans[join.plan_slot as usize].property_name(),
                    target.property_name.as_ref()
                );
            }
            expected_first_join = end;
        }
        assert_eq!(expected_first_join as usize, self.runtime_joins.len());
    }

    fn target(&self, name: &str) -> Option<&TargetProjection> {
        self.targets
            .iter()
            .find(|target| target.property_name.as_ref() == name)
    }
}

struct FrozenAggregate {
    generation: FrozenGenerationLease,
    resources: FrozenResourceLease,
    snapshot: Rc<RegistrationSnapshot>,
    draft: AggregateDraft,
}

impl FrozenAggregate {
    fn resolve_client(&self, property_name: &str) -> Option<&'static str> {
        let plan_slot = self
            .draft
            .plans
            .iter()
            .position(|plan| plan.property_name() == property_name)?;
        let join = self
            .draft
            .runtime_joins
            .iter()
            .find(|join| join.plan_slot as usize == plan_slot)?;
        let candidate = self.draft.candidates[join.candidate_slot as usize];
        let registration = self
            .snapshot
            .registrations
            .get(candidate.registration_ordinal() as usize)?;
        registration
            .matches(candidate)
            .then_some(registration.client_name)
    }

    fn reclaim(self) {
        let Self {
            generation,
            resources,
            snapshot: _,
            draft: _,
        } = self;
        resources.reclaim();
        generation.reclaim();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdmissionCause {
    NoConsumerRegistration,
    AmbiguousFirstProofRegistrations,
    UnsupportedFirstProofSecurity,
    CompilerBounds,
    CompilerStart,
    CompilerStep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FailedAdmission {
    cause: AdmissionCause,
    provisional_artifacts: usize,
}

fn shape_bytes(enumeration: &Enumeration<'_>, thing_id: &str) -> u64 {
    let plan_bytes: usize = enumeration
        .coordinates
        .iter()
        .map(|coordinate| {
            size_of::<LogicalInteractionPlan>()
                + thing_id.len()
                + coordinate.property_name.len()
                + coordinate.resolved_target.len()
                + coordinate.content_type.len()
                + coordinate.subprotocol.map_or(0, str::len)
        })
        .sum();
    let target_bytes: usize = enumeration
        .targets
        .iter()
        .map(|target| size_of::<TargetProjection>() + target.property_name.len())
        .sum();
    (plan_bytes
        + target_bytes
        + enumeration.coordinates.len()
            * (size_of::<BindingCandidate>()
                + size_of::<BindingArtifactRef>()
                + size_of::<BindingPlanRefEquivalent>())) as u64
}

fn settle(
    cause: AdmissionCause,
    provisional_artifacts: usize,
    generation: Option<PlanSetBuildLease>,
    resources: ResourceBuildLease,
) -> FailedAdmission {
    if let Some(generation) = generation {
        generation.abort();
    }
    resources.fail();
    FailedAdmission {
        cause,
        provisional_artifacts,
    }
}

fn admit(
    validated: ValidatedConsumerThing,
    snapshot: Rc<RegistrationSnapshot>,
    generations: PlanSetArena,
    resources: ResourceArena,
) -> Result<FrozenAggregate, FailedAdmission> {
    let mut resource_lease = resources.transfer_source(validated.census.admitted_source_bytes);
    let selected_ordinal = match snapshot.select_first_proof_consumer() {
        Ok(ordinal) => ordinal,
        Err(cause) => return Err(settle(cause, 0, None, resource_lease)),
    };
    let registration = &snapshot.registrations[selected_ordinal];
    let identity = registration.identity;
    let compiler = registration.compiler.clone();

    let enumeration_temporary = u64::from(validated.census.forms)
        * (size_of::<CoordinateSeed<'_>>() as u64)
        + u64::from(validated.census.properties) * (size_of::<TargetSeed<'_>>() as u64);
    resource_lease.reserve_enumeration(enumeration_temporary);
    let enumeration = match enumerate(&validated.thing) {
        Ok(enumeration) => enumeration,
        Err(cause) => return Err(settle(cause, 0, None, resource_lease)),
    };
    resource_lease.enumerated();

    let generation_lease = generations.reserve();
    let thing_id = validated
        .thing
        .id
        .as_ref()
        .expect("validated fixture has Thing id")
        .as_str();
    resource_lease.reserve_shape(
        shape_bytes(&enumeration, thing_id),
        size_of::<AggregateDiagnostic>() as u64,
    );

    let candidate = BindingCandidate::new(
        identity.binding_id(),
        identity.binding_generation(),
        identity.configuration(),
        identity.artifact_compatibility(),
        selected_ordinal as u32,
        0,
    );
    let targets: Vec<TargetProjection> = enumeration
        .targets
        .iter()
        .map(|target| TargetProjection {
            property_name: Box::from(target.property_name),
            first_join: target.first_join,
            join_count: target.join_count,
        })
        .collect();
    let mut coordinates = Vec::with_capacity(enumeration.coordinates.len());
    for (slot, seed) in enumeration.coordinates.iter().enumerate() {
        let plan_id = generation_lease.plan_id(slot as u32);
        let plan = Box::new(
            LogicalInteractionPlan::try_property_read(
                plan_id,
                ThingId::from(thing_id),
                Box::from(seed.property_name),
                seed.form_index,
                Box::from(seed.resolved_target.as_str()),
                Some(Box::from(seed.content_type)),
                seed.subprotocol.map(Box::from),
            )
            .expect("fixture coordinate is a valid final plan"),
        );
        resources
            .trace
            .push(TraceEvent::PlanMaterialized(plan.plan_id()));
        coordinates.push(MaterializedCoordinate {
            target_slot: seed.target_slot,
            plan,
            candidate,
        });
    }
    drop(enumeration);
    resource_lease.release_enumeration();

    let mut bounded = Vec::with_capacity(coordinates.len());
    let mut artifact_reservation = 0u64;
    let mut compiler_temporary = 0u64;
    for coordinate in coordinates {
        let input = BindingCompilerInput::new(
            coordinate.plan.as_ref(),
            coordinate.candidate,
            BindingArtifactRole::ConsumerCall,
        );
        let bounds = match compiler.bounds(&input) {
            Ok(bounds) => bounds,
            Err(_) => {
                return Err(settle(
                    AdmissionCause::CompilerBounds,
                    0,
                    Some(generation_lease),
                    resource_lease,
                ));
            }
        };
        artifact_reservation += size_of::<BindingArtifactEnvelope<FixtureArtifact>>() as u64
            + bounds.artifact().retained_bytes();
        compiler_temporary = compiler_temporary.max(
            bounds
                .cursor_bytes()
                .checked_add(bounds.temporary_bytes())
                .expect("fixture compiler temporary bound"),
        );
        bounded.push(BoundedCoordinate {
            target_slot: coordinate.target_slot,
            plan: coordinate.plan,
            candidate: coordinate.candidate,
            admitted: bounds.artifact(),
            work: bounds.into_work(),
        });
    }
    resource_lease.reserve_compiler(artifact_reservation, compiler_temporary);

    let diagnostic = AggregateDiagnostic {
        snapshot_ordinal: selected_ordinal as u32,
        registration_diagnostic_ordinal: identity.diagnostic_ordinal(),
        target_count: targets.len() as u32,
        readable_coordinate_count: bounded.len() as u32,
    };
    let mut draft = AggregateDraft {
        plans: Vec::with_capacity(bounded.len()),
        candidates: Vec::with_capacity(bounded.len()),
        artifacts: Vec::with_capacity(bounded.len()),
        artifact_refs: Vec::with_capacity(bounded.len()),
        runtime_joins: Vec::with_capacity(bounded.len()),
        targets,
        diagnostic,
    };

    for mut coordinate in bounded {
        let input = BindingCompilerInput::new(
            coordinate.plan.as_ref(),
            coordinate.candidate,
            BindingArtifactRole::ConsumerCall,
        );
        let mut cursor = match compiler.start(&input) {
            Ok(cursor) => cursor,
            Err(_) => {
                return Err(settle(
                    AdmissionCause::CompilerStart,
                    draft.artifacts.len(),
                    Some(generation_lease),
                    resource_lease,
                ));
            }
        };
        let artifact = loop {
            match compiler.step(&input, cursor, &mut coordinate.work) {
                BindingCompilerStep::Pending(next) => cursor = next,
                BindingCompilerStep::Complete(output) => break output.into_artifact(),
                BindingCompilerStep::Failed(failure) => {
                    let (_, returned_cursor) = failure.into_parts();
                    compiler.abort(returned_cursor);
                    return Err(settle(
                        AdmissionCause::CompilerStep,
                        draft.artifacts.len(),
                        Some(generation_lease),
                        resource_lease,
                    ));
                }
            }
        };
        let artifact_slot = draft.artifacts.len() as u32;
        let plan_slot = draft.plans.len() as u32;
        let identity = BindingArtifactIdentity::new(
            generation_lease.generation,
            coordinate.plan.plan_id(),
            coordinate.candidate.binding_id(),
            coordinate.candidate.binding_generation(),
            coordinate.candidate.configuration(),
            coordinate.candidate.compatibility(),
            BindingArtifactRole::ConsumerCall,
        );
        let envelope = BindingArtifactEnvelope::try_new(identity, coordinate.admitted, artifact)
            .expect("fixture measured artifact fits the reserved identity");
        draft.plans.push(coordinate.plan);
        draft.candidates.push(coordinate.candidate);
        draft.artifact_refs.push(BindingArtifactRef::new(
            identity,
            SlotIndex::new(artifact_slot),
        ));
        draft.artifacts.push(envelope);
        draft.runtime_joins.push(BindingPlanRefEquivalent {
            target_slot: coordinate.target_slot,
            plan_slot,
            candidate_slot: plan_slot,
            artifact_slot,
        });
    }

    resource_lease.release_compiler_temporary();
    draft.validate(generation_lease.generation, &snapshot);
    let measured_runtime = draft.measured_runtime_bytes();
    let measured_diagnostic = size_of::<AggregateDiagnostic>() as u64;

    // The source owner is dropped before either authority is committed to
    // Frozen; the aggregate above contains no TD lifetime.
    drop(validated);
    resource_lease.release_source();
    let frozen_resources = resource_lease.commit(measured_runtime, measured_diagnostic);
    let frozen_generation = generation_lease.commit();
    Ok(FrozenAggregate {
        generation: frozen_generation,
        resources: frozen_resources,
        snapshot,
        draft,
    })
}

fn fixture_thing() -> Thing {
    Thing::builder("Aggregate sensor")
        .id("urn:test:aggregate-consumer")
        .nosec()
        .property(
            "humidity",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    Form::write_property("mock://thing/humidity")
                        .build()
                        .expect("fixture write form"),
                )
                .build()
                .expect("fixture humidity property"),
        )
        .property(
            "pressure",
            PropertyAffordance::builder(DataSchema::number())
                .form(
                    Form::read_property("mock://thing/pressure")
                        .build()
                        .expect("fixture read form"),
                )
                .build()
                .expect("fixture pressure property"),
        )
        .property(
            "temperature",
            PropertyAffordance::builder(DataSchema::number())
                .form(
                    Form::write_property("mock://thing/temperature")
                        .build()
                        .expect("fixture write form"),
                )
                .form(
                    Form::read_property("mock://thing/temperature")
                        .subprotocol("fixture")
                        .build()
                        .expect("fixture read form"),
                )
                .build()
                .expect("fixture temperature property"),
        )
        .build()
        .expect("fixture Thing")
}

fn event_position(events: &[TraceEvent], predicate: impl Fn(&TraceEvent) -> bool) -> usize {
    events
        .iter()
        .position(predicate)
        .expect("expected trace event")
}

#[test]
fn owned_validated_input_becomes_one_fully_joined_frozen_aggregate() {
    let trace = Trace::default();
    let compiler = Rc::new(FixtureCompiler::new(trace.clone()));
    let snapshot = Rc::new(RegistrationSnapshot::first_proof(compiler.clone()));
    let snapshot_weak = Rc::downgrade(&snapshot);
    let generations = PlanSetArena::default();
    let resources = ResourceArena::new(trace.clone());
    let validated = ValidatedConsumerThing::new(fixture_thing()).expect("Basic-valid owned TD");
    assert_eq!(validated.census.properties, 3);
    assert_eq!(validated.census.forms, 4);
    assert!(validated.census.admitted_source_bytes > 0);

    let frozen = match admit(
        validated,
        snapshot.clone(),
        generations.clone(),
        resources.clone(),
    ) {
        Ok(frozen) => frozen,
        Err(failure) => panic!("success fixture failed: {failure:?}"),
    };

    assert_eq!(frozen.generation.generation, PlanSetGeneration::INITIAL);
    assert_eq!(frozen.draft.plans.len(), 2);
    assert_eq!(
        frozen
            .draft
            .plans
            .iter()
            .map(|plan| (plan.property_name(), plan.form_index()))
            .collect::<Vec<_>>(),
        vec![("pressure", 0), ("temperature", 1)]
    );
    assert_eq!(frozen.draft.target("humidity").unwrap().join_count, 0);
    assert!(frozen.draft.target("absent").is_none());
    assert_eq!(frozen.draft.diagnostic.snapshot_ordinal, 1);
    assert_eq!(frozen.draft.diagnostic.registration_diagnostic_ordinal, 17);
    for plan in &frozen.draft.plans {
        assert_eq!(
            plan.plan_id().generation(),
            frozen.generation.generation.get()
        );
        let address = plan.as_ref() as *const LogicalInteractionPlan as usize;
        let observations = compiler
            .bounds
            .borrow()
            .iter()
            .chain(compiler.starts.borrow().iter())
            .chain(compiler.steps.borrow().iter())
            .filter(|observation| observation.plan_id == plan.plan_id())
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(observations.len(), 4);
        assert!(
            observations
                .iter()
                .all(|observation| observation.plan_address == address)
        );
    }

    let events = trace.snapshot();
    let shape = event_position(&events, |event| matches!(event, TraceEvent::ShapeReserved));
    let first_plan = event_position(&events, |event| {
        matches!(event, TraceEvent::PlanMaterialized(_))
    });
    let last_bounds = events
        .iter()
        .rposition(|event| matches!(event, TraceEvent::BoundsObserved(_)))
        .expect("bounds event");
    let compiler_reserved = event_position(&events, |event| {
        matches!(event, TraceEvent::CompilerReserved)
    });
    let first_start = event_position(&events, |event| {
        matches!(event, TraceEvent::StartObserved(_))
    });
    assert!(shape < first_plan);
    assert!(last_bounds < compiler_reserved && compiler_reserved < first_start);

    let accounts = resources.snapshot();
    assert_eq!(accounts.retained_source, 0);
    assert_eq!(accounts.temporary_build, 0);
    assert_eq!(accounts.persistent_effective_document, 0);
    assert_eq!(accounts.persistent_runtime_reserved, 0);
    assert!(accounts.persistent_runtime_committed > 0);
    assert!(accounts.diagnostic_committed > 0);
    assert_eq!(accounts.cleanup_state, 0);
    assert!(accounts.peak_live >= accounts.live());
    assert!(accounts.largest_allocation > 0);

    drop(snapshot);
    assert!(snapshot_weak.upgrade().is_some());
    assert_eq!(frozen.resolve_client("pressure"), Some("consumer-client"));
    assert_eq!(frozen.resolve_client("humidity"), None);
    assert_eq!(compiler.artifact_drops.get(), 0);
    frozen.reclaim();
    assert_eq!(compiler.artifact_drops.get(), 2);
    assert!(snapshot_weak.upgrade().is_none());
    assert!(resources.snapshot().settled_zero());
    assert_eq!(
        generations.next_generation(),
        PlanSetGeneration::INITIAL.checked_next().unwrap()
    );
}

#[test]
fn partial_compiler_failure_settles_before_failed_and_invalidates_the_whole_set() {
    let trace = Trace::default();
    let compiler = Rc::new(FixtureCompiler::new(trace.clone()));
    compiler.fail_slot.set(Some(1));
    let snapshot = Rc::new(RegistrationSnapshot::first_proof(compiler.clone()));
    let generations = PlanSetArena::default();
    let resources = ResourceArena::new(trace.clone());
    let failure = match admit(
        ValidatedConsumerThing::new(fixture_thing()).unwrap(),
        snapshot,
        generations.clone(),
        resources.clone(),
    ) {
        Ok(_) => panic!("failure fixture unexpectedly froze"),
        Err(failure) => failure,
    };

    assert_eq!(failure.cause, AdmissionCause::CompilerStep);
    assert_eq!(failure.provisional_artifacts, 1);
    assert_eq!(compiler.aborts.get(), 1);
    assert_eq!(compiler.artifact_drops.get(), 1);
    assert!(resources.snapshot().settled_zero());
    let next = PlanSetGeneration::INITIAL.checked_next().unwrap();
    assert_eq!(generations.next_generation(), next);
    let reused = generations.reserve();
    assert_eq!(reused.plan_id(0).generation(), next.get());
    assert_eq!(reused.plan_id(1).generation(), next.get());
    reused.abort();

    let events = trace.snapshot();
    let abort = event_position(&events, |event| {
        matches!(event, TraceEvent::CursorAborted(_))
    });
    let failed = event_position(&events, |event| matches!(event, TraceEvent::FailedSettled));
    assert!(abort < failed);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, TraceEvent::Frozen))
    );
}

#[test]
fn first_proof_selection_rejects_zero_or_multiple_consumers_without_reserving_identity() {
    for (snapshot, expected) in [
        (
            RegistrationSnapshot::producer_only(Trace::default()),
            AdmissionCause::NoConsumerRegistration,
        ),
        (
            RegistrationSnapshot::first_proof(Rc::new(FixtureCompiler::new(Trace::default())))
                .with_second_consumer(),
            AdmissionCause::AmbiguousFirstProofRegistrations,
        ),
    ] {
        let trace = Trace::default();
        let generations = PlanSetArena::default();
        let resources = ResourceArena::new(trace);
        let failure = match admit(
            ValidatedConsumerThing::new(fixture_thing()).unwrap(),
            Rc::new(snapshot),
            generations.clone(),
            resources.clone(),
        ) {
            Ok(_) => panic!("unsupported registration shape unexpectedly froze"),
            Err(failure) => failure,
        };
        assert_eq!(failure.cause, expected);
        assert_eq!(generations.next_generation(), PlanSetGeneration::INITIAL);
        assert!(resources.snapshot().settled_zero());
    }
}

#[test]
fn first_proof_rejects_credential_security_instead_of_silently_omitting_a_form() {
    let mut thing = fixture_thing();
    thing.security.clear();
    thing.security_definitions.clear();
    thing.security.push("basic".to_owned());
    thing
        .security_definitions
        .insert("basic".to_owned(), SecurityScheme::basic("Authorization"));
    let validated = ValidatedConsumerThing::new(thing).expect("Basic security TD validates");
    let trace = Trace::default();
    let compiler = Rc::new(FixtureCompiler::new(trace.clone()));
    let snapshot = Rc::new(RegistrationSnapshot::first_proof(compiler));
    let generations = PlanSetArena::default();
    let resources = ResourceArena::new(trace);

    let failure = match admit(validated, snapshot, generations.clone(), resources.clone()) {
        Ok(_) => panic!("credential security unexpectedly entered the NoSec proof"),
        Err(failure) => failure,
    };

    assert_eq!(failure.cause, AdmissionCause::UnsupportedFirstProofSecurity);
    assert_eq!(failure.provisional_artifacts, 0);
    assert_eq!(generations.next_generation(), PlanSetGeneration::INITIAL);
    assert!(resources.snapshot().settled_zero());
}
