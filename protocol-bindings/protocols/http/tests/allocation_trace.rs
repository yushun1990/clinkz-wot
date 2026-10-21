#![cfg(feature = "prepare")]
//! Allocation observations for the real public path. No allocator replacement
//! or null injection is attempted inside unqualified infallible SDK operations.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
};
#[derive(Clone, Copy)]
struct Trace {
    armed: bool,
    count: usize,
    deallocations: usize,
    live: usize,
    peak: usize,
    largest: usize,
    last_layout: (usize, usize),
    blocks: [(usize, usize); 512],
    sizes: [(usize, usize); 64],
}
const EMPTY: Trace = Trace {
    armed: false,
    count: 0,
    deallocations: 0,
    live: 0,
    peak: 0,
    largest: 0,
    last_layout: (0, 0),
    blocks: [(0, 0); 512],
    sizes: [(0, 0); 64],
};
thread_local! { static TRACE: RefCell<Trace> = const { RefCell::new(EMPTY) }; }
thread_local! {
    // On this pinned public path, the first 4096-byte, align-1 allocation
    // after acceptance is the fallible body reservation, not SDK internals.
    static DENY_NEXT_BODY: Cell<bool> = const { Cell::new(false) };
    static BODY_REFUSALS: Cell<usize> = const { Cell::new(0) };
    static DENY_NEXT_CALL: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
    static CALL_REFUSALS: Cell<usize> = const { Cell::new(0) };
}
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Refuse only the measured constructor Layout, before SDK progress.
        // Feature unification can change the raw call representation.
        if DENY_NEXT_CALL
            .try_with(|deny| {
                if deny.get() == Some((layout.size(), layout.align())) {
                    deny.set(None);
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
        {
            CALL_REFUSALS.with(|count| count.set(count.get() + 1));
            return std::ptr::null_mut();
        }
        if layout.size() == 4096
            && layout.align() == 1
            && DENY_NEXT_BODY
                .try_with(|deny| deny.replace(false))
                .unwrap_or(false)
        {
            BODY_REFUSALS.with(|count| count.set(count.get() + 1));
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(layout) };
        let _ = TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.armed && !p.is_null() {
                t.count += 1;
                t.last_layout = (layout.size(), layout.align());
                let slot = t.sizes.iter_mut().find(|(n, _)| *n == layout.size());
                if let Some((_, count)) = slot {
                    *count += 1;
                } else {
                    let slot = t
                        .sizes
                        .iter_mut()
                        .find(|(n, _)| *n == 0)
                        .expect("size trace capacity");
                    *slot = (layout.size(), 1);
                }
                t.live += layout.size();
                t.peak = t.peak.max(t.live);
                t.largest = t.largest.max(layout.size());
                let slot = t
                    .blocks
                    .iter_mut()
                    .find(|b| b.0 == 0)
                    .expect("trace capacity");
                *slot = (p as usize, layout.size());
            }
        });
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        let _ = TRACE.try_with(|t| {
            let mut t = t.borrow_mut();
            if t.armed {
                t.deallocations += 1;
            }
            if let Some(i) = t.blocks.iter().position(|b| b.0 == p as usize) {
                assert_eq!(t.blocks[i].1, layout.size());
                t.live -= layout.size();
                t.blocks[i] = (0, 0);
            }
        });
        unsafe { System.dealloc(p, layout) }
    }
}
// Keep the application/peer implementation in one place. The normal tests are
// imported only in the real_peer integration target, not repeated here.
include!("support/application.rs");
// Reuse the same allocation observer, without repeating the accepted pressure
// matrix. These cases qualify previously uncovered runtime/independent owners.
include!("support/runtime_cases.rs");

