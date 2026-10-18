include!("support/application.rs");
use std::sync::mpsc;
async fn take_woken(call: &mut ConsumerPropertyReadCall) -> CoreResult<InteractionOutput> {
    tokio::time::timeout(
        Duration::from_secs(5),
        std::future::poll_fn(|cx| {
            call.step(cx, &mut work());
            match call.try_take(&mut work()).unwrap() {
                Some(result) => std::task::Poll::Ready(result),
                None => std::task::Poll::Pending,
            }
        }),
    )
    .await
    .expect("call made no wake-driven progress")
}
fn response_case(bytes: Vec<u8>, fragmented: bool, success: bool) {
    response_case_with_body(bytes, fragmented, success, None, true);
}
fn response_case_with_body(
    bytes: Vec<u8>,
    fragmented: bool,
    success: bool,
    expected_body: Option<&[u8]>,
    close_write: bool,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        request(&mut socket);
        if fragmented {
            for part in bytes.chunks(7) {
                if socket.write_all(part).is_err() {
                    break;
                }
            }
        } else {
            let _ = socket.write_all(&bytes);
        }
        if close_write {
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
        let mut byte = [0];
        let retirement = socket.read(&mut byte);
        assert!(
            matches!(retirement, Ok(0) | Err(_)),
            "unexpected second request"
        );
        listener.set_nonblocking(true).unwrap();
        assert!(listener.accept().is_err(), "retry/reconnect is forbidden");
    });
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let result = take_woken(&mut call).await;
        assert_eq!(result.is_ok(), success, "{result:?}");
        if let Some(expected) = expected_body {
            assert_eq!(&*result.unwrap().data().unwrap().body, expected);
        }
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
    peer.join().unwrap();
}

#[test]
fn hundred_alternating_public_reads_are_source_free_and_use_fresh_connections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        for i in 0..100 {
            let (mut socket, _) = listener.accept().unwrap();
            let head = request(&mut socket);
            let path = if i % 2 == 0 { "/a?x=1" } else { "/b" };
            assert!(
                head.starts_with(&format!("GET {path} HTTP/1.1\r\n")),
                "{head}"
            );
            assert!(
                head.to_lowercase()
                    .contains(&format!("host: 127.0.0.1:{port}\r\n"))
            );
            let body = if i % 2 == 0 { "17" } else { "29" };
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{body}").unwrap();
            let mut end = [0];
            assert_eq!(
                socket.read(&mut end).unwrap(),
                0,
                "connection not retired at result"
            );
        }
    });
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        for i in 0..100 {
            let mut call = handle
                .read_property(
                    if i % 2 == 0 { "a" } else { "b" },
                    InteractionOptions::new(),
                    &mut work(),
                )
                .unwrap();
            assert!(
                handle
                    .read_property("a", InteractionOptions::new(), &mut work())
                    .is_err()
            );
            let output = take(&mut call).await.unwrap();
            assert_eq!(
                &*output.data().unwrap().body,
                if i % 2 == 0 { b"17" } else { b"29" }
            );
            assert_eq!(
                output.metadata().binding_response().unwrap().status_code(),
                200
            );
        }
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
    peer.join().unwrap();
}

#[test]
fn framing_and_body_boundaries() {
    for fragmented in [false, true] {
        for n in [0, 1, 4095, 4096, 4097] {
            let mut fixed = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {n}\r\n\r\n"
            )
            .into_bytes();
            fixed.extend(std::iter::repeat_n(b'1', n));
            response_case(fixed, fragmented, n <= 4096);
            let mut close = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n".to_vec();
            close.extend(std::iter::repeat_n(b'1', n));
            response_case(close, fragmented, n <= 4096);
        }
    }
}
#[test]
fn exact_and_one_over_raw_header_limits() {
    for n in [8191, 8192, 8193] {
        let prefix =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\nX-Pad: ";
        let mut bytes = prefix.to_vec();
        bytes.extend(std::iter::repeat_n(b'x', n - prefix.len() - 4));
        bytes.extend_from_slice(b"\r\n\r\n1");
        response_case(bytes, true, n <= 8192);
    }
    for n in [31, 32, 33] {
        let mut bytes =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n".to_vec();
        for _ in 2..n {
            bytes.extend_from_slice(b"X: y\r\n");
        }
        bytes.extend_from_slice(b"\r\n1");
        response_case(bytes, true, n <= 32);
    }
}
#[test]
fn unsupported_and_broken_responses_never_deliver_success() {
    for bytes in [
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX: y\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n1",
        "HTTP/1.1 302 Redirect\r\nLocation: /b\r\n\r\n",
        "HTTP/1.1 500 Error\r\n\r\nsecret peer error",
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\r\n",
        "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n1",
        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nbad header\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n",
        "",
    ] {
        response_case(bytes.as_bytes().to_vec(), true, false);
    }
}

