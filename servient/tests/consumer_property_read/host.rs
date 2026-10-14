use super::*;
use std::sync::Mutex;

fn build(runtime: &ConsumerPropertyReadRuntime) -> ConsumerPropertyReadHandle {
    let source = source();
    match runtime
        .consume(&source, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap()
    {
        StartStatus::Ready(h) => h,
        _ => panic!("publication pending"),
    }
}
fn shutdown(runtime: &ConsumerPropertyReadRuntime) {
    for _ in 0..10000 {
        if matches!(
            runtime.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            return;
        }
    }
    panic!("shutdown stalled");
}

fn rejected_host_startup(kind: ResourceKind, limit: u64) -> u64 {
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let identity = registration.registration().identity();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(kind, Some(limit)));
    let policy = policy_with(limits);
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock: Arc<dyn RuntimeClock + Send + Sync> = Arc::new(Clock::default());
    observe(0);
    let rejection = match ConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        clock.clone(),
        parent.clone(),
    ) {
        Err(rejection) => rejection,
        Ok(_) => panic!("unsupported startup was admitted"),
    };
    assert_eq!(trace().requests, 0);
    disarm();
    let (inputs, error) = rejection.into_parts();
    let requested = match error.error() {
        CoreError::LimitExceeded {
            resource,
            requested: Some(n),
            ..
        } => {
            assert_eq!(*resource, kind);
            *n
        }
        error => panic!("wrong startup rejection: {error:?}"),
    };
    assert_eq!(inputs.0.registration().identity(), identity);
    assert!(Arc::ptr_eq(&inputs.2, &clock));
    assert!(Arc::ptr_eq(&inputs.3, &parent));
    assert!(
        parent
            .lock()
            .unwrap()
            .iter()
            .all(|a| a.used() == 0 && a.peak() == 0)
    );
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    drop(inputs);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    requested
}

#[test]
fn host_binding_storage_all_scopes_disable_and_boundary_before_invocation() {
    let required = rejected_host_startup(ResourceKind::HostBindingCallBytesPerItemMax, 0);
    for kind in [
        ResourceKind::HostBindingCallBytesPerItemMax,
        ResourceKind::HostBindingCallBytesPerBindingMax,
        ResourceKind::HostBindingCallBytesPerThingMax,
        ResourceKind::HostBindingCallBytesGlobalMax,
    ] {
        for limit in [0, required - 1] {
            assert_eq!(rejected_host_startup(kind, limit), required);
        }
        for limit in [required, required + 1] {
            let stats = Arc::new(Stats::default());
            let registration =
                binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(kind, Some(limit)));
            // A disabled static representation must not disable Host.
            assert!(limits.set(ResourceKind::BindingSlotStateBytesGlobalMax, Some(0)));
            let policy = policy_with(limits);
            let parent = Arc::new(Mutex::new(policy.parent_accounts()));
            let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                Arc::new(Clock::default()),
                parent.clone(),
            )
            .unwrap_or_else(|_| panic!("exact Host storage bound rejected"));
            let handle = build(&runtime);
            let mut call = handle
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            call.step(&mut cx(), &mut work(100000));
            assert_eq!(
                category(call.try_take(&mut work(100000)).unwrap().unwrap()),
                "ok"
            );
            assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
            drop(call);
            drop(handle);
            shutdown(&runtime);
            drop(runtime);
            drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
    }
    println!("Host complete call storage allowance={required}");
}

#[test]
fn host_callback_and_destination_capacity_disable_and_exact_boundaries() {
    for kind in [
        ResourceKind::BindingPollTemporaryBytesPerCallMax,
        ResourceKind::BindingPollTemporaryBytesGlobalMax,
        ResourceKind::BindingCancelBufferBytesPerCallMax,
        ResourceKind::BindingCancelBufferBytesGlobalMax,
        ResourceKind::CleanupTransferSlotsGlobalMax,
        ResourceKind::CleanupTransferBytesGlobalMax,
    ] {
        let required = rejected_host_startup(kind, 0);
        assert_eq!(rejected_host_startup(kind, required - 1), required);
        for limit in [required, required + 1] {
            for cancel in [false, true] {
                let stats = Arc::new(Stats::default());
                if matches!(
                    kind,
                    ResourceKind::CleanupTransferSlotsGlobalMax
                        | ResourceKind::CleanupTransferBytesGlobalMax
                ) {
                    stats.set_mode(Mode::Transfer);
                }
                let mut limits = GatewayDefaultV1::LIMITS.clone();
                assert!(limits.set(kind, Some(limit)));
                let policy = policy_with(limits);
                let parent = Arc::new(Mutex::new(policy.parent_accounts()));
                let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
                    binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
                    policy,
                    Arc::new(Clock::default()),
                    parent.clone(),
                )
                .unwrap_or_else(|_| panic!("exact callback/destination bound rejected"));
                let handle = build(&runtime);
                let mut call = handle
                    .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                    .unwrap();
                if cancel {
                    call.cancel().unwrap();
                }
                runtime.step(&mut cx(), &mut work(100000));
                runtime.step(&mut cx(), &mut work(100000));
                assert_eq!(
                    category(call.try_take(&mut work(100000)).unwrap().unwrap()),
                    "ok"
                );
                assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
                if cancel {
                    assert!(stats.cancels.load(Ordering::SeqCst) > 0);
                }
                drop(call);
                drop(handle);
                shutdown(&runtime);
                drop(runtime);
                drop(observer);
                assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
            }
        }
        println!("Host {kind:?} complete allowance={required}");
    }
}

