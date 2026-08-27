use std::{
    collections::BTreeMap,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};
#[cfg(feature = "std")]
use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

use clinkz_wot_core::{
    AffordanceTarget, BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCallSettlement, BindingCancellationDisposition, BindingCompilerBounds,
    BindingCompilerExtension, BindingCompilerInput, BindingCompilerOutput, BindingCompilerStep,
    BindingConfigurationDigest, BindingDeliveryOutcome, BindingExecutionSupport, BindingGeneration,
    BindingId, BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint,
    BindingOperationalError, BindingRegistrationCapabilities, BindingRegistrationIdentity,
    BindingResourceDeclarations, BindingStateLayout, BindingStatusPolicy, CleanupOperation,
    CleanupPhaseContext, CleanupReservation, CleanupSlotId, CoreError, CoreResult, Deadline,
    ErrorContext, ErrorPhase, InteractionOutput, NoCleanupSuccessor, OutboundRequest, Payload,
    PlanId, PlanSetGeneration, PollClientBinding, PollServerBinding, PrepareInput, RetryClass,
    RouteAcceptEvent, RouteActivationOutcome, RouteActivationPermit, RouteCleanupOutcome,
    RouteCommitOutcome, RouteInboundResponse, RoutePrepareOutcome, RouteReadinessOutcome,
    RouteReadinessSlot, ServerResponseSlot, ServerRouteSlot, StartStatus,
    StaticBindingCompilerRegistration, StaticBindingComponents, StaticBindingRegistration,
    StaticBindingRegistrationInput, ThingId,
};
#[cfg(feature = "std")]
use clinkz_wot_core::{
    HostActiveRouteGuard, HostBindingArtifact, HostBindingCall, HostBindingCallBox,
    HostBindingCompilerRegistration, HostBindingRegistration, HostBindingRegistrationInput,
    HostCommittedRouteGuard, HostPreparedRouteGuard, HostRouteCleanupSuccessor,
    LogicalInteractionPlan, RouteAbortInput, RouteServerBinding, RouteShutdownInput,
    binding::ClientBinding as TargetClientBinding,
};
use clinkz_wot_foundation::{
    ClockId, Generation, MonotonicInstant, SlotIndex, WorkBudget, WorkClass,
};

const COMPATIBILITY: BindingArtifactCompatibility = BindingArtifactCompatibility::new([41; 16]);

#[derive(Debug, Eq, PartialEq)]
struct TestArtifact {
    resolved_target: Box<str>,
}

#[derive(Debug)]
struct TestCompiler;

impl BindingCompilerExtension for TestCompiler {
    type Cursor = ();
    type Artifact = TestArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }

    fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, 64),
            0,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }

    fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        Ok(())
    }

    fn step(
        &self,
        _: &BindingCompilerInput<'_>,
        _: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(());
        }
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            COMPATIBILITY,
            BindingArtifactFootprint::new(1, 32),
            TestArtifact {
                resolved_target: Box::from("mock://sensor/temperature"),
            },
        )))
    }

    fn abort(&self, _: Self::Cursor) {}
}

fn plan_id(slot: u32) -> PlanId {
    PlanId::new(SlotIndex::new(slot), Generation::INITIAL)
}

fn artifact_identity(plan_slot: u32) -> BindingArtifactIdentity {
    BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL,
        plan_id(plan_slot),
        BindingId::new(43),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([47; 32]),
        COMPATIBILITY,
        BindingArtifactRole::ConsumerCall,
    )
}

fn artifact_ref(plan_slot: u32) -> BindingArtifactRef {
    BindingArtifactRef::new(artifact_identity(plan_slot), SlotIndex::new(53))
}

fn static_artifact(plan_slot: u32) -> BindingArtifactEnvelope<TestArtifact> {
    BindingArtifactEnvelope::try_new(
        artifact_identity(plan_slot),
        BindingArtifactFootprint::new(1, 64),
        BindingArtifact::new(
            COMPATIBILITY,
            BindingArtifactFootprint::new(1, 32),
            TestArtifact {
                resolved_target: Box::from("mock://sensor/temperature"),
            },
        ),
    )
    .unwrap()
}

