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
    BindingResourceDeclarations, BindingResponseMetadata, BindingStateLayout, BindingStatusPolicy,
    BindingTransientFootprint, CleanupOperation, CleanupPhaseContext, CleanupReservation,
    CleanupSlotId, CoreError, CoreResult, Deadline, ErrorContext, ErrorPhase, InteractionOutput,
    InteractionOutputMetadata, InteractionStatus, NoCleanupSuccessor, OutboundRequest, Payload,
    PlanId, PlanSetGeneration, PollClientBinding, PollServerBinding, PrepareInput,
    ResponsePayloadRole, RetryClass, RouteAcceptEvent, RouteActivationOutcome,
    RouteActivationPermit, RouteCleanupOutcome, RouteCommitOutcome, RouteInboundResponse,
    RoutePrepareOutcome, RouteReadinessOutcome, RouteReadinessSlot, ServerResponseSlot,
    ServerRouteSlot, StartStatus, StaticBindingCompilerRegistration, StaticBindingComponents,
    StaticBindingRegistration, StaticBindingRegistrationInput, StaticConsumerPropertyReadSlot,
    ThingId,
};
#[cfg(feature = "std")]
use clinkz_wot_core::{
    CleanupHandle, CleanupRecord, CleanupTransferAcceptance, CleanupTransferEnvelope,
    CleanupTransferTarget, HostActiveRouteGuard, HostBindingArtifact, HostBindingCall,
    HostBindingCallBox, HostBindingCompilerRegistration, HostBindingRegistration,
    HostBindingRegistrationInput, HostCommittedRouteGuard, HostPreparedRouteGuard,
    HostRouteCleanupSuccessor, LogicalInteractionPlan, RouteAbortInput, RouteServerBinding,
    RouteShutdownInput, binding::ClientBinding as TargetClientBinding,
};
use clinkz_wot_foundation::{
    ClockId, GatewayDefaultV1, Generation, MonotonicInstant, ResourceKind, SlotIndex,
    StaticResourceProfile, WorkBudget, WorkClass,
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

#[derive(Clone, Copy, Debug)]
enum OutputKind {
    Valid,
    BindingId,
    BindingGeneration,
    PlanId,
    ResponseSelection,
    MissingMetadata,
    MissingPayload,
    Status,
    PayloadRole,
    ActionReference,
}

fn output_for(request: &OutboundRequest, kind: OutputKind) -> InteractionOutput {
    let binding_id = if matches!(kind, OutputKind::BindingId) {
        BindingId::new(request.binding_id().get() + 1)
    } else {
        request.binding_id()
    };
    let binding_generation = if matches!(kind, OutputKind::BindingGeneration) {
        request.binding_generation().checked_next().unwrap()
    } else {
        request.binding_generation()
    };
    let selected_plan = if matches!(kind, OutputKind::PlanId) {
        plan_id(request.plan_id().slot().get() + 1)
    } else {
        request.plan_id()
    };
    let response = if matches!(kind, OutputKind::ResponseSelection) {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .try_with_limit(ResourceKind::AdditionalResponsesPerFormMax, Some(1))
            .unwrap();
        BindingResponseMetadata::try_additional(
            binding_id,
            binding_generation,
            selected_plan,
            0,
            200,
            &limits,
        )
        .unwrap()
    } else {
        BindingResponseMetadata::primary(binding_id, binding_generation, selected_plan, 200)
    };
    let mut metadata = InteractionOutputMetadata::default();
    if !matches!(kind, OutputKind::MissingMetadata) {
        metadata = metadata.with_untrusted_binding_response(response);
    }
    if matches!(kind, OutputKind::PayloadRole) {
        metadata = metadata.with_payload_role(ResponsePayloadRole::OperationStatus);
    }
    if matches!(kind, OutputKind::ActionReference) {
        metadata = metadata.with_action_invocation(clinkz_wot_core::ActionInvocationRef::new(
            SlotIndex::new(79),
            Generation::INITIAL,
        ));
    }
    let output = if matches!(kind, OutputKind::MissingPayload) {
        InteractionOutput::empty()
    } else {
        InteractionOutput::with_data(Payload::new(Vec::from(&b"23.5"[..]), "application/json"))
    };
    let output = output.try_with_metadata(metadata).unwrap();
    if matches!(kind, OutputKind::Status) {
        output.with_status(InteractionStatus::Accepted)
    } else {
        output
    }
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

fn cleanup_phase_with_transfer_owner() -> CleanupPhaseContext {
    CleanupPhaseContext::bind_with_transfer_owner(
        CleanupReservation::new(
            CleanupSlotId::new(SlotIndex::new(59), Generation::INITIAL),
            BindingLifetimeFootprint::new(8, 2_048),
            1,
            binding_budget(2),
        ),
        CleanupSlotId::new(SlotIndex::new(60), Generation::INITIAL),
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
    let admitted = BindingLifetimeFootprint::new(8, 2_048);
    BindingResourceDeclarations::new(BindingLifetimeFootprint::new(2, 128), admitted)
        .with_state_footprints(admitted, admitted, admitted)
}

fn resources_with_admitted(
    admitted: BindingLifetimeFootprint,
    transient: BindingTransientFootprint,
) -> BindingResourceDeclarations {
    BindingResourceDeclarations::new(BindingLifetimeFootprint::new(1, 32), admitted)
        .with_state_footprints(admitted, admitted, admitted)
        .with_transient(transient)
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
    PendingFailure,
    SynchronousFailure,
    Reject,
    LateOnCancel,
    FabricatedTransfer,
}

struct StaticState {
    result: Option<CoreResult<InteractionOutput>>,
    derived_target_len: usize,
    acknowledged: bool,
}

struct StaticClient {
    compatibility: BindingArtifactCompatibility,
    mode: StaticMode,
    output_kind: OutputKind,
    calls: usize,
    lifetime: BindingLifetimeFootprint,
    transient: BindingTransientFootprint,
}

impl StaticClient {
    fn new(mode: StaticMode) -> Self {
        Self {
            compatibility: COMPATIBILITY,
            mode,
            output_kind: OutputKind::Valid,
            calls: 0,
            lifetime: BindingLifetimeFootprint::new(1, 64),
            transient: BindingTransientFootprint::new(0),
        }
    }

    fn with_output(mut self, output_kind: OutputKind) -> Self {
        self.output_kind = output_kind;
        self
    }

    fn with_transient(mut self, transient: BindingTransientFootprint) -> Self {
        self.transient = transient;
        self
    }
}

impl PollClientBinding for StaticClient {
    type Compiler = TestCompiler;
    type RequestState = StaticState;

    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<StaticState>(self.lifetime).with_transient(self.transient)
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
        let result = if matches!(
            self.mode,
            StaticMode::PendingFailure | StaticMode::SynchronousFailure
        ) {
            Err(binding_error(404).into_parts().1)
        } else {
            Ok(output_for(&request, self.output_kind))
        };
        if matches!(
            self.mode,
            StaticMode::Synchronous | StaticMode::SynchronousFailure
        ) {
            return Ok(StartStatus::Ready(result));
        }
        slot.initialize(
            request,
            StaticState {
                result: Some(result),
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
        Poll::Ready(slot.state_mut().result.take().unwrap())
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
            return Ok(StartStatus::Ready(BindingCallSettlement::Returned(
                slot.state_mut().result.take().unwrap(),
            )));
        }
        if matches!(self.mode, StaticMode::FabricatedTransfer) {
            let fabricated = CleanupPhaseContext::bind_with_transfer_owner(
                CleanupReservation::new(
                    CleanupSlotId::new(SlotIndex::new(80), Generation::INITIAL),
                    BindingLifetimeFootprint::new(1, 64),
                    1,
                    binding_budget(1),
                ),
                CleanupSlotId::new(SlotIndex::new(81), Generation::INITIAL),
                CleanupOperation::CancelRequest,
                CoreError::TimedOut(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never)),
                Deadline::NONE,
            )
            .try_into_transfer_request()
            .expect("the deliberately fabricated phase has an owner");
            return Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::TransferRequired(fabricated),
            }));
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
    Failure,
    Cancel,
    LateOnCancel,
    Transfer,
}

#[cfg(feature = "std")]
struct HostCall {
    result: Option<CoreResult<InteractionOutput>>,
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
        Poll::Ready(self.result.take().unwrap())
    }

    fn start_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        if matches!(self.mode, HostMode::LateOnCancel) {
            return Ok(StartStatus::Ready(BindingCallSettlement::Returned(
                self.result.take().unwrap(),
            )));
        }
        if matches!(self.mode, HostMode::Transfer) {
            let transfer = cleanup
                .try_into_transfer_request()
                .expect("Host transfer mode requires a production-provided owner");
            return Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::TransferRequired(transfer),
            }));
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
    output_kind: OutputKind,
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
        let result = if matches!(self.mode, HostMode::Failure) {
            Err(binding_error(412).into_parts().1)
        } else {
            Ok(output_for(&request, self.output_kind))
        };
        Ok(HostBindingCallBox::new(HostCall {
            result: Some(result),
            mode: self.mode,
            side_effects: Arc::clone(&self.side_effects),
            deadline: request.deadline(),
        }))
    }
}

