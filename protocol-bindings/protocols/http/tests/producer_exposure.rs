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
) -> CoreResult<ComposedProducerExposure> {
    host.expose(
        td.to_string().into(),
        read_behaviors(names, value, count),
        ExposurePolicy {
            replaces,
            handler_backing_bytes: 32,
        },
        &mut work(),
    )
    .map_err(|e| e.error().clone())
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
    publication_result(host, exposure).unwrap();
}
fn publication_result(
    host: &mut HostProducer,
    exposure: &mut ComposedProducerExposure,
) -> CoreResult<()> {
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        if let Poll::Ready(result) = host.poll_publish(
            exposure,
            &mut Context::from_waker(Waker::noop()),
            MonotonicInstant::new(ClockId::new(1), 0),
            &mut work(),
            |_, _| work(),
        ) {
            return result;
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
    get(host, exposure, address, path)
}
fn get(
    host: &mut HostProducer,
    exposure: &mut ComposedProducerExposure,
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
        Err(e) if matches!(e.error(), CoreError::Backpressure(_))
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
        Err(e) if matches!(e.error(), CoreError::LimitExceeded { resource: EngineLiveBytesGlobalMax, .. })
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

fn http_registration(listen: HttpProducerConfig, id: u32) -> HttpFiniteProducer {
    HttpFiniteProducer::try_new(listen, BindingId::new(id), BindingGeneration::INITIAL, 100)
        .unwrap()
}

#[test]
fn assigned_base_prepays_a_split_final_membership_and_refunds_unused_slots() {
    use ResourceKind::{CleanupItemsMax, RouteReadinessTokensGlobalMax};
    for (split, allowance) in [(true, 2), (false, 2), (false, 1)] {
        let accounts = bank();
        let fixed_socket = TcpListener::bind(zero()).unwrap();
        let fixed = fixed_socket.local_addr().unwrap();
        let listen = HttpProducerConfig::for_loopback(fixed).unwrap();
        let pressure = Rc::new(RefCell::new(Vec::new()));
        let mut host = HostProducer::with_assigned_base(
            vec![
                FiniteProducerRegistration::fixed(http_registration(listen, 20)),
                FiniteProducerRegistration::at_base(draft(), HttpFiniteProducer::at_endpoint),
            ],
            {
                let accounts = accounts.clone();
                let pressure = pressure.clone();
                move || {
                    assert_eq!(
                        accounts.account(RouteReadinessTokensGlobalMax).used(),
                        allowance
                    );
                    // One cleanup item per member, the logical Thing and the closed base.
                    assert_eq!(accounts.account(CleanupItemsMax).used(), allowance + 2);
                    for kind in [RouteReadinessTokensGlobalMax, CleanupItemsMax] {
                        let a = accounts.account(kind);
                        pressure
                            .borrow_mut()
                            .push(accounts.reserve(kind, a.limit() - a.used()).unwrap());
                    }
                    HttpProducerEndpoint::acquire(zero())
                }
            },
            4096,
            limits(),
            Arc::new(GatewayDefaultV1::LIMITS.clone().with_limit(
                ResourceKind::RouteReadinessTokensPerThingMax,
                Some(allowance),
            )),
            accounts.clone(),
        )
        .unwrap();
        let count = Rc::new(Cell::new(0));
        let mut td = source(true, None);
        td["base"] = json!(format!("http://{fixed}/"));
        let other = if split {
            format!("http://{fixed}/constant")
        } else {
            "constant".to_owned()
        };
        td["properties"]["other"] =
            json!({"type":"integer","readOnly":true,"forms":[{"href":other}]});
        if split {
            for (kind, free) in [(RouteReadinessTokensGlobalMax, 1), (CleanupItemsMax, 3)] {
                let hold = accounts
                    .reserve(kind, accounts.account(kind).limit() - free)
                    .unwrap();
                assert!(expose(&mut host, &td, &["relative", "other"], 27, &count, None).is_err());
                assert!(pressure.borrow().is_empty());
                drop(hold);
            }
        }
        let mut thing = expose(&mut host, &td, &["relative", "other"], 27, &count, None).unwrap();
        for kind in [RouteReadinessTokensGlobalMax, CleanupItemsMax] {
            let a = accounts.account(kind);
            assert_eq!(a.limit() - a.used(), allowance - if split { 2 } else { 1 });
        }
        drop(fixed_socket);
        publish(&mut host, &mut thing);
        // Each later Read requires its own independent exchange/cleanup capacity.
        pressure.borrow_mut().clear();
        let (body, result) = read_http(&mut host, &mut thing);
        assert_eq!(body, "27");
        assert_eq!(result.binding().0, BindingId::new(10));
        let td: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
        let assigned = addr(td["base"].as_str().unwrap());
        assert_ne!(assigned, fixed);
        assert_eq!(
            get(
                &mut host,
                &mut thing,
                if split { fixed } else { assigned },
                "constant"
            ),
            "27"
        );
        assert_eq!(
            outcome(&mut host, &mut thing).binding().0,
            BindingId::new(if split { 20 } else { 10 })
        );
        assert_eq!(count.get(), 2);
        retire(&mut host, &mut thing);
        drop((result, thing, host, pressure));
        empty(&accounts);
    }
}

struct UniqueReading {
    reading: Reading,
    drops: Rc<Cell<usize>>,
}
impl RawFiniteHandler for UniqueReading {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        self.reading.authorize(input)
    }
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.reading.handle(context, input)
    }
}
impl Drop for UniqueReading {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

#[test]
fn refusal_returns_the_original_handler_and_short_finalization_resumes_on_the_original_socket() {
    let accounts = bank();
    let acquired = Acquired::default();
    let mut host = deployment(
        udp::Datagram::new(),
        &[21],
        accounts.clone(),
        acquired.clone(),
    );
    let td: Arc<str> = source(true, None).to_string().into();
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let handler = UniqueReading {
        reading: Reading(41, calls.clone()),
        drops: drops.clone(),
    };
    let failure = host
        .expose(
            td.clone(),
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property("relative".into()),
                Operation::ReadProperty,
                handler,
            )],
            ExposurePolicy {
                replaces: None,
                handler_backing_bytes: 48,
            },
            &mut WorkBudget::new(),
        )
        .err()
        .unwrap();
    assert!(matches!(failure.error(), CoreError::Backpressure(_)));
    assert!(!failure.is_staged());
    assert!(acquired.borrow().is_empty());
    assert_eq!(drops.get(), 0);
    let original = failure.into_input();
    assert!(Arc::ptr_eq(&original.description, &td));
    assert_eq!(original.policy.handler_backing_bytes, 48);
    let short = || work().with_remaining(WorkClass::CodecOutputBytes, 24);
    let failure = host
        .expose(
            original.description,
            original.behaviors,
            original.policy,
            &mut short(),
        )
        .err()
        .unwrap();
    assert!(matches!(failure.error(), CoreError::Backpressure(_)));
    assert!(failure.is_staged());
    let address = acquired.borrow()[0];
    assert!(TcpListener::bind(address).is_err());
    let failure = failure.retry(&mut host, &mut short()).err().unwrap();
    assert!(matches!(failure.error(), CoreError::Backpressure(_)));
    // This turn reaches HTTP response preparation with an exhausted Binding lane.
    let mut response_short = work().with_remaining(
        WorkClass::BindingPolls,
        4096 + clinkz_wot_planning::security::PREPARATION_WORK,
    );
    let failure = failure.retry(&mut host, &mut response_short).err().unwrap();
    assert_eq!(response_short.remaining(WorkClass::BindingPolls), 0);
    assert!(failure.is_staged());
    assert!(matches!(failure.error(), CoreError::Backpressure(_)));
    // The unselected datagram service needs its own work during final qualification.
    let failure = failure
        .retry(
            &mut host,
            &mut work().with_remaining(WorkClass::ProviderProbes, 0),
        )
        .err()
        .unwrap();
    assert!(failure.is_staged());
    assert!(matches!(failure.error(), CoreError::Backpressure(_)));
    let mut foreign = deployment(udp::Datagram::new(), &[22], bank(), Acquired::default());
    let failure = failure.retry(&mut foreign, &mut work()).err().unwrap();
    assert!(matches!(failure.error(), CoreError::StaleHandle(_)));
    assert_eq!(acquired.borrow().as_slice(), &[address]);
    assert_eq!((calls.get(), drops.get()), (0, 0));
    let mut sibling = expose(
        &mut host,
        &source(true, None),
        &["relative"],
        3,
        &calls,
        None,
    )
    .unwrap();
    publish(&mut host, &mut sibling);
    let mut thing = failure.retry(&mut host, &mut work()).unwrap();
    publish(&mut host, &mut thing);
    let final_td: Value = serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
    assert_eq!(addr(final_td["base"].as_str().unwrap()), address);
    assert_eq!(acquired.borrow().len(), 2);
    assert_eq!(read_http(&mut host, &mut thing).0, "41");
    assert_eq!(read_http(&mut host, &mut sibling).0, "3");
    assert_eq!((calls.get(), drops.get()), (2, 0));
    retire(&mut host, &mut thing);
    retire(&mut host, &mut sibling);
    drop((thing, sibling, host));
    assert_eq!(drops.get(), 1);
    empty(&accounts);
}

