use clinkz_wot_binding_http::*;
use clinkz_wot_core::{finite::*, host_accounts::HostResourceAccounts, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_servient::{host_producer::*, producer_composition::ComposedProducerExposure};
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

fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
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
        retained_results: 3,
        candidate_bytes: 128 * 1024,
    }
}
fn bank() -> Arc<HostResourceAccounts> {
    Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS))
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
    accounts: Arc<HostResourceAccounts>,
    acquired: Acquired,
) -> HostProducer {
    // The port-zero registration uniquely owns the draft relative Form. Each
    // final candidate has a different draft address and the same acquired socket.
    let mut registrations = vec![
        FiniteProducerRegistration::fixed(udp),
        FiniteProducerRegistration::fixed(draft()),
    ];
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
        4096,
        limits(),
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts,
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
fn expose(
    host: &mut HostProducer,
    td: &Value,
    names: &[&str],
    value: u8,
    count: &Rc<Cell<usize>>,
    replaces: Option<ProducerInstanceKey>,
) -> CoreResult<ComposedProducerExposure> {
    let behaviors = names
        .iter()
        .map(|&name| {
            RawFiniteBehavior::new(
                AffordanceTarget::Property(name.into()),
                Operation::ReadProperty,
                Reading(value, count.clone()),
            )
        })
        .collect();
    host.expose(
        td.to_string().into(),
        behaviors,
        ExposurePolicy {
            replaces,
            handler_backing_bytes: 32,
        },
        &mut work(),
    )
}
fn step(host: &mut HostProducer, exposure: &mut ComposedProducerExposure) {
    host.step(
        exposure,
        &mut Context::from_waker(Waker::noop()),
        MonotonicInstant::new(ClockId::new(1), 0),
        |_, _| work(),
    )
    .unwrap();
}
fn publish(host: &mut HostProducer, exposure: &mut ComposedProducerExposure) {
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        if let Poll::Ready(result) = host.poll_publish(
            exposure,
            &mut Context::from_waker(Waker::noop()),
            MonotonicInstant::new(ClockId::new(1), 0),
            &mut work(),
            |_, _| work(),
        ) {
            result.unwrap();
            return;
        }
        assert!(Instant::now() < end, "closed installation stalled");
    }
}
fn read_http(
    host: &mut HostProducer,
    exposure: &mut ComposedProducerExposure,
) -> (String, Box<ProducerFiniteSettlement>) {
    (http_wire(host, exposure), outcome(host, exposure))
}
fn http_wire(host: &mut HostProducer, exposure: &mut ComposedProducerExposure) -> String {
    let td: Value = serde_json::from_str(host.description(exposure.key()).unwrap()).unwrap();
    let address = addr(td["base"].as_str().unwrap());
    let path = td["properties"]["relative"]["forms"][0]["href"]
        .as_str()
        .unwrap();
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
fn read_udp(
    host: &mut HostProducer,
    exposure: &mut ComposedProducerExposure,
    address: SocketAddr,
) -> u8 {
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
fn outcome(
    host: &mut HostProducer,
    exposure: &mut ComposedProducerExposure,
) -> Box<ProducerFiniteSettlement> {
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        step(host, exposure);
        if let Some(result) = exposure.take_finite_settlement() {
            return result;
        }
        assert!(Instant::now() < end, "finite Read stalled");
    }
}
fn retire(host: &mut HostProducer, exposure: &mut ComposedProducerExposure) {
    exposure.begin_close();
    for _ in 0..16 {
        step(host, exposure);
        if exposure.members_retired() {
            break;
        }
    }
    host.retire(exposure, &mut work()).unwrap();
}
fn empty(bank: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(bank.account(k).used(), 0, "{k:?}");
    }
}

