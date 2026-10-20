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

fn unfinished_chunked_trailers_must_reject(trailers: Vec<u8>) {
    let mut bytes = chunked_head();
    bytes.extend_from_slice(b"0\r\n");
    bytes.extend_from_slice(&trailers);
    unfinished_response_must_reject(bytes);
}
fn unfinished_response_must_reject(bytes: Vec<u8>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        assert!(request(&mut socket).starts_with("GET /a?x=1 HTTP/1.1\r\n"));
        socket.write_all(&bytes).unwrap();
        // No empty trailer terminator and no peer EOF. Retirement must result
        // from the binding's policy, without caller cancellation/deadline.
        let mut byte = [0];
        match socket.read(&mut byte) {
            Ok(0) => {}
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {}
            other => panic!("unfinished response was not retired: {other:?}"),
        }
        let (mut next, _) = listener.accept().unwrap();
        assert!(request(&mut next).starts_with("GET /b HTTP/1.1\r\n"));
        next.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n2",
        )
        .unwrap();
        assert_eq!(next.read(&mut byte).unwrap(), 0);
    });
    let (timed_out, error) = recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let outcome = tokio::time::timeout(
            Duration::from_millis(250),
            std::future::poll_fn(|cx| {
                call.step(cx, &mut work());
                match call.try_take(&mut work()).unwrap() {
                    Some(result) => std::task::Poll::Ready(result),
                    None => std::task::Poll::Pending,
                }
            }),
        )
        .await;
        let timed_out = outcome.is_err();
        let error = match outcome {
            Ok(result) => result.unwrap_err(),
            Err(_) => {
                // Failure-only cleanup keeps the counterexample's ownership
                // and peer assertions useful before the production fix too.
                call.cancel().unwrap();
                take(&mut call).await.unwrap_err()
            }
        };
        drop(call);
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
        (timed_out, error)
    });
    peer.join().unwrap();
    assert!(
        !timed_out,
        "over-limit unfinished trailers remained pending until caller cancellation"
    );
    assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
    assert_eq!(error.context().cause_code(), Some(606));
    assert_eq!(
        error.context().resource_failure().unwrap().1,
        ResourceFailureDomain::BindingPolicy
    );
    assert!(error.context().plan().is_some());
    assert_eq!(
        error.context().binding(),
        Some((BindingId::new(600), BindingGeneration::INITIAL))
    );
}

#[test]
fn unfinished_main_head_and_extension_overflow_reject_while_peer_stays_open() {
    let mut head = b"HTTP/1.1 200 OK\r\nX: ".to_vec();
    head.resize(8192, b' ');
    unfinished_response_must_reject(head);
    for final_zero in [false, true] {
        let mut bytes = chunked_head();
        if final_zero {
            bytes.extend_from_slice(b"1\r\n1\r\n0;");
        } else {
            bytes.extend_from_slice(b"1;");
        }
        bytes.extend_from_slice(&[b'x'; 16384]);
        unfinished_response_must_reject(bytes);
    }
}

#[test]
fn unfinished_chunked_trailers_over_field_limit_reject_without_caller_cancellation() {
    unfinished_chunked_trailers_must_reject("X: y\r\n".repeat(33).into_bytes());
}

#[test]
fn unfinished_chunked_trailers_over_byte_limit_reject_without_caller_cancellation() {
    for size in [8192, 8193] {
        let mut trailers = b"X: ".to_vec();
        trailers.resize(size, b'x');
        unfinished_chunked_trailers_must_reject(trailers);
    }
}