#[test]
fn raw_call_allocation_refusal_recovers_input_and_releases_binding_occupancy() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    recipe().block_on(async {
        let (runtime, handle, observer, parent) = engine(port);
        let baseline: Vec<_> = parent.lock().unwrap().iter().map(|a| a.used()).collect();
        // The final allocation in Client::invoke is HostBindingCallBox::try_new.
        // Measure this graph's actual Layout using an accepted, unpolled call;
        // the rejection's cause below independently checks the raw owner.
        TRACE.with(|t| *t.borrow_mut() = Trace { armed: true, ..EMPTY });
        let mut probe = handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
        let layout = TRACE.with(|t| { let mut t = t.borrow_mut(); t.armed = false; t.last_layout });
        assert_ne!(layout.0, 0);
        probe.cancel().unwrap();
        assert!(matches!(take(&mut probe).await, Err(CoreError::Cancelled(_))));
        drop(probe);
        assert_eq!(parent.lock().unwrap().iter().map(|a| a.used()).collect::<Vec<_>>(), baseline);
        println!("fallible raw-call allocation Layout={layout:?}");
        CALL_REFUSALS.with(|n| n.set(0));
        DENY_NEXT_CALL.with(|deny| deny.set(Some(layout)));
        let rejected = handle.read_property("a", InteractionOptions::new().with_timeout(Duration::from_millis(321)), &mut work()).err().expect("raw box refusal");
        assert!(DENY_NEXT_CALL.with(|deny| deny.replace(None)).is_none(), "raw box allocation was not reached");
        assert_eq!(CALL_REFUSALS.with(Cell::get), 1);
        let ((options, original), error) = rejected.into_parts();
        assert_eq!(options.timeout, Some(Duration::from_millis(321)));
        assert_eq!(error.error().context().cause_code(), Some(600), "the raw HTTP constructor, not Core's envelope, must reject");
        assert_eq!(error.error().context().plan(), Some(original.as_ref().unwrap().plan_id()));
        assert_eq!(error.error().context().resource_failure().unwrap().1, ResourceFailureDomain::DeploymentFailure);
        drop((options, original, error));
        assert_eq!(parent.lock().unwrap().iter().map(|a| a.used()).collect::<Vec<_>>(), baseline);
        assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        listener.set_nonblocking(false).unwrap();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            assert!(request(&mut socket).starts_with("GET /b HTTP/1.1\r\n"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n29").unwrap();
            assert_retired(&mut socket);
        });
        let mut retry = handle.read_property("b", InteractionOptions::new(), &mut work()).unwrap();
        assert_eq!(&*take(&mut retry).await.unwrap().data().unwrap().body, b"29");
        drop(retry);
        peer.join().unwrap();
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
    });
}

