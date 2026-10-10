//! External completion evidence using only the public production Core surface.
#![allow(unused_variables)]
#[cfg(feature = "std")]
use clinkz_wot_core::binding::ClientBinding;
use clinkz_wot_core::*;
#[cfg(feature = "std")]
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

fn test_error() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

struct Endpoint<C> {
    marker: PhantomData<C>,
    compatibility: BindingArtifactCompatibility,
    drops: Arc<AtomicUsize>,
}
impl<C> Drop for Endpoint<C> {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl<C> Endpoint<C> {
    fn new(compatibility: BindingArtifactCompatibility, drops: &Arc<AtomicUsize>) -> Self {
        Self {
            marker: PhantomData,
            compatibility,
            drops: drops.clone(),
        }
    }
}

impl<C: BindingCompilerExtension> PollServerBinding for Endpoint<C> {
    type Compiler = C;
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
        input: PrepareInput,
        artifact: &BindingArtifactEnvelope<<Self::Compiler as BindingCompilerExtension>::Artifact>,
        route: &mut ServerRouteSlot<Self::RouteState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<RoutePrepareOutcome<()>>, BindingInputRejection<PrepareInput>> {
        Err(BindingInputRejection::new(
            input,
            BindingOperationalError::new(test_error()),
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
            BindingOperationalError::new(test_error()),
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
        self.compatibility
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
            BindingOperationalError::new(test_error()),
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
        Err(BindingInputRejection::new(
            input,
            BindingOperationalError::new(test_error()),
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
            BindingOperationalError::new(test_error()),
        ))
    }
}

#[cfg(feature = "std")]
impl<C: BindingCompilerExtension + Send + Sync> ClientBinding for Endpoint<C> {
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
        Err(BindingInputRejection::new(
            request,
            BindingOperationalError::new(test_error()),
        ))
    }
}

// The observer is local to this test binary and thread. It changes no process
// policy in production and performs no allocation or formatting in its hooks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Request {
    address: usize,
    size: usize,
    align: usize,
}
#[derive(Clone, Copy, Debug)]
struct Observation {
    active: bool,
    fail_next: bool,
    allocations: usize,
    releases: usize,
    allocated: [Request; 16],
    released: [Request; 16],
}
const EMPTY_OBSERVATION: Observation = Observation {
    active: false,
    fail_next: false,
    allocations: 0,
    releases: 0,
    allocated: [Request {
        address: 0,
        size: 0,
        align: 0,
    }; 16],
    released: [Request {
        address: 0,
        size: 0,
        align: 0,
    }; 16],
};
std::thread_local! { static OBSERVATION: Cell<Observation> = const { Cell::new(EMPTY_OBSERVATION) }; }
struct Observer;
#[global_allocator]
static ALLOCATOR: Observer = Observer;
// SAFETY: System receives each unchanged Layout and pointer. The only deviation
// is an explicitly armed null return at a known fallible production acquisition.
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = OBSERVATION
            .try_with(|cell| {
                let mut state = cell.get();
                let fail = state.active && state.fail_next;
                state.fail_next = false;
                cell.set(state);
                fail
            })
            .unwrap_or(false);
        let pointer = if fail {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(layout) }
        };
        let _ = OBSERVATION.try_with(|cell| {
            let mut state = cell.get();
            if state.active {
                if state.allocations < state.allocated.len() {
                    state.allocated[state.allocations] = Request {
                        address: pointer as usize,
                        size: layout.size(),
                        align: layout.align(),
                    };
                }
                state.allocations += 1;
                cell.set(state);
            }
        });
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let _ = OBSERVATION.try_with(|cell| {
            let mut state = cell.get();
            if state.active {
                if state.releases < state.released.len() {
                    state.released[state.releases] = Request {
                        address: pointer as usize,
                        size: layout.size(),
                        align: layout.align(),
                    };
                }
                state.releases += 1;
                cell.set(state);
            }
        });
        unsafe { System.dealloc(pointer, layout) }
    }
}
fn observe<T>(fail_next: bool, f: impl FnOnce() -> T) -> (T, Observation) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            OBSERVATION.with(|cell| {
                let mut state = cell.get();
                state.active = false;
                cell.set(state);
            });
        }
    }
    OBSERVATION.with(|cell| {
        assert!(!cell.get().active);
        cell.set(Observation {
            active: true,
            fail_next,
            ..EMPTY_OBSERVATION
        });
    });
    let reset = Reset;
    let value = f();
    drop(reset);
    let state = OBSERVATION.with(Cell::get);
    assert!(
        !state.fail_next,
        "the intended allocation site was not reached"
    );
    assert!(state.allocations <= 16 && state.releases <= 16);
    (value, state)
}
fn no_requests(state: Observation) {
    assert_eq!((state.allocations, state.releases), (0, 0));
}
#[cfg(feature = "std")]
fn acquired(state: Observation, layout: Layout, success: bool) -> Request {
    assert_eq!((state.allocations, state.releases), (1, 0));
    let request = state.allocated[0];
    assert_eq!(
        (request.size, request.align),
        (layout.size(), layout.align())
    );
    assert_eq!(request.address != 0, success);
    request
}
#[cfg(feature = "std")]
fn released(state: Observation, request: Request) {
    assert_eq!((state.allocations, state.releases), (0, 1));
    assert_eq!(state.released[0], request);
}
fn work(units: u64) -> WorkBudget {
    WorkBudget::new().with_remaining(WorkClass::BindingPolls, units)
}
fn identity(compiler: &ResolvedTargetCompiler, variant: u32) -> BindingRegistrationIdentity {
    let mut generation = BindingGeneration::INITIAL;
    for _ in 0..variant {
        generation = generation.checked_next().unwrap();
    }
    BindingRegistrationIdentity::new(
        BindingId::new(201 + variant),
        generation,
        compiler.configuration(),
        compiler.compatibility(),
        13 + variant,
    )
}
fn plan(target: &str) -> LogicalInteractionPlan {
    LogicalInteractionPlan::try_property_read(
        PlanId::new(
            SlotIndex::new(23),
            Generation::INITIAL.checked_next().unwrap(),
        ),
        ThingId::from("urn:production:compiler"),
        "temperature".into(),
        1,
        target.into(),
        None,
        None,
    )
    .unwrap()
}
fn candidate(id: BindingRegistrationIdentity) -> BindingCandidate {
    BindingCandidate::new(
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        id.diagnostic_ordinal(),
        19,
    )
}
fn artifact_identity(
    id: BindingRegistrationIdentity,
    plan: &LogicalInteractionPlan,
) -> BindingArtifactIdentity {
    BindingArtifactIdentity::new(
        PlanSetGeneration::INITIAL.checked_next().unwrap(),
        plan.plan_id(),
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        BindingArtifactRole::ConsumerCall,
    )
}
fn resources() -> BindingResourceDeclarations {
    BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(8, 4096),
    )
}
type Complete<C> = StaticBindingRegistration<StaticBindingComponents<Endpoint<C>, Endpoint<C>>>;
fn static_registration<C: BindingCompilerExtension>(
    compiler: C,
    id: BindingRegistrationIdentity,
    consumer: bool,
    execution: BindingExecutionSupport,
    drops: &Arc<AtomicUsize>,
) -> Result<
    Complete<C>,
    BindingInputRejection<
        StaticBindingRegistrationInput<StaticBindingComponents<Endpoint<C>, Endpoint<C>>>,
    >,
