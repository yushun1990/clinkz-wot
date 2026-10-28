include!("support.rs");

#[test]
fn static_binding_storage_all_scopes_disable_and_boundary_before_invocation() {
    let required = rejected_static_startup(ResourceKind::BindingSlotStateBytesPerItemMax, 0);
    assert!(
        required >= core::mem::size_of::<StaticConsumerPropertyReadSlot<binding::State>>() as u64
    );
    for kind in [
        ResourceKind::BindingSlotStateBytesPerItemMax,
        ResourceKind::BindingSlotStateBytesPerThingMax,
        ResourceKind::BindingSlotStateBytesGlobalMax,
    ] {
        for limit in [0, required - 1] {
            assert_eq!(rejected_static_startup(kind, limit), required);
        }
        for limit in [required, required + 1] {
            let stats = Arc::new(Stats::default());
            let registration =
                binding::static_registration(StaticClient::new(stats.clone(), outputs()));
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(kind, Some(limit)));
            // Host storage is inapplicable to this representation.
            assert!(limits.set(ResourceKind::HostBindingCallBytesGlobalMax, Some(0)));
            let policy = policy_with(limits);
            let parent = RefCell::new(policy.parent_accounts());
            let clock = Clock::default();
            let mut report = report_storage();
            let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                &clock,
                &parent,
                &mut report,
            )
            .unwrap_or_else(|_| panic!("exact complete slot bound rejected"));
            publish_static(&mut runtime);
            let generation = runtime
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            runtime.step(&mut cx(), &mut work(100000));
            assert_eq!(
                category(
                    runtime
                        .try_take(generation, &mut work(100000))
                        .unwrap()
                        .unwrap()
                ),
                "ok"
            );
            assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
            drop(runtime);
            drop(observer);
            assert!(parent.borrow().iter().all(|a| a.used() == 0));
        }
    }
    println!("Static complete resident slot storage allowance={required}");
}

#[test]
fn static_callback_capacity_disable_and_exact_boundaries() {
    for kind in [
        ResourceKind::BindingPollTemporaryBytesPerCallMax,
        ResourceKind::BindingPollTemporaryBytesGlobalMax,
        ResourceKind::BindingCancelBufferBytesPerCallMax,
        ResourceKind::BindingCancelBufferBytesGlobalMax,
    ] {
        let required = rejected_static_startup(kind, 0);
        assert_eq!(rejected_static_startup(kind, required - 1), required);
        for limit in [required, required + 1] {
            for cancel in [false, true] {
                let stats = Arc::new(Stats::default());
                let mut limits = GatewayDefaultV1::LIMITS.clone();
                assert!(limits.set(kind, Some(limit)));
                // A static associated slot has no Host transfer destination.
                assert!(limits.set(ResourceKind::CleanupTransferSlotsGlobalMax, Some(0)));
                assert!(limits.set(ResourceKind::CleanupTransferBytesGlobalMax, Some(0)));
                let policy = policy_with(limits);
                let parent = RefCell::new(policy.parent_accounts());
                let clock = Clock::default();
                let mut report = report_storage();
                let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
                    binding::static_registration(StaticClient::new(stats.clone(), outputs())),
                    policy,
                    &clock,
                    &parent,
                    &mut report,
                )
                .unwrap_or_else(|_| panic!("exact callback bound rejected"));
                publish_static(&mut runtime);
                let generation = runtime
                    .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                    .unwrap();
                if cancel {
                    runtime.cancel(generation).unwrap();
                }
                runtime.step(&mut cx(), &mut work(100000));
                runtime.step(&mut cx(), &mut work(100000));
                assert_eq!(
                    category(
                        runtime
                            .try_take(generation, &mut work(100000))
                            .unwrap()
                            .unwrap()
                    ),
                    "ok"
                );
                assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
                if cancel {
                    assert!(stats.cancels.load(Ordering::SeqCst) > 0);
                }
                drop(runtime);
                drop(observer);
                assert!(parent.borrow().iter().all(|a| a.used() == 0));
            }
        }
        println!("Static {kind:?} complete allowance={required}");
    }
}