#[test]
fn host_callback_scratch_shared_parent_covers_every_callback_until_retirement() {
    let kind = ResourceKind::BindingPollTemporaryBytesGlobalMax;
    let required = rejected_host_startup(kind, 0);
    let policy_a = policy();
    let mut accounts = policy_a.parent_accounts();
    let index = ConsumerPropertyReadPolicy::PARENT_KINDS
        .iter()
        .position(|&k| k == kind)
        .unwrap();
    accounts[index] = ResourceAccount::new(SlotIndex::new(92), Generation::INITIAL, kind, required);
    let parent = Arc::new(Mutex::new(accounts));
    let stats_a = Arc::new(Stats::default());
    let (runtime_a, observer_a) = ConsumerPropertyReadRuntime::try_new(
        binding::host_registration(binding::HostClient::new(stats_a.clone(), outputs())),
        policy_a,
        Arc::new(Clock::default()),
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!());
    assert_eq!(parent.lock().unwrap()[index].used(), required);
    let stats_b = Arc::new(Stats::default());
    let registration_b =
        binding::host_registration(binding::HostClient::new(stats_b.clone(), outputs()));
    let clock_b: Arc<dyn RuntimeClock + Send + Sync> = Arc::new(Clock::default());
    let policy_b = policy();
    observe(0);
    let rejection = match ConsumerPropertyReadRuntime::try_new(
        registration_b,
        policy_b,
        clock_b,
        parent.clone(),
    ) {
        Err(rejection) => rejection,
        Ok(_) => panic!("independent global callback scratch bypassed"),
    };
    assert_eq!(trace().requests, 0);
    disarm();
    assert!(matches!(
        rejection.error().error(),
        CoreError::Backpressure(_)
    ));
    let (registration_b, policy_b, clock_b, parent_b) = rejection.into_input();
    assert_eq!(stats_b.starts.load(Ordering::SeqCst), 0);
    let callbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = callbacks.clone();
    let scratch_parent = parent.clone();
    *stats_a.hook.lock().unwrap() = Some(Arc::new(move || {
        // This runs inside invoke, result poll, cancel and cancellation poll.
        // The competing root has no capacity to give back to this owner.
        assert_eq!(scratch_parent.lock().unwrap()[index].used(), required);
        seen.fetch_add(1, Ordering::SeqCst);
    }));
    let handle_a = build(&runtime_a);
    for cancel in [true, false] {
        let mut call = handle_a
            .read_property("a-read", InteractionOptions::new(), &mut work(100000))
            .unwrap();
        if cancel {
            call.cancel().unwrap();
        }
        runtime_a.step(&mut cx(), &mut work(100000));
        runtime_a.step(&mut cx(), &mut work(100000));
        assert_eq!(parent.lock().unwrap()[index].used(), required);
        assert_eq!(
            category(call.try_take(&mut work(100000)).unwrap().unwrap()),
            "ok"
        );
        drop(call);
        assert_eq!(parent.lock().unwrap()[index].used(), required);
    }
    assert_eq!(callbacks.load(Ordering::SeqCst), 5);
    *stats_a.hook.lock().unwrap() = None;
    drop(handle_a);
    shutdown(&runtime_a);
    assert_eq!(stats_a.clients_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(parent.lock().unwrap()[index].used(), 0);
    // Retiring the registration releases scratch even with the facade alive.
    let (runtime_b, observer_b) =
        ConsumerPropertyReadRuntime::try_new(registration_b, policy_b, clock_b, parent_b)
            .unwrap_or_else(|_| panic!("retired scratch capacity unavailable"));
    let handle_b = build(&runtime_b);
    let mut call_b = handle_b
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    call_b.step(&mut cx(), &mut work(100000));
    assert_eq!(
        category(call_b.try_take(&mut work(100000)).unwrap().unwrap()),
        "ok"
    );
    drop(call_b);
    drop(handle_b);
    shutdown(&runtime_b);
    drop(runtime_b);
    drop(observer_b);
    drop(runtime_a);
    drop(observer_a);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_binding_storage_shared_parent_is_held_through_transfer_and_unread_result() {
    for kind in [
        ResourceKind::HostBindingCallBytesGlobalMax,
        ResourceKind::CleanupTransferSlotsGlobalMax,
        ResourceKind::CleanupTransferBytesGlobalMax,
        ResourceKind::BindingCancelBufferBytesGlobalMax,
    ] {
        let required = rejected_host_startup(kind, 0);
        for accept in [true, false] {
            let policy_a = policy();
            let policy_b = policy();
            let mut accounts = policy_a.parent_accounts();
            let index = ConsumerPropertyReadPolicy::PARENT_KINDS
                .iter()
                .position(|&k| k == kind)
                .unwrap();
            accounts[index] =
                ResourceAccount::new(SlotIndex::new(90), Generation::INITIAL, kind, required);
            let parent = Arc::new(Mutex::new(accounts));
            let stats_a = Arc::new(Stats::default());
            stats_a.set_mode(Mode::Transfer);
            let stats_b = Arc::new(Stats::default());
            let (runtime_a, report_a) = ConsumerPropertyReadRuntime::try_new(
                binding::host_registration(binding::HostClient::new(stats_a.clone(), outputs())),
                policy_a,
                Arc::new(Clock::default()),
                parent.clone(),
            )
            .unwrap_or_else(|_| panic!());
            let (runtime_b, report_b) = ConsumerPropertyReadRuntime::try_new(
                binding::host_registration(binding::HostClient::new(stats_b.clone(), outputs())),
                policy_b,
                Arc::new(Clock::default()),
                parent.clone(),
            )
            .unwrap_or_else(|_| panic!());
            let handle_a = build(&runtime_a);
            let handle_b = build(&runtime_b);
            let mut call = handle_a
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            call.cancel().unwrap();
            for phase in 0..3 {
                assert_eq!(parent.lock().unwrap()[index].used(), required);
                let mut options = InteractionOptions::new();
                options.uri_variables.insert("room".into(), "west".into());
                let address = options.uri_variables["room"].as_ptr();
                observe(0);
                let rejection = handle_b
                    .read_property("a-read", options, &mut work(100000))
                    .err()
                    .expect("shared Host byte capacity must reject before invocation");
                assert_eq!(trace().requests, 0);
                disarm();
                assert!(matches!(
                    rejection.error().error(),
                    CoreError::Backpressure(_)
                ));
                let (options, request) = rejection.into_input();
                assert!(request.is_none());
                assert_eq!(options.uri_variables["room"].as_ptr(), address);
                assert_eq!(stats_b.starts.load(Ordering::SeqCst), 0);
                if phase != 2 {
                    runtime_a.step_with_transfer_offer(
                        &mut cx(),
                        &mut work(100000),
                        &mut |request| {
                            assert!(request.phase().transfer_owner().is_some());
                            assert_eq!(parent.lock().unwrap()[index].used(), required);
                            accept
                        },
                    );
                }
            }
            assert_eq!(report_a().0.is_some_and(|r| r.is_pending()), accept);
            assert_eq!(
                category(call.try_take(&mut work(100000)).unwrap().unwrap()),
                "ok"
            );
            assert_eq!(parent.lock().unwrap()[index].used(), 0);
            let mut next = handle_b
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            next.step(&mut cx(), &mut work(100000));
            assert_eq!(
                category(next.try_take(&mut work(100000)).unwrap().unwrap()),
                "ok"
            );
            drop(next);
            drop(call);
            drop(handle_a);
            drop(handle_b);
            shutdown(&runtime_a);
            shutdown(&runtime_b);
            drop(runtime_a);
            drop(runtime_b);
            drop(report_a);
            drop(report_b);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
    }
}

#[test]
fn host_report_complete_footprint_is_checked_before_provisioning() {
    let required = rejected_host_startup(ResourceKind::CleanupItemBytesMax, 1);
    for limit in [0, required - 1] {
        assert_eq!(
            rejected_host_startup(ResourceKind::CleanupItemBytesMax, limit),
            required
        );
    }
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::CleanupItemBytesMax, Some(required)));
    let policy = policy_with(limits);
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let stats = Arc::new(Stats::default());
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        binding::host_registration(binding::HostClient::new(stats, outputs())),
        policy,
        Arc::new(Clock::default()),
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!("exact report bound rejected"));
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    println!("Host complete cleanup report footprint={required}");
}