> {
    let components = StaticBindingComponents::new(
        Endpoint::new(id.artifact_compatibility(), drops),
        Endpoint::new(id.artifact_compatibility(), drops),
    );
    let input = if consumer {
        StaticBindingRegistrationInput::producer_and_consumer_property_read(
            id,
            execution,
            StaticBindingCompilerRegistration::new(compiler),
            components,
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(3, 5),
        )
    } else {
        StaticBindingRegistrationInput::new(
            id,
            BindingRegistrationCapabilities::producer_property_read(),
            execution,
            StaticBindingCompilerRegistration::new(compiler),
            components,
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(3, 5),
        )
    };
    if consumer {
        StaticBindingRegistration::producer_and_consumer_property_read(input)
    } else {
        StaticBindingRegistration::new(input)
    }
}
fn typed(
    compiler: ResolvedTargetCompiler,
    id: BindingRegistrationIdentity,
    drops: &Arc<AtomicUsize>,
) -> Complete<ResolvedTargetCompiler> {
    static_registration(
        compiler,
        id,
        true,
        BindingExecutionSupport::application_static(),
        drops,
    )
    .unwrap_or_else(|_| panic!("complete typed construction"))
}
#[cfg(feature = "std")]
fn host_registration(
    compiler: HostBindingCompilerRegistration,
    id: BindingRegistrationIdentity,
    consumer: bool,
    execution: BindingExecutionSupport,
    drops: &Arc<AtomicUsize>,
) -> Result<HostBindingRegistration, BindingInputRejection<HostBindingRegistrationInput>> {
    let server = Box::new(Endpoint::<ResolvedTargetCompiler>::new(
        id.artifact_compatibility(),
        drops,
    ));
    let input = if consumer {
        HostBindingRegistrationInput::producer_and_consumer_property_read(
            id,
            execution,
            compiler,
            server,
            Box::new(Endpoint::<ResolvedTargetCompiler>::new(
                id.artifact_compatibility(),
                drops,
            )),
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(3, 5),
        )
    } else {
        HostBindingRegistrationInput::new(
            id,
            BindingRegistrationCapabilities::producer_property_read(),
            execution,
            compiler,
            server,
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(3, 5),
        )
    };
    HostBindingRegistration::new(input)
}
#[cfg(feature = "std")]
fn host(
    compiler: ResolvedTargetCompiler,
    id: BindingRegistrationIdentity,
    drops: &Arc<AtomicUsize>,
) -> HostBindingRegistration {
    host_registration(
        HostBindingCompilerRegistration::try_new_consumer(compiler).unwrap(),
        id,
        true,
        BindingExecutionSupport::host_erased(),
        drops,
    )
    .unwrap_or_else(|_| panic!("complete Host construction"))
}
fn check_descriptor(support: &ConsumerCompilerSupport, capacity: usize, host: bool) {
    assert_eq!(support.target_capacity(), capacity);
    assert_eq!(
        support.cursor_layout(),
        Layout::new::<ResolvedTargetCompilerCursor>()
    );
    assert_eq!(
        support.artifact_layout(),
        Layout::new::<ResolvedTargetArtifact>()
    );
    assert_eq!(
        support.output_layout(),
        Layout::new::<BindingCompilerOutput<ResolvedTargetArtifact>>()
    );
    type NativeTemporary = (
        ResolvedTargetCompilerCursor,
        BindingCompilerStep<ResolvedTargetCompilerCursor, ResolvedTargetArtifact>,
        BindingCompilerOutput<ResolvedTargetArtifact>,
        ResolvedTargetArtifact,
        CoreError,
    );
    #[allow(unused_mut)]
    let mut temporary = Layout::new::<NativeTemporary>();
    #[cfg(feature = "std")]
    if host {
        temporary = Layout::new::<(
            NativeTemporary,
            BindingCompilerStep<HostBindingCompilerCursor, HostBindingArtifact>,
        )>();
    }
    assert_eq!(support.temporary_layout(), temporary);
    assert_eq!(
        (
            support.compatibility_work(),
            support.bounds_work(),
            support.start_work(),
            support.abort_work(),
            support.destruction_work()
        ),
        (1, 1, 1, 1, 1)
    );
    assert_eq!(
        (
            support.step_work(),
            support.allocation_work(),
            support.release_work()
        ),
        (2 + u64::from(host), u64::from(host), u64::from(host))
    );
}