#[test]
fn static_callback_scratch_shared_parent_survives_slot_reuse_until_retirement() {
    let kind = ResourceKind::BindingPollTemporaryBytesGlobalMax;
    let required = rejected_static_startup(kind, 0);
    let policy_a = policy();
    let mut accounts = policy_a.parent_accounts();
    let index = ConsumerPropertyReadPolicy::PARENT_KINDS
        .iter()
        .position(|&k| k == kind)
        .unwrap();
    accounts[index] = ResourceAccount::new(SlotIndex::new(93), Generation::INITIAL, kind, required);
    let parent = RefCell::new(accounts);
    let clock = Clock::default();
    let mut storage_a = report_storage();
    let mut storage_b = report_storage();
    let stats_a = Arc::new(Stats::default());
    let (mut runtime_a, observer_a) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats_a.clone(), outputs())),
        policy_a,
        &clock,
        &parent,
        &mut storage_a,
    )
    .unwrap_or_else(|_| panic!());
    assert_eq!(parent.borrow()[index].used(), required);
    let stats_b = Arc::new(Stats::default());
    let registration_b =
        binding::static_registration(StaticClient::new(stats_b.clone(), outputs()));
    let policy_b = policy();
    observe(0);
    let rejection = match StaticConsumerPropertyReadRuntime::try_new(
        registration_b,
        policy_b,
        &clock,
        &parent,
        &mut storage_b,
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
    let (registration_b, policy_b) = rejection.into_input();
    assert_eq!(stats_b.starts.load(Ordering::SeqCst), 0);
    publish_static(&mut runtime_a);
    for cancel in [true, false] {
        let generation = runtime_a
            .read_property("a-read", InteractionOptions::new(), &mut work(100000))
            .unwrap();
        assert_eq!(parent.borrow()[index].used(), required);
        if cancel {
            runtime_a.cancel(generation).unwrap();
        }
        runtime_a.step(&mut cx(), &mut work(100000));
        assert_eq!(parent.borrow()[index].used(), required);
        runtime_a.step(&mut cx(), &mut work(100000));
        assert_eq!(parent.borrow()[index].used(), required);
        assert_eq!(
            category(
                runtime_a
                    .try_take(generation, &mut work(100000))
                    .unwrap()
                    .unwrap()
            ),
            "ok"
        );
        assert_eq!(parent.borrow()[index].used(), required);
    }
    assert_eq!(stats_a.starts.load(Ordering::SeqCst), 2);
    assert_eq!(stats_a.polls.load(Ordering::SeqCst), 1);
    assert_eq!(stats_a.cancels.load(Ordering::SeqCst), 2);
    for _ in 0..10000 {
        if matches!(
            runtime_a.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            break;
        }
        assert_eq!(parent.borrow()[index].used(), required);
    }
    assert_eq!(runtime_a.state(), CompiledPlanSetState::Reclaimed);
    assert_eq!(stats_a.clients_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(parent.borrow()[index].used(), 0);
    // The complete facade/slot remains live; retired registration scratch does not.
    let (mut runtime_b, observer_b) = StaticConsumerPropertyReadRuntime::try_new(
        registration_b,
        policy_b,
        &clock,
        &parent,
        &mut storage_b,
    )
    .unwrap_or_else(|_| panic!("retired scratch capacity unavailable"));
    publish_static(&mut runtime_b);
    let generation = runtime_b
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    runtime_b.step(&mut cx(), &mut work(100000));
    assert_eq!(
        category(
            runtime_b
                .try_take(generation, &mut work(100000))
                .unwrap()
                .unwrap()
        ),
        "ok"
    );
    drop(runtime_b);
    drop(observer_b);
    drop(runtime_a);
    drop(observer_a);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_binding_storage_shared_parent_is_held_for_the_resident_complete_slot() {
    let required = rejected_static_startup(ResourceKind::BindingSlotStateBytesGlobalMax, 0);
    let policy_a = policy();
    let mut accounts = policy_a.parent_accounts();
    let index = ConsumerPropertyReadPolicy::PARENT_KINDS
        .iter()
        .position(|&k| k == ResourceKind::BindingSlotStateBytesGlobalMax)
        .unwrap();
    accounts[index] = ResourceAccount::new(
        SlotIndex::new(91),
        Generation::INITIAL,
        ResourceKind::BindingSlotStateBytesGlobalMax,
        required,
    );
    let parent = RefCell::new(accounts);
    let clock = Clock::default();
    let mut storage_a = report_storage();
    let mut storage_b = report_storage();
    let stats_a = Arc::new(Stats::default());
    stats_a.set_mode(Mode::Complete);
    let (mut runtime_a, observer_a) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats_a.clone(), outputs())),
        policy_a,
        &clock,
        &parent,
        &mut storage_a,
    )
    .unwrap_or_else(|_| panic!());
    let stats_b = Arc::new(Stats::default());
    let registration_b =
        binding::static_registration(StaticClient::new(stats_b.clone(), outputs()));
    let (registration_b, policy_b) = match StaticConsumerPropertyReadRuntime::try_new(
        registration_b,
        policy(),
        &clock,
        &parent,
        &mut storage_b,
    ) {
        Err(rejection) => {
            assert!(matches!(
                rejection.error().error(),
                CoreError::Backpressure(_)
            ));
            rejection.into_input()
        }
        Ok(_) => panic!("independent global slot budget bypassed"),
    };
    assert_eq!(parent.borrow()[index].used(), required);
    publish_static(&mut runtime_a);
    let generation = runtime_a
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    runtime_a.cancel(generation).unwrap();
    runtime_a.step(&mut cx(), &mut work(100000));
    assert_eq!(parent.borrow()[index].used(), required);
    assert_eq!(
        category(
            runtime_a
                .try_take(generation, &mut work(100000))
                .unwrap()
                .unwrap()
        ),
        "cancelled"
    );
    // A vacant slot still occupies its physically provisioned inline backing.
    assert_eq!(parent.borrow()[index].used(), required);
    drop(runtime_a);
    assert_eq!(parent.borrow()[index].used(), 0);
    drop(observer_a);
    let (mut runtime_b, observer_b) = StaticConsumerPropertyReadRuntime::try_new(
        registration_b,
        policy_b,
        &clock,
        &parent,
        &mut storage_b,
    )
    .unwrap_or_else(|_| panic!("released resident capacity unavailable"));
    publish_static(&mut runtime_b);
    let generation = runtime_b
        .read_property("c-温度", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    runtime_b.step(&mut cx(), &mut work(100000));
    assert_eq!(
        category(
            runtime_b
                .try_take(generation, &mut work(100000))
                .unwrap()
                .unwrap()
        ),
        "ok"
    );
    assert_eq!(stats_b.starts.load(Ordering::SeqCst), 1);
    drop(runtime_b);
    drop(observer_b);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_report_complete_footprint_is_checked_before_provisioning() {
    let required = rejected_static_startup(ResourceKind::CleanupItemBytesMax, 1);
    assert_eq!(required, core::mem::size_of::<ReportStorage>() as u64);
    for limit in [0, required - 1] {
        assert_eq!(
            rejected_static_startup(ResourceKind::CleanupItemBytesMax, limit),
            required
        );
    }
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::CleanupItemBytesMax, Some(required)));
    let policy = policy_with(limits);
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let stats = Arc::new(Stats::default());
    let (runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats, outputs())),
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!("exact static report bound rejected"));
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_cleanup_quantum_caps_the_whole_shutdown_and_retains_pending_work() {
    for too_small in [1, 3] {
        assert_eq!(
            rejected_static_startup(ResourceKind::CleanupWorkItemsPerStepMax, too_small),
            4
        );
    }
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Complete);
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::CleanupWorkItemsPerStepMax, Some(5)));
    let policy = policy_with_cleanup_costs(limits);
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats.clone(), outputs())),
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    publish_static(&mut runtime);
    let generation = runtime
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    runtime.abandon_call(generation).unwrap();
    let mut first = work(100000);
    runtime.step(&mut cx(), &mut first);
    assert_eq!(100000 - first.remaining(WorkClass::CleanupItems), 1);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 0);
    assert_eq!(stats.acknowledgements.load(Ordering::SeqCst), 0);
    let mut finished = false;
    let mut pending = 0;
    for _ in 0..10000 {
        let mut budget = work(100000);
        let status = runtime.shutdown(&mut cx(), &mut budget);
        assert!(100000 - budget.remaining(WorkClass::CleanupItems) <= 5);
        if let StartStatus::Ready(outcome) = status {
            assert!(outcome.is_complete());
            finished = true;
            break;
        }
        pending += 1;
    }
    assert!(finished && pending > 0);
    assert_eq!(stats.cancels.load(Ordering::SeqCst), 1);
    assert_eq!(stats.acknowledgements.load(Ordering::SeqCst), 1);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}
