//! Call-owned Hyper driver and bounded ingress. No HTTP framing implementation:
//! httparse inspects the raw head; Hyper owns request/response/body framing.
use super::*;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper_util::rt::TokioIo;
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
                for header in response.headers.iter() {
                    if [
                        "transfer-encoding",
                        "content-encoding",
                        "trailer",
                        "upgrade",
                    ]
                    .iter()
                    .any(|name| header.name.eq_ignore_ascii_case(name))
                    {
                        return Err(invalid());
                    }
                    if header.name.eq_ignore_ascii_case("content-type") {
                        media += 1;
                        if header.value != b"application/json" {
                            return Err(invalid());
                        }
                    }
                    if header.name.eq_ignore_ascii_case("content-length") {
                        let length = std::str::from_utf8(header.value)
                            .ok()
                            .and_then(|v| v.parse::<u64>().ok())
                            .ok_or_else(invalid)?;
                        if length > BODY {
                            return Err(invalid());
                        }
                    }
                }
                if media != 1 {
                    return Err(invalid());
                }
                self.head_done = true;
            }
        }
        Ok(())
    }
}
impl AsyncRead for Ingress {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
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
impl AsyncWrite for Ingress {
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
    let io = TokioIo::new(Ingress {
        socket,
        head: [0; 8192],
        head_len: 0,
        head_done: false,
        wire: 0,
        allowance,
    });
    let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
        .max_headers(32)
        .max_header_size(8192)
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
        .header("connection", "close")
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
        result
            .try_reserve_exact(BODY as usize)
            .map_err(|_| error(607, "HTTP body backing unavailable"))?;
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| error(606, "HTTP body rejected"))?;
            let data = frame
                .into_data()
                .map_err(|_| error(606, "HTTP trailers rejected"))?;
            if data.len() > BODY as usize - result.len() {
                return Err(error(608, "HTTP body limit exceeded"));
            }
            result.extend_from_slice(&data);
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
