#![cfg(feature = "prepare")]
//! Downstream Action facet: independently typed invoke/control Bindings, native
//! datagrams, opaque continuation and delayed/failed physical retirement.
use clinkz_wot_core::{consumer::*, consumer_action::*, host_accounts::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::open_consumer::*;
use clinkz_wot_servient_finite::consumer_action::*;
use independent_open_consumer::{Probe, configuration, services};
use std::{
    any::Any,
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
#[derive(Default)]
struct NativeProbe {
    close_blocked: AtomicBool,
    fail_close: AtomicBool,
    replies: AtomicUsize,
    live: AtomicUsize,
    drops_funded: AtomicUsize,
    bank: std::sync::Mutex<Option<Arc<HostResourceAccounts>>>,
    ports: std::sync::Mutex<Vec<SocketAddr>>,
}
#[derive(Clone)]
struct Native {
    id: BindingRegistrationIdentity,
    peer: SocketAddr,
    endpoint: Box<str>,
    operation: Operation,
    probe: Arc<NativeProbe>,
}
struct Invoke(Native);
struct Control(Native);
struct Token(u8);
impl Native {
    fn start(
        &self,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        let bytes = match self.operation {
            Operation::InvokeAction if continuation.is_none() => {
                assert_eq!(input.unwrap().body.as_ref(), b"12");
                b"INVOKE 12".to_vec()
            }
            Operation::QueryAction | Operation::CancelAction if input.is_none() => {
                let token = continuation
                    .and_then(|t| t.downcast_ref::<Token>())
                    .ok_or_else(invalid)?;
                assert_eq!(token.0, 7);
                if self.operation == Operation::QueryAction {
                    b"QUERY 7".to_vec()
                } else {
                    b"CANCEL 7".to_vec()
                }
            }
            _ => return Err(invalid()),
        };
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        self.probe
            .ports
            .lock()
            .unwrap()
            .push(socket.local_addr().unwrap());
        socket.connect(self.peer).unwrap();
        socket.set_nonblocking(true).unwrap();
        self.probe.live.fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(Exchange {
            socket: Some(socket),
            bytes,
            sent: false,
            answered: false,
            probe: self.probe.clone(),
        }))
    }
}
macro_rules! mapping {
    ($ty:ty) => {
        impl ConsumerActionMapping for $ty {
            fn identity(&self) -> BindingRegistrationIdentity {
                self.0.id
            }
            fn operation(&self) -> Operation {
                self.0.operation
            }
            fn bounds(&self) -> ConsumerActionBounds {
                ConsumerActionBounds {
                    exchange_bytes: 2048,
                    temporary_bytes: 256,
                    continuation_bytes: 64,
                    response_bytes: 2048,
                    poll_work: 32,
                }
            }
            fn native_definition(&self) -> &dyn Any {
                self
            }
            fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()> {
                let origin = invoke
                    .native_definition()
                    .downcast_ref::<Invoke>()
                    .ok_or_else(invalid)?;
                if self.0.operation == Operation::InvokeAction || origin.0.peer != self.0.peer {
                    return Err(invalid());
                }
                Ok(())
            }
            fn destination(&self, _: Option<&dyn Any>) -> CoreResult<Box<str>> {
                Ok(self.0.endpoint.clone())
            }
            fn start(
                &self,
                input: Option<&Payload>,
                continuation: Option<&dyn Any>,
            ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
                self.0.start(input, continuation)
            }
        }
    };
}
mapping!(Invoke);
mapping!(Control);
struct Exchange {
    socket: Option<UdpSocket>,
    bytes: Vec<u8>,
    sent: bool,
    answered: bool,
    probe: Arc<NativeProbe>,
}
impl ConsumerActionExchange for Exchange {
    fn poll(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>> {
        let s = self.socket.as_ref().unwrap();
        if !self.sent {
            s.send(&self.bytes).unwrap();
            self.sent = true;
        }
        let mut data = [0u8; 128];
        match s.recv(&mut data) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            Err(_) => Poll::Ready(Err(invalid())),
            Ok(n) => {
                self.answered = true;
                self.probe.replies.fetch_add(1, Ordering::Relaxed);
                Poll::Ready(match &data[..n] {
                    b"ACCEPT 7" => Ok(ConsumerActionReply::Accepted(Box::new(Token(7)))),
                    b"WAIT" => Ok(ConsumerActionReply::Pending),
                    b"STOP REQUESTED" => Ok(ConsumerActionReply::CancelRequested),
                    bytes if bytes.starts_with(b"OUTPUT ") => Ok(ConsumerActionReply::Complete(
                        InteractionOutput::with_data(Payload::new(&bytes[7..], "application/json")),
                    )),
                    _ => Err(invalid()),
                })
            }
        }
    }
    fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<Option<ConsumerActionReply>>> {
        if self.probe.fail_close.swap(false, Ordering::Relaxed) {
            return Poll::Ready(Err(invalid()));
        }
        if self.probe.close_blocked.load(Ordering::Relaxed) {
            return Poll::Pending;
        }
        self.socket = None;
        Poll::Ready(Ok(None))
    }
    fn abort_local(self: Box<Self>) {
        drop(self);
    }
}
impl Drop for Exchange {
    fn drop(&mut self) {
        self.socket = None;
        if self
            .probe
            .bank
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .account(ResourceKind::CleanupItemsMax)
            .used()
            > 0
        {
            self.probe.drops_funded.fetch_add(1, Ordering::Relaxed);
        }
        self.probe.live.fetch_sub(1, Ordering::Relaxed);
    }
}
struct Compiler {
    id: BindingRegistrationIdentity,
    invoke: bool,
    peer: SocketAddr,
    probe: Arc<NativeProbe>,
}
impl ConsumerActionPreparation for Compiler {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.id
    }
    fn bounds(&self) -> ConsumerPreparationBounds {
        ConsumerPreparationBounds {
            work: 32,
            temporary_bytes: 256,
            retained_bytes: 1024,
        }
    }
    fn compile_action(
        &self,
        input: ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        if !input.endpoint.starts_with("linkedudp://")
            || self.invoke != (input.operation == Operation::InvokeAction)
        {
            return Ok(None);
        }
        let native = Native {
            id: self.id,
            peer: self.peer,
            endpoint: input.endpoint.into(),
            operation: input.operation,
            probe: self.probe.clone(),
        };
        Ok(Some(if self.invoke {
            Box::new(Invoke(native))
        } else {
            Box::new(Control(native))
        }))
    }
}
struct Access;
impl ConsumerAccessService for Access {
    fn authorize(&self, _: BindingRegistrationIdentity, t: &str, _: Operation) -> CoreResult<()> {
        if t.starts_with("linkedudp://127.0.0.1:") {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn authorize_instance(
        &self,
        _: &ConsumerInstanceRef,
        b: BindingRegistrationIdentity,
        t: &str,
        o: Operation,
    ) -> CoreResult<()> {
        self.authorize(b, t, o)
    }
    fn work(&self) -> u64 {
        1
    }
    fn retained_bytes(&self) -> u64 {
        0
    }
}
fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, c| b.with_remaining(c, 1_000_000))
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 0)
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
fn instance() -> ConsumerInstance {
    ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(1), Generation::INITIAL))
}
fn client(
    peer: SocketAddr,
    bank: Arc<HostResourceAccounts>,
    probe: Arc<NativeProbe>,
    data: Arc<Probe>,
    owner: &ConsumerInstance,
) -> ConsumerActionRuntime {
    *probe.bank.lock().unwrap() = Some(bank.clone());
    let registry = ConsumerActionRegistry::try_new(
        (0..2)
            .map(|i| {
                let mut services = services(data.clone());
                services.access = Arc::new(Access);
                ConsumerActionRegistration {
                    preparation: Box::new(Compiler {
                        id: BindingRegistrationIdentity::new(
                            BindingId::new(70 + i),
                            BindingGeneration::INITIAL,
                            configuration().configuration(),
                            configuration().native_compatibility().unwrap(),
                            i,
                        ),
                        invoke: i == 0,
                        peer,
                        probe: probe.clone(),
                    }),
                    services,
                }
            })
            .collect(),
    )
    .unwrap();
    let td = serde_json::from_value(serde_json::json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"native Action", "security":["n"], "securityDefinitions":{"n":{"scheme":"nosec"}},
        "actions":{"run":{"input":{"type":"integer"}, "output":{"type":"integer"}, "forms":[
            {"href":format!("linkedudp://{peer}/action"), "op":["invokeaction","queryaction","cancelaction"]}]}}})).unwrap();
    let d = registry
        .prepare_action(
            &td,
            "run",
            ConsumerActionSelectors::default(),
            GatewayDefaultV1::LIMITS,
            bank,
            &mut work(),
        )
        .unwrap();
    drop(td);
    drop(registry);
    ConsumerActionRuntime::try_new(owner.reference(), vec![d], 1)
        .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
