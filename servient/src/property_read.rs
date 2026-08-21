//! Narrow Property Read lifecycle composition shared by the static and host
//! Servient profiles.

use alloc::{boxed::Box, sync::Arc};
use core::task::{Context, Poll};

use clinkz_wot_core::binding::BindingRouteKey;
use clinkz_wot_core::{
    AffordanceTarget, BindingArtifactEnvelope, BindingArtifactRef, BindingArtifactRole,
    BindingCompilerExtension, BindingDeliveryOutcome, BindingLifetimeFootprint,
    BindingOperationalError, BindingRegistrationIdentity, CleanupOperation, CleanupPhaseContext,
    CleanupRecord, CleanupReservation, CleanupSlotId, CoreError, CoreResult, Deadline,
    ErrorContext, ErrorPhase, HandlerContext, HandlerFootprint, PendingWork, PendingWorkClass,
    PlanId, PlanSetGeneration, PollServerBinding, PrepareInput, ReadPropertyHandler, RetryClass,
    RouteAcceptEvent, RouteAcceptLease, RouteActivationOutcome, RouteCleanupOutcome,
    RouteCommitOutcome, RouteInboundResponse, RoutePrepareOutcome, RouteReadinessOutcome,
    RouteReadinessSlot, RouteTerminal, ServerResponseSlot, ServerRouteSlot,
    ServingActivationAuthority, StartStatus, StaticBindingRegistration, StaticHandlerRegistration,
    StepStatus, ThingId, ThingSlotId,
};
use clinkz_wot_foundation::{ResourceKind, ResourceLimits, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_planning::{
    PlanBuildOutput, PlanBuildStep, PlanCompiler, PropertyReadBuildCursor, PropertyReadPlanCompiler,
};
use clinkz_wot_td::{data_type::Operation, thing::Thing};

#[cfg(feature = "std")]
use clinkz_wot_core::{
    HostActiveRouteGuard, HostBindingArtifact, HostBindingCallBox, HostBindingCompilerCursor,
    HostBindingRegistration, HostCommittedRouteGuard, HostPreparedRouteGuard,
    HostRouteCleanupSuccessor, HostShutdownRouteGuard, RouteAbortInput, RouteShutdownInput,
};

/// Read-only exposure lifecycle view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExposeState {
    Draft,
    Preparing,
    ReadyPendingActivation,
    Activating,
    Committing,
    Serving,
    Cancelling,
    Draining,
    CleanupPending,
    Cancelled,
    Destroyed,
    Failed,
}

/// Read-only lifecycle view for one immutable compiled plan set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompiledPlanSetState {
    Building,
    Frozen,
    Published,
    Draining,
    Failed,
    Reclaimed,
}

/// Application-static manually progressed Servient boundary.
pub trait StaticServient {
    /// Closes route selection before later caller-budgeted cleanup steps.
    fn begin_destroy(&mut self) -> CoreResult<()>;

    /// Drives one bounded transition of the static lifecycle cell.
    fn step(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()>;
}

/// Typestate builder for one application-static Property Read Servient.
pub struct StaticServientBuilder<B = (), H = ()> {
    td: Thing,
    thing_slot: ThingSlotId,
    limits: ResourceLimits,
    deadline: Deadline,
    binding: B,
    handler: H,
    handler_name: Option<Box<str>>,
}

impl<'h, H> StaticServientBuilder<(), Option<StaticHandlerRegistration<'h, H>>>
where
    H: ReadPropertyHandler + 'h,
{
    pub fn new(
        td: Thing,
        thing_slot: ThingSlotId,
        limits: ResourceLimits,
        deadline: Deadline,
    ) -> Self {
        Self {
            td,
            thing_slot,
            limits,
            deadline,
            binding: (),
            handler: None,
            handler_name: None,
        }
    }
}

impl<H> StaticServientBuilder<(), H> {
    pub fn binding_registration<B>(
        self,
        binding: StaticBindingRegistration<B>,
    ) -> StaticServientBuilder<StaticBindingRegistration<B>, H>
    where
        B: PollServerBinding,
    {
        StaticServientBuilder {
            td: self.td,
            thing_slot: self.thing_slot,
            limits: self.limits,
            deadline: self.deadline,
            binding,
            handler: self.handler,
            handler_name: self.handler_name,
        }
    }
}

impl<'h, B, H> StaticServientBuilder<B, Option<StaticHandlerRegistration<'h, H>>>
where
    H: ReadPropertyHandler + 'h,
{
    pub fn read_property_handler(
        self,
        name: impl Into<Box<str>>,
        handler: StaticHandlerRegistration<'h, H>,
    ) -> StaticServientBuilder<B, StaticHandlerRegistration<'h, H>> {
        StaticServientBuilder {
            td: self.td,
            thing_slot: self.thing_slot,
            limits: self.limits,
            deadline: self.deadline,
            binding: self.binding,
            handler,
            handler_name: Some(name.into()),
        }
    }
}

impl<'h, B, H> StaticServientBuilder<StaticBindingRegistration<B>, StaticHandlerRegistration<'h, H>>
where
    B: PollServerBinding + 'h,
    H: ReadPropertyHandler + 'h,
{
    pub fn build(self) -> CoreResult<impl StaticServient + 'h> {
        let admission = AdmissionReservations::new(
            &self.limits,
            self.thing_slot,
            self.binding.resources().route_state(),
            self.binding.status().retained_records(),
            self.deadline,
        )?;
        if self.handler.slot_id().generation() != self.thing_slot.generation() {
            return Err(validation_error(self.thing_slot));
        }
        validate_handler_footprint(&self.limits, self.thing_slot, self.handler.footprint())?;
        Ok(StaticPropertyReadServient::new(
            self.td,
            self.thing_slot,
            self.binding,
            self.handler,
            self.handler_name
                .expect("typestate guarantees a Property Read target"),
            admission,
        ))
    }
}

#[cfg(feature = "std")]
struct PropertyReadHandlerRecord {
    target: Box<str>,
    handler: Box<dyn ReadPropertyHandler + Send + Sync>,
    footprint: HandlerFootprint,
}

struct PlanSetLease {
    generation: PlanSetGeneration,
}

struct CompiledPlanSetRecord<A> {
    state: CompiledPlanSetState,
    output: Option<PlanBuildOutput<A>>,
    lease: Option<PlanSetLease>,
}

impl<A> CompiledPlanSetRecord<A> {
    fn building() -> Self {
        Self {
            state: CompiledPlanSetState::Building,
            output: None,
            lease: None,
        }
    }

    fn freeze(&mut self, output: PlanBuildOutput<A>, generation: PlanSetGeneration) {
        self.output = Some(output);
        self.lease = Some(PlanSetLease { generation });
        self.state = CompiledPlanSetState::Frozen;
    }

    fn resolve_prepare_artifact<'a>(
        &'a self,
        input: &PrepareInput,
        registration: BindingRegistrationIdentity,
        thing_slot: ThingSlotId,
        admitted_footprint: BindingLifetimeFootprint,
    ) -> CoreResult<&'a BindingArtifactEnvelope<A>> {
        if self.state != CompiledPlanSetState::Frozen
            || input.admitted_footprint() != admitted_footprint
        {
            return Err(validation_error(thing_slot));
        }
        let output = self
            .output
            .as_ref()
            .ok_or_else(|| validation_error(thing_slot))?;
        let lease = self
            .lease
            .as_ref()
            .ok_or_else(|| validation_error(thing_slot))?;
        let [plan] = output.logical_plans() else {
            return Err(validation_error(thing_slot));
        };
        let reference = input.artifact();
        let slot = usize::try_from(reference.artifact_slot().get())
            .map_err(|_| validation_error(thing_slot))?;
        let envelope = output
            .artifacts()
            .get(slot)
            .ok_or_else(|| validation_error(thing_slot))?;
        let stored_reference = output
            .artifact_refs()
            .get(slot)
            .ok_or_else(|| validation_error(thing_slot))?;
        let identity = envelope.identity();
        let route = *input.route();
        if output.artifacts().len() != output.artifact_refs().len()
            || stored_reference != &reference
            || reference.identity() != identity
            || identity.plan_set_generation() != lease.generation
            || identity.plan_id() != plan.plan_id()
            || identity.binding_id() != registration.binding_id()
            || identity.binding_generation() != registration.binding_generation()
            || identity.configuration() != registration.configuration()
            || identity.compatibility() != registration.artifact_compatibility()
            || identity.compatibility() != envelope.artifact().compatibility()
            || identity.role() != BindingArtifactRole::ProducerRoute
            || route.binding_id() != identity.binding_id()
            || route.binding_generation() != identity.binding_generation()
            || route.route_generation() != thing_slot.generation()
            || route.plan_set_generation() != identity.plan_set_generation()
            || route.plan_id() != identity.plan_id()
            || envelope.route_reservation() != Some(route.reservation())
        {
            return Err(validation_error(thing_slot));
        }
        Ok(envelope)
    }

    fn publish(&mut self) {
        debug_assert!(self.lease.as_ref().is_some_and(|lease| {
            self.output
                .as_ref()
                .and_then(|output| output.artifact_refs().first())
                .is_some_and(|reference| {
                    reference.identity().plan_set_generation() == lease.generation
                })
        }));
        self.state = CompiledPlanSetState::Published;
    }

    fn reclaim(&mut self) {
        self.state = CompiledPlanSetState::Draining;
        self.output = None;
        self.lease = None;
        self.state = CompiledPlanSetState::Reclaimed;
    }
}

struct BindingRouteRecord<S> {
    key: Option<BindingRouteKey>,
    state: S,
}

struct InFlightRecord {
    state: InFlightState,
}

enum InFlightState {
    Accepted(clinkz_wot_core::RouteInboundRequest),
    Response(RouteInboundResponse),
    Delivering {
        route: BindingRouteKey,
        correlation: clinkz_wot_core::CorrelationId,
    },
}

enum LifecycleFailure {
    Core(CoreError),
    Binding(BindingOperationalError),
}

impl LifecycleFailure {
    fn core_error(&self) -> &CoreError {
        match self {
            Self::Core(error) => error,
            Self::Binding(error) => error.error(),
        }
    }
}

