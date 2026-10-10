//! Compiler construction is private and closed; a compatibility tuple is never proof.
#![allow(unused_variables)]
use crate::compiler::{self, CAPACITY, COMPATIBILITY, Impostor, InlineCompiler, Scenario};
#[cfg(feature = "std")]
use clinkz_wot_core::binding::ClientBinding;
use clinkz_wot_core::*;
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use core::{
    marker::PhantomData,
    task::{Context, Poll},
};

struct Endpoint<C>(PhantomData<C>);
impl<C> Endpoint<C> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<C: BindingCompilerExtension> PollServerBinding for Endpoint<C> {
    type Compiler = C;
    type RouteState = ();
    type ReadinessState = ();
    type ResponseState = ();
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
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
        artifact: &BindingArtifactEnvelope<<Self::Compiler as BindingCompilerExtension>::Artifact>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<RoutePrepareOutcome<()>>, BindingInputRejection<PrepareInput>> {
        Err(BindingInputRejection::new(
            input,
            BindingOperationalError::new(compiler::error()),
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
            BindingOperationalError::new(compiler::error()),
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

impl<C: BindingCompilerExtension> PollClientBinding for Endpoint<C> {
    type Compiler = C;
    type RequestState = ();
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<()>(BindingLifetimeFootprint::new(0, 0))
    }
    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<<Self::Compiler as BindingCompilerExtension>::Artifact>,
        slot: &mut ClientRequestSlot<Self::RequestState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        Err(BindingInputRejection::new(
            request,
            BindingOperationalError::new(compiler::error()),
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
        COMPATIBILITY
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
            BindingOperationalError::new(compiler::error()),
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
            BindingOperationalError::new(compiler::error()),
        ))
    }
}

#[cfg(feature = "std")]
impl<C: BindingCompilerExtension + Send + Sync> ClientBinding for Endpoint<C> {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
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
            BindingOperationalError::new(compiler::error()),
        ))
    }
}

type Complete<C> = StaticBindingRegistration<StaticBindingComponents<Endpoint<C>, Endpoint<C>>>;
enum Registration {
    Inline(Complete<InlineCompiler>),
    Impostor(Complete<Impostor>),
    #[cfg(feature = "std")]
    LegacyHost(HostBindingRegistration),
}

/// Opaque complete bundle, constructed by this fixture's closed factory.
pub struct CompleteRegistration(Registration);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Representation {
    Static,
    ReservedHostPrototype,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureError {
    Absent,
    Mismatch,
    UnsupportedImplementation,
    UnsupportedRepresentation,
    Configuration,
    Capability,
}

/// Private construction prevents a caller minting support from declarations.
///
/// ```compile_fail
/// use consumer_compiler_support_probe::{CheckedRegistration, Representation};
/// let forged = CheckedRegistration {
///     registration: unsafe { core::mem::zeroed() },
///     representation: Representation::Static,
/// };
/// ```
pub struct CheckedRegistration {
    registration: Complete<InlineCompiler>,
    representation: Representation,
}

fn identity(capacity: usize, scenario: Scenario) -> BindingRegistrationIdentity {
    let mut digest = [0x71; 32];
    digest[0] = capacity as u8;
    digest[1] = scenario as u8;
    BindingRegistrationIdentity::new(
        BindingId::new(7),
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new(digest),
        COMPATIBILITY,
        0,
    )
}
fn resources() -> BindingResourceDeclarations {
    BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(8, 4096),
    )
}
fn complete<C: BindingCompilerExtension>(
    compiler: C,
    capacity: usize,
    scenario: Scenario,
) -> Complete<C> {
    let input = StaticBindingRegistrationInput::producer_and_consumer_property_read(
        identity(capacity, scenario),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(compiler),
        StaticBindingComponents::new(Endpoint::<C>::new(), Endpoint::<C>::new()),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(0, 0),
    );
    StaticBindingRegistration::producer_and_consumer_property_read(input)
        .unwrap_or_else(|_| panic!("complete fixture registration rejected"))
}

