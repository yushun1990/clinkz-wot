//! Narrow Property Read lifecycle composition shared by the static and host
//! Servient profiles.

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::task::{Context, Poll};

use clinkz_wot_core::binding::BindingRouteKey;
use clinkz_wot_core::{
    AffordanceTarget, BindingArtifactEnvelope, BindingArtifactRef, BindingArtifactRole,
    BindingCallSettlement, BindingCancellationDisposition, BindingCompilerExtension,
    BindingDeliveryOutcome, BindingIngressPolicy, BindingLifetimeFootprint,
    BindingOperationalError, BindingRegistrationIdentity, BindingResourceDeclarations,
    BindingStateLayout, BindingStatusPolicy, CleanupOperation, CleanupPhaseContext, CleanupRecord,
    CleanupReservation, CleanupSlotId, CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase,
    HandlerContext, HandlerFootprint, LogicalInteractionPlan, PendingWork, PendingWorkClass,
    PlanId, PlanSetGeneration, PollServerBinding, PrepareInput, ReadPropertyHandler, RetryClass,
    RouteAcceptEvent, RouteAcceptLease, RouteActivationOutcome, RouteCleanupOutcome,
    RouteCommitOutcome, RouteInboundResponse, RoutePrepareOutcome, RouteReadinessOutcome,
    RouteReadinessSlot, RouteTerminal, ServerResponseSlot, ServerRouteSlot,
    ServingActivationAuthority, StartStatus, StaticBindingRegistration, StaticHandlerRegistration,
    StepStatus, ThingId, ThingSlotId,
};
use clinkz_wot_foundation::{
    ResourceAccount, ResourceKind, ResourceLimits, SlotIndex, WorkBudget, WorkClass,
};
use clinkz_wot_planning::{
    PlanBuildOutput, PlanBuildStep, PlanCompiler, PropertyReadBuildCursor, PropertyReadPlanCompiler,
};
use clinkz_wot_td::{data_type::Operation, thing::Thing};

