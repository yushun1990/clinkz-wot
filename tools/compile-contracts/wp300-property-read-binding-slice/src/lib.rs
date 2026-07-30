#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::task::{Context, Poll};

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint, BindingCallSettlement,
    BindingCancellationDisposition, BindingCompilerBounds, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerOutput, BindingCompilerStep, BindingDeliveryOutcome,
    BindingExecutionSupport, BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint,
    BindingRegistrationCapabilities, BindingRegistrationIdentity, BindingResourceDeclarations,
    BindingStateLayout, BindingStatusPolicy, CleanupPhaseContext, NoCleanupSuccessor,
    PollServerBinding, PrepareInput, RouteActivationOutcome, RouteActivationPermit,
    RouteCleanupOutcome, RouteCommitOutcome, RouteInboundResponse, RoutePrepareOutcome,
    RouteReadinessOutcome, RouteReadinessSlot, ServerResponseSlot, ServerRouteSlot,
    StaticBindingCompilerRegistration, StaticBindingRegistration, StaticBindingRegistrationInput,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

/// Pure compiler cursor authored outside the engine workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockCompilerCursor(bool);

/// Immutable mock protocol route data.
#[derive(Debug, Eq, PartialEq)]
pub struct MockArtifact {
    target: Box<str>,
}

/// Compiler paired with the mock server implementation.
#[derive(Clone, Copy)]
pub struct MockCompiler {
    compatibility: BindingArtifactCompatibility,
}

impl MockCompiler {
    /// Creates the compiler for one stable execution compatibility identity.
    pub const fn new(compatibility: BindingArtifactCompatibility) -> Self {
        Self { compatibility }
    }
}

impl BindingCompilerExtension for MockCompiler {
    type Cursor = MockCompilerCursor;
    type Artifact = MockArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, input.logical_plan().resolved_target().len() as u64),
            core::mem::size_of::<MockCompilerCursor>() as u64,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }

    fn start(
        &self,
        _input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<Self::Cursor> {
        Ok(MockCompilerCursor(false))
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
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

/// Protocol state retained in one caller-owned route slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockRouteState {
    phase: u8,
}

/// Externally visible readiness state; zero means ready.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockReadinessState {
    remaining_polls: u8,
}

/// One response delivery cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockResponseState {
    accepted: bool,
}

/// Third-party manual-progress server with no TD, handler, or Servient access.
pub struct ManualMockBinding {
    compatibility: BindingArtifactCompatibility,
    external_readiness_polls: u8,
}

impl ManualMockBinding {
    /// Creates either an immediate or externally-ready mock binding.
    pub const fn new(
        compatibility: BindingArtifactCompatibility,
        external_readiness_polls: u8,
    ) -> Self {
        Self {
            compatibility,
            external_readiness_polls,
        }
    }
}

