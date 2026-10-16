//! Successor evidence through the production Core/Planning/Servient owners.
use super::*;
use binding::ownership::{OwnedPolicy, PrivatePool};

fn owned_policy(contract: BindingResourceContract) -> ConsumerPropertyReadPolicy {
    owned_policy_with_limits(contract, GatewayDefaultV1::LIMITS.clone())
}
fn owned_policy_with_limits(
    contract: BindingResourceContract,
    limits: ResourceLimits,
) -> ConsumerPropertyReadPolicy {
    ConsumerPropertyReadPolicy::try_new_owned(
        contract,
        ResourceProfileId::GATEWAY_DEFAULT_V1,
        limits,
        BindingLifetimeFootprint::new(4, 4096),
        BindingLifetimeFootprint::new(8, 1280),
        [1, 64, 64, 64],
        [64, 32],
        [2, 2, 1, 1, 1, 1, 1],
        [50, 5],
        65536,
    )
    .unwrap()
}
fn pool() -> Arc<PrivatePool> {
    PrivatePool::new(16, Arc::new(core::sync::atomic::AtomicUsize::new(0)))
}
fn author(
    contract: BindingResourceContract,
    mode: Mode,
    pool: Arc<PrivatePool>,
    dispose: bool,
) -> OwnedPolicy {
    OwnedPolicy::try_new(contract, mode, pool, dispose, 64, 64, 32).unwrap()
}
fn stop_static(r: &mut StaticConsumerPropertyReadRuntime<'_, StaticClient>) {
    r.close();
    for _ in 0..10000 {
        if matches!(
            r.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            return;
        }
    }
    panic!("shutdown stalled")
}

#[test]
fn owned_provisioning_contract_and_result_mismatch_precede_clock_allocation_and_parent_effects() {
    struct NoClock;
    impl RuntimeClock for NoClock {
        fn now(&self) -> MonotonicInstant {
            panic!("mismatch sampled the clock")
        }
        fn ticks_per_second(&self) -> core::num::NonZeroU64 {
            panic!("mismatch observed clock configuration")
        }
    }
    for strict in [true, false] {
        let contract = BindingResourceContract::ConsumerOwnedV2;
        let policy = if strict {
            policy()
        } else {
            ConsumerPropertyReadPolicy::try_new_owned(
                contract,
                ResourceProfileId::GATEWAY_DEFAULT_V1,
                GatewayDefaultV1::LIMITS.clone(),
                BindingLifetimeFootprint::new(4, 4096),
                BindingLifetimeFootprint::new(8, 1280),
                [1, 64, 64, 64],
                [63, 32],
                [2, 2, 1, 1, 1, 1, 1],
                [50, 5],
                65536,
            )
            .unwrap()
        };
        let key = policy.resource_identity();
        let parent = RefCell::new(policy.parent_accounts());
        let mut report = report_storage();
        let stats = Arc::new(Stats::default());
        let registration = binding::static_registration(StaticClient::new_owned(
            stats.clone(),
            outputs(),
            author(contract, Mode::Normal, pool(), false),
        ));
        let id = registration.registration().identity();
        observe(0);
        let error = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &NoClock,
            &parent,
            &mut report,
        )
        .err()
        .expect("mismatch");
        assert_eq!(trace().requests, 0);
        disarm();
        assert_eq!(error.input().0.registration().identity(), id);
        assert_eq!(
            error
                .error()
                .error()
                .context()
                .resource_failure()
                .unwrap()
                .0,
            key
        );
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
        assert!(
            parent
                .borrow()
                .iter()
                .all(|a| a.used() == 0 && a.peak() == 0)
        );
    }
}

#[test]
fn owned_parent_pressure_reports_actual_limiting_coordinates_without_losing_inputs() {
    for (index, kind, slot) in [
        (0, ResourceKind::EngineLiveBytesGlobalMax, 911),
        (10, ResourceKind::CleanupItemsMax, 912),
    ] {
        let contract = BindingResourceContract::ConsumerOwnedV2;
        let policy = owned_policy(contract);
        let key = policy.resource_identity();
        let mut bank = policy.parent_accounts();
        let generation = Generation::INITIAL.checked_next().unwrap();
        bank[index] = ResourceAccount::new(SlotIndex::new(slot), generation, kind, 0);
        let bank = RefCell::new(bank);
        let mut report = report_storage();
        let clock = Clock::default();
        let registration = binding::static_registration(StaticClient::new_owned(
            Arc::new(Stats::default()),
            outputs(),
            author(contract, Mode::Normal, pool(), false),
        ));
        let identity = registration.registration().identity();
        observe(0);
        let rejected = StaticConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            &clock,
            &bank,
            &mut report,
        )
        .err()
        .expect("actual parent exhausted");
        assert_eq!(trace().requests, 0);
        disarm();
        let error = rejected.error().error();
        assert_eq!(rejected.input().0.registration().identity(), identity);
        assert_eq!(
            error.context().resource_failure(),
            Some((
                key,
                ResourceFailureDomain::EngineCapacity,
                (index == 0).then_some((SlotIndex::new(0), Generation::INITIAL)),
                Some((SlotIndex::new(slot), generation))
            ))
        );
        assert_eq!(error.limit_details().unwrap().0, kind);
        assert_eq!(error.limit_details().unwrap().1, 0);
        if index == 10 {
            assert_eq!(error.limit_details().unwrap().2, Some(1));
        } else {
            assert!(error.limit_details().unwrap().2.unwrap() > 0);
        }
        assert_eq!(error.limit_details().unwrap().3, Some(0));
        assert!(bank.borrow().iter().all(|a| a.used() == 0));
        assert_eq!(bank.borrow()[index].peak(), 0);
        assert!(report.0.get().0.is_none());
        assert_eq!(report.1.get(), 0);
        assert_eq!(report.2.borrow().used(), 0);
    }
}

