#![cfg(feature = "prepare")]
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_servient::host_producer::*;
use clinkz_wot_servient_finite::ProducerFiniteSettlement;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
#[path = "support/exposure_udp.rs"]
mod udp;

fn zero() -> SocketAddr {
    ([127, 0, 0, 1], 0).into()
}
fn limits() -> HostProducerLimits {
    HostProducerLimits {
        registry: RawProducerRegistryLimits {
            instances: 4,
            behaviors_per_instance: 4,
            target_name_bytes: 256,
        },
        exchange: RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 0,
            output_body_bytes: 256,
            output_media_capacity: 64,
            output_retained_bytes: 2048,
        },
        active_exchanges: 1,
        shared_handler_quota: None,
        retained_results: 3,
        candidates: 4,
        members: 4,
    }
}
fn draft() -> HttpFiniteProducer {
    HttpFiniteProducer::for_assigned_port(
        zero().ip(),
        BindingId::new(10),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap()
}
type Acquired = Rc<RefCell<Vec<SocketAddr>>>;
fn addr(base: &str) -> SocketAddr {
    base.strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/')
        .parse()
        .unwrap()
}
fn deployment(
    udp: udp::Datagram,
    finals: &[u32],
    members: usize,
    acquired: Acquired,
) -> HostProducer {
    // No registration matches the relative HTTP Form at its provisional port.
    // Each eligible HTTP service specializes to the real closed endpoint.
    let mut registrations = vec![FiniteProducerRegistration::fixed(udp)];
    for &id in finals {
        let provider = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(([127, 0, 0, 1], 8080).into()).unwrap(),
            BindingId::new(id),
            BindingGeneration::INITIAL,
            100,
        )
        .unwrap();
        registrations.push(FiniteProducerRegistration::at_base(
            provider,
            HttpFiniteProducer::at_endpoint,
        ));
    }
    HostProducer::with_assigned_base(
        registrations,
        move || {
            let endpoint = HttpProducerEndpoint::acquire(zero())?;
            acquired.borrow_mut().push(addr(endpoint.as_ref()));
            Ok(endpoint)
        },
        HostProducerLimits {
            members,
            ..limits()
        },
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap()
}
fn source(relative: bool, absolute: Option<SocketAddr>) -> Value {
    let mut properties = json!({});
    if relative {
        properties["relative"] =
            json!({"type":"integer","readOnly":true,"forms":[{"href":"level"}]});
    }
    if let Some(address) = absolute {
        properties["absolute"] = json!({"type":"integer","readOnly":true,"forms":[{"href":format!("udp://{address}/read")}]});
    }
    json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"exposure","base":"http://127.0.0.1:0/",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},"properties":properties})
}
struct Reading(u8, Rc<Cell<usize>>);
impl RawFiniteHandler for Reading {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        self.1.set(self.1.get() + 1);
        Ok(InteractionOutput::with_data(Payload::new(
            self.0.to_string().as_bytes(),
            "application/json",
        )))
    }
}
fn read_behaviors(names: &[&str], value: u8, count: &Rc<Cell<usize>>) -> Vec<RawFiniteBehavior> {
    names
        .iter()
        .map(|&name| {
            RawFiniteBehavior::new(
                AffordanceTarget::Property(name.into()),
                Operation::ReadProperty,
                Reading(value, count.clone()),
            )
        })
        .collect()
}
fn expose(
    host: &mut HostProducer,
    td: &Value,
    names: &[&str],
    value: u8,
    count: &Rc<Cell<usize>>,
    replaces: Option<ProducerInstanceKey>,
) -> CoreResult<HostExposure> {
    host.expose(
        td.to_string().into(),
        read_behaviors(names, value, count),
        ExposurePolicy {
            replaces,
            ..Default::default()
        },
    )
    .map_err(|e| e.into_parts().1.into_parts().1)
}
fn now(ticks: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), ticks)
}
fn step_at(host: &mut HostProducer, exposure: &mut HostExposure, time: MonotonicInstant) {
    host.step(exposure, &mut Context::from_waker(Waker::noop()), time)
        .unwrap();
}
fn step(host: &mut HostProducer, exposure: &mut HostExposure) {
    step_at(host, exposure, now(0));
}
fn publish(host: &mut HostProducer, exposure: &mut HostExposure) {
    publication_result(host, exposure).unwrap();
}
fn publication_result(host: &mut HostProducer, exposure: &mut HostExposure) -> CoreResult<()> {
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        if let Poll::Ready(result) = host.poll_publish(
            exposure,
            &mut Context::from_waker(Waker::noop()),
            MonotonicInstant::new(ClockId::new(1), 0),
        ) {
            return result;
        }
        assert!(Instant::now() < end, "closed installation stalled");
    }
}
fn read_http(
    host: &mut HostProducer,
    exposure: &mut HostExposure,
) -> (String, ProducerFiniteSettlement) {
    (http_wire(host, exposure), outcome(host, exposure))
}
fn http_wire(host: &mut HostProducer, exposure: &mut HostExposure) -> String {
    let td: Value = serde_json::from_str(host.description(exposure.key()).unwrap()).unwrap();
    let address = addr(td["base"].as_str().unwrap());
    let path = td["properties"]["relative"]["forms"][0]["href"]
        .as_str()
        .unwrap();
    get(host, exposure, address, path)
}
fn get(
    host: &mut HostProducer,
    exposure: &mut HostExposure,
    address: SocketAddr,
    path: &str,
) -> String {
    let mut peer = TcpStream::connect(address).unwrap();
    write!(
        peer,
        "GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    peer.set_nonblocking(true).unwrap();
    let mut reply = Vec::new();
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        step(host, exposure);
        let mut bytes = [0; 256];
        match peer.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => reply.extend_from_slice(&bytes[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("HTTP Read failed: {e}"),
        }
        assert!(Instant::now() < end, "HTTP Read stalled");
    }
    let reply = String::from_utf8(reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 200"));
    reply.split_once("\r\n\r\n").unwrap().1.to_owned()
}
fn read_udp(host: &mut HostProducer, exposure: &mut HostExposure, address: SocketAddr) -> u8 {
    let peer = UdpSocket::bind(zero()).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    peer.send_to(b"READ", address).unwrap();
    let result = outcome(host, exposure);
    assert_eq!(result.binding().0, BindingId::new(70));
    assert_eq!(result.recipient(), Some(exposure.key()));
    let mut bytes = [0; 16];
    let len = peer.recv(&mut bytes).unwrap();
    serde_json::from_slice(&bytes[..len]).unwrap()
}
fn outcome(host: &mut HostProducer, exposure: &mut HostExposure) -> ProducerFiniteSettlement {
    outcome_at(host, exposure, now(0))
}
fn outcome_at(
    host: &mut HostProducer,
    exposure: &mut HostExposure,
    time: MonotonicInstant,
) -> ProducerFiniteSettlement {
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        step_at(host, exposure, time);
        if let Some(result) = exposure.take_finite_settlement() {
            return result;
        }
        assert!(Instant::now() < end, "finite Read stalled");
    }
}
fn retire(host: &mut HostProducer, exposure: &mut HostExposure) {
    exposure.begin_close();
    for _ in 0..16 {
        step(host, exposure);
        if exposure.members_retired() {
            break;
        }
    }
    host.retire(exposure).unwrap();
}

#[test]
fn alternative_final_bindings_serve_both_wire_forms_and_keep_original_results_through_replacement()
{
    for id in [21, 22] {
        let acquired = Acquired::default();
        let udp = udp::Datagram::new();
        let absolute = udp.address();
        let mut host = deployment(udp, &[id], 3, acquired.clone());
        let count = Rc::new(Cell::new(0));
        let td = source(true, Some(absolute));
        let mut thing =
            expose(&mut host, &td, &["relative", "absolute"], 42, &count, None).unwrap();
        assert!(acquired.borrow().is_empty());
        publish(&mut host, &mut thing);
        let actual: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
        let mut expected = td;
        expected["base"] = json!(format!("http://{}/", acquired.borrow()[0]));
        assert_eq!(actual, expected);
        let (value, first) = read_http(&mut host, &mut thing);
        assert_eq!(value, "42");
        assert_eq!(first.binding().0, BindingId::new(id));
        assert_eq!(read_udp(&mut host, &mut thing, absolute), 42);
        assert_eq!(http_wire(&mut host, &mut thing), "42");
        let old = thing.key();
        let mut next = expose(
            &mut host,
            &source(true, None),
            &["relative"],
            7,
            &count,
            Some(old),
        )
        .unwrap();
        publish(&mut host, &mut next);
        assert!(host.description(old).is_none());
        assert_eq!(read_http(&mut host, &mut next).0, "7");
        let second = thing.take_finite_settlement().unwrap();
        retire(&mut host, &mut thing);
        retire(&mut host, &mut next);
        drop((thing, next, host));
        for result in [first, second] {
            assert_eq!(result.recipient(), Some(old));
            assert_eq!(result.delivery(), FiniteDelivery::Delivered);
            assert_eq!(result.result().unwrap().data().unwrap().body, b"42");
        }
    }
}
#[test]
fn last_exact_member_contends_after_both_endpoints_exist_and_refusal_returns_unique_inputs() {
    let acquired = Acquired::default();
    let udp = udp::Datagram::new();
    let address = udp.address();
    let mut host = deployment(udp, &[21], 2, acquired.clone());
    let count = Rc::new(Cell::new(0));
    let mut incumbent = expose(
        &mut host,
        &source(false, Some(address)),
        &["absolute"],
        3,
        &count,
        None,
    )
    .unwrap();
    publish(&mut host, &mut incumbent);
    let td = source(true, None);
    let mut winner = expose(&mut host, &td, &["relative"], 7, &count, None).unwrap();
    let mut loser = expose(&mut host, &td, &["relative"], 9, &count, None).unwrap();
    for candidate in [&mut winner, &mut loser] {
        assert!(tick(&mut host, candidate).is_pending());
    }
    let ports = acquired.borrow().clone();
    for address in &ports[1..] {
        assert!(TcpListener::bind(address).is_err());
    }
    publish(&mut host, &mut winner);
    assert!(matches!(
        publication_result(&mut host, &mut loser),
        Err(CoreError::Backpressure(_))
    ));
    assert!(matches!(
        tick(&mut host, &mut loser),
        Poll::Ready(Err(CoreError::Backpressure(_)))
    ));
    TcpListener::bind(ports[2]).unwrap();
    assert_eq!(read_http(&mut host, &mut winner).0, "7");
    assert_eq!(read_udp(&mut host, &mut incumbent, address), 3);
    let input = loser.into_input().ok().unwrap();
    assert_eq!(&*input.description, td.to_string());
    retire(&mut host, &mut winner);
    let mut recovered = host
        .expose(input.description, input.behaviors, input.policy)
        .ok()
        .unwrap();
    publish(&mut host, &mut recovered);
    assert_eq!(read_http(&mut host, &mut recovered).0, "9");
    assert_eq!(count.get(), 3);
}
fn tick(host: &mut HostProducer, thing: &mut HostExposure) -> Poll<CoreResult<()>> {
    host.poll_publish(
        thing,
        &mut Context::from_waker(Waker::noop()),
        MonotonicInstant::new(ClockId::new(1), 0),
    )
}
#[test]
fn ambiguous_final_and_missing_behavior_release_only_the_candidate() {
    let acquired = Acquired::default();
    let udp = udp::Datagram::new();
    let address = udp.address();
    let mut host = deployment(udp, &[20, 21], 3, acquired.clone());
    let count = Rc::new(Cell::new(0));
    let mut incumbent = expose(
        &mut host,
        &source(false, Some(address)),
        &["absolute"],
        3,
        &count,
        None,
    )
    .unwrap();
    publish(&mut host, &mut incumbent);
    let mut missing = expose(
        &mut host,
        &source(true, Some(address)),
        &["absolute"],
        2,
        &count,
        None,
    )
    .unwrap();
    assert!(publication_result(&mut host, &mut missing).is_err());
    assert_eq!(acquired.borrow().len(), 1);
    let mut extra = expose(
        &mut host,
        &source(false, Some(address)),
        &["absolute", "unused"],
        2,
        &count,
        None,
    )
    .unwrap();
    assert!(matches!(
        publication_result(&mut host, &mut extra),
        Err(CoreError::UnsupportedOperation(_))
    ));
    let mut invalid_td = source(true, None);
    invalid_td["properties"]["relative"]["type"] = json!("invalid");
    let mut invalid = expose(&mut host, &invalid_td, &["relative"], 2, &count, None).unwrap();
    assert!(publication_result(&mut host, &mut invalid).is_err());
    assert_eq!(acquired.borrow().len(), 1);
    let mut contender = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        7,
        &count,
        None,
    )
    .unwrap();
    assert!(publication_result(&mut host, &mut contender).is_err());
    TcpListener::bind(acquired.borrow()[1]).unwrap();
    assert!(contender.publication_failure().is_some());
    assert_eq!(read_udp(&mut host, &mut incumbent, address), 3);
}

