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
#[derive(Default)]
struct ValidationTrace {
    enabled: bool,
    blocks: [(usize, usize); 32],
    live: usize,
    peak: usize,
    largest: usize,
}
std::thread_local! {
    static TRACE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MAX: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FAIL_SIZE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static OBSERVE_FREE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FUNDED_AT_FREE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static OBSERVED_BANK: std::cell::RefCell<Option<Arc<ProducerInstallationAccounts>>> = const { std::cell::RefCell::new(None) };
    static VALIDATION_TRACE: std::cell::RefCell<ValidationTrace> = const { std::cell::RefCell::new(ValidationTrace {
        enabled: false, blocks: [(0, 0); 32], live: 0, peak: 0, largest: 0,
    }) };
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
        let pointer = unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) };
        let _ = VALIDATION_TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled && !pointer.is_null() {
                let slot = t
                    .blocks
                    .iter()
                    .position(|b| b.0 == 0)
                    .expect("bounded trace slots");
                t.blocks[slot] = (pointer as usize, layout.size());
                t.live += layout.size();
                t.peak = t.peak.max(t.live);
                t.largest = t.largest.max(layout.size());
            }
        });
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        let _ = VALIDATION_TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled
                && let Some(i) = t.blocks.iter().position(|b| b.0 == pointer as usize)
            {
                t.live -= t.blocks[i].1;
                t.blocks[i] = (0, 0);
            }
        });
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

