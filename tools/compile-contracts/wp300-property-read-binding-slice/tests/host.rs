use core::pin::Pin;
use core::task::{Context, Poll};

use clinkz_wot_core::{
    BindingArtifactCompatibility, BindingCallSettlement, BindingDeliveryOutcome,
    BindingExecutionSupport, BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint,
    BindingRegistrationCapabilities, BindingRegistrationIdentity, BindingResourceDeclarations,
    BindingStatusPolicy, CleanupPhaseContext, Deadline, HostActiveRouteGuard, HostBindingCall,
    HostBindingCallBox, HostBindingCompilerRegistration, HostBindingRegistration,
    HostBindingRegistrationInput, HostCommittedRouteGuard, HostPreparedRouteGuard,
    HostRouteCleanupSuccessor, HostShutdownRouteGuard, PrepareInput, RouteAbortInput,
    RouteActivationOutcome, RouteCleanupOutcome, RouteCommitOutcome, RouteInboundResponse,
    RoutePrepareOutcome, RouteReadinessOutcome, RouteServerBinding, RouteShutdownInput,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_wp300_property_read_binding_slice_contract::MockCompiler;

struct ReadyCall<T, C> {
    value: Option<T>,
    footprint: BindingLifetimeFootprint,
    pending_polls: u8,
    _cleanup: core::marker::PhantomData<C>,
}

impl<T, C> ReadyCall<T, C> {
    fn new(value: T) -> Self {
        Self {
            value: Some(value),
            footprint: BindingLifetimeFootprint::new(1, 64),
            pending_polls: 0,
            _cleanup: core::marker::PhantomData,
        }
    }

    fn pending_once(value: T) -> Self {
        Self {
            value: Some(value),
            footprint: BindingLifetimeFootprint::new(1, 64),
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
        self.footprint
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
                .expect("ready call polled after completion"),
        )
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::CoreResult<clinkz_wot_core::StartStatus<BindingCallSettlement<T, C>>>
    {
        Ok(clinkz_wot_core::StartStatus::Pending)
    }

    fn poll_cancel(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<T, C>>> {
        Poll::Ready(Ok(BindingCallSettlement::Returned(
            self.value.take().expect("cancel polled after completion"),
        )))
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

struct HostMockBinding {
    compatibility: BindingArtifactCompatibility,
    external_readiness: bool,
}

impl RouteServerBinding for HostMockBinding {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn prepare(
        &self,
        input: PrepareInput,
    ) -> Result<
        HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>,
        BindingInputRejection<PrepareInput>,
    > {
        let guard = HostPreparedRouteGuard::new(input, BindingLifetimeFootprint::new(1, 64), 0_u8);
        Ok(HostBindingCallBox::new(ReadyCall::new(
            RoutePrepareOutcome::Prepared(guard),
        )))
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
        let outcome = RouteReadinessOutcome::Ready(guard);
        if self.external_readiness {
            Ok(HostBindingCallBox::new(ReadyCall::pending_once(outcome)))
        } else {
            Ok(HostBindingCallBox::new(ReadyCall::new(outcome)))
        }
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
        let active = HostActiveRouteGuard::new(guard, 1_u8);
        Ok(HostBindingCallBox::new(ReadyCall::new(
            RouteActivationOutcome::Active(active),
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
        let committed = HostCommittedRouteGuard::new(guard, 2_u8);
        Ok(HostBindingCallBox::new(ReadyCall::new(
            RouteCommitOutcome::Committed(committed),
        )))
    }

    fn poll_accept(
        &self,
        _route: Pin<&mut HostCommittedRouteGuard>,
        _permit: clinkz_wot_core::RouteActivationPermit<'_>,
        _cx: &mut Context<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        Poll::Pending
    }

    fn abort(
        &self,
        _input: RouteAbortInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteAbortInput>,
    > {
        Ok(HostBindingCallBox::new(ReadyCall::new(
            RouteCleanupOutcome::Complete,
        )))
    }

    fn shutdown(
        &self,
        _input: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteShutdownInput>,
    > {
        Ok(HostBindingCallBox::new(ReadyCall::new(
            RouteCleanupOutcome::Complete,
        )))
    }

    fn deliver_response(
        &self,
        _response: RouteInboundResponse,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<RouteInboundResponse>,
    > {
        Ok(HostBindingCallBox::new(ReadyCall::new(
            BindingDeliveryOutcome::Delivered,
        )))
    }
}

fn host_registration(
    identity: BindingRegistrationIdentity,
    compatibility: BindingArtifactCompatibility,
    capabilities: BindingRegistrationCapabilities,
    execution: BindingExecutionSupport,
    resources: BindingResourceDeclarations,
    ingress: BindingIngressPolicy,
    status: BindingStatusPolicy,
    external_readiness: bool,
) -> Result<HostBindingRegistration, BindingInputRejection<HostBindingRegistrationInput>> {
    HostBindingRegistration::new(HostBindingRegistrationInput::new(
        identity,
        capabilities,
        execution,
        HostBindingCompilerRegistration::new(MockCompiler::new(compatibility)),
        Box::new(HostMockBinding {
            compatibility,
            external_readiness,
        }),
        resources,
        ingress,
        status,
    ))
}

#[test]
fn public_host_author_can_construct_both_readiness_shapes() {
    let _constructor = host_registration;
    let _shutdown_guard: Option<HostShutdownRouteGuard> = None;
}