#[test]
fn static_common_case_table_and_exact_acknowledgement() {
    println!("static common case table version={CASE_TABLE_VERSION}");
    for (mode, cancel, advance, expected) in CASES {
        let stats = Arc::new(Stats::default());
        stats.set_mode(mode);
        let registration =
            binding::static_registration(StaticClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = RefCell::new(policy.parent_accounts());
        let mut report = report_storage();
        let clock = Clock::default();
        let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &clock,
            &parent,
            &mut report,
        )
        .unwrap_or_else(|_| panic!("construct static"));
        let source = source();
        assert!(matches!(
            runtime.consume(&source, &mut work(100000), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!("build stalled");
            }),
            Ok(StartStatus::Ready(()))
        ));
        drop(source);
        let generation = runtime
            .read_property(
                "a-read",
                InteractionOptions::new().with_form_index(2),
                &mut work(100000),
            )
            .unwrap();
        assert!(
            runtime
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .is_err()
        );
        let before = stats.polls.load(Ordering::SeqCst);
        runtime.step(&mut cx(), &mut work(0));
        assert_eq!(stats.polls.load(Ordering::SeqCst), before);
        if cancel {
            runtime.cancel(generation).unwrap();
        }
        if advance {
            runtime.step(&mut cx(), &mut work(100000));
            for _ in 0..4 {
                runtime.step(&mut cx(), &mut work(100000));
            }
            assert_eq!(stats.cancels.load(Ordering::SeqCst), 1, "no cleanup retry");
            clock.0.store(16, Ordering::SeqCst);
        }
        for _ in 0..4 {
            runtime.step(&mut cx(), &mut work(100000));
        }
        assert_eq!(
            category(
                runtime
                    .try_take(generation, &mut work(100000))
                    .unwrap()
                    .unwrap()
            ),
            expected,
            "{mode:?}"
        );
        assert_eq!(
            stats.acknowledgements.load(Ordering::SeqCst),
            usize::from(!advance)
        );
        assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
        runtime.close();
        for _ in 0..10000 {
            if matches!(
                runtime.shutdown(&mut cx(), &mut work(4096)),
                StartStatus::Ready(_)
            ) {
                break;
            }
        }
        assert_eq!(runtime.state(), CompiledPlanSetState::Reclaimed);
        drop(runtime);
        assert!(observer().0.is_some());
        drop(observer);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn static_distinct_compiled_coordinates_repeated_reads_and_generation_reuse() {
    let stats = Arc::new(Stats::default());
    let registration = binding::static_registration(StaticClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    let mut previous = runtime.generations();
    for _ in 0..2 {
        let source = source();
        runtime
            .consume(&source, &mut work(100000), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!();
            })
            .unwrap();
        drop(source);
        let current = runtime.generations();
        assert_ne!((previous.0, previous.1), (current.0, current.1));
        previous = current;
        for (name, form, target) in [
            ("a-read", 2, "https://peer.test/a/second"),
            ("c-温度", 1, "https://peer.test/a/last"),
        ] {
            let generation = runtime
                .read_property(
                    name,
                    InteractionOptions::new().with_form_index(form),
                    &mut work(100000),
                )
                .unwrap();
            runtime.step(&mut cx(), &mut work(100000));
            assert_eq!(
                category(
                    runtime
                        .try_take(generation, &mut work(100000))
                        .unwrap()
                        .unwrap()
                ),
                "ok"
            );
            let n = stats.target_len.load(Ordering::SeqCst);
            let bytes: Vec<_> = stats.target[..n]
                .iter()
                .map(|v| v.load(Ordering::SeqCst))
                .collect();
            assert_eq!(bytes, target.as_bytes());
            assert!(runtime.cancel(Generation::INITIAL).is_err());
        }
        runtime.close();
        for _ in 0..10000 {
            runtime.step(&mut cx(), &mut work(4096));
            if runtime.state() == CompiledPlanSetState::Reclaimed {
                break;
            }
        }
    }
    for _ in 0..10000 {
        if matches!(
            runtime.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            break;
        }
    }
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
    assert_eq!(stats.starts.load(Ordering::SeqCst), 4);
}

#[test]
fn static_final_root_drop_is_finite_source_free_and_reported_before_destruction() {
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Pending);
    let registration = binding::static_registration(StaticClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    observe(usize::MAX);
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    disarm();
    let source = source();
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = true;
        t.set(s);
    });
    runtime
        .consume(&source, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap();
    runtime
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    let before = trace();
    let polls = stats.polls.load(Ordering::SeqCst);
    let cancels = stats.cancels.load(Ordering::SeqCst);
    drop(runtime);
    let after = trace();
    disarm();
    assert_eq!(after.requests, before.requests);
    assert_eq!(after.live, 0);
    assert_eq!(stats.polls.load(Ordering::SeqCst), polls);
    assert_eq!(stats.cancels.load(Ordering::SeqCst), cancels);
    assert!(observer().0.unwrap().is_residual());
    assert!(observer().1[0] <= observer().1[1]);
    let terminal = observer().1;
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
    println!(
        "static build: live={}, peak={}, largest={}, requests={}; terminal={terminal:?}; deallocations={}",
        before.live, before.peak, before.largest, before.requests, after.releases
    );
}