#[test]
fn host_cleanup_quantum_caps_the_whole_shutdown_and_retains_pending_work() {
    for too_small in [1, 3] {
        assert_eq!(
            rejected_host_startup(ResourceKind::CleanupWorkItemsPerStepMax, too_small),
            4
        );
    }
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Complete);
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::CleanupWorkItemsPerStepMax, Some(4)));
    let policy = policy_with_cleanup_costs(limits);
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
        policy,
        Arc::new(Clock::default()),
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!());
    let handle = build(&runtime);
    let call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    drop(call);
    drop(handle);
    let mut first = work(100000);
    runtime.step(&mut cx(), &mut first);
    assert_eq!(100000 - first.remaining(WorkClass::CleanupItems), 1);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 0);
    let mut finished = false;
    let mut pending = 0;
    for _ in 0..10000 {
        let mut budget = work(100000);
        let status = runtime.shutdown(&mut cx(), &mut budget);
        assert!(100000 - budget.remaining(WorkClass::CleanupItems) <= 4);
        if let StartStatus::Ready(outcome) = status {
            assert!(outcome.is_complete());
            finished = true;
            break;
        }
        pending += 1;
    }
    assert!(finished && pending > 0);
    assert_eq!(stats.cancels.load(Ordering::SeqCst), 1);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    drop(runtime);
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_manual_cleanup_commits_terminal_report_before_delivery_or_discard_destruction() {
    let transfer_bytes = rejected_host_startup(ResourceKind::CleanupTransferBytesGlobalMax, 0);
    let cancel_bytes = rejected_host_startup(ResourceKind::BindingCancelBufferBytesGlobalMax, 0);
    for discard in [false, true] {
        for expire in [false, true] {
            let stats = Arc::new(Stats::default());
            stats.set_mode(Mode::Transfer);
            let policy = policy();
            let parent = Arc::new(Mutex::new(policy.parent_accounts()));
            let clock = Arc::new(Clock::default());
            let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
                binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
                policy,
                clock.clone(),
                parent.clone(),
            )
            .unwrap_or_else(|_| panic!());
            let observer = Arc::new(observer);
            let handle = build(&runtime);
            let check = observer.clone();
            let ordered = Arc::new(core::sync::atomic::AtomicBool::new(true));
            let observed = ordered.clone();
            let capacity = parent.clone();
            let check: Arc<dyn Fn()> = Arc::new(move || {
                // Persist the observation without unwinding from GlobalAlloc.
                let terminal = check().0.is_some_and(|outcome| {
                    outcome.is_residual() == expire && outcome.is_complete() != expire
                });
                let bank = capacity.lock().unwrap_or_else(|e| e.into_inner());
                let backed = [
                    (ResourceKind::CleanupTransferSlotsGlobalMax, 1),
                    (ResourceKind::CleanupTransferBytesGlobalMax, transfer_bytes),
                    (
                        ResourceKind::BindingCancelBufferBytesGlobalMax,
                        cancel_bytes,
                    ),
                    (ResourceKind::BindingPollTemporaryBytesGlobalMax, 128),
                ]
                .into_iter()
                .all(|(kind, bytes)| {
                    bank.iter()
                        .find(|a| a.kind() == kind)
                        .is_some_and(|a| a.used() == bytes)
                });
                observed.fetch_and(terminal && backed, Ordering::SeqCst);
            });
            observe(usize::MAX);
            let mut call = handle
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            call.cancel().unwrap();
            runtime.step(&mut cx(), &mut work(100000));
            assert!(observer().0.unwrap().is_pending());
            if expire {
                clock.0.store(1000, Ordering::SeqCst);
            }
            runtime.step(&mut cx(), &mut work(100000));
            assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 0);
            DROP_CHECK.with(|c| *c.borrow_mut() = Some(check));
            if discard {
                drop(call);
                runtime.step(&mut cx(), &mut work(100000));
            } else {
                let result = call.try_take(&mut work(100000)).unwrap().unwrap();
                assert_eq!(category(result), if expire { "cancelled" } else { "ok" });
                drop(call);
            }
            DROP_CHECK.with(|c| *c.borrow_mut() = None);
            assert!(
                ordered.load(Ordering::SeqCst),
                "terminal report and capacity must precede raw/Core deallocation"
            );
            assert!(
                parent
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|a| matches!(
                        a.kind(),
                        ResourceKind::CleanupTransferSlotsGlobalMax
                            | ResourceKind::CleanupTransferBytesGlobalMax
                            | ResourceKind::BindingCancelBufferBytesGlobalMax
                    ))
                    .all(|a| a.used() == 0)
            );
            assert_eq!(trace().live, 0);
            disarm();
            assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
            assert_eq!(runtime.cleanup_record().unwrap().is_complete(), !expire);
            assert_eq!(observer().0.unwrap().is_residual(), expire);
            drop(handle);
            shutdown(&runtime);
            drop(runtime);
            drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
    }
}

