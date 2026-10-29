//! Production construction evidence, reusing the existing real Core helpers.
//! Observation is thread-local and fixed-size; no allocator/policy is exported.
use super::*;

#[cfg(feature = "std")]
use clinkz_wot_planning::{HostConsumerBindingPreparation, HostConsumerBindingPreparationInput};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    marker::PhantomPinned,
    mem::{align_of, size_of},
    sync::Mutex,
};

#[derive(Clone, Copy, Default, Debug)]
struct Trace {
    armed: bool,
    fail: Option<usize>,
    requests: usize,
    after_acceptance: usize,
    layouts: [(usize, usize); 8],
}
#[derive(Clone, Copy, Default, Debug)]
struct Backing {
    addresses: [usize; 8],
    layouts: [(usize, usize); 8],
    releases: [usize; 8],
    live: usize,
    peak: usize,
    mismatch: bool,
}
std::thread_local! {
    static TRACE: Cell<Trace> = Cell::new(Trace::default());
    static BACKING: Cell<Backing> = Cell::new(Backing::default());
    static ACCEPTED: Cell<bool> = const { Cell::new(false) };
    static ZST_DROPS: Cell<usize> = const { Cell::new(0) };
    static REQUEST_RELEASE: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}
struct Observer;
#[global_allocator]
static ALLOCATOR: Observer = Observer;
unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = TRACE
            .try_with(|trace| {
                let mut t = trace.get();
                if !t.armed {
                    return false;
                }
                let index = t.requests;
                if index < t.layouts.len() {
                    t.layouts[index] = (layout.size(), layout.align());
                }
                t.requests += 1;
                if ACCEPTED.with(Cell::get) {
                    t.after_acceptance += 1;
                }
                let fail = t.fail == Some(index);
                trace.set(t);
                fail
            })
            .unwrap_or(false);
        if fail {
            return core::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACE.try_with(|t| t.get().armed).unwrap_or(false) {
            BACKING.with(|backing| {
                let mut b = backing.get();
                if let Some(i) = b.addresses.iter().position(|a| *a == 0) {
                    b.addresses[i] = pointer as usize;
                    b.layouts[i] = (layout.size(), layout.align());
                    b.live += layout.size();
                    b.peak = b.peak.max(b.live);
                } else {
                    b.mismatch = true;
                }
                backing.set(b);
            });
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let _ = REQUEST_RELEASE.try_with(|r| {
            let (address, count) = r.get();
            if address != 0 && address == pointer as usize {
                r.set((address, count + 1));
            }
        });
        let _ = BACKING.try_with(|backing| {
            let mut b = backing.get();
            if let Some(i) = (0..b.addresses.len())
                .find(|i| b.addresses[*i] == pointer as usize && b.releases[*i] == 0)
            {
                b.releases[i] += 1;
                b.mismatch |= b.layouts[i] != (layout.size(), layout.align());
                b.live = b.live.saturating_sub(layout.size());
            }
            backing.set(b);
        });
        unsafe { System.dealloc(pointer, layout) }
    }
}
fn observe(fail: Option<usize>) {
    TRACE.with(|t| {
        t.set(Trace {
            armed: true,
            fail,
            ..Trace::default()
        })
    });
}
fn disarm() -> Trace {
    TRACE.with(|t| t.replace(Trace::default()))
}
fn reset_backing() {
    BACKING.with(|b| {
        assert_eq!(
            b.get().live,
            0,
            "release the original owners before another interval"
        );
        b.set(Backing::default());
    });
    ACCEPTED.with(|a| a.set(false));
}
fn assert_released(expected: usize) {
    BACKING.with(|b| {
        let b = b.get();
        assert!(!b.mismatch, "{b:?}");
        assert_eq!(b.live, 0, "{b:?}");
        assert_eq!(b.addresses.iter().filter(|p| **p != 0).count(), expected);
        for (p, n) in b.addresses.into_iter().zip(b.releases) {
            assert_eq!(n, usize::from(p != 0), "{b:?}");
        }
    });
}