pub fn registration(
    capacity: usize,
    scenario: Scenario,
) -> Result<CompleteRegistration, CaptureError> {
    if !(1..=CAPACITY).contains(&capacity) {
        return Err(CaptureError::Configuration);
    }
    Ok(CompleteRegistration(Registration::Inline(complete(
        InlineCompiler { capacity, scenario },
        capacity,
        scenario,
    ))))
}
pub fn unsupported_registration() -> CompleteRegistration {
    CompleteRegistration(Registration::Impostor(complete(
        Impostor,
        CAPACITY,
        Scenario::Success,
    )))
}
pub fn producer_only_registration() -> CompleteRegistration {
    let input = StaticBindingRegistrationInput::new(
        identity(CAPACITY, Scenario::Success),
        BindingRegistrationCapabilities::producer_property_read(),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(InlineCompiler {
            capacity: CAPACITY,
            scenario: Scenario::Success,
        }),
        StaticBindingComponents::new(
            Endpoint::<InlineCompiler>::new(),
            Endpoint::<InlineCompiler>::new(),
        ),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(0, 0),
    );
    CompleteRegistration(Registration::Inline(
        StaticBindingRegistration::new(input)
            .unwrap_or_else(|_| panic!("Producer-only input rejected")),
    ))
}
/// Negative witness: startup validity does not authenticate compiler configuration.
pub fn mismatched_configuration_registration() -> CompleteRegistration {
    CompleteRegistration(Registration::Inline(complete(
        InlineCompiler {
            capacity: CAPACITY / 2,
            scenario: Scenario::Success,
        },
        CAPACITY,
        Scenario::Success,
    )))
}
#[cfg(feature = "std")]
pub fn legacy_host_registration() -> CompleteRegistration {
    use alloc::boxed::Box;
    let input = HostBindingRegistrationInput::producer_and_consumer_property_read(
        identity(CAPACITY, Scenario::Success),
        BindingExecutionSupport::host_erased(),
        HostBindingCompilerRegistration::new(InlineCompiler {
            capacity: CAPACITY,
            scenario: Scenario::Success,
        }),
        Box::new(Endpoint::<InlineCompiler>::new()),
        Box::new(Endpoint::<InlineCompiler>::new()),
        resources(),
        BindingIngressPolicy::hidden(),
        BindingStatusPolicy::new(0, 0),
    );
    CompleteRegistration(Registration::LegacyHost(
        HostBindingRegistration::new(input)
            .unwrap_or_else(|_| panic!("complete Host fixture registration rejected")),
    ))
}
impl CompleteRegistration {
    pub fn identity(&self) -> BindingRegistrationIdentity {
        match &self.0 {
            Registration::Inline(r) => r.identity(),
            Registration::Impostor(r) => r.identity(),
            #[cfg(feature = "std")]
            Registration::LegacyHost(r) => r.identity(),
        }
    }
}

