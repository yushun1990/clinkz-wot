#![no_std]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, sync::Arc};
use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
    task::{Context, Poll},
};

use clinkz_wot_core::{
    AffordanceTarget, BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactRole, BindingCallSettlement,
    BindingCancellationDisposition, BindingCompilerBounds, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerOutput, BindingCompilerStep, BindingConfigurationDigest,
    BindingDeliveryOutcome, BindingExecutionSupport, BindingGeneration, BindingId,
    BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint, BindingOperationalError,
    BindingRegistrationCapabilities, BindingRegistrationIdentity, BindingResourceDeclarations,
    BindingStateLayout, BindingStatusPolicy, CleanupPhaseContext, CollisionDomainId, CoreError,
    CorrelationId, EndpointReservationKey, ErrorContext, ErrorPhase, InteractionInput,
    NoCleanupSuccessor, PollServerBinding, PrepareInput, RetryClass, RouteActivationOutcome,
    RouteActivationPermit, RouteCleanupOutcome, RouteCommitOutcome, RouteInboundRequest,
    RouteInboundResponse, RoutePrepareOutcome, RouteReadinessOutcome, RouteReadinessSlot,
    RouteReservationIdentity, RouteTerminal, ServerResponseSlot, ServerRouteSlot,
    StaticBindingCompilerRegistration, StaticBindingRegistration, StaticBindingRegistrationInput,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

/// Pure compiler cursor authored outside the engine workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockCompilerCursor(bool);

/// Immutable mock protocol route data.
#[derive(Debug)]
pub struct MockArtifact {
    variant: MockArtifactVariant,
    drop_probe: Option<Arc<AtomicU32>>,
}

#[derive(Debug)]
enum MockArtifactVariant {
    Target(Box<str>),
    UnsupportedVariant,
}

impl MockArtifact {
    /// Borrows the compiler-produced protocol target when this server owns the variant.
    pub fn target(&self) -> Option<&str> {
        match &self.variant {
            MockArtifactVariant::Target(target) => Some(target),
            MockArtifactVariant::UnsupportedVariant => None,
        }
    }

    /// Creates the deliberately unsupported application-static test variant.
    pub fn unsupported_variant() -> Self {
        Self {
            variant: MockArtifactVariant::UnsupportedVariant,
            drop_probe: None,
        }
    }
}

impl Drop for MockArtifact {
    fn drop(&mut self) {
        if let Some(counter) = &self.drop_probe {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Compiler paired with the mock server implementation.
#[derive(Clone)]
pub struct MockCompiler {
    compatibility: BindingArtifactCompatibility,
    artifact_drops: Option<Arc<AtomicU32>>,
}

impl MockCompiler {
    /// Creates the compiler for one stable execution compatibility identity.
    pub const fn new(compatibility: BindingArtifactCompatibility) -> Self {
        Self {
            compatibility,
            artifact_drops: None,
        }
    }

    fn with_artifact_drop_probe(
        compatibility: BindingArtifactCompatibility,
        artifact_drops: Arc<AtomicU32>,
    ) -> Self {
        Self {
            compatibility,
            artifact_drops: Some(artifact_drops),
        }
    }

    fn route_reservation(&self, input: &BindingCompilerInput<'_>) -> RouteReservationIdentity {
        let mut endpoint = *input.candidate().configuration().as_bytes();
        for (index, byte) in input
            .logical_plan()
            .resolved_target()
            .as_bytes()
            .iter()
            .enumerate()
        {
            let slot = index % endpoint.len();
            endpoint[slot] = endpoint[slot]
                .wrapping_add(*byte)
                .rotate_left((index % 8) as u32);
        }
        RouteReservationIdentity::new(
            CollisionDomainId::new(*self.compatibility.as_bytes()),
            EndpointReservationKey::new(endpoint),
        )
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
        let payload = MockArtifact {
            variant: MockArtifactVariant::Target(target),
            drop_probe: self.artifact_drops.as_ref().map(Arc::clone),
        };
        let artifact = if input.role() == BindingArtifactRole::ProducerRoute {
            BindingArtifact::producer_route(
                self.compatibility,
                footprint,
                self.route_reservation(input),
                payload,
            )
        } else {
            BindingArtifact::new(self.compatibility, footprint, payload)
        };
        BindingCompilerStep::Complete(BindingCompilerOutput::new(artifact))
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

/// Protocol state retained in one caller-owned route slot.
#[derive(Debug, Eq, PartialEq)]
pub struct MockRouteState {
    phase: u8,
    target: Box<str>,
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

struct StaticProbeState {
    queued: Option<(Box<str>, InteractionInput)>,
    next_correlation: u64,
    delivered: u32,
    routes: u32,
    in_flight: u32,
    cleanup: u32,
    aborts: u32,
    closed: bool,
    prepared_target: Option<Box<str>>,
}

impl Default for StaticProbeState {
    fn default() -> Self {
        Self {
            queued: None,
            next_correlation: 1,
            delivered: 0,
            routes: 0,
            in_flight: 0,
            cleanup: 0,
            aborts: 0,
            closed: false,
            prepared_target: None,
        }
    }
}

/// Deterministic no-default protocol I/O visible to the WP-400 runner.
#[derive(Clone)]
pub struct StaticPropertyReadProbe {
    state: Rc<RefCell<StaticProbeState>>,
    artifact_drops: Arc<AtomicU32>,
}

impl StaticPropertyReadProbe {
    pub fn enqueue_property_read(&self, name: &str, input: InteractionInput) {
        let mut state = self.state.borrow_mut();
        assert!(!state.closed, "request queued after route closure");
        assert!(state.queued.is_none(), "mock ingress slot is occupied");
        state.queued = Some((Box::from(name), input));
    }

    pub fn delivered_responses(&self) -> u32 {
        self.state.borrow().delivered
    }

    pub fn outstanding_counts(&self) -> (u32, u32, u32, u32) {
        let state = self.state.borrow();
        (
            state.routes,
            u32::from(state.queued.is_some()),
            state.in_flight,
            state.cleanup,
        )
    }

    pub fn aborted_routes(&self) -> u32 {
        self.state.borrow().aborts
    }

    pub fn prepared_target(&self) -> Option<Box<str>> {
        self.state.borrow().prepared_target.clone()
    }

    pub fn artifact_drops(&self) -> u32 {
        self.artifact_drops.load(Ordering::SeqCst)
    }

    pub fn poll_after_close(&self, _cx: &mut Context<'_>) -> Poll<bool> {
        if self.state.borrow().closed {
            Poll::Ready(false)
        } else {
            Poll::Pending
        }
    }
}

/// Third-party manual-progress server with no TD, handler, or Servient access.
pub struct ManualMockBinding {
    compatibility: BindingArtifactCompatibility,
    external_readiness_polls: u8,
    fail_readiness: bool,
    probe: Option<Rc<RefCell<StaticProbeState>>>,
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
            fail_readiness: false,
            probe: None,
        }
    }

    fn with_probe(
        compatibility: BindingArtifactCompatibility,
        probe: Rc<RefCell<StaticProbeState>>,
        fail_readiness: bool,
    ) -> Self {
        Self {
            compatibility,
            external_readiness_polls: 0,
            fail_readiness,
            probe: Some(probe),
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
        BindingStateLayout::of::<MockRouteState>(BindingLifetimeFootprint::new(2, 128))
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
        artifact: &BindingArtifactEnvelope<MockArtifact>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Result<
        clinkz_wot_core::StartStatus<RoutePrepareOutcome<()>>,
        BindingInputRejection<PrepareInput>,
    > {
        let Some(target) = artifact.artifact().payload().target() else {
            let error = artifact_input_error(*input.route());
            return Err(BindingInputRejection::new(input, error));
        };
        route.initialize(
            input,
            MockRouteState {
                phase: 0,
                target: Box::from(target),
            },
        );
        if self.probe.is_some() {
            Ok(clinkz_wot_core::StartStatus::Pending)
        } else {
            route.state_mut().phase = 1;
            Ok(clinkz_wot_core::StartStatus::Ready(
                RoutePrepareOutcome::Prepared(()),
            ))
        }
    }

    fn poll_prepare(
        &mut self,
        _cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<()>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        let state = route.state_mut();
        state.phase = 1;
        if let Some(probe) = &self.probe {
            let mut probe = probe.borrow_mut();
            assert_eq!(probe.routes, 0);
            probe.prepared_target = Some(state.target.clone());
            probe.routes = 1;
        }
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
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteReadinessOutcome<()>> {
        readiness.initialize_state(MockReadinessState {
            remaining_polls: self.external_readiness_polls,
        });
        if self.fail_readiness {
            self.fail_readiness = false;
            return clinkz_wot_core::StartStatus::Ready(RouteReadinessOutcome::Failed {
                guard: (),
                error: BindingOperationalError::for_route(
                    *route.input().route(),
                    CoreError::Binding(ErrorContext::new(ErrorPhase::Readiness, RetryClass::Never)),
                ),
            });
        }
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
        permit: RouteActivationPermit<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        let Some(probe) = &self.probe else {
            return Poll::Pending;
        };
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        let mut state = probe.borrow_mut();
        if state.closed {
            return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Terminal(
                RouteTerminal::Closed {
                    route: *permit.route(),
                },
            )));
        }
        let Some((name, input)) = state.queued.take() else {
            return Poll::Pending;
        };
        let correlation = CorrelationId::new(state.next_correlation);
        state.next_correlation += 1;
        state.in_flight = 1;
        Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Request(
            RouteInboundRequest::new(
                *permit.route(),
                correlation,
                AffordanceTarget::Property(Arc::from(name)),
                input,
            ),
        )))
    }

    fn start_abort(
        &mut self,
        _cleanup: CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCleanupOutcome> {
        if let Some(probe) = &self.probe {
            let mut state = probe.borrow_mut();
            state.cleanup = 1;
            state.aborts += 1;
            state.routes = 0;
            state.cleanup = 0;
            state.closed = true;
        }
        clinkz_wot_core::StartStatus::Ready(RouteCleanupOutcome::Complete)
    }

    fn poll_abort(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        if let Some(probe) = &self.probe {
            let mut state = probe.borrow_mut();
            state.routes = 0;
            state.cleanup = 0;
            state.closed = true;
        }
        Poll::Ready(RouteCleanupOutcome::Complete)
    }

    fn start_shutdown(
        &mut self,
        _cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCleanupOutcome> {
        let Some(probe) = &self.probe else {
            return clinkz_wot_core::StartStatus::Ready(RouteCleanupOutcome::Complete);
        };
        probe.borrow_mut().cleanup = 1;
        if budget.consume(WorkClass::CleanupItems, 1).is_err() {
            route.state_mut().phase = 4;
            return clinkz_wot_core::StartStatus::Pending;
        }
        let mut state = probe.borrow_mut();
        state.routes = 0;
        state.cleanup = 0;
        state.closed = true;
        clinkz_wot_core::StartStatus::Ready(RouteCleanupOutcome::Complete)
    }

    fn poll_shutdown(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        let Some(probe) = &self.probe else {
            return Poll::Ready(RouteCleanupOutcome::Complete);
        };
        if budget.consume(WorkClass::CleanupItems, 1).is_err() {
            return Poll::Pending;
        }
        let mut state = probe.borrow_mut();
        state.routes = 0;
        state.cleanup = 0;
        state.closed = true;
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
        if let Some(probe) = &self.probe {
            assert!(response.result().is_ok(), "fixture handler response failed");
            let mut state = probe.borrow_mut();
            assert_eq!(state.in_flight, 1);
            state.in_flight = 0;
            state.delivered += 1;
        }
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

/// Builds the complete no-default registration and deterministic I/O probe
/// used by the WP-400 runtime contract.
pub fn static_property_read_fixture() -> (
    StaticBindingRegistration<ManualMockBinding>,
    StaticPropertyReadProbe,
) {
    static_property_read_fixture_with_readiness(false)
}

/// Builds a no-default registration whose prepared route fails readiness and
/// therefore must be returned intact to the explicit abort path.
pub fn static_property_read_readiness_failure_fixture() -> (
    StaticBindingRegistration<ManualMockBinding>,
    StaticPropertyReadProbe,
) {
    static_property_read_fixture_with_readiness(true)
}

fn static_property_read_fixture_with_readiness(
    fail_readiness: bool,
) -> (
    StaticBindingRegistration<ManualMockBinding>,
    StaticPropertyReadProbe,
) {
    let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
    let identity = BindingRegistrationIdentity::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new([0x52; 32]),
        compatibility,
        0,
    );
    let state = Rc::new(RefCell::new(StaticProbeState::default()));
    let artifact_drops = Arc::new(AtomicU32::new(0));
    let input = StaticBindingRegistrationInput::new(
        identity,
        BindingRegistrationCapabilities::producer_property_read(),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(MockCompiler::with_artifact_drop_probe(
            compatibility,
            Arc::clone(&artifact_drops),
        )),
        ManualMockBinding::with_probe(compatibility, state.clone(), fail_readiness),
        BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(4, 256),
            BindingLifetimeFootprint::new(4, 256),
        ),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(1, 64),
    );
    let registration = match StaticBindingRegistration::new(input) {
        Ok(registration) => registration,
        Err(_) => panic!("complete static mock registration was rejected"),
    };
    (
        registration,
        StaticPropertyReadProbe {
            state,
            artifact_drops,
        },
    )
}

fn artifact_input_error(
    route: clinkz_wot_core::binding::BindingRouteKey,
) -> BindingOperationalError {
    BindingOperationalError::for_route(
        route,
        CoreError::Binding(ErrorContext::new(ErrorPhase::Prepare, RetryClass::Never)),
    )
}

fn cancelled<T>() -> BindingCallSettlement<T, ()> {
    BindingCallSettlement::Cancelled {
        retry_class: clinkz_wot_core::RetryClass::Never,
        disposition: BindingCancellationDisposition::Complete { successor: () },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_core::binding::BindingRouteKey;
    use clinkz_wot_core::{BindingArtifactIdentity, BindingArtifactRef, PlanId, PlanSetGeneration};
    use clinkz_wot_foundation::{Generation, SlotIndex};

    #[test]
    fn static_variant_mismatch_returns_input_before_route_state_or_side_effect() {
        let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
        let registration_identity = BindingRegistrationIdentity::new(
            BindingId::new(7),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([0x52; 32]),
            compatibility,
            0,
        );
        let resources = BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(4, 256),
            BindingLifetimeFootprint::new(4, 256),
        );
        let mut registration = match static_registration(
            registration_identity,
            compatibility,
            BindingRegistrationCapabilities::producer_property_read(),
            BindingExecutionSupport::application_static(),
            resources,
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(1, 64),
            0,
        ) {
            Ok(registration) => registration,
            Err(_) => panic!("complete static registration was rejected"),
        };
        let plan_id = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
        let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
        let artifact_identity = BindingArtifactIdentity::new(
            plan_set_generation,
            plan_id,
            registration_identity.binding_id(),
            registration_identity.binding_generation(),
            registration_identity.configuration(),
            compatibility,
            BindingArtifactRole::ProducerRoute,
        );
        let reservation = RouteReservationIdentity::new(
            CollisionDomainId::new([0x61; 16]),
            EndpointReservationKey::new([0x62; 32]),
        );
        let artifact_footprint = BindingArtifactFootprint::new(1, 1);
        let envelope = BindingArtifactEnvelope::try_new(
            artifact_identity,
            artifact_footprint,
            BindingArtifact::producer_route(
                compatibility,
                artifact_footprint,
                reservation,
                MockArtifact::unsupported_variant(),
            ),
        )
        .expect("admitted wrong static variant");
        let artifact_ref = BindingArtifactRef::new(artifact_identity, SlotIndex::new(0));
        let route = BindingRouteKey::new(
            registration_identity.binding_id(),
            registration_identity.binding_generation(),
            Generation::INITIAL,
            plan_set_generation,
            plan_id,
            reservation,
        );
        let prepare = PrepareInput::new(route, artifact_ref, resources.route_state());
        let mut route_slot = ServerRouteSlot::new();
        let rejection = registration
            .server_mut()
            .start_prepare(prepare, &envelope, &mut route_slot, &mut WorkBudget::new())
            .expect_err("unsupported static artifact variant must be rejected");

        assert!(route_slot.is_vacant());
        assert_eq!(rejection.input().route(), &route);
        assert_eq!(rejection.input().artifact(), artifact_ref);
        assert_eq!(
            rejection.into_input().admitted_footprint(),
            resources.route_state()
        );
    }
}

#[cfg(feature = "std")]
mod host_fixture {
    use alloc::{boxed::Box, sync::Arc};
    use core::pin::Pin;
    use core::sync::atomic::{AtomicU32, Ordering};
    use core::task::{Context, Poll};

    use clinkz_wot_core::{
        AffordanceTarget, BindingArtifactCompatibility, BindingArtifactEnvelope,
        BindingCallSettlement, BindingCancellationDisposition, BindingConfigurationDigest,
        BindingDeliveryOutcome, BindingExecutionSupport, BindingGeneration, BindingId,
        BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint,
        BindingOperationalError, BindingRegistrationCapabilities, BindingRegistrationIdentity,
        BindingResourceDeclarations, BindingStatusPolicy, CleanupPhaseContext, CoreError,
        CorrelationId, Deadline, ErrorContext, ErrorPhase, HostActiveRouteGuard,
        HostBindingArtifact, HostBindingCall, HostBindingCallBox, HostBindingCompilerRegistration,
        HostBindingRegistration, HostBindingRegistrationInput, HostCommittedRouteGuard,
        HostPreparedRouteGuard, HostRouteCleanupSuccessor, InteractionInput, NoCleanupSuccessor,
        PrepareInput, RetryClass, RouteAbortInput, RouteActivationOutcome, RouteCleanupOutcome,
        RouteCommitOutcome, RouteInboundRequest, RouteInboundResponse, RoutePrepareOutcome,
        RouteReadinessOutcome, RouteServerBinding, RouteShutdownInput, RouteTerminal, StartStatus,
        WotLock,
    };
    use clinkz_wot_foundation::{WorkBudget, WorkClass};

    use super::{MockArtifact, MockCompiler, artifact_input_error};

    struct ProbeState {
        queued: Option<(Box<str>, InteractionInput)>,
        next_correlation: u64,
        delivered: u32,
        routes: u32,
        in_flight: u32,
        cleanup: u32,
        aborts: u32,
        shutdowns: u32,
        reject_readiness_once: bool,
        reject_abort_once: bool,
        reject_shutdown_once: bool,
        readiness_rejections: u32,
        abort_rejections: u32,
        shutdown_rejections: u32,
        closed: bool,
        prepared_target: Option<Box<str>>,
    }

    impl Default for ProbeState {
        fn default() -> Self {
            Self {
                queued: None,
                next_correlation: 1,
                delivered: 0,
                routes: 0,
                in_flight: 0,
                cleanup: 0,
                aborts: 0,
                shutdowns: 0,
                reject_readiness_once: false,
                reject_abort_once: false,
                reject_shutdown_once: false,
                readiness_rejections: 0,
                abort_rejections: 0,
                shutdown_rejections: 0,
                closed: false,
                prepared_target: None,
            }
        }
    }

    /// Deterministic protocol-I/O and instrumentation state for the WP-400
    /// host runner. It creates no plan, route, permit, handler, or response.
    #[derive(Clone)]
    pub struct HostPropertyReadProbe {
        state: WotLock<ProbeState>,
        artifact_drops: Arc<AtomicU32>,
    }

    impl HostPropertyReadProbe {
        pub fn enqueue_property_read(&self, name: &str, input: InteractionInput) {
            self.state.with(|state| {
                assert!(!state.closed, "request queued after route closure");
                assert!(state.queued.is_none(), "mock ingress slot is occupied");
                state.queued = Some((Box::from(name), input));
            });
        }

        pub fn delivered_responses(&self) -> u32 {
            self.state.with_read(|state| state.delivered)
        }

        pub fn outstanding_counts(&self) -> (u32, u32, u32, u32) {
            self.state.with_read(|state| {
                (
                    state.routes,
                    u32::from(state.queued.is_some()),
                    state.in_flight,
                    state.cleanup,
                )
            })
        }

        pub fn poll_after_close(&self, _cx: &mut Context<'_>) -> Poll<bool> {
            self.state.with_read(|state| {
                if state.closed {
                    Poll::Ready(false)
                } else {
                    Poll::Pending
                }
            })
        }

        pub fn cleanup_attempts(&self) -> (u32, u32) {
            self.state
                .with_read(|state| (state.aborts, state.shutdowns))
        }

        pub fn input_rejections(&self) -> (u32, u32, u32) {
            self.state.with_read(|state| {
                (
                    state.readiness_rejections,
                    state.abort_rejections,
                    state.shutdown_rejections,
                )
            })
        }

        pub fn prepared_target(&self) -> Option<Box<str>> {
            self.state.with_read(|state| state.prepared_target.clone())
        }

        pub fn artifact_drops(&self) -> u32 {
            self.artifact_drops.load(Ordering::SeqCst)
        }
    }

    struct PrepareCall {
        input: Option<PrepareInput>,
        target: Option<Box<str>>,
        probe: WotLock<ProbeState>,
        pending_once: bool,
    }

    impl HostBindingCall<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>
        for PrepareCall
    {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            BindingLifetimeFootprint::new(2, 128)
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RoutePrepareOutcome<HostPreparedRouteGuard>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let input = self.input.take().expect("prepare call completed twice");
            let target = self.target.take().expect("prepare target completed twice");
            self.probe.with(|state| {
                assert_eq!(state.routes, 0);
                state.prepared_target = Some(target.clone());
                state.routes = 1;
            });
            let guard =
                HostPreparedRouteGuard::new(input, BindingLifetimeFootprint::new(2, 128), target);
            Poll::Ready(RoutePrepareOutcome::Prepared(guard))
        }

        fn start_cancel(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<
                BindingCallSettlement<
                    RoutePrepareOutcome<HostPreparedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _budget: &mut WorkBudget,
        ) -> Poll<
            clinkz_wot_core::CoreResult<
                BindingCallSettlement<
                    RoutePrepareOutcome<HostPreparedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            let input = self.input.take().expect("prepare call cancelled twice");
            self.target = None;
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: HostRouteCleanupSuccessor::NoRouteResource {
                        route: *input.route(),
                    },
                },
            }))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct ReadyCall<T, C> {
        value: Option<T>,
        pending_polls: u8,
        _cleanup: core::marker::PhantomData<C>,
    }

    impl<T, C> ReadyCall<T, C> {
        fn new(value: T) -> Self {
            Self {
                value: Some(value),
                pending_polls: 0,
                _cleanup: core::marker::PhantomData,
            }
        }

        fn pending_once(value: T) -> Self {
            Self {
                value: Some(value),
                pending_polls: 1,
                _cleanup: core::marker::PhantomData,
            }
        }
    }

    impl<T, C> HostBindingCall<T, C> for ReadyCall<T, C>
    where
        T: Send + Unpin + 'static,
        C: Send + Unpin + 'static,
    {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            BindingLifetimeFootprint::new(1, 128)
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<T> {
            if self.pending_polls != 0 {
                if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                    return Poll::Pending;
                }
                self.pending_polls -= 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready(
                self.value
                    .take()
                    .expect("host call polled after completion"),
            )
        }

        fn start_cancel(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<StartStatus<BindingCallSettlement<T, C>>> {
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _budget: &mut WorkBudget,
        ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<T, C>>> {
            Poll::Ready(Ok(BindingCallSettlement::Returned(
                self.value.take().expect("host call cancelled twice"),
            )))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct DeliveryCall {
        response: Option<RouteInboundResponse>,
        probe: WotLock<ProbeState>,
    }

    impl HostBindingCall<BindingDeliveryOutcome, NoCleanupSuccessor> for DeliveryCall {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            BindingLifetimeFootprint::new(1, 128)
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<BindingDeliveryOutcome> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            let response = self.response.take().expect("response delivered twice");
            assert!(response.result().is_ok(), "fixture handler response failed");
            self.probe.with(|state| {
                assert_eq!(state.in_flight, 1);
                state.in_flight = 0;
                state.delivered += 1;
            });
            Poll::Ready(BindingDeliveryOutcome::Delivered)
        }

        fn start_cancel(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<StartStatus<BindingCallSettlement<BindingDeliveryOutcome>>>
        {
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _budget: &mut WorkBudget,
        ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>>
        {
            Poll::Pending
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    enum CleanupInput {
        Abort(RouteAbortInput),
        Shutdown(RouteShutdownInput),
    }

    struct CleanupCall {
        input: Option<CleanupInput>,
        probe: WotLock<ProbeState>,
    }

    impl HostBindingCall<RouteCleanupOutcome, HostRouteCleanupSuccessor> for CleanupCall {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            BindingLifetimeFootprint::new(1, 128)
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RouteCleanupOutcome> {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            let input = self.input.take().expect("cleanup completed twice");
            self.probe.with(|state| {
                match input {
                    CleanupInput::Abort(input) => {
                        let _ = input.into_parts();
                        state.aborts += 1;
                    }
                    CleanupInput::Shutdown(input) => {
                        let _ = input.into_parts();
                        state.shutdowns += 1;
                    }
                }
                state.routes = 0;
                state.cleanup = 0;
                state.closed = true;
            });
            Poll::Ready(RouteCleanupOutcome::Complete)
        }

        fn start_cancel(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>>,
        > {
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _budget: &mut WorkBudget,
        ) -> Poll<
            clinkz_wot_core::CoreResult<
                BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
            >,
        > {
            Poll::Pending
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct HostMockBinding {
        compatibility: BindingArtifactCompatibility,
        probe: WotLock<ProbeState>,
    }

    impl RouteServerBinding for HostMockBinding {
        fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
        }

        fn prepare(
            &self,
            input: PrepareInput,
            artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
        ) -> Result<
            HostBindingCallBox<
                RoutePrepareOutcome<HostPreparedRouteGuard>,
                HostRouteCleanupSuccessor,
            >,
            BindingInputRejection<PrepareInput>,
        > {
            let Some(payload) = artifact
                .artifact()
                .try_payload::<MockArtifact>(self.compatibility)
            else {
                let error = artifact_input_error(*input.route());
                return Err(BindingInputRejection::new(input, error));
            };
            let Some(target) = payload.target() else {
                let error = artifact_input_error(*input.route());
                return Err(BindingInputRejection::new(input, error));
            };
            Ok(HostBindingCallBox::new(PrepareCall {
                input: Some(input),
                target: Some(Box::from(target)),
                probe: self.probe.clone(),
                pending_once: true,
            }))
        }

        fn start_readiness(
            &self,
            guard: HostPreparedRouteGuard,
        ) -> Result<
            HostBindingCallBox<
                RouteReadinessOutcome<HostPreparedRouteGuard>,
                HostRouteCleanupSuccessor,
            >,
            BindingInputRejection<HostPreparedRouteGuard>,
        > {
            let rejected = self.probe.with(|state| {
                if state.reject_readiness_once {
                    state.reject_readiness_once = false;
                    state.readiness_rejections += 1;
                    true
                } else {
                    false
                }
            });
            if rejected {
                let error = BindingOperationalError::for_route(
                    *guard.route(),
                    CoreError::Binding(ErrorContext::new(ErrorPhase::Readiness, RetryClass::Never)),
                );
                return Err(BindingInputRejection::new(guard, error));
            }
            Ok(HostBindingCallBox::new(ReadyCall::pending_once(
                RouteReadinessOutcome::Ready(guard),
            )))
        }

        fn activate(
            &self,
            guard: HostPreparedRouteGuard,
        ) -> Result<
            HostBindingCallBox<
                RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
                HostRouteCleanupSuccessor,
            >,
            BindingInputRejection<HostPreparedRouteGuard>,
        > {
            Ok(HostBindingCallBox::new(ReadyCall::new(
                RouteActivationOutcome::Active(HostActiveRouteGuard::new(guard, 1_u8)),
            )))
        }

        fn commit(
            &self,
            guard: HostActiveRouteGuard,
        ) -> Result<
            HostBindingCallBox<
                RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
                HostRouteCleanupSuccessor,
            >,
            BindingInputRejection<HostActiveRouteGuard>,
        > {
            Ok(HostBindingCallBox::new(ReadyCall::new(
                RouteCommitOutcome::Committed(HostCommittedRouteGuard::new(guard, 2_u8)),
            )))
        }

        fn poll_accept(
            &self,
            _route: Pin<&mut HostCommittedRouteGuard>,
            permit: clinkz_wot_core::RouteActivationPermit<'_>,
            _cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            self.probe.with(|state| {
                if state.closed {
                    return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Terminal(
                        RouteTerminal::Closed {
                            route: *permit.route(),
                        },
                    )));
                }
                let Some((name, input)) = state.queued.take() else {
                    return Poll::Pending;
                };
                let correlation = CorrelationId::new(state.next_correlation);
                state.next_correlation += 1;
                state.in_flight = 1;
                Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Request(
                    RouteInboundRequest::new(
                        *permit.route(),
                        correlation,
                        AffordanceTarget::Property(Arc::from(name)),
                        input,
                    ),
                )))
            })
        }

        fn abort(
            &self,
            input: RouteAbortInput,
        ) -> Result<
            HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
            BindingInputRejection<RouteAbortInput>,
        > {
            let rejected = self.probe.with(|state| {
                if state.reject_abort_once {
                    state.reject_abort_once = false;
                    state.abort_rejections += 1;
                    true
                } else {
                    false
                }
            });
            if rejected {
                let (guard, cleanup) = input.into_parts();
                let error = BindingOperationalError::for_route(
                    *guard.route(),
                    CoreError::Binding(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never)),
                );
                return Err(BindingInputRejection::new(
                    RouteAbortInput::new(guard, cleanup),
                    error,
                ));
            }
            self.probe.with(|state| state.cleanup = 1);
            Ok(HostBindingCallBox::new(CleanupCall {
                input: Some(CleanupInput::Abort(input)),
                probe: self.probe.clone(),
            }))
        }

        fn shutdown(
            &self,
            input: RouteShutdownInput,
        ) -> Result<
            HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
            BindingInputRejection<RouteShutdownInput>,
        > {
            let rejected = self.probe.with(|state| {
                if state.reject_shutdown_once {
                    state.reject_shutdown_once = false;
                    state.shutdown_rejections += 1;
                    true
                } else {
                    false
                }
            });
            if rejected {
                let (guard, cleanup) = input.into_parts();
                let error = BindingOperationalError::for_route(
                    *guard.route(),
                    CoreError::Binding(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never)),
                );
                return Err(BindingInputRejection::new(
                    RouteShutdownInput::new(guard, cleanup),
                    error,
                ));
            }
            self.probe.with(|state| state.cleanup = 1);
            Ok(HostBindingCallBox::new(CleanupCall {
                input: Some(CleanupInput::Shutdown(input)),
                probe: self.probe.clone(),
            }))
        }

        fn deliver_response(
            &self,
            response: RouteInboundResponse,
        ) -> Result<
            HostBindingCallBox<BindingDeliveryOutcome>,
            BindingInputRejection<RouteInboundResponse>,
        > {
            Ok(HostBindingCallBox::new(DeliveryCall {
                response: Some(response),
                probe: self.probe.clone(),
            }))
        }
    }

    /// Builds the complete host-erased mock registration and its independent
    /// deterministic I/O probe.
    pub fn host_property_read_fixture() -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(false, false, false)
    }

    /// Rejects readiness and the first abort-constructor attempt while
    /// returning both complete inputs to the Servient cleanup owner.
    pub fn host_property_read_readiness_rejection_fixture()
    -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(true, true, false)
    }

    /// Rejects the first shutdown-constructor attempt while returning the
    /// complete committed guard and cleanup phase for a later retry.
    pub fn host_property_read_shutdown_rejection_fixture()
    -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(false, false, true)
    }

    fn host_property_read_fixture_with_rejections(
        reject_readiness_once: bool,
        reject_abort_once: bool,
        reject_shutdown_once: bool,
    ) -> (HostBindingRegistration, HostPropertyReadProbe) {
        let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
        let identity = BindingRegistrationIdentity::new(
            BindingId::new(7),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([0x52; 32]),
            compatibility,
            0,
        );
        let mut probe_state = ProbeState::default();
        probe_state.reject_readiness_once = reject_readiness_once;
        probe_state.reject_abort_once = reject_abort_once;
        probe_state.reject_shutdown_once = reject_shutdown_once;
        let state = WotLock::new(probe_state);
        let artifact_drops = Arc::new(AtomicU32::new(0));
        let input = HostBindingRegistrationInput::new(
            identity,
            BindingRegistrationCapabilities::producer_property_read(),
            BindingExecutionSupport::host_erased(),
            HostBindingCompilerRegistration::new(MockCompiler::with_artifact_drop_probe(
                compatibility,
                Arc::clone(&artifact_drops),
            )),
            Box::new(HostMockBinding {
                compatibility,
                probe: state.clone(),
            }),
            BindingResourceDeclarations::new(
                BindingLifetimeFootprint::new(4, 256),
                BindingLifetimeFootprint::new(4, 256),
            ),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(1, 64),
        );
        let registration = match HostBindingRegistration::new(input) {
            Ok(registration) => registration,
            Err(_) => panic!("complete host mock registration was rejected"),
        };
        (
            registration,
            HostPropertyReadProbe {
                state,
                artifact_drops,
            },
        )
    }
}

#[cfg(feature = "std")]
pub use host_fixture::{
    HostPropertyReadProbe, host_property_read_fixture,
    host_property_read_readiness_rejection_fixture, host_property_read_shutdown_rejection_fixture,
};
