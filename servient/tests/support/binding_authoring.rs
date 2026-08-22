use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll};
use std::sync::Arc;

use super::property_read_binding::{MockArtifact, MockCompiler};
use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactIdentity, BindingArtifactRef, BindingArtifactRole,
    BindingCallSettlement, BindingCandidate, BindingCompilerBounds, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerOutput, BindingCompilerStep, BindingConfigurationDigest,
    BindingDeliveryOutcome, BindingExecutionSupport, BindingGeneration, BindingId,
    BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint, BindingOperationalError,
    BindingRegistrationCapabilities, BindingRegistrationIdentity, BindingResourceDeclarations,
    BindingStatusPolicy, CleanupPhaseContext, CollisionDomainId, CoreError, Deadline,
    EndpointReservationKey, ErrorContext, ErrorPhase, HostActiveRouteGuard, HostBindingArtifact,
    HostBindingCall, HostBindingCallBox, HostBindingCompilerRegistration, HostBindingRegistration,
    HostBindingRegistrationInput, HostCommittedRouteGuard, HostPreparedRouteGuard,
    HostRouteCleanupSuccessor, HostShutdownRouteGuard, LogicalInteractionPlan, PlanId,
    PlanSetGeneration, PrepareInput, RetryClass, RouteAbortInput, RouteActivationOutcome,
    RouteCleanupOutcome, RouteCommitOutcome, RouteInboundResponse, RoutePrepareOutcome,
    RouteReadinessOutcome, RouteReservationIdentity, RouteServerBinding, RouteShutdownInput,
    ThingId,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};

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

struct AuthorRouteState {
    target: Box<str>,
    stage: AtomicU32,
}

struct OwnedCleanupCall<I> {
    input: Option<I>,
}

impl<I> OwnedCleanupCall<I> {
    fn new(input: I) -> Self {
        Self { input: Some(input) }
    }
}