fn chunked_head(bytes: Option<usize>) -> Vec<u8> {
    let mut head =
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n"
            .to_vec();
    if let Some(bytes) = bytes {
        head.extend_from_slice(b"X-Pad: ");
        head.resize(bytes - 4, b'x');
        head.extend_from_slice(b"\r\n");
    }
    head.extend_from_slice(b"\r\n");
    head
}

#[test]
fn chunked_success_and_decoded_body_boundaries_do_not_require_peer_eof() {
    for fragmented in [false, true] {
        for n in [0, 1, 4095, 4096, 4097] {
            let mut bytes = chunked_head(None);
            let body = vec![b'1'; n];
            // Several chunks, an extension, and an empty body all exercise the
            // real SDK decoder; the binding only sees its body frames.
            for part in body.chunks(511) {
                bytes.extend_from_slice(format!("{:X};example=value\r\n", part.len()).as_bytes());
                bytes.extend_from_slice(part);
                bytes.extend_from_slice(b"\r\n");
            }
            bytes.extend_from_slice(b"0\r\n\r\n");
            response_case_with_body(
                bytes,
                fragmented,
                n <= 4096,
                (n <= 4096).then_some(body.as_slice()),
                false,
            );
        }
    }
}

#[test]
fn slow_chunked_octets_wake_public_runtime_across_every_delimiter() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        request(&mut socket);
        socket.set_nodelay(true).unwrap();
        // Independent writes with a pause split status/head, chunk-size,
        // extension, data, zero chunk and every CRLF across reactor turns.
        for byte in b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: cHuNkEd\r\n\r\n7;note=ok\r\n[12,34]\r\n0\r\n\r\n" {
            socket.write_all(&[*byte]).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        assert_eq!(
            &*take_woken(&mut call).await.unwrap().data().unwrap().body,
            b"[12,34]"
        );
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
    peer.join().unwrap();
}

#[test]
fn cumulative_chunked_wire_exact_and_one_over_include_all_framing() {
    for fragmented in [false, true] {
        for wire in [32767, 32768, 32769] {
            // A maximal head and 4094 one-byte chunks leave seven bytes for
            // extension syntax at the exact wire limit. Body bytes remain
            // below 4096, so body policy cannot mask the wire boundary.
            let mut bytes = chunked_head(Some(8192));
            assert_eq!(bytes.len(), 8192);
            let base = 8192 + 4094 * 6 + 5;
            bytes.extend_from_slice(b"1;");
            bytes.extend(std::iter::repeat_n(b'x', wire - base - 1));
            bytes.extend_from_slice(b"\r\n1\r\n");
            for _ in 1..4094 {
                bytes.extend_from_slice(b"1\r\n1\r\n");
            }
            bytes.extend_from_slice(b"0\r\n\r\n");
            assert_eq!(bytes.len(), wire);
            let body = vec![b'1'; 4094];
            response_case_with_body(
                bytes,
                fragmented,
                wire <= 32768,
                (wire <= 32768).then_some(body.as_slice()),
                false,
            );
        }
    }
}

