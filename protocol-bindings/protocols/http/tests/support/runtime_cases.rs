fn parent_usage(parent: &Arc<clinkz_wot_servient::host_accounts::HostResourceAccounts>) -> [u64; 20] {
    let accounts = parent.snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS);
    std::array::from_fn(|i| accounts[i].used())
}

async fn read_b(handle: &ConsumerPropertyReadHandle) {
    let mut call = handle
        .read_property("b", InteractionOptions::new(), &mut work())
        .unwrap();
    assert_eq!(&*take(&mut call).await.unwrap().data().unwrap().body, b"29");
}

async fn finish_woken(call: &mut ConsumerPropertyReadCall) -> CoreResult<InteractionOutput> {
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
    .expect("independent HTTP owner lost wake-driven progress")
}

fn reply_b(listener: TcpListener) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        assert!(request(&mut socket).starts_with("GET /b HTTP/1.1\r\n"));
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n29",
            )
            .unwrap();
        assert_retired(&mut socket);
    })
}

async fn application_echo(mut socket: tokio::net::TcpStream) {
    use std::{future::poll_fn, pin::Pin};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    let mut written = 0;
    while written < 2 {
        let n = poll_fn(|cx| Pin::new(&mut socket).poll_write(cx, &b"ok"[written..]))
            .await
            .unwrap();
        assert!(n > 0);
        written += n;
    }
    let mut bytes = [0; 2];
    let mut read = ReadBuf::new(&mut bytes);
    while read.remaining() > 0 {
        let before = read.filled().len();
        poll_fn(|cx| Pin::new(&mut socket).poll_read(cx, &mut read))
            .await
            .unwrap();
        assert!(read.filled().len() > before);
    }
    assert_eq!(&bytes, b"ok");
}

#[test]
fn pending_connect_reset_is_a_deployment_failure_then_same_runtime_recovers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let reactor = recipe();
    let (runtime, handle, observer, parent) = engine(port);
    let baseline = parent_usage(&parent);
    let mut call = {
        let _entered = reactor.enter();
        handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap()
    };
    // Entering the stored handle does not drive the current-thread reactor.
    // First poll creates a real connection but its readiness future stays Pending.
    call.step(&mut Context::from_waker(Waker::noop()), &mut work());
    assert!(call.try_take(&mut work()).unwrap().is_none());
    let (mut socket, _) = listener.accept().expect("real pending connection");
    socket.set_nonblocking(true).unwrap();
    assert_eq!(
        socket.read(&mut [0]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    // Real peer reset before the application next drives readiness. This is a
    // pending-connect fault, unlike the already accepted immediate EMFILE case.
    let socket = socket2::Socket::from(socket);
    socket.set_linger(Some(Duration::ZERO)).unwrap();
    drop(socket);
    reactor.block_on(async {
        let error = take(&mut call).await.unwrap_err();
        assert_eq!(error.context().cause_code(), Some(604), "{error:?}");
        assert_eq!(
            error.context().resource_failure().unwrap().1,
            ResourceFailureDomain::DeploymentFailure
        );
        assert_eq!(error.context().retry_class(), RetryClass::Never);
        assert!(error.context().plan().is_some());
        assert_eq!(
            error.context().binding(),
            Some((BindingId::new(600), BindingGeneration::INITIAL))
        );
        drop(call);
        assert_eq!(parent_usage(&parent), baseline);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "failed connect retried"
        );
        listener.set_nonblocking(false).unwrap();
        let peer = reply_b(listener);
        read_b(&handle).await;
        peer.join().unwrap();
        assert_eq!(parent_usage(&parent), baseline);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert_eq!(parent_usage(&parent), [0; 20]);
    });
}

#[test]
fn stopped_reactor_registration_refuses_before_acceptance_and_live_reactor_reuses_owner() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let (runtime, handle, observer, parent) = engine(port);
    let baseline = parent_usage(&parent);
    let stopped = recipe();
    let stopped_handle = stopped.handle().clone();
    drop(stopped);
    {
        let _entered = stopped_handle.enter();
        // try_current succeeds; actual Tokio socket registration returns its
        // runtime-shutdown error. No panic/OOM injection or SDK replacement.
        assert!(tokio::runtime::Handle::try_current().is_ok());
        let rejection = handle
            .read_property(
                "a",
                InteractionOptions::new().with_timeout(Duration::from_millis(321)),
                &mut work(),
            )
            .err()
            .expect("stopped I/O driver accepted a call");
        let ((options, request), error) = rejection.into_parts();
        assert_eq!(options.timeout, Some(Duration::from_millis(321)));
        let request = request.unwrap();
        assert!(request.uri_variables().is_empty());
        assert_eq!(error.error().context().plan(), Some(request.plan_id()));
        assert_eq!(
            error.error().context().resource_failure().unwrap().1,
            ResourceFailureDomain::DeploymentFailure
        );
    }
    assert_eq!(parent_usage(&parent), baseline);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    listener.set_nonblocking(false).unwrap();
    let peer = reply_b(listener);
    recipe().block_on(async {
        read_b(&handle).await;
        peer.join().unwrap();
        assert_eq!(parent_usage(&parent), baseline);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert_eq!(parent_usage(&parent), [0; 20]);
    });
}

