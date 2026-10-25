//! External Binding with actual TCP I/O and a separate retained-reply probe.
use clinkz_wot_core::{
    finite::{
        FiniteCandidate, FiniteIngress, FiniteIngressBinding, RawFiniteHandler, RawFiniteReply,
    },
    producer::{ProducerInstanceAuthority, ProducerInstanceKey},
    *,
};
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget, WorkClass};
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
    }
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
) -> RawFiniteInstance {
    RawFiniteInstance::new(
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
        let byte = reply
            .result
            .as_ref()
            .ok()
            .and_then(InteractionOutput::data)
            .map(|p| p.body[0])
            .unwrap_or(b'E');
        let mut socket = &reply.opportunity.0;
        match socket.write(&[byte]) {
            Ok(1) => Poll::Ready(Ok(())),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            _ => Poll::Ready(Err(error())),
        }
    }
    fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.listener = None;
        self.pending = None;
        Poll::Ready(Ok(()))
    }
}
fn step<B: FiniteIngressBinding>(rt: &mut RawFiniteRuntime<B>) -> Poll<RawFiniteState> {
    rt.step(&mut context(), &mut budget(1))
}
fn exchange(rt: &mut RawFiniteRuntime<TcpBinding>, addr: SocketAddr, message: u8, expected: u8) {
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
        assert_eq!(step(rt), Poll::Ready(state));
    }
    let mut response = [0];
    client.read_exact(&mut response).unwrap();
    assert_eq!(response[0], expected);
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
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        7,
        vec![
            instance(a, Operation::ReadProperty, calls_a.clone(), b"1"),
            instance(b, Operation::WriteProperty, calls_b.clone(), b"2"),
        ],
        limits(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::InstalledClosed));
    let addr = addr.lock().unwrap().unwrap();
    let mut early = TcpStream::connect(addr).unwrap();
    early.write_all(b"a").unwrap();
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::InstalledClosed));
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(calls_a.load(Ordering::SeqCst), 0);
    rt.activate_raw().unwrap();
    for state in [
        RawFiniteState::Dispatching,
        RawFiniteState::Delivering,
        RawFiniteState::Serving,
    ] {
        assert_eq!(step(&mut rt), Poll::Ready(state));
    }
    let mut first = [0];
    early.read_exact(&mut first).unwrap();
    assert_eq!(first, *b"1");
    exchange(&mut rt, addr, b'b', b'2');
    exchange(&mut rt, addr, b'd', b'E'); // authorization precedes behavior
    exchange(&mut rt, addr, b'm', b'E'); // exclusive reply ambiguity refuses
    exchange(&mut rt, addr, b'o', b'E'); // retained input exceeds declared limit
    exchange(&mut rt, addr, b'v', b'E'); // no registered operation for matched Thing
    exchange(&mut rt, addr, b'x', b'E'); // zero native candidates
    // Native matching preceded revocation, but semantic acceptance has not.
    let mut stale = TcpStream::connect(addr).unwrap();
    stale.write_all(b"a").unwrap();
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Dispatching));
    assert!(rt.revoke(ak));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Delivering));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Serving));
    let mut rejected = [0];
    stale.read_exact(&mut rejected).unwrap();
    assert_eq!(rejected, *b"E");
    assert!(!rt.revoke(ak));
    exchange(&mut rt, addr, b'a', b'E');
    exchange(&mut rt, addr, b'b', b'2'); // sibling remains available
    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(calls_b.load(Ordering::SeqCst), 2);
    rt.begin_close();
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Closed));
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
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        7,
        vec![instance(
            replacement,
            Operation::ReadProperty,
            calls.clone(),
            b"1",
        )],
        limits(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::InstalledClosed));
    rt.activate_raw().unwrap();
    exchange(&mut rt, addr.lock().unwrap().unwrap(), b'a', b'E');
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
    fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        if self.pending_close.load(Ordering::SeqCst) {
            Poll::Pending
        } else if self.fail_close.load(Ordering::SeqCst) {
            Poll::Ready(Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Cleanup,
                RetryClass::Never,
            ))))
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

#[test]
fn pending_reply_survives_revocation_and_closing_until_native_retirement() {
    let a = authority(5);
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
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")],
        limits(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::InstalledClosed));
    rt.activate_raw().unwrap();
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Dispatching));
    assert_eq!(rt.step(&mut context(), &mut budget(0)), Poll::Pending);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Delivering));
    assert_eq!(step(&mut rt), Poll::Pending);
    rt.begin_close();
    assert_eq!(step(&mut rt), Poll::Pending);
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    assert!(rt.activate_raw().is_err());
    pending_close.store(false, Ordering::SeqCst);
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Closed));
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
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
    let mut rt = RawFiniteRuntime::try_new(
        binding,
        (),
        vec![instance(
            a,
            Operation::ReadProperty,
            Arc::new(AtomicUsize::new(0)),
            b"1",
        )],
        limits(),
    )
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::InstalledClosed));
    rt.activate_raw().unwrap();
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Dispatching));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Delivering));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Closing));
    assert!(matches!(rt.first_cause(), Some(CoreError::Binding(_))));
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Closing));
    assert!(matches!(rt.first_cause(), Some(CoreError::Binding(_))));
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    fail_close.store(false, Ordering::SeqCst);
    assert_eq!(step(&mut rt), Poll::Ready(RawFiniteState::Closed));
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn invalid_behavior_set_refuses_before_native_start_and_returns_owners() {
    let a = authority(8);
    let binding = PendingBinding {
        ingress: None,
        pending_send: Arc::new(AtomicBool::new(false)),
        pending_close: Arc::new(AtomicBool::new(false)),
        fail_send: Arc::new(AtomicBool::new(false)),
        fail_close: Arc::new(AtomicBool::new(false)),
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let invalid = RawFiniteInstance::new(
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
    let rejected = RawFiniteRuntime::try_new(binding, (), vec![invalid], limits())
        .err()
        .unwrap();
    assert!(matches!(rejected.error().error(), CoreError::Validation(_)));
    let (binding, (), instances) = rejected.into_input();
    assert!(binding.ingress.is_none());
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
    let original = vec![instance(a, Operation::ReadProperty, calls.clone(), b"1")];
    let refused = RawFiniteRuntime::try_new(binding, 8, original, limits())
        .err()
        .unwrap();
    assert_eq!(*refused.input().0.configuration(), 7);
    assert!(address.lock().unwrap().is_none());
    let (binding, _, instances) = refused.into_input();
    let authority = authority(11);
    let invalid = RawFiniteInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Action("run".into()),
            Operation::InvokeAction,
            Handler { calls, value: b"1" },
        )],
    );
    let refused = RawFiniteRuntime::try_new(binding, 7, vec![invalid], limits())
        .err()
        .unwrap();
    assert!(matches!(refused.error().error(), CoreError::Validation(_)));
    assert_eq!(instances.len(), 1);
    assert!(address.lock().unwrap().is_none());
}