fn drive(h: &ConsumerActionHandle, expected: ConsumerActionState) {
    let start = Instant::now();
    loop {
        if h.progress(&mut cx(), now(), &mut work()).unwrap() == expected {
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::yield_now();
    }
}
fn clean(b: &HostResourceAccounts) {
    for k in ResourceKind::ALL {
        assert_eq!(b.account(k).used(), 0, "{k:?}");
    }
}
fn peer(
    messages: Vec<(&'static [u8], &'static [u8])>,
) -> (SocketAddr, std::thread::JoinHandle<()>) {
    let s = UdpSocket::bind("127.0.0.1:0").unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let address = s.local_addr().unwrap();
    (
        address,
        std::thread::spawn(move || {
            for (expected, answer) in messages {
                let mut bytes = [0u8; 128];
                let (n, address) = s.recv_from(&mut bytes).unwrap();
                assert_eq!(&bytes[..n], expected);
                s.send_to(answer, address).unwrap();
            }
        }),
    )
}
#[test]
fn distinct_downstream_invoke_and_control_bindings_execute_native_action_lifecycle() {
    let (address, peer) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"WAIT"),
        (b"CANCEL 7", b"STOP REQUESTED"),
        (b"QUERY 7", b"OUTPUT 24"),
    ]);
    let b = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let p = Arc::new(NativeProbe::default());
    let data = Arc::new(Probe::default());
    let owner = instance();
    let runtime = client(address, b.clone(), p.clone(), data.clone(), &owner);
    let h = runtime
        .invoke_action(
            "run",
            Some(Payload::new(b"12".as_slice(), "application/json")),
            Deadline::NONE,
            now(),
            &mut work(),
        )
        .ok()
        .unwrap();
    assert_eq!(
        h.progress(&mut cx(), now(), &mut WorkBudget::new())
            .unwrap(),
        ConsumerActionState::Invoking
    );
    assert_eq!(p.replies.load(Ordering::Relaxed), 0);
    drive(&h, ConsumerActionState::Pending);
    h.query(now(), &mut work()).unwrap();
    drive(&h, ConsumerActionState::Pending);
    h.request_cancel(now(), &mut work()).unwrap();
    drive(&h, ConsumerActionState::Pending);
    h.query(now(), &mut work()).unwrap();
    drive(&h, ConsumerActionState::Completed);
    let s = h.take_settlement(now(), &mut work()).unwrap();
    assert!(s.accepted());
    assert_eq!(
        s.remote_cancellation(),
        ConsumerActionCancellation::Requested
    );
    assert_eq!(s.result().unwrap().data().unwrap().body.as_ref(), b"24");
    assert_ne!(
        s.selected_form(Operation::InvokeAction).unwrap().0,
        s.selected_form(Operation::QueryAction).unwrap().0
    );
    assert_eq!(p.live.load(Ordering::Relaxed), 0);
    assert_eq!(p.drops_funded.load(Ordering::Relaxed), 4);
    drop(h);
    drop(runtime);
    drop(owner);
    assert!(b.account(ResourceKind::EngineLiveBytesGlobalMax).used() > 0);
    drop(s);
    clean(&b);
    peer.join().unwrap();
}
#[test]
fn delayed_cleanup_keeps_late_output_validation_first_cause_and_original_funding() {
    let (address, peer) = peer(vec![
        (b"INVOKE 12", b"ACCEPT 7"),
        (b"QUERY 7", b"OUTPUT invalid"),
    ]);
    let b = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let p = Arc::new(NativeProbe::default());
    let data = Arc::new(Probe::default());
    let owner = instance();
    let runtime = client(address, b.clone(), p.clone(), data.clone(), &owner);
    let h = runtime
        .invoke_action(
            "run",
            Some(Payload::new(b"12".as_slice(), "application/json")),
            Deadline::NONE,
            now(),
            &mut work(),
        )
        .ok()
        .unwrap();
    drive(&h, ConsumerActionState::Pending);
    p.close_blocked.store(true, Ordering::Relaxed);
    h.query(now(), &mut work()).unwrap();
    let start = Instant::now();
    while p.replies.load(Ordering::Relaxed) < 2 {
        h.progress(&mut cx(), now(), &mut work()).unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::yield_now();
    }
    h.cancel_local();
    owner.revoke();
    p.fail_close.store(true, Ordering::Relaxed);
    h.progress(&mut cx(), now(), &mut work()).unwrap();
    assert!(h.take_settlement(now(), &mut work()).is_err());
    assert_eq!(p.live.load(Ordering::Relaxed), 1);
    assert!(b.account(ResourceKind::CleanupBytesMax).used() > 0);
    // Losing the root does not dispose work retained by its original handle.
    drop(runtime);
    let replacement = instance();
    drop(replacement);
    p.close_blocked.store(false, Ordering::Relaxed);
    drive(&h, ConsumerActionState::Closed);
    let s = h.take_settlement(now(), &mut work()).unwrap();
    assert!(matches!(s.first_cause(), Some(CoreError::Cancelled(_))));
    assert!(s.cleanup_error().is_some());
    assert!(s.validation_error().is_some());
    assert!(s.native_result().unwrap().is_ok());
    assert!(s.result().is_err());
    assert!(s.accepted());
    assert_eq!(data.validates.load(Ordering::Relaxed), 2);
    assert_eq!(p.live.load(Ordering::Relaxed), 0);
    assert_eq!(p.drops_funded.load(Ordering::Relaxed), 2);
    drop(h);
    drop(owner);
    drop(s);
    clean(&b);
    peer.join().unwrap();
}