struct LifecycleDisposition {
    first_failure: Option<LifecycleFailure>,
    cleanup_cause: Option<CoreError>,
    residual_cleanup: Option<CleanupRecord>,
    cancelled_before_publication: bool,
}

impl LifecycleDisposition {
    const fn new() -> Self {
        Self {
            first_failure: None,
            cleanup_cause: None,
            residual_cleanup: None,
            cancelled_before_publication: false,
        }
    }

    fn record_core_failure(&mut self, error: CoreError) {
        if self.first_failure.is_none() {
            self.first_failure = Some(LifecycleFailure::Core(error));
        }
    }

    fn record_binding_failure(&mut self, error: BindingOperationalError) {
        if self.first_failure.is_none() {
            self.first_failure = Some(LifecycleFailure::Binding(error));
        }
    }

    fn cleanup_cause(&mut self, thing_slot: ThingSlotId) -> CoreError {
        let cause = self
            .first_failure
            .as_ref()
            .map(|failure| failure.core_error().clone())
            .unwrap_or_else(|| {
                CoreError::Cancelled(
                    ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never)
                        .with_thing(thing_slot),
                )
            });
        if self.cleanup_cause.is_none() {
            self.cleanup_cause = Some(cause.clone());
        }
        cause
    }

    fn retain_residual(&mut self, record: CleanupRecord) {
        self.residual_cleanup = Some(record);
    }

    fn mark_cancelled_before_publication(&mut self) {
        self.cancelled_before_publication = true;
    }

    const fn failed(&self) -> bool {
        self.first_failure.is_some()
    }

    const fn terminal_expose_state(&self) -> ExposeState {
        if self.failed() {
            ExposeState::Failed
        } else if self.cancelled_before_publication {
            ExposeState::Cancelled
        } else {
            ExposeState::Destroyed
        }
    }
}

struct ServingActivationRecord {
    authority: ServingActivationAuthority,
    lease: RouteAcceptLease,
    published: bool,
}

impl ServingActivationRecord {
    fn new(authority: ServingActivationAuthority, route: BindingRouteKey) -> Self {
        let lease = RouteAcceptLease::new(&authority, route);
        Self {
            authority,
            lease,
            published: false,
        }
    }

    fn publish(&mut self) {
        self.published = true;
    }

    fn close(&mut self) {
        self.published = false;
    }
}

struct AdmissionReservations {
    cleanup: Option<CleanupReservation>,
    deadline: Deadline,
}

impl AdmissionReservations {
    fn new(
        limits: &ResourceLimits,
        thing_slot: ThingSlotId,
        route_footprint: BindingLifetimeFootprint,
        durable_status_records: u32,
        deadline: Deadline,
    ) -> CoreResult<Self> {
        for kind in [
            ResourceKind::PlanSetsPerThingMax,
            ResourceKind::PlanPinsPerPlanSetMax,
            ResourceKind::HandlerSlotsPerThingMax,
            ResourceKind::PendingHandlerCallsPerThingMax,
            ResourceKind::InFlightResponsesPerThingMax,
            ResourceKind::BindingRoutesPerThingMax,
            ResourceKind::RouteReadinessTokensPerThingMax,
            ResourceKind::CleanupItemsMax,
            ResourceKind::CleanupBytesMax,
        ] {
            require_capacity(limits, thing_slot, kind, 1)?;
        }
        let route_bytes = route_footprint.retained_bytes();
        require_capacity(
            limits,
            thing_slot,
            ResourceKind::RouteGuardBytesPerItemMax,
            route_bytes,
        )?;
        let cleanup_bytes = limits
            .get(ResourceKind::CleanupItemBytesMax)
            .unwrap_or(route_bytes)
            .min(route_bytes.max(1));
        let cleanup_steps = limits
            .get(ResourceKind::CleanupWorkItemsPerStepMax)
            .unwrap_or(1)
            .max(1);
        let cleanup = CleanupReservation::new(
            CleanupSlotId::new(thing_slot.slot(), thing_slot.generation()),
            BindingLifetimeFootprint::new(1, cleanup_bytes),
            durable_status_records.max(1),
            WorkBudget::new().with_remaining(WorkClass::CleanupItems, cleanup_steps),
        );
        Ok(Self {
            cleanup: Some(cleanup),
            deadline,
        })
    }

    fn cleanup_context(
        &mut self,
        thing_slot: ThingSlotId,
        operation: CleanupOperation,
        first_cause: CoreError,
    ) -> CoreResult<CleanupPhaseContext> {
        let reservation = self
            .cleanup
            .take()
            .ok_or_else(|| validation_error(thing_slot))?;
        Ok(CleanupPhaseContext::bind(
            reservation,
            operation,
            first_cause,
            self.deadline,
        ))
    }
}

fn validate_handler_footprint(
    limits: &ResourceLimits,
    thing_slot: ThingSlotId,
    footprint: HandlerFootprint,
) -> CoreResult<()> {
    require_capacity(
        limits,
        thing_slot,
        ResourceKind::HandlerStateBytesPerThingMax,
        footprint.retained_bytes(),
    )?;
    require_capacity(
        limits,
        thing_slot,
        ResourceKind::BindingResponseBufferBytesPerRouteMax,
        footprint.pending_call_bytes().max(1),
    )
}

fn require_capacity(
    limits: &ResourceLimits,
    thing_slot: ThingSlotId,
    kind: ResourceKind,
    requested: u64,
) -> CoreResult<()> {
    let limit = limits.get(kind).unwrap_or(0);
    if requested > limit {
        return Err(CoreError::LimitExceeded {
            resource: kind,
            limit,
            requested: Some(requested),
            observed: None,
            context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                .with_thing(thing_slot),
        });
    }
    Ok(())
}

fn validation_error(thing_slot: ThingSlotId) -> CoreError {
    CoreError::Validation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never).with_thing(thing_slot),
    )
}

struct DerivedRoute {
    key: BindingRouteKey,
    artifact_ref: BindingArtifactRef,
    thing_id: ThingId,
    target: AffordanceTarget,
    plan_id: PlanId,
}

fn derive_route<A>(
    output: &PlanBuildOutput<A>,
    registration: BindingRegistrationIdentity,
    thing_slot: ThingSlotId,
) -> CoreResult<DerivedRoute> {
    let [plan] = output.logical_plans() else {
        return Err(validation_error(thing_slot));
    };
    let [artifact] = output.artifacts() else {
        return Err(validation_error(thing_slot));
    };
    let [artifact_ref] = output.artifact_refs() else {
        return Err(validation_error(thing_slot));
    };
    let identity = artifact.identity();
    if artifact_ref.identity() != identity
        || artifact_ref.artifact_slot() != SlotIndex::new(0)
        || identity.plan_id() != plan.plan_id()
        || identity.binding_id() != registration.binding_id()
        || identity.binding_generation() != registration.binding_generation()
        || identity.configuration() != registration.configuration()
        || identity.compatibility() != registration.artifact_compatibility()
        || identity.role() != BindingArtifactRole::ProducerRoute
    {
        return Err(validation_error(thing_slot));
    }
    let reservation = artifact
        .route_reservation()
        .ok_or_else(|| validation_error(thing_slot))?;
    let key = BindingRouteKey::new(
        registration.binding_id(),
        registration.binding_generation(),
        thing_slot.generation(),
        identity.plan_set_generation(),
        plan.plan_id(),
        reservation,
    );
    Ok(DerivedRoute {
        key,
        artifact_ref: *artifact_ref,
        thing_id: plan.thing_id().clone(),
        target: AffordanceTarget::Property(Arc::from(plan.property_name())),
        plan_id: plan.plan_id(),
    })
}

fn progress(class: PendingWorkClass) -> StepStatus<()> {
    StepStatus::progress(None, Some(PendingWork::new(class)))
}

type StaticCursor<B> = PropertyReadBuildCursor<
    <<B as PollServerBinding>::Compiler as BindingCompilerExtension>::Cursor,
    <<B as PollServerBinding>::Compiler as BindingCompilerExtension>::Artifact,
>;

type StaticArtifact<B> = <<B as PollServerBinding>::Compiler as BindingCompilerExtension>::Artifact;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticRoutePhase {
    Absent,
    Preparing,
    Prepared,
    AwaitingReadiness,
    Ready,
    Activating,
    Active,
    Committing,
    CommittedClosed,
    Serving,
    Dispatching,
    Delivering,
    DeliveredResponse,
    FailedResponse,
    Aborting,
    Cleaning,
    FinalizingCleanup,
    Closed,
}

struct StaticRouteStorage<B: PollServerBinding> {
    phase: StaticRoutePhase,
    route: ServerRouteSlot<B::RouteState>,
    readiness: RouteReadinessSlot<B::ReadinessState>,
    response: ServerResponseSlot<B::ResponseState>,
}

struct StaticPropertyReadServient<'h, B, H>
where
    B: PollServerBinding,
    H: ReadPropertyHandler,
{
    td: Option<Thing>,
    thing_slot: ThingSlotId,
    registration: StaticBindingRegistration<B>,
    handler: StaticHandlerRegistration<'h, H>,
    handler_name: Box<str>,
    expose: ExposeState,
    plan_set: CompiledPlanSetRecord<StaticArtifact<B>>,
    cursor: Option<StaticCursor<B>>,
    derived: Option<DerivedRoute>,
    route: BindingRouteRecord<StaticRouteStorage<B>>,
    activation: Option<ServingActivationRecord>,
    in_flight: Option<InFlightRecord>,
    admission: AdmissionReservations,
    disposition: LifecycleDisposition,
    cleanup_outcome: Option<RouteCleanupOutcome>,
}

