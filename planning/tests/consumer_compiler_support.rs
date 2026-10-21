//! External completion evidence using only the public production Core surface.
#![allow(unused_variables)]
#[cfg(feature = "std")]
use clinkz_wot_core::binding::ClientBinding;
use clinkz_wot_core::*;
#[cfg(feature = "std")]
use clinkz_wot_foundation::{AdmissionLedger, ResourceKind};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
use clinkz_wot_planning::*;
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

#[test]
fn owned_configuration_canonical_vectors_namespace_and_constructor_boundaries() {
    let private = BindingConfigurationDigest::new(core::array::from_fn(|i| i as u8));
    let make = |contract, capacity, private, body, media| {
        ConsumerBindingConfiguration::try_new(contract, capacity, private, body, media)
    };
    let base = make(
        BindingResourceContract::ConsumerOwnedV2,
        64,
        private,
        64,
        32,
    )
    .unwrap();
    assert_eq!(
        base.compilation_cell(),
        if cfg!(feature = "std") {
            "std"
        } else if cfg!(feature = "async") {
            "async-no-std"
        } else {
            "no-default"
        }
    );
    let expected = if cfg!(feature = "std") {
        "c10c25e570a3783d6a2f5e71c4fc417de20e74bc3896c207de2786f91952ed18"
    } else if cfg!(feature = "async") {
        "e758e21849b61173aeffc37746f45127efeb3c8c760b9b6eb9e92c98d692471c"
    } else {
        "e1fa9be77c1b672e0967a5f1ef0303fab9d44d76780393f4ce62c751bcd3e5e8"
    };
    let hex = |c: ConsumerBindingConfiguration| {
        c.configuration()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_eq!(hex(base), expected);
    assert!(!base.requires_default_json_forms());
    let (restricted, requests) = observe(false, || base.with_default_json_forms().unwrap());
    no_requests(requests);
    assert!(restricted.requires_default_json_forms());
    assert_eq!(restricted.with_default_json_forms().unwrap(), restricted);
    let expected = if cfg!(feature = "std") {
        "0650f7a9e195b9d2bb70276678d9d636a9287542d836ee8820d11a5b285264a3"
    } else if cfg!(feature = "async") {
        "f28d56485b79a2c4f633d8cf0e558a5f5dd2b2622641fe14f669af9f0ee83926"
    } else {
        "3ef164109f98114818458634105c878df47105612e92ad9ca6612f8284934928"
    };
    // Independently derived with Python hashlib, over the unchanged v1 vectors.
    assert_eq!(hex(restricted), expected);
    assert_ne!(restricted.configuration(), base.configuration());
    println!(
        "configuration/plan layouts: {:?} {:?}",
        Layout::new::<ConsumerBindingConfiguration>(),
        Layout::new::<LogicalInteractionPlan>()
    );
    if cfg!(feature = "std") {
        for (c, expected) in [
            (
                make(
                    BindingResourceContract::ConsumerOwnedV2,
                    63,
                    private,
                    64,
                    32,
                )
                .unwrap(),
                "7e981eaec45be7edc00741e3ee8ec48e3ac2756034e015caa1db8502e24c243d",
            ),
            (
                make(
                    BindingResourceContract::ConsumerOwnedHostV2,
                    64,
                    private,
                    64,
                    32,
                )
                .unwrap(),
                "4c6928e0daa0a9488f8ee0290fac269d1f1b6b85206e163b208b819ac5b27c86",
            ),
            (
                make(
                    BindingResourceContract::ConsumerOwnedV2,
                    64,
                    BindingConfigurationDigest::new(core::array::from_fn(|i| {
                        if i == 0 { 1 } else { i as u8 }
                    })),
                    64,
                    32,
                )
                .unwrap(),
                "62dba0284ec41b0a31774222833f25da7348d2bb61f409fdcedf93c8331d7aed",
            ),
            (
                make(
                    BindingResourceContract::ConsumerOwnedV2,
                    64,
                    private,
                    65,
                    32,
                )
                .unwrap(),
                "80846971ab34df8fedcdf56593356b91f5f1b173e24aaf2117f9a009ecdece77",
            ),
            (
                make(
                    BindingResourceContract::ConsumerOwnedV2,
                    64,
                    private,
                    64,
                    33,
                )
                .unwrap(),
                "408b35c59f09274ccb16a817b4c1bb1fcb456e5d94a71ce9d61fd9e8816353a6",
            ),
        ] {
            assert_eq!(hex(c), expected);
        }
    } else {
        assert!(
            make(
                BindingResourceContract::ConsumerOwnedHostV2,
                64,
                private,
                64,
                32
            )
            .is_err()
        );
    }
    for capacity in 1..=64 {
        let strict = ResolvedTargetCompiler::try_new(capacity).unwrap();
        let mut expected = [0; 32];
        expected[..4].copy_from_slice(&[0x63, 0x74, 1, capacity as u8]);
        assert_eq!(
            strict.configuration(),
            BindingConfigurationDigest::new(expected)
        );
        let (owned, requests) = observe(false, || {
            make(
                BindingResourceContract::ConsumerOwnedV2,
                capacity,
                private,
                64,
                32,
            )
        });
        no_requests(requests);
        let owned = owned.unwrap();
        assert_ne!(owned.configuration(), strict.configuration());
        let restricted = owned.with_default_json_forms().unwrap();
        assert_ne!(restricted.configuration(), owned.configuration());
        for other_capacity in 1..=64 {
            assert_ne!(
                restricted.configuration(),
                ResolvedTargetCompiler::try_new(other_capacity)
                    .unwrap()
                    .configuration()
            );
        }
    }
    for (capacity, body, media) in [
        (0, 64, 32),
        (65, 64, 32),
        (64, 0, 32),
        (64, 64, 0),
        (64, u64::MAX, 32),
        (64, 64, u64::MAX),
    ] {
        let (r, requests) = observe(false, || {
            make(
                BindingResourceContract::ConsumerOwnedV2,
                capacity,
                private,
                body,
                media,
            )
        });
        no_requests(requests);
        assert!(r.is_err());
    }
    assert!(make(BindingResourceContract::StrictV1, 64, private, 64, 32).is_err());
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

impl<C: BindingCompilerExtension> clinkz_wot_planning::PrepareServerBinding for Endpoint<C> {
    type Compiler = C;
}
impl<C: BindingCompilerExtension> PollServerBinding for Endpoint<C> {
    type Artifact = <C as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
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
        artifact: &BindingArtifactEnvelope<Self::Artifact>,
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

impl<C: BindingCompilerExtension> clinkz_wot_planning::PrepareClientBinding for Endpoint<C> {
    type Compiler = C;
}
impl<C: BindingCompilerExtension> PollClientBinding for Endpoint<C> {
    type Artifact = <C as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
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
        artifact: &BindingArtifactEnvelope<Self::Artifact>,
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
fn json_plan(metadata: Option<bool>, media: Option<&str>) -> LogicalInteractionPlan {
    let plan = LogicalInteractionPlan::try_property_read(
        PlanId::new(
            SlotIndex::new(23),
            Generation::INITIAL.checked_next().unwrap(),
        ),
        ThingId::from("urn:production:compiler"),
        "temperature".into(),
        1,
        "mock://s/a".into(),
        media.map(Into::into),
        None,
    )
    .unwrap();
    assert_eq!(plan.has_additional_form_metadata(), None);
    match metadata {
        Some(present) => plan.with_additional_form_metadata(present),
        None => plan,
    }
}

fn assert_form_rejected(error: &CoreError, input: &BindingCompilerInput<'_>) {
    assert!(matches!(error, CoreError::UnsupportedOperation(_)));
    let context = error.context();
    assert_eq!(context.phase(), ErrorPhase::Admission);
    assert_eq!(context.retry_class(), RetryClass::Never);
    assert_eq!(context.operation(), Some(input.logical_plan().operation()));
    assert_eq!(
        context.form_index(),
        Some(input.logical_plan().form_index())
    );
    assert_eq!(context.plan(), Some(input.logical_plan().plan_id()));
    assert_eq!(
        context.binding(),
        Some((
            input.candidate().binding_id(),
            input.candidate().binding_generation()
        ))
    );
}

#[test]
fn restricted_native_bounds_start_and_paid_step_reject_unknown_present_and_media() {
    let original = ConsumerBindingConfiguration::try_new(
        BindingResourceContract::ConsumerOwnedV2,
        64,
        BindingConfigurationDigest::new([5; 32]),
        4096,
        32,
    )
    .unwrap();
    let compiler = ResolvedTargetCompiler::new_owned(original.with_default_json_forms().unwrap());
    let id = identity(&compiler, 2);
    let valid = json_plan(Some(false), Some("application/json"));
    let valid_input =
        BindingCompilerInput::new(&valid, candidate(id), BindingArtifactRole::ConsumerCall);
    assert_eq!(
        compiler
            .bounds(&valid_input)
            .unwrap()
            .work()
            .remaining(WorkClass::BindingPolls),
        6
    );
    for metadata in [None, Some(false), Some(true)] {
        for media in [
            None,
            Some("application/json"),
            Some("text/plain"),
            Some("application/json;charset=utf-8"),
            Some("APPLICATION/JSON"),
        ] {
            let plan = json_plan(metadata, media);
            let input =
                BindingCompilerInput::new(&plan, candidate(id), BindingArtifactRole::ConsumerCall);
            let (bounds, requests) = observe(false, || compiler.bounds(&input));
            no_requests(requests);
            let (start, requests) = observe(false, || compiler.start(&input));
            no_requests(requests);
            let eligible = metadata == Some(false) && media == Some("application/json");
            assert_eq!(bounds.is_ok(), eligible);
            assert_eq!(start.is_ok(), eligible);
            if !eligible {
                assert_form_rejected(&bounds.unwrap_err(), &input);
                assert_form_rejected(&start.unwrap_err(), &input);
            }
            let mut cursor = compiler.start(&valid_input).unwrap();
            for _ in 0..3 {
                for units in 0..3 {
                    let mut budget = work(units);
                    let (step, requests) =
                        observe(false, || compiler.step(&input, cursor, &mut budget));
                    no_requests(requests);
                    let BindingCompilerStep::Pending(next) = step else {
                        panic!("short credit advanced")
                    };
                    cursor = next;
                    assert_eq!(budget.remaining(WorkClass::BindingPolls), units);
                }
            }
            let mut budget = work(3);
            let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut budget));
            no_requests(requests);
            assert_eq!(budget.remaining(WorkClass::BindingPolls), 0);
            if eligible {
                let BindingCompilerStep::Pending(cursor) = step else {
                    panic!()
                };
                let BindingCompilerStep::Complete(output) =
                    compiler.step(&input, cursor, &mut work(3))
                else {
                    panic!()
                };
                assert_eq!(output.artifact().payload().target(), "mock://s/a");
            } else {
                let BindingCompilerStep::Failed(failure) = step else {
                    panic!()
                };
                let (cause, cursor) = failure.into_parts();
                assert_form_rejected(&cause, &input);
                compiler.abort(cursor);
            }
            // The original unrestricted owned and strict copy primitive retain
            // their accepted behavior for all metadata/media combinations.
            for unrestricted in [
                ResolvedTargetCompiler::try_new(64).unwrap(),
                ResolvedTargetCompiler::new_owned(original),
            ] {
                let input = BindingCompilerInput::new(
                    &plan,
                    candidate(identity(&unrestricted, 2)),
                    BindingArtifactRole::ConsumerCall,
                );
                assert!(unrestricted.bounds(&input).is_ok());
                unrestricted.abort(unrestricted.start(&input).unwrap());
            }
        }
    }
}

#[cfg(feature = "std")]
#[test]
fn restricted_public_host_compiler_preserves_slot_on_short_credit_and_eligibility_failure() {
    let configuration = ConsumerBindingConfiguration::try_new(
        BindingResourceContract::ConsumerOwnedHostV2,
        64,
        BindingConfigurationDigest::new([5; 32]),
        4096,
        32,
    )
    .unwrap()
    .with_default_json_forms()
    .unwrap();
    let native = ResolvedTargetCompiler::new_owned(configuration);
    let id = identity(&native, 2);
    let compiler = HostBindingCompilerRegistration::try_new_consumer(native).unwrap();
    let valid = json_plan(Some(false), Some("application/json"));
    let valid_input =
        BindingCompilerInput::new(&valid, candidate(id), BindingArtifactRole::ConsumerCall);
    assert_eq!(
        compiler
            .bounds(&valid_input)
            .unwrap()
            .work()
            .remaining(WorkClass::BindingPolls),
        8
    );
    for (metadata, media) in [
        (None, Some("application/json")),
        (Some(true), Some("application/json")),
        (Some(false), None),
        (Some(false), Some("text/plain")),
        (Some(false), Some("application/json;charset=utf-8")),
        (Some(false), Some("application/json")),
    ] {
        let plan = json_plan(metadata, media);
        let input =
            BindingCompilerInput::new(&plan, candidate(id), BindingArtifactRole::ConsumerCall);
        let eligible = metadata == Some(false) && media == Some("application/json");
        if !eligible {
            assert_form_rejected(&compiler.bounds(&input).unwrap_err(), &input);
            let (rejected, allocations) = observe(false, || compiler.start(&input));
            assert_form_rejected(&rejected.unwrap_err(), &input);
            assert_eq!((allocations.allocations, allocations.releases), (1, 1));
            assert_eq!(allocations.allocated[0], allocations.released[0]);
        }
        let mut cursor = compiler.start(&valid_input).unwrap();
        for _ in 0..3 {
            for units in 0..4 {
                let mut budget = work(units);
                let (step, allocations) =
                    observe(false, || compiler.step(&input, cursor, &mut budget));
                no_requests(allocations);
                let BindingCompilerStep::Pending(next) = step else {
                    panic!("short Host credit advanced")
                };
                cursor = next;
                assert_eq!(budget.remaining(WorkClass::BindingPolls), units);
            }
        }
        let mut budget = work(4);
        let (step, allocations) = observe(false, || compiler.step(&input, cursor, &mut budget));
        no_requests(allocations);
        assert_eq!(budget.remaining(WorkClass::BindingPolls), 0);
        if eligible {
            let BindingCompilerStep::Pending(cursor) = step else {
                panic!()
            };
            let BindingCompilerStep::Complete(output) = compiler.step(&input, cursor, &mut work(4))
            else {
                panic!()
            };
            assert_eq!(
                output
                    .artifact()
                    .try_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                    .unwrap()
                    .target(),
                "mock://s/a"
            );
        } else {
            let BindingCompilerStep::Failed(failure) = step else {
                panic!()
            };
            let (cause, cursor) = failure.into_parts();
            assert_form_rejected(&cause, &input);
            let ((), allocations) = observe(false, || compiler.abort(cursor).unwrap());
            assert_eq!((allocations.allocations, allocations.releases), (0, 1));
        }
    }
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
type Complete<C> = StaticBindingPreparation<StaticBindingComponents<Endpoint<C>, Endpoint<C>>>;
fn static_registration<C: BindingCompilerExtension>(
    compiler: C,
    id: BindingRegistrationIdentity,
    consumer: bool,
    execution: BindingExecutionSupport,
    drops: &Arc<AtomicUsize>,
) -> Result<
    Complete<C>,
    BindingInputRejection<
        StaticBindingPreparationInput<StaticBindingComponents<Endpoint<C>, Endpoint<C>>>,
    >,
> {
    let components = StaticBindingComponents::new(
        Endpoint::new(id.artifact_compatibility(), drops),
        Endpoint::new(id.artifact_compatibility(), drops),
    );
    let input = if consumer {
        StaticBindingPreparationInput::producer_and_consumer_property_read(
            id,
            execution,
            StaticBindingCompilerRegistration::new(compiler),
            components,
            resources(),
            BindingIngressPolicy::hidden(),
            BindingStatusPolicy::new(3, 5),
        )
    } else {
        StaticBindingPreparationInput::new(
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
        StaticBindingPreparation::producer_and_consumer_property_read(input)
    } else {
        StaticBindingPreparation::new(input)
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
) -> Result<HostBindingPreparation, BindingInputRejection<HostBindingPreparationInput>> {
    let server = Box::new(Endpoint::<ResolvedTargetCompiler>::new(
        id.artifact_compatibility(),
        drops,
    ));
    let input = if consumer {
        HostBindingPreparationInput::producer_and_consumer_property_read(
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
        HostBindingPreparationInput::new(
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
    HostBindingPreparation::new(input)
}
#[cfg(feature = "std")]
fn host(
    compiler: ResolvedTargetCompiler,
    id: BindingRegistrationIdentity,
    drops: &Arc<AtomicUsize>,
) -> HostBindingPreparation {
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
    assert_eq!(support.consumer_configuration(), None);
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

mod consumer_only_support {
    use super::*;
    struct CaptureClient<C> {
        marker: PhantomData<C>,
        compatibility: BindingArtifactCompatibility,
        owned: Option<ConsumerBindingConfiguration>,
        drops: Arc<AtomicUsize>,
    }
    impl<C> Drop for CaptureClient<C> {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl<C> CaptureClient<C> {
        fn new(compatibility: BindingArtifactCompatibility, drops: &Arc<AtomicUsize>) -> Self {
            Self {
                marker: PhantomData,
                compatibility,
                owned: None,
                drops: drops.clone(),
            }
        }
    }
    impl<C: BindingCompilerExtension> clinkz_wot_planning::PrepareClientBinding for CaptureClient<C> {
        type Compiler = C;
    }
    impl<C: BindingCompilerExtension> PollClientBinding for CaptureClient<C> {
        type Artifact = <C as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
        type RequestState = ();
        fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
            self.owned
        }
        fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
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
        ) -> Result<
            StartStatus<CoreResult<InteractionOutput>>,
            BindingInputRejection<OutboundRequest>,
        > {
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
    impl<C: BindingCompilerExtension + Send + Sync> ClientBinding for CaptureClient<C> {
        fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
            self.owned
        }
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

    fn typed<C: BindingCompilerExtension>(
        compiler: C,
        id: BindingRegistrationIdentity,
        drops: &Arc<AtomicUsize>,
    ) -> StaticConsumerBindingPreparation<CaptureClient<C>> {
        StaticConsumerBindingPreparation::new(StaticConsumerBindingPreparationInput::new(
            id,
            BindingExecutionSupport::application_static(),
            StaticBindingCompilerRegistration::new(compiler),
            CaptureClient::new(id.artifact_compatibility(), drops),
            resources(),
            BindingStatusPolicy::new(3, 5),
        ))
        .unwrap_or_else(|_| panic!("Consumer-only static"))
    }
    #[cfg(feature = "std")]
    fn host_registration(
        compiler: HostBindingCompilerRegistration,
        id: BindingRegistrationIdentity,
        consumer: bool,
        execution: BindingExecutionSupport,
        drops: &Arc<AtomicUsize>,
    ) -> Result<
        HostConsumerBindingPreparation,
        BindingInputRejection<HostConsumerBindingPreparationInput>,
    > {
        assert!(consumer);
        HostConsumerBindingPreparation::new(HostConsumerBindingPreparationInput::new(
            id,
            execution,
            compiler,
            Box::new(CaptureClient::<ResolvedTargetCompiler>::new(
                id.artifact_compatibility(),
                drops,
            )),
            resources(),
            BindingStatusPolicy::new(3, 5),
        ))
    }
    #[cfg(feature = "std")]
    fn host(
        compiler: ResolvedTargetCompiler,
        id: BindingRegistrationIdentity,
        drops: &Arc<AtomicUsize>,
    ) -> HostConsumerBindingPreparation {
        host_registration(
            HostBindingCompilerRegistration::try_new_consumer(compiler).unwrap(),
            id,
            true,
            BindingExecutionSupport::host_erased(),
            drops,
        )
        .unwrap_or_else(|_| panic!("Consumer-only Host"))
    }

    #[test]
    fn consumer_only_typed_complete_owner_and_nondefault_identity_survive_actual_source_destruction()
     {
        for (capacity, variant) in [(32, 2), (64, 5)] {
            let drops = Arc::new(AtomicUsize::new(0));
            let c = ResolvedTargetCompiler::try_new(capacity).unwrap();
            let id = identity(&c, variant);
            let (checked, trace) = observe(false, || {
                typed(c, id, &drops)
                    .try_into_consumer_compiler()
                    .unwrap_or_else(|_| panic!("capture"))
            });
            no_requests(trace);
            check_descriptor(checked.support(), capacity, false);
            assert_eq!(checked.registration().identity(), id);
            assert!(
                !checked
                    .registration()
                    .capabilities()
                    .supports_producer_property_read()
            );
            let source = plan("mock://s/温度");
            let input = BindingCompilerInput::new(
                &source,
                candidate(id),
                BindingArtifactRole::ConsumerCall,
            );
            let compiler = checked.registration().compiler().compiler();
            let (output, trace) = observe(false, || {
                let bounds = compiler.bounds(&input).unwrap();
                let cursor = compiler.start(&input).unwrap();
                let mut budget = bounds.into_work();
                let BindingCompilerStep::Pending(cursor) =
                    compiler.step(&input, cursor, &mut budget)
                else {
                    panic!("first paid step")
                };
                let BindingCompilerStep::Complete(output) =
                    compiler.step(&input, cursor, &mut budget)
                else {
                    panic!("second paid step")
                };
                output
            });
            no_requests(trace);
            let qualified = artifact_identity(id, &source);
            let envelope = BindingArtifactEnvelope::try_new(
                qualified,
                output.artifact().footprint(),
                output.into_artifact(),
            )
            .unwrap();
            drop(checked);
            drop(source);
            assert_eq!(drops.load(Ordering::SeqCst), 1);
            assert_eq!(envelope.identity(), qualified);
            assert_eq!(envelope.artifact().payload().target(), "mock://s/温度");
            let ((), trace) = observe(false, || drop(envelope));
            no_requests(trace);
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn consumer_only_public_host_complete_owner_retains_one_actual_slot_through_completion() {
        for (capacity, variant) in [(32, 2), (64, 5)] {
            let drops = Arc::new(AtomicUsize::new(0));
            let c = ResolvedTargetCompiler::try_new(capacity).unwrap();
            let id = identity(&c, variant);
            let owner = host(c, id, &drops);
            let (checked, trace) = observe(false, || {
                owner
                    .try_into_consumer_compiler()
                    .unwrap_or_else(|_| panic!("capture"))
            });
            no_requests(trace);
            check_descriptor(checked.support(), capacity, true);
            assert_eq!(checked.registration().identity(), id);
            assert!(
                !checked
                    .registration()
                    .capabilities()
                    .supports_producer_property_read()
            );
            let layout = checked.support().host_slot_layout().unwrap();
            let source = plan("mock://s/温度");
            let input = BindingCompilerInput::new(
                &source,
                candidate(id),
                BindingArtifactRole::ConsumerCall,
            );
            let compiler = checked.registration().compiler();
            let bounds = compiler.bounds(&input).unwrap();
            let (cursor, trace) = observe(false, || compiler.start(&input).unwrap());
            let request = acquired(trace, layout, true);
            let mut budget = bounds.into_work();
            let (output, trace) = observe(false, || {
                let BindingCompilerStep::Pending(cursor) =
                    compiler.step(&input, cursor, &mut budget)
                else {
                    panic!("first paid step")
                };
                let BindingCompilerStep::Complete(output) =
                    compiler.step(&input, cursor, &mut budget)
                else {
                    panic!("second paid step")
                };
                output
            });
            no_requests(trace);
            let qualified = artifact_identity(id, &source);
            let envelope = BindingArtifactEnvelope::try_new(
                qualified,
                output.artifact().footprint(),
                output.into_artifact(),
            )
            .unwrap();
            drop(checked);
            drop(source);
            assert_eq!(drops.load(Ordering::SeqCst), 1);
            assert_eq!(envelope.identity(), qualified);
            let (payload, trace) = observe(false, || {
                envelope
                    .into_artifact()
                    .try_into_payload::<ResolvedTargetArtifact>(id.artifact_compatibility())
                    .unwrap()
            });
            released(trace, request);
            assert_eq!(payload.target(), "mock://s/温度");
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn consumer_only_every_new_actual_allocation_can_fail_and_slot_rollback_retains_ledger_history()
    {
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
        let (failure, requests) =
            observe(false, || checked.registration().compiler().start(&invalid));
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
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn closed_support_rejects_same_compatibility_and_actual_configuration_without_owner_loss() {
        let drops = Arc::new(AtomicUsize::new(0));
        let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&compiler, 7);
        let callbacks = Arc::new(AtomicUsize::new(0));
        let owner = typed(ForeignCompiler(callbacks.clone()), id, &drops);
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
        let (rejected, trace) = observe(false, || {
            owner.try_into_consumer_compiler().err().expect("impostor")
        });
        no_requests(trace);
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
        assert_eq!(rejected.input().identity(), id);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        let owner = rejected.into_input();
        assert_eq!(
            owner.into_input().client().compatibility,
            id.artifact_compatibility()
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        let owner = typed(ResolvedTargetCompiler::try_new(32).unwrap(), id, &drops);
        let (rejected, trace) = observe(false, || {
            owner
                .try_into_consumer_compiler()
                .err()
                .expect("actual config")
        });
        no_requests(trace);
        assert_eq!(rejected.input().identity(), id);
        assert_eq!(
            rejected.input().compiler().compiler().configuration(),
            ResolvedTargetCompiler::try_new(32).unwrap().configuration()
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        drop(rejected.into_input());
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }
    #[cfg(feature = "std")]
    #[test]
    fn host_support_requires_the_actual_supported_adapter_and_returns_exact_client() {
        let drops = Arc::new(AtomicUsize::new(0));
        let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
        let id = identity(&compiler, 7);
        let callbacks = Arc::new(AtomicUsize::new(0));
        for compiler in [
            HostBindingCompilerRegistration::new(compiler),
            HostBindingCompilerRegistration::new(ForeignCompiler(callbacks.clone())),
            HostBindingCompilerRegistration::try_new_consumer(
                ResolvedTargetCompiler::try_new(32).unwrap(),
            )
            .unwrap(),
        ] {
            let client: Box<dyn ClientBinding> = Box::new(
                CaptureClient::<ResolvedTargetCompiler>::new(id.artifact_compatibility(), &drops),
            );
            let pointer = &*client as *const dyn ClientBinding as *const ();
            let input = HostConsumerBindingPreparationInput::new(
                id,
                BindingExecutionSupport::host_erased(),
                compiler,
                client,
                resources(),
                BindingStatusPolicy::new(3, 5),
            );
            let owner = HostConsumerBindingPreparation::new(input)
                .unwrap_or_else(|_| panic!("valid native declaration"));
            let before = drops.load(Ordering::SeqCst);
            let (rejected, trace) = observe(false, || {
                owner
                    .try_into_consumer_compiler()
                    .err()
                    .expect("unsupported capture")
            });
            no_requests(trace);
            assert_eq!(rejected.input().identity(), id);
            assert_eq!(drops.load(Ordering::SeqCst), before);
            let returned = rejected.into_input().into_input();
            assert_eq!(
                returned.client() as *const dyn ClientBinding as *const (),
                pointer
            );
            drop(returned);
            assert_eq!(drops.load(Ordering::SeqCst), before + 1);
        }
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    }
    fn owned_configuration(
        capacity: usize,
        private: u8,
        body: u64,
        media: u64,
    ) -> ConsumerBindingConfiguration {
        ConsumerBindingConfiguration::try_new(
            BindingResourceContract::ConsumerOwnedV2,
            capacity,
            BindingConfigurationDigest::new([private; 32]),
            body,
            media,
        )
        .unwrap()
    }
    fn owned_resources(c: ConsumerBindingConfiguration) -> BindingResourceDeclarations {
        resources()
            .with_resource_contract(c.resource_contract())
            .with_consumer_result_limits(c.result_limits().0, c.result_limits().1)
    }
    #[test]
    fn restricted_and_unrestricted_compiler_and_artifact_substitution_rejects_before_client() {
        let original = owned_configuration(64, 1, 64, 32);
        let restricted = original.with_default_json_forms().unwrap();
        let drops = Arc::new(AtomicUsize::new(0));
        let plan = json_plan(Some(false), Some("application/json"));
        for (actual, foreign) in [(original, restricted), (restricted, original)] {
            let compiler = ResolvedTargetCompiler::new_owned(actual);
            let id = identity(&compiler, 7);
            let foreign_compiler = ResolvedTargetCompiler::new_owned(foreign);
            let foreign_id = identity(&foreign_compiler, 7);
            let foreign_input = BindingCompilerInput::new(
                &plan,
                candidate(foreign_id),
                BindingArtifactRole::ConsumerCall,
            );
            assert!(matches!(
                compiler.bounds(&foreign_input),
                Err(CoreError::Validation(_))
            ));
            assert!(matches!(
                compiler.start(&foreign_input),
                Err(CoreError::Validation(_))
            ));
            let cursor = foreign_compiler.start(&foreign_input).unwrap();
            let BindingCompilerStep::Failed(failure) =
                compiler.step(&foreign_input, cursor, &mut work(3))
            else {
                panic!("compiler substitution")
            };
            let (cause, cursor) = failure.into_parts();
            assert!(matches!(cause, CoreError::Validation(_)));
            foreign_compiler.abort(cursor);
            let cursor = foreign_compiler.start(&foreign_input).unwrap();
            let BindingCompilerStep::Pending(cursor) =
                foreign_compiler.step(&foreign_input, cursor, &mut work(3))
            else {
                panic!()
            };
            let BindingCompilerStep::Complete(output) =
                foreign_compiler.step(&foreign_input, cursor, &mut work(3))
            else {
                panic!()
            };
            let envelope = BindingArtifactEnvelope::try_new(
                artifact_identity(foreign_id, &plan),
                output.artifact().footprint(),
                output.into_artifact(),
            )
            .unwrap();
            let request = OutboundRequest::property_read(
                BindingArtifactRef::new(envelope.identity(), SlotIndex::new(0)),
                Default::default(),
                None,
            )
            .unwrap();
            let expected = OutboundRequest::property_read(
                BindingArtifactRef::new(envelope.identity(), SlotIndex::new(0)),
                Default::default(),
                None,
            )
            .unwrap();
            let mut client = CaptureClient::new(id.artifact_compatibility(), &drops);
            client.owned = Some(actual);
            let mut owner =
                StaticConsumerBindingPreparation::new(StaticConsumerBindingPreparationInput::new(
                    id,
                    BindingExecutionSupport::application_static(),
                    StaticBindingCompilerRegistration::new(compiler),
                    client,
                    owned_resources(actual),
                    BindingStatusPolicy::new(3, 5),
                ))
                .unwrap_or_else(|_| panic!());
            let mut slot = StaticConsumerPropertyReadSlot::new();
            let (rejected, requests) = observe(false, || {
                owner.start_consumer_property_read(request, &envelope, &mut slot, &mut work(100))
            });
            no_requests(requests);
            let rejected = rejected.err().expect("artifact substitution");
            assert_eq!(rejected.input(), &expected);
            assert!(
                matches!(rejected.error().error(), CoreError::Validation(_)),
                "raw client returns UnsupportedOperation if reached"
            );
            assert!(slot.is_vacant());

            #[cfg(feature = "std")]
            {
                let mut client = CaptureClient::<ResolvedTargetCompiler>::new(
                    id.artifact_compatibility(),
                    &drops,
                );
                client.owned = Some(actual);
                let owner =
                    HostConsumerBindingPreparation::new(HostConsumerBindingPreparationInput::new(
                        id,
                        BindingExecutionSupport::host_erased(),
                        HostBindingCompilerRegistration::try_new_consumer(
                            ResolvedTargetCompiler::new_owned(actual),
                        )
                        .unwrap(),
                        Box::new(client),
                        owned_resources(actual),
                        BindingStatusPolicy::new(3, 5),
                    ))
                    .unwrap_or_else(|_| panic!());
                let foreign_compiler =
                    HostBindingCompilerRegistration::try_new_consumer(foreign_compiler).unwrap();
                let cursor = foreign_compiler.start(&foreign_input).unwrap();
                let BindingCompilerStep::Pending(cursor) =
                    foreign_compiler.step(&foreign_input, cursor, &mut work(4))
                else {
                    panic!()
                };
                let BindingCompilerStep::Complete(output) =
                    foreign_compiler.step(&foreign_input, cursor, &mut work(4))
                else {
                    panic!()
                };
                let envelope = BindingArtifactEnvelope::try_new(
                    artifact_identity(foreign_id, &plan),
                    output.artifact().footprint(),
                    output.into_artifact(),
                )
                .unwrap();
                let request = OutboundRequest::property_read(
                    BindingArtifactRef::new(envelope.identity(), SlotIndex::new(0)),
                    Default::default(),
                    None,
                )
                .unwrap();
                let expected = OutboundRequest::property_read(
                    BindingArtifactRef::new(envelope.identity(), SlotIndex::new(0)),
                    Default::default(),
                    None,
                )
                .unwrap();
                let (rejected, requests) = observe(false, || {
                    owner.start_consumer_property_read(request, &envelope)
                });
                no_requests(requests);
                let rejected = rejected.err().expect("Host artifact substitution");
                assert_eq!(rejected.input(), &expected);
                assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
            }
        }
    }
    #[test]
    fn owned_capture_checks_every_client_operand_and_preserves_exact_complete_owner() {
        let original = owned_configuration(64, 1, 64, 32);
        for actual in [original, original.with_default_json_forms().unwrap()] {
            let drops = Arc::new(AtomicUsize::new(0));
            for client_config in [
                None,
                Some(if actual.requires_default_json_forms() {
                    original
                } else {
                    original.with_default_json_forms().unwrap()
                }),
                Some(owned_configuration(63, 1, 64, 32)),
                Some(owned_configuration(64, 2, 64, 32)),
                Some(owned_configuration(64, 1, 65, 32)),
                Some(owned_configuration(64, 1, 64, 33)),
            ] {
                let compiler = ResolvedTargetCompiler::new_owned(actual);
                let id = identity(&compiler, 7);
                let mut client = CaptureClient::new(id.artifact_compatibility(), &drops);
                client.owned = client_config;
                let input = StaticConsumerBindingPreparationInput::new(
                    id,
                    BindingExecutionSupport::application_static(),
                    StaticBindingCompilerRegistration::new(compiler),
                    client,
                    owned_resources(actual),
                    BindingStatusPolicy::new(3, 5),
                );
                let (result, requests) =
                    observe(false, || StaticConsumerBindingPreparation::new(input));
                no_requests(requests);
                let returned = result
                    .err()
                    .expect("actual client metadata must match before callbacks")
                    .into_input();
                assert_eq!(returned.identity(), id);
                assert!(Arc::ptr_eq(&returned.client().drops, &drops));
                drop(returned);
            }
            let compiler = ResolvedTargetCompiler::new_owned(actual);
            let id = identity(&compiler, 7);
            let mut client = CaptureClient::new(id.artifact_compatibility(), &drops);
            client.owned = Some(actual);
            let (owner, requests) = observe(false, || {
                StaticConsumerBindingPreparation::new(StaticConsumerBindingPreparationInput::new(
                    id,
                    BindingExecutionSupport::application_static(),
                    StaticBindingCompilerRegistration::new(compiler),
                    client,
                    owned_resources(actual),
                    BindingStatusPolicy::new(3, 5),
                ))
            });
            no_requests(requests);
            let owner = owner.unwrap_or_else(|_| panic!("owned construction"));
            let (owner, requests) = observe(false, || owner.try_into_consumer_compiler());
            no_requests(requests);
            let owner = owner.unwrap_or_else(|_| panic!("owned capture"));
            let (projection, requests) =
                observe(false, || owner.support().consumer_configuration());
            no_requests(requests);
            assert_eq!(projection, Some(actual));
            let extra = u64::from(actual.requires_default_json_forms());
            assert_eq!(
                (
                    owner.support().bounds_work(),
                    owner.support().start_work(),
                    owner.support().step_work()
                ),
                (1 + extra, 1 + extra, 2 + extra)
            );
            let plan = json_plan(Some(false), Some("application/json"));
            let input =
                BindingCompilerInput::new(&plan, candidate(id), BindingArtifactRole::ConsumerCall);
            let c = owner.registration().compiler().compiler();
            let bounds = c.bounds(&input).unwrap();
            let cursor = c.start(&input).unwrap();
            let BindingCompilerStep::Pending(cursor) = c.step(&input, cursor, &mut work(0)) else {
                panic!()
            };
            let changed = ResolvedTargetCompiler::new_owned(owned_configuration(63, 1, 64, 32));
            let BindingCompilerStep::Failed(f) = changed.step(&input, cursor, &mut work(2)) else {
                panic!("paid continuation cannot change configuration")
            };
            c.abort(f.into_parts().1);
            assert!(bounds.artifact().retained_bytes() > 0);
            println!(
                "owned compiler/support/cursor/output/temporary layouts: {:?} {:?} {:?} {:?} {:?}",
                Layout::new::<ResolvedTargetCompiler>(),
                Layout::new::<ConsumerCompilerSupport>(),
                owner.support().cursor_layout(),
                owner.support().output_layout(),
                owner.support().temporary_layout()
            );
        }
    }
    #[cfg(feature = "std")]
    #[test]
    fn owned_host_capture_recovers_client_address_and_prestart_slot_failure() {
        let original = owned_configuration(64, 1, 64, 32);
        for actual in [original, original.with_default_json_forms().unwrap()] {
            let drops = Arc::new(AtomicUsize::new(0));
            let compiler = ResolvedTargetCompiler::new_owned(actual);
            let (returned, requests) = observe(true, || {
                HostBindingCompilerRegistration::try_new_consumer(compiler)
            });
            acquired(requests, Layout::new::<ResolvedTargetCompiler>(), false);
            assert_eq!(
                returned.unwrap_err().input().configuration(),
                actual.configuration()
            );
            for match_client in [false, true] {
                let c = ResolvedTargetCompiler::new_owned(actual);
                let id = identity(&c, 7);
                let adapter = HostBindingCompilerRegistration::try_new_consumer(c).unwrap();
                let mut client = CaptureClient::<ResolvedTargetCompiler>::new(
                    id.artifact_compatibility(),
                    &drops,
                );
                client.owned = Some(if match_client {
                    actual
                } else {
                    if actual.requires_default_json_forms() {
                        original
                    } else {
                        original.with_default_json_forms().unwrap()
                    }
                });
                let client: Box<dyn ClientBinding> = Box::new(client);
                let pointer = &*client as *const dyn ClientBinding as *const ();
                let input = HostConsumerBindingPreparationInput::new(
                    id,
                    BindingExecutionSupport::host_erased(),
                    adapter,
                    client,
                    owned_resources(actual),
                    BindingStatusPolicy::new(3, 5),
                );
                let (owner, requests) =
                    observe(false, || HostConsumerBindingPreparation::new(input));
                no_requests(requests);
                if !match_client {
                    let input = owner.err().expect("mismatch").into_input();
                    assert_eq!(
                        input.client() as *const dyn ClientBinding as *const (),
                        pointer
                    );
                    drop(input);
                    continue;
                }
                let owner = owner
                    .unwrap_or_else(|_| panic!())
                    .try_into_consumer_compiler()
                    .unwrap_or_else(|_| panic!());
                let (projection, requests) =
                    observe(false, || owner.support().consumer_configuration());
                no_requests(requests);
                assert_eq!(projection, Some(actual));
                let extra = u64::from(actual.requires_default_json_forms());
                assert_eq!(
                    (
                        owner.support().bounds_work(),
                        owner.support().start_work(),
                        owner.support().step_work()
                    ),
                    (1 + extra, 1 + extra, 3 + extra)
                );
                println!(
                    "Host restricted={}: adapter={:?}; slot={:?}; cursor={:?}; output={:?}; temporary={:?}; callback work={:?}",
                    actual.requires_default_json_forms(),
                    owner.support().host_adapter_layout(),
                    owner.support().host_slot_layout(),
                    owner.support().host_cursor_owner_layout(),
                    owner.support().host_output_owner_layout(),
                    owner.support().temporary_layout(),
                    (
                        owner.support().bounds_work(),
                        owner.support().start_work(),
                        owner.support().step_work()
                    )
                );
                let plan = json_plan(Some(false), Some("application/json"));
                let input = BindingCompilerInput::new(
                    &plan,
                    candidate(id),
                    BindingArtifactRole::ConsumerCall,
                );
                let (failure, requests) =
                    observe(true, || owner.registration().compiler().start(&input));
                acquired(requests, owner.support().host_slot_layout().unwrap(), false);
                assert!(failure.is_err());
                let (cursor, requests) =
                    observe(false, || owner.registration().compiler().start(&input));
                acquired(requests, owner.support().host_slot_layout().unwrap(), true);
                owner
                    .registration()
                    .compiler()
                    .abort(cursor.unwrap())
                    .unwrap();
                let input = owner.into_registration().into_input();
                assert_eq!(
                    input.client() as *const dyn ClientBinding as *const (),
                    pointer
                );
            }
            assert_eq!(drops.load(Ordering::SeqCst), 2);
        }
    }
}

// External authorship: no source-known compiler, marker or private erasure hook.
#[cfg(feature = "std")]
mod generic_host_storage {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[derive(Debug, Default)]
    struct Counts {
        starts: AtomicUsize,
        steps: AtomicUsize,
        aborts: AtomicUsize,
        cursors: AtomicUsize,
        artifacts: AtomicUsize,
        reject_start: AtomicBool,
        reject_step: AtomicBool,
    }
    #[derive(Debug)]
    struct Compiler(Arc<Counts>);
    // Cell deliberately makes the native cursor !Sync, while still Send.
    struct Cursor {
        turn: Cell<u8>,
        counts: Arc<Counts>,
        _scratch: [u8; 769],
    }
    impl Drop for Cursor {
        fn drop(&mut self) {
            self.counts.cursors.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[repr(align(128))]
    struct Artifact {
        instructions: [u8; 257],
        counts: Arc<Counts>,
    }
    impl Drop for Artifact {
        fn drop(&mut self) {
            self.counts.artifacts.fetch_add(1, Ordering::SeqCst);
        }
    }
    const FORMAT: BindingArtifactCompatibility = BindingArtifactCompatibility::new([42; 16]);
    impl BindingCompilerExtension for Compiler {
        type Cursor = Cursor;
        type Artifact = Artifact;
        fn compatibility(&self) -> BindingArtifactCompatibility {
            FORMAT
        }
        fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, std::mem::size_of::<Artifact>() as u64),
                std::mem::size_of::<Cursor>() as u64,
                0,
                work(4),
            ))
        }
        fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<Cursor> {
            self.0.starts.fetch_add(1, Ordering::SeqCst);
            if self.0.reject_start.load(Ordering::SeqCst) {
                return Err(test_error());
            }
            Ok(Cursor {
                turn: Cell::new(0),
                counts: self.0.clone(),
                _scratch: [0; 769],
            })
        }
        fn step(
            &self,
            _: &BindingCompilerInput<'_>,
            cursor: Cursor,
            budget: &mut WorkBudget,
        ) -> BindingCompilerStep<Cursor, Artifact> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return BindingCompilerStep::Pending(cursor);
            }
            self.0.steps.fetch_add(1, Ordering::SeqCst);
            if self.0.reject_step.load(Ordering::SeqCst) {
                return BindingCompilerStep::Failed(BindingCompilerFailure::new(
                    test_error(),
                    cursor,
                ));
            }
            cursor.turn.set(cursor.turn.get() + 1);
            if cursor.turn.get() < 3 {
                return BindingCompilerStep::Pending(cursor);
            }
            // Native instructions, not a copied URI; payload owns no source loan.
            BindingCompilerStep::Complete(BindingCompilerOutput::new(
                BindingArtifact::producer_route(
                    FORMAT,
                    BindingArtifactFootprint::new(1, std::mem::size_of::<Artifact>() as u64),
                    RouteReservationIdentity::new(
                        CollisionDomainId::new([3; 16]),
                        EndpointReservationKey::new([4; 32]),
                    ),
                    Artifact {
                        instructions: [cursor.turn.get(); 257],
                        counts: self.0.clone(),
                    },
                ),
            ))
        }
        fn abort(&self, _: Cursor) {
            self.0.aborts.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn input(plan: &LogicalInteractionPlan) -> BindingCompilerInput<'_> {
        BindingCompilerInput::new(
            plan,
            BindingCandidate::new(
                BindingId::new(8),
                BindingGeneration::INITIAL,
                BindingConfigurationDigest::new([9; 32]),
                FORMAT,
                0,
                0,
            ),
            BindingArtifactRole::ProducerRoute,
        )
    }

    #[test]
    fn generic_construction_and_start_refusal_preserve_the_compiler_and_existing_job() {
        let counts = Arc::new(Counts::default());
        let (rejected, requests) = observe(true, || {
            HostBindingCompilerRegistration::try_new(Compiler(counts.clone())).unwrap_err()
        });
        acquired(requests, Layout::new::<Compiler>(), false);
        assert!(Arc::ptr_eq(&rejected.input().0, &counts));
        assert_eq!(counts.starts.load(Ordering::SeqCst), 0);
        let compiler = HostBindingCompilerRegistration::try_new(rejected.into_input()).unwrap();
        let source = plan("native:address");
        let input = input(&source);
        let (held, requests) = observe(false, || compiler.start(&input).unwrap());
        let held_allocation = acquired(requests, compiler.slot_layout(), true);
        let (refused, requests) = observe(true, || compiler.start(&input));
        acquired(requests, compiler.slot_layout(), false);
        assert!(matches!(refused, Err(CoreError::Backpressure(_))));
        assert_eq!(counts.starts.load(Ordering::SeqCst), 1);
        counts.reject_start.store(true, Ordering::SeqCst);
        let (refused, requests) = observe(false, || compiler.start(&input));
        assert!(matches!(refused, Err(CoreError::UnsupportedOperation(_))));
        assert_eq!((requests.allocations, requests.releases), (1, 1));
        assert_eq!(requests.allocated[0], requests.released[0]);
        let ((), requests) = observe(false, || compiler.abort(held).unwrap());
        released(requests, held_allocation);
        assert_eq!(counts.cursors.load(Ordering::SeqCst), 1);
        assert_eq!(counts.aborts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn generic_pending_failure_and_completion_keep_one_aligned_allocation() {
        fn shareable<T: Send + Sync>() {}
        shareable::<HostBindingArtifact>();
        let counts = Arc::new(Counts::default());
        let compiler = HostBindingCompilerRegistration::new(Compiler(counts.clone()));
        let source = plan("native:address");
        let input = input(&source);
        let (mut cursor, requests) = observe(false, || compiler.start(&input).unwrap());
        let allocation = acquired(requests, compiler.slot_layout(), true);
        assert_eq!(allocation.align, 128);
        for _ in 0..4 {
            let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(0)));
            no_requests(requests);
            let BindingCompilerStep::Pending(returned) = step else {
                panic!("unpaid")
            };
            cursor = returned;
        }
        assert_eq!(counts.steps.load(Ordering::SeqCst), 0);
        counts.reject_step.store(true, Ordering::SeqCst);
        for _ in 0..3 {
            let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(1)));
            no_requests(requests);
            let BindingCompilerStep::Failed(returned) = step else {
                panic!("failed with retained owner")
            };
            cursor = returned.into_parts().1;
        }
        counts.reject_step.store(false, Ordering::SeqCst);
        for _ in 0..2 {
            let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(1)));
            no_requests(requests);
            let BindingCompilerStep::Pending(returned) = step else {
                panic!("paid pending")
            };
            cursor = returned;
        }
        let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(1)));
        no_requests(requests);
        let BindingCompilerStep::Complete(output) = step else {
            panic!("complete")
        };
        assert!(output.artifact().route_reservation().is_some());
        assert_eq!(counts.cursors.load(Ordering::SeqCst), 1);
        let artifact = output.into_artifact();
        assert_eq!(
            artifact.payload().storage_overhead() + std::mem::size_of::<Artifact>() as u64,
            allocation.size as u64,
        );
        drop(source);
        drop(compiler);
        let payload = artifact.try_payload::<Artifact>(FORMAT).unwrap();
        let address = payload as *const Artifact as usize;
        assert_eq!(address % 128, 0);
        assert!(
            address >= allocation.address
                && address + std::mem::size_of::<Artifact>()
                    <= allocation.address + allocation.size
        );
        assert_eq!(payload.instructions, [3; 257]);
        let (artifact, requests) = observe(false, || {
            artifact.try_into_payload::<u8>(FORMAT).unwrap_err()
        });
        no_requests(requests);
        let (artifact, requests) = observe(false, || {
            artifact
                .try_into_payload::<Artifact>(BindingArtifactCompatibility::new([0; 16]))
                .err()
                .unwrap()
        });
        no_requests(requests);
        // Shared access from another thread is valid although the old cursor
        // was !Sync. The allocation now contains only the completed artifact.
        std::thread::scope(|scope| {
            let task = scope.spawn(|| {
                artifact
                    .try_payload::<Artifact>(FORMAT)
                    .unwrap()
                    .instructions[0]
            });
            assert_eq!(task.join().unwrap(), 3);
        });
        let (payload, requests) = observe(false, || {
            artifact.try_into_payload::<Artifact>(FORMAT).ok().unwrap()
        });
        released(requests, allocation);
        assert_eq!(counts.artifacts.load(Ordering::SeqCst), 0);
        drop(payload);
        assert_eq!(counts.artifacts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn generic_mismatch_with_identical_native_types_returns_the_original_job() {
        struct OtherCompiler;
        impl BindingCompilerExtension for OtherCompiler {
            type Cursor = Cursor;
            type Artifact = Artifact;
            fn compatibility(&self) -> BindingArtifactCompatibility {
                FORMAT
            }
            fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
                panic!("wrong compiler")
            }
            fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<Cursor> {
                panic!("wrong compiler")
            }
            fn step(
                &self,
                _: &BindingCompilerInput<'_>,
                _: Cursor,
                _: &mut WorkBudget,
            ) -> BindingCompilerStep<Cursor, Artifact> {
                panic!("wrong compiler")
            }
            fn abort(&self, _: Cursor) {
                panic!("wrong compiler")
            }
        }
        let counts = Arc::new(Counts::default());
        let compiler = HostBindingCompilerRegistration::new(Compiler(counts.clone()));
        let wrong = HostBindingCompilerRegistration::new(OtherCompiler);
        let source = plan("native:address");
        let input = input(&source);
        let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
        let allocation = acquired(requests, compiler.slot_layout(), true);
        let (step, requests) = observe(false, || wrong.step(&input, cursor, &mut work(1)));
        no_requests(requests);
        let BindingCompilerStep::Failed(rejected) = step else {
            panic!("compiler type mismatch")
        };
        let (cursor, requests) =
            observe(false, || wrong.abort(rejected.into_parts().1).unwrap_err());
        no_requests(requests);
        let ((), requests) = observe(false, || drop(cursor));
        released(requests, allocation);
        assert_eq!(counts.cursors.load(Ordering::SeqCst), 1);
        assert_eq!(counts.aborts.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn generic_drop_of_completed_output_releases_slot_and_native_payload_once() {
        let counts = Arc::new(Counts::default());
        let compiler = HostBindingCompilerRegistration::new(Compiler(counts.clone()));
        let source = plan("native:address");
        let input = input(&source);
        let (mut cursor, requests) = observe(false, || compiler.start(&input).unwrap());
        let allocation = acquired(requests, compiler.slot_layout(), true);
        for _ in 0..2 {
            let BindingCompilerStep::Pending(returned) =
                compiler.step(&input, cursor, &mut work(1))
            else {
                panic!("pending")
            };
            cursor = returned;
        }
        let BindingCompilerStep::Complete(output) = compiler.step(&input, cursor, &mut work(1))
        else {
            panic!("complete")
        };
        let ((), requests) = observe(false, || drop(output));
        released(requests, allocation);
        assert_eq!(counts.artifacts.load(Ordering::SeqCst), 1);
        assert_eq!(counts.cursors.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn zero_sized_compiler_cursor_and_payload_need_only_the_retained_output_slot() {
        struct UnitCompiler;
        impl BindingCompilerExtension for UnitCompiler {
            type Cursor = ();
            type Artifact = ();
            fn compatibility(&self) -> BindingArtifactCompatibility {
                FORMAT
            }
            fn bounds(&self, _: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
                Ok(BindingCompilerBounds::new(
                    BindingArtifactFootprint::new(1, 0),
                    0,
                    0,
                    work(1),
                ))
            }
            fn start(&self, _: &BindingCompilerInput<'_>) -> CoreResult<()> {
                Ok(())
            }
            fn step(
                &self,
                _: &BindingCompilerInput<'_>,
                cursor: (),
                budget: &mut WorkBudget,
            ) -> BindingCompilerStep<(), ()> {
                if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                    return BindingCompilerStep::Pending(cursor);
                }
                BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                    FORMAT,
                    BindingArtifactFootprint::new(1, 0),
                    (),
                )))
            }
            fn abort(&self, _: ()) {}
        }
        let (compiler, requests) = observe(false, || {
            HostBindingCompilerRegistration::try_new(UnitCompiler)
                .ok()
                .unwrap()
        });
        no_requests(requests);
        let source = plan("native:address");
        let input = input(&source);
        let (cursor, requests) = observe(false, || compiler.start(&input).unwrap());
        let allocation = acquired(requests, compiler.slot_layout(), true);
        let (step, requests) = observe(false, || compiler.step(&input, cursor, &mut work(1)));
        no_requests(requests);
        let BindingCompilerStep::Complete(output) = step else {
            panic!("complete")
        };
        assert_eq!(
            output.artifact().payload().storage_overhead(),
            allocation.size as u64
        );
        let ((), requests) = observe(false, || {
            output
                .into_artifact()
                .try_into_payload::<()>(FORMAT)
                .unwrap()
        });
        released(requests, allocation);
    }
}
