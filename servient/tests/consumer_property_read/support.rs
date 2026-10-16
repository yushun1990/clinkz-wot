use clinkz_wot_consumer_property_read_binding_fixture as binding;
use clinkz_wot_consumer_property_read_binding_fixture::{Clock, Mode, StaticClient, Stats};
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
use clinkz_wot_servient::{CompiledPlanSetState, consumer_property_read::*};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
    sync::{Arc, atomic::Ordering},
    task::{Context, Waker},
};

#[derive(Clone, Copy, Default, Debug)]
struct Block {
    address: usize,
    bytes: usize,
    align: usize,
}
#[derive(Clone, Copy, Debug)]
struct Trace {
    armed: bool,
    fail: usize,
    requests: usize,
    live: usize,
    peak: usize,
    largest: usize,
    releases: usize,
    records: [Block; 256],
    layouts: [(usize, usize); 256],
}
const EMPTY: Trace = Trace {
    armed: false,
    fail: usize::MAX,
    requests: 0,
    live: 0,
    peak: 0,
    largest: 0,
    releases: 0,
    records: [Block {
        address: 0,
        bytes: 0,
        align: 0,
    }; 256],
    layouts: [(0, 0); 256],
};
std::thread_local! { static TRACE: Cell<Trace> = const { Cell::new(EMPTY) }; }
std::thread_local! { static DROP_CHECK: RefCell<Option<Arc<dyn Fn()>>> = const { RefCell::new(None) }; }
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let fail = TRACE
            .try_with(|t| {
                let mut s = t.get();
                if !s.armed {
                    return false;
                }
                let index = s.requests;
                s.requests += 1;
                if index < s.layouts.len() {
                    s.layouts[index] = (layout.size(), layout.align());
                }
                s.largest = s.largest.max(layout.size());
                t.set(s);
                index == s.fail
            })
            .unwrap_or(false);
        if fail {
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let _ = TRACE.try_with(|t| {
                let mut s = t.get();
                if s.armed {
                    let index = s
                        .records
                        .iter()
                        .position(|b| b.address == 0)
                        .expect("fixed trace capacity");
                    s.records[index] = Block {
                        address: p as usize,
                        bytes: layout.size(),
                        align: layout.align(),
                    };
                    s.live += layout.size();
                    s.peak = s.peak.max(s.live);
                    t.set(s);
                }
            });
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        let tracked = TRACE
            .try_with(|t| t.get().records.iter().any(|b| b.address == p as usize))
            .unwrap_or(false);
        if tracked {
            let _ = DROP_CHECK.try_with(|check| {
                if let Some(check) = &*check.borrow() {
                    check();
                }
            });
        }
        let _ = TRACE.try_with(|t| {
            let mut s = t.get();
            if let Some(i) = s.records.iter().position(|b| b.address == p as usize) {
                assert_eq!(
                    (s.records[i].bytes, s.records[i].align),
                    (layout.size(), layout.align())
                );
                s.live -= layout.size();
                s.releases += 1;
                s.records[i] = Block::default();
                t.set(s);
            }
        });
        unsafe { System.dealloc(p, layout) }
    }
}
fn observe(fail: usize) {
    TRACE.with(|t| {
        assert_eq!(t.get().live, 0);
        t.set(Trace {
            armed: true,
            fail,
            ..EMPTY
        });
    });
}
fn trace() -> Trace {
    TRACE.with(Cell::get)
}
fn disarm() {
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = false;
        t.set(s);
    });
}
// Start another allocation window without losing already live physical owners.
fn resume_observing() {
    TRACE.with(|t| {
        let mut s = t.get();
        s.armed = true;
        s.requests = 0;
        s.releases = 0;
        s.layouts = [(0, 0); 256];
        s.largest = 0;
        s.peak = s.live;
        t.set(s);
    });
}
fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, n))
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn policy() -> ConsumerPropertyReadPolicy {
    policy_with(GatewayDefaultV1::LIMITS.clone())
}
fn policy_with(limits: ResourceLimits) -> ConsumerPropertyReadPolicy {
    ConsumerPropertyReadPolicy::try_new(
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
fn publish_static(runtime: &mut StaticConsumerPropertyReadRuntime<'_, StaticClient>) {
    let thing = source();
    assert!(matches!(
        runtime
            .consume(&thing, &mut work(100000), |build| {
                for _ in 0..100000 {
                    build.step(&mut work(4096));
                    if build.is_finished() {
                        return;
                    }
                }
                panic!("bounded fixture build stalled");
            })
            .unwrap(),
        StartStatus::Ready(())
    ));
}
fn policy_with_cleanup_costs(limits: ResourceLimits) -> ConsumerPropertyReadPolicy {
    // A more conservative whole-target destructor declaration makes cancel
    // plus release exceed a valid single-step quantum. Registration retirement
    // still fits, so this tests deferral rather than startup rejection alone.
    ConsumerPropertyReadPolicy::try_new(
        limits,
        BindingLifetimeFootprint::new(1, 4096),
        BindingLifetimeFootprint::new(8, 1280),
        [1, 64, 64, 64],
        [64, 32],
        [2, 2, 1, 1, 1, 4, 1],
        [50, 5],
        65536,
    )
    .unwrap()
}
fn rejected_static_startup(kind: ResourceKind, limit: u64) -> u64 {
    rejected_static_startup_for(kind, limit, BindingResourceContract::StrictV1)
}
fn rejected_static_startup_for(
    kind: ResourceKind,
    limit: u64,
    contract: BindingResourceContract,
) -> u64 {
    let stats = Arc::new(Stats::default());
    let registration =
        binding::static_registration(if contract == BindingResourceContract::StrictV1 {
            StaticClient::new(stats.clone(), outputs())
        } else {
            StaticClient::new_owned(
                stats.clone(),
                outputs(),
                author(contract, Mode::Normal, pool(), false),
            )
        });
    let identity = registration.registration().identity();
    let mut limits = GatewayDefaultV1::LIMITS.clone();
    assert!(limits.set(kind, Some(limit)));
    let policy = if contract == BindingResourceContract::StrictV1 {
        policy_with(limits)
    } else {
        owned_policy_with_limits(contract, limits)
    };
    let key = policy.resource_identity();
    let parent = RefCell::new(policy.parent_accounts());
    let clock = Clock::default();
    let mut report = report_storage();
    observe(0);
    let rejection = match StaticConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        &clock,
        &parent,
        &mut report,
    ) {
        Err(rejection) => rejection,
        Ok(_) => panic!("unsupported static startup admitted"),
    };
    assert_eq!(trace().requests, 0);
    disarm();
    let (inputs, error) = rejection.into_parts();
    if contract != BindingResourceContract::StrictV1 {
        let failure = error.error().context().resource_failure().unwrap();
        assert_eq!(
            (failure.0, failure.1),
            (key, ResourceFailureDomain::EngineCapacity)
        );
    }
    let requested = match error.error() {
        CoreError::LimitExceeded {
            resource,
            requested: Some(n),
            ..
        } => {
            assert_eq!(*resource, kind);
            *n
        }
        error => panic!("wrong static rejection: {error:?}"),
    };
    assert_eq!(inputs.0.registration().identity(), identity);
    assert!(
        parent
            .borrow()
            .iter()
            .all(|a| a.used() == 0 && a.peak() == 0)
    );
    assert_eq!(report.0.get(), (None, [0; 4]));
    assert_eq!(report.1.get(), 0);
    assert_eq!(report.2.borrow().used(), 0);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 0);
    assert_eq!(stats.starts.load(Ordering::SeqCst), 0);
    drop(inputs);
    assert_eq!(stats.clients_destroyed.load(Ordering::SeqCst), 1);
    requested
}

