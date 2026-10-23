//! One retained native HTTP request/reply owner. Hyper frames the exchange;
//! its service only marks a candidate and waits for a Core-sealed response.
use super::producer::HttpProducerProgram;
use bytes::Bytes;
use clinkz_wot_core::producer::*;
use http_body_util::Full;
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
    type Future = ReplyFuture;
    fn call(&self, _request: Request<Incoming>) -> Self::Future {
        // Raw head policy has already refused unsupported requests. Dropping
        // Incoming is safe because only a bodyless request can reach here.
        self.0.lock().unwrap().candidate = true;
        ReplyFuture(self.0.clone())
    }
}
struct Ingress {
    socket: TcpStream,
    bridge: Arc<Mutex<Bridge>>,
    head: [u8; 8192],
    len: usize,
    complete: bool,
    authority: [u8; 64],
    authority_len: usize,
    path: [u8; 64],
    path_len: usize,
}
fn invalid() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidData)
}
impl Ingress {
    fn inspect(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.complete {
            return if bytes.is_empty() {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if bytes.len() > self.head.len() - self.len {
            return Err(invalid());
        }
        self.head[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
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
        if end != self.len
            || request.version != Some(1)
            || request.method != Some("GET")
            || request.path.map(str::as_bytes) != Some(&self.path[..self.path_len])
        {
            return Err(invalid());
        }
        let mut host = 0;
        let mut length = 0;
        for h in request.headers.iter() {
            if h.name.eq_ignore_ascii_case("host") {
                host += 1;
                if h.value != &self.authority[..self.authority_len] {
                    return Err(invalid());
                }
            }
            if h.name.eq_ignore_ascii_case("content-length") {
                length += 1;
                if h.value.is_empty() || !h.value.iter().all(|b| *b == b'0') {
                    return Err(invalid());
                }
            }
            if [
                "transfer-encoding",
                "expect",
                "upgrade",
                "content-encoding",
                "trailer",
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
        if host != 1 || length > 1 {
            return Err(invalid());
        }
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
                self.bridge.lock().unwrap().write_left -= n;
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
        let bridge = Arc::new(Mutex::new(Bridge::default()));
        let mut ingress = Ingress {
            socket,
            bridge: bridge.clone(),
            head: [0; 8192],
            len: 0,
            complete: false,
            authority: [0; 64],
            authority_len: program.authority().len(),
            path: [0; 64],
            path_len: program.path_and_query().len(),
        };
        ingress.authority[..ingress.authority_len].copy_from_slice(program.authority().as_bytes());
        ingress.path[..ingress.path_len].copy_from_slice(program.path_and_query().as_bytes());
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
        if !self.offered && self.bridge.lock().unwrap().candidate {
            self.offered = true;
            true
        } else {
            false
        }
    }
    pub(super) fn reply(&mut self, response: &ProducerResponse) {
        if self.replied {
            return;
        }
        self.replied = true;
        // IgnoredAny checks syntax without building a JSON tree, but skips
        // UTF-8 checks in strings and object keys. Validate the bytes first.
        let body = response.result().ok().and_then(|o| o.data()).filter(|p| {
            p.content_type == "application/json"
                && p.content_coding.is_none()
                && p.body.len() <= 4096
                && std::str::from_utf8(&p.body)
                    .is_ok_and(|json| serde_json::from_str::<serde::de::IgnoredAny>(json).is_ok())
        });
        let (status, bytes) = match body {
            Some(p) => (200, Bytes::copy_from_slice(&p.body)),
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
    }
}