#[cfg(feature = "std")]
fn host_artifact(plan_slot: u32) -> BindingArtifactEnvelope<HostBindingArtifact> {
    let plan = LogicalInteractionPlan::try_property_read(
        plan_id(plan_slot),
        ThingId::from("urn:test:consumer-binding"),
        Box::from("temperature"),
        0,
        Box::from("mock://sensor/temperature"),
        Some(Box::from("application/json")),
        None,
    )
    .unwrap();
    let candidate = clinkz_wot_core::BindingCandidate::new(
        BindingId::new(43),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([47; 32]),
        COMPATIBILITY,
        0,
        0,
    );
    let input = BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ConsumerCall);
    let compiler = HostBindingCompilerRegistration::new(TestCompiler);
    let cursor = compiler.start(&input).unwrap();
    let mut budget = binding_budget(1);
    let BindingCompilerStep::Complete(output) = compiler.step(&input, cursor, &mut budget) else {
        panic!("one compiler poll completes the host artifact")
    };
    BindingArtifactEnvelope::try_new(
        artifact_identity(plan_slot),
        BindingArtifactFootprint::new(1, 64),
        output.into_artifact(),
    )
    .unwrap()
}

fn request(plan_slot: u32) -> OutboundRequest {
    let mut uri_variables = BTreeMap::new();
    uri_variables.insert(String::from("room"), String::from("west"));
    OutboundRequest::property_read(
        ThingId::from("urn:test:consumer-binding"),
        AffordanceTarget::Property(Arc::from("temperature")),
        artifact_ref(plan_slot),
        uri_variables,
        Some(Deadline::at(MonotonicInstant::new(ClockId::new(5), 101))),
    )
    .unwrap()
}

fn output() -> InteractionOutput {
    InteractionOutput::with_data(Payload::new(Vec::from(&b"23.5"[..]), "application/json"))
}

fn binding_budget(polls: u64) -> WorkBudget {
    WorkBudget::new().with_remaining(WorkClass::BindingPolls, polls)
}

fn binding_error(code: u16) -> BindingOperationalError {
    BindingOperationalError::new(CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(code, "test binding rejected input"),
    ))
}

fn cleanup_phase() -> CleanupPhaseContext {
    CleanupPhaseContext::bind(
        CleanupReservation::new(
            CleanupSlotId::new(SlotIndex::new(59), Generation::INITIAL),
            BindingLifetimeFootprint::new(1, 64),
            1,
            binding_budget(2),
        ),
        CleanupOperation::CancelRequest,
        CoreError::TimedOut(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)),
        Deadline::NONE,
    )
}

fn registration_identity(
    compatibility: BindingArtifactCompatibility,
) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(43),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([47; 32]),
        compatibility,
        0,
    )
}

fn resources() -> BindingResourceDeclarations {
    let admitted = BindingLifetimeFootprint::new(4, 256);
    BindingResourceDeclarations::new(BindingLifetimeFootprint::new(2, 128), admitted)
        .with_state_footprints(admitted, admitted, admitted)
}

#[derive(Debug)]
struct NoopServer {
    compatibility: BindingArtifactCompatibility,
}

