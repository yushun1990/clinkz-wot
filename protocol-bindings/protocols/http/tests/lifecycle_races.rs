//! Deterministic clock/peer ordering over the actual SDK and public Runtime.
#![allow(dead_code)] // The shared application also supplies wall-clock helpers.
include!("support/application.rs");
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc,
};

struct TestClock(AtomicU64);
impl RuntimeClock for TestClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(ClockId::new(600), self.0.load(Ordering::SeqCst))
    }
    fn ticks_per_second(&self) -> NonZeroU64 {
        NonZeroU64::new(1000).unwrap()
    }
}

#[test]
fn real_transport_result_cancel_deadline_and_short_credit_orders() {
    for framing in ["fixed", "chunked", "close", "invalid-trailer"] {
        for order in [
            "result-first",
            "cancel-first",
            "deadline-first",
            "cancel-then-deadline",
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
                let clock = Arc::new(TestClock(AtomicU64::new(0)));
                let (runtime, handle, observer, parent) = engine_with_clock(
                    format!("http://127.0.0.1:{port}/a?x=1"),
                    format!("http://127.0.0.1:{port}/b"),
                    clock.clone(),
                );
                let baseline: Vec<_> = parent.lock().unwrap().iter().map(|a| a.used()).collect();
                let mut call = handle
                    .read_property(
                        "a",
                        InteractionOptions::new().with_timeout(Duration::from_millis(100)),
                        &mut work(),
                    )
                    .unwrap();
                assert_ne!(
                    parent
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|a| a.used())
                        .collect::<Vec<_>>(),
                    baseline
                );
                // Model an unrelated engine owner consuming every remaining
                // parent unit after acceptance, including cleanup accounts.
                // The admitted call must use its already backed destinations.
                let pressure: [u64; 20] = {
                    let mut accounts = parent.lock().unwrap();
                    std::array::from_fn(|i| {
                        let amount = accounts[i].limit() - accounts[i].used();
                        accounts[i].try_reserve(amount).unwrap().commit();
                        amount
                    })
                };
                let held: Vec<_> = parent.lock().unwrap().iter().map(|a| a.used()).collect();
                let end = Instant::now() + Duration::from_secs(5);
                loop {
                    call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                    if received.try_recv().is_ok() {
                        break;
                    }
                    assert!(Instant::now() < end);
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                // Permit actual decoder progress, then withhold the final byte /
                // trailer / EOF. Neither zero nor short callback/cleanup credit
                // can dispose the original owner or make the slot reusable.
                for _ in 0..3 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                    for credit in [0, 1, 63] {
                        call.step(
                            &mut Context::from_waker(Waker::noop()),
                            &mut work()
                                .with_remaining(WorkClass::BindingPolls, credit)
                                .with_remaining(WorkClass::CleanupItems, 0),
                        );
                        assert!(call.try_take(&mut WorkBudget::new()).unwrap().is_none());
                    }
                    assert_eq!(
                        parent
                            .lock()
                            .unwrap()
                            .iter()
                            .map(|a| a.used())
                            .collect::<Vec<_>>(),
                        held
                    );
                    assert!(retirement.try_recv().is_err());
                }
                finish.send(()).unwrap();
                received.recv_timeout(Duration::from_secs(5)).unwrap();
                match order {
                    "result-first" => {
                        // Observe actual socket retirement before result taking;
                        // retain the funded terminal result under cleanup pressure.
                        loop {
                            call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                            if retirement.try_recv().is_ok() {
                                break;
                            }
                            assert!(Instant::now() < end);
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                        assert!(
                            call.try_take(&mut work().with_remaining(WorkClass::CleanupItems, 0))
                                .unwrap()
                                .is_none()
                        );
                        assert_eq!(
                            parent
                                .lock()
                                .unwrap()
                                .iter()
                                .map(|a| a.used())
                                .collect::<Vec<_>>(),
                            held
                        );
                        clock.0.store(100, Ordering::SeqCst);
                        call.cancel().unwrap();
                    }
                    "deadline-first" => {
                        clock.0.store(100, Ordering::SeqCst);
                        // Clock expiry commits even with no progress credit.
                        call.step(
                            &mut Context::from_waker(Waker::noop()),
                            &mut WorkBudget::new(),
                        );
                        call.cancel().unwrap();
                    }
                    _ => {
                        call.cancel().unwrap();
                        if order == "cancel-then-deadline" {
                            clock.0.store(100, Ordering::SeqCst);
                        }
                    }
                }
                let result = take(&mut call).await;
                match order {
                    "result-first" if framing == "invalid-trailer" => {
                        let error = result.unwrap_err();
                        assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
                        assert_eq!(
                            error.context().resource_failure().unwrap().1,
                            ResourceFailureDomain::BindingPolicy
                        );
                        assert!(error.context().plan().is_some());
                    }
                    "result-first" => {
                        let output = result.unwrap();
                        assert_eq!(&*output.data().unwrap().body, b"12");
                        let response = output.metadata().binding_response().unwrap();
                        assert_eq!(response.status_code(), 200);
                        assert_eq!(response.binding_id(), BindingId::new(600));
                    }
                    "deadline-first" => {
                        assert!(matches!(result, Err(CoreError::TimedOut(_))), "{result:?}")
                    }
                    _ => assert!(matches!(result, Err(CoreError::Cancelled(_))), "{result:?}"),
                }
                if order != "result-first" {
                    retirement.recv_timeout(Duration::from_secs(5)).unwrap();
                }
                drop(call);
                for (account, amount) in parent.lock().unwrap().iter_mut().zip(pressure) {
                    assert!(account.release_committed(amount));
                }
                assert_eq!(
                    parent
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|a| a.used())
                        .collect::<Vec<_>>(),
                    baseline
                );
                let mut retry = handle
                    .read_property("b", InteractionOptions::new(), &mut work())
                    .unwrap();
                assert_eq!(
                    &*take(&mut retry).await.unwrap().data().unwrap().body,
                    b"29"
                );
                drop(retry);
                drop(handle);
                shutdown(&runtime);
                drop(runtime);
                assert!(observer().0.unwrap().is_complete());
                drop(observer);
                assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
            });
            peer.join().unwrap();
        }
    }
}