#[test]
fn body_allocator_refusal_preserves_deployment_domain_and_allows_retry() {
    for chunked in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            for attempt in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                assert!(request(&mut socket).starts_with("GET /a?x=1 HTTP/1.1\r\n"));
                if chunked {
                    let body = if attempt == 1 {
                        vec![b'1'; 4097]
                    } else {
                        vec![b'1']
                    };
                    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n", body.len()).unwrap();
                    socket.write_all(&body).unwrap();
                    let _ = socket.write_all(b"\r\n0\r\nX: y\r\n\r\n");
                } else if attempt == 1 {
                    // No Content-Length: reach the actual decoded-body policy limit.
                    socket
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n")
                        .unwrap();
                    socket.write_all(&[b'1'; 4097]).unwrap();
                    socket.shutdown(std::net::Shutdown::Write).unwrap();
                } else {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n1").unwrap();
                }
                let mut byte = [0];
                assert_eq!(
                    socket.read(&mut byte).unwrap(),
                    0,
                    "socket survived settlement"
                );
            }
        });
        recipe().block_on(async {
            let (runtime, handle, observer, parent) = engine(port);
            let mut failures = Vec::new();
            BODY_REFUSALS.with(|count| count.set(0));
            for domain in [
                ResourceFailureDomain::DeploymentFailure,
                ResourceFailureDomain::BindingPolicy,
            ] {
                let mut call = handle
                    .read_property("a", InteractionOptions::new(), &mut work())
                    .unwrap();
                DENY_NEXT_BODY
                    .with(|deny| deny.set(domain == ResourceFailureDomain::DeploymentFailure));
                let result = take(&mut call).await;
                let still_armed = DENY_NEXT_BODY.with(|deny| deny.replace(false));
                assert!(!still_armed, "body reservation was not reached");
                let error = result.unwrap_err();
                assert!(matches!(error, CoreError::Binding(_)), "{error:?}");
                let (key, actual_domain, local, parent_owner) =
                    error.context().resource_failure().unwrap();
                assert_eq!(actual_domain, domain, "{error:?}");
                assert_eq!((local, parent_owner), (None, None));
                assert_eq!(
                    error.context().binding(),
                    Some((BindingId::new(600), BindingGeneration::INITIAL))
                );
                assert!(error.context().plan().is_some());
                assert_eq!(error.context().phase(), ErrorPhase::Binding);
                assert_eq!(error.retry_class(), RetryClass::Never);
                assert_eq!(
                    error.context().cause_code(),
                    Some(if domain == ResourceFailureDomain::DeploymentFailure {
                        607
                    } else {
                        608
                    })
                );
                assert_eq!(
                    error.context().redacted_cause(),
                    Some(if domain == ResourceFailureDomain::DeploymentFailure {
                        "HTTP body backing unavailable"
                    } else {
                        "HTTP body limit exceeded"
                    })
                );
                failures.push((key, error.context().plan()));
                drop(call);
            }
            assert_eq!(BODY_REFUSALS.with(Cell::get), 1);
            assert_eq!(
                failures[0], failures[1],
                "policy identity or selected plan changed"
            );
            let mut retry = handle
                .read_property("a", InteractionOptions::new(), &mut work())
                .unwrap();
            let output = take(&mut retry).await.unwrap();
            assert_eq!(&*output.data().unwrap().body, b"1");
            assert_eq!(
                Some(output.metadata().binding_response().unwrap().plan_id()),
                failures[0].1
            );
            drop(output);
            drop(retry);
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
fn public_path_allocation_observation_and_complete_release() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        request(&mut socket);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4096\r\n\r\n").unwrap();
        socket.write_all(&[b'1'; 4096]).unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    let io_runtime = recipe();
    let _ = std::thread::current();
    io_runtime.block_on(async {});
    // Runtime creation is outside the measurement. First-use reactor growth on
    // this thread is still included and reported, not assigned to engine rows.
    let (startup, accepted, terminal, parent_peak, accepted_layouts) = io_runtime.block_on(async {
        TRACE.with(|t| t.borrow_mut().armed = true);
        let (runtime, handle, observer, parent) = engine(port);
        let startup_snapshot = TRACE.with(|t| *t.borrow());
        let startup = startup_snapshot.live;
        let mut call = handle
            .read_property("a", InteractionOptions::new(), &mut work())
            .unwrap();
        let accepted_snapshot = TRACE.with(|t| *t.borrow());
        let accepted = accepted_snapshot.live;
        let accepted_layouts: [usize; 512] = std::array::from_fn(|i| {
            let block = accepted_snapshot.blocks[i];
            if block.0 != 0 && !startup_snapshot.blocks.iter().any(|old| old.0 == block.0) {
                block.1
            } else {
                0
            }
        });
        let output = take(&mut call).await.unwrap();
        assert_eq!(output.data().unwrap().content_type.capacity(), 16);
        let terminal = TRACE.with(|t| t.borrow().live);
        let parent_peak: u64 = parent.lock().unwrap().iter().map(|a| a.peak()).sum();
        drop(output);
        drop(call);
        drop(handle);
        shutdown(&runtime);
        drop(runtime);
        drop(observer);
        assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        (startup, accepted, terminal, parent_peak, accepted_layouts)
    });
    // Shared-runtime shutdown may initialize its thread-local parker; that
    // deployment allocation is outside this binding-path observation.
    TRACE.with(|t| t.borrow_mut().armed = false);
    drop(io_runtime);
    let measured = TRACE.with(|t| {
        let mut t = t.borrow_mut();
        t.armed = false;
        *t
    });
    assert_eq!(
        measured.live, 0,
        "tracked backing survived root and I/O runtime destruction"
    );
    println!(
        "startup={startup} accepted={accepted} terminal={terminal} requests={} peak={} largest={} parent_peak_sum={parent_peak} remaining={}",
        measured.count, measured.peak, measured.largest, measured.live
    );
    println!(
        "accepted live allocation sizes: {:?}",
        accepted_layouts
            .into_iter()
            .filter(|n| *n != 0)
            .collect::<Vec<_>>()
    );
    peer.join().unwrap();
}

#[test]
fn cancellation_observes_sdk_disposal_and_checks_allocation_free_engine_handoff() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sent, received) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        request(&mut socket);
        sent.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
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
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            *t = EMPTY;
            t.armed = true;
        });
        call.cancel().unwrap();
        assert_eq!(TRACE.with(|t| t.borrow().count), 0);
        call.step(&mut Context::from_waker(Waker::noop()), &mut work());
        let disposal = TRACE.with(|t| *t.borrow());
        let result = call.try_take(&mut work()).unwrap().unwrap();
        let requests = TRACE.with(|t| {
            let mut t = t.borrow_mut();
            t.armed = false;
            t.count
        });
        assert_eq!(requests, disposal.count, "engine result-taking allocated");
        println!("SDK cancellation: requests={} peak={} largest={} retained_before_reactor_turn={}; engine cancel/status and result-taking=0", disposal.count,disposal.peak,disposal.largest,disposal.live);
        assert!(matches!(result, Err(CoreError::Cancelled(_))));
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
fn stock_chunked_growth_overlap_and_incomplete_trailer_disposal_observations() {
    for mode in ["maxima", "reject-byte8192", "cancel-byte8191"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            request(&mut socket);
            let mut bytes = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n".to_vec();
            for _ in 0..29 {
                bytes.extend_from_slice(b"X: y\r\n");
            }
            bytes.extend_from_slice(b"X-Pad: ");
            bytes.resize(8188, b'x');
            bytes.extend_from_slice(b"\r\n\r\n1000\r\n");
            bytes.extend_from_slice(&[b'1'; 4096]);
            bytes.extend_from_slice(b"\r\n0\r\n");
            let start = bytes.len();
            bytes.extend_from_slice(b"X: y");
            let trailer_len = if mode == "reject-byte8192" {
                8192
            } else {
                8191
            };
            bytes.resize(
                start + trailer_len - if mode == "maxima" { 4 } else { 0 },
                b' ',
            );
            if mode == "maxima" {
                bytes.extend_from_slice(b"\r\n\r\n");
            }
            // No peer EOF: complete, over-limit, and pending disposal are distinct.
            let _ = socket.write_all(&bytes);
            assert_retired(&mut socket);
        });
        let io_runtime = recipe();
        io_runtime.block_on(async {
            let (runtime, handle, observer, parent) = engine(port);
            let mut call = handle.read_property("a", InteractionOptions::new(), &mut work()).unwrap();
            TRACE.with(|t| { *t.borrow_mut() = Trace { armed: true, ..EMPTY }; });
            let end = Instant::now() + Duration::from_secs(5);
            let mut max_poll_allocations = 0;
            let mut pending_snapshot = EMPTY;
            let result = loop {
                let before = TRACE.with(|t| t.borrow().count);
                call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                max_poll_allocations = max_poll_allocations.max(TRACE.with(|t| t.borrow().count) - before);
                if let Some(result) = call.try_take(&mut work()).unwrap() { break result; }
                if mode == "cancel-byte8191" && TRACE.with(|t| t.borrow().sizes.iter().any(|(n, count)| *n == 8192 && *count >= 2)) {
                    // Both Hyper read and trailer backing have grown. Leave ample
                    // real reactor turns for the unfinished section to be consumed.
                    for _ in 0..20 {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                        assert!(call.try_take(&mut work()).unwrap().is_none());
                    }
                    pending_snapshot = TRACE.with(|t| *t.borrow());
                    let before = pending_snapshot.count;
                    call.cancel().unwrap();
                    assert_eq!(TRACE.with(|t| t.borrow().count), before, "engine cancel/status allocated");
                    call.step(&mut Context::from_waker(Waker::noop()), &mut work());
                    let after_disposal = TRACE.with(|t| t.borrow().count);
                    let result = call.try_take(&mut work()).unwrap().unwrap();
                    assert_eq!(TRACE.with(|t| t.borrow().count), after_disposal, "engine result-taking allocated");
                    break result;
                }
                assert!(Instant::now() < end, "allocation observation stalled: {mode}");
                tokio::time::sleep(Duration::from_millis(1)).await;
            };
            match mode {
                "maxima" => assert_eq!(result.unwrap().data().unwrap().body.len(), 4096),
                "reject-byte8192" => assert!(matches!(result, Err(CoreError::Binding(_)))),
                _ => assert!(matches!(result, Err(CoreError::Cancelled(_)))),
            }
            let snapshot = TRACE.with(|t| { let mut t=t.borrow_mut(); t.armed=false; *t });
            println!("stock {mode}: requests={} peak={} largest={} terminal_live={} max_observed_poll_allocations={} pending_live={} disposal_requests={:?}",
                snapshot.count, snapshot.peak, snapshot.largest, snapshot.live, max_poll_allocations,
                pending_snapshot.live, if mode == "cancel-byte8191" { Some(snapshot.count - pending_snapshot.count) } else { None });
            println!("stock {mode} allocation size/counts: {:?}", snapshot.sizes.into_iter().filter(|(n,_)| *n != 0).collect::<Vec<_>>());
            drop(call); drop(handle); shutdown(&runtime); drop(runtime); drop(observer);
            assert!(parent.lock().unwrap().iter().all(|a| a.used() == 0));
        });
        drop(io_runtime);
        assert_eq!(
            TRACE.with(|t| t.borrow().live),
            0,
            "stock backing survived runtime destruction"
        );
        peer.join().unwrap();
    }
}