#[test]
fn typed_complete_owner_and_nondefault_identity_survive_actual_source_destruction() {
    for (capacity, variant, target) in [
        (32, 2, "mock://s/温度"),
        (
            64,
            5,
            "x:abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz0123456789",
        ),
    ] {
        let drops = Arc::new(AtomicUsize::new(0));
        let compiler = ResolvedTargetCompiler::try_new(capacity).unwrap();
        let id = identity(&compiler, variant);
        let complete = typed(compiler, id, &drops);
        let (checked, requests) = observe(false, || {
            let checked = complete
                .try_into_consumer_compiler()
                .unwrap_or_else(|_| panic!("typed capture"));
            check_descriptor(checked.support(), capacity, false);
            assert_eq!(checked.registration().identity(), id);
            assert_eq!(checked.registration().resources(), resources());
            assert!(
                checked
                    .registration()
                    .capabilities()
                    .supports_consumer_property_read()
            );
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            checked
        });
        no_requests(requests);
        let source = plan(target);
        let input =
            BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
        let compiler = checked.registration().compiler().compiler();
        let ((bounds, cursor), requests) = observe(false, || {
            (
                compiler.bounds(&input).unwrap(),
                compiler.start(&input).unwrap(),
            )
        });
        no_requests(requests);
        assert_eq!(
            bounds.artifact().retained_bytes(),
            checked.support().artifact_layout().size() as u64
        );
        assert_eq!(
            bounds.cursor_bytes(),
            checked.support().cursor_layout().size() as u64
        );
        assert_eq!(
            bounds.temporary_bytes(),
            checked.support().temporary_layout().size() as u64
        );
        let mut cursor = cursor;
        for _ in 0..4 {
            for units in [0, 1] {
                let mut budget = work(units);
                let (step, requests) =
                    observe(false, || compiler.step(&input, cursor, &mut budget));
                no_requests(requests);
                assert_eq!(budget.remaining(WorkClass::BindingPolls), units);
                let BindingCompilerStep::Pending(returned) = step else {
                    panic!("unpaid Pending")
                };
                cursor = returned;
            }
        }
        let mut budget = bounds.into_work();
        let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut budget));
        no_requests(requests);
        let BindingCompilerStep::Pending(cursor) = step else {
            panic!("paid Pending")
        };
        let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut budget));
        no_requests(requests);
        assert_eq!(budget.remaining(WorkClass::BindingPolls), 0);
        let BindingCompilerStep::Complete(output) = step else {
            panic!("Complete")
        };
        let full_identity = artifact_identity(id, &source);
        let envelope = BindingArtifactEnvelope::try_new(
            full_identity,
            output.artifact().footprint(),
            output.into_artifact(),
        )
        .unwrap();
        drop(checked);
        drop(source);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        assert_eq!(envelope.identity(), full_identity);
        assert_eq!(envelope.artifact().payload().target(), target);
        assert_eq!(envelope.route_reservation(), None);
        let ((), requests) = observe(false, || drop(envelope));
        no_requests(requests);
    }
}

