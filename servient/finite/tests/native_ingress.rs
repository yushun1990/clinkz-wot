//! External Binding with actual TCP I/O and a separate retained-reply probe.
#[path = "native_ingress/shared_ownership.rs"]
mod shared_ownership;
use clinkz_wot_core::{
    finite::{
        FiniteCandidate, FiniteDelivery, FiniteEffect, FiniteIngress, FiniteIngressBinding,
        RawFiniteHandler, RawFiniteReply,
    },
    producer::{ProducerInstanceAuthority, ProducerInstanceKey},
    *,
};
use clinkz_wot_foundation::{
    ClockId, Generation, MonotonicInstant, SlotIndex, WorkBudget, WorkClass,
};
use clinkz_wot_servient_finite::*;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 0)
}
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn budget(n: u64) -> WorkBudget {
    WorkBudget::new()
        .with_remaining(WorkClass::BindingPolls, n)
        .with_remaining(WorkClass::HandlerSteps, n)
        .with_remaining(WorkClass::CleanupItems, n)
}
fn limits() -> RawFiniteLimits {
    RawFiniteLimits {
        candidates: 2,
        input_body_bytes: 16,
        output_body_bytes: 16,
        output_media_capacity: 32,
        output_retained_bytes: 256,
    }
}
fn registry_limits() -> RawProducerRegistryLimits {
    RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: 1,
        target_name_bytes: 64,
    }
}
fn servient(
    instances: Vec<RawProducerInstance>,
    limits: RawProducerRegistryLimits,
) -> RawProducerServient {
    RawProducerServient::try_new(instances, limits).unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn authority(slot: u32) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(slot), Generation::INITIAL))
        .unwrap()
}
fn error() -> CoreError {
    CoreError::Binding(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
struct Handler {
    calls: Arc<AtomicUsize>,
    value: &'static [u8],
}
impl RawFiniteHandler for Handler {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        if input.data.as_ref().is_some_and(|p| &*p.body == b"deny") {
            return Err(CoreError::Security {
                reason: SecurityFailureReason::AuthorizationDenied,
                context: ErrorContext::new(ErrorPhase::Validate, RetryClass::Never),
            });
        }
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert!(ctx.producer_instance().is_some());
        assert!(ctx.thing_id().is_none());
        assert!(ctx.plan_id().is_none());
        self.calls.fetch_add(1, Ordering::SeqCst);
        if ctx.operation() == Operation::WriteProperty {
            return Ok(InteractionOutput::empty());
        }
        Ok(InteractionOutput::with_data(Payload::new(
            self.value,
            "application/json",
        )))
    }
}
fn instance(
    authority: ProducerInstanceAuthority,
    operation: Operation,
    calls: Arc<AtomicUsize>,
    value: &'static [u8],
) -> RawProducerInstance {
    RawProducerInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property("value".into()),
            operation,
            Handler { calls, value },
        )],
    )
}