#[test]
fn host_every_runtime_build_and_call_allocation_failure_returns_complete_owners() {
    for fail in 0..2 {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        observe(fail);
        let rejected =
            match ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            {
                Err(e) => e,
                Ok(_) => panic!("allocation {fail} did not fail"),
            };
        disarm();
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
        let (registration, _policy, _clock, returned_parent) = rejected.into_input();
        assert!(Arc::ptr_eq(&parent, &returned_parent));
        assert_eq!(
            registration.registration().identity().binding_id(),
            BindingId::new(223)
        );
        drop(registration);
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(trace().live, 0);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
    let mut sites = 0;
    for fail in 0..256 {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) =
            ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
                .unwrap_or_else(|_| panic!());
        let source = source();
        observe(fail);
        let result = runtime.consume(&source, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        });
        let requests = trace().requests;
        disarm();
        if fail < requests {
            assert!(result.is_err(), "build allocation {fail}");
            sites += 1;
        }
        drop(result);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert_eq!(trace().live, 0, "build allocation {fail}");
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        if fail >= requests {
            break;
        }
    }
    println!("host recoverable runtime sites=2, build sites={sites}, call sites=2");
    assert!(sites > 20);
    for fail in 0..2 {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) =
            ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
                .unwrap_or_else(|_| panic!());
        let handle = build(&runtime);
        let mut options = InteractionOptions::new();
        options.uri_variables.insert("room".into(), "west".into());
        let address = options.uri_variables["room"].as_ptr();
        observe(fail);
        let rejected = match handle.read_property("a-read", options, &mut work(100000)) {
            Err(e) => e,
            Ok(_) => panic!(),
        };
        let (options, request) = rejected.into_input();
        assert!(options.uri_variables.is_empty());
        assert_eq!(
            request.as_ref().unwrap().uri_variables()["room"].as_ptr(),
            address
        );
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        assert_eq!(trace().live, 0);
        disarm();
        drop(request);
        drop(options);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn host_abandon_every_unpublished_step_keeps_original_graph_and_account() {
    let mut boundaries = 0;
    for cutoff in 0..20000 {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) =
            ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
                .unwrap_or_else(|_| panic!());
        let source = source();
        let mut finished = false;
        observe(usize::MAX);
        let _ = runtime.consume(&source, &mut work(0), |b| {
            for _ in 0..cutoff {
                b.step(&mut work(4096));
                if b.is_finished() {
                    finished = true;
                    break;
                }
            }
            if cutoff % 2 == 1 {
                b.cancel();
                b.step(&mut work(0));
            }
        });
        let before = trace();
        drop(runtime);
        let after = trace();
        disarm();
        assert_eq!(after.requests, before.requests);
        assert_eq!(after.live, 0, "step {cutoff}");
        assert!(observer().0.unwrap().is_residual());
        assert!(observer().1[0] <= observer().1[1]);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        if finished {
            boundaries = cutoff;
            break;
        }
    }
    assert!(boundaries > 30);
    println!("host unpublished step boundaries={boundaries}");
}