#[test]
#[cfg(feature = "std")]
fn public_host_complete_owner_retains_one_actual_slot_through_completion() {
    for (capacity, variant) in [(32, 2), (64, 5)] {
        let drops = Arc::new(AtomicUsize::new(0));
        let compiler = ResolvedTargetCompiler::try_new(capacity).unwrap();
        let id = identity(&compiler, variant);
        let complete = host(compiler, id, &drops);
        let (checked, requests) = observe(false, || {
            complete
                .try_into_consumer_compiler()
                .unwrap_or_else(|_| panic!("Host capture"))
        });
        no_requests(requests);
        check_descriptor(checked.support(), capacity, true);
        assert_eq!(checked.registration().identity(), id);
        let layout = checked.support().host_slot_layout().unwrap();
        let temporary = checked.support().temporary_layout();
        assert_eq!(
            checked.support().host_adapter_layout(),
            Some(Layout::new::<ResolvedTargetCompiler>())
        );
        assert_eq!(
            checked.support().host_cursor_owner_layout(),
            Some(Layout::new::<HostBindingCompilerCursor>())
        );
        assert_eq!(
            checked.support().host_output_owner_layout(),
            Some(Layout::new::<BindingCompilerOutput<HostBindingArtifact>>())
        );
        let source = plan("mock://s/温度");
        let input =
            BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
        let compiler = checked.registration().compiler();
        let (bounds, requests) = observe(false, || compiler.bounds(&input).unwrap());
        no_requests(requests);
        assert_eq!(bounds.temporary_bytes(), temporary.size() as u64);
        assert_eq!(
            bounds.cursor_bytes(),
            checked.support().cursor_layout().size() as u64
        );
        assert_eq!(
            bounds.artifact().retained_bytes(),
            checked.support().artifact_layout().size() as u64
        );
        let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
        let request = acquired(requests, layout, true);
        let mut cursor = cursor;
        for _ in 0..4 {
            for units in [0, 1, 2] {
                let mut budget = work(units);
                let (step, requests) =
                    observe(false, || compiler.step(&input, cursor, &mut budget));
                no_requests(requests);
                assert_eq!(budget.remaining(WorkClass::BindingPolls), units);
                let BindingCompilerStep::Pending(returned) = step else {
                    panic!("unpaid Host Pending")
                };
                cursor = returned;
            }
        }
        let mut budget = bounds.into_work();
        let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut budget));
        no_requests(requests);
        let BindingCompilerStep::Pending(cursor) = step else {
            panic!("paid Host Pending")
        };
        let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut budget));
        no_requests(requests);
        assert_eq!(budget.remaining(WorkClass::BindingPolls), 0);
        let BindingCompilerStep::Complete(output) = step else {
            panic!("Host Complete")
        };
        let full_identity = artifact_identity(id, &source);
        let envelope = BindingArtifactEnvelope::try_new(
            full_identity,
            output.artifact().footprint(),
            output.into_artifact(),
        )
        .unwrap();
        drop(checked);
        drop(source);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        assert_eq!(envelope.identity(), full_identity);
        let artifact = envelope.into_artifact();
        let payload = artifact
            .try_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
            .unwrap();
        assert_eq!(payload.target(), "mock://s/温度");
        let address = payload as *const ResolvedTargetArtifact as usize;
        assert!(
            address >= request.address
                && address + std::mem::size_of_val(payload) <= request.address + request.size
        );
        assert_eq!(artifact.route_reservation(), None);
        assert!(
            artifact
                .try_payload::<u8>(id.artifact_compatibility())
                .is_none()
        );
        assert!(
            artifact
                .try_payload::<ResolvedTargetArtifact>(BindingArtifactCompatibility::new([0; 16]))
                .is_none()
        );
        let (artifact, requests) = observe(false, || {
            artifact
                .try_into_payload::<ResolvedTargetArtifact>(BindingArtifactCompatibility::new(
                    [0; 16],
                ))
                .unwrap_err()
        });
        no_requests(requests);
        let (artifact, requests) = observe(false, || {
            artifact
                .try_into_payload::<u8>(id.artifact_compatibility())
                .unwrap_err()
        });
        no_requests(requests);
        assert_eq!(artifact.compatibility(), id.artifact_compatibility());
        let (payload, requests) = observe(false, || {
            artifact
                .try_into_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                .unwrap()
        });
        released(requests, request);
        assert_eq!(payload.target(), "mock://s/温度");
        let ((), requests) = observe(false, || drop(payload));
        no_requests(requests);
        println!("production Host: slot=({}, {}), cursor owner=({}, {}), output owner=({}, {}), payload={}, native cursor={}, native output={}, native result={}, declared Host temporary={}", layout.size(), layout.align(), Layout::new::<HostBindingCompilerCursor>().size(), Layout::new::<HostBindingCompilerCursor>().align(), Layout::new::<BindingCompilerOutput<HostBindingArtifact>>().size(), Layout::new::<BindingCompilerOutput<HostBindingArtifact>>().align(), Layout::new::<ResolvedTargetArtifact>().size(), Layout::new::<ResolvedTargetCompilerCursor>().size(), Layout::new::<BindingCompilerOutput<ResolvedTargetArtifact>>().size(), Layout::new::<BindingCompilerStep<ResolvedTargetCompilerCursor, ResolvedTargetArtifact>>().size(), temporary.size());
    }
}