#[test]
fn static_whole_provisioning_request_call_and_global_live_measurements() {
    let thing = source();
    observe(usize::MAX);
    let stats = Arc::new(Stats::default());
    let registration = binding::static_registration(StaticClient::new(stats.clone(), outputs()));
    let binding_provision = trace();
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    let provision = trace();
    assert_eq!(provision.requests, binding_provision.requests);
    runtime
        .consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!("bounded fixture build stalled");
        })
        .unwrap();
    let built = trace();
    let mut options = InteractionOptions::new();
    options.uri_variables.insert("room".into(), "west".into());
    let request = trace();
    let generation = runtime
        .read_property("a-read", options, &mut work(100000))
        .unwrap();
    let accepted = trace();
    runtime.step(&mut cx(), &mut work(100000));
    assert_eq!(
        category(
            runtime
                .try_take(generation, &mut work(100000))
                .unwrap()
                .unwrap()
        ),
        "ok"
    );
    assert_eq!(trace().requests, request.requests);
    for _ in 0..100000 {
        if let StartStatus::Ready(outcome) = runtime.shutdown(&mut cx(), &mut work(4096)) {
            assert!(outcome.is_complete());
            break;
        }
    }
    assert_eq!(runtime.state(), CompiledPlanSetState::Reclaimed);
    drop(runtime);
    assert!(observer().0.unwrap().is_complete());
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
    drop(stats);
    let final_trace = trace();
    disarm();
    assert_eq!(final_trace.live, 0);
    println!(
        "whole static fixture: binding provisioning live={}, allocations={}; runtime provisioning allocations={}; build live={}, peak={}, largest={}; request allocations={:?}; call allocations={}; global peak={}, final retained={}, releases={}",
        binding_provision.live,
        binding_provision.requests,
        provision.requests - binding_provision.requests,
        built.live,
        built.peak,
        built.largest,
        &request.layouts[built.requests..request.requests],
        accepted.requests - request.requests,
        final_trace.peak,
        final_trace.live,
        final_trace.releases
    );
}