impl<'h, B, H> StaticPropertyReadServient<'h, B, H>
where
    B: PollServerBinding,
    H: ReadPropertyHandler,
{
    fn new(
        td: Thing,
        thing_slot: ThingSlotId,
        registration: StaticBindingRegistration<B>,
        handler: StaticHandlerRegistration<'h, H>,
        handler_name: Box<str>,
        admission: AdmissionReservations,
    ) -> Self {
        Self {
            td: Some(td),
            thing_slot,
            registration,
            handler,
            handler_name,
            expose: ExposeState::Preparing,
            plan_set: CompiledPlanSetRecord::building(),
            cursor: None,
            derived: None,
            route: BindingRouteRecord {
                key: None,
                state: StaticRouteStorage {
                    phase: StaticRoutePhase::Absent,
                    route: ServerRouteSlot::new(),
                    readiness: RouteReadinessSlot::new(),
                    response: ServerResponseSlot::new(),
                },
            },
            activation: None,
            in_flight: None,
            admission,
            disposition: LifecycleDisposition::new(),
            cleanup_outcome: None,
        }
    }

    fn fail_without_route(&mut self, error: CoreError) -> StepStatus<()> {
        self.disposition.record_core_failure(error);
        self.plan_set.reclaim();
        self.route.state.phase = StaticRoutePhase::Closed;
        self.expose = ExposeState::Failed;
        StepStatus::Terminal(())
    }

    fn fail_binding_without_route(&mut self, error: BindingOperationalError) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.plan_set.reclaim();
        self.route.state.phase = StaticRoutePhase::Closed;
        self.expose = ExposeState::Failed;
        StepStatus::Terminal(())
    }

    fn finish_rejected_route(&mut self, error: BindingOperationalError) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        if self.route.state.route.is_vacant() {
            self.plan_set.reclaim();
            self.route.state.phase = StaticRoutePhase::Closed;
            self.expose = ExposeState::Failed;
            return StepStatus::Terminal(());
        }
        self.cleanup_outcome = Some(RouteCleanupOutcome::Complete);
        self.route.state.phase = StaticRoutePhase::FinalizingCleanup;
        self.finalize_cleanup()
    }

    fn start_failed_abort(
        &mut self,
        error: BindingOperationalError,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
    }

    fn start_failed_shutdown(
        &mut self,
        error: BindingOperationalError,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
    }

    fn finish_failed_response(
        &mut self,
        error: BindingOperationalError,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.in_flight = None;
        self.route.state.phase = StaticRoutePhase::FailedResponse;
        self.retry_failed_response(budget)
    }

    fn retry_failed_response(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        if let Err(error) = self
            .registration
            .server_mut()
            .acknowledge_response(&mut self.route.state.response)
        {
            self.disposition.record_core_failure(error);
            self.route.state.phase = StaticRoutePhase::FailedResponse;
            return progress(PendingWorkClass::ResponseDelivery);
        }
        self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
    }

    fn finish_delivered_response(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        if let Err(error) = self
            .registration
            .server_mut()
            .acknowledge_response(&mut self.route.state.response)
        {
            self.disposition.record_core_failure(error);
            self.route.state.phase = StaticRoutePhase::DeliveredResponse;
            return progress(PendingWorkClass::ResponseDelivery);
        }
        self.in_flight = None;
        if self.disposition.failed() {
            return self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget);
        }
        self.route.state.phase = StaticRoutePhase::Serving;
        progress(PendingWorkClass::BindingInput)
    }

    fn start_failed_shutdown_core(
        &mut self,
        error: CoreError,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        self.disposition.record_core_failure(error);
        self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
    }

    fn step_planning(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        let td = self.td.as_ref().expect("building retains the TD");
        let plan_id = PlanId::new(self.thing_slot.slot(), self.thing_slot.generation());
        let generation = PlanSetGeneration::new(self.thing_slot.generation());
        let compiler =
            PropertyReadPlanCompiler::producer_route(plan_id, self.registration.identity(), 0, 0);
        let registrations = core::slice::from_ref(self.registration.compiler());
        let input = clinkz_wot_planning::PlanBuildInput::new(td, registrations, generation);
        let cursor = match self.cursor.take() {
            Some(cursor) => cursor,
            None => match compiler.start(&input) {
                Ok(cursor) => cursor,
                Err(error) => return self.fail_without_route(error),
            },
        };
        match compiler.step(&input, cursor, budget) {
            PlanBuildStep::Pending(cursor) => {
                self.cursor = Some(cursor);
                progress(PendingWorkClass::BindingInput)
            }
            PlanBuildStep::Failed(failure) => {
                let (error, _) = failure.into_parts();
                self.fail_without_route(error)
            }
            PlanBuildStep::Complete(output) => {
                let derived =
                    match derive_route(&output, self.registration.identity(), self.thing_slot) {
                        Ok(derived) => derived,
                        Err(error) => return self.fail_without_route(error),
                    };
                if derived.target.name() != Some(&self.handler_name) {
                    return self.fail_without_route(validation_error(self.thing_slot));
                }
                let authority = ServingActivationAuthority::new(
                    derived.thing_id.clone(),
                    self.thing_slot.generation(),
                    generation,
                );
                let prepare = PrepareInput::new(
                    derived.key,
                    derived.artifact_ref,
                    self.registration.resources().route_state(),
                );
                self.plan_set.freeze(output, generation);
                let artifact = match self.plan_set.resolve_prepare_artifact(
                    &prepare,
                    self.registration.identity(),
                    self.thing_slot,
                    self.registration.resources().route_state(),
                ) {
                    Ok(artifact) => artifact,
                    Err(error) => return self.fail_without_route(error),
                };
                self.activation = Some(ServingActivationRecord::new(authority, derived.key));
                self.route.key = Some(derived.key);
                self.td = None;
                self.derived = Some(derived);
                match self.registration.server_mut().start_prepare(
                    prepare,
                    artifact,
                    &mut self.route.state.route,
                    budget,
                ) {
                    Ok(StartStatus::Pending) => {
                        self.route.state.phase = StaticRoutePhase::Preparing;
                    }
                    Ok(StartStatus::Ready(RoutePrepareOutcome::Prepared(()))) => {
                        self.route.state.phase = StaticRoutePhase::Prepared;
                        self.expose = ExposeState::ReadyPendingActivation;
                    }
                    Ok(StartStatus::Ready(RoutePrepareOutcome::RejectedNoResource(error))) => {
                        return self.finish_rejected_route(error);
                    }
                    Err(rejection) => {
                        let (_, error) = rejection.into_parts();
                        return self.fail_binding_without_route(error);
                    }
                }
                progress(PendingWorkClass::BindingInput)
            }
        }
    }

    fn accept_request(
        &mut self,
        request: clinkz_wot_core::RouteInboundRequest,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let derived = self
            .derived
            .as_ref()
            .expect("serving retains route projection");
        if request.route() != &derived.key
            || request.target() != &derived.target
            || self.in_flight.is_some()
        {
            self.in_flight = Some(InFlightRecord {
                state: InFlightState::Response(RouteInboundResponse::failure(
                    request.into_response_opportunity(),
                    validation_error(self.thing_slot),
                )),
            });
            self.route.state.phase = StaticRoutePhase::Dispatching;
            return self.start_pending_response(budget);
        }
        self.in_flight = Some(InFlightRecord {
            state: InFlightState::Accepted(request),
        });
        self.route.state.phase = StaticRoutePhase::Dispatching;
        self.dispatch_pending(budget)
    }

    fn dispatch_pending(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        if budget.consume(WorkClass::HandlerSteps, 1).is_err() {
            return progress(PendingWorkClass::HandlerCall);
        }
        let request = match self.in_flight.take() {
            Some(InFlightRecord {
                state: InFlightState::Accepted(request),
            }) => request,
            state => {
                self.in_flight = state;
                return self.start_failed_shutdown_core(validation_error(self.thing_slot), budget);
            }
        };
        let derived = self
            .derived
            .as_ref()
            .expect("serving retains route projection");
        let (route, _correlation, target, input, opportunity) = request.into_parts();
        let context = match HandlerContext::try_new(
            &derived.thing_id,
            self.thing_slot,
            &target,
            Operation::ReadProperty,
            derived.plan_id,
            Some((route.binding_id(), route.binding_generation())),
        ) {
            Ok(context) => context,
            Err(error) => {
                self.in_flight = Some(InFlightRecord {
                    state: InFlightState::Response(RouteInboundResponse::failure(
                        opportunity,
                        error,
                    )),
                });
                return self.start_pending_response(budget);
            }
        };
        let result = self.handler.handler().handle(context, &input);
        let response = RouteInboundResponse::new(opportunity, result);
        debug_assert_eq!(response.opportunity().route(), &route);
        self.in_flight = Some(InFlightRecord {
            state: InFlightState::Response(response),
        });
        self.start_pending_response(budget)
    }

    fn start_pending_response(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        let response = match self.in_flight.take() {
            Some(InFlightRecord {
                state: InFlightState::Response(response),
            }) => response,
            state => {
                self.in_flight = state;
                return self.start_failed_shutdown_core(validation_error(self.thing_slot), budget);
            }
        };
        let route = *response.opportunity().route();
        let correlation = response.opportunity().correlation();
        match self.registration.server_mut().start_response(
            response,
            &mut self.route.state.response,
            budget,
        ) {
            Ok(StartStatus::Pending) => {
                self.in_flight = Some(InFlightRecord {
                    state: InFlightState::Delivering { route, correlation },
                });
                self.route.state.phase = StaticRoutePhase::Delivering;
            }
            Ok(StartStatus::Ready(BindingDeliveryOutcome::Delivered)) => {
                return self.finish_delivered_response(budget);
            }
            Ok(StartStatus::Ready(BindingDeliveryOutcome::Failed(error))) => {
                return self.finish_failed_response(error, budget);
            }
            Err(rejection) => {
                self.in_flight = Some(InFlightRecord {
                    state: InFlightState::Response(rejection.into_input()),
                });
                self.route.state.phase = StaticRoutePhase::Dispatching;
            }
        }
        progress(PendingWorkClass::ResponseDelivery)
    }

    fn retain_cleanup_outcome(&mut self, outcome: RouteCleanupOutcome) -> StepStatus<()> {
        self.cleanup_outcome = Some(outcome);
        self.route.state.phase = StaticRoutePhase::FinalizingCleanup;
        self.finalize_cleanup()
    }

    fn finalize_cleanup(&mut self) -> StepStatus<()> {
        if let Err(error) = self
            .registration
            .server_mut()
            .acknowledge_route(&mut self.route.state.route)
        {
            self.disposition.record_core_failure(error);
            self.route.state.phase = StaticRoutePhase::FinalizingCleanup;
            return progress(PendingWorkClass::Cleanup);
        }
        let outcome = self
            .cleanup_outcome
            .take()
            .expect("finalization retains a cleanup outcome");
        if let RouteCleanupOutcome::ResidualExternalState(record) = outcome {
            self.disposition.record_core_failure(CoreError::Cleanup(
                ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never)
                    .with_thing(self.thing_slot),
            ));
            self.disposition.retain_residual(record);
        }
        self.activation = None;
        self.plan_set.reclaim();
        self.route.state.phase = StaticRoutePhase::Closed;
        self.expose = self.disposition.terminal_expose_state();
        progress(PendingWorkClass::Cleanup)
    }

    fn start_route_cleanup(
        &mut self,
        operation: CleanupOperation,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        self.expose = ExposeState::CleanupPending;
        self.plan_set.state = CompiledPlanSetState::Draining;
        let cause = self.disposition.cleanup_cause(self.thing_slot);
        let phase = match self
            .admission
            .cleanup_context(self.thing_slot, operation, cause)
        {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.expose = ExposeState::Failed;
                self.plan_set.state = CompiledPlanSetState::Failed;
                return StepStatus::Terminal(());
            }
        };
        let status = if operation == CleanupOperation::AbortPreparedRoute {
            self.registration
                .server_mut()
                .start_abort(phase, &mut self.route.state.route, budget)
        } else {
            self.registration.server_mut().start_shutdown(
                phase,
                &mut self.route.state.route,
                budget,
            )
        };
        match status {
            StartStatus::Pending => {
                self.route.state.phase = if operation == CleanupOperation::AbortPreparedRoute {
                    StaticRoutePhase::Aborting
                } else {
                    StaticRoutePhase::Cleaning
                };
                progress(PendingWorkClass::RouteCleanup)
            }
            StartStatus::Ready(outcome) => self.retain_cleanup_outcome(outcome),
        }
    }

    fn start_cleanup(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
    }
}