#[test]
fn checked_row_rejections_keep_owned_context_and_strict_behavior_before_effects() {
    struct NoClock;
    impl RuntimeClock for NoClock {
        fn now(&self) -> MonotonicInstant {
            panic!("row rejection sampled the clock")
        }
        fn ticks_per_second(&self) -> core::num::NonZeroU64 {
            panic!("row rejection observed clock configuration")
        }
    }
    for kind in [
        ResourceKind::BindingSlotStateBytesPerItemMax,
        ResourceKind::BindingSlotStateBytesPerThingMax,
        ResourceKind::BindingSlotStateBytesGlobalMax,
    ] {
        for limit in [None, Some(0)] {
            for owned in [false, true] {
                let mut limits = GatewayDefaultV1::LIMITS.clone();
                assert!(limits.set(kind, limit));
                let policy = if owned {
                    owned_policy_with_limits(BindingResourceContract::ConsumerOwnedV2, limits)
                } else {
                    policy_with(limits)
                };
                let key = policy.resource_identity();
                let parent = RefCell::new(policy.parent_accounts());
                let mut report = report_storage();
                let stats = Arc::new(Stats::default());
                let client = if owned {
                    StaticClient::new_owned(
                        stats.clone(),
                        outputs(),
                        author(
                            BindingResourceContract::ConsumerOwnedV2,
                            Mode::Normal,
                            pool(),
                            false,
                        ),
                    )
                } else {
                    StaticClient::new(stats.clone(), outputs())
                };
                let registration = binding::static_registration(client);
                let identity = registration.registration().identity();
                observe(usize::MAX);
                let rejected = StaticConsumerPropertyReadRuntime::try_new(
                    registration,
                    policy,
                    &NoClock,
                    &parent,
                    &mut report,
                )
                .err()
                .expect("missing or zero applicable row must reject");
                assert_eq!(trace().requests, 0);
                disarm();
                let error = rejected.error().error();
                if limit.is_none() {
                    assert!(matches!(error, CoreError::UnsupportedOperation(_)));
                } else {
                    let (actual, ceiling, requested, observed) = error.limit_details().unwrap();
                    assert_eq!((actual, ceiling, observed), (kind, 0, None));
                    assert!(requested.unwrap() > 0);
                }
                assert_eq!(
                    error.context().resource_failure(),
                    owned.then_some((key, ResourceFailureDomain::EngineCapacity, None, None)),
                    "{kind:?} {limit:?} owned={owned}"
                );
                assert_eq!(error.context().phase(), ErrorPhase::Admission);
                assert_eq!(error.context().retry_class(), RetryClass::Never);
                assert_eq!(rejected.input().0.registration().identity(), identity);
                assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
                assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
                assert!(
                    parent
                        .borrow()
                        .iter()
                        .all(|a| a.used() == 0 && a.peak() == 0)
                );
                assert!(report.0.get().0.is_none());
                assert_eq!(report.1.get(), 0);
                assert_eq!(report.2.borrow().used(), 0);
                drop(rejected);
                assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
            }
        }
    }
}

#[test]
fn owned_static_build_quota_failure_keeps_the_policy_key_and_actual_row() {
    let contract = BindingResourceContract::ConsumerOwnedV2;
    let required = clinkz_wot_planning::ConsumerPropertyReadCleanupSlot::<ResolvedTargetArtifact>::required_capacity();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(ResourceKind::CleanupItemBytesMax, Some(required - 1)));
    let policy = owned_policy_with_limits(contract, limits);
    let key = policy.resource_identity();
    let parent = RefCell::new(policy.parent_accounts());
    let mut report = report_storage();
    let clock = Clock::default();
    let stats = Arc::new(Stats::default());
    let registration = binding::static_registration(StaticClient::new_owned(
        stats.clone(),
        outputs(),
        author(contract, Mode::Normal, pool(), false),
    ));
    let (mut r, o) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!("report fits; build root does not"));
    let source = source();
    observe(usize::MAX);
    let error = r
        .consume(&source, &mut work(100000), |_| panic!("rejected before TD"))
        .unwrap_err();
    assert_eq!(trace().requests, 0);
    disarm();
    assert_eq!(
        error.limit_details(),
        Some((
            ResourceKind::CleanupItemBytesMax,
            required - 1,
            Some(required),
            None
        ))
    );
    assert_eq!(
        error.context().resource_failure(),
        Some((key, ResourceFailureDomain::EngineCapacity, None, None))
    );
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    stop_static(&mut r);
    drop(r);
    drop(o);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[test]