struct TcpBinding {
    config: u8,
    listener: Option<TcpListener>,
    pending: Option<TcpStream>,
    addr: Arc<Mutex<Option<SocketAddr>>>,
    a: ProducerInstanceKey,
    b: ProducerInstanceKey,
    polls: Arc<AtomicUsize>,
}
struct SocketReply(TcpStream);
impl FiniteIngressBinding for TcpBinding {
    type Reply = SocketReply;
    type Configuration = u8;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(3), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &u8 {
        &self.config
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        if self.listener.is_none() {
            let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| error())?;
            listener.set_nonblocking(true).map_err(|_| error())?;
            *self.addr.lock().unwrap() = Some(listener.local_addr().unwrap());
            self.listener = Some(listener);
        }
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        let mut socket = if let Some(pending) = self.pending.take() {
            pending
        } else {
            let (socket, _) = match self.listener.as_ref().unwrap().accept() {
                Ok(pair) => pair,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Poll::Pending,
                Err(_) => return Poll::Ready(Err(error())),
            };
            socket.set_nonblocking(true).map_err(|_| error())?;
            socket
        };
        let mut byte = [0];
        // No peer is allowed to block the manual driver. The native service
        // owns this socket and its reply opportunity, not a per-Thing listener.
        match socket.read(&mut byte) {
            Ok(1) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                self.pending = Some(socket);
                return Poll::Pending;
            }
            _ => return Poll::Pending,
        }
        let candidate = |key, op| FiniteCandidate {
            instance: key,
            target: AffordanceTarget::Property("value".into()),
            operation: op,
        };
        let candidates = match byte[0] {
            b'a' => vec![candidate(self.a, Operation::ReadProperty)],
            b'b' | b'd' => vec![candidate(self.b, Operation::WriteProperty)],
            b'm' => vec![
                candidate(self.a, Operation::ReadProperty),
                candidate(self.b, Operation::WriteProperty),
            ],
            b'o' => vec![candidate(self.b, Operation::WriteProperty)],
            b'v' => vec![candidate(self.b, Operation::ReadProperty)],
            _ => vec![],
        };
        let input = match byte[0] {
            b'd' => {
                InteractionInput::with_data(Payload::new(b"deny".as_slice(), "application/json"))
            }
            b'b' => InteractionInput::with_data(Payload::new(b"13".as_slice(), "application/json")),
            b'o' => InteractionInput::with_data(Payload::new(
                b"0123456789abcdefg".as_slice(),
                "application/json",
            )),
            _ => InteractionInput::empty(),
        };
        Poll::Ready(Ok(FiniteIngress {
            candidates,
            input,
            reply: SocketReply(socket),
        }))
    }
    fn poll_send(
        &mut self,
        _: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        let byte = match &reply.result {
            Ok(_) if reply.operation == Some(Operation::WriteProperty) => b'2',
            Ok(output) => output.data().map(|p| p.body[0]).unwrap_or(b'E'),
            Err(_) => b'E',
        };
        let mut socket = &reply.ingress.reply.0;
        match socket.write(&[byte]) {
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
        cause: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        let disposition = if let Some(ingress) = ingress {
            assert!(matches!(cause, Some(CoreError::Cancelled(_))));
            let mut socket = &ingress.reply.0;
            match socket.write(b"E") {
                Ok(1) => FiniteDelivery::Delivered,
                _ => FiniteDelivery::Unknown,
            }
        } else if let Some(reply) = reply {
            let mut socket = &reply.ingress.reply.0;
            // A native rejection for an unmatched/unaccepted request; after a
            // handler invocation the wire success may already be uncertain.
            match socket.write(b"E") {
                Ok(1) if reply.effect == FiniteEffect::NotInvoked => FiniteDelivery::Delivered,
                Ok(1) => FiniteDelivery::NotDelivered,
                _ => FiniteDelivery::Unknown,
            }
        } else {
            FiniteDelivery::NotDelivered
        };
        self.listener = None;
        self.pending = None;
        Poll::Ready(Ok(disposition))
    }
}
fn step<B: FiniteIngressBinding>(
    servient: &mut RawProducerServient,
    rt: &mut RawFiniteRuntime<B>,
) -> Poll<RawFiniteState> {
    servient
        .step(rt, &mut context(), &mut budget(5000), now())
        .unwrap()
}
fn exchange(
    servient: &mut RawProducerServient,
    rt: &mut RawFiniteRuntime<TcpBinding>,
    addr: SocketAddr,
    message: u8,
    expected: u8,
) {
    let mut client = TcpStream::connect(addr).unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    client.write_all(&[message]).unwrap();
    for state in [
        RawFiniteState::Dispatching,
        RawFiniteState::Delivering,
        RawFiniteState::Serving,
    ] {
        assert_eq!(step(servient, rt), Poll::Ready(state));
    }
    let mut response = [0];
    client.read_exact(&mut response).unwrap();
    assert_eq!(response[0], expected);
    let settled = rt.take_settlement().expect("each exchange is reported");
    assert_eq!(settled.delivery, FiniteDelivery::Delivered);
}