impl<I> HostBindingCall<RouteCleanupOutcome, HostRouteCleanupSuccessor> for OwnedCleanupCall<I>
where
    I: Send + Unpin + 'static,
{
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        BindingLifetimeFootprint::new(1, 64)
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        if budget.consume(WorkClass::CleanupItems, 1).is_err() {
            return Poll::Pending;
        }
        drop(self.input.take().expect("cleanup polled after completion"));
        Poll::Ready(RouteCleanupOutcome::Complete)
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::CoreResult<
        clinkz_wot_core::StartStatus<
            BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        >,
    > {
        Ok(clinkz_wot_core::StartStatus::Pending)
    }

    fn poll_cancel(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<
        clinkz_wot_core::CoreResult<
            BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        >,
    > {
        drop(self.input.take().expect("cleanup cancelled twice"));
        Poll::Ready(Ok(BindingCallSettlement::Returned(
            RouteCleanupOutcome::Complete,
        )))
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

struct HostMockBinding {
    compatibility: BindingArtifactCompatibility,
    external_readiness: bool,
    prepares: Arc<AtomicU32>,
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
        HostBindingCallBox<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>,
        BindingInputRejection<PrepareInput>,
    > {
        let Some(target) = artifact
            .artifact()
            .try_payload::<MockArtifact>(self.compatibility)
            .and_then(MockArtifact::target)
        else {
            let error = BindingOperationalError::for_route(
                *input.route(),
                CoreError::Binding(ErrorContext::new(ErrorPhase::Prepare, RetryClass::Never)),
            );
            return Err(BindingInputRejection::new(input, error));
        };
        self.prepares.fetch_add(1, Ordering::SeqCst);
        let guard = HostPreparedRouteGuard::new(
            input,
            BindingLifetimeFootprint::new(1, 64),
            AuthorRouteState {
                target: Box::from(target),
                stage: AtomicU32::new(0),
            },
        );
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
        let state = guard
            .try_state_pin_ref::<AuthorRouteState>()
            .expect("external author recovers prepared shared state by type");
        assert_eq!(state.get_ref().stage.load(Ordering::SeqCst), 0);
        assert!(!state.get_ref().target.is_empty());
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
        guard
            .try_state_pin_ref::<AuthorRouteState>()
            .expect("external author projects prepared shared state")
            .get_ref()
            .stage
            .store(1, Ordering::SeqCst);
        let active = HostActiveRouteGuard::new(guard);
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
        let state = guard
            .try_state_pin_ref::<AuthorRouteState>()
            .expect("external author recovers active shared state by type");
        assert_eq!(state.get_ref().stage.load(Ordering::SeqCst), 1);
        state.get_ref().stage.store(2, Ordering::SeqCst);
        let committed = HostCommittedRouteGuard::new(guard);
        Ok(HostBindingCallBox::new(ReadyCall::new(
            RouteCommitOutcome::Committed(committed),
        )))
    }

    fn poll_accept(
        &self,
        route: &HostCommittedRouteGuard,
        _permit: clinkz_wot_core::RouteActivationPermit<'_>,
        _cx: &mut Context<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        assert_eq!(
            route
                .try_state_pin_ref::<AuthorRouteState>()
                .expect("external author recovers committed shared state by type")
                .get_ref()
                .stage
                .load(Ordering::SeqCst),
            2
        );
        Poll::Pending
    }

    fn abort(
        &self,
        input: RouteAbortInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteAbortInput>,
    > {
        Ok(HostBindingCallBox::new(OwnedCleanupCall::new(input)))
    }

    fn shutdown(
        &self,
        input: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteShutdownInput>,
    > {
        Ok(HostBindingCallBox::new(OwnedCleanupCall::new(input)))
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
            prepares: Arc::new(AtomicU32::new(0)),
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

struct WrongPayloadCompiler {
    compatibility: BindingArtifactCompatibility,
    reservation: RouteReservationIdentity,
}

impl BindingCompilerExtension for WrongPayloadCompiler {
    type Cursor = ();
    type Artifact = u32;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(
        &self,
        _input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, 4),
            0,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }

    fn start(&self, _input: &BindingCompilerInput<'_>) -> clinkz_wot_core::CoreResult<()> {
        Ok(())
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: (),
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<(), u32> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        assert_eq!(input.role(), BindingArtifactRole::ProducerRoute);
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::producer_route(
            self.compatibility,
            BindingArtifactFootprint::new(1, 4),
            self.reservation,
            0xfeed_beef,
        )))
    }

    fn abort(&self, _cursor: ()) {}
}

#[test]
fn host_concrete_payload_mismatch_returns_input_before_prepare_acceptance() {
    let compatibility = BindingArtifactCompatibility::new([0x71; 16]);
    let configuration = BindingConfigurationDigest::new([0x72; 32]);
    let binding_id = BindingId::new(9);
    let binding_generation = BindingGeneration::INITIAL;
    let plan_id = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
    let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
    let reservation = RouteReservationIdentity::new(
        CollisionDomainId::new([0x73; 16]),
        EndpointReservationKey::new([0x74; 32]),
    );
    let plan = LogicalInteractionPlan::try_property_read(
        plan_id,
        ThingId::from("urn:test:host-artifact-mismatch"),
        Box::from("level"),
        0,
        Box::from("mock://tank/level"),
        None,
        None,
    )
    .expect("valid logical plan");
    let candidate = BindingCandidate::new(
        binding_id,
        binding_generation,
        configuration,
        compatibility,
        0,
        0,
    );
    let compiler_input =
        BindingCompilerInput::new(&plan, candidate, BindingArtifactRole::ProducerRoute);
    let compiler = HostBindingCompilerRegistration::new(WrongPayloadCompiler {
        compatibility,
        reservation,
    });
    let cursor = compiler
        .start(&compiler_input)
        .expect("wrong payload cursor");
    let artifact = match compiler.step(
        &compiler_input,
        cursor,
        &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
    ) {
        BindingCompilerStep::Complete(output) => output.into_artifact(),
        _ => panic!("wrong payload compiler did not complete"),
    };
    let artifact_identity = BindingArtifactIdentity::new(
        plan_set_generation,
        plan_id,
        binding_id,
        binding_generation,
        configuration,
        compatibility,
        BindingArtifactRole::ProducerRoute,
    );
    let envelope = BindingArtifactEnvelope::try_new(
        artifact_identity,
        BindingArtifactFootprint::new(1, 4),
        artifact,
    )
    .expect("admitted wrong host payload type");
    let artifact_ref = BindingArtifactRef::new(artifact_identity, SlotIndex::new(0));
    let route = clinkz_wot_core::binding::BindingRouteKey::new(
        binding_id,
        binding_generation,
        Generation::INITIAL,
        plan_set_generation,
        plan_id,
        reservation,
    );
    let footprint = BindingLifetimeFootprint::new(2, 128);
    let prepare = PrepareInput::new(route, artifact_ref, footprint);
    let prepares = Arc::new(AtomicU32::new(0));
    let binding = HostMockBinding {
        compatibility,
        external_readiness: false,
        prepares: Arc::clone(&prepares),
    };
    let rejection = match binding.prepare(prepare, &envelope) {
        Ok(_) => panic!("wrong host payload type reached preparation"),
        Err(rejection) => rejection,
    };

    assert_eq!(prepares.load(Ordering::SeqCst), 0);
    assert_eq!(rejection.input().route(), &route);
    assert_eq!(rejection.input().artifact(), artifact_ref);
    assert_eq!(rejection.into_input().admitted_footprint(), footprint);
}