impl PollServerBinding for NoopServer {
    type Compiler = TestCompiler;
    type RouteState = ();
    type ReadinessState = ();
    type ResponseState = ();

    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn route_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(0, 0))
    }
    fn readiness_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(0, 0))
    }
    fn response_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(0, 0))
    }
    fn start_prepare(
        &mut self,
        _: PrepareInput,
        _: &BindingArtifactEnvelope<TestArtifact>,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Result<StartStatus<RoutePrepareOutcome<()>>, BindingInputRejection<PrepareInput>> {
        unimplemented!()
    }
    fn poll_prepare(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<()>> {
        unimplemented!()
    }
    fn poll_cancel_prepare(
        &mut self,
        _: &mut Context<'_>,
        _: &CleanupPhaseContext,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RoutePrepareOutcome<()>, ()>>> {
        unimplemented!()
    }
    fn start_readiness(
        &mut self,
        _: &mut ServerRouteSlot<()>,
        _: &mut RouteReadinessSlot<()>,
        _: &mut WorkBudget,
    ) -> StartStatus<RouteReadinessOutcome<()>> {
        unimplemented!()
    }
    fn poll_readiness(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: &mut RouteReadinessSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<RouteReadinessOutcome<()>> {
        unimplemented!()
    }
    fn poll_cancel_readiness(
        &mut self,
        _: &mut Context<'_>,
        _: &CleanupPhaseContext,
        _: &mut ServerRouteSlot<()>,
        _: &mut RouteReadinessSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteReadinessOutcome<()>, ()>>> {
        unimplemented!()
    }
    fn start_activate(
        &mut self,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> StartStatus<RouteActivationOutcome<(), ()>> {
        unimplemented!()
    }
    fn poll_activate(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<RouteActivationOutcome<(), ()>> {
        unimplemented!()
    }
    fn poll_cancel_activate(
        &mut self,
        _: &mut Context<'_>,
        _: &CleanupPhaseContext,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteActivationOutcome<(), ()>, ()>>> {
        unimplemented!()
    }
    fn start_commit(
        &mut self,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> StartStatus<RouteCommitOutcome<(), ()>> {
        unimplemented!()
    }
    fn poll_commit(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<RouteCommitOutcome<(), ()>> {
        unimplemented!()
    }
    fn poll_cancel_commit(
        &mut self,
        _: &mut Context<'_>,
        _: &CleanupPhaseContext,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteCommitOutcome<(), ()>, ()>>> {
        unimplemented!()
    }
    fn poll_accept(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: RouteActivationPermit<'_>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>> {
        unimplemented!()
    }
    fn start_abort(
        &mut self,
        _: CleanupPhaseContext,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome> {
        unimplemented!()
    }
    fn poll_abort(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        unimplemented!()
    }
    fn start_shutdown(
        &mut self,
        _: CleanupPhaseContext,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome> {
        unimplemented!()
    }
    fn poll_shutdown(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerRouteSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        unimplemented!()
    }
    fn acknowledge_route(&mut self, _: &mut ServerRouteSlot<()>) -> CoreResult<()> {
        unimplemented!()
    }
    fn start_response(
        &mut self,
        _: RouteInboundResponse,
        _: &mut ServerResponseSlot<()>,
        _: &mut WorkBudget,
    ) -> Result<StartStatus<BindingDeliveryOutcome>, BindingInputRejection<RouteInboundResponse>>
    {
        unimplemented!()
    }
    fn poll_response(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ServerResponseSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<BindingDeliveryOutcome> {
        unimplemented!()
    }
    fn poll_cancel_response(
        &mut self,
        _: &mut Context<'_>,
        _: &CleanupPhaseContext,
        _: &mut ServerResponseSlot<()>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>> {
        unimplemented!()
    }
    fn acknowledge_response(&mut self, _: &mut ServerResponseSlot<()>) -> CoreResult<()> {
        unimplemented!()
    }
}

#[cfg(feature = "std")]
struct NoopHostServer {
    compatibility: BindingArtifactCompatibility,
}

#[cfg(feature = "std")]
impl RouteServerBinding for NoopHostServer {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn prepare(
        &self,
        _: PrepareInput,
        _: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>,
        BindingInputRejection<PrepareInput>,
    > {
        unimplemented!()
    }
    fn start_readiness(
        &self,
        _: HostPreparedRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteReadinessOutcome<HostPreparedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
        BindingInputRejection<HostPreparedRouteGuard>,
    > {
        unimplemented!()
    }
    fn activate(
        &self,
        _: HostPreparedRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
        BindingInputRejection<HostPreparedRouteGuard>,
    > {
        unimplemented!()
    }
    fn commit(
        &self,
        _: HostActiveRouteGuard,
    ) -> Result<
        HostBindingCallBox<
            RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
            HostRouteCleanupSuccessor,
        >,
        BindingInputRejection<HostActiveRouteGuard>,
    > {
        unimplemented!()
    }
    fn poll_accept(
        &self,
        _: &HostCommittedRouteGuard,
        _: RouteActivationPermit<'_>,
        _: &mut Context<'_>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>> {
        unimplemented!()
    }
    fn abort(
        &self,
        _: RouteAbortInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteAbortInput>,
    > {
        unimplemented!()
    }
    fn shutdown(
        &self,
        _: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteShutdownInput>,
    > {
        unimplemented!()
    }
    fn deliver_response(
        &self,
        _: RouteInboundResponse,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<RouteInboundResponse>,
    > {
        unimplemented!()
    }
}

#[derive(Clone, Copy)]
enum StaticMode {
    Pending,
    Synchronous,
    Reject,
    LateOnCancel,
}

struct StaticState {
    output: Option<InteractionOutput>,
    derived_target_len: usize,
    acknowledged: bool,
}

struct StaticClient {
    compatibility: BindingArtifactCompatibility,
    mode: StaticMode,
    calls: usize,
    lifetime: BindingLifetimeFootprint,
}

impl StaticClient {
    fn new(mode: StaticMode) -> Self {
        Self {
            compatibility: COMPATIBILITY,
            mode,
            calls: 0,
            lifetime: BindingLifetimeFootprint::new(1, 64),
        }
    }
}

impl PollClientBinding for StaticClient {
    type Compiler = TestCompiler;
    type RequestState = StaticState;

    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<StaticState>(self.lifetime)
    }

    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<TestArtifact>,
        slot: &mut clinkz_wot_core::ClientRequestSlot<StaticState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        if self.compatibility != artifact.identity().compatibility()
            || request.artifact().identity() != artifact.identity()
            || artifact.artifact().compatibility() != self.compatibility
        {
            return Err(BindingInputRejection::new(request, binding_error(401)));
        }
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Err(BindingInputRejection::new(request, binding_error(402)));
        }
        self.calls += 1;
        if matches!(self.mode, StaticMode::Reject) {
            return Err(BindingInputRejection::new(request, binding_error(403)));
        }
        if matches!(self.mode, StaticMode::Synchronous) {
            return Ok(StartStatus::Ready(Ok(output())));
        }
        slot.initialize(
            request,
            StaticState {
                output: Some(output()),
                derived_target_len: artifact.artifact().payload().resolved_target.len(),
                acknowledged: false,
            },
        );
        Ok(StartStatus::Pending)
    }

    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut clinkz_wot_core::ClientRequestSlot<StaticState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(slot.state_mut().output.take().unwrap()))
    }

    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        slot: &mut clinkz_wot_core::ClientRequestSlot<StaticState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        if matches!(self.mode, StaticMode::LateOnCancel) {
            return Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(
                slot.state_mut().output.take().unwrap(),
            ))));
        }
        Ok(StartStatus::Pending)
    }

    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut clinkz_wot_core::ClientRequestSlot<StaticState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: NoCleanupSuccessor,
            },
        }))
    }

    fn acknowledge_request(
        &mut self,
        slot: &mut clinkz_wot_core::ClientRequestSlot<StaticState>,
    ) -> CoreResult<()> {
        slot.state_mut().acknowledged = true;
        Ok(())
    }
}

#[derive(Clone, Copy)]
#[cfg(feature = "std")]
enum HostMode {
    Result,
    Cancel,
    LateOnCancel,
}

#[cfg(feature = "std")]
struct HostCall {
    output: Option<InteractionOutput>,
    mode: HostMode,
    side_effects: Arc<AtomicUsize>,
    deadline: Option<Deadline>,
}

#[cfg(feature = "std")]
impl HostBindingCall<CoreResult<InteractionOutput>> for HostCall {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        BindingLifetimeFootprint::new(1, 64)
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(Ok(self.output.take().unwrap()))
    }

    fn start_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        if matches!(self.mode, HostMode::LateOnCancel) {
            return Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(
                self.output.take().unwrap(),
            ))));
        }
        Ok(StartStatus::Pending)
    }

    fn poll_cancel(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: NoCleanupSuccessor,
            },
        }))
    }

    fn next_deadline(&self) -> Option<Deadline> {
        self.deadline
    }
}