#[cfg(feature = "std")]
fn host_client(
    mode: HostMode,
    output_kind: OutputKind,
    compatibility: BindingArtifactCompatibility,
    accepted: Arc<AtomicUsize>,
    side_effects: Arc<AtomicUsize>,
) -> HostClient {
    HostClient {
        compatibility,
        mode,
        output_kind,
        accepted,
        side_effects,
    }
}

type TestStaticComponents = StaticBindingComponents<NoopServer, StaticClient>;
type TestStaticRegistration = StaticBindingRegistration<TestStaticComponents>;
type TestStaticRegistrationInput = StaticBindingRegistrationInput<TestStaticComponents>;

fn static_registration(client: StaticClient) -> TestStaticRegistration {
    static_registration_with(client, resources())
        .unwrap_or_else(|_| panic!("valid static dual-role registration"))
}

fn static_registration_with(
    client: StaticClient,
    resources: BindingResourceDeclarations,
) -> Result<TestStaticRegistration, BindingInputRejection<TestStaticRegistrationInput>> {
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
        resources,
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(1, 64),
    );
    StaticBindingRegistration::producer_and_consumer_property_read(input)
}

#[cfg(feature = "std")]
fn host_registration(
    mode: HostMode,
    compatibility: BindingArtifactCompatibility,
    accepted: Arc<AtomicUsize>,
    side_effects: Arc<AtomicUsize>,
) -> Result<HostBindingRegistration, BindingInputRejection<HostBindingRegistrationInput>> {
    host_registration_with(
        mode,
        OutputKind::Valid,
        compatibility,
        accepted,
        side_effects,
        resources(),
    )
}

