// Independent raw TCP peer: no Hyper, httparse or binding transport in the peer.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
#[allow(unused_imports)] // Semantic compiler suites share this application include.
use clinkz_wot_planning::*;
use clinkz_wot_servient::consumer_property_read::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    num::NonZeroU64,
    sync::{Arc, Mutex},
    task::{Context, Waker},
    time::{Duration, Instant},
};

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}
struct Clock(Instant);
impl RuntimeClock for Clock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(600), self.0.elapsed().as_millis() as u64)
    }
    fn ticks_per_second(&self) -> NonZeroU64 {
        NonZeroU64::new(1000).unwrap()
    }
}
fn policy() -> ConsumerPropertyReadPolicy {
    ConsumerPropertyReadPolicy::try_new_owned(
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
type Engine<O> = (
    ConsumerPropertyReadRuntime,
    ConsumerPropertyReadHandle,
    O,
    Arc<Mutex<[ResourceAccount; 20]>>,
);
fn engine(port: u16) -> Engine<impl Fn() -> (Option<CleanupOutcome>, [u64; 4])> {
    engine_with_targets(
        format!("http://127.0.0.1:{port}/a?x=1"),
        format!("http://127.0.0.1:{port}/b"),
    )
}
fn engine_with_targets(
    a: String,
    b: String,
) -> Engine<impl Fn() -> (Option<CleanupOutcome>, [u64; 4])> {
    engine_with_clock(a, b, Arc::new(Clock(Instant::now())))
}
fn engine_with_clock(
    a: String,
    b: String,
    clock: Arc<dyn RuntimeClock + Send + Sync>,
) -> Engine<impl Fn() -> (Option<CleanupOutcome>, [u64; 4])> {
    let policy = policy();
    let parent = Arc::new(Mutex::new(policy.parent_accounts()));
    let registration = try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(600),
        BindingGeneration::INITIAL,
        600,
    )
    .unwrap();
    let (runtime, observer) =
        ConsumerPropertyReadRuntime::try_new(registration, policy, clock, parent.clone())
            .unwrap_or_else(|e| panic!("provision: {:?}", e.error()));
    let thing: clinkz_wot_td::thing::Thing = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"real HTTP", "id":"urn:real-http",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties": { "a":{"forms":[{"href":a}]},
                        "b":{"forms":[{"href":b}]}}
    }))
    .unwrap();
    let handle = match runtime
        .consume(&thing, &mut work(), |build| {
            for _ in 0..10000 {
                build.step(&mut work());
                if build.is_finished() {
                    return;
                }
            }
            panic!("build stalled");
        })
        .unwrap()
    {
        StartStatus::Ready(handle) => handle,
        _ => panic!("publication pending"),
    };
    drop(thing);
    (runtime, handle, observer, parent)
}
fn recipe() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
}
fn request(socket: &mut TcpStream) -> String {
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 1);
        head.push(byte[0]);
        assert!(head.len() < 2048);
    }
    String::from_utf8(head).unwrap()
}
async fn take(call: &mut ConsumerPropertyReadCall) -> CoreResult<InteractionOutput> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        call.step(&mut Context::from_waker(Waker::noop()), &mut work());
        if let Some(result) = call.try_take(&mut work()).unwrap() {
            return result;
        }
        assert!(Instant::now() < end, "call stalled");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
fn shutdown(runtime: &ConsumerPropertyReadRuntime) {
    for _ in 0..10000 {
        if let StartStatus::Ready(outcome) =
            runtime.shutdown(&mut Context::from_waker(Waker::noop()), &mut work())
        {
            assert!(outcome.is_complete());
            return;
        }
    }
    panic!("shutdown stalled");
}

fn assert_retired(socket: &mut TcpStream) {
    let mut byte = [0];
    match socket.read(&mut byte) {
        Ok(0) => {}
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
        other => panic!("socket was not retired locally: {other:?}"),
    }
}
