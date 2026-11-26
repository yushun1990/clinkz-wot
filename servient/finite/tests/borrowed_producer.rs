//! Downstream source-free datagram Read, driven only by the application.
use clinkz_wot_core::{
    data_schema::{PreparedSchema, ValueSchemas, ValueType},
    execution::{
        ValueExecution,
        producer::{FiniteDelivery, FiniteEffect, ReplyRetirementError},
        read::ReadMapping,
    },
    security::SecurityRequirement,
    *,
};
use clinkz_wot_foundation::{ClockId, MonotonicInstant};
use clinkz_wot_servient_finite::borrowed_producer::*;
use std::{
    cell::{Cell, RefCell},
    net::{SocketAddr, UdpSocket},
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

fn now(t: u64) -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(9), t)
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn error() -> CoreError {
    CoreError::Binding(ErrorContext::new(ErrorPhase::Cleanup, RetryClass::Safe))
}
fn socket() -> UdpSocket {
    UdpSocket::bind("127.0.0.1:0").unwrap()
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Failure {
    Isolated,
    Member,
    Ingress,
}
#[derive(Default)]
struct Stats {
    retire: Cell<usize>,
    disposed: Cell<usize>,
    sent: Cell<usize>,
    lost: Cell<bool>,
    fail: Cell<Option<Failure>>,
}
struct Reply(SocketAddr, Rc<Stats>);
impl Drop for Reply {
    fn drop(&mut self) {
        self.1.disposed.set(self.1.disposed.get() + 1);
    }
}
struct Datagram {
    socket: UdpSocket,
    stats: Rc<Stats>,
}
impl Datagram {
    fn new(stats: Rc<Stats>) -> Self {
        let socket = socket();
        socket.set_nonblocking(true).unwrap();
        Self { socket, stats }
    }
}
impl ReadMember for Datagram {
    type Program = SocketAddr;
    type Reply = Reply;
    type Scratch = [u8; 4];
    fn revision(&self) -> u64 {
        1
    }
    fn covers(&self, d: &PreparedRead<'_, SocketAddr>) -> bool {
        self.socket.local_addr().ok() == Some(*d.program)
            && d.mapping.target == format!("udp://{}/read", d.program)
            && d.meaning.site() == AffordanceSite::Property("level")
            && d.mapping.binding == BindingId::new(71)
            && d.mapping.generation == BindingGeneration::INITIAL
            && d.media == "application/json"
            && d.meaning.schemas().output == Some(PreparedSchema::Type(ValueType::Integer))
    }
    fn check_ingress(&self) -> CoreResult<()> {
        if self.stats.lost.get() {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<(Reply, Deadline)>> {
        let mut buf = [0; 2];
        match self.socket.recv_from(&mut buf) {
            Ok((1, addr)) => Poll::Ready(Ok((
                Reply(addr, self.stats.clone()),
                Deadline::at(now(buf[0] as u64)),
            ))),
            Ok(_) => Poll::Ready(Err(error())),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            Err(_) => Poll::Ready(Err(error())),
        }
    }
    fn validate(
        &self,
        _: &PreparedRead<'_, SocketAddr>,
        v: PayloadView<'_>,
        scratch: &mut [u8; 4],
    ) -> CoreResult<()> {
        let bytes = scratch.get_mut(..v.body.len()).ok_or_else(error)?;
        bytes.copy_from_slice(v.body);
        if bytes.len() == 1 && bytes[0].is_ascii_digit() {
            Ok(())
        } else {
            Err(error())
        }
    }
    fn poll_send(
        &mut self,
        r: &mut Reply,
        v: Result<PayloadView<'_>, &CoreError>,
        _: &mut Context<'_>,
    ) -> Poll<CoreResult<()>> {
        self.socket
            .send_to(v.map_err(Clone::clone)?.body, r.0)
            .map_err(|_| error())?;
        self.stats.sent.set(self.stats.sent.get() + 1);
        Poll::Ready(Ok(()))
    }
    fn poll_retire(
        &mut self,
        _: &mut Reply,
        _: &CoreError,
        withdrawn: bool,
        _: &mut Context<'_>,
    ) -> Poll<Result<FiniteDelivery, ReplyRetirementError>> {
        self.stats.retire.set(self.stats.retire.get() + 1);
        match self.stats.fail.take() {
            Some(Failure::Member) => {
                assert!(!withdrawn);
                Poll::Ready(Err(ReplyRetirementError::Member(error())))
            }
            Some(Failure::Isolated) => Poll::Ready(Err(ReplyRetirementError::Isolated(error()))),
            _ => Poll::Ready(Ok(FiniteDelivery::NotDelivered)),
        }
    }
}
struct Application(Cell<u8>);
impl ReadBehavior for Application {
    fn authorize(&self, site: AffordanceSite<'_>) -> CoreResult<()> {
        assert_eq!(site, AffordanceSite::Property("level"));
        Ok(())
    }
    fn read(&self, _: AffordanceSite<'_>, out: &mut [u8]) -> CoreResult<usize> {
        self.0.set(self.0.get() + 1);
        out[0] = b'0' + self.0.get();
        Ok(1)
    }
}
fn definition<'a>(
    address: &'a SocketAddr,
    target: &'a str,
    td: &'a str,
    schemas: &'a ValueSchemas,
) -> PreparedRead<'a, SocketAddr> {
    PreparedRead {
        description: td,
        meaning: ValueExecution::try_new(
            AffordanceSite::Property("level"),
            Operation::ReadProperty,
            schemas,
            &SecurityRequirement::NOSEC,
        )
        .unwrap(),
        program: address,
        mapping: ReadMapping {
            target,
            binding: BindingId::new(71),
            generation: BindingGeneration::INITIAL,
            plan: PlanId::new(
                clinkz_wot_foundation::SlotIndex::new(0),
                clinkz_wot_foundation::Generation::INITIAL,
            ),
        },
        media: "application/json",
        output_bytes: 4,
    }
}
fn source(target: &str) -> String {
    format!(
        r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"Meter","security":"n","securityDefinitions":{{"n":{{"scheme":"nosec"}}}},"properties":{{"level":{{"type":"integer","readOnly":true,"forms":[{{"href":"{target}"}}]}}}}}}"#
    )
}
fn offer<'a>(
    p: &mut ReadPublication<'a, '_, Datagram>,
    addr: SocketAddr,
    peer: &UdpSocket,
    expires: u8,
    t: u64,
) -> ReadOffer<'a, Reply> {
    peer.send_to(&[expires], addr).unwrap();
    match p.poll_offer(&mut cx(), now(t)) {
        Poll::Ready(Ok(o)) => o,
        _ => panic!("missing original datagram"),
    }
}
fn wire(peer: &UdpSocket) -> u8 {
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut buf = [0; 4];
    assert_eq!(peer.recv(&mut buf).unwrap(), 1);
    buf[0]
}
fn finish<'a>(
    p: &mut ReadPublication<'a, '_, Datagram>,
    key: ExchangeKey<'_>,
    t: u64,
) -> ReadSettlement<'a> {
    match p.poll_exchange(key, &mut cx(), now(t)) {
        Poll::Ready(Ok(s)) => s,
        _ => panic!("expected original retirement"),
    }
}
#[test]
fn original_reply_survives_timeout_while_a_sibling_finishes_and_result_outlives_replacement() {
    trial(Failure::Isolated);
}
#[test]
fn loss_of_isolation_fences_only_the_dependent_publication() {
    trial(Failure::Member);
}
#[test]
fn ingress_loss_during_isolated_retirement_drains_original_replies() {
    trial(Failure::Ingress);
}

