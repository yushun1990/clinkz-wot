//! These are downstream test Bindings, not qualified product protocol mappings.
use super::*;
use std::{
    cell::{Cell, RefCell},
    net::UdpSocket,
    rc::Rc,
};

type Dispatches = Rc<RefCell<Vec<(ProducerInstanceKey, BindingId)>>>;

// Non-Clone, non-Send behavior with its actual mutable value inside the one
// registered object. Reconstructing a behavior per Binding would reset it.
struct CountingHandler {
    value: Cell<u8>,
    dispatches: Dispatches,
    dropped: Rc<Cell<usize>>,
}
impl Drop for CountingHandler {
    fn drop(&mut self) {
        self.dropped.set(self.dropped.get() + 1);
    }
}
impl RawFiniteHandler for CountingHandler {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.dispatches
            .borrow_mut()
            .push((ctx.producer_instance().unwrap(), ctx.binding().unwrap().0));
        self.value.set(self.value.get() + 1);
        Ok(InteractionOutput::with_data(Payload::new(
            vec![self.value.get()],
            "application/json",
        )))
    }
}
fn counted(
    authority: ProducerInstanceAuthority,
    initial: u8,
    dispatches: &Dispatches,
    dropped: &Rc<Cell<usize>>,
) -> RawProducerInstance {
    RawProducerInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property("value".into()),
            Operation::ReadProperty,
            CountingHandler {
                value: Cell::new(initial),
                dispatches: dispatches.clone(),
                dropped: dropped.clone(),
            },
        )],
    )
}
fn tcp(
    a: ProducerInstanceKey,
    b: ProducerInstanceKey,
) -> (TcpBinding, Arc<Mutex<Option<SocketAddr>>>) {
    let addr = Arc::new(Mutex::new(None));
    (
        TcpBinding {
            config: 7,
            listener: None,
            pending: None,
            addr: addr.clone(),
            a,
            b,
            polls: Arc::new(AtomicUsize::new(0)),
        },
        addr,
    )
}
fn start<B: FiniteIngressBinding>(
    servient: &mut RawProducerServient,
    runtime: &mut RawFiniteRuntime<B>,
) {
    assert_eq!(
        step(servient, runtime),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    runtime.activate_raw().unwrap();
}

// A differently typed native runtime: datagram correlation and endpoint lifetime
// stay private here. The Engine has no TCP/UDP branch or session representation.
struct DatagramReply(SocketAddr);
struct DatagramBinding {
    socket: Option<UdpSocket>,
    addr: Rc<Cell<Option<SocketAddr>>>,
    live: Rc<Cell<bool>>,
    old: ProducerInstanceKey,
    next: ProducerInstanceKey,
    send_pending: Rc<Cell<bool>>,
}
impl Drop for DatagramBinding {
    fn drop(&mut self) {
        self.live.set(false);
    }
}
fn datagram(old: ProducerInstanceKey, next: ProducerInstanceKey) -> DatagramBinding {
    DatagramBinding {
        socket: None,
        addr: Rc::new(Cell::new(None)),
        live: Rc::new(Cell::new(false)),
        old,
        next,
        send_pending: Rc::new(Cell::new(false)),
    }
}
impl FiniteIngressBinding for DatagramBinding {
    type Reply = DatagramReply;
    type Configuration = ();
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(5), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &() {
        &()
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        let socket = UdpSocket::bind("127.0.0.1:0").map_err(|_| error())?;
        socket.set_nonblocking(true).map_err(|_| error())?;
        self.addr.set(Some(socket.local_addr().unwrap()));
        self.socket = Some(socket);
        self.live.set(true);
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        let mut bytes = [0u8; 1];
        match self.socket.as_ref().unwrap().recv_from(&mut bytes) {
            Ok((1, peer)) => Poll::Ready(Ok(FiniteIngress {
                candidates: vec![FiniteCandidate {
                    instance: if bytes[0] == b'n' {
                        self.next
                    } else {
                        self.old
                    },
                    target: AffordanceTarget::Property("value".into()),
                    operation: Operation::ReadProperty,
                }],
                input: InteractionInput::empty(),
                reply: DatagramReply(peer),
            })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            _ => Poll::Ready(Err(error())),
        }
    }
    fn poll_send(
        &mut self,
        _: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        if self.send_pending.get() {
            return Poll::Pending;
        }
        let byte = reply
            .result
            .as_ref()
            .ok()
            .and_then(|output| output.data())
            .map_or(b'E', |payload| payload.body[0]);
        match self
            .socket
            .as_ref()
            .unwrap()
            .send_to(&[byte], reply.ingress.reply.0)
        {
            Ok(1) => Poll::Ready(Ok(())),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            _ => Poll::Ready(Err(error())),
        }
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        ingress: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        _: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        // This native mapping has no remote cancellation handshake. Close
        // releases its local endpoint; no original reply was sent by close.
        assert!(ingress.is_none() || reply.is_none());
        self.socket = None;
        self.live.set(false);
        Poll::Ready(Ok(FiniteDelivery::NotDelivered))
    }
}
fn client(addr: SocketAddr, byte: u8) -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    socket.send_to(&[byte], addr).unwrap();
    socket
}
fn udp_exchange(
    servient: &mut RawProducerServient,
    runtime: &mut RawFiniteRuntime<DatagramBinding>,
    addr: SocketAddr,
    request: u8,
    expected: u8,
) -> clinkz_wot_core::finite::RawFiniteSettlement {
    let client = client(addr, request);
    for state in [
        RawFiniteState::Dispatching,
        RawFiniteState::Delivering,
        RawFiniteState::Serving,
    ] {
        assert_eq!(step(servient, runtime), Poll::Ready(state));
    }
    let mut reply = [0u8; 1];
    assert_eq!(client.recv(&mut reply).unwrap(), 1);
    assert_eq!(reply, [expected]);
    runtime.take_settlement().unwrap()
}

#[test]
fn one_mutable_behavior_serves_two_native_types_and_survives_one_retirement() {
    let authority = authority(200);
    let key = authority.key();
    let dispatches = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(Cell::new(0));
    let mut servient = servient(
        vec![counted(authority, b'0', &dispatches, &dropped)],
        registry_limits(),
    );
    let (binding, tcp_addr) = tcp(key, key);
    let mut tcp_rt = RawFiniteRuntime::try_new(
        binding,
        7,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let binding = datagram(key, key);
    let (udp_addr, live) = (binding.addr.clone(), binding.live.clone());
    let mut udp_rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    start(&mut servient, &mut tcp_rt);
    start(&mut servient, &mut udp_rt);
    let (tcp_addr, udp_addr) = (tcp_addr.lock().unwrap().unwrap(), udp_addr.get().unwrap());
    exchange(&mut servient, &mut tcp_rt, tcp_addr, b'a', b'1');
    let outcome = udp_exchange(&mut servient, &mut udp_rt, udp_addr, b'o', b'2');
    assert_eq!(
        outcome.binding,
        (BindingId::new(5), BindingGeneration::INITIAL)
    );
    assert_eq!(outcome.recipient, Some(key));
    assert_eq!(
        outcome.target,
        Some(AffordanceTarget::Property("value".into()))
    );
    exchange(&mut servient, &mut tcp_rt, tcp_addr, b'a', b'3');
    tcp_rt.begin_close();
    assert_eq!(
        tcp_rt.step(&mut context(), &mut budget(0), now()),
        Poll::Pending
    );
    assert_eq!(
        tcp_rt.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert!(TcpStream::connect(tcp_addr).is_err());
    drop(tcp_rt);
    assert!(live.get());
    assert_eq!(dropped.get(), 0);
    udp_exchange(&mut servient, &mut udp_rt, udp_addr, b'o', b'4');
    assert_eq!(
        &*dispatches.borrow(),
        &[
            (key, BindingId::new(3)),
            (key, BindingId::new(5)),
            (key, BindingId::new(3)),
            (key, BindingId::new(5))
        ]
    );
    assert!(servient.revoke(key));
    let refused = udp_exchange(&mut servient, &mut udp_rt, udp_addr, b'o', b'E');
    assert_eq!(refused.effect, FiniteEffect::NotInvoked);
    assert!(matches!(refused.result, Err(CoreError::StaleHandle(_))));
    assert!(live.get()); // logical revocation does not retire native resources
    udp_rt.begin_close();
    assert_eq!(
        udp_rt.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert!(!live.get());
    drop(servient);
    assert_eq!(dropped.get(), 1);
}

#[test]
fn revocation_and_same_generation_replacement_fence_both_deferred_native_candidates() {
    let old = authority(201);
    let next = authority(201); // even identical slot/generation is a new authority
    let (old_key, next_key) = (old.key(), next.key());
    assert_eq!(old_key.slot(), next_key.slot());
    assert_ne!(old_key, next_key);
    let dispatches = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(Cell::new(0));
    let mut servient = servient(
        vec![counted(old, b'0', &dispatches, &dropped)],
        registry_limits(),
    );
    let (binding, address) = tcp(old_key, next_key);
    let mut tcp_rt = RawFiniteRuntime::try_new(
        binding,
        7,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let binding = datagram(old_key, next_key);
    let udp_addr = binding.addr.clone();
    let mut udp_rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    start(&mut servient, &mut tcp_rt);
    start(&mut servient, &mut udp_rt);
    let tcp_addr = address.lock().unwrap().unwrap();
    let udp_addr = udp_addr.get().unwrap();
    let mut tcp_client = TcpStream::connect(tcp_addr).unwrap();
    tcp_client
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    tcp_client.write_all(b"a").unwrap();
    let udp_client = client(udp_addr, b'o');
    // Native progress alone can match, but cannot invoke the single behavior.
    assert_eq!(
        tcp_rt.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        udp_rt.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        tcp_rt.step(&mut context(), &mut budget(5000), now()),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert!(dispatches.borrow().is_empty());
    assert!(servient.revoke(old_key));
    // Revocation wins for TCP before replacement; replacement wins for UDP.
    assert_eq!(
        step(&mut servient, &mut tcp_rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    servient
        .replace(old_key, counted(next, b'5', &dispatches, &dropped))
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(dropped.get(), 1);
    assert_eq!(
        step(&mut servient, &mut udp_rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(
        step(&mut servient, &mut tcp_rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    assert_eq!(
        step(&mut servient, &mut udp_rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    let mut response = [0];
    tcp_client.read_exact(&mut response).unwrap();
    assert_eq!(response, *b"E");
    udp_client.recv(&mut response).unwrap();
    assert_eq!(response, *b"E");
    for outcome in [
        tcp_rt.take_settlement().unwrap(),
        udp_rt.take_settlement().unwrap(),
    ] {
        assert_eq!(outcome.recipient, Some(old_key));
        assert_eq!(outcome.effect, FiniteEffect::NotInvoked);
        assert!(matches!(outcome.result, Err(CoreError::StaleHandle(_))));
    }
    assert!(dispatches.borrow().is_empty());
    exchange(&mut servient, &mut tcp_rt, tcp_addr, b'v', b'6');
    udp_exchange(&mut servient, &mut udp_rt, udp_addr, b'n', b'7');
    exchange(&mut servient, &mut tcp_rt, tcp_addr, b'a', b'E');
    udp_exchange(&mut servient, &mut udp_rt, udp_addr, b'o', b'E');
    // Local disposal of a still-serving Binding also cannot revoke the instance.
    drop(tcp_rt);
    assert!(TcpStream::connect(tcp_addr).is_err());
    udp_exchange(&mut servient, &mut udp_rt, udp_addr, b'n', b'8');
    assert_eq!(
        dispatches.borrow().iter().map(|d| d.0).collect::<Vec<_>>(),
        vec![next_key; 3]
    );
}

fn pending(key: ProducerInstanceKey) -> PendingBinding {
    PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: key,
                target: AffordanceTarget::Property("value".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(Arc::new(AtomicUsize::new(0))),
        }),
        pending_send: Arc::new(AtomicBool::new(true)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(true)),
    }
}

#[test]
fn two_accepted_results_pin_original_behavior_until_both_acknowledge_while_sibling_progresses() {
    let old = authority(202);
    let next = authority(202);
    let (old_key, next_key) = (old.key(), next.key());
    let dispatches = Rc::new(RefCell::new(Vec::new()));
    let old_dropped = Rc::new(Cell::new(0));
    let new_dropped = Rc::new(Cell::new(0));
    let mut servient = servient(
        vec![counted(old, b'0', &dispatches, &old_dropped)],
        registry_limits(),
    );
    let binding = pending(old_key);
    let fail_close = binding.fail_close.clone();
    let reply_dropped = binding.ingress.as_ref().unwrap().reply.0.clone();
    let mut stuck = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let binding = datagram(old_key, next_key);
    let (addr, send_pending) = (binding.addr.clone(), binding.send_pending.clone());
    send_pending.set(true);
    let mut sibling = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    start(&mut servient, &mut stuck);
    start(&mut servient, &mut sibling);
    assert_eq!(
        step(&mut servient, &mut stuck),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut stuck),
        Poll::Ready(RawFiniteState::Delivering)
    );
    let addr = addr.get().unwrap();
    let udp_client = client(addr, b'o');
    assert_eq!(
        step(&mut servient, &mut sibling),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut sibling),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(step(&mut servient, &mut stuck), Poll::Pending);
    assert_eq!(step(&mut servient, &mut sibling), Poll::Pending);
    servient
        .replace(old_key, counted(next, b'5', &dispatches, &new_dropped))
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert!(!servient.revoke(old_key));
    assert_eq!(old_dropped.get(), 0);
    stuck.begin_close();
    for _ in 0..3 {
        assert_eq!(
            stuck.step(&mut context(), &mut budget(0), now()),
            Poll::Pending
        );
        assert_eq!(
            stuck.step(&mut context(), &mut budget(1), now()),
            Poll::Ready(RawFiniteState::Closing)
        );
        assert_eq!(reply_dropped.load(Ordering::SeqCst), 0);
    }
    assert!(matches!(stuck.first_cause(), Some(CoreError::Cancelled(_))));
    send_pending.set(false);
    assert_eq!(
        step(&mut servient, &mut sibling),
        Poll::Ready(RawFiniteState::Serving)
    );
    let mut byte = [0];
    udp_client.recv(&mut byte).unwrap();
    assert_eq!(byte, *b"2");
    let second = sibling.take_settlement().unwrap();
    assert_eq!(second.recipient, Some(old_key));
    assert_eq!(second.binding.0, BindingId::new(5));
    assert_eq!(second.delivery, FiniteDelivery::Delivered);
    assert_eq!(&*second.result.unwrap().data().unwrap().body, b"2");
    assert_eq!(old_dropped.get(), 0); // the other accepted result still pins it
    udp_exchange(&mut servient, &mut sibling, addr, b'n', b'6');
    assert_eq!(new_dropped.get(), 0);
    fail_close.store(false, Ordering::SeqCst);
    assert_eq!(
        stuck.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert_eq!(reply_dropped.load(Ordering::SeqCst), 1);
    assert_eq!(old_dropped.get(), 0); // native close is not acknowledgement
    let first = stuck.take_settlement().unwrap();
    assert_eq!(old_dropped.get(), 1);
    assert_eq!(first.recipient, Some(old_key));
    assert_eq!(first.binding.0, BindingId::new(4));
    assert_eq!(
        first.target,
        Some(AffordanceTarget::Property("value".into()))
    );
    assert_eq!(first.effect, FiniteEffect::Unknown);
    assert_eq!(first.delivery, FiniteDelivery::Unknown);
    assert_eq!(&*first.result.unwrap().data().unwrap().body, b"1");
    assert!(stuck.take_settlement().is_none());
    assert_eq!(
        &*dispatches.borrow(),
        &[
            (old_key, BindingId::new(4)),
            (old_key, BindingId::new(5)),
            (next_key, BindingId::new(5))
        ]
    );
}

#[test]
fn wrong_issuer_cannot_dispatch_and_owner_drop_fences_unaccepted_but_retains_accepted_work() {
    let old = authority(203);
    let key = old.key();
    let dispatches = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(Cell::new(0));
    let mut original = servient(
        vec![counted(old, b'0', &dispatches, &dropped)],
        registry_limits(),
    );
    let mut other = servient(
        vec![counted(
            authority(203),
            b'5',
            &dispatches,
            &Rc::new(Cell::new(0)),
        )],
        registry_limits(),
    );
    let binding = pending(key);
    binding.fail_close.store(false, Ordering::SeqCst);
    let send_pending = binding.pending_send.clone();
    let mut accepted = RawFiniteRuntime::try_new(
        binding,
        (),
        &original,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    let binding = pending(key);
    let fail_close = binding.fail_close.clone();
    let mut deferred = RawFiniteRuntime::try_new(
        binding,
        (),
        &original,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    start(&mut original, &mut accepted);
    start(&mut original, &mut deferred);
    assert_eq!(
        step(&mut original, &mut accepted),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut original, &mut deferred),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    let mut credits = budget(5000);
    assert!(matches!(
        other.step(&mut deferred, &mut context(), &mut credits, now()),
        Err(CoreError::StaleHandle(_))
    ));
    assert_eq!(credits.remaining(WorkClass::HandlerSteps), 5000);
    assert_eq!(deferred.state(), RawFiniteState::Dispatching);
    assert!(dispatches.borrow().is_empty());
    assert_eq!(
        step(&mut original, &mut accepted),
        Poll::Ready(RawFiniteState::Delivering)
    );
    drop(original);
    assert_eq!(dropped.get(), 0);
    assert_eq!(
        deferred.step(&mut context(), &mut budget(0), now()),
        Poll::Pending
    );
    assert_eq!(deferred.state(), RawFiniteState::Dispatching);
    assert_eq!(
        deferred.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closing)
    );
    assert!(matches!(
        deferred.first_cause(),
        Some(CoreError::StaleHandle(_))
    ));
    assert_eq!(
        deferred.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closing)
    );
    fail_close.store(false, Ordering::SeqCst);
    assert_eq!(
        deferred.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closed)
    );
    let refused = deferred.take_settlement().unwrap();
    assert_eq!(refused.effect, FiniteEffect::NotInvoked);
    assert!(matches!(refused.result, Err(CoreError::StaleHandle(_))));
    // Accepted success remains deliverable and owned after the registry dies.
    assert_eq!(
        accepted.step(&mut context(), &mut budget(1), now()),
        Poll::Pending
    );
    send_pending.store(false, Ordering::SeqCst);
    assert_eq!(
        accepted.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Serving)
    );
    accepted.begin_close();
    assert_eq!(
        accepted.step(&mut context(), &mut budget(1), now()),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert_eq!(dropped.get(), 0);
    let outcome = accepted.take_settlement().unwrap();
    assert_eq!(outcome.recipient, Some(key));
    assert_eq!(outcome.delivery, FiniteDelivery::Delivered);
    assert_eq!(&*outcome.result.unwrap().data().unwrap().body, b"1");
    assert_eq!(dropped.get(), 1);
    assert_eq!(dispatches.borrow().len(), 1);
}

#[test]
fn failed_replacement_returns_exact_owner_and_does_not_change_current_authority() {
    let old = authority(204);
    let old_key = old.key();
    let dispatches = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(Cell::new(0));
    let mut servient = servient(
        vec![counted(old, b'0', &dispatches, &dropped)],
        registry_limits(),
    );
    let candidate_dropped = Rc::new(Cell::new(0));
    let candidate = counted(authority(205), b'5', &dispatches, &candidate_dropped);
    let candidate_key = candidate.key();
    let refusal = servient.replace(old_key, candidate).err().unwrap();
    assert_eq!(candidate_dropped.get(), 0);
    assert_eq!(refusal.input().key(), candidate_key);
    assert!(matches!(refusal.error().error(), CoreError::Validation(_)));
    let recovered = refusal.into_input();
    assert_eq!(recovered.key(), candidate_key);
    // Invalid same-slot behavior must also leave the current entry untouched.
    let invalid = RawProducerInstance::new(authority(204), vec![]);
    assert!(servient.replace(old_key, invalid).is_err());
    let binding = datagram(old_key, old_key);
    let addr = binding.addr.clone();
    let mut runtime = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    start(&mut servient, &mut runtime);
    udp_exchange(&mut servient, &mut runtime, addr.get().unwrap(), b'o', b'1');
    assert_eq!(dropped.get(), 0);
    assert_eq!(candidate_dropped.get(), 0);
    drop(recovered);
    assert_eq!(candidate_dropped.get(), 1);
}