fn owned_every_static_build_allocation_failure_reclaims_original_output_and_accounts() {
    let mut sites = 0;
    for fail in 0..256 {
        let contract = BindingResourceContract::ConsumerOwnedV2;
        let policy = owned_policy(contract);
        let key = policy.resource_identity();
        let parent = RefCell::new(policy.parent_accounts());
        let mut report = report_storage();
        let clock = Clock::default();
        let stats = Arc::new(Stats::default());
        let r = binding::static_registration(StaticClient::new_owned(
            stats.clone(),
            outputs(),
            author(contract, Mode::Normal, pool(), false),
        ));
        let (mut r, o) =
            StaticConsumerPropertyReadRuntime::try_new(r, policy, &clock, &parent, &mut report)
                .unwrap_or_else(|_| panic!());
        let source = source();
        observe(fail);
        let result = r.consume(&source, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!()
        });
        let requests = trace().requests;
        disarm();
        if fail < requests {
            assert!(result.is_err(), "allocation {fail}");
            assert_eq!(
                result.as_ref().err().unwrap().context().resource_failure(),
                Some((key, ResourceFailureDomain::DeploymentFailure, None, None)),
                "allocation {fail}"
            );
            sites += 1;
        }
        drop(result);
        stop_static(&mut r);
        drop(r);
        drop(o);
        assert_eq!(trace().live, 0);
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        assert!(parent.borrow().iter().all(|a| a.used() == 0));
        if fail >= requests {
            break;
        }
    }
    assert!(sites > 20);
    println!("owned static recoverable build allocation sites={sites}");
}
fn hex(key: BindingConfigurationDigest) -> String {
    key.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

// Cargo features unify per package, not per workspace. Servient's existing
// dev graph supplies a real std Core while this library is built no-default
// or async-no-std. The accepted contract requires matching compilation cells.
#[cfg(not(feature = "std"))]
#[test]
fn owned_mixed_core_and_servient_cells_must_reject_before_clock_and_parent_mutation() {
    let private = BindingConfigurationDigest::new(core::array::from_fn(|i| i as u8));
    let core = ConsumerBindingConfiguration::try_new(
        BindingResourceContract::ConsumerOwnedV2,
        64,
        private,
        64,
        32,
    )
    .unwrap();
    assert_eq!(
        hex(core.configuration()),
        "c10c25e570a3783d6a2f5e71c4fc417de20e74bc3896c207de2786f91952ed18",
        "precondition: this dependency graph compiled Core's std cell"
    );
    struct ObservedClock(Cell<usize>);
    impl RuntimeClock for ObservedClock {
        fn now(&self) -> MonotonicInstant {
            self.0.set(self.0.get() + 1);
            MonotonicInstant::new(ClockId::new(23), 10)
        }
        fn ticks_per_second(&self) -> core::num::NonZeroU64 {
            core::num::NonZeroU64::new(1000).unwrap()
        }
    }
    let contract = BindingResourceContract::ConsumerOwnedV2;
    let policy = owned_policy(contract);
    println!(
        "mixed-cell witness: Core=std; Servient={}; policy={}",
        if cfg!(feature = "async") {
            "async-no-std"
        } else {
            "no-default"
        },
        hex(policy.resource_identity())
    );
    let parent = RefCell::new(policy.parent_accounts());
    let mut report = report_storage();
    let clock = ObservedClock(Cell::new(0));
    let registration = binding::static_registration(StaticClient::new_owned(
        Arc::new(Stats::default()),
        outputs(),
        author(contract, Mode::Normal, pool(), false),
    ));
    let result = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    );
    let rejected = result.is_err();
    drop(result);
    assert!(
        rejected,
        "owned provisioning accepted Core std metadata under a portable Servient policy; clock observations={}",
        clock.0.get()
    );
    assert_eq!(clock.0.get(), 0);
    assert!(
        parent
            .borrow()
            .iter()
            .all(|a| a.used() == 0 && a.peak() == 0)
    );
}

