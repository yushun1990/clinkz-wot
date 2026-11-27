#![cfg(feature = "prepare")]
//! Deterministic clock/peer ordering over the actual SDK and public Runtime.
#![allow(dead_code)] // The shared application also supplies wall-clock helpers.
include!("support/application.rs");
use std::sync::mpsc;

#[test]
fn real_transport_handoff_cancellation_deadline_and_late_native_outcome() {
    for framing in ["fixed", "chunked", "close", "invalid-trailer"] {
        for order in [
            "result-first",
            "cancel-first",
            "deadline-first",
            "cancel-then-deadline",
            "native-first-deadline-before-handoff",
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (sent, received) = mpsc::channel();
            let (finish, finishing) = mpsc::channel();
            let (retired, retirement) = mpsc::channel();
            let peer = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                assert!(request(&mut socket).starts_with("GET /a?x=1 HTTP/1.1\r\n"));
                socket.write_all(match framing {
                    "fixed" => b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n1",
                    "close" => b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n12",
                    _ => b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n12\r\n0\r\n",
                }).unwrap();
                sent.send(()).unwrap();
                finishing.recv_timeout(Duration::from_secs(5)).unwrap();
                match framing {
                    "fixed" => socket.write_all(b"2").unwrap(),
                    "close" => socket.shutdown(std::net::Shutdown::Write).unwrap(),
                    "chunked" => socket.write_all(b"X: y\r\n\r\n").unwrap(),
                    _ => socket
                        .write_all(b"Content-Type: text/plain\r\n\r\n")
                        .unwrap(),
                }
                sent.send(()).unwrap();
                assert_retired(&mut socket);
                drop(socket);
                retired.send(()).unwrap();
                let (mut socket, _) = listener.accept().unwrap();
                assert!(request(&mut socket).starts_with("GET /b HTTP/1.1\r\n"));
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n29").unwrap();
                assert_retired(&mut socket);
            });
            recipe().block_on(async {
                let (owner, handle, calls, bank) = engine(port);
                let baseline = usage(&bank);
                let instant = |tick| MonotonicInstant::new(ClockId::new(600), tick);
                let mut call = handle
                    .read_property(&calls, "a", Deadline::at(instant(100)), instant(0))
                    .unwrap();
                assert_ne!(usage(&bank), baseline);
                let pressure: Vec<_> = ResourceKind::ALL
                    .into_iter()
                    .map(|kind| {
                        let account = bank.account(kind);
                        bank.reserve(kind, account.limit() - account.used())
                            .unwrap()
                    })
                    .collect();
                let held = usage(&bank);
                let mut cx = Context::from_waker(Waker::noop());
                let end = Instant::now() + Duration::from_secs(5);
                loop {
                    assert!(call.progress(&mut cx, instant(0)).is_pending());
                    if received.try_recv().is_ok() {
                        break;
                    }
                    assert!(Instant::now() < end);
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                // Let the decoder reach the deliberately incomplete frame.
                for _ in 0..3 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    assert!(call.progress(&mut cx, instant(1)).is_pending());
                    assert!(!call.is_settled());
                    assert_eq!(usage(&bank), held);
                    assert!(retirement.try_recv().is_err());
                }
                finish.send(()).unwrap();
                received.recv_timeout(Duration::from_secs(5)).unwrap();
                let tick = match order {
                    "result-first" | "native-first-deadline-before-handoff" => {
                        while call.progress(&mut cx, instant(2)).is_pending() {
                            assert!(Instant::now() < end);
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                        assert!(call.is_settled());
                        assert_eq!(usage(&bank), held);
                        if order == "result-first" { 2 } else { 100 }
                    }
                    "deadline-first" => {
                        assert!(matches!(
                            call.progress(&mut cx, instant(100)),
                            Poll::Ready(Ok(()))
                        ));
                        call.cancel();
                        100
                    }
                    "cancel-then-deadline" => {
                        call.cancel();
                        100
                    }
                    _ => {
                        call.cancel();
                        2
                    }
                };
                let Poll::Ready(Ok(result)) = call.poll(&mut cx, instant(tick)) else {
                    panic!("bounded native retirement did not settle");
                };
                retirement.recv_timeout(Duration::from_secs(5)).unwrap();
                // The handoff and old handle have independent lifetimes.
                call.cancel();
                drop(call);
                match order {
                    "result-first" | "native-first-deadline-before-handoff"
                        if framing == "invalid-trailer" =>
                    {
                        let error = result.result().unwrap_err();
                        assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
                        assert_eq!(
                            error.context().resource_failure().unwrap().1,
                            ResourceFailureDomain::BindingPolicy
                        );
                    }
                    "result-first" => {
                        let output = result.result().unwrap();
                        assert_eq!(output.data().unwrap().body, b"12");
                        let response = output.metadata().binding_response().unwrap();
                        assert_eq!(response.status_code().unwrap(), 200);
                        assert_eq!(response.binding_id(), BindingId::new(600));
                    }
                    "native-first-deadline-before-handoff" => {
                        assert!(matches!(result.result(), Err(CoreError::TimedOut(_))));
                        let clinkz_wot_servient_finite::consumer::ConsumerNativeOutcome::Returned(
                            Ok(native),
                        ) = result.native_outcome()
                        else {
                            panic!("late native value was lost");
                        };
                        assert_eq!(native.data().unwrap().body, b"12");
                    }
                    "deadline-first" => {
                        assert!(matches!(result.result(), Err(CoreError::TimedOut(_))))
                    }
                    _ => assert!(matches!(result.result(), Err(CoreError::Cancelled(_)))),
                }
                // Other owners may give their quotas back before this result.
                // The outstanding unit remains owned until disposal.
                drop(pressure);
                let outstanding = ResourceKind::PendingHandlerCallsGlobalMax;
                assert_eq!(bank.account(outstanding).used(), 1);
                let holder = bank
                    .reserve(outstanding, bank.account(outstanding).limit() - 1)
                    .unwrap();
                assert!(matches!(
                    handle.read_property(&calls, "b", Deadline::NONE, instant(tick)),
                    Err(CoreError::LimitExceeded { resource, .. }) if resource == outstanding
                ));
                drop(result);
                let mut retry = handle
                    .read_property(&calls, "b", Deadline::NONE, instant(tick))
                    .unwrap();
                let reply = tokio::time::timeout(
                    Duration::from_secs(5),
                    std::future::poll_fn(|cx| retry.poll(cx, instant(tick))),
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(reply.result().unwrap().data().unwrap().body, b"29");
                drop((reply, retry, holder));
                assert_eq!(usage(&bank), baseline);
                release((owner, handle, calls, bank));
            });
            peer.join().unwrap();
        }
    }
}
