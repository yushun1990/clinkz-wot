// Appended only to the disposable Core integration test, sharing its real
// registration/artifact/cleanup helpers and unchanged Core result validators.
#[cfg(feature = "std")]
mod call_backing_candidate {
    use super::*;
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
        sync::Mutex,
    };

    #[derive(Clone, Copy, Default, Debug)]
    struct Trace {
        armed: bool,
        requests: usize,
        fail: Option<usize>,
        after_acceptance: usize,
        bytes: usize,
        layouts: [(usize, usize); 4],
    }
    std::thread_local! {
        static TRACE: Cell<Trace> = Cell::new(Trace::default());
        static ACCEPTED: Cell<bool> = const { Cell::new(false) };
        static BACKING: Cell<([usize; 4], [usize; 4])> = const { Cell::new(([0; 4], [0; 4])) };
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
                    t.bytes += layout.size();
                    if ACCEPTED.with(Cell::get) {
                        t.after_acceptance += 1;
                    }
                    let fail = t.fail == Some(index);
                    if fail {
                        t.armed = false;
                    }
                    trace.set(t);
                    fail
                })
                .unwrap_or(false);
            if fail {
                eprintln!("FAIL_AFTER_ACCEPTANCE={}", ACCEPTED.with(Cell::get));
                return core::ptr::null_mut();
            }
            let pointer = unsafe { System.alloc(layout) };
            if !pointer.is_null() && TRACE.try_with(|t| t.get().armed).unwrap_or(false) {
                BACKING.with(|b| {
                    let (mut addresses, releases) = b.get();
                    if let Some(i) = addresses.iter().position(|a| *a == 0) {
                        addresses[i] = pointer as usize;
                    }
                    b.set((addresses, releases));
                });
            }
            pointer
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            let _ = BACKING.try_with(|b| {
                let (addresses, mut releases) = b.get();
                if let Some(i) = addresses.iter().position(|a| *a == pointer as usize) {
                    releases[i] += 1;
                }
                b.set((addresses, releases));
            });
            unsafe { System.dealloc(pointer, layout) }
        }
    }
    fn observe(fail: Option<usize>) {
        ACCEPTED.with(|s| s.set(false));
        TRACE.with(|s| {
            s.set(Trace {
                armed: true,
                fail,
                ..Trace::default()
            })
        });
    }
    fn disarm() -> Trace {
        TRACE.with(|s| {
            let t = s.get();
            s.set(Trace::default());
            t
        })
    }
    fn alternative() -> String {
        std::env::var("CLINKZ_HOST_CALL_CANDIDATE").unwrap()
    }
    fn candidate() -> bool {
        alternative() != "baseline"
    }
    fn reset_backing() {
        BACKING.with(|b| b.set(([0; 4], [0; 4])));
    }
    fn assert_released(expected: usize) {
        BACKING.with(|b| {
            let (addresses, releases) = b.get();
            assert_eq!(addresses.iter().filter(|p| **p != 0).count(), expected);
            for (p, n) in addresses.into_iter().zip(releases) {
                assert_eq!(n, usize::from(p != 0));
            }
        });
    }
    fn raw_box<B: HostBindingCall<CoreResult<InteractionOutput>>>(
        call: B,
    ) -> Result<HostBindingCallBox<CoreResult<InteractionOutput>>, B> {
        HOST_CALL_CONSTRUCT
    }
    #[derive(Default)]
    struct Stats {
        invokes: AtomicUsize,
        footprints: AtomicUsize,
        polls: AtomicUsize,
        drops: AtomicUsize,
    }
    struct Client {
        output: Mutex<Option<InteractionOutput>>,
        stats: Arc<Stats>,
        footprint: BindingLifetimeFootprint,
    }
    struct Call {
        request: Option<OutboundRequest>,
        output: Option<InteractionOutput>,
        stats: Arc<Stats>,
        footprint: BindingLifetimeFootprint,
    }
    impl Drop for Call {
        fn drop(&mut self) {
            self.stats.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl HostBindingCall<CoreResult<InteractionOutput>> for Call {
        fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
            self.stats.footprints.fetch_add(1, Ordering::SeqCst);
            self.footprint
        }
        fn next_deadline(&self) -> Option<Deadline> {
            None
        }
        fn poll_result(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<CoreResult<InteractionOutput>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            self.stats.polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Ok(self.output.take().unwrap()))
        }
        fn start_cancel(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: CleanupPhaseContext,
            budget: &mut WorkBudget,
        ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Ok(StartStatus::Pending);
            }
            self.stats.polls.fetch_add(1, Ordering::SeqCst);
            Ok(StartStatus::Ready(BindingCallSettlement::Returned(Ok(
                self.output.take().unwrap(),
            ))))
        }
        fn poll_cancel(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            budget: &mut WorkBudget,
        ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
            if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                return Poll::Pending;
            }
            self.stats.polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Ok(BindingCallSettlement::Returned(Ok(self
                .output
                .take()
                .unwrap()))))
        }
    }
    impl TargetClientBinding for Client {
        fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
            COMPATIBILITY
        }
        fn invoke(
            &self,
            request: OutboundRequest,
            _: &BindingArtifactEnvelope<HostBindingArtifact>,
        ) -> Result<
            HostBindingCallBox<CoreResult<InteractionOutput>>,
            BindingInputRejection<OutboundRequest>,
        > {
            self.stats.invokes.fetch_add(1, Ordering::SeqCst);
            if request.uri_variables().len() != 1
                || request
                    .uri_variables()
                    .get("room")
                    .is_none_or(|value| value != "west")
            {
                return Err(BindingInputRejection::new(request, binding_error(410)));
            }
            // Bounded, nonblocking one-element source. Pool exhaustion/contended
            // access rejects before moving the request or performing protocol work.
            let Ok(mut output) = self.output.try_lock() else {
                return Err(BindingInputRejection::new(request, binding_error(410)));
            };
            let Some(output) = output.take() else {
                return Err(BindingInputRejection::new(request, binding_error(410)));
            };
            let raw = Call {
                request: Some(request),
                output: Some(output),
                stats: self.stats.clone(),
                footprint: self.footprint,
            };
            match raw_box(raw) {
                Ok(call) => {
                    ACCEPTED.with(|s| s.set(true));
                    Ok(call)
                }
                Err(mut raw) => {
                    // The request leaves the failed concrete state unchanged.
                    let request = raw.request.take().unwrap();
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
    fn owner(
        ceiling: BindingLifetimeFootprint,
        raw: BindingLifetimeFootprint,
    ) -> (HostBindingRegistration, Arc<Stats>) {
        let stats = Arc::new(Stats::default());
        observe(None);
        let output = output_for(&request(72), OutputKind::Valid);
        let prepared_bytes = disarm().bytes;
        assert!(prepared_bytes + core::mem::size_of::<Call>() < 2048);
        let owner = HostBindingRegistration::new(
            HostBindingRegistrationInput::producer_and_consumer_property_read(
                registration_identity(COMPATIBILITY),
                BindingExecutionSupport::host_erased(),
                HostBindingCompilerRegistration::new(TestCompiler),
                Box::new(NoopHostServer {
                    compatibility: COMPATIBILITY,
                }),
                Box::new(Client {
                    output: Mutex::new(Some(output)),
                    stats: stats.clone(),
                    footprint: raw,
                }),
                resources_with_admitted(ceiling, BindingTransientFootprint::new(256)),
                BindingIngressPolicy::hidden(),
                BindingStatusPolicy::new(1, 64),
            ),
        )
        .unwrap_or_else(|_| panic!("complete fixture owner"));
        (owner, stats)
    }
    #[test]
    fn installed_construction_has_no_core_allocation_in_either_accepted_branch() {
        for raw in [
            BindingLifetimeFootprint::new(7, 2048),
            BindingLifetimeFootprint::new(u32::MAX, u64::MAX),
        ] {
            let (owner, stats) = owner(BindingLifetimeFootprint::new(8, 4096), raw);
            let artifact = host_artifact(72);
            let input = request(72);
            reset_backing();
            observe(None);
            let result = owner.start_consumer_property_read(input, &artifact);
            let return_layout = (
                core::mem::size_of_val(&result),
                core::mem::align_of_val(&result),
            );
            let mut call = result.unwrap();
            let trace = disarm();
            assert_eq!(
                trace.requests,
                if alternative() == "inline" { 1 } else { 2 }
            );
            assert_eq!(trace.after_acceptance, if candidate() { 0 } else { 1 });
            assert_eq!(stats.footprints.load(Ordering::SeqCst), 1);
            let mut cx = Context::from_waker(Waker::noop());
            observe(None);
            let result = call
                .as_pin_mut()
                .poll_result(&mut cx, &mut binding_budget(1));
            assert_eq!(disarm().requests, 0);
            assert_eq!(
                stats.polls.load(Ordering::SeqCst),
                usize::from(raw.retained_items() == 7)
            );
            assert!(matches!(result, Poll::Ready(Ok(_))) == (raw.retained_items() == 7));
            let call_layout = (
                core::mem::size_of_val(&call),
                core::mem::align_of_val(&call),
            );
            drop(call);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
            assert_released(trace.requests);
            println!(
                "alternative={}: accepted raw={raw:?}, requests={}, post_acceptance={}, layouts={:?}, installed_call={call_layout:?}, installed_return={return_layout:?}",
                alternative(),
                trace.requests,
                trace.after_acceptance,
                trace.layouts
            );
        }
    }
    #[test]
    fn backing_ceiling_boundaries_and_raw_rejection_keep_ownership() {
        let raw = BindingLifetimeFootprint::new(7, 2048);
        let (registration, _) = owner(BindingLifetimeFootprint::new(8, 4096), raw);
        let artifact = host_artifact(72);
        let input = request(72);
        reset_backing();
        observe(None);
        let mut call = registration
            .start_consumer_property_read(input, &artifact)
            .unwrap();
        let trace = disarm();
        let overhead = if alternative() == "inline" {
            core::mem::size_of_val(&call)
        } else if alternative() == "preallocated" {
            trace.layouts[0].0
        } else {
            trace.layouts[1].0
        } as u64;
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(
            call.as_pin_mut()
                .poll_result(&mut cx, &mut binding_budget(1)),
            Poll::Ready(Ok(_))
        ));
        drop(call);
        assert_released(trace.requests);
        for bytes in [2048 + overhead - 1, 2048 + overhead, 2048 + overhead + 1] {
            let (registration, stats) = owner(BindingLifetimeFootprint::new(8, bytes), raw);
            let mut call = registration
                .start_consumer_property_read(request(72), &artifact)
                .unwrap();
            let result = call
                .as_pin_mut()
                .poll_result(&mut cx, &mut binding_budget(1));
            let accepted = bytes >= 2048 + overhead;
            assert_eq!(matches!(result, Poll::Ready(Ok(_))), accepted);
            assert_eq!(stats.polls.load(Ordering::SeqCst), usize::from(accepted));
            drop(call);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
        }
        // The first owner is exhausted. Core must release its vacant envelope
        // and return the exact second input when the raw client rejects it.
        let input = request(72);
        let address = input.uri_variables().get("room").unwrap().as_ptr();
        reset_backing();
        observe(None);
        let rejection = registration
            .start_consumer_property_read(input, &artifact)
            .unwrap_err();
        let trace = disarm();
        let expected = usize::from(alternative() == "preallocated");
        assert_eq!(trace.requests, expected);
        assert_eq!(trace.after_acceptance, 0);
        assert_released(expected);
        let returned = rejection.into_input();
        assert_eq!(returned, request(72));
        assert_eq!(
            returned.uri_variables().get("room").unwrap().as_ptr(),
            address
        );
    }
    #[test]
    fn construction_failure_child() {
        let Ok(fail) = std::env::var("CLINKZ_CALL_FAIL") else {
            return;
        };
        let overrun = std::env::var_os("CLINKZ_CALL_OVERRUN").is_some();
        let (owner, stats) = owner(
            if overrun {
                BindingLifetimeFootprint::new(1, 64)
            } else {
                BindingLifetimeFootprint::new(8, 4096)
            },
            BindingLifetimeFootprint::new(7, 2048),
        );
        let original = request(72);
        let expected = request(72);
        let variable_address = original.uri_variables().get("room").unwrap().as_ptr();
        let artifact = host_artifact(72);
        reset_backing();
        observe(Some(fail.parse().unwrap()));
        let result = owner.start_consumer_property_read(original, &artifact);
        let trace = disarm();
        if candidate() && !ACCEPTED.with(Cell::get) {
            let rejection = result.unwrap_err();
            assert!(matches!(
                rejection.error().error(),
                CoreError::Backpressure(_)
            ));
            let returned = rejection.into_input();
            assert_eq!(returned, expected);
            assert_eq!(
                returned.uri_variables().get("room").unwrap().as_ptr(),
                variable_address
            );
            assert_eq!(stats.footprints.load(Ordering::SeqCst), 0);
            assert_eq!(stats.polls.load(Ordering::SeqCst), 0);
            assert_eq!(
                stats.drops.load(Ordering::SeqCst),
                usize::from(alternative() != "preallocated" || fail != "0")
            );
            assert_eq!(
                stats.invokes.load(Ordering::SeqCst),
                usize::from(alternative() != "preallocated" || fail != "0")
            );
            assert_released(trace.requests - 1);
            eprintln!("RECOVERED_EXACT_REQUEST");
        } else {
            assert!(candidate() && result.is_ok());
            assert_eq!(
                trace.requests, 1,
                "no attempted allocation at old Core wrapper site"
            );
            assert_eq!(stats.drops.load(Ordering::SeqCst), 0);
            drop(result);
            assert_eq!(stats.drops.load(Ordering::SeqCst), 1);
            assert_released(1);
            eprintln!("NO_POST_ACCEPTANCE_ALLOCATION");
        }
    }
    #[test]
    fn fail_each_baseline_site_in_both_branches() {
        for overrun in [false, true] {
            for fail in [0, 1] {
                let mut command = std::process::Command::new(std::env::current_exe().unwrap());
                command
                    .args([
                        "--exact",
                        "call_backing_candidate::construction_failure_child",
                        "--nocapture",
                    ])
                    .env("CLINKZ_CALL_FAIL", fail.to_string());
                if overrun {
                    command.env("CLINKZ_CALL_OVERRUN", "1");
                } else {
                    command.env_remove("CLINKZ_CALL_OVERRUN");
                }
                let output = command.output().unwrap();
                let diagnostic = String::from_utf8_lossy(&output.stderr);
                if candidate() {
                    assert!(output.status.success(), "{diagnostic}");
                    assert!(
                        diagnostic.contains(if fail == 0 || alternative() == "preallocated" {
                            "RECOVERED_EXACT_REQUEST"
                        } else {
                            "NO_POST_ACCEPTANCE_ALLOCATION"
                        }),
                        "{diagnostic}"
                    );
                } else {
                    assert!(!output.status.success());
                    assert!(
                        diagnostic.contains(&format!("FAIL_AFTER_ACCEPTANCE={}", fail == 1)),
                        "{diagnostic}"
                    );
                    assert!(diagnostic.contains("memory allocation of"), "{diagnostic}");
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        assert_eq!(output.status.signal(), Some(6));
                    }
                }
            }
        }
    }
}