#[test]
fn chunked_failures_reject_without_draining_or_reinterpreting_framing() {
    for tail in [
        "g\r\n1\r\n0\r\n\r\n",         // invalid chunk size
        "10000000000000000\r\n",       // overflow
        "1\r\n",                       // missing body
        "2\r\n1",                      // truncated body
        "1\r\n1X\n0\r\n\r\n",          // missing chunk CR
        "1\r\n1\rX0\r\n\r\n",          // missing chunk LF
        "1;extension\n1\r\n0\r\n\r\n", // invalid extension
        "1\r\n1\r\n",                  // missing final chunk
        "0\r\n",                       // missing trailer terminator
        "0\r\nX: y\r\n\r\n",           // actual unannounced trailers
        "0\r\nX: y\r\n",               // truncated trailers
    ] {
        let mut bytes = chunked_head(None);
        bytes.extend_from_slice(tail.as_bytes());
        response_case(bytes, true, false);
    }
    for fields in [
        "Transfer-Encoding: gzip, chunked\r\n",
        "Transfer-Encoding: chunked, chunked\r\n",
        "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n",
        "Transfer-Encoding: chunked\r\nContent-Length: 1\r\n",
        "Transfer-Encoding: chunked\r\nTrailer: X\r\n",
    ] {
        response_case(
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{fields}\r\n1\r\n1\r\n0\r\n\r\n")
                .into_bytes(),
            true,
            false,
        );
    }
}

#[test]
fn partial_trailer_and_sdk_extension_restrictions_remain_explicit() {
    for n in [16, 17] {
        let mut bytes = chunked_head(None);
        bytes.extend_from_slice(b"0\r\n");
        bytes.extend_from_slice("X: y\r\n".repeat(n).as_bytes());
        bytes.extend_from_slice(b"\r\n");
        response_case(bytes, true, false);
    }
    for n in [2048, 2049] {
        let mut bytes = chunked_head(None);
        bytes.extend_from_slice(b"0\r\n");
        let start = bytes.len();
        bytes.extend_from_slice(b"X: y");
        bytes.resize(start + n - 4, b' ');
        bytes.extend_from_slice(b"\r\n\r\n");
        response_case(bytes, true, false);
    }
    for n in [16383, 16384] {
        let mut bytes = chunked_head(None);
        bytes.extend_from_slice(b"1;");
        bytes.extend(std::iter::repeat_n(b'x', n));
        bytes.extend_from_slice(b"\r\n1\r\n0\r\n\r\n");
        assert!(bytes.len() < 32768);
        response_case_with_body(bytes, true, n < 16384, (n < 16384).then_some(b"1"), false);
    }
}

#[test]
fn incomplete_chunked_phases_cancel_or_expire_then_reuse_original_runtime() {
    for tail in ["1", "2\r\n1", "1\r\n1\r\n", "0\r\nX: "] {
        for mode in ["cancel", "drop", "deadline"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (sent, received) = mpsc::channel();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                request(&mut socket);
                socket.write_all(&chunked_head(None)).unwrap();
                socket.write_all(tail.as_bytes()).unwrap();
                sent.send(()).unwrap();
                let mut byte = [0];
                assert_eq!(
                    socket.read(&mut byte).unwrap(),
                    0,
                    "retirement waited for framing"
                );
                let (mut next, _) = listener.accept().unwrap();
                assert!(request(&mut next).starts_with("GET /b HTTP/1.1\r\n"));
                next.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n2").unwrap();
            });
            recipe().block_on(async {
                let (runtime, handle, observer, parent) = engine(port);
                let mut call = handle
                    .read_property(
                        "a",
                        if mode == "deadline" {
                            InteractionOptions::new().with_timeout(Duration::from_millis(100))
                        } else {
                            InteractionOptions::new()
                        },
                        &mut work(),
                    )
                    .unwrap();
                let end = Instant::now() + Duration::from_secs(5);
                loop {
                    call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                    if received.try_recv().is_ok() {
                        break;
                    }
                    assert!(Instant::now() < end);
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                // Reactor progress lets the decoder consume the partial frame.
                for _ in 0..3 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                    assert!(call.try_take(&mut work()).unwrap().is_none());
                }
                assert!(parent.lock().unwrap().iter().any(|a| a.used() != 0));
                // Result occupancy cannot be borrowed by an ordinary new read.
                let rejected = handle
                    .read_property(
                        "b",
                        InteractionOptions::new().with_uri_variable("proof", "original"),
                        &mut work(),
                    )
                    .err()
                    .unwrap();
                let (options, request) = rejected.into_parts().0;
                assert!(
                    request.is_none(),
                    "engine occupancy must reject before raw construction"
                );
                assert_eq!(options.uri_variables().get("proof").unwrap(), "original");
                if mode == "cancel" {
                    call.cancel().unwrap();
                }
                if mode != "drop" {
                    let error = take(&mut call).await.unwrap_err();
                    if mode == "deadline" {
                        assert!(matches!(error, CoreError::TimedOut(_)), "{error:?}");
                    } else {
                        assert!(matches!(error, CoreError::Cancelled(_)), "{error:?}");
                    }
                }
                drop(call);
                for _ in 0..3 {
                    runtime.step(&mut Context::from_waker(Waker::noop()), &mut work());
                }
                let mut next = handle
                    .read_property("b", InteractionOptions::new(), &mut work())
                    .unwrap();
                assert_eq!(
                    &*take_woken(&mut next).await.unwrap().data().unwrap().body,
                    b"2"
                );
                drop(next);
                drop(handle);
                shutdown(&runtime);
                drop(runtime);
                drop(observer);
                assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
            });
            peer.join().unwrap();
        }
    }
}

