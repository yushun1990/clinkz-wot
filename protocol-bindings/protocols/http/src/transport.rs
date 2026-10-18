//! Call-owned Hyper driver and bounded ingress. No HTTP framing implementation:
//! httparse inspects the raw head; Hyper owns request/response/body framing.
use super::*;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use std::{future::poll_fn, io};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};

pub(super) fn runtime() -> CoreResult<tokio::runtime::Handle> {
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|_| error(603, "entered Tokio I/O runtime required"))?;
    // Tokio has no public nonpanicking I/O-driver query. Register and immediately
    // drop one unconnected nonblocking socket. No bind/connect/read/write occurs.
    // This detects a runtime built without enable_io before accepting the call.
    let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)
        .map_err(|_| error(603, "runtime probe socket unavailable"))?;
    socket
        .set_nonblocking(true)
        .map_err(|_| error(603, "runtime probe unavailable"))?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        TcpStream::from_std(socket.into())
    }))
    .map_err(|_| error(603, "Tokio I/O driver required"))?
    .map_err(|_| error(603, "Tokio I/O registration failed"))?;
    Ok(handle)
}

struct Ingress {
    socket: TcpStream,
    head: [u8; 8192],
    head_len: usize,
    head_done: bool,
    wire: usize,
    allowance: Arc<AtomicUsize>,
}
fn forbidden_trailer(name: &str) -> bool {
    [
        "content-type",
        "content-length",
        "transfer-encoding",
        "content-encoding",
        "trailer",
        "upgrade",
        "connection",
        "host",
        "te",
    ]
    .iter()
    .any(|field| name.eq_ignore_ascii_case(field))
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "HTTP ingress rejected")
}
impl Ingress {
    fn inspect(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.wire = self.wire.checked_add(bytes.len()).ok_or_else(invalid)?;
        if self.wire > 32768 {
            return Err(invalid());
        }
        if self.head_done {
            return Ok(());
        }
        let n = bytes.len().min(self.head.len() - self.head_len);
        self.head[self.head_len..self.head_len + n].copy_from_slice(&bytes[..n]);
        self.head_len += n;
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut response = httparse::Response::new(&mut headers);
        match response
            .parse(&self.head[..self.head_len])
            .map_err(|_| invalid())?
        {
            httparse::Status::Partial => {
                if self.head_len == self.head.len() {
                    return Err(invalid());
                }
            }
            httparse::Status::Complete(_) => {
                if response.version != Some(1) || response.code != Some(200) {
                    return Err(invalid());
                }
                let mut media = 0;
                let mut transfer = 0;
                let mut lengths = 0;
                let mut declares_trailers = false;
                for header in response.headers.iter() {
                    if ["content-encoding", "upgrade"]
                        .iter()
                        .any(|name| header.name.eq_ignore_ascii_case(name))
                    {
                        return Err(invalid());
                    }
                    if header.name.eq_ignore_ascii_case("transfer-encoding") {
                        transfer += 1;
                        if !header.value.eq_ignore_ascii_case(b"chunked") {
                            return Err(invalid());
                        }
                    }
                    if header.name.eq_ignore_ascii_case("trailer") {
                        declares_trailers = true;
                        for field in header.value.split(|b| *b == b',') {
                            let field = std::str::from_utf8(field).map_err(|_| invalid())?.trim();
                            if hyper::header::HeaderName::from_bytes(field.as_bytes()).is_err()
                                || forbidden_trailer(field)
                            {
                                return Err(invalid());
                            }
                        }
                    }
                    if header.name.eq_ignore_ascii_case("content-type") {
                        media += 1;
                        if header.value != b"application/json" {
                            return Err(invalid());
                        }
                    }
                    if header.name.eq_ignore_ascii_case("content-length") {
                        lengths += 1;
                        let length = std::str::from_utf8(header.value)
                            .ok()
                            .and_then(|v| v.parse::<u64>().ok())
                            .ok_or_else(invalid)?;
                        if length > BODY {
                            return Err(invalid());
                        }
                    }
                }
                if media != 1
                    || transfer > 1
                    || lengths > 1
                    || (transfer != 0 && lengths != 0)
                    || (declares_trailers && transfer == 0)
                {
                    return Err(invalid());
                }
                self.head_done = true;
            }
        }
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
            .allowance
            .load(Ordering::Relaxed)
            .min(4096)
            .min(output.remaining())
            .min(32768 - self.wire);
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if self.wire == 32768 {
            return Poll::Ready(Err(invalid()));
        }
        if available == 0 {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let mut scratch = [0; 4096];
        let mut read = ReadBuf::new(&mut scratch[..available]);
        match Pin::new(&mut self.socket).poll_read(cx, &mut read) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                let bytes = read.filled();
                self.allowance.fetch_sub(bytes.len(), Ordering::Relaxed);
                self.inspect(bytes)?;
                output.put_slice(bytes);
                Poll::Ready(Ok(()))
            }
        }
    }
}
impl hyper::rt::Write for Ingress {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.socket).poll_write(cx, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_shutdown(cx)
    }
}