#[test]
fn host_transfer_acknowledges_whole_call_or_retains_identical_manual_owner() {
    for accept in [true, false] {
        let stats = Arc::new(Stats::default());
        stats.set_mode(Mode::Transfer);
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) =
            ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
                .unwrap_or_else(|_| panic!());
        let handle = build(&runtime);
        let mut call = handle
            .read_property("a-read", InteractionOptions::new(), &mut work(100000))
            .unwrap();
        call.cancel().unwrap();
        let mut offered = 0;
        runtime.step_with_transfer_offer(&mut cx(), &mut work(100000), &mut |request| {
            offered += 1;
            assert_eq!(
                request.phase().first_cause().context().cause_code(),
                Some(401)
            );
            accept
        });
        assert_eq!(offered, 1);
        assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime.cleanup_record().is_some_and(|r| r.is_pending()),
            accept
        );
        assert!(
            handle
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .is_err()
        );
        runtime.step(&mut cx(), &mut work(100000));
        assert_eq!(
            category(call.try_take(&mut work(100000)).unwrap().unwrap()),
            "ok"
        );
        assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn host_publication_zero_short_exact_and_both_real_cancel_orders() {
    for cancel_first in [true, false] {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) =
            ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
                .unwrap_or_else(|_| panic!());
        let source = source();
        runtime
            .consume(&source, &mut work(0), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!();
            })
            .unwrap();
        assert_eq!(runtime.state(), CompiledPlanSetState::Frozen);
        assert!(runtime.take_handle().is_none());
        observe(usize::MAX);
        for credit in [0, ConsumerPropertyReadPolicy::FINAL_INSTALL_WORK - 1] {
            let mut short = work(credit);
            runtime.step(&mut cx(), &mut short);
            assert_eq!(runtime.state(), CompiledPlanSetState::Frozen);
            assert_eq!(short.remaining(WorkClass::PlanningItems), credit);
        }
        assert_eq!(trace().requests, 0);
        assert_eq!(trace().releases, 0);
        disarm();
        let before = Arc::new(std::sync::Barrier::new(2));
        let after = Arc::new(std::sync::Barrier::new(2));
        std::thread::scope(|threads| {
            threads.spawn(|| {
                before.wait();
                runtime.close();
                after.wait();
            });
            if cancel_first {
                before.wait();
                after.wait();
            }
            observe(usize::MAX);
            runtime.step(
                &mut cx(),
                &mut work(ConsumerPropertyReadPolicy::FINAL_INSTALL_WORK),
            );
            let observed = runtime.state();
            let handle = runtime.take_handle();
            assert_eq!(trace().requests, 0, "final permit allocated");
            if !cancel_first {
                assert_eq!(trace().releases, 0, "final permit released backing");
            }
            assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
            disarm();
            assert_eq!(handle.is_some(), !cancel_first);
            if !cancel_first {
                assert_eq!(observed, CompiledPlanSetState::Published);
                before.wait();
                after.wait();
            } else {
                assert_ne!(observed, CompiledPlanSetState::Published);
            }
            drop(handle);
        });
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn host_quiescent_driver_moves_threads_and_generations_are_not_registration_generations() {
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (mut runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .unwrap_or_else(|_| panic!());
    let mut previous = runtime.generations();
    for _ in 0..2 {
        let handle = build(&runtime);
        let now = runtime.generations();
        assert_ne!((previous.0, previous.1), (now.0, now.1));
        previous = now;
        let inside = stats.clone();
        let (returned, handle) = std::thread::spawn(move || {
            for (name, form, target) in [
                ("a-read", 2, "https://peer.test/a/second"),
                ("c-温度", 1, "https://peer.test/a/last"),
            ] {
                let mut call = handle
                    .read_property(
                        name,
                        InteractionOptions::new().with_form_index(form),
                        &mut work(100000),
                    )
                    .unwrap();
                call.step(&mut cx(), &mut work(100000));
                let output = call.try_take(&mut work(100000)).unwrap().unwrap().unwrap();
                assert_eq!(
                    output
                        .metadata()
                        .binding_response()
                        .unwrap()
                        .binding_generation(),
                    BindingGeneration::INITIAL.checked_next().unwrap()
                );
                let n = inside.target_len.load(Ordering::SeqCst);
                let actual: Vec<_> = inside.target[..n]
                    .iter()
                    .map(|v| v.load(Ordering::SeqCst))
                    .collect();
                assert_eq!(actual, target.as_bytes());
                drop(call);
            }
            (runtime, handle)
        })
        .join()
        .unwrap();
        runtime = returned;
        drop(handle);
        for _ in 0..10000 {
            runtime.step(&mut cx(), &mut work(4096));
            if runtime.state() == CompiledPlanSetState::Reclaimed {
                break;
            }
        }
    }
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    assert_eq!(stats.starts.load(Ordering::SeqCst), 4);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_callbacks_reenter_without_structural_lock_and_drop_interest_during_checkout() {
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock.clone(), parent.clone())
            .unwrap_or_else(|_| panic!());
    let runtime = Arc::new(runtime);
    let handle = build(&runtime);
    let weak = Arc::downgrade(&runtime);
    *stats.hook.lock().unwrap() = Some(Arc::new(move || {
        assert_eq!(
            weak.upgrade().unwrap().state(),
            CompiledPlanSetState::Published
        )
    }));
    let call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    let entered = Arc::new(std::sync::Barrier::new(2));
    let leave = Arc::new(std::sync::Barrier::new(2));
    let e = entered.clone();
    let l = leave.clone();
    *stats.hook.lock().unwrap() = Some(Arc::new(move || {
        e.wait();
        l.wait();
    }));
    let held = parent.lock().unwrap()[0].used();
    std::thread::scope(|threads| {
        threads.spawn(|| {
            runtime.step(&mut cx(), &mut work(100000));
        });
        entered.wait();
        drop(call);
        drop(handle);
        assert_eq!(runtime.state(), CompiledPlanSetState::Draining);
        assert_eq!(
            parent.lock().unwrap()[0].used(),
            held,
            "busy owner released capacity"
        );
        assert!(runtime.cleanup_record().is_none());
        leave.wait();
    });
    *stats.hook.lock().unwrap() = None;
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_root_and_handle_drop_while_exclusive_driver_is_outside_lock() {
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .unwrap_or_else(|_| panic!());
    let handle = build(&runtime);
    let call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    let entered = Arc::new(std::sync::Barrier::new(2));
    let leave = Arc::new(std::sync::Barrier::new(2));
    let e = entered.clone();
    let l = leave.clone();
    *stats.hook.lock().unwrap() = Some(Arc::new(move || {
        e.wait();
        l.wait();
    }));
    std::thread::scope(|threads| {
        threads.spawn(move || {
            call.step(&mut cx(), &mut work(100000));
            drop(call);
        });
        entered.wait();
        drop(handle);
        drop(runtime);
        assert!(
            observer().0.is_none(),
            "driver still retains a strong root and complete owner"
        );
        leave.wait();
    });
    *stats.hook.lock().unwrap() = None;
    assert!(observer().0.unwrap().is_residual());
    drop(observer);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_unwind_reparks_the_original_owner_and_explicit_shutdown_can_finish() {
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .unwrap_or_else(|_| panic!());
    let handle = build(&runtime);
    *stats.hook.lock().unwrap() = Some(Arc::new(|| panic!("driver adapter unwind")));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = handle.read_property("a-read", InteractionOptions::new(), &mut work(100000));
        }))
        .is_err()
    );
    *stats.hook.lock().unwrap() = None;
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.state(), CompiledPlanSetState::Draining);
    drop(handle);
    shutdown(&runtime);
    drop(runtime);
    assert!(observer().0.unwrap().is_residual());
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_two_roots_exhaust_real_parent_at_build_output_and_call_peaks() {
    for resource in [
        ResourceKind::AdmissionTemporaryBytesGlobalMax,
        ResourceKind::CompiledRuntimeBytesGlobalMax,
        ResourceKind::PlanPinsGlobalMax,
    ] {
        let policy_a = policy();
        let policy_b = policy();
        let mut accounts = policy_a.parent_accounts();
        let index = ConsumerPropertyReadPolicy::PARENT_KINDS
            .iter()
            .position(|&k| k == resource)
            .unwrap();
        let cap = match resource {
            ResourceKind::AdmissionTemporaryBytesGlobalMax => 69000,
            ResourceKind::CompiledRuntimeBytesGlobalMax => 6000,
            _ => 1,
        };
        accounts[index] =
            ResourceAccount::new(SlotIndex::new(0), Generation::INITIAL, resource, cap);
        let parent = Arc::new(Mutex::new(accounts));
        let clock = Arc::new(Clock::default());
        let stats_a = Arc::new(Stats::default());
        let stats_b = Arc::new(Stats::default());
        let (a, observer_a) = ConsumerPropertyReadRuntime::try_new(
            binding::host_registration(binding::HostClient::new(stats_a, outputs())),
            policy_a,
            clock.clone(),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!());
        let (b, observer_b) = ConsumerPropertyReadRuntime::try_new(
            binding::host_registration(binding::HostClient::new(stats_b.clone(), outputs())),
            policy_b,
            clock,
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!());
        let thing = source();
        let handle_a = match a
            .consume(&thing, &mut work(100000), |build| {
                if resource == ResourceKind::AdmissionTemporaryBytesGlobalMax {
                    assert!(matches!(
                        b.consume(&thing, &mut work(100000), |_| panic!(
                            "no capacity may reach TD"
                        )),
                        Err(CoreError::Backpressure(_))
                    ));
                }
                for _ in 0..100000 {
                    build.step(&mut work(4096));
                    if build.is_finished() {
                        return;
                    }
                }
                panic!();
            })
            .unwrap()
        {
            StartStatus::Ready(h) => h,
            _ => panic!(),
        };
        if resource != ResourceKind::AdmissionTemporaryBytesGlobalMax {
            let result = b.consume(&thing, &mut work(100000), |build| {
                for _ in 0..100000 {
                    build.step(&mut work(4096));
                    if build.is_finished() {
                        return;
                    }
                }
                panic!();
            });
            if resource == ResourceKind::CompiledRuntimeBytesGlobalMax {
                assert!(result.is_err());
            } else {
                let handle_b = match result.unwrap() {
                    StartStatus::Ready(h) => h,
                    _ => panic!(),
                };
                let call = handle_a
                    .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                    .unwrap();
                let rejection = match handle_b.read_property(
                    "a-read",
                    InteractionOptions::new(),
                    &mut work(100000),
                ) {
                    Err(e) => e,
                    Ok(_) => panic!(),
                };
                assert!(matches!(
                    rejection.error().error(),
                    CoreError::Backpressure(_)
                ));
                drop(call);
                drop(handle_b);
            }
        }
        assert_eq!(stats_b.starts.load(Ordering::SeqCst), 0);
        drop(handle_a);
        drop(a);
        drop(b);
        drop(observer_a);
        drop(observer_b);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn host_late_validation_deadline_overrun_and_preacceptance_recovery() {
    for mode in [
        Mode::Normal,
        Mode::LateStart,
        Mode::LatePoll,
        Mode::Transfer,
    ] {
        for validation in 1..=3 {
            let stats = Arc::new(Stats::default());
            stats.set_mode(mode);
            stats.validation.store(validation, Ordering::SeqCst);
            let policy = policy();
            let parent = Arc::new(Mutex::new(policy.parent_accounts()));
            let clock = Arc::new(Clock::default());
            let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
                binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
                policy,
                clock,
                parent.clone(),
            )
            .unwrap_or_else(|_| panic!());
            let handle = build(&runtime);
            let mut call = handle
                .read_property(
                    "c-温度",
                    InteractionOptions::new().with_form_index(1),
                    &mut work(100000),
                )
                .unwrap();
            if mode != Mode::Normal {
                call.cancel().unwrap();
            }
            for _ in 0..3 {
                runtime.step(&mut cx(), &mut work(100000));
            }
            assert_eq!(
                category(call.try_take(&mut work(100000)).unwrap().unwrap()),
                if validation == 3 {
                    "binding"
                } else {
                    "validation"
                }
            );
            assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
            drop(call);
            drop(handle);
            shutdown(&runtime);
            drop(runtime);
            drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
    }
    for mode in [Mode::Pending, Mode::Overrun, Mode::Reject] {
        let stats = Arc::new(Stats::default());
        stats.set_mode(mode);
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
            policy,
            clock.clone(),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!());
        let handle = build(&runtime);
        let mut options = InteractionOptions::new();
        options.uri_variables.insert("room".into(), "west".into());
        let address = options.uri_variables["room"].as_ptr();
        let result = handle.read_property("a-read", options, &mut work(100000));
        if mode == Mode::Reject {
            let (options, request) = match result {
                Err(e) => e.into_input(),
                Ok(_) => panic!(),
            };
            assert!(options.uri_variables.is_empty());
            assert_eq!(request.unwrap().uri_variables()["room"].as_ptr(), address);
            assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        } else {
            let mut call = result.unwrap();
            if mode == Mode::Pending {
                assert_eq!(
                    runtime
                        .next_deadline(&mut work(100000))
                        .unwrap()
                        .unwrap()
                        .instant()
                        .unwrap()
                        .ticks(),
                    60
                );
                clock.0.store(61, Ordering::SeqCst);
                runtime.step(&mut cx(), &mut work(100000));
                assert_eq!(
                    runtime
                        .next_deadline(&mut work(100000))
                        .unwrap()
                        .unwrap()
                        .instant()
                        .unwrap()
                        .ticks(),
                    60
                );
                clock.0.store(67, Ordering::SeqCst);
                runtime.step(&mut cx(), &mut work(100000));
                assert_eq!(
                    category(call.try_take(&mut work(100000)).unwrap().unwrap()),
                    "deadline"
                );
            } else {
                runtime.step(&mut cx(), &mut work(100000));
                let error = call
                    .try_take(&mut work(100000))
                    .unwrap()
                    .unwrap()
                    .unwrap_err();
                assert!(matches!(error, CoreError::Validation(_))); // Core accepted-error mediator.
                assert_eq!(stats.polls.load(Ordering::SeqCst), 0);
            }
            assert!(call.cancel().is_err());
            drop(call);
        }
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn host_missing_id_and_later_bounds_cannot_start_any_compiler_or_binding() {
    for missing_id in [true, false] {
        let mut thing = source();
        if missing_id {
            thing.id = None;
        } else {
            thing
                .properties
                .as_mut()
                .unwrap()
                .get_mut("c-温度")
                .unwrap()
                ._interaction
                .forms[1]
                .href = clinkz_wot_td::data_type::FormHref::parse(&format!(
                "https://peer.test/{}",
                "x".repeat(80)
            ))
            .unwrap();
        }
        let stats = Arc::new(Stats::default());
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let artifact_layout = registration.support().host_slot_layout().unwrap();
        let (runtime, observer) =
            ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
                .unwrap_or_else(|_| panic!());
        observe(usize::MAX);
        let result = runtime.consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        });
        let observed = trace();
        disarm();
        assert!(result.is_err());
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        assert!(
            !observed.layouts[..observed.requests]
                .contains(&(artifact_layout.size(), artifact_layout.align())),
            "all bounds precede first compiler slot acquisition"
        );
        if missing_id {
            assert_eq!(parent.lock().unwrap()[3].peak(), 0);
        }
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert_eq!(trace().live, 0);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn host_residual_precedes_every_controlled_local_deallocation() {
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Pending);
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    observe(usize::MAX);
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .unwrap_or_else(|_| panic!());
    disarm();
    let thing = source();
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = true;
        t.set(s);
    });
    let handle = match runtime
        .consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap()
    {
        StartStatus::Ready(h) => h,
        _ => panic!(),
    };
    let call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    disarm();
    let observer = Arc::new(observer);
    let check = observer.clone();
    let checked = Arc::new(core::sync::atomic::AtomicUsize::new(0));
    let count = checked.clone();
    DROP_CHECK.with(|c| {
        *c.borrow_mut() = Some(Arc::new(move || {
            assert!(
                check().0.unwrap().is_residual(),
                "local state released before residual"
            );
            count.fetch_add(1, Ordering::SeqCst);
        }))
    });
    drop(call);
    drop(handle);
    drop(runtime);
    DROP_CHECK.with(|c| drop(c.borrow_mut().take()));
    assert!(checked.load(Ordering::SeqCst) > 10);
    assert!(observer().0.unwrap().is_residual());
    drop(observer);
    assert_eq!(trace().live, 0);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_whole_provisioning_request_call_and_global_live_measurements() {
    let thing = source();
    observe(usize::MAX);
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let binding_provision = trace();
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock.clone(), parent.clone())
            .unwrap_or_else(|_| panic!());
    let root_provision = trace();
    let handle = match runtime
        .consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap()
    {
        StartStatus::Ready(h) => h,
        _ => panic!(),
    };
    let built = trace();
    let mut options = InteractionOptions::new();
    options.uri_variables.insert("room".into(), "west".into());
    let request = trace();
    let mut call = handle
        .read_property("a-read", options, &mut work(100000))
        .unwrap();
    let accepted = trace();
    call.step(&mut cx(), &mut work(100000));
    assert_eq!(trace().requests, accepted.requests);
    assert_eq!(
        category(call.try_take(&mut work(100000)).unwrap().unwrap()),
        "ok"
    );
    drop(call);
    drop(handle);
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    drop(stats);
    drop(clock);
    drop(parent);
    let final_trace = trace();
    disarm();
    assert_eq!(final_trace.live, 0);
    println!(
        "whole fixture: binding provisioning live={}, allocations={}; all provisioning live={}, allocations={}; build live={}, peak={}, largest={}; request allocations={:?}; call allocations={:?}; global peak={}, final retained={}, releases={}",
        binding_provision.live,
        binding_provision.requests,
        root_provision.live,
        root_provision.requests,
        built.live,
        built.peak,
        built.largest,
        &request.layouts[built.requests..request.requests],
        &accepted.layouts[request.requests..accepted.requests],
        final_trace.peak,
        final_trace.live,
        final_trace.releases
    );
    assert!(accepted.requests - request.requests <= 2);
    assert!(accepted.live - request.live <= 1024);
}