#[test]
fn buffered_chunked_completion_respects_both_cancel_and_result_orders() {
    for cancel_first in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (sent, received) = mpsc::channel();
        let (finish, finishing) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            request(&mut socket);
            socket.write_all(&chunked_head(None)).unwrap();
            socket.write_all(b"1\r\n1\r\n").unwrap();
            sent.send(()).unwrap();
            finishing.recv_timeout(Duration::from_secs(5)).unwrap();
            socket.write_all(b"0\r\n\r\n").unwrap();
            sent.send(()).unwrap();
            let mut byte = [0];
            assert!(matches!(socket.read(&mut byte), Ok(0) | Err(_)));
        });
        recipe().block_on(async {
            let (runtime, handle, observer, parent) = engine(port);
            let mut call = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                if received.try_recv().is_ok() {
                    break;
                }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            finish.send(()).unwrap();
            // Do not poll the engine until the final chunk is available.
            while received.try_recv().is_err() {
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            if cancel_first {
                call.cancel().unwrap();
                assert!(matches!(
                    take_woken(&mut call).await,
                    Err(CoreError::Cancelled(_))
                ));
            } else {
                let output = take_woken(&mut call).await.unwrap();
                assert_eq!(&*output.data().unwrap().body, b"1");
                assert!(call.cancel().is_err());
                drop(output);
            }
            drop(call);
            drop(handle);
            shutdown(&runtime);
            drop(runtime);
            drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        });
        peer.join().unwrap();
    }
}
#[test]
fn cancellation_and_dropped_ticket_retire_socket_and_allow_next_read() {
    for drop_ticket in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (sent, received) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            request(&mut socket);
            sent.send(()).unwrap();
            // Peer stays silent; cancellation cannot wait for network progress.
            let mut byte = [0];
            assert_eq!(socket.read(&mut byte).unwrap(), 0);
            let (mut next, _) = listener.accept().unwrap();
            assert!(request(&mut next).starts_with("GET /b HTTP/1.1"));
            next.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n2",
            )
            .unwrap();
        });
        recipe().block_on(async {
            let (runtime, handle, observer, parent) = engine(port);
            let mut call = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                if received.try_recv().is_ok() {
                    break;
                }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            if !drop_ticket {
                call.cancel().unwrap();
                assert!(take(&mut call).await.is_err());
            }
            drop(call);
            for _ in 0..10 {
                runtime.step(&mut Context::from_waker(Waker::noop()), &mut work());
            }
            let mut next = handle
                .read_property("b", InteractionOptions::new(), &mut work())
                .unwrap();
            assert!(take(&mut next).await.is_ok());
            drop(next);
            drop(handle);
            shutdown(&runtime);
            drop(runtime);
            drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        });
        peer.join().unwrap();
    }
}

