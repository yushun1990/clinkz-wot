//! Request-owned Hyper/rustls connection. No pool, tasks, redirects or retries.
//! Wire/header/body ceilings include chunk overhead and TLS handshake traffic.
use super::*;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper_util::rt::TokioIo;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "bounded Discovery HTTP")
}
struct Wire<S> {
    socket: S,
    remaining: usize,
}
impl<S: AsyncRead + Unpin> AsyncRead for Wire<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let n = self.remaining.min(4096).min(output.remaining());
        if n == 0 {
            return Poll::Ready(Err(invalid()));
        }
        let mut scratch = [0; 4096];
        let mut read = ReadBuf::new(&mut scratch[..n]);
        match Pin::new(&mut self.socket).poll_read(cx, &mut read) {
            Poll::Ready(Ok(())) => {
                self.remaining -= read.filled().len();
                output.put_slice(read.filled());
                Poll::Ready(Ok(()))
            }
            result => result,
        }
    }
}
impl<S: AsyncWrite + Unpin> AsyncWrite for Wire<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if !bytes.is_empty() && self.remaining == 0 {
            return Poll::Ready(Err(invalid()));
        }
        let n = bytes.len().min(4096).min(self.remaining);
        match Pin::new(&mut self.socket).poll_write(cx, &bytes[..n]) {
            Poll::Ready(Ok(n)) => {
                self.remaining -= n;
                Poll::Ready(Ok(n))
            }
            result => result,
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_shutdown(cx)
    }
}
// Preserve framing checks before Hyper can normalize duplicate framing fields,
// remove transfer-encoding or hide an informational response.
struct Head<S> {
    socket: S,
    bytes: [u8; 16_384],
    used: usize,
    done: bool,
}
impl<S> Head<S> {
    fn inspect(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.done {
            return Ok(());
        }
        let n = bytes.len().min(self.bytes.len() - self.used);
        self.bytes[self.used..self.used + n].copy_from_slice(&bytes[..n]);
        self.used += n;
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut head = httparse::Response::new(&mut headers);
        match head
            .parse(&self.bytes[..self.used])
            .map_err(|_| invalid())?
        {
            httparse::Status::Partial if self.used == self.bytes.len() => return Err(invalid()),
            httparse::Status::Partial => return Ok(()),
            httparse::Status::Complete(_) => {}
        }
        if head.version != Some(1) || head.code.is_none_or(|code| code < 200) {
            return Err(invalid());
        }
        let mut length = 0;
        let mut transfer = 0;
        for h in head.headers {
            if h.name.eq_ignore_ascii_case("content-length") {
                length += 1;
            }
            if h.name.eq_ignore_ascii_case("transfer-encoding") {
                transfer += 1;
                if !h.value.eq_ignore_ascii_case(b"chunked") {
                    return Err(invalid());
                }
            }
            if h.name.eq_ignore_ascii_case("trailer") || h.name.eq_ignore_ascii_case("upgrade") {
                return Err(invalid());
            }
        }
        if length > 1 || transfer > 1 || (length != 0 && transfer != 0) {
            return Err(invalid());
        }
        self.done = true;
        Ok(())
    }
}
impl<S: AsyncRead + Unpin> AsyncRead for Head<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let n = output.remaining().min(4096);
        if n == 0 {
            return Poll::Ready(Ok(()));
        }
        let mut scratch = [0; 4096];
        let mut read = ReadBuf::new(&mut scratch[..n]);
        match Pin::new(&mut self.socket).poll_read(cx, &mut read) {
            Poll::Ready(Ok(())) => {
                if let Err(e) = self.inspect(read.filled()) {
                    return Poll::Ready(Err(e));
                }
                output.put_slice(read.filled());
                Poll::Ready(Ok(()))
            }
            result => result,
        }
    }
}
impl<S: AsyncWrite + Unpin> AsyncWrite for Head<S> {
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
pub(super) async fn get(
    client: &Client,
    url: &str,
    directory: bool,
) -> Result<(Vec<u8>, hyper::HeaderMap)> {
    let uri = parse_url(url)?;
    if audience(&uri) != audience(&client.endpoint.uri) {
        return Err(E::DestinationDenied);
    }
    let socket = tokio::net::TcpStream::connect(client.endpoint.address)
        .await
        .map_err(|_| E::Transport)?;
    let wire = Wire {
        socket,
        remaining: client.limits.page_bytes * 2 + 65_536,
    };
    if let Some(tls) = &client.endpoint.transport.0 {
        let host = uri.host().unwrap().trim_matches(['[', ']']);
        let name = rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|_| E::Configuration)?;
        let socket = tokio_rustls::TlsConnector::from(tls.clone())
            .connect(name, wire)
            .await
            .map_err(|_| E::Trust)?;
        // Confidentiality and authenticated host precede credential encoding.
        send(socket, &uri, client, directory).await
    } else {
        if client.credentials.is_some() {
            return Err(E::DestinationDenied);
        }
        send(wire, &uri, client, directory).await
    }
}
async fn send<S: AsyncRead + AsyncWrite + Unpin>(
    socket: S,
    uri: &Uri,
    client: &Client,
    directory: bool,
) -> Result<(Vec<u8>, hyper::HeaderMap)> {
    let io = TokioIo::new(Head {
        socket,
        bytes: [0; 16_384],
        used: 0,
        done: false,
    });
    let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
        .max_headers(64)
        .max_buf_size(16_384)
        .writev(false)
        .handshake::<_, Empty<Bytes>>(io)
        .await
        .map_err(|_| E::Protocol)?;
    let mut request = hyper::Request::builder()
        .method("GET")
        .uri(uri.path_and_query().map_or("/", |v| v.as_str()))
        .header("host", uri.authority().unwrap().as_str())
        .header("connection", "close")
        .header(
            "accept",
            if directory {
                "application/ld+json, application/json"
            } else {
                "application/td+json, application/json"
            },
        )
        .header("accept-encoding", "identity");
    if let Some(credentials) = &client.credentials {
        request = request.header("authorization", credentials.header(uri)?);
    }
    let request = request.body(Empty::new()).map_err(|_| E::Configuration)?;
    let operation = async {
        let response = sender
            .send_request(request)
            .await
            .map_err(|_| E::Protocol)?;
        match response.status().as_u16() {
            200 => {}
            401 => return Err(E::Authentication),
            403 => return Err(E::Forbidden),
            status => return Err(E::HttpStatus(status)),
        }
        let headers = response.headers();
        let mut media = headers.get_all("content-type").iter();
        let value = media
            .next()
            .ok_or(E::UnsupportedMedia)?
            .to_str()
            .map_err(|_| E::UnsupportedMedia)?;
        if media.next().is_some() {
            return Err(E::UnsupportedMedia);
        }
        // JSON is UTF-8. A charset parameter can only reaffirm that encoding;
        // unknown/duplicate representation parameters refuse this profile.
        let mut parts = value.split(';');
        let mime = parts.next().unwrap().trim();
        if !mime.eq_ignore_ascii_case("application/json")
            && !mime.eq_ignore_ascii_case(if directory {
                "application/ld+json"
            } else {
                "application/td+json"
            })
        {
            return Err(E::UnsupportedMedia);
        }
        if let Some(parameter) = parts.next() {
            let (name, value) = parameter
                .trim()
                .split_once('=')
                .ok_or(E::UnsupportedMedia)?;
            if !name.trim().eq_ignore_ascii_case("charset")
                || !value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
                || parts.next().is_some()
            {
                return Err(E::UnsupportedMedia);
            }
        }
        if headers.contains_key("content-encoding") || headers.contains_key("trailer") {
            return Err(E::UnsupportedMedia);
        }
        if let Some(length) = headers.get("content-length") {
            let length: usize = length
                .to_str()
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or(E::Protocol)?;
            if length > client.limits.page_bytes {
                return Err(E::Limit);
            }
        }
        let headers = headers.clone();
        let mut body = response.into_body();
        let mut bytes = Vec::new();
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| E::Protocol)?;
            if frame.is_trailers() {
                return Err(E::Protocol);
            }
            if let Ok(data) = frame.into_data() {
                if data.len() > client.limits.page_bytes - bytes.len() {
                    return Err(E::Limit);
                }
                bytes.extend_from_slice(&data);
            }
        }
        Ok((bytes, headers))
    };
    tokio::pin!(connection);
    tokio::pin!(operation);
    // Both futures live in this request. Every return/drop releases the actual
    // socket, including error bodies not drained and partially delivered pages.
    tokio::select! {
        result = &mut operation => result,
        result = &mut connection => { result.map_err(|_| E::Protocol)?; operation.await }
    }
}
