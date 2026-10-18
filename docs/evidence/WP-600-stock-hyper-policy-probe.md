# WP-600 stock Hyper parser-policy counterexamples

This is executable SDK evidence for the [ADR-0013 admission amendment](../work-packages/WP-600-http-property-read-target.md#http-parser-policy-amendment),
not a second binding, production qualification or an SDK memory bound.
[Independent review 5562587383](https://github.com/yushun1990/clinkz-wot/pull/197#pullrequestreview-5562587383)
demonstrated the alternative. These small probes make it reproducible without
PR #197's private SDK or production changes. The completion evidence TOML owns
execution results and source identities.

Run from the repository root. This extracts the two code blocks below into a
temporary standalone crate, resolves its pinned upstream dependencies offline,
then tests the resulting lock. No repository file is generated or modified.

```sh
WP600_POLICY_PROBE_DIR=$(mktemp -d /tmp/wp600-stock-hyper.XXXXXX)
python3 - "$WP600_POLICY_PROBE_DIR" <<'PY'
from pathlib import Path
import re, sys
source = Path("docs/evidence/WP-600-stock-hyper-policy-probe.md").read_text()
target = Path(sys.argv[1])
for language, name in [("toml", "Cargo.toml"), ("rust", "policy.rs")]:
    block, = re.findall(r"^```" + language + r"\n(.*?)^```$", source, re.M | re.S)
    (target / name).write_text(block)
print(target)
PY
cargo generate-lockfile --offline --manifest-path "$WP600_POLICY_PROBE_DIR/Cargo.toml"
cargo test --locked --offline --manifest-path "$WP600_POLICY_PROBE_DIR/Cargo.toml" -- --test-threads=1 --nocapture
```

```toml
[package]
name = "wp600-stock-hyper-policy-probe"
version = "0.0.0"
edition = "2024"
publish = false

[[test]]
name = "policy"
path = "policy.rs"

[dependencies]
hyper = { version = "=1.12.0", default-features = false, features = ["client", "http1"] }
hyper-util = { version = "=0.1.20", default-features = false, features = ["tokio"] }
http-body-util = { version = "=0.1.3", default-features = false }
bytes = { version = "=1.11.1", default-features = false }
tokio = { version = "=1.52.3", default-features = false, features = ["net", "rt", "time"] }
```

All peers keep their write side open: rejection/success must precede peer EOF.
The tests use real I/O wakes and a failure timeout, and check local socket
retirement. Trailer lengths are **raw** lengths including the final empty line;
duplicate field lines count separately. The SDK may buffer the rejecting byte
and read ahead. Neither accepted lengths nor `max_buf_size` prove allocation
capacity, transient overlap, continuation stack or disposal bounds.

```rust
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper_util::rt::TokioIo;
use std::{
    future::{Future, poll_fn},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    task::Poll,
    time::Duration,
};

fn decode(bytes: Vec<u8>, fragment: usize) -> Result<(Vec<u8>, usize), ()> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            assert_eq!(socket.read(&mut byte).unwrap(), 1);
            request.push(byte[0]);
        }
        for part in bytes.chunks(fragment) {
            if socket.write_all(part).is_err() {
                break; // A client may already have rejected earlier bytes.
            }
        }
        // No shutdown(Write): a pending decoder cannot borrow peer EOF.
        let mut byte = [0];
        match socket.read(&mut byte) {
            Ok(0) => {}
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
            other => panic!("client did not retire locally: {other:?}"),
        }
    });
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_io().enable_time().build().unwrap().block_on(async {
            let socket = tokio::net::TcpStream::from_std({
                let s = TcpStream::connect(address).unwrap();
                s.set_nonblocking(true).unwrap();
                s
            }).unwrap();
            let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
                .max_headers(32).max_header_size(8192).max_buf_size(8192)
                .writev(false).handshake::<_, Empty<Bytes>>(TokioIo::new(socket))
                .await.unwrap();
            let operation = async {
                let request = hyper::Request::builder()
                    .header("host", "127.0.0.1").header("connection", "close")
                    .body(Empty::new()).unwrap();
                let response = sender.send_request(request).await.map_err(|_| ())?;
                let mut body = response.into_body();
                let mut data = Vec::new();
                let mut fields = 0;
                while let Some(frame) = body.frame().await {
                    let frame = frame.map_err(|_| ())?;
                    match frame.into_data() {
                        Ok(bytes) => data.extend_from_slice(&bytes),
                        Err(frame) => fields += frame.into_trailers().unwrap().len(),
                    }
                }
                Ok((data, fields))
            };
            let mut operation = std::pin::pin!(operation);
            let mut connection = std::pin::pin!(connection);
            let mut ended = false;
            tokio::time::timeout(Duration::from_secs(2), poll_fn(|cx| {
                if !ended && connection.as_mut().poll(cx).is_ready() {
                    ended = true;
                }
                match operation.as_mut().poll(cx) {
                    Poll::Pending if ended => Poll::Ready(Err(())),
                    result => result,
                }
            })).await.expect("decoder waited for EOF/cancellation")
        });
    peer.join().unwrap();
    result
}

fn head(maximal: bool) -> Vec<u8> {
    let mut bytes = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n".to_vec();
    if maximal {
        bytes.extend_from_slice("X: y\r\n".repeat(29).as_bytes());
        bytes.extend_from_slice(b"Pad: ");
        bytes.resize(8192 - 4, b'x');
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"\r\n");
    if maximal { assert_eq!(bytes.len(), 8192); } // 32 field lines.
    bytes
}

fn response(trailers: &[u8], maximal: bool) -> Vec<u8> {
    let mut bytes = head(maximal);
    bytes.extend_from_slice(b"1000\r\n");
    bytes.extend(std::iter::repeat_n(b'1', 4096));
    bytes.extend_from_slice(b"\r\n0\r\n");
    bytes.extend_from_slice(trailers);
    assert!(bytes.len() < 32768);
    bytes
}

fn trailers(size: usize) -> Vec<u8> {
    let mut bytes = "X: y\r\n".repeat(31).into_bytes();
    bytes.extend_from_slice(b"Pad: z");
    bytes.resize(size - 4, b' ');
    bytes.extend_from_slice(b"\r\n\r\n");
    assert_eq!(bytes.len(), size);
    bytes
}

#[test]
fn stock_envelope_is_bounded_without_independent_trailer_settings() {
    for fragment in [usize::MAX, 7] {
        let (data, fields) = decode(response(&trailers(8191), true), fragment).unwrap();
        assert_eq!(data, vec![b'1'; 4096]);
        assert_eq!(fields, 32);
        assert!(decode(response(&trailers(8192), true), fragment).is_err());
        let mut unfinished = b"X: ".to_vec();
        unfinished.resize(8192, b' '); // No field/section terminator or EOF.
        assert!(decode(response(&unfinished, false), fragment).is_err());
        for count in [32, 33] {
            let complete = format!("{}\r\n", "X: y\r\n".repeat(count));
            assert_eq!(decode(response(complete.as_bytes(), false), fragment).is_ok(), count == 32);
        }
        let unfinished = "X: y\r\n".repeat(33);
        assert!(decode(response(unfinished.as_bytes(), false), fragment).is_err());
    }
}

#[test]
fn stock_extensions_have_a_separate_cumulative_ceiling_including_zero_chunk() {
    for total in [16383, 16384] {
        let mut bytes = head(false);
        bytes.extend_from_slice(b"1;");
        bytes.extend(std::iter::repeat_n(b'x', 8191));
        bytes.extend_from_slice(b"\r\n1\r\n0;");
        bytes.extend(std::iter::repeat_n(b'x', total - 8191));
        bytes.extend_from_slice(b"\r\n\r\n");
        assert!(bytes.len() < 32768);
        assert_eq!(decode(bytes, 7).is_ok(), total == 16383);
    }
}
```

Also retain and run the two committed original-policy counterexamples at
`protocol-bindings/protocols/http/tests/sdk_limits.rs`:

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test sdk_limits -- --test-threads=1 --nocapture
```

Those prove why independent inclusive 16/2048 trailers cannot be obtained from
the selected stock API while retaining 32/8192 main headers, and why the native
extension restriction can reject before the total-wire ceiling. Their historical
names do not make the superseded policy a successor acceptance criterion.