#[cfg(feature = "std")]
struct HostClient {
    compatibility: BindingArtifactCompatibility,
    mode: HostMode,
    accepted: Arc<AtomicUsize>,
    side_effects: Arc<AtomicUsize>,
}

#[cfg(feature = "std")]
impl TargetClientBinding for HostClient {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        let compatible = self.compatibility == artifact.identity().compatibility()
            && request.artifact().identity() == artifact.identity()
            && artifact
                .artifact()
                .try_payload::<TestArtifact>(self.compatibility)
                .is_some();
        if !compatible {
            return Err(BindingInputRejection::new(request, binding_error(411)));
        }
        self.accepted.fetch_add(1, Ordering::SeqCst);
        Ok(HostBindingCallBox::new(HostCall {
            output: Some(output()),
            mode: self.mode,
            side_effects: Arc::clone(&self.side_effects),
            deadline: request.deadline(),
        }))
    }
}

fn static_registration(
    client: StaticClient,
) -> StaticBindingRegistration<StaticBindingComponents<NoopServer, StaticClient>> {
    let input = StaticBindingRegistrationInput::producer_and_consumer_property_read(
        registration_identity(COMPATIBILITY),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(TestCompiler),
        StaticBindingComponents::new(
            NoopServer {
                compatibility: COMPATIBILITY,
            },
            client,
        ),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(1, 64),
    );
    StaticBindingRegistration::producer_and_consumer_property_read(input)
        .unwrap_or_else(|_| panic!("valid static dual-role registration"))
}

