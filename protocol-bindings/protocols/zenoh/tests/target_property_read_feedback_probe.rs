#![cfg(feature = "zenoh")]

use std::{
    future::Future,
    net::TcpListener,
    pin::Pin,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

use clinkz_wot_core::{
    AffordanceTarget, BindingArtifact, BindingArtifactCompatibility, BindingArtifactEnvelope,
    BindingArtifactFootprint, BindingArtifactRole, BindingCallSettlement,
    BindingCancellationDisposition, BindingCompilerBounds, BindingCompilerExtension,
    BindingCompilerInput, BindingCompilerOutput, BindingCompilerStep, BindingConfigurationDigest,
    BindingDeliveryOutcome, BindingExecutionSupport, BindingGeneration, BindingId,
    BindingIngressLimits, BindingIngressPolicy, BindingInputRejection, BindingLifetimeFootprint,
    BindingOperationalError, BindingRegistrationCapabilities, BindingRegistrationIdentity,
    BindingResourceDeclarations, BindingStateLayout, BindingStatusPolicy,
    BindingTransientFootprint, CleanupHandle, CleanupPhaseContext, CleanupRecord,
    CollisionDomainId, CoreError, CoreResult, CorrelationId, Deadline, EndpointReservationKey,
    ErrorContext, ErrorPhase, HandlerContext, HandlerFootprint, HandlerSlotId,
    HostActiveRouteGuard, HostBindingArtifact, HostBindingCall, HostBindingCallBox,
    HostBindingCompilerRegistration, HostBindingRegistration, HostBindingRegistrationInput,
    HostCommittedRouteGuard, HostPreparedRouteGuard, HostRouteCleanupSuccessor,
    HostShutdownRouteGuard, InteractionInput, InteractionOutput, NoCleanupSuccessor, Payload,
    PollServerBinding, PrepareInput, ReadPropertyHandler, RetryClass, RouteAbortInput,
    RouteActivationOutcome, RouteActivationPermit, RouteCleanupOutcome, RouteCommitOutcome,
    RouteInboundRequest, RouteInboundResponse, RoutePreparationVisibility, RoutePrepareOutcome,
    RouteReadinessOutcome, RouteReadinessSlot, RouteReservationIdentity, RouteServerBinding,
    RouteShutdownInput, ServerResponseSlot, ServerRouteSlot, StartStatus,
    StaticBindingCompilerRegistration, StaticBindingRegistration, StaticBindingRegistrationInput,
    StaticHandlerRegistration, ThingSlotId,
};
use clinkz_wot_foundation::{
    BenchmarkStaticReferenceV1, GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile,
    WorkBudget, WorkClass,
};
use clinkz_wot_protocol_bindings_zenoh::extract_zenoh_target_from_resolved_href;
use clinkz_wot_servient::{
    ExposedThingHandle, Servient, ServientBuilder, StaticServient, StaticServientBuilder,
};
use clinkz_wot_td::{
    affordance::{InteractionHelper, PropertyAffordance},
    data_schema::DataSchema,
    form::Form,
    thing::Thing,
};
use zenoh::{
    Config, Wait,
    query::{Query, Queryable},
};

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const ROUTE_FOOTPRINT: BindingLifetimeFootprint = BindingLifetimeFootprint::new(8, 4096);
const READINESS_FOOTPRINT: BindingLifetimeFootprint = BindingLifetimeFootprint::new(1, 64);
const RESPONSE_FOOTPRINT: BindingLifetimeFootprint = BindingLifetimeFootprint::new(3, 512);

type ProbeFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ZenohRouteArtifact {
    transport: Box<str>,
    authority: Box<str>,
    key_expr: Box<str>,
    property: Box<str>,
    content_type: Option<Box<str>>,
    subprotocol: Option<Box<str>>,
    form_index: u32,
}

impl ZenohRouteArtifact {
    fn footprint(&self) -> BindingArtifactFootprint {
        let bytes = self.transport.len()
            + self.authority.len()
            + self.key_expr.len()
            + self.property.len()
            + self.content_type.as_deref().map_or(0, str::len)
            + self.subprotocol.as_deref().map_or(0, str::len);
        BindingArtifactFootprint::new(6, bytes as u64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ZenohCompilerCursor;

#[derive(Clone)]
struct ZenohRouteCompiler {
    compatibility: BindingArtifactCompatibility,
}

impl ZenohRouteCompiler {
    fn compile(&self, input: &BindingCompilerInput<'_>) -> CoreResult<ZenohRouteArtifact> {
        if input.role() != BindingArtifactRole::ProducerRoute {
            return Err(core_error(ErrorPhase::Prepare));
        }
        let plan = input.logical_plan();
        let target = extract_zenoh_target_from_resolved_href(plan.resolved_target())
            .map_err(|_| core_error(ErrorPhase::Prepare))?;
        Ok(ZenohRouteArtifact {
            transport: target.transport.into_boxed_str(),
            authority: target.authority.into_boxed_str(),
            key_expr: target.key_expr.into_boxed_str(),
            property: Box::from(plan.property_name()),
            content_type: plan.content_type().map(Box::from),
            subprotocol: plan.subprotocol().map(Box::from),
            form_index: plan.form_index(),
        })
    }

    fn reservation(
        &self,
        input: &BindingCompilerInput<'_>,
        artifact: &ZenohRouteArtifact,
    ) -> RouteReservationIdentity {
        let mut endpoint = *input.candidate().configuration().as_bytes();
        for (index, byte) in artifact
            .transport
            .bytes()
            .chain(artifact.authority.bytes())
            .chain(artifact.key_expr.bytes())
            .enumerate()
        {
            let slot = index % endpoint.len();
            endpoint[slot] = endpoint[slot]
                .wrapping_add(byte)
                .rotate_left((index % 8) as u32);
        }
        RouteReservationIdentity::new(
            CollisionDomainId::new(*self.compatibility.as_bytes()),
            EndpointReservationKey::new(endpoint),
        )
    }
}

impl BindingCompilerExtension for ZenohRouteCompiler {
    type Cursor = ZenohCompilerCursor;
    type Artifact = ZenohRouteArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        let artifact = self.compile(input)?;
        Ok(BindingCompilerBounds::new(
            artifact.footprint(),
            std::mem::size_of::<ZenohCompilerCursor>() as u64,
            256,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }

    fn start(&self, _input: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        Ok(ZenohCompilerCursor)
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
        let artifact = match self.compile(input) {
            Ok(artifact) => artifact,
            Err(error) => {
                return BindingCompilerStep::Failed(clinkz_wot_core::BindingCompilerFailure::new(
                    error, cursor,
                ));
            }
        };
        let footprint = artifact.footprint();
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::producer_route(
            self.compatibility,
            footprint,
            self.reservation(input, &artifact),
            artifact,
        )))
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProbeSnapshot {
    prepared_artifact: Option<ZenohRouteArtifact>,
    declarations_started: u32,
    declarations_completed: u32,
    readiness_polls: u32,
    readiness_failures: u32,
    activations: u32,
    commits_closed: u32,
    queries_arrived: u32,
    permitted_accept_polls: u32,
    correlations_accepted: Vec<u64>,
    responses_delivered: u32,
    aborts_started: u32,
    shutdowns_started: u32,
    undeclarations_completed: u32,
    sessions_closed: u32,
    terminal_cleanups: u32,
    rejected_while_closed_or_full: u32,
    accepted_routes: Vec<clinkz_wot_core::binding::BindingRouteKey>,
    prepared_state_address: Option<usize>,
    active_state_address: Option<usize>,
    committed_state_address: Option<usize>,
    prepared_footprint: Option<BindingLifetimeFootprint>,
    active_footprint: Option<BindingLifetimeFootprint>,
    committed_footprint: Option<BindingLifetimeFootprint>,
    route_state_drops: u32,
}

#[derive(Clone, Default)]
struct ProbeTelemetry(Arc<Mutex<ProbeSnapshot>>);

impl ProbeTelemetry {
    fn update<R>(&self, operation: impl FnOnce(&mut ProbeSnapshot) -> R) -> R {
        operation(
            &mut self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    fn snapshot(&self) -> ProbeSnapshot {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

struct InFlightQuery {
    correlation: CorrelationId,
    query: Query,
    key_expr: Box<str>,
    default_content_type: Box<str>,
}

struct RouteIo {
    accepting: bool,
    pending: Option<Query>,
    in_flight: Option<InFlightQuery>,
    next_correlation: u64,
    waker: Option<Waker>,
}

impl Default for RouteIo {
    fn default() -> Self {
        Self {
            accepting: true,
            pending: None,
            in_flight: None,
            next_correlation: 1,
            waker: None,
        }
    }
}

trait DeclaredRoute: Send {
    fn into_cleanup(self: Box<Self>) -> ProbeFuture<Result<(), String>>;
}

struct DeclaredQueryable {
    session: zenoh::Session,
    queryable: Queryable<()>,
    telemetry: ProbeTelemetry,
}

impl DeclaredRoute for DeclaredQueryable {
    fn into_cleanup(self: Box<Self>) -> ProbeFuture<Result<(), String>> {
        let Self {
            session,
            queryable,
            telemetry,
        } = *self;
        Box::pin(async move {
            queryable
                .undeclare()
                .await
                .map_err(|error| error.to_string())?;
            telemetry.update(|snapshot| snapshot.undeclarations_completed += 1);
            session.close().await.map_err(|error| error.to_string())?;
            telemetry.update(|snapshot| snapshot.sessions_closed += 1);
            Ok(())
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RouteStage {
    Declaring,
    Prepared,
    Active,
    CommittedClosed,
    Cleaning,
    Closed,
}

struct ZenohRouteLifecycle {
    stage: RouteStage,
    declaration: Option<ProbeFuture<Result<Box<dyn DeclaredRoute>, String>>>,
    declared: Option<Box<dyn DeclaredRoute>>,
    cleanup: Option<ProbeFuture<Result<(), String>>>,
    cleanup_context: Option<CleanupPhaseContext>,
}

struct ZenohRouteState {
    lifecycle: Mutex<ZenohRouteLifecycle>,
    metadata: ZenohRouteArtifact,
    io: Arc<Mutex<RouteIo>>,
    telemetry: ProbeTelemetry,
}

impl ZenohRouteState {
    fn stage(&self) -> RouteStage {
        self.lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .stage
    }

    fn transition(&self, from: RouteStage, to: RouteStage) {
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(lifecycle.stage, from);
        lifecycle.stage = to;
    }

    fn poll_declaration(&self, cx: &mut Context<'_>) -> Poll<Result<(), String>> {
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let declaration = lifecycle
            .declaration
            .as_mut()
            .expect("Host preparing route retains declaration future");
        match declaration.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(declared)) => {
                lifecycle.declaration = None;
                lifecycle.declared = Some(declared);
                lifecycle.stage = RouteStage::Prepared;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                lifecycle.declaration = None;
                lifecycle.stage = RouteStage::Closed;
                Poll::Ready(Err(error))
            }
        }
    }

    fn begin_cleanup(&self, cleanup: CleanupPhaseContext, abort: bool) {
        if abort {
            self.telemetry
                .update(|snapshot| snapshot.aborts_started += 1);
        } else {
            self.telemetry
                .update(|snapshot| snapshot.shutdowns_started += 1);
        }
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lifecycle.declaration = None;
        let declared = lifecycle.declared.take();
        let io = Arc::clone(&self.io);
        let telemetry = self.telemetry.clone();
        lifecycle.stage = RouteStage::Cleaning;
        lifecycle.cleanup_context = Some(cleanup);
        lifecycle.cleanup = Some(Box::pin(async move {
            let (pending, in_flight) = {
                let mut io = io.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                io.accepting = false;
                io.waker = None;
                (io.pending.take(), io.in_flight.take())
            };
            if let Some(query) = pending {
                query
                    .reply_err("route draining")
                    .await
                    .map_err(|error| error.to_string())?;
            }
            if let Some(in_flight) = in_flight {
                in_flight
                    .query
                    .reply_err("route draining")
                    .await
                    .map_err(|error| error.to_string())?;
            }
            if let Some(declared) = declared {
                declared.into_cleanup().await?;
            }
            telemetry.update(|snapshot| snapshot.terminal_cleanups += 1);
            Ok(())
        }));
    }

    fn poll_cleanup(
        &self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        if budget.consume(WorkClass::CleanupItems, 1).is_err() {
            return Poll::Pending;
        }
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let cleanup = lifecycle
            .cleanup
            .as_mut()
            .expect("cleanup future is retained");
        match cleanup.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                lifecycle.cleanup = None;
                lifecycle.cleanup_context = None;
                lifecycle.stage = RouteStage::Closed;
                Poll::Ready(RouteCleanupOutcome::Complete)
            }
            Poll::Ready(Err(_)) => {
                let context = lifecycle
                    .cleanup_context
                    .take()
                    .expect("failed cleanup retains its phase");
                lifecycle.cleanup = None;
                lifecycle.stage = RouteStage::Closed;
                Poll::Ready(RouteCleanupOutcome::ResidualExternalState(cleanup_record(
                    &context,
                )))
            }
        }
    }
}

impl Drop for ZenohRouteState {
    fn drop(&mut self) {
        self.telemetry
            .update(|snapshot| snapshot.route_state_drops += 1);
    }
}

fn new_zenoh_route_state(
    metadata: ZenohRouteArtifact,
    telemetry: ProbeTelemetry,
) -> (ZenohRouteState, Weak<Mutex<RouteIo>>) {
    let io = Arc::new(Mutex::new(RouteIo::default()));
    let callback_io = Arc::clone(&io);
    let callback_telemetry = telemetry.clone();
    let endpoint = format!("{}/{}", metadata.transport, metadata.authority);
    let key_expr = metadata.key_expr.to_string();
    let declaration_telemetry = telemetry.clone();
    let declaration = Box::pin(async move {
        let session = zenoh::open(server_config(&endpoint))
            .await
            .map_err(|error| error.to_string())?;
        let queryable = session
            .declare_queryable(key_expr)
            .callback(move |query| {
                let rejected = {
                    let mut io = callback_io
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if !io.accepting || io.pending.is_some() || io.in_flight.is_some() {
                        true
                    } else {
                        io.pending = Some(query);
                        if let Some(waker) = io.waker.take() {
                            waker.wake();
                        }
                        callback_telemetry.update(|snapshot| snapshot.queries_arrived += 1);
                        false
                    }
                };
                if rejected {
                    callback_telemetry
                        .update(|snapshot| snapshot.rejected_while_closed_or_full += 1);
                }
            })
            .await
            .map_err(|error| error.to_string())?;
        Ok(Box::new(DeclaredQueryable {
            session,
            queryable,
            telemetry: declaration_telemetry,
        }) as Box<dyn DeclaredRoute>)
    });
    telemetry.update(|snapshot| {
        snapshot.prepared_artifact = Some(metadata.clone());
        snapshot.declarations_started += 1;
    });
    let state = ZenohRouteState {
        lifecycle: Mutex::new(ZenohRouteLifecycle {
            stage: RouteStage::Declaring,
            declaration: Some(declaration),
            declared: None,
            cleanup: None,
            cleanup_context: None,
        }),
        metadata,
        io,
        telemetry,
    };
    let response_io = Arc::downgrade(&state.io);
    (state, response_io)
}

fn begin_zenoh_response(
    response: RouteInboundResponse,
    response_io: Option<&Weak<Mutex<RouteIo>>>,
) -> Result<(RouteInboundResponse, ZenohResponseState), BindingInputRejection<RouteInboundResponse>>
{
    let route_key = *response.opportunity().route();
    let correlation = response.opportunity().correlation();
    let Some(io) = response_io.and_then(Weak::upgrade) else {
        return Err(BindingInputRejection::new(
            response,
            operational_error(route_key, ErrorPhase::Delivery),
        ));
    };
    let in_flight = {
        let mut io = io.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match io.in_flight.take() {
            Some(in_flight) if in_flight.correlation == correlation => in_flight,
            other => {
                io.in_flight = other;
                return Err(BindingInputRejection::new(
                    response,
                    operational_error(route_key, ErrorPhase::Delivery),
                ));
            }
        }
    };
    let reply = match response.result() {
        Ok(output) => Ok(output.data().cloned()),
        Err(_) => Err(()),
    };
    let future = Box::pin(async move {
        match reply {
            Ok(payload) => {
                let payload = payload.unwrap_or_else(|| {
                    Payload::new(Vec::<u8>::new(), in_flight.default_content_type.to_string())
                });
                in_flight
                    .query
                    .reply(in_flight.key_expr.as_ref(), payload.body.as_ref().to_vec())
                    .encoding(payload.content_type)
                    .await
                    .map_err(|error| error.to_string())
            }
            Err(()) => in_flight
                .query
                .reply_err("property read handler failed")
                .await
                .map_err(|error| error.to_string()),
        }
    });
    Ok((response, ZenohResponseState { future }))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ZenohReadinessState {
    pending_once: bool,
}

struct ZenohResponseState {
    future: ProbeFuture<Result<(), String>>,
}

struct StaticZenohServer {
    compatibility: BindingArtifactCompatibility,
    fail_readiness: bool,
    io: Option<Weak<Mutex<RouteIo>>>,
    telemetry: ProbeTelemetry,
}

impl StaticZenohServer {
    fn start_cleanup(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<ZenohRouteState>,
        abort: bool,
    ) -> StartStatus<RouteCleanupOutcome> {
        route.state_mut().begin_cleanup(cleanup, abort);
        StartStatus::Pending
    }

    fn poll_cleanup(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<ZenohRouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        route.state_mut().poll_cleanup(cx, budget)
    }
}

impl PollServerBinding for StaticZenohServer {
    type Compiler = ZenohRouteCompiler;
    type RouteState = ZenohRouteState;
    type ReadinessState = ZenohReadinessState;
    type ResponseState = ZenohResponseState;

    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn route_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<ZenohRouteState>(ROUTE_FOOTPRINT)
            .with_transient(BindingTransientFootprint::new(512))
    }

    fn readiness_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<ZenohReadinessState>(READINESS_FOOTPRINT)
    }

    fn response_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<ZenohResponseState>(RESPONSE_FOOTPRINT)
            .with_transient(BindingTransientFootprint::new(256))
    }

    fn start_prepare(
        &mut self,
        input: PrepareInput,
        artifact: &BindingArtifactEnvelope<ZenohRouteArtifact>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Result<StartStatus<RoutePrepareOutcome<()>>, BindingInputRejection<PrepareInput>> {
        let metadata = artifact.artifact().payload().clone();
        if metadata.transport.as_ref() != "tcp"
            || metadata.key_expr.is_empty()
            || metadata.authority.is_empty()
            || self.io.is_some()
        {
            let error = operational_error(*input.route(), ErrorPhase::Prepare);
            return Err(BindingInputRejection::new(input, error));
        }
        let (state, response_io) = new_zenoh_route_state(metadata, self.telemetry.clone());
        self.io = Some(response_io);
        route.initialize(input, state);
        let state_address = route.state_mut() as *mut ZenohRouteState as usize;
        self.telemetry.update(|snapshot| {
            snapshot.prepared_state_address = Some(state_address);
            snapshot.prepared_footprint = Some(ROUTE_FOOTPRINT);
        });
        Ok(StartStatus::Pending)
    }

    fn poll_prepare(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<()>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        match route.state_mut().poll_declaration(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                self.telemetry
                    .update(|snapshot| snapshot.declarations_completed += 1);
                Poll::Ready(RoutePrepareOutcome::Prepared(()))
            }
            Poll::Ready(Err(_)) => Poll::Ready(RoutePrepareOutcome::RejectedNoResource(
                operational_error(*route.input().route(), ErrorPhase::Prepare),
            )),
        }
    }

    fn poll_cancel_prepare(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RoutePrepareOutcome<()>, ()>>> {
        Poll::Pending
    }

    fn start_readiness(
        &mut self,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        _budget: &mut WorkBudget,
    ) -> StartStatus<RouteReadinessOutcome<()>> {
        readiness.initialize_state(ZenohReadinessState { pending_once: true });
        StartStatus::Pending
    }

    fn poll_readiness(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteReadinessOutcome<()>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.telemetry
            .update(|snapshot| snapshot.readiness_polls += 1);
        if self.fail_readiness {
            self.fail_readiness = false;
            self.telemetry
                .update(|snapshot| snapshot.readiness_failures += 1);
            return Poll::Ready(RouteReadinessOutcome::Failed {
                guard: (),
                error: operational_error(*route.input().route(), ErrorPhase::Readiness),
            });
        }
        let state = readiness.state_mut();
        if state.pending_once {
            state.pending_once = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(RouteReadinessOutcome::Ready(()))
    }

    fn poll_cancel_readiness(
        &mut self,
        _cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        _route: &mut ServerRouteSlot<Self::RouteState>,
        _readiness: &mut RouteReadinessSlot<Self::ReadinessState>,
        _budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteReadinessOutcome<()>, ()>>> {
        Poll::Pending
    }

    fn start_activate(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> StartStatus<RouteActivationOutcome<(), ()>> {
        let state = route.state_mut();
        state.transition(RouteStage::Prepared, RouteStage::Active);
        let address = state as *mut ZenohRouteState as usize;
        self.telemetry.update(|snapshot| {
            snapshot.activations += 1;
            snapshot.active_state_address = Some(address);
            snapshot.active_footprint = Some(ROUTE_FOOTPRINT);
        });
        StartStatus::Ready(RouteActivationOutcome::Active(()))
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
    ) -> Poll<CoreResult<BindingCallSettlement<RouteActivationOutcome<(), ()>, ()>>> {
        Poll::Pending
    }

    fn start_commit(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> StartStatus<RouteCommitOutcome<(), ()>> {
        let state = route.state_mut();
        state.transition(RouteStage::Active, RouteStage::CommittedClosed);
        let address = state as *mut ZenohRouteState as usize;
        self.telemetry.update(|snapshot| {
            snapshot.commits_closed += 1;
            snapshot.committed_state_address = Some(address);
            snapshot.committed_footprint = Some(ROUTE_FOOTPRINT);
        });
        StartStatus::Ready(RouteCommitOutcome::Committed(()))
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
    ) -> Poll<CoreResult<BindingCallSettlement<RouteCommitOutcome<(), ()>, ()>>> {
        Poll::Pending
    }

    fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        permit: RouteActivationPermit<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        if permit.route() != route.input().route()
            || route.state_mut().stage() != RouteStage::CommittedClosed
        {
            return Poll::Ready(Err(core_error(ErrorPhase::Binding)));
        }
        self.telemetry
            .update(|snapshot| snapshot.permitted_accept_polls += 1);
        let metadata = route.state_mut().metadata.clone();
        let io = Arc::clone(&route.state_mut().io);
        let mut io = io.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        io.waker = Some(cx.waker().clone());
        let Some(query) = io.pending.take() else {
            return Poll::Pending;
        };
        if io.in_flight.is_some() || io.next_correlation == 0 {
            io.pending = Some(query);
            return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::OperationalError(
                operational_error(*permit.route(), ErrorPhase::Binding),
            )));
        }
        let correlation = CorrelationId::new(io.next_correlation);
        io.next_correlation = io.next_correlation.checked_add(1).unwrap_or(0);
        let input = query_input(&query);
        io.in_flight = Some(InFlightQuery {
            correlation,
            query,
            key_expr: metadata.key_expr.clone(),
            default_content_type: metadata
                .content_type
                .clone()
                .unwrap_or_else(|| Box::from("application/octet-stream")),
        });
        io.waker = None;
        self.telemetry.update(|snapshot| {
            snapshot.correlations_accepted.push(correlation.get());
            snapshot.accepted_routes.push(*permit.route());
        });
        Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Request(
            RouteInboundRequest::new(
                *permit.route(),
                correlation,
                AffordanceTarget::Property(Arc::from(metadata.property.as_ref())),
                input,
            ),
        )))
    }

    fn start_abort(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome> {
        self.start_cleanup(cleanup, route, true)
    }

    fn poll_abort(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        self.poll_cleanup(cx, route, budget)
    }

    fn start_shutdown(
        &mut self,
        cleanup: CleanupPhaseContext,
        route: &mut ServerRouteSlot<Self::RouteState>,
        _budget: &mut WorkBudget,
    ) -> StartStatus<RouteCleanupOutcome> {
        self.start_cleanup(cleanup, route, false)
    }

    fn poll_shutdown(
        &mut self,
        cx: &mut Context<'_>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        self.poll_cleanup(cx, route, budget)
    }

    fn acknowledge_route(
        &mut self,
        route: &mut ServerRouteSlot<Self::RouteState>,
    ) -> CoreResult<()> {
        if route.state_mut().stage() != RouteStage::Closed {
            return Err(core_error(ErrorPhase::Cleanup));
        }
        route.clear();
        self.io = None;
        assert_eq!(
            self.telemetry.snapshot().route_state_drops,
            1,
            "static terminal acknowledgement drops route state exactly once"
        );
        Ok(())
    }

    fn start_response(
        &mut self,
        response: RouteInboundResponse,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        _budget: &mut WorkBudget,
    ) -> Result<StartStatus<BindingDeliveryOutcome>, BindingInputRejection<RouteInboundResponse>>
    {
        let (response, state) = begin_zenoh_response(response, self.io.as_ref())?;
        slot.initialize(response, state);
        Ok(StartStatus::Pending)
    }

    fn poll_response(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Poll<BindingDeliveryOutcome> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        match slot.state_mut().future.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                self.telemetry
                    .update(|snapshot| snapshot.responses_delivered += 1);
                Poll::Ready(BindingDeliveryOutcome::Delivered)
            }
            Poll::Ready(Err(_)) => Poll::Ready(BindingDeliveryOutcome::Failed(operational_error(
                *slot.response().opportunity().route(),
                ErrorPhase::Delivery,
            ))),
        }
    }

    fn poll_cancel_response(
        &mut self,
        cx: &mut Context<'_>,
        _cleanup: &CleanupPhaseContext,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>> {
        match self.poll_response(cx, slot, budget) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(outcome) => Poll::Ready(Ok(BindingCallSettlement::Returned(outcome))),
        }
    }

    fn acknowledge_response(
        &mut self,
        slot: &mut ServerResponseSlot<Self::ResponseState>,
    ) -> CoreResult<()> {
        slot.clear();
        Ok(())
    }
}

fn prepared_zenoh_state(guard: &HostPreparedRouteGuard) -> Pin<&ZenohRouteState> {
    guard
        .try_state_pin_ref::<ZenohRouteState>()
        .expect("prepared Host guard retains Zenoh route state")
}

fn active_zenoh_state(guard: &HostActiveRouteGuard) -> Pin<&ZenohRouteState> {
    guard
        .try_state_pin_ref::<ZenohRouteState>()
        .expect("active Host guard retains Zenoh route state")
}

fn committed_zenoh_state(guard: &HostCommittedRouteGuard) -> Pin<&ZenohRouteState> {
    guard
        .try_state_pin_ref::<ZenohRouteState>()
        .expect("committed Host guard retains Zenoh route state")
}

struct HostReadyCall<T, C> {
    value: Option<T>,
    _cleanup: core::marker::PhantomData<C>,
}

impl<T, C> HostReadyCall<T, C> {
    fn new(value: T) -> Self {
        Self {
            value: Some(value),
            _cleanup: core::marker::PhantomData,
        }
    }
}

impl<T, C> HostBindingCall<T, C> for HostReadyCall<T, C>
where
    T: Send + Unpin + 'static,
    C: Send + Unpin + 'static,
{
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        READINESS_FOOTPRINT
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<T> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(self.value.take().expect("Host ready call completed twice"))
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<T, C>>> {
        Ok(StartStatus::Pending)
    }

    fn poll_cancel(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<T, C>>> {
        Poll::Ready(Ok(BindingCallSettlement::Returned(
            self.value.take().expect("Host ready call cancelled twice"),
        )))
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

struct HostZenohPrepareCall {
    guard: Option<HostPreparedRouteGuard>,
    response_io: Arc<Mutex<Option<Weak<Mutex<RouteIo>>>>>,
    telemetry: ProbeTelemetry,
}

impl HostBindingCall<RoutePrepareOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>
    for HostZenohPrepareCall
{
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        ROUTE_FOOTPRINT
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<RoutePrepareOutcome<HostPreparedRouteGuard>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        let guard = self
            .guard
            .as_mut()
            .expect("Host prepare call completed twice");
        let route = *guard.route();
        let state = prepared_zenoh_state(guard);
        match state.get_ref().poll_declaration(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                self.telemetry
                    .update(|snapshot| snapshot.declarations_completed += 1);
                Poll::Ready(RoutePrepareOutcome::Prepared(
                    self.guard.take().expect("Host prepared guard is retained"),
                ))
            }
            Poll::Ready(Err(_)) => {
                *self
                    .response_io
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
                drop(self.guard.take());
                Poll::Ready(RoutePrepareOutcome::RejectedNoResource(operational_error(
                    route,
                    ErrorPhase::Prepare,
                )))
            }
        }
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> CoreResult<
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
        CoreResult<
            BindingCallSettlement<
                RoutePrepareOutcome<HostPreparedRouteGuard>,
                HostRouteCleanupSuccessor,
            >,
        >,
    > {
        let guard = self
            .guard
            .take()
            .expect("Host prepare cancellation completed twice");
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: HostRouteCleanupSuccessor::AbortPrepared(guard),
            },
        }))
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

struct HostZenohReadinessCall {
    guard: Option<HostPreparedRouteGuard>,
    fail: bool,
    pending_once: bool,
    telemetry: ProbeTelemetry,
}

impl HostBindingCall<RouteReadinessOutcome<HostPreparedRouteGuard>, HostRouteCleanupSuccessor>
    for HostZenohReadinessCall
{
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        READINESS_FOOTPRINT
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteReadinessOutcome<HostPreparedRouteGuard>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.telemetry
            .update(|snapshot| snapshot.readiness_polls += 1);
        if self.fail {
            self.fail = false;
            self.telemetry
                .update(|snapshot| snapshot.readiness_failures += 1);
            let guard = self.guard.take().expect("readiness guard is retained");
            let error = operational_error(*guard.route(), ErrorPhase::Readiness);
            return Poll::Ready(RouteReadinessOutcome::Failed { guard, error });
        }
        if self.pending_once {
            self.pending_once = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(RouteReadinessOutcome::Ready(
            self.guard.take().expect("readiness guard is retained"),
        ))
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> CoreResult<
        StartStatus<
            BindingCallSettlement<
                RouteReadinessOutcome<HostPreparedRouteGuard>,
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
        CoreResult<
            BindingCallSettlement<
                RouteReadinessOutcome<HostPreparedRouteGuard>,
                HostRouteCleanupSuccessor,
            >,
        >,
    > {
        let guard = self.guard.take().expect("readiness cancelled twice");
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: HostRouteCleanupSuccessor::AbortPrepared(guard),
            },
        }))
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

enum HostZenohCleanupGuard {
    Prepared(HostPreparedRouteGuard),
    Shutdown(HostShutdownRouteGuard),
}

impl HostZenohCleanupGuard {
    fn state(&self) -> Pin<&ZenohRouteState> {
        match self {
            Self::Prepared(guard) => prepared_zenoh_state(guard),
            Self::Shutdown(HostShutdownRouteGuard::Active(guard)) => active_zenoh_state(guard),
            Self::Shutdown(HostShutdownRouteGuard::Committed(guard)) => {
                committed_zenoh_state(guard)
            }
        }
    }
}

struct HostZenohCleanupCall {
    guard: Option<HostZenohCleanupGuard>,
    response_io: Arc<Mutex<Option<Weak<Mutex<RouteIo>>>>>,
    telemetry: ProbeTelemetry,
}

impl HostZenohCleanupCall {
    fn abort(
        input: RouteAbortInput,
        response_io: Arc<Mutex<Option<Weak<Mutex<RouteIo>>>>>,
        telemetry: ProbeTelemetry,
    ) -> Self {
        let (guard, cleanup) = input.into_parts();
        prepared_zenoh_state(&guard)
            .get_ref()
            .begin_cleanup(cleanup, true);
        Self {
            guard: Some(HostZenohCleanupGuard::Prepared(guard)),
            response_io,
            telemetry,
        }
    }

    fn shutdown(
        input: RouteShutdownInput,
        response_io: Arc<Mutex<Option<Weak<Mutex<RouteIo>>>>>,
        telemetry: ProbeTelemetry,
    ) -> Self {
        let (guard, cleanup) = input.into_parts();
        let state = match &guard {
            HostShutdownRouteGuard::Active(guard) => active_zenoh_state(guard),
            HostShutdownRouteGuard::Committed(guard) => committed_zenoh_state(guard),
        };
        state.get_ref().begin_cleanup(cleanup, false);
        Self {
            guard: Some(HostZenohCleanupGuard::Shutdown(guard)),
            response_io,
            telemetry,
        }
    }
}

impl HostBindingCall<RouteCleanupOutcome, HostRouteCleanupSuccessor> for HostZenohCleanupCall {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        ROUTE_FOOTPRINT
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<RouteCleanupOutcome> {
        assert_eq!(
            self.telemetry.snapshot().route_state_drops,
            0,
            "Host Zenoh state cannot drop before terminal cleanup"
        );
        let outcome = self
            .guard
            .as_ref()
            .expect("Host cleanup call completed twice")
            .state()
            .get_ref()
            .poll_cleanup(cx, budget);
        if outcome.is_ready() {
            *self
                .response_io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            drop(self.guard.take());
            assert_eq!(
                self.telemetry.snapshot().route_state_drops,
                1,
                "Host terminal cleanup drops route state exactly once"
            );
        }
        outcome
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> CoreResult<
        StartStatus<BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>>,
    > {
        Ok(StartStatus::Pending)
    }

    fn poll_cancel(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<RouteCleanupOutcome, HostRouteCleanupSuccessor>>>
    {
        match self.as_mut().poll_result(cx, budget) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(outcome) => Poll::Ready(Ok(BindingCallSettlement::Returned(outcome))),
        }
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

struct HostZenohResponseCall {
    response: Option<RouteInboundResponse>,
    state: ZenohResponseState,
    telemetry: ProbeTelemetry,
}

impl HostBindingCall<BindingDeliveryOutcome, NoCleanupSuccessor> for HostZenohResponseCall {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        RESPONSE_FOOTPRINT
    }

    fn poll_result(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<BindingDeliveryOutcome> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        match self.state.future.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                self.response = None;
                self.telemetry
                    .update(|snapshot| snapshot.responses_delivered += 1);
                Poll::Ready(BindingDeliveryOutcome::Delivered)
            }
            Poll::Ready(Err(_)) => {
                let route = *self
                    .response
                    .as_ref()
                    .expect("failed response retains route identity")
                    .opportunity()
                    .route();
                self.response = None;
                Poll::Ready(BindingDeliveryOutcome::Failed(operational_error(
                    route,
                    ErrorPhase::Delivery,
                )))
            }
        }
    }

    fn start_cancel(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _cleanup: CleanupPhaseContext,
        _budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<BindingDeliveryOutcome>>> {
        Ok(StartStatus::Pending)
    }

    fn poll_cancel(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<BindingDeliveryOutcome>>> {
        match self.as_mut().poll_result(cx, budget) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(outcome) => Poll::Ready(Ok(BindingCallSettlement::Returned(outcome))),
        }
    }

    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
}

struct HostZenohServer {
    compatibility: BindingArtifactCompatibility,
    fail_readiness: AtomicBool,
    response_io: Arc<Mutex<Option<Weak<Mutex<RouteIo>>>>>,
    telemetry: ProbeTelemetry,
}

impl RouteServerBinding for HostZenohServer {
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
        let Some(metadata) = artifact
            .artifact()
            .try_payload::<ZenohRouteArtifact>(self.compatibility)
        else {
            let error = operational_error(*input.route(), ErrorPhase::Prepare);
            return Err(BindingInputRejection::new(input, error));
        };
        if metadata.transport.as_ref() != "tcp"
            || metadata.key_expr.is_empty()
            || metadata.authority.is_empty()
            || self
                .response_io
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
                .and_then(Weak::upgrade)
                .is_some()
        {
            let error = operational_error(*input.route(), ErrorPhase::Prepare);
            return Err(BindingInputRejection::new(input, error));
        }
        let footprint = input.admitted_footprint();
        let (state, response_io) = new_zenoh_route_state(metadata.clone(), self.telemetry.clone());
        let guard = HostPreparedRouteGuard::new(input, footprint, state);
        let state_address =
            prepared_zenoh_state(&guard).get_ref() as *const ZenohRouteState as usize;
        self.telemetry.update(|snapshot| {
            snapshot.prepared_state_address = Some(state_address);
            snapshot.prepared_footprint = Some(footprint);
        });
        *self
            .response_io
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(response_io);
        Ok(HostBindingCallBox::new(HostZenohPrepareCall {
            guard: Some(guard),
            response_io: Arc::clone(&self.response_io),
            telemetry: self.telemetry.clone(),
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
        assert_eq!(
            prepared_zenoh_state(&guard).get_ref().stage(),
            RouteStage::Prepared
        );
        Ok(HostBindingCallBox::new(HostZenohReadinessCall {
            guard: Some(guard),
            fail: self.fail_readiness.swap(false, Ordering::SeqCst),
            pending_once: true,
            telemetry: self.telemetry.clone(),
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
        let footprint = guard.lifetime_footprint();
        let state = prepared_zenoh_state(&guard);
        state
            .get_ref()
            .transition(RouteStage::Prepared, RouteStage::Active);
        let address = state.get_ref() as *const ZenohRouteState as usize;
        self.telemetry.update(|snapshot| {
            snapshot.activations += 1;
            snapshot.active_state_address = Some(address);
            snapshot.active_footprint = Some(footprint);
        });
        Ok(HostBindingCallBox::new(HostReadyCall::new(
            RouteActivationOutcome::Active(HostActiveRouteGuard::new(guard)),
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
        let footprint = guard.lifetime_footprint();
        let state = active_zenoh_state(&guard);
        state
            .get_ref()
            .transition(RouteStage::Active, RouteStage::CommittedClosed);
        let address = state.get_ref() as *const ZenohRouteState as usize;
        self.telemetry.update(|snapshot| {
            snapshot.commits_closed += 1;
            snapshot.committed_state_address = Some(address);
            snapshot.committed_footprint = Some(footprint);
        });
        Ok(HostBindingCallBox::new(HostReadyCall::new(
            RouteCommitOutcome::Committed(HostCommittedRouteGuard::new(guard)),
        )))
    }

    fn poll_accept(
        &self,
        route: &HostCommittedRouteGuard,
        permit: RouteActivationPermit<'_>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<clinkz_wot_core::RouteAcceptEvent>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        let route_key = *route.route();
        let state = committed_zenoh_state(route);
        if permit.route() != &route_key || state.get_ref().stage() != RouteStage::CommittedClosed {
            return Poll::Ready(Err(core_error(ErrorPhase::Binding)));
        }
        self.telemetry
            .update(|snapshot| snapshot.permitted_accept_polls += 1);
        let metadata = state.get_ref().metadata.clone();
        let mut io = state
            .get_ref()
            .io
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        io.waker = Some(cx.waker().clone());
        let Some(query) = io.pending.take() else {
            return Poll::Pending;
        };
        if io.in_flight.is_some() || io.next_correlation == 0 {
            io.pending = Some(query);
            return Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::OperationalError(
                operational_error(route_key, ErrorPhase::Binding),
            )));
        }
        let correlation = CorrelationId::new(io.next_correlation);
        io.next_correlation = io.next_correlation.checked_add(1).unwrap_or(0);
        let input = query_input(&query);
        io.in_flight = Some(InFlightQuery {
            correlation,
            query,
            key_expr: metadata.key_expr.clone(),
            default_content_type: metadata
                .content_type
                .clone()
                .unwrap_or_else(|| Box::from("application/octet-stream")),
        });
        io.waker = None;
        drop(io);
        self.telemetry.update(|snapshot| {
            snapshot.correlations_accepted.push(correlation.get());
            snapshot.accepted_routes.push(route_key);
        });
        Poll::Ready(Ok(clinkz_wot_core::RouteAcceptEvent::Request(
            RouteInboundRequest::new(
                route_key,
                correlation,
                AffordanceTarget::Property(Arc::from(metadata.property.as_ref())),
                input,
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
        assert_eq!(self.telemetry.snapshot().route_state_drops, 0);
        Ok(HostBindingCallBox::new(HostZenohCleanupCall::abort(
            input,
            Arc::clone(&self.response_io),
            self.telemetry.clone(),
        )))
    }

    fn shutdown(
        &self,
        input: RouteShutdownInput,
    ) -> Result<
        HostBindingCallBox<RouteCleanupOutcome, HostRouteCleanupSuccessor>,
        BindingInputRejection<RouteShutdownInput>,
    > {
        assert_eq!(self.telemetry.snapshot().route_state_drops, 0);
        Ok(HostBindingCallBox::new(HostZenohCleanupCall::shutdown(
            input,
            Arc::clone(&self.response_io),
            self.telemetry.clone(),
        )))
    }

    fn deliver_response(
        &self,
        response: RouteInboundResponse,
    ) -> Result<
        HostBindingCallBox<BindingDeliveryOutcome>,
        BindingInputRejection<RouteInboundResponse>,
    > {
        let response_io = self
            .response_io
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let (response, state) = begin_zenoh_response(response, response_io.as_ref())?;
        Ok(HostBindingCallBox::new(HostZenohResponseCall {
            response: Some(response),
            state,
            telemetry: self.telemetry.clone(),
        }))
    }
}

struct RecordingHandler {
    expected_thing: Box<str>,
    expected_property: Box<str>,
    expected_binding: BindingId,
    response: Payload,
    calls: Arc<Mutex<u32>>,
}

impl ReadPropertyHandler for RecordingHandler {
    fn handle(
        &self,
        context: HandlerContext<'_>,
        _input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(context.thing_id().as_str(), self.expected_thing.as_ref());
        assert_eq!(
            context.target().name(),
            Some(self.expected_property.as_ref())
        );
        assert_eq!(
            context.binding().map(|(binding, _)| binding),
            Some(self.expected_binding)
        );
        *self
            .calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
        Ok(InteractionOutput::with_data(self.response.clone()))
    }
}

fn registration(
    binding_id: BindingId,
    port: u16,
    fail_readiness: bool,
) -> (StaticBindingRegistration<StaticZenohServer>, ProbeTelemetry) {
    let compatibility = BindingArtifactCompatibility::new([0x7a; 16]);
    let mut configuration = [0x31; 32];
    configuration[..2].copy_from_slice(&port.to_be_bytes());
    let identity = BindingRegistrationIdentity::new(
        binding_id,
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new(configuration),
        compatibility,
        0,
    );
    let telemetry = ProbeTelemetry::default();
    let limits = BindingIngressLimits::new(1, 4096);
    let input = StaticBindingRegistrationInput::new(
        identity,
        BindingRegistrationCapabilities::producer_property_read(),
        BindingExecutionSupport::application_static(),
        StaticBindingCompilerRegistration::new(ZenohRouteCompiler { compatibility }),
        StaticZenohServer {
            compatibility,
            fail_readiness,
            io: None,
            telemetry: telemetry.clone(),
        },
        BindingResourceDeclarations::new(ROUTE_FOOTPRINT, ROUTE_FOOTPRINT)
            .with_state_footprints(ROUTE_FOOTPRINT, READINESS_FOOTPRINT, RESPONSE_FOOTPRINT)
            .with_transient(BindingTransientFootprint::new(512)),
        BindingIngressPolicy::new(
            RoutePreparationVisibility::BufferWithinAdmittedLimits,
            limits,
            limits,
            limits,
        ),
        BindingStatusPolicy::new(2, 256),
    );
    let registration = StaticBindingRegistration::new(input)
        .unwrap_or_else(|_| panic!("real Zenoh probe registration must validate"));
    (registration, telemetry)
}

fn build_servient<'a>(
    td: Thing,
    property: &str,
    port: u16,
    binding_id: BindingId,
    fail_readiness: bool,
    handler: &'a RecordingHandler,
) -> (impl StaticServient + 'a, ProbeTelemetry) {
    let (registration, telemetry) = registration(binding_id, port, fail_readiness);
    let thing_slot = ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL);
    let handler = StaticHandlerRegistration::new(
        HandlerSlotId::new(SlotIndex::new(0), Generation::INITIAL),
        handler,
        HandlerFootprint::new(1, 0, 0),
    );
    let servient = StaticServientBuilder::new(
        td,
        thing_slot,
        BenchmarkStaticReferenceV1::LIMITS.clone(),
        Deadline::NONE,
    )
    .binding_registration(registration)
    .read_property_handler(property, handler)
    .build()
    .expect("public static Servient accepts the real Zenoh registration");
    (servient, telemetry)
}

fn host_registration(
    binding_id: BindingId,
    port: u16,
    fail_readiness: bool,
) -> (HostBindingRegistration, ProbeTelemetry) {
    let compatibility = BindingArtifactCompatibility::new([0x7a; 16]);
    let mut configuration = [0x31; 32];
    configuration[..2].copy_from_slice(&port.to_be_bytes());
    let identity = BindingRegistrationIdentity::new(
        binding_id,
        BindingGeneration::INITIAL,
        BindingConfigurationDigest::new(configuration),
        compatibility,
        0,
    );
    let telemetry = ProbeTelemetry::default();
    let limits = BindingIngressLimits::new(1, 4096);
    let response_io = Arc::new(Mutex::new(None));
    let input = HostBindingRegistrationInput::new(
        identity,
        BindingRegistrationCapabilities::producer_property_read(),
        BindingExecutionSupport::host_erased(),
        HostBindingCompilerRegistration::new(ZenohRouteCompiler { compatibility }),
        Box::new(HostZenohServer {
            compatibility,
            fail_readiness: AtomicBool::new(fail_readiness),
            response_io,
            telemetry: telemetry.clone(),
        }),
        BindingResourceDeclarations::new(ROUTE_FOOTPRINT, ROUTE_FOOTPRINT)
            .with_state_footprints(ROUTE_FOOTPRINT, READINESS_FOOTPRINT, RESPONSE_FOOTPRINT)
            .with_transient(BindingTransientFootprint::new(512)),
        BindingIngressPolicy::new(
            RoutePreparationVisibility::BufferWithinAdmittedLimits,
            limits,
            limits,
            limits,
        ),
        BindingStatusPolicy::new(2, 256),
    );
    let registration = HostBindingRegistration::new(input)
        .unwrap_or_else(|_| panic!("real Zenoh Host probe registration must validate"));
    (registration, telemetry)
}

fn build_host_servient(
    td: Thing,
    property: &str,
    port: u16,
    binding_id: BindingId,
    fail_readiness: bool,
    handler: RecordingHandler,
) -> (Servient, ExposedThingHandle, ProbeTelemetry) {
    let (registration, telemetry) = host_registration(binding_id, port, fail_readiness);
    let servient = ServientBuilder::new()
        .resource_limits(GatewayDefaultV1::LIMITS.clone())
        .binding_registration(registration)
        .build()
        .expect("public Host Servient accepts the real Zenoh registration");
    let exposed = servient
        .produce_td(td)
        .expect("public Host Servient produces the probe Thing");
    exposed
        .set_read_property_handler(property, handler, HandlerFootprint::new(1, 0, 0))
        .expect("public Host Servient accepts the Property Read handler");
    exposed
        .begin_expose()
        .expect("public Host Servient begins real Zenoh exposure");
    (servient, exposed, telemetry)
}

fn absolute_thing(port: u16, key_expr: &str) -> Thing {
    let write_form = Form::write_property("https://not-selected.invalid/status")
        .build()
        .expect("valid non-read form");
    let read_form = Form::read_property(format!("zenoh+tcp://127.0.0.1:{port}/{key_expr}"))
        .content_type("application/json")
        .subprotocol("zenoh-query")
        .build()
        .expect("valid absolute Zenoh form");
    Thing::builder("Probe Lamp")
        .id("urn:clinkz:probe:lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(write_form)
                .form(read_form)
                .build()
                .expect("valid status property"),
        )
        .build()
        .expect("valid absolute probe Thing")
}

fn relative_thing(port: u16, key_root: &str, id: &str, property: &str) -> Thing {
    let read_form = Form::read_property(format!("properties/{property}"))
        .content_type("text/plain")
        .build()
        .expect("valid relative Zenoh form");
    Thing::builder("Probe Sensor")
        .id(id)
        .base(&format!("zenoh://127.0.0.1:{port}/{key_root}/"))
        .nosec()
        .property(
            property,
            PropertyAffordance::builder(DataSchema::string())
                .form(read_form)
                .build()
                .expect("valid relative property"),
        )
        .build()
        .expect("valid relative probe Thing")
}

fn drive_until(servient: &mut impl StaticServient, condition: impl Fn() -> bool, label: &str) {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    for _ in 0..512 {
        if condition() {
            return;
        }
        let mut budget = WorkBudget::new()
            .with_remaining(WorkClass::BindingPolls, 16)
            .with_remaining(WorkClass::HandlerSteps, 4)
            .with_remaining(WorkClass::CleanupItems, 16);
        let _ = servient.step(&mut cx, &mut budget);
        thread::sleep(Duration::from_millis(5));
    }
    panic!("bounded probe did not reach {label}");
}

fn drive_host_until(servient: &Servient, condition: impl Fn() -> bool, label: &str) {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    for _ in 0..512 {
        if condition() {
            return;
        }
        let mut budget = WorkBudget::new()
            .with_remaining(WorkClass::BindingPolls, 16)
            .with_remaining(WorkClass::HandlerSteps, 4)
            .with_remaining(WorkClass::CleanupItems, 16);
        let _ = servient.step(&mut cx, &mut budget);
        thread::sleep(Duration::from_millis(5));
    }
    panic!("bounded Host probe did not reach {label}");
}

fn wait_until(condition: impl Fn() -> bool, label: &str) {
    for _ in 0..512 {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("bounded external probe did not reach {label}");
}

fn unused_loopback_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("reserve loopback probe port")
        .local_addr()
        .expect("read loopback probe port")
        .port()
}

fn server_config(endpoint: &str) -> Config {
    let mut config = Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config
        .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
        .unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    config
        .insert_json5("transport/shared_memory/enabled", "false")
        .unwrap();
    config
}

fn client_config(endpoint: &str) -> Config {
    let mut config = Config::default();
    config.insert_json5("mode", "\"client\"").unwrap();
    config
        .insert_json5("connect/endpoints", &format!("[\"{endpoint}\"]"))
        .unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    config
        .insert_json5("transport/shared_memory/enabled", "false")
        .unwrap();
    config
}

fn query_input(query: &Query) -> InteractionInput {
    match query.payload() {
        Some(payload) => InteractionInput::with_data(Payload::new(
            payload.to_bytes().into_owned(),
            query
                .encoding()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )),
        None => InteractionInput::empty(),
    }
}

fn core_error(phase: ErrorPhase) -> CoreError {
    CoreError::Binding(ErrorContext::new(phase, RetryClass::Never))
}

fn operational_error(
    route: clinkz_wot_core::binding::BindingRouteKey,
    phase: ErrorPhase,
) -> BindingOperationalError {
    BindingOperationalError::for_route(route, core_error(phase))
}

fn cleanup_record(context: &CleanupPhaseContext) -> CleanupRecord {
    let subject = context.reservation().subject();
    CleanupRecord::try_new(
        CleanupHandle::new(subject),
        subject,
        subject,
        context.operation(),
        0,
        RetryClass::Never,
        0x7a01,
        0,
    )
    .expect("initial residual record fits the reserved retry bound")
}

fn assert_success_semantics(snapshot: &ProbeSnapshot, binding_id: BindingId) {
    assert_eq!(snapshot.declarations_completed, 1);
    assert!(snapshot.readiness_polls >= 2);
    assert_eq!(snapshot.activations, 1);
    assert_eq!(snapshot.commits_closed, 1);
    assert!(snapshot.permitted_accept_polls > 0);
    assert_eq!(snapshot.correlations_accepted, vec![1]);
    assert_eq!(snapshot.responses_delivered, 1);
    assert_eq!(snapshot.shutdowns_started, 1);
    assert_eq!(snapshot.undeclarations_completed, 1);
    assert_eq!(snapshot.sessions_closed, 1);
    assert_eq!(snapshot.terminal_cleanups, 1);
    assert_eq!(snapshot.route_state_drops, 1);
    assert_eq!(snapshot.accepted_routes.len(), 1);
    let route = snapshot.accepted_routes[0];
    assert_eq!(route.binding_id(), binding_id);
    assert_eq!(route.binding_generation(), BindingGeneration::INITIAL);
    assert_eq!(route.route_generation(), Generation::INITIAL);
    assert_eq!(
        snapshot.prepared_state_address,
        snapshot.active_state_address
    );
    assert_eq!(
        snapshot.prepared_state_address,
        snapshot.committed_state_address
    );
    assert!(snapshot.prepared_state_address.is_some());
    assert_eq!(snapshot.prepared_footprint, snapshot.active_footprint);
    assert_eq!(snapshot.prepared_footprint, snapshot.committed_footprint);
    assert_eq!(snapshot.prepared_footprint, Some(ROUTE_FOOTPRINT));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_target_zenoh_property_read_round_trip_and_terminal_drain() {
    zenoh::init_log_from_env_or("error");
    let port = unused_loopback_port();
    let endpoint = format!("tcp/127.0.0.1:{port}");
    let key_expr = format!("clinkz/probe/{port}/lamp/status");
    let binding_id = BindingId::new(0x7a01);
    let calls = Arc::new(Mutex::new(0));
    let handler = RecordingHandler {
        expected_thing: Box::from("urn:clinkz:probe:lamp"),
        expected_property: Box::from("status"),
        expected_binding: binding_id,
        response: Payload::new(br#"{\"status\":\"on\"}"#.to_vec(), "application/json"),
        calls: Arc::clone(&calls),
    };
    let (mut servient, telemetry) = build_servient(
        absolute_thing(port, &key_expr),
        "status",
        port,
        binding_id,
        false,
        &handler,
    );

    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut zero = WorkBudget::new();
    let _ = servient.step(&mut cx, &mut zero);
    assert_eq!(telemetry.snapshot(), ProbeSnapshot::default());

    drive_until(
        &mut servient,
        || telemetry.snapshot().permitted_accept_polls > 0,
        "permit-gated serving",
    );
    let serving = telemetry.snapshot();
    let artifact = serving
        .prepared_artifact
        .as_ref()
        .expect("compiler artifact reached preparation");
    assert_eq!(artifact.transport.as_ref(), "tcp");
    assert_eq!(artifact.authority.as_ref(), format!("127.0.0.1:{port}"));
    assert_eq!(artifact.key_expr.as_ref(), key_expr);
    assert_eq!(artifact.property.as_ref(), "status");
    assert_eq!(artifact.content_type.as_deref(), Some("application/json"));
    assert_eq!(artifact.subprotocol.as_deref(), Some("zenoh-query"));
    assert_eq!(artifact.form_index, 1);
    assert_eq!(serving.declarations_completed, 1);
    assert!(serving.readiness_polls >= 2);
    assert_eq!(serving.activations, 1);
    assert_eq!(serving.commits_closed, 1);

    let client = zenoh::open(client_config(&endpoint))
        .wait()
        .expect("open independent Zenoh requester session");
    let requester = client.clone();
    let request_key = key_expr.clone();
    let reply_thread = thread::spawn(move || {
        let replies = requester
            .get(request_key)
            .wait()
            .expect("send real Zenoh Property Read query");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("wait for real Zenoh Property Read reply")
            .expect("target route must send one reply")
            .into_result()
            .expect("target route returned an error reply")
    });
    drive_until(
        &mut servient,
        || telemetry.snapshot().responses_delivered == 1,
        "one target-SPI response delivery",
    );
    let reply = reply_thread.join().expect("join Zenoh requester");
    assert_eq!(
        reply.payload().to_bytes().as_ref(),
        br#"{\"status\":\"on\"}"#
    );
    assert_eq!(reply.encoding().to_string(), "application/json");
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        1
    );
    assert_eq!(telemetry.snapshot().correlations_accepted, vec![1]);

    servient
        .begin_destroy()
        .expect("Servient accepts route drain ownership");
    drive_until(
        &mut servient,
        || telemetry.snapshot().terminal_cleanups == 1,
        "Zenoh undeclare and session close",
    );
    let terminal = telemetry.snapshot();
    assert_eq!(terminal.shutdowns_started, 1);
    assert_eq!(terminal.undeclarations_completed, 1);
    assert_eq!(terminal.sessions_closed, 1);
    assert_eq!(terminal.terminal_cleanups, 1);
    assert_success_semantics(&terminal, binding_id);

    let replies = client
        .get(key_expr)
        .timeout(Duration::from_millis(250))
        .wait()
        .expect("query after target cleanup");
    assert!(
        !matches!(replies.recv_timeout(Duration::from_secs(1)), Ok(Some(_))),
        "undeclared route must not answer after terminal cleanup"
    );
    client.close().wait().expect("close requester session");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_target_readiness_failure_aborts_declared_relative_route() {
    zenoh::init_log_from_env_or("error");
    let port = unused_loopback_port();
    let endpoint = format!("tcp/127.0.0.1:{port}");
    let key_expr = format!("clinkz/probe/{port}/sensor-failure/properties/temperature");
    let binding_id = BindingId::new(0x7a02);
    let calls = Arc::new(Mutex::new(0));
    let handler = RecordingHandler {
        expected_thing: Box::from("urn:clinkz:probe:sensor-failure"),
        expected_property: Box::from("temperature"),
        expected_binding: binding_id,
        response: Payload::new(b"21".to_vec(), "text/plain"),
        calls: Arc::clone(&calls),
    };
    let (mut servient, telemetry) = build_servient(
        relative_thing(
            port,
            &format!("clinkz/probe/{port}/sensor-failure"),
            "urn:clinkz:probe:sensor-failure",
            "temperature",
        ),
        "temperature",
        port,
        binding_id,
        true,
        &handler,
    );
    drive_until(
        &mut servient,
        || telemetry.snapshot().declarations_completed == 1,
        "externally visible route before readiness",
    );
    let client = zenoh::open(client_config(&endpoint))
        .wait()
        .expect("open readiness-failure requester");
    let requester = client.clone();
    let reply_thread = thread::spawn(move || {
        let replies = requester
            .get(key_expr)
            .wait()
            .expect("query externally declared route before readiness failure");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("wait for readiness-failure drain reply")
            .expect("readiness failure returns an explicit terminal reply")
            .into_result()
    });
    wait_until(
        || telemetry.snapshot().queries_arrived == 1,
        "pre-readiness Zenoh query arrival",
    );
    drive_until(
        &mut servient,
        || telemetry.snapshot().terminal_cleanups == 1,
        "readiness failure rollback",
    );
    assert!(
        reply_thread
            .join()
            .expect("join readiness-failure requester")
            .is_err(),
        "readiness failure must be visible as a Zenoh error reply"
    );
    client
        .close()
        .wait()
        .expect("close readiness-failure requester");
    let snapshot = telemetry.snapshot();
    let artifact = snapshot
        .prepared_artifact
        .expect("relative target reached real compiler artifact");
    assert_eq!(artifact.form_index, 0);
    assert_eq!(artifact.property.as_ref(), "temperature");
    assert_eq!(artifact.content_type.as_deref(), Some("text/plain"));
    assert_eq!(snapshot.declarations_completed, 1);
    assert_eq!(snapshot.readiness_failures, 1);
    assert_eq!(snapshot.activations, 0);
    assert_eq!(snapshot.commits_closed, 0);
    assert_eq!(snapshot.aborts_started, 1);
    assert_eq!(snapshot.undeclarations_completed, 1);
    assert_eq!(snapshot.sessions_closed, 1);
    assert_eq!(snapshot.route_state_drops, 1);
    assert!(snapshot.prepared_state_address.is_some());
    assert!(snapshot.active_state_address.is_none());
    assert!(snapshot.committed_state_address.is_none());
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_target_prepublication_cancellation_drains_declared_route() {
    zenoh::init_log_from_env_or("error");
    let port = unused_loopback_port();
    let endpoint = format!("tcp/127.0.0.1:{port}");
    let key_expr = format!("clinkz/probe/{port}/sensor-cancel/properties/humidity");
    let binding_id = BindingId::new(0x7a03);
    let calls = Arc::new(Mutex::new(0));
    let handler = RecordingHandler {
        expected_thing: Box::from("urn:clinkz:probe:sensor-cancel"),
        expected_property: Box::from("humidity"),
        expected_binding: binding_id,
        response: Payload::new(b"40".to_vec(), "text/plain"),
        calls: Arc::clone(&calls),
    };
    let (mut servient, telemetry) = build_servient(
        relative_thing(
            port,
            &format!("clinkz/probe/{port}/sensor-cancel"),
            "urn:clinkz:probe:sensor-cancel",
            "humidity",
        ),
        "humidity",
        port,
        binding_id,
        false,
        &handler,
    );
    drive_until(
        &mut servient,
        || telemetry.snapshot().declarations_completed == 1,
        "externally visible prepared route",
    );
    let client = zenoh::open(client_config(&endpoint))
        .wait()
        .expect("open cancellation requester");
    let requester = client.clone();
    let reply_thread = thread::spawn(move || {
        let replies = requester
            .get(key_expr)
            .wait()
            .expect("query externally declared route before cancellation");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("wait for cancellation drain reply")
            .expect("cancellation returns an explicit terminal reply")
            .into_result()
    });
    wait_until(
        || telemetry.snapshot().queries_arrived == 1,
        "pre-publication Zenoh query arrival",
    );
    servient
        .begin_destroy()
        .expect("Servient accepts pre-publication cancellation");
    drive_until(
        &mut servient,
        || telemetry.snapshot().terminal_cleanups == 1,
        "cancelled route terminal cleanup",
    );
    assert!(
        reply_thread
            .join()
            .expect("join cancellation requester")
            .is_err(),
        "pre-publication cancellation must be visible as a Zenoh error reply"
    );
    client.close().wait().expect("close cancellation requester");
    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.readiness_polls, 0);
    assert_eq!(snapshot.activations, 0);
    assert_eq!(snapshot.commits_closed, 0);
    assert_eq!(snapshot.permitted_accept_polls, 0);
    assert_eq!(snapshot.aborts_started, 1);
    assert_eq!(snapshot.undeclarations_completed, 1);
    assert_eq!(snapshot.sessions_closed, 1);
    assert_eq!(snapshot.route_state_drops, 1);
    assert!(snapshot.prepared_state_address.is_some());
    assert!(snapshot.active_state_address.is_none());
    assert!(snapshot.committed_state_address.is_none());
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_target_host_erased_property_read_round_trip_and_terminal_drain() {
    zenoh::init_log_from_env_or("error");
    let port = unused_loopback_port();
    let endpoint = format!("tcp/127.0.0.1:{port}");
    let key_expr = format!("clinkz/probe/{port}/host-lamp/status");
    let binding_id = BindingId::new(0x7b01);
    let calls = Arc::new(Mutex::new(0));
    let handler = RecordingHandler {
        expected_thing: Box::from("urn:clinkz:probe:host-lamp"),
        expected_property: Box::from("status"),
        expected_binding: binding_id,
        response: Payload::new(br#"{\"status\":\"host-on\"}"#.to_vec(), "application/json"),
        calls: Arc::clone(&calls),
    };
    let td = Thing::builder("Host Probe Lamp")
        .id("urn:clinkz:probe:host-lamp")
        .nosec()
        .property(
            "status",
            PropertyAffordance::builder(DataSchema::string())
                .form(
                    Form::read_property(format!("zenoh+tcp://127.0.0.1:{port}/{key_expr}"))
                        .content_type("application/json")
                        .subprotocol("zenoh-query")
                        .build()
                        .expect("valid Host Zenoh form"),
                )
                .build()
                .expect("valid Host status property"),
        )
        .build()
        .expect("valid Host probe Thing");
    let (servient, exposed, telemetry) =
        build_host_servient(td, "status", port, binding_id, false, handler);

    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut zero = WorkBudget::new();
    let _ = servient.step(&mut cx, &mut zero);
    assert_eq!(telemetry.snapshot(), ProbeSnapshot::default());

    drive_host_until(
        &servient,
        || telemetry.snapshot().permitted_accept_polls > 0,
        "Host permit-gated serving",
    );
    let serving = telemetry.snapshot();
    let artifact = serving
        .prepared_artifact
        .as_ref()
        .expect("Host compiler artifact reached preparation");
    assert_eq!(artifact.key_expr.as_ref(), key_expr);
    assert_eq!(artifact.property.as_ref(), "status");
    assert_eq!(artifact.content_type.as_deref(), Some("application/json"));
    assert_eq!(artifact.subprotocol.as_deref(), Some("zenoh-query"));
    assert_eq!(serving.prepared_state_address, serving.active_state_address);
    assert_eq!(
        serving.prepared_state_address,
        serving.committed_state_address
    );
    assert_eq!(serving.prepared_footprint, serving.active_footprint);
    assert_eq!(serving.prepared_footprint, serving.committed_footprint);
    assert_eq!(serving.route_state_drops, 0);

    let client = zenoh::open(client_config(&endpoint))
        .wait()
        .expect("open Host Zenoh requester session");
    let requester = client.clone();
    let request_key = key_expr.clone();
    let reply_thread = thread::spawn(move || {
        let replies = requester
            .get(request_key)
            .wait()
            .expect("send real Host Zenoh Property Read query");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("wait for real Host Zenoh Property Read reply")
            .expect("Host target route must send one reply")
            .into_result()
            .expect("Host target route returned an error reply")
    });
    drive_host_until(
        &servient,
        || telemetry.snapshot().responses_delivered == 1,
        "one Host target-SPI response delivery",
    );
    let reply = reply_thread.join().expect("join Host Zenoh requester");
    assert_eq!(
        reply.payload().to_bytes().as_ref(),
        br#"{\"status\":\"host-on\"}"#
    );
    assert_eq!(reply.encoding().to_string(), "application/json");
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        1
    );

    exposed
        .begin_destroy()
        .expect("Host Servient accepts route drain ownership");
    drive_host_until(
        &servient,
        || telemetry.snapshot().terminal_cleanups == 1,
        "Host Zenoh undeclare and session close",
    );
    let terminal = telemetry.snapshot();
    assert_success_semantics(&terminal, binding_id);

    let replies = client
        .get(key_expr)
        .timeout(Duration::from_millis(250))
        .wait()
        .expect("query after Host target cleanup");
    assert!(
        !matches!(replies.recv_timeout(Duration::from_secs(1)), Ok(Some(_))),
        "Host route must not answer after terminal cleanup"
    );
    client.close().wait().expect("close Host requester session");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_target_host_erased_readiness_failure_aborts_declared_route() {
    zenoh::init_log_from_env_or("error");
    let port = unused_loopback_port();
    let endpoint = format!("tcp/127.0.0.1:{port}");
    let key_expr = format!("clinkz/probe/{port}/host-failure/properties/temperature");
    let binding_id = BindingId::new(0x7b02);
    let calls = Arc::new(Mutex::new(0));
    let handler = RecordingHandler {
        expected_thing: Box::from("urn:clinkz:probe:host-failure"),
        expected_property: Box::from("temperature"),
        expected_binding: binding_id,
        response: Payload::new(b"21".to_vec(), "text/plain"),
        calls: Arc::clone(&calls),
    };
    let (servient, _exposed, telemetry) = build_host_servient(
        relative_thing(
            port,
            &format!("clinkz/probe/{port}/host-failure"),
            "urn:clinkz:probe:host-failure",
            "temperature",
        ),
        "temperature",
        port,
        binding_id,
        true,
        handler,
    );
    drive_host_until(
        &servient,
        || telemetry.snapshot().declarations_completed == 1,
        "externally visible Host route before readiness",
    );
    assert_eq!(telemetry.snapshot().route_state_drops, 0);

    let client = zenoh::open(client_config(&endpoint))
        .wait()
        .expect("open Host readiness-failure requester");
    let requester = client.clone();
    let reply_thread = thread::spawn(move || {
        let replies = requester
            .get(key_expr)
            .wait()
            .expect("query Host route before readiness failure");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("wait for Host readiness-failure drain reply")
            .expect("Host readiness failure returns an explicit terminal reply")
            .into_result()
    });
    wait_until(
        || telemetry.snapshot().queries_arrived == 1,
        "pre-readiness Host Zenoh query arrival",
    );
    assert_eq!(telemetry.snapshot().route_state_drops, 0);
    drive_host_until(
        &servient,
        || telemetry.snapshot().terminal_cleanups == 1,
        "Host readiness failure rollback",
    );
    assert!(
        reply_thread
            .join()
            .expect("join Host readiness-failure requester")
            .is_err(),
        "Host readiness failure must be visible as a Zenoh error reply"
    );
    client
        .close()
        .wait()
        .expect("close Host readiness-failure requester");
    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.declarations_completed, 1);
    assert_eq!(snapshot.readiness_failures, 1);
    assert_eq!(snapshot.activations, 0);
    assert_eq!(snapshot.commits_closed, 0);
    assert_eq!(snapshot.aborts_started, 1);
    assert_eq!(snapshot.undeclarations_completed, 1);
    assert_eq!(snapshot.sessions_closed, 1);
    assert_eq!(snapshot.route_state_drops, 1);
    assert!(snapshot.prepared_state_address.is_some());
    assert!(snapshot.active_state_address.is_none());
    assert!(snapshot.committed_state_address.is_none());
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_target_host_erased_prepublication_cancellation_cleans_terminally() {
    zenoh::init_log_from_env_or("error");
    let port = unused_loopback_port();
    let endpoint = format!("tcp/127.0.0.1:{port}");
    let key_expr = format!("clinkz/probe/{port}/host-cancel/properties/humidity");
    let binding_id = BindingId::new(0x7b03);
    let calls = Arc::new(Mutex::new(0));
    let handler = RecordingHandler {
        expected_thing: Box::from("urn:clinkz:probe:host-cancel"),
        expected_property: Box::from("humidity"),
        expected_binding: binding_id,
        response: Payload::new(b"40".to_vec(), "text/plain"),
        calls: Arc::clone(&calls),
    };
    let (servient, exposed, telemetry) = build_host_servient(
        relative_thing(
            port,
            &format!("clinkz/probe/{port}/host-cancel"),
            "urn:clinkz:probe:host-cancel",
            "humidity",
        ),
        "humidity",
        port,
        binding_id,
        false,
        handler,
    );
    drive_host_until(
        &servient,
        || telemetry.snapshot().declarations_completed == 1,
        "externally visible prepared Host route",
    );
    assert_eq!(telemetry.snapshot().route_state_drops, 0);

    let client = zenoh::open(client_config(&endpoint))
        .wait()
        .expect("open Host cancellation requester");
    let requester = client.clone();
    let reply_thread = thread::spawn(move || {
        let replies = requester
            .get(key_expr)
            .wait()
            .expect("query Host route before cancellation");
        replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("wait for Host cancellation drain reply")
            .expect("Host cancellation returns an explicit terminal reply")
            .into_result()
    });
    wait_until(
        || telemetry.snapshot().queries_arrived == 1,
        "pre-publication Host Zenoh query arrival",
    );
    exposed
        .begin_destroy()
        .expect("Host Servient accepts pre-publication cancellation");
    assert_eq!(telemetry.snapshot().route_state_drops, 0);
    drive_host_until(
        &servient,
        || telemetry.snapshot().terminal_cleanups == 1,
        "cancelled Host route terminal cleanup",
    );
    assert!(
        reply_thread
            .join()
            .expect("join Host cancellation requester")
            .is_err(),
        "Host pre-publication cancellation must be visible as a Zenoh error reply"
    );
    client
        .close()
        .wait()
        .expect("close Host cancellation requester");
    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.readiness_polls, 0);
    assert_eq!(snapshot.activations, 0);
    assert_eq!(snapshot.commits_closed, 0);
    assert_eq!(snapshot.permitted_accept_polls, 0);
    assert_eq!(snapshot.aborts_started, 1);
    assert_eq!(snapshot.undeclarations_completed, 1);
    assert_eq!(snapshot.sessions_closed, 1);
    assert_eq!(snapshot.route_state_drops, 1);
    assert!(snapshot.prepared_state_address.is_some());
    assert!(snapshot.active_state_address.is_none());
    assert!(snapshot.committed_state_address.is_none());
    assert_eq!(
        *calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
        0
    );
}
