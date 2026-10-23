#![cfg(feature = "prepare")]
//! Actual OS effects through the funded production owner, without a Tokio
//! runtime, dummy Consumer, stages or test-local lifecycle implementation.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{
    ProducerPropertyReadAdmission, ProducerPropertyReadStep, producer_execution::*,
};
use clinkz_wot_servient::producer_installation::*;
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    time::Duration,
};

// These cases observe process descriptor totals, so serialize this test binary.
static SERIAL: Mutex<()> = Mutex::new(());
struct AllocationObserver;
std::thread_local! {
    static TRACE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MAX: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FAIL_SIZE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static OBSERVE_FREE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FUNDED_AT_FREE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static OBSERVED_BANK: std::cell::RefCell<Option<Arc<ProducerInstallationAccounts>>> = const { std::cell::RefCell::new(None) };
}
// SAFETY: forwards the exact Layout/pointer to System; only an explicitly
// selected allocation on this test thread is refused. Observation allocates
// nothing and never touches the memory being allocated/freed.
unsafe impl std::alloc::GlobalAlloc for AllocationObserver {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if TRACE.try_with(|t| t.get()).unwrap_or(false) {
            COUNT.with(|c| c.set(c.get() + 1));
            MAX.with(|m| m.set(m.get().max(layout.size())));
        }
        if FAIL_SIZE
            .try_with(|f| {
                if f.get() != 0 && f.get() == layout.size() {
                    f.set(0);
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
        {
            return core::ptr::null_mut();
        }
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        if OBSERVE_FREE
            .try_with(|o| o.get() == layout.size() && o.get() != 0)
            .unwrap_or(false)
        {
            let funded = OBSERVED_BANK.with(|b| {
                b.borrow()
                    .as_ref()
                    .unwrap()
                    .snapshot(INSTALLATION_ACCOUNT_KINDS)
                    .iter()
                    .any(|a| {
                        a.kind() == ResourceKind::BindingSlotStateBytesGlobalMax
                            && a.used() >= layout.size() as u64
                    })
            });
            FUNDED_AT_FREE.with(|f| f.set(funded));
        }
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationObserver = AllocationObserver;

fn accepted<T, I>(r: Result<T, BindingInputRejection<I>>) -> T {
    r.unwrap_or_else(|r| panic!("{:?}", r.error()))
}
fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, n))
}
fn slot(generation: Generation) -> ThingSlotId {
    ThingSlotId::new(SlotIndex::new(44), generation)
}
fn document(address: SocketAddr) -> Value {
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"installed", "securityDefinitions":{"n":{"scheme":"nosec"}},"security":["n"],"properties":{"":{"type":"number","readOnly":true,"forms":[{"href":format!("http://{address}/value")}]}}})
}
struct Handler;
impl ReadPropertyHandler for Handler {
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        panic!("closed installation dispatched a handler")
    }
}
fn prepare(
    address: SocketAddr,
    generation: Generation,
) -> (
    PreparedProducer<HttpProducerExecutor, Handler>,
    ProducerInstanceAuthority,
) {
    let authority = ProducerInstanceAuthority::try_new(slot(generation)).unwrap();
    let td: Thing = serde_json::from_value(document(address)).unwrap();
    let config = ValidatedThingAdmissionConfig::try_from_limits(&GatewayDefaultV1::LIMITS).unwrap();
    let mut c = ValidatedThingCursor::from_thing(
        &td,
        &config,
        AdmissionLedger::new(SlotIndex::new(44), generation, 0, 8 << 20, 0, 0, 0, 0),
    );
    let proof = loop {
        c = match c.step(&mut work(1_000_000), false) {
            ValidatedThingProgress::Pending(c) => c,
            ValidatedThingProgress::Complete(p) => break p,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        };
    };
    let mut c = ProducerPropertyReadAdmission::new(proof, "");
    while c.step(&mut work(1_000_000), false).unwrap() != ProducerPropertyReadStep::Covered {}
    let config = HttpProducerConfig::for_loopback(address).unwrap();
    let pair = accepted(try_producer_registration(
        config,
        BindingId::new(601),
        BindingGeneration::INITIAL,
        0,
    ));
    let prepared = accepted(pair.prepare_property_read(
        &mut c,
        &authority,
        ProducerHandlerRegistration::new(
            &authority,
            "".into(),
            Handler,
            HandlerFootprint::new(0, 0, 0),
        ),
        &mut work(1_000_000),
    ));
    (prepared, authority)
    // Both the TD and its continuation are destroyed on return.
}
type Runtime = ProducerInstallationRuntime<HttpProducerExecutor, Handler>;
fn address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn bank() -> Arc<ProducerInstallationAccounts> {
    Arc::new(producer_installation_accounts(&GatewayDefaultV1::LIMITS))
}
fn runtime(
    address: SocketAddr,
    generation: Generation,
    bank: &Arc<ProducerInstallationAccounts>,
) -> Runtime {
    let (prepared, authority) = prepare(address, generation);
    accepted(Runtime::try_new(
        prepared,
        authority,
        GatewayDefaultV1::LIMITS.clone(),
        bank.clone(),
        Deadline::NONE,
    ))
}
fn step(r: &mut Runtime, n: u64) -> Poll<ProducerInstallationState> {
    r.step(
        &mut Context::from_waker(Waker::noop()),
        &mut work(n),
        MonotonicInstant::new(ClockId::new(7), 0),
    )
}
fn installed(r: &mut Runtime) {
    assert_eq!(
        step(r, 1),
        Poll::Ready(ProducerInstallationState::Installing)
    );
    assert_eq!(
        step(r, 1),
        Poll::Ready(ProducerInstallationState::InstalledClosed)
    );
}
fn free(address: SocketAddr) {
    drop(TcpListener::bind(address).expect("native listener was retained"));
}
fn empty(b: &Arc<ProducerInstallationAccounts>) {
    assert!(
        b.snapshot(INSTALLATION_ACCOUNT_KINDS)
            .iter()
            .all(|a| a.used() == 0)
    );
}
#[test]
fn real_listener_stays_unpublished_then_paid_close_and_last_owner_drop_retire_it() {
    let _serial = SERIAL.lock().unwrap();
    for close in [true, false] {
        let address = address();
        let b = bank();
        let mut r = runtime(address, Generation::INITIAL, &b);
        assert!(step(&mut r, 0).is_pending());
        free(address);
        assert_eq!(
            step(&mut r, 1),
            Poll::Ready(ProducerInstallationState::Installing)
        );
        free(address);
        assert!(step(&mut r, 0).is_pending());
        free(address);
        assert_eq!(
            step(&mut r, 1),
            Poll::Ready(ProducerInstallationState::InstalledClosed)
        );
        assert_eq!(
            TcpListener::bind(address).unwrap_err().kind(),
            std::io::ErrorKind::AddrInUse
        );
        let mut peer = TcpStream::connect(address).unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(25)))
            .unwrap();
        peer.write_all(format!("GET /value HTTP/1.1\r\nHost: {address}\r\n\r\n").as_bytes())
            .unwrap();
        let mut byte = [0];
        assert!(matches!(
            peer.read(&mut byte).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ));
        if close {
            r.cancel();
            assert!(step(&mut r, 0).is_pending());
            assert!(TcpListener::bind(address).is_err());
            assert_eq!(
                step(&mut r, 1),
                Poll::Ready(ProducerInstallationState::Closed)
            );
        }
        drop(r);
        empty(&b);
        free(address);
        assert!(
            matches!(peer.read(&mut byte), Ok(0) | Err(_)),
            "root loss retained a live peer"
        );
    }
}
#[test]
fn occupied_port_is_a_scoped_deployment_failure_and_original_owner_drains() {
    let _serial = SERIAL.lock().unwrap();
    let incumbent = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = incumbent.local_addr().unwrap();
    let b = bank();
    let mut r = runtime(address, Generation::INITIAL, &b);
    let _ = step(&mut r, 1);
    assert_eq!(
        step(&mut r, 1),
        Poll::Ready(ProducerInstallationState::Closing)
    );
    let cause = r.first_cause().unwrap().clone();
    assert_eq!(
        cause.context().resource_failure().unwrap().1,
        ResourceFailureDomain::DeploymentFailure
    );
    assert_eq!(cause.context().thing(), Some(slot(Generation::INITIAL)));
    assert_eq!(
        cause.context().binding(),
        Some((BindingId::new(601), BindingGeneration::INITIAL))
    );
    r.cancel();
    assert_eq!(r.first_cause(), Some(&cause));
    let _ = step(&mut r, 1);
    drop(r);
    empty(&b);
    drop(incumbent);
    free(address);
    let mut fresh = runtime(address, Generation::INITIAL.checked_next().unwrap(), &b);
    installed(&mut fresh);
    drop(fresh);
    empty(&b);
    free(address);
}
#[test]
fn parent_capacity_refusal_has_zero_native_effects_and_preserves_source_free_input() {
    let _serial = SERIAL.lock().unwrap();
    let address = address();
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(ResourceKind::BindingRoutesGlobalMax, Some(1))
        .unwrap();
    let b = Arc::new(producer_installation_accounts(&limits));
    let sibling = runtime(address, Generation::INITIAL.checked_next().unwrap(), &b);
    let before = b.snapshot(INSTALLATION_ACCOUNT_KINDS).map(|a| a.used());
    let (prepared, authority) = prepare(address, Generation::INITIAL);
    let refused = Runtime::try_new(
        prepared,
        authority,
        GatewayDefaultV1::LIMITS.clone(),
        b.clone(),
        Deadline::NONE,
    )
    .err()
    .unwrap();
    assert_eq!(
        b.snapshot(INSTALLATION_ACCOUNT_KINDS).map(|a| a.used()),
        before
    );
    let ((prepared, authority, limits, original), _) = refused.into_parts();
    assert!(Arc::ptr_eq(&b, &original));
    assert_eq!(prepared.entry().program().listen_address(), address);
    free(address);
    drop(sibling);
    empty(&b);
    let mut r = accepted(Runtime::try_new(
        prepared,
        authority,
        limits,
        original,
        Deadline::NONE,
    ));
    installed(&mut r);
    drop(r);
    empty(&b);
    free(address);
}
#[test]
fn repeated_distinct_generations_release_listeners_descriptors_and_original_accounts() {
    let _serial = SERIAL.lock().unwrap();
    let address = address();
    let b = bank();
    let mut generation = Generation::INITIAL;
    #[cfg(target_os = "linux")]
    let baseline = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for _ in 0..100 {
        let mut r = runtime(address, generation, &b);
        installed(&mut r);
        assert_eq!(r.entry().unwrap().instance(), slot(generation));
        drop(r);
        empty(&b);
        free(address);
        generation = generation.checked_next().unwrap();
    }
    #[cfg(target_os = "linux")]
    assert_eq!(
        std::fs::read_dir("/proc/self/fd").unwrap().count(),
        baseline
    );
}
#[test]
fn compiler_configuration_and_producer_policy_identity_are_checked_without_io() {
    let _serial = SERIAL.lock().unwrap();
    let held = TcpListener::bind("127.0.0.1:0").unwrap();
    let a = HttpProducerConfig::for_loopback(held.local_addr().unwrap()).unwrap();
    let b = HttpProducerConfig::for_loopback(address()).unwrap();
    drop(held);
    let e = producer_execution(b, BindingId::new(601), BindingGeneration::INITIAL, 0);
    let identity = e.identity();
    let refused = ProducerBindingPreparation::try_new(a, e, a).err().unwrap();
    let (c, e, snapshot) = refused.into_input();
    assert_eq!(c, a);
    assert_eq!(snapshot, a);
    assert!(e.configuration_matches(&b));
    assert_ne!(
        producer_execution(a, BindingId::new(601), BindingGeneration::INITIAL, 0)
            .identity()
            .configuration(),
        identity.configuration()
    );
    free(a.listen_address());
    free(b.listen_address());
    let (prepared, _authority) = prepare(a.listen_address(), Generation::INITIAL);
    assert!(
        e.check_program(prepared.entry().program()).is_err(),
        "same native type with different actual listen operands"
    );
}