#[cfg(feature = "std")]
fn host_registration(
    mode: HostMode,
    compatibility: BindingArtifactCompatibility,
    accepted: Arc<AtomicUsize>,
    side_effects: Arc<AtomicUsize>,
) -> Result<HostBindingRegistration, BindingInputRejection<HostBindingRegistrationInput>> {
    HostBindingRegistration::new(
        HostBindingRegistrationInput::producer_and_consumer_property_read(
            registration_identity(COMPATIBILITY),
            BindingExecutionSupport::host_erased(),
            HostBindingCompilerRegistration::new(TestCompiler),
            Box::new(NoopHostServer {
                compatibility: COMPATIBILITY,
            }),
            Box::new(HostClient {
                compatibility,
                mode,
                accepted,
                side_effects,
            }),
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(1, 64),
        ),
    )
}

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn context() -> Context<'static> {
    let waker = Waker::from(Arc::new(NoopWake));
    Context::from_waker(Box::leak(Box::new(waker)))
}

#[test]
fn static_external_authoring_covers_sync_pending_rejection_budget_and_reuse() {
    let artifact = static_artifact(61);

    let mut synchronous = static_registration(StaticClient::new(StaticMode::Synchronous));
    let mut slot = clinkz_wot_core::ClientRequestSlot::new();
    assert!(matches!(
        synchronous.server_mut().client_mut().start_request(
            request(61),
            &artifact,
            &mut slot,
            &mut binding_budget(1)
        ),
        Ok(StartStatus::Ready(Ok(_)))
    ));
    assert!(slot.is_vacant());

    let mut rejected = static_registration(StaticClient::new(StaticMode::Reject));
    let rejected_request = rejected
        .server_mut()
        .client_mut()
        .start_request(request(61), &artifact, &mut slot, &mut binding_budget(1))
        .expect_err("pre-acceptance rejection returns the exact request")
        .into_input();
    assert_eq!(rejected_request, request(61));
    assert!(slot.is_vacant());

    let mut pending = static_registration(StaticClient::new(StaticMode::Pending));
    let zero_budget = pending
        .server_mut()
        .client_mut()
        .start_request(request(61), &artifact, &mut slot, &mut binding_budget(0))
        .expect_err("zero budget preserves ownership without slot mutation");
    assert_eq!(zero_budget.input(), &request(61));
    assert!(slot.is_vacant());
    assert_eq!(pending.server().client().calls, 0);

    let wrong_artifact = static_artifact(60);
    let mismatch = pending
        .server_mut()
        .client_mut()
        .start_request(
            request(61),
            &wrong_artifact,
            &mut slot,
            &mut binding_budget(1),
        )
        .expect_err("the exact artifact identity is checked before acceptance");
    assert_eq!(mismatch.input(), &request(61));
    assert!(slot.is_vacant());
    assert_eq!(pending.server().client().calls, 0);

    assert_eq!(
        pending
            .server_mut()
            .client_mut()
            .start_request(request(61), &artifact, &mut slot, &mut binding_budget(1))
            .unwrap(),
        StartStatus::Pending
    );
    assert_eq!(slot.request().plan_id(), plan_id(61));
    assert_eq!(
        slot.state_mut().derived_target_len,
        "mock://sensor/temperature".len()
    );
    let mut cx = context();
    assert!(matches!(
        pending
            .server_mut()
            .client_mut()
            .poll_request(&mut cx, &mut slot, &mut binding_budget(0)),
        Poll::Pending
    ));
    assert!(matches!(
        pending
            .server_mut()
            .client_mut()
            .poll_request(&mut cx, &mut slot, &mut binding_budget(1)),
        Poll::Ready(Ok(_))
    ));
    pending
        .server_mut()
        .client_mut()
        .acknowledge_request(&mut slot)
        .unwrap();
    assert!(slot.state_mut().acknowledged);
    slot.clear();

    let next_artifact = static_artifact(62);
    assert_eq!(
        pending
            .server_mut()
            .client_mut()
            .start_request(
                request(62),
                &next_artifact,
                &mut slot,
                &mut binding_budget(1)
            )
            .unwrap(),
        StartStatus::Pending
    );
    assert_eq!(slot.request().plan_id(), plan_id(62));
}