impl<B, H> StaticServient for StaticPropertyReadServient<'_, B, H>
where
    B: PollServerBinding,
    H: ReadPropertyHandler,
{
    fn begin_destroy(&mut self) -> CoreResult<()> {
        match self.expose {
            ExposeState::Preparing if self.route.state.phase == StaticRoutePhase::Absent => {
                self.disposition.mark_cancelled_before_publication();
                self.cursor = None;
                self.td = None;
                self.plan_set.reclaim();
                self.route.state.phase = StaticRoutePhase::Closed;
                self.expose = ExposeState::Cancelled;
                Ok(())
            }
            ExposeState::Preparing
            | ExposeState::ReadyPendingActivation
            | ExposeState::Activating
            | ExposeState::Committing => {
                self.disposition.mark_cancelled_before_publication();
                if let Some(activation) = self.activation.as_mut() {
                    activation.close();
                }
                self.plan_set.state = CompiledPlanSetState::Draining;
                self.expose = ExposeState::Cancelling;
                Ok(())
            }
            ExposeState::Serving => {
                self.activation
                    .as_mut()
                    .expect("serving has activation authority")
                    .close();
                self.plan_set.state = CompiledPlanSetState::Draining;
                self.expose = ExposeState::Draining;
                Ok(())
            }
            ExposeState::Cancelling
            | ExposeState::Draining
            | ExposeState::CleanupPending
            | ExposeState::Cancelled
            | ExposeState::Destroyed => Ok(()),
            _ => Err(validation_error(self.thing_slot)),
        }
    }

    fn step(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()> {
        if self.plan_set.state == CompiledPlanSetState::Building {
            return self.step_planning(budget);
        }
        match self.route.state.phase {
            StaticRoutePhase::Absent => StepStatus::Idle,
            StaticRoutePhase::Preparing => match self.registration.server_mut().poll_prepare(
                cx,
                &mut self.route.state.route,
                budget,
            ) {
                Poll::Pending => progress(PendingWorkClass::BindingInput),
                Poll::Ready(RoutePrepareOutcome::Prepared(())) => {
                    if self.expose == ExposeState::Cancelling {
                        return self
                            .start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget);
                    }
                    self.route.state.phase = StaticRoutePhase::Prepared;
                    self.expose = ExposeState::ReadyPendingActivation;
                    progress(PendingWorkClass::RouteReadiness)
                }
                Poll::Ready(RoutePrepareOutcome::RejectedNoResource(error)) => {
                    self.finish_rejected_route(error)
                }
            },
            StaticRoutePhase::Prepared if self.expose == ExposeState::Cancelling => {
                self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
            }
            StaticRoutePhase::Prepared => match self.registration.server_mut().start_readiness(
                &mut self.route.state.route,
                &mut self.route.state.readiness,
                budget,
            ) {
                StartStatus::Pending => {
                    self.route.state.phase = StaticRoutePhase::AwaitingReadiness;
                    progress(PendingWorkClass::RouteReadiness)
                }
                StartStatus::Ready(RouteReadinessOutcome::Ready(())) => {
                    if !self.route.state.readiness.is_vacant() {
                        self.route.state.readiness.clear();
                    }
                    self.route.state.phase = StaticRoutePhase::Ready;
                    self.expose = ExposeState::Activating;
                    progress(PendingWorkClass::BindingInput)
                }
                StartStatus::Ready(RouteReadinessOutcome::Failed { error, .. }) => {
                    if !self.route.state.readiness.is_vacant() {
                        self.route.state.readiness.clear();
                    }
                    self.start_failed_abort(error, budget)
                }
            },
            StaticRoutePhase::AwaitingReadiness => {
                match self.registration.server_mut().poll_readiness(
                    cx,
                    &mut self.route.state.route,
                    &mut self.route.state.readiness,
                    budget,
                ) {
                    Poll::Pending => progress(PendingWorkClass::RouteReadiness),
                    Poll::Ready(RouteReadinessOutcome::Ready(())) => {
                        self.route.state.readiness.clear();
                        if self.expose == ExposeState::Cancelling {
                            return self
                                .start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget);
                        }
                        self.route.state.phase = StaticRoutePhase::Ready;
                        self.expose = ExposeState::Activating;
                        progress(PendingWorkClass::BindingInput)
                    }
                    Poll::Ready(RouteReadinessOutcome::Failed { error, .. }) => {
                        self.route.state.readiness.clear();
                        self.start_failed_abort(error, budget)
                    }
                }
            }
            StaticRoutePhase::Ready if self.expose == ExposeState::Cancelling => {
                self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
            }
            StaticRoutePhase::Ready => match self
                .registration
                .server_mut()
                .start_activate(&mut self.route.state.route, budget)
            {
                StartStatus::Pending => {
                    self.route.state.phase = StaticRoutePhase::Activating;
                    progress(PendingWorkClass::BindingInput)
                }
                StartStatus::Ready(RouteActivationOutcome::Active(())) => {
                    self.route.state.phase = StaticRoutePhase::Active;
                    self.expose = ExposeState::Committing;
                    progress(PendingWorkClass::BindingInput)
                }
                StartStatus::Ready(RouteActivationOutcome::NotActivated { error, .. }) => {
                    self.start_failed_abort(error, budget)
                }
            },
            StaticRoutePhase::Activating => match self.registration.server_mut().poll_activate(
                cx,
                &mut self.route.state.route,
                budget,
            ) {
                Poll::Pending => progress(PendingWorkClass::BindingInput),
                Poll::Ready(RouteActivationOutcome::Active(())) => {
                    if self.expose == ExposeState::Cancelling {
                        return self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget);
                    }
                    self.route.state.phase = StaticRoutePhase::Active;
                    self.expose = ExposeState::Committing;
                    progress(PendingWorkClass::BindingInput)
                }
                Poll::Ready(RouteActivationOutcome::NotActivated { error, .. }) => {
                    self.start_failed_abort(error, budget)
                }
            },
            StaticRoutePhase::Active if self.expose == ExposeState::Cancelling => {
                self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
            }
            StaticRoutePhase::Active => match self
                .registration
                .server_mut()
                .start_commit(&mut self.route.state.route, budget)
            {
                StartStatus::Pending => {
                    self.route.state.phase = StaticRoutePhase::Committing;
                    progress(PendingWorkClass::BindingInput)
                }
                StartStatus::Ready(RouteCommitOutcome::Committed(())) => {
                    self.route.state.phase = StaticRoutePhase::CommittedClosed;
                    progress(PendingWorkClass::BindingInput)
                }
                StartStatus::Ready(RouteCommitOutcome::NotCommitted { error, .. }) => {
                    self.start_failed_shutdown(error, budget)
                }
            },
            StaticRoutePhase::Committing => match self.registration.server_mut().poll_commit(
                cx,
                &mut self.route.state.route,
                budget,
            ) {
                Poll::Pending => progress(PendingWorkClass::BindingInput),
                Poll::Ready(RouteCommitOutcome::Committed(())) => {
                    if self.expose == ExposeState::Cancelling {
                        return self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget);
                    }
                    self.route.state.phase = StaticRoutePhase::CommittedClosed;
                    progress(PendingWorkClass::BindingInput)
                }
                Poll::Ready(RouteCommitOutcome::NotCommitted { error, .. }) => {
                    self.start_failed_shutdown(error, budget)
                }
            },
            StaticRoutePhase::CommittedClosed => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget);
                }
                self.activation
                    .as_mut()
                    .expect("route has authority")
                    .publish();
                self.plan_set.publish();
                self.route.state.phase = StaticRoutePhase::Serving;
                self.expose = ExposeState::Serving;
                progress(PendingWorkClass::BindingInput)
            }
            StaticRoutePhase::Serving => {
                if self.expose == ExposeState::Draining {
                    return self.start_cleanup(budget);
                }
                let activation = self.activation.as_mut().expect("serving has authority");
                if !activation.published {
                    return self
                        .start_failed_shutdown_core(validation_error(self.thing_slot), budget);
                }
                let permit = match activation.authority.claim_route(&mut activation.lease) {
                    Ok(claim) => claim.into_permit(),
                    Err(_) => {
                        return self
                            .start_failed_shutdown_core(validation_error(self.thing_slot), budget);
                    }
                };
                match self.registration.server_mut().poll_accept(
                    cx,
                    &mut self.route.state.route,
                    permit,
                    budget,
                ) {
                    Poll::Pending => StepStatus::Idle,
                    Poll::Ready(Ok(RouteAcceptEvent::Request(request))) => {
                        self.accept_request(request, budget)
                    }
                    Poll::Ready(Ok(RouteAcceptEvent::OperationalError(_))) => {
                        progress(PendingWorkClass::BindingInput)
                    }
                    Poll::Ready(Ok(RouteAcceptEvent::Terminal(RouteTerminal::Failed {
                        error,
                        ..
                    }))) => self.start_failed_shutdown(error, budget),
                    Poll::Ready(Ok(RouteAcceptEvent::Terminal(RouteTerminal::Closed {
                        ..
                    }))) => self.start_failed_shutdown_core(
                        CoreError::Lifecycle(
                            ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
                                .with_thing(self.thing_slot),
                        ),
                        budget,
                    ),
                    Poll::Ready(Err(error)) => self.start_failed_shutdown_core(error, budget),
                }
            }
            StaticRoutePhase::Dispatching => match self.in_flight.as_ref() {
                Some(InFlightRecord {
                    state: InFlightState::Accepted(_),
                }) => self.dispatch_pending(budget),
                Some(InFlightRecord {
                    state: InFlightState::Response(_),
                }) => self.start_pending_response(budget),
                _ => self.start_failed_shutdown_core(validation_error(self.thing_slot), budget),
            },
            StaticRoutePhase::Delivering => match self.registration.server_mut().poll_response(
                cx,
                &mut self.route.state.response,
                budget,
            ) {
                Poll::Pending => progress(PendingWorkClass::ResponseDelivery),
                Poll::Ready(BindingDeliveryOutcome::Delivered) => {
                    if let Some(InFlightRecord {
                        state: InFlightState::Delivering { route, correlation },
                    }) = self.in_flight.as_ref()
                    {
                        let opportunity = self.route.state.response.response().opportunity();
                        debug_assert_eq!(route, opportunity.route());
                        debug_assert_eq!(*correlation, opportunity.correlation());
                    } else {
                        self.disposition
                            .record_core_failure(validation_error(self.thing_slot));
                    }
                    self.finish_delivered_response(budget)
                }
                Poll::Ready(BindingDeliveryOutcome::Failed(error)) => {
                    self.finish_failed_response(error, budget)
                }
            },
            StaticRoutePhase::DeliveredResponse => self.finish_delivered_response(budget),
            StaticRoutePhase::FailedResponse => self.retry_failed_response(budget),
            StaticRoutePhase::Aborting => match self.registration.server_mut().poll_abort(
                cx,
                &mut self.route.state.route,
                budget,
            ) {
                Poll::Pending => progress(PendingWorkClass::RouteCleanup),
                Poll::Ready(outcome) => self.retain_cleanup_outcome(outcome),
            },
            StaticRoutePhase::Cleaning => match self.registration.server_mut().poll_shutdown(
                cx,
                &mut self.route.state.route,
                budget,
            ) {
                Poll::Pending => progress(PendingWorkClass::RouteCleanup),
                Poll::Ready(outcome) => self.retain_cleanup_outcome(outcome),
            },
            StaticRoutePhase::FinalizingCleanup => self.finalize_cleanup(),
            StaticRoutePhase::Closed => StepStatus::Idle,
        }
    }
}