#[test]
fn local_retirement_keeps_independent_http_owner_and_application_io_live() {
    // Two complete public Runtimes/registrations have separate accounts but use
    // the same application reactor. Also keep an unrelated application socket.
    for connected in [false, true] {
        let a = TcpListener::bind("127.0.0.1:0").unwrap();
        let b = TcpListener::bind("127.0.0.1:0").unwrap();
        let raw = TcpListener::bind("127.0.0.1:0").unwrap();
        let a_port = a.local_addr().unwrap().port();
        let b_port = b.local_addr().unwrap().port();
        let raw_address = raw.local_addr().unwrap();
        let (a_sent, a_received) = std::sync::mpsc::channel();
        let a_peer = std::thread::spawn(move || {
            let (mut socket, _) = a.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            if connected {
                request(&mut socket);
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n1\r\n1\r\n").unwrap();
                a_sent.send(()).unwrap();
            }
            assert_retired(&mut socket);
            a.set_nonblocking(true).unwrap();
            assert_eq!(
                a.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        });
        let (b_sent, b_received) = std::sync::mpsc::channel();
        let (release_b, b_release) = std::sync::mpsc::channel();
        let b_peer = std::thread::spawn(move || {
            let (mut socket, _) = b.accept().unwrap();
            request(&mut socket);
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n2").unwrap();
            b_sent.send(()).unwrap();
            b_release.recv_timeout(Duration::from_secs(5)).unwrap();
            socket.write_all(b"9").unwrap();
            assert_retired(&mut socket);
            drop(socket);
            let peer = reply_b(b);
            peer.join().unwrap();
        });
        let raw_peer = std::thread::spawn(move || {
            let (mut socket, _) = raw.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = [0; 2];
            socket.read_exact(&mut bytes).unwrap();
            assert_eq!(&bytes, b"ok");
            socket.write_all(b"ok").unwrap();
        });
        recipe().block_on(async {
            let raw_socket = tokio::net::TcpStream::connect(raw_address).await.unwrap();
            let (a_runtime, a_handle, a_observer, a_parent) = engine(a_port);
            let (b_runtime, b_handle, b_observer, b_parent) = engine(b_port);
            let b_baseline = parent_usage(&b_parent);
            let mut b_call = b_handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                b_call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                if b_received.try_recv().is_ok() { break; }
                assert!(Instant::now() < end);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            // Explicitly consume the partial B input, leaving a live read owner.
            std::future::poll_fn(|cx| {
                b_call.step(cx, &mut work());
                assert!(b_call.try_take(&mut work()).unwrap().is_none());
                std::task::Poll::Ready(())
            }).await;
            let b_pending = parent_usage(&b_parent);
            let mut a_call = a_handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
            a_call.step(&mut Context::from_waker(Waker::noop()), &mut work());
            assert!(a_call.try_take(&mut work()).unwrap().is_none());
            if connected {
                loop {
                    a_call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                    if a_received.try_recv().is_ok() { break; }
                    assert!(Instant::now() < end);
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                a_call.step(&mut Context::from_waker(Waker::noop()), &mut work());
            }
            TRACE.with(|t| { *t.borrow_mut() = Trace { armed: true, ..EMPTY }; });
            let started = Instant::now();
            a_call.cancel().unwrap();
            a_call.step(&mut Context::from_waker(Waker::noop()), &mut work());
            let elapsed = started.elapsed();
            let trace = TRACE.with(|t| { let mut t = t.borrow_mut(); t.armed = false; *t });
            // No await/reactor tick or peer release occurs inside retirement.
            assert!(matches!(a_call.try_take(&mut work()).unwrap().unwrap(), Err(CoreError::Cancelled(_))));
            println!("independent-owner disposal connected={connected}: elapsed_ns={} allocations={} deallocations={} largest={} retained_new={}", elapsed.as_nanos(), trace.count, trace.deallocations, trace.largest, trace.live);
            drop(a_call);
            drop(a_handle);
            shutdown(&a_runtime);
            drop(a_runtime);
            drop(a_observer);
            assert_eq!(parent_usage(&a_parent), [0; 20]);
            assert_eq!(parent_usage(&b_parent), b_pending);
            assert!(b_call.try_take(&mut work()).unwrap().is_none());
            a_peer.join().unwrap();
            // Both independent owners must still make actual I/O after A's
            // entire Runtime/registration has retired on this same reactor.
            application_echo(raw_socket).await;
            release_b.send(()).unwrap();
            assert_eq!(&*finish_woken(&mut b_call).await.unwrap().data().unwrap().body, b"29");
            drop(b_call);
            assert_eq!(parent_usage(&b_parent), b_baseline);
            read_b(&b_handle).await;
            b_peer.join().unwrap();
            raw_peer.join().unwrap();
            drop(b_handle);
            shutdown(&b_runtime);
            drop(b_runtime);
            drop(b_observer);
            assert_eq!(parent_usage(&b_parent), [0; 20]);
        });
    }
}
