//! One retained native HTTP request/reply owner. Hyper frames the exchange;
//! its service only marks a candidate and waits for the retained Engine result.
use super::producer::HttpProducerProgram;
use bytes::Bytes;
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, InteractionInput, InteractionOutput,
    Operation, Payload, RetryClass,
};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::Service};
use std::{
    convert::Infallible,
    future::Future,
    io::{self, Read, Write},
    net::TcpStream,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

type Reply = Response<Full<Bytes>>;
#[derive(Default)]
struct Bridge {
    candidate: bool,
    reply: Option<Reply>,
    read_left: usize,
    write_left: usize,
    wrote: bool,
    mapping: Option<usize>,
    operation: Option<Operation>,
    input: Option<InteractionInput>,
}
struct ReplyFuture(Arc<Mutex<Bridge>>);
impl Future for ReplyFuture {
    type Output = Result<Reply, Infallible>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.lock().unwrap().reply.take() {
            Some(reply) => Poll::Ready(Ok(reply)),
            None => Poll::Pending,
        }
    }
}
struct BridgeService(Arc<Mutex<Bridge>>);
impl Service<Request<Incoming>> for BridgeService {
    type Response = Reply;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Reply, Infallible>> + Send>>;
    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let bridge = self.0.clone();
        let write = request.method() == hyper::Method::PUT;
        Box::pin(async move {
            let mut body = request.into_body();
            let mut bytes = Vec::new();
            while let Some(frame) = body.frame().await {
                match frame.ok().and_then(|f| f.into_data().ok()) {
                    Some(data) if data.len() <= 4096 - bytes.len() => {
                        bytes.extend_from_slice(&data)
                    }
                    _ => {
                        return Ok(Response::builder()
                            .status(400)
                            .body(Full::new(Bytes::new()))
                            .unwrap());
                    }
                }
            }
            {
                let mut state = bridge.lock().unwrap();
                state.input = Some(if write {
                    InteractionInput::with_data(Payload::new(bytes, "application/json"))
                } else {
                    InteractionInput::empty()
                });
                state.candidate = true;
            }
            ReplyFuture(bridge).await
        })
    }
}
struct Ingress {
    socket: TcpStream,
    bridge: Arc<Mutex<Bridge>>,
    head: [u8; 8192],
    len: usize,
    complete: bool,
    programs: Vec<HttpProducerProgram>,
    body_length: usize,
    body_received: usize,
}
fn invalid() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidData)
}
impl Ingress {
    fn inspect(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.complete {
            self.body_received += bytes.len();
            return if self.body_received <= self.body_length {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        let copied = bytes.len().min(self.head.len() - self.len);
        self.head[self.len..self.len + copied].copy_from_slice(&bytes[..copied]);
        self.len += copied;
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut request = httparse::Request::new(&mut headers);
        let end = match request
            .parse(&self.head[..self.len])
            .map_err(|_| invalid())?
        {
            httparse::Status::Partial if self.len == self.head.len() => return Err(invalid()),
            httparse::Status::Partial => return Ok(()),
            httparse::Status::Complete(end) => end,
        };
        if request.version != Some(1) {
            return Err(invalid());
        }
        let mapping = self
            .programs
            .iter()
            .position(|p| {
                request.path == Some(p.path_and_query())
                    && request.method
                        == Some(if p.operation == Operation::WriteProperty {
                            "PUT"
                        } else {
                            "GET"
                        })
            })
            .ok_or_else(invalid)?;
        let program = &self.programs[mapping];
        let write = program.operation == Operation::WriteProperty;
        let mut host = 0;
        let mut length = 0;
        let mut media = 0;
        for h in request.headers.iter() {
            if h.name.eq_ignore_ascii_case("host") {
                host += 1;
                if h.value != program.authority().as_bytes() {
                    return Err(invalid());
                }
            }
            if h.name.eq_ignore_ascii_case("content-length") {
                length += 1;
                if h.value.is_empty() || !h.value.iter().all(u8::is_ascii_digit) {
                    return Err(invalid());
                }
                self.body_length = std::str::from_utf8(h.value)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(invalid)?;
                if self.body_length > if write { 4096 } else { 0 } {
                    return Err(invalid());
                }
            }
            if h.name.eq_ignore_ascii_case("content-type") {
                media += 1;
                if write && h.value != b"application/json" {
                    return Err(invalid());
                }
            }
            if [
                "transfer-encoding",
                "expect",
                "upgrade",
                "content-encoding",
                "trailer",
                "authorization",
                "proxy-authorization",
                "cookie",
            ]
            .iter()
            .any(|n| h.name.eq_ignore_ascii_case(n))
            {
                return Err(invalid());
            }
            if h.name.eq_ignore_ascii_case("connection") && !h.value.eq_ignore_ascii_case(b"close")
            {
                return Err(invalid());
            }
        }
        if host != 1
            || length > 1
            || media > 1
            || (write && (length != 1 || self.body_length == 0 || media != 1))
        {
            return Err(invalid());
        }
        self.body_received = self.len - end + bytes.len() - copied;
        if self.body_received > self.body_length {
            return Err(invalid());
        }
        let mut bridge = self.bridge.lock().unwrap();
        bridge.mapping = Some(mapping);
        bridge.operation = Some(program.operation);
        self.complete = true;
        Ok(())
    }
}
impl hyper::rt::Read for Ingress {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut output: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        let available = self
            .bridge
            .lock()
            .unwrap()
            .read_left
            .min(output.remaining())
            .min(4096);
        if available == 0 {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let mut bytes = [0; 4096];
        match self.socket.read(&mut bytes[..available]) {
            Ok(n) => {
                self.bridge.lock().unwrap().read_left -= n;
                self.inspect(&bytes[..n])?;
                output.put_slice(&bytes[..n]);
                Poll::Ready(Ok(()))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
impl hyper::rt::Write for Ingress {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let n = bytes.len().min(self.bridge.lock().unwrap().write_left);
        if n == 0 && !bytes.is_empty() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match self.socket.write(&bytes[..n]) {
            Ok(n) => {
                let mut bridge = self.bridge.lock().unwrap();
                bridge.write_left -= n;
                bridge.wrote |= n != 0;
                Poll::Ready(Ok(n))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub(super) struct NativeConnection {
    // This is the sole socket owner. No task, executor or external poll handle.
    driver: Pin<Box<http1::Connection<Ingress, BridgeService>>>,
    bridge: Arc<Mutex<Bridge>>,
    offered: bool,
    replied: bool,
}
impl NativeConnection {
    pub(super) fn new(socket: TcpStream, program: &HttpProducerProgram) -> Self {
        Self::from_programs(socket, vec![program.clone()])
    }
    pub(super) fn from_programs(socket: TcpStream, programs: Vec<HttpProducerProgram>) -> Self {
        let bridge = Arc::new(Mutex::new(Bridge::default()));
        let ingress = Ingress {
            socket,
            bridge: bridge.clone(),
            head: [0; 8192],
            len: 0,
            complete: false,
            programs,
            body_length: 0,
            body_received: 0,
        };
        let driver = Box::pin(
            http1::Builder::new()
                .keep_alive(false)
                .half_close(true)
                .max_headers(16)
                .max_buf_size(8192)
                .writev(false)
                .serve_connection(ingress, BridgeService(bridge.clone())),
        );
        Self {
            driver,
            bridge,
            offered: false,
            replied: false,
        }
    }
    pub(super) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), hyper::Error>> {
        {
            let mut b = self.bridge.lock().unwrap();
            b.read_left = 4096;
            b.write_left = 4096;
        }
        self.driver.as_mut().poll(cx)
    }
    pub(super) fn take_candidate(&mut self) -> bool {
        self.take_request().is_some()
    }
    pub(super) fn take_request(&mut self) -> Option<(usize, InteractionInput)> {
        if !self.offered && self.bridge.lock().unwrap().candidate {
            self.offered = true;
            let mut bridge = self.bridge.lock().unwrap();
            Some((bridge.mapping.unwrap(), bridge.input.take().unwrap()))
        } else {
            None
        }
    }
    pub(super) fn write_started(&self) -> bool {
        self.bridge.lock().unwrap().wrote
    }
    pub(super) fn reply(
        &mut self,
        result: Result<&InteractionOutput, &CoreError>,
    ) -> CoreResult<()> {
        if self.replied {
            return Ok(());
        }
        // IgnoredAny checks syntax without building a JSON tree, but skips
        // UTF-8 checks in strings and object keys. Validate the bytes first.
        let body = result.ok().and_then(|o| o.data()).filter(|p| {
            p.content_type == "application/json"
                && p.content_coding.is_none()
                && p.body.len() <= 4096
                && std::str::from_utf8(&p.body)
                    .is_ok_and(|json| serde_json::from_str::<serde::de::IgnoredAny>(json).is_ok())
        });
        let write_success = self.bridge.lock().unwrap().operation == Some(Operation::WriteProperty)
            && result.is_ok_and(|o| {
                clinkz_wot_core::finite::raw_finite_property_shape(Operation::WriteProperty, o)
            });
        if result.is_ok() && body.is_none() && !write_success {
            // Raw results still need an executable native encoding. Refuse
            // before queuing/writing anything; a replacement 500 would not be
            // delivery of this retained successful value. Its original owner
            // stays with Servient through native retirement and settlement.
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
                    .with_redacted_cause(628, "HTTP Producer result cannot be encoded"),
            ));
        }
        self.replied = true;
        let (status, bytes) = match body {
            Some(p) => (200, Bytes::copy_from_slice(&p.body)),
            None if write_success => (204, Bytes::new()),
            None => (500, Bytes::from_static(b"null")),
        };
        self.bridge.lock().unwrap().reply = Some(
            Response::builder()
                .status(status)
                .header("content-type", "application/json")
                .header("connection", "close")
                .body(Full::new(bytes))
                .expect("fixed HTTP response"),
        );
        Ok(())
    }
}