#[cfg(feature = "std")]
type HostCursor = PropertyReadBuildCursor<HostBindingCompilerCursor, HostBindingArtifact>;

#[cfg(feature = "std")]
type HostPrepareCall =
    HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>;
#[cfg(feature = "std")]
type HostReadinessCall =
    HostBindingCallBox<RouteReadinessOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>;
#[cfg(feature = "std")]
type HostActivationCall = HostBindingCallBox<
    RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
    HostRouteCleanupSuccessor,
>;
#[cfg(feature = "std")]
type HostCommitCall = HostBindingCallBox<
    RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
    HostRouteCleanupSuccessor,
>;
#[cfg(feature = "std")]
type HostCleanupCall = HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>;

#[cfg(feature = "std")]
enum HostRouteState {
    Absent,
    Preparing(HostPrepareCall),
    Prepared(HostPreparedRouteGuard),
    AwaitingReadiness(HostReadinessCall),
    Ready(HostPreparedRouteGuard),
    Activating(HostActivationCall),
    Active(HostActiveRouteGuard),
    Committing(HostCommitCall),
    CommittedClosed(HostCommittedRouteGuard),
    Serving(HostCommittedRouteGuard),
    Dispatching(HostCommittedRouteGuard),
    Delivering {
        guard: HostCommittedRouteGuard,
        call: HostBindingCallBox<BindingDeliveryOutcome>,
    },
    AbortPending(RouteAbortInput),
    ShutdownPending(RouteShutdownInput),
    Cleaning(HostCleanupCall),
    Closed,
}

#[cfg(feature = "std")]
pub(crate) struct HostPropertyReadConfig {
    limits: ResourceLimits,
    registration: HostBindingRegistration,
}

#[cfg(feature = "std")]
impl HostPropertyReadConfig {
    pub(crate) fn new(limits: ResourceLimits, registration: HostBindingRegistration) -> Self {
        Self {
            limits,
            registration,
        }
    }
}

#[cfg(feature = "std")]
struct HostPropertyReadRuntime {
    td: Option<Thing>,
    thing_slot: ThingSlotId,
    limits: ResourceLimits,
    registration: HostBindingRegistration,
    handler: Option<PropertyReadHandlerRecord>,
    expose: ExposeState,
    plan_set: CompiledPlanSetRecord<HostBindingArtifact>,
    cursor: Option<HostCursor>,
    derived: Option<DerivedRoute>,
    route: BindingRouteRecord<HostRouteState>,
    activation: Option<ServingActivationRecord>,
    in_flight: Option<InFlightRecord>,
    admission: AdmissionReservations,
    disposition: LifecycleDisposition,
}

#[cfg(feature = "std")]
impl HostPropertyReadRuntime {
    fn new(td: Thing, thing_slot: ThingSlotId, config: HostPropertyReadConfig) -> CoreResult<Self> {
        let admission = AdmissionReservations::new(
            &config.limits,
            thing_slot,
            config.registration.resources().route_state(),
            config.registration.status().retained_records(),
            Deadline::NONE,
        )?;
        Ok(Self {
            td: Some(td),
            thing_slot,
            limits: config.limits,
            registration: config.registration,
            handler: None,
            expose: ExposeState::Draft,
            plan_set: CompiledPlanSetRecord::building(),
            cursor: None,
            derived: None,
            route: BindingRouteRecord {
                key: None,
                state: HostRouteState::Absent,
            },
            activation: None,
            in_flight: None,
            admission,
            disposition: LifecycleDisposition::new(),
        })
    }

    fn set_read_property_handler<H>(
        &mut self,
        name: impl Into<Box<str>>,
        handler: H,
        footprint: HandlerFootprint,
    ) -> CoreResult<()>
    where
        H: ReadPropertyHandler + Send + Sync + 'static,
    {
        if self.expose != ExposeState::Draft {
            return Err(validation_error(self.thing_slot));
        }
        validate_handler_footprint(&self.limits, self.thing_slot, footprint)?;
        self.handler = Some(PropertyReadHandlerRecord {
            target: name.into(),
            handler: Box::new(handler),
            footprint,
        });
        Ok(())
    }

    fn begin_expose(&mut self) -> CoreResult<()> {
        if self.expose != ExposeState::Draft || self.handler.is_none() {
            return Err(validation_error(self.thing_slot));
        }
        self.expose = ExposeState::Preparing;
        Ok(())
    }

    fn begin_destroy(&mut self) -> CoreResult<()> {
        match self.expose {
            ExposeState::Draft => {
                self.expose = ExposeState::Destroyed;
                self.plan_set.reclaim();
                self.route.state = HostRouteState::Closed;
            }
            ExposeState::Preparing if matches!(&self.route.state, HostRouteState::Absent) => {
                self.disposition.mark_cancelled_before_publication();
                self.cursor = None;
                self.td = None;
                self.plan_set.reclaim();
                self.route.state = HostRouteState::Closed;
                self.expose = ExposeState::Cancelled;
            }
            ExposeState::Preparing
            | ExposeState::ReadyPendingActivation
            | ExposeState::Activating
            | ExposeState::Committing => {
                self.disposition.mark_cancelled_before_publication();
                if let Some(activation) = self.activation.as_mut() {
                    activation.close();
                }
                self.plan_set.state = CompiledPlanSetState::Draining;
                self.expose = ExposeState::Cancelling;
            }
            ExposeState::Serving => {
                self.activation
                    .as_mut()
                    .expect("serving has activation authority")
                    .close();
                self.plan_set.state = CompiledPlanSetState::Draining;
                self.expose = ExposeState::Draining;
            }
            ExposeState::Cancelling
            | ExposeState::Draining
            | ExposeState::CleanupPending
            | ExposeState::Cancelled
            | ExposeState::Destroyed => {}
            _ => return Err(validation_error(self.thing_slot)),
        }
        Ok(())
    }

    fn fail_without_route(&mut self, error: CoreError) -> StepStatus<()> {
        self.disposition.record_core_failure(error);
        self.plan_set.reclaim();
        self.route.state = HostRouteState::Closed;
        self.expose = ExposeState::Failed;
        StepStatus::Terminal(())
    }