#[cfg(feature = "std")]
use clinkz_wot_core::{
    HostActiveRouteGuard, HostBindingArtifact, HostBindingCallBox, HostBindingCompilerCursor,
    HostBindingRegistration, HostCommittedRouteGuard, HostPreparedRouteGuard,
    HostRouteCleanupSuccessor, HostShutdownRouteGuard, RouteAbortInput, RouteCleanupSuccessor,
    RouteShutdownInput,
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
        let storage = FirstEntryStorage::Static {
            route: self.binding.server().route_state_layout(),
            readiness: self.binding.server().readiness_state_layout(),
            response: self.binding.server().response_state_layout(),
        };
        let mut admission = AdmissionReservations::new(
            &self.limits,
            self.thing_slot,
            self.binding.resources(),
            self.binding.ingress(),
            self.binding.status(),
            storage,
            self.deadline,
        )?;
        if self.handler.slot_id().generation() != self.thing_slot.generation() {
            return Err(validation_error(self.thing_slot));
        }
        admission.reserve_handler(&self.limits, self.thing_slot, self.handler.footprint())?;
        Ok(StaticPropertyReadServient::new(
            self.td,
            self.thing_slot,
            self.limits,
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
    residual_cleanup: Vec<CleanupRecord>,
    cancelled_before_publication: bool,
}

impl LifecycleDisposition {
    const fn new() -> Self {
        Self {
            first_failure: None,
            cleanup_cause: None,
            residual_cleanup: Vec::new(),
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
        self.residual_cleanup.push(record);
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
    charges: Vec<ResourceAccount>,
    call_cleanup: Option<CleanupReservation>,
    route_cleanup: Option<CleanupReservation>,
    #[cfg(feature = "std")]
    cleanup_call_cleanup: Option<CleanupReservation>,
    compiled: bool,
    handler: Option<HandlerFootprint>,
    response_bytes: u64,
    #[cfg(feature = "std")]
    host_call_ceiling: Option<BindingLifetimeFootprint>,
    deadline: Deadline,
}

#[derive(Clone, Copy)]
enum FirstEntryStorage {
    Static {
        route: BindingStateLayout,
        readiness: BindingStateLayout,
        response: BindingStateLayout,
    },
    #[cfg(feature = "std")]
    Host,
}

impl AdmissionReservations {
    #[allow(clippy::too_many_arguments)]
    fn new(
        limits: &ResourceLimits,
        thing_slot: ThingSlotId,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
        storage: FirstEntryStorage,
        deadline: Deadline,
    ) -> CoreResult<Self> {
        let cleanup_slots: u32 = match storage {
            #[cfg(feature = "std")]
            FirstEntryStorage::Host => 3,
            FirstEntryStorage::Static { .. } => 2,
        };
        let cleanups = u64::from(cleanup_slots);
        if status.retained_records() < 2 || status.retained_bytes() == 0 {
            return Err(validation_error(thing_slot));
        }
        let route_bytes = resources.route_state().retained_bytes().max(1);
        let readiness_bytes = resources.readiness_state().retained_bytes().max(1);
        let response_bytes = resources.response_state().retained_bytes().max(1);
        let call_bytes = resources.admitted().retained_bytes().max(1);
        let cleanup_bytes = route_bytes
            .max(readiness_bytes)
            .max(response_bytes)
            .max(call_bytes);
        let cleanup_items = resources
            .route_state()
            .retained_items()
            .max(resources.readiness_state().retained_items())
            .max(resources.response_state().retained_items())
            .max(resources.admitted().retained_items())
            .max(1);
        let cleanup_total = cleanup_bytes
            .checked_mul(cleanups)
            .ok_or_else(|| validation_error(thing_slot))?;
        let cleanup_steps = limits
            .get(ResourceKind::CleanupWorkItemsPerStepMax)
            .filter(|value| *value != 0)
            .ok_or_else(|| validation_error(thing_slot))?;
        let mut charges = Vec::new();
        for (kind, amount) in [
            (ResourceKind::PlanSetsPerThingMax, 1),
            (ResourceKind::PlanSetsGlobalMax, 1),
            (ResourceKind::PlanPinsPerPlanSetMax, 1),
            (ResourceKind::PlanPinsGlobalMax, 1),
            (ResourceKind::BindingArtifactsPerThingMax, 1),
            (ResourceKind::BindingArtifactsGlobalMax, 1),
            (ResourceKind::HandlerSlotsPerThingMax, 1),
            (ResourceKind::HandlerSlotsGlobalMax, 1),
            (ResourceKind::PendingHandlerCallsPerThingMax, 1),
            (ResourceKind::PendingHandlerCallsGlobalMax, 1),
            (ResourceKind::InFlightResponsesPerThingMax, 1),
            (ResourceKind::InFlightResponsesGlobalMax, 1),
            (ResourceKind::BindingRoutesPerThingMax, 1),
            (ResourceKind::BindingRoutesGlobalMax, 1),
            (ResourceKind::EndpointReservationsPerThingMax, 1),
            (ResourceKind::EndpointReservationsGlobalMax, 1),
            (ResourceKind::RouteReadinessTokensPerThingMax, 1),
            (ResourceKind::RouteReadinessTokensGlobalMax, 1),
            (ResourceKind::CleanupItemsMax, cleanups),
            (ResourceKind::CleanupRetryRecordsMax, cleanups),
            (ResourceKind::CleanupTransferSlotsGlobalMax, cleanups),
            (ResourceKind::DurableStatusEntriesPerBindingMax, 2),
            (ResourceKind::RouteGuardBytesPerItemMax, route_bytes),
            (ResourceKind::RouteGuardBytesPerThingMax, route_bytes),
            (ResourceKind::RouteGuardBytesGlobalMax, route_bytes),
            (
                ResourceKind::RouteReadinessTokenBytesPerItemMax,
                readiness_bytes,
            ),
            (
                ResourceKind::RouteReadinessTokenBytesGlobalMax,
                readiness_bytes,
            ),
            (
                ResourceKind::BindingPollTemporaryBytesPerCallMax,
                resources.transient().peak_bytes(),
            ),
            (
                ResourceKind::BindingPollTemporaryBytesGlobalMax,
                resources.transient().peak_bytes(),
            ),
            (
                ResourceKind::BindingCancelBufferBytesPerCallMax,
                cleanup_bytes,
            ),
            (
                ResourceKind::BindingCancelBufferBytesGlobalMax,
                cleanup_bytes,
            ),
            (ResourceKind::CleanupItemBytesMax, cleanup_bytes),
            (ResourceKind::CleanupBytesMax, cleanup_total),
            (ResourceKind::CleanupTransferBytesGlobalMax, cleanup_total),
            (
                ResourceKind::DurableStatusBytesPerBindingMax,
                status.retained_bytes(),
            ),
            (
                ResourceKind::DurableStatusBytesGlobalMax,
                status.retained_bytes(),
            ),
            (ResourceKind::RouteReadinessStepsMax, 1),
            (
                ResourceKind::BindingIngressItemsPerRouteMax,
                u64::from(ingress.per_route().items()),
            ),
            (
                ResourceKind::BindingIngressItemsPerBindingMax,
                u64::from(ingress.per_binding().items()),
            ),
            (
                ResourceKind::BindingIngressItemsGlobalMax,
                u64::from(ingress.global().items()),
            ),
            (
                ResourceKind::BindingIngressBytesPerRouteMax,
                ingress.per_route().bytes(),
            ),
            (
                ResourceKind::BindingIngressBytesPerBindingMax,
                ingress.per_binding().bytes(),
            ),
            (
                ResourceKind::BindingIngressBytesGlobalMax,
                ingress.global().bytes(),
            ),
        ] {
            reserve_charge(&mut charges, limits, thing_slot, kind, amount)?;
        }
        match storage {
            FirstEntryStorage::Static {
                route,
                readiness,
                response,
            } => {
                let r = route
                    .size()
                    .max(route.lifetime_footprint().retained_bytes());
                let q = readiness
                    .size()
                    .max(readiness.lifetime_footprint().retained_bytes());
                let s = response
                    .size()
                    .max(response.lifetime_footprint().retained_bytes());
                let total = r
                    .checked_add(q)
                    .and_then(|v| v.checked_add(s))
                    .ok_or_else(|| validation_error(thing_slot))?;
                for (kind, amount) in [
                    (
                        ResourceKind::BindingSlotStateBytesPerItemMax,
                        r.max(q).max(s),
                    ),
                    (ResourceKind::BindingSlotStateBytesPerThingMax, total),
                    (ResourceKind::BindingSlotStateBytesGlobalMax, total),
                ] {
                    reserve_charge(&mut charges, limits, thing_slot, kind, amount)?;
                }
            }
            #[cfg(feature = "std")]
            FirstEntryStorage::Host => {
                for kind in [
                    ResourceKind::HostBindingCallBytesPerItemMax,
                    ResourceKind::HostBindingCallBytesPerBindingMax,
                    ResourceKind::HostBindingCallBytesPerThingMax,
                    ResourceKind::HostBindingCallBytesGlobalMax,
                ] {
                    reserve_charge(&mut charges, limits, thing_slot, kind, call_bytes)?;
                }
            }
        }
        #[cfg(feature = "std")]
        if matches!(storage, FirstEntryStorage::Host) {
            reserve_charge(
                &mut charges,
                limits,
                thing_slot,
                ResourceKind::HostBindingCancelDrainTimeoutMillisMax,
                1,
            )?;
        }
        let base = thing_slot
            .slot()
            .get()
            .checked_mul(cleanup_slots)
            .ok_or_else(|| validation_error(thing_slot))?;
        let cleanup = |slot| {
            CleanupReservation::new(
                CleanupSlotId::new(SlotIndex::new(slot), thing_slot.generation()),
                BindingLifetimeFootprint::new(cleanup_items, cleanup_bytes),
                1,
                WorkBudget::new().with_remaining(WorkClass::CleanupItems, cleanup_steps),
            )
        };
        Ok(Self {
            charges,
            call_cleanup: Some(cleanup(base)),
            route_cleanup: Some(cleanup(
                base.checked_add(1)
                    .ok_or_else(|| validation_error(thing_slot))?,
            )),
            #[cfg(feature = "std")]
            cleanup_call_cleanup: matches!(storage, FirstEntryStorage::Host)
                .then(|| cleanup(base.checked_add(2).expect("three cleanup slots admitted"))),
            compiled: false,
            handler: None,
            response_bytes,
            #[cfg(feature = "std")]
            host_call_ceiling: matches!(storage, FirstEntryStorage::Host)
                .then_some(resources.admitted()),
            deadline,
        })
    }

    fn reserve_handler(
        &mut self,
        limits: &ResourceLimits,
        slot: ThingSlotId,
        footprint: HandlerFootprint,
    ) -> CoreResult<()> {
        if let Some(saved) = self.handler {
            return if saved == footprint {
                Ok(())
            } else {
                Err(validation_error(slot))
            };
        }
        let response = self
            .response_bytes
            .max(footprint.pending_call_bytes().max(1));
        for (kind, amount) in [
            (
                ResourceKind::HandlerStateBytesPerThingMax,
                footprint.retained_bytes(),
            ),
            (
                ResourceKind::HandlerStateBytesGlobalMax,
                footprint.retained_bytes(),
            ),
            (
                ResourceKind::BindingResponseBufferBytesPerRouteMax,
                response,
            ),
            (ResourceKind::BindingResponseBufferBytesGlobalMax, response),
        ] {
            reserve_charge(&mut self.charges, limits, slot, kind, amount)?;
        }
        self.handler = Some(footprint);
        Ok(())
    }

    fn reserve_compiled<A>(
        &mut self,
        limits: &ResourceLimits,
        slot: ThingSlotId,
        artifact: &BindingArtifactEnvelope<A>,
        logical: u64,
    ) -> CoreResult<()> {
        if self.compiled {
            return Err(validation_error(slot));
        }
        let footprint = artifact.artifact().footprint();
        let structural = (core::mem::size_of::<PlanBuildOutput<A>>()
            + core::mem::size_of::<BindingArtifactEnvelope<A>>()
            + core::mem::size_of::<BindingArtifactRef>()) as u64;
        let compiled = logical
            .checked_add(footprint.retained_bytes())
            .and_then(|v| v.checked_add(structural))
            .ok_or_else(|| validation_error(slot))?;
        for (kind, amount) in [
            (
                ResourceKind::BindingArtifactBytesPerItemMax,
                footprint.retained_bytes(),
            ),
            (
                ResourceKind::BindingArtifactBytesPerThingMax,
                footprint.retained_bytes(),
            ),
            (
                ResourceKind::BindingArtifactBytesGlobalMax,
                footprint.retained_bytes(),
            ),
            (ResourceKind::LogicalPlanBytesPerThingMax, logical),
            (ResourceKind::CompiledPlanBytesMax, compiled),
            (ResourceKind::CompiledRuntimeBytesPerThingMax, compiled),
            (ResourceKind::CompiledRuntimeBytesGlobalMax, compiled),
        ] {
            reserve_charge(&mut self.charges, limits, slot, kind, amount)?;
        }
        self.compiled = true;
        Ok(())
    }

    #[cfg(feature = "std")]
    fn admits_host_call<T: 'static, C: 'static>(
        &self,
        call: &mut HostBindingCallBox<T, C>,
    ) -> bool {
        self.host_call_ceiling
            .is_some_and(|ceiling| call.as_pin_mut().lifetime_footprint().fits_within(ceiling))
    }

    fn call_context(
        &mut self,
        slot: ThingSlotId,
        operation: CleanupOperation,
        cause: CoreError,
    ) -> CoreResult<CleanupPhaseContext> {
        let reservation = self
            .call_cleanup
            .take()
            .ok_or_else(|| validation_error(slot))?;
        Ok(CleanupPhaseContext::bind(
            reservation,
            operation,
            cause,
            self.deadline,
        ))
    }

    #[cfg(feature = "std")]
    fn cleanup_call_context(
        &mut self,
        slot: ThingSlotId,
        cause: CoreError,
    ) -> CoreResult<CleanupPhaseContext> {
        let reservation = self
            .cleanup_call_cleanup
            .take()
            .ok_or_else(|| validation_error(slot))?;
        Ok(CleanupPhaseContext::bind(
            reservation,
            CleanupOperation::CancelProcess,
            cause,
            self.deadline,
        ))
    }

    fn route_context(
        &mut self,
        slot: ThingSlotId,
        operation: CleanupOperation,
        cause: CoreError,
    ) -> CoreResult<CleanupPhaseContext> {
        let reservation = self
            .route_cleanup
            .take()
            .ok_or_else(|| validation_error(slot))?;
        Ok(CleanupPhaseContext::bind(
            reservation,
            operation,
            cause,
            self.deadline,
        ))
    }

    fn complete(&self) -> bool {
        self.call_cleanup.is_some()
            && self.route_cleanup.is_some()
            && {
                #[cfg(feature = "std")]
                {
                    self.host_call_ceiling.is_none() || self.cleanup_call_cleanup.is_some()
                }
                #[cfg(not(feature = "std"))]
                {
                    true
                }
            }
            && self.compiled
            && self.handler.is_some()
            && self.charges.iter().all(|a| a.used() != 0)
    }
}

impl Drop for AdmissionReservations {
    fn drop(&mut self) {
        for account in &mut self.charges {
            let used = account.used();
            if used != 0 {
                let _ = account.release_committed(used);
            }
        }
    }
}

fn reserve_charge(
    charges: &mut Vec<ResourceAccount>,
    limits: &ResourceLimits,
    slot: ThingSlotId,
    kind: ResourceKind,
    amount: u64,
) -> CoreResult<()> {
    if amount == 0 {
        return Ok(());
    }
    let limit = limits.get(kind).unwrap_or(0);
    let mut account = ResourceAccount::new(slot.slot(), slot.generation(), kind, limit);
    let reservation = account
        .try_reserve(amount)
        .ok_or_else(|| CoreError::LimitExceeded {
            resource: kind,
            limit,
            requested: Some(amount),
            observed: None,
            context: ErrorContext::new(ErrorPhase::Admission, RetryClass::Never).with_thing(slot),
        })?;
    reservation.commit();
    charges.push(account);
    Ok(())
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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
    logical_bytes: u64,
}

#[allow(clippy::too_many_arguments)]
fn verify_first_entry_closure<A>(
    plan_set: &CompiledPlanSetRecord<A>,
    prepare: &PrepareInput,
    artifact: &BindingArtifactEnvelope<A>,
    registration: BindingRegistrationIdentity,
    admitted_route_footprint: BindingLifetimeFootprint,
    thing_slot: ThingSlotId,
    limits: &ResourceLimits,
    derived: &DerivedRoute,
    handler_target: &str,
    handler_generation: Option<clinkz_wot_foundation::Generation>,
    handler_footprint: HandlerFootprint,
    route_key: Option<BindingRouteKey>,
    activation: Option<&ServingActivationRecord>,
    admission: &mut AdmissionReservations,
    route_and_response_owners_vacant: bool,
    supports_required_cell: bool,
    supports_property_read: bool,
    retained_status_records: u32,
) -> CoreResult<()> {
    let resolved = plan_set.resolve_prepare_artifact(
        prepare,
        registration,
        thing_slot,
        admitted_route_footprint,
    )?;
    let activation = activation.ok_or_else(|| validation_error(thing_slot))?;
    admission.reserve_compiled(limits, thing_slot, artifact, derived.logical_bytes)?;
    admission.reserve_handler(limits, thing_slot, handler_footprint)?;
    let route = *prepare.route();
    if !core::ptr::eq(resolved, artifact)
        || route_key != Some(route)
        || derived.key != route
        || derived.artifact_ref != prepare.artifact()
        || derived.plan_id != route.plan_id()
        || derived.target.name() != Some(handler_target)
        || handler_generation.is_some_and(|generation| generation != thing_slot.generation())
        || !admission.complete()
        || !route_and_response_owners_vacant
        || !supports_required_cell
        || !supports_property_read
        || retained_status_records == 0
        || activation.published
        || activation.authority.thing_id() != &derived.thing_id
        || activation.authority.produced_generation() != &thing_slot.generation()
        || activation.authority.plan_set_generation() != &route.plan_set_generation()
        || activation.lease.route() != &route
    {
        return Err(validation_error(thing_slot));
    }
    Ok(())
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
        logical_bytes: logical_plan_bytes(plan).ok_or_else(|| validation_error(thing_slot))?,
    })
}

fn logical_plan_bytes(plan: &LogicalInteractionPlan) -> Option<u64> {
    let mut bytes = core::mem::size_of_val(plan) as u64;
    for value in [
        plan.thing_id().as_str(),
        plan.property_name(),
        plan.resolved_target(),
        plan.content_type().unwrap_or_default(),
        plan.subprotocol().unwrap_or_default(),
    ] {
        bytes = bytes.checked_add(value.len() as u64)?;
    }
    Some(bytes)
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
    limits: ResourceLimits,
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
    call_phase: Option<CleanupPhaseContext>,
}

impl<'h, B, H> StaticPropertyReadServient<'h, B, H>
where
    B: PollServerBinding,
    H: ReadPropertyHandler,
{
    fn new(
        td: Thing,
        thing_slot: ThingSlotId,
        limits: ResourceLimits,
        registration: StaticBindingRegistration<B>,
        handler: StaticHandlerRegistration<'h, H>,
        handler_name: Box<str>,
        admission: AdmissionReservations,
    ) -> Self {
        Self {
            td: Some(td),
            thing_slot,
            limits,
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
            call_phase: None,
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
                if let Err(error) = verify_first_entry_closure(
                    &self.plan_set,
                    &prepare,
                    artifact,
                    self.registration.identity(),
                    self.registration.resources().route_state(),
                    self.thing_slot,
                    &self.limits,
                    self.derived
                        .as_ref()
                        .expect("first entry retains the route projection"),
                    &self.handler_name,
                    Some(self.handler.slot_id().generation()),
                    self.handler.footprint(),
                    self.route.key,
                    self.activation.as_ref(),
                    &mut self.admission,
                    self.route.state.route.is_vacant()
                        && self.route.state.readiness.is_vacant()
                        && self.route.state.response.is_vacant()
                        && self.in_flight.is_none(),
                    self.registration.execution().supports_application_static(),
                    self.registration
                        .capabilities()
                        .supports_producer_property_read(),
                    self.registration.status().retained_records(),
                ) {
                    self.activation = None;
                    self.derived = None;
                    self.route.key = None;
                    return self.fail_without_route(error);
                }
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
        let response = RouteInboundResponse::seal_property_read_handler_result(opportunity, result);
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
            .route_context(self.thing_slot, operation, cause)
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

    fn ensure_call_phase(&mut self) -> CoreResult<()> {
        if self.call_phase.is_none() {
            let cause = self.disposition.cleanup_cause(self.thing_slot);
            self.call_phase = Some(self.admission.call_context(
                self.thing_slot,
                CleanupOperation::CancelRouteReadiness,
                cause,
            )?);
        }
        Ok(())
    }

    fn settle_cancel(
        &mut self,
        disposition: BindingCancellationDisposition<()>,
        operation: CleanupOperation,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        match disposition {
            BindingCancellationDisposition::Complete { .. } => {
                self.call_phase = None;
                self.start_route_cleanup(operation, budget)
            }
            BindingCancellationDisposition::TransferRequired(_) => {
                progress(PendingWorkClass::Cleanup)
            }
            BindingCancellationDisposition::ResidualExternalState { record, .. } => {
                self.disposition.retain_residual(record);
                self.call_phase = None;
                self.start_route_cleanup(operation, budget)
            }
        }
    }

    fn cancel_prepare(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()> {
        if let Err(error) = self.ensure_call_phase() {
            self.disposition.record_core_failure(error);
            return progress(PendingWorkClass::Cleanup);
        }
        match self.registration.server_mut().poll_cancel_prepare(
            cx,
            self.call_phase.as_ref().unwrap(),
            &mut self.route.state.route,
            budget,
        ) {
            Poll::Pending => progress(PendingWorkClass::Cleanup),
            Poll::Ready(Err(error)) => {
                self.disposition.record_core_failure(error);
                progress(PendingWorkClass::Cleanup)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(RoutePrepareOutcome::Prepared(())))) => {
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(
                RoutePrepareOutcome::RejectedNoResource(error),
            ))) => {
                self.call_phase = None;
                self.finish_rejected_route(error)
            }
            Poll::Ready(Ok(BindingCallSettlement::Cancelled { disposition, .. })) => {
                self.settle_cancel(disposition, CleanupOperation::AbortPreparedRoute, budget)
            }
        }
    }

    fn cancel_readiness(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        if let Err(error) = self.ensure_call_phase() {
            self.disposition.record_core_failure(error);
            return progress(PendingWorkClass::Cleanup);
        }
        match self.registration.server_mut().poll_cancel_readiness(
            cx,
            self.call_phase.as_ref().unwrap(),
            &mut self.route.state.route,
            &mut self.route.state.readiness,
            budget,
        ) {
            Poll::Pending => progress(PendingWorkClass::Cleanup),
            Poll::Ready(Err(error)) => {
                self.disposition.record_core_failure(error);
                progress(PendingWorkClass::Cleanup)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(RouteReadinessOutcome::Ready(())))) => {
                self.route.state.readiness.clear();
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(RouteReadinessOutcome::Failed {
                error,
                ..
            }))) => {
                self.route.state.readiness.clear();
                self.disposition.record_binding_failure(error);
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Cancelled { disposition, .. })) => {
                self.settle_cancel(disposition, CleanupOperation::AbortPreparedRoute, budget)
            }
        }
    }

    fn cancel_activate(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()> {
        if let Err(error) = self.ensure_call_phase() {
            self.disposition.record_core_failure(error);
            return progress(PendingWorkClass::Cleanup);
        }
        match self.registration.server_mut().poll_cancel_activate(
            cx,
            self.call_phase.as_ref().unwrap(),
            &mut self.route.state.route,
            budget,
        ) {
            Poll::Pending => progress(PendingWorkClass::Cleanup),
            Poll::Ready(Err(error)) => {
                self.disposition.record_core_failure(error);
                progress(PendingWorkClass::Cleanup)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(RouteActivationOutcome::Active(
                (),
            )))) => {
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(
                RouteActivationOutcome::NotActivated { error, .. },
            ))) => {
                self.disposition.record_binding_failure(error);
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::AbortPreparedRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Cancelled { disposition, .. })) => {
                self.settle_cancel(disposition, CleanupOperation::ShutdownRoute, budget)
            }
        }
    }

    fn cancel_commit(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> StepStatus<()> {
        if let Err(error) = self.ensure_call_phase() {
            self.disposition.record_core_failure(error);
            return progress(PendingWorkClass::Cleanup);
        }
        match self.registration.server_mut().poll_cancel_commit(
            cx,
            self.call_phase.as_ref().unwrap(),
            &mut self.route.state.route,
            budget,
        ) {
            Poll::Pending => progress(PendingWorkClass::Cleanup),
            Poll::Ready(Err(error)) => {
                self.disposition.record_core_failure(error);
                progress(PendingWorkClass::Cleanup)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(RouteCommitOutcome::Committed(())))) => {
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Returned(
                RouteCommitOutcome::NotCommitted { error, .. },
            ))) => {
                self.disposition.record_binding_failure(error);
                self.call_phase = None;
                self.start_route_cleanup(CleanupOperation::ShutdownRoute, budget)
            }
            Poll::Ready(Ok(BindingCallSettlement::Cancelled { disposition, .. })) => {
                self.settle_cancel(disposition, CleanupOperation::ShutdownRoute, budget)
            }
        }
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
            StaticRoutePhase::Preparing if self.expose == ExposeState::Cancelling => {
                self.cancel_prepare(cx, budget)
            }
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
            StaticRoutePhase::AwaitingReadiness if self.expose == ExposeState::Cancelling => {
                self.cancel_readiness(cx, budget)
            }
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
            StaticRoutePhase::Activating if self.expose == ExposeState::Cancelling => {
                self.cancel_activate(cx, budget)
            }
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
            StaticRoutePhase::Committing if self.expose == ExposeState::Cancelling => {
                self.cancel_commit(cx, budget)
            }
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
    CancellingPrepare(HostPrepareCall),
    JoiningPrepare(HostPrepareCall),
    Prepared(HostPreparedRouteGuard),
    AwaitingReadiness(HostReadinessCall),
    CancellingReadiness(HostReadinessCall),
    JoiningReadiness(HostReadinessCall),
    Ready(HostPreparedRouteGuard),
    Activating(HostActivationCall),
    CancellingActivation(HostActivationCall),
    JoiningActivation(HostActivationCall),
    Active(HostActiveRouteGuard),
    Committing(HostCommitCall),
    CancellingCommit(HostCommitCall),
    JoiningCommit(HostCommitCall),
    CommittedClosed(HostCommittedRouteGuard),
    Serving(HostCommittedRouteGuard),
    Dispatching(HostCommittedRouteGuard),
    Delivering {
        guard: HostCommittedRouteGuard,
        call: HostBindingCallBox<BindingDeliveryOutcome>,
    },
    RejectingDelivery {
        guard: HostCommittedRouteGuard,
        call: HostBindingCallBox<BindingDeliveryOutcome>,
    },
    CancellingDelivery {
        guard: HostCommittedRouteGuard,
        call: HostBindingCallBox<BindingDeliveryOutcome>,
    },
    JoiningDelivery {
        guard: HostCommittedRouteGuard,
        call: HostBindingCallBox<BindingDeliveryOutcome>,
    },
    AbortPending(RouteAbortInput),
    ShutdownPending(RouteShutdownInput),
    Cleaning(HostCleanupCall),
    RejectingCleanup(HostCleanupCall),
    CancellingCleanup(HostCleanupCall),
    JoiningCleanup(HostCleanupCall),
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
            config.registration.resources(),
            config.registration.ingress(),
            config.registration.status(),
            FirstEntryStorage::Host,
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
            Ok(mut call) => {
                self.route.state = if self.admission.admits_host_call(&mut call) {
                    HostRouteState::Cleaning(call)
                } else {
                    self.disposition
                        .record_core_failure(validation_error(self.thing_slot));
                    HostRouteState::RejectingCleanup(call)
                };
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
            Ok(mut call) => {
                self.route.state = if self.admission.admits_host_call(&mut call) {
                    HostRouteState::Cleaning(call)
                } else {
                    self.disposition
                        .record_core_failure(validation_error(self.thing_slot));
                    HostRouteState::RejectingCleanup(call)
                };
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
        let phase = match self.admission.route_context(
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
        let phase = match self.admission.route_context(
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

    fn finish_cancelled_no_route(&mut self) -> StepStatus<()> {
        self.activation = None;
        self.plan_set.reclaim();
        self.route.state = HostRouteState::Closed;
        self.expose = self.disposition.terminal_expose_state();
        progress(PendingWorkClass::Cleanup)
    }

    fn cancel_successor(&mut self, successor: HostRouteCleanupSuccessor) -> StepStatus<()> {
        match successor {
            RouteCleanupSuccessor::NoRouteResource { .. }
            | RouteCleanupSuccessor::ResidualRouteState { .. } => self.finish_cancelled_no_route(),
            RouteCleanupSuccessor::AbortPrepared(guard) => self.start_abort(guard),
            RouteCleanupSuccessor::ShutdownActive(guard) => {
                self.start_shutdown(HostShutdownRouteGuard::Active(guard))
            }
            RouteCleanupSuccessor::ShutdownCommitted(guard) => {
                self.start_shutdown(HostShutdownRouteGuard::Committed(guard))
            }
        }
    }

    fn cancel_disposition(
        &mut self,
        disposition: BindingCancellationDisposition<HostRouteCleanupSuccessor>,
    ) -> Option<StepStatus<()>> {
        match disposition {
            BindingCancellationDisposition::Complete { successor } => {
                Some(self.cancel_successor(successor))
            }
            BindingCancellationDisposition::TransferRequired(_) => None,
            BindingCancellationDisposition::ResidualExternalState { successor, record } => {
                self.disposition.retain_residual(record);
                Some(self.cancel_successor(successor))
            }
        }
    }

    fn settle_prepare(
        &mut self,
        call: HostPrepareCall,
        settlement: BindingCallSettlement<
            RoutePrepareOutcome<HostPreparedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
    ) -> StepStatus<()> {
        match settlement {
            BindingCallSettlement::Returned(RoutePrepareOutcome::Prepared(guard)) => {
                drop(call);
                self.start_abort(guard)
            }
            BindingCallSettlement::Returned(RoutePrepareOutcome::RejectedNoResource(error)) => {
                drop(call);
                self.fail_binding_without_route(error)
            }
            BindingCallSettlement::Cancelled { disposition, .. } => {
                match self.cancel_disposition(disposition) {
                    Some(status) => {
                        drop(call);
                        status
                    }
                    None => {
                        self.route.state = HostRouteState::CancellingPrepare(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
        }
    }

    fn settle_readiness(
        &mut self,
        call: HostReadinessCall,
        settlement: BindingCallSettlement<
            RouteReadinessOutcome<HostPreparedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
    ) -> StepStatus<()> {
        match settlement {
            BindingCallSettlement::Returned(RouteReadinessOutcome::Ready(guard)) => {
                drop(call);
                self.start_abort(guard)
            }
            BindingCallSettlement::Returned(RouteReadinessOutcome::Failed { guard, error }) => {
                drop(call);
                self.start_failed_abort(guard, error)
            }
            BindingCallSettlement::Cancelled { disposition, .. } => {
                match self.cancel_disposition(disposition) {
                    Some(status) => {
                        drop(call);
                        status
                    }
                    None => {
                        self.route.state = HostRouteState::CancellingReadiness(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
        }
    }

    fn settle_activation(
        &mut self,
        call: HostActivationCall,
        settlement: BindingCallSettlement<
            RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
    ) -> StepStatus<()> {
        match settlement {
            BindingCallSettlement::Returned(RouteActivationOutcome::Active(guard)) => {
                drop(call);
                self.start_shutdown(HostShutdownRouteGuard::Active(guard))
            }
            BindingCallSettlement::Returned(RouteActivationOutcome::NotActivated {
                guard,
                error,
            }) => {
                drop(call);
                self.start_failed_abort(guard, error)
            }
            BindingCallSettlement::Cancelled { disposition, .. } => {
                match self.cancel_disposition(disposition) {
                    Some(status) => {
                        drop(call);
                        status
                    }
                    None => {
                        self.route.state = HostRouteState::CancellingActivation(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
        }
    }

    fn settle_commit(
        &mut self,
        call: HostCommitCall,
        settlement: BindingCallSettlement<
            RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
    ) -> StepStatus<()> {
        match settlement {
            BindingCallSettlement::Returned(RouteCommitOutcome::Committed(guard)) => {
                drop(call);
                self.start_shutdown(HostShutdownRouteGuard::Committed(guard))
            }
            BindingCallSettlement::Returned(RouteCommitOutcome::NotCommitted { guard, error }) => {
                drop(call);
                self.start_failed_shutdown(HostShutdownRouteGuard::Active(guard), error)
            }
            BindingCallSettlement::Cancelled { disposition, .. } => {
                match self.cancel_disposition(disposition) {
                    Some(status) => {
                        drop(call);
                        status
                    }
                    None => {
                        self.route.state = HostRouteState::CancellingCommit(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
        }
    }

    fn finish_delivery_result(
        &mut self,
        guard: HostCommittedRouteGuard,
        outcome: BindingDeliveryOutcome,
    ) -> StepStatus<()> {
        match outcome {
            BindingDeliveryOutcome::Delivered => {
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
                if self.expose == ExposeState::Draining {
                    self.start_shutdown(HostShutdownRouteGuard::Committed(guard))
                } else {
                    self.route.state = HostRouteState::Serving(guard);
                    progress(PendingWorkClass::BindingInput)
                }
            }
            BindingDeliveryOutcome::Failed(error) => {
                self.in_flight = None;
                self.start_failed_shutdown(HostShutdownRouteGuard::Committed(guard), error)
            }
        }
    }

    fn settle_delivery(
        &mut self,
        call: HostBindingCallBox<BindingDeliveryOutcome>,
        guard: HostCommittedRouteGuard,
        settlement: BindingCallSettlement<BindingDeliveryOutcome>,
    ) -> StepStatus<()> {
        match settlement {
            BindingCallSettlement::Returned(outcome) => {
                drop(call);
                self.finish_delivery_result(guard, outcome)
            }
            BindingCallSettlement::Cancelled { disposition, .. } => match disposition {
                BindingCancellationDisposition::Complete { .. } => {
                    drop(call);
                    self.in_flight = None;
                    self.start_shutdown(HostShutdownRouteGuard::Committed(guard))
                }
                BindingCancellationDisposition::ResidualExternalState { record, .. } => {
                    drop(call);
                    self.in_flight = None;
                    self.disposition.retain_residual(record);
                    self.start_shutdown(HostShutdownRouteGuard::Committed(guard))
                }
                BindingCancellationDisposition::TransferRequired(_) => {
                    self.route.state = HostRouteState::CancellingDelivery { guard, call };
                    progress(PendingWorkClass::Cleanup)
                }
            },
        }
    }

    fn settle_cleanup_call(
        &mut self,
        call: HostCleanupCall,
        settlement: BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
    ) -> StepStatus<()> {
        match settlement {
            BindingCallSettlement::Returned(outcome) => {
                drop(call);
                self.finish_cleanup(outcome)
            }
            BindingCallSettlement::Cancelled { disposition, .. } => {
                match self.cancel_disposition(disposition) {
                    Some(status) => {
                        drop(call);
                        status
                    }
                    None => {
                        self.route.state = HostRouteState::CancellingCleanup(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
        }
    }

    fn start_cancel_prepare(
        &mut self,
        mut call: HostPrepareCall,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let phase = match self.admission.call_context(
            self.thing_slot,
            CleanupOperation::CancelRouteReadiness,
            self.disposition.cleanup_cause(self.thing_slot),
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningPrepare(call);
                return progress(PendingWorkClass::Cleanup);
            }
        };
        match call.as_pin_mut().start_cancel(cx, phase, budget) {
            Ok(StartStatus::Pending) => {
                self.route.state = HostRouteState::CancellingPrepare(call);
                progress(PendingWorkClass::Cleanup)
            }
            Ok(StartStatus::Ready(settlement)) => self.settle_prepare(call, settlement),
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningPrepare(call);
                progress(PendingWorkClass::Cleanup)
            }
        }
    }

    fn start_cancel_readiness(
        &mut self,
        mut call: HostReadinessCall,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let phase = match self.admission.call_context(
            self.thing_slot,
            CleanupOperation::CancelRouteReadiness,
            self.disposition.cleanup_cause(self.thing_slot),
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningReadiness(call);
                return progress(PendingWorkClass::Cleanup);
            }
        };
        match call.as_pin_mut().start_cancel(cx, phase, budget) {
            Ok(StartStatus::Pending) => {
                self.route.state = HostRouteState::CancellingReadiness(call);
                progress(PendingWorkClass::Cleanup)
            }
            Ok(StartStatus::Ready(settlement)) => self.settle_readiness(call, settlement),
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningReadiness(call);
                progress(PendingWorkClass::Cleanup)
            }
        }
    }

    fn start_cancel_activation(
        &mut self,
        mut call: HostActivationCall,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let phase = match self.admission.call_context(
            self.thing_slot,
            CleanupOperation::CancelRouteReadiness,
            self.disposition.cleanup_cause(self.thing_slot),
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningActivation(call);
                return progress(PendingWorkClass::Cleanup);
            }
        };
        match call.as_pin_mut().start_cancel(cx, phase, budget) {
            Ok(StartStatus::Pending) => {
                self.route.state = HostRouteState::CancellingActivation(call);
                progress(PendingWorkClass::Cleanup)
            }
            Ok(StartStatus::Ready(settlement)) => self.settle_activation(call, settlement),
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningActivation(call);
                progress(PendingWorkClass::Cleanup)
            }
        }
    }

    fn start_cancel_commit(
        &mut self,
        mut call: HostCommitCall,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let phase = match self.admission.call_context(
            self.thing_slot,
            CleanupOperation::CancelRouteReadiness,
            self.disposition.cleanup_cause(self.thing_slot),
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningCommit(call);
                return progress(PendingWorkClass::Cleanup);
            }
        };
        match call.as_pin_mut().start_cancel(cx, phase, budget) {
            Ok(StartStatus::Pending) => {
                self.route.state = HostRouteState::CancellingCommit(call);
                progress(PendingWorkClass::Cleanup)
            }
            Ok(StartStatus::Ready(settlement)) => self.settle_commit(call, settlement),
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningCommit(call);
                progress(PendingWorkClass::Cleanup)
            }
        }
    }

    fn start_cancel_delivery(
        &mut self,
        mut call: HostBindingCallBox<BindingDeliveryOutcome>,
        guard: HostCommittedRouteGuard,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let phase = match self.admission.call_context(
            self.thing_slot,
            CleanupOperation::CancelResponseDelivery,
            self.disposition.cleanup_cause(self.thing_slot),
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningDelivery { guard, call };
                return progress(PendingWorkClass::Cleanup);
            }
        };
        match call.as_pin_mut().start_cancel(cx, phase, budget) {
            Ok(StartStatus::Pending) => {
                self.route.state = HostRouteState::CancellingDelivery { guard, call };
                progress(PendingWorkClass::Cleanup)
            }
            Ok(StartStatus::Ready(settlement)) => self.settle_delivery(call, guard, settlement),
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningDelivery { guard, call };
                progress(PendingWorkClass::Cleanup)
            }
        }
    }

    fn start_cancel_cleanup_call(
        &mut self,
        mut call: HostCleanupCall,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        let phase = match self.admission.cleanup_call_context(
            self.thing_slot,
            self.disposition.cleanup_cause(self.thing_slot),
        ) {
            Ok(phase) => phase,
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningCleanup(call);
                return progress(PendingWorkClass::Cleanup);
            }
        };
        match call.as_pin_mut().start_cancel(cx, phase, budget) {
            Ok(StartStatus::Pending) => {
                self.route.state = HostRouteState::CancellingCleanup(call);
                progress(PendingWorkClass::Cleanup)
            }
            Ok(StartStatus::Ready(settlement)) => self.settle_cleanup_call(call, settlement),
            Err(error) => {
                self.disposition.record_core_failure(error);
                self.route.state = HostRouteState::JoiningCleanup(call);
                progress(PendingWorkClass::Cleanup)
            }
        }
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
                let handler = self.handler.as_ref().expect("exposure freezes a handler");
                if let Err(error) = verify_first_entry_closure(
                    &self.plan_set,
                    &prepare,
                    artifact,
                    self.registration.identity(),
                    self.registration.resources().route_state(),
                    self.thing_slot,
                    &self.limits,
                    self.derived
                        .as_ref()
                        .expect("first entry retains the route projection"),
                    &handler.target,
                    None,
                    handler.footprint,
                    self.route.key,
                    self.activation.as_ref(),
                    &mut self.admission,
                    matches!(&self.route.state, HostRouteState::Absent) && self.in_flight.is_none(),
                    self.registration.execution().supports_host_erased(),
                    self.registration
                        .capabilities()
                        .supports_producer_property_read(),
                    self.registration.status().retained_records(),
                ) {
                    self.activation = None;
                    self.derived = None;
                    self.route.key = None;
                    return self.fail_without_route(error);
                }
                let mut call = match self.registration.server().prepare(prepare, artifact) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (_, error) = rejection.into_parts();
                        return self.fail_binding_without_route(error);
                    }
                };
                if !self.admission.admits_host_call(&mut call) {
                    drop(call);
                    return self.fail_without_route(validation_error(self.thing_slot));
                }
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
        let response = RouteInboundResponse::seal_property_read_handler_result(opportunity, result);
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
        let mut call = match self.registration.server().deliver_response(response) {
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
        self.route.state = if self.admission.admits_host_call(&mut call) {
            HostRouteState::Delivering { guard, call }
        } else {
            self.disposition
                .record_core_failure(validation_error(self.thing_slot));
            self.expose = ExposeState::Draining;
            HostRouteState::RejectingDelivery { guard, call }
        };
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
            HostRouteState::Preparing(call) if self.expose == ExposeState::Cancelling => {
                self.start_cancel_prepare(call, cx, budget)
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
            HostRouteState::CancellingPrepare(mut call) => {
                match call.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::CancellingPrepare(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(Ok(settlement)) => self.settle_prepare(call, settlement),
                    Poll::Ready(Err(error)) => {
                        self.disposition.record_core_failure(error);
                        self.route.state = HostRouteState::CancellingPrepare(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
            HostRouteState::JoiningPrepare(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::JoiningPrepare(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(RoutePrepareOutcome::Prepared(guard)) => {
                        drop(call);
                        self.start_abort(guard)
                    }
                    Poll::Ready(RoutePrepareOutcome::RejectedNoResource(error)) => {
                        drop(call);
                        self.fail_binding_without_route(error)
                    }
                }
            }
            HostRouteState::Prepared(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_abort(guard);
                }
                let mut call = match self.registration.server().start_readiness(guard) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (guard, error) = rejection.into_parts();
                        return self.start_failed_abort(guard, error);
                    }
                };
                if !self.admission.admits_host_call(&mut call) {
                    self.disposition
                        .record_core_failure(validation_error(self.thing_slot));
                    self.expose = ExposeState::Cancelling;
                    return self.start_cancel_readiness(call, cx, budget);
                }
                self.route.state = HostRouteState::AwaitingReadiness(call);
                progress(PendingWorkClass::RouteReadiness)
            }
            HostRouteState::AwaitingReadiness(call) if self.expose == ExposeState::Cancelling => {
                self.start_cancel_readiness(call, cx, budget)
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
            HostRouteState::CancellingReadiness(mut call) => {
                match call.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::CancellingReadiness(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(Ok(settlement)) => self.settle_readiness(call, settlement),
                    Poll::Ready(Err(error)) => {
                        self.disposition.record_core_failure(error);
                        self.route.state = HostRouteState::CancellingReadiness(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
            HostRouteState::JoiningReadiness(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::JoiningReadiness(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(RouteReadinessOutcome::Ready(guard)) => {
                        drop(call);
                        self.start_abort(guard)
                    }
                    Poll::Ready(RouteReadinessOutcome::Failed { guard, error }) => {
                        drop(call);
                        self.start_failed_abort(guard, error)
                    }
                }
            }
            HostRouteState::Ready(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_abort(guard);
                }
                let mut call = match self.registration.server().activate(guard) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (guard, error) = rejection.into_parts();
                        return self.start_failed_abort(guard, error);
                    }
                };
                if !self.admission.admits_host_call(&mut call) {
                    self.disposition
                        .record_core_failure(validation_error(self.thing_slot));
                    self.expose = ExposeState::Cancelling;
                    return self.start_cancel_activation(call, cx, budget);
                }
                self.route.state = HostRouteState::Activating(call);
                progress(PendingWorkClass::BindingInput)
            }
            HostRouteState::Activating(call) if self.expose == ExposeState::Cancelling => {
                self.start_cancel_activation(call, cx, budget)
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
            HostRouteState::CancellingActivation(mut call) => {
                match call.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::CancellingActivation(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(Ok(settlement)) => self.settle_activation(call, settlement),
                    Poll::Ready(Err(error)) => {
                        self.disposition.record_core_failure(error);
                        self.route.state = HostRouteState::CancellingActivation(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
            HostRouteState::JoiningActivation(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::JoiningActivation(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(RouteActivationOutcome::Active(guard)) => {
                        drop(call);
                        self.start_shutdown(HostShutdownRouteGuard::Active(guard))
                    }
                    Poll::Ready(RouteActivationOutcome::NotActivated { guard, error }) => {
                        drop(call);
                        self.start_failed_abort(guard, error)
                    }
                }
            }
            HostRouteState::Active(guard) => {
                if self.expose == ExposeState::Cancelling {
                    return self.start_shutdown(HostShutdownRouteGuard::Active(guard));
                }
                let mut call = match self.registration.server().commit(guard) {
                    Ok(call) => call,
                    Err(rejection) => {
                        let (guard, error) = rejection.into_parts();
                        return self
                            .start_failed_shutdown(HostShutdownRouteGuard::Active(guard), error);
                    }
                };
                if !self.admission.admits_host_call(&mut call) {
                    self.disposition
                        .record_core_failure(validation_error(self.thing_slot));
                    self.expose = ExposeState::Cancelling;
                    return self.start_cancel_commit(call, cx, budget);
                }
                self.route.state = HostRouteState::Committing(call);
                progress(PendingWorkClass::BindingInput)
            }
            HostRouteState::Committing(call) if self.expose == ExposeState::Cancelling => {
                self.start_cancel_commit(call, cx, budget)
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
            HostRouteState::CancellingCommit(mut call) => {
                match call.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::CancellingCommit(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(Ok(settlement)) => self.settle_commit(call, settlement),
                    Poll::Ready(Err(error)) => {
                        self.disposition.record_core_failure(error);
                        self.route.state = HostRouteState::CancellingCommit(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
            HostRouteState::JoiningCommit(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::JoiningCommit(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(RouteCommitOutcome::Committed(guard)) => {
                        drop(call);
                        self.start_shutdown(HostShutdownRouteGuard::Committed(guard))
                    }
                    Poll::Ready(RouteCommitOutcome::NotCommitted { guard, error }) => {
                        drop(call);
                        self.start_failed_shutdown(HostShutdownRouteGuard::Active(guard), error)
                    }
                }
            }
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
            HostRouteState::Delivering { guard, call } if self.expose == ExposeState::Draining => {
                self.start_cancel_delivery(call, guard, cx, budget)
            }
            HostRouteState::Delivering { guard, mut call } => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::Delivering { guard, call };
                        progress(PendingWorkClass::ResponseDelivery)
                    }
                    Poll::Ready(outcome) => self.finish_delivery_result(guard, outcome),
                }
            }
            HostRouteState::RejectingDelivery { guard, call } => {
                self.start_cancel_delivery(call, guard, cx, budget)
            }
            HostRouteState::CancellingDelivery { guard, mut call } => {
                match call.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::CancellingDelivery { guard, call };
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(Ok(settlement)) => self.settle_delivery(call, guard, settlement),
                    Poll::Ready(Err(error)) => {
                        self.disposition.record_core_failure(error);
                        self.route.state = HostRouteState::CancellingDelivery { guard, call };
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
            HostRouteState::JoiningDelivery { guard, mut call } => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::JoiningDelivery { guard, call };
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(outcome) => self.finish_delivery_result(guard, outcome),
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
            HostRouteState::RejectingCleanup(call) => {
                self.start_cancel_cleanup_call(call, cx, budget)
            }
            HostRouteState::CancellingCleanup(mut call) => {
                match call.as_pin_mut().poll_cancel(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::CancellingCleanup(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(Ok(settlement)) => self.settle_cleanup_call(call, settlement),
                    Poll::Ready(Err(error)) => {
                        self.disposition.record_core_failure(error);
                        self.route.state = HostRouteState::CancellingCleanup(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                }
            }
            HostRouteState::JoiningCleanup(mut call) => {
                match call.as_pin_mut().poll_result(cx, budget) {
                    Poll::Pending => {
                        self.route.state = HostRouteState::JoiningCleanup(call);
                        progress(PendingWorkClass::Cleanup)
                    }
                    Poll::Ready(outcome) => self.finish_cleanup(outcome),
                }
            }
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