#[test]
fn static_cancellation_retains_late_output_and_explicit_settlement() {
    let artifact = static_artifact(63);
    let mut late = static_registration(StaticClient::new(StaticMode::LateOnCancel));
    let mut slot = clinkz_wot_core::ClientRequestSlot::new();
    late.server_mut()
        .client_mut()
        .start_request(request(63), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap_or_else(|_| panic!("valid host dual-role registration"));
    let mut cx = context();
    let settlement = late
        .server_mut()
        .client_mut()
        .start_cancel_request(&mut cx, cleanup_phase(), &mut slot, &mut binding_budget(1))
        .unwrap_or_else(|_| panic!("valid host late-result registration"));
    assert!(matches!(
        settlement,
        StartStatus::Ready(BindingCallSettlement::Returned(Ok(_)))
    ));
    late.server_mut()
        .client_mut()
        .acknowledge_request(&mut slot)
        .unwrap();
    slot.clear();

    let mut cancelled = static_registration(StaticClient::new(StaticMode::Pending));
    cancelled
        .server_mut()
        .client_mut()
        .start_request(request(63), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap_or_else(|_| panic!("valid host cancellation registration"));
    assert_eq!(
        cancelled
            .server_mut()
            .client_mut()
            .start_cancel_request(&mut cx, cleanup_phase(), &mut slot, &mut binding_budget(1))
            .unwrap(),
        StartStatus::Pending
    );
    assert!(matches!(
        cancelled.server_mut().client_mut().poll_cancel_request(
            &mut cx,
            &mut slot,
            &mut binding_budget(1)
        ),
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            disposition: BindingCancellationDisposition::Complete { .. },
            ..
        }))
    ));
}