#[cfg(feature = "std")]
fn host_registration_with(
    mode: HostMode,
    output_kind: OutputKind,
    compatibility: BindingArtifactCompatibility,
    accepted: Arc<AtomicUsize>,
    side_effects: Arc<AtomicUsize>,
    resources: BindingResourceDeclarations,
) -> Result<HostBindingRegistration, BindingInputRejection<HostBindingRegistrationInput>> {
    HostBindingRegistration::new(
        HostBindingRegistrationInput::producer_and_consumer_property_read(
            registration_identity(COMPATIBILITY),
            BindingExecutionSupport::host_erased(),
            HostBindingCompilerRegistration::new(TestCompiler),
            Box::new(NoopHostServer {
                compatibility: COMPATIBILITY,
            }),
            Box::new(host_client(
                mode,
                output_kind,
                compatibility,
                accepted,
                side_effects,
            )),
            resources,
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

#[cfg(feature = "std")]
type ConsumerHostCall = HostBindingCallBox<CoreResult<InteractionOutput>>;

#[cfg(feature = "std")]
struct RejectConsumerTransfer;

#[cfg(feature = "std")]
impl CleanupTransferTarget<ConsumerHostCall> for RejectConsumerTransfer {
    fn try_accept(
        &mut self,
        transfer: CleanupTransferEnvelope<ConsumerHostCall>,
    ) -> CleanupTransferAcceptance<ConsumerHostCall> {
        CleanupTransferAcceptance::Rejected(transfer)
    }
}

#[cfg(feature = "std")]
struct AcceptConsumerTransfer {
    accepted: Option<CleanupTransferEnvelope<ConsumerHostCall>>,
}

#[cfg(feature = "std")]
impl CleanupTransferTarget<ConsumerHostCall> for AcceptConsumerTransfer {
    fn try_accept(
        &mut self,
        transfer: CleanupTransferEnvelope<ConsumerHostCall>,
    ) -> CleanupTransferAcceptance<ConsumerHostCall> {
        let request = transfer.request();
        let owner = request.requested_owner();
        let record = CleanupRecord::try_new(
            CleanupHandle::new(owner),
            request.phase().reservation().subject(),
            owner,
            request.phase().operation(),
            0,
            RetryClass::Never,
            0,
            0,
        )
        .unwrap();
        self.accepted = Some(transfer);
        CleanupTransferAcceptance::Accepted(record)
    }
}

#[test]
fn static_external_authoring_covers_sync_pending_rejection_budget_and_reuse() {
    let artifact = static_artifact(61);

    let mut synchronous = StaticClient::new(StaticMode::Synchronous);
    let mut slot = clinkz_wot_core::ClientRequestSlot::new();
    assert!(matches!(
        synchronous.start_request(request(61), &artifact, &mut slot, &mut binding_budget(1)),
        Ok(StartStatus::Ready(Ok(_)))
    ));
    assert!(slot.is_vacant());

    let mut rejected = StaticClient::new(StaticMode::Reject);
    let rejected_request = rejected
        .start_request(request(61), &artifact, &mut slot, &mut binding_budget(1))
        .expect_err("pre-acceptance rejection returns the exact request")
        .into_input();
    assert_eq!(rejected_request, request(61));
    assert!(slot.is_vacant());

    let mut pending = StaticClient::new(StaticMode::Pending);
    let zero_budget = pending
        .start_request(request(61), &artifact, &mut slot, &mut binding_budget(0))
        .expect_err("zero budget preserves ownership without slot mutation");
    assert_eq!(zero_budget.input(), &request(61));
    assert!(slot.is_vacant());
    assert_eq!(pending.calls, 0);

    let wrong_artifact = static_artifact(60);
    let mismatch = pending
        .start_request(
            request(61),
            &wrong_artifact,
            &mut slot,
            &mut binding_budget(1),
        )
        .expect_err("the exact artifact identity is checked before acceptance");
    assert_eq!(mismatch.input(), &request(61));
    assert!(slot.is_vacant());
    assert_eq!(pending.calls, 0);

    assert_eq!(
        pending
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
        pending.poll_request(&mut cx, &mut slot, &mut binding_budget(0)),
        Poll::Pending
    ));
    assert!(matches!(
        pending.poll_request(&mut cx, &mut slot, &mut binding_budget(1)),
        Poll::Ready(Ok(_))
    ));
    pending.acknowledge_request(&mut slot).unwrap();
    assert!(slot.state_mut().acknowledged);
    slot.clear();

    let next_artifact = static_artifact(62);
    assert_eq!(
        pending
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
    let mut late = StaticClient::new(StaticMode::LateOnCancel);
    let mut slot = clinkz_wot_core::ClientRequestSlot::new();
    late.start_request(request(63), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap_or_else(|_| panic!("valid host dual-role registration"));
    let mut cx = context();
    let settlement = late
        .start_cancel_request(&mut cx, cleanup_phase(), &mut slot, &mut binding_budget(1))
        .unwrap_or_else(|_| panic!("valid host late-result registration"));
    assert!(matches!(
        settlement,
        StartStatus::Ready(BindingCallSettlement::Returned(Ok(_)))
    ));
    late.acknowledge_request(&mut slot).unwrap();
    slot.clear();

    let mut cancelled = StaticClient::new(StaticMode::Pending);
    cancelled
        .start_request(request(63), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap_or_else(|_| panic!("valid host cancellation registration"));
    assert_eq!(
        cancelled
            .start_cancel_request(&mut cx, cleanup_phase(), &mut slot, &mut binding_budget(1))
            .unwrap(),
        StartStatus::Pending
    );
    assert!(matches!(
        cancelled.poll_cancel_request(&mut cx, &mut slot, &mut binding_budget(1)),
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            disposition: BindingCancellationDisposition::Complete { .. },
            ..
        }))
    ));
}

#[test]
fn installed_static_path_seals_sync_pending_failure_and_late_results() {
    let artifact = static_artifact(64);

    let mut rejected = static_registration(StaticClient::new(StaticMode::Pending));
    let mut slot = StaticConsumerPropertyReadSlot::new();
    let exact = rejected
        .start_consumer_property_read(
            request(64),
            &static_artifact(63),
            &mut slot,
            &mut binding_budget(1),
        )
        .expect_err("installed mismatch returns the exact request before acceptance");
    assert_eq!(exact.into_input(), request(64));
    assert!(slot.is_vacant());

    let mut synchronous = static_registration(StaticClient::new(StaticMode::Synchronous));
    assert!(matches!(
        synchronous.start_consumer_property_read(
            request(64),
            &artifact,
            &mut slot,
            &mut binding_budget(1),
        ),
        Ok(StartStatus::Ready(Ok(_)))
    ));
    assert!(!slot.is_vacant());
    synchronous
        .acknowledge_consumer_property_read(&mut slot)
        .unwrap();
    assert!(slot.is_vacant());

    let mut pending = static_registration(StaticClient::new(StaticMode::Pending));
    assert_eq!(
        pending
            .start_consumer_property_read(
                request(64),
                &artifact,
                &mut slot,
                &mut binding_budget(1),
            )
            .unwrap(),
        StartStatus::Pending
    );
    assert!(
        pending
            .acknowledge_consumer_property_read(&mut slot)
            .is_err(),
        "an unsealed pending result cannot be discarded"
    );
    let mut cx = context();
    assert!(matches!(
        pending.poll_consumer_property_read(&mut cx, &mut slot, &mut binding_budget(0)),
        Poll::Pending
    ));
    assert!(matches!(
        pending.poll_consumer_property_read(&mut cx, &mut slot, &mut binding_budget(1)),
        Poll::Ready(Ok(_))
    ));
    pending
        .acknowledge_consumer_property_read(&mut slot)
        .unwrap();
    assert!(slot.is_vacant());

    let mut synchronous_failure =
        static_registration(StaticClient::new(StaticMode::SynchronousFailure));
    assert!(matches!(
        synchronous_failure.start_consumer_property_read(
            request(64),
            &artifact,
            &mut slot,
            &mut binding_budget(1),
        ),
        Ok(StartStatus::Ready(Err(CoreError::Binding(_))))
    ));
    synchronous_failure
        .acknowledge_consumer_property_read(&mut slot)
        .unwrap();

    let mut pending_failure = static_registration(StaticClient::new(StaticMode::PendingFailure));
    assert_eq!(
        pending_failure
            .start_consumer_property_read(
                request(64),
                &artifact,
                &mut slot,
                &mut binding_budget(1),
            )
            .unwrap(),
        StartStatus::Pending
    );
    assert!(matches!(
        pending_failure.poll_consumer_property_read(&mut cx, &mut slot, &mut binding_budget(1),),
        Poll::Ready(Err(CoreError::Binding(_)))
    ));
    pending_failure
        .acknowledge_consumer_property_read(&mut slot)
        .unwrap();

    let mut late = static_registration(StaticClient::new(StaticMode::LateOnCancel));
    assert_eq!(
        late.start_consumer_property_read(
            request(64),
            &artifact,
            &mut slot,
            &mut binding_budget(1),
        )
        .unwrap(),
        StartStatus::Pending
    );
    assert!(matches!(
        late.start_cancel_consumer_property_read(
            &mut cx,
            cleanup_phase(),
            &mut slot,
            &mut binding_budget(1),
        ),
        Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(_))))
    ));
    late.acknowledge_consumer_property_read(&mut slot).unwrap();
    assert!(slot.is_vacant());
}