#[test]
fn either_final_owner_serves_the_resolved_form_and_the_independent_absolute_sibling() {
    for id in [21, 22] {
        let accounts = bank();
        let acquired = Acquired::default();
        let udp = udp::Datagram::new();
        let absolute = udp.address();
        let mut host = deployment(udp, &[id], accounts.clone(), acquired.clone());
        let count = Rc::new(Cell::new(0));
        let td = source(true, Some(absolute));
        let mut thing =
            expose(&mut host, &td, &["relative", "absolute"], 42, &count, None).unwrap();
        assert!(host.description(thing.key()).is_none());
        assert_eq!(acquired.borrow().len(), 1);
        let staged = accounts
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used();
        publish(&mut host, &mut thing);
        assert_eq!(
            accounts
                .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
                .used(),
            0
        );
        assert!(
            accounts
                .account(ResourceKind::EngineLiveBytesGlobalMax)
                .used()
                < staged
        );
        let actual: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
        let mut expected = td.clone();
        expected["base"] = json!(format!("http://{}/", acquired.borrow()[0]));
        assert_eq!(actual, expected);
        let (body, retained) = read_http(&mut host, &mut thing);
        assert_eq!(body, "42");
        assert_eq!(retained.binding().0, BindingId::new(id));
        assert_eq!(read_udp(&mut host, &mut thing, absolute), 42);
        assert_eq!(count.get(), 2);
        // A second HTTP exchange finishes on the same member. Leave its
        // settlement unread throughout cutover and the successor's first Read.
        assert_eq!(http_wire(&mut host, &mut thing), "42");
        let key = thing.key();
        let mut next = expose(
            &mut host,
            &source(true, None),
            &["relative"],
            7,
            &count,
            Some(key),
        )
        .unwrap();
        let old_revocation = host.revocation(key).unwrap();
        publish(&mut host, &mut next);
        old_revocation.revoke();
        assert!(host.description(key).is_none());
        assert_eq!(read_http(&mut host, &mut next).0, "7");
        let unread = thing.take_finite_settlement().unwrap();
        assert_eq!(unread.recipient(), Some(key));
        assert_eq!(unread.result().unwrap().data().unwrap().body, b"42");
        assert_eq!(unread.delivery(), retained.delivery());
        assert_eq!(unread.effect(), retained.effect());
        assert!(unread.sequence() > retained.sequence());
        drop(unread);
        retire(&mut host, &mut thing);
        retire(&mut host, &mut next);
        drop((old_revocation, thing, next, host));
        assert_eq!(retained.recipient(), Some(key));
        assert_eq!(retained.result().unwrap().data().unwrap().body, b"42");
        assert_eq!(retained.delivery(), FiniteDelivery::Delivered);
        assert!(retained.first_cause().is_none());
        assert!(accounts.account(ResourceKind::CleanupBytesMax).used() > 0);
        drop(retained);
        empty(&accounts);
    }
}

#[test]
fn draft_refusal_acquires_nothing_and_final_ambiguity_rolls_back_only_the_candidate() {
    let accounts = bank();
    let acquired = Acquired::default();
    let udp = udp::Datagram::new();
    let address = udp.address();
    let mut host = deployment(udp, &[21, 22], accounts.clone(), acquired.clone());
    let count = Rc::new(Cell::new(0));
    let td = source(true, Some(address));
    assert!(matches!(
        expose(&mut host, &td, &["absolute"], 7, &count, None),
        Err(CoreError::UnsupportedOperation(_))
    ));
    assert!(acquired.borrow().is_empty());
    let mut old = expose(
        &mut host,
        &source(false, Some(address)),
        &["absolute"],
        5,
        &count,
        None,
    )
    .unwrap();
    publish(&mut host, &mut old);
    let old_key = old.key();
    let before = accounts
        .account(ResourceKind::EngineLiveBytesGlobalMax)
        .used();
    assert!(matches!(
        expose(
            &mut host,
            &td,
            &["absolute", "relative"],
            7,
            &count,
            Some(old_key)
        ),
        Err(CoreError::UnsupportedOperation(_))
    ));
    assert_eq!(acquired.borrow().len(), 2);
    TcpListener::bind(acquired.borrow()[1]).expect("refused lease released");
    assert_eq!(
        accounts
            .account(ResourceKind::EngineLiveBytesGlobalMax)
            .used(),
        before
    );
    assert_eq!(
        accounts
            .account(ResourceKind::AdmissionTemporaryBytesGlobalMax)
            .used(),
        0
    );
    assert_eq!(count.get(), 0);
    assert_eq!(read_udp(&mut host, &mut old, address), 5);
    retire(&mut host, &mut old);
    drop((old, host));
    empty(&accounts);
}

#[test]
fn cancelled_closed_installations_release_their_original_ports() {
    let accounts = bank();
    let acquired = Acquired::default();
    let mut host = deployment(
        udp::Datagram::new(),
        &[21],
        accounts.clone(),
        acquired.clone(),
    );
    let count = Rc::new(Cell::new(0));
    let td = source(true, None);
    let mut old = expose(&mut host, &td, &["relative"], 11, &count, None).unwrap();
    publish(&mut host, &mut old);
    for ready in [false, true] {
        let mut candidate =
            expose(&mut host, &td, &["relative"], 8, &count, Some(old.key())).unwrap();
        let address = *acquired.borrow().last().unwrap();
        let mut peer = TcpStream::connect(address).unwrap();
        write!(peer, "GET /level HTTP/1.1\r\nHost: {address}\r\n\r\n").unwrap();
        if ready {
            step(&mut host, &mut candidate);
            step(&mut host, &mut candidate);
        }
        assert_eq!(count.get(), usize::from(ready));
        assert!(host.description(candidate.key()).is_none());
        retire(&mut host, &mut candidate);
        assert!(matches!(
            host.poll_publish(
                &mut candidate,
                &mut Context::from_waker(Waker::noop()),
                MonotonicInstant::new(ClockId::new(1), 0),
                &mut work(),
                |_, _| work()
            ),
            Poll::Ready(Err(_))
        ));
        TcpListener::bind(address)
            .expect("original socket released even while candidate is retained");
        assert_eq!(read_http(&mut host, &mut old).0, "11");
    }
    retire(&mut host, &mut old);
    drop((old, host));
    empty(&accounts);
}

