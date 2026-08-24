//! Deterministic external Property Read binding used by the aggregate architecture proof.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{
    boxed::Box,
    rc::{Rc, Weak as RcWeak},
    sync::Arc,
};
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
    BindingIngressLimits, BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint,
    BindingOperationalError, BindingRegistrationCapabilities, BindingRegistrationIdentity,
    BindingResourceDeclarations, BindingStateLayout, BindingStatusPolicy, CleanupPhaseContext,
    CollisionDomainId, CoreError, CorrelationId, EndpointReservationKey, ErrorContext, ErrorPhase,
    InteractionInput, NoCleanupSuccessor, PollServerBinding, PrepareInput, RetryClass,
    RouteActivationOutcome, RouteActivationPermit, RouteCleanupOutcome, RouteCommitOutcome,
    RouteInboundRequest, RouteInboundResponse, RoutePreparationVisibility, RoutePrepareOutcome,
    RouteReadinessOutcome, RouteReadinessSlot, RouteReservationIdentity, RouteTerminal,
    ServerResponseSlot, ServerRouteSlot, StaticBindingCompilerRegistration,
    StaticBindingRegistration, StaticBindingRegistrationInput,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

const FIXTURE_INGRESS_ITEMS: u32 = 1;
const FIXTURE_INGRESS_BYTES: u64 = 1_024;

/// Returns the exact one-route ingress declaration enforced by both aggregate cells.
pub const fn fixture_ingress_policy() -> BindingIngressPolicy {
    let limits = BindingIngressLimits::new(FIXTURE_INGRESS_ITEMS, FIXTURE_INGRESS_BYTES);
    BindingIngressPolicy::new(RoutePreparationVisibility::Hidden, limits, limits, limits)
}

fn ingress_retained_bytes(name: &str, input: &InteractionInput) -> Option<u64> {
    let mut bytes = core::mem::size_of::<(Box<str>, InteractionInput)>() as u64;
    let mut add = |len: usize| {
        bytes = bytes.checked_add(u64::try_from(len).ok()?)?;
        Some(())
    };
    add(name.len())?;
    if let Some(payload) = input.data.as_ref() {
        add(payload.body.len())?;
        add(payload.content_type.len())?;
        if let Some(coding) = payload.content_coding.as_ref() {
            add(coding.len())?;
        }
    }
    for (key, value) in &input.uri_variables {
        add(key.len())?;
        add(value.len())?;
    }
    if let Some(principal) = input.principal.as_ref() {
        add(principal.id.as_str().len())?;
        for scope in &principal.scopes {
            add(scope.len())?;
        }
    }
    if let Some(accept) = input.accept.as_ref() {
        add(accept.preferred.as_str().len())?;
        if let Some(alternatives) = accept.alternatives.as_ref() {
            for media_type in alternatives {
                add(media_type.as_str().len())?;
            }
        }
    }
    Some(bytes)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct IngressUsage {
    items: u32,
    bytes: u64,
}

#[derive(Debug)]
struct IngressAccounting {
    limits: [BindingIngressLimits; 3],
    used: [IngressUsage; 3],
}

impl IngressAccounting {
    fn new(policy: BindingIngressPolicy) -> Self {
        Self {
            limits: [policy.per_route(), policy.per_binding(), policy.global()],
            used: [IngressUsage::default(); 3],
        }
    }

    fn try_admit(&mut self, bytes: u64) -> bool {
        let mut next = self.used;
        for (usage, limit) in next.iter_mut().zip(self.limits) {
            let Some(items) = usage.items.checked_add(1) else {
                return false;
            };
            let Some(retained_bytes) = usage.bytes.checked_add(bytes) else {
                return false;
            };
            if items > limit.items() || retained_bytes > limit.bytes() {
                return false;
            }
            usage.items = items;
            usage.bytes = retained_bytes;
        }
        self.used = next;
        true
    }

    fn release(&mut self, bytes: u64) {
        for usage in &mut self.used {
            usage.items = usage
                .items
                .checked_sub(1)
                .expect("ingress item released without admission");
            usage.bytes = usage
                .bytes
                .checked_sub(bytes)
                .expect("ingress bytes released without admission");
        }
    }

    fn usage(&self) -> [(u32, u64); 3] {
        self.used.map(|usage| (usage.items, usage.bytes))
    }
}

fn validate_live_response_identity(
    response: RouteInboundResponse,
    expected: Option<(clinkz_wot_core::binding::BindingRouteKey, CorrelationId)>,
) -> Result<RouteInboundResponse, BindingInputRejection<RouteInboundResponse>> {
    let route = *response.opportunity().route();
    let correlation = response.opportunity().correlation();
    if expected != Some((route, correlation)) {
        return Err(BindingInputRejection::new(
            response,
            BindingOperationalError::for_route(
                route,
                CoreError::Binding(ErrorContext::new(ErrorPhase::Delivery, RetryClass::Never)),
            ),
        ));
    }
    Ok(response)
}

/// Protocol-edge observation of one terminal response delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveredResponseEvidence {
    correlation: CorrelationId,
    payload: Option<Box<[u8]>>,
    media_type: Option<Box<str>>,
    validation_failure: bool,
}

impl DeliveredResponseEvidence {
    fn from_response(response: &RouteInboundResponse) -> Self {
        let (payload, media_type) = response
            .result()
            .ok()
            .and_then(|output| output.data())
            .map(|payload| {
                (
                    Some(Box::from(payload.body.as_ref())),
                    Some(Box::from(payload.content_type.as_str())),
                )
            })
            .unwrap_or((None, None));
        Self {
            correlation: response.opportunity().correlation(),
            payload,
            media_type,
            validation_failure: matches!(response.result(), Err(CoreError::Validation(_))),
        }
    }

    /// Returns the binding-owned request/response correlation token.
    pub const fn correlation(&self) -> CorrelationId {
        self.correlation
    }

    /// Returns the delivered application bytes for a successful response.
    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    /// Returns the delivered payload media type for a successful response.
    pub fn media_type(&self) -> Option<&str> {
        self.media_type.as_deref()
    }

    /// Returns whether Core sealed an invalid nominal success into validation failure.
    pub const fn is_validation_failure(&self) -> bool {
        self.validation_failure
    }
}

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

#[derive(Debug)]
struct StaticRouteIo {
    route: clinkz_wot_core::binding::BindingRouteKey,
    accounting: IngressAccounting,
    queued: Option<(Box<str>, InteractionInput)>,
    retained_ingress_bytes: Option<u64>,
    next_correlation: u64,
    in_flight: Option<CorrelationId>,
    closed: bool,
}

impl StaticRouteIo {
    fn new(route: clinkz_wot_core::binding::BindingRouteKey) -> Self {
        Self {
            route,
            accounting: IngressAccounting::new(fixture_ingress_policy()),
            queued: None,
            retained_ingress_bytes: None,
            next_correlation: 1,
            in_flight: None,
            closed: false,
        }
    }

    fn enqueue(&mut self, name: &str, input: InteractionInput) -> Result<(), InteractionInput> {
        let Some(bytes) = ingress_retained_bytes(name, &input) else {
            return Err(input);
        };
        if self.closed
            || self.queued.is_some()
            || self.in_flight.is_some()
            || self.retained_ingress_bytes.is_some()
            || bytes > FIXTURE_INGRESS_BYTES
            || !self.accounting.try_admit(bytes)
        {
            return Err(input);
        }
        self.queued = Some((Box::from(name), input));
        self.retained_ingress_bytes = Some(bytes);
        Ok(())
    }

    fn accept(&mut self) -> Option<((Box<str>, InteractionInput), CorrelationId)> {
        let request = self.queued.take()?;
        let correlation = CorrelationId::new(self.next_correlation);
        self.next_correlation = self.next_correlation.checked_add(1).unwrap_or(0);
        if self.next_correlation == 0 {
            self.queued = Some(request);
            return None;
        }
        self.in_flight = Some(correlation);
        Some((request, correlation))
    }