#[cfg(feature = "std")]
#[test]
fn host_external_authoring_preserves_constructor_and_call_ownership() {
    let accepted = Arc::new(AtomicUsize::new(0));
    let side_effects = Arc::new(AtomicUsize::new(0));
    let registration = host_registration(
        HostMode::Result,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid host dual-role registration"));
    assert!(
        registration
            .capabilities()
            .supports_consumer_property_read()
    );
    let artifact = host_artifact(67);
    let mut call = registration
        .client()
        .unwrap()
        .invoke(request(67), &artifact)
        .unwrap();
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    assert_eq!(side_effects.load(Ordering::SeqCst), 0);
    assert_eq!(
        call.as_pin_mut().lifetime_footprint(),
        BindingLifetimeFootprint::new(1, 64)
    );
    assert_eq!(
        call.as_pin_mut().next_deadline(),
        Some(Deadline::at(MonotonicInstant::new(ClockId::new(5), 101)))
    );
    let mut cx = context();
    assert!(matches!(
        call.as_pin_mut()
            .poll_result(&mut cx, &mut binding_budget(1)),
        Poll::Ready(Ok(_))
    ));
    assert_eq!(side_effects.load(Ordering::SeqCst), 1);

    let wrong = host_artifact(68);
    let rejected = registration
        .client()
        .unwrap()
        .invoke(request(67), &wrong)
        .expect_err("identity mismatch rejects before protocol work");
    assert_eq!(rejected.into_input(), request(67));
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    assert_eq!(side_effects.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "std")]
#[test]
fn host_cancellation_has_only_late_return_or_explicit_cleanup_settlement() {
    let accepted = Arc::new(AtomicUsize::new(0));
    let side_effects = Arc::new(AtomicUsize::new(0));
    let late = host_registration(
        HostMode::LateOnCancel,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid host late-result registration"));
    let artifact = host_artifact(71);
    let mut call = late
        .client()
        .unwrap()
        .invoke(request(71), &artifact)
        .unwrap();
    let mut cx = context();
    assert!(matches!(
        call.as_pin_mut()
            .start_cancel(&mut cx, cleanup_phase(), &mut binding_budget(1))
            .unwrap(),
        StartStatus::Ready(BindingCallSettlement::Returned(Ok(_)))
    ));

    let cancelled = host_registration(
        HostMode::Cancel,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid host cancellation registration"));
    let mut call = cancelled
        .client()
        .unwrap()
        .invoke(request(71), &artifact)
        .unwrap();
    assert_eq!(
        call.as_pin_mut()
            .start_cancel(&mut cx, cleanup_phase(), &mut binding_budget(1))
            .unwrap(),
        StartStatus::Pending
    );
    assert!(matches!(
        call.as_pin_mut()
            .poll_cancel(&mut cx, &mut binding_budget(1)),
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            disposition: BindingCancellationDisposition::Complete { .. },
            ..
        }))
    ));
}

#[test]
fn dual_registration_rejects_mismatch_and_oversized_request_state() {
    let other = BindingArtifactCompatibility::new([99; 16]);
    #[cfg(feature = "std")]
    assert!(
        host_registration(
            HostMode::Result,
            other,
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        )
        .is_err()
    );

    let mut client = StaticClient::new(StaticMode::Pending);
    client.compatibility = other;
    let mismatch = StaticBindingRegistrationInput::producer_and_consumer_property_read(
        registration_identity(COMPATIBILITY),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(TestCompiler),
        StaticBindingComponents::new(
            NoopServer {
                compatibility: COMPATIBILITY,
            },
            client,
        ),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(1, 64),
    );
    assert!(StaticBindingRegistration::producer_and_consumer_property_read(mismatch).is_err());

    let mut oversized = StaticClient::new(StaticMode::Pending);
    oversized.lifetime = BindingLifetimeFootprint::new(5, 257);
    let oversized = StaticBindingRegistrationInput::producer_and_consumer_property_read(
        registration_identity(COMPATIBILITY),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(TestCompiler),
        StaticBindingComponents::new(
            NoopServer {
                compatibility: COMPATIBILITY,
            },
            oversized,
        ),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(1, 64),
    );
    assert!(StaticBindingRegistration::producer_and_consumer_property_read(oversized).is_err());
}