fn observe_validation(
    program: &HttpProducerProgram,
    output: &InteractionOutput,
) -> (CoreResult<()>, ValidationTrace) {
    VALIDATION_TRACE.with(|t| {
        *t.borrow_mut() = ValidationTrace {
            enabled: true,
            ..ValidationTrace::default()
        }
    });
    let result = program.validate(output);
    let trace = VALIDATION_TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()));
    (result, trace)
}

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
    prepare_with_schema(address, generation, json!({"type":"number"}))
}
fn prepare_with_schema(
    address: SocketAddr,
    generation: Generation,
    mut schema: Value,
) -> (
    PreparedProducer<HttpProducerExecutor, Handler>,
    ProducerInstanceAuthority,
) {
    let authority = ProducerInstanceAuthority::try_new(slot(generation)).unwrap();
    let mut value = document(address);
    schema["readOnly"] = json!(true);
    schema["forms"] = value["properties"][""]["forms"].take();
    value["properties"][""] = schema;
    let td: Thing = serde_json::from_value(value).unwrap();
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
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
    Arc::new(producer_installation_accounts(GatewayDefaultV1::LIMITS))
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
fn schema_backing_counts_toward_public_installation_artifact_quotas() {
    let _serial = SERIAL.lock().unwrap();
    use ResourceKind as K;
    let address = address();
    let b = bank();
    let (prepared, authority) =
        prepare_with_schema(address, Generation::INITIAL, json!({"type":"object"}));
    let r = accepted(Runtime::try_new(
        prepared,
        authority,
        GatewayDefaultV1::LIMITS.clone(),
        b.clone(),
        Deadline::NONE,
    ));
    let primitive_bytes = b.account(K::BindingArtifactBytesGlobalMax).used();
    let inline_bytes = (std::mem::size_of::<ProducerExecutionEntry<HttpProducerProgram>>()
        + 2 * std::mem::size_of::<usize>()) as u64;
    assert!(
        primitive_bytes > inline_bytes,
        "the primitive schema Arc must be funded"
    );
    drop(r);
    empty(&b);

    let properties: serde_json::Map<_, _> = (0..32)
        .map(|n| (format!("field{n}"), json!({"type":"integer","minimum":0})))
        .collect();
    let schema = json!({"type":"object","properties":properties});
    for kind in [
        K::BindingArtifactBytesGlobalMax,
        K::BindingArtifactBytesPerItemMax,
        K::BindingArtifactBytesPerThingMax,
    ] {
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .try_with_limit(kind, Some(primitive_bytes))
            .unwrap();
        let (prepared, authority) =
            prepare_with_schema(address, Generation::INITIAL, schema.clone());
        let key = authority.key();
        let refused = Runtime::try_new(prepared, authority, limits, b.clone(), Deadline::NONE)
            .err()
            .expect("structured backing must exceed the primitive program quota");
        assert!(matches!(
            refused.error().error(),
            CoreError::LimitExceeded { resource, requested: Some(n), .. }
                if *resource == kind && *n > primitive_bytes
        ));
        empty(&b);
        free(address);
        let (prepared, authority, _, original) = refused.into_input();
        assert_eq!(authority.key(), key);
        assert!(Arc::ptr_eq(&original, &b));
        let r = accepted(Runtime::try_new(
            prepared,
            authority,
            GatewayDefaultV1::LIMITS.clone(),
            original,
            Deadline::NONE,
        ));
        assert!(b.account(K::BindingArtifactBytesGlobalMax).used() > primitive_bytes);
        drop(r);
        empty(&b);
    }
}

#[test]
fn structured_validation_scratch_is_funded_before_execution() {
    let _serial = SERIAL.lock().unwrap();
    use ResourceKind as K;
    let address = address();
    let string = json!({"type":"string","minLength":1,"maxLength":4094});
    let object = json!({"type":"object","properties":{"label":string},"required":["label"]});
    let object_prefix = "{\"la\\u0062el\":\"";
    let object_suffix = "\\n\"}";
    let unknown_suffix = "\\u0061\":0,\"la\\u0062el\":\"ok\"}";
    let duplicate_suffix = "\\u0061\":0,\"label\":\"ok\",\"la\\u0062el\":\"duplicate\"}";
    let nested_prefix = "{\"co\\u006efig\":{\"la\\u0062el\":\"";
    let nested_suffix = "\\n\"}}";
    let cases = [
        (
            "plain",
            string.clone(),
            format!("\"{}\"", "a".repeat(3000)),
            true,
        ),
        (
            "body limit",
            string.clone(),
            format!("\"{}\"", "a".repeat(4094)),
            true,
        ),
        // A late escape can double a nearly full decoder scratch buffer. The
        // observer includes old/new allocation overlap and the owned String.
        (
            "late escape",
            string.clone(),
            format!("\"{}\\n\"", "a".repeat(4092)),
            true,
        ),
        (
            "late Unicode escape",
            string.clone(),
            format!("\"{}\\u0062\"", "a".repeat(4088)),
            true,
        ),
        (
            "surrogate pair",
            string.clone(),
            format!("\"{}\\ud83d\\ude00\"", "a".repeat(4082)),
            true,
        ),
        (
            "invalid length",
            json!({"type":"string","maxLength":2}),
            format!("\"{}\"", "a".repeat(3000)),
            false,
        ),
        (
            "escaped key/value",
            object.clone(),
            format!(
                "{object_prefix}{}{object_suffix}",
                "a".repeat(4096 - object_prefix.len() - object_suffix.len())
            ),
            true,
        ),
        (
            "long unknown key",
            object.clone(),
            format!(
                "{{\"{}{unknown_suffix}",
                "k".repeat(4094 - unknown_suffix.len())
            ),
            true,
        ),
        (
            "duplicate decoded key",
            object.clone(),
            format!(
                "{{\"{}{duplicate_suffix}",
                "k".repeat(4094 - duplicate_suffix.len())
            ),
            false,
        ),
        (
            "nested lookup",
            json!({"type":"object","properties":{"config":object},"required":["config"]}),
            format!(
                "{nested_prefix}{}{nested_suffix}",
                "a".repeat(4096 - nested_prefix.len() - nested_suffix.len())
            ),
            true,
        ),
    ];
    for (label, schema, body, valid) in cases {
        assert!(body.len() <= 4096);
        let (prepared, authority) = prepare_with_schema(address, Generation::INITIAL, schema);
        let policy = prepared.install_policy();
        let limits = GatewayDefaultV1::LIMITS
            .clone()
            .try_with_limit(
                K::BindingPollTemporaryBytesPerCallMax,
                Some(policy.temporary_bytes),
            )
            .unwrap()
            .try_with_limit(
                K::BindingPollTemporaryBytesGlobalMax,
                Some(policy.temporary_bytes),
            )
            .unwrap()
            .try_with_limit(
                K::LargestContiguousAllocationBytesMax,
                Some(policy.largest_indirect_allocation),
            )
            .unwrap();
        let b = Arc::new(producer_installation_accounts(&limits));
        let r = accepted(Runtime::try_new(
            prepared,
            authority,
            limits,
            b.clone(),
            Deadline::NONE,
        ));
        assert_eq!(
            b.account(K::BindingPollTemporaryBytesGlobalMax).used(),
            policy.temporary_bytes
        );
        let before = b.snapshot(INSTALLATION_ACCOUNT_KINDS);
        // Provision the output before tracing only the same captured program
        // validator used by dispatch; no handler or HTTP SDK allocation enters.
        let output =
            InteractionOutput::with_data(Payload::new(body.into_bytes(), "application/json"));
        let (result, trace) = observe_validation(r.entry().unwrap().program(), &output);
        eprintln!(
            "{label}: validation peak={}, largest={}, declared={}/{}",
            trace.peak, trace.largest, policy.temporary_bytes, policy.largest_indirect_allocation
        );
        assert_eq!(result.is_ok(), valid, "{label}: {result:?}");
        assert!(
            trace.peak > 1024,
            "{label} must exercise allocating decoding"
        );
        assert_eq!(
            trace.live, 0,
            "validation scratch must be released on return"
        );
        assert!(
            trace.peak as u64 <= policy.temporary_bytes,
            "{label}: validation peak {} exceeds declared {}",
            trace.peak,
            policy.temporary_bytes
        );
        assert!(
            trace.largest as u64 <= policy.largest_indirect_allocation,
            "{label}"
        );
        assert_eq!(b.snapshot(INSTALLATION_ACCOUNT_KINDS), before);
        drop(r);
        empty(&b);
        free(address);
    }
}

#[test]
fn structured_validation_scratch_limits_refuse_before_native_effects() {
    let _serial = SERIAL.lock().unwrap();
    use ResourceKind as K;
    let address = address();
    let schema = json!({"type":"string","minLength":1});
    let (prepared, authority) = prepare_with_schema(address, Generation::INITIAL, schema.clone());
    let policy = prepared.install_policy();
    assert!(policy.temporary_bytes > 1024);
    drop((prepared, authority));
    for (kind, amount) in [
        (
            K::BindingPollTemporaryBytesPerCallMax,
            policy.temporary_bytes,
        ),
        (
            K::BindingPollTemporaryBytesGlobalMax,
            policy.temporary_bytes,
        ),
        (
            K::LargestContiguousAllocationBytesMax,
            policy.largest_indirect_allocation,
        ),
    ] {
        for ceiling in [amount, amount - 1, 1024] {
            let limits = GatewayDefaultV1::LIMITS
                .clone()
                .try_with_limit(kind, Some(ceiling))
                .unwrap();
            let b = bank();
            let (prepared, authority) =
                prepare_with_schema(address, Generation::INITIAL, schema.clone());
            let key = authority.key();
            let result = Runtime::try_new(prepared, authority, limits, b.clone(), Deadline::NONE);
            assert_eq!(result.is_ok(), ceiling == amount, "{kind:?}/{ceiling}");
            match result {
                Ok(r) => drop(r),
                Err(refused) => {
                    assert!(
                        matches!(refused.error().error(), CoreError::LimitExceeded { resource, .. } if *resource == kind)
                    );
                    empty(&b);
                    free(address);
                    let (prepared, authority, _, original) = refused.into_input();
                    assert_eq!(authority.key(), key);
                    assert!(Arc::ptr_eq(&original, &b));
                    drop(accepted(Runtime::try_new(
                        prepared,
                        authority,
                        GatewayDefaultV1::LIMITS.clone(),
                        original,
                        Deadline::NONE,
                    )));
                }
            }
            empty(&b);
            free(address);
        }
    }
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(
            K::BindingPollTemporaryBytesGlobalMax,
            Some(policy.temporary_bytes - 1),
        )
        .unwrap();
    let b = Arc::new(producer_installation_accounts(&limits));
    let (prepared, authority) = prepare_with_schema(address, Generation::INITIAL, schema);
    let refused = Runtime::try_new(
        prepared,
        authority,
        GatewayDefaultV1::LIMITS.clone(),
        b.clone(),
        Deadline::NONE,
    )
    .err()
    .expect("the original parent must fund all validator scratch");
    assert!(matches!(
        refused.error().error(),
        CoreError::LimitExceeded {
            resource: K::BindingPollTemporaryBytesGlobalMax,
            ..
        }
    ));
    drop(refused);
    empty(&b);
    free(address);
}

#[test]
fn nested_schema_limits_and_final_disposal_keep_the_original_funding() {
    let _serial = SERIAL.lock().unwrap();
    use ResourceKind as K;
    use clinkz_wot_core::data_schema::{PreparedSchema, SchemaProperty};
    let address = address();
    let mut properties: serde_json::Map<_, _> = (0..31)
        .map(|n| (format!("outer{n}"), json!({"type":"integer","minimum":0})))
        .collect();
    let nested: serde_json::Map<_, _> = (0..31)
        .map(|n| (format!("inner{n}"), json!({"type":"integer","maximum":100})))
        .collect();
    let name_bytes = properties
        .keys()
        .chain(nested.keys())
        .map(String::len)
        .sum::<usize>()
        + "nested".len();
    properties.insert(
        "nested".into(),
        json!({"type":"object","properties":nested}),
    );
    // 64 nodes in two field tables, with total backing larger than any single
    // allocation. This distinguishes the retained-byte and contiguous limits.
    let schema = json!({"type":"object","properties":properties});
    let field_table = 32 * std::mem::size_of::<SchemaProperty>();
    let schema_bytes = (std::mem::size_of::<PreparedSchema>()
        + 2 * std::mem::size_of::<usize>()
        + 63 * std::mem::size_of::<SchemaProperty>()
        + name_bytes) as u64;
    // The program now also retains the common primary response and its media,
    // sharing that physical owner with configuration/installation snapshots.
    let response_bytes =
        (std::mem::size_of::<clinkz_wot_core::execution::responses::FiniteResponses>()
            + 2 * std::mem::size_of::<usize>()
            + "application/json".len()) as u64;
    let b = bank();
    let mut required = Vec::new();
    for close in [true, false] {
        let (prepared, authority) =
            prepare_with_schema(address, Generation::INITIAL, schema.clone());
        let policy = prepared.install_policy();
        assert_eq!(policy.program_bytes, schema_bytes + response_bytes);
        assert!(policy.largest_indirect_allocation >= field_table as u64);
        assert!(policy.local_drop_work >= 2 * PreparedSchema::MAX_NODES as u64);
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
        let mut r = accepted(result);
        let owner_block = MAX.with(|n| n.get());
        let used = |kind| b.account(kind).used();
        if required.is_empty() {
            for (global, local) in [
                (K::EngineLiveBytesGlobalMax, K::EngineLiveBytesGlobalMax),
                (
                    K::CompiledRuntimeBytesGlobalMax,
                    K::CompiledRuntimeBytesPerThingMax,
                ),
                (
                    K::BindingArtifactBytesGlobalMax,
                    K::BindingArtifactBytesPerItemMax,
                ),
                (
                    K::BindingArtifactBytesGlobalMax,
                    K::BindingArtifactBytesPerThingMax,
                ),
                (
                    K::BindingSlotStateBytesGlobalMax,
                    K::BindingSlotStateBytesPerItemMax,
                ),
                (
                    K::BindingSlotStateBytesGlobalMax,
                    K::BindingSlotStateBytesPerThingMax,
                ),
                (K::CleanupBytesMax, K::CleanupItemBytesMax),
            ] {
                required.push((local, used(global)));
            }
            required.push((K::CleanupWorkItemsPerStepMax, policy.local_drop_work));
            required.push((
                K::LargestContiguousAllocationBytesMax,
                owner_block
                    .max(field_table)
                    .max(policy.largest_indirect_allocation as usize)
                    .max(4096 + 2 * std::mem::size_of::<usize>()) as u64,
            ));
        }
        assert!(used(K::BindingArtifactBytesGlobalMax) >= schema_bytes);
        installed(&mut r);
        let before = b.snapshot(INSTALLATION_ACCOUNT_KINDS);
        OBSERVED_BANK.with(|p| *p.borrow_mut() = Some(b.clone()));
        FUNDED_AT_FREE.with(|f| f.set(false));
        OBSERVE_FREE.with(|s| s.set(field_table));
        if close {
            r.cancel();
            assert!(step(&mut r, 0).is_pending());
            assert_eq!(b.snapshot(INSTALLATION_ACCOUNT_KINDS), before);
            assert_eq!(
                step(&mut r, 1),
                Poll::Ready(ProducerInstallationState::Closed)
            );
            assert_eq!(used(K::BindingArtifactBytesGlobalMax), 0);
        }
        drop(r);
        assert!(
            FUNDED_AT_FREE.with(|f| f.get()),
            "field table must be freed while funded"
        );
        OBSERVE_FREE.with(|s| s.set(0));
        OBSERVED_BANK.with(|p| p.borrow_mut().take());
        empty(&b);
        free(address);
    }
    for (kind, amount) in required {
        for ceiling in [amount, amount - 1] {
            let limits = GatewayDefaultV1::LIMITS
                .clone()
                .try_with_limit(kind, Some(ceiling))
                .unwrap();
            let (prepared, authority) =
                prepare_with_schema(address, Generation::INITIAL, schema.clone());
            let result = Runtime::try_new(prepared, authority, limits, b.clone(), Deadline::NONE);
            assert_eq!(result.is_ok(), ceiling == amount, "{kind:?}");
            if let Err(refused) = &result {
                assert!(
                    matches!(refused.error().error(), CoreError::LimitExceeded { resource, .. } if *resource == kind)
                );
            }
            drop(result);
            empty(&b);
            free(address);
        }
    }
}

#[test]
fn measured_owner_layout_exact_limits_allocation_refusal_and_deallocation_funding() {
    let _serial = SERIAL.lock().unwrap();
    let address = address();
    let b = bank();
    let (prepared, authority) = prepare(address, Generation::INITIAL);
    let policy = prepared.install_policy();
    let schema_bytes = policy.program_bytes;
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
        let extra = schema_bytes
            + 2 * std::mem::size_of::<usize>() as u64
            + consumer_result_backing(4096, 64).unwrap();
        assert_eq!(bank[4].used(), block as u64 + extra);
        assert_eq!(
            bank[0].used(),
            (block + std::mem::size_of::<Runtime>()) as u64 + extra + policy.temporary_bytes
        );
        assert_eq!(
            bank[1].used(),
            (block + std::mem::size_of::<Runtime>()) as u64 + extra
        );
    }
    OBSERVED_BANK.with(|p| *p.borrow_mut() = Some(b.clone()));
    FUNDED_AT_FREE.with(|f| f.set(false));
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