fn source() -> clinkz_wot_td::thing::Thing {
    serde_json::from_str(r#"{
 "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"aggregate","id":"urn:aggregate:id","base":"https://peer.test/a/",
 "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
 "properties":{
  "a-read":{"forms":[{"href":"skip","op":[]},{"href":"../first","scopes":["二",""]},{"href":"second","contentCoding":"gzip"}]},
  "b-empty":{"forms":[{"href":"skip","op":[]}]},
  "c-温度":{"forms":[{"href":"skip","op":["writeproperty"]},{"href":"last","scopes":["three"]}]},
  "z-empty":{"forms":[]}
 }}"#).unwrap()
}
fn outputs() -> [Option<InteractionOutput>; 8] {
    core::array::from_fn(|_| {
        Some(InteractionOutput::with_data(Payload::new(
            &b"17"[..],
            "application/json",
        )))
    })
}
type ReportStorage = (
    Cell<(Option<CleanupOutcome>, [u64; 4])>,
    Cell<u8>,
    RefCell<ResourceAccount>,
);
fn report_storage() -> ReportStorage {
    (
        Cell::new((None, [0; 4])),
        Cell::new(0),
        RefCell::new(ResourceAccount::new(
            SlotIndex::new(50),
            Generation::INITIAL,
            ResourceKind::CleanupBytesMax,
            8192,
        )),
    )
}
fn category(result: CoreResult<InteractionOutput>) -> &'static str {
    match result {
        Ok(o) => {
            assert_eq!(o.data().unwrap().body.as_ref(), b"17");
            assert_eq!(o.metadata().binding_response().unwrap().status_code(), 599);
            "ok"
        }
        Err(CoreError::Validation(_)) => "validation",
        Err(CoreError::Binding(_)) => "binding",
        Err(CoreError::Cancelled(_)) => "cancelled",
        Err(CoreError::TimedOut(_)) => "deadline",
        Err(e) => panic!("unexpected result {e:?}"),
    }
}
const CASE_TABLE_VERSION: u32 = 1;
const CASES: [(Mode, bool, bool, &str); 9] = [
    (Mode::Normal, false, false, "ok"),
    (Mode::Error, false, false, "binding"),
    (Mode::InvalidIdentity, false, false, "validation"),
    (Mode::InvalidShape, false, false, "validation"),
    (Mode::LateStart, true, false, "ok"),
    (Mode::LatePoll, true, false, "ok"),
    (Mode::Complete, true, false, "cancelled"),
    (Mode::Residual, true, false, "cancelled"),
    (Mode::CancelError, true, true, "binding"),
];