pub(super) async fn exchange(target: Target, allowance: Arc<AtomicUsize>) -> CoreResult<Vec<u8>> {
    let socket = TcpStream::connect(target.address)
        .await
        .map_err(|_| error(604, "HTTP connect failed"))?;
    let io = Ingress {
        socket,
        head: [0; 8192],
        head_len: 0,
        head_done: false,
        wire: 0,
        allowance,
    };
    let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
        .max_headers(32)
        .max_header_size(8192)
        .chunked_limits(16, 2048, 32768)
        .max_buf_size(8192)
        .writev(false)
        .handshake::<_, Empty<Bytes>>(io)
        .await
        .map_err(|_| error(605, "HTTP handshake failed"))?;
    let request = hyper::Request::builder()
        .method("GET")
        .uri(target.uri.path_and_query().map_or("/", |v| v.as_str()))
        .header("host", target.uri.authority().unwrap().as_str())
        .header("accept", "application/json")
        .header("connection", "close, te")
        .header("te", "trailers")
        .body(Empty::<Bytes>::new())
        .map_err(|_| error(605, "HTTP request failed"))?;
    let operation = async {
        let response = sender
            .send_request(request)
            .await
            .map_err(|_| error(606, "HTTP response rejected"))?;
        // Defense in depth; the raw-head guard rejects before Hyper can hide
        // informational messages, normalize whitespace or remove framing fields.
        if response.status() != hyper::StatusCode::OK {
            return Err(error(606, "HTTP status rejected"));
        }
        let mut body = response.into_body();
        let mut result = Vec::new();
        result.try_reserve_exact(BODY as usize).map_err(|_| {
            // The captured protocol ceiling permits this reservation;
            // refusal belongs to the application's allocator/deployment.
            CoreError::Binding(
                ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
                    .with_resource_failure(
                        BindingConfigurationDigest::new(PRIVATE),
                        ResourceFailureDomain::DeploymentFailure,
                        None,
                        None,
                    )
                    .with_redacted_cause(607, "HTTP body backing unavailable"),
            )
        })?;
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| error(606, "HTTP body rejected"))?;
            match frame.into_data() {
                Ok(data) => {
                    if data.len() > BODY as usize - result.len() {
                        return Err(error(608, "HTTP body limit exceeded"));
                    }
                    result.extend_from_slice(&data);
                }
                Err(frame) => {
                    let trailers = frame
                        .into_trailers()
                        .map_err(|_| error(606, "HTTP frame rejected"))?;
                    if trailers.keys().any(|name| forbidden_trailer(name.as_str())) {
                        return Err(error(606, "HTTP trailer field rejected"));
                    }
                    // Trailer metadata never changes the primary media, status
                    // or Core result. The SDK already checked raw ingress limits.
                }
            }
        }
        Ok(result)
    };
    let mut operation = std::pin::pin!(operation);
    let mut connection = std::pin::pin!(connection);
    let mut driver_done = false;
    // The connection driver is polled here, never spawned or transferred.
    // Returning or dropping this future destroys the socket before settlement.
    poll_fn(|cx| {
        let mut driver_error = false;
        if !driver_done {
            match connection.as_mut().poll(cx) {
                Poll::Ready(result) => {
                    driver_done = true;
                    driver_error = result.is_err();
                }
                Poll::Pending => {}
            }
        }
        match operation.as_mut().poll(cx) {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending if driver_done => Poll::Ready(Err(error(
                606,
                if driver_error {
                    "HTTP connection failed"
                } else {
                    "HTTP connection ended"
                },
            ))),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::rt::{Read, ReadBuf as HyperReadBuf};
    use std::{io::Write, task::Waker};

    #[test]
    fn real_socket_ingress_shares_one_callback_credit_and_never_refills_wire() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n".to_vec();
            bytes.resize(32769, b'1');
            socket.write_all(&bytes).unwrap();
        });
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(async {
                let socket = TcpStream::connect(address).await.unwrap();
                let allowance = Arc::new(AtomicUsize::new(0));
                let mut ingress = Ingress {
                    socket,
                    head: [0; 8192],
                    head_len: 0,
                    head_done: false,
                    wire: 0,
                    allowance: allowance.clone(),
                };
                let mut storage = [0; 16384];
                let mut cx = Context::from_waker(Waker::noop());
                assert!(
                    Pin::new(&mut ingress)
                        .poll_read(&mut cx, HyperReadBuf::new(&mut storage).unfilled())
                        .is_pending()
                );
                assert_eq!(ingress.wire, 0);
                for callback in 1..=8 {
                    allowance.store(4096, Ordering::Relaxed);
                    let mut received = 0;
                    while allowance.load(Ordering::Relaxed) != 0 {
                        // Vary requested capacity so multiple reads must share
                        // the allowance, independently of Hyper's read strategy.
                        let capacity = if received == 0 { 17 } else { storage.len() };
                        let n = poll_fn(|cx| {
                            let mut output = HyperReadBuf::new(&mut storage[..capacity]);
                            match Pin::new(&mut ingress).poll_read(cx, output.unfilled()) {
                                Poll::Ready(Ok(())) => Poll::Ready(output.filled().len()),
                                Poll::Ready(Err(e)) => panic!("unexpected ingress error: {e}"),
                                Poll::Pending => Poll::Pending,
                            }
                        })
                        .await;
                        assert!((1..=4096).contains(&n));
                        received += n;
                    }
                    assert_eq!(received, 4096);
                    assert_eq!(ingress.wire, callback * 4096);
                    if callback != 8 {
                        assert!(
                            Pin::new(&mut ingress)
                                .poll_read(&mut cx, HyperReadBuf::new(&mut storage).unfilled())
                                .is_pending()
                        );
                    }
                }
                allowance.store(4096, Ordering::Relaxed);
                assert!(matches!(
                    Pin::new(&mut ingress)
                        .poll_read(&mut cx, HyperReadBuf::new(&mut storage).unfilled()),
                    Poll::Ready(Err(_))
                ));
                assert_eq!(ingress.wire, 32768);
                assert_eq!(allowance.load(Ordering::Relaxed), 4096);
                // The byte beyond the lifetime wire ceiling stayed in the OS;
                // it was never received into parser/body backing.
                let mut byte = [0];
                let n = poll_fn(|cx| {
                    let mut output = ReadBuf::new(&mut byte);
                    Pin::new(&mut ingress.socket)
                        .poll_read(cx, &mut output)
                        .map_ok(|()| output.filled().len())
                })
                .await
                .unwrap();
                assert_eq!(n, 1);
            });
        peer.join().unwrap();
    }
}
