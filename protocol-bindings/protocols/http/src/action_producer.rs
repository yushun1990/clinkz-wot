//! Manually driven HTTP/1.1 Producer Action participation. A listener is one
//! possible native lease, not a logical Thing or a general Engine resource.
use crate::{producer::producer_unsupported, producer_validation};
use clinkz_wot_core::{
    AffordanceTarget, BindingGeneration, BindingId, CoreError, CoreResult, ErrorContext,
    ErrorPhase, InteractionInput, InteractionOutput, Operation, Payload, RetryClass,
    action::{ActionMappingKey, ActionRoute, ActionValidator, RawActionRequest},
    producer::ProducerInstanceKey,
    thing::AffordanceSite,
    value_type::TypeSchema,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use clinkz_wot_servient_finite::{
    ClosedProducerParticipation, ProducerOwnerToken, ProducerParticipationId, RawActionHandle,
    RawActionSettlement, RawActionState, RawProducerServient,
};
use std::{
    any::Any,
    cell::Cell,
    hash::{BuildHasher, Hasher},
    io::{Read, Write},
    mem::size_of,
    net::{SocketAddr, TcpListener, TcpStream},
    rc::Rc,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    task::Poll,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpActionDefinition {
    pub(crate) instance_binding: (BindingId, BindingGeneration),
    pub(crate) action: Arc<str>,
    pub(crate) operation: Operation,
    pub(crate) form: u32,
    pub(crate) address: SocketAddr,
    pub(crate) path: Arc<str>,
    pub(crate) input_schema: Option<TypeSchema>,
    pub(crate) output_schema: Option<TypeSchema>,
    pub(crate) control: bool,
}
impl HttpActionDefinition {
    fn route(&self) -> ActionRoute {
        ActionRoute {
            binding: self.instance_binding,
            mapping: ActionMappingKey(u64::from(self.form) + 1),
        }
    }
}

struct Schemas {
    input: Option<TypeSchema>,
    output: Option<TypeSchema>,
}
impl ActionValidator for Schemas {
    fn validate_input(&self, input: &InteractionInput) -> CoreResult<()> {
        match (self.input, input.data.as_ref()) {
            (Some(schema), Some(data))
                if data.content_type == "application/json" && data.content_coding.is_none() =>
            {
                producer_validation::validate_json(schema, &data.body)
            }
            (None, None) => Ok(()),
            _ => Err(invalid()),
        }
    }
    fn validate_output(&self, output: &InteractionOutput) -> CoreResult<()> {
        match (self.output, output.data()) {
            (Some(schema), Some(_)) => producer_validation::validate(schema, output),
            (None, None) => Ok(()),
            _ => Err(invalid()),
        }
    }
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
}
fn io_error() -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_redacted_cause(640, "HTTP Action native I/O failed"),
    )
}

struct Record {
    id: u32,
    handle: RawActionHandle,
    finished: Option<Instant>,
}
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(0);
static PROCESS_SALT: OnceLock<u64> = OnceLock::new();
struct Exchange {
    stream: TcpStream,
    deadline: Instant,
    received: Vec<u8>,
    response: Vec<u8>,
    written: usize,
    invoked: Option<u32>,
}

/// A completed exchange's local write is separate from application effect and
/// does not prove that the remote peer received or processed the response.
#[derive(Debug)]
pub struct HttpActionDelivery {
    pub invocation: Option<u32>,
    pub locally_written: bool,
    pub write_started: bool,
    pub cause: Option<CoreError>,
}