use clinkz_wot_consumer_property_read_binding_fixture::ownership::{OwnedPolicy, PrivatePool};
fn owned_policy(contract: BindingResourceContract) -> ConsumerPropertyReadPolicy {
    owned_policy_with_limits(contract, GatewayDefaultV1::LIMITS.clone())
}
fn owned_policy_with_limits(
    contract: BindingResourceContract,
    limits: ResourceLimits,
) -> ConsumerPropertyReadPolicy {
    try_owned_policy_with_limits(contract, limits).unwrap()
}
fn try_owned_policy_with_limits(
    contract: BindingResourceContract,
    limits: ResourceLimits,
) -> CoreResult<ConsumerPropertyReadPolicy> {
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

fn hex(key: BindingConfigurationDigest) -> String {
    key.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn owned_engine_local_boundaries(
    representation: &str,
    run: impl Fn(ResourceLimits) -> CoreResult<()>,
) {
    for kind in [
        ResourceKind::AdmissionTemporaryBytesPerOperationMax,
        ResourceKind::PeakLiveBytesPerAdmissionMax,
        ResourceKind::LargestContiguousAllocationBytesMax,
        ResourceKind::CompiledRuntimeBytesPerThingMax,
    ] {
        let probe = |limit| {
            let mut limits = GatewayDefaultV1::LIMITS.clone();
            assert!(limits.set(kind, Some(limit)));
            run(limits)
        };
        let (mut low, mut high) = (0, GatewayDefaultV1::LIMITS.get(kind).unwrap());
        probe(high).unwrap();
        // Find the boundary from actual constructors, build and call execution;
        // no duplicate accounting formula or relaxed production budget is used.
        while low < high {
            let mid = low + (high - low) / 2;
            if probe(mid).is_ok() {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        assert!(low > 0);
        for limit in [0, low - 1] {
            assert!(probe(limit).is_err());
        }
        for limit in [low, low + 1] {
            probe(limit).unwrap();
        }
        println!("owned {representation} local {kind:?} exact={low}");
    }
}
