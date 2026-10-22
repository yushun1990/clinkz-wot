#![cfg(feature = "prepare")]
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::{ProducerPropertyReadAdmission, ProducerPropertyReadStep};
use clinkz_wot_servient::{
    Servient, ServientBuilder, host_accounts::HostResourceAccounts,
    producer_installation::ProducerInstallationState as State,
};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCursor, ValidatedThingProgress, thing::Thing,
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Waker},
    time::{Duration, Instant},
};

struct AllocationTrace;
struct Trace {
    enabled: bool,
    blocks: [(usize, usize); 256],
    live: usize,
    peak: usize,
    allocations: usize,
}
std::thread_local! { static TRACE: std::cell::RefCell<Trace> = const { std::cell::RefCell::new(Trace { enabled:false, blocks:[(0,0);256], live:0, peak:0, allocations:0 }) }; }
// SAFETY: exact forwarding to System; a fixed allocation-free TLS table only
// observes this test thread's blocks. It never dereferences observed pointers.
unsafe impl std::alloc::GlobalAlloc for AllocationTrace {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(l) };
        let _ = TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled && !p.is_null() {
                let slot = t
                    .blocks
                    .iter()
                    .position(|b| b.0 == 0)
                    .expect("bounded trace slots");
                t.blocks[slot] = (p as usize, l.size());
                t.live += l.size();
                t.peak = t.peak.max(t.live);
                t.allocations += 1;
            }
        });
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        let _ = TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.enabled
                && let Some(i) = t.blocks.iter().position(|b| b.0 == p as usize)
            {
                t.live -= t.blocks[i].1;
                t.blocks[i] = (0, 0);
            }
        });
        unsafe { std::alloc::System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationTrace = AllocationTrace;

fn work(n: u64) -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, n))
}
fn address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn accepted<T, I>(result: Result<T, BindingInputRejection<I>>) -> T {
    result.unwrap_or_else(|e| panic!("{:?}", e.error()))
}
struct Handler {
    calls: Arc<AtomicUsize>,
    body: &'static [u8],
    invalid: bool,
}
impl ReadPropertyHandler for Handler {
    fn handle(
        &self,
        cx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(cx.operation(), Operation::ReadProperty);
        assert_eq!(cx.thing_id(), None);
        assert_eq!(cx.plan_id(), None);
        assert_eq!(cx.producer_instance().unwrap().slot(), cx.thing_slot());
        assert_eq!(cx.target().name(), Some(""));
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.invalid {
            Ok(InteractionOutput::empty())
        } else {
            Ok(InteractionOutput::with_data(Payload::new(
                self.body,
                "application/json",
            )))
        }
    }
}
type Server = Servient<HttpProducerExecutor, Handler>;
fn server(
    address: SocketAddr,
    bank: Arc<HostResourceAccounts>,
    calls: Arc<AtomicUsize>,
    body: &'static [u8],
    invalid: bool,
) -> Server {
    let authority = ProducerInstanceAuthority::try_new(ThingSlotId::new(
        SlotIndex::new(0),
        Generation::INITIAL,
    ))
    .unwrap();
    // Missing source id and an empty Property name are both valid. All source
    // and compiler owners die before the public Servient first touches a socket.
    let td: Thing = serde_json::from_value(serde_json::json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"real Producer", "securityDefinitions":{"n":{"scheme":"nosec"}},"security":["n"], "properties":{"":{"type":"number","readOnly":true,"forms":[{"href":format!("http://{address}/value")} ]}}})).unwrap();
    let config = ValidatedThingAdmissionConfig::try_from_limits(GatewayDefaultV1::LIMITS).unwrap();
    let mut cursor = ValidatedThingCursor::from_thing(
        &td,
        &config,
        AdmissionLedger::new(
            SlotIndex::new(0),
            Generation::INITIAL,
            0,
            8 << 20,
            0,
            0,
            0,
            0,
        ),
    );
    let proof = loop {
        cursor = match cursor.step(&mut work(1_000_000), false) {
            ValidatedThingProgress::Complete(p) => break p,
            ValidatedThingProgress::Pending(c) => c,
            ValidatedThingProgress::Failed(e) => panic!("{e:?}"),
        };
    };
    let mut admission = ProducerPropertyReadAdmission::new(proof, "");
    while admission.step(&mut work(1_000_000), false).unwrap() != ProducerPropertyReadStep::Covered
    {
    }
    let pair = accepted(try_producer_registration(
        HttpProducerConfig::for_loopback(address).unwrap(),
        BindingId::new(601),
        BindingGeneration::INITIAL,
        0,
    ));
    let handler = Handler {
        calls,
        body,
        invalid,
    };
    let prepared = accepted(pair.prepare_property_read(
        &mut admission,
        &authority,
        ProducerHandlerRegistration::new(
            &authority,
            "".into(),
            handler,
            HandlerFootprint::new(std::mem::size_of::<Handler>() as u64, 0, 0),
        ),
        &mut work(1_000_000),
    ));
    accepted(
        ServientBuilder::new(GatewayDefaultV1::LIMITS.clone(), bank).produce(
            prepared,
            authority,
            Deadline::NONE,
        ),
    )
}
fn step(s: &mut Server, budget: &mut WorkBudget, tick: u64) {
    let _ = s.step(
        &mut Context::from_waker(Waker::noop()),
        budget,
        MonotonicInstant::new(ClockId::new(1), tick),
    );
}
fn installed(s: &mut Server) {
    for _ in 0..4 {
        step(s, &mut work(1), 0);
        if s.state() == State::InstalledClosed {
            return;
        }
    }
    panic!("installation: {:?} {:?}", s.state(), s.first_cause());
}
fn peer(address: SocketAddr, request: &[u8]) -> TcpStream {
    let mut peer = TcpStream::connect(address).unwrap();
    peer.write_all(request).unwrap();
    peer.set_nonblocking(true).unwrap();
    peer
}
fn exchange(s: &mut Server, address: SocketAddr, request: &[u8]) -> Vec<u8> {
    let mut peer = peer(address, request);
    let mut response = Vec::new();
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        step(s, &mut work(1), 0);
        let mut bytes = [0; 8192];
        match peer.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&bytes[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
            Err(e) => panic!("{e:?}"),
        }
        assert!(
            Instant::now() < end,
            "stalled {:?} {:?}",
            s.state(),
            s.first_cause()
        );
        std::thread::yield_now();
    }
    response
}
fn empty(bank: &HostResourceAccounts) {
    assert!(
        ResourceKind::ALL
            .into_iter()
            .all(|k| bank.account(k).used() == 0)
    );
}
fn request(a: SocketAddr) -> String {
    format!("GET /value HTTP/1.1\r\nHost: {a}\r\n\r\n")
}