#[cfg(feature = "std")]
mod host;

#[test]
fn static_every_build_allocation_failure_recovers_and_releases_original_accounts() {
    let mut sites = 0;
    for fail in 0..256 {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::static_registration(StaticClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = RefCell::new(policy.parent_accounts());
        let clock = Clock::default();
        let mut report = report_storage();
        let source = source();
        let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &clock,
            &parent,
            &mut report,
        )
        .unwrap_or_else(|_| panic!());
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
            assert!(result.is_err(), "site {fail}: {result:?}");
            sites += 1;
        } else {
            assert!(matches!(result, Ok(StartStatus::Ready(()))));
        }
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        for _ in 0..10000 {
            if matches!(
                runtime.shutdown(&mut cx(), &mut work(4096)),
                StartStatus::Ready(_)
            ) {
                break;
            }
        }
        assert_eq!(
            runtime.state(),
            CompiledPlanSetState::Reclaimed,
            "site {fail}"
        );
        drop(runtime);
        drop(observer);
        assert_eq!(trace().live, 0, "site {fail}");
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
        if fail >= requests {
            break;
        }
    }
    assert!(sites > 20);
    println!("static recoverable build allocation sites={sites}");
}

#[test]
fn static_abandon_and_cancel_at_every_build_step_keep_a_bounded_cleanup_owner() {
    let mut completed_at = 0;
    for cutoff in (0..20000).step_by(1) {
        let stats = Arc::new(Stats::default());
        let registration =
            binding::static_registration(StaticClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = RefCell::new(policy.parent_accounts());
        let clock = Clock::default();
        let mut report = report_storage();
        let source = source();
        let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &clock,
            &parent,
            &mut report,
        )
        .unwrap_or_else(|_| panic!());
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
        assert_eq!(before.requests, after.requests, "abandoned step {cutoff}");
        assert_eq!(after.live, 0, "abandoned step {cutoff}");
        assert!(observer().0.unwrap().is_residual());
        assert!(observer().1[0] <= observer().1[1]);
        drop(observer);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        if finished {
            completed_at = cutoff;
            break;
        }
    }
    assert!(completed_at > 30);
    println!("static unpublished step boundaries={completed_at}");
}

