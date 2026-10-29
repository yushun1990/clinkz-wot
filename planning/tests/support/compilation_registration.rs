//! Inert execution halves for tests of the real checked Planning compiler.
#![allow(dead_code, unused_variables)]
#[cfg(feature = "std")]
use clinkz_wot_core::binding::ClientBinding;
use clinkz_wot_core::*;
use clinkz_wot_foundation::WorkBudget;
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
use core::{
    marker::PhantomData,
    task::{Context, Poll},
};

struct Endpoint<C>(PhantomData<C>, BindingArtifactCompatibility);
impl<C> Endpoint<C> {
    fn for_compatibility(compatibility: BindingArtifactCompatibility) -> Self {
        Self(PhantomData, compatibility)
    }
}

impl<C: BindingCompilerExtension> clinkz_wot_planning::PrepareServerBinding for Endpoint<C> {
    type Compiler = C;
}
impl<C: BindingCompilerExtension> PollServerBinding for Endpoint<C> {
    type Artifact = <C as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
    type RouteState = ();
    type ReadinessState = ();
    type ResponseState = ();
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.1
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
        input: PrepareInput,
        artifact: &BindingArtifactEnvelope<Self::Artifact>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<RoutePrepareOutcome<()>>, BindingInputRejection<PrepareInput>> {
        Err(BindingInputRejection::new(
            input,
            BindingOperationalError::new(error()),
        ))
    }
    fn poll_prepare(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_cancel_prepare(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RoutePrepareOutcome<()>, ()>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_readiness(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteReadinessOutcome<()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_readiness(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteReadinessOutcome<()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_cancel_readiness(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteReadinessOutcome<()>, ()>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_activate(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteActivationOutcome<(), ()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_activate(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteActivationOutcome<(), ()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_cancel_activate(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteActivationOutcome<(), ()>, ()>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_commit(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteCommitOutcome<(), ()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_commit(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCommitOutcome<(), ()>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_cancel_commit(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteCommitOutcome<(), ()>, ()>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        permit: RouteActivationPermit<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_abort(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_abort(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_shutdown(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_shutdown(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        panic!("execution is outside this compiler-only witness")
    }
    fn acknowledge_route(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
    ) -> CoreResult<()> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_response(
        &mut self,
        response: RouteInboundResponse,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<BindingDeliveryOutcome>, BindingInputRejection<RouteInboundResponse>>
    {
        Err(BindingInputRejection::new(
            response,
            BindingOperationalError::new(error()),
        ))
    }
    fn poll_response(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Poll<BindingDeliveryOutcome> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_cancel_response(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: &CleanupPhaseContext,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn acknowledge_response(
        &mut self,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
    ) -> CoreResult<()> {
        panic!("execution is outside this compiler-only witness")
    }
}

impl<C: BindingCompilerExtension> clinkz_wot_planning::PrepareClientBinding for Endpoint<C> {
    type Compiler = C;
}
impl<C: BindingCompilerExtension> PollClientBinding for Endpoint<C> {
    type Artifact = <C as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
    type RequestState = ();
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.1
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(0, 0))
    }
    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<Self::Artifact>,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        Err(BindingInputRejection::new(
            request,
            BindingOperationalError::new(error()),
        ))
    }
    fn poll_request(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn start_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_cancel_request(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn acknowledge_request(
        &mut self,
        slot: &mut ClientRequestSlot<Self::RequestState>,
    ) -> CoreResult<()> {
        panic!("execution is outside this compiler-only witness")
    }
}

#[cfg(feature = "std")]
impl<C: BindingCompilerExtension + Send + Sync> RouteServerBinding for Endpoint<C> {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.1
    }
    fn prepare(
        &self,
        input: PrepareInput,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>,
        BindingInputRejection<PrepareInput>,
    > {
        Err(BindingInputRejection::new(
            input,
            BindingOperationalError::new(error()),
        ))
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
        panic!("execution is outside this compiler-only witness")
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
        panic!("execution is outside this compiler-only witness")
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
        panic!("execution is outside this compiler-only witness")
    }
    fn poll_accept(
        &self,
        route: &HostCommittedRouteGuard,
        permit: RouteActivationPermit<'_>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<RouteAcceptEvent>> {
        panic!("execution is outside this compiler-only witness")
    }
    fn abort(
        &self,
        input: RouteAbortInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteAbortInput>,
    > {
        panic!("execution is outside this compiler-only witness")
    }
    fn shutdown(
        &self,
        input: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteShutdownInput>,
    > {
        panic!("execution is outside this compiler-only witness")
    }
    fn deliver_response(
        &self,
        response: RouteInboundResponse,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<RouteInboundResponse>,
    > {
        Err(BindingInputRejection::new(
            response,
            BindingOperationalError::new(error()),
        ))
    }
}

#[cfg(feature = "std")]
impl<C: BindingCompilerExtension + Send + Sync> ClientBinding for Endpoint<C> {
    fn supports_operation(&self, operation: clinkz_wot_core::Operation) -> bool {
        operation == clinkz_wot_core::Operation::ReadProperty
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.1
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        Err(BindingInputRejection::new(
            request,
            BindingOperationalError::new(error()),
        ))
    }
}

fn error() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

pub fn production_static_registration(
    compiler: ResolvedTargetCompiler,
    identity: BindingRegistrationIdentity,
) -> InlineConsumerCompilerRegistration<
    StaticBindingPreparation<
        impl PrepareServerBinding<Compiler = ResolvedTargetCompiler, Artifact = ResolvedTargetArtifact>,
    >,
> {
    let compatibility = identity.artifact_compatibility();
    let input = StaticBindingPreparationInput::producer_and_consumer_property_read(
        identity,
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(compiler),
        StaticBindingComponents::new(
            Endpoint::<ResolvedTargetCompiler>::for_compatibility(compatibility),
            Endpoint::<ResolvedTargetCompiler>::for_compatibility(compatibility),
        ),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(3, 5),
    );
    StaticBindingPreparation::producer_and_consumer_property_read(input)
        .unwrap_or_else(|_| panic!("production complete static registration"))
        .try_into_inline_consumer_compiler()
        .unwrap_or_else(|_| panic!("production static support capture"))
}

#[cfg(feature = "std")]
pub fn production_host_registration(
    compiler: ResolvedTargetCompiler,
    identity: BindingRegistrationIdentity,
) -> InlineConsumerCompilerRegistration<HostBindingPreparation> {
    let compatibility = identity.artifact_compatibility();
    let compiler = HostBindingCompilerRegistration::try_new_inline_consumer(compiler)
        .unwrap_or_else(|_| panic!("production supported Host adapter"));
    let input = HostBindingPreparationInput::producer_and_consumer_property_read(
        identity,
        BindingExecutionSupport::host_erased(),
        compiler,
        Box::new(Endpoint::<ResolvedTargetCompiler>::for_compatibility(
            compatibility,
        )),
        Box::new(Endpoint::<ResolvedTargetCompiler>::for_compatibility(
            compatibility,
        )),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(3, 5),
    );
    HostBindingPreparation::new(input)
        .unwrap_or_else(|_| panic!("production complete Host registration"))
        .try_into_inline_consumer_compiler()
        .unwrap_or_else(|_| panic!("production Host support capture"))
}

fn resources() -> BindingResourceDeclarations {
    BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(8, 4096),
    )
}