#[test]
fn owned_complete_policy_vectors_and_every_independent_operand_are_frozen() {
    let contract = BindingResourceContract::ConsumerOwnedV2;
    observe(usize::MAX);
    let p = owned_policy(contract);
    assert_eq!(trace().requests, 0);
    disarm();
    let key = p.resource_identity();
    if cfg!(feature = "std") {
        assert_eq!(
            hex(key),
            "860ee2e2dddefee91fdd88e355dcc2ca45a0b29f542c0ce4b215246a25f00b73"
        );
    }
    assert_eq!(p.profile_origin(), ResourceProfileId::GATEWAY_DEFAULT_V1);
    assert_eq!(p.resource_limits(), GatewayDefaultV1::LIMITS);
    let r = [4, 4096];
    let c = [8, 1280];
    let input = [1, 64, 64, 64];
    let result = [64, 32];
    let work = [2, 2, 1, 1, 1, 1, 1];
    let time = [50, 5];
    for operand in 0..20 {
        let (mut r, mut c, mut input, mut result, mut work, mut time, mut scratch) =
            (r, c, input, result, work, time, 65536);
        match operand {
            0..=1 => r[operand] += 1,
            2..=3 => c[operand - 2] += 1,
            4..=7 => input[operand - 4] += 1,
            8..=9 => result[operand - 8] += 1,
            10..=16 => work[operand - 10] += 1,
            17..=18 => time[operand - 17] += 1,
            _ => scratch *= 2,
        }
        // Keep the input capacity operand independent but valid.
        if operand == 5 {
            input[1] -= 2;
        }
        let next = ConsumerPropertyReadPolicy::try_new_owned(
            contract,
            ResourceProfileId::GATEWAY_DEFAULT_V1,
            GatewayDefaultV1::LIMITS.clone(),
            BindingLifetimeFootprint::new(r[0] as u32, r[1]),
            BindingLifetimeFootprint::new(c[0] as u32, c[1]),
            input,
            result,
            work,
            time,
            scratch,
        )
        .unwrap();
        assert_ne!(next.resource_identity(), key, "operand {operand}");
        if cfg!(feature = "std") && operand == 19 {
            assert_eq!(
                hex(next.resource_identity()),
                "7a1a9c27659dea7959de5f6e5875b3f9fe526af37e18afa13f1c4dba85c1401e"
            );
        }
        if cfg!(feature = "std") && operand == 8 {
            assert_eq!(
                hex(next.resource_identity()),
                "08f43279aee42671cb9685c7e807ecf81302cc8b9d3c8f743808f9ff6d522cf4"
            );
        }
        if cfg!(feature = "std") && operand == 17 {
            assert_eq!(
                hex(next.resource_identity()),
                "de7c632500393402199644abf0de865c823626c55c2da123b643ebf2ea4fa682"
            );
        }
    }
    if cfg!(feature = "std") {
        assert_eq!(
            hex(owned_policy(BindingResourceContract::ConsumerOwnedHostV2).resource_identity()),
            "c51a34a8c9945d4c551e31a484ea208021d463b03fef34ab411a23b1724614ef"
        );
    }
}

#[test]
fn owned_static_real_planning_multiple_targets_source_destruction_and_zero_engine_progress_allocation()
 {
    let contract = BindingResourceContract::ConsumerOwnedV2;
    let pool = pool();
    let stats = Arc::new(Stats::default());
    let author = author(contract, Mode::Normal, pool.clone(), false);
    let key = author.configuration().configuration();
    let registration = binding::static_registration(StaticClient::new_owned(
        stats.clone(),
        outputs(),
        author.clone(),
    ));
    assert_eq!(registration.registration().identity().configuration(), key);
    let policy = owned_policy(contract);
    let parent = RefCell::new(policy.parent_accounts());
    let mut report = report_storage();
    let clock = Clock::default();
    observe(usize::MAX);
    let (mut runtime, observer) = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    )
    .unwrap_or_else(|_| panic!("owned static"));
    let provision = trace();
    disarm();
    let thing = source();
    observe(usize::MAX);
    runtime
        .consume(&thing, &mut work(100000), |b| {
            for _ in 0..100000 {
                b.step(&mut work(4096));
                if b.is_finished() {
                    return;
                }
            }
            panic!()
        })
        .unwrap();
    let build = trace();
    disarm();
    drop(thing);
    for (name, form) in [("a-read", 2), ("c-温度", 1)] {
        TRACE.with(|t| {
            let mut s = t.get();
            s.armed = true;
            s.requests = 0;
            s.layouts = [(0, 0); 256];
            s.largest = 0;
            s.peak = s.live;
            t.set(s);
        });
        let g = runtime
            .read_property(
                name,
                InteractionOptions::new().with_form_index(form),
                &mut work(100000),
            )
            .unwrap();
        assert_eq!(trace().requests, 0);
        runtime.step(&mut cx(), &mut work(100000));
        // Exactly one 8-byte private protocol growth allocation; Core/Servient
        // seals, progress and acknowledgement allocate zero.
        assert_eq!(trace().requests, 1);
        assert_eq!(trace().layouts[0], (8, 1));
        let out = runtime
            .try_take(g, &mut work(100000))
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            out.metadata().binding_response().unwrap().status_code(),
            599
        );
        assert_eq!(pool.used(), 8);
        assert_eq!(author.configuration().configuration(), key);
        disarm();
    }
    stop_static(&mut runtime);
    drop(runtime);
    drop(observer);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
    println!(
        "owned static provision: requests={} live={} peak={} largest={}; build: requests={} live={} peak={} largest={}; private growth=8; terminal={}",
        provision.requests,
        provision.live,
        provision.peak,
        provision.largest,
        build.requests,
        build.live,
        build.peak,
        build.largest,
        trace().live
    );
    assert_eq!(trace().live, 0);
}