#[test]
fn installed_static_path_rejects_every_untrusted_output_after_transfer() {
    let invalid = [
        OutputKind::BindingId,
        OutputKind::BindingGeneration,
        OutputKind::PlanId,
        OutputKind::ResponseSelection,
        OutputKind::MissingMetadata,
        OutputKind::MissingPayload,
        OutputKind::Status,
        OutputKind::PayloadRole,
        OutputKind::ActionReference,
    ];
    let artifact = static_artifact(65);
    for output_kind in invalid {
        let mut registration = static_registration(
            StaticClient::new(StaticMode::Synchronous).with_output(output_kind),
        );
        let mut slot = StaticConsumerPropertyReadSlot::new();
        assert!(matches!(
            registration.start_consumer_property_read(
                request(65),
                &artifact,
                &mut slot,
                &mut binding_budget(1),
            ),
            Ok(StartStatus::Ready(Err(CoreError::Validation(_))))
        ));
        assert!(!slot.is_vacant());
        registration
            .acknowledge_consumer_property_read(&mut slot)
            .unwrap();
        assert!(slot.is_vacant());
    }

    let mut pending =
        static_registration(StaticClient::new(StaticMode::Pending).with_output(OutputKind::PlanId));
    let mut slot = StaticConsumerPropertyReadSlot::new();
    pending
        .start_consumer_property_read(request(65), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap();
    assert!(matches!(
        pending.poll_consumer_property_read(&mut context(), &mut slot, &mut binding_budget(1),),
        Poll::Ready(Err(CoreError::Validation(_)))
    ));
    pending
        .acknowledge_consumer_property_read(&mut slot)
        .unwrap();

    let mut late = static_registration(
        StaticClient::new(StaticMode::LateOnCancel).with_output(OutputKind::PayloadRole),
    );
    late.start_consumer_property_read(request(65), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap();
    assert!(matches!(
        late.start_cancel_consumer_property_read(
            &mut context(),
            cleanup_phase(),
            &mut slot,
            &mut binding_budget(1),
        ),
        Ok(StartStatus::Ready(BindingCallSettlement::Returned(Err(
            CoreError::Validation(_)
        ))))
    ));
    late.acknowledge_consumer_property_read(&mut slot).unwrap();
}

#[test]
fn installed_static_cancellation_has_no_transfer_authority_or_clear_bypass() {
    let phase = cleanup_phase();
    assert_eq!(phase.transfer_owner(), None);
    let unchanged = phase
        .try_into_transfer_request()
        .expect_err("the static cleanup phase has no transfer owner");
    assert_eq!(unchanged.transfer_owner(), None);

    let artifact = static_artifact(66);
    let mut registration = static_registration(StaticClient::new(StaticMode::FabricatedTransfer));
    let mut slot = StaticConsumerPropertyReadSlot::new();
    registration
        .start_consumer_property_read(request(66), &artifact, &mut slot, &mut binding_budget(1))
        .unwrap();
    assert!(
        registration
            .start_cancel_consumer_property_read(
                &mut context(),
                cleanup_phase_with_transfer_owner(),
                &mut slot,
                &mut binding_budget(1),
            )
            .is_err(),
        "caller-provided static transfer authority is rejected before binding progress"
    );
    assert!(
        registration
            .acknowledge_consumer_property_read(&mut slot)
            .is_err()
    );
    assert!(
        registration
            .start_cancel_consumer_property_read(
                &mut context(),
                cleanup_phase(),
                &mut slot,
                &mut binding_budget(1),
            )
            .is_err(),
        "a binding-fabricated transfer request is not projected"
    );
    assert!(!slot.is_vacant());
    assert!(matches!(
        registration.poll_cancel_consumer_property_read(
            &mut context(),
            &mut slot,
            &mut binding_budget(1),
        ),
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            disposition: BindingCancellationDisposition::Complete { .. },
            ..
        }))
    ));
    registration
        .acknowledge_consumer_property_read(&mut slot)
        .unwrap();
    assert!(slot.is_vacant());
}