#[test]
fn host_common_case_table_and_result_saturation() {
    println!("Host common case table version={CASE_TABLE_VERSION}");
    for (mode, cancel, advance, expected) in CASES {
        let stats = Arc::new(Stats::default());
        stats.set_mode(mode);
        let registration =
            binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let clock = Arc::new(Clock::default());
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            clock.clone(),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!("host construct"));
        let source = source();
        let handle = match runtime
            .consume(&source, &mut work(100000), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!("stalled");
            })
            .unwrap()
        {
            StartStatus::Ready(h) => h,
            _ => panic!(),
        };
        drop(source);
        let mut call = handle
            .read_property(
                "a-read",
                InteractionOptions::new().with_form_index(2),
                &mut work(100000),
            )
            .unwrap();
        assert!(
            handle
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .is_err()
        );
        let before = stats.polls.load(Ordering::SeqCst);
        call.step(&mut cx(), &mut work(0));
        assert_eq!(stats.polls.load(Ordering::SeqCst), before);
        if cancel {
            call.cancel().unwrap();
        }
        if advance {
            call.step(&mut cx(), &mut work(100000));
            for _ in 0..4 {
                call.step(&mut cx(), &mut work(100000));
            }
            assert_eq!(stats.cancels.load(Ordering::SeqCst), 1, "no cleanup retry");
            clock.0.store(16, Ordering::SeqCst);
        }
        for _ in 0..4 {
            call.step(&mut cx(), &mut work(100000));
        }
        assert!(
            handle
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .is_err()
        );
        assert_eq!(
            category(call.try_take(&mut work(100000)).unwrap().unwrap()),
            expected,
            "{mode:?}"
        );
        assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
        drop(call);
        drop(handle);
        for _ in 0..10000 {
            if let StartStatus::Ready(_) = runtime.shutdown(&mut cx(), &mut work(4096)) {
                break;
            }
        }
        assert_eq!(runtime.state(), CompiledPlanSetState::Reclaimed);
        drop(runtime);
        assert!(observer().0.is_some());
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn host_transfer_offer_unwind_retains_complete_call_until_recorded_residual() {
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Transfer);
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
        policy,
        clock.clone(),
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!());
    let handle = build(&runtime);
    let mut call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    call.cancel().unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime
            .step_with_transfer_offer(&mut cx(), &mut work(100000), &mut |_| panic!(
                "offer unwind"
            ))))
        .is_err()
    );
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 0);
    assert_eq!(parent.lock().unwrap()[8].used(), 1);
    assert!(call.try_take(&mut work(100000)).unwrap().is_none());
    clock.0.store(16, Ordering::SeqCst);
    call.step(&mut cx(), &mut work(100000));
    assert!(matches!(
        call.try_take(&mut work(100000)).unwrap().unwrap(),
        Err(CoreError::Application(_))
    ));
    assert!(observer().0.unwrap().is_residual());
    drop(call);
    drop(handle);
    shutdown(&runtime);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    drop(runtime);
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_completed_tickets_hold_no_root_and_live_ticket_survives_facade_drop() {
    let stats = Arc::new(Stats::default());
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs())),
        policy,
        clock,
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!());
    let handle = build(&runtime);
    let mut completed = Vec::new();
    for _ in 0..7 {
        let mut call = handle
            .read_property("a-read", InteractionOptions::new(), &mut work(100000))
            .unwrap();
        call.step(&mut cx(), &mut work(100000));
        assert_eq!(
            category(call.try_take(&mut work(100000)).unwrap().unwrap()),
            "ok"
        );
        completed.push(call);
    }
    stats.set_mode(Mode::LateStart);
    let mut call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    drop(runtime);
    drop(handle);
    call.step(&mut cx(), &mut work(100000));
    assert_eq!(
        category(call.try_take(&mut work(100000)).unwrap().unwrap()),
        "ok"
    );
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    assert!(observer().0.unwrap().is_residual());
    drop(completed);
    drop(call);
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_late_manual_cancel_gets_fresh_drain_time_without_refill_extension() {
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Pending);
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
        binding::host_registration(binding::HostClient::new(stats, outputs())),
        policy,
        clock.clone(),
        parent.clone(),
    )
    .unwrap_or_else(|_| panic!());
    let handle = build(&runtime);
    let mut call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    clock.0.store(1000, Ordering::SeqCst);
    call.cancel().unwrap();
    call.step(&mut cx(), &mut work(0));
    for _ in 0..5 {
        call.step(&mut cx(), &mut work(100000));
        assert!(call.try_take(&mut work(100000)).unwrap().is_none());
    }
    clock.0.store(1006, Ordering::SeqCst);
    call.step(&mut cx(), &mut work(100000));
    let error = call
        .try_take(&mut work(100000))
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.context().cause_code(), Some(401));
    drop(call);
    drop(handle);
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
}