#[test]
fn public_servient_raw_peer_publication_dispatch_credit_and_one_reply() {
    let a = address();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = server(a, bank.clone(), calls.clone(), b"42", false);
    step(&mut s, &mut work(0), 0);
    assert_eq!(s.state(), State::Reserved);
    installed(&mut s);
    let mut p = peer(a, request(a).as_bytes());
    for _ in 0..10 {
        step(&mut s, &mut work(100), 0);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        p.read(&mut [0; 1]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    s.begin_expose(100).unwrap();
    for _ in 0..100 {
        step(
            &mut s,
            &mut WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
            0,
        );
        if s.state() == State::Dispatching {
            break;
        }
    }
    assert_eq!(s.state(), State::Dispatching);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for _ in 0..10 {
        step(&mut s, &mut work(0), 0);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    step(
        &mut s,
        &mut WorkBudget::new().with_remaining(WorkClass::HandlerSteps, 1),
        0,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(s.state(), State::Delivering);
    for _ in 0..100 {
        step(&mut s, &mut work(1), 0);
        if s.state() == State::Serving {
            break;
        }
    }
    p.set_nonblocking(false).unwrap();
    p.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let mut bytes = Vec::new();
    p.read_to_end(&mut bytes).unwrap();
    assert!(bytes.starts_with(b"HTTP/1.1 200 OK\r\n"));
    assert!(bytes.ends_with(b"42"));
    assert_eq!(bytes.windows(8).filter(|s| *s == b"HTTP/1.1").count(), 1);
    s.begin_close();
    step(&mut s, &mut work(0), 0);
    assert!(TcpListener::bind(a).is_err());
    step(&mut s, &mut work(1), 0);
    assert_eq!(s.state(), State::Closed);
    drop(s);
    empty(&bank);
    drop(TcpListener::bind(a).unwrap());
}

#[test]
fn malformed_or_unsupported_requests_never_dispatch_then_a_fresh_read_succeeds() {
    let a = address();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = server(a, bank.clone(), calls.clone(), b"42", false);
    s.begin_expose(100).unwrap();
    for bad in [
        format!("POST /value HTTP/1.1\r\nHost: {a}\r\n\r\n"),
        format!("GET /wrong HTTP/1.1\r\nHost: {a}\r\n\r\n"),
        "GET /value HTTP/1.1\r\nHost: bad\r\n\r\n".into(),
        format!("GET /value HTTP/1.1\r\nHost: {a}\r\nHost: {a}\r\n\r\n"),
        format!("GET /value HTTP/1.1\r\nHost: {a}\r\nContent-Length: 1\r\n\r\nx"),
        format!("GET /value HTTP/1.1\r\nHost: {a}\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"),
        format!("GET /value HTTP/1.1\r\nHost: {a}\r\nExpect: 100-continue\r\n\r\n"),
        format!("GET /value HTTP/1.1\r\nHost: {a}\r\nConnection: upgrade\r\n\r\n"),
        format!("{}{}", request(a), request(a)),
    ] {
        while !matches!(s.state(), State::Serving) {
            step(&mut s, &mut work(1), 0);
        }
        let reply = exchange(&mut s, a, bad.as_bytes());
        assert!(!reply.starts_with(b"HTTP/1.1 200"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    assert!(exchange(&mut s, a, request(a).as_bytes()).ends_with(b"42"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(s);
    empty(&bank);
}

#[test]
fn invalid_handler_success_is_core_sealed_as_failure() {
    let a = address();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = server(a, bank.clone(), calls.clone(), b"42", true);
    installed(&mut s);
    s.begin_expose(100).unwrap();
    assert!(exchange(&mut s, a, request(a).as_bytes()).starts_with(b"HTTP/1.1 500"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(s);
    empty(&bank);
}

#[test]
fn unfinished_head_deadline_and_last_root_loss_retire_native_participation() {
    for deadline in [false, true] {
        let a = address();
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut s = server(a, bank.clone(), calls.clone(), b"42", false);
        installed(&mut s);
        s.begin_expose(10).unwrap();
        let mut p = peer(a, b"GET /value HTTP/1.1\r\nHost:");
        for _ in 0..5 {
            step(&mut s, &mut work(1), 0);
        }
        if deadline {
            step(&mut s, &mut work(0), 10);
            assert_eq!(s.state(), State::Closing);
            assert!(matches!(s.first_cause(), Some(CoreError::TimedOut(_))));
            s.cancel();
            assert!(matches!(s.first_cause(), Some(CoreError::TimedOut(_))));
        }
        drop(s);
        empty(&bank);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(TcpListener::bind(a).unwrap());
        assert!(matches!(p.read(&mut [0; 1]), Ok(0) | Err(_)));
    }
}

#[test]
fn one_hundred_real_reads_and_distinct_instance_incarnations() {
    let a = address();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut previous = None;
    for body in [b"42".as_slice(), b"43".as_slice()] {
        let mut s = server(a, bank.clone(), calls.clone(), body, false);
        assert_ne!(Some(s.instance_key()), previous);
        previous = Some(s.instance_key());
        installed(&mut s);
        s.begin_expose(100).unwrap();
        let baseline = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        for _ in 0..100 {
            assert!(exchange(&mut s, a, request(a).as_bytes()).ends_with(body));
            assert_eq!(
                bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
                baseline
            );
        }
        drop(s);
        empty(&bank);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 200);
}

#[test]
fn exact_head_and_field_limits_and_one_over_are_enforced_before_dispatch() {
    let a = address();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = server(a, bank.clone(), calls.clone(), b"42", false);
    installed(&mut s);
    s.begin_expose(100).unwrap();
    let prefix = format!("GET /value HTTP/1.1\r\nHost: {a}\r\nX-Pad: ");
    let exact = format!("{}{}\r\n\r\n", prefix, "x".repeat(8192 - prefix.len() - 4));
    assert_eq!(exact.len(), 8192);
    let mut p = peer(a, exact.as_bytes());
    step(&mut s, &mut work(1), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        s.state(),
        State::Serving,
        "first native callback reads at most 4096 bytes"
    );
    for _ in 0..10 {
        step(&mut s, &mut work(1), 0);
    }
    p.set_nonblocking(false).unwrap();
    p.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let mut reply = Vec::new();
    p.read_to_end(&mut reply).unwrap();
    assert!(reply.ends_with(b"42"));
    let over = format!("{}{}\r\n\r\n", prefix, "x".repeat(8193 - prefix.len() - 4));
    assert!(!exchange(&mut s, a, over.as_bytes()).starts_with(b"HTTP/1.1 200"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for fields in [15, 16] {
        let req = format!(
            "GET /value HTTP/1.1\r\nHost: {a}\r\n{}\r\n",
            (0..fields)
                .map(|i| format!("X-{i}: v\r\n"))
                .collect::<String>()
        );
        let good = exchange(&mut s, a, req.as_bytes()).starts_with(b"HTTP/1.1 200");
        assert_eq!(good, fields == 15);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    drop(s);
    empty(&bank);
}

#[test]
fn cancelled_dispatch_and_unread_reply_keep_the_original_response_capacity_until_drop() {
    for state in [State::Dispatching, State::Delivering] {
        let a = address();
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut s = server(a, bank.clone(), calls.clone(), b"42", false);
        installed(&mut s);
        s.begin_expose(100).unwrap();
        let mut p = peer(a, request(a).as_bytes());
        for _ in 0..100 {
            step(&mut s, &mut work(1), 0);
            if s.state() == state {
                break;
            }
        }
        assert_eq!(s.state(), state);
        let before = bank.account(ResourceKind::EngineLiveBytesGlobalMax).used();
        s.cancel();
        step(&mut s, &mut work(0), 0);
        assert_eq!(
            bank.account(ResourceKind::EngineLiveBytesGlobalMax).used(),
            before
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            usize::from(state == State::Delivering)
        );
        drop(s);
        empty(&bank);
        p.set_nonblocking(false).unwrap();
        p.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut bytes = Vec::new();
        let _ = p.read_to_end(&mut bytes);
        assert!(!bytes.starts_with(b"HTTP/1.1 200"));
    }
}

#[test]
fn response_body_bound_and_json_syntax_are_checked_before_wire_success() {
    static EXACT: [u8; 4096] = {
        let mut b = [b' '; 4096];
        b[4095] = b'7';
        b
    };
    static OVER: [u8; 4097] = {
        let mut b = [b' '; 4097];
        b[4096] = b'7';
        b
    };
    for (body, good) in [
        (&EXACT[..], true),
        (&OVER[..], false),
        (b"not-json".as_slice(), false),
    ] {
        let a = address();
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut s = server(a, bank.clone(), calls.clone(), body, false);
        installed(&mut s);
        s.begin_expose(100).unwrap();
        let reply = exchange(&mut s, a, request(a).as_bytes());
        assert_eq!(reply.starts_with(b"HTTP/1.1 200"), good);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(s);
        empty(&bank);
    }
}

struct Clock(Instant);
impl RuntimeClock for Clock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(2), self.0.elapsed().as_millis() as u64)
    }
    fn ticks_per_second(&self) -> std::num::NonZeroU64 {
        std::num::NonZeroU64::new(1000).unwrap()
    }
}
fn consumer_policy() -> clinkz_wot_servient::consumer_property_read::ConsumerPropertyReadPolicy {
    clinkz_wot_servient::consumer_property_read::ConsumerPropertyReadPolicy::try_new_owned(
        BindingResourceContract::ConsumerOwnedHostV2,
        ResourceProfileId::GATEWAY_DEFAULT_V1,
        GatewayDefaultV1::LIMITS.clone(),
        BindingLifetimeFootprint::new(8, 8192),
        BindingLifetimeFootprint::new(8, 4096),
        [1, 64, 64, 64],
        [4096, 16],
        [2, 2, 1, 1, 1, 1, 1],
        [30_000, 1000],
        65536,
    )
    .unwrap()
}
#[test]
fn public_consumer_and_producer_share_capacity_and_execute_two_source_free_generations() {
    use clinkz_wot_servient::consumer_property_read::ConsumerPropertyReadRuntime;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    executor.block_on(async {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        for body in [b"42".as_slice(), b"43".as_slice()] {
            let a = address();
            let calls = Arc::new(AtomicUsize::new(0));
            let mut s = server(a, bank.clone(), calls.clone(), body, false);
            installed(&mut s);
            s.begin_expose(30_000).unwrap();
            let registration = try_consumer_registration(
                HttpConsumerConfig::for_cleartext_ip().unwrap(),
                BindingId::new(602),
                BindingGeneration::INITIAL,
                0,
            )
            .unwrap();
            let (client, report) = accepted(ConsumerPropertyReadRuntime::try_new(
                registration,
                consumer_policy(),
                Arc::new(Clock(Instant::now())),
                bank.clone(),
            ));
            let td: Thing = serde_json::from_value(serde_json::json!({
                "@context": "https://www.w3.org/2022/wot/td/v1.1",
                "title": "client", "id": "urn:test:consumer",
                "securityDefinitions": {"n": {"scheme": "nosec"}}, "security": ["n"],
                "properties": {"value": {"forms": [{"href": format!("http://{a}/value")}]}}
            }))
            .unwrap();
            let handle = match client
                .consume(&td, &mut work(1_000_000), |build| {
                    for _ in 0..10000 {
                        build.step(&mut work(1_000_000));
                        if build.is_finished() {
                            return;
                        }
                    }
                    panic!("Consumer preparation stalled");
                })
                .unwrap()
            {
                StartStatus::Ready(handle) => handle,
                _ => panic!("Consumer not installed"),
            };
            drop(td);
            let baseline = bank.snapshot(ResourceKind::ALL).map(|a| a.used());
            for _ in 0..100 {
                let mut call = handle
                    .read_property("value", InteractionOptions::new(), &mut work(1_000_000))
                    .unwrap();
                let end = Instant::now() + Duration::from_secs(2);
                let output = loop {
                    step(&mut s, &mut work(1), 0);
                    call.step(
                        &mut Context::from_waker(Waker::noop()),
                        &mut work(1_000_000),
                    );
                    if let Some(result) = call.try_take(&mut work(1_000_000)).unwrap() {
                        break result.unwrap();
                    }
                    assert!(Instant::now() < end, "public loop stalled {:?}", s.state());
                    tokio::time::sleep(Duration::from_millis(1)).await;
                };
                assert_eq!(&*output.data().unwrap().body, body);
                drop(output);
                drop(call);
                assert_eq!(bank.snapshot(ResourceKind::ALL).map(|a| a.used()), baseline);
            }
            assert_eq!(calls.load(Ordering::SeqCst), 100);
            drop(handle);
            let mut retired = false;
            for _ in 0..10000 {
                if let StartStatus::Ready(outcome) = client.shutdown(
                    &mut Context::from_waker(Waker::noop()),
                    &mut work(1_000_000),
                ) {
                    assert!(outcome.is_complete());
                    retired = true;
                    break;
                }
            }
            assert!(retired);
            drop(client);
            drop(report);
            drop(s);
            empty(&bank);
        }
    });
}

#[test]
fn a_live_producer_exhausts_the_same_cleanup_capacity_used_by_public_consumer() {
    use clinkz_wot_servient::consumer_property_read::ConsumerPropertyReadRuntime;
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(ResourceKind::CleanupItemsMax, Some(1))
        .unwrap();
    let bank = Arc::new(HostResourceAccounts::new(&limits));
    let a = address();
    let mut s = server(a, bank.clone(), Arc::new(AtomicUsize::new(0)), b"42", false);
    installed(&mut s);
    let registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(602),
        BindingGeneration::INITIAL,
        0,
    )
    .unwrap();
    let rejection = ConsumerPropertyReadRuntime::try_new(
        registration,
        consumer_policy(),
        Arc::new(Clock(Instant::now())),
        bank.clone(),
    )
    .err()
    .expect("shared cleanup capacity must refuse");
    assert_eq!(bank.account(ResourceKind::CleanupItemsMax).used(), 1);
    assert_eq!(
        rejection
            .error()
            .error()
            .context()
            .resource_failure()
            .unwrap()
            .1,
        ResourceFailureDomain::EngineCapacity
    );
    let (registration, policy, clock, original) = rejection.into_input();
    assert!(Arc::ptr_eq(&bank, &original));
    drop(s);
    empty(&bank);
    let (client, report) = accepted(ConsumerPropertyReadRuntime::try_new(
        registration,
        policy,
        clock,
        original,
    ));
    drop(client);
    drop(report);
    empty(&bank);
}

#[test]
fn native_allocations_and_descriptors_retire_for_success_unfinished_head_and_unread_reply() {
    for mode in 0..3 {
        let a = address();
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut s = server(a, bank.clone(), calls, b"42", false);
        installed(&mut s);
        s.begin_expose(100).unwrap();
        let req = if mode == 1 {
            "GET /value HTTP/1.1\r\nHost:".into()
        } else {
            request(a)
        };
        let mut p = peer(a, req.as_bytes());
        #[cfg(target_os = "linux")]
        let descriptors = std::fs::read_dir("/proc/self/fd").unwrap().count();
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            t.live = 0;
            t.peak = 0;
            t.allocations = 0;
            t.enabled = true;
        });
        if mode == 0 {
            let mut received = 0;
            for _ in 0..1000 {
                step(&mut s, &mut work(1), 0);
                let mut bytes = [0; 512];
                match p.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => received += n,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => panic!("{e:?}"),
                }
            }
            assert!(received > 0);
        } else {
            for _ in 0..10 {
                step(&mut s, &mut work(1), 0);
                if mode == 2 && s.state() == State::Delivering {
                    break;
                }
            }
            s.cancel();
            step(&mut s, &mut work(1), 0);
        }
        let (live, peak, count) = TRACE.with(|t| {
            let mut t = t.borrow_mut();
            t.enabled = false;
            (t.live, t.peak, t.allocations)
        });
        assert_eq!(live, 0, "native/handler backing escaped retirement");
        assert!(count > 0);
        eprintln!("native Producer mode={mode}: allocations={count}, peak={peak}, retained={live}");
        #[cfg(target_os = "linux")]
        assert_eq!(
            std::fs::read_dir("/proc/self/fd").unwrap().count(),
            descriptors - usize::from(mode != 0)
        );
        drop(s);
        drop(p);
        empty(&bank);
    }
}

#[test]
fn real_peer_reset_during_reply_closes_serving_authority_and_preserves_first_cause() {
    let a = address();
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = server(a, bank.clone(), calls, b"42", false);
    installed(&mut s);
    s.begin_expose(100).unwrap();
    let p = peer(a, request(a).as_bytes());
    for _ in 0..100 {
        step(&mut s, &mut work(1), 0);
        if s.state() == State::Delivering {
            break;
        }
    }
    assert_eq!(s.state(), State::Delivering);
    socket2::SockRef::from(&p)
        .set_linger(Some(Duration::ZERO))
        .unwrap();
    drop(p);
    for _ in 0..10 {
        step(&mut s, &mut work(1), 0);
        if s.state() == State::Closing {
            break;
        }
    }
    assert_eq!(s.state(), State::Closing);
    let cause = s.first_cause().unwrap().clone();
    s.cancel();
    assert_eq!(s.first_cause(), Some(&cause));
    step(&mut s, &mut work(1), 0);
    assert_eq!(s.state(), State::Closed);
    drop(s);
    empty(&bank);
}