fn trial(mode: Failure) {
    let stats = Rc::new(Stats::default());
    let first = Datagram::new(stats.clone());
    let other = Datagram::new(Rc::default());
    let successor = Datagram::new(Rc::default());
    let [a, b, c] = [&first, &other, &successor].map(|m| m.socket.local_addr().unwrap());
    let urls = [a, b, c].map(|addr| format!("udp://{addr}/read"));
    let descriptions = urls.each_ref().map(|u| source(u));
    let schemas = ValueSchemas {
        input: None,
        output: Some(PreparedSchema::Type(ValueType::Integer)),
    };
    let defs = [(&a, 0), (&b, 1), (&c, 2)]
        .map(|(addr, i)| definition(addr, &urls[i], &descriptions[i], &schemas));
    let turns = ProducerTurns::default();
    let slot = PublicationSlot::new(&turns);
    let independent = PublicationSlot::new(&turns);
    let app = Application(Cell::new(0));
    let buffers: [RefCell<[u8; 4]>; 3] = std::array::from_fn(|_| RefCell::new([0; 4]));
    let mut slots = [ReadSlot::new([0; 4]), ReadSlot::new([0; 4])];
    let mut other_slots = [ReadSlot::new([0; 4])];
    let mut next_slots = [ReadSlot::new([0; 4])];
    let mut p = ReadPublication::stage(
        &slot,
        None,
        &defs[0],
        first,
        &app,
        &mut slots,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    let mut sibling = ReadPublication::stage(
        &independent,
        None,
        &defs[1],
        other,
        &app,
        &mut other_slots,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    for service in [&mut p, &mut sibling] {
        assert!(matches!(
            service.poll_publish(&mut cx(), now(0)),
            Poll::Ready(Ok(()))
        ));
    }
    let peers = [socket(), socket(), socket()];
    let one = offer(&mut p, a, &peers[0], 4, 1);
    let two = offer(&mut p, a, &peers[1], 15, 1);
    let (one, err) = sibling.accept(one, &buffers[0], now(2)).err().unwrap();
    assert!(matches!(err, CoreError::StaleHandle(_)));
    assert_eq!(one.deadline(), Deadline::at(now(4)));
    assert_eq!(app.0.get(), 0);
    let i = p.accept(one, &buffers[0], now(2)).ok().unwrap();
    let j = p.accept(two, &buffers[1], now(2)).ok().unwrap();
    let extra = offer(&mut p, a, &peers[2], 17, 2);
    let (extra, err) = p.accept(extra, &buffers[2], now(2)).err().unwrap();
    assert!(matches!(err, CoreError::Backpressure(_)));
    assert_eq!(extra.deadline(), Deadline::at(now(17)));
    assert_eq!(app.0.get(), 2);
    stats.fail.set(Some(if mode == Failure::Ingress {
        Failure::Isolated
    } else {
        mode
    }));
    assert!(p.poll_exchange(i, &mut cx(), now(4)).is_pending());
    let evidence = p.retirement(i).unwrap();
    assert!(matches!(evidence.first_cause, Some(CoreError::TimedOut(_))));
    assert!(matches!(
        evidence.cleanup_error,
        Some(CoreError::Binding(_))
    ));
    assert_eq!(stats.disposed.get(), 0);
    assert!(buffers[0].try_borrow_mut().is_err());
    if mode == Failure::Ingress {
        stats.lost.set(true);
    }
    let retained = finish(&mut p, j, 5);
    if mode == Failure::Isolated {
        assert!(p.description().is_some());
        assert_eq!(retained.delivery(), FiniteDelivery::Delivered);
        assert_eq!(wire(&peers[1]), b'2');
        let (extra, err) = p.accept(extra, &buffers[1], now(5)).err().unwrap();
        assert!(matches!(err, CoreError::Backpressure(_)));
        let k = p.accept(extra, &buffers[2], now(5)).ok().unwrap();
        drop(finish(&mut p, k, 5));
        assert_eq!(wire(&peers[2]), b'3');
        assert!(matches!(
            p.poll_exchange(j, &mut cx(), now(5)),
            Poll::Ready(Err(CoreError::StaleHandle(_)))
        ));
    } else {
        assert!(p.description().is_none());
        assert_eq!(p.failure(), Some(&error()));
        assert_eq!(retained.first_cause(), Some(&error()));
        assert_eq!(retained.delivery(), FiniteDelivery::NotDelivered);
        let (extra, e) = p.accept(extra, &buffers[2], now(5)).err().unwrap();
        assert!(matches!(e, CoreError::StaleHandle(_)));
        drop(extra);
    }
    let timeout = finish(&mut p, i, 6);
    assert_eq!(timeout.delivery(), FiniteDelivery::NotDelivered);
    assert!(matches!(
        timeout.first_cause(),
        Some(CoreError::TimedOut(_))
    ));
    assert!(matches!(
        timeout.cleanup_error(),
        Some(CoreError::Binding(_))
    ));
    assert_eq!(timeout.result().unwrap(), b"1");
    assert_eq!(timeout.effect(), FiniteEffect::Unknown);
    assert_eq!(
        stats.sent.get(),
        if mode == Failure::Isolated { 2 } else { 0 }
    );
    assert_eq!(stats.disposed.get(), 3);
    assert_eq!(
        stats.retire.get(),
        if mode == Failure::Isolated { 2 } else { 3 }
    );
    drop(timeout);
    let request = offer(&mut sibling, b, &peers[0], 20, 7);
    let k = sibling.accept(request, &buffers[0], now(7)).ok().unwrap();
    drop(finish(&mut sibling, k, 7));
    assert_eq!(wire(&peers[0]), b'0' + app.0.get());
    let old = p.key();
    let replaces = old.is_current().then_some(old);
    let mut newer = ReadPublication::stage(
        &slot,
        replaces,
        &defs[2],
        successor,
        &app,
        &mut next_slots,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    assert!(matches!(
        newer.poll_publish(&mut cx(), now(8)),
        Poll::Ready(Ok(()))
    ));
    p.retire().unwrap();
    drop(p);
    UdpSocket::bind(a).expect("original member released");
    assert!(retained.recipient() == old && !old.is_current());
    assert_eq!(retained.result().unwrap(), b"2");
    let next = offer(&mut newer, c, &peers[1], 21, 8);
    let (next, e) = newer.accept(next, &buffers[1], now(8)).err().unwrap();
    assert!(matches!(e, CoreError::Backpressure(_)));
    drop(retained);
    let k = newer.accept(next, &buffers[1], now(9)).ok().unwrap();
    drop(finish(&mut newer, k, 9));
    assert_eq!(wire(&peers[1]), b'0' + app.0.get());
    let expired = offer(&mut newer, c, &peers[1], 10, 9);
    let effects = app.0.get();
    let (expired, error) = newer.accept(expired, &buffers[1], now(10)).err().unwrap();
    assert!(matches!(error, CoreError::TimedOut(_)));
    assert_eq!(expired.deadline(), Deadline::at(now(10)));
    assert_eq!(expired.into_reply().0, peers[1].local_addr().unwrap());
    assert_eq!(app.0.get(), effects);
}

#[test]
fn revocation_in_delivery_authorization_keeps_the_original_value_off_the_wire() {
    struct Guard<'a> {
        revoke: Cell<Option<ReadRevocation<'a>>>,
        reads: Cell<u32>,
    }
    impl ReadBehavior for Guard<'_> {
        fn authorize(&self, _: AffordanceSite<'_>) -> CoreResult<()> {
            if let Some(revocation) = self.revoke.take() {
                revocation.revoke();
            }
            Ok(())
        }
        fn read(&self, _: AffordanceSite<'_>, bytes: &mut [u8]) -> CoreResult<usize> {
            self.reads.set(self.reads.get() + 1);
            bytes[0] = b'7';
            Ok(1)
        }
    }
    let stats = Rc::new(Stats::default());
    let member = Datagram::new(stats.clone());
    let address = member.socket.local_addr().unwrap();
    let target = format!("udp://{address}/read");
    let td = source(&target);
    let schemas = ValueSchemas {
        input: None,
        output: Some(PreparedSchema::Type(ValueType::Integer)),
    };
    let definition = definition(&address, &target, &td, &schemas);
    let turns = ProducerTurns::default();
    let slot = PublicationSlot::new(&turns);
    let guard = Guard {
        revoke: Cell::new(None),
        reads: Cell::new(0),
    };
    let output = RefCell::new([0; 4]);
    let mut slots = [ReadSlot::new([0; 4])];
    let mut published = ReadPublication::stage(
        &slot,
        None,
        &definition,
        member,
        &guard,
        &mut slots,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    assert!(matches!(
        published.poll_publish(&mut cx(), now(0)),
        Poll::Ready(Ok(()))
    ));
    let peer = socket();
    let request = offer(&mut published, address, &peer, 8, 1);
    let key = published.accept(request, &output, now(1)).ok().unwrap();
    guard.revoke.set(published.revocation());
    let retained = finish(&mut published, key, 2);
    assert_eq!(guard.reads.get(), 1);
    assert!(published.description().is_none());
    assert!(matches!(
        retained.first_cause(),
        Some(CoreError::StaleHandle(_))
    ));
    assert_eq!(retained.result().unwrap(), b"7");
    assert_eq!(retained.delivery(), FiniteDelivery::NotDelivered);
    assert_eq!(
        (stats.sent.get(), stats.retire.get(), stats.disposed.get()),
        (0, 1, 1)
    );
    peer.set_nonblocking(true).unwrap();
    assert_eq!(
        peer.recv(&mut [0]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn close_interrupts_original_exchanges_before_their_next_turn() {
    let stats = Rc::new(Stats::default());
    let member = Datagram::new(stats.clone());
    let address = member.socket.local_addr().unwrap();
    let target = format!("udp://{address}/read");
    let td = source(&target);
    let schemas = ValueSchemas {
        input: None,
        output: Some(PreparedSchema::Type(ValueType::Integer)),
    };
    let definition = definition(&address, &target, &td, &schemas);
    let turns = ProducerTurns::default();
    let slot = PublicationSlot::new(&turns);
    let app = Application(Cell::new(0));
    let buffers = [const { RefCell::new([0; 4]) }; 3];
    let mut slots = std::array::from_fn::<_, 3, _>(|_| ReadSlot::new([0; 4]));
    let mut p = ReadPublication::stage(
        &slot,
        None,
        &definition,
        member,
        &app,
        &mut slots,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    assert!(matches!(
        p.poll_publish(&mut cx(), now(0)),
        Poll::Ready(Ok(()))
    ));
    let peer = socket();
    let [expired, early, late] = [0, 1, 2].map(|i| {
        let request = offer(&mut p, address, &peer, if i == 0 { 3 } else { 6 }, 2);
        p.accept(request, &buffers[i], now(2)).ok().unwrap()
    });
    stats.fail.set(Some(Failure::Isolated));
    assert!(p.poll_exchange(expired, &mut cx(), now(3)).is_pending());

    p.close();
    p.close();
    assert!(p.description().is_none());
    assert!(matches!(p.failure(), Some(CoreError::Cancelled(_))));
    for key in [early, late] {
        assert!(matches!(
            p.retirement(key).unwrap().first_cause,
            Some(CoreError::Cancelled(_))
        ));
    }
    for (key, t, value) in [(early, 3, b'2'), (late, 9, b'3')] {
        let result = finish(&mut p, key, t);
        assert!(matches!(
            result.first_cause(),
            Some(CoreError::Cancelled(_))
        ));
        assert_eq!(result.delivery(), FiniteDelivery::NotDelivered);
        assert_eq!(result.result().unwrap(), &[value]);
    }
    let result = finish(&mut p, expired, 9);
    assert!(matches!(result.first_cause(), Some(CoreError::TimedOut(_))));
    assert_eq!(result.cleanup_error(), Some(&error()));
    assert_eq!(result.result().unwrap(), b"1");
    assert_eq!(app.0.get(), 3);
    assert_eq!(
        (stats.sent.get(), stats.retire.get(), stats.disposed.get()),
        (0, 4, 3)
    );
    p.retire().unwrap();
}