    fn fail_binding_without_route(&mut self, error: BindingOperationalError) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.plan_set.reclaim();
        self.route.state = HostRouteState::Closed;
        self.expose = ExposeState::Failed;
        StepStatus::Terminal(())
    }

    fn finish_cleanup(&mut self, outcome: RouteCleanupOutcome) -> StepStatus<()> {
        if let RouteCleanupOutcome::ResidualExternalState(record) = outcome {
            self.disposition.record_core_failure(CoreError::Cleanup(
                ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never)
                    .with_thing(self.thing_slot),
            ));
            self.disposition.retain_residual(record);
        }
        self.route.state = HostRouteState::Closed;
        self.activation = None;
        self.plan_set.reclaim();
        self.expose = self.disposition.terminal_expose_state();
        progress(PendingWorkClass::Cleanup)
    }

    fn submit_abort(&mut self, input: RouteAbortInput) -> StepStatus<()> {
        match self.registration.server().abort(input) {
            Ok(call) => {
                self.route.state = HostRouteState::Cleaning(call);
                progress(PendingWorkClass::RouteCleanup)
            }
            Err(rejection) => {
                self.route.state = HostRouteState::AbortPending(rejection.into_input());
                progress(PendingWorkClass::RouteCleanup)
            }
        }
    }

    fn submit_shutdown(&mut self, input: RouteShutdownInput) -> StepStatus<()> {
        match self.registration.server().shutdown(input) {
            Ok(call) => {
                self.route.state = HostRouteState::Cleaning(call);
                progress(PendingWorkClass::RouteCleanup)
            }
            Err(rejection) => {
                self.route.state = HostRouteState::ShutdownPending(rejection.into_input());
                progress(PendingWorkClass::RouteCleanup)
            }
        }
    }

    fn start_abort(&mut self, guard: HostPreparedRouteGuard) -> StepStatus<()> {
        self.expose = ExposeState::CleanupPending;
        self.plan_set.state = CompiledPlanSetState::Draining;
        let cause = self.disposition.cleanup_cause(self.thing_slot);
        let phase = match self.admission.cleanup_context(
            self.thing_slot,
            CleanupOperation::AbortPreparedRoute,
            cause,
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.route.state = HostRouteState::Prepared(guard);
                self.disposition.record_core_failure(error);
                self.expose = ExposeState::Failed;
                self.plan_set.state = CompiledPlanSetState::Failed;
                return StepStatus::Terminal(());
            }
        };
        self.submit_abort(RouteAbortInput::new(guard, phase))
    }

    fn start_shutdown(&mut self, guard: HostShutdownRouteGuard) -> StepStatus<()> {
        self.expose = ExposeState::CleanupPending;
        self.plan_set.state = CompiledPlanSetState::Draining;
        let cause = self.disposition.cleanup_cause(self.thing_slot);
        let phase = match self.admission.cleanup_context(
            self.thing_slot,
            CleanupOperation::ShutdownRoute,
            cause,
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.expose = ExposeState::Failed;
                self.plan_set.state = CompiledPlanSetState::Failed;
                self.route.state = match guard {
                    HostShutdownRouteGuard::Active(guard) => HostRouteState::Active(guard),
                    HostShutdownRouteGuard::Committed(guard) => HostRouteState::Serving(guard),
                };
                return StepStatus::Terminal(());
            }
        };
        self.submit_shutdown(RouteShutdownInput::new(guard, phase))
    }

    fn start_failed_abort(
        &mut self,
        guard: HostPreparedRouteGuard,
        error: BindingOperationalError,
    ) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.start_abort(guard)
    }

    fn start_failed_shutdown(
        &mut self,
        guard: HostShutdownRouteGuard,
        error: BindingOperationalError,
    ) -> StepStatus<()> {
        self.disposition.record_binding_failure(error);
        self.start_shutdown(guard)
    }

    fn start_failed_shutdown_core(
        &mut self,
        guard: HostShutdownRouteGuard,
        error: CoreError,
    ) -> StepStatus<()> {
        self.disposition.record_core_failure(error);
        self.start_shutdown(guard)
    }

    fn step_planning(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        let td = self.td.as_ref().expect("building retains the TD");
        let plan_id = PlanId::new(self.thing_slot.slot(), self.thing_slot.generation());
        let generation = PlanSetGeneration::new(self.thing_slot.generation());
        let compiler =
            PropertyReadPlanCompiler::producer_route(plan_id, self.registration.identity(), 0, 0);
        let registrations = core::slice::from_ref(self.registration.compiler());
        let input = clinkz_wot_planning::PlanBuildInput::new(td, registrations, generation);
        let cursor = match self.cursor.take() {
            Some(cursor) => cursor,
            None => match compiler.start(&input) {
                Ok(cursor) => cursor,
                Err(error) => return self.fail_without_route(error),
            },
        };
        match compiler.step(&input, cursor, budget) {
            PlanBuildStep::Pending(cursor) => {
                self.cursor = Some(cursor);
                progress(PendingWorkClass::BindingInput)
            }
            PlanBuildStep::Failed(failure) => {
                let (error, _) = failure.into_parts();
                self.fail_without_route(error)
            }
            PlanBuildStep::Complete(output) => {
                let derived =
                    match derive_route(&output, self.registration.identity(), self.thing_slot) {
                        Ok(derived) => derived,
                        Err(error) => return self.fail_without_route(error),
                    };
                if self.handler.as_ref().map(|record| &*record.target) != derived.target.name() {
                    return self.fail_without_route(validation_error(self.thing_slot));
                }
                let authority = ServingActivationAuthority::new(
                    derived.thing_id.clone(),
                    self.thing_slot.generation(),
                    generation,
                );
                let prepare = PrepareInput::new(
                    derived.key,
                    derived.artifact_ref,
                    self.registration.resources().route_state(),
                );
                self.plan_set.freeze(output, generation);
                let artifact = match self.plan_set.resolve_prepare_artifact(
                    &prepare,
                    self.registration.identity(),
                    self.thing_slot,
                    self.registration.resources().route_state(),
                ) {
                    Ok(artifact) => artifact,
                    Err(error) => return self.fail_without_route(error),
                };
                self.activation = Some(ServingActivationRecord::new(authority, derived.key));
                self.route.key = Some(derived.key);
                self.td = None;
                self.derived = Some(derived);
                let call = match self.registration.server().prepare(prepare, artifact) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (_, error) = rejection.into_parts();
                        return self.fail_binding_without_route(error);
                    }
                };
                self.route.state = HostRouteState::Preparing(call);
                progress(PendingWorkClass::BindingInput)
            }
        }
    }

    fn accept_request(
        &mut self,
        guard: HostCommittedRouteGuard,
        request: clinkz_wot_core::RouteInboundRequest,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let derived = self
            .derived
            .as_ref()
            .expect("serving retains route projection");
        if request.route() != &derived.key
            || request.target() != &derived.target
            || self.in_flight.is_some()
        {
            let error = validation_error(self.thing_slot);
            self.in_flight = Some(InFlightRecord {
                state: InFlightState::Response(RouteInboundResponse::failure(
                    request.into_response_opportunity(),
                    error,
                )),
            });
            return self.start_pending_response(guard);
        }
        self.in_flight = Some(InFlightRecord {
            state: InFlightState::Accepted(request),
        });
        self.dispatch_pending(guard, budget)
    }

    fn dispatch_pending(
        &mut self,
        guard: HostCommittedRouteGuard,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        if budget.consume(WorkClass::HandlerSteps, 1).is_err() {
            self.route.state = HostRouteState::Dispatching(guard);
            return progress(PendingWorkClass::HandlerCall);
        }
        let request = match self.in_flight.take() {
            Some(InFlightRecord {
                state: InFlightState::Accepted(request),
            }) => request,
            state => {
                self.in_flight = state;
                return self.start_failed_shutdown_core(
                    HostShutdownRouteGuard::Committed(guard),
                    validation_error(self.thing_slot),
                );
            }
        };
        let derived = self
            .derived
            .as_ref()
            .expect("serving retains route projection");
        let (route, _correlation, target, input, opportunity) = request.into_parts();
        let context = match HandlerContext::try_new(
            &derived.thing_id,
            self.thing_slot,
            &target,
            Operation::ReadProperty,
            derived.plan_id,
            Some((route.binding_id(), route.binding_generation())),
        ) {
            Ok(context) => context,
            Err(error) => {
                self.in_flight = Some(InFlightRecord {
                    state: InFlightState::Response(RouteInboundResponse::failure(
                        opportunity,
                        error,
                    )),
                });
                return self.start_pending_response(guard);
            }
        };
        let handler = self.handler.as_ref().expect("exposure freezes a handler");
        debug_assert!(
            handler.footprint.retained_bytes() > 0 || handler.footprint.pending_call_bytes() == 0
        );
        let result = handler.handler.handle(context, &input);
        let response = RouteInboundResponse::new(opportunity, result);
        debug_assert_eq!(response.opportunity().route(), &route);
        self.in_flight = Some(InFlightRecord {
            state: InFlightState::Response(response),
        });
        self.start_pending_response(guard)
    }

    fn start_pending_response(&mut self, guard: HostCommittedRouteGuard) -> StepStatus<()> {
        let response = match self.in_flight.take() {
            Some(InFlightRecord {
                state: InFlightState::Response(response),
            }) => response,
            state => {
                self.in_flight = state;
                return self.start_failed_shutdown_core(
                    HostShutdownRouteGuard::Committed(guard),
                    validation_error(self.thing_slot),
                );
            }
        };
        let route = *response.opportunity().route();
        let correlation = response.opportunity().correlation();
        let call = match self.registration.server().deliver_response(response) {
            Ok(call) => call,
            Err(rejection) => {
                self.in_flight = Some(InFlightRecord {
                    state: InFlightState::Response(rejection.into_input()),
                });
                self.route.state = HostRouteState::Dispatching(guard);
                return progress(PendingWorkClass::ResponseDelivery);
            }
        };
        self.in_flight = Some(InFlightRecord {
            state: InFlightState::Delivering { route, correlation },
        });
        self.route.state = HostRouteState::Delivering { guard, call };
        progress(PendingWorkClass::ResponseDelivery)
    }

    fn step(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()> {
        if self.expose == ExposeState::Preparing
            && self.plan_set.state == CompiledPlanSetState::Building
        {
            return self.step_planning(budget);
        }
        let state = core::mem::replace(&mut self.route.state, HostRouteState::Absent);
        match state {
            HostRouteState::Absent => {
                self.route.state = HostRouteState::Absent;
                StepStatus::Idle
            }
            HostRouteState::Preparing(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::Preparing(call);
                        progress(PendingWorkClass::BindingInput)
                    }
                    Poll::Ready(RoutePrepareOutcome::Prepared(guard)) => {
                        if self.expose == ExposeState::Cancelling {
                            return self.start_abort(guard);
                        }
                        self.route.state = HostRouteState::Prepared(guard);
                        self.expose = ExposeState::ReadyPendingActivation;
                        progress(PendingWorkClass::RouteReadiness)
                    }
                    Poll::Ready(RoutePrepareOutcome::RejectedNoResource(error)) => {
                        self.fail_binding_without_route(error)
                    }
                }
            }
            HostRouteState::Prepared(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_abort(guard);
                }
                let call = match self.registration.server().start_readiness(guard) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (guard, error) = rejection.into_parts();
                        return self.start_failed_abort(guard, error);
                    }
                };
                self.route.state = HostRouteState::AwaitingReadiness(call);
                progress(PendingWorkClass::RouteReadiness)
            }
            HostRouteState::AwaitingReadiness(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::AwaitingReadiness(call);
                        progress(PendingWorkClass::RouteReadiness)
                    }
                    Poll::Ready(RouteReadinessOutcome::Ready(guard)) => {
                        if self.expose == ExposeState::Cancelling {
                            return self.start_abort(guard);
                        }
                        self.route.state = HostRouteState::Ready(guard);
                        self.expose = ExposeState::Activating;
                        progress(PendingWorkClass::BindingInput)
                    }
                    Poll::Ready(RouteReadinessOutcome::Failed { guard, error }) => {
                        self.start_failed_abort(guard, error)
                    }
                }
            }
            HostRouteState::Ready(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_abort(guard);
                }
                let call = match self.registration.server().activate(guard) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (guard, error) = rejection.into_parts();
                        return self.start_failed_abort(guard, error);
                    }
                };
                self.route.state = HostRouteState::Activating(call);
                progress(PendingWorkClass::BindingInput)
            }
            HostRouteState::Activating(mut call) => match call.as_pin_mut().poll_result(cx, budget)
            {
                Poll::Pending => {
                    self.route.state = HostRouteState::Activating(call);
                    progress(PendingWorkClass::BindingInput)
                }
                Poll::Ready(RouteActivationOutcome::Active(guard)) => {
                    if self.expose == ExposeState::Cancelling {
                        return self.start_shutdown(HostShutdownRouteGuard::Active(guard));
                    }
                    self.route.state = HostRouteState::Active(guard);
                    self.expose = ExposeState::Committing;
                    progress(PendingWorkClass::BindingInput)
                }
                Poll::Ready(RouteActivationOutcome::NotActivated { guard, error }) => {
                    self.start_failed_abort(guard, error)
                }
            },
            HostRouteState::Active(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_shutdown(HostShutdownRouteGuard::Active(guard));
                }
                let call = match self.registration.server().commit(guard) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (guard, error) = rejection.into_parts();
                        return self
                            .start_failed_shutdown(HostShutdownRouteGuard::Active(guard), error);
                    }
                };
                self.route.state = HostRouteState::Committing(call);
                progress(PendingWorkClass::BindingInput)
            }
            HostRouteState::Committing(mut call) => match call.as_pin_mut().poll_result(cx, budget)
            {
                Poll::Pending => {
                    self.route.state = HostRouteState::Committing(call);
                    progress(PendingWorkClass::BindingInput)
                }
                Poll::Ready(RouteCommitOutcome::Committed(guard)) => {
                    if self.expose == ExposeState::Cancelling {
                        return self.start_shutdown(HostShutdownRouteGuard::Committed(guard));
                    }
                    self.route.state = HostRouteState::CommittedClosed(guard);
                    progress(PendingWorkClass::BindingInput)
                }
                Poll::Ready(RouteCommitOutcome::NotCommitted { guard, error }) => {
                    self.start_failed_shutdown(HostShutdownRouteGuard::Active(guard), error)
                }
            },
            HostRouteState::CommittedClosed(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_shutdown(HostShutdownRouteGuard::Committed(guard));
                }
                self.activation
                    .as_mut()
                    .expect("route has authority")
                    .publish();
                self.plan_set.publish();
                self.expose = ExposeState::Serving;
                self.route.state = HostRouteState::Serving(guard);
                progress(PendingWorkClass::BindingInput)
            }
            HostRouteState::Serving(guard) => {
                if self.expose == ExposeState::Draining {
                    return self.start_shutdown(HostShutdownRouteGuard::Committed(guard));
                }
                let activation = self.activation.as_mut().expect("serving has authority");
                if !activation.published {
                    return self.start_failed_shutdown_core(
                        HostShutdownRouteGuard::Committed(guard),
                        validation_error(self.thing_slot),
                    );
                }
                let permit = match activation.authority.claim_route(&mut activation.lease) {
                    Ok(claim) => claim.into_permit(),
                    Err(_) => {
                        return self.start_failed_shutdown_core(
                            HostShutdownRouteGuard::Committed(guard),
                            validation_error(self.thing_slot),
                        );
                    }
                };
                match self
                    .registration
                    .server()
                    .poll_accept(&guard, permit, cx, budget)
                {
                    Poll::Pending => {
                        self.route.state = HostRouteState::Serving(guard);
                        StepStatus::Idle
                    }
                    Poll::Ready(Ok(RouteAcceptEvent::Request(request))) => {
                        self.accept_request(guard, request, budget)
                    }
                    Poll::Ready(Ok(RouteAcceptEvent::OperationalError(_))) => {
                        self.route.state = HostRouteState::Serving(guard);
                        progress(PendingWorkClass::BindingInput)
                    }
                    Poll::Ready(Ok(RouteAcceptEvent::Terminal(RouteTerminal::Failed {
                        error,
                        ..
                    }))) => {
                        self.start_failed_shutdown(HostShutdownRouteGuard::Committed(guard), error)
                    }
                    Poll::Ready(Ok(RouteAcceptEvent::Terminal(RouteTerminal::Closed {
                        ..
                    }))) => self.start_failed_shutdown_core(
                        HostShutdownRouteGuard::Committed(guard),
                        CoreError::Lifecycle(
                            ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
                                .with_thing(self.thing_slot),
                        ),
                    ),
                    Poll::Ready(Err(error)) => self.start_failed_shutdown_core(
                        HostShutdownRouteGuard::Committed(guard),
                        error,
                    ),
                }
            }
            HostRouteState::Dispatching(guard) => {
                if matches!(
                    self.in_flight.as_ref(),
                    Some(InFlightRecord {
                        state: InFlightState::Accepted(_),
                    })
                ) {
                    self.dispatch_pending(guard, budget)
                } else if matches!(
                    self.in_flight.as_ref(),
                    Some(InFlightRecord {
                        state: InFlightState::Response(_),
                    })
                ) {
                    self.start_pending_response(guard)
                } else {
                    self.start_failed_shutdown_core(
                        HostShutdownRouteGuard::Committed(guard),
                        validation_error(self.thing_slot),
                    )
                }
            }
            HostRouteState::Delivering { guard, mut call } => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::Delivering { guard, call };
                        progress(PendingWorkClass::ResponseDelivery)
                    }
                    Poll::Ready(BindingDeliveryOutcome::Delivered) => {
                        if let Some(InFlightRecord {
                            state: InFlightState::Delivering { route, correlation },
                        }) = self.in_flight.take()
                        {
                            debug_assert_eq!(route, *guard.route());
                            debug_assert!(!correlation.is_empty());
                        } else {
                            return self.start_failed_shutdown_core(
                                HostShutdownRouteGuard::Committed(guard),
                                validation_error(self.thing_slot),
                            );
                        }
                        self.route.state = HostRouteState::Serving(guard);
                        progress(PendingWorkClass::BindingInput)
                    }
                    Poll::Ready(BindingDeliveryOutcome::Failed(error)) => {
                        self.in_flight = None;
                        self.start_failed_shutdown(HostShutdownRouteGuard::Committed(guard), error)
                    }
                }
            }
            HostRouteState::AbortPending(input) => self.submit_abort(input),
            HostRouteState::ShutdownPending(input) => self.submit_shutdown(input),
            HostRouteState::Cleaning(mut call) => match call.as_pin_mut().poll_result(cx, budget) {
                Poll::Pending => {
                    self.route.state = HostRouteState::Cleaning(call);
                    progress(PendingWorkClass::RouteCleanup)
                }
                Poll::Ready(outcome) => self.finish_cleanup(outcome),
            },
            HostRouteState::Closed => {
                self.route.state = HostRouteState::Closed;
                StepStatus::Idle
            }
        }
    }
}