#[test]
fn a_short_composed_handoff_preserves_the_acquisition_facts_and_can_be_abandoned() {
    let accounts = bank();
    let acquired = Acquired::default();
    let service = udp::Datagram::new();
    let enabled = service.enabled.clone();
    let mut host = deployment(service, &[21], accounts.clone(), acquired.clone());
    let baseline = ResourceKind::ALL.map(|kind| accounts.account(kind).used());
    let source: Arc<str> = source(true, None).to_string().into();
    let calls = Rc::new(Cell::new(0));
    for invalidate in [false, true] {
        let failure = host
            .expose(
                source.clone(),
                vec![RawFiniteBehavior::new(
                    AffordanceTarget::Property("relative".into()),
                    Operation::ReadProperty,
                    Reading(9, calls.clone()),
                )],
                ExposurePolicy::default(),
                &mut work().with_remaining(WorkClass::HandlerSteps, 2),
            )
            .err()
            .unwrap();
        assert!(matches!(failure.error(), CoreError::Backpressure(_)));
        let address = *acquired.borrow().last().unwrap();
        assert!(TcpListener::bind(address).is_err());
        let failure = if invalidate {
            enabled.borrow_mut().replace(false);
            enabled.borrow_mut().replace(true);
            let stale = failure.retry(&mut host, &mut work()).err().unwrap();
            assert!(matches!(stale.error(), CoreError::StaleHandle(_)));
            TcpListener::bind(address)
                .expect("stale candidate released before returning the refusal");
            stale
        } else {
            failure
        };
        let original = failure.into_input();
        assert!(Arc::ptr_eq(&source, &original.description));
        assert_eq!(original.behaviors.len(), 1);
        TcpListener::bind(address).expect("the caller released the staged endpoint");
        assert_eq!(
            ResourceKind::ALL.map(|kind| accounts.account(kind).used()),
            baseline
        );
    }
    assert_eq!(calls.get(), 0);
    assert_eq!(acquired.borrow().len(), 2);
    drop(host);
    empty(&accounts);
}