    fn settle(&mut self, expected: CorrelationId) -> bool {
        if self.in_flight != Some(expected) {
            return false;
        }
        self.in_flight = None;
        let bytes = self
            .retained_ingress_bytes
            .take()
            .expect("live correlation retains its ingress byte charge");
        self.accounting.release(bytes);
        true
    }

    fn close(&mut self) {
        self.queued = None;
        if let Some(bytes) = self.retained_ingress_bytes.take() {
            self.accounting.release(bytes);
        }
        self.in_flight = None;
        self.closed = true;
    }
}

/// Protocol state retained in one caller-owned route slot.
#[derive(Debug)]
pub struct MockRouteState {
    phase: u8,
    target: Box<str>,
    io: Rc<RefCell<StaticRouteIo>>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum MockLifecyclePhase {
    Prepare,
    Readiness,
    Activate,
    Commit,
    ResponseDelivery,
    Abort,
    Shutdown,
    CleanupCallCancellation,
}

const MOCK_PHASE_COUNT: usize = 8;

struct StaticProbeState {
    ingress: Option<RcWeak<RefCell<StaticRouteIo>>>,
    delivered: u32,
    delivered_validation_errors: u32,
    response_settlements: u32,
    routes: u32,
    in_flight: u32,
    in_flight_identity: Option<(clinkz_wot_core::binding::BindingRouteKey, CorrelationId)>,
    last_accepted_correlation: Option<CorrelationId>,
    delivered_response: Option<DeliveredResponseEvidence>,
    cleanup: u32,
    aborts: u32,
    carrier_checks: u32,
    preparation_side_effects: u32,
    lifecycle_starts: [u32; MOCK_PHASE_COUNT],
    lifecycle_cancellations: [u32; MOCK_PHASE_COUNT],
    closed: bool,
    prepared_target: Option<Box<str>>,
}

impl Default for StaticProbeState {
    fn default() -> Self {
        Self {
            ingress: None,
            delivered: 0,
            delivered_validation_errors: 0,
            response_settlements: 0,
            routes: 0,
            in_flight: 0,
            in_flight_identity: None,
            last_accepted_correlation: None,
            delivered_response: None,
            cleanup: 0,
            aborts: 0,
            carrier_checks: 0,
            preparation_side_effects: 0,
            lifecycle_starts: [0; MOCK_PHASE_COUNT],
            lifecycle_cancellations: [0; MOCK_PHASE_COUNT],
            closed: false,
            prepared_target: None,
        }
    }
}

/// Deterministic no-default protocol I/O visible to the Servient runner.
#[derive(Clone)]
pub struct StaticPropertyReadProbe {
    state: Rc<RefCell<StaticProbeState>>,
    artifact_drops: Arc<AtomicU32>,
}

impl StaticPropertyReadProbe {
    pub fn enqueue_property_read(&self, name: &str, input: InteractionInput) {
        let ingress = self
            .state
            .borrow()
            .ingress
            .as_ref()
            .and_then(RcWeak::upgrade)
            .expect("caller-owned route ingress is not prepared");
        ingress
            .borrow_mut()
            .enqueue(name, input)
            .expect("mock ingress item/byte capacity is exhausted");
    }

    pub fn delivered_responses(&self) -> u32 {
        self.state.borrow().delivered
    }

    /// Returns Core-sealed validation failures accepted for delivery.
    pub fn delivered_validation_errors(&self) -> u32 {
        self.state.borrow().delivered_validation_errors
    }

    /// Returns terminal response-delivery settlements.
    pub fn response_settlements(&self) -> u32 {
        self.state.borrow().response_settlements
    }

    /// Returns the correlation allocated for the accepted request.
    pub fn last_accepted_correlation(&self) -> Option<CorrelationId> {
        self.state.borrow().last_accepted_correlation
    }

    /// Returns the protocol-edge observation of the delivered response.
    pub fn delivered_response(&self) -> Option<DeliveredResponseEvidence> {
        self.state.borrow().delivered_response.clone()
    }

    pub fn outstanding_counts(&self) -> (u32, u32, u32, u32) {
        let state = self.state.borrow();
        let queued = state
            .ingress
            .as_ref()
            .and_then(RcWeak::upgrade)
            .is_some_and(|io| io.borrow().queued.is_some());
        (
            state.routes,
            u32::from(queued),
            state.in_flight,
            state.cleanup,
        )
    }

    pub fn ingress_usage(&self) -> [(u32, u64); 3] {
        self.state
            .borrow()
            .ingress
            .as_ref()
            .and_then(RcWeak::upgrade)
            .map_or([(0, 0); 3], |io| io.borrow().accounting.usage())
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

    /// Returns successful production-carrier checks performed before route mutation.
    pub fn carrier_checks(&self) -> u32 {
        self.state.borrow().carrier_checks
    }

    /// Returns binding preparation side effects observed by the protocol fixture.
    pub fn preparation_side_effects(&self) -> u32 {
        self.state.borrow().preparation_side_effects
    }

    pub fn lifecycle_starts(&self, phase: MockLifecyclePhase) -> u32 {
        self.state.borrow().lifecycle_starts[phase as usize]
    }

    pub fn lifecycle_cancellations(&self, phase: MockLifecyclePhase) -> u32 {
        self.state.borrow().lifecycle_cancellations[phase as usize]
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
    route_io: Option<RcWeak<RefCell<StaticRouteIo>>>,
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
            route_io: None,
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
            route_io: None,
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
        if input.artifact().identity() != artifact.identity()
            || artifact.route_reservation() != Some(input.route().reservation())
        {
            let error = artifact_input_error(*input.route());
            return Err(BindingInputRejection::new(input, error));
        }
        if let Some(probe) = &self.probe {
            let mut probe = probe.borrow_mut();
            probe.carrier_checks += 1;
            probe.lifecycle_starts[MockLifecyclePhase::Prepare as usize] += 1;
        }
        let io = Rc::new(RefCell::new(StaticRouteIo::new(*input.route())));
        self.route_io = Some(Rc::downgrade(&io));
        route.initialize(
            input,
            MockRouteState {
                phase: 0,
                target: Box::from(target),
                io,
            },
        );
        if let Some(probe) = &self.probe {
            probe.borrow_mut().ingress = self.route_io.clone();
        }
        if let Some(probe) = &self.probe {
            probe.borrow_mut().preparation_side_effects += 1;
        }
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
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_cancellations[MockLifecyclePhase::Prepare as usize] += 1;
        }
        Poll::Ready(Ok(cancelled()))
    }

    fn start_readiness(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteReadinessOutcome<()>> {
        readiness.initialize_state(MockReadinessState {
            remaining_polls: self
                .external_readiness_polls
                .max(u8::from(self.probe.is_some())),
        });
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_starts[MockLifecyclePhase::Readiness as usize] += 1;
        }
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
        if self.external_readiness_polls == 0 && self.probe.is_none() {
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
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_cancellations[MockLifecyclePhase::Readiness as usize] += 1;
        }
        Poll::Ready(Ok(cancelled()))
    }

    fn start_activate(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteActivationOutcome<(), ()>> {
        route.state_mut().phase = 2;
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_starts[MockLifecyclePhase::Activate as usize] += 1;
            clinkz_wot_core::StartStatus::Pending
        } else {
            clinkz_wot_core::StartStatus::Ready(RouteActivationOutcome::Active(()))
        }
    }

    fn poll_activate(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteActivationOutcome<(), ()>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
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
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_cancellations[MockLifecyclePhase::Activate as usize] += 1;
        }
        Poll::Ready(Ok(cancelled()))
    }

    fn start_commit(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCommitOutcome<(), ()>> {
        route.state_mut().phase = 3;
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_starts[MockLifecyclePhase::Commit as usize] += 1;
            clinkz_wot_core::StartStatus::Pending
        } else {
            clinkz_wot_core::StartStatus::Ready(RouteCommitOutcome::Committed(()))
        }
    }

    fn poll_commit(
        &mut self,
        _cx: &mut Context<'_>,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCommitOutcome<(), ()>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
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
        if let Some(probe) = &self.probe {
            probe.borrow_mut().lifecycle_cancellations[MockLifecyclePhase::Commit as usize] += 1;
        }
        Poll::Ready(Ok(cancelled()))
    }