fn http_registration(listen: HttpProducerConfig, id: u32) -> HttpFiniteProducer {
    HttpFiniteProducer::try_new(listen, BindingId::new(id), BindingGeneration::INITIAL, 100)
        .unwrap()
}

#[test]
fn fixed_registration_shares_one_listener_across_independent_publications() {
    let socket = TcpListener::bind(zero()).unwrap();
    let address = socket.local_addr().unwrap();
    let listen = HttpProducerConfig::for_loopback(address).unwrap();
    let listener = HttpFiniteListener::new(listen, 100).unwrap();
    let mismatch = HttpFiniteListener::new(listen, 101).unwrap();
    assert!(
        http_registration(listen, 10)
            .with_listener(&mismatch)
            .is_err()
    );
    assert!(draft().with_listener(&listener).is_err());
    let fixed = http_registration(listen, 10)
        .with_listener(&listener)
        .unwrap();
    let mut host = HostProducer::new(
        vec![FiniteProducerRegistration::fixed(fixed)],
        limits(),
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap();
    let make_td = |path: &str| {
        let mut td = source(true, None);
        td["base"] = json!(format!("http://{address}/"));
        td["properties"]["relative"]["forms"][0]["href"] = json!(path);
        td
    };
    let calls = Rc::new(Cell::new(0));
    let mut alpha = expose(
        &mut host,
        &make_td("alpha"),
        &["relative"],
        11,
        &calls,
        None,
    )
    .unwrap();
    let mut beta = expose(&mut host, &make_td("beta"), &["relative"], 22, &calls, None).unwrap();
    drop(socket);
    publish(&mut host, &mut alpha);
    publish(&mut host, &mut beta);
    let (value, unread) = read_http(&mut host, &mut alpha);
    assert_eq!(value, "11");
    assert_eq!(unread.recipient(), Some(alpha.key()));
    let mut collision = expose(
        &mut host,
        &make_td("alpha"),
        &["relative"],
        33,
        &calls,
        None,
    )
    .unwrap();
    assert!(publication_result(&mut host, &mut collision).is_err());
    assert!(host.description(collision.key()).is_none());
    assert!(collision.publication_failure().is_some());
    assert_eq!(read_http(&mut host, &mut beta).0, "22");
    drop(unread);
    retire(&mut host, &mut alpha);
    assert!(TcpListener::bind(address).is_err());
    assert_eq!(read_http(&mut host, &mut beta).0, "22");
    assert_eq!(calls.get(), 3);
    retire(&mut host, &mut beta);
    TcpListener::bind(address).expect("final route released the shared socket");
    drop((alpha, beta, collision, host, listener));
}

#[test]
fn selected_members_can_split_after_acquisition_and_two_reads_fit_one_cleanup_slot() {
    for (split, members) in [(false, 1), (true, 1), (true, 2)] {
        let fixed_socket = TcpListener::bind(zero()).unwrap();
        let fixed = fixed_socket.local_addr().unwrap();
        let listen = HttpProducerConfig::for_loopback(fixed).unwrap();
        let mut host = HostProducer::with_assigned_base(
            vec![
                FiniteProducerRegistration::fixed(http_registration(listen, 20)),
                FiniteProducerRegistration::at_base(draft(), HttpFiniteProducer::at_endpoint),
                FiniteProducerRegistration::fixed(udp::Datagram::new()),
            ],
            || HttpProducerEndpoint::acquire(zero()),
            HostProducerLimits {
                members,
                ..limits()
            },
            GatewayDefaultV1::LIMITS.clone(),
        )
        .unwrap();
        let count = Rc::new(Cell::new(0));
        let mut td = source(true, None);
        td["base"] = json!(format!("http://{fixed}/"));
        let other = if split {
            format!("http://{fixed}/constant")
        } else {
            "constant".into()
        };
        td["properties"]["other"] =
            json!({"type":"integer","readOnly":true,"forms":[{"href":other}]});
        let mut thing = expose(&mut host, &td, &["relative", "other"], 27, &count, None).unwrap();
        assert!(tick(&mut host, &mut thing).is_pending());
        drop(fixed_socket);
        if split && members == 1 {
            assert!(matches!(
                publication_result(&mut host, &mut thing),
                Err(CoreError::Backpressure(_))
            ));
            assert_eq!(count.get(), 0);
            continue;
        }
        publish(&mut host, &mut thing);
        let (value, result) = read_http(&mut host, &mut thing);
        assert_eq!(value, "27");
        assert_eq!(result.binding().0, BindingId::new(10));
        let td: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
        let address = if split {
            fixed
        } else {
            addr(td["base"].as_str().unwrap())
        };
        assert_eq!(get(&mut host, &mut thing, address, "constant"), "27");
        assert_eq!(
            outcome(&mut host, &mut thing).binding().0,
            BindingId::new(if split { 20 } else { 10 })
        );
    }
}

#[test]
fn passive_staging_foreign_drives_and_terminal_member_failure_preserve_the_original_owners() {
    let acquired = Acquired::default();
    let udp = udp::Datagram::new();
    let mut foreign = deployment(udp::Datagram::new(), &[15], 2, Acquired::default());
    let mut host = deployment(udp.clone(), &[14], 2, acquired.clone());
    let count = Rc::new(Cell::new(0));
    let td = source(true, Some(udp.address()));
    let mut thing = expose(&mut host, &td, &["absolute", "relative"], 7, &count, None).unwrap();
    assert!(matches!(
        tick(&mut foreign, &mut thing),
        Poll::Ready(Err(CoreError::StaleHandle(_)))
    ));
    assert!(host.description(thing.key()).is_none());
    assert_eq!(udp.observations.get(), 0);
    assert!(acquired.borrow().is_empty());
    while udp.starts.get() == 0 {
        assert!(tick(&mut host, &mut thing).is_pending());
    }
    let address = acquired.borrow()[0];
    assert!(TcpListener::bind(address).is_err());
    udp.associated.set(false);
    assert!(publication_result(&mut host, &mut thing).is_err());
    assert!(thing.publication_failure().is_some());
    assert!(host.description(thing.key()).is_none());
    TcpListener::bind(address).unwrap();
    udp.associated.set(true);
    assert!(publication_result(&mut host, &mut thing).is_err());
    let input = thing.into_input().ok().unwrap();
    assert_eq!(&*input.description, td.to_string());
    let mut next = host
        .expose(input.description, input.behaviors, input.policy)
        .ok()
        .unwrap();
    publish(&mut host, &mut next);
    assert_eq!(read_udp(&mut host, &mut next, udp.address()), 7);
    assert_eq!(count.get(), 1);
}

#[test]
fn cancellation_and_expiry_at_closed_native_readiness_release_the_assigned_socket() {
    let acquired = Acquired::default();
    let udp = udp::Datagram::new();
    let mut host = deployment(udp.clone(), &[14], 2, acquired.clone());
    let count = Rc::new(Cell::new(0));
    for polls in [1, 6] {
        for expire in [false, true] {
            let td: Arc<str> = source(true, Some(udp.address())).to_string().into();
            let mut candidate = host
                .expose(
                    td.clone(),
                    read_behaviors(&["relative", "absolute"], 4, &count),
                    ExposurePolicy {
                        deadline: Deadline::at(MonotonicInstant::new(ClockId::new(1), 1)),
                        ..Default::default()
                    },
                )
                .ok()
                .unwrap();
            for _ in 0..polls {
                assert!(tick(&mut host, &mut candidate).is_pending());
            }
            let address = *acquired.borrow().last().unwrap();
            assert!(TcpListener::bind(address).is_err());
            if expire {
                assert!(matches!(
                    host.poll_publish(
                        &mut candidate,
                        &mut Context::from_waker(Waker::noop()),
                        MonotonicInstant::new(ClockId::new(1), 1)
                    ),
                    Poll::Ready(Err(CoreError::TimedOut(_)))
                ));
            } else {
                candidate.begin_close();
            }
            assert!(host.description(candidate.key()).is_none());
            TcpListener::bind(address).unwrap();
            assert!(Arc::ptr_eq(
                &candidate.into_input().ok().unwrap().description,
                &td
            ));
            assert_eq!(count.get(), 0);
        }
    }
}

#[path = "support/allocation_meter.rs"]
mod meter;
#[global_allocator]
static ALLOCATOR: meter::Allocator = meter::Allocator;

#[test]
fn checked_handoff_of_escaped_structured_values_has_no_heap_scratch() {
    use clinkz_wot_core::data_schema::{PreparedSchema, SchemaProperty};
    let schema = PreparedSchema::object(vec![SchemaProperty {
        name: "na\u{8}me".into(),
        required: true,
        schema: PreparedSchema::String {
            min_length: Some(512),
            max_length: Some(512),
        },
    }])
    .unwrap();
    let value = format!(
        r#"{{"n\u0061\bme":"{}","extra":[{{"x":true}}]}}"#,
        "\\u0061".repeat(512)
    );
    let before = meter::read();
    clinkz_wot_codec_json::validate_json(&schema, value.as_bytes()).unwrap();
    let after = meter::read();
    assert_eq!(after.allocations, before.allocations);
    assert_eq!(after.allocated_bytes, before.allocated_bytes);
    assert_eq!(after.live, before.live);
}

#[test]
fn final_result_disposal_enables_a_waiting_http_read_on_the_next_maintenance_turn() {
    let mut host = HostProducer::with_assigned_base(
        vec![FiniteProducerRegistration::at_base(
            draft(),
            HttpFiniteProducer::at_endpoint,
        )],
        || HttpProducerEndpoint::acquire(zero()),
        HostProducerLimits {
            retained_results: 1,
            ..limits()
        },
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let mut thing = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        7,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut thing);
    assert_eq!(http_wire(&mut host, &mut thing), "7");
    let td: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
    let address = addr(td["base"].as_str().unwrap());
    let mut peer = TcpStream::connect(address).unwrap();
    write!(
        peer,
        "GET /level HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    for _ in 0..8 {
        step(&mut host, &mut thing);
    }
    let original = thing.take_finite_settlement().unwrap();
    for _ in 0..8 {
        step(&mut host, &mut thing);
    }
    assert_eq!(calls.get(), 1);
    drop(original);
    assert_eq!(
        outcome(&mut host, &mut thing)
            .result()
            .unwrap()
            .data()
            .unwrap()
            .body,
        b"7"
    );
    let mut wire = String::new();
    peer.read_to_string(&mut wire).unwrap();
    assert!(wire.starts_with("HTTP/1.1 200"));
    assert_eq!(calls.get(), 2);
    let key = thing.key();
    drop(thing);
    TcpListener::bind(address).unwrap();
    host.retire_revoked(key).unwrap();
}

#[test]
fn busy_specialization_retries_the_same_lease_and_observed_provider_drift_is_terminal() {
    struct Endpoint {
        http: HttpProducerEndpoint,
        busy: Rc<Cell<bool>>,
    }
    impl AsRef<str> for Endpoint {
        fn as_ref(&self) -> &str {
            self.http.as_ref()
        }
    }
    let udp = udp::Datagram::new();
    let held = Rc::new(Cell::new(true));
    let addresses = Acquired::default();
    let mut host = HostProducer::with_assigned_base(
        vec![
            FiniteProducerRegistration::fixed(udp.clone()),
            FiniteProducerRegistration::at_base(draft(), |s, e: &Endpoint| {
                if e.busy.get() {
                    return Err(CoreError::Backpressure(ErrorContext::new(
                        ErrorPhase::Prepare,
                        RetryClass::Safe,
                    )));
                }
                s.at_endpoint(&e.http)
            }),
        ],
        {
            let held = held.clone();
            let addresses = addresses.clone();
            move || {
                let http = HttpProducerEndpoint::acquire(zero())?;
                addresses.borrow_mut().push(addr(http.as_ref()));
                Ok(Endpoint {
                    http,
                    busy: held.clone(),
                })
            }
        },
        limits(),
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap();
    let count = Rc::new(Cell::new(0));
    for invalidate in [true, false] {
        held.set(true);
        let mut candidate = expose(
            &mut host,
            &source(true, None),
            &["relative"],
            7,
            &count,
            None,
        )
        .unwrap();
        for _ in 0..6 {
            assert!(tick(&mut host, &mut candidate).is_pending());
        }
        let address = *addresses.borrow().last().unwrap();
        assert!(TcpListener::bind(address).is_err());
        held.set(false);
        if invalidate {
            udp.enabled.borrow_mut().replace(false);
            udp.enabled.borrow_mut().replace(true);
            assert!(matches!(
                publication_result(&mut host, &mut candidate),
                Err(CoreError::StaleHandle(_))
            ));
            TcpListener::bind(address).unwrap();
        } else {
            publish(&mut host, &mut candidate);
            assert_eq!(read_http(&mut host, &mut candidate).0, "7");
        }
    }
    held.set(true);
    let mut exhausted = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        7,
        &count,
        None,
    )
    .unwrap();
    assert!(matches!(
        publication_result(&mut host, &mut exhausted),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(addresses.borrow().len(), 3);
    TcpListener::bind(addresses.borrow()[2])
        .expect("retry exhaustion released the original endpoint");
}

#[test]
fn one_host_dispatch_defers_nested_sibling_drives_and_revocation_is_immediate() {
    struct Nested {
        reading: Reading,
        nested: Box<dyn Fn()>,
    }
    impl RawFiniteHandler for Nested {
        fn authorize(&self, i: &InteractionInput) -> CoreResult<()> {
            self.reading.authorize(i)
        }
        fn handle(
            &self,
            c: HandlerContext<'_>,
            i: &InteractionInput,
        ) -> CoreResult<InteractionOutput> {
            (self.nested)();
            self.reading.handle(c, i)
        }
    }
    type Serving = Rc<RefCell<(HostProducer, HostExposure)>>;
    fn drive(p: &Serving) {
        let mut loan = p.borrow_mut();
        let (host, thing) = &mut *loan;
        step(host, thing);
    }
    fn ready(calls: &Rc<Cell<usize>>, nested: impl Fn() + 'static) -> (Serving, UdpSocket) {
        let udp = udp::Datagram::new();
        let address = udp.address();
        let mut host = HostProducer::new(
            vec![FiniteProducerRegistration::fixed(udp)],
            limits(),
            GatewayDefaultV1::LIMITS.clone(),
        )
        .unwrap();
        let mut thing = host
            .expose(
                source(false, Some(address)).to_string().into(),
                vec![RawFiniteBehavior::new(
                    AffordanceTarget::Property("absolute".into()),
                    Operation::ReadProperty,
                    Nested {
                        reading: Reading(7, calls.clone()),
                        nested: Box::new(nested),
                    },
                )],
                ExposurePolicy::default(),
            )
            .ok()
            .unwrap();
        publish(&mut host, &mut thing);
        let peer = UdpSocket::bind(zero()).unwrap();
        peer.send_to(b"READ", address).unwrap();
        step(&mut host, &mut thing); // original offer is ready for handler entry
        (Rc::new(RefCell::new((host, thing))), peer)
    }
    let calls = Rc::new(Cell::new(0));
    let (third, _) = ready(&calls, || {});
    let revoke = {
        let t = third.borrow();
        t.0.revocation(t.1.key()).unwrap()
    };
    let (second, _) = ready(&calls, {
        let third = third.clone();
        move || drive(&third)
    });
    let (first, _) = ready(&calls, {
        let second = second.clone();
        move || {
            drive(&second);
            drive(&second);
            revoke.revoke();
        }
    });
    drive(&first);
    assert_eq!(calls.get(), 1);
    {
        let t = third.borrow();
        assert!(t.0.description(t.1.key()).is_none());
    }
    drive(&second);
    assert_eq!(calls.get(), 2);
    drive(&third);
    assert_eq!(calls.get(), 2);
}

#[test]
fn host_retires_an_isolated_send_error_without_losing_the_member() {
    let udp = udp::Datagram::new();
    let addr = udp.address();
    let mut host = deployment(udp.clone(), &[], 1, Acquired::default());
    let calls = Rc::new(Cell::new(0));
    let mut thing = expose(
        &mut host,
        &source(false, Some(addr)),
        &["absolute"],
        7,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut thing);
    udp.fail_send.set(true);
    let peer = UdpSocket::bind(zero()).unwrap();
    peer.send_to(b"READ", addr).unwrap();
    let held = outcome(&mut host, &mut thing);
    assert!(matches!(
        held.first_cause(),
        Some(CoreError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        held.cleanup_error(),
        Some(CoreError::Backpressure(_))
    ));
    assert_eq!(held.delivery(), FiniteDelivery::NotDelivered);
    assert_eq!(held.result().unwrap().data().unwrap().body, b"7");
    assert_eq!(udp.retire_attempts.get(), 2);
    assert!(host.description(thing.key()).is_some());
    assert_eq!(read_udp(&mut host, &mut thing, addr), 7);
    assert_eq!(calls.get(), 2);
}

#[test]
fn committed_idle_member_failure_reports_its_original_cause_through_retirement() {
    let udp = udp::Datagram::new();
    let mut host = deployment(udp.clone(), &[20], 2, Acquired::default());
    let calls = Rc::new(Cell::new(0));
    let mut idle = expose(
        &mut host,
        &source(false, Some(udp.address())),
        &["absolute"],
        1,
        &calls,
        None,
    )
    .unwrap();
    let mut independent = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        2,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut idle);
    publish(&mut host, &mut independent);
    udp.fail_ingress.set(true);
    step(&mut host, &mut idle);
    let cause = CoreError::Binding(ErrorContext::new(ErrorPhase::Binding, RetryClass::Safe));
    assert!(host.description(idle.key()).is_none());
    assert_eq!(idle.publication_failure(), Some(&cause));
    assert!(idle.take_finite_settlement().is_none());
    assert_eq!(calls.get(), 0);
    assert_eq!(read_http(&mut host, &mut independent).0, "2");
    retire(&mut host, &mut idle);
    assert_eq!(idle.publication_failure(), Some(&cause));
    assert!(independent.publication_failure().is_none());
}

#[test]
fn assigned_port_selects_the_only_final_registration_despite_an_ambiguous_draft() {
    let acquired = Acquired::default();
    let record = acquired.clone();
    let serving = HttpFiniteProducer::for_assigned_port(
        zero().ip(),
        BindingId::new(12),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let mut host = HostProducer::with_assigned_base(
        vec![
            FiniteProducerRegistration::fixed(draft()),
            FiniteProducerRegistration::at_base(serving, HttpFiniteProducer::at_endpoint),
        ],
        move || {
            let endpoint = HttpProducerEndpoint::acquire(zero())?;
            record.borrow_mut().push(addr(endpoint.as_ref()));
            Ok(endpoint)
        },
        limits(),
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let mut thing = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        7,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut thing);
    let td: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
    assert_eq!(&*acquired.borrow(), &[addr(td["base"].as_str().unwrap())]);
    let (wire, result) = read_http(&mut host, &mut thing);
    assert_eq!(wire, "7");
    assert_eq!(result.binding().0, BindingId::new(12));
    assert_eq!(calls.get(), 1);
}

#[test]
fn two_host_deployments_share_one_handler_permit_through_result_disposal() {
    use clinkz_wot_core::host_accounts::HostResourceAccounts;
    let kind = ResourceKind::PendingHandlerCallsGlobalMax;
    let bank = Arc::new(HostResourceAccounts::new(
        &GatewayDefaultV1::LIMITS.clone().with_limit(kind, Some(1)),
    ));
    let config = || HostProducerLimits {
        shared_handler_quota: Some(bank.clone()),
        ..limits()
    };
    let mut assigned = HostProducer::with_assigned_base(
        vec![FiniteProducerRegistration::at_base(
            draft(),
            HttpFiniteProducer::at_endpoint,
        )],
        || HttpProducerEndpoint::acquire(zero()),
        config(),
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap();
    let endpoint = HttpProducerEndpoint::acquire(zero()).unwrap();
    let address = addr(endpoint.as_ref());
    let mut fixed_td = source(true, None);
    fixed_td["base"] = json!(endpoint.as_ref());
    let mut fixed = HostProducer::new(
        vec![FiniteProducerRegistration::fixed(
            draft().at_endpoint(&endpoint).unwrap(),
        )],
        config(),
        GatewayDefaultV1::LIMITS.clone(),
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let mut first = expose(
        &mut assigned,
        &source(true, None),
        &["relative"],
        1,
        &calls,
        None,
    )
    .unwrap();
    let mut second = expose(&mut fixed, &fixed_td, &["relative"], 2, &calls, None).unwrap();
    publish(&mut assigned, &mut first);
    publish(&mut fixed, &mut second);
    assert_eq!(http_wire(&mut assigned, &mut first), "1");
    assert_eq!(bank.account(kind).used(), 1);

    let mut peer = TcpStream::connect(address).unwrap();
    write!(
        peer,
        "GET /level HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    for _ in 0..8 {
        step(&mut fixed, &mut second);
    }
    let first_result = first.take_finite_settlement().unwrap();
    for _ in 0..8 {
        step(&mut fixed, &mut second);
    }
    assert_eq!(calls.get(), 1);
    assert!(second.take_finite_settlement().is_none());
    assert_eq!(bank.account(kind).used(), 1);
    drop(first_result);
    assert_eq!(bank.account(kind).used(), 0);
    let second_result = outcome(&mut fixed, &mut second);
    assert_eq!(bank.account(kind).used(), 1);
    assert_eq!(calls.get(), 2);
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut wire = String::new();
    peer.read_to_string(&mut wire).unwrap();
    assert_eq!(wire.split_once("\r\n\r\n").unwrap().1, "2");
    drop(second_result);
    assert_eq!(bank.account(kind).used(), 0);
}

#[test]
fn exposure_clock_fences_a_late_http_reply_across_member_turns() {
    let binding = ErrorContext::new(ErrorPhase::Binding, RetryClass::Never);
    for (next_udp, next_http, cause) in [
        (now(150), now(3), CoreError::InternalInvariant(binding)),
        (
            now(150),
            MonotonicInstant::new(ClockId::new(2), 151),
            CoreError::InternalInvariant(binding),
        ),
        (now(1), now(3), CoreError::InternalInvariant(binding)),
        (now(150), now(151), CoreError::TimedOut(binding)),
    ] {
        let udp = udp::Datagram::new();
        let acquired = Acquired::default();
        let mut host = deployment(udp.clone(), &[20], 2, acquired.clone());
        let calls = Rc::new(Cell::new(0));
        let mut thing = expose(
            &mut host,
            &source(true, Some(udp.address())),
            &["relative", "absolute"],
            7,
            &calls,
            None,
        )
        .unwrap();
        publish(&mut host, &mut thing);
        let address = acquired.borrow()[0];
        let mut peer = TcpStream::connect(address).unwrap();
        write!(
            peer,
            "GET /level HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        while calls.get() == 0 {
            step_at(&mut host, &mut thing, now(2));
            assert!(Instant::now() < end);
        }
        // The HTTP result is ready with a request deadline of 102. UDP gets the next turn.
        step_at(&mut host, &mut thing, next_udp);
        assert!(thing.take_finite_settlement().is_none());
        step_at(&mut host, &mut thing, next_http);

        let observed = thing.publication_failure().cloned();
        let withdrawn = host.description(thing.key()).is_none();
        let held = outcome_at(&mut host, &mut thing, next_http);
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut wire = String::new();
        peer.read_to_string(&mut wire).unwrap();
        assert!(wire.is_empty(), "unexpected HTTP response {wire:?}");
        let clock_fault = matches!(cause, CoreError::InternalInvariant(_));
        assert_eq!(observed.as_ref(), clock_fault.then_some(&cause));
        assert_eq!(withdrawn, clock_fault);
        assert_eq!(held.first_cause(), Some(&cause));
        assert!(held.cleanup_error().is_none());
        assert_eq!(held.effect(), FiniteEffect::Unknown);
        assert_eq!(held.delivery(), FiniteDelivery::NotDelivered);
        assert_eq!(held.recipient(), Some(thing.key()));
        assert_eq!(held.result().unwrap().data().unwrap().body, b"7");
        assert_eq!(calls.get(), 1);
        thing.begin_close();
        while !thing.members_retired() {
            step_at(&mut host, &mut thing, next_http);
            assert!(Instant::now() < end);
        }
        host.retire(&mut thing).unwrap();
        TcpListener::bind(address).unwrap();
    }
}

#[test]
fn first_serving_turn_keeps_the_commit_clock() {
    let udp = udp::Datagram::new();
    let mut host = deployment(udp.clone(), &[20], 1, Acquired::default());
    let calls = Rc::new(Cell::new(0));
    let mut thing = expose(
        &mut host,
        &source(false, Some(udp.address())),
        &["absolute"],
        7,
        &calls,
        None,
    )
    .unwrap();
    while udp.starts.get() == 0 {
        assert!(tick(&mut host, &mut thing).is_pending());
    }
    assert_eq!(
        host.poll_publish(
            &mut thing,
            &mut Context::from_waker(Waker::noop()),
            now(100)
        ),
        Poll::Ready(Ok(()))
    );
    let peer = UdpSocket::bind(zero()).unwrap();
    peer.send_to(b"READ", udp.address()).unwrap();
    step_at(&mut host, &mut thing, now(2));
    let error =
        CoreError::InternalInvariant(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never));
    assert_eq!(thing.publication_failure(), Some(&error));
    assert!(host.description(thing.key()).is_none());
    step_at(&mut host, &mut thing, now(101));
    assert!(thing.take_finite_settlement().is_none());
    assert_eq!(calls.get(), 0);
    host.retire(&mut thing).unwrap();
    // Another exposure on this Host starts its own clock at zero.
    let mut fresh = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        8,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut fresh);
    assert_eq!(read_http(&mut host, &mut fresh).0, "8");
}

#[test]
fn clock_fault_during_retirement_preserves_the_original_exchange_errors() {
    let udp = udp::Datagram::new();
    let address = udp.address();
    let mut host = deployment(udp.clone(), &[20], 2, Acquired::default());
    let calls = Rc::new(Cell::new(0));
    let mut thing = expose(
        &mut host,
        &source(true, Some(address)),
        &["absolute", "relative"],
        9,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut thing);
    udp.fail_send.set(true);
    let peer = UdpSocket::bind(zero()).unwrap();
    peer.send_to(b"READ", address).unwrap();
    let end = Instant::now() + Duration::from_secs(2);
    while udp.retire_attempts.get() == 0 {
        step_at(&mut host, &mut thing, now(2));
        assert!(Instant::now() < end);
    }
    step_at(&mut host, &mut thing, now(150)); // HTTP
    step_at(&mut host, &mut thing, now(3)); // UDP cleanup
    let error =
        CoreError::InternalInvariant(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Never));
    assert_eq!(thing.publication_failure(), Some(&error));
    let held = outcome_at(&mut host, &mut thing, now(3));
    assert!(matches!(
        held.first_cause(),
        Some(CoreError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        held.cleanup_error(),
        Some(CoreError::Backpressure(_))
    ));
    assert_eq!(held.delivery(), FiniteDelivery::NotDelivered);
    assert_eq!(held.result().unwrap().data().unwrap().body, b"9");
    assert_eq!(calls.get(), 1);
}

#[test]
fn staged_clock_fault_rolls_back_before_and_after_the_first_native_start() {
    let udp = udp::Datagram::new();
    let acquired = Acquired::default();
    let mut host = deployment(udp.clone(), &[20], 2, acquired.clone());
    let calls = Rc::new(Cell::new(0));
    for start_udp in [false, true] {
        let mut thing = expose(
            &mut host,
            &source(true, Some(udp.address())),
            &["absolute", "relative"],
            4,
            &calls,
            None,
        )
        .unwrap();
        let mut cx = Context::from_waker(Waker::noop());
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            assert!(host.poll_publish(&mut thing, &mut cx, now(8)).is_pending());
            if !start_udp || udp.starts.get() == 1 {
                break;
            }
            assert!(Instant::now() < end);
        }
        let error = if start_udp {
            CoreError::InternalInvariant(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
        } else {
            CoreError::Validation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
        };
        assert_eq!(
            host.poll_publish(&mut thing, &mut cx, now(7)),
            Poll::Ready(Err(error.clone()))
        );
        assert_eq!(thing.publication_failure(), Some(&error));
        assert_eq!(thing.into_input().ok().unwrap().behaviors.len(), 2);
        TcpListener::bind(*acquired.borrow().last().unwrap()).unwrap();
        assert_eq!(calls.get(), 0);
    }
}
