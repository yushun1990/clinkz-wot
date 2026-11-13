//! Manually driven HTTP/1.1 Producer Action participation. A listener is one
//! possible native lease, not a logical Thing or a general Engine resource.
use crate::{producer::producer_unsupported, producer_validation};
use clinkz_wot_core::publication::{
    ClosedProducerParticipation, ProducerOwnerToken, ProducerParticipationId,
};
use clinkz_wot_core::{
    ActionInvocationRef, AffordanceTarget, BindingGeneration, BindingId, CoreError, CoreResult,
    ErrorContext, ErrorPhase, InteractionInput, InteractionOutput, Operation, Payload, RetryClass,
    action::{
        ActionMappingKey, ActionRoute, ActionValidator, ProducerActionBinding,
        ProducerActionCandidate, ProducerActionReply, RawActionState, RawActionView,
    },
    data_schema::ValueSchemas,
    producer::ProducerInstanceKey,
    thing::AffordanceSite,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
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
    // Immutable meaning survives definition clones and captured validators.
    // It has no invocation, native continuation or logical authority lifetime.
    pub(crate) schemas: Arc<ValueSchemas>,
    pub(crate) control: bool,
    pub(crate) collection: Option<clinkz_wot_core::action::ActionCollectionSchema>,
}
impl HttpActionDefinition {
    fn route(&self) -> ActionRoute {
        ActionRoute {
            binding: self.instance_binding,
            mapping: ActionMappingKey(u64::from(self.form) + 1),
        }
    }
}