#[test]
fn producer_only_registration_apis_remain_source_and_behavior_compatible() {
    let static_input = StaticBindingRegistrationInput::new(
        registration_identity(COMPATIBILITY),
        BindingRegistrationCapabilities::producer_property_read(),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(TestCompiler),
        NoopServer {
            compatibility: COMPATIBILITY,
        },
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(1, 64),
    );
    let static_registration = StaticBindingRegistration::new(static_input)
        .unwrap_or_else(|_| panic!("valid legacy Producer-only static registration"));
    assert!(
        !static_registration
            .capabilities()
            .supports_consumer_property_read()
    );

    #[cfg(feature = "std")]
    {
        let host_input = HostBindingRegistrationInput::new(
            registration_identity(COMPATIBILITY),
            BindingRegistrationCapabilities::producer_property_read(),
            BindingExecutionSupport::host_erased(),
            HostBindingCompilerRegistration::new(TestCompiler),
            Box::new(NoopHostServer {
                compatibility: COMPATIBILITY,
            }),
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(1, 64),
        );
        let host_registration = HostBindingRegistration::new(host_input)
            .unwrap_or_else(|_| panic!("valid legacy Producer-only host registration"));
        assert!(host_registration.client().is_none());
    }
}

#[cfg(all(feature = "std", feature = "async"))]
mod legacy_poison {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use clinkz_wot_core::{
        BindingRequest, ClientBinding, CoreResult, InteractionOutput, Subscription,
        SubscriptionGuard,
    };
    use clinkz_wot_td::{data_type::Operation, form::Form, thing::Thing};

    pub static LEGACY_CALLS: AtomicUsize = AtomicUsize::new(0);

    pub struct PoisonLegacyClient;

    #[async_trait::async_trait]
    impl ClientBinding for PoisonLegacyClient {
        fn supports(&self, _: &Form, _: Operation) -> bool {
            LEGACY_CALLS.fetch_add(1, Ordering::SeqCst);
            panic!("target path called legacy supports")
        }

        fn supports_with_thing(&self, _: &Thing, _: &Form, _: Operation) -> bool {
            LEGACY_CALLS.fetch_add(1, Ordering::SeqCst);
            panic!("target path called legacy supports_with_thing")
        }

        async fn invoke(&self, _: BindingRequest) -> CoreResult<InteractionOutput> {
            LEGACY_CALLS.fetch_add(1, Ordering::SeqCst);
            panic!("target path called legacy async invoke")
        }

        async fn subscribe(
            &self,
            _: BindingRequest,
        ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
            LEGACY_CALLS.fetch_add(1, Ordering::SeqCst);
            panic!("target path called legacy subscribe")
        }
    }
}

#[cfg(all(feature = "std", feature = "async"))]
#[test]
fn target_host_path_succeeds_with_every_legacy_client_entry_poisoned() {
    use legacy_poison::{LEGACY_CALLS, PoisonLegacyClient};

    let _poison = PoisonLegacyClient;
    let accepted = Arc::new(AtomicUsize::new(0));
    let side_effects = Arc::new(AtomicUsize::new(0));
    let registration = host_registration(HostMode::Result, COMPATIBILITY, accepted, side_effects)
        .unwrap_or_else(|_| panic!("valid target host registration"));
    let artifact = host_artifact(73);
    let _call = registration
        .client()
        .unwrap()
        .invoke(request(73), &artifact)
        .unwrap();
    assert_eq!(LEGACY_CALLS.load(Ordering::SeqCst), 0);
}