impl PollServerBinding for ManualMockBinding {
    type Compiler = MockCompiler;
    type RouteState = MockRouteState;
    type ReadinessState = MockReadinessState;
    type ResponseState = MockResponseState;

    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn route_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<MockRouteState>(BindingLifetimeFootprint::new(1, 1))
    }

    fn readiness_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<MockReadinessState>(BindingLifetimeFootprint::new(1, 1))
    }

    fn response_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<MockResponseState>(BindingLifetimeFootprint::new(1, 1))
    }

    fn start_prepare(
        &mut self,
        input: PrepareInput,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Result<
        clinkz_wot_core::StartStatus<RoutePrepareOutcome<()>>,
        BindingInputRejection<PrepareInput>,
    > {
        route.initialize(input, MockRouteState { phase: 1 });
        Ok(clinkz_wot_core::StartStatus::Ready(
            RoutePrepareOutcome::Prepared(()),
        ))
    }

    fn poll_prepare(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<()>> {
        Poll::Ready(RoutePrepareOutcome::Prepared(()))
    }

    fn poll_cancel_prepare(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<RoutePrepareOutcome<()>, ()>>> {
        Poll::Ready(Ok(cancelled()))
    }

    fn start_readiness(
        &mut self,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteReadinessOutcome<()>> {
        readiness.initialize_state(MockReadinessState {
            remaining_polls: self.external_readiness_polls,
        });
        if self.external_readiness_polls == 0 {
            clinkz_wot_core::StartStatus::Ready(RouteReadinessOutcome::Ready(()))
        } else {
            clinkz_wot_core::StartStatus::Pending
        }
    }

    fn poll_readiness(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteReadinessOutcome<()>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        let state = readiness.state_mut();
        state.remaining_polls -= 1;
        if state.remaining_polls == 0 {
            Poll::Ready(RouteReadinessOutcome::Ready(()))
        } else {
            Poll::Pending
        }
    }

    fn poll_cancel_readiness(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<RouteReadinessOutcome<()>, ()>>>
    {
        Poll::Ready(Ok(cancelled()))
    }

    fn start_activate(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteActivationOutcome<(), ()>> {
        route.state_mut().phase = 2;
        clinkz_wot_core::StartStatus::Ready(RouteActivationOutcome::Active(()))
    }

    fn poll_activate(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RouteActivationOutcome<(), ()>> {
        Poll::Ready(RouteActivationOutcome::Active(()))
    }

    fn poll_cancel_activate(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<RouteActivationOutcome<(), ()>, ()>>>
    {
        Poll::Ready(Ok(cancelled()))
    }

    fn start_commit(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCommitOutcome<(), ()>> {
        route.state_mut().phase = 3;
        clinkz_wot_core::StartStatus::Ready(RouteCommitOutcome::Committed(()))
    }

    fn poll_commit(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RouteCommitOutcome<(), ()>> {
        Poll::Ready(RouteCommitOutcome::Committed(()))
    }

    fn poll_cancel_commit(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<RouteCommitOutcome<(), ()>, ()>>>
    {
        Poll::Ready(Ok(cancelled()))
    }

    fn poll_accept(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _permit: RouteActivationPermit<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        Poll::Pending
    }

    fn start_abort(
        &mut self,
        _cleanup: CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCleanupOutcome> {
        clinkz_wot_core::StartStatus::Ready(RouteCleanupOutcome::Complete)
    }

    fn poll_abort(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        Poll::Ready(RouteCleanupOutcome::Complete)
    }

    fn start_shutdown(
        &mut self,
        _cleanup: CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCleanupOutcome> {
        clinkz_wot_core::StartStatus::Ready(RouteCleanupOutcome::Complete)
    }

    fn poll_shutdown(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        Poll::Ready(RouteCleanupOutcome::Complete)
    }

    fn acknowledge_route(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
    ) -> clinkz_wot_core::CoreResult<()> {
        route.clear();
        Ok(())
    }

    fn start_response(
        &mut self,
        response: RouteInboundResponse,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        _budget: &mut WorkBudget,
    ) -> Result<
        clinkz_wot_core::StartStatus<BindingDeliveryOutcome>,
        BindingInputRejection<RouteInboundResponse>,
    > {
        slot.initialize(response, MockResponseState { accepted: true });
        Ok(clinkz_wot_core::StartStatus::Ready(
            BindingDeliveryOutcome::Delivered,
        ))
    }

    fn poll_response(
        &mut self,
        _cx: &mut Context<'_>,
        _slot: &mut ServerResponseSlot<Self::ResponseState>,
        _budget: &mut WorkBudget,
    ) -> Poll<BindingDeliveryOutcome> {
        Poll::Ready(BindingDeliveryOutcome::Delivered)
    }

    fn poll_cancel_response(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _slot: &mut ServerResponseSlot<Self::ResponseState>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>> {
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            retry_class: clinkz_wot_core::RetryClass::Never,
            disposition: BindingCancellationDisposition::<NoCleanupSuccessor>::Complete {
                successor: NoCleanupSuccessor,
            },
        }))
    }

    fn acknowledge_response(
        &mut self,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
    ) -> clinkz_wot_core::CoreResult<()> {
        slot.clear();
        Ok(())
    }
}

/// Constructs the constrained complete registration from public dependencies.
pub fn static_registration(
    identity: BindingRegistrationIdentity,
    compatibility: BindingArtifactCompatibility,
    capabilities: BindingRegistrationCapabilities,
    execution: BindingExecutionSupport,
    resources: BindingResourceDeclarations,
    ingress: BindingIngressPolicy,
    status: BindingStatusPolicy,
    external_readiness_polls: u8,
) -> Result<
    StaticBindingRegistration<ManualMockBinding>,
    BindingInputRejection<StaticBindingRegistrationInput<ManualMockBinding>>,
> {
    StaticBindingRegistration::new(StaticBindingRegistrationInput::new(
        identity,
        capabilities,
        execution,
        StaticBindingCompilerRegistration::new(MockCompiler::new(compatibility)),
        ManualMockBinding::new(compatibility, external_readiness_polls),
        resources,
        ingress,
        status,
    ))
}

fn cancelled<T>() -> BindingCallSettlement<T, ()> {
    BindingCallSettlement::Cancelled {
        retry_class: clinkz_wot_core::RetryClass::Never,
        disposition: BindingCancellationDisposition::Complete { successor: () },
    }
}