#[test]
fn one_native_listener_matches_two_instances_and_revocation_is_local() {
    let a = authority(1);
    let b = authority(2);
    let (ak, bk) = (a.key(), b.key());
    let calls_a = Arc::new(AtomicUsize::new(0));
    let calls_b = Arc::new(AtomicUsize::new(0));
    let polls = Arc::new(AtomicUsize::new(0));
    let addr = Arc::new(Mutex::new(None));
    let binding = TcpBinding {
        config: 7,
        listener: None,
        pending: None,
        addr: addr.clone(),
        a: ak,
        b: bk,
        polls: polls.clone(),
    };
    let mut servient = servient(
        vec![
            instance(a, Operation::ReadProperty, calls_a.clone(), b"1"),
            instance(b, Operation::WriteProperty, calls_b.clone(), b"2"),
        ],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        7,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    let addr = addr.lock().unwrap().unwrap();
    let mut early = TcpStream::connect(addr).unwrap();
    early.write_all(b"a").unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(calls_a.load(Ordering::SeqCst), 0);
    rt.activate_raw().unwrap();
    for state in [
        RawFiniteState::Dispatching,
        RawFiniteState::Delivering,
        RawFiniteState::Serving,
    ] {
        assert_eq!(step(&mut servient, &mut rt), Poll::Ready(state));
    }
    let mut first = [0];
    early.read_exact(&mut first).unwrap();
    assert_eq!(first, *b"1");
    let polls_before_ack = polls.load(Ordering::SeqCst);
    assert_eq!(
        servient
            .step(&mut rt, &mut context(), &mut budget(0), now())
            .unwrap(),
        Poll::Ready(RawFiniteState::Serving)
    );
    assert_eq!(polls.load(Ordering::SeqCst), polls_before_ack);
    assert_eq!(rt.take_settlement().unwrap().effect, FiniteEffect::Unknown);
    exchange(&mut servient, &mut rt, addr, b'b', b'2');
    exchange(&mut servient, &mut rt, addr, b'd', b'E'); // authorization precedes behavior
    exchange(&mut servient, &mut rt, addr, b'm', b'E'); // exclusive reply ambiguity refuses
    exchange(&mut servient, &mut rt, addr, b'o', b'E'); // retained input exceeds declared limit
    exchange(&mut servient, &mut rt, addr, b'v', b'E'); // no registered operation for matched Thing
    exchange(&mut servient, &mut rt, addr, b'x', b'E'); // zero native candidates
    // Native matching preceded revocation, but semantic acceptance has not.
    let mut stale = TcpStream::connect(addr).unwrap();
    stale.write_all(b"a").unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert!(servient.revoke(ak));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    let mut rejected = [0];
    stale.read_exact(&mut rejected).unwrap();
    assert_eq!(rejected, *b"E");
    assert_eq!(
        rt.take_settlement().unwrap().effect,
        FiniteEffect::NotInvoked
    );
    assert!(!servient.revoke(ak));
    exchange(&mut servient, &mut rt, addr, b'a', b'E');
    exchange(&mut servient, &mut rt, addr, b'b', b'2'); // sibling remains available
    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(calls_b.load(Ordering::SeqCst), 2);
    rt.begin_close();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert!(TcpStream::connect(addr).is_err());
}

#[test]
fn same_slot_new_authority_cannot_receive_old_native_candidate() {
    let old = authority(1);
    let replacement = authority(1);
    let calls = Arc::new(AtomicUsize::new(0));
    let addr = Arc::new(Mutex::new(None));
    let binding = TcpBinding {
        config: 7,
        listener: None,
        addr: addr.clone(),
        a: old.key(),
        b: replacement.key(),
        pending: None,
        polls: Arc::new(AtomicUsize::new(0)),
    };
    let mut servient = servient(
        vec![instance(
            replacement,
            Operation::ReadProperty,
            calls.clone(),
            b"1",
        )],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        7,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    exchange(
        &mut servient,
        &mut rt,
        addr.lock().unwrap().unwrap(),
        b'a',
        b'E',
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

struct RetainedReply(Arc<AtomicUsize>);
impl Drop for RetainedReply {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
struct PendingBinding {
    ingress: Option<FiniteIngress<RetainedReply>>,
    pending_send: Arc<AtomicBool>,
    pending_close: Arc<AtomicBool>,
    fail_send: Arc<AtomicBool>,
    fail_close: Arc<AtomicBool>,
}
impl FiniteIngressBinding for PendingBinding {
    type Reply = RetainedReply;
    type Configuration = ();
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(4), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &() {
        &()
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        self.ingress
            .take()
            .map_or(Poll::Pending, |i| Poll::Ready(Ok(i)))
    }
    fn poll_send(
        &mut self,
        _: &mut Context<'_>,
        _: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        if self.pending_send.load(Ordering::SeqCst) {
            Poll::Pending
        } else if self.fail_send.load(Ordering::SeqCst) {
            Poll::Ready(Err(error()))
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        ingress: Option<&FiniteIngress<Self::Reply>>,
        reply: Option<&RawFiniteReply<Self::Reply>>,
        _: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        assert!(!(ingress.is_some() && reply.is_some()));
        if self.pending_close.load(Ordering::SeqCst) {
            Poll::Pending
        } else if self.fail_close.load(Ordering::SeqCst) {
            Poll::Ready(Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))))
        } else {
            Poll::Ready(Ok(FiniteDelivery::Unknown))
        }
    }
}

#[test]
fn pending_reply_survives_revocation_and_closing_until_native_retirement() {
    let a = authority(5);
    let key = a.key();
    let dropped = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let pending_send = Arc::new(AtomicBool::new(true));
    let pending_close = Arc::new(AtomicBool::new(true));
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property("value".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(dropped.clone()),
        }),
        pending_send: pending_send.clone(),
        pending_close: pending_close.clone(),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let mut servient = servient(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        servient
            .step(&mut rt, &mut context(), &mut budget(0), now())
            .unwrap(),
        Poll::Pending
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(step(&mut servient, &mut rt), Poll::Pending);
    assert!(servient.revoke(key));
    rt.begin_close();
    assert_eq!(step(&mut servient, &mut rt), Poll::Pending);
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    assert!(rt.activate_raw().is_err());
    pending_close.store(false, Ordering::SeqCst);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let settlement = rt.take_settlement().unwrap();
    assert_eq!(settlement.effect, FiniteEffect::Unknown);
    assert_eq!(settlement.delivery, FiniteDelivery::Unknown);
    let output = settlement.result.unwrap();
    assert_eq!(&*output.data().unwrap().body, b"1");
    assert!(matches!(rt.first_cause(), Some(CoreError::Cancelled(_))));
    pending_send.store(false, Ordering::SeqCst);
}

#[test]
fn close_retains_full_unaccepted_input_until_native_retirement() {
    let a = authority(6);
    let body: Arc<[u8]> = Arc::from(b"held".as_slice());
    let dropped = Arc::new(AtomicUsize::new(0));
    let pending_close = Arc::new(AtomicBool::new(true));
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property("value".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::with_data(Payload::new(body.clone(), "application/json")),
            reply: RetainedReply(dropped.clone()),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: pending_close.clone(),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let mut servient = servient(
        vec![instance(
            a,
            Operation::ReadProperty,
            Arc::new(AtomicUsize::new(0)),
            b"1",
        )],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    rt.begin_close();
    assert_eq!(step(&mut servient, &mut rt), Poll::Pending);
    assert_eq!(Arc::strong_count(&body), 2);
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    pending_close.store(false, Ordering::SeqCst);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    let outcome = rt.take_settlement().unwrap();
    assert_eq!(outcome.effect, FiniteEffect::NotInvoked);
    assert_eq!(outcome.delivery, FiniteDelivery::Unknown);
    assert!(matches!(outcome.result, Err(CoreError::Cancelled(_))));
    assert_eq!(Arc::strong_count(&body), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn denied_input_and_reply_stay_owned_across_pending_send_and_failed_close() {
    let a = authority(7);
    let body: Arc<[u8]> = Arc::from(b"deny".as_slice());
    let dropped = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let pending_send = Arc::new(AtomicBool::new(true));
    let fail_close = Arc::new(AtomicBool::new(true));
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property("value".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::with_data(Payload::new(body.clone(), "application/json")),
            reply: RetainedReply(dropped.clone()),
        }),
        pending_send: pending_send.clone(),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: fail_close.clone(),
    };
    let mut servient = servient(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(step(&mut servient, &mut rt), Poll::Pending);
    assert_eq!(Arc::strong_count(&body), 2);
    rt.begin_close();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closing)
    );
    assert_eq!(Arc::strong_count(&body), 2);
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    fail_close.store(false, Ordering::SeqCst);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    let settled = rt.take_settlement().unwrap();
    assert_eq!(settled.effect, FiniteEffect::NotInvoked);
    assert_eq!(settled.delivery, FiniteDelivery::Unknown);
    assert!(matches!(settled.result, Err(CoreError::Security { .. })));
    assert_eq!(Arc::strong_count(&body), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    pending_send.store(false, Ordering::SeqCst);
}

#[test]
fn failure_keeps_original_reply_and_first_cause_during_failed_close() {
    let a = authority(9);
    let dropped = Arc::new(AtomicUsize::new(0));
    let fail_close = Arc::new(AtomicBool::new(true));
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property("value".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(dropped.clone()),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(true)),
        fail_close: fail_close.clone(),
    };
    let mut servient = servient(
        vec![instance(
            a,
            Operation::ReadProperty,
            Arc::new(AtomicUsize::new(0)),
            b"1",
        )],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closing)
    );
    assert!(matches!(rt.first_cause(), Some(CoreError::Binding(_))));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closing)
    );
    assert!(matches!(rt.first_cause(), Some(CoreError::Binding(_))));
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    fail_close.store(false, Ordering::SeqCst);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    let settlement = rt.take_settlement().unwrap();
    assert_eq!(settlement.effect, FiniteEffect::Unknown);
    assert_eq!(settlement.delivery, FiniteDelivery::Unknown);
}

#[test]
fn invalid_behavior_set_refuses_before_native_start_and_returns_owners() {
    let a = authority(8);
    let calls = Arc::new(AtomicUsize::new(0));
    let invalid = RawProducerInstance::new(
        a,
        vec![
            RawFiniteBehavior::new(
                AffordanceTarget::Property("".into()),
                Operation::ReadProperty,
                Handler {
                    calls: calls.clone(),
                    value: b"1",
                },
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property("".into()),
                Operation::ReadProperty,
                Handler { calls, value: b"2" },
            ),
        ],
    );
    let rejected = RawProducerServient::try_new(vec![invalid], registry_limits())
        .err()
        .unwrap();
    assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
    let instances = rejected.into_input();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].key().slot().slot(), SlotIndex::new(8));
}