#[test]
fn static_deadline_first_cause_fabricated_transfer_and_unread_result_cleanup() {
    for mode in [
        Mode::Pending,
        Mode::Transfer,
        Mode::Normal,
        Mode::AckFailure,
    ] {
        let stats = Arc::new(Stats::default());
        stats.set_mode(mode);
        let registration =
            binding::static_registration(StaticClient::new(stats.clone(), outputs()));
        let policy = policy();
        let parent = RefCell::new(policy.parent_accounts());
        let clock = Clock::default();
        let mut report = report_storage();
        let source = source();
        let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &clock,
            &parent,
            &mut report,
        )
        .unwrap_or_else(|_| panic!());
        runtime
            .consume(&source, &mut work(100000), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!();
            })
            .unwrap();
        let generation = runtime
            .read_property("a-read", InteractionOptions::new(), &mut work(100000))
            .unwrap();
        if mode == Mode::Normal || mode == Mode::AckFailure {
            runtime.step(&mut cx(), &mut work(100000));
        } else {
            runtime.cancel(generation).unwrap();
            runtime.close();
            runtime.step(&mut cx(), &mut work(100000));
            assert!(
                runtime
                    .try_take(generation, &mut work(100000))
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                runtime.next_deadline().unwrap().instant().unwrap().ticks(),
                15
            );
            clock.0.store(16, Ordering::SeqCst);
            runtime.step(&mut cx(), &mut work(100000));
            let error = runtime
                .try_take(generation, &mut work(100000))
                .unwrap()
                .unwrap()
                .unwrap_err();
            if mode == Mode::Transfer {
                assert!(matches!(error, CoreError::Validation(_)));
            } else {
                assert_eq!(error.context().cause_code(), Some(401));
            } // Manual wins over Drain/expiry.
            assert!(runtime.cleanup_record().unwrap().is_residual());
        }
        let before = trace();
        drop(runtime);
        assert_eq!(trace().requests, before.requests);
        assert!(observer().0.unwrap().is_residual());
        drop(observer);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn static_final_credit_rejection_input_capacity_and_partial_rollback() {
    let stats = Arc::new(Stats::default());
    let registration = binding::static_registration(StaticClient::new(stats.clone(), outputs()));
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let source = source();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
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
    let mut short = work(ConsumerPropertyReadPolicy::FINAL_INSTALL_WORK - 1);
    runtime.step(&mut cx(), &mut short);
    assert_eq!(runtime.state(), CompiledPlanSetState::Frozen);
    assert_eq!(
        short.remaining(WorkClass::PlanningItems),
        ConsumerPropertyReadPolicy::FINAL_INSTALL_WORK - 1
    );
    runtime.step(
        &mut cx(),
        &mut work(ConsumerPropertyReadPolicy::FINAL_INSTALL_WORK),
    );
    assert_eq!(runtime.state(), CompiledPlanSetState::Published);
    let mut oversized = String::with_capacity(128);
    oversized.push_str("west");
    let address = oversized.as_ptr();
    let mut options = InteractionOptions::new();
    options.uri_variables.insert("room".into(), oversized);
    let rejection = runtime
        .read_property("a-read", options, &mut work(100000))
        .unwrap_err();
    let (options, request) = rejection.into_input();
    assert!(request.is_none());
    assert_eq!(options.uri_variables["room"].as_ptr(), address);
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    runtime.close();
    // One byte of the original rollback is paid; Drop must continue with that
    // retained partial byte credit and a single finite, non-refilled allowance.
    runtime.step(&mut cx(), &mut work(1));
    let before = trace();
    drop(runtime);
    assert_eq!(trace().requests, before.requests);
    assert!(observer().0.unwrap().is_residual());
    assert!(observer().1[0] <= observer().1[1]);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_two_roots_exhaust_the_same_real_build_output_and_call_parent() {
    for resource in [
        ResourceKind::AdmissionTemporaryBytesGlobalMax,
        ResourceKind::CompiledRuntimeBytesGlobalMax,
        ResourceKind::PlanPinsGlobalMax,
        ResourceKind::BindingCancelBufferBytesGlobalMax,
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
            ResourceKind::BindingCancelBufferBytesGlobalMax => rejected_static_startup(resource, 0),
            _ => 1,
        };
        accounts[index] =
            ResourceAccount::new(SlotIndex::new(0), Generation::INITIAL, resource, cap);
        let parent = RefCell::new(accounts);
        let clock = Clock::default();
        let mut report_a = report_storage();
        let mut report_b = report_storage();
        let stats_a = Arc::new(Stats::default());
        let stats_b = Arc::new(Stats::default());
        let (mut a, observer_a) = StaticConsumerPropertyReadRuntime::try_new(
            binding::static_registration(StaticClient::new(stats_a, outputs())),
            policy_a,
            &clock,
            &parent,
            &mut report_a,
        )
        .unwrap_or_else(|_| panic!());
        let (mut b, observer_b) = StaticConsumerPropertyReadRuntime::try_new(
            binding::static_registration(StaticClient::new(stats_b.clone(), outputs())),
            policy_b,
            &clock,
            &parent,
            &mut report_b,
        )
        .unwrap_or_else(|_| panic!());
        let thing = source();
        a.consume(&thing, &mut work(100000), |build| {
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
        .unwrap();
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
                result.unwrap();
                a.read_property("a-read", InteractionOptions::new(), &mut work(100000))
                    .unwrap();
                assert!(matches!(
                    b.read_property("a-read", InteractionOptions::new(), &mut work(100000))
                        .unwrap_err()
                        .error()
                        .error(),
                    CoreError::Backpressure(_)
                ));
            }
        }
        assert_eq!(stats_b.starts.load(Ordering::SeqCst), 0);
        drop(a);
        drop(b);
        drop(observer_a);
        drop(observer_b);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn static_missing_id_and_later_bounds_reject_before_any_client_start() {
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
        let parent = RefCell::new(policy.parent_accounts());
        let clock = Clock::default();
        let mut report = report_storage();
        let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
            binding::static_registration(StaticClient::new(stats.clone(), outputs())),
            policy,
            &clock,
            &parent,
            &mut report,
        )
        .unwrap_or_else(|_| panic!());
        let result = runtime.consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        });
        assert!(result.is_err());
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        if missing_id {
            assert_eq!(parent.borrow()[3].peak(), 0);
        }
        drop(runtime);
        assert!(observer().0.unwrap().is_residual());
        drop(observer);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
    }
}