#[cfg(feature = "std")]
#[test]
fn host_external_authoring_preserves_constructor_and_call_ownership() {
    let accepted = Arc::new(AtomicUsize::new(0));
    let side_effects = Arc::new(AtomicUsize::new(0));
    let client = host_client(
        HostMode::Result,
        OutputKind::Valid,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    );
    let artifact = host_artifact(67);
    let mut call = client.invoke(request(67), &artifact).unwrap();
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
    let rejected = client
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
    let late = host_client(
        HostMode::LateOnCancel,
        OutputKind::Valid,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    );
    let artifact = host_artifact(71);
    let mut call = late.invoke(request(71), &artifact).unwrap();
    let mut cx = context();
    assert!(matches!(
        call.as_pin_mut()
            .start_cancel(&mut cx, cleanup_phase(), &mut binding_budget(1))
            .unwrap(),
        StartStatus::Ready(BindingCallSettlement::Returned(Ok(_)))
    ));

    let cancelled = host_client(
        HostMode::Cancel,
        OutputKind::Valid,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    );
    let mut call = cancelled.invoke(request(71), &artifact).unwrap();
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

#[cfg(feature = "std")]
#[test]
fn installed_host_path_seals_normal_failure_and_late_results() {
    let accepted = Arc::new(AtomicUsize::new(0));
    let side_effects = Arc::new(AtomicUsize::new(0));
    let artifact = host_artifact(72);
    let registration = host_registration(
        HostMode::Result,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let mut call = registration
        .start_consumer_property_read(request(72), &artifact)
        .unwrap();
    let footprint = call.as_pin_mut().lifetime_footprint();
    assert_eq!(footprint.retained_items(), 2);
    assert!(footprint.retained_bytes() > 64);
    assert!(footprint.fits_within(registration.resources().admitted()));
    assert_eq!(side_effects.load(Ordering::SeqCst), 0);
    assert!(matches!(
        call.as_pin_mut()
            .poll_result(&mut context(), &mut binding_budget(1)),
        Poll::Ready(Ok(_))
    ));
    assert_eq!(side_effects.load(Ordering::SeqCst), 1);

    let failure = host_registration(
        HostMode::Failure,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let mut call = failure
        .start_consumer_property_read(request(72), &artifact)
        .unwrap();
    assert!(matches!(
        call.as_pin_mut()
            .poll_result(&mut context(), &mut binding_budget(1)),
        Poll::Ready(Err(CoreError::Binding(_)))
    ));

    let valid_late = host_registration(
        HostMode::LateOnCancel,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let mut call = valid_late
        .start_consumer_property_read(request(72), &artifact)
        .unwrap();
    assert!(matches!(
        call.as_pin_mut()
            .start_cancel(&mut context(), cleanup_phase(), &mut binding_budget(1)),
        Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(_))))
    ));

    let late = host_registration_with(
        HostMode::LateOnCancel,
        OutputKind::Status,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
        resources(),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let mut call = late
        .start_consumer_property_read(request(72), &artifact)
        .unwrap();
    assert!(matches!(
        call.as_pin_mut()
            .start_cancel(&mut context(), cleanup_phase(), &mut binding_budget(1),),
        Ok(StartStatus::Ready(BindingCallSettlement::Returned(Err(
            CoreError::Validation(_)
        ))))
    ));
}

#[cfg(feature = "std")]
#[test]
fn installed_host_path_rejects_every_untrusted_output_after_transfer() {
    let invalid = [
        OutputKind::BindingId,
        OutputKind::BindingGeneration,
        OutputKind::PlanId,
        OutputKind::ResponseSelection,
        OutputKind::MissingMetadata,
        OutputKind::MissingPayload,
        OutputKind::Status,
        OutputKind::PayloadRole,
        OutputKind::ActionReference,
    ];
    let artifact = host_artifact(74);
    for output_kind in invalid {
        let registration = host_registration_with(
            HostMode::Result,
            output_kind,
            COMPATIBILITY,
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            resources(),
        )
        .unwrap_or_else(|_| panic!("valid installed Host registration"));
        let mut call = registration
            .start_consumer_property_read(request(74), &artifact)
            .unwrap();
        assert!(matches!(
            call.as_pin_mut()
                .poll_result(&mut context(), &mut binding_budget(1)),
            Poll::Ready(Err(CoreError::Validation(_)))
        ));
    }
}

#[cfg(feature = "std")]
#[test]
fn installed_host_rejects_before_acceptance_and_reports_accepted_resource_overrun() {
    let accepted = Arc::new(AtomicUsize::new(0));
    let side_effects = Arc::new(AtomicUsize::new(0));
    let registration = host_registration(
        HostMode::Result,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let wrong_artifact = host_artifact(75);
    let rejected = registration
        .start_consumer_property_read(request(76), &wrong_artifact)
        .expect_err("installed identity mismatch is pre-acceptance rejection");
    assert_eq!(rejected.into_input(), request(76));
    assert_eq!(accepted.load(Ordering::SeqCst), 0);
    assert_eq!(side_effects.load(Ordering::SeqCst), 0);

    let too_small = resources_with_admitted(
        BindingLifetimeFootprint::new(1, 64),
        BindingTransientFootprint::new(0),
    );
    let registration = host_registration_with(
        HostMode::Result,
        OutputKind::Valid,
        COMPATIBILITY,
        Arc::clone(&accepted),
        Arc::clone(&side_effects),
        too_small,
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let artifact = host_artifact(76);
    let mut accepted_error = registration
        .start_consumer_property_read(request(76), &artifact)
        .expect("resource overrun occurs only after the raw call accepted the request");
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    assert_eq!(side_effects.load(Ordering::SeqCst), 0);
    assert!(matches!(
        accepted_error
            .as_pin_mut()
            .poll_result(&mut context(), &mut binding_budget(1)),
        Poll::Ready(Err(CoreError::Validation(_)))
    ));
    assert_eq!(side_effects.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "std")]
#[test]
fn host_cleanup_transfer_moves_or_returns_the_complete_sealed_call() {
    let artifact = host_artifact(77);
    let registration = host_registration(
        HostMode::Transfer,
        COMPATIBILITY,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let mut call = registration
        .start_consumer_property_read(request(77), &artifact)
        .unwrap();
    let footprint = call.as_pin_mut().lifetime_footprint();
    let StartStatus::Ready(BindingCallSettlement::Cancelled {
        disposition: BindingCancellationDisposition::TransferRequired(transfer),
        ..
    }) = call
        .as_pin_mut()
        .start_cancel(
            &mut context(),
            cleanup_phase_with_transfer_owner(),
            &mut binding_budget(1),
        )
        .unwrap()
    else {
        panic!("Host transfer mode returns one provisional transfer request")
    };
    let mut reject = RejectConsumerTransfer;
    let CleanupTransferAcceptance::Rejected(envelope) =
        reject.try_accept(CleanupTransferEnvelope::new(transfer, call))
    else {
        panic!("rejecting target returns the complete envelope")
    };
    let (_, mut returned_call) = envelope.into_parts();
    assert_eq!(returned_call.as_pin_mut().lifetime_footprint(), footprint);
    assert!(matches!(
        returned_call
            .as_pin_mut()
            .poll_cancel(&mut context(), &mut binding_budget(1)),
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            disposition: BindingCancellationDisposition::Complete { .. },
            ..
        }))
    ));

    let registration = host_registration(
        HostMode::Transfer,
        COMPATIBILITY,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    )
    .unwrap_or_else(|_| panic!("valid installed Host registration"));
    let mut call = registration
        .start_consumer_property_read(request(77), &artifact)
        .unwrap();
    let StartStatus::Ready(BindingCallSettlement::Cancelled {
        disposition: BindingCancellationDisposition::TransferRequired(transfer),
        ..
    }) = call
        .as_pin_mut()
        .start_cancel(
            &mut context(),
            cleanup_phase_with_transfer_owner(),
            &mut binding_budget(1),
        )
        .unwrap()
    else {
        panic!("Host transfer mode returns one provisional transfer request")
    };
    let mut accept = AcceptConsumerTransfer { accepted: None };
    assert!(matches!(
        accept.try_accept(CleanupTransferEnvelope::new(transfer, call)),
        CleanupTransferAcceptance::Accepted(_)
    ));
    let mut accepted_call = accept
        .accepted
        .take()
        .expect("accepted owner retains the complete decorated call")
        .into_parts()
        .1;
    assert_eq!(accepted_call.as_pin_mut().lifetime_footprint(), footprint);
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
    oversized.lifetime = BindingLifetimeFootprint::new(9, 2_049);
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
fn static_registration_accounts_complete_core_slot_and_transient_validation_work() {
    let complete_bytes = 64
        + u64::try_from(core::mem::size_of::<
            StaticConsumerPropertyReadSlot<StaticState>,
        >())
        .unwrap();
    let exact = resources_with_admitted(
        BindingLifetimeFootprint::new(2, complete_bytes),
        BindingTransientFootprint::new(7),
    );
    assert!(
        static_registration_with(
            StaticClient::new(StaticMode::Pending)
                .with_transient(BindingTransientFootprint::new(7)),
            exact,
        )
        .is_ok()
    );

    let one_byte_short = resources_with_admitted(
        BindingLifetimeFootprint::new(2, complete_bytes - 1),
        BindingTransientFootprint::new(7),
    );
    assert!(
        static_registration_with(StaticClient::new(StaticMode::Pending), one_byte_short).is_err()
    );

    let transient_short = resources_with_admitted(
        BindingLifetimeFootprint::new(2, complete_bytes),
        BindingTransientFootprint::new(6),
    );
    assert!(
        static_registration_with(
            StaticClient::new(StaticMode::Pending)
                .with_transient(BindingTransientFootprint::new(7)),
            transient_short,
        )
        .is_err()
    );
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
        assert!(
            !host_registration
                .capabilities()
                .supports_consumer_property_read()
        );
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
        .start_consumer_property_read(request(73), &artifact)
        .unwrap();
    assert_eq!(LEGACY_CALLS.load(Ordering::SeqCst), 0);
}