#[test]
fn fixed_registration_shares_one_listener_across_independent_publications() {
    let accounts = bank();
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
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
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
    retire(&mut host, &mut collision);
    assert_eq!(read_http(&mut host, &mut beta).0, "22");
    drop(unread);
    retire(&mut host, &mut alpha);
    assert!(TcpListener::bind(address).is_err());
    assert_eq!(read_http(&mut host, &mut beta).0, "22");
    assert_eq!(calls.get(), 3);
    retire(&mut host, &mut beta);
    TcpListener::bind(address).expect("final route released the shared socket");
    drop((alpha, beta, collision, host, listener));
    empty(&accounts);
}

#[test]
fn oversized_native_backing_releases_the_endpoint_and_returns_the_original_inputs() {
    const BUFFER: usize = 64 * 1024;
    let service = udp::Datagram::with_receive_bytes(BUFFER);
    let absolute = service.address();
    let acquired = Acquired::default();
    let make_host = |bytes, accounts| {
        let acquired = acquired.clone();
        HostProducer::with_assigned_base(
            vec![
                FiniteProducerRegistration::at_base(draft(), HttpFiniteProducer::at_endpoint),
                FiniteProducerRegistration::fixed(service.clone()),
            ],
            move || {
                let endpoint = HttpProducerEndpoint::acquire(zero())?;
                acquired.borrow_mut().push(addr(endpoint.as_ref()));
                Ok(endpoint)
            },
            4096,
            HostProducerLimits {
                candidate_bytes: bytes,
                ..limits()
            },
            Arc::new(GatewayDefaultV1::LIMITS.clone()),
            accounts,
        )
        .unwrap()
    };
    let accounts = bank();
    let mut small = make_host(BUFFER, accounts.clone());
    let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
    let calls = Rc::new(Cell::new(0));
    let td: Arc<str> = source(true, Some(absolute)).to_string().into();
    let failure = small
        .expose(
            td.clone(),
            read_behaviors(&["relative", "absolute"], 76, &calls),
            ExposurePolicy::default(),
            &mut work(),
        )
        .err()
        .unwrap();
    assert!(
        !failure.is_staged(),
        "fixed native backing cannot grow on retry"
    );
    let (kind, available, requested, _) = failure.error().limit_details().unwrap();
    assert_eq!(kind, ResourceKind::EngineLiveBytesGlobalMax);
    assert!(available < BUFFER as u64 && requested.unwrap() >= BUFFER as u64);
    assert_eq!(acquired.borrow().len(), 1);
    TcpListener::bind(acquired.borrow()[0]).expect("terminal refusal releases the socket");
    assert_eq!(
        ResourceKind::ALL.map(|k| accounts.account(k).used()),
        baseline
    );
    let original = failure.into_input();
    assert!(Arc::ptr_eq(&original.description, &td));
    assert_eq!(original.behaviors.len(), 2);
    drop(small);
    empty(&accounts);
    let mut large = make_host(2 * BUFFER, accounts.clone());
    let mut thing = large
        .expose(
            original.description,
            original.behaviors,
            original.policy,
            &mut work(),
        )
        .unwrap();
    publish(&mut large, &mut thing);
    assert_eq!(read_http(&mut large, &mut thing).0, "76");
    assert_eq!(read_udp(&mut large, &mut thing, absolute), 76);
    assert_eq!(calls.get(), 2);
    retire(&mut large, &mut thing);
    drop((large, thing));
    empty(&accounts);
}

