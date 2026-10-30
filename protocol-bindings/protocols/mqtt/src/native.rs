use super::*;
use clinkz_wot_core::{
    BindingInputRejection, BindingOperationalError, CoreResult, event::*,
    producer::ProducerInstanceKey, thing::AffordanceSite,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_servient_finite::{
    ClosedProducerParticipation, ProducerEventEmission, ProducerOwnerToken,
    ProducerParticipationId, RawProducerServient,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::Future,
    io,
    mem::size_of,
    pin::Pin,
    rc::Weak,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::net::TcpStream;

const MAX_LEASES: usize = 32;
const MAX_PACKET: usize = MAX_EVENT_BYTES + MAX_TOPIC_BYTES + 7;
const DATA_BACKING: usize = MAX_EVENT_BYTES + 256;
static NEXT_CLIENT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    New,
    Connecting,
    SendingConnect,
    Connack,
    Ready,
    Failed,
    Closed,
}
type Connect = Pin<Box<dyn Future<Output = io::Result<TcpStream>>>>;
struct Session {
    registration: Rc<Registration>,
    phase: Phase,
    connect: Option<Connect>,
    stream: Option<TcpStream>,
    handshake: [u8; 64],
    handshake_len: usize,
    written: usize,
    connack: [u8; 4],
    received: usize,
    started: Option<Instant>,
    first_cause: Option<CoreError>,
    leases: Vec<Weak<Lease>>,
    busy: bool,
    locally_published: u64,
}
impl Session {
    fn fail(&mut self, cause: CoreError) {
        if self.first_cause.is_none() {
            self.first_cause = Some(cause);
        }
        self.connect = None;
        self.stream = None;
        self.phase = Phase::Failed;
    }
    fn step(&mut self, cx: &mut Context<'_>, budget: &mut WorkBudget) -> Poll<()> {
        if matches!(self.phase, Phase::Failed | Phase::Closed) {
            return Poll::Ready(());
        }
        if self.phase != Phase::Ready
            && self
                .started
                .is_some_and(|t| t.elapsed() >= Duration::from_secs(2))
        {
            self.fail(CoreError::TimedOut(ErrorContext::new(
                ErrorPhase::Readiness,
                RetryClass::Never,
            )));
            return Poll::Ready(());
        }
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        match self.phase {
            Phase::New => {
                self.started = Some(Instant::now());
                self.connect = Some(Box::pin(TcpStream::connect(
                    self.registration.config.address,
                )));
                self.phase = Phase::Connecting;
            }
            Phase::Connecting => match self.connect.as_mut().expect("connecting").as_mut().poll(cx)
            {
                Poll::Ready(Ok(stream)) => {
                    if stream.set_nodelay(true).is_err() {
                        self.fail(io_error());
                        return Poll::Ready(());
                    }
                    self.stream = Some(stream);
                    self.connect = None;
                    self.phase = Phase::SendingConnect;
                }
                Poll::Ready(Err(_)) => self.fail(io_error()),
                Poll::Pending => {}
            },
            Phase::SendingConnect => {
                let end = (self.written + self.registration.config.write_bytes_per_step)
                    .min(self.handshake_len);
                match write(
                    self.stream.as_ref().expect("connected"),
                    cx,
                    &self.handshake[self.written..end],
                ) {
                    Poll::Ready(Ok(n)) => {
                        self.written += n;
                        if self.written == self.handshake_len {
                            self.phase = Phase::Connack;
                        }
                    }
                    Poll::Ready(Err(_)) => self.fail(io_error()),
                    Poll::Pending => {}
                }
            }
            Phase::Connack => match read(
                self.stream.as_ref().expect("connected"),
                cx,
                &mut self.connack[self.received..],
            ) {
                Poll::Ready(Ok(n)) => {
                    self.received += n;
                    if self.received == 4 {
                        // Clean session, protocol 3.1.1; no session-present or
                        // other server response can be silently accepted.
                        if self.connack == [0x20, 2, 0, 0] {
                            self.phase = Phase::Ready;
                        } else {
                            self.fail(io_error());
                        }
                    }
                }
                Poll::Ready(Err(_)) => self.fail(io_error()),
                Poll::Pending => {}
            },
            Phase::Ready => {
                // This publisher never subscribes or requests MQTT ACKs. Any
                // incoming packet, EOF or I/O error fences the native service.
                let mut unexpected = [0; 1];
                if read(
                    self.stream.as_ref().expect("connected"),
                    cx,
                    &mut unexpected,
                )
                .is_ready()
                {
                    self.fail(io_error());
                }
            }
            Phase::Failed | Phase::Closed => unreachable!(),
        }
        if matches!(self.phase, Phase::Ready | Phase::Failed) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
fn write(stream: &TcpStream, cx: &mut Context<'_>, data: &[u8]) -> Poll<io::Result<usize>> {
    match stream.poll_write_ready(cx) {
        Poll::Pending => Poll::Pending,
        Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        Poll::Ready(Ok(())) => match stream.try_write(data) {
            Ok(0) => Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            r => Poll::Ready(r),
        },
    }
}
fn read(stream: &TcpStream, cx: &mut Context<'_>, data: &mut [u8]) -> Poll<io::Result<usize>> {
    match stream.poll_read_ready(cx) {
        Poll::Pending => Poll::Pending,
        Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        Poll::Ready(Ok(())) => match stream.try_read(data) {
            Ok(0) => Poll::Ready(Err(io::ErrorKind::UnexpectedEof.into())),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            r => Poll::Ready(r),
        },
    }
}

/// Unique application-owned native service. Participations may share its one
/// connection across Things/Servients; polling a lease grants no shutdown right.
/// Last actual lease/work disposal closes the socket. No idle pool or reconnect.
pub struct MqttEventService {
    session: Rc<RefCell<Session>>,
}
impl MqttEventService {
    pub fn new(producer: &MqttEventProducer) -> CoreResult<Self> {
        let id = NEXT_CLIENT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| unsupported())?;
        // 23 ASCII characters, valid for MQTT 3.1.1's minimum client-id scope.
        let client = format!("cz{:05x}{id:016x}", std::process::id() & 0xfffff);
        let mut handshake = [0; 64];
        let len = 14 + client.len();
        handshake[..14].copy_from_slice(&[
            0x10,
            (len - 2) as u8,
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
        ]);
        handshake[14..len].copy_from_slice(client.as_bytes());
        Ok(Self {
            session: Rc::new(RefCell::new(Session {
                registration: Rc::clone(&producer.registration),
                phase: Phase::New,
                connect: None,
                stream: None,
                handshake,
                handshake_len: len,
                written: 0,
                connack: [0; 4],
                received: 0,
                started: None,
                first_cause: None,
                leases: Vec::with_capacity(MAX_LEASES),
                busy: false,
                locally_published: 0,
            })),
        })
    }
    /// Explicit whole-service shutdown belongs only to this owner. Dependents
    /// observe failure on progress; no uncertain publication is retried.
    pub fn shutdown(&mut self) {
        self.session.borrow_mut().fail(io_error());
    }
    pub fn connected(&self) -> bool {
        self.session.borrow().stream.is_some()
    }
    pub fn locally_published(&self) -> u64 {
        self.session.borrow().locally_published
    }
    pub fn first_failure(&self) -> Option<CoreError> {
        self.session.borrow().first_cause.clone()
    }

    pub fn participation(
        &self,
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
        definitions: Vec<MqttEventDefinition>,
    ) -> CoreResult<MqttEventRuntime> {
        if definitions.is_empty() || definitions.len() > 32 {
            return Err(unsupported());
        }
        let mut session = self.session.borrow_mut();
        session.leases.retain(|w| w.strong_count() != 0);
        if session.leases.len() == MAX_LEASES
            || matches!(session.phase, Phase::Failed | Phase::Closed)
            || definitions
                .iter()
                .any(|d| !Rc::ptr_eq(&d.registration, &session.registration))
        {
            return Err(unsupported());
        }
        let mut events: Vec<EventMapping> = Vec::new();
        for d in &definitions {
            if d.operation != Operation::SubscribeEvent {
                continue;
            }
            if events.len() == 16
                || events
                    .iter()
                    .any(|e| e.event == d.event || e.topic == d.topic)
            {
                return Err(unsupported());
            }
            events.push(EventMapping {
                event: Arc::clone(&d.event),
                topic: Arc::clone(&d.topic),
                route: EventRoute::new(session.registration.binding),
                services: Rc::new(JsonNoSec { schema: d.schema }),
            });
        }
        if events.is_empty()
            || definitions.iter().any(|d| {
                !events.iter().any(|e| {
                    e.event == d.event && e.topic == d.topic && e.services.schema == d.schema
                })
            })
            || definitions.iter().enumerate().any(|(i, d)| {
                definitions[..i]
                    .iter()
                    .any(|p| p.event == d.event && p.operation == d.operation)
            })
        {
            return Err(unsupported());
        }
        // Refuse ambiguous attribution between independent logical sources.
        // A staged successor may reuse its old numeric slot/topic, but cannot
        // inherit its original mapping identity or pending publication.
        if session.leases.iter().filter_map(Weak::upgrade).any(|l| {
            if l.closing.get() {
                return false;
            }
            if l.token.belongs_to(owner) && l.key.slot().slot() == key.slot().slot() {
                return false;
            }
            l.events
                .iter()
                .any(|a| events.iter().any(|b| a.topic == b.topic))
        }) {
            return Err(unsupported());
        }
        let gate = Rc::new(Cell::new(false));
        let lease = Rc::new(Lease {
            session: Rc::clone(&self.session),
            token: owner.participation_token(),
            key,
            events,
            definitions,
            closing: Cell::new(false),
            gate: Rc::clone(&gate),
            funding: RefCell::new(None),
        });
        session.leases.push(Rc::downgrade(&lease));
        Ok(MqttEventRuntime {
            lease: Some(lease),
            token: owner.participation_token(),
            key,
            gate,
            id: ProducerParticipationId::new(),
            started: false,
            closed: false,
            first_cause: None,
        })
    }
}

struct JsonNoSec {
    schema: TypeSchema,
}
impl ProducerEventServices for JsonNoSec {
    fn authorize(&self, _data: &EventData, budget: &mut WorkBudget) -> CoreResult<()> {
        budget
            .consume(WorkClass::SecurityBranches, 1)
            .map_err(|_| busy())
    }
    fn validate(&self, data: &EventData, budget: &mut WorkBudget) -> CoreResult<()> {
        budget
            .consume(WorkClass::CodecInputBytes, data.0.body.len() as u64)
            .map_err(|_| busy())?;
        clinkz_wot_codec_json::validate_payload(self.schema, &data.0)
    }
}
struct EventMapping {
    event: Arc<str>,
    topic: Arc<str>,
    route: EventRoute,
    services: Rc<JsonNoSec>,
}
struct Lease {
    session: Rc<RefCell<Session>>,
    token: ProducerOwnerToken,
    key: ProducerInstanceKey,
    events: Vec<EventMapping>,
    definitions: Vec<MqttEventDefinition>,
    closing: Cell<bool>,
    gate: Rc<Cell<bool>>,
    // Last disposal includes native publications which outlive the exposure.
    funding: RefCell<Option<Rc<dyn Any>>>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        let mut s = self.session.borrow_mut();
        if !s.leases.iter().any(|w| w.strong_count() != 0) {
            s.connect = None;
            s.stream = None;
            if s.phase != Phase::New && s.phase != Phase::Failed {
                s.phase = Phase::Closed;
            }
        }
    }
}

pub struct MqttEventRuntime {
    lease: Option<Rc<Lease>>,
    token: ProducerOwnerToken,
    key: ProducerInstanceKey,
    gate: Rc<Cell<bool>>,
    id: ProducerParticipationId,
    started: bool,
    closed: bool,
    first_cause: Option<CoreError>,
}
impl MqttEventRuntime {
    /// Drive native preparation/health and local lease retirement. This checks
    /// the actual logical issuer; native failure fences this Thing's authority.
    pub fn step(
        &mut self,
        owner: &mut RawProducerServient,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<()>> {
        if !self.token.belongs_to(owner) {
            return Err(unsupported());
        }
        let Some(lease) = &self.lease else {
            return Ok(Poll::Ready(()));
        };
        if self.closed {
            if Rc::strong_count(lease) == 1 {
                if budget.consume(WorkClass::CleanupItems, 1).is_err() {
                    return Ok(Poll::Pending);
                }
                self.lease = None;
                return Ok(Poll::Ready(()));
            }
            return Ok(Poll::Pending);
        }
        self.started = true;
        let result = lease.session.borrow_mut().step(cx, budget);
        let failure = lease.session.borrow().first_cause.clone();
        if let Some(cause) = failure {
            if self.first_cause.is_none() {
                self.first_cause = Some(cause);
            }
            owner.revoke(self.key);
            self.begin_retire();
        } else if self.gate.get() && owner.description(self.key).is_none() {
            self.begin_retire();
        }
        Ok(result)
    }
    pub fn first_failure(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    fn covers_definition(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> bool {
        let Some(l) = &self.lease else {
            return false;
        };
        let Some(d) = definition.downcast_ref::<MqttEventDefinition>() else {
            return false;
        };
        self.key == key
            && binding == d.registration.binding
            && site == AffordanceSite::Event(&d.event)
            && form == d.form
            && operation == d.operation
            && l.definitions.contains(d)
    }
}
impl ClosedProducerParticipation for MqttEventRuntime {
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.id
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        if let Some(l) = &self.lease {
            let old = l.funding.borrow_mut().take();
            *l.funding.borrow_mut() = Some(Rc::new((old, owner)));
        }
    }
    fn admission_backing_bytes(
        &self,
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        let l = self.lease.as_ref()?;
        if self.started
            || self.closed
            || !self.token.belongs_to(owner)
            || key != self.key
            || l.funding.borrow().is_some()
        {
            return None;
        }
        let mut bytes = size_of::<Self>()
            + size_of::<Lease>()
            + size_of::<ProducerEventEmission<MqttPublication>>()
            + DATA_BACKING
            + 32 * size_of::<usize>()
            + l.events.capacity() * size_of::<EventMapping>()
            + l.definitions.capacity() * size_of::<MqttEventDefinition>();
        for d in &l.definitions {
            bytes = bytes.checked_add(d.event.len() + d.topic.len() + 4 * size_of::<usize>())?;
        }
        for e in &l.events {
            bytes = bytes.checked_add(
                e.event.len() + e.topic.len() + size_of::<JsonNoSec>() + 10 * size_of::<usize>(),
            )?;
        }
        Some(bytes)
    }
    fn begin_retire(&mut self) {
        self.closed = true;
        self.gate.set(false);
        if let Some(l) = &self.lease {
            l.closing.set(true);
        }
    }
    fn ready_for(&self, owner: &RawProducerServient, key: ProducerInstanceKey) -> bool {
        self.token.belongs_to(owner)
            && self.key == key
            && self.started
            && !self.closed
            && self
                .lease
                .as_ref()
                .is_some_and(|l| l.session.borrow().phase == Phase::Ready)
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> bool {
        self.covers_definition(key, binding, site, form, operation, definition)
    }
    fn event_publication(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: AffordanceSite<'_>,
        form: u32,
        definition: &dyn Any,
    ) -> Option<(EventRoute, Rc<dyn ProducerEventServices>, usize)> {
        if !self.covers_definition(
            key,
            binding,
            site,
            form,
            Operation::SubscribeEvent,
            definition,
        ) {
            return None;
        }
        let AffordanceSite::Event(name) = site else {
            return None;
        };
        let e = self
            .lease
            .as_ref()?
            .events
            .iter()
            .find(|e| e.event.as_ref() == name)?;
        Some((e.route.clone(), e.services.clone(), DATA_BACKING))
    }
    fn ingress_gate(&self) -> &Rc<Cell<bool>> {
        &self.gate
    }
    fn retired(&self) -> bool {
        self.closed && self.lease.is_none()
    }
}
impl Drop for MqttEventRuntime {
    fn drop(&mut self) {
        self.begin_retire();
    }
}
impl ProducerEventBinding for MqttEventRuntime {
    type Publication = MqttPublication;
    fn event_route(&self, event: &str) -> Option<&EventRoute> {
        self.lease
            .as_ref()?
            .events
            .iter()
            .find(|e| e.event.as_ref() == event)
            .map(|e| &e.route)
    }
    fn start_event(
        &mut self,
        route: &EventRoute,
        data: EventData,
    ) -> Result<MqttPublication, BindingInputRejection<EventData>> {
        let Some(l) = self
            .lease
            .as_ref()
            .filter(|l| !l.closing.get() && l.gate.get())
        else {
            return Err(reject(data, unsupported()));
        };
        let Some(e) = l.events.iter().find(|e| e.route.same_mapping(route)) else {
            return Err(reject(data, unsupported()));
        };
        let mut s = l.session.borrow_mut();
        if s.phase != Phase::Ready {
            return Err(reject(data, s.first_cause.clone().unwrap_or_else(io_error)));
        }
        if s.busy {
            return Err(reject(data, busy()));
        }
        if data.0.body.len() > MAX_EVENT_BYTES {
            return Err(reject(data, busy()));
        }
        let mut frame = [0; MAX_PACKET];
        frame[0] = 0x30; // QoS 0, DUP 0, RETAIN 0; no MQTT ACK or dataResponse.
        let remaining = 2 + e.topic.len() + data.0.body.len();
        let mut n = remaining;
        let mut at = 1;
        loop {
            let mut byte = (n % 128) as u8;
            n /= 128;
            if n != 0 {
                byte |= 128;
            }
            frame[at] = byte;
            at += 1;
            if n == 0 {
                break;
            }
        }
        frame[at..at + 2].copy_from_slice(&(e.topic.len() as u16).to_be_bytes());
        at += 2;
        frame[at..at + e.topic.len()].copy_from_slice(e.topic.as_bytes());
        at += e.topic.len();
        frame[at..at + data.0.body.len()].copy_from_slice(&data.0.body);
        at += data.0.body.len();
        s.busy = true;
        Ok(MqttPublication {
            lease: Rc::clone(l),
            frame,
            len: at,
            written: 0,
            outcome: None,
            data,
        })
    }
}

/// Original native slot, frame and data remain owned through Engine settlement
/// acknowledgement. There is no per-subscriber replication or queued replay.
pub struct MqttPublication {
    frame: [u8; MAX_PACKET],
    len: usize,
    written: usize,
    outcome: Option<NativeEventOutcome>,
    data: EventData,
    lease: Rc<Lease>,
}
impl NativeEventPublication for MqttPublication {
    fn poll_publish(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        cancel: bool,
    ) -> Poll<NativeEventOutcome> {
        if let Some(outcome) = &self.outcome {
            return Poll::Ready(outcome.clone());
        }
        let mut s = self.lease.session.borrow_mut();
        if cancel || self.lease.closing.get() || !self.lease.gate.get() {
            if self.written != 0 {
                s.fail(io_error());
            }
            self.outcome = Some(NativeEventOutcome {
                effect: if self.written == 0 {
                    EventPublishEffect::NotPublished
                } else {
                    EventPublishEffect::Uncertain
                },
                cause: s.first_cause.clone(),
            });
        } else {
            let _ = s.step(cx, budget);
            if s.phase == Phase::Failed || s.phase == Phase::Closed {
                self.outcome = Some(NativeEventOutcome {
                    effect: if self.written == 0 {
                        EventPublishEffect::NotPublished
                    } else {
                        EventPublishEffect::Uncertain
                    },
                    cause: Some(s.first_cause.clone().unwrap_or_else(io_error)),
                });
            } else {
                if budget.consume(WorkClass::BindingPolls, 1).is_err() {
                    return Poll::Pending;
                }
                let end = (self.written + s.registration.config.write_bytes_per_step).min(self.len);
                match write(
                    s.stream.as_ref().expect("ready"),
                    cx,
                    &self.frame[self.written..end],
                ) {
                    Poll::Ready(Ok(n)) => {
                        self.written += n;
                        if self.written == self.len {
                            s.locally_published = s.locally_published.saturating_add(1);
                            self.outcome = Some(NativeEventOutcome {
                                effect: EventPublishEffect::LocallyPublished,
                                cause: None,
                            });
                        }
                    }
                    Poll::Ready(Err(_)) => {
                        s.fail(io_error());
                        self.outcome = Some(NativeEventOutcome {
                            effect: if self.written == 0 {
                                EventPublishEffect::NotPublished
                            } else {
                                EventPublishEffect::Uncertain
                            },
                            cause: s.first_cause.clone(),
                        });
                    }
                    Poll::Pending => {}
                }
            }
        }
        self.outcome.clone().map_or(Poll::Pending, Poll::Ready)
    }
    fn poll_retire(
        &mut self,
        _cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<()>> {
        if self.outcome.is_none() || budget.consume(WorkClass::CleanupItems, 1).is_err() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}
impl Drop for MqttPublication {
    fn drop(&mut self) {
        let mut s = self.lease.session.borrow_mut();
        // A partially written TCP packet cannot be followed by another frame.
        // Closing this shared service is a native framing failure, reported to
        // all dependents, rather than silently corrupting a sibling publication.
        if self.written > 0 && self.written < self.len {
            s.fail(io_error());
        }
        s.busy = false;
        let _ = &self.data; // actual input backing stays retained until this Drop
    }
}
fn busy() -> CoreError {
    CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
fn reject(data: EventData, cause: CoreError) -> BindingInputRejection<EventData> {
    BindingInputRejection::new(data, BindingOperationalError::new(cause))
}