#[cfg(feature = "std")]
struct HostPropertyReadOwnerInner {
    config: std::sync::Mutex<Option<HostPropertyReadConfig>>,
    runtime: std::sync::Mutex<Option<HostPropertyReadRuntime>>,
}

#[cfg(feature = "std")]
#[derive(Clone)]
pub(crate) struct HostPropertyReadOwner {
    inner: Arc<HostPropertyReadOwnerInner>,
}

#[cfg(feature = "std")]
impl HostPropertyReadOwner {
    pub(crate) fn new(config: HostPropertyReadConfig) -> Self {
        Self {
            inner: Arc::new(HostPropertyReadOwnerInner {
                config: std::sync::Mutex::new(Some(config)),
                runtime: std::sync::Mutex::new(None),
            }),
        }
    }

    pub(crate) fn install(&self, td: Thing, thing_slot: ThingSlotId) -> CoreResult<()> {
        let config = self
            .inner
            .config
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .ok_or_else(|| validation_error(thing_slot))?;
        let runtime = HostPropertyReadRuntime::new(td, thing_slot, config)?;
        let mut slot = self
            .inner
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.is_some() {
            return Err(validation_error(thing_slot));
        }
        *slot = Some(runtime);
        Ok(())
    }

    fn with_runtime<R>(
        &self,
        operation: impl FnOnce(&mut HostPropertyReadRuntime) -> R,
    ) -> Option<R> {
        let mut runtime = self
            .inner
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()?;
        let result = operation(&mut runtime);
        *self
            .inner
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(runtime);
        Some(result)
    }