#[test]
fn last_local_owner_uses_funded_nonblocking_abort_without_remote_cancel() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_nonblocking(true).unwrap();
    let b = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
    let p = Arc::new(NativeProbe::default());
    let data = Arc::new(Probe::default());
    let owner = instance();
    let runtime = client(
        peer.local_addr().unwrap(),
        b.clone(),
        p.clone(),
        data,
        &owner,
    );
    let h = runtime
        .invoke_action(
            "run",
            Some(Payload::new(b"12".as_slice(), "application/json")),
            Deadline::NONE,
            now(),
            &mut work(),
        )
        .ok()
        .unwrap();
    p.close_blocked.store(true, Ordering::Relaxed);
    drop(runtime);
    h.progress(&mut cx(), now(), &mut work()).unwrap();
    assert!(h.take_settlement(now(), &mut work()).is_err());
    assert_eq!(p.live.load(Ordering::Relaxed), 1);
    drop(h);
    drop(owner);
    assert_eq!(p.live.load(Ordering::Relaxed), 0);
    assert_eq!(p.drops_funded.load(Ordering::Relaxed), 1);
    for port in p.ports.lock().unwrap().iter() {
        drop(UdpSocket::bind(port).unwrap());
    }
    let mut bytes = [0; 32];
    assert_eq!(
        peer.recv(&mut bytes).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    clean(&b);
}
