use super::*;
use clinkz_wot_core::{consumer::*, notification::*, *};
use std::{
    cell::RefCell,
    collections::VecDeque,
    convert::Infallible,
    future::Future,
    io,
    pin::Pin,
    rc::Weak,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
use tokio::{net::TcpStream, time::Sleep};

const LEASES: usize = 32;
const QUEUE: usize = 4;
const PACKET: usize = MAX_EVENT_BYTES + MAX_TOPIC_BYTES + 7;
static CLIENT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqttConsumerConfig {
    pub(crate) address: SocketAddr,
    write_bytes: usize,
}
impl MqttConsumerConfig {
    pub fn for_loopback(address: SocketAddr, write_bytes: usize) -> CoreResult<Self> {
        MqttProducerConfig::for_loopback(address, write_bytes)?;
        Ok(Self {
            address,
            write_bytes,
        })
    }
}
struct ConsumerRegistration {
    config: MqttConsumerConfig,
    identity: BindingRegistrationIdentity,
}
/// Actual startup registration and a retained reference to application-owned
/// infrastructure. Clones grant neither logical permission nor shutdown power.
#[derive(Clone)]
pub struct MqttEventConsumer {
    registration: Rc<ConsumerRegistration>,
    session: Rc<RefCell<ConsumerSession>>,
}
/// Application-owned physical service. Last real participation disposes the
/// socket even while this owner/registrations/definitions remain retained.
pub struct MqttConsumerService {
    binding: MqttEventConsumer,
}
#[derive(Clone)]
pub struct MqttConsumerProgram {
    registration: Rc<ConsumerRegistration>,
    pub(crate) topic: Arc<str>,
    cancellation: bool,
}
impl MqttConsumerService {
    pub fn new(
        config: MqttConsumerConfig,
        id: BindingId,
        generation: BindingGeneration,
    ) -> CoreResult<Self> {
        let mut digest = [0; 32];
        match config.address.ip() {
            std::net::IpAddr::V4(ip) => {
                digest[0] = 4;
                digest[1..5].copy_from_slice(&ip.octets());
            }
            std::net::IpAddr::V6(ip) => {
                digest[0] = 6;
                digest[1..17].copy_from_slice(&ip.octets());
            }
        }
        digest[17..19].copy_from_slice(&config.address.port().to_be_bytes());
        digest[19..27].copy_from_slice(&(config.write_bytes as u64).to_be_bytes());
        let identity = BindingRegistrationIdentity::new(
            id,
            generation,
            BindingConfigurationDigest::new(digest),
            BindingArtifactCompatibility::new(*b"mqtt-event-rx-01"),
            0,
        );
        let registration = Rc::new(ConsumerRegistration { config, identity });
        let client = CLIENT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| unsupported())?;
        let client = format!("cr{:05x}{client:016x}", std::process::id() & 0xfffff);
        let mut connect = vec![
            0x10,
            (12 + client.len()) as u8,
            0,
            4,
            b'M',
            b'Q',
            b'T',
            b'T',
            4,
            2,
            0,
            0,
            0,
            client.len() as u8,
        ];
        connect.extend_from_slice(client.as_bytes());
        let session = Rc::new(RefCell::new(ConsumerSession {
            registration: registration.clone(),
            connect_frame: connect,
            phase: ConnectionPhase::New,
            connecting: None,
            stream: None,
            written: 0,
            timer: None,
            failed: None,
            interests: Vec::with_capacity(LEASES),
            topics: Vec::with_capacity(LEASES),
            control: None,
            next_packet: 1,
            rx: [0; PACKET],
            received: 0,
            expected: None,
            wakes: Arc::new(NativeWakes(Mutex::new(std::array::from_fn(|_| None)))),
        }));
        Ok(Self {
            binding: MqttEventConsumer {
                registration,
                session,
            },
        })
    }
    pub fn binding(&self) -> MqttEventConsumer {
        self.binding.clone()
    }
    pub fn connected(&self) -> bool {
        self.binding.session.borrow().stream.is_some()
    }
    pub fn first_failure(&self) -> Option<CoreError> {
        self.binding.session.borrow().failed.clone()
    }
    /// Native progress may fill native queues; it never dispatches applications.
    pub fn step(&mut self, cx: &mut Context<'_>) {
        self.binding.session.borrow_mut().step(cx, LEASES);
    }
    pub fn shutdown(&mut self) {
        self.binding.session.borrow_mut().fail(io_error());
    }
}

