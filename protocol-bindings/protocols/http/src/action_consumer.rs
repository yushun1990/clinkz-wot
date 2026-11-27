//! Consumer counterpart of the qualified Producer's documented native mapping.
//! Tokens, template expansion, framing and status conversion stay entirely here.
use super::*;
use clinkz_wot_core::{artifact::try_box, consumer_action::*};
use std::{
    any::Any,
    future::poll_fn,
    io::{ErrorKind, Write},
};

const BODY_LIMIT: usize = 4096;
const MESSAGE_LIMIT: usize = 16_384;
const TOKEN_LIMIT: usize = 27;
const IO_CHUNK: usize = 4096;

#[derive(Clone)]
pub struct HttpActionConsumer {
    identity: BindingRegistrationIdentity,
    busy: Arc<AtomicBool>,
}
impl HttpActionConsumer {
    pub fn new(
        config: HttpConsumerConfig,
        id: BindingId,
        generation: BindingGeneration,
        ordinal: u32,
    ) -> Self {
        Self {
            identity: BindingRegistrationIdentity::new(
                id,
                generation,
                config.configuration.configuration(),
                COMPATIBILITY,
                ordinal,
            ),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }
    /// Construct this concrete native mapping without TD/Planning. This is not
    /// TD qualification; callers must provide the common definition/services.
    pub fn mapping(
        &self,
        endpoint: &str,
        operation: Operation,
    ) -> CoreResult<Box<dyn ConsumerActionMapping>> {
        let control = match operation {
            Operation::InvokeAction => false,
            Operation::QueryAction | Operation::CancelAction => true,
            _ => return Err(unsupported()),
        };
        let base = if control {
            endpoint.strip_suffix("{?id}").ok_or_else(unsupported)?
        } else {
            endpoint
        };
        let target = Target::parse(base)?;
        let path = target.uri.path_and_query().map_or("/", |p| p.as_str());
        if path.len() > 64 || (control && path.contains('?')) {
            return Err(unsupported());
        }
        Ok(Box::new(Mapping {
            registration: self.clone(),
            address: target.address,
            path: Arc::from(path),
            operation,
        }))
    }
}
struct Mapping {
    registration: HttpActionConsumer,
    address: SocketAddr,
    path: Arc<str>,
    operation: Operation,
}
/// Opaque to the Engine and never a permission token. The original invoke
/// mapping identity is retained independently of an exchange's correlation.
struct Continuation {
    address: SocketAddr,
    token: [u8; TOKEN_LIMIT],
    len: usize,
}
impl Continuation {
    fn token(&self) -> &str {
        std::str::from_utf8(&self.token[..self.len]).expect("checked HTTP Action token")
    }
}
impl Mapping {
    fn continuation<'a>(&self, c: Option<&'a dyn Any>) -> CoreResult<&'a Continuation> {
        let c = c
            .and_then(|c| c.downcast_ref::<Continuation>())
            .ok_or_else(invalid)?;
        // The native counterpart has a per-listener invocation store. Different
        // registrations can use its token; another listener cannot.
        if c.address != self.address {
            return Err(invalid());
        }
        Ok(c)
    }
}
impl ConsumerActionMapping for Mapping {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.registration.identity
    }
    fn operation(&self) -> Operation {
        self.operation
    }
    fn bounds(&self) -> ConsumerActionBounds {
        ConsumerActionBounds {
            exchange_bytes: 32768,
            temporary_bytes: 32768,
            continuation_bytes: if self.operation == Operation::InvokeAction {
                std::mem::size_of::<Continuation>() as u64
            } else {
                0
            },
            response_bytes: if self.operation == Operation::QueryAction {
                (BODY_LIMIT + std::mem::size_of::<ConsumerActionOutput>()) as u64
            } else {
                0
            },
        }
    }
    fn native_definition(&self) -> &dyn Any {
        self
    }
    fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()> {
        let invoke = invoke
            .native_definition()
            .downcast_ref::<Mapping>()
            .ok_or_else(unsupported)?;
        if invoke.operation != Operation::InvokeAction
            || invoke.address != self.address
            || self.operation == Operation::InvokeAction
        {
            return Err(unsupported());
        }
        Ok(())
    }
    fn destination(&self, continuation: Option<&dyn Any>) -> CoreResult<Box<str>> {
        if self.operation == Operation::InvokeAction {
            Ok(format!("http://{}{}", self.address, self.path).into())
        } else {
            let c = self.continuation(continuation)?;
            Ok(format!("http://{}{}?id={}", self.address, self.path, c.token()).into())
        }
    }
    fn start(
        &self,
        destination: &str,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>> {
        let handle = transport::runtime()?;
        let target = Target::parse(destination)?;
        if target.address != self.address
            || (self.operation != Operation::InvokeAction && input.is_some())
        {
            return Err(invalid());
        }
        let path = target.uri.path_and_query().ok_or_else(invalid)?;
        if self.operation == Operation::InvokeAction {
            if path.as_str() != &*self.path || continuation.is_some() {
                return Err(invalid());
            }
        } else if path.path() != &*self.path
            || path.query().and_then(|q| q.strip_prefix("id="))
                != Some(self.continuation(continuation)?.token())
        {
            return Err(invalid());
        }
        if input.is_some_and(|p| {
            p.body.len() > BODY_LIMIT
                || p.content_type != "application/json"
                || p.content_coding.is_some()
        }) {
            return Err(invalid());
        }
        let occupancy = Occupancy::claim(&self.registration.busy)?;
        let method = match self.operation {
            Operation::InvokeAction => "POST",
            Operation::QueryAction => "GET",
            _ => "DELETE",
        };
        let body = input.map_or(&[][..], |p| p.body.as_ref());
        let media = if body.is_empty() {
            ""
        } else {
            "Content-Type: application/json\r\n"
        };
        // Reserve the complete wire and handoff buffers before the first socket
        // poll. The successful reply moves its token or output out of this owner.
        let mut request = buffer(body.len() + 512)?;
        write!(request, "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n{media}Content-Length: {}\r\n\r\n",
            self.address, body.len()).map_err(|_| invalid())?;
        request.extend_from_slice(body);
        let backing = ReplyBacking {
            bytes: buffer(MESSAGE_LIMIT)?,
            output: if self.operation == Operation::QueryAction {
                buffer(BODY_LIMIT)?
            } else {
                Vec::new()
            },
            continuation: if self.operation == Operation::InvokeAction {
                Some(
                    try_box(Continuation {
                        address: self.address,
                        token: [0; TOKEN_LIMIT],
                        len: 0,
                    })
                    .map_err(|_| busy())?,
                )
            } else {
                None
            },
        };
        let io_available = Arc::new(AtomicBool::new(false));
        let future = try_box(exchange(
            target.address,
            request,
            self.operation,
            io_available.clone(),
            backing,
        ))
        .map_err(|_| busy())?;
        Ok(try_box(Exchange {
            future: Some(Box::into_pin(future)),
            handle,
            io_available,
            _occupancy: occupancy,
        })
        .map_err(|_| busy())?)
    }
}
fn buffer(bytes: usize) -> CoreResult<Vec<u8>> {
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(bytes).map_err(|_| busy())?;
    Ok(buffer)
}
struct Occupancy(Arc<AtomicBool>);
impl Occupancy {
    fn claim(busy: &Arc<AtomicBool>) -> CoreResult<Self> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| CoreError::Backpressure(context()))?;
        Ok(Self(busy.clone()))
    }
}
impl Drop for Occupancy {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
struct ReplyBacking {
    bytes: Vec<u8>,
    output: Vec<u8>,
    continuation: Option<Box<Continuation>>,
}
type ExchangeFuture = Pin<Box<dyn Future<Output = CoreResult<ConsumerActionReply>>>>;
struct Exchange {
    future: Option<ExchangeFuture>,
    handle: tokio::runtime::Handle,
    io_available: Arc<AtomicBool>,
    _occupancy: Occupancy,
}
impl ConsumerActionExchange for Exchange {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>> {
        self.io_available.store(true, Ordering::Relaxed);
        let _entered = self.handle.enter();
        let result = self.future.as_mut().ok_or_else(invalid);
        match result {
            Ok(f) => match f.as_mut().poll(cx) {
                Poll::Ready(r) => {
                    self.future = None;
                    Poll::Ready(r)
                }
                Poll::Pending => Poll::Pending,
            },
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_close(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<Option<ConsumerActionReply>>> {
        self.future = None;
        Poll::Ready(Ok(None))
    }
    fn abort_local(self: Box<Self>) {
        drop(self);
    }
}
async fn exchange(
    address: SocketAddr,
    request: Vec<u8>,
    operation: Operation,
    io_available: Arc<AtomicBool>,
    mut backing: ReplyBacking,
) -> CoreResult<ConsumerActionReply> {
    let socket = tokio::net::TcpStream::connect(address)
        .await
        .map_err(|_| error(650, "HTTP Action connect failed"))?;
    let mut written = 0;
    while written != request.len() {
        let n = poll_fn(|cx| {
            if !io_available.swap(false, Ordering::Relaxed) {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let available = IO_CHUNK.min(request.len() - written);
            match socket.poll_write_ready(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(_)) => Poll::Ready(Err(error(651, "HTTP Action write failed"))),
                Poll::Ready(Ok(_)) => {
                    match socket.try_write(&request[written..written + available]) {
                        Ok(0) => Poll::Ready(Err(error(651, "HTTP Action write ended"))),
                        Ok(n) => Poll::Ready(Ok(n)),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        }
                        Err(_) => Poll::Ready(Err(error(651, "HTTP Action write failed"))),
                    }
                }
            }
        })
        .await?;
        written += n;
    }
    let bytes = &mut backing.bytes;
    let mut header_complete = false;
    loop {
        let mut scratch = [0u8; IO_CHUNK];
        let n = poll_fn(|cx| {
            if !io_available.swap(false, Ordering::Relaxed) {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            match socket.poll_read_ready(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(_)) => Poll::Ready(Err(error(652, "HTTP Action read failed"))),
                Poll::Ready(Ok(_)) => match socket.try_read(&mut scratch) {
                    Ok(n) => Poll::Ready(Ok(n)),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Err(_) => Poll::Ready(Err(error(652, "HTTP Action read failed"))),
                },
            }
        })
        .await?;
        if n == 0 {
            break;
        }
        if n > MESSAGE_LIMIT - bytes.len() {
            return Err(invalid());
        }
        bytes.extend_from_slice(&scratch[..n]);
        // Reject malformed/oversized headers as soon as they are inspectable.
        if !header_complete {
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut response = httparse::Response::new(&mut headers);
            match response.parse(bytes).map_err(|_| invalid())? {
                httparse::Status::Partial if bytes.len() >= 8192 => return Err(invalid()),
                httparse::Status::Complete(end) if end <= 8192 => header_complete = true,
                httparse::Status::Complete(_) => return Err(invalid()),
                _ => {}
            }
        }
    }
    decode(backing, operation)
}

struct Envelope<'a> {
    status: &'a str,
    id: Option<&'a str>,
    output: Option<&'a serde_json::value::RawValue>,
}
impl<'de> serde::Deserialize<'de> for Envelope<'de> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Envelope<'de>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an Action response")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let (mut status, mut id, mut output) = (None, None, None);
                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "status" if status.is_none() => status = Some(map.next_value()?),
                        "id" if id.is_none() => id = Some(map.next_value()?),
                        "output" if output.is_none() => output = Some(map.next_value()?),
                        _ => {
                            return Err(serde::de::Error::custom(
                                "unsupported or duplicate Action response field",
                            ));
                        }
                    }
                }
                Ok(Envelope {
                    status: status.ok_or_else(|| serde::de::Error::missing_field("status"))?,
                    id,
                    output,
                })
            }
        }
        d.deserialize_map(Visitor)
    }
}
fn decode(mut backing: ReplyBacking, operation: Operation) -> CoreResult<ConsumerActionReply> {
    let bytes = &backing.bytes;
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut response = httparse::Response::new(&mut headers);
    let end = match response.parse(bytes).map_err(|_| invalid())? {
        httparse::Status::Complete(end) if end <= 8192 => end,
        _ => return Err(invalid()),
    };
    let expected = if operation == Operation::InvokeAction {
        202
    } else {
        200
    };
    if response.version != Some(1) {
        return Err(invalid());
    }
    if response.code != Some(expected) {
        return Err(response_error(response.code));
    }
    let (mut media, mut length, mut location) = (false, None, false);
    for h in response.headers {
        if h.name.eq_ignore_ascii_case("content-type") {
            if media || h.value != b"application/json" {
                return Err(invalid());
            }
            media = true;
        } else if h.name.eq_ignore_ascii_case("content-length") {
            if length.is_some() || h.value.is_empty() || !h.value.iter().all(u8::is_ascii_digit) {
                return Err(invalid());
            }
            length = Some(
                std::str::from_utf8(h.value)
                    .map_err(|_| invalid())?
                    .parse::<usize>()
                    .map_err(|_| invalid())?,
            );
        } else if h.name.eq_ignore_ascii_case("location") {
            // Described Forms alone select controls. A wire Location can never
            // redirect authority or cause a hidden network request.
            if location || operation != Operation::InvokeAction {
                return Err(invalid());
            }
            location = true;
        } else if [
            "transfer-encoding",
            "content-encoding",
            "trailer",
            "upgrade",
        ]
        .iter()
        .any(|n| h.name.eq_ignore_ascii_case(n))
        {
            return Err(invalid());
        }
    }
    if !media || length.is_none_or(|n| n > 8192 || bytes.len() != end + n) {
        return Err(invalid());
    }
    let envelope: Envelope<'_> = serde_json::from_slice(&bytes[end..]).map_err(|_| invalid())?;
    match (operation, envelope.status, envelope.id, envelope.output) {
        (Operation::InvokeAction, "accepted", Some(token), None) if valid_token(token) => {
            let mut continuation = backing.continuation.take().ok_or_else(invalid)?;
            continuation.token[..token.len()].copy_from_slice(token.as_bytes());
            continuation.len = token.len();
            Ok(ConsumerActionReply::Accepted(continuation))
        }
        (Operation::QueryAction, "running", None, None) => Ok(ConsumerActionReply::Pending),
        (Operation::QueryAction, "completed", None, output) => {
            let output = match output {
                Some(raw) if raw.get().len() <= BODY_LIMIT => {
                    backing.output.extend_from_slice(raw.get().as_bytes());
                    ConsumerActionOutput::buffered(backing.output, "application/json")
                }
                Some(_) => return Err(invalid()),
                None => InteractionOutput::empty().into(),
            };
            Ok(ConsumerActionReply::Complete(output))
        }
        (Operation::QueryAction, "failed", None, None) => Ok(ConsumerActionReply::Failed(error(
            654,
            "remote Action execution failed",
        ))),
        (Operation::CancelAction, "cancelling", None, None) => {
            Ok(ConsumerActionReply::CancelRequested)
        }
        (Operation::CancelAction, "terminal", None, None) => {
            Ok(ConsumerActionReply::AlreadyTerminal)
        }
        _ => Err(invalid()),
    }
}
fn valid_token(token: &str) -> bool {
    token.split_once('-').is_some_and(|(epoch, number)| {
        epoch.len() == 16
            && epoch.bytes().all(|b| b.is_ascii_hexdigit())
            && !number.is_empty()
            && number.len() <= 10
            && number.bytes().all(|b| b.is_ascii_digit())
            && number.parse::<u32>().is_ok()
    })
}
pub(crate) fn check_control_destination(target: &str) -> CoreResult<()> {
    let base = if let Some(base) = target.strip_suffix("{?id}") {
        base
    } else if let Some((base, token)) = target.split_once("?id=") {
        if !valid_token(token) {
            return Err(invalid());
        }
        base
    } else {
        return Err(unsupported());
    };
    if base.contains('?') {
        return Err(unsupported());
    }
    Target::parse(base).map(|_| ())
}
fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn response_error(code: Option<u16>) -> CoreError {
    match code {
        Some(400) => invalid(),
        Some(403) => CoreError::Security {
            reason: SecurityFailureReason::AuthorizationDenied,
            context: context(),
        },
        Some(404) => CoreError::NotFound(context()),
        Some(410) => CoreError::StaleHandle(context()),
        Some(503) => CoreError::Backpressure(context()),
        _ => CoreError::Binding(context().with_redacted_cause(653, "HTTP Action response refused")),
    }
}

#[cfg(feature = "prepare")]
impl clinkz_wot_planning::open_consumer::ConsumerActionPreparation for HttpActionConsumer {
    fn identity(&self) -> BindingRegistrationIdentity {
        self.identity
    }
    fn bounds(&self) -> clinkz_wot_planning::open_consumer::ConsumerPreparationBounds {
        clinkz_wot_planning::open_consumer::ConsumerPreparationBounds {
            work: 4096,
            temporary_bytes: 8192,
            retained_bytes: 1024,
        }
    }
    fn compile_action(
        &self,
        input: clinkz_wot_planning::open_consumer::ConsumerActionPreparationInput<'_>,
    ) -> CoreResult<Option<Box<dyn ConsumerActionMapping>>> {
        let base = input
            .endpoint
            .strip_suffix("{?id}")
            .unwrap_or(input.endpoint);
        if Target::parse(base).is_err() {
            return Ok(None);
        }
        crate::action_preparation::qualify_action(
            input.document,
            input.name,
            input.form,
            input.operation,
            input.endpoint,
        )?;
        self.mapping(input.endpoint, input.operation).map(Some)
    }
}