#[test]
fn fixed_downstream_registration_serves_the_supplied_td_verbatim() {
    let accounts = bank();
    let service = udp::Datagram::new();
    let address = service.address();
    let original = format!("\n{}\n", source(false, Some(address)));
    let mut host = HostProducer::new(
        vec![FiniteProducerRegistration::fixed(service)],
        limits(),
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
    )
    .unwrap();
    let count = Rc::new(Cell::new(0));
    let mut thing = host
        .expose(
            original.clone().into(),
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property("absolute".into()),
                Operation::ReadProperty,
                Reading(19, count.clone()),
            )],
            ExposurePolicy::default(),
            &mut work(),
        )
        .unwrap();
    publish(&mut host, &mut thing);
    assert_eq!(host.description(thing.key()), Some(original.as_str()));
    assert_eq!(read_udp(&mut host, &mut thing, address), 19);
    retire(&mut host, &mut thing);
    drop((thing, host));
    empty(&accounts);
}

#[test]
fn finalization_and_rollback_use_the_original_candidates_prepaid_bank() {
    use ResourceKind::*;
    let accounts = bank();
    let uses = Rc::new(Cell::new(0));
    let pressure = Rc::new(RefCell::new(Vec::new()));
    let mut host = HostProducer::with_assigned_base(
        vec![FiniteProducerRegistration::at_base(
            draft(),
            HttpFiniteProducer::at_endpoint,
        )],
        {
            let accounts = accounts.clone();
            let uses = uses.clone();
            let pressure = pressure.clone();
            move || {
                uses.set(uses.get() + 1);
                if uses.get() == 2 {
                    for kind in [
                        EngineLiveBytesGlobalMax,
                        CleanupBytesMax,
                        AdmissionTemporaryBytesGlobalMax,
                        RouteReadinessTokensGlobalMax,
                        ThingsGlobalMax,
                        CleanupItemsMax,
                    ] {
                        let account = accounts.account(kind);
                        pressure.borrow_mut().push(
                            accounts
                                .reserve(kind, account.limit() - account.used())
                                .unwrap(),
                        );
                    }
                }
                HttpProducerEndpoint::acquire(zero())
            }
        },
        4096,
        limits(),
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
    )
    .unwrap();
    let count = Rc::new(Cell::new(0));
    let td = source(true, None);
    let behavior = || {
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property("relative".into()),
            Operation::ReadProperty,
            Reading(1, count.clone()),
        )]
    };
    assert!(matches!(
        host.expose(
            td.to_string().into(),
            behavior(),
            ExposurePolicy::default(),
            &mut work().with_remaining(WorkClass::CleanupItems, 0)
        ),
        Err(CoreError::Backpressure(_))
    ));
    assert!(matches!(
        host.expose(
            td.to_string().into(),
            behavior(),
            ExposurePolicy {
                handler_backing_bytes: limits().candidate_bytes,
                ..Default::default()
            },
            &mut work()
        ),
        Err(CoreError::Backpressure(_))
    ));
    assert_eq!(uses.get(), 0);

    let mut first = expose(&mut host, &td, &["relative"], 17, &count, None).unwrap();
    let mut second = expose(&mut host, &td, &["relative"], 23, &count, None).unwrap();
    assert_eq!(uses.get(), 2);
    let full = accounts.account(EngineLiveBytesGlobalMax);
    assert_eq!(full.used(), full.limit());
    let short = host.poll_publish(
        &mut first,
        &mut Context::from_waker(Waker::noop()),
        MonotonicInstant::new(ClockId::new(1), 0),
        &mut WorkBudget::new(),
        |_, _| work(),
    );
    assert!(short.is_pending());
    assert!(host.description(first.key()).is_none());
    publish(&mut host, &mut first);
    assert!(accounts.account(EngineLiveBytesGlobalMax).used() < full.used());
    retire(&mut host, &mut second);
    assert!(host.description(second.key()).is_none());
    pressure.borrow_mut().clear();
    assert_eq!(read_http(&mut host, &mut first).0, "17");
    retire(&mut host, &mut first);
    drop((first, second, host));
    empty(&accounts);
}