#[test]
fn preacceptance_rejections_recover_the_original_request_without_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    // Missing runtime and a runtime without enable_io both reject before raw
    // acceptance. Registration and consume themselves require no Tokio context.
    for disabled in [false, true] {
        let (runtime, handle, observer, parent) = engine(port);
        let disabled_runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let entered = disabled.then(|| disabled_runtime.enter());
        let rejected = match handle.read_property("a", InteractionOptions::new(), &mut work()) {
            Err(r) => r,
            Ok(_) => panic!("missing I/O services accepted"),
        };
        let (_, request) = rejected.into_parts().0;
        assert!(request.unwrap().uri_variables().is_empty());
        drop(entered);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
    let rt = recipe();
    let _entered = rt.enter();
    for target in [
        "https://127.0.0.1/a",
        "http://localhost/a",
        "http://user@127.0.0.1/a",
        "http://127.0.0.1:0/a",
    ] {
        let (runtime, handle, observer, parent) =
            engine_with_targets(target.into(), format!("http://127.0.0.1:{port}/b"));
        let rejected = match handle.read_property("a", InteractionOptions::new(), &mut work()) {
            Err(r) => r,
            Ok(_) => panic!("unsupported target accepted: {target}"),
        };
        assert!(rejected.into_parts().0.1.is_some());
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    }
    let (runtime, handle, observer, parent) = engine(port);
    let rejected = match handle.read_property(
        "a",
        InteractionOptions::new().with_uri_variable("x", "original"),
        &mut work(),
    ) {
        Err(r) => r,
        Ok(_) => panic!("URI variables accepted"),
    };
    let request = rejected.into_parts().0.1.unwrap();
    assert_eq!(request.uri_variables().get("x").unwrap(), "original");
    drop(handle);
    shutdown(&runtime);
    drop(runtime);
    drop(observer);
    assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    assert!(listener.accept().is_err());
}

#[test]
fn deadline_drain_and_root_loss_retire_a_silent_peer() {
    for mode in ["deadline", "drain", "root-loss"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (sent, received) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            request(&mut socket);
            sent.send(()).unwrap();
            let mut byte = [0];
            assert_eq!(socket.read(&mut byte).unwrap(), 0);
        });
        recipe().block_on(async {
            let (runtime, handle, observer, parent) = engine(port);
            let options = if mode == "deadline" {
                InteractionOptions::new().with_timeout(Duration::from_millis(100))
            } else {
                InteractionOptions::new()
            };
            let mut call = handle.read_property("a", options, &mut work()).unwrap();
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                if received.try_recv().is_ok() {
                    break;
                }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            if mode == "root-loss" {
                drop(runtime);
                drop(handle);
                drop(call);
            } else {
                if mode == "drain" {
                    runtime.close();
                }
                let result = take(&mut call).await.unwrap_err();
                if mode == "deadline" {
                    assert!(matches!(result, CoreError::TimedOut(_)), "{result:?}");
                }
                drop(call);
                drop(handle);
                shutdown(&runtime);
                drop(runtime);
            }
            if mode == "root-loss" {
                assert!(observer().0.unwrap().is_residual());
            } else {
                assert!(observer().0.unwrap().is_complete());
            }
            drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        });
        peer.join().unwrap();
    }
}

#[test]
fn independent_python_standard_library_peer() {
    use std::io::{BufRead, BufReader};
    let mut peer = std::process::Command::new("python3")
        .args([
            "-u",
            "-c",
            r#"
from http.server import BaseHTTPRequestHandler, HTTPServer
class Peer(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def do_GET(self):
        assert self.path == '/a?x=1'
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', '2')
        self.end_headers()
        self.wfile.write(b'37')
    def log_message(self, *args): pass
server = HTTPServer(('127.0.0.1', 0), Peer)
server.timeout = 5
print(server.server_port, flush=True)
server.handle_request()
server.server_close()
"#,
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(peer.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let port = line.trim().parse().unwrap();
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let output = take(&mut call).await.unwrap();
        assert_eq!(&*output.data().unwrap().body, b"37");
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
    assert!(peer.wait().unwrap().success());
}

#[test]
fn accepted_but_unpolled_call_has_no_network_effect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(listener.local_addr().unwrap().port());
        let call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        assert!(listener.accept().is_err());
        call.cancel().unwrap();
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(listener.accept().is_err());
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
}