#[test]
fn a_busy_specializer_resumes_on_its_original_endpoint() {
    struct Configuration {
        metadata: RefCell<()>,
        attempts: Cell<usize>,
        prefix: Cell<usize>,
        final_udp: udp::Datagram,
    }
    struct Endpoint {
        http: HttpProducerEndpoint,
        config: Rc<Configuration>,
    }
    impl AsRef<str> for Endpoint {
        fn as_ref(&self) -> &str {
            self.http.as_ref()
        }
    }

    let accounts = bank();
    let acquired = Acquired::default();
    let config = Rc::new(Configuration {
        metadata: RefCell::new(()),
        attempts: Cell::new(0),
        prefix: Cell::new(0),
        final_udp: udp::Datagram::new(),
    });
    let mut host = HostProducer::with_assigned_base(
        vec![
            FiniteProducerRegistration::at_base(udp::Datagram::new(), |_, lease: &Endpoint| {
                lease.config.prefix.set(lease.config.prefix.get() + 1);
                Ok(lease.config.final_udp.clone())
            }),
            FiniteProducerRegistration::at_base(draft(), |http, lease: &Endpoint| {
                lease.config.attempts.set(lease.config.attempts.get() + 1);
                let _metadata = lease.config.metadata.try_borrow().map_err(|_| {
                    CoreError::Backpressure(ErrorContext::new(
                        ErrorPhase::Prepare,
                        RetryClass::Safe,
                    ))
                })?;
                http.at_endpoint(&lease.http)
            }),
        ],
        {
            let acquired = acquired.clone();
            let config = config.clone();
            move || {
                let http = HttpProducerEndpoint::acquire(zero())?;
                acquired.borrow_mut().push(addr(http.as_ref()));
                Ok(Endpoint {
                    http,
                    config: config.clone(),
                })
            }
        },
        4096,
        limits(),
        Arc::new(GatewayDefaultV1::LIMITS.clone()),
        accounts.clone(),
    )
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let td: Arc<str> = source(true, None).to_string().into();
    let baseline = ResourceKind::ALL.map(|k| accounts.account(k).used());
    enum AfterWait {
        RecoverInputs,
        InvalidateView,
        Publish,
    }
    for (index, action) in [
        AfterWait::RecoverInputs,
        AfterWait::InvalidateView,
        AfterWait::Publish,
    ]
    .into_iter()
    .enumerate()
    {
        let exclusive = config.metadata.borrow_mut();
        let observing = matches!(action, AfterWait::RecoverInputs)
            .then(|| config.final_udp.enabled.borrow_mut());
        let before = config.attempts.get();
        let failure = host
            .expose(
                td.clone(),
                read_behaviors(&["relative"], 31, &calls),
                ExposurePolicy::default(),
                &mut work(),
            )
            .err()
            .unwrap();
        assert!(failure.is_staged());
        assert!(
            matches!(failure.error(), CoreError::Backpressure(context) if context.retry_class() == RetryClass::Safe)
        );
        let failure = if let Some(guard) = observing {
            assert_eq!(config.attempts.get(), before);
            drop(guard);
            // The first view already exists. Resume its observation, then reach
            // the second specializer while its metadata is still borrowed.
            failure.retry(&mut host, &mut work()).err().unwrap()
        } else {
            failure
        };
        assert_eq!(config.attempts.get(), before + 1);
        let address = acquired.borrow()[index];
        assert!(TcpListener::bind(address).is_err());
        let tried = config.attempts.get();
        let failure = failure
            .retry(&mut host, &mut WorkBudget::new())
            .err()
            .unwrap();
        assert!(failure.is_staged());
        assert_eq!(config.attempts.get(), tried);
        let mut one = WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1);
        let failure = failure.retry(&mut host, &mut one).err().unwrap();
        assert_eq!(one.remaining(WorkClass::BindingPolls), 0);
        assert_eq!(config.attempts.get(), tried + 1);
        assert_eq!(config.prefix.get(), index + 1);
        drop(exclusive);
        match action {
            AfterWait::Publish => {
                let mut thing = failure.retry(&mut host, &mut work()).unwrap();
                publish(&mut host, &mut thing);
                let final_td: Value =
                    serde_json::from_str(host.description(thing.key()).unwrap()).unwrap();
                assert_eq!(addr(final_td["base"].as_str().unwrap()), address);
                assert_eq!(read_http(&mut host, &mut thing).0, "31");
                retire(&mut host, &mut thing);
            }
            AfterWait::InvalidateView => {
                config.final_udp.enabled.borrow_mut().replace(false);
                config.final_udp.enabled.borrow_mut().replace(true);
                let stale = failure.retry(&mut host, &mut work()).err().unwrap();
                assert!(matches!(stale.error(), CoreError::StaleHandle(_)));
                assert!(!stale.is_staged());
                assert_eq!(config.attempts.get(), tried + 1);
            }
            AfterWait::RecoverInputs => {
                let input = failure.into_input();
                assert!(Arc::ptr_eq(&input.description, &td));
                assert_eq!(input.behaviors.len(), 1);
            }
        }
        assert_eq!(config.prefix.get(), index + 1);
        assert_eq!(acquired.borrow().len(), index + 1);
        TcpListener::bind(address).expect("released original native endpoint");
        assert_eq!(
            ResourceKind::ALL.map(|k| accounts.account(k).used()),
            baseline
        );
    }
    assert_eq!(calls.get(), 1);
    drop(host);
    empty(&accounts);
}