    fn poll_accept(
        &mut self,
        _cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        permit: RouteActivationPermit<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        let Some(probe) = &self.probe else {
            return Poll::Pending;
        };
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        if probe.borrow().closed {
            return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Terminal(
                RouteTerminal::Closed {
                    route: *permit.route(),
                },
            )));
        }
        let Some(((name, input), correlation)) = route.state_mut().io.borrow_mut().accept() else {
            return Poll::Pending;
        };
        let mut state = probe.borrow_mut();
        state.in_flight = 1;
        state.in_flight_identity = Some((*permit.route(), correlation));
        state.last_accepted_correlation = Some(correlation);
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
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> clinkz_wot_core::StartStatus<RouteCleanupOutcome> {
        if let Some(probe) = &self.probe {
            route.state_mut().io.borrow_mut().close();
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
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        if let Some(probe) = &self.probe {
            route.state_mut().io.borrow_mut().close();
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
        route.state_mut().io.borrow_mut().close();
        state.routes = 0;
        state.cleanup = 0;
        state.closed = true;
        clinkz_wot_core::StartStatus::Ready(RouteCleanupOutcome::Complete)
    }

    fn poll_shutdown(
        &mut self,
        _cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        let Some(probe) = &self.probe else {
            return Poll::Ready(RouteCleanupOutcome::Complete);
        };
        if budget.consume(WorkClass::CleanupItems, 1).is_err() {
            return Poll::Pending;
        }
        route.state_mut().io.borrow_mut().close();
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
            let io = self
                .route_io
                .as_ref()
                .and_then(RcWeak::upgrade)
                .expect("response retains its caller-owned route state");
            let expected = {
                let io = io.borrow();
                io.in_flight.map(|correlation| (io.route, correlation))
            };
            let response = validate_live_response_identity(response, expected)?;
            let evidence = DeliveredResponseEvidence::from_response(&response);
            assert!(
                io.borrow_mut().settle(response.opportunity().correlation()),
                "response settles the admitted ingress correlation exactly once"
            );
            let mut state = probe.borrow_mut();
            assert_eq!(state.in_flight, 1);
            state.in_flight = 0;
            state.in_flight_identity = None;
            state.delivered += 1;
            state.delivered_validation_errors += u32::from(evidence.is_validation_failure());
            state.response_settlements += 1;
            assert!(
                state.delivered_response.replace(evidence).is_none(),
                "fixture response settled twice"
            );
            slot.initialize(response, MockResponseState { accepted: true });
        } else {
            slot.initialize(response, MockResponseState { accepted: true });
        }
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
/// used by the Servient runtime contract.
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
        fixture_ingress_policy(),
        BindingStatusPolicy::new(2, 128),
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
    use clinkz_wot_core::{PlanId, PlanSetGeneration, RouteResponseOpportunity};
    use clinkz_wot_foundation::{Generation, SlotIndex};

    #[test]
    fn static_response_delivery_rejects_stale_identity_and_preserves_the_response() {
        let compatibility = BindingArtifactCompatibility::new([0x41; 16]);
        let plan_id = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
        let plan_set_generation = PlanSetGeneration::new(Generation::INITIAL);
        let reservation = RouteReservationIdentity::new(
            CollisionDomainId::new([0x61; 16]),
            EndpointReservationKey::new([0x62; 32]),
        );
        let expected_route = BindingRouteKey::new(
            BindingId::new(7),
            BindingGeneration::INITIAL,
            Generation::INITIAL,
            plan_set_generation,
            plan_id,
            reservation,
        );
        let expected_correlation = CorrelationId::new(11);
        let next_generation = Generation::INITIAL.checked_next().expect("next generation");
        let stale_inputs = [
            (
                BindingRouteKey::new(
                    BindingId::new(7),
                    BindingGeneration::new(next_generation),
                    Generation::INITIAL,
                    plan_set_generation,
                    plan_id,
                    reservation,
                ),
                expected_correlation,
            ),
            (
                BindingRouteKey::new(
                    BindingId::new(7),
                    BindingGeneration::INITIAL,
                    next_generation,
                    plan_set_generation,
                    plan_id,
                    reservation,
                ),
                expected_correlation,
            ),
            (expected_route, CorrelationId::new(12)),
        ];

        for (stale_route, stale_correlation) in stale_inputs {
            let state = Rc::new(RefCell::new(StaticProbeState {
                in_flight: 1,
                in_flight_identity: Some((expected_route, expected_correlation)),
                ..StaticProbeState::default()
            }));
            let mut route_io = StaticRouteIo::new(expected_route);
            assert!(route_io.accounting.try_admit(1));
            route_io.retained_ingress_bytes = Some(1);
            route_io.in_flight = Some(expected_correlation);
            let route_io = Rc::new(RefCell::new(route_io));
            state.borrow_mut().ingress = Some(Rc::downgrade(&route_io));
            let mut binding =
                ManualMockBinding::with_probe(compatibility, Rc::clone(&state), false);
            binding.route_io = Some(Rc::downgrade(&route_io));
            let application_error =
                CoreError::Application(ErrorContext::new(ErrorPhase::Handler, RetryClass::Never));
            let response = RouteInboundResponse::failure(
                RouteResponseOpportunity::new(stale_route, stale_correlation),
                application_error.clone(),
            );
            let mut slot = ServerResponseSlot::new();
            let rejection = binding
                .start_response(response, &mut slot, &mut WorkBudget::new())
                .expect_err("stale response identity must be rejected");

            assert!(slot.is_vacant());
            assert_eq!(state.borrow().in_flight, 1);
            assert_eq!(
                state.borrow().in_flight_identity,
                Some((expected_route, expected_correlation))
            );
            assert_eq!(route_io.borrow().in_flight, Some(expected_correlation));
            assert_eq!(route_io.borrow().accounting.usage(), [(1, 1); 3]);
            let returned = rejection.into_input();
            assert_eq!(returned.opportunity().route(), &stale_route);
            assert_eq!(returned.opportunity().correlation(), stale_correlation);
            assert_eq!(returned.result(), Err(&application_error));
        }
    }
}

#[cfg(feature = "std")]
mod host_fixture {
    use core::pin::Pin;
    use core::sync::atomic::{AtomicU32, Ordering};
    use core::task::{Context, Poll};
    use std::{
        boxed::Box,
        sync::{
            Arc, Mutex, Weak,
            mpsc::{Receiver, SyncSender, sync_channel},
        },
    };

    use clinkz_wot_core::{
        AffordanceTarget, BindingArtifactCompatibility, BindingArtifactEnvelope,
        BindingCallSettlement, BindingCancellationDisposition, BindingConfigurationDigest,
        BindingDeliveryOutcome, BindingExecutionSupport, BindingGeneration, BindingId,
        BindingInputRejection, BindingLifetimeFootprint, BindingOperationalError,
        BindingRegistrationCapabilities, BindingRegistrationIdentity, BindingResourceDeclarations,
        BindingStatusPolicy, CleanupOperation, CleanupPhaseContext, CleanupSlotId, CoreError,
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

    use super::{
        DeliveredResponseEvidence, FIXTURE_INGRESS_BYTES, IngressAccounting, MOCK_PHASE_COUNT,
        MockArtifact, MockCompiler, MockLifecyclePhase, artifact_input_error,
        ingress_retained_bytes, validate_live_response_identity,
    };

    const HOST_CALL_FOOTPRINT: BindingLifetimeFootprint = BindingLifetimeFootprint::new(8, 1_024);

    /// Complete comparable projection of one retained cleanup phase.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct CleanupContextEvidence {
        subject: CleanupSlotId,
        lifetime: BindingLifetimeFootprint,
        durable_status_records: u32,
        work: [u64; 10],
        operation: CleanupOperation,
        first_cause: CoreError,
        deadline: Deadline,
    }

    impl CleanupContextEvidence {
        fn from_context(context: &CleanupPhaseContext) -> Self {
            Self {
                subject: context.reservation().subject(),
                lifetime: context.reservation().lifetime_footprint(),
                durable_status_records: context.reservation().durable_status_records(),
                work: core::array::from_fn(|index| {
                    context
                        .reservation()
                        .work()
                        .remaining(WorkClass::ALL[index])
                }),
                operation: context.operation(),
                first_cause: context.first_cause().clone(),
                deadline: context.deadline(),
            }
        }

        pub const fn operation(&self) -> CleanupOperation {
            self.operation
        }

        pub const fn subject(&self) -> CleanupSlotId {
            self.subject
        }
    }

    #[derive(Debug)]
    struct HostIngressItem {
        name: Box<str>,
        input: InteractionInput,
        retained_bytes: u64,
    }

    #[derive(Clone)]
    struct HostIngressSender {
        sender: SyncSender<HostIngressItem>,
        accounting: Weak<Mutex<IngressAccounting>>,
    }

    struct ProbeState {
        ingress: Option<HostIngressSender>,
        queued: u32,
        delivered: u32,
        delivered_validation_errors: u32,
        response_settlements: u32,
        routes: u32,
        in_flight: u32,
        last_accepted_correlation: Option<CorrelationId>,
        delivered_response: Option<DeliveredResponseEvidence>,
        cleanup: u32,
        aborts: u32,
        shutdowns: u32,
        reject_readiness_once: bool,
        reject_abort_once: bool,
        reject_shutdown_once: bool,
        readiness_rejections: u32,
        abort_rejections: u32,
        shutdown_rejections: u32,
        carrier_checks: u32,
        preparation_side_effects: u32,
        lifecycle_starts: [u32; MOCK_PHASE_COUNT],
        lifecycle_cancellations: [u32; MOCK_PHASE_COUNT],
        cleanup_context_started: [Option<CleanupContextEvidence>; MOCK_PHASE_COUNT],
        cleanup_context_settled: [Option<CleanupContextEvidence>; MOCK_PHASE_COUNT],
        closed: bool,
        prepared_target: Option<Box<str>>,
        prepared_state_address: Option<usize>,
        active_state_address: Option<usize>,
        committed_state_address: Option<usize>,
        prepared_footprint: Option<BindingLifetimeFootprint>,
        active_footprint: Option<BindingLifetimeFootprint>,
        committed_footprint: Option<BindingLifetimeFootprint>,
    }

    impl Default for ProbeState {
        fn default() -> Self {
            Self {
                ingress: None,
                queued: 0,
                delivered: 0,
                delivered_validation_errors: 0,
                response_settlements: 0,
                routes: 0,
                in_flight: 0,
                last_accepted_correlation: None,
                delivered_response: None,
                cleanup: 0,
                aborts: 0,
                shutdowns: 0,
                reject_readiness_once: false,
                reject_abort_once: false,
                reject_shutdown_once: false,
                readiness_rejections: 0,
                abort_rejections: 0,
                shutdown_rejections: 0,
                carrier_checks: 0,
                preparation_side_effects: 0,
                lifecycle_starts: [0; MOCK_PHASE_COUNT],
                lifecycle_cancellations: [0; MOCK_PHASE_COUNT],
                cleanup_context_started: core::array::from_fn(|_| None),
                cleanup_context_settled: core::array::from_fn(|_| None),
                closed: false,
                prepared_target: None,
                prepared_state_address: None,
                active_state_address: None,
                committed_state_address: None,
                prepared_footprint: None,
                active_footprint: None,
                committed_footprint: None,
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum HostRouteStage {
        Prepared,
        Active,
        CommittedClosed,
        Cleaning,
        Closed,
    }

    struct HostRouteIo {
        route: clinkz_wot_core::binding::BindingRouteKey,
        ingress: Receiver<HostIngressItem>,
        accounting: Arc<Mutex<IngressAccounting>>,
        retained_ingress_bytes: Option<u64>,
        next_correlation: u64,
        in_flight: Option<CorrelationId>,
        accepting: bool,
    }

    impl HostRouteIo {
        fn release_retained(&mut self) {
            if let Some(bytes) = self.retained_ingress_bytes.take() {
                self.accounting
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .release(bytes);
            }
        }

        fn drain_ingress(&mut self) {
            while let Ok(item) = self.ingress.try_recv() {
                self.accounting
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .release(item.retained_bytes);
            }
        }
    }

    struct HostMockRouteLifecycle {
        stage: HostRouteStage,
        cleanup: Option<CleanupPhaseContext>,
    }

    struct HostMockRouteState {
        lifecycle: Mutex<HostMockRouteLifecycle>,
        target: Box<str>,
        io: Arc<Mutex<HostRouteIo>>,
        drops: Arc<AtomicU32>,
    }

    impl HostMockRouteState {
        fn stage(&self) -> HostRouteStage {
            self.lifecycle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .stage
        }

        fn transition(&self, from: HostRouteStage, to: HostRouteStage) {
            let mut lifecycle = self
                .lifecycle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert_eq!(lifecycle.stage, from);
            lifecycle.stage = to;
        }

        fn begin_cleanup(&self, cleanup: CleanupPhaseContext) {
            let mut lifecycle = self
                .lifecycle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            lifecycle.stage = HostRouteStage::Cleaning;
            lifecycle.cleanup = Some(cleanup);
            self.io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .accepting = false;
        }

        fn finish_cleanup(&self) -> CleanupPhaseContext {
            let mut lifecycle = self
                .lifecycle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert_eq!(lifecycle.stage, HostRouteStage::Cleaning);
            let cleanup = lifecycle
                .cleanup
                .take()
                .expect("cleanup phase stays in route state");
            lifecycle.stage = HostRouteStage::Closed;
            cleanup
        }
    }

    impl Drop for HostMockRouteState {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn prepared_route_state(guard: &HostPreparedRouteGuard) -> Pin<&HostMockRouteState> {
        guard
            .try_state_pin_ref::<HostMockRouteState>()
            .expect("prepared guard retains the mock route state")
    }

    fn active_route_state(guard: &HostActiveRouteGuard) -> Pin<&HostMockRouteState> {
        guard
            .try_state_pin_ref::<HostMockRouteState>()
            .expect("active guard retains the mock route state")
    }

    fn committed_route_state(guard: &HostCommittedRouteGuard) -> Pin<&HostMockRouteState> {
        guard
            .try_state_pin_ref::<HostMockRouteState>()
            .expect("committed guard retains the mock route state")
    }

    /// Deterministic protocol-I/O and instrumentation state for the Servient
    /// host runner. It creates no plan, route, permit, handler, or response.
    #[derive(Clone)]
    pub struct HostPropertyReadProbe {
        state: WotLock<ProbeState>,
        artifact_drops: Arc<AtomicU32>,
        route_state_drops: Arc<AtomicU32>,
    }

    impl HostPropertyReadProbe {
        pub fn enqueue_property_read(&self, name: &str, input: InteractionInput) {
            let ingress = self.state.with_read(|state| {
                assert!(!state.closed, "request queued after route closure");
                state
                    .ingress
                    .clone()
                    .expect("route-owned ingress receiver is not prepared")
            });
            let retained_bytes = ingress_retained_bytes(name, &input)
                .filter(|bytes| *bytes <= FIXTURE_INGRESS_BYTES)
                .expect("mock ingress item exceeds its admitted byte capacity");
            let accounting = ingress
                .accounting
                .upgrade()
                .expect("route-owned ingress accounting is not prepared");
            assert!(
                accounting
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .try_admit(retained_bytes),
                "mock ingress capacity is exhausted"
            );
            if let Err(error) = ingress.sender.try_send(HostIngressItem {
                name: Box::from(name),
                input,
                retained_bytes,
            }) {
                accounting
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .release(retained_bytes);
                panic!("mock ingress slot is occupied: {error:?}");
            }
            self.state.with(|state| {
                state.queued += 1;
            });
        }

        pub fn delivered_responses(&self) -> u32 {
            self.state.with_read(|state| state.delivered)
        }

        /// Returns Core-sealed validation failures accepted for delivery.
        pub fn delivered_validation_errors(&self) -> u32 {
            self.state
                .with_read(|state| state.delivered_validation_errors)
        }

        /// Returns terminal response-delivery settlements.
        pub fn response_settlements(&self) -> u32 {
            self.state.with_read(|state| state.response_settlements)
        }

        /// Returns the correlation allocated for the accepted request.
        pub fn last_accepted_correlation(&self) -> Option<CorrelationId> {
            self.state
                .with_read(|state| state.last_accepted_correlation)
        }

        /// Returns the protocol-edge observation of the delivered response.
        pub fn delivered_response(&self) -> Option<DeliveredResponseEvidence> {
            self.state
                .with_read(|state| state.delivered_response.clone())
        }

        pub fn outstanding_counts(&self) -> (u32, u32, u32, u32) {
            self.state
                .with_read(|state| (state.routes, state.queued, state.in_flight, state.cleanup))
        }

        pub fn ingress_usage(&self) -> [(u32, u64); 3] {
            let accounting = self.state.with_read(|state| {
                state
                    .ingress
                    .as_ref()
                    .and_then(|ingress| ingress.accounting.upgrade())
            });
            accounting.map_or([(0, 0); 3], |accounting| {
                accounting
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .usage()
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

        pub fn route_state_drops(&self) -> u32 {
            self.route_state_drops.load(Ordering::SeqCst)
        }

        /// Returns successful production-carrier checks performed before route mutation.
        pub fn carrier_checks(&self) -> u32 {
            self.state.with_read(|state| state.carrier_checks)
        }

        /// Returns binding preparation side effects observed by the protocol fixture.
        pub fn preparation_side_effects(&self) -> u32 {
            self.state.with_read(|state| state.preparation_side_effects)
        }

        pub fn lifecycle_starts(&self, phase: MockLifecyclePhase) -> u32 {
            self.state
                .with_read(|state| state.lifecycle_starts[phase as usize])
        }

        pub fn lifecycle_cancellations(&self, phase: MockLifecyclePhase) -> u32 {
            self.state
                .with_read(|state| state.lifecycle_cancellations[phase as usize])
        }

        pub fn cleanup_context_evidence(
            &self,
            phase: MockLifecyclePhase,
        ) -> (
            Option<CleanupContextEvidence>,
            Option<CleanupContextEvidence>,
        ) {
            self.state.with_read(|state| {
                (
                    state.cleanup_context_started[phase as usize].clone(),
                    state.cleanup_context_settled[phase as usize].clone(),
                )
            })
        }

        pub fn carrier_evidence(
            &self,
        ) -> (
            Option<usize>,
            Option<usize>,
            Option<usize>,
            Option<BindingLifetimeFootprint>,
            Option<BindingLifetimeFootprint>,
            Option<BindingLifetimeFootprint>,
        ) {
            self.state.with_read(|state| {
                (
                    state.prepared_state_address,
                    state.active_state_address,
                    state.committed_state_address,
                    state.prepared_footprint,
                    state.active_footprint,
                    state.committed_footprint,
                )
            })
        }
    }

    fn record_cleanup_context_start(
        probe: &WotLock<ProbeState>,
        phase: MockLifecyclePhase,
        context: &CleanupPhaseContext,
    ) {
        let evidence = CleanupContextEvidence::from_context(context);
        probe.with(|state| {
            assert!(
                state.cleanup_context_started[phase as usize]
                    .replace(evidence)
                    .is_none(),
                "cleanup context started twice"
            );
        });
    }

    fn record_cleanup_context_settlement(
        probe: &WotLock<ProbeState>,
        phase: MockLifecyclePhase,
        context: &CleanupPhaseContext,
    ) {
        let evidence = CleanupContextEvidence::from_context(context);
        probe.with(|state| {
            assert!(
                state.cleanup_context_settled[phase as usize]
                    .replace(evidence)
                    .is_none(),
                "cleanup context settled twice"
            );
        });
    }

    struct PrepareCall {
        input: Option<PrepareInput>,
        target: Option<Box<str>>,
        probe: WotLock<ProbeState>,
        response_io: Arc<Mutex<Option<Weak<Mutex<HostRouteIo>>>>>,
        route_state_drops: Arc<AtomicU32>,
        pending_once: bool,
        started: bool,
        cancellation: Option<CleanupPhaseContext>,
    }

    impl HostBindingCall<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>
        for PrepareCall
    {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            HOST_CALL_FOOTPRINT
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RoutePrepareOutcome<HostPreparedRouteGuard>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            if !self.started {
                self.started = true;
                self.probe.with(|state| {
                    state.carrier_checks += 1;
                    state.lifecycle_starts[MockLifecyclePhase::Prepare as usize] += 1;
                });
            }
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let input = self.input.take().expect("prepare call completed twice");
            let target = self.target.take().expect("prepare target completed twice");
            let route = *input.route();
            let footprint = BindingLifetimeFootprint::new(2, 128);
            self.probe.with(|state| {
                state.preparation_side_effects += 1;
            });
            let (ingress, receiver) = sync_channel(1);
            let accounting = Arc::new(Mutex::new(IngressAccounting::new(
                super::fixture_ingress_policy(),
            )));
            let io = Arc::new(Mutex::new(HostRouteIo {
                route,
                ingress: receiver,
                accounting: Arc::clone(&accounting),
                retained_ingress_bytes: None,
                next_correlation: 1,
                in_flight: None,
                accepting: true,
            }));
            *self
                .response_io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::downgrade(&io));
            let guard = HostPreparedRouteGuard::new(
                input,
                footprint,
                HostMockRouteState {
                    lifecycle: Mutex::new(HostMockRouteLifecycle {
                        stage: HostRouteStage::Prepared,
                        cleanup: None,
                    }),
                    target,
                    io,
                    drops: Arc::clone(&self.route_state_drops),
                },
            );
            let state = prepared_route_state(&guard);
            let state_address = state.get_ref() as *const HostMockRouteState as usize;
            let prepared_target = state.get_ref().target.clone();
            self.probe.with(|state| {
                assert_eq!(state.routes, 0);
                state.ingress = Some(HostIngressSender {
                    sender: ingress,
                    accounting: Arc::downgrade(&accounting),
                });
                state.prepared_target = Some(prepared_target);
                state.prepared_state_address = Some(state_address);
                state.prepared_footprint = Some(footprint);
                state.routes = 1;
            });
            Poll::Ready(RoutePrepareOutcome::Prepared(guard))
        }

        fn start_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<
                BindingCallSettlement<
                    RoutePrepareOutcome<HostPreparedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            assert!(
                self.cancellation.is_none(),
                "prepare cancellation started twice"
            );
            record_cleanup_context_start(&self.probe, MockLifecyclePhase::Prepare, &cleanup);
            self.cancellation = Some(cleanup);
            self.probe.with(|state| {
                state.lifecycle_cancellations[MockLifecyclePhase::Prepare as usize] += 1
            });
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
            let cleanup = self
                .cancellation
                .take()
                .expect("prepare cancellation lost its cleanup context");
            record_cleanup_context_settlement(&self.probe, MockLifecyclePhase::Prepare, &cleanup);
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

    struct ReadinessCall {
        guard: Option<HostPreparedRouteGuard>,
        pending_once: bool,
        cancel_pending_once: bool,
        started: bool,
        probe: WotLock<ProbeState>,
        cancellation: Option<CleanupPhaseContext>,
    }

    impl HostBindingCall<RouteReadinessOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>
        for ReadinessCall
    {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            HOST_CALL_FOOTPRINT
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RouteReadinessOutcome<HostPreparedRouteGuard>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            if !self.started {
                self.started = true;
                self.probe.with(|state| {
                    state.lifecycle_starts[MockLifecyclePhase::Readiness as usize] += 1
                });
            }
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready(RouteReadinessOutcome::Ready(
                self.guard.take().expect("readiness completed twice"),
            ))
        }

        fn start_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<
                BindingCallSettlement<
                    RouteReadinessOutcome<HostPreparedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            assert!(
                self.cancellation.is_none(),
                "readiness cancellation started twice"
            );
            record_cleanup_context_start(&self.probe, MockLifecyclePhase::Readiness, &cleanup);
            self.cancellation = Some(cleanup);
            self.probe.with(|state| {
                state.lifecycle_cancellations[MockLifecyclePhase::Readiness as usize] += 1
            });
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<
            clinkz_wot_core::CoreResult<
                BindingCallSettlement<
                    RouteReadinessOutcome<HostPreparedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            if self.cancel_pending_once {
                self.cancel_pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let cleanup = self
                .cancellation
                .take()
                .expect("readiness cancellation lost its cleanup context");
            record_cleanup_context_settlement(&self.probe, MockLifecyclePhase::Readiness, &cleanup);
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: HostRouteCleanupSuccessor::AbortPrepared(
                        self.guard.take().expect("readiness cancelled twice"),
                    ),
                },
            }))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct ActivationCall {
        guard: Option<HostPreparedRouteGuard>,
        footprint: BindingLifetimeFootprint,
        pending_once: bool,
        cancel_pending_once: bool,
        started: bool,
        probe: WotLock<ProbeState>,
        cancellation: Option<CleanupPhaseContext>,
    }

    impl
        HostBindingCall<
            RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
            HostRouteCleanupSuccessor,
        > for ActivationCall
    {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            self.footprint
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            if !self.started {
                self.started = true;
                self.probe.with(|state| {
                    state.lifecycle_starts[MockLifecyclePhase::Activate as usize] += 1
                });
            }
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let guard = self.guard.take().expect("activation completed twice");
            let footprint = guard.lifetime_footprint();
            let state = prepared_route_state(&guard);
            state
                .get_ref()
                .transition(HostRouteStage::Prepared, HostRouteStage::Active);
            let address = state.get_ref() as *const HostMockRouteState as usize;
            self.probe.with(|probe| {
                probe.active_state_address = Some(address);
                probe.active_footprint = Some(footprint);
            });
            Poll::Ready(RouteActivationOutcome::Active(HostActiveRouteGuard::new(
                guard,
            )))
        }

        fn start_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<
                BindingCallSettlement<
                    RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            assert!(
                self.cancellation.is_none(),
                "activation cancellation started twice"
            );
            record_cleanup_context_start(&self.probe, MockLifecyclePhase::Activate, &cleanup);
            self.cancellation = Some(cleanup);
            self.probe.with(|state| {
                state.lifecycle_cancellations[MockLifecyclePhase::Activate as usize] += 1
            });
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<
            clinkz_wot_core::CoreResult<
                BindingCallSettlement<
                    RouteActivationOutcome<HostPreparedRouteGuard, HostActiveRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            if self.cancel_pending_once {
                self.cancel_pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let cleanup = self
                .cancellation
                .take()
                .expect("activation cancellation lost its cleanup context");
            record_cleanup_context_settlement(&self.probe, MockLifecyclePhase::Activate, &cleanup);
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: HostRouteCleanupSuccessor::AbortPrepared(
                        self.guard.take().expect("activation cancelled twice"),
                    ),
                },
            }))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct CommitCall {
        guard: Option<HostActiveRouteGuard>,
        pending_once: bool,
        cancel_pending_once: bool,
        started: bool,
        probe: WotLock<ProbeState>,
        cancellation: Option<CleanupPhaseContext>,
    }

    impl
        HostBindingCall<
            RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
            HostRouteCleanupSuccessor,
        > for CommitCall
    {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            HOST_CALL_FOOTPRINT
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            if !self.started {
                self.started = true;
                self.probe
                    .with(|state| state.lifecycle_starts[MockLifecyclePhase::Commit as usize] += 1);
            }
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let guard = self.guard.take().expect("commit completed twice");
            let footprint = guard.lifetime_footprint();
            let state = active_route_state(&guard);
            state
                .get_ref()
                .transition(HostRouteStage::Active, HostRouteStage::CommittedClosed);
            let address = state.get_ref() as *const HostMockRouteState as usize;
            self.probe.with(|probe| {
                probe.committed_state_address = Some(address);
                probe.committed_footprint = Some(footprint);
            });
            Poll::Ready(RouteCommitOutcome::Committed(HostCommittedRouteGuard::new(
                guard,
            )))
        }

        fn start_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<
                BindingCallSettlement<
                    RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            assert!(
                self.cancellation.is_none(),
                "commit cancellation started twice"
            );
            record_cleanup_context_start(&self.probe, MockLifecyclePhase::Commit, &cleanup);
            self.cancellation = Some(cleanup);
            self.probe.with(|state| {
                state.lifecycle_cancellations[MockLifecyclePhase::Commit as usize] += 1
            });
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<
            clinkz_wot_core::CoreResult<
                BindingCallSettlement<
                    RouteCommitOutcome<HostActiveRouteGuard, HostCommittedRouteGuard>,
                    HostRouteCleanupSuccessor,
                >,
            >,
        > {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            if self.cancel_pending_once {
                self.cancel_pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let cleanup = self
                .cancellation
                .take()
                .expect("commit cancellation lost its cleanup context");
            record_cleanup_context_settlement(&self.probe, MockLifecyclePhase::Commit, &cleanup);
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: HostRouteCleanupSuccessor::ShutdownActive(
                        self.guard.take().expect("commit cancelled twice"),
                    ),
                },
            }))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct DeliveryCall {
        response: Option<RouteInboundResponse>,
        probe: WotLock<ProbeState>,
        response_io: Weak<Mutex<HostRouteIo>>,
        pending_once: bool,
        cancel_pending_once: bool,
        started: bool,
        cancellation: Option<CleanupPhaseContext>,
    }

    impl HostBindingCall<BindingDeliveryOutcome, NoCleanupSuccessor> for DeliveryCall {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            HOST_CALL_FOOTPRINT
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<BindingDeliveryOutcome> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            if !self.started {
                self.started = true;
                self.probe.with(|state| {
                    state.lifecycle_starts[MockLifecyclePhase::ResponseDelivery as usize] += 1
                });
            }
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let response = self.response.take().expect("response delivered twice");
            let io = self
                .response_io
                .upgrade()
                .expect("delivery retains live route I/O");
            let mut io = io.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let expected = io.in_flight.map(|correlation| (io.route, correlation));
            let response = match validate_live_response_identity(response, expected) {
                Ok(response) => response,
                Err(rejection) => {
                    let (_, error) = rejection.into_parts();
                    return Poll::Ready(BindingDeliveryOutcome::Failed(error));
                }
            };
            io.in_flight = None;
            io.release_retained();
            drop(io);
            let evidence = DeliveredResponseEvidence::from_response(&response);
            self.probe.with(|state| {
                assert_eq!(state.in_flight, 1);
                state.in_flight = 0;
                state.delivered += 1;
                state.delivered_validation_errors += u32::from(evidence.is_validation_failure());
                state.response_settlements += 1;
                assert!(
                    state.delivered_response.replace(evidence).is_none(),
                    "fixture response settled twice"
                );
            });
            Poll::Ready(BindingDeliveryOutcome::Delivered)
        }

        fn start_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<StartStatus<BindingCallSettlement<BindingDeliveryOutcome>>>
        {
            assert!(
                self.cancellation.is_none(),
                "delivery cancellation started twice"
            );
            record_cleanup_context_start(
                &self.probe,
                MockLifecyclePhase::ResponseDelivery,
                &cleanup,
            );
            self.cancellation = Some(cleanup);
            self.probe.with(|state| {
                state.lifecycle_cancellations[MockLifecyclePhase::ResponseDelivery as usize] += 1
            });
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<clinkz_wot_core::CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>>
        {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            if self.cancel_pending_once {
                self.cancel_pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let cleanup = self
                .cancellation
                .take()
                .expect("delivery cancellation lost its cleanup context");
            record_cleanup_context_settlement(
                &self.probe,
                MockLifecyclePhase::ResponseDelivery,
                &cleanup,
            );
            let response = self.response.take().expect("response cancelled twice");
            let io = self
                .response_io
                .upgrade()
                .expect("cancelled delivery retains live route I/O");
            let mut io = io.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let expected = io.in_flight.map(|correlation| (io.route, correlation));
            let response = match validate_live_response_identity(response, expected) {
                Ok(response) => response,
                Err(rejection) => {
                    self.response = Some(rejection.into_input());
                    return Poll::Ready(Err(CoreError::Binding(ErrorContext::new(
                        ErrorPhase::Delivery,
                        RetryClass::Never,
                    ))));
                }
            };
            io.in_flight = None;
            io.release_retained();
            drop(io);
            drop(response);
            self.probe.with(|state| {
                state.in_flight = 0;
                state.response_settlements += 1;
            });
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: NoCleanupSuccessor,
                },
            }))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    enum CleanupGuard {
        Prepared(HostPreparedRouteGuard),
        Shutdown(clinkz_wot_core::HostShutdownRouteGuard),
    }

    impl CleanupGuard {
        fn state(&self) -> Pin<&HostMockRouteState> {
            match self {
                Self::Prepared(guard) => prepared_route_state(guard),
                Self::Shutdown(clinkz_wot_core::HostShutdownRouteGuard::Active(guard)) => {
                    active_route_state(guard)
                }
                Self::Shutdown(clinkz_wot_core::HostShutdownRouteGuard::Committed(guard)) => {
                    committed_route_state(guard)
                }
            }
        }
    }

    enum CleanupInput {
        Abort {
            guard: HostPreparedRouteGuard,
            cleanup: CleanupPhaseContext,
        },
        Shutdown {
            guard: clinkz_wot_core::HostShutdownRouteGuard,
            cleanup: CleanupPhaseContext,
        },
    }

    struct CleanupCall {
        input: Option<CleanupInput>,
        guard: Option<CleanupGuard>,
        route: clinkz_wot_core::binding::BindingRouteKey,
        phase: MockLifecyclePhase,
        probe: WotLock<ProbeState>,
        response_io: Arc<Mutex<Option<Weak<Mutex<HostRouteIo>>>>>,
        route_state_drops: Arc<AtomicU32>,
        footprint: BindingLifetimeFootprint,
        pending_once: bool,
        cancel_pending_once: bool,
        started: bool,
        cancellation: Option<CleanupPhaseContext>,
    }

    impl CleanupCall {
        fn abort(
            input: RouteAbortInput,
            probe: WotLock<ProbeState>,
            response_io: Arc<Mutex<Option<Weak<Mutex<HostRouteIo>>>>>,
            route_state_drops: Arc<AtomicU32>,
            footprint: BindingLifetimeFootprint,
        ) -> Self {
            let (guard, cleanup) = input.into_parts();
            let route = *guard.route();
            Self {
                input: Some(CleanupInput::Abort { guard, cleanup }),
                guard: None,
                route,
                phase: MockLifecyclePhase::Abort,
                probe,
                response_io,
                route_state_drops,
                footprint,
                pending_once: true,
                cancel_pending_once: true,
                started: false,
                cancellation: None,
            }
        }

        fn shutdown(
            input: RouteShutdownInput,
            probe: WotLock<ProbeState>,
            response_io: Arc<Mutex<Option<Weak<Mutex<HostRouteIo>>>>>,
            route_state_drops: Arc<AtomicU32>,
            footprint: BindingLifetimeFootprint,
        ) -> Self {
            let (guard, cleanup) = input.into_parts();
            let route = match &guard {
                clinkz_wot_core::HostShutdownRouteGuard::Active(guard) => *guard.route(),
                clinkz_wot_core::HostShutdownRouteGuard::Committed(guard) => *guard.route(),
            };
            Self {
                input: Some(CleanupInput::Shutdown { guard, cleanup }),
                guard: None,
                route,
                phase: MockLifecyclePhase::Shutdown,
                probe,
                response_io,
                route_state_drops,
                footprint,
                pending_once: true,
                cancel_pending_once: true,
                started: false,
                cancellation: None,
            }
        }

        fn begin_if_needed(&mut self) {
            if self.started {
                return;
            }
            let (guard, cleanup) = match self.input.take().expect("cleanup input consumed twice") {
                CleanupInput::Abort { guard, cleanup } => (CleanupGuard::Prepared(guard), cleanup),
                CleanupInput::Shutdown { guard, cleanup } => {
                    (CleanupGuard::Shutdown(guard), cleanup)
                }
            };
            record_cleanup_context_start(&self.probe, self.phase, &cleanup);
            guard.state().get_ref().begin_cleanup(cleanup);
            self.guard = Some(guard);
            self.started = true;
            self.probe.with(|state| {
                state.lifecycle_starts[self.phase as usize] += 1;
                state.cleanup = 1;
            });
        }

        fn complete(&mut self) -> RouteCleanupOutcome {
            let guard = self.guard.take().expect("cleanup completed twice");
            let state = guard.state();
            assert_eq!(state.get_ref().stage(), HostRouteStage::Cleaning);
            let mut io = state
                .get_ref()
                .io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            io.accepting = false;
            io.in_flight = None;
            io.release_retained();
            io.drain_ingress();
            drop(io);
            let cleanup = state.get_ref().finish_cleanup();
            record_cleanup_context_settlement(&self.probe, self.phase, &cleanup);
            let phase = self.phase;
            self.probe.with(|state| {
                match phase {
                    MockLifecyclePhase::Abort => state.aborts += 1,
                    MockLifecyclePhase::Shutdown => state.shutdowns += 1,
                    _ => unreachable!("cleanup call has abort or shutdown phase"),
                }
                state.ingress = None;
                state.routes = 0;
                state.queued = 0;
                state.in_flight = 0;
                state.cleanup = 0;
                state.closed = true;
            });
            *self
                .response_io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            drop(guard);
            assert_eq!(
                self.route_state_drops.load(Ordering::SeqCst),
                1,
                "terminal cleanup drops the one route state exactly once"
            );
            RouteCleanupOutcome::Complete
        }
    }

    impl HostBindingCall<RouteCleanupOutcome, HostRouteCleanupSuccessor> for CleanupCall {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            self.footprint
        }

        fn poll_result(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<RouteCleanupOutcome> {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            assert_eq!(
                self.route_state_drops.load(Ordering::SeqCst),
                0,
                "route state dropped before terminal cleanup"
            );
            self.begin_if_needed();
            if self.pending_once {
                self.pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready(self.complete())
        }

        fn start_cancel(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            cleanup: CleanupPhaseContext,
            _budget: &mut WorkBudget,
        ) -> clinkz_wot_core::CoreResult<
            StartStatus<BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>>,
        > {
            assert!(
                self.cancellation.is_none(),
                "cleanup-call cancellation started twice"
            );
            record_cleanup_context_start(
                &self.probe,
                MockLifecyclePhase::CleanupCallCancellation,
                &cleanup,
            );
            self.cancellation = Some(cleanup);
            self.probe.with(|state| {
                state.lifecycle_cancellations
                    [MockLifecyclePhase::CleanupCallCancellation as usize] += 1
            });
            Ok(StartStatus::Pending)
        }

        fn poll_cancel(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<
            clinkz_wot_core::CoreResult<
                BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
            >,
        > {
            if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                return Poll::Pending;
            }
            self.begin_if_needed();
            if self.cancel_pending_once {
                self.cancel_pending_once = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let cancellation = self
                .cancellation
                .take()
                .expect("cleanup-call cancellation lost its cleanup context");
            record_cleanup_context_settlement(
                &self.probe,
                MockLifecyclePhase::CleanupCallCancellation,
                &cancellation,
            );
            let route = self.route;
            let _ = self.complete();
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: HostRouteCleanupSuccessor::NoRouteResource { route },
                },
            }))
        }

        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
    }

    struct HostMockBinding {
        compatibility: BindingArtifactCompatibility,
        probe: WotLock<ProbeState>,
        response_io: Arc<Mutex<Option<Weak<Mutex<HostRouteIo>>>>>,
        route_state_drops: Arc<AtomicU32>,
        oversized_activation: bool,
        oversized_cleanup: bool,
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
            if input.artifact().identity() != artifact.identity()
                || artifact.route_reservation() != Some(input.route().reservation())
            {
                let error = artifact_input_error(*input.route());
                return Err(BindingInputRejection::new(input, error));
            }
            Ok(HostBindingCallBox::new(PrepareCall {
                input: Some(input),
                target: Some(Box::from(target)),
                probe: self.probe.clone(),
                response_io: Arc::clone(&self.response_io),
                route_state_drops: Arc::clone(&self.route_state_drops),
                pending_once: true,
                started: false,
                cancellation: None,
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
                assert_eq!(self.route_state_drops.load(Ordering::SeqCst), 0);
                assert_eq!(
                    prepared_route_state(&guard).get_ref().stage(),
                    HostRouteStage::Prepared
                );
                let error = BindingOperationalError::for_route(
                    *guard.route(),
                    CoreError::Binding(ErrorContext::new(ErrorPhase::Readiness, RetryClass::Never)),
                );
                return Err(BindingInputRejection::new(guard, error));
            }
            Ok(HostBindingCallBox::new(ReadinessCall {
                guard: Some(guard),
                pending_once: true,
                cancel_pending_once: true,
                started: false,
                probe: self.probe.clone(),
                cancellation: None,
            }))
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
            Ok(HostBindingCallBox::new(ActivationCall {
                guard: Some(guard),
                footprint: if self.oversized_activation {
                    BindingLifetimeFootprint::new(9, 1_024)
                } else {
                    HOST_CALL_FOOTPRINT
                },
                pending_once: true,
                cancel_pending_once: true,
                started: false,
                probe: self.probe.clone(),
                cancellation: None,
            }))
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
            Ok(HostBindingCallBox::new(CommitCall {
                guard: Some(guard),
                pending_once: true,
                cancel_pending_once: true,
                started: false,
                probe: self.probe.clone(),
                cancellation: None,
            }))
        }

        fn poll_accept(
            &self,
            route: &HostCommittedRouteGuard,
            permit: clinkz_wot_core::RouteActivationPermit<'_>,
            _cx: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<clinkz_wot_core::CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            let state = committed_route_state(route);
            if state.get_ref().stage() == HostRouteStage::Closed {
                return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Terminal(
                    RouteTerminal::Closed {
                        route: *permit.route(),
                    },
                )));
            }
            if state.get_ref().stage() != HostRouteStage::CommittedClosed {
                return Poll::Ready(Err(CoreError::Binding(ErrorContext::new(
                    ErrorPhase::Binding,
                    RetryClass::Never,
                ))));
            }
            let mut io = state
                .get_ref()
                .io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !io.accepting || permit.route() != &io.route {
                return Poll::Ready(Err(CoreError::Binding(ErrorContext::new(
                    ErrorPhase::Binding,
                    RetryClass::Never,
                ))));
            }
            let Ok(item) = io.ingress.try_recv() else {
                return Poll::Pending;
            };
            if io.in_flight.is_some()
                || io.retained_ingress_bytes.is_some()
                || io.next_correlation == 0
                || item.retained_bytes > FIXTURE_INGRESS_BYTES
            {
                return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::OperationalError(
                    BindingOperationalError::for_route(
                        io.route,
                        CoreError::Binding(ErrorContext::new(
                            ErrorPhase::Binding,
                            RetryClass::Never,
                        )),
                    ),
                )));
            }
            let correlation = CorrelationId::new(io.next_correlation);
            io.next_correlation = io.next_correlation.checked_add(1).unwrap_or(0);
            io.in_flight = Some(correlation);
            io.retained_ingress_bytes = Some(item.retained_bytes);
            drop(io);
            self.probe.with(|probe| {
                probe.queued -= 1;
                probe.in_flight = 1;
                probe.last_accepted_correlation = Some(correlation);
            });
            Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Request(
                RouteInboundRequest::new(
                    *permit.route(),
                    correlation,
                    AffordanceTarget::Property(Arc::from(item.name)),
                    item.input,
                ),
            )))
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
                assert_eq!(self.route_state_drops.load(Ordering::SeqCst), 0);
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
            Ok(HostBindingCallBox::new(CleanupCall::abort(
                input,
                self.probe.clone(),
                Arc::clone(&self.response_io),
                Arc::clone(&self.route_state_drops),
                if self.oversized_cleanup {
                    BindingLifetimeFootprint::new(9, 1_024)
                } else {
                    HOST_CALL_FOOTPRINT
                },
            )))
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
                assert_eq!(self.route_state_drops.load(Ordering::SeqCst), 0);
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
            Ok(HostBindingCallBox::new(CleanupCall::shutdown(
                input,
                self.probe.clone(),
                Arc::clone(&self.response_io),
                Arc::clone(&self.route_state_drops),
                if self.oversized_cleanup {
                    BindingLifetimeFootprint::new(9, 1_024)
                } else {
                    HOST_CALL_FOOTPRINT
                },
            )))
        }

        fn deliver_response(
            &self,
            response: RouteInboundResponse,
        ) -> Result<
            HostBindingCallBox<BindingDeliveryOutcome>,
            BindingInputRejection<RouteInboundResponse>,
        > {
            let route = *response.opportunity().route();
            let io = self
                .response_io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let Some(io) = io else {
                return Err(BindingInputRejection::new(
                    response,
                    BindingOperationalError::for_route(
                        route,
                        CoreError::Binding(ErrorContext::new(
                            ErrorPhase::Delivery,
                            RetryClass::Never,
                        )),
                    ),
                ));
            };
            Ok(HostBindingCallBox::new(DeliveryCall {
                response: Some(response),
                probe: self.probe.clone(),
                response_io: io,
                pending_once: true,
                cancel_pending_once: true,
                started: false,
                cancellation: None,
            }))
        }
    }

    /// Builds the complete host-erased mock registration and its independent
    /// deterministic I/O probe.
    pub fn host_property_read_fixture() -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(false, false, false, false, false)
    }

    /// Rejects readiness and the first abort-constructor attempt while
    /// returning both complete inputs to the Servient cleanup owner.
    pub fn host_property_read_readiness_rejection_fixture()
    -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(true, true, false, false, false)
    }

    /// Rejects the first shutdown-constructor attempt while returning the
    /// complete committed guard and cleanup phase for a later retry.
    pub fn host_property_read_shutdown_rejection_fixture()
    -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(false, false, true, false, false)
    }

    /// Returns an activation call whose truthful footprint exceeds registration admission.
    pub fn host_property_read_oversized_activation_fixture()
    -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(false, false, false, true, false)
    }

    /// Returns an oversized shutdown call that must settle through cancellation.
    pub fn host_property_read_oversized_shutdown_fixture()
    -> (HostBindingRegistration, HostPropertyReadProbe) {
        host_property_read_fixture_with_rejections(false, false, false, false, true)
    }

    fn host_property_read_fixture_with_rejections(
        reject_readiness_once: bool,
        reject_abort_once: bool,
        reject_shutdown_once: bool,
        oversized_activation: bool,
        oversized_cleanup: bool,
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
        let route_state_drops = Arc::new(AtomicU32::new(0));
        let response_io = Arc::new(Mutex::new(None));
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
                response_io,
                route_state_drops: Arc::clone(&route_state_drops),
                oversized_activation,
                oversized_cleanup,
            }),
            BindingResourceDeclarations::new(HOST_CALL_FOOTPRINT, HOST_CALL_FOOTPRINT),
            super::fixture_ingress_policy(),
            BindingStatusPolicy::new(2, 128),
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
                route_state_drops,
            },
        )
    }
}

#[cfg(feature = "std")]
pub use host_fixture::{
    CleanupContextEvidence, HostPropertyReadProbe, host_property_read_fixture,
    host_property_read_oversized_activation_fixture, host_property_read_oversized_shutdown_fixture,
    host_property_read_readiness_rejection_fixture, host_property_read_shutdown_rejection_fixture,
};