    pub(crate) fn set_read_property_handler<H>(
        &self,
        name: impl Into<Box<str>>,
        handler: H,
        footprint: HandlerFootprint,
    ) -> CoreResult<()>
    where
        H: ReadPropertyHandler + Send + Sync + 'static,
    {
        self.with_runtime(|runtime| runtime.set_read_property_handler(name, handler, footprint))
            .unwrap_or_else(|| {
                Err(CoreError::Backpressure(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Safe,
                )))
            })
    }

    pub(crate) fn begin_expose(&self) -> CoreResult<()> {
        self.with_runtime(HostPropertyReadRuntime::begin_expose)
            .unwrap_or_else(|| {
                Err(CoreError::Backpressure(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Safe,
                )))
            })
    }

    pub(crate) fn begin_destroy(&self) -> CoreResult<()> {
        self.with_runtime(HostPropertyReadRuntime::begin_destroy)
            .unwrap_or_else(|| {
                Err(CoreError::Backpressure(ErrorContext::new(
                    ErrorPhase::Cleanup,
                    RetryClass::Safe,
                )))
            })
    }

    pub(crate) fn step(&self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()> {
        self.with_runtime(|runtime| runtime.step(cx, budget))
            .unwrap_or(StepStatus::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{boxed::Box, vec};
    use clinkz_wot_core::binding::BindingRouteKey;
    use clinkz_wot_core::{
        BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint,
        BindingArtifactIdentity, BindingConfigurationDigest, BindingGeneration, BindingId,
        CollisionDomainId, EndpointReservationKey, LogicalInteractionPlan,
        RouteReservationIdentity,
    };
    use clinkz_wot_foundation::Generation;

    fn next_generation() -> Generation {
        Generation::INITIAL.checked_next().expect("next generation")
    }

    fn thing_slot() -> ThingSlotId {
        ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL)
    }

    fn plan_id() -> PlanId {
        PlanId::new(SlotIndex::new(0), Generation::INITIAL)
    }

    fn other_plan_id() -> PlanId {
        PlanId::new(SlotIndex::new(1), Generation::INITIAL)
    }

    fn plan_set_generation() -> PlanSetGeneration {
        PlanSetGeneration::new(Generation::INITIAL)
    }

    fn other_plan_set_generation() -> PlanSetGeneration {
        PlanSetGeneration::new(next_generation())
    }

    fn compatibility() -> BindingArtifactCompatibility {
        BindingArtifactCompatibility::new([0x81; 16])
    }

    fn configuration() -> BindingConfigurationDigest {
        BindingConfigurationDigest::new([0x82; 32])
    }

    fn reservation() -> RouteReservationIdentity {
        RouteReservationIdentity::new(
            CollisionDomainId::new([0x83; 16]),
            EndpointReservationKey::new([0x84; 32]),
        )
    }

    fn registration() -> BindingRegistrationIdentity {
        BindingRegistrationIdentity::new(
            BindingId::new(7),
            BindingGeneration::INITIAL,
            configuration(),
            compatibility(),
            0,
        )
    }

    fn identity(role: BindingArtifactRole) -> BindingArtifactIdentity {
        BindingArtifactIdentity::new(
            plan_set_generation(),
            plan_id(),
            registration().binding_id(),
            registration().binding_generation(),
            configuration(),
            compatibility(),
            role,
        )
    }

    fn route() -> BindingRouteKey {
        BindingRouteKey::new(
            registration().binding_id(),
            registration().binding_generation(),
            Generation::INITIAL,
            plan_set_generation(),
            plan_id(),
            reservation(),
        )
    }

    fn footprint() -> BindingLifetimeFootprint {
        BindingLifetimeFootprint::new(2, 128)
    }

    fn plan(id: PlanId) -> LogicalInteractionPlan {
        LogicalInteractionPlan::try_property_read(
            id,
            ThingId::from("urn:test:prepare-artifact"),
            Box::from("level"),
            0,
            Box::from("mock://tank/level"),
            None,
            None,
        )
        .expect("valid logical plan")
    }

    fn record(
        artifact_identity: BindingArtifactIdentity,
        logical_plan_id: PlanId,
        stored_reference: BindingArtifactRef,
        frozen_generation: PlanSetGeneration,
    ) -> CompiledPlanSetRecord<u8> {
        let artifact_footprint = BindingArtifactFootprint::new(1, 1);
        let artifact = if artifact_identity.role() == BindingArtifactRole::ProducerRoute {
            BindingArtifact::producer_route(
                artifact_identity.compatibility(),
                artifact_footprint,
                reservation(),
                17,
            )
        } else {
            BindingArtifact::new(artifact_identity.compatibility(), artifact_footprint, 17)
        };
        let envelope =
            BindingArtifactEnvelope::try_new(artifact_identity, artifact_footprint, artifact)
                .expect("admitted fixture artifact");
        let output = PlanBuildOutput::new(
            vec![plan(logical_plan_id)],
            vec![envelope],
            vec![stored_reference],
        );
        let mut record = CompiledPlanSetRecord::building();
        record.freeze(output, frozen_generation);
        record
    }

    fn base_record() -> CompiledPlanSetRecord<u8> {
        let artifact_identity = identity(BindingArtifactRole::ProducerRoute);
        record(
            artifact_identity,
            plan_id(),
            BindingArtifactRef::new(artifact_identity, SlotIndex::new(0)),
            plan_set_generation(),
        )
    }

    fn prepare(reference: BindingArtifactRef, route: BindingRouteKey) -> PrepareInput {
        PrepareInput::new(route, reference, footprint())
    }

    fn base_prepare() -> PrepareInput {
        let artifact_identity = identity(BindingArtifactRole::ProducerRoute);
        prepare(
            BindingArtifactRef::new(artifact_identity, SlotIndex::new(0)),
            route(),
        )
    }

    fn assert_rejected(
        record: &CompiledPlanSetRecord<u8>,
        input: &PrepareInput,
        registration: BindingRegistrationIdentity,
    ) {
        assert!(
            record
                .resolve_prepare_artifact(input, registration, thing_slot(), footprint())
                .is_err()
        );
    }

    #[test]
    fn prepare_artifact_resolution_accepts_only_the_exact_frozen_envelope() {
        let record = base_record();
        let input = base_prepare();
        let envelope = record
            .resolve_prepare_artifact(&input, registration(), thing_slot(), footprint())
            .expect("exact frozen Producer-route envelope");
        assert_eq!(envelope.identity(), input.artifact().identity());
        assert_eq!(
            envelope.route_reservation(),
            Some(input.route().reservation())
        );
    }

    #[test]
    fn prepare_artifact_resolution_rejects_ref_route_and_registration_mutations() {
        let base_identity = identity(BindingArtifactRole::ProducerRoute);

        let bad_slot = prepare(
            BindingArtifactRef::new(base_identity, SlotIndex::new(1)),
            route(),
        );
        assert_rejected(&base_record(), &bad_slot, registration());

        let wrong_ref_identity = BindingArtifactIdentity::new(
            plan_set_generation(),
            plan_id(),
            registration().binding_id(),
            registration().binding_generation(),
            BindingConfigurationDigest::new([0x91; 32]),
            compatibility(),
            BindingArtifactRole::ProducerRoute,
        );
        let bad_ref = prepare(
            BindingArtifactRef::new(wrong_ref_identity, SlotIndex::new(0)),
            route(),
        );
        assert_rejected(&base_record(), &bad_ref, registration());

        let wrong_routes = [
            BindingRouteKey::new(
                BindingId::new(8),
                registration().binding_generation(),
                Generation::INITIAL,
                plan_set_generation(),
                plan_id(),
                reservation(),
            ),
            BindingRouteKey::new(
                registration().binding_id(),
                BindingGeneration::new(next_generation()),
                Generation::INITIAL,
                plan_set_generation(),
                plan_id(),
                reservation(),
            ),
            BindingRouteKey::new(
                registration().binding_id(),
                registration().binding_generation(),
                next_generation(),
                plan_set_generation(),
                plan_id(),
                reservation(),
            ),
            BindingRouteKey::new(
                registration().binding_id(),
                registration().binding_generation(),
                Generation::INITIAL,
                other_plan_set_generation(),
                plan_id(),
                reservation(),
            ),
            BindingRouteKey::new(
                registration().binding_id(),
                registration().binding_generation(),
                Generation::INITIAL,
                plan_set_generation(),
                other_plan_id(),
                reservation(),
            ),
            BindingRouteKey::new(
                registration().binding_id(),
                registration().binding_generation(),
                Generation::INITIAL,
                plan_set_generation(),
                plan_id(),
                RouteReservationIdentity::new(
                    CollisionDomainId::new([0x92; 16]),
                    EndpointReservationKey::new([0x93; 32]),
                ),
            ),
        ];
        for wrong_route in wrong_routes {
            let bad_route = prepare(
                BindingArtifactRef::new(base_identity, SlotIndex::new(0)),
                wrong_route,
            );
            assert_rejected(&base_record(), &bad_route, registration());
        }

        let wrong_registrations = [
            BindingRegistrationIdentity::new(
                BindingId::new(8),
                registration().binding_generation(),
                configuration(),
                compatibility(),
                0,
            ),
            BindingRegistrationIdentity::new(
                registration().binding_id(),
                BindingGeneration::new(next_generation()),
                configuration(),
                compatibility(),
                0,
            ),
            BindingRegistrationIdentity::new(
                registration().binding_id(),
                registration().binding_generation(),
                BindingConfigurationDigest::new([0x94; 32]),
                compatibility(),
                0,
            ),
            BindingRegistrationIdentity::new(
                registration().binding_id(),
                registration().binding_generation(),
                configuration(),
                BindingArtifactCompatibility::new([0x95; 16]),
                0,
            ),
        ];
        for wrong_registration in wrong_registrations {
            assert_rejected(&base_record(), &base_prepare(), wrong_registration);
        }
    }

    #[test]
    fn prepare_artifact_resolution_rejects_plan_lease_role_and_footprint_mutations() {
        let base_identity = identity(BindingArtifactRole::ProducerRoute);
        let stored_reference = BindingArtifactRef::new(base_identity, SlotIndex::new(0));
        let wrong_plan = record(
            base_identity,
            other_plan_id(),
            stored_reference,
            plan_set_generation(),
        );
        assert_rejected(&wrong_plan, &base_prepare(), registration());

        let wrong_lease = record(
            base_identity,
            plan_id(),
            stored_reference,
            other_plan_set_generation(),
        );
        assert_rejected(&wrong_lease, &base_prepare(), registration());

        let wrong_role_identity = identity(BindingArtifactRole::ConsumerCall);
        let wrong_role = record(
            wrong_role_identity,
            plan_id(),
            BindingArtifactRef::new(wrong_role_identity, SlotIndex::new(0)),
            plan_set_generation(),
        );
        let wrong_role_prepare = prepare(
            BindingArtifactRef::new(wrong_role_identity, SlotIndex::new(0)),
            route(),
        );
        assert_rejected(&wrong_role, &wrong_role_prepare, registration());

        let wrong_footprint = PrepareInput::new(
            route(),
            stored_reference,
            BindingLifetimeFootprint::new(3, 129),
        );
        assert_rejected(&base_record(), &wrong_footprint, registration());
    }
}