#[derive(Default)]
struct Stats {
    invokes: AtomicUsize,
    footprints: AtomicUsize,
    deadlines: AtomicUsize,
    polls: AtomicUsize,
    cancels: AtomicUsize,
    drops: AtomicUsize,
}
#[derive(Clone, Copy)]
enum Mode {
    Result,
    LateStart,
    LatePoll,
    Cancel,
    Transfer,
}
struct Client {
    output: Mutex<Option<CoreResult<InteractionOutput>>>,
    stats: Arc<Stats>,
    footprint: BindingLifetimeFootprint,
    mode: Mode,
}
struct Call {
    request: Option<OutboundRequest>,
    output: Cell<Option<CoreResult<InteractionOutput>>>,
    stats: Arc<Stats>,
    footprint: BindingLifetimeFootprint,
    mode: Mode,
    address: Cell<usize>,
    _pin: PhantomPinned,
}
impl Call {
    fn check_address(&self) {
        let address = self as *const Self as usize;
        let original = self.address.replace(address);
        assert!(
            original == 0 || original == address,
            "pinned raw state moved"
        );
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        self.check_address();
        self.stats.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl HostBindingCall<CoreResult<InteractionOutput>> for Call {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        self.check_address();
        self.stats.footprints.fetch_add(1, Ordering::SeqCst);
        self.footprint
    }
    fn next_deadline(&self) -> Option<Deadline> {
        self.check_address();
        self.stats.deadlines.fetch_add(1, Ordering::SeqCst);
        self.request.as_ref().unwrap().deadline()
    }
    fn poll_result(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.check_address();
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.stats.polls.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(self.output.take().unwrap())
    }
    fn start_cancel(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.check_address();
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        self.stats.cancels.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            Mode::LateStart => Ok(StartStatus::Ready(BindingCallSettlement::Returned(
                self.output.take().unwrap(),
            ))),
            Mode::Transfer => Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::TransferRequired(
                    cleanup.try_into_transfer_request().unwrap(),
                ),
            })),
            _ => Ok(StartStatus::Pending),
        }
    }
    fn poll_cancel(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.check_address();
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.stats.cancels.fetch_add(1, Ordering::SeqCst);
        if matches!(self.mode, Mode::LatePoll | Mode::Transfer) {
            Poll::Ready(Ok(BindingCallSettlement::Returned(
                self.output.take().unwrap(),
            )))
        } else {
            Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::Complete {
                    successor: NoCleanupSuccessor,
                },
            }))
        }
    }
}
impl TargetClientBinding for Client {
    fn supports_operation(&self, operation: clinkz_wot_core::Operation) -> bool {
        operation == clinkz_wot_core::Operation::ReadProperty
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        _: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<ConsumerHostCall, BindingInputRejection<OutboundRequest>> {
        self.stats.invokes.fetch_add(1, Ordering::SeqCst);
        // One finite input and one prebuilt response; no protocol I/O, waiting,
        // output construction or other allocation in this concrete callback.
        if request.uri_variables().len() != 1
            || request
                .uri_variables()
                .get("room")
                .is_none_or(|v| v != "west")
        {
            return Err(BindingInputRejection::new(request, binding_error(410)));
        }
        let Ok(mut source) = self.output.try_lock() else {
            return Err(BindingInputRejection::new(request, binding_error(410)));
        };
        let Some(output) = source.take() else {
            return Err(BindingInputRejection::new(request, binding_error(410)));
        };
        match HostBindingCallBox::try_new(Call {
            request: Some(request),
            output: Cell::new(Some(output)),
            stats: self.stats.clone(),
            footprint: self.footprint,
            mode: self.mode,
            address: Cell::new(0),
            _pin: PhantomPinned,
        }) {
            Ok(call) => {
                ACCEPTED.with(|a| a.set(true));
                Ok(call)
            }
            Err(mut call) => {
                let request = call.request.take().unwrap();
                let error = BindingOperationalError::new(CoreError::Backpressure(
                    ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
                        .with_plan(request.plan_id())
                        .with_binding(request.binding_id(), request.binding_generation())
                        .with_redacted_cause(410, "fixed raw call backing unavailable"),
                ));
                Err(BindingInputRejection::new(request, error))
            }
        }
    }
}

// Test-only dispatch across the two unchanged complete public registrations.
enum Owner {
    Dual(HostBindingPreparation),
    Consumer(HostConsumerBindingPreparation),
}
impl Owner {
    fn start(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<ConsumerHostCall, BindingInputRejection<OutboundRequest>> {
        match self {
            Self::Dual(o) => o.start_consumer_property_read(request, artifact),
            Self::Consumer(o) => o.start_finite(request, artifact),
        }
    }
}
fn owner(
    consumer_only: bool,
    ceiling: BindingLifetimeFootprint,
    raw: BindingLifetimeFootprint,
    mode: Mode,
    kind: OutputKind,
    failure: bool,
) -> (Owner, Arc<Stats>) {
    let stats = Arc::new(Stats::default());
    let client = Box::new(Client {
        output: Mutex::new(Some(if failure {
            Err(binding_error(412).into_parts().1)
        } else {
            Ok(output_for(&request(72), kind))
        })),
        stats: stats.clone(),
        footprint: raw,
        mode,
    });
    let resources = resources_with_admitted(ceiling, BindingTransientFootprint::new(2048));
    let compiler = HostBindingCompilerRegistration::new(TestCompiler);
    let registration = if consumer_only {
        Owner::Consumer(
            HostConsumerBindingPreparation::new(HostConsumerBindingPreparationInput::new(
                registration_identity(COMPATIBILITY),
                BindingExecutionSupport::host_erased(),
                compiler,
                client,
                resources,
                BindingStatusPolicy::new(1, 64),
            ))
            .unwrap_or_else(|_| panic!("complete Consumer owner")),
        )
    } else {
        Owner::Dual(
            HostBindingPreparation::new(
                HostBindingPreparationInput::producer_and_consumer_property_read(
                    registration_identity(COMPATIBILITY),
                    BindingExecutionSupport::host_erased(),
                    compiler,
                    Box::new(NoopHostServer {
                        compatibility: COMPATIBILITY,
                    }),
                    client,
                    resources,
                    BindingIngressPolicy::hidden(),
                    BindingStatusPolicy::new(1, 64),
                ),
            )
            .unwrap_or_else(|_| panic!("complete dual-role owner")),
        )
    };
    (registration, stats)
}
const RAW: BindingLifetimeFootprint = BindingLifetimeFootprint::new(7, 2048);
const CEILING: BindingLifetimeFootprint = BindingLifetimeFootprint::new(8, 4096);

fn cleanup_context(transfer: bool) -> CleanupPhaseContext {
    let reservation = CleanupReservation::new(
        CleanupSlotId::new(SlotIndex::new(59), Generation::INITIAL),
        CEILING,
        1,
        binding_budget(2),
    );
    let cause = CoreError::TimedOut(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never));
    if transfer {
        CleanupPhaseContext::bind_with_transfer_owner(
            reservation,
            CleanupSlotId::new(SlotIndex::new(60), Generation::INITIAL),
            CleanupOperation::CancelRequest,
            cause,
            Deadline::NONE,
        )
    } else {
        CleanupPhaseContext::bind(
            reservation,
            CleanupOperation::CancelRequest,
            cause,
            Deadline::NONE,
        )
    }
}

#[test]
fn both_backing_failures_return_the_exact_request_before_protocol_work() {
    let artifact = host_artifact(72);
    for consumer_only in [false, true] {
        for overrun in [false, true] {
            for fail in [0, 1] {
                let (owner, stats) = owner(
                    consumer_only,
                    if overrun {
                        BindingLifetimeFootprint::new(1, 64)
                    } else {
                        CEILING
                    },
                    RAW,
                    Mode::Result,
                    OutputKind::Valid,
                    false,
                );
                let input = request(72);
                let expected = request(72);
                let variable = input.uri_variables().get("room").unwrap().as_ptr();
                REQUEST_RELEASE.with(|r| r.set((variable as usize, 0)));
                reset_backing();
                observe(Some(fail));
                let rejection = owner.start(input, &artifact).unwrap_err();
                let trace = disarm();
                let error = rejection.error().error();
                assert!(matches!(error, CoreError::Backpressure(_)));
                assert_eq!(error.context().phase(), ErrorPhase::Binding);
                assert_eq!(error.context().plan(), Some(expected.plan_id()));
                assert_eq!(
                    error.context().binding(),
                    Some((expected.binding_id(), expected.binding_generation()))
                );
                let returned = rejection.into_input();
                assert_eq!(returned, expected);
                assert_eq!(
                    returned.uri_variables().get("room").unwrap().as_ptr(),
                    variable
                );
                assert_eq!(trace.requests, fail + 1);
                assert_eq!(trace.after_acceptance, 0);
                assert!(!ACCEPTED.with(Cell::get));
                assert_eq!(stats.invokes.load(Ordering::SeqCst), fail);
                assert_eq!(stats.drops.load(Ordering::SeqCst), fail);
                assert_eq!(stats.footprints.load(Ordering::SeqCst), 0);
                assert_eq!(stats.polls.load(Ordering::SeqCst), 0);
                assert_eq!(stats.cancels.load(Ordering::SeqCst), 0);
                assert_released(fail);
                assert_eq!(REQUEST_RELEASE.with(|r| r.get().1), 0);
                drop(returned);
                assert_eq!(REQUEST_RELEASE.with(|r| r.get().1), 1);
                REQUEST_RELEASE.with(|r| r.set((0, 0)));
            }
        }
    }
}

#[test]
fn both_accepted_branches_have_zero_allocation_after_acceptance_and_exact_release() {
    let artifact = host_artifact(72);
    for consumer_only in [false, true] {
        for raw in [
            RAW,
            BindingLifetimeFootprint::new(u32::MAX, 2048),
            BindingLifetimeFootprint::new(7, u64::MAX),
            BindingLifetimeFootprint::new(u32::MAX, u64::MAX),
        ] {
            let (owner, stats) = owner(
                consumer_only,
                CEILING,
                raw,
                Mode::Result,
                OutputKind::Valid,
                false,
            );
            let input = request(72);
            reset_backing();
            // Site 2 must never be attempted, including during terminal progress.
            observe(Some(2));
            let result = owner.start(input, &artifact);
            let return_layout = Layout::for_value(&result);
            let mut call = result.unwrap();
            let footprint = call.as_pin_mut().lifetime_footprint();
            let trace = disarm();
            assert_eq!(trace.requests, 2);
            assert_eq!(trace.after_acceptance, 0);
            assert_eq!(trace.layouts[1], (size_of::<Call>(), align_of::<Call>()));
            assert_eq!(stats.footprints.load(Ordering::SeqCst), 1);
            assert_eq!(stats.polls.load(Ordering::SeqCst), 0);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
            if raw == RAW {
                assert_eq!(
                    footprint,
                    BindingLifetimeFootprint::new(8, 2048 + trace.layouts[0].0 as u64)
                );
            } else {
                assert_eq!(footprint, BindingLifetimeFootprint::new(u32::MAX, u64::MAX));
            }
            let peak = BACKING.with(|b| b.get().peak);
            assert_eq!(peak, trace.layouts[0].0 + trace.layouts[1].0);
            observe(Some(0));
            let result = call.as_pin_mut().poll_result(
                &mut Context::from_waker(Waker::noop()),
                &mut binding_budget(1),
            );
            assert_eq!(disarm().requests, 0);
            assert_eq!(matches!(result, Poll::Ready(Ok(_))), raw == RAW);
            assert_eq!(stats.polls.load(Ordering::SeqCst), usize::from(raw == RAW));
            assert_eq!(
                stats.drops.load(Ordering::SeqCst),
                0,
                "terminal reporting retains raw ownership"
            );
            assert_eq!(BACKING.with(|b| b.get().live), peak);
            observe(Some(0));
            drop(call);
            assert_eq!(
                disarm().requests,
                0,
                "finite terminal destruction does not allocate"
            );
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
            assert_released(2);
            println!(
                "consumer_only={consumer_only}, raw={raw:?}, backing={:?}, call={:?}, return={return_layout:?}, heap_overlap={peak}",
                trace.layouts,
                Layout::new::<ConsumerHostCall>()
            );
        }
    }
}

#[test]
fn identity_barrier_raw_rejection_and_byte_item_boundaries_preserve_owners() {
    let artifact = host_artifact(72);
    for consumer_only in [false, true] {
        let (registration, stats) = owner(
            consumer_only,
            CEILING,
            RAW,
            Mode::Result,
            OutputKind::Valid,
            false,
        );
        let input = request(73);
        let expected = request(73);
        reset_backing();
        observe(Some(0));
        let rejected = registration.start(input, &artifact).unwrap_err();
        assert_eq!(disarm().requests, 0);
        assert_eq!(rejected.into_input(), expected);
        assert_eq!(stats.invokes.load(Ordering::SeqCst), 0);
        let input = request(72);
        observe(None);
        let mut call = registration.start(input, &artifact).unwrap();
        let envelope = disarm().layouts[0].0 as u64;
        assert!(
            call.as_pin_mut()
                .poll_result(
                    &mut Context::from_waker(Waker::noop()),
                    &mut binding_budget(1)
                )
                .is_ready()
        );
        drop(call);
        assert_released(2);
        // The one-element response source is now empty: raw rejection returns
        // the exact second request and releases only the vacant Core envelope.
        let input = request(72);
        let variable = input.uri_variables().get("room").unwrap().as_ptr();
        reset_backing();
        observe(None);
        let rejected = registration.start(input, &artifact).unwrap_err();
        assert_eq!(disarm().requests, 1);
        assert_eq!(
            rejected
                .input()
                .uri_variables()
                .get("room")
                .unwrap()
                .as_ptr(),
            variable
        );
        assert_eq!(rejected.into_input(), request(72));
        assert_released(1);
        for (items, bytes, accepted) in [
            (8, 2048 + envelope - 1, false),
            (8, 2048 + envelope, true),
            (8, 2048 + envelope + 1, true),
            (7, 2048 + envelope, false),
            (9, 2048 + envelope, true),
        ] {
            let (owner, stats) = owner(
                consumer_only,
                BindingLifetimeFootprint::new(items, bytes),
                RAW,
                Mode::Result,
                OutputKind::Valid,
                false,
            );
            let mut call = owner.start(request(72), &artifact).unwrap();
            assert_eq!(
                call.as_pin_mut().lifetime_footprint(),
                BindingLifetimeFootprint::new(8, 2048 + envelope)
            );
            let result = call.as_pin_mut().poll_result(
                &mut Context::from_waker(Waker::noop()),
                &mut binding_budget(1),
            );
            assert_eq!(matches!(result, Poll::Ready(Ok(_))), accepted);
            assert_eq!(stats.polls.load(Ordering::SeqCst), usize::from(accepted));
            assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
            drop(call);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
        }
    }
}

#[test]
fn normal_and_both_late_paths_preserve_validation_raw_errors_budget_and_deadline() {
    let artifact = host_artifact(72);
    let kinds = [
        OutputKind::Valid,
        OutputKind::NativeStatus(599),
        OutputKind::BindingId,
        OutputKind::BindingGeneration,
        OutputKind::PlanId,
        OutputKind::ResponseSelection,
        OutputKind::MissingMetadata,
        OutputKind::MissingPayload,
        OutputKind::Status,
        OutputKind::PayloadRole,
        OutputKind::ActionReference,
    ];
    for consumer_only in [false, true] {
        for mode in [Mode::Result, Mode::LateStart, Mode::LatePoll] {
            for (kind, failure) in kinds
                .into_iter()
                .map(|k| (k, false))
                .chain([(OutputKind::Valid, true)])
            {
                let (owner, stats) = owner(consumer_only, CEILING, RAW, mode, kind, failure);
                let mut call = owner.start(request(72), &artifact).unwrap();
                let mut cx = Context::from_waker(Waker::noop());
                let deadline = request(72).deadline();
                observe(Some(0));
                assert_eq!(call.as_pin_mut().next_deadline(), deadline);
                assert!(
                    call.as_pin_mut()
                        .poll_result(&mut cx, &mut binding_budget(0))
                        .is_pending()
                );
                let result = match mode {
                    Mode::Result => match call
                        .as_pin_mut()
                        .poll_result(&mut cx, &mut binding_budget(1))
                    {
                        Poll::Ready(r) => r,
                        _ => panic!("paid normal result"),
                    },
                    _ => {
                        assert!(matches!(
                            call.as_pin_mut()
                                .start_cancel(
                                    &mut cx,
                                    cleanup_context(false),
                                    &mut binding_budget(0)
                                )
                                .unwrap(),
                            StartStatus::Pending
                        ));
                        let started = call
                            .as_pin_mut()
                            .start_cancel(&mut cx, cleanup_context(false), &mut binding_budget(1))
                            .unwrap();
                        match started {
                            StartStatus::Ready(BindingCallSettlement::Returned(r)) => r,
                            StartStatus::Pending => {
                                assert!(
                                    call.as_pin_mut()
                                        .poll_cancel(&mut cx, &mut binding_budget(0))
                                        .is_pending()
                                );
                                match call
                                    .as_pin_mut()
                                    .poll_cancel(&mut cx, &mut binding_budget(1))
                                {
                                    Poll::Ready(Ok(BindingCallSettlement::Returned(r))) => r,
                                    _ => panic!("paid late result stays Returned"),
                                }
                            }
                            _ => panic!("late result stays Returned"),
                        }
                    }
                };
                assert_eq!(disarm().requests, 0);
                if failure {
                    assert_eq!(result.unwrap_err(), binding_error(412).into_parts().1);
                } else if matches!(kind, OutputKind::Valid | OutputKind::NativeStatus(_)) {
                    let output = result.unwrap();
                    assert_eq!(
                        output.metadata().binding_response().unwrap().status_code(),
                        if matches!(kind, OutputKind::NativeStatus(_)) {
                            599
                        } else {
                            200
                        }
                    );
                } else {
                    assert!(matches!(result, Err(CoreError::Validation(_))));
                }
                assert_eq!(stats.deadlines.load(Ordering::SeqCst), 1);
                assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
                drop(call);
                assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
            }
        }
    }
}

#[test]
fn cancellation_and_rejected_accepted_settlement_retain_raw_until_explicit_release() {
    let artifact = host_artifact(72);
    let mut cx = Context::from_waker(Waker::noop());
    for consumer_only in [false, true] {
        for overrun in [false, true] {
            let (owner, stats) = owner(
                consumer_only,
                CEILING,
                if overrun {
                    BindingLifetimeFootprint::new(u32::MAX, u64::MAX)
                } else {
                    RAW
                },
                Mode::Cancel,
                OutputKind::Valid,
                false,
            );
            let input = request(72);
            reset_backing();
            observe(None);
            let mut call = owner.start(input, &artifact).unwrap();
            assert_eq!(disarm().requests, 2);
            observe(Some(0));
            let started = call
                .as_pin_mut()
                .start_cancel(&mut cx, cleanup_context(false), &mut binding_budget(1))
                .unwrap();
            if overrun {
                assert_eq!(call.as_pin_mut().next_deadline(), None);
                assert!(matches!(
                    started,
                    StartStatus::Ready(BindingCallSettlement::Returned(Err(
                        CoreError::Validation(_)
                    )))
                ));
                assert!(matches!(
                    call.as_pin_mut()
                        .poll_cancel(&mut cx, &mut binding_budget(1)),
                    Poll::Ready(Err(CoreError::Validation(_)))
                ));
                assert_eq!(stats.cancels.load(Ordering::SeqCst), 0);
                assert_eq!(stats.deadlines.load(Ordering::SeqCst), 0);
            } else {
                assert!(matches!(started, StartStatus::Pending));
                assert!(
                    call.as_pin_mut()
                        .poll_cancel(&mut cx, &mut binding_budget(0))
                        .is_pending()
                );
                assert!(matches!(
                    call.as_pin_mut()
                        .poll_cancel(&mut cx, &mut binding_budget(1)),
                    Poll::Ready(Ok(BindingCallSettlement::Cancelled {
                        retry_class: RetryClass::Never,
                        disposition: BindingCancellationDisposition::Complete { .. },
                    }))
                ));
                assert_eq!(stats.cancels.load(Ordering::SeqCst), 2);
            }
            assert_eq!(disarm().requests, 0);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
            drop(call);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
            assert_released(2);
        }
    }
}

#[test]
fn cleanup_offers_preserve_complete_call_first_cause_accounting_and_unused_seal() {
    let artifact = host_artifact(72);
    let mut cx = Context::from_waker(Waker::noop());
    for consumer_only in [false, true] {
        for accept in [false, true] {
            let (owner, stats) = owner(
                consumer_only,
                CEILING,
                RAW,
                Mode::Transfer,
                OutputKind::Valid,
                false,
            );
            let input = request(72);
            reset_backing();
            observe(None);
            let mut call = owner.start(input, &artifact).unwrap();
            assert_eq!(disarm().requests, 2);
            let footprint = call.as_pin_mut().lifetime_footprint();
            let phase = cleanup_context(true);
            let expected_phase = cleanup_context(true);
            observe(Some(0));
            let StartStatus::Ready(BindingCallSettlement::Cancelled {
                disposition: BindingCancellationDisposition::TransferRequired(transfer),
                ..
            }) = call
                .as_pin_mut()
                .start_cancel(&mut cx, phase, &mut binding_budget(1))
                .unwrap()
            else {
                panic!("complete transfer request");
            };
            let envelope = CleanupTransferEnvelope::new(transfer, call);
            let envelope = if accept {
                let mut target = AcceptConsumerTransfer { accepted: None };
                assert!(matches!(
                    target.try_accept(envelope),
                    CleanupTransferAcceptance::Accepted(_)
                ));
                target.accepted.take().unwrap()
            } else {
                let CleanupTransferAcceptance::Rejected(envelope) =
                    RejectConsumerTransfer.try_accept(envelope)
                else {
                    panic!("rejection returns original owner");
                };
                envelope
            };
            assert_eq!(envelope.request().phase(), &expected_phase);
            let (_, mut retained) = envelope.into_parts();
            assert_eq!(retained.as_pin_mut().lifetime_footprint(), footprint);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
            // A provisional transfer does not consume the result seal. Original
            // owner can still receive a late validated result after succession.
            assert!(matches!(
                retained
                    .as_pin_mut()
                    .poll_cancel(&mut cx, &mut binding_budget(1)),
                Poll::Ready(Ok(BindingCallSettlement::Returned(Ok(_))))
            ));
            assert_eq!(disarm().requests, 0);
            drop(retained);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
            assert_released(2);
        }
    }
}

struct ZeroCall;
impl Drop for ZeroCall {
    fn drop(&mut self) {
        ZST_DROPS.with(|n| n.set(n.get() + 1));
    }
}
#[repr(align(256))]
struct PinnedCall {
    request: OutboundRequest,
    stats: Arc<Stats>,
    address: Cell<usize>,
    _pin: PhantomPinned,
}
impl PinnedCall {
    fn check_address(&self) {
        let address = self as *const Self as usize;
        assert_eq!(address % align_of::<Self>(), 0);
        let original = self.address.replace(address);
        assert!(
            original == 0 || original == address,
            "pinned concrete state moved"
        );
    }
}
impl Drop for PinnedCall {
    fn drop(&mut self) {
        self.check_address();
        self.stats.drops.fetch_add(1, Ordering::SeqCst);
    }
}
macro_rules! constructor_call {
    ($ty:ty, $check:expr) => {
        impl HostBindingCall<()> for $ty {
            fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
                BindingLifetimeFootprint::new(1, size_of::<Self>() as u64)
            }
            fn next_deadline(&self) -> Option<Deadline> {
                None
            }
            fn poll_result(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &mut WorkBudget,
            ) -> Poll<()> {
                ($check)(self.as_ref().get_ref());
                Poll::Ready(())
            }
            fn start_cancel(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: CleanupPhaseContext,
                _: &mut WorkBudget,
            ) -> CoreResult<StartStatus<BindingCallSettlement<()>>> {
                Ok(StartStatus::Pending)
            }
            fn poll_cancel(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &mut WorkBudget,
            ) -> Poll<CoreResult<BindingCallSettlement<()>>> {
                Poll::Pending
            }
        }
    };
}
constructor_call!(ZeroCall, |_: &ZeroCall| {});
constructor_call!(PinnedCall, |call: &PinnedCall| call.check_address());