fn response_case(bytes: Vec<u8>, fragmented: bool, success: bool) {
    response_case_with_close(bytes, fragmented, success, true);
}
fn response_case_with_close(bytes: Vec<u8>, fragmented: bool, success: bool, close_write: bool) {
    response_case_with_body(bytes, fragmented, success, None, close_write);
}
fn response_case_with_body(
    bytes: Vec<u8>,
    fragmented: bool,
    success: bool,
    expected: Option<&[u8]>,
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
            matches!(retirement, Ok(0))
                || matches!(&retirement, Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset),
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
        if let Some(body) = expected {
            assert_eq!(&*result.unwrap().data().unwrap().body, body);
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

fn chunked_head() -> Vec<u8> {
    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n"
        .to_vec()
}

#[test]
fn chunked_body_boundaries_deliver_exact_bytes_without_peer_eof() {
    for fragmented in [false, true] {
        for n in [0, 1, 4095, 4096, 4097] {
            let mut bytes = chunked_head();
            let body = vec![b'1'; n];
            // Multiple real chunks, extensions and the final empty section.
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
        // Split head, chunks, extensions and trailers across actual I/O wakes.
        for byte in b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: cHuNkEd\r\nTrailer: X\r\n\r\n7;note=ok\r\n[12,34]\r\n0\r\nX: y\r\n\r\n" {
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
fn unsupported_transfer_framing_rejects_without_draining() {
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
        "0\r\nbad field\r\n\r\n",      // malformed trailer
        "0\r\nX: y\r\n",               // truncated trailers
    ] {
        let mut bytes = chunked_head();
        bytes.extend_from_slice(tail.as_bytes());
        response_case(bytes, true, false);
    }
    for fields in [
        "Transfer-Encoding: gzip, chunked\r\n",
        "Transfer-Encoding: chunked, chunked\r\n",
        "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n",
        "Transfer-Encoding: chunked\r\nContent-Length: 1\r\n",
        "Transfer-Encoding: chunked\r\nTrailer: Content-Type\r\n",
        "Transfer-Encoding: chunked\r\nTrailer: bad field\r\n",
        "Content-Length: 1\r\nTrailer: X\r\n",
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
fn shared_trailer_fields_are_inclusive_and_raw_bytes_exclusive() {
    for fragmented in [false, true] {
        for n in [0, 16, 17, 31, 32, 33] {
            let mut bytes = chunked_head();
            bytes.extend_from_slice(b"1\r\n1\r\n0\r\n");
            bytes.extend_from_slice("X: y\r\n".repeat(n).as_bytes());
            bytes.extend_from_slice(b"\r\n");
            response_case_with_body(bytes, fragmented, n <= 32, (n <= 32).then_some(b"1"), false);
        }
        for n in [2048, 2049, 8190, 8191, 8192] {
            let mut bytes = chunked_head();
            bytes.extend_from_slice(b"1\r\n1\r\n0\r\n");
            let start = bytes.len();
            bytes.extend_from_slice(b"X: y");
            bytes.resize(start + n - 4, b' ');
            bytes.extend_from_slice(b"\r\n\r\n");
            response_case_with_body(
                bytes,
                fragmented,
                n < 8192,
                (n < 8192).then_some(b"1"),
                false,
            );
        }
    }
}

#[test]
fn cumulative_extensions_include_every_chunk_and_final_zero_chunk() {
    for fragmented in [false, true] {
        for total in [16382, 16383, 16384, 16385] {
            for split in [0, 1, total / 2, total] {
                let mut bytes = chunked_head();
                bytes.extend_from_slice(b"1");
                if split != 0 {
                    bytes.push(b';');
                    bytes.extend(std::iter::repeat_n(b'x', split));
                }
                bytes.extend_from_slice(b"\r\n1\r\n0");
                if total != split {
                    bytes.push(b';');
                    bytes.extend(std::iter::repeat_n(b'x', total - split));
                }
                bytes.extend_from_slice(b"\r\n\r\n");
                assert!(bytes.len() < 32768);
                response_case_with_body(
                    bytes,
                    fragmented,
                    total < 16384,
                    (total < 16384).then_some(b"1"),
                    false,
                );
            }
        }
    }
}

#[test]
fn simultaneous_main_header_body_and_trailer_limits() {
    for fragmented in [false, true] {
        let mut bytes = chunked_head();
        bytes.truncate(bytes.len() - 2);
        for _ in 2..31 {
            bytes.extend_from_slice(b"X: y\r\n");
        }
        bytes.extend_from_slice(b"X-Pad: ");
        bytes.resize(8192 - 4, b'x');
        bytes.extend_from_slice(b"\r\n\r\n");
        assert_eq!(bytes.len(), 8192);
        bytes.extend_from_slice(b"1000\r\n");
        let body = vec![b'1'; 4096];
        bytes.extend_from_slice(&body);
        bytes.extend_from_slice(b"\r\n0\r\n");
        let start = bytes.len();
        for _ in 0..31 {
            bytes.extend_from_slice(b"X: y\r\n");
        }
        bytes.extend_from_slice(b"X-Pad: y");
        bytes.resize(start + 8191 - 4, b' ');
        bytes.extend_from_slice(b"\r\n\r\n");
        assert_eq!(bytes.len() - start, 8191);
        response_case_with_body(bytes, fragmented, true, Some(&body), false);
    }
}

#[test]
fn cumulative_chunked_wire_exact_and_one_over_include_all_framing() {
    for fragmented in [false, true] {
        for wire in [32767, 32768, 32769] {
            let mut bytes = chunked_head();
            bytes.truncate(bytes.len() - 2);
            bytes.extend_from_slice(b"X-Pad: ");
            bytes.resize(8192 - 4, b'x');
            bytes.extend_from_slice(b"\r\n\r\n");
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
fn trailer_declaration_lists_ignore_empty_members_and_only_sp_htab() {
    for declaration in ["", " \t", "X", "X,", ",,X,,", " , \tX , , Y, ", ",,,"] {
        let bytes = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nTrailer: {declaration}\r\n\r\n2\r\n17\r\n0\r\nX: yes\r\nY: unannounced\r\n\r\n").into_bytes();
        response_case_with_body(bytes, true, true, Some(b"17"), false);
    }
    for declaration in [
        "bad field",
        "X;param",
        "X, bad field,",
        "X,\x0bY",
        "X,\x0cY",
    ] {
        let bytes = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nTrailer: {declaration}\r\n\r\n0\r\n\r\n").into_bytes();
        response_case(bytes, true, false);
    }
}

#[test]
fn trailers_cannot_override_primary_representation_or_framing() {
    for name in [
        "content-type",
        "content-length",
        "content-encoding",
        "transfer-encoding",
        "trailer",
        "connection",
        "upgrade",
        "host",
        "te",
    ] {
        let name = name.to_ascii_uppercase();
        response_case(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nTrailer: , X, {name},\r\n\r\n0\r\n\r\n").into_bytes(), true, false);
        let mut bytes = chunked_head();
        bytes.extend_from_slice(format!("1\r\n1\r\n0\r\n{name}: value\r\n\r\n").as_bytes());
        response_case(bytes, true, false);
    }
}

#[test]
fn incomplete_supported_responses_cancel_or_expire_then_reuse_original_runtime() {
    for partial in [
        "HTTP/1.1 200 OK\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n1",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n1\r\n1\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX: ",
    ] {
        for mode in ["cancel", "drop", "deadline"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (sent, received) = mpsc::channel();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                request(&mut socket);
                socket.write_all(partial.as_bytes()).unwrap();
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
fn buffered_completion_respects_cancel_and_result_orders() {
    for chunked in [false, true] {
        for cancel_first in [true, false] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (sent, received) = mpsc::channel();
            let (finish, finishing) = mpsc::channel();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                request(&mut socket);
                if chunked {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n12\r\n0\r\nX: y\r\n").unwrap();
                } else {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n1").unwrap();
                }
                sent.send(()).unwrap();
                finishing.recv_timeout(Duration::from_secs(5)).unwrap();
                socket
                    .write_all(if chunked { b"\r\n" } else { b"2" })
                    .unwrap();
                sent.send(()).unwrap();
                assert_retired(&mut socket);
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
                // Do not poll the engine until the final body byte is available.
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
                    assert_eq!(&*output.data().unwrap().body, b"12");
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
        // Unsupported native targets now fail during compilation/selection,
        // before construction of an OutboundRequest.
        assert!(rejected.into_parts().0.1.is_none());
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
fn deadline_drain_and_root_loss_retire_partial_chunked_peer() {
    for mode in ["deadline", "drain", "root-loss"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (sent, received) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            request(&mut socket);
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n1\r\n1\r\n0\r\nX: partial").unwrap();
            sent.send(()).unwrap();
            assert_retired(&mut socket);
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
        assert self.headers['TE'] is None
        self.send_header('Transfer-Encoding', 'chunked')
        self.send_header('Trailer', 'X-Check')
        self.end_headers()
        self.wfile.write(b'1;note=python\r\n3\r\n1\r\n7\r\n0\r\nX-Check: yes\r\n\r\n')
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