// One aggregate socket/timer waker preserves interest from every participant.
// Wakers run after the bounded lock is released and never call application data.
struct NativeWakes(Mutex<[Option<Waker>; LEASES + 1]>);
impl Wake for NativeWakes {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let wakes = self.0.lock().unwrap().clone();
        for w in wakes.into_iter().flatten() {
            w.wake();
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum ConnectionPhase {
    New,
    Connecting,
    Connect,
    Connack,
    Ready,
    Failed,
}
#[derive(Clone, Copy, PartialEq)]
enum TopicPhase {
    New,
    Subscribing,
    Active,
    Unsubscribing,
}
struct Topic {
    name: Arc<str>,
    phase: TopicPhase,
}
struct Control {
    topic: Arc<str>,
    subscribe: bool,
    packet: u16,
    frame: Vec<u8>,
    written: usize,
    timer: Pin<Box<Sleep>>,
}
struct Interest {
    recipient: NotificationIdentity,
    topic: Arc<str>,
    slot: usize,
    started: bool,
    closing: bool,
    authorized_cancel: bool,
    remote: RemoteCancellation,
    queue: VecDeque<Payload>,
    loss: u64,
}
struct ConsumerSession {
    registration: Rc<ConsumerRegistration>,
    connect_frame: Vec<u8>,
    phase: ConnectionPhase,
    connecting: Option<Pin<Box<dyn Future<Output = io::Result<TcpStream>>>>>,
    stream: Option<TcpStream>,
    written: usize,
    timer: Option<Pin<Box<Sleep>>>,
    failed: Option<CoreError>,
    interests: Vec<Weak<RefCell<Interest>>>,
    topics: Vec<Topic>,
    control: Option<Control>,
    next_packet: u16,
    rx: [u8; PACKET],
    received: usize,
    expected: Option<usize>,
    wakes: Arc<NativeWakes>,
}
impl ConsumerSession {
    fn fail(&mut self, error: CoreError) {
        self.failed.get_or_insert(error);
        self.stream = None;
        self.connecting = None;
        self.timer = None;
        self.control = None;
        self.phase = ConnectionPhase::Failed;
        self.wakes.wake_by_ref();
    }
    fn live(&self, topic: &str) -> bool {
        self.interests.iter().filter_map(Weak::upgrade).any(|i| {
            let i = i.borrow();
            i.started && !i.closing && i.recipient.consumer.is_current() && &*i.topic == topic
        })
    }
    fn step(&mut self, caller: &mut Context<'_>, slot: usize) {
        self.wakes.0.lock().unwrap()[slot] = Some(caller.waker().clone());
        let waker = Waker::from(self.wakes.clone());
        let cx = &mut Context::from_waker(&waker);
        if self
            .timer
            .as_mut()
            .is_some_and(|t| t.as_mut().poll(cx).is_ready())
        {
            self.fail(CoreError::TimedOut(ErrorContext::new(
                ErrorPhase::Readiness,
                RetryClass::Never,
            )));
            return;
        }
        match self.phase {
            ConnectionPhase::New => {
                if !self.interests.iter().filter_map(Weak::upgrade).any(|i| {
                    let i = i.borrow();
                    i.started && !i.closing && i.recipient.consumer.is_current()
                }) {
                    return;
                }
                self.timer = Some(Box::pin(tokio::time::sleep(Duration::from_secs(2))));
                self.connecting = Some(Box::pin(TcpStream::connect(
                    self.registration.config.address,
                )));
                self.phase = ConnectionPhase::Connecting;
                self.wakes.wake_by_ref();
            }
            ConnectionPhase::Connecting => {
                match self.connecting.as_mut().unwrap().as_mut().poll(cx) {
                    Poll::Ready(Ok(stream)) => {
                        if stream.set_nodelay(true).is_err() {
                            self.fail(io_error());
                            return;
                        }
                        self.stream = Some(stream);
                        self.connecting = None;
                        self.phase = ConnectionPhase::Connect;
                        self.wakes.wake_by_ref();
                    }
                    Poll::Ready(Err(_)) => self.fail(io_error()),
                    Poll::Pending => {}
                }
            }
            ConnectionPhase::Connect => {
                let end = (self.written + self.registration.config.write_bytes)
                    .min(self.connect_frame.len());
                match native::write(
                    self.stream.as_ref().unwrap(),
                    cx,
                    &self.connect_frame[self.written..end],
                ) {
                    Poll::Ready(Ok(n)) => {
                        self.written += n;
                        if self.written == self.connect_frame.len() {
                            self.phase = ConnectionPhase::Connack;
                        }
                        self.wakes.wake_by_ref();
                    }
                    Poll::Ready(Err(_)) => self.fail(io_error()),
                    Poll::Pending => {}
                }
            }
            ConnectionPhase::Connack => {
                match native::read(
                    self.stream.as_ref().unwrap(),
                    cx,
                    &mut self.rx[self.received..4],
                ) {
                    Poll::Ready(Ok(n)) => {
                        self.received += n;
                        if self.received == 4 {
                            if self.rx[..4] != [0x20, 2, 0, 0] {
                                self.fail(io_error());
                                return;
                            }
                            self.received = 0;
                            self.timer = None;
                            self.phase = ConnectionPhase::Ready;
                        }
                        self.wakes.wake_by_ref();
                    }
                    Poll::Ready(Err(_)) => self.fail(io_error()),
                    Poll::Pending => {}
                }
            }
            ConnectionPhase::Ready => {
                self.control_step(cx);
                if self.phase == ConnectionPhase::Ready {
                    self.read_step(cx);
                }
            }
            ConnectionPhase::Failed => {}
        }
    }
    fn control_step(&mut self, cx: &mut Context<'_>) {
        if self.control.is_none() {
            let next = self
                .topics
                .iter()
                .find_map(|t| match (t.phase, self.live(&t.name)) {
                    (TopicPhase::New, true) => Some((t.name.clone(), true)),
                    (TopicPhase::Active, false) => Some((t.name.clone(), false)),
                    _ => None,
                });
            if let Some((topic, subscribe)) = next {
                // At most one original control in flight; ids are not reused
                // before its ACK. A service never reconnects after failure.
                let packet = self.next_packet;
                self.next_packet = self.next_packet.checked_add(1).unwrap_or(1);
                let mut body = packet.to_be_bytes().to_vec();
                body.extend_from_slice(&(topic.len() as u16).to_be_bytes());
                body.extend_from_slice(topic.as_bytes());
                if subscribe {
                    body.push(0);
                }
                let mut frame = vec![if subscribe { 0x82 } else { 0xa2 }];
                remaining(&mut frame, body.len());
                frame.extend_from_slice(&body);
                self.topics
                    .iter_mut()
                    .find(|t| t.name == topic)
                    .unwrap()
                    .phase = if subscribe {
                    TopicPhase::Subscribing
                } else {
                    TopicPhase::Unsubscribing
                };
                if !subscribe {
                    for i in self.interests.iter().filter_map(Weak::upgrade) {
                        let mut i = i.borrow_mut();
                        if i.topic == topic && i.closing && i.authorized_cancel {
                            i.remote = RemoteCancellation::Unknown;
                        }
                    }
                }
                self.control = Some(Control {
                    topic,
                    subscribe,
                    packet,
                    frame,
                    written: 0,
                    timer: Box::pin(tokio::time::sleep(Duration::from_secs(2))),
                });
            }
        }
        let Some(control) = &mut self.control else {
            return;
        };
        if control.timer.as_mut().poll(cx).is_ready() {
            self.fail(io_error());
            return;
        }
        if control.written == control.frame.len() {
            return;
        }
        let end = (control.written + self.registration.config.write_bytes).min(control.frame.len());
        match native::write(
            self.stream.as_ref().unwrap(),
            cx,
            &control.frame[control.written..end],
        ) {
            Poll::Ready(Ok(n)) => {
                control.written += n;
                self.wakes.wake_by_ref();
            }
            Poll::Ready(Err(_)) => self.fail(io_error()),
            Poll::Pending => {}
        }
    }
    fn read_step(&mut self, cx: &mut Context<'_>) {
        // Read only the current frame; at most one complete packet per step.
        let end = self.expected.unwrap_or(self.received + 1);
        match native::read(
            self.stream.as_ref().unwrap(),
            cx,
            &mut self.rx[self.received..end],
        ) {
            Poll::Ready(Ok(n)) => self.received += n,
            Poll::Ready(Err(_)) => {
                self.fail(io_error());
                return;
            }
            Poll::Pending => return,
        }
        if self.expected.is_none() && self.received > 1 {
            let mut length = 0usize;
            for (i, b) in self.rx[1..self.received].iter().enumerate() {
                length += usize::from(b & 127) << (7 * i);
                if i == 3 && b & 128 != 0 {
                    self.fail(io_error());
                    return;
                }
                if b & 128 == 0 {
                    let total = length + i + 2;
                    if total > PACKET || total <= self.received {
                        self.fail(io_error());
                        return;
                    }
                    self.expected = Some(total);
                    break;
                }
            }
        }
        if self.expected == Some(self.received) {
            self.packet();
            self.received = 0;
            self.expected = None;
        }
        self.wakes.wake_by_ref();
    }
    fn packet(&mut self) {
        let header = self.rx[0];
        let offset = 1
            + self.rx[1..self.received]
                .iter()
                .position(|b| b & 128 == 0)
                .unwrap()
            + 1;
        let body = &self.rx[offset..self.received];
        if matches!(header, 0x90 | 0xb0) {
            let Some(c) = self.control.as_ref() else {
                self.fail(io_error());
                return;
            };
            if body.len() != if c.subscribe { 3 } else { 2 }
                || body[..2] != c.packet.to_be_bytes()
                || c.written != c.frame.len()
                || header != if c.subscribe { 0x90 } else { 0xb0 }
                || c.subscribe && body[2] != 0
            {
                self.fail(io_error());
                return;
            }
            let topic = c.topic.clone();
            let t = self.topics.iter_mut().find(|t| t.name == topic).unwrap();
            t.phase = if c.subscribe {
                TopicPhase::Active
            } else {
                TopicPhase::New
            };
            if !c.subscribe {
                for i in self.interests.iter().filter_map(Weak::upgrade) {
                    let mut i = i.borrow_mut();
                    if i.topic == topic && i.remote == RemoteCancellation::Unknown {
                        i.remote = RemoteCancellation::Confirmed;
                    }
                }
            }
            self.control = None;
            return;
        }
        if !matches!(header, 0x30 | 0x31) || body.len() < 2 {
            self.fail(io_error());
            return;
        }
        let n = usize::from(u16::from_be_bytes([body[0], body[1]]));
        if n == 0
            || n > MAX_TOPIC_BYTES
            || n + 2 > body.len()
            || body.len() - n - 2 > MAX_EVENT_BYTES
        {
            self.fail(io_error());
            return;
        }
        let topic = &body[2..2 + n];
        let payload = Payload::new(body[2 + n..].to_vec(), "application/json");
        for interest in self.interests.iter().filter_map(Weak::upgrade) {
            let mut i = interest.borrow_mut();
            if !i.started
                || i.closing
                || !i.recipient.consumer.is_current()
                || i.topic.as_bytes() != topic
            {
                continue;
            }
            // Initial broker-retained data is never fabricated as a new Event.
            if header & 1 != 0 || i.queue.len() == QUEUE {
                i.loss = i.loss.saturating_add(1);
            } else {
                i.queue.push_back(payload.clone());
            }
        }
    }
    fn dispose(&mut self, interest: &Rc<RefCell<Interest>>) {
        let slot = interest.borrow().slot;
        self.wakes.0.lock().unwrap()[slot] = None;
        self.interests
            .retain(|w| w.upgrade().is_some_and(|p| !Rc::ptr_eq(&p, interest)));
        // Slots for unsubscribed topics can be reused, preserving an in-flight
        // control even when the original logical recipient has gone away.
        let interests = &self.interests;
        self.topics.retain(|t| {
            t.phase != TopicPhase::New
                || interests
                    .iter()
                    .filter_map(Weak::upgrade)
                    .any(|i| i.borrow().topic == t.name)
        });
        if self.interests.is_empty() {
            self.stream = None;
            self.connecting = None;
            self.control = None;
            self.timer = None;
            // A later explicit logical subscription may start a fresh clean
            // connection. Failed services stay terminal; no old work is retried.
            self.phase = if self.failed.is_some() {
                ConnectionPhase::Failed
            } else {
                ConnectionPhase::New
            };
            self.topics.clear();
            self.written = 0;
            self.received = 0;
            self.expected = None;
            *self.wakes.0.lock().unwrap() = std::array::from_fn(|_| None);
        }
        self.wakes.wake_by_ref();
    }
}
fn remaining(frame: &mut Vec<u8>, mut n: usize) {
    loop {
        let b = (n % 128) as u8;
        n /= 128;
        frame.push(b | if n != 0 { 128 } else { 0 });
        if n == 0 {
            break;
        }
    }
}

pub struct MqttSubscription {
    session: Rc<RefCell<ConsumerSession>>,
    interest: Rc<RefCell<Interest>>,
    disposed: bool,
}
impl ConsumerNotificationBinding for MqttEventConsumer {
    type Program = MqttConsumerProgram;
    type Configuration = MqttConsumerConfig;
    type Participation = MqttSubscription;
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (
            self.registration.identity.binding_id(),
            self.registration.identity.binding_generation(),
        )
    }
    fn configuration(&self) -> &MqttConsumerConfig {
        &self.registration.config
    }
    fn support(
        &self,
        p: &MqttConsumerProgram,
        kind: NotificationKind,
    ) -> CoreResult<NotificationSupport> {
        if kind != NotificationKind::Event || !Rc::ptr_eq(&p.registration, &self.registration) {
            return Err(unsupported());
        }
        Ok(NotificationSupport {
            event_responses: false,
            cancellation: p.cancellation,
        })
    }
    fn prepare(
        &mut self,
        program: MqttConsumerProgram,
        recipient: &NotificationIdentity,
    ) -> Result<MqttSubscription, BindingInputRejection<MqttConsumerProgram>> {
        let build = (|| {
            self.support(&program, NotificationKind::Event)?;
            let mut s = self.session.borrow_mut();
            if s.phase == ConnectionPhase::Failed {
                return Err(io_error());
            }
            if s.interests.len() == LEASES {
                return Err(busy());
            }
            let interests = &s.interests.clone();
            s.topics.retain(|t| {
                t.phase != TopicPhase::New
                    || interests
                        .iter()
                        .filter_map(Weak::upgrade)
                        .any(|i| i.borrow().topic == t.name)
            });
            let slot = (0..LEASES)
                .find(|n| {
                    !s.interests
                        .iter()
                        .filter_map(Weak::upgrade)
                        .any(|i| i.borrow().slot == *n)
                })
                .unwrap();
            if !s.topics.iter().any(|t| t.name == program.topic) {
                if s.topics.len() == LEASES {
                    return Err(busy());
                }
                s.topics.push(Topic {
                    name: program.topic.clone(),
                    phase: TopicPhase::New,
                });
            }
            let interest = Rc::new(RefCell::new(Interest {
                recipient: recipient.clone(),
                topic: program.topic.clone(),
                slot,
                started: false,
                closing: false,
                authorized_cancel: false,
                remote: RemoteCancellation::NotAttempted,
                queue: VecDeque::with_capacity(QUEUE),
                loss: 0,
            }));
            s.interests.push(Rc::downgrade(&interest));
            Ok(MqttSubscription {
                session: self.session.clone(),
                interest,
                disposed: false,
            })
        })();
        build.map_err(|e| BindingInputRejection::new(program, BindingOperationalError::new(e)))
    }
}
impl NotificationParticipation for MqttSubscription {
    type Response = Infallible;
    fn poll_start(
        &mut self,
        cx: &mut Context<'_>,
        input: &SubscriptionInput,
    ) -> Poll<CoreResult<()>> {
        if input.0.is_some() {
            return Poll::Ready(Err(unsupported()));
        }
        let slot = {
            let mut i = self.interest.borrow_mut();
            i.started = true;
            i.slot
        };
        let mut s = self.session.borrow_mut();
        s.step(cx, slot);
        if let Some(e) = &s.failed {
            return Poll::Ready(Err(e.clone()));
        }
        if s.topics
            .iter()
            .any(|t| t.name == self.interest.borrow().topic && t.phase == TopicPhase::Active)
        {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        max: usize,
    ) -> Poll<CoreResult<NotificationSignal<Infallible>>> {
        let slot = self.interest.borrow().slot;
        let mut s = self.session.borrow_mut();
        s.step(cx, slot);
        if let Some(e) = &s.failed {
            return Poll::Ready(Err(e.clone()));
        }
        let mut i = self.interest.borrow_mut();
        if i.loss != 0 {
            let n = std::mem::take(&mut i.loss);
            return Poll::Ready(Ok(NotificationSignal::Loss(n)));
        }
        let Some(p) = i.queue.front() else {
            return Poll::Pending;
        };
        if notification_payload_backing(Some(p)).is_none_or(|n| n > max) {
            return Poll::Ready(Err(busy()));
        }
        let data = i.queue.pop_front();
        Poll::Ready(Ok(NotificationSignal::Data(NativeNotification {
            recipient: i.recipient.clone(),
            data,
            response: None,
        })))
    }
    fn poll_close(
        &mut self,
        cx: &mut Context<'_>,
        input: Option<&CancellationInput>,
    ) -> Poll<CoreResult<RemoteCancellation>> {
        if input.is_some_and(|i| i.0.is_some()) {
            return Poll::Ready(Err(unsupported()));
        }
        let (slot, topic) = {
            let mut i = self.interest.borrow_mut();
            i.authorized_cancel |= input.is_some();
            i.closing = true;
            i.queue.clear();
            (i.slot, i.topic.clone())
        };
        let mut s = self.session.borrow_mut();
        // Shared native interests survive this logical stop. Only the final
        // matching lease sends UNSUBSCRIBE, without granting global shutdown.
        let shared = s.live(&topic);
        if !shared && s.phase == ConnectionPhase::Ready {
            s.step(cx, slot);
        }
        let pending = !shared
            && s.phase == ConnectionPhase::Ready
            && s.topics
                .iter()
                .any(|t| t.name == topic && t.phase != TopicPhase::New);
        if pending {
            return Poll::Pending;
        }
        let result = self.interest.borrow().remote;
        s.dispose(&self.interest);
        self.disposed = true;
        Poll::Ready(Ok(result))
    }
    fn abort_local(self) {
        drop(self);
    }
}
impl Drop for MqttSubscription {
    fn drop(&mut self) {
        if !self.disposed {
            self.interest.borrow_mut().closing = true;
            self.session.borrow_mut().dispose(&self.interest);
        }
    }
}

/// Actual concrete JSON/type decoder for Consumer selection and delivery.
pub struct MqttJsonData;
impl ConsumerDataService for MqttJsonData {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn validate(&self, schema: TypeSchema, payload: &Payload) -> CoreResult<()> {
        clinkz_wot_codec_json::validate_payload(schema, payload)
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 0,
            temporary_bytes: 4096,
            body_bytes: MAX_EVENT_BYTES as u64,
            validation_work: 65536,
        }
    }
}

#[cfg(feature = "prepare")]
pub(crate) fn consumer_identity(b: &MqttEventConsumer) -> BindingRegistrationIdentity {
    b.registration.identity
}
#[cfg(feature = "prepare")]
pub(crate) fn consumer_program(
    b: &MqttEventConsumer,
    topic: Arc<str>,
    cancellation: bool,
) -> MqttConsumerProgram {
    MqttConsumerProgram {
        registration: b.registration.clone(),
        topic,
        cancellation,
    }
}