#[test]
fn public_try_new_handles_zst_overalignment_pinning_and_returns_original_concrete_owner() {
    ZST_DROPS.with(|n| n.set(0));
    reset_backing();
    observe(Some(0));
    let zero =
        HostBindingCallBox::try_new(ZeroCall).unwrap_or_else(|_| panic!("ZST needs no allocation"));
    let trace = disarm();
    assert_eq!(trace.requests, 0);
    drop(zero);
    assert_eq!(ZST_DROPS.with(Cell::get), 1);
    assert_released(0);
    for fail in [true, false] {
        let request = request(72);
        let expected = super::request(72);
        let variable = request.uri_variables().get("room").unwrap().as_ptr();
        let stats = Arc::new(Stats::default());
        let raw = PinnedCall {
            request,
            stats: stats.clone(),
            address: Cell::new(0),
            _pin: PhantomPinned,
        };
        reset_backing();
        observe(if fail { Some(0) } else { None });
        let result = HostBindingCallBox::try_new(raw);
        let trace = disarm();
        assert_eq!(trace.requests, 1);
        assert_eq!(
            trace.layouts[0],
            (size_of::<PinnedCall>(), align_of::<PinnedCall>())
        );
        if fail {
            let raw = result
                .err()
                .expect("null returns the original concrete owner");
            assert_eq!(raw.address.get(), 0, "failure never pins the owner");
            assert!(Arc::ptr_eq(&raw.stats, &stats));
            assert_eq!(raw.request, expected);
            assert_eq!(
                raw.request.uri_variables().get("room").unwrap().as_ptr(),
                variable
            );
            assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
            drop(raw);
            assert_released(0);
        } else {
            let mut call = result.unwrap_or_else(|_| panic!("one concrete allocation"));
            assert!(
                call.as_pin_mut()
                    .poll_result(
                        &mut Context::from_waker(Waker::noop()),
                        &mut binding_budget(0)
                    )
                    .is_ready()
            );
            // Move the owning erased box and poll again; concrete address stays fixed.
            let mut moved = Some(call).take().unwrap();
            assert!(
                moved
                    .as_pin_mut()
                    .poll_result(
                        &mut Context::from_waker(Waker::noop()),
                        &mut binding_budget(0)
                    )
                    .is_ready()
            );
            drop(moved);
            assert_released(1);
        }
        assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
        println!(
            "pinned-concrete={:?}, concrete-return={:?}, erased={:?}",
            Layout::new::<PinnedCall>(),
            Layout::new::<Result<HostBindingCallBox<()>, PinnedCall>>(),
            Layout::new::<HostBindingCallBox<()>>()
        );
    }
}