pub fn capture(
    registration: Option<CompleteRegistration>,
    expected: BindingRegistrationIdentity,
    representation: Representation,
) -> Result<CheckedRegistration, (Option<CompleteRegistration>, CaptureError)> {
    let Some(registration) = registration else {
        return Err((None, CaptureError::Absent));
    };
    if registration.identity() != expected {
        return Err((Some(registration), CaptureError::Mismatch));
    }
    if !cfg!(feature = "std") && representation == Representation::ReservedHostPrototype {
        return Err((Some(registration), CaptureError::UnsupportedRepresentation));
    }
    match registration.0 {
        Registration::Inline(registration) => {
            if !registration
                .capabilities()
                .supports_consumer_property_read()
            {
                return Err((
                    Some(CompleteRegistration(Registration::Inline(registration))),
                    CaptureError::Capability,
                ));
            }
            let compiler = registration.compiler().compiler();
            if registration.identity() != identity(compiler.capacity, compiler.scenario) {
                return Err((
                    Some(CompleteRegistration(Registration::Inline(registration))),
                    CaptureError::Configuration,
                ));
            }
            Ok(CheckedRegistration {
                registration,
                representation,
            })
        }
        Registration::Impostor(r) => Err((
            Some(CompleteRegistration(Registration::Impostor(r))),
            CaptureError::UnsupportedImplementation,
        )),
        #[cfg(feature = "std")]
        Registration::LegacyHost(r) => Err((
            Some(CompleteRegistration(Registration::LegacyHost(r))),
            CaptureError::UnsupportedRepresentation,
        )),
    }
}
impl CheckedRegistration {
    pub fn identity(&self) -> BindingRegistrationIdentity {
        self.registration.identity()
    }
    pub fn representation(&self) -> Representation {
        self.representation
    }
    pub(crate) fn compiler(&self) -> &InlineCompiler {
        self.registration.compiler().compiler()
    }
    pub(crate) fn matches(&self, input: &BindingCompilerInput<'_>) -> bool {
        let id = self.identity();
        let candidate = input.candidate();
        input.role() == BindingArtifactRole::ConsumerCall
            && id.binding_id() == candidate.binding_id()
            && id.binding_generation() == candidate.binding_generation()
            && id.configuration() == candidate.configuration()
            && id.artifact_compatibility() == candidate.compatibility()
    }
    /// Fixed cost is known from source before bounds is invoked.
    pub fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<BindingCompilerBounds> {
        if !self.matches(input) || budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Err(compiler::error());
        }
        self.compiler().bounds(input)
    }
    pub fn start_static<'r>(
        &'r self,
        input: &BindingCompilerInput<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StaticCursor<'r>> {
        if self.representation != Representation::Static
            || !self.matches(input)
            || budget.remaining(WorkClass::BindingPolls) < 1
            || budget.remaining(WorkClass::CleanupItems) < 1
        {
            return Err(compiler::error());
        }
        budget.consume(WorkClass::BindingPolls, 1).unwrap();
        budget.consume(WorkClass::CleanupItems, 1).unwrap();
        let cursor = self.compiler().start(input)?;
        Ok(StaticCursor {
            checked: self,
            plan: input.logical_plan().plan_id(),
            cursor: Some(cursor),
            remaining: 2,
        })
    }
    pub(crate) fn drive(
        &self,
        input: &BindingCompilerInput<'_>,
        plan: PlanId,
        cursor: compiler::InlineCursor,
        remaining: &mut u32,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<compiler::InlineCursor, compiler::InlineArtifact> {
        if !self.matches(input) || input.logical_plan().plan_id() != plan || *remaining == 0 {
            return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                compiler::error(),
                cursor,
            ));
        }
        // Atomic callback plus compiler debit: a one-unit retry changes nothing.
        if budget.consume(WorkClass::BindingPolls, 2).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        *remaining -= 1;
        let mut primitive = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        self.compiler().step(input, cursor, &mut primitive)
    }
}

pub struct StaticCursor<'r> {
    checked: &'r CheckedRegistration,
    plan: PlanId,
    cursor: Option<compiler::InlineCursor>,
    remaining: u32,
}
pub enum StaticStep<'r> {
    Pending(StaticCursor<'r>),
    Complete(BindingCompilerOutput<compiler::InlineArtifact>),
    Failed(CoreError, StaticCursor<'r>),
}
impl<'r> StaticCursor<'r> {
    pub fn step(
        mut self,
        input: &BindingCompilerInput<'_>,
        budget: &mut WorkBudget,
    ) -> StaticStep<'r> {
        match self.checked.drive(
            input,
            self.plan,
            self.cursor.take().unwrap(),
            &mut self.remaining,
            budget,
        ) {
            BindingCompilerStep::Pending(cursor) => {
                self.cursor = Some(cursor);
                StaticStep::Pending(self)
            }
            BindingCompilerStep::Failed(failure) => {
                let (error, cursor) = failure.into_parts();
                self.cursor = Some(cursor);
                StaticStep::Failed(error, self)
            }
            BindingCompilerStep::Complete(output) => StaticStep::Complete(output),
        }
    }
}
impl Drop for StaticCursor<'_> {
    fn drop(&mut self) {
        if let Some(cursor) = self.cursor.take() {
            self.checked.compiler().abort(cursor);
        }
    }
}