/// One HTTP native participant for one Action and its *actually prepared*
/// Forms. Several such participants may compose in the same Thing; a Binding
/// can implement a shared listener as another participant implementation.
pub struct HttpActionRuntime {
    key: ProducerInstanceKey,
    // Native correlation epoch: a stale Location from another lease (including
    // replacement) cannot address this participant's fresh Action records.
    nonce: u64,
    token: ProducerOwnerToken,
    definitions: Vec<HttpActionDefinition>,
    validator: Rc<dyn ActionValidator>,
    max_records: usize,
    records: Vec<Record>,
    next_work: usize,
    listener: Option<TcpListener>,
    exchange: Option<Exchange>,
    delivery: Option<HttpActionDelivery>,
    first_cause: Option<CoreError>,
    closed: bool,
    participation: ProducerParticipationId,
    gate: Rc<Cell<bool>>,
    retained_owner: Option<Rc<dyn Any>>,
}
impl HttpActionRuntime {
    /// Pure construction. The definition set may be selected from a general
    /// PreparedThing; no TD/source loan or native socket is retained here.
    pub fn try_new(
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
        definitions: Vec<HttpActionDefinition>,
        max_records: usize,
    ) -> CoreResult<Self> {
        let Some(invoke) = definitions.first() else {
            return Err(producer_unsupported());
        };
        // This concrete participant has no cross-Binding correlation service.
        // Require the entire advertised HTTP control set here; a different
        // Binding's query/cancel cannot be silently routed to this handle store.
        if invoke.operation != Operation::InvokeAction
            || invoke.control
            || max_records == 0
            || max_records > 128
            || definitions.len() != 3
            || ![Operation::QueryAction, Operation::CancelAction]
                .iter()
                .all(|op| definitions.iter().any(|d| d.operation == *op))
            || definitions.iter().enumerate().any(|(i, d)| {
                d.address != invoke.address
                    || d.action != invoke.action
                    || d.instance_binding != invoke.instance_binding
                    || d.input_schema != invoke.input_schema
                    || d.output_schema != invoke.output_schema
                    || d.control != (d.operation != Operation::InvokeAction)
                    || definitions[..i].iter().any(|p| p.operation == d.operation)
            })
        {
            return Err(producer_unsupported());
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(max_records)
            .map_err(|_| io_error())?;
        let ordinal = NEXT_EPOCH
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| producer_unsupported())?;
        let salt = *PROCESS_SALT.get_or_init(|| {
            std::collections::hash_map::RandomState::new()
                .build_hasher()
                .finish()
        });
        Ok(Self {
            key,
            nonce: salt ^ ordinal,
            token: owner.participation_token(),
            validator: Rc::new(Schemas {
                input: invoke.input_schema,
                output: invoke.output_schema,
            }),
            definitions,
            max_records,
            records,
            next_work: 0,
            listener: None,
            exchange: None,
            delivery: None,
            first_cause: None,
            closed: false,
            participation: ProducerParticipationId::new(),
            gate: Rc::new(Cell::new(false)),
            retained_owner: None,
        })
    }

    pub fn bound_address(&self) -> Option<SocketAddr> {
        self.listener.as_ref()?.local_addr().ok()
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    pub fn take_delivery(&mut self) -> Option<HttpActionDelivery> {
        self.delivery.take()
    }
    pub fn records(&self) -> usize {
        self.records.len()
    }
    pub fn action_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.records.iter().map(|r| r.id)
    }