#[test]
fn finite_witness_records_real_request_output_and_inline_transport_layouts() {
    reset_backing();
    observe(None);
    let input = request(72);
    let request_trace = disarm();
    let request_bytes = BACKING.with(|b| b.get().peak);
    assert_eq!(request_trace.requests, 3); // fixed BTree node and its two strings
    drop(input);
    assert_released(3);

    let input = request(72);
    reset_backing();
    observe(None);
    let output = output_for(&input, OutputKind::Valid);
    let output_trace = disarm();
    let output_bytes = BACKING.with(|b| b.get().peak);
    assert_eq!(output_trace.requests, 3); // temporary Vec, Arc body, content type
    drop(output);
    assert_released(3);
    assert!(request_bytes + output_bytes + size_of::<Call>() <= RAW.retained_bytes() as usize);
    println!(
        "finite witness: request={:?} bytes={request_bytes}, output={:?} bytes={output_bytes}, raw={:?}, raw_return={:?}, request_owner={:?}, result_inline={:?}, cleanup_transport={:?}, client={:?}, shared_stats={:?}",
        request_trace.layouts,
        output_trace.layouts,
        Layout::new::<Call>(),
        Layout::new::<Result<ConsumerHostCall, Call>>(),
        Layout::new::<OutboundRequest>(),
        Layout::new::<CoreResult<InteractionOutput>>(),
        Layout::new::<CleanupTransferEnvelope<ConsumerHostCall>>(),
        Layout::new::<Client>(),
        Layout::new::<Stats>()
    );
}