#[derive(Debug)]
struct ForeignCompiler(Arc<AtomicUsize>);
impl BindingCompilerExtension for ForeignCompiler {
    // Same associated types and compatibility do not establish source support.
    type Cursor = ResolvedTargetCompilerCursor;
    type Artifact = ResolvedTargetArtifact;
    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.0.fetch_add(1, Ordering::SeqCst);
        ResolvedTargetCompiler::try_new(64).unwrap().compatibility()
    }
    fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
        panic!("foreign bounds callback reached")
    }
    fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<Self::Cursor> {
        panic!("foreign start callback reached")
    }
    fn step(
        &self,
        _: &BindingCompilerInput<'_>,
        _: Self::Cursor,
        _: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        panic!("foreign step callback reached")
    }
    fn abort(&self, _: Self::Cursor) {
        panic!("foreign abort callback reached")
    }
}

#[test]
fn capture_rejects_foreign_configuration_and_role_without_losing_complete_owners() {
    assert!(ResolvedTargetCompiler::try_new(0).is_err());
    assert!(ResolvedTargetCompiler::try_new(65).is_err());
    assert!(ResolvedTargetCompiler::try_new(usize::MAX).is_err());
    assert!(ResolvedTargetCompiler::try_new(1).is_ok());
    let drops = Arc::new(AtomicUsize::new(0));
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler, 3);
    let callbacks = Arc::new(AtomicUsize::new(0));
    let foreign = static_registration(
        ForeignCompiler(callbacks.clone()),
        id,
        true,
        BindingExecutionSupport::application_static(),
        &drops,
    )
    .unwrap_or_else(|_| panic!("valid foreign registration"));
    assert_eq!(callbacks.load(Ordering::SeqCst), 1); // Startup compatibility only.
    let (rejection, requests) = observe(false, || {
        foreign
            .try_into_consumer_compiler()
            .err()
            .expect("foreign must be unsupported")
    });
    no_requests(requests);
    assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    assert_eq!(rejection.input().identity(), id);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let foreign = rejection.into_input();
    assert_eq!(foreign.resources(), resources());
    drop(foreign);
    assert_eq!(drops.load(Ordering::SeqCst), 2);

    for consumer in [true, false] {
        let compiler = ResolvedTargetCompiler::try_new(if consumer { 32 } else { 64 }).unwrap();
        let complete = static_registration(
            compiler,
            id,
            consumer,
            BindingExecutionSupport::application_static(),
            &drops,
        )
        .unwrap_or_else(|_| panic!("startup-valid registration"));
        let before = drops.load(Ordering::SeqCst);
        let (rejection, requests) = observe(false, || {
            complete
                .try_into_consumer_compiler()
                .err()
                .expect("configuration/capability mismatch")
        });
        no_requests(requests);
        assert_eq!(rejection.input().identity(), id);
        assert_eq!(drops.load(Ordering::SeqCst), before);
        let original = rejection.into_input();
        assert_eq!(
            original.capabilities().supports_consumer_property_read(),
            consumer
        );
        drop(original);
        assert_eq!(drops.load(Ordering::SeqCst), before + 2);
    }
    assert!(
        static_registration(
            ResolvedTargetCompiler::try_new(64).unwrap(),
            id,
            true,
            BindingExecutionSupport::host_erased(),
            &drops
        )
        .is_err()
    );
    let wrong_compatibility = BindingRegistrationIdentity::new(
        id.binding_id(),
        id.binding_generation(),
        id.configuration(),
        BindingArtifactCompatibility::new([9; 16]),
        id.diagnostic_ordinal(),
    );
    let rejection = static_registration(
        ResolvedTargetCompiler::try_new(64).unwrap(),
        wrong_compatibility,
        true,
        BindingExecutionSupport::application_static(),
        &drops,
    )
    .err()
    .unwrap();
    assert_eq!(rejection.input().identity(), wrong_compatibility);
}