    /// Closed native setup, before any local publication or application call.
    pub fn install_closed(&mut self) -> CoreResult<()> {
        if self.closed || self.first_cause.is_some() {
            return Err(producer_unsupported());
        }
        if self.listener.is_some() {
            return Ok(());
        }
        let address = self.definitions[0].address;
        let socket = socket2::Socket::new(
            socket2::Domain::for_address(address),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .map_err(|_| self.fail(io_error()))?;
        self.listener = Some(socket.into());
        let result = (|| -> std::io::Result<()> {
            let sock = socket2::SockRef::from(self.listener.as_ref().unwrap());
            sock.set_reuse_address(true)?;
            sock.bind(&address.into())?;
            sock.listen(128)?;
            sock.set_nonblocking(true)
        })();
        result.map_err(|_| self.fail(io_error()))
    }
    fn fail(&mut self, error: CoreError) -> CoreError {
        self.first_cause.get_or_insert_with(|| error.clone());
        self.gate.set(false);
        error
    }
    pub fn begin_close(&mut self) {
        self.gate.set(false);
        self.closed = true;
        self.listener = None;
        // A possibly partially written reply retains its uncertainty in the
        // delivery record; the original Action remains in the logical store.
        if let Some(exchange) = self.exchange.take() {
            self.delivery = Some(HttpActionDelivery {
                invocation: exchange.invoked,
                locally_written: false,
                write_started: exchange.written != 0,
                cause: Some(CoreError::Cancelled(ErrorContext::new(
                    ErrorPhase::Cleanup,
                    RetryClass::Never,
                ))),
            });
        }
    }
    pub fn handle(&self, id: u32) -> Option<RawActionHandle> {
        self.records
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.handle.clone())
    }
    /// Progress one independent application continuation, even when no HTTP
    /// exchange is active. Completion remains observable over query until the
    /// caller explicitly acknowledges it; an unread result pins the original
    /// Action and capacity. The round-robin cursor cannot starve a second job.
    pub fn progress_actions(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<Option<u32>>> {
        if !self.token.belongs_to(owner) {
            return Err(producer_unsupported());
        }
        if self.records.is_empty() {
            return Ok(Poll::Ready(None));
        }
        let index = self.next_work % self.records.len();
        self.next_work = (index + 1) % self.records.len();
        let record = &mut self.records[index];
        let progress = owner.step_raw_action(&record.handle, budget)?;
        if progress.is_ready() {
            record.finished.get_or_insert_with(Instant::now);
        }
        Ok(progress.map(|()| Some(record.id)))
    }

    /// Bounded terminal-result retention policy. A status resource disappears
    /// only after the application continuation is terminal. Callers may choose
    /// a longer window or explicitly acknowledge by id instead.
    pub fn reap_completed(
        &mut self,
        owner: &mut RawProducerServient,
        age: Duration,
    ) -> CoreResult<Option<RawActionSettlement>> {
        let Some(id) = self
            .records
            .iter()
            .find(|r| r.finished.is_some_and(|t| t.elapsed() >= age))
            .map(|r| r.id)
        else {
            return Ok(None);
        };
        self.acknowledge(owner, id)
    }
    fn wire_id(&self, id: u32) -> String {
        format!("{:016x}-{id}", self.nonce)
    }
    /// Acknowledgement is explicit: unread results and application work pin the
    /// original logical instance even after closing the HTTP listener.
    pub fn acknowledge(
        &mut self,
        owner: &mut RawProducerServient,
        id: u32,
    ) -> CoreResult<Option<RawActionSettlement>> {
        let i = self
            .records
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(invalid)?;
        let settlement = owner.take_raw_action_settlement(&self.records[i].handle)?;
        if settlement.is_some() {
            self.records.remove(i);
        }
        Ok(settlement)
    }

    /// One bounded native I/O/dispatch step. No background task or native
    /// callback gains a handler; caller drives the same Servient for all facets.
    pub fn step(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<()>> {
        if self.closed || !self.token.belongs_to(owner) {
            return Err(producer_unsupported());
        }
        if !self.gate.get() {
            return Ok(Poll::Pending);
        }
        budget.consume(WorkClass::BindingPolls, 1).map_err(|_| {
            CoreError::Backpressure(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
        })?;
        if self.delivery.is_some() {
            return Ok(Poll::Pending);
        }
        if self.exchange.is_none() {
            let Some(listener) = &self.listener else {
                return Err(self.fail(io_error()));
            };
            match listener.accept() {
                Ok((stream, _)) => {
                    stream
                        .set_nonblocking(true)
                        .map_err(|_| self.fail(io_error()))?;
                    self.exchange = Some(Exchange {
                        stream,
                        deadline: Instant::now() + Duration::from_secs(10),
                        received: Vec::with_capacity(8192 + 4096),
                        response: Vec::new(),
                        written: 0,
                        invoked: None,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Poll::Pending),
                Err(_) => return Err(self.fail(io_error())),
            }
        }
        if self
            .exchange
            .as_ref()
            .is_some_and(|e| Instant::now() >= e.deadline)
        {
            let exchange = self.exchange.take().unwrap();
            self.delivery = Some(HttpActionDelivery {
                invocation: exchange.invoked,
                locally_written: false,
                write_started: exchange.written != 0,
                cause: Some(CoreError::TimedOut(ErrorContext::new(
                    ErrorPhase::Binding,
                    RetryClass::Never,
                ))),
            });
            return Ok(Poll::Ready(()));
        }
        if self.exchange.as_ref().unwrap().response.is_empty() {
            let exchange = self.exchange.as_mut().unwrap();
            let mut buf = [0u8; 4096];
            match exchange.stream.read(&mut buf) {
                Ok(0) => {
                    self.exchange = None;
                    return Ok(Poll::Ready(()));
                }
                Ok(n) if exchange.received.len() + n <= 8192 + 4096 => {
                    exchange.received.extend_from_slice(&buf[..n])
                }
                Ok(_) => {
                    self.exchange = None;
                    return Ok(Poll::Ready(()));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Poll::Pending),
                Err(_) => {
                    self.exchange = None;
                    return Ok(Poll::Ready(()));
                }
            }
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut request = httparse::Request::new(&mut headers);
            let end = match request.parse(&exchange.received) {
                Ok(httparse::Status::Partial) if exchange.received.len() <= 8192 => {
                    return Ok(Poll::Pending);
                }
                Ok(httparse::Status::Complete(end)) if end <= 8192 => end,
                _ => {
                    self.exchange = None;
                    return Ok(Poll::Ready(()));
                }
            };
            let mut length = None;
            let mut host = 0;
            let mut media = 0;
            let mut valid = request.version == Some(1);
            for h in request.headers.iter() {
                if h.name.eq_ignore_ascii_case("host") {
                    host += 1;
                    valid &= h.value == self.definitions[0].address.to_string().as_bytes();
                } else if h.name.eq_ignore_ascii_case("content-length") {
                    valid &= length.is_none();
                    length = std::str::from_utf8(h.value)
                        .ok()
                        .and_then(|n| n.parse::<usize>().ok());
                    valid &= length.is_some_and(|n| n <= 4096);
                } else if h.name.eq_ignore_ascii_case("content-type") {
                    media += 1;
                    valid &= h.value == b"application/json";
                } else if [
                    "transfer-encoding",
                    "expect",
                    "upgrade",
                    "content-encoding",
                    "trailer",
                    "authorization",
                ]
                .iter()
                .any(|name| h.name.eq_ignore_ascii_case(name))
                {
                    valid = false;
                }
            }
            valid &= host == 1 && media <= 1;
            if !valid {
                self.exchange = None;
                return Ok(Poll::Ready(()));
            }
            let Some(definition) = self
                .definitions
                .iter()
                .find(|d| {
                    request.method
                        == Some(match d.operation {
                            Operation::InvokeAction => "POST",
                            Operation::QueryAction => "GET",
                            _ => "DELETE",
                        })
                        && request.path.is_some_and(|path| {
                            if d.control {
                                path.starts_with(&format!("{}?id=", d.path))
                            } else {
                                path == d.path.as_ref()
                            }
                        })
                })
                .cloned()
            else {
                self.exchange = None;
                return Ok(Poll::Ready(()));
            };
            let body_len = if definition.control {
                0
            } else {
                length.unwrap_or(0)
            };
            if (definition.control && (length.unwrap_or(0) != 0 || media != 0))
                || (!definition.control && media != usize::from(body_len != 0))
                || exchange.received.len() > end + body_len
            {
                self.exchange = None;
                return Ok(Poll::Ready(()));
            }
            if exchange.received.len() < end + body_len {
                return Ok(Poll::Pending);
            }
            let path = request.path.unwrap().to_owned();
            let body = exchange.received[end..].to_vec();
            self.answer(owner, budget, &definition, &path, body)?;
        }
        let exchange = self.exchange.as_mut().unwrap();
        match exchange
            .stream
            .write(&exchange.response[exchange.written..])
        {
            Ok(0) => Ok(Poll::Pending),
            Ok(n) => {
                exchange.written += n;
                if exchange.written == exchange.response.len() {
                    self.delivery = Some(HttpActionDelivery {
                        invocation: exchange.invoked,
                        locally_written: true,
                        write_started: true,
                        cause: None,
                    });
                    self.exchange = None;
                    Ok(Poll::Ready(()))
                } else {
                    Ok(Poll::Pending)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(Poll::Pending),
            Err(_) => {
                self.delivery = Some(HttpActionDelivery {
                    invocation: exchange.invoked,
                    locally_written: false,
                    write_started: exchange.written != 0,
                    cause: Some(io_error()),
                });
                self.exchange = None;
                Ok(Poll::Ready(()))
            }
        }
    }
    fn answer(
        &mut self,
        owner: &mut RawProducerServient,
        budget: &mut WorkBudget,
        definition: &HttpActionDefinition,
        path: &str,
        body: Vec<u8>,
    ) -> CoreResult<()> {
        let mut status = 500;
        let mut result = String::from("{\"status\":\"failed\"}");
        let mut location = None;
        let input = if body.is_empty() {
            InteractionInput::empty()
        } else {
            InteractionInput::with_data(Payload::new(body, "application/json"))
        };
        let mut id = None;
        if definition.operation == Operation::InvokeAction {
            if self.records.len() < self.max_records {
                let request = RawActionRequest {
                    instance: self.key,
                    target: &AffordanceTarget::Action(Arc::clone(&definition.action)),
                    invoke: definition.route(),
                    query: self
                        .definitions
                        .iter()
                        .find(|d| d.operation == Operation::QueryAction)
                        .map(HttpActionDefinition::route),
                    cancel: self
                        .definitions
                        .iter()
                        .find(|d| d.operation == Operation::CancelAction)
                        .map(HttpActionDefinition::route),
                    input: &input,
                    validator: Some(Rc::clone(&self.validator)),
                };
                match owner.invoke_raw_action(request, budget) {
                    Ok(handle) => {
                        let number = handle.reference().slot().get();
                        self.records.push(Record {
                            id: number,
                            handle,
                            finished: None,
                        });
                        id = Some(number);
                        status = 202;
                        let token = self.wire_id(number);
                        result = format!("{{\"id\":\"{token}\",\"status\":\"accepted\"}}");
                        if let Some(query) = self
                            .definitions
                            .iter()
                            .find(|d| d.operation == Operation::QueryAction)
                        {
                            location =
                                Some(format!("http://{}{}?id={token}", query.address, query.path));
                        }
                    }
                    Err(CoreError::Validation(_)) => {
                        status = 400;
                        result = "{\"status\":\"invalid\"}".into();
                    }
                    Err(CoreError::Backpressure(_)) => {
                        status = 503;
                        result = "{\"status\":\"busy\"}".into();
                    }
                    Err(_) => {}
                }
            } else {
                status = 503;
                result = "{\"status\":\"busy\"}".into();
            }
        } else if let Some(number) = path
            .strip_prefix(&format!("{}?id={:016x}-", definition.path, self.nonce))
            .and_then(|n| (!n.is_empty() && n.bytes().all(|c| c.is_ascii_digit())).then_some(n))
            .and_then(|n| n.parse::<u32>().ok())
        {
            if let Some(handle) = self.handle(number) {
                let input = InteractionInput::empty();
                let control = if definition.operation == Operation::QueryAction {
                    owner
                        .query_raw_action(
                            &handle,
                            definition.instance_binding,
                            definition.route().mapping,
                            &input,
                            budget,
                        )
                        .map(|view| match view.state {
                            RawActionState::Running => String::from("{\"status\":\"running\"}"),
                            RawActionState::Complete(Ok(output)) => {
                                if let Some(data) = output.data() {
                                    format!(
                                        "{{\"status\":\"completed\",\"output\":{}}}",
                                        std::str::from_utf8(&data.body).unwrap_or("null")
                                    )
                                } else {
                                    String::from("{\"status\":\"completed\"}")
                                }
                            }
                            RawActionState::Complete(Err(_)) => {
                                String::from("{\"status\":\"failed\"}")
                            }
                        })
                } else {
                    owner
                        .cancel_raw_action(
                            &handle,
                            definition.instance_binding,
                            definition.route().mapping,
                            &input,
                            budget,
                        )
                        .map(|requested| {
                            if requested {
                                "{\"status\":\"cancelling\"}"
                            } else {
                                "{\"status\":\"terminal\"}"
                            }
                            .into()
                        })
                };
                if let Ok(value) = control {
                    status = 200;
                    result = value;
                } else {
                    status = 403;
                    result = "{\"status\":\"denied\"}".into();
                }
            } else {
                status = 404;
                result = "{\"status\":\"missing\"}".into();
            }
        } else {
            status = 400;
            result = "{\"status\":\"invalid\"}".into();
        }
        let exchange = self.exchange.as_mut().unwrap();
        exchange.invoked = id;
        exchange.response = format!("HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{result}",
            match status { 200 => "OK", 202 => "Accepted", 400 => "Bad Request", 403 => "Forbidden", 404 => "Not Found", 503 => "Service Unavailable", _ => "Internal Server Error" },
            result.len(), location.map_or(String::new(), |url| format!("Location: {url}\r\n"))).into_bytes();
        Ok(())
    }
}
impl ClosedProducerParticipation for HttpActionRuntime {
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.participation
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.retained_owner = Some(Rc::new((self.retained_owner.take(), owner)));
    }
    fn admission_backing_bytes(
        &self,
        owner: &RawProducerServient,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        if self.key != key
            || !self.token.belongs_to(owner)
            || self.listener.is_some()
            || self.retained_owner.is_some()
            || self.closed
        {
            return None;
        }
        let definitions = self.definitions.iter().try_fold(
            self.definitions
                .capacity()
                .checked_mul(size_of::<HttpActionDefinition>())?,
            |sum, d| {
                sum.checked_add(d.action.len())?
                    .checked_add(d.path.len())?
                    .checked_add(4 * size_of::<usize>())
            },
        )?;
        size_of::<Self>()
            .checked_add(definitions)?
            .checked_add(self.max_records.checked_mul(size_of::<Record>())?)?
            .checked_add(size_of::<Schemas>() + size_of::<Cell<bool>>() + size_of::<[usize; 8]>())?
            // Retained ingress, decoded payload, reply and bounded formatting
            // overlap. Native OS/socket state is separately Binding-owned.
            .checked_add(40 * 1024)
    }
    fn begin_retire(&mut self) {
        self.begin_close();
    }
    fn ready_for(&self, owner: &RawProducerServient, key: ProducerInstanceKey) -> bool {
        self.key == key
            && self.token.belongs_to(owner)
            && !self.closed
            && self.first_cause.is_none()
            && !self.gate.get()
            && self.bound_address() == Some(self.definitions[0].address)
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: AffordanceSite<'_>,
        form: u32,
        op: Operation,
        definition: &dyn Any,
    ) -> bool {
        key == self.key
            && matches!(site, AffordanceSite::Action(name) if name == self.definitions[0].action.as_ref())
            && self.definitions.iter().any(|d| {
                d.instance_binding == binding
                    && d.form == form
                    && d.operation == op
                    && definition.downcast_ref::<HttpActionDefinition>() == Some(d)
            })
    }
    fn ingress_gate(&self) -> &Rc<Cell<bool>> {
        &self.gate
    }
    fn retired(&self) -> bool {
        self.closed && self.exchange.is_none() && self.delivery.is_none() && self.records.is_empty()
    }
}

// Do not wait in Drop. Accepted work is retained by the Servient Action store;
// the caller must explicitly settle it before final publication retirement.
impl Drop for HttpActionRuntime {
    fn drop(&mut self) {
        self.begin_close();
    }
}