struct Schemas(Arc<ValueSchemas>);
impl ActionValidator for Schemas {
    fn additional_validation_work(&self) -> u64 {
        // The same complete structured JSON envelope used by finite HTTP
        // validation. Property lookup can scan the payload for every described
        // field; the owner's single body traversal cannot fund that work.
        if self
            .0
            .input
            .iter()
            .chain(self.0.output.iter())
            .any(|s| !s.is_type_only())
        {
            4096 * 80
        } else {
            0
        }
    }
    fn validate_input(&self, input: &InteractionInput) -> CoreResult<()> {
        match (self.0.input.as_ref(), input.data.as_ref()) {
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
        match (self.0.output.as_ref(), output.data()) {
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
fn refused(error: &CoreError) -> (u16, &'static str) {
    match error {
        CoreError::Security { .. } => (403, "{\"status\":\"denied\"}"),
        CoreError::Backpressure(_) | CoreError::LimitExceeded { .. } => {
            (503, "{\"status\":\"busy\"}")
        }
        CoreError::StaleHandle(_) | CoreError::NotFound(_) => (410, "{\"status\":\"gone\"}"),
        CoreError::Validation(_) | CoreError::Payload(_) | CoreError::InvalidDocument(_) => {
            (400, "{\"status\":\"invalid\"}")
        }
        _ => (500, "{\"status\":\"failed\"}"),
    }
}

struct Record {
    id: u32,
    reference: ActionInvocationRef,
    definition: usize,
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
    request_error: Option<CoreError>,
    identity: Option<HttpActionExchangeIdentity>,
    definition: Option<usize>,
}

/// A completed exchange's local write is separate from application effect and
/// does not prove that the remote peer received or processed the response.
#[derive(Debug)]
pub struct HttpActionExchangeIdentity {
    pub instance: ProducerInstanceKey,
    pub target: AffordanceTarget,
    pub operation: Operation,
    pub route: ActionRoute,
}

#[derive(Debug)]
pub struct HttpActionDelivery {
    /// Original selected operation, including a root aggregate with no single
    /// invocation. A partial unclassified request has no semantic identity.
    pub exchange: Option<HttpActionExchangeIdentity>,
    pub invocation: Option<u32>,
    pub locally_written: bool,
    pub write_started: bool,
    pub cause: Option<CoreError>,
    /// Original semantic refusal, independent of native write success/failure.
    pub request_error: Option<CoreError>,
}

/// One HTTP native participant for several Actions and their actually prepared
/// Forms, optionally including a complete Thing-root status inventory. Other
/// native participants may compose on the same logical Thing.
pub struct HttpActionRuntime {
    key: ProducerInstanceKey,
    // Native correlation epoch: a stale Location from another lease (including
    // replacement) cannot address this participant's fresh Action records.
    nonce: u64,
    token: ProducerOwnerToken,
    definitions: Vec<HttpActionDefinition>,
    validators: Vec<(Arc<str>, Rc<dyn ActionValidator>)>,
    max_records: usize,
    records: Vec<Record>,
    aggregate_records: Vec<(u32, String, String)>,
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
        owner: &ProducerOwnerToken,
        key: ProducerInstanceKey,
        definitions: Vec<HttpActionDefinition>,
        max_records: usize,
    ) -> CoreResult<Self> {
        let Some(invoke) = definitions.first() else {
            return Err(producer_unsupported());
        };
        // Native completeness: every Action named by the root must have its
        // entire invoke/control set in this SAME actual correlation owner.
        if max_records == 0
            || max_records > 128
            || definitions.len() > 32
            || definitions
                .iter()
                .filter(|d| d.operation == Operation::QueryAllActions)
                .count()
                > 1
            || definitions.iter().any(|d| {
                d.address != invoke.address || d.instance_binding != invoke.instance_binding
            })
            || definitions.iter().enumerate().any(|(i, d)| {
                definitions[..i].iter().any(|p| {
                    p.operation == d.operation && (p.action == d.action || p.path == d.path)
                })
            })
        {
            return Err(producer_unsupported());
        }
        let mut validators: Vec<(Arc<str>, Rc<dyn ActionValidator>)> = Vec::new();
        for d in definitions
            .iter()
            .filter(|d| d.operation != Operation::QueryAllActions)
        {
            if d.collection.is_some()
                || d.control != (d.operation != Operation::InvokeAction)
                || ![
                    Operation::InvokeAction,
                    Operation::QueryAction,
                    Operation::CancelAction,
                ]
                .iter()
                .all(|op| {
                    definitions.iter().any(|other| {
                        other.action == d.action
                            && other.operation == *op
                            && other.schemas == d.schemas
                    })
                })
            {
                return Err(producer_unsupported());
            }
            if d.operation == Operation::InvokeAction {
                validators.push((
                    Arc::clone(&d.action),
                    Rc::new(Schemas(Arc::clone(&d.schemas))),
                ));
            }
        }
        if validators.is_empty()
            && definitions.iter().all(|d| {
                d.collection
                    .as_ref()
                    .is_none_or(|s| !s.members().is_empty())
            })
        {
            return Err(producer_unsupported());
        }
        for root in definitions
            .iter()
            .filter(|d| d.operation == Operation::QueryAllActions)
        {
            let schema = root.collection.as_ref().ok_or_else(producer_unsupported)?;
            if root.control
                || schema.members().len() != validators.len()
                || schema.members().iter().any(|m| {
                    !definitions.iter().any(|d| {
                        d.operation == Operation::InvokeAction
                            && d.action.as_ref() == m.name.as_ref()
                            && d.schemas.output == m.output
                    })
                })
            {
                return Err(producer_unsupported());
            }
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
            token: owner.clone(),
            validators,
            definitions,
            max_records,
            records,
            aggregate_records: Vec::with_capacity(max_records),
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
                request_error: exchange.request_error,
                exchange: exchange.identity,
            });
        }
    }
    fn wire_id(&self, id: u32) -> String {
        format!("{:016x}-{id}", self.nonce)
    }
}
impl ProducerActionBinding for HttpActionRuntime {
    fn instance(&self) -> ProducerInstanceKey {
        self.key
    }
    fn invocation_capacity(&self) -> bool {
        self.records.len() < self.max_records
    }
    fn invocation(&self, id: u32) -> Option<ActionInvocationRef> {
        self.records
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.reference)
    }
    fn forget_invocation(&mut self, reference: ActionInvocationRef) {
        self.records.retain(|r| r.reference != reference);
    }
    fn action_inventory(&self) -> impl ExactSizeIterator<Item = ActionInvocationRef> {
        self.records.iter().rev().map(|record| record.reference)
    }
    /// One bounded native I/O step, yielding a candidate or local completion.
    /// No callback receives logical dispatch or continuation authority.
    fn poll_action(
        &mut self,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<Option<ProducerActionCandidate>>> {
        match self.native_step(budget) {
            Ok(progress) => progress.map(Ok),
            Err(error) => Poll::Ready(Err(error)),
        }
    }
    fn begin_action_collection(&mut self, budget: &mut WorkBudget) -> CoreResult<()> {
        budget
            .consume(WorkClass::BindingPolls, 65_536)
            .map_err(|_| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
            })?;
        self.aggregate_records.clear();
        Ok(())
    }
    fn push_action_status(
        &mut self,
        name: &str,
        reference: ActionInvocationRef,
        view: RawActionView<'_>,
    ) -> CoreResult<()> {
        let record = self
            .records
            .iter()
            .find(|r| r.reference == reference)
            .ok_or_else(invalid)?;
        let invoke = &self.definitions[record.definition];
        let query = self
            .definitions
            .iter()
            .find(|d| d.action == invoke.action && d.operation == Operation::QueryAction)
            .ok_or_else(invalid)?;
        let href = format!(
            "http://{}{}?id={}",
            query.address,
            query.path,
            self.wire_id(record.id)
        );
        let (state, output) = match view.state {
            RawActionState::Running => ("running", None),
            RawActionState::Complete(Ok(output)) => ("completed", output.data()),
            RawActionState::Complete(Err(_)) => ("failed", None),
        };
        let mut value = format!(
            "{{\"status\":\"{state}\",\"href\":{}}}",
            serde_json::to_string(&href).map_err(|_| invalid())?
        );
        if let Some(output) = output {
            value.pop();
            value.push_str(",\"output\":");
            value.push_str(std::str::from_utf8(output.body).map_err(|_| invalid())?);
            value.push('}');
        }
        if self.aggregate_records.len() >= self.max_records
            || self
                .aggregate_records
                .iter()
                .map(|(_, name, value)| name.len() + value.len() + 4)
                .sum::<usize>()
                + name.len()
                + value.len()
                + 4
                > 4096
        {
            return Err(CoreError::Backpressure(ErrorContext::new(
                ErrorPhase::Binding,
                RetryClass::Never,
            )));
        }
        self.aggregate_records.push((record.id, name.into(), value));
        Ok(())
    }
    fn answer_action(
        &mut self,
        candidate: &ProducerActionCandidate,
        reply: Result<ProducerActionReply<'_>, &CoreError>,
    ) -> CoreResult<()> {
        let index = self
            .exchange
            .as_ref()
            .and_then(|e| e.definition)
            .ok_or_else(invalid)?;
        let definition = &self.definitions[index];
        if candidate.instance != self.key
            || candidate.route != definition.route()
            || candidate.operation != definition.operation
        {
            return Err(invalid());
        }
        let mut location = None;
        let mut id = None;
        let (status, result) = match reply {
            Err(error) => {
                self.exchange.as_mut().unwrap().request_error = Some(error.clone());
                let (status, body) = refused(error);
                (status, body.into())
            }
            Ok(ProducerActionReply::Accepted(reference)) => {
                let number = reference.slot().get();
                self.records.push(Record {
                    id: number,
                    reference,
                    definition: index,
                });
                id = Some(number);
                let token = self.wire_id(number);
                if let Some(query) = self.definitions.iter().find(|d| {
                    d.action == definition.action && d.operation == Operation::QueryAction
                }) {
                    location = Some(format!("http://{}{}?id={token}", query.address, query.path));
                }
                (
                    202,
                    format!("{{\"id\":\"{token}\",\"status\":\"accepted\"}}"),
                )
            }
            Ok(ProducerActionReply::Status(view)) => (
                200,
                match view.state {
                    RawActionState::Running => String::from("{\"status\":\"running\"}"),
                    RawActionState::Complete(Ok(output)) => {
                        if let Some(data) = output.data() {
                            format!(
                                "{{\"status\":\"completed\",\"output\":{}}}",
                                std::str::from_utf8(data.body).unwrap_or("null")
                            )
                        } else {
                            String::from("{\"status\":\"completed\"}")
                        }
                    }
                    RawActionState::Complete(Err(_)) => String::from("{\"status\":\"failed\"}"),
                },
            ),
            Ok(ProducerActionReply::CancellationRequested(requested)) => (
                200,
                if requested {
                    "{\"status\":\"cancelling\"}"
                } else {
                    "{\"status\":\"terminal\"}"
                }
                .into(),
            ),
            Ok(ProducerActionReply::Collection) => match self.aggregate(definition) {
                Ok(body) => (200, body),
                Err(error) => {
                    let (status, body) = refused(&error);
                    self.exchange.as_mut().unwrap().request_error = Some(error);
                    (status, body.into())
                }
            },
        };
        self.set_response(status, result, location, id);
        self.aggregate_records.clear();
        Ok(())
    }
}
impl HttpActionRuntime {
    fn native_step(
        &mut self,
        budget: &mut WorkBudget,
    ) -> CoreResult<Poll<Option<ProducerActionCandidate>>> {
        if self.closed {
            return Err(producer_unsupported());
        }
        // Revocation fences new connections and semantic ingress, not a socket
        // already accepted by this native owner or its pending response write.
        if (!self.gate.get() || !self.token.is_alive()) && self.exchange.is_none() {
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
                        request_error: None,
                        identity: None,
                        definition: None,
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
                request_error: exchange.request_error,
                exchange: exchange.identity,
            });
            return Ok(Poll::Ready(None));
        }
        if self
            .exchange
            .as_ref()
            .is_some_and(|e| e.definition.is_some() && e.response.is_empty())
        {
            return Ok(Poll::Pending);
        }
        if self.exchange.as_ref().unwrap().response.is_empty() {
            let exchange = self.exchange.as_mut().unwrap();
            let mut buf = [0u8; 4096];
            match exchange.stream.read(&mut buf) {
                Ok(0) => {
                    self.exchange = None;
                    return Ok(Poll::Ready(None));
                }
                Ok(n) if exchange.received.len() + n <= 8192 + 4096 => {
                    exchange.received.extend_from_slice(&buf[..n])
                }
                Ok(_) => {
                    self.exchange = None;
                    return Ok(Poll::Ready(None));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(Poll::Pending),
                Err(_) => {
                    self.exchange = None;
                    return Ok(Poll::Ready(None));
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
                    return Ok(Poll::Ready(None));
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
                return Ok(Poll::Ready(None));
            }
            let Some(definition) = self
                .definitions
                .iter()
                .find(|d| {
                    request.method
                        == Some(match d.operation {
                            Operation::InvokeAction => "POST",
                            Operation::QueryAction | Operation::QueryAllActions => "GET",
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
                return Ok(Poll::Ready(None));
            };
            let no_input = definition.operation != Operation::InvokeAction;
            let body_len = if no_input { 0 } else { length.unwrap_or(0) };
            if (no_input && (length.unwrap_or(0) != 0 || media != 0))
                || (!no_input && media != usize::from(body_len != 0))
                || exchange.received.len() > end + body_len
            {
                self.exchange = None;
                return Ok(Poll::Ready(None));
            }
            if exchange.received.len() < end + body_len {
                return Ok(Poll::Pending);
            }
            let path = request.path.unwrap().to_owned();
            let body = exchange.received[end..].to_vec();
            // A complete request arriving on a previously accepted connection
            // after revocation is not an accepted Action/control operation.
            if !self.gate.get() || !self.token.is_alive() {
                self.exchange = None;
                return Ok(Poll::Ready(None));
            }
            let index = self
                .definitions
                .iter()
                .position(|d| d == &definition)
                .unwrap();
            self.exchange.as_mut().unwrap().definition = Some(index);
            let target = if definition.operation == Operation::QueryAllActions {
                AffordanceTarget::Thing
            } else {
                AffordanceTarget::Action(Arc::clone(&definition.action))
            };
            self.exchange.as_mut().unwrap().identity = Some(HttpActionExchangeIdentity {
                instance: self.key,
                target: target.clone(),
                operation: definition.operation,
                route: definition.route(),
            });
            let invocation = if definition.control {
                let reference = path
                    .strip_prefix(&format!("{}?id=", definition.path))
                    .and_then(|token| token.split_once('-'))
                    .filter(|(epoch, n)| {
                        epoch.len() == 16
                            && epoch.bytes().all(|c| c.is_ascii_hexdigit())
                            && !n.is_empty()
                            && n.bytes().all(|c| c.is_ascii_digit())
                    })
                    .and_then(|(epoch, n)| {
                        Some((u64::from_str_radix(epoch, 16).ok()?, n.parse::<u32>().ok()?))
                    });
                let Some((epoch, number)) = reference else {
                    self.set_response(400, "{\"status\":\"invalid\"}".into(), None, None);
                    return Ok(Poll::Pending);
                };
                let Some(record) = self.records.iter().find(|r| {
                    epoch == self.nonce
                        && r.id == number
                        && self.definitions[r.definition].action == definition.action
                }) else {
                    self.set_response(404, "{\"status\":\"missing\"}".into(), None, None);
                    return Ok(Poll::Pending);
                };
                Some(record.reference)
            } else {
                None
            };
            return Ok(Poll::Ready(Some(ProducerActionCandidate {
                instance: self.key,
                target,
                operation: definition.operation,
                route: definition.route(),
                query: self
                    .definitions
                    .iter()
                    .find(|d| {
                        d.action == definition.action && d.operation == Operation::QueryAction
                    })
                    .map(HttpActionDefinition::route),
                cancel: self
                    .definitions
                    .iter()
                    .find(|d| {
                        d.action == definition.action && d.operation == Operation::CancelAction
                    })
                    .map(HttpActionDefinition::route),
                invocation,
                input: if body.is_empty() {
                    InteractionInput::empty()
                } else {
                    InteractionInput::with_data(Payload::new(body, "application/json"))
                },
            })));
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
                        request_error: exchange.request_error.take(),
                        exchange: exchange.identity.take(),
                    });
                    self.exchange = None;
                    Ok(Poll::Ready(None))
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
                    request_error: exchange.request_error.take(),
                    exchange: exchange.identity.take(),
                });
                self.exchange = None;
                Ok(Poll::Ready(None))
            }
        }
    }
    fn aggregate(&self, definition: &HttpActionDefinition) -> CoreResult<String> {
        let schema = definition
            .collection
            .as_ref()
            .ok_or_else(producer_unsupported)?;
        let records = &self.aggregate_records;
        let mut result = String::from("{");
        for (i, member) in schema.members().iter().enumerate() {
            if i != 0 {
                result.push(',');
            }
            result.push_str(&serde_json::to_string(member.name.as_ref()).map_err(|_| invalid())?);
            result.push_str(":[");
            let mut first = true;
            for record in self.records.iter().rev().filter(|record| {
                self.definitions[record.definition].action.as_ref() == member.name.as_ref()
            }) {
                let Some((_, _, value)) = records
                    .iter()
                    .find(|(id, name, _)| *id == record.id && name == member.name.as_ref())
                else {
                    continue;
                };
                if !first {
                    result.push(',');
                }
                first = false;
                result.push_str(value);
            }
            result.push(']');
        }
        result.push('}');
        crate::action_status::validate(
            schema,
            &Payload::new(result.as_bytes().to_vec(), "application/json"),
        )?;
        Ok(result)
    }

    fn set_response(
        &mut self,
        status: u16,
        result: String,
        location: Option<String>,
        id: Option<u32>,
    ) {
        let exchange = self.exchange.as_mut().unwrap();
        exchange.invoked = id;
        exchange.response = format!("HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{result}",
            match status { 200 => "OK", 202 => "Accepted", 400 => "Bad Request", 403 => "Forbidden", 404 => "Not Found", 410 => "Gone", 503 => "Service Unavailable", _ => "Internal Server Error" },
            result.len(), location.map_or(String::new(), |url| format!("Location: {url}\r\n"))).into_bytes();
    }
}
impl ClosedProducerParticipation for HttpActionRuntime {
    fn associated_with(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool {
        self.key == key && self.token.same_owner(owner)
    }
    fn native_failure(&self) -> Option<&CoreError> {
        self.first_cause.as_ref()
    }
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.participation
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.retained_owner = Some(Rc::new((self.retained_owner.take(), owner)));
    }
    fn admission_backing_bytes(
        &self,
        owner: &ProducerOwnerToken,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        if self.key != key
            || !self.token.same_owner(owner)
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
                    .checked_add(d.schemas.retained_bytes())?
                    .checked_add(size_of::<ValueSchemas>() + 2 * size_of::<usize>())?
                    .checked_add(d.collection.as_ref().map_or(0, |s| s.retained_bytes()))?
                    .checked_add(4 * size_of::<usize>())
            },
        )?;
        size_of::<Self>()
            .checked_add(definitions)?
            .checked_add(
                self.max_records
                    .checked_mul(size_of::<Record>() + size_of::<(u32, String, String)>())?,
            )?
            .checked_add(
                self.validators.capacity() * size_of::<(Arc<str>, Rc<dyn ActionValidator>)>(),
            )?
            .checked_add(self.validators.len() * (size_of::<Schemas>() + 2 * size_of::<usize>()))?
            .checked_add(size_of::<Cell<bool>>() + size_of::<[usize; 8]>())?
            // Retained ingress, decoded payload, reply and bounded formatting
            // overlap. Native OS/socket state is separately Binding-owned.
            .checked_add(128 * 1024)
    }
    fn begin_retire(&mut self) {
        self.begin_close();
    }
    fn ready_for(&self, owner: &ProducerOwnerToken, key: ProducerInstanceKey) -> bool {
        self.key == key
            && self.token.same_owner(owner)
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
            && self.definitions.iter().any(|d| {
                (match site {
                    AffordanceSite::Action(name) => {
                        name == d.action.as_ref() && d.operation != Operation::QueryAllActions
                    }
                    AffordanceSite::Thing => d.operation == Operation::QueryAllActions,
                    _ => false,
                }) && d.instance_binding == binding
                    && d.form == form
                    && d.operation == op
                    && definition.downcast_ref::<HttpActionDefinition>() == Some(d)
            })
    }
    fn action_validation(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: AffordanceSite<'_>,
        form: u32,
        definition: &dyn Any,
    ) -> Option<(ActionRoute, Rc<dyn ActionValidator>)> {
        self.covers(
            key,
            binding,
            site,
            form,
            Operation::InvokeAction,
            definition,
        )
        .then(|| {
            (
                definition
                    .downcast_ref::<HttpActionDefinition>()
                    .unwrap()
                    .route(),
                Rc::clone(&self.validators.iter().find(|(name,_)| matches!(site, AffordanceSite::Action(n) if n == name.as_ref())).unwrap().1),
            )
        })
    }
    fn action_control_route(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> Option<ActionRoute> {
        matches!(
            operation,
            Operation::QueryAction | Operation::CancelAction | Operation::QueryAllActions
        )
        .then_some(())
        .filter(|_| self.covers(key, binding, site, form, operation, definition))
        .map(|_| {
            definition
                .downcast_ref::<HttpActionDefinition>()
                .unwrap()
                .route()
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