#[test]
fn a_one_token_bank_serves_two_forms_and_refuses_a_genuine_second_member() {
    use ResourceKind::RouteReadinessTokensGlobalMax as Slots;
    let accounts = Arc::new(HostResourceAccounts::new(
        &GatewayDefaultV1::LIMITS.clone().with_limit(Slots, Some(1)),
    ));
    let acquired = Acquired::default();
    let service = udp::Datagram::new();
    let absolute = service.address();
    let mut host = deployment(service, &[21], accounts.clone(), acquired.clone());
    let calls = Rc::new(Cell::new(0));
    let mut td = source(true, None);
    td["properties"]["other"] =
        json!({"type":"integer","readOnly":true,"forms":[{"href":"other"}]});
    let mut thing = expose(&mut host, &td, &["relative", "other"], 14, &calls, None).unwrap();
    assert_eq!(accounts.account(Slots).used(), 1);
    publish(&mut host, &mut thing);
    assert_eq!(read_http(&mut host, &mut thing).0, "14");
    assert_eq!(
        get(&mut host, &mut thing, acquired.borrow()[0], "other"),
        "14"
    );
    assert_eq!(
        outcome(&mut host, &mut thing).recipient(),
        Some(thing.key())
    );
    let occupied = expose(&mut host, &td, &["relative", "other"], 14, &calls, None)
        .err()
        .unwrap();
    assert_eq!(occupied.limit_details(), Some((Slots, 1, Some(1), None)));
    assert_eq!(acquired.borrow().len(), 1);
    retire(&mut host, &mut thing);
    drop(thing);
    let split = host
        .expose(
            source(true, Some(absolute)).to_string().into(),
            read_behaviors(&["relative", "absolute"], 15, &calls),
            ExposurePolicy::default(),
            &mut work(),
        )
        .err()
        .unwrap();
    assert_eq!(
        split.error().limit_details(),
        Some((Slots, 1, Some(2), None))
    );
    assert!(!split.is_staged());
    assert_eq!(acquired.borrow().len(), 2);
    TcpListener::bind(acquired.borrow()[1])
        .expect("unfundable final membership released the lease");
    assert_eq!(calls.get(), 2);
    drop(host);
    empty(&accounts);
}