#[test]
fn static_normal_immediate_and_polled_late_results_share_core_validation() {
    for mode in [Mode::Normal, Mode::LateStart, Mode::LatePoll] {
        for validation in 1..=3 {
            let stats = Arc::new(Stats::default());
            stats.set_mode(mode);
            stats.validation.store(validation, Ordering::SeqCst);
            let policy = policy();
            let parent = RefCell::new(policy.parent_accounts());
            let clock = Clock::default();
            let mut report = report_storage();
            let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
                binding::static_registration(StaticClient::new(stats.clone(), outputs())),
                policy,
                &clock,
                &parent,
                &mut report,
            )
            .unwrap_or_else(|_| panic!());
            let thing = source();
            runtime
                .consume(&thing, &mut work(100000), |b| {
                    for _ in 0..100000 {
                        b.step(&mut work(4096));
                        if b.is_finished() {
                            return;
                        }
                    }
                    panic!();
                })
                .unwrap();
            let generation = runtime
                .read_property(
                    "c-温度",
                    InteractionOptions::new().with_form_index(1),
                    &mut work(100000),
                )
                .unwrap();
            if mode != Mode::Normal {
                runtime.cancel(generation).unwrap();
            }
            for _ in 0..3 {
                runtime.step(&mut cx(), &mut work(100000));
            }
            assert_eq!(
                category(
                    runtime
                        .try_take(generation, &mut work(100000))
                        .unwrap()
                        .unwrap()
                ),
                if validation == 3 {
                    "binding"
                } else {
                    "validation"
                }
            );
            assert_eq!(stats.starts.load(Ordering::SeqCst), 1);
            assert_eq!(stats.acknowledgements.load(Ordering::SeqCst), 1);
            drop(runtime);
            drop(observer);
            assert!(parent.borrow().iter().all(|a| a.used() == 0));
        }
    }
}

#[test]
fn static_ack_failure_is_bounded_by_independent_shutdown_deadline() {
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::AckFailure);
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats.clone(), outputs())),
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    let thing = source();
    runtime
        .consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap();
    runtime
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    runtime.step(&mut cx(), &mut work(100000));
    assert!(matches!(
        runtime.shutdown(&mut cx(), &mut work(100000)),
        StartStatus::Pending
    ));
    clock.0.store(16, Ordering::SeqCst);
    for _ in 0..10000 {
        if let StartStatus::Ready(outcome) = runtime.shutdown(&mut cx(), &mut work(4096)) {
            assert!(outcome.is_residual());
            break;
        }
    }
    assert_eq!(runtime.state(), CompiledPlanSetState::Reclaimed);
    assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_policy_qualification_is_before_clock_parent_input_and_allocation() {
    struct ClockObserved(core::sync::atomic::AtomicUsize);
    impl RuntimeClock for ClockObserved {
        fn now(&self) -> MonotonicInstant {
            self.0.fetch_add(1, Ordering::SeqCst);
            MonotonicInstant::new(ClockId::new(1), 10)
        }
        fn ticks_per_second(&self) -> core::num::NonZeroU64 {
            core::num::NonZeroU64::new(1000).unwrap()
        }
    }
    let stats = Arc::new(Stats::default());
    let registration = binding::static_registration(StaticClient::new(stats.clone(), outputs()));
    let policy = ConsumerPropertyReadPolicy::try_new(
        GatewayDefaultV1::LIMITS.clone(),
        BindingLifetimeFootprint::new(4, 4096),
        BindingLifetimeFootprint::new(17, 8193),
        [1, 64, 64, 64],
        [64, 32],
        [2, 2, 1, 1, 1, 1, 1],
        [50, 5],
        65536,
    )
    .unwrap();
    let parent = RefCell::new(policy.parent_accounts());
    let mut report = report_storage();
    let clock = ClockObserved(core::sync::atomic::AtomicUsize::new(0));
    observe(0);
    let rejected = match StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    ) {
        Err(e) => e,
        Ok(_) => panic!(),
    };
    assert_eq!(trace().requests, 0);
    disarm();
    assert_eq!(clock.0.load(Ordering::SeqCst), 0);
    assert!(parent.borrow().iter().all(|a| a.peak() == 0));
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
    drop(rejected);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    assert!(
        ConsumerPropertyReadPolicy::try_new(
            GatewayDefaultV1::LIMITS.clone(),
            BindingLifetimeFootprint::new(4, 4096),
            BindingLifetimeFootprint::new(8, 1280),
            [1, 64, 64, 64],
            [64, 32],
            [0, 2, 1, 1, 1, 1, 1],
            [50, 5],
            65536
        )
        .is_err()
    );
}