fn excessive(kind: u8) -> [Option<InteractionOutput>; 8] {
    core::array::from_fn(|_| {
        let mut p = Payload::new(
            if kind == 0 { vec![0; 65] } else { vec![0; 2] },
            "application/json",
        );
        if kind == 1 {
            p.content_type = String::with_capacity(33);
            p.content_type.push_str("x");
        }
        if kind == 2 {
            p.content_coding = Some(String::with_capacity(33));
        }
        Some(InteractionOutput::with_data(p))
    })
}
#[test]
fn owned_static_normal_and_both_late_seals_reject_full_result_backing() {
    for mode in [Mode::Normal, Mode::LateStart, Mode::LatePoll] {
        for kind in 0..3 {
            let contract = BindingResourceContract::ConsumerOwnedV2;
            let stats = Arc::new(Stats::default());
            let policy = owned_policy(contract);
            let key = policy.resource_identity();
            let parent = RefCell::new(policy.parent_accounts());
            let mut report = report_storage();
            let clock = Clock::default();
            let registration = binding::static_registration(StaticClient::new_owned(
                stats.clone(),
                excessive(kind),
                author(contract, mode, pool(), false),
            ));
            let (mut r, o) = StaticConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                &clock,
                &parent,
                &mut report,
            )
            .unwrap_or_else(|_| panic!());
            publish_static(&mut r);
            let g = r
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            if mode != Mode::Normal {
                r.cancel(g).unwrap();
            }
            for _ in 0..4 {
                r.step(&mut cx(), &mut work(100000));
            }
            observe(usize::MAX);
            let error = r
                .try_take(g, &mut work(100000))
                .unwrap()
                .unwrap()
                .unwrap_err();
            let delivery = trace();
            disarm();
            assert_eq!(delivery.requests, 0, "{mode:?}/{kind}");
            assert!(matches!(error, CoreError::Validation(_)), "{mode:?}/{kind}");
            assert_eq!(error.context().cause_code(), Some(327), "{mode:?}/{kind}");
            assert_eq!(error.context().phase(), ErrorPhase::Validate);
            assert_eq!(error.context().retry_class(), RetryClass::Never);
            assert_eq!(
                error.context().resource_failure(),
                Some((key, ResourceFailureDomain::EngineCapacity, None, None)),
                "{mode:?}/{kind}"
            );
            assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
            stop_static(&mut r);
            drop(r);
            drop(o);
            assert!(parent.borrow().iter().all(|a| a.used() == 0));
        }
    }
}

#[test]
fn owned_static_two_roots_private_pressure_preserves_unread_and_late_owners_then_reuses() {
    let contract = BindingResourceContract::ConsumerOwnedV2;
    let pool = pool();
    let policy = owned_policy(contract);
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let (mut ar, mut br) = (report_storage(), report_storage());
    let a = author(contract, Mode::Normal, pool.clone(), false);
    let b = author(contract, Mode::LateStart, pool.clone(), false);
    assert_ne!(
        a.configuration().private_policy(),
        b.configuration().private_policy()
    );
    let make = |p: OwnedPolicy| {
        binding::static_registration(StaticClient::new_owned(
            Arc::new(Stats::default()),
            outputs(),
            p,
        ))
    };
    let (mut a, ao) =
        StaticConsumerPropertyReadRuntime::try_new(make(a), policy, &clock, &parent, &mut ar)
            .unwrap_or_else(|_| panic!());
    let (mut b, bo) = StaticConsumerPropertyReadRuntime::try_new(
        binding::ownership::second_registration(b, outputs()[0].take().unwrap()),
        owned_policy(contract),
        &clock,
        &parent,
        &mut br,
    )
    .unwrap_or_else(|_| panic!());
    publish_static(&mut a);
    let thing = source();
    b.consume(&thing, &mut work(100000), |build| {
        for _ in 0..100000 {
            build.step(&mut work(4096));
            if build.is_finished() {
                return;
            }
        }
        panic!()
    })
    .unwrap();
    drop(thing);
    let ga = a
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    a.step(&mut cx(), &mut work(100000));
    assert_eq!(pool.used(), 16);
    let gb = b
        .read_property("a-read", InteractionOptions::new(), &mut work(100000))
        .unwrap();
    let before: Vec<_> = parent.borrow().iter().map(|a| a.used()).collect();
    b.step(&mut cx(), &mut work(100000));
    assert_eq!(pool.refusals.load(Ordering::SeqCst), 1);
    assert!(b.try_take(gb, &mut work(100000)).unwrap().is_none());
    assert_eq!(
        before,
        parent.borrow().iter().map(|a| a.used()).collect::<Vec<_>>()
    );
    a.cancel(ga).unwrap();
    assert!(a.try_take(ga, &mut work(100000)).unwrap().unwrap().is_ok());
    assert_eq!(pool.used(), 8);
    b.step(&mut cx(), &mut work(100000));
    assert_eq!(pool.used(), 16);
    // The second root's original output, request and funded cleanup survived
    // refusal; no replacement call was constructed after capacity became free.
    stop_static(&mut a);
    drop(a);
    drop(ao);
    assert_eq!(pool.retired.load(Ordering::SeqCst), 0);
    assert_eq!(pool.used(), 16);
    assert!(b.try_take(gb, &mut work(100000)).unwrap().unwrap().is_ok());
    b.close();
    for _ in 0..10000 {
        if matches!(
            b.shutdown(&mut cx(), &mut work(4096)),
            StartStatus::Ready(_)
        ) {
            break;
        }
    }
    drop(b);
    drop(bo);
    assert_eq!(pool.used(), 8);
    let retired = pool.retired.clone();
    drop(pool);
    assert_eq!(retired.load(Ordering::SeqCst), 1);
    assert!(parent.borrow().iter().all(|a| a.used() == 0));
}