#[test]
#[cfg(feature = "std")]
fn public_host_capture_requires_private_supported_kind_and_matching_actual_configuration() {
    let drops = Arc::new(AtomicUsize::new(0));
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler, 4);
    let callbacks = Arc::new(AtomicUsize::new(0));
    let foreign = HostBindingCompilerRegistration::new(ForeignCompiler(callbacks.clone()));
    let complete = host_registration(
        foreign,
        id,
        true,
        BindingExecutionSupport::host_erased(),
        &drops,
    )
    .unwrap_or_else(|_| panic!("foreign complete Host"));
    let (rejection, requests) = observe(false, || {
        complete
            .try_into_consumer_compiler()
            .err()
            .expect("foreign Host rejected")
    });
    no_requests(requests);
    assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    assert_eq!(rejection.input().identity(), id);
    drop(rejection.into_input());
    for (compiler, consumer) in [
        (HostBindingCompilerRegistration::new(compiler), true),
        (
            HostBindingCompilerRegistration::try_new_consumer(
                ResolvedTargetCompiler::try_new(32).unwrap(),
            )
            .unwrap(),
            true,
        ),
        (
            HostBindingCompilerRegistration::try_new_consumer(
                ResolvedTargetCompiler::try_new(64).unwrap(),
            )
            .unwrap(),
            false,
        ),
    ] {
        let complete = host_registration(
            compiler,
            id,
            consumer,
            BindingExecutionSupport::host_erased(),
            &drops,
        )
        .unwrap_or_else(|_| panic!("startup-valid Host"));
        let before = drops.load(Ordering::SeqCst);
        let (rejection, requests) = observe(false, || {
            complete
                .try_into_consumer_compiler()
                .err()
                .expect("Host support must reject")
        });
        no_requests(requests);
        assert_eq!(drops.load(Ordering::SeqCst), before);
        assert_eq!(rejection.input().identity(), id);
        let original = rejection.into_input();
        assert_eq!(
            original.capabilities().supports_consumer_property_read(),
            consumer
        );
        drop(original);
        assert_eq!(
            drops.load(Ordering::SeqCst),
            before + 1 + usize::from(consumer)
        );
    }
    assert!(
        host_registration(
            HostBindingCompilerRegistration::try_new_consumer(
                ResolvedTargetCompiler::try_new(64).unwrap()
            )
            .unwrap(),
            id,
            true,
            BindingExecutionSupport::application_static(),
            &drops
        )
        .is_err()
    );
}

// Selection of the expected complete owner remains a caller obligation. These
// checks use only the existing identity and read-only projections; they are a
// construction discriminator, not an implementation of the future aggregate.
fn matches(
    id: BindingRegistrationIdentity,
    expected: BindingRegistrationIdentity,
    input: &BindingCompilerInput<'_>,
) -> bool {
    id == expected
        && input.role() == BindingArtifactRole::ConsumerCall
        && id.binding_id() == input.candidate().binding_id()
        && id.binding_generation() == input.candidate().binding_generation()
        && id.configuration() == input.candidate().configuration()
        && id.artifact_compatibility() == input.candidate().compatibility()
        && id.diagnostic_ordinal() == input.candidate().registration_ordinal()
}
#[test]
fn qualified_owner_mismatches_are_detectable_before_preparation_and_capture_is_consuming() {
    let drops = Arc::new(AtomicUsize::new(0));
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler, 3);
    let complete = typed(compiler, id, &drops);
    let checked = complete
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("capture"));
    #[cfg(feature = "std")]
    let checked_host = host(ResolvedTargetCompiler::try_new(64).unwrap(), id, &drops)
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("Host capture"));
    let source = plan("mock://s/temperature");
    let input =
        BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
    let mismatches = [
        BindingRegistrationIdentity::new(
            BindingId::new(999),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            id.diagnostic_ordinal(),
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation().checked_next().unwrap(),
            id.configuration(),
            id.artifact_compatibility(),
            id.diagnostic_ordinal(),
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation(),
            BindingConfigurationDigest::new([9; 32]),
            id.artifact_compatibility(),
            id.diagnostic_ordinal(),
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            BindingArtifactCompatibility::new([9; 16]),
            id.diagnostic_ordinal(),
        ),
        BindingRegistrationIdentity::new(
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            id.diagnostic_ordinal() + 1,
        ),
    ];
    for expected in mismatches {
        let (accepted, requests) = observe(false, || {
            matches(checked.registration().identity(), expected, &input)
        });
        no_requests(requests);
        assert!(!accepted);
        #[cfg(feature = "std")]
        {
            let (accepted, requests) = observe(false, || {
                matches(checked_host.registration().identity(), expected, &input)
            });
            no_requests(requests);
            assert!(!accepted);
        }
        let foreign_input = BindingCompilerInput::new(
            &source,
            candidate(expected),
            BindingArtifactRole::ConsumerCall,
        );
        assert!(!matches(
            checked.registration().identity(),
            id,
            &foreign_input
        ));
    }
    for role in [
        BindingArtifactRole::ProducerRoute,
        BindingArtifactRole::ProducerPublication,
        BindingArtifactRole::ConsumerSubscription,
    ] {
        assert!(!matches(
            id,
            id,
            &BindingCompilerInput::new(&source, candidate(id), role)
        ));
    }
    // Returning the complete owner consumes support; the rustdoc negative proves
    // the previous support loan cannot survive this move or be reused.
    let original = checked.into_registration();
    assert_eq!(original.identity(), id);
    let checked = original
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("recapture original"));
    assert_eq!(checked.registration().identity(), id);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
}