#[test]
fn static_native_layouts_and_original_td_diagnostic() {
    use core::mem::{align_of, size_of};
    println!(
        "static layouts: runtime={:?}, build={:?}, slot={:?}, state={:?}, registration={:?}, request={:?}, result={:?}, report={:?}",
        (
            size_of::<StaticConsumerPropertyReadRuntime<'static, StaticClient>>(),
            align_of::<StaticConsumerPropertyReadRuntime<'static, StaticClient>>()
        ),
        (
            size_of::<StaticConsumerPropertyReadBuild<'static, 'static, 'static, StaticClient>>(),
            align_of::<StaticConsumerPropertyReadBuild<'static, 'static, 'static, StaticClient>>()
        ),
        (
            size_of::<StaticConsumerPropertyReadSlot<binding::State>>(),
            align_of::<StaticConsumerPropertyReadSlot<binding::State>>()
        ),
        (size_of::<binding::State>(), align_of::<binding::State>()),
        (
            size_of::<
                InlineConsumerCompilerRegistration<StaticConsumerBindingPreparation<StaticClient>>,
            >(),
            align_of::<
                InlineConsumerCompilerRegistration<StaticConsumerBindingPreparation<StaticClient>>,
            >()
        ),
        (size_of::<OutboundRequest>(), align_of::<OutboundRequest>()),
        (
            size_of::<CoreResult<InteractionOutput>>(),
            align_of::<CoreResult<InteractionOutput>>()
        ),
        (size_of::<ReportStorage>(), align_of::<ReportStorage>())
    );
    let stats = Arc::new(Stats::default());
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats.clone(), outputs())),
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    let mut thing = source();
    thing._metadata.title = None;
    assert!(
        runtime
            .consume(&thing, &mut work(100000), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!();
            })
            .is_err()
    );
    assert!(matches!(
        runtime.td_diagnostic(),
        Some(clinkz_wot_td::ValidatedThingCause::Invalid(_))
    ));
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_reference_profile_constructs_reads_and_reclaims_without_changing_budgets() {
    let stats = Arc::new(Stats::default());
    let policy = policy_with(BenchmarkStaticReferenceV1::LIMITS.clone());
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats, outputs())),
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!("reference construction"));
    let thing = source();
    assert!(matches!(
        runtime.consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        }),
        Ok(StartStatus::Ready(()))
    ));
    let generation = runtime
        .read_property(
            "c-温度",
            InteractionOptions::new().with_form_index(1),
            &mut work(100000),
        )
        .unwrap();
    runtime.step(&mut cx(), &mut work(4096));
    assert_eq!(
        category(
            runtime
                .try_take(generation, &mut work(4096))
                .unwrap()
                .unwrap()
        ),
        "ok"
    );
    for _ in 0..10000 {
        if matches!(
            runtime.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            break;
        }
    }
    assert_eq!(runtime.state(), CompiledPlanSetState::Reclaimed);
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn static_late_manual_cancel_gets_fresh_drain_time_without_refill_extension() {
    let stats = Arc::new(Stats::default());
    stats.set_mode(Mode::Pending);
    let policy = policy();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        binding::static_registration(StaticClient::new(stats, outputs())),
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!());
    let thing = source();
    runtime
        .consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!();
        })
        .unwrap();
    let generation = runtime
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    clock.0.store(1000, Ordering::SeqCst);
    runtime.cancel(generation).unwrap();
    runtime.step(&mut cx(), &mut work(0));
    for _ in 0..5 {
        runtime.step(&mut cx(), &mut work(100000));
        assert_eq!(
            runtime.next_deadline().unwrap().instant().unwrap().ticks(),
            1005
        );
        assert!(
            runtime
                .try_take(generation, &mut work(100000))
                .unwrap()
                .is_none()
        );
    }
    clock.0.store(1006, Ordering::SeqCst);
    runtime.step(&mut cx(), &mut work(100000));
    let error = runtime
        .try_take(generation, &mut work(100000))
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.context().cause_code(), Some(401));
    for _ in 0..10000 {
        if matches!(
            runtime.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            break;
        }
    }
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[cfg(feature = "std")]
mod ownership;
#[cfg(not(feature = "std"))]
#[path = "ownership_mixed_cells.rs"]
mod ownership;
