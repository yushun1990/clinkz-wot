//! Public upstream API discriminators, not an engine adapter or a network bound.
use bytes::Bytes;
use http_body_util::Empty;
use hyper::body::Body;
use hyper::{Request, client::conn::http1};
use hyper_util::rt::TokioIo;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

struct Meter;
thread_local! {
    static OBSERVING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}
fn allocation(size: usize) {
    if OBSERVING.try_with(Cell::get).unwrap_or(false) {
        ALLOCATIONS.with(|n| n.set(n.get() + 1));
        BYTES.with(|n| n.set(n.get() + size));
    }
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocation(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocation(size);
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static METER: Meter = Meter;

fn observed(f: impl FnOnce()) -> (usize, usize) {
    ALLOCATIONS.with(|n| n.set(0));
    BYTES.with(|n| n.set(0));
    OBSERVING.with(|n| n.set(true));
    f();
    OBSERVING.with(|n| n.set(false));
    (ALLOCATIONS.with(Cell::get), BYTES.with(Cell::get))
}

#[derive(Default)]
struct IoEvidence {
    written: AtomicUsize,
    destroyed: AtomicUsize,
}

/// A silent peer after the fixed response prefix. No tasks, timers or sockets.
/// It makes each suspended HTTP phase deterministic without network timing.
struct ScriptIo {
    response: &'static [u8],
    offset: usize,
    evidence: Arc<IoEvidence>,
}
impl Drop for ScriptIo {
    fn drop(&mut self) {
        self.evidence.destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
impl AsyncRead for ScriptIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.evidence.written.load(Ordering::SeqCst) == 0 || self.offset == self.response.len() {
            return Poll::Pending;
        }
        let n = buf.remaining().min(self.response.len() - self.offset);
        buf.put_slice(&self.response[self.offset..self.offset + n]);
        self.offset += n;
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for ScriptIo {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.evidence.written.fetch_add(buf.len(), Ordering::SeqCst);
        Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

type Connection = http1::Connection<TokioIo<ScriptIo>, Empty<Bytes>>;
fn connection(
    response: &'static [u8],
) -> (
    http1::SendRequest<Empty<Bytes>>,
    Connection,
    Arc<IoEvidence>,
) {
    let evidence = Arc::new(IoEvidence::default());
    let io = ScriptIo {
        response,
        offset: 0,
        evidence: evidence.clone(),
    };
    let mut builder = http1::Builder::new();
    builder
        .max_headers(16)
        .max_header_size(4096)
        .max_buf_size(8192)
        .writev(false);
    let mut handshake = Box::pin(builder.handshake(TokioIo::new(io)));
    let Poll::Ready(Ok((sender, conn))) = handshake.as_mut().poll(&mut context()) else {
        panic!("HTTP/1 handshake is local construction");
    };
    (sender, conn, evidence)
}
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn request() -> Request<Empty<Bytes>> {
    Request::builder()
        .uri("/value")
        .header("Host", "127.0.0.1")
        .body(Empty::new())
        .unwrap()
}
fn expect_discard(label: &str, counts: (usize, usize), evidence: &IoEvidence, allocating: bool) {
    println!(
        "{label}: destructor allocator requests={}, requested bytes={}, IO destroyed={}",
        counts.0,
        counts.1,
        evidence.destroyed.load(Ordering::SeqCst)
    );
    assert_eq!(evidence.destroyed.load(Ordering::SeqCst), 1);
    if allocating {
        assert!(
            counts.0 > 0,
            "the upstream counterexample changed; reassess admission"
        );
    } else {
        assert_eq!(counts, (0, 0));
    }
}
fn patched() -> bool {
    std::env::var_os("HTTP_DISPOSAL_PATCH").is_some()
}

fn accept_with_deadline(listener: &std::net::TcpListener) -> std::net::TcpStream {
    use std::time::{Duration, Instant};
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "fixture peer accept timed out");
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => panic!("fixture peer accept failed: {e}"),
        }
    }
}

#[test]
fn idle_connection_discard_is_allocation_free() {
    let (sender, conn, evidence) = connection(b"");
    let counts = observed(|| {
        drop(sender);
        drop(conn);
    });
    expect_discard("idle", counts, &evidence, false);
}

#[test]
fn queued_request_allocates_even_after_response_interest_is_dropped() {
    let (mut sender, conn, evidence) = connection(b"");
    let response = sender.send_request(request());
    let counts = observed(|| {
        drop(response);
        drop(sender);
        drop(conn);
    });
    expect_discard("queued, interest first", counts, &evidence, !patched());
}

#[test]
fn pending_response_discard_allocates_in_both_owner_orders() {
    for interest_first in [true, false] {
        let (mut sender, conn, evidence) = connection(b"");
        let response = sender.send_request(request());
        let mut conn = Box::pin(conn);
        assert!(conn.as_mut().poll(&mut context()).is_pending());
        assert!(evidence.written.load(Ordering::SeqCst) > 0);
        let counts = observed(|| {
            if interest_first {
                drop(response);
                drop(sender);
                drop(conn);
            } else {
                drop(conn);
                drop(sender);
                drop(response);
            }
        });
        expect_discard(
            if interest_first {
                "pending, interest first"
            } else {
                "pending, connection first"
            },
            counts,
            &evidence,
            !patched() || !interest_first,
        );
    }
}

#[test]
fn incomplete_body_discard_allocates_in_both_owner_orders() {
    for interest_first in [true, false] {
        let (mut sender, conn, evidence) =
            connection(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nx");
        let mut response = Box::pin(sender.send_request(request()));
        let mut conn = Box::pin(conn);
        assert!(conn.as_mut().poll(&mut context()).is_pending());
        assert!(conn.as_mut().poll(&mut context()).is_pending());
        let Poll::Ready(Ok(response)) = response.as_mut().poll(&mut context()) else {
            panic!("head should be delivered");
        };
        let counts = observed(|| {
            if interest_first {
                drop(response);
                drop(sender);
                drop(conn);
            } else {
                drop(conn);
                drop(sender);
                drop(response);
            }
        });
        expect_discard(
            if interest_first {
                "body, interest first"
            } else {
                "body, connection first"
            },
            counts,
            &evidence,
            !patched() || !interest_first,
        );
    }
}

#[test]
fn completed_empty_response_discard_is_allocation_free() {
    let (mut sender, conn, evidence) =
        connection(b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
    let mut response = Box::pin(sender.send_request(request()));
    let mut conn = Box::pin(conn);
    assert!(conn.as_mut().poll(&mut context()).is_pending());
    assert!(conn.as_mut().poll(&mut context()).is_ready());
    let Poll::Ready(Ok(response)) = response.as_mut().poll(&mut context()) else {
        panic!("response missing");
    };
    let counts = observed(|| {
        drop(response);
        drop(sender);
        drop(conn);
    });
    expect_discard("completed", counts, &evidence, false);
}

#[test]
fn retry_callback_and_queued_envelope_obey_the_same_discard_order() {
    for dispatched in [false, true] {
        let (mut sender, conn, evidence) = connection(b"");
        let response = sender.try_send_request(request());
        let mut conn = Box::pin(conn);
        if dispatched {
            assert!(conn.as_mut().poll(&mut context()).is_pending());
            assert!(evidence.written.load(Ordering::SeqCst) > 0);
        }
        let counts = observed(|| {
            drop(response);
            drop(sender);
            drop(conn);
        });
        expect_discard(
            if dispatched {
                "retry pending"
            } else {
                "retry queued"
            },
            counts,
            &evidence,
            !patched(),
        );
    }
}

#[test]
fn dropping_connection_still_reports_errors_to_live_response_observers() {
    for dispatched in [false, true] {
        let (mut sender, conn, evidence) = connection(b"");
        let mut response = Box::pin(sender.send_request(request()));
        let mut conn = Box::pin(conn);
        if dispatched {
            assert!(conn.as_mut().poll(&mut context()).is_pending());
        }
        let counts = observed(|| drop(conn));
        expect_discard("live response observer", counts, &evidence, true);
        let Poll::Ready(Err(error)) = response.as_mut().poll(&mut context()) else {
            panic!("observer must receive its error");
        };
        if dispatched {
            assert!(error.is_user());
        } else {
            assert!(error.is_canceled());
        }
        drop(sender);
    }
}

#[test]
fn dropping_connection_still_reports_incomplete_body_to_live_observer() {
    let (mut sender, conn, evidence) =
        connection(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nx");
    let mut response = Box::pin(sender.send_request(request()));
    let mut conn = Box::pin(conn);
    assert!(conn.as_mut().poll(&mut context()).is_pending());
    assert!(conn.as_mut().poll(&mut context()).is_pending());
    let Poll::Ready(Ok(response)) = response.as_mut().poll(&mut context()) else {
        panic!("head missing");
    };
    let mut body = response.into_body();
    let counts = observed(|| drop(conn));
    expect_discard("live body observer", counts, &evidence, true);
    // Already accepted data must precede the terminal error.
    let first = Pin::new(&mut body).poll_frame(&mut context());
    let terminal = match first {
        Poll::Ready(Some(Ok(frame))) => {
            assert_eq!(frame.into_data().unwrap(), b"x"[..]);
            Pin::new(&mut body).poll_frame(&mut context())
        }
        other => other,
    };
    let Poll::Ready(Some(Err(error))) = terminal else {
        panic!("body must report incomplete, not clean EOF");
    };
    assert!(error.is_incomplete_message());
    drop(sender);
}

#[test]
fn discard_with_a_delivered_body_frame_releases_every_local_owner() {
    let (mut sender, conn, evidence) =
        connection(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nx");
    let mut response_future = Box::pin(sender.send_request(request()));
    let mut conn = Box::pin(conn);
    assert!(conn.as_mut().poll(&mut context()).is_pending());
    assert!(conn.as_mut().poll(&mut context()).is_pending());
    let Poll::Ready(Ok(response)) = response_future.as_mut().poll(&mut context()) else {
        panic!("head missing");
    };
    let mut body = response.into_body();
    let frame = match Pin::new(&mut body).poll_frame(&mut context()) {
        Poll::Ready(Some(Ok(frame))) => frame,
        Poll::Pending => {
            assert!(conn.as_mut().poll(&mut context()).is_pending());
            let Poll::Ready(Some(Ok(frame))) = Pin::new(&mut body).poll_frame(&mut context())
            else {
                panic!("data missing");
            };
            frame
        }
        _ => panic!("data missing"),
    };
    assert_eq!(frame.data_ref().unwrap().as_ref(), b"x");
    let counts = observed(|| {
        drop(frame);
        drop(body);
        drop(response_future);
        drop(sender);
        drop(conn);
    });
    expect_discard("held frame, interest first", counts, &evidence, !patched());
}

#[test]
fn incomplete_tcp_body_discard_has_the_same_allocation_boundary() {
    use std::{
        future::poll_fn,
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (release, released) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream = accept_with_deadline(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::with_capacity(512);
        while !request.ends_with(b"\r\n\r\n") {
            assert!(request.len() < 512);
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nx")
            .unwrap();
        released.recv_timeout(Duration::from_secs(3)).unwrap();
        let mut byte = [0];
        // Unread input can cause a TCP reset instead of EOF; both certify that
        // the local socket has closed, without claiming remote rollback.
        match stream.read(&mut byte) {
            Ok(0) => (),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => (),
            other => panic!("local socket remained live: {other:?}"),
        }
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(3), async {
            let socket = tokio::net::TcpStream::connect(address).await.unwrap();
            let mut builder = http1::Builder::new();
            builder
                .max_headers(16)
                .max_header_size(4096)
                .max_buf_size(8192)
                .writev(false);
            let (mut sender, conn) = builder.handshake(TokioIo::new(socket)).await.unwrap();
            let mut conn = Box::pin(conn);
            let mut response_future = Box::pin(sender.send_request(request()));
            let response = poll_fn(|cx| {
                assert!(conn.as_mut().poll(cx).is_pending());
                response_future.as_mut().poll(cx)
            })
            .await
            .unwrap();
            let counts = observed(|| {
                drop(response);
                drop(response_future);
                drop(sender);
                drop(conn);
            });
            println!(
                "TCP incomplete body: destructor allocator requests={}, requested bytes={}",
                counts.0, counts.1
            );
            if patched() {
                assert_eq!(counts, (0, 0));
            } else {
                assert!(counts.0 > 0);
            }
        })
        .await
        .unwrap();
    });
    release.send(()).unwrap();
    peer.join().unwrap();
}

#[test]
fn two_tcp_reads_drive_request_body_and_connection_without_client_tasks() {
    use std::{
        future::poll_fn,
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    // This independently implemented peer is the only spawned fixture thread.
    let peer = std::thread::spawn(move || {
        for (target, value) in [("/one", "1"), ("/two", "2")] {
            let mut stream = accept_with_deadline(&listener);
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::with_capacity(512);
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 512);
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            assert!(request.starts_with(format!("GET {target} HTTP/1.1\r\n").as_bytes()));
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\nConnection: close\r\n\r\n{value}").as_bytes()).unwrap();
        }
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(3), async {
            for (target, expected) in [("/one", b'1'), ("/two", b'2')] {
                let socket = tokio::net::TcpStream::connect(address).await.unwrap();
                let mut builder = http1::Builder::new();
                builder
                    .max_headers(16)
                    .max_header_size(4096)
                    .max_buf_size(8192)
                    .writev(false);
                let (mut sender, conn) = builder.handshake(TokioIo::new(socket)).await.unwrap();
                let mut conn = Box::pin(conn);
                let mut done = false;
                let mut response = Box::pin(
                    sender.send_request(
                        Request::builder()
                            .uri(target)
                            .header("Host", address.to_string())
                            .body(Empty::<Bytes>::new())
                            .unwrap(),
                    ),
                );
                let response = poll_fn(|cx| {
                    if !done {
                        if let Poll::Ready(result) = conn.as_mut().poll(cx) {
                            result.unwrap();
                            done = true;
                        }
                    }
                    response.as_mut().poll(cx)
                })
                .await
                .unwrap();
                assert_eq!(response.status(), 200);
                let mut body = response.into_body();
                let mut bytes = Vec::with_capacity(16);
                loop {
                    let frame = poll_fn(|cx| {
                        if !done {
                            if let Poll::Ready(result) = conn.as_mut().poll(cx) {
                                result.unwrap();
                                done = true;
                            }
                        }
                        Pin::new(&mut body).poll_frame(cx)
                    })
                    .await;
                    let Some(frame) = frame else {
                        break;
                    };
                    let data = frame.unwrap().into_data().unwrap();
                    assert!(bytes.len() + data.len() <= 16);
                    bytes.extend_from_slice(&data);
                }
                assert_eq!(bytes, [expected]);
                drop(body);
                drop(sender);
                drop(conn);
            }
        })
        .await
        .unwrap();
    });
    peer.join().unwrap();
}