#[test]
fn measured_owner_layout_exact_limits_allocation_refusal_and_deallocation_funding() {
    let _serial = SERIAL.lock().unwrap();
    let address = address();
    let b = bank();
    let (prepared, authority) = prepare(address, Generation::INITIAL);
    COUNT.with(|n| n.set(0));
    MAX.with(|n| n.set(0));
    TRACE.with(|n| n.set(true));
    let result = Runtime::try_new(
        prepared,
        authority,
        GatewayDefaultV1::LIMITS.clone(),
        b.clone(),
        Deadline::NONE,
    );
    TRACE.with(|n| n.set(false));
    let r = accepted(result);
    let block = MAX.with(|n| n.get());
    eprintln!(
        "Producer owner allocation: {block} bytes; inline runtime: {} bytes",
        std::mem::size_of::<Runtime>()
    );
    assert_eq!(
        COUNT.with(|n| n.get()),
        1,
        "admission must allocate only the one fallible controlled owner"
    );
    {
        let bank = b.snapshot(INSTALLATION_ACCOUNT_KINDS);
        let extra =
            2 * std::mem::size_of::<usize>() as u64 + consumer_result_backing(4096, 64).unwrap();
        assert_eq!(bank[4].used(), block as u64 + extra);
        assert_eq!(
            bank[0].used(),
            (block + std::mem::size_of::<Runtime>()) as u64 + extra + 1024
        );
        assert_eq!(
            bank[1].used(),
            (block + std::mem::size_of::<Runtime>()) as u64 + extra
        );
    }
    OBSERVED_BANK.with(|p| *p.borrow_mut() = Some(b.clone()));
    OBSERVE_FREE.with(|s| s.set(block));
    drop(r);
    assert!(
        FUNDED_AT_FREE.with(|f| f.get()),
        "original account released before the physical owner block"
    );
    OBSERVE_FREE.with(|s| s.set(0));
    OBSERVED_BANK.with(|p| p.borrow_mut().take());
    empty(&b);
    let largest = block.max(4096 + 2 * std::mem::size_of::<usize>());
    for ceiling in [largest, largest - 1] {
        let (prepared, authority) = prepare(address, Generation::INITIAL);
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .try_with_limit(
                ResourceKind::LargestContiguousAllocationBytesMax,
                Some(ceiling as u64),
            )
            .unwrap();
        let result = Runtime::try_new(prepared, authority, limits, b.clone(), Deadline::NONE);
        assert_eq!(result.is_ok(), ceiling == largest);
        drop(result);
        empty(&b);
    }
    let (prepared, authority) = prepare(address, Generation::INITIAL);
    FAIL_SIZE.with(|s| s.set(block));
    let refused = Runtime::try_new(
        prepared,
        authority,
        GatewayDefaultV1::LIMITS.clone(),
        b.clone(),
        Deadline::NONE,
    )
    .err()
    .expect("fallible owner allocation");
    assert_eq!(FAIL_SIZE.with(|s| s.get()), 0);
    assert!(matches!(
        refused.error().error(),
        CoreError::Backpressure(_)
    ));
    empty(&b);
    free(address);
    let ((prepared, authority, limits, original), _) = refused.into_parts();
    let mut recovered = accepted(Runtime::try_new(
        prepared,
        authority,
        limits,
        original,
        Deadline::NONE,
    ));
    installed(&mut recovered);
    drop(recovered);
    empty(&b);
    free(address);
}
#[cfg(target_os = "linux")]
#[test]
fn fd_pressure_remains_a_deployment_failure_and_releases_the_same_owner() {
    let _serial = SERIAL.lock().unwrap();
    const CHILD: &str = "CLINKZ_HTTP_PRODUCER_INSTALL_FD_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new("bash")
            .args(["-c", "ulimit -n 128 && exec \"$@\"", "producer-fd-pressure"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "fd_pressure_remains_a_deployment_failure_and_releases_the_same_owner",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let address = address();
    let b = bank();
    let mut r = runtime(address, Generation::INITIAL, &b);
    let _ = step(&mut r, 1);
    let mut files = Vec::new();
    loop {
        match std::fs::File::open("/dev/null") {
            Ok(f) => files.push(f),
            Err(e) => {
                assert_eq!(e.raw_os_error(), Some(24));
                break;
            }
        }
    }
    let _ = step(&mut r, 1);
    assert_eq!(r.state(), ProducerInstallationState::Closing);
    assert_eq!(
        r.first_cause()
            .unwrap()
            .context()
            .resource_failure()
            .unwrap()
            .1,
        ResourceFailureDomain::DeploymentFailure
    );
    let _ = step(&mut r, 1);
    drop(r);
    empty(&b);
    drop(files);
    free(address);
    let mut fresh = runtime(address, Generation::INITIAL.checked_next().unwrap(), &b);
    installed(&mut fresh);
    drop(fresh);
    empty(&b);
    free(address);
}

#[test]
fn serving_ingress_response_and_instance_quotas_refuse_before_native_effects() {
    let _serial = SERIAL.lock().unwrap();
    use ResourceKind as K;
    let address = address();
    let request = std::mem::size_of::<ProducerRequest>() as u64;
    let response = consumer_result_backing(4096, 64).unwrap();
    let fields = [
        (K::PendingHandlerCallsPerThingMax, 1),
        (K::InFlightResponsesPerThingMax, 1),
        (K::BindingIngressItemsPerRouteMax, 1),
        (K::BindingIngressItemsPerBindingMax, 1),
        (K::BindingIngressBytesPerRouteMax, request),
        (K::BindingIngressBytesPerBindingMax, request),
        (K::BindingResponseBufferBytesPerRouteMax, response),
        (K::PendingHandlerCallsGlobalMax, 1),
        (K::InFlightResponsesGlobalMax, 1),
        (K::BindingIngressItemsGlobalMax, 1),
        (K::BindingIngressBytesGlobalMax, request),
        (K::BindingResponseBufferBytesGlobalMax, response),
        (K::ThingsGlobalMax, 1),
        (K::BindingsGlobalMax, 1),
    ];
    for (kind, required) in fields {
        for ceiling in [required, required - 1] {
            let limits = GatewayDefaultV1::LIMITS
                .clone()
                .try_with_limit(kind, Some(ceiling))
                .unwrap();
            let b = bank();
            let (prepared, authority) = prepare(address, Generation::INITIAL);
            let key = authority.key();
            let result = Runtime::try_new(prepared, authority, limits, b.clone(), Deadline::NONE);
            assert_eq!(result.is_ok(), ceiling == required, "{kind:?}");
            match result {
                Ok(runtime) => drop(runtime),
                Err(refused) => {
                    assert!(
                        matches!(refused.error().error(), CoreError::LimitExceeded { resource, .. } if *resource == kind)
                    );
                    let (_, returned, _, original) = refused.into_input();
                    assert_eq!(returned.key(), key);
                    assert!(Arc::ptr_eq(&original, &b));
                }
            }
            empty(&b);
            free(address);
        }
        if INSTALLATION_ACCOUNT_KINDS.contains(&kind) {
            let parent_limits = GatewayDefaultV1::LIMITS
                .clone()
                .try_with_limit(kind, Some(required - 1))
                .unwrap();
            let b = Arc::new(producer_installation_accounts(&parent_limits));
            let (prepared, authority) = prepare(address, Generation::INITIAL);
            let refused = Runtime::try_new(
                prepared,
                authority,
                GatewayDefaultV1::LIMITS.clone(),
                b.clone(),
                Deadline::NONE,
            )
            .err()
            .expect("original parent capacity");
            assert!(
                matches!(refused.error().error(), CoreError::LimitExceeded { resource, .. } if *resource == kind)
            );
            empty(&b);
            free(address);
        }
    }
}