#[cfg(feature = "std")]
mod host_owned {
    use super::super::host::{build, shutdown};
    use super::*;
    use std::sync::Mutex;
    #[test]
    fn owned_host_complete_backing_measurements_and_exact_admission_boundaries() {
        let contract = BindingResourceContract::ConsumerOwnedV2;
        let mut required = 0;
        for limit in [0, 1, 2, 3] {
            let ceiling = match limit {
                0 => 0,
                1 => required - 1,
                2 => required,
                _ => required + 1,
            };
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(ResourceKind::HostBindingCallBytesPerItemMax, Some(ceiling)));
            let policy = ConsumerPropertyReadPolicy::try_new_owned(
                contract,
                ResourceProfileId::GATEWAY_DEFAULT_V1,
                limits,
                BindingLifetimeFootprint::new(4, 4096),
                BindingLifetimeFootprint::new(8, 1280),
                [1, 64, 64, 64],
                [64, 32],
                [2, 2, 1, 1, 1, 1, 1],
                [50, 5],
                65536,
            )
            .unwrap();
            let parent = Arc::new(Mutex::new(policy.parent_accounts()));
            let pool = pool();
            let registration = binding::host_registration(binding::HostClient::new_owned(
                Arc::new(Stats::default()),
                outputs(),
                author(contract, Mode::Normal, pool, false),
            ));
            let clock = Arc::new(Clock::default());
            observe(usize::MAX);
            let result =
                ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone());
            if limit < 2 {
                let error = result.err().expect("zero or one short");
                assert_eq!(trace().requests, 0);
                let (_, _, requested, _) = error.error().error().limit_details().unwrap();
                required = requested.unwrap();
                disarm();
                drop(error);
            } else {
                let (r, o) = result.unwrap_or_else(|_| panic!("exact owned bound"));
                let provision = trace();
                disarm();
                let h = build(&r);
                drop(h);
                shutdown(&r);
                drop(r);
                drop(o);
                assert_eq!(trace().live, 0);
                println!(
                    "owned Host call bound={required}; provisioning requests={} live={} peak={} largest={} layouts={:?}",
                    provision.requests,
                    provision.live,
                    provision.peak,
                    provision.largest,
                    &provision.layouts[..provision.requests]
                );
            }
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
    }
    #[test]
    fn owned_host_every_provision_and_build_allocation_failure_retains_exact_owner() {
        let contract = BindingResourceContract::ConsumerOwnedV2;
        for fail in 0..2 {
            let policy = owned_policy(contract);
            let parent = Arc::new(Mutex::new(policy.parent_accounts()));
            let clock: Arc<dyn RuntimeClock + Send + Sync> = Arc::new(Clock::default());
            let stats = Arc::new(Stats::default());
            let registration = binding::host_registration(binding::HostClient::new_owned(
                stats.clone(),
                outputs(),
                author(contract, Mode::Normal, pool(), false),
            ));
            let identity = registration.registration().identity();
            observe(fail);
            let result = ConsumerPropertyReadRuntime::try_new(
                registration,
                policy,
                clock.clone(),
                parent.clone(),
            );
            let rejected = result.err().expect("recoverable provisioning allocation");
            disarm();
            assert_eq!(rejected.input().0.registration().identity(), identity);
            assert!(Arc::ptr_eq(&rejected.input().2, &clock));
            assert!(Arc::ptr_eq(&rejected.input().3, &parent));
            assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
            assert_eq!(
                rejected
                    .error()
                    .error()
                    .context()
                    .resource_failure()
                    .unwrap()
                    .1,
                ResourceFailureDomain::DeploymentFailure
            );
            drop(rejected);
            assert_eq!(trace().live, 0);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
        let mut sites = 0;
        for fail in 0..256 {
            let (r, o, parent, _, stats) = setup(Mode::Normal, false, outputs());
            let key = owned_policy(contract).resource_identity();
            let source = source();
            observe(fail);
            let result = r.consume(&source, &mut work(100000), |b| {
                for _ in 0..100000 {
                    b.step(&mut work(4096));
                    if b.is_finished() {
                        return;
                    }
                }
                panic!()
            });
            let requests = trace().requests;
            disarm();
            if fail < requests {
                assert!(result.is_err(), "allocation {fail}");
                assert_eq!(
                    result.as_ref().err().unwrap().context().resource_failure(),
                    Some((key, ResourceFailureDomain::DeploymentFailure, None, None)),
                    "allocation {fail}"
                );
                sites += 1;
            }
            drop(result);
            shutdown(&r);
            drop(r);
            drop(o);
            assert_eq!(trace().live, 0);
            assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
            if fail >= requests {
                break;
            }
        }
        assert!(sites > 20);
        println!("owned Host recoverable provisioning=2, build={sites}, call=2");
    }
    fn setup(
        mode: Mode,
        dispose: bool,
        output: [Option<InteractionOutput>; 8],
    ) -> (
        ConsumerPropertyReadRuntime,
        impl Fn() -> (Option<CleanupOutcome>, [u64; 4]) + Send + Sync + 'static,
        Arc<Mutex<[ResourceAccount; 20]>>,
        Arc<PrivatePool>,
        Arc<Stats>,
    ) {
        let contract = if dispose {
            BindingResourceContract::ConsumerOwnedHostV2
        } else {
            BindingResourceContract::ConsumerOwnedV2
        };
        let pool = pool();
        let stats = Arc::new(Stats::default());
        let policy = owned_policy(contract);
        let parent = Arc::new(Mutex::new(policy.parent_accounts()));
        let r = binding::host_registration(binding::HostClient::new_owned(
            stats.clone(),
            output,
            author(contract, mode, pool.clone(), dispose),
        ));
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            r,
            policy,
            Arc::new(Clock::default()),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!("owned Host"));
        (runtime, observer, parent, pool, stats)
    }
    #[test]
    fn owned_host_normal_and_late_backing_faults_keep_the_original_raw_owner() {
        for mode in [Mode::Normal, Mode::LateStart, Mode::LatePoll] {
            for kind in 0..3 {
                let (r, o, parent, _, stats) = setup(mode, false, excessive(kind));
                let key = owned_policy(BindingResourceContract::ConsumerOwnedV2).resource_identity();
                let h = build(&r);
                let mut c = h
                    .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                    .unwrap();
                if mode != Mode::Normal {
                    c.cancel().unwrap();
                }
                for _ in 0..4 {
                    r.step(&mut cx(), &mut work(100000));
                }
                observe(usize::MAX);
                let error = c.try_take(&mut work(100000)).unwrap().unwrap().unwrap_err();
                let delivery = trace();
                disarm();
                assert_eq!(delivery.requests, 0, "{mode:?}/{kind}");
                assert!(matches!(error, CoreError::Validation(_)), "{mode:?}/{kind}");
                assert_eq!(error.context().cause_code(), Some(327));
                assert_eq!(error.context().phase(), ErrorPhase::Validate);
                assert_eq!(error.context().retry_class(), RetryClass::Never);
                assert_eq!(
                    error.context().resource_failure(),
                    Some((key, ResourceFailureDomain::EngineCapacity, None, None)),
                    "{mode:?}/{kind}"
                );
                assert_eq!(stats.states_destroyed.load(Ordering::SeqCst), 1);
                drop(c);
                drop(h);
                shutdown(&r);
                drop(r);
                drop(o);
                assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
            }
        }
    }
    #[test]
    fn owned_host_both_preacceptance_failures_return_exact_request_and_no_postacceptance_engine_allocation()
     {
        for fail in 0..=2 {
            let (r, o, parent, pool, stats) = setup(Mode::Normal, false, outputs());
            let binding = author(
                BindingResourceContract::ConsumerOwnedV2,
                Mode::Normal,
                pool.clone(),
                false,
            )
            .configuration();
            let policy_key =
                owned_policy(BindingResourceContract::ConsumerOwnedV2).resource_identity();
            let h = build(&r);
            let mut options = InteractionOptions::new();
            options.uri_variables.insert("key".into(), "value".into());
            let address = options.uri_variables.first_key_value().unwrap().0.as_ptr();
            observe(if fail < 2 { fail } else { usize::MAX });
            let result = h.read_property("a-read", options, &mut work(100000));
            if fail < 2 {
                let rejection = result.err().expect("both recoverable call allocations");
                assert_eq!(
                    rejection.error().error().context().resource_failure(),
                    Some((
                        if fail == 0 {
                            policy_key
                        } else {
                            binding.private_policy()
                        },
                        ResourceFailureDomain::DeploymentFailure,
                        None,
                        None,
                    ))
                );
                assert_eq!(
                    rejection.error().error().context().cause_code(),
                    (fail == 0).then_some(314)
                );
                assert_eq!(
                    rejection.error().error().context().phase(),
                    ErrorPhase::Binding
                );
                assert_eq!(
                    rejection.error().error().context().retry_class(),
                    RetryClass::Never
                );
                let ((options, request), _) = rejection.into_parts();
                let request = request.unwrap();
                assert!(options.uri_variables.is_empty());
                assert_eq!(
                    request
                        .uri_variables()
                        .first_key_value()
                        .unwrap()
                        .0
                        .as_ptr(),
                    address
                );
                assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
                assert_eq!(pool.used(), 8);
            } else {
                let mut c = result.unwrap();
                assert_eq!(trace().requests, 2);
                r.step(&mut cx(), &mut work(100000));
                assert_eq!(trace().requests, 3);
                assert_eq!(trace().layouts[2], (8, 1));
                assert!(c.try_take(&mut work(100000)).unwrap().unwrap().is_ok());
                assert_eq!(pool.used(), 8);
                drop(c);
            }
            disarm();
            drop(h);
            shutdown(&r);
            drop(r);
            drop(o);
            assert_eq!(trace().live, 0);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        }
    }
    #[test]
    fn owned_host_binding_private_rejection_keeps_its_key_and_request() {
        let (r, o, parent, pool, stats) = setup(Mode::Reject, false, outputs());
        let configuration = author(
            BindingResourceContract::ConsumerOwnedV2,
            Mode::Reject,
            pool,
            false,
        )
        .configuration();
        let h = build(&r);
        observe(usize::MAX);
        let rejected = h
            .read_property("a-read", InteractionOptions::new(), &mut work(100000))
            .err()
            .expect("actual binding policy refusal");
        assert_eq!(trace().requests, 1); // The Core envelope is released on rejection.
        assert_eq!(trace().live, 0);
        disarm();
        let context = rejected.error().error().context();
        assert_eq!(
            context.resource_failure(),
            Some((
                configuration.private_policy(),
                ResourceFailureDomain::BindingPolicy,
                None,
                None
            ))
        );
        let request = rejected.input().1.as_ref().unwrap();
        assert_eq!(
            request.artifact().identity().configuration(),
            configuration.configuration()
        );
        assert_eq!(
            context.binding(),
            Some((request.binding_id(), request.binding_generation()))
        );
        assert_eq!(context.plan(), Some(request.plan_id()));
        assert_eq!(context.phase(), ErrorPhase::Binding);
        assert_eq!(context.retry_class(), RetryClass::Never);
        assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
        drop(rejected);
        drop(h);
        shutdown(&r);
        drop(r);
        drop(o);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
    #[test]
    fn owned_host_residual_and_engine_funding_precede_finite_allocating_disposal_and_supported_error()
     {
        assert!(
            OwnedPolicy::try_new(
                BindingResourceContract::ConsumerOwnedV2,
                Mode::Pending,
                pool(),
                true,
                64,
                64,
                32
            )
            .is_err()
        );
        for fail in [false, true] {
            let (r, o, parent, pool, stats) = setup(Mode::Pending, true, outputs());
            let o = Arc::new(o);
            let h = build(&r);
            let c = h
                .read_property("a-read", InteractionOptions::new(), &mut work(100000))
                .unwrap();
            r.step(&mut cx(), &mut work(100000));
            assert_eq!(pool.used(), 16);
            let before = (
                stats.polls.load(Ordering::SeqCst),
                stats.cancels.load(Ordering::SeqCst),
            );
            let seen = Arc::new(core::sync::atomic::AtomicBool::new(true));
            let check = seen.clone();
            let observer = o.clone();
            let bank = parent.clone();
            DROP_CHECK.with(|d| {
                *d.borrow_mut() = Some(Arc::new(move || {
                    let funded = bank
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|a| a.kind() == ResourceKind::HostBindingCallBytesGlobalMax)
                        .unwrap()
                        .used()
                        > 0;
                    check.fetch_and(
                        observer().0.is_some_and(|o| o.is_residual()) && funded,
                        Ordering::SeqCst,
                    );
                }))
            });
            observe(if fail { 0 } else { usize::MAX });
            drop(c);
            drop(h);
            drop(r);
            let end = trace();
            disarm();
            DROP_CHECK.with(|d| *d.borrow_mut() = None);
            assert_eq!(end.requests, 1);
            assert_eq!(end.layouts[0], (8, 1));
            assert!(seen.load(Ordering::SeqCst));
            assert_eq!(
                before,
                (
                    stats.polls.load(Ordering::SeqCst),
                    stats.cancels.load(Ordering::SeqCst)
                )
            );
            assert!(o().0.unwrap().is_residual());
            assert_eq!(pool.disposal_requests.load(Ordering::SeqCst), 1);
            assert_eq!(
                pool.disposal_errors.load(Ordering::SeqCst),
                usize::from(fail)
            );
            assert_eq!(pool.used(), 8);
            drop(o);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
            assert_eq!(trace().live, 0);
        }
    }
}