#[test]
fn actual_binding_configuration_and_finite_operation_are_checked_before_effects() {
    let a = authority(10);
    let calls = Arc::new(AtomicUsize::new(0));
    let address = Arc::new(Mutex::new(None));
    let binding = TcpBinding {
        config: 7,
        listener: None,
        pending: None,
        addr: address.clone(),
        a: a.key(),
        b: a.key(),
        polls: Arc::new(AtomicUsize::new(0)),
    };
    let servient = servient(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry_limits(),
    );
    let refused = RawFiniteRuntime::try_new(
        binding,
        8,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .err()
    .unwrap();
    assert_eq!(*refused.input().0.configuration(), 7);
    assert!(address.lock().unwrap().is_none());
    let (binding, _) = refused.into_input();
    let authority = authority(11);
    let invalid = RawProducerInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Action("run".into()),
            Operation::InvokeAction,
            Handler { calls, value: b"1" },
        )],
    );
    let refused = RawProducerServient::try_new(vec![invalid], registry_limits())
        .err()
        .unwrap();
    assert!(matches!(refused.error().error(), CoreError::Validation(_)));
    assert_eq!(refused.into_input().len(), 1);
    assert_eq!(*binding.configuration(), 7);
    assert!(address.lock().unwrap().is_none());
}