#[test]
fn native_failed_coordinate_retries_never_refill_lifetime_work() {
    let compiler = ResolvedTargetCompiler::try_new(32).unwrap();
    let id = identity(&compiler, 2);
    let source = plan("mock://s/temperature");
    let input =
        BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
    let other = plan("mock://s/other");
    let changed = BindingCompilerInput::new(
        &other,
        BindingCandidate::new(
            id.binding_id(),
            id.binding_generation(),
            id.configuration(),
            id.artifact_compatibility(),
            id.diagnostic_ordinal(),
            99,
        ),
        BindingArtifactRole::ConsumerCall,
    );
    let mut cursor = compiler.start(&input).unwrap();
    for attempt in 0..5 {
        let mut budget = work(2);
        let (step, requests) = observe(false, || compiler.step(&changed, cursor, &mut budget));
        no_requests(requests);
        let BindingCompilerStep::Failed(failure) = step else {
            panic!("wrong coordinate must fail")
        };
        cursor = failure.into_parts().1;
        assert_eq!(
            budget.remaining(WorkClass::BindingPolls),
            if attempt < 2 { 0 } else { 2 }
        );
    }
    let ((), requests) = observe(false, || compiler.abort(cursor));
    no_requests(requests);
}

#[test]
#[cfg(feature = "std")]
fn every_new_actual_allocation_can_fail_and_slot_rollback_retains_ledger_history() {
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler, 3);
    let (rejection, requests) = observe(true, || {
        HostBindingCompilerRegistration::try_new_consumer(compiler).unwrap_err()
    });
    acquired(requests, Layout::new::<ResolvedTargetCompiler>(), false);
    assert_eq!(rejection.input().configuration(), id.configuration());
    let (adapter, requests) = observe(false, || {
        HostBindingCompilerRegistration::try_new_consumer(rejection.into_input()).unwrap()
    });
    let startup = acquired(requests, Layout::new::<ResolvedTargetCompiler>(), true);
    let drops = Arc::new(AtomicUsize::new(0));
    let complete = host_registration(
        adapter,
        id,
        true,
        BindingExecutionSupport::host_erased(),
        &drops,
    )
    .unwrap_or_else(|_| panic!("Host"));
    let checked = complete
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("capture"));
    let layout = checked.support().host_slot_layout().unwrap();
    let source = plan("mock://s/temperature");
    let input =
        BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
    let mut ledger =
        AdmissionLedger::new(SlotIndex::new(41), Generation::INITIAL, 0, 0, 0, 4096, 0, 0);
    ledger
        .try_reserve_persistent_runtime(ResourceKind::CompiledRuntimeBytesPerThingMax, 7)
        .unwrap()
        .commit();
    assert_eq!(
        (
            ledger.live_bytes(),
            ledger.peak_live_bytes(),
            ledger.largest_contiguous_allocation()
        ),
        (7, 7, 7)
    );
    // The caller owns logical admission. Core acquires physical backing only.
    let reservation = ledger
        .try_reserve_persistent_runtime(
            ResourceKind::CompiledRuntimeBytesPerThingMax,
            layout.size() as u64,
        )
        .unwrap();
    let (failure, requests) = observe(true, || checked.registration().compiler().start(&input));
    acquired(requests, layout, false);
    assert!(matches!(failure, Err(CoreError::Backpressure(_))));
    drop(reservation);
    assert_eq!(
        (
            ledger.live_bytes(),
            ledger.peak_live_bytes(),
            ledger.largest_contiguous_allocation()
        ),
        (7, 7 + layout.size() as u64, layout.size() as u64)
    );
    println!(
        "production slot rollback: (live, peak, largest) = ({}, {}, {}); no physical backing acquired",
        ledger.live_bytes(),
        ledger.peak_live_bytes(),
        ledger.largest_contiguous_allocation()
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(checked.registration().identity(), id);
    let reservation = ledger
        .try_reserve_persistent_runtime(
            ResourceKind::CompiledRuntimeBytesPerThingMax,
            layout.size() as u64,
        )
        .unwrap();
    let (held, requests) = observe(false, || {
        checked.registration().compiler().start(&input).unwrap()
    });
    let first = acquired(requests, layout, true);
    reservation.commit();
    let live_before = ledger.live_bytes();
    let reservation = ledger
        .try_reserve_persistent_runtime(
            ResourceKind::CompiledRuntimeBytesPerThingMax,
            layout.size() as u64,
        )
        .unwrap();
    let (failure, requests) = observe(true, || checked.registration().compiler().start(&input));
    acquired(requests, layout, false);
    assert!(failure.is_err());
    drop(reservation);
    assert_eq!(ledger.live_bytes(), live_before);
    assert_eq!(ledger.peak_live_bytes(), live_before + layout.size() as u64);
    let (result, requests) = observe(false, || checked.registration().compiler().abort(held));
    assert!(result.is_ok());
    released(requests, first);
    assert!(ledger.release_persistent_runtime(layout.size() as u64));
    assert_eq!(ledger.live_bytes(), 7);
    // A native start failure releases the already acquired vacant slot.
    let invalid_source = plan(&"x".repeat(65));
    let invalid = BindingCompilerInput::new(
        &invalid_source,
        candidate(id),
        BindingArtifactRole::ConsumerCall,
    );
    let (failure, requests) = observe(false, || checked.registration().compiler().start(&invalid));
    assert!(failure.is_err());
    assert_eq!((requests.allocations, requests.releases), (1, 1));
    assert_eq!(requests.allocated[0], requests.released[0]);
    assert_eq!(requests.allocated[0].size, layout.size());
    // Recovering the complete owner and destroying it releases its startup
    // adapter exactly once; endpoint allocations are baseline, separately owned.
    let complete = checked.into_registration();
    let ((), requests) = observe(false, || drop(complete));
    assert_eq!(requests.allocations, 0);
    assert_eq!(
        requests.released[..requests.releases]
            .iter()
            .filter(|r| **r == startup)
            .count(),
        1
    );
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
#[cfg(feature = "std")]
fn host_failed_retries_abort_drop_and_type_mismatch_preserve_original_slot() {
    let drops = Arc::new(AtomicUsize::new(0));
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&compiler, 2);
    let checked = host(compiler, id, &drops)
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("capture"));
    let source = plan("mock://s/temperature");
    let input =
        BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
    let other_candidate = BindingCandidate::new(
        BindingId::new(999),
        id.binding_generation(),
        id.configuration(),
        id.artifact_compatibility(),
        id.diagnostic_ordinal(),
        19,
    );
    let changed =
        BindingCompilerInput::new(&source, other_candidate, BindingArtifactRole::ConsumerCall);
    let compiler = checked.registration().compiler();
    let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
    let request = acquired(
        requests,
        checked.support().host_slot_layout().unwrap(),
        true,
    );
    let mut cursor = cursor;
    for attempt in 0..5 {
        let mut budget = work(3);
        let (step, requests) = observe(false, || compiler.step(&changed, cursor, &mut budget));
        no_requests(requests);
        let BindingCompilerStep::Failed(failure) = step else {
            panic!("Failed")
        };
        cursor = failure.into_parts().1;
        assert_eq!(
            budget.remaining(WorkClass::BindingPolls),
            if attempt < 2 { 0 } else { 3 }
        );
    }
    let (result, requests) = observe(false, || compiler.abort(cursor));
    assert!(result.is_ok());
    released(requests, request);
    // Dropping an abandoned pure cursor also releases its sole physical owner.
    let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
    let request = acquired(
        requests,
        checked.support().host_slot_layout().unwrap(),
        true,
    );
    let ((), requests) = observe(false, || drop(cursor));
    released(requests, request);

    let generic =
        HostBindingCompilerRegistration::new(ResolvedTargetCompiler::try_new(64).unwrap());
    let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
    let request = acquired(
        requests,
        checked.support().host_slot_layout().unwrap(),
        true,
    );
    let (step, requests) = observe(false, || generic.step(&input, cursor, &mut work(3)));
    no_requests(requests);
    let BindingCompilerStep::Failed(failure) = step else {
        panic!("generic cannot consume supported storage")
    };
    let (cursor, requests) = observe(false, || generic.abort(failure.into_parts().1).unwrap_err());
    no_requests(requests);
    let (result, requests) = observe(false, || compiler.abort(cursor));
    assert!(result.is_ok());
    released(requests, request);
    let cursor = generic.start(&input).unwrap();
    let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(3)));
    no_requests(requests);
    let BindingCompilerStep::Failed(failure) = step else {
        panic!("supported cannot consume generic storage")
    };
    let (cursor, requests) = observe(false, || {
        compiler.abort(failure.into_parts().1).unwrap_err()
    });
    no_requests(requests);
    generic.abort(cursor).unwrap();
}

#[test]
#[cfg(feature = "std")]
fn dropping_a_completed_host_artifact_releases_backing_once_without_extraction() {
    let drops = Arc::new(AtomicUsize::new(0));
    let native = ResolvedTargetCompiler::try_new(64).unwrap();
    let id = identity(&native, 2);
    let checked = host(native, id, &drops)
        .try_into_consumer_compiler()
        .unwrap_or_else(|_| panic!("capture"));
    let source = plan("mock://s/temperature");
    let input =
        BindingCompilerInput::new(&source, candidate(id), BindingArtifactRole::ConsumerCall);
    let compiler = checked.registration().compiler();
    let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
    let request = acquired(
        requests,
        checked.support().host_slot_layout().unwrap(),
        true,
    );
    let BindingCompilerStep::Pending(cursor) = compiler.step(&input, cursor, &mut work(3)) else {
        panic!("Pending")
    };
    let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(3)));
    no_requests(requests);
    let BindingCompilerStep::Complete(output) = step else {
        panic!("Complete")
    };
    drop(checked);
    drop(source);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    let ((), requests) = observe(false, || drop(output));
    released(requests, request);
}