#[test]
fn host_actual_provisioning_build_call_and_final_root_measurements() {
    fn layout<T>() -> (usize, usize) {
        (core::mem::size_of::<T>(), core::mem::align_of::<T>())
    }
    let layouts = [
        layout::<ConsumerPropertyReadRuntime>(),
        layout::<ConsumerPropertyReadHandle>(),
        layout::<ConsumerPropertyReadCall>(),
        layout::<HostConsumerPropertyReadBuild<'static, 'static>>(),
        layout::<ConsumerCompilerRegistration<HostConsumerBindingRegistration>>(),
        layout::<HostBindingCallBox<CoreResult<InteractionOutput>>>(),
        layout::<
            Result<
                HostBindingCallBox<CoreResult<InteractionOutput>>,
                BindingInputRejection<OutboundRequest>,
            >,
        >(),
        layout::<
            Result<
                ConsumerPropertyReadCall,
                BindingInputRejection<(InteractionOptions, Option<OutboundRequest>)>,
            >,
        >(),
        layout::<CoreResult<Option<CoreResult<InteractionOutput>>>>(),
    ];
    let stats = Arc::new(Stats::default());
    let registration =
        binding::host_registration(binding::HostClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let clock = Arc::new(Clock::default());
    observe(usize::MAX);
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .unwrap_or_else(|_| panic!());
    let provision = trace();
    disarm();
    let source = source();
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = true;
        t.set(s);
    });
    let handle = match runtime
        .consume(&source, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap()
    {
        StartStatus::Ready(h) => h,
        _ => panic!(),
    };
    let build = trace();
    disarm();
    drop(source);
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = true;
        t.set(s);
    });
    let mut call = handle
        .read_property(
            "c-温度",
            InteractionOptions::new().with_form_index(1),
            &mut work(100000),
        )
        .unwrap();
    let accepted = trace();
    call.step(&mut cx(), &mut work(100000));
    let result = trace();
    assert_eq!(result.requests, accepted.requests);
    assert_eq!(
        category(call.try_take(&mut work(100000)).unwrap().unwrap()),
        "ok"
    );
    assert_eq!(trace().live, build.live);
    drop(call);
    stats.set_mode(Mode::Pending);
    let call = handle
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    let before = trace();
    let polls = stats.polls.load(Ordering::SeqCst);
    let cancels = stats.cancels.load(Ordering::SeqCst);
    drop(call);
    drop(handle);
    drop(runtime);
    let after = trace();
    disarm();
    assert_eq!(before.requests, after.requests);
    assert_eq!(stats.polls.load(Ordering::SeqCst), polls);
    assert_eq!(stats.cancels.load(Ordering::SeqCst), cancels);
    assert!(observer().0.unwrap().is_residual());
    let terminal = observer().1;
    assert!(terminal[0] <= terminal[1]);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(after.live, provision.layouts[0].0);
    drop(observer);
    assert_eq!(trace().live, 0);
    let accounts = parent.lock().unwrap();
    assert!(accounts.iter().all(|a| a.used() == 0));
    println!(
        "host provisioning: live={}, peak={}, largest={}, requests={}, layouts={:?}",
        provision.live,
        provision.peak,
        provision.largest,
        provision.requests,
        &provision.layouts[..provision.requests]
    );
    println!(
        "host inline layouts (runtime, handle, ticket, build, registration, decorated carrier, Core construction return, Runtime admission return, result observation return)={layouts:?}; unread result live={}",
        result.live
    );
    println!(
        "host published: live={}, peak={}, largest={}, requests={}; call live={}, peak={}, layouts={:?}; terminal={terminal:?}; parent peaks={:?}",
        build.live,
        build.peak,
        build.largest,
        build.requests,
        accepted.live,
        accepted.peak,
        &accepted.layouts[build.requests..accepted.requests],
        accounts.iter().map(|a| a.peak()).collect::<Vec<_>>()
    );
}
