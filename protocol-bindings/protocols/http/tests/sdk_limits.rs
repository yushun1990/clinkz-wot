#![cfg(feature = "prepare")]
//! Minimal pinned-SDK counterexamples, deliberately separate from production
//! acceptance tests in real_peer. No alternate parser or binding is supplied.
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::HeaderMap;
use hyper_util::rt::TokioIo;
use std::{
    future::{Future, poll_fn},
    io::{Read, Write},
    net::TcpListener,
    task::Poll,
    time::Duration,
};

fn decode(bytes: Vec<u8>, fields: usize, size: usize) -> Result<HeaderMap, ()> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            assert_eq!(socket.read(&mut byte).unwrap(), 1);
            request.push(byte[0]);
        }
        let _ = socket.write_all(&bytes);
        socket.shutdown(std::net::Shutdown::Write).unwrap();
    });
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap()
        .block_on(async {
            let socket = tokio::net::TcpStream::connect(address).await.unwrap();
            let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
                .max_headers(fields)
                .max_header_size(size)
                .max_buf_size(8192)
                .writev(false)
                .handshake::<_, Empty<Bytes>>(TokioIo::new(socket))
                .await
                .unwrap();
            let operation = async {
                let request = hyper::Request::builder()
                    .header("host", "127.0.0.1")
                    .header("connection", "close")
                    .body(Empty::new())
                    .unwrap();
                let response = sender.send_request(request).await.map_err(|_| ())?;
                let mut body = response.into_body();
                let mut trailers = HeaderMap::new();
                while let Some(frame) = body.frame().await {
                    if let Ok(map) = frame.map_err(|_| ())?.into_trailers() {
                        trailers = map;
                    }
                }
                Ok(trailers)
            };
            let mut operation = std::pin::pin!(operation);
            let mut connection = std::pin::pin!(connection);
            let mut ended = false;
            poll_fn(|cx| {
                if !ended && connection.as_mut().poll(cx).is_ready() {
                    ended = true;
                }
                match operation.as_mut().poll(cx) {
                    Poll::Pending if ended => Poll::Ready(Err(())),
                    result => result,
                }
            })
            .await
        });
    peer.join().unwrap();
    result
}

fn response(head_size: Option<usize>, trailers: &[u8]) -> Vec<u8> {
    let mut bytes =
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n"
            .to_vec();
    if let Some(size) = head_size {
        bytes.extend_from_slice(b"X: ");
        bytes.resize(size - 4, b'x');
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"\r\n1\r\n1\r\n0\r\n");
    bytes.extend_from_slice(trailers);
    bytes
}

fn padded_trailer(size: usize) -> Vec<u8> {
    let mut bytes = b"X: y".to_vec();
    bytes.resize(size - 4, b' ');
    bytes.extend_from_slice(b"\r\n\r\n");
    assert_eq!(bytes.len(), size);
    bytes
}

#[test]
fn header_configuration_cannot_enforce_independent_trailer_limits() {
    // Current production SDK configuration accepts both sides of the admitted
    // trailer-byte boundary and erases the distinguishing wire whitespace.
    let exact = decode(response(Some(8192), &padded_trailer(2048)), 32, 8192).unwrap();
    let over = decode(response(Some(8192), &padded_trailer(2049)), 32, 8192).unwrap();
    assert_eq!(exact, over);
    assert_eq!(exact["x"], "y");
    for count in [16, 17] {
        let trailers = format!("{}\r\n", "X: y\r\n".repeat(count));
        let map = decode(response(None, trailers.as_bytes()), 32, 8192).unwrap();
        assert_eq!(map.len(), count, "duplicate fields must still count");
    }

    // Size is exclusive in Hyper's trailer decoder. Raising it by one fixes
    // that boundary only, while also refusing admissible main response heads.
    assert!(decode(response(None, &padded_trailer(2047)), 32, 2048).is_ok());
    assert!(decode(response(None, &padded_trailer(2048)), 32, 2048).is_err());
    assert!(decode(response(None, &padded_trailer(2048)), 32, 2049).is_ok());
    assert!(decode(response(None, &padded_trailer(2049)), 32, 2049).is_err());
    assert!(decode(response(Some(8192), b"\r\n"), 32, 2049).is_err());

    let mut head = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n".to_vec();
    head.extend_from_slice("X: y\r\n".repeat(31).as_bytes());
    head.extend_from_slice(b"\r\n0\r\n\r\n");
    assert!(decode(head.clone(), 32, 8192).is_ok());
    assert!(decode(head, 16, 8192).is_err());
}

#[test]
fn sdk_chunk_extension_ceiling_precedes_the_admitted_total_wire_limit() {
    for count in [16383, 16384] {
        let mut bytes = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;".to_vec();
        bytes.extend(std::iter::repeat_n(b'x', count));
        bytes.extend_from_slice(b"\r\n1\r\n0\r\n\r\n");
        assert!(bytes.len() < 32768);
        assert_eq!(decode(bytes, 32, 8192).is_ok(), count < 16384);
    }
}
