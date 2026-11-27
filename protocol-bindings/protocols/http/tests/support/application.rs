// Independent raw TCP peer: no Hyper, httparse or binding transport in the peer.
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
#[path = "consumer_read.rs"]
mod consumer_read;
use clinkz_wot_core::{consumer::ConsumerInstance, host_accounts::HostResourceAccounts};
use clinkz_wot_planning::open_consumer::ConsumerSelector;
use clinkz_wot_servient_finite::consumer::{
    ConsumerAssociation, ConsumerCall, ConsumerCalls, ConsumerSettlement,
    StandaloneConsumerAssociation,
};
use consumer_read::{http_registration, prepare_consumer};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, OnceLock},
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

fn now() -> MonotonicInstant {
    static START: OnceLock<Instant> = OnceLock::new();
    MonotonicInstant::new(
        ClockId::new(600),
        START.get_or_init(Instant::now).elapsed().as_millis() as u64,
    )
}
#[allow(dead_code)] // Used by a subset of the wire and pressure suites.
fn after(millis: u64) -> Deadline {
    Deadline::at(now().checked_add_ticks(millis).unwrap())
}
type Engine = (
    (ConsumerInstance, StandaloneConsumerAssociation),
    ConsumerAssociation,
    ConsumerCalls,
    Arc<HostResourceAccounts>,
);
fn engine(port: u16) -> Engine {
    engine_with_targets(
        format!("http://127.0.0.1:{port}/a?x=1"),
        format!("http://127.0.0.1:{port}/b"),
    )
}
fn engine_with_targets(a: String, b: String) -> Engine {
    let thing: clinkz_wot_td::thing::Thing = serde_json::from_value(serde_json::json!({
        "@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"real HTTP", "id":"urn:real-http",
        "security":["none"], "securityDefinitions":{"none":{"scheme":"nosec"}},
        "properties": { "a":{"readOnly":true,"forms":[{"href":a}]},
                        "b":{"readOnly":true,"forms":[{"href":b}]}}
    }))
    .unwrap();
    consumer(thing, http_registration(), ConsumerSelector::default()).unwrap()
}
fn consumer(
    thing: clinkz_wot_td::thing::Thing,
    registration: clinkz_wot_planning::open_consumer::ConsumerRegistrationInput,
    selector: ConsumerSelector,
) -> CoreResult<Engine> {
    let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let owner = prepare_consumer(&thing, &bank, registration, selector)?;
    drop(thing);
    let handle = owner.1.handle();
    let calls = ConsumerCalls::try_new(bank.clone(), 1)?;
    Ok((owner, handle, calls, bank))
}
fn usage(bank: &HostResourceAccounts) -> [u64; ResourceKind::ALL.len()] {
    ResourceKind::ALL.map(|kind| bank.account(kind).used())
}
fn release(engine: Engine) {
    let (owner, handle, calls, bank) = engine;
    assert!(calls.is_idle());
    drop((handle, calls, owner));
    assert!(usage(&bank).iter().all(|used| *used == 0));
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
fn settled(result: CoreResult<ConsumerSettlement>) -> CoreResult<ConsumerSettlement> {
    let result = result?;
    result.result()?;
    Ok(result)
}
#[allow(dead_code)] // Used by a subset of the wire and pressure suites.
fn take_ready(call: &mut ConsumerCall) -> Option<CoreResult<ConsumerSettlement>> {
    match call.poll(&mut Context::from_waker(Waker::noop()), now()) {
        Poll::Ready(result) => Some(settled(result)),
        Poll::Pending => None,
    }
}
#[allow(dead_code)] // Used by a subset of the wire and pressure suites.
async fn take(call: &mut ConsumerCall) -> CoreResult<ConsumerSettlement> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Poll::Ready(result) = call.poll(&mut Context::from_waker(Waker::noop()), now()) {
            return settled(result);
        }
        assert!(Instant::now() < end, "call stalled");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
#[allow(dead_code)] // Used by a subset of the wire and pressure suites.
async fn take_woken(call: &mut ConsumerCall) -> CoreResult<ConsumerSettlement> {
    settled(
        tokio::time::timeout(
            Duration::from_secs(5),
            std::future::poll_fn(|cx| call.poll(cx, now())),
        )
        .await
        .expect("call made no wake-driven progress"),
    )
}

fn assert_retired(socket: &mut TcpStream) {
    let mut byte = [0];
    match socket.read(&mut byte) {
        Ok(0) => {}
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
        other => panic!("socket was not retired locally: {other:?}"),
    }
}

#[allow(dead_code)] // Used by the framing and allocation suites.
fn json_body(bytes: usize) -> Vec<u8> {
    let mut body = vec![b' '; bytes];
    if let Some(last) = body.last_mut() {
        *last = b'1';
    }
    body
}