#[test]
fn close_before_dispatch_returns_original_native_refusal_without_running_handler() {
    let a = authority(20);
    let calls = Arc::new(AtomicUsize::new(0));
    let addr = Arc::new(Mutex::new(None));
    let binding = TcpBinding {
        config: 7,
        listener: None,
        pending: None,
        addr: addr.clone(),
        a: a.key(),
        b: a.key(),
        polls: Arc::new(AtomicUsize::new(0)),
    };
    let mut servient = servient(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        7,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    let mut client = TcpStream::connect(addr.lock().unwrap().unwrap()).unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    client.write_all(b"a").unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    rt.begin_close();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    let mut response = [0];
    client.read_exact(&mut response).unwrap();
    assert_eq!(response, *b"E");
    let settlement = rt.take_settlement().unwrap();
    assert_eq!(settlement.effect, FiniteEffect::NotInvoked);
    assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
    assert!(matches!(settlement.result, Err(CoreError::Cancelled(_))));
    assert!(matches!(rt.first_cause(), Some(CoreError::Cancelled(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn close_after_write_reports_unknown_effect_and_undelivered_success() {
    let a = authority(21);
    let calls = Arc::new(AtomicUsize::new(0));
    let addr = Arc::new(Mutex::new(None));
    let binding = TcpBinding {
        config: 7,
        listener: None,
        pending: None,
        addr: addr.clone(),
        a: a.key(),
        b: a.key(),
        polls: Arc::new(AtomicUsize::new(0)),
    };
    let mut servient = servient(
        vec![instance(a, Operation::WriteProperty, calls.clone(), b"1")],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        7,
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    let mut client = TcpStream::connect(addr.lock().unwrap().unwrap()).unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    client.write_all(b"b").unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    rt.begin_close();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Closed)
    );
    let mut response = [0];
    client.read_exact(&mut response).unwrap();
    assert_eq!(response, *b"E");
    let settlement = rt.take_settlement().unwrap();
    assert_eq!(settlement.effect, FiniteEffect::Unknown);
    assert_eq!(settlement.delivery, FiniteDelivery::NotDelivered);
    assert!(settlement.result.is_ok());
    assert!(matches!(rt.first_cause(), Some(CoreError::Cancelled(_))));
}

struct FixedHandler {
    calls: Arc<AtomicUsize>,
    outcome: Mutex<Option<CoreResult<InteractionOutput>>>,
}
impl RawFiniteHandler for FixedHandler {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.outcome.lock().unwrap().take().unwrap()
    }
}

fn fixed_exchange(
    operation: Operation,
    outcome: CoreResult<InteractionOutput>,
    limits: RawFiniteLimits,
) -> (
    RawProducerServient,
    RawFiniteRuntime<PendingBinding>,
    Arc<AtomicUsize>,
) {
    let a = authority(22);
    let calls = Arc::new(AtomicUsize::new(0));
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property("value".into()),
                operation,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(Arc::new(AtomicUsize::new(0))),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let instance = RawProducerInstance::new(
        a,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property("value".into()),
            operation,
            FixedHandler {
                calls: calls.clone(),
                outcome: Mutex::new(Some(outcome)),
            },
        )],
    );
    let mut servient = servient(vec![instance], registry_limits());
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits,
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    (servient, rt, calls)
}

#[test]
fn raw_property_success_requires_operation_shape_not_merely_a_small_body() {
    let binding_metadata = BindingResponseMetadata::primary(
        BindingId::new(4),
        BindingGeneration::INITIAL,
        PlanId::new(SlotIndex::new(1), Generation::INITIAL),
        200,
    );
    let action_metadata = InteractionOutputMetadata::default().with_action_invocation(
        ActionInvocationRef::new(SlotIndex::new(1), Generation::INITIAL),
    );
    let cases = [
        (Operation::ReadProperty, InteractionOutput::empty()),
        (
            Operation::ReadProperty,
            InteractionOutput::with_data(Payload::new(b"1".as_slice(), "application/json"))
                .with_status(InteractionStatus::Accepted),
        ),
        (
            Operation::ReadProperty,
            InteractionOutput::with_data(Payload::new(b"1".as_slice(), "application/json"))
                .try_with_metadata(action_metadata)
                .unwrap(),
        ),
        (
            Operation::ReadProperty,
            InteractionOutput::with_data(Payload::new(b"1".as_slice(), "application/json"))
                .try_with_metadata(
                    InteractionOutputMetadata::default()
                        .with_untrusted_binding_response(binding_metadata),
                )
                .unwrap(),
        ),
        (
            Operation::WriteProperty,
            InteractionOutput::empty().with_status(InteractionStatus::Accepted),
        ),
        (
            Operation::WriteProperty,
            InteractionOutput::with_data(Payload::new(b"1".as_slice(), "application/json")),
        ),
        (
            Operation::WriteProperty,
            InteractionOutput::empty()
                .try_with_metadata(action_metadata)
                .unwrap(),
        ),
    ];
    for (operation, output) in cases {
        let (_servient, mut rt, calls) = fixed_exchange(operation, Ok(output), limits());
        let outcome = rt.take_settlement().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(outcome.effect, FiniteEffect::Unknown);
        assert_eq!(outcome.delivery, FiniteDelivery::Delivered);
        assert!(matches!(outcome.result, Err(CoreError::Validation(_))));
    }
}

#[test]
fn retained_output_counts_arc_and_both_string_capacities_and_refuses_one_over() {
    let mut media = String::with_capacity(16);
    media.push_str("application/json");
    let mut coding = String::with_capacity(8);
    coding.push_str("identity");
    let payload = Payload::new(b"1".as_slice(), media).with_content_coding(coding);
    let output = InteractionOutput::with_data(payload);
    let bytes = clinkz_wot_core::interaction_output_backing(&output).unwrap();
    let mut exact = limits();
    exact.output_body_bytes = 1;
    exact.output_media_capacity = 16;
    exact.output_retained_bytes = bytes;
    let (_servient, mut rt, _) = fixed_exchange(Operation::ReadProperty, Ok(output.clone()), exact);
    assert!(rt.take_settlement().unwrap().result.is_ok());
    exact.output_retained_bytes -= 1;
    let (_servient, mut rt, _) = fixed_exchange(Operation::ReadProperty, Ok(output), exact);
    let refused = rt.take_settlement().unwrap();
    assert_eq!(refused.effect, FiniteEffect::Unknown);
    assert!(matches!(refused.result, Err(CoreError::Backpressure(_))));

    // A tiny body with spare String capacity must not escape the media ceiling.
    let mut oversized = String::with_capacity(17);
    oversized.push_str("application/json");
    let payload = Payload::new(b"".as_slice(), oversized);
    let mut zero_body = limits();
    zero_body.output_body_bytes = 0;
    zero_body.output_media_capacity = 16;
    let (_servient, mut rt, _) = fixed_exchange(
        Operation::ReadProperty,
        Ok(InteractionOutput::with_data(payload)),
        zero_body,
    );
    assert!(matches!(
        rt.take_settlement().unwrap().result,
        Err(CoreError::Backpressure(_))
    ));
}

#[test]
fn write_handler_error_and_oversized_result_never_prove_effect_absent() {
    let (_servient, mut rt, calls) =
        fixed_exchange(Operation::WriteProperty, Err(error()), limits());
    let error_result = rt.take_settlement().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(error_result.effect, FiniteEffect::Unknown);
    assert!(matches!(error_result.result, Err(CoreError::Binding(_))));

    let mut tiny = limits();
    tiny.output_body_bytes = 0;
    let oversized = InteractionOutput::with_data(Payload::new(b"1".as_slice(), "application/json"));
    let (_servient, mut rt, calls) = fixed_exchange(Operation::WriteProperty, Ok(oversized), tiny);
    let overrun = rt.take_settlement().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(overrun.effect, FiniteEffect::Unknown);
    assert!(matches!(overrun.result, Err(CoreError::Backpressure(_))));
}

#[test]
fn worst_case_registry_lookup_is_prepaid_and_zero_credit_is_inert() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut instances = Vec::new();
    let mut last = None;
    for slot in 0..32 {
        let a = authority(100 + slot);
        last = Some(a.key());
        let behaviors = (0..16)
            .map(|n| {
                RawFiniteBehavior::new(
                    AffordanceTarget::Property(format!("b{n}").into()),
                    Operation::ReadProperty,
                    Handler {
                        calls: count.clone(),
                        value: b"1",
                    },
                )
            })
            .collect();
        instances.push(RawProducerInstance::new(a, behaviors));
    }
    let key = last.unwrap();
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: key,
                target: AffordanceTarget::Property("b15".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(Arc::new(AtomicUsize::new(0))),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let max = limits();
    let mut registry = registry_limits();
    registry.instances = 32;
    registry.behaviors_per_instance = 16;
    registry.target_name_bytes = 3;
    let mut servient = servient(instances, registry);
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        max,
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    let mut insufficient = budget(96);
    assert_eq!(
        servient
            .step(&mut rt, &mut context(), &mut insufficient, now())
            .unwrap(),
        Poll::Pending
    );
    assert_eq!(insufficient.remaining(WorkClass::HandlerSteps), 96);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let mut exact = budget(97);
    assert_eq!(
        servient
            .step(&mut rt, &mut context(), &mut exact, now())
            .unwrap(),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(exact.remaining(WorkClass::HandlerSteps), 0);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    assert_eq!(rt.take_settlement().unwrap().recipient, Some(key));
}

#[test]
fn equal_prefix_target_misses_pay_for_every_bounded_name_byte() {
    let a = authority(140);
    let prefix = "x".repeat(63);
    let calls = Arc::new(AtomicUsize::new(0));
    let behaviors = (b'A'..=b'P')
        .map(|suffix| {
            RawFiniteBehavior::new(
                AffordanceTarget::Property(format!("{prefix}{}", char::from(suffix)).into()),
                Operation::ReadProperty,
                Handler {
                    calls: calls.clone(),
                    value: b"1",
                },
            )
        })
        .collect();
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property(format!("{prefix}Z").into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(Arc::new(AtomicUsize::new(0))),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let max = limits();
    let mut registry = registry_limits();
    registry.instances = 1;
    registry.behaviors_per_instance = 16;
    registry.target_name_bytes = 64;
    let mut servient = servient(vec![RawProducerInstance::new(a, behaviors)], registry);
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        max,
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    // 1 instance + 16 * (1 length/discriminant + 64 bytes) + 1 callback.
    let mut insufficient = budget(1041);
    assert_eq!(
        servient
            .step(&mut rt, &mut context(), &mut insufficient, now())
            .unwrap(),
        Poll::Pending
    );
    assert_eq!(insufficient.remaining(WorkClass::HandlerSteps), 1041);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut exact = budget(1042);
    assert_eq!(
        servient
            .step(&mut rt, &mut context(), &mut exact, now())
            .unwrap(),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(exact.remaining(WorkClass::HandlerSteps), 0);
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    let settled = rt.take_settlement().unwrap();
    assert_eq!(settled.effect, FiniteEffect::NotInvoked);
    assert!(matches!(
        settled.result,
        Err(CoreError::UnsupportedOperation(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn target_name_limit_checks_both_registration_and_native_candidate() {
    let a = authority(141);
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Property("values".into()),
                operation: Operation::ReadProperty,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(Arc::new(AtomicUsize::new(0))),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let max = limits();
    let mut registry = registry_limits();
    registry.target_name_bytes = 5;
    let mut servient = servient(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry,
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        max,
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    let refused = rt.take_settlement().unwrap();
    assert_eq!(refused.effect, FiniteEffect::NotInvoked);
    assert!(matches!(refused.result, Err(CoreError::Backpressure(_))));
    // A fresh authority tests pre-effect registration refusal as well.
    let a = authority(142);
    registry.target_name_bytes = 4;
    let rejected = RawProducerServient::try_new(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry,
    )
    .err()
    .unwrap();
    assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn thing_level_candidate_is_unsupported_not_a_name_capacity_failure() {
    let a = authority(143);
    let calls = Arc::new(AtomicUsize::new(0));
    let binding = PendingBinding {
        ingress: Some(FiniteIngress {
            candidates: vec![FiniteCandidate {
                instance: a.key(),
                target: AffordanceTarget::Thing,
                operation: Operation::ReadAllProperties,
            }],
            input: InteractionInput::empty(),
            reply: RetainedReply(Arc::new(AtomicUsize::new(0))),
        }),
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let mut servient = servient(
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        registry_limits(),
    );
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        &servient,
        limits(),
        FiniteOutputValidation::Raw,
        Deadline::NONE,
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    rt.activate_raw().unwrap();
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Dispatching)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Delivering)
    );
    assert_eq!(
        step(&mut servient, &mut rt),
        Poll::Ready(RawFiniteState::Serving)
    );
    let settled = rt.take_settlement().unwrap();
    assert_eq!(settled.effect, FiniteEffect::NotInvoked);
    assert_eq!(settled.delivery, FiniteDelivery::Delivered);
    assert!(matches!(
        settled.result,
        Err(CoreError::UnsupportedOperation(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn settlement_debug_is_bounded_and_redacts_owned_output() {
    let mut max = limits();
    max.output_body_bytes = 2048;
    max.output_media_capacity = 64;
    max.output_retained_bytes = 4096;
    let output =
        InteractionOutput::with_data(Payload::new(vec![b's'; 2048], "private-media-marker"));
    let (_servient, mut rt, _) = fixed_exchange(Operation::ReadProperty, Ok(output), max);
    let settled = rt.take_settlement().unwrap();
    let diagnostic = format!("{settled:?}");
    assert!(
        diagnostic.len() < 512,
        "diagnostics must not scale with payload"
    );
    assert!(!diagnostic.contains("private-media-marker"));
    assert!(!diagnostic.contains("115, 115, 115"));
    assert_eq!(settled.result.unwrap().data().unwrap().body.len(), 2048);
}
