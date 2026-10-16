use super::*;

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
    assert_eq!(core.compilation_cell(), "std");
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
            self.0.set(self.0.get() + 1);
            core::num::NonZeroU64::new(1000).unwrap()
        }
    }
    let contract = BindingResourceContract::ConsumerOwnedV2;
    let policy = owned_policy(contract);
    let key = policy.resource_identity();
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
    let stats = Arc::new(Stats::default());
    let registration = binding::static_registration(StaticClient::new_owned(
        stats.clone(),
        outputs(),
        author(contract, Mode::Normal, pool(), false),
    ));
    let identity = registration.registration().identity();
    let captured = registration.support().consumer_configuration().unwrap();
    assert_eq!(captured.compilation_cell(), "std");
    assert_eq!(captured.configuration(), identity.configuration());
    let report_before = report.0.get();
    observe(usize::MAX);
    let result = StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    );
    let allocations = trace().requests;
    disarm();
    let rejected = result.is_err();
    assert!(
        rejected,
        "owned provisioning accepted Core std metadata under a portable Servient policy; clock observations={}",
        clock.0.get()
    );
    let rejection = result.err().unwrap();
    assert_eq!(rejection.input().0.registration().identity(), identity);
    assert_eq!(
        rejection.error().error().context().resource_failure(),
        Some((key, ResourceFailureDomain::EngineCapacity, None, None))
    );
    assert_eq!(allocations, 0);
    assert_eq!(clock.0.get(), 0);
    for counter in [
        &stats.starts,
        &stats.polls,
        &stats.cancels,
        &stats.acknowledgements,
    ] {
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
    drop(rejection);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(report.0.get(), report_before);
    assert!(
        parent
            .borrow()
            .iter()
            .all(|a| a.used() == 0 && a.peak() == 0)
    );
}
